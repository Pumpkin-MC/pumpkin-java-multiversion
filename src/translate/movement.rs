//! Entity movement: MOVE_ENTITY_POS, MOVE_ENTITY_POS_ROT, MOVE_ENTITY_ROT, ENTITY_POSITION_SYNC,
//! SET_ENTITY_MOTION, PLAYER_ROTATION, PLAYER_POSITION.

use super::legacy_bytes;
use pumpkin_protocol::java::legacy::LegacyPacket;
use pumpkin_protocol::{
    codec::lp_vector_3d::LpVector3d,
    java::client::play::{
        CEntityPositionSync, CEntityVelocity, CPlayerPosition, CPlayerRotation, CUpdateEntityPos,
        CUpdateEntityPosRot, CUpdateEntityRot,
    },
    ser::NetworkReadExt,
};
use pumpkin_util::{math::vector3::Vector3, version::JavaMinecraftVersion};

pub fn pos_from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    super::reencode_current::<CUpdateEntityPos>(payload, version)
}

pub fn pos_rot_from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    super::reencode_current::<CUpdateEntityPosRot>(payload, version)
}

/// On ground moved in front of the rotation in 26.3.
pub fn rot_from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    super::reencode_current::<CUpdateEntityRot>(payload, version)
}

/// ENTITY_POSITION_SYNC, as `(client packet id, payload)`: since 26.3 a path replaces the
/// delta; TELEPORT_ENTITY before 1.21.2. `None` for non-linear paths.
// TODO: the velocity older clients expect is sent as zero.
pub fn position_sync_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<(i32, Vec<u8>)> {
    let entity_id = payload.get_var_int().ok()?;
    if payload.get_var_int().ok()?.0 != 0 {
        return None;
    }
    let position = Vector3::new(
        payload.get_f64_be().ok()?,
        payload.get_f64_be().ok()?,
        payload.get_f64_be().ok()?,
    );
    let yaw = payload.get_f32_be().ok()?;
    let pitch = payload.get_f32_be().ok()?;
    let on_ground = payload.get_bool().ok()?;
    let packet = CEntityPositionSync::new(
        entity_id,
        position,
        Vector3::new(0.0, 0.0, 0.0),
        yaw,
        pitch,
        on_ground,
    );
    Some((
        CEntityPositionSync::legacy_id(version)?,
        legacy_bytes(&packet, version)?,
    ))
}

/// SET_ENTITY_MOTION: packed velocity since 1.21.9, three shorts before. Core's writer handles
/// every older layout.
pub fn entity_motion_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let entity_id = payload.get_var_int().ok()?;
    let LpVector3d(velocity) = LpVector3d::read(&mut payload).ok()?;
    legacy_bytes(&CEntityVelocity::new(entity_id, velocity), version)
}

/// PLAYER_ROTATION: each angle got a relative flag in 1.21.9.
// TODO: relative rotations need the player's rotation (ViaBackwards `PlayerRotationStorage`);
// they are dropped. Core itself only sends absolute ones.
pub fn player_rotation_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let yaw = payload.get_f32_be().ok()?;
    let relative_yaw = payload.get_bool().ok()?;
    let pitch = payload.get_f32_be().ok()?;
    let relative_pitch = payload.get_bool().ok()?;
    if relative_yaw || relative_pitch {
        return None;
    }
    legacy_bytes(&CPlayerRotation { yaw, pitch }, version)
}

/// PLAYER_POSITION: core's writer has every layout; before 1.21.2 the flags are a byte and
/// the teleport id comes last.
// TODO: the delta is dropped before 1.21.2; Via sends it as a separate entity motion
pub fn player_position_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    super::reencode_current::<CPlayerPosition>(payload, version)
}

#[cfg(test)]
mod tests {
    use pumpkin_data::packet::CURRENT_MC_VERSION;
    use pumpkin_protocol::ClientPacket;
    use pumpkin_protocol::VarInt;
    use pumpkin_protocol::java::legacy::LegacyWrite;
    use pumpkin_protocol::ser::NetworkWriteExt;

    use super::*;

    const VERSIONS: [JavaMinecraftVersion; 3] = [
        JavaMinecraftVersion::V_1_21_11,
        JavaMinecraftVersion::V_1_12_2,
        JavaMinecraftVersion::V_1_7_6,
    ];

    fn check(
        packet: &impl LegacyWrite,
        translate: fn(&[u8], JavaMinecraftVersion) -> Option<Vec<u8>>,
    ) {
        let current = legacy_bytes(packet, CURRENT_MC_VERSION).unwrap();
        for version in VERSIONS {
            assert_eq!(
                translate(&current, version),
                legacy_bytes(packet, version),
                "{version:?}"
            );
        }
    }

    #[test]
    fn player_position_matches_direct_encode() {
        check(
            &CPlayerPosition::new(
                VarInt(3),
                Vector3::new(1.5, 64.0, -2.5),
                Vector3::new(0.0, 0.0, 0.0),
                90.0,
                10.0,
                Vec::new(),
            ),
            player_position_from_current,
        );
    }

    #[test]
    fn movement_matches_direct_encode() {
        let delta = Vector3::new(4096, -128, 300);
        for on_ground in [true, false] {
            check(
                &CUpdateEntityPos::new(VarInt(9), delta, on_ground),
                pos_from_current,
            );
            check(
                &CUpdateEntityPosRot::new(VarInt(9), delta, 64, 200, on_ground),
                pos_rot_from_current,
            );
            check(
                &CUpdateEntityRot::new(VarInt(9), 64, 200, on_ground),
                rot_from_current,
            );
        }
    }

    #[test]
    fn pos_drops_stepped_delta() {
        let mut payload = Vec::new();
        payload.write_var_int(&VarInt(9)).unwrap();
        payload.write_var_int(&VarInt(2)).unwrap();
        payload.write_i16_be(1).unwrap();
        payload.write_i16_be(2).unwrap();
        payload.write_i16_be(3).unwrap();
        assert_eq!(
            pos_from_current(&payload, JavaMinecraftVersion::V_1_21_11),
            None
        );
    }

    #[test]
    fn position_sync_drops_path() {
        let zero = Vector3::new(0.0, 0.0, 0.0);
        let packet = CEntityPositionSync::new(
            VarInt(3),
            Vector3::new(1.5, 64.0, -2.25),
            zero,
            90.0,
            10.0,
            true,
        );
        let current = legacy_bytes(&packet, CURRENT_MC_VERSION).unwrap();
        for version in VERSIONS {
            let expected = (
                CEntityPositionSync::legacy_id(version).unwrap(),
                legacy_bytes(&packet, version).unwrap(),
            );
            assert_eq!(
                position_sync_from_current(&current, version),
                Some(expected)
            );
        }
    }

    #[test]
    fn entity_motion_uses_shorts_before_1_21_9() {
        for velocity in [Vector3::new(0.5, -0.25, 0.0), Vector3::new(0.0, 0.0, 0.0)] {
            let packet = CEntityVelocity::new(VarInt(7), velocity);
            let mut current = Vec::new();
            packet.write_packet_data(&mut current).unwrap();
            let version = JavaMinecraftVersion::V_1_21_7;
            let mut expected = Vec::new();
            packet.write_legacy(&mut expected, &version).unwrap();
            assert_eq!(
                entity_motion_from_current(&current, version),
                Some(expected)
            );
        }
    }
}
