//! SET_PLAYER_TEAM.

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    ClientPacket,
    java::client::play::{CSetPlayerTeam, TeamMethod, TeamParameters},
    ser::{NetworkReadExt, NetworkReadSliceExt},
};
use pumpkin_util::version::JavaMinecraftVersion;

const fn method(id: i8) -> Option<TeamMethod> {
    Some(match id {
        0 => TeamMethod::Create,
        1 => TeamMethod::Remove,
        2 => TeamMethod::Update,
        3 => TeamMethod::AddPlayers,
        4 => TeamMethod::RemovePlayers,
        _ => return None,
    })
}

const fn nametag_visibility(id: i32) -> &'static str {
    match id {
        1 => "never",
        2 => "hideForOtherTeams",
        3 => "hideForOwnTeam",
        _ => "always",
    }
}

const fn collision_rule(id: i32) -> &'static str {
    match id {
        1 => "never",
        2 => "pushOtherTeams",
        3 => "pushOwnTeam",
        _ => "always",
    }
}

/// Parameters since 26.2: display name, prefix, suffix, visibility and collision ids,
/// optional color, options. Core's writer handles every older layout.
pub fn set_player_team_from_current(
    mut payload: &[u8],
    version: JavaMinecraftVersion,
) -> Option<Vec<u8>> {
    let current = CURRENT_MC_VERSION;
    let team_name = payload.get_str_borrowed().ok()?.to_string();
    let method = method(payload.get_i8().ok()?)?;

    let has_parameters = matches!(method, TeamMethod::Create | TeamMethod::Update);
    let components = if has_parameters {
        let display_name = payload.get_component(&current).ok()?;
        let prefix = payload.get_component(&current).ok()?;
        let suffix = payload.get_component(&current).ok()?;
        Some((display_name, prefix, suffix))
    } else {
        None
    };
    let rest = if has_parameters {
        let visibility = payload.get_var_int().ok()?.0;
        let collision = payload.get_var_int().ok()?.0;
        // No color is written as reset (-1) for older clients
        let color = if payload.get_bool().ok()? {
            payload.get_var_int().ok()?.0
        } else {
            -1
        };
        let options = payload.get_i8().ok()?;
        Some((visibility, collision, color, options))
    } else {
        None
    };
    let parameters = components.as_ref().zip(rest).map(
        |((display_name, prefix, suffix), (visibility, collision, color, options))| {
            TeamParameters {
                display_name,
                options,
                nametag_visibility: nametag_visibility(visibility),
                collision_rule: collision_rule(collision),
                color,
                player_prefix: prefix,
                player_suffix: suffix,
            }
        },
    );

    let players = if matches!(
        method,
        TeamMethod::Create | TeamMethod::AddPlayers | TeamMethod::RemovePlayers
    ) {
        payload
            .get_list(|read| read.get_str().map(String::from))
            .ok()?
    } else {
        Vec::new()
    };

    let packet = CSetPlayerTeam {
        team_name,
        method,
        parameters,
        players: players.into_boxed_slice(),
    };
    let mut out = Vec::new();
    packet.write_packet_data(&mut out, &version).ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use pumpkin_util::text::TextComponent;

    use super::*;

    #[test]
    fn create_matches_direct_encode() {
        let display_name = TextComponent::text("Red");
        let prefix = TextComponent::text("[R] ");
        let suffix = TextComponent::text("");
        for color in [12, -1] {
            let packet = CSetPlayerTeam {
                team_name: "red".to_string(),
                method: TeamMethod::Create,
                parameters: Some(TeamParameters {
                    display_name: &display_name,
                    options: 3,
                    nametag_visibility: "hideForOtherTeams",
                    collision_rule: "pushOwnTeam",
                    color,
                    player_prefix: &prefix,
                    player_suffix: &suffix,
                }),
                players: vec!["Steve".to_string(), "Alex".to_string()].into_boxed_slice(),
            };
            let mut payload = Vec::new();
            packet
                .write_packet_data(&mut payload, &CURRENT_MC_VERSION)
                .unwrap();
            for version in [
                JavaMinecraftVersion::V_1_21_11,
                JavaMinecraftVersion::V_1_12_2,
            ] {
                let mut expected = Vec::new();
                packet.write_packet_data(&mut expected, &version).unwrap();
                let out = set_player_team_from_current(&payload, version).unwrap();
                assert_eq!(out, expected, "{version:?} color {color}");
            }
        }
    }
}
