//! Serverbound play packets from older clients to 26.3.

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    ClientPacket, ServerPacket, VarInt,
    java::server::play::{ActionType, SInteract},
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::packet::mappings::serverbound::play;

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
        /// Sideways and forward floats, jump and sneak flags.
        V1_7 = V_1_7_2,
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
    let interact = SInteract::read(&mut payload, &version).ok()?;
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
            .write_packet_data(&mut out, &CURRENT_MC_VERSION)
            .ok()?;
            Some((play::INTERACT.current(), out))
        }
    }
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
        let pos_val = payload.get_i64_be().ok()?;
        let is_front = if SignFormat::of(version) == SignFormat::V1_20 {
            payload.get_bool().unwrap_or(true)
        } else {
            true
        };
        let line1 = payload.get_str_borrowed().unwrap_or("");
        let line2 = payload.get_str_borrowed().unwrap_or("");
        let line3 = payload.get_str_borrowed().unwrap_or("");
        let line4 = payload.get_str_borrowed().unwrap_or("");
        let mut out = Vec::new();
        let _ = out.write_i64_be(pos_val);
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
        let pos_val = payload.get_i64_be().ok()?;
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
        let _ = out.write_i64_be(pos_val);
        let _ = out.write_u8(face);
        let _ = out.write_var_int(&sequence);
        return Some(out);
    }

    // PLAYER_COMMAND
    let command = PlayerCommandFormat::of(version);
    if new_id == play::PLAYER_COMMAND.current() && command != PlayerCommandFormat::V1_21_6 {
        let var_ints = command == PlayerCommandFormat::V1_8;
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
        let modern_action = if action_id.0 >= 2 {
            VarInt(action_id.0 - 2)
        } else {
            return Some(Vec::new());
        };
        let mut out = Vec::new();
        let _ = out.write_var_int(&entity_id);
        let _ = out.write_var_int(&modern_action);
        let _ = out.write_var_int(&jump_boost);
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
    if new_id == play::PLAYER_INPUT.current()
        && PlayerInputFormat::of(version) == PlayerInputFormat::V1_7
    {
        let sideways = payload.get_f32_be().unwrap_or(0.0);
        let forward = payload.get_f32_be().unwrap_or(0.0);
        let jumping = payload.get_bool().unwrap_or(false);
        let sneaking = payload.get_bool().unwrap_or(false);
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
    fn interact_at_becomes_interact() {
        let mut client = vec![5, 2];
        for value in [0.5f32, 1.0, -0.25] {
            client.write_f32_be(value).unwrap();
        }
        client.extend_from_slice(&[1, 1]);
        let (id, payload) = interact_to_current(&client, V1_21_11).unwrap();
        assert_eq!(id, play::INTERACT.current());
        let read = SInteract::read(&mut payload.as_slice(), &CURRENT_MC_VERSION).unwrap();
        assert_eq!(read.entity_id, VarInt(5));
        assert_eq!(read.hand, Some(VarInt(1)));
        assert!(read.sneaking);
        let target = read.target_position.unwrap();
        assert!((target.x - 0.5).abs() < 0.01 && (target.z + 0.25).abs() < 0.01);
    }
}
