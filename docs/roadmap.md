# Coverage and implementation gates

| Area | Audited inventory | Implemented in 0.4 | Next gate |
| --- | --- | --- | --- |
| Scalar settings | 353 baseline / 402 audited-main | All nine type families have draft controls; literal bounds; source links; separate observed values | Runtime descriptions, richer help, enumerations and dependency validation |
| Imported configuration | Legacy and Lua source grammars audited | Legacy literals; resource/syscall-restricted static Lua projection; guarded copies of existing literals; private source-backed `.hye` snapshots; explicit symlink-chain capture; reviewed single-root revisions; old JSON imports | Lossless multi-file/include graph, generated ownership, multi-file reconciliation, richer scope analysis |
| Effective state | Runtime descriptions and object schemas audited | Session/process/socket identity checks; manual descriptions snapshot; unsupported formats explicitly unknown | Event invalidation, more live codecs, multi-instance choice, real-compositor certification |
| Monitors, devices, workspaces | Structured fields audited | Feature inventory only | Typed structured models, cross-field validation, graphical editors |
| Rules, bindings, animations | Versioned parsers and handlers audited | Feature inventory only | Dedicated controls, codecs, matcher semantics, replay fixtures |
| Runtime actions and plugins | APIs/methods/fields/events audited | Feature inventory only | Capability and identity boundaries; unsupported opaque state remains unknown |
| Review and storage | Draft/publication distinction specified | Override/source-copy review, clipboard export, private source-backed workspaces, exact schema pins, unsaved-source/edit guards, asynchronous save; reviewed scalar/source reconciliation and inverse undo | Multi-file patch/merge/conflict review and watched-path persistence certification |
| Live apply | Recovery and watcher gates specified | Disabled; no writer exists | Independent recovery agent, session-pinned transactions, fault injection |
| Isolated preview | Private-compositor plan specified | Unavailable | Software rendering feasibility, namespace/bus isolation, private input/frame transport |
| Resource use | 100 MiB ordinary-editor target | Recycled list widgets, selected forms, bounded per-field undo, background jobs | Repeatable PSS/CPU measurements across supported systems |
| Distribution | Linux packaging plan | Desktop entry, icon, source build, CI | AppStream/Flatpak or distro package, signing and stable release policy |

The 1,453 contracts are a requirements register, not a count of completed controls. Source audit and generated metadata cannot establish preview fidelity, hardware correctness or release-wide runtime compatibility.

Implementation sequence: (1) strengthen the draft editor with Lua syntax and effective-state inspection; (2) structured editors and complete codecs; (3) isolated validation and preview; (4) independent recovery and certified live publication; (5) packaging, accessibility, resource budgets and release certification. See the blueprint for engine-level contracts and acceptance tests.

The [full completion register](completion-register.md) retains all audit obligations and named proof gates. Foundation tests are not release-wide feature certification.
