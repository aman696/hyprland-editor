//! Ordered evaluation of an immutable source bundle under an explicit, non-executing model.
//! Module resolutions are supplied observations, never filesystem permissions or path guesses.
mod eval;
use super::{named, source_hash, syntax_tree, text};
use crate::{
    module_cache::{self, CachedValue},
    AUDITED_MAIN, BASELINE, MAX_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Arc,
};
use tree_sitter::{Node, Tree};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    HyprlandLua54,
    SyntheticEmit,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCallLine {
    Callee,
    Arguments,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerProfile {
    pub release: String,
    pub library_identity: String,
    pub error_call_line: Option<ErrorCallLine>,
    pub schema_sha256: String,
}
/// The model's linked compiler is an explicit assumption, not a detected target
/// compositor dependency. Unknown profiles cannot supply guessed call locations.
pub fn compiler_profile() -> CompilerProfile {
    static PROFILE: std::sync::LazyLock<CompilerProfile> = std::sync::LazyLock::new(|| {
        // SAFETY: original adapter returns only its compile-time header constant.
        let code = unsafe { hye_lua54_release() };
        let release = format!("{}.{}.{}", code / 10_000, (code / 100) % 100, code % 100);
        let mut identity = [0u8; 512];
        // SAFETY: original adapter writes at most this buffer's capacity.
        let length = unsafe { hye_lua54_identity(identity.as_mut_ptr(), identity.len()) };
        let library_identity = String::from_utf8_lossy(&identity[..length]).into_owned();
        let source = include_str!("../../../../data/lua-profiles.json");
        let matrix: serde_json::Value =
            serde_json::from_str(source).expect("Bundled Lua profile schema");
        let error_call_line = matrix["profiles"]
            .as_array()
            .expect("Bundled Lua profiles")
            .iter()
            .find(|row| {
                row["release"] == release && library_identity.contains(&format!("Lua {release} "))
            })
            .map(|row| {
                serde_json::from_value(row["error_call_line"].clone())
                    .expect("Bundled call-line rule")
            });
        CompilerProfile {
            release,
            library_identity,
            error_call_line,
            schema_sha256: source_hash(source),
        }
    });
    PROFILE.clone()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub id: String,
    pub source: String,
    pub sha256: String,
    pub load_name: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Loader {
    SafeRequire,
    OriginalRequire,
    Dofile,
    Loadfile,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub document: String,
    pub sha256: String,
    pub span: Range<usize>,
    pub loader: Loader,
    pub module: Vec<u8>,
    pub package_path: Option<Vec<u8>>,
    /// Complete initial templates followed by an uncaptured inherited tail.
    /// False + None means a wholly unknown path, never host defaults.
    #[serde(default)]
    pub package_path_unknown_tail: bool,
    pub working_directory: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Resolution {
    Loaded {
        document: String,
        loader_data: Vec<u8>,
    },
    Missing,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub request: Request,
    pub resolution: Resolution,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub version: String,
    pub profile: Profile,
    pub root: String,
    pub documents: Vec<Document>,
    pub bindings: Vec<Binding>,
    pub initial_package_path: Option<Vec<u8>>,
    #[serde(default)]
    pub initial_package_path_unknown_tail: bool,
    pub working_directory: Option<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Observed {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Bytes(Vec<u8>),
    Table(Vec<(Observed, Observed)>),
    Function(String),
    Cycle(usize),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub order: usize,
    pub document: String,
    pub span: Range<usize>,
    pub api: String,
    pub arguments: Vec<Observed>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LoadOutcome {
    Cached,
    Preload,
    SuppliedSnapshot {
        document: String,
        loader_data: Vec<u8>,
    },
    SuppliedMissing,
    NeedsResolution,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadEvent {
    pub order: usize,
    pub request: Request,
    pub outcome: LoadOutcome,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Halt {
    NeedsResolution {
        request: Request,
    },
    Unresolved {
        document: String,
        span: Range<usize>,
        reason: String,
    },
    LuaError {
        value: Observed,
    },
    Budget {
        reason: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub compiler_profile: CompilerProfile,
    pub version: String,
    pub profile: Profile,
    pub events: Vec<Event>,
    pub loads: Vec<LoadEvent>,
    pub returns: Vec<Observed>,
    pub halt: Option<Halt>,
    pub logged_module_errors: usize,
    pub runtime_verified: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Bytes(Arc<[u8]>),
    /// Known complete initial templates with an unknown optional inherited tail.
    /// Copying/overwriting this value preserves or replaces that provenance.
    PartialPath(Arc<[u8]>),
    Table(usize),
    Closure(usize),
    Native(Native),
    Unknown(&'static str),
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Native {
    Require(bool),
    Dofile,
    Loadfile,
    Observe(&'static str),
    Error,
    Pcall,
    Type,
    Rawget,
    Rawset,
    Select,
    Unmodeled(&'static str),
}
impl CachedValue for Value {
    fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }
    fn is_truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Bool(false))
    }
    fn is_lua_true(&self) -> bool {
        matches!(self, Self::Bool(true))
    }
    fn is_table(&self) -> bool {
        matches!(self, Self::Table(_))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Bool(bool),
    Int(i64),
    Float(u64),
    Bytes(Arc<[u8]>),
    Table(usize),
    Closure(usize),
}
#[derive(Clone, Default)]
struct Env {
    locals: BTreeMap<Arc<str>, usize>,
    varargs: Vec<Value>,
    variadic: bool,
}
struct Table {
    values: BTreeMap<Key, Value>,
    unknown: bool,
}
struct Closure<'a> {
    document: usize,
    body: Option<Node<'a>>,
    params: Vec<Arc<str>>,
    env: Env,
    variadic: bool,
}
struct Parsed<'a> {
    document: &'a Document,
    tree: std::result::Result<Tree, String>,
    compile: std::result::Result<(), CompileFailure>,
}
enum CompileFailure {
    Syntax(Vec<u8>),
    Budget,
}
unsafe extern "C" {
    fn hye_lua54_release() -> u32;
    fn hye_lua54_identity(output: *mut u8, capacity: usize) -> usize;
    fn hye_lua54_compile(
        source: *const u8,
        length: usize,
        name: *const libc::c_char,
        error: *mut u8,
        capacity: usize,
        written: *mut usize,
    ) -> libc::c_int;
}
fn compile(document: &Document) -> std::result::Result<(), CompileFailure> {
    let mut name = vec![b'@'];
    name.extend(&document.load_name);
    let name = std::ffi::CString::new(name).map_err(|_| CompileFailure::Budget)?;
    let mut error = vec![0_u8; 4096];
    let mut written = 0;
    // SAFETY: all buffers stay alive for the call; name is NUL terminated.
    // The original adapter writes at most capacity bytes and returns the
    // written count. It retains no pointers and never invokes compiled code.
    let status = unsafe {
        hye_lua54_compile(
            document.source.as_ptr(),
            document.source.len(),
            name.as_ptr(),
            error.as_mut_ptr(),
            error.len(),
            &mut written,
        )
    };
    error.truncate(written.min(error.len()));
    match status {
        0 => Ok(()),
        1 => Err(CompileFailure::Syntax(error)),
        _ => Err(CompileFailure::Budget),
    }
}
enum Failure {
    Pending(Box<Request>),
    Unknown(usize, Range<usize>, String),
    Lua(Value),
    Budget(String),
}
type Result<T> = std::result::Result<T, Failure>;
enum Flow {
    Next,
    Return(Vec<Value>),
    Break,
}
#[derive(PartialEq, Eq)]
enum Place {
    Cell(usize),
    Field(usize, Key),
}
struct Engine<'a> {
    program: &'a Program,
    documents: &'a [Parsed<'a>],
    tables: Vec<Table>,
    cells: Vec<Value>,
    closures: Vec<Closure<'a>>,
    globals: usize,
    package: usize,
    registry: usize,
    preload: usize,
    searchers: usize,
    events: Vec<Event>,
    loads: Vec<LoadEvent>,
    order: usize,
    logged: usize,
    visits: usize,
    bytes: usize,
    calls: usize,
    output: usize,
    entries: usize,
}
impl<'a> Engine<'a> {
    fn budget<T>(&self, reason: &str) -> Result<T> {
        Err(Failure::Budget(reason.into()))
    }
    fn unknown<T>(&self, doc: usize, node: Node<'_>, reason: &str) -> Result<T> {
        Err(Failure::Unknown(doc, node.byte_range(), reason.into()))
    }
    fn tick(&mut self) -> Result<()> {
        self.visits += 1;
        if self.visits > 100_000 {
            return self.budget("Static replay exceeds 100,000 syntax/operation visits");
        }
        Ok(())
    }
    fn bytes(&mut self, bytes: &[u8]) -> Result<Value> {
        self.bytes = self.bytes.saturating_add(bytes.len());
        if bytes.len() > 65_536 || self.bytes > 4 * MAX_DOCUMENT_BYTES {
            return self.budget("Static replay string retention exceeds its budget");
        }
        Ok(Value::Bytes(Arc::from(bytes)))
    }
    fn table(&mut self, unknown: bool) -> Result<usize> {
        if self.tables.len() >= 4096 {
            return self.budget("Static replay table limit exceeded");
        }
        let id = self.tables.len();
        self.tables.push(Table {
            values: BTreeMap::new(),
            unknown,
        });
        Ok(id)
    }
    fn cell(&mut self, value: Value) -> Result<usize> {
        if self.cells.len() >= 16_384 {
            return self.budget("Static replay cell limit exceeded");
        }
        let id = self.cells.len();
        self.cells.push(value);
        Ok(id)
    }
    fn name(&mut self, name: &str) -> Result<Arc<str>> {
        self.bytes = self.bytes.saturating_add(name.len());
        if self.bytes > 4 * MAX_DOCUMENT_BYTES {
            return self.budget("Static replay name retention exceeds budget");
        }
        Ok(Arc::from(name))
    }
    fn copied(&mut self, env: &Env) -> Result<Env> {
        self.visits = self.visits.saturating_add(env.locals.len());
        self.tick()?;
        Ok(env.clone())
    }
    fn key(&self, value: Value) -> Result<Key> {
        Ok(match value {
            Value::Bool(v) => Key::Bool(v),
            Value::Int(v) => Key::Int(v),
            Value::Float(v) if v.is_finite() => {
                if v.fract() == 0. && v >= i64::MIN as f64 && v < -(i64::MIN as f64) {
                    Key::Int(v as i64)
                } else {
                    Key::Float(v.to_bits())
                }
            }
            Value::Bytes(v) => Key::Bytes(v),
            Value::Table(v) => Key::Table(v),
            Value::Closure(v) => Key::Closure(v),
            Value::Unknown(reason) => return Err(Failure::Unknown(0, 0..0, reason.into())),
            Value::PartialPath(_) => {
                return Err(Failure::Unknown(
                    0,
                    0..0,
                    "A table key includes an uncaptured package.path tail".into(),
                ))
            }
            Value::Nil => {
                return Err(Failure::Lua(Value::Unknown(
                    "The exact Lua table-index error text is not modeled",
                )))
            }
            _ => {
                return Err(Failure::Unknown(
                    0,
                    0..0,
                    "Table key domain is not modeled".into(),
                ))
            }
        })
    }
    fn get(&self, table: usize, key: &Key) -> Value {
        self.tables[table]
            .values
            .get(key)
            .cloned()
            .unwrap_or(if self.tables[table].unknown {
                Value::Unknown("Unmodeled library field")
            } else {
                Value::Nil
            })
    }
    fn set(&mut self, table: usize, key: Key, value: Value) -> Result<()> {
        self.tick()?;
        if table == self.searchers {
            return Err(Failure::Unknown(
                0,
                0..0,
                "Custom package.searchers changes require a modeled searcher pipeline".into(),
            ));
        }
        if !self.tables[table].values.contains_key(&key) && self.entries >= 16_384 {
            return self.budget("Static replay table-entry limit exceeded");
        }
        if value.is_nil() {
            if self.tables[table].values.remove(&key).is_some() {
                self.entries -= 1;
            }
        } else if self.tables[table].values.insert(key, value).is_none() {
            self.entries += 1;
        }
        Ok(())
    }
    fn field(&self, table: usize, name: &[u8]) -> Value {
        self.get(table, &Key::Bytes(Arc::from(name)))
    }
    fn field_set(&mut self, table: usize, name: &[u8], value: Value) -> Result<()> {
        self.set(table, Key::Bytes(Arc::from(name)), value)
    }
    fn new(program: &'a Program, documents: &'a [Parsed<'a>]) -> Result<Self> {
        let mut e = Self {
            program,
            documents,
            tables: vec![],
            cells: vec![],
            closures: vec![],
            globals: usize::MAX,
            package: usize::MAX,
            registry: usize::MAX,
            preload: usize::MAX,
            searchers: usize::MAX,
            events: vec![],
            loads: vec![],
            order: 0,
            logged: 0,
            visits: 0,
            bytes: 0,
            calls: 0,
            output: 0,
            entries: 0,
        };
        e.globals = e.table(program.profile == Profile::HyprlandLua54)?;
        e.package = e.table(true)?;
        e.registry = e.table(false)?;
        e.preload = e.table(false)?;
        // Prevent the initialization stores from looking like a user searcher mutation.
        let searchers = e.table(true)?;
        for (name, value) in [
            ("_G", Value::Table(e.globals)),
            ("package", Value::Table(e.package)),
            ("require", Value::Native(Native::Require(true))),
            ("__require", Value::Native(Native::Require(false))),
            ("dofile", Value::Native(Native::Dofile)),
            ("loadfile", Value::Native(Native::Loadfile)),
            ("error", Value::Native(Native::Error)),
            ("pcall", Value::Native(Native::Pcall)),
            ("type", Value::Native(Native::Type)),
            ("rawget", Value::Native(Native::Rawget)),
            ("rawset", Value::Native(Native::Rawset)),
            ("select", Value::Native(Native::Select)),
        ] {
            e.field_set(e.globals, name.as_bytes(), value)?;
        }
        for name in [
            "assert",
            "collectgarbage",
            "getmetatable",
            "ipairs",
            "load",
            "next",
            "pairs",
            "print",
            "rawequal",
            "rawlen",
            "setmetatable",
            "tonumber",
            "tostring",
            "warn",
            "xpcall",
        ] {
            e.field_set(
                e.globals,
                name.as_bytes(),
                Value::Native(Native::Unmodeled(name)),
            )?;
        }
        for name in [
            "coroutine",
            "debug",
            "io",
            "math",
            "os",
            "string",
            "table",
            "utf8",
        ] {
            let id = e.table(true)?;
            e.field_set(e.globals, name.as_bytes(), Value::Table(id))?;
            e.field_set(e.registry, name.as_bytes(), Value::Table(id))?;
        }
        e.field_set(e.registry, b"_G", Value::Table(e.globals))?;
        e.field_set(e.registry, b"package", Value::Table(e.package))?;
        e.field_set(e.package, b"loaded", Value::Table(e.registry))?;
        e.field_set(e.package, b"preload", Value::Table(e.preload))?;
        e.field_set(e.package, b"searchers", Value::Table(searchers))?;
        let path = match &program.initial_package_path {
            Some(path) if program.initial_package_path_unknown_tail => {
                let Value::Bytes(bytes) = e.bytes(path)? else {
                    unreachable!()
                };
                Value::PartialPath(bytes)
            }
            Some(path) => e.bytes(path)?,
            None => Value::Unknown("Inherited package.path is not captured"),
        };
        e.field_set(e.package, b"path", path)?;
        let hl = e.table(true)?;
        e.field_set(hl, b"config", Value::Native(Native::Observe("hl.config")))?;
        e.field_set(e.globals, b"hl", Value::Table(hl))?;
        if program.profile == Profile::SyntheticEmit {
            let fixture = e.bytes(b"<root>")?;
            e.field_set(e.globals, b"__fixture_root", fixture)?;
            e.field_set(e.globals, b"emit", Value::Native(Native::Observe("emit")))?;
        }
        let version = e.bytes(b"Lua 5.4")?;
        e.field_set(e.globals, b"_VERSION", version)?;
        e.searchers = searchers;
        Ok(e)
    }
    fn cache(&self, module: &[u8]) -> Result<module_cache::Cache<Value>> {
        // A transition touches only this key. Keep a thin one-key view rather
        // than copying every unrelated cache entry for each require call.
        // Nil is an explicit tombstone; the rest of this view stays unknown.
        let entries = |id: usize| BTreeMap::from([(module.to_vec(), self.field(id, module))]);
        let mut cache =
            module_cache::Cache::new(&self.program.version, entries(self.registry), false)
                .map_err(Failure::Budget)?;
        let loaded = match self.field(self.globals, b"package") {
            Value::Table(package) => self.field(package, b"loaded"),
            Value::Unknown(reason) => return Err(Failure::Unknown(0, 0..0, reason.into())),
            _ => Value::Nil,
        };
        match loaded {
            Value::Table(id) if id == self.registry => {}
            Value::Table(id) => cache
                .rebind_global(entries(id), false)
                .map_err(Failure::Budget)?,
            Value::Unknown(reason) => return Err(Failure::Unknown(0, 0..0, reason.into())),
            _ => cache.global_not_table(),
        }
        Ok(cache)
    }
    fn snapshot(
        &self,
        value: &Value,
        seen: &mut BTreeSet<usize>,
        depth: usize,
        cost: &mut usize,
    ) -> Result<Observed> {
        *cost = cost.saturating_add(96);
        if let Value::Bytes(v) = value {
            *cost = cost.saturating_add(v.len().saturating_mul(4));
        }
        if let Value::Closure(id) = value {
            // Function labels contain a caller-supplied document ID. JSON can
            // expand each control byte to six bytes; charge before formatting.
            let label = self.documents[self.closures[*id].document]
                .document
                .id
                .len()
                + 32;
            *cost = cost.saturating_add(label.saturating_mul(6));
        }
        if *cost > 2 * MAX_DOCUMENT_BYTES {
            return self.budget("Observed replay values exceed output budget");
        }
        if depth > 32 {
            return self.budget("Observed replay value exceeds 32 levels");
        }
        Ok(match value {
            Value::Nil => Observed::Nil,
            Value::Bool(v) => Observed::Bool(*v),
            Value::Int(v) => Observed::Int(*v),
            Value::Float(v) => Observed::Float(*v),
            Value::Bytes(v) => Observed::Bytes(v.to_vec()),
            Value::PartialPath(_) => {
                return Err(Failure::Unknown(
                    0,
                    0..0,
                    "The inherited package.path tail is not captured".into(),
                ))
            }
            Value::Table(id) => {
                if self.tables[*id].unknown {
                    return Err(Failure::Unknown(
                        0,
                        0..0,
                        "Unmodeled library table contents".into(),
                    ));
                }
                if !seen.insert(*id) {
                    return Ok(Observed::Cycle(*id));
                }
                let mut rows = vec![];
                for (k, v) in &self.tables[*id].values {
                    let key = match k {
                        Key::Bool(v) => Value::Bool(*v),
                        Key::Int(v) => Value::Int(*v),
                        Key::Float(v) => Value::Float(f64::from_bits(*v)),
                        Key::Bytes(v) => Value::Bytes(v.clone()),
                        Key::Table(v) => Value::Table(*v),
                        Key::Closure(v) => Value::Closure(*v),
                    };
                    rows.push((
                        self.snapshot(&key, seen, depth + 1, cost)?,
                        self.snapshot(v, seen, depth + 1, cost)?,
                    ));
                }
                seen.remove(id);
                Observed::Table(rows)
            }
            Value::Closure(id) => Observed::Function(format!(
                "{}:closure:{id}",
                self.documents[self.closures[*id].document].document.id
            )),
            Value::Native(n) => Observed::Function(format!("{n:?}")),
            Value::Unknown(reason) => return Err(Failure::Unknown(0, 0..0, (*reason).into())),
        })
    }
    fn observed(&mut self, values: &[Value]) -> Result<Vec<Observed>> {
        let mut cost = self.output;
        let out = values
            .iter()
            .map(|v| self.snapshot(v, &mut BTreeSet::new(), 0, &mut cost))
            .collect::<Result<Vec<_>>>()?;
        self.output = cost;
        Ok(out)
    }
}

pub fn replay(program: &Program) -> std::result::Result<Report, String> {
    if ![BASELINE, AUDITED_MAIN].contains(&program.version.as_str())
        || program.documents.is_empty()
        || program.documents.len() > 32
        || program.bindings.len() > 512
        || (program.initial_package_path_unknown_tail
            && program
                .initial_package_path
                .as_ref()
                .is_none_or(|p| p.is_empty()))
        || program
            .initial_package_path
            .as_ref()
            .is_some_and(|p| p.len() > 65_536 || p.contains(&0))
        || program
            .working_directory
            .as_ref()
            .is_some_and(|p| p.len() > 4096 || p.contains(&0))
    {
        return Err("Invalid static replay bundle limits or snapshot".into());
    }
    let mut ids = BTreeSet::new();
    let mut bytes = 0_usize;
    for doc in &program.documents {
        bytes = bytes.saturating_add(doc.source.len());
        if doc.id.is_empty()
            || doc.id.len() > 64
            || !ids.insert(doc.id.as_str())
            || doc.source.len() > MAX_DOCUMENT_BYTES
            || doc.source.as_bytes().contains(&0)
            || doc.sha256 != source_hash(&doc.source)
            || doc.load_name.len() > 4096
            || doc.load_name.contains(&0)
        {
            return Err("Invalid replay document identity or bytes".into());
        }
    }
    if bytes > 4 * MAX_DOCUMENT_BYTES {
        return Err("Replay source bundle exceeds 4 MiB".into());
    }
    let root = program
        .documents
        .iter()
        .position(|d| d.id == program.root)
        .ok_or("Missing replay root")?;
    for (i, b) in program.bindings.iter().enumerate() {
        let d = program
            .documents
            .iter()
            .find(|d| d.id == b.request.document)
            .ok_or("Unknown resolution source")?;
        let span = &b.request.span;
        if b.request.sha256 != d.sha256
            || span.start >= span.end
            || span.end > d.source.len()
            || !d.source.is_char_boundary(span.start)
            || !d.source.is_char_boundary(span.end)
            || b.request.module.len() > 16_384
            || b.request.module.contains(&0)
            || (b.request.package_path_unknown_tail
                && b.request.package_path.as_ref().is_none_or(|p| p.is_empty()))
            || (matches!(b.request.loader, Loader::Dofile | Loader::Loadfile)
                && (b.request.package_path.is_some() || b.request.package_path_unknown_tail))
            || b.request
                .package_path
                .as_ref()
                .is_some_and(|p| p.len() > 65_536 || p.contains(&0))
            || b.request
                .working_directory
                .as_ref()
                .is_some_and(|p| p.len() > 4096 || p.contains(&0))
            || program.bindings[..i].iter().any(|p| p.request == b.request)
        {
            return Err("Invalid or duplicate replay resolution binding".into());
        }
        if let Resolution::Loaded {
            document,
            loader_data,
        } = &b.resolution
        {
            if !ids.contains(document.as_str())
                || loader_data.len() > 4096
                || loader_data.contains(&0)
            {
                return Err("Invalid replay module target".into());
            }
        }
    }
    let documents = program
        .documents
        .iter()
        .map(|document| {
            let compile = compile(document);
            let tree = if compile.is_ok() {
                syntax_tree(&document.source)
            } else {
                Err("Document did not compile".into())
            };
            Parsed {
                document,
                tree,
                compile,
            }
        })
        .collect::<Vec<_>>();
    let mut engine =
        Engine::new(program, &documents).map_err(|_| "Cannot initialize bounded replay model")?;
    let result = engine
        .chunk(root, engine.globals, vec![])
        .and_then(|values| engine.observed(&values));
    let (returns, halt) = match result {
        Ok(values) => (values, None),
        Err(failure) => {
            let halt = match failure {
                Failure::Pending(request) => Halt::NeedsResolution { request: *request },
                Failure::Unknown(doc, span, reason) => Halt::Unresolved {
                    document: program.documents[doc].id.clone(),
                    span,
                    reason,
                },
                Failure::Lua(value) => match engine.observed(&[value]) {
                    Ok(mut values) => Halt::LuaError {
                        value: values.remove(0),
                    },
                    Err(Failure::Unknown(doc, span, reason)) => Halt::Unresolved {
                        document: program.documents[doc].id.clone(),
                        span,
                        reason,
                    },
                    Err(Failure::Budget(reason)) => Halt::Budget { reason },
                    Err(_) => Halt::Unresolved {
                        document: program.root.clone(),
                        span: 0..0,
                        reason: "Error object cannot be observed by this model".into(),
                    },
                },
                Failure::Budget(reason) => Halt::Budget { reason },
            };
            (vec![], Some(halt))
        }
    };
    Ok(Report {
        compiler_profile: compiler_profile(),
        version: program.version.clone(),
        profile: program.profile.clone(),
        events: engine.events,
        loads: engine.loads,
        returns,
        halt,
        logged_module_errors: engine.logged,
        runtime_verified: false,
    })
}
