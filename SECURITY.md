# Security policy

Only the newest published development revision is currently maintained. There are no stable releases or guarantees suitable for unattended live configuration changes.

Use GitHub's private vulnerability reporting for a security issue (when enabled), or contact the maintainer privately through their GitHub profile. Do not publish private config files, tokens, screenshots containing secrets, or weaponized configurations in a public issue. Ordinary bugs can use public issues with synthetic reproductions.

## Current boundaries

- Imported text is data. Legacy commands, source directives, variables and arbitrary Lua are not executed.
- Import reads reject non-regular files, final-component symlinks and documents larger than 1 MiB. Intermediate path components follow the local user's selected path; this is not a filesystem sandbox.
- Draft JSON denies unknown structural fields, validates all setting IDs against the selected snapshot, checks value types and finite numbers, and limits text size and gradient stops.
- Draft storage uses an owned state root, a private 0700 app directory, held directory descriptors, no-follow opens, exclusive 0600 temporary files and atomic no-overwrite publication. Draft data is synced before publication. Invalid or symlinked destinations fail closed.
- The app opens no network listener, performs no telemetry and has no compositor IPC or config publication interface. Clipboard export is an explicit user action; the desktop clipboard may be visible to other applications.
- GTK color/font rendering and dependency vulnerabilities remain part of the platform trust boundary. This editor is not a sandbox for malicious native code.

## Before live apply or preview

Implement the blueprint's session pinning, capability separation, watcher handling, transaction journal, independent recovery agent, bounded IPC and isolation probes. Config verification must not execute imported Lua on the host. A failed sandbox probe must not fall back to the user's live compositor. Memory and CPU budgets must be measured. These are implementation gates, not currently completed protections.
