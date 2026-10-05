//! Pre-26.3 encodings from `pumpkin-protocol`'s `java/client/play/commands.rs`.

use super::ReferenceWrite;
use pumpkin_protocol::java::client::play::{
    ArgumentType, CCommands, ProtoNode, ProtoNodeType, StringProtoArgBehavior, SuggestionProviders,
};
use pumpkin_protocol::{ser::NetworkWriteExt, ser::WritingError};
use pumpkin_util::identifier::Identifier;
use pumpkin_util::version::JavaMinecraftVersion;
use std::io::Write;

impl ReferenceWrite for CCommands<'_> {
    fn write_legacy(
        &self,
        write: impl Write,
        version: &JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        let mut write = write;
        write.write_list(&self.nodes, |bytebuf, node: &ProtoNode| {
            node.write_to_legacy(bytebuf, *version)
        })?;
        write.write_var_int(&self.root_node_index)
    }
}

const FLAG_IS_EXECUTABLE: u8 = 4;
const FLAG_HAS_REDIRECT: u8 = 8;
const FLAG_HAS_SUGGESTION_TYPE: u8 = 16;
const FLAG_IS_RESTRICTED: u8 = 32;

const fn suggestion_resource_location(provider: SuggestionProviders) -> &'static str {
    match provider {
        SuggestionProviders::AskServer => "minecraft:ask_server",
        SuggestionProviders::AllRecipes => "minecraft:all_recipes",
        SuggestionProviders::AvailableSounds => "minecraft:available_sounds",
        SuggestionProviders::SummonableEntities => "minecraft:summonable_entities",
    }
}

trait LegacyProtoNode {
    fn write_to_legacy(
        &self,
        bytebuf: &mut impl Write,
        version: JavaMinecraftVersion,
    ) -> Result<(), WritingError>;
}

impl LegacyProtoNode for ProtoNode<'_> {
    fn write_to_legacy(
        &self,
        write: &mut impl Write,
        version: JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        let v1_21_6 = version >= JavaMinecraftVersion::V_1_21_6;

        // flags
        let mut redirect_target_on_flag = 0i32;

        let flags = match self.node_type {
            ProtoNodeType::Root => 0,
            ProtoNodeType::Literal {
                name: _,
                is_executable,
                redirect_target,
                restricted,
            } => {
                let mut n = 1;
                if restricted && v1_21_6 {
                    n |= FLAG_IS_RESTRICTED;
                }
                if is_executable {
                    n |= FLAG_IS_EXECUTABLE;
                }
                if let Some(target) = redirect_target {
                    n |= FLAG_HAS_REDIRECT;
                    redirect_target_on_flag = target;
                }
                n
            }
            ProtoNodeType::Argument {
                name: _,
                is_executable,
                parser: _,
                override_suggestion_type,
                redirect_target,
                restricted,
            } => {
                let mut n = 2;
                if restricted && v1_21_6 {
                    n |= FLAG_IS_RESTRICTED;
                }
                if override_suggestion_type.is_some() {
                    n |= FLAG_HAS_SUGGESTION_TYPE;
                }
                if is_executable {
                    n |= FLAG_IS_EXECUTABLE;
                }
                if let Some(target) = redirect_target {
                    n |= FLAG_HAS_REDIRECT;
                    redirect_target_on_flag = target;
                }
                n
            }
        };
        write.write_u8(flags)?;

        // child count + children
        write.write_list(&self.children, |bytebuf, child| {
            bytebuf.write_var_int(child)
        })?;

        // redirect node
        if flags & FLAG_HAS_REDIRECT != 0 {
            write.write_var_int(&redirect_target_on_flag.into())?;
        }

        // name
        match self.node_type {
            ProtoNodeType::Argument { name, .. } | ProtoNodeType::Literal { name, .. } => {
                write.write_string(name)?;
            }
            ProtoNodeType::Root => {}
        }

        // parser id + properties
        if let ProtoNodeType::Argument { parser, .. } = &self.node_type {
            parser.write_to_buffer_legacy(write, version)?;
        }

        if flags & FLAG_HAS_SUGGESTION_TYPE != 0 {
            match &self.node_type {
                ProtoNodeType::Argument {
                    override_suggestion_type,
                    ..
                } => {
                    // suggestion type
                    let suggestion_type = override_suggestion_type.as_ref().ok_or_else(|| {
                        WritingError::Message("ProtoNode::FLAG_HAS_SUGGESTION_TYPE set but override_suggestion_type is None".into())
                    })?;
                    write.write_string(suggestion_resource_location(*suggestion_type))?;
                }
                _ => return Err(WritingError::Message(
                    "`ProtoNode::FLAG_HAS_SUGGESTION_TYPE` is only implemented for `ProtoNodeType::Argument`".into()
                )),
            }
        }

        Ok(())
    }
}

trait LegacyArgumentType {
    fn write_to_buffer_legacy(
        &self,
        write: &mut impl Write,
        version: JavaMinecraftVersion,
    ) -> Result<(), WritingError>;
    fn legacy_identifier_name(&self, version: JavaMinecraftVersion) -> (&'static str, bool);
}

impl LegacyArgumentType for ArgumentType {
    #[expect(clippy::match_same_arms)]
    fn write_to_buffer_legacy(
        &self,
        write: &mut impl Write,
        version: JavaMinecraftVersion,
    ) -> Result<(), WritingError> {
        if version >= JavaMinecraftVersion::V_1_19 {
            let id = self.to_id();
            write.write_var_int(&(id).into())?;
            if id == 5 {
                let behavior_val = match self {
                    Self::String(StringProtoArgBehavior::SingleWord) => 0,
                    Self::String(StringProtoArgBehavior::QuotablePhrase) => 1,
                    Self::String(StringProtoArgBehavior::GreedyPhrase) => 2,
                    _ => 0,
                };
                return write.write_var_int(&behavior_val.into());
            }

            match self {
                Self::Float { min, max } => write_number_arg(*min, *max, write),
                Self::Double { min, max } => write_number_arg(*min, *max, write),
                Self::Integer { min, max } => write_number_arg(*min, *max, write),
                Self::Long { min, max } => write_number_arg(*min, *max, write),
                Self::Entity { flags } => write_with_flags(*flags, write),
                Self::ScoreHolder { flags } => write_with_flags(*flags, write),
                Self::Time { min } => {
                    if version >= JavaMinecraftVersion::V_1_19_4 {
                        write.write_i32_be(*min)
                    } else {
                        Ok(())
                    }
                }
                Self::ResourceOrTag { identifier } => write_with_identifier(identifier, write),
                Self::ResourceOrTagKey { identifier } => write_with_identifier(identifier, write),
                Self::Resource { identifier } => write_with_identifier(identifier, write),
                Self::ResourceKey { identifier } => write_with_identifier(identifier, write),
                _ => Ok(()),
            }
        } else {
            let (identifier, is_remapped_to_string) = self.legacy_identifier_name(version);
            write.write_string(identifier)?;
            if is_remapped_to_string {
                let behavior_val = match self {
                    Self::String(StringProtoArgBehavior::SingleWord) => 0,
                    Self::String(StringProtoArgBehavior::QuotablePhrase) => 1,
                    Self::String(StringProtoArgBehavior::GreedyPhrase) => 2,
                    _ => 0,
                };
                return write.write_var_int(&behavior_val.into());
            }

            match self {
                Self::Float { min, max } => write_number_arg(*min, *max, write),
                Self::Double { min, max } => write_number_arg(*min, *max, write),
                Self::Integer { min, max } => write_number_arg(*min, *max, write),
                Self::Long { min, max } => {
                    if version >= JavaMinecraftVersion::V_1_14 {
                        write_number_arg(*min, *max, write)
                    } else {
                        let min_i32 = min.map(|v| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
                        let max_i32 = max.map(|v| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
                        write_number_arg(min_i32, max_i32, write)
                    }
                }
                Self::Entity { flags } => write_with_flags(*flags, write),
                Self::ScoreHolder { flags } => write_with_flags(*flags, write),
                Self::Time { .. } => Ok(()),
                _ => Ok(()),
            }
        }
    }

    #[expect(clippy::match_same_arms)]
    fn legacy_identifier_name(&self, version: JavaMinecraftVersion) -> (&'static str, bool) {
        match self {
            Self::Bool => ("brigadier:bool", false),
            Self::Float { .. } => ("brigadier:float", false),
            Self::Double { .. } => ("brigadier:double", false),
            Self::Integer { .. } => ("brigadier:integer", false),
            Self::Long { .. } => {
                if version >= JavaMinecraftVersion::V_1_14 {
                    ("brigadier:long", false)
                } else {
                    ("brigadier:integer", false)
                }
            }
            Self::String(_) => ("brigadier:string", true),
            Self::Entity { .. } => ("minecraft:entity", false),
            Self::GameProfile => ("minecraft:game_profile", false),
            Self::BlockPos => ("minecraft:block_pos", false),
            Self::ColumnPos => ("minecraft:column_pos", false),
            Self::Vec3 => ("minecraft:vec3", false),
            Self::Vec2 => ("minecraft:vec2", false),
            Self::BlockState => ("minecraft:block_state", false),
            Self::BlockPredicate => ("minecraft:block_predicate", false),
            Self::ItemStack => ("minecraft:item_stack", false),
            Self::ItemPredicate => ("minecraft:item_predicate", false),
            Self::Color => ("minecraft:color", false),
            Self::Component => ("minecraft:component", false),
            Self::Message => ("minecraft:message", false),
            Self::NbtCompound => {
                if version >= JavaMinecraftVersion::V_1_14 {
                    ("minecraft:nbt_compound_tag", false)
                } else {
                    ("minecraft:nbt", false)
                }
            }
            Self::NbtTag => {
                if version >= JavaMinecraftVersion::V_1_14 {
                    ("minecraft:nbt_tag", false)
                } else {
                    ("minecraft:nbt", false)
                }
            }
            Self::NbtPath => ("minecraft:nbt_path", false),
            Self::Objective => ("minecraft:objective", false),
            Self::ObjectiveCriteria => ("minecraft:objective_criteria", false),
            Self::Operation => ("minecraft:operation", false),
            Self::Particle => ("minecraft:particle", false),
            Self::Angle => {
                if version >= JavaMinecraftVersion::V_1_16 {
                    ("minecraft:angle", false)
                } else {
                    ("brigadier:string", true)
                }
            }
            Self::Rotation => ("minecraft:rotation", false),
            Self::ScoreboardSlot => ("minecraft:scoreboard_slot", false),
            Self::ScoreHolder { .. } => ("minecraft:score_holder", false),
            Self::Swizzle => ("minecraft:swizzle", false),
            Self::Team => ("minecraft:team", false),
            Self::ItemSlot | Self::ItemSlots => ("minecraft:item_slot", false),
            Self::ResourceLocation => ("minecraft:resource_location", false),
            Self::Function => ("minecraft:function", false),
            Self::EntityAnchor => ("minecraft:entity_anchor", false),
            Self::IntRange => ("minecraft:int_range", false),
            Self::FloatRange => {
                if version >= JavaMinecraftVersion::V_1_14 {
                    ("minecraft:float_range", false)
                } else {
                    ("brigadier:string", true)
                }
            }
            Self::Dimension => ("minecraft:dimension", false),
            Self::Gamemode => ("brigadier:string", true),
            Self::Time { .. } => {
                if version >= JavaMinecraftVersion::V_1_14 {
                    ("minecraft:time", false)
                } else {
                    ("brigadier:string", true)
                }
            }
            Self::TemplateMirror => {
                if version >= JavaMinecraftVersion::V_1_19 {
                    ("minecraft:template_mirror", false)
                } else {
                    ("brigadier:string", true)
                }
            }
            Self::TemplateRotation => {
                if version >= JavaMinecraftVersion::V_1_19 {
                    ("minecraft:template_rotation", false)
                } else {
                    ("brigadier:string", true)
                }
            }
            Self::Uuid if version >= JavaMinecraftVersion::V_1_16 => ("minecraft:uuid", false),
            _ => ("brigadier:string", true),
        }
    }
}

fn write_number_arg<T: NumberCmdArg>(
    min: Option<T>,
    max: Option<T>,
    write: &mut impl Write,
) -> Result<(), WritingError> {
    let mut flags: u8 = 0;
    if min.is_some() {
        flags |= 1;
    }
    if max.is_some() {
        flags |= 2;
    }

    write.write_u8(flags)?;
    if let Some(min) = min {
        min.write(write)?;
    }
    if let Some(max) = max {
        max.write(write)?;
    }

    Ok(())
}

fn write_with_flags(flags: u8, write: &mut impl Write) -> Result<(), WritingError> {
    write.write_u8(flags)
}

fn write_with_identifier(
    extra_identifier: &Identifier,
    write: &mut impl Write,
) -> Result<(), WritingError> {
    write.write_string(extra_identifier.to_string().as_str())
}

trait NumberCmdArg {
    fn write(self, write: &mut impl Write) -> std::result::Result<(), WritingError>;
}

impl NumberCmdArg for f32 {
    fn write(self, write: &mut impl Write) -> std::result::Result<(), WritingError> {
        write.write_f32_be(self)
    }
}

impl NumberCmdArg for f64 {
    fn write(self, write: &mut impl Write) -> std::result::Result<(), WritingError> {
        write.write_f64_be(self)
    }
}

impl NumberCmdArg for i32 {
    fn write(self, write: &mut impl Write) -> std::result::Result<(), WritingError> {
        write.write_i32_be(self)
    }
}

impl NumberCmdArg for i64 {
    fn write(self, write: &mut impl Write) -> std::result::Result<(), WritingError> {
        write.write_i64_be(self)
    }
}
