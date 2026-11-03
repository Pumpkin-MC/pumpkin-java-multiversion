//! Config/login custom payload channel names.

use pumpkin_protocol::ser::{NetworkReadSliceExt, NetworkWriteExt};
use pumpkin_util::version::JavaMinecraftVersion;

const CURRENT_BRAND: &str = "minecraft:brand";
const LEGACY_BRAND: &str = "MC|Brand";

fn brand_channel(version: JavaMinecraftVersion) -> &'static str {
    if version < JavaMinecraftVersion::V_1_13 {
        LEGACY_BRAND
    } else {
        CURRENT_BRAND
    }
}

fn rewrite_channel(payload: &[u8], from: &str, to: &str) -> Option<Vec<u8>> {
    let mut reader = payload;
    let channel = reader.get_str_borrowed().ok()?;
    if channel != from {
        return None;
    }
    let mut out = Vec::with_capacity(to.len() + 2 + reader.len());
    out.write_string(to).ok()?;
    out.extend_from_slice(reader);
    Some(out)
}

/// `minecraft:brand` to `MC|Brand` before 1.13.
pub fn from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let to = brand_channel(version);
    if to == CURRENT_BRAND {
        return None;
    }
    rewrite_channel(payload, CURRENT_BRAND, to)
}

/// `MC|Brand` to `minecraft:brand`.
pub fn to_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    if version >= JavaMinecraftVersion::V_1_13 {
        return None;
    }
    rewrite_channel(payload, LEGACY_BRAND, CURRENT_BRAND)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_protocol::ser::NetworkReadSliceExt;

    fn payload(channel: &str, data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_string(channel).unwrap();
        buf.extend_from_slice(data);
        buf
    }

    #[test]
    fn brand_becomes_legacy_before_1_13() {
        let data = payload(CURRENT_BRAND, b"Pumpkin");
        let out = from_current(&data, JavaMinecraftVersion::V_1_12_2).unwrap();
        let mut reader = out.as_slice();
        assert_eq!(reader.get_str_borrowed().unwrap(), LEGACY_BRAND);
        assert_eq!(reader, b"Pumpkin");
    }

    #[test]
    fn brand_stays_on_1_13() {
        let data = payload(CURRENT_BRAND, b"Pumpkin");
        assert!(from_current(&data, JavaMinecraftVersion::V_1_13).is_none());
    }
}
