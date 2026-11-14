//! Biome ids in chunk sections.

use pumpkin_util::version::JavaMinecraftVersion;

use super::{
    ceil_log2,
    registry::{IdRemap, current_names, id_remap},
};

// TODO: CHUNKS_BIOMES (`/fillbiome`) still carries 26.3 biome ids.

/// Classic numeric biome id for clients before 1.16 (no synced biome registry).
const fn legacy_biome_id(name: &str) -> u32 {
    match name.as_bytes() {
        b"ocean" => 0,
        b"plains" | b"sunflower_plains" => 1,
        b"desert" => 2,
        b"windswept_hills"
        | b"windswept_gravelly_hills"
        | b"windswept_forest"
        | b"meadow"
        | b"stony_peaks"
        | b"jagged_peaks"
        | b"frozen_peaks" => 3,
        b"forest" | b"flower_forest" | b"cherry_grove" => 4,
        b"taiga" | b"old_growth_pine_taiga" | b"old_growth_spruce_taiga" | b"grove" => 5,
        b"swamp" | b"mangrove_swamp" => 6,
        b"river" => 7,
        b"nether_wastes" | b"crimson_forest" | b"warped_forest" | b"soul_sand_valley"
        | b"basalt_deltas" => 8,
        b"the_end" | b"small_end_islands" | b"end_midlands" | b"end_highlands" | b"end_barrens" => {
            9
        }
        b"frozen_ocean" | b"deep_frozen_ocean" => 10,
        b"frozen_river" => 11,
        b"snowy_plains" | b"ice_spikes" => 12,
        b"mushroom_fields" => 14,
        b"beach" | b"stony_shore" => 16,
        b"jungle" | b"sparse_jungle" | b"bamboo_jungle" => 21,
        b"deep_ocean"
        | b"warm_ocean"
        | b"lukewarm_ocean"
        | b"deep_lukewarm_ocean"
        | b"cold_ocean"
        | b"deep_cold_ocean" => 24,
        b"birch_forest" | b"old_growth_birch_forest" => 27,
        b"dark_forest" | b"pale_garden" => 29,
        b"snowy_taiga" => 30,
        b"savanna" | b"savanna_plateau" | b"windswept_savanna" => 35,
        b"badlands" | b"wooded_badlands" | b"eroded_badlands" => 37,
        _ => 1,
    }
}

/// 26.3 biome ids to the client's, `plains` for biomes the client lacks.
pub struct BiomeRemap(Option<IdRemap>);

impl BiomeRemap {
    #[must_use]
    pub fn new(version: JavaMinecraftVersion) -> Self {
        if version < JavaMinecraftVersion::V_1_16
            && let Some(current) = current_names("worldgen/biome")
        {
            return Self(Some(IdRemap {
                table: current.iter().map(|name| legacy_biome_id(name)).collect(),
                client_len: 40,
            }));
        }
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
