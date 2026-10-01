use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_plugin_api::events_wit::ConnectionState;
use pumpkin_protocol::java::client::login::CEncryptionRequest;
use pumpkin_util::version::JavaMinecraftVersion;

use crate::remap;
use crate::translate::{
    advancement, animation, attribute, block, chunk, commands, entity, entity_data, explosion,
    game_event, inventory, light, login, movement, particle, player_info, player_spawn,
    plugin_message, recipe, reencode_current, registry, resource_pack, serverbound, sound, tags,
    team, time,
};
use pumpkin_protocol::java::legacy::ids;

type PayloadTranslator = fn(&[u8], JavaMinecraftVersion) -> Option<Vec<u8>>;
type IdAndPayloadTranslator = fn(&[u8], JavaMinecraftVersion) -> Option<(i32, Vec<u8>)>;

/// Converts the WIT-generated `JavaMinecraftVersion` into the internal `pumpkin_util` version.
#[must_use]
pub const fn from_wasm_java_version(
    version: pumpkin_plugin_api::wit::pumpkin::plugin::player::JavaMinecraftVersion,
) -> JavaMinecraftVersion {
    use pumpkin_plugin_api::wit::pumpkin::plugin::player::JavaMinecraftVersion as W;
    match version {
        W::V172 => JavaMinecraftVersion::V_1_7_2,
        W::V176 => JavaMinecraftVersion::V_1_7_6,
        W::V18 => JavaMinecraftVersion::V_1_8,
        W::V19 => JavaMinecraftVersion::V_1_9,
        W::V191 => JavaMinecraftVersion::V_1_9_1,
        W::V192 => JavaMinecraftVersion::V_1_9_2,
        W::V193 => JavaMinecraftVersion::V_1_9_3,
        W::V110 => JavaMinecraftVersion::V_1_10,
        W::V111 => JavaMinecraftVersion::V_1_11,
        W::V1111 => JavaMinecraftVersion::V_1_11_1,
        W::V112 => JavaMinecraftVersion::V_1_12,
        W::V1121 => JavaMinecraftVersion::V_1_12_1,
        W::V1122 => JavaMinecraftVersion::V_1_12_2,
        W::V113 => JavaMinecraftVersion::V_1_13,
        W::V1131 => JavaMinecraftVersion::V_1_13_1,
        W::V1132 => JavaMinecraftVersion::V_1_13_2,
        W::V114 => JavaMinecraftVersion::V_1_14,
        W::V1141 => JavaMinecraftVersion::V_1_14_1,
        W::V1142 => JavaMinecraftVersion::V_1_14_2,
        W::V1143 => JavaMinecraftVersion::V_1_14_3,
        W::V1144 => JavaMinecraftVersion::V_1_14_4,
        W::V115 => JavaMinecraftVersion::V_1_15,
        W::V1151 => JavaMinecraftVersion::V_1_15_1,
        W::V1152 => JavaMinecraftVersion::V_1_15_2,
        W::V116 => JavaMinecraftVersion::V_1_16,
        W::V1161 => JavaMinecraftVersion::V_1_16_1,
        W::V1162 => JavaMinecraftVersion::V_1_16_2,
        W::V1163 => JavaMinecraftVersion::V_1_16_3,
        W::V1164 => JavaMinecraftVersion::V_1_16_4,
        W::V117 => JavaMinecraftVersion::V_1_17,
        W::V1171 => JavaMinecraftVersion::V_1_17_1,
        W::V118 => JavaMinecraftVersion::V_1_18,
        W::V1182 => JavaMinecraftVersion::V_1_18_2,
        W::V119 => JavaMinecraftVersion::V_1_19,
        W::V1191 => JavaMinecraftVersion::V_1_19_1,
        W::V1193 => JavaMinecraftVersion::V_1_19_3,
        W::V1194 => JavaMinecraftVersion::V_1_19_4,
        W::V120 => JavaMinecraftVersion::V_1_20,
        W::V1202 => JavaMinecraftVersion::V_1_20_2,
        W::V1203 => JavaMinecraftVersion::V_1_20_3,
        W::V1205 => JavaMinecraftVersion::V_1_20_5,
        W::V121 => JavaMinecraftVersion::V_1_21,
        W::V1212 => JavaMinecraftVersion::V_1_21_2,
        W::V1214 => JavaMinecraftVersion::V_1_21_4,
        W::V1215 => JavaMinecraftVersion::V_1_21_5,
        W::V1216 => JavaMinecraftVersion::V_1_21_6,
        W::V1217 => JavaMinecraftVersion::V_1_21_7,
        W::V1219 => JavaMinecraftVersion::V_1_21_9,
        W::V12111 => JavaMinecraftVersion::V_1_21_11,
        W::V261 => JavaMinecraftVersion::V_26_1,
        W::V262 => JavaMinecraftVersion::V_26_2,
        W::V263 => CURRENT_MC_VERSION,
        W::Unknown => JavaMinecraftVersion::Unknown,
    }
}

/// Converts the internal `pumpkin_util` version into the WIT-generated one.
#[must_use]
pub const fn to_wasm_java_version(
    version: JavaMinecraftVersion,
) -> pumpkin_plugin_api::wit::pumpkin::plugin::player::JavaMinecraftVersion {
    use pumpkin_plugin_api::wit::pumpkin::plugin::player::JavaMinecraftVersion as W;
    match version {
        JavaMinecraftVersion::V_1_7_2 => W::V172,
        JavaMinecraftVersion::V_1_7_6 => W::V176,
        JavaMinecraftVersion::V_1_8 => W::V18,
        JavaMinecraftVersion::V_1_9 => W::V19,
        JavaMinecraftVersion::V_1_9_1 => W::V191,
        JavaMinecraftVersion::V_1_9_2 => W::V192,
        JavaMinecraftVersion::V_1_9_3 => W::V193,
        JavaMinecraftVersion::V_1_10 => W::V110,
        JavaMinecraftVersion::V_1_11 => W::V111,
        JavaMinecraftVersion::V_1_11_1 => W::V1111,
        JavaMinecraftVersion::V_1_12 => W::V112,
        JavaMinecraftVersion::V_1_12_1 => W::V1121,
        JavaMinecraftVersion::V_1_12_2 => W::V1122,
        JavaMinecraftVersion::V_1_13 => W::V113,
        JavaMinecraftVersion::V_1_13_1 => W::V1131,
        JavaMinecraftVersion::V_1_13_2 => W::V1132,
        JavaMinecraftVersion::V_1_14 => W::V114,
        JavaMinecraftVersion::V_1_14_1 => W::V1141,
        JavaMinecraftVersion::V_1_14_2 => W::V1142,
        JavaMinecraftVersion::V_1_14_3 => W::V1143,
        JavaMinecraftVersion::V_1_14_4 => W::V1144,
        JavaMinecraftVersion::V_1_15 => W::V115,
        JavaMinecraftVersion::V_1_15_1 => W::V1151,
        JavaMinecraftVersion::V_1_15_2 => W::V1152,
        JavaMinecraftVersion::V_1_16 => W::V116,
        JavaMinecraftVersion::V_1_16_1 => W::V1161,
        JavaMinecraftVersion::V_1_16_2 => W::V1162,
        JavaMinecraftVersion::V_1_16_3 => W::V1163,
        JavaMinecraftVersion::V_1_16_4 => W::V1164,
        JavaMinecraftVersion::V_1_17 => W::V117,
        JavaMinecraftVersion::V_1_17_1 => W::V1171,
        JavaMinecraftVersion::V_1_18 => W::V118,
        JavaMinecraftVersion::V_1_18_2 => W::V1182,
        JavaMinecraftVersion::V_1_19 => W::V119,
        JavaMinecraftVersion::V_1_19_1 => W::V1191,
        JavaMinecraftVersion::V_1_19_3 => W::V1193,
        JavaMinecraftVersion::V_1_19_4 => W::V1194,
        JavaMinecraftVersion::V_1_20 => W::V120,
        JavaMinecraftVersion::V_1_20_2 => W::V1202,
        JavaMinecraftVersion::V_1_20_3 => W::V1203,
        JavaMinecraftVersion::V_1_20_5 => W::V1205,
        JavaMinecraftVersion::V_1_21 => W::V121,
        JavaMinecraftVersion::V_1_21_2 => W::V1212,
        JavaMinecraftVersion::V_1_21_4 => W::V1214,
        JavaMinecraftVersion::V_1_21_5 => W::V1215,
        JavaMinecraftVersion::V_1_21_6 => W::V1216,
        JavaMinecraftVersion::V_1_21_7 => W::V1217,
        JavaMinecraftVersion::V_1_21_9 => W::V1219,
        JavaMinecraftVersion::V_1_21_11 => W::V12111,
        JavaMinecraftVersion::V_26_1 => W::V261,
        JavaMinecraftVersion::V_26_2 => W::V262,
        JavaMinecraftVersion::V_26_3 => W::V263,
        JavaMinecraftVersion::Unknown => W::Unknown,
    }
}

/// The core connection state of the plugin's `state`.
const fn protocol_state(state: ConnectionState) -> pumpkin_protocol::ConnectionState {
    use pumpkin_protocol::ConnectionState as P;
    match state {
        ConnectionState::Handshake => P::HandShake,
        ConnectionState::Status => P::Status,
        ConnectionState::Login | ConnectionState::Transfer => P::Login,
        ConnectionState::Config => P::Config,
        ConnectionState::Play => P::Play,
    }
}

/// Current id of the client's `client_id` in `state`. Ids are only unique within a state.
fn serverbound_id(
    state: ConnectionState,
    client_id: i32,
    version: JavaMinecraftVersion,
) -> Option<i32> {
    ids::serverbound_to_current(protocol_state(state), client_id, version)
}

/// Client id of the current `current_id` in `state`. `None` when the client has no such packet.
fn clientbound_id(
    state: ConnectionState,
    current_id: i32,
    version: JavaMinecraftVersion,
) -> Option<i32> {
    ids::clientbound_id(protocol_state(state), current_id, version)
}

pub struct PacketTranslator;

impl PacketTranslator {
    /// Translates an incoming play packet ID from the client's version into the current one.
    #[must_use]
    pub fn translate_serverbound_packet_id(
        packet_id: i32,
        version: JavaMinecraftVersion,
    ) -> Option<i32> {
        if version == CURRENT_MC_VERSION {
            return Some(packet_id);
        }
        serverbound_id(ConnectionState::Play, packet_id, version)
    }

    /// Translates an outgoing current play packet ID into the client's version.
    #[must_use]
    pub fn translate_clientbound_packet_id(
        current_id: i32,
        version: JavaMinecraftVersion,
    ) -> Option<i32> {
        if version == CURRENT_MC_VERSION {
            return Some(current_id);
        }
        clientbound_id(ConnectionState::Play, current_id, version)
    }

    /// Translates a sound ID from 26.3 to the client's version.
    #[must_use]
    pub fn translate_sound_id(sound_id: u16, version: JavaMinecraftVersion) -> u16 {
        remap::sound_id_remap::remap_sound_id_for_version(sound_id, version)
    }

    /// Translates a block state ID from 26.3 to the client's version.
    #[must_use]
    pub fn translate_block_state(state_id: u16, version: JavaMinecraftVersion) -> u16 {
        remap::block_state_remap::remap_block_state_for_version(state_id, version)
    }

    /// Translates an item ID from 26.3 to the client's version.
    #[must_use]
    pub fn translate_item_id(item_id: u16, version: JavaMinecraftVersion) -> u16 {
        remap::item_id_remap::remap_item_id_for_version(item_id, version)
    }

    /// Translates an incoming item ID from the client's version to 26.3.
    #[must_use]
    pub fn translate_item_id_to_server(item_id: u16, version: JavaMinecraftVersion) -> u16 {
        remap::item_id_remap::remap_item_id_from_version(item_id, version)
    }

    /// Translates an entity type ID from 26.3 to the client's version.
    #[must_use]
    pub fn translate_entity_id(entity_id: u16, version: JavaMinecraftVersion) -> u16 {
        remap::entity_id_remap::remap_entity_id_for_version(entity_id, version)
    }

    /// Translates a particle ID from 26.3 to the client's version.
    #[must_use]
    pub fn translate_particle_id(particle_id: u16, version: JavaMinecraftVersion) -> u16 {
        remap::particle_id_remap::remap_particle_id_for_version(particle_id, version)
    }

    /// Translates a menu ID from 26.3 to the client's version.
    #[must_use]
    pub fn translate_menu_id(menu_id: u8, version: JavaMinecraftVersion) -> u8 {
        remap::menu_id_remap::remap_menu_id_for_version(menu_id, version)
    }

    /// Translates an attribute ID from 26.3 to the client's version.
    #[must_use]
    pub fn translate_attribute_id(attr_id: u8, version: JavaMinecraftVersion) -> u8 {
        remap::attribute_id_remap::remap_attribute_id_for_version(u32::from(attr_id), version) as u8
    }

    /// Translates a custom stat ID from 26.3 to the client's version.
    #[must_use]
    pub fn translate_custom_stat_id(stat_id: u16, version: JavaMinecraftVersion) -> u16 {
        remap::custom_stat_id_remap::remap_custom_stat_id_for_version(u32::from(stat_id), version)
            as u16
    }

    /// Translates a painting variant ID from 26.3 to the client's version.
    #[must_use]
    pub fn translate_painting_variant(variant_id: u32, version: JavaMinecraftVersion) -> u32 {
        remap::painting_variant_id_remap::remap_motive_id_for_version(variant_id, version)
    }

    /// Translates an incoming packet (from an older client to 26.3).
    /// Returns the normalized 26.3 packet ID and potentially translated payload.
    /// `rotation` is the player's yaw and pitch, only read for packets that lack them.
    #[must_use]
    pub fn translate_incoming_packet(
        packet_id: i32,
        raw_payload: &[u8],
        version: JavaMinecraftVersion,
        rotation: impl FnOnce() -> (f32, f32),
    ) -> Option<(i32, Vec<u8>)> {
        if version == CURRENT_MC_VERSION {
            return None;
        }

        let new_id = Self::translate_serverbound_packet_id(packet_id, version)?;
        if new_id == ids::serverbound::play::INTERACT.current() {
            return serverbound::interact_to_current(raw_payload, version);
        }
        // Dropped when unreadable: kept as is, the client's item ids would be stored
        if new_id == ids::serverbound::play::SET_CREATIVE_MODE_SLOT.current() {
            return inventory::creative_slot_to_current(raw_payload, version)
                .map(|payload| (new_id, payload));
        }
        if new_id == ids::serverbound::play::CONTAINER_CLICK.current() {
            return inventory::container_click_to_current(raw_payload, version)
                .map(|payload| (new_id, payload));
        }
        let translated_payload =
            serverbound::play_to_current(new_id, raw_payload, version, rotation)
                .unwrap_or_else(|| raw_payload.to_vec());
        Some((new_id, translated_payload))
    }

    /// Translates an incoming pre-play packet (status / login / config) to 26.3. Ids are only
    /// unique within a state, so the lookup is limited to `state`'s table.
    #[must_use]
    pub fn translate_connection_incoming(
        state: ConnectionState,
        packet_id: i32,
        raw_payload: &[u8],
        version: JavaMinecraftVersion,
    ) -> Option<(i32, Vec<u8>)> {
        if version == CURRENT_MC_VERSION {
            return None;
        }
        let new_id = serverbound_id(state, packet_id, version)?;

        let translated_payload = match state {
            ConnectionState::Login | ConnectionState::Transfer
                if new_id == ids::serverbound::login::HELLO.current() =>
            {
                login::hello_to_current(raw_payload, version)
            }
            ConnectionState::Login | ConnectionState::Transfer
                if new_id == ids::serverbound::login::KEY.current() =>
            {
                login::key_to_current(raw_payload, version)
            }
            ConnectionState::Config
                if new_id == ids::serverbound::config::RESOURCE_PACK.current() =>
            {
                resource_pack::response_to_current(raw_payload, version)
            }
            ConnectionState::Config
                if new_id == ids::serverbound::config::CUSTOM_PAYLOAD.current() =>
            {
                plugin_message::to_current(raw_payload, version)
            }
            _ => None,
        };
        Some((
            new_id,
            translated_payload.unwrap_or_else(|| raw_payload.to_vec()),
        ))
    }

    fn connection_outgoing_payload(
        state: ConnectionState,
        current_id: i32,
        raw_payload: &[u8],
        version: JavaMinecraftVersion,
    ) -> Option<Vec<u8>> {
        match state {
            ConnectionState::Login | ConnectionState::Transfer
                if current_id == ids::clientbound::login::HELLO.current() =>
            {
                reencode_current::<CEncryptionRequest>(raw_payload, version)
            }
            ConnectionState::Login | ConnectionState::Transfer
                if current_id == ids::clientbound::login::LOGIN_FINISHED.current() =>
            {
                login::login_success_from_current(raw_payload, version)
            }
            ConnectionState::Config
                if current_id == ids::clientbound::config::RESOURCE_PACK_PUSH.current() =>
            {
                resource_pack::push_from_current(raw_payload, version)
            }
            ConnectionState::Config
                if current_id == ids::clientbound::config::UPDATE_TAGS.current() =>
            {
                tags::update_tags_from_current(raw_payload, version)
            }
            ConnectionState::Config
                if current_id == ids::clientbound::config::CUSTOM_PAYLOAD.current() =>
            {
                plugin_message::from_current(raw_payload, version)
            }
            _ => None,
        }
    }

    /// Maps an outgoing pre-play 26.3 packet id to the client's id for `state`.
    /// `None` when the packet does not exist for the client and must be dropped.
    #[must_use]
    pub fn translate_connection_outgoing_id(
        state: ConnectionState,
        current_id: i32,
        version: JavaMinecraftVersion,
    ) -> Option<i32> {
        if version == CURRENT_MC_VERSION {
            return Some(current_id);
        }
        clientbound_id(state, current_id, version)
    }

    /// Translates an outgoing pre-play 26.3 packet to the client's id and payload.
    /// `None` when the packet does not exist for the client and must be dropped.
    #[must_use]
    pub fn translate_connection_outgoing(
        state: ConnectionState,
        current_id: i32,
        raw_payload: &[u8],
        version: JavaMinecraftVersion,
    ) -> Option<(i32, Vec<u8>)> {
        let client_id = Self::translate_connection_outgoing_id(state, current_id, version)?;
        if version == CURRENT_MC_VERSION {
            return Some((client_id, raw_payload.to_vec()));
        }
        if version < JavaMinecraftVersion::V_1_8
            && current_id == ids::clientbound::login::LOGIN_COMPRESSION.current()
        {
            return None;
        }
        if matches!(state, ConnectionState::Config)
            && current_id == ids::clientbound::config::REGISTRY_DATA.current()
        {
            let payload = registry::registry_data_from_current(raw_payload, version)?;
            return Some((client_id, payload));
        }
        let payload = Self::connection_outgoing_payload(state, current_id, raw_payload, version)
            .unwrap_or_else(|| raw_payload.to_vec());
        Some((client_id, payload))
    }

    /// Translates an outgoing packet (from 26.3 server to an older client): packet id,
    /// plus the payload for packets whose encoding changed.
    #[must_use]
    pub fn translate_outgoing_packet(
        packet_id: i32,
        raw_payload: &[u8],
        version: JavaMinecraftVersion,
    ) -> Option<(i32, Vec<u8>)> {
        if version == CURRENT_MC_VERSION {
            return None;
        }
        if let Some(translate) = Self::play_outgoing_packet(packet_id) {
            return translate(raw_payload, version);
        }

        let client_id = Self::translate_clientbound_packet_id(packet_id, version)?;
        // A 26.3 payload the client cannot read is dropped, not sent
        let payload = match Self::play_outgoing_payload(packet_id) {
            Some(translate) => translate(raw_payload, version)?,
            None => raw_payload.to_vec(),
        };
        Some((client_id, payload))
    }

    /// Packets whose client id depends on the version or payload.
    fn play_outgoing_packet(current_id: i32) -> Option<IdAndPayloadTranslator> {
        use ids::clientbound::play;

        Some(match current_id {
            id if id == play::ADD_ENTITY.current() => entity::add_entity_from_current,
            id if id == play::ENTITY_POSITION_SYNC.current() => {
                movement::position_sync_from_current
            }
            id if id == play::SWING_ANIMATION.current() => animation::swing_from_current,
            _ => return None,
        })
    }

    /// Payload translation of packets whose layout changed since the client's version.
    fn play_outgoing_payload(current_id: i32) -> Option<PayloadTranslator> {
        use ids::clientbound::play;

        Some(match current_id {
            id if id == play::LOGIN.current() => player_spawn::login_from_current,
            id if id == play::COMMANDS.current() => commands::commands_from_current,
            id if id == play::RESPAWN.current() => player_spawn::respawn_from_current,
            id if id == play::SET_DEFAULT_SPAWN_POSITION.current() => {
                player_spawn::spawn_position_from_current
            }
            id if id == play::UPDATE_ADVANCEMENTS.current() => {
                advancement::update_advancements_from_current
            }
            id if id == play::LEVEL_CHUNK_WITH_LIGHT.current() => chunk::chunk_from_current,
            id if id == play::LIGHT_UPDATE.current() => light::light_update_from_current,
            id if id == play::SET_PLAYER_TEAM.current() => team::set_player_team_from_current,
            id if id == play::PLAYER_INFO_UPDATE.current() => {
                player_info::player_info_update_from_current
            }
            id if id == play::MOVE_ENTITY_POS.current() => movement::pos_from_current,
            id if id == play::MOVE_ENTITY_POS_ROT.current() => movement::pos_rot_from_current,
            id if id == play::MOVE_ENTITY_ROT.current() => movement::rot_from_current,
            id if id == play::SET_ENTITY_MOTION.current() => movement::entity_motion_from_current,
            id if id == play::PLAYER_ROTATION.current() => movement::player_rotation_from_current,
            id if id == play::PLAYER_POSITION.current() => movement::player_position_from_current,
            id if id == play::ANIMATE.current() => animation::animate_from_current,
            id if id == play::SET_TIME.current() => time::set_time_from_current,
            id if id == play::RECIPE_BOOK_ADD.current() => recipe::recipe_book_add_from_current,
            id if id == play::PLACE_GHOST_RECIPE.current() => {
                recipe::place_ghost_recipe_from_current
            }
            id if id == play::UPDATE_RECIPES.current() => recipe::update_recipes_from_current,
            id if id == play::UPDATE_TAGS.current() => tags::play_update_tags_from_current,
            id if id == play::BLOCK_UPDATE.current() => block::block_update_from_current,
            id if id == play::BLOCK_ENTITY_DATA.current() => block::block_entity_data_from_current,
            id if id == play::SECTION_BLOCKS_UPDATE.current() => {
                block::section_blocks_update_from_current
            }
            id if id == play::SET_ENTITY_DATA.current() => {
                entity_data::set_entity_data_from_current
            }
            id if id == play::REMOVE_ENTITIES.current() => {
                entity_data::remove_entities_from_current
            }
            id if id == play::CONTAINER_SET_CONTENT.current() => {
                inventory::container_set_content_from_current
            }
            id if id == play::CONTAINER_SET_SLOT.current() => {
                inventory::container_set_slot_from_current
            }
            id if id == play::SET_CURSOR_ITEM.current() => inventory::set_cursor_item_from_current,
            id if id == play::SET_PLAYER_INVENTORY.current() => {
                inventory::set_player_inventory_from_current
            }
            id if id == play::SET_EQUIPMENT.current() => inventory::set_equipment_from_current,
            id if id == play::SOUND.current() => sound::sound_from_current,
            id if id == play::EXPLODE.current() => explosion::explode_from_current,
            id if id == play::GAME_EVENT.current() => game_event::game_event_from_current,
            id if id == play::LEVEL_PARTICLES.current() => particle::level_particles_from_current,
            id if id == play::UPDATE_ATTRIBUTES.current() => {
                attribute::update_attributes_from_current
            }
            id if id == play::SOUND_ENTITY.current() => sound::sound_entity_from_current,
            id if id == play::STOP_SOUND.current() => sound::stop_sound_from_current,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1_21_11: JavaMinecraftVersion = JavaMinecraftVersion::V_1_21_11;

    #[test]
    fn renamed_packets_use_the_older_row() {
        use ids::{clientbound, serverbound};

        assert_eq!(
            PacketTranslator::translate_clientbound_packet_id(
                clientbound::play::SET_HELD_SLOT.current(),
                V1_21_11
            ),
            Some(clientbound::play::SET_CARRIED_ITEM.to_id(V1_21_11))
        );
        assert_eq!(
            PacketTranslator::translate_serverbound_packet_id(
                serverbound::play::SWING.to_id(V1_21_11),
                V1_21_11
            ),
            Some(serverbound::play::PUNCH.current())
        );
        let v1_20 = JavaMinecraftVersion::V_1_20;
        assert_eq!(
            PacketTranslator::translate_connection_outgoing_id(
                ConnectionState::Login,
                clientbound::login::LOGIN_FINISHED.current(),
                v1_20
            ),
            Some(clientbound::login::GAME_PROFILE.to_id(v1_20))
        );
    }

    #[test]
    fn only_new_play_packets_are_missing_for_1_21_11() {
        use ids::clientbound::play;

        let new = [
            play::ADD_TRANSIENT_BLOCK.current(),
            play::GAME_RULE_VALUES.current(),
            play::LOW_DISK_SPACE_WARNING.current(),
            play::POST_EFFECTS.current(),
            play::SWING_ANIMATION.current(),
        ];
        let current_ids = (0..256).filter(|&id| {
            ids::clientbound_id(
                pumpkin_protocol::ConnectionState::Play,
                id,
                CURRENT_MC_VERSION,
            )
            .is_some()
        });
        for current in current_ids {
            let id = PacketTranslator::translate_clientbound_packet_id(current, V1_21_11);
            assert_eq!(id.is_none(), new.contains(&current), "{current}");
        }
    }

    #[test]
    fn every_1_21_11_serverbound_play_packet_maps() {
        let mapped = |id| PacketTranslator::translate_serverbound_packet_id(id, V1_21_11).is_some();
        let last = (0..256)
            .rfind(|&id| mapped(id))
            .expect("1.21.11 has play packets");
        assert!((0..=last).all(mapped));
    }
}
