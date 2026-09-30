//! Synced registries of older clients. Core only knows 26.3's.

use pumpkin_protocol::{
    VarInt,
    ser::{NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

pub struct SyncedRegistry {
    /// Without the `minecraft:` namespace.
    pub id: &'static str,
    /// `(entry, network NBT)` in the client's registry order.
    pub entries: &'static [(&'static str, &'static [u8])],
}

include!(concat!(env!("OUT_DIR"), "/registry.rs"));

eras! {
    /// Which `assets/datapacks` folder a client's registries come from.
    pub enum DatapackVersion {
        /// No synced registries.
        V1_7 = V_1_7_2,
        V1_16 = V_1_16,
        V1_16_2 = V_1_16_2,
        V1_17 = V_1_17,
        V1_18 = V_1_18,
        V1_19 = V_1_19,
        V1_20 = V_1_19_4,
        /// Configuration state; one registry codec packet.
        V1_20_2 = V_1_20_2,
        /// One packet per registry.
        V1_21 = V_1_20_5,
        V1_21_2 = V_1_21_2,
        V1_21_4 = V_1_21_4,
        V1_21_5 = V_1_21_5,
        V1_21_6 = V_1_21_6,
        V1_21_7 = V_1_21_7,
        V1_21_9 = V_1_21_9,
        V1_21_11 = V_1_21_11,
        V26_1 = V_26_1,
        V26_2 = V_26_2,
        /// Core's own registries.
        V26_3 = V_26_3,
    }
}

enum Synced {
    /// One packet holding every registry.
    Codec(&'static [u8]),
    /// One packet per registry.
    Registries(&'static [SyncedRegistry]),
}

/// `None` for 26.3 and versions without the configuration state.
fn synced_for(version: JavaMinecraftVersion) -> Option<Synced> {
    use DatapackVersion as D;
    Some(match DatapackVersion::of(version) {
        D::V1_7 | D::V1_16 | D::V1_16_2 | D::V1_17 | D::V1_18 | D::V1_19 | D::V1_20 => {
            return None;
        }
        D::V1_20_2 => Synced::Codec(CODEC_1_20_2),
        D::V1_21 => Synced::Registries(REGISTRIES_1_21),
        D::V1_21_2 => Synced::Registries(REGISTRIES_1_21_2),
        D::V1_21_4 => Synced::Registries(REGISTRIES_1_21_4),
        D::V1_21_5 => Synced::Registries(REGISTRIES_1_21_5),
        D::V1_21_6 => Synced::Registries(REGISTRIES_1_21_6),
        D::V1_21_7 => Synced::Registries(REGISTRIES_1_21_7),
        D::V1_21_9 => Synced::Registries(REGISTRIES_1_21_9),
        D::V1_21_11 => Synced::Registries(REGISTRIES_1_21_11),
        D::V26_1 => Synced::Registries(REGISTRIES_26_1),
        D::V26_2 => Synced::Registries(REGISTRIES_26_2),
        D::V26_3 => return None,
    })
}

/// The login packet's registry codec, as named NBT. `None` outside 1.16 - 1.20.1.
#[must_use]
pub fn login_codec(version: JavaMinecraftVersion) -> Option<&'static [u8]> {
    use DatapackVersion as D;
    Some(match DatapackVersion::of(version) {
        D::V1_16 => LOGIN_CODEC_1_16,
        D::V1_16_2 => LOGIN_CODEC_1_16_2,
        D::V1_17 => LOGIN_CODEC_1_17,
        D::V1_18 => LOGIN_CODEC_1_18,
        D::V1_19 => LOGIN_CODEC_1_19,
        D::V1_20 => LOGIN_CODEC_1_20,
        _ => return None,
    })
}

/// The dimension type of `dimension` as named NBT, for the login and respawn packets of
/// 1.16.2 - 1.18.2. Dimensions the client lacks use the overworld's.
#[must_use]
pub fn dimension_type_nbt(version: JavaMinecraftVersion, dimension: &str) -> Option<&'static [u8]> {
    use DatapackVersion as D;
    let types = match DatapackVersion::of(version) {
        D::V1_16_2 => DIMENSION_TYPES_1_16_2,
        D::V1_17 => DIMENSION_TYPES_1_17,
        D::V1_18 => DIMENSION_TYPES_1_18,
        _ => return None,
    };
    let name = dimension.strip_prefix("minecraft:").unwrap_or(dimension);
    let find = |name: &str| types.iter().find(|(n, _)| *n == name).map(|(_, nbt)| *nbt);
    find(name).or_else(|| find("overworld"))
}

type Names = &'static [(&'static str, &'static [&'static str])];

/// Entry order of the client's synced registries. `None` for 26.3 and before 1.16.
fn names_for(version: JavaMinecraftVersion) -> Option<Names> {
    use DatapackVersion as D;
    Some(match DatapackVersion::of(version) {
        D::V1_7 | D::V26_3 => return None,
        D::V1_16 => NAMES_1_16,
        D::V1_16_2 => NAMES_1_16_2,
        D::V1_17 => NAMES_1_17,
        D::V1_18 => NAMES_1_18,
        D::V1_19 => NAMES_1_19,
        D::V1_20 => NAMES_1_20,
        D::V1_20_2 => NAMES_1_20_2,
        D::V1_21 => NAMES_1_21,
        D::V1_21_2 => NAMES_1_21_2,
        D::V1_21_4 => NAMES_1_21_4,
        D::V1_21_5 => NAMES_1_21_5,
        D::V1_21_6 => NAMES_1_21_6,
        D::V1_21_7 => NAMES_1_21_7,
        D::V1_21_9 => NAMES_1_21_9,
        D::V1_21_11 => NAMES_1_21_11,
        D::V26_1 => NAMES_26_1,
        D::V26_2 => NAMES_26_2,
    })
}

/// 26.3's entries of `registry`, in core's id order.
#[must_use]
pub fn current_names(registry: &str) -> Option<&'static [&'static str]> {
    NAMES_26_3
        .iter()
        .find(|(id, _)| *id == registry)
        .map(|(_, names)| *names)
}

/// 26.3 registry ids mapped to the client's by entry name.
pub struct IdRemap {
    /// Indexed by the 26.3 id.
    pub table: Vec<u32>,
    /// Entries in the client's registry.
    pub client_len: u32,
}

impl IdRemap {
    #[must_use]
    pub fn get(&self, id: u32) -> u32 {
        self.table.get(id as usize).copied().unwrap_or(0)
    }
}

/// `registry` without namespace, e.g. `worldgen/biome`. Entries the client lacks become
/// `fallback`. `None` when the client uses 26.3's order.
#[must_use]
pub fn id_remap(registry: &str, version: JavaMinecraftVersion, fallback: &str) -> Option<IdRemap> {
    let (_, client) = names_for(version)?.iter().find(|(id, _)| *id == registry)?;
    let current = current_names(registry)?;
    if current == *client {
        return None;
    }
    let position = |name: &str| client.iter().position(|n| *n == name);
    let fallback = position(fallback).unwrap_or(0) as u32;
    Some(IdRemap {
        table: current
            .iter()
            .map(|name| position(name).map_or(fallback, |i| i as u32))
            .collect(),
        client_len: client.len() as u32,
    })
}

/// Tags of the client's datapack that 26.3 no longer has, as `(registry, tag, entry names)`.
#[must_use]
pub fn missing_tags(
    version: JavaMinecraftVersion,
) -> &'static [(&'static str, &'static str, &'static [&'static str])] {
    use DatapackVersion as D;
    match DatapackVersion::of(version) {
        D::V1_20_2 => MISSING_TAGS_1_20_2,
        D::V1_21 => MISSING_TAGS_1_21,
        D::V1_21_2 => MISSING_TAGS_1_21_2,
        D::V1_21_4 => MISSING_TAGS_1_21_4,
        D::V1_21_5 => MISSING_TAGS_1_21_5,
        D::V1_21_6 => MISSING_TAGS_1_21_6,
        D::V1_21_7 => MISSING_TAGS_1_21_7,
        D::V1_21_9 => MISSING_TAGS_1_21_9,
        D::V1_21_11 => MISSING_TAGS_1_21_11,
        D::V26_1 => MISSING_TAGS_26_1,
        D::V26_2 => MISSING_TAGS_26_2,
        _ => &[],
    }
}

/// Like [`id_remap`], but entries the client lacks are `None` instead of a fallback.
#[must_use]
pub fn id_remap_or_drop(registry: &str, version: JavaMinecraftVersion) -> Option<Vec<Option<u32>>> {
    let (_, client) = names_for(version)?.iter().find(|(id, _)| *id == registry)?;
    let current = current_names(registry)?;
    if current == *client {
        return None;
    }
    Some(
        current
            .iter()
            .map(|name| client.iter().position(|n| n == name).map(|i| i as u32))
            .collect(),
    )
}

/// REGISTRY_DATA: swaps 26.3's entries for the client version's vanilla ones.
/// `None` when the client does not sync this registry and the packet must be dropped.
// TODO: datapack damage types merged by core are lost; merge them into the version's list.
// TODO: play packets still carry 26.3 ids of dimension types, damage types, variants and
// enchantments; remap them with `id_remap` where the orders differ.
#[must_use]
pub fn registry_data_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let mut reader = payload;
    let Ok(registry_id) = reader.get_str_borrowed() else {
        return Some(payload.to_vec());
    };
    let name = registry_id
        .strip_prefix("minecraft:")
        .unwrap_or(registry_id);

    match synced_for(version) {
        None => Some(payload.to_vec()),
        // Core sends the biome registry first; the rest are already in the codec.
        Some(Synced::Codec(codec)) => (name == "worldgen/biome").then(|| codec.to_vec()),
        Some(Synced::Registries(registries)) => {
            let registry = registries.iter().find(|r| r.id == name)?;
            let mut out = Vec::new();
            out.write_string(registry_id).ok()?;
            out.write_var_int(&VarInt(registry.entries.len() as i32))
                .ok()?;
            for (entry, data) in registry.entries {
                out.write_string(&format!("minecraft:{entry}")).ok()?;
                out.write_bool(true).ok()?;
                out.extend_from_slice(data);
            }
            Some(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_payload(id: &str) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.write_string(id).unwrap();
        payload.write_var_int(&VarInt(0)).unwrap();
        payload
    }

    #[test]
    fn biome_1_21_11_has_no_26_3_attributes() {
        let out = registry_data_from_current(
            &registry_payload("minecraft:worldgen/biome"),
            JavaMinecraftVersion::V_1_21_11,
        )
        .unwrap();
        let needle = b"natural_mob_spawns";
        assert!(!out.windows(needle.len()).any(|w| w == needle));
        assert!(out.len() > 1000);
    }

    #[test]
    fn unknown_registry_is_dropped() {
        assert!(
            registry_data_from_current(
                &registry_payload("minecraft:world_clock"),
                JavaMinecraftVersion::V_1_21_11,
            )
            .is_none()
        );
    }

    #[test]
    fn codec_1_20_2_only_on_biome() {
        let version = JavaMinecraftVersion::V_1_20_2;
        assert!(
            registry_data_from_current(&registry_payload("minecraft:worldgen/biome"), version)
                .is_some()
        );
        assert!(
            registry_data_from_current(&registry_payload("minecraft:chat_type"), version).is_none()
        );
    }

    fn read_named(bytes: &[u8]) -> pumpkin_nbt::compound::NbtCompound {
        let mut cursor = std::io::Cursor::new(bytes);
        let mut reader = pumpkin_nbt::deserializer::NbtReadHelperJava::new(&mut cursor);
        let nbt = pumpkin_nbt::Nbt::read(&mut reader).unwrap();
        assert_eq!(cursor.position() as usize, bytes.len());
        nbt.root_tag
    }

    #[test]
    fn login_codec_1_18_biomes_match_the_remap_order() {
        use pumpkin_nbt::tag::NbtTag;

        let codec = read_named(login_codec(JavaMinecraftVersion::V_1_18_2).unwrap());
        let biomes = codec.get_compound("minecraft:worldgen/biome").unwrap();
        let Some(NbtTag::List(values)) = biomes.get("value") else {
            panic!("no biome list");
        };
        let (_, names) = NAMES_1_18
            .iter()
            .find(|(id, _)| *id == "worldgen/biome")
            .unwrap();
        assert_eq!(values.len(), names.len());
        for (id, (value, name)) in values.iter().zip(*names).enumerate() {
            let NbtTag::Compound(value) = value else {
                panic!("biome entry is not a compound");
            };
            assert_eq!(value.get_int("id"), Some(id as i32));
            assert_eq!(
                value.get_string("name"),
                Some(format!("minecraft:{name}").as_str())
            );
        }
        assert!(codec.get_compound("minecraft:chat_type").is_none());
    }

    #[test]
    fn login_codec_1_16_lists_dimensions() {
        let codec = read_named(login_codec(JavaMinecraftVersion::V_1_16_1).unwrap());
        assert!(codec.get("dimension").is_some());
        assert!(login_codec(JavaMinecraftVersion::V_1_15_2).is_none());
        assert!(login_codec(JavaMinecraftVersion::V_1_20_2).is_none());
    }

    #[test]
    fn dimension_type_only_from_1_16_2_to_1_18_2() {
        let nether = read_named(
            dimension_type_nbt(JavaMinecraftVersion::V_1_16_2, "minecraft:the_nether").unwrap(),
        );
        assert_eq!(nether.get_int("ultrawarm"), Some(1));
        let custom = dimension_type_nbt(JavaMinecraftVersion::V_1_17, "plugin:custom");
        assert_eq!(
            custom,
            dimension_type_nbt(JavaMinecraftVersion::V_1_17, "minecraft:overworld")
        );
        assert!(
            dimension_type_nbt(JavaMinecraftVersion::V_1_16_1, "minecraft:overworld").is_none()
        );
        assert!(dimension_type_nbt(JavaMinecraftVersion::V_1_19, "minecraft:overworld").is_none());
    }
}
