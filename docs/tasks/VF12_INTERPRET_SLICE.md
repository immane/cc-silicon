# Wave 2 Slice 8: VF12 Symbolic-Interpret Freeze (`/17`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/17` (hash `c519c5b4…2043`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/16` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/17` delta), [T13](T13_VERIFICATION_CHIPS.md) VF12 row + M1 qualifier, [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-CL-05` |
| Non-goal | Multi-block functions, wider op coverage, `Add` emission, target execution, symbolic-execution engine, before/after-opt differential |

## 1. What this slice is

One read-only verifier closing the M1 meaning gap: `Vf12Chip` (VF12)
reads one committed `Function` and symbolically models the M1 covered
subset — one entry block (ordinal 0), `Constant` then `Return` — without
executing target code and without target widths. The `Return` operand
must be the value the `Constant` materializes from the committed
`ConstRecord`; the chip completes `Record` of that `ConstId` (an already
committed record, so no predicted-ID row is needed). Any non-covered
shape fails loudly and is never a pass. The acceptance fixture is the
full PP→LX→PA→TY→SE→VF06→IR chain ending in **`models return 5`**.

## 2. Freeze items

| # | Item | Acceptance |
|---|---|---|
| S1-1 | Symbolic M1 coverage: exactly one block (ordinal 0, equals `entry`, owned by the function); exactly two instructions in ascending-ID order (`Constant` then `Return`); constant takes no operands, names the modeled const + returned value; return names exactly that value, carries no payload; value type is canonical `int` | positive + shape-mutation negatives |
| S1-2 | No recompute, no appends: the chip contains no arithmetic and emits no `AppendRecords`; arena counts are unchanged across the tick; the completed `Const` value is the fold's `vec![5]` | count + value assertions |
| S1-3 | `verification.ir_interpret` (`VERIFICATION` local `17`, `Frozen`) + `vf12_slice()` registry (30 entries, cumulative over `pp_slice`); stage row (`→ 6`); routed layer 6; `VF12_CHIP = ChipId(20)`; no writes so no allowlist rows | kind/stage/allowlist/layer gates green |
| S1-4 | No schema change: the slice reuses the PP-slice schema (all read families frozen in `/11`–`/15`); no new record bodies, no snapshot encoding change | schema-reuse assertion |
| S1-5 | `Complete` carries `Record(Const)` of an already-committed const (below the preflight base, so the predicted-ID rule passes it through); non-`Running` state, wrong kind, malformed payload, and dangling function refs fail loudly | negative cases |
| S1-6 | Worker template (narrow projection, pure `compute`, ZST, stage/layer, lint) | `c17_vf12` 6 tests green |
| S1-7 | Hash rules `vf12.interpret-symbolic-m1`, `vf12.unsupported-never-pass` | freeze + replay determinism |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c17_vf12.rs`
(6 tests: kind/stage freeze, end-to-end model of return `5` with
unchanged arena counts, malformed/missing negatives, compute-level
non-covered-shape negatives, stage/layer + manifest gates including the
stale-registry rejection, snapshot replay determinism).

## 4. Explicitly deferred

- Multi-block functions and any op beyond `Constant`/`Return` (the frozen
  `IrOp` set has only the two M1 variants; a wider covered set needs a
  new slice with new hash rules).
- Before/after-optimization differential runs (needs the T10 pipeline).
- Target execution and ABI behavior (T11, probe-gated); this
  interpretation is symbolic Part A only, not a runner.
- VF01–VF05/VF07–VF11/VF13–VF14 (still planned; the H6/H9 batch fixtures
  remain T01/T02/T13 co-freeze-pending and are untouched by this slice).
