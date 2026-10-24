# Explicit setting inputs for a sample

**Prepare Preview Draft → Review setting values** rebuilds captured scalar arguments and private source revisions in the restricted staging worker. The review shows all 353/402 standard setting rows for the selected audited snapshot, each with its pending override, captured-model value when available, and source-pinned native default metadata when resolved. These are three separate inputs; none is an observation of effective compositor state.

For example, a pending border size of 3 and a captured/private-source value of 7 stay separate. Choose **Use pending value**, **Use captured value**, or **Leave unset in sample**. The last choice emits no override and leaves the target's effective default unverified. It does not substitute an unresolved default expression. A bulk action selects available source values; missing, rejected, ambiguous, unrepresentable and out-of-draft-constraint values cannot be adopted as source values. Default choices preserve existing pending values and leave other settings unset. No source value is automatically adopted when opening the review.

**Use these inputs** retains choices only for that preparation view. It changes no draft, source revision, saved workspace, undo history or original file. Cancel preserves any previously accepted sample choices. Closing preparation drops its choices. Complete scene/selection persistence and history remain required. Source/draft generation changes invalidate pending reviews and preparation results.

The list supports search, lazy row controls and an optional full-value/provenance view. A source argument's document/order identifies the modeled call site; it is not ownership of an original literal or proof of execution. Pinned defaults may be unresolved, especially complex defaults. Private source revisions are materialized before values are inspected; original captures remain intact.

## Reconstruction and validation

The workspace must pass its existing exact schema/model and serialization bounds. A frozen graph is reconstructed from captured bytes and context only. Fresh base replay checks its compiler profile, every supplied dependency request/outcome and reached captured view. Current private pieces are materialized, compiled/reparsed and replayed again. Base captures that become unreachable after private edits stay preserved. Single-root Lua can be inspected without granting folders; multiple sources require their captured graph. Legacy source analysis remains unsupported. No original path is opened or folder capability restored.

Existing registration-aware conversion inspects the reached `hl.config` observations. Unknown execution, loading, native API/value behavior, ambiguous table keys, failed conversions and partial traversal remain explicit. The `model_complete` flag describes completion of this modeled initial scalar analysis, never future effects or effective runtime configuration. Partial observed values require the review's explicit acknowledgement before they can enter a sample. A later rejected/unresolved declaration never silently falls back to an earlier value.

The selected sample is reconstructed from a fresh review inside the staging worker. Review, input, model, compiler, registration and graph-schema fingerprints must still match; stale or unknown choices fail. Every emitted scalar argument must have an accepted, editable finite representation under the pinned value model. Selected source values additionally round-trip to exactly the same modeled native representation, including float/vector sign bits. A private span identifies the actual generated scalar block used by this check; it is created by the emitter and is not restored from a report. Floating literals keep float spelling and negative zero.

The output remains fresh generated Lua. Imported Lua/modules/plugins are never copied for execution. Shader/keymap assets and host-output references still follow the existing omission policy, even when selected from a source row. The report records selected source/pending/unset counts, unresolved source rows, source round-trip counts, the resolved workspace hash, review/selection hashes and the original input hash. A selection can therefore include a value that the later asset/hardware policy explicitly omits; its report does not certify that value was rendered.

Parent response checks correlate the requested layer selection as well as the original workspace/scenario and all eleven schema identities. A reported hash is content/correlation evidence, never authorization to execute returned Lua. The same one-shot syscall/resource/deadline/cancellation restrictions apply to source review and generation. Rows are bounded to 512, each row to 512 KiB and the complete review to 4 MiB; choice maps are bounded to 512 unique keys and 64 KiB. Unknown fields and duplicate choice keys are refused.

## Read-only diagnostics

```sh
hyprland-editor --check-preview-layers /absolute/workspace.hye
hyprland-editor --check-preview-projection-selected /absolute/workspace.hye /absolute/scenario.json /absolute/selection.json
```

The first reports a review and its SHA-256. The second requires a selection file such as:

```json
{
  "review_sha256": "the 64 lowercase hexadecimal characters from the review",
  "acknowledge_partial": false,
  "choices": {"general:border_size": "captured"}
}
```

Unspecified choices preserve pending values or leave settings unset. The selection must refer to the exact fresh review. All selected files use the bounded held-inode regular-file reader. These commands write only stdout/error diagnostics; they do not save, apply, start a compositor or silently migrate workspaces.

Tests cover both audited snapshots, repeated declarations, all three choices, unchanged input bytes, all nine families, native representation/sign-bit preservation, incomplete/rejected/absent inputs, acknowledgement, stale/unknown/duplicate choices and unrepresentable pending numeric values. The actual application worker also rebuilds a module's private `3 → 7` piece revision after deleting originals while retaining its separate pending value of 5. Native tests select source 7 while keeping pending 3, inspect narrow layout/search, block unacknowledged partial adoption and prepare accepted samples without changing the workspace.

Full structured source-layer resolution, effective defaults, native manager dependencies/effects, safe asset copies, live discovery, actual renderer/capture/input, persistent history/export and every original security/resource/release requirement remain mandatory. This is a reviewed scalar-input compiler increment, not a complete source compiler or rendered preview certificate.
