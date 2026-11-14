//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/open_sign_editor.rs`.

use crate::legacy::LegacyWrite;
use crate::legacy::LegacyWriteExt;
use pumpkin_protocol::java::client::play::COpenSignEditor;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for COpenSignEditor {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i32_be(self.location.0.x)?;
            write.write_i32_be(self.location.0.y)?;
            write.write_i32_be(self.location.0.z)?;
        } else {
            write.write_block_pos_legacy(&self.location, version)?;
        }
        if *version >= JavaMinecraftVersion::V_1_20 {
            write.write_bool(self.is_front_text)?;
        }
        Ok(())
    }
}
