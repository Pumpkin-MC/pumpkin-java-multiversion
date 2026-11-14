//! LEVEL_CHUNK_WITH_LIGHT: heightmaps, sections, block entities, light.

use std::io::Write;

use pumpkin_nbt::{Nbt, compound::NbtCompound, tag::NbtTag};
use pumpkin_protocol::{
    VarInt,
    codec::bit_set::BitSet,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use super::{
    biome::BiomeRemap,
    block, ceil_log2,
    light::{
        decode_current_light, inline_section_lights, write_chunk_light_packet, write_light_data,
    },
    nbt::{split_network_nbt, write_network_nbt},
    palette::{self, Container, PaletteFormat},
};

eras! {
    pub enum ChunkFormat {
        /// Zlib-compressed split arrays (IDs, nibbles, light, add, biomes).
        V1_7 = V_1_7_2,
        /// Uncompressed little-endian `u16` block states + light + 256-byte biomes.
        V1_8 = V_1_8,
        /// Paletted block states + 0..256 world height; light inline (< 1.14) or separate (1.14 - 1.17.1).
        V1_9 = V_1_9,
        /// Combined chunk + light, NBT heightmaps, paletted biomes.
        V1_18 = V_1_18,
        /// Heightmap map; section buffer padded as if long arrays were length-prefixed.
        V1_21_5 = V_1_21_5,
        /// Padding dropped.
        V1_21_6 = V_1_21_6,
        /// Fluid count per section; same as 26.3.
        V26_1 = V_26_1,
    }
}

/// 26.3 chunk for `version` (`1.7.2` - `26.2`).
pub fn chunk_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let format = ChunkFormat::of(version);
    if format < ChunkFormat::V1_18 {
        return legacy_chunk_from_current(payload, format, version);
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

/// Companion `LIGHT_UPDATE` payload for 1.14 - 1.17.1 chunks, where light is a separate packet.
pub fn light_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    if version < JavaMinecraftVersion::V_1_14 || version >= JavaMinecraftVersion::V_1_18 {
        return None;
    }
    let chunk_x = payload.get_i32_be().ok()?;
    let chunk_z = payload.get_i32_be().ok()?;
    let _ = read_heightmaps(&mut payload)?;
    let data_len = usize::try_from(payload.get_var_int().ok()?.0).ok()?;
    let (mut sec_data, rest) = payload.split_at_checked(data_len)?;
    payload = rest;
    let sec_count = count_sections(&mut sec_data)?;
    skip_block_entities(&mut payload)?;
    let light = decode_current_light(payload)?;
    let base_section = if sec_count >= 24 { 4 } else { 0 };
    write_chunk_light_packet(chunk_x, chunk_z, base_section, &light, version)
}

struct DecodedSection {
    block_count: i16,
    states: Vec<u32>,
    biomes: Vec<u32>,
}

fn read_heightmaps(read: &mut &[u8]) -> Option<Vec<(i32, Vec<i64>)>> {
    let count = read.get_var_int().ok()?.0;
    let mut maps = Vec::with_capacity(count.max(0) as usize);
    for _ in 0..count {
        let kind = read.get_var_int().ok()?.0;
        let len = read.get_var_int().ok()?.0;
        let longs = (0..len)
            .map(|_| read.get_i64_be().ok())
            .collect::<Option<Vec<_>>>()?;
        maps.push((kind, longs));
    }
    Some(maps)
}

fn count_sections(read: &mut &[u8]) -> Option<usize> {
    let mut n = 0usize;
    while !read.is_empty() {
        let _ = read.get_i16_be().ok()?;
        let _ = read.get_i16_be().ok()?;
        let _ = palette::unpack(read, 4096, 8, |id| id)?;
        let _ = palette::unpack(read, 64, 3, |id| id)?;
        n += 1;
    }
    Some(n)
}

fn skip_block_entities(read: &mut &[u8]) -> Option<()> {
    let count = read.get_var_int().ok()?.0;
    for _ in 0..count {
        let _ = read.get_u8().ok()?;
        let _ = read.get_i16_be().ok()?;
        let _ = read.get_var_int().ok()?;
        let _ = split_network_nbt(read)?;
    }
    Some(())
}

fn decode_sections(mut data: &[u8], version: JavaMinecraftVersion) -> Option<Vec<DecodedSection>> {
    let biome_remap = BiomeRemap::new(version);
    let mut out = Vec::new();
    while !data.is_empty() {
        let block_count = data.get_i16_be().ok()?;
        let _fluid_count = data.get_i16_be().ok()?;
        let states = palette::unpack(&mut data, 4096, 8, |id| block::remap_state(id, version))?;
        let biomes = palette::unpack(&mut data, 64, 3, |id| biome_remap.get(id))?;
        out.push(DecodedSection {
            block_count,
            states,
            biomes,
        });
    }
    Some(out)
}

fn legacy_chunk_from_current(
    mut payload: &[u8],
    format: ChunkFormat,
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let chunk_x = payload.get_i32_be().ok()?;
    let chunk_z = payload.get_i32_be().ok()?;
    let maps = read_heightmaps(&mut payload)?;

    let data_len = usize::try_from(payload.get_var_int().ok()?.0).ok()?;
    let (sec_bytes, rest) = payload.split_at_checked(data_len)?;
    payload = rest;
    let all_sections = decode_sections(sec_bytes, version)?;

    skip_block_entities(&mut payload)?;
    let light = decode_current_light(payload)?;

    let base_section = if all_sections.len() >= 24 { 4 } else { 0 };
    let min_y = if base_section == 4 { -64 } else { 0 };
    let has_sky_light = all_sections.len() > 16 || !light.sky_light_arrays.is_empty();
    let (block_lights, sky_lights) = inline_section_lights(&light, base_section);

    let mut chunk_mask = 0u32;
    for i in 0..16 {
        if let Some(sec) = all_sections.get(base_section + i)
            && (sec.block_count != 0 || sec.states.iter().any(|&s| s != 0))
        {
            chunk_mask |= 1 << i;
        }
    }

    match format {
        ChunkFormat::V1_7 => encode_v1_7(
            chunk_x,
            chunk_z,
            chunk_mask as u16,
            base_section,
            &all_sections,
            &block_lights,
            &sky_lights,
            has_sky_light,
        ),
        ChunkFormat::V1_8 => encode_v1_8(
            chunk_x,
            chunk_z,
            chunk_mask as u16,
            base_section,
            &all_sections,
            &block_lights,
            &sky_lights,
            has_sky_light,
        ),
        ChunkFormat::V1_9 => encode_v1_9(
            chunk_x,
            chunk_z,
            chunk_mask,
            min_y,
            base_section,
            &maps,
            &all_sections,
            &block_lights,
            &sky_lights,
            has_sky_light,
            version,
        ),
        _ => None,
    }
}

#[expect(clippy::too_many_arguments)]
fn encode_v1_7(
    chunk_x: i32,
    chunk_z: i32,
    chunk_mask: u16,
    base_section: usize,
    sections: &[DecodedSection],
    block_lights: &[[u8; 2048]; 16],
    sky_lights: &[[u8; 2048]; 16],
    has_sky_light: bool,
) -> Option<Vec<u8>> {
    let mut extended_mask = 0u16;
    for i in 0..16 {
        if (chunk_mask & (1 << i)) != 0
            && sections[base_section + i]
                .states
                .iter()
                .any(|&s| (s >> 4) > 255)
        {
            extended_mask |= 1 << i;
        }
    }

    let mut raw = Vec::new();
    for i in 0..16 {
        if (chunk_mask & (1 << i)) != 0 {
            for &s in &sections[base_section + i].states {
                raw.push(((s >> 4) & 0xFF) as u8);
            }
        }
    }
    for i in 0..16 {
        if (chunk_mask & (1 << i)) != 0 {
            for pair in sections[base_section + i].states.as_chunks::<2>().0 {
                raw.push(((pair[0] & 0x0F) as u8) | (((pair[1] & 0x0F) as u8) << 4));
            }
        }
    }
    for (i, bl) in block_lights.iter().enumerate() {
        if (chunk_mask & (1 << i)) != 0 {
            raw.extend_from_slice(bl);
        }
    }
    if has_sky_light {
        for (i, sl) in sky_lights.iter().enumerate() {
            if (chunk_mask & (1 << i)) != 0 {
                raw.extend_from_slice(sl);
            }
        }
    }
    if extended_mask != 0 {
        for i in 0..16 {
            if (extended_mask & (1 << i)) != 0 {
                for pair in sections[base_section + i].states.as_chunks::<2>().0 {
                    let e0 = (((pair[0] >> 4) >> 8) & 0x0F) as u8;
                    let e1 = (((pair[1] >> 4) >> 8) & 0x0F) as u8;
                    raw.push(e0 | (e1 << 4));
                }
            }
        }
    }
    write_biomes_2d_u8(&mut raw, sections.get(base_section));

    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&raw).ok()?;
    let compressed = encoder.finish().ok()?;

    let mut out = Vec::with_capacity(18 + compressed.len());
    out.write_i32_be(chunk_x).ok()?;
    out.write_i32_be(chunk_z).ok()?;
    out.write_bool(true).ok()?;
    out.write_u16_be(chunk_mask).ok()?;
    out.write_u16_be(extended_mask).ok()?;
    out.write_i32_be(compressed.len() as i32).ok()?;
    out.extend_from_slice(&compressed);
    Some(out)
}

#[expect(clippy::too_many_arguments)]
fn encode_v1_8(
    chunk_x: i32,
    chunk_z: i32,
    chunk_mask: u16,
    base_section: usize,
    sections: &[DecodedSection],
    block_lights: &[[u8; 2048]; 16],
    sky_lights: &[[u8; 2048]; 16],
    has_sky_light: bool,
) -> Option<Vec<u8>> {
    let mut data = Vec::new();
    for i in 0..16 {
        if (chunk_mask & (1 << i)) != 0 {
            for &s in &sections[base_section + i].states {
                data.extend_from_slice(&(s as u16).to_le_bytes());
            }
        }
    }
    for (i, bl) in block_lights.iter().enumerate() {
        if (chunk_mask & (1 << i)) != 0 {
            data.extend_from_slice(bl);
        }
    }
    if has_sky_light {
        for (i, sl) in sky_lights.iter().enumerate() {
            if (chunk_mask & (1 << i)) != 0 {
                data.extend_from_slice(sl);
            }
        }
    }
    write_biomes_2d_u8(&mut data, sections.get(base_section));

    let mut out = Vec::with_capacity(16 + data.len());
    out.write_i32_be(chunk_x).ok()?;
    out.write_i32_be(chunk_z).ok()?;
    out.write_bool(true).ok()?;
    out.write_u16_be(chunk_mask).ok()?;
    out.write_var_int(&VarInt(data.len() as i32)).ok()?;
    out.extend_from_slice(&data);
    Some(out)
}

#[expect(clippy::too_many_arguments, clippy::too_many_lines)]
fn encode_v1_9(
    chunk_x: i32,
    chunk_z: i32,
    chunk_mask: u32,
    min_y: i32,
    base_section: usize,
    maps: &[(i32, Vec<i64>)],
    sections: &[DecodedSection],
    block_lights: &[[u8; 2048]; 16],
    sky_lights: &[[u8; 2048]; 16],
    has_sky_light: bool,
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    out.write_i32_be(chunk_x).ok()?;
    out.write_i32_be(chunk_z).ok()?;

    if version < JavaMinecraftVersion::V_1_17 {
        out.write_bool(true).ok()?;
    }
    if version == JavaMinecraftVersion::V_1_16 || version == JavaMinecraftVersion::V_1_16_1 {
        out.write_bool(true).ok()?;
    }

    if version >= JavaMinecraftVersion::V_1_17 {
        crate::legacy::write_bit_set_legacy(
            &mut out,
            &BitSet(Box::new([i64::from(chunk_mask)])),
            &version,
        )
        .ok()?;
    } else {
        out.write_var_int(&VarInt(chunk_mask as i32)).ok()?;
    }

    if version >= JavaMinecraftVersion::V_1_14 {
        let mut comp = NbtCompound::new();
        for &(kind, name) in &[
            (1, "WORLD_SURFACE"),
            (4, "MOTION_BLOCKING"),
            (5, "MOTION_BLOCKING_NO_LEAVES"),
        ] {
            let modern = maps
                .iter()
                .find(|(k, _)| *k == kind)
                .map(|(_, v)| v.as_slice());
            let packed =
                convert_heightmap_to_256(modern, min_y, version >= JavaMinecraftVersion::V_1_16);
            comp.put(name, NbtTag::LongArray(packed));
        }
        write_network_nbt(&Nbt::from(comp).write_unnamed(), version, &mut out);
    }

    if version >= JavaMinecraftVersion::V_1_15 {
        if version >= JavaMinecraftVersion::V_1_16_2 {
            out.write_var_int(&VarInt(1024)).ok()?;
        }
        for i in 0..16 {
            let sec = sections.get(base_section + i).or_else(|| sections.first());
            for idx in 0..64 {
                let biome_id = sec.and_then(|s| s.biomes.get(idx).copied()).unwrap_or(1);
                if version >= JavaMinecraftVersion::V_1_16_2 {
                    out.write_var_int(&VarInt(biome_id as i32)).ok()?;
                } else {
                    out.write_i32_be(biome_id as i32).ok()?;
                }
            }
        }
    }

    let direct_bpe = block::direct_bits(version) as usize;
    let modern_pack = version >= JavaMinecraftVersion::V_1_16;
    let mut data_buf = Vec::new();
    for i in 0..16 {
        if (chunk_mask & (1 << i)) == 0 {
            continue;
        }
        let sec = &sections[base_section + i];
        if version >= JavaMinecraftVersion::V_1_14 {
            let non_air = sec.states.iter().filter(|&&s| s != 0).count() as i16;
            data_buf.write_i16_be(non_air).ok()?;
        }

        let mut unique = Vec::<u32>::new();
        let mut indices = Vec::<u32>::with_capacity(4096);
        let mut overflow = false;
        for &s in &sec.states {
            if let Some(pos) = unique.iter().position(|&u| u == s) {
                indices.push(pos as u32);
            } else if unique.len() < 256 {
                indices.push(unique.len() as u32);
                unique.push(s);
            } else {
                overflow = true;
                break;
            }
        }

        if !overflow {
            let bpe = (ceil_log2(unique.len() as u32) as usize).max(4);
            data_buf.push(bpe as u8);
            data_buf.write_var_int(&VarInt(unique.len() as i32)).ok()?;
            for &state in &unique {
                data_buf.write_var_int(&VarInt(state as i32)).ok()?;
            }
            let packed = if modern_pack {
                pack_modern_data(&indices, bpe)
            } else {
                pack_legacy_data(&indices, bpe)
            };
            data_buf
                .write_list(&packed, |buf, &p| buf.write_i64_be(p))
                .ok()?;
        } else {
            data_buf.push(direct_bpe as u8);
            if version < JavaMinecraftVersion::V_1_13 {
                // 1.9 - 1.12.2 GlobalPalette writes a 0-length palette
                data_buf.write_var_int(&VarInt(0)).ok()?;
            }
            let packed = if modern_pack {
                pack_modern_data(&sec.states, direct_bpe)
            } else {
                pack_legacy_data(&sec.states, direct_bpe)
            };
            data_buf
                .write_list(&packed, |buf, &p| buf.write_i64_be(p))
                .ok()?;
        }

        if version < JavaMinecraftVersion::V_1_14 {
            data_buf.extend_from_slice(&block_lights[i]);
            if has_sky_light {
                data_buf.extend_from_slice(&sky_lights[i]);
            }
        }
    }

    if version < JavaMinecraftVersion::V_1_15 {
        let sec = sections.get(base_section).or_else(|| sections.first());
        for z in 0..16 {
            for x in 0..16 {
                let idx = (z / 4) * 4 + (x / 4);
                let biome_id = sec.and_then(|s| s.biomes.get(idx).copied()).unwrap_or(1);
                if version >= JavaMinecraftVersion::V_1_13 {
                    data_buf.write_i32_be(biome_id as i32).ok()?;
                } else {
                    data_buf.push(biome_id as u8);
                }
            }
        }
    }

    out.write_var_int(&VarInt(data_buf.len() as i32)).ok()?;
    out.extend_from_slice(&data_buf);

    if version >= JavaMinecraftVersion::V_1_9_3 {
        out.write_var_int(&VarInt(0)).ok()?;
    }

    Some(out)
}

fn write_biomes_2d_u8(out: &mut Vec<u8>, sec: Option<&DecodedSection>) {
    for z in 0..16 {
        for x in 0..16 {
            let idx = (z / 4) * 4 + (x / 4);
            let biome_id = sec.and_then(|s| s.biomes.get(idx).copied()).unwrap_or(1);
            out.push(biome_id as u8);
        }
    }
}

fn convert_heightmap_to_256(modern: Option<&[i64]>, min_y: i32, non_spanning: bool) -> Vec<i64> {
    let mut heights = [0u32; 256];
    if let Some(data) = modern {
        for (col, h) in heights.iter_mut().enumerate() {
            let word = data.get(col / 7).copied().unwrap_or(0) as u64;
            let raw = ((word >> ((col % 7) * 9)) & 0x1FF) as i32;
            if raw > 0 {
                let world_y = raw + min_y;
                *h = world_y.clamp(0, 256) as u32;
            }
        }
    }
    if non_spanning {
        pack_modern_data(&heights, 9)
    } else {
        pack_legacy_data(&heights, 9)
    }
}

fn pack_modern_data(entries: &[u32], bits_per_entry: usize) -> Vec<i64> {
    if bits_per_entry == 0 {
        return Vec::new();
    }
    let per_long = 64 / bits_per_entry;
    let mut out = vec![0i64; entries.len().div_ceil(per_long)];
    for (i, &val) in entries.iter().enumerate() {
        out[i / per_long] |= (u64::from(val) << ((i % per_long) * bits_per_entry)) as i64;
    }
    out
}

fn pack_legacy_data(entries: &[u32], bpe: usize) -> Vec<i64> {
    if bpe == 0 || entries.is_empty() {
        return Vec::new();
    }
    let long_count = (entries.len() * bpe).div_ceil(64);
    let mut data = vec![0u64; long_count];
    let mask = (1u64 << bpe) - 1;
    for (index, &value) in entries.iter().enumerate() {
        let val = u64::from(value) & mask;
        let bit_index = index * bpe;
        let start = bit_index / 64;
        let end = ((index + 1) * bpe - 1) / 64;
        let shift = bit_index % 64;
        data[start] =
            (data[start] & !(mask.wrapping_shl(shift as u32))) | val.wrapping_shl(shift as u32);
        if start != end {
            let rem = 64 - shift;
            let j1 = bpe - rem;
            data[end] = ((data[end] >> j1) << j1) | (val >> rem);
        }
    }
    data.into_iter().map(|w| w as i64).collect()
}

/// 26.3 writes `(type, longs)` pairs; `ChunkFormat::V1_18` an NBT compound of long arrays.
fn heightmaps(
    read: &mut &[u8],
    format: ChunkFormat,
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let maps = read_heightmaps(read)?;

    if format != ChunkFormat::V1_18 {
        out.write_var_int(&VarInt(maps.len() as i32)).ok()?;
        for (kind, longs) in maps {
            out.write_var_int(&VarInt(kind)).ok()?;
            out.write_var_int(&VarInt(longs.len() as i32)).ok()?;
            for long in longs {
                out.write_i64_be(long).ok()?;
            }
        }
        return Some(());
    }

    let mut compound = NbtCompound::new();
    for (kind, longs) in maps {
        let name = match kind {
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
