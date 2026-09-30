//! Resource pack push (clientbound) and response (serverbound).

use crate::legacy::LegacyWrite;
use pumpkin_protocol::{
    java::client::config::CConfigAddResourcePack,
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

eras! {
    pub enum ResponseFormat {
        /// Hash, result.
        V1_7 = V_1_7_2,
        /// Result only.
        V1_10 = V_1_10,
        /// UUID, result; same as 26.3.
        V1_20_3 = V_1_20_3,
    }
}

/// RESOURCE_PACK response: a nil UUID for clients that send none.
pub fn response_to_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    match ResponseFormat::of(version) {
        ResponseFormat::V1_7 => {
            let _ = payload.get_str_borrowed();
        }
        ResponseFormat::V1_10 => {}
        ResponseFormat::V1_20_3 => return None,
    }
    let result = payload.get_var_int().ok()?;
    let mut out = Vec::new();
    let _ = out.write_uuid(&uuid::Uuid::nil());
    let _ = out.write_var_int(&result);
    Some(out)
}

/// RESOURCE_PACK_PUSH (UUID only from 1.20.3).
pub fn push_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let uuid = payload.get_uuid().ok()?;
    let url = payload.get_str_borrowed().ok()?;
    let hash = payload.get_str_borrowed().ok()?;
    let forced = payload.get_bool().ok()?;
    let prompt_message = if payload.get_bool().ok()? {
        Some(payload.get_component().ok()?)
    } else {
        None
    };
    let packet = CConfigAddResourcePack::new(&uuid, url, hash, forced, prompt_message);
    let mut out = Vec::new();
    packet.write_legacy(&mut out, &version).ok()?;
    Some(out)
}
