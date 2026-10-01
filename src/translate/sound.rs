//! Sounds: SOUND, SOUND_ENTITY and STOP_SOUND.

use pumpkin_protocol::java::legacy::LegacyWrite;
use pumpkin_protocol::{
    IdOr, SoundEvent, VarInt,
    java::client::play::{CEntitySoundEffect, CSoundEffect},
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::{math::vector3::Vector3, version::JavaMinecraftVersion};

use crate::remap::sound_id_remap::remap_sound_id_for_version;

eras! {
    pub enum SoundFormat {
        /// Sounds by name, which core looks up with the 26.3 id.
        V1_7 = V_1_7_2,
        /// Sounds by registry id.
        V1_9 = V_1_9,
    }
}

eras! {
    pub enum SoundSourceFormat {
        /// No `ui` source.
        V1_7 = V_1_7_2,
        /// `ui` source added; same as 26.3.
        V1_21_6 = V_1_21_6,
    }
}

const UI_SOURCE: i32 = 10;

/// A 26.3 sound holder with the client's sound id.
pub fn read_sound(read: &mut &[u8], version: JavaMinecraftVersion) -> Option<IdOr<SoundEvent>> {
    let sound = IdOr::read(read, |read| {
        Ok(SoundEvent {
            sound_name: read.get_str()?.into(),
            range: read.get_option(NetworkReadExt::get_f32_be)?,
        })
    })
    .ok()?;
    Some(match sound {
        IdOr::Id(id) if SoundFormat::of(version) >= SoundFormat::V1_9 => {
            IdOr::Id(remap_sound_id_for_version(id, version))
        }
        sound => sound,
    })
}

/// The sound source, `ui` becoming `master` before 1.21.6.
fn read_source(read: &mut &[u8], version: JavaMinecraftVersion) -> Option<VarInt> {
    let source = read.get_var_int().ok()?;
    Some(
        if source.0 == UI_SOURCE && SoundSourceFormat::of(version) < SoundSourceFormat::V1_21_6 {
            VarInt(0)
        } else {
            source
        },
    )
}

fn write(packet: &impl LegacyWrite, version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    packet.write_legacy(&mut out, &version).ok()?;
    Some(out)
}

/// SOUND.
pub fn sound_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let packet = CSoundEffect {
        sound_event: read_sound(&mut payload, version)?,
        sound_category: read_source(&mut payload, version)?,
        position: Vector3::new(
            payload.get_i32_be().ok()?,
            payload.get_i32_be().ok()?,
            payload.get_i32_be().ok()?,
        ),
        volume: payload.get_f32_be().ok()?,
        pitch: payload.get_f32_be().ok()?,
        seed: payload.get_i64_be().ok()?,
    };
    write(&packet, version)
}

/// SOUND_ENTITY.
pub fn sound_entity_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let packet = CEntitySoundEffect {
        sound_event: read_sound(&mut payload, version)?,
        sound_category: read_source(&mut payload, version)?,
        entity_id: payload.get_var_int().ok()?,
        volume: payload.get_f32_be().ok()?,
        pitch: payload.get_f32_be().ok()?,
        seed: payload.get_i64_be().ok()?,
    };
    write(&packet, version)
}

/// STOP_SOUND: flags, then the source when bit 0 is set.
pub fn stop_sound_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let flags = payload.get_u8().ok()?;
    let mut out = vec![flags];
    if flags & 1 != 0 {
        out.write_var_int(&read_source(&mut payload, version)?)
            .ok()?;
    }
    out.extend_from_slice(payload);
    Some(out)
}

#[cfg(test)]
mod tests {
    use pumpkin_data::{packet::CURRENT_MC_VERSION, sound::SoundCategory};

    use super::*;

    #[test]
    fn sound_id_is_remapped() {
        let packet = CSoundEffect::new(
            IdOr::Id(500),
            SoundCategory::Blocks,
            &Vector3::new(1.0, 2.0, 3.0),
            1.0,
            0.5,
            7,
        );
        let current = write(&packet, CURRENT_MC_VERSION).unwrap();
        let version = JavaMinecraftVersion::V_1_21_11;
        let remapped = CSoundEffect {
            sound_event: IdOr::Id(remap_sound_id_for_version(500, version)),
            ..packet
        };
        assert_eq!(
            sound_from_current(&current, version),
            write(&remapped, version)
        );
    }
}
