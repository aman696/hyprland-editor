# Hyprland feature implementation contracts

Design revision 1, 13 October 2026. All entries are specified requirements, not implemented or certified functionality. Shared engine contracts live in the engineering blueprint.

The scalar union contains 403 distinct keys from the two pinned snapshots; each row retains exact version membership and per-version metadata in the JSON. Other rows are fields, actions, inspections or documentation candidates, not scalar settings.

## Coverage counts

| Kind | Contract rows |
|---|---:|
| animation_field | 8 |
| animation_node | 34 |
| bind_option | 17 |
| bind_structure | 8 |
| component_documented_entry | 214 |
| gesture_field | 13 |
| ipc_command | 39 |
| layout_message | 39 |
| legacy_dispatcher | 71 |
| legacy_keyword | 21 |
| lua_api | 100 |
| lua_event | 34 |
| match_predicate | 18 |
| runtime_object_field | 142 |
| runtime_object_method | 39 |
| scalar | 403 |
| socket_event | 45 |
| startup_capability | 15 |
| structured_field | 150 |
| structured_wrapper | 11 |
| typed_spec_field | 32 |

Current core documentation has an additional 756 candidate table entries in the JSON. They are not counted as distinct implemented features.

## Shared engines

| Engine | Blueprint section | Model | Algorithm and validation |
|---|---:|---|---|
| SCALAR | 11 | FieldState<Value> | Typed edit reducer; minimal expression patch or owned generation; version-certified scalar setter/publication. Type/bounds/maps/special grammar, patch reparse, isolated parser, source-defined normalized readback |
| MONITOR | 12 | MonitorSpec | Logical/physical topology model; typed monitor declaration; isolated mapping and guarded hardware apply. Selector/mode/scale/mirror/reserved/color capability checks; usable-output recovery |
| DEVICE | 12 | DeviceSpec | Per-device override graph; exact field table; XKB/curve/region editor; device-targeted activation. 43-field whitelist, input capability, keymap/region/last-input checks |
| WORKSPACE | 12 | WorkspaceSpec/LayoutSpec | Typed selector/declaration and layout options; preserve workspace identity versus display name. Filter existence, target mapping, layout-specific options and ordered runtime effects |
| RULE | 12 | RuleSpec | Ordered predicates/effects; static/dynamic and named/anonymous precedence; owned replacement/reload. Exact effect/match grammar, conflict/precedence fixtures and observed outcomes |
| BIND | 12 | BindSpec/SubmapGraph | Physical/symbol key forms, modifier/device/submap conditions, ordered actions and escape transitions. Key conversion, conflict intersections, flag combinations and focus-bounded recorder |
| GESTURE | 12 | GestureSpec | Identity tuple plus tagged action or start/update/finish workflows; replace/unset identity explicitly. Direction/fingers/mod/scale/variant checks; callback simulation versus physical test distinguished |
| ANIMATION | 12 | AnimationNode/CurveSpec | Tree inheritance, duration/style/curve graph and source-version alias printer. Strict curve bounds, accepted styles, required enabled-node fields and deterministic scene |
| WORKFLOW | 13 | WorkflowSpec/ActionSpec | Closed typed nodes and variant forms; transitive effect set; owned lifecycle handles; AST compiler. Branch/argument conformance, lifetimes, optional targets, effect and operation budgets |
| INSPECTOR | 10 | Observation/RuntimeHandle | Whitelisted IPC or local typed filter; unknown where introspection is unavailable. Sampled interval, target generation, bounded reply/event parsing and field mutability classification |
| LEGACY | 7 | LegacyDirective/LegacyAction | Lossless directive-specific grammar and separately versioned migration to typed action/declaration. Suffix/context/variable/include semantics and target-format support; no execute on import |
| COMPONENT | 19 | ComponentDraft | Exact-version adapter parse/serialize/activate/verify; documentation row classification required. Independent component schema/source/watch behavior, private preview and restore contract |
| STARTUP | 15 | SessionLaunchPlan | Owned launcher/environment source; stage next-session installation; never write active files as draft. Launch ownership, config precedence, special FD restrictions and startup-only effects |

Every row below also carries owner, observation strategy, activation policy, effect, preview status and test IDs in the JSON. Shared template test names are requirements, not evidence of passed tests.

## animation_field

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| animation-field:bezier_points | baseline, current | ANIMATION | bezier_points-form | C-CURVE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation-field:spring_mass | baseline, current | ANIMATION | spring_mass-form | C-CURVE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation-field:spring_stiffness | baseline, current | ANIMATION | spring_stiffness-form | C-CURVE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation-field:spring_damping | baseline, current | ANIMATION | spring_damping-form | C-CURVE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation-field:duration | baseline, current | ANIMATION | duration-form | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation-field:style | baseline, current | ANIMATION | style-form | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation-field:curve_reference | baseline, current | ANIMATION | curve_reference-form | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation-field:enabled | baseline, current | ANIMATION | enabled-form | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |

## animation_node

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| animation:global | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:windows | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:layers | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fade | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:border | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:borderangle | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:shadowangle | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:glowangle | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:workspaces | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:zoomFactor | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:monitorAdded | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:layersIn | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:layersOut | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:windowsIn | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:windowsOut | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:windowsMove | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeIn | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeOut | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeSwitch | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeShadow | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeGlow | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeDim | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeLayers | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeLayersIn | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeLayersOut | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadePopups | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadePopupsIn | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadePopupsOut | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:fadeDpms | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:workspacesIn | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:workspacesOut | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:specialWorkspace | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:specialWorkspaceIn | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| animation:specialWorkspaceOut | baseline, current | ANIMATION | inherited-node-override | C-ANIMATION | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |

## bind_option

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| bind-option:allow_input_capture | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; security-sensitive |
| bind-option:auto_consuming | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:click | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:desc | baseline, current | BIND | description-entry | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:description | baseline, current | BIND | description-entry | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:device | baseline, current | BIND | device-set-builder | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:dont_inhibit | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; security-sensitive |
| bind-option:drag | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:ignore_mods | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:locked | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; security-sensitive |
| bind-option:long_press | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:mouse | current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:non_consuming | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:release | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:repeating | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:submap_universal | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |
| bind-option:transparent | baseline, current | BIND | flag-switch | C-BIND | reviewed version-certified publication; no mutation while editing; ordinary-declaration |

## bind_structure

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| bind-structure:keys | baseline, current | BIND | keys-builder | C-BIND | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| bind-structure:actions | baseline, current | BIND | actions-builder | C-BIND | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| bind-structure:submap | baseline, current | BIND | submap-builder | C-BIND | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| bind-structure:nested_submap | baseline, current | BIND | nested_submap-builder | C-BIND | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| bind-structure:auto_transition | baseline, current | BIND | auto_transition-builder | C-BIND | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| bind-structure:catchall | baseline, current | BIND | catchall-builder | C-BIND | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| bind-structure:reset | baseline, current | BIND | reset-builder | C-BIND | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| bind-structure:device_tags | baseline, current | BIND | device_tags-builder | C-BIND | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |

## component_documented_entry

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| hypridle / General / lock_cmd | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / General / unlock_cmd | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / General / on_lock_cmd | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / General / on_unlock_cmd | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / General / before_sleep_cmd | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / General / after_sleep_cmd | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / General / ignore_dbus_inhibit | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / General / ignore_systemd_inhibit | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / General / ignore_wayland_inhibit | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / General / inhibit_sleep | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / Listeners / timeout | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / Listeners / on-timeout | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / Listeners / on-resume | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / Listeners / ignore_inhibit | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / Listeners / condition_cmd | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hypridle / Listeners / condition_retry | independent-component-version-needed | COMPONENT | component-hypridle-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprland-qt-support / Configuration / roundness | independent-component-version-needed | COMPONENT | component-hyprland-qt-support-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprland-qt-support / Configuration / border_width | independent-component-version-needed | COMPONENT | component-hyprland-qt-support-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprland-qt-support / Configuration / reduce_motion | independent-component-version-needed | COMPONENT | component-hyprland-qt-support-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / General / grab_focus | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / Cache / enabled | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / Finders / default_finder | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / Finders / desktop_prefix | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / Finders / unicode_prefix | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / Finders / math_prefix | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / Finders / font_prefix | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / Finders / desktop_launch_prefix | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / Finders / desktop_icons | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlauncher / UI / window_size | independent-component-version-needed | COMPONENT | component-hyprlauncher-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Command-line Arguments / -v`/`--verbose | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Command-line Arguments / -q`/`--quiet | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Command-line Arguments / -c`/`--config` FILE | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Command-line Arguments / --display` NAME | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Command-line Arguments / --grace` SECONDS | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Command-line Arguments / --immediate-render | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Command-line Arguments / --no-fade-in | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Command-line Arguments / -V`/`--version | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Command-line Arguments / -h`/`--help | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Variable Types / layoutxy | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / General / hide_cursor | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / General / ignore_empty_input | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / General / immediate_render | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / General / text_trim | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / General / fractional_scaling | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / General / screencopy_mode | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / General / fail_timeout | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Authentication / pam:enabled | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Authentication / pam:module | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Authentication / fingerprint:enabled | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Authentication / fingerprint:ready_message | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Authentication / fingerprint:present_message | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Authentication / fingerprint:retry_delay | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Animations / enabled | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Keyboard Shortcuts and Actions / ESC | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Keyboard Shortcuts and Actions / Ctrl + u | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Keyboard Shortcuts and Actions / Ctrl + Backspace | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shadowable / shadow_passes | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shadowable / shadow_size | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shadowable / shadow_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shadowable / shadow_boost | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Clickable / onclick | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; code-or-external-effect |
| hyprlock / Background / monitor | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / path | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / blur_passes | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / blur_size | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / noise | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / contrast | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / brightness | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / vibrancy | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / vibrancy_darkness | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / reload_time | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / reload_cmd | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / crossfade_time | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Background / zindex | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / monitor | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / path | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / size | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / rounding | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / border_size | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / border_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / rotate | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / reload_time | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / reload_cmd | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / position | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / halign | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / valign | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Image / zindex | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / monitor | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / size | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / rounding | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / rotate | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / border_size | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / border_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / xray | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / position | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / halign | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / valign | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Shape / zindex | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / monitor | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / size | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / outline_thickness | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / dots_size | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / dots_spacing | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / dots_center | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / dots_rounding | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / dots_text_format | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / outer_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / inner_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / font_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / font_family | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / fade_on_empty | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / fade_timeout | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / placeholder_text | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / hide_input | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / hide_input_base_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / rounding | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / check_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / check_text | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / fail_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / fail_text | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / capslock_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / numlock_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / bothlock_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / invert_numlock | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / swap_font_color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / position | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / halign | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / valign | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Input Field / zindex | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / monitor | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / text | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / text_align | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / color | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / font_size | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / font_family | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / rotate | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / position | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / halign | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprlock / Label / valign | independent-component-version-needed | COMPONENT | component-hyprlock-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Setting wallpapers / monitor | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Setting wallpapers / path | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Setting wallpapers / fit_mode | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Setting wallpapers / timeout | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Setting wallpapers / order | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Setting wallpapers / recursive | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Misc Options / splash | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Misc Options / splash_offset | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Misc Options / splash_opacity | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpaper / Misc Options / ipc | independent-component-version-needed | COMPONENT | component-hyprpaper-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -a`/`--autocopy | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -f`/`--format= | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -o`/`--output-format= | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -n`/`--notify | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -b`/`--no-fancy | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -h`/`--help | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -r`/`--render-inactive | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -z`/`--no-zoom | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -q`/`--quiet | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -v`/`--verbose | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -t`/`--no-fractional | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -d`/`--disable-hex-preview | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -l`/`--lowercase-hex | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -s`/`--scale=scale | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -u`/`--radius=radius | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -N`/`--name | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprpicker / Configuration / -V`/`--version | independent-component-version-needed | COMPONENT | component-hyprpicker-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Theme / color_scheme | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Theme / icon_theme | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Theme / style | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Theme / font_fixed | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Theme / font_fixed_size | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Theme / font | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Theme / font_size | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Misc / single_click_activate | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Misc / menus_have_icons | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprqt6engine / Misc / shortcuts_for_context_menus | independent-component-version-needed | COMPONENT | component-hyprqt6engine-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprshutdown / Command-Line Options / --vt N | independent-component-version-needed | COMPONENT | component-hyprshutdown-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprshutdown / Command-Line Options / --dry-run | independent-component-version-needed | COMPONENT | component-hyprshutdown-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprshutdown / Command-Line Options / --no-exit | independent-component-version-needed | COMPONENT | component-hyprshutdown-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; hardware-or-session-effect |
| hyprshutdown / Command-Line Options / --top-label`, `-t | independent-component-version-needed | COMPONENT | component-hyprshutdown-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprshutdown / Command-Line Options / --post-cmd`, `-p | independent-component-version-needed | COMPONENT | component-hyprshutdown-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprshutdown / Command-Line Options / --no-fork | independent-component-version-needed | COMPONENT | component-hyprshutdown-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprshutdown / Command-Line Options / --verbose | independent-component-version-needed | COMPONENT | component-hyprshutdown-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprshutdown / Command-Line Options / --help`, `-h | independent-component-version-needed | COMPONENT | component-hyprshutdown-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprsunset / Configuration / max-gamma | independent-component-version-needed | COMPONENT | component-hyprsunset-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprsunset / Profile / time | independent-component-version-needed | COMPONENT | component-hyprsunset-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprsunset / Profile / temperature | independent-component-version-needed | COMPONENT | component-hyprsunset-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprsunset / Profile / gamma | independent-component-version-needed | COMPONENT | component-hyprsunset-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprsunset / Profile / identity | independent-component-version-needed | COMPONENT | component-hyprsunset-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprsunset / Profile / on-switch | independent-component-version-needed | COMPONENT | component-hyprsunset-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; code-or-external-effect |
| hyprtoolkit / Configuration / background | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / base | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / text | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / alternate_base | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / bright_text | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / accent | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / accent_secondary | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / h1_size | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / h2_size | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / h3_size | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / font_size | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / small_font_size | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / icon_theme | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / font_family | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / font_family_monospace | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / rounding_large | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| hyprtoolkit / Configuration / rounding_small | independent-component-version-needed | COMPONENT | component-hyprtoolkit-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| xdg-desktop-portal-hyprland / Screencopy / max_fps | independent-component-version-needed | COMPONENT | component-xdg-desktop-portal-hyprland-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| xdg-desktop-portal-hyprland / Screencopy / allow_token_by_default | independent-component-version-needed | COMPONENT | component-xdg-desktop-portal-hyprland-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| xdg-desktop-portal-hyprland / Screencopy / custom_picker_binary | independent-component-version-needed | COMPONENT | component-xdg-desktop-portal-hyprland-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| xdg-desktop-portal-hyprland / Screencopy / force_shm | independent-component-version-needed | COMPONENT | component-xdg-desktop-portal-hyprland-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |
| xdg-desktop-portal-hyprland / Screencopy / cursor_mode | independent-component-version-needed | COMPONENT | component-xdg-desktop-portal-hyprland-field-or-action-form | C-COMPONENT | Blocked until classified as setting/action/alias/reference and component source schema certified; ordinary-declaration |

## gesture_field

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| gesture:fingers | baseline, current | GESTURE | fingers-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:direction | baseline, current | GESTURE | direction-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:action | baseline, current | GESTURE | action-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:mods | baseline, current | GESTURE | mods-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:scale | baseline, current | GESTURE | scale-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:mode | baseline, current | GESTURE | mode-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:zoom_level | baseline, current | GESTURE | zoom_level-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:workspace_name | baseline, current | GESTURE | workspace_name-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:disable_inhibit | baseline, current | GESTURE | disable_inhibit-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:start | baseline, current | GESTURE | start-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:update | baseline, current | GESTURE | update-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:finish | baseline, current | GESTURE | finish-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| gesture:unset_identity | baseline, current | GESTURE | unset_identity-form-or-workflow | C-GESTURE | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |

## ipc_command

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| ipc-command:workspaces | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:workspacerules | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:activeworkspace | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:clients | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:kill | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:activewindow | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:layers | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:version | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:devices | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:splash | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:cursorpos | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:binds | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:globalshortcuts | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:systeminfo | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:animations | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:rollinglog | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:configerrors | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:locked | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:descriptions | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:submap | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:status | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:reloadshaders | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:monitors | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:reload | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:plugin | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; code-or-external-effect |
| ipc-command:notify | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:dismissnotify | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:getprop | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:seterror | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:switchxkblayout | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:output | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:dispatch | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:keyword | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:setcursor | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:getoption | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:decorations | baseline | INSPECTOR | read-only-inspector | C-IPC | read whitelist with argument scope; read-only; diagnostics may expose private metadata |
| ipc-command:[[BATCH]] | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:eval | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |
| ipc-command:repl | baseline | WORKFLOW | restricted-action-form; no general console | C-IPC | mutation capability and reviewed operation only; ordinary-declaration |

## layout_message

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| layout-message:dwindle-layout:movetoroot | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:dwindle-layout:preselect | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:dwindle-layout:rotatesplit | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:dwindle-layout:splitratio | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:dwindle-layout:swapsplit | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:dwindle-layout:togglesplit | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:addmaster | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:cyclenext | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:cycleprev | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:focusmaster | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:mfact | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:orientationbottom | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:orientationcenter | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:orientationcycle | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:orientationleft | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:orientationnext | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:orientationprev | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:orientationright | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:orientationtop | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:removemaster | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:rollnext | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:rollprev | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:swapnext | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:swapprev | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:master-layout:swapwithmaster | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:monocle-layout:cyclenext | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:monocle-layout:cycleprev | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:center | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:colresize | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:consume | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:consume_or_expel | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:expel | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:fit | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:fit_into_view | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:focus | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:inhibit_scroll | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:move | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:promote | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |
| layout-message:scrolling-layout:swapcol | current-docs-candidate | WORKFLOW | layout-specific-message-form | C-LAYOUT-MESSAGE | action only; exact handler version conformance required; conservative classification; unknown effects block ordinary automatic apply |

## legacy_dispatcher

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| legacy-dispatcher:exec | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; code-or-external-effect |
| legacy-dispatcher:execr | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; code-or-external-effect |
| legacy-dispatcher:killactive | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:forcekillactive | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:closewindow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:killwindow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:signal | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:signalwindow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:togglefloating | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:setfloating | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:settiled | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:workspace | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:renameworkspace | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:fullscreen | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:fullscreenstate | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:movetoworkspace | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:movetoworkspacesilent | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:pseudo | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:movefocus | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:movewindow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:swapwindow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:centerwindow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:togglegroup | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:changegroupactive | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:movegroupwindow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:focusmonitor | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:movecursortocorner | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:movecursor | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:workspaceopt | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:exit | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; hardware-or-session-effect |
| legacy-dispatcher:movecurrentworkspacetomonitor | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:focusworkspaceoncurrentmonitor | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:moveworkspacetomonitor | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:togglespecialworkspace | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:forcerendererreload | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:resizeactive | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:moveactive | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:cyclenext | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:focuswindowbyclass | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:focuswindow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:tagwindow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:toggleswallow | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:submap | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:pass | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:sendshortcut | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:sendkeystate | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:layoutmsg | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:dpms | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; hardware-or-session-effect |
| legacy-dispatcher:movewindowpixel | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:resizewindowpixel | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:swapnext | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:swapactiveworkspaces | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:pin | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:mouse | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:bringactivetotop | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:alterzorder | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:focusurgentorlast | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:focuscurrentorlast | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:lockgroups | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:lockactivegroup | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:moveintogroup | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:moveoutofgroup | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:movewindoworgroup | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:moveintoorcreategroup | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:setignoregrouplock | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:denywindowfromgroup | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:event | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:global | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:setprop | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:forceidle | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |
| legacy-dispatcher:releaseinputcapture | baseline | LEGACY | typed-legacy-action-form | C-LEGACY-ACTION | baseline action or reviewed migration; never execute on import; ordinary-declaration |

## legacy_keyword

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| legacy-keyword:exec | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-EXEC | baseline publication only; current target needs explicit semantic migration; code-or-external-effect |
| legacy-keyword:execr | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-EXECR | baseline publication only; current target needs explicit semantic migration; code-or-external-effect |
| legacy-keyword:exec-once | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-EXEC-ONCE | baseline publication only; current target needs explicit semantic migration; code-or-external-effect |
| legacy-keyword:execr-once | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-EXECR-ONCE | baseline publication only; current target needs explicit semantic migration; code-or-external-effect |
| legacy-keyword:exec-shutdown | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-EXEC-SHUTDOWN | baseline publication only; current target needs explicit semantic migration; code-or-external-effect |
| legacy-keyword:monitor | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-MONITOR | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:bind | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-BIND | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:unbind | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-UNBIND | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:workspace | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-WORKSPACE | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:windowrule | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-WINDOWRULE | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:layerrule | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-LAYERRULE | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:bezier | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-BEZIER | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:animation | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-ANIMATION | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:source | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-SOURCE | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:submap | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-SUBMAP | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:plugin | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-PLUGIN | baseline publication only; current target needs explicit semantic migration; code-or-external-effect |
| legacy-keyword:permission | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-PERMISSION | baseline publication only; current target needs explicit semantic migration; security-sensitive |
| legacy-keyword:gesture | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-GESTURE | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:env | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-ENV | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:windowrulev2 | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-WINDOWRULEV2 | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |
| legacy-keyword:layerrulev2 | baseline | LEGACY | directive-specific-graphical-editor | C-LEGACY-LAYERRULEV2 | baseline publication only; current target needs explicit semantic migration; ordinary-declaration |

## lua_api

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| api:hl.animation | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.bind | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.clear_crashed_lockscreen | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.config | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.curve | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.define_submap | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.device | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dispatch | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.cursor.move | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.cursor.move_to_corner | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.dpms | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | guarded test or explicit irreversible action; hardware-or-session-effect |
| api:hl.dsp.event | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.exec_cmd | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | separate reviewed effect; blocked in ordinary preview; code-or-external-effect |
| api:hl.dsp.exec_raw | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | separate reviewed effect; blocked in ordinary preview; code-or-external-effect |
| api:hl.dsp.exit | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | guarded test or explicit irreversible action; hardware-or-session-effect |
| api:hl.dsp.focus | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.force_idle | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | guarded test or explicit irreversible action; hardware-or-session-effect |
| api:hl.dsp.force_renderer_reload | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.global | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.group.active | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.group.lock | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.group.lock_active | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.group.move_window | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.group.next | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.group.prev | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.group.toggle | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.layout | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.no_op | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.pass | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.release_input_capture | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.reload_config | current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.send_key_state | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.send_shortcut | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.submap | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.alter_zorder | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.bring_to_top | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.center | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.clear_tags | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.close | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.cycle_next | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.deny_from_group | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.drag | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.float | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.fullscreen | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.fullscreen_state | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.kill | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.move | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.pin | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.pseudo | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.resize | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.set_prop | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.signal | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.swap | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.tag | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.window.toggle_swallow | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.workspace.change_id | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.workspace.move | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.workspace.rename | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.workspace.swap_monitors | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.dsp.workspace.toggle_special | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.env | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.exec_cmd | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | separate reviewed effect; blocked in ordinary preview; code-or-external-effect |
| api:hl.exec_scheduled_prop_refresh_immediately | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | separate reviewed effect; blocked in ordinary preview; code-or-external-effect |
| api:hl.gesture | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.get_active_monitor | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_active_special_workspace | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_active_window | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_active_workspace | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_config | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_current_submap | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_cursor_pos | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_devices | current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_last_window | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_last_workspace | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_layers | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_loaded_plugins | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | separate reviewed effect; blocked in ordinary preview; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_monitor | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_monitor_at | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_monitor_at_cursor | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_monitors | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_urgent_window | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_window | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_windows | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_workspace | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_workspace_windows | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.get_workspaces | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.is_key_down | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.layer_rule | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.layout.register | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.monitor | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.notification.create | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.notification.get | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.on | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.permission | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | restart-only unless certified source activation proves otherwise; security-sensitive |
| api:hl.plugin.load | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | separate reviewed effect; blocked in ordinary preview; code-or-external-effect |
| api:hl.timer | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.unbind | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.version | baseline, current | WORKFLOW | typed-query-node | C-LUA-API | version-certified runtime setter or reviewed publication; ('query-node', 'none until used inside a reviewed workflow', 'typed synthetic query backend') |
| api:hl.window_rule | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |
| api:hl.workspace_rule | baseline, current | WORKFLOW | typed-action-or-declaration-node | C-LUA-API | version-certified runtime setter or reviewed publication; ordinary-declaration |

## lua_event

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| lua-event:config.props_refreshed | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:config.reloaded | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:config.unload | current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:hyprland.shutdown | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:hyprland.start | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:input.keyboard.key | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; payload-dependent; keyboard payload privacy-sensitive |
| lua-event:keybinds.submap | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:layer.closed | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:layer.opened | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:monitor.added | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:monitor.focused | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:monitor.layout_changed | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:monitor.removed | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:screenshare.state | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.active | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.bell | current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.class | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.close | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.destroy | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.fullscreen | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.kill | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.minimize | current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.move_to_workspace | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.open | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.open_early | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.pin | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.title | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.update_rules | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:window.urgent | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:workspace.active | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:workspace.created | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:workspace.move_to_monitor | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:workspace.removed | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |
| lua-event:workspace.special_active | baseline, current | WORKFLOW | typed-trigger-and-payload-node | C-EVENT | owned subscription created during reviewed workflow installation; lifecycle cleaned; callback effects computed transitively |

## match_predicate

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| match:window:class | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:title | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:initial_class | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:initial_title | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:float | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:tag | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:xwayland | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:fullscreen | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:pin | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:focus | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:group | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:modal | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:fullscreen_state_internal | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:fullscreen_state_client | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:workspace | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:content | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:window:xdg_tag | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| match:layer:namespace | baseline, current | RULE | typed-predicate-and-negation-builder | C-MATCH | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |

## runtime_object_field

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| object-field:HL.Device:active_keymap | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:active_layout_index | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:address | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:belongs_to | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:caps_lock | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:default_speed | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:layout | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:main | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:model | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:name | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:num_lock | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:options | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:rules | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:scroll_factor | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:type | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Device:variant | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Group:current | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Group:current_index | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Group:denied | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Group:locked | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Group:members | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Group:size | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:allow_input_capture | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:arg | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:auto_consuming | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:catchall | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:click | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:description | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:device_inclusive | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:devices | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:display_key | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:dont_inhibit | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:drag | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:enabled | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:handler | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:has_description | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:ignore_mods | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:key | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:keycode | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:locked | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:long_press | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:modmask | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:mouse | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:non_consuming | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:release | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:repeating | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:submap | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:submap_universal | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Keybind:transparent | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:above_fullscreen | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:address | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:h | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:interactivity | baseline | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:keyboard_interactivity | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:layer | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:mapped | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:monitor | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:namespace | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:pid | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:w | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:x | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.LayerSurface:y | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:active_special_workspace | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:active_workspace | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:available_modes | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:cm | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:description | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:dpms_status | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:enabled | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:focused | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:hardware_details | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:height | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:id | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:is_mirror | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:mirrors | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:mode | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:name | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:physical_height | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:physical_width | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:position | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:refresh_rate | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:reserved | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:scale | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:serial | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:size | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:transform | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:vrr_active | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:width | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:x | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Monitor:y | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:accepts_input | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:active | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:address | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:allowed_over_fullscreen | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:at | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:class | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:content_type | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:floating | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:focus_history_id | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:fullscreen | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:fullscreen_client | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:fullscreen_handler | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:group | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:hidden | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:inhibiting_idle | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:initial_class | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:initial_title | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:layout | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:mapped | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:monitor | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:pid | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:pin_fullscreened | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:pinned | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:size | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:stable_id | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:swallowing | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:tags | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:tearing_hint | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:title | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:visible | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:workspace | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:xdg_description | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:xdg_tag | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Window:xwayland | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:active | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:addressable_name | current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:config_name | baseline | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:fullscreen_mode | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:fullscreen_window | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:groups | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:has_fullscreen | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:has_urgent | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:id | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:is_empty | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:is_persistent | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:last_window | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:monitor | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:name | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:special | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:tiled_layout | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:visible | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |
| object-field:HL.Workspace:windows | baseline, current | INSPECTOR | read-only-inspector-and-query-output | C-OBSERVATION | not a setting; setters only through separate explicit certified nodes; read-only observation |

## runtime_object_method

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| object-method:HL.EventSubscription:is_active | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.EventSubscription:remove | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Group:add | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Group:remove | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Keybind:is_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Keybind:remove | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Keybind:set_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Keybind:unbind | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.LayerRule:is_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.LayerRule:set_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Monitor:set_special_workspace | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Monitor:set_workspace | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:dismiss | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:get_color | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:get_elapsed | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:get_elapsed_since_creation | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:get_font_size | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:get_icon | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:get_text | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:get_timeout | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:is_alive | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:is_paused | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:pause | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:resume | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:set_color | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:set_font_size | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:set_icon | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:set_paused | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:set_text | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Notification:set_timeout | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Timer:is_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Timer:set_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Timer:set_timeout | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.WindowRule:is_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.WindowRule:set_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Workspace:get_groups | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.Workspace:get_windows | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.WorkspaceRule:is_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| object-method:HL.WorkspaceRule:set_enabled | baseline, current | WORKFLOW | typed-handle-method-node | C-HANDLE-METHOD | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |

## scalar

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| scalar:animations:enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:animations:workspace_wraparound | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:allow_pin_fullscreen | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:allow_workspace_cycles | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:disable_keybind_grabbing | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:drag_center_window | current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:drag_threshold | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:focus_preferred_method | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:hide_special_on_workspace_change | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:ignore_group_lock | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:movefocus_cycles_fullscreen | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:movefocus_cycles_groupfirst | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:pass_mouse_when_bound | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:scroll_event_delay | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:window_direction_monitor_fallback | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:workspace_back_and_forth | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:binds:workspace_center_on | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:default_monitor | baseline, current | SCALAR | output-selector | C-SELECTOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:enable_hyprcursor | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:hide_on_key_press | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:hide_on_tablet | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:hide_on_touch | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:hotspot_padding | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:inactive_timeout | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:invisible | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:min_refresh_rate | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:no_break_fs_vrr | baseline, current | SCALAR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| scalar:cursor:no_hardware_cursors | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:no_warps | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:persistent_warps | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:sync_gsettings_theme | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:use_cpu_buffer | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:warp_back_after_non_mouse_input | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:warp_on_change_workspace | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:warp_on_monitor_change | current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:warp_on_toggle_special | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:zoom_detached_camera | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:zoom_disable_aa | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:zoom_factor | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:cursor:zoom_rigid | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:debug:colored_stdout_logs | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:damage_blink | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:damage_tracking | baseline, current | SCALAR | integer-or-enum-picker | C-INT | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:disable_logs | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:disable_scale_checks | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:disable_time | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:ds_handle_same_buffer | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:ds_handle_same_buffer_fifo | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:enable_stdout_logs | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:error_limit | baseline, current | SCALAR | integer-or-enum-picker | C-INT | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:error_position | baseline, current | SCALAR | integer-or-enum-picker | C-INT | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:fifo_pending_workaround | baseline | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:full_cm_proto | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:gl_debugging | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:invalidate_buffers | current | SCALAR | integer-or-enum-picker | C-INT | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:invalidate_fp16 | baseline, current | SCALAR | integer-or-enum-picker | C-INT | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:log_damage | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:manual_crash | baseline, current | SCALAR | integer-or-enum-picker | C-INT | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:overlay | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:pass | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:render_solitary_wo_damage | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:suppress_errors | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:debug:vfr | baseline, current | SCALAR | boolean-switch | C-BOOL | reviewed diagnostic change; diagnostic/privacy |
| scalar:decoration:active_opacity | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:acrylic:aberration | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:acrylic:bulb | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:acrylic:clarity | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:acrylic:refraction | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:acrylic:tint | current | SCALAR | rgba-picker | C-COLOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:aurora:color1 | current | SCALAR | rgba-picker | C-COLOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:aurora:color2 | current | SCALAR | rgba-picker | C-COLOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:aurora:intensity | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:aurora:speed | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:brightness | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:contrast | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:drops:speed | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:fluid_jar:color | current | SCALAR | rgba-picker | C-COLOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:fluid_jar:distortion | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:fluid_jar:fill_amount | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:fluid_jar:mass | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:fluid_jar:precision | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:fluid_jar:speed | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:fluid_jar:turbulence | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:glass:refraction | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:glass:roughness | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:glass:size | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:haze:intensity | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:haze:iridescence | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:heat_shimmer:speed | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:ignore_opacity | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:input_methods | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:input_methods_ignorealpha | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:new_optimizations | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:noise | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:passes | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:popups | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:popups_ignorealpha | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:ripple:duration | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:ripple:radius | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:ripple:strength | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:ripple:width | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:size | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:special | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:variant | current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:vibrancy | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:vibrancy_darkness | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:water:damping | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:water:duration | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:water:radius | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:water:speed | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:water:strength | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:blur:xray | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:border_part_of_window | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:dim_around | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:dim_inactive | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:dim_modal | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:dim_special | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:dim_strength | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:fullscreen_opacity | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:glow:color | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:glow:color_inactive | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:glow:enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:glow:range | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:glow:render_power | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:inactive_opacity | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:motion_blur:enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:motion_blur:samples | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:rounding | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:rounding_power | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:screen_shader | baseline, current | SCALAR | shader-asset-and-material-graph | C-SHADER | separate reviewed effect; blocked in ordinary preview; code-or-external-effect |
| scalar:decoration:shadow:color | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:shadow:color_inactive | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:shadow:enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:shadow:offset | baseline, current | SCALAR | positional-two-axis-control | C-VEC2 | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:shadow:range | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:shadow:render_power | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:shadow:scale | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:shadow:sharp | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:wobble:damping | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:wobble:enabled | current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:wobble:intensity | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:wobble:mass | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:wobble:mesh | current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:wobble:stiffness | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:wobble:value_epsilon | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:decoration:wobble:velocity_epsilon | current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:default_split_ratio | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:force_split | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:permanent_direction_override | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:precise_mouse_move | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:preserve_split | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:smart_resizing | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:smart_split | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:special_scale_factor | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:split_bias | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:split_width_multiplier | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:dwindle:use_active_for_splits | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:ecosystem:enforce_permissions | baseline, current | SCALAR | boolean-switch | C-BOOL | restart-only unless certified source activation proves otherwise; security-sensitive |
| scalar:ecosystem:no_donation_nag | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:ecosystem:no_update_news | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:experimental:wp_cm_1_2 | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:allow_tearing | baseline, current | SCALAR | boolean-switch | C-BOOL | guarded test or explicit irreversible action; hardware-or-session-effect |
| scalar:general:border_size | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:col.active_border | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:col.inactive_border | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:col.nogroup_border | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:col.nogroup_border_active | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:extend_border_grab_area | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:float_gaps | baseline, current | SCALAR | four-side-spacing | C-GAP | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:gaps_in | baseline, current | SCALAR | four-side-spacing | C-GAP | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:gaps_out | baseline, current | SCALAR | four-side-spacing | C-GAP | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:gaps_workspaces | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:hover_icon_on_border | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:layout | baseline, current | SCALAR | layout-choice | C-LAYOUT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:locale | baseline, current | SCALAR | locale-picker | C-LOCALE | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:modal_parent_blocking | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:no_focus_fallback | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:resize_corner | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:resize_on_border | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:snap:border_overlap | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:snap:enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:snap:monitor_gap | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:snap:respect_gaps | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:general:snap:window_gap | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:close_max_timeout | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:scrolling:move_snap_cursor | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:scrolling:move_snap_to_grid | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_cancel_ratio | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_create_new | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_direction_lock | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_direction_lock_threshold | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_distance | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_forever | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_invert | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_min_speed_to_force | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_touch | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_touch_invert | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:gestures:workspace_swipe_use_r | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:auto_group | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:col.border_active | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:col.border_inactive | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:col.border_locked_active | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | reviewed safety policy; security-sensitive |
| scalar:group:col.border_locked_inactive | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | reviewed safety policy; security-sensitive |
| scalar:group:drag_into_group | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:focus_removed_window | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:group_on_movetoworkspace | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:blur | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:col.active | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:col.inactive | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:col.locked_active | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | reviewed safety policy; security-sensitive |
| scalar:group:groupbar:col.locked_inactive | baseline, current | SCALAR | ordered-colors-and-angle | C-GRADIENT | reviewed safety policy; security-sensitive |
| scalar:group:groupbar:disable_when_only | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:font_family | baseline, current | SCALAR | font-family-picker | C-FONT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:font_size | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:font_weight_active | baseline, current | SCALAR | font-weight-picker | C-FONTWEIGHT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:font_weight_inactive | baseline, current | SCALAR | font-weight-picker | C-FONTWEIGHT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:gaps_in | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:gaps_out | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:gradient_round_only_edges | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:gradient_rounding | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:gradient_rounding_power | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:gradients | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:height | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:indicator_gap | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:indicator_height | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:keep_upper_gap | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:middle_click_close | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:priority | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:render_titles | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:round_only_edges | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:rounding | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:rounding_power | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:scrolling | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:stacked | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:text_color | baseline, current | SCALAR | rgba-picker | C-COLOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:text_color_inactive | baseline, current | SCALAR | rgba-picker | C-COLOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:text_color_locked_active | baseline, current | SCALAR | rgba-picker | C-COLOR | reviewed safety policy; security-sensitive |
| scalar:group:groupbar:text_color_locked_inactive | baseline, current | SCALAR | rgba-picker | C-COLOR | reviewed safety policy; security-sensitive |
| scalar:group:groupbar:text_offset | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:groupbar:text_padding | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:insert_after_current | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:merge_floated_into_tiled_on_groupbar | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:merge_groups_on_drag | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:group:merge_groups_on_groupbar | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:input-capture:capture_modifiers | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:input-capture:enforce_barriers | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:input:accel_profile | baseline, current | SCALAR | acceleration-profile-and-points | C-CURVEPOINTS | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:emulate_discrete_scroll | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:float_switch_override_focus | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:focus_on_close | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:follow_mouse | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:follow_mouse_shrink | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:follow_mouse_threshold | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:force_no_accel | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:kb_file | baseline, current | SCALAR | keymap-asset-picker | C-ASSET | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:kb_layout | baseline, current | SCALAR | xkb-layout-list | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:kb_model | baseline, current | SCALAR | xkb-model-picker | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:kb_options | baseline, current | SCALAR | xkb-option-groups | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:kb_rules | baseline, current | SCALAR | xkb-rules-picker | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:kb_variant | baseline, current | SCALAR | xkb-linked-variant-list | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:left_handed | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:mouse_refocus | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:natural_scroll | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:numlock_by_default | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:off_window_axis_events | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:repeat_delay | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:repeat_rate | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:resolve_binds_by_sym | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:rotation | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:scroll_button | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:scroll_button_lock | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:scroll_factor | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:scroll_method | baseline, current | SCALAR | scroll-method-picker | C-ENUMTEXT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:scroll_points | baseline, current | SCALAR | ordered-scroll-point-editor | C-CURVEPOINTS | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:sensitivity | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:special_fallthrough | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:absolute_region_position | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:active_area_position | baseline, current | SCALAR | positional-two-axis-control | C-VEC2 | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:active_area_size | baseline, current | SCALAR | positional-two-axis-control | C-VEC2 | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:enabled | current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:left_handed | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:output | baseline, current | SCALAR | output-selector | C-SELECTOR | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:region_position | baseline, current | SCALAR | positional-two-axis-control | C-VEC2 | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:region_size | baseline, current | SCALAR | positional-two-axis-control | C-VEC2 | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:relative_input | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablet:transform | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablettool:eraser_button_mode | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablettool:eraser_button_override | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablettool:pressure_range_max | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:tablettool:pressure_range_min | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchdevice:enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchdevice:output | baseline, current | SCALAR | output-selector | C-SELECTOR | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchdevice:transform | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:clickfinger_behavior | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:disable_while_typing | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:drag_3fg | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:drag_lock | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:flip_x | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:flip_y | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:middle_button_emulation | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:natural_scroll | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:scroll_factor | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:tap-and-drag | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:tap-to-click | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:touchpad:tap_button_map | baseline, current | SCALAR | tap-button-map-picker | C-ENUMTEXT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:virtualkeyboard:release_pressed_on_close | baseline, current | SCALAR | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:input:virtualkeyboard:share_states | baseline, current | SCALAR | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| scalar:layout:single_window_aspect_ratio | baseline, current | SCALAR | positional-two-axis-control | C-VEC2 | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:layout:single_window_aspect_ratio_tolerance | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:allow_small_split | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:always_keep_position | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:center_ignores_reserved | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:center_master_fallback | baseline, current | SCALAR | fallback-orientation-picker | C-ENUMTEXT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:drop_at_cursor | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:focus_master_on_close | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:mfact | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:new_on_active | baseline, current | SCALAR | new-window-placement-picker | C-ENUMTEXT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:new_on_top | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:new_status | baseline, current | SCALAR | master-status-picker | C-ENUMTEXT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:orientation | baseline, current | SCALAR | orientation-picker | C-ENUMTEXT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:slave_count_for_center_master | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:smart_resizing | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:master:special_scale_factor | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:allow_session_lock_restore | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:always_follow_on_dnd | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:animate_manual_resizes | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:animate_mouse_windowdragging | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:anr_missed_pings | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:background_color | baseline, current | SCALAR | rgba-picker | C-COLOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:bell_sound | current | SCALAR | sound-asset-picker | C-ASSET | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:close_special_on_empty | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:col.splash | baseline, current | SCALAR | rgba-picker | C-COLOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:disable_autoreload | baseline, current | SCALAR | boolean-switch | C-BOOL | watcher-gate lifecycle policy; transaction-control |
| scalar:misc:disable_hyprland_guiutils_check | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:disable_hyprland_logo | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:disable_scale_notification | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:disable_splash_rendering | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:disable_watchdog_warning | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:disable_xdg_env_checks | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:enable_anr_dialog | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:enable_swallow | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:exit_window_retains_fullscreen | baseline, current | SCALAR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| scalar:misc:float_force_onscreen | current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:focus_on_activate | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:font_family | baseline, current | SCALAR | font-family-picker | C-FONT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:force_default_wallpaper | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:initial_workspace_token_timeout | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:initial_workspace_tracking | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:key_press_enables_dpms | baseline, current | SCALAR | boolean-switch | C-BOOL | guarded test or explicit irreversible action; hardware-or-session-effect |
| scalar:misc:layers_hog_keyboard_focus | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:lockdead_screen_delay | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:middle_click_paste | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:mouse_move_enables_dpms | baseline, current | SCALAR | boolean-switch | C-BOOL | guarded test or explicit irreversible action; hardware-or-session-effect |
| scalar:misc:mouse_move_focuses_monitor | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:name_vk_after_proc | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:new_float_force_onscreen | current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:on_focus_under_fullscreen | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:render_unfocused_fps | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:screencopy_force_8b | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:session_lock_blur | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:session_lock_xray | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:size_limits_tiled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:splash_font_family | baseline, current | SCALAR | font-family-picker | C-FONT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:swallow_exception_regex | baseline, current | SCALAR | regex-match-builder | C-REGEX | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:swallow_regex | baseline, current | SCALAR | regex-match-builder | C-REGEX | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:misc:vrr | baseline, current | SCALAR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| scalar:opengl:nvidia_anti_flicker | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:quirks:prefer_hdr | baseline, current | SCALAR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| scalar:quirks:skip_non_kms_dmabuf_formats | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:async_commit | current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:cm_auto_hdr | baseline, current | SCALAR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| scalar:render:cm_enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:cm_sdr_eotf | baseline, current | SCALAR | transfer-function-picker | C-ENUMTEXT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:commit_timing_enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:ctm_animation | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:direct_scanout | baseline, current | SCALAR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| scalar:render:expand_undersized_textures | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:fp16_sdr_tf | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:icc_vcgt_enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:keep_unmodified_copy | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:new_render_scheduling | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:non_shader_cm | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:non_shader_cm_interop | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:not_shown_fifo_lock | current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:send_content_type | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:use_fp16 | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:use_shader_blur_blend | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:render:xp_mode | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:scrolling:column_width | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:scrolling:direction | baseline, current | SCALAR | direction-picker | C-ENUMTEXT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:scrolling:explicit_column_widths | baseline, current | SCALAR | ordered-column-width-list | C-NUMBERLIST | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:scrolling:focus_fit_method | baseline, current | SCALAR | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:scrolling:follow_focus | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:scrolling:follow_min_visible | baseline, current | SCALAR | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:scrolling:fullscreen_on_one_column | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:scrolling:wrap_focus | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:scrolling:wrap_swapcol | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:xwayland:create_abstract_socket | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:xwayland:enabled | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:xwayland:force_zero_scaling | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| scalar:xwayland:use_nearest_neighbor | baseline, current | SCALAR | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |

## socket_event

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| socket2-event:activelayout | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:activespecial | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:activespecialv2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:activewindow | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:activewindowv2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:bell | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:changefloatingmode | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:changeworkspaceid | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:closelayer | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:closewindow | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:configreloaded | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:createworkspace | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:createworkspacev2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:custom | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:destroyworkspace | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:destroyworkspacev2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:focusedmon | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:focusedmonv2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:fullscreen | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:kill | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:lockgroups | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:minimized | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:monitoradded | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:monitoraddedv2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:monitorremoved | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:monitorremovedv2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:moveintogroup | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:moveoutofgroup | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:movewindow | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:movewindowv2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:moveworkspace | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:moveworkspacev2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:openlayer | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:openwindow | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:pin | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:renameworkspace | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:screencast | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:screencastv2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:submap | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:togglegroup | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:urgent | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:windowtitle | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:windowtitlev2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:workspace | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |
| socket2-event:workspacev2 | baseline, current | INSPECTOR | bounded-event-inspector | C-IPC-EVENT | subscription only; read-only notification; payload privacy scoped |

## startup_capability

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| startup:config_path | version-and-owner-specific | STARTUP | owned-session-config_path-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:format_precedence | version-and-owner-specific | STARTUP | owned-session-format_precedence-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:safe_mode | version-and-owner-specific | STARTUP | owned-session-safe_mode-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:socket_handover | version-and-owner-specific | STARTUP | owned-session-socket_handover-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:watchdog_handover | version-and-owner-specific | STARTUP | owned-session-watchdog_handover-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:locked_startup | version-and-owner-specific | STARTUP | owned-session-locked_startup-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:environment | version-and-owner-specific | STARTUP | owned-session-environment-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:gpu_ordering | version-and-owner-specific | STARTUP | owned-session-gpu_ordering-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:toolkit_backend | version-and-owner-specific | STARTUP | owned-session-toolkit_backend-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:cursor_environment | version-and-owner-specific | STARTUP | owned-session-cursor_environment-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:uwsm_environment | version-and-owner-specific | STARTUP | owned-session-uwsm_environment-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:systemd_autostart | version-and-owner-specific | STARTUP | owned-session-systemd_autostart-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:nix_generated_source | version-and-owner-specific | STARTUP | owned-session-nix_generated_source-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:next_session_install | version-and-owner-specific | STARTUP | owned-session-next_session_install-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |
| startup:profile_portable_mapping | version-and-owner-specific | STARTUP | owned-session-profile_portable_mapping-form | C-STARTUP | next-session/owner export; not ordinary live apply; startup/lifecycle authority; FD handovers not arbitrary numeric fields |

## structured_field

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| field:monitor:bitdepth | baseline, current | MONITOR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:cm | baseline, current | MONITOR | monitor-cm-form | C-MONITOR-CM | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:disabled | baseline, current | MONITOR | boolean-switch | C-BOOL | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:icc | baseline, current | MONITOR | validated-asset-picker | C-ASSET | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:max_avg_luminance | baseline, current | MONITOR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:max_luminance | baseline, current | MONITOR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:min_luminance | baseline, current | MONITOR | finite-decimal-control | C-FLOAT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:mirror | baseline, current | MONITOR | output-selector | C-SELECTOR | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:mode | baseline, current | MONITOR | monitor-mode-form | C-MONITOR-MODE | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:position | baseline, current | MONITOR | monitor-position-form | C-MONITOR-POSITION | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:reserved | baseline, current | MONITOR | monitor-reserved-form | C-GAP | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:reserved_area | baseline, current | MONITOR | monitor-reserved_area-form | C-GAP | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:scale | baseline, current | MONITOR | monitor-scale-form | C-MONITOR-SCALE | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:sdr_eotf | baseline, current | MONITOR | monitor-sdr_eotf-form | C-MONITOR-SDR_EOTF | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:sdr_max_luminance | baseline, current | MONITOR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:sdr_min_luminance | baseline, current | MONITOR | finite-decimal-control | C-FLOAT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:sdrbrightness | baseline, current | MONITOR | finite-decimal-control | C-FLOAT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:sdrsaturation | baseline, current | MONITOR | finite-decimal-control | C-FLOAT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:supports_hdr | baseline, current | MONITOR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:supports_wide_color | baseline, current | MONITOR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:transform | baseline, current | MONITOR | monitor-transform-form | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:monitor:vrr | baseline, current | MONITOR | integer-or-enum-picker | C-INT | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:device:absolute_region_position | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:accel_profile | baseline, current | DEVICE | acceleration-profile-and-points | C-CURVEPOINTS | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:active_area_position | baseline, current | DEVICE | positional-two-axis-control | C-VEC2 | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:active_area_size | baseline, current | DEVICE | positional-two-axis-control | C-VEC2 | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:clickfinger_behavior | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:disable_while_typing | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:drag_3fg | baseline, current | DEVICE | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:drag_lock | baseline, current | DEVICE | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:enabled | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:flip_x | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:flip_y | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:kb_file | baseline, current | DEVICE | keymap-asset-picker | C-ASSET | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:kb_layout | baseline, current | DEVICE | xkb-layout-list | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:kb_model | baseline, current | DEVICE | xkb-model-picker | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:kb_options | baseline, current | DEVICE | xkb-option-groups | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:kb_rules | baseline, current | DEVICE | xkb-rules-picker | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:kb_variant | baseline, current | DEVICE | xkb-linked-variant-list | C-XKB | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:keybinds | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:left_handed | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:middle_button_emulation | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:natural_scroll | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:numlock_by_default | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:output | baseline, current | DEVICE | target-selector | C-SELECTOR | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:region_position | baseline, current | DEVICE | positional-two-axis-control | C-VEC2 | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:region_size | baseline, current | DEVICE | positional-two-axis-control | C-VEC2 | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:relative_input | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:release_pressed_on_close | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:repeat_delay | baseline, current | DEVICE | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:repeat_rate | baseline, current | DEVICE | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:resolve_binds_by_sym | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:rotation | baseline, current | DEVICE | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:scroll_button | baseline, current | DEVICE | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:scroll_button_lock | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:scroll_factor | baseline, current | DEVICE | finite-decimal-control | C-FLOAT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:scroll_method | baseline, current | DEVICE | scroll-method-picker | C-ENUMTEXT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:scroll_points | baseline, current | DEVICE | ordered-scroll-point-editor | C-CURVEPOINTS | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:sensitivity | baseline, current | DEVICE | finite-decimal-control | C-FLOAT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:share_states | baseline, current | DEVICE | integer-or-enum-picker | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:tags | baseline, current | DEVICE | tags-list-picker | C-TAGLIST | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:tap_and_drag | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:tap_button_map | baseline, current | DEVICE | tap_button_map-list-picker | C-ENUMTEXT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:tap_to_click | baseline, current | DEVICE | boolean-switch | C-BOOL | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:device:transform | baseline, current | DEVICE | monitor-transform-form | C-INT | certified input apply; recovery if usability/route is at risk; input-behavior |
| field:workspace_rule:animation | baseline, current | WORKSPACE | rule-animation-builder | C-RULE-ANIMATION | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:border_size | baseline, current | WORKSPACE | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:decorate | baseline, current | WORKSPACE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:default | baseline, current | WORKSPACE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:default_name | baseline, current | WORKSPACE | literal-text | C-STRING | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:float_gaps | baseline, current | WORKSPACE | four-side-spacing | C-GAP | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:gaps_in | baseline, current | WORKSPACE | four-side-spacing | C-GAP | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:gaps_out | baseline, current | WORKSPACE | four-side-spacing | C-GAP | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:layout | baseline, current | WORKSPACE | rule-layout-builder | C-RULE-LAYOUT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:monitor | baseline, current | WORKSPACE | target-selector | C-SELECTOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:no_border | baseline, current | WORKSPACE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:no_rounding | baseline, current | WORKSPACE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:no_shadow | baseline, current | WORKSPACE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:workspace_rule:on_created_empty | baseline, current | WORKSPACE | command-workflow-and-review | C-COMMAND | separate reviewed effect; blocked in ordinary preview; code-or-external-effect |
| field:workspace_rule:persistent | baseline, current | WORKSPACE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:allows_input | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:animation | baseline, current | RULE | rule-animation-builder | C-RULE-ANIMATION | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:border_color | baseline, current | RULE | ordered-colors-and-angle | C-GRADIENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:border_size | baseline, current | RULE | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:center | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:confine_pointer | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:content | baseline, current | RULE | rule-content-builder | C-RULE-CONTENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:decorate | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:dim_around | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:float | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:focus_on_activate | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:force_rgbx | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:fullscreen | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:fullscreen_state | baseline, current | RULE | rule-fullscreen_state-builder | C-RULE-FULLSCREEN_STATE | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:group | baseline, current | RULE | rule-group-builder | C-RULE-GROUP | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:idle_inhibit | baseline, current | RULE | rule-idle_inhibit-builder | C-RULE-IDLE_INHIBIT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:immediate | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:keep_aspect_ratio | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:max_size | baseline, current | RULE | two-axis-expression-graph | C-PLACEMENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:maximize | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:min_size | baseline, current | RULE | two-axis-expression-graph | C-PLACEMENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:monitor | baseline, current | RULE | target-selector | C-SELECTOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:move | baseline, current | RULE | two-axis-expression-graph | C-PLACEMENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:nearest_neighbor | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_anim | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_auto_hdr | baseline, current | RULE | boolean-switch | C-BOOL | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:window_rule_effect:no_blur | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_close_for | baseline, current | RULE | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_dim | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_focus | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_follow_mouse | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_glow | current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_initial_focus | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_max_size | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_screen_share | baseline, current | RULE | boolean-switch | C-BOOL | reviewed safety policy; security-sensitive |
| field:window_rule_effect:no_shadow | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_shortcuts_inhibit | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_vrr | baseline, current | RULE | boolean-switch | C-BOOL | guarded test or explicit irreversible action; hardware-or-session-effect |
| field:window_rule_effect:no_wobble | current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:no_xdg_drags | current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:opacity | baseline, current | RULE | rule-opacity-builder | C-RULE-OPACITY | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:opaque | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:persistent_size | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:pin | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:pseudo | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:render_unfocused | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:rounding | baseline, current | RULE | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:rounding_power | baseline, current | RULE | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:scroll_mouse | baseline, current | RULE | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:scroll_touchpad | baseline, current | RULE | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:scrolling_width | baseline, current | RULE | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:size | baseline, current | RULE | two-axis-expression-graph | C-PLACEMENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:stay_focused | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:suppress_event | baseline, current | RULE | rule-suppress_event-builder | C-RULE-SUPPRESS_EVENT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:sync_fullscreen | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:tag | baseline, current | RULE | rule-tag-builder | C-RULE-TAG | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:tile | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:tonemap | baseline, current | RULE | rule-tonemap-builder | C-RULE-TONEMAP | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:workspace | baseline, current | RULE | target-selector | C-SELECTOR | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:window_rule_effect:xray | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:layer_rule_effect:above_lock | baseline, current | RULE | integer-or-enum-picker | C-INT | reviewed safety policy; security-sensitive |
| field:layer_rule_effect:animation | baseline, current | RULE | rule-animation-builder | C-RULE-ANIMATION | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:layer_rule_effect:blur | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:layer_rule_effect:blur_popups | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:layer_rule_effect:dim_around | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:layer_rule_effect:ignore_alpha | baseline, current | RULE | finite-decimal-control | C-FLOAT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:layer_rule_effect:no_anim | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:layer_rule_effect:no_screen_share | baseline, current | RULE | boolean-switch | C-BOOL | reviewed safety policy; security-sensitive |
| field:layer_rule_effect:order | baseline, current | RULE | integer-or-enum-picker | C-INT | version-certified runtime setter or reviewed publication; ordinary-declaration |
| field:layer_rule_effect:xray | baseline, current | RULE | boolean-switch | C-BOOL | version-certified runtime setter or reviewed publication; ordinary-declaration |

## structured_wrapper

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| wrapper:monitor:output | baseline, current | MONITOR | selector-builder | C-SELECTOR | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:device:name | baseline, current | DEVICE | typed-name-form | C-STRUCTURED | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:workspace_rule:workspace | baseline, current | WORKSPACE | selector-builder | C-SELECTOR | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:workspace_rule:enabled | baseline, current | WORKSPACE | typed-enabled-form | C-STRUCTURED | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:workspace_rule:layout_opts | baseline, current | WORKSPACE | typed-layout_opts-form | C-STRUCTURED | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:window_rule:name | baseline, current | RULE | typed-name-form | C-STRUCTURED | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:window_rule:enabled | baseline, current | RULE | typed-enabled-form | C-STRUCTURED | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:window_rule:match | baseline, current | RULE | selector-builder | C-SELECTOR | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:layer_rule:name | baseline, current | RULE | typed-name-form | C-STRUCTURED | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:layer_rule:enabled | baseline, current | RULE | typed-enabled-form | C-STRUCTURED | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| wrapper:layer_rule:match | baseline, current | RULE | selector-builder | C-SELECTOR | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |

## typed_spec_field

| Feature / field | Versions | Engine | Graphical representation | Codec | Activation / effect |
|---|---|---|---|---|---|
| spec-field:HL.LayoutTarget:index | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutTarget:window | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutTarget:box | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutTarget:place | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutTarget:set_box | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutContext:area | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutContext:targets | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutContext:grid_cell | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutContext:column | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutContext:row | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutContext:split | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutProvider:recalculate | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayoutProvider:layout_msg? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.TimerOptions:timeout | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.TimerOptions:type | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.PermissionSpec:binary | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.PermissionSpec:type | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.PermissionSpec:mode | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.NotificationOptions:text | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.NotificationOptions:timeout | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.NotificationOptions:color? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.NotificationOptions:icon? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.NotificationOptions:font_size? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayerQueryFilter:monitor? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.LayerQueryFilter:namespace? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.WindowQueryFilter:class? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.WindowQueryFilter:floating? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.WindowQueryFilter:mapped? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.WindowQueryFilter:monitor? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.WindowQueryFilter:tag? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.WindowQueryFilter:title? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |
| spec-field:HL.WindowQueryFilter:workspace? | baseline, current | WORKFLOW | typed-input-or-geometry-node | C-LUA-API | reviewed version-certified publication; no mutation while editing; conservative classification; unknown effects block ordinary automatic apply |

## Per-function source argument facts

These are discovery facts from the pinned handlers. Required reads may occur in an optional variant. Field tables and helper defaults are separate. The production ActionSpec must combine these facts with the blueprint variant constraints and parser acceptance tests.

### hl.animation

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlAnimation](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L390) | none found | none found | none found | style |
| current | [hlAnimation](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L395) | none found | none found | none found | style |

### hl.bind

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlBind](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L132) | none found | none found | 1:string | allow_input_capture, auto_consuming, click, desc, description, device, dont_inhibit, drag, ignore_mods, inclusive, list, locked, long_press, non_consuming, release, repeating, submap_universal, transparent |
| current | [hlBind](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L83) | none found | none found | 1:string | allow_input_capture, auto_consuming, click, desc, description, device, dont_inhibit, drag, ignore_mods, inclusive, list, locked, long_press, mouse, non_consuming, release, repeating, submap_universal, transparent |

### hl.clear_crashed_lockscreen

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlClearCrashedLockscreen](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L337) | none found | none found | none found | none found |
| current | [hlClearCrashedLockscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L332) | none found | none found | none found | none found |

### hl.config

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlConfig](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L969) | none found | none found | none found | none found |
| current | [hlConfig](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L976) | none found | none found | none found | none found |

### hl.curve

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlCurve](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L278) | none found | none found | none found | dampening, mass, points, stiffness |
| current | [hlCurve](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L278) | none found | none found | none found | dampening, damping, mass, points, stiffness |

### hl.define_submap

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlDefineSubmap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L258) | none found | none found | 1:string | none found |
| current | [hlDefineSubmap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L253) | none found | none found | 1:string | none found |

### hl.device

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlDevice](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1046) | none found | none found | none found | name |
| current | [hlDevice](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1053) | none found | none found | none found | name |

### hl.dispatch

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlDispatch](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L352) | none found | none found | none found | none found |
| current | [hlDispatch](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L347) | none found | none found | none found | none found |

### hl.dsp.cursor.move

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlCursorMove](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L77) | x (Num), y (Num) | none found | none found | none found |
| current | [hlCursorMove](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L88) | x (Num), y (Num) | none found | none found | none found |

### hl.dsp.cursor.move_to_corner

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlCursorMoveToCorner](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L67) | corner (Num) | none found | none found | none found |
| current | [hlCursorMoveToCorner](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L78) | corner (Num) | none found | none found | none found |

### hl.dsp.dpms

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlDpms](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L297) | none found | monitor (MonitorSelector) | none found | none found |
| current | [hlDpms](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L311) | none found | monitor (MonitorSelector) | none found | none found |

### hl.dsp.event

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlEvent](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L313) | none found | none found | 1:string | none found |
| current | [hlEvent](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L327) | none found | none found | 1:string | none found |

### hl.dsp.exec_cmd

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlExecCmd](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L233) | none found | none found | 1:string | none found |
| current | [hlExecCmd](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L242) | none found | none found | 1:string | none found |

### hl.dsp.exec_raw

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlExecRaw](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L252) | none found | none found | 1:string | none found |
| current | [hlExecRaw](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L261) | none found | none found | 1:string | none found |

### hl.dsp.exit

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlExit](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L262) | none found | none found | none found | none found |
| current | [hlExit](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L271) | none found | none found | none found | none found |

### hl.dsp.focus

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlFocus](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1141) | none found | last (Bool), on_current_monitor (Bool), urgent_or_last (Bool), monitor (MonitorSelector), direction (Str), window (WindowSelector), workspace (WorkspaceSelector) | none found | none found |
| current | [hlFocus](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1091) | none found | last (Bool), on_current_monitor (Bool), urgent_or_last (Bool), monitor (MonitorSelector), direction (Str), window (WindowSelector), workspace (WorkspaceSelector) | none found | none found |

### hl.dsp.force_idle

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlForceIdle](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L338) | none found | none found | 1:number | none found |
| current | [hlForceIdle](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L352) | none found | none found | 1:number | none found |

### hl.dsp.force_renderer_reload

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlForceRendererReload](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L333) | none found | none found | none found | none found |
| current | [hlForceRendererReload](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L347) | none found | none found | none found | none found |

### hl.dsp.global

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGlobal](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L323) | none found | none found | 1:string | none found |
| current | [hlGlobal](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L337) | none found | none found | 1:string | none found |

### hl.dsp.group.active

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGroupActive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L119) | index (Num) | none found | none found | none found |
| current | [hlGroupActive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L130) | index (Num) | none found | none found | none found |

### hl.dsp.group.lock

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGroupLock](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L129) | none found | none found | none found | none found |
| current | [hlGroupLock](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L140) | none found | none found | none found | none found |

### hl.dsp.group.lock_active

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGroupLockActive](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L137) | none found | none found | none found | none found |
| current | [hlGroupLockActive](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L148) | none found | none found | none found | none found |

### hl.dsp.group.move_window

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGroupMoveWindow](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L107) | none found | forward (Bool) | none found | none found |
| current | [hlGroupMoveWindow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L118) | none found | forward (Bool) | none found | none found |

### hl.dsp.group.next

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGroupNext](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L93) | none found | none found | none found | none found |
| current | [hlGroupNext](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L104) | none found | none found | none found | none found |

### hl.dsp.group.prev

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGroupPrev](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L100) | none found | none found | none found | none found |
| current | [hlGroupPrev](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L111) | none found | none found | none found | none found |

### hl.dsp.group.toggle

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGroupToggle](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L87) | none found | none found | none found | none found |
| current | [hlGroupToggle](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L98) | none found | none found | none found | none found |

### hl.dsp.layout

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlLayout](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L287) | none found | none found | 1:string | none found |
| current | [hlLayout](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L301) | none found | none found | 1:string | none found |

### hl.dsp.no_op

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlNoop](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1202) | none found | none found | none found | none found |
| current | [hlNoop](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1152) | none found | none found | none found | none found |

### hl.dsp.pass

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlPass](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L277) | window (WindowSelector) | none found | none found | none found |
| current | [hlPass](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L291) | window (WindowSelector) | none found | none found | none found |

### hl.dsp.release_input_capture

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlReleaseInputCapture](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L348) | none found | none found | none found | none found |
| current | [hlReleaseInputCapture](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L362) | none found | none found | none found | none found |

### hl.dsp.reload_config

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| current | [hlReloadConfig](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L276) | none found | none found | none found | none found |

### hl.dsp.send_key_state

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlSendKeyState](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L456) | key (Str), mods (Str), state (Str) | none found | none found | none found |
| current | [hlSendKeyState](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L418) | key (Str), mods (Str), state (Str) | none found | none found | none found |

### hl.dsp.send_shortcut

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlSendShortcut](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L442) | key (Str), mods (Str) | none found | none found | none found |
| current | [hlSendShortcut](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L404) | key (Str), mods (Str) | none found | none found | none found |

### hl.dsp.submap

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlSubmap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L267) | none found | none found | 1:string | none found |
| current | [hlSubmap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L281) | none found | none found | 1:string | none found |

### hl.dsp.window.alter_zorder

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowAlterZOrder](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1035) | mode (Str) | none found | none found | none found |
| current | [hlWindowAlterZOrder](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L988) | mode (Str) | none found | none found | none found |

### hl.dsp.window.bring_to_top

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowBringToTop](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1030) | none found | none found | none found | none found |
| current | [hlWindowBringToTop](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L983) | none found | none found | none found | none found |

### hl.dsp.window.center

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowCenter](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L971) | none found | none found | none found | none found |
| current | [hlWindowCenter](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L924) | none found | none found | none found | none found |

### hl.dsp.window.clear_tags

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowClearTags](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1011) | none found | none found | none found | none found |
| current | [hlWindowClearTags](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L964) | none found | none found | none found | none found |

### hl.dsp.window.close

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowClose](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L699) | none found | none found | none found | none found |
| current | [hlWindowClose](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L652) | none found | none found | none found | none found |

### hl.dsp.window.cycle_next

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowCycleNext](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L977) | none found | floating (Bool), next (Bool), tiled (Bool) | none found | none found |
| current | [hlWindowCycleNext](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L930) | none found | floating (Bool), next (Bool), tiled (Bool) | none found | none found |

### hl.dsp.window.deny_from_group

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowDenyFromGroup](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1059) | none found | none found | none found | none found |
| current | [hlWindowDenyFromGroup](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1012) | none found | none found | none found | none found |

### hl.dsp.window.drag

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowDrag](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1067) | none found | none found | none found | none found |
| current | [hlWindowDrag](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1020) | none found | none found | none found | none found |

### hl.dsp.window.float

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowFloat](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L721) | none found | none found | none found | none found |
| current | [hlWindowFloat](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L674) | none found | none found | none found | none found |

### hl.dsp.window.fullscreen

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowFullscreen](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L730) | none found | layout_aware (Bool), action (Str), mode (Str) | none found | none found |
| current | [hlWindowFullscreen](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L683) | none found | layout_aware (Bool), action (Str), mode (Str) | none found | none found |

### hl.dsp.window.fullscreen_state

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowFullscreenState](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L782) | none found | layout_aware (Bool), client (Num), internal (Num), action (Str) | none found | none found |
| current | [hlWindowFullscreenState](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L735) | none found | layout_aware (Bool), client (Num), internal (Num), action (Str) | none found | none found |

### hl.dsp.window.kill

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowKill](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L705) | none found | none found | none found | none found |
| current | [hlWindowKill](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L658) | none found | none found | none found | none found |

### hl.dsp.window.move

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowMove](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L824) | none found | follow (Bool), group_aware (Bool), out_of_group (Bool), relative (Bool), monitor (MonitorSelector), x (Num), y (Num), direction (Str), into_group (Str), into_or_create_group (Str), out_of_group (Str), workspace (WorkspaceSelector) | none found | none found |
| current | [hlWindowMove](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L777) | none found | follow (Bool), group_aware (Bool), out_of_group (Bool), relative (Bool), monitor (MonitorSelector), x (Num), y (Num), direction (Str), into_group (Str), into_or_create_group (Str), out_of_group (Str), workspace (WorkspaceSelector) | none found | none found |

### hl.dsp.window.pin

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowPin](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1021) | none found | none found | none found | none found |
| current | [hlWindowPin](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L974) | none found | none found | none found | none found |

### hl.dsp.window.pseudo

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowPseudo](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L815) | none found | none found | none found | none found |
| current | [hlWindowPseudo](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L768) | none found | none found | none found | none found |

### hl.dsp.window.resize

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowResize](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1072) | none found | keep_aspect_ratio (Bool), relative (Bool), x (Num), y (Num) | none found | none found |
| current | [hlWindowResize](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1025) | none found | keep_aspect_ratio (Bool), relative (Bool), x (Num), y (Num) | none found | none found |

### hl.dsp.window.set_prop

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowSetProp](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1046) | prop (Str), value (Str) | none found | none found | none found |
| current | [hlWindowSetProp](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L999) | prop (Str), value (Str) | none found | none found | none found |

### hl.dsp.window.signal

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowSignal](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L711) | signal (Num) | none found | none found | none found |
| current | [hlWindowSignal](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L664) | signal (Num) | none found | none found | none found |

### hl.dsp.window.swap

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowSwap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L924) | none found | next (Bool), prev (Bool), direction (Str), other (WindowSelector), target (WindowSelector), with (WindowSelector) | none found | none found |
| current | [hlWindowSwap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L877) | none found | next (Bool), prev (Bool), direction (Str), other (WindowSelector), target (WindowSelector), with (WindowSelector) | none found | none found |

### hl.dsp.window.tag

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowTag](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1000) | tag (Str) | none found | none found | none found |
| current | [hlWindowTag](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L953) | tag (Str) | none found | none found | none found |

### hl.dsp.window.toggle_swallow

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowToggleSwallow](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1017) | none found | none found | none found | none found |
| current | [hlWindowToggleSwallow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L970) | none found | none found | none found | none found |

### hl.dsp.workspace.change_id

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWorkspaceChangeID](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1296) | id (Num), workspace (WorkspaceSelector) | none found | none found | none found |
| current | [hlWorkspaceChangeID](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1246) | id (Num), workspace (WorkspaceSelector) | none found | none found | none found |

### hl.dsp.workspace.move

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWorkspaceMove](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1312) | monitor (MonitorSelector) | workspace (WorkspaceSelector) | none found | none found |
| current | [hlWorkspaceMove](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1262) | monitor (MonitorSelector) | workspace (WorkspaceSelector) | none found | none found |

### hl.dsp.workspace.rename

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWorkspaceRename](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1280) | workspace (WorkspaceSelector) | name (Str) | none found | none found |
| current | [hlWorkspaceRename](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1230) | workspace (WorkspaceSelector) | name (Str) | none found | none found |

### hl.dsp.workspace.swap_monitors

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWorkspaceSwapMonitors](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1331) | monitor1 (MonitorSelector), monitor2 (MonitorSelector) | none found | none found | none found |
| current | [hlWorkspaceSwapMonitors](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1281) | monitor1 (MonitorSelector), monitor2 (MonitorSelector) | none found | none found | none found |

### hl.dsp.workspace.toggle_special

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWorkspaceToggleSpecial](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1274) | none found | none found | none found | none found |
| current | [hlWorkspaceToggleSpecial](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsDispatchers.cpp#L1224) | none found | none found | none found | none found |

### hl.env

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlEnv](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L478) | none found | none found | none found | none found |
| current | [hlEnv](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L483) | none found | none found | none found | none found |

### hl.exec_cmd

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlExecCmd](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L321) | none found | none found | none found | none found |
| current | [hlExecCmd](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L316) | none found | none found | none found | none found |

### hl.exec_scheduled_prop_refresh_immediately

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlExecuteScheduledRefreshImmediately](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L530) | none found | none found | none found | none found |
| current | [hlExecuteScheduledRefreshImmediately](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L511) | none found | none found | none found | none found |

### hl.gesture

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGesture](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L737) | none found | none found | none found | action, disable_inhibit, finish, mods, scale, start, update |
| current | [hlGesture](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L738) | none found | none found | none found | action, disable_inhibit, finish, mods, scale, start, update |

### hl.get_active_monitor

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetActiveMonitor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L225) | none found | none found | none found | none found |
| current | [hlGetActiveMonitor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L283) | none found | none found | none found | none found |

### hl.get_active_special_workspace

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetActiveSpecialWorkspace](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L193) | none found | none found | none found | none found |
| current | [hlGetActiveSpecialWorkspace](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L197) | none found | none found | none found | none found |

### hl.get_active_window

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetActiveWindow](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L125) | none found | none found | none found | none found |
| current | [hlGetActiveWindow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L129) | none found | none found | none found | none found |

### hl.get_active_workspace

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetActiveWorkspace](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L182) | none found | none found | none found | none found |
| current | [hlGetActiveWorkspace](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L186) | none found | none found | none found | none found |

### hl.get_config

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetConfig](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1020) | none found | none found | 1:string | none found |
| current | [hlGetConfig](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1027) | none found | none found | 1:string | none found |

### hl.get_current_submap

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetCurrentSubmap](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L387) | none found | none found | none found | none found |
| current | [hlGetCurrentSubmap](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L445) | none found | none found | none found | none found |

### hl.get_cursor_pos

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetCursorPos](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L282) | none found | none found | none found | none found |
| current | [hlGetCursorPos](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L340) | none found | none found | none found | none found |

### hl.get_devices

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| current | [hlGetDevices](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L239) | none found | none found | none found | none found |

### hl.get_last_window

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetLastWindow](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L298) | none found | none found | none found | none found |
| current | [hlGetLastWindow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L356) | none found | none found | none found | none found |

### hl.get_last_workspace

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetLastWorkspace](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L318) | none found | none found | none found | none found |
| current | [hlGetLastWorkspace](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L376) | none found | none found | none found | none found |

### hl.get_layers

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetLayers](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L343) | none found | none found | none found | none found |
| current | [hlGetLayers](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L401) | none found | none found | none found | none found |

### hl.get_loaded_plugins

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetPlugins](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L294) | none found | none found | none found | none found |
| current | [hlGetPlugins](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L289) | none found | none found | none found | none found |

### hl.get_monitor

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetMonitor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L214) | none found | none found | none found | none found |
| current | [hlGetMonitor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L272) | none found | none found | none found | none found |

### hl.get_monitor_at

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetMonitorAt](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L236) | none found | x (Num), y (Num) | 1:number, 2:number | none found |
| current | [hlGetMonitorAt](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L294) | none found | x (Num), y (Num) | 1:number, 2:number | none found |

### hl.get_monitor_at_cursor

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetMonitorAtCursor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L271) | none found | none found | none found | none found |
| current | [hlGetMonitorAtCursor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L329) | none found | none found | none found | none found |

### hl.get_monitors

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetMonitors](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L204) | none found | none found | none found | none found |
| current | [hlGetMonitors](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L208) | none found | none found | none found | all |

### hl.get_urgent_window

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetUrgentWindow](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L147) | none found | none found | none found | none found |
| current | [hlGetUrgentWindow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L151) | none found | none found | none found | none found |

### hl.get_window

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetWindow](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L136) | none found | none found | none found | none found |
| current | [hlGetWindow](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L140) | none found | none found | none found | none found |

### hl.get_windows

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetWindows](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L110) | none found | none found | none found | none found |
| current | [hlGetWindows](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L114) | none found | none found | none found | none found |

### hl.get_workspace

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetWorkspace](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L171) | none found | none found | none found | none found |
| current | [hlGetWorkspace](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L175) | none found | none found | none found | none found |

### hl.get_workspace_windows

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetWorkspaceWindows](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L376) | none found | none found | none found | none found |
| current | [hlGetWorkspaceWindows](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L434) | none found | none found | none found | none found |

### hl.get_workspaces

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetWorkspaces](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsQuery.cpp#L158) | none found | none found | none found | none found |
| current | [hlGetWorkspaces](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsQuery.cpp#L162) | none found | none found | none found | none found |

### hl.is_key_down

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlIsKeyDown](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L418) | none found | none found | none found | none found |
| current | [hlIsKeyDown](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L413) | none found | none found | none found | none found |

### hl.layer_rule

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlLayerRule](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1277) | none found | none found | none found | enabled, match, name |
| current | [hlLayerRule](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1284) | none found | none found | none found | enabled, match, name |

### hl.layout.register

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlLayoutRegister](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/layout/LuaLayoutProvider.cpp#L306) | none found | none found | 1:string | none found |
| current | [hlLayoutRegister](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/layout/LuaLayoutProvider.cpp#L306) | none found | none found | 1:string | none found |

### hl.monitor

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlMonitor](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1103) | none found | none found | none found | output |
| current | [hlMonitor](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1110) | none found | none found | none found | output |

### hl.notification.create

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlNotificationCreate](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsNotification.cpp#L66) | text (Str) | duration (Num), font_size (Num), time (Num), timeout (Num) | none found | color, icon |
| current | [hlNotificationCreate](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsNotification.cpp#L66) | text (Str) | duration (Num), font_size (Num), time (Num), timeout (Num) | none found | color, icon |

### hl.notification.get

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlGetNotifications](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsNotification.cpp#L118) | none found | none found | none found | none found |
| current | [hlGetNotifications](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsNotification.cpp#L118) | none found | none found | none found | none found |

### hl.on

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlOn](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L377) | none found | none found | 1:string | none found |
| current | [hlOn](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L372) | none found | none found | 1:string | none found |

### hl.permission

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlPermission](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L556) | none found | binary (Str), mode (Str), target (Str), type (Str) | 1:string, 2:string, 3:string | none found |
| current | [hlPermission](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L561) | none found | binary (Str), mode (Str), target (Str), type (Str) | 1:string, 2:string, 3:string | none found |

### hl.plugin.load

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlPluginLoad](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L537) | none found | none found | none found | none found |
| current | [hlPluginLoad](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L542) | none found | none found | none found | none found |

### hl.timer

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlTimer](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L460) | none found | none found | none found | timeout, type |
| current | [hlTimer](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L441) | none found | none found | none found | timeout, type |

### hl.unbind

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlUnbind](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L404) | none found | none found | 1:string | none found |
| current | [hlUnbind](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L399) | none found | none found | 1:string | none found |

### hl.version

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlVersion](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsToplevel.cpp#L289) | none found | none found | none found | none found |
| current | [hlVersion](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsToplevel.cpp#L284) | none found | none found | none found | none found |

### hl.window_rule

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWindowRule](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1167) | none found | none found | none found | enabled, match, name |
| current | [hlWindowRule](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L1174) | none found | none found | none found | enabled, match, name |

### hl.workspace_rule

| Version | Handler source | Required reads in branches | Optional reads | Positional checks | Other literal fields |
|---|---|---|---|---|---|
| baseline | [hlWorkspaceRule](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L624) | none found | none found | none found | enabled, workspace |
| current | [hlWorkspaceRule](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/bindings/LuaBindingsConfigRules.cpp#L629) | none found | none found | none found | enabled, workspace |

## Runtime object method argument facts

Method callsites and direct helper facts are retained in the JSON, including templated rule helpers. The production handle-method node must still certify argument semantics, expired-handle behavior and return types.

| Method | Version | Handler / expression | Source |
|---|---|---|---|
| HL.EventSubscription:is_active | baseline | eventSubscriptionIsActive | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaEventSubscription.cpp#L45) |
| HL.EventSubscription:is_active | current | eventSubscriptionIsActive | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaEventSubscription.cpp#L45) |
| HL.EventSubscription:remove | baseline | eventSubscriptionRemove | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaEventSubscription.cpp#L34) |
| HL.EventSubscription:remove | current | eventSubscriptionRemove | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaEventSubscription.cpp#L34) |
| HL.Group:add | baseline | groupAdd | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaGroup.cpp#L77) |
| HL.Group:add | current | groupAdd | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaGroup.cpp#L78) |
| HL.Group:remove | baseline | groupRemove | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaGroup.cpp#L109) |
| HL.Group:remove | current | groupRemove | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaGroup.cpp#L110) |
| HL.Keybind:is_enabled | baseline | keybindIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaKeybind.cpp#L57) |
| HL.Keybind:is_enabled | current | keybindIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaKeybind.cpp#L71) |
| HL.Keybind:remove | baseline | keybindRemove | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaKeybind.cpp#L68) |
| HL.Keybind:remove | current | keybindRemove | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaKeybind.cpp#L82) |
| HL.Keybind:set_enabled | baseline | keybindSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaKeybind.cpp#L46) |
| HL.Keybind:set_enabled | current | keybindSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaKeybind.cpp#L60) |
| HL.Keybind:unbind | baseline | keybindRemove | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaKeybind.cpp#L68) |
| HL.Keybind:unbind | current | keybindRemove | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaKeybind.cpp#L82) |
| HL.LayerRule:is_enabled | baseline | layerRuleIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaLayerRule.cpp#L45) |
| HL.LayerRule:is_enabled | current | layerRuleIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaLayerRule.cpp#L45) |
| HL.LayerRule:set_enabled | baseline | layerRuleSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaLayerRule.cpp#L32) |
| HL.LayerRule:set_enabled | current | layerRuleSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaLayerRule.cpp#L32) |
| HL.Monitor:set_special_workspace | baseline | monitorSetSpecialWorkspace | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaMonitor.cpp#L54) |
| HL.Monitor:set_special_workspace | current | monitorSetSpecialWorkspace | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaMonitor.cpp#L63) |
| HL.Monitor:set_workspace | baseline | monitorSetWorkspace | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaMonitor.cpp#L38) |
| HL.Monitor:set_workspace | current | monitorSetWorkspace | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaMonitor.cpp#L42) |
| HL.Notification:dismiss | baseline | notificationDismiss | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L219) |
| HL.Notification:dismiss | current | notificationDismiss | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L219) |
| HL.Notification:get_color | baseline | notificationGetColor | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L254) |
| HL.Notification:get_color | current | notificationGetColor | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L254) |
| HL.Notification:get_elapsed | baseline | notificationGetElapsed | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L293) |
| HL.Notification:get_elapsed | current | notificationGetElapsed | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L293) |
| HL.Notification:get_elapsed_since_creation | baseline | notificationGetElapsedSinceCreation | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L306) |
| HL.Notification:get_elapsed_since_creation | current | notificationGetElapsedSinceCreation | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L306) |
| HL.Notification:get_font_size | baseline | notificationGetFontSize | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L280) |
| HL.Notification:get_font_size | current | notificationGetFontSize | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L280) |
| HL.Notification:get_icon | baseline | notificationGetIcon | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L267) |
| HL.Notification:get_icon | current | notificationGetIcon | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L267) |
| HL.Notification:get_text | baseline | notificationGetText | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L228) |
| HL.Notification:get_text | current | notificationGetText | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L228) |
| HL.Notification:get_timeout | baseline | notificationGetTimeout | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L241) |
| HL.Notification:get_timeout | current | notificationGetTimeout | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L241) |
| HL.Notification:is_alive | baseline | notificationIsAlive | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L319) |
| HL.Notification:is_alive | current | notificationIsAlive | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L319) |
| HL.Notification:is_paused | baseline | notificationIsPaused | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L144) |
| HL.Notification:is_paused | current | notificationIsPaused | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L144) |
| HL.Notification:pause | baseline | notificationPause | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L104) |
| HL.Notification:pause | current | notificationPause | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L104) |
| HL.Notification:resume | baseline | notificationResume | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L119) |
| HL.Notification:resume | current | notificationResume | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L119) |
| HL.Notification:set_color | baseline | notificationSetColor | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L180) |
| HL.Notification:set_color | current | notificationSetColor | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L180) |
| HL.Notification:set_font_size | baseline | notificationSetFontSize | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L206) |
| HL.Notification:set_font_size | current | notificationSetFontSize | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L206) |
| HL.Notification:set_icon | baseline | notificationSetIcon | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L193) |
| HL.Notification:set_icon | current | notificationSetIcon | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L193) |
| HL.Notification:set_paused | baseline | notificationSetPaused | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L134) |
| HL.Notification:set_paused | current | notificationSetPaused | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L134) |
| HL.Notification:set_text | baseline | notificationSetText | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L157) |
| HL.Notification:set_text | current | notificationSetText | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L157) |
| HL.Notification:set_timeout | baseline | notificationSetTimeout | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaNotification.cpp#L167) |
| HL.Notification:set_timeout | current | notificationSetTimeout | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaNotification.cpp#L167) |
| HL.Timer:is_enabled | baseline | timerIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaTimer.cpp#L72) |
| HL.Timer:is_enabled | current | timerIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaTimer.cpp#L72) |
| HL.Timer:set_enabled | baseline | timerSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaTimer.cpp#L37) |
| HL.Timer:set_enabled | current | timerSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaTimer.cpp#L37) |
| HL.Timer:set_timeout | baseline | timerSetTimeout | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaTimer.cpp#L53) |
| HL.Timer:set_timeout | current | timerSetTimeout | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaTimer.cpp#L53) |
| HL.WindowRule:is_enabled | baseline | windowRuleIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaWindowRule.cpp#L45) |
| HL.WindowRule:is_enabled | current | windowRuleIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaWindowRule.cpp#L45) |
| HL.WindowRule:set_enabled | baseline | windowRuleSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaWindowRule.cpp#L32) |
| HL.WindowRule:set_enabled | current | windowRuleSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaWindowRule.cpp#L32) |
| HL.Workspace:get_groups | baseline | workspaceGetGroups | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaWorkspace.cpp#L64) |
| HL.Workspace:get_groups | current | workspaceGetGroups | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaWorkspace.cpp#L69) |
| HL.Workspace:get_windows | baseline | workspaceGetWindows | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaWorkspace.cpp#L45) |
| HL.Workspace:get_windows | current | workspaceGetWindows | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaWorkspace.cpp#L50) |
| HL.WorkspaceRule:is_enabled | baseline | workspaceRuleIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaWorkspaceRule.cpp#L45) |
| HL.WorkspaceRule:is_enabled | current | workspaceRuleIsEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaWorkspaceRule.cpp#L45) |
| HL.WorkspaceRule:set_enabled | baseline | workspaceRuleSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/lua/objects/LuaWorkspaceRule.cpp#L32) |
| HL.WorkspaceRule:set_enabled | current | workspaceRuleSetEnabled | [source](https://github.com/hyprwm/Hyprland/blob/6a1b5f0fbfa169d4e6cf2f49a5f919a273aeaec7/src/config/lua/objects/LuaWorkspaceRule.cpp#L32) |
## Certification boundary

The manifest is an engineering assignment map. It cannot be loaded as a ready production schema: defaults/bounds/overloads, component classifications, exact runtime behavior and source-preserving codecs require the specified certification gates. Unsupported or opaque input is preserved and reported, not secretly executed or treated as complete.
