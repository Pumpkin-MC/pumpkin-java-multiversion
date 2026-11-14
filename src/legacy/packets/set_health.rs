//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/set_health.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::play::CSetHealth;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CSetHealth {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_f32_be(self.health)?;
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i16_be(self.food.0 as i16)?;
        } else {
            write.write_var_int(&self.food)?;
        }
        write.write_f32_be(self.food_saturation)?;
        Ok(())
    }
}
