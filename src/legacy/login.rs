//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/login.rs`.

use crate::legacy::player_spawn_data::write_game_modes;
use crate::legacy::{LegacyWrite, LegacyWriteExt};
use pumpkin_protocol::java::client::play::CLogin;
use pumpkin_protocol::{
    VarInt,
    ser::{NetworkWriteExt, WritingError},
};
use pumpkin_util::version::JavaMinecraftVersion;

impl LegacyWrite for CLogin<'_> {
    #[expect(clippy::too_many_lines)]
    fn write_legacy(
        &self,
        mut write: impl std::io::Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_i32_be(self.entity_id)?;

        let v1_20_2 = *version >= JavaMinecraftVersion::V_1_20_2;
        let v1_20_5 = *version >= JavaMinecraftVersion::V_1_20_5;
        let v1_21_2 = *version >= JavaMinecraftVersion::V_1_21_2;
        let v1_26_2 = *version >= JavaMinecraftVersion::V_26_2;
        let v1_19 = *version >= JavaMinecraftVersion::V_1_19;
        let v1_18 = *version >= JavaMinecraftVersion::V_1_18;
        let v1_16_2 = *version >= JavaMinecraftVersion::V_1_16_2;
        let v1_16 = *version >= JavaMinecraftVersion::V_1_16;
        let v1_15 = *version >= JavaMinecraftVersion::V_1_15;
        let v1_14 = *version >= JavaMinecraftVersion::V_1_14;
        let v1_8 = *version >= JavaMinecraftVersion::V_1_8;

        // Hardcore & GameMode
        if v1_16_2 {
            write.write_bool(self.is_hardcore)?;
            if !v1_20_2 {
                write.write_u8(self.spawn_data.game_mode)?;
            }
        } else {
            let mut game_mode_id = self.spawn_data.game_mode;
            if self.is_hardcore {
                game_mode_id |= 0x08;
            }
            write.write_u8(game_mode_id)?;
        }

        // Previous GameMode & Worlds & Dimension Codec / Dimension Type
        if v1_16 {
            if !v1_20_2 {
                write.write_i8(self.spawn_data.previous_gamemode)?;
            }
            write.write_list(self.dimension_names, |write, dim| write.write_string(dim))?;
            if !v1_20_2 {
                let registry_codec = build_v1_20_registry_codec(*version);
                let nbt_bytes = pumpkin_nbt::Nbt::new(String::new(), registry_codec).write();
                write.write_all(&nbt_bytes)?;
                if v1_16_2 && *version < JavaMinecraftVersion::V_1_19 {
                    // In 1.16.2 - 1.18.2, this field is the dimension type NBT Compound!
                    let dim_type_compound =
                        get_dimension_type_nbt(*version, self.spawn_data.dimension.minecraft_name);
                    let dim_bytes = pumpkin_nbt::Nbt::new(String::new(), dim_type_compound).write();
                    write.write_all(&dim_bytes)?;
                } else {
                    // In 1.16 - 1.16.1 and 1.19 - 1.20.1, this field is the dimension Identifier string
                    write.write_string(self.spawn_data.dimension.minecraft_name)?;
                }
                write.write_string(self.spawn_data.dimension.minecraft_name)?;
            }
        } else {
            let legacy_dim_id: i32 = match self.spawn_data.dimension.minecraft_name {
                "minecraft:the_nether" => -1,
                "minecraft:the_end" => 1,
                _ => 0,
            };
            if *version >= JavaMinecraftVersion::V_1_9_1 {
                write.write_i32_be(legacy_dim_id)?;
            } else {
                write.write_i8(legacy_dim_id as i8)?;
            }
            if !v1_14 {
                // Difficulty (0: peaceful, 1: easy, 2: normal, 3: hard) - default 2 (Normal)
                write.write_u8(2)?;
            }
        }

        // Hashed Seed (Added in 1.15, moved in 1.20.2)
        if v1_15 && !v1_20_2 {
            write.write_i64_be(self.spawn_data.hashed_seed)?;
        }

        // Max Players, View Distance, etc.
        if v1_16 {
            if v1_16_2 {
                write.write_var_int(&self.max_players)?;
            } else {
                write.write_u8(self.max_players.0 as u8)?;
            }
            write.write_var_int(&self.view_distance)?;
            if v1_18 {
                write.write_var_int(&self.simulated_distance)?;
            }
            write.write_bool(self.reduced_debug_info)?;
            write.write_bool(self.enabled_respawn_screen)?;
            if v1_20_2 {
                write.write_bool(self.limited_crafting)?;
                if v1_20_5 {
                    write.write_var_int(&VarInt(self.spawn_data.dimension.id as i32))?;
                } else {
                    write.write_string(self.spawn_data.dimension.minecraft_name)?;
                }
                write.write_string(self.spawn_data.dimension.minecraft_name)?;
                write.write_i64_be(self.spawn_data.hashed_seed)?;
                write_game_modes(
                    &mut write,
                    self.spawn_data.game_mode,
                    self.spawn_data.previous_gamemode,
                    *version,
                )?;
            }
            write.write_bool(self.spawn_data.debug)?;
            write.write_bool(self.spawn_data.is_flat)?;
        } else {
            write.write_u8(self.max_players.0 as u8)?;
            let level_type = if self.spawn_data.is_flat {
                "flat"
            } else if self.spawn_data.debug {
                "debug_all_block_states"
            } else {
                "default"
            };
            write.write_string(level_type)?;
            if v1_14 {
                write.write_var_int(&self.view_distance)?;
            }
            if v1_8 {
                write.write_bool(self.reduced_debug_info)?;
            }
            if v1_15 {
                write.write_bool(self.enabled_respawn_screen)?;
            }
        }

        // Last Death Position (Added in 1.19)
        if v1_19 {
            write.write_option(
                &self.spawn_data.death_dimension_name,
                |write, (dim, pos)| {
                    write.write_string(dim)?;
                    write.write_block_pos_legacy(pos, version)?;
                    Ok(())
                },
            )?;
        }

        // Portal Cooldown (Added in 1.20)
        if *version >= JavaMinecraftVersion::V_1_20 {
            write.write_var_int(&self.spawn_data.portal_cooldown)?;
        }

        // Sea Level (Added in 1.21.2)
        if v1_21_2 {
            write.write_var_int(&self.spawn_data.sealevel)?;
        }

        // Online Mode (Added in 26.2)
        if v1_26_2 {
            write.write_bool(self.online_mode)?;
        }

        // Enforces Secure Chat (Added in 1.20.5)
        if v1_20_5 {
            write.write_bool(self.enforce_secure_chat)?;
        }

        Ok(())
    }
}

#[must_use]
pub fn build_v1_20_registry_codec(
    version: JavaMinecraftVersion,
) -> pumpkin_nbt::compound::NbtCompound {
    use pumpkin_nbt::compound::NbtCompound;
    use pumpkin_nbt::deserializer::NbtReadHelperJava;
    use pumpkin_nbt::tag::NbtTag;
    use std::io::Cursor;

    let mut root = NbtCompound::new();
    let synced = pumpkin_data::registry::Registry::get_synced(version);

    if version < JavaMinecraftVersion::V_1_16_2 {
        for reg in synced {
            let reg_name = if let Some(suffix) = reg.registry_id.strip_prefix("minecraft:") {
                suffix
            } else {
                &reg.registry_id
            };
            if reg_name == "dimension_type" {
                let mut dim_list = Vec::new();
                for entry in &reg.registry_entries {
                    if let Some(ref data) = entry.data {
                        let mut cursor = Cursor::new(&data[..]);
                        let mut reader = NbtReadHelperJava::new(&mut cursor);
                        if let Ok(element_nbt) = pumpkin_nbt::Nbt::read_unnamed(&mut reader) {
                            let mut entry_compound = element_nbt.root_tag;
                            entry_compound
                                .put("name", NbtTag::String(entry.entry_id.clone().into()));
                            dim_list.push(NbtTag::Compound(entry_compound));
                        }
                    }
                }
                root.put("dimension", NbtTag::List(dim_list));
            }
        }
        return root;
    }

    for reg in synced {
        let reg_name = if let Some(suffix) = reg.registry_id.strip_prefix("minecraft:") {
            suffix
        } else {
            &reg.registry_id
        };
        if version < JavaMinecraftVersion::V_1_20_2
            && !matches!(
                reg_name,
                "dimension_type"
                    | "worldgen/biome"
                    | "chat_type"
                    | "damage_type"
                    | "trim_pattern"
                    | "trim_material"
            )
        {
            continue;
        }

        let mut reg_compound = NbtCompound::new();
        let reg_type = if reg.registry_id.contains(':') {
            reg.registry_id.clone()
        } else {
            format!("minecraft:{}", reg.registry_id)
        };
        reg_compound.put("type", NbtTag::String(reg_type.clone().into()));

        let mut values_list = Vec::new();
        for (i, entry) in reg.registry_entries.iter().enumerate() {
            let mut entry_compound = NbtCompound::new();
            entry_compound.put("name", NbtTag::String(entry.entry_id.clone().into()));
            entry_compound.put("id", NbtTag::Int(i as i32));

            if let Some(ref data) = entry.data {
                let mut cursor = Cursor::new(&data[..]);
                let mut reader = NbtReadHelperJava::new(&mut cursor);
                if let Ok(element_nbt) = pumpkin_nbt::Nbt::read_unnamed(&mut reader) {
                    entry_compound.put("element", NbtTag::Compound(element_nbt.root_tag));
                }
            }
            values_list.push(NbtTag::Compound(entry_compound));
        }

        reg_compound.put("value", NbtTag::List(values_list));
        root.put(&reg_type, NbtTag::Compound(reg_compound));
    }

    root
}

#[must_use]
pub fn get_dimension_type_nbt(
    version: JavaMinecraftVersion,
    dimension_name: &str,
) -> pumpkin_nbt::compound::NbtCompound {
    use pumpkin_nbt::compound::NbtCompound;
    use pumpkin_nbt::deserializer::NbtReadHelperJava;
    use std::io::Cursor;

    let target_dim = dimension_name
        .strip_prefix("minecraft:")
        .unwrap_or(dimension_name);
    let synced = pumpkin_data::registry::Registry::get_synced(version);

    for reg in synced {
        let reg_name = reg
            .registry_id
            .strip_prefix("minecraft:")
            .unwrap_or(&reg.registry_id);
        if reg_name == "dimension_type" {
            for entry in &reg.registry_entries {
                let entry_id = entry
                    .entry_id
                    .strip_prefix("minecraft:")
                    .unwrap_or(&entry.entry_id);
                if entry_id == target_dim
                    && let Some(ref data) = entry.data
                {
                    let mut cursor = Cursor::new(&data[..]);
                    let mut reader = NbtReadHelperJava::new(&mut cursor);
                    if let Ok(element_nbt) = pumpkin_nbt::Nbt::read_unnamed(&mut reader) {
                        return element_nbt.root_tag;
                    }
                }
            }
            if let Some(first_entry) = reg.registry_entries.first()
                && let Some(ref data) = first_entry.data
            {
                let mut cursor = Cursor::new(&data[..]);
                let mut reader = NbtReadHelperJava::new(&mut cursor);
                if let Ok(element_nbt) = pumpkin_nbt::Nbt::read_unnamed(&mut reader) {
                    return element_nbt.root_tag;
                }
            }
        }
    }
    NbtCompound::new()
}
