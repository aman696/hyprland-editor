//! Ordered session declarations. Serialization never changes process environment,
//! launches commands, or installs permission rules.
use crate::{quote_lua, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    ConfigLoad,
    Start,
    Reloaded,
    Shutdown,
}
impl Trigger {
    pub const ALL: [Self; 4] = [
        Self::ConfigLoad,
        Self::Start,
        Self::Reloaded,
        Self::Shutdown,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::ConfigLoad => "Each configuration load",
            Self::Start => "Session starts",
            Self::Reloaded => "Configuration reloaded",
            Self::Shutdown => "Session shuts down",
        }
    }
    pub fn event(self) -> Option<&'static str> {
        match self {
            Self::ConfigLoad => None,
            Self::Start => Some("hyprland.start"),
            Self::Reloaded => Some("config.reloaded"),
            Self::Shutdown => Some("hyprland.shutdown"),
        }
    }
    pub fn from_event(event: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.event() == Some(event))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionType {
    Screencopy,
    Cursorpos,
    Plugin,
    Keyboard,
    InputCapture,
}
impl PermissionType {
    pub const ALL: [Self; 5] = [
        Self::Screencopy,
        Self::Cursorpos,
        Self::Plugin,
        Self::Keyboard,
        Self::InputCapture,
    ];
    pub fn text(self) -> &'static str {
        match self {
            Self::Screencopy => "screencopy",
            Self::Cursorpos => "cursorpos",
            Self::Plugin => "plugin",
            Self::Keyboard => "keyboard",
            Self::InputCapture => "input-capture",
        }
    }
    pub fn parse(text: &str) -> Option<Self> {
        if text == "keeb" {
            return Some(Self::Keyboard);
        }
        Self::ALL.into_iter().find(|t| t.text() == text)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    Ask,
    Allow,
    Deny,
}
impl PermissionMode {
    pub const ALL: [Self; 3] = [Self::Ask, Self::Allow, Self::Deny];
    pub fn text(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.text() == text)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Entry {
    Environment {
        name: String,
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dbus: Option<bool>,
    },
    Command {
        command: String,
        trigger: Trigger,
    },
    Permission {
        binary: String,
        permission: PermissionType,
        mode: PermissionMode,
    },
}
fn string(value: &str, empty: bool) -> Result<(), String> {
    if (!empty && value.is_empty()) || value.len() > 16 * 1024 || value.contains('\0') {
        return Err("Use a bounded string without NUL bytes".into());
    }
    Ok(())
}
impl Entry {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Environment { name, value, .. } => {
                string(name, false)?;
                string(value, true)?;
                if name.len() > 255
                    || !name.bytes().enumerate().all(|(i, c)| {
                        c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                    })
                {
                    return Err("Variable names must start with a letter or underscore and contain only letters, digits and underscores".into());
                }
            }
            Self::Command { command, .. } => string(command, false)?,
            Self::Permission { binary, .. } => string(binary, false)?,
        }
        Ok(())
    }
    pub fn label(&self) -> String {
        match self {
            Self::Environment { name, .. } => format!("Environment · {name}"),
            Self::Command { command, trigger } => format!(
                "{} · {}",
                trigger.label(),
                command.chars().take(60).collect::<String>()
            ),
            Self::Permission {
                binary,
                permission,
                mode,
            } => format!(
                "Permission · {} · {} · {binary}",
                permission.text(),
                mode.text()
            ),
        }
    }
    pub fn export_lua(&self) -> Result<String, String> {
        self.validate()?;
        Ok(match self {
            Self::Environment { name, value, dbus } => format!(
                "hl.env({}, {}{})",
                quote_lua(name),
                quote_lua(value),
                dbus.map(|v| format!(", {v}")).unwrap_or_default()
            ),
            Self::Command { command, trigger } => {
                let call = format!("hl.exec_cmd({})", quote_lua(command));
                if let Some(event) = trigger.event() {
                    format!("hl.on({}, function() {call} end)", quote_lua(event))
                } else {
                    call
                }
            }
            Self::Permission {
                binary,
                permission,
                mode,
            } => format!(
                "hl.permission({{ binary = {}, type = {}, mode = {} }})",
                quote_lua(binary),
                quote_lua(permission.text()),
                quote_lua(mode.text())
            ),
        })
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub entries: Vec<Entry>,
}
impl Draft {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn validate(&self, version: &str) -> Result<(), String> {
        if ![BASELINE, AUDITED_MAIN].contains(&version) {
            return Err("Unknown session schema version".into());
        }
        if self.entries.len() > 256 {
            return Err("Session draft exceeds 256 entries".into());
        }
        let mut total = 0usize;
        for entry in &self.entries {
            entry.validate()?;
            total = total.saturating_add(entry.export_lua()?.len() + 1);
        }
        if total > MAX_DOCUMENT_BYTES {
            return Err("Session declarations exceed 1 MiB".into());
        }
        Ok(())
    }
    pub fn export_lua(&self, version: &str) -> Result<String, String> {
        self.validate(version)?;
        let mut out = String::new();
        for entry in &self.entries {
            out.push_str(&entry.export_lua()?);
            out.push('\n');
        }
        Ok(out)
    }
}

pub fn schema_hash() -> String {
    crate::lua::source_hash(include_str!("../../../data/session.json"))
}
