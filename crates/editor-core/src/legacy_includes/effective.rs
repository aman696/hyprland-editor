//! Ordered source evidence for native scalar settings. No host path resolution.
pub use super::variables::Origin as VariableOrigin;
use super::{
    captures::{self, LinkedRevision, Outcome, Reference, References, MAX_DEPTH, MAX_REFERENCES},
    variables::Variables,
};
use crate::{
    import,
    legacy_syntax::{code, trim, unescape},
    source::CapturedSource,
    Setting, BASELINE,
};
use std::{
    borrow::Cow,
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
const MAX_VARIABLE_TEXT: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Literal,
    Resolved,
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
    VariableContext,
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
    pub expanded: Option<ExpandedValue>,
}

#[derive(Debug)]
pub struct ExpandedValue {
    pub value: Arc<str>,
    pub definitions: Vec<VariableOrigin>,
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
    Include(Arc<str>),
    Define(Arc<str>, Option<Arc<str>>),
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
    Ok(walk(root_path, root, files, revisions, catalogue, false)?.0)
}

/// References used by the explicit linked-file capture picker.
pub(super) fn references(
    root_path: &[u8],
    root: &str,
    files: &[CapturedSource],
    revisions: &[LinkedRevision],
) -> Result<References, String> {
    Ok(walk(root_path, root, files, revisions, &[], true)?.1)
}

fn walk(
    root_path: &[u8],
    root: &str,
    files: &[CapturedSource],
    revisions: &[LinkedRevision],
    catalogue: &[Setting],
    collect_references: bool,
) -> Result<(Report, References), String> {
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
        variables: Variables::default(),
        references: collect_references.then(References::default),
        replay_budget: MAX_EVENTS,
        index_budget: MAX_EVENTS,
    };
    walker.visit(0, &mut Vec::new());
    Ok((walker.report, walker.references.unwrap_or_default()))
}

struct Traversal<'a> {
    sources: Vec<(&'a [u8], &'a str)>,
    indexed: Vec<Option<Arc<[Event]>>>,
    by_path: BTreeMap<PathBuf, usize>,
    by_key: BTreeMap<&'a str, (usize, &'a Setting)>,
    catalogue: &'a [Setting],
    report: Report,
    variables: Variables,
    references: Option<References>,
    replay_budget: usize,
    index_budget: usize,
}

impl Traversal<'_> {
    fn visit(&mut self, parent: usize, ancestors: &mut Vec<usize>) {
        ancestors.push(parent);
        let events = self.indexed[parent]
            .get_or_insert_with(|| {
                index_file(self.sources[parent].1, &self.by_key, &mut self.index_budget).into()
            })
            .clone();
        for event in events.iter() {
            if self.replay_budget == 0 {
                self.issue(parent, event.line, IssueKind::Limit);
                break;
            }
            self.replay_budget -= 1;
            match &event.item {
                Item::Include(template) => {
                    self.include(parent, event.line, template, ancestors);
                }
                Item::Define(name, raw) => {
                    self.variables
                        .define(name, raw.as_deref(), parent.checked_sub(1), event.line);
                }
                Item::Issue(kind) => {
                    if *kind == IssueKind::UnresolvedInclude {
                        if let Some(references) = &mut self.references {
                            references.opaque_directives += 1;
                        }
                    }
                    self.issue(parent, event.line, *kind);
                }
                Item::Setting(index, value, kind) => {
                    let setting = &self.catalogue[*index];
                    let expanded = if *kind == ValueKind::Expansion {
                        self.variables.expand(value).and_then(|expanded| {
                            Some((unescape(&expanded.text).ok()?, expanded.origins))
                        })
                    } else {
                        None
                    };
                    let kind = expanded.as_ref().map_or(*kind, |(computed, _)| {
                        if valid_scalar(setting, computed) {
                            ValueKind::Resolved
                        } else {
                            ValueKind::Unverified
                        }
                    });
                    let history = self.report.settings.entry(setting.id.clone()).or_default();
                    history.total += 1;
                    if history.assignments.len() == MAX_HISTORY {
                        history.assignments.pop_front();
                    }
                    history.assignments.push_back(Assignment {
                        file: parent.checked_sub(1),
                        line: event.line,
                        value: if value.len() > MAX_VALUE_TEXT {
                            clipped(value)
                        } else {
                            value.clone()
                        },
                        kind,
                        expanded: expanded.map(|(value, definitions)| ExpandedValue {
                            value: clipped(&value),
                            definitions,
                        }),
                    });
                }
            }
        }
        ancestors.pop();
    }

    fn issue(&mut self, parent: usize, line: usize, kind: IssueKind) {
        self.variables.invalidate();
        self.report.issue(parent, Some(line), kind);
        if let Some(references) = &mut self.references {
            match kind {
                IssueKind::Limit => references.truncated = true,
                IssueKind::Conditional
                | IssueKind::ComputedKey
                | IssueKind::Syntax
                | IssueKind::VariableContext => {
                    references.opaque_directives += 1;
                }
                _ => {}
            }
        }
    }

    fn include(&mut self, parent: usize, line: usize, template: &str, ancestors: &mut Vec<usize>) {
        if self
            .references
            .as_ref()
            .is_some_and(|refs| refs.rows.len() == MAX_REFERENCES)
        {
            self.issue(parent, line, IssueKind::Limit);
            self.replay_budget = 0;
            return;
        }
        let path = if needs_expansion(template) {
            self.variables
                .expand(template)
                .and_then(|value| unescape(&value.text).ok())
                .map(Cow::Owned)
        } else {
            Some(Cow::Borrowed(template))
        };
        let expected = path.as_deref().and_then(|raw| {
            super::validate_template(raw).ok()?;
            let target = captures::literal_path(self.sources[parent].0, raw)?;
            let bytes = target.as_os_str().as_bytes();
            (bytes.len() <= super::MAX_PATH
                && !bytes
                    .iter()
                    .any(|b| matches!(b, b'\\' | b'*' | b'?' | b'[')))
            .then_some(target)
        });
        let outcome = match expected.as_ref() {
            None => Outcome::UnresolvedTemplate,
            Some(path) => match self.by_path.get(path).copied() {
                None => Outcome::NeedsSelection,
                Some(0) => Outcome::RootCycle,
                Some(child) if ancestors.contains(&child) => Outcome::Cycle(child - 1),
                Some(_) if ancestors.len() > MAX_DEPTH => Outcome::DepthLimit,
                Some(child) => Outcome::Captured(child - 1),
            },
        };
        if let Some(refs) = &mut self.references {
            refs.rows.push(Reference {
                parent: parent.checked_sub(1),
                line,
                template: template.into(),
                resolved: path.filter(|value| value != template).map(Cow::into_owned),
                expected,
                outcome,
            });
            if let Outcome::Captured(child) = outcome {
                refs.reached.insert(child);
            }
        }
        let issue = match outcome {
            Outcome::Captured(child) => return self.visit(child + 1, ancestors),
            Outcome::NeedsSelection => IssueKind::MissingInclude,
            Outcome::UnresolvedTemplate => IssueKind::UnresolvedInclude,
            Outcome::DepthLimit => IssueKind::IncludeDepth,
            Outcome::RootCycle | Outcome::Cycle(_) => IssueKind::IncludeCycle,
        };
        self.issue(parent, line, issue);
    }
}

fn index_file(
    text: &str,
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
        let Some(item) = item(logical, &mut scopes, by_key) else {
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
    line: &str,
    scopes: &mut Vec<String>,
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
        if let Some(name) = key.strip_prefix('$') {
            return Some(if crate::legacy_variables::identifier(name) {
                let raw = trim(raw);
                Item::Define(
                    name.into(),
                    (raw.len() <= MAX_VARIABLE_TEXT).then(|| raw.into()),
                )
            } else {
                Item::Issue(IssueKind::VariableContext)
            });
        }
        if key.contains('$') {
            return Some(Item::Issue(IssueKind::ComputedKey));
        }
        if key == "source" {
            let raw = trim(raw);
            return Some(
                if scopes.is_empty() && super::validate_template(raw).is_ok() {
                    Item::Include(raw.into())
                } else {
                    Item::Issue(IssueKind::UnresolvedInclude)
                },
            );
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
        let kind = if needs_expansion(raw) && raw.len() <= MAX_VARIABLE_TEXT {
            ValueKind::Expansion
        } else if !raw.contains('\\') && valid_scalar(setting, raw) {
            ValueKind::Literal
        } else {
            ValueKind::Unverified
        };
        let value = if kind == ValueKind::Expansion {
            raw.into()
        } else {
            clipped(raw)
        };
        return Some(Item::Setting(index, value, kind));
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

fn needs_expansion(raw: &str) -> bool {
    raw.contains('$') || raw.contains("{{")
}

fn valid_scalar(setting: &Setting, raw: &str) -> bool {
    raw.len() <= MAX_SCALAR_LITERAL
        && import::literal(setting, raw).is_some_and(|value| setting.validate(&value).is_ok())
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
