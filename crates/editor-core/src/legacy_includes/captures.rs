//! Ordered references derived from retained native source bytes.
use super::scan;
use crate::{
    legacy_syntax, legacy_variables::Revision, source::CapturedSource, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
};

pub const MAX_FILES: usize = 31;
pub const MAX_BYTES: usize = 2 * MAX_DOCUMENT_BYTES;
pub(super) const MAX_DEPTH: usize = 16;
const MAX_REFERENCES: usize = 1024;

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

struct Target {
    line: usize,
    template: String,
    expected: Option<PathBuf>,
}

struct File {
    targets: Vec<Target>,
    opaque: usize,
}

fn index_file(path: &[u8], source: &str) -> Result<File, String> {
    let parsed = scan(source)?;
    let targets = parsed
        .rows
        .into_iter()
        .zip(parsed.lines)
        .map(|(row, line)| {
            let expected = literal_path(path, &row.template);
            Target {
                line,
                template: row.template,
                expected,
            }
        })
        .collect();
    Ok(File {
        targets,
        opaque: parsed.unresolved,
    })
}

pub fn inspect(
    root_path: &[u8],
    root_content: &str,
    files: &[CapturedSource],
    revisions: &[LinkedRevision],
) -> Result<References, String> {
    let root_path = absolute_components(Path::new(OsStr::from_bytes(root_path)))
        .ok_or("Invalid including path.")?;
    let root = index_file(root_path.as_os_str().as_bytes(), root_content)?;
    let mut by_path = BTreeMap::new();
    let mut indexed = Vec::with_capacity(files.len());
    for (index, file) in files.iter().enumerate() {
        let path = Path::new(OsStr::from_bytes(&file.identity.selected_path));
        let path = absolute_components(path).ok_or("Invalid snapshot path.")?;
        by_path.insert(path, index);
        indexed.push(
            index_file(
                &file.identity.selected_path,
                effective_text(file, revisions),
            )
            .unwrap_or(File {
                targets: Vec::new(),
                opaque: 1,
            }),
        );
    }

    let mut traversal = Traversal {
        by_path,
        root_path,
        ancestry: Vec::new(),
        result: References::default(),
    };
    traversal.visit(None, &root, &indexed);
    Ok(traversal.result)
}

struct Traversal {
    by_path: BTreeMap<PathBuf, usize>,
    root_path: PathBuf,
    ancestry: Vec<usize>,
    result: References,
}

impl Traversal {
    fn visit(&mut self, parent: Option<usize>, file: &File, indexed: &[File]) {
        self.result.opaque_directives += file.opaque;
        for target in &file.targets {
            if self.result.rows.len() >= MAX_REFERENCES {
                self.result.truncated = true;
                break;
            }
            let outcome = match &target.expected {
                None => Outcome::UnresolvedTemplate,
                Some(path) if *path == self.root_path => Outcome::RootCycle,
                Some(path) => match self.by_path.get(path).copied() {
                    None => Outcome::NeedsSelection,
                    Some(index) if self.ancestry.contains(&index) => Outcome::Cycle(index),
                    Some(_) if self.ancestry.len() >= MAX_DEPTH => Outcome::DepthLimit,
                    Some(index) => Outcome::Captured(index),
                },
            };
            self.result.rows.push(Reference {
                parent,
                line: target.line,
                template: target.template.clone(),
                expected: target.expected.clone(),
                outcome,
            });
            if let Outcome::Captured(index) = outcome {
                self.result.reached.insert(index);
                self.ancestry.push(index);
                self.visit(Some(index), &indexed[index], indexed);
                self.ancestry.pop();
            }
        }
    }
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
}
