//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/login/encryption_request.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::login::CEncryptionRequest;
use pumpkin_protocol::ser::NetworkWriteExt;
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CEncryptionRequest<'_> {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), pumpkin_protocol::ser::WritingError> {
        write.write_string(self.server_id)?;
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i16_be(self.public_key.len() as i16)?;
        } else {
            write.write_var_int(&pumpkin_protocol::VarInt(self.public_key.len() as i32))?;
        }
        write.write_all(self.public_key)?;
        if *version <= JavaMinecraftVersion::V_1_7_6 {
            write.write_i16_be(self.verify_token.len() as i16)?;
        } else {
            write.write_var_int(&pumpkin_protocol::VarInt(self.verify_token.len() as i32))?;
        }
        write.write_all(self.verify_token)?;
        if version >= &JavaMinecraftVersion::V_1_20_5 {
            write.write_bool(self.should_authenticate)?;
        }
        Ok(())
    }
}
