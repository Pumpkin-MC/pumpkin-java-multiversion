//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/change_difficulty.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::VarInt;
use pumpkin_protocol::java::client::play::CChangeDifficulty;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CChangeDifficulty {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_1_21_6 {
            write.write_var_int(&VarInt(i32::from(self.difficulty)))?;
        } else {
            write.write_u8(self.difficulty)?;
        }
        if *version >= JavaMinecraftVersion::V_1_14 {
            write.write_bool(self.locked)?;
        }
        Ok(())
    }
}
