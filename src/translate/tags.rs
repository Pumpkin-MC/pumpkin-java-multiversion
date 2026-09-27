//! UPDATE_TAGS.

use std::{cell::OnceCell, collections::HashMap};

use pumpkin_data::{Block, BlockId};
use pumpkin_protocol::{
    VarInt,
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::remap::{
    entity_id_remap::remap_entity_id_for_version, item_id_remap::remap_item_id_for_version,
};
use crate::tag::RegistryKey;
use crate::translate::{block::remap_state, registry::id_remap_or_drop};

include!(concat!(env!("OUT_DIR"), "/block_id.rs"));

/// 26.3 block ids as the client's. Blocks the client lacks become the block their default
/// state is shown as, like Via's block state substitutes.
struct BlockIds {
    version: JavaMinecraftVersion,
    /// Via's block id table for the client; `None` when the ids are unchanged.
    table: Option<&'static [i32]>,
    /// Client state to client block, from the blocks both versions have. Built on first use.
    state_owner: OnceCell<HashMap<u32, u32>>,
}

impl BlockIds {
    /// `None` before 1.13, which has no block tags.
    fn new(version: JavaMinecraftVersion) -> Option<Self> {
        let (_, table) = BLOCK_IDS
            .iter()
            .rev()
            .find(|(start, _)| version >= *start)?;
        Some(Self {
            version,
            table: *table,
            state_owner: OnceCell::new(),
        })
    }

    fn mapped(&self, id: u16) -> Option<u32> {
        match self.table {
            None => Some(u32::from(id)),
            Some(table) => table
                .get(usize::from(id))
                .and_then(|&id| u32::try_from(id).ok()),
        }
    }

    fn get(&self, id: u32) -> Option<u32> {
        let id = u16::try_from(id).ok()?;
        if let Some(mapped) = self.mapped(id) {
            return Some(mapped);
        }
        let block = Block::from_id(BlockId::new(id)?);
        let state = remap_state(u32::from(block.default_state.id.as_u16()), self.version);
        self.state_owner
            .get_or_init(|| {
                let mut owner = HashMap::new();
                for id in 0..BlockId::COUNT {
                    let (Some(block_id), Some(client_block)) = (BlockId::new(id), self.mapped(id))
                    else {
                        continue;
                    };
                    for state in Block::from_id(block_id).states {
                        owner
                            .entry(remap_state(u32::from(state.id.as_u16()), self.version))
                            .or_insert(client_block);
                    }
                }
                owner
            })
            .get(&state)
            .copied()
    }
}

/// How a registry's 26.3 ids become the client's.
enum IdMap {
    Block(Option<BlockIds>),
    Item,
    EntityType,
    /// Synced registry, by entry name.
    ByName(Vec<Option<u32>>),
    // TODO: fluid, game event, potion and point of interest ids are not remapped yet
    Unchanged,
}

impl IdMap {
    fn new(key: RegistryKey, version: JavaMinecraftVersion) -> Self {
        match key {
            RegistryKey::Block => Self::Block(BlockIds::new(version)),
            RegistryKey::Item => Self::Item,
            RegistryKey::EntityType => Self::EntityType,
            _ => id_remap_or_drop(key.identifier_string(), version)
                .map_or(Self::Unchanged, Self::ByName),
        }
    }

    fn get(&self, id: u32, version: JavaMinecraftVersion) -> Option<u32> {
        match self {
            Self::Block(blocks) => blocks.as_ref()?.get(id),
            // Unmapped items become air, which no tag holds
            Self::Item => {
                Some(u32::from(remap_item_id_for_version(id as u16, version))).filter(|&id| id != 0)
            }
            Self::EntityType => Some(u32::from(remap_entity_id_for_version(id as u16, version))),
            Self::ByName(table) => table.get(id as usize).copied().flatten(),
            Self::Unchanged => Some(id),
        }
    }
}

/// Keeps the registries the client knows, with the entries as the client's ids (ViaBackwards
/// `TagRewriter`). Blocks the client lacks become their substitute, other missing entries are
/// dropped.
pub fn update_tags_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let count = payload.get_var_int().ok()?.0;
    let mut kept = 0;
    let mut body = Vec::new();
    for _ in 0..count {
        let name = payload.get_str_borrowed().ok()?;
        let key = RegistryKey::from_string(name.strip_prefix("minecraft:").unwrap_or(name))
            .filter(|key| key.is_valid_for_version(version));
        let ids = key.map(|key| IdMap::new(key, version));
        if key.is_some() {
            kept += 1;
            body.write_string(name).ok()?;
        }

        let tag_count = payload.get_var_int().ok()?;
        if key.is_some() {
            body.write_var_int(&tag_count).ok()?;
        }
        for _ in 0..tag_count.0 {
            let tag = payload.get_str_borrowed().ok()?;
            let entries = payload
                .get_list(|read| Ok(read.get_var_int()?.0 as u32))
                .ok()?;
            let Some(ids) = &ids else { continue };
            let entries: Vec<_> = entries
                .into_iter()
                .filter_map(|id| ids.get(id, version))
                .collect();
            body.write_string(tag).ok()?;
            body.write_list(&entries, |out, &id| out.write_var_int(&VarInt(id as i32)))
                .ok()?;
        }
    }

    let mut out = Vec::new();
    out.write_var_int(&VarInt(kept)).ok()?;
    out.extend_from_slice(&body);
    Some(out)
}

#[cfg(test)]
mod tests {
    use pumpkin_data::packet::CURRENT_MC_VERSION;
    use pumpkin_protocol::{ClientPacket, java::client::config::CUpdateTags};

    use super::*;

    fn registries(mut payload: &[u8]) -> Vec<String> {
        let count = payload.get_var_int().unwrap().0;
        let mut names = Vec::new();
        for _ in 0..count {
            names.push(payload.get_str_borrowed().unwrap().to_string());
            for _ in 0..payload.get_var_int().unwrap().0 {
                let _ = payload.get_str_borrowed().unwrap();
                for _ in 0..payload.get_var_int().unwrap().0 {
                    let _ = payload.get_var_int().unwrap();
                }
            }
        }
        names
    }

    fn current() -> Vec<u8> {
        let mut current = Vec::new();
        CUpdateTags::new(pumpkin_data::tag::RegistryKey::NETWORK_KEYS)
            .write_packet_data(&mut current, &CURRENT_MC_VERSION)
            .unwrap();
        current
    }

    #[test]
    fn timeline_tags_only_reach_1_21_11_and_later() {
        let timeline = "minecraft:timeline".to_string();
        let old = update_tags_from_current(&current(), JavaMinecraftVersion::V_1_21_9).unwrap();
        assert!(!registries(&old).contains(&timeline));
        let new = update_tags_from_current(&current(), JavaMinecraftVersion::V_1_21_11).unwrap();
        assert!(registries(&new).contains(&timeline));
    }
}
