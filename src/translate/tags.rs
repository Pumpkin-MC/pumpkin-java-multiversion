//! UPDATE_TAGS.

use pumpkin_protocol::{
    VarInt,
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::tag::{RegistryKey, get_registry_key_tags};

/// Keeps the registries of the 26.3 packet the client knows, with that version's tag lists.
/// Core only has the 26.3 tags, so the lists come from the plugin's own tables.
pub fn update_tags_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let count = payload.get_var_int().ok()?.0;
    let mut keys = Vec::new();
    for _ in 0..count {
        let name = payload.get_str_borrowed().ok()?;
        let identifier = name.strip_prefix("minecraft:").unwrap_or(name);
        if let Some(key) = RegistryKey::from_string(identifier)
            && key.is_valid_for_version(version)
        {
            keys.push(key);
        }
        let tag_count = payload.get_var_int().ok()?.0;
        for _ in 0..tag_count {
            let _ = payload.get_str_borrowed().ok()?;
            let id_count = payload.get_var_int().ok()?.0;
            for _ in 0..id_count {
                let _ = payload.get_var_int().ok()?;
            }
        }
    }

    let mut out = Vec::new();
    out.write_list(&keys, |out, &key| {
        out.write_string(&format!("minecraft:{}", key.identifier_string()))?;
        let Some(tags) = get_registry_key_tags(version, key) else {
            return out.write_var_int(&VarInt(0));
        };
        out.write_var_int(&VarInt(tags.len() as i32))?;
        for (name, (_, ids)) in tags.entries() {
            out.write_string_bounded(name, u16::MAX as usize)?;
            out.write_list(ids, |out, &id| out.write_var_int(&VarInt::from(id)))?;
        }
        Ok(())
    })
    .ok()?;
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

    #[test]
    fn timeline_tags_only_reach_1_21_11_and_later() {
        let mut current = Vec::new();
        CUpdateTags::new(pumpkin_data::tag::RegistryKey::NETWORK_KEYS)
            .write_packet_data(&mut current, &CURRENT_MC_VERSION)
            .unwrap();
        let timeline = "minecraft:timeline".to_string();

        let old = update_tags_from_current(&current, JavaMinecraftVersion::V_1_21_9).unwrap();
        assert!(!registries(&old).contains(&timeline));
        let new = update_tags_from_current(&current, JavaMinecraftVersion::V_1_21_11).unwrap();
        assert!(registries(&new).contains(&timeline));
    }
}
