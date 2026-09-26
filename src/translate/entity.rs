//! Entity spawning: ADD_ENTITY.

use pumpkin_data::entity::EntityType;
use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    ClientPacket, MultiVersionJavaPacket, VarInt, java::client::play::CSpawnEntity,
};
use pumpkin_util::{math::position::BlockPos, version::JavaMinecraftVersion};

use crate::packet::legacy::{CSpawnLivingEntity, CSpawnPainting};
use crate::packet::mappings;
use crate::remap::{
    block_state_remap::remap_block_state_for_version,
    entity_id_remap::{remap_entity_id_for_version, remap_object_type_for_version},
};
use crate::translate::entity_data;

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
    let spawn_entity = CSpawnEntity::read_packet_data(raw_payload, &CURRENT_MC_VERSION).ok()?;
    let entity_type_id = spawn_entity.r#type.0 as u16;
    entity_data::track_spawn(spawn_entity.entity_id.0, entity_type_id);
    let format = SpawnFormat::of(version);

    if format < SpawnFormat::V1_19 && entity_type_id == EntityType::PAINTING.id {
        let painting = CSpawnPainting::new(
            spawn_entity.entity_id,
            spawn_entity.entity_uuid,
            String::new(),
            spawn_entity.data,
            BlockPos::new(
                spawn_entity.position.x.floor() as i32,
                spawn_entity.position.y.floor() as i32,
                spawn_entity.position.z.floor() as i32,
            ),
            spawn_entity.yaw,
        );
        let mut buf = Vec::new();
        if painting.write_packet_data(&mut buf, &version).is_ok() {
            return Some((CSpawnPainting::to_id(version), buf));
        }
    }

    if format < SpawnFormat::V1_19 && EntityType::from_raw(entity_type_id).is_some_and(|e| e.mob) {
        let living = CSpawnLivingEntity::new(
            spawn_entity.entity_id,
            spawn_entity.entity_uuid,
            spawn_entity.r#type,
            spawn_entity.position,
            spawn_entity.pitch_degrees(),
            spawn_entity.yaw_degrees(),
            spawn_entity.head_yaw_degrees(),
            spawn_entity.velocity.0,
            None,
        );
        let mut buf = Vec::new();
        if living.write_packet_data(&mut buf, &version).is_ok() {
            return Some((CSpawnLivingEntity::to_id(version), buf));
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
    modified_spawn.write_packet_data(&mut buf, &version).ok()?;
    Some((mappings::clientbound::play::ADD_ENTITY.to_id(version), buf))
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .write_packet_data(&mut current, &CURRENT_MC_VERSION)
            .unwrap();
        let client_type = remap_entity_id_for_version(EntityType::WARDEN.id, version);
        let mut expected = Vec::new();
        spawn(client_type)
            .write_packet_data(&mut expected, &version)
            .unwrap();
        assert_eq!(
            add_entity_from_current(&current, version),
            Some((CSpawnEntity::to_id(version), expected))
        );
    }
}
