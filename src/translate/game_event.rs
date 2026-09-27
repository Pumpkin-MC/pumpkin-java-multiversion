//! GAME_EVENT.

use pumpkin_protocol::java::client::play::GameEvent;
use pumpkin_util::version::JavaMinecraftVersion;

eras! {
    pub enum GameEventSet {
        /// No `START_WAITING_CHUNKS`.
        V1_7 = V_1_7_2,
        /// Same as 26.3.
        V1_20_3 = V_1_20_3,
    }
}

/// Events the client does not know are dropped. the rest pass unchanged.
pub fn game_event_from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let event = *payload.first()?;
    if GameEventSet::of(version) == GameEventSet::V1_7
        && event == GameEvent::StartWaitingChunks as u8
    {
        return None;
    }
    Some(payload.to_vec())
}

#[cfg(test)]
mod tests {
    use pumpkin_data::packet::CURRENT_MC_VERSION;
    use pumpkin_protocol::{ClientPacket, java::client::play::CGameEvent};

    use super::*;

    fn payload(event: GameEvent) -> Vec<u8> {
        let mut out = Vec::new();
        CGameEvent::new(event, 0.0)
            .write_packet_data(&mut out, &CURRENT_MC_VERSION)
            .unwrap();
        out
    }

    #[test]
    fn start_waiting_chunks_only_reaches_1_20_3_and_later() {
        let waiting = payload(GameEvent::StartWaitingChunks);
        assert_eq!(
            game_event_from_current(&waiting, JavaMinecraftVersion::V_1_20_2),
            None
        );
        assert_eq!(
            game_event_from_current(&waiting, JavaMinecraftVersion::V_1_20_3),
            Some(waiting.clone())
        );

        let rain = payload(GameEvent::BeginRaining);
        assert_eq!(
            game_event_from_current(&rain, JavaMinecraftVersion::V_1_20),
            Some(rain.clone())
        );
    }
}
