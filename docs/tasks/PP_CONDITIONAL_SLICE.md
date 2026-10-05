# Wave 2 Slice 13: PP Conditional-Inclusion Freeze (`/22`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/22` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/21` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/22` delta), [T03](T03_PREPROCESS_CHIPS.md) PP19–PP22 rows, [directive slice](PP_DIRECTIVE_SLICE.md) (recognition predicate reused) |
| Non-goal | Macro definitions/invocation (PP06/09–16, next), include (PP17/18), separate PP20/PP21/PP22 tasks (folded, see §6), target-width semantics |

## 1. What this slice is

One new worker, `PpConditionalChip` (PP19), evaluating conditional
inclusion over the committed pp-token stream. It tracks the conditional
stack, evaluates `#if`/`#elif` expressions with a chip-local PP-int
evaluator (recursive descent + `defined`, both folded from the PP20–PP22
catalog rows with promotion criteria below), and completes
`Record`s of the ACTIVE-line token refs (active directive lines INCLUDED
verbatim — downstream PP05/PP06 see them; inactive lines dropped).
Pipeline order (frozen): SCAN → DIRECTIVE is amended — PP19 runs on the
full stream, PP05 dispatches on PP19's ACTIVE output, LX consumes the
post-PP28-equivalent filtered refs supplied by the caller (driver/tests;
a control chip later). No new record families, no schema change.

## 2. Conditional stack (frozen)

Levels carry `{parent_active, taken, else_seen}`. Directives recognized
with the `/21` walk-back predicate (same line grouping, chip-local copy
per the shared-helper precedent):

- `#if EXPR`: malformed/empty expression → `Fail` error. Evaluate
  (always, even in inactive regions — fail-closed, matches GCC on hard
  errors); `active = parent_active && value != 0`; push
  `{taken: active, else_seen: false}`.
- `#ifdef X` / `#ifndef X`: `X` must be `Identifier`, else malformed
  `Fail`. `defined(X)` is frozen-`false` (no macro table exists; `-D`
  deferred) — so `#ifdef` takes nothing and `#ifndef` takes all, pinned
  by tests until PP06 amends this rule.
- `#elif EXPR` (level open, else `Fail`; after `#else`, `Fail`): if the
  level already `taken` → inactive; else evaluate, `active =
  parent_active && value != 0`, `taken ||= active`.
- `#else` (level open, else `Fail`; second `#else` → `Fail`):
  `active = parent_active && !taken`, `taken = true`, `else_seen`.
- `#endif`: pop (empty stack → `Fail`). EOF with a non-empty stack →
  `Fail` (unterminated conditional).
- Any other directive line → explicit `Fail(Unsupported)` reusing the
  `/21` taxonomy owners (macros/conditionals... — wait, conditionals ARE
  this chip: `if/ifdef/ifndef/elif/else/endif` handled; `define/include/
  pragma/line/warning/error` → deferred owners; `error` lines in ACTIVE
  regions are KEPT for PP05 (not failed here!). `#error` in an active
  region passes through to PP05; in an inactive region it is dropped.
- Malformed directive (`#` + non-identifier, `#if` with no expression)
  → `Fail` error even in inactive regions (fail-closed; constraints
  apply in skipped groups per C11 6.10).

## 3. PP-int expression evaluator (frozen, chip-local)

Recursive descent over expression token refs (pp-tokens, positions, no
`#`): `|| && | ^ & == != < > <= >= << >> + - * / %` (C precedence),
unary `+ - ~ !`, parentheses, ternary `?:`, `defined id` /
`defined ( id )`, identifiers (undefined → `0`, frozen), integer
spellings (decimal, `0x` hex, `0` octal; character constants →
explicit `Unsupported`). All arithmetic is exact `i128`; division/modulo
by zero, negative/oversized shifts, and overflow → typed `Fail` (never
wrap, never silent). `defined(X)` is frozen-`false` (see §2).

## 4. Output contract

`Complete(Records)` carrying, in stream order: every pp-token ref on
ACTIVE lines (directive lines in active regions INCLUDED) plus the EOF
token ref. Inactive-region tokens (directive or not) are dropped. Empty
active set (outside EOF) is legal (e.g. whole file skipped) — still
completes with `[EOF-ref]`.

## 5. Frozen registration

- `preprocess.conditional` (`PREPROCESS` local 22), `Frozen`;
  `pp_conditional_slice()` registry (35 entries, cumulative over
  `pp_directive_slice()`); stage row (`→ 1`); routed layer 1;
  `PP19_CHIP = ChipId(25)`; no writes so no allowlist rows (wave-gated
  via `is_pp_conditional_slice_kind`). Same schemas.
- Hash rules: `pp.conditional-stack`, `pp.expr-ppint-exact`,
  `pp.defined-frozen-false`.

## 6. Catalog folding (explicit, not silent)

PP20 (`defined`), PP21 (parse), PP22 (evaluate) ship as tested pure
helpers inside `pp_conditional.rs`, NOT as tasks: no task boundary needs
them yet (values never cross tasks — evaluation is subroutine to the
conditional decision). Promotion criteria (all required): a consumer
task needs expression values independently (e.g. macro-table `defined`
queries from another phase, or expression caching across ticks), plus a
frozen value-record family. Until then, folding is the honest shape.

## 7. Freeze items

| # | Item | Acceptance |
|---|---|---|
| X1-1 | Stack machine §2 (`#if/ifdef/ifndef/elif/else/endif`, nesting, unterminated/EOF, stray errors) | nesting + negative cases |
| X1-2 | Expression evaluator §3 (precedence, ternary, shifts, div-zero/overflow fails, char-literal unsupported) | operator-matrix + failure cases |
| X1-3 | `defined` frozen-false + `#ifndef`-takes-all (+PP06-amends marker) | pinned true/false cases |
| X1-4 | Output contract §4 (active incl. directives, EOF always, empty-active legal) | ref-list assertions |
| X1-5 | Inactive-region `#error` dropped; active `#error` kept for PP05 (chain test) | PP19→PP05 chain green |
| X1-6 | Registration freeze (kind/registry/stage/layer/chip, gates) | freeze gates green |
| X1-7 | Worker template (pure compute, lint-clean, ZST, stage/layer) | `c22_conditional` green |

## 8. Execution record

Implemented serially by the integrator (single worker file; no parallel
split point — expression evaluation is subroutine to the conditional
decision). Verified by `compiler/tests/c22_conditional.rs` (8 tests:
kind/stage/registry freeze, taken/dropped/`#elif` branches,
`defined`-frozen-false, expression matrix incl. ternary/hex/octal/
shifts, hard-error failures, active-`#error`-to-PP05 chain, gates,
replay determinism). One pipeline-architecture correction during
implementation: conditional lines are fully consumed (never kept), so
PP05 never sees them; active non-conditional lines pass through. The
`candidate` driver runs a CONDITIONAL step (M1 and `#if 1`-wrapped M1
both model return `5`; fully-skipped bodies fail loudly at PA).

## 9. Explicitly deferred

- Macro definition/invocation/expansion (PP06/09–16, next slices).
- Include resolution (PP17/18); pragma/line (PP23/25); `#warning` wire.
- Target-width PP-int semantics (i128-exact + fail-closed overflow here).
- L1 carryover (multi-line-comment tail classification, from `/21`).
