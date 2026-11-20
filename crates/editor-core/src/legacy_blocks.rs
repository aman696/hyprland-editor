//! Shared bounded flat native category scanner and source-relative field patches.
use crate::{
    legacy_session::Site,
    legacy_syntax::{code, trim},
    MAX_DOCUMENT_BYTES,
};
use std::{collections::BTreeMap, ops::Range};
#[derive(Clone)]
pub(crate) struct Field {
    pub key: String,
    pub value: String,
    pub statement: Range<usize>,
    pub value_span: Range<usize>,
}
pub(crate) struct Block {
    pub site: Site,
    pub footer: usize,
    pub fields: Vec<Field>,
    pub editable: bool,
    depth: usize,
}
pub(crate) fn scan(
    content: &str,
    category: &str,
    mut visit: impl FnMut(Block),
) -> Result<(), String> {
    if content.len() > MAX_DOCUMENT_BYTES || content.contains('\0') {
        return Err("Legacy source exceeds its bounds".into());
    }
    let (mut offset, mut depth, mut flags, mut continued) = (0usize, 0usize, false, false);
    let mut current: Option<Block> = None;
    for (index, line) in content.split_inclusive('\n').enumerate() {
        let physical = line.trim_end_matches(['\r', '\n']);
        let previous = continued;
        continued = physical.ends_with('\\');
        let (decoded, end) = code(physical);
        let value = trim(&decoded);
        let raw = &physical[..end];
        let statement = offset + raw.len()
            - raw
                .trim_start_matches(|c: char| c.is_ascii_whitespace())
                .len()
            ..offset
                + raw
                    .trim_end_matches(|c: char| c.is_ascii_whitespace())
                    .len();
        if trim(physical).starts_with('#')
            && trim(trim(physical).trim_start_matches('#')).starts_with("hyprlang")
        {
            flags = true;
        }
        if let Some(block) = current.as_mut() {
            block.editable &= !flags && !previous && !continued;
            if value == "}" && depth == block.depth {
                let mut block = current.take().unwrap();
                block.site.statement.end = statement.end;
                block.footer = offset;
                visit(block);
            } else if !value.is_empty() {
                if depth != 1 || value.ends_with('{') || block.fields.len() >= 128 {
                    block.editable = false;
                } else if let Some((key, rhs)) = value.split_once('=') {
                    if let Some(eq) = raw.find('=') {
                        let raw_rhs = &raw[eq + 1..];
                        let start = offset + eq + 1 + raw_rhs.len()
                            - raw_rhs
                                .trim_start_matches(|c: char| c.is_ascii_whitespace())
                                .len();
                        block.fields.push(Field {
                            key: trim(key).into(),
                            value: trim(rhs).into(),
                            statement: statement.clone(),
                            value_span: start
                                ..offset
                                    + raw
                                        .trim_end_matches(|c: char| c.is_ascii_whitespace())
                                        .len(),
                        });
                    } else {
                        block.editable = false;
                    }
                } else {
                    block.editable = false;
                }
            }
        } else if value
            .strip_suffix('{')
            .is_some_and(|name| trim(name) == category)
            && !value.contains('=')
        {
            current = Some(Block {
                site: Site {
                    line: index + 1,
                    statement: statement.clone(),
                },
                footer: offset,
                fields: vec![],
                depth: depth + 1,
                editable: depth == 0 && !flags && !previous && !continued,
            });
        }
        if value == "}" {
            depth = depth.saturating_sub(1);
        } else if value.ends_with('{') && !value.contains('=') {
            depth += 1;
            if depth > 16 {
                return Err("Legacy category nesting exceeds 16".into());
            }
        }
        offset += line.len();
    }
    if let Some(mut block) = current {
        block.editable = false;
        block.site.statement.end = content.len();
        visit(block);
    }
    Ok(())
}
/// Return source-relative patches; unchanged field spellings and comments stay exact.
pub(crate) fn patches(
    content: &str,
    block: &Block,
    old: &BTreeMap<String, String>,
    new: &BTreeMap<String, String>,
) -> Result<Vec<(Range<usize>, String)>, String> {
    let mut edits = vec![];
    for field in &block.fields {
        if old.get(&field.key) == new.get(&field.key) {
            continue;
        }
        if let Some(value) = new.get(&field.key) {
            if block
                .fields
                .iter()
                .rev()
                .find(|f| f.key == field.key)
                .is_some_and(|last| last.statement == field.statement)
            {
                let suffix = if content.as_bytes().get(field.value_span.end) == Some(&b'#') {
                    " "
                } else {
                    ""
                };
                edits.push((field.value_span.clone(), format!("{value}{suffix}")));
            }
        } else {
            edits.push((field.statement.clone(), String::new()));
        }
    }
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut added = String::new();
    for (key, value) in new {
        if !block.fields.iter().any(|field| &field.key == key) {
            added.push_str(&format!("    {key} = {value}{newline}"));
        }
    }
    if !added.is_empty() {
        edits.push((block.footer..block.footer, added));
    }
    Ok(edits)
}
/// Preserve the block's comments if its operation is explicitly changed or removed.
pub(crate) fn comments(content: &str, span: Range<usize>) -> String {
    let mut result = String::new();
    for line in content[span].split_inclusive('\n') {
        let physical = line.trim_end_matches(['\r', '\n']);
        let (decoded, end) = code(physical);
        if trim(&decoded).is_empty() {
            result.push_str(line);
        } else {
            result.push_str(&line[end..]);
        }
    }
    result
}
