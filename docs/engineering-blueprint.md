# Hyprland editor engineering blueprint

Design revision 1 — 13 October 2026, Asia/Kolkata.

This document specifies how to implement the audited editor. Decisions below are proposed implementation requirements, not claims that code has been built or security properties have been demonstrated. Interface sketches are design notation, not compiled Rust.

Read this with the [feature implementation contracts](hyprland-implementation-contracts.md), [machine-readable contracts](hyprland-implementation-contracts.json), and [helper request schema](hyprland-helper-request.schema.json). The previous [audit inventory](hyprland-feature-inventory.json) remains the evidence catalogue.

The companion manifest assigns 1,453 contract rows, including 403 distinct scalar keys across the two snapshots, 150 structured descriptor fields, 100 public Lua functions and 39 runtime object methods. Rows also cover flags, events, inspectors, startup capabilities and component documentation candidates. These are engineering assignments, not 1,453 independent settings or a claimed completion percentage.

The implementation contract covers the union of the audited 0.56.2 release and the pinned current-git snapshot, with explicit version membership. The snapshots are efb50993780079460b0cbed1363e2166a2de1d9f and 6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7. “Current” here means that audited commit, not all future commits. An installed build is writable only after its schema and behavior suite are certified; version strings alone do not establish compatibility with patched builds.

## 1. Corrections to the earlier architecture

**Save is not inherently consequence-free.** Hyprland watches configuration files; writing an active file can trigger reload. The source [config watcher](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/shared/inotify/ConfigWatcher.cpp) changes its watch list according to misc:disable_autoreload and also watches wildcard directories and symlink targets. Therefore Save draft writes only editor-owned draft storage. Publish to active configuration is always an apply operation. Pending restart changes remain drafts until a reviewed installation at the appropriate lifecycle point.

**A generated settings catalogue is not a production schema.** The audit retained C++ expressions and heuristic signatures. Production schemas must resolve defaults, required arguments, units, aliases, enum maps, special values, activation behavior and source encoding. Unresolved entries are visible but blocked from apply.

**The GUI must not host all untrusted parsers.** Tree-sitter, regex compatibility and native asset decoding operate in limited workers. Only validated normalized results cross into the GUI. An imported config cannot acquire a writer or live-compositor capability through a parser response.

**A file transaction and a compositor transaction are different.** The filesystem offers atomic replacement per file; Hyprland does not offer a general all-feature atomic transaction. Publishing multiple files, reloading Lua and restoring runtime state need separate protocols and explicit partial-failure states.

## 2. Firm implementation choices

| Decision | Selected design | Reason and acceptance condition |
|---|---|---|
| Application | Rust native application, GTK4/libadwaita | One UI model, native accessibility, no embedded browser; total-process memory gate |
| UI baseline | GTK 4.10 APIs; newer APIs behind capability checks | Avoid unnecessary dependence on newest GTK; distro build matrix determines supported packages |
| Event loop | GLib main loop, bounded worker threads/processes | No separate permanent Tokio runtime; GTK objects remain on the UI thread |
| Lua syntax | Tree-sitter Rust bindings plus pinned tree-sitter-lua grammar | Concrete byte spans and incremental parsing; no Lua execution on import |
| Legacy syntax | Dedicated lossless Rust lexer/parser for Hyprlang config | Comments, variables, blocks and ordered directives retained; upstream differential corpus |
| Serialization | serde/serde_json, typed AST printers, ryu-compatible finite float output | No source templates interpolating raw imported text |
| OS operations | rustix, isolated audited FFI where missing | Owned descriptors, explicit Linux operations, no shell command construction |
| Regex compatibility | RE2 through a narrow C ABI worker | Same engine family as Hyprland; Rust regex is not substituted as a semantic oracle |
| File identity | SHA-256 of raw bytes plus device/inode and metadata snapshots | Hash content; do not identify files by modification time alone |
| Preview isolation | bubblewrap namespace construction, seccomp policy, resource supervisor | Required isolation properties are tested; bubblewrap alone is not a security policy |
| Preview transport | Private Wayland capture/input broker; bounded raw-RGBA frame transport | No host display connection inside the lab; no untrusted image decoder in the frame receiver |
| Privileged operations | Excluded from normal editor | System/PAM/driver changes export a plan; any future helper gets its own narrow review |
| Storage | Versioned JSON documents and append-only transaction journals | No database server; no resident daemon; atomic checkpoints |
| Update mechanism | Distro packages with embedded schemas initially | No automatic code/schema downloader; unknown unsigned schema bundles cannot enable writes |
| Extensions | Declarative adapters compiled/packaged with the application | No arbitrary executable GUI plugin loader |

Exact dependency artifacts and grammar commits are pinned in Cargo.lock, a Rust toolchain file and a dependency manifest when the implementation repository is initialized. This blueprint deliberately does not invent an untested compatible dependency-version tuple. Packaging CI must build that tuple before release; release builds never resolve “latest” dynamically.

GTK threading and API availability are documented by [gtk4-rs](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/). [Gtk.ListView](https://docs.gtk.org/gtk4/class.ListView.html) creates widgets for visible items, which is the selected implementation for large setting and rule lists. [Tree-sitter](https://tree-sitter.github.io/tree-sitter/using-parsers/1-getting-started.html) and the [Lua grammar](https://github.com/tree-sitter-grammars/tree-sitter-lua) provide syntax parsing, not evaluation.

## 3. Modules, dependency direction and authority

Use one Cargo workspace. Crate names below are implementation identifiers, not separate background services.

| Module | Owns | Must not do |
|---|---|---|
| editor-types | IDs, typed values, provenance, errors, generation numbers | File/network/process operations |
| editor-schema | Release packs, capabilities, typed form/action specifications | Execute source or download updates |
| editor-source | Lossless syntax, dependency graph, patch plans | Write files or call live IPC |
| editor-model | Saved/active/effective/draft state, reducers, undo | Execute callbacks or hold arbitrary source execution authority |
| editor-codec | Lua/legacy/component printers, migration mappings | Emit unvalidated strings as source fragments |
| editor-discovery | Instance identity, safe query catalogue, file ownership | Arbitrary eval, dispatch, reload or service mutation |
| editor-validation | Static checks, worker validation requests, reports | Treat missing dependencies as successful validation |
| editor-apply | Effect classification, prepared transactions, targeted mutations | Let UI widgets bypass preparation |
| editor-storage | Authorized paths, drafts, backups, journal/checkpoints | Expand user-controlled paths after authorization |
| editor-preview | Projection compiler, supervisor, lab state | Receive a live-compositor mutation capability |
| editor-adapters | Independently versioned tool contracts | Global restart/kill by process name |
| editor-ui | GTK views, recording, review, accessibility | Direct filesystem writes or raw Hyprland commands |
| parse-worker / asset-worker | Static parsing, RE2 checks, bounded decoding | Host home/runtime/network access |
| lab-worker / lab-broker | Private compositor and synthetic clients | Host Wayland/session-bus/input/DRM-card access |
| apply-agent / recovery-agent | Reviewed writes and bounded recovery | Interpret arbitrary Lua or accept general shell commands |

Pure modules depend only toward types/schema. Discovery and apply use separate transport wrappers, even if their low-level socket implementation is shared. A ReadSession has no mutate method. A MutationSession is minted by transaction preparation and pinned to the selected identity. A LabSession cannot be converted into either.

~~~mermaid
flowchart TB
    UI[GTK UI] --> Model[Pure draft reducer]
    Model --> Prepare[Review and transaction preparation]
    Prepare --> Apply[Apply agent and recovery agent]
    Discover[Read-only discovery] --> Model
    Parse[Limited parse and asset workers] --> Model
    Model --> Projection[Trusted preview projection]
    Projection --> Supervisor[Preview supervisor]
    Supervisor --> Lab[Private compositor and broker]
    Lab --> Frames[Bounded frame receiver]
    Frames --> UI
~~~

“Read-only” refers to our command whitelist, not a claim that arbitrary same-user IPC clients are powerless. A compromised user account or deliberately malicious already-running compositor is outside the protection promised by this editor. Parsers, imported files and lab code are treated as hostile inputs within their boundaries.

## 4. Shared data model and invariants

All runtime IDs carry a session generation. Window addresses are opaque runtime handles, never portable profile identifiers. Paths are byte-safe owned PathBuf values internally; helper JSON uses an opaque authorized document ID, not arbitrary absolute paths. New user text is UTF-8, NUL-free and bounded before GTK or native APIs receive it. Invalid imported bytes are preserved with an explicit edit limitation.

~~~rust
// Design notation; not a compiled implementation.
struct SessionIdentity {
    uid: u32,
    signature: String,
    pid: u32,
    process_start_ticks: u64,
    boot_id: String,
    schema_revision: String,
    generation: u64,
}

enum Slot<T> {
    Unset,
    Explicit(T),
    Derived { expression: ExprId, certainty: Certainty },
    Unknown { reason: UnknownReason },
}

enum Value {
    Bool(bool), Int(i64), Float(f64), Text(String),
    Color(ColorValue), Vec2([f64; 2]), Gap(Sides),
    Gradient(GradientValue), FontWeight(FontWeightValue),
    Selector(SelectorExpr), Placement(PlacementExpr),
}

struct Observation<T> {
    value: T,
    session_generation: u64,
    sampled_start_ns: u64,
    sampled_end_ns: u64,
    certainty: Certainty,
}

struct FieldState {
    feature: FeatureId,
    saved: Slot<Value>,
    active: Option<Observation<Value>>,
    draft: Slot<Value>,
    provenance: Vec<DeclarationRef>,
    capability: CapabilityStatus,
}

struct Edit {
    base_document_hash: Hash,
    span: ByteRange,
    expected_old_bytes_hash: Hash,
    replacement: Vec<u8>,
    semantic_target: FeatureId,
}
~~~

Model invariants:

1. Unset, false, zero, empty and unknown remain distinct.
2. Finite floating-point values only; no NaN/infinity crossing serialization.
3. Source observations and active values are not automatically copied into the draft.
4. A declared literal is a syntactic saved declaration; opaque earlier code can invalidate certainty about its effective result.
5. Schema revision, document generation and session generation accompany every asynchronous request/result.
6. Older results are discarded, never rebound to a new session or modified draft.
7. A feature outside the selected pack's version membership is never serialized for live use.
8. UI descriptions, paths and IPC values are escaped display data.
9. Runtime handles are re-resolved immediately before an action; disappearing targets produce a typed error.
10. Opaque or unresolved fields cannot pass a “fully validated” gate by default.

Undo stores inverse reducer operations and affected source spans, not copies of the whole graph. Checkpoint every 100 operations or 30 seconds with pending edits, cap in-memory history at 1,000 operations/8 MiB, and retain older history on disk within the user's configured quota. A failed checkpoint leaves the existing draft intact and displays an error.

## 5. Internal service interfaces

~~~rust
trait SourceService {
    fn parse(&self, input: SealedBytes, pack: PackId) -> ParseReport;
    fn extract(&self, parsed: ParsedId) -> DeclarationGraph;
    fn plan_edit(&self, graph: &DeclarationGraph, op: &DraftOp) -> EditPlan;
    fn validate_patch(&self, original: SealedBytes, plan: &EditPlan) -> PatchReport;
}
trait DiscoveryService {
    fn enumerate(&self) -> Vec<InstanceCandidate>;
    fn attach_readonly(&self, chosen: InstanceCandidate) -> Result<ReadSession>;
    fn snapshot(&self, session: &ReadSession, scope: QueryScope) -> Snapshot;
}
trait ValidationService {
    fn check_static(&self, draft: &Draft, pack: &Pack) -> ValidationReport;
    fn check_isolated(&self, request: ValidationJob) -> ValidationReport;
}
trait ApplyService {
    fn prepare(&self, reviewed: ReviewedDraft, target: TargetContext) -> PreparedPlan;
    fn commit(&self, id: PreparedPlanId, digest: Hash) -> ApplyProgress;
    fn recover(&self, id: TransactionId) -> RecoveryReport;
}
trait PreviewService {
    fn project(&self, draft: &Draft, scenario: Scenario) -> ProjectionReport;
    fn start(&self, projection: ApprovedProjection) -> LabHandle;
    fn stop(&self, handle: LabHandle) -> CleanupReport;
}
trait ComponentAdapter {
    fn discover(&self, scope: ReadCapabilities) -> ComponentState;
    fn parse(&self, bytes: SealedBytes, version: ComponentVersion) -> ParseReport;
    fn serialize(&self, draft: ComponentDraft) -> ComponentPatch;
    fn activation(&self, patch: &ComponentPatch) -> EffectPlan;
    fn preview(&self, draft: &ComponentDraft) -> PreviewProjection;
    fn verify(&self, target: PinnedComponentTarget) -> VerificationReport;
}
~~~

These signatures encode authority boundaries. Large source/asset bytes use sealed descriptors, not huge JSON messages. No function takes a user-created shell command as an implementation mechanism. Unknown dependency, incompatible build, unsupported syntax, conflict, stale generation, cancelled, quota exceeded, uncertain observation and partial apply are distinct error variants.

Every report includes requested scope, exact pack/component versions, checks actually performed, skipped/unknown checks, findings, affected features and source spans. Empty findings with skipped checks is not a successful conformance certificate.

## 6. Version packs and extraction

A pack contains source commits/hashes; feature membership; resolved typed defaults; validation bounds/maps; special-value rules; required/optional argument variants; serializer IDs; activation policies; getter definitions; preview contracts; deprecations/migrations; and test evidence.

Build-time generation first exports the scalar registry through an isolated release-specific exporter. Its output is compared with source registration names and runtime descriptions from a synthetic test instance. C++ expressions in the audit are never evaluated in the installed GUI. A release cannot carry unresolved symbolic defaults as ready-to-apply data.

Structured descriptors supply field names/types/defaults. Handwritten semantic specifications cover required wrappers, overloaded action variants, exclusivity, ordering, source-only conditions, special syntax, selectors and expressions. A narrow C++ extraction tool can aid discovery; its inferred argument list is not proof of acceptance.

Feature conformance tests construct valid/invalid boundary examples and feed generated output into the corresponding isolated parser. Save accepted inputs, rejection categories, normalized results, source references and test binary hashes. Generated schema changes are reviewable diffs.

Do not select a schema merely because a version number has the same major/minor. Allow certification records for exact upstream commit plus known distro patches/build features. Unknown binaries receive discovery/preservation/offline export but no guessed mutation codec. Upstream security-support status is displayed separately from compatibility.

Production packs resolve:

- Lua dotted keys versus legacy colon keys and any version aliases.
- Scalar versus rule/device field meanings; a device does not accept every input option.
- Field defaults versus absent effect semantics; rule constructor defaults do not justify emitting unused effects.
- Spring dampening in 0.56.2 versus damping plus compatibility alias in the audited current snapshot.
- Strict spring bounds, animation required fields and actual curve/style syntax.
- Legacy .conf support in 0.56.2 and Lua-only current source.
- Current-only 50 added scalars, three added window effects, two added functions and three added Lua events.

## 7. Lossless parsing and source editing

### Lua

The parser worker receives immutable file bytes plus a grammar revision. Tree-sitter supplies concrete ranges. Retain raw bytes and token trivia in shared buffers; represent semantic nodes by indices rather than copying strings into every node.

Recognize calls to verified hl bindings, tables, literals, local variables and approved pure expressions. Track lexical scopes: a local variable named hl, reassignment of hl.config, metatables, dynamic indexing or unknown calls must not be mistaken for the genuine API.

The static expression evaluator is our own restricted evaluator over AST nodes. It supports finite arithmetic, boolean operations, literal string concatenation, literal tables and acyclic local references. It never invokes Lua, user functions, require, os, io, debug or native libraries. Unsupported operations produce Derived/Unknown instead of executing.

Track ordered declarations and possible writes. Branches produce conditional provenance; unknown loops/functions produce unknown effective results. Even if a later literal can be edited safely by span, its live effect can remain uncertain because other code may overwrite it.

Explicit require paths use the release's resolution rules; named modules have a separately recorded search path. Wildcards preserve sorted expansion order and watched parent directories. Cycles, missing files, computed paths and package.path modifications are reported. A file outside the authorized source graph is not fetched automatically.

Editing one literal replaces only its expression span. Reparse the changed file, ensure no new syntax errors, check intended semantic changes and compare all unaffected declarations. Deleting a field removes only its owned table entry and necessary punctuation, preserving adjacent comments or attaching them to a documented surviving position.

### Legacy Hyprlang

Implement a byte-preserving tokenizer for statements, comments, variables, category blocks, special blocks, continuations and quoted/escaped values according to the pinned parser. It yields ordered statement nodes with raw value spans. Do not treat commas or braces inside every value as generic separators.

Each directive codec parses its own grammar: binding flags, monitor fields, rule match/effect syntax, environment modifiers, animation/Bezier and include/source. A context stack records categories, active submap and file origin. Relative includes, variable expansion and special-block inheritance follow upstream fixtures.

Parsing legacy source for migration remains available even when the selected target accepts only Lua. Migration emits a separate managed configuration and a semantic-change report, never silently renames the original file into the active higher-priority format.

### Ownership and patch authorization

Classify files as user-owned editable, symlink-managed, generated, immutable/store-owned, system-owned, unknown or editor-managed. Read symlink chains with cycle/depth checks and record both link and target identities. No target is authorized merely because the link sits inside the configuration directory.

An EditPlan contains sorted non-overlapping byte ranges, expected source hashes and semantic targets. Apply-agent rejects unauthorized documents, overlapping edits, stale hashes and cross-document IDs. The parser worker cannot authorize paths. New source files are created only in an approved generation/root plan.

## 8. Managed configuration and reload lifecycle

Managed mode uses an immutable generation directory containing the reviewed model, generated Lua/legacy files and asset references. A small explicit root entry references one generation. No wildcard include is introduced.

Generation publishing is a single-root switch only when all generated dependencies are staged outside the active watched graph and semantics are proven portable. Keep the prior generation until readback and retention checks establish it is no longer required. A broad user wildcard can invalidate this strategy; do not assume directory staging is invisible.

Imported unowned configurations use source patches and the multi-file watcher-gate protocol in section 15. Users can choose managed conversion, but it is an explicit ownership change with comparison, not an automatic “repair.”

Managed node IDs are stable UUIDs in the editor manifest; emitted rule/bind/workflow names use a deterministic reserved prefix plus ID. Avoid relying on arbitrary runtime pointers. Removing a rule uses a verified replacement/disable/full-reload mechanism, not an invented remove method.

On full Lua reload, rebuild owned objects in the version-supported lifecycle. On incremental owned workflow replacement, first disable prior owned binds/timers/subscriptions through supported handles, then create the new generation, retaining cleanup handles. Anonymous imported rules and callbacks are not incrementally replaced by guesswork.

Use config.unload only in packs that actually support it. 0.56.2 gets a separately tested lifecycle strategy. require caches and returning module handles need fixtures for ordinary reload, partial failure and managed replacement. Ten reloads must leave one intended bind, subscription and timer per node and must not repeatedly launch startup applications.

The application can display all public Hyprland API behavior through dedicated editors or visual nodes. Arbitrary standard-library Lua, custom native plugin logic and imported unknown modules are additional programming surfaces. They remain explicit unknown/untrusted nodes until an adapter or advanced visual-language implementation represents them; preserving source is not counted as a graphical implementation.

## 9. Typed values, expressions and serialization

### Common rules

Serialization consumes typed values and semantic AST nodes. Lua source strings use a single audited quoted-string printer: escape backslash/quote/control characters; disambiguate numeric escapes; reject NUL where the target cannot represent it; enforce valid lengths. Never build source by inserting imported expressions into a format string.

Use decimal integers within the target's range, finite shortest-roundtrip decimal floats with a dot regardless of locale, and deterministic field order. JSON helper representations of 64-bit identities are decimal strings to avoid accidental precision loss in non-Rust tooling.

For an unchanged imported value, retain the original source representation. For an edited value, emit a version-certified canonical encoding. Preserve comments and field absence; no printer materializes all defaults.

| Codec | Internal representation | Graphical control | Essential tests |
|---|---|---|---|
| C-BOOL | Bool plus Unset slot | Switch with inherit/reset | false versus unset; all accepted imported spellings |
| C-INT | i64 plus enum/special-value metadata | Integer entry or mapped picker | endpoints, sentinels, overflow, changed enum maps |
| C-FLOAT | finite f64 plus units/bounds | Decimal entry/slider | exact bounds, tiny/large values, locale, finite rejection |
| C-STRING | UTF-8 data or specialized grammar ID | Label/name entry or dedicated builder | quoting, controls, delimiter injection, preserved unknown values |
| C-COLOR | normalized channel values plus original lexeme/color notation | RGBA picker | RGBA versus C++ packed-color ordering, alpha and precision |
| C-VEC2 | Two finite components and units | Paired controls | exact accepted table/string form, missing/extra components |
| C-GAP | Four independent sides and optional scalar shorthand | Linked/unlinked side editor | inheritance, scalar equivalence, asymmetric roundtrip |
| C-GRADIENT | Ordered colors and angle in the supported grammar | Stops/angle editor | order, singleton, angle units, alpha, limits |
| C-FONTWEIGHT | Named weight or numeric value | Name/weight picker | constructor default 400, accepted names and numbers |
| C-PLACEMENT | Restricted two-axis expression AST | Formula blocks with coordinate preview | allowed variables/operators, precedence, unsupported imported grammar |
| C-SELECTOR | Versioned selector AST, target type and optional negation | Target/filter builder | relative/absolute identity, regex escaping, stale targets |

The exact accepted source shapes come from pinned type-parser evidence and the codec conformance report. Complex type examples cannot be copied from a getter's returned table and assumed accepted by its setter.

Placement expressions use a version-defined grammar for available geometry variables and operators. The visual editor builds nodes such as number, allowed variable, add/subtract/multiply/divide, grouping and allowed functions. If the version accepts more constructs, its grammar/editor must expose them before the feature is certified. Division-by-zero and unresolved geometry produce visible unknown preview results.

Regex match builders construct literal/exact/negated/pattern nodes, use a restricted RE2 worker to compile and match, and never silently switch engines. Show matching sample records and error positions. A raw regex field is an advanced input option, not the only graphical route to matching.

Shader support has an asset picker plus a node/material graph for supported generated effects. Unknown imported GLSL remains an opaque code asset and does not count as fully graphical shader authoring. Compiling arbitrary shaders is a separately trusted lab action with driver/time limits; shader authoring cannot be made safe merely by treating it as a filename.

### Specialized string controls

The feature contracts give exact overrides for layout selection, locale, XKB model/layout/variant/options/rules/file, acceleration profiles/custom curves, scroll points/methods, tap maps, monitor selection, fonts, swallow regexes, SDR transfer function, master placement/orientation, scrolling widths/direction, shader assets and sound assets. A String type does not justify a generic textbox for these grammars.

## 10. Discovery, IPC and asynchronous behavior

Enumerate candidates through the supported instance discovery mechanism and validate filesystem/process ownership. Pin signature, PID/start ticks, UID, boot ID, socket identity and schema. Prefer pidfds to monitor the chosen process; [pidfd_open](https://man7.org/linux/man-pages/man2/pidfd_open.2.html) avoids identifying process lifetime by a recycled PID alone.

Connect only the chosen Unix socket. Check peer credentials where supported and the expected compositor identity; close each request immediately after its bounded response. [Unix socket credentials](https://man7.org/linux/man-pages/man7/unix.7.html) identify peers but do not protect against a fully compromised same-user account.

Read operations are explicitly whitelisted: metadata/descriptions, version, selected monitors/devices/windows/workspaces/layers/binds/animations/status/configerrors and necessary inspection getters. Plugin list is read-only; plugin load/unload is not. kill, eval, repl, reload, output creation, notify and dispatcher requests never enter the read wrapper.

One event connection is maintained while the UI needs live data. Initial snapshots are sampled intervals rather than advertised atomic state. Subscribe before snapshotting, buffer/coalesce events, then reconcile affected scopes. Events and repeated fullscreen notifications are not state transitions to invert blindly. Titles may contain delimiters; bounded parsing is version-specific and errors trigger a resnapshot.

At most one ordinary query is in flight per instance, with a proposed one-second client deadline and immediate close on cancellation. Metadata limits are separate from event-line limits. Query-size caps reject with a visible “response too large” result; they do not truncate state and call it complete.

Refresh on events, view focus, known apply actions or explicit refresh. Scalar values changed by arbitrary callbacks may lack an event; display sampled/stale status and refresh when a relevant view is opened. No promise of perfect omniscient real-time detection.

Controlled generated Lua evaluation is permitted only in MutationSession when a certified release requires it for typed setters/actions. Its source is printed from a closed AST using schema-known calls and values. Raw source fragments, arbitrary function names, imported callbacks and UI-supplied eval strings are excluded. A reader never uses eval merely to fetch hidden state.

All background jobs carry draft/document/session generations. Editing cancels obsolete validation after a 200 ms debounce; a late result is ignored. GTK receives only normalized model changes through the main loop. Search reads an in-memory index of keys, aliases and descriptions; it does not query Hyprland on each keystroke.

## 11. Concrete scalar codecs checked against source

The pinned parser files were downloaded as inert audit inputs for this blueprint. Their hashes and extracted argument facts are retained in the contracts; temporary source files are removed at the end. Static inspection establishes accepted syntax, not successful runtime execution.

Use these canonical Lua forms for the inspected complex types:

~~~lua
hl.config({
    ["general.border_size"] = 2,
    ["general.gaps_in"] = { top = 4, right = 6, bottom = 4, left = 6 },
    ["general.col.active_border"] = {
        colors = { "0xFFFF0000", "0xFF0000FF" },
        angle = 45,
    },
})
-- A Vec2 setter takes a two-element array: { x_value, y_value }.
-- A placement expression takes two string/number elements, or its supported string form.
~~~

The inspected [Vec2 setter](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/types/LuaConfigVec2.cpp) accepts an array or exactly two numbers in a string. Although its getter also supplies x/y named fields, a named-only x/y table is not the same setter form. Always emit the positional pair.

The [gap setter](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/types/LuaConfigCssGap.cpp) accepts a scalar or side table. Missing sides in an explicit table default to zero in this parser, not inherited opposite sides. The GUI expands a partially imported table according to those semantics, preserves its source until edited, and emits all four sides after an edit.

The [gradient setter](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/types/LuaConfigGradient.cpp) accepts one color string or a table with a nonempty colors array and optional numeric angle in degrees. A legacy multi-color gradient string cannot simply be quoted and used as the Lua gradient value.

The inspected [color parser](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/shared/parserUtils/ParserUtils.cpp) distinguishes packed hexadecimal color from rgba notation. Canonical emitted color strings use 0xAARRGGBB; the UI stores/displays RGBA channels and performs an explicit conversion. Font weights emit numeric values unless retaining an unchanged recognized name; quoted numeric weights are rejected by the inspected Lua parser.

ExpressionVec2 emits a positional pair of strings/numbers built from the approved expression grammar. It is not a Vec2 with arbitrary Lua expressions pasted into a table. Compound-rule values such as opacity/fullscreen-state/group options use separate semantic codecs, not a generic String printer.

Legacy codecs have separate printers: categories/colon keys, gap ordering, gradients with angle units, ordered directive argument separators and variables. The production corpus must prove equivalent accepted values and reveal rounding/coercion differences. Existing parser coercions are preserved on import; new GUI numeric input follows the declared integer/finite-float type.

## 12. Structured editors and their model contracts

### Monitor editor: M1

MonitorSpec contains output selector, optional mode/position/scale, transform, mirror target, disable state, reserved sides, bit depth, color-management/transfer-function settings, VRR, ICC reference, SDR brightness/saturation and advertised/luminance overrides. Each field is a Slot so absence remains meaningful. All 22 descriptor fields plus wrapper fields are individually mapped.

Mode is a tagged union: preferred, highest-refresh, highest-resolution, explicit resolution/rate or supported custom modeline. Position is explicit logical coordinates or a supported automatic relation. The canvas keeps logical and physical rectangles separately and applies transform before displaying scale-dependent bounds.

Validation checks output-selector grammar, finite positive scale, mode timing/ranges, mirror references/cycles, disabled-last-output risks, overlapping/reserved geometry and color/HDR interactions. Unsupported EDID capabilities are findings, not silent modifications. Missing outputs keep their rule in the draft.

The lab maps connector identities to private named outputs and reports substitutions. It renders geometry and visuals. KMS modes, VRR, HDR scanout, color accuracy and refresh behavior require a guarded physical test. The topology model includes a recovery output/input plan before allowing that test.

### Device editor: D1

DeviceSpec contains the exact 43 per-device descriptors plus name. A capability record identifies relevant keyboard, pointer, touchpad, tablet/touch or virtual-device fields. Irrelevant fields remain visible in an advanced view with a reason, rather than disappearing from the catalogue.

Global input defaults and device overrides have separate provenance. XKB forms use linked layout/variant lists and option groups; raw keymap files are asset references with validation. Curve/scroll-point editors use ordered numeric points. Region editors store finite coordinates, units and output identity and expose relative/absolute modes distinctly.

Changing tags recalculates all referencing device-specific bind filters in the draft. Conflicting bindings use device-set intersection, not just matching displayed shortcut strings. Disabling a last usable keyboard/pointer is guarded.

The deeper source check also found 13 current HL.Device properties delegated to keyboard/pointer/tablet-pad helpers that the generated stub inventory missed: default_speed, scroll_factor, rules, model, layout, variant, options, main, active_layout_index, active_keymap, caps_lock, num_lock and belongs_to. The contracts include these conditional read-only outputs. A field may be nil for another device type or an expired handle; it is not automatically a writable setting.

Keyboard preview uses a local keymap compiler in a worker with explicit inputs. It does not change the real layout. Synthetic devices verify private compositor routing where supported; physical touch/tablet/gesture interpretation remains outside that guarantee.

### Workspace and layouts: W1 / L1

WorkspaceSpec contains selector, enablement, all 15 descriptor fields and layout_opts. Selection identity is separate from display name. Filters for existing workspaces cannot be treated as declarations of nonexistent workspaces.

LayoutSpec holds built-in layout selection, that layout's scalar controls, per-workspace options and typed messages. Every documented/source-supported message receives an action form, argument grammar, synthetic scenario and accepted-version record. A layout message is an action, not a persistent setting by itself.

A custom layout graph uses context area, target iteration, grid/row/column/split operations and gap-aware placement. Validate finite boxes, target lifetime, zero-target behavior and layout-provider callback budgets. Manual set_box is separately identified because it bypasses automatic placement adjustments.

### Rule engine: R1

RuleSpec has stable editor ID, source identity, optional public name, enabled state, typed match predicates and ordered typed effects. Include all 60 union window effects, all layer effects and the baseline public match set. Alias mappings and added effects are version-gated.

Match groups follow the actual version semantics; the UI must not offer unsupported OR grouping as if Hyprland accepts it. Where OR requires duplicate rules, emit explicit duplicates with tracked IDs and disclose the resulting order.

The match inspector uses metadata from selected windows/layers; contents are not captured. Implement the source algorithm for named/anonymous ordering, static/dynamic application and effect precedence through differential synthetic fixtures. Show declared candidates and observed outcomes separately.

Placement, size limits, opacity modes, group actions, suppress-event choices, idle-inhibit, internal/client fullscreen state, selectors, tags and tonemapping get distinct effect models. No-effect/absent is not the same as an explicit false. Editing a disabled rule retains its contents.

### Bind/submap/gesture editor: B1 / G1

BindSpec stores ordered actions/callback graph, key identity, modifier mask, flags, source order, submap path and inclusive/exclusive device/tag set. Shortcut recording uses GTK key events while focused; keycode conversion is verified against keymap fixtures, not a hardcoded assumed offset. Host-reserved keys can also be chosen through an onscreen keyboard/key catalogue.

If supported, a focused shortcut-inhibition request may aid recording; it is released on blur/close/timeout. Do not subscribe to global keyboard events or open /dev/input merely to make the recorder convenient. GTK recording does not necessarily identify the physical device; the user selects device targeting separately.

The current handler also accepts an explicit mouse bind flag, unlike the baseline handler's option reads. This was found during the deeper blueprint inspection and is added as a current-only contract. Enforce click/drag exclusivity, repeating versus long-press/release incompatibility and mouse incompatibilities according to the selected parser. A flag present in a newer stub cannot be emitted into the older release blindly.

Submap graphs require a visible escape/reset route, catchall semantics, nested transitions, universal bindings and version-supported automatic transitions. Duplicate shortcut actions can be intentional; conflicts are based on activation conditions and source order.

GestureSpec contains fingers/direction/modifiers/scale/inhibitor policy and a tagged action with its own arguments or start/update/finish workflow references. Emit unset with the original identity tuple; changing one identity field requires removing/replacing the prior gesture. A private virtual pointer is not proof of native trackpad gesture injection. Callback payload simulations and real gesture tests have different verification statuses.

### Animation editor: A1

An AnimationNode override contains enabled, speed/duration, curve kind/reference and style variant. Absence inherits its parent. Display all 34 public nodes, style compatibility and version-specific parameters. Emit disabled nodes without unnecessary speed/curve fields.

CurveSpec is Bezier with two control points, or Spring with mass/stiffness/damping. The pack maps friendly damping to baseline dampening where needed. Keep curve reference ownership and rename dependents together. Curve-response graphs are illustrative; the actual lab checks compositor behavior.

Scene triggers drive opening/closing, workspace transitions, grouping and focus. Angle loops require an explicit battery-cost indicator and stop when the preview is hidden. Own UI animations honor reduced-motion preferences independently of the user's desired compositor theme.

## 13. Action variants and visual programming

The machine contracts retain argument-read facts from both pinned handlers. These facts distinguish direct required reads, optional reads, positional arguments and callback fields, but are not automatically interpreted as a complete function signature. Required reads inside one branch are not required for every overload.

ActionSpec is a closed variant with schema-known callee, parameter variant and effect set. Important overloaded forms:

| Family | Distinct forms to expose | Validation |
|---|---|---|
| Focus | Direction, monitor, workspace/current-monitor behavior, window, urgent-or-last, last | Exactly one selected form; target grammar and availability |
| Window move | Direction/group-aware; x+y/relative; workspace/follow; monitor/follow; into group; into/create group; out of group | Variant-specific paired parameters; source precedence never silently resolves conflicting forms |
| Window swap | Direction; target aliases; next; previous | One variant; aliases normalize on edits |
| Window resize | Interactive/default; interactive aspect policy; x+y/relative/target | Paired coordinate requirement; mouse-bind compatibility |
| Fullscreen | Mode/action/target and optional layout awareness | Version accepted enums; internal/client state distinct |
| Fullscreen state | Internal/client/action/layout awareness | Typed state masks and parser-required combination |
| Group | Toggle/next/previous/index/move/lock/active-lock | Appropriate optional target, action or index form |
| Workspace | Rename/display-name, identity change, move to monitor, swap monitor workspaces, special toggle | Stable selector versus display name; special-prefix context |
| Cursor | Coordinates or corner with optional window | Coordinate units and allowed corner selection |
| Key forwarding | Pass, shortcut, key state, global shortcut | Required target/mod/key/state fields and release semantics |
| Process execution | Shell command plus optional spawn rule; raw execution | Explicit effect/trust classification and approved argument strategy |
| Session/device actions | Exit, DPMS, force-idle, renderer reload, release input capture, reload-config | Separate irreversible/hardware/lifecycle effects |
| Layout/custom event | Typed message or event payload | Version message grammar; private lab destination |

Forms compile through the release-specific action codec. Dispatcher construction and action execution are represented separately; constructing a dispatcher is not assumed to run it. Current wrapper implementation differences are part of the pack, not hardcoded into a universal compiler.

CommandSpec has distinct LaunchApp, ArgumentVector, ShellProgram and OpaqueImportedCommand variants. LaunchApp selects a discovered executable/desktop entry and typed arguments; imported desktop-entry metadata is displayed as data, not launched during discovery. Ordinary argument nodes never concatenate a window title/path into shell source.

For generated argument-vector launches, use a small fixed installed launcher helper with a typed, bounded argument payload where the compositor's raw-command splitter cannot preserve the intended vector exactly. Its contract is execute one reviewed command as the user, with no writer/recovery authority or resident service. The pack must prove exact quoting/argument transport. Explicit ShellProgram is a separately reviewed effect with its own source/trust record; OpaqueImportedCommand is preserved and mocked until understood. Config-string escaping and shell-argument escaping are separate operations.

WorkflowSpec is a typed graph with trigger, variables, nodes, edges, scopes and referenced resources. Initial node families are literal/value reference, query/filter, optional-match, arithmetic/compare, condition, sequence, bounded loop, action, owned setting/rule enablement, subscription, timer, notification, function call and geometry/layout.

Type-check the graph, detect unreachable nodes and cycles, validate lifetimes, ensure every branch handles missing targets, and calculate effect sets transitively. The review lists effects originating from callbacks as well as immediately executed actions.

Generated Lua uses our AST printer and stable identifiers. Calls use schema-defined argument forms. Named functions/callback closures are generated from graph nodes; no raw-body field is accepted by the ordinary compiler. An imported opaque function stays preserved but is excluded from safe preview and graphical-completeness claims.

An event node declares payload types from the release schema. Timers use the supported repeat/oneshot form, time units and owned handles. Notifications expose text/color/icon/font size/timeout plus supported object actions. Queries have typed optional results and selectors. Immediate property refresh is an advanced effect with a budget; it is never automatically appended after every action.

Managed callbacks have an operation/iteration budget derived from the graph and target count, with a conservative configurable bound. There is no unbounded recursion in ordinary workflows. Advanced language extensions require separate limits, isolation and coverage declarations.

## 14. Validation pipeline and evidence

Validation is staged:

1. Syntax and lossless-patch consistency.
2. Selected-version field/argument/type/enum and bounded grammar checks.
3. Semantic dependencies, conflicting variants, source ordering and ownership.
4. Effect/trust classification, resource limits, activation compatibility.
5. Isolated parser acceptance of generated output.
6. Private compositor scenario checks for relevant features.
7. Optional explicit physical-hardware trial.

The report includes checks skipped because of unavailable binary, component version, renderer, privilege, dependency or opaque source. A safe projection may pass while the full imported configuration remains unverified; show both scopes.

verify-config executes configuration logic and therefore runs in the isolated validation profile. There is no unsandboxed fallback. The strict profile has staged dependencies only; missing shell/modules/tools may change behavior. Such failures are classified as unavailable dependencies or environment differences rather than definitive proof of a broken live config.

Release CI can validate known trusted full fixtures under fuller private dependencies. User import does not auto-install or expose host dependencies to imitate every script.

## 15. Draft storage, publication and runtime apply

### Modes and storage

Editor-owned state lives under the user's XDG state/data/cache/runtime roots in a hyprland-editor namespace, with directories 0700 and private documents 0600 unless the user chooses a different exported artifact mode.

Draft storage contains model/version/source hashes and checkpoints. It is not linked into an active config. If discovery shows that the chosen draft directory is inside a watched wildcard graph, select another private location or disable draft persistence until an isolated path is chosen. No change to misc:disable_autoreload is made merely to save a draft.

If the watched graph is unknown because source computes dependency paths, do not assume the default storage location is unwatched. Keep the draft in memory until an appropriate persistence target is verified. Prefer passing preview bundles as sealed descriptors and materializing files inside namespace-private filesystems instead of staging them in a potentially watched host directory.

Apply modes are:

- SaveDraft: editor-owned draft only; no live socket mutations.
- Export: chosen non-active destination; warn/block if it is an active source target.
- ApplyRuntime: certified typed mutable fields/actions; disk remains untouched; effects reviewed.
- PublishAndApply: journaled active-file publication and activation.
- StageForNextSession: private pending generation and install plan; active files untouched now.

Next-session installation requires an explicit launcher/session integration that commits the pending generation before the compositor starts. Without that integration, provide a reviewed install operation after logout; do not advertise writing watched files as “restart later.”

### Prepared plan

Preparation produces an immutable plan containing document identities and authorized descriptors, old/new byte hashes, affected instances/components, exact source/schema/binary fingerprints, effect set, activation steps, backups, preconditions, expected readback and recovery limits. Hash its exact bytes and show a human-readable projection. Any draft/source/target change invalidates the prepared plan and requires new review.

All live mutations require this plan, including generated Lua setters. A slider changes the draft; it does not dispatch a command.

### Watcher handling

For multiple watched files, use a version-certified watcher gate:

1. Acquire the editor transaction lease for affected sources/instances.
2. Record current autoreload state and complete intended change/rollback bytes.
3. Arm the independent recovery agent before altering watcher state.
4. Through a closed trusted setter, disable autoreload, force the supported pending refresh, and verify the gate using the release's certified watcher behavior.
5. Stage and sync every replacement, validate all preconditions again, then replace files and sync directories.
6. Trigger one deliberate reload after all files are published.
7. Read back relevant config/runtime state and report reload errors/effects.
8. Restore the intended autoreload policy after verification or rollback.

Reading a true scalar alone does not prove an inotify gate is effective. Certification includes queued events, refresh ordering, wildcard parents, symlink targets, rename behavior and scripts that modify autoreload state. Reentrant reload or policy changes during the transaction invalidate the gate.

If the gate cannot be certified, use a proven single-root managed generation switch or defer multi-file active publication until the session is stopped. Never write files sequentially and call that atomic. Independent ecosystem daemons have their own watch/reload behavior; do not assume the compositor gate pauses them.

Reloading an imported Lua/legacy config may execute commands, callbacks or startup-like hooks even if the edited field is ordinary. Therefore the transaction's effect set includes reachable reload logic and unknown opaque effects. Pure scalar runtime apply can avoid full reload only where the target-specific setter is certified.

### Durable file algorithm

Open authorized parent directory descriptors. Resolve targets with ownership-aware checks; for the selected Linux baseline use openat2 resolution restrictions to prevent path escape and magic-link substitution. [openat2](https://man7.org/linux/man-pages/man2/openat2.2.html) supplies resolution controls, but concurrent replacement by other processes still requires identity/content revalidation.

Create random exclusive temporary regular files in the same filesystem, write prepared bytes, apply intended mode/ownership/ACL/xattr policy, sync files, then journal the prepared identities/hashes. Recheck source before rename. Atomically replace per file and sync directories; record each operation durably. Do not silently lose metadata if it cannot be retained: report and block unless an explicit intended ownership/mode change is part of review.

Backup exact old bytes and relevant metadata before publication, restricted to this transaction. Active publication is not atomic against all independent writers: Hyprland and unrelated editors do not share our lock. Concurrent edits after revalidation remain a detectable/recoverable conflict risk, not a guaranteed global lock. Stop/mark partial if published bytes no longer match our expected version.

Verification compares typed readback with source-defined normalization/precision tolerances, checks relevant topology/usable input, and collects configuration errors. A successful socket reply is insufficient.

## 16. Recovery protocol and state machine

~~~mermaid
stateDiagram-v2
    [*] --> Draft
    Draft --> Prepared: review accepted
    Prepared --> Armed: backup and independent recovery ready
    Armed --> Publishing: watcher gate or approved root switch
    Publishing --> Activating: all intended bytes published
    Activating --> Verifying
    Verifying --> Committed: ordinary checks pass
    Verifying --> Trial: guarded hardware change
    Trial --> Committed: explicit keep before deadline
    Trial --> Recovering: timeout or failure
    Publishing --> Recovering: interrupted or failed step
    Activating --> Recovering: activation failure
    Recovering --> RolledBack: verified recoverable state restored
    Recovering --> NeedsAttention: conflicts or unavailable target
~~~

The journal records monotonically numbered events with transaction ID, boot ID, plan digest, target generation, action sequence, file hashes and state. Replay is idempotent by action ID; it never blindly repeats external commands. A current checkpoint plus incomplete final journal event determines recovery.

The recovery agent runs independently from GUI crash cleanup. Prefer a narrowly configured transient systemd user service with a fixed installed executable and recoverable journal. A setsid/orphan alternative is enabled only after lifecycle tests prove survival on supported launchers/cgroups. Where neither is dependable, guarded hardware apply is unavailable. No privileged always-on watchdog is installed.

The control socket is in a private random runtime directory. Authenticate expected peer credentials, a fresh session nonce and the prepared-plan digest. Protocol operations are arm, progress acknowledgement, keep, revert and status; no raw paths, commands or eval. The agent obtains only already-authorized file/target capabilities.

Use a proposed 30-second hardware trial deadline based on CLOCK_BOOTTIME, not wall-clock timestamps. A stale or replayed keep message cannot extend it. If the UI dies, the agent reverts; if the agent crashes, the service/journal recovery handles it. Hardware mutation is forbidden before Armed acknowledgement.

Before each live rollback operation, recheck the original compositor identity. If it has exited, never dispatch to a replacement instance. File restoration is allowed only when current bytes are ours, source ownership is still valid and no conflicting active instance/config use is detected. Otherwise mark NeedsAttention and preserve recovery artifacts. There is no claim of perfect isolation against arbitrary concurrent session restarts or malicious same-user writers.

Restore recoverable file/runtime state and watcher policy, then verify usable topology/input where possible. Explicitly report what could not be restored. Killed windows, arbitrary program effects, plugin side effects, power/session actions and hardware failures are not general rollback targets.

Persist recovery instructions and an independent CLI status/revert entry that uses the same journal validation. The CLI does not accept arbitrary root-config paths to bypass authorization. Backups default to five successful generations or 30 days within a configurable 50 MiB quota; incomplete recovery material is protected from automatic deletion.

## 17. Private helper protocol

Use inherited or explicitly authenticated Unix socket pairs, preferably SOCK_SEQPACKET for control messages, with CLOEXEC by default and an explicit descriptor allowlist. No TCP/HTTP server. Role-specific endpoints accept only their own operations; a shared request schema is not authority to use every operation.

Do not assume SO_PEERCRED on a socketpair created before fork identifies the spawned worker: creation-time credentials can refer to the creator. Use a connect-after-spawn endpoint or verified per-message SCM_CREDENTIALS/SO_PASSCRED where sender identity is needed, accounting for namespace translation. Endpoint possession and the fixed role remain the primary capability boundary; PID checks are separately tied to process lifetime.

The helper request schema is a closed tagged union for parse, validate, preview start/input/stop, commit, recovery arm/keep/revert and status. Unknown versions, fields, operations, duplicate JSON keys, invalid IDs, oversized messages or unbound descriptor indices are rejected. JSON has a 64 KiB control-message cap and depth 32; large sources/assets/plan bytes are passed by sealed FD and verified hash.

Envelope fields are protocol version, request ID, transaction/session generation and operation-specific typed payload. A nonce and prepared-plan digest are required on recovery operations. Protocol generations are decimal strings on the wire, parsed with checked integer conversion. Every reply includes the request ID and generation.

Descriptor transfer is explicit: an FD is tagged SourceBytes, ProjectionBytes, PreparedPlan, AuthorizedDirectory or FrameBytes with expected hash/size and direction. Reject unexpected descriptor counts/types; close extras before responding. The receiver never treats “FD index 0” as a pathname.

Parser results contain ranges, symbols and candidate declarations only. The parent rechecks ranges against its immutable source bytes. Validation reports cannot approve mutations by themselves. Commit references an already-prepared plan and bound writer capabilities; it does not accept replacement shell commands or new target paths.

Frame transport is separate from JSON control. The broker sends an immutable sealed memfd plus width, height, stride, pixel format, byte length, lab ID and monotonically increasing sequence. Accepted format initially is one specified RGBA8 layout. The receiver checks multiplication overflow, dimensions, stride, FD type, seals and exact accessible length before mapping read-only.

Default preview size is 1280×720 at 15 FPS; cap initial negotiated dimensions at 1920×1080, each frame at 16 MiB and at most two retained frames. Drop older frames under backpressure; do not queue all frames. The broker creates/copies/seals a completed frame before transfer; reusing a writable shared buffer is not accepted as immutable transport.

Input messages are normalized pointer position, bounded scroll, keycode/key state, button state and release-all. They carry lab ID/generation/sequence and target the private seat only. On blur, stop or disconnect send/reconcile release-all. Input never names a host output or compositor socket. Clipboard, drag-drop, audio, file chooser and portal bridges are absent from ordinary preview.

## 18. Actual compositor preview implementation

### P1: trusted projection

The projection compiler accepts the typed draft and a scenario, emits a fresh minimal config, and copies selected bounded assets into a private bundle. It does not copy the original executable config wholesale. The output manifest lists included settings, mapped identities, omitted actions/plugins/source, mocked dependencies and known fidelity limits.

Preset scenarios create synthetic windows with chosen class/title/state, layer surfaces/panels/popups, workspaces, private output topology and repeatable interaction steps. Source order and rule declarations are retained where represented. Real client content, credentials, agent sockets and home paths are not inputs.

Owned graphical workflows run against a private/mocked effect backend. External commands, global shortcuts, session exit, idle power actions and host service changes are mocked. Native plugins and opaque Lua/modules are excluded. Shader graphs generated from trusted templates can be included; imported opaque shaders require a separately approved lab profile.

### P2: private compositor and frame/input broker

Start the supervisor, establish resource/isolation profile, create private runtime/state/config/data/cache roots and a private bus only if needed, then launch the exact supported compositor binary with an explicit generated config and private socket.

The broker connects only to that private Wayland socket. It negotiates the private output capture protocol and private virtual keyboard/pointer interfaces actually supported by the pack. Use [wayland-rs](https://github.com/Smithay/wayland-rs) client/scanner bindings and pinned protocol XML; the audited compositor protocol implementations are the behavioral reference. Missing protocols disable the related feature rather than causing a host connection.

Initial transport uses shared-memory capture copied into sealed raw frames. DMA-BUF import and cross-process GPU sharing are postponed; they need a separate buffer/device security contract. This intentionally favors a reviewable boundary over maximum frame rate.

Virtual keyboard/pointer actions are private only. Native multi-finger trackpad, physical tablet, USB switch and GPU/monitor behavior are not established by this protocol bridge. Those get simulated workflow payloads or VM/physical test fixtures with a different status.

Permission-policy scenarios may require narrowly identified broker exceptions so the control channel can display the lab while a separate synthetic client tests denial. Record those exceptions and do not claim that the broker itself is subject to the untouched user policy.

### P3: isolation profiles

The preferred safe profile is software/headless rendering with no GPU device exposure, subject to a proof that this Hyprland/backend build can start and render that way. Environment switches are profile-specific and tested; setting a Mesa variable does not prove software rendering works.

An optional trusted-projection render-node profile allows only a selected render node, never a DRM card/KMS device. It has shared-kernel/driver and resource risks and is labeled separately. Native plugins/untrusted full code are not permitted in that profile. A VM is the selected stronger boundary for an optional future full-code/plugin lab; it is not part of the ordinary memory budget.

Use bubblewrap to create private user/mount/PID/network/IPC/UTS boundaries as supported. Bind only a reviewed runtime image/dependency manifest read-only, explicit copied assets read-only, and private writable paths. Do not bind the entire home, host /tmp, host /run/user, session-bus sockets, /dev/input, /dev/uinput, DRM cards, host /proc or a host Wayland display. The namespace-local /proc and private tmpfs are different resources.

Build the environment from an allowlist. No inherited HYPRLAND_INSTANCE_SIGNATURE, WAYLAND_SOCKET, DISPLAY, XAUTHORITY, host DBUS address, credentials, LD_PRELOAD, arbitrary library/module search paths or shell startup variables. Install only required fixed locale/font/toolkit data and private lab values.

Set no-new-privileges, drop capabilities and apply a traced, versioned seccomp policy. Deny unauthorized network families, ptrace/process-memory access, namespace/mount changes after setup and other unneeded authority. GPU-specific ioctl policies, when allowed, are separately tested. Do not claim a generic syscall list works for every renderer.

Use delegated cgroup v2 for memory, CPU and PID limits and whole-lab teardown. [Kernel cgroup v2 documentation](https://docs.kernel.org/admin-guide/cgroup-v2.html) defines resource controllers and delegation; availability is probed rather than assumed. Where required controllers/delegation are missing, ordinary in-app sample remains available but executing lab modes fail closed unless an independently certified equivalent profile exists. No request for root to work around that automatically.

[Bubblewrap's own documentation](https://github.com/containers/bubblewrap) explains that protection depends on the configured sandbox. Our release profile must demonstrate the required boundaries rather than equate successful process launch with isolation.

### P4: supervisor and teardown

Supervisor states are Staging, Isolated, Starting, Ready, Degraded, Stopping, Cleaned or Failed. Ready requires expected process identity, private socket, sample output/client, valid frame, input-release behavior and active limits. A timeout or mismatch kills the private process group/cgroup and reports the failed gate.

Stop capture and release input, disconnect private clients, terminate the compositor/services, escalate after a bounded grace period, then verify no descendants/FDS/buffers remain and remove temporary storage. Use cgroup membership/process identities, never a broad kill by executable name.

Stop on user close, GUI loss, supervisor timeout or resource violation. Suspend rendering when hidden; terminate after a proposed two-minute hidden timeout. GPU memory/caches may not release instantly; measure and report teardown behavior rather than promise zero residual driver allocation.

Preview fidelity labels are SampleApproximation, PrivateCompositorVerified, MockedEffect, OmittedUntrusted, HardwareUnverified and Unsupported. Before/after frames use the same scenario and projection constraints. Changed resource limits that reduce visual fidelity appear in the projection report.

## 19. Ecosystem and platform adapters

Each adapter implements the section 5 trait and carries exact component version/source identity, grammar/schema, source ownership, runtime identity/control endpoint, reload/watch behavior, restore semantics and complete coverage list.

| Adapter | Implementation model | Activation/recovery contract |
|---|---|---|
| hyprpaper | Output-keyed source/mode/slideshow model and bounded asset references | Use version-specific daemon IPC; target one instance/output; restore known wallpaper state |
| hyprlock | Ordered widget scene graph, shared properties, variables/commands and auth model | Save draft/private render only normally; real auth/lock testing explicit; no actual password in preview |
| hypridle | Listener timeline, inhibition policy and typed/mock command hooks | Restart/reload exact owned instance if supported; never force sleep to test timeout editing |
| hyprsunset | Time/profile schedule with temperature/gamma/identity and hooks | Profile/runtime color state separately tracked; physical display trial needed for faithful color |
| hyprcursor | Theme manifest/assets/fallback/size per target toolkit | Inspect installed themes; stage imports; no arbitrary archive installer |
| hyprqt6engine | Typed theme palette/font/icon/style fields | Separate Qt sample client; identify process/session environment scope |
| hyprtoolkit | Typed global theme/style fields | Separate sample client; keep toolkit and application schema versions independent |
| XDPH | Screencopy preferences and portal-routing model | Private bus for preview; real services targeted; token/capture privacy explicit |
| hyprlauncher | Finder/behavior/output/history/cache model | Synthetic search/history; start/stop only owned selected daemon |
| hyprpicker | Typed capture/format action | Ordinary sample uses synthetic image; real capture explicitly user-triggered |
| hyprshutdown | Session exit/progress workflow | Mock by default; no reversible-transaction claim for actual exit |
| Qt support/polkit/system-info/PipeWire tools | Tool-specific settings/status/integration action models | No broad process restart, privilege bypass or private-data upload |
| Selected bars/launchers/notifications/themes | Versioned component-specific model; script modules opaque unless represented | Exact watch/apply rules; dynamic scripts treated as code |
| Nix/Home Manager/Hjem | Generated-source ownership mapping plus explicit structured export | Do not mutate /nix/store; no arbitrary Nix evaluation/build on import |
| UWSM/systemd/launcher integration | Typed service/environment/launch argument ownership | No duplicate autostart; stage restart/session-lifecycle changes separately |

The inventory's ecosystem tables mix configuration fields with CLI actions and widget lists. The contract records that uncertainty; a documentation row does not automatically become an editable configuration property. Each adopted adapter needs its own source certification before it is advertised complete.

hyprlock's preview is a widget scene with fake authentication state. Dynamic labels and onclick commands are mocked unless represented by safe nodes. The absence of PAM/fingerprint testing is displayed; a visual preview is not lockscreen security validation. Validate at least one usable authentication method and relevant locked-bind/above-lock interactions before any explicit real lock test.

Themes/profiles export a manifest of component versions, schema revisions, asset hashes, source trust/effects and portable selectors. Reject archive escape, special files, unsafe symlinks, duplicate/colliding targets, excessive expansion and undeclared executable hooks. Import never installs fonts/plugins/packages or executes palette-generation commands automatically.

Profile application across multiple tools is a dependency-ordered saga, not one atomic desktop transaction. Journal each tool's durable files/runtime action and compensate only where a verified restore operation exists. Surface a partial desktop profile if a later adapter fails.

## 20. Resource policy and performance implementation

The previous 100 MiB target is retained for ordinary editing, measured as summed process PSS with preview off and a documented representative fixture. It includes parser/asset workers present during that test. Large imports and actual preview have separate budgets; do not claim the target for unlimited input size.

| Resource | Proposed initial bound | Failure behavior |
|---|---|---|
| Ordinary steady editor PSS | At most 100 MiB on reference fixture | Performance gate fails; optimize/reconsider toolkit |
| Quiet idle CPU | Under 0.1% of one core averaged 60 seconds | Find polling/render loop; no “Linux is light” substitute |
| In-memory undo | 1,000 operations or 8 MiB | Checkpoint/spill older history |
| Thumbnail cache | 16 MiB | LRU eviction and downsampling |
| Source import | 8 MiB/file, 64 MiB graph, 1,024 files initially | Explicit quota result; configurable increase with changed memory expectation |
| Syntax/semantic nesting | 128 source nesting; helper JSON depth 32 | Bounded failure; preserve raw source for export |
| Worker count | Two ordinary jobs simultaneously | Bounded priority queue; cancel obsolete jobs |
| Query reply | 8 MiB metadata snapshot initially | Abort/reason; never silent truncation |
| Event record / queue | 64 KiB record, 256 records / 1 MiB total | Drop/coalesce nonessential events; invalidate and resnapshot |
| Private lab | 512 MiB memory.max, 64 PIDs, one-core CPU quota initially | Stop with resource report; explicit larger profile available |
| Lab storage | 256 MiB private scratch initially | Reject/evict safe temporary material; no host fallback |
| Preview frames | Default 720p/15 FPS; two retained frames | Drop older frames, bounded memory |
| Static parse/asset job | Five-second wall deadline and profile-specific address/memory cap | Cancel/kill worker, report oversized/incompatible input |

Hard limits are policy, not universal proof of sufficiency. Controllers, virtual-address limits and driver allocations have different meanings. The lab must demonstrate that its selected limit mechanism works; RLIMIT_AS is not misreported as resident-memory accounting. GPU resource containment remains a limitation of render-node mode.

Keep schema strings interned and load the selected pack only; memory-map/embed immutable schema data where useful. Share source bytes with spans, build view models for visible rows and keep search indexes compact. No duplicate whole-state JSON/native/HTML models.

Workers return bounded normalized results. Image workers render thumbnails to raw pixels; the GUI does not decode arbitrary original assets. Raw frame buffers are freed on lab stop, and model caches do not retain a full preview history.

Measure cold/warm launch, representative ≤1 MiB config, 10,000 binds/rules, maximum accepted import, repeated edits/search, event storms, preview cycling and large images. Record P50/P95 response, total PSS/RSS, child process counts, idle CPU, disk/network activity and GPU usage when available. Search target is <50 ms; edit feedback <100 ms; these remain unmeasured requirements.

## 21. Security invariants and test cases

These are mandatory implementation tests, not tests executed by this blueprint.

| ID | Requirement | Adversarial or failure fixture |
|---|---|---|
| S01 | Import is inert | Lua writes file/spawns process/loads library at top level; no effect occurs |
| S02 | Parser lacks authority | Malicious parser result requests arbitrary path/span/operation; parent rejects |
| S03 | Codec injection resistance | Quotes, controls, Unicode, delimiter/brace/statement payloads remain literal data |
| S04 | Instance isolation | Two sessions, socket replacement, PID reuse and restart invalidate pending mutation |
| S05 | Draft save is separate | Watch active config while saving draft; no reload or active-file changes |
| S06 | Watcher gate correctness | Queued events, wildcard dirs, symlink target, autoreload callback changes and reentrant reload |
| S07 | Publication crash recovery | Kill agent before/after each sync/rename/journal event; consistent recovery classification |
| S08 | Concurrent edit preservation | Another writer changes source/published bytes; conflict, not blind overwrite |
| S09 | Recovery independence | GUI SIGKILL and helper failure during risky change; armed recovery or explicit unavailability |
| S10 | Recovery target lifetime | Original compositor exits/restarts; no dispatch to replacement |
| S11 | Lab host isolation | Attempt home/runtime/Wayland/bus/agent/network/input/DRM-card access; denied |
| S12 | Lab exhaustion | Infinite Lua/large allocation/fork storm/excess frames; resource termination and cleanup |
| S13 | Protocol parser robustness | Invalid JSON/version/duplicate key/depth/size/FD count/replayed sequence |
| S14 | Frame integrity | Bad strides, overflow, mutable/truncated/nonregular FDs, wrong seals/format |
| S15 | Command/plugin trust | Import/build/preview does not execute host commands or load native host plugins |
| S16 | Capture/auth privacy | Ordinary preview receives no host surface/password; shortcut capture stops on blur |
| S17 | Permission truthfulness | Disabled enforcement/restart pending/narrow broker exception reported accurately |
| S18 | Asset/profile safety | Traversal, symlinks, decompression bombs, malformed images/fonts/ICC/shaders |
| S19 | Lifecycle idempotence | Repeated reload/apply leaves intended owned timers/binds/subscriptions once |
| S20 | Effect and rollback honesty | Exit/kill/suspend/plugin/opaque reload effects cannot be labeled fully reversible |
| S21 | Diagnostics privacy | Redaction fixtures for titles, paths, usernames, tokens, environment and commands |
| S22 | Configuration-manager ownership | Nix/store/system/generated/symlink roots cannot be silently overwritten |
| S23 | Supply-chain integrity | Tampered pack/dependency/runtime profile cannot gain write capability |
| S24 | Helper authorization | Parser/lab socket cannot invoke writer methods; nonce/digest/role failures rejected |

Complement them with grammar fuzzing, differential upstream-parser fixtures, action/rule/layout synthetic integration tests, UI keyboard/accessibility tests, hardware tests and resource/lifecycle tests. Run sanitizer-enabled native/FFI workers and file-race/fault-injection suites in CI. No real session-exit/lock/suspend action is used by ordinary unit tests.

Sensitive diagnostics have structured categories; tokens/source secrets are redacted before persistence. Logging is local, bounded and off for keyboard/capture payloads. No telemetry or automatic upload. A diagnostic export is a reviewable redacted artifact.

## 22. Packaging, certification and completeness accounting

First supported platforms are Linux x86_64/aarch64 builds with the required native dependencies. Certify each actual distro/kernel/sandbox profile; “runs on Linux” is not a sandbox compatibility claim. Nix builds must preserve pinned tool/runtime paths without guessing store regexes.

Ship the GUI, limited workers, apply/recovery agent, schemas and runtime-profile manifests through the package. Executables are resolved by installed fixed paths/hashes, not an imported profile's PATH. No downloaded compositor is silently executed. The selected installed binary must have matching certification; otherwise lab/apply is unavailable or uses a separately packaged verified lab runtime with its identity disclosed.

Release artifacts include a machine-readable dependency/SBOM manifest, source and binary hashes, third-party notices, reviewed dependency changes and reproducible-build records where feasible. License compatibility and attribution are checked before copying or linking upstream implementation code; extracting an API catalogue does not by itself settle redistribution requirements.

Do not invent a permissive supported-version range. The audited two snapshots are blueprint targets; a pack becomes supported after its full behavior evidence is attached. Packaging includes explicit compatibility versus upstream security-support status.

Feature accounting uses the implementation-contract manifest. For each version, every scalar, descriptor field/wrapper, match predicate, flag, API function, object operation, animation node, event/action grammar and declared adapter setting needs owner module, editor/node, codec, validator, observation strategy, effect/activation policy, preview status and test IDs.

Coverage states are Specified, Implemented, ParserCertified, ScenarioVerified and Released. This blueprint assigns Specified only; it does not fabricate passed tests. Shared engines can satisfy many rows only when parameterized tests exercise differing semantics and version variants. Display-only runtime fields are intentionally inspectors, not falsely editable settings.

CI compares upstream registration/descriptor inventories with the selected pack and fails on unexplained omissions or unknown codecs. Documentation candidate rows require classification as setting/action/reference/alias/unsupported before an adapter completeness claim. Existing unsupported source remains visible and exportable.

Schema/data migration is pure and deterministic, with old/new manifests and explicit changed behavior. Active configuration is not rewritten by opening a new app version. Old drafts remain recoverable if migration fails.

## 23. Engineering work orders and proof gates

| Work order | Build artifact | Completion gate |
|---|---|---|
| E01 | Workspace/types plus request schema and error model | Strict decoding, role-bound operations, no direct UI mutation path |
| E02 | Release exporter and codec corpus | Every audited registry entry resolved; source/default/argument discrepancies explained |
| E03 | Limited Lua/legacy parser and provenance graph | Inert import; lossless no-op; differential include/scope/variable fixtures |
| E04 | Instance-safe read transport and model reducer | Reconnect/stale-result/PID tests; honest sampled-state UI |
| E05 | Source patching and draft storage | No live reload on SaveDraft; ownership/conflict/metadata tests |
| E06 | Apply journal and independent recovery | Fault injection and watcher-gate tests before any real monitor/input apply |
| E07 | All scalar/special-string editors | Per-row codec/boundary/source-patch evidence plus accessibility |
| E08 | Structured and visual workflow editors | Every descriptor/variant/API node covered; ordering/lifecycle tests |
| E09 | Headless broker/sandbox prototype | Actual private frame/input without host sockets, active limits and verified teardown |
| E10 | Preview scenario suite | Fidelity/omission labels backed by results; no unsafe fallback |
| E11 | Exact-version ecosystem adapters | Complete classified source schema and independent activation/recovery suite |
| E12 | Packaging/performance/security certification | Full coverage report, reference benchmarks and all mandatory security gates |

Prototype E03, E06 and E09 early. The actual preview and watcher/recovery protocols can invalidate later UI assumptions. This is a scheduling decision, not permission to market a partial settings app as complete.

A failed headless/renderer experiment disables that lab profile; a failed recovery or watcher gate blocks the associated active publication mode; a failed parser/codec test blocks that feature/version from apply; a failed memory budget triggers optimization or a reviewed architecture change. No proof gate falls back to host execution.

## 24. What this blueprint does and does not establish

The blueprint fixes module boundaries, authority, dependency choices, data/state models, parser/edit algorithms, canonical complex-value forms, feature editor families, action variants, worker messages, preview architecture, resource policy, apply/recovery states, adapter contracts, test requirements and release gates. The per-feature contracts make those requirements traceable.

Static source inspection and document consistency checks can validate coverage and catch wrong API assumptions. They cannot establish sandbox resistance, crash recovery, hardware behavior, library-version build compatibility or the 100 MiB target. Those require the specified prototypes and tests. No GUI/compiler build, real apply, preview launch or penetration test was performed for this blueprint.

Exact dependency locks, syscall/runtime manifests and certification records are implementation artifacts produced by E01/E02/E09/E12. Their required structure and acceptance criteria are specified here; there is no invented claim that an unbuilt binary/profile is already certified.

All downloaded parser/source files are inert audit material and are removed after the blueprint checks. The retained evidence manifest stores URLs, revisions and SHA-256 hashes rather than repository checkouts.

## Artifact verification record

The manifest was checked for unique IDs, required engineering contracts, exact scalar/descriptor/API coverage against both audited snapshots, and handler links/hashes for all 100 union public Lua functions and all 39 object methods in each available version. The helper schema examples passed 12 positive and 19 negative shape checks using a standard-library validator for the keywords used; a full production decoder/metaschema validation remains an E01 requirement.

All 163 downloaded source-evidence files and temporary generation/check scripts were deleted after those checks. URLs, commit IDs and hashes remain in the contract JSON. No live Hyprland configuration, package, service or toolchain was changed. No runtime safety or performance certification is claimed.
