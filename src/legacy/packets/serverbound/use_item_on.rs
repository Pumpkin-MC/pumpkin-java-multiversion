//! Pre-26.3 layouts from `pumpkin-protocol`'s `java/server/play/use_item_on.rs`.

use crate::legacy::LegacyRead;
use crate::legacy::LegacyReadExt;
use pumpkin_protocol::VarInt;
use pumpkin_protocol::java::server::play::SUseItemOn;
use pumpkin_protocol::ser::{NetworkReadExt, ReadingError};
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::version::JavaMinecraftVersion;

impl<'a> LegacyRead<'a> for SUseItemOn {
    fn read_legacy(
        read: &mut &'a [u8],
        version: &JavaMinecraftVersion,
    ) -> Result<Self, ReadingError> {
        // 1.7 - 1.8: block placement with an item stack, translated by the caller.
        if *version < JavaMinecraftVersion::V_1_9 {
            return Err(ReadingError::Message(
                "1.7 - 1.8 block placement is translated by the caller".into(),
            ));
        }
        if *version >= JavaMinecraftVersion::V_1_14 {
            let hand = read.get_var_int()?;
            let position = read.get_block_pos_legacy(version)?;
            let face = read.get_var_int()?;
            let cursor_pos =
                Vector3::new(read.get_f32_be()?, read.get_f32_be()?, read.get_f32_be()?);
            let inside_block = read.get_bool()?;
            let is_against_world_border =
                *version >= JavaMinecraftVersion::V_1_21_2 && read.get_bool()?;
            let sequence = if *version >= JavaMinecraftVersion::V_1_19 {
                read.get_var_int()?
            } else {
                VarInt(0)
            };
            return Ok(Self {
                hand,
                position,
                face,
                cursor_pos,
                inside_block,
                is_against_world_border,
                sequence,
            });
        }
        let position = read.get_block_pos_legacy(version)?;
        let face = read.get_var_int()?;
        let hand = read.get_var_int()?;
        let cursor_pos = if *version >= JavaMinecraftVersion::V_1_11 {
            Vector3::new(read.get_f32_be()?, read.get_f32_be()?, read.get_f32_be()?)
        } else {
            Vector3::new(
                f32::from(read.get_u8()?) / 16.0,
                f32::from(read.get_u8()?) / 16.0,
                f32::from(read.get_u8()?) / 16.0,
            )
        };
        Ok(Self {
            hand,
            position,
            face,
            cursor_pos,
            inside_block: false,
            is_against_world_border: false,
            sequence: VarInt(0),
        })
    }
}
