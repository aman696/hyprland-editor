//! A self-contained copy of explicitly captured native includes.
use super::{captures, edit, scan};
use crate::{
    legacy_syntax, lua::source_hash, source::display_path_bytes, source::CapturedSource,
    MAX_DOCUMENT_BYTES,
};
use captures::{LinkedRevision, Outcome};
use std::{collections::BTreeMap, ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};

pub const MAX_COPY_BYTES: usize = MAX_DOCUMENT_BYTES + 2 * captures::MAX_BYTES;

pub struct File {
    pub source_path: String,
    pub name: String,
    pub content: String,
    pub sha256: String,
}

pub struct Prepared {
    pub files: Vec<File>,
    pub include_count: usize,
    pub detached_count: usize,
}

/// The caller supplies a materialized root draft. Every other byte comes from
/// a capture or an accepted revision, with no filesystem resolution.
pub fn prepare(
    root_path: &[u8],
    root: &str,
    files: &[CapturedSource],
    revisions: &[LinkedRevision],
) -> Result<Prepared, String> {
    let root_label = display_path_bytes(root_path);
    let report = captures::inspect(root_path, root, files, revisions)?;
    check_continuations(root).map_err(|error| format!("{root_label}{error}"))?;
    for &index in &report.reached {
        let file = &files[index];
        check_continuations(captures::effective_text(file, revisions))
            .map_err(|error| format!("{}{error}", file.identity.display_path()))?;
    }
    let mut order = Vec::new();
    let mut names = BTreeMap::new();
    let mut edges = BTreeMap::<Option<usize>, BTreeMap<usize, usize>>::new();
    for row in &report.rows {
        let target = match row.outcome {
            Outcome::Captured(index) => index,
            other => {
                let path = row.parent.map_or_else(
                    || root_label.clone(),
                    |index| files[index].identity.display_path(),
                );
                let remedy = match other {
                    Outcome::NeedsSelection => "Choose its file in Linked File Snapshots.",
                    Outcome::UnresolvedTemplate => {
                        "Use a supported literal path or an earlier source-defined path."
                    }
                    Outcome::Cycle(_) | Outcome::RootCycle => "Remove the cycle before exporting.",
                    Outcome::DepthLimit => "This chain exceeds the 16-file inclusion depth.",
                    Outcome::Captured(_) => unreachable!(),
                };
                return Err(format!(
                    "{path}:{} · {}\n{remedy}",
                    row.line,
                    display_path_bytes(row.template.as_bytes()),
                ));
            }
        };
        names.entry(target).or_insert_with(|| {
            order.push(target);
            linked_name(&files[target].identity.selected_path, order.len())
        });
        if edges
            .entry(row.parent)
            .or_default()
            .insert(row.line, target)
            .is_some_and(|previous| previous != target)
        {
            let path = row.parent.map_or_else(
                || root_label.clone(),
                |index| files[index].identity.display_path(),
            );
            return Err(format!(
                "{path}:{} · This include resolves to different files on repeated visits. A single exported copy cannot preserve those targets.",
                row.line,
            ));
        }
    }
    if report.truncated {
        return Err("The include traversal exceeded its export budget. Reduce repeated or nested includes before exporting.".into());
    }
    let mut output = Vec::with_capacity(order.len() + 1);
    let mut bytes = 0;
    for index in std::iter::once(None).chain(order.iter().copied().map(Some)) {
        let (label, name, source) = match index {
            None => (root_label.clone(), "hyprland.conf", root),
            Some(i) => (
                files[i].identity.display_path(),
                names[&i].as_str(),
                captures::effective_text(&files[i], revisions),
            ),
        };
        let outgoing = edges.get(&index);
        let content =
            relocate(source, outgoing, &names).map_err(|error| format!("{label}{error}"))?;
        bytes += content.len();
        if bytes > MAX_COPY_BYTES {
            return Err("The relocated files exceed the 5 MiB export limit.".into());
        }
        output.push(File {
            source_path: label,
            name: name.into(),
            sha256: source_hash(&content),
            content,
        });
    }
    Ok(Prepared {
        files: output,
        include_count: report.rows.len(),
        detached_count: files.len() - order.len(),
    })
}

fn linked_name(path: &[u8], number: usize) -> String {
    let stem = Path::new(OsStr::from_bytes(path))
        .file_stem()
        .map_or(b"include".as_slice(), OsStr::as_bytes);
    let slug: String = stem
        .iter()
        .take(48)
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' => char::from(*b),
            _ => '-',
        })
        .collect();
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "include" } else { slug };
    format!("{number:02}-{slug}.conf")
}

fn relocate(
    source: &str,
    outgoing: Option<&BTreeMap<usize, usize>>,
    names: &BTreeMap<usize, String>,
) -> Result<String, String> {
    let mut dynamic_key = None;
    legacy_syntax::walk(source, |statement| {
        if !statement.key.starts_with('$') && statement.key.contains('$') {
            dynamic_key.get_or_insert(statement.line);
        }
    })
    .map_err(|error| format!(" · {error}"))?;
    if let Some(line) = dynamic_key {
        return Err(format!(
            ":{line} · A variable in a directive name may produce an include. Use a literal directive name before exporting."
        ));
    }
    let parsed = scan(source).map_err(|error| format!(" · {error}"))?;
    if let Some(line) = parsed.first_unresolved {
        return Err(format!(
            ":{line} · An include is outside the supported top-level syntax. Review this file on its own."
        ));
    }
    if parsed.rows.len() != outgoing.map_or(0, BTreeMap::len) {
        return Err(" · The include list differs from the captured traversal.".into());
    }
    let mut replaced = parsed.rows.clone();
    for (row, line) in replaced.iter_mut().zip(&parsed.lines) {
        let target = outgoing
            .and_then(|sites| sites.get(line))
            .and_then(|index| names.get(index))
            .ok_or_else(|| format!(":{line} · No reviewed copy was assigned to this include."))?;
        row.template.clone_from(target);
    }
    edit(source, parsed, &replaced).map_err(|error| format!(" · {error}"))
}

fn check_continuations(source: &str) -> Result<(), String> {
    if let Some(line) = source
        .split('\n')
        .position(|line| line.trim_end_matches('\r').ends_with('\\'))
    {
        return Err(format!(
            ":{} · Continued lines may hide include directives. Review this file on its own.",
            line + 1,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        legacy_variables::Revision,
        source::{RecordedOwnership, SourceIdentity},
    };

    fn captured(path: &str, content: &str) -> CapturedSource {
        CapturedSource {
            content: content.into(),
            identity: SourceIdentity {
                selected_path: path.as_bytes().to_vec(),
                resolved_path: None,
                links: vec![],
                device: 1,
                inode: 1,
                owner: 1000,
                mode: 0o100600,
                size: content.len() as u64,
                modified_seconds: 0,
                modified_nanoseconds: 0,
                changed_seconds: 0,
                changed_nanoseconds: 0,
                sha256: source_hash(content),
                recorded_ownership: RecordedOwnership::UserOwned,
            },
        }
    }

    #[test]
    fn rewrites_nested_absolute_and_repeated_includes_without_losing_their_slots() {
        let files = [
            captured("/config/other/colors.conf", "general:border_size = 7\n"),
            captured("/config/parts/colors.conf", "# captured before edit\n"),
            captured("/config/parts/dark#mode.conf", "general:border_size = 9\n"),
            captured("/config/detached.conf", "source = missing.conf\n"),
        ];
        let revision = LinkedRevision {
            path: files[1].identity.selected_path.clone(),
            revision: Revision::new(
                &files[1].content,
                "  source=./dark##mode.conf # palette\n# keep\n".into(),
            )
            .unwrap(),
        };
        let copy = prepare(
            b"/config/main.conf",
            concat!(
                "  source   = parts/colors.conf  # first\r\n",
                "general:border_size = 3\r\n",
                "source=/config/other/colors.conf\r\n",
                "source=parts//colors.conf\r\n"
            ),
            &files,
            &[revision],
        )
        .unwrap();
        assert_eq!(copy.detached_count, 1);
        assert_eq!(copy.include_count, 5);
        assert_eq!(
            copy.files
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            [
                "hyprland.conf",
                "01-colors.conf",
                "02-dark-mode.conf",
                "03-colors.conf"
            ],
        );
        assert_eq!(
            copy.files[0].content,
            concat!(
                "  source   = 01-colors.conf  # first\r\n",
                "general:border_size = 3\r\n",
                "source=03-colors.conf\r\n",
                "source=01-colors.conf\r\n"
            ),
        );
        assert_eq!(
            copy.files[1].content,
            "  source=02-dark-mode.conf # palette\n# keep\n"
        );
        assert_eq!(copy.files[2].content, files[2].content);
        assert_eq!(copy.files[3].content, files[0].content);
        let relocated = copy.files[1..]
            .iter()
            .map(|file| captured(&format!("/moved/{}", file.name), &file.content))
            .collect::<Vec<_>>();
        let report = captures::inspect(
            b"/moved/hyprland.conf",
            &copy.files[0].content,
            &relocated,
            &[],
        )
        .unwrap();
        assert_eq!(
            report
                .rows
                .iter()
                .map(|row| row.outcome)
                .collect::<Vec<_>>(),
            [0, 1, 2, 0, 1].map(Outcome::Captured),
        );
    }

    #[test]
    fn reports_incomplete_or_ambiguous_relocations_at_their_source() {
        let cases = [
            (
                "source = missing.conf\n",
                "/config/root.conf:1",
                "Choose its file",
            ),
            (
                "source = $root/*.conf\n",
                "/config/root.conf:1",
                "literal path",
            ),
            (
                "source = ./root.conf\n",
                "/config/root.conf:1",
                "Remove the cycle",
            ),
            (
                "general {\n source = other.conf\n}\n",
                "/config/root.conf:2",
                "top-level",
            ),
            (
                "sou\\\nrce = old.conf\n",
                "/config/root.conf:1",
                "Continued lines",
            ),
            (
                "$name = urce\nso$name = old.conf\n",
                "/config/root.conf:2",
                "directive name",
            ),
        ];
        for (source, location, reason) in cases {
            let error = prepare(b"/config/root.conf", source, &[], &[])
                .err()
                .unwrap();
            assert!(
                error.contains(location) && error.contains(reason),
                "{error}"
            );
        }
        let nested = captured("/config/child.conf", "source = missing.conf\n");
        let error = prepare(
            b"/config/root.conf",
            "source = child.conf\n",
            &[nested],
            &[],
        )
        .err()
        .unwrap();
        assert!(error.starts_with("/config/child.conf:1"), "{error}");
    }
}
