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
        ConnectionPacketReceivedEventData, ConnectionPacketSentEventData, JavaConnectionFeatures,
        PacketReceivedEventData, PacketSentEventData, RawPacket,
    },
    register_plugin,
};

use crate::packet::mappings;
use crate::packet::translator::{PacketTranslator, from_wasm_java_version, to_wasm_java_version};
use crate::translate::tags;
use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_util::version::JavaMinecraftVersion;

/// What the client's protocol has beyond the packet format.
fn connection_features(version: JavaMinecraftVersion) -> JavaConnectionFeatures {
    [
        (
            version >= JavaMinecraftVersion::V_1_20_2,
            JavaConnectionFeatures::CONFIGURATION_STATE | JavaConnectionFeatures::CHUNK_BATCH_ACKS,
        ),
        (
            version >= JavaMinecraftVersion::V_1_21_4,
            JavaConnectionFeatures::PLAYER_LOADED,
        ),
        (
            (JavaMinecraftVersion::V_1_19_3..JavaMinecraftVersion::V_1_20_2).contains(&version),
            JavaConnectionFeatures::OPTIONAL_VERIFY_TOKEN,
        ),
    ]
    .into_iter()
    .filter(|(has, _)| *has)
    .fold(JavaConnectionFeatures::empty(), |features, (_, flags)| {
        features | flags
    })
}

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

        let versions: Vec<_> = JavaMinecraftVersion::KNOWN
            .iter()
            .filter(|&&version| version != CURRENT_MC_VERSION)
            .map(|&version| to_wasm_java_version(version))
            .collect();
        context.register_java_versions(&versions);

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

        tracing::info!(
            "Pumpkin Java Multi-Version Plugin enabled! Supporting {}-{CURRENT_MC_VERSION}",
            JavaMinecraftVersion::OLDEST
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
            let is_join = event.packet_id == mappings::clientbound::play::LOGIN.current();
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
            if is_join
                && !event.cancelled
                && let Some(payload) = tags::join_tags(version)
                && let Some(packet_id) = PacketTranslator::translate_clientbound_packet_id(
                    mappings::clientbound::play::UPDATE_TAGS.current(),
                    version,
                )
            {
                event.extra_packets.push(RawPacket { packet_id, payload });
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
        event.features = connection_features(version);
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
