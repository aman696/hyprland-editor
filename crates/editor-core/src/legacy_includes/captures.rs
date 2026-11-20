//! Ordered references derived from retained native source bytes.
use crate::{
    legacy_syntax, legacy_variables::Revision, source::CapturedSource, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
};

pub const MAX_FILES: usize = 31;
pub const MAX_BYTES: usize = 2 * MAX_DOCUMENT_BYTES;
pub(super) const MAX_DEPTH: usize = 16;
pub(super) const MAX_REFERENCES: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Captured(usize),
    RootCycle,
    Cycle(usize),
    NeedsSelection,
    UnresolvedTemplate,
    DepthLimit,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Reference {
    pub parent: Option<usize>,
    pub line: usize,
    pub template: String,
    pub resolved: Option<String>,
    pub expected: Option<PathBuf>,
    pub outcome: Outcome,
}

#[derive(Debug, Default)]
pub struct References {
    pub rows: Vec<Reference>,
    pub reached: BTreeSet<usize>,
    pub opaque_directives: usize,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkedRevision {
    pub path: Vec<u8>,
    pub revision: Revision,
}

pub fn effective_text<'a>(file: &'a CapturedSource, revisions: &'a [LinkedRevision]) -> &'a str {
    revisions
        .iter()
        .find(|r| r.path == file.identity.selected_path)
        .map_or(file.content.as_str(), |r| r.revision.content.as_str())
}

pub fn validate_revisions(
    files: &[CapturedSource],
    revisions: &[LinkedRevision],
) -> Result<(), String> {
    if revisions.len() > files.len() {
        return Err("Linked revisions exceed captured files.".into());
    }
    let mut paths = BTreeSet::new();
    let mut total = 0;
    for r in revisions {
        if !paths.insert(r.path.as_slice()) {
            return Err("Duplicate linked file revision.".into());
        }
        let file = files
            .iter()
            .find(|f| f.identity.selected_path == r.path)
            .ok_or("A linked revision has no original snapshot.")?;
        r.revision.validate(&file.content)?;
        total += r.revision.content.len();
        if total > MAX_BYTES {
            return Err("Linked file revisions exceed 2 MiB.".into());
        }
    }
    Ok(())
}

pub fn validate(files: &[CapturedSource]) -> Result<(), String> {
    if files.len() > MAX_FILES {
        return Err("This project can keep at most 31 linked file snapshots.".into());
    }
    let mut total = 0;
    let mut paths = BTreeSet::new();
    for file in files {
        file.identity.validate(&file.content)?;
        total += file.content.len();
        if total > MAX_BYTES {
            return Err("Linked file snapshots exceed 2 MiB.".into());
        }
        let selected = Path::new(OsStr::from_bytes(&file.identity.selected_path));
        let path = absolute_components(selected).ok_or("Invalid snapshot path.")?;
        if !paths.insert(path) {
            return Err("This path is already captured.".into());
        }
    }
    Ok(())
}

pub(super) fn absolute_components(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut result = PathBuf::from("/");
    for part in path.components() {
        match part {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(name) => result.push(name),
            Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    Some(result)
}

pub fn matches_selection(expected: &Path, selected: &Path) -> bool {
    absolute_components(selected).as_deref() == Some(expected)
}

pub(super) fn literal_path(parent: &[u8], template: &str) -> Option<PathBuf> {
    // Expansion, glob escaping and symlink-sensitive parent traversal need
    // separate resolution evidence. This matcher consumes capture metadata only.
    if template.ends_with('/')
        || template.ends_with("/.")
        || template == "."
        || template.starts_with('~')
        || template.contains(['$', '\\', '*', '?', '[', ']', '{', '}'])
        || legacy_syntax::has_expansion(template)
    {
        return None;
    }
    let path = Path::new(template);
    let parent = Path::new(OsStr::from_bytes(parent)).parent()?;
    absolute_components(&parent.join(path))
}

pub fn inspect(
    root_path: &[u8],
    root_content: &str,
    files: &[CapturedSource],
    revisions: &[LinkedRevision],
) -> Result<References, String> {
    super::effective::references(root_path, root_content, files, revisions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{RecordedOwnership, SourceIdentity};

    fn file(path: &str, body: &str) -> CapturedSource {
        CapturedSource {
            content: body.into(),
            identity: SourceIdentity {
                selected_path: path.as_bytes().to_vec(),
                resolved_path: None,
                links: Vec::new(),
                device: 1,
                inode: 1,
                owner: 1000,
                mode: 0o100600,
                size: body.len() as u64,
                modified_seconds: 0,
                modified_nanoseconds: 0,
                changed_seconds: 0,
                changed_nanoseconds: 0,
                sha256: crate::lua::source_hash(body),
                recorded_ownership: RecordedOwnership::UserOwned,
            },
        }
    }

    #[test]
    fn traverses_saved_literal_edges_in_order_and_keeps_detached_files() {
        let files = [
            file("/virtual/parts/colors.conf", "source = ./nested.conf\n"),
            file(
                "/virtual/parts/nested.conf",
                "source = colors.conf\nsource = /virtual/root.conf\n",
            ),
            file("/virtual/old.conf", "source = /never/read.conf\n"),
        ];
        validate(&files).unwrap();
        let report = inspect(
            b"/virtual/root.conf",
            "source = parts//colors.conf\nsource = $theme/*.conf\nsource = ../elsewhere.conf\nsource = parts/colors.conf\nsource = missing.conf\n",
            &files,
            &[],
        ).unwrap();
        assert_eq!(report.reached, BTreeSet::from([0, 1]));
        assert_eq!(
            report
                .rows
                .iter()
                .map(|r| (r.parent, r.line, r.outcome))
                .collect::<Vec<_>>(),
            [
                (None, 1, Outcome::Captured(0)),
                (Some(0), 1, Outcome::Captured(1)),
                (Some(1), 1, Outcome::Cycle(0)),
                (Some(1), 2, Outcome::RootCycle),
                (None, 2, Outcome::UnresolvedTemplate),
                (None, 3, Outcome::UnresolvedTemplate),
                (None, 4, Outcome::Captured(0)),
                (Some(0), 1, Outcome::Captured(1)),
                (Some(1), 1, Outcome::Cycle(0)),
                (Some(1), 2, Outcome::RootCycle),
                (None, 5, Outcome::NeedsSelection),
            ]
        );
        assert_eq!(
            report.rows.last().unwrap().expected.as_deref(),
            Some(Path::new("/virtual/missing.conf"))
        );
    }

    #[test]
    fn linked_edits_change_outgoing_references_and_are_tied_to_original_bytes() {
        let captured = file("/virtual/child.conf", "source = original.conf\n");
        let revised = LinkedRevision {
            path: captured.identity.selected_path.clone(),
            revision: Revision::new(&captured.content, "source = changed.conf\n".into()).unwrap(),
        };
        let files = [captured.clone()];
        validate_revisions(&files, std::slice::from_ref(&revised)).unwrap();
        let inspected = inspect(
            b"/virtual/root.conf",
            "source = child.conf\n",
            &files,
            std::slice::from_ref(&revised),
        )
        .unwrap();
        assert_eq!(
            inspected.rows[1].expected.as_deref(),
            Some(Path::new("/virtual/changed.conf"))
        );
        assert_eq!(files[0].content, "source = original.conf\n");
        assert!(validate_revisions(
            &[file("/virtual/child.conf", "source = replacement.conf\n")],
            std::slice::from_ref(&revised)
        )
        .is_err());
        assert!(validate_revisions(&[], &[revised]).is_err());
    }

    #[test]
    fn validates_capture_bytes_and_rejects_duplicate_or_ambiguous_paths() {
        let mut captured = file("/virtual/file.conf", "# original\n");
        assert!(matches_selection(
            Path::new("/virtual/file.conf"),
            Path::new("/virtual/./file.conf")
        ));
        assert!(!matches_selection(
            Path::new("/virtual/file.conf"),
            Path::new("/virtual/other/../file.conf")
        ));
        let duplicate = file("/virtual/./file.conf", "# another\n");
        assert!(validate(&[captured.clone(), duplicate]).is_err());
        captured.content.push('!');
        assert!(validate(&[captured]).is_err());
    }

    #[test]
    fn source_setting_trace_interleaves_repeated_files_and_kept_revisions() {
        use crate::legacy_includes::effective::{self, ValueKind};
        let files = [
            file(
                "/config/leaf.conf",
                "general {\r\n border_size = 5\r\n}\r\n",
            ),
            file("/config/layout.conf", "general:border_size = 3\n"),
            file("/config/detached.conf", "source = never-read.conf\n"),
        ];
        let revision = LinkedRevision {
            path: files[1].identity.selected_path.clone(),
            revision: Revision::new(
                &files[1].content,
                "general:border_size = 4\nsource = leaf.conf\n".into(),
            )
            .unwrap(),
        };
        let report = effective::inspect(
            b"/config/root.conf",
            "general:border_size = 2\nsource = layout.conf\ngeneral:border_size = 8\nsource = /config/layout.conf\n",
            &files,
            &[revision],
            &crate::catalogue(),
        )
        .unwrap();
        assert_eq!(report.issue_count, 0);
        let history = &report.settings["general:border_size"];
        assert_eq!(history.total, 6);
        assert_eq!(
            history
                .assignments
                .iter()
                .map(|a| (a.file, a.line, a.value.as_ref(), a.kind))
                .collect::<Vec<_>>(),
            [
                (None, 1, "2", ValueKind::Literal),
                (Some(1), 1, "4", ValueKind::Literal),
                (Some(0), 2, "5", ValueKind::Literal),
                (None, 3, "8", ValueKind::Literal),
                (Some(1), 1, "4", ValueKind::Literal),
                (Some(0), 2, "5", ValueKind::Literal),
            ]
        );
    }

    #[test]
    fn source_setting_trace_retains_uncertain_assignments_and_their_locations() {
        use crate::legacy_includes::effective::{self, IssueKind, ValueKind};
        let files = [file("/config/child.conf", "source = root.conf\n")];
        let report = effective::inspect(
            b"/config/root.conf",
            concat!(
                "general:border_size = 4\n",
                "source = child.conf\n",
                "source = missing.conf\n",
                "source = $path/*.conf\n",
                "# hyprlang if FLAG\n",
                "general:border_size = $size\n",
                "# hyprlang endif\n",
                "general:bord\\\ner_size = 9\n",
                "so$name = path.conf\n",
                "general:gaps_in = nonsense\n"
            ),
            &files,
            &[],
            &crate::catalogue(),
        )
        .unwrap();
        assert_eq!(report.issue_count, 6);
        assert_eq!(
            report
                .issues
                .iter()
                .map(|issue| (issue.file, issue.line, issue.kind))
                .collect::<Vec<_>>(),
            [
                (Some(0), Some(1), IssueKind::IncludeCycle),
                (None, Some(3), IssueKind::MissingInclude),
                (None, Some(4), IssueKind::UnresolvedInclude),
                (None, Some(5), IssueKind::Conditional),
                (None, Some(7), IssueKind::Conditional),
                (None, Some(10), IssueKind::ComputedKey),
            ],
        );
        let assignments = &report.settings["general:border_size"].assignments;
        assert_eq!(assignments[1].kind, ValueKind::Expansion);
        assert_eq!(
            (assignments[2].line, assignments[2].value.as_ref()),
            (8, "9")
        );
        assert_eq!(
            report.settings["general:gaps_in"].assignments[0].kind,
            ValueKind::Unverified
        );
    }

    #[test]
    fn source_setting_trace_bounds_repeated_history_and_marks_truncated_traversal() {
        use crate::legacy_includes::effective::{self, IssueKind};
        let source = "general:border_size = 3\n".repeat(2_000);
        let files = [file("/config/child.conf", &source)];
        let report = effective::inspect(
            b"/config/root.conf",
            &"source = child.conf\n".repeat(9),
            &files,
            &[],
            &crate::catalogue(),
        )
        .unwrap();
        let history = &report.settings["general:border_size"];
        assert!(history.total > 16_000 && history.total < 18_000);
        assert_eq!(history.assignments.len(), 32);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::Limit));
    }

    #[test]
    fn ordered_variables_follow_nested_includes_and_keep_definition_provenance() {
        use crate::legacy_includes::effective::{self, ValueKind};
        let source = concat!(
            "$dir = parts\n",
            "$target = $dir/theme.conf\n",
            "$size = 4\n",
            "source = $target\n",
            "general:border_size = {{ size + 1 }}\n",
            "source = $palette\n",
            "source = $target\n",
            "general:border_size = $size\n",
        );
        let files = [
            file(
                "/config/parts/theme.conf",
                "general:border_size = $size\n$size = {{ size + 2 }}\n$palette = parts/palette.conf\n",
            ),
            file("/config/parts/palette.conf", "general:border_size = $size\n"),
        ];
        let report = effective::inspect(
            b"/config/root.conf",
            source,
            &files,
            &[],
            &crate::catalogue(),
        )
        .unwrap();
        assert_eq!(report.issue_count, 0);
        let trace = &report.settings["general:border_size"];
        let expanded = trace
            .assignments
            .iter()
            .map(|value| {
                assert_eq!(value.kind, ValueKind::Resolved);
                value.expanded.as_ref().unwrap().value.as_ref()
            })
            .collect::<Vec<_>>();
        assert_eq!(expanded, ["4", "7", "6", "6", "8"]);
        let origins = &trace
            .assignments
            .back()
            .unwrap()
            .expanded
            .as_ref()
            .unwrap()
            .definitions;
        assert_eq!(
            origins
                .iter()
                .map(|origin| (origin.name.as_ref(), origin.file, origin.line))
                .collect::<Vec<_>>(),
            [("size", None, 3), ("size", Some(0), 2)],
        );
        let refs = effective::references(b"/config/root.conf", source, &files, &[]).unwrap();
        assert_eq!(refs.rows.len(), 3);
        assert_eq!(refs.rows[1].template, "$palette");
        assert_eq!(refs.rows[1].resolved.as_deref(), Some("parts/palette.conf"));
        assert_eq!(refs.rows[1].outcome, Outcome::Captured(1));
    }

    #[test]
    fn unresolved_includes_block_later_variable_path_suggestions_until_captured() {
        use crate::legacy_includes::effective;
        let source = concat!(
            "$first = first.conf\n",
            "$next = fallback.conf\n",
            "source = $first\n",
            "source = $next\n",
            "source = literal.conf\n",
        );
        let root = b"/config/root.conf";
        let refs = effective::references(root, source, &[], &[]).unwrap();
        assert_eq!(
            refs.rows.iter().map(|row| row.outcome).collect::<Vec<_>>(),
            [
                Outcome::NeedsSelection,
                Outcome::UnresolvedTemplate,
                Outcome::NeedsSelection
            ],
        );
        assert_eq!(
            refs.rows[0].expected.as_deref(),
            Some(Path::new("/config/first.conf"))
        );
        assert!(refs.rows[1].expected.is_none());
        let first = file("/config/first.conf", "$next = later.conf\n");
        let refs = effective::references(root, source, &[first], &[]).unwrap();
        assert_eq!(refs.rows[0].outcome, Outcome::Captured(0));
        assert_eq!(
            refs.rows[1].expected.as_deref(),
            Some(Path::new("/config/later.conf"))
        );
    }

    #[test]
    fn ordered_variables_match_longest_prefix_and_bound_unverified_expressions() {
        use crate::legacy_includes::effective::{self, ValueKind};
        let report = effective::inspect(
            b"/config/root.conf",
            concat!(
                "$k = us\n",
                "$kb = fr\n",
                "input:kb_layout = $kbSuffix\n",
                "$kbSuffix = $unknown\n",
                "input:kb_layout = $kbSuffix\n",
                "input:kb_layout = $k$kb\n",
                "input:kb_layout = {{ 4060.28125 + 0 }}\n",
                "input:kb_layout = {{ 1 / 0 }}\n",
                "input:kb_layout = {{ 100000 + 0 }}\n",
                "input:kb_layout = {{ 1e-1000 + 1 }}\n"
            ),
            &[],
            &[],
            &crate::catalogue(),
        )
        .unwrap();
        let trace = &report.settings["input:kb_layout"].assignments;
        assert_eq!(
            trace[0].expanded.as_ref().unwrap().value.as_ref(),
            "frSuffix"
        );
        assert_eq!(trace[1].kind, ValueKind::Expansion);
        assert_eq!(trace[2].kind, ValueKind::Expansion);
        assert_eq!(
            trace[3].expanded.as_ref().unwrap().value.as_ref(),
            "4060.2812"
        );
        assert_eq!(trace[4].kind, ValueKind::Expansion);
        assert_eq!(trace[5].kind, ValueKind::Expansion);
        assert_eq!(trace[6].kind, ValueKind::Expansion);
    }

    #[test]
    fn variable_include_export_rewrites_stable_paths_and_refuses_conflicting_visits() {
        use crate::legacy_includes::project_copy;
        let files = [
            file("/config/parts/includes.conf", "source = $target # keep\n"),
            file("/config/parts/a.conf", "general:border_size = 5\n"),
            file("/config/parts/b.conf", "general:border_size = 7\n"),
        ];
        let root = b"/config/root.conf";
        let prefix = "$target = a.conf\nsource = parts/includes.conf\n";
        let copy = project_copy::prepare(root, prefix, &files, &[]).unwrap();
        assert_eq!(copy.include_count, 2);
        assert_eq!(copy.detached_count, 1);
        assert_eq!(copy.files[1].content, "source = 02-a.conf # keep\n");
        let conflict = format!("{prefix}$target = b.conf\nsource = parts/includes.conf\n");
        let error = project_copy::prepare(root, &conflict, &files, &[])
            .err()
            .unwrap();
        assert!(error.contains("/config/parts/includes.conf:1"));
        assert!(error.contains("different files on repeated visits"));
    }
}
