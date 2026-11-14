//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/update_mob_effect.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::play::CUpdateMobEffect;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CUpdateMobEffect {
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
        if *version >= JavaMinecraftVersion::V_1_20_5 {
            write.write_var_int(&self.amplifier)?;
        } else {
            write.write_i8(self.amplifier.0 as i8)?;
        }
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i16_be(self.duration.0 as i16)?;
        } else {
            write.write_var_int(&self.duration)?;
        }
        if *version > JavaMinecraftVersion::V_1_7_6 {
            write.write_i8(self.flags)?;
        }
        // factor data flag
        if *version >= JavaMinecraftVersion::V_1_19 && *version < JavaMinecraftVersion::V_1_20_5 {
            write.write_bool(false)?;
        }
        Ok(())
    }
}
