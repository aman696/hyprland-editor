//! Syntax-local compound edits. Preserves every comment and all existing trivia;
//! modeled comparison still decides whether aliases/later writes changed more.
use super::*;
use crate::lua::{named, syntax_tree, text, Expr};
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Bool(bool),
    Int(i64),
    Bytes(Vec<u8>),
}
impl Key {
    fn observed(&self) -> Observed {
        match self {
            Self::Bool(v) => Observed::Bool(*v),
            Self::Int(v) => Observed::Int(*v),
            Self::Bytes(v) => Observed::Bytes(v.clone()),
        }
    }
    fn from(value: &Observed) -> Result<Self, String> {
        match value {
            Observed::Bool(v) => Ok(Self::Bool(*v)),
            Observed::Int(v) => Ok(Self::Int(*v)),
            Observed::Bytes(v) => Ok(Self::Bytes(v.clone())),
            _ => Err("Table layout needs exact boolean, integer or string keys".into()),
        }
    }
}
#[derive(Clone, Copy)]
enum Shape {
    Pair,
    Gap,
    Gradient,
    Colors,
}
impl Shape {
    fn owns(self, key: &Key) -> bool {
        match (self, key) {
            (Self::Pair | Self::Colors, Key::Int(i)) => *i > 0,
            (Self::Gap, Key::Bytes(k)) => {
                [b"top".as_slice(), b"right", b"bottom", b"left"].contains(&k.as_slice())
            }
            (Self::Gradient, Key::Bytes(k)) => k == b"colors" || k == b"angle",
            _ => false,
        }
    }
    fn child(self, key: &Key) -> Option<Self> {
        matches!((self, key), (Self::Gradient, Key::Bytes(k)) if k == b"colors")
            .then_some(Self::Colors)
    }
}
fn rows(value: &Observed) -> Result<BTreeMap<Key, Observed>, String> {
    let Observed::Table(rows) = value else {
        return Err("Preserve table layout requires an existing table value".into());
    };
    let mut result = BTreeMap::new();
    for (key, value) in rows {
        if result.insert(Key::from(key)?, value.clone()).is_some() {
            return Err("Ambiguous observed table keys".into());
        }
    }
    Ok(result)
}
fn key(field: Node<'_>, source: &str, array: &mut i64) -> Result<Key, String> {
    let Some(node) = field.child_by_field_name("name") else {
        let key = Key::Int(*array);
        *array += 1;
        return Ok(key);
    };
    if node.kind() == "identifier" && !text(field, source).trim_start().starts_with('[') {
        return Ok(Key::Bytes(text(node, source).as_bytes().to_vec()));
    }
    // No environment, callbacks or evaluation of supplied Lua. This independent
    // literal decoder is deliberately narrower than the program evaluator.
    let value = crate::lua::evaluate(node, source, &BTreeMap::new(), 0)
        .ok_or("A computed table key has no syntax-local layout edit")?;
    match value {
        Expr::Text(v) => Ok(Key::Bytes(v.into_bytes())),
        Expr::Bool(v) => Ok(Key::Bool(v)),
        Expr::Int(v) => Ok(Key::Int(v)),
        // Reject float-key normalization here instead of inventing a cast.
        _ => Err("This table key needs a canonical full-expression comparison".into()),
    }
}
// Remove syntax tokens only. Comments and whitespace survive even inside a
// removed field or replaced expression. Quoted '--' is a string token, not a comment.
fn erase_tokens(node: Node<'_>, source: &str) -> Result<String, String> {
    let mut pending = vec![node];
    let mut tokens = vec![];
    let mut visited = 0;
    while let Some(current) = pending.pop() {
        visited += 1;
        if visited > 8192 || pending.len() > 8192 {
            return Err("Compound trivia traversal exceeds its syntax-node limit".into());
        }
        if current.kind() == "comment" {
            continue;
        }
        if current.child_count() == 0 {
            if !current.byte_range().is_empty() {
                tokens.push(current.byte_range());
            }
        } else {
            let mut cursor = current.walk();
            pending.extend(current.children(&mut cursor));
        }
    }
    tokens.sort_by_key(|span| span.start);
    let mut output = text(node, source).to_string();
    for span in tokens.into_iter().rev() {
        output.replace_range(
            span.start - node.start_byte()..span.end - node.start_byte(),
            "",
        );
    }
    Ok(output)
}
struct Edit {
    span: std::ops::Range<usize>,
    text: String,
}
fn same_component(
    shape: Shape,
    key: &Key,
    old: &Observed,
    new: &Observed,
    parser: &mut scalar_codec::Parser,
) -> Result<bool, String> {
    use scalar_codec::Codec;
    let (codec, a, b) = match (shape, key) {
        (Shape::Colors, Key::Int(_)) => (Codec::Color, old.clone(), new.clone()),
        (Shape::Pair, Key::Int(_)) => {
            let pair = |v: &Observed| {
                Observed::Table(vec![
                    (Observed::Int(1), v.clone()),
                    (Observed::Int(2), Observed::Int(0)),
                ])
            };
            (Codec::Vec2 { validator: None }, pair(old), pair(new))
        }
        (Shape::Gap, Key::Bytes(_)) => {
            let side = |v: &Observed| fields([("top", v.clone())]);
            (
                Codec::CssGap {
                    minimum: None,
                    maximum: None,
                },
                side(old),
                side(new),
            )
        }
        (Shape::Gradient, Key::Bytes(k)) if k == b"angle" => {
            let angle = |v: &Observed| {
                fields([
                    ("angle", v.clone()),
                    (
                        "colors",
                        Observed::Table(vec![(
                            Observed::Int(1),
                            Observed::Bytes(b"#000".to_vec()),
                        )]),
                    ),
                ])
            };
            (Codec::Gradient, angle(old), angle(new))
        }
        _ => return Ok(false),
    };
    Ok(
        matches!((parser.decode(&codec,&a)?,parser.decode(&codec,&b)?), (scalar_codec::Outcome::Accepted {native:a,..},scalar_codec::Outcome::Accepted {native:b,..}) if a==b),
    )
}
fn rewrite(
    node: Node<'_>,
    source: &str,
    old: &Observed,
    new: &Observed,
    shape: Option<Shape>,
    depth: usize,
    parser: &mut scalar_codec::Parser,
) -> Result<(String, Observed), String> {
    if depth > 4 {
        return Err("Compound layout nesting limit exceeded".into());
    }
    if edits::same_value(old, new) {
        return Ok((text(node, source).into(), old.clone()));
    }
    let Some(shape) = shape else {
        return Ok((
            format!("{}{}", print(new)?, erase_tokens(node, source)?),
            new.clone(),
        ));
    };
    if node.kind() != "table_constructor" {
        return Err("Preserve table layout needs a directly selected table constructor; choose full-expression comparison for a computed value".into());
    }
    let old_rows = rows(old)?;
    let new_rows = rows(new)?;
    let mut expected = old_rows.clone();
    expected.retain(|key, _| !shape.owns(key));
    expected.extend(new_rows.clone());
    let mut array = 1;
    let mut fields = BTreeMap::new();
    let mut edits = vec![];
    let children = named(node);
    if children.len() > 512 {
        return Err("Compound layout field limit exceeded".into());
    }
    for field in children {
        if field.kind() != "field" {
            return Err("Unsupported constructor field syntax".into());
        }
        let k = key(field, source, &mut array)?;
        let value = field
            .child_by_field_name("value")
            .ok_or("Missing constructor value")?;
        if field.child_by_field_name("name").is_none()
            && matches!(value.kind(), "function_call" | "vararg_expression")
        {
            return Err("An expanding array expression needs full-expression comparison".into());
        }
        if fields.insert(k.clone(), (field, value)).is_some() {
            return Err("Duplicate constructor keys cannot preserve layout".into());
        }
    }
    if old_rows.keys().any(|k| !fields.contains_key(k)) {
        return Err(
            "The observed table contains fields created outside the selected constructor".into(),
        );
    }
    for (k, (field, value)) in &fields {
        if !shape.owns(k) {
            continue;
        }
        if let Some(new) = new_rows.get(k) {
            let old = old_rows.get(k).unwrap_or(&Observed::Nil);
            let (replacement, observed) = if same_component(shape, k, old, new, parser)? {
                (text(*value, source).to_string(), old.clone())
            } else {
                rewrite(*value, source, old, new, shape.child(k), depth + 1, parser)?
            };
            expected.insert(k.clone(), observed);
            if replacement != text(*value, source) {
                edits.push(Edit {
                    span: value.byte_range(),
                    text: replacement,
                });
            }
        } else {
            edits.push(Edit {
                span: field.byte_range(),
                text: erase_tokens(*field, source)?,
            });
            // Separators are syntax siblings; erase only the following one.
            let mut following = field.next_sibling();
            while following.is_some_and(|n| n.kind() == "comment") {
                following = following.and_then(|n| n.next_sibling());
            }
            if let Some(next) = following.filter(|n| matches!(n.kind(), "," | ";")) {
                edits.push(Edit {
                    span: next.byte_range(),
                    text: String::new(),
                });
            } else {
                // Last field without trailing separator: remove its preceding separator,
                // skipping comments which remain in their original byte positions.
                let mut previous = field.prev_sibling();
                while previous.is_some_and(|n| n.kind() == "comment") {
                    previous = previous.and_then(|n| n.prev_sibling());
                }
                if let Some(previous) = previous.filter(|n| matches!(n.kind(), "," | ";")) {
                    edits.push(Edit {
                        span: previous.byte_range(),
                        text: String::new(),
                    });
                }
            }
        }
    }
    let missing = new_rows
        .iter()
        .filter(|(k, _)| !fields.contains_key(*k))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        // Explicit keys do not advance implicit array indices. Insert immediately
        // after the opening brace so no preceding line comment can swallow them,
        // and no separator removed with a different field can strand this one.
        let newline = if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let mut inserted = newline.to_string();
        for (k, v) in missing {
            inserted.push_str(&format!(
                "[{}] = {},{}",
                print(&k.observed())?,
                print(v)?,
                newline
            ));
        }
        let start = node.start_byte() + 1;
        edits.push(Edit {
            span: start..start,
            text: inserted,
        });
    }
    edits.sort_by_key(|e| (e.span.start, e.span.end));
    // Adjacent field removals can identify the same separator. Remove once.
    edits.dedup_by(|a, b| a.span == b.span && a.text == b.text);
    if edits.windows(2).any(|w| w[0].span.end > w[1].span.start) {
        return Err("Overlapping compound layout edits".into());
    }
    let mut output = text(node, source).to_string();
    for edit in edits.into_iter().rev() {
        output.replace_range(
            edit.span.start - node.start_byte()..edit.span.end - node.start_byte(),
            &edit.text,
        );
    }
    if output.len() > crate::MAX_DOCUMENT_BYTES {
        return Err("Compound layout output exceeds source limit".into());
    }
    let observed = Observed::Table(
        expected
            .into_iter()
            .map(|(k, v)| (k.observed(), v))
            .collect(),
    );
    Ok((output, observed))
}
pub(super) fn prepare(
    source: &str,
    span: std::ops::Range<usize>,
    kind: &str,
    old: &Observed,
    new: &Observed,
) -> Result<(String, Observed), String> {
    let shape = match kind {
        "Vec2" => Shape::Pair,
        "CssGap" => Shape::Gap,
        "Gradient" => Shape::Gradient,
        _ => {
            return Err("Preserve table layout is available for vectors, gaps and gradients".into())
        }
    };
    let tree = syntax_tree(source)?;
    let node = tree
        .root_node()
        .descendant_for_byte_range(span.start, span.end)
        .filter(|n| n.byte_range() == span)
        .ok_or("The selected source span is not an exact syntax node")?;
    // Bounded lexical construction; native syntax/model checks follow independently.
    rewrite(
        node,
        source,
        old,
        new,
        Some(shape),
        0,
        &mut scalar_codec::Parser::new()?,
    )
}
