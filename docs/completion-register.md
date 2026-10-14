# Full-project completion register

The goal remains the complete editor described in the audit and blueprint. This register preserves the full scope, including all 1,453 feature contracts, 756 documentation candidates, 24 blueprint sections, 12 engineering work orders, 24 mandatory security gates, nine implementation stages and ten shared model invariants.

**The project is not complete.** None of the full feature contracts has a release-wide certificate. The working foundations are narrower than those contracts. Code/test references identify evidence to inspect; a reference does not establish that a test ran, that it covers a whole requirement or that a physical behavior was certified.

The [machine-readable register](completion-register.json) retains every contract ID, version membership, required editor/codec and named test gate. It also retains the source and exact text hashes for blueprint section scopes. Pending documentation candidates are not silently counted as implemented settings. The existing [implementation status](implementation-status.md) supplies the next work sequence.

| Engine | Full contracts | Full acceptance |
| --- | ---: | --- |
| ANIMATION | 42 | Not proven |
| BIND | 25 | Not proven |
| COMPONENT | 214 | Not proven |
| DEVICE | 44 | Not proven |
| GESTURE | 13 | Not proven |
| INSPECTOR | 211 | Not proven |
| LEGACY | 92 | Not proven |
| MONITOR | 23 | Not proven |
| RULE | 94 | Not proven |
| SCALAR | 403 | Not proven |
| STARTUP | 15 | Not proven |
| WORKFLOW | 259 | Not proven |
| WORKSPACE | 18 | Not proven |

## Current foundations and limits

- **binding/submap draft editor**: 51/52 public dispatcher constructor forms, chords/flags, focused recording, literal submap rename/duplication and escape drafts, static findings, private persistence and inverse undo; no complete selector/workflow builders, effective conflicts/device intersections, source editing, live delivery, apply/recovery or compositor preview certification.
- **device draft editor**: 43 fields and name selector, author category hints, linked XKB choices from installed metadata, numeric acceleration/scroll builders, ordered drafts and private persistence; no hardware capability discovery, effective global/default merge, tag-to-bind analysis, keyboard compilation, source editing, live input recovery or activation/preview certificates.
- **monitor draft editor**: 22 versioned field controls and output selector, ordered drafts, explicit logical geometry, static Lua import/export and private saved snapshots; no hardware detection, effective rule resolution, source-span mutation/reconciliation, ICC decoding, monitor recovery, live apply or compositor preview certification.
- **scalar draft forms**: initial values for 755 forms; not full per-field conformance, runtime activation or accessibility certification.
- **static Lua and parser isolation**: restricted source projection and existing literals; no include/scope completeness or preview isolation proof.
- **private source-backed draft snapshots**: single Lua root, symlink-chain evidence and reviewed static revision reconciliation; no generated ownership/include graph, multi-file merge, persistent history/checkpoint journal or watched-path certification.
- **read-only selected-session inspection**: version/descriptions fixtures; no actual host compositor certificate, other inspectors or event subscription.

## Maintaining the register

Run `python3 scripts/implementation_report.py` after changing the audit manifest or blueprint. CI runs `--check` to reject drift. This check only proves register consistency with the pinned audit; it does not certify upstream completeness, feature behavior or release safety. Future certification records must bind an exact source/schema/binary/profile identity to requirement-specific execution results before a full contract advances beyond Specified.

The source persistence milestone does not close E03/E05: lossless include/symlink graphs, full multi-file changed-revision conflict handling, actual watched-path isolation, source insertion/deletion and recovery still need implementation and verification. Green ordinary tests cannot substitute for those gates.
