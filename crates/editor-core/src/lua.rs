//! Static Lua source projection and byte-preserving copy preparation. Never runs Lua.
pub mod config_inspection;
pub mod config_table;
mod dependencies;
pub mod registration;
pub mod replay;
pub mod scalar_codec;
use crate::{parse_color, Draft, Setting, Value, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
pub use dependencies::{
    inspect as inspect_dependencies, Barrier as DependencyBarrier,
    BarrierKind as DependencyBarrierKind, Binding as DependencyBinding,
    Inventory as DependencyInventory, Loader as DependencyLoader, Site as DependencySite,
    SiteScope as DependencyScope,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ops::{ControlFlow, Range},
    time::{Duration, Instant},
};
use tree_sitter::{Node, ParseOptions, Parser};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub line: usize,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub id: String,
    pub span: Range<usize>,
    pub value: Option<Value>,
    pub editable: bool,
    pub call: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LuaReport {
    pub source_sha256: String,
    pub draft: Draft,
    #[serde(default)]
    pub monitors: Vec<crate::monitor::Rule>,
    #[serde(default)]
    pub devices: Vec<crate::device::Spec>,
    #[serde(default)]
    pub bindings: crate::binding::Draft,
    #[serde(default)]
    pub workspace_rules: Vec<crate::workspace_rule::Rule>,
    #[serde(default)]
    pub rules: Vec<crate::rule::Rule>,
    #[serde(default)]
    pub animations: crate::animation::Draft,
    #[serde(default)]
    pub gestures: crate::gesture::Draft,
    pub declarations: Vec<Declaration>,
    pub diagnostics: Vec<Diagnostic>,
    pub opaque_effects: bool,
}
#[derive(Clone, Debug)]
pub struct LuaDocument {
    pub source: String,
    pub report: LuaReport,
}
pub fn source_hash(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}
#[derive(Clone, Debug)]
enum Expr {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Table(Vec<(Key, Expr)>),
    Action(Box<crate::action::Action>),
}
#[derive(Clone, Debug, PartialEq)]
enum Key {
    Name(String),
    Index(usize),
}
impl Expr {
    fn number(&self) -> Option<f64> {
        match self {
            Self::Int(v) => Some(*v as f64),
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }
    fn truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Bool(false))
    }
}
fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}
fn named(node: Node<'_>) -> Vec<Node<'_>> {
    let mut c = node.walk();
    node.named_children(&mut c)
        .filter(|n| n.kind() != "comment")
        .collect()
}

fn decode_string(raw: &str) -> Option<String> {
    if let Some(rest) = raw.strip_prefix('[') {
        let second = rest.find('[')? + 1;
        if !raw[1..second].chars().all(|c| c == '=') {
            return None;
        }
        let delimiter = format!("]{}]", &raw[1..second]);
        let content = raw.get(second + 1..)?.strip_suffix(&delimiter)?;
        // Lua converts CR, LF, CRLF and LFCR into one LF per newline.
        // Normalize in one pass: chained replacements mishandle CRLFCR.
        let mut chars = content.chars().peekable();
        let mut normalized = String::new();
        while let Some(c) = chars.next() {
            if c == '\r' || c == '\n' {
                if chars
                    .peek()
                    .is_some_and(|next| (*next == '\r' || *next == '\n') && *next != c)
                {
                    chars.next();
                }
                normalized.push('\n');
            } else {
                normalized.push(c);
            }
        }
        return Some(normalized.strip_prefix('\n').unwrap_or(&normalized).into());
    }
    let quote = raw.as_bytes().first()?;
    if ![b'\'', b'"'].contains(quote) || raw.as_bytes().last()? != quote {
        return None;
    }
    let input = &raw.as_bytes()[1..raw.len() - 1];
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        if input[i] != b'\\' {
            out.push(input[i]);
            i += 1;
            continue;
        }
        i += 1;
        let c = *input.get(i)?;
        i += 1;
        match c {
            b'a' => out.push(7),
            b'b' => out.push(8),
            b'f' => out.push(12),
            b'n' => out.push(10),
            b'r' => out.push(13),
            b't' => out.push(9),
            b'v' => out.push(11),
            b'\\' | b'\'' | b'"' => out.push(c),
            b'\n' => out.push(b'\n'),
            b'\r' => {
                if input.get(i) == Some(&b'\n') {
                    i += 1;
                }
                out.push(b'\n');
            }
            b'z' => {
                while input.get(i).is_some_and(u8::is_ascii_whitespace) {
                    i += 1;
                }
            }
            b'x' => {
                let s = std::str::from_utf8(input.get(i..i + 2)?).ok()?;
                out.push(u8::from_str_radix(s, 16).ok()?);
                i += 2;
            }
            b'0'..=b'9' => {
                let start = i - 1;
                while i - start < 3 && input.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
                out.push(
                    std::str::from_utf8(&input[start..i])
                        .ok()?
                        .parse::<u8>()
                        .ok()?,
                );
            }
            // Unknown escapes and Unicode escapes stay unresolved rather than being guessed.
            _ => return None,
        }
    }
    String::from_utf8(out).ok()
}
fn evaluate(
    node: Node<'_>,
    source: &str,
    env: &BTreeMap<String, Expr>,
    depth: usize,
) -> Option<Expr> {
    if depth > 32 {
        return None;
    }
    let raw = text(node, source);
    let eval = |n| evaluate(n, source, env, depth + 1);
    let result = match node.kind() {
        "nil" => Expr::Nil,
        "true" => Expr::Bool(true),
        "false" => Expr::Bool(false),
        "number" => {
            if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
                Expr::Int(i64::from_str_radix(hex, 16).ok()?)
            } else if let Ok(v) = raw.parse::<i64>() {
                Expr::Int(v)
            } else {
                Expr::Float(raw.parse().ok()?)
            }
        }
        "string" => Expr::Text(decode_string(raw)?),
        "identifier" => env.get(raw)?.clone(),
        "parenthesized_expression" => eval(named(node).first().copied()?)?,
        "unary_expression" => {
            let value = eval(node.child_by_field_name("operand")?)?;
            match text(node.child_by_field_name("operator")?, source) {
                "-" => match value {
                    Expr::Int(v) => Expr::Int(v.checked_neg()?),
                    _ => Expr::Float(-value.number()?),
                },
                "not" => Expr::Bool(!value.truthy()),
                _ => return None,
            }
        }
        "binary_expression" => {
            let left = eval(node.child_by_field_name("left")?)?;
            let op = text(node.child_by_field_name("operator")?, source);
            if op == "and" && !left.truthy() || op == "or" && left.truthy() {
                return Some(left);
            }
            let right = eval(node.child_by_field_name("right")?)?;
            match op {
                "and" | "or" => right,
                ".." => {
                    let (Expr::Text(a), Expr::Text(b)) = (left, right) else {
                        return None;
                    };
                    if a.len() + b.len() > 16384 {
                        return None;
                    }
                    Expr::Text(a + &b)
                }
                "+" | "-" | "*" => {
                    if let (Expr::Int(a), Expr::Int(b)) = (&left, &right) {
                        Expr::Int(match op {
                            "+" => a.checked_add(*b)?,
                            "-" => a.checked_sub(*b)?,
                            _ => a.checked_mul(*b)?,
                        })
                    } else {
                        let a = left.number()?;
                        let b = right.number()?;
                        Expr::Float(match op {
                            "+" => a + b,
                            "-" => a - b,
                            _ => a * b,
                        })
                    }
                }
                "/" => {
                    let b = right.number()?;
                    if b == 0. {
                        return None;
                    }
                    Expr::Float(left.number()? / b)
                }
                _ => return None,
            }
        }
        "table_constructor" => {
            let fields = named(node);
            if fields.len() > 512 {
                return None;
            }
            let mut array_index = 1;
            let mut out = vec![];
            for field in fields {
                let value = eval(field.child_by_field_name("value")?)?;
                let key = if let Some(n) = field.child_by_field_name("name") {
                    if n.kind() == "identifier"
                        && !text(field, source).trim_start().starts_with('[')
                    {
                        Key::Name(text(n, source).into())
                    } else {
                        match eval(n)? {
                            Expr::Text(s) => Key::Name(s),
                            Expr::Int(i) if i > 0 => Key::Index(i.try_into().ok()?),
                            _ => return None,
                        }
                    }
                } else {
                    let key = Key::Index(array_index);
                    array_index += 1;
                    key
                };
                // Keep ambiguous duplicate constructor fields unresolved, including
                // tables reached through pure local variables.
                if out.iter().any(|(k, _)| *k == key) {
                    return None;
                }
                out.push((key, value));
            }
            Expr::Table(out)
        }
        _ => return None,
    };
    match &result {
        Expr::Float(n) if !n.is_finite() => None,
        Expr::Text(s) if s.len() > 16384 => None,
        _ => Some(result),
    }
}
fn array(table: &[(Key, Expr)], size: usize) -> Option<Vec<&Expr>> {
    (1..=size)
        .map(|i| {
            table
                .iter()
                .find(|(k, _)| *k == Key::Index(i))
                .map(|(_, v)| v)
        })
        .collect()
}
fn member<'a>(table: &'a [(Key, Expr)], name: &str) -> Option<&'a Expr> {
    table
        .iter()
        .find(|(k, _)| *k == Key::Name(name.into()))
        .map(|(_, v)| v)
}
fn integer(v: &Expr) -> Option<i64> {
    match v {
        Expr::Int(i) => Some(*i),
        Expr::Float(n) if n.fract() == 0. && n.abs() <= (1_u64 << 53) as f64 => Some(*n as i64),
        _ => None,
    }
}
fn as_color(v: &Expr) -> Option<u32> {
    match v {
        Expr::Text(s) => {
            if let Some(hex) = s.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
                if hex.len() != 8 {
                    return None;
                }
                return u32::from_str_radix(hex, 16).ok().map(|n| n.rotate_right(8));
            }
            if let Some(hex) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
                if hex.len() != 6 {
                    return None;
                }
                return u32::from_str_radix(hex, 16).ok().map(|n| n | 0xff000000);
            }
            parse_color(s).ok()
        }
        _ => None,
    }
}
fn typed(setting: &Setting, expr: Expr) -> Option<Value> {
    let value = match (setting.kind.as_str(), expr) {
        ("Bool", Expr::Bool(v)) => Value::Bool(v),
        ("Int", v) => Value::Int(integer(&v)?),
        ("Float", v) => Value::Float(v.number()?),
        ("String", Expr::Text(v)) => Value::String(v),
        ("Color", v) => Value::Color(as_color(&v)?),
        ("FontWeight", v) => Value::FontWeight(integer(&v)?),
        ("Vec2", Expr::Table(v)) => {
            let a = array(&v, 2)?;
            Value::Vec2([a[0].number()?, a[1].number()?])
        }
        ("Vec2", Expr::Text(v)) => {
            let a: Vec<f64> = v
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<_, _>>()
                .ok()?;
            Value::Vec2(a.try_into().ok()?)
        }
        ("CssGap", Expr::Table(v)) => {
            let mut gaps = [0; 4];
            for (i, name) in ["top", "right", "bottom", "left"].into_iter().enumerate() {
                if let Some(n) = member(&v, name) {
                    gaps[i] = integer(n)?;
                }
            }
            Value::CssGap(gaps)
        }
        ("CssGap", v) => Value::CssGap([integer(&v)?; 4]),
        ("Gradient", Expr::Text(v)) => Value::Gradient {
            colors: vec![as_color(&Expr::Text(v))?],
            angle: 0.,
        },
        ("Gradient", Expr::Table(v)) => {
            let Expr::Table(colors) = member(&v, "colors")? else {
                return None;
            };
            let c = array(colors, colors.len())?
                .into_iter()
                .map(as_color)
                .collect::<Option<Vec<_>>>()?;
            let angle = member(&v, "angle").map(Expr::number).unwrap_or(Some(0.))?;
            Value::Gradient { colors: c, angle }
        }
        _ => return None,
    };
    setting.validate(&value).ok()?;
    Some(value)
}
fn is_literal(node: Node<'_>) -> bool {
    literal_depth(node, 0)
}
fn literal_depth(node: Node<'_>, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    match node.kind() {
        "number" | "string" | "true" | "false" => true,
        "unary_expression" => named(node).iter().all(|n| literal_depth(*n, depth + 1)),
        "table_constructor" => named(node).iter().all(|f| {
            f.child_by_field_name("value")
                .is_some_and(|v| literal_depth(v, depth + 1))
                && f.child_by_field_name("name").is_none_or(|n| {
                    n.kind() == "identifier" || n.kind() == "string" || n.kind() == "number"
                })
        }),
        _ => false,
    }
}
struct Projection<'a> {
    source: &'a str,
    catalogue: &'a [Setting],
    env: BTreeMap<String, Expr>,
    report: LuaReport,
    api_available: bool,
    call: usize,
    submap: Option<String>,
    animation_beziers: std::collections::BTreeSet<String>,
    animation_springs: std::collections::BTreeSet<String>,
}
impl Projection<'_> {
    fn diagnostic(&mut self, node: Node<'_>, message: &str) {
        if self.report.diagnostics.len() < 256 {
            self.report.diagnostics.push(Diagnostic {
                line: node.start_position().row + 1,
                message: message.into(),
            });
        }
    }
    fn fields(&mut self, node: Node<'_>, prefix: &str, depth: usize) -> Result<(), String> {
        if depth > 16 {
            return Err("Config nesting exceeds 16 levels".into());
        }
        for field in named(node) {
            let Some(key_node) = field.child_by_field_name("name") else {
                if let Some(value) = field.child_by_field_name("value") {
                    self.ignored_expression(value);
                }
                self.diagnostic(field, "Non-string configuration key ignored");
                continue;
            };
            let key = if key_node.kind() == "identifier"
                && !text(field, self.source).trim_start().starts_with('[')
            {
                text(key_node, self.source).to_string()
            } else if let Some(Expr::Text(s)) = evaluate(key_node, self.source, &self.env, 0) {
                s
            } else {
                self.report.opaque_effects = true;
                self.api_available = false;
                self.diagnostic(field, "Computed configuration key is unresolved");
                continue;
            };
            // The pinned hlConfig constructs std::string(lua_tostring(key)),
            // so the API key ends at the first NUL even though Lua retains the
            // complete byte string. Keep source bytes intact when editing.
            let key = key.split('\0').next().unwrap_or_default().to_string();
            if key.len() > 256 {
                if let Some(value) = field.child_by_field_name("value") {
                    self.ignored_expression(value);
                }
                self.diagnostic(field, "Configuration key is too long");
                continue;
            }
            let full = if prefix.is_empty() {
                key
            } else {
                format!("{prefix}.{key}")
            };
            let Some(value_node) = field.child_by_field_name("value") else {
                continue;
            };
            let setting = self
                .catalogue
                .iter()
                .find(|s| s.version == self.report.draft.version && s.lua_key == full);
            if let Some(setting) = setting {
                let id = setting.id.clone();
                let value =
                    evaluate(value_node, self.source, &self.env, 0).and_then(|v| typed(setting, v));
                if value.is_none() {
                    self.report.opaque_effects = true;
                    self.api_available = false;
                    self.diagnostic(
                        value_node,
                        "Setting expression or value is unresolved; it was not executed",
                    );
                    self.report.draft.values.remove(&id);
                } else {
                    self.report
                        .draft
                        .values
                        .insert(id.clone(), value.clone().unwrap());
                }
                if self.report.declarations.len() >= 2048 {
                    return Err("Too many configuration declarations".into());
                }
                self.report.declarations.push(Declaration {
                    id,
                    span: value_node.byte_range(),
                    value,
                    editable: is_literal(value_node),
                    call: self.call,
                });
            } else if value_node.kind() == "table_constructor" {
                self.fields(value_node, &full, depth + 1)?;
            } else {
                self.diagnostic(field, "Setting is outside the selected catalogue");
                // Ignoring a key does not make its expression inert at runtime. Lua
                // evaluates all table values, including calls behind unknown keys.
                self.ignored_expression(value_node);
            }
        }
        Ok(())
    }
    fn ignored_expression(&mut self, node: Node<'_>) {
        if evaluate(node, self.source, &self.env, 0).is_none() {
            self.report.opaque_effects = true;
            self.api_available = false;
            self.diagnostic(node,"Unknown setting expression may change bindings; subsequent API calls remain unresolved");
        }
    }
    fn monitor(&mut self, node: Node<'_>) -> Result<(), String> {
        let args = node
            .child_by_field_name("arguments")
            .map(named)
            .unwrap_or_default();
        let parsed = (|| -> Result<crate::monitor::Rule, String> {
            if args.len() != 1 || args[0].kind() != "table_constructor" {
                return Err("Monitor call needs a static table".into());
            }
            let Some(Expr::Table(entries)) = evaluate(args[0], self.source, &self.env, 0) else {
                return Err("Monitor expression is unresolved; it was not executed".into());
            };
            let mut rule = crate::monitor::Rule::new(&format!(
                "source-monitor-{}",
                self.report.monitors.len()
            ));
            let mut seen = std::collections::BTreeSet::new();
            let fields = crate::monitor::catalogue(&self.report.draft.version);
            let mut output_present = false;
            for (key, expr) in entries {
                let Key::Name(key) = key else {
                    return Err("Monitor key must be a field name".into());
                };
                if !seen.insert(key.clone()) {
                    return Err("Duplicate monitor field is ambiguous".into());
                }
                if key == "output" {
                    let Expr::Text(output) = expr else {
                        return Err("Monitor output must be a static string".into());
                    };
                    rule.output = if output.is_empty() {
                        crate::monitor::Output::Fallback
                    } else if let Some(name) = output.strip_prefix("desc:") {
                        crate::monitor::Output::Description(name.into())
                    } else {
                        crate::monitor::Output::Connector(output)
                    };
                    output_present = true;
                } else {
                    let setting = fields
                        .iter()
                        .find(|s| s.lua_key == key)
                        .ok_or("Monitor field is outside the audited snapshot")?;
                    // Unlike scalar gaps, monitor reserved areas permit signed values.
                    let value = if setting.kind == "CssGap" {
                        match expr {
                            Expr::Table(ref sides) => {
                                let mut gaps = [0; 4];
                                if sides.iter().any(|(key,_)| !matches!(key,Key::Name(n) if ["top","right","bottom","left"].contains(&n.as_str()))) {
                                    return Err("Unknown reserved-area side".into());
                                }
                                let mut side_keys = std::collections::BTreeSet::new();
                                if sides
                                    .iter()
                                    .any(|(key, _)| !side_keys.insert(format!("{key:?}")))
                                {
                                    return Err("Duplicate reserved-area side".into());
                                }
                                for (i, name) in
                                    ["top", "right", "bottom", "left"].iter().enumerate()
                                {
                                    if let Some(v) = member(sides, name) {
                                        gaps[i] =
                                            integer(v).ok_or("Reserved area needs integers")?;
                                    }
                                }
                                Value::CssGap(gaps)
                            }
                            expr => Value::CssGap(
                                [integer(&expr).ok_or("Reserved area needs integers")?; 4],
                            ),
                        }
                    } else {
                        typed(setting, expr).ok_or("Monitor field has an invalid type or value")?
                    };
                    rule.fields.insert(key, value);
                }
            }
            if !output_present {
                return Err("Monitor output selector is missing".into());
            }
            rule.validate(&self.report.draft.version)?;
            Ok(rule)
        })();
        match parsed {
            Ok(rule) => {
                if self.report.monitors.len() >= crate::monitor::MAX_RULES {
                    return Err("Too many monitor declarations".into());
                }
                self.report.monitors.push(rule);
            }
            Err(error) => {
                self.report.opaque_effects = true;
                self.diagnostic(node, &error);
                // Unknown evaluation can change the binding. Do not infer later calls.
                if args
                    .iter()
                    .any(|n| evaluate(*n, self.source, &self.env, 0).is_none())
                {
                    self.api_available = false;
                }
            }
        }
        Ok(())
    }
    fn device(&mut self, node: Node<'_>) -> Result<(), String> {
        let args = node
            .child_by_field_name("arguments")
            .map(named)
            .unwrap_or_default();
        let parsed = (|| -> Result<crate::device::Spec, String> {
            if args.len() != 1 || args[0].kind() != "table_constructor" {
                return Err("Device call needs a static table".into());
            }
            let Some(Expr::Table(entries)) = evaluate(args[0], self.source, &self.env, 0) else {
                return Err("Device expression is unresolved; it was not executed".into());
            };
            let fields = crate::device::catalogue(&self.report.draft.version);
            let mut spec = crate::device::Spec::new(
                &format!("source-device-{}", self.report.devices.len()),
                "",
            );
            for (key, expr) in entries {
                let Key::Name(key) = key else {
                    return Err("Device keys need field names".into());
                };
                if key == "name" {
                    let Expr::Text(name) = expr else {
                        return Err("Device name needs a static string".into());
                    };
                    spec.name = name;
                } else {
                    let setting = fields
                        .iter()
                        .find(|s| s.lua_key == key)
                        .ok_or("Unknown audited device field")?;
                    let value = typed(setting, expr).ok_or("Invalid device field type or value")?;
                    spec.fields.insert(key, value);
                }
            }
            spec.validate(&self.report.draft.version)?;
            Ok(spec)
        })();
        match parsed {
            Ok(spec) => {
                if self.report.devices.len() >= crate::device::MAX_DEVICES {
                    return Err("Too many device declarations".into());
                }
                self.report.devices.push(spec);
            }
            Err(e) => {
                self.report.opaque_effects = true;
                self.diagnostic(node, &e);
                if args
                    .iter()
                    .any(|n| evaluate(*n, self.source, &self.env, 0).is_none())
                {
                    self.api_available = false;
                }
            }
        }
        Ok(())
    }
    fn gesture(&mut self, node: Node<'_>) -> Result<(), String> {
        use crate::gesture::{Callbacks, GestureAction, Spec};
        let args = node
            .child_by_field_name("arguments")
            .map(named)
            .unwrap_or_default();
        let parsed = (|| -> Result<Spec, String> {
            if args.len() != 1 || args[0].kind() != "table_constructor" {
                return Err("Gesture requires one explicit static table".into());
            }
            let mut fields = BTreeMap::new();
            for field in named(args[0]) {
                let key = field
                    .child_by_field_name("name")
                    .ok_or("Gesture fields must be named")?;
                let key = if key.kind() == "identifier"
                    && !text(field, self.source).trim_start().starts_with('[')
                {
                    text(key, self.source).to_string()
                } else if let Some(Expr::Text(s)) = evaluate(key, self.source, &self.env, 0) {
                    s
                } else {
                    return Err("Gesture key is unresolved".into());
                };
                if fields
                    .insert(
                        key,
                        field
                            .child_by_field_name("value")
                            .ok_or("Gesture value missing")?,
                    )
                    .is_some()
                {
                    return Err("Duplicate gesture field".into());
                }
            }
            let take = |fields: &mut BTreeMap<String, Node<'_>>,
                        key: &str|
             -> Result<Option<Value>, String> {
                let Some(node) = fields.remove(key) else {
                    return Ok(None);
                };
                Ok(match evaluate(node, self.source, &self.env, 0) {
                    Some(Expr::Nil) => None,
                    Some(Expr::Bool(v)) => Some(Value::Bool(v)),
                    Some(Expr::Int(v)) => Some(Value::Int(v)),
                    Some(Expr::Float(v)) => Some(Value::Float(v)),
                    Some(Expr::Text(v)) => Some(Value::String(v)),
                    _ => return Err(format!("Gesture {key} is unresolved")),
                })
            };
            let fingers = take(&mut fields, "fingers")?.ok_or("Gesture fingers required")?;
            let Some(Value::String(direction)) = take(&mut fields, "direction")? else {
                return Err("Gesture direction requires text".into());
            };
            let action_node = fields.remove("action").ok_or("Gesture action required")?;
            let action = if action_node.kind() == "function_definition" {
                GestureAction::LegacyFinish(self.callback_workflow(action_node, true)?)
            } else if action_node.kind() == "table_constructor" {
                let mut callbacks = Callbacks::default();
                let mut seen = std::collections::BTreeSet::new();
                for field in named(action_node) {
                    let key = field
                        .child_by_field_name("name")
                        .ok_or("Callback phase must be named")?;
                    let key = if key.kind() == "identifier"
                        && !text(field, self.source).trim_start().starts_with('[')
                    {
                        text(key, self.source).to_string()
                    } else if let Some(Expr::Text(s)) = evaluate(key, self.source, &self.env, 0) {
                        s
                    } else {
                        return Err("Callback phase is unresolved".into());
                    };
                    if !seen.insert(key.clone()) {
                        return Err("Duplicate gesture callback phase".into());
                    }
                    let target = match key.as_str() {
                        "start" => &mut callbacks.start,
                        "update" => &mut callbacks.update,
                        "finish" => &mut callbacks.finish,
                        _ => return Err("Unmodeled gesture callback phase".into()),
                    };
                    let value = field
                        .child_by_field_name("value")
                        .ok_or("Callback value missing")?;
                    if value.kind() != "nil" {
                        *target = Some(self.callback_workflow(value, true)?);
                    }
                }
                GestureAction::Callbacks(callbacks)
            } else {
                let Some(Expr::Text(name)) = evaluate(action_node, self.source, &self.env, 0)
                else {
                    return Err("Gesture action is unresolved".into());
                };
                GestureAction::Builtin(name)
            };
            let spec = Spec {
                id: format!("source-gesture-{}", self.report.gestures.declarations.len()),
                fingers,
                direction,
                action,
                mods: take(&mut fields, "mods")?,
                scale: take(&mut fields, "scale")?,
                disable_inhibit: take(&mut fields, "disable_inhibit")?,
                mode: take(&mut fields, "mode")?,
                zoom_level: take(&mut fields, "zoom_level")?,
                workspace_name: take(&mut fields, "workspace_name")?,
            };
            if !fields.is_empty() {
                return Err("Unmodeled gesture fields remain in source".into());
            }
            spec.validate(&self.report.draft.version)?;
            if self.report.gestures.declarations.len() >= crate::gesture::MAX_GESTURES {
                return Err("Too many gesture declarations".into());
            }
            Ok(spec)
        })();
        match parsed {
            Ok(spec) => self.report.gestures.declarations.push(spec),
            Err(e) => {
                self.report.opaque_effects = true;
                self.diagnostic(node, &e);
                // Literal function creation is inert; unresolved non-function argument expressions can run at load.
                fn creation_is_static(
                    node: Node<'_>,
                    source: &str,
                    env: &BTreeMap<String, Expr>,
                ) -> bool {
                    if node.kind() == "function_definition" {
                        return true;
                    }
                    if node.kind() == "table_constructor" {
                        return named(node).iter().all(|f| {
                            f.child_by_field_name("value")
                                .is_some_and(|v| creation_is_static(v, source, env))
                                && f.child_by_field_name("name").is_none_or(|k| {
                                    k.kind() == "identifier"
                                        || evaluate(k, source, env, 0).is_some()
                                })
                        });
                    }
                    evaluate(node, source, env, 0).is_some()
                }
                if args
                    .iter()
                    .any(|n| !creation_is_static(*n, self.source, &self.env))
                {
                    self.api_available = false;
                }
            }
        }
        Ok(())
    }
    fn animation(&mut self, node: Node<'_>, curve: bool) -> Result<(), String> {
        use crate::animation::{Curve, CurveKind, Declaration, Node as AnimationNode, Operation};
        let args = node
            .child_by_field_name("arguments")
            .map(named)
            .unwrap_or_default();
        let parsed = (|| -> Result<Operation, String> {
            let literal = |value: Expr| match value {
                Expr::Bool(v) => Some(Value::Bool(v)),
                Expr::Int(v) => Some(Value::Int(v)),
                Expr::Float(v) => Some(Value::Float(v)),
                Expr::Text(v) => Some(Value::String(v)),
                _ => None,
            };
            if curve {
                if args.len() != 2 {
                    return Err("Curve requires a literal name and one static table".into());
                }
                let Some(Expr::Text(name)) = evaluate(args[0], self.source, &self.env, 0) else {
                    return Err("Curve name coercion is unresolved".into());
                };
                let Some(Expr::Table(entries)) = evaluate(args[1], self.source, &self.env, 0)
                else {
                    return Err("Curve expression is unresolved".into());
                };
                let mut fields = BTreeMap::new();
                for (key, value) in entries {
                    let Key::Name(key) = key else {
                        return Err("Curve fields must be named".into());
                    };
                    if fields.insert(key, value).is_some() {
                        return Err("Duplicate curve field".into());
                    }
                }
                let Some(Expr::Text(kind)) = fields.remove("type") else {
                    return Err("Curve type is required".into());
                };
                let shape = match kind.as_str() {
                    "bezier" => {
                        let Some(Expr::Table(points)) = fields.remove("points") else {
                            return Err("Bézier points are required".into());
                        };
                        if points.len() != 2 {
                            return Err("Bézier needs two points".into());
                        }
                        let p =
                            array(&points, 2).ok_or("Bézier points must be a contiguous array")?;
                        let point = |p: &Expr| -> Result<[Value; 2], String> {
                            let Expr::Table(p) = p else {
                                return Err("Bézier point needs two coordinates".into());
                            };
                            if p.len() != 2 {
                                return Err("Bézier point needs two coordinates".into());
                            }
                            let p = array(p, 2).ok_or("Bézier coordinate array is ambiguous")?;
                            Ok([
                                literal(p[0].clone()).ok_or("Unresolved coordinate")?,
                                literal(p[1].clone()).ok_or("Unresolved coordinate")?,
                            ])
                        };
                        CurveKind::Bezier {
                            points: [point(p[0])?, point(p[1])?],
                        }
                    }
                    "spring" => {
                        let stiffness = literal(
                            fields
                                .remove("stiffness")
                                .ok_or("Spring stiffness is required")?,
                        )
                        .ok_or("Unresolved stiffness")?;
                        let mass = literal(fields.remove("mass").ok_or("Spring mass is required")?)
                            .ok_or("Unresolved mass")?;
                        let alias = self.report.draft.version == crate::BASELINE
                            || !fields.contains_key("damping")
                            || matches!(fields.get("damping"), Some(Expr::Nil));
                        if alias {
                            fields.remove("damping");
                        }
                        let damping = literal(
                            fields
                                .remove(if alias { "dampening" } else { "damping" })
                                .ok_or("Spring damping is required")?,
                        )
                        .ok_or("Unresolved damping")?;
                        CurveKind::Spring {
                            stiffness,
                            damping,
                            mass,
                            dampening_alias: alias,
                        }
                    }
                    _ => return Err("Curve type is outside the selected API".into()),
                };
                if !fields.is_empty() {
                    return Err("Unmodeled curve fields remain in the source".into());
                }
                Ok(Operation::Curve(Curve { name, shape }))
            } else {
                if args.len() != 1 {
                    return Err("Animation requires one static table".into());
                }
                let Some(Expr::Table(entries)) = evaluate(args[0], self.source, &self.env, 0)
                else {
                    return Err("Animation expression is unresolved".into());
                };
                let mut fields = BTreeMap::new();
                for (key, value) in entries {
                    let Key::Name(key) = key else {
                        return Err("Animation fields must be named".into());
                    };
                    if fields.insert(key, value).is_some() {
                        return Err("Duplicate animation field".into());
                    }
                }
                let Some(Expr::Text(leaf)) = fields.remove("leaf") else {
                    return Err("Animation leaf is required".into());
                };
                let enabled = literal(
                    fields
                        .remove("enabled")
                        .ok_or("Animation enabled is required")?,
                )
                .ok_or("Unresolved enabled value")?;
                let speed = match fields.remove("speed") {
                    None | Some(Expr::Nil) => None,
                    Some(value) => Some(literal(value).ok_or("Unresolved speed")?),
                };
                let mut string = |key: &str| -> Result<Option<String>, String> {
                    match fields.remove(key) {
                        None | Some(Expr::Nil) => Ok(None),
                        Some(Expr::Text(s)) => Ok(Some(s)),
                        _ => Err(format!("Unresolved animation {key} coercion")),
                    }
                };
                let bezier = string("bezier")?;
                let spring = string("spring")?;
                let style = string("style")?;
                if !fields.is_empty() {
                    return Err("Unmodeled animation fields remain in source".into());
                }
                Ok(Operation::Node(AnimationNode {
                    leaf,
                    enabled,
                    speed,
                    bezier,
                    spring,
                    style,
                }))
            }
        })();
        let parsed = parsed.and_then(|operation| {
            if self.report.animations.declarations.len() >= crate::animation::MAX_OPERATIONS {
                return Err("Too many animation declarations".into());
            }
            match &operation {
                Operation::Curve(c) => c.validate()?,
                Operation::Node(n) => {
                    n.validate(&self.report.draft.version)?;
                    if crate::animation::enabled(&n.enabled)? {
                        if let Some(name) = &n.bezier {
                            if !self.animation_beziers.contains(name) {
                                return Err("Bézier reference is not yet registered".into());
                            }
                        } else if let Some(name) = &n.spring {
                            if !self.animation_springs.contains(name) {
                                return Err("Spring reference is not yet registered".into());
                            }
                        }
                    }
                }
            }
            Ok(Declaration {
                id: format!(
                    "source-animation-{}",
                    self.report.animations.declarations.len()
                ),
                operation,
            })
        });
        match parsed {
            Ok(declaration) => {
                if let Operation::Curve(c) = &declaration.operation {
                    match c.shape {
                        CurveKind::Bezier { .. } => {
                            self.animation_beziers.insert(c.name.clone());
                        }
                        CurveKind::Spring { .. } => {
                            self.animation_springs.insert(c.name.clone());
                        }
                    }
                }
                self.report.animations.declarations.push(declaration);
            }
            Err(e) => {
                self.report.opaque_effects = true;
                self.diagnostic(node, &e);
                if args
                    .iter()
                    .any(|n| evaluate(*n, self.source, &self.env, 0).is_none())
                {
                    self.api_available = false;
                }
            }
        }
        Ok(())
    }
    fn match_effect_rule(&mut self, node: Node<'_>, kind: crate::rule::Kind) -> Result<(), String> {
        use crate::rule::{Effect, EffectValue, Geometry, Predicate, Rule};
        let args = node
            .child_by_field_name("arguments")
            .map(named)
            .unwrap_or_default();
        let parsed = (|| -> Result<Rule, String> {
            if args.len() != 1 {
                return Err("Rule needs exactly one static table".into());
            }
            let Some(Expr::Table(entries)) = evaluate(args[0], self.source, &self.env, 0) else {
                return Err("Rule expression is unresolved".into());
            };
            let mut rule = Rule::new(&format!("source-rule-{}", self.report.rules.len()), kind);
            let mut keys = std::collections::BTreeSet::new();
            let fields = crate::rule::catalogue(kind, &self.report.draft.version);
            for (key, value) in entries {
                let Key::Name(key) = key else {
                    return Err("Rule requires named fields".into());
                };
                if !keys.insert(key.clone()) {
                    return Err("Duplicate rule table field is ambiguous".into());
                }
                match key.as_str() {
                    "name" => match value {
                        Expr::Text(v) => rule.name = Some(v),
                        Expr::Nil => {}
                        _ => {
                            return Err(
                                "Rule name coercion is unresolved; use a literal string".into()
                            )
                        }
                    },
                    "enabled" => match value {
                        Expr::Bool(v) => rule.enabled = Some(v),
                        Expr::Nil => {}
                        _ => return Err("Enabled needs a bool or unset".into()),
                    },
                    "match" => {
                        if matches!(value, Expr::Nil) {
                            continue;
                        }
                        let Expr::Table(matches) = value else {
                            return Err("Match needs a literal table".into());
                        };
                        for (key, value) in matches {
                            let Key::Name(property) = key else {
                                return Err("Match needs named properties".into());
                            };
                            let value = match value {
                                Expr::Text(s) => Value::String(s),
                                Expr::Bool(b) => Value::Bool(b),
                                Expr::Int(n) => Value::Int(n),
                                _ => return Err("Match value coercion is unresolved".into()),
                            };
                            rule.predicates.push(Predicate { property, value });
                        }
                    }
                    _ => {
                        let setting = fields.iter().find(|s| s.lua_key == key).ok_or(
                            "Unknown audited effect; plugin effect projection is unresolved",
                        )?;
                        let value = if setting.kind == "ExpressionVec2" {
                            let geometry = match value {
                                Expr::Text(s) => Geometry::Text(s),
                                Expr::Table(ref values) => {
                                    if values.len() != 2 {
                                        return Err("Geometry requires exactly two axes".into());
                                    }
                                    let axes = array(values, 2)
                                        .ok_or("Geometry requires exactly two axes")?;
                                    let axis = |value: &Expr| match value {
                                        Expr::Text(s) => Some(Value::String(s.clone())),
                                        Expr::Int(n) => Some(Value::Int(*n)),
                                        Expr::Float(n) => Some(Value::Float(*n)),
                                        _ => None,
                                    };
                                    Geometry::Axes([
                                        axis(axes[0]).ok_or("Unresolved geometry axis")?,
                                        axis(axes[1]).ok_or("Unresolved geometry axis")?,
                                    ])
                                }
                                _ => return Err("Geometry needs a string or two-axis array".into()),
                            };
                            EffectValue::Geometry(geometry)
                        } else if kind == crate::rule::Kind::Window && key == "border_color" {
                            match value {
                                Expr::Text(s) => EffectValue::BorderText(s),
                                v => {
                                    if let Expr::Table(ref entries) = v {
                                        let mut keys = std::collections::BTreeSet::new();
                                        for (key, _) in entries {
                                            let Key::Name(key) = key else {
                                                return Err("Gradient requires named fields".into());
                                            };
                                            if !["colors", "angle"].contains(&key.as_str())
                                                || !keys.insert(key)
                                            {
                                                return Err("Unresolved gradient fields".into());
                                            }
                                        }
                                        if let Some(Expr::Table(colors)) = member(entries, "colors")
                                        {
                                            if colors.is_empty()
                                                || colors.len() > 32
                                                || array(colors, colors.len()).is_none()
                                            {
                                                return Err(
                                                    "Gradient needs a contiguous color array"
                                                        .into(),
                                                );
                                            }
                                        }
                                    }
                                    EffectValue::Literal(
                                        typed(setting, v).ok_or("Invalid border gradient")?,
                                    )
                                }
                            }
                        } else {
                            EffectValue::Literal(
                                typed(setting, value).ok_or("Invalid effect literal")?,
                            )
                        };
                        rule.effects.push(Effect { key, value });
                    }
                }
            }
            rule.validate(&self.report.draft.version)?;
            Ok(rule)
        })();
        match parsed {
            Ok(rule) => {
                if self.report.rules.len() >= crate::rule::MAX_RULES {
                    return Err("Too many rule declarations".into());
                }
                self.report.rules.push(rule);
            }
            Err(e) => {
                self.report.opaque_effects = true;
                self.diagnostic(node, &e);
                if args
                    .iter()
                    .any(|n| evaluate(*n, self.source, &self.env, 0).is_none())
                {
                    self.api_available = false;
                }
            }
        }
        Ok(())
    }
    fn workspace_rule(&mut self, node: Node<'_>) -> Result<(), String> {
        let args = node
            .child_by_field_name("arguments")
            .map(named)
            .unwrap_or_default();
        let parsed = (|| -> Result<crate::workspace_rule::Rule, String> {
            if args.len() != 1 {
                return Err("Workspace rule needs one static table".into());
            }
            let Some(Expr::Table(entries)) = evaluate(args[0], self.source, &self.env, 0) else {
                return Err("Workspace rule expression is unresolved".into());
            };
            let mut rule = crate::workspace_rule::Rule::new(&format!(
                "source-workspace-{}",
                self.report.workspace_rules.len()
            ));
            let mut selector_present = false;
            let fields = crate::workspace_rule::catalogue(&self.report.draft.version);
            for (key, value) in entries {
                let Key::Name(key) = key else {
                    return Err("Workspace rule needs named fields".into());
                };
                match key.as_str() {
                    "workspace" => {
                        let Expr::Text(v) = value else {
                            return Err("Workspace selector needs a static string".into());
                        };
                        rule.selector = v;
                        selector_present = true;
                    }
                    "enabled" => match value {
                        Expr::Bool(v) => rule.enabled = Some(v),
                        Expr::Nil => {}
                        _ => return Err("Enabled needs a bool or unset".into()),
                    },
                    "layout_opts" => {
                        let Expr::Table(options) = value else {
                            return Err("Layout options need a literal table".into());
                        };
                        for (key, value) in options {
                            let Key::Name(key) = key else {
                                return Err("Layout option names must be strings".into());
                            };
                            let value = match value {
                                Expr::Bool(v) => Value::Bool(v),
                                Expr::Int(v) => Value::Int(v),
                                Expr::Float(v) => Value::Float(v),
                                Expr::Text(v) => Value::String(v),
                                _ => return Err("Layout options need literal scalar values".into()),
                            };
                            rule.layout_options.insert(key, value);
                        }
                    }
                    _ => {
                        let setting = fields
                            .iter()
                            .find(|s| s.lua_key == key)
                            .ok_or("Unknown audited workspace field")?;
                        let value = typed(setting, value).ok_or("Invalid workspace field value")?;
                        rule.fields.insert(key, value);
                    }
                }
            }
            if !selector_present {
                return Err("Workspace selector field is required".into());
            }
            rule.validate(&self.report.draft.version)?;
            Ok(rule)
        })();
        match parsed {
            Ok(rule) => {
                if self.report.workspace_rules.len() >= crate::workspace_rule::MAX_RULES {
                    return Err("Too many workspace declarations".into());
                }
                self.report.workspace_rules.push(rule);
            }
            Err(e) => {
                self.report.opaque_effects = true;
                self.diagnostic(node, &e);
                if args
                    .iter()
                    .any(|n| evaluate(*n, self.source, &self.env, 0).is_none())
                {
                    self.api_available = false;
                }
            }
        }
        Ok(())
    }
    fn action(&self, node: Node<'_>, at_load: bool) -> Result<crate::action::Action, String> {
        if node.kind() == "identifier" {
            if let Some(Expr::Action(a)) = self.env.get(text(node, self.source)) {
                return Ok((**a).clone());
            }
        }
        if node.kind() != "function_call" {
            return Err(
                "Dispatcher must be a known constructor or captured static dispatcher".into(),
            );
        }
        let name = node
            .child_by_field_name("name")
            .ok_or("Dispatcher name missing")?;
        let constructor = text(name, self.source)
            .strip_prefix("hl.dsp.")
            .ok_or("Only pinned hl.dsp constructors are projected")?;
        let def = crate::action::find(constructor, &self.report.draft.version)
            .ok_or("Dispatcher is outside the selected snapshot")?;
        let args = node
            .child_by_field_name("arguments")
            .map(named)
            .unwrap_or_default();
        if args.len() > 1 {
            return Err("Dispatcher argument shape is unsupported".into());
        }
        let evaluated = args
            .first()
            .and_then(|n| evaluate(*n, self.source, &self.env, 0));
        if !args.is_empty() && evaluated.is_none() {
            return Err("Dispatcher arguments are unresolved; they were not executed".into());
        }
        for variant in &def.variants {
            let mut fields = BTreeMap::new();
            match (&evaluated, variant.shape.as_str()) {
                (None | Some(Expr::Nil), "none") => {}
                (None | Some(Expr::Nil), "table" | "position")
                    if !variant.table_required && !variant.fields.iter().any(|f| f.required) => {}
                (Some(Expr::Table(entries)), "table") => {
                    let mut valid = true;
                    for (key, expr) in entries {
                        let Key::Name(key) = key else {
                            valid = false;
                            break;
                        };
                        if matches!(expr, Expr::Nil) {
                            continue;
                        }
                        let value = match expr {
                            Expr::Text(s) => Value::String(s.clone()),
                            Expr::Bool(v) => Value::Bool(*v),
                            Expr::Int(n) => Value::Int(*n),
                            Expr::Float(n) => Value::Float(*n),
                            _ => {
                                valid = false;
                                break;
                            }
                        };
                        fields.insert(key.clone(), value);
                    }
                    if !valid {
                        continue;
                    }
                }
                (Some(expr), "position") => {
                    let value = match expr {
                        Expr::Text(s) => Value::String(s.clone()),
                        Expr::Int(n) => Value::Int(*n),
                        Expr::Float(n) => Value::Float(*n),
                        _ => continue,
                    };
                    fields.insert("argument".into(), value);
                }
                _ => continue,
            }
            for field in &variant.fields {
                if field.kind == "Number" {
                    if let Some(Value::Int(n)) = fields.get(&field.name) {
                        fields.insert(field.name.clone(), Value::Float(*n as f64));
                    }
                }
            }
            let action = crate::action::Action {
                constructor: constructor.into(),
                variant: variant.id.clone(),
                at_load,
                fields,
            };
            if action.validate(&self.report.draft.version).is_ok() {
                return Ok(action);
            }
        }
        Err("Dispatcher fields/branch are outside the reviewed argument shapes".into())
    }
    fn callback_workflow(
        &self,
        node: Node<'_>,
        allow_parameter: bool,
    ) -> Result<crate::gesture::Workflow, String> {
        if node.kind() != "function_definition" {
            return Err("Callback must be a literal function".into());
        }
        let params = node
            .child_by_field_name("parameters")
            .ok_or("Callback parameters missing")?;
        let params = named(params);
        let event_parameter = if params.is_empty() {
            None
        } else {
            if !allow_parameter || params.len() != 1 || params[0].kind() != "identifier" {
                return Err("Callback parameter shape needs the full workflow editor".into());
            }
            let name = text(params[0], self.source);
            if self.env.contains_key(name) {
                return Err("Callback parameter shadows a captured static value".into());
            }
            Some(name.to_string())
        };
        let mut actions = vec![];
        let statements = node
            .child_by_field_name("body")
            .map(named)
            .unwrap_or_default();
        for statement in statements {
            if ["comment", "empty_statement"].contains(&statement.kind()) {
                continue;
            }
            if statement.kind() != "function_call"
                || statement
                    .child_by_field_name("name")
                    .is_none_or(|n| text(n, self.source) != "hl.dispatch")
            {
                return Err(
                    "Callback contains opaque workflow statements; it was not executed".into(),
                );
            }
            let args = statement
                .child_by_field_name("arguments")
                .map(named)
                .unwrap_or_default();
            if args.len() != 1 {
                return Err("Dispatch needs one constructor".into());
            }
            actions.push(self.action(args[0], false)?);
        }
        if actions.len() > 64 {
            return Err("Callback supports at most 64 static dispatchers".into());
        }
        let workflow = crate::gesture::Workflow {
            actions,
            event_parameter,
        };
        workflow.validate(&self.report.draft.version)?;
        Ok(workflow)
    }
    fn callback_actions(&self, node: Node<'_>) -> Result<Vec<crate::action::Action>, String> {
        if node.kind() != "function_definition" {
            return self.action(node, true).map(|a| vec![a]);
        }
        self.callback_workflow(node, false).map(|w| w.actions)
    }
    fn binding(&mut self, node: Node<'_>) -> Result<(), String> {
        let parsed = (|| -> Result<crate::binding::Bind, String> {
            let args = node
                .child_by_field_name("arguments")
                .map(named)
                .unwrap_or_default();
            if args.len() < 2 || args.len() > 3 {
                return Err("Bind needs a chord, dispatcher/callback and optional options".into());
            }
            let Some(Expr::Text(chord)) = evaluate(args[0], self.source, &self.env, 0) else {
                return Err("Bind chord is unresolved".into());
            };
            let mut bind = crate::binding::Bind {
                id: format!("source-bind-{}", self.report.bindings.binds.len()),
                chord: crate::binding::Chord::parse(&chord),
                actions: self.callback_actions(args[1])?,
                flags: Default::default(),
                description: None,
                description_alias: false,
                device: None,
                submap: self.submap.clone(),
            };
            if let Some(options) = args.get(2) {
                let Some(Expr::Table(entries)) = evaluate(*options, self.source, &self.env, 0)
                else {
                    return Err("Bind options are unresolved".into());
                };
                let mut description = None;
                let mut alias = None;
                for (key, value) in entries {
                    let Key::Name(key) = key else {
                        return Err("Bind option names must be strings".into());
                    };
                    if crate::binding::FLAGS.contains(&key.as_str()) {
                        if value.truthy() {
                            bind.flags.insert(key);
                        }
                    } else if ["description", "desc"].contains(&key.as_str()) {
                        if matches!(value, Expr::Nil) {
                            continue;
                        }
                        let Expr::Text(value) = value else {
                            return Err("Description needs a static string".into());
                        };
                        if key == "description" {
                            description = Some(value);
                        } else {
                            alias = Some(value);
                        }
                    } else if key == "device" {
                        let Expr::Table(fields) = value else {
                            return Err("Device filter needs a static table".into());
                        };
                        let inclusive = member(&fields, "inclusive")
                            .map(Expr::truthy)
                            .unwrap_or(true);
                        let mut items = vec![];
                        if let Some(v) = member(&fields, "list") {
                            let Expr::Table(list) = v else {
                                return Err("Device filter list needs a static table".into());
                            };
                            for (_, v) in list {
                                let Expr::Text(s) = v else {
                                    return Err("Device/tag targets need strings".into());
                                };
                                items.push(s.clone());
                            }
                        }
                        bind.device = Some(crate::binding::DeviceFilter { inclusive, items });
                    } else {
                        return Err("Unknown bind option retained in source".into());
                    }
                }
                if let Some(d) = description {
                    bind.description = Some(d);
                } else if let Some(d) = alias {
                    bind.description = Some(d);
                    bind.description_alias = true;
                }
            }
            bind.validate(&self.report.draft.version)?;
            Ok(bind)
        })();
        match parsed {
            Ok(bind) => {
                if self.report.bindings.binds.len() >= 10000 {
                    return Err("Too many binding declarations".into());
                }
                self.report.bindings.order.push(bind.id.clone());
                self.report.bindings.binds.push(bind);
            }
            Err(error) => {
                self.report.opaque_effects = true;
                self.diagnostic(node, &error);
                let args = node
                    .child_by_field_name("arguments")
                    .map(named)
                    .unwrap_or_default();
                if args.iter().enumerate().any(|(i, n)| {
                    if i == 1 && n.kind() == "function_definition" {
                        return false;
                    }
                    evaluate(*n, self.source, &self.env, 0).is_none()
                        && !(i == 1 && self.action(*n, true).is_ok())
                }) {
                    self.api_available = false;
                }
            }
        }
        Ok(())
    }
    fn submap(&mut self, node: Node<'_>) -> Result<(), String> {
        let args = node
            .child_by_field_name("arguments")
            .map(named)
            .unwrap_or_default();
        let parsed = (|| -> Result<(String, Option<String>, Option<Node<'_>>), String> {
            if args.len() < 2 || args.len() > 3 {
                return Err("Submap needs name, optional reset and literal callback".into());
            }
            let Some(Expr::Text(name)) = evaluate(args[0], self.source, &self.env, 0) else {
                return Err("Submap name is unresolved".into());
            };
            let reset = if args.len() == 3 {
                let Some(Expr::Text(reset)) = evaluate(args[1], self.source, &self.env, 0) else {
                    return Err("Submap reset is unresolved".into());
                };
                Some(reset)
            } else {
                None
            };
            let callback = *args.last().unwrap();
            if callback.kind() != "function_definition"
                || callback
                    .child_by_field_name("parameters")
                    .is_none_or(|n| !named(n).is_empty())
            {
                return Err("Submap needs a parameterless static callback".into());
            }
            let body = callback.child_by_field_name("body");
            Ok((name, reset, body))
        })();
        match parsed {
            Ok((name, reset, body)) => {
                if self.report.bindings.submaps.len() >= 128 {
                    return Err("Too many submaps".into());
                }
                let id = format!("source-submap-{}", self.report.bindings.submaps.len());
                self.report.bindings.order.push(id.clone());
                self.report.bindings.submaps.push(crate::binding::Submap {
                    id: id.clone(),
                    name,
                    parent: self.submap.clone(),
                    reset,
                });
                let parent = self.submap.replace(id);
                let env = self.env.clone();
                if let Some(body) = body {
                    for statement in named(body) {
                        self.statement(statement)?;
                    }
                }
                self.env = env;
                self.submap = parent;
            }
            Err(e) => {
                self.report.opaque_effects = true;
                self.api_available = false;
                self.diagnostic(node, &e);
            }
        }
        Ok(())
    }
    fn statement(&mut self, node: Node<'_>) -> Result<(), String> {
        match node.kind() {
            "comment" | "hash_bang_line" | "empty_statement" => {}
            "do_statement" => {
                let env = self.env.clone();
                if let Some(body) = node.child_by_field_name("body") {
                    for n in named(body) {
                        self.statement(n)?;
                    }
                }
                self.env = env;
            }

            "variable_declaration" => {
                let children = named(node);
                let Some(first) = children.first() else {
                    return Ok(());
                };
                let assignment = if first.kind() == "assignment_statement" {
                    *first
                } else {
                    node
                };
                let Some(vars) = named(assignment)
                    .into_iter()
                    .find(|n| n.kind() == "variable_list")
                else {
                    return Ok(());
                };
                let values = named(assignment)
                    .into_iter()
                    .find(|n| n.kind() == "expression_list")
                    .map(named)
                    .unwrap_or_default();
                let evaluated: Vec<_> = values
                    .iter()
                    .map(|n| {
                        evaluate(*n, self.source, &self.env, 0).or_else(|| {
                            if self.api_available {
                                self.action(*n, true)
                                    .ok()
                                    .map(|a| Expr::Action(Box::new(a)))
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                if evaluated.iter().any(Option::is_none) {
                    self.report.opaque_effects = true;
                    self.api_available = false;
                    self.diagnostic(node,"Unresolved initializer may change bindings; subsequent API calls stay unresolved");
                }
                for (i, var) in named(vars).into_iter().enumerate() {
                    let name = text(var, self.source).to_string();
                    if ["hl", "_ENV", "_G"].contains(&name.as_str()) {
                        self.api_available = false;
                        self.diagnostic(
                            var,
                            "Hyprland binding is shadowed; subsequent calls remain unresolved",
                        );
                    }
                    self.env.remove(&name);
                    if let Some(value) = evaluated.get(i).cloned().flatten() {
                        if self.env.len() >= 1024 {
                            return Err("Too many static local bindings".into());
                        }
                        self.env.insert(name, value);
                    } else {
                        self.diagnostic(var, "Local value is unresolved");
                    }
                }
            }
            "function_call" => {
                let Some(name) = node.child_by_field_name("name") else {
                    return Ok(());
                };
                if text(name, self.source) == "hl.bind" && self.api_available {
                    self.binding(node)?;
                } else if text(name, self.source) == "hl.define_submap" && self.api_available {
                    self.submap(node)?;
                } else if text(name, self.source) == "hl.curve" && self.api_available {
                    self.animation(node, true)?;
                } else if text(name, self.source) == "hl.gesture" && self.api_available {
                    self.gesture(node)?;
                } else if text(name, self.source) == "hl.animation" && self.api_available {
                    self.animation(node, false)?;
                } else if text(name, self.source) == "hl.window_rule" && self.api_available {
                    self.match_effect_rule(node, crate::rule::Kind::Window)?;
                } else if text(name, self.source) == "hl.layer_rule" && self.api_available {
                    self.match_effect_rule(node, crate::rule::Kind::Layer)?;
                } else if text(name, self.source) == "hl.workspace_rule" && self.api_available {
                    self.workspace_rule(node)?;
                } else if text(name, self.source) == "hl.device" && self.api_available {
                    self.device(node)?;
                } else if text(name, self.source) == "hl.monitor" && self.api_available {
                    self.monitor(node)?;
                } else if text(name, self.source) == "hl.config" && self.api_available {
                    let args = node
                        .child_by_field_name("arguments")
                        .map(named)
                        .unwrap_or_default();
                    if args.len() == 1 && args[0].kind() == "table_constructor" {
                        self.call += 1;
                        let start = self.report.declarations.len();
                        self.fields(args[0], "", 0)?;
                        let mut counts = BTreeMap::new();
                        for decl in &self.report.declarations[start..] {
                            *counts.entry(decl.id.clone()).or_insert(0) += 1;
                        }
                        for (id, count) in counts {
                            if count > 1 {
                                self.report.draft.values.remove(&id);
                                for d in &mut self.report.declarations[start..] {
                                    if d.id == id {
                                        d.editable = false;
                                    }
                                }
                                self.diagnostic(node,"Duplicate or aliased keys in one table have ambiguous traversal order");
                            }
                        }
                    } else {
                        self.diagnostic(
                            node,
                            "hl.config needs a literal table for static projection",
                        );
                        self.report.opaque_effects = true;
                    }
                } else {
                    self.report.opaque_effects = true;
                    self.diagnostic(
                        node,
                        "Call left opaque; commands, includes and callbacks are never executed",
                    );
                    self.api_available = false;
                }
            }
            _ => {
                self.report.opaque_effects = true;
                self.api_available = false;
                self.env.clear();
                self.diagnostic(
                    node,
                    "Statement or control flow is opaque; subsequent API calls are unresolved",
                );
            }
        }
        Ok(())
    }
}
fn syntax_tree(source: &str) -> Result<tree_sitter::Tree, String> {
    if source.len() > MAX_DOCUMENT_BYTES {
        return Err("Lua document exceeds 1 MiB".into());
    }
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_lua::LANGUAGE.into())
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut progress = |_: &tree_sitter::ParseState| {
        if start.elapsed() > Duration::from_millis(500) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let bytes = source.as_bytes();
    let tree = parser
        .parse_with_options(
            &mut |i, _| &bytes[i.min(bytes.len())..],
            None,
            Some(ParseOptions::new().progress_callback(&mut progress)),
        )
        .ok_or("Lua parsing exceeded its time budget")?;
    if tree.root_node().has_error() {
        return Err("Lua has syntax errors; static inspection was stopped".into());
    }
    Ok(tree)
}
pub fn parse(source: &str, version: &str, catalogue: &[Setting]) -> Result<LuaReport, String> {
    if source.len() > MAX_DOCUMENT_BYTES {
        return Err("Lua document exceeds 1 MiB".into());
    }
    if ![BASELINE, AUDITED_MAIN].contains(&version) {
        return Err("Unknown Lua catalogue snapshot".into());
    }
    let tree = syntax_tree(source)?;
    let mut p = Projection {
        source,
        catalogue,
        env: BTreeMap::new(),
        report: LuaReport {
            source_sha256: source_hash(source),
            draft: Draft::new(version),
            monitors: vec![],
            devices: vec![],
            bindings: crate::binding::Draft::default(),
            workspace_rules: vec![],
            rules: vec![],
            animations: Default::default(),
            gestures: Default::default(),
            declarations: vec![],
            diagnostics: vec![],
            opaque_effects: false,
        },
        api_available: true,
        call: 0,
        submap: None,
        animation_beziers: ["default".into(), "linear".into()].into(),
        animation_springs: ["default".into()].into(),
    };
    for node in named(tree.root_node()) {
        p.statement(node)?;
    }
    p.report.animations.validate(version)?;
    p.report.gestures.validate(version)?;
    crate::rule::validate(&p.report.rules, version)?;
    p.report.draft.validate(catalogue)?;
    p.report.bindings.validate(version)?;
    crate::workspace_rule::validate(&p.report.workspace_rules, version)?;
    Ok(p.report)
}
impl LuaDocument {
    pub fn from_report(
        source: String,
        report: LuaReport,
        catalogue: &[Setting],
    ) -> Result<Self, String> {
        if source.len() > MAX_DOCUMENT_BYTES
            || source_hash(&source) != report.source_sha256
            || report.declarations.len() > 2048
            || report.diagnostics.len() > 256
        {
            return Err("Invalid or stale parser report".into());
        }
        report.animations.validate(&report.draft.version)?;
        report.gestures.validate(&report.draft.version)?;
        crate::rule::validate(&report.rules, &report.draft.version)?;
        report.draft.validate(catalogue)?;
        crate::device::validate(&report.devices, &report.draft.version)?;
        report.bindings.validate(&report.draft.version)?;
        crate::workspace_rule::validate(&report.workspace_rules, &report.draft.version)?;
        for rule in &report.monitors {
            rule.validate(&report.draft.version)?;
        }
        if report.monitors.len() > crate::monitor::MAX_RULES {
            return Err("Too many reported monitors".into());
        }
        for d in &report.declarations {
            if d.span.start >= d.span.end
                || !source.is_char_boundary(d.span.start)
                || !source.is_char_boundary(d.span.end)
                || d.span.end > source.len()
            {
                return Err("Parser returned invalid source ranges".into());
            }
        }
        Ok(Self { source, report })
    }
    /// Produces a reviewed copy only; there is intentionally no filesystem publication API.
    pub fn edited_copy(&self, draft: &Draft, catalogue: &[Setting]) -> Result<String, String> {
        if source_hash(&self.source) != self.report.source_sha256 {
            return Err("Source changed since parsing".into());
        }
        draft.validate(catalogue)?;
        if draft.version != self.report.draft.version {
            return Err("Source and draft use different snapshots".into());
        }
        if self
            .report
            .draft
            .values
            .keys()
            .any(|key| !draft.values.contains_key(key))
        {
            return Err(
                "Source-field deletion is not implemented; use override export instead".into(),
            );
        }
        let mut edits = vec![];
        for (id, value) in &draft.values {
            if self.report.draft.values.get(id) == Some(value) {
                continue;
            }
            let decl = self
                .report
                .declarations
                .iter()
                .rev()
                .find(|d| d.id == *id)
                .ok_or("Adding a source field is not implemented; use override export instead")?;
            if !decl.editable || decl.value.is_none() {
                return Err("Only existing unambiguous literal expressions can be patched".into());
            }
            edits.push((decl.span.clone(), value.lua()));
        }
        edits.sort_by_key(|(r, _)| r.start);
        if edits.windows(2).any(|pair| pair[0].0.end > pair[1].0.start) {
            return Err("Source edits overlap".into());
        }
        let mut copy = self.source.clone();
        for (range, replacement) in edits.into_iter().rev() {
            copy.replace_range(range, &replacement);
        }
        let checked = parse(&copy, &draft.version, catalogue)?;
        if checked.draft.values != draft.values {
            return Err("Patched copy did not preserve the intended declaration projection".into());
        }
        Ok(copy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue;
    #[test]
    fn empty_statements_do_not_invalidate_pure_locals_or_api_calls() {
        let source =
            ";local n=2;;hl.config{general={border_size=n}};;hl.config{decoration={rounding=n+1}};";
        let report = parse(source, BASELINE, &catalogue()).unwrap();
        assert!(!report.opaque_effects);
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.draft.values["general:border_size"], Value::Int(2));
        assert_eq!(report.draft.values["decoration:rounding"], Value::Int(3));
    }
    #[test]
    fn unknown_field_calls_cannot_make_later_api_bindings_look_trusted() {
        for unknown in [
            "unknown=untrusted()".to_string(),
            "untrusted()".to_string(),
            format!("['{}']=untrusted()", "x".repeat(257)),
            "unknown={untrusted()}".into(),
        ] {
            let source=format!("hl.config{{{unknown},general={{border_size=2}}}}\nhl.config{{general={{border_size=7}}}}");
            let report = parse(&source, BASELINE, &catalogue()).unwrap();
            assert!(report.opaque_effects, "{unknown}");
            assert_eq!(report.draft.values["general:border_size"], Value::Int(2));
            assert!(report
                .diagnostics
                .iter()
                .any(|d| d.message.contains("Unknown setting expression")));
            assert!(!report
                .declarations
                .iter()
                .any(|d| d.value == Some(Value::Int(7))));
        }
    }
    #[test]
    fn import_nested_literals_and_pure_locals() {
        let r=parse("local n = 2 + 3\nhl.config({general={border_size=n, gaps_in={top=4,left=6}}, input={kb_layout='us'}})",BASELINE,&catalogue()).unwrap();
        assert_eq!(r.draft.values["general:border_size"], Value::Int(5));
        assert_eq!(
            r.draft.values["general:gaps_in"],
            Value::CssGap([4, 0, 0, 6])
        );
        assert_eq!(r.draft.values.len(), 3);
    }
    #[test]
    fn exported_drafts_round_trip_all_types() {
        let c = catalogue();
        let mut d = Draft::new(BASELINE);
        for s in c.iter().filter(|s| s.version == BASELINE) {
            if let Some(v) = s.known_default() {
                d.values.insert(s.id.clone(), v);
            }
        }
        let r = parse(&d.export_lua(&c).unwrap(), BASELINE, &c).unwrap();
        assert_eq!(r.draft.values, d.values);
    }
    #[test]
    fn opaque_code_is_never_executed_or_misidentified() {
        for source in [
            "local hl = {}; hl.config({['general.border_size']=3})",
            "hl.config = evil; hl.config({['general.border_size']=3})",
            "setmetatable(hl, {}); hl.config({['general.border_size']=3})",
            "if true then hl.config({['general.border_size']=3}) end",
        ] {
            assert!(parse(source, BASELINE, &catalogue())
                .unwrap()
                .draft
                .values
                .is_empty());
        }
    }
    #[test]
    fn callbacks_and_requires_remain_opaque() {
        let r=parse("hl.config({['general.border_size']=3})\nrequire('private.module')\nhl.config({['general.border_size']=7})",BASELINE,&catalogue()).unwrap();
        assert_eq!(r.draft.values["general:border_size"], Value::Int(3));
        assert!(r.opaque_effects);
        assert!(!r.diagnostics.is_empty());
    }
    #[test]
    fn later_unresolved_value_is_not_guessed() {
        let r=parse("hl.config({['general.border_size']=2})\nhl.config({['general.border_size']=calculate()})",BASELINE,&catalogue()).unwrap();
        assert!(r.draft.values.is_empty());
    }
    #[test]
    fn copy_preserves_comments_and_unaffected_bytes() {
        let c = catalogue();
        let source="-- keep this\nhl.config({\n  ['general.border_size'] = 2, -- nearby\n  ['input.kb_layout'] = 'us',\n})\n-- trailing\n";
        let report = parse(source, BASELINE, &c).unwrap();
        let doc = LuaDocument::from_report(source.into(), report, &c).unwrap();
        let mut d = doc.report.draft.clone();
        d.values.insert("general:border_size".into(), Value::Int(4));
        assert_eq!(
            doc.edited_copy(&d, &c).unwrap(),
            source.replacen("= 2,", "= 4,", 1)
        );
    }
    #[test]
    fn refuses_stale_sources_additions_deletions_and_derived_edits() {
        let c = catalogue();
        let source = "local n=2;hl.config({['general.border_size']=n})";
        let report = parse(source, BASELINE, &c).unwrap();
        let mut doc = LuaDocument::from_report(source.into(), report, &c).unwrap();
        let mut d = doc.report.draft.clone();
        d.values.insert("general:border_size".into(), Value::Int(4));
        assert!(doc.edited_copy(&d, &c).is_err());
        doc.source.push(' ');
        assert!(doc.edited_copy(&d, &c).is_err());
    }
    #[test]
    fn ambiguous_aliases_do_not_produce_a_guessed_value() {
        let r = parse(
            "hl.config({['general.border_size']=2,general={border_size=3}})",
            BASELINE,
            &catalogue(),
        )
        .unwrap();
        assert!(r.draft.values.is_empty());
        assert!(r.declarations.iter().all(|d| !d.editable));
    }
    #[test]
    fn native_nul_key_aliases_replace_prior_values_and_copy_preserves_their_source_spelling() {
        let c = catalogue();
        for version in [BASELINE, AUDITED_MAIN] {
            let source="hl.config{general={border_size=3}};hl.config{['general\\0suffix']={['border_size\\000ignored']=9}}";
            let report = parse(source, version, &c).unwrap();
            assert_eq!(report.draft.values["general:border_size"], Value::Int(9));
            let mut draft = report.draft.clone();
            draft
                .values
                .insert("general:border_size".into(), Value::Int(7));
            let doc = LuaDocument::from_report(source.into(), report, &c).unwrap();
            let copy = doc.edited_copy(&draft, &c).unwrap();
            assert_eq!(copy, source.replace("ignored']=9", "ignored']=7"));
            let source = "hl.config{['general.border_size']=3,['general.border_size\\0suffix']=9}";
            let report = parse(source, version, &c).unwrap();
            assert!(report.draft.values.is_empty());
            assert!(report.declarations.iter().all(|d| !d.editable));
            let long = format!(
                "hl.config{{['general.border_size\\0{}']=8}}",
                "x".repeat(1000)
            );
            assert_eq!(
                parse(&long, version, &c).unwrap().draft.values["general:border_size"],
                Value::Int(8)
            );
        }
    }
    #[test]
    fn syntax_size_nul_and_nonfinite_rejection() {
        assert!(parse("hl.config({", BASELINE, &catalogue()).is_err());
        assert!(parse(&"x".repeat(MAX_DOCUMENT_BYTES + 1), BASELINE, &[]).is_err());
        let r = parse(
            "hl.config({['input.kb_layout']='\\000', ['decoration.active_opacity']=1e999})",
            BASELINE,
            &catalogue(),
        )
        .unwrap();
        assert!(r.draft.values.is_empty());
    }
    #[test]
    fn long_strings_normalize_lua_newlines_without_changing_source_bytes() {
        let c = catalogue();
        let source="-- Unicode ✓\r\nhl.config({['input.kb_options']=[=[\r\na\r\nb\n\rc\r\n\rd]=],['general.border_size']=2})";
        let report = parse(source, BASELINE, &c).unwrap();
        assert_eq!(
            report.draft.values["input:kb_options"],
            Value::String("a\nb\nc\n\nd".into())
        );
        let doc = LuaDocument::from_report(source.into(), report, &c).unwrap();
        let mut draft = doc.report.draft.clone();
        draft
            .values
            .insert("general:border_size".into(), Value::Int(4));
        assert_eq!(
            doc.edited_copy(&draft, &c).unwrap(),
            source.replace("=2", "=4")
        );
    }
    #[test]
    fn quoted_and_long_strings() {
        let r = parse(
            "hl.config({['input.kb_layout']=[=[\nus]=], ['input.kb_options']='a\\010b\\x21'})",
            BASELINE,
            &catalogue(),
        )
        .unwrap();
        assert_eq!(
            r.draft.values["input:kb_layout"],
            Value::String("us".into())
        );
        assert_eq!(
            r.draft.values["input:kb_options"],
            Value::String("a\nb!".into())
        );
    }
}
