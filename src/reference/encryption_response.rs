//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/server/login/encryption_response.rs`.

use super::ReferenceWrite;
use pumpkin_protocol::java::server::login::SEncryptionResponse;
use pumpkin_util::version::JavaMinecraftVersion;

impl ReferenceWrite for SEncryptionResponse {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), pumpkin_protocol::ser::WritingError> {
        use pumpkin_protocol::ser::NetworkWriteExt;
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i16_be(self.shared_secret.len() as i16)?;
            write.write_all(&self.shared_secret)?;
            write.write_i16_be(self.verify_token.len() as i16)?;
            write.write_all(&self.verify_token)?;
            return Ok(());
        }
        write.write_var_int(&pumpkin_protocol::VarInt(self.shared_secret.len() as i32))?;
        write.write_all(&self.shared_secret)?;
        if version >= &JavaMinecraftVersion::V_1_19_3 && version < &JavaMinecraftVersion::V_1_20_2 {
            write.write_bool(true)?;
        }
        write.write_var_int(&pumpkin_protocol::VarInt(self.verify_token.len() as i32))?;
        write.write_all(&self.verify_token)?;
        Ok(())
    }
}
