//! Ordered references derived from retained native source bytes.
use super::scan;
use crate::{legacy_syntax, source::CapturedSource, MAX_DOCUMENT_BYTES};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
};

pub const MAX_FILES: usize = 31;
pub const MAX_BYTES: usize = 2 * MAX_DOCUMENT_BYTES;
const MAX_DEPTH: usize = 16;
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

fn absolute_components(path: &Path) -> Option<PathBuf> {
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

fn literal_path(parent: &[u8], template: &str) -> Option<PathBuf> {
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
            index_file(&file.identity.selected_path, &file.content).unwrap_or(File {
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
}
