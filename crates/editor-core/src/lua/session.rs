//! Static session declarations only. Command bodies are inspected, never run.
use super::*;
use crate::session::{Entry, PermissionMode, PermissionType, Trigger};
impl Projection<'_> {
    pub(super) fn session_declaration(&mut self, node: Node<'_>) -> Result<(), String> {
        let parsed = (|| {
            let api = node
                .child_by_field_name("name")
                .map(|n| text(n, self.source))
                .ok_or("Missing declaration name")?;
            let args = node
                .child_by_field_name("arguments")
                .map(named)
                .unwrap_or_default();
            let text_value = |n: Node<'_>| match evaluate(n, self.source, &self.env, 0) {
                Some(Expr::Text(s)) => Ok(s),
                _ => Err("Session values need known strings"),
            };
            let entry = match api {
                "hl.env" => {
                    if !(2..=3).contains(&args.len()) {
                        return Err("Environment needs a name, value and optional activation-environment flag".into());
                    }
                    let dbus = match args.get(2).map(|n| evaluate(*n, self.source, &self.env, 0)) {
                        None | Some(Some(Expr::Nil)) => None,
                        Some(Some(Expr::Bool(b))) => Some(b),
                        _ => return Err("Activation-environment flag needs a boolean".into()),
                    };
                    Entry::Environment {
                        name: text_value(args[0])?,
                        value: text_value(args[1])?,
                        dbus,
                    }
                }
                "hl.permission" => {
                    let (binary, kind, mode) = if args.len() == 1 {
                        let Some(Expr::Table(rows)) = evaluate(args[0], self.source, &self.env, 0)
                        else {
                            return Err("Permission needs a known table".into());
                        };
                        let mut fields = BTreeMap::new();
                        for (key, value) in rows {
                            let (Key::Name(key), Expr::Text(value)) = (key, value) else {
                                return Err(
                                    "Permission fields need known string keys and values".into()
                                );
                            };
                            if !["binary", "target", "type", "mode"].contains(&key.as_str())
                                || fields.insert(key, value).is_some()
                            {
                                return Err("Unknown or duplicate permission field".into());
                            }
                        }
                        let binary = fields
                            .remove("binary")
                            .or_else(|| fields.remove("target"))
                            .ok_or("Permission target missing")?;
                        if fields.contains_key("target") {
                            return Err("Ambiguous binary and target aliases".into());
                        }
                        (
                            binary,
                            fields.remove("type").ok_or("Permission type missing")?,
                            fields.remove("mode").ok_or("Permission mode missing")?,
                        )
                    } else if args.len() == 3 {
                        (
                            text_value(args[0])?,
                            text_value(args[1])?,
                            text_value(args[2])?,
                        )
                    } else {
                        return Err("Permission needs a target, type and mode".into());
                    };
                    Entry::Permission {
                        binary,
                        permission: PermissionType::parse(&kind)
                            .ok_or("Unknown permission type")?,
                        mode: PermissionMode::parse(&mode).ok_or("Unknown permission mode")?,
                    }
                }
                "hl.exec_cmd" => {
                    if args.is_empty()
                        || args.len() > 2
                        || args.get(1).is_some_and(|n| {
                            !matches!(evaluate(*n, self.source, &self.env, 0), Some(Expr::Nil))
                        })
                    {
                        return Err("Launch rules need their own graphical conversion; this command remains in source".into());
                    }
                    Entry::Command {
                        command: text_value(args[0])?,
                        trigger: Trigger::ConfigLoad,
                    }
                }
                "hl.on" => {
                    if args.len() != 2 {
                        return Err(
                            "A session hook needs an event and a literal command callback".into(),
                        );
                    }
                    let trigger = Trigger::from_event(&text_value(args[0])?)
                        .ok_or("This event needs the full visual workflow editor")?;
                    let function = args[1];
                    if function.kind() != "function_definition"
                        || function
                            .child_by_field_name("parameters")
                            .is_none_or(|n| !named(n).is_empty())
                    {
                        return Err("Session command callback must be a literal function without parameters".into());
                    }
                    let body = function
                        .child_by_field_name("body")
                        .map(named)
                        .unwrap_or_default();
                    if body.len() != 1
                        || body[0].kind() != "function_call"
                        || body[0]
                            .child_by_field_name("name")
                            .is_none_or(|n| text(n, self.source) != "hl.exec_cmd")
                    {
                        return Err("This callback needs the full visual workflow editor; its source is retained".into());
                    }
                    let command_args = body[0]
                        .child_by_field_name("arguments")
                        .map(named)
                        .unwrap_or_default();
                    if command_args.len() != 1 {
                        return Err("Session callback launch rules remain unresolved".into());
                    }
                    Entry::Command {
                        command: text_value(command_args[0])?,
                        trigger,
                    }
                }
                _ => return Err("Unknown session declaration".into()),
            };
            entry.validate()?;
            if let Entry::Permission { binary, .. } = &entry {
                crate::regex::check(&crate::regex::Sample {
                    pattern: binary.clone(),
                    text: String::new(),
                    negated: false,
                })?;
            }
            Ok::<_, String>(entry)
        })();
        match parsed {
            Ok(entry) => {
                if self.report.session.entries.len() >= 256 {
                    return Err("Session declaration count exceeds 256".into());
                }
                self.report.session.entries.push(entry);
            }
            Err(reason) => {
                self.report.opaque_effects = true;
                self.api_available = false;
                self.diagnostic(node, &reason);
            }
        }
        Ok(())
    }
}
