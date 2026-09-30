//! COMMANDS.

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::{
    VarInt,
    java::client::play::{ArgumentType, StringProtoArgBehavior},
    ser::{NetworkReadExt, NetworkReadSliceExt, NetworkWriteExt},
};
use pumpkin_util::{identifier::Identifier, version::JavaMinecraftVersion};

use crate::remap::argument_type_id_remap::remap_argument_type_id_for_version;

const FLAG_ARGUMENT: u8 = 2;
const FLAG_HAS_REDIRECT: u8 = 8;
const FLAG_HAS_SUGGESTION_TYPE: u8 = 16;
/// New in 1.21.6.
const FLAG_IS_RESTRICTED: u8 = 32;
/// `brigadier:string`, the fallback of argument types the client lacks.
const STRING_ID: u32 = 5;

eras! {
    pub enum CommandsFormat {
        // TODO: argument types are identifiers before 1.19; the tree is dropped until then
        Identifiers = V_1_7_2,
        /// Argument type ids.
        V1_19 = V_1_19,
        /// Time arguments carry a minimum.
        V1_19_4 = V_1_19_4,
        /// Restricted flag.
        V1_21_6 = V_1_21_6,
    }
}

/// Properties following an argument type id, as core writes them.
enum Properties {
    None,
    /// Flags, then a min and max of the given size.
    Range(usize),
    VarInt,
    Byte,
    /// Time minimum.
    Int,
    String,
}

fn properties(id: u32) -> Properties {
    let id = id as i32;
    let of = |argument: ArgumentType| argument.to_id(&CURRENT_MC_VERSION);
    let resource = || Identifier::vanilla_static("");
    if id
        == of(ArgumentType::Float {
            min: None,
            max: None,
        })
        || id
            == of(ArgumentType::Integer {
                min: None,
                max: None,
            })
    {
        Properties::Range(4)
    } else if id
        == of(ArgumentType::Double {
            min: None,
            max: None,
        })
        || id
            == of(ArgumentType::Long {
                min: None,
                max: None,
            })
    {
        Properties::Range(8)
    } else if id == of(ArgumentType::String(StringProtoArgBehavior::SingleWord)) {
        Properties::VarInt
    } else if id == of(ArgumentType::Entity { flags: 0 })
        || id == of(ArgumentType::ScoreHolder { flags: 0 })
    {
        Properties::Byte
    } else if id == of(ArgumentType::Time { min: 0 }) {
        Properties::Int
    } else if id
        == of(ArgumentType::ResourceOrTag {
            identifier: resource(),
        })
        || id
            == of(ArgumentType::ResourceOrTagKey {
                identifier: resource(),
            })
        || id
            == of(ArgumentType::Resource {
                identifier: resource(),
            })
        || id
            == of(ArgumentType::ResourceKey {
                identifier: resource(),
            })
    {
        Properties::String
    } else {
        Properties::None
    }
}

/// Copies or drops an argument's properties. `write` is false when the argument became a string.
fn copy_properties(
    read: &mut &[u8],
    out: &mut Vec<u8>,
    properties: &Properties,
    write: bool,
    format: CommandsFormat,
) -> Option<()> {
    let mut sink = Vec::new();
    let target = if write { out } else { &mut sink };
    match properties {
        Properties::None => {}
        Properties::Range(size) => {
            let flags = read.get_u8().ok()?;
            target.write_u8(flags).ok()?;
            let count = usize::from(flags & 1) + usize::from((flags >> 1) & 1);
            target
                .write_slice(read.read_slice_borrowed(count * size).ok()?)
                .ok()?;
        }
        Properties::VarInt => target.write_var_int(&read.get_var_int().ok()?).ok()?,
        Properties::Byte => target.write_u8(read.get_u8().ok()?).ok()?,
        Properties::Int => {
            let min = read.get_i32_be().ok()?;
            if format >= CommandsFormat::V1_19_4 {
                target.write_i32_be(min).ok()?;
            }
        }
        Properties::String => target.write_string(read.get_str_borrowed().ok()?).ok()?,
    }
    Some(())
}

/// COMMANDS: argument type ids as the client's. Types the client lacks become quotable strings
/// (ViaVersion `CommandRewriter`).
pub fn commands_from_current(mut payload: &[u8], version: JavaMinecraftVersion) -> Option<Vec<u8>> {
    let format = CommandsFormat::of(version);
    if format == CommandsFormat::Identifiers {
        return None;
    }
    let mut out = Vec::new();
    let count = payload.get_var_int().ok()?;
    out.write_var_int(&count).ok()?;
    for _ in 0..count.0 {
        let mut flags = payload.get_u8().ok()?;
        if format < CommandsFormat::V1_21_6 {
            flags &= !FLAG_IS_RESTRICTED;
        }
        out.write_u8(flags).ok()?;
        let children = payload.get_list(|read| read.get_var_int()).ok()?;
        out.write_list(&children, |out, child| out.write_var_int(child))
            .ok()?;
        if flags & FLAG_HAS_REDIRECT != 0 {
            out.write_var_int(&payload.get_var_int().ok()?).ok()?;
        }
        if flags & 3 != 0 {
            out.write_string(payload.get_str_borrowed().ok()?).ok()?;
        }
        if flags & 3 == FLAG_ARGUMENT {
            let id = payload.get_var_int().ok()?.0 as u32;
            let client_id = remap_argument_type_id_for_version(id, version);
            let replaced = client_id == STRING_ID && id != STRING_ID;
            out.write_var_int(&VarInt(client_id as i32)).ok()?;
            copy_properties(&mut payload, &mut out, &properties(id), !replaced, format)?;
            if replaced {
                out.write_var_int(&VarInt(StringProtoArgBehavior::QuotablePhrase as i32))
                    .ok()?;
            }
        }
        if flags & FLAG_HAS_SUGGESTION_TYPE != 0 {
            out.write_string(payload.get_str_borrowed().ok()?).ok()?;
        }
    }
    out.write_var_int(&payload.get_var_int().ok()?).ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use crate::legacy::LegacyWrite;
    use pumpkin_protocol::{
        ClientPacket,
        java::client::play::{CCommands, ProtoNode, ProtoNodeType},
    };

    use super::*;

    #[test]
    fn stable_argument_types_match_a_direct_1_21_5_encode() {
        let nodes = || {
            vec![
                ProtoNode {
                    children: vec![VarInt(1)].into_boxed_slice(),
                    node_type: ProtoNodeType::Root,
                },
                ProtoNode {
                    children: vec![VarInt(2)].into_boxed_slice(),
                    node_type: ProtoNodeType::Literal {
                        name: "give",
                        is_executable: false,
                        redirect_target: None,
                        restricted: true,
                    },
                },
                ProtoNode {
                    children: Box::new([]),
                    node_type: ProtoNodeType::Argument {
                        name: "count",
                        is_executable: true,
                        redirect_target: None,
                        parser: ArgumentType::Integer {
                            min: Some(1),
                            max: None,
                        },
                        override_suggestion_type: None,
                        restricted: false,
                    },
                },
            ]
            .into_boxed_slice()
        };
        let mut current = Vec::new();
        CCommands::new(nodes(), VarInt(0))
            .write_packet_data(&mut current, &CURRENT_MC_VERSION)
            .unwrap();
        let version = JavaMinecraftVersion::V_1_21_5;
        let mut expected = Vec::new();
        CCommands::new(nodes(), VarInt(0))
            .write_legacy(&mut expected, &version)
            .unwrap();
        assert_eq!(commands_from_current(&current, version), Some(expected));
    }
}
