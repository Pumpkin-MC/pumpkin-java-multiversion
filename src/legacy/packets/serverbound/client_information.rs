//! Pre-26.3 layouts from `pumpkin-protocol`'s `java/server/play/client_information.rs`.

use crate::legacy::LegacyRead;
use pumpkin_protocol::VarInt;
use pumpkin_protocol::java::server::play::SClientInformationPlay;
use pumpkin_protocol::ser::{NetworkReadExt, NetworkReadSliceExt, ReadingError};
use pumpkin_util::version::JavaMinecraftVersion;

impl<'a> LegacyRead<'a> for SClientInformationPlay<'a> {
    fn read_legacy(
        read: &mut &'a [u8],
        version: &JavaMinecraftVersion,
    ) -> Result<Self, ReadingError> {
        let locale = read.get_str_borrowed()?;
        let view_distance = read.get_i8()?;
        let chat_mode = read.get_var_int()?;
        let chat_colors = read.get_bool()?;
        let skin_parts = read.get_u8()?;
        let main_hand = if *version >= JavaMinecraftVersion::V_1_9 {
            read.get_var_int()?
        } else {
            VarInt(1)
        };
        let text_filtering = *version >= JavaMinecraftVersion::V_1_17 && read.get_bool()?;
        let server_listing = *version < JavaMinecraftVersion::V_1_18 || read.get_bool()?;
        Ok(Self {
            locale,
            view_distance,
            chat_mode,
            chat_colors,
            skin_parts,
            main_hand,
            text_filtering,
            server_listing,
        })
    }
}
