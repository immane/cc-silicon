# A15: IR, CFG, Memory, Effects, and Versioning Contract Requirements

| Field | Value |
|---|---|
| Status | **Stage A design deliverable; not a contract freeze, implementation authorization, or acceptance claim** |
| Task | A-PTS-05 under [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) |
| Date | 2026-11-07 |
| Contract baseline | `t01-c01-c06/39` (`compiler/contracts/CONTRACT_VERSION`; `compiler/src/contract.rs:38-44`) |
| Scope | Smallest shared IR/CFG/memory/effect/versioning contract needed for V1–V3 and later T10 optimization |
| Write scope | This file only. Source, tests, contracts, Cargo files, `mod.rs`, shared indexes, and user drafts are read-only for this task |
| Authority | `AGENTS.md` §2 precedence: user instruction > accepted ADR > accepted contract > architecture > implementation. This report is below an accepted T01 freeze and does not amend `/39` |

**Purpose.** Produce the contract-requirement package the T01 integrator can turn into one small,
ordered freeze package for IR. The package must be sufficient for the V1 integer/local/assignment
slice, the V2 CFG slice, and the V3 function/memory/call slice, without inventing a private IR,
target widths, or target ABI values.

**Summary of findings.**

- `/39` freezes only the M1 `Constant`/`Return` IR. V1–V3 need one **record extension**, not new
  record families: `InstructionRecord.successors/effect/detail` and `FunctionRecord.form`.
- The largest blockers are semantic, not cosmetic: `EffectMask` has no assigned bits and is
  zero-only; there is no CFG, memory, call, or dynamic-stack shape; `ConstRecord` has no `ty`;
  `TypeKind` has no pointer/array; `LayoutId` is reserved; `StorePatch` replacement is log-only;
  and there is no per-function IR version.
- The recommended minimal versioning model is a per-function `IrVersionStore`
  (`revision` + `memory_epoch`) updated by commit, with `StoreVersions[Ir]` as the coarse guard.
- The recommended minimal mutation model is a typed `MutateRecords` proposal; `StorePatch` cannot
  carry a replacement body.
- V1–V3 verification can use result-only `Ack`/`Fail` facts; a durable analysis/verification
  store is deferred to T10/T13.
- No implementation is ready. The ordered freeze package (F-IR-01..03) must land before any
  V1–V3 chip work order.

## 1. Goal, Scope, and Non-Claims

### 1.1 Goal

Define the smallest shared semantic vocabulary and record set that lets independently owned IR
lowering, verification, and optimization chips communicate through the bus:

- one IR representation with explicit functions, blocks, values, instructions, terminators,
  memory access, and effects;
- explicit CFG and value/effect legality rules that are checkable from committed records;
- explicit per-function IR revisions, tombstones, and analysis-invalidation hooks;
- a stable error taxonomy and verification-fact vocabulary;
- a field-level schema, task/result envelope, ownership matrix, and ordered freeze/implementation
  work orders.

### 1.2 In Scope

| Area | Included |
|---|---|
| Records | Function, Block, Value, Instruction, Terminator (as instruction class), Memory, Effect, Call, Object/storage, Switch |
| CFG | Entry, block ownership/order, terminator uniqueness/arity, successor edges, reachability, completion |
| Values | Type binding, producer rules, parameter values, value-category mapping |
| Memory | Load/store, volatile/atomic/order, layout binding, object/storage classes, aggregate copy |
| Calls | ABI-neutral call/return shape, parameters, variadics, indirect calls |
| Dynamic stack | Alloca/stack-save/stack-restore shape and lifetime/cleanup obligations (deferred execution) |
| Versioning | Per-function revision/memory epoch, store-version guard, stale-patch rejection |
| Tombstones | ID lifetime, delete/rewrite mutation, snapshot/replay visibility |
| Invalidation | Analysis fact version keys and commit-owned invalidation hooks |
| Verification | VF07 CFG, VF08 IR, VF09 SSA, VF12 interpreter facts, VF13 replay projection |
| Errors | Commit, chip, dispatcher, verifier carriers and stable rule IDs |

### 1.3 Out of Scope

- Chip implementation, tests, contract edits, `mod.rs`, Cargo, and version/hash changes.
- Concrete target widths, alignments, register classes, AAPCS64 values, stack frame layout, or
  ELF/assembly details. The IR is target-neutral; T11 owns all concrete values and remains
  probe-gated for Part B.
- Machine IR, register allocation, instruction selection, and assembly emission.
- Full C type semantics, bitfields, floating point, vectors, inline assembly, and atomics beyond
  the shape needed to keep the IR extension forward-compatible.
- Any pass-rate, torture, probe, or execution claim.

### 1.4 Non-Claims

- `/39` remains the current frozen artifact. Nothing in this report is frozen.
- No IR chip is implemented; no test, probe, replay, or target run was performed for this report.
- The proposed records are **shared contract candidates**, not private types. They must be frozen
  by the T01 integrator before any implementation.
- The current M1 IR shape (`Constant` + `Return`) is real and tested, but it is **not** a base for
  V1–V3 without the extensions and version bumps identified here.

## 2. Baseline: What `/39` Actually Freezes

### 2.1 Frozen IR-Related Shapes

| Artifact | Frozen shape / rule | Evidence |
|---|---|---|
| Contract ID | `t01-c01-c06/39`, hash `07f4eade…f2fa24` | `compiler/src/contract.rs:38-44`; `compiler/contracts/CONTRACT_VERSION` |
| IDs | `FunctionId`, `BlockId`, `ValueId`, `InstructionId` | `compiler/src/ids.rs:70-74` |
| `FunctionRecord` | `symbol`, `signature`, `entry`, `linkage` | `compiler/src/bus.rs:449-460` |
| `BlockRecord` | `function`, `ordinal` | `compiler/src/bus.rs:462-470` |
| `ValueRecord` | `ty` | `compiler/src/bus.rs:472-478` |
| `InstructionRecord` | `op`, `block`, `operands`, `immediate`, `result` | `compiler/src/bus.rs:480-494` |
| `IrOp` | exactly `Constant`, `Return` | `compiler/src/bus.rs:440-447`; `compiler/src/contract.rs:434-435` |
| `ValueCategory` | `Lvalue`, `NonLvalue`, `FunctionDesignator`, `Void` | `compiler/src/bus.rs:422-433` |
| `EffectMask` | `u32`; M1 allows only `0`; bits reserved/unassigned | `compiler/src/bus.rs:435-438`; `compiler/src/contract.rs:110` |
| `SemRecord` | `node`, `ty`, `category`, `effects`; no `conversions` | `compiler/src/bus.rs:496-513` |
| IR task | `IR_FUNCTION` local 16, whole-function M1 lowering | `compiler/src/task.rs:278-282` |
| IR store | `ir.functions`, `ir.blocks`, `ir.values`, `ir.instructions` | `compiler/src/manifest.rs:243-261` |
| IR allowlist | one chip (`IR_FUNCTION_CHIP = ChipId(16)`) writes the four IR fields for `IR_FUNCTION` | `compiler/src/manifest.rs:877-899`, `:1026` |
| IR stage | `IR_FUNCTION → stage 5` | `compiler/src/manifest.rs:1467` |
| IR rules | `ir.constant-no-refold`, `ir.return-single-terminator`, `ir.function-single-entry`, `commit.ir-materialized` | `compiler/src/contract.rs:165-168` |
| Versioning | per-store `StoreVersions` counters; `StorePatch.version` guard; `ResultRecord.version` | `compiler/src/commit.rs:47-72`, `:1898-1905`; `compiler/src/task.rs:1741-1754` |
| Tombstones | arenas never reuse IDs; `remove` leaves a tombstone | `compiler/src/arena.rs:7-9`, `:52-57`, `:192-205` |
| Snapshot | IR bodies, store versions, tombstone gaps | `compiler/src/snapshot.rs:729-799`, `:854-857` |
| M1 tests | `c15_ir` (7 tests) | `compiler/tests/c15_ir.rs` |

### 2.2 What `/39` Does **Not** Provide

The following are absent, M1-scoped, or only candidate text; none may be treated as frozen:

1. **CFG:** only `Return` exists as a terminator. There is no `Branch`, `CondBranch`, `Switch`,
   `Unreachable`, successor edge list, edge-arity rule, reachability rule, or CFG verifier kind.
2. **Memory:** no load/store/alloca/copy op, no object/address model, no memory access descriptor,
   no alignment/layout binding, no volatile/atomic access shape.
3. **Effects:** `EffectMask` exists but has no assigned bit classes. M1 rejects any nonzero mask
   (`sem.effect-mask-zero-only`), so V1 memory and V3 calls cannot be represented under `/39`.
4. **Calls/returns:** `Return` exists only for M1 `main`; no `Call`, no parameter values, no call
   signature/argument shape, no variadic/indirect call shape.
5. **Dynamic stack:** no `Alloca`, stack save/restore, VLA lifetime, or cleanup representation.
6. **Non-SSA join:** `ValueRecord` has no producer link and the M1 rule assumes a unique producing
   instruction. T09 requires non-SSA join slots; `/39` does not define how they are represented.
7. **Versions:** `StoreVersions` is per logical store, not per function. There is no per-function
   revision, memory epoch, or analysis version key.
8. **Mutation apply:** `StorePatch` with `Replace`/`Tombstone` is shape-validated and logged
   (`compiler/src/commit.rs:1700-1718`, `:1848-1923`), but the commit path does **not** mutate the
   target arena. `CommittedPatch.value` is only a `RecordRef`; it carries no replacement body.
9. **Verification:** no VF07/VF08/VF09 task kinds and no durable verification-fact carrier. VF
   chips currently complete `Ack`/`Fail`; VF12 covers only the M1 `Constant`/`Return` shape.
10. **Types/layout:** `TypeKind` is closed to `Void`, `Bool`, `Char`, `Int`, `Function`
    (`compiler/src/bus.rs:297-323`). There is no pointer, array, aggregate, qualified, or
    `StackMark` type; `LayoutId` is a `ReservedArena` (`compiler/src/bus.rs:645-648`).

### 2.3 Current Implementation Facts (Not Frozen Promises)

| Fact | Evidence | Consequence |
|---|---|---|
| M1 lowering creates one Function, one Block, one Value, two Instructions | `compiler/src/chips/ir_lower/ir_function.rs:321-384` | Correct for M1; not a V1–V3 lowering framework |
| M1 lowering discovers the symbol/type/const by scanning whole arenas | `compiler/src/chips/ir_lower/ir_function.rs:122-241` | Duplicate symbols/types/consts fail ambiguously; does not scale to multiple functions |
| M1 lowering reads whole store fields, not a narrow committed-ref payload | `compiler/src/chips/ir_lower/ir_function.rs:267-289` | `IR_FUNCTION` payload must become explicit committed refs for V1+ |
| VF12 scans all blocks/instructions/types | `compiler/src/chips/verify/vf_interpret.rs:90-120` | Correct for M1; V1+ needs a function-scoped view/index |
| `StorePatch` apply is log-only | `compiler/src/commit.rs:1700-1718` | T10 OP20 cannot rewrite IR today |
| No `opt` or `machine` arena exists | `compiler/src/bus.rs:609-671`; `StoreId::Opt`/`Machine` exist at `compiler/src/task.rs:2097-2098` | T10 analysis storage has no materialization path |
| `ConstRecord` is `value` + `negative`, no `ty` | `compiler/src/bus.rs:594-602` | T09's `ConstRecord.ty == ValueRecord.ty` check cannot be enforced as written |
| `SemRecord` has no `conversions` | `compiler/src/bus.rs:496-513` | V1 non-identity conversions have no committed plan carrier |
| `TypeKind` has no pointer/array | `compiler/src/bus.rs:297-323` | V1 address values and V3 pointers/arrays cannot be typed |

## 3. V1–V3 Requirements

### 3.1 What Each Train Must Express

| Train | Required language surface | Required IR concepts | `/39` gap | Freeze dependency |
|---|---|---|---|---|
| V1 | Integer expressions, parentheses/precedence, local variables, declaration/initialization, assignment, conversions | Constants, integer arithmetic, address-of-object, load, store, conversion ops, undefined value, effects | only Constant/Return; no memory/effects/pointers | F-IR-01..03; T06 pointer type (TY15); T08 layout and `ConstRecord.ty`; T07 effect bits |
| V2 | `if`/`else`, loops, `break`/`continue`, short-circuit, CFG | Branch, conditional branch, switch, unreachable, successor edges, non-SSA join slots, CFG verifier | no CFG | F-IR-01..03; V1; VF07 |
| V3 | Function parameters, calls, returns, globals, pointers, arrays | Parameter values, ABI-neutral call/return, object addresses, pointer arithmetic, aggregate copy, VF08/VF09 | no params/calls/pointers/arrays | F-IR-01..04; T06 TY15/TY16/TY17; T08 layout/init; T11 ABI (execution Part B) |

### 3.2 Required Semantic Guarantees

- V1 must lower a well-typed `int` local read/write without inventing a target width or host
  `sizeof`; the IR carries `TypeId` and optional `LayoutId`.
- V2 must make short-circuit and loop side effects explicit in the CFG; no “constant condition”
  shortcut may skip an effectful RHS or duplicate an effect.
- V3 must keep call/return ABI-neutral; argument locations, sret, register saves, and variadic
  save areas are T11 work, never IR fields.
- Every train must be reversible/verifiable from committed records: no semantic state in chip
  locals, no cross-chip calls, no hidden lookup tables.
- Unsupported features must fail as typed `Unsupported` diagnostics, never as a no-op success.

## 4. Minimal Shared Schema (Proposal)

### 4.1 Design Principle

**Extend the four existing IR records; do not add new record families or ID families for V1–V3.**
Terminator, Memory, Effect, Call, Object, and Switch are **inline sub-records or fields of
`InstructionRecord`/`FunctionRecord`**, not new arenas. This is the smallest shared shape that
still supports a real CFG, memory model, and later optimization.

Why this is the minimum:

- A separate `TerminatorId` would duplicate instruction identity and require a new tag, arena,
  snapshot encoder, and hash entry for no semantic gain.
- A separate `MemoryId`/`EffectId` would be an indirection around a small inline descriptor and
  would create a second owner for the same instruction.
- A separate `CallId` would duplicate the instruction/call-site identity.
- A `ValueDef` arena would duplicate the producer relation already derivable from
  `Instruction.result`; V1–V3 use the non-SSA memory form, so single-definition values suffice.

### 4.2 Status Legend

| Status | Meaning |
|---|---|
| **FROZEN (`/39`)** | Existing field/type/rule; must not be silently changed |
| **PROPOSED-EXTEND** | Add fields to an existing frozen record; requires a version bump |
| **PROPOSED-NEW** | New enum/sub-record/op/task kind; requires a freeze |
| **OPEN** | Not decided here; owner/recommendation given |
| **DEFERRED** | Shape named for forward compatibility; not implemented in V1–V3 |

### 4.3 `FunctionRecord`

| Field | Type | Status | Rule |
|---|---|---|---|
| `symbol` | `SymbolId` | **FROZEN** | Live symbol; `SymbolKind::Function`; `SymbolRecord.ty` matches `signature` |
| `signature` | `TypeId` | **FROZEN** | Live `TypeKind::Function`; gives result/params/prototype/variadic |
| `entry` | `BlockId` | **FROZEN** | Live block owned by this function; entry ordinal must be `0` |
| `linkage` | `Linkage` | **FROZEN** | Matches the symbol’s linkage |
| `form` | `IrForm` | **PROPOSED-EXTEND** | `NonSsa` default for V1–V3; `Ssa` only after OP15. VF09 checks SSA only when `Ssa` |

Proposed enum:

```rust
// PROPOSED, not frozen.
pub enum IrForm { NonSsa, Ssa }
```

No `params` field is added: parameter values are explicit `Param` instructions in the entry block
(§4.8). This preserves the `/39` rule that a value is produced by an instruction and avoids an
extra producer exception.

### 4.4 `BlockRecord`

| Field | Type | Status | Rule |
|---|---|---|---|
| `function` | `FunctionId` | **FROZEN** | Live function; block belongs to exactly one function |
| `ordinal` | `u32` | **FROZEN** | Unique per function; assigned in creation order; never reused after tombstone |

The block’s instruction list is **derived** by ascending `InstructionId` where
`InstructionRecord.block == this block` (frozen `/39` convention). There is no stored instruction
list; adding one would create a second source of truth.

**Entry ordinal:** the entry block must have `ordinal == 0`. Other blocks have strictly increasing
ordinals in creation order; tombstones leave gaps. Deterministic block order is
`(ordinal, BlockId)`.

### 4.5 `ValueRecord`

| Field | Type | Status | Rule |
|---|---|---|---|
| `ty` | `TypeId` | **FROZEN** | Live non-`Void` type; no `Void` values |

A value is defined by exactly one of:

1. a live `InstructionRecord` whose `result == Some(value)`; or
2. a `Param` instruction in the entry block (which is also an instruction result).

This preserves single-definition values and the `/39` producer convention. Non-SSA joins are
represented by explicit memory slots (`Temporary` objects + `Load`/`Store`), not by multiple
definitions of one `ValueId`. SSA form is optional later (OP14/OP15), not the V1–V3 base.
`M1_TARGET_ACCEPTANCE.md:188` IR-9 already makes non-SSA legal and restricts SSA checks; this
report preserves that rule.

### 4.6 `InstructionRecord` (PROPOSED v2)

```rust
// PROPOSED, not frozen. Fields above `// --- /39 ---` preserve the frozen record.
pub struct InstructionRecord {
    // --- /39 ---
    pub op: IrOp,
    pub block: BlockId,
    pub operands: Vec<ValueId>,
    pub immediate: Option<ConstId>,
    pub result: Option<ValueId>,
    // --- PROPOSED v2 ---
    pub successors: Vec<BlockId>,          // terminator edges only; empty otherwise
    pub effect: EffectMask,                // exact effect of this instruction
    pub detail: Option<InstructionDetail>, // memory/call/object/switch payload
}
```

M1 compatibility: for `Constant`/`Return`, `successors` is empty, `effect` is `EffectMask(0)`, and
`detail` is `None`. Existing M1 records can be re-encoded losslessly after adding the three fields.

| Field | Type | Status | Rule |
|---|---|---|---|
| `op` | `IrOp` | **FROZEN type**, **PROPOSED-NEW variants** | Closed op table; each new op is a contract change |
| `block` | `BlockId` | **FROZEN** | Live block in the same function |
| `operands` | `Vec<ValueId>` | **FROZEN** | Ordered; live; non-`Void`; arity per op |
| `immediate` | `Option<ConstId>` | **FROZEN** | Only `Constant`; the constant body must be live |
| `result` | `Option<ValueId>` | **FROZEN** | `Some` iff the op produces a value; `None` for stores/terminators/void calls |
| `successors` | `Vec<BlockId>` | **PROPOSED-EXTEND** | Only terminators; ordered per §4.7; all live/same function |
| `effect` | `EffectMask` | **PROPOSED-EXTEND** | Exact per-op mask from §4.10; verifier-checked |
| `detail` | `Option<InstructionDetail>` | **PROPOSED-NEW** | Closed enum below; absent for pure scalar ops/terminators without a payload |

```rust
// PROPOSED, not frozen.
pub enum InstructionDetail {
    Memory(MemoryAccess),
    Call(CallShape),
    Object(ObjectRef),
    Switch(SwitchShape),
}
```

`InstructionDetail` is a single closed enum, not four optional fields, to keep the record small
and make illegal combinations unrepresentable. An instruction uses at most one detail; a `Load`
uses `Memory`, a `Call` uses `Call`, an `Address`/`Alloca` uses `Object`, a `Switch` uses `Switch`.
`Alloca` also carries stack semantics through its `IrOp` and `ObjectRef.class = Dynamic`.

### 4.7 Terminator Shape (No Separate Record)

A terminator is an `InstructionRecord` whose `IrOp` is in the closed terminator set. `successors`
is the **single edge source**. No separate `TerminatorRecord` is introduced.

| Terminator | `successors` | Operands | Result | Detail | Notes |
|---|---|---|---|---|---|
| `Return` | `[]` | `0` (void) or `1` | `None` | `None` | Frozen op; unique terminator of the block; operand type matches signature result |
| `Unreachable` | `[]` | `0` | `None` | `None` | Path is undefined; no fallthrough |
| `Branch` | `[target]` exactly 1 | `0` | `None` | `None` | Unconditional edge |
| `CondBranch` | `[true, false]` exactly 2 | `1` condition | `None` | `None` | Condition is a scalar/bool value; edges ordered true then false |
| `Switch` | `[default, case0, case1, …]` | `1` condition | `None` | `Switch(SwitchShape)` | `successors[0]` is the default; `successors[i+1]` is case `cases[i]` |

Rules:

- A terminator is the **greatest `InstructionId`** belonging to its block; no instruction may be
  appended after it.
- Each live block has exactly one terminator when the function is complete. During construction a
  block may be open; the entry block must be terminated before the `FunctionEnd` completion fact.
- A non-terminator must have empty `successors`.
- `successors` order is semantic and must be preserved by lowering, snapshot, and replay.
- Self-edges are legal (loops). Duplicate successors are not allowed for `Branch`/`CondBranch`;
  `Switch` case targets may repeat only if the cases are distinct values and the duplicate is
  explicitly represented in `successors`.
- `SwitchShape.cases` is ordered by the mathematical value of the promoted condition; no
  duplicates. Case ranges/GNU `...` are deferred (T12/T09 extension).

### 4.8 `IrOp` Closed Table for V1–V3

```rust
// PROPOSED, not frozen. `Constant` and `Return` are the only /39 variants.
pub enum IrOp {
    Constant, Return,                         // FROZEN
    Undef,                                    // V1: undefined value, effect 0
    Add, Sub, Mul, Div, Rem,                  // V1 integer arithmetic
    Neg, LogNot, BitNot,                      // V1 unary
    Cmp(CmpPredicate),                        // V1/V2 comparisons
    Convert(ConversionOp),                    // V1 conversions
    Address, Load, Store,                     // V1 memory
    Branch, CondBranch, Switch, Unreachable,  // V2 CFG
    Param, Call,                              // V3 functions
    Alloca, StackSave, StackRestore,          // V3/D dynamic stack
    Atomic,                                   // V3/D atomic load/store/RMW/CAS/fence
    Memcpy, Memmove, Memset,                  // V3 aggregate/init copy
}

pub enum CmpPredicate { Eq, Ne, LtS, LeS, GtS, GeS, LtU, LeU, GtU, GeU }
// FP ordered/unordered predicates are deferred to the T09 FP slice.

pub enum ConversionOp {
    Trunc, SExt, ZExt,        // integer width changes (target width comes from T08/T11)
    Bitcast,                  // same-size representation change
    PtrToInt, IntToPtr,       // pointer/integer conversion
    BoolToInt,                // truth conversion
    IntToFloat, FloatToInt, FloatToFloat, // deferred until FP is in scope
}
// `Identity` is not an IR op: T09 emits no instruction and forwards the same ValueId.
```

| Op | Train | Operands | Immediate | Result | Successors | Detail | Effect class |
|---|---|---|---|---|---|---|---|
| `Constant` | M1 / V1 | 0 | `Some(Const)` | `Some` | `[]` | none | 0 |
| `Undef` | V1 | 0 | none | `Some` | `[]` | none | 0 |
| `Add`/`Sub`/`Mul` | V1 | 2 | none | `Some` | `[]` | none | 0 |
| `Div`/`Rem` | V1 | 2 | none | `Some` | `[]` | none | `TRAP` |
| `Neg`/`LogNot`/`BitNot` | V1 | 1 | none | `Some` | `[]` | none | 0 |
| `Cmp(pred)` | V1/V2 | 2 | none | `Some` | `[]` | none | 0 |
| `Convert(op)` | V1 | 1 | none | `Some` | `[]` | none | 0 |
| `Address` | V1 | 0..N offsets | none | `Some` | `[]` | `Object` | 0 |
| `Load` | V1 | 1 address | none | `Some` | `[]` | `Memory(Load)` | `READ\|TRAP` (+`VOLATILE`/`ATOMIC` if marked) |
| `Store` | V1 | 2 address,value | none | `None` | `[]` | `Memory(Store)` | `WRITE\|TRAP` (+flags) |
| `Branch` | V2 | 0 | none | `None` | 1 | none | 0 |
| `CondBranch` | V2 | 1 | none | `None` | 2 | none | 0 |
| `Switch` | V2 | 1 | none | `None` | cases+1 | `Switch` | 0 |
| `Unreachable` | V2 | 0 | none | `None` | 0 | none | 0 |
| `Param` | V3 | 0 | none | `Some` | `[]` | none | 0 |
| `Call` | V3 | args (callee optional operand 0) | none | `Some`/`None` | `[]` | `Call` | `CALL\|READ\|WRITE` unless known pure |
| `Return` | M1 / V3 | 0/1 | none | `None` | 0 | none | 0 |
| `Alloca` | V3/D | 1 size | none | `Some` | `[]` | `Object(Dynamic)` | `STACK` |
| `StackSave` | V3/D | 0 | none | `Some` | `[]` | none | `STACK` |
| `StackRestore` | V3/D | 1 mark | none | `None` | `[]` | none | `STACK` |
| `Atomic` | V3/D | per `MemoryKind` | none | per kind | `[]` | `Memory(kind)` | per kind (`READ`/`WRITE`/`ATOMIC`/`FENCE`) |
| `Memcpy`/`Memmove` | V3 | 3 dest,src,len | none | `None` | `[]` | `Memory(kind)` | `READ\|WRITE\|TRAP` |
| `Memset` | V3 | 3 dest,value,len | none | `None` | `[]` | `Memory(Memset)` | `WRITE\|TRAP` |

**Op-table freeze rules:**

- The table is closed. An op not listed is `UnsupportedIrOp` at the chip or verifier, never a
  fallback or no-op.
- Adding an op after freeze requires a contract version bump, a snapshot name, an op-table rule,
  and a verifier update.
- `Add` is **not** in `/39`; it is in T09/M1 proposal text (`docs/tasks/T09_IR_LOWER_CHIPS.md:83`;
  `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1858-1866`). It becomes legal only through F-IR-01.
- `Cmp`/`Convert` payloads are part of `IrOp`; no extra `InstructionRecord` field is used for
  predicate/conversion. Snapshot encodes the payload name (e.g. `cmp.lt.s`, `convert.sext`).
- `Atomic` is one IR op; `MemoryAccess.kind` selects `AtomicLoad`/`AtomicStore`/`AtomicRmw`/
  `AtomicCas`/`Fence`. `Fence` therefore uses `IrOp::Atomic` + `MemoryKind::Fence`, not a separate
  op. `Memcpy`/`Memmove`/`Memset` are their own ops and their `MemoryKind` must match the op.

### 4.9 `MemoryAccess`

```rust
// PROPOSED, not frozen.
pub struct MemoryAccess {
    pub kind: MemoryKind,
    pub order: MemoryOrder,
    pub failure_order: Option<MemoryOrder>, // AtomicCas only; must be None otherwise
    pub volatile: bool,
    pub layout: Option<LayoutId>,           // None = type-natural; Some = T08 layout
}

pub enum MemoryKind {
    Load, Store,
    AtomicLoad, AtomicStore, AtomicRmw, AtomicCas, Fence,
    Memcpy, Memmove, Memset,
}

pub enum MemoryOrder { None, Relaxed, Acquire, Release, AcqRel, SeqCst }
```

| Field | Rule |
|---|---|
| `kind` | Must match the `IrOp`; `Load`/`Store` for non-atomic; atomic kinds only with atomic types/flags |
| `order` | `None` for non-atomic; legal C11 order for atomic; `Fence` uses a real order. `memory_order_consume` is deferred to T12. |
| `failure_order` | `AtomicCas` only; must not be `Release`/`AcqRel`; otherwise `None` |
| `volatile` | Exact C `volatile` access; no merging/deletion/reordering |
| `layout` | `Some(LayoutId)` when T08 has a committed layout (packed/bitfield/over-aligned); `None` means the type-natural layout supplied by T11; no inline byte width |

**Operand conventions:**

- `Load`: operand 0 = address value; result = loaded value.
- `Store`: operand 0 = address value; operand 1 = value; no result.
- `AtomicRmw`: operand 0 = address; operand 1 = operand value; result = old value.
- `AtomicCas`: operand 0 = address; operand 1 = expected; operand 2 = desired; result = old/observed.
- `Fence`: no operands; result none.
- `Memcpy`/`Memmove`: operands dest, src, len; `Memset`: dest, value, len; result none.
- `Alloca`: operand 0 = size value; result = pointer/mark-like value; `detail = Object(Dynamic)`.
- `StackSave`: no operands; result = opaque mark; `StackRestore`: operand 0 = mark.

No target alignment/width is stored in `MemoryAccess`; `LayoutId` is symbolic and T08-owned.
The concrete byte size/alignment is T11 lowering input.

### 4.10 `EffectMask` Bit Taxonomy (PROPOSED)

`EffectMask` is a **may-effect** bitmask, not a must-effect or mode enum. A set bit means the
instruction may perform the effect; a clear bit means it definitely does not. This is the
conservative direction required for optimization legality.

| Bit | Name | Meaning | Typical setters |
|---:|---|---|---|
| 0 | `READ_MEMORY` | May read ordinary memory | `Load`, `Memcpy`, `Memmove`, unknown call |
| 1 | `WRITE_MEMORY` | May write ordinary memory | `Store`, `Memcpy`, `Memmove`, `Memset`, unknown call |
| 2 | `VOLATILE` | May access a volatile object | volatile load/store/RMW |
| 3 | `ATOMIC` | May perform an atomic access | atomic load/store/RMW/CAS |
| 4 | `FENCE` | May act as a memory fence/barrier | `Fence`, atomic with ordering |
| 5 | `CALL` | May call an unknown/external function | ordinary `Call` |
| 6 | `TRAP` | May fault/trap on a defined path (e.g. div-by-zero, invalid access) | `Div`, `Rem`, `Load`, `Store`, copy |
| 7 | `STACK` | May change dynamic stack state | `Alloca`, `StackSave`, `StackRestore` |
| 8 | `NONLOCAL` | May transfer control nonlocally | `setjmp`/`longjmp`/`asm goto` (T12) |
| 9–31 | reserved | Must be zero in a committed mask until assigned by a future freeze | — |

Rules:

- `EFFECT_NONE = EffectMask(0)` is pure/observably inert.
- `EFFECT_KNOWN_MASK = 0x0000_01FF` is bits 0–8.
- `EFFECT_UNKNOWN = EffectMask(EFFECT_KNOWN_MASK)` means “may perform any currently defined
  effect”; consumers must treat it conservatively. If a future freeze assigns a bit 9+, it must
  update `EFFECT_UNKNOWN` and bump the contract version.
- `union(a, b) = a | b`; `is_pure(mask) = mask == 0`; `may_read = mask & READ_MEMORY != 0`;
  `may_write = mask & WRITE_MEMORY != 0`. `EFFECT_UNKNOWN` has `READ_MEMORY` and `WRITE_MEMORY`
  set, so it reads/writes conservatively.
- A committed mask with any reserved bit 9–31 set is `EffectMaskUnsupported`. This replaces the
  `/39` zero-only rule for V1+.
- `SemRecord.effects` uses the **same bit definitions** at the C semantic level. A nonzero semantic
  mask is legal for V1+ under the new rule; the M1 `EffectMask(0)` rule is M1-scoped.
- An instruction’s mask must be exactly the op-table mask plus its detail flags; it is not allowed
  to hide effects conservatively in an otherwise pure op (that would block legal optimization) or
  to omit a real effect (unsound). The verifier checks the exact expected mask.
- Unknown/opaque operations (inline asm, unknown builtins) use `EFFECT_UNKNOWN`; optimization must
  not assume purity.

**Effect class mapping used by T10 drafts:**

| T10 draft class | EffectMask interpretation |
|---|---|
| `Pure` | `mask == 0` |
| `reads-memory` | `READ_MEMORY` set, `WRITE_MEMORY` clear, no other effect |
| `writes-memory` | `WRITE_MEMORY` set, `READ_MEMORY` clear, no other effect |
| `unknown-effect` | `EFFECT_UNKNOWN` or any bit outside the pure/read/write subset |

This resolves the “effect taxonomy location” open question in `OP_FREEZE_02.md` §6 item 3: the
taxonomy lives on `EffectMask`/`InstructionRecord`, not in per-analysis private enums.

### 4.11 `CallShape` and `Return`

```rust
// PROPOSED, not frozen.
pub struct CallShape {
    pub callee_symbol: Option<SymbolId>, // Some = direct call; None = indirect
    pub signature: TypeId,               // live TypeKind::Function
}
```

| Field | Rule |
|---|---|
| `callee_symbol` | `Some` must be a live function symbol; `None` means `operands[0]` is the callee value (function pointer) and args start at operand 1 |
| `signature` | Must match the callee’s function type; gives result type, named parameter types, prototype, variadic |
| `operands` | Direct call: arguments in source order. Indirect call: callee then arguments |
| `result` | `Some` iff the signature result is not `void`; result `ValueRecord.ty` equals the signature result |
| `effect` | `CALL\|READ\|WRITE` for ordinary calls; `EFFECT_UNKNOWN` when the callee is unknown/asm; a frozen pure builtin can narrow only with a rule ID |

- Variadic calls: the signature’s `variadic` flag is authoritative. Default argument promotions
  and argument conversion plans are T07/T08 results; IR arguments are already converted.
- Old-style/K&R calls: represented by the frozen function type’s `prototype` flag; no special IR
  record.
- Return: `Return` is an instruction (terminator) with 0 or 1 operand. The operand type must match
  `FunctionRecord.signature`’s result; `void` functions have no operand. Aggregate returns stay a
  single aggregate value at IR level; T11 decides registers/sret.
- `_Noreturn`/nonlocal-return: deferred; a call that does not return must be modeled with
  `NONLOCAL`/`Unreachable` effects, not by deleting the fallthrough edge silently.

### 4.12 `ObjectRef`, Storage Classes, and Addresses

```rust
// PROPOSED, not frozen.
pub struct ObjectRef {
    pub symbol: Option<SymbolId>,  // Some for named objects/functions; None for temp/string/dynamic
    pub class: StorageClass,
    pub layout: Option<LayoutId>,  // Some for Alloca/aggregate objects; None means symbol type
    pub literal: Option<LiteralId>, // StringLiteral only; T04/T08-owned bytes
}

pub enum StorageClass {
    Function,      // function designator/address
    Parameter,     // named parameter object
    Local,         // automatic block object
    StaticLocal,   // static block object
    Global,        // file-scope static-duration object
    ThreadLocal,   // thread-local object
    StringLiteral, // read-only string object
    Temporary,     // compiler-created non-SSA join/slot object
    Dynamic,       // VLA/alloca/dynamic-stack object
}
```

`StorageClass` is a **derived classification**, not a new `SymbolRecord` field. The T06/T08 owner
classifies from existing committed fields:

| Class | Derivation from `SymbolRecord` (`compiler/src/bus.rs:360-377`) | `symbol` |
|---|---|---|
| `Function` | `kind == Function` | required |
| `Parameter` | `kind == Object`, `storage == Automatic`, `decl` is in the function parameter list | required |
| `Local` | `kind == Object`, `storage == Automatic`, block scope, not a parameter | required |
| `StaticLocal` | `kind == Object`, `storage == Static`, block scope | required |
| `Global` | `kind == Object`, `storage == Static`, file scope | required |
| `ThreadLocal` | `kind == Object`, `storage == Thread` | required |
| `StringLiteral` | no symbol; `literal: Some(LiteralId)`; read-only bytes owned by T04/T08 | `None` |
| `Temporary` | compiler-created non-SSA slot; identity = the `Address` result `ValueId` | `None` |
| `Dynamic` | `Alloca`/VLA object; identity = the `Alloca` result `ValueId` | `None` |

`ObjectRef.literal` is `Some` only for `StringLiteral` (and `symbol` is `None`). Anonymous
`Temporary`/`Dynamic` objects have no arena ID of their own; the address/`Alloca` result value is
their handle. `ObjectRef` is an instruction detail, not a new record family.

- `Address` instruction: computes an address of `ObjectRef` plus optional offset operands; result
  type is a pointer to the object type. Address computation is not a memory read; effect is 0.
- `Alloca`: `detail = Object(Dynamic)`, operand 0 = size value, result pointer/mark-like value,
  effect `STACK`; `layout` may be `Some(LayoutId)`.
- Globals/statics/TLS/string literals are addressed through `ObjectRef`; their initializers are
  T08 `InitPlan` data, not IR stores unless runtime initialization is required.
- No target address space, byte width, or relocation is stored in the IR. T11 maps the object to
  the target address space/relocation.

### 4.13 Value-Category Mapping

`ValueCategory` stays a **semantic** property on `SemRecord`; `ValueRecord` does not carry a
duplicate category.

| `SemRecord.category` | IR representation | Rules |
|---|---|---|
| `Lvalue` | `Address` value (pointer-like), no `Load` until an rvalue is needed | Address computed once; `&` yields it directly; assignment stores through it |
| `NonLvalue` | Ordinary value-producing instruction result | Every use reads the same `ValueId`; no implicit reload |
| `FunctionDesignator` | `Address` with `StorageClass::Function` | Decays to a function pointer value for calls; direct call may use `callee_symbol` |
| `Void` | Instruction with `result = None` | Never used as an operand; expression statement/void call |

- A `Void` value cannot be an operand, parameter, return operand (for non-void), or `ValueRecord.ty`.
- A bitfield lvalue is **not** represented by an `Address`; IR17 lowers it to explicit
  extract/insert operations with T08 layout facts (deferred beyond V1–V3).
- The lowering chip is responsible for inserting `Load` only when the C context requires an
  rvalue; the verifier checks that a `Load` result is never used as an lvalue address.
- `SemRecord.effects` and `InstructionRecord.effect` use the same mask; a lowering chip must not
  drop a semantic effect. The verifier checks the IR instruction’s exact op-table mask; the
  sem→IR effect superset check is a T09/VF06/VF08 interaction test, not a separate record.

### 4.14 `SwitchShape`

```rust
// PROPOSED, not frozen.
pub struct SwitchShape { pub cases: Vec<ConstId> } // successors[0] = default; successors[i+1] = cases[i]
```

- `cases` are live integer `ConstRecord`s of the promoted condition type.
- Cases are ordered by mathematical value; duplicates are illegal. `successors.len() ==
  cases.len() + 1`.
- No default case is expressed by a default edge to the fallthrough/merge block; the edge always
  exists.
- GNU case ranges, duplicate case diagnostics, and jump tables are deferred to the T09/T12
  extensions.

### 4.15 IR Versions and Tombstones

#### 4.15.1 Per-Function Version Register (PROPOSED)

The existing per-store counters are too coarse for T10 analyses and rewrites: any IR append in any
function would invalidate every function’s analyses. Add one explicit bus register:

```rust
// PROPOSED, not frozen.
pub struct IrVersionStore {
    pub functions: BTreeMap<FunctionId, IrFunctionVersion>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IrFunctionVersion {
    pub revision: u64,     // monotonic; +1 per successful mutation batch for this function
    pub memory_epoch: u64, // monotonic; +1 when a mutation may change memory effects
}
```

- `BTreeMap` iteration is deterministic and sorted by `FunctionId`; it is not a hash map.
- The register is explicit bus state, snapshot-encoded in sorted order, and updated only by the
  commit-apply path (never by a chip).
- Initial entry: created at `FunctionBegin` with `revision = 0`, `memory_epoch = 0`. The M1
  `IR_FUNCTION` compatibility path must either create the same entry or define an explicit
  implicit-zero rule; the freeze must pick one and test it.
- On a successful batch that appends/mutates function `F`’s IR: `revision += 1` exactly once.
  If the batch changes any memory access, memory effect, call/clobber, volatile/atomic, or stack
  state, `memory_epoch += 1`.
- On batch failure: no revision change, no epoch change, no partial state.
- `StoreVersions[Ir]` still bumps once per batch as the coarse `StorePatch.version` guard.
- Analyses and rewrite patches carry `(function, revision, memory_epoch)`; an exact match is
  required. A mismatch is stale and rejected/recomputed, never used.
- **Analysis invalidation hook:** the hook is the commit-apply version bump itself, not a chip
  callback and not a hidden cache. A consumer reads the current `IrVersionStore` entry and either
  matches the fact’s version or rejects/recomputes it. OP21 may record an invalidation/tombstone
  fact in the optimization store, but it performs no IR mutation and is not called by commit.
  Unknown/opaque effects bump `memory_epoch` conservatively.

Alternatives rejected in §12.2: a new revision record family, mutable `FunctionRecord.revision`,
and per-analysis version maps at V1–V3.

#### 4.15.2 Tombstones

- IDs are never reused. Deleting an IR record calls the checked arena `remove`, leaving a
  tombstone (`compiler/src/arena.rs:192-205`).
- Tombstones are visible in snapshots as allocated-count/live-record gaps. The encoder must preserve
  the allocated count and omit the tombstoned body.
- A live record must not reference a tombstone: no operand, result, block, successor, const,
  symbol, type, or function link may point at a deleted ID.
- Deleting a block is legal only if no live instruction or successor edge targets it, and it is not
  the entry block.
- Deleting an instruction is legal only if its result is not live and its removal is legal under
  T10 OP09’s effect rules; terminators require an explicit replacement CFG.
- Deleting a value is legal only after its producer and all uses are deleted.
- Every deletion bumps the function revision; memory-effect deletions bump `memory_epoch`.
- `StorePatch::Tombstone` is currently log-only; the new mutation apply path must actually remove
  the record or the tombstone contract is not satisfied.

#### 4.15.3 Mutation Apply Envelope (OPEN, recommended)

The current `StorePatch` cannot express a typed replacement body: `value: Option<RecordRef>` names
a record, not a new record body. The recommended minimal extension is a new proposal:

```rust
// PROPOSED, not frozen.
pub enum RecordMutation {
    Replace { target: RecordRef, body: G1DraftBody },
    Delete { target: RecordRef },
}
// Proposal::MutateRecords { task: TaskId, batch: Vec<RecordMutation> }
```

- New wire tag after the frozen `0..=7`; `is_transition() == false`.
- Exactly one transition per dispatched task still holds; `MutateRecords` may accompany a
  `Complete`/`Progress`/`AwaitChildren` transition.
- Commit validates, in deterministic batch order: target family/liveness, body family, manifest
  write, `(store, field)` allowlist, store-version guard, predicted refs, and all-or-nothing
  application. Apply uses checked `TypedArena::replace`/`remove`.
- On success: bump `StoreVersions[Ir]` once, bump the per-function revision once, compute
  `memory_epoch` from the before/after effects, append the patch log, then latch.
- On failure: no mutation, no version bump, typed `CommitError`; H6 recovery handles dispatched
  tasks.

`StorePatch` remains for field-level patches to non-IR stores. It must not be overloaded with a
body field unless the integrator explicitly chooses that alternative; the body-less `Replace` is
the confirmed defect to fix. To avoid two mutation paths, `StorePatch::Replace` and
`StorePatch::Tombstone` must be rejected or explicitly restricted to stores without typed record
bodies; they must not coexist with `MutateRecords` as a second record-mutation mechanism.

### 4.16 Verification Facts

#### 4.16.1 V1–V3 Carrier (Minimal)

For V1–V3, a verification fact is the **committed terminal task result** of the verifier task:

- pass: `TaskState::Completed(ResultId)` with `ResultValue::Ack`;
- fail: `TaskState::Failed(DiagnosticId)` with a structured `DiagnosticDraft` carrying the stable
  rule ID;
- unsupported: `Fail` with `DiagGroup::Unsupported`, never a pass;
- not applicable (e.g. VF09 on non-SSA): **OPEN**; see §12. The current `ResultValue` set has no
  `NotApplicable` variant, and VF09 must not report a legal non-SSA function as a violation.

No new verification store is required for V1–V3. The replay/evidence carrier is the existing
snapshot/trace plus the task result/diagnostic.

#### 4.16.2 VF07/VF08/VF09 Predicate Sets

| Checker | Proved facts (IDs in §6) | Input | Output |
|---|---|---|---|
| VF07 `CfgInvariantChip` | CFG-1..CFG-9, CFG-11 | `[FunctionId]` + committed IR | `Ack` / typed `Fail` |
| VF08 `IrInvariantChip` | VAL-1..VAL-10, EFF-1..EFF-4, MEM-1..MEM-4 | `[FunctionId]` + committed IR | `Ack` / typed `Fail` |
| VF09 `SsaInvariantChip` | SSA-1..SSA-4, only if `form == Ssa` | `[FunctionId]` + committed IR | `Ack` / `NotApplicable` (OPEN) / typed `Fail` |
| VF12 `IrInterpretChip` | covered subset small-step | existing `[FunctionId]` | modeled `Const` / typed `Fail` / `Unsupported` |
| VF13 `ReplayCompareChip` | canonical semantic projection + scheduler snapshot | two traces | first divergence / `Ack` |

#### 4.16.3 Later Optimization Facts (Deferred)

T10 analyses need durable facts keyed by `(kind, function, revision, memory_epoch)`. This report
does **not** freeze an analysis store or record family. The smallest later shape is:

```rust
// DEFERRED; T10/T01 co-freeze.
pub struct AnalysisFactRecord {
    pub kind: AnalysisKind,       // UseDef, Dominators, Alias, Loop, ...
    pub function: FunctionId,
    pub revision: u64,
    pub memory_epoch: u64,
    pub payload: Vec<RecordRef>,  // committed facts, stable order
}
```

Open: whether facts live in `opt.analyses` (requires a typed `Arenas.opt` and schema field), in a
new `verify`/`analysis` store, or are recomputed per pass with no persistence. The sibling
[A17_VERIFICATION_REQUIREMENTS.md](A17_VERIFICATION_REQUIREMENTS.md) proposes a
`VerificationRecord` family/store; if the integrator accepts it, use that carrier for VF facts and
do not create a second verification store. V1–V3 do not depend on a durable store.

### 4.17 Error Taxonomy

#### 4.17.1 Carriers

| Carrier | Scope | Semantics |
|---|---|---|
| Dispatcher error | pre-dispatch | No state mutation; affected tasks stay `Ready`; structured tick diagnostic |
| `CommitError` | pre-apply, whole batch | Reject whole batch; H6 recovery; no partial state |
| `DiagnosticDraft`/`Fail` | one task | Task-scoped failure; other tasks continue; typed `DiagGroup`/`DiagnosticCode` |
| T01 phase-2b hook | `FunctionEnd` completion fact | Entry-termination check; rejection semantics vs H6 remains **OPEN (OB-26)** |
| Verifier `Fail`/`Unsupported` | committed IR fact | Post-commit detection; never a pass; does not roll back committed state |
| `Unsupported` | any layer | Explicit gap; never a no-op or fabricated success |

#### 4.17.2 Rule ID Inventory (PROPOSED names; no numeric tags)

| Rule ID | Trigger | Recommended carrier |
|---|---|---|
| `ir.function-single-entry` | Function has zero/multiple entries or entry not ordinal 0 | Commit / VF07 |
| `ir.block-owner` | Block’s function is missing/mismatched | Commit |
| `ir.block-ordinal-unique` | Duplicate `(function, ordinal)` | Commit / VF07 |
| `ir.cfg.reachable` | Live block not reachable from entry | VF07 |
| `ir.cfg.single-terminator` | Block has zero/multiple terminators at completion | Commit / VF07 |
| `ir.cfg.terminator-last` | Instruction after terminator, or terminator not greatest ID | Commit / VF07 |
| `ir.cfg.edge-arity` | `Branch`/`CondBranch`/`Switch` successor count wrong | Chip / VF07 |
| `ir.cfg.successor-live` | Successor is missing/tombstoned/foreign function | Commit / VF07 |
| `ir.cfg.entry-terminated` | Entry block unterminated at `FunctionEnd` | T01 phase-2b hook (`ir.terminator-missing` candidate) |
| `ir.op.arity` | Operand count does not match op table | Chip / VF08 |
| `ir.op.immediate` | `Constant` missing/extra immediate | Chip / VF08 |
| `ir.op.immediate-type` | `ConstRecord` type does not match result `ValueRecord.ty` | Chip / VF08 |
| `ir.op.result` | Result presence/absence wrong | Chip / VF08 |
| `ir.op.type` | Operand/result type violates op table | Chip / VF08 |
| `ir.value.producer` | Value has zero/multiple live producers | Commit / VF08 |
| `ir.value.parameter` | `Param` not in entry, wrong order/count/type, or after a non-Param | VF08 |
| `ir.memory.access` | `Load`/`Store` address/value/result type mismatch | Chip / VF08 |
| `ir.memory.effect` | Effect mask does not match access kind/flags | Chip / VF08 |
| `ir.effect.reserved-bit` | Reserved `EffectMask` bit set | Chip / VF08 |
| `ir.call.signature` | Callee/signature/args/result mismatch | Chip / VF08 |
| `ir.return.type` | Return operand type ≠ signature result, or void misuse | Chip / VF08 |
| `ir.void.use` | `Void` value used as operand/result | VF08 |
| `ir.atomic.order` | Illegal order/failure order/CAS shape | Chip / VF08 |
| `ir.stack.balance` | Unbalanced `Alloca`/`StackRestore` on a path | VF08 |
| `ir.tombstone.referenced` | Live record references a tombstone | Commit / VF08 |
| `ir.version.stale` | Analysis/patch version mismatch | Commit / OP20 |
| `ir.rewrite.no-partial` | Mutation batch fails after partial apply attempt | Commit |
| `ir.verify.not-applicable` | VF09 on non-SSA (must not be a violation) | **OPEN** |

**Alias retirement:** the T09 candidate pair `ir.op-immediate-type` /
`ir.op-immediate-type-result-ty` and the pair `ir.terminator-missing` /
`ir.terminator-missing-explicit-commit-marker` are **not** duplicated here. Freeze one spelling
(recommended: `ir.op.immediate-type`, `ir.cfg.entry-terminated` mapped to the stable
`ir.terminator-missing` alias if required) and retire the other. See `docs/tasks/T09_IR_LOWER_CHIPS.md:55`
and `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1907-1917`.

**Carrier split recommendation:** keep local op-shape errors task-scoped (chip `Fail`), cross-record
integrity errors whole-batch (`CommitError`), and post-commit invariants in VF facts. The
`docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:2457-2466` `CommitError` list is one candidate; it should not force
a whole batch to fail for a single malformed local op. The integrator must choose one carrier per
rule and record it in the hashed rule inventory.

## 5. Task/Result Envelopes for V1–V3

### 5.1 Existing Frozen Task

`IR_FUNCTION` (local 16) remains the M1 whole-function lowerer. It is **not** extended into a
multi-function/multi-block lowerer. V1+ uses granular kinds below; the M1 kind stays for
compatibility and may be retired only by an explicit contract change.

### 5.2 Proposed IR Task Kinds (names; numeric local codes are freeze-time)

All are `TaskGroup::IR_LOWER`, `phase = Propagation`, `category = Emulable`,
`backend_class = CpuReference`, stage 5, layer 5.

#### 5.2.1 Payload Encoding Constraint (Important)

The frozen task envelope is `Payload { refs: Vec<RecordRef> }`
(`compiler/src/task.rs:1248-1262`); T01 §4 and `compiler/src/task.rs:1885-1887` require `RecordRef`-only
payloads. Therefore the descriptors in §4 (`MemoryAccess`, `ObjectRef`, `CmpPredicate`,
`ConversionOp`, `UnaryOp`/`BinaryOp`, block `ordinal`) are **not carried inline** in the table
below. They are either:

1. **Derived by the chip** from committed `Node`, `Sem`, `Type`, `Layout`, `Const`, and `Symbol`
   records (recommended for V1–V3); or
2. carried by a **T01/T02-owned typed payload extension** (a per-kind payload enum with owned
   scalar fields), which is a shared contract change, not an IR-private type; or
3. replaced by folding block creation into the parent construct task, so ordinals are derived
   from the projected arena count and the local batch index.

The table lists the **semantic** input refs; it is not legal payload text until the freeze picks
one encoding. The recommended V1 path is derivation: `ir.load` receives a `Load` node plus an
address `ValueId` and derives volatile/atomic/layout from the checked type; `ir.binary` receives
a `BinaryExpression` node and derives the op from its `NodeKind`; `ir.convert` receives a cast
node and reads the committed T07 conversion plan. This preserves the frozen envelope and avoids a
second type system.

| Kind (proposed) | Catalog | Payload (committed `RecordRef`s; descriptors derived) | Result | Reads (beyond active task) | Writes |
|---|---|---|---|---|---|
| `ir.function_begin` | IR01 | `[FunctionDefinition Node, Symbol, Signature Type]` | `Record(Function)` | `parse.nodes`, `symbols.symbols`, `types.records`, `sem.records` | `ir.functions`, `ir.blocks` (entry), `ir.instructions` (params) |
| `ir.block_create` | IR02 | `[Function, Parent Block/Node]`; ordinal derived or typed payload | `Record(Block)` | `ir.functions`, `ir.blocks` | `ir.blocks` |
| `ir.constant` | IR03 | `[Function, Block, Const, Type]` | `Records([Instruction, Value])` | `ir.*`, `constants.records` | `ir.instructions`, `ir.values` |
| `ir.address` | IR04 | `[Function, Block, Object Node/Symbol, Type]` | `Records([Instruction, Value])` | `ir.*`, `symbols.symbols`, `types.records` | `ir.instructions`, `ir.values` |
| `ir.load` | IR05 | `[Function, Block, Load Node, Address Value, Type]` | `Records([Instruction, Value])` | `ir.*`, `layout.*`, `types.records` | `ir.instructions`, `ir.values` |
| `ir.store` | IR06 | `[Function, Block, Store Node, Address Value, Value]` | `Record(Instruction)` | `ir.*`, `layout.*`, `types.records` | `ir.instructions` |
| `ir.convert` | IR07 | `[Function, Block, Cast Node, Value, Type]` | `Records([Instruction, Value])` | `ir.*`, `types.records`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.unary` | IR08 | `[Function, Block, Unary Node, Value]` | `Records([Instruction, Value])` | `ir.*`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.binary` | IR09 | `[Function, Block, Binary Node, Lhs, Rhs]` | `Records([Instruction, Value])` | `ir.*`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.pointer_arith` | IR10 | `[Function, Block, PointerArith Node, Pointer Value, Index Value]` | `Records([Instruction, Value])` | `ir.*`, `layout.*`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.compare` | IR11 | `[Function, Block, Compare Node, Lhs, Rhs]` | `Records([Instruction, Value])` | `ir.*`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.short_circuit` | IR12 | `[Function, Block, Logical Node]` | `Records([Block, Instruction, ...])` | `ir.*`, `sem.records` | `ir.blocks`, `ir.instructions`, `ir.values` |
| `ir.conditional` | IR13 | `[Function, Block, Conditional Node]` | `Records([Block, Instruction, ...])` | `ir.*`, `sem.records` | `ir.blocks`, `ir.instructions`, `ir.values` |
| `ir.assignment` | IR14 | `[Function, Block, Assignment Node, Address Value, Value]` | `Records([Instruction, ...])` | `ir.*`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.rmw` | IR15 | `[Function, Block, Rmw Node, Address Value, Value]` | `Records([Instruction, Value])` | `ir.*`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.member` | IR16 | `[Function, Block, Member Node, Base Value]` | `Records([Instruction, Value])` | `ir.*`, `layout.*`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.bitfield` | IR17 | `[Function, Block, BitField Node, Base Value]` | `Records([Instruction, Value])` | `ir.*`, `layout.*`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.call` | IR18 | `[Function, Block, Call Node, Callee Symbol/Value, Args...]` | `Records([Instruction, Value?])` | `ir.*`, `types.records`, `sem.records` | `ir.instructions`, `ir.values` |
| `ir.return` | IR19 | `[Function, Block, Return Node, Option<Value>]` | `Record(Instruction)` | `ir.*`, `sem.records` | `ir.instructions` |
| `ir.if` | IR20 | `[Function, Block, If Node]` | `Records([Block, Instruction, ...])` | `ir.*`, `sem.records` | `ir.blocks`, `ir.instructions` |
| `ir.loop` | IR21 | `[Function, Block, Loop Node]` | `Records([Block, Instruction, ...])` | `ir.*`, `sem.records` | `ir.blocks`, `ir.instructions` |
| `ir.switch` | IR22 | `[Function, Block, Switch Node]` | `Records([Block, Instruction, ...])` | `ir.*`, `sem.records` | `ir.blocks`, `ir.instructions` |
| `ir.jump` | IR23 | `[Function, Block, Jump Node/Label]` | `Record(Instruction)` | `ir.*`, `sem.records` | `ir.instructions` |
| `ir.init` | IR24 | `[Function, Block, InitPlan]` | `Records([Instruction, ...])` | `ir.*`, `init.*`, `layout.*` | `ir.instructions`, `ir.values` |
| `ir.copy` | IR25 | `[Function, Block, Copy Node, Dest, Src, Layout]` | `Records([Instruction, ...])` | `ir.*`, `layout.*` | `ir.instructions` |
| `ir.dynamic_stack` | IR26 | `[Function, Block, VlaPlan]` | `Records([Instruction, Value?])` | `ir.*`, `layout.*` | `ir.instructions`, `ir.values` |
| `ir.atomic` | IR27 | `[Function, Block, AtomicPlan]` | `Records([Instruction, Value?])` | `ir.*`, `types.records` | `ir.instructions`, `ir.values` |
| `ir.function_end` | IR28 | `[Function]` | `Ack` | `ir.*` | none |
| `ir.extension` | IR29 | `[Function, Block, ExtensionPlan]` | `Records(...)` | `ir.*`, `ext.*` | `ir.instructions`, `ir.values` |

`Type` above means `RecordRef::Type`; `Value` means `RecordRef::Value`; `Block` means
`RecordRef::Block`; `Symbol` means `RecordRef::Symbol`; `Node` means `RecordRef::Node`.
`InitPlan`, `VlaPlan`, `AtomicPlan`, `ExtensionPlan`, `CheckedLogical`, etc. are T07/T08/T09/T12
plan references, not new shared bus records; their owners must freeze the carriers. The IR record
stores only the resulting `IrOp`/`detail`.

**Train assignment:** V1 = IR01–IR09, IR14, IR15; V2 = IR11–IR13, IR20–IR23; V3 = IR10, IR16,
IR18, IR19, IR24–IR26, IR28; D = IR17, IR27, IR29. This matches §3.1 and the implementation work
orders; IR17/IR27/IR29 are not V1–V3 blockers.

**Payload discipline (V1 requirement):** every task carries the committed refs it needs. A chip must
not rediscover “the one symbol/type/constant” by scanning an arena. The current M1 lowerer’s
singleton scans (`compiler/src/chips/ir_lower/ir_function.rs:122-241`) are acceptable only for M1 and must not be copied into
V1+.

**Result discipline:** no new `ResultValue` variant. Use `Record`, `Records` (production order),
`Ack`, or `Fail`. `ir.function_end` completes `Ack`; the deterministic completion fact is the
committed terminal result of the task whose kind is `FunctionEnd` (T09 selected direction; no
marker family, no new `ResultValue` variant).

### 5.3 Proposed Verification Task Kinds

| Kind (proposed) | Checker | Payload | Result | Stage |
|---|---|---|---|---|
| `verification.cfg_invariant` | VF07 | `[Function]` | `Ack` / `Fail` | 6 |
| `verification.ir_invariant` | VF08 | `[Function]` | `Ack` / `Fail` | 6 |
| `verification.ssa_invariant` | VF09 | `[Function]` | `Ack` / `NotApplicable` (OPEN) / `Fail` | 7 (candidate; after T10 SSA/`Phi`) |
| `verification.ir_interpret` | VF12 | `[Function]` | modeled `Const` / `Fail` / `Unsupported` | 6 (existing) |
| `verification.replay_compare` | VF13 | two traces | `Ack` / first divergence | 6 (deferred) |

`STAGE_ASSIGNMENT` additions: all IR kinds → 5; VF07/VF08 and VF12/VF13 → 6; VF09 → 7
(candidate, after the T10 SSA/`Phi` freeze). Routing layer must equal the stage (existing
`check_stage_layer_agreement`). No VF chip writes a producer store; VF07/VF08 are read-only
(`Ack`/`Fail` only). If the integrator accepts the sibling A17 verification family, VF facts may
write `verification.records` instead of being result-only; do not create both carriers.

### 5.4 Candidate Local-Code Plan (not frozen)

`IR_FUNCTION = 16` stays. Candidate reservations (freeze-time numeric assignment by T01):

| Local | Kind | Local | Kind |
|---:|---|---:|---|
| 17 | `function_begin` | 32 | `member` |
| 18 | `block_create` | 33 | `bitfield` |
| 19 | `constant` | 34 | `call` |
| 20 | `address` | 35 | `return` |
| 21 | `load` | 36 | `if` |
| 22 | `store` | 37 | `loop` |
| 23 | `convert` | 38 | `switch` |
| 24 | `unary` | 39 | `jump` |
| 25 | `binary` | 40 | `init` |
| 26 | `pointer_arith` | 41 | `copy` |
| 27 | `compare` | 42 | `dynamic_stack` |
| 28 | `short_circuit` | 43 | `atomic` |
| 29 | `conditional` | 44 | `function_end` |
| 30 | `assignment` | 45 | `extension` |
| 31 | `rmw` | | |

Do not claim these codes are frozen. The integrator must check the `TaskKindRegistry` head and
reject collisions at freeze, exactly as `/36` and `/39` did for PA/SE collisions (see T01 §7.1).

## 6. Legality Rules

Rules are grouped by enforcement layer. “Commit” means pre-apply `CommitError`; “chip” means
task-scoped `Fail`; “VF” means post-commit verifier fact. The exact carrier split remains a
freeze-time decision (§4.17.2).

### 6.1 CFG Rules

| ID | Rule | Enforcement |
|---|---|---|
| CFG-1 | Exactly one live entry block per function; `entry` belongs to the function; entry ordinal = 0 | Commit + VF07 |
| CFG-2 | Every live block belongs to exactly one live function; `(function, ordinal)` unique; ordinals never reused | Commit + VF07 |
| CFG-3 | When the function is complete, every live block is reachable from the entry via `successors`; unreachable live blocks are illegal | VF07; OP08 must delete/detach in the same successful batch |
| CFG-4 | Every live block has exactly one terminator when the function is complete; the terminator is the greatest `InstructionId` in the block | Commit + VF07 |
| CFG-5 | No instruction may follow a terminator in the same block; no second terminator | Commit + VF07 |
| CFG-6 | Successor arity: `Return`/`Unreachable` 0; `Branch` 1; `CondBranch` 2 (ordered true/false); `Switch` `cases+1` (default first) | Chip + VF07 |
| CFG-7 | Every successor is live and belongs to the same function; no duplicate successors for `Branch`/`CondBranch` | Commit + VF07 |
| CFG-8 | `CondBranch` condition is a live scalar/bool value; `Switch` condition is a live integer value; case values are unique constants of the promoted condition type | Chip + VF08 |
| CFG-9 | Entry block terminated at the `FunctionEnd` completion fact; no instruction after the terminator | T01 phase-2b hook (OB-26 open) |
| CFG-10 | `Return` operand type matches the function result; `void` functions have no operand | VF08 |
| CFG-11 | A `Call` is a non-terminator; a non-returning call requires `NONLOCAL`/`Unreachable` modeling, not a silent edge deletion | VF08 / T12 |
| CFG-12 | `Unreachable` terminates a path with no successors; no optimization may introduce a new path to it | VF07/VF08 |
| CFG-13 | `Switch` default edge always exists (may target the merge/fallthrough block) | VF07 |
| CFG-14 | Block deletion requires no live instruction/edge target and cannot delete the entry | Commit |
| CFG-15 | The CFG is acyclic only where the source is; loops/back-edges are legal and must be preserved | VF07 (reachability, not acyclicity) |

### 6.2 Value and Type Rules

| ID | Rule | Enforcement |
|---|---|---|
| VAL-1 | `ValueRecord.ty` is a live type and is not `Void` | Commit + VF08 |
| VAL-2 | Every value has exactly one live producer instruction (`result == Some(value)`); no duplicate producers | Commit + VF08 |
| VAL-3 | `Param` instructions are only in the entry block, appear before any non-`Param` instruction, and match the signature parameter count/order/types | VF08 |
| VAL-4 | Every operand/result type satisfies the op table; no implicit host-width conversion | Chip + VF08 |
| VAL-5 | `Constant.immediate` is a live `ConstRecord`; its type matches the result `ValueRecord.ty` (requires the `ConstRecord.ty` resolution in §9) | Chip + VF08 |
| VAL-6 | `Convert(op)` polarity is legal: `Trunc` narrows, `SExt`/`ZExt` widen, `Bitcast` is same-size, pointer/int conversions require pointer/integer types; no target width is inferred from the host | Chip + VF08 |
| VAL-7 | `Cmp(pred)` operands have compatible types and the predicate’s signedness is legal; result is the canonical boolean/int type | Chip + VF08 |
| VAL-8 | `Address` result is a pointer to the referenced object type; offset operands are integers | Chip + VF08 |
| VAL-9 | `Load` result type equals the accessed object type; `Store` value type equals the accessed object type after the committed conversion | Chip + VF08 |
| VAL-10 | `Call` callee/signature/arguments/result are compatible; variadic arguments are already promoted; indirect callee is a function pointer | Chip + VF08 |
| VAL-11 | `Alloca` size is an integer value; result is a pointer/mark type; layout is T08-owned | Chip + VF08 |
| VAL-12 | `Memcpy`/`Memmove`/`Memset` operand shapes and lengths are legal; overlap rules are explicit per op | Chip + VF08 |
| VAL-13 | A `Void` value is never an operand, parameter, return operand, or `ValueRecord.ty` | VF08 |
| VAL-14 | A value is never used after its producer/tombstone; no live reference to a tombstone | Commit + VF08 |

### 6.3 Memory and Effect Rules

| ID | Rule | Enforcement |
|---|---|---|
| EFF-1 | `InstructionRecord.effect` equals the op-table base mask plus detail flags; unknown effects must use `EFFECT_UNKNOWN`, not arbitrary extra bits; no real effect omitted | Chip + VF08 |
| EFF-2 | `SemRecord.effects` uses the same bit definitions; a V1+ nonzero mask is legal only with the new bit taxonomy; M1’s zero-only rule is M1-scoped | T07/T01 freeze; VF06/VF08 |
| EFF-3 | Volatile accesses are never merged, deleted, or reordered relative to other volatile/atomic accesses; exact count/order is observable | T10 legality; VF08 |
| EFF-4 | Atomic accesses preserve C11 order; `AtomicCas.failure_order` is legal; no atomic is disguised as non-atomic | Chip + VF08 |
| EFF-5 | A `Call` without a frozen purity rule sets `CALL\|READ\|WRITE` (or `EFFECT_UNKNOWN`); it blocks memory reordering/deletion | VF08 + T10 |
| EFF-6 | `TRAP` marks a possible fault; optimization must not introduce a new trap on a path that did not execute, and must not remove a required trap | T10 legality; VF08 |
| EFF-7 | `STACK` is a barrier for stack/`Alloca` operations; stack operations are not reordered across calls or other stack operations without a proof | T10 legality; VF08 |
| EFF-8 | `NONLOCAL` (setjmp/longjmp/asm goto) makes CFG/alias facts conservative; no optimization assumes a normal return | T12/T10; VF08 |
| EFF-9 | A reserved effect bit set in a committed mask is `EffectMaskUnsupported` | Chip + VF08 |
| MEM-1 | `MemoryAccess.layout` is a live `LayoutId` when present; `None` means natural layout, never a host `sizeof` | VF08 |
| MEM-2 | `Load`/`Store` address operand is a pointer/address value; no integer address without an explicit `IntToPtr` | VF08 |
| MEM-3 | `Alloca`/`StackSave`/`StackRestore` form a balanced LIFO chain on every path; a mark is used at most once and after its save | VF08 |
| MEM-4 | Aggregate copy respects overlap: `Memcpy` undefined on overlap, `Memmove` defined; `Memset` writes the exact byte range | VF08 + T10 |

### 6.4 Version, Rewrite, and Tombstone Rules

| ID | Rule | Enforcement |
|---|---|---|
| VER-1 | Every completed function has an `IrVersionStore` entry; initial revision 0 at `FunctionBegin` | Commit |
| VER-2 | A successful mutation batch bumps the function revision exactly once; no gaps/duplicates | Commit |
| VER-3 | A memory-effect-changing batch bumps `memory_epoch` exactly once; conservative “any memory/stack/call/volatile/atomic change” is sufficient | Commit |
| VER-4 | A failed batch changes no IR, no revision, no epoch, no patch log, no analysis fact | Commit |
| VER-5 | A rewrite/analysis with `(function, revision, memory_epoch)` not equal to current is stale: reject or recompute, never use | Commit / OP20 |
| VER-6 | `StoreVersions[Ir]` bumps once per successful IR batch; `StorePatch.version` must equal the current store version | Commit |
| VER-7 | A successful rewrite applies all replacements/deletions or none; no partial application | Commit |
| VER-8 | A tombstone is permanent; IDs are never reused; snapshots preserve allocated-count gaps | Commit + snapshot |
| VER-9 | Deleting a live value/instruction/block with live references is illegal | Commit + VF08 |
| VER-10 | Deletion of volatile/atomic/call/terminator or an unknown-effect instruction is illegal without an explicit effect-law rule | OP09/OP20 + VF08 |
| VER-11 | Analysis facts and rewrite patches are invalidated by a revision/epoch mismatch; OP21 may only mark/recompute, not mutate IR | OP21 |

### 6.5 SSA Rules (Conditional)

| ID | Rule | Enforcement |
|---|---|---|
| SSA-1 | Only functions with `form == Ssa` are checked; non-SSA functions must not be flagged | VF09 |
| SSA-2 | Every value has exactly one definition and every use is dominated by it | VF09 |
| SSA-3 | Every phi/join edge has exactly one incoming value per predecessor; type-compatible | VF09 |
| SSA-4 | Non-SSA join slots (`Temporary` objects + `Load`/`Store`) remain legal in non-SSA functions and are not SSA violations | VF09 |

**Deferred SSA op:** `Phi` is **not** in the V1–V3 op table. SSA/phi promotion is a T10 E-OPT
extension that must add a `Phi` op (or an equivalent join record) and a contract version. Until
that freeze, `FunctionRecord.form` stays `NonSsa` and VF09 is not applicable/does not run.

## 7. Ownership Matrix

| Component | Owner | Exact paths | May write | Must not write |
|---|---|---|---|---|
| IR records, IDs, op table, effects, version register, tombstone/apply hooks | T01 integrator | `compiler/src/bus.rs`, `compiler/src/ids.rs` (only if a new family is ever approved), `compiler/src/task.rs`, `compiler/src/commit.rs`, `compiler/src/arena.rs`, `compiler/src/snapshot.rs`, `compiler/src/contract.rs`, `compiler/src/manifest.rs`, `compiler/src/records.rs`, `compiler/src/lib.rs`, `compiler/src/prelude.rs`, `compiler/src/chips/mod.rs`, `compiler/Cargo.toml`, `compiler/contracts/CONTRACT_VERSION` | Shared contract/version/hash, registration, routing, snapshot | Language-chip semantics; private IR copies |
| IR lowering (IR01–IR29) | T09 chip owner | `compiler/src/chips/ir_lower/ir_*.rs` (new files) and assigned tests | Own chip files, own narrow projections, proposals | `compiler/src/bus.rs`, `task.rs`, `commit.rs`, `snapshot.rs`, `manifest.rs`, `mod.rs`, Cargo, other groups |
| Optimization (OP01–OP22) | T10 chip owner | `compiler/src/chips/optimize/op_*.rs` (new directory), assigned tests | Own analysis/rewrite chips, proposals | IR records, versions, commit apply, `mod.rs`, Cargo |
| Verification (VF07–VF09, VF12–VF14) | T13 chip owner | `compiler/src/chips/verify/vf_*.rs` (existing + new files), assigned tests | Own verifier chips, read-only facts/results | IR records, task enums, store schemas, `mod.rs` |
| Types/pointers/arrays | T06 owner | `compiler/src/chips/types/*`, `compiler/src/chips/symbols/*` | Type records, compatibility, conversion plans | IR records, IR lowering |
| Layout/init/constants | T08 owner | `compiler/src/chips/constant_layout_init/*` | Layout/init/const records | IR records, IR lowering |
| Target/ABI/machine | T11 owner | `compiler/src/chips/target/*` (future) | Machine records, ABI plans, assembly | IR records, IR semantics |
| Host/toolchain | Host owner | `compiler/src/host/*` (future), `compiler/src/bin/candidate.rs` | File/process/artifact I/O | Any semantic IR decision |
| Dispatch ledger/indexes | Integrator | `docs/tasks/DISPATCH_LEDGER.md`, shared task indexes | Ledger/status | Worker reports, source |

**Rules:**

- One active writer per file; chip owners submit a file list and the integrator generates
  `mod.rs`/registration/manifest aggregation.
- No chip imports another chip’s implementation or creates a private IR type.
- No chip mutates the bus directly; all writes are proposals applied by commit.
- The IR records are a single shared schema; a group may not fork `FunctionRecord` etc.

## 8. Determinism and Replay Impact

### 8.1 Ordering and Allocation

| State | Deterministic rule | Encoding/check |
|---|---|---|
| Function/Block/Value/Instruction IDs | Dense allocation order; no reuse; no address derivation | Ascending ID iteration; snapshot allocated count |
| Blocks | `(ordinal, BlockId)`; entry ordinal 0; reachable from entry | Ordinal uniqueness VF07; no stored list |
| Instructions | Ascending `InstructionId`; grouped by `block`; terminator greatest in block | `c15_ir` terminator test; VF07 |
| Successors | Terminator-defined order; `CondBranch` true/false; `Switch` default first then cases | Snapshot order preserved; VF07 |
| `SwitchShape.cases` | Ascending mathematical value; no duplicates | Snapshot order; VF08 |
| `Param` | Entry block, signature order, before non-Param | VF08 |
| Call arguments | Source/conversion order | Snapshot order; VF08 |
| `EffectMask` | Bitwise union; no iteration order | u32 encode |
| `IrVersionStore` | `BTreeMap` sorted by `FunctionId` | Count + sorted entries |
| Tombstones | Allocated count + live records; gaps visible | Snapshot `allocated()` + iter; no reuse |
| Diagnostics | Commit ordinal order (existing) | Snapshot/trace |
| Store versions | Fixed `StoreId::ALL` order (existing) | `compiler/src/snapshot.rs:854-857` |

### 8.2 Version Commit Order

1. Validate all proposals/mutations (no mutation).
2. Apply record appends/replacements/deletions in deterministic proposal order.
3. Apply task transitions and result/diagnostic appends.
4. Bump `StoreVersions[Ir]` once if any IR field changed.
5. Bump per-function `revision` once; bump `memory_epoch` if required.
6. Latch; append tick report.
7. On any failure before step 5: no semantic state, no version bump; H6 recovery.

This order must be part of the atomic commit contract. It is **not** implemented today.

### 8.3 Replay and Snapshot Impact

- `Snapshot::capture` must encode the new `InstructionRecord` fields (`successors`, `effect`,
  `detail`), `FunctionRecord.form`, and the `IrVersionStore` in a fixed order.
- The static contract hash must include the new field-name lists, op names, effect-bit names, and
  rule IDs (`compiler/src/contract.rs`), not runtime values.
- `compiler/tests/freeze.rs` must recompute the hash and verify the version file.
- `compiler/tests/c15_ir.rs` must be updated for the new record shape while retaining the M1
  two-instruction assertion; `c17_vf12`, `c19_vf01`, and `freeze` must be re-run.
- VF13’s canonical semantic comparison projection must include IR records, tombstone gaps, and
  `IrVersionStore` (explicit semantic version state). The coarse `StoreVersions` are derived
  guards and may be excluded only if the freeze proves they cannot affect compared behavior
  (A17 OQ-8/A23); if a patch guard can observe them, include them. Scheduler-only registers
  (`in_flight`, queues, `report`) stay in the separate scheduler snapshot
  (`docs/tasks/VF13_FREEZE.md:25-53`).
- Identical initial state/config/pins must yield identical IDs, record order, successors, cases,
  revisions, and epochs. No `HashMap` iteration, pointer values, wall clock, or host ABI may enter
  the snapshot.
- `BTreeMap`/`BTreeSet` are allowed because they are ordered; any new map must be sorted before
  encoding.

### 8.4 Replay Failure Modes to Test

| Failure | Expected result |
|---|---|
| Hash-map iteration changes successor/case order | Snapshot/replay mismatch; forbidden by construction |
| Per-function revision bump twice or not at all | VER-2 failure |
| Failed batch leaves a bumped revision | VER-4 failure |
| Tombstone gap collapsed | Snapshot mismatch / VF01/VF13 failure |
| Stale rewrite accepted | VER-5 failure |
| Two producers for one value | VAL-2 / VF08 failure |
| Non-SSA function flagged by VF09 | SSA-1 / `NotApplicable` open question |
| Function block not reachable | CFG-3 / VF07 failure |

## 9. Conflicts with `/39` and Current Drafts

### 9.1 `/39` Conflicts

| ID | Source | Conflict | Required resolution | Owner |
|---|---|---|---|---|
| X1 | `/39` `IrOp` `compiler/src/bus.rs:440-447`; `compiler/src/contract.rs:434-435` | Only `Constant`/`Return`; V1 needs arithmetic/memory, V2 CFG, V3 calls | F-IR-01 op-table freeze; version bump; snapshot names; M1 tests updated | T01 |
| X2 | `/39` `InstructionRecord` `compiler/src/bus.rs:480-494` | No `successors`, `effect`, or `detail`; cannot express CFG/memory/call | F-IR-01 record extension; M1 defaults empty/0/None | T01 |
| X3 | `/39` `EffectMask` zero-only `compiler/src/contract.rs:110` | V1 memory/V3 calls have real effects | F-IR-01 bit taxonomy; replace `sem.effect-mask-zero-only` with a versioned rule | T01 + T07 |
| X4 | `/39` `ConstRecord { value, negative }` `compiler/src/bus.rs:594-602` vs T09/M1 proposal `ConstRecord.ty` `docs/tasks/T09_IR_LOWER_CHIPS.md:57`; `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1561` | `ir.op.immediate-type` cannot compare `ConstRecord.ty` to `ValueRecord.ty` | Add `ty: TypeId` to `ConstRecord` (T08/Gate 1 change) **or** drop the type check and require a producer-carried expected type; recommend adding `ty` | T08/T01 |
| X5 | `/39` `SemRecord` four fields `compiler/src/bus.rs:496-513` vs M1 proposal `conversions` `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1487-1490` | V1 non-identity conversions have no committed plan carrier | T07/T01 freeze the conversion carrier; T09 lowers it; M1 identity stays | T07/T01 |
| X6 | `/39` `TypeKind` `compiler/src/bus.rs:297-323` | No pointer/array; V1 address values/V3 pointers/arrays cannot be typed | T06 TY15/TY16 freeze; IR uses `TypeId` | T06 |
| X7 | `/39` `layouts` reserved `compiler/src/bus.rs:645-648` | V1 load/store width/alignment has no committed layout | T08 `LayoutRecord` freeze; IR uses `Option<LayoutId>` | T08 |
| X8 | `/39` `StorePatch` apply log-only `compiler/src/commit.rs:1700-1718` | T10 OP20 cannot replace/delete IR records | F-IR-02 typed `MutateRecords`/checked replace/remove | T01 |
| X9 | `/39` per-store versions only `compiler/src/commit.rs:47-72`; typed appends do not bump the store version (A23-F05) | T10 analysis/rewrite needs per-function precision and append-safe guards | F-IR-02 `IrVersionStore` + append/mutation bump + stale rule | T01 |
| X10 | `/39` `ValueRecord` unique-producer comment `compiler/src/bus.rs:472-478` vs T09 non-SSA join `docs/tasks/T09_IR_LOWER_CHIPS.md:39` | Non-SSA joins appear to need multi-def values | Base IR uses memory slots (`Temporary` + `Load`/`Store`); values stay single-def; SSA later via `form` | T09/T01 |
| X11 | T09 `FunctionEnd` result typing / OB-26 `docs/tasks/T09_IR_LOWER_CHIPS.md:84-94` | Completion fact carrier and rejection semantics unresolved | Keep task-kind completion fact + T01 phase-2b hook; do not add a marker family or `ResultValue` variant; resolve OB-26 before freeze | T09/T01 |
| X12 | `IR_FUNCTION` whole-function M1 task `compiler/src/task.rs:278-282`; scans `compiler/src/chips/ir_lower/ir_function.rs:122-241` | Cannot scale to multi-function/multi-constant V1 | F-IR-03 granular kinds; payload carries committed refs; M1 kind kept for compatibility | T09/T01 |
| X13 | `ResultValue` set `compiler/src/task.rs:1727-1739` | No mutation-summary variant | Commit computes versions/invalidation; no new `ResultValue` variant; OP tasks use `Ack`/`Records` | T01 |
| X14 | No VF07/VF08/VF09 kinds `compiler/src/task.rs:511-532` | V2/V3 CFG/IR/SSA cannot be verified | F-IR-04 verification kind freeze; VF07–VF09 stage 6 | T13/T01 |
| X15 | `StoreId::Opt` exists `compiler/src/task.rs:2097` but no `Arenas.opt` `compiler/src/bus.rs:609-671` | T10 analysis storage has no path | Defer analysis store to T10 freeze; V1–V3 use results/versions only | T10/T01 |
| X16 | `RecordFamily`/wire tags `compiler/src/ids.rs:332-400`; `RecordRef` tags `compiler/src/ids.rs:215-257` | New Terminator/Memory/Effect families would require tags/arenas/snapshot | Do not add families for V1–V3; inline sub-records only | T01 |
| X17 | `TaskKind` local 17+ unreserved in the IR group | V1–V3 need many kinds | F-IR-03 reservation; collision check against `/39` head | T01 |
| X18 | Snapshot encoders `compiler/src/snapshot.rs:2193-2318`; hash lists `compiler/src/contract.rs:434-459` | New fields/versions not encoded/hashed | F-IR-01/02 update; `freeze.rs` recompute; M1 snapshot test updated | T01 |
| X19 | `TypeKind::Function` has params/variadic `compiler/src/bus.rs:313-322` | Function signature is usable, but no pointer type for function pointers | T06 TY15/TY17; no new IR call record | T06 |
| X20 | `FunctionRecord.linkage`/`SymbolRecord.linkage` `compiler/src/bus.rs:360-377`, `:449-460` | Possible mismatch | VF08/IR01 check equality | T09/T13 |
| X21 | T09 alias pairs `docs/tasks/T09_IR_LOWER_CHIPS.md:55`; `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1907-1917` | Duplicate rule IDs | Freeze one spelling; retire the other | T01/T09 |
| X22 | M1 proposal `CommitError` list `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:2457-2466` | Proposes whole-batch failure for local op-shape errors | Adopt a per-rule carrier split (§4.17.2); do not force one batch failure for one local op | T01/T09 |
| X23 | `Cmp`/`Convert` have no field in `/39` | V1 conversions/comparisons need a predicate/op | Put payloads in `IrOp` (recommended) or add fields; no private enum | T01/T09 |
| X24 | `SwitchShape` absent; GNU ranges `docs/tasks/T09_IR_LOWER_CHIPS.md:30` | V2 switch cases/ranges | V2 simple constant cases; ranges deferred to T09/T12 | T09 |
| X25 | VF12 M1-only `compiler/src/chips/verify/vf_interpret.rs:5-13` | V1 arithmetic/memory not modeled | T13 extension work order; `Unsupported` still never pass | T13 |
| X26 | `c15_ir` two-op assumptions `compiler/tests/c15_ir.rs:525-575` | New op table/fields break exact tests | Update M1 tests to the new record shape while retaining M1 semantics | T01/T09 |

### 9.2 Current Draft Conflicts

| Draft | Assumption / conflict | Resolution |
|---|---|---|
| [T09_IR_LOWER_CHIPS.md](../../tasks/T09_IR_LOWER_CHIPS.md) | T09/M1 proposal includes `Add` (T09 line 83; M1 proposal lines 1858-1866) but `/39` has only `Constant`/`Return`; `ConstRecord.ty` check (T09 line 57); non-SSA join (line 39); `FunctionEnd` direction (lines 63-69); alias pairs (line 55) | Add `Add` via freeze; resolve `ConstRecord.ty` (X4); use memory-slot joins (X10); keep no-marker direction (X11); freeze one alias (X21) |
| [OP_FREEZE_01.md](../../tasks/OP_FREEZE_01.md) | `UseDef` keyed by `(function_id, ir_version)` and `def_site(value_id)` unique (lines 47-57) | Use `(function, revision, memory_epoch)`; non-SSA has multiple def sites per slot, so key by `(value_or_slot, instruction)`; SSA-only single def (SSA-2) |
| [OP_FREEZE_02.md](../../tasks/OP_FREEZE_02.md) | Effect class taxonomy location open (lines 21-39, 53-57) | Use `EffectMask` bits (§4.10); unknown = `EFFECT_UNKNOWN`; no private effect enum |
| [OP_FREEZE_03.md](../../tasks/OP_FREEZE_03.md) | Version scope open (line 77); `GuardedPatch` apply assumes an apply path; `MutationSummary` open (line 78) | Per-function revision + memory_epoch (§4.15); F-IR-02 typed mutation apply; commit computes summary; no new `ResultValue` variant |
| [VF13_FREEZE.md](../../tasks/VF13_FREEZE.md) | Canonical semantic projection vs scheduler snapshot (lines 25-53) | Include IR records, versions, tombstones in semantic projection; scheduler registers separate; no byte-identical whole snapshot |
| [CT_FREEZE_01.md](../../tasks/CT_FREEZE_01.md) | Stage/phase ordering and continuation/task binding (lines 44-49) | IR is stage 5; no conflict; IR tasks use the existing task envelope; version bump is commit-owned |
| [M1_PART_A_CONTRACT_PROPOSAL.md](../../tasks/M1_PART_A_CONTRACT_PROPOSAL.md) | `EffectMask` bits unassigned (lines 1450-1457); `SemRecord.conversions`; `ConstRecord { ty, value }`; `TypeKind` no pointer; `CommitError` list; `TerminatorMissing` history | Use §4.10 bits; T07/T01 freeze conversions; add `ConstRecord.ty` or drop check; T06 pointer freeze; carrier split; no marker family |
| [T10_OPTIMIZE_CHIPS.md](../../tasks/T10_OPTIMIZE_CHIPS.md) | `Analyze(function/version)`, `RewriteCandidate(instruction/version)` (lines 3-5) assume an IR version and apply path that do not exist | F-IR-02 provides `IrVersionStore` + typed apply; `chips/optimize/` directory is new |
| [T13_VERIFICATION_CHIPS.md](../../tasks/T13_VERIFICATION_CHIPS.md) | VF07–VF09 out of M1 (line 87); VF12 M1-only; no fact store | V1–V3 add VF07–VF09; VF12 extension; facts are results for now |
| [T07_SEMANTIC_CHIPS.md](../../tasks/T07_SEMANTIC_CHIPS.md) | M1 `EffectMask(0)` only (lines 56, 75, 100); effect bit classes reserved | V1+ freeze replaces the M1 rule; same mask bits for sem/IR |
| [T08_CONSTANT_LAYOUT_INIT_CHIPS.md](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md) | `ConstRecord`/layout shapes pending; `LayoutId` reserved | T08 freeze; `ConstRecord.ty` resolution; `Option<LayoutId>` in IR |
| [T06_SYMBOL_TYPE_CHIPS.md](../../tasks/T06_SYMBOL_TYPE_CHIPS.md) | TY15 pointer/TY16 array/TY17 function types are catalog rows, not frozen | V1/V3 dependency; IR does not invent types |

### 9.3 Blocking Defects Returned to the Integrator

These are structured defects, not implementation requests. They block a freeze or a later train,
not the whole Stage A audit.

| Defect | Conflict | Why it blocks | Minimal change request | Owner |
|---|---|---|---|---|
| DEFECT-A15-01 | X8 | `StorePatch` `Replace`/`Tombstone` is log-only and has no typed body; OP20 cannot mutate IR | Add typed `MutateRecords` (or accepted equivalent) and checked `replace`/`remove`; reject body-less replace | T01/T02 |
| DEFECT-A15-02 | X4 | `ConstRecord` has no `ty`; the T09 `ir.op.immediate-type` check cannot be implemented as written | Adopt A14's `ConstRecord` v2 (`ty` + tagged `ConstValue`) or add `ty` minimally; otherwise remove the check and document the weaker rule | T08/T01 |
| DEFECT-A15-03 | X3 | `EffectMask` has no assigned bits and M1 allows only zero; V1 memory/V3 calls cannot be represented | Freeze effect bits 0–8 and replace the zero-only rule with a versioned rule | T01/T07 |
| DEFECT-A15-04 | X6/X7 | No pointer/array type and no typed layout; V1 address values/V3 pointers/arrays cannot be typed or lowered | T06 TY15/TY16 and T08 layout freeze before V1 memory code | T06/T08 |
| DEFECT-A15-05 | §5.2.1 | `Payload` is `RecordRef`-only while IR tasks need ordinals/flags/op selectors | Derive descriptors from committed records and fold block creation into construct tasks, or freeze a typed payload extension | T01/T02/T09 |
| DEFECT-A15-06 | X9 | Per-store versions cannot guard per-function analyses; append path does not bump versions; no invalidation hook | Add `IrVersionStore` + commit-owned revision/epoch bump, bump every mutated store once (append or patch), and stale rule | T01/T10 |
| DEFECT-A15-07 | X11 | `FunctionEnd` result typing/OB-26 rejection semantics unresolved | Select the task result/carrier and hook rejection semantics; keep no marker family | T09/T01 |

### 9.4 Cross-Report Corroboration (Stage A Siblings)

These reports are **not frozen** and were produced in the same Stage A batch. They are cited only
to deduplicate requirements; where they propose a different carrier, this report records the
conflict instead of silently choosing both.

| Sibling report | Finding | Alignment / conflict |
|---|---|---|
| [A14_CONST_LAYOUT_INIT_CONTRACT.md](A14_CONST_LAYOUT_INIT_CONTRACT.md) | Proposes `ConstRecord` v2 with `ty: TypeId` and a tagged `ConstValue`, replacing `{value, negative}`; proposes `LayoutRecord`/`LayoutId` typed | Corroborates X4/X7 and Q2/Q4. Adopt A14's `ConstRecord` v2 at freeze rather than adding a second constant shape; IR's `Option<LayoutId>` matches A14 |
| [A16_CONTROL_PROTOCOL_REQUIREMENTS.md](A16_CONTROL_PROTOCOL_REQUIREMENTS.md) | Confirms `StorePatch` only appends to `patch_log`; `AppendRecords` materializes typed bodies; `Progress`/`StorePatch` coexistence is a freeze conflict | Corroborates X8 and DEFECT-A15-01; our `MutateRecords` proposal is the typed alternative. A16's `Progress` conflict is outside IR |
| [A17_VERIFICATION_REQUIREMENTS.md](A17_VERIFICATION_REQUIREMENTS.md) | Proposes one `VerificationRecord` family/store (`VerificationFact`/`Result`/`TestEvidence`), VF07/VF08 stage 6, VF09 stage 7 candidate; OQ-8 leaves `StoreVersions` out of the semantic projection | Our V1–V3 result-only carrier is the minimal subset; if A17's family is accepted, use its `Fact`/`Result` and VF09 stage 7. Do **not** create both a result-only and a family carrier. Include `IrVersionStore` in the semantic projection; treat coarse `StoreVersions` as a freeze decision (A17 OQ-8) |
| [A23_DETERMINISM_REVIEW.md](A23_DETERMINISM_REVIEW.md) | Finds `store_versions` bumped only for `StorePatch`, not typed appends; `ResultRecord.version` always 0; tombstone log-only | Corroborates X8/X9 and DEFECT-A15-01/06. F-IR-02 must bump every mutated store exactly once, make `ResultRecord.version` meaningful or remove it, and apply tombstones |
| [A07_V2_CONTROL_FLOW_CONTRACT.md](A07_V2_CONTROL_FLOW_CONTRACT.md) | O9: the `FunctionEnd` phase-2b hook may need to check every live block/successor, not only the entry, or be explicitly scoped | Our CFG-4/VF07 checks all blocks; CFG-9 keeps the T09 entry-only hook. See Q23; do not silently widen the hook without a contract update |
| [A08_V3_FUNCTIONS_MEMORY_ABI_CONTRACT.md](A08_V3_FUNCTIONS_MEMORY_ABI_CONTRACT.md) | Proposes V3 IR kinds `ir.global_object`, `ir.function` at stage 5 and VF08 `[Function]` | Our report proposes granular IR kinds. Record/CFG/effect invariants are independent of task granularity; the freeze must pick one producer set, not both. A fused `ir.function` must still carry the same committed refs/records, but `/39` `IR_FUNCTION` local 16 is the M1 two-instruction shape — do not silently broaden it; add a new kind or freeze a breaking replacement. `ir.global_object` is a T09 specialization of `ir.address`/`ir.init` or a distinct kind |
| [A18_DIAGNOSTIC_TAXONOMY.md](A18_DIAGNOSTIC_TAXONOMY.md) | Owns the shared diagnostic/rule-ID carrier and numeric-code freeze | Our rule-ID names are candidates; A18/T01 owns the carrier and numeric inventory. Do not create a second rule-ID type |
| [A22_TARGET_MODEL_REQUIREMENTS.md](A22_TARGET_MODEL_REQUIREMENTS.md) | Owns target model/probe values; IR must stay target-neutral | Aligns with §1.3/§4; no target width/ABI value is added by this report |

## 10. Ordered Freeze Work Orders

All work orders are **PROPOSED** and **not dispatchable** until the integrator accepts them. No
freeze work order authorizes chip code. The `/40`–`/43` labels below are **placeholders**; the
integrator assigns the actual next version after deduplicating the sibling Stage A freeze packages
(A14, A17, etc.).

**Freeze order:**

0. **Upstream co-freeze:** T06 pointer/array (TY15/TY16), T08 `LayoutRecord` and
   `ConstRecord.ty`, T07 `EffectMask` bits, and the T09/T01 `FunctionEnd`/OB-26 direction.
1. **F-IR-01** record/op/effect shape.
2. **F-IR-02** per-function versions and typed mutation apply.
3. **F-IR-03** task kinds/stage/allowlist.
4. **F-IR-04** verification fact carrier (optional for V1–V3; required before a durable fact
   store or a full VF09 verdict).

Do not start F-IR-02 before F-IR-01 because the mutation bodies/effect comparison depend on the
record shape. Do not start F-IR-03 before F-IR-01 because payload/task shapes depend on the op
table. Do not start any implementation work order before F-IR-01..03 are recorded in the contract
version/hash and dependent tests are re-run.

### F-IR-01 — Freeze IR record/op/effect shape (candidate `/40`)

- **Role:** T01 integrator (one logical writer).
- **Scope:** `FunctionRecord.form`; `InstructionRecord.successors/effect/detail`;
  `InstructionDetail`; `MemoryAccess`/`MemoryKind`/`MemoryOrder`; `CallShape`; `ObjectRef`/
  `StorageClass`; `SwitchShape`; `IrForm`; `IrOp` variants and `CmpPredicate`/`ConversionOp`;
  `EffectMask` bit names; field-name lists and rule IDs.
- **Files:** `compiler/src/bus.rs`; `compiler/src/contract.rs`;
  `compiler/src/snapshot.rs` (encode/decode); `compiler/src/records.rs` (draft bodies if
  `InstructionDetail` needs a typed draft); `compiler/src/lib.rs`/`prelude.rs` exports;
  `compiler/contracts/CONTRACT_VERSION`; `compiler/tests/freeze.rs`; `compiler/tests/c15_ir.rs`;
  dependent `compiler/tests/c17_vf12.rs` and `compiler/tests/c19_vf01.rs` (record shape);
  new `compiler/tests/c40_ir_shape.rs`.
- **Must not:** add chip files, task kinds, store fields, or a new ID family.
- **Acceptance:** M1 `Constant`/`Return` round-trip with empty/default new fields; all new
  sub-records round-trip; effect bit names/`IR_OP_NAMES`/field lists in the hash; no target width
  or ABI value; `fmt`/`clippy`/targeted tests pass.
- **Dependencies:** T06 pointer/array and T08 layout/`ConstRecord.ty` co-freeze for the fields that
  reference them; T07 effect-bit co-freeze.
- **Version:** bump only after the full shape is reviewed; if split, record each partial version.

### F-IR-02 — Freeze per-function versions and typed mutation apply (candidate `/41`)

- **Role:** T01 integrator + T02 commit owner.
- **Scope:** `IrVersionStore`; `RecordMutation`/`MutateRecords` (or the accepted alternative);
  checked `replace`/`remove`; atomic apply order; revision/epoch bump; stale rejection;
  tombstone-reference validation; snapshot encoding of versions.
- **Files:** `compiler/src/bus.rs`; `compiler/src/task.rs` (proposal/wire tag); `compiler/src/commit.rs`;
  `compiler/src/arena.rs`; `compiler/src/snapshot.rs`; `compiler/src/contract.rs`;
  `compiler/tests/freeze.rs`; dependent `compiler/tests/c03_task.rs` and
  `compiler/tests/c05_codec.rs` (proposal tag/encoding); new `compiler/tests/c41_ir_mutation.rs`.
- **Must not:** implement OP chips or rewrite algorithms; no language rules.
- **Acceptance:** replace/delete atomicity; stale patch rejection; no partial apply; tombstone
  no-live-reference; revision +1 once; memory epoch only on memory change; every mutated store
  (typed append or patch) bumps exactly once; failed batch no bump; snapshot/replay identical;
  H6 recovery path. `ResultRecord.version` must either become the real production version or be
  removed from the semantic guard.
- **Dependencies:** F-IR-01; T02 H6 atomic-batch direction; `StoreVersions` contract.
- **Open:** mutation apply envelope (§12 Q5); FunctionEnd/OB-26 rejection semantics (§12 Q7).

### F-IR-03 — Freeze IR task kinds, stage, and allowlist (candidate `/42`)

- **Role:** T01 integrator.
- **Scope:** `ir.*` kinds (§5.2), `verification.cfg_invariant`/`ir_invariant`/`ssa_invariant`;
  local-code reservation; `STAGE_ASSIGNMENT` rows; `STORE_OWNER_ALLOWLIST` rows; task payload
  shape guards; `StoreSchema` fields (`ir.*` unchanged; no `opt` yet).
- **Files:** `compiler/src/task.rs`; `compiler/src/manifest.rs`; `compiler/src/contract.rs`;
  `compiler/tests/c42_ir_kinds.rs`; `compiler/tests/freeze.rs`.
- **Must not:** add chip implementations or modify existing PA/SE/TY kinds.
- **Acceptance:** unique kind names/codes; stage 5/6/7 and layer agreement (VF09 at 7); allowlist
  rows for every writer; read-only verifier kinds; no duplicate producer; M1 `IR_FUNCTION`
  unchanged.
- **Dependencies:** F-IR-01; T09 catalog names; T13 VF rows.

### F-IR-04 — Freeze verification fact carrier (candidate `/43`, optional for V1–V3)

- **Role:** T01 integrator + T13 owner.
- **Scope:** decide result-only facts vs durable `AnalysisFactRecord`/verification store; define
  `NotApplicable` carrier if needed; freeze VF07–VF09 rules.
- **Files:** `compiler/src/task.rs`; `compiler/src/manifest.rs`; `compiler/src/contract.rs`;
  `compiler/src/snapshot.rs` only if a store is added; `compiler/tests/c43_ir_verify_facts.rs`.
- **Must not:** add a private verifier store without the integrator's schema freeze.
- **Acceptance:** non-SSA VF09 is not a false failure; every verifier verdict is typed; no
  `Unsupported` pass.
- **Dependencies:** F-IR-03; T13 VF07–VF09 rows.
- **Note:** V1–V3 can proceed with result-only facts; this order can be deferred without blocking
  F-IR-01..03.

## 11. Ordered Implementation Work Orders (Not Authorized)

All are **PROPOSED**; they become READY only after F-IR-01..03 (and F-IR-04 where stated). Each
owner edits only its files; the integrator updates `mod.rs`, registration, manifests, and test entry
points. Tests must cover normal, boundary, invalid, unsupported/recovery, replay, non-matching
task, and unauthorized-field-unchanged cases.

### I-IR-V1-01 — Function, block, constant, address, load, store, convert

- **Chips:** IR01 `ir_function_begin`, IR02 `ir_block_create`, IR03 `ir_constant`, IR04
  `ir_address`, IR05 `ir_load`, IR06 `ir_store`, IR07 `ir_convert`.
- **Paths:** `compiler/src/chips/ir_lower/ir_function_begin.rs`,
  `compiler/src/chips/ir_lower/ir_block_create.rs`,
  `compiler/src/chips/ir_lower/ir_constant.rs`,
  `compiler/src/chips/ir_lower/ir_address.rs`,
  `compiler/src/chips/ir_lower/ir_load.rs`,
  `compiler/src/chips/ir_lower/ir_store.rs`,
  `compiler/src/chips/ir_lower/ir_convert.rs`.
- **Tests:** `compiler/tests/c44_ir_v1.rs`.
- **Reads:** task payload; `parse.nodes`, `symbols.symbols`, `types.records`, `sem.records`,
  `constants.records`, `layout.*`, `ir.*` as needed. **Writes:** `ir.functions`, `ir.blocks`,
  `ir.values`, `ir.instructions` only.
- **Acceptance:** `int` local init/read/write; address-taken local; `char`/`int` truncation/extension
  under a committed T07 plan; unaligned/packed load via `LayoutId`; volatile load/store preserved;
  no target width; no refold; stable IDs; replay.
- **Prereq:** F-IR-01..03; T06 pointer type; T08 layout/`ConstRecord.ty`; T07 effect bits.
- **Forbidden:** shared records, task enums, commit, snapshot, `mod.rs`, Cargo, other groups.
- **Note:** do not extend the M1 `ir_function.rs` singleton scans; new chips must take committed
  refs in the payload.

### I-IR-V1-02 — Integer unary/binary and assignment/RMW

- **Chips:** IR08 `ir_unary`, IR09 `ir_binary`, IR14 `ir_assignment`, IR15 `ir_rmw`.
- **Paths:** `compiler/src/chips/ir_lower/ir_unary.rs`, `ir_binary.rs`,
  `ir_assignment.rs`, `ir_rmw.rs`.
- **Tests:** `compiler/tests/c45_ir_v1_expr.rs`.
- **Acceptance:** signed/unsigned division/remainder; `INT_MIN`/shift UB not host-folded;
  unaudited alias/volatile blocked; lvalue address computed once; compound RMW old/new values;
  replay.
- **Prereq:** I-IR-V1-01; T08 layout; T07 checked conversions/effects; T13 VF08.
- **Forbidden:** shared paths; target widths; host `sizeof`.

### I-IR-V2-01 — CFG lowering and VF07

- **Chips:** IR11 `ir_compare`, IR12 `ir_short_circuit`, IR13 `ir_conditional`, IR20 `ir_if`,
  IR21 `ir_loop`, IR22 `ir_switch`, IR23 `ir_jump`; VF07 `verification.cfg_invariant`.
- **Paths:** `compiler/src/chips/ir_lower/ir_compare.rs`, `ir_short_circuit.rs`,
  `ir_conditional.rs`, `ir_if.rs`, `ir_loop.rs`, `ir_switch.rs`, `ir_jump.rs`;
  `compiler/src/chips/verify/vf07_cfg.rs`.
- **Tests:** `compiler/tests/c46_ir_v2.rs`, `compiler/tests/c47_vf07.rs`.
- **Acceptance:** short-circuit RHS executes only on the selected path; loops/continue/break;
  switch default/fallthrough; no instruction after a terminator; all blocks reachable; VF07 catches
  dangling edges/duplicate terminators; replay.
- **Prereq:** V1; F-IR-03; VF07 kind.
- **Forbidden:** shared paths; CFG in a helper that hides the traversal.

### I-IR-V3-01 — Calls, returns, pointers/arrays/member, init/copy, dynamic stack, VF08

- **Chips:** IR10 `ir_pointer_arith`, IR16 `ir_member`, IR18 `ir_call`, IR19 `ir_return`, IR24
  `ir_init`, IR25 `ir_copy`, IR26 `ir_dynamic_stack`, IR28 `ir_function_end`; VF08
  `verification.ir_invariant`.
- **Paths:** `compiler/src/chips/ir_lower/ir_pointer_arith.rs`, `ir_member.rs`, `ir_call.rs`,
  `ir_return.rs`, `ir_init.rs`, `ir_copy.rs`, `ir_dynamic_stack.rs`, `ir_function_end.rs`;
  `compiler/src/chips/verify/vf08_ir.rs`.
- **Tests:** `compiler/tests/c48_ir_v3.rs`, `compiler/tests/c49_vf08.rs`.
- **Acceptance:** direct/indirect calls; mixed args; variadic signature; aggregate return;
  recursive call; global/static/TLS addresses; array/pointer arithmetic; member/subscript
  addressing; VLA/alloca cleanup on early return; `FunctionEnd` completion fact; VF08
  type/effect/memory; replay. VF09 is a separate SSA work order.
- **Prereq:** V1/V2; T06 TY15/TY16/TY17; T08 init/layout; T11 ABI for execution (Part B).
- **Forbidden:** target ABI values in IR; shared paths; new `ResultValue` variant.

### I-OPT-01 — Analysis/rewrite/versioning

- **Chips:** OP01 `PassSelect`, OP11 `UseDef`, OP20 `ApplyRewrite`, OP21 `Invalidate`, OP22
  `Fixpoint` (minimal version-guarded set).
- **Paths:** `compiler/src/chips/optimize/op01_pass_select.rs`, `op11_use_def.rs`,
  `op20_apply_rewrite.rs`, `op21_invalidate.rs`, `op22_fixpoint.rs` (new directory).
- **Tests:** `compiler/tests/c50_opt_versioning.rs`.
- **Acceptance:** stale patch rejected; no partial apply; revision/epoch bump; analysis facts
  keyed by version; invalidation after data/CFG/memory change; deterministic fixpoint; no
  volatile/atomic/call deletion without legality; before/after IR interpretation/target diff.
- **Prereq:** F-IR-02; V1–V3 IR; T10 legality drafts (OP_FREEZE_01/02/03) reconciled.
- **Forbidden:** direct IR mutation; shared records; private analysis types.

### I-VER-01 — VF12/VF13 extension

- **Chips:** VF12 `verification.ir_interpret` extension; VF13 `verification.replay_compare`
  (deferred).
- **Paths:** `compiler/src/chips/verify/vf_interpret.rs` (extend), `vf_replay.rs` (new).
- **Tests:** `compiler/tests/c51_ir_replay.rs`.
- **Acceptance:** V1 arithmetic/memory subset modeled; `Unsupported` never pass; VF13 semantic
  projection includes IR records/versions/tombstones; scheduler snapshot compared separately.
- **Prereq:** V1–V3; F-IR-02; VF13 freeze direction.
- **Forbidden:** target execution; shared paths.

### I-VER-02 — VF09 SSA invariant (after SSA/`Phi` freeze)

- **Chips:** VF09 `verification.ssa_invariant`.
- **Paths:** `compiler/src/chips/verify/vf09_ssa.rs`.
- **Tests:** `compiler/tests/c52_vf09.rs`.
- **Acceptance:** only `form == Ssa` is checked; definitions dominate uses; join/phi incoming
  edges; non-SSA functions are not flagged; `NotApplicable` carrier per F-IR-04/A17.
- **Prereq:** I-OPT-01 (SSA/`Phi`); F-IR-03; F-IR-04.
- **Forbidden:** flagging non-SSA functions; shared paths.

## 12. Open Questions and Rejected Alternatives

### 12.1 Open Questions

| ID | Question | Owner | Recommendation |
|---|---|---|---|
| Q1 | Final `EffectMask` bit assignment; reserved bits 9–31 | T01 + T07 | Adopt §4.10 bits 0–8; keep 9–31 reserved/zero; `EFFECT_UNKNOWN = 0x1FF` (all defined bits) |
| Q2 | `ConstRecord.ty`: add it, or drop `ir.op.immediate-type`? | T08 + T01 | Add `ty: TypeId`; otherwise the T09 check is unenforceable |
| Q3 | Pointer/array/qualified type shapes | T06 | TY15/TY16/TY14 freeze; IR uses `TypeId` only |
| Q4 | `LayoutRecord` shape and `LayoutId` stability | T08 | Freeze size/alignment/bitfield descriptors; IR uses `Option<LayoutId>` |
| Q5 | Mutation apply envelope: `MutateRecords` vs extend `StorePatch` | T01 + T02 | New `MutateRecords` with typed bodies; keep `StorePatch` mechanical |
| Q6 | Verification fact persistence and `NotApplicable` | T13 + T01 | Result-only for V1–V3; define a typed verdict only if VF09 needs it |
| Q7 | `FunctionEnd` result carrier and OB-26 rejection semantics | T09 + T01 | `Ack` + task-kind completion fact; resolve OB-26 before F-IR-03 |
| Q8 | Commit vs chip vs verifier carrier per rule | T01 + T09 + T13 | Local op shape = chip; cross-record = commit; post-commit = VF |
| Q9 | `StackMark` type and stack balance | T06 + T11 | Pointer-to-opaque or `StackMark` type; explicit `StackSave`/`StackRestore`; T11 maps |
| Q10 | String literal address carrier | T04 + T08 + T09 | `ObjectRef { class: StringLiteral, literal: Some(LiteralId) }` + T08 bytes/init; no new record family |
| Q11 | GNU switch ranges / duplicate cases | T09 + T12 | V2 simple values; ranges deferred |
| Q12 | Analysis version granularity | T10 + T01 | Single per-function `revision` + `memory_epoch` for V1–E-OPT; refine only with evidence |
| Q13 | Function→block traversal/index | T01 + T09 | Reachability invariant (CFG-3) removes the need for a block list; revisit if profiling shows a bottleneck |
| Q14 | `Undef` semantics/folding | T09 + T10 | `Undef` is a value, effect 0; optimizations must not fabricate a defined value |
| Q15 | `Alloca`/`Address` effect classification | T09 + T10 | `Alloca` = `STACK`; `Address` = 0; no ordinary read/write |
| Q16 | FP `Cmp`/`Convert` predicates | T09 + T12 | Defer; keep `CmpPredicate`/`ConversionOp` extensible |
| Q17 | `Param` op vs `FunctionRecord.params` | T09 + T01 | `Param` op (uniform producer rule); entry order |
| Q18 | `FunctionRecord.form` vs version-store `form` | T01 | `FunctionRecord.form` (semantic body property); version store only versions |
| Q19 | `IrVersionStore` snapshot/hash scope | T01 + C05 | Snapshot-encode sorted entries; static hash includes field/rule names, not runtime values |
| Q20 | `MutateRecords` wire tag and proposal ordering | T01 + T02 | Tag 8 after frozen 0–7; deterministic batch order; one transition still required |
| Q21 | IR task scalar parameters (block ordinal, flags, op selectors) vs `RecordRef`-only `Payload` | T01 + T02 + T09 | Prefer deriving from committed Node/Sem/Type/Layout records and folding block creation into construct tasks; if not sufficient, freeze a typed per-kind payload extension, not an IR-private type |
| Q22 | `Atomic`/`Fence` op split and `MemoryKind` matching | T09 + T01 | One `Atomic` op + `MemoryKind` selector; `Fence` is `Atomic+Fence`; no duplicate fence op |
| Q23 | `FunctionEnd` phase-2b hook scope: entry-only vs every live block/successor (A07-O9) | T09 + T01 + T13 | Keep the hook entry-only as selected, and let VF07 check all live blocks/successors; if the hook is extended, update CFG-9 and add no new marker family |

### 12.2 Rejected Alternatives

| Alternative | Why rejected |
|---|---|
| Separate `TerminatorRecord`/`MemoryRecord`/`EffectRecord` families | Duplicates instruction identity and requires new IDs, tags, arenas, snapshot encoders, and hash entries; inline `successors`/`effect`/`detail` is smaller and sufficient |
| SSA-only base IR / multiple definitions per `ValueId` | V1–V3 correctness first; non-SSA memory slots avoid multi-def values and keep `ValueRecord` frozen; SSA is optional via OP14/OP15 |
| `StorePatch { value: RecordRef }` used for typed replacement | `RecordRef` has no body; replacement is impossible without a typed body or a new proposal |
| New `ResultValue` variant for mutation summary | T09 explicitly avoids new result variants for `FunctionEnd`; commit can compute versions/invalidation without a result variant |
| Requiring a durable verification store before V1–V3 | Result-only `Ack`/`Fail` is sufficient; a store adds an ID/family/tag/snapshot/hash cost before optimization needs it. A17 may still freeze it later |
| Per-analysis version maps at V1 | Over-invalidation is correct and simpler; refine only with measured evidence |
| Target widths/alignments/registers/`sret` in IR | Violates the target-neutral IR boundary and the probe gate; T11 owns concrete values |
| Global singleton scans for “the one symbol/type/const” | Breaks with multiple functions/constants; use committed refs in the task payload |
| Stored `ValueRecord.def` / producer back-link | A second source of truth; the producer is the unique instruction `result`; verifier builds an index when needed |
| Stored block instruction list | A second source of truth; derived by ascending `InstructionId` + `block` |
| Unreachable live blocks allowed | Makes a function→block index necessary; reachability invariant (CFG-3) keeps traversal from entry and lets OP08 delete in the same batch |
| `FunctionRecord.params` | Adds a producer exception; `Param` instructions are uniform and verifier-friendly |
| In-place `FunctionRecord.revision` mutation | Mixes semantic body and version state; a per-function register is explicit and snapshot-friendly |
| `(&'static str)` rule IDs embedded only in messages | Messages are not structured; a `rule` field or `RuleId` is an error-taxonomy freeze item (A18/T01), not an IR-private choice |

## 13. Validation and Evidence for This Report

- **Change type:** documentation-only; no source, test, contract, Cargo, `mod.rs`, shared index, or
  user draft was modified.
- **Files read (selection):** `AGENTS.md`; `docs/tasks/README.md`;
  `docs/tasks/T01_COMPILER_CONTRACT.md`; `docs/tasks/PARALLEL_EXECUTION.md`;
  `docs/tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md`; `docs/tasks/T09_IR_LOWER_CHIPS.md`;
  `docs/tasks/T10_OPTIMIZE_CHIPS.md`; `docs/tasks/T13_VERIFICATION_CHIPS.md`;
  `docs/tasks/TASK_TEMPLATE.md`; `docs/tasks/MASS_PARALLEL_DISPATCH.md`;
  `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md`; `docs/tasks/M1_TARGET_ACCEPTANCE.md`;
  `compiler/src/{bus,ids,task,commit,arena,snapshot,contract,manifest,records}.rs`;
  `compiler/src/chips/ir_lower/ir_function.rs`;
  `compiler/src/chips/verify/{vf_interpret,vf_invariant,vf_store}.rs`;
  `compiler/tests/c15_ir.rs`; current user drafts `OP_FREEZE_01..03.md`, `VF13_FREEZE.md`,
  `CT_FREEZE_01/03.md`; Stage A sibling reports A07, A08, A14, A16, A17, A18, A22, A23 (for
  cross-report corroboration only).
- **Checks performed:** path/line citation consistency, schema field/status consistency, rule-ID
  uniqueness within this report, proposal-vs-frozen separation. No runtime test, probe, corpus, or
  target run was performed; none is claimed.
- **Checks not performed:** full `cargo fmt`/`clippy`/`test` suite (not applicable to a
  documentation-only report and not authorized by this task); contract validator (no applicable
  validator path confirmed); target probe; torture corpus.
- **Known limitations:** the report proposes names/shapes but does not freeze numeric local codes,
  wire tags, hash values, or target values. The exact `EffectMask` bit freeze, `ConstRecord.ty`,
  pointer/layout types, mutation-apply envelope, and `FunctionEnd`/OB-26 carrier remain open.

## 14. Revision Record

| Date | Change | Status |
|---|---|---|
| 2026-11-07 | Initial A-PTS-05 report: baseline `/39`, V1–V3 requirements, minimal shared schema, CFG/effect/version/tombstone rules, ownership, determinism/replay, `/39` and draft conflicts, ordered freeze/implementation work orders, open questions/rejected alternatives. | Design only; no freeze, no code, no tests run |
