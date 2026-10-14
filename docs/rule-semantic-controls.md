# Structured rule values — 0.10

This milestone replaces encoded text entry for opacity, fullscreen state, grouping, suppressed events, idle inhibition, content type, tone mapping and active/inactive border gradients. The editor still authors private drafts; no compositor, command, hook or plugin is activated.

## Controls and preservation

- Opacity has active, inactive and fullscreen amounts, common-range sliders and independent override controls. Numeric inputs retain the finite float32 domain. A one-amount imported value still means all three channels; two amounts leave fullscreen at the upstream default. Untouched spelling, including the optional `opacity` token, stays exact.
- Fullscreen state has separate internal and optional client numbers. The parser's 32-bit integer domain, absent client state and ignored trailing fields are distinguished. Hexadecimal and unusual separator spellings stay unchanged until an edit.
- Grouping uses ordered operations with move/add/remove controls. Set/New, Lock, Invade, Barred, Deny, Override, Clear and Always are distinct. Override resets flags but continues the current list; Clear stops it. A prior override blocks later group declarations. Unsupported/diagnosed forms remain literals.
- Suppressed event checkboxes cover the six consumed event names. Mode selectors cover idle inhibition, content and tone mapping. Numeric aliases are retained without normalization on an untouched submit.
- Border controls have separate active and optional inactive palettes, alpha-capable color pickers, a ten-color cap per gradient and whole-degree direction controls. Baseline requires active colors; the newer snapshot permits inactive-only colors. Imported typed gradients and legacy strings keep their original representation until edited. Values outside the structured decoder remain preserved rather than being clamped or silently rewritten.

Only the selected rule effect form is built. Name/enabled metadata is collapsed, effect navigation shares one row and opacity controls use compact channel rows so all three fit on the ordinary editor viewport. Unknown original values expose an explicit replacement switch and a collapsed original-value view. Form switching, update/save/close guards and the existing schema-pinned snapshot/history behavior remain integrated. The saved wire shape is unchanged: specialized values still serialize as literal strings or existing border representations, so old workspaces do not require a migration for these forms.

## Evidence

`data/rule-codec-provenance.json` pins reviewed source files from both audited Hyprland commits and Hyprutils v0.14.2 (`a21e87b878e72ee5dfd375899e8b2ad0d0c4c0e1`). `scripts/rule_codec_source_probe.py --check` fetches hash-checked sources into a temporary directory, compiles the numeric/list/color parsers and extracted rule functions, and compares 120 stored observations. These cover defaults, replication/override, extra/empty tokens, 32-bit integer bounds, float32 underflow/overflow, color-channel ordering, border cardinality/version behavior and grouping order/initial flags.

The source probe uses actual pinned numeric/list helpers, the actual color parser and group consumer logic. Gradient/color storage objects are inert. A fixture-only string/string-view concatenation compatibility operator supports the C++23 compiler; the simple public color-string preparation path is an original wrapper, not the full Lua descriptor implementation. This is primitive parsing evidence, not a renderer, Lua ABI, compositor, dependency-matrix or hardware certificate. Escaped lists, exotic numeric/color spellings and non-finite values remain original literals when unsupported by the structured decoder.

Rust fixtures compare typed decoder results with these observations. Native GTK tests exercise unchanged known/unknown values, opacity changes, border version controls, ordering and disposal of closed forms. The existing rule tests also exercise the new forms through declaration management and static export/reopen.

## Remaining full-scope work

Complete escaped-list and color domains, graphical geometry expressions, monitor/workspace/action selectors, animation styles, named-rule lifecycle, source codecs, effective discovery, real compositor interoperability, isolated preview, recovery/apply, accessibility and repeatable resource acceptance remain required. All original full audit contracts stay unproven. These controls deepen existing RULE foundations; they do not increase the count of feature contracts with partial implementations.
