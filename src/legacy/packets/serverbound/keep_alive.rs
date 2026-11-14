//! Pre-26.3 layouts from `pumpkin-protocol`'s `java/server/play/keep_alive.rs`.

use crate::legacy::LegacyRead;
use pumpkin_protocol::java::server::play::SKeepAlive;
use pumpkin_protocol::ser::{NetworkReadExt, ReadingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl<'a> LegacyRead<'a> for SKeepAlive {
    fn read_legacy(
        read: &mut &'a [u8],
        version: &JavaMinecraftVersion,
    ) -> Result<Self, ReadingError> {
        let keep_alive_id = if *version >= JavaMinecraftVersion::V_1_12_2 {
            read.get_i64_be()?
        } else if *version >= JavaMinecraftVersion::V_1_8 {
            i64::from(read.get_var_int()?.0)
        } else {
            i64::from(read.get_i32_be()?)
        };
        Ok(Self { keep_alive_id })
    }
}
