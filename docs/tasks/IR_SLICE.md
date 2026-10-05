# Wave 2 Slice 6: IR Function-Lowering Freeze (`/15`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-05** as `t01-c01-c06/15` (hash `d6c06cc4…2ef05`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/14` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/15` delta), [T09](T09_IR_LOWER_CHIPS.md) M1 shape checklist, [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-CL-05` |
| Non-goal | Arithmetic/lowering beyond `Constant`/`Return`, `Add` emission, FunctionEnd/IR28, phase-2b hook, conversions in IR, target code |

## 1. What this slice is

One worker (`IrFunctionChip`, IR01/IR02/IR03/IR19 slice scope) turning
one committed `FunctionDefinition` node into one `Function`, one `Block`,
one `Value`, and two `Instruction`s (`Constant` + `Return`). The
`Constant` carries the fold's `ConstId` verbatim and never recomputes the
value. This completes **`M1-CL-05`**: the T07→T08→T09 handoff runs on real
tasks from committed inputs, and T09 consumes the same committed
`ConstRecord` the fold produced.

## 2. Freeze items

| # | Item | Acceptance |
|---|---|---|
| R1-1 | `FunctionRecord { symbol, signature, entry, linkage }`, `BlockRecord { function, ordinal }` (instructions derived, never stored), `ValueRecord { ty }` (producer is the unique result, never a back-link), `InstructionRecord { op, block, operands, immediate, result }`, `IrOp {Constant, Return}` | struct + snapshot round-trip |
| R1-2 | Exactly-one-Const rule: zero committed consts is missing input, more than one is ambiguous; both fail loudly, never a guessed constant | negative tests |
| R1-3 | `ir.function` (`group 8`, local `16`, `Frozen`) + `ir_slice()` registry; stage row (`function → 5`); four allowlist rows; `IR_FUNCTION_CHIP = ChipId(16)` | kind/stage/allowlist/layer gates green |
| R1-4 | `AppendRecords` materialization for the four IR families (1:1 bodies, per-arena capacity, predicted refs) | shared-path negatives + capacity |
| R1-5 | Snapshot bodies + `IR_OP_NAMES` + four record-field lists in the hash | encode round-trip + replay determinism |
| R1-6 | Worker template: `IrFunctionChip` (`IrFunctionInput` projection, pure `compute` with no arithmetic, ZST, stage/layer, lint) | `c15_ir` 7 tests green |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c15_ir.rs`
(7 tests: kind/stage/allowlist freeze, exact M1 shape with the
same-`ConstId` handoff, missing-fact and ambiguous-constant negatives,
non-function/malformed negatives, stage/layer + manifest gates, snapshot
replay determinism). Chain inputs come from the real SE slice.

## 4. Explicitly deferred

- FunctionEnd/IR28 + the T01-owned phase-2b hook (OB-26 stays open).
- `Add` emission and every other op/lowering chip (IR04–IR27, IR29).
- `ir.*` rejection rule-id inventory (`ir.op-immediate-type` etc.).
- Conversions in IR, target code (T11), interpreters (VF12).
