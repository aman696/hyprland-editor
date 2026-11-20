//! Ordered edits to source directives in a captured legacy file.
use crate::{
    legacy_syntax::{trim, walk},
    workspace::Workspace,
    Setting, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ops::Range};

pub mod captures;
pub mod project_copy;

pub const MAX_ROWS: usize = 1024;
pub const MAX_PATH: usize = 4096;
const MAX_TEXT: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Include {
    pub original: Option<usize>,
    /// Native path template after comment decoding, before expansion.
    pub template: String,
}

struct Scan {
    rows: Vec<Include>,
    sites: Vec<Range<usize>>,
    lines: Vec<usize>,
    unresolved: usize,
    first_unresolved: Option<usize>,
}

fn scan(content: &str) -> Result<Scan, String> {
    let mut result = Scan {
        rows: Vec::new(),
        sites: Vec::new(),
        lines: Vec::new(),
        unresolved: 0,
        first_unresolved: None,
    };
    let mut total = 0;
    walk(content, |statement| {
        if statement.key != "source" {
            return;
        }
        if statement.editable
            && result.rows.len() < MAX_ROWS
            && total + statement.value.len() <= MAX_TEXT
            && valid_template(statement.value)
        {
            total += statement.value.len();
            result.rows.push(Include {
                original: Some(result.rows.len()),
                template: statement.value.to_owned(),
            });
            result.sites.push(statement.statement);
            result.lines.push(statement.line);
        } else {
            result.unresolved += 1;
            result.first_unresolved.get_or_insert(statement.line);
        }
    })?;
    Ok(result)
}

fn valid_template(value: &str) -> bool {
    (2..=MAX_PATH).contains(&value.len())
        && trim(value) == value
        && !value.chars().any(char::is_control)
        && !value.ends_with('\\')
}

pub fn validate_template(value: &str) -> Result<(), String> {
    if valid_template(value) {
        Ok(())
    } else {
        Err(
            "Enter a single path template between 2 and 4,096 bytes without a trailing backslash."
                .into(),
        )
    }
}

pub fn inspect(content: &str) -> Result<(Vec<Include>, usize), String> {
    let scanned = scan(content)?;
    Ok((scanned.rows, scanned.unresolved))
}

fn same(a: &[Include], b: &[Include]) -> bool {
    a.iter()
        .map(|item| &item.template)
        .eq(b.iter().map(|item| &item.template))
}

pub fn prepare(
    workspace: &Workspace,
    target: &[Include],
    catalogue: &[Setting],
) -> Result<String, String> {
    let current = workspace
        .legacy_content()
        .ok_or("Import a .conf project before editing includes.")?;
    let base = scan(current)?;
    let copy = crate::legacy_variables::materialize(workspace, catalogue)?;
    let materialized = scan(&copy)?;
    if !same(&base.rows, &materialized.rows) || base.unresolved != materialized.unresolved {
        return Err(
            "Other draft edits changed the include list. Reopen the editor and review the source."
                .into(),
        );
    }
    let revised = edit(&copy, materialized, target)?;
    crate::legacy::Document::new_with_binding_mode(
        revised.clone(),
        catalogue,
        workspace
            .bindings
            .legacy
            .as_ref()
            .is_some_and(|draft| draft.local_variables),
    )?;
    Ok(revised)
}

fn edit(content: &str, base: Scan, target: &[Include]) -> Result<String, String> {
    if target.len() > MAX_ROWS {
        return Err("Too many include directives in this draft.".into());
    }
    let mut retained = BTreeSet::new();
    let mut total = 0;
    for row in target {
        validate_template(&row.template)?;
        total += row.template.len();
        if total > MAX_TEXT {
            return Err("Include templates exceed the 256 KiB draft limit.".into());
        }
        if let Some(index) = row.original {
            if index >= base.rows.len() || !retained.insert(index) {
                return Err("Invalid or duplicate original include slot.".into());
            }
        }
    }

    // Removed directives stay removed from their own source slots. Reordering
    // fills surviving slots in the requested order, retaining their comments.
    let expected = target;
    let mut target = target.iter().peekable();
    let mut patches = Vec::new();
    let nl = newline(content);
    for (index, span) in base.sites.iter().enumerate() {
        if !retained.contains(&index) {
            let suffix = content[span.end..].split('\n').next().unwrap_or_default();
            if trim(trim(suffix).trim_start_matches('#')).starts_with("hyprlang") {
                return Err("Removing this directive would turn its inline parser comment into a standalone comment. Edit its path instead.".into());
            }
            patches.push((span.clone(), String::new()));
            continue;
        }
        let raw = &content[span.clone()];
        let line_start = content[..span.start].rfind('\n').map_or(0, |at| at + 1);
        let indent = &content[line_start..span.start];
        let mut replacement = String::new();
        while target.peek().is_some_and(|row| row.original.is_none()) {
            let row = target.next().unwrap();
            append_directive(&mut replacement, row, nl)?;
            replacement.push_str(indent);
        }
        let next = target.next().expect("one row per retained slot");
        if next.template == base.rows[index].template {
            replacement.push_str(raw);
        } else {
            let eq = raw
                .find('=')
                .ok_or("Captured include lost its assignment.")?
                + 1;
            let leading = raw[eq..].len()
                - raw[eq..]
                    .trim_start_matches(|c: char| c.is_ascii_whitespace())
                    .len();
            replacement.push_str(&raw[..eq + leading]);
            replacement.push_str(&crate::legacy_variables::value(&next.template)?);
        }
        patches.push((span.clone(), replacement));
    }
    if target.peek().is_some() {
        let at = base.sites.last().map_or(0, |span| {
            content[span.end..]
                .find('\n')
                .map_or(content.len(), |offset| span.end + offset + 1)
        });
        let mut addition = String::new();
        if at != 0 && !content[..at].ends_with('\n') {
            addition.push_str(nl);
        }
        for row in target {
            append_directive(&mut addition, row, nl)?;
        }
        patches.push((at..at, addition));
    }

    let mut output = content.to_owned();
    for (span, replacement) in patches.into_iter().rev() {
        output.replace_range(span, &replacement);
    }
    if output.len() > MAX_DOCUMENT_BYTES {
        return Err("Include source copy exceeds 1 MiB.".into());
    }
    let reparsed = scan(&output)?;
    if reparsed.unresolved != base.unresolved || !same(&reparsed.rows, expected) {
        return Err("Could not reproduce the requested include order in the source copy.".into());
    }
    Ok(output)
}

fn append_directive(text: &mut String, row: &Include, newline: &str) -> Result<(), String> {
    text.push_str("source = ");
    text.push_str(&crate::legacy_variables::value(&row.template)?);
    text.push_str(newline);
    Ok(())
}

fn newline(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_removal_and_order_keep_comments_local_and_accept_native_templates() {
        let source = concat!(
            "source = remove.conf # first\r\n",
            "general:border_size = 3\r\n",
            "  source   = old.conf # middle\r\n",
            "$config = ~/.config/hypr\r\n",
            "source = final.conf # last\r\n",
        );
        let base = scan(source).unwrap();
        let mut target = vec![base.rows[2].clone(), base.rows[1].clone()];
        target[1].template = "$config/themes/*.conf".into();
        target.insert(
            1,
            Include {
                original: None,
                template: "literal#fragment.conf".into(),
            },
        );
        let revised = edit(source, base, &target).unwrap();
        assert_eq!(
            revised,
            concat!(
                " # first\r\n",
                "general:border_size = 3\r\n",
                "  source   = final.conf # middle\r\n",
                "$config = ~/.config/hypr\r\n",
                "source = literal##fragment.conf\r\n",
                "source = $config/themes/*.conf # last\r\n",
            )
        );
        let kept = scan(&revised).unwrap();
        assert_eq!(
            edit(&revised, scan(&revised).unwrap(), &kept.rows).unwrap(),
            revised
        );
    }

    #[test]
    fn opaque_directives_remain_and_invalid_or_duplicated_slots_are_refused() {
        let source = "general {\n source = nested.conf\n}\nsource = editable.conf\n";
        let (rows, unresolved) = inspect(source).unwrap();
        assert_eq!(unresolved, 1);
        assert_eq!(rows.len(), 1);
        let duplicate = [rows[0].clone(), rows[0].clone()];
        assert!(edit(source, scan(source).unwrap(), &duplicate).is_err());
        assert_eq!(
            edit(source, scan(source).unwrap(), &[]).unwrap(),
            "general {\n source = nested.conf\n}\n\n"
        );

        let parser_comment = "source = example.conf # hyprlang if FLAG\n";
        assert!(edit(parser_comment, scan(parser_comment).unwrap(), &[]).is_err());
        assert!(validate_template("file.conf\nsource = other.conf").is_err());
        assert!(validate_template("file.conf\\").is_err());
    }
}
