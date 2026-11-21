//! Text components before 26.3. Core writes the current NBT, like `ViaBackwards` component
//! rewriters

use std::fmt::Write as _;

use crate::translate::item::component_id_for_version;
use pumpkin_nbt::{compound::NbtCompound, serializer::NbtWriteHelperJava, tag::NbtTag};
use pumpkin_util::{
    text::{TextComponent, TextComponentBase, color::Color},
    translation::Locale,
    version::JavaMinecraftVersion,
};

/// NBT on the wire since 1.20.3
#[must_use]
pub fn encode(component: &TextComponent, version: &JavaMinecraftVersion) -> Box<[u8]> {
    let mut bytes = Vec::new();
    let _ = to_nbt(component, version).serialize(&mut NbtWriteHelperJava::new(&mut bytes));
    bytes.into_boxed_slice()
}

/// JSON before 1.20.3: also the sign text layout up to 1.21.4
#[must_use]
pub fn to_json(component: &TextComponent, version: &JavaMinecraftVersion) -> String {
    tag_to_json(&to_nbt(component, version)).to_string()
}

#[must_use]
pub fn to_nbt(component: &TextComponent, version: &JavaMinecraftVersion) -> NbtTag {
    let mut tag = component.to_nbt_tag_for_version(&pumpkin_data::packet::CURRENT_MC_VERSION);
    downgrade(&mut tag, *version);
    tag
}

/// `§` codes, RGB colours before 1.16 become the nearest named colour.
#[must_use]
pub fn to_legacy_string(
    component: &TextComponent,
    version: &JavaMinecraftVersion,
    locale: Locale,
) -> String {
    if *version >= JavaMinecraftVersion::V_1_16 {
        return component.to_legacy_string(locale);
    }
    let mut base = component.0.clone();
    name_rgb_colors(&mut base);
    TextComponent(base).to_legacy_string(locale)
}

fn name_rgb_colors(base: &mut TextComponentBase) {
    base.style.color = base.style.color.map(|color| color.downsample());
    base.extra.iter_mut().for_each(name_rgb_colors);
}

fn downgrade(tag: &mut NbtTag, version: JavaMinecraftVersion) {
    match tag {
        NbtTag::Compound(compound) => downgrade_compound(compound, version),
        NbtTag::List(list) => list.iter_mut().for_each(|tag| downgrade(tag, version)),
        _ => {}
    }
}

fn downgrade_compound(tag: &mut NbtCompound, version: JavaMinecraftVersion) {
    for key in ["extra", "with", "fallback"] {
        if let Some(child) = tag.child_tags.get_mut(key) {
            downgrade(child, version);
        }
    }
    for key in ["hover_event", "hoverEvent"] {
        if let Some(NbtTag::Compound(hover)) = tag.child_tags.get_mut(key) {
            for inner in ["value", "name", "contents"] {
                if let Some(child) = hover.child_tags.get_mut(inner) {
                    downgrade(child, version);
                }
            }
            remap_hover(hover, version);
        }
    }

    // 1.21.9 object contents (ViaBackwards 1.21.9 -> 1.21.7)
    if version < JavaMinecraftVersion::V_1_21_9 {
        object_to_text(tag);
    }
    // Snake case events, flat hover fields (ViaBackwards 1.21.5 -> 1.21.4)
    if version < JavaMinecraftVersion::V_1_21_5 {
        if let Some(NbtTag::Compound(mut click)) = tag.child_tags.remove("click_event") {
            click_to_1_21_4(&mut click);
            tag.put_compound("clickEvent", click);
        }
        if let Some(NbtTag::Compound(mut hover)) = tag.child_tags.remove("hover_event") {
            hover_to_1_21_4(&mut hover);
            tag.put_compound("hoverEvent", hover);
        }
    }
    if version < JavaMinecraftVersion::V_1_21_4 {
        tag.child_tags.remove("shadow_color");
    }
    // Hex colours, hover contents and fonts (ViaBackwards 1.16 -> 1.15.2)
    if version < JavaMinecraftVersion::V_1_16 {
        if let Some(NbtTag::String(color)) = tag.child_tags.get_mut("color")
            && let Some(Color::Rgb(rgb)) = color.strip_prefix('#').and_then(Color::from_hex_str)
        {
            *color = rgb.to_nearest_named().name().into();
        }
        tag.child_tags.remove("font");
        if let Some(NbtTag::Compound(hover)) = tag.child_tags.get_mut("hoverEvent") {
            hover_to_1_15(hover, version);
        }
    }
    // copy_to_clipboard was added in 1.15
    if version < JavaMinecraftVersion::V_1_15
        && let Some(NbtTag::Compound(click)) = tag.child_tags.get_mut("clickEvent")
        && click.get_string("action") == Some("copy_to_clipboard")
    {
        click.put_string("action", "suggest_command".to_string());
    }
}

/// A hover naming an item or component the client lacks makes it drop the whole packet.
fn remap_hover(hover: &mut NbtCompound, version: JavaMinecraftVersion) {
    if hover.get_string("action") == Some("show_item") {
        remap_hover_item(hover, version);
    }
}

fn remap_hover_item(hover: &mut NbtCompound, version: JavaMinecraftVersion) {
    use crate::remap::item_id_remap::{remap_item_id_for_version, remap_item_id_from_version};
    use pumpkin_data::{data_component::DataComponent, item::Item};

    if let Some(item) = hover
        .get_string("id")
        .and_then(|id| Item::from_registry_key(id.strip_prefix("minecraft:").unwrap_or(id)))
    {
        // Back and forth through the tables: the client's id stands for a 26.3 item, maybe another one
        let client_id = remap_item_id_for_version(item.id, version);
        if let Some(native) = Item::from_id(remap_item_id_from_version(client_id, version))
            && native.id != item.id
        {
            hover.put_string("id", format!("minecraft:{}", native.registry_key));
        }
    }
    // shortcut: kept components keep their 26.3 payload, like in `translate::item`
    if let Some(NbtTag::Compound(components)) = hover.child_tags.get_mut("components") {
        components.child_tags.retain(|key, _| {
            let name = key.strip_prefix('!').unwrap_or(key);
            // A key without a namespace means `minecraft:`, which is the form some names need
            let name = if name.contains(':') {
                name.to_string()
            } else {
                format!("minecraft:{name}")
            };
            DataComponent::try_from_name(&name)
                .is_none_or(|id| component_id_for_version(id.to_id(), version).is_some())
        });
        for (key, child) in &mut components.child_tags {
            if matches!(
                key.strip_prefix("minecraft:").unwrap_or(key),
                "custom_name" | "item_name" | "lore"
            ) {
                downgrade(child, version);
            }
        }
    }
}

/// Keeps the text vanilla shows when the object can't be drawn.
fn object_to_text(tag: &mut NbtCompound) {
    let is_object = tag.get_string("type") == Some("object")
        || ["player", "sprite", "atlas", "object"]
            .iter()
            .any(|key| tag.has(key));
    if !is_object {
        return;
    }
    let default = tag
        .get_compound("player")
        .map_or_else(String::new, |player| {
            player.get_string("name").map_or_else(
                || "[unknown player head]".to_string(),
                |name| format!("[{name} head]"),
            )
        });
    let fallback = tag.child_tags.remove("fallback");
    for key in ["type", "object", "player", "hat", "sprite", "atlas"] {
        tag.child_tags.remove(key);
    }
    match fallback {
        Some(fallback) => {
            tag.put_string("text", String::new());
            let mut extra = vec![fallback];
            if let Some(NbtTag::List(rest)) = tag.child_tags.remove("extra") {
                extra.extend(rest);
            }
            tag.put_list("extra", extra);
        }
        None => tag.put_string("text", default),
    }
}

fn click_to_1_21_4(click: &mut NbtCompound) {
    let value = match click.get_string("action") {
        Some("open_url") => click.child_tags.remove("url"),
        Some("open_file") => click.child_tags.remove("path"),
        Some("suggest_command") => click.child_tags.remove("command"),
        // Commands needed their slash until 1.21.5
        Some("run_command") => match click.child_tags.remove("command") {
            Some(NbtTag::String(command)) if !command.starts_with('/') => {
                Some(NbtTag::String(format!("/{command}").into()))
            }
            command => command,
        },
        Some("change_page") => click.child_tags.remove("page").and_then(|page| match page {
            NbtTag::Int(page) => Some(NbtTag::String(page.to_string().into())),
            _ => None,
        }),
        _ => None,
    };
    if let Some(value) = value {
        click.put("value", value);
    }
}

fn hover_to_1_21_4(hover: &mut NbtCompound) {
    match hover.get_string("action") {
        Some("show_text") => {
            if let Some(value) = hover.child_tags.remove("value") {
                hover.put("contents", value);
            }
        }
        Some("show_item") => {
            let mut contents = NbtCompound::new();
            for key in ["id", "count", "components"] {
                if let Some(value) = hover.child_tags.remove(key) {
                    contents.put(key, value);
                }
            }
            hover.put_compound("contents", contents);
        }
        Some("show_entity") => {
            let mut contents = NbtCompound::new();
            for (from, to) in [("id", "type"), ("uuid", "id"), ("name", "name")] {
                if let Some(value) = hover.child_tags.remove(from) {
                    contents.put(to, value);
                }
            }
            hover.put_compound("contents", contents);
        }
        _ => {}
    }
}

/// Before 1.16 items and entities were SNBT in a text `value`.
fn hover_to_1_15(hover: &mut NbtCompound, version: JavaMinecraftVersion) {
    let Some(contents) = hover.child_tags.remove("contents") else {
        return;
    };
    let value = match (hover.get_string("action"), contents) {
        (Some("show_item"), NbtTag::Compound(item)) => {
            let id = item.get_string("id").unwrap_or("minecraft:air");
            let count = item.get_int("count").unwrap_or(1);
            NbtTag::String(format!("{{id:{},Count:{count}b}}", snbt_string(id)).into())
        }
        (Some("show_entity"), NbtTag::Compound(entity)) => {
            let mut snbt = String::from("{");
            if let Some(name) = entity.get("name") {
                // A JSON component since 1.13, plain text before
                let name = if version >= JavaMinecraftVersion::V_1_13 {
                    tag_to_json(name).to_string()
                } else {
                    TextComponent::from_nbt(name).get_text()
                };
                let _ = write!(snbt, "name:{},", snbt_string(&name));
            }
            let kind = entity.get_string("type").unwrap_or_default();
            let id = entity.get_string("id").unwrap_or_default();
            let _ = write!(snbt, "type:{},id:{}}}", snbt_string(kind), snbt_string(id));
            NbtTag::String(snbt.into())
        }
        (_, text) => text,
    };
    hover.put("value", value);
}

fn snbt_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

const BOOLEAN_KEYS: [&str; 7] = [
    "bold",
    "italic",
    "underlined",
    "strikethrough",
    "obfuscated",
    "interpret",
    "hat",
];

/// Component NBT to JSON: style flags are booleans and `{"": x}` list wrappers unwrap.
fn tag_to_json(tag: &NbtTag) -> serde_json::Value {
    match tag {
        NbtTag::Compound(compound) => {
            if compound.child_tags.len() == 1
                && let Some(inner) = compound.get("")
            {
                return tag_to_json(inner);
            }
            compound
                .child_tags
                .iter()
                .map(|(key, value)| {
                    let json = match value {
                        NbtTag::Byte(flag) if BOOLEAN_KEYS.contains(&&**key) => {
                            serde_json::Value::Bool(*flag != 0)
                        }
                        other => tag_to_json(other),
                    };
                    (key.to_string(), json)
                })
                .collect::<serde_json::Map<_, _>>()
                .into()
        }
        NbtTag::List(list) => list.iter().map(tag_to_json).collect(),
        NbtTag::String(text) => serde_json::Value::String(text.to_string()),
        NbtTag::Byte(v) => (*v).into(),
        NbtTag::Short(v) => (*v).into(),
        NbtTag::Int(v) => (*v).into(),
        NbtTag::Long(v) => (*v).into(),
        NbtTag::Float(v) => (*v).into(),
        NbtTag::Double(v) => (*v).into(),
        NbtTag::ByteArray(v) => v.iter().copied().collect(),
        NbtTag::IntArray(v) => v.iter().copied().collect(),
        NbtTag::LongArray(v) => v.iter().copied().collect(),
        NbtTag::End => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use pumpkin_util::text::{click::ClickEvent, color::RGBColor, hover::HoverEvent};

    use super::*;

    fn json(component: &TextComponent, version: JavaMinecraftVersion) -> serde_json::Value {
        serde_json::from_str(&to_json(component, &version)).unwrap()
    }

    fn hover_item(id: &str, version: JavaMinecraftVersion) -> String {
        let mut hover = NbtCompound::new();
        hover.put_string("action", "show_item".to_string());
        hover.put_string("id", id.to_string());
        remap_hover(&mut hover, version);
        hover.get_string("id").unwrap().to_string()
    }

    #[test]
    fn show_item_names_an_item_the_client_has() {
        for version in [
            JavaMinecraftVersion::V_1_21_4,
            JavaMinecraftVersion::V_26_1,
            JavaMinecraftVersion::V_26_2,
        ] {
            assert_eq!(hover_item("minecraft:stone", version), "minecraft:stone");
        }
        let id = hover_item("minecraft:poplar_planks", JavaMinecraftVersion::V_26_2);
        assert_ne!(id, "minecraft:poplar_planks");
        assert_eq!(
            hover_item("minecraft:poplar_planks", JavaMinecraftVersion::V_26_3),
            "minecraft:poplar_planks"
        );
    }

    #[test]
    fn show_item_drops_components_the_client_lacks() {
        use pumpkin_data::data_component::DataComponent;

        let version = JavaMinecraftVersion::V_26_2;
        let lacking = (1..=u8::MAX)
            .filter_map(DataComponent::try_from_id)
            .find(|c| component_id_for_version(c.to_id(), version).is_none())
            .unwrap();
        let mut components = NbtCompound::new();
        components.put_string(lacking.to_name(), String::new());
        components.put_string("minecraft:custom_data", String::new());
        let mut hover = NbtCompound::new();
        hover.put_string("action", "show_item".to_string());
        hover.put_string("id", "minecraft:stone".to_string());
        hover.put_compound("components", components);
        remap_hover(&mut hover, version);
        let components = hover.get_compound("components").unwrap();
        assert!(!components.has(lacking.to_name()));
        assert!(components.has("minecraft:custom_data"));

        // the same component without its namespace is dropped too
        let bare = lacking
            .to_name()
            .trim_start_matches("minecraft:")
            .to_string();
        let mut components = NbtCompound::new();
        components.put_string(&bare, String::new());
        let mut hover = NbtCompound::new();
        hover.put_string("action", "show_item".to_string());
        hover.put_string("id", "minecraft:stone".to_string());
        hover.put_compound("components", components);
        remap_hover(&mut hover, version);
        assert!(!hover.get_compound("components").unwrap().has(&bare));
    }

    #[test]
    fn show_item_downgrades_namespaced_component_texts() {
        let version = JavaMinecraftVersion::V_26_2;
        let mut inner = NbtCompound::new();
        inner.put_string("action", "show_item".to_string());
        inner.put_string("id", "minecraft:poplar_planks".to_string());
        let mut name = NbtCompound::new();
        name.put_string("text", "x".to_string());
        name.put_compound("hover_event", inner);
        let mut components = NbtCompound::new();
        components.put_compound("minecraft:custom_name", name);
        let mut hover = NbtCompound::new();
        hover.put_string("action", "show_item".to_string());
        hover.put_string("id", "minecraft:stone".to_string());
        hover.put_compound("components", components);
        remap_hover(&mut hover, version);
        let name = hover
            .get_compound("components")
            .and_then(|c| c.get_compound("minecraft:custom_name"))
            .and_then(|n| n.get_compound("hover_event"))
            .unwrap();
        assert_ne!(name.get_string("id"), Some("minecraft:poplar_planks"));
    }

    #[test]
    fn click_event_uses_value_before_1_21_5() {
        let link = TextComponent::text("link").click_event(ClickEvent::OpenUrl {
            url: Cow::Borrowed("https://example.com"),
        });
        let tag = to_nbt(&link, &JavaMinecraftVersion::V_1_21_4);
        let NbtTag::Compound(tag) = tag else { panic!() };
        let click = tag.get_compound("clickEvent").unwrap();
        assert_eq!(click.get_string("value"), Some("https://example.com"));
        assert!(click.get_string("url").is_none());

        let run = TextComponent::text("go").click_event(ClickEvent::RunCommand {
            command: Cow::Borrowed("spawn"),
        });
        assert_eq!(
            json(&run, JavaMinecraftVersion::V_1_20_2)["clickEvent"]["value"],
            "/spawn"
        );

        let page = TextComponent::text("next").click_event(ClickEvent::ChangePage { page: 3 });
        assert_eq!(
            json(&page, JavaMinecraftVersion::V_1_20_2)["clickEvent"]["value"],
            "3"
        );
    }

    #[test]
    fn hover_uses_contents_from_1_16_and_snbt_before() {
        let entity = TextComponent::text("pig").hover_event(HoverEvent::ShowEntity {
            id: Cow::Borrowed("minecraft:pig"),
            uuid: Cow::Borrowed("00000000-0000-0000-0000-000000000001"),
            name: Some(vec![TextComponent::text("Bob \"B\"").0]),
        });
        let hover = &json(&entity, JavaMinecraftVersion::V_1_16)["hoverEvent"];
        assert_eq!(hover["contents"]["type"], "minecraft:pig");
        assert_eq!(hover["contents"]["name"], "Bob \"B\"");

        let hover = &json(&entity, JavaMinecraftVersion::V_1_12_2)["hoverEvent"];
        assert_eq!(
            hover["value"],
            r#"{name:"Bob \"B\"",type:"minecraft:pig",id:"00000000-0000-0000-0000-000000000001"}"#
        );
    }

    #[test]
    fn rgb_becomes_nearest_named_before_1_16() {
        let text = TextComponent::text("hi").color_rgb(RGBColor::new(0xff, 0x50, 0x50));
        assert_eq!(json(&text, JavaMinecraftVersion::V_1_15_2)["color"], "red");
        assert_eq!(
            json(&text, JavaMinecraftVersion::V_1_16)["color"],
            "#FF5050"
        );
        assert_eq!(
            to_legacy_string(&text, &JavaMinecraftVersion::V_1_12_2, Locale::EnUs),
            "§chi"
        );
    }

    #[test]
    fn plain_text_stays_a_string_and_flags_are_booleans() {
        let plain = TextComponent::text("hello");
        assert_eq!(json(&plain, JavaMinecraftVersion::V_1_8), "hello");
        let bold = TextComponent::text("hey").bold();
        assert_eq!(json(&bold, JavaMinecraftVersion::V_1_8)["bold"], true);
    }
}
