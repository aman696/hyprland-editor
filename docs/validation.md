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
