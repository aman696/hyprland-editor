//! Bounded package-registry metadata parser. Never resolves DTDs or external entities.
use quick_xml::{events::Event, Reader};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub choice: Choice,
    pub variants: Vec<Choice>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub choice: Choice,
    pub multiple: bool,
    pub options: Vec<Choice>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub models: Vec<Choice>,
    pub layouts: Vec<Layout>,
    pub groups: Vec<Group>,
}
fn valid(choice: &Choice) -> bool {
    !choice.id.is_empty()
        && choice.id.len() <= 256
        && choice
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_:-+.()".contains(&b))
        && choice.label.len() <= 4096
        && !choice
            .label
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
}
fn unique(choices: impl Iterator<Item = Choice>) -> bool {
    let mut ids = BTreeSet::new();
    choices.into_iter().all(|c| valid(&c) && ids.insert(c.id))
}
impl Registry {
    pub fn validate(&self) -> Result<(), String> {
        if self.models.len() > 1024
            || self.layouts.len() > 1024
            || self.groups.len() > 256
            || self.layouts.iter().any(|l| l.variants.len() > 512)
            || self.groups.iter().any(|g| g.options.len() > 1024)
            || !unique(self.models.iter().cloned())
            || !unique(self.layouts.iter().map(|l| l.choice.clone()))
            || {
                let mut ids = BTreeSet::new();
                self.groups.iter().any(|g| {
                    g.choice.id.trim().is_empty()
                        || g.choice.id.len() > 256
                        || g.choice.id.chars().any(char::is_control)
                        || g.choice.label.len() > 4096
                        || g.choice
                            .label
                            .chars()
                            .any(|c| c.is_control() && c != '\n' && c != '\t')
                        || !ids.insert(&g.choice.id)
                })
            }
            || self
                .layouts
                .iter()
                .any(|l| !unique(l.variants.iter().cloned()))
            || self
                .groups
                .iter()
                .any(|g| !unique(g.options.iter().cloned()))
        {
            return Err("Invalid, duplicate or oversized keyboard registry records".into());
        }
        Ok(())
    }
}
pub fn parse(xml: &str) -> Result<Registry, String> {
    if xml.len() > crate::MAX_DOCUMENT_BYTES {
        return Err("Keyboard registry exceeds 1 MiB".into());
    }
    let mut reader = Reader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    let mut path = Vec::<String>::new();
    let mut root = false;
    let mut r = Registry::default();
    let (mut model, mut variant, mut option) =
        (Choice::default(), Choice::default(), Choice::default());
    let mut layout = Layout::default();
    let mut group = Group::default();
    loop {
        let event = reader
            .read_event()
            .map_err(|_| "Invalid keyboard registry XML")?;
        let chunk = match event {
            Event::Start(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .map_err(|_| "Invalid registry tag")?
                    .to_string();
                if name.len() > 128 || path.len() >= 16 {
                    return Err("Registry nesting/tag quota exceeded".into());
                }
                if path.is_empty() {
                    if root || name != "xkbConfigRegistry" {
                        return Err("Expected one xkbConfigRegistry root".into());
                    }
                    root = true;
                }
                let parent = path.last().map(String::as_str);
                match (parent, name.as_str()) {
                    (Some("modelList"), "model") => model = Choice::default(),
                    (Some("layoutList"), "layout") => layout = Layout::default(),
                    (Some("variantList"), "variant") => variant = Choice::default(),
                    (Some("optionList"), "group") => {
                        group = Group::default();
                        for a in e.attributes() {
                            let a = a.map_err(|_| "Invalid registry attribute")?;
                            if a.key.as_ref() == b"allowMultipleSelection" {
                                group.multiple = match a.value.as_ref() {
                                    b"true" => true,
                                    b"false" => false,
                                    _ => return Err("Invalid option-group selection policy".into()),
                                };
                            }
                        }
                    }
                    (Some("group"), "option") => option = Choice::default(),
                    _ => {}
                }
                path.push(name);
                None
            }
            Event::End(e) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .map_err(|_| "Invalid registry end tag")?
                    .to_string();
                if path.pop().as_deref() != Some(&name) {
                    return Err("Unbalanced registry XML".into());
                }
                match (path.last().map(String::as_str), name.as_str()) {
                    (Some("modelList"), "model") => r.models.push(std::mem::take(&mut model)),
                    (Some("variantList"), "variant") => {
                        layout.variants.push(std::mem::take(&mut variant))
                    }
                    (Some("layoutList"), "layout") => r.layouts.push(std::mem::take(&mut layout)),
                    (Some("group"), "option") => group.options.push(std::mem::take(&mut option)),
                    (Some("optionList"), "group") => r.groups.push(std::mem::take(&mut group)),
                    _ => {}
                }
                None
            }
            Event::Text(e) => Some(
                e.decode()
                    .map_err(|_| "Invalid registry text")?
                    .into_owned(),
            ),
            Event::CData(e) => Some(
                e.decode()
                    .map_err(|_| "Invalid registry CDATA")?
                    .into_owned(),
            ),
            Event::GeneralRef(e) => {
                let name = e.decode().map_err(|_| "Invalid entity reference")?;
                let s = match name.as_ref() {
                    "amp" => "&".into(),
                    "lt" => "<".into(),
                    "gt" => ">".into(),
                    "apos" => "'".into(),
                    "quot" => "\"".into(),
                    n if n.starts_with('#') => {
                        let value = if let Some(n) = n.strip_prefix("#x") {
                            u32::from_str_radix(n, 16)
                        } else {
                            n[1..].parse()
                        }
                        .map_err(|_| "Invalid numeric reference")?;
                        char::from_u32(value)
                            .ok_or("Invalid entity codepoint")?
                            .to_string()
                    }
                    _ => return Err("External or custom XML entities are not supported".into()),
                };
                Some(s)
            }
            Event::DocType(e) => {
                if e.decode().map_err(|_| "Invalid DTD")?.as_ref()
                    != "xkbConfigRegistry SYSTEM \"xkb.dtd\""
                {
                    return Err(
                        "Custom DTDs are not accepted; no external entities are resolved".into(),
                    );
                }
                None
            }
            Event::Eof => break,
            _ => None,
        };
        if let Some(s) = chunk {
            if path.len() >= 3 && path[path.len() - 2] == "configItem" {
                let leaf = path.last().unwrap().as_str();
                let owner = path[path.len() - 3].as_str();
                let choice = match owner {
                    "model" => Some(&mut model),
                    "layout" => Some(&mut layout.choice),
                    "variant" => Some(&mut variant),
                    "group" => Some(&mut group.choice),
                    "option" => Some(&mut option),
                    _ => None,
                };
                if let Some(c) = choice {
                    let target = match leaf {
                        "name" => Some(&mut c.id),
                        "description" => Some(&mut c.label),
                        _ => None,
                    };
                    if let Some(target) = target {
                        if target.len() + s.len() > 4096 {
                            return Err("Registry text quota exceeded".into());
                        }
                        target.push_str(&s);
                    }
                }
            }
        }
    }
    if !root || !path.is_empty() {
        return Err("Incomplete registry XML".into());
    }
    r.validate()?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    pub const SAMPLE: &str = r#"<?xml version="1.0"?><!DOCTYPE xkbConfigRegistry SYSTEM "xkb.dtd"><xkbConfigRegistry><modelList><model><configItem><name>pc105</name><description>PC &amp; USB</description></configItem></model></modelList><layoutList><layout><configItem><name>us</name><description>English</description></configItem><variantList><variant><configItem><name>dvorak</name><description>Dvorak</description></configItem></variant></variantList></layout><layout><configItem><name>de</name><description>German</description></configItem></layout></layoutList><optionList><group allowMultipleSelection="false"><configItem><name>caps</name><description>Caps Behavior</description></configItem><option><configItem><name>caps:escape</name><description>Escape</description></configItem></option><option><configItem><name>caps:ctrl</name><description>Control</description></configItem></option></group></optionList></xkbConfigRegistry>"#;
    #[test]
    fn reads_linked_variants_options_and_escaped_labels_without_a_dtd() {
        let r = parse(SAMPLE).unwrap();
        assert_eq!(r.models[0].label, "PC & USB");
        assert_eq!(r.layouts[0].variants[0].id, "dvorak");
        assert_eq!(r.layouts[1].choice.id, "de");
        assert!(!r.groups[0].multiple);
        assert_eq!(r.groups[0].options.len(), 2);
    }
    #[test]
    fn rejects_entities_bad_roots_duplicates_incomplete_and_oversized_xml() {
        for xml in ["<other/>","<xkbConfigRegistry>","<xkbConfigRegistry>&external;</xkbConfigRegistry>","<!DOCTYPE xkbConfigRegistry [<!ENTITY payload SYSTEM 'file:///etc/passwd'>]><xkbConfigRegistry>&payload;</xkbConfigRegistry>","<xkbConfigRegistry/><xkbConfigRegistry/>"] {assert!(parse(xml).is_err(),"{xml}");}
        assert!(parse(&SAMPLE.replace("<name>de</name>", "<name>us</name>")).is_err());
        assert!(parse(&" ".repeat(crate::MAX_DOCUMENT_BYTES + 1)).is_err());
        assert!(parse(&format!(
            "{}{}",
            "<xkbConfigRegistry>".repeat(17),
            "</xkbConfigRegistry>".repeat(17)
        ))
        .is_err());
    }
}
