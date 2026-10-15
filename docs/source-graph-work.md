# Source graph implementation work

This branch starts E03/E05. It is not yet a multi-file editor, and it does not relax the application's refusal to reopen unsupported multi-file workspaces. The full audit remains authoritative; no additional feature contract or security gate is marked complete.

## Implemented candidate planner

`module_path::Context` carries the exact main filename, optional captured home path and optional inherited Lua search path as byte arrays. The planner never queries the process environment or filesystem. Unknown inherited search paths remain an explicit unknown tail, distinct from a captured empty path. Both audited snapshots have the same inspected initial path rules. These plans describe the upstream initial loader; modified `package.path`, searchers, preload/cache state, aliases and arbitrary Lua still require graph evaluation and provenance.

Explicit modules begin `/`, `./`, `../` or `~/`. Relative explicit paths use the main configuration filename's directory, including the upstream helper's string replacement behavior. File candidates are the exact base, `.lua` when not already suffixed, and `init.lua`. The original Lua-file searcher remains a fallback after explicit misses; a preloaded/cached module or another searcher can take precedence. A candidate is not proof that a file is read or a permission to read it.

Explicit wildcard plans retain the raw pattern and raw directory candidate preceding its first metacharacter. The directory must actually exist before upstream records its lexically normalized watcher path. The planner does not enumerate directories, follow symlinks or claim watcher delivery. Actual globbing, regular-file filtering, normalized/sorted/unique results and missing-match categories are preserved in independent source observations for the forthcoming authorized resolver.

Named file-search plans apply the initial main-directory template pair and the captured inherited templates, replacing dots in the module name and question marks in templates according to observed Lua 5.4 searchpath behavior. Empty inherited entries are retained as failed candidates; a wholly empty inherited string is omitted by the Hyprland constructor. Repeated separators at the main filename boundary differ between the raw `absolutePath` helper and C++ filesystem parent/join operations; tests preserve that difference.

Path bytes are kept intact, including non-UTF-8 evidence. Candidate paths retain `..` and symlink-sensitive components until the appropriate source-derived stage. In the source fixture, checking a symlink/../ path and subsequently normalizing its chosen load filename can refer to different physical locations. The future graph capture must record and authorize both stages; lexical normalization is not a filesystem containment check.

The restricted parser worker has a typed `PlanModules` request. It receives only bytes and an audited version; filesystem, network and process-creation restrictions remain active. Limits are 16 KiB per module/main/home path, 64 KiB inherited search text or expanded candidate, 512 templates/requests, 1 MiB aggregate candidate bytes per plan and 2 MiB encoded batch results. Existing worker request/response/CPU/memory/deadline limits still apply. Oversized expansion fails explicitly before returning a partial plan. These are editor budgets, not upstream semantic bounds or a large-config certification.

## Evidence

`data/source-graph-semantics.json` retains immutable SHA-256/Git-blob provenance. `scripts/source_graph_probe.py --check` compiles actual upstream path/candidate/glob/watcher helpers and `absolutePath` for both source snapshots. It observes 43 private-file cases per snapshot, with direct Lua 5.4 searchpath queries for three inherited-path variants per case. The only filesystem status adapter records calls and forwards them unchanged. The ConfigManager adapter supplies a main path and tracked-path vector. Inputs cover extension precedence, directories, misses, home and relative paths, named/dotted modules, wildcard sorting/filtering, alias/broken symlinks, traversal and redundant main-path separators. Temporary fixtures are removed when the probe exits.

Four core tests compare candidate order/base/prefix queries and Lua search results against those observations, and check byte paths, unknown context, invalid versions/NUL and amplification limits. An actual restricted-worker test verifies supplied-byte-only plans and bounded expansion. This evidence does not certify package caching/preload/custom searchers, module execution/error handling, file races, capture authorization, ownership, source edits or compositor behavior. Installed Lua 5.4 observations are not a dependency ABI certificate for every supported pack.

## Required continuation

1. Compile the pinned safe-require/searcher/cache functions with private synthetic module fixtures. Establish named versus explicit cache identity, wildcard result/error continuation, false/nil returns, package.path changes and reload invalidation before static projection relies on them.
2. Add source-spanned dependency requests and lexical scopes to the inert AST projection, including aliases, computed paths, deferred/conditional calls and unresolved native/custom loaders. An unknown edge stays visible; it cannot authorize a fetch or prove effective configuration.
3. Implement a separately authorized capture graph with stable document identities, bounded wildcard parent evidence, cycles/misses, symlink/generator/store/system ownership and hard I/O deadlines. Captured path evidence and saved path hints never grant writer authority.
4. Replay authorized snapshot modules in actual declaration order using the restricted evaluator, preserving per-file lexical state and module return/cache semantics without executing arbitrary Lua. Retain byte sources and spans per document; do not concatenate/flatten the user's source graph.
5. Connect graph review to GUI import, schema-pinned private storage/reopen, changed-revision reconciliation and source-aware history. Then add owned insertion/deletion codecs with comment/trivia and unaffected-declaration checks across all structured features and legacy grammar.
6. Close the full watched-save, watcher, conflict, metadata and ownership gates with real scoped evidence. Effective discovery, isolated compositor preview, independent recovery, guarded application, resources/accessibility/localization/packaging and all remaining contracts stay required.

The original source-graph audit cache is retained only while this larger loader audit is unfinished. Delete it after the audit/probes complete, as requested; retain hashes and independent fixtures. No live file or compositor is modified by this work.
