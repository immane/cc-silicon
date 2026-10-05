# Wave 2 Slice 12: PP Directive-Dispatch Freeze (`/21`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/21` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/20` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/21` delta), [T03](T03_PREPROCESS_CHIPS.md) PP05/PP26 rows, [PP scan slice](PP_FULL_SCAN_SLICE.md) (shared predicate) |
| Non-goal | Conditionals/macros/include (later slices), `#warning` non-fatal protocol (no wire exists for it), multi-line-comment-then-directive classification (documented limitation L1) |

## 1. What this slice is

Two new workers establishing directive handling: `PpDirectiveChip`
(PP05) groups the committed pp-token stream into lines, classifies
directive lines, fans out one `PP26` child per `#error` line and awaits
them all (proven SE_BIN enqueue/await-all pattern); any other directive
fails fast as explicit `Unsupported`. `PpDiagnosticChip` (PP26) reports
one `#error` line by failing with its message (the diagnostic IS the
product). No new record families, no schema change.

## 2. Directive-line recognition (frozen)

A line is a directive line iff its FIRST pp-token is `#` or `%:` AND the
`#` passes the raw walk-back over the committed source bytes: walk back
from the `#`'s raw offset over `[ \t]`; a `\n` or input start means
directive; a same-line `/*...*/` (no newline inside) is skipped once and
walking continues; anything else means NOT a directive (commented-out
`// #define`, mid-line `#`, `/*c` unterminated, multi-line comment tail).
Rationale: PP03 replaced `//` comments with spaces, so the check MUST use
raw bytes — a `//`-killed directive must stay dead. The multi-line-comment
tail case (L1) conservatively misfires to non-directive (loud downstream
parse failure, never silent; recorded gap).

Directive name = the next token if `Identifier`, else malformed (`Fail`,
error). Name table (PP diagnostic taxonomy F4, frozen here):
`error` → PP26 child; `warning` and every other name → explicit
`Fail(Unsupported)` naming the deferred owner (macros/conditionals/
include/pragma/line). The taxonomy table itself is the deliverable even
where bodies are deferred.

## 3. PP05 rules

- Payload: all committed pp-token refs (LEX_INTERN pattern). Groups lines
  via token spans → raw offsets (first token per raw line).
- Zero directive lines → `Ack` (M1 sources flow through untouched).
- Any non-`error` directive → immediate `Fail(Unsupported)` naming it
  (fail-closed; a file needing macros CANNOT be miscompiled).
- Malformed (`#` + non-identifier) → `Fail` error. A bare `#` line
  (null directive) is an explicit no-op, never an error.
- Else (only `error` lines): enqueue one PP26 child per line (owner
  PP26_CHIP, payload = line refs after `#`) + `AwaitChildren` (SE_BIN
  pattern verbatim). Resume rule follows the frozen `/9` join, NOT an
  aggregate: when any awaited child is `Failed`, the join itself fails
  this task once reusing the FIRST failed child's committed diagnostic
  (`poll_await_joins`, no new record) — so the surfaced message IS the
  first `#error` line's message, deterministic in enqueue order. The
  resume-compute aggregate branch stays as unreachable-defensive code
  only (it runs solely in the all-`Completed` case, which PP26 never
  produces); no new diagnostic is minted on the failure path.
- Reads: Tasks `active.*`, Pp `tokens`, Sources `bytes`+`spans`,
  Diagnostics `entries`. No writes.

## 4. PP26 rules

- Payload: exactly one directive line's refs (post-`#`). First ref must
  be Identifier `error`, else protocol fault (wrong dispatch).
- Message = remaining token spellings joined with single spaces (empty →
  `"error directive"`). Always `Fail`s with that message (error group,
  `Task` 4). This never-complete worker is by design; its negative-only
  acceptance is pinned.
- Reads: Tasks `active.*`, Pp `tokens`. No writes.

## 5. Frozen registration

- `preprocess.directive` (`PREPROCESS` local 20) + `preprocess.diagnostic`
  (local 21), `Frozen`; `pp_directive_slice()` registry (34 entries,
  cumulative over `vf01_slice()`); stage rows (`→ 1` both); routed layers 1;
  `PP05_CHIP = ChipId(23)`, `PP26_CHIP = ChipId(24)`; no writes so no
  allowlist rows (wave-gated via `is_pp_directive_slice_kind`).
- Same schemas. Hash rules: `pp.directive-dispatch-lines`,
  `pp.error-fails-message`, `pp.diagnostic-taxonomy-frozen`.

## 6. Freeze items

| # | Item | Acceptance |
|---|---|---|
| W1-1 | Directive recognition §2 (real lines, `//`-killed dead, mid-line dead, L1 documented) | unit + tick cases |
| W1-2 | Taxonomy: `error`→child, `warning`/others→named `Unsupported`, malformed→error | per-name cases |
| W1-3 | Fan-out + await-all under the frozen join (first failed child's diagnostic reused, deterministic) | multi-error surfaces first message, no new record |
| W1-4 | PP26 negative-only: exact message join, empty-message default | message cases |
| W1-5 | Registration freeze (kinds/registry/stage/layer/chips, manifest gates) | freeze gates green |
| W1-6 | Worker template ×2 (pure computes, lint-clean, ZST, stage/layer) | `c21_directive` green |
| W1-7 | M1 regression: directive-free sources Ack through PP05 (incl. driver) | chain + `candidate` green |

## 7. Execution record

Implemented by two parallel chip owners against this freeze
(`pp_directive.rs`, `pp_diagnostic.rs`), integrated by the T01
integrator. Verified by `compiler/tests/c21_directive.rs` (8 tests:
kind/stage/registry freeze, clean-source + null-directive Ack, `#error`
message with frozen-join propagation, taxonomy fail-fast per name,
malformed typed failure, commented/mid-line dead lines, stage/layer +
manifest gates, replay determinism). One freeze-design correction during
integration: the resume-aggregate rule contradicted the frozen `/9`
join (`poll_await_joins` fails the waiter reusing the first failed
child's diagnostic), so the freeze now records join-reuse as the rule
and the aggregate branch as unreachable-defensive; plus a null-directive
(`#` alone) explicit no-op rule. The `candidate` driver runs a PP05 step
(M1 sources Ack; `#error` surfaces exit 1). Both deliveries arrived
lint-clean; registry chaining fixed at integration (`pp_directive_slice`
extends `vf01_slice`, 34 entries, keeping one linear cumulative chain).

## 8. Explicitly deferred

- Conditionals (PP19–22), macros (PP06/09–16), include (PP17/18),
  pragma/line (PP23/25) — taxonomy names them, bodies later.
- `#warning` non-fatal diagnostics (no protocol wire; needs a design).
- L1: multi-line-comment tail + same-line directive → conservative
  non-directive (loud downstream failure, recorded).
- `%:`-only edge spellings beyond the frozen table (none: table is full).
