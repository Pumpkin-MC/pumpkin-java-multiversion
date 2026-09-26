//! Recipe displays: RECIPE_BOOK_ADD, PLACE_GHOST_RECIPE, UPDATE_RECIPES.

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    VarInt,
    codec::item_stack_seralizer::ItemStackSerializer,
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use super::item::{write_item_for_version, write_template_for_version};
use crate::remap::{
    data_component_type_id_remap::remap_data_component_type_id_for_version,
    item_id_remap::remap_item_id_for_version,
    slot_display_id_remap::remap_slot_display_id_for_version,
};

eras! {
    pub enum RecipeFormat {
        /// Recipe displays added; tag slot displays hold a tag key, item stacks are full
        /// stacks, the smithing trim pattern is a slot display.
        V1_21_2 = V_1_21_2,
        /// Smithing trim pattern is a trim pattern holder.
        V1_21_5 = V_1_21_5,
        /// Item stack slot displays are templates.
        V26_1 = V_26_1,
        /// Tag slot displays hold an item holder set (26.3).
        V26_3 = V_26_3,
    }
}

// Current (26.3) slot display type ids
const EMPTY: i32 = 0;
const ANY_FUEL: i32 = 1;
const WITH_ANY_POTION: i32 = 2;
const ONLY_WITH_COMPONENT: i32 = 3;
const ITEM: i32 = 4;
const ITEM_STACK: i32 = 5;
const TAG: i32 = 6;
const DYED: i32 = 7;
const SMITHING_TRIM: i32 = 8;
const WITH_REMAINDER: i32 = 9;
const COMPOSITE: i32 = 10;

struct Writer {
    version: JavaMinecraftVersion,
    format: RecipeFormat,
    out: Vec<u8>,
}

impl Writer {
    fn new(version: JavaMinecraftVersion) -> Self {
        Self {
            version,
            format: RecipeFormat::of(version),
            out: Vec::new(),
        }
    }

    fn var_int(&mut self, value: i32) -> Option<()> {
        self.out.write_var_int(&VarInt(value)).ok()
    }

    fn item_id(&mut self, id: i32) -> Option<()> {
        let id = remap_item_id_for_version(u16::try_from(id).ok()?, self.version);
        self.var_int(i32::from(id))
    }

    /// Client slot display type, `None` when the client lacks it.
    fn slot_type(&self, current: i32) -> Option<i32> {
        let mapped = remap_slot_display_id_for_version(current as u32, self.version) as i32;
        (current == EMPTY || mapped != EMPTY).then_some(mapped)
    }

    /// Count 0 followed by a tag key, or count + 1 item ids.
    fn holder_set(&mut self, read: &mut &[u8]) -> Option<()> {
        let count = read.get_var_int().ok()?.0;
        self.var_int(count)?;
        if count == 0 {
            return self.out.write_string(read.get_str_borrowed().ok()?).ok();
        }
        for _ in 1..count {
            self.item_id(read.get_var_int().ok()?.0)?;
        }
        Some(())
    }

    fn slot_display_list(&mut self, read: &mut &[u8]) -> Option<()> {
        let count = read.get_var_int().ok()?.0;
        self.var_int(count)?;
        for _ in 0..count {
            self.slot_display(read)?;
        }
        Some(())
    }

    /// Displays the client lacks are replaced by the display they wrap.
    fn slot_display(&mut self, read: &mut &[u8]) -> Option<()> {
        let current = read.get_var_int().ok()?.0;
        let client = self.slot_type(current);
        match current {
            EMPTY | ANY_FUEL => self.var_int(client?),
            WITH_ANY_POTION => {
                if let Some(client) = client {
                    self.var_int(client)?;
                }
                self.slot_display(read)
            }
            ONLY_WITH_COMPONENT => {
                if let Some(client) = client {
                    self.var_int(client)?;
                    self.slot_display(read)?;
                    let component = read.get_var_int().ok()?.0 as u32;
                    let component =
                        remap_data_component_type_id_for_version(component, self.version);
                    self.var_int(component as i32)
                } else {
                    self.slot_display(read)?;
                    read.get_var_int().ok().map(|_| ())
                }
            }
            ITEM => {
                self.var_int(client?)?;
                self.item_id(read.get_var_int().ok()?.0)
            }
            ITEM_STACK => {
                self.var_int(client?)?;
                let stack = ItemStackSerializer::read_template0(read, &CURRENT_MC_VERSION)
                    .ok()?
                    .to_stack();
                if self.format >= RecipeFormat::V26_1 {
                    write_template_for_version(&stack, self.version, &mut self.out).ok()
                } else {
                    write_item_for_version(&stack, self.version, &mut self.out).ok()
                }
            }
            TAG => self.tag(read, client?),
            DYED => {
                if let Some(client) = client {
                    self.var_int(client)?;
                    self.slot_display(read)?;
                } else {
                    // The dye is dropped, the target stays
                    self.skip_slot_display(read)?;
                }
                self.slot_display(read)
            }
            SMITHING_TRIM => {
                // TODO: before 1.21.5 the pattern is a slot display.
                if self.format < RecipeFormat::V1_21_5 {
                    return None;
                }
                self.var_int(client?)?;
                self.slot_display(read)?;
                self.slot_display(read)?;
                // Registry holder, 0 would be an inline pattern
                let pattern = read.get_var_int().ok()?.0;
                (pattern != 0).then_some(())?;
                self.var_int(pattern)
            }
            WITH_REMAINDER => {
                self.var_int(client?)?;
                self.slot_display(read)?;
                self.slot_display(read)
            }
            COMPOSITE => {
                self.var_int(client?)?;
                self.slot_display_list(read)
            }
            _ => None,
        }
    }

    /// Before 26.3 a tag key; an item list becomes a composite of item displays.
    fn tag(&mut self, read: &mut &[u8], client: i32) -> Option<()> {
        if self.format == RecipeFormat::V26_3 {
            self.var_int(client)?;
            return self.holder_set(read);
        }
        let count = read.get_var_int().ok()?.0;
        if count == 0 {
            self.var_int(client)?;
            let key = read.get_str_borrowed().ok()?;
            return self
                .out
                .write_string(key.strip_prefix('#').unwrap_or(key))
                .ok();
        }
        self.var_int(self.slot_type(COMPOSITE)?)?;
        self.var_int(count - 1)?;
        let item = self.slot_type(ITEM)?;
        for _ in 1..count {
            self.var_int(item)?;
            self.item_id(read.get_var_int().ok()?.0)?;
        }
        Some(())
    }

    fn skip_slot_display(&mut self, read: &mut &[u8]) -> Option<()> {
        let mut scratch = Self {
            version: self.version,
            format: self.format,
            out: Vec::new(),
        };
        scratch.slot_display(read)
    }

    fn recipe_display(&mut self, read: &mut &[u8]) -> Option<()> {
        let kind = read.get_var_int().ok()?.0;
        self.var_int(kind)?;
        match kind {
            // shapeless
            0 => {
                self.slot_display_list(read)?;
                self.slot_display(read)?;
                self.slot_display(read)
            }
            // shaped
            1 => {
                self.var_int(read.get_var_int().ok()?.0)?;
                self.var_int(read.get_var_int().ok()?.0)?;
                self.slot_display_list(read)?;
                self.slot_display(read)?;
                self.slot_display(read)
            }
            // furnace
            2 => {
                for _ in 0..4 {
                    self.slot_display(read)?;
                }
                self.var_int(read.get_var_int().ok()?.0)?;
                self.out.write_f32_be(read.get_f32_be().ok()?).ok()
            }
            // stonecutter
            3 => (0..3).try_for_each(|_| self.slot_display(read)),
            // smithing
            4 => (0..5).try_for_each(|_| self.slot_display(read)),
            _ => None,
        }
    }
}

/// RECIPE_BOOK_ADD: display id, display, group, category, requirements, flags per entry,
/// then the replace flag.
pub fn recipe_book_add_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let read = &mut payload;
    let mut writer = Writer::new(version);
    let count = read.get_var_int().ok()?.0;
    writer.var_int(count)?;
    for _ in 0..count {
        writer.var_int(read.get_var_int().ok()?.0)?;
        writer.recipe_display(read)?;
        // group (optional var int) and category
        writer.var_int(read.get_var_int().ok()?.0)?;
        writer.var_int(read.get_var_int().ok()?.0)?;
        let has_requirements = read.get_bool().ok()?;
        writer.out.write_bool(has_requirements).ok()?;
        if has_requirements {
            let ingredients = read.get_var_int().ok()?.0;
            writer.var_int(ingredients)?;
            for _ in 0..ingredients {
                writer.holder_set(read)?;
            }
        }
        writer.out.write_u8(read.get_u8().ok()?).ok()?;
    }
    writer.out.extend_from_slice(read);
    Some(writer.out)
}

/// PLACE_GHOST_RECIPE: container id, display.
pub fn place_ghost_recipe_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let read = &mut payload;
    let mut writer = Writer::new(version);
    writer.var_int(read.get_var_int().ok()?.0)?;
    writer.recipe_display(read)?;
    Some(writer.out)
}

/// UPDATE_RECIPES: item property sets, stonecutter recipes.
pub fn update_recipes_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let read = &mut payload;
    let mut writer = Writer::new(version);
    let sets = read.get_var_int().ok()?.0;
    writer.var_int(sets)?;
    for _ in 0..sets {
        writer
            .out
            .write_string(read.get_str_borrowed().ok()?)
            .ok()?;
        let items = read.get_var_int().ok()?.0;
        writer.var_int(items)?;
        for _ in 0..items {
            writer.item_id(read.get_var_int().ok()?.0)?;
        }
    }
    let stonecutter = read.get_var_int().ok()?.0;
    writer.var_int(stonecutter)?;
    for _ in 0..stonecutter {
        writer.holder_set(read)?;
        writer.slot_display(read)?;
    }
    Some(writer.out)
}

#[cfg(test)]
mod tests {
    use pumpkin_protocol::{ClientPacket, java::client::play::CRecipeBookAdd};

    use super::*;

    /// Walks a 1.21.11 slot display (types: empty, any fuel, item, item stack, tag, smithing
    /// trim, with remainder, composite).
    fn skip_slot_display_1_21_11(read: &mut &[u8]) {
        match read.get_var_int().unwrap().0 {
            0 | 1 => {}
            2 => {
                read.get_var_int().unwrap();
            }
            3 => {
                assert!(read.get_var_int().unwrap().0 > 0);
                read.get_var_int().unwrap();
                assert_eq!(read.get_var_int().unwrap().0, 0);
                assert_eq!(read.get_var_int().unwrap().0, 0);
            }
            4 => {
                read.get_str_borrowed().unwrap();
            }
            5 => {
                skip_slot_display_1_21_11(read);
                skip_slot_display_1_21_11(read);
                read.get_var_int().unwrap();
            }
            6 => {
                skip_slot_display_1_21_11(read);
                skip_slot_display_1_21_11(read);
            }
            7 => {
                for _ in 0..read.get_var_int().unwrap().0 {
                    skip_slot_display_1_21_11(read);
                }
            }
            other => panic!("unknown 1.21.11 slot display {other}"),
        }
    }

    #[test]
    fn recipe_book_add_walks_as_1_21_11() {
        let mut payload = Vec::new();
        CRecipeBookAdd::new(false, &[])
            .write_packet_data(&mut payload, &CURRENT_MC_VERSION)
            .unwrap();
        let out = recipe_book_add_from_current(&payload, JavaMinecraftVersion::V_1_21_11).unwrap();

        let read = &mut out.as_slice();
        let count = read.get_var_int().unwrap().0;
        assert!(count > 100);
        for _ in 0..count {
            read.get_var_int().unwrap();
            let kind = read.get_var_int().unwrap().0;
            let slots = match kind {
                0 => {
                    for _ in 0..read.get_var_int().unwrap().0 {
                        skip_slot_display_1_21_11(read);
                    }
                    2
                }
                1 => {
                    read.get_var_int().unwrap();
                    read.get_var_int().unwrap();
                    for _ in 0..read.get_var_int().unwrap().0 {
                        skip_slot_display_1_21_11(read);
                    }
                    2
                }
                2 => 4,
                other => panic!("unexpected display {other}"),
            };
            for _ in 0..slots {
                skip_slot_display_1_21_11(read);
            }
            if kind == 2 {
                read.get_var_int().unwrap();
                read.get_f32_be().unwrap();
            }
            read.get_var_int().unwrap();
            read.get_var_int().unwrap();
            if read.get_bool().unwrap() {
                for _ in 0..read.get_var_int().unwrap().0 {
                    let n = read.get_var_int().unwrap().0;
                    if n == 0 {
                        read.get_str_borrowed().unwrap();
                    }
                    for _ in 1..n {
                        read.get_var_int().unwrap();
                    }
                }
            }
            read.get_u8().unwrap();
        }
        // replace flag
        read.get_bool().unwrap();
        assert!(read.is_empty());
    }
}
