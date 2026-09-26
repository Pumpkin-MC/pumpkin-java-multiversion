//! Entity animations: ANIMATE and SWING_ANIMATION.

use pumpkin_protocol::{
    ClientPacket, MultiVersionJavaPacket, VarInt,
    java::client::play::CSwingArm,
    ser::{NetworkReadExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

/// ANIMATE: 26.3 renumbered the animations left after moving the swings out.
pub fn animate_from_current(mut payload: &[u8], _version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let entity_id = payload.get_var_int().ok()?;
    let animation = match payload.get_u8().ok()? {
        0 => 2, // Leave bed
        1 => 4, // Critical effect
        2 => 5, // Magic critical effect
        other => other,
    };
    let mut out = Vec::new();
    out.write_var_int(&entity_id).ok()?;
    out.write_u8(animation).ok()?;
    Some(out)
}

/// SWING_ANIMATION (26.3), as `(client packet id, payload)`: an ANIMATE swing for older clients.
pub fn swing_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<(i32, Vec<u8>)> {
    let entity_id = payload.get_var_int().ok()?;
    let off_hand = payload.get_var_int().ok()? != VarInt(0);
    let mut out = Vec::new();
    CSwingArm::new(entity_id, off_hand)
        .write_packet_data(&mut out, &version)
        .ok()?;
    Some((CSwingArm::to_id(version), out))
}

#[cfg(test)]
mod tests {
    use pumpkin_data::packet::CURRENT_MC_VERSION;
    use pumpkin_protocol::java::client::play::{Animation, CEntityAnimation};

    use super::*;

    fn write(packet: &impl ClientPacket, version: JavaMinecraftVersion) -> Vec<u8> {
        let mut out = Vec::new();
        packet.write_packet_data(&mut out, &version).unwrap();
        out
    }
}
