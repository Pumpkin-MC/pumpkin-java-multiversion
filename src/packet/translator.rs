use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_plugin_api::events_wit::ConnectionState;
use pumpkin_protocol::java::client::login::CEncryptionRequest;
use pumpkin_util::version::JavaMinecraftVersion;

use crate::packet::mappings::{self, PacketId};
use crate::remap;
use crate::translate::{
    advancement, animation, attribute, block, chunk, commands, entity, entity_data, explosion,
    game_event, inventory, light, login, movement, particle, player_info, player_spawn,
    plugin_message, recipe, reencode_current, registry, resource_pack, serverbound, sound, tags,
    team, time,
};

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

/// Packets renamed since 26.2, as `(current row, older row)`. The generated table keeps both
/// rows; the older one holds the ids from before the rename.
type Renames = &'static [(&'static PacketId, &'static PacketId)];

static SERVERBOUND_PLAY_RENAMES: Renames = &[
    (
        &mappings::serverbound::play::COMMAND_SUGGESTION,
        &mappings::serverbound::play::COMMAND_SUGGESTIONS,
    ),
    (
        &mappings::serverbound::play::TELEPORT_TO_ENTITY,
        &mappings::serverbound::play::SPECTATE_ENTITY,
    ),
    (
        &mappings::serverbound::play::PUNCH,
        &mappings::serverbound::play::SWING,
    ),
];

static CLIENTBOUND_LOGIN_RENAMES: Renames = &[(
    &mappings::clientbound::login::LOGIN_FINISHED,
    &mappings::clientbound::login::GAME_PROFILE,
)];

static CLIENTBOUND_PLAY_RENAMES: Renames = &[
    (
        &mappings::clientbound::play::PLAYER_CHAT,
        &mappings::clientbound::play::CHAT,
    ),
    (
        &mappings::clientbound::play::PLAYER_ROTATION,
        &mappings::clientbound::play::MOVE_PLAYER_ROT,
    ),
    (
        &mappings::clientbound::play::SET_HELD_SLOT,
        &mappings::clientbound::play::SET_CARRIED_ITEM,
    ),
];

const fn serverbound_table(state: ConnectionState) -> (&'static [&'static PacketId], Renames) {
    match state {
        ConnectionState::Handshake => (SERVERBOUND_HANDSHAKE, &[]),
        ConnectionState::Status => (SERVERBOUND_STATUS, &[]),
        ConnectionState::Login | ConnectionState::Transfer => (SERVERBOUND_LOGIN, &[]),
        ConnectionState::Config => (SERVERBOUND_CONFIG, &[]),
        ConnectionState::Play => (SERVERBOUND_PLAY, SERVERBOUND_PLAY_RENAMES),
    }
}

const fn clientbound_table(state: ConnectionState) -> (&'static [&'static PacketId], Renames) {
    match state {
        ConnectionState::Handshake => (&[], &[]),
        ConnectionState::Status => (CLIENTBOUND_STATUS, &[]),
        ConnectionState::Login | ConnectionState::Transfer => {
            (CLIENTBOUND_LOGIN, CLIENTBOUND_LOGIN_RENAMES)
        }
        ConnectionState::Config => (CLIENTBOUND_CONFIG, &[]),
        ConnectionState::Play => (CLIENTBOUND_PLAY, CLIENTBOUND_PLAY_RENAMES),
    }
}

/// Current id of the client's `client_id` in `state`. Ids are only unique within a state.
fn serverbound_id(
    state: ConnectionState,
    client_id: i32,
    version: JavaMinecraftVersion,
) -> Option<i32> {
    let (table, renames) = serverbound_table(state);
    table
        .iter()
        .find(|packet| packet.to_id(version) == client_id && packet.current() != -1)
        .map(|packet| packet.current())
        .or_else(|| {
            renames
                .iter()
                .find(|(_, older)| older.to_id(version) == client_id)
                .map(|(current, _)| current.current())
        })
}

/// Client id of the current `current_id` in `state`. `None` when the client has no such packet.
fn clientbound_id(
    state: ConnectionState,
    current_id: i32,
    version: JavaMinecraftVersion,
) -> Option<i32> {
    let (table, renames) = clientbound_table(state);
    let packet = table.iter().find(|packet| packet.current() == current_id)?;
    let client_id = match packet.to_id(version) {
        -1 => renames
            .iter()
            .find(|(current, _)| current.current() == current_id)
            .map_or(-1, |(_, older)| older.to_id(version)),
        id => id,
    };
    (client_id != -1).then_some(client_id)
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
        if new_id == mappings::serverbound::play::INTERACT.current() {
            return serverbound::interact_to_current(raw_payload, version);
        }
        // Dropped when unreadable: kept as is, the client's item ids would be stored
        if new_id == mappings::serverbound::play::SET_CREATIVE_MODE_SLOT.current() {
            return inventory::creative_slot_to_current(raw_payload, version)
                .map(|payload| (new_id, payload));
        }
        if new_id == mappings::serverbound::play::CONTAINER_CLICK.current() {
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
                if new_id == mappings::serverbound::login::HELLO.current() =>
            {
                login::hello_to_current(raw_payload, version)
            }
            ConnectionState::Login | ConnectionState::Transfer
                if new_id == mappings::serverbound::login::KEY.current() =>
            {
                login::key_to_current(raw_payload, version)
            }
            ConnectionState::Config
                if new_id == mappings::serverbound::config::RESOURCE_PACK.current() =>
            {
                resource_pack::response_to_current(raw_payload, version)
            }
            ConnectionState::Config
                if new_id == mappings::serverbound::config::CUSTOM_PAYLOAD.current() =>
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
                if current_id == mappings::clientbound::login::HELLO.current() =>
            {
                reencode_current::<CEncryptionRequest>(raw_payload, version)
            }
            ConnectionState::Login | ConnectionState::Transfer
                if current_id == mappings::clientbound::login::LOGIN_FINISHED.current() =>
            {
                login::login_success_from_current(raw_payload, version)
            }
            ConnectionState::Config
                if current_id == mappings::clientbound::config::RESOURCE_PACK_PUSH.current() =>
            {
                resource_pack::push_from_current(raw_payload, version)
            }
            ConnectionState::Config
                if current_id == mappings::clientbound::config::UPDATE_TAGS.current() =>
            {
                tags::update_tags_from_current(raw_payload, version)
            }
            ConnectionState::Config
                if current_id == mappings::clientbound::config::CUSTOM_PAYLOAD.current() =>
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
            && current_id == mappings::clientbound::login::LOGIN_COMPRESSION.current()
        {
            return None;
        }
        if matches!(state, ConnectionState::Config)
            && current_id == mappings::clientbound::config::REGISTRY_DATA.current()
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
        use mappings::clientbound::play;

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
        use mappings::clientbound::play;

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
        use mappings::{clientbound, serverbound};

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
        use mappings::clientbound::play;

        let new = [
            play::ADD_TRANSIENT_BLOCK.current(),
            play::GAME_RULE_VALUES.current(),
            play::LOW_DISK_SPACE_WARNING.current(),
            play::POST_EFFECTS.current(),
            play::SWING_ANIMATION.current(),
        ];
        for packet in CLIENTBOUND_PLAY.iter().filter(|p| p.current() != -1) {
            let id = PacketTranslator::translate_clientbound_packet_id(packet.current(), V1_21_11);
            assert_eq!(id.is_none(), new.contains(&packet.current()), "{packet:?}");
        }
    }

    #[test]
    fn every_1_21_11_serverbound_play_packet_maps() {
        for packet in SERVERBOUND_PLAY.iter().filter(|p| p.to_id(V1_21_11) != -1) {
            let id =
                PacketTranslator::translate_serverbound_packet_id(packet.to_id(V1_21_11), V1_21_11);
            assert!(id.is_some(), "{packet:?}");
        }
    }
}
