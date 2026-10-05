# Wave 2 Slice 9: VF05 Token-AST Invariant Freeze (`/18`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/18` (hash `7ceeaee5…8256`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/17` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/18` delta), [T13](T13_VERIFICATION_CHIPS.md) VF05 row, [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-LX-01..07` + 9-node TU |
| Non-goal | Wider syntax, multi-TU arenas, token emission past M1, span provenance beyond committed tokens |

## 1. What this slice is

One read-only verifier closing the M1 token↔AST link: `Vf05Chip` (VF05)
reads one committed TU node plus the committed token and literal arenas
and re-verifies the syntax contract PA established — parent/children
reciprocity by re-walk (never trusted from walk order), token ranges
contained in the parent range with ordered non-overlapping siblings,
per-kind child counts over the closed 8-kind M1 set, required fields
(`Declarator` name, `IntLiteral` literal whose origin token is the leaf
token), all referenced tokens committed, and a unique trailing EOF.
Completes `Ack`; any gap fails loudly. The acceptance fixture is the
real PP→LX→PA chain ending in **`Ack` over the 9-node tree**.

## 2. Freeze items

| # | Item | Acceptance |
|---|---|---|
| T1-1 | Range/order rules: child ranges inside the parent range, siblings ordered and non-overlapping, `first <= last`, every range at or before the unique trailing EOF (exactly one `Eof`, greatest index) | positive + swap/escape negatives |
| T1-2 | Required fields per closed kind table (TU 1 / FuncDef 3 / Spec 0 / Decl 0+name / Compound 1 / Return 1 / BinaryAdd 2 / IntLiteral 0+literal); `IntLiteral` leaves single-token with origin token == leaf token; no other name/literal presence | presence-mutation negatives |
| T1-3 | `verification.token_ast_invariant` (`VERIFICATION` local `18`, `Frozen`) + `vf05_slice()` registry (31 entries, cumulative over `vf12_slice`); stage row (`→ 2`, shared with the checked phase); routed layer 2; `VF05_CHIP = ChipId(21)`; no writes so no allowlist rows | kind/stage/allowlist/layer gates green |
| T1-4 | No schema change: the slice reuses the PP-slice schema (token/literal/node families frozen in `/11`–`/12`); no new record bodies, no snapshot encoding change | schema-reuse assertion |
| T1-5 | Non-TU payload, malformed payload, dangling nodes, and non-`Running` state fail loudly | negative cases |
| T1-6 | Worker template (narrow projection, pure `compute`, ZST, stage/layer, lint) | `c18_vf05` 6 tests green |
| T1-7 | Hash rules `vf05.syntax-ranges-ordered`, `vf05.required-fields-complete` | freeze + replay determinism |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c18_vf05.rs`
(6 tests: kind/stage freeze, end-to-end `Ack` with unchanged arena
counts, malformed/dangling negatives, compute-level contract-break
negatives, stage/layer + manifest gates including the stale-registry
rejection, snapshot replay determinism).

## 4. Explicitly deferred

- Wider C syntax (new node/token kinds need a new slice; the match over
  the closed kind set fails compilation first, which is the tripwire).
- Multi-TU arenas and cross-TU checks (VF01 scope).
- `TokenRecord.literal` forward link production (LX-side; VF05 checks
  the reverse `LiteralRecord.token` → leaf direction only).
- VF01–VF04/VF07–VF11/VF13–VF14 (still planned; the H6/H9 batch fixtures
  remain T01/T02/T13 co-freeze-pending and are untouched by this slice).
