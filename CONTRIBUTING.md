# Contributing

Start with the coverage table and engineering blueprint. Changes must state which exact Hyprland release or commit they support. This project aims for complete coverage, but does not label audit requirements as implemented features.

Use the pinned toolchain, run the checks in the README, and include tests for parser boundaries, version differences and unsafe inputs when those behaviors change. Keep core changes independent of graphical sessions. Never use personal active configuration files as test fixtures; use synthetic inputs.

A new control needs a typed representation, validation, unset behavior, version scope, codec tests and a visible limitation when effective state cannot be observed. A live operation additionally needs verified session identity, rollback semantics and a recovery plan before its UI can be enabled. Do not add generic shell execution, Lua evaluation, remote APIs, root helpers or silent config rewriting to simplify an implementation.

Use small patches with concrete behavior and validation results. Report vulnerabilities privately as described in SECURITY.md. User-facing labels should explain the effect without requiring users to write configuration syntax.
