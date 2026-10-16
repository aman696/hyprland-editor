# Ordered immutable-module evaluation

`lua::replay` is the source-graph branch's ordered evaluator. It operates on supplied, immutable document bytes under explicit assumptions. The GUI still captures files for temporary review; it does not yet construct this evaluator's bundle, import its observations into drafts or save a multi-file workspace. All original audit obligations remain. No full feature or security gate is certified by this increment.

## Inputs and evidence

A program names an audited Hyprland snapshot, model profile, root document, optional initial `package.path`, optional working directory, documents and resolution bindings. Documents carry stable IDs, exact source hashes and byte-oriented load names. A resolution request includes the calling document/hash/span, loader kind, evaluated module bytes and current path/context. Bindings match that complete request. Duplicate or stale bindings and invalid target identities are rejected.

Bindings are supplied observations or hypotheses. They do not prove path search order, file absence, ownership, containment, normalized-load identity or authorization. A bundle can describe a hypothetical module substitution. `runtime_verified` is always false. A missing binding halts with `NeedsResolution`; absence of information is never treated as a missing file. An explicitly supplied miss can produce a known protected-call failure, but its actual diagnostic remains unknown. No file is read by this evaluator.

The production profile conservatively leaves uncaptured global/package/library fields unknown. The separate synthetic `emit` profile fixes a private test environment. Production must not infer an absent native global from that test environment.

## Order and identity

Lexical environments hold shared cells, while each module receives its own local environment. Closures capture bindings visible at creation, retain outer mutations and do not acquire later local declarations. Tables and closures have identity; returning or caching a table shares that object. Statement order, evaluated function calls, argument/result adjustment, varargs, branches, lexical blocks, integer numeric loops, while/repeat loops and returns are modeled directly from syntax trees. Source files are never concatenated.

Require uses the existing source-derived cache transitions with a thin view of the requested key. It distinguishes the registry cache from rebound global `package.loaded`, false misses, nil completion, preloads, original require and supported safe-error fallback. `dofile` invokes the supplied document afresh; `loadfile` returns a fresh closure without invoking its body. Model uncertainty and resource failures propagate outside Lua error handling, including through `pcall`.

API argument observations and loader edges share a monotonically increasing order. Edges distinguish a cache hit, preload, supplied snapshot, supplied miss and unresolved request. Each retains the calling document/hash/span. A supplied snapshot means a selected input to the model, not a verified or successfully executed physical file. `hl.config` is currently an inert argument observer: it does not implement actual binding conversions, register configuration, prove acceptance or discover effective settings. Other Hyprland APIs still need their audited codecs connected to ordered evaluation.

## Compilation and isolation

Tree-sitter supplies the syntax nodes. Before a document is entered, its compilation result is checked using the system Lua 5.4 compiler. This catches errors such as assigning a constant or using varargs in a non-variadic function before any statement can be observed. The compiler's [load API does not run the chunk](https://www.lua.org/manual/5.4/manual.html#luaL_loadbufferx). The adapter opens no libraries, accepts text only, uses an 8 MiB allocator and never invokes the returned function. An allocation failure or a diagnostic exceeding 4 KiB is a budget failure, not an error safe require can hide. Compilation results for provided documents are currently prepared up front; unused syntax errors are not reported as entered-module errors.

The application exposes this only through `ReplayModules` in its existing no-filesystem parser role. Environment clearing, closed inherited descriptors, no-new-privileges, closed syscall policy, 512 MiB address-space / 8 MiB stack / two-second CPU limits and the parent's three-second post-spawn deadline remain in force. The compiler introduces a native dependency and attack surface; fuzzing, sanitizer coverage and dependency-profile certification remain required. The worker has no preview, file grant, compositor connection or publication capability.

## Editor budgets

- At most 32 documents, 1 MiB per source and 4 MiB cumulative source; 512 supplied bindings. The worker additionally caps its entire JSON request at 4 MiB, including encoding overhead.
- Module names 16 KiB, search-path text 64 KiB, working-directory/load-name bytes 4 KiB. Unsupported NUL/path domains are rejected or left unresolved.
- 100,000 operation visits, 4,096 tables, 16,384 table entries/cells and 1,024 closures; expression/block/call depth limits of 128/64/32.
- Individual string values 64 KiB and cumulative string/name allocation 4 MiB. Environment copying charges operation visits.
- At most 2,048 API observations and 512 loader edges. Aggregate estimated output is 2 MiB, charging recursively expanded aliases and repeated loader-path bytes before retaining results. Snapshot depth is 32. Cycles use explicit references rather than recursive expansion. Transport responses remain capped at 4 MiB.

These are explicit editor limits, not Hyprland semantic limits or a measured whole-process memory certificate. Filesystem startup/I/O deadlines, GUI resource targets and large real configurations still need separate evidence.

## Verification and remaining work

`scripts/ordered_module_probe.py --check` fetches exact hash-pinned Hyprland loader helpers into memory and compiles them in a temporary directory with real Lua 5.4. Only private trusted fixture programs execute. Ninety observations across the two audited snapshots are stored in `data/ordered-module-fixtures.json`, including source/module bytes and the audit-manifest hash. The inert `hl.config` fixture records its argument only; it does not stand in for field codecs. Temporary upstream originals are deleted on exit.

Core tests compare ordered traces, protected failure outcomes and error counts against those observations. Additional checks cover source/context identity, interleaved loader/API provenance, cyclic/shared values, repeated-path and table expansion, compile-before-observation, unknown native effects and control-flow budgets. Actual restricted-worker tests exercise compilation, shared module state, denied filesystem/network access, inert commands, malformed identities and bounded recursion/output.

Unimplemented semantics stay explicit: wildcard capture/replay, generic iteration, arbitrary custom/native loaders, metatables, complete standard libraries, all Lua byte-string/numeric/coercion domains, full error-stack/diagnostic formatting and complete native global state. They must be implemented or accurately represented as opaque dependencies before broad configuration support can be claimed. Captured graph construction, typed resolution/miss evidence, full per-value provenance, all feature codecs, multi-file save/reopen/history/reconciliation, lossless source editing, effective discovery, isolated compositor preview, independent recovery, guarded apply and resource/security/release certification remain required.
