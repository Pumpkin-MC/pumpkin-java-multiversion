//! Older versions' synced registries and tags, written into `assets/datapacks` at build time
//! from Mojang's server jars: their files, or before 1.19.4 their data generator's reports.

use std::ffi::OsString;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// Directory of `<version>.jar` server jars to use instead of downloading them.
const JAR_DIR_ENV: &str = "PUMPKIN_MINECRAFT_JAR_DIR";

/// Java 17+ for the data generator, else `JAVA_HOME`, else `java` on `PATH`.
const JAVA_ENV: &str = "PUMPKIN_JAVA";

/// Datapack folder and the server jar it comes from
const JAR_SOURCES: &[(&str, &str)] = &[
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
    ("26_3", "26.3"),
];

/// Where extracted folders go
pub const CACHE_DIR: &str = "assets/datapacks";

/// Datapack folder and the server jar whose data generator reports its registries. Built into
/// the game before 1.19.4, so no jar has them as files.
const GENERATOR_SOURCES: &[(&str, &str)] = &[
    ("1_16_2", "1.16.2"),
    ("1_17", "1.17.1"),
    ("1_18", "1.18.2"),
    ("1_19", "1.19"),
];

type DimensionTypeStep = (&'static str, &'static str, fn(&mut Map<String, Value>));

/// Dimension types no report has before 1.18: folder, folder it comes from, conversion.
const DIMENSION_TYPE_STEPS: &[DimensionTypeStep] = &[
    ("1_17", "1_18", dimension_type_1_17),
    ("1_16_2", "1_17", dimension_type_1_16_2),
    ("1_16", "1_16_2", dimension_type_1_16_1),
];

/// Stamp of the generated folders. Rebuilt together: dimension types derive across folders.
const GENERATED_STAMP: &str = "reports 1.16.2 1.17.1 1.18.2 1.19";

/// Biome fields only the server reads (`Biome.NETWORK_CODEC` leaves them out). Last three: before
/// 1.18.
const BIOME_SERVER_FIELDS: &[&str] = &[
    "carvers",
    "features",
    "spawners",
    "spawn_costs",
    "creature_spawn_probability",
    "player_spawn_friendly",
    "starts",
    "surface_builder",
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

/// Extracts every folder whose `.version` stamp doesn't match its source.
pub fn ensure_datapacks(synced_registries: &[&str]) -> Result<(), BoxError> {
    let mut client = None;
    for &(folder, version) in JAR_SOURCES.iter().filter(|(f, v)| is_stale(f, v)) {
        let jar = server_jar(&mut client, version)?;
        let dir = fresh_dir(folder)?;
        extract_jar(&jar, version, &dir, synced_registries)?;
        fs::write(stamp_path(folder), version)?;
    }

    let generated: Vec<_> = GENERATOR_SOURCES
        .iter()
        .map(|(folder, _)| *folder)
        .chain(DIMENSION_TYPE_STEPS.iter().map(|(folder, ..)| *folder))
        .collect();
    if !generated
        .iter()
        .any(|folder| is_stale(folder, GENERATED_STAMP))
    {
        return Ok(());
    }
    for folder in &generated {
        fresh_dir(folder)?;
    }
    let java = java();
    for &(folder, version) in GENERATOR_SOURCES {
        let jar = server_jar(&mut client, version)?;
        let dir = Path::new(CACHE_DIR).join(folder);
        run_generator(&java, &jar, version, &dir, synced_registries)?;
    }
    for &(folder, from, convert) in DIMENSION_TYPE_STEPS {
        derive_dimension_types(from, folder, convert)?;
    }
    for folder in &generated {
        fs::write(stamp_path(folder), GENERATED_STAMP)?;
    }
    Ok(())
}

fn is_stale(folder: &str, version: &str) -> bool {
    fs::read_to_string(stamp_path(folder)).ok().as_deref() != Some(version)
}

fn stamp_path(folder: &str) -> PathBuf {
    Path::new(CACHE_DIR).join(folder).join(".version")
}

/// Empty folder for `folder`, old contents removed.
fn fresh_dir(folder: &str) -> Result<PathBuf, BoxError> {
    let dir = Path::new(CACHE_DIR).join(folder);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(dir)
}

fn server_jar(client: &mut Option<Downloader>, version: &str) -> Result<Vec<u8>, BoxError> {
    if let Some(dir) = std::env::var_os(JAR_DIR_ENV)
        && let Ok(jar) = fs::read(Path::new(&dir).join(format!("{version}.jar")))
    {
        return Ok(jar);
    }
    let client = match client {
        Some(client) => client,
        None => client.insert(Downloader::new()?),
    };
    client.server_jar(version)
}

fn java() -> OsString {
    if let Some(java) = std::env::var_os(JAVA_ENV) {
        return java;
    }
    std::env::var_os("JAVA_HOME").map_or_else(
        || "java".into(),
        |home| Path::new(&home).join("bin").join("java").into_os_string(),
    )
}

struct Downloader {
    client: reqwest::blocking::Client,
    manifest: Option<Value>,
}

impl Downloader {
    fn new() -> Result<Self, BoxError> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::blocking::Client::builder()
            .user_agent("pumpkin-java-multiversion-build")
            .build()?;
        Ok(Self {
            client,
            manifest: None,
        })
    }

    fn get(&self, url: &str) -> Result<reqwest::blocking::Response, BoxError> {
        Ok(self.client.get(url).send()?.error_for_status()?)
    }

    fn server_jar(&mut self, version: &str) -> Result<Vec<u8>, BoxError> {
        let manifest = match &self.manifest {
            Some(manifest) => manifest,
            None => {
                let manifest = self.get(MANIFEST_URL)?.json()?;
                self.manifest.insert(manifest)
            }
        };
        let package_url = manifest["versions"]
            .as_array()
            .and_then(|versions| versions.iter().find(|v| v["id"] == version))
            .and_then(|v| v["url"].as_str())
            .ok_or_else(|| format!("Minecraft {version} is not in the version manifest"))?
            .to_owned();
        let package: Value = self.get(&package_url)?.json()?;
        let server_url = package["downloads"]["server"]["url"]
            .as_str()
            .ok_or_else(|| format!("Minecraft {version} has no server download"))?;
        println!("cargo:warning=Downloading the Minecraft {version} server jar...");
        Ok(self.get(server_url)?.bytes()?.to_vec())
    }
}

/// Since 1.18 the server jar is a bundler around the real one.
fn bundled_jar_name(version: &str) -> String {
    format!("META-INF/versions/{version}/server-{version}.jar")
}

/// Writes the jar's tags and synced registries, the registries as vanilla sends them.
fn extract_jar(jar: &[u8], version: &str, dir: &Path, synced: &[&str]) -> Result<(), BoxError> {
    let mut outer = zip::ZipArchive::new(Cursor::new(jar))?;
    let mut archive = match outer.by_name(&bundled_jar_name(version)) {
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
        } else if let Some(registry) = synced_registry(rel, synced) {
            (rel.to_string(), Some(registry))
        } else {
            continue;
        };

        let mut content = String::new();
        file.read_to_string(&mut content)?;
        write_entry(dir, &out_rel, registry, serde_json::from_str(&content)?)?;
    }
    Ok(())
}

/// Runs the jar's data generator and writes the synced registries it reports.
fn run_generator(
    java: &OsString,
    jar: &[u8],
    version: &str,
    dir: &Path,
    synced: &[&str],
) -> Result<(), BoxError> {
    let work = Path::new(CACHE_DIR).join(".generator");
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    fs::create_dir_all(&work)?;
    fs::write(work.join("server.jar"), jar)?;
    let bundled = zip::ZipArchive::new(Cursor::new(jar))?
        .by_name(&bundled_jar_name(version))
        .is_ok();

    println!("cargo:warning=Running the Minecraft {version} data generator...");
    let mut command = Command::new(java);
    command.current_dir(&work);
    if bundled {
        command.args([
            "-DbundlerMainClass=net.minecraft.data.Main",
            "-jar",
            "server.jar",
        ]);
    } else {
        command.args(["-cp", "server.jar", "net.minecraft.data.Main"]);
    }
    let output = command
        .args(["--reports", "--output", "generated"])
        .output()
        .map_err(|e| format!("Failed to run {}: {e}", java.to_string_lossy()))?;
    if !output.status.success() {
        // Exception head on stderr, else the log's end on stdout.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let log: Vec<_> = if stderr.trim().is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<_> = stdout.lines().map(str::to_owned).collect();
            lines[lines.len().saturating_sub(10)..].to_vec()
        } else {
            stderr.lines().take(5).map(str::to_owned).collect()
        };
        return Err(format!(
            "Minecraft {version} data generator failed (needs Java 17+, set {JAVA_ENV}):\n{}",
            log.join("\n")
        )
        .into());
    }

    let reports = work.join("generated/reports");
    let mut files = Vec::new();
    list_files(&reports, &mut files)?;
    for path in files {
        let rel = path
            .strip_prefix(&reports)?
            .to_string_lossy()
            .replace('\\', "/");
        let Some(rel) = report_registry_path(&rel) else {
            continue;
        };
        let Some(rel) = rel.strip_suffix(".json") else {
            continue;
        };
        let Some(registry) = synced_registry(rel, synced) else {
            continue;
        };
        let value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        write_entry(dir, rel, Some(registry), value)?;
    }
    fs::remove_dir_all(&work)?;
    Ok(())
}

/// Registry path of a report: `biomes/` before 1.18, `worldgen/minecraft/` in 1.18,
/// `minecraft/` since 1.19.
fn report_registry_path(rel: &str) -> Option<String> {
    if let Some(entry) = rel.strip_prefix("biomes/") {
        return Some(format!("worldgen/biome/{entry}"));
    }
    rel.strip_prefix("worldgen/minecraft/")
        .or_else(|| rel.strip_prefix("minecraft/"))
        .map(str::to_owned)
}

fn list_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), BoxError> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            list_files(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// Synced registry `rel` is a direct entry of.
fn synced_registry<'a>(rel: &str, synced: &[&'a str]) -> Option<&'a str> {
    synced.iter().copied().find(|registry| {
        rel.strip_prefix(registry)
            .and_then(|e| e.strip_prefix('/'))
            .is_some_and(|entry| !entry.contains('/'))
    })
}

/// Writes `value` to `<dir>/data/minecraft/<rel>.json`, a registry entry as vanilla sends it.
fn write_entry(
    dir: &Path,
    rel: &str,
    registry: Option<&str>,
    mut value: Value,
) -> Result<(), BoxError> {
    if let Some(registry) = registry {
        to_network(registry, &mut value);
    }
    write_json(
        &dir.join("data/minecraft").join(format!("{rel}.json")),
        &value,
    )
}

fn write_json(path: &Path, value: &Value) -> Result<(), BoxError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

/// Writes `folder`'s dimension types: `from`'s, converted.
fn derive_dimension_types(
    from: &str,
    folder: &str,
    convert: fn(&mut Map<String, Value>),
) -> Result<(), BoxError> {
    let types = Path::new("data/minecraft/dimension_type");
    let dir = Path::new(CACHE_DIR).join(folder).join(types);
    for entry in fs::read_dir(Path::new(CACHE_DIR).join(from).join(types))? {
        let path = entry?.path();
        let mut value: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        if let Value::Object(map) = &mut value {
            convert(map);
        }
        let name = path.file_name().ok_or("dimension type without file name")?;
        write_json(&dir.join(name), &value)?;
    }
    Ok(())
}

/// 1.17: `infiniburn` a tag id without `#`, world from 0 to 256.
fn dimension_type_1_17(map: &mut Map<String, Value>) {
    if let Some(Value::String(infiniburn)) = map.get_mut("infiniburn")
        && let Some(id) = infiniburn.strip_prefix('#')
    {
        *infiniburn = id.to_owned();
    }
    map.insert("min_y".to_owned(), 0.into());
    map.insert("height".to_owned(), 256.into());
    if map
        .get("logical_height")
        .and_then(Value::as_i64)
        .is_some_and(|height| height > 256)
    {
        map.insert("logical_height".to_owned(), 256.into());
    }
}

/// 1.16.2: no `min_y` and `height`.
fn dimension_type_1_16_2(map: &mut Map<String, Value>) {
    map.remove("min_y");
    map.remove("height");
}

/// 1.16.1: no `effects`, `shrunk` in place of `coordinate_scale`.
fn dimension_type_1_16_1(map: &mut Map<String, Value>) {
    map.remove("effects");
    let scale = map
        .remove("coordinate_scale")
        .and_then(|scale| scale.as_f64())
        .unwrap_or(1.0);
    map.insert("shrunk".to_owned(), Value::Bool(scale > 1.0));
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
