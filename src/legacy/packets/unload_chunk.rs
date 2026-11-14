//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/unload_chunk.rs`.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::java::client::play::CUnloadChunk;
use pumpkin_protocol::ser::{NetworkWriteExt, WritingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CUnloadChunk {
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if *version >= JavaMinecraftVersion::V_1_20_2 {
            let chunk_key = (i64::from(self.z) << 32) | i64::from(self.x as u32);
            write.write_i64_be(chunk_key)?;
        } else {
            write.write_i32_be(self.x)?;
            write.write_i32_be(self.z)?;
        }
        Ok(())
    }
}
