//! Clientbound text components, chat, scoreboard, and scalar layout shifts.

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    ClientPacket, ServerPacket, VarInt,
    java::client::play::{
        CChangeDifficulty, CDisplayObjective, CHeadRot, CKeepAlive, COpenSignEditor,
        CRemoveMobEffect, CSetEntityLink, CSetExperience, CSetHealth, CSetSelectedSlot,
        CTakeItemEntity, CUnloadChunk, CUpdateMobEffect,
    },
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

use crate::legacy::{LegacyWriteExt, ids};

fn reencode<'a, P: ServerPacket<'a> + ClientPacket>(
    mut payload: &'a [u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let packet = P::read(&mut payload, &CURRENT_MC_VERSION).ok()?;
    let mut out = Vec::new();
    packet.write_packet_data(&mut out, &version).ok()?;
    Some(out)
}

fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Single `TextComponent` packet (`DISCONNECT`, `SET_ACTION_BAR_TEXT`, `SET_TITLE_TEXT`, `SET_SUBTITLE_TEXT`).
pub fn component_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let comp = payload.get_component(&CURRENT_MC_VERSION).ok()?;
    let mut out = Vec::new();
    out.write_component_legacy(&comp, &version).ok()?;
    Some(out)
}

/// `TAB_LIST`: header and footer `TextComponent`s.
pub fn tab_list_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let header = payload.get_component(&CURRENT_MC_VERSION).ok()?;
    let footer = payload.get_component(&CURRENT_MC_VERSION).ok()?;
    let mut out = Vec::new();
    out.write_component_legacy(&header, &version).ok()?;
    out.write_component_legacy(&footer, &version).ok()?;
    Some(out)
}

/// `SYSTEM_CHAT` (1.19+) / `CHAT` (1.7 - 1.18.2).
pub fn system_chat_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<(i32, Vec<u8>)> {
    let content = payload.get_component(&CURRENT_MC_VERSION).ok()?;
    let overlay = payload.get_bool().ok()?;

    if version <= JavaMinecraftVersion::V_1_7_6 && overlay {
        return None;
    }

    let packet_id = if version >= JavaMinecraftVersion::V_1_19 {
        ids::clientbound::play::SYSTEM_CHAT.to_id(version)
    } else {
        ids::clientbound::play::CHAT.to_id(version)
    };
    if packet_id == -1 {
        return None;
    }

    let mut out = Vec::new();
    out.write_component_legacy(&content, &version).ok()?;
    if version >= JavaMinecraftVersion::V_1_19_1 {
        out.write_bool(overlay).ok()?;
    } else if version >= JavaMinecraftVersion::V_1_19 {
        out.write_var_int(&VarInt(if overlay { 2 } else { 1 }))
            .ok()?;
    } else if version >= JavaMinecraftVersion::V_1_16 {
        out.write_u8(if overlay { 2 } else { 1 }).ok()?;
        out.write_uuid(&uuid::Uuid::nil()).ok()?;
    } else if version >= JavaMinecraftVersion::V_1_8 {
        out.write_u8(if overlay { 2 } else { 1 }).ok()?;
    }
    Some((packet_id, out))
}

/// `SERVER_DATA` (1.19+).
pub fn server_data_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let motd = payload.get_component(&CURRENT_MC_VERSION).ok()?;
    let _has_icon = payload.get_bool().ok()?;
    let mut out = Vec::new();
    if version >= JavaMinecraftVersion::V_1_20_5 {
        out.write_component_legacy(&motd, &version).ok()?;
        out.write_bool(false).ok()?;
    } else if version >= JavaMinecraftVersion::V_1_19_4 {
        out.write_component_legacy(&motd, &version).ok()?;
        out.write_bool(false).ok()?;
        out.write_bool(false).ok()?;
    } else {
        out.write_bool(true).ok()?;
        out.write_component_legacy(&motd, &version).ok()?;
        out.write_bool(false).ok()?;
        if version < JavaMinecraftVersion::V_1_19_3 {
            out.write_bool(false).ok()?;
        }
        if version >= JavaMinecraftVersion::V_1_19_1 {
            out.write_bool(false).ok()?;
        }
    }
    Some(out)
}

pub fn change_difficulty_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    reencode::<CChangeDifficulty>(payload, version)
}

pub fn keep_alive_from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    reencode::<CKeepAlive>(payload, version)
}

pub fn set_held_slot_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let slot = payload.get_var_int().ok()?.0 as i8;
    let mut out = Vec::new();
    CSetSelectedSlot::new(slot)
        .write_packet_data(&mut out, &version)
        .ok()?;
    Some(out)
}

pub fn forget_level_chunk_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    reencode::<CUnloadChunk>(payload, version)
}

pub fn set_health_from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    reencode::<CSetHealth>(payload, version)
}

pub fn set_experience_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    reencode::<CSetExperience>(payload, version)
}

pub fn rotate_head_from_current(payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    reencode::<CHeadRot>(payload, version)
}

pub fn take_item_entity_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    reencode::<CTakeItemEntity>(payload, version)
}

pub fn set_entity_link_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    reencode::<CSetEntityLink>(payload, version)
}

pub fn display_objective_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    reencode::<CDisplayObjective>(payload, version)
}

pub fn open_sign_editor_from_current(
    payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    reencode::<COpenSignEditor>(payload, version)
}

pub fn set_objective_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let name = payload.get_str_borrowed().ok()?;
    let mode = payload.get_u8().ok()?;
    let mut out = Vec::new();
    let max_name = if version >= JavaMinecraftVersion::V_1_18 {
        32767
    } else {
        16
    };
    out.write_string(truncate_chars(name, max_name)).ok()?;
    if version <= JavaMinecraftVersion::V_1_7_6 {
        // 1.7 writes value before mode byte!
        let display = if mode == 0 || mode == 2 {
            payload
                .get_component(&CURRENT_MC_VERSION)
                .ok()?
                .to_pretty_console()
        } else {
            String::new()
        };
        out.write_string(truncate_chars(&display, 32)).ok()?;
        out.write_u8(mode).ok()?;
        return Some(out);
    }
    out.write_u8(mode).ok()?;
    if mode == 0 || mode == 2 {
        let display = payload.get_component(&CURRENT_MC_VERSION).ok()?;
        let render_type = payload.get_var_int().ok()?;
        if version < JavaMinecraftVersion::V_1_13 {
            let text = display.to_pretty_console();
            out.write_string(truncate_chars(&text, 32)).ok()?;
            let render_str = if render_type.0 == 1 {
                "hearts"
            } else {
                "integer"
            };
            out.write_string(render_str).ok()?;
        } else {
            out.write_component_legacy(&display, &version).ok()?;
            out.write_var_int(&render_type).ok()?;
            if version >= JavaMinecraftVersion::V_1_20_3 {
                out.write_bool(false).ok()?;
            }
        }
    }
    Some(out)
}

pub fn set_score_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let entity_name = payload.get_str_borrowed().ok()?;
    let objective_name = payload.get_str_borrowed().ok()?;
    let value = payload.get_var_int().ok()?;
    let display_name = payload
        .get_option(|r| r.get_component(&CURRENT_MC_VERSION))
        .ok()?;
    let mut out = Vec::new();
    if version >= JavaMinecraftVersion::V_1_20_3 {
        out.write_string(entity_name).ok()?;
        out.write_string(objective_name).ok()?;
        out.write_var_int(&value).ok()?;
        out.write_option(&display_name, |w, comp| {
            w.write_component_legacy(comp, &version)
        })
        .ok()?;
        out.write_bool(false).ok()?;
    } else if version <= JavaMinecraftVersion::V_1_7_6 {
        out.write_string(truncate_chars(entity_name, 16)).ok()?;
        out.write_u8(0).ok()?;
        out.write_string(truncate_chars(objective_name, 16)).ok()?;
        out.write_i32_be(value.0).ok()?;
    } else {
        let max_ent = if version >= JavaMinecraftVersion::V_1_18 {
            32767
        } else {
            40
        };
        let max_obj = if version >= JavaMinecraftVersion::V_1_18 {
            32767
        } else {
            16
        };
        out.write_string(truncate_chars(entity_name, max_ent))
            .ok()?;
        out.write_u8(0).ok()?;
        out.write_string(truncate_chars(objective_name, max_obj))
            .ok()?;
        out.write_var_int(&value).ok()?;
    }
    Some(out)
}

pub fn update_mob_effect_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let entity_id = payload.get_var_int().ok()?;
    let effect_id = payload.get_var_int().ok()?;
    let amplifier = payload.get_var_int().ok()?;
    let duration = payload.get_var_int().ok()?;
    let flags = payload.get_i8().ok()?;
    let mut out = Vec::new();
    CUpdateMobEffect::new(entity_id, effect_id, amplifier, duration, flags)
        .write_packet_data(&mut out, &version)
        .ok()?;
    Some(out)
}

pub fn remove_mob_effect_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let entity_id = payload.get_var_int().ok()?;
    let effect_id = payload.get_var_int().ok()?;
    let mut out = Vec::new();
    CRemoveMobEffect::new(entity_id, effect_id)
        .write_packet_data(&mut out, &version)
        .ok()?;
    Some(out)
}

pub fn block_event_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let location = payload.get_block_pos(&CURRENT_MC_VERSION).ok()?;
    if version < JavaMinecraftVersion::V_1_18 && !(0..256).contains(&location.0.y) {
        return None;
    }
    let action_id = payload.get_u8().ok()?;
    let action_parameter = payload.get_u8().ok()?;
    let block_type = payload.get_var_int().ok()?;
    let mut out = Vec::new();
    if version <= JavaMinecraftVersion::V_1_7_6 {
        out.write_i32_be(location.0.x).ok()?;
        out.write_i16_be(location.0.y as i16).ok()?;
        out.write_i32_be(location.0.z).ok()?;
    } else {
        out.write_block_pos_legacy(&location, &version).ok()?;
    }
    out.write_u8(action_id).ok()?;
    out.write_u8(action_parameter).ok()?;
    out.write_var_int(&block_type).ok()?;
    Some(out)
}

pub fn block_destruction_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let entity_id = payload.get_var_int().ok()?;
    let location = payload.get_block_pos(&CURRENT_MC_VERSION).ok()?;
    let stage = payload.get_i8().ok()?;
    let mut out = Vec::new();
    out.write_var_int(&entity_id).ok()?;
    if version <= JavaMinecraftVersion::V_1_7_6 {
        out.write_i32_be(location.0.x).ok()?;
        out.write_i32_be(location.0.y).ok()?;
        out.write_i32_be(location.0.z).ok()?;
    } else {
        out.write_block_pos_legacy(&location, &version).ok()?;
    }
    out.write_i8(stage).ok()?;
    Some(out)
}

pub fn level_event_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let event = payload.get_i32_be().ok()?;
    let location = payload.get_block_pos(&CURRENT_MC_VERSION).ok()?;
    let mut data = payload.get_i32_be().ok()?;
    let disable_relative_volume = payload.get_bool().ok()?;
    if event == 2001 && data >= 0 {
        let remapped = super::block::remap_state(data as u32, version);
        data = if version <= JavaMinecraftVersion::V_1_7_6 {
            ((remapped >> 4) | ((remapped & 0xF) << 12)) as i32
        } else {
            remapped as i32
        };
    }
    let mut out = Vec::new();
    out.write_i32_be(event).ok()?;
    if version <= JavaMinecraftVersion::V_1_7_6 {
        if !(0..256).contains(&location.0.y) {
            return None;
        }
        out.write_i32_be(location.0.x).ok()?;
        out.write_u8(location.0.y as u8).ok()?;
        out.write_i32_be(location.0.z).ok()?;
    } else {
        out.write_block_pos_legacy(&location, &version).ok()?;
    }
    out.write_i32_be(data).ok()?;
    out.write_bool(disable_relative_volume).ok()?;
    Some(out)
}
