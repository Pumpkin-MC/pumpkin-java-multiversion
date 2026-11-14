//! Pre-26.3 layouts from `pumpkin-protocol`'s `java/server/play/player_abilities.rs`.

use crate::legacy::LegacyRead;
use pumpkin_protocol::java::server::play::SPlayerAbilities;
use pumpkin_protocol::ser::{NetworkReadExt, ReadingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl<'a> LegacyRead<'a> for SPlayerAbilities {
    fn read_legacy(
        read: &mut &'a [u8],
        version: &JavaMinecraftVersion,
    ) -> Result<Self, ReadingError> {
        let flags = read.get_i8()?;
        let (fly_speed, walk_speed) = if *version < JavaMinecraftVersion::V_1_16 {
            (Some(read.get_f32_be()?), Some(read.get_f32_be()?))
        } else {
            (None, None)
        };
        Ok(Self {
            flags,
            fly_speed,
            walk_speed,
        })
    }
}
