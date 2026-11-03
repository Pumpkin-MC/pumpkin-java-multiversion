//! Paletted containers (block states and biomes in chunk sections).

use pumpkin_protocol::{
    VarInt,
    ser::{NetworkReadExt, NetworkWriteExt},
};
eras! {
    pub enum PaletteFormat {
        /// Long arrays are length-prefixed.
        V1_18 = V_1_18,
        /// No prefix; the client derives the length from the bits.
        V1_21_5 = V_1_21_5,
    }
}

/// What a container holds, and the client's global palette.
pub struct Container<F: Fn(u32) -> u32> {
    /// 4096 block states or 64 biomes.
    pub entries: usize,
    /// Above this the palette is the global registry.
    pub max_indirect_bits: u8,
    /// Client's global palette width.
    pub direct_bits: u8,
    /// 26.3 id to the client's.
    pub remap: F,
}

const fn long_count(entries: usize, bits: u8) -> usize {
    if bits == 0 {
        return 0;
    }
    let per_long = 64 / bits as usize;
    entries.div_ceil(per_long)
}

fn write_longs(out: &mut Vec<u8>, longs: &[i64], format: PaletteFormat) -> Option<()> {
    if format == PaletteFormat::V1_18 {
        out.write_var_int(&VarInt(longs.len() as i32)).ok()?;
    }
    for &long in longs {
        out.write_i64_be(long).ok()?;
    }
    Some(())
}

/// Rewrites one 26.3 container for the client's format. Returns the long count written.
pub fn translate<F: Fn(u32) -> u32>(
    read: &mut &[u8],
    out: &mut Vec<u8>,
    container: &Container<F>,
    format: PaletteFormat,
) -> Option<usize> {
    let bits = read.get_u8().ok()?;
    if bits == 0 {
        let id = read.get_var_int().ok()?.0 as u32;
        out.write_u8(0).ok()?;
        out.write_var_int(&VarInt((container.remap)(id) as i32))
            .ok()?;
        write_longs(out, &[], format)?;
        return Some(0);
    }

    let palette = if bits <= container.max_indirect_bits {
        Some(read_palette(read)?)
    } else {
        None
    };
    let longs = (0..long_count(container.entries, bits))
        .map(|_| read.get_i64_be().ok())
        .collect::<Option<Vec<_>>>()?;

    if let Some(palette) = palette {
        // Indices into the palette stay as they are.
        out.write_u8(bits).ok()?;
        out.write_var_int(&VarInt(palette.len() as i32)).ok()?;
        for id in palette {
            out.write_var_int(&VarInt((container.remap)(id) as i32))
                .ok()?;
        }
        write_longs(out, &longs, format)?;
        return Some(longs.len());
    }

    let repacked = repack(
        &longs,
        container.entries,
        bits,
        container.direct_bits,
        |id| (container.remap)(id),
    );
    out.write_u8(container.direct_bits).ok()?;
    write_longs(out, &repacked, format)?;
    Some(repacked.len())
}

fn read_palette(read: &mut &[u8]) -> Option<Vec<u32>> {
    let len = read.get_var_int().ok()?.0;
    let mut palette = Vec::with_capacity(len.max(0) as usize);
    for _ in 0..len {
        palette.push(read.get_var_int().ok()?.0 as u32);
    }
    Some(palette)
}

/// Unpacks `entries` values of `from_bits`, maps them and packs them with `to_bits`.
fn repack(
    longs: &[i64],
    entries: usize,
    from_bits: u8,
    to_bits: u8,
    map: impl Fn(u32) -> u32,
) -> Vec<i64> {
    let from_per_long = 64 / from_bits as usize;
    let to_per_long = 64 / to_bits as usize;
    let from_mask = (1u64 << from_bits) - 1;
    let to_mask = (1u64 << to_bits) - 1;
    let mut out = vec![0i64; long_count(entries, to_bits)];
    for index in 0..entries {
        let word = longs.get(index / from_per_long).copied().unwrap_or(0) as u64;
        let value = (word >> ((index % from_per_long) * from_bits as usize)) & from_mask;
        let mapped = u64::from(map(value as u32)) & to_mask;
        out[index / to_per_long] |= (mapped << ((index % to_per_long) * to_bits as usize)) as i64;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repack_widens_and_maps() {
        let values: Vec<u32> = (0..64).collect();
        let packed = repack(&pack(&values, 6), 64, 6, 7, |v| v + 1);
        let expected = pack(&values.iter().map(|v| v + 1).collect::<Vec<_>>(), 7);
        assert_eq!(packed, expected);
    }

    fn pack(values: &[u32], bits: u8) -> Vec<i64> {
        let per_long = 64 / bits as usize;
        let mut out = vec![0i64; values.len().div_ceil(per_long)];
        for (i, &v) in values.iter().enumerate() {
            out[i / per_long] |= (u64::from(v) << ((i % per_long) * bits as usize)) as i64;
        }
        out
    }
}
