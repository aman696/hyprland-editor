# Separate Lua 5.5 source analysis

The `hyprland-editor-source55` image now links the ordered, non-executing source model to an explicitly selected Lua 5.5 adapter. The default GUI and saved-workspace reconstruction still use Lua 5.4. This is a separately built development component; no GUI action, saved path or request chooses its executable, and its installed launcher/package admission is unfinished.

## Build and family separation

Build each image independently. The core defaults to `native54`; `native55` requires disabling defaults. Missing or combined native families fail in the build script before library selection and in the Rust library. The separate compiler-only image continues to have its own symbols and no core/RE2 dependency.

```sh
cargo build --locked -p hyprland-editor-core --no-default-features --features native55 --release --bin hyprland-editor-source55
```

The source image requires the reviewed `lua5.5` package release 5.5.0 or 5.5.1, uses the existing seeded three-argument constructor and pre-constructor release guard, and compiles the complex text adapter once. The original Lua 5.4 profile schema is unchanged. Rust native calls bind explicitly to the selected family's symbols. RE2 remains a source-model dependency; GTK/libadwaita and the other Lua family are excluded. This does not establish complete dependency ABI, package authenticity or selected-compositor compatibility.

## Closed analysis role

The image requires its exact `--source-worker` argument and an empty environment. Before reading input it requires ordinary matching user/group identities and no retained Linux capabilities, closes descriptors above stderr, applies no-new-privileges, installs the existing closed compiler syscall policy, and sets 256 MiB address-space, two-second CPU, eight MiB stack, zero core and sixteen-descriptor limits. Stdin/stdout use nonblocking I/O with one three-second wall deadline and four MiB wire bounds. These are containment limits, not representative performance measurements or a complete sandbox certificate.

Protocol `hye-lua55-source-1` accepts only profile, probe and analyze requests. Every reply binds the 32-byte nonce and SHA-256 of the exact request. Analyze requires the exact compiler observation, including the actual library identity, schema, text and numeric compiler fingerprints, and an explicit `hyprland_lua55` program. Legacy and synthetic profiles are refused at this entry point. The core independently refuses the wrong native family. Documents retain their original hashes, load names, captured dependency bindings and existing model budgets; no filesystem resolution is inferred.

The native adapter compiles imported source for syntax and provides bounded primitive conversions. It never invokes the imported chunk. The Rust model produces ordered API/loader/value/provenance observations. Unknown syntax, APIs, native state or compiler behavior remain explicit barriers. The report retains `runtime_verified=false`; it is not an effective-state observation.

## Version-specific semantics and limits

The current Tree-sitter grammar recognizes contextual `global` declarations and also emits an unnamed contextual identifier terminal. The native55 child traversal retains that terminal in value/argument lists. Its evaluator handles that identifier in reads, writes, parameters, literal table keys and numeric/provenance classification. A named declaration shadows outer local bindings without replacing its `_ENV` field; old closures retain their captured cells. A wildcard declaration changes only native compile-time defaults. Native compilation checks declared-name and constant restrictions before modeled effects, including read-only numeric/generic control variables. Direct environment writes retain their normal modeled behavior.

Global variable/function initialization remains unresolved: existing-slot checks, initializer ordering and unknown environment state need independent runtime references before implementation. Generic iteration order, local attributes and other pre-existing unsupported language features retain their barriers. This component does not claim complete Lua 5.5 grammar/evaluation or all native Hyprland API routing.

Lua 5.5 converts a nil error object to `<no error object>`. The model applies that conversion after `error`'s optional string-location handling; a false error remains false. The Lua 5.4 path retains its original behavior. Numeric compilation uses the actual fixed non-executing fingerprint; restricted native55 model returns are independently compared with original and narrowly patched upstream compilers.

## Validation and saved compatibility

`scripts/workspace_target_profile_probe.py --worker-only --source-worker-conformance` privately builds both hash-pinned official releases, rejects absent/combined feature families, runs the actual source-process suite and existing ordered/primitive/complex model tests, then checks strict lint, the optimized image's dependencies and exact profile reply. The process suite covers both audited Hyprland snapshots, exact compiler/profile refusal, all sixteen existing native55 language cases with explicit initialization/iteration barriers, private dependency replay and tamper refusal, denied filesystem/network/process syscalls, inherited descriptor closure, malformed/oversized input and blocked input/output deadlines. Temporary upstream sources and builds are removed on success or failure. Both official releases passed 35 Rust cases each, strict role lint and optimized ELF/profile checks under explicit GNU ld.bfd. The source/image identities are recorded in [the local observation](../data/lua55-source-worker-observation.json). The fixture process is not an installed production launcher or GUI profile-selection proof.

The new 83-input model has nineteen exact reviewed predecessors. The native-admission predecessor is frozen as `graph-model-native-admission.json`. The transition changes only explicit native family/build/API routing, the separately restricted source image and native55-only language branches. Existing numeric observations must remain exact for both preceding numeric-aware writers; earlier writers retain only their already reviewed observation-renewal path. No compiler family or release migration is permitted. Saved Lua 5.4 bytes and private active/archive data must reconstruct independently, without opening original paths or restoring folder access.

Full source-profile GUI selection, signed/installed source-worker admission, all-family editing/export, workspace primary/result routing, actual isolated renderer/capture/input, guarded apply/recovery, representative resources, accessibility, packaging and every original audit/security/release obligation remain required.
