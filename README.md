# Hyprland Editor

[Preview preparation](docs/preview-preparation.md) runs fresh sample staging in a restricted cancellable worker, with explicit layouts/mappings and a searchable fidelity review. [Review setting values](docs/preview-setting-layers.md) optionally compares pending values with reconstructed scalar source arguments and pinned default metadata; explicit choices affect only the sample. Preparation changes no configuration and starts no compositor. Full source/default resolution, assets and actual rendering/capture/input remain required.

A native GTK4/libadwaita workspace for making Hyprland configuration graphical, with a versioned catalogue and a safety-first editing core. The long-term goal is complete configuration coverage, including rules, monitors, bindings, animations, devices and isolated preview.

**Status: experimental 0.12 editor. This is the first working slice, not a complete Hyprland configuration manager.** Its live inspector is read-only. It does not apply changes to the running compositor.

![Native draft workspace](docs/screenshots/draft-workspace.png)

![Monitor draft editor](docs/screenshots/monitor-drafts.png)

![Device draft editor](docs/screenshots/device-drafts.png)

## Available now

- Browse by category and search readable names or original setting keys for every audited scalar: 353 for **0.56.2**, 402 for **audited-main**, an exact development commit. Switch snapshots without losing their separate in-memory drafts.
- Adaptive sidebar navigation, focused value cards, expandable technical details, a main menu and keyboard shortcuts. The layout follows the [GNOME HIG](https://developer.gnome.org/hig/principles.html); see the [UI design rules](docs/ui-design.md).
- Graphical switches, number controls, text fields, color pickers, gradient stops, coordinates, four-sided gaps and font weights.
- Stage validated overrides, remove them, undo field and source revisions within a rolling 1,000-operation / estimated 8 MiB retained-data budget, and review generated Lua before copying it. Unsaved drafts require explicit confirmation before closure or replacement.
- Import literal legacy `.conf` settings without executing commands, expanding variables or following `source` directives. Skipped lines and repeated keys are reported. This is a partial import, not an effective-state reader or source-preserving round trip.
- Edit ordered workspace drafts with all 15 audited fields, workspace-address/filter builders, enabled/unset controls, layout choices, master/scroller options, ordering warnings and private persistence. Commands are never run and workspaces are never created; complete layout/workflow/live behavior remains open. See [workspace scope](docs/workspace-editor.md).
- Build binding/submap drafts with graphical chord choices and focused recording, all audited flags, ordered action forms for 51/52 public dispatcher constructors, automatic transitions, escape bindings and literal rename/subtree duplication. Save/reopen, reorder and undo without executing actions. Full selector/workflow builders and live behavior remain open; see [binding scope](docs/binding-editor.md).
- Edit device drafts for 47 fields (43 original plus four source-discovered tablet-tool fields) and the device name. Use linked layout/variant lists and grouped keyboard options from installed XKB metadata, numeric acceleration/scroll-point builders, touchpad choices, tablet coordinates, eraser/button choices, pressure defaults/ranges, transforms and keymap path selection. Save/reopen, unset, reorder, duplicate and undo. Categories are author hints, not detected capabilities; hardware is never changed. See [device scope and limitations](docs/device-editor.md).
- Edit ordered monitor drafts with all 22 audited monitor fields and fallback/name/description output matching. Use graphical resolution/refresh/modeline builders, automatic placement or signed coordinates, scale, rotation, mirroring, color/HDR/VRR controls, reserved space and ICC path selection. Add, duplicate, reorder, remove and undo; save/reopen rules in private `.hye` workspaces. A layout diagram supports dragging outputs with explicit dimensions, scale and position; it is **not a compositor preview**. See [monitor scope and limitations](docs/monitor-editor.md).
- Import a conservative static projection of Lua `hl.config`, `hl.monitor`, `hl.device`, `hl.bind`, `hl.define_submap`, `hl.workspace_rule`, `hl.window_rule` and `hl.layer_rule`, `hl.curve`, `hl.animation` and `hl.gesture` declarations and restricted pure local expressions. A separate parser process has CPU/memory/time limits and denies filesystem, networking and process-creation syscalls after startup. No Lua is executed. Per-line import diagnostics explain the unresolved parts.
- Prepare a copy of imported Lua with only changed, existing literal expressions replaced. Source hashes, spans, reparsing and semantic comparison guard the copy; comments and unaffected bytes remain intact. Adding/removing fields and editing derived expressions remain unsupported. Source files are never overwritten.
- Read a manually refreshed scalar snapshot from the compositor selected by your session environment. Runtime directories, held process lifetime, executable fingerprint, held socket-node identity and peer credentials are checked. Only `j/version` and `j/descriptions` can be sent. Exact audited commits receive typed observations; unsupported formats stay raw or unknown.
- Save private `.hye` workspaces in `$XDG_STATE_HOME/hyprland-editor` (normally `~/.local/state/hyprland-editor`), including scalar, monitor, device, binding, workspace, window/layer rule, animation and gesture drafts, exact upstream/catalogue identity and an imported Lua source snapshot. Reopening reparses snapshot bytes in the restricted worker; it never reopens recorded original paths or trusts saved parser spans. Value-only JSON drafts remain importable. Retain an analyzed graph of up to 32 Lua load views, with exact source bytes and resolution inputs; reopening rebuilds its model without opening original paths or restoring folder access. See [graph workspace scope](docs/source-graph-workspace.md). Three exact older graph versions can be rebuilt with [explicit review](docs/graph-model-migration.md), preserving the old file and saving a new private copy.
- Select Lua/legacy source files through bounded symlink-chain resolution; record resolved targets and link identities without executing config includes. Saved snapshots retain this evidence without accessing recorded paths.
- Inspect possible module/file-loader calls, then open **Review Dependency Files** to allow folders and capture selected files for temporary review. Forget access, inspect immutable source snapshots and remove captures without changing the draft or desktop. Captured graphs can be retained and reopened privately; complete dependency evaluation, source editing and graph reconciliation remain unfinished. See [capture scope and limits](docs/dependency-capture.md).
- With a retained source graph, capture an incoming configuration under temporary folder approval and review file matches, archive/discard choices, changed context and text conflicts. Resolve observed private imports through fresh folder capture, explicit preserved/archive reuse or an acknowledged missing-input assumption. Verify the proposal before returning to final review; keeping, saving/reopening and undo preserve the root and graph together. General ownership/export and full workflow acceptance remain unfinished. See [external merge review](docs/source-text-merge.md#graphical-review) and [private import resolution](docs/private-import-resolution.md).
- Verify an explicitly selected original source or identical relocated copy. The editor checks descriptor-derived device/inode/owner/mode/timestamps, link-chain identity and SHA-256; changed content is rejected. Recorded ownership is provenance, not permission to overwrite a file. Source identity changes require saving, and late saves cannot mark newer edits as saved.
- Compare an explicitly selected changed Lua file against the imported snapshot and your draft. Independent scalar declaration changes merge; conflicting edits/deletions require a choice. Incomplete projections require explicit acknowledgement and never infer a missing value as unset. Review both source texts, accept the captured revision into memory, and undo its source identity and changed values together. No original file is written.
- Inspect the 1,453 source-audited feature contracts and the engineering blueprint.

- Edit window/layer rule declarations with all 57/60 audited window effects, ten layer effects, consumed conditions, exact-text/regex sample matching, geometry inputs, named accumulation findings and explicit ordering. Save/reopen drafts, duplicate, remove, reorder and undo. Opacity, fullscreen, grouping, event/mode and border-pair controls are available. Complete geometry/selector/style builders and runtime conformance remain in progress; see [structured rule controls](docs/rule-semantic-controls.md). See [rule scope](docs/rule-editor.md).

- Edit ordered animation branch, Bézier and spring drafts with inherited defaults, versioned spring spelling, curve references, style presets, control-point dragging and source-derived timing. Save/reopen, duplicate, reorder, remove and undo. The curve diagram is not a compositor preview. See [animation scope](docs/animation-editor.md).

- Edit ordered gesture drafts with fingers/direction/modifier matching, versioned motion multipliers, built-in actions, phase/legacy action blocks, exact unset identity and registration findings. Save/reopen, reorder, duplicate and undo. Callback data samples stay inert; general event-dependent workflows and real input behavior remain open. See [gesture scope](docs/gesture-editor.md).

The development worker also has an [ordered immutable-module evaluator](docs/ordered-module-evaluation.md), tested against independent Lua/source observations. It is connected to captured files in **Source analysis**, including a [graphical typed source comparison](docs/source-change-gui.md). Choose a settings declaration, change a typed value and compare its selected expression or a traceable original literal. The comparison identifies the source file and additional modeled effects. Keep it in the private draft, save/reopen its piece-based source revisions and undo within the current session. Original captures stay intact. **Review Value Overrides** compares the separate layers and lets you explicitly keep them separate, use the source or write a supported typed override into its last modeled declaration. Review the proposal before keeping it; one undo restores both layers. See [source/override review](docs/source-override-review.md). Override-only export remains blocked while source revisions exist because full source export is unfinished. See [private source revisions](docs/private-source-revisions.md). The source model now covers all nine scalar parser families, including colors, gradients, vectors, signed gaps and font weights, with exact numeric transport and versioned constraints. See [complex value codecs](docs/complex-value-codecs.md). Its `hl.config` records are argument observations, not verified effective settings.

## Not available yet

Complete Lua interpretation/include graphs, source-field insertion/deletion, complete editable multi-file source graphs and reconciliation, event-based live refresh, multi-instance selection, complete rule semantics/builders, complete animation and gesture semantics/builders and other structured editors, complete workspace/layout/workflow codecs, complete binding selectors/workflows and live behavior, device hardware inspection and source editing, monitor hardware inspection and source editing, plugins, live apply, display recovery, sandboxed compositor preview, distribution packages and release certification. Unknown defaults stay visibly unresolved. Bounds are literal source metadata, not a guarantee that combinations are valid on your system. Hardware behavior has not been tested.

Saved workspaces contain the full imported Lua source and path/ownership evidence; keep them private when sharing reproductions. Saving a draft never targets `hyprland.conf` or `hyprland.lua`. The application has no config publication API. If your own configuration explicitly watches the app's independent state directory, choose a separate state root before running it. Draft export contains only overrides and must not replace your full configuration. Source-copy export contains preserved imported text, including opaque statements, and needs separate review before any manual integration.

## Build and run

Linux (pidfd support for inspection, close_range/seccomp support for workers), Rust 1.90, GTK >= 4.12 libadwaita >= 1.5 libxkbcommon, RE2 and a C++17 compiler are required. Importing or comparing an explicitly selected Lua/legacy file additionally requires enabled Landlock ABI 3 or newer; unavailable isolation produces an error. Draft editing and reopening private saved workspaces remain available. No web server, browser runtime, background daemon or root access is required.

On Ubuntu 24.04:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libxkbcommon-dev libre2-dev liblua5.4-dev
```

On Arch Linux:

```sh
sudo pacman -S --needed rust pkgconf gtk4 libadwaita libxkbcommon re2 lua54 gcc
```

From the repository, using a working Rust toolchain:

```sh
cargo build --locked --release
./target/release/hyprland-editor
```

The repository pins Rust through `rust-toolchain.toml` for rustup users. Existing system Rust installations must meet the stated compiler requirement. GTK development packages are needed at build time; GTK and libadwaita runtime libraries are needed to run the resulting binary. It can open as a draft workspace on any graphical Linux desktop; Hyprland itself is not required for this slice.

The development branch links Lua 5.4 for compilation-only validation in the restricted worker, with 64-bit integers and IEEE double precision. It never invokes the compiled source. Install the [Lua 5.4 package](https://archlinux.org/packages/extra/x86_64/lua54/) alongside newer Lua versions; the build explicitly probes `lua5.4`.

Both audited Hyprland source snapshots request C++26 and search Lua 5.5 aliases followed by generic alternatives. That search does not enforce a combined Lua version range. The application’s Lua 5.4 model is not verified compatible with the selected compositor. Source analysis displays this limitation; [the compatibility plan and workspace reference matrix](docs/native-runtime-compatibility.md) describe the separate profiles and required worker integration.

An [optional separate Lua 5.5 compiler worker](docs/lua55-compiler-worker.md) now builds and passes isolated process tests against official 5.5.0/5.5.1 libraries. It provides compile-only and primitive C-API operations without GTK or Lua 5.4. The menu’s [Check Optional Compiler](docs/compiler-worker-launcher.md) action checks a root-packaged self-contained image through a sealed executable and bounded parent protocol. Source-profile selection and Lua 5.5 ordered evaluation remain unfinished; saved graphs still reconstruct with Lua 5.4.

The [numeric compiler observation](docs/lua-numeric-codegen.md) distinguishes the linked compiler’s zero-subtraction optimization, including reviewed backports. A fixed compile/dump probe never runs Lua; unknown modes leave affected expressions unresolved. Source comparisons and private save/reopen/undo retain zero sign bits. Older saved graphs require explicit review before the compiler observation is renewed.

The development worker reports its compiler/library profile; the reference matrix checks Lua 5.4.6–5.4.9. See [ordered evaluation and limits](docs/ordered-module-evaluation.md). For explicitly selected private fixtures, `hyprland-editor --probe-source-folder FOLDER CANDIDATE` reports bounded metadata without reading source bytes; [the capture contract](docs/dependency-capture.md) explains the distinct missing/blocked outcomes. This diagnostic does not import or verify an effective dependency graph.

A saved workspace can be checked without opening the GUI:

```sh
./target/release/hyprland-editor --check-workspace /absolute/path/to/workspace.hye
```

A changed Lua source can also be compared without a GUI:

```sh
./target/release/hyprland-editor --check-source-comparison /absolute/path/workspace.hye /explicitly/selected/config.lua
```

This reports required choices and projection limits; it does not accept the revision or write either input. The results describe draft/static-source checks and explicitly report that runtime behavior is unverified. Schema mismatches require migration; this command does not update the file.

## Development

```sh
cargo fmt --all --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
python3 scripts/check_catalogue.py
python3 scripts/generate_actions.py --check
python3 scripts/workspace_source_probe.py
python3 scripts/implementation_report.py --check
```

Pure editing/storage tests need no display. Native UI smoke testing is separate. See [CONTRIBUTING](CONTRIBUTING.md), [SECURITY](SECURITY.md), [coverage and roadmap](docs/roadmap.md), [next implementation gates](docs/implementation-status.md), [full completion register](docs/completion-register.md), and [engineering blueprint](docs/engineering-blueprint.md).

## Architecture and memory

`editor-core` owns validation, static source projection, span-based copy preparation, Lua encoding and private draft storage. `editor-discovery` owns the fixed read-only IPC interface. `editor-app` owns the native interface and launches its own inode through `/proc/self/exe` for isolated parser, [selected-file capture](docs/source-capture.md), and approved-folder metadata/capture roles. The parser has no filesystem access; capture access is bound to held file/directory descriptors. There is no compositor mutation interface. Background results are discarded when the draft generation changes.

Old JSON drafts contain values only. New `.hye` workspaces retain immutable source bytes and file identity evidence, with an exact catalogue fingerprint. Parser reports remain in memory and are recomputed on reopen. An analyzed Lua graph can retain up to 32 load views and their historical resolution inputs. Reopening reconstructs its model from private saved bytes. Single-root operations refuse to discard attached graph inputs; private source revisions persist separately from captured bytes; complete graph reconciliation, persistent operation history and legacy source reopening remain unfinished. Importing or reopening never reads referenced files automatically; dependency analysis requires explicit allowed folders and an Analyze action. Live observations are point-in-time reads, kept separate from draft values; they can become stale. File source projections do not establish effective runtime values.

The setting list uses GTK's recycled list items; only the selected setting builds its form. Feature contracts load on demand. Undo stores inverse field changes and shares immutable source snapshots. Its rolling history drops oldest operations within the operation/data budgets; checkpoints and persistent history remain outstanding. The retained-data estimate is not a bound on allocator overhead or whole-process memory. No background daemon or resident helper is installed. The 100 MiB proportional-memory budget from the blueprint is a target, not a measured promise; see [validation](docs/validation.md) for actual checks.

## Provenance and license

[Catalogue provenance](data/provenance.json) pins the audited Hyprland and wiki commits. `audited-main` is not automatically the latest version. Generated metadata records link each setting to its source. [Third-party notices](THIRD_PARTY_NOTICES.md) explain the distinction between source-derived facts and executable dependencies. Original project code is MIT licensed. Contributions should preserve upstream provenance and never treat the audit catalogue as runtime-certified.


The source-graph development branch now includes **Review Dependency Files → Analyze File**. It analyzes a selected root snapshot and dependencies from explicitly allowed folders, showing source files, modeled calls and partial-result reasons without running source code or changing the draft/desktop. Captured graphs can be retained in the draft and saved/reopened privately. Primitive source changes can be compared, kept privately and saved/reopened with within-session undo. Complete multi-file feature editing, persistent history and reconciliation remain unfinished. See [scope and limits](docs/ordered-module-evaluation.md).

For private fixtures: `hyprland-editor --check-source-graph FOLDER ROOT VERSION [CWD]` uses the same protected capture/model path. Supply an audited version and only a working directory you intend to model; no active compositor context is inferred.

The dependency review’s **Analysis Context** dialog supplies home/working-directory folder hypotheses and inherited Lua search templates. Unknown context stays unknown, and these selections do not grant file access or identify the active compositor’s context.

Current [resource observations](docs/resource-observations.md) record three 60-second empty-workspace trials at 92.3–93.3 MiB PSS, with zero recorded CPU ticks. Imported configurations, reviews, history, workers and other rendering profiles still require resource verification.

Preview engineering status: [resource preparation and backend feasibility](docs/preview-supervision.md). This is an internal foundation; rendered preview remains unfinished.
