//! Biome ids in chunk sections.

use pumpkin_util::version::JavaMinecraftVersion;

use super::{
    ceil_log2,
    registry::{IdRemap, current_names, id_remap},
};

// TODO: CHUNKS_BIOMES (`/fillbiome`) still carries 26.3 biome ids.

/// 26.3 biome ids to the client's, `plains` for biomes the client lacks.
pub struct BiomeRemap(Option<IdRemap>);

impl BiomeRemap {
    #[must_use]
    pub fn new(version: JavaMinecraftVersion) -> Self {
        Self(id_remap("worldgen/biome", version, "plains"))
    }

    #[must_use]
    pub fn get(&self, id: u32) -> u32 {
        self.0.as_ref().map_or(id, |remap| remap.get(id))
    }

    /// Width of the client's global biome palette.
    #[must_use]
    pub fn direct_bits(&self) -> u8 {
        let len = self.0.as_ref().map_or_else(
            || current_names("worldgen/biome").map_or(64, |names| names.len() as u32),
            |remap| remap.client_len,
        );
        ceil_log2(len)
    }
}
