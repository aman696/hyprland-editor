//! Ordered source evidence for native scalar settings. No host path resolution.
use super::captures::{self, LinkedRevision, MAX_DEPTH};
use crate::{
    import,
    legacy_syntax::{code, trim},
    source::CapturedSource,
    Setting, BASELINE,
};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::Arc,
};

const MAX_EVENTS: usize = 16_384;
const MAX_HISTORY: usize = 32;
const MAX_ISSUES: usize = 64;
const MAX_VALUE_TEXT: usize = 1_024;
const MAX_SCALAR_LITERAL: usize = 16_384;
const MAX_KEY: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Literal,
    Expansion,
    Unverified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueKind {
    MissingInclude,
    UnresolvedInclude,
    IncludeCycle,
    IncludeDepth,
    Conditional,
    ComputedKey,
    Syntax,
    Limit,
}

#[derive(Debug)]
pub struct Issue {
    pub file: Option<usize>,
    pub line: Option<usize>,
    pub kind: IssueKind,
}

#[derive(Debug)]
pub struct Assignment {
    pub file: Option<usize>,
    pub line: usize,
    pub value: Arc<str>,
    pub kind: ValueKind,
}

#[derive(Debug, Default)]
pub struct History {
    pub assignments: VecDeque<Assignment>,
    pub total: usize,
}

#[derive(Debug, Default)]
pub struct Report {
    pub settings: BTreeMap<String, History>,
    pub issues: Vec<Issue>,
    pub issue_count: usize,
}

impl Report {
    fn issue(&mut self, parent: usize, line: Option<usize>, kind: IssueKind) {
        self.issue_count += 1;
        if self.issues.len() < MAX_ISSUES {
            self.issues.push(Issue {
                file: parent.checked_sub(1),
                line,
                kind,
            });
        }
    }
}

enum Item {
    Include(Result<usize, IssueKind>),
    Issue(IssueKind),
    Setting(usize, Arc<str>, ValueKind),
}

struct Event {
    line: usize,
    item: Item,
}

/// Trace the materialized root and kept linked revisions using only captured bytes.
pub fn inspect(
    root_path: &[u8],
    root: &str,
    files: &[CapturedSource],
    revisions: &[LinkedRevision],
    catalogue: &[Setting],
) -> Result<Report, String> {
    captures::validate(files)?;
    captures::validate_revisions(files, revisions)?;
    if root.len() > crate::MAX_DOCUMENT_BYTES || root.contains('\0') {
        return Err("The native root exceeds its size policy or contains NUL.".into());
    }
    let sources = std::iter::once((root_path, root))
        .chain(files.iter().map(|file| {
            (
                file.identity.selected_path.as_slice(),
                captures::effective_text(file, revisions),
            )
        }))
        .collect::<Vec<_>>();
    let mut by_path = BTreeMap::new();
    for (index, (path, _)) in sources.iter().enumerate() {
        let path = captures::absolute_components(Path::new(OsStr::from_bytes(path)))
            .ok_or("Invalid captured source path.")?;
        if by_path.insert(path, index).is_some() {
            return Err("A captured source duplicates the root path.".into());
        }
    }
    let by_key = catalogue
        .iter()
        .enumerate()
        .filter(|(_, setting)| setting.version == BASELINE)
        .map(|(index, setting)| (setting.id.as_str(), (index, setting)))
        .collect::<BTreeMap<_, _>>();
    let mut walker = Traversal {
        indexed: vec![None; sources.len()],
        sources,
        by_path,
        by_key,
        catalogue,
        report: Report::default(),
        replay_budget: MAX_EVENTS,
        index_budget: MAX_EVENTS,
    };
    walker.visit(0, &mut Vec::new());
    Ok(walker.report)
}

struct Traversal<'a> {
    sources: Vec<(&'a [u8], &'a str)>,
    indexed: Vec<Option<Arc<[Event]>>>,
    by_path: BTreeMap<PathBuf, usize>,
    by_key: BTreeMap<&'a str, (usize, &'a Setting)>,
    catalogue: &'a [Setting],
    report: Report,
    replay_budget: usize,
    index_budget: usize,
}

impl Traversal<'_> {
    fn visit(&mut self, parent: usize, ancestors: &mut Vec<usize>) {
        ancestors.push(parent);
        let events = self.indexed[parent]
            .get_or_insert_with(|| {
                let (path, text) = self.sources[parent];
                index_file(
                    path,
                    text,
                    &self.by_path,
                    &self.by_key,
                    &mut self.index_budget,
                )
                .into()
            })
            .clone();
        for event in events.iter() {
            if self.replay_budget == 0 {
                self.report
                    .issue(parent, Some(event.line), IssueKind::Limit);
                break;
            }
            self.replay_budget -= 1;
            match &event.item {
                Item::Include(target) => {
                    match target.and_then(|child| include_target(child, ancestors)) {
                        Ok(child) => self.visit(child, ancestors),
                        Err(issue) => self.report.issue(parent, Some(event.line), issue),
                    }
                }
                Item::Issue(kind) => self.report.issue(parent, Some(event.line), *kind),
                Item::Setting(index, value, kind) => {
                    let key = &self.catalogue[*index].id;
                    let history = self.report.settings.entry(key.clone()).or_default();
                    history.total += 1;
                    if history.assignments.len() == MAX_HISTORY {
                        history.assignments.pop_front();
                    }
                    history.assignments.push_back(Assignment {
                        file: parent.checked_sub(1),
                        line: event.line,
                        value: value.clone(),
                        kind: *kind,
                    });
                }
            }
        }
        ancestors.pop();
    }
}

fn include_target(child: usize, ancestors: &[usize]) -> Result<usize, IssueKind> {
    if ancestors.contains(&child) {
        return Err(IssueKind::IncludeCycle);
    }
    if ancestors.len() > MAX_DEPTH {
        return Err(IssueKind::IncludeDepth);
    }
    Ok(child)
}

fn index_file(
    path: &[u8],
    text: &str,
    by_path: &BTreeMap<PathBuf, usize>,
    by_key: &BTreeMap<&str, (usize, &Setting)>,
    remaining: &mut usize,
) -> Vec<Event> {
    let mut indexed = Vec::new();
    let mut scopes: Vec<String> = Vec::new();
    let mut lines = text.split_terminator('\n').enumerate();
    let mut joined = String::new();
    while let Some((line, physical)) = lines.next() {
        let line = line + 1;
        if *remaining == 0 {
            indexed.push(Event {
                line,
                item: Item::Issue(IssueKind::Limit),
            });
            break;
        }
        let continued = physical.ends_with('\\');
        let logical = if continued {
            joined.clear();
            joined.push_str(physical);
            while let Some(prefix) = joined.strip_suffix('\\') {
                let len = prefix.trim_end_matches([' ', '\t']).len();
                joined.truncate(len);
                let Some((_, next)) = lines.next() else {
                    indexed.push(Event {
                        line,
                        item: Item::Issue(IssueKind::Syntax),
                    });
                    return indexed;
                };
                joined.push_str(next);
            }
            joined.as_str()
        } else {
            physical
        };
        let Some(item) = item(path, logical, &mut scopes, by_path, by_key) else {
            continue;
        };
        indexed.push(Event { line, item });
        *remaining -= 1;
    }
    if !scopes.is_empty() && *remaining != 0 {
        indexed.push(Event {
            line: text.lines().count().max(1),
            item: Item::Issue(IssueKind::Syntax),
        });
    }
    indexed
}

fn item(
    path: &[u8],
    line: &str,
    scopes: &mut Vec<String>,
    by_path: &BTreeMap<PathBuf, usize>,
    by_key: &BTreeMap<&str, (usize, &Setting)>,
) -> Option<Item> {
    let line = trim(line);
    if let Some(comment) = line.strip_prefix('#') {
        let comment = trim(comment);
        return (comment.starts_with("hyprlang")
            && comment
                .split_ascii_whitespace()
                .any(|word| matches!(word, "if" | "endif")))
        .then_some(Item::Issue(IssueKind::Conditional));
    }
    let (decoded, _) = code(line);
    let code = trim(&decoded);
    if let Some((key, raw)) = code.split_once('=') {
        let key = trim(key);
        if key.starts_with('$') {
            return None;
        }
        if key.contains('$') {
            return Some(Item::Issue(IssueKind::ComputedKey));
        }
        if key == "source" {
            let raw = trim(raw);
            let target = (scopes.is_empty() && super::validate_template(raw).is_ok())
                .then(|| captures::literal_path(path, raw))
                .flatten()
                .filter(|target| {
                    !target
                        .as_os_str()
                        .as_bytes()
                        .iter()
                        .any(|b| matches!(b, b'\\' | b'*' | b'?' | b'['))
                });
            let child = target
                .ok_or(IssueKind::UnresolvedInclude)
                .and_then(|target| {
                    by_path
                        .get(&target)
                        .copied()
                        .ok_or(IssueKind::MissingInclude)
                });
            return Some(Item::Include(child));
        }
        if key.is_empty() {
            return Some(Item::Issue(IssueKind::Syntax));
        }
        if scopes.iter().map(|part| part.len() + 1).sum::<usize>() + key.len() > MAX_KEY {
            return None;
        }
        let full = if scopes.is_empty() {
            key.into()
        } else {
            format!("{}:{key}", scopes.join(":"))
        };
        let &(index, setting) = by_key.get(full.as_str())?;
        let raw = trim(raw);
        let kind = if raw.contains('$') || raw.contains("{{") {
            ValueKind::Expansion
        } else if raw.len() > MAX_SCALAR_LITERAL
            || raw.contains('\\')
            || import::literal(setting, raw).is_none_or(|value| setting.validate(&value).is_err())
        {
            ValueKind::Unverified
        } else {
            ValueKind::Literal
        };
        return Some(Item::Setting(index, clipped(raw), kind));
    }
    if code == "}" {
        return scopes
            .pop()
            .is_none()
            .then_some(Item::Issue(IssueKind::Syntax));
    }
    if let Some(name) = code.strip_suffix('{').map(trim) {
        if scopes.len() == 16 || name.is_empty() || code.contains('}') {
            return Some(Item::Issue(IssueKind::Syntax));
        }
        if name.len() > MAX_KEY {
            // Keep the nesting boundary without matching this unsupported category.
            scopes.push("\0".into());
            return Some(Item::Issue(IssueKind::Syntax));
        }
        scopes.push(name.into());
        return None;
    }
    (!code.is_empty()).then_some(Item::Issue(IssueKind::Syntax))
}

fn clipped(text: &str) -> Arc<str> {
    if text.len() <= MAX_VALUE_TEXT {
        return text.into();
    }
    let end = (0..MAX_VALUE_TEXT)
        .rev()
        .find(|&index| text.is_char_boundary(index))
        .expect("zero is a UTF-8 boundary");
    format!("{}…", &text[..end]).into()
}
