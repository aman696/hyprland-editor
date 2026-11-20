# Legacy configuration workflow

Import a `.conf` file, edit the recognized scalar values through the normal controls, and open **Menu → Environment and Startup…** for recognized environment variables, startup commands and permission rules. Choose **Save Draft** to retain the original source and edited drafts in a private `.hye` project. Reopening reparses the saved text without opening the recorded original path. The original may have moved or been deleted.

**Edit Monitors** also imports classic `monitor = ...` declarations into their existing graphical controls. Choose between defining a rule, disabling an output, updating only its transform and updating only reserved space. The editor preserves these as different operations. Changing operation explicitly keeps compatible fields and can be undone; the field picker offers fields supported by that operation. Resolution/modelines, position, scale and the eight classic options are available. **Review Monitor Config** prepares a fragment; the main **Review Source Copy** includes monitor, scalar, session and binding changes together. See [monitor editing](monitor-editor.md).

**Edit Bindings** opens ordered literal shortcuts, native submap switches and unbind operations. Graphical chord/flag/device controls, a 71-dispatcher picker and common action presets feed private drafts, undo and combined source review. Known preceding local definitions such as `$mainMod` now populate supported shortcuts while unchanged fields retain references. Includes, external values and unresolved submaps preserve scope barriers; older literal-only projects offer an explicit reviewed upgrade. See [local-variable editing](legacy-binding-variables.md). See [legacy shortcut editing](legacy-binding-editor.md).

Choose **Review Source Copy** to prepare a separate edited `.conf`. The review offers **Copy Config** and **Save Config File…**, with the same asynchronous, new-file-only output used for Lua reviews. Existing files and symlink targets are never replaced. The export snapshot does not change if the draft subsequently changes. No desktop reload is requested.

The scalar editor changes the last assignment of a modified setting, appends an absent setting using its qualified key, and removes all assignments of a removed setting so an older duplicate does not reappear. Unchanged text, comments, unmodeled directives, Unicode and existing line endings are preserved. All nine scalar value kinds have encoders. The prepared result must reparse to the exact requested scalar projection before review; signed zero remains distinguishable. Both input and edited source are limited to 1 MiB.

The session editor recognizes top-level `env`, `envd`, `exec`, `execr`, `exec-once`, `execr-once`, `exec-shutdown` and `permission`. It retains their order and distinguishes raw commands from commands with inline launch rules. The graphical launch-mode control makes this choice explicit. Reload-only and raw-shutdown modes are refused because no corresponding legacy directive exists. Commands are never executed, environment values are never installed and permissions are never activated by editing or review.

Session source copies update supported declaration slots, retain comments and unresolved lines, append new entries and independently reparse the complete result alongside scalar edits. Literal hashes, backslashes, braces and representable escaped permission commas use the pinned legacy rules. Multiline values, variable references, edge whitespace and unrepresentable patterns can remain saved drafts but cannot produce a misleading source copy. Explicit false and unspecified activation flags both serialize as `env`; `envd` represents true. Lua command intent requires an explicit legacy launch choice before conversion.

**Import Diagnostics** lists unresolved lines and repeated keys. Includes, unresolved or other-family variable uses, conditional directives, continuations, nested session/monitor declarations and unsupported suffixes remain inert source text. Unknown/dynamic monitor blocks and other structured legacy families still need editing/export codecs; source copying refuses separate drafts from unsupported families rather than omitting them. Source paths retain their original references. This is a bounded selected-file projection, not effective compositor state or verified portable relocation.

Literal flat `monitorv2 { ... }` blocks now open as **Monitor block (HDR and all fields)**. The first field must select a single explicit output. Existing HDR, wide-color, transfer-function and luminance controls join all 22 audited monitor names (including the reserved-area alias). The codec maps the native `addreserved` name/order to the graphical sides. Empty block ICC values remain explicit. Updating one field preserves other field spellings and comments; removing a repeated field removes all its assignments, while changing it updates the last assignment. New fields are inserted before the closing brace. Block duplication, removal, order changes and explicit operation conversion retain comments.

The editor presents source declarations separately. Hyprland processes keyed monitor blocks after classic keyword rules, and blocks for the same output can combine fields. The displayed list is not an effective matching/default/ordering simulation. Unknown fields, nested/dynamic/conditional/continued blocks, qualified category assignments and unsupported literal shapes stay unresolved; their original source remains retained.

Old projects pinned to the frozen classic monitor codec remain readable. Newly recognized blocks are added for review without restoring classic targets the user intentionally removed. Saving a new project records the current codec; later reopening respects an intentional empty block target. The source-model transition retains captured inputs and existing compiler observations.

The scalar codec remains more conservative: scalar lines with hashes/backslashes requiring unmodeled escaping cannot be rewritten, and changed scalar strings requiring escaping, expansion, quotes or braces are refused. Saving the project still retains those draft values. Complete legacy grammar, other structured-source editing, include resolution, dependency relocation, runtime validation and guarded apply remain unfinished.

The baseline [Hyprland handlers](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/legacy/ConfigManager.cpp), [hyprlang lexer](https://github.com/hyprwm/hyprlang/blob/090117506ddc3d7f26e650ff344d378c2ec329cc/src/config.cpp) and hyprutils argument splitters are pinned by content in `data/legacy-session.json`. Quotes do not shield hashes in this lexer. No automatic audited-main syntax conversion is claimed.

New projects record the captured legacy source and codec identities even when the session target is deliberately empty. Older projects without this marker rebuild the supported source entries and append their previously separate session drafts for review; the original project file is not changed. Previous session-schema projects remain readable without accepting the new legacy command variant under an old schema pin.

Monitor drafts have their own captured-source and codec markers, including deliberate removal of every monitor declaration. Older projects without them rebuild supported monitor source entries and append older separate monitor drafts without identity collisions. Existing Lua monitor intentions need an explicit legacy operation choice before `.conf` export. Legacy operations cannot silently export as Lua or enter ordinary preview projection; the latter records their omission without output/path payloads. This does not implement actual isolated compositor preview.

Validation covers scalar edits, all eight session directives, raw launch modes, ordered insertion/removal, CRLF/Unicode, hash/escape boundaries, unknown-line retention and schema compatibility. The private graphical workflow imports a captured `.conf`, deletes the original, changes real controls, saves/reopens, prepares the combined source review and checks intentional empty targets and older projects. No host configuration, command, environment or policy is modified. These fixtures do not establish complete legacy grammar or audit acceptance.

## Workspace rules

Literal workspace declarations now populate the graphical workspace editor. All 14 native handler fields, ordered repeated selectors and string layout options can join the combined source review. Deliberate empty targets persist; older separate Lua drafts remain separate. Unknown syntax stays unresolved, and no hook is executed or implicitly projected into preview. See [workspace editing](workspace-editor.md) for the field mapping, workflow and remaining limits.

## Animations and Bézier curves

Literal top-level and flat `animations` category declarations now populate the graphical animation editor and join the combined source copy. Native timing/ranges, disabled ignored arguments and ordered curve references remain distinct from Lua drafts. Private markers retain intentional empty views and older separate drafts; ordinary preview omits native curve/style payloads. See [animation editing](animation-editor.md) for the workflow and limits.

## Gestures

Literal native gestures now populate matching/action controls and join the combined source copy. Native inhibition flags, modifier/scale prefixes, builtin parameter order and inert dispatcher arguments remain distinct from Lua callbacks. Save/reopen and undo preserve deliberate empty targets and older separate drafts. See [gesture editing](gesture-editor.md) for supported forms, registration identities and remaining limits.


**Edit Devices** now opens literal flat `device` blocks with all 47 registered fields and exact native values alongside guided controls. Native aliases, explicit unset/duplicate semantics and comments are retained. Device edits join the combined eight-group source review, with private save/reopen and intentional empty-target markers. Unknown/dynamic blocks remain unresolved, and repeated names do not establish effective hardware state. See [native device editing](device-editor.md#native-device-blocks).


**Edit Window / Layer Rules** now projects literal native inline declarations and flat named blocks into the rule editor. All baseline registered effects and match fields remain available, with native strings/order and version-specific integer enable handling. Rule edits join the combined nine-group source review. Unknown syntax stays inert. See [native rules](rule-editor.md#native-configuration-rules).


**Edit Shared Variables…** edits ordered source definitions with color/number/modifier/switch helpers. Review includes other staged native edits, reparses supported values and keeps an undoable private source revision. The immutable original capture stays separate; source-copy review uses the current revision. See [shared variables](legacy-variable-editor.md).


## Included files

Choose **Edit Included Files…** after importing a `.conf` project. Add, duplicate, remove, edit or reorder top-level `source` directives. Enter the path as a Hyprland template, which can include variables or a wildcard. The editor preserves the template and leaves matching and file loading to Hyprland. The pinned compositor resolves relative paths against the including file and processes matched files while reading each directive. [Pinned Hyprland source handler](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/legacy/ConfigManager.cpp#L1670-L1717)

Removing a directive clears its selected source slot and retains its comment. Reordering fills the surviving slots in the requested order, with comments attached to their original positions. New directives go before the next surviving slot or after the last original directive. On a file with no editable include directives, new ones start at the top. The review shows the full materialized source before and after the change, including other staged edits. Inspect the positions when a directive moves across a definition or setting. The editor preserves unsupported include contexts in the source.

**Keep Private Changes** records the proposal in the project's current source revision. Undo restores the previous source and typed drafts together. Save and reopen work from captured bytes. Resolving wildcard and variable templates and verifying effective cross-file settings remain open.


### Linked file snapshots

From **Edit Included Files…**, open **Linked File Snapshots…**. This view uses the last kept version of the root. Select a literal include and choose the local file at the displayed path. Relative paths are matched against the recorded including file's directory. The file picker gives the editor access to that one selection and the captured bytes go into the private draft. Saved projects reopen these bytes from the project. Inspect a captured entry to read its contents. Capture a newer version by selecting the same path again. **Remove Snapshot** affects the private draft and supports Undo.

The reference list follows the include directives in source order. Captured children contribute their own references, repeated includes stay visible, and path cycles are marked. Files detached by a root edit remain available as retained snapshots until removed. Variable and wildcard templates, home expansion, parent traversal and unsupported contexts remain unresolved. The view reports when it reaches its reference or nesting limit. The draft can hold 31 linked snapshots and 2 MiB of linked text in total, within the ordinary project size limit.

The main settings editor and **Review Source Copy** operate on the root file. The capture list uses recorded path matching. [Pinned Hyprland source handler](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/legacy/ConfigManager.cpp#L1802-L1855)


### Edit a linked file

Select a captured file in **Linked File Snapshots…** and choose **Edit Linked File…**. Find a scalar setting or open a group such as Monitors, Bindings, Rules, Variables or Included Files. Stage changes inside that file's draft. **Undo File Edit** runs inside the file editor. **Review File Changes** shows the before and proposed source. **Keep Linked File Changes** stores the revision in the project and closes the file editor. The main project Undo reverses that keep operation.

The snapshot browser previews the revised text and uses it to find outgoing include references. **Export Linked File Copy…** saves the selected file to a new location. Relative references keep their original spelling, so check the destination layout before using the copy. **Restore Captured Version** recovers the original snapshot and is undoable. Restore the captured version before replacing a snapshot with different bytes or removing an edited file. Reopening a project preserves both the original and the kept revision.

Source fields outside the supported native codecs remain as text. Each linked file is edited independently, so a value in the file editor may be overridden later in the full configuration.

### Export the configuration folder

In **Linked File Snapshots…**, choose **Review Entire Configuration Folder…**. The review includes staged root settings and kept revisions of every reachable captured file. Select a filename to see its original path and the full proposed text. The root is named `hyprland.conf`. Linked files get numbered names in first-include order so different files with the same name stay distinct. Nested and repeated literal `source` directives point to the exported copies. Their positions, comments and line endings are preserved. Detached files stay in the private project and are omitted from the export.

**Save New Folder…** creates a private folder with the reviewed files. Choose an unused name and a path without wildcard characters or backslashes, since Hyprland applies globbing to the full resolved include path. The editor reads only captured project data to prepare the copy. Other paths and commands stay as written and can still reference external resources.

Folder export reports missing captures and cycles. Variable, home and wildcard expansion, parent traversal, continued lines, computed directive names and unsupported include contexts still require separate review. For an individual file, **Review Source Copy** and **Export Linked File Copy…** preserve its original include spelling. [Pinned hyprlang parser](https://github.com/hyprwm/hyprlang/blob/090117506ddc3d7f26e650ff344d378c2ec329cc/src/config.cpp#L683-L888)

### Trace a setting across files

Choose **Trace This Setting…** in the main settings panel, or **Trace Source Settings…** in **Linked File Snapshots…**. Search by setting name or key. The trace uses staged root edits and kept linked-file revisions. It visits a linked file at each include directive, then resumes the parent. Repeated includes appear as repeated assignments. Each entry shows the value written in the draft, its file and its starting line, with the latest entry first.

**Last captured literal** identifies the latest explicit value accepted by the editor's literal parser. A later variable or expression is shown at its own line as needing expansion. Missing or unsupported includes, cycles, parser conditions, computed keys and scan limits give the trace an incomplete status with source locations. The view retains the latest 32 assignments for a setting and shows the count of older assignments. Desktop observations remain available through **Refresh Desktop Values**.

When the latest visible assignment is in a captured linked file, **Edit in Linked File…** opens the setting in that file's draft. Keep the reviewed linked edit to refresh the trace. The view currently covers scalar assignments. Defaults, general variable and include expansion, structured declarations and live compositor behavior still require separate evidence. [Pinned Hyprland include handler](https://github.com/hyprwm/Hyprland/blob/efb50993780079460b0cbed1363e2166a2de1d9f/src/config/legacy/ConfigManager.cpp#L1802-L1855)
