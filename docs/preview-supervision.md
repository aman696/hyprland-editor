# Preview resource supervision: current implementation

The editor does not yet provide a rendered compositor preview. This increment starts E09 with an owned resource-domain library, a read-only delegation diagnostic and original-source backend observations. None certifies a preview profile, authorizes live configuration changes or satisfies a full security gate.

## Resource preparation

`crates/editor-preview/src/cgroup.rs` accepts an explicitly supplied, live directory descriptor. It duplicates that descriptor with close-on-exec and checks directory type, cgroup v2 filesystem identity, effective-user ownership and other-user write permissions. Stored observations cannot reconstruct this capability: neither `Delegation` nor `Domain` implements deserialization.

Creation requires a normal, empty delegation domain with CPU, memory and process controllers already enabled for children. It creates one unpredictable private leaf. It does not enable controllers or alter a shared parent's limits. The prepared default is 512 MiB memory, no swap, group OOM handling, 64 tasks and one CPU core of quota. Limits have explicit supported bounds. Each control write must complete as a single transaction and match readback. Opening the future membership descriptor does not prove that a child has joined; there is deliberately no launch or arbitrary-PID attachment API.

Explicit cleanup kills through the held private control descriptor, checks kernel population with a bounded wait, compares the held directory identity with the current parent entry and requests nonrecursive removal. A timeout, malformed population reply or failed removal is an error. Destruction is only a best-effort fallback. Preparation errors preserve cleanup errors and the generated identity when cleanup cannot be proved. If creation succeeds but the directory cannot be opened, the result identifies unresolved creation rather than removing an unverified pathname.

This is not complete teardown certification. Identity comparison and pathname removal are separate operations. A provider must ensure exclusive ownership of the delegated parent; concurrent same-user substitution between those operations remains an open certification case. Real kernel limit enforcement, successful migration, descendants, OOM and failure/race behavior require a properly dedicated test delegation. Ordinary-directory fixtures exercise refusal and POSIX failure handling; they cannot establish positive cgroup behavior.

## Read-only diagnostic

```text
hyprland-editor --check-preview-delegation /explicitly/chosen/delegation
```

This command runs before graphical application creation. It opens the selected directory without following a final symlink or blocking on a FIFO, then reports the observed domain and controller availability. It performs no controller writes or process migration. Its `live_limits_verified` and `compositor_verified` fields remain false. The report is evidence for inspection, never launch authority.

The recorded development candidate exposes CPU, memory and process controllers but has only memory and process controllers enabled for children. CPU delegation is missing. The implementation does not modify that shared candidate or start a host service to work around the missing delegation.

## Backend feasibility evidence

`data/preview-backend-sources.json` pins eight original files: Aquamarine 0.15.0 and 0.15.1 backend/header/headless files, and both audited Hyprland compositor files. `scripts/preview_backend_probe.py` checks their bytes and hashes, extracts original backend start/descriptor methods and compositor request construction, and compiles them with inert adapters. Temporary originals and compiled programs are deleted after the probe, including failure paths.

The resulting 32 start-flow observations are in `data/preview-backend-fixtures.json`. They establish these narrower facts:

- Headless descriptor methods return no DRM descriptors. Headless alone fails the observed allocator requirement.
- A NULL-first backend can pass start without an allocator; that result establishes neither rendering nor frame delivery.
- The first allocator attempt, reopen failure, mandatory backend failure and optional-backend filtering have distinct outcomes.
- Both pinned compositor request lists include mandatory headless, optional DRM and fallback Wayland.

The DRM reopen and allocation adapters never open a device or create a real allocator. A successful inert case is not a working preview. These observations rule out treating a stock headless-start flag as sufficient proof of GPU-free rendering. They do not establish that every possible software, nested-Wayland, render-node or virtual-machine profile is impossible.

`data/preview-development-observation.json` records one environment's read-only version and namespace capability observations. The installed binary matches the baseline commit, while its compiled and runtime Aquamarine versions differ. Separate namespace identities were observed for a trusted probe; a private root image, private proc mount, compositor isolation, GPU access, frames and input were not tested. These observations are not portable acceptance evidence.

## Required next implementation and proof

The original blueprint remains mandatory. Before a candidate config can run, implement a dedicated resource provider, stopped-child membership and readback, descriptor and environment filtering, a verified immutable runtime image, private root/proc and namespaces, device/IPC/network restrictions, the syscall policy and a supervisor whose teardown survives crashes and cancellation. Record exact compositor/library/renderer identity and enforce profile limits through kernel observations.

Then prove actual frame delivery and bounded input routing against synthetic clients, with no host session credentials or sockets available. Establish the supported rendering profile with real compositor evidence. Keep software, render-node and VM dependencies explicit; do not silently fall back to the host compositor or GPU. Test every required malicious-config, resource-exhaustion, descendant, race and crash case. Add user-facing preview only after those boundaries and the relevant blueprint gates are demonstrated.

Source ownership/reconciliation, persistent history, effective discovery, independent recovery, guarded live apply, accessibility, packaging and all original resource/security/release obligations remain open independently of this resource layer.

The cgroup implementation follows the kernel's [cgroup v2 interfaces](https://docs.kernel.org/admin-guide/cgroup-v2.html). Sandbox construction will also need configuration-specific validation; the [Bubblewrap project](https://github.com/containers/bubblewrap) describes a building block rather than an application policy certificate. Exact compositor/backend source URLs are retained in the pinned source manifest.
