# Linked compiler numeric observations

The ordered Lua 5.4 model now distinguishes the linked compiler's integer-zero subtraction optimization. Official reference builds turn `nz - 0` into addition of zero, producing positive zero when `nz` is negative zero. A mutable variable holding integer zero retains subtraction and negative zero. The [upstream fix](https://github.com/lua/lua/commit/b35736b9d71182bae67492af2a387b3f6e89fd05) preserves subtraction in both cases. Release strings cannot identify a backported fix.

## Runtime observation

The existing allocator-bounded native adapter compiles the fixed literal `return function(x) return x - 0 end` and dumps its stripped bytecode into a 4 KiB buffer. It never invokes the probe, opens libraries or accepts a caller's source/name. The state closes on success or refusal. The legacy and separate Lua 5.5 images expose different prefixed symbols and retain their existing constructor policies.

The Lua 5.4 compiler profile records the bytecode SHA-256, bundled reference-schema SHA-256 and an optional observed mode. Observation is cached once per process. Unknown fingerprints, conflicting rows or failed probes do not infer behavior from a version string. Affected expressions stop as unresolved when no mode is available. Bytecode fingerprints depend on build/architecture; another legitimate build may therefore require a new reviewed reference.

## Source interpretation

Only the affected pattern—negative floating zero minus integer zero—uses the descriptor inspection. The bounded tree walk models supported Lua 5.4 constant kinds and unresolved true/false jump chains. An executed integer value of zero does not establish compiler constant status: variables, fields and calls remain registers. Parentheses, supported numeric folding and logical expression discharge retain their compiler distinctions. Unsupported forms remain explicit refusals; the walk has depth/visit limits and its work is charged to the existing evaluation budget. This does not establish complete arithmetic, compiler, Lua or selected-compositor conformance.

Current saved graphs, reports and edit plans include this observation in their compiler identity. Normal reconstruction and edit review reject changed or missing observations. Source comparisons retain binary64 sign bits, including a change between positive and negative zero. The small runtime fingerprint table is embedded; the development source/observation corpus is separate.

## Independent references

`scripts/lua_numeric_probe.py --check --model-reference` downloads hash-pinned official Lua 5.4.6/.7/.8/.9 and 5.5.0/.1 into temporary folders. It compares each original compiler with the same compiler containing only the reviewed upstream zero-subtraction guard. Each variant checks three independently compiled/dumped probes, a real output-capacity refusal and 37 trusted literal expression cases, including literal/variable/field/call zero, integer folding, floating zero and logical jumps. Only these development fixtures execute Lua; imported configuration never executes.

For each Lua 5.4 variant, a separately built reference image links the exact private headers and a uniquely named static library. ELF dependency inspection rejects a system Lua dependency, and responses must match the exact release and fingerprint. The image uses the application's restricted parser syscall policy, closed inherited descriptors and resource limits. Its actual ordered model runs all 37 cases under both audited Hyprland snapshots. Lua 5.5 variants remain native references; they are not admitted into the Lua 5.4 model.

The local matrix passes 444 native expression observations and 592 restricted-model comparisons. Native right-hand value-kind controls are checked separately. All downloaded sources and temporary builds are removed. CI repeats the four Lua 5.4 model/reference pairs in independent jobs and both Lua 5.5 native pairs in their target-reference jobs. These finite matrices do not prove unrelated arithmetic or full sandbox security.

## Saved workspaces

The 80-input model has seventeen exact reviewed predecessors. Explicit migration requires every old compiler field to equal the currently observed legacy identity and requires the old numeric field to be absent, including refusing an injected `null`. It renews the numeric observation, independently rebuilds captured/private sources and discloses possible numeric interpretation changes before acceptance. No compiler release/family migration is granted. Reversing both metadata changes must recover the entire decoded input exactly. Cancel, a new private save and unchanged old bytes retain their existing behavior; unknown models remain refused.

Tests cover both-snapshot native/model comparisons, unknown/conflicting reference lookup, descriptor limits, sign-sensitive source comparison, stale edit-profile refusal, private revision save/reopen, actual module-source transaction/undo, all seventeen predecessors, five preceding active/archive private-layer writers and forged old/current compiler payload refusal. Full language compatibility, Lua 5.5 production integration, graphical preview, live apply/recovery and all original audit gates remain required.
