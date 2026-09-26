//! Player spawn info: play LOGIN and RESPAWN.

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    ClientPacket,
    java::client::play::{CLogin, CRespawn, PlayerSpawnData},
    ser::NetworkReadExt,
};
use pumpkin_util::version::JavaMinecraftVersion;

use super::reencode_current;

/// LOGIN (play): game modes are var ints since 26.3, online mode added in 26.2.
pub fn login_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let current = CURRENT_MC_VERSION;
    let entity_id = payload.get_i32_be().ok()?;
    let is_hardcore = payload.get_bool().ok()?;
    let dimension_names = payload
        .get_list(|read| read.get_str().map(String::from))
        .ok()?;
    let max_players = payload.get_var_int().ok()?;
    let view_distance = payload.get_var_int().ok()?;
    let simulated_distance = payload.get_var_int().ok()?;
    let reduced_debug_info = payload.get_bool().ok()?;
    let enabled_respawn_screen = payload.get_bool().ok()?;
    let limited_crafting = payload.get_bool().ok()?;
    let spawn_data = PlayerSpawnData::read(&mut payload, &current).ok()?;
    let online_mode = payload.get_bool().ok()?;
    let enforce_secure_chat = payload.get_bool().ok()?;

    let packet = CLogin {
        entity_id,
        is_hardcore,
        dimension_names: &dimension_names,
        max_players,
        view_distance,
        simulated_distance,
        reduced_debug_info,
        enabled_respawn_screen,
        limited_crafting,
        spawn_data,
        online_mode,
        enforce_secure_chat,
    };
    let mut out = Vec::new();
    packet.write_packet_data(&mut out, &version).ok()?;
    Some(out)
}

/// RESPAWN: same game mode change as LOGIN.
pub fn respawn_from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    reencode_current::<CRespawn>(payload, version)
}

#[cfg(test)]
mod tests {
    use pumpkin_protocol::VarInt;

    use super::*;

    #[test]
    fn login_outgoing_1_21_11_matches_direct_encode() {
        let dimension_names = vec!["minecraft:overworld".to_string()];
        let packet = CLogin {
            entity_id: 7,
            is_hardcore: false,
            dimension_names: &dimension_names,
            max_players: VarInt(20),
            view_distance: VarInt(10),
            simulated_distance: VarInt(10),
            reduced_debug_info: false,
            enabled_respawn_screen: true,
            limited_crafting: false,
            spawn_data: PlayerSpawnData::new(
                pumpkin_data::dimension::Dimension::OVERWORLD,
                42,
                1,
                -1,
                false,
                false,
                None,
                VarInt(0),
                VarInt(63),
            ),
            online_mode: true,
            enforce_secure_chat: false,
        };
        let mut payload = Vec::new();
        packet
            .write_packet_data(&mut payload, &CURRENT_MC_VERSION)
            .unwrap();
        let mut expected = Vec::new();
        packet
            .write_packet_data(&mut expected, &JavaMinecraftVersion::V_1_21_11)
            .unwrap();
        let out = login_from_current(&payload, JavaMinecraftVersion::V_1_21_11).unwrap();
        assert_eq!(out, expected);
    }
}
