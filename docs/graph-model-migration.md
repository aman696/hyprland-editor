# Reviewed saved-graph migration

Opening a supported older `.hye` workspace rebuilds its saved source graph in the current restricted workers and presents **Review rebuilt workspace**. Cancel is the default. **Use rebuilt copy** retains an in-memory draft; the existing unsaved-draft confirmation still applies. A draft change during either review discards the pending result. The rebuilt copy remains unsaved until the existing Save action writes a new private workspace. The selected old workspace is never overwritten.

Only two exact previous model identities are supported:

| Previous model SHA-256 | Reviewed difference from this build |
| --- | --- |
| `f7017978da7127ee461497d1021e418bb2c2e171b4d28a43b6671f8abb2531dd` | Root workspace manifest/lock changes adding the internal projection crate, plus exposing the existing bounded storage reader |
| `046fb098cb5c1f38d6922f37274cfcba56818ec35c68dbcd5d78fc1e5e4d7371` | Exposing the existing bounded storage reader |

The current target is `7273a441eca7d6ae6424b92d1188be64e642d007a77ae9e6a909239284f23ffc`. Embedded previous manifests must hash to their declared identity. All 58 input paths must match; only the listed input hashes may differ. The storage helper's read logic is unchanged: public visibility and documentation let the application reuse its held-inode, bounded regular-file reader. Parser, replay, emitters, compiler profile and graph representation inputs are unchanged by these reviewed transitions. A future model change disables these migrations until its differences are reviewed explicitly.

The reader accepts one selected regular `.hye` file, bounded to 8 MiB, with no final symlink following or FIFO/device I/O. Unknown saved payloads that the current decoder would discard are refused. After changing the candidate's model metadata, normal workspace validation and fresh restricted-worker replay verify compiler identity, captured views, source hashes, reached resolution inputs and private piece revisions. Materialized private edits are independently rebuilt and replayed. Original paths are historical evidence; migration never opens them or restores folder permissions. Lua is analyzed and compiled for syntax, never executed.

Reversing the candidate's model change must recover the entire decoded input exactly, including captured bytes, original identities, private pieces, typed drafts and resolution evidence. The summary binds the selected input and rebuilt workspace hashes, counts and unresolved analysis findings. A stopped or partial analysis remains partial; migration is not proof of effective compositor values or runtime behavior.

For a read-only check without GTK:

```sh
hyprland-editor --check-workspace-migration /absolute/path/old.hye
```

It reports the reviewed transition and reanalysis, writes nothing and requires exactly one argument. Ordinary `--check-workspace` continues to reject old graph models; it never silently migrates them.

Tests rebuild both supported models for both audited Hyprland snapshots with deleted original files and retained private source revisions. They refuse unknown models, foreign compiler profiles, source corruption, mismatched dependency requests, hidden payloads and unknown views. The native GUI workflow covers cancel, changed-draft refusal, protecting unsaved values, explicit replacement, a new private save and unchanged old-file bytes.

General older-model conversion, schema transformations, persistent operation history, full feature-family reconciliation and full source export remain required. This specific compatibility path does not certify those broader audit contracts.
