//! Player spawn info: play LOGIN, RESPAWN and SET_DEFAULT_SPAWN_POSITION.

use pumpkin_data::entity::EntityType;
use pumpkin_protocol::java::legacy::LegacyWriteWith;
use pumpkin_protocol::{
    ServerPacket,
    java::client::play::{CLogin, CPlayerSpawnPosition, CRespawn, PlayerSpawnData},
    ser::{NetworkReadExt, NetworkReadSliceExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use super::{entity_data, registry};

/// LOGIN (play): game modes are var ints since 26.3, online mode added in 26.2.
pub fn login_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let entity_id = payload.get_i32_be().ok()?;
    // The own player never gets an ADD_ENTITY
    entity_data::track_spawn(entity_id, EntityType::PLAYER.id);
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
    let spawn_data = PlayerSpawnData::read(&mut payload).ok()?;
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
    let registry = registry::legacy_nbt(version, packet.spawn_data.dimension.minecraft_name);
    let mut out = Vec::new();
    packet
        .write_legacy_with(&mut out, &version, &registry)
        .ok()?;
    Some(out)
}

/// RESPAWN: same game mode change as LOGIN.
pub fn respawn_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let packet = CRespawn::read(&mut payload).ok()?;
    let registry = registry::legacy_nbt(version, packet.player_spawn_info.dimension.minecraft_name);
    let mut out = Vec::new();
    packet
        .write_legacy_with(&mut out, &version, &registry)
        .ok()?;
    Some(out)
}

/// SET_DEFAULT_SPAWN_POSITION: dimension and pitch since 1.21.9, angle since 1.17, packed
/// position since 1.14 (ViaBackwards). Core's writer handles every older layout.
pub fn spawn_position_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let dimension = payload.get_str_borrowed().ok()?.to_string();
    let location = payload.get_block_pos().ok()?;
    let yaw = payload.get_f32_be().ok()?;
    let pitch = payload.get_f32_be().ok()?;
    super::legacy_bytes(
        &CPlayerSpawnPosition::new(location, yaw, pitch, dimension),
        version,
    )
}

#[cfg(test)]
mod tests {
    use pumpkin_protocol::ClientPacket;
    use pumpkin_protocol::VarInt;
    use pumpkin_protocol::java::legacy::LegacyWrite;

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
        packet.write_packet_data(&mut payload).unwrap();
        let mut expected = Vec::new();
        let version = JavaMinecraftVersion::V_1_21_11;
        let registry = registry::legacy_nbt(version, packet.spawn_data.dimension.minecraft_name);
        packet
            .write_legacy_with(&mut expected, &version, &registry)
            .unwrap();
        let out = login_from_current(&payload, JavaMinecraftVersion::V_1_21_11).unwrap();
        assert_eq!(out, expected);
    }

    #[test]
    fn spawn_position_matches_each_older_layout() {
        use pumpkin_util::math::{position::BlockPos, vector3::Vector3};

        let packet = || {
            CPlayerSpawnPosition::new(
                BlockPos(Vector3::new(12, 70, -5)),
                90.0,
                10.0,
                "minecraft:overworld".to_string(),
            )
        };
        let mut current = Vec::new();
        packet().write_packet_data(&mut current).unwrap();
        for version in [
            JavaMinecraftVersion::V_1_21_7,
            JavaMinecraftVersion::V_1_16,
            JavaMinecraftVersion::V_1_8,
        ] {
            let mut expected = Vec::new();
            packet().write_legacy(&mut expected, &version).unwrap();
            assert_eq!(
                spawn_position_from_current(&current, version),
                Some(expected)
            );
        }
    }
}
