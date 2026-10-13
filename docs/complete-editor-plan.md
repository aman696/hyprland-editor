# Complete graphical Hyprland editor: audit and implementation plan

Audit date: 12 October 2026. This is a design and source audit, not a completed application or a security certification.

Implementation detail is now specified in the [engineering blueprint](engineering-blueprint.md), dated 13 October 2026. It supersedes architecture-level ambiguity here: Save means an independent draft, while publishing to active files is an apply operation because file watchers can reload them. Restart-pending changes stay staged until a reviewed lifecycle installation.

The proposed product is a native desktop application that discovers an existing Hyprland setup, exposes every supported configuration feature graphically, creates an independent draft, previews that draft in an isolated test environment, and applies selected changes through a recoverable transaction. Completeness is a release requirement with an item-by-item checklist.

## Evidence and scope

I collected all 100 pages linked by the archived 0.56 wiki and inventoried 121 Markdown documents from the current official wiki repository. I cross-checked the configuration registry, structured parsers, API registrations, rules, animation tree, IPC, Lua execution, config-path selection, and security policy against Hyprland 0.56.2 source. This is the installed binary's version; the archived documentation is 0.56.0, so it is release-family guidance rather than exact source authority.

The baseline source revision is [efb50993780079460b0cbed1363e2166a2de1d9f](https://github.com/hyprwm/Hyprland/tree/efb50993780079460b0cbed1363e2166a2de1d9f). The current wiki revision is [fde84d41b9a7154a5d211cfb1de50d5910791231](https://github.com/hyprwm/hyprland-wiki/tree/fde84d41b9a7154a5d211cfb1de50d5910791231). The [version selector](https://wiki.hypr.land/version-selector/) distinguishes released documentation from latest git.

I also downloaded and compared current Hyprland source at [6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7](https://github.com/hyprwm/Hyprland/tree/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7). This snapshot has **402 scalar keys**, including 50 additions and one removal since the installed baseline; 17 common keys have changed type/default/metadata. It has 100 registered Lua functions, 60 window-rule effects and 34 predefined Lua events. Both snapshots' complete scalar lists and structured descriptor lists are included in the inventory. These counts do not certify every newer parser behavior.

The current snapshot adds acrylic, aurora, drops, fluid-jar, glass, haze, heat-shimmer, ripple and water blur controls; wobble physics; additional dragging, cursor-warp, tablet, bell, floating-window and renderer behavior. Its extra window effects are no_glow, no_wobble and no_xdg_drags; extra API functions are hl.get_devices and hl.dsp.reload_config; extra events are config.unload, window.bell and window.minimize. The current source accepts damping with dampening as a compatibility alias, and its config manager is Lua-only. This corrects any blanket assumption from the earlier audit that 0.56.2 itself already required Lua-only configuration. See the pinned [current path selector](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/supplementary/jeremy/Jeremy.cpp) and [current config manager](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/ConfigManager.cpp).

The companion [feature inventory](hyprland-feature-inventory.md) lists every discovered baseline scalar key, structured field, rule effect, public animation node, Lua function, event, and inferred runtime object field. Its [JSON form](hyprland-feature-inventory.json) retains source references, defaults as source expressions, bounds, planned controls, documentation headings/table entries, and revision hashes. It is an audit input; constructor expressions and heuristic signatures still need a production schema exporter and parser acceptance tests.

The inventory also includes all 21 baseline registered legacy directives, 71 legacy dispatcher names, 44 current documented socket2 event names, and 756 current core documentation table entries. Suffix-enabled directives, aliases and action argument forms are recorded separately from scalar settings.

| Surface in 0.56.2 | Discovered entries | Required treatment |
|---|---:|---|
| Scalar config keys, including built-in layout options | 353 | Every key gets a typed graphical control |
| Monitor descriptor fields | 22 | Plus required output selector |
| Per-device descriptor fields | 43 | Plus required device name |
| Workspace descriptor fields | 15 | Plus workspace, enabled, layout options |
| Window-rule effects | 57 | Plus name, enabled, 17 match properties |
| Layer-rule effects | 10 | Plus name, enabled, namespace match |
| Registered Lua functions | 98 | Includes 51 dispatcher constructors; does not include object methods |
| Predefined Lua events | 31 | Plus custom events |
| Public animation tree nodes | 34 | Excludes one internal implementation node |
| Runtime object classes found by the upstream generator | 12 | Inspection, handle lifecycle, and supported actions |
| Current ecosystem documentation table entries | 214 | Mixed settings, widget fields, and switches; independently versioned |

Do not add these counts together into a misleading percentage. Rule instances, bindings, profiles and callbacks have arbitrary multiplicity. Plugin-defined features and third-party programs have no fixed universal count.

The current documentation register includes configuration, selectors, layouts, IPC, permissions, plugins, ecosystem tools, Nix, GPU guidance, startup, diagnostics and utilities. Configuration and safety pages received focused semantic/source checks. Index/community/reference pages were triaged; linked third-party manuals are separate work. The plan does not claim a full source-security audit of every ecosystem application.

## What “each and every feature” must mean

There are three independently tracked commitments:

1. **Complete compositor support for named releases.** All built-in settings, structured configuration, actions, public API capabilities and relevant startup features for each supported release have a graphical representation and meaningful validation. No absent-from-file setting is hidden.
2. **Complete supported-adapter support.** Every setting of an explicitly supported version of hyprlock, hypridle, hyprpaper and other adopted components has a graphical editor. Installed unsupported components remain visible with a precise support status.
3. **Extensible programming support.** Visual workflows, expressions and layout builders represent public Lua behavior. Plugin authors supply versioned descriptions or dedicated adapters for custom features.

A universal automatic “simple sliders for every possible Lua program, native plugin and external app” is not feasible. Lua programs can compute settings from files, networks and arbitrary algorithms; plugins can invent new syntax and behavior. A full visual Lua programming system can express more of this, but it becomes programming with blocks. We should support that honestly rather than count a raw text box as a graphical implementation.

For configurations we cannot safely interpret, preserve the original source and identify which results are unknown or dynamically produced. Offer a graphical override where its precedence is well-defined, or an explicit guided conversion to managed workflows. Do not pretend to reconstruct arbitrary callback closures from their current effects.

Each inventory row needs separate statuses: visible, saved-state readable, active-state readable, graphically editable, validated, saveable, previewable, applicable now, restart required, and supported version. A feature is not complete just because its name appears in search.

## Important audit findings that change the design

| Finding | Evidence | Consequence |
|---|---|---|
| Default configuration is not exhaustive | [Official core documentation](https://wiki.hypr.land/configuring/core/) | Generate the inventory from registries and handlers, never from the example config |
| Current docs can disagree with a released parser | 0.56.2 [curve and animation handlers](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L337) | Match schemas to commit/version and test generated configuration against that parser |
| 0.56.2 uses spring field dampening; current docs use damping | Same handler and [animation documentation](https://wiki.hypr.land/configuring/core/animations/) | A version adapter maps the friendly control to the correct syntax |
| Spring parser rejects values at or below 0.5 although its error says at least 0.5 | Same source handler | Bounds need behavioral tests, not scraped error strings |
| Enabled animations require speed and bezier or spring; the docs' curve shorthand is not accepted by this handler | [Animation handler](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L390) | Generate the accepted form; preserve disabled-node and inheritance semantics |
| Lua and legacy .conf are both supported in this baseline | [Config path selection](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/supplementary/jeremy/Jeremy.cpp#L19) | Support both parsers; do not force conversion or create a higher-priority Lua file accidentally |
| Upstream generated Lua stubs omit window-rule effects and accepted auto_consuming binding flag | [Stub generator](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/meta/generateLuaStubs.py), [rule descriptors](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsInternal.hpp#L49), [binding handler](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L188) | Use stubs for discovery; descriptor arrays and actual argument parsers remain authoritative |
| A descriptions request returns registry metadata and current/default values | [IPC handler](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/debug/HyprCtl.cpp#L1979), [registry serialization](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/values/ConfigValues.cpp) | Use a snapshot instead of hundreds of individual getoption requests |
| Lua opens standard libraries | [Lua initialization](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/ConfigManager.cpp#L520) | Reading a config must not execute it; validation also needs isolation |
| Permission enforcement is disabled by default; configured permissions require restart | [Official permissions documentation](https://wiki.hypr.land/configuring/core/advanced-configuration/permissions/) | Show enforcement state and pending changes; never imply save/reload activates these rules |
| Native plugins execute with compositor authority | [Official plugin documentation](https://wiki.hypr.land/hyprland-plugins/using-plugins/) | Plugin download/build/load are distinct trust decisions |
| Compositor IPC requests are synchronous | [Official IPC documentation](https://wiki.hypr.land/ipc/) | Short-lived requests, timeouts, one event subscription, bounded refresh |

The earlier alternative-editor audit found useful pieces but no examined project met this entire contract. HyprViz has broad native controls but incomplete baseline coverage; HyprSettings has generated coverage gaps and a network-exposed write-service design; hyprconfig has promising Lua source preservation but major unfinished editors and live display testing. See the [previous audit](prior-art-audit.md) for evidence and test limits. This is a finding about those audited revisions, not proof that no other project exists anywhere.

## Complete functional scope

### All scalar settings

All 353 source keys are listed individually in the inventory. The category counts are general 23; decoration 43; animations 2; input 60; gestures 14; group 47; misc 40; binds 14; xwayland 4; opengl 1; render 17; cursor 22; ecosystem 3; debug 22; layout 2; dwindle 11; master 14; scrolling 9; experimental 1; input-capture 2; quirks 2.

The UI must cover gaps, borders and gradients, focus/snapping, opacity/dimming, blur/shadows/glow/motion blur, grouping/groupbar fonts and behavior, keyboard/mouse/touchpad/touch/tablet configuration, cursor behavior, layout defaults, rendering/color management/VRR/direct scanout/tearing, XWayland compatibility, miscellaneous session behavior, diagnostic options, permissions enablement and experimental controls. New current-doc features such as wobble and expanded blur variants are candidates for newer schemas, not silently enabled on 0.56.2.

Use switches, mapped-choice selectors, bounded integer/decimal entries, paired vectors, four-sided gap editors, gradient editors and font-weight controls. Strings that contain structured syntax need builders: acceleration curves, scroll points, selectors, shader/file paths and style arguments. A text field is acceptable for an actual name or free-form label, not for a hidden mini-language that the product promises to make graphical.

Every setting shows default, explicit versus inherited/unset state, saved value, active value when known, pending value, source file/location, restrictions, dependencies, preview fidelity and activation timing. Numeric zero, false, empty string and unset are different. Preserve accepted special values and unknown future values; never silently clamp an imported value.

### Monitors and output management

Provide a drag-and-align output map with logical coordinates, scaling, fractional-scale effects, transforms, mirrored-output relationships, disabled outputs, fallback rules and reserved areas. Include modes/refresh/custom modes, output selection by connector/description, position auto rules, bit depth, VRR, color presets, SDR transfer/brightness/saturation, ICC and HDR/luminance overrides.

Use output capabilities and EDID as evidence, not certainty. Validate connector identifiers, mode availability, mirror cycles, unsupported combinations and layouts that leave no usable display. Keep unplugged-output rules editable. Profile matching should use stable descriptions plus user-controlled fallbacks rather than assuming connector names travel between machines.

A visual sample can demonstrate relative positions and tone mapping. It cannot certify a monitor's HDR, actual refresh, VRR, tearing, ICC accuracy, GPU compatibility or successful KMS modeset. These require a separate guarded hardware test.

### Devices and input

Expose all 43 accepted per-device fields separately from the global input defaults. Discover keyboard, pointer, touchpad, touch, tablet/tool and switch capabilities; display virtual/physical origin when known. Support tags, inclusive/exclusive binding device lists, keyboard layouts/variants/options/rules/models/files, repeat and numlock, left-handed/natural scrolling, acceleration/custom points, touchpad gestures, tap/drag/click methods, tablet regions/output mapping/transforms/rotation and enablement.

Use a local XKB-backed picker and test panel for layout preview. Shortcut recording is active only while the recorder has focus and is explicitly armed. Device settings must not disable the user's last usable input path without guarded recovery.

### Workspaces, layout options and selectors

Provide named/numeric/special/persistent/default workspaces, monitor associations, per-workspace gaps/borders/decoration/shadows, empty-workspace actions, names, animations, layout and layout_opts. Build selectors with forms for relative/absolute IDs, previous/per-monitor navigation, ranges, names, monitor filters, window-count flags, fullscreen predicates and empty-workspace queries.

Window selectors need class/title/initial variants, tags, PID/address/stable identifiers and active/floating/tiled choices where accepted by that version. Monitor selectors need connector/description/ID/object/current/direction forms. A temporary runtime address must not become a supposedly portable saved selector.

Include all built-in dwindle, master, scrolling and monocle behavior; every source-supported layout message; and per-workspace options. Explain inheritance and which selectors only match already-existing workspaces. Custom Lua layouts require a visual geometry builder with grids/rows/columns/splits, ratios, gap-aware placement, optional manual boxes, target iteration and message handlers.

### Window and layer rules

Implement complete create/read/edit/delete/duplicate/enable workflows, all 57 window effects, all 10 layer effects and every public match property. The inventory enumerates the fields. Build regex/negation, boolean state, tag and workspace predicates graphically, with literal escaping and a match tester.

Rules must distinguish initial/static effects from dynamic effects and reflect Hyprland's named/anonymous ordering and actual precedence. Include a “why this applies” view with matching predicates, effects and overrides. A simulator is only reliable for the implemented version and known state; callbacks can change the result later.

Specialized effect editors are needed for placement/size expressions, opacity modes, grouping, suppressed events, content hints, idle inhibition, fullscreen internal/client state, tags, scrolling width, gradients, tonemapping and capture exclusions. For layers include namespace match, blur/popups, alpha threshold, xray, dimming, animation/order, above-lock and no-screen-share.

Do not simply append another rule or flatten source files. A duplicate or differently ordered rule can change semantics. Security-sensitive above-lock/input/capture effects need contextual explanations.

### Keybindings, submaps, switches and gestures

Provide a searchable action library for all 51 Lua dispatcher constructors and the legacy dispatcher catalogue, with typed parameters. Include action sequences, conditions and callback workflows. Cover keysyms/keycodes, modifier combinations, multiple-key conventions supported by the parser, mouse buttons/wheel, media keys, switches, descriptions and device/tag targeting.

All accepted flags must be represented: repeating, locked, release, non_consuming, auto_consuming, transparent, ignore_mods, dont_inhibit, long_press, submap_universal, click, drag and allow_input_capture. Description and desc are aliases, not two independent settings. Legacy bind flag combinations must preserve their accepted semantics.

Submaps need nesting, transitions, reset/escape, catchall, universal bindings and automatic follow-up submaps where supported. Detect conflicts using the actual key identity, modifier/device/submap/flag conditions; show legitimate ordered multi-action bindings rather than declaring every duplicate an error.

Gesture editors need 2–9 fingers, swipe/pinch/directional forms, modifiers, scale, inhibitor bypass, all actions and their mode/zoom/workspace arguments, unset identity matching, ordinary callbacks and live start/update/finish callbacks. The simulator uses synthetic gesture payloads; physical gesture reliability remains a hardware test.

### Animations and curves

Display the full 34-node inheritance tree, override/reset, enablement, duration, curve selection and per-node style validation. Include slide/popin/gnomed/fade and workspace directional/fade combinations, percentages, forced sides and once/loop styles where accepted.

Provide draggable cubic Bézier points and spring mass/stiffness/damping controls, including response graphs and exact release syntax. An angle-loop animation can cause continuous rendering even when decorations are not visible; show the expected battery consequence without forcing a particular aesthetic.

### Programmable behavior, lifecycle and notifications

Expose every public Lua function from the inventory as a suitable editor, action or inspector. A visual workflow engine needs typed values, variables/references, arithmetic and comparisons, if/else, bounded iteration, sequence, query nodes, selectors, optional values, custom events, reusable functions and lifecycle handles.

Support all 31 predefined events, custom events, subscription removal, keybind/rule enablement, repeating/one-shot timers and timer controls. Show callback lifecycle, reloading and potential event loops. Preserve the difference between a scheduled property refresh and an immediate refresh; avoid repeated forced refreshes.

Provide graphical startup/shutdown/reload workflows, environment entries, command/app selection, argument handling, notifications with styling/timeouts and object actions such as pause/resume/dismiss. Imported shell commands are displayed for review, never executed during discovery or ordinary preview.

The advanced visual programming view can represent all public Hyprland API calls. Arbitrary Lua standard-library behavior and custom modules require additional nodes and trust policies. Unsupported code is preserved with a visible coverage gap; it cannot count toward the “no typing” claim until its behavior has a graphical representation.

### Runtime inspection and troubleshooting

Show selected instance/version/commit, monitors and capabilities, windows/workspaces/groups/layers, devices, bindings/global shortcuts, current submap, animations, loaded plugins, lock/capture state, config errors, relevant status and read-only diagnostics. Separate object fields from actual configurable values.

Do not inject a plugin or evaluate arbitrary Lua just to inspect a session. Public IPC lacks complete introspection of every callback closure, imported module and custom plugin state. Use safe documented queries, owned named handles where appropriate, and explicit unknowns. Upstream structured introspection for rules/callbacks would materially improve detection and should be proposed separately.

### Startup, environments, ownership and platform setup

Discover explicit config paths, HYPRLAND_CONFIG, XDG lookup, Lua/conf precedence and safe-mode recovery paths. Inspect session launch arguments, start-hyprland/UWSM/systemd ownership and duplicate autostarts. Include restart-only backend/GPU variables, cursor environment, toolkit backend/theme/scaling choices, trace/debug variables and session activation-environment ownership.

Launch options such as custom socket, inherited Wayland/watchdog descriptors, safe mode and locked startup are part of the startup model. Internal handover descriptors are not random editable numbers. The root-check bypass must never become a recommended setup toggle.

Cover AMD/Intel/NVIDIA, multi-GPU, virtual GPU, tearing, XWayland, missing portals/polkit and diagnostic guidance as capability checks. Driver installation, kernel/module, PAM, udev, login-manager and system configuration are separate privileged operations with narrow helpers or exportable instructions.

NixOS/Home Manager/Hjem and symlink-managed dotfiles need ownership detection. Do not overwrite generated targets, immutable store files or a configuration manager's output. Provide structured export/adapters to the owning source and show unresolved dynamic Nix expressions. An ordinary file GUI cannot automatically understand every arbitrary Nix program either.

### Ricing beyond the compositor

These are separate adapters with their own schemas, versions, config ownership, runtime control, risk and preview support.

| Component | Required graphical scope | Preview/security boundary |
|---|---|---|
| hyprpaper | Per-output wallpaper sources, modes, preload/cache behavior where supported, directories/slideshows and daemon control | Private outputs; bounded image decoding; no host wallpaper daemon control |
| hyprlock | Every general/auth option, animation tree, background/image/shape/label/input-field widget, shared shadow/click fields, positions/percentages/alignment/z-order, gradients, variables and dynamic content | Fake credentials and synthetic background; no host session lock or real authentication attempts |
| hypridle | General lock/unlock/sleep hooks, inhibition modes, all listener timeouts/resume/conditions | Virtual clock and mock actions; no actual suspend, shutdown, lock or DPMS |
| hyprsunset | All profiles, time, temperature, gamma, identity, max gamma, on-switch commands and supported IPC | Approximate visual sample; actual display color changes need explicit hardware test |
| hyprcursor | Installed theme metadata/assets, fallbacks, size and toolkit limitations | Synthetic pointer preview; no silent asset installers |
| hyprqt6engine | Every documented color/style/icon/font and Qt theme selection field | Separate sample Qt client; versioned adapter |
| hyprtoolkit | All theme colors, fonts, sizing/rounding and toolkit-wide appearance settings | Separate sample client; library version matching |
| XDPH | All screencopy options, restore-token default, picker, shared-memory/cursor modes, portal routing/global shortcuts | Private bus/services; never capture host screen in ordinary preview |
| hyprlauncher | All available behavior/finder, output, history/cache and daemon options | Synthetic catalogue/history; no executing selected real apps |
| hyprpicker | Supported capture/color-format behavior and integration | Explicit capture action only; synthetic sampler otherwise |
| hyprshutdown | Session-exit integration and supported options | Mock exit/progress; no live shutdown during preview |
| hyprpolkitagent, Qt support, system-info and PipeWire tools | Relevant integration, status, preferences and explicit launch controls | Privilege requests and real audio/network actions are separate operations |
| Waybar/Quickshell/AGS/Eww | Versioned bar/widget/panel schemas, modules, placement, styling and workflows | Separate trusted samples; imported scripts treated as code |
| Launchers, notifications, GTK/Qt/icons/fonts, terminals, palette generators | Adapters for selected installed products with complete per-version field inventories | Their settings are not Hyprland scalar keys; no universal unsupported-app claim |

The 214 official ecosystem table entries are in the inventory, but docs are not complete source schemas for every independently released tool. Before claiming an adapter complete, audit that component's exact source and CLI. Third-party manuals mentioned by the wiki were identified as integration candidates, not silently treated as audited.

## Architecture and data flow

Use a native Rust core with GTK4/libadwaita as the initial implementation choice. This fits Linux/Wayland, gives mature accessibility and virtualized lists, and avoids embedding a browser runtime. It is a hypothesis about efficiency, not a measured RAM result. Confirm it in an early benchmark; Qt or Hyprtoolkit remain viable if measured constraints warrant a change.

Keep the schema/model, source parsers, discovery, validation, preview supervision and apply engine independent of widgets. The editor normally has one process. Optional preview workers and a temporary recovery helper exist only when needed. No continuously resident editor service.

~~~mermaid
flowchart LR
    Files[Saved files and ownership] --> Discover[Read-only discovery]
    IPC[Chosen live instance] --> Discover
    Schema[Versioned schema and adapters] --> Model[Typed draft and provenance]
    Discover --> Model
    Model --> GUI[Graphical editors and inspectors]
    Model --> Safe[Safe preview projection]
    Safe --> Lab[Isolated compositor and sample clients]
    Model --> Validate[Static and isolated validation]
    Validate --> Review[Change and effect review]
    Review --> Apply[Journaled file transaction and targeted apply]
    Apply --> Verify[Read back and independent recovery]
~~~

### Schema generation and maintenance

The authoritative order is installed-release parser behavior, source registrations/descriptors, runtime metadata, then matched documentation for explanation. Documentation parsing alone cannot detect defaults, aliases, required fields and parser quirks reliably.

At build/release time, export the complete scalar registry and structured/API argument definitions into normalized data. Prefer a machine-readable upstream export or compiler-assisted extraction over fragile regex inference. Keep intentional handwritten editors for expression languages and dependent structures. Preserve types, bounds, maps, special values, refresh requirements, argument constraints, aliases and deprecations.

Each schema row records stable ID, version range/commit, scope, source/docs reference, type/default, units, widget, dependencies, runtime getter, serializer, mutation/apply method, reload/restart requirement, effect/risk, preview fidelity and verification test. Runtime descriptions enrich the selected-release schema; they do not authorize executing imported data.

For unknown/newer builds, display discovered metadata and preserve unknown source. Do not guess an older serializer works. Pin signed application/schema releases; review new schema entries and migrations. CI must fail when a registered feature disappears from the inventory without an explicit disposition.

### Discovery: saved, active, effective and draft

Maintain four distinct states: saved source declarations; observed active compositor state; effective per-window/device/workspace state when it can be determined; and the proposed draft. Track whether an observation is direct, inferred, stale or unavailable.

Select an instance explicitly by stable signature, UID and process identity. Verify socket ownership and process start time; multiple live/preview instances must never be confused. On compositor restart, reconnect and rediscover rather than reusing old handles.

Read scalar metadata in one snapshot where supported, then independently query required monitors/devices/binds/rules/status with a whitelist. Subscribe to one event socket, coalesce events, resnapshot after reconnect or event loss, and refresh only the relevant views. Events are notifications, not a complete transaction log.

A config change made by a hotkey, script, another editor or runtime command may differ from disk and may not have a complete event trail. Show that difference; offer explicit save/restore choices. Unknown callback results remain unknown. Exporting active scalars alone would lose startup logic and rule semantics.

### Source preservation

Use syntax trees with concrete source spans for Lua and legacy Hyprlang. Preserve comments, formatting, order, strings, relative-path semantics, variable references, conditionals and includes/modules. Resolve legacy includes relative to the relevant file; follow Lua require behavior for the chosen version. Detect cycles, bounded wildcard expansion and missing dependencies.

Recognize literal declarations safely without running them. Expressions need a dependency/provenance graph. A derived setting edit can update its shared variable or create a scoped override after explaining affected consumers. Never flatten the user's include graph simply to simplify writing.

For owned GUI configuration, maintain a clearly identified managed module and stable names/handles. Reload behavior must be idempotent: no duplicated binds, subscriptions, timers or startup processes. Named rules/modules need lifecycle cleanup and a versioned migration path. An override is not valid if later user code wins; detect that or mark it unresolved.

Offline editing, missing compositor, partial imports, multiple config profiles, comparison, undo/redo, reset by field/category, import/export with version/dependency manifests and portable path/output mapping are required. Search must find unset settings and aliases. Provide keyboard navigation, screen-reader labels, high contrast, translation and scalable layouts.

## Preview without live configuration consequences

There must be an enforceable boundary: ordinary preview cannot change host config, host compositor state, services, permissions, input configuration, real wallpaper/locker or run imported host commands. Preview still consumes CPU/RAM and possibly shared GPU resources; “zero consequences” cannot literally mean zero resource impact.

Provide three explicitly labeled levels:

1. **Fast visual sample.** An in-app scene for colors, spacing, geometry, gradients and timing. Very cheap and approximate. It does not prove compositor behavior.
2. **Actual compositor lab.** A separately supervised Hyprland of the matching version, synthetic windows/layers/devices, private config/state/cache/runtime directories and private services. This verifies many visual/rule/layout/interaction behaviors.
3. **Guarded hardware test.** A deliberate live action with independent recovery for physical output/input/startup effects that the lab cannot reproduce. It is not ordinary preview.

Build the lab from trusted schema-generated declarations and asset copies. Omit imported autostart, shutdown, shell commands, callbacks, module loads and native plugins by default; display the resulting preview omissions. Safe workflow nodes can execute against mocks or allowed private targets. An optional fuller untrusted-code lab needs stronger isolation, preferably a VM for native plugins, and a clear separate mode.

A nested compositor is not a sandbox: giving it the host Wayland socket, session bus, runtime directory or home directory gives it host capabilities. Prototype a headless/private-output compositor with a narrowly defined frame/input bridge. The compositor must not receive the host display socket. The trusted bridge displays only preview frames and forwards only input directed to the preview. If a restricted nested proxy is used, audit its allowed protocols explicitly.

The lab needs filesystem/process/network namespaces, private runtime and D-Bus, an environment allowlist, no inherited Hyprland signatures or host IPC, no credentials/agent sockets, no host /dev/input or DRM modesetting devices, and bounded CPU/memory/PIDs/output. Optional render-node access shares a kernel/GPU attack and exhaustion surface; it is not perfect isolation. Software rendering or VM isolation may cost more and may be unsupported on a particular build.

First prove headless startup, rendering, capture/bridge, sample clients and teardown on the supported backend/GPU combinations. The source has a headless backend, but that does not prove this complete constrained launch works. If isolation cannot be established, disable the compositor lab with a reason; never fall back to live mutations.

All validation that executes Lua or a compositor, including verify-config, belongs in this same trust boundary. The Lua instruction watchdog is not a sandbox or a comprehensive defense against blocking I/O/native code.

Preview scenarios should include active/inactive/floating/fullscreen/pinned/grouped/urgent windows, opening/closing/resizing, all layouts, special workspaces, layer panels/popups, multi-output scale/rotation, animations, binding/submap transitions, gestures, idle schedules and mock lock/auth states. Let users compare before/after with a reproducible scenario. Never copy live window contents or passwords into those scenes automatically.

Destroy all preview process groups, sockets, buses, textures and temporary assets on close/crash/timeout. Use a supervisor so GUI termination does not leave preview daemons running.

## Apply, persistence and recovery

Editing only changes the draft. Separate Save, Apply now and Save for restart by actual activation behavior. A reviewed transaction lists changed files and host effects, commands/plugins/services involved, unavailable preview checks and recovery limits.

Before mutation, verify the selected instance and source file hashes, validate syntax/semantics and prepare restricted backups and a durable transaction journal. Write a temporary file in the same filesystem, retain intended metadata, sync it, atomically rename and sync its directory. Multi-file changes are journaled and recoverable; several renames are not one globally atomic transaction.

Resolve intended symlink ownership and protect path operations against races. Use directory-relative handles and no-follow/revalidation where appropriate. Do not replace a symlink or edit its target without knowing the configuration manager's ownership. Recovery must not overwrite somebody else's newer edits.

Order live changes by dependencies and keep permission/startup-only changes pending. Query replies and active state after apply; successful IPC transport is not proof of the desired result. A batch request is not necessarily atomic. Failures produce a clear partial-state report and recovery options.

For risky monitor/input changes, start a temporary independent recovery helper before the first mutation. It holds a snapshot and a deadline; the user explicitly keeps the result or it restores recoverable state. Test GUI crash, helper crash, compositor restart, logout, hot-unplug and interrupted disk writes. The helper is pinned to the original instance and must not mutate a replacement session.

Rollback can restore owned configuration and many settings. It cannot revive killed windows, undo arbitrary shell programs, reverse all plugin effects or guarantee a crashed compositor restarts. Commands, plugin load/unload, session exit, suspend and lock are separately classified actions, not promised reversible transactions.

Keep a usable emergency escape path and exportable recovery instructions for TTY use. Do not silently install a new session manager, root service or broad privilege mechanism to support recovery.

## Security requirements and acceptance tests

The trust model covers downloaded themes/profiles, Lua/modules, plugin source/binaries, malformed IPC, file races, malicious assets, local network clients and accidental destructive edits. The native editor has powerful same-user authority; it is not protection against a completely compromised user account. A preview boundary can still protect against untrusted imported data if correctly implemented.

| Threat or failure | Required control | Release acceptance evidence |
|---|---|---|
| Config executes commands on open | Static parsing; never evaluate for discovery | A fixture with file writes/process execution produces no side effects |
| Verification executes untrusted Lua | Isolated validator; private environment/resources | Fixtures cannot reach host files, session sockets, credentials or network |
| Preview reaches live compositor | No host sockets/signature/bus; narrow bridge | Attempts to dispatch host commands fail and live state/config hashes stay unchanged |
| Wrong Hyprland instance | Verify UID, signature, socket owner, PID start time; fail closed | Two instances plus restart cannot redirect a pending mutation |
| Plugin arbitrary code | Pin source/ABI; sandbox build; separate explicit load | Import/build never loads into live compositor; incompatible binary is blocked |
| Broad HTTP write service | Prefer no HTTP server; narrow local IPC with peer credentials | No TCP listener; unauthorized local requests cannot write/apply |
| Shell/Lua/config injection | Typed AST serialization; separate argv and explicit shell mode | Quotes/newlines/delimiters/path payloads stay data; generated programs parse as intended |
| IPC malformed/large responses | Length/time limits, resilient parsing, bounded queues | Fuzzing/event floods cause no crash or unbounded memory |
| Synchronous IPC stalls | Request immediately, close promptly, timeout/cancel; no idle request socket | Disconnect and timeout tests leave the compositor responsive |
| File race/symlink substitution | Ownership-aware directory handles and hash/inode checks | Race tests cannot write outside intended targets |
| Lost user edits | Recheck source hashes; conflict-aware merge/rebase | External edit during apply is preserved or produces a conflict |
| Disk failure/partial multi-file write | Atomic per-file replacement plus durable journal | Fault injection at each step yields identifiable, recoverable state |
| Secrets in backups/exports | Restricted modes, redaction, bounded retention and explicit asset selection | Private variables/paths are absent from diagnostics and unintended exports |
| Keylogging through recording | Focused, armed shortcut recorder; minimal retention | Losing focus/stopping recorder stops capture and clears transient state |
| Host screen or password disclosure | Synthetic scenes; explicit host capture only in separate tools | Ordinary preview captures no host surfaces; auth preview never uses real secrets |
| Capture permissions misleading | Show enforcement and restart-pending state; validate narrow rules | Saved pending permissions are not reported active |
| Lockscreen/input bypass | Context for locked binds, inhibitor bypass, above-lock and input capture | Security-sensitive combinations flagged; no live test on ordinary preview |
| Idle/lock-before-suspend race | Validate workflow ordering; verify actual lock-ready events where supported | Mock timeline and explicit hardware test detect failure to lock before sleep |
| Last screen/input lost | Capability checks and independent timed recovery | GUI termination during trial restores usable state when compositor remains recoverable |
| Archive traversal/bombs | Reject escaping paths/unsafe symlinks; bound expansion/count | Malicious archives cannot escape staging or exhaust allowed resources |
| Image/font/ICC/shader parser exploitation | Supported decoders, worker isolation, size limits, trusted shader policy | Fuzz/resource tests; shader hangs time out or lab terminates |
| Preview CPU/process/memory exhaustion | Resource caps, bounded callbacks and supervisor cleanup | Infinite loop/fork/large allocation fixtures are terminated and cleaned up |
| Update/schema supply-chain change | Signed/pinned packages, reviewable migrations, provenance and compatibility | Tampered or incompatible schema/update rejected |
| Excess privilege | No normal root operation; narrow explicit helper contracts only if needed | Ordinary editing runs without elevated privileges |
| Unwanted background work | No telemetry by default; no auto installs/downloads or imported hooks | Idle/offline operation performs no undisclosed network/process activity |

Permission editing includes screencopy, plugin, keyboard, cursorpos and input-capture with allow/ask/deny, target patterns, ordering and enforcement status. Portal settings and sandbox access to IPC sockets are separate layers. A permissive plugin rule for a general-purpose control binary can grant arbitrary plugin loading; show the consequence and offer narrow targets. The [permission documentation](https://wiki.hypr.land/configuring/core/advanced-configuration/permissions/) explicitly notes that Hyprland cannot itself block application access to its IPC sockets.

Diagnostics must minimize window titles, executable paths, usernames, monitor identifiers and environment values. Redact before showing an export preview; never silently upload. Keyboard event workflows need especially careful logging defaults. Keep clipboard content and authentication material out of persistent state.

A plugin catalogue is data, not trusted code. Building hyprpm manifests can execute build commands, and loading the result grants compositor authority. Do not assume an isolated build makes the resulting binary safe. UI schema adapters themselves should be declarative; executable adapter plugins require their own isolation/trust contract.

## Memory, CPU and responsiveness plan

“Linux” does not guarantee low memory use. Set budgets early, measure the entire process tree, and reject designs that fail them.

| Metric | Initial engineering target | Measurement conditions |
|---|---|---|
| Ordinary editor steady memory | At most 100 MiB total PSS, aiming lower | Preview off; release build; full schema and representative config loaded |
| Idle CPU | Under 0.1% of one core averaged over 60 seconds | Quiet compositor; static view; no preview/recording |
| Search/filter response | Under 50 ms for the baseline catalogue | Warm schema; no synchronous compositor round trips |
| Normal edit feedback | Under 100 ms | Widget/model work only; validation is cancellable background work |
| Asset thumbnail cache | Initially cap at 16 MiB | Downsampled, bounded LRU; full wallpaper not retained unnecessarily |
| Preview resources | Separate measured budget; configurable hard cap and low-resolution/FPS modes | Includes compositor, clients, bridge, textures and GPU allocations |

These are targets, not benchmark results. Hardware/reference datasets and percentile methods must be recorded before a release claim. GPU memory is not captured fully by ordinary process RSS; report it separately where tooling permits.

Use virtualized setting/rule/bind lists, category widgets created on demand, one shared typed model, bounded undo history, compressed or disk-backed checkpoints, incremental parsing/search, lazy asset loading and short-lived workers. Bound logs/event queues and coalesce floods; do not copy full state for every small event.

Use file watchers and compositor events, not fast polling. Refresh focused views on demand and after known changes. Parsing/validation are debounced and cancellable; do not execute a new compositor for every slider movement. No always-on thumbnail/animation render loop. Suspend preview when hidden and stop it when closed.

Benchmark cold/warm start, 353+ schema entries, large included configs, 10,000 bindings/rules, rapid search, repeated preview open/close, oversized assets, event storms, hotplug and offline mode on AMD/Intel/NVIDIA reference systems, including low-RAM hardware. Limits must fail visibly rather than silently dropping valid features.

Measure toolkit alternatives with the same model and tasks before committing deeply. A Rust label alone is not a memory strategy; duplicated web/native state, GPU caches and preview child processes can dominate.

## Implementation stages and completion gates

| Stage | Concrete deliverable | Gate before proceeding |
|---|---|---|
| 1. Release schema and coverage tooling | Exact baseline exporter, current/released diffs, scalar/structured/API inventories, capability matrix | No unexplained registration omissions; aliases/defaults/bounds accepted by parser |
| 2. Read-only discovery and preservation | Instance selection, saved/active comparison, Lua/conf source graph, ownership detection | No execution on import; round-trip/no-change files byte-identical; unknown behavior labeled |
| 3. Safe draft and transactions | Undo/profiles, source patches, conflict handling, journaled writes and recovery | Crash/race/disk-failure tests pass; targeted mutation cannot cross instances |
| 4. Low-memory native shell and all scalar editors | Search, provenance, typed controls, complete baseline key coverage | Every scalar has meaningful edit/reset/validate/save tests; measured performance budget |
| 5. Complete structured editors | Monitor/device/workspace/rule/bind/submap/gesture/curve editors | Every descriptor/effect/action variant covered; precedence and activation semantics tested |
| 6. Preview proof and supervisor | Safe sample plus actual isolated compositor bridge and scenario suite | Host-isolation tests pass; unsupported backend fails closed; teardown/resource caps verified |
| 7. Visual behavior/layout programming | API nodes, event/timer workflows, selectors/expressions, custom layouts | All supported public API argument forms represented; lifecycle/reload idempotence tested |
| 8. Ecosystem and configuration-manager adapters | Independently versioned Hypr tools, selected ricing apps, Nix ownership/export | Per-adapter source inventories and activation/preview/security checks complete |
| 9. Release hardening and ongoing schema maintenance | Packaging, accessibility, diagnostics, signed updates, compatibility matrix | Full coverage report plus security/performance/hardware results; no silent unsupported entries |

Stages can overlap only where their contracts are established. Preview feasibility and parser correctness should be prototyped early because they can invalidate architecture choices. Early builds are partial editors; they must not be marketed as complete.

The highest-value first engineering work is a source-backed schema exporter, a source-preserving parser, an instance-safe reader and a sandboxed preview feasibility prototype. Designing hundreds of polished controls before resolving these risks would create expensive rework.

## Required verification before “complete”

Release coverage must map every registry key, structured field, alias, dispatcher/action form, animation node/style, public event/object method and startup capability to a graphical representation and test. It must separately show full coverage for each adopted ecosystem adapter. New upstream registration diffs block a compatibility claim until resolved.

Parser fixtures cover both formats, includes/require paths, comments, escaped/multiline strings, nil/false/zero/unset, repeated declarations, variables, conditionals, defaults, unavailable files, symlinks, generated Nix paths, unsupported future values and unknown plugin syntax. Test semantic equivalence of edits, not merely whether an output file can be opened.

Behavior fixtures cover named/anonymous rule ordering, static/dynamic effects, selectors, synthetic clients/layers, keyboard identities/device targeting, sequences/submaps, gesture callbacks, all layouts/messages, curve bounds/styles, timers/events, scheduled refresh and reload cleanup. Validate generated output against each supported compositor parser inside isolation.

Security tests include all rows in the threat table, sandbox boundary probes, absence of host services/sockets/credentials, event/parser fuzzing, malicious themes/assets, helper failure, transaction crash recovery and process/resource cleanup. Hardware tests separately cover multi-output hotplug, mixed DPI, transformations, refresh/VRR/HDR where available, multi-GPU, XWayland and input loss/recovery.

Accessibility, keyboard-only operation, localization, corrupted-state recovery, offline workflows, distro packaging, missing optional tools and generated-config ownership also need acceptance checks. Store provenance and known limitations alongside results; do not claim a hardware guarantee from a mock preview.

## Outstanding decisions and limits of this audit

The plan commits to a native architecture, exact version schemas, safe static discovery, independent drafts, gated isolated preview and recoverable application. Remaining engineering decisions include the final toolkit after benchmarking, sandbox/bridge implementation, whether VM mode is worth its footprint, exact supported release window, first third-party adapter list and the managed-source ownership contract.

This audit did not build the editor, launch a preview compositor, mutate live settings, test permission enforcement, penetrate a sandbox or measure RAM/GPU use. Live compositor queries were unavailable in the earlier environment check, so actual runtime introspection remains to be verified. The installed Rust toolchain also failed to start in the earlier repository audit; no Rust runtime test success is claimed and no toolchain was changed.

The upstream [security policy](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/SECURITY.md) supports only its latest release, without LTS. Our compatibility support cannot imply upstream security support for an older release.

The result is an auditable route to complete support for declared versions, including every discovered baseline feature and the infrastructure needed to keep coverage current. It explicitly preserves uncertainty where arbitrary programs, native plugins, external applications or physical hardware make automatic detection or consequence-free preview impossible.

All downloaded Hyprland/wiki repositories and temporary audit files were removed after the inventory checks. Only the user-facing reports and inventories remain.
