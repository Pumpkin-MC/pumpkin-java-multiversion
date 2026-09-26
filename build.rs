//! Per-version synced registries from `assets/datapacks`, as network NBT.

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

fn byte_literal(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 + 3);
    out.push_str("b\"");
    for &b in bytes {
        let _ = write!(out, "\\x{b:02x}");
    }
    out.push('"');
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

    // { "minecraft:<registry>": { type, value: [{ name, id, element }] } }
    let mut codec = NbtCompound::new();
    for (reg_name, entries) in load_version(CODEC_VERSION) {
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
    let codec = Nbt::from(codec).write_unnamed();

    let mut out = String::from("/* Generated by build.rs from assets/datapacks. */\n");
    for (id, blob) in blobs.iter().enumerate() {
        let _ = writeln!(out, "static B{id}: &[u8] = {};", byte_literal(blob));
    }
    out.push_str(&tables);
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
}
