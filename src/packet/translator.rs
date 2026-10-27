use pumpkin_data::entity::EntityType;
use pumpkin_plugin_api::events::packet::ConnectionState;
use pumpkin_protocol::{
    ClientPacket, MultiVersionJavaPacket, VarInt, java::client::play::CSpawnEntity,
};
use pumpkin_util::{math::position::BlockPos, version::JavaMinecraftVersion};

use crate::packet::legacy::{CSpawnLivingEntity, CSpawnPainting};
use crate::packet::mappings::{self, PacketId};
use crate::remap::{
    self, block_state_remap::remap_block_state_for_version,
    entity_id_remap::remap_object_type_for_version,
};

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
        W::V263 => JavaMinecraftVersion::V_26_3,
        W::Unknown => JavaMinecraftVersion::Unknown,
    }
}

pub static SERVERBOUND_HANDSHAKE: &[&PacketId] = &[&mappings::serverbound::handshake::INTENTION];

pub static SERVERBOUND_STATUS: &[&PacketId] = &[
    &mappings::serverbound::status::PING_REQUEST,
    &mappings::serverbound::status::STATUS_REQUEST,
];

pub static SERVERBOUND_LOGIN: &[&PacketId] = &[
    &mappings::serverbound::login::COOKIE_RESPONSE,
    &mappings::serverbound::login::CUSTOM_QUERY_ANSWER,
    &mappings::serverbound::login::HELLO,
    &mappings::serverbound::login::KEY,
    &mappings::serverbound::login::LOGIN_ACKNOWLEDGED,
];

pub static SERVERBOUND_CONFIG: &[&PacketId] = &[
    &mappings::serverbound::config::ACCEPT_CODE_OF_CONDUCT,
    &mappings::serverbound::config::CLIENT_INFORMATION,
    &mappings::serverbound::config::COOKIE_RESPONSE,
    &mappings::serverbound::config::CUSTOM_CLICK_ACTION,
    &mappings::serverbound::config::CUSTOM_PAYLOAD,
    &mappings::serverbound::config::FINISH_CONFIGURATION,
    &mappings::serverbound::config::KEEP_ALIVE,
    &mappings::serverbound::config::PONG,
    &mappings::serverbound::config::RESOURCE_PACK,
    &mappings::serverbound::config::SELECT_KNOWN_PACKS,
];

pub static SERVERBOUND_PLAY: &[&PacketId] = &[
    &mappings::serverbound::play::ACCEPT_TELEPORTATION,
    &mappings::serverbound::play::ATTACK,
    &mappings::serverbound::play::BLOCK_ENTITY_TAG_QUERY,
    &mappings::serverbound::play::BUNDLE_ITEM_SELECTED,
    &mappings::serverbound::play::CHANGE_DIFFICULTY,
    &mappings::serverbound::play::CHANGE_GAME_MODE,
    &mappings::serverbound::play::CHAT,
    &mappings::serverbound::play::CHAT_ACK,
    &mappings::serverbound::play::CHAT_COMMAND,
    &mappings::serverbound::play::CHAT_COMMAND_SIGNED,
    &mappings::serverbound::play::CHAT_PREVIEW,
    &mappings::serverbound::play::CHAT_SESSION_UPDATE,
    &mappings::serverbound::play::CHUNK_BATCH_RECEIVED,
    &mappings::serverbound::play::CLIENT_COMMAND,
    &mappings::serverbound::play::CLIENT_INFORMATION,
    &mappings::serverbound::play::CLIENT_TICK_END,
    &mappings::serverbound::play::COMMAND_SUGGESTION,
    &mappings::serverbound::play::COMMAND_SUGGESTIONS,
    &mappings::serverbound::play::CONFIGURATION_ACKNOWLEDGED,
    &mappings::serverbound::play::CONTAINER_BUTTON_CLICK,
    &mappings::serverbound::play::CONTAINER_CLICK,
    &mappings::serverbound::play::CONTAINER_CLOSE,
    &mappings::serverbound::play::CONTAINER_SLOT_STATE_CHANGED,
    &mappings::serverbound::play::COOKIE_RESPONSE,
    &mappings::serverbound::play::CUSTOM_CLICK_ACTION,
    &mappings::serverbound::play::CUSTOM_PAYLOAD,
    &mappings::serverbound::play::DEBUG_SAMPLE_SUBSCRIPTION,
    &mappings::serverbound::play::DEBUG_SUBSCRIPTION_REQUEST,
    &mappings::serverbound::play::EDIT_BOOK,
    &mappings::serverbound::play::ENTITY_TAG_QUERY,
    &mappings::serverbound::play::INTERACT,
    &mappings::serverbound::play::JIGSAW_GENERATE,
    &mappings::serverbound::play::KEEP_ALIVE,
    &mappings::serverbound::play::LOCK_DIFFICULTY,
    &mappings::serverbound::play::MOVE_PLAYER_POS,
    &mappings::serverbound::play::MOVE_PLAYER_POS_ROT,
    &mappings::serverbound::play::MOVE_PLAYER_ROT,
    &mappings::serverbound::play::MOVE_PLAYER_STATUS_ONLY,
    &mappings::serverbound::play::MOVE_VEHICLE,
    &mappings::serverbound::play::PADDLE_BOAT,
    &mappings::serverbound::play::PICK_ITEM,
    &mappings::serverbound::play::PICK_ITEM_FROM_BLOCK,
    &mappings::serverbound::play::PICK_ITEM_FROM_ENTITY,
    &mappings::serverbound::play::PING_REQUEST,
    &mappings::serverbound::play::PLACE_RECIPE,
    &mappings::serverbound::play::PLAYER_ABILITIES,
    &mappings::serverbound::play::PLAYER_ACTION,
    &mappings::serverbound::play::PLAYER_COMMAND,
    &mappings::serverbound::play::PLAYER_INPUT,
    &mappings::serverbound::play::PLAYER_LOADED,
    &mappings::serverbound::play::PONG,
    &mappings::serverbound::play::PUNCH,
    &mappings::serverbound::play::RECIPE_BOOK_CHANGE_SETTINGS,
    &mappings::serverbound::play::RECIPE_BOOK_DATA,
    &mappings::serverbound::play::RECIPE_BOOK_SEEN_RECIPE,
    &mappings::serverbound::play::RENAME_ITEM,
    &mappings::serverbound::play::RESOURCE_PACK,
    &mappings::serverbound::play::SEEN_ADVANCEMENTS,
    &mappings::serverbound::play::SELECT_TRADE,
    &mappings::serverbound::play::SET_BEACON,
    &mappings::serverbound::play::SET_CARRIED_ITEM,
    &mappings::serverbound::play::SET_COMMAND_BLOCK,
    &mappings::serverbound::play::SET_COMMAND_MINECART,
    &mappings::serverbound::play::SET_CREATIVE_MODE_SLOT,
    &mappings::serverbound::play::SET_GAME_RULE,
    &mappings::serverbound::play::SET_JIGSAW_BLOCK,
    &mappings::serverbound::play::SET_STRUCTURE_BLOCK,
    &mappings::serverbound::play::SET_TEST_BLOCK,
    &mappings::serverbound::play::SIGN_UPDATE,
    &mappings::serverbound::play::SPECTATE_ENTITY,
    &mappings::serverbound::play::SPECTATOR_ACTION,
    &mappings::serverbound::play::STEER_VEHICLE,
    &mappings::serverbound::play::SWING,
    &mappings::serverbound::play::TELEPORT_TO_ENTITY,
    &mappings::serverbound::play::TEST_INSTANCE_BLOCK_ACTION,
    &mappings::serverbound::play::USE_ITEM,
    &mappings::serverbound::play::USE_ITEM_ON,
    &mappings::serverbound::play::WINDOW_CONFIRMATION,
];

pub static CLIENTBOUND_STATUS: &[&PacketId] = &[
    &mappings::clientbound::status::PONG_RESPONSE,
    &mappings::clientbound::status::STATUS_RESPONSE,
];

pub static CLIENTBOUND_LOGIN: &[&PacketId] = &[
    &mappings::clientbound::login::COOKIE_REQUEST,
    &mappings::clientbound::login::CUSTOM_QUERY,
    &mappings::clientbound::login::GAME_PROFILE,
    &mappings::clientbound::login::HELLO,
    &mappings::clientbound::login::LOGIN_COMPRESSION,
    &mappings::clientbound::login::LOGIN_DISCONNECT,
    &mappings::clientbound::login::LOGIN_FINISHED,
];

pub static CLIENTBOUND_CONFIG: &[&PacketId] = &[
    &mappings::clientbound::config::CLEAR_DIALOG,
    &mappings::clientbound::config::CODE_OF_CONDUCT,
    &mappings::clientbound::config::COOKIE_REQUEST,
    &mappings::clientbound::config::CUSTOM_PAYLOAD,
    &mappings::clientbound::config::CUSTOM_REPORT_DETAILS,
    &mappings::clientbound::config::DISCONNECT,
    &mappings::clientbound::config::FINISH_CONFIGURATION,
    &mappings::clientbound::config::KEEP_ALIVE,
    &mappings::clientbound::config::PING,
    &mappings::clientbound::config::POST_EFFECTS,
    &mappings::clientbound::config::REGISTRY_DATA,
    &mappings::clientbound::config::RESET_CHAT,
    &mappings::clientbound::config::RESOURCE_PACK_POP,
    &mappings::clientbound::config::RESOURCE_PACK_PUSH,
    &mappings::clientbound::config::SELECT_KNOWN_PACKS,
    &mappings::clientbound::config::SERVER_LINKS,
    &mappings::clientbound::config::SHOW_DIALOG,
    &mappings::clientbound::config::STORE_COOKIE,
    &mappings::clientbound::config::TRANSFER,
    &mappings::clientbound::config::UPDATE_ENABLED_FEATURES,
    &mappings::clientbound::config::UPDATE_TAGS,
];

pub static CLIENTBOUND_PLAY: &[&PacketId] = &[
    &mappings::clientbound::play::ACKNOWLEDGE_PLAYER_DIGGING,
    &mappings::clientbound::play::ADD_ENTITY,
    &mappings::clientbound::play::ADD_TRANSIENT_BLOCK,
    &mappings::clientbound::play::ANIMATE,
    &mappings::clientbound::play::AWARD_STATS,
    &mappings::clientbound::play::BLOCK_CHANGED_ACK,
    &mappings::clientbound::play::BLOCK_DESTRUCTION,
    &mappings::clientbound::play::BLOCK_ENTITY_DATA,
    &mappings::clientbound::play::BLOCK_EVENT,
    &mappings::clientbound::play::BLOCK_UPDATE,
    &mappings::clientbound::play::BOSS_EVENT,
    &mappings::clientbound::play::BUNDLE_DELIMITER,
    &mappings::clientbound::play::CHANGE_DIFFICULTY,
    &mappings::clientbound::play::CHAT,
    &mappings::clientbound::play::CHAT_PREVIEW_PACKET,
    &mappings::clientbound::play::CHUNKS_BIOMES,
    &mappings::clientbound::play::CHUNK_BATCH_FINISHED,
    &mappings::clientbound::play::CHUNK_BATCH_START,
    &mappings::clientbound::play::CLEAR_DIALOG,
    &mappings::clientbound::play::CLEAR_TITLES,
    &mappings::clientbound::play::COMBAT_EVENT,
    &mappings::clientbound::play::COMMANDS,
    &mappings::clientbound::play::COMMAND_SUGGESTIONS,
    &mappings::clientbound::play::CONTAINER_CLOSE,
    &mappings::clientbound::play::CONTAINER_SET_CONTENT,
    &mappings::clientbound::play::CONTAINER_SET_DATA,
    &mappings::clientbound::play::CONTAINER_SET_SLOT,
    &mappings::clientbound::play::COOKIE_REQUEST,
    &mappings::clientbound::play::COOLDOWN,
    &mappings::clientbound::play::CUSTOM_CHAT_COMPLETIONS,
    &mappings::clientbound::play::CUSTOM_PAYLOAD,
    &mappings::clientbound::play::CUSTOM_REPORT_DETAILS,
    &mappings::clientbound::play::DAMAGE_EVENT,
    &mappings::clientbound::play::DEBUG_BLOCK_VALUE,
    &mappings::clientbound::play::DEBUG_CHUNK_VALUE,
    &mappings::clientbound::play::DEBUG_ENTITY_VALUE,
    &mappings::clientbound::play::DEBUG_EVENT,
    &mappings::clientbound::play::DEBUG_SAMPLE,
    &mappings::clientbound::play::DELETE_CHAT,
    &mappings::clientbound::play::DISCONNECT,
    &mappings::clientbound::play::DISGUISED_CHAT,
    &mappings::clientbound::play::DISPLAY_CHAT_PREVIEW,
    &mappings::clientbound::play::ENTITY_EVENT,
    &mappings::clientbound::play::ENTITY_MOVEMENT,
    &mappings::clientbound::play::ENTITY_POSITION_SYNC,
    &mappings::clientbound::play::EXPLODE,
    &mappings::clientbound::play::FORGET_LEVEL_CHUNK,
    &mappings::clientbound::play::GAME_EVENT,
    &mappings::clientbound::play::GAME_RULE_VALUES,
    &mappings::clientbound::play::GAME_TEST_HIGHLIGHT_POS,
    &mappings::clientbound::play::HURT_ANIMATION,
    &mappings::clientbound::play::INITIALIZE_BORDER,
    &mappings::clientbound::play::KEEP_ALIVE,
    &mappings::clientbound::play::LEVEL_CHUNK_WITH_LIGHT,
    &mappings::clientbound::play::LEVEL_EVENT,
    &mappings::clientbound::play::LEVEL_PARTICLES,
    &mappings::clientbound::play::LIGHT_UPDATE,
    &mappings::clientbound::play::LOGIN,
    &mappings::clientbound::play::LOW_DISK_SPACE_WARNING,
    &mappings::clientbound::play::MAP_CHUNK_BULK,
    &mappings::clientbound::play::MAP_ITEM_DATA,
    &mappings::clientbound::play::MERCHANT_OFFERS,
    &mappings::clientbound::play::MOUNT_SCREEN_OPEN,
    &mappings::clientbound::play::MOVE_ENTITY_POS,
    &mappings::clientbound::play::MOVE_ENTITY_POS_ROT,
    &mappings::clientbound::play::MOVE_ENTITY_ROT,
    &mappings::clientbound::play::MOVE_MINECART_ALONG_TRACK,
    &mappings::clientbound::play::MOVE_PLAYER_ROT,
    &mappings::clientbound::play::MOVE_VEHICLE,
    &mappings::clientbound::play::NAMED_SOUND_EFFECT,
    &mappings::clientbound::play::OPEN_BOOK,
    &mappings::clientbound::play::OPEN_SCREEN,
    &mappings::clientbound::play::OPEN_SIGN_EDITOR,
    &mappings::clientbound::play::PING,
    &mappings::clientbound::play::PLACE_GHOST_RECIPE,
    &mappings::clientbound::play::PLAYER_ABILITIES,
    &mappings::clientbound::play::PLAYER_CHAT,
    &mappings::clientbound::play::PLAYER_CHAT_HEADER,
    &mappings::clientbound::play::PLAYER_COMBAT_END,
    &mappings::clientbound::play::PLAYER_COMBAT_ENTER,
    &mappings::clientbound::play::PLAYER_COMBAT_KILL,
    &mappings::clientbound::play::PLAYER_INFO,
    &mappings::clientbound::play::PLAYER_INFO_REMOVE,
    &mappings::clientbound::play::PLAYER_INFO_UPDATE,
    &mappings::clientbound::play::PLAYER_LOOK_AT,
    &mappings::clientbound::play::PLAYER_POSITION,
    &mappings::clientbound::play::PLAYER_ROTATION,
    &mappings::clientbound::play::PONG_RESPONSE,
    &mappings::clientbound::play::POST_EFFECTS,
    &mappings::clientbound::play::PROJECTILE_POWER,
    &mappings::clientbound::play::RECIPE_BOOK_ADD,
    &mappings::clientbound::play::RECIPE_BOOK_REMOVE,
    &mappings::clientbound::play::RECIPE_BOOK_SETTINGS,
    &mappings::clientbound::play::REMOVE_ENTITIES,
    &mappings::clientbound::play::REMOVE_MOB_EFFECT,
    &mappings::clientbound::play::RESET_SCORE,
    &mappings::clientbound::play::RESOURCE_PACK_POP,
    &mappings::clientbound::play::RESOURCE_PACK_PUSH,
    &mappings::clientbound::play::RESPAWN,
    &mappings::clientbound::play::ROTATE_HEAD,
    &mappings::clientbound::play::SCULK_VIBRATION_SIGNAL,
    &mappings::clientbound::play::SECTION_BLOCKS_UPDATE,
    &mappings::clientbound::play::SELECT_ADVANCEMENTS_TAB,
    &mappings::clientbound::play::SERVER_DATA,
    &mappings::clientbound::play::SERVER_LINKS,
    &mappings::clientbound::play::SET_ACTION_BAR_TEXT,
    &mappings::clientbound::play::SET_BORDER_CENTER,
    &mappings::clientbound::play::SET_BORDER_LERP_SIZE,
    &mappings::clientbound::play::SET_BORDER_SIZE,
    &mappings::clientbound::play::SET_BORDER_WARNING_DELAY,
    &mappings::clientbound::play::SET_BORDER_WARNING_DISTANCE,
    &mappings::clientbound::play::SET_CAMERA,
    &mappings::clientbound::play::SET_CARRIED_ITEM,
    &mappings::clientbound::play::SET_CHUNK_CACHE_CENTER,
    &mappings::clientbound::play::SET_CHUNK_CACHE_RADIUS,
    &mappings::clientbound::play::SET_COMPRESSION,
    &mappings::clientbound::play::SET_CURSOR_ITEM,
    &mappings::clientbound::play::SET_DEFAULT_SPAWN_POSITION,
    &mappings::clientbound::play::SET_DISPLAY_OBJECTIVE,
    &mappings::clientbound::play::SET_ENTITY_DATA,
    &mappings::clientbound::play::SET_ENTITY_LINK,
    &mappings::clientbound::play::SET_ENTITY_MOTION,
    &mappings::clientbound::play::SET_EQUIPMENT,
    &mappings::clientbound::play::SET_EXPERIENCE,
    &mappings::clientbound::play::SET_HEALTH,
    &mappings::clientbound::play::SET_HELD_SLOT,
    &mappings::clientbound::play::SET_OBJECTIVE,
    &mappings::clientbound::play::SET_PASSENGERS,
    &mappings::clientbound::play::SET_PLAYER_INVENTORY,
    &mappings::clientbound::play::SET_PLAYER_TEAM,
    &mappings::clientbound::play::SET_SCORE,
    &mappings::clientbound::play::SET_SIMULATION_DISTANCE,
    &mappings::clientbound::play::SET_SUBTITLE_TEXT,
    &mappings::clientbound::play::SET_TIME,
    &mappings::clientbound::play::SET_TITLES_ANIMATION,
    &mappings::clientbound::play::SET_TITLE_TEXT,
    &mappings::clientbound::play::SHOW_DIALOG,
    &mappings::clientbound::play::SIGN_UPDATE,
    &mappings::clientbound::play::SOUND,
    &mappings::clientbound::play::SOUND_ENTITY,
    &mappings::clientbound::play::SPAWN_EXPERIENCE_ORB,
    &mappings::clientbound::play::SPAWN_LIVING_ENTITY,
    &mappings::clientbound::play::SPAWN_PAINTING,
    &mappings::clientbound::play::SPAWN_PLAYER,
    &mappings::clientbound::play::SPAWN_WEATHER_ENTITY,
    &mappings::clientbound::play::START_CONFIGURATION,
    &mappings::clientbound::play::STOP_SOUND,
    &mappings::clientbound::play::STORE_COOKIE,
    &mappings::clientbound::play::SWING_ANIMATION,
    &mappings::clientbound::play::SYSTEM_CHAT,
    &mappings::clientbound::play::TAB_LIST,
    &mappings::clientbound::play::TAG_QUERY,
    &mappings::clientbound::play::TAKE_ITEM_ENTITY,
    &mappings::clientbound::play::TELEPORT_ENTITY,
    &mappings::clientbound::play::TEST_INSTANCE_BLOCK_STATUS,
    &mappings::clientbound::play::TICKING_STATE,
    &mappings::clientbound::play::TICKING_STEP,
    &mappings::clientbound::play::TITLE,
    &mappings::clientbound::play::TRANSFER,
    &mappings::clientbound::play::UNLOCK_RECIPES,
    &mappings::clientbound::play::UPDATE_ADVANCEMENTS,
    &mappings::clientbound::play::UPDATE_ATTRIBUTES,
    &mappings::clientbound::play::UPDATE_ENABLED_FEATURES,
    &mappings::clientbound::play::UPDATE_ENTITY_NBT,
    &mappings::clientbound::play::UPDATE_MOB_EFFECT,
    &mappings::clientbound::play::UPDATE_RECIPES,
    &mappings::clientbound::play::UPDATE_TAGS,
    &mappings::clientbound::play::USE_BED,
    &mappings::clientbound::play::WAYPOINT,
    &mappings::clientbound::play::WINDOW_CONFIRMATION,
    &mappings::clientbound::play::WORLD_BORDER,
];

pub struct PacketTranslator;

impl PacketTranslator {
    /// Translates an incoming serverbound packet ID from a specific client version into the 26.3 packet ID.
    #[must_use]
    pub fn translate_serverbound_packet_id(
        packet_id: i32,
        version: JavaMinecraftVersion,
        state: ConnectionState,
    ) -> Option<i32> {
        if version == JavaMinecraftVersion::V_26_3 {
            return Some(packet_id);
        }
        match state {
            ConnectionState::Play => {
                for &packet in SERVERBOUND_PLAY {
                    if packet.to_id(version) == packet_id && packet.v26_3 != -1 {
                        return Some(packet.v26_3);
                    }
                }
            }
            ConnectionState::Config => {
                for &packet in SERVERBOUND_CONFIG {
                    if packet.to_id(version) == packet_id && packet.v26_3 != -1 {
                        return Some(packet.v26_3);
                    }
                }
            }
            ConnectionState::Login => {
                for &packet in SERVERBOUND_LOGIN {
                    if packet.to_id(version) == packet_id && packet.v26_3 != -1 {
                        return Some(packet.v26_3);
                    }
                }
            }
            ConnectionState::Status => {
                for &packet in SERVERBOUND_STATUS {
                    if packet.to_id(version) == packet_id && packet.v26_3 != -1 {
                        return Some(packet.v26_3);
                    }
                }
            }
            ConnectionState::Handshake => {
                for &packet in SERVERBOUND_HANDSHAKE {
                    if packet.to_id(version) == packet_id && packet.v26_3 != -1 {
                        return Some(packet.v26_3);
                    }
                }
            }
        }
        None
    }

    /// Translates an outgoing 26.3 clientbound packet ID into the target client version packet ID.
    #[must_use]
    pub fn translate_clientbound_packet_id(
        packet_id_26_3: i32,
        version: JavaMinecraftVersion,
        state: ConnectionState,
    ) -> Option<i32> {
        if version == JavaMinecraftVersion::V_26_3 {
            return Some(packet_id_26_3);
        }
        match state {
            ConnectionState::Play => {
                for &packet in CLIENTBOUND_PLAY {
                    if packet.v26_3 == packet_id_26_3 {
                        let client_id = packet.to_id(version);
                        if client_id != -1 {
                            return Some(client_id);
                        }
                    }
                }
            }
            ConnectionState::Config => {
                for &packet in CLIENTBOUND_CONFIG {
                    if packet.v26_3 == packet_id_26_3 {
                        let client_id = packet.to_id(version);
                        if client_id != -1 {
                            return Some(client_id);
                        }
                    }
                }
            }
            ConnectionState::Login => {
                for &packet in CLIENTBOUND_LOGIN {
                    if packet.v26_3 == packet_id_26_3 {
                        let client_id = packet.to_id(version);
                        if client_id != -1 {
                            return Some(client_id);
                        }
                    }
                }
            }
            ConnectionState::Status => {
                for &packet in CLIENTBOUND_STATUS {
                    if packet.v26_3 == packet_id_26_3 {
                        let client_id = packet.to_id(version);
                        if client_id != -1 {
                            return Some(client_id);
                        }
                    }
                }
            }
            ConnectionState::Handshake => {}
        }
        None
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
    #[must_use]
    pub fn translate_incoming_packet(
        packet_id: i32,
        raw_payload: &[u8],
        version: JavaMinecraftVersion,
        state: ConnectionState,
    ) -> Option<(i32, Vec<u8>)> {
        if version == JavaMinecraftVersion::V_26_3 {
            return None;
        }

        let new_id = Self::translate_serverbound_packet_id(packet_id, version, state)?;
        let translated_payload =
            Self::translate_serverbound_payload(new_id, raw_payload, version, state)
                .unwrap_or_else(|| raw_payload.to_vec());
        Some((new_id, translated_payload))
    }

    fn translate_serverbound_payload(
        new_id: i32,
        mut payload: &[u8],
        version: JavaMinecraftVersion,
        state: ConnectionState,
    ) -> Option<Vec<u8>> {
        use pumpkin_protocol::ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt};

        if state == ConnectionState::Login {
            // HELLO / LOGIN_START (< 1.20.2 missing UUID)
            if new_id == mappings::serverbound::login::HELLO.v26_3
                && version < JavaMinecraftVersion::V_1_20_2
            {
                let name = payload.get_str_borrowed().ok()?;
                let client_uuid = if version >= JavaMinecraftVersion::V_1_19_3 {
                    if let Ok(true) = payload.get_bool() {
                        payload.get_uuid().ok()
                    } else {
                        None
                    }
                } else {
                    None
                };
                let player_uuid = client_uuid.unwrap_or_else(|| {
                    uuid::Uuid::new_v3(
                        &uuid::Uuid::nil(),
                        format!("OfflinePlayer:{name}").as_bytes(),
                    )
                });
                let mut out = Vec::new();
                let _ = out.write_string(name);
                let _ = out.write_uuid(&player_uuid);
                return Some(out);
            }

            // KEY / ENCRYPTION_RESPONSE
            if new_id == mappings::serverbound::login::KEY.v26_3
                && version != JavaMinecraftVersion::V_26_3
            {
                use pumpkin_protocol::{ClientPacket, ServerPacket};
                if let Ok(resp) = pumpkin_protocol::java::server::login::SEncryptionResponse::read(
                    &mut payload,
                    &version,
                ) {
                    let mut out = Vec::new();
                    if resp
                        .write_packet_data(&mut out, &JavaMinecraftVersion::V_26_3)
                        .is_ok()
                    {
                        return Some(out);
                    }
                }
            }
            return None;
        }

        // 1. PUNCH / SWING (26.3 punch has 0 bytes)
        if new_id == mappings::serverbound::play::PUNCH.v26_3 {
            return Some(Vec::new());
        }

        // 2. ACCEPT_TELEPORTATION (< 26.3 only sent teleport_id: VarInt)
        if new_id == mappings::serverbound::play::ACCEPT_TELEPORTATION.v26_3
            && version < JavaMinecraftVersion::V_26_3
        {
            let teleport_id = payload.get_var_int().ok()?;
            let mut out = Vec::new();
            let _ = out.write_var_int(&teleport_id);
            let _ = out.write_f64_be(0.0);
            let _ = out.write_f64_be(0.0);
            let _ = out.write_f64_be(0.0);
            let _ = out.write_f32_be(0.0);
            let _ = out.write_f32_be(0.0);
            return Some(out);
        }

        // 3. USE_ITEM (< 1.21 sequence / yaw / pitch missing)
        if new_id == mappings::serverbound::play::USE_ITEM.v26_3
            && version < JavaMinecraftVersion::V_1_21
        {
            let hand = payload.get_var_int().ok()?;
            let sequence = if version >= JavaMinecraftVersion::V_1_19 {
                payload.get_var_int().unwrap_or(VarInt(0))
            } else {
                VarInt(0)
            };
            let mut out = Vec::new();
            let _ = out.write_var_int(&hand);
            let _ = out.write_var_int(&sequence);
            let _ = out.write_f32_be(0.0);
            let _ = out.write_f32_be(0.0);
            return Some(out);
        }

        // 4. SIGN_UPDATE (< 26.3 is_front_text was bool before lines or missing in < 1.20)
        if new_id == mappings::serverbound::play::SIGN_UPDATE.v26_3
            && version < JavaMinecraftVersion::V_26_3
        {
            let pos_val = payload.get_i64_be().ok()?;
            let is_front = if version >= JavaMinecraftVersion::V_1_20 {
                payload.get_bool().unwrap_or(true)
            } else {
                true
            };
            let line1 = payload.get_str_borrowed().unwrap_or("");
            let line2 = payload.get_str_borrowed().unwrap_or("");
            let line3 = payload.get_str_borrowed().unwrap_or("");
            let line4 = payload.get_str_borrowed().unwrap_or("");
            let mut out = Vec::new();
            let _ = out.write_i64_be(pos_val);
            let _ = out.write_string(line1);
            let _ = out.write_string(line2);
            let _ = out.write_string(line3);
            let _ = out.write_string(line4);
            let _ = out.write_var_int(&VarInt(if is_front { 1 } else { 0 }));
            return Some(out);
        }

        // 5. PLAYER_ACTION (< 26.3 status shifted by 1, < 1.19 sequence missing)
        if new_id == mappings::serverbound::play::PLAYER_ACTION.v26_3
            && version < JavaMinecraftVersion::V_26_3
        {
            let status = if version >= JavaMinecraftVersion::V_1_9 {
                payload.get_var_int().ok()?
            } else {
                VarInt(i32::from(payload.get_u8().ok()?))
            };
            let pos_val = payload.get_i64_be().ok()?;
            let face = payload.get_u8().ok()?;
            let sequence = if version >= JavaMinecraftVersion::V_1_19 {
                payload.get_var_int().unwrap_or(VarInt(0))
            } else {
                VarInt(0)
            };
            let new_status = if status.0 >= 1 {
                VarInt(status.0 + 1)
            } else {
                status
            };
            let mut out = Vec::new();
            let _ = out.write_var_int(&new_status);
            let _ = out.write_i64_be(pos_val);
            let _ = out.write_u8(face);
            let _ = out.write_var_int(&sequence);
            return Some(out);
        }

        // 6. PLAYER_COMMAND (< 1.21.6 action 0/1 were sneak)
        if new_id == mappings::serverbound::play::PLAYER_COMMAND.v26_3
            && version < JavaMinecraftVersion::V_1_21_6
        {
            let entity_id = if version >= JavaMinecraftVersion::V_1_8 {
                payload.get_var_int().ok()?
            } else {
                VarInt(payload.get_i32_be().ok()?)
            };
            let action_id = if version >= JavaMinecraftVersion::V_1_8 {
                payload.get_var_int().ok()?
            } else {
                VarInt(i32::from(payload.get_u8().ok()?))
            };
            let jump_boost = if version >= JavaMinecraftVersion::V_1_8 {
                payload.get_var_int().ok()?
            } else {
                VarInt(payload.get_i32_be().ok()?)
            };
            let modern_action = if action_id.0 >= 2 {
                VarInt(action_id.0 - 2)
            } else {
                return Some(Vec::new());
            };
            let mut out = Vec::new();
            let _ = out.write_var_int(&entity_id);
            let _ = out.write_var_int(&modern_action);
            let _ = out.write_var_int(&jump_boost);
            return Some(out);
        }

        // 7. RESOURCE_PACK response (< 1.20.3 missing UUID)
        if (new_id == mappings::serverbound::play::RESOURCE_PACK.v26_3
            || new_id == mappings::serverbound::config::RESOURCE_PACK.v26_3)
            && version < JavaMinecraftVersion::V_1_20_3
        {
            if version < JavaMinecraftVersion::V_1_10 {
                let _ = payload.get_str_borrowed();
            }
            let result = payload.get_var_int().ok()?;
            let mut out = Vec::new();
            let _ = out.write_uuid(&uuid::Uuid::nil());
            let _ = out.write_var_int(&result);
            return Some(out);
        }

        // 9. CHANGE_DIFFICULTY (< 1.21.6 was u8, now VarInt)
        if new_id == mappings::serverbound::play::CHANGE_DIFFICULTY.v26_3
            && version < JavaMinecraftVersion::V_1_21_6
        {
            let diff = payload.get_u8().ok()?;
            let mut out = Vec::new();
            let _ = out.write_var_int(&VarInt(i32::from(diff)));
            return Some(out);
        }

        // 10. PLAYER_INPUT (< 1.21.2 was floats + bools, now i8 bitmask)
        if new_id == mappings::serverbound::play::PLAYER_INPUT.v26_3
            && version < JavaMinecraftVersion::V_1_21_2
        {
            let sideways = payload.get_f32_be().unwrap_or(0.0);
            let forward = payload.get_f32_be().unwrap_or(0.0);
            let jumping = payload.get_bool().unwrap_or(false);
            let sneaking = payload.get_bool().unwrap_or(false);
            let mut input: i8 = 0;
            if forward > 0.0 {
                input |= 1;
            } else if forward < 0.0 {
                input |= 2;
            }
            if sideways > 0.0 {
                input |= 4;
            } else if sideways < 0.0 {
                input |= 8;
            }
            if jumping {
                input |= 16;
            }
            if sneaking {
                input |= 32;
            }
            let mut out = Vec::new();
            let _ = out.write_i8(input);
            return Some(out);
        }

        // 11. MOVE_VEHICLE (< 1.21.4 missing on_ground bool)
        if new_id == mappings::serverbound::play::MOVE_VEHICLE.v26_3
            && version < JavaMinecraftVersion::V_1_21_4
        {
            let x = payload.get_f64_be().ok()?;
            let y = payload.get_f64_be().ok()?;
            let z = payload.get_f64_be().ok()?;
            let yaw = payload.get_f32_be().ok()?;
            let pitch = payload.get_f32_be().ok()?;
            let mut out = Vec::new();
            let _ = out.write_f64_be(x);
            let _ = out.write_f64_be(y);
            let _ = out.write_f64_be(z);
            let _ = out.write_f32_be(yaw);
            let _ = out.write_f32_be(pitch);
            let _ = out.write_bool(false);
            return Some(out);
        }

        // 12. CONTAINER_BUTTON_CLICK (< 1.21.2 button was i8)
        if new_id == mappings::serverbound::play::CONTAINER_BUTTON_CLICK.v26_3
            && version < JavaMinecraftVersion::V_1_21_2
        {
            let window_id = payload.get_u8().ok()?;
            let button_id = payload.get_i8().ok()?;
            let mut out = Vec::new();
            let _ = out.write_var_int(&VarInt(i32::from(window_id)));
            let _ = out.write_var_int(&VarInt(i32::from(button_id)));
            return Some(out);
        }

        None
    }

    /// Translates an outgoing packet (from 26.3 server to an older client).
    ///
    /// Intercepts entity spawning (`ADD_ENTITY`) and wraps with `CSpawnLivingEntity`
    /// or `CSpawnPainting` when targeting versions before entity unification.
    /// Also remaps packet IDs and payload IDs (sounds, particles, blocks, items, etc.).
    #[must_use]
    pub fn translate_outgoing_packet(
        packet_id: i32,
        raw_payload: &[u8],
        version: JavaMinecraftVersion,
        state: ConnectionState,
    ) -> Option<(i32, Vec<u8>)> {
        if version == JavaMinecraftVersion::V_26_3 {
            return None;
        }

        use pumpkin_protocol::ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt};

        // 1. CLoginSuccess (LOGIN_FINISHED / GAME_PROFILE) in Login state
        if state == ConnectionState::Login
            && packet_id == mappings::clientbound::login::LOGIN_FINISHED.v26_3
        {
            let mut read_slice = raw_payload;
            if let (Ok(uuid), Ok(username), Ok(properties)) = (
                read_slice.get_uuid(),
                read_slice.get_str_borrowed(),
                read_slice.get_list(pumpkin_protocol::Property::read),
            ) {
                let session_id = read_slice.get_uuid().unwrap_or(uuid::Uuid::nil());
                let mut out = Vec::new();
                if version < JavaMinecraftVersion::V_1_16 {
                    let _ = out.write_string(&uuid.to_string());
                } else {
                    let _ = out.write_uuid(&uuid);
                }
                let _ = out.write_string(username);
                if version >= JavaMinecraftVersion::V_1_19 {
                    let _ = out.write_list(&properties, |w, p| p.write(w));
                }
                if version >= JavaMinecraftVersion::V_26_2 {
                    let _ = out.write_uuid(&session_id);
                }
                if version >= JavaMinecraftVersion::V_1_20_5
                    && version < JavaMinecraftVersion::V_1_21_2
                {
                    let _ = out.write_bool(false);
                }
                let target_id = if version >= JavaMinecraftVersion::V_1_21_2 {
                    mappings::clientbound::login::LOGIN_FINISHED.to_id(version)
                } else {
                    mappings::clientbound::login::GAME_PROFILE.to_id(version)
                };
                return Some((target_id, out));
            }
        }

        // 2. CEncryptionRequest (HELLO) in Login state
        if state == ConnectionState::Login && packet_id == mappings::clientbound::login::HELLO.v26_3
        {
            use pumpkin_protocol::{ClientPacket, ServerPacket};
            let mut read_slice = raw_payload;
            if let Ok(req) = pumpkin_protocol::java::client::login::CEncryptionRequest::read(
                &mut read_slice,
                &JavaMinecraftVersion::V_26_3,
            ) {
                let mut out = Vec::new();
                if req.write_packet_data(&mut out, &version).is_ok() {
                    let target_id = mappings::clientbound::login::HELLO.to_id(version);
                    return Some((target_id, out));
                }
            }
        }

        // 3. Check for CSpawnEntity (ADD_ENTITY) in Play state
        if state == ConnectionState::Play
            && packet_id == mappings::clientbound::play::ADD_ENTITY.v26_3
            && let Ok(spawn_entity) =
                CSpawnEntity::read_packet_data(raw_payload, &JavaMinecraftVersion::V_26_3)
        {
            let entity_type_id = spawn_entity.r#type.0 as u16;

            // 1. Check if entity is a Painting in <= 1.18.2
            if version <= JavaMinecraftVersion::V_1_18_2
                && entity_type_id == EntityType::PAINTING.id
            {
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
                    let target_id = CSpawnPainting::to_id(version);
                    return Some((target_id, buf));
                }
            }

            // 2. Check if entity is a living mob in < 1.19
            if version < JavaMinecraftVersion::V_1_19 {
                let is_mob = EntityType::from_raw(entity_type_id).is_some_and(|e| e.mob);
                if is_mob {
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
                        let target_id = CSpawnLivingEntity::to_id(version);
                        return Some((target_id, buf));
                    }
                }
            }

            // 3. Normal entity: remap object/entity type and falling block state
            let remapped_type = if version < JavaMinecraftVersion::V_1_14 {
                VarInt(i32::from(remap_object_type_for_version(
                    entity_type_id,
                    version,
                )))
            } else {
                VarInt(i32::from(
                    remap::entity_id_remap::remap_entity_id_for_version(entity_type_id, version),
                ))
            };

            let remapped_data = if entity_type_id == EntityType::FALLING_BLOCK.id {
                u16::try_from(spawn_entity.data.0).map_or(spawn_entity.data, |state_id| {
                    VarInt(i32::from(remap_block_state_for_version(state_id, version)))
                })
            } else {
                spawn_entity.data
            };

            let modified_spawn = CSpawnEntity {
                entity_id: spawn_entity.entity_id,
                entity_uuid: spawn_entity.entity_uuid,
                r#type: remapped_type,
                position: spawn_entity.position,
                velocity: spawn_entity.velocity,
                pitch: spawn_entity.pitch,
                yaw: spawn_entity.yaw,
                head_yaw: spawn_entity.head_yaw,
                data: remapped_data,
            };

            let mut buf = Vec::new();
            if modified_spawn.write_packet_data(&mut buf, &version).is_ok() {
                let target_id = mappings::clientbound::play::ADD_ENTITY.to_id(version);
                return Some((target_id, buf));
            }
        }

        // Generic packet ID translation
        let client_id = Self::translate_clientbound_packet_id(packet_id, version, state)?;
        Some((client_id, raw_payload.to_vec()))
    }
}
