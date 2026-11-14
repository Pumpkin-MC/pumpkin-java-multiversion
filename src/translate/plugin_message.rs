//! Custom payload channel names and 1.7 length framing.

use pumpkin_protocol::ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt};
use pumpkin_util::version::JavaMinecraftVersion;

const CURRENT_BRAND: &str = "minecraft:brand";
const LEGACY_BRAND: &str = "MC|Brand";

/// `minecraft:brand` to `MC|Brand` before 1.13, with `u16` length framing on 1.7.
pub fn from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    if version >= JavaMinecraftVersion::V_1_13 {
        return None;
    }
    let mut reader = payload;
    let channel = reader.get_str_borrowed().ok()?;
    let legacy_channel = if channel == CURRENT_BRAND {
        LEGACY_BRAND
    } else if channel.len() <= 20 && !channel.contains(':') {
        channel
    } else {
        return None;
    };
    let mut out = Vec::with_capacity(legacy_channel.len() + 4 + reader.len());
    out.write_string(legacy_channel).ok()?;
    if version <= JavaMinecraftVersion::V_1_7_6 {
        let len = u16::try_from(reader.len()).ok()?;
        out.write_u16_be(len).ok()?;
    }
    out.extend_from_slice(reader);
    Some(out)
}

/// `MC|Brand` to `minecraft:brand`, stripping 1.7 `u16` length framing.
pub fn to_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    if version >= JavaMinecraftVersion::V_1_13 {
        return None;
    }
    let mut reader = payload;
    let channel = reader.get_str_borrowed().ok()?;
    if version <= JavaMinecraftVersion::V_1_7_6 {
        let len = usize::from(reader.get_u16_be().ok()?);
        reader = reader.get(..len)?;
    }
    let mapped_storage;
    let modern_channel = if channel == LEGACY_BRAND {
        CURRENT_BRAND
    } else if channel.contains(':') {
        channel
    } else {
        let sanitized: String = channel
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/') {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        mapped_storage = format!("legacy:{sanitized}");
        &mapped_storage
    };
    let mut out = Vec::with_capacity(modern_channel.len() + 2 + reader.len());
    out.write_string(modern_channel).ok()?;
    out.extend_from_slice(reader);
    Some(out)
}

/// Play `CUSTOM_PAYLOAD`: rewrites pre-1.13 channels and keeps 1.13+ payloads intact.
pub fn play_from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    if version >= JavaMinecraftVersion::V_1_13 {
        Some(payload.to_vec())
    } else {
        from_current(payload, version)
    }
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
