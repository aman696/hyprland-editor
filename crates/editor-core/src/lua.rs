//! Static Lua source projection and byte-preserving copy preparation. Never runs Lua.
use crate::{parse_color, Draft, Setting, Value, AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES};
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
                // Literal Lua table fields use the last write for the exact key.
                if let Some(old) = out.iter_mut().find(|(k, _)| *k == key) {
                    *old = (key, value);
                } else {
                    out.push((key, value));
                }
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
        ("FontWeight", v) => Value::FontWeight(integer(&v)?.try_into().ok()?),
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
            if key.len() > 256 {
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
            }
        }
        Ok(())
    }
    fn statement(&mut self, node: Node<'_>) -> Result<(), String> {
        match node.kind() {
            "comment" | "hash_bang_line" => {}
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
                    .map(|n| evaluate(*n, self.source, &self.env, 0))
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
                if text(name, self.source) == "hl.config" && self.api_available {
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
pub fn parse(source: &str, version: &str, catalogue: &[Setting]) -> Result<LuaReport, String> {
    if source.len() > MAX_DOCUMENT_BYTES {
        return Err("Lua document exceeds 1 MiB".into());
    }
    if ![BASELINE, AUDITED_MAIN].contains(&version) {
        return Err("Unknown Lua catalogue snapshot".into());
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
        return Err("Lua has syntax errors; no declarations were imported".into());
    }
    let mut p = Projection {
        source,
        catalogue,
        env: BTreeMap::new(),
        report: LuaReport {
            source_sha256: source_hash(source),
            draft: Draft::new(version),
            declarations: vec![],
            diagnostics: vec![],
            opaque_effects: false,
        },
        api_available: true,
        call: 0,
    };
    for node in named(tree.root_node()) {
        p.statement(node)?;
    }
    p.report.draft.validate(catalogue)?;
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
        report.draft.validate(catalogue)?;
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
