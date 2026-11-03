use pumpkin_data::{
    data_component::DataComponent,
    data_component_impl::{
        BundleContentsImpl, ContainerImpl, CustomModelDataImpl, DataComponentImpl,
    },
    item_stack::ItemStack,
};
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

eras! {
    pub enum NestedItemFormat {
        /// `ItemStack`: count, id, component patch; containers mark empty slots with count 0.
        V1_20_5 = V_1_20_5,
        /// `ItemStackTemplate`: id, count, component patch; containers use a present flag.
        V26_1 = V_26_1,
    }
}

eras! {
    pub enum CustomModelDataFormat {
        /// One var int.
        V1_20_5 = V_1_20_5,
        /// Float, flag, string and color lists; same as 26.3.
        V1_21_4 = V_1_21_4,
    }
}

/// A 26.3 item stack in the client's `ItemStack` layout, with the client's item and component ids.
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

/// Added and removed components with the client's component ids and payloads.
fn write_patch(
    stack: &ItemStack,
    version: JavaMinecraftVersion,
    write: &mut Vec<u8>,
) -> Result<(), WritingError> {
    let mut added = 0;
    let mut added_out = Vec::new();
    for (id, data) in &stack.patch {
        let Some(data) = data else { continue };
        let Some(client_id) = component_id_for_version(id.to_id(), version) else {
            continue;
        };
        let mut payload = Vec::new();
        if write_component(*id, data.as_ref(), version, &mut payload)? {
            added_out.write_var_int(&VarInt(client_id))?;
            added_out.extend_from_slice(&payload);
            added += 1;
        }
    }
    let removed: Vec<_> = stack
        .patch
        .iter()
        .filter(|(_, data)| data.is_none())
        .filter_map(|(id, _)| component_id_for_version(id.to_id(), version))
        .collect();

    write.write_var_int(&VarInt(added))?;
    write.write_var_int(&VarInt(removed.len() as i32))?;
    write.extend_from_slice(&added_out);
    for client_id in removed {
        write.write_var_int(&VarInt(client_id))?;
    }
    Ok(())
}

/// A component payload in the client's format, `false` when the client cannot hold it.
// TODO: only custom model data and nested items are converted, the rest keep the 26.3 format.
fn write_component(
    id: DataComponent,
    data: &dyn DataComponentImpl,
    version: JavaMinecraftVersion,
    write: &mut Vec<u8>,
) -> Result<bool, WritingError> {
    let nested = NestedItemFormat::of(version);
    let any = data.as_any();
    if let Some(model) = any.downcast_ref::<CustomModelDataImpl>()
        && CustomModelDataFormat::of(version) == CustomModelDataFormat::V1_20_5
    {
        // The first float, as ViaBackwards maps it
        let Some(&first) = model.floats.first() else {
            return Ok(false);
        };
        write.write_var_int(&VarInt(first as i32))?;
    } else if let Some(bundle) = any.downcast_ref::<BundleContentsImpl>() {
        write.write_var_int(&VarInt(bundle.items.len() as i32))?;
        for item in &bundle.items {
            write_nested_item(item, version, write)?;
        }
    } else if let Some(container) = any.downcast_ref::<ContainerImpl>() {
        let len = container
            .items
            .iter()
            .map(|(slot, _)| usize::from(*slot) + 1)
            .max()
            .unwrap_or(0);
        write.write_var_int(&VarInt(len as i32))?;
        for slot in 0..len {
            let item = container
                .items
                .iter()
                .find(|(item_slot, item)| usize::from(*item_slot) == slot && !item.is_empty());
            match (item, nested) {
                (Some((_, item)), NestedItemFormat::V1_20_5) => {
                    write_item_for_version(item, version, write)?;
                }
                (Some((_, item)), NestedItemFormat::V26_1) => {
                    write.write_bool(true)?;
                    write_template_for_version(item, version, write)?;
                }
                (None, NestedItemFormat::V1_20_5) => write.write_var_int(&VarInt(0))?,
                (None, NestedItemFormat::V26_1) => write.write_bool(false)?,
            }
        }
    } else if nested == NestedItemFormat::V1_20_5 && id == DataComponent::ChargedProjectiles {
        // Core keeps no projectile stacks, and an empty `ItemStack` fails to decode
        write.write_var_int(&VarInt(0))?;
    } else if nested == NestedItemFormat::V1_20_5 && id == DataComponent::UseRemainder {
        // Core keeps no remainder stack
        return Ok(false);
    } else {
        serialize(id, data, write)?;
    }
    Ok(true)
}

/// A non-empty stack inside a component.
fn write_nested_item(
    stack: &ItemStack,
    version: JavaMinecraftVersion,
    write: &mut Vec<u8>,
) -> Result<(), WritingError> {
    match NestedItemFormat::of(version) {
        NestedItemFormat::V1_20_5 => write_item_for_version(stack, version, write),
        NestedItemFormat::V26_1 => write_template_for_version(stack, version, write),
    }
}

#[cfg(test)]
mod tests {
    use pumpkin_data::item::Item;

    use super::*;

    fn var_ints(values: &[i32]) -> Vec<u8> {
        let mut out = Vec::new();
        for &value in values {
            out.write_var_int(&VarInt(value)).unwrap();
        }
        out
    }

    fn client_component(id: DataComponent, version: JavaMinecraftVersion) -> i32 {
        component_id_for_version(id.to_id(), version).unwrap()
    }

    fn client_item(item: &Item, version: JavaMinecraftVersion) -> i32 {
        i32::from(remap_item_id_for_version(item.id, version))
    }

    fn written(stack: &ItemStack, version: JavaMinecraftVersion) -> Vec<u8> {
        let mut out = Vec::new();
        write_item_for_version(stack, version, &mut out).unwrap();
        out
    }

    fn model_data(floats: Vec<f32>) -> ItemStack {
        let mut stack = ItemStack::new(1, &Item::DIRT);
        stack.set_data_component(CustomModelDataImpl {
            floats,
            flags: vec![true],
            strings: Vec::new(),
            colors: Vec::new(),
        });
        stack
    }

    #[test]
    fn custom_model_data_becomes_its_first_float() {
        let version = JavaMinecraftVersion::V_1_21_2;
        let dirt = client_item(&Item::DIRT, version);
        let model = client_component(DataComponent::CustomModelData, version);
        assert_eq!(
            written(&model_data(vec![7.9, 2.0]), version),
            var_ints(&[1, dirt, 1, 0, model, 7])
        );
        // Nothing to map without a float
        assert_eq!(
            written(&model_data(Vec::new()), version),
            var_ints(&[1, dirt, 0, 0])
        );
    }

    #[test]
    fn custom_model_data_lists_since_1_21_4() {
        let version = JavaMinecraftVersion::V_1_21_4;
        let stack = model_data(vec![7.9]);
        let mut expected = var_ints(&[
            1,
            client_item(&Item::DIRT, version),
            1,
            0,
            client_component(DataComponent::CustomModelData, version),
        ]);
        serialize(
            DataComponent::CustomModelData,
            stack.patch[0].1.as_deref().unwrap(),
            &mut expected,
        )
        .unwrap();
        assert_eq!(written(&stack, version), expected);
    }

    fn bundle() -> ItemStack {
        let mut stack = ItemStack::new(1, &Item::BUNDLE);
        stack.set_data_component(BundleContentsImpl {
            items: vec![ItemStack::new(2, &Item::DIRT)],
        });
        stack
    }

    #[test]
    fn bundle_items_are_client_stacks() {
        for (version, inner) in [
            // count, id, patch
            (JavaMinecraftVersion::V_1_21_11, [2, -1]),
            // id, count, patch
            (JavaMinecraftVersion::V_26_2, [-1, 2]),
        ] {
            let dirt = client_item(&Item::DIRT, version);
            let inner = inner.map(|value| if value == -1 { dirt } else { value });
            let mut expected = var_ints(&[
                1,
                client_item(&Item::BUNDLE, version),
                1,
                0,
                client_component(DataComponent::BundleContents, version),
                1,
            ]);
            expected.extend(var_ints(&inner));
            expected.extend(var_ints(&[0, 0]));
            assert_eq!(written(&bundle(), version), expected, "{version}");
        }
    }
}
