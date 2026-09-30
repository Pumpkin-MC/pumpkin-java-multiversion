//! Encodings of core's packets for versions before 26.3. Core's `pumpkin-protocol` only
//! reads and writes the current version; these are the older layouts it no longer has.

pub mod add_resource_pack;
pub mod commands_write;
pub mod encryption_request;
pub mod encryption_response;
pub mod encryption_response_write;
pub mod entity_animation;
pub mod entity_position_sync;
pub mod entity_sound_effect;
pub mod entity_velocity;
pub mod explode;
pub mod interact;
pub mod login;
pub mod login_success;
pub mod particle;
pub mod player_info_update;
pub mod player_position;
pub mod player_rotation;
pub mod player_spawn_data;
pub mod player_spawn_position;
pub mod respawn;
pub mod set_player_team;
pub mod set_time;
pub mod sound_effect;
pub mod spawn_entity;
pub mod update_advancement_write;
pub mod update_attributes;
pub mod update_entity_pos;
pub mod update_entity_pos_rot;
pub mod update_entity_rot;

use std::io::Write;

use pumpkin_protocol::{
    VarInt,
    ser::{NetworkReadExt, NetworkWriteExt, ReadingError, WritingError},
};
use pumpkin_util::{text::TextComponent, version::JavaMinecraftVersion};

/// A core packet written for an older client.
pub trait LegacyWrite {
    fn write_legacy(
        &self,
        write: impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError>;
}

/// A core packet read from an older client.
pub trait LegacyRead<'a>: Sized {
    fn read_legacy(
        read: &mut &'a [u8],
        version: &JavaMinecraftVersion,
    ) -> Result<Self, ReadingError>;
}

/// Field encodings that changed before 26.3.
pub trait LegacyWriteExt: NetworkWriteExt {
    /// JSON text before 1.20.3, NBT since.
    fn write_component_legacy(
        &mut self,
        component: &TextComponent,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version < JavaMinecraftVersion::V_1_20_3 {
            let json = component.to_json_for_version(version);
            let max_len = if *version >= JavaMinecraftVersion::V_1_13 {
                262_144
            } else {
                32767
            };
            self.write_string_bounded(&json, max_len)
        } else {
            self.write_slice(&component.encode_for_version(version))
        }
    }

    /// A byte before 1.21.2, a var int since.
    fn write_container_id_legacy(
        &mut self,
        container_id: &VarInt,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_1_21_2 {
            self.write_var_int(container_id)
        } else {
            self.write_u8(container_id.0 as u8)
        }
    }
}

impl<W: Write> LegacyWriteExt for W {}

pub trait LegacyReadExt: NetworkReadExt {
    /// A byte before 1.21.2, a var int since.
    fn get_container_id_legacy(
        &mut self,
        version: &JavaMinecraftVersion,
    ) -> Result<VarInt, ReadingError> {
        if *version >= JavaMinecraftVersion::V_1_21_2 {
            self.get_var_int()
        } else {
            Ok(VarInt(i32::from(self.get_u8()?)))
        }
    }
}

impl<R: NetworkReadExt> LegacyReadExt for R {}

/// Equipment slot ids until 1.8: no off hand, so every armour slot is one lower.
#[must_use]
pub const fn slot_to_version(slot: i8, version: &JavaMinecraftVersion) -> i8 {
    if (*version as u8) <= (JavaMinecraftVersion::V_1_8 as u8) {
        match slot {
            0 => 0,
            2 => 1,
            3 => 2,
            4 => 3,
            5 => 4,
            _ => slot,
        }
    } else {
        slot
    }
}

/// Bit sets were a var int count of big endian longs before 26.3.
pub fn write_bit_set_legacy(
    mut write: impl Write,
    bit_set: &pumpkin_protocol::codec::bit_set::BitSet,
    version: &JavaMinecraftVersion,
) -> Result<(), WritingError> {
    if *version >= JavaMinecraftVersion::V_26_3 {
        return bit_set.encode_with_version(&mut write, version);
    }
    write.write_var_int(&VarInt(bit_set.0.len() as i32))?;
    for word in &bit_set.0 {
        write.write_i64_be(*word)?;
    }
    Ok(())
}

const LEGACY_VELOCITY_CLAMP: f64 = 3.9;
const LEGACY_VELOCITY_SCALE: f64 = 8000.0;

/// Velocity component before 1.21.9: clamped, times 8000, as a short.
#[must_use]
pub fn encode_legacy_velocity_component(component: f64) -> i16 {
    (component.clamp(-LEGACY_VELOCITY_CLAMP, LEGACY_VELOCITY_CLAMP) * LEGACY_VELOCITY_SCALE) as i16
}

/// Velocity before 1.21.9: three shorts.
pub fn write_legacy_velocity(
    mut write: impl Write,
    velocity: &pumpkin_util::math::vector3::Vector3<f64>,
) -> Result<(), WritingError> {
    write.write_i16_be(encode_legacy_velocity_component(velocity.x))?;
    write.write_i16_be(encode_legacy_velocity_component(velocity.y))?;
    write.write_i16_be(encode_legacy_velocity_component(velocity.z))
}
