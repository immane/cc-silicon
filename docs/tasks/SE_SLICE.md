# Wave 2 Slice 5: SE Semantic-Check + VF06 Freeze (`/14`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-05** as `t01-c01-c06/14` (hash `881f9a3f…19d44`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/13` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/14` delta), [T07](T07_SEMANTIC_CHIPS.md), [T13](T13_VERIFICATION_CHIPS.md) VF06, [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-SE-01`–`M1-SE-03` |
| Non-goal | Non-`Int` operands, `ConversionPlan`, query-point lookup ordering, automatic File-Enter edge, T09 lowering |

## 1. What this slice is

Three check workers plus one verifier closing the M1 semantic loop on
committed records: `SeLitChip` (literal facts) → `SeBinChip` (binary
facts, forwarding to a real `const_fold` child) → `SeRetChip` (return
identity) → `Vf06Chip` (typed-fact completeness over the checked set).
The binary handoff is the real `M1-CL-05` path up to T09: the fold's
`ConstRecord` is committed by the fold worker and read directly by the
(next-slice) T09 lowering. The acceptance fixture is **`P1-SE-01`**.

## 2. Frozen decisions

| # | Decision | Rationale |
|---|---|---|
| E1-1 | `SemRecord { node, ty, category, effects }`, `ValueCategory {Lvalue, NonLvalue, FunctionDesignator, Void}`, `EffectMask(u32)`; M1 emits `int`/`NonLvalue`/`0` only | Proposal shape minus deferred `conversions` |
| E1-2 | M1 identity is the absence of a plan (no `ConversionPlan` type, no empty-vs-`Identity` ambiguity at slice scope) | TC-02 answered; OB-23/OB-51 |
| E1-3 | Binary check is two-phase: Sem + fold-child + await, then complete on resume; the join consumes nothing | OPEN-02 parent-owns-consumption; T09 reads `Const` directly |
| E1-4 | `semantic.literal_expr(18)` / `binary_expr(19)` / `return_stmt(20)`, `verification.typed_invariant(16)`; all stage 4; chips 12–15; three allowlist rows | Slice registration (locals avoid the Gate 1 semantic codes) |
| E1-5 | VF06 checks the M1 checked set (`IntLiteral`, `BinaryAdd`, `Return`): one fact each, `int`-typed, `NonLvalue`, zero effects; conversion completeness is M1-minimal (nothing to miss) | T13 rev-46 role fixed; registration closed here |
| E1-6 | `Sem` append materialization; `Semantic` diagnostic group (conflict = 1, undeclared = 2) | Commit/snapshot/hash closure |
| E1-7 | Lookup-miss style misses stay T06; non-`Int` operands are chip `Unsupported` | Carrier discipline (no silent success) |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c14_se.rs`
(9 tests: kind/stage/allowlist freeze, literal facts + reuse, the
two-phase binary handoff with a real fold child, operand-ordering
enforcement, return identity, VF06 pass/gap, malformed/dangling
negatives, stage/layer + manifest gates, snapshot replay determinism).
Chain inputs come from the real LX+PA+TY slices.

## 4. Explicitly deferred

- T09 lowering (IR03/IR19) consuming `Const` + `Sem`: completes `M1-CL-05`.
- `ConversionPlan` + non-identity conversions + per-`(node, role)` commit rule.
- Automatic File-Enter edge firing; query-point lookup ordering.
- Full-catalog split (SE03–SE30); nonzero effects; non-`Int` programs.
