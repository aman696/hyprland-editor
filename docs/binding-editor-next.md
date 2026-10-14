# Binding implementation handoff

The original B1/G1 and workflow contracts remain authoritative. This handoff records source checks made after the device milestone; it does not count as a working binding editor or a release certificate.

The binding/toplevel and dispatcher source files were fetched at both exact audit commits. Their SHA-256 values were compared with every matching handler record in `hyprland-implementation-contracts.json`. The baseline has 51 `hl.dsp` constructor contracts; audited-main has 52, adding `hl.dsp.reload_config`. The mouse bind option is also current-only. Neither may be offered/exported into the older pack blindly.

The baseline key parser accepts `+`-separated tokens, leading uppercase modifier keywords/aliases, multiple ordinary keysyms or numeric `code:` tokens, and a single special mouse/wheel/switch token. Ordinary keysyms are validated through libxkbcommon's case-insensitive lookup. `catchall` is restricted to submaps. Modifiers after keys and combinations of special/ordinary keys are errors. The current handler delegates key parsing to its separate manager, which still needs a direct source/fixture comparison before sharing one codec.

The next implementation must supply a typed chord builder, focused GTK recording with blur/close/timeout cleanup, a key catalogue for reserved shortcuts, version-scoped flags and description aliases, inclusive/exclusive device/tag targeting, ordered actions and nested submaps with visible reset routes. Recording cannot infer a physical device or a keycode offset. Click/drag are mutually exclusive and imply release; repeating conflicts with long-press/release, and mouse incompatibilities follow the selected handler.

The action library must cover all 51 baseline / 52 current constructors. Lexical argument facts are discovery inputs, not complete signatures. Read helper behavior, defaults, aliases, optional object references, enum conversion and branch-specific arguments before producing dedicated builders and meaningful accepted/rejected fixtures. External command, exit/reload, input injection, power and other effects must be classified; editing/import must never execute them, and ordinary preview must mock or explicitly omit them.

Device category hints are not capability observations. Device-filter conflict checks need normalized name/tag-set intersection, inclusive/exclusive semantics, submap/flag conditions and source order. Preserve intentionally ordered multi-action bindings and leave ambiguous runtime overlap unknown. Tag changes must eventually update referencing filters; the current device editor does not do this.

Persistence, source-aware review/undo, static projection and lossless source codecs must be integrated without silently dropping opaque callbacks or unsupported key forms. The initial monitor/device milestones still need hardware/effective-state inspection and source-span editing; do not mark those original obligations complete while adding bindings.
