pub mod packet;
pub mod remap;
pub mod tag;
pub mod translate;

use pumpkin_plugin_api::{
    Context, Plugin, PluginMetadata, Server,
    events::{
        EventHandler, EventPriority,
        packet::{
            ConnectionPacketReceivedEvent, ConnectionPacketSentEvent, PacketReceivedEvent,
            PacketSentEvent,
        },
    },
    events_wit::{
        ConnectionPacketReceivedEventData, ConnectionPacketSentEventData, PacketReceivedEventData,
        PacketSentEventData,
    },
    register_plugin,
};

use crate::packet::translator::{PacketTranslator, from_wasm_java_version};
use pumpkin_data::packet::CURRENT_MC_VERSION;

/// The multi-version plugin allowing Minecraft Java clients across versions (1.7 - 26.2)
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

        // Register packet event handlers with High priority to translate before/after game logic
        context.register_event_handler(PacketReceivedHandler, EventPriority::Highest, true)?;

        context.register_event_handler(PacketSentHandler, EventPriority::Lowest, true)?;

        // Before play (status / login / config) there is no player, only the connection.
        context.register_event_handler(
            ConnectionPacketReceivedHandler,
            EventPriority::Highest,
            true,
        )?;
        context.register_event_handler(ConnectionPacketSentHandler, EventPriority::Lowest, true)?;

        tracing::info!("Pumpkin Java Multi-Version Plugin enabled! Supporting 1.7.2 - 26.3");
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
            // A current id means a different packet to the client, so drop what has none
            match PacketTranslator::translate_outgoing_packet(
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
        }
        event
    }
}

/// Translates incoming pre-play packets from older clients to 26.3.
struct ConnectionPacketReceivedHandler;

impl EventHandler<ConnectionPacketReceivedEvent> for ConnectionPacketReceivedHandler {
    fn handle(
        &self,
        _server: Server,
        mut event: ConnectionPacketReceivedEventData,
    ) -> ConnectionPacketReceivedEventData {
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
struct ConnectionPacketSentHandler;

impl EventHandler<ConnectionPacketSentEvent> for ConnectionPacketSentHandler {
    fn handle(
        &self,
        _server: Server,
        mut event: ConnectionPacketSentEventData,
    ) -> ConnectionPacketSentEventData {
        let version = from_wasm_java_version(event.version);
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
