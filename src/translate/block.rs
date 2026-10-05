//! Block states and block entity types: BLOCK_UPDATE, SECTION_BLOCKS_UPDATE, BLOCK_ENTITY_DATA,
//! chunk palettes.

use std::sync::{Mutex, PoisonError};

use pumpkin_protocol::{
    VarInt,
    codec::var_long::VarLong,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use super::{
    ceil_log2,
    nbt::{split_network_nbt, write_network_nbt},
};
use crate::remap::{
    block_entity_type_id_remap::remap_block_entity_type_id_for_version,
    block_state_remap::remap_block_state_for_version,
};
use pumpkin_protocol::java::legacy::block_pos_to_version;

#[must_use]
pub fn remap_state(id: u32, version: JavaMinecraftVersion) -> u32 {
    u16::try_from(id).map_or(0, |id| {
        u32::from(remap_block_state_for_version(id, version))
    })
}

#[must_use]
pub fn remap_block_entity_type(id: u32, version: JavaMinecraftVersion) -> u32 {
    remap_block_entity_type_id_for_version(id, version)
}

/// Width of the client's global block state palette, from the highest state 26.3 maps to.
pub fn direct_bits(version: JavaMinecraftVersion) -> u8 {
    static CACHE: Mutex<Vec<(JavaMinecraftVersion, u8)>> = Mutex::new(Vec::new());
    let mut cache = CACHE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(&(_, bits)) = cache.iter().find(|(v, _)| *v == version) {
        return bits;
    }
    let max = (0..=u16::MAX)
        .map(|id| remap_block_state_for_version(id, version))
        .max()
        .unwrap_or(0);
    let bits = ceil_log2(u32::from(max) + 1);
    cache.push((version, bits));
    bits
}

/// BLOCK_UPDATE: position, state.
pub fn block_update_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let position = payload.get_i64_be().ok()?;
    let state = payload.get_var_int().ok()?.0 as u32;
    let mut out = Vec::new();
    out.write_i64_be(block_pos_to_version(position, &version))
        .ok()?;
    out.write_var_int(&VarInt(remap_state(state, version) as i32))
        .ok()?;
    Some(out)
}

eras! {
    pub enum BlockEntityDataFormat {
        // TODO: a byte action per type before 1.18; dropped until then
        Action = V_1_7_2,
        /// Type id.
        V1_18 = V_1_18,
    }
}

/// BLOCK_ENTITY_DATA: position, type, NBT. Types the client lacks are dropped.
pub fn block_entity_data_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    if BlockEntityDataFormat::of(version) == BlockEntityDataFormat::Action {
        return None;
    }
    let position = payload.get_i64_be().ok()?;
    let kind = payload.get_var_int().ok()?.0 as u32;
    let client_kind = remap_block_entity_type(kind, version);
    if client_kind == 0 && kind != 0 {
        return None;
    }
    let mut out = Vec::new();
    out.write_i64_be(position).ok()?;
    out.write_var_int(&VarInt(client_kind as i32)).ok()?;
    write_network_nbt(split_network_nbt(&mut payload)?, version, &mut out);
    Some(out)
}

eras! {
    pub enum SectionUpdateFormat {
        /// Per chunk with short entries, not translated yet.
        V1_7 = V_1_7_2,
        /// Per section with packed var long entries and a light flag.
        V1_16 = V_1_16,
        /// Light flag removed; same as 26.3.
        V1_20 = V_1_20,
    }
}

/// SECTION_BLOCKS_UPDATE: `state << 12 | local position` entries.
// TODO: SectionUpdateFormat::V1_7.
pub fn section_blocks_update_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let format = SectionUpdateFormat::of(version);
    if format == SectionUpdateFormat::V1_7 {
        return None;
    }
    let section = payload.get_i64_be().ok()?;
    let count = payload.get_var_int().ok()?;
    let mut out = Vec::new();
    out.write_i64_be(section).ok()?;
    if format == SectionUpdateFormat::V1_16 {
        out.write_bool(false).ok()?;
    }
    out.write_var_int(&count).ok()?;
    for _ in 0..count.0 {
        let entry = payload.get_var_long().ok()?.0 as u64;
        let state = remap_state((entry >> 12) as u32, version);
        let packed = (u64::from(state) << 12) | (entry & 0xFFF);
        out.write_var_long(&VarLong(packed as i64)).ok()?;
    }
    Some(out)
}
