//! Serverbound play packets from older clients to 26.3.

use crate::legacy::{LegacyRead, block_pos_to_current};
use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    ClientPacket, ServerPacket, VarInt,
    java::server::play::{
        ActionType, SChatMessage, SClientInformationPlay, SInteract, SKeepAlive, SPlayerAbilities,
        SPlayerPosition, SPlayerPositionRotation, SUseItemOn,
    },
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::{
    math::{position::BlockPos, vector3::Vector3},
    version::JavaMinecraftVersion,
};

use crate::legacy::ids::serverbound::play;

eras! {
    enum UseItemFormat {
        /// Hand only.
        V1_7 = V_1_7_2,
        /// Sequence added.
        V1_19 = V_1_19,
        /// Yaw and pitch added; same as 26.3.
        V1_21 = V_1_21,
    }
}

eras! {
    enum SignFormat {
        /// Position, lines.
        V1_7 = V_1_7_2,
        /// Front flag before the lines.
        V1_20 = V_1_20,
    }
}

eras! {
    enum PlayerActionFormat {
        /// u8 status, no sequence.
        V1_7 = V_1_7_2,
        /// Var int status.
        V1_9 = V_1_9,
        /// Sequence added.
        V1_19 = V_1_19,
    }
}

eras! {
    enum PlayerCommandFormat {
        /// i32 entity, u8 action, i32 jump boost; actions 0/1 are sneaking.
        V1_7 = V_1_7_2,
        /// Var ints.
        V1_8 = V_1_8,
        /// Sneak actions also sent as the shift flag of PLAYER_INPUT.
        V1_21_2 = V_1_21_2,
        /// Sneak actions removed; same as 26.3.
        V1_21_6 = V_1_21_6,
    }
}

eras! {
    enum DifficultyFormat {
        /// u8.
        V1_7 = V_1_7_2,
        /// Var int; same as 26.3.
        V1_21_6 = V_1_21_6,
    }
}

eras! {
    enum PlayerInputFormat {
        /// Sideways and forward floats, jump and sneak booleans.
        V1_7 = V_1_7_2,
        /// Sideways and forward floats, flags byte (jump 1, sneak 2).
        V1_8 = V_1_8,
        /// Bitmask; same as 26.3.
        V1_21_2 = V_1_21_2,
    }
}

eras! {
    enum MoveVehicleFormat {
        /// No on-ground flag.
        V1_7 = V_1_7_2,
        /// Same as 26.3.
        V1_21_4 = V_1_21_4,
    }
}

eras! {
    enum ButtonClickFormat {
        /// u8 window, i8 button.
        V1_7 = V_1_7_2,
        /// Var ints; same as 26.3.
        V1_21_2 = V_1_21_2,
    }
}

eras! {
    enum InteractFormat {
        /// No interact-at; the plain interact is the only right click.
        V1_7 = V_1_7_2,
        /// Right clicks send interact-at, then interact.
        V1_8 = V_1_8,
        /// Attacks split into ATTACK; same as 26.3.
        V26_1 = V_26_1,
    }
}

/// INTERACT, as the 26.3 `(packet id, payload)`: attacks become ATTACK, interact-at the new
/// INTERACT. `None` drops the plain interact that follows an interact-at.
// TODO: spectators should send TELEPORT_TO_ENTITY instead of ATTACK
pub fn interact_to_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<(i32, Vec<u8>)> {
    let format = InteractFormat::of(version);
    if format == InteractFormat::V26_1 {
        return Some((play::INTERACT.current(), payload.to_vec()));
    }
    let interact = SInteract::read_legacy(&mut payload, &version).ok()?;
    let action = ActionType::try_from(interact.r#type.0).ok()?;
    let mut out = Vec::new();
    match action {
        ActionType::Attack => {
            out.write_var_int(&interact.entity_id).ok()?;
            Some((play::ATTACK.current(), out))
        }
        ActionType::Interact if format >= InteractFormat::V1_8 => None,
        ActionType::Interact | ActionType::InteractAt => {
            SInteract {
                r#type: VarInt(ActionType::InteractAt as i32),
                ..interact
            }
            .write_packet_data(&mut out, &pumpkin_data::packet::CURRENT_MC_VERSION)
            .ok()?;
            Some((play::INTERACT.current(), out))
        }
    }
}

/// PLAYER_COMMAND: sneak actions become the shift flag of
/// PLAYER_INPUT, or are dropped when the client's PLAYER_INPUT already carries it.
// TODO: before 1.21.2 infer the movement, jump and sprint keys per tick (needs per-player state)
pub fn player_command_to_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<(i32, Vec<u8>)> {
    let format = PlayerCommandFormat::of(version);
    if format == PlayerCommandFormat::V1_21_6 {
        return Some((play::PLAYER_COMMAND.current(), payload.to_vec()));
    }
    let var_ints = format >= PlayerCommandFormat::V1_8;
    let entity_id = if var_ints {
        payload.get_var_int().ok()?
    } else {
        VarInt(payload.get_i32_be().ok()?)
    };
    let action_id = if var_ints {
        payload.get_var_int().ok()?
    } else {
        VarInt(i32::from(payload.get_u8().ok()?))
    };
    let jump_boost = if var_ints {
        payload.get_var_int().ok()?
    } else {
        VarInt(payload.get_i32_be().ok()?)
    };
    let mut out = Vec::new();
    match action_id.0 {
        // Off a vehicle older clients only report sneaking here, so the other keys stay unset
        0 | 1 if format < PlayerCommandFormat::V1_21_2 => {
            out.write_i8(if action_id.0 == 0 { 32 } else { 0 }).ok()?;
            Some((play::PLAYER_INPUT.current(), out))
        }
        0 | 1 => None,
        action => {
            out.write_var_int(&entity_id).ok()?;
            out.write_var_int(&VarInt(action - 2)).ok()?;
            out.write_var_int(&jump_boost).ok()?;
            Some((play::PLAYER_COMMAND.current(), out))
        }
    }
}

/// `CLIENT_COMMAND`: drops legacy `OPEN_INVENTORY_ACHIEVEMENT` (`2` before 1.16) so 26.3 does
/// not interpret it as `REQUEST_GAMERULE_VALUES`, and converts 1.7 `u8` action IDs.
pub fn client_command_to_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<(i32, Vec<u8>)> {
    let action_id = if version <= JavaMinecraftVersion::V_1_7_6 {
        i32::from(payload.get_u8().ok()?)
    } else {
        payload.get_var_int().ok()?.0
    };
    if action_id == 2 && version < JavaMinecraftVersion::V_1_16 {
        return None;
    }
    let mut out = Vec::new();
    out.write_var_int(&VarInt(action_id)).ok()?;
    Some((play::CLIENT_COMMAND.current(), out))
}

/// `USE_ITEM_ON`: converts 1.7-1.8 air right-clicks (`face == 255`) into `USE_ITEM`, and
/// translates pre-1.21.2 block placement layouts.
pub fn use_item_on_to_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
    rotation: impl FnOnce() -> (f32, f32),
) -> Option<(i32, Vec<u8>)> {
    if version >= JavaMinecraftVersion::V_1_21_2 {
        return Some((play::USE_ITEM_ON.current(), payload.to_vec()));
    }
    if version >= JavaMinecraftVersion::V_1_9 {
        let packet = SUseItemOn::read(&mut payload, &version).ok()?;
        let mut out = Vec::new();
        packet
            .write_packet_data(&mut out, &CURRENT_MC_VERSION)
            .ok()?;
        return Some((play::USE_ITEM_ON.current(), out));
    }

    let position = if version <= JavaMinecraftVersion::V_1_7_6 {
        let x = payload.get_i32_be().ok()?;
        let y = i32::from(payload.get_u8().ok()?);
        let z = payload.get_i32_be().ok()?;
        BlockPos::new(x, y, z)
    } else {
        payload.get_block_pos(&version).ok()?
    };
    let face = payload.get_u8().ok()?;
    if face == 255 {
        let (yaw, pitch) = rotation();
        let mut out = Vec::new();
        out.write_var_int(&VarInt(0)).ok()?;
        out.write_var_int(&VarInt(0)).ok()?;
        out.write_f32_be(yaw).ok()?;
        out.write_f32_be(pitch).ok()?;
        return Some((play::USE_ITEM.current(), out));
    }
    if payload.len() < 3 {
        return None;
    }
    let c = &payload[payload.len() - 3..];
    let packet = SUseItemOn {
        hand: VarInt(0),
        position,
        face: VarInt(i32::from(face)),
        cursor_pos: Vector3::new(
            f32::from(c[0]) / 16.0,
            f32::from(c[1]) / 16.0,
            f32::from(c[2]) / 16.0,
        ),
        inside_block: false,
        is_against_world_border: false,
        sequence: VarInt(0),
    };
    let mut out = Vec::new();
    packet
        .write_packet_data(&mut out, &CURRENT_MC_VERSION)
        .ok()?;
    Some((play::USE_ITEM_ON.current(), out))
}

/// `None` when the payload is already in the 26.3 format. Only called for clients below 26.3.
/// `rotation` is the player's yaw and pitch, for packets that did not carry them yet.
#[expect(clippy::too_many_lines)]
pub fn play_to_current(
    new_id: i32,
    mut payload: &[u8],
    version: JavaMinecraftVersion,
    rotation: impl FnOnce() -> (f32, f32),
) -> Option<Vec<u8>> {
    // PUNCH / SWING (26.3 punch has 0 bytes)
    if new_id == play::PUNCH.current() {
        return Some(Vec::new());
    }

    if new_id == play::CUSTOM_PAYLOAD.current() && version < JavaMinecraftVersion::V_1_13 {
        return super::plugin_message::to_current(payload, version);
    }

    if new_id == play::KEEP_ALIVE.current() && version < JavaMinecraftVersion::V_1_12_2 {
        let pkt = SKeepAlive::read(&mut payload, &version).ok()?;
        let mut out = Vec::new();
        pkt.write_packet_data(&mut out, &CURRENT_MC_VERSION).ok()?;
        return Some(out);
    }

    if new_id == play::CLIENT_INFORMATION.current() && version < JavaMinecraftVersion::V_1_18 {
        let pkt = if version <= JavaMinecraftVersion::V_1_7_6 {
            let locale = payload.get_str_borrowed().ok()?;
            let view_distance = payload.get_i8().ok()?;
            let chat_mode = VarInt(i32::from(payload.get_i8().ok()?));
            let chat_colors = payload.get_bool().ok()?;
            let _difficulty = payload.get_u8().ok()?;
            let show_cape = payload.get_bool().ok()?;
            SClientInformationPlay {
                locale,
                view_distance,
                chat_mode,
                chat_colors,
                skin_parts: u8::from(show_cape),
                main_hand: VarInt(1),
                text_filtering: false,
                server_listing: true,
            }
        } else {
            SClientInformationPlay::read(&mut payload, &version).ok()?
        };
        let mut out = Vec::new();
        pkt.write_packet_data(&mut out, &CURRENT_MC_VERSION).ok()?;
        return Some(out);
    }

    if new_id == play::MOVE_PLAYER_POS.current() && version <= JavaMinecraftVersion::V_1_7_6 {
        let pkt = SPlayerPosition::read(&mut payload, &version).ok()?;
        let mut out = Vec::new();
        pkt.write_packet_data(&mut out, &CURRENT_MC_VERSION).ok()?;
        return Some(out);
    }

    if new_id == play::MOVE_PLAYER_POS_ROT.current() && version <= JavaMinecraftVersion::V_1_7_6 {
        let pkt = SPlayerPositionRotation::read(&mut payload, &version).ok()?;
        let mut out = Vec::new();
        pkt.write_packet_data(&mut out, &CURRENT_MC_VERSION).ok()?;
        return Some(out);
    }

    if new_id == play::PLAYER_ABILITIES.current() && version < JavaMinecraftVersion::V_1_16 {
        let pkt = SPlayerAbilities::read(&mut payload, &version).ok()?;
        let mut out = Vec::new();
        pkt.write_packet_data(&mut out, &CURRENT_MC_VERSION).ok()?;
        return Some(out);
    }

    if new_id == play::CHAT.current() && version < JavaMinecraftVersion::V_1_21_5 {
        let pkt = if version < JavaMinecraftVersion::V_1_19_3 {
            let message = payload.get_str_borrowed().ok()?;
            SChatMessage {
                message,
                timestamp: 0,
                salt: 0,
                signature: None,
                message_count: VarInt(0),
                acknowledged: &[0; 3],
                checksum: 0,
            }
        } else {
            SChatMessage::read(&mut payload, &version).ok()?
        };
        let mut out = Vec::new();
        pkt.write_packet_data(&mut out, &CURRENT_MC_VERSION).ok()?;
        return Some(out);
    }

    // ACCEPT_TELEPORTATION (only teleport_id: VarInt before 26.3)
    if new_id == play::ACCEPT_TELEPORTATION.current() {
        let teleport_id = payload.get_var_int().ok()?;
        let mut out = Vec::new();
        let _ = out.write_var_int(&teleport_id);
        let _ = out.write_f64_be(0.0);
        let _ = out.write_f64_be(0.0);
        let _ = out.write_f64_be(0.0);
        let _ = out.write_f32_be(0.0);
        let _ = out.write_f32_be(0.0);
        return Some(out);
    }

    // USE_ITEM (before 1.21 the server used the player's rotation)
    let use_item = UseItemFormat::of(version);
    if new_id == play::USE_ITEM.current() && use_item != UseItemFormat::V1_21 {
        let hand = payload.get_var_int().ok()?;
        let sequence = if use_item == UseItemFormat::V1_19 {
            payload.get_var_int().unwrap_or(VarInt(0))
        } else {
            VarInt(0)
        };
        let (yaw, pitch) = rotation();
        let mut out = Vec::new();
        let _ = out.write_var_int(&hand);
        let _ = out.write_var_int(&sequence);
        let _ = out.write_f32_be(yaw);
        let _ = out.write_f32_be(pitch);
        return Some(out);
    }

    // SIGN_UPDATE (26.3 moved the front flag after the lines as a var int)
    if new_id == play::SIGN_UPDATE.current() {
        let mut out = Vec::new();
        if version <= JavaMinecraftVersion::V_1_7_6 {
            let x = payload.get_i32_be().ok()?;
            let y = i32::from(payload.get_i16_be().ok()?);
            let z = payload.get_i32_be().ok()?;
            let _ = out.write_block_pos(&BlockPos::new(x, y, z), &CURRENT_MC_VERSION);
        } else {
            let pos_val = block_pos_to_current(payload.get_i64_be().ok()?, &version);
            let _ = out.write_i64_be(pos_val);
        }
        let is_front = if SignFormat::of(version) == SignFormat::V1_20 {
            payload.get_bool().unwrap_or(true)
        } else {
            true
        };
        let line1 = payload.get_str_borrowed().unwrap_or("");
        let line2 = payload.get_str_borrowed().unwrap_or("");
        let line3 = payload.get_str_borrowed().unwrap_or("");
        let line4 = payload.get_str_borrowed().unwrap_or("");
        let _ = out.write_string(line1);
        let _ = out.write_string(line2);
        let _ = out.write_string(line3);
        let _ = out.write_string(line4);
        let _ = out.write_var_int(&VarInt(i32::from(is_front)));
        return Some(out);
    }

    // PLAYER_ACTION (26.3 shifted statuses above 0 by one)
    if new_id == play::PLAYER_ACTION.current() {
        let format = PlayerActionFormat::of(version);
        let status = if format >= PlayerActionFormat::V1_9 {
            payload.get_var_int().ok()?
        } else {
            VarInt(i32::from(payload.get_u8().ok()?))
        };
        let pos_bytes = if version <= JavaMinecraftVersion::V_1_7_6 {
            let x = payload.get_i32_be().ok()?;
            let y = i32::from(payload.get_u8().ok()?);
            let z = payload.get_i32_be().ok()?;
            let mut b = Vec::with_capacity(8);
            b.write_block_pos(&BlockPos::new(x, y, z), &CURRENT_MC_VERSION)
                .ok()?;
            b
        } else {
            let pos_val = block_pos_to_current(payload.get_i64_be().ok()?, &version);
            pos_val.to_be_bytes().to_vec()
        };
        let face = payload.get_u8().ok()?;
        let sequence = if format == PlayerActionFormat::V1_19 {
            payload.get_var_int().unwrap_or(VarInt(0))
        } else {
            VarInt(0)
        };
        let new_status = if status.0 >= 1 {
            VarInt(status.0 + 1)
        } else {
            status
        };
        let mut out = Vec::new();
        let _ = out.write_var_int(&new_status);
        out.extend_from_slice(&pos_bytes);
        let _ = out.write_u8(face);
        let _ = out.write_var_int(&sequence);
        return Some(out);
    }

    // RESOURCE_PACK response
    if new_id == play::RESOURCE_PACK.current() {
        return super::resource_pack::response_to_current(payload, version);
    }

    // CHANGE_DIFFICULTY
    if new_id == play::CHANGE_DIFFICULTY.current()
        && DifficultyFormat::of(version) == DifficultyFormat::V1_7
    {
        let diff = payload.get_u8().ok()?;
        let mut out = Vec::new();
        let _ = out.write_var_int(&VarInt(i32::from(diff)));
        return Some(out);
    }

    // PLAYER_INPUT
    let input_format = PlayerInputFormat::of(version);
    if new_id == play::PLAYER_INPUT.current() && input_format < PlayerInputFormat::V1_21_2 {
        let sideways = payload.get_f32_be().unwrap_or(0.0);
        let forward = payload.get_f32_be().unwrap_or(0.0);
        let (jumping, sneaking) = if input_format == PlayerInputFormat::V1_7 {
            (
                payload.get_bool().unwrap_or(false),
                payload.get_bool().unwrap_or(false),
            )
        } else {
            let flags = payload.get_u8().unwrap_or(0);
            (flags & 1 != 0, flags & 2 != 0)
        };
        let mut input: i8 = 0;
        if forward > 0.0 {
            input |= 1;
        } else if forward < 0.0 {
            input |= 2;
        }
        if sideways > 0.0 {
            input |= 4;
        } else if sideways < 0.0 {
            input |= 8;
        }
        if jumping {
            input |= 16;
        }
        if sneaking {
            input |= 32;
        }
        let mut out = Vec::new();
        let _ = out.write_i8(input);
        return Some(out);
    }

    // MOVE_VEHICLE
    if new_id == play::MOVE_VEHICLE.current()
        && MoveVehicleFormat::of(version) == MoveVehicleFormat::V1_7
    {
        let x = payload.get_f64_be().ok()?;
        let y = payload.get_f64_be().ok()?;
        let z = payload.get_f64_be().ok()?;
        let yaw = payload.get_f32_be().ok()?;
        let pitch = payload.get_f32_be().ok()?;
        let mut out = Vec::new();
        let _ = out.write_f64_be(x);
        let _ = out.write_f64_be(y);
        let _ = out.write_f64_be(z);
        let _ = out.write_f32_be(yaw);
        let _ = out.write_f32_be(pitch);
        let _ = out.write_bool(false);
        return Some(out);
    }

    // CONTAINER_BUTTON_CLICK
    if new_id == play::CONTAINER_BUTTON_CLICK.current()
        && ButtonClickFormat::of(version) == ButtonClickFormat::V1_7
    {
        let window_id = payload.get_u8().ok()?;
        let button_id = payload.get_i8().ok()?;
        let mut out = Vec::new();
        let _ = out.write_var_int(&VarInt(i32::from(window_id)));
        let _ = out.write_var_int(&VarInt(i32::from(button_id)));
        return Some(out);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_protocol::ServerPacket;

    const V1_21_11: JavaMinecraftVersion = JavaMinecraftVersion::V_1_21_11;

    #[test]
    fn attack_becomes_attack_packet() {
        // Entity 5, attack, not sneaking
        let client = [5, 1, 0];
        assert_eq!(
            interact_to_current(&client, V1_21_11),
            Some((play::ATTACK.current(), vec![5]))
        );
        // The plain interact after an interact-at is dropped
        assert_eq!(interact_to_current(&[5, 0, 0, 0], V1_21_11), None);
    }

    #[test]
    fn sneak_command_becomes_player_input() {
        // Entity 5, start / stop sneaking, no jump boost
        assert_eq!(
            player_command_to_current(&[5, 0, 0], JavaMinecraftVersion::V_1_8),
            Some((play::PLAYER_INPUT.current(), vec![32]))
        );
        assert_eq!(
            player_command_to_current(&[5, 1, 0], JavaMinecraftVersion::V_1_21),
            Some((play::PLAYER_INPUT.current(), vec![0]))
        );
        // 1.7: i32 entity, u8 action, i32 jump boost
        assert_eq!(
            player_command_to_current(&[0, 0, 0, 5, 0, 0, 0, 0, 0], JavaMinecraftVersion::V_1_7_2),
            Some((play::PLAYER_INPUT.current(), vec![32]))
        );
        // PLAYER_INPUT already carries the shift flag
        assert_eq!(
            player_command_to_current(&[5, 0, 0], JavaMinecraftVersion::V_1_21_4),
            None
        );
    }

    #[test]
    fn legacy_player_input_flags() {
        let input = |payload: &[u8], version| {
            play_to_current(play::PLAYER_INPUT.current(), payload, version, || {
                (0.0, 0.0)
            })
        };
        // Sideways 0.98 (left), forward -0.98 (back)
        let floats = [0x3F, 0x7A, 0xE1, 0x48, 0xBF, 0x7A, 0xE1, 0x48];
        // 1.8+: one flags byte, jump 1, sneak 2
        assert_eq!(
            input(&[&floats[..], &[3]].concat(), JavaMinecraftVersion::V_1_21),
            Some(vec![2 | 4 | 16 | 32])
        );
        assert_eq!(
            input(&[&floats[..], &[2]].concat(), JavaMinecraftVersion::V_1_8),
            Some(vec![2 | 4 | 32])
        );
        // 1.7: jump and sneak booleans
        assert_eq!(
            input(
                &[&floats[..], &[0, 1]].concat(),
                JavaMinecraftVersion::V_1_7_2
            ),
            Some(vec![2 | 4 | 32])
        );
    }

    #[test]
    fn other_commands_shift_down() {
        // Start sprinting: 3 before 1.21.6, 1 in 26.3
        assert_eq!(
            player_command_to_current(&[5, 3, 0], JavaMinecraftVersion::V_1_21_4),
            Some((play::PLAYER_COMMAND.current(), vec![5, 1, 0]))
        );
        assert_eq!(
            player_command_to_current(&[5, 1, 0], V1_21_11),
            Some((play::PLAYER_COMMAND.current(), vec![5, 1, 0]))
        );
    }

    #[test]
    fn interact_at_becomes_interact() {
        let mut client = vec![5, 2];
        for value in [0.5f32, 1.0, -0.25] {
            client.write_f32_be(value).unwrap();
        }
        client.extend_from_slice(&[1, 1]);
        let (id, payload) = interact_to_current(&client, V1_21_11).unwrap();
        assert_eq!(id, play::INTERACT.current());
        let read = SInteract::read(
            &mut payload.as_slice(),
            &pumpkin_data::packet::CURRENT_MC_VERSION,
        )
        .unwrap();
        assert_eq!(read.entity_id, VarInt(5));
        assert_eq!(read.hand, Some(VarInt(1)));
        assert!(read.sneaking);
        let target = read.target_position.unwrap();
        assert!((target.x - 0.5).abs() < 0.01 && (target.z + 0.25).abs() < 0.01);
    }
}
