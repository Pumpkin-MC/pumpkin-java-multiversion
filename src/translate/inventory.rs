//! Inventory items: container content and slots, cursor, player inventory, equipment, and the
//! creative slots and clicks sent back.

use pumpkin_data::{item_stack::ItemStack, packet::CURRENT_MC_VERSION};
use pumpkin_protocol::{
    VarInt,
    codec::item_stack_seralizer::ItemStackSerializer,
    java::client::play::slot_to_version,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::remap::{
    data_component_type_id_remap::remap_data_component_type_id_from_version,
    item_id_remap::remap_item_id_from_version,
};
use crate::translate::item::write_item_for_version;

eras! {
    pub enum ContainerFormat {
        /// i16 slot count, no state id or carried item.
        V1_7 = V_1_7_2,
        /// State id, var int slot count and carried item.
        V1_17_1 = V_1_17_1,
    }
}

eras! {
    pub enum EquipmentFormat {
        /// i32 entity, one i16 slot with the old slot order.
        V1_7 = V_1_7_2,
        /// Var int entity.
        V1_8 = V_1_8,
        /// Var int slot.
        V1_9 = V_1_9,
        /// Every slot, the high bit marking more to follow.
        V1_16 = V_1_16,
    }
}

eras! {
    pub enum ClientItemFormat {
        // TODO: items from clients before 1.21.5 still carry their ids
        Unsupported = V_1_7_2,
        /// Length-prefixed components in creative slots, hashed stacks in clicks; same as 26.3.
        V1_21_5 = V_1_21_5,
    }
}

fn read_item(read: &mut &[u8]) -> Option<ItemStack> {
    Some(ItemStackSerializer::read(read).ok()?.to_stack())
}

fn write_item(read: &mut &[u8], version: JavaMinecraftVersion, out: &mut Vec<u8>) -> Option<()> {
    write_item_for_version(&read_item(read)?, version, out).ok()
}

/// CONTAINER_SET_CONTENT.
pub fn container_set_content_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let format = ContainerFormat::of(version);
    let container_id = payload.get_container_id(&CURRENT_MC_VERSION).ok()?;
    let state_id = payload.get_var_int().ok()?;
    let count = payload.get_var_int().ok()?;

    let mut out = Vec::with_capacity(payload.len());
    out.write_container_id(&container_id, &version).ok()?;
    if format >= ContainerFormat::V1_17_1 {
        out.write_var_int(&state_id).ok()?;
        out.write_var_int(&count).ok()?;
    } else {
        out.write_i16_be(i16::try_from(count.0).ok()?).ok()?;
    }
    for _ in 0..count.0 {
        write_item(&mut payload, version, &mut out)?;
    }
    if format >= ContainerFormat::V1_17_1 {
        write_item(&mut payload, version, &mut out)?;
    }
    Some(out)
}

/// CONTAINER_SET_SLOT.
pub fn container_set_slot_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let container_id = payload.get_container_id(&CURRENT_MC_VERSION).ok()?;
    let state_id = payload.get_var_int().ok()?;
    let slot = payload.get_i16_be().ok()?;

    let mut out = Vec::new();
    out.write_container_id(&container_id, &version).ok()?;
    if ContainerFormat::of(version) >= ContainerFormat::V1_17_1 {
        out.write_var_int(&state_id).ok()?;
    }
    out.write_i16_be(slot).ok()?;
    write_item(&mut payload, version, &mut out)?;
    Some(out)
}

/// SET_CURSOR_ITEM.
pub fn set_cursor_item_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    write_item(&mut payload, version, &mut out)?;
    Some(out)
}

/// SET_PLAYER_INVENTORY.
pub fn set_player_inventory_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    out.write_var_int(&payload.get_var_int().ok()?).ok()?;
    write_item(&mut payload, version, &mut out)?;
    Some(out)
}

/// SET_EQUIPMENT. Clients before 1.16 get the first slot only.
pub fn set_equipment_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let format = EquipmentFormat::of(version);
    let entity_id = payload.get_var_int().ok()?;

    let mut out = Vec::new();
    if format == EquipmentFormat::V1_7 {
        out.write_i32_be(entity_id.0).ok()?;
    } else {
        out.write_var_int(&entity_id).ok()?;
    }
    loop {
        let slot = payload.get_u8().ok()?;
        let more = slot & 0x80 != 0;
        let slot = (slot & 0x7F) as i8;
        if format == EquipmentFormat::V1_16 {
            out.write_i8(if more { slot | i8::MIN } else { slot })
                .ok()?;
        } else {
            let slot = slot_to_version(slot, &version);
            if format == EquipmentFormat::V1_9 {
                out.write_var_int(&VarInt(i32::from(slot))).ok()?;
            } else {
                out.write_i16_be(i16::from(slot)).ok()?;
            }
        }
        write_item(&mut payload, version, &mut out)?;
        if !more || format < EquipmentFormat::V1_16 {
            break;
        }
    }
    Some(out)
}

/// 26.3 component id of a client one, `None` when 26.3 has no such component.
fn component_id_to_current(
    read: &mut &[u8],
    version: JavaMinecraftVersion,
) -> Option<Option<VarInt>> {
    let id = read.get_var_int().ok()?.0;
    let mapped = remap_data_component_type_id_from_version(id as u32, version);
    Some((mapped != 0 || id == 0).then_some(VarInt(mapped as i32)))
}

fn write_item_id_to_current(
    read: &mut &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let id = read.get_var_int().ok()?.0;
    let id = remap_item_id_from_version(u16::try_from(id).ok()?, version);
    out.write_var_int(&VarInt(i32::from(id))).ok()
}

/// An optional stack with length-prefixed components, as 26.3's ids.
// TODO: component payloads are kept in the client's format.
fn untrusted_item_to_current(
    read: &mut &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let count = read.get_var_int().ok()?;
    out.write_var_int(&count).ok()?;
    if count.0 <= 0 {
        return Some(());
    }
    write_item_id_to_current(read, version, out)?;
    let added = read.get_var_int().ok()?.0;
    let removed = read.get_var_int().ok()?.0;
    let mut added_out = Vec::new();
    let mut added_count = 0;
    for _ in 0..added {
        let id = component_id_to_current(read, version)?;
        let len = read.get_var_int().ok()?;
        let (data, rest) = read.split_at_checked(usize::try_from(len.0).ok()?)?;
        *read = rest;
        if let Some(id) = id {
            added_out.write_var_int(&id).ok()?;
            added_out.write_var_int(&len).ok()?;
            added_out.extend_from_slice(data);
            added_count += 1;
        }
    }
    let mut removed_ids = Vec::new();
    for _ in 0..removed {
        removed_ids.extend(component_id_to_current(read, version)?);
    }
    out.write_var_int(&VarInt(added_count)).ok()?;
    out.write_var_int(&VarInt(removed_ids.len() as i32)).ok()?;
    out.extend_from_slice(&added_out);
    for id in removed_ids {
        out.write_var_int(&id).ok()?;
    }
    Some(())
}

/// An optional hashed stack, as 26.3's ids.
fn hashed_item_to_current(
    read: &mut &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let present = read.get_bool().ok()?;
    out.write_bool(present).ok()?;
    if !present {
        return Some(());
    }
    write_item_id_to_current(read, version, out)?;
    out.write_var_int(&read.get_var_int().ok()?).ok()?;
    let mut added = Vec::new();
    for _ in 0..read.get_var_int().ok()?.0 {
        let id = component_id_to_current(read, version)?;
        let hash = read.get_i32_be().ok()?;
        added.extend(id.map(|id| (id, hash)));
    }
    let mut removed = Vec::new();
    for _ in 0..read.get_var_int().ok()?.0 {
        removed.extend(component_id_to_current(read, version)?);
    }
    out.write_var_int(&VarInt(added.len() as i32)).ok()?;
    for (id, hash) in added {
        out.write_var_int(&id).ok()?;
        out.write_i32_be(hash).ok()?;
    }
    out.write_var_int(&VarInt(removed.len() as i32)).ok()?;
    for id in removed {
        out.write_var_int(&id).ok()?;
    }
    Some(())
}

/// SET_CREATIVE_MODE_SLOT. `None` keeps the payload.
pub fn creative_slot_to_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    if ClientItemFormat::of(version) == ClientItemFormat::Unsupported {
        return None;
    }
    let mut out = Vec::new();
    out.write_i16_be(payload.get_i16_be().ok()?).ok()?;
    untrusted_item_to_current(&mut payload, version, &mut out)?;
    Some(out)
}

/// CONTAINER_CLICK. `None` keeps the payload.
pub fn container_click_to_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    if ClientItemFormat::of(version) == ClientItemFormat::Unsupported {
        return None;
    }
    let mut out = Vec::new();
    out.write_var_int(&payload.get_var_int().ok()?).ok()?; // container
    out.write_var_int(&payload.get_var_int().ok()?).ok()?; // state id
    out.write_i16_be(payload.get_i16_be().ok()?).ok()?; // slot
    out.write_i8(payload.get_i8().ok()?).ok()?; // button
    out.write_var_int(&payload.get_var_int().ok()?).ok()?; // input
    let changed = payload.get_var_int().ok()?;
    out.write_var_int(&changed).ok()?;
    for _ in 0..changed.0 {
        out.write_i16_be(payload.get_i16_be().ok()?).ok()?;
        hashed_item_to_current(&mut payload, version, &mut out)?;
    }
    hashed_item_to_current(&mut payload, version, &mut out)?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use pumpkin_data::item::Item;
    use pumpkin_protocol::{ClientPacket, java::client::play::CSetContainerSlot};

    use crate::remap::item_id_remap::remap_item_id_for_version;

    use super::*;

    const V1_21_11: JavaMinecraftVersion = JavaMinecraftVersion::V_1_21_11;

    #[test]
    fn slot_item_gets_the_client_id() {
        let stack = ItemStackSerializer::from(ItemStack::new(3, &Item::DIRT));
        let mut current = Vec::new();
        CSetContainerSlot::new(0, 5, 36, &stack)
            .write_packet_data(&mut current, &CURRENT_MC_VERSION)
            .unwrap();

        let dirt = remap_item_id_for_version(Item::DIRT.id, V1_21_11);
        assert_eq!(dirt, 28);
        let mut expected = Vec::new();
        expected.write_var_int(&VarInt(0)).unwrap();
        expected.write_var_int(&VarInt(5)).unwrap();
        expected.write_i16_be(36).unwrap();
        // count, id, no components
        expected.extend_from_slice(&[3, dirt as u8, 0, 0]);
        assert_eq!(
            container_set_slot_from_current(&current, V1_21_11),
            Some(expected)
        );
    }

    #[test]
    fn creative_slot_gets_the_current_id() {
        let mut client = Vec::new();
        client.write_i16_be(36).unwrap();
        client.extend_from_slice(&[1, 28, 0, 0]);
        let out = creative_slot_to_current(&client, V1_21_11).unwrap();
        let mut read = &out[2..];
        assert_eq!(read.get_var_int().unwrap(), VarInt(1));
        assert_eq!(
            read.get_var_int().unwrap(),
            VarInt(i32::from(Item::DIRT.id))
        );
    }
}
