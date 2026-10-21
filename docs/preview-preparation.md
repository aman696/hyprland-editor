# Restricted preview preparation

**Main Menu → Prepare Preview Draft** opens a separate preparation view. Choose one virtual display at 1280×720 or two at 640×480 each. Each monitor draft can explicitly target a virtual display or be omitted. Each device draft can target a uniquely named synthetic device or be omitted. Changing the layout resets monitor choices. These are requested sample identities; no output or input device is created by preparation.

**Prepare and review** captures that draft generation and runs the [fresh typed-draft projection](preview-projection.md) in a short-lived restricted worker. The result is a searchable fidelity list with lazy rows. The result does not modify the draft, save a workspace or start a compositor. The optional [Review setting values](preview-setting-layers.md) action reconstructs captured scalar arguments/private revisions and compares them with pending values and pinned default metadata. Explicit pending/captured/unset choices affect only this sample; partial observations require acknowledgement. Full source/default resolution, assets, actual sample clients, rendering, capture and input remain mandatory unfinished work. Closing cancels in-flight preparation. Changed drafts invalidate results. Repeated opening reuses the current visible view; closing clears only that view's cache, including when another reference keeps its object alive.

The process launches through an explicitly held application inode, with an empty environment, piped input/output and discarded stderr. After library startup, before reading the request, it installs 512 MiB address-space, two-second CPU, zero-core and 8 MiB stack limits, closes inherited descriptors above standard I/O, sets no-new-privileges and applies the existing default-deny parser syscall policy. That policy denies file opening, networking, process/thread creation and subsequent execution. Failure to install any restriction returns a failure before parsing the input. Neither original Lua nor projected Lua is executed; generated syntax is checked using the linked compiler and independent parser.

The request is capped at 10 MiB; the workspace must independently satisfy its existing 8 MiB serialization limit. The response envelope is capped at 12 MiB, with the existing 1 MiB Lua and 4 MiB report limits. Transported collections are bounded while decoding: four outputs, 256 entries per mapping, 16 schema pins, 16 limitations and 16,384 fidelity entries. Source reviews additionally allow at most 512 rows, 512 KiB per row and 4 MiB overall; choices allow at most 512 unique keys and 64 KiB. Duplicate mapping/choice keys and unknown fields are refused. Report labels also have per-field limits. These are transport bounds, not measured whole-product memory acceptance.

The supervisor uses nonblocking pipes in one background operation, a three-second exchange deadline and cancellation checks between bounded read batches. Every exit path owns killing/reaping its child and closing its pipes; no writer/reader threads are left blocked on a full pipe. This worker cannot create descendants after isolation. Kernel-stall/crash behavior and whole-application lifecycle/resource gates remain unproven.

The parent independently constructs the expected workspace hash, upstream version, compiler/model/schema identities and scenario. It checks these against the bounded response and verifies the generated Lua/report content hash. Optional setting choices also bind exact selection/review hashes; source reconstruction and native representation checks run afresh inside the worker. That detects mismatched or changed results; a hash does not authenticate a package or certify arbitrary returned code as safe to execute. The returned artifact provides read access only and authorizes no launch. A future runtime must independently meet the approved-image, projection, isolation and execution gates.

For a read-only diagnostic without GTK:

```sh
hyprland-editor --check-preview-projection /absolute/workspace.hye /absolute/scenario.json
```

The scenario uses this explicit shape:

```json
{
  "outputs": [{"name": "HEADLESS-1", "width": 1280, "height": 720}],
  "monitor_targets": {},
  "device_targets": {}
}
```

Selected workspace/scenario inputs use the held-inode regular-file reader; the scenario file is bounded to 64 KiB. It writes the staged artifact and explicit non-runtime flags to stdout, changes no selected file, and never silently migrates a workspace. The worker role accepts no filenames or environment-supplied configuration.

Tests exercise the real child on both audited snapshots, explicit mappings, inert actions, deleted original sources, exact input preservation and absence of side effects. A direct role probe observes exact EPERM denials for file opening, networking, process creation and execution, plus EBADF for an intentionally inherited fixture descriptor. Additional cases cover malformed/oversized requests, unknown scenarios, symlinks, artifact correlation, bounded collections, active cancellation/deadline reaping and cancellation before launch. The native workflow exercises layout/mapping selection, searchable results, unchanged draft bytes, view reuse/reopening, stale-generation rejection, close/cancel and action visibility at 350 pixels. These tests certify their described staging behavior, not a rendered preview or any full audit contract.
