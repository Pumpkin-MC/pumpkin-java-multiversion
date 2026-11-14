//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/take_item.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::play::CTakeItemEntity;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CTakeItemEntity {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i32_be(self.entity_id.0)?;
            write.write_i32_be(self.collector_entity_id.0)?;
        } else {
            write.write_var_int(&self.entity_id)?;
            write.write_var_int(&self.collector_entity_id)?;
        }
        if *version >= JavaMinecraftVersion::V_1_11 {
            write.write_var_int(&self.stack_amount)?;
        }
        Ok(())
    }
}
