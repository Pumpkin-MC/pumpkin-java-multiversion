//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/display_objective.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::play::CDisplayObjective;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CDisplayObjective {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_1_20_2 {
            write.write_var_int(&self.position)?;
        } else {
            write.write_i8(self.position.0 as i8)?;
        }
        if *version >= JavaMinecraftVersion::V_1_18 {
            write.write_string(&self.score_name)?;
        } else {
            write.write_string_bounded(&self.score_name, 16)?;
        }
        Ok(())
    }
}
