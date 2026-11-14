//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/set_held_item.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::VarInt;
use pumpkin_protocol::java::client::play::CSetSelectedSlot;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CSetSelectedSlot {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_1_21_4 {
            write.write_var_int(&VarInt(i32::from(self.slot)))?;
        } else {
            write.write_i8(self.slot)?;
        }
        Ok(())
    }
}
