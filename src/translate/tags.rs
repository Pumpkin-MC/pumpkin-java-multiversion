//! UPDATE_TAGS.

use std::{cell::OnceCell, collections::HashMap};

use pumpkin_data::{Block, BlockId, entity::EntityType, item::Item};
use pumpkin_protocol::{
    ClientPacket, VarInt,
    java::client::play::CUpdateTagsPlay,
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::remap::{
    entity_id_remap::remap_entity_id_for_version, item_id_remap::remap_item_id_for_version,
};
use crate::tag::RegistryKey;
use crate::translate::{
    block::remap_state,
    registry::{current_names, id_remap_or_drop, missing_tags},
};

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

    /// 26.3 id of an entry name, for the tags 26.3 no longer has.
    fn current_id(key: RegistryKey, name: &str) -> Option<u32> {
        match key {
            RegistryKey::Block => Block::from_name(name).map(|block| u32::from(block.id.as_u16())),
            RegistryKey::Item => Item::from_registry_key(name).map(|item| u32::from(item.id)),
            RegistryKey::EntityType => {
                EntityType::from_name(name).map(|entity| u32::from(entity.id))
            }
            _ => current_names(key.identifier_string())?
                .iter()
                .position(|n| *n == name)
                .map(|i| i as u32),
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
/// dropped. Tags of the client's version that 26.3 no longer has are added back.
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
        let mut tags = Vec::new();
        let tag_count = payload.get_var_int().ok()?.0;
        for _ in 0..tag_count {
            let tag = payload.get_str_borrowed().ok()?;
            let entries = payload
                .get_list(|read| Ok(read.get_var_int()?.0 as u32))
                .ok()?;
            tags.push((tag, entries));
        }
        let Some(key) = key else { continue };

        let ids = IdMap::new(key, version);
        let registry = key.identifier_string();
        let missing = missing_tags(version)
            .iter()
            .filter(|(missing_registry, _, _)| *missing_registry == registry)
            .map(|(_, tag, names)| {
                let entries = names
                    .iter()
                    .filter_map(|name| IdMap::current_id(key, name))
                    .collect();
                (*tag, entries)
            });
        let tags: Vec<_> = tags.into_iter().chain(missing).collect();

        kept += 1;
        body.write_string(name).ok()?;
        body.write_var_int(&VarInt(tags.len() as i32)).ok()?;
        for (tag, entries) in tags {
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

/// Registries of the fixed layout before 1.17, in order. Entity types since 1.14.
const LEGACY_CATEGORIES: [&str; 4] = [
    "minecraft:block",
    "minecraft:item",
    "minecraft:fluid",
    "minecraft:entity_type",
];

/// Play UPDATE_TAGS. Before 1.17 the registries are the fixed [`LEGACY_CATEGORIES`] without
/// their names.
pub fn play_update_tags_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let translated = update_tags_from_current(payload, version)?;
    if version >= JavaMinecraftVersion::V_1_17 {
        return Some(translated);
    }

    // Each registry's tag list, which is the category layout as is.
    let mut registries = HashMap::new();
    let mut reader = &translated[..];
    for _ in 0..reader.get_var_int().ok()?.0 {
        let name = reader.get_str_borrowed().ok()?;
        let start = reader;
        for _ in 0..reader.get_var_int().ok()?.0 {
            let _ = reader.get_str_borrowed().ok()?;
            let _ = reader.get_list(|read| read.get_var_int()).ok()?;
        }
        registries.insert(name, &start[..start.len() - reader.len()]);
    }

    let categories = if version >= JavaMinecraftVersion::V_1_14 {
        &LEGACY_CATEGORIES[..]
    } else {
        &LEGACY_CATEGORIES[..3]
    };
    let mut out = Vec::new();
    for category in categories {
        match registries.get(category) {
            Some(tags) => out.extend_from_slice(tags),
            None => out.write_var_int(&VarInt(0)).ok()?,
        }
    }
    Some(out)
}

/// Before 1.20.2 there is no configuration state, so vanilla sends the tags in play right
/// after the join packet. `None` for clients that get them in configuration or have none.
pub fn join_tags(version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    if !(JavaMinecraftVersion::V_1_13..JavaMinecraftVersion::V_1_20_2).contains(&version) {
        return None;
    }
    let mut current = Vec::new();
    CUpdateTagsPlay::new(pumpkin_data::tag::RegistryKey::NETWORK_KEYS)
        .write_packet_data(&mut current)
        .ok()?;
    play_update_tags_from_current(&current, version)
}

#[cfg(test)]
mod tests {
    use pumpkin_protocol::java::client::config::CUpdateTags;

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
            .write_packet_data(&mut current)
            .unwrap();
        current
    }

    #[test]
    fn join_tags_only_before_the_configuration_state() {
        assert!(join_tags(JavaMinecraftVersion::V_1_12_2).is_none());
        assert!(join_tags(JavaMinecraftVersion::V_1_13).is_some());
        assert!(join_tags(JavaMinecraftVersion::V_1_20).is_some());
        assert!(join_tags(JavaMinecraftVersion::V_1_20_2).is_none());
    }

    /// 1.14 - 1.16: block, item, fluid and entity type lists, no registry names.
    #[test]
    fn legacy_play_tags_are_the_fixed_categories() {
        let mut payload = &join_tags(JavaMinecraftVersion::V_1_16_4).unwrap()[..];
        for _ in 0..4 {
            for _ in 0..payload.get_var_int().unwrap().0 {
                let tag = payload.get_str_borrowed().unwrap();
                assert!(!tag.is_empty());
                let _ = payload.get_list(|read| read.get_var_int()).unwrap();
            }
        }
        assert!(payload.is_empty());
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
