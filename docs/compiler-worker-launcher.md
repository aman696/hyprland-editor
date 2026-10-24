# Installed compiler launcher and read-only check

The application can check an optional installed Lua 5.5 compiler on demand. **Check Optional Compiler…** reports availability, keeps the GTK loop responsive and explicitly says that source analysis still uses Lua 5.4. It does not switch a saved graph, execute configuration, validate the running compositor or apply changes.

## Package admission

The production lookup is fixed at `/usr/libexec/hyprland-editor/lua55`. It opens every directory component from `/` with directory/no-follow flags and requires root ownership with no group/other writes. The selected `manifest.json` and `compiler` must be single-link regular root-owned files with the same permission restriction. FIFO/device/symlink inputs are refused before reading. No environment variable, source setting, saved path, manifest path or worker reply selects an executable.

The bounded 16 KiB registry has exactly these fields:

```json
{
  "format": "hye-installed-compiler55-1",
  "release": "5.5.1",
  "image_bytes": 1820608,
  "image_sha256": "replace-with-the-exact-built-image-sha256"
}
```

The size above is an illustrative observed build, not a package constant. Packaging must generate both exact size and digest for its own image. Only reviewed 5.5.0/5.5.1 releases and nonempty images up to 64 MiB are admitted. Unknown registry fields are refused. Installation remains a distribution packaging task; the GUI never runs an installer or requests privilege elevation. Local/AppImage/signature-based distribution channels still need their own admission design and testing.

Root-owned package metadata is the current trusted installation channel. The digest proves content correspondence, not publisher identity. Signed releases, dependencies, update ownership and general supply-chain acceptance remain required; an attacker who controls trusted root installation is outside this channel's configuration-input boundary.

## Immutable executable

The existing runtime-image capture verifies exact size, SHA-256 and original identity before/after copying, then seals its data snapshot. The compiler client additionally checks bounded native ELF64 little-endian program headers: native machine/type/version, table extents, executable entry, load extents, no writable executable segments, no executable stack, no interpreter and no external library/search-path dynamic entries. Unknown architectures or unsupported shapes are refused.

Only that approved snapshot is copied into a new executable memfd, mode 0500, sealed against content/size changes and execution-bit changes with `F_SEAL_EXEC`. No source file descriptor or package directory survives into the worker. This path requires the kernel's executable-memfd support; unavailable support yields an error without switching to a pathname or dynamic image. The old Lua 5.4 source model remains available independently.

Private GNU builds use explicit native targets and `-C target-feature=+crt-static`, linking the exact hash-pinned private Lua archive. ELF inspection must show no interpreter or `DT_NEEDED` libraries, and the actual profile must match the requested release. In local reference builds the two images are about 1.8 MiB. They are created temporarily, exercised and removed; no system package is installed. A dynamically linked worker remains usable as the existing development component but is refused by this installed launcher.

## Parent protocol and lifecycle

The parent refuses privileged identities or retained capabilities and creates a fresh 32-byte nonce using bounded nonblocking kernel entropy. It validates typed input bounds before launch and accepts only the existing profile/compile/primitive/fixed-denial operations. The shared wire types have strict decoding in both directions. Empty profile/probe variants are represented as empty structs: a regression demonstrated that Serde's unit variants could ignore extra fields despite the enum annotation. Valid version-1 wire shapes remain unchanged; hidden fields are now refused.

The child executes the held sealed image with an empty environment. The pre-exec path uses only fixed async-signal-safe syscalls: parent-death kill with a parent-race check, no-new-privileges, close-on-exec for unrelated descriptors, 128 MiB address space, two CPU seconds, eight MiB stack, no core dumps and sixteen descriptor slots. The static worker applies its closed compiler role policy before consuming input. It never invokes imported Lua.

Parent stdin/stdout/stderr are nonblocking. One loop advances request writes, drains both outputs, observes cancellation/exit and enforces one three-second parent deadline. It uses no blocking reader/writer threads, waits for both output EOFs and refuses early exit, malformed/extra/truncated output, more than 4 MiB stdout or 64 KiB stderr. Per-iteration reads/writes are bounded. Cancellation, timeout, error and unwind keep an owned child guard, kill/reap it and close owned pipes. Parent shutdown also requests kernel parent-death killing. Whole-application fatal crash behavior and broader hostile lifecycle fuzzing still require dedicated acceptance evidence.

Replies must match protocol, exact request-byte SHA-256, nonce, operation, actual release/family/library identity, C numeric locale and private random seed. Imported-execution/full-language/selected-compositor verification claims must remain false. Conversion results must match input count and flag/text/aggregate limits; valid compilation cannot contain a diagnostic. A reply is data and cannot supply another executable or syscall policy.

## GUI lifetime

One check per window runs in a background thread; GTK receives a bounded message. Close, destroyed/invisible parents and changed draft generations invalidate results. The GTK loop joins only an already-finished thread and removes the parent signal handler. A discovered GTK lifetime issue is covered: destroying a window can leave its object alive through other references, so object destruction alone is insufficient; parent visibility is checked too. Duplicate starts are disabled while the check finishes.

Package capture occurs off the GTK thread with byte bounds. General interruptibility of trusted-filesystem reads, representative full-workflow latency/PSS, package updates and all-operation cancellation remain open gates. This component is not a complete sandbox or resource certificate.

## Evidence and limits

Seven ordinary parent cases cover exact correlation/strict schemas, duplex progress, child stalls/deadline/reaping, pre/active cancellation, unwind, both output floods, malformed/failed output and ELF/dependency/digest refusal. A separate explicitly invoked fixture runs the actual static worker after its original input is truncated, checking profile, compile-only effects, invalid source, exact primitive bits/strings, denial and cancellation for both official releases. Each release also reruns all ten process cases with the ordinary and static worker images.

The private graphical workflow covers success/unavailability presentation, duplicate starts, close, generation changes and parent destruction. Its probes are trusted controlled test closures, separate from positive installed-package conformance. The screenshot is a fixture presentation, not a claim that a package was installed on the user's host.

`scripts/compiler_install_probe.py` supplies a separate **ephemeral CI-only** root setup. It copies exact hash-checked built inputs into a temporary package hierarchy, then drops supplementary groups/GID/UID before running the Rust admission test. A good package must run; writable directories/manifests, foreign ownership, symlink image/registry, FIFO, hardlink, wrong hash and unknown fields must fail. It removes the hierarchy without touching `/usr`, policies, controllers or desktop configuration. Local execution evidence for this privileged profile is intentionally absent; hosted results must be observed for the exact published head. The test-only directory adapter is not callable by production configurations.

The reconstruction model remains 80 inputs with seventeen reviewed predecessors: this diagnostic does not change legacy reconstruction or saved compiler profiles. Production Lua 5.5 ordered evaluation/grammar/primary codecs, profile selection/migration, complete graphical source editing/export, actual preview, live apply/recovery, representative resources, accessibility/packaging/security and every original audit contract remain required.
