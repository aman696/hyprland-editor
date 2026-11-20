# Primitive scalar value conformance

The typed import backend now has Bool, Int, Float and String family models plus a private parser-worker batch interface. It compares native parse outcomes with representable editor values. The [registration-aware source analysis](registration-aware-inspection.md) now uses these outcomes with frozen per-setting metadata. Complex codecs, ordinary-form constraint integration and graph ownership remain required before editable multi-file import. All existing `native_values_verified` flags remain false.

## Independent reference

`scripts/scalar_value_probe.py` extracts unmodified `parse(lua_State*)` methods from both hash-pinned source snapshots. Real Lua C-API fixture values enter those methods with explicit inert storage and fixture bounds/name maps. There is no string validator in this primitive reference adapter; literal choice validators are separately checked by the registration probe. Source files, official Lua archives and build products exist only in temporary directories and are removed on exit.

The 592 observations preserve the actual input type, signed integer, binary64 float bits or raw bytes before parsing. They record error code, whether an error message exists, native storage, prior storage, set-by-user state and the post-parse stack type. Tests consume those independent input snapshots rather than asking the editor to evaluate the reference expressions. Rejection checks require the independent native storage to remain unchanged. Full error strings and unordered map diagnostics are not certified.

Native storage aliases are separately pinned to `src/config/shared/Types.hpp` at both commits, SHA-256 `337556a3fbedda3e34f31f600f4a44d4e85f53a04220f3c90d8394f98d1eb638`: INTEGER is signed 64-bit, FLOAT is binary32, BOOL is bool and STRING is a byte string. The fixed parser models have these behaviors:

| Family | Modeled source behavior |
| --- | --- |
| Bool | Boolean, or Lua numeric conversion exactly equal to 0 or 1, including qualifying numeric strings. |
| Int | Boolean/integer with exact integer bounds; other string-compatible inputs use the optional name map. A Lua decimal `3.0` is not an integer. Map lookup uses C-string spelling and skips numeric bounds in the native branch. |
| Float | Boolean/integer comparison with FLOAT bounds occurs in binary32; Lua numbers/numeric strings compare in binary64 before binary32 storage. Decimal and integer inputs can therefore differ at a precision boundary. |
| String | Lua strings and numbers use native C-API string conversion and NUL truncation. An unimplemented validator prevents acceptance being inferred. |

These are family models under supplied codec descriptors. They do not prove that every actual setting's constructor, bounds, map or validator was captured correctly. A separate generated registration schema now retains named choices, validators and version-specific Lua constraint flow. Ordinary scalar/device forms and static projection still need integration with that schema; the original catalogue alone omits some semantics. Color, Gradient, Vec2, CssGap and FontWeight native codecs remain open.

## Conversion boundary and assumptions

Only the editor-owned native conversion function runs inside `lua_pcall`; no user bytecode, Lua closure, metatable, hook, library or object is loaded into its state. This protects allocating C-API operations from propagating Lua longjmp through Rust. A fresh state has room for the two nonallocating trusted-call pushes. Its stack is cleared after each request and its allocator has an 8 MiB ceiling. The reused output buffer and individual byte inputs are limited to 64 KiB. The state is uniquely owned, is not transferable between threads and is closed on drop.

The product `DecodePrimitives` request runs in the existing short-lived worker with cleared environment, closed inherited descriptors, no-new-privileges, seccomp, 512 MiB address-space/8 MiB stack/two-second CPU limits and the parent's three-second deadline. It accepts 1–256 fixed-schema cases and at most an estimated 2 MiB of summed encoded outcomes, inside the existing 4 MiB envelope. Limit failure returns an error, never truncated successful results. No filesystem, network, process creation, configuration publication or compositor action is added.

Conversion requires a recorded matching Lua release/library identity and the `hye-primitive-coercions-1` profile. It also requires the current process's C numeric locale without changing global locale. Worker output records that locale and the library profile. The matrix checks the conversion adapter and all 592 original parse observations against official hash-pinned Lua 5.4.6–5.4.9: 2,368 scalar comparisons, in addition to the existing ordered/path observations. This does not detect the target compositor's library, locale, patches, build options or runtime state. Other platforms, ABIs, locales, full numeric domains, allocator failure paths and fuzz/resource certification remain required.

## Native results and draft representation

Accepted native results retain Bool, Int, binary32 bits or raw C-string bytes. They do not imply file ownership, lossless editability, supported setting registration, successful refresh or effective desktop state. Rejection returns a native error category. Unsupported validators remain unresolved; no input normalization invents acceptance.

Nonfinite floating inputs cross worker JSON as a strict 16-digit binary64 bit object; ordinary finite-number encoding remains compatible. Null, malformed bit strings and unknown fields are refused. Native nonfinite floating results remain binary32 bit patterns, while editor drafts require finite numbers. Invalid UTF-8 and strings above the editor's 16 KiB text limit retain native bytes with an unavailable representation. Finite FLOAT values convert to the exact binary64 representation of native binary32 storage. Nonfinite bounds are refused during descriptor validation and serialization rather than silently becoming absent JSON bounds. Setting-specific policy and edit ownership remain separate checks.

Tests cover the 592 independent comparisons, source spelling/NUL behavior, unknown validators, native versus draft limits, precision branches, reusable conversion state and invalid descriptor/input/output budgets. An actual worker fixture confirms that command-looking text remains inert and that oversized/empty batches are refused. This is backend work; no additional GUI form or live operation is enabled. All original source editing, discovery, isolated preview, guarded apply/recovery, resource and release obligations remain active.
