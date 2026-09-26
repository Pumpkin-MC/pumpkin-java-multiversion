//! UPDATE_TAGS.

use pumpkin_data::tag::RegistryKey;
use pumpkin_protocol::{
    ClientPacket,
    java::client::config::CUpdateTags,
    ser::{NetworkReadExt, NetworkReadSliceExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

/// Rewrites with the client's version so pumpkin-data emits that version's tag lists.
pub fn update_tags_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let count = payload.get_var_int().ok()?.0;
    let mut keys = Vec::new();
    for _ in 0..count {
        let name = payload.get_str_borrowed().ok()?;
        let identifier = name.strip_prefix("minecraft:").unwrap_or(name);
        if let Some(key) = RegistryKey::from_string(identifier) {
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
    let packet = CUpdateTags::new(&keys);
    let mut out = Vec::new();
    packet.write_packet_data(&mut out, &version).ok()?;
    Some(out)
}
