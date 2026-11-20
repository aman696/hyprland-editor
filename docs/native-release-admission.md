# Native Lua release admission

Both native adapters now check the linked library's constant identity against the exact compiled header release before creating any Lua state. The shared check compares the `$LuaVersion: Lua x.y.z ` prefix first. Only a matching prefix permits the `lua_version` call. That ordering matters because even the version API can have a different signature in another Lua family. Every syntax compiler, fixed bytecode probe and primitive-state constructor goes through the guarded state factory. The Lua 5.5 factory checks before requesting a random seed.

This corrects a demonstrated private harness hazard: generic archive resolution previously paired Lua 5.4 headers with an actual Lua 5.5 library. The harness itself was fixed with uniquely named archives, but an adapter should independently refuse mismatched dependencies before using its header-specific constructor. The legacy two-argument and separate three-argument constructors remain distinct. Rejection uses the existing unavailable-state path; it cannot become successful source compilation or primitive conversion.

The identity check is a release admission guard, not a complete ABI, publisher, build-option, arithmetic or dependency-security certificate. A matching version string alone cannot establish every ABI/build property. The existing exact official archive checks, native-width assertions, compiler fingerprints, root package channel and finite references retain their separate scopes. Backports retaining the same release identity still use actual numeric compiler observations; unknown traits remain unknown.

## Independent mismatched-library test

`scripts/lua_admission_probe.py` downloads hash-pinned official Lua 5.4.6/.7/.8/.9 and 5.5.0/.1 into one temporary scope and links all 36 header/library combinations using GNU ld.bfd. It uses the real library's exported constant identity. Wrapped constructor/version functions count calls without forwarding to a potentially incompatible implementation. No state is created and no imported source runs.

For each exact match, the harness reaches the instrumented constructor three times (compile, fixed dump probe and primitive-state creation) and the version stub four times, including the explicit initial query. For all 30 mismatched pairs, both counters must remain zero. The constructor stub deliberately returns null, and each bounded entry must retain its unavailable result and empty output. Actual successful compilation/conversion remains covered separately by the four legacy release suites and the two ordinary/static worker/parent suites.

The local matrix passes all 36 pairs. Temporary sources, archives, object files and images are removed. The recorded metadata binds header/library releases, official library archive hashes and exact adapter/test source hashes. This is finite pre-constructor admission evidence, not a hostile-process or full compiler certificate.

## Native linker order

The core now emits each native adapter before the RE2/Lua libraries it calls, then rechecks package version/header context when emitting those dependencies. This extends the compiler-worker GNU ld correction to the legacy GUI/core path. An actual full ordinary workspace run passes with explicit GNU ld.bfd in a private build directory, independently of the default Rust linker. Its private target is removed. Other platform/build/dependency combinations remain subject to their own acceptance checks.

## Existing saved graphs

The 80-input model now has eighteen exact reviewed predecessors. The numeric-model snapshot is frozen in `data/migrations/graph-model-numeric-codegen.json`; only the core build, legacy adapter and shared guard differ. Fresh restricted reconstruction remains mandatory.

Earlier writers that predate numeric observations must still omit that field, including refusing injected null. The preceding numeric writer must retain its exact current compiler observation; migration cannot erase or renew it. Its existing numeric profile, captured/private bytes, typed values and resolution evidence remain unchanged. All eighteen predecessors and six preceding private-layer writers are checked across both audited Hyprland snapshots, including active and archived inputs. Reversing reviewed metadata changes must still recover the exact decoded input; no original path is opened or folder grant restored.

GUI/source evaluation remains Lua 5.4. Full Lua 5.5 language/ordered/native API integration, source-profile selection/migration, all-family graphical editing/export, actual rendered preview, live apply/recovery and every original security/resource/accessibility/packaging/release obligation remain required.
