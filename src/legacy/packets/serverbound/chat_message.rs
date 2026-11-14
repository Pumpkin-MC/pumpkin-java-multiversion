//! Pre-26.3 layouts from `pumpkin-protocol`'s `java/server/play/chat_message.rs`.

use crate::legacy::LegacyRead;
use pumpkin_protocol::VarInt;
use pumpkin_protocol::java::server::play::SChatMessage;
use pumpkin_protocol::ser::{NetworkReadExt, NetworkReadSliceExt, ReadingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl<'a> LegacyRead<'a> for SChatMessage<'a> {
    fn read_legacy(
        read: &mut &'a [u8],
        version: &JavaMinecraftVersion,
    ) -> Result<Self, ReadingError> {
        // 1.19 - 1.19.2: signed layout not mapped.
        if (JavaMinecraftVersion::V_1_19..JavaMinecraftVersion::V_1_19_3).contains(version) {
            return Err(ReadingError::Message(
                "1.19 - 1.19.2 chat layout not supported".into(),
            ));
        }
        let max_len = if *version >= JavaMinecraftVersion::V_1_11 {
            256
        } else {
            100
        };
        let message = read.get_str_bounded_borrowed(max_len)?;
        let mut packet = Self {
            message,
            timestamp: 0,
            salt: 0,
            signature: None,
            message_count: VarInt(0),
            acknowledged: &[],
            checksum: 0,
        };
        if *version >= JavaMinecraftVersion::V_1_19_3 {
            packet.timestamp = read.get_i64_be()?;
            packet.salt = read.get_i64_be()?;
            packet.signature = read.get_option(|read| read.read_slice_borrowed(256))?;
            packet.message_count = read.get_var_int()?;
            packet.acknowledged = read.read_slice_borrowed(3)?;
        }
        if *version >= JavaMinecraftVersion::V_1_21_5 {
            packet.checksum = read.get_u8()?;
        }
        Ok(packet)
    }
}
