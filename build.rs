//! Per-version synced registries from `assets/datapacks`, as network NBT, and block id remaps
//! from the ViaBackwards mappings.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use pumpkin_nbt::{Nbt, compound::NbtCompound, tag::NbtTag};
use serde_json::Value;

/// Datapack folders sent as one registry per packet (1.20.5+).
const VERSIONS: &[&str] = &[
    "1_21", "1_21_2", "1_21_4", "1_21_5", "1_21_6", "1_21_7", "1_21_9", "1_21_11", "26_1", "26_2",
];

/// Datapack folder sent as a single registry codec (1.20.2 - 1.20.4).
const CODEC_VERSION: &str = "1_20_2";

/// Datapack folders whose registry codec is part of the login packet (1.16 - 1.20.1).
const LOGIN_CODEC_VERSIONS: &[&str] = &["1_16", "1_16_2", "1_17", "1_18", "1_19", "1_20"];

/// Datapack folders whose login and respawn packets carry the dimension type (1.16.2 - 1.18.2).
const DIMENSION_TYPE_VERSIONS: &[&str] = &["1_16_2", "1_17", "1_18"];

/// Datapack folders whose entry order is needed to remap registry ids.
const NAME_VERSIONS: &[&str] = &[
    "1_16", "1_16_2", "1_17", "1_18", "1_19", "1_20", "1_20_2", "1_21", "1_21_2", "1_21_4",
    "1_21_5", "1_21_6", "1_21_7", "1_21_9", "1_21_11", "26_1", "26_2", "26_3",
];

/// Same list as core's codegen. A version syncs the ones its datapack has.
const SYNCED_REGISTRIES: &[&str] = &[
    "worldgen/biome",
    "chat_type",
    "trim_pattern",
    "trim_material",
    "wolf_variant",
    "wolf_sound_variant",
    "pig_variant",
    "pig_sound_variant",
    "frog_variant",
    "cat_variant",
    "cat_sound_variant",
    "cow_variant",
    "cow_sound_variant",
    "chicken_variant",
    "chicken_sound_variant",
    "zombie_nautilus_variant",
    "painting_variant",
    "dimension_type",
    "damage_type",
    "jukebox_song",
    "banner_pattern",
    "instrument",
    "enchantment",
    "timeline",
    "dialog",
    "world_clock",
    "test_environment",
    "test_instance",
    "sulfur_cube_archetype",
    "decorated_pot_pattern",
    "block_transformer",
    "worldgen/block_state_provider",
];

/// `assets/tracked_data` folders, newest first, and whether they use Mojang field names.
const ENTITY_DATA_VERSIONS: &[(&str, bool)] = &[
    ("26_2", true),
    ("26_1", true),
    ("1_21_11", false),
    ("1_21_9", false),
    ("1_21_7", false),
    ("1_21_6", false),
    ("1_21_5", false),
    ("1_21_4", false),
    ("1_21_2", false),
    ("1_21", false),
];

/// Older serializer names with the same wire format, as 26.3 calls them.
const SERIALIZER_RENAMES: &[(&str, &str)] = &[
    ("integer", "int"),
    ("text_component", "component"),
    ("optional_text_component", "optional_component"),
    ("rotation", "rotations"),
    ("facing", "direction"),
    ("lazy_entity_reference", "optional_living_entity_reference"),
    ("optional_uuid", "optional_living_entity_reference"),
    ("particle_list", "particles"),
    ("optional_int", "optional_unsigned_int"),
    ("entity_pose", "pose"),
    ("oxidation_level", "weathering_copper_state"),
    ("vector_3f", "vector3"),
    ("vector3f", "vector3"),
    ("quaternion_f", "quaternion"),
    ("quaternionf", "quaternion"),
    ("profile", "resolvable_profile"),
    ("arm", "humanoid_arm"),
];

/// 26.1 fields 1.21.11 lacks, besides those with serializers it lacks. The rest keep their order.
/// TODO more Versions may add fields, but 26.1 is the only one that removed any
const FIELDS_ADDED_IN_26_1: &[&str] = &["AGE_LOCKED", "DATA_VILLAGER_DATA_FINALIZED"];

/// Base `Entity` fields, the same in every version with tracked data.
const BASE_FIELDS: u8 = 8;

struct Field {
    name: String,
    id: u8,
    serializer: String,
}

fn serializer_name(name: &str) -> String {
    SERIALIZER_RENAMES
        .iter()
        .find(|(old, _)| *old == name)
        .map_or(name, |(_, new)| new)
        .to_string()
}

/// Entity name to its fields, sorted by id.
fn load_tracked(folder: &str) -> HashMap<String, Vec<Field>> {
    let path = format!("assets/tracked_data/{folder}_tracked_data.json");
    let json: HashMap<String, HashMap<String, Value>> =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    json.into_iter()
        .map(|(entity, fields)| {
            let mut fields: Vec<_> = fields
                .into_iter()
                .map(|(name, field)| Field {
                    name,
                    id: field["id"].as_u64().unwrap() as u8,
                    serializer: serializer_name(field["type"].as_str().unwrap()),
                })
                .collect();
            fields.sort_by_key(|f| f.id);
            (entity, fields)
        })
        .collect()
}

fn load_serializers(folder: &str) -> HashMap<String, i64> {
    let path = format!("assets/meta_data_type/{folder}_meta_data_type.json");
    let json: HashMap<String, i64> =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    json.into_iter()
        .map(|(name, id)| (serializer_name(&name), id))
        .collect()
}

/// Field ids of `older` by field id of `newer`, for one entity.
fn step_fields(
    newer: &[Field],
    older: &[Field],
    same_names: bool,
    older_serializers: &HashMap<String, i64>,
    entity: &str,
) -> HashMap<u8, u8> {
    if same_names {
        return newer
            .iter()
            .filter_map(|n| {
                let o = older
                    .iter()
                    .find(|o| o.name == n.name && o.serializer == n.serializer)?;
                Some((n.id, o.id))
            })
            .collect();
    }
    // Mojang to Yarn names (26.1 to 1.21.11): only added fields differ
    let kept: Vec<_> = newer
        .iter()
        .filter(|n| {
            !FIELDS_ADDED_IN_26_1.contains(&n.name.as_str())
                && older_serializers.contains_key(&n.serializer)
        })
        .collect();
    assert!(
        kept.len() == older.len()
            && kept
                .iter()
                .zip(older)
                .all(|(n, o)| n.serializer == o.serializer),
        "entity data of {entity} does not line up between 26.1 and 1.21.11"
    );
    kept.iter().zip(older).map(|(n, o)| (n.id, o.id)).collect()
}

/// Per client version: serializer ids and field ids by 26.3's, 255 / -1 where the client lacks them.
fn entity_data_tables() -> String {
    let current = load_tracked("26_3");
    let current_serializers = load_serializers("26_3");
    let mut current_serializer_names: Vec<_> = current_serializers.iter().collect();
    current_serializer_names.sort_by_key(|(_, id)| **id);

    let mut entities: Vec<_> = current.keys().cloned().collect();
    entities.sort();
    // 26.3 field id to the id in the version processed last
    let mut state: HashMap<&str, Vec<Option<u8>>> = entities
        .iter()
        .map(|e| {
            let len = current[e].last().map_or(0, |f| usize::from(f.id) + 1);
            (e.as_str(), (0..len).map(|id| Some(id as u8)).collect())
        })
        .collect();

    let mut out = String::from("/* Generated by build.rs from assets/tracked_data. */\n");
    let mut newer = current;
    let mut newer_mojang = true;
    for &(folder, mojang) in ENTITY_DATA_VERSIONS {
        let older = load_tracked(folder);
        let older_serializers = load_serializers(folder);
        for entity in &entities {
            let ids = state.get_mut(entity.as_str()).unwrap();
            match (newer.get(entity), older.get(entity)) {
                (Some(n), Some(o)) => {
                    let step =
                        step_fields(n, o, newer_mojang == mojang, &older_serializers, entity);
                    for id in ids.iter_mut() {
                        *id = id.and_then(|id| step.get(&id).copied());
                    }
                }
                // The client spawns another entity; only the base fields carry over
                _ => {
                    for (i, id) in ids.iter_mut().enumerate() {
                        *id = id.filter(|_| i < usize::from(BASE_FIELDS));
                    }
                }
            }
        }

        let serializers: Vec<i64> = current_serializer_names
            .iter()
            .map(|(name, _)| older_serializers.get(*name).copied().unwrap_or(-1))
            .collect();
        let _ = write!(
            out,
            "pub static ENTITY_DATA_{}: EntityDataTables = EntityDataTables {{ serializers: &{serializers:?}, fields: &[",
            folder.to_uppercase()
        );
        for entity in &entities {
            let ids: Vec<u8> = state[entity.as_str()]
                .iter()
                .map(|id| id.unwrap_or(u8::MAX))
                .collect();
            let _ = write!(out, "({entity:?}, &{ids:?}),");
        }
        let _ = writeln!(out, "] }};");
        newer = older;
        newer_mojang = mojang;
    }
    out
}

fn json_to_nbt_tag(v: &Value) -> NbtTag {
    match v {
        Value::Null => NbtTag::End,
        Value::Bool(b) => NbtTag::Byte(i8::from(*b)),
        Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                i32::try_from(i).map_or(NbtTag::Long(i), NbtTag::Int)
            } else if let Some(f) = num.as_f64() {
                NbtTag::Double(f)
            } else {
                NbtTag::Int(0)
            }
        }
        Value::String(s) => NbtTag::String(s.clone().into()),
        Value::Array(arr) => NbtTag::List(arr.iter().map(json_to_nbt_tag).collect()),
        Value::Object(obj) => {
            let mut compound = NbtCompound::new();
            for (k, val) in obj {
                compound.put(k, json_to_nbt_tag(val));
            }
            NbtTag::Compound(compound)
        }
    }
}

/// `(registry, [(entry, element)])` in core's order: sorted file names, `raw` chat type last.
fn load_version(folder: &str) -> Vec<(&'static str, Vec<(String, NbtCompound)>)> {
    let base = Path::new("assets/datapacks")
        .join(folder)
        .join("data/minecraft");
    let mut registries = Vec::new();
    for &reg_name in SYNCED_REGISTRIES {
        let Ok(dir) = fs::read_dir(base.join(reg_name)) else {
            continue;
        };
        let mut paths: Vec<_> = dir
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
            .collect();
        paths.sort();

        let mut entries = Vec::new();
        for path in paths {
            let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
            let content = fs::read_to_string(&path).unwrap();
            let value: Value = serde_json::from_str(&content).unwrap();
            if let NbtTag::Compound(compound) = json_to_nbt_tag(&value) {
                entries.push((stem, compound));
            }
        }
        // Pumpkin sends raw chat messages with this type
        if reg_name == "chat_type" {
            let raw = serde_json::json!({
                "chat": { "translation_key": "%s", "parameters": ["content"] },
                "narration": { "translation_key": "%s says %s", "parameters": ["sender", "content"] }
            });
            if let NbtTag::Compound(compound) = json_to_nbt_tag(&raw) {
                entries.push(("raw".to_string(), compound));
            }
        }
        if !entries.is_empty() {
            registries.push((reg_name, entries));
        }
    }
    registries
}

/// `{ "minecraft:<registry>": { type, value: [{ name, id, element }] } }`, from 1.16.2.
fn registry_codec(folder: &str) -> NbtCompound {
    let mut codec = NbtCompound::new();
    for (reg_name, entries) in load_version(folder) {
        let key = format!("minecraft:{reg_name}");
        let value = entries
            .into_iter()
            .enumerate()
            .map(|(id, (entry, element))| {
                let mut e = NbtCompound::new();
                e.put("name", NbtTag::String(format!("minecraft:{entry}").into()));
                e.put("id", NbtTag::Int(id as i32));
                e.put("element", NbtTag::Compound(element));
                NbtTag::Compound(e)
            })
            .collect();
        let mut registry = NbtCompound::new();
        registry.put("type", NbtTag::String(key.clone().into()));
        registry.put("value", NbtTag::List(value));
        codec.put(&key, NbtTag::Compound(registry));
    }
    codec
}

/// `{ dimension: [{ name, ..element }] }`, before 1.16.2.
fn dimension_codec(folder: &str) -> NbtCompound {
    let dimensions = load_version(folder)
        .into_iter()
        .filter(|(reg_name, _)| *reg_name == "dimension_type")
        .flat_map(|(_, entries)| entries)
        .map(|(entry, mut element)| {
            element.put("name", NbtTag::String(format!("minecraft:{entry}").into()));
            NbtTag::Compound(element)
        })
        .collect();
    let mut codec = NbtCompound::new();
    codec.put("dimension", NbtTag::List(dimensions));
    codec
}

/// Login codecs and dimension types, as named NBT like the packets before 1.20.2 send it.
fn login_tables() -> String {
    let mut out = String::new();
    for folder in LOGIN_CODEC_VERSIONS {
        let codec = if *folder == "1_16" {
            dimension_codec(folder)
        } else {
            registry_codec(folder)
        };
        let _ = writeln!(
            out,
            "pub static LOGIN_CODEC_{}: &[u8] = {};",
            folder.to_uppercase(),
            byte_literal(&Nbt::from(codec).write())
        );
    }
    for folder in DIMENSION_TYPE_VERSIONS {
        let _ = write!(
            out,
            "pub static DIMENSION_TYPES_{}: &[(&str, &[u8])] = &[",
            folder.to_uppercase()
        );
        for (reg_name, entries) in load_version(folder) {
            if reg_name != "dimension_type" {
                continue;
            }
            for (entry, element) in entries {
                let bytes = Nbt::from(element).write();
                let _ = write!(out, "({entry:?}, {}),", byte_literal(&bytes));
            }
        }
        let _ = writeln!(out, "];");
    }
    out
}

fn byte_literal(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 + 3);
    out.push_str("b\"");
    for &b in bytes {
        let _ = write!(out, "\\x{b:02x}");
    }
    out.push('"');
    out
}

/// ViaBackwards mapping files from 26.3 down to 1.13, the first version with block tags.
/// Each step maps the newer version's ids to the named older version's.
const BLOCK_ID_CHAIN: &[(&str, &str)] = &[
    ("V_26_2", "26.3to26.2"),
    ("V_26_1", "26.2to26.1"),
    ("V_1_21_11", "26.1to1.21.11"),
    ("V_1_21_9", "1.21.11to1.21.9"),
    ("V_1_21_7", "1.21.9to1.21.7"),
    ("V_1_21_6", "1.21.7to1.21.6"),
    ("V_1_21_5", "1.21.6to1.21.5"),
    ("V_1_21_4", "1.21.5to1.21.4"),
    ("V_1_21_2", "1.21.4to1.21.2"),
    ("V_1_21", "1.21.2to1.21"),
    ("V_1_20_5", "1.21to1.20.5"),
    ("V_1_20_3", "1.20.5to1.20.3"),
    ("V_1_20_2", "1.20.3to1.20.2"),
    ("V_1_20", "1.20.2to1.20"),
    ("V_1_19_4", "1.20to1.19.4"),
    ("V_1_19_3", "1.19.4to1.19.3"),
    ("V_1_19", "1.19.3to1.19"),
    ("V_1_18", "1.19to1.18"),
    ("V_1_17", "1.18to1.17"),
    ("V_1_16_2", "1.17to1.16.2"),
    ("V_1_16", "1.16.2to1.16"),
    ("V_1_15", "1.16to1.15"),
    ("V_1_14", "1.15to1.14"),
    ("V_1_13_2", "1.14to1.13.2"),
    ("V_1_13", "1.13.2to1.13"),
];

fn read_var_int(bytes: &mut &[u8]) -> Option<i32> {
    let mut result = 0i32;
    for shift in 0..5 {
        let (&b, rest) = bytes.split_first()?;
        *bytes = rest;
        result |= i32::from(b & 0x7F) << (7 * shift);
        if b & 0x80 == 0 {
            return Some(result);
        }
    }
    None
}

fn read_zigzag(bytes: &mut &[u8]) -> Option<i32> {
    let value = read_var_int(bytes)? as u32;
    Some((value >> 1) as i32 ^ -((value & 1) as i32))
}

fn as_bytes(values: &[i8]) -> Vec<u8> {
    values.iter().map(|&b| b as u8).collect()
}

/// Delta coded `(at, value)` pairs of the compact shift and change strategies.
fn at_value_pairs(values: &[i8]) -> Vec<(i32, i32)> {
    let bytes = as_bytes(values);
    let mut read = bytes.as_slice();
    let (mut at, mut value) = (-1, 0);
    let mut pairs = Vec::new();
    while let Some(diff_at) = read_var_int(&mut read) {
        at += 1 + diff_at;
        value += read_zigzag(&mut read).unwrap_or(0);
        pairs.push((at, value));
    }
    pairs
}

/// Via's `blocks` section as newer id -> older id (`-1` unmapped). `None` means unchanged.
fn via_blocks(file: &str) -> Option<Vec<i32>> {
    use pumpkin_nbt::deserializer::NbtReadHelperJava;

    let path = format!("assets/viabackwards/data/mappings-{file}.nbt");
    println!("cargo:rerun-if-changed={path}");
    let bytes = fs::read(&path).unwrap();
    let nbt = Nbt::read(&mut NbtReadHelperJava::new(std::io::Cursor::new(bytes))).unwrap();
    let section = nbt.root_tag.get_compound("blocks")?;
    let size = |fallback: i32| section.get_int("size").unwrap_or(fallback) as usize;
    Some(match section.get_byte("id").unwrap() {
        // Direct
        0 => {
            if let Some(values) = section.get_int_array("val") {
                values.to_vec()
            } else {
                let bytes = as_bytes(section.get_byte_array("val").unwrap());
                let mut read = bytes.as_slice();
                let mut prev = 0;
                (0..size(section.get_int("mappedSize").unwrap()))
                    .map(|_| {
                        prev += read_zigzag(&mut read).unwrap_or(0);
                        prev
                    })
                    .collect()
            }
        }
        // Shifts
        1 => {
            let pairs = match (section.get_int_array("at"), section.get_int_array("to")) {
                (Some(at), Some(to)) => at.iter().copied().zip(to.iter().copied()).collect(),
                _ => at_value_pairs(section.get_byte_array("val").unwrap()),
            };
            let size = size(0);
            let mut out: Vec<i32> = (0..size as i32).collect();
            for (i, &(from, to)) in pairs.iter().enumerate() {
                let end = pairs.get(i + 1).map_or(size as i32, |next| next.0);
                for (offset, id) in (from..end).enumerate() {
                    out[id as usize] = to + offset as i32;
                }
            }
            out
        }
        // Changes
        2 => {
            let pairs = match (section.get_int_array("at"), section.get_int_array("val")) {
                (Some(at), Some(val)) => at.iter().copied().zip(val.iter().copied()).collect(),
                _ => at_value_pairs(section.get_byte_array("val").unwrap()),
            };
            let fill = section.get("nofill").is_none();
            let size = size(0);
            let mut out: Vec<i32> = if fill {
                (0..size as i32).collect()
            } else {
                vec![-1; size]
            };
            for (at, value) in pairs {
                out[at as usize] = value;
            }
            out
        }
        // Identity
        3 => return None,
        strategy => panic!("unknown blocks strategy {strategy} in {path}"),
    })
}

/// 26.3 block id -> the version's block id per chain step. `None` when unchanged.
fn block_id_tables() -> String {
    let mut out = String::from("/* Generated by build.rs from the ViaBackwards mappings. */\n");
    let mut composed: Option<Vec<i32>> = None;
    for (version, file) in BLOCK_ID_CHAIN {
        if let Some(step) = via_blocks(file) {
            composed = Some(match composed {
                None => step,
                Some(prev) => prev
                    .iter()
                    .map(|&id| {
                        usize::try_from(id)
                            .ok()
                            .and_then(|id| step.get(id).copied())
                            .unwrap_or(-1)
                    })
                    .collect(),
            });
        }
        match &composed {
            Some(table) => {
                let _ = writeln!(
                    out,
                    "static BLOCK_IDS_{version}: Option<&[i32]> = Some(&{table:?});"
                );
            }
            None => {
                let _ = writeln!(out, "static BLOCK_IDS_{version}: Option<&[i32]> = None;");
            }
        }
    }
    let _ = write!(
        out,
        "/// Block id tables by the version they map to, oldest first.\n\
         static BLOCK_IDS: &[(pumpkin_util::version::JavaMinecraftVersion, Option<&[i32]>)] = &["
    );
    for (version, _) in BLOCK_ID_CHAIN.iter().rev() {
        let _ = write!(
            out,
            "(pumpkin_util::version::JavaMinecraftVersion::{version}, BLOCK_IDS_{version}),"
        );
    }
    let _ = writeln!(out, "];");
    out
}

/// Datapack folders of the configuration state versions, whose tags the translator completes.
const TAG_VERSIONS: &[&str] = &[
    "1_20_2", "1_21", "1_21_2", "1_21_4", "1_21_5", "1_21_6", "1_21_7", "1_21_9", "1_21_11",
    "26_1", "26_2",
];

/// `(registry, tag)` to its raw values
fn load_tags(folder: &str) -> HashMap<(String, String), Vec<String>> {
    fn walk(dir: &Path, rel: &str, out: &mut HashMap<(String, String), Vec<String>>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if path.is_dir() {
                walk(&path, &rel, out);
            } else if let Some(rel) = rel.strip_suffix(".json") {
                // worldgen registries take two segments, e.g. `worldgen/biome`
                let segments = if rel.starts_with("worldgen/") { 2 } else { 1 };
                let mut parts = rel.splitn(segments + 1, '/');
                let registry: Vec<_> = parts.by_ref().take(segments).collect();
                let Some(tag) = parts.next() else { continue };
                let json: Value =
                    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
                let values = json["values"]
                    .as_array()
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(|v| v.as_str().or_else(|| v["id"].as_str()))
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                out.insert((registry.join("/"), tag.to_string()), values);
            }
        }
    }
    let mut out = HashMap::new();
    walk(
        &Path::new("assets/datapacks")
            .join(folder)
            .join("data/minecraft/tags"),
        "",
        &mut out,
    );
    out
}

/// Entry names of a tag with nested `#tag` references resolved.
fn resolve_tag(
    tags: &HashMap<(String, String), Vec<String>>,
    registry: &str,
    tag: &str,
    seen: &mut Vec<String>,
    out: &mut Vec<String>,
) {
    if seen.iter().any(|t| t == tag) {
        return;
    }
    seen.push(tag.to_string());
    for value in tags
        .get(&(registry.to_string(), tag.to_string()))
        .into_iter()
        .flatten()
    {
        if let Some(nested) = value.strip_prefix('#') {
            let nested = nested.strip_prefix("minecraft:").unwrap_or(nested);
            resolve_tag(tags, registry, nested, seen, out);
        } else {
            let name = value
                .strip_prefix("minecraft:")
                .unwrap_or(value)
                .to_string();
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
}

/// Per version, the tags 26.3 no longer has, with resolved entry names.
fn missing_tag_tables() -> String {
    let current = load_tags("26_3");
    let mut out = String::new();
    for folder in TAG_VERSIONS {
        let tags = load_tags(folder);
        let mut missing: Vec<_> = tags
            .keys()
            .filter(|key| !current.contains_key(*key))
            .collect();
        missing.sort();
        let _ = write!(
            out,
            "pub static MISSING_TAGS_{}: &[(&str, &str, &[&str])] = &[",
            folder.to_uppercase()
        );
        for (registry, tag) in missing {
            let mut entries = Vec::new();
            resolve_tag(&tags, registry, tag, &mut Vec::new(), &mut entries);
            let _ = write!(out, "({registry:?}, \"minecraft:{tag}\", &{entries:?}),");
        }
        let _ = writeln!(out, "];");
    }
    out
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/datapacks");
    println!("cargo:rerun-if-changed=assets/tracked_data");
    println!("cargo:rerun-if-changed=assets/meta_data_type");

    let mut blobs: Vec<Vec<u8>> = Vec::new();
    let mut blob_ids: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut tables = String::new();

    for folder in VERSIONS {
        let _ = writeln!(
            tables,
            "pub static REGISTRIES_{}: &[SyncedRegistry] = &[",
            folder.to_uppercase()
        );
        for (reg_name, entries) in load_version(folder) {
            let _ = write!(tables, "SyncedRegistry {{ id: {reg_name:?}, entries: &[");
            for (entry, compound) in entries {
                let bytes = Nbt::from(compound).write_unnamed().to_vec();
                let next = blobs.len();
                let id = *blob_ids.entry(bytes.clone()).or_insert(next);
                if id == next {
                    blobs.push(bytes);
                }
                let _ = write!(tables, "({entry:?}, B{id}),");
            }
            let _ = writeln!(tables, "] }},");
        }
        let _ = writeln!(tables, "];");
    }

    for folder in NAME_VERSIONS {
        let _ = write!(
            tables,
            "pub static NAMES_{}: &[(&str, &[&str])] = &[",
            folder.to_uppercase()
        );
        for (reg_name, entries) in load_version(folder) {
            let names: Vec<_> = entries.iter().map(|(name, _)| name.as_str()).collect();
            let _ = write!(tables, "({reg_name:?}, &{names:?}),");
        }
        let _ = writeln!(tables, "];");
    }

    let codec = Nbt::from(registry_codec(CODEC_VERSION)).write_unnamed();

    let mut out = String::from("/* Generated by build.rs from assets/datapacks. */\n");
    for (id, blob) in blobs.iter().enumerate() {
        let _ = writeln!(out, "static B{id}: &[u8] = {};", byte_literal(blob));
    }
    out.push_str(&tables);
    out.push_str(&missing_tag_tables());
    out.push_str(&login_tables());
    let _ = writeln!(
        out,
        "pub static CODEC_{}: &[u8] = {};",
        CODEC_VERSION.to_uppercase(),
        byte_literal(&codec)
    );

    let out_dir = std::env::var("OUT_DIR").unwrap();
    fs::write(Path::new(&out_dir).join("registry.rs"), out).unwrap();
    fs::write(
        Path::new(&out_dir).join("entity_data.rs"),
        entity_data_tables(),
    )
    .unwrap();
    fs::write(Path::new(&out_dir).join("block_id.rs"), block_id_tables()).unwrap();
}
