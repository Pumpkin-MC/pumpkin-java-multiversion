//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/set_entity_link.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::play::CSetEntityLink;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CSetEntityLink {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_i32_be(self.attached_entity_id)?;
        write.write_i32_be(self.holding_entity_id)?;
        if *version <= JavaMinecraftVersion::V_1_8 {
            write.write_bool(self.leash)?;
        }
        Ok(())
    }
}
