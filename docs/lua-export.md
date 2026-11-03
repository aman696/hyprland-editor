# Save reviewed Lua from the GUI

Open **Review and Export Draft**, check the generated text, then choose **Save Lua File…** and select a local destination. **Copy Lua** remains available alongside it. The existing **Review Source Copy** window offers the same actions for supported single-file source changes. For a retained project with multiple source files, **Review Source Copy** opens **Draft source files** instead. In its **Files** tab, select a file and choose **Review / Export This File**. The review contains the complete current source, including kept private edits, rather than the shortened inspection excerpt.

The saved bytes are the snapshot shown in the review window. Export creates a new private file and refuses existing files or symlinks; choose another filename to keep an existing copy. A save is reported only after writing and closing the file. Cancellation leaves the draft untouched. A failed write reports that a partial copy may remain. The maximum export is 4 MiB.

Saving a Lua copy does not mark the private `.hye` workspace as saved. Use **Save Draft** to retain the editing session. Export does not request a compositor reload.

Each source-file export is an independent copy. Import statements, relative/absolute paths and separate value overrides are not rewritten or combined automatically. This action exports the existing supported draft, single-source result or a selected materialized project file. Full multi-file source export, unresolved source changes and complete source/draft reconciliation remain in progress; their existing blockers remain visible.

Single Lua source review now combines scalar and structured draft changes. See [combined source copy](combined-source-copy.md) for supported placement, reconstruction checks and remaining dynamic/multi-file limits.
