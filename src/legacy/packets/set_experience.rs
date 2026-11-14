//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/set_experience.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::play::CSetExperience;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CSetExperience {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_f32_be(self.progress)?;
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i16_be(self.level.0 as i16)?;
            write.write_i16_be(self.total_experience.0 as i16)?;
        } else {
            write.write_var_int(&self.level)?;
            write.write_var_int(&self.total_experience)?;
        }
        Ok(())
    }
}
