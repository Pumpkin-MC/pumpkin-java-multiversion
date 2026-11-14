//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/head_rot.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::play::CHeadRot;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CHeadRot {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i32_be(self.entity_id.0)?;
        } else {
            write.write_var_int(&self.entity_id)?;
        }
        write.write_u8(self.head_yaw)?;
        Ok(())
    }
}
