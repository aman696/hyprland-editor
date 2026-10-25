# Legacy configuration workflow

Import a `.conf` file, edit the recognized scalar values through the normal controls, and choose **Save Draft** to retain both the original source and the edited draft in a private `.hye` project. Reopening that project reparses its saved text without opening the recorded original path. The original may have moved or been deleted.

Choose **Review Source Copy** to prepare a separate edited `.conf`. The review offers **Copy Config** and **Save Config File…**, with the same asynchronous, new-file-only output used for Lua reviews. Existing files and symlink targets are never replaced. The export snapshot does not change if the draft subsequently changes. No desktop reload is requested.

The scalar editor changes the last assignment of a modified setting, appends an absent setting using its qualified key, and removes all assignments of a removed setting so an older duplicate does not reappear. Unchanged text, comments, unmodeled directives, Unicode and existing line endings are preserved. All nine scalar value kinds have encoders. The prepared result must reparse to the exact requested scalar projection before review; signed zero remains distinguishable. Both input and edited source are limited to 1 MiB.

**Import Diagnostics** lists unresolved lines and repeated keys. Includes, variables, commands and structured legacy declarations remain inert source text; this is a selected-file scalar projection, not the effective compositor configuration. A source copy preserves those directives, including their original relative references. Moving the exported file does not relocate dependencies.

Lines with escaped hashes or backslashes cannot be rewritten yet. Text requiring escaping, expansion, control characters, quotes or braces is refused when changed. Saving the `.hye` project still retains those draft values. Source copying is also refused if structured draft changes would be omitted. Complete legacy grammar, structured-source editing, include resolution, dependency relocation, runtime validation and guarded apply remain unfinished.

This intentionally conservative scope follows the documented distinction between comments, escaped hashes and variable expansion in the [official hyprlang guide](https://wiki.hypr.land/0.56.0/Hypr-Ecosystem/hyprlang/); the importer is not a replacement for that full parser.

Validation includes scalar byte-preserving edit/insert/remove cases, nested duplicates, CRLF/Unicode, nine value kinds and unmodeled escaping. The private graphical workflow imports a real captured `.conf`, deletes the original, edits, saves, reopens and clicks the source-copy action, then compares the entire review with the expected edited bytes. It checks both export actions and the narrow-window minimum. This validates the selected workflow, not the full legacy grammar or audit scope.
