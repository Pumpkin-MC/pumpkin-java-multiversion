//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/remove_mob_effect.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::play::CRemoveMobEffect;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CRemoveMobEffect {
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
        if *version >= JavaMinecraftVersion::V_1_18_2 {
            write.write_var_int(&self.effect_id)?;
        } else {
            write.write_u8(self.effect_id.0 as u8)?;
        }
        Ok(())
    }
}
