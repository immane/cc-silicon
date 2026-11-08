# Contract Change Request B2-01: M1 Part A A3 Closure (`SE29` → `VF06` → IR)

| Field | Value |
|---|---|
| CCR ID | **B2-01** (Stage B2, `docs/tasks/DISPATCH_LEDGER.md` §3C) |
| Status | **ACCEPTED 2026-11-09 — frozen as `t01-c01-c06/40`; implemented and verified (D-09 remains open)** |
| Date | Submitted 2026-11-07; accepted 2026-11-09 |
| Approval | **T01 integrator ACCEPTED 2026-11-09**; O-1..O-12 defaults recorded in §11 |
| Baseline artifact | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` |
| Target baseline | `aarch64-unknown-linux-gnu`, `target_verification=unverified`, Part B not available |
| Related findings | **D-02 (P0)**: SE29 is not dispatched and the FunctionDefinition `SemRecord` is neither committed nor consumed. **D-09 (P1)**: `ConstRecord` has no `ty`; the documented immediate-type rule is not enforceable. |
| Inputs | `AGENTS.md` §3–5; T01 §3–5; T07 SE29/SemRecord/VF06 candidate rows; T13 VF06 amendments; `M1_TARGET_ACCEPTANCE.md` A3 and IR-4; `M1_VERTICAL_SLICE_ACCEPTANCE.md` VF06/VF06-gap; `SE_FUNC_SLICE.md`; `A05_IR_TARGET_HOST.md` §3.2 G3/G4; `A04_PARSE_TYPE_SEMANTICS.md` §5.3; `A17_VERIFICATION_REQUIREMENTS.md`; current source and tests listed in §2 |
| Write scope of this CCR | This file only. No source, test, contract, Cargo, `mod.rs`, ledger, or other report was edited. No test was run for the original submission; all pre-acceptance statements below are read-only inspection plus proposed work. Post-acceptance implementation evidence is recorded in §11. |
| Line references | Line numbers were read from the current working tree on 2026-11-07. Concurrent B1-01/B1-02 edits may shift them; the code identifiers and anchors are authoritative, not the numbers. |

---

## 1. Executive Summary

The M1 fixture `int main(void){return 2+3;}` has a working deterministic Part A pipeline for the implemented slice, but M1 A3 is not complete:

1. `candidate` installs the `/31` `pp_emit_slice()` registry and never dispatches `SEMANTIC_FUNCTION_DEF` (SE29). No `FunctionDefinition` `SemRecord` is committed.
2. VF06 checks only `IntLiteral`, `BinaryAdd`, and `Return`, and applies one blanket `sem.ty == canonical int` rule. If a `FunctionDefinition` fact were added to the checked set unchanged, VF06 would reject its legitimate function-signature `TypeId`.
3. `IrFunctionChip` obtains the signature from the declarator-linked `SymbolRecord.ty` instead of consuming the committed `FunctionDefinition` `SemRecord`. A missing or wrong function-definition fact would not be detected by lowering.

This CCR proposes the minimal M1 Part A closure:

- **Wire SE29 into the candidate** after the SE return check and before VF06. SE29 itself is already frozen at `/39`; no SE29 logic change is required for the minimal closure.
- **Extend VF06** from a three-kind checked set to a per-kind typed-fact table that includes `FunctionDefinition` with the committed `int(void)` signature `TypeId`, while preserving the `/14` rules for `IntLiteral`, `BinaryAdd`, and `Return`.
- **Make T09 consume** the committed `FunctionDefinition` `SemRecord` as the source of `FunctionRecord.signature`; the declarator-linked symbol remains a structural link and must be cross-checked, never used as a fallback signature source.
- **Keep `ConstRecord.ty` out of this CCR.** D-09 is assigned to a separate tagged-constant CCR (proposed ID **B2-02**). M1 A3 is closable without it; the M1 Part A IR-4(b) immediate-type rule is not, and must not be claimed at `/40`.

### 1.1 Decisions requested by this CCR

| # | Decision | Recommendation |
|---|---|---|
| D1 | SE29 chip scope | Keep SE29 frozen exactly as `/39`; add candidate wiring only. |
| D2 | VF06 checked set | Extend to four M1 kinds with a per-kind expected-type table; `FunctionDefinition` expects the unique committed `int(void)` signature. |
| D3 | T09 function fact | Require exactly one committed `FunctionDefinition` `SemRecord`; take `signature` from it; cross-check `symbol.ty == signature`; no fallback. |
| D4 | `ConstRecord.ty` | Separate tagged-constant CCR (B2-02). Do not add `ty` in B2-01 and do not claim `ir.op-immediate-type` at `/40`. |
| D5 | Registration/schema delta | No new task kind, chip, stage, allowlist row, record family, store field, `RecordRef` tag, or `ResultValue` variant. Candidate registry selection changes; contract shape does not. |
| D6 | Scope | Minimal M1 Part A A3 only. No V1 conversions, AST, parser generalisation, tagged constants, verification-record family, or Part B target work. |

### 1.2 Non-claims

- This CCR does **not** claim M1 acceptance, M1 Part A acceptance, target acceptance, or a GCC torture rate.
- This CCR does **not** authorize implementation. Implementation starts only after the T01 integrator accepts the CCR and assigns the freeze.
- This CCR does **not** edit or supersede the frozen `/39` slice records. Any status change is a new `/40` amendment.
- No test, probe, corpus, assembler, linker, or runner was executed for this CCR.

---

## 2. Current State (Read-Only Evidence)

### 2.1 Candidate: SE29 is installed nowhere on the pipeline

`compiler/src/bin/candidate.rs`:

- `install()` sets `bus.kinds = TaskKindRegistry::pp_emit_slice();` and `bus.schema = StoreSchema::pp_macro_slice();` (`:214-216`). The `/31` PP-emit registry has 48 entries and predates `/39`; it does **not** contain `SEMANTIC_FUNCTION_DEF`.
- The worker array (`:220-247`) registers `SeLitChip`, `SeBinChip`, `SeRetChip`, `Vf06Chip`, `Vf05Chip`, `IrFunctionChip`, `Vf12Chip`, `Vf01Chip`, but **not** `SeFuncChip`.
- The route table (`:252-289`) routes `SEMANTIC_LITERAL_EXPR`, `SEMANTIC_BINARY_EXPR`, and `SEMANTIC_RETURN_STMT` to chips 12–14 at layer 4, and VF06 to chip 15 at layer 4, but has **no** `SEMANTIC_FUNCTION_DEF` route.
- `run()` checks literals, binaries, and returns (`:803-832`), then dispatches VF06 (`:834-841`), then finds the `FunctionDefinition` node only for IR lowering (`:842-856`). No SE29 task is bootstrapped.
- `build_workers()` (`:381-465`) does not register `SeFuncChip`.

Consequence: the candidate's M1 evidence contains no `FunctionDefinition` `SemRecord`; the `sem` arena holds the literal/binary/return facts only; VF06 never sees a function fact; IR lowering has no function fact to consume.

### 2.2 SE29 `SeFuncChip`: frozen and complete for the M1 shape, but unwired

`compiler/src/chips/semantic/se_function.rs`:

- Kind `SE_FUNC_TASK_KIND = TaskKind::SEMANTIC_FUNCTION_DEF` (`:28`), SEMANTIC local 21 (`compiler/src/task.rs:266-273`).
- Chip `SE_FUNC_CHIP = ChipId(57)` (`compiler/src/manifest.rs:1168`), stage 4 (`compiler/src/manifest.rs:1510`), route layer 4 in `c39_sefunc.rs`.
- Payload: exactly one `RecordRef::Node` naming the committed `FunctionDefinition` node (`:64-78`).
- Projection validates the M1 definition: three children (`Specifiers`, `Declarator`, `Compound`), declarator with no children (no K&R list), exactly one committed TY17 `int(void)` signature, exactly one symbol declared at the declarator with that signature, one `Return` child with an `int`/`NonLvalue`/`EffectMask(0)` `SemRecord` (`:79-255`).
- Compute (`:306-346`): if a `SemRecord` already exists for the node, complete with that existing `SemId`; otherwise append exactly one `SemRecord { node, ty: signature, category: NonLvalue, effects: EffectMask(0) }` and complete with its predicted `SemId`.
- Manifest reads `Tasks.active.{id,kind,payload,state,owner}`, `Parse/nodes`, `Symbols/symbols`, `Types/records`, `Sem/records`; writes `Sem/records` (`:270-295`).
- Frozen rule IDs: `se.function-signature`, `se.function-body-checked` (`compiler/src/contract.rs:239-240`).
- Tests: `compiler/tests/c39_sefunc.rs` (8 tests) exercise the normal signature, reuse, missing body, K&R, mismatch, determinism, and stage/layer gates.

The chip is already the right M1 fact producer. The frontend fixture `M1-SE-04` in `M1_VERTICAL_SLICE_ACCEPTANCE.md` §5.5 already defines the SE29 expectation, but the production driver's only SE29 evidence is the chip-local `c39_sefunc.rs`; the candidate does not exercise it end-to-end. The defect is that the production driver never dispatches it.

### 2.3 VF06 `Vf06Chip`: three-kind checked set with a blanket `ty == int` rule

`compiler/src/chips/verify/vf_invariant.rs`:

- Kind `VERIFICATION_TYPED_INVARIANT` (VERIFICATION local 16), chip `VF06_CHIP = ChipId(15)`, stage 4; payload exactly one `RecordRef::Node(TU)`; result `Ack` or `Fail`; no writes.
- Projection (`:39-127`) reads the task, the TU node, the canonical `int` type, the pre-order committed node tree, and all committed `SemRecord`s.
- Compute (`:170-239`) checks exactly three node kinds:

```rust
let checked = matches!(
    node.kind,
    NodeKind::IntLiteral | NodeKind::BinaryAdd | NodeKind::Return
);
```

For each checked node it requires exactly one committed `SemRecord`, then checks:

```rust
if sem.ty != input.int { /* fail */ }
if sem.category != ValueCategory::NonLvalue { /* fail */ }
if sem.effects != EffectMask(0) { /* fail */ }
```

- The rule ID `vf06.checked-set-complete` is frozen (`compiler/src/contract.rs:163`).
- `compiler/tests/c14_se.rs:651-703` is the positive/negative VF06 test.

The `FunctionDefinition` node is not checked at all. If it were added to the checked set without changing the type rule, the legitimate `SemRecord.ty = signature` (a `TypeKind::Function` `TypeId`) would fail `sem.ty != input.int`.

### 2.4 T09 `IrFunctionChip`: signature re-inferred from the declarator-linked symbol

`compiler/src/chips/ir_lower/ir_function.rs`:

- Kind `IR_FUNCTION` (IR_LOWER local 16), chip `IR_FUNCTION_CHIP = ChipId(16)`, stage 5; payload exactly one `RecordRef::Node(FunctionDefinition)`; result `Record(Function)`; writes one `Function`, one `Block`, one `Value`, and two `Instruction`s (`Constant`, `Return`).
- Projection (`:120-144`) scans `SymbolRecord`s for `sym.decl == declarator` and takes `symbol_body.ty` as the signature.
- It checks only `Return` and `BinaryAdd` `SemRecord`s (`:204-222`); it never reads a `FunctionDefinition` `SemRecord`.
- It requires exactly one committed `ConstRecord` (`:223-241`) and carries that `ConstId` into the IR `Constant` without refolding.
- Rule IDs `ir.constant-no-refold`, `ir.return-single-terminator`, `ir.function-single-entry` are frozen (`compiler/src/contract.rs:165-167`).
- `compiler/tests/c15_ir.rs:475-577` is the M1 lowering test.

Consequence: T09 can lower a function whose `FunctionDefinition` fact is missing, duplicated, or typed with a non-signature type. The symbol link is structural (`SymbolRecord.decl`), but the signature is a semantic fact and must come from the committed `SemRecord`.

### 2.5 `SemRecord` and `ConstRecord` at `/39`

`compiler/src/bus.rs:496-513`:

```rust
pub struct SemRecord {
    pub node: NodeId,
    pub ty: TypeId,
    pub category: ValueCategory,
    pub effects: EffectMask,
}
```

One record per checked `NodeId`. There is no `conversions` field; M1 identity/no-conversion is represented by the absence of a plan. The frozen fields are hashed as `SEM_RECORD_FIELDS = ["node", "ty", "category", "effects"]` (`compiler/src/contract.rs:431-432`).

`compiler/src/bus.rs:590-602`:

```rust
pub struct ConstRecord {
    pub value: Vec<u8>,
    pub negative: bool,
}
```

`ConstRecord` has **no `ty`**. The frozen fields are `CONST_RECORD_FIELDS = ["value", "negative"]` (`compiler/src/contract.rs:347-348`). `FoldChip` is the sole writer; T09 carries the committed `ConstId` into IR `Constant` without refolding.

### 2.6 Why wiring SE29 alone is insufficient

Wiring `SeFuncChip` into the candidate would commit the function fact, but:

- **VF06 would still pass vacuously** over the function fact because it never visits `FunctionDefinition`. A missing function fact would not be detected by the verifier that is supposed to check A3.
- **VF06 would fail if naively extended**: the blanket `ty == int` rule rejects the function-signature `TypeId`.
- **T09 would still not consume the fact**: it would keep deriving the signature from `symbol.ty`, so a missing/wrong function fact would not block lowering, and the A3 "T09 reads those committed `SemRecord`s without re-inferring" requirement would remain unmet.

D-02 therefore requires three coordinated changes: candidate dispatch, VF06 per-kind checking, and T09 consumption.

### 2.7 Known adjacent gaps not closed by this CCR

| Gap | Status | Why not here |
|---|---|---|
| D-01: `PaTuChip` accepts non-M1 operators/delimiters | Separate B1-01 fix | Parser exact-shape guard is independent of A3. B2-01 must not hide or regress it. |
| D-09: `ConstRecord.ty` missing | Separate tagged-constant CCR (proposed B2-02) | A3 is about the function-definition fact; IR-4(b) immediate-type is a different schema obligation. |
| VF06 registration text drift | Docs reconciliation | `/14` already froze `verification.typed_invariant` stage 4 chip 15; T13's "registration open" wording is stale (A05 §2.5). The `/40` amendment should correct it. |
| VF07–VF11, CFG/SSA/machine/ABI | Out of M1 | No M1 stage; recorded gap. |
| Target codegen/run | Part B, probe-gated | Not touched. |

---

## 3. Proposed Contract Delta

### 3.1 SE29 task, payload, result, and record (unchanged shape)

B2-01 **does not change** the SE29 chip, its kind, its chip ID, its stage, its allowlist row, or its `SemRecord` shape. The existing `/39` slice is the producer.

| Item | Current `/39` | Proposed `/40` |
|---|---|---|
| Task kind | `SEMANTIC_FUNCTION_DEF` (SEMANTIC local 21) | Same |
| Chip / owner | `SE_FUNC_CHIP = ChipId(57)` | Same |
| Stage / route layer | 4 / 4 | Same |
| Payload | `Payload::from_refs(vec![RecordRef::Node(funcdef)])` (exactly one ref) | Same |
| Result | `ResultValue::Record(RecordRef::Sem(sem))` (`Completed`) | Same |
| Writes | One `AppendRecords` batch with one `G1DraftBody::Sem` when absent; none when reusing | Same |
| Record | `SemRecord { node: funcdef, ty: signature, category: NonLvalue, effects: EffectMask(0) }` | Same |
| Signature | Committed TY17 `TypeKind::Function { result: int, params: [], prototype: true, variadic: false }` | Same |
| Failure modes | Missing return fact, K&R list, prototype mismatch, duplicate type/symbol, non-M1 shape | Same |

The only change is **who dispatches it**: the candidate pipeline in §3.2.

Field-placement resolution (T07 S2/S6): the M1 signature is carried in `SemRecord.ty`; there is no dedicated function-signature field. The `Return` node owns the return-role facts; the function-definition record carries no `Return`-role plan. This is the frozen `/39` SE29 choice and is deliberate, not an accidental overload of `ty`. This records the **M1** field-placement sub-item only; the broader T07/CDR F bundle (full conversion matrix, V1 `SemRecord` carrier, non-identity plans) remains pending.

Recommended optional hardening (not required for D-02; open question O-7): when SE29 finds an existing `SemRecord` for the node, validate that it matches the expected signature/category/effects before reusing it; a mismatch should fail as a DEFECT instead of reusing a wrong fact. The current `/39` reuse path returns the existing fact without revalidation. VF06/T09 will catch a wrong fact later, so this is defense-in-depth, not a closure blocker.

### 3.2 Candidate pipeline insertion point

`compiler/src/bin/candidate.rs` changes:

1. **Registry**: replace

   ```rust
   bus.kinds = TaskKindRegistry::pp_emit_slice();
   ```

   with

   ```rust
   bus.kinds = TaskKindRegistry::se_function_slice();
   ```

   `se_function_slice()` is the cumulative `/39` registry (69 entries, cumulative over `vf_evidence_slice()`); it contains `semantic.function_def` and all kinds the candidate already uses. `bus.schema` stays `StoreSchema::pp_macro_slice()`: it already declares `Sem/records` and every field SE29 reads/writes; no schema change is needed.

2. **Install**: add `&SeFuncChip` to the registered worker array (after `&SeRetChip`). `SeFuncChip` is already re-exported by `compiler/src/chips/mod.rs:109-111`; no `mod.rs` change is needed.

3. **Route**: add

   ```rust
   (TaskKind::SEMANTIC_FUNCTION_DEF, SE_FUNC_CHIP, 4),
   ```

   to the route table, after the `SEMANTIC_RETURN_STMT` row. The stage assignment is already 4 (`compiler/src/manifest.rs:1510`), and the route layer must agree.

4. **Workers**: register `SeFuncChip` in `build_workers()` after `SeRetChip`.

5. **Run order**: insert one SE29 step **after** the `Return` loop and **before** the VF06 step:

   ```text
   SE literal → SE binary → SE return → SE function → VF06 → IR → VF12 → VF01
   ```

   The step is:

   ```rust
   let funcdefs = nodes_of(&bus, NodeKind::FunctionDefinition);
   if funcdefs.len() != 1 {
       return Err("expected exactly one function definition".to_string());
   }
   let funcdef = funcdefs[0];
   let _sem = step(
       &mut bus,
       &workers,
       &mut trace,
       TaskKind::SEMANTIC_FUNCTION_DEF,
       SE_FUNC_CHIP,
       Payload::from_refs(vec![RecordRef::Node(funcdef)]),
   )?;
   // Expected: ResultValue::Record(RecordRef::Sem(_)); T09 uses the node-keyed link.
   ```

   The task is a root M1 task like the other SE steps: `parent: None`, `continuation: None`, `owner: SE_FUNC_CHIP`, and the payload shown above. The candidate should assert that the returned value is `ResultValue::Record(RecordRef::Sem(_))`; a different family is a driver error, not evidence. The candidate may ignore the `SemId` because T09 uses the node-keyed committed link. Then dispatch VF06 with the TU payload as today. Reuse the same `funcdef` for the subsequent `IR_FUNCTION` step; do not call `nodes_of(FunctionDefinition)` a second time.

6. **Failure behavior**: the `step` helper drains to quiescence and returns a structured `Err` on a failed task; the candidate exits 1 with `candidate diagnostic: ...` and emits no snapshot/trace/interpret evidence. If SE29 is not installed/routed, VF06 must fail on the missing function fact (after §3.3), so the old pipeline cannot silently succeed.

7. **`-E` path**: no change. `pp_prefix()`/`run_emit()` stop at PP28 and never dispatch SE29. The `-E` golden tests are unaffected.

The insertion is a **host orchestration order**, not a new scheduler rule. SE29 and VF06 are both stage 4; the candidate bootstraps and drains them sequentially, so the SE29 `SemRecord` is committed before VF06 is bootstrapped. Do not bootstrap both in the same tick and rely on the scheduler to order them.

### 3.3 VF06 rule change

#### 3.3.1 Kind, payload, result (unchanged)

| Item | Current `/39` | Proposed `/40` |
|---|---|---|
| Task kind | `VERIFICATION_TYPED_INVARIANT` (VERIFICATION local 16) | Same |
| Chip | `VF06_CHIP = ChipId(15)` | Same |
| Stage / layer | 4 / 4 | Same |
| Payload | `Payload::from_refs(vec![RecordRef::Node(tu)])` | Same |
| Result | `Ack` on complete facts; `Fail` with one primary diagnostic otherwise | Same |
| Writes | None | None |

#### 3.3.2 Projection shape

Current `Vf06Input` (`compiler/src/chips/verify/vf_invariant.rs:22-36`):

```rust
pub struct Vf06Input {
    pub task: TaskId,
    pub state: TaskState,
    pub tu: NodeId,
    pub int: TypeId,
    pub tree: Vec<(NodeId, NodeRecord)>,
    pub sems: Vec<SemRecord>,
}
```

Proposed `/40`:

```rust
pub struct Vf06Input {
    pub task: TaskId,
    pub state: TaskState,
    pub tu: NodeId,
    pub int: TypeId,
    /// The unique committed M1 `int(void)` signature `TypeId`.
    pub signature: TypeId,
    pub tree: Vec<(NodeId, NodeRecord)>,
    pub sems: Vec<SemRecord>,
}
```

`project_vf06_input` continues to resolve the canonical `int`; additionally it scans `Types` for `TypeKind::Function` and requires exactly one function type whose shape is `result == int`, `params.is_empty()`, `prototype == true`, `variadic == false`. Zero matching signatures is a `Task` group missing-input failure; duplicates or a non-M1 function type are explicit failures. This is mechanical projection, not a second semantic analysis: the shape was already checked by TY17/SE29; VF06 needs the `TypeId` to compare against the function-definition fact.

#### 3.3.3 Per-kind typed-fact table

Replace the blanket `matches!` + `sem.ty != input.int` logic with:

| `NodeKind` | Required committed `SemRecord` count | Expected `SemRecord.ty` | Expected `category` | Expected `effects` |
|---|---:|---|---|---|
| `IntLiteral` | exactly 1 | canonical `int` | `NonLvalue` | `EffectMask(0)` |
| `BinaryAdd` | exactly 1 | canonical `int` | `NonLvalue` | `EffectMask(0)` |
| `Return` | exactly 1 | canonical `int` | `NonLvalue` | `EffectMask(0)` |
| `FunctionDefinition` | exactly 1 | committed M1 signature `int(void)` (`input.signature`) | `NonLvalue` | `EffectMask(0)` |

The first three rows preserve the `/14` behavior exactly. The fourth row is new.

Additional M1 rule:

- The TU tree must contain **exactly one** `FunctionDefinition` node. Zero is a `Task` group failure ("missing function definition"); more than one is an explicit failure ("multiple function definitions outside M1"). This intentionally duplicates VF05's AST-count check: VF06 must not pass vacuously if the node the lowering consumer needs is absent. If the integrator prefers to rely solely on VF05, this can be relaxed to "each present `FunctionDefinition` node" (open question O-5).
- The function-definition fact's `ty` is compared to the **same** signature `TypeId` that T09 will place in `FunctionRecord.signature`; this is what makes VF06 and T09 agree on the typed link.
- The function-definition fact does **not** carry a `Return`-role conversion plan (T07 S6); VF06 must not require one. M1 identity/no-conversion remains the absence of a plan.
- The `ValueCategory` for the function-definition fact is the frozen `/39` `NonLvalue`. Changing it to `FunctionDesignator` would be a separate SE29/SemRecord semantics change (open question O-2).

#### 3.3.4 Failure precedence and diagnostics

Preserve the current deterministic order:

1. task state (`Running`) and payload shape;
2. TU node kind;
3. canonical `int` and unique signature presence;
4. pre-order tree walk;
5. for each checked node: missing fact, duplicate fact, type mismatch, category mismatch, effects mismatch.

Use the existing diagnostic pattern: `DiagnosticCode::new(DiagGroup::Task, 4)` for missing/duplicate/mismatch (missing input, DEFECT, verification failure); `DiagnosticDraft::unsupported` for non-M1 shapes. Do not add a diagnostic group, code range, `ResultValue` variant, or verification record family. A17's proposed `Verification`/`Fact`/`Result` family is **not** adopted by this CCR (see §8).

#### 3.3.5 Rule IDs

Keep the frozen rule ID `vf06.checked-set-complete`; its scope now covers the four M1 kinds. Do not rename it: renaming would needlessly break continuity and the existing hash reference. Add one rule:

- `vf06.function-signature-fact` — the `FunctionDefinition` `SemRecord` must carry the committed M1 `int(void)` signature `TypeId`, `NonLvalue`, and `EffectMask(0)`.

#### 3.3.6 Conversion completeness

B2-01 does **not** add a conversion field or a conversion matrix. VF06's M1 conversion obligation remains:

- identity/no-conversion is accepted;
- no conversion is required for the M1 `int`/`int` fixture;
- a missing required conversion is a future non-M1 fixture (T13 item 3(c) has no M1-producible instance);
- the function-definition record has no `Return`-role plan.

This is a deliberate minimal-scope boundary, not an omission.

### 3.4 T09 IR consumption change

#### 3.4.1 Kind, payload, result (unchanged)

| Item | Current `/39` | Proposed `/40` |
|---|---|---|
| Task kind | `IR_FUNCTION` (IR_LOWER local 16) | Same |
| Chip | `IR_FUNCTION_CHIP = ChipId(16)` | Same |
| Stage / layer | 5 / 5 | Same |
| Payload | `Payload::from_refs(vec![RecordRef::Node(funcdef)])` | Same |
| Result | `ResultValue::Record(RecordRef::Function(function))` | Same |
| Writes | `Ir/functions`, `Ir/blocks`, `Ir/values`, `Ir/instructions` | Same |

#### 3.4.2 Input shape

Current `IrFunctionInput` (`compiler/src/chips/ir_lower/ir_function.rs:27-52`):

```rust
pub struct IrFunctionInput {
    pub task: TaskId,
    pub state: TaskState,
    pub symbol: SymbolId,
    pub linkage: Linkage,
    pub signature: TypeId,
    pub int: TypeId,
    pub constant: ConstId,
    pub functions_allocated: u32,
    pub blocks_allocated: u32,
    pub values_allocated: u32,
    pub instructions_allocated: u32,
}
```

Proposed `/40` adds one traceability field:

```rust
pub struct IrFunctionInput {
    pub task: TaskId,
    pub state: TaskState,
    pub symbol: SymbolId,
    pub linkage: Linkage,
    /// Signature consumed from the committed FunctionDefinition SemRecord.
    pub signature: TypeId,
    /// The committed FunctionDefinition fact that supplied `signature`.
    pub function_sem: SemId,
    pub int: TypeId,
    pub constant: ConstId,
    pub functions_allocated: u32,
    pub blocks_allocated: u32,
    pub values_allocated: u32,
    pub instructions_allocated: u32,
}
```

`function_sem` is a projection traceability field, not a new bus record field. If the integrator prefers no unused input field, it can be dropped; the `signature` sourcing and the negative tests are the substantive change.

#### 3.4.3 Projection check order

The projection must become:

1. task kind/state and payload (exactly one `Node`);
2. `FunctionDefinition` node shape, declarator, compound, single `Return`, single `BinaryAdd` (as today);
3. canonical `int` (as today);
4. **find the committed `SemRecord` with `sem.node == funcdef`**:
   - zero → `Task` group missing input: `"lower reads unchecked function definition"`; never fall back to `symbol.ty`;
   - more than one → `Task` group DEFECT: duplicate function-definition fact;
   - validate `sem.category == NonLvalue` and `sem.effects == EffectMask(0)`; mismatch → verification/protocol failure, never silently accepted;
5. take `signature = sem.ty`; fetch the `TypeRecord` and require the M1 shape `Function { result: int, params: [], prototype: true, variadic: false }`; a missing type or non-M1 shape is `Unsupported`;
6. find exactly one symbol with `sym.decl == declarator` (as today); require `symbol.ty == Some(signature)`:
   - `None` → `Task` group untyped symbol;
   - `Some(other)` → `Unsupported` definition/prototype mismatch; never use `other` as the signature;
7. check the `Return` and `BinaryAdd` `SemRecord`s (as today);
8. require exactly one committed `ConstRecord` (as today);
9. compute and append the IR records.

The check order is part of the behavior: with a fresh bus that has no `SemRecord`s, the missing function-definition fact now fails **before** the missing return fact. Existing negative tests that expect a specific failure message must be updated to the new precedence; they must not be weakened to "any failure" if they were specifically asserting the return-fact path.

#### 3.4.4 Symbol link versus semantic fact

The `SemRecord` does not carry a `SymbolId`. T09 still needs the `SymbolId` for `FunctionRecord.symbol` and the linkage. The proposed split is:

- **semantic signature**: from the committed `FunctionDefinition` `SemRecord.ty`;
- **structural symbol link**: `SymbolRecord.decl == Declarator` (already committed by TY07);
- **cross-check**: `SymbolRecord.ty == Some(sem.ty)`; mismatch is a typed failure;
- **never**: use `SymbolRecord.ty` as the source when the fact is missing.

This satisfies A3's "T09 reads those committed `SemRecord`s without re-inferring" without adding `SymbolId` to `SemRecord` or a new payload field. The `FunctionRecord` is created by this T09 task; the fact comes from the T07 `SemRecord`, not from a pre-existing `FunctionRecord`, so there is no T07→T09→T07 temporal cycle. In the valid M1 case `symbol.ty == sem.ty`, so the emitted `FunctionRecord.signature` is the same `TypeId` as before; the IR records themselves are unchanged. The T07 S7 open item "explicit committed typed link" is resolved for M1 as the node-keyed committed link `SemRecord.node == FunctionDefinition NodeId`; the alternative `RecordRef::Sem` in the payload is rejected for this CCR as non-minimal (open question O-3).

#### 3.4.5 Constant typing (interim, D-09)

`ConstRecord` has no `ty`. B2-01 does **not** add one and does **not** check `ConstRecord.ty == ValueRecord.ty`.

To keep the IR type rule checkable at `/40`, add an interim rule:

- `ir.constant-result-type-from-sem` — the IR `Constant` result `ValueRecord.ty` must equal the committed `SemRecord.ty` of the expression that produced the folded constant (M1: the `BinaryAdd` record, canonical `int`); the `Return` operand `ValueRecord.ty` must equal the committed `Return` `SemRecord.ty` (M1: canonical `int`).

This is weaker than the documented immediate-type rule and must be described as such. When B2-02 adds `ConstRecord.ty`, the immediate-type rule `ir.op-immediate-type` (`ConstRecord.ty == ValueRecord.ty`) can be added, and the interim rule can be retired or kept as an additional consistency check.

#### 3.4.6 Rule IDs

Keep `ir.constant-no-refold`, `ir.return-single-terminator`, and `ir.function-single-entry`. Add:

- `ir.function-consumes-committed-sem` — T09 requires exactly one committed `FunctionDefinition` `SemRecord`, takes the signature from it, and cross-checks the structural symbol link; no `symbol.ty` fallback.
- `ir.constant-result-type-from-sem` — the interim constant/return value type rule in §3.4.5.

Do **not** add `ir.op-immediate-type` to the frozen rules until B2-02 lands.

### 3.5 `ConstRecord.ty` decision (D-09)

**Decision: separate tagged-constant CCR (proposed B2-02), not part of B2-01.**

Why:

- A14 §4.2 proposes replacing `ConstRecord { value, negative }` with a tagged family `ConstRecord { ty: TypeId, value: ConstValue }`, where `ConstValue` has closed integer/float/address payloads. This is a schema change with a much larger blast radius than A3: `FoldChip`, `commit.rs` materialization, `snapshot.rs` encoder/decoder, `CONST_RECORD_FIELDS`, the contract hash, `c08_gate1`, `c15_ir`, `c37_const_branch`, and future Part B target values.
- Adding only `ty: TypeId` now would be a half-step that B2-02 would immediately replace; it would also force every `ConstRecord` constructor and snapshot test to change for a rule that A3 does not need.
- D-02 (A3) and D-09 (constant typing) are independent findings in the ledger. A3 can close without D-09.

Dependency statement:

- **B2-01 depends on B2-02 for nothing.** It can freeze and be implemented first.
- **M1 Part A IR-4(b) depends on B2-02.** The documented immediate-type rule in `M1_TARGET_ACCEPTANCE.md` §3.2 IR-4(b) and the op-table note cannot be enforced or claimed at `/40`. `/40` must mark it as an explicit deferred gap.
- If the integrator requires a single M1 Part A closure freeze, B2-01 and B2-02 must be sequenced in one freeze train; B2-01's rule IDs, VF06/T09 changes, and tests remain valid under B2-02.
- Neither CCR may substitute `SemRecord.ty` or `SymbolRecord.ty` for `ConstRecord.ty` and claim the documented immediate-type rule.

Proposed B2-02 contents (for dependency clarity only; not specified here):

- `ConstRecord { ty: TypeId, value: ConstValue }` with a closed `ConstValue` inventory;
- `ConstRecord.ty` must name a committed T06 type compatible with the payload;
- one `ConstRecord` per evaluation, no interning in M1;
- update `FoldChip`, commit materialization, snapshot encode/decode, contract field list/hash, and dependent tests;
- add `ir.op-immediate-type` to the frozen rules when the field lands.

### 3.6 Task/payload/result summary

| Task | Kind | Chip | Stage | Payload | Result | Writes |
|---|---|---:|---:|---|---|---|
| SE29 | `SEMANTIC_FUNCTION_DEF` | 57 | 4 | `[Node(funcdef)]` | `Record(Sem)` | one `Sem/records` append when absent |
| VF06 | `VERIFICATION_TYPED_INVARIANT` | 15 | 4 | `[Node(tu)]` | `Ack` / `Fail` | none |
| T09 | `IR_FUNCTION` | 16 | 5 | `[Node(funcdef)]` | `Record(Function)` | `Ir/functions`, `Ir/blocks`, `Ir/values`, `Ir/instructions` |
| Fold (unchanged) | `CONSTANT_CONST_FOLD` | `/7` fold chip | 2 | literal/binary as today | `Record(Const)` | one `Constants/records` append |

No new kind/chip/stage/payload variant/result variant is introduced.

---

## 4. Affected Chips, Manifests, Candidate, Tests, and Documentation

### 4.1 Chips

| Chip / file | Change | Contract shape |
|---|---|---|
| `SeFuncChip` / `compiler/src/chips/semantic/se_function.rs` | **No logic change** required for D-02. Newly dispatched by the candidate. Optional reuse revalidation (O-7). | Unchanged `/39` |
| `Vf06Chip` / `compiler/src/chips/verify/vf_invariant.rs` | Add `signature` to the projection; replace the three-kind blanket rule with the per-kind table; add the exact-one-function-definition rule. | Chip-local projection shape; no bus record change |
| `IrFunctionChip` / `compiler/src/chips/ir_lower/ir_function.rs` | Require and consume the `FunctionDefinition` `SemRecord`; source `signature` from it; cross-check `symbol.ty`; add `function_sem` traceability; interim constant type rule. | Chip-local projection shape; no bus record change |
| `FoldChip` / `compiler/src/chips/constant_layout_init/fold.rs` | No change. Still one `ConstRecord` per evaluation; no refold. | Unchanged |
| `Vf01Chip` / `Vf12Chip` | No logic change. VF01 validates the extra `SemRecord`'s node reference as a normal committed record; VF12 reads the IR function whose signature is the same `TypeId`. | Unchanged |
| PP/LX/PA/TY chips | No change. | Unchanged |

### 4.2 Manifests and registrations

| Item | Current | Proposed | Change |
|---|---|---|---|
| `SE_FUNC_CHIP` manifest | Reads Tasks/Parse/Symbols/Types/Sem; writes Sem/records | Same | None |
| `VF06_CHIP` manifest | Reads Tasks/Parse/Types/Sem; writes none | Same fields; `signature` is still `Types/records` | None |
| `IR_FUNCTION_CHIP` manifest | Reads Tasks/Parse/Symbols/Types/Sem/Constants/Ir; writes Ir | Same fields; function fact is `Sem/records` | None |
| `STORE_OWNER_ALLOWLIST` | 33 rows, includes `(SE_FUNC_CHIP, Sem, records, SEMANTIC_FUNCTION_DEF)` | Same 33 rows | None |
| `STAGE_ASSIGNMENT` | Includes `(SEMANTIC_FUNCTION_DEF, 4)`, `(VERIFICATION_TYPED_INVARIANT, 4)`, `(IR_FUNCTION, 5)` | Same | None |
| `TaskKindRegistry` entries | `SEMANTIC_FUNCTION_DEF` already frozen `/39` | Same entry | None |
| Candidate registry | `pp_emit_slice()` | `se_function_slice()` | Host selection only; no contract registration |

No new manifest row, allowlist row, stage row, chip ID, or store field is required. This is a deliberate minimality property: SE29 was frozen at `/39` but not wired; B2-01 wires it and fixes its two consumers. The only possible manifest-metadata edit is adding `compiler/tests/c43_a3.rs` to the `tests` vector of VF06/T09/SE29; that vector is runtime metadata, excluded from the frozen hash, and is not a read/write/task-kind/stage/allowlist change.

### 4.3 Candidate

`compiler/src/bin/candidate.rs` only:

- import `SeFuncChip` and `SE_FUNC_CHIP`;
- registry switch, chip registration, route, worker registration;
- SE29 step before VF06;
- reuse the single `funcdef` for IR;
- update the evidence version string after the freeze to `/40`;
- no `-E`, `-D`, `-U`, `-I`, `-S`, `-c`, or multi-source change.

### 4.4 Tests

#### Updated tests

| Test | Required update | Why |
|---|---|---|
| `compiler/tests/c14_se.rs` | `install_se` must use `se_function_slice()` and register `SeFuncChip` + route (or seed the function fact directly); extend `vf06_passes_on_complete_facts_and_fails_on_gaps` with the function-definition fact and negatives. `base_chain()` already creates the `int` and `int(void)` types but does not declare the `main` symbol; running the real SE29 there also requires the TY07 declare step (as c15/c39 do), while seeding the fact is the smaller unit-test change. | VF06 now checks four kinds. |
| `compiler/tests/c15_ir.rs` | `install_ir` must use `se_function_slice()` and register `SeFuncChip` + route; `full_chain()` must run SE29 before IR; update the M1 test to assert the signature source; add no-fallback negatives; update failure precedence in existing negative tests. | T09 now requires the function fact. |
| `compiler/tests/c39_sefunc.rs` | Expected green unchanged. It already installs `se_function_slice()`, registers `SeFuncChip`, and checks the appended signature fact. Do not duplicate the c43 integration here. | SE29 logic is unchanged. |
| `compiler/tests/h04_candidate.rs` | Update the version assertion to `/40`; keep the `value=05` and determinism assertions. | Candidate evidence string changes. |
| `compiler/tests/freeze.rs` | Update the version assertion and hash; add the new rule IDs to the rule-presence list. | Contract version/hash bump. |
| `compiler/tests/c08_gate1.rs` | Update version assertions/markers; add the new rule IDs to the presence list. | Contract version/hash bump. |
| `compiler/tests/c20_ppscan.rs` | Update the version string. | Contract version bump. |
| `compiler/tests/c04_manifest.rs` | Verify the allowlist count remains 33; no edit expected. | No new allowlist row. |

#### New integration test (recommended)

`compiler/tests/c43_a3.rs` — the A3 integration gate. It should use the same seeded PP layer as `c15_ir`/`c39_sefunc` (not the full candidate) and run:

```text
LX → PA → TY (int, int(void), scope, declare)
   → SE literal ×2 → SE binary (+fold) → SE return
   → SE29 function
   → VF06
   → IR_FUNCTION
```

Assertions:

1. Before SE29: exactly 4 committed `SemRecord`s; no `SemRecord` for the `FunctionDefinition` node.
2. After SE29: exactly 5 `SemRecord`s; exactly one for `funcdef`; `ty == func`; `category == NonLvalue`; `effects == EffectMask(0)`.
3. VF06 completes `Ack` with the function fact present.
4. `FunctionRecord.signature == func` (the same `TypeId` as the `SemRecord.ty`); `FunctionRecord.symbol` is the declared `main`; linkage `External`; `Constant.immediate` is the one folded `ConstId`; no refold.
5. `Snapshot::capture` twice on the same deterministic chain yields identical bytes/hash.
6. Negative: omit SE29 → VF06 fails (missing function fact) and IR fails (unchecked function definition); no snapshot/trace evidence path.
7. Negative: commit a `FunctionDefinition` `SemRecord` with `ty = int` → VF06 fails wrong signature; IR fails non-M1 signature.
8. Negative: duplicate `FunctionDefinition` `SemRecord` → VF06 and IR fail duplicate fact.
9. Negative: wrong `category`/`effects` on the function fact → VF06 fails.

If the integrator prefers not to add a new test file, these assertions can be distributed across `c14_se` (VF06) and `c15_ir` (T09) plus one `c39_sefunc` integration case. A dedicated `c43` is recommended because it names the A3 property in one place.

### 4.5 Documentation

| Document | Proposed `/40` amendment | Constraint |
|---|---|---|
| `docs/tasks/T01_COMPILER_CONTRACT.md` §7.1 | Add a `/40` amendment paragraph: SE29 candidate wiring, VF06 four-kind table, T09 consumption, new rule IDs, D-09 deferral. Keep `/39` history. | Docs-only; no shape rewrite. |
| `docs/tasks/T13_VERIFICATION_CHIPS.md` | Correct the VF06 registration drift (it is frozen at `/14`: `verification.typed_invariant`, stage 4, chip 15); update the M1 checked set to four kinds; keep conversion completeness M1-minimal; add the c14 + c43 fixture note. | Do not claim a broader VF06 matrix. |
| `docs/tasks/M1_TARGET_ACCEPTANCE.md` §3.1 A3 | Mark the `FunctionDefinition` fact path closed by `/40` (pending implementation evidence); keep the exact `SemRecord` field encoding frozen. | Do not mark all of Part A closed. |
| `docs/tasks/M1_TARGET_ACCEPTANCE.md` §3.2 IR-4(b) | Explicitly mark `ConstRecord.ty`/`ir.op-immediate-type` as deferred to B2-02; do not claim the immediate-type rule at `/40`. | D-09 remains open. |
| `docs/tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md` VF06 gap note | Replace "registration and fixtures pending" with "registration frozen `/14`; checked set extended `/40`; fixtures c14/c43; D-09 remains." | Keep the recorded-gap discipline for VF07–VF11. |
| `docs/tasks/SE_FUNC_SLICE.md` | Add a post-freeze `/40` wiring note/banner: `/39` chip logic unchanged, candidate wiring added, VF06/T09 consumers added. | Never rewrite the frozen `/39` execution record. |
| `docs/tasks/T07_SEMANTIC_CHIPS.md` §3 and S2/S6 checklist | Confirm the function signature fact and the node-keyed M1 link; update the VF06 registration note to `/14` + `/40`. | D-09 and conversion-plan opens remain. |
| `compiler/README.md`, `docs/tasks/README.md` | Status line only after code lands; not required for the CCR. | Do not overclaim. |

Do not edit the read-only audit reports (`A04`, `A05`, `A17`) to make them agree; they are historical evidence. The CCR references them.

### 4.6 Files explicitly not changed

`compiler/src/bus.rs`, `compiler/src/ids.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/commit.rs`, `compiler/src/snapshot.rs`, `compiler/src/records.rs`, all `mod.rs` files, and `Cargo.toml`/lockfiles require **no change** for B2-01. If an implementation attempt needs to edit any of these, the change is outside the minimal A3 closure and must be surfaced to the integrator before proceeding.

---

## 5. Snapshot, Hash, and Version Impact

### 5.1 Version and hash

| Item | Current `/39` | Proposed `/40` |
|---|---|---|
| `CONTRACT_VERSION` | `t01-c01-c06/39` | `t01-c01-c06/40` |
| `CONTRACT_HASH` | `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` | Recomputed by the integrator |
| `CONTRACT_VERSION` file | `/39` + current hash | `/40` + recomputed hash |
| `/39` artifact | Current | Preserved as history; never deleted or rewritten |

The hash changes for two reasons: the version string is encoded in `FrozenSchema`, and the new normative rule IDs are encoded in `NORMATIVE_RULES`. The hash does **not** cover chip logic; the behavioral change is evidenced by tests, not by the hash.

### 5.2 Shape/registry impact

| Item | Impact |
|---|---|
| `SemRecord` | **Unchanged** `{node, ty, category, effects}`; no new field; `SEM_RECORD_FIELDS` unchanged. |
| `ConstRecord` | **Unchanged** `{value, negative}`; no `ty`; `CONST_RECORD_FIELDS` unchanged. |
| `RECORD_KINDS` / `RecordFamily` | Unchanged (28 families). |
| `RecordRef` wire tags | Unchanged (0–27; no new tag). |
| `StoreId` | Unchanged (21 stores). |
| New store/field | None. |
| `ResultValue` | Unchanged five variants. |
| `Proposal` | Unchanged. |
| `TaskKind` | No new kind; `SEMANTIC_FUNCTION_DEF` already frozen `/39`. |
| Chip ID | No new ID; `SE_FUNC_CHIP=57` already frozen. |
| Stage/allowlist | No new row; allowlist remains 33. |
| `M1_SEED_VERSION` | Unchanged `m1-append/1`. |
| `M1_STORE_FAMILY_ARENA` | Unchanged. |
| `snapshot.rs` encoders | No new encoder/decoder; `encode_sem`/`encode_const` unchanged. |
| `commit.rs` materialization | No new family; Sem/Const append paths unchanged. |

### 5.3 Runtime snapshot/trace impact

- The M1 fixture's committed `sem` arena grows from **4** records (two literals, one binary, one return) to **5** (plus the function-definition record).
- The candidate's snapshot bytes and snapshot hash change because the sem arena body count changes.
- The trace changes because one more task is dispatched (SE29) before VF06.
- No candidate test pins the snapshot/trace hash values; `h04_candidate` only asserts presence and run-to-run determinism. The `/39` observed values in A05 (`snapshot 833659dd…`, `trace 8b466fad…`) are **history**, not `/40` acceptance values.
- If the integrator wants a frozen M1 evidence golden, capture it **after** `/40` and record the exact source bytes, config, limits, and command. Do not compare `/39` and `/40` snapshots byte-for-byte; the extra record is expected.
- The canonical semantic comparison projection (when VF13 lands) must include the function-definition `SemRecord`; it is a semantic record, not a scheduler-only register.

### 5.4 New normative rule IDs (proposed)

Append to `NORMATIVE_RULES` after the existing `se.function-body-checked` entry:

```text
vf06.function-signature-fact
ir.function-consumes-committed-sem
ir.constant-result-type-from-sem
```

Notes:

- Keep `vf06.checked-set-complete` and extend its meaning; do not rename it.
- Do not add `ir.op-immediate-type` until B2-02.
- If the integrator wants the D-09 deferral itself to be hash-visible, an optional marker rule `ir.constant-immediate-type-deferred-d09` may be added and removed by B2-02 (open question O-4). The recommended default is to document the deferral in T01/M1 docs and not encode a "deferred" marker as a normative rule.
- Rule names are proposals; the integrator owns the final spelling. Any rename must update the CCR, `contract.rs`, `freeze.rs`, and `c08_gate1.rs` together.

---

## 6. Migration and Freeze Steps

This CCR is input to the T01 integrator. The integrator owns the version/hash and shared registrations; chip owners edit only their assigned chip/test files. No implementation starts before approval.

### 6.1 Ordered steps

| Step | Owner | Work | Gate |
|---:|---|---|---|
| 0 | T01 integrator | Review/accept B2-01; assign `/40`; confirm rule names and the D-09 split. | CCR accepted |
| 1 | Integrator + T13/T09/H04 owners | Docs-only contract text: T01 §7.1 `/40`, T13 VF06 row/checklist, M1 acceptance A3/IR-4(b), M1 vertical-slice VF06 note, SE slice post-freeze note, T07 note. | Links/IDs consistent; no `/39` history rewrite |
| 2 | T13 owner | VF06 projection/per-kind table + c14 tests. | c14 green |
| 3 | T09 owner | T09 function-fact consumption + c15 tests. | c15 green |
| 4 | H04/integrator | Candidate registry/chip/route/step + h04 tests. | h04 green; direct candidate run |
| 5 | Integrator/T13 | New c43 A3 integration test. | c43 green |
| 6 | Integrator | Add new rule IDs; set `/40`; recompute hash; update `CONTRACT_VERSION`; update freeze/c08/c20/h04 version pins. | freeze + targeted suite green |
| 7 | Integrator | Full compiler suite, root tests, fmt, clippy (or accurately record the dirty-tree blocker), 3-run candidate determinism. | Evidence recorded |
| 8 | Integrator | Freeze `/40`; update status docs and ledger D-02 only with evidence; keep D-09 open. | No overclaim |

### 6.2 Freeze mechanics

1. Add the three rule IDs to `NORMATIVE_RULES` in `compiler/src/contract.rs`.
2. Keep the frozen shapes unchanged; do not edit `SEM_RECORD_FIELDS`, `CONST_RECORD_FIELDS`, `STAGE_ASSIGNMENT`, or `STORE_OWNER_ALLOWLIST`.
3. Set `CONTRACT_VERSION = "t01-c01-c06/40"` and update the header comment only if the integrator wants the `/40` delta named there. If the header changes, update `contract_version_file()` so the `freeze.rs` whole-file equality remains true.
4. Run:

   ```sh
   cargo run --manifest-path compiler/Cargo.toml --example freeze_hash
   ```

   Copy the printed hash into `CONTRACT_HASH` and `compiler/contracts/CONTRACT_VERSION`.
5. Run `cargo test --manifest-path compiler/Cargo.toml --test freeze`; then the targeted suite in §7.4.
6. Update `compiler/tests/freeze.rs`, `c08_gate1.rs`, `c20_ppscan.rs`, and `h04_candidate.rs` version pins in the same change set.
7. Do not commit or push unless the user explicitly authorizes it. The CCR itself authorizes nothing.

### 6.3 Rollback

- Revert the code/tests/docs to `/39`; the append-only arenas require no data migration.
- `/39` remains a valid historical artifact; SE29 remains frozen but unwired in the reverted candidate.
- Snapshot/trace hashes produced under `/40` are not comparable to `/39`; do not carry `/40` golden values into a `/39` tree.
- If the freeze is partially applied, the freeze test fails closed on version/hash; do not "fix" it by weakening the assertion.

### 6.4 Freeze gate

`/40` is not freeze-ready until:

- c14, c15, c39, c43, h04, freeze, c08, and c20 are green;
- the full compiler suite is green;
- formatting is green;
- clippy either is green or its dirty-tree failure is explicitly attributed to the unregistered user drafts (D-04/B-INT-00) and **not** claimed as a B2-01 regression;
- the direct candidate run emits `/40`, `value=05`, and deterministic snapshot/trace values across three runs;
- no test, probe, or corpus claim is made beyond what was actually run.

---

## 7. Test Plan and Acceptance Evidence

No test in this section was run for this CCR. The case list is the required post-freeze plan.

### 7.1 Unit cases

| ID | Chip | Case | Expected |
|---|---|---|---|
| A3-U01 | VF06 | Full M1 facts: 2 literals, binary, return, function definition | `Ack` |
| A3-U02 | VF06 | Function-definition fact missing | `Fail` (Task: missing typed fact) |
| A3-U03 | VF06 | Duplicate function-definition facts | `Fail` (Task: duplicate typed fact) |
| A3-U04 | VF06 | Function-definition fact `ty = canonical int` | `Fail` (wrong signature type) |
| A3-U05 | VF06 | Function-definition fact wrong category or nonzero effects | `Fail` |
| A3-U06 | VF06 | Zero or multiple `FunctionDefinition` nodes in the M1 TU | `Fail` (no vacuous pass) |
| A3-U07 | T09 | Valid function fact present | IR `Function.signature == SemRecord.ty`; `function_sem` points to it |
| A3-U08 | T09 | Function fact missing, symbol present | `Fail` (Task: unchecked function definition); **no** `symbol.ty` fallback |
| A3-U09 | T09 | Function fact `ty = int` / non-M1 function type | `Fail` (`Unsupported`) |
| A3-U10 | T09 | `symbol.ty` differs from the fact signature | `Fail` (`Unsupported` mismatch) |
| A3-U11 | T09 | Duplicate function facts | `Fail` (Task DEFECT) |
| A3-U12 | T09 | Fresh bus with no SemRecords | Fails on the missing function fact **before** the missing return fact (deterministic precedence) |
| A3-U13 | SE29 | Existing fact reused only when it matches; optional hardening | Reuse valid; wrong existing fact fails (if O-7 accepted) |
| A3-U14 | VF06 | Non-M1 signature shape in `Types` (e.g. parameters present) | `Fail`/`Unsupported`; no silent pass |

### 7.2 Integration cases

| ID | Harness | Case | Expected |
|---|---|---|---|
| A3-I01 | c43 | Full seeded chain through SE29 → VF06 → IR | 5 SemRecords; one function fact; VF06 `Ack`; `FunctionRecord.signature == func`; one folded `ConstId`; no refold |
| A3-I02 | c43 | Same chain without SE29 | VF06 and IR fail closed; no IR function |
| A3-I03 | c43 | Wrong function fact type / duplicate / wrong category | VF06 and IR fail; no fabricated function |
| A3-I04 | h04 | Candidate on `int main(void){return 2+3;}` | `/40` evidence header; `interpret: value=05 negative=false`; snapshot/trace present |
| A3-I05 | h04 | Two candidate runs | Byte-identical stdout/evidence |
| A3-I06 | c43 | Snapshot capture twice on the same chain | Identical snapshot bytes/hash |
| A3-I07 | h04 | Three repeated candidate runs | Identical snapshot/trace values (D1/D2-style determinism for Part A) |

### 7.3 Boundary, unsupported, replay, and write-scope

| Class | Cases |
|---|---|
| Boundary | Exactly one function definition; exactly 4→5 SemRecords; zero/duplicate function facts; duplicate symbol/type (existing SE29 cases). |
| Unsupported | K&R list, prototype mismatch, non-`int(void)` signature, multi-statement body, non-return body (existing SE29 cases; unchanged). |
| Replay | c43 snapshot double-capture; h04 repeated runs; no hash-map or pointer-derived order. |
| Write-scope | VF06 writes nothing; T09 writes only the four Ir fields; SE29 writes only Sem/records; all manifests/allowed fields unchanged; `STORE_OWNER_ALLOWLIST` remains 33 rows. |
| Negative mutation | Delete/alter the function fact in a fixture bus; both VF06 and T09 must fail. A verifier that still passes after the fact is removed is defective. |

### 7.4 Commands (planned; not run)

```sh
cargo test --manifest-path compiler/Cargo.toml \
  --test c14_se --test c15_ir --test c39_sefunc --test c43_a3 \
  --test h04_candidate --test freeze --test c08_gate1 --test c20_ppscan

cargo test --manifest-path compiler/Cargo.toml
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --manifest-path compiler/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path compiler/Cargo.toml --example freeze_hash

# Direct Part A evidence (repeat three times; compare snapshot/trace)
FIXTURE=/path/to/main.c
compiler/target/debug/candidate "$FIXTURE"
```

Known environment caveat: the current dirty workspace fails `clippy -D warnings` because unregistered user parse drafts are compiled through `parse/mod.rs` (A04 §1.2, A05 §2.4, ledger D-04). B2-01 must report that blocker accurately; it must not claim a clean clippy gate until the integrator resolves the draft disposition (B-INT-00). The compiler test suite baseline observed in this dispatch is 654 tests across 44 suites (ledger §1); B2-01 adds cases, so the post-freeze count must be re-recorded, not assumed.

### 7.5 Mapping to M1 acceptance

| M1 requirement | B2-01 effect |
|---|---|
| A1: exactly one TU and one function definition `main` | Candidate still requires exactly one; VF06 adds a fail-closed check; VF05 remains the AST authority. |
| A2: `main` has type `int (void)` | SE29/TY17/IR signature unchanged; VF06 verifies the fact's signature. |
| A3: every checked node, including `FunctionDefinition`, has exactly one committed `SemRecord` carrying the signature; T09 consumes without re-inferring | **Proposed closure by B2-01** (pending implementation evidence): candidate SE29 dispatch + VF06 four-kind table + T09 function-fact consumption. |
| A4: `2`, `3`, `2+3` are `int`, no implicit conversion | Unchanged; VF06/T09 still check `int`/`NonLvalue`/`0`. |
| A5: tokens/spans ordered, one EOF | Unchanged (VF05). |
| IR-1..IR-3, IR-5..IR-9 | Unchanged except the signature source and the interim constant type rule. |
| IR-4(b): `ConstRecord.ty == ValueRecord.ty` | **Not closed**; D-09/B2-02 dependency. Do not claim. |
| IR-4(c): effects `0` | Unchanged; VF06 checks the function fact too. |
| Part B target rows | Unaffected; target remains unverified. |

---

## 8. Non-Goals: Minimal M1 Part A vs V1

B2-01 is deliberately **not** a V1 semantic contract. The following are out of scope and must not be smuggled in as "while we are here" changes:

1. **No general function definitions.** SE29 remains the M1 `int main(void)` shape: zero parameters, `(void)` prototype, single `return` body. K&R, parameters, variadics, multi-statement bodies, prototypes/incomplete types, and old-style definitions stay explicit `Unsupported` or future work.
2. **No V1 AST/parser changes.** No `Identifier`, `Declaration`, `Assignment`, `Initializer`, `ExpressionStatement`, `Call`, `If`, `Loop`, etc. No PA01 general external-declaration loop. D-01/B1-01 remains a separate exact-shape guard.
3. **No conversion plan.** No `ConversionPlan`, `ConversionOp`, `ConversionRole`, `(op, role)` matrix, or `SemRecord.conversions` field. M1 identity is still "no plan". V1 conversions require a separate schema freeze (A04 V1-5).
4. **No tagged constants.** No `ConstRecord.ty`, `ConstValue`, float/address constants, or `ir.op-immediate-type`. B2-02 owns this.
5. **No verification-record family.** A17's `Verification`/`Fact`/`Result`/`Evidence` family is not adopted. VF06 remains `Ack`/`Fail`, zero-write. If the integrator later adopts A17, VF06 can append facts in a separate CCR; B2-01 does not pre-empt that schema.
6. **No VF07–VF11/VF13, CFG/SSA/machine/ABI/replay chips.** They remain out of M1 or required-pending as recorded in T13/M1 docs.
7. **No target codegen, assembly, ELF, linker, runner, probe, or ABI claim.** Part B remains fail-closed and probe-gated.
8. **No candidate `-I`/multi-source/source-encoding work.** H04 Part A scope only.
9. **No new diagnostics taxonomy.** Reuse the existing `Task`/`Unsupported` patterns; no `Verification` diagnostic group.
10. **No function-context ID.** `CheckRequest` remains `(NodeId, ScopeId)`; T07 rev 1 stands.

The minimal M1 closure is exactly the three facts in §1; everything else is a separate contract decision.

---

## 9. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| VF06's signature scan diverges from SE29's signature check | A valid M1 definition could fail VF06, or an invalid one could pass | Both use the same frozen `int(void)` shape; add c43 positive/negative; do not duplicate a second variant list. |
| T09 check-order change breaks existing negative tests | Tests may assert old failure messages | Update tests to the new precedence; keep the strongest assertion that fails if the function fact is missing. |
| Candidate registry switch to `se_function_slice()` pulls in later kinds | No runtime effect unless a task is bootstrapped, but a future accidental bootstrap could activate an unregistered chip | Candidate only bootstraps its explicit steps; add a wiring/test assertion that no unexpected task kinds appear in the trace. |
| Extra SemRecord changes snapshot/trace values | `/39` goldens or external comparisons become invalid | Record `/40` values only after freeze; state that cross-version hashes are not comparable. |
| `FunctionDefinition` `NonLvalue` category is semantically debatable | A future change to `FunctionDesignator` would ripple into VF06/T09 | Keep the frozen `/39` choice; record O-2; any change is a separate SE29 contract change. |
| D-09 remains open | M1 Part A IR-4(b) cannot be claimed | Explicit deferral in T01/M1 docs; B2-02 dependency; do not substitute a different type field. |
| Dirty-tree clippy failure | B2-01 cannot claim a clean gate | Attribute to D-04/B-INT-00; run B2-01-scoped clippy/tests; report the blocker. |
| Candidate exactly-one function definition is a driver error | Future V1 needs a real loop and typed diagnostic | Keep M1 exactly-one; record as V1 work, not a B2-01 defect. |
| `function_sem` traceability field is unused in compute | Possible dead-code/style debate | It is a public projection field; drop it if the integrator prefers. The rule and negative tests are the substantive evidence. |
| Concurrent B1-02 edit to `compiler/src/bin/candidate.rs` | Merge conflict or overwriting the argv-hardening user work | Integrator sequences B1-02 before B2-01 or reserves the file; B2-01 rebases and never overwrites B1-02 changes. |
| Concurrent B1-01 edit to `pa_tu.rs` | Candidate negative tests may change behaviour while B2-01 is in flight | B2-01 does not modify PA01; run B1-01's c40/h04 gates separately; do not use B2-01 to mask D-01. |

---

## 10. Open Questions and Requested Decisions

| # | Question | Recommended default |
|---|---|---|
| O-1 | Is D-09 handled in this CCR or a separate tagged-constant CCR? | **Separate B2-02.** B2-01 closes A3 only; IR-4(b) stays deferred. |
| O-2 | Should the `FunctionDefinition` fact's `ValueCategory` be `NonLvalue` or `FunctionDesignator`? | Keep the frozen `/39` `NonLvalue`; changing it is a separate SE29 semantics change. |
| O-3 | Is the M1 T09 link node-keyed (`SemRecord.node == funcdef`) or an explicit `RecordRef::Sem` in the payload / `SymbolId` in `SemRecord`? | **Node-keyed lookup.** It is already committed, one-per-node, and needs no payload/record change. |
| O-4 | Should the D-09 deferral be a hash-visible rule marker? | Document it in T01/M1 docs; do not add a "deferred" normative rule unless the integrator wants the gap in the hash. |
| O-5 | Must VF06 require exactly one `FunctionDefinition` node, or rely on VF05? | Require exactly one in M1; fail closed rather than pass vacuously. Relax only with an explicit VF05-ordering argument. |
| O-6 | Should T09 re-check the function fact's category/effects, or trust VF06? | Re-check cheaply; T09 must be safe even if the verifier is not in the pipeline. |
| O-7 | Should SE29 validate a reused existing fact? | Recommended optional hardening; not a D-02 blocker because VF06/T09 fail closed. |
| O-8 | Should the candidate use `se_function_slice()` or a dedicated minimal `candidate_m1_slice()`? | Use `se_function_slice()`; it is the existing cumulative `/39` registry and adds no runtime registration. |
| O-9 | Should zero/multiple `FunctionDefinition` nodes be a driver `Err` or a typed candidate diagnostic? | Keep the M1 driver error for now; V1 needs a typed diagnostic and a real external-declaration loop. |
| O-10 | Should a canonical M1 Part A semantic projection/snapshot hash be frozen as evidence? | Capture after `/40` if required; do not reuse `/39` hashes. |
| O-11 | If A17's `Verification` family is adopted later, does VF06 append facts? | Separate CCR; B2-01 keeps VF06 zero-write. |
| O-12 | Should `ir.constant-result-type-from-sem` be added now or left implicit until B2-02? | Add it now as the interim checkable rule; otherwise the IR type rule is weaker than documented and the D-09 gap is less visible. |

---

## 11. Approval and Co-Freeze Record

This CCR was **SUBMITTED 2026-11-07** and **ACCEPTED 2026-11-09** by the T01 integrator with freeze target `t01-c01-c06/40`. The acceptance adopts the decision record and the open-question defaults below; it is **not** implementation or test evidence. The integrator owns the contract/version/hash freeze; the chip owners own their files.

| Role | Scope | Status |
|---|---|---|
| CCR author (B2-01 worker) | This document | **SUBMITTED 2026-11-07** (preserved history) |
| T01 integrator | Contract text, `/40` version/hash, shared registration decisions | **ACCEPTED 2026-11-09; freeze target `/40`** |
| T13 owner | VF06 per-kind table, c14 VF06 cases | **COMPLETE 2026-11-09** — `c14_se` 16/16 |
| T09 owner | T09 function-fact consumption, c15 cases | **COMPLETE 2026-11-09** — `c15_ir` 13/13 |
| H04 owner | Candidate registry/route/step, h04 cases | **COMPLETE 2026-11-09** — `h04_candidate` 18/18; direct candidate `value=05` |
| T13/T09 co-owner | c43 A3 integration test | **COMPLETE 2026-11-09** — `c43_a3` 5/5 |
| Documentation owner | T01/T07/T13/M1/SE slice amendments | **COMPLETE 2026-11-09** — docs amendments plus evidence rows |

**Post-acceptance implementation evidence (2026-11-09).** The `/40` freeze was applied to `compiler/src/contract.rs` (three new normative rules), `compiler/contracts/CONTRACT_VERSION`, and `compiler/tests/freeze.rs`/`c08_gate1.rs`/`c20_ppscan.rs`; the frozen hash is `c6961ac65fb3fb7e4ad4fdb666c47584b94042949cb6ed17111a4d81b3b10c14`. The candidate dispatches SE29 before VF06 and reuses the same `funcdef` for T09; VF06 checks four M1 kinds; T09 consumes the committed function fact. The full compiler suite is 700 tests across 50 suites, with `c14_se` 16, `c15_ir` 13, `c39_sefunc` 8, `c43_a3` 5, and `h04_candidate` 18 passing; three direct candidate runs were byte-identical and emitted `t01-c01-c06/40` and `interpret: value=05 negative=false`. D-09 (`ConstRecord.ty`/`ir.op-immediate-type`) remains open for B2-02, and no target/probe/torture/M1-acceptance result is claimed.

**Accepted defaults (T01 integrator, 2026-11-09).**

| # | Accepted default |
|---|---|
| O-1 | D-09 handled by a separate tagged-constant CCR (**B2-02**); B2-01 closes A3 only and IR-4(b) stays deferred. |
| O-2 | `FunctionDefinition` fact `ValueCategory` stays **`NonLvalue`** (frozen `/39`); `FunctionDesignator` is not adopted. |
| O-3 | T09 link is **node-keyed** (`SemRecord.node == FunctionDefinition`); no payload `RecordRef::Sem` or `SymbolId` field. |
| O-4 | D-09 deferral is **document-only** in T01/M1 docs; no hash-visible "deferred" rule. |
| O-5 | VF06 **requires exactly one `FunctionDefinition` node** in the M1 TU; no vacuous pass. |
| O-6 | T09 **re-checks** the function fact's `category`/`effects`; it does not trust VF06 alone. |
| O-7 | SE29 existing-fact reuse validation is **DEFERRED** (optional hardening, **not in `/40`**). |
| O-8 | Candidate uses the cumulative **`se_function_slice()`** registry; no dedicated `candidate_m1_slice()`. |
| O-9 | Zero/multiple `FunctionDefinition` nodes remain a candidate **driver error** in M1; no new typed diagnostic. |
| O-10 | Any canonical M1 Part A evidence is captured **after `/40`**; no frozen golden is adopted by this acceptance. |
| O-11 | Any future A17 `Verification` family adoption is a **separate CCR**; VF06 stays zero-write at `/40`. |
| O-12 | `ir.constant-result-type-from-sem` is **added now** as the interim checkable IR type rule. |

Acceptance does not claim implementation, tests, or evidence; the integrator records those only after the `/40` freeze and the implementation work. Any later change to the accepted rule names, the T09 link encoding, or the D-09 split must update this CCR and the dependent contracts together.

---

## 12. References

- [AGENTS.md](../../AGENTS.md) §3–5 (architecture, correctness, interfaces, validation).
- [T01_COMPILER_CONTRACT.md](T01_COMPILER_CONTRACT.md) §3–5, §7.1; `/40` current artifact.
- [T07_SEMANTIC_CHIPS.md](T07_SEMANTIC_CHIPS.md) SE29 row, operative amendments 1/4/5, M1 `SemRecord` checklist S1–S7, §3 VF06 position.
- [T13_VERIFICATION_CHIPS.md](T13_VERIFICATION_CHIPS.md) operative VF06 amendment and M1 fixture checklist items 1–7.
- [M1_TARGET_ACCEPTANCE.md](M1_TARGET_ACCEPTANCE.md) §3.1 A3, §3.2 IR-4, §2 stage table.
- [M1_VERTICAL_SLICE_ACCEPTANCE.md](M1_VERTICAL_SLICE_ACCEPTANCE.md) §5.5 `M1-SE-04`, §7 owner table, §8 VF06/VF07–VF09 gap notes.
- [SE_FUNC_SLICE.md](SE_FUNC_SLICE.md) `/39` SE29 freeze, §4 registration, §6 execution record, §7 deferred.
- [A05_IR_TARGET_HOST.md](../reviews/dispatch/A05_IR_TARGET_HOST.md) §3.2 G3/G4 and §6.2 open questions 1–2.
- [A04_PARSE_TYPE_SEMANTICS.md](../reviews/dispatch/A04_PARSE_TYPE_SEMANTICS.md) §5.3 semantic record production findings.
- [A17_VERIFICATION_REQUIREMENTS.md](../reviews/dispatch/A17_VERIFICATION_REQUIREMENTS.md) §2.1/§6.1 current VF06 carriers, §5 proposed verification family (not adopted here).
- [A14_CONST_LAYOUT_INIT_CONTRACT.md](../reviews/dispatch/A14_CONST_LAYOUT_INIT_CONTRACT.md) §4.2 proposed `ConstRecord` v2 (B2-02 input).
- [DISPATCH_LEDGER.md](DISPATCH_LEDGER.md) §3A D-02/D-09, §3C B2-01.
- Source: `compiler/src/bin/candidate.rs`, `compiler/src/chips/semantic/se_function.rs`, `compiler/src/chips/verify/vf_invariant.rs`, `compiler/src/chips/ir_lower/ir_function.rs`, `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/src/snapshot.rs`.
- Tests: `compiler/tests/c14_se.rs`, `c15_ir.rs`, `c39_sefunc.rs`, `c43_a3.rs`, `h04_candidate.rs`, `freeze.rs`, `c08_gate1.rs`, `c20_ppscan.rs`.
