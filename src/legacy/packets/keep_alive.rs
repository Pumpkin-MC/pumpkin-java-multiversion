//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/keep_alive.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::VarInt;
use pumpkin_protocol::java::client::play::CKeepAlive;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CKeepAlive {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_1_12_2 {
            write.write_i64_be(self.keep_alive_id)?;
        } else if *version >= JavaMinecraftVersion::V_1_8 {
            write.write_var_int(&VarInt(self.keep_alive_id as i32))?;
        } else {
            write.write_i32_be(self.keep_alive_id as i32)?;
        }
        Ok(())
    }
}
