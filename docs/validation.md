# Initial-build validation

Validation date: 13 October 2026. Rust 1.90.0, Linux x86-64, GTK 4.22.5, libadwaita 1.9.4. The local system compiler could not start, so checks used a verified official Rust toolchain isolated inside the ignored development directory; system packages were not modified.

- Release build succeeds with the checked-in lockfile.
- Formatting and strict Clippy checks pass.
- Fifteen core tests cover catalogue membership, codec semantics, string escaping, type/bounds rejection, non-finite numbers, strict draft decoding, round trips, inert legacy import, unresolved overrides, malformed input, private permissions, symlink refusal and FIFO refusal.
- An additional graphical test constructs all 755 version-scoped forms, validates their initial values, exercises staging, snapshot changes, removal and undo, and captures the actual rendered window. It is ignored by ordinary headless tests and run explicitly with a graphical display.
- Metadata checks confirm 353 baseline and 402 audited-main scalar records and 1,453 unique feature contracts. Reproducible generation checks compare application metadata with the checked-in pinned audit.
- The desktop entry passes `desktop-file-validate`. No user configuration was used as a test input, and no compositor setting was changed.

## Resource observation

A single cold release-build run, using GTK's Cairo renderer and an in-memory settings backend, was sampled after two seconds of startup. The main workspace was idle; the feature-register window, import dialogs and preview were not open.

| Measurement | Observation |
| --- | --- |
| PSS | 89,763 KiB (87.7 MiB) |
| RSS | 174,452 KiB (170.4 MiB, including shared library pages) |
| Idle CPU sample | 0 scheduler ticks over 5 seconds |

PSS apportions shared library pages and is the budget metric in the blueprint. This sample is preliminary, not a guarantee. Hardware rendering, multiple workspaces, long editing sessions, large text values, accessibility tools and different GTK versions need independent profiling. A zero-tick sample only establishes that no CPU time was measurable at this sampling resolution.

## Remaining validation

Hosted CI is configured to run build, core and graphical tests, formatting, strict lint and catalogue checks on Ubuntu 24.04. Its results must be assessed independently of this local record. Lossless Lua import, live state inspection, display recovery and preview isolation are not implemented and have no successful runtime certification claim.

## Stage 0.2 validation

The next planned foundation adds static Lua import/source copies and read-only scalar snapshots. Twenty-six core tests, four real parser-process tests, four Unix IPC fixture tests and one graphical test pass locally (35 in total), alongside strict Clippy, formatting and release build checks. The graphical test also verifies that unsaved edits prevent immediate closure. Parser probes confirm that filesystem opens and network sockets are denied after startup; an opaque command fixture cannot create its marker file.

Read-only fixture tests check both allowed request strings, peer/process identity, socket replacement, malformed replies and absolute timeout behavior. The actual host compositor PID is not visible through this local sandbox's `/proc`, so a real host snapshot cannot be certified here. Attachment fails closed rather than bypassing process/peer checks. Real-compositor certification remains a separate milestone. Snapshot strings must be valid JSON; malformed upstream output is not heuristically repaired or replaced with Lua evaluation.

A preliminary idle 0.2 release run measured **88,194 KiB PSS (86.1 MiB)** and **173,924 KiB RSS**, with zero recorded scheduler ticks over five seconds. It used the same Cairo/in-memory-settings setup as the initial sample, with one main workspace, no imported document or live snapshot, and no resident parser child. Shared-page accounting and startup variation make comparisons preliminary. Large imports, open source-review buffers and repeated editing need stress profiling.

See [implementation status](implementation-status.md) for the exact scope, evidence and next gates. The earlier source/audit register has not been relabeled as certified functionality.

Hosted Ubuntu testing exposed immediate socket-inode reuse in the replacement fixture. The repair holds an `O_PATH` descriptor to the original socket node for the entire read session and pins the process with a pidfd. A replacement cannot reuse that pinned filesystem inode; process-exit checks do not rely solely on recycled PID numbers. This change is covered by the original replacement fixture and rechecked in hosted CI.

## Interface redesign validation

The main workspace now follows the [UI design rules](ui-design.md). The same graphical workflow test constructs every scalar form and checks version switching, removal/undo and unsaved-close protection, with added checks for category/search filtering, disabled Review state, menu-to-review routing, initially collapsed technical details and reopening a selected row after returning to search. Repeated notifications for an unchanged search preserve selection.

A verified, privately extracted Xvfb package supplies an isolated display for local rendering checks; no system package or desktop configuration is changed. The test renders at 1100 × 760 and resizes to a 360 × 720 window request (350 × 710 content after decoration). All nine scalar control families fit the content allocation. The hosted graphical test now includes that exact resize check.

![Desktop workspace](screenshots/draft-workspace.png)

![Narrow editor](screenshots/narrow-editor.png)

The screenshots use the inherited native theme and Cairo renderer. A full accessibility review, translated/large-text layouts and user usability study remain outstanding.

A single idle release-build sample of the redesigned empty workspace on that isolated Xvfb display measured **91,768 KiB PSS (89.6 MiB)** and **175,848 KiB RSS (171.7 MiB)**, with zero scheduler ticks recorded over five seconds. It used Cairo and an in-memory settings backend, sampled two seconds after startup. This is a preliminary observation, not a memory ceiling; the display backend also differs from the earlier desktop sample.

## Source persistence foundation (0.3)

The source-backed workspace path adds six core storage/identity tests, one duplicate-draft-key test and three real-application reopening tests. The application tests execute the actual binary with its restricted parser child: a recorded original path can point to a FIFO without being opened, saved opaque commands cannot create a marker, source snapshots are reparsed, edits survive reopening, and bad hashes, forged report fields and syntax errors are rejected. A mismatched exact catalogue fingerprint is rejected by core validation rather than silently migrated.

The native workflow additionally imports a synthetic source model, saves its draft and identity to a private fixture directory, checks that an identity change is unsaved, and verifies that a save started before a later edit cannot clear that newer edit. All fixtures use synthetic files; no user configuration is an input or output. Native layout checks continue to cover all 755 initial forms and all nine families at narrow width.

The completion-register consistency check tracks all 1,453 full feature contracts, 756 documentation candidates, 24 blueprint sections, 12 work orders, 24 security gates, nine stages and ten shared model invariants. That mechanical check is not a conformance certificate. Watched-path isolation, lossless include/symlink graphs, full source operations, runtime parser certification, preview, hardware recovery and the other outstanding obligations remain unverified.

A source-review regression covers calls hidden in ignored, positional, oversized and nested table fields. Those expressions can affect runtime bindings even when their keys are outside the catalogue. They now mark the source opaque and prevent subsequent API calls from being treated as statically trusted. A real restricted-worker fixture confirms an embedded command stays inert and the later declaration is not falsely imported.

The finalized 0.3 foundation passes 34 core tests, five restricted-parser process tests, three real-application reopening tests, four IPC fixture tests and one explicit native graphical workflow (47 in total), plus formatting, strict Clippy, catalogue generation, completion-register consistency and release build checks locally. Hosted checks are recorded separately for the published revision.

## Explicit symlink source capture

Selected Lua/legacy sources now record symlink chains and resolved leaf identity without following config includes. Six path fixtures cover relative/absolute/directory links, non-UTF-8 targets, cycles, broken links, linked FIFOs, procfs links, trailing-slash semantics and replaced link targets with identical bytes. A workspace fixture deletes the recorded link and target before reopening its saved snapshot. A display fixture checks escaping of newlines, bidi controls, literal backslashes and invalid UTF-8 bytes.

All import readers inspect an `O_PATH` node before opening a regular inode for I/O; workspace/value-draft paths still refuse final-component symlinks. This does not certify hostile concurrent directory mutation, generated-file managers, dependency authorization or remote/FUSE I/O deadlines.

This revision passes 42 core tests, five restricted-parser process tests, three real-application reopening tests, four IPC fixture tests and one explicit native graphical workflow (55 total), plus formatting, strict Clippy, metadata/register consistency and release build checks locally. These checks use synthetic fixtures and an isolated display; no live configuration is modified. Hosted results belong to the exact published commit.

## Single-root source reconciliation (0.4)

Validation date: 14 October 2026 (Asia/Kolkata).

Eight core comparison tests cover independent edits/additions/deletions, false and empty literals, divergent edit/delete conflicts, opaque absence, explicit adoption of a visible partial literal, acknowledgement without scalar rows, report/source/draft staleness, version mixing and refusal to adopt derived projections as literals when opaque code can invalidate cached locals. Three application history tests cover operation/data budgets, shared source allocation accounting, inverse-value scope and oversized-operation refusal without losing existing history. A real-binary comparison fixture restores a saved snapshot, parses an explicitly selected changed/opaque file with restricted children, reports conflicts and proves neither input was written and no embedded command created its marker.

The native workflow checks choice gating, partial acknowledgement, unavailable unresolved adoption, unchanged original files, source/value undo, dirty-state restoration, field undo after a merge, stale-review rejection and both conflict/partial surfaces at narrow width. It exercises fixture models; the separate real-binary check covers worker/process comparison. Local checks pass 51 core tests, three history tests, six parser-process tests, four workspace/comparison application tests, four IPC fixtures and one native graphical workflow (69 total). Formatting, strict Clippy, release build and metadata/register checks are recorded separately for the final revision.

This is static scalar reconciliation for one Lua root, not a general source merge or runtime transaction certificate. The undo data quota estimates owned capacities with shared source de-duplication and conservative structure overhead; allocator overhead, GTK source-review buffers and total process memory are separate. Persistent checkpoint/spill history, stress profiling, full include/scope semantics, multi-file merge and live conflict/recovery testing remain outstanding.

The derived/partial regression also exposed valid Lua semicolons being treated as opaque statements. The pinned grammar defines `empty_statement` as `;`; it now has no effect on static locals/API certainty. A core fixture and a real restricted-process fixture verify semicolon-separated pure declarations remain projectable without executing Lua.

An isolated 0.4 release run for the reconciliation foundation (`678b34d`) measured **107,159 KiB PSS (104.6 MiB)** and **176,928 KiB RSS (172.8 MiB)**, with zero scheduler ticks over five seconds after two seconds of startup. The run used a private Xvfb display and private D-Bus session, Cairo, in-memory settings and empty fixture config/cache/state directories, with one idle empty workspace and no imported source, desktop snapshot, open review or parser child. The app and private bus/display were terminated and fixture directories removed afterwards. **This sample exceeds the 100 MiB PSS target.** Its bus/config setup and shared-page accounting differ from earlier samples, so it does not establish a version-to-version regression. Repeatable resource profiling, large source/review/history stress and memory reductions remain required.

Comparison reviews also pin the complete selected catalogue content. A schema guard fixture changes bounds while preserving all currently valid values, and checks that review acceptance still fails; duplicate IDs and non-finite bounds are refused. Source/report/draft identity alone does not certify that field semantics stayed unchanged.

## Monitor draft editor (0.5)

Verification resumed on 14 October 2026. Checkpoint `9c982c1` passes 57 core tests, four history tests, seven restricted-parser process tests, five workspace/application tests, four read-only IPC fixtures and one explicit graphical workflow (78 total). The local compositor socket fixtures required network permission because the workspace sandbox blocks local Unix sockets; the initial permission-denied run was an environment restriction, and all four affected fixtures passed after permission was granted.

Formatting, strict Clippy, pinned catalogue generation, completion-register consistency and the release build passed. The exact checkpoint also passed [GitHub CI run 38457016554](https://github.com/aman696/hyprland-editor/actions/runs/38457016554), including the hosted graphical test and release build. The final screenshot was inspected after the native test. No host compositor configuration was modified.

Monitor tests cover every audited field in both snapshots through typed Lua export/static import, modes and signed placement, alias/type/schema/boundary rejection, mirror cycles and later clearing, explicit logical geometry, workspace round trips, inert opaque expressions in the real restricted worker, source-versus-draft persistence, undo capacity accounting, and native add/duplicate/reorder/remove/undo/review. Native layout checks cover all 22 monitor forms at 360 pixels; the fixture also exercises diagram drag/undo and private saving of monitor rules alongside source/scalar drafts.

This proves the tested draft behavior, not complete monitor hardware support. No available-mode/EDID inspection, compositor interoperability, monitor source editing/reconciliation, ICC decoding, live apply/recovery or isolated compositor preview is certified. No new 0.5 memory observation was taken; the earlier 104.6 MiB PSS sample remains above the 100 MiB target.
