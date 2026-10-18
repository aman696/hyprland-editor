# Preserve compound source layout

The source-change editor defaults to **Preserve table layout and comments** for directly selected Vec2, CssGap and Gradient constructors. It builds the proposed expression from the captured syntax, replacing changed components and inserting/removing owned fields while retaining existing comments, whitespace, keys and unrelated fields. No files or live settings are changed. Compare and Keep in Draft use the same independent worker review, revision guards and private persistence as other source changes.

Unchanged primary values retain their original spelling and computations. For example, adding a gradient stop can leave `'#123'` and `angle=45` intact even when the GUI angle contains the native float32 conversion. A gap field such as `right=2.9` can remain intact when the requested side is zero, reflecting the parser's explicit-field integer conversion. The complete merged input must independently parse to the requested native value; preserving extra fields cannot bypass native constraints.

Missing components use explicit keys inserted after the opening brace. This keeps positional array indices stable and prevents a preceding line comment from consuming the addition. New lines follow the source's CRLF style when present. Removed array fields lose their syntax tokens and separator; comments and existing whitespace survive, including comments within keys and values. Existing strings that merely contain comment-like text stay strings. Replaced expressions retain their internal comments in the resulting expression.

## Review boundaries

This is syntax-local preservation, not proof of exclusive ownership. A changed component may remove a modeled call or alter other effects; review reports those differences. The table itself, its aliases, later assignments, unexecuted branches and native compositor behavior are not certified. Existing original captures remain available independently of revisions.

The mode requires a directly selected constructor and exact static keys. Dynamic keys, duplicate keys, expanding positional calls, unsupported key forms or fields created outside the constructor produce a refusal. There is no silent fallback. A user can explicitly choose a full-expression comparison to replace a computed expression and review its consequences. Inherited table definitions and general assignment/alias editing still require a broader ownership editor.

Scalar override reconciliation also selects preservation for directly reached vector/gap/gradient constructors. A layout refusal leaves the override and source unchanged; the user must resolve the expression in the source-change editor. Other scalar expressions retain the existing explicit replacement policy.

Limits remain 512 fields per traversed constructor, four compound levels, 8192 visited syntax nodes for token removal, existing source-size limits and restricted-worker time/data budgets. Gradient controls retain their 32-stop representation limit. These logical bounds do not certify the whole-process memory target.

## Evidence

Nine core cases cover both pinned snapshots, exact unrelated bytes/CRLF/Unicode, ignored boolean/string fields, missing/nil gap sides, internal comments, signed zero, original native-equivalent spellings, effect-removal findings, dynamic/alias refusal and override reconciliation without fallback. The separator matrix additionally exercises 64 old/new length and trailing-separator combinations across both snapshots, retaining every marked comment while independent compiler/model checks verify the result.

The graphical workflow edits and keeps gradient/vector/gap constructors through the default preservation choice, checks the review notice, saves and reopens after removing the original fixture and verifies every marked comment/extra field. It also retains the five-family source edit/undo workflow. Full accepted-domain/source-language coverage, general insertion/deletion/alias ownership, external multi-file reconciliation and persistent history remain unfinished. All original discovery, isolated preview, apply/recovery, security, resource and release requirements remain in scope.
