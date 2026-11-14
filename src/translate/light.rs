//! Light data: LIGHT_UPDATE and the tail of LEVEL_CHUNK_WITH_LIGHT.

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    VarInt,
    codec::bit_set::BitSet,
    java::client::play::LightData,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

/// Reads 26.3 light data.
pub fn decode_current_light(mut light: &[u8]) -> Option<LightData> {
    let mut data = LightData::read(&mut light, &CURRENT_MC_VERSION).ok()?;
    data.trust_edges = true;
    Some(data)
}

fn shift_mask_and_arrays(
    mask: &BitSet,
    empty_mask: &BitSet,
    arrays: &[Vec<u8>],
    base_section: usize,
) -> (BitSet, BitSet, Vec<Vec<u8>>) {
    let mut dst_mask = 0u64;
    let mut dst_empty = 0u64;
    let mut dst_arrays = Vec::new();
    let mut src_idx = 0usize;
    let max_bits = mask.0.len() * 64;
    for bit in 0..max_bits {
        if mask.get_bit(bit) {
            if let Some(arr) = arrays.get(src_idx)
                && bit >= base_section
                && bit - base_section < 18
            {
                dst_mask |= 1u64 << (bit - base_section);
                dst_arrays.push(arr.clone());
            }
            src_idx += 1;
        }
    }
    for dst_bit in 0..18 {
        if (dst_mask & (1u64 << dst_bit)) == 0 {
            let src_bit = dst_bit + base_section;
            if src_bit >= empty_mask.0.len() * 64 || empty_mask.get_bit(src_bit) {
                dst_empty |= 1u64 << dst_bit;
            }
        }
    }
    (
        BitSet::from_u64(dst_mask),
        BitSet::from_u64(dst_empty),
        dst_arrays,
    )
}

/// Shifts a 26.3 `LightData` to the 18-bit (`Y = -1..=16`) layout expected before 1.18.
#[must_use]
pub fn shift_light_for_legacy_height(light: &LightData, base_section: usize) -> LightData {
    let (sky_light_mask, empty_sky_light_mask, sky_light_arrays) = shift_mask_and_arrays(
        &light.sky_light_mask,
        &light.empty_sky_light_mask,
        &light.sky_light_arrays,
        base_section,
    );
    let (block_light_mask, empty_block_light_mask, block_light_arrays) = shift_mask_and_arrays(
        &light.block_light_mask,
        &light.empty_block_light_mask,
        &light.block_light_arrays,
        base_section,
    );
    LightData {
        trust_edges: true,
        sky_light_mask,
        block_light_mask,
        empty_sky_light_mask,
        empty_block_light_mask,
        sky_light_arrays,
        block_light_arrays,
    }
}

/// Unpacks per-section 2048-byte nibble arrays for `0..16` (starting at `base_section`).
#[must_use]
pub fn inline_section_lights(
    light: &LightData,
    base_section: usize,
) -> ([[u8; 2048]; 16], [[u8; 2048]; 16]) {
    let mut block_out = [[0u8; 2048]; 16];
    let mut sky_out = [[0xFFu8; 2048]; 16];

    let mut idx = 0usize;
    for bit in 0..light.block_light_mask.0.len() * 64 {
        if light.block_light_mask.get_bit(bit) {
            if let Some(arr) = light.block_light_arrays.get(idx)
                && arr.len() == 2048
                && bit > base_section
                && bit < base_section + 17
            {
                block_out[bit - (base_section + 1)].copy_from_slice(arr);
            }
            idx += 1;
        }
    }

    for (i, sky_sec) in sky_out.iter_mut().enumerate() {
        let bit = base_section + 1 + i;
        if bit < light.empty_sky_light_mask.0.len() * 64 && light.empty_sky_light_mask.get_bit(bit)
        {
            sky_sec.fill(0);
        }
    }
    let mut idx = 0usize;
    for bit in 0..light.sky_light_mask.0.len() * 64 {
        if light.sky_light_mask.get_bit(bit) {
            if let Some(arr) = light.sky_light_arrays.get(idx)
                && arr.len() == 2048
                && bit > base_section
                && bit < base_section + 17
            {
                sky_out[bit - (base_section + 1)].copy_from_slice(arr);
            }
            idx += 1;
        }
    }

    (block_out, sky_out)
}

/// Current light data (four masks, sky and block arrays) for `version`.
pub fn write_light_data(
    light: &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let data = decode_current_light(light)?;
    if version < JavaMinecraftVersion::V_1_18 {
        let max_bits = data
            .sky_light_mask
            .0
            .len()
            .max(data.empty_sky_light_mask.0.len())
            * 64;
        let has_extended = (18..max_bits).any(|b| {
            (b < data.sky_light_mask.0.len() * 64 && data.sky_light_mask.get_bit(b))
                || (b < data.empty_sky_light_mask.0.len() * 64
                    && data.empty_sky_light_mask.get_bit(b))
                || (b < data.block_light_mask.0.len() * 64 && data.block_light_mask.get_bit(b))
                || (b < data.empty_block_light_mask.0.len() * 64
                    && data.empty_block_light_mask.get_bit(b))
        });
        let shifted = shift_light_for_legacy_height(&data, if has_extended { 4 } else { 0 });
        shifted.write(out, &version).ok()
    } else {
        data.write(out, &version).ok()
    }
}

/// Standalone `LIGHT_UPDATE` payload for 1.14 - 1.17.1 chunks.
pub fn write_chunk_light_packet(
    chunk_x: i32,
    chunk_z: i32,
    base_section: usize,
    light: &LightData,
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    out.write_var_int(&VarInt(chunk_x)).ok()?;
    out.write_var_int(&VarInt(chunk_z)).ok()?;
    let shifted = shift_light_for_legacy_height(light, base_section);
    shifted.write(&mut out, &version).ok()?;
    Some(out)
}

/// LIGHT_UPDATE: chunk x, z, light data.
pub fn light_update_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let x = payload.get_var_int().ok()?;
    let z = payload.get_var_int().ok()?;
    let mut out = Vec::new();
    out.write_var_int(&x).ok()?;
    out.write_var_int(&z).ok()?;
    write_light_data(payload, version, &mut out)?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_become_long_arrays_before_26_3() {
        let mask = BitSet(Box::new([0b1011_0000_0001]));
        let mut light = Vec::new();
        for _ in 0..4 {
            mask.encode_with_version(&mut light, &CURRENT_MC_VERSION)
                .unwrap();
        }
        let masks_len = light.len();
        // one sky array, no block arrays
        light.write_var_int(&VarInt(1)).unwrap();
        light.write_var_int(&VarInt(2048)).unwrap();
        light.extend_from_slice(&[0x11; 2048]);
        light.write_var_int(&VarInt(0)).unwrap();

        let mut expected = Vec::new();
        for _ in 0..4 {
            crate::legacy::write_bit_set_legacy(
                &mut expected,
                &mask,
                &JavaMinecraftVersion::V_1_21_11,
            )
            .unwrap();
        }
        expected.extend_from_slice(&light[masks_len..]);

        let mut out = Vec::new();
        write_light_data(&light, JavaMinecraftVersion::V_1_21_11, &mut out).unwrap();
        assert_eq!(out, expected);
    }
}
