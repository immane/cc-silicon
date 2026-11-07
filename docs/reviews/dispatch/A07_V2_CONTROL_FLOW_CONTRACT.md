# A-PTS-02: V2 Control-Flow Contract Requirements

| Field | Value |
|---|---|
| Status | **PROPOSED contract-requirement package. Not a freeze, not an implementation authorization, not a test or acceptance report.** |
| Date | 2026-11-07 |
| Task | Stage A `A-PTS-02` under [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) |
| V2 definition | `if/else`, loops, `break`/`continue`, short-circuit expressions, and explicit CFG ([MASS_PARALLEL_DISPATCH.md §6.2](../../tasks/MASS_PARALLEL_DISPATCH.md)) |
| Contract baseline inspected | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` ([CONTRACT_VERSION](../../../compiler/contracts/CONTRACT_VERSION)) |
| Authority order used | User instruction > accepted ADR > accepted contract > architecture/design > existing implementation ([AGENTS.md §2](../../../AGENTS.md)) |
| Evidence basis | Read-only inspection of `compiler/src`, `compiler/tests`, and task documents. **No test suite, probe, corpus, or execution was run for this report.** Uncommitted workspace drafts are treated as user work and were not modified. The parallel Stage A drafts A06/A15/A19 were also read to record conflicts; they remain non-frozen. |
| Sole write | This file only. Source, tests, contracts, `Cargo` files, `mod.rs`, shared indexes, and all other files are unchanged. |

> **Separation rule used in this report.**
> "FROZEN" means the shape exists in `/39` and is verified by the current freeze test.
> "PROPOSED" means this report's recommended minimal requirement for the next V2 freeze.
> "OPEN" means the integrator/owner must decide before implementation.
> "BLOCKED" means a named upstream freeze or implementation does not exist yet.
> No sentence in this report should be read as an accepted contract, a design freeze, or a claim that V2 works today.

---

## 1. Goal and exact scope

### 1.1 Goal

Produce the smallest shared contract that makes a real V2 slice possible:

```text
source
  -> PA parse tasks (AST + child links + parse continuation)
  -> TY scope/type legality (for/compound scopes; V1 types)
  -> SE semantic facts (condition legality, jump-target binding, logical facts)
  -> CL/IR explicit CFG (blocks, Branch/CondBranch terminators, CFG successors,
                        control regions, lowering frames)
  -> verification (VF05 AST, VF06 typed facts, VF07 CFG, VF12 interpreter,
                   VF13 replay)
  -> a defined verdict on every V2 fact, with no silent pass and no
     constant-folding substitution for control flow.
```

V2 is a **contract and correctness slice**, not an optimization slice. It does not require target code generation. When target semantics are not unlocked, V2 reports Part A evidence only and an explicit target blocker.

### 1.2 In scope

1. `if` / `else`, including the **dangling-else** rule.
2. `while`, `do ... while`, and `for` loops, including empty `for` clauses.
3. `break` and `continue`, including target binding to the nearest enclosing construct.
4. Short-circuit `&&` and `||` in **condition/branch context** (the V2 exit gate).
5. `?:` only as an explicitly gated V2 extension (see §8.1 O4); not a V2 exit gate.
6. Explicit CFG for the above: blocks, conditional/unconditional terminators, edges, joins, loop back edges, break/continue targets.
7. Task nesting/await rules, parse/semantic/lowering continuations, stage placement, and scope context needed by the above.
8. Error, recovery, determinism, snapshot, and replay requirements.
9. Verification facts and the minimum verification-chip registration needed to make the slice honest.

### 1.3 Out of scope (must fail explicitly, never silently pass)

- `switch` / `case` / `default` (T05 PA30). `break` inside `switch` is therefore not a V2 exit gate. The shared records reserve the switch region kind so a later freeze does not need a shape change.
- `goto`, labels, computed goto, GNU label addresses (T05 PA32/PA33, T12). `break`/`continue` must not be rewritten as labels as a shortcut.
- Function calls, pointers, arrays, aggregates, floating point, VLA, `setjmp`/`longjmp`, inline assembly, and all target ABI/instruction work (V3 and later).
- Constant folding of conditions as a substitute for control flow. The T08 CL04 selected-branch chips ([T01 `/37`](../../tasks/T01_COMPILER_CONTRACT.md), [`fold_branch.rs`](../../../compiler/src/chips/constant_layout_init/fold_branch.rs)) are `Ack`-only certifications and must not prune or rewrite the V2 CFG.
- Optimization. V2 establishes the correctness baseline; T10 may only rewrite later under its own legality rules.
- Target execution, probe, GCC torture, or any pass-rate claim.

### 1.4 Upstream dependency that is not V2-owned

V2 lowers conditions and statements, but it does **not** own general expression parsing, expression typing, local variables, assignment, comparison/arithmetic IR, or the V1 slot/load/store representation. Those are V1 common-base items (A-PTS-01). V2 can freeze its control-flow records in parallel, but V2 integration is **BLOCKED** until the V1 common base supplies at least:

- a general expression AST and `SemRecord` typing for integer arithmetic/comparison/assignment (the current `/39` AST has only `BinaryAdd`, `IntLiteral`, and no general expression node);
- an IR expression-lowering task that returns a `ValueId` (the current `ir.function` emits only a folded `Constant` + `Return`);
- a local/slot/load/store representation if `&&`/`||` must be used in **value** context rather than condition context;
- a general `return <expression>;` and expression statement.

This is the central V2 conflict: the current `/39` frozen slice is a fixed single-function `2+3` tree, not a base on which V2 can be grown without a V1/V2 common-base amendment.

### 1.5 Definition of "real V2 slice" used here

A V2 slice is real only when all of the following are true on a committed state:

1. PA produces a committed AST with `If`, loop, `Break`, `Continue`, `LogicalAnd`, and `LogicalOr` nodes whose ranges and child links are verifiable.
2. TY provides the scope context for compound bodies and `for` declarations (if V1 supports `for (int i = ...)`).
3. SE commits one `SemRecord` per checked control node and one `JumpTargetRecord` per `break`/`continue`.
4. IR commits an explicit CFG whose every block has exactly one terminator, whose edges resolve inside the function, whose `break`/`continue` targets are the declared loop exits/continuations, and whose `if`/loop joins are explicit.
5. VF05/VF06/VF07/VF12/VF13 return a defined verdict; unsupported cases return `Unsupported`, never `Ack`.
6. Two identical runs produce identical task/record/block/edge IDs and an identical canonical snapshot (scheduler registers excluded where the contract says so).
7. A deliberately broken CFG (dangling edge, missing terminator, instruction after terminator, wrong break target) is rejected by the verifier, not accidentally accepted.

---

## 2. Baseline facts and conflict register

### 2.1 Frozen `/39` facts relevant to V2

| Area | `/39` fact | V2 consequence |
|---|---|---|
| AST kinds | `NodeKind` is a closed 8-variant M1 set: `TranslationUnit`, `FunctionDefinition`, `Specifiers`, `Declarator`, `Compound`, `Return`, `BinaryAdd`, `IntLiteral` ([`compiler/src/bus.rs`](../../../compiler/src/bus.rs) `NodeKind`; [`contract.rs` `NODE_KIND_NAMES`](../../../compiler/src/contract.rs)). | V2 needs new variants; the list and hash must change. |
| AST record | `NodeRecord { kind, parent, children, first_token, last_token, name, literal }` ([`bus.rs`](../../../compiler/src/bus.rs)). `children` is a committed `Vec<NodeId>`. | A node is appended once. A parent cannot append its `children` after child tasks complete, and a child cannot append before its parent exists. This is a structural ordering conflict (§2.2 C2). |
| AST producer | `PaTuChip` validates the exact 13-token M1 TU and appends the fixed nine-node tree in one pre-order batch ([`pa_tu.rs`](../../../compiler/src/chips/parse/pa_tu.rs)). | The single-batch prediction pattern is the only currently working cross-node link mechanism. It does not scale to task-nested statements. |
| Parse continuation | `ContinuationRecord` has exactly nine fields: `production`, `cursor`, `context`, `binding_power`, `scope`, `parent`, `partial_children`, `next_child_ordinal`, `previous`; no `awaited` ([`task.rs`](../../../compiler/src/task.rs)). | Reusable for V2 statement frames if `ParseContext` is extended. No new continuation record is needed for PA. |
| Parse context | `ParseContext` is a closed 10-member enum ending at `Primary` ([`task.rs`](../../../compiler/src/task.rs)). | No `Statement`, `StatementList`, `Condition`, `ForClause`, `LoopTail`, or `ElseTail`. |
| Parse task kinds | Frozen PARSE locals 16-26. `parse.block` (20) is the exact 7-token `{ return <int> + <int> ; }`; `parse.return` (21) is the exact 5-token M1 return ([`pa_block.rs`](../../../compiler/src/chips/parse/pa_block.rs); [`task.rs`](../../../compiler/src/task.rs)). | V2 needs a real compound/statement list and expression returns; both frozen semantics must be amended or superseded. |
| Semantic record | `SemRecord { node, ty, category, effects }`, one per checked `NodeId`; M1 allows only `EffectMask(0)` ([`bus.rs`](../../../compiler/src/bus.rs); [T07 rev 46](../../tasks/T07_SEMANTIC_CHIPS.md)). | No jump-target field, no control-context field, no non-zero effect classes. |
| IR record | `IrOp` is closed to `Constant` and `Return`; `InstructionRecord { op, block, operands, immediate, result }`; no successors field; block terminator is the greatest `InstructionId` ([`bus.rs`](../../../compiler/src/bus.rs); [`ir_function.rs`](../../../compiler/src/chips/ir_lower/ir_function.rs)). | No `Branch`/`CondBranch`, no explicit successors, no join/region/frame record. |
| IR shape | M1 emits exactly one entry block, `Constant`, `Return`; multi-block functions fail VF12 ([`vf_interpret.rs`](../../../compiler/src/chips/verify/vf_interpret.rs)). | V2 must extend both the IR shape and the interpreter's covered subset. |
| Verification | VF05 checks the 8-kind tree; VF06 checks the M1 checked set; VF07 `CfgInvariantChip` exists only as a table row and is **out of M1** ([T13](../../tasks/T13_VERIFICATION_CHIPS.md)). | V2 activates VF07 and extends VF05/VF06/VF12; none of these extensions exists today. |
| Materialization | `G1DraftBody` is a closed set; families outside it are rejected with "record draft materialization pending the records track" ([`records.rs`](../../../compiler/src/records.rs)). | New families cannot be appended until the integrator adds typed bodies and commit materialization. |
| Allowlist gate | `check_store_owner_allowlist` is wave-gated: a manifest claiming **no recognized slice kind** passes untouched ([`manifest.rs`](../../../compiler/src/manifest.rs)). | New V2 kinds must be added to a recognized slice predicate and get allowlist rows, or V2 writers bypass owner validation. This is a security/correctness defect if forgotten. |
| Stage assignment | `STAGE_ASSIGNMENT` has no V2 kinds; registration fails `StageUnassigned` ([`manifest.rs`](../../../compiler/src/manifest.rs)). | Every new kind needs an explicit stage row. |
| Version | `t01-c01-c06/39`, hash `07f4...` ([`CONTRACT_VERSION`](../../../compiler/contracts/CONTRACT_VERSION)). | V2 requires a new artifact (proposed `/40`) with `/39` preserved as history. |

### 2.2 Conflict register

| ID | Conflict | Evidence | Required resolution |
|---|---|---|---|
| C1 | `NodeKind` is closed; V2 cannot add `If`/loop/jump/logical variants without a schema/hash change. | `bus.rs` `NodeKind`; `contract.rs` `NODE_KIND_NAMES`; `snapshot.rs` `node_kind_name`/`parse_node_kind`. | Append-only `NodeKind` variants under a new version; update all name maps and decoders. |
| C2 | `NodeRecord.children` is a single-shot committed vector; cross-task parent/child construction creates a parent-before-children vs children-before-parent ordering cycle. | `NodeRecord`; `pa_tu.rs` single-batch pre-order; `NodeRecord.children` has no patch path (patches carry `RecordRef`, not record bodies). | Add an append-only child-link record owned by T05; parent appends links after child results are committed. Keep `children` for `/39` batch trees only. |
| C3 | `ParseContext` has no statement contexts. | `task.rs` 10-member enum; `snapshot.rs` ordinal 0-9 decoder. | Append `Statement`, `StatementList`, `Condition`, `ForClause`, `LoopTail`, `ElseTail`. |
| C4 | `parse.block` and `parse.return` are frozen to exact M1 token shapes and `Ack` results. | `pa_block.rs` task header and `M1_BLOCK_KINDS`/`M1_RETURN_KINDS`. | Amend the same kinds to real grammar/result semantics under a version bump, or add new kinds and retire the old producers in the same freeze. Do not let both write the same nodes. |
| C5 | `SemRecord` has no jump target and `EffectMask` is zero-only. | `bus.rs`; T07 rev 46. | Add a separate `JumpTargetRecord`; do not add an optional field to every `SemRecord`. Keep `EffectMask(0)` for V2; see O5. |
| C6 | `IrOp` has no `Branch`/`CondBranch`; `InstructionRecord` has no successors. | `bus.rs`; `ir_function.rs`. | Adopt A15's `InstructionRecord.successors` field and `Branch`/`CondBranch` ops as the primary shape; use a separate `CfgEdgeRecord` only as the documented fallback. Do not add a third targets field. |
| C7 | No durable IR lowering frame/control region exists; `ContinuationRecord` is parse-specific. | `ContinuationRecord`; T09 "lower frames/proposals" has no record. | Add `ControlRegionRecord` and `LoweringFrameRecord` owned by T09. |
| C8 | `G1DraftBody` cannot materialize new families. | `records.rs` closed enum and comment. | Integrator adds typed bodies, arenas, IDs, tags, encoders, and commit materialization before any V2 chip writes. |
| C9 | `check_store_owner_allowlist` bypasses unrecognized kinds. | `manifest.rs` wave-gated predicate. | Add a `v2_control` slice predicate and one allowlist row per writer/field/kind. |
| C10 | VF05/VF06/VF12 assume the M1 shape; VF07 is not registered; VF01's reserved-arena inventory expects `layouts`/`inits`/`vregs` empty and does not know the new V2 arenas. | `vf_syntax.rs`, `vf_invariant.rs`, `vf_interpret.rs`, `vf_store.rs`; T13. | Extend the verifiers and register VF07; add the new arenas to the VF01 reserved/live inventory; never let a multi-block function fall through VF12 as `Ack`. |
| C11 | SE29 requires a direct committed `Return` child of `FunctionDefinition`. | `se_function.rs` header; `SEMANTIC_FUNCTION_DEF`. | V2 function bodies contain `If`/loop nodes, not a direct `Return`; amend SE29 to check body completeness through the committed AST/links or add a V2 function-definition checker. |
| C12 | Uncommitted `pa_stmt.rs` is a candidate that claims PARSE locals 27-30 and `ChipId(58)`, rejects every `if`/loop input as `Unsupported`, writes no AST, and declares an empty test list. | `compiler/src/chips/parse/pa_stmt.rs` task header, `PA27..PA31` constants, `manifest()` `tests: vec![]`. | It cannot register (`MissingTests`) and cannot satisfy V2. It is read-only user work; V2 must use new files or an integrator-supervised rewrite. Do not overwrite it. |
| C13 | `parse/mod.rs` is modified in the working tree to include `pa_stmt.rs` and other drafts. | `git status` (` M compiler/src/chips/parse/mod.rs`); uncommitted `pa_cast.rs`, `pa_decl_spec.rs`, `pa_expr_ext.rs`, `pa_ext_dispatch.rs`, `pa_init.rs`, `pa_stmt.rs`, `pa_typedef_query.rs`. | `mod.rs` is integrator-owned; V2 work orders must not touch it. The integrator registers/retires drafts. |
| C14 | T05/T07/T09/T13 documents contain candidate/selected-direction text that is not a freeze and does not include V2 shapes. | T05 items E/G/H; T07 rev 46; T09 candidate defaults; T13 VF07 out of M1. | The V2 freeze must update those documents in the same batch; this report is a requirement input, not their replacement. |
| C15 | A task cannot attach or update its own resume frame after enqueue: `Task.continuation` is `Option<ContinuationId>` fixed at enqueue, `TaskDraft.continuation` is not a `ContinuationRef`, and `Proposal` has no frame-update transition. T05 item C records the `OwnBatch` migration as pending. | `task.rs` `TaskDraft.continuation` note; `Proposal` variants; T05 item C. | Freeze one of: (a) a generic task frame update (`FrameRef` + `Proposal::SetFrame`), (b) the `TaskDraft.continuation`/`ContinuationRef` migration with same-commit resolution, or (c) a committed-frame successor-task convention. Without this, V2 cannot resume after `AwaitChildren` with a new frame. See §3.5.1 and O16. |

### 2.3 Conflicts with the parallel Stage A drafts (A06/A15/A19)

These reports were written in the same Stage A batch and are **not frozen**. They are recorded here because V2 must not silently invent a second type system or contradict the likely V1/IR base.

| ID | Parallel draft | Conflict with this report | Recommended reconciliation |
|---|---|---|---|
| C16 | **A06 V1** recommends one `PARSE_TU` task that appends the complete V1 tree in one `AppendRecords` batch and keeps `NodeRecord.children` authoritative; it explicitly rejects the taskified parser and leaves continuation materialization deferred. | This report's taskified PA/`NodeLink`/`SetFrame` design is a different architecture. A06's one-task parser avoids `NodeLink` and C15 entirely. | Decide at the V1/V2 common-base freeze: (a) accept A06's bootstrap parser for V1 and **extend the same single task to V2**, dropping `NodeLink`/C15 from the V2 contract; or (b) accept this report's taskified parser and explicitly retire/limit the A06 single task. Do not run both AST producers. See O17. |
| C17 | **A15 IR** proposes `InstructionRecord.successors` and `IrOp::{Branch, CondBranch, Switch, Unreachable, ...}`; it forbids unreachable live blocks (CFG-3). | This report's earlier draft proposed a separate `CfgEdgeRecord`, `Branch` conditional, `Jump` unconditional, and dead blocks. The names and edge carrier collide. | Adopt A15's `successors` + `Branch`/`CondBranch` as the primary V2 IR shape (§3.4.1-3.4.2); use `CfgEdgeRecord` only as the documented fallback. Adopt A15's no-live-unreachable rule and omit unreachable source statements from IR rather than emitting dead blocks. See O18. |
| C18 | **A15 IR** task table uses `[Function, Block, ...]` payloads and `Records([Block, Instruction, ...])` results for `ir.short_circuit`/`ir.if`/`ir.loop`/`ir.jump`. | This report's task table uses `[Node, Block, ...]` payloads and `Record(Block)`/`Ack` results as a minimal carrier. | Treat this report's task table as a **functional requirement**; the exact payload/result spelling must be co-frozen with A15. Prefer A15's carrier where it already covers the same kind; do not invent a second result variant. |
| C19 | **A19 V2 tests** owns the independent acceptance suite (`compiler/tests/v2_support/`, `v2_parse.rs`, `v2_sem.rs`, `v2_cfg.rs`, `v2_e2e.rs`) and expects A-PTS-02/A15/A17 reconciliation. | This report also lists verification facts and test ideas; duplicating the file ownership would create two test owners. | A19 owns the test files and expected values; this report's Appendix A is a requirement/fact input only. No test file may be written by the V2 chip owners. |
| C20 | **A06 V1** claims SEMANTIC locals 22-25 and IR_LOWER locals 17-18; this report's first candidate table claimed overlapping SEMANTIC/IR locals. | A06 §3.6.2 table. | V2 starts above the frozen V1 locals: this report proposes SEMANTIC 26-30 and IR_LOWER 19-26, with exact assignment O15. The integrator must renumber in one batch if V1 freezes more locals. |

---

## 3. Required shared schema, task, and result changes

All records below are **PROPOSED**. Field order is the proposed canonical order. "Committed" means a live record in the corresponding arena; drafts, wires, and predicted IDs are never durable references (Guardrails §6.1).

### 3.1 AST additions (T05-owned)

#### 3.1.1 `NodeKind` append-only variants

Append after `IntLiteral`; do not renumber or reinterpret the `/39` variants.

| Variant | Children by `ordinal` (text order) | Token range | Semantics / validation |
|---|---|---|---|
| `If` | 0 = condition expression; 1 = then statement; 2 = else statement if present | `if` .. end of then/else statement | Exactly 2 or 3 links. No else means no ordinal 2. `condition` is any expression node; scalar legality is T07's job. |
| `While` | 0 = condition expression; 1 = body statement | `while` .. end of body | Exactly 2 links. |
| `DoWhile` | 0 = body statement; 1 = condition expression | `do` .. `;` after `while (...)` | Exactly 2 links. Body precedes condition in text order; runtime body-first order is lowered explicitly. |
| `For` | 0 = init (optional); 1 = condition (optional); 2 = step (optional); 3 = body | `for` .. end of body | Up to 4 links; missing clauses have no link. `init` may be an expression/declaration node; V1 decides the node kind. |
| `Break` | none | `break` .. `;` | Leaf. Semantic target is not stored here; see `JumpTargetRecord`. |
| `Continue` | none | `continue` .. `;` | Leaf. |
| `LogicalAnd` | 0 = lhs; 1 = rhs | first .. last operand | Exactly 2 links. `&&` semantics are short-circuit; the AST does not encode evaluation order beyond operand order. |
| `LogicalOr` | 0 = lhs; 1 = rhs | first .. last operand | Exactly 2 links. |
| `Conditional` (optional, O4) | 0 = condition; 1 = then; 2 = else | `?` .. end of else | Exactly 3 links. Required only if `?:` is accepted into V2. |

Rules:

1. `NodeKind` numeric discriminants are declaration order; appending is the only allowed change. `snapshot.rs` must append wire names and `parse_node_kind` arms; unknown names still fail.
2. A node's `first_token`/`last_token` are committed tokens; the range includes the construct's keywords and terminating `;`/`}`. Child ranges must be contained and ordered.
3. The AST does not store an operator field. Following the existing `BinaryAdd` precedent, V2 uses op-specific `NodeKind` variants (`LogicalAnd`, `LogicalOr`). If V1 instead freezes a general `BinaryExpression` with an operator enum, V2 consumes that enum and appends the two logical values there; it must not introduce a second incompatible expression representation. This is an OPEN common-base decision (O2).

#### 3.1.2 `NodeLinkRecord` (new family; T05-owned)

The smallest cross-task AST child carrier. It is append-only and never replaces a record.

| Field | Type | Presence | Meaning | Validation |
|---|---|---|---|---|
| `parent` | `NodeId` | required | Committed parent node. | Must be live; kind must permit the child ordinal; parent `NodeRecord.children` must be empty for a linked node. |
| `child` | `NodeId` | required | Committed child node. | Must be live; `child != parent`; `child.parent == Some(parent)`. |
| `ordinal` | `u32` | required | Child slot in the parent's ordered child list. | Unique per `(parent, ordinal)`. For fixed-shape parents (`If`, loops, `LogicalAnd`/`LogicalOr`, `ExpressionStatement`, `Return`) the allowed slots are exactly those in §3.1.1 and optional slots may be absent; for `Compound`/statement lists the slots are dense `0..n-1`. |

Structural rules:

- Exactly one of two representations per node: a `/39` batch tree has `NodeRecord.children` populated and **no** `NodeLink` rows; a task-built tree has `children` empty and `NodeLink` rows. VF05 rejects a node using both or neither when it has children.
- Only the parent's parse task appends links, after the child's committed `Record(Node)` result is visible. Links therefore contain committed IDs only; no predicted child/parent references are needed.
- A parent task determines its own `first_token`/`last_token` range by a bounded scan of committed delimiters before appending the parent node; it does not need the child nodes to exist first. Child ranges are validated to be contained afterward.
- Links are appended in ordinal order in one batch when the parent resumes, or incrementally for a statement list; the final set uses each allowed slot at most once and is dense only for statement-list parents.
- A failed child means the parent fails before appending its links. A partial committed parent node may remain; the job is failed and no successful verification is claimed. VF05 runs only on successful jobs.

### 3.2 Parse context and continuation (T05/T01-owned)

#### 3.2.1 `ParseContext` append-only members

| Member | Ordinal | Grammar entry / meaning |
|---|---:|---|
| `Statement` | 10 | At a statement keyword, expression start, `{`, or `;`. |
| `StatementList` | 11 | Inside a compound, at the next block item or closing `}`. |
| `Condition` | 12 | Inside the `( ... )` of `if`/`while`/`do`/`for`. |
| `ForClause` | 13 | Inside `for ( ... )`, at init/condition/step. |
| `LoopTail` | 14 | After a `do` body, expecting `while ( condition ) ;`. |
| `ElseTail` | 15 | After an `if` then-statement, expecting optional `else`. |

- `context` remains a grammar-entry constraint distinct from `production` (the registered task kind); no one-to-one mapping is invented here.
- The snapshot decoder extends from ordinal `0..=9` to `0..=15`; `>=16` is `InvalidTag`.
- `ContinuationRecord` is reused unchanged. `production` = the parser task to resume; `cursor` = the next committed token; `scope` = the current block/for scope; `parent` = the enclosing committed `NodeId`; `partial_children` may hold already-committed child IDs for bounded constructs; `next_child_ordinal` is the next child slot; `previous` forms the committed acyclic frame chain. No `awaited` field is added; `TaskState::Waiting(WaitSet)` remains the sole awaited-child source.
- For unbounded statement lists, prefer incremental links plus `Progress` (or bounded chunks) over filling `partial_children` with all statements.

### 3.3 Semantic additions (T07-owned)

#### 3.3.1 `JumpTargetRecord` (new family)

`SemRecord` stays the **V1-frozen shape**; V2 adds no `jump_target` field. The `/39` shape is `{ node, ty, category, effects }`; if the V1 common base appends `conversions`/`constant` (A06 proposal), V2 consumes that shape rather than redefining it. A separate `JumpTargetRecord` avoids making every checked node carry an optional jump target and preserves the one-record-per-node invariant.

| Field | Type | Presence | Meaning | Validation |
|---|---|---|---|---|
| `site` | `NodeId` | required | Committed `Break` or `Continue` node. | Must be live and of kind `Break` or `Continue`; exactly one record per site. |
| `target` | `NodeId` | required | Nearest enclosing loop node (`While`, `DoWhile`, `For`), or `Switch` when that is added. | Must be a committed AST ancestor of `site`; for `Continue`, target must be a loop; for `Break`, loop or switch. |
| `kind` | `JumpKind` | required | `Break` or `Continue`. | Must equal the site's `NodeKind` (catches mismatched producers). |

`JumpKind` is a closed enum `{ Break = 0, Continue = 1 }`; exact wire spelling is an O15 item. The record is appended in the same semantic batch as the site's `SemRecord` (proposed result `Records([Sem, JumpTarget])`).

#### 3.3.2 `SemRecord` for control nodes

| Node kind | `ty` | `category` | `effects` |
|---|---|---|---|
| `If`, `While`, `DoWhile`, `For` | `Void` (V1 must expose a canonical `void`) | `Void` | OR of child effects under the V1 mask; V2 exit fixtures use `0`; V2 adds no new bit |
| `Break`, `Continue` | `Void` | `Void` | `EffectMask(0)` |
| `LogicalAnd`, `LogicalOr` | `int` | `NonLvalue` | OR of operand effects under the V1 mask; V2 exit fixtures use `0` |
| `Conditional` (if adopted) | arm common type (V1) | per V1 | OR of selected-arm/condition effects under the V1 mask |

If V1 cannot expose a canonical `void` type, V2 must freeze one; do not use `int` as a statement type placeholder. This is a V1/V2 common-base item.

#### 3.3.3 TY prerequisite: canonical `void` type

V2 control-statement `SemRecord`s use `ty = void`. `/39` has `TypeKind::Void`, but the frozen TY producers only emit `int` and `int(void)`; there is no guaranteed canonical `Void` producer. The V2 freeze must either reuse a V1 canonical void type or add one TY kind:

| Proposed kind | Group/local (candidate) | Stage | Payload | Result | Writer |
|---|---|---:|---|---|---|
| `symbol_type.void_type` | SYMBOL_TYPE 26 (above V1's 25; exact O15) | 3 | empty | `Record(Type)` of a canonical `Void` `TypeRecord`; reuse if already committed | TY type chip (one producer) |

If V1 already freezes a canonical void producer, V2 consumes it and does not add a second one.

### 3.4 IR additions (T09-owned)

#### 3.4.1 `IrOp` append-only variants

**Naming reconciliation:** the parallel IR contract report A15 (`A15_IR_CONTRACT_REQUIREMENTS.md` §4.6-4.8) proposes `Branch` as the **unconditional** edge and `CondBranch` as the **conditional** edge, with a `successors: Vec<BlockId>` field on `InstructionRecord`. This report **recommends adopting** those names and that field as the primary V2 shape. The table below is the recommended shape; §3.4.2 records the separate-edge fallback only if the `/15` field append is rejected. Do not implement both.

| Variant | Operands | Immediate | Result | Terminator | Successors | Semantics |
|---|---|---|---|---|---|---|
| `Branch` | 0 | `None` | `None` | yes | exactly 1 (`[target]`) | Unconditional jump. Used for loop back edges, if joins, and `break`/`continue`. |
| `CondBranch` | exactly 1 (`cond: ValueId`) | `None` | `None` | yes | exactly 2 (`[true, false]`) | Continue at successor 0 if `cond != 0`, else successor 1. `cond` must be `int`/`_Bool` after an explicit V1 truth representation; no target flags, no host truthiness. |
| `Switch` (reserved) | 1 condition | `SwitchShape` via `detail` (A15) | `None` | yes | default then cases | Reserved for the later switch freeze; **not produced in V2**. |
| `Unreachable` (A15) | 0 | `None` | `None` | yes | 0 | A15's terminator for a reachable undefined path; not required by V2 and never a substitute for pruning constant conditions. |
| `Return` (existing) | 1 (V1) | `None` | `None` | yes | 0 | Function return. |
| `Constant` (existing) | 0 | `Some(ConstId)` | `Some(ValueId)` | no | 0 | Materialize a committed constant; never refold. |

V1 expression operations (add/compare/assign/load/store) are consumed, not defined here. `CondBranch` is the only V2 condition consumer; it does not perform arithmetic. A15's `Unreachable` terminator may be used for a reachable no-return path; it is not a substitute for pruning constant conditions.

#### 3.4.2 CFG edges: `InstructionRecord.successors` (primary) or `CfgEdgeRecord` (fallback)

**Primary recommendation (A15 alignment):** append one field to `InstructionRecord`:

| Field | Type | Presence | Meaning | Validation |
|---|---|---|---|---|
| `successors` | `Vec<BlockId>` | required; empty for non-terminators | Ordered successor blocks of a terminator. | `Branch` exactly 1; `CondBranch` exactly 2 (true, false); `Return`/`Unreachable` 0; `Switch` reserved. All targets live, same `FunctionId`, no duplicate successors for `Branch`/`CondBranch`. |

This is the smallest **type-count** change: no new family, ID, arena, wire tag, or store field. It does change the frozen `/15` `InstructionRecord` field set and therefore the `INSTRUCTION_RECORD_FIELDS` hash, the snapshot encoder, and every instruction body. A15 already proposes it; V2 consumes it rather than inventing a second edge representation. A15 also proposes `InstructionRecord.effect` and `InstructionRecord.detail`; V2 does not require a non-zero effect or a detail payload for `Branch`/`CondBranch`/`Return`, but must not conflict with those A15 fields if they are frozen in the same batch.

**Fallback only if the integrator rejects the `/15` field append:** a new append-only family with the same information.

| Field | Type | Presence | Meaning | Validation |
|---|---|---|---|---|
| `from` | `InstructionId` | required | Committed terminator instruction. | Live; op is `Branch`, `CondBranch`, or a reserved switch terminator; unique terminator of its block. |
| `to` | `BlockId` | required | Successor block. | Live, same function. |
| `ordinal` | `u32` | required | Edge ordinal for this `from`. | Unique per `(from, ordinal)`; contiguous; arity `Branch` 1, `CondBranch` 2, `Return` 0. |

Rules common to either representation:

- The CFG is explicit; it is never inferred from token order or a hidden scan.
- A block's terminator is the greatest `InstructionId` in that block; no instruction may be appended after it. The "entry block is terminated" rule in [T09 TerminatorMissing](../../tasks/T09_IR_LOWER_CHIPS.md) is a special case of "every live block is terminated".
- Join blocks are ordinary `BlockRecord`s with at least two incoming edges when reachable. No phi record and no SSA assumption is introduced. Loop-carried values use V1 slots/loads/stores, not phi (O2).
- If the fallback family is chosen, do not also add `successors`; one edge source only.

#### 3.4.3 `ControlRegionRecord` (new family)

Durable, verifiable mapping from an AST control construct to its CFG targets.

| Field | Type | Presence | Meaning | Validation |
|---|---|---|---|---|
| `function` | `FunctionId` | required | Owning function. | Must be live. |
| `construct` | `NodeId` | required | Committed `If`, loop, or (future) `Switch` node. | Must be live and in the same function's AST. Exactly one region per `(function, construct)`. |
| `kind` | `RegionKind` | required | `If`, `Loop`, `Switch`. | Must match the construct kind; `Switch` is reserved and must not be produced in V2. |
| `entry` | `BlockId` | required | Region entry block (condition/header or body for do-while). | Same function. |
| `join` | `Option<BlockId>` | required for `Loop`; optional for `If` | Region fallthrough/exit block. | Same function. `Loop` always has `Some(exit)`. `If` has `Some(join)` when an arm falls through; `None` when both arms terminate (no fallthrough). `Switch` reserved. |
| `break_target` | `Option<BlockId>` | required for `Loop`/`Switch` | Loop exit / switch join. | Same function; `None` for `If`. |
| `continue_target` | `Option<BlockId>` | required for `Loop` | Loop continuation point (header for `while`, condition for `do`, step for `for`). | Same function; `None` for `If`/`Switch`. |
| `parent` | `Option<ControlRegionId>` | optional | Lexical enclosing region. | Committed, acyclic, same function. |

#### 3.4.4 `LoweringFrameRecord` (new family)

Durable resume state for T09 subtree lowering. It is not a parse continuation.

| Field | Type | Presence | Meaning | Validation |
|---|---|---|---|---|
| `function` | `FunctionId` | required | Owning function. | Live. |
| `node` | `NodeId` | required | AST node being lowered. | Live. |
| `region` | `Option<ControlRegionId>` | optional | Enclosing control region. | Same function; `None` only for the function/root frame. |
| `block` | `BlockId` | required | Current insertion block. | Same function. |
| `next_child_ordinal` | `u32` | required | Next child slot to lower. | Within the node kind's allowed slot range (§3.1.1); monotonic on resume. |
| `parent` | `Option<LoweringFrameId>` | optional | Lexical parent frame. | Committed, acyclic, same function. |
| `previous` | `Option<LoweringFrameId>` | optional | Previous committed frame for the same task. | Committed, acyclic; task continuation points at the latest head. |

Rules:

- Frames are immutable and append-only. A resume appends a new frame with `previous = old_head`; `Task.continuation`/the task's own frame reference points at the new head. This mirrors the committed `ContinuationRecord.previous` pattern.
- `parent` is lexical nesting; `previous` is resume history. They are not interchangeable.
- A frame chain and the task parent chain are both bounded by `max_task_depth`/`max_tasks_total`; no unbounded Rust recursion is allowed.

#### 3.4.5 Block and function semantics

- `FunctionRecord.entry` remains the unique entry block.
- `BlockRecord { function, ordinal }` remains unchanged. Block ordinals must be unique per function and allocated in the frozen order in §6.1. Instructions are still derived by ascending `InstructionId`; blocks do not store instruction lists.
- **No live unreachable block (A15 CFG-3 alignment).** Every live block must be reachable from entry through explicit successors. Unreachable source statements after a terminator are still parsed and semantically checked, but they are **not lowered into live IR**; they produce no block. A lowerer must not emit an unreachable "dead block" just to hold unreachable code.
- If both `if` arms terminate, the `if` has no fallthrough and no join block is emitted; the enclosing remainder is unreachable and is not lowered. If one arm falls through, a reachable join block is emitted.
- Function falloff (`main` reaching `}`) must be made explicit by the function shell: emit the C-mandated `return 0` for `main` or reject the shape as `Unsupported`. Never leave the last live block unterminated.
- `FunctionEnd`/IR28 completion is the per-function completion fact. The T01-owned phase-2b hook must extend from "entry block terminated" to "every live block in the function is terminated and every successor is valid" or the freeze must explicitly scope the hook to the entry block and leave the rest to VF07. The current `CommitError::TerminatorMissing` is shape-only and not implemented ([`commit.rs`](../../../compiler/src/commit.rs)). This is OPEN O9.

### 3.5 Task and result shapes

The frozen envelope stays `Task { id, kind, payload, owner, parent, continuation, state, ... }`, `Payload` remains `RecordRef`-only, and `ResultValue` remains the closed `{ Empty, Ack, Record, Records, Diagnostic }` set. No new `ResultValue` variant is proposed. Ordered payload conventions are the task-kind contract.

| Proposed kind | Group/local (candidate; V1 claims SEMANTIC 22-25 and IR_LOWER 17-18, so V2 starts above them; exact assignment O15) | Stage | Payload order | Transition path | Result | Continuation / await |
|---|---|---:|---|---|---|---|
| `parse.statement` | PARSE 27 | 2 | `[Token(first), Node(parent), Scope]` | Running -> Enqueue one production child -> `AwaitChildren` -> Resume -> Complete | `Record(Node)` forwarded from the selected production | Context `Statement`; one child; no node of its own. |
| `parse.if` | PARSE 28 | 2 | `[Token(if), Node(parent), Scope]` | Tick 1: append `If` node (children empty) + `Progress` anchor. Tick 2: Enqueue condition/then/else children with the committed `If` ID + `AwaitChildren` -> Resume -> append `NodeLink` rows -> Complete | `Record(Node(If))` | Context `ElseTail`; children in order condition, then, else; await-all. |
| `parse.loop` | PARSE 30 | 2 | `[Token(while/do/for), Node(parent), Scope]` | Tick 1: append `While`/`DoWhile`/`For` node + `Progress`. Tick 2: Enqueue clause/body children with the committed loop ID + `AwaitChildren` -> Resume -> append links -> Complete | `Record(Node(loop))` | Context `Condition`/`ForClause`/`LoopTail`; up to four children; await-all. |
| `parse.jump` | PARSE 31 | 2 | `[Token(break/continue), Node(parent), Scope]` | Append leaf node -> Complete | `Record(Node(Break/Continue))` | Context `Statement`; no children. `return` stays `parse.return`. |
| `parse.block` (amended) | PARSE 20 | 2 | `[Token({), Node(parent), Scope]` | Tick 1: append `Compound` node + `Progress`. Tick 2..: parse items in bounded chunks; each item child -> `AwaitChildren` -> append links -> `Progress`/next chunk -> Complete | `Record(Node(Compound))` | Context `StatementList`; one/few children per chunk; await-all per chunk. |
| `parse.return` (amended) | PARSE 21 | 2 | `[Token(return), Node(parent), Scope]` | Tick 1: append `Return` node + `Progress`. Tick 2: Enqueue expression child with the committed `Return` ID (or none for `return;`) + `AwaitChildren` -> append link -> Complete | `Record(Node(Return))` | Context `Statement`; one child. |
| `parse.tu` (amended) | PARSE 16 | 2 | `[Token(first), Scope]` | Tick 1: append TU root + `Progress`. Tick 2..: parse external declarations; links -> Complete | `Record(Node(TU))` | Context `TranslationUnit`; bounded chunks; await-all. |
| `semantic.logical` | SEMANTIC 26 | 4 | `[Node(logical)]` | Check scalar/lvalue/effect facts; append `SemRecord`; Complete | `Record(Sem)` | No child needed if children are already checked; otherwise enqueue operand checks and await. |
| `semantic.if_stmt` | SEMANTIC 27 | 4 | `[Node(if)]` | Check condition child `SemRecord` scalar `int`/`_Bool`; await arms if their effects are needed; append `SemRecord(If)` with OR effects; Complete | `Record(Sem)` | May enqueue `semantic.condition`/child checks; await-all. |
| `semantic.loop_stmt` | SEMANTIC 28 | 4 | `[Node(loop)]` | Check condition/body; append `SemRecord(loop)` with OR effects; Complete | `Record(Sem)` | May enqueue `semantic.condition`/child checks; await-all. |
| `semantic.jump_stmt` | SEMANTIC 29 | 4 | `[Node(break/continue)]` | Resolve nearest target; append `SemRecord(site)` + `JumpTargetRecord`; Complete | `Records([Sem, JumpTarget])` | No child; target walk is over committed `NodeRecord.parent` links. |
| `semantic.condition` (optional) | SEMANTIC 30 | 4 | `[Node(condition expression)]` | Check scalar `int`/`_Bool`; Complete | `Ack` | Optional fusion candidate; if omitted, the if/loop chips own the check. |
| `ir.statement` | IR_LOWER 25 | 5 | `[Node(statement), Block(current), ControlRegion?]` | Dispatch to `ir.if`/`ir.loop`/`ir.jump`/`ir.return`/V1 expr; await; Complete | `Record(Block(fallthrough))` if control can continue, else `Ack` | Frame context; one child at a time. |
| `ir.short_circuit` | IR_LOWER 19 | 5 | `[Node(logical), Block(current), Block(true_target), Block(false_target)]` | Emit lhs/rhs evaluation and `CondBranch`es; RHS only in the guarded block; Complete | `Ack` | Condition/branch context only; value context is O3/Unsupported. |
| `ir.if` | IR_LOWER 20 | 5 | `[Node(if), Block(current)]` | Tick 1: allocate then/else/join blocks (join only if an arm can fall through), append `ControlRegion` + `Progress`. Tick 2: lower condition/arms via `ir.statement`/`ir.short_circuit` (committed block/region IDs) -> append `CondBranch`/`Branch` terminators -> Complete | `Record(Block(join))` if fallthrough, else `Ack` | Context region; children in order condition, then, else; await-all. |
| `ir.loop` | IR_LOWER 21 | 5 | `[Node(loop), Block(current)]` | Tick 1: allocate header/body/step/exit blocks, append `ControlRegion` + `Progress`. Tick 2: lower clauses/body (committed block/region IDs) -> append back edges/terminators (`CondBranch` at the test, `Branch` for back edges) -> Complete | `Record(Block(exit))` | Context region; children in frozen order; await-all. |
| `ir.jump` | IR_LOWER 22 | 5 | `[Node(break/continue), Block(current), ControlRegion?]` | Read `JumpTargetRecord`; walk the committed `ControlRegion.parent` chain from the current region to the target construct; emit `Branch` to `break_target`/`continue_target`; Complete | `Ack` | No fallthrough; the parent omits the unreachable remainder (no dead block). |
| `ir.conditional` (optional) | IR_LOWER 23 | 5 | `[Node(conditional), Block(current)]` | Allocate then/else/join blocks, lower condition and arms, emit `CondBranch`/`Branch`; Complete | `Record(Block(join))` if fallthrough, else `Ack` | Only if O4 accepts `?:`; otherwise explicit `Unsupported`. |
| `ir.return` | IR_LOWER 26 | 5 | `[Node(return), Block(current)]` | Lower expression via V1; emit `Return`; Complete | `Ack` | No fallthrough. |
| `ir.function` (amended) | IR_LOWER 16 | 5 | `[Node(FunctionDefinition)]` | Tick 1: append `Function`+entry + `Progress`. Tick 2: lower body via `ir.statement`; enqueue `ir.function_end`; Complete | `Record(Function)` | Root frame; body child; await-all. |
| `ir.function_end` | IR_LOWER 24 | 5 | `[Function]` | Verify all live blocks terminated and all successors resolve; Complete | `Ack` | Activates the T01 `TerminatorMissing` hook (O9). |
| `verification.cfg_invariant` | VERIFICATION 21 | 6 | `[Function]` | Read-only VF07 check; Complete `Ack` or typed `Fail` | `Ack`/`Fail` | No writes; no children. |

Rules:

0. **Committed-only handoff.** A record appended by a task is passed to a child only after it is committed. A task that must pass a newly appended anchor (`NodeId`, `BlockId`, `ControlRegionId`, `FunctionId`) to a child inserts a `Progress` tick between the append and the child `Enqueue`; the child payload then contains only committed IDs. Same-batch predicted references are permitted only inside one task's own `AppendRecords` batch under the existing `/39` PA01 exception; they are never placed in an `Enqueue` payload, a `WaitSet`, a continuation, or a lowering frame. This is the rule that keeps the parent-before-children AST and block allocation legal.
   The "Tick 1 ... `Progress`" steps in the table are also where the task appends its resume frame and sets it through the O16 mechanism; a task must not enter tick 2 without a durable frame that names its new anchor and its next child ordinal.
1. A task that cannot finish in one tick appends its continuation, enqueues children, and emits `AwaitChildren`; children are visible next tick. No same-tick child execution.
2. Await-all is the only join policy: all committed children terminal before the parent decision; all `Completed` -> `Ready`; any `Failed` -> parent `Failed` exactly once, never `Ready` (T05/T02 accepted direction).
3. The join consumes no child result. The parent reads committed child `ResultRecord` values on resume and appends links/edges. If the integrator chooses to consume instead, consumption and the parent's append must be one atomic commit-visible unit; a failed commit must not consume (OPEN O11).
4. A new task enqueued in tick T is executable in tick T+1. Ordering remains `(phase priority, enqueue ordinal, TaskId)`; V2 adds no scheduler rule.
5. Statement lists and any multi-child construct must be bounded: bounded chunk size, `max_proposals_per_tick`, `stage_queue_bound`, `max_tasks_total`, `max_task_depth`, and `max_task_progress` apply. A resource exhaustion is a typed diagnostic, never a spin.
6. `parse.statement` and `ir.statement` are dispatchers only; they do not append a second node/block for the same statement. The selected production owns the record.
7. `ir.statement` owns the fallthrough rule: after a child that does not fall through (`ir.jump`, `ir.return`, or a terminating `ir.if` arm), it **stops lowering the remaining statements in that list**. The remaining AST/semantic facts stay checked but produce no live IR; no dead block is emitted. This is the A15 CFG-3 no-live-unreachable policy.
8. `ir.if` chooses its condition lowering by node kind: a `LogicalAnd`/`LogicalOr` condition goes to `ir.short_circuit` with the then/else targets; any other V2 condition is lowered by the V1 expression task to a value and consumed by `CondBranch`. No condition is silently folded or skipped.
9. Parent tasks append `NodeLink` rows only after the child results are committed. They never reference another task's draft, a wire, or an address.

#### 3.5.1 Continuation ownership gap (blocking protocol issue)

The current protocol has no way for a task to **attach or replace its own resume frame** after it has been enqueued:

- `Task.continuation` is `Option<ContinuationId>` and is fixed by `TaskDraft` at enqueue ([`task.rs`](../../../compiler/src/task.rs)).
- `TaskDraft.continuation` is still `Option<ContinuationId>`, not `ContinuationRef`; the note in `task.rs` says the `ContinuationRef`/own-batch migration is integrator-owned commit-track work.
- `Proposal` has `Enqueue`/`Complete`/`Fail`/`AwaitHost`/`StorePatch`/`AppendRecords`/`Progress`/`AwaitChildren`, but no "set/replace the current task's frame" transition.
- `LoweringFrameRecord` is a T09 frame, but `Task` has no field that can point at it.

Consequences: a V2 task that appends a continuation/frame, enqueues children, and then needs to resume with a **new** frame cannot record that frame. Re-deriving from a bus scan is forbidden (hidden full scan), and keeping all state in the original payload is impossible because payloads are immutable.

Acceptable resolutions (integrator must choose one in F1/F3):

| Option | Shape | Cost |
|---|---|---|
| A (recommended) | Add a tagged `FrameRef { Continuation(ContinuationId), Lowering(LoweringFrameId) }` and a `Proposal::SetFrame { task, frame: FrameRef }` transition (or equivalent task-owned frame update). The frame is committed before the task resumes. | One new proposal wire tag, Task/snapshot field, and commit validation; explicit and phase-neutral. |
| B | Complete the `TaskDraft.continuation`/`Task.continuation` migration to `ContinuationRef` (including `OwnBatch`) and add a same-commit frame update. | Reuses the existing `ContinuationRef` design; still needs a proposal/commit path and a lowering-frame variant. |
| C | Use a committed-frame **successor task**: the current task appends its frame, enqueues a successor task whose payload carries the committed frame, and completes. | No Task/proposal schema change, but doubles task kinds or payload shapes and must keep the task-tree depth bounded by setting the successor's parent to the construct's owning task, not the predecessor. |

The report's task tables assume option A unless the integrator selects B or C. This is a **freeze blocker**, not a chip-local choice.

### 3.6 Store, schema, allowlist, stage, and registration changes

| Store field | Record family | Owner | Proposed allowlist writer kinds | Stage |
|---|---|---|---|---|
| `parse.nodes` (existing) | `Node` | T05 | `parse.if`, `parse.loop`, `parse.jump`, `parse.block`, `parse.return`, `parse.tu` | 2 |
| `parse.node_links` (new) | `NodeLink` | T05 | `parse.if`, `parse.loop`, `parse.block`, `parse.tu` (only the parent task appends a link) | 2 |
| `sem.records` (existing) | `Sem` | T07 | `semantic.logical`, `semantic.if_stmt`, `semantic.loop_stmt`, `semantic.jump_stmt` | 4 |
| `types.records` (existing) | `Type` | T06 | V1 type producers plus `symbol_type.void_type` (unless V1 already provides it) | 3 |
| `sem.jump_targets` (new) | `JumpTarget` | T07 | `semantic.jump_stmt` | 4 |
| `ir.functions` (existing) | `Function` | T09 | `ir.function` | 5 |
| `ir.blocks` (existing) | `Block` | T09 | `ir.function`, `ir.if`, `ir.loop` | 5 |
| `ir.instructions` (existing) | `Instruction` | T09 | `ir.if`, `ir.loop`, `ir.jump`, `ir.return`, `ir.short_circuit`; `successors` is part of this record (primary) | 5 |
| `ir.values` (existing) | `Value` | T09/V1 | V1 expression-lowering tasks only; V2 control tasks produce no value except through V1 children | 5 |
| `ir.cfg_edges` (new; **fallback only**) | `CfgEdge` | T09 | only if the `InstructionRecord.successors` field append is rejected | 5 |
| `ir.control_regions` (new) | `ControlRegion` | T09 | `ir.if`, `ir.loop` | 5 |
| `ir.lowering_frames` (new) | `LoweringFrame` | T09 | `ir.function`, `ir.if`, `ir.loop`, `ir.statement`, `ir.return` | 5 |
| `verification.*` | none (read-only) | T13 | none | 6 |

Required registration work (integrator-owned):

1. Add `NodeLink`, `JumpTarget`, `ControlRegion`, and `LoweringFrame` to the ID/arena/family/wire-tag inventories (and `CfgEdge` only if the fallback edge family is chosen). Proposed wire tags 28-32 and family ordinals appended; exact numeric assignment is O15. Family ordinals and wire tags remain distinct inventories.
2. Add `G1DraftBody` variants and commit materialization for the chosen families, with structural validation and capacity preflight. Add `InstructionRecord.successors` (primary) to the existing instruction encoder; do not add a separate edge family at the same time. Families outside the closed set remain rejected.
3. Add `StoreSchema` fields for the new store fields (`parse.node_links`, `sem.jump_targets`, `ir.control_regions`, `ir.lowering_frames`; `ir.cfg_edges` only for the fallback) and seed them in the frozen schema builder. Post-seed `declare()` stays excluded from the seed hash; the new frozen fields must be in the seed.
4. Add a recognized V2 slice predicate (for example `is_v2_control_slice_kind`) to `manifest.rs` and one `STORE_OWNER_ALLOWLIST` row per (chip, store, field, kind) above. Without this, the wave-gated allowlist check passes unrecognized writers.
5. Add `STAGE_ASSIGNMENT` rows for every new kind. Registration fails `StageUnassigned` otherwise.
6. Add a cumulative registry builder (for example `v2_control_slice()` over the `/39` registry) and register every kind `Frozen`.
7. Add a non-empty `tests` list to every manifest; the current candidate `pa_stmt.rs` has an empty list and cannot register.
8. Assign `ChipId`s only in the integrator's registry. The uncommitted `PA_STMT_CANDIDATE_CHIP = ChipId(58)` must not be assumed; that draft is not registrable.

### 3.7 Result and error-carrier changes

No new `ResultValue` variant is proposed. The existing variants are sufficient:

| Result | Used for | Consumer rule |
|---|---|---|
| `Record(RecordRef::Node)` | parse node result; `parse.statement` forwarded result | Parent reads on resume; links only after committed. |
| `Record(RecordRef::Sem)` | semantic fact | VF06/T09 read. |
| `Records(vec![Sem, JumpTarget])` | semantic jump result | T09/VF07 read both in frozen order. |
| `Record(RecordRef::Block)` | IR fallthrough/join/exit block | Parent lowering frame uses it as the next current block. |
| `Record(RecordRef::Function)` | IR function shell | VF07/VF12/VF13 read. |
| `Ack` | pure checks (`ir.short_circuit`, `ir.jump`, `ir.return`, `ir.function_end`, VF07) | No fabricated value. |
| `Diagnostic`/`Fail` | structured errors | Exactly one transition per dispatched task. |

Error carriers:

- Language/recoverable faults are chip `Fail` diagnostics (`DiagnosticDraft`), not commit errors.
- Structural protocol faults (invalid child ref, invalid continuation ref, transition-free task, unpredicted record) are `CommitError`s and reject the whole batch before mutation.
- `TerminatorMissing` remains a commit-apply hook shape; V2 proposes extending it to all blocks (O9).
- Unsupported constructs are explicit `Unsupported` failures, never `Ack`, never a no-op instruction, never a silent skip.

---

## 4. Ownership and producer-consumer matrix

### 4.1 Single-writer matrix

| Artifact | Sole writer | Producers (task kinds) | Readers (phase) | Must not write |
|---|---|---|---|---|
| `NodeRecord` / `NodeLinkRecord` | T05 PA | `parse.tu`, `parse.block`, `parse.if`, `parse.loop`, `parse.jump`, `parse.return` | T06 (scope requests), T07 (semantics), T09 (lowering), VF05 | T06/T07/T09/VF05 |
| `ScopeRecord` / `ScopeEventRecord` | T06 TY | `symbol_type.scope_enter`, `symbol_type.scope_exit` | PA continuation, T07, VF05 | PA/SE/IR |
| `TypeRecord` | T06 TY | V1 type producers + `symbol_type.func_type` | PA/SE/IR/VF | PA/SE/IR |
| `SemRecord` | T07 SE | `semantic.*` | VF06, T09 | PA/IR/VF |
| `JumpTargetRecord` | T07 SE | `semantic.jump_stmt` | T09, VF07 | PA/IR/VF |
| `FunctionRecord` / `BlockRecord` / `ValueRecord` / `InstructionRecord` | T09 IR | `ir.function`, `ir.if`, `ir.loop`, `ir.statement`, `ir.return`, `ir.short_circuit` | VF07/VF08/VF12/VF13 | SE/PA/VF |
| `InstructionRecord.successors` (primary) / `CfgEdgeRecord` (fallback) | T09 IR | `ir.if`, `ir.loop`, `ir.jump`, `ir.short_circuit` | VF07/VF12 | SE/PA/VF |
| `ControlRegionRecord` | T09 IR | `ir.if`, `ir.loop` | `ir.jump`, VF07 | SE/PA/VF |
| `LoweringFrameRecord` | T09 IR | `ir.function`, `ir.if`, `ir.loop`, `ir.statement`, `ir.return` | T09 task continuation | SE/PA/VF |
| `Task`/`Result`/queues | T01/T02 commit path only | all chips propose | scheduler/commit/VF | any chip direct write |
| Diagnostics | T02 commit path | chips propose `Fail` | Host/VF | any chip direct write |
| `constants.records` | T08 CL | `constant_layout_init.const_fold` | T09/VF12 | T07/T09/VF |

### 4.2 Producer-consumer arcs (data/task dependencies, not calls)

```text
PA (AST + NodeLink + continuation)
  -> T06 scope_enter/scope_exit (compound/for scope)
  -> SE semantic.* (consumes committed AST + SemRecords of children)
       -> T08 const.evaluate (only for genuine constant-expression requests)
       -> T09 ir.* (consumes committed SemRecord + JumpTarget + AST)
            -> VF06 (typed facts, before lowering)
            -> VF07 (CFG after lowering)
            -> VF12 (covered-subset interpretation; no target execution)
            -> VF13 (replay comparison)
```

No arrow is a function call. Every arrow is a task request, a committed record reference, or a wire proposal. Chips never import or call each other.

### 4.3 Ownership conflict checks

1. `NodeRecord.children` and `NodeLinkRecord` are two representations of the same relation. Exactly one writer per node representation; VF05 rejects mixed use.
2. `SemRecord` + `JumpTargetRecord` are both T07-owned; no other chip may append either.
3. `ControlRegionRecord`, `LoweringFrameRecord`, and the terminator successor list (whether `InstructionRecord.successors` or the fallback `CfgEdgeRecord`) are T09-owned; VF07 is read-only.
4. T08 CL04 remains a const-only certification; it does not write any CFG, AST, or jump target.
5. The integrator owns `bus.rs`, `ids.rs`, `records.rs`, `task.rs`, `manifest.rs`, `contract.rs`, `snapshot.rs`, `commit.rs`, `mod.rs`, `Cargo`, and the version/hash. Chip owners never edit those files; if a field is missing, they return a structured DEFECT.

---

## 5. Control-flow legality, error, and recovery rules

### 5.1 Syntax and AST construction

| Rule ID (proposed) | Phase | Requirement | Failure |
|---|---|---|---|
| `parse.if-requires-parenthesized-condition` | PA | `if` must be followed by `(` expression `)` and a statement. | Typed parse diagnostic; PA38 recovery at `;`/`}`. |
| `parse.else-binds-nearest` | PA | An `else` binds to the nearest unmatched `if`. `if (a) if (b) s1; else s2;` produces `If(a, If(b, s1, s2))`, not `If(a, If(b, s1), s2)`. | No error; the parser must build the nearest binding. A verifier may reject a different shape. |
| `parse.if-no-else` | PA | An `if` without `else` has exactly two links (condition, then). | No synthetic else node. |
| `parse.while-requires-body` | PA | `while ( cond ) statement`. | Typed parse diagnostic. |
| `parse.do-while-terminator` | PA | `do statement while ( cond ) ;`; the trailing `;` is mandatory. | Typed parse diagnostic; do not synthesize `;`. |
| `parse.for-clause-grammar` | PA | `for ( init? ; cond? ; step? ) statement`; exactly two `;`; each clause may be empty. | Typed parse diagnostic; no guessed missing clause. |
| `parse.break-continue-syntax` | PA | `break ;` and `continue ;` are syntactically valid in any statement position. | Missing `;` is a parse diagnostic; the statement itself is not rejected for being outside a loop (semantic rule). |
| `parse.statement-progress` | PA | Every cursor advance must be finite; an empty statement or missing token must not loop. | `ParseCursorDidNotAdvance` typed diagnostic. |
| `parse.depth-bounded` | PA | Nested statement/expression frames are bounded by `max_task_depth`; check before enqueue. | `ParseDepthExceeded` typed diagnostic. |
| `parse.link-committed-only` | PA | `NodeLink` endpoints are committed IDs; only the parent task appends links after child results. | Commit/verification reject. |

Token-range rules:

- A parent's range contains every child's range; sibling ranges are ordered and non-overlapping (except zero-width empty statements, if adopted).
- `Break`/`Continue` ranges include the keyword and `;`.
- `If` range ends at the then/else statement end; `For` range ends at the body end; `DoWhile` ends at the trailing `;`.
- A leaf has `first == last`; `NodeLink` ordinals follow text order.

### 5.2 Semantic legality and `break`/`continue` target binding

| Rule ID (proposed) | Phase | Requirement | Failure |
|---|---|---|---|
| `sem.condition-scalar-int-v2` | SE | V2 conditions must have scalar integer type (`int`/`_Bool` after V1 promotions). Pointer/float/aggregate conditions are explicit `Unsupported` until V3. | Typed `Fail`; no lowering. |
| `sem.break-outside-loop` | SE | `break` must have an enclosing loop (V2) or switch (future). | Typed semantic error at the `Break` node. |
| `sem.continue-outside-loop` | SE | `continue` must have an enclosing loop. A switch alone does not make `continue` legal. | Typed semantic error at the `Continue` node. |
| `sem.jump-target-nearest` | SE | The target is the nearest enclosing construct of the applicable kind, determined by the committed AST parent chain. | `JumpTargetRecord` must name that node; VF07 re-checks. |
| `sem.break-target-kind` | SE | `Break` target is `While`/`DoWhile`/`For` in V2; `Switch` reserved. | Wrong target is a defect, not a warning. |
| `sem.continue-target-kind` | SE | `Continue` target is `While`/`DoWhile`/`For` only. | Wrong target is a defect. |
| `sem.jump-single-record` | SE | Exactly one `JumpTargetRecord` per `Break`/`Continue` site. | Duplicate/missing is a commit/verification failure. |
| `sem.statement-void` | SE | Control statement `SemRecord` has `ty = void`, `category = Void`; effects are the OR of child effects under the V1 mask (0 in V2 exit fixtures). | Typed `Fail` if a canonical void type is unavailable (V1 blocker). |
| `sem.logical-int-result` | SE | `&&`/`\|\|` produce `int`, `NonLvalue`; effects are the OR of operand effects under the V1 mask (0 in V2 exit fixtures). | Typed `Fail`. |
| `sem.short-circuit-order` | SE | The RHS of `&&`/`\|\|` is conditionally evaluated; no early RHS effect. | Fact recorded for IR; IR guard is mandatory. |
| `sem.unreachable-legal` | SE | Unreachable statements are legal and must still be checked; IR may omit them from live code after a proven terminator. | No diagnostic solely for unreachability; no live unreachable block. |
| `sem.constant-condition-no-prune` | SE | `if (0)`, `while (0)`, etc. do not delete the arm or loop; no constant-fold substitution for CFG. | Lowering must emit the explicit region; pruning is T10-only with legality checks. |
| `sem.effect-mask-v2` | SE | V2 adds no effect bit. `EffectMask(0)` is the V2 exit baseline; if V1 freezes `WritesLocal`, V2 may propagate it but must not invent a new class. | Nonzero outside the frozen V1 mask is explicit `Unsupported`, never a pass. |

Target-binding algorithm (deterministic, no hidden recursion):

1. Start at the jump site's committed `NodeRecord.parent`.
2. Walk upward over committed parent links. For `Continue`, stop at the first `While`/`DoWhile`/`For`. For `Break`, stop at the first loop (or `Switch` when added).
3. If no applicable ancestor exists, fail with `sem.break-outside-loop`/`sem.continue-outside-loop`.
4. Append `JumpTargetRecord { site, target, kind }` in the same batch as the site's `SemRecord`.
5. The IR lowerer reads the committed `JumpTargetRecord`, then walks the committed `ControlRegion.parent` chain from the current region to the region whose `construct` equals `target`. It reads the region's `break_target`/`continue_target`; it never re-parses tokens or scans the AST again.
6. The walk is bounded by `max_task_depth` and reads only committed `parent` fields. It never reads `children`, a draft, or an address. A memo/index is not required and must not be added as a hidden cache.

### 5.3 Short-circuit evaluation order

V2 exit gate is **condition/branch context**. The condition is consumed by `CondBranch`; no boolean value is materialized. The lowering sequence is frozen as follows:

`a && b` in condition context with true target `T` and false target `F`:

```text
current:      evaluate a -> va
              CondBranch va -> L_rhs (true), F (false)
L_rhs:        evaluate b -> vb      // only reachable when a != 0
              CondBranch vb -> T, F
```

`a || b`:

```text
current:      evaluate a -> va
              CondBranch va -> T, L_rhs
L_rhs:        evaluate b -> vb      // only reachable when a == 0
              CondBranch vb -> T, F
```

Rules:

1. The RHS block has exactly one predecessor in the simple case: the guard edge from the lhs. Nested short-circuits may add more; VF07 checks edge structure, not a semantic proof of exclusivity.
2. The RHS is never emitted in the current block before the guard. It must not be evaluated unconditionally and combined with a bitwise op.
3. The result of `&&`/`||` in condition context is represented only by the branch edges. No `int 0/1` value is required.
4. `!` (if V1 supports it) swaps the true/false targets rather than materializing a value.
5. Value-context `x = a && b;` / `return a && b;` requires a V1 slot/load/store or a join representation. V2 does not invent a phi. Until V1 freezes slots, value context is explicit `Unsupported` (O3). A constant false/true result without evaluating the RHS is not a legitimate substitute for general value context.
6. T08 `constant_layout_init.const_branch_and/or/cond` may certify a constant condition, but its `Ack` must not replace the CFG. In V2 the IR still emits both arms/edges.
7. Evaluation order of operands beyond `&&`/`||` is V1's sequencing rule; V2 preserves it and adds no unspecified reordering.

### 5.4 Unreachable and constant conditions

| Case | Required behavior |
|---|---|
| `if (0) s1; else s2;` | Emit condition evaluation, `CondBranch` (both successors), then/else blocks, join. Do not delete `s1`. |
| `if (1) s1;` | Same; both successors exist structurally; `s1` is not specially marked. |
| `while (0) s;` | Emit header, condition, `CondBranch`, body block, exit; the body edge exists and the block is well formed. Do not delete the body. |
| `for (;;) s;` | The omitted condition is materialized as a true constant and consumed by `CondBranch` to body/exit; both successors exist and exit is reachable. Do not delete the exit. |
| Statement after `return`/`break`/`continue` | It is still parsed and semantically checked, but it is **not lowered into live IR**; no dead block is emitted. Do not drop the AST/SemRecord facts. |
| Both `if` arms terminate | No join block and no fallthrough; the enclosing remainder is unreachable and is not lowered. |
| No `else` | The false successor goes directly to the join; no synthetic else node. |
| Constant expression evaluation | T08 may produce a committed `ConstRecord`; T09 may materialize it as a `Constant` value. T08/T09 must not rewrite the CFG based on its value. |

"Unreachable" is a graph property, not a parse/semantic error. Every **live** block must be reachable from entry; unreachable source statements are checked but omitted from IR. VF07 must reject a live unreachable block, a dangling successor, and a missing terminator.

### 5.5 Error, recovery, and unsupported rules

| Failure class | Carrier | Recovery |
|---|---|---|
| Missing `)`, `;`, `}`, malformed `for`/`do` | Chip `Fail` typed diagnostic | PA38 synchronizes to `;`/`)`/`}`/`{`-stop/EOF; no synthetic delimiter; parent fails; scope cleanup is an explicit T06 request. |
| `break`/`continue` outside loop | Chip `Fail` semantic diagnostic | No `JumpTargetRecord`; no IR; job diagnostic retained. |
| Non-scalar V2 condition | Chip `Fail`/`Unsupported` | No lowering; do not insert a truth conversion guessing the type. |
| Child task failed | Await-all fails the parent exactly once | No links/edges appended by the parent; partial committed records remain but the job is failed; no successful verification claim. |
| Structural child/continuation ref invalid | `CommitError` (`AwaitChildrenRefInvalid`/`ContinuationRefInvalid`) | Whole batch rejected before mutation; H6 bounded recovery transitions dispatched tasks to `Failed` once in dispatch order. |
| Unterminated block / dangling successor | `TerminatorMissing` (if hook extended) or VF07 `Fail` | No partial CFG accepted; task-scoped vs whole-batch rejection is O9. |
| Instruction appended after terminator | VF07 `Fail` (and lowering chip must prevent it) | No success; fix lowering; never silently drop the instruction. |
| Unsupported `switch`/`goto`/label/float condition/value-context logical | Explicit `Unsupported` | Stay in the denominator; never a no-op; never `Ack`. |
| Resource bound (`max_task_depth`, `max_tasks_total`, `max_proposals_per_tick`, `stage_queue_bound`, `max_task_progress`, `max_records_*`) | Structured `LimitError`/chip diagnostic | Fail the task or job per the frozen limit rule; no panic; no silent drop. |
| Non-advancing parser/lowering loop | `ParseCursorDidNotAdvance`/commit `NonAdvancingProgress` | Typed fail; no spin; bounded retry. |

Recovery invariants:

1. A failed parse subtree never receives parent links; a successful VF05 only runs on a successful job.
2. A failed semantic/lowering child fails the parent; no downstream records are fabricated.
3. A failed commit commits no semantic state; recovery transitions are the only post-failure mutation (H6 direction).
4. Replay/retry must not double-append links/edges or double-consume results. Parent resume is driven by a committed frame cursor; a retry re-runs from the same committed state and appends the same rows once.
5. No error path may turn an unsupported construct into a successful `Ack`.

### 5.6 Task nesting, await, and continuation rules

1. A parent appends its own AST/IR anchor record first (pre-order), then enqueues children and `AwaitChildren`. It appends links/edges only on resume after all children are terminal.
2. A parent's continuation records `production`, `cursor` (PA), `context`, `scope`, `parent`, `partial_children`, `next_child_ordinal`, `previous`. For IR, the equivalent resume state is `LoweringFrameRecord`.
3. Children are enqueued in a frozen order (§6.2). A parent that has more children than the configured queue/commit bounds uses bounded chunks with `Progress`; it does not enqueue an unbounded fan-out.
4. A child is visible next tick. The parent is `Waiting` in the meantime and is not selected.
5. Await-all: any failed child fails the parent once; no partial links/edges.
6. The join consumes no result. The parent reads committed child results on resume. If consumption is required, it is atomic with the parent's append (O11).
7. Depth is bounded by `max_task_depth`; the parser checks before enqueuing a descent, and the IR frame chain is bounded by the same limit. No Rust call-stack recursion.
8. A task with no transition is a protocol error (`TaskNotTransitioned`); a non-advancing `Progress` is `NonAdvancingProgress`; exceeding `max_task_progress` fails the task.
9. The scheduler order remains `(phase priority, enqueue ordinal, TaskId)`. V2 does not require quota > 1; quota 1 is the baseline.

### 5.7 Scope and label context

- **Scopes.** T06 owns `ScopeRecord`/`ScopeEventRecord`. V2 requires:
  - one block scope per `Compound` body (owner = the `Compound` node);
  - one `for` scope per `For` node when the init clause declares a name (owner = the `For` node), covering init/condition/step/body and closing after the body;
  - no scope for `If`/`While`/`DoWhile` themselves; their compound bodies own their scopes.
  The parser requests scope enter/exit via T06 tasks and carries the current `ScopeId` in the continuation. It does not write scope records.
- **Labels.** `break`/`continue` are **not** labels. V2 stores no label names and performs no label lookup. `goto`/labels are out of scope. The future `switch` target is reserved in `RegionKind`/`JumpKind` but not produced in V2.
- **Scope balance on error.** A failed compound/for must request the matching scope exit or fail the job; scope events remain append-only and exactly one Enter/at most one Exit (T06 rule). No silent scope leak.

---

### 5.8 Verification facts required for the V2 slice

| Fact ID | Verifier | Positive fact | Negative fact that must fail |
|---|---|---|---|
| V2-VF05-1 | VF05 | Every V2 node uses exactly one child representation (`children` or `NodeLink`); slots match the node kind; ranges contained. | Mixed representation, duplicate slot, child range outside parent, dangling link. |
| V2-VF06-1 | VF06 | Every checked control node has exactly one `SemRecord`; `If`/loop are `void`/`Void`; `&&`/`\|\|` are `int`/`NonLvalue`; `JumpTargetRecord` exists for every `Break`/`Continue`. | Missing SemRecord, wrong category/type, missing/duplicate jump target. |
| V2-VF07-1 | VF07 | Every live block reachable; exactly one terminator as greatest instruction; successor arity and target validity; targets same function; no live unreachable block. | Dangling successor, two terminators, instruction after terminator, cross-function target, unreachable live block. |
| V2-VF07-2 | VF07 | `Break`/`Continue` `JumpTargetRecord.target` is the nearest applicable ancestor; the target `ControlRegion` exists; the `Branch` reaches its `break_target`/`continue_target`. | Break outside loop, continue in switch-only context, wrong region, target block not the declared exit/continuation. |
| V2-VF12-1 | VF12 | A covered V2 function models `Constant`, `CondBranch`, `Branch`, `Return` with the frozen step budget and produces the expected Part A value. | Unsupported op, multi-block shape outside the covered subset, budget exhaustion treated as `Ack`, non-terminating loop. |
| V2-VF13-1 | VF13 | Same source/config/pins -> identical task graph, IDs, block order, successors, and canonical snapshot. | Divergence in block/edge order, double append, double consume, wall-clock/address dependence. |

These facts are requirements, not test results. A19 owns the executable independence and expected values.

## 6. Determinism, snapshot, and replay impact

### 6.1 Frozen block allocation order

Block IDs and ordinals are semantic: the same input/config must produce the same order. No hash-map iteration or address may decide allocation.

| Construct | Allocation order after the current block |
|---|---|
| `if` with else | then block, else block, join block |
| `if` without else | then block, join block |
| `while` | header is the current block; body block, exit block |
| `do ... while` | body is the current block; condition block, exit block |
| `for` | init is the current block (or condition if init is empty); condition block, body block, step block, exit block |
| `&&` in condition context | rhs block; short/false target is provided by the caller |
| `\|\|` in condition context | rhs block; short/true target is provided by the caller |
| unreachable statement after a terminator | no block; the statement is checked but not lowered |
| function | exactly one entry block; body blocks follow |

Within a block, instructions are in ascending `InstructionId`; a terminator is the last instruction. `CondBranch` successors are appended true then false; `Branch` has one successor; `Return` has none.

### 6.2 Frozen task/enqueue order

| Construct | Child enqueue order |
|---|---|
| `if` | condition, then, else (if present) |
| `while` | condition, body |
| `do ... while` | body, condition |
| `for` | init (if present), condition (if present), step (if present), body |
| `&&`/`\|\|` | lhs, rhs |
| compound | block items in source order, bounded chunks |
| function body | body statement, then `ir.function_end` |

Analysis order is deterministic but is not runtime evaluation order. Runtime order is encoded only by the CFG and the V1 sequencing facts.

### 6.3 Determinism rules

1. Stable IDs: arenas append; IDs are never reused; `u32::MAX` remains the no-record sentinel.
2. No hash-map iteration, address-keyed lookup, wall clock, randomness, filesystem, environment, or process access in chips.
3. Proposal/commit order remains `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal index)`.
4. New tasks are visible next tick; no same-tick re-entry.
5. Same initial state + config + pins -> identical task graph, record IDs, block/edge IDs, diagnostics order, and canonical snapshot.
6. Quota 1 is the correctness baseline. V2 does not authorize quota > 1, fusion, or parallel bus mutation.
7. Memory/record bounds are configured; exhaustion is a structured diagnostic, never a panic or silent drop.

### 6.4 Snapshot/hash impact

| Change | Snapshot/hash requirement |
|---|---|
| New `NodeKind`/`IrOp`/`ParseContext` members | Append wire names/discriminants; update `node_kind_name`/`parse_node_kind`, `ir_op_name`/`parse_ir_op`, continuation encoder/decoder. Unknown tags fail. |
| `NodeLinkRecord` | New typed arena body + encode/decode; arena count/live IDs in snapshot; field list in hash. |
| `JumpTargetRecord` | Same; `JumpKind` name list in hash. |
| `InstructionRecord.successors` (primary) / `CfgEdgeRecord` (fallback) | Encode the successor list in the instruction body (primary) or the fallback edge family; order is semantic and hash-participating. |
| `ControlRegionRecord` | Same; `RegionKind` name list + optional block IDs encoded. |
| `LoweringFrameRecord` | Same; frame chain and `next_child_ordinal` encoded. |
| New store fields | Add to the seed `StoreSchema`; post-seed runtime declarations remain excluded from the frozen hash (two-tier model). |
| New task kinds/stages/allowlist | Add to `STAGE_ASSIGNMENT`, registry slice, and allowlist; rule IDs and kind names participate in the contract hash. |
| New normative rules | Append to `NORMATIVE_RULES`; freeze test recomputes the hash; `/39` is preserved as history. |
| Proposed version | `t01-c01-c06/40` (or the integrator's next batch number); exact hash is integrator-computed, never invented here. |

`G1DraftBody`, `commit.rs` materialization, `Arenas::allocated_total`, `RECORD_KINDS`, `RecordFamily::ALL`, and `RecordRef` wire tags all need the new families. A new record family that is not materialized is rejected at commit by design; V2 must not ship a chip that cannot append.

### 6.5 Replay and verification impact

- VF13 compares the canonical semantic comparison projection (semantic records/results/diagnostics + semantic trace; scheduler-only registers excluded) and the deterministic scheduler snapshot separately.
- Replay fixtures must include: a normal nested if/loop, a failed child, a failed commit with H6 recovery, a join-only tick, a `Progress`-only tick, and a re-run of the same source. Expected: identical IDs, block order, edges, and verdict.
- A second consumption of a result must still return `ResultAlreadyConsumed`; a join must not consume.
- The `join_then_replay_retry` fixture (T13 H6-M15) is directly relevant to V2 parent resumes and must be part of the V2 verification set.
- No wall-clock, host path, or address may appear in the canonical projection.

---

## 7. Ordered freeze and implementation work orders

These are **candidate work orders**, not dispatch authorization. Each item becomes dispatchable only after its predecessor is accepted and the contract version is bumped. The integrator owns all shared files; chip owners edit only their assigned implementation files.

### 7.1 Freeze order

| Order | ID | Deliverable | Sole writer | Exact files | Exit criteria |
|---:|---|---|---|---|---|
| F0 | `F-V2-00` | V1 common-base gate (not V2-owned): general expression AST/Sem/IR expression task, locals/assignment, general return/expression statement, optional slots. | V1 integrator/owners | V1-owned files; no V2 edits. | V2 integration is BLOCKED until this is frozen or V2 explicitly restricts itself to condition-context/logical and V1 literals. |
| F1 | `F-V2-01` | AST + parse context + parse task kinds (`If`, loops, `Break`/`Continue`, `LogicalAnd`/`LogicalOr`, `NodeLink`, `ParseContext` 10-15, `parse.statement/if/loop/jump`, amended `parse.block/return/tu`) and the continuation/frame-ownership choice (O16). | T01 integrator | `compiler/src/bus.rs`, `compiler/src/ids.rs`, `compiler/src/records.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/src/snapshot.rs`, `compiler/tests/freeze.rs`; docs `docs/tasks/T05_PARSE_CHIPS.md`, `docs/tasks/T01_COMPILER_CONTRACT.md`. | Field tables §3.1-3.2 frozen; names/ordinals/tags fixed; O16 resolved; `freeze.rs` passes; no chip code. |
| F2 | `F-V2-02` | Semantic control facts: `JumpTargetRecord`, `JumpKind`, semantic kinds `logical/if_stmt/loop_stmt/jump_stmt` (+ optional `condition`), canonical void prerequisite, void/effect rules. | T01 integrator | Same shared files as F1 plus `docs/tasks/T07_SEMANTIC_CHIPS.md`, `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md`. | §3.3 frozen; one-writer rules; void producer or reuse; tests for target binding frozen. |
| F3 | `F-V2-03` | IR CFG: `Branch`/`CondBranch`, `InstructionRecord.successors` (or fallback `CfgEdge`), `ControlRegion`, `LoweringFrame` (through the O16 frame mechanism), block/terminator rules, amended `ir.function`, new `ir.*` kinds, `TerminatorMissing` scope decision. | T01 integrator | Same shared files plus `compiler/src/commit.rs` (hook shape) and `docs/tasks/T09_IR_LOWER_CHIPS.md`. | §3.4-3.5 frozen; block order §6.1; successor arity; LoweringFrame selectable by the task; hook decision O9 recorded; one edge carrier only. |
| F4 | `F-V2-04` | Verification: VF07 registration, VF05 link/kind extension, VF06 control facts, VF12 multi-block/step budget, VF01 arena inventory. | T01 integrator | `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/src/snapshot.rs`, `docs/tasks/T13_VERIFICATION_CHIPS.md`. | Verification fact table §5 and VF07 checks frozen; VF01 accepts the new arenas; no verifier writes. |
| F5 | `F-V2-05` | Version/hash/migration: `t01-c01-c06/40` (proposed), seed schema, allowlist rows, stage rows, registry slice, all name/field lists. | T01 integrator | `compiler/contracts/CONTRACT_VERSION`, `compiler/src/contract.rs`, `compiler/tests/freeze.rs`, `compiler/contracts/COMPILER_SFL_MANIFEST.md`, affected `/39` tests. | Hash recomputed; `/39` history preserved; no unregistered V2 kind; allowlist enforced. |

Freeze rules:

- Do not freeze a chip-local private type. If a needed field is absent, stop and amend the shared record.
- Do not add a new family without an ID, arena, family ordinal, wire tag, schema field, allowlist row, snapshot encoder, and commit materializer in the same freeze.
- Do not change `NodeKind`/`IrOp` discriminants; append only.
- Do not edit `CONTRACT_VERSION` outside F5.
- Do not use `/39` tests as proof of `/40`; update expected inventories.

### 7.2 Implementation order

| Order | ID | Deliverable | Owner | Exact owned paths | Forbidden paths | Acceptance |
|---:|---|---|---|---|---|---|
| I1 | `I-V2-PA-01` | Statement dispatcher, if, loop, jump parser chips; bounded statement-list blocks; node links. | PA implementer (one writer) | New files `compiler/src/chips/parse/pa_statement_v2.rs`, `pa_if_v2.rs`, `pa_loop_v2.rs`, `pa_jump_v2.rs` (or one new `pa_control_v2.rs` if the integrator fuses). Tests are A19-owned `compiler/tests/v2_parse.rs`; the chip owner must not edit expectations. Do **not** edit the uncommitted `pa_stmt.rs`. | `bus.rs`, `ids.rs`, `records.rs`, `task.rs`, `manifest.rs`, `contract.rs`, `snapshot.rs`, `commit.rs`, `parse/mod.rs`, `Cargo`, `CONTRACT_VERSION`, `pa_stmt.rs`, `compiler/tests/v2_*`. | Normal/boundary/invalid/replay tests; dangling-else AST; link reciprocity; no same-tick child; no unbounded fan-out. |
| I2 | `I-V2-PA-02` | Amend `pa_tu.rs`/`pa_block.rs`/`pa_return` behavior to the taskified V2 grammar, or retire them in the same freeze. | PA owner of the existing files (same writer as freeze) | `compiler/src/chips/parse/pa_tu.rs`, `pa_block.rs`; test updates `compiler/tests/c12_parse.rs`, `c34_parse.rs`, `c36_recovery.rs` as authorized. | All shared files; `pa_stmt.rs`; uncommitted drafts. | No duplicate Node producers; `/39` M1 tests either preserved under a legacy mode or explicitly superseded in the freeze record. |
| I3 | `I-V2-TY-01` | Compound and `for` scope entry/exit; `symbol_type.void_type` or reuse of a V1 canonical void. | TY implementer | New `compiler/src/chips/symbols/ty_control_scope.rs` (scope) and (if needed) a void producer in the assigned TY type file; A19-owned `compiler/tests/v2_sem.rs`. | Shared schema; existing `ty_scope.rs`/`ty_types.rs` unless the integrator assigns them; test expectations. | Scope lifecycle tests; one canonical void; no scope leak on child failure; one Enter/at most one Exit. |
| I4 | `I-V2-SE-01` | `semantic.logical`, `if_stmt`, `loop_stmt`, `jump_stmt`; jump target binding; condition scalar check; SemRecords. | SE implementer | New `compiler/src/chips/semantic/se_control.rs`; A19-owned `compiler/tests/v2_sem.rs`. | Shared schema; `se_function.rs` unless assigned; `SemRecord` fields; test expectations. | Normal/boundary/invalid/replay; break/continue outside loop; nearest target; no early RHS effect. |
| I5 | `I-V2-SE-02` | Amend SE29 function-definition check for control body (no direct `Return` child requirement) or add a V2 function checker. | SE owner assigned by integrator | `compiler/src/chips/semantic/se_function.rs` plus test updates; or new `se_function_control.rs`. | Shared schema. | A function whose body is if/loop/return checks successfully; no false `Unsupported`. |
| I6 | `I-V2-IR-01` | `ir.statement`, `ir.short_circuit`, `ir.if`, `ir.loop`, `ir.jump`, `ir.return`, `ir.function_end`; successor/region/frame representation; block order. | IR implementer | New `compiler/src/chips/ir_lower/ir_control.rs` (or per-kind files assigned by integrator); A19-owned `compiler/tests/v2_cfg.rs`. | Shared schema; `ir_function.rs` unless assigned; `mod.rs`; test expectations. | CFG golden shapes; one terminator per block; branch/loop/break/continue targets; no constant pruning; no live unreachable block; successor list matches the op. |
| I7 | `I-V2-IR-02` | Amend `ir_function.rs` to the V2 shell or retire it; keep one `FunctionRecord` producer. | IR owner assigned by integrator | `compiler/src/chips/ir_lower/ir_function.rs`; A19-owned `v2_cfg.rs`/`v2_e2e.rs` expectations. | Shared schema; test expectations. | Existing M1 fixture either still passes under the new shell or is explicitly superseded in the freeze record. |
| I8 | `I-V2-VF-01` | VF07 CFG verifier; VF05 link/kind extension; VF06 control facts; VF12 multi-block interpreter with step budget; VF01 arena inventory. | Verification implementer | New `compiler/src/chips/verify/vf_cfg.rs`; edits to `compiler/src/chips/verify/vf_syntax.rs`, `vf_invariant.rs`, `vf_interpret.rs`, `vf_store.rs`; A19-owned `v2_cfg.rs`/`v2_e2e.rs` expectations. | Shared schema; other verifier files unless assigned; test expectations. | Positive/negative facts; dangling successor, missing terminator, instruction after terminator, wrong break target all rejected; new arenas accepted; unsupported never `Ack`. |
| I9 | `I-V2-INT-01` | End-to-end Part A integration fixture and evidence: source -> PA -> TY -> SE -> IR -> VF05/VF06/VF07/VF12; replay. | Integrator-appointed integration owner | A19-owned `compiler/tests/v2_e2e.rs` and `compiler/tests/v2_support/`; no shared schema. | All shared files; `mod.rs`; Cargo; test expectations. | Real source produces a committed CFG and defined verdicts; no target-execution claim; unsupported target half reported as blocker. |

Implementation rules:

- One active writer per file. The integrator adds `mod.rs`/registration entries after the file is delivered and reviewed.
- Every chip is a zero-field `RestrictedChip`/`Worker` with a narrow projection; no bus mutation in `compute`; no cross-chip calls; no I/O.
- Every manifest has exact reads/writes, a non-empty test list, stage/layer agreement, and capability `Emulable`/`CpuReference` (non-target group rule).
- No test special-casing, no filename-based behavior, no GCC/Clang delegation, no success for unsupported.
- A worker that needs an unfrozen field returns a structured DEFECT instead of editing shared schema.

### 7.3 Verification commands for the eventual work orders

Run in `compiler/` (the repository is not one Cargo workspace):

```sh
cargo fmt --manifest-path compiler/Cargo.toml --all -- --check
cargo clippy --manifest-path compiler/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path compiler/Cargo.toml --test freeze
cargo test --manifest-path compiler/Cargo.toml --test v2_parse
cargo test --manifest-path compiler/Cargo.toml --test v2_sem
cargo test --manifest-path compiler/Cargo.toml --test v2_cfg
cargo test --manifest-path compiler/Cargo.toml --test v2_e2e
```

A19 owns these V2 test targets and their expected values. The implementation work orders must not edit the test expectations; they deliver code and let the independent test owner verify. If the integrator chooses different target names, update the work orders in the same freeze; do not invent a second test suite.

Do not run the whole suite for this report. When implementation lands, run the package-specific commands plus the full `compiler` suite at the integration batch. Report exact commands, results, blocked checks, and unrun checks. Root/torture commands are irrelevant to V2 Part A and must not be presented as V2 evidence.

### 7.4 Exit gates

A V2 freeze exit requires:

1. All §3 records/tasks/fields are in one integrator-owned version bump.
2. The name/field/tag/stage/allowlist inventories are unique and hash-covered.
3. VF05/VF06/VF07/VF12 have registered kinds and fixtures with negative cases.
4. `/39` history is preserved; no `/39` test is silently deleted.
5. No chip is registered without a manifest test path and a stage row.
6. The V2 integration fixture runs end to end on committed records.
7. The report of unrun target work is an explicit blocker, not a pass.

---

## 8. Open questions and rejected alternatives

### 8.1 Open questions (must be decided by the integrator/owner)

| ID | Question | Why it blocks | Recommended default |
|---|---|---|---|
| O1 | Is `NodeLinkRecord` accepted, or does the contract choose a different cross-task AST child carrier? | `NodeRecord.children` cannot be filled across task boundaries (C2). | Accept `NodeLinkRecord`; keep `/39` `children` only for batch trees. |
| O2 | What V1 common base exists for general expressions, locals, assignment, comparison IR, and slots? | V2 conditions and value-context logical need it; `ir.expr` does not exist today. | Freeze V1 first; V2 restricts to condition-context `&&`/`\|\|` until slots exist. |
| O3 | Is value-context `x = a && b` / `return a && b` in V2? | Requires slots or a non-SSA join representation. | Defer unless V1 slots are frozen; otherwise explicit `Unsupported`. |
| O4 | Is `?:` in the V2 exit gate? | It is short-circuit but not listed in the V2 one-line definition. | Optional extension; add `Conditional` only if V1's conditional expression is already available. |
| O5 | Does V2 extend `EffectMask` for side-effecting conditions? | `/39` allows only `EffectMask(0)`; A06 proposes bit 0 `WritesLocal`; sequencing tests may need effects. | Consume V1's `EffectMask` if frozen; V2 adds no new bit. Use `EffectMask(0)` fixtures for the V2 exit gate; side-effecting short-circuit is a V2 extension after V1's effect bits are frozen. |
| O6 | Is `for (int i = ...)` (for-declaration scope) in V2 or V1? | Needs V1 declarations and T06 For-owner scope. | V2 if V1 declares locals; otherwise V2.1. |
| O7 | Is `Switch` reserved in `RegionKind`/`JumpKind` now? | Avoids a later schema change for `break` in switch. | Reserve the values; produce no switch in V2. |
| O8 | Amend frozen `parse.block`/`parse.return`/`parse.tu` or add new V2 kinds? | Both writers would violate one-producer ownership if duplicated. | Amend the same kinds under a version bump, with `/39` behavior explicitly superseded. |
| O9 | Is `TerminatorMissing` a whole-batch reject or a task-scoped fail, and does it check every block or only entry? | Current `CommitError` is shape-only; T09 OB-26 open. | Task-scoped `FunctionEnd` fail for V2; VF07 checks all blocks; do not invent a whole-batch rollback. |
| O10 | Are `ControlRegionRecord` and `LoweringFrameRecord` separate, or one record? | They have different lifetimes and validation; merging risks mutable frame state. | Keep separate. |
| O11 | Does the parent consume child results or only read them? | `consume_result` is a separate mutable call; atomic consume+append is not implemented. | Read-only for structural children; if consumption is chosen, make it atomic with the append and fail atomically. |
| O12 | Are unreachable/dead blocks allowed, and what does VF07 require? | C permits unreachable statements; A15 forbids live unreachable blocks. | Adopt A15: no live unreachable block; check unreachable statements in AST/SE but omit them from IR. VF07 requires all live blocks reachable and well formed. |
| O13 | What is the statement-list chunk size / `Progress` policy? | Unbounded fan-out or per-statement `Progress` hits configured limits. | Bounded chunks; typed `ParseResourceExhausted` if the configured bounds cannot cover the input. |
| O14 | What is VF12's step budget for loops? | A naive interpreter can diverge on loops. | Frozen deterministic step budget; exhaustion is `Unsupported`/`BudgetExceeded`, never `Ack`. |
| O15 | Exact numeric tags, family ordinals, local codes, and chip IDs? | The hash and registries are integrator-owned; A06 claims some V1 locals. | Integrator assigns in the freeze batch; V2 starts above V1's SEMANTIC 22-25 and IR 17-18; this report only proposes relative order. |
| O16 | Which continuation/frame-ownership option (A/B/C in §3.5.1) is frozen? | Without it, no V2 task can resume with a new frame; this blocks all taskified PA/IR work. | Option A: tagged `FrameRef` + `Proposal::SetFrame`; fall back to B only if the `ContinuationRef` migration is already in the same freeze. |
| O17 | Does V2 keep A06's single-task `PARSE_TU` AST producer or adopt taskified PA/`NodeLink`? | The two architectures cannot both write `parse.nodes`; A06 avoids C15/NodeLink, this report needs them. | If the goal is the smallest V2 contract, extend A06's single task for V2 and drop `NodeLink`; if the goal is T05 taskification, adopt this report and retire the bootstrap parser. Decide once. |
| O18 | Does V2 adopt A15's `InstructionRecord.successors`/`Branch`/`CondBranch` and no-live-unreachable rule? | A15 is the IR owner report; a competing edge family/op naming would fragment the contract. | Adopt A15 as primary; keep this report's `CfgEdgeRecord` only as a fallback and its task table as functional requirements. |

### 8.2 Rejected alternatives

| ID | Alternative | Why rejected |
|---|---|---|
| R1 | Reuse T08 constant-branch chips to delete `if (0)`/`while (0)` arms and call it V2. | V2 explicitly requires real conditions and explicit CFG; constant interpretation must not impersonate control flow. |
| R2 | Fill `NodeRecord.children` from child tasks after the fact. | A record is append-only; `StorePatch` carries `RecordRef`, not a new record body; parent/child ordering cycle. |
| R3 | Build the whole AST with one chip per function using Rust recursion. | Violates no-hidden-recursion/single-responsibility and the guardrails; deep nesting must use explicit frames. |
| R4 | Use SSA/phi for joins. | The baseline is non-SSA; T10 must not assume SSA; V1 slots are the intended join representation. |
| R5 | Add a **third** edge representation (for example both `InstructionRecord.successors` and `CfgEdgeRecord`) to cover both drafts. | Two edge sources can disagree and break replay. Choose one: A15's `successors` primary, or the `CfgEdgeRecord` fallback, not both. |
| R6 | Reuse `ContinuationRecord` for IR lowering. | It is parse-specific (`cursor: TokenId`, `ParseContext`, `binding_power`); using it for IR conflates phases and adds nullable fields. |
| R7 | Re-parse tokens in SE/IR to find loop bodies. | Consumes committed AST; re-parsing duplicates producers and can diverge. |
| R8 | Scan the AST parent chain in T09 to find break/continue blocks. | Works for legality but not for CFG target choice; a committed `JumpTargetRecord` + `ControlRegionRecord` is explicit and verifiable. |
| R9 | Rewrite `break`/`continue` as `goto` labels at parse time. | Labels/goto are out of V2; it also changes the AST and hides the binding rule. |
| R10 | Whole-function single-chip IR lowering with an internal worklist. | Hides the control-flow phase in one helper; violates modularity/fusion rules. |
| R11 | Treat `break`/`continue` as always-valid and defer all checks to codegen. | The semantic phase must reject constraint violations; downstream must not guess. |
| R12 | Encode `NodeKind`/`IrOp` numeric tags by arithmetic on family ordinals. | Family ordinals and wire tags are separate inventories; explicit mapping is required. |

### 8.3 Proposed-but-not-decided items that must not leak into code

- Exact `ChipId` assignments, PARSE/SEMANTIC/IR/VERIFICATION local codes, and wire tags (O15).
- Exact `NodeLink`/`JumpTarget`/`ControlRegion`/`LoweringFrame` wire encoding and hash spellings, and the primary `InstructionRecord.successors` vs fallback `CfgEdgeRecord` choice (O18).
- Exact `ParseContext`/`RegionKind`/`JumpKind` numeric tags.
- Whether `semantic.condition` is a separate chip or fused into `semantic.if_stmt`/`loop_stmt`.
- Whether `ir.statement` is one dispatcher or split into one chip per statement kind.
- The V2 chunk size and VF12 step budget.
- The final decision on `?:` and value-context logical.

---

## Appendix A. Independent V2 fixture facts (design only; not run)

These are expected-value facts for the future independent tester. They are derived from the C rules and the proposed contract, not from any implementation. **A19 (`A19_V2_INDEPENDENT_TESTS.md`) owns the actual test files, harness, and expected values;** this table is a requirement/fact input and must not be used to claim a test result.

| Fact | Fixture | Expected verdict/fact |
|---|---|---|
| F-PA-1 | `if (a) if (b) s1; else s2;` | `Else` binds to the inner `If`; AST has `If(a, If(b, s1, s2))`; no outer else. |
| F-PA-2 | `if (a) s1;` | `If` has exactly two links; no synthetic else. |
| F-PA-3 | `while (c) s; do s; while (c); for (;;) s;` | Child ordinals match the table in §3.1.1; `do` range includes the trailing `;`; `for` has an empty init/cond/step. |
| F-PA-4 | `while (c) { break; continue; }` | `Break`/`Continue` are leaves; no label name; parent is the `Compound`/loop as appropriate. |
| F-SE-1 | `int main(void){ break; }` | Typed semantic failure: break outside loop. |
| F-SE-2 | `int main(void){ continue; }` | Typed semantic failure: continue outside loop. |
| F-SE-3 | `while (a) { while (b) { break; continue; } }` | `JumpTargetRecord.target` is the inner loop for both. |
| F-SE-4 | `if (p) s;` with pointer `p` | V2 explicit `Unsupported`; no guessed truth conversion. |
| F-SE-5 | `if (1) s;` | No CFG pruning; both branch edges are present. |
| F-IR-1 | any V2 function | Every block has exactly one terminator as its greatest instruction. |
| F-IR-2 | `if (c) s1; else s2;` | Then/else blocks each end in `Branch` to the join; the condition block ends in `CondBranch` with successors 0/1. |
| F-IR-3 | `while (c) s;` | Body has a back edge to the header; exit is the break target; continue target is the header. |
| F-IR-4 | `for (i=0; i<n; i=i+1) { continue; }` | Continue target is the step block; break target is the exit. |
| F-IR-5 | `do { continue; } while (c);` | Continue target is the condition block. |
| F-IR-6 | `if (0) s;` | True edge/block exists and is well formed; no deletion. |
| F-IR-7 | `if (a && b) s;` | RHS `b` is in a block reachable only through the lhs true edge; no unconditional evaluation. |
| F-IR-8 | `return 1; x = 2;` | `x = 2` is parsed and checked but not lowered; no live unreachable block; no instruction is appended to the terminated block. |
| F-VF-1 | dangling successor (`successors` target) or `CfgEdgeRecord.to` in the fallback | VF07 `Fail`; no pass. |
| F-VF-2 | instruction after terminator | VF07 `Fail`; no pass. |
| F-VF-3 | wrong `break` target | VF07/VF06 `Fail`; no pass. |
| F-VF-4 | `NodeLink` duplicate/mixed with `children` | VF05 `Fail`; no pass. |
| F-VF-5 | multi-block function at VF12 | Either a correct covered-subset model or `Unsupported`; never `Ack` as if M1. |
| F-REP-1 | same source twice | Identical record/block/edge IDs, task graph, snapshot; no wall-clock/address dependence. |
| F-REP-2 | failed commit then retry | No partial semantic state; links/edges appended once; results not double-consumed. |

## Appendix B. Integrator checklist

- [ ] Accept or reject `NodeLinkRecord` (O1) and the `NodeRecord.children` compatibility rule.
- [ ] Confirm the V1 common base and the V2 dependency gate (O2/O3).
- [ ] Confirm `?:` and value-context logical scope (O3/O4).
- [ ] Confirm `EffectMask(0)` for V2 or extend it (O5).
- [ ] Confirm for-declaration scope and switch reservation (O6/O7).
- [ ] Decide amend-vs-add for `parse.block`/`parse.return`/`parse.tu` (O8).
- [ ] Decide the `TerminatorMissing` scope and error carrier (O9).
- [ ] Confirm region/frame separation (O10) and result-consumption policy (O11).
- [ ] Confirm dead-block/reachability policy (O12), chunking/progress (O13), VF12 budget (O14).
- [ ] Assign exact IDs/tags/local codes and bump the contract version/hash (O15).
- [ ] Add all new families to `G1DraftBody`, arenas, schema, allowlist, stage rows, snapshot, hash, and freeze tests in one batch.
- [ ] Preserve `/39` history and do not claim V2 implementation or target execution from this report.

---

**Report end. Status: PROPOSED. No source, test, contract, Cargo, `mod.rs`, shared index, or user work was modified. No commit or push was performed.**
