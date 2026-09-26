//! Entity data: SET_ENTITY_DATA, and the entity types its field ids depend on.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};

use pumpkin_data::{entity::EntityType, meta_data_type::MetaDataType};
use pumpkin_protocol::{
    VarInt,
    codec::item_stack_seralizer::ItemStackSerializer,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::translate::{
    block::remap_state, item::write_item_for_version, nbt::split_network_nbt,
    particle::write_particle, registry::id_remap,
};

pub struct EntityDataTables {
    /// Client serializer id by 26.3 serializer id, -1 where the client lacks it.
    pub serializers: &'static [i8],
    /// 26.3 entity name and the client field id by 26.3 field id, 255 where the client lacks it.
    pub fields: &'static [(&'static str, &'static [u8])],
}

include!(concat!(env!("OUT_DIR"), "/entity_data.rs"));

eras! {
    pub enum EntityDataFormat {
        // TODO: field ids before 1.21
        Unsupported = V_1_7_2,
        V1_21 = V_1_21,
        V1_21_2 = V_1_21_2,
        V1_21_4 = V_1_21_4,
        V1_21_5 = V_1_21_5,
        V1_21_6 = V_1_21_6,
        V1_21_7 = V_1_21_7,
        V1_21_9 = V_1_21_9,
        V1_21_11 = V_1_21_11,
        V26_1 = V_26_1,
        V26_2 = V_26_2,
        /// Core's own ids.
        V26_3 = V_26_3,
    }
}

fn tables_for(version: JavaMinecraftVersion) -> Option<&'static EntityDataTables> {
    use EntityDataFormat as F;
    Some(match EntityDataFormat::of(version) {
        F::Unsupported | F::V26_3 => return None,
        F::V1_21 => &ENTITY_DATA_1_21,
        F::V1_21_2 => &ENTITY_DATA_1_21_2,
        F::V1_21_4 => &ENTITY_DATA_1_21_4,
        F::V1_21_5 => &ENTITY_DATA_1_21_5,
        F::V1_21_6 => &ENTITY_DATA_1_21_6,
        F::V1_21_7 => &ENTITY_DATA_1_21_7,
        F::V1_21_9 => &ENTITY_DATA_1_21_9,
        F::V1_21_11 => &ENTITY_DATA_1_21_11,
        F::V26_1 => &ENTITY_DATA_26_1,
        F::V26_2 => &ENTITY_DATA_26_2,
    })
}

/// 26.3 entity type and spawns sent to older clients, by entity id. Pumpkin never reuses ids.
static ENTITY_TYPES: LazyLock<Mutex<HashMap<i32, (u16, u32)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Remembers the type of an entity spawned for an older client.
pub fn track_spawn(entity_id: i32, entity_type: u16) {
    let mut types = ENTITY_TYPES.lock().unwrap_or_else(PoisonError::into_inner);
    types.entry(entity_id).or_insert((entity_type, 0)).1 += 1;
}

/// REMOVE_ENTITIES: forgets entity types no older client sees anymore.
pub fn remove_entities_from_current(
    payload: &[u8],
    _version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let mut read = payload;
    let ids = read.get_list(|read| read.get_var_int()).ok()?;
    let mut types = ENTITY_TYPES.lock().unwrap_or_else(PoisonError::into_inner);
    for VarInt(id) in ids {
        if let Some((_, spawns)) = types.get_mut(&id) {
            *spawns -= 1;
            if *spawns == 0 {
                types.remove(&id);
            }
        }
    }
    Some(payload.to_vec())
}

fn entity_type(entity_id: i32) -> Option<u16> {
    let types = ENTITY_TYPES.lock().unwrap_or_else(PoisonError::into_inner);
    types.get(&entity_id).map(|(entity_type, _)| *entity_type)
}

fn copy(read: &mut &[u8], len: usize, out: &mut Vec<u8>) -> Option<()> {
    let (value, rest) = read.split_at_checked(len)?;
    out.extend_from_slice(value);
    *read = rest;
    Some(())
}

fn copy_var_int(read: &mut &[u8], out: &mut Vec<u8>) -> Option<()> {
    out.write_var_int(&read.get_var_int().ok()?).ok()
}

fn copy_optional(
    read: &mut &[u8],
    out: &mut Vec<u8>,
    value: impl FnOnce(&mut &[u8], &mut Vec<u8>) -> Option<()>,
) -> Option<()> {
    let present = read.get_bool().ok()?;
    out.write_bool(present).ok()?;
    if present { value(read, out) } else { Some(()) }
}

fn copy_nbt(read: &mut &[u8], out: &mut Vec<u8>) -> Option<()> {
    out.extend_from_slice(split_network_nbt(read)?);
    Some(())
}

/// Synced registry holding the values of a 26.3 variant serializer.
fn variant_registry(serializer: i32) -> Option<&'static str> {
    use MetaDataType as M;
    Some(match serializer {
        id if id == M::CAT_VARIANT.id => "cat_variant",
        id if id == M::CAT_SOUND_VARIANT.id => "cat_sound_variant",
        id if id == M::COW_VARIANT.id => "cow_variant",
        id if id == M::COW_SOUND_VARIANT.id => "cow_sound_variant",
        id if id == M::WOLF_VARIANT.id => "wolf_variant",
        id if id == M::WOLF_SOUND_VARIANT.id => "wolf_sound_variant",
        id if id == M::FROG_VARIANT.id => "frog_variant",
        id if id == M::PIG_VARIANT.id => "pig_variant",
        id if id == M::PIG_SOUND_VARIANT.id => "pig_sound_variant",
        id if id == M::CHICKEN_VARIANT.id => "chicken_variant",
        id if id == M::CHICKEN_SOUND_VARIANT.id => "chicken_sound_variant",
        id if id == M::ZOMBIE_NAUTILUS_VARIANT.id => "zombie_nautilus_variant",
        _ => return None,
    })
}

fn remap_registry_id(registry: &str, id: i32, version: JavaMinecraftVersion) -> i32 {
    id_remap(registry, version, "").map_or(id, |remap| remap.get(id as u32) as i32)
}

/// Reads one 26.3 value and writes it for `version`. `None` for values that are not
/// translated yet, which ends the entry list.
fn write_value(
    serializer: i32,
    read: &mut &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    use MetaDataType as M;
    if let Some(registry) = variant_registry(serializer) {
        let id = read.get_var_int().ok()?.0;
        return out
            .write_var_int(&VarInt(remap_registry_id(registry, id, version)))
            .ok();
    }
    match serializer {
        id if id == M::BYTE.id || id == M::BOOLEAN.id => copy(read, 1, out),
        id if id == M::FLOAT.id => copy(read, 4, out),
        id if id == M::BLOCK_POS.id => copy(read, 8, out),
        id if id == M::ROTATIONS.id || id == M::VECTOR3.id => copy(read, 12, out),
        id if id == M::QUATERNION.id => copy(read, 16, out),
        id if id == M::INT.id
            || id == M::DIRECTION.id
            || id == M::OPTIONAL_UNSIGNED_INT.id
            || id == M::POSE.id
            || id == M::SNIFFER_STATE.id
            || id == M::ARMADILLO_STATE.id
            || id == M::COPPER_GOLEM_STATE.id
            || id == M::WEATHERING_COPPER_STATE.id
            || id == M::HUMANOID_ARM.id
            || id == M::DYE_COLOR.id =>
        {
            copy_var_int(read, out)
        }
        id if id == M::LONG.id => out.write_var_long(&read.get_var_long().ok()?).ok(),
        id if id == M::STRING.id => {
            let len = read.get_var_int().ok()?;
            out.write_var_int(&len).ok()?;
            copy(read, usize::try_from(len.0).ok()?, out)
        }
        id if id == M::COMPONENT.id => copy_nbt(read, out),
        id if id == M::OPTIONAL_COMPONENT.id => copy_optional(read, out, copy_nbt),
        id if id == M::OPTIONAL_BLOCK_POS.id => copy_optional(read, out, |r, o| copy(r, 8, o)),
        id if id == M::OPTIONAL_LIVING_ENTITY_REFERENCE.id => {
            copy_optional(read, out, |r, o| copy(r, 16, o))
        }
        id if id == M::OPTIONAL_GLOBAL_POS.id => copy_optional(read, out, |r, o| {
            let dimension = r.get_var_int().ok()?;
            o.write_var_int(&dimension).ok()?;
            copy(r, usize::try_from(dimension.0).ok()?, o)?;
            copy(r, 8, o)
        }),
        // 0 is empty, so no remap needed
        id if id == M::BLOCK_STATE.id || id == M::OPTIONAL_BLOCK_STATE.id => {
            let state = read.get_var_int().ok()?.0 as u32;
            out.write_var_int(&VarInt(remap_state(state, version) as i32))
                .ok()
        }
        id if id == M::ITEM_STACK.id => {
            let stack = ItemStackSerializer::read(read).ok()?.to_stack();
            write_item_for_version(&stack, version, out).ok()
        }
        id if id == M::PARTICLE.id => write_particle(read, version, out),
        id if id == M::PARTICLES.id => {
            let count = read.get_var_int().ok()?;
            out.write_var_int(&count).ok()?;
            for _ in 0..count.0 {
                write_particle(read, version, out)?;
            }
            Some(())
        }
        // Type and profession are built-in registries that kept their order
        id if id == M::VILLAGER_DATA.id => {
            for _ in 0..3 {
                copy_var_int(read, out)?;
            }
            Some(())
        }
        // 0 is an inline variant
        id if id == M::PAINTING_VARIANT.id => {
            let holder = read.get_var_int().ok()?.0;
            if holder == 0 {
                return None;
            }
            let id = remap_registry_id("painting_variant", holder - 1, version);
            out.write_var_int(&VarInt(id + 1)).ok()
        }
        // TODO: resolvable profiles and inline painting variants
        _ => None,
    }
}

/// SET_ENTITY_DATA: the client's field and serializer ids, dropping fields it lacks.
/// Entries after a value that cannot be translated yet are dropped.
pub fn set_entity_data_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let tables = tables_for(version)?;
    let entity_id = payload.get_var_int().ok()?;
    let name = EntityType::from_raw(entity_type(entity_id.0)?)?.resource_name;
    let fields = tables
        .fields
        .binary_search_by_key(&name, |(entity, _)| entity)
        .map(|i| tables.fields[i].1)
        .ok()?;

    let mut out = Vec::with_capacity(payload.len() + 5);
    out.write_var_int(&entity_id).ok()?;
    let mut value = Vec::new();
    loop {
        let index = payload.get_u8().ok()?;
        if index == u8::MAX {
            break;
        }
        let serializer = payload.get_var_int().ok()?.0;
        value.clear();
        if write_value(serializer, &mut payload, version, &mut value).is_none() {
            break;
        }
        let field = fields.get(usize::from(index)).copied().unwrap_or(u8::MAX);
        let client_serializer = tables
            .serializers
            .get(serializer as usize)
            .copied()
            .unwrap_or(-1);
        if field == u8::MAX || client_serializer < 0 {
            continue;
        }
        out.push(field);
        out.write_var_int(&VarInt(i32::from(client_serializer)))
            .ok()?;
        out.extend_from_slice(&value);
    }
    out.push(u8::MAX);
    Some(out)
}

#[cfg(test)]
mod tests {
    use pumpkin_data::tracked_data;

    use super::*;

    const V1_21_11: JavaMinecraftVersion = JavaMinecraftVersion::V_1_21_11;

    fn entry(index: u8, serializer: MetaDataType, value: &[u8], out: &mut Vec<u8>) {
        out.push(index);
        out.write_var_int(&VarInt(serializer.id)).unwrap();
        out.extend_from_slice(value);
    }

    #[test]
    fn ageable_fields_shift_and_age_locked_is_dropped() {
        track_spawn(-7, EntityType::COW.id);
        let mut current = Vec::new();
        current.write_var_int(&VarInt(-7)).unwrap();
        // Shared flags, baby, age locked, health
        entry(0, MetaDataType::BYTE, &[0x20], &mut current);
        entry(16, MetaDataType::BOOLEAN, &[1], &mut current);
        entry(17, MetaDataType::BOOLEAN, &[1], &mut current);
        entry(9, MetaDataType::FLOAT, &10.0f32.to_be_bytes(), &mut current);
        current.push(u8::MAX);

        let mut expected = Vec::new();
        expected.write_var_int(&VarInt(-7)).unwrap();
        expected.extend_from_slice(&[0, 0, 0x20, 16, 8, 1, 9, 3]);
        expected.extend_from_slice(&10.0f32.to_be_bytes());
        expected.push(u8::MAX);
        assert_eq!(
            set_entity_data_from_current(&current, V1_21_11),
            Some(expected)
        );
    }

    #[test]
    fn item_entity_keeps_its_stack_field() {
        track_spawn(-8, EntityType::ITEM.id);
        assert_eq!(tracked_data::item::ITEM.id.0, 8);
        let tables = tables_for(V1_21_11).unwrap();
        let (_, fields) = tables.fields.iter().find(|(e, _)| *e == "item").unwrap();
        assert_eq!(fields[8], 8);
        // 26.3's humanoid arm was 1.21.11's arm (38)
        assert_eq!(
            tables.serializers[MetaDataType::HUMANOID_ARM.id as usize],
            38
        );
        assert_eq!(tables.serializers[MetaDataType::DYE_COLOR.id as usize], -1);
    }
}
