//! Payload translation between 26.3 and older clients, one file per encoding step.

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{ClientPacket, ServerPacket};
use pumpkin_util::version::JavaMinecraftVersion;

/// A component's encoding eras. Each variant starts at its version and lasts until the next
/// one, so versions where the component did not change share a variant. `of` is the only
/// version comparison; versions before the first start use the first variant.
macro_rules! eras {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$variant_meta:meta])* $variant:ident = $start:ident),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        $vis enum $name {
            $($(#[$variant_meta])* $variant),+
        }

        impl $name {
            #[must_use]
            #[allow(unused_assignments)]
            $vis fn of(version: ::pumpkin_util::version::JavaMinecraftVersion) -> Self {
                let mut era = [$(Self::$variant),+][0];
                $(
                    if version >= ::pumpkin_util::version::JavaMinecraftVersion::$start {
                        era = Self::$variant;
                    }
                )+
                era
            }
        }
    };
}

pub mod advancement;
pub mod animation;
pub mod biome;
pub mod block;
pub mod chunk;
pub mod entity;
pub mod item;
pub mod light;
pub mod login;
pub mod movement;
pub mod nbt;
pub mod palette;
pub mod player_spawn;
pub mod recipe;
pub mod registry;
pub mod resource_pack;
pub mod serverbound;
pub mod tags;
pub mod team;
pub mod time;

/// Reads `P` as 26.3 and writes it for `version`.
pub(crate) fn reencode_current<'a, P: ServerPacket<'a> + ClientPacket>(
    mut payload: &'a [u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let packet = P::read(&mut payload, &CURRENT_MC_VERSION).ok()?;
    let mut out = Vec::new();
    packet.write_packet_data(&mut out, &version).ok()?;
    Some(out)
}

/// Bits needed to index `count` entries (vanilla `Mth.ceillog2`).
pub(crate) const fn ceil_log2(count: u32) -> u8 {
    if count <= 1 {
        0
    } else {
        (32 - (count - 1).leading_zeros()) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::{chunk::ChunkFormat, registry::DatapackVersion};
    use pumpkin_util::version::JavaMinecraftVersion as V;

    #[test]
    fn eras_cover_ranges() {
        assert_eq!(ChunkFormat::of(V::V_1_7_2), ChunkFormat::V1_7);
        assert_eq!(ChunkFormat::of(V::V_1_17_1), ChunkFormat::V1_7);
        assert_eq!(ChunkFormat::of(V::V_1_18), ChunkFormat::V1_18);
        assert_eq!(ChunkFormat::of(V::V_1_21_4), ChunkFormat::V1_18);
        assert_eq!(ChunkFormat::of(V::V_1_21_11), ChunkFormat::V1_21_6);
        assert_eq!(ChunkFormat::of(V::V_26_3), ChunkFormat::V26_1);
        // Eras may start mid-release: 1.19.4 already uses the 1.20 datapack
        assert_eq!(DatapackVersion::of(V::V_1_19_3), DatapackVersion::V1_19);
        assert_eq!(DatapackVersion::of(V::V_1_19_4), DatapackVersion::V1_20);
        assert_eq!(DatapackVersion::of(V::Unknown), DatapackVersion::V26_3);
    }
}
