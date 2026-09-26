//! LEVEL_CHUNK_WITH_LIGHT: heightmaps, sections, block entities, light.

use pumpkin_nbt::{Nbt, compound::NbtCompound, tag::NbtTag};
use pumpkin_protocol::{
    VarInt,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use super::{
    biome::BiomeRemap,
    block,
    light::write_light_data,
    nbt::{split_network_nbt, write_network_nbt},
    palette::{self, Container, PaletteFormat},
};

eras! {
    pub enum ChunkFormat {
        /// Other layout, not translated yet.
        V1_7 = V_1_7_2,
        /// NBT heightmaps.
        V1_18 = V_1_18,
        /// Heightmap map; section buffer padded as if long arrays were length-prefixed.
        V1_21_5 = V_1_21_5,
        /// Padding dropped.
        V1_21_6 = V_1_21_6,
        /// Fluid count per section; same as 26.3.
        V26_1 = V_26_1,
    }
}

/// 26.3 chunk for `version`. `None` for `ChunkFormat::V1_7`.
// TODO: 1.7 - 1.17 chunks.
pub fn chunk_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let format = ChunkFormat::of(version);
    if format == ChunkFormat::V1_7 {
        return None;
    }
    let mut out = Vec::with_capacity(payload.len());
    out.write_i32_be(payload.get_i32_be().ok()?).ok()?;
    out.write_i32_be(payload.get_i32_be().ok()?).ok()?;

    heightmaps(&mut payload, format, version, &mut out)?;

    let data_len = payload.get_var_int().ok()?.0;
    let (data, rest) = payload.split_at_checked(usize::try_from(data_len).ok()?)?;
    payload = rest;
    let sections = sections(data, format, version)?;
    out.write_var_int(&VarInt(sections.len() as i32)).ok()?;
    out.extend_from_slice(&sections);

    block_entities(&mut payload, version, &mut out)?;
    write_light_data(payload, version, &mut out)?;
    Some(out)
}

/// 26.3 writes `(type, longs)` pairs; `ChunkFormat::V1_18` an NBT compound of long arrays.
fn heightmaps(
    read: &mut &[u8],
    format: ChunkFormat,
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let count = read.get_var_int().ok()?;
    let mut maps = Vec::with_capacity(count.0.max(0) as usize);
    for _ in 0..count.0 {
        let kind = read.get_var_int().ok()?;
        let len = read.get_var_int().ok()?.0;
        let longs = (0..len)
            .map(|_| read.get_i64_be().ok())
            .collect::<Option<Vec<_>>>()?;
        maps.push((kind, longs));
    }

    if format != ChunkFormat::V1_18 {
        out.write_var_int(&count).ok()?;
        for (kind, longs) in maps {
            out.write_var_int(&kind).ok()?;
            out.write_var_int(&VarInt(longs.len() as i32)).ok()?;
            for long in longs {
                out.write_i64_be(long).ok()?;
            }
        }
        return Some(());
    }

    let mut compound = NbtCompound::new();
    for (kind, longs) in maps {
        let name = match kind.0 {
            1 => "WORLD_SURFACE",
            4 => "MOTION_BLOCKING",
            5 => "MOTION_BLOCKING_NO_LEAVES",
            _ => continue,
        };
        compound.put(name, NbtTag::LongArray(longs));
    }
    write_network_nbt(&Nbt::from(compound).write_unnamed(), version, out);
    Some(())
}

/// Per section: block count, fluid count, block states, biomes.
fn sections(
    mut data: &[u8],
    format: ChunkFormat,
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let palette_format = PaletteFormat::of(version);
    let biome_remap = BiomeRemap::new(version);
    let blocks = Container {
        entries: 4096,
        max_indirect_bits: 8,
        direct_bits: block::direct_bits(version),
        remap: |id| block::remap_state(id, version),
    };
    let biomes = Container {
        entries: 64,
        max_indirect_bits: 3,
        direct_bits: biome_remap.direct_bits(),
        remap: |id| biome_remap.get(id),
    };

    let mut out = Vec::with_capacity(data.len());
    let mut storage_len_bytes = 0;
    while !data.is_empty() {
        out.write_i16_be(data.get_i16_be().ok()?).ok()?;
        let fluid_count = data.get_i16_be().ok()?;
        if format == ChunkFormat::V26_1 {
            out.write_i16_be(fluid_count).ok()?;
        }
        let block_longs = palette::translate(&mut data, &mut out, &blocks, palette_format)?;
        let biome_longs = palette::translate(&mut data, &mut out, &biomes, palette_format)?;
        storage_len_bytes +=
            VarInt(block_longs as i32).written_size() + VarInt(biome_longs as i32).written_size();
    }
    if format == ChunkFormat::V1_21_5 {
        out.resize(out.len() + storage_len_bytes, 0);
    }
    Some(out)
}

/// Packed xz, y, type, NBT.
fn block_entities(
    read: &mut &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let count = read.get_var_int().ok()?;
    out.write_var_int(&count).ok()?;
    for _ in 0..count.0 {
        out.write_u8(read.get_u8().ok()?).ok()?;
        out.write_i16_be(read.get_i16_be().ok()?).ok()?;
        let kind = read.get_var_int().ok()?.0 as u32;
        out.write_var_int(&VarInt(block::remap_block_entity_type(kind, version) as i32))
            .ok()?;
        write_network_nbt(split_network_nbt(read)?, version, out);
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ids<'a> {
        fluid_count: bool,
        block: &'a dyn Fn(u32) -> u32,
        biome: &'a dyn Fn(u32) -> u32,
        block_entity: &'a dyn Fn(u32) -> u32,
    }

    /// A chunk with a single-valued and an indirect section, one block entity and light.
    fn chunk(ids: &Ids) -> Vec<u8> {
        let mut data = Vec::new();
        for (blocks, biomes) in [(vec![1], vec![0]), (vec![1, 2000, 29000], vec![0, 66])] {
            data.write_i16_be(4096).unwrap();
            if ids.fluid_count {
                data.write_i16_be(0).unwrap();
            }
            for (ids_in, map, entries, bits) in [
                (blocks, ids.block, 4096usize, 4u8),
                (biomes, ids.biome, 64, 1),
            ] {
                if ids_in.len() == 1 {
                    data.write_u8(0).unwrap();
                    data.write_var_int(&VarInt(map(ids_in[0]) as i32)).unwrap();
                    continue;
                }
                data.write_u8(bits).unwrap();
                data.write_var_int(&VarInt(ids_in.len() as i32)).unwrap();
                for id in ids_in {
                    data.write_var_int(&VarInt(map(id) as i32)).unwrap();
                }
                for i in 0..entries.div_ceil(64 / bits as usize) {
                    data.write_i64_be(i as i64 * 0x0101).unwrap();
                }
            }
        }

        let mut out = Vec::new();
        out.write_i32_be(3).unwrap();
        out.write_i32_be(-4).unwrap();
        out.write_var_int(&VarInt(1)).unwrap();
        out.write_var_int(&VarInt(4)).unwrap();
        out.write_var_int(&VarInt(2)).unwrap();
        out.write_i64_be(7).unwrap();
        out.write_i64_be(8).unwrap();
        out.write_var_int(&VarInt(data.len() as i32)).unwrap();
        out.extend_from_slice(&data);

        let mut nbt = NbtCompound::new();
        nbt.put("Items", NbtTag::List(Vec::new()));
        out.write_var_int(&VarInt(1)).unwrap();
        out.write_u8(0x12).unwrap();
        out.write_i16_be(64).unwrap();
        out.write_var_int(&VarInt((ids.block_entity)(5) as i32))
            .unwrap();
        out.extend_from_slice(&Nbt::from(nbt).write_unnamed());

        // Light: four empty bit sets and no arrays
        out.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        out
    }

    #[test]
    fn chunk_1_21_11_drops_fluid_count_and_remaps_ids() {
        let version = JavaMinecraftVersion::V_1_21_11;
        let identity = |id| id;
        let payload = chunk(&Ids {
            fluid_count: true,
            block: &identity,
            biome: &identity,
            block_entity: &identity,
        });
        let biomes = BiomeRemap::new(version);
        let expected = chunk(&Ids {
            fluid_count: false,
            block: &|id| block::remap_state(id, version),
            biome: &|id| biomes.get(id),
            block_entity: &|id| block::remap_block_entity_type(id, version),
        });
        assert_eq!(chunk_from_current(&payload, version).unwrap(), expected);
        // 66 is the last 26.3 biome; 1.21.11 lacks it
        assert_ne!(biomes.get(66), 66);
    }
}
