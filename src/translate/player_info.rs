//! PLAYER_INFO_UPDATE.

use pumpkin_protocol::java::legacy::LegacyWrite;
use pumpkin_protocol::{
    Property,
    java::client::play::{CPlayerInfoUpdate, InitChat, Player, PlayerAction, PlayerInfoFlags},
    ser::{NetworkReadExt, NetworkReadSliceExt, ReadingError},
};
use pumpkin_util::{text::TextComponent, version::JavaMinecraftVersion};

eras! {
    pub enum PlayerInfoFormat {
        // TODO: `PLAYER_INFO` before 1.19.3 has another layout; dropped until then
        PlayerInfo = V_1_7_2,
        /// Action bit set, which core's writer masks per version.
        V1_19_3 = V_1_19_3,
    }
}

/// One player's actions, owned so the borrowed `PlayerAction`s can point into it.
struct Entry {
    uuid: uuid::Uuid,
    add: Option<(Box<str>, Vec<Property>)>,
    chat: Option<Option<InitChat>>,
    game_mode: Option<i32>,
    listed: Option<bool>,
    latency: Option<i32>,
    display_name: Option<Option<TextComponent>>,
    list_order: Option<i32>,
    hat: Option<bool>,
}

fn read_entry(read: &mut &[u8], flags: PlayerInfoFlags) -> Option<Entry> {
    let uuid = read.get_uuid().ok()?;
    let add = if flags.contains(PlayerInfoFlags::ADD_PLAYER) {
        let name = read.get_str().ok()?;
        let properties = read
            .get_list(|read| {
                Ok(Property {
                    name: read.get_str()?,
                    value: read.get_str()?,
                    signature: read.get_option(|read| read.get_str())?,
                })
            })
            .ok()?;
        Some((name, properties))
    } else {
        None
    };
    let chat = if flags.contains(PlayerInfoFlags::INITIALIZE_CHAT) {
        Some(
            read.get_option(|read| {
                let session_id = read.get_uuid()?;
                let expires_at = read.get_i64_be()?;
                let key_len = read.get_var_int()?.0 as usize;
                let public_key = read.read_slice_borrowed(key_len)?.into();
                let signature_len = read.get_var_int()?.0 as usize;
                let signature = read.read_slice_borrowed(signature_len)?.into();
                Ok(InitChat {
                    session_id,
                    expires_at,
                    public_key,
                    signature,
                })
            })
            .ok()?,
        )
    } else {
        None
    };
    let game_mode = flags
        .contains(PlayerInfoFlags::UPDATE_GAME_MODE)
        .then(|| read.get_var_int().map(|v| v.0))
        .transpose()
        .ok()?;
    let listed = flags
        .contains(PlayerInfoFlags::UPDATE_LISTED)
        .then(|| read.get_bool())
        .transpose()
        .ok()?;
    let latency = flags
        .contains(PlayerInfoFlags::UPDATE_LATENCY)
        .then(|| read.get_var_int().map(|v| v.0))
        .transpose()
        .ok()?;
    let display_name = flags
        .contains(PlayerInfoFlags::UPDATE_DISPLAY_NAME)
        .then(|| read.get_option(|read| read.get_component()))
        .transpose()
        .ok()?;
    let list_order = flags
        .contains(PlayerInfoFlags::UPDATE_LIST_PRIORITY)
        .then(|| read.get_var_int().map(|v| v.0))
        .transpose()
        .ok()?;
    let hat = flags
        .contains(PlayerInfoFlags::UPDATE_HAT)
        .then(|| read.get_bool())
        .transpose()
        .ok()?;
    Some(Entry {
        uuid,
        add,
        chat,
        game_mode,
        listed,
        latency,
        display_name,
        list_order,
        hat,
    })
}

/// The actions present in `entry` ->  the writer orders them by flag.
fn actions(entry: &Entry) -> Vec<PlayerAction<'_>> {
    let mut actions = Vec::new();
    if let Some((name, properties)) = &entry.add {
        actions.push(PlayerAction::AddPlayer { name, properties });
    }
    if let Some(chat) = &entry.chat {
        actions.push(PlayerAction::InitializeChat(chat.as_ref().map(|chat| {
            InitChat {
                session_id: chat.session_id,
                expires_at: chat.expires_at,
                public_key: chat.public_key.clone(),
                signature: chat.signature.clone(),
            }
        })));
    }
    if let Some(game_mode) = entry.game_mode {
        actions.push(PlayerAction::UpdateGameMode(game_mode.into()));
    }
    if let Some(listed) = entry.listed {
        actions.push(PlayerAction::UpdateListed(listed));
    }
    if let Some(latency) = entry.latency {
        actions.push(PlayerAction::UpdateLatency(latency.into()));
    }
    if let Some(display_name) = &entry.display_name {
        actions.push(PlayerAction::UpdateDisplayName(display_name.as_ref()));
    }
    if let Some(order) = entry.list_order {
        actions.push(PlayerAction::UpdateListOrder(order.into()));
    }
    if let Some(hat) = entry.hat {
        actions.push(PlayerAction::UpdateHat(hat));
    }
    actions
}

/// Drops the list order before 1.21.2 and the hat before 1.21.4, display names as JSON
/// before 1.20.3 (ViaBackwards `Protocol1_21_4To1_21_2` and earlier).
pub fn player_info_update_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    if PlayerInfoFormat::of(version) == PlayerInfoFormat::PlayerInfo {
        return None;
    }
    let bits = payload.get_u8().ok()?;
    let flags = PlayerInfoFlags::from_bits_retain(bits);
    let entries = payload
        .get_list(|read| {
            read_entry(read, flags).ok_or_else(|| ReadingError::Message("player".into()))
        })
        .ok()?;
    let actions: Vec<_> = entries.iter().map(actions).collect();
    let players: Vec<_> = entries
        .iter()
        .zip(&actions)
        .map(|(entry, actions)| Player {
            uuid: entry.uuid,
            actions,
        })
        .collect();
    let mut out = Vec::new();
    CPlayerInfoUpdate::new(bits, &players)
        .write_legacy(&mut out, &version)
        .ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pumpkin_data::packet::CURRENT_MC_VERSION;
    use pumpkin_protocol::java::legacy::LegacyWrite;

    fn packet(version: JavaMinecraftVersion) -> Vec<u8> {
        let properties = [Property {
            name: "textures".into(),
            value: "abc".into(),
            signature: None,
        }];
        let display_name = TextComponent::text("Steve");
        let actions = [
            PlayerAction::AddPlayer {
                name: "Steve",
                properties: &properties,
            },
            PlayerAction::UpdateGameMode(1.into()),
            PlayerAction::UpdateListed(true),
            PlayerAction::UpdateLatency(20.into()),
            PlayerAction::UpdateDisplayName(Some(&display_name)),
            PlayerAction::UpdateListOrder(3.into()),
            PlayerAction::UpdateHat(true),
        ];
        let players = [Player {
            uuid: uuid::Uuid::from_u128(7),
            actions: &actions,
        }];
        let bits = (PlayerInfoFlags::all() - PlayerInfoFlags::INITIALIZE_CHAT).bits();
        let mut out = Vec::new();
        CPlayerInfoUpdate::new(bits, &players)
            .write_legacy(&mut out, &version)
            .unwrap();
        out
    }

    #[test]
    fn older_clients_get_their_own_layout() {
        let current = packet(CURRENT_MC_VERSION);
        for version in [
            JavaMinecraftVersion::V_1_19_3,
            JavaMinecraftVersion::V_1_20_3,
            JavaMinecraftVersion::V_1_21_2,
            JavaMinecraftVersion::V_1_21_4,
        ] {
            assert_eq!(
                player_info_update_from_current(&current, version),
                Some(packet(version)),
                "{version}"
            );
        }
        assert_ne!(
            packet(JavaMinecraftVersion::V_1_21_2),
            current,
            "hat is dropped"
        );
    }
}
