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
