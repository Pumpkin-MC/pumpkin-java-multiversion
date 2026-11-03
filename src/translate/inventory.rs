//! Inventory items: container content and slots, cursor, player inventory, equipment, and the
//! creative slots and clicks sent back.

use pumpkin_data::{
    block_properties::BLOCK_ENTITY_TYPES, data_component::DataComponent, entity::EntityType,
    item_stack::ItemStack,
};
use pumpkin_nbt::tag::NbtTag;
use pumpkin_protocol::java::legacy::{LegacyReadExt, LegacyWriteExt};
use pumpkin_protocol::{
    VarInt,
    codec::{data_component::deserialize, item_stack_seralizer::ItemStackSerializer},
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::remap::{
    data_component_type_id_remap::remap_data_component_type_id_from_version,
    item_id_remap::remap_item_id_from_version,
};
use crate::translate::{
    item::{CustomModelDataFormat, ItemFormat, NestedItemFormat, write_item_for_version},
    nbt::skip_client_nbt,
};
use pumpkin_protocol::java::legacy::slot_to_version;

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
        /// Items in the client's `ItemStack` layout, see [`ItemFormat`].
        Legacy = V_1_7_2,
        /// Length-prefixed components in creative slots, hashed stacks in clicks; same as 26.3.
        V1_21_5 = V_1_21_5,
    }
}

eras! {
    enum TooltipFormat {
        /// Show-in-tooltip flags in components, `hide_tooltip` and `hide_additional_tooltip`.
        V1_20_5 = V_1_20_5,
        /// Their ids shifted by the components added in 1.21.2.
        V1_21_2 = V_1_21_2,
        /// Tooltip display component; same as 26.3.
        V1_21_5 = V_1_21_5,
    }
}

eras! {
    enum PotionContentsFormat {
        /// No custom name.
        V1_20_5 = V_1_20_5,
        /// Same as 26.3.
        V1_21_2 = V_1_21_2,
    }
}

eras! {
    enum TypedDataFormat {
        /// Entity and block entity data carry their type as the NBT `id`.
        V1_20_5 = V_1_20_5,
        /// Type id before the NBT; same as 26.3.
        V1_21_9 = V_1_21_9,
    }
}

eras! {
    pub enum ClickFormat {
        /// Action number, byte click type, the clicked item instead of changed slots.
        V1_7 = V_1_7_2,
        /// Var int click type.
        V1_9 = V_1_9,
        /// Changed slots and carried item, no action number.
        V1_17 = V_1_17,
        /// State id.
        V1_17_1 = V_1_17_1,
    }
}

/// A stack from a client below 1.21.5, with the 26.3 item id.
enum ClientItem {
    Empty,
    Stack { id: u16, count: i32 },
}

/// `None` when unreadable: components before 1.21.5 carry no length.
// TODO: NBT and components are dropped; pre-1.13 damage values are ignored.
fn read_legacy_item(read: &mut &[u8], version: JavaMinecraftVersion) -> Option<ClientItem> {
    let format = ItemFormat::of(version);
    let (id, count) = match format {
        ItemFormat::V1_7 | ItemFormat::V1_13 => {
            let Ok(id) = u16::try_from(read.get_i16_be().ok()?) else {
                return Some(ClientItem::Empty);
            };
            let count = read.get_i8().ok()?;
            if format == ItemFormat::V1_7 {
                read.get_i16_be().ok()?; // damage
            }
            skip_client_nbt(read, version)?;
            (id, i32::from(count))
        }
        ItemFormat::V1_13_2 => {
            if !read.get_bool().ok()? {
                return Some(ClientItem::Empty);
            }
            let id = u16::try_from(read.get_var_int().ok()?.0).ok()?;
            let count = read.get_i8().ok()?;
            skip_client_nbt(read, version)?;
            (id, i32::from(count))
        }
        ItemFormat::V1_20_5 => {
            let count = read.get_var_int().ok()?.0;
            if count <= 0 {
                return Some(ClientItem::Empty);
            }
            let id = u16::try_from(read.get_var_int().ok()?.0).ok()?;
            patch_to_current(read, version, false, &mut Vec::new())?;
            (id, count)
        }
    };
    if count <= 0 {
        return Some(ClientItem::Empty);
    }
    Some(ClientItem::Stack {
        id: remap_item_id_from_version(id, version),
        count,
    })
}

/// A legacy stack as 26.3's untrusted stack, without components.
fn write_untrusted(item: &ClientItem, out: &mut Vec<u8>) -> Option<()> {
    match *item {
        ClientItem::Empty => out.write_var_int(&VarInt(0)).ok(),
        ClientItem::Stack { id, count } => {
            out.write_var_int(&VarInt(count)).ok()?;
            out.write_var_int(&VarInt(i32::from(id))).ok()?;
            out.write_var_int(&VarInt(0)).ok()?;
            out.write_var_int(&VarInt(0)).ok()
        }
    }
}

/// A legacy stack as 26.3's hashed stack, without components.
fn write_hashed(item: &ClientItem, out: &mut Vec<u8>) -> Option<()> {
    match *item {
        ClientItem::Empty => out.write_bool(false).ok(),
        ClientItem::Stack { id, count } => {
            out.write_bool(true).ok()?;
            out.write_var_int(&VarInt(i32::from(id))).ok()?;
            out.write_var_int(&VarInt(count)).ok()?;
            out.write_var_int(&VarInt(0)).ok()?;
            out.write_var_int(&VarInt(0)).ok()
        }
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
    let container_id = payload.get_container_id().ok()?;
    let state_id = payload.get_var_int().ok()?;
    let count = payload.get_var_int().ok()?;

    let mut out = Vec::with_capacity(payload.len());
    out.write_container_id_legacy(&container_id, &version)
        .ok()?;
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
    let container_id = payload.get_container_id().ok()?;
    let state_id = payload.get_var_int().ok()?;
    let slot = payload.get_i16_be().ok()?;

    let mut out = Vec::new();
    out.write_container_id_legacy(&container_id, &version)
        .ok()?;
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

/// A client component, as 26.3 knows it.
enum ClientComponent {
    Current(DataComponent),
    /// Part of the tooltip display since 1.21.5.
    HideTooltip,
    /// Part of the tooltip display since 1.21.5.
    HideAdditionalTooltip,
    /// 26.3 has no such component.
    Unknown,
}

fn read_client_component(
    read: &mut &[u8],
    version: JavaMinecraftVersion,
) -> Option<ClientComponent> {
    let id = read.get_var_int().ok()?.0;
    let hide_ids = match TooltipFormat::of(version) {
        TooltipFormat::V1_20_5 => Some((15, 14)),
        TooltipFormat::V1_21_2 => Some((16, 15)),
        TooltipFormat::V1_21_5 => None,
    };
    match hide_ids {
        Some((hide, _)) if id == hide => return Some(ClientComponent::HideTooltip),
        Some((_, additional)) if id == additional => {
            return Some(ClientComponent::HideAdditionalTooltip);
        }
        _ => {}
    }
    let mapped = remap_data_component_type_id_from_version(id as u32, version);
    let component = (mapped != 0 || id == 0)
        .then(|| DataComponent::try_from_id(u8::try_from(mapped).ok()?))
        .flatten();
    Some(component.map_or(ClientComponent::Unknown, ClientComponent::Current))
}

/// Components `hide_additional_tooltip` hid, as ViaVersion maps them.
const HIDE_ADDITIONAL: [DataComponent; 17] = [
    DataComponent::BannerPatterns,
    DataComponent::Bees,
    DataComponent::BlockEntityData,
    DataComponent::BlockState,
    DataComponent::BundleContents,
    DataComponent::ChargedProjectiles,
    DataComponent::Container,
    DataComponent::ContainerLoot,
    DataComponent::FireworkExplosion,
    DataComponent::Fireworks,
    DataComponent::Instrument,
    DataComponent::MapId,
    DataComponent::PaintingVariant,
    DataComponent::PotDecorations,
    DataComponent::PotionContents,
    DataComponent::TropicalFishPattern,
    DataComponent::WrittenBookContent,
];

/// The tooltip display of a stack from before 1.21.5, gathered from its components.
#[derive(Default)]
struct Tooltip {
    hide: bool,
    hidden: Vec<DataComponent>,
}

impl Tooltip {
    fn hide_component(&mut self, id: DataComponent) {
        if !self.hidden.contains(&id) {
            self.hidden.push(id);
        }
    }
}

/// A client's component patch as 26.3's. Only `top_level` stacks carry component lengths in 26.3,
/// and from the client since 1.21.5.
fn patch_to_current(
    read: &mut &[u8],
    version: JavaMinecraftVersion,
    top_level: bool,
    out: &mut Vec<u8>,
) -> Option<()> {
    let delimited = top_level && ClientItemFormat::of(version) == ClientItemFormat::V1_21_5;
    let added = read.get_var_int().ok()?.0;
    let removed = read.get_var_int().ok()?.0;
    let mut tooltip = Tooltip::default();
    let mut components = Vec::new();
    for _ in 0..added {
        let component = read_client_component(read, version)?;
        let mut data;
        let payload: &mut &[u8] = if delimited {
            let len = usize::try_from(read.get_var_int().ok()?.0).ok()?;
            let rest;
            (data, rest) = read.split_at_checked(len)?;
            *read = rest;
            &mut data
        } else {
            &mut *read
        };
        match component {
            ClientComponent::Current(id) => {
                let mut out = Vec::new();
                component_payload_to_current(id, payload, version, &mut out, &mut tooltip)?;
                components.push((id, out));
            }
            ClientComponent::HideTooltip => tooltip.hide = true,
            ClientComponent::HideAdditionalTooltip => {
                HIDE_ADDITIONAL
                    .into_iter()
                    .for_each(|id| tooltip.hide_component(id));
            }
            // Without a length its end is unknown
            ClientComponent::Unknown if !delimited => return None,
            ClientComponent::Unknown => {}
        }
    }
    if tooltip.hide || !tooltip.hidden.is_empty() {
        let mut payload = Vec::new();
        payload.write_bool(tooltip.hide).ok()?;
        payload
            .write_var_int(&VarInt(tooltip.hidden.len() as i32))
            .ok()?;
        for id in tooltip.hidden {
            payload.write_var_int(&VarInt(i32::from(id.to_id()))).ok()?;
        }
        components.push((DataComponent::TooltipDisplay, payload));
    }
    let mut removed_ids = Vec::new();
    for _ in 0..removed {
        if let ClientComponent::Current(id) = read_client_component(read, version)? {
            removed_ids.push(id);
        }
    }

    out.write_var_int(&VarInt(components.len() as i32)).ok()?;
    out.write_var_int(&VarInt(removed_ids.len() as i32)).ok()?;
    for (id, payload) in components {
        out.write_var_int(&VarInt(i32::from(id.to_id()))).ok()?;
        if top_level {
            out.write_var_int(&VarInt(payload.len() as i32)).ok()?;
        }
        out.extend_from_slice(&payload);
    }
    for id in removed_ids {
        out.write_var_int(&VarInt(i32::from(id.to_id()))).ok()?;
    }
    Some(())
}

/// An optional stack with length-prefixed components (unprefixed before 1.21.5), as 26.3's ids
/// and payloads.
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
    patch_to_current(read, version, true, out)
}

/// Copies a payload already in the 26.3 format, measured by the core's decoder.
fn copy_payload(id: DataComponent, read: &mut &[u8], out: &mut Vec<u8>) -> Option<()> {
    let start = *read;
    deserialize(id, read).ok()?;
    out.extend_from_slice(&start[..start.len() - read.len()]);
    Some(())
}

/// Skips status effect details: amplifier, duration, three flags and the optional hidden effect.
fn skip_effect_details(read: &mut &[u8]) -> Option<()> {
    read.get_var_int().ok()?;
    read.get_var_int().ok()?;
    for _ in 0..3 {
        read.get_bool().ok()?;
    }
    if read.get_bool().ok()? {
        skip_effect_details(read)?;
    }
    Some(())
}

/// A client component payload in the 26.3 format. Components holding items get them rewritten.
// TODO: components whose format changed since the client's version and are not converted here
// fail to read and drop the stack.
fn component_payload_to_current(
    id: DataComponent,
    read: &mut &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
    tooltip: &mut Tooltip,
) -> Option<()> {
    let show_flags = TooltipFormat::of(version) < TooltipFormat::V1_21_5;
    match id {
        DataComponent::BundleContents | DataComponent::ChargedProjectiles => {
            let len = read.get_var_int().ok()?;
            out.write_var_int(&len).ok()?;
            for _ in 0..len.0 {
                nested_item_to_current(read, version, out)?;
            }
        }
        DataComponent::UseRemainder => nested_item_to_current(read, version, out)?,
        DataComponent::Container => {
            let len = read.get_var_int().ok()?;
            out.write_var_int(&len).ok()?;
            for _ in 0..len.0 {
                let present = match NestedItemFormat::of(version) {
                    NestedItemFormat::V1_20_5 => {
                        // Empty slots are a 0 count, left unread for a stack
                        let mut peek = *read;
                        let empty = peek.get_var_int().ok()?.0 <= 0;
                        if empty {
                            *read = peek;
                        }
                        !empty
                    }
                    NestedItemFormat::V26_1 => read.get_bool().ok()?,
                };
                out.write_bool(present).ok()?;
                if present {
                    nested_item_to_current(read, version, out)?;
                }
            }
        }
        // Show-in-tooltip flags moved to the tooltip display
        DataComponent::Unbreakable if show_flags => {
            if !read.get_bool().ok()? {
                tooltip.hide_component(id);
            }
        }
        DataComponent::Enchantments
        | DataComponent::StoredEnchantments
        | DataComponent::DyedColor
            if show_flags =>
        {
            copy_payload(id, read, out)?;
            if !read.get_bool().ok()? {
                tooltip.hide_component(id);
            }
        }
        // Not converted yet; reading them as 26.3's would misread the rest of the stack
        DataComponent::CanPlaceOn
        | DataComponent::CanBreak
        | DataComponent::AttributeModifiers
        | DataComponent::Trim
        | DataComponent::JukeboxPlayable
        | DataComponent::Tool
        | DataComponent::Instrument
        | DataComponent::Profile
        | DataComponent::Bees
        | DataComponent::Equippable
            if show_flags =>
        {
            return None;
        }
        DataComponent::CustomModelData
            if CustomModelDataFormat::of(version) == CustomModelDataFormat::V1_20_5 =>
        {
            // The value becomes the only float, as ViaVersion maps it
            let value = read.get_var_int().ok()?.0;
            out.write_var_int(&VarInt(1)).ok()?;
            out.write_f32_be(value as f32).ok()?;
            for _ in 0..3 {
                out.write_var_int(&VarInt(0)).ok()?;
            }
        }
        DataComponent::PotionContents
            if PotionContentsFormat::of(version) == PotionContentsFormat::V1_20_5 =>
        {
            let start = *read;
            if read.get_bool().ok()? {
                read.get_var_int().ok()?;
            }
            if read.get_bool().ok()? {
                read.get_i32_be().ok()?;
            }
            for _ in 0..read.get_var_int().ok()?.0 {
                read.get_var_int().ok()?;
                skip_effect_details(read)?;
            }
            out.extend_from_slice(&start[..start.len() - read.len()]);
            out.write_bool(false).ok()?; // custom name
        }
        DataComponent::EntityData | DataComponent::BlockEntityData
            if TypedDataFormat::of(version) == TypedDataFormat::V1_20_5 =>
        {
            let Some(NbtTag::Compound(mut nbt)) = read.get_nbt_owned().ok()? else {
                return None;
            };
            let name = nbt.get_string("id")?;
            let name = name.strip_prefix("minecraft:").unwrap_or(name);
            let type_id = if id == DataComponent::EntityData {
                i32::from(EntityType::from_name(name)?.id)
            } else {
                BLOCK_ENTITY_TYPES.iter().position(|n| *n == name)? as i32
            };
            nbt.child_tags.remove("id");
            out.write_var_int(&VarInt(type_id)).ok()?;
            out.write_compound_nbt(&nbt).ok()?;
        }
        _ => copy_payload(id, read, out)?,
    }
    Some(())
}

/// A stack inside a component as 26.3's `ItemStackTemplate`, its components without length.
fn nested_item_to_current(
    read: &mut &[u8],
    version: JavaMinecraftVersion,
    out: &mut Vec<u8>,
) -> Option<()> {
    let count = if NestedItemFormat::of(version) == NestedItemFormat::V1_20_5 {
        Some(read.get_var_int().ok()?)
    } else {
        None
    };
    write_item_id_to_current(read, version, out)?;
    let count = match count {
        Some(count) => count,
        None => read.get_var_int().ok()?,
    };
    out.write_var_int(&count).ok()?;
    patch_to_current(read, version, false, out)
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

/// SET_CREATIVE_MODE_SLOT. `None` drops it, so no item with the client's id gets stored.
pub fn creative_slot_to_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    out.write_i16_be(payload.get_i16_be().ok()?).ok()?;
    if ItemFormat::of(version) == ItemFormat::V1_20_5 {
        untrusted_item_to_current(&mut payload, version, &mut out)?;
        // Left over bytes mean a component was misread
        if !payload.is_empty() {
            return None;
        }
    } else {
        write_untrusted(&read_legacy_item(&mut payload, version)?, &mut out)?;
    }
    Some(out)
}

/// CONTAINER_CLICK. `None` drops it.
pub fn container_click_to_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    if ClientItemFormat::of(version) == ClientItemFormat::Legacy {
        return legacy_click_to_current(payload, version);
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

/// CONTAINER_CLICK before 1.21.5. The server replays the click itself; the stacks only sync it,
/// so unreadable or missing ones are sent empty and the server corrects the client.
fn legacy_click_to_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let format = ClickFormat::of(version);
    let container = payload.get_container_id_legacy(&version).ok()?;
    // A wrong state id makes the server resend the whole container
    let state_id = if format >= ClickFormat::V1_17_1 {
        payload.get_var_int().ok()?
    } else {
        VarInt(-1)
    };
    let slot = payload.get_i16_be().ok()?;
    let button = payload.get_i8().ok()?;
    if format < ClickFormat::V1_17 {
        payload.get_i16_be().ok()?; // action number
    }
    let mode = if format >= ClickFormat::V1_9 {
        payload.get_var_int().ok()?
    } else {
        VarInt(i32::from(payload.get_i8().ok()?))
    };

    let mut stacks = Vec::new();
    let mut carried = ClientItem::Empty;
    if format >= ClickFormat::V1_17 {
        let read_stacks = |payload: &mut &[u8]| -> Option<(Vec<(i16, ClientItem)>, ClientItem)> {
            let count = payload.get_var_int().ok()?.0;
            let mut changed = Vec::new();
            for _ in 0..count {
                let slot = payload.get_i16_be().ok()?;
                changed.push((slot, read_legacy_item(payload, version)?));
            }
            Some((changed, read_legacy_item(payload, version)?))
        };
        if let Some((changed, item)) = read_stacks(&mut payload) {
            stacks = changed;
            carried = item;
        }
    }

    let mut out = Vec::new();
    out.write_var_int(&container).ok()?;
    out.write_var_int(&state_id).ok()?;
    out.write_i16_be(slot).ok()?;
    out.write_i8(button).ok()?;
    out.write_var_int(&mode).ok()?;
    out.write_var_int(&VarInt(stacks.len() as i32)).ok()?;
    for (slot, item) in &stacks {
        out.write_i16_be(*slot).ok()?;
        write_hashed(item, &mut out)?;
    }
    write_hashed(&carried, &mut out)?;
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
            .write_packet_data(&mut current)
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

    #[test]
    fn creative_bundle_items_become_templates() {
        use crate::remap::data_component_type_id_remap::remap_data_component_type_id_for_version;
        let client_component = |id: DataComponent| {
            remap_data_component_type_id_for_version(u32::from(id.to_id()), V1_21_11) as i32
        };
        let client_item = |item: &Item| i32::from(remap_item_id_for_version(item.id, V1_21_11));
        let var_ints = |values: &[i32]| {
            let mut out = Vec::new();
            for &value in values {
                out.write_var_int(&VarInt(value)).unwrap();
            }
            out
        };

        // One dirt as count, id, no components
        let client_payload = var_ints(&[1, 2, client_item(&Item::DIRT), 0, 0]);
        let mut client = vec![0, 36];
        client.extend(var_ints(&[
            1,
            client_item(&Item::BUNDLE),
            1,
            0,
            client_component(DataComponent::BundleContents),
            client_payload.len() as i32,
        ]));
        client.extend(client_payload);

        // The same dirt as id, count, no components
        let payload = var_ints(&[1, i32::from(Item::DIRT.id), 2, 0, 0]);
        let mut expected = vec![0, 36];
        expected.extend(var_ints(&[
            1,
            i32::from(Item::BUNDLE.id),
            1,
            0,
            i32::from(DataComponent::BundleContents.to_id()),
            payload.len() as i32,
        ]));
        expected.extend(payload);
        assert_eq!(creative_slot_to_current(&client, V1_21_11), Some(expected));
    }

    /// 26.3 untrusted stack: count, id, no components.
    fn current_untrusted(count: u8, id: u16) -> Vec<u8> {
        let mut out = Vec::new();
        out.write_var_int(&VarInt(i32::from(count))).unwrap();
        out.write_var_int(&VarInt(i32::from(id))).unwrap();
        out.extend_from_slice(&[0, 0]);
        out
    }

    #[test]
    fn legacy_creative_slot_gets_the_current_id() {
        // 1.20: present, id, count, named NBT with a damage entry
        let version = JavaMinecraftVersion::V_1_20;
        let mut client = Vec::new();
        client.write_i16_be(36).unwrap();
        client.write_bool(true).unwrap();
        client
            .write_var_int(&VarInt(i32::from(remap_item_id_for_version(
                Item::DIRT.id,
                version,
            ))))
            .unwrap();
        client.write_i8(2).unwrap();
        client.extend_from_slice(&[
            10, 0, 0, 3, 0, 6, b'D', b'a', b'm', b'a', b'g', b'e', 0, 0, 0, 1, 0,
        ]);
        let mut expected = vec![0, 36];
        expected.extend(current_untrusted(2, Item::DIRT.id));
        assert_eq!(creative_slot_to_current(&client, version), Some(expected));
    }

    #[test]
    fn creative_slot_reads_unsized_components() {
        let version = JavaMinecraftVersion::V_1_21_4;
        let dirt = remap_item_id_for_version(Item::DIRT.id, version);
        let creative = |components: &[u8]| {
            let mut client = vec![0, 36, 1];
            client.write_var_int(&VarInt(i32::from(dirt))).unwrap();
            client.extend_from_slice(components);
            creative_slot_to_current(&client, version)
        };
        let expected = |components: &[u8]| {
            let mut out = vec![0, 36, 1];
            out.write_var_int(&VarInt(i32::from(Item::DIRT.id)))
                .unwrap();
            out.extend_from_slice(components);
            Some(out)
        };

        // Max stack size 5, which has no length before 1.21.5 but one in 26.3
        assert_eq!(creative(&[1, 0, 1, 5]), expected(&[1, 0, 1, 1, 5]));
        // Stored enchantments hidden from the tooltip, hide tooltip
        assert_eq!(
            creative(&[2, 0, 33, 1, 0, 3, 0, 16]),
            expected(&[2, 0, 45, 3, 1, 0, 3, 18, 3, 1, 1, 45])
        );
        // Attribute modifiers are not converted yet
        assert_eq!(creative(&[1, 0, 13, 0, 1]), None);
        // Bytes left over after the stack
        assert_eq!(creative(&[1, 0, 1, 5, 0]), None);
    }

    #[test]
    fn creative_custom_model_data_becomes_a_float() {
        let version = JavaMinecraftVersion::V_1_21;
        let dirt = remap_item_id_for_version(Item::DIRT.id, version);
        let mut client = vec![0, 36, 1];
        client.write_var_int(&VarInt(i32::from(dirt))).unwrap();
        client.extend_from_slice(&[1, 0, 13, 7]);

        let mut expected = vec![0, 36, 1];
        expected
            .write_var_int(&VarInt(i32::from(Item::DIRT.id)))
            .unwrap();
        expected.extend_from_slice(&[1, 0, 17, 8, 1]);
        expected.extend_from_slice(&7.0f32.to_be_bytes());
        expected.extend_from_slice(&[0, 0, 0]);
        assert_eq!(creative_slot_to_current(&client, version), Some(expected));
    }

    #[test]
    fn legacy_clicks_become_26_3_clicks() {
        let version = JavaMinecraftVersion::V_1_21_4;
        let dirt = remap_item_id_for_version(Item::DIRT.id, version);
        // container, state id, slot, button, click type, one changed slot, carried
        let mut client = vec![0, 7, 0, 36, 0, 0, 1, 0, 36];
        client.extend(current_untrusted(1, dirt));
        client.push(0);
        let mut expected = vec![0, 7, 0, 36, 0, 0, 1, 0, 36, 1];
        expected
            .write_var_int(&VarInt(i32::from(Item::DIRT.id)))
            .unwrap();
        expected.extend_from_slice(&[1, 0, 0, 0]);
        assert_eq!(container_click_to_current(&client, version), Some(expected));

        // 1.16: action number, the clicked item and no state id
        let version = JavaMinecraftVersion::V_1_16;
        let client = [1, 0, 36, 0, 0, 5, 0, 0];
        let mut expected = vec![1];
        expected.write_var_int(&VarInt(-1)).unwrap();
        expected.extend_from_slice(&[0, 36, 0, 0, 0, 0]);
        assert_eq!(container_click_to_current(&client, version), Some(expected));
    }
}
