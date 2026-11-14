//! Pre-26.3 layouts from `pumpkin-protocol`'s `java/server/play/player_position_rotation.rs`.

use crate::legacy::LegacyRead;
use pumpkin_protocol::java::server::play::SPlayerPositionRotation;
use pumpkin_protocol::ser::{NetworkReadExt, ReadingError};
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::version::JavaMinecraftVersion;

impl<'a> LegacyRead<'a> for SPlayerPositionRotation {
    fn read_legacy(
        read: &mut &'a [u8],
        version: &JavaMinecraftVersion,
    ) -> Result<Self, ReadingError> {
        let x = read.get_f64_be()?;
        let y = read.get_f64_be()?;
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            let _stance = read.get_f64_be()?;
        }
        let z = read.get_f64_be()?;
        let yaw = read.get_f32_be()?;
        let pitch = read.get_f32_be()?;
        let collision = read.get_u8()?;
        Ok(Self {
            position: Vector3::new(x, y, z),
            yaw,
            pitch,
            collision,
        })
    }
}
