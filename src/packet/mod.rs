use pumpkin_util::version::JavaMinecraftVersion;

pub mod legacy;
pub mod mappings;
pub mod translator;

/// Returns whether a given Java edition version is supported by this multiversion plugin.
#[must_use]
pub fn is_version_supported(version: JavaMinecraftVersion) -> bool {
    matches!(
        version,
        JavaMinecraftVersion::V_1_7_2
            | JavaMinecraftVersion::V_1_7_6
            | JavaMinecraftVersion::V_1_8
            | JavaMinecraftVersion::V_1_9
            | JavaMinecraftVersion::V_1_9_1
            | JavaMinecraftVersion::V_1_9_2
            | JavaMinecraftVersion::V_1_9_3
            | JavaMinecraftVersion::V_1_10
            | JavaMinecraftVersion::V_1_11
            | JavaMinecraftVersion::V_1_11_1
            | JavaMinecraftVersion::V_1_12
            | JavaMinecraftVersion::V_1_12_1
            | JavaMinecraftVersion::V_1_12_2
            | JavaMinecraftVersion::V_1_13
            | JavaMinecraftVersion::V_1_13_1
            | JavaMinecraftVersion::V_1_13_2
            | JavaMinecraftVersion::V_1_14
            | JavaMinecraftVersion::V_1_14_1
            | JavaMinecraftVersion::V_1_14_2
            | JavaMinecraftVersion::V_1_14_3
            | JavaMinecraftVersion::V_1_14_4
            | JavaMinecraftVersion::V_1_15
            | JavaMinecraftVersion::V_1_15_1
            | JavaMinecraftVersion::V_1_15_2
            | JavaMinecraftVersion::V_1_16
            | JavaMinecraftVersion::V_1_16_1
            | JavaMinecraftVersion::V_1_16_2
            | JavaMinecraftVersion::V_1_16_3
            | JavaMinecraftVersion::V_1_16_4
            | JavaMinecraftVersion::V_1_17
            | JavaMinecraftVersion::V_1_17_1
            | JavaMinecraftVersion::V_1_18
            | JavaMinecraftVersion::V_1_18_2
            | JavaMinecraftVersion::V_1_19
            | JavaMinecraftVersion::V_1_19_1
            | JavaMinecraftVersion::V_1_19_3
            | JavaMinecraftVersion::V_1_19_4
            | JavaMinecraftVersion::V_1_20
            | JavaMinecraftVersion::V_1_20_2
            | JavaMinecraftVersion::V_1_20_3
            | JavaMinecraftVersion::V_1_20_5
            | JavaMinecraftVersion::V_1_21
            | JavaMinecraftVersion::V_1_21_2
            | JavaMinecraftVersion::V_1_21_4
            | JavaMinecraftVersion::V_1_21_5
            | JavaMinecraftVersion::V_1_21_6
            | JavaMinecraftVersion::V_1_21_7
            | JavaMinecraftVersion::V_1_21_9
            | JavaMinecraftVersion::V_1_21_11
            | JavaMinecraftVersion::V_26_1
            | JavaMinecraftVersion::V_26_2
            | JavaMinecraftVersion::V_26_3
    )
}

impl mappings::PacketId {
    /// The packet's id in the version core speaks (26.3).
    #[must_use]
    pub const fn current(&self) -> i32 {
        self.to_id(pumpkin_data::packet::CURRENT_MC_VERSION)
    }
}
