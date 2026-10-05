# Gate 1: M1 First-Slice Freeze Checklist (const-fold chain)

| Field | Value |
|---|---|
| Status | **EXECUTED 2026-10-06** — frozen as `t01-c01-c06/7` (hash `a56de65b…89d5c`); branch `feat/gate1-const-fold` |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | Each item names its decider; only the named authority signs |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/6` foundation + `/7` delta), [M1 Part A proposal rev 40](M1_PART_A_CONTRACT_PROPOSAL.md), [CDR rev 56](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md) |
| Non-goal | Parse/return chain, Part B, quota>1, full C vocabularies |

## 0. Decision record (user, 2026-10-06)

- **Version:** new amendment `/7` (hash move can never stay `/6`).
- **`ConstRecord` carrier:** magnitude bytes (`value: Vec<u8>` big-endian
  + `negative: bool`), mirroring `LiteralRecord`.
- **`Lx08CandidateType`:** M1-closed enum (`{Int}`); future categories
  append members (and bump the version).
- **Envelope (T01 integration):** no new `ResultValue` variant —
  `Legal` completes with `Record`, non-legal fails with a structured
  diagnostic (implements the already-hashed `/6` rules
  `const.legal-completes-record`, `const.non-legal-fails-no-record`,
  `const.one-record-per-request`).
- **`required_kind`/`op`:** implied by the M1 slice kinds, zero wire
  bytes; future purposes need new kinds/variants.

## 1. What Gate 1 is

Gate 1 freezes the **smallest closed loop that proves typed records can
flow through the protocol**: the M1-CL-05 const-fold chain
(T07 → T08 → T09) for `2 + 3`:

```text
committed LiteralRecord(2), LiteralRecord(3),                    [frozen G1 fixtures,
committed BinaryExpression node, checked Add fact                  not chip outputs]
  → ConstantRequest::Literal × 2, then Binary{Add}   [T07 request shape]
  → exactly one ConstRecord(5), folded with checked addition   [T08]
  → T09 Constant op consuming RecordRef::Const without re-folding
```

Upstream production (T04 literal-commit chips, T05 nodes, T07 checked
facts, T09 `Return`) is **not** Gate 1: those need the token→literal
link, `ParseContext`, `SemRecord`, and the `FunctionEnd` hook, all
still open. Gate 1 seeds the boxed inputs as frozen committed records
so the typed chain is testable without inventing those mechanisms.
The acceptance fixture is **`G1-CL-01`** (defined below), explicitly
**not** `M1-CL-05`: `M1-CL-05` requires real upstream artifacts and is
the Wave-2 acceptance ([CDR §9A Gate 2](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md):
fixtures do not substitute for upstream contracts).

No language-chip code is written before every item below is frozen.
After the freeze, Wave 1 dispatches **only** the slice chips
([PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md)).

## 2. Freeze items

| # | Item | Decider | Acceptance |
|---|---|---|---|
| G1-1 | `LiteralRecord` exact 8 ordered fields (`token, kind, radix, suffix, value, negative, spelling, candidate_type`) in code, replacing `ReservedArena<LiteralId>` | T01 `[INT]` after T04/T08 owners | Struct + snapshot `encode_*` round-trip; field order pinned by test |
| G1-2 | M1 enums: `LiteralKind{Integer,Character,String}` (only `Integer` produced), `LiteralSuffix{None,U,L,UL,LL,ULL}` (only `None` produced), radix domain `{2,8,10,16}` (M1 decimal only) | T01 `[INT]` after T04/T08 | Closed enums in code; out-of-subset input → explicit unsupported diagnostic, never a silent default (CDR rev 53) |
| G1-3 | `Lx08CandidateType` M1 scope: closed `{Int}`, target-independent, no bit width | T01 `[INT]` after T04/T08 | M1 literals `2`/`3` carry `Int`; complete future member set explicitly **not** frozen |
| G1-4 | `ConstantRequest::{Literal,Binary}` variants + `ConstExprOp` (M1: `Add`) + `RequiredKind` (M1: `IntegerConstantExpression`) | T01/T07/T08 co-freeze (OPEN-03) | Committed-input path only (committed `NodeId`, committed literal refs in source order); `G1-CL-01` green (`M1-CL-05` stays the Wave-2 acceptance on real upstream artifacts) |
| G1-5 | `ConstRecord` value carrier (rev-54 left formula/carrier open: `i128` superseded) + `ConstantResult{value, legality}` envelope carrier (open per rev 55) + `ConstLegality{Legal,NotConstantExpression,Unsupported}` | T01 `[INT]` after T08 | Exactly one `ConstRecord` per fold; `ConstRecord` identity/reuse rule stated |
| G1-6 | Slice task kinds registered (T07 const-request kind, T08 const-evaluate kind, T09 const-emit kind) with exact names/codes; T04 production kinds deferred to slice 2 | T01 `[INT]` after group owners | `TaskKindRegistry` entries + `KindStatus` transitions covered by test |
| G1-7 | kind→stage rows for the slice kinds; `check_stage_assignment` enforces **these kinds** (global turn-on is a separate decision) | T01 `[INT]` + T02 | `StageUnassigned` reachable and tested for slice kinds; no behavior change for existing kinds |
| G1-8 | `STORE_OWNER_ALLOWLIST` rows for the slice chips (chip IDs assigned by T01; `tasks.ready` stays zero-writer) | T01 `[INT]` | Sole-writer violation test for each slice store/field |
| G1-9 | `AppendRecords` materialization for `Literal` + `Const` families only (all other families keep the explicit rejection) | T01 `[INT]` | Reserve/resolve/materialize round-trip; cycle-safe reservation per the T04 open item or a stated restriction |
| G1-10 | Snapshot bodies for the new records + freeze-test coverage (seed self-consistency, kind/stage/allowlist coverage, negative tests) | T01 `[INT]` | `cargo test` green; replay byte-identical |

## 3. Version rule

If any G1 item changes the hashed seed, the result is a **new
amendment version** (e.g. `/7`), not `/6` with the same hash. The
two-tier rule (CDR rev 50) still applies: the frozen seed participates
in the hash, post-seed runtime declarations do not. The version bump
decision belongs to the user on T01's proposal.

## 4. Explicitly deferred

- T04 token→literal production link (needs the cycle-safe ID
  reservation; slice 2, on real source text).
- T05 nodes, T07 checked facts, T09 `Return` emission and the
  `FunctionEnd` hook (slice 3; `M1-CL-05` covers them at Wave 2).
- T05/T06/T07 parse→symbol→`return` chain and T09 `Return`/`Add`/
  `FunctionEnd` emission (slice 3).
- `ParseContext` final encoding, `ConversionOp`/`ConversionRole`
  inventory, VF06 exact kind/stage, target widths, Part B substrate.
- quota>1 paths stay measured post-freeze work; Gate 1 runs quota 1.

## 5. Done means

All G1-1–G1-10 accepted and signed, version bumped if the hash moved,
and the `G1-CL-01` fixture runs end-to-end on frozen types. Only then do
the slice chips get dispatched to Wave 1 authors.

## 6. Execution record (2026-10-06)

All items executed by the T01 integrator on branch
`feat/gate1-const-fold`, verified by `compiler/tests/c08_gate1.rs`
(11 tests: G1-CL-01 chain, decode/routing conventions, materialization
negatives, capacity, stage/allowlist/layer gates, snapshot replay,
schema/hash presence):

- G1-1–G1-5: `LiteralRecord`/`ConstRecord` typed arenas in `bus.rs`;
  M1-closed enums (`LiteralKind`, `LiteralSuffix`, `{Int}`, `{Add}`,
  `{IntegerConstantExpression}`, `ConstLegality`); `ConstantRequest`
  decode + `ConstantResult::route` in `task.rs`; `G1DraftBody` in
  `records.rs`; body encoders in `snapshot.rs`.
- G1-6: three slice kinds (`semantic.const_eval_literal`,
  `semantic.const_eval_binary`, `constant_layout_init.const_fold`),
  all `Frozen`, in `TaskKindRegistry::m1_slice()`.
- G1-7: `STAGE_ASSIGNMENT` (control 0, request 1, fold 2);
  `check_stage_assignment` enforces every claimed kind;
  `check_stage_layer_agreement` covers routed kinds.
- G1-8: one allowlist row (`G1_FOLD_CHIP`, `Constants/records`,
  `const_fold`); enforcement wave-gated to slice kinds.
- G1-9: `AppendRecords` materializes `Literal`/`Const` bodies 1:1
  with handles (M1-subset enforced, other families keep the explicit
  rejection); per-arena capacity preflight; infallible apply with
  `CommitReport::appended`.
- G1-10: `StoreSchema::m1_slice()`, snapshot bodies, freeze-test
  inventory pins.
- Version: `t01-c01-c06/7`, hash
  `a56de65b153e0e5dc1bec24040f2bdfcb95ead76efd0fd1f5661e40524589d5c`
  (`compiler/contracts/CONTRACT_VERSION`).

Wave 1 (slice chips per `TASK_TEMPLATE`) is unblocked on these types;
`M1-CL-05` on real upstream artifacts remains the Wave-2 acceptance.

## 7. Phase 0 addenda (Wave 1 template, branch `feat/wave1-fold-chip`)

- `ConstantRequest::decode` accepts all three slice kinds and dispatches
  on payload shape (single literal → `Literal`, node + two literals →
  `Binary`): the T07 requester forwards identical payload refs to its
  `const_fold` child, so the fold worker sees one convention.
- Predicted-ID rule (frozen): a worker's Nth body of family F gets
  `arena.allocated() + earlier F-bodies in apply order`; the commit
  verifies every future-dated `Complete` reference
  (`CommitError::UnpredictedRecord`, Protocol 17) instead of completing
  with a wrong reference. Enforcement of existing rules — no hash change.
- Worker template: `compiler/src/chips/` (`Worker` trait, host
  `WorkerRegistry` + `drive_task`, `FoldChip` first chip). The routing
  shell never invokes workers; the host drives, the commit decides.
