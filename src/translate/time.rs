//! SET_TIME.

use pumpkin_protocol::{
    codec::var_long::VarLong, java::client::play::CUpdateTime, ser::NetworkReadExt,
};
use pumpkin_util::version::JavaMinecraftVersion;

/// Since 26.1 a list of clock updates follows the game time; older clients get the first
/// clock as day time. Core's writer handles every older layout.
// TODO: game time syncs carry no clock, so older clients get day 0; keep the last clock per
// connection and send that instead.
pub fn set_time_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let game_time = payload.get_i64_be().ok()?;
    let clock_updates = payload
        .get_list(|read| {
            Ok((
                read.get_var_int()?.0,
                read.get_var_long().map(|VarLong(ticks)| ticks)?,
                read.get_f32_be()?,
                read.get_f32_be()?,
            ))
        })
        .ok()?;
    super::legacy_bytes(
        &CUpdateTime {
            game_time,
            clock_updates,
        },
        version,
    )
}

#[cfg(test)]
mod tests {

    use pumpkin_protocol::ClientPacket;
    use pumpkin_protocol::java::legacy::LegacyWrite;

    use super::*;

    #[test]
    fn set_time_matches_direct_encode() {
        for packet in [
            CUpdateTime::new_clock(1200, 0, 6000, 0.25, 1.0),
            CUpdateTime::new_clock(1200, 0, 6000, 0.0, 0.0),
        ] {
            let mut current = Vec::new();
            packet.write_packet_data(&mut current).unwrap();
            for version in [
                JavaMinecraftVersion::V_1_21_11,
                JavaMinecraftVersion::V_1_20,
            ] {
                let mut expected = Vec::new();
                packet.write_legacy(&mut expected, &version).unwrap();
                assert_eq!(set_time_from_current(&current, version), Some(expected));
            }
        }
    }
}
