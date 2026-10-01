//! Entity spawning: ADD_ENTITY.

use pumpkin_data::entity::EntityType;
use pumpkin_protocol::java::legacy::{LegacyPacket, LegacyWrite};
use pumpkin_protocol::{VarInt, java::client::play::CSpawnEntity};
use pumpkin_util::{math::position::BlockPos, version::JavaMinecraftVersion};

use crate::remap::{
    block_state_remap::remap_block_state_for_version,
    entity_id_remap::{remap_entity_id_for_version, remap_object_type_for_version},
    painting_variant_id_remap::remap_motive_id_for_version,
};
use crate::translate::entity_data;
use pumpkin_protocol::java::legacy::ids;
use pumpkin_protocol::java::legacy::removed::{
    spawn_living_entity::CSpawnLivingEntity, spawn_painting::CSpawnPainting,
};

eras! {
    pub enum SpawnFormat {
        /// Object type ids; paintings and living mobs have their own spawn packets.
        V1_7 = V_1_7_2,
        /// Entity type ids.
        V1_14 = V_1_14,
        /// Every entity spawns with ADD_ENTITY.
        V1_19 = V_1_19,
    }
}

/// ADD_ENTITY, as `(client packet id, payload)`: the client's spawn packet, entity type and
/// falling block state.
pub fn add_entity_from_current(
    raw_payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<(i32, Vec<u8>)> {
    let spawn_entity = CSpawnEntity::read_packet_data(raw_payload).ok()?;
    let entity_type_id = spawn_entity.r#type.0 as u16;
    entity_data::track_spawn(spawn_entity.entity_id.0, entity_type_id);
    let format = SpawnFormat::of(version);

    if format < SpawnFormat::V1_19 && entity_type_id == EntityType::PAINTING.id {
        let painting = CSpawnPainting::new(
            spawn_entity.entity_id,
            spawn_entity.entity_uuid,
            String::new(),
            VarInt(remap_motive_id_for_version(spawn_entity.data.0 as u32, version) as i32),
            BlockPos::new(
                spawn_entity.position.x.floor() as i32,
                spawn_entity.position.y.floor() as i32,
                spawn_entity.position.z.floor() as i32,
            ),
            spawn_entity.yaw,
        );
        let mut buf = Vec::new();
        if let Some(id) = CSpawnPainting::legacy_id(version)
            && painting.write_legacy(&mut buf, &version).is_ok()
        {
            return Some((id, buf));
        }
    }

    if format < SpawnFormat::V1_19 && EntityType::from_raw(entity_type_id).is_some_and(|e| e.mob) {
        let living = CSpawnLivingEntity::new(
            spawn_entity.entity_id,
            spawn_entity.entity_uuid,
            VarInt(i32::from(remap_living_mob_type_for_version(
                entity_type_id,
                version,
            ))),
            spawn_entity.position,
            spawn_entity.pitch_degrees(),
            spawn_entity.yaw_degrees(),
            spawn_entity.head_yaw_degrees(),
            spawn_entity.velocity.0,
            None,
        );
        let mut buf = Vec::new();
        if let Some(id) = CSpawnLivingEntity::legacy_id(version)
            && living.write_legacy(&mut buf, &version).is_ok()
        {
            return Some((id, buf));
        }
    }

    let remapped_type = if format == SpawnFormat::V1_7 {
        VarInt(i32::from(remap_object_type_for_version(
            entity_type_id,
            version,
        )))
    } else {
        VarInt(i32::from(remap_entity_id_for_version(
            entity_type_id,
            version,
        )))
    };
    let remapped_data = if entity_type_id == EntityType::FALLING_BLOCK.id {
        u16::try_from(spawn_entity.data.0).map_or(spawn_entity.data, |state_id| {
            VarInt(i32::from(remap_block_state_for_version(state_id, version)))
        })
    } else {
        spawn_entity.data
    };

    let modified_spawn = CSpawnEntity {
        r#type: remapped_type,
        data: remapped_data,
        ..spawn_entity
    };
    let mut buf = Vec::new();
    modified_spawn.write_legacy(&mut buf, &version).ok()?;
    Some((ids::clientbound::play::ADD_ENTITY.to_id(version), buf))
}

/// Mob type ids of the separate living spawn packet.
#[must_use]
#[expect(clippy::too_many_lines)]
pub fn remap_living_mob_type_for_version(entity_id: u16, version: JavaMinecraftVersion) -> u16 {
    if version >= JavaMinecraftVersion::V_1_14 {
        return remap_entity_id_for_version(entity_id, version);
    }

    if version >= JavaMinecraftVersion::V_1_11 {
        // 1.11 - 1.13 separate entity IDs
        if entity_id == EntityType::ELDER_GUARDIAN.id {
            return 4;
        } else if entity_id == EntityType::WITHER_SKELETON.id {
            return 5;
        } else if entity_id == EntityType::STRAY.id {
            return 6;
        } else if entity_id == EntityType::HUSK.id {
            return 23;
        } else if entity_id == EntityType::ZOMBIE_VILLAGER.id {
            return 27;
        } else if entity_id == EntityType::SKELETON_HORSE.id {
            return 28;
        } else if entity_id == EntityType::ZOMBIE_HORSE.id {
            return 29;
        } else if entity_id == EntityType::DONKEY.id {
            return 31;
        } else if entity_id == EntityType::MULE.id {
            return 32;
        } else if entity_id == EntityType::EVOKER.id {
            return 34;
        } else if entity_id == EntityType::VEX.id {
            return 35;
        } else if entity_id == EntityType::VINDICATOR.id {
            return 36;
        } else if entity_id == EntityType::ILLUSIONER.id {
            return 37;
        } else if entity_id == EntityType::LLAMA.id || entity_id == EntityType::TRADER_LLAMA.id {
            return 103;
        } else if entity_id == EntityType::PARROT.id {
            return 105;
        }
    }

    // 1.7.10 - 1.10 (and shared legacy base IDs for 1.11 - 1.12)
    if entity_id == EntityType::CREEPER.id {
        50
    } else if entity_id == EntityType::SKELETON.id
        || entity_id == EntityType::WITHER_SKELETON.id
        || entity_id == EntityType::STRAY.id
        || entity_id == EntityType::BOGGED.id
        || entity_id == EntityType::PARCHED.id
    {
        51
    } else if entity_id == EntityType::SPIDER.id {
        52
    } else if entity_id == EntityType::GIANT.id {
        53
    } else if entity_id == EntityType::ZOMBIE.id
        || entity_id == EntityType::DROWNED.id
        || entity_id == EntityType::HUSK.id
        || entity_id == EntityType::ZOMBIE_VILLAGER.id
    {
        54
    } else if entity_id == EntityType::SLIME.id {
        55
    } else if entity_id == EntityType::GHAST.id || entity_id == EntityType::HAPPY_GHAST.id {
        56
    } else if entity_id == EntityType::ZOMBIFIED_PIGLIN.id
        || entity_id == EntityType::PIGLIN.id
        || entity_id == EntityType::PIGLIN_BRUTE.id
    {
        57
    } else if entity_id == EntityType::ENDERMAN.id || entity_id == EntityType::CREAKING.id {
        58
    } else if entity_id == EntityType::CAVE_SPIDER.id {
        59
    } else if entity_id == EntityType::SILVERFISH.id {
        60
    } else if entity_id == EntityType::BLAZE.id || entity_id == EntityType::BREEZE.id {
        61
    } else if entity_id == EntityType::MAGMA_CUBE.id {
        62
    } else if entity_id == EntityType::ENDER_DRAGON.id {
        63
    } else if entity_id == EntityType::WITHER.id {
        64
    } else if entity_id == EntityType::BAT.id
        || entity_id == EntityType::VEX.id
        || entity_id == EntityType::ALLAY.id
        || entity_id == EntityType::BEE.id
        || entity_id == EntityType::PARROT.id
    {
        65
    } else if entity_id == EntityType::WITCH.id {
        66
    } else if entity_id == EntityType::ENDERMITE.id {
        67
    } else if entity_id == EntityType::GUARDIAN.id || entity_id == EntityType::ELDER_GUARDIAN.id {
        68
    } else if entity_id == EntityType::SHULKER.id {
        69
    } else if entity_id == EntityType::PIG.id
        || entity_id == EntityType::HOGLIN.id
        || entity_id == EntityType::ZOGLIN.id
        || entity_id == EntityType::STRIDER.id
    {
        90
    } else if entity_id == EntityType::SHEEP.id
        || entity_id == EntityType::GOAT.id
        || entity_id == EntityType::SNIFFER.id
        || entity_id == EntityType::ARMADILLO.id
    {
        91
    } else if entity_id == EntityType::COW.id || entity_id == EntityType::PANDA.id {
        92
    } else if entity_id == EntityType::CHICKEN.id {
        93
    } else if entity_id == EntityType::SQUID.id
        || entity_id == EntityType::GLOW_SQUID.id
        || entity_id == EntityType::DOLPHIN.id
        || entity_id == EntityType::COD.id
        || entity_id == EntityType::SALMON.id
        || entity_id == EntityType::PUFFERFISH.id
        || entity_id == EntityType::TROPICAL_FISH.id
        || entity_id == EntityType::TADPOLE.id
        || entity_id == EntityType::AXOLOTL.id
        || entity_id == EntityType::FROG.id
        || entity_id == EntityType::NAUTILUS.id
    {
        94
    } else if entity_id == EntityType::WOLF.id || entity_id == EntityType::FOX.id {
        95
    } else if entity_id == EntityType::MOOSHROOM.id {
        96
    } else if entity_id == EntityType::SNOW_GOLEM.id {
        97
    } else if entity_id == EntityType::OCELOT.id || entity_id == EntityType::CAT.id {
        98
    } else if entity_id == EntityType::IRON_GOLEM.id
        || entity_id == EntityType::COPPER_GOLEM.id
        || entity_id == EntityType::RAVAGER.id
        || entity_id == EntityType::WARDEN.id
    {
        99
    } else if entity_id == EntityType::HORSE.id
        || entity_id == EntityType::DONKEY.id
        || entity_id == EntityType::MULE.id
        || entity_id == EntityType::ZOMBIE_HORSE.id
        || entity_id == EntityType::SKELETON_HORSE.id
        || entity_id == EntityType::CAMEL.id
        || entity_id == EntityType::LLAMA.id
        || entity_id == EntityType::TRADER_LLAMA.id
    {
        100
    } else if entity_id == EntityType::RABBIT.id {
        if version <= JavaMinecraftVersion::V_1_7_6 {
            93 // Chicken fallback in 1.7.10
        } else {
            101
        }
    } else if entity_id == EntityType::POLAR_BEAR.id {
        if version < JavaMinecraftVersion::V_1_10 {
            92 // Cow fallback in < 1.10
        } else {
            102
        }
    } else if entity_id == EntityType::VILLAGER.id
        || entity_id == EntityType::WANDERING_TRADER.id
        || entity_id == EntityType::PILLAGER.id
        || entity_id == EntityType::VINDICATOR.id
        || entity_id == EntityType::EVOKER.id
        || entity_id == EntityType::ILLUSIONER.id
    {
        120
    } else {
        54
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pumpkin_protocol::ClientPacket;

    #[test]
    fn standing_mob_gets_the_client_type() {
        let version = JavaMinecraftVersion::V_1_21_11;
        let spawn = |r#type: u16| {
            CSpawnEntity::new(
                VarInt(40),
                uuid::Uuid::nil(),
                VarInt(i32::from(r#type)),
                pumpkin_util::math::vector3::Vector3::new(1.0, 64.0, -3.0),
                10.0,
                90.0,
                90.0,
                VarInt(0),
                pumpkin_util::math::vector3::Vector3::new(0.0, 0.0, 0.0),
            )
        };
        let mut current = Vec::new();
        spawn(EntityType::WARDEN.id)
            .write_packet_data(&mut current)
            .unwrap();
        let client_type = remap_entity_id_for_version(EntityType::WARDEN.id, version);
        let mut expected = Vec::new();
        spawn(client_type)
            .write_legacy(&mut expected, &version)
            .unwrap();
        assert_eq!(
            add_entity_from_current(&current, version),
            Some((ids::clientbound::play::ADD_ENTITY.to_id(version), expected))
        );
    }
}
