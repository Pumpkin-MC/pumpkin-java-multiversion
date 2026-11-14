//! Pre-26.3 encodings of `CPlayerInfoUpdate`; `pumpkin-protocol` already writes them per version.

use crate::legacy::LegacyWrite;
use pumpkin_protocol::ClientPacket;
use pumpkin_protocol::java::client::play::CPlayerInfoUpdate;
use pumpkin_protocol::ser::WritingError;
use pumpkin_util::version::JavaMinecraftVersion;
use std::io::Write;

impl LegacyWrite for CPlayerInfoUpdate<'_> {
    fn write_legacy(
        &self,
        write: impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        self.write_packet_data(write, version)
    }
}
