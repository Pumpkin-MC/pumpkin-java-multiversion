//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/entity_position_sync.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::VarInt;
use pumpkin_protocol::java::client::play::CEntityPositionSync;
use pumpkin_protocol::ser::NetworkWriteExt;
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CEntityPositionSync {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), pumpkin_protocol::ser::WritingError> {
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i32_be(self.entity_id.0)?;
        } else {
            write.write_var_int(&self.entity_id)?;
        }
        // Since 26.3 the position is a path. 0 is a linear path, which is just the end position.
        if *version >= JavaMinecraftVersion::V_26_3 {
            write.write_var_int(&VarInt(0))?;
        }
        if *version >= JavaMinecraftVersion::V_1_9 {
            write.write_f64_be(self.position.x)?;
            write.write_f64_be(self.position.y)?;
            write.write_f64_be(self.position.z)?;
        } else {
            write.write_i32_be((self.position.x * 32.0).floor() as i32)?;
            write.write_i32_be((self.position.y * 32.0).floor() as i32)?;
            write.write_i32_be((self.position.z * 32.0).floor() as i32)?;
        }
        if *version >= JavaMinecraftVersion::V_1_21_2 {
            // The delta was replaced by the path in 26.3.
            if *version < JavaMinecraftVersion::V_26_3 {
                write.write_f64_be(self.delta.x)?;
                write.write_f64_be(self.delta.y)?;
                write.write_f64_be(self.delta.z)?;
            }
            write.write_f32_be(self.yaw)?;
            write.write_f32_be(self.pitch)?;
        } else {
            write.write_u8((self.yaw.rem_euclid(360.0) * 256.0 / 360.0).floor() as u8)?;
            write.write_u8((self.pitch.rem_euclid(360.0) * 256.0 / 360.0).floor() as u8)?;
        }
        if *version >= JavaMinecraftVersion::V_1_8 {
            write.write_bool(self.on_ground)?;
        }
        Ok(())
    }
}
