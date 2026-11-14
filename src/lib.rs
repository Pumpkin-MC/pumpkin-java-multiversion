pub mod legacy;
pub mod packet;
#[cfg(test)]
mod reference;
pub mod remap;
pub mod tag;
pub mod translate;

use pumpkin_plugin_api::{
    Context, Plugin, PluginMetadata, Server,
    events::{
        EventHandler, EventPriority,
        packet::{
            JavaConnectionPacketReceivedEvent, JavaConnectionPacketSentEvent, PacketReceivedEvent,
            PacketSentEvent,
        },
        player::player_leave::PlayerLeaveEvent,
    },
    events_wit::{
        JavaConnectionPacketReceivedEventData, JavaConnectionPacketSentEventData,
        PacketReceivedEventData, PacketSentEventData, PlayerLeaveEventData, RawPacket,
    },
    player::Player,
    register_plugin,
};

use crate::legacy::ids;
use crate::packet::translator::{PacketTranslator, from_wasm_java_version, to_wasm_java_version};
use crate::translate::{chunk, entity_data, tags};
use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_util::version::JavaMinecraftVersion;

/// Oldest client with chunks, player info and entity data translated.
const OLDEST_SUPPORTED: JavaMinecraftVersion = JavaMinecraftVersion::V_1_7_2;

/// The multi-version plugin allowing Minecraft Java clients across versions (1.7.2 - 26.2)
/// to connect to a Pumpkin 26.3 server.
pub struct MultiVersionPlugin;

impl Plugin for MultiVersionPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "pumpkin-java-multiversion".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Pumpkin Developer".into()],
            description: "Multi-version Java Edition protocol translation plugin for Pumpkin."
                .into(),
            dependencies: vec![],
            permissions: vec![],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        tracing::info!("Loading Pumpkin Java Multi-Version Plugin...");

        let versions: Vec<_> = crate::legacy::KNOWN_VERSIONS
            .iter()
            .filter(|&&version| version >= OLDEST_SUPPORTED && version != CURRENT_MC_VERSION)
            .map(|&version| to_wasm_java_version(version))
            .collect();
        context.register_java_versions(&versions);

        // Register packet event handlers with High priority to translate before/after game logic
        context.register_event_handler(PacketReceivedHandler, EventPriority::Highest, true)?;

        context.register_event_handler(PacketSentHandler, EventPriority::Lowest, true)?;

        // Before play (status / login / config) there is no player, only the connection.
        context.register_event_handler(
            JavaConnectionPacketReceivedHandler,
            EventPriority::Highest,
            true,
        )?;
        context.register_event_handler(
            JavaConnectionPacketSentHandler,
            EventPriority::Lowest,
            true,
        )?;

        context.register_event_handler(PlayerLeaveHandler, EventPriority::Lowest, false)?;

        tracing::info!(
            "Pumpkin Java Multi-Version Plugin enabled! Supporting {OLDEST_SUPPORTED}-{CURRENT_MC_VERSION}"
        );
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        tracing::info!("Unloading Pumpkin Java Multi-Version Plugin");
        Ok(())
    }
}

/// Handles incoming packets from clients and translates them if the client is on an older version.
struct PacketReceivedHandler;

impl EventHandler<PacketReceivedEvent> for PacketReceivedHandler {
    fn handle(
        &self,
        _server: Server,
        mut event: PacketReceivedEventData,
    ) -> PacketReceivedEventData {
        if let Some(java_player) = event.player.as_java() {
            let version = from_wasm_java_version(java_player.get_version());
            if version == CURRENT_MC_VERSION {
                return event;
            }
            // An id with no current counterpart would reach the wrong handler
            match PacketTranslator::translate_incoming_packet(
                event.packet_id,
                &event.raw_payload,
                version,
                || (event.player.get_yaw(), event.player.get_pitch()),
            ) {
                Some((new_id, new_payload)) => {
                    event.packet_id = new_id;
                    event.raw_payload = new_payload;
                }
                None => event.cancelled = true,
            }
        }
        event
    }
}

/// Handles outgoing packets to clients and translates them to match the client's expected version.
struct PacketSentHandler;

impl EventHandler<PacketSentEvent> for PacketSentHandler {
    fn handle(&self, _server: Server, mut event: PacketSentEventData) -> PacketSentEventData {
        if let Some(java_player) = event.player.as_java() {
            let version = from_wasm_java_version(java_player.get_version());
            if version == CURRENT_MC_VERSION {
                return event;
            }
            let is_join = event.packet_id == ids::clientbound::play::LOGIN.current();
            if is_join {
                entity_data::join_viewer(viewer(&event.player));
            }
            // Versions below 1.18 want light as its own packet; chunk::chunk_from_current only
            // produces the chunk half, so keep the original payload to build it separately.
            let chunk_payload = (event.packet_id
                == ids::clientbound::play::LEVEL_CHUNK_WITH_LIGHT.current())
            .then(|| event.raw_payload.clone());
            // A current id means a different packet to the client, so drop what has none
            match entity_data::for_viewer(viewer(&event.player), || {
                PacketTranslator::translate_outgoing_packet(
                    event.packet_id,
                    &event.raw_payload,
                    version,
                )
            }) {
                Some((new_id, new_payload)) => {
                    event.packet_id = new_id;
                    event.raw_payload = new_payload;
                }
                None => event.cancelled = true,
            }
            if is_join
                && !event.cancelled
                && let Some(payload) = tags::join_tags(version)
                && let Some(packet_id) = PacketTranslator::translate_clientbound_packet_id(
                    ids::clientbound::play::UPDATE_TAGS.current(),
                    version,
                )
            {
                event.extra_packets.push(RawPacket { packet_id, payload });
            }
            if !event.cancelled
                && let Some(original) = chunk_payload
                && let Some(light_payload) = chunk::light_from_current(&original, version)
                && let Some(packet_id) = PacketTranslator::translate_clientbound_packet_id(
                    ids::clientbound::play::LIGHT_UPDATE.current(),
                    version,
                )
            {
                event.extra_packets.push(RawPacket {
                    packet_id,
                    payload: light_payload,
                });
            }
        }
        event
    }
}

/// Key of the client's per-connection translation state.
fn viewer(player: &Player) -> u128 {
    let id = player.get_id();
    (u128::from(id.high) << 64) | u128::from(id.low)
}

/// Drops the translation state of a leaving client.
struct PlayerLeaveHandler;

impl EventHandler<PlayerLeaveEvent> for PlayerLeaveHandler {
    fn handle(&self, _server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        entity_data::forget_viewer(viewer(&event.player));
        event
    }
}

/// Translates incoming pre-play packets from older clients to 26.3.
struct JavaConnectionPacketReceivedHandler;

impl EventHandler<JavaConnectionPacketReceivedEvent> for JavaConnectionPacketReceivedHandler {
    fn handle(
        &self,
        _server: Server,
        mut event: JavaConnectionPacketReceivedEventData,
    ) -> JavaConnectionPacketReceivedEventData {
        let version = from_wasm_java_version(event.version);
        if version == CURRENT_MC_VERSION {
            return event;
        }
        match PacketTranslator::translate_connection_incoming(
            event.state,
            event.packet_id,
            &event.raw_payload,
            version,
        ) {
            Some((new_id, new_payload)) => {
                event.packet_id = new_id;
                event.raw_payload = new_payload;
            }
            None => event.cancelled = true,
        }
        event
    }
}

/// Translates outgoing pre-play 26.3 packets to the client's id and payload.
struct JavaConnectionPacketSentHandler;

impl EventHandler<JavaConnectionPacketSentEvent> for JavaConnectionPacketSentHandler {
    fn handle(
        &self,
        _server: Server,
        mut event: JavaConnectionPacketSentEventData,
    ) -> JavaConnectionPacketSentEventData {
        let version = from_wasm_java_version(event.version);
        if version == CURRENT_MC_VERSION {
            return event;
        }
        match PacketTranslator::translate_connection_outgoing(
            event.state,
            event.packet_id,
            &event.raw_payload,
            version,
        ) {
            Some((client_id, payload)) => {
                event.packet_id = client_id;
                event.raw_payload = payload;
            }
            None => event.cancelled = true,
        }
        event
    }
}

register_plugin!(MultiVersionPlugin);
