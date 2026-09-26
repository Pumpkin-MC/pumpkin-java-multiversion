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
}
