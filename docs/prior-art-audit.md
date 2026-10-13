# Hyprland graphical configuration audit

Audited on 12 October 2026. Target compositor: the installed Hyprland 0.56.2 binary.

## Decision

None of the three reviewed projects meets the complete requirement: expose every setting for the installed release, distinguish saved configuration from active state, preserve existing configuration, provide an isolated compositor preview, and keep resource use small.

For this particular product, I recommend a native Rust/GTK interface with a new configuration and preview core. HyprViz supplies useful examples of native editors and lazy category loading. HyprSettings supplies a useful example of generating a catalogue from upstream definitions. Spinualexandru's hyprconfig supplies useful Lua parsing and targeted editing techniques. A direct fork of any one project would still need substantial architectural work.

This is a source audit with isolated Python parser checks. It is not a runtime certification or a memory benchmark. No application was installed or launched, and no live Hyprland setting was changed. The downloaded repositories and temporary audit material were removed after preparing this report.

## Exact revisions

| Project | Revision reviewed | Last commit date |
|---|---|---|
| [HyprViz](https://github.com/timasoft/hyprviz/tree/bf13f24859251756531e0b70cf0d3ff4eecc399e) | `bf13f24859251756531e0b70cf0d3ff4eecc399e` | 25 September 2026 |
| [HyprSettings](https://github.com/acropolis914/hyprsettings/tree/2343820372012eb436d7d24fe645f7948651d37b) | `2343820372012eb436d7d24fe645f7948651d37b` | 14 May 2026 |
| [hyprconfig — spinualexandru](https://github.com/spinualexandru/hyprconfig/tree/6bae742ca2a1598ec4f66c6edf9334050f96f3ff) | `6bae742ca2a1598ec4f66c6edf9334050f96f3ff` | 26 September 2026 |

Source links below are pinned to these revisions. Repository names alone are ambiguous: several unrelated projects use “HyprConfig.”

## Requirements comparison

| Requirement | HyprViz | HyprSettings | spinualexandru/hyprconfig |
|---|---|---|---|
| Current Lua config support | No in reviewed application paths | No in reviewed config-loading API | Yes, for statically recognizable constructs |
| Complete settings catalogue | Broad, hard-coded controls; gaps against 0.56.2 | Generated 0.54 base catalogue plus extras; gaps against 0.56.2 | Selected functionality; no complete scalar catalogue |
| Saved versus active values | Primarily file-based; monitors/system information queried | Primarily file-based; runtime queries mainly serve app-window behavior | Monitor state queried; bindings, variables, rules primarily parsed from files |
| Existing config preservation | Separate managed `.conf`, source expansion and atomic writes | Preserves many comments/structures; reconstructs formatting; include bug reproduced | Targeted Lua edits with stale-content check; heuristic parsing limits |
| Isolated desktop preview | No implementation found | No implementation found | No implementation found |
| Apply/recovery | Save/reload and edit history; no independent recovery helper found | Direct file writes; optional dry-run/manual save; no independent recovery helper found | Immediate writes/reload; display timer lives in frontend |
| Memory architecture | Native GTK; categories created on demand | Python, Flask and webview; eager config-item rendering | Rust/Tauri and React webview; current page rendered on demand |
| Measured memory | Not measured | Not measured | Not measured |

“Not found” refers to the reviewed application source, not a claim about every historical branch or dependency.

## 1. HyprViz

### Useful parts

The project has many explicit controls and structured editors for monitors, animations, bindings, workspaces and rules. Its editor code includes a Bézier drawing widget and typed controls for complex configuration values. [Advanced editors](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/advanced_editors.rs)

Categories are built when first visited. The configuration is shared between category objects; the application does not build every category's widgets immediately. Once rendered, categories remain available. This is a useful resource-saving pattern, although it does not establish a particular memory footprint. [Lazy rendering](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/widget.rs#L1438), [category initialization](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/gui.rs#L1360)

Writes use temporary files, file synchronization and rename, and resolve symlinks before replacement. These are useful foundations for preserving user-managed dotfiles. [Atomic writes](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/utils.rs#L266)

### Gaps relevant to this product

The main path is hard-coded to `~/.config/hypr/hyprland.conf`; the generated file is `hyprviz.conf`. The main flow parses Hyprlang and can add a source directive to the original configuration after a confirmation. It does not implement the Lua configuration path required for a current Lua setup. [Path constants](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/utils.rs#L1959), [main flow](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/main.rs#L66)

Controls come from explicit definitions in `widget.rs`, rather than a catalogue discovered from the installed compositor. The comparison found matching literal control names for 316 of 353 scalar keys in Hyprland 0.56.2's main value registry. The remaining 37 were not matched by that extractor. This measures explicit names only; generic text entry does not count as a verified graphical editor, and a name match does not prove correctness. [Control definitions](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/widget.rs#L1465)

A concrete wrong-path example appears in the decoration editor: `input_methods_ignorealpha` is registered without the `blur:` prefix. The save logic consequently places it in the decoration section instead of decoration/blur. Hyprland's 0.56.2 registry expects `decoration:blur:input_methods_ignorealpha`. This was established by inspecting the definition and serialization paths; it was not applied to a running compositor. [Control](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/widget.rs#L1960), [serialization](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/gui.rs#L1490)

The saved configuration is used as the basis for most controls. Runtime monitor queries are present, but no general option-value reconciliation or compositor event subscription was found. Saving writes the managed configuration and reloads the real compositor. The curve drawing/editor is not a separate Hyprland desktop preview. [Save path](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/gui.rs#L997), [runtime utilities](https://github.com/timasoft/hyprviz/blob/bf13f24859251756531e0b70cf0d3ff4eecc399e/src/utils.rs#L172)

Verdict: the strongest native GUI starting point of these three, but substantial work remains in Lua support, catalogue maintenance, state reconciliation, preview isolation and recovery.

## 2. HyprSettings

### Useful parts

It generates a typed catalogue from an upstream configuration-description header and merges manually maintained additions. This is an existing implementation of the source-derived catalogue idea. The reviewed generator selects `ConfigDescriptions0.54.hpp`, and the shipped catalogue identifies the same source version. [Generator](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/tooling/ConfigDescriptions/sconfigdescriptions_parser.py#L112), [generated catalogue](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/ui-src/scripts/HyprlandSpecific/hyprConfigDescriptions.ts), [catalogue assembly](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/ui-src/scripts/utils/configDescriptionTools.ts#L14)

Its parser retains nodes for comments, blanks, groups, keys and sourced files. A temporary fixture confirmed that ordinary header and inline comments survive reconstruction, although indentation and final-newline formatting change. It offers a dry-run setting that suppresses writes, plus manual-saving behavior. Dry-run is useful draft behavior, but supplies no actual compositor preview. [Parser](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/hyprsettings_utils/hyprland_parser.py), [saving behavior](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/ui-src/scripts/utils/utils.ts#L99)

### Gaps relevant to this product

The config-loading API handles `.conf` and Niri `.kdl` files, with no `.lua` branch. In this revision it therefore is not a Lua Hyprland configurator. [Loading API](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/hyprsettings_utils/pywebview_apis.py#L33)

The bundled base catalogue contains 328 unique keys; 324 match the 353 keys in Hyprland 0.56.2's main scalar registry. The extra catalogue adds none of the missing 29 scalar names. Four old names do not match that registry. This is a catalogue-name comparison, not a 92% overall feature-coverage claim. Structured features, plugin options and control correctness remain separate.

A nested relative-source failure was reproduced using only temporary files:

```text
hyprland.conf -> ./sub/first.conf
sub/first.conf -> ./second.conf
```

The parser looked for `second.conf` beside the main config rather than beside `sub/first.conf`, and did not load the actual second file. The implementation uses a module-wide base path when resolving relative sources. Upstream's legacy handler instead tracks the current sourced file while recursively parsing. [Parser source handling](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/hyprsettings_utils/hyprland_parser.py), [upstream source handling](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/config/legacy/ConfigManager.cpp#L1802)

Saving reconstructs selected files and replaces them through debounced temporary writes. No compositor validation, backup/recovery transaction or stale-content check was found in this save path. A normal close uses immediate process exit, while file writes are timer-based, creating a code-level risk of losing pending writes if closed immediately. This close/write race was not reproduced. [Writes](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/hyprsettings_utils/hyprland_parser.py#L83), [closing](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/hyprsettings_utils/window_manager.py#L60)

There is also a concrete issue to fix before reusing its local HTTP architecture: the host is `0.0.0.0`, the Flask server uses that host, and the read/write routes have no authentication checks in the inspected code. Cross-origin access is enabled. Actual remote reachability depends on firewall/network configuration; no server was launched during this audit. [Host](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/hyprsettings_utils/shared.py#L83), [server startup](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/hyprsettings#L31), [routes](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/hyprsettings_utils/flask_routes.py#L18)

The application runs Python, Flask and a webview. Its renderer walks and creates editor items across the configuration rather than virtualizing visible rows. Daemon mode can retain the app after hiding the window. These are resource considerations, not measurements. [Renderer](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/ui-src/scripts/ConfigRenderer/_configRenderer.ts#L209), [window lifecycle](https://github.com/acropolis914/hyprsettings/blob/2343820372012eb436d7d24fe645f7948651d37b/src/hyprsettings_utils/window_manager.py#L60)

Verdict: useful catalogue-generation and traditional-config editing reference; a poor direct base for this project's current Lua, isolated-preview and native-memory requirements.

## 3. spinualexandru/hyprconfig

### Useful parts

Its Lua support is real. The backend discovers recognizable imports, masks comments while preserving byte offsets, parses selected `hl.*` calls, and replaces only changed argument spans. Before targeted edits, it rereads the file and rejects stale contents. These are valuable techniques for preserving unrelated comments and code. [Lua backend](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src-tauri/src/hyprland_lua_backend.rs#L1081), [targeted writing](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src-tauri/src/hyprland_lua_backend.rs#L1369)

It checks a custom config environment variable, XDG config location, and Lua versus legacy files. It also reads active monitor information and checks the compositor's reply when applying monitor changes. [Discovery](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src-tauri/src/hyprland_lua_backend.rs#L28), [monitor application](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src-tauri/src/hyprland_backend.rs#L1309)

Nineteen unit tests in the main Lua backend cover comment masking, dispatcher translation, targeted edits, monitor-field preservation and modular configurations. These were inspected but could not be executed in this environment.

### Gaps relevant to this product

The General page is a placeholder. Appearance mainly handles wallpaper and Matugen preferences. Window and layer rule pages support inspection and deletion rather than a complete rule-creation/editing workflow. There is no general dynamically discovered scalar-settings catalogue. [General page](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src/pages/General.tsx#L25), [Appearance](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src/pages/Appearance.tsx), [rule UI](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src/pages/Windowrules.tsx), [registered backend commands](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src-tauri/src/main.rs#L12)

The Lua parser is a static recognizer, not an evaluator of the active configuration. Calls are collected by scanning source, and variables are merged into one name map. This cannot generally distinguish bindings generated by loops, calls inside unexecuted functions, or shadowed locals. Source-only parsing must not be presented as authoritative active state. These limitations are inferred from the implementation; the proposed Rust probes did not run. [Binding extraction](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src-tauri/src/hyprland_lua_backend.rs#L425), [variable map](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src-tauri/src/hyprland_lua_backend.rs#L680), [call collection](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src-tauri/src/hyprland_lua_backend.rs#L1544)

Lua writes use direct file replacement through `fs::write`, followed by reload. They lack atomic rename, backup and compositor-validation stages in the inspected path. The content conflict check is good, but is not a recovery transaction. [Write path](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src-tauri/src/hyprland_lua_backend.rs#L1358)

Display application changes the real compositor first and starts a ten-second frontend timer afterwards. Closing or unmounting the page clears the timer without reverting. A failure midway through a multi-monitor apply can return before the recovery timer is established. Recovery therefore does not survive the frontend closing or crashing. [Display flow](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src/pages/Displays.tsx#L150), [apply loop and timer](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src/pages/Displays.tsx#L251)

The frontend renders the current page rather than all pages, and no continuous option polling was found. It still uses a webview with React; this is not a fully native GTK application despite its Rust backend. No memory numbers were measured. [Page selection](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/src/App.tsx#L31), [dependencies](https://github.com/spinualexandru/hyprconfig/blob/6bae742ca2a1598ec4f66c6edf9334050f96f3ff/package.json)

Verdict: the best Lua editing reference of the three, but insufficient settings coverage and recovery for a direct adoption.

## Verification and limits

Completed:

- Downloaded and inspected the three pinned repository revisions.
- Compared unique literal scalar names against Hyprland 0.56.2's `getConfigValues()` registry, including built-in layout values.
- Checked the HyprSettings extra catalogue for the missing scalar names.
- Ran its actual parser on temporary comment, empty-group and nested-source fixtures. Comments survived ordinary reconstruction; formatting changed; nested relative source discovery failed.
- Traced application writes, reloads, preview-related code, monitor recovery and resource-related architecture.
- Confirmed all repository working trees were unchanged before removing the downloads.

Not completed:

- Rust parser tests: the installed compiler failed before compilation with a missing LLVM/C++ symbol while loading `librustc_driver`. No Rust test result is claimed.
- Full application builds, live desktop integration, GPU/driver behavior, screen-lock behavior and memory/CPU benchmarks.
- Exhaustive validation of every graphical control. Scalar counts are name inventories, not a certified coverage percentage.

## Changes to the proposed product plan

1. Establish the complete catalogue before designing individual pages. For 0.56.2 the main source registry provides 353 scalar keys; structured configuration and plugins require separate inventories.
2. Read active values from compositor APIs wherever possible. Keep unresolved source expressions and unknown runtime state visibly distinct.
3. Maintain a version/commit-specific schema with automated differences for additions, removals, type changes and default changes. Bundle it for offline use.
4. Use native controls generated from the schema plus specialized editors for rules, monitors, shortcuts and curves. Track readable/editable/savable/previewable capability separately.
5. Stage every change in a draft. Run actual compositor previews in an isolated, disposable environment; none of the audited applications supplies this subsystem.
6. Use atomic writes, preserved source spans, conflict checks, backups and a recovery journal. Put any live-display recovery timer in an independent temporary helper.
7. Benchmark total proportional memory across the editor and child processes. Measure preview/GPU resources separately; language choice is not evidence of memory use.

## Missing HyprSettings scalar catalogue names against 0.56.2

The following 29 keys were absent from the bundled base catalogue and its extra scalar definitions:

```text
debug:invalidate_fp16
debug:vfr
decoration:glow:color
decoration:glow:color_inactive
decoration:glow:enabled
decoration:glow:range
decoration:glow:render_power
decoration:motion_blur:enabled
decoration:motion_blur:samples
experimental:wp_cm_1_2
gestures:scrolling:move_snap_cursor
gestures:scrolling:move_snap_to_grid
group:groupbar:disable_when_only
group:groupbar:middle_click_close
input-capture:capture_modifiers
input-capture:enforce_barriers
input:follow_mouse_shrink
input:tablettool:eraser_button_mode
input:tablettool:eraser_button_override
input:tablettool:pressure_range_max
input:tablettool:pressure_range_min
master:center_master_fallback
master:focus_master_on_close
misc:initial_workspace_token_timeout
misc:session_lock_blur
render:fp16_sdr_tf
render:keep_unmodified_copy
render:non_shader_cm_interop
render:use_fp16
```

Reference: [Hyprland 0.56.2 value registry](https://github.com/hyprwm/Hyprland/blob/v0.56.2/src/config/values/ConfigValues.cpp#L159). These are scalar configuration names, not a list of every Hyprland feature or configuration structure.
