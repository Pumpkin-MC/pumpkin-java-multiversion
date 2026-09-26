//! UPDATE_ADVANCEMENTS.

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    codec::item_stack_seralizer::ItemStackSerializer,
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use super::item::write_item_for_version;

eras! {
    pub enum AdvancementFormat {
        /// Criteria list, no telemetry flag.
        V1_12 = V_1_7_2,
        /// Telemetry flag added.
        V1_20 = V_1_20,
        /// Criteria list dropped.
        V1_20_2 = V_1_20_2,
        /// Trailing show-advancements flag added.
        V1_21_5 = V_1_21_5,
    }
}

/// Icon is an item template since 26.1, position moved out of the display in 26.3,
/// criteria list dropped in 1.20.2, show-advancements flag added in 1.21.5.
pub fn update_advancements_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let current = CURRENT_MC_VERSION;
    let format = AdvancementFormat::of(version);
    let mut out = Vec::new();
    out.write_bool(payload.get_bool().ok()?).ok()?;

    let added = payload.get_var_int().ok()?;
    out.write_var_int(&added).ok()?;
    for _ in 0..added.0 {
        out.write_string(payload.get_str_borrowed().ok()?).ok()?;
        let has_parent = payload.get_bool().ok()?;
        out.write_bool(has_parent).ok()?;
        if has_parent {
            out.write_string(payload.get_str_borrowed().ok()?).ok()?;
        }

        let has_display = payload.get_bool().ok()?;
        out.write_bool(has_display).ok()?;
        if has_display {
            let title = payload.get_component(&current).ok()?;
            let description = payload.get_component(&current).ok()?;
            out.write_component(&title, &version).ok()?;
            out.write_component(&description, &version).ok()?;
            let icon = ItemStackSerializer::read_template0(&mut payload, &current)
                .ok()?
                .to_stack();
            write_item_for_version(&icon, version, &mut out).ok()?;
            out.write_var_int(&payload.get_var_int().ok()?).ok()?;
            let flags = payload.get_i32_be().ok()?;
            out.write_i32_be(flags).ok()?;
            if flags & 1 != 0 {
                out.write_string(payload.get_str_borrowed().ok()?).ok()?;
            }
        }

        let requirements = payload
            .get_list(|read| read.get_list(|read| read.get_str_borrowed()))
            .ok()?;
        let send_telemetry = payload.get_bool().ok()?;
        let x = payload.get_f32_be().ok()?;
        let y = payload.get_f32_be().ok()?;

        if has_display {
            out.write_f32_be(x).ok()?;
            out.write_f32_be(y).ok()?;
        }
        if format < AdvancementFormat::V1_20_2 {
            let mut criteria: Vec<&str> = requirements.iter().flatten().copied().collect();
            criteria.sort_unstable();
            criteria.dedup();
            out.write_list(&criteria, |write, c| write.write_string(c))
                .ok()?;
        }
        out.write_list(&requirements, |write, req| {
            write.write_list(req, |write, r| write.write_string(r))
        })
        .ok()?;
        if format >= AdvancementFormat::V1_20 {
            out.write_bool(send_telemetry).ok()?;
        }
    }

    // Removed ids and progress are unchanged.
    let rest = if format == AdvancementFormat::V1_21_5 {
        payload
    } else {
        payload.get(..payload.len().checked_sub(1)?)?
    };
    out.extend_from_slice(rest);
    Some(out)
}

#[cfg(test)]
mod tests {
    use pumpkin_data::Advancement;
    use pumpkin_protocol::{ClientPacket, java::client::play::CUpdateAdvancements};

    use super::*;
    use crate::remap::item_id_remap::remap_item_id_for_version;

    #[test]
    fn outgoing_matches_direct_encode() {
        let all = Advancement::get_advancements_list();
        for version in [
            JavaMinecraftVersion::V_1_21_11,
            JavaMinecraftVersion::V_1_21_4,
        ] {
            // Core writes 26.3 item ids, so compare only icons whose id is unchanged.
            let same_icon: Vec<_> = all
                .iter()
                .copied()
                .filter(|adv| {
                    adv.display.is_none_or(|d| {
                        d.item_icon.patch.is_empty()
                            && remap_item_id_for_version(d.item_icon.item.id, version)
                                == d.item_icon.item.id
                    })
                })
                .collect();
            assert!(!same_icon.is_empty());
            let packet = CUpdateAdvancements::new(true, same_icon, Vec::new(), Vec::new(), true);
            let mut payload = Vec::new();
            packet
                .write_packet_data(&mut payload, &CURRENT_MC_VERSION)
                .unwrap();
            let mut expected = Vec::new();
            packet.write_packet_data(&mut expected, &version).unwrap();
            let out = update_advancements_from_current(&payload, version).unwrap();
            assert_eq!(out, expected, "{version:?}");

            let packet = CUpdateAdvancements::new(true, all.to_vec(), Vec::new(), Vec::new(), true);
            let mut payload = Vec::new();
            packet
                .write_packet_data(&mut payload, &CURRENT_MC_VERSION)
                .unwrap();
            assert!(update_advancements_from_current(&payload, version).is_some());
        }
    }
}
