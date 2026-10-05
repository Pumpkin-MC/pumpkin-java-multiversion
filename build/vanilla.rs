//! Older versions' synced registries and tags, extracted from Mojang's server jars into
//! `assets/datapacks`.

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use serde_json::Value;

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// Directory of `<version>.jar` server jars to use instead of downloading them.
const JAR_DIR_ENV: &str = "PUMPKIN_MINECRAFT_JAR_DIR";

/// Datapack folder and the server jar it comes from
pub const JAR_SOURCES: &[(&str, &str)] = &[
    ("1_20", "1.19.4"),
    ("1_20_2", "1.20.4"),
    ("1_21", "1.21.1"),
    ("1_21_2", "1.21.3"),
    ("1_21_4", "1.21.4"),
    ("1_21_5", "1.21.5"),
    ("1_21_6", "1.21.6"),
    ("1_21_7", "1.21.8"),
    ("1_21_9", "1.21.10"),
    ("1_21_11", "1.21.11"),
    ("26_1", "26.1.2"),
    ("26_2", "26.2"),
];

/// Where extracted folders go
pub const CACHE_DIR: &str = "assets/datapacks";

/// Registries that were built into the game before 1.19.4, so no jar has them.
pub const BUILTIN_DIR: &str = "assets/builtin_registries";

/// Biome fields only the server reads (`Biome.NETWORK_CODEC` leaves them out).
const BIOME_SERVER_FIELDS: &[&str] = &[
    "carvers",
    "features",
    "spawners",
    "spawn_costs",
    "creature_spawn_probability",
];

/// `EnvironmentAttributes` registered without `.syncable()`, under `minecraft:gameplay/`.
const NON_SYNCABLE_ATTRIBUTES: &[&str] = &[
    "can_start_raid",
    "bed_rule",
    "straw_bed_rule",
    "respawn_anchor_works",
    "nether_portal_spawns_piglin",
    "increased_fire_burnout",
    "eyeblossom_open",
    "turtle_egg_hatch_chance",
    "snow_golem_melts",
    "surface_slime_spawn_chance",
    "cat_waking_up_gift_chance",
    "bees_stay_in_hive",
    "monsters_burn",
    "can_pillager_patrol_spawn",
    "natural_mob_spawns",
    "creature_world_gen_spawn_probability",
    "villager_activity",
    "baby_villager_activity",
];

/// Tag folders before 1.21 were plural.
const PLURAL_TAG_DIRS: &[(&str, &str)] = &[
    ("blocks", "block"),
    ("items", "item"),
    ("fluids", "fluid"),
    ("entity_types", "entity_type"),
    ("game_events", "game_event"),
];

type BoxError = Box<dyn std::error::Error>;

/// Extracts every [`JAR_SOURCES`] folder whose `.version` stamp doesn't match its jar.
pub fn ensure_datapacks(synced_registries: &[&str]) -> Result<(), BoxError> {
    let stale: Vec<_> = JAR_SOURCES
        .iter()
        .filter(|(folder, version)| {
            fs::read_to_string(stamp_path(folder)).ok().as_deref() != Some(version)
        })
        .collect();
    if stale.is_empty() {
        return Ok(());
    }

    let mut client = None;
    for (folder, version) in stale {
        let jar = match local_jar(version) {
            Some(jar) => jar,
            None => {
                let client = match &mut client {
                    Some(client) => client,
                    None => client.insert(Downloader::new()?),
                };
                client.server_jar(version)?
            }
        };
        let dir = Path::new(CACHE_DIR).join(folder);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        extract(&jar, version, &dir, synced_registries)?;
        fs::write(stamp_path(folder), version)?;
    }
    Ok(())
}

fn stamp_path(folder: &str) -> PathBuf {
    Path::new(CACHE_DIR).join(folder).join(".version")
}

fn local_jar(version: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os(JAR_DIR_ENV)?;
    fs::read(Path::new(&dir).join(format!("{version}.jar"))).ok()
}

struct Downloader {
    client: reqwest::blocking::Client,
    manifest: Value,
}

impl Downloader {
    fn new() -> Result<Self, BoxError> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::blocking::Client::builder()
            .user_agent("pumpkin-java-multiversion-build")
            .build()?;
        let manifest = client
            .get(MANIFEST_URL)
            .send()?
            .error_for_status()?
            .json()?;
        Ok(Self { client, manifest })
    }

    fn server_jar(&self, version: &str) -> Result<Vec<u8>, BoxError> {
        let package_url = self.manifest["versions"]
            .as_array()
            .and_then(|versions| versions.iter().find(|v| v["id"] == version))
            .and_then(|v| v["url"].as_str())
            .ok_or_else(|| format!("Minecraft {version} is not in the version manifest"))?;
        let package: Value = self
            .client
            .get(package_url)
            .send()?
            .error_for_status()?
            .json()?;
        let server_url = package["downloads"]["server"]["url"]
            .as_str()
            .ok_or_else(|| format!("Minecraft {version} has no server download"))?;
        println!("cargo:warning=Downloading the Minecraft {version} server jar...");
        Ok(self
            .client
            .get(server_url)
            .send()?
            .error_for_status()?
            .bytes()?
            .to_vec())
    }
}

/// Writes the jar's tags and synced registries, the registries as vanilla sends them.
fn extract(jar: &[u8], version: &str, dir: &Path, synced: &[&str]) -> Result<(), BoxError> {
    let mut outer = zip::ZipArchive::new(Cursor::new(jar))?;
    // Since 1.18 the server jar is a bundler around the real one
    let inner_name = format!("META-INF/versions/{version}/server-{version}.jar");
    let mut archive = match outer.by_name(&inner_name) {
        Ok(mut inner) => {
            let mut bytes = Vec::with_capacity(inner.size() as usize);
            inner.read_to_end(&mut bytes)?;
            zip::ZipArchive::new(Cursor::new(bytes))?
        }
        Err(_) => zip::ZipArchive::new(Cursor::new(jar.to_vec()))?,
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let Some(rel) = file.name().strip_prefix("data/minecraft/") else {
            continue;
        };
        let Some(rel) = rel.strip_suffix(".json") else {
            continue;
        };
        let (out_rel, registry) = if let Some(tag) = rel.strip_prefix("tags/") {
            (format!("tags/{}", singular_tag_path(tag)), None)
        } else if let Some(registry) = synced.iter().find(|registry| {
            rel.strip_prefix(**registry)
                .and_then(|e| e.strip_prefix('/'))
                .is_some_and(|entry| !entry.contains('/'))
        }) {
            (rel.to_string(), Some(*registry))
        } else {
            continue;
        };

        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let mut value: Value = serde_json::from_str(&content)?;
        if let Some(registry) = registry {
            to_network(registry, &mut value);
        }
        let path = dir.join("data/minecraft").join(format!("{out_rel}.json"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(&value)?)?;
    }
    Ok(())
}

fn singular_tag_path(tag: &str) -> String {
    let (dir, rest) = tag.split_once('/').unwrap_or((tag, ""));
    let dir = PLURAL_TAG_DIRS
        .iter()
        .find(|(plural, _)| *plural == dir)
        .map_or(dir, |(_, singular)| singular);
    format!("{dir}/{rest}")
}

/// Drops what vanilla's network codecs leave out, so older clients get what vanilla sends.
fn to_network(registry: &str, value: &mut Value) {
    let Value::Object(entry) = value else {
        return;
    };
    if registry == "worldgen/biome" {
        for field in BIOME_SERVER_FIELDS {
            entry.remove(*field);
        }
    }
    if registry.ends_with("_variant") {
        entry.remove("spawn_conditions");
    }
    for key in ["attributes", "tracks"] {
        let Some(Value::Object(map)) = entry.get_mut(key) else {
            continue;
        };
        map.retain(|attribute, _| {
            !attribute
                .strip_prefix("minecraft:gameplay/")
                .is_some_and(|name| NON_SYNCABLE_ATTRIBUTES.contains(&name))
        });
        if map.is_empty() {
            entry.remove(key);
        }
    }
}
