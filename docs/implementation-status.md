# Execution plan and acceptance gates

The existing engineering blueprint remains the design authority. This register maps it to the code and tracks the next concrete steps. A checked design requirement is not an implemented feature or runtime certificate.

## Completed foundation in 0.2

| Blueprint area | Working implementation | Acceptance evidence | Remaining limits |
| --- | --- | --- | --- |
| Sections 4–6: models and packs | Version-scoped typed drafts, literal source metadata, field-level undo; observations stay separate | Type/bounds/version tests; UI workflow | Richer semantic schemas and independently certified binary packs |
| Section 7: Lua source | Pinned Tree-sitter grammar; top-level `hl.config` tables, nested categories, literals and restricted pure locals | Round trips for decoded defaults; shadowing/opaque-code tests; escape/size/syntax tests | No full static interpreter, include graph, branch evaluation or persistent source manifest |
| Section 7: source edits | Existing unambiguous literal ranges only; SHA-256 source identity; reparse and projection comparison; clipboard copy | Byte-preservation tests; real child-process copy test | No source insertion/deletion or publication; derived expressions remain uneditable |
| Sections 5 and 16: parser boundary | Self executable through `/proc/self/exe`; private pipes, empty environment, closed extra descriptors, no-new-privileges, seccomp allowlist and resource limits | Actual binary probes deny file opens and network sockets; inert command fixture; bounded invalid input | Linux x86-64/aarch64 syscall policy; local and hosted x86-64 checks only; no preview sandbox certification |
| Section 11: discovery | Fixed version/descriptions queries; owned runtime/lock/socket, peer PID/UID, held pidfd/start ticks, executable fingerprint and held socket-node identity | Real Unix-socket fixtures for allowed requests, replacement, malformed data and timeouts | Selected environment session only; manual refresh; real host compositor unavailable to local test sandbox; gradients remain raw |
| Section 4: asynchronous state | Background import/inspection/copy; draft generations discard obsolete parser results; explicit guards for unsaved edits | Native graphical workflow; parser process integration tests | No background persistence/checkpoint journal yet |

## Next implementation order

1. **Finish source ownership and persistence.** Store source manifests independently from active configuration; record device/inode/hash; support reattaching saved value drafts to a reviewed source revision; extend diagnostics to include graphs. Implement include authorization and dependency graphs without executing imports. Add insertion/deletion codecs only with comment/trivia preservation and conflict tests.
2. **Finish effective-state inspection.** Test against exact, isolated real Hyprland builds; certify complex live codecs; read monitors/devices/workspaces/rules/actions through explicitly whitelisted adapters; add an instance chooser and one bounded event subscription. Unknown opaque/plugin state remains unknown. Do not extend the reader into eval/repl or a generic command transport.
3. **Build structured feature editors.** Use the existing per-feature contracts for monitors, devices, workspaces, match/effect rules, bindings/submaps, animation curves/styles, gestures and layout messages. Each control needs version membership, typed validation, unset behavior, canonical serialization, meaningful boundary tests and provenance.
4. **Prove isolated validation and preview.** Prototype software rendering and private compositor transport in a separate supervisor. Require filesystem/bus/runtime/input isolation, bounded resources and teardown tests. A failed probe disables preview; never use the host compositor as a fallback. Source-derived scene projection excludes opaque native code and arbitrary Lua.
5. **Implement recovery before apply.** Independent recovery agent, pinned live identity, watcher graph, reviewed patch transaction, conflict checks, journal and rollback deadline. Fault injection must cover GUI/helper death, session replacement and partial file operations. No live writer is enabled before these gates pass.
6. **Package and certify.** Accessibility, localization, distribution packaging, resource stress tests, supported-release matrix, signed release process and dependency notices. Ordinary-editor 100 MiB PSS / idle CPU goals require repeatable measurements beyond a single startup sample.

The blueprint and the 1,453 feature contracts supply the technical requirements for these stages. They do not resolve every empirical question: preview feasibility, hardware behavior, parser interoperability and rollback success must still be demonstrated with controlled tests.
