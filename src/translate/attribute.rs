//! Entity attributes: UPDATE_ATTRIBUTES.

use pumpkin_data::attributes::Attributes;
use pumpkin_protocol::{ServerPacket, VarInt, java::client::play::CUpdateAttributes};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::remap::attribute_id_remap::remap_attribute_id_for_version;

eras! {
    pub enum AttributeFormat {
        /// Attributes by name, which core writes from the 26.3 id.
        Names = V_1_7_2,
        /// Attributes by registry id.
        Ids = V_1_20_5,
    }
}

/// The client's attribute id, `None` when the client lacks the attribute.
fn client_attribute(id: VarInt, version: JavaMinecraftVersion) -> Option<VarInt> {
    if AttributeFormat::of(version) == AttributeFormat::Names {
        return Some(id);
    }
    let mapped = remap_attribute_id_for_version(id.0 as u32, version);
    // Missing attributes map to 0, which is also armor
    (mapped != 0 || id.0 == i32::from(Attributes::ARMOR.id)).then_some(VarInt(mapped as i32))
}

/// UPDATE_ATTRIBUTES: the client's attribute ids, dropping attributes it lacks.
pub fn update_attributes_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let mut packet = CUpdateAttributes::read(&mut payload).ok()?;
    packet.properties.retain_mut(|property| {
        client_attribute(property.id, version).is_some_and(|id| {
            property.id = id;
            true
        })
    });
    super::legacy_bytes(&packet, version)
}

#[cfg(test)]
mod tests {
    use pumpkin_protocol::ClientPacket;
    use pumpkin_protocol::java::client::play::Property;
    use pumpkin_protocol::java::legacy::LegacyWrite;

    use super::*;

    #[test]
    fn attributes_get_client_ids() {
        let packet = CUpdateAttributes::new(
            VarInt(1),
            vec![
                Property::new(
                    VarInt(i32::from(Attributes::SAFE_FALL_DISTANCE.id)),
                    3.0,
                    vec![],
                ),
                Property::new(
                    VarInt(i32::from(Attributes::FRICTION_MODIFIER.id)),
                    1.0,
                    vec![],
                ),
                Property::new(VarInt(i32::from(Attributes::ARMOR.id)), 2.0, vec![]),
            ],
        );
        let mut current = Vec::new();
        packet.write_packet_data(&mut current).unwrap();

        let version = JavaMinecraftVersion::V_1_21_11;
        // 1.21.11 has no friction modifier
        let expected = CUpdateAttributes::new(
            VarInt(1),
            vec![
                Property::new(VarInt(24), 3.0, vec![]),
                Property::new(VarInt(0), 2.0, vec![]),
            ],
        );
        let mut expected_bytes = Vec::new();
        expected
            .write_legacy(&mut expected_bytes, &version)
            .unwrap();
        assert_eq!(
            update_attributes_from_current(&current, version),
            Some(expected_bytes)
        );
    }
}
