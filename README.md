# Hyprland Editor

A native GTK4/libadwaita workspace for making Hyprland configuration graphical, with a versioned catalogue and a safety-first editing core. The long-term goal is complete configuration coverage, including rules, monitors, bindings, animations, devices and isolated preview.

**Status: experimental 0.3 editor. This is the first working slice, not a complete Hyprland configuration manager.** Its live inspector is read-only. It does not apply changes to the running compositor.

![Native draft workspace](docs/screenshots/draft-workspace.png)

## Available now

- Browse by category and search readable names or original setting keys for every audited scalar: 353 for **0.56.2**, 402 for **audited-main**, an exact development commit. Switch snapshots without losing their separate in-memory drafts.
- Adaptive sidebar navigation, focused value cards, expandable technical details, a main menu and keyboard shortcuts. The layout follows the [GNOME HIG](https://developer.gnome.org/hig/principles.html); see the [UI design rules](docs/ui-design.md).
- Graphical switches, number controls, text fields, color pickers, gradient stops, coordinates, four-sided gaps and font weights.
- Stage validated overrides, remove them, undo up to 32 field changes, and review generated Lua before copying it. Unsaved drafts require explicit confirmation before closure or replacement.
- Import literal legacy `.conf` settings without executing commands, expanding variables or following `source` directives. Skipped lines and repeated keys are reported. This is a partial import, not an effective-state reader or source-preserving round trip.
- Import a conservative static projection of Lua `hl.config` tables and restricted pure local expressions. A separate parser process has CPU/memory/time limits and denies filesystem, networking and process-creation syscalls after startup. No Lua is executed. Per-line import diagnostics explain the unresolved parts.
- Prepare a copy of imported Lua with only changed, existing literal expressions replaced. Source hashes, spans, reparsing and semantic comparison guard the copy; comments and unaffected bytes remain intact. Adding/removing fields and editing derived expressions remain unsupported. Source files are never overwritten.
- Read a manually refreshed scalar snapshot from the compositor selected by your session environment. Runtime directories, held process lifetime, executable fingerprint, held socket-node identity and peer credentials are checked. Only `j/version` and `j/descriptions` can be sent. Exact audited commits receive typed observations; unsupported formats stay raw or unknown.
- Save private `.hye` workspaces in `$XDG_STATE_HOME/hyprland-editor` (normally `~/.local/state/hyprland-editor`), including draft values, exact upstream/catalogue identity and an imported Lua source snapshot. Reopening reparses snapshot bytes in the restricted worker; it never reopens recorded original paths or trusts saved parser spans. Value-only JSON drafts remain importable.
- Verify an explicitly selected original source or identical relocated copy. The editor checks descriptor-derived device/inode/owner/mode/timestamps and SHA-256; changed content is rejected. Recorded ownership is provenance, not permission to overwrite a file. Source identity changes require saving, and late saves cannot mark newer edits as saved.
- Inspect the 1,453 source-audited feature contracts and the engineering blueprint.

## Not available yet

Complete Lua interpretation/include graphs, source-field insertion/deletion, multi-file/source dependency graphs, changed-revision reconciliation, event-based live refresh, multi-instance selection, structured feature controls, plugins, live apply, display recovery, sandboxed compositor preview, distribution packages and release certification. Unknown defaults stay visibly unresolved. Bounds are literal source metadata, not a guarantee that combinations are valid on your system. Hardware behavior has not been tested.

Saved workspaces contain the full imported Lua source and path/ownership evidence; keep them private when sharing reproductions. Saving a draft never targets `hyprland.conf` or `hyprland.lua`. The application has no config publication API. If your own configuration explicitly watches the app's independent state directory, choose a separate state root before running it. Draft export contains only overrides and must not replace your full configuration. Source-copy export contains preserved imported text, including opaque statements, and needs separate review before any manual integration.

## Build and run

Linux (pidfd support for inspection, close_range/seccomp support for Lua workers), Rust 1.90, GTK >= 4.12 and libadwaita >= 1.5 are required. No web server, browser runtime, background daemon or root access is required.

On Ubuntu 24.04:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
```

On Arch Linux:

```sh
sudo pacman -S --needed rust pkgconf gtk4 libadwaita
```

From the repository, using a working Rust toolchain:

```sh
cargo build --locked --release
./target/release/hyprland-editor
```

The repository pins Rust through `rust-toolchain.toml` for rustup users. Existing system Rust installations must meet the stated compiler requirement. GTK development packages are needed at build time; GTK and libadwaita runtime libraries are needed to run the resulting binary. It can open as a draft workspace on any graphical Linux desktop; Hyprland itself is not required for this slice.

A saved workspace can be checked without opening the GUI:

```sh
./target/release/hyprland-editor --check-workspace /absolute/path/to/workspace.hye
```

The result describes draft/static-source checks and explicitly reports that runtime behavior is unverified. Schema mismatches require migration; this command does not update the file.

## Development

```sh
cargo fmt --all --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
python3 scripts/check_catalogue.py
python3 scripts/implementation_report.py --check
```

Pure editing/storage tests need no display. Native UI smoke testing is separate. See [CONTRIBUTING](CONTRIBUTING.md), [SECURITY](SECURITY.md), [coverage and roadmap](docs/roadmap.md), [next implementation gates](docs/implementation-status.md), [full completion register](docs/completion-register.md), and [engineering blueprint](docs/engineering-blueprint.md).

## Architecture and memory

`editor-core` owns validation, static source projection, span-based copy preparation, Lua encoding and private draft storage. `editor-discovery` owns the fixed read-only IPC interface. `editor-app` owns the native interface and launches its own inode as a restricted parser worker through `/proc/self/exe`. There is no compositor mutation interface. Background results are discarded when the draft generation changes.

Old JSON drafts contain values only. New `.hye` workspaces retain immutable source bytes and file identity evidence, with an exact catalogue fingerprint. Parser reports remain in memory and are recomputed on reopen. The GUI currently supports one Lua root; additional documents and legacy source snapshots are rejected explicitly rather than discarded. No include file is opened automatically. Live observations are point-in-time reads, kept separate from draft values; they can become stale. File source projections do not establish effective runtime values.

The setting list uses GTK's recycled list items; only the selected setting builds its form. Feature contracts load on demand. Undo history is bounded. No polling or resident helper is installed. The 100 MiB proportional-memory budget from the blueprint is a target, not a measured promise; see [validation](docs/validation.md) for actual checks.

## Provenance and license

[Catalogue provenance](data/provenance.json) pins the audited Hyprland and wiki commits. `audited-main` is not automatically the latest version. Generated metadata records link each setting to its source. [Third-party notices](THIRD_PARTY_NOTICES.md) explain the distinction between source-derived facts and executable dependencies. Original project code is MIT licensed. Contributions should preserve upstream provenance and never treat the audit catalogue as runtime-certified.
