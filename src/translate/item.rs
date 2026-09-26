use pumpkin_data::item_stack::ItemStack;
use pumpkin_protocol::{
    VarInt,
    codec::data_component::serialize,
    ser::{NetworkWriteExt, WritingError},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::remap::{
    data_component_type_id_remap::remap_data_component_type_id_for_version,
    item_id_remap::remap_item_id_for_version,
};

/// Client component id, `None` when the component does not exist for the client.
fn component_id_for_version(id: u8, version: JavaMinecraftVersion) -> Option<i32> {
    let mapped = remap_data_component_type_id_for_version(u32::from(id), version);
    (mapped != 0 || id == 0).then_some(mapped as i32)
}

eras! {
    pub enum ItemFormat {
        /// Short id (-1 empty), count, damage, NBT.
        V1_7 = V_1_7_2,
        /// Damage moved into NBT.
        V1_13 = V_1_13,
        /// Present flag, var int id, count, NBT.
        V1_13_2 = V_1_13_2,
        /// Count (0 empty), id, component patch.
        V1_20_5 = V_1_20_5,
    }
}

/// A 26.3 item stack in the client's `ItemStack` layout, with the client's item and component ids.
// TODO: component payloads are written in the 26.3 format.
pub fn write_item_for_version(
    stack: &ItemStack,
    version: JavaMinecraftVersion,
    write: &mut Vec<u8>,
) -> Result<(), WritingError> {
    let item_id = remap_item_id_for_version(stack.item.id, version);
    let format = ItemFormat::of(version);
    if stack.is_empty() {
        return match format {
            ItemFormat::V1_7 | ItemFormat::V1_13 => write.write_i16_be(-1),
            ItemFormat::V1_13_2 => write.write_bool(false),
            ItemFormat::V1_20_5 => write.write_var_int(&VarInt(0)),
        };
    }
    match format {
        ItemFormat::V1_7 | ItemFormat::V1_13 => {
            write.write_i16_be(item_id as i16)?;
            write.write_i8(stack.item_count as i8)?;
            if format == ItemFormat::V1_7 {
                // damage / metadata
                write.write_i16_be(0)?;
            }
            // TAG_End: no NBT
            write.write_u8(0)
        }
        ItemFormat::V1_13_2 => {
            write.write_bool(true)?;
            write.write_var_int(&VarInt::from(item_id))?;
            write.write_i8(stack.item_count as i8)?;
            // TAG_End: no NBT
            write.write_u8(0)
        }
        ItemFormat::V1_20_5 => write_patched(stack, item_id, version, write),
    }
}

/// `ItemFormat::V1_20_5`: count, id, component patch.
fn write_patched(
    stack: &ItemStack,
    item_id: u16,
    version: JavaMinecraftVersion,
    write: &mut Vec<u8>,
) -> Result<(), WritingError> {
    write.write_var_int(&VarInt::from(stack.item_count))?;
    write.write_var_int(&VarInt::from(item_id))?;
    write_patch(stack, version, write)
}

/// A non-empty 26.3 item stack as an `ItemStackTemplate` (26.1+): id, count, component patch.
pub fn write_template_for_version(
    stack: &ItemStack,
    version: JavaMinecraftVersion,
    write: &mut Vec<u8>,
) -> Result<(), WritingError> {
    let item_id = remap_item_id_for_version(stack.item.id, version);
    write.write_var_int(&VarInt::from(item_id))?;
    write.write_var_int(&VarInt::from(stack.item_count))?;
    write_patch(stack, version, write)
}

/// Added and removed components with the client's component ids.
fn write_patch(
    stack: &ItemStack,
    version: JavaMinecraftVersion,
    write: &mut Vec<u8>,
) -> Result<(), WritingError> {
    let added: Vec<_> = stack
        .patch
        .iter()
        .filter_map(|(id, data)| {
            let data = data.as_ref()?;
            Some((component_id_for_version(id.to_id(), version)?, *id, data))
        })
        .collect();
    let removed: Vec<_> = stack
        .patch
        .iter()
        .filter(|(_, data)| data.is_none())
        .filter_map(|(id, _)| component_id_for_version(id.to_id(), version))
        .collect();

    write.write_var_int(&VarInt(added.len() as i32))?;
    write.write_var_int(&VarInt(removed.len() as i32))?;
    for (client_id, id, data) in added {
        write.write_var_int(&VarInt(client_id))?;
        serialize(id, data.as_ref(), write)?;
    }
    for client_id in removed {
        write.write_var_int(&VarInt(client_id))?;
    }
    Ok(())
}
