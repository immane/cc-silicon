# A23 — Determinism, Replay, Canonical Encoding, and Invariants Review

**Status:** Complete — read-only review. No source, test, contract, Cargo, `mod.rs`, shared schema, routing table, or index file was changed. Only this report was created.
**Date:** 2026-11-07
**Audited revision:** `430ae29162efef90f86667f384097964d611604f` (`wave4-development`, 2026-11-07)
**Contract baseline:** `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` (`compiler/contracts/CONTRACT_VERSION`)
**Target state:** identity `aarch64-unknown-linux-gnu` / ELF / LP64 / little-endian / AAPCS64; all concrete values remain `target_verification=unverified`; no probe has run.

**Workspace state at audit time:** dirty, preserving user work. Relevant files were read, not modified. The pre-existing uncommitted state was:

- modified: `compiler/src/chips/parse/mod.rs`;
- untracked candidate chips under `compiler/src/chips/lex/` and `compiler/src/chips/parse/`;
- untracked planning/freeze drafts under `docs/tasks/`, including `CT_FREEZE_01..03`, `OP_FREEZE_01..03`, `VF13_FREEZE.md`, `DISPATCH_LEDGER.md`;
- untracked review directory `docs/reviews/dispatch/`.

No foundation file (`snapshot.rs`, `codec.rs`, `contract.rs`, `ids.rs`, `task.rs`, `commit.rs`, `routing.rs`, `bus.rs`, `manifest.rs`, `limits.rs`, `target.rs`, `intern.rs`, `arena.rs`) was modified by this review.

---

## 1. Executive assessment

`/39` has a deterministic core: IDs are dense newtypes, arenas are append-only, the normal scheduler/commit path uses explicit vectors/BTree maps rather than hash-map iteration, quota-1 selection is a stable three-key order, commit validation is preflighted before mutation, and two fresh runs of the same fixture usually produce equal snapshot bytes.

It does **not** yet establish the stronger claims needed for a Stage-B replay contract:

1. **The canonical snapshot is not complete.** A pre-commit `AppendRecords` proposal encodes only its draft count; two different append batches with the same count produce identical snapshot bytes. The normative rule `snapshot.wire-payloads-encoded` is therefore false for the largest new wire arm.
2. **The replay evidence is re-execution, not restoration.** `Snapshot` has no decoder/restore API, `Trace` has no reader/diff, and existing "replay" tests construct two fresh buses and run the same scenario twice. No test proves that a serialized snapshot can be restored and continued, or that the same trace corresponds to the same pins.
3. **Quota-1 determination is internally consistent, but quota>1 is not auditable.** `max_inflight_per_tick` is publicly configurable; when it is `>1`, the dispatcher runs a batch, while `TickReport`, `TickRecord`, `TickMetrics`, and `Trace` all project only the first selected task. There is no canonical semantic projection for quota-1 vs quota>1 comparison, and no test executes quota>1 at all.
4. **Store version rules are incomplete.** `store_versions` is bumped only for `StorePatch`, not for typed `AppendRecords`; the `Names` comment claims a bump that never happens; `ResultRecord.version` is always `0`; and `StorePatch`/tombstone writes are logged but never materialized.
5. **The scheduler does not consume `STAGE_ASSIGNMENT`.** T01 §4 and `task.rs` define a three-key order `(phase priority, enqueue ordinal, TaskId)`, while M1 target acceptance D5 defines `(stage ordinal, phase priority, enqueue ordinal, TaskId)`. The implemented "phase priority" is the task-group ordinal, not the stage ordinal. The two documents conflict before Stage B.
6. **Limit accounting is not closed.** `Arenas::allocated_total()` omits existing macro records, so `max_records_total` undercounts them; `stage_queue_bound` is validated/hashed/snapshotted but never enforced; a zero quota can be silently clamped to one when the infallible public constructor is used.
7. **Reference liveness is weaker than the rule names suggest.** `vf01.refs-resolve` checks `index < allocated`, not `is_live`; tombstoned IDs pass. `TaskDraft.continuation` is accepted without checking that the continuation exists.
8. **Contract vocabulary and hash coverage have gaps.** `SYMBOL_KIND_NAMES` says `"variable"` while the snapshot codec writes/reads `"object"`; `ParseContext` ordinal semantics and later-wave task-kind names are not in the frozen hash.

---

## 2. Claim-vs-evidence table

| # | Claim / requirement | Code and test evidence | Verdict |
|---|---|---|---|
| 1 | "A snapshot encodes every observable store; canonical bytes; addresses and private caches never participate" — `compiler/src/snapshot.rs:4-16`, `:520-878` | `Snapshot::capture` encodes every arena plus most control/task/wire state, but `AppendRecords` is count-only (`snapshot.rs:446-455`), and `config` is represented only by `config_hash` (`:548`). No full-snapshot decoder exists. | **Partial / overstated.** Wire bytes are incomplete and the artifact is not restorable. |
| 2 | "Replay with identical input, configuration, and initial state is consistent per tick" — T01 §7.1 C05 (`T01_COMPILER_CONTRACT.md:138-139`) | `c05_codec.rs:101-112`, `c08_gate1.rs:748-800`, `c17_vf12.rs:653-667`, `c19_vf01.rs:626-632` all run the scenario twice from freshly constructed buses; none decodes/restores a snapshot. | **Run-to-run determinism tested; replay/restore not tested.** Pins are not recorded in snapshot or trace. |
| 3 | "Trace records one entry per tick" — `routing.rs:596-601` | `clock_tick` records only when `propagate` returns `Ok` (`routing.rs:627-633`); error paths at `:342-366` and `:371-376` skip `record_tick`, but `latch`/`advance_tick` still run (`:631-633`). | **False on governed error paths.** |
| 4 | "Selection order is `(phase priority, enqueue ordinal, TaskId)`" — T01 §4 (`T01_COMPILER_CONTRACT.md:78`), `task.rs:1783-1789` | `RoutingShell::phase_priority` returns `kind.group().raw()` (`routing.rs:34-38`); `select_batch` sorts `(phase, enqueue, id)` (`:223-243`). | **True for the T01 three-key order.** |
| 5 | "Selection order is `(stage ordinal, phase priority, enqueue ordinal, TaskId)`" — `M1_TARGET_ACCEPTANCE.md:369` (D5) | `STAGE_ASSIGNMENT` exists (`manifest.rs:1444-1514`), but `schedule_key` has no stage component (`task.rs:1783-1789`); `stage_of` is used only for manifest/worker-layer checks. | **False against D5; two contract orders conflict.** |
| 6 | "Quota 1 is the baseline equivalence mode; quota >1 is a measured post-freeze change" — `limits.rs:54-60`, `COMPILER_DEVELOPMENT_GUARDRAILS.md:187-198` | `Limits::try_new` accepts `max_inflight_per_tick > 1` (`limits.rs:115-151`); `propagate_inner` dispatches the whole selected batch (`routing.rs:340-405`). A quota-2 configuration can run today. | **Implementation is reachable ahead of the stated authority; no equivalence test.** |
| 7 | "New tasks are visible next tick" — T01 §4, `task.rs:12-15` | `push_task_reserved` uses `tick+1`; `reinsert_ready` uses `tick+1` (`commit.rs:1445`, `:2303-2310`); `select_batch` filters `ready_tick > tick` (`routing.rs:230-231`). | **True in normal operation**; wrap at `u64::MAX` is theoretical. |
| 8 | "Completion and result consumption are exactly once" — T01 §4, `commit.rs:564-580`, `:2091-2102` | `completing`/`completed_by_batch`/`failed_by_batch` enforce one transition; `consume_result` marks `consumed` before returning. | **True for the commit API.** Join does not consume results by design. |
| 9 | "Store version guard rejects stale patches" — `contract.rs:82`, `commit.rs:1898-1905` | `store_versions` is bumped only for stores touched by `StorePatch` (`commit.rs:1712-1718`). Typed appends (`:1493-1658`) never insert into `touched_stores`; `intern_name` does not bump `Names`. | **False for typed appends / names.** |
| 10 | "Tombstoned IDs never alias a new record; deletion leaves a tombstone" — `arena.rs:6-16`, `:192-215` | `Arena::remove` is implemented and tested (`c01_arena.rs:16-19`), but `commit` never calls it; `StorePatch::Tombstone` only validates shape and appends to `patch_log` (`commit.rs:1700-1713`, `:1906-1922`). | **Arena primitive true; commit-level tombstone semantics unproven / not materialized.** |
| 11 | "`vf01.refs-resolve`: task payload/parent/continuation, result, and span references resolve" — `contract.rs:177-179` | `Vf01Chip::compute` checks `index < allocated` (`chips/verify/vf_store.rs:209-246`, `:346-360`, `:379-386`); no `is_live` / tombstone check. `TaskDraft.continuation` is not checked on Enqueue (`commit.rs:823-862`). | **Partial: allocated-bound check only, not liveness.** |
| 12 | "AppendRecords materializes 1:1 bodies; at most one batch per task" — `commit.rs:981-1197` | Commit validates body/handle length and family, and appends bodies (`:1493-1658`). | **True for commit materialization.** Snapshot wire encoding still omits the bodies (finding F01). |
| 13 | "Every configured limit is enforced" — `contract.rs:63` | `Arenas::allocated_total()` omits `macros` (`bus.rs:679-707`); `stage_queue_bound` has no enforcement call (`commit.rs:1425-1438` uses `max_queue_len`, stage 0); `dispatch_quota` clamps zero to one (`routing.rs:251-253`). | **False in at least three places.** |
| 14 | "No hash-map iteration/address dependence" — T01 §1, ADR-0001 §2.6 | No `HashMap`/`HashSet` in `compiler/src`; arenas/registries use vectors or `BTreeMap`. Intern table is vector-backed with a BTree index. | **Core true.** Manifest/schema/ready-queue insertion order can still change snapshot bytes (hypothesis H-A). |
| 15 | "Contract hash fingerprints normative shapes/rule IDs" — `contract.rs:4-17`, `:515-756` | Hash includes foundation kinds, Gate 1 slice kinds, stage/allowlist rows, enum name lists, and rules. It omits later-wave task-kind names, `ParseContext` names/ordinals, and numeric `RecordRef` tags beyond the M1 seed mapping. | **Partial.** A vocabulary/tag change can leave the hash unchanged. |
| 16 | "Wire proposal payloads are encoded" — rule `snapshot.wire-payloads-encoded`, `contract.rs:125` | `push_wires` encodes all arms except `AppendRecords`, which writes only `batch.records.len()` (`snapshot.rs:446-455`). | **False for one arm.** |
| 17 | "Two-trace comparison reports the first divergence" — `M1_TARGET_ACCEPTANCE.md:371` (D7) | `Trace` has `to_bytes`/`hash` but no parser/diff API (`snapshot.rs:952-1064`). | **Not implemented.** |
| 18 | "Snapshot excludes host paths, addresses, and wall-clock data" — `M1_TARGET_ACCEPTANCE.md:368` (D4) | No addresses/clock fields are encoded by construction; however `SourceRecord.name`/diagnostic messages can contain host-supplied path text and are encoded (`snapshot.rs:643-650`, `:903-924`). No redaction test exists. | **Mostly true; host-path wording needs a precise definition/test.** |
| 19 | "No hidden semantic state / all state in the bus" — T01 §4.1, guardrails §2.4 | `CompilerBus` fields are public (`bus.rs:936-968`), including `arenas`, `tasks.ready`, `store_versions`, `registrations`, `routing`, `report`; tests and candidate code mutate them directly. The host-installed `TickHandler` is an arbitrary closure (`routing.rs:129-131`). | **Not mechanically enforced; trust boundary is documented, not proven.** |
| 20 | "Stable IDs and invalid-value policy" — T01 §3, `ids.rs:1-9` | IDs are newtypes; `u32::MAX` is a sentinel. `ChipId::NONE` (`ids.rs:104-106`) is not rejected by manifest registration or routing; `Task.id` is not checked against its arena key. | **Partial.** |

---

## 3. Observed defects

Severity is relative to a Stage-B determinism freeze: **P0** blocks a reliable canonical/replay contract; **P1** must be resolved or explicitly narrowed before Stage B; **P2** is lower-risk cleanup.

### A23-F01 (P0) — `AppendRecords` wire proposals lose their draft bodies in the snapshot

**Evidence.**

- `compiler/src/snapshot.rs:446-455` handles `Proposal::AppendRecords` by writing only `task.index()` and `batch.records.len()`; neither `RecordDraft` handles nor `G1DraftBody` bodies are encoded.
- `compiler/src/task.rs:1861-1867` defines `AppendBatch { records: Vec<RecordDraft>, bodies: Vec<G1DraftBody> }`; `records.rs:25-96` defines the family/index and 18 body variants.
- The contract rule `snapshot.wire-payloads-encoded` (`compiler/src/contract.rs:125`) and the snapshot header claim (`snapshot.rs:4-16`) cover wire proposals.
- `compiler/tests/c05_codec.rs:633-650` exercises only an **empty** `AppendBatch`; it cannot detect body/family sensitivity.

**Impact.** Two snapshots captured at the same point in a tick, one with `Const(5)` and one with `Const(6)` (or `Literal` vs `Const`, or different `RecordDraft.index` values), are byte-identical. A replay driver that captures/restores a mid-tick snapshot loses the pending append work. The final committed snapshot may still differ, but the claimed "snapshot of the whole bus" does not.

**Minimal reproduction (text only).**

```rust
let task = TaskId::from_index(0);
let body = |v: u8| G1DraftBody::Const(ConstRecord { value: vec![v], negative: false });
let snap = |v: u8| {
    let mut bus = CompilerBus::default();
    bus.wires.proposals.push(TaggedProposal {
        chip: ChipId(2),
        task,
        proposal: Proposal::AppendRecords {
            task,
            batch: AppendBatch {
                records: vec![RecordDraft { family: RecordFamily::Const, index: DraftRef(0) }],
                bodies: vec![body(v)],
            },
        },
    });
    Snapshot::capture(&bus).bytes().to_vec()
};
assert_ne!(snap(5), snap(6)); // currently fails: bytes are equal
```

**Required fix / invariant.** Encode each draft handle `(family, index)` and either the full body via the existing per-record encoder or a canonical body hash (with a versioned body-envelope tag). Add a sensitivity test over every `G1DraftBody` variant and every `RecordDraft.family`/`index`, plus a snapshot round-trip/replay test. If body encoding is deliberately deferred, remove `snapshot.wire-payloads-encoded` from the current guarantee and record the arm as explicitly excluded.

**Affected paths:** `compiler/src/snapshot.rs`, `compiler/src/task.rs`, `compiler/src/records.rs`, `compiler/src/contract.rs`, `compiler/tests/c05_codec.rs`.

---

### A23-F02 (P1) — quota>1 dispatch is reported as one task in `TickReport`, `TickRecord`, `TickMetrics`, and `Trace`

**Evidence.**

- `RoutingShell::dispatch_quota` reads `max_inflight_per_tick` (`routing.rs:251-253`); `propagate_inner` selects and dispatches the full batch (`:340-405`), then sets `selected = batch.first()` (`:401-404`.
- `TickOutcome::Executed` carries one `task`, one `kind`, one `resolution` (`routing.rs:161-170`); `outcome_for_batch` projects only `routed.first()` (`:542-563`).
- `record_tick` builds `dispatched` from `report.selected.into_iter().collect()` and `metrics.dispatched` from that length (`:680-695`).
- `TickRecord` documents `dispatched` as "Tasks dispatched this tick, in dispatch order" (`bus.rs:856-865`).
- `Trace::record` stores `selected` and commit counts but no dispatched list (`snapshot.rs:986-1008`).

**Impact.** A quota-2 tick that completes two tasks writes `dispatched.len() == 1`, `metrics.dispatched == 1`, one `selected`, and a trace that does not identify the second task. This is a false scheduler record, not merely a missing optional metric. It also makes the D8 quota comparison impossible without a new projection.

**Minimal reproduction (text only).**

```rust
let limits = Limits { max_inflight_per_tick: 2, ..Limits::fixture() };
let config = CompilerConfig::try_new(
    TargetSpec::aarch64_unknown_linux_gnu_unverified(),
    Dialect::C11, OptLevel::O0, Vec::new(), limits,
).unwrap();
let mut bus = CompilerBus::try_new(config).unwrap();
for _ in 0..2 {
    bus.bootstrap_task(TaskDraft {
        kind: TaskKind::CONTROL_NOOP,
        payload: Payload::empty(),
        owner: ChipId(0),
        parent: None,
        continuation: None,
    }).unwrap();
}
RoutingShell::new().clock_tick(&CompilerPins::default(), &mut bus).unwrap();
assert_eq!(bus.report[0].dispatched.len(), 2); // currently 1
assert_eq!(bus.report[0].metrics.dispatched, 2); // currently 1
```

**Required fix / invariant.** Carry the full dispatched batch and dispatch ordinals in `TickReport`/`TickRecord`/`Trace`; define a canonical semantic projection that excludes scheduler-only fields (D8); add quota-1 vs quota-N equivalence tests and deterministic repeated-run tests.

**Affected paths:** `compiler/src/routing.rs`, `compiler/src/bus.rs`, `compiler/src/snapshot.rs`, `compiler/tests/c06_routing.rs`, `compiler/tests/c09_readiness.rs`.

---

### A23-F03 (P1) — quota>1 commit order is enqueue order, not dispatch order

**Evidence.**

- `commit_proposals` orders proposals by `(task.enqueue_ordinal, task.index, proposal_index)` (`commit.rs:541-551`).
- `select_batch` orders dispatch by `(phase priority, enqueue ordinal, TaskId)` (`routing.rs:223-243`).
- D8 defines the canonical write-conflict order as `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal index)` (`M1_TARGET_ACCEPTANCE.md:372`).
- The dispatcher stores `bus.tasks.in_flight` in dispatch order (`routing.rs:382-399`) but commit never reads it.

**Impact.** Two tasks in different groups can be dispatched in one order and committed in the opposite order. Record IDs and side effects therefore follow enqueue order rather than the scheduler's dispatch order. This is deterministic for a fixed state, but it is not the frozen D8 canonical order and can change which task receives which next ID when group ordering differs from enqueue order.

**Minimal reproduction (text only).** Bootstrap a `VERIFICATION_*` task first (group 12, enqueue 0) and a `SEMANTIC_*` task second (group 6, enqueue 1), set quota 2, and run one tick. Dispatch order is semantic then verification; commit order is verification then semantic because commit sorts by enqueue ordinal.

**Required fix / invariant.** Pass a dispatch ordinal into commit (for example an ephemeral `batch_ordinal` map or a `TickRecord`-owned selection list) and sort by `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal_index)`. Add a cross-group quota>1 ID-allocation test.

**Affected paths:** `compiler/src/commit.rs`, `compiler/src/routing.rs`, `compiler/src/bus.rs`, `compiler/tests/c06_routing.rs`.

---

### A23-F04 (P1) — `clock_tick` can advance the tick and latch without recording a `TickRecord`

**Evidence.**

- `clock_tick` records only if `propagate` returns `Ok` (`routing.rs:627-633`); it still calls `bus.latch(pins)` and `bus.advance_tick()`.
- `poll_await_joins` can return `Err(CommitError::Limit(LimitError::Queue { .. }))` on an idle tick (`routing.rs:347-366`); `commit_proposals` can return `Err` via `SelectionBatchOverflow` (`:371-376`), though that guard is currently unreachable because `select_batch` clamps.
- The doc comment claims "Exactly one [`TickRecord`] is appended ... per tick" (`routing.rs:596-601`).

**Impact.** A trace/report consumer sees a tick number advance with no report entry; `bus.report.len()` is not a reliable tick count. This violates the trace-lifecycle claim independently of quota.

**Minimal reproduction (text only).** Configure `max_queue_len = 1`; leave one future-ready task in `tasks.ready` so selection is empty; create one `Waiting` parent whose child is already `Completed`. The idle `poll_await_joins` needs to reinsert the parent, making the projected queue length 2. `clock_tick` returns the queue `LimitError` from `propagate`, appends no `TickRecord`, and still increments `control.tick`.

**Required fix / invariant.** Either record a structured terminal/error tick (with the error) before latching/advancing, or do not advance the tick on a pre-commit error and document the retry. Add a test for every `TickOutcome`/error path asserting `bus.report.len() == bus.control.tick` (or a documented offset).

**Affected paths:** `compiler/src/routing.rs`, `compiler/src/bus.rs`, `compiler/tests/c06_routing.rs`.

---

### A23-F05 (P1) — store versions and result versions do not cover all mutations

**Evidence.**

- `commit_proposals` inserts only `StorePatch.store` into `touched_stores` (`commit.rs:1700-1718`). Typed append arms (`:1493-1658`) append records but never touch `store_versions`.
- `intern_name` (`bus.rs:1096-1100`) and `AppendRecords` `Name` bodies (`commit.rs:1643-1656`) do not bump `StoreId::Names`.
- `TaskStore`/`Names` comment says the version slot "bumps only when at least one genuinely new name is interned (commit-track behavior)" (`task.rs:2071-2077`); the code does not do this.
- `ResultRecord.version` is hard-coded to `0` on completion (`commit.rs:1455-1462`), while its doc says "Store/protocol version the result was produced against" (`task.rs:1750-1752`).
- Rule `commit.store-version-guard` (`contract.rs:82`) therefore covers only the `StorePatch` subset.

**Impact.** After a successful `AppendRecords` to `lex.literals`, `constants.records`, `pp.tokens`, `parse.nodes`, etc., the store version remains unchanged. A later `StorePatch` carrying that unchanged version passes `validate_patch` (`commit.rs:1898-1905`) even though the store has changed since the patch was computed. This is a stale-write hole for any future group that uses version-guarded patches against a store also mutated by appends.

**Minimal reproduction (text only).**

```rust
let mut bus = CompilerBus::default();
bus.kinds = TaskKindRegistry::m1_slice();
bus.schema = StoreSchema::m1_slice();
// Seed a Running fold task exactly as c08_gate1.rs: seed_g1_fixtures/running_task.
let before = bus.store_versions.get(StoreId::Constants);
commit_proposals(&mut bus, vec![
    tag(task, fold_chip(), Proposal::AppendRecords { task, batch: const_append(task, &[0]) }),
    tag(task, fold_chip(), ConstantResult { value: RecordRef::Const(ConstId::from_index(0)), legality: ConstLegality::Legal }.route(task)),
]).unwrap();
assert_eq!(bus.store_versions.get(StoreId::Constants), before); // currently unchanged
assert_eq!(bus.arenas.consts.live(), 1);

// Names comment check:
let mut names = CompilerBus::default();
names.intern_name(b"x").unwrap();
assert_eq!(names.store_versions.get(StoreId::Names), 0); // currently 0
```

**Required fix / invariant.** Define "version" as per commit batch or per record, and bump every store actually mutated by that batch exactly once (including typed appends, names, and any future patches). Or forbid mixing `StorePatch` and typed appends on the same store until a per-record version protocol exists. Make `ResultRecord.version` meaningful or remove it. Add tests: typed append changes the owning store version; a stale patch after append is rejected; a new name bumps `Names` exactly once; duplicate names do not bump.

**Affected paths:** `compiler/src/commit.rs`, `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/contract.rs`, `compiler/tests/c05_codec.rs`, `compiler/tests/c08_gate1.rs`.

---

### A23-F06 (P1) — `StorePatch` replace/tombstone is accepted but never applied; target liveness/family is not checked

**Evidence.**

- `validate_patch` checks owner, task, store, field, version, and shape only (`commit.rs:1848-1923`); it does not check that `patch.target` exists, is live, or belongs to the patch's store/family.
- The apply arm logs the patch into `patch_log` and bumps store versions (`commit.rs:1700-1718`); no `Arena::remove`/replace call exists in `commit`.
- The module header says materialization into typed group stores is "owned by each group's commit integration" (`commit.rs:23-26`), but no such integration exists in `/39`.
- `vf01.refs-resolve` only checks `index < allocated` (`chips/verify/vf_store.rs:209-246`); tombstoned IDs pass.

**Impact.** A `Tombstone`/`Replace` proposal is reported as committed (`CommitReport.patches` increments) but the store is unchanged. A tombstone of a nonexistent/future ID, or a target of the wrong family, is accepted. A later reader still sees the old record, and VF01 cannot detect the stale reference. The "tombstones + versions avoid stale rewrites" claim in T01 §3 is unproven.

**Minimal reproduction (text only).** In a bus with no `lex.literals` record, submit a `StorePatch { store: Lex, field: "literals", op: Tombstone, target: Some(RecordRef::Literal(LiteralId::from_index(99))), value: None, version: 0 }` from a declared writer and complete the task. Commit succeeds; `patch_log` contains the patch; `arenas.literals` is unchanged and `LiteralId(99)` was never live. A `Replace` with `target: Some(RecordRef::Node(...))` and `store: Lex, field: "literals"` is likewise accepted.

**Required fix / invariant.** Either materialize `StorePatch` in commit (with a store-to-family map, live target check, tombstone application, and version bump), or explicitly reject `Replace`/`Tombstone` as not implemented while keeping the audit log separate from semantic writes. Add a positive tombstone test and negative tests for nonexistent, tombstoned, wrong-family, and stale-version targets.

**Affected paths:** `compiler/src/commit.rs`, `compiler/src/arena.rs`, `compiler/src/bus.rs`, `compiler/src/chips/verify/vf_store.rs`, `compiler/tests/c03_task.rs`, `compiler/tests/c19_vf01.rs`.

---

### A23-F07 (P1) — The scheduler ignores `STAGE_ASSIGNMENT`; T01 and D5 conflict

**Evidence.**

- `phase_priority(kind)` is `kind.group().raw()` (`routing.rs:34-38`); `Task::schedule_key` is `(phase_priority, enqueue_ordinal, id)` (`task.rs:1783-1789`); `select_batch` sorts that key (`routing.rs:223-243`).
- `STAGE_ASSIGNMENT` (`manifest.rs:1444-1514`) assigns stage ordinals such as `PARSE`/`LEX` = 2, `SYMBOL_TYPE` = 3, `SEMANTIC` = 4, `IR` = 5, `VERIFICATION` = 6, but `stage_of` is consumed only by manifest registration (`manifest.rs:1536-1573`) and worker dispatch (`chips/mod.rs:325-334`, `:379-391`).
- T01 §4 (`T01_COMPILER_CONTRACT.md:78`) specifies the three-key order; M1 target D5 (`M1_TARGET_ACCEPTANCE.md:369`) specifies a four-key stage-first order.

**Impact.** `STAGE_ASSIGNMENT` is not the scheduler order. Example: a ready `VERIFICATION_TOKEN_AST_INVARIANT` (stage 2, group 12) and a ready `SEMANTIC_LITERAL_EXPR` (stage 4, group 6) are selected semantic-first, even though the declared stage order is verification-first. If D5 is authoritative, the scheduler is wrong; if T01's group order is authoritative, D5 and the stage table are misleading. Either way, Stage B cannot claim a stage-priority scheduler from the current code.

**Minimal reproduction (text only).**

```rust
let mut bus = CompilerBus::default();
let vf05 = bus.bootstrap_task(TaskDraft {
    kind: TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
    payload: Payload::empty(), owner: ChipId(0), parent: None, continuation: None,
}).unwrap();
let sem = bus.bootstrap_task(TaskDraft {
    kind: TaskKind::SEMANTIC_LITERAL_EXPR,
    payload: Payload::empty(), owner: ChipId(0), parent: None, continuation: None,
}).unwrap();
assert_eq!(RoutingShell::new().select_batch(&bus, 2), vec![vf05, sem]); // currently [sem, vf05]
```

**Required fix / invariant.** Freeze one order. If D5 wins, carry `stage_of(kind)` (or a stage ordinal in the task) and sort `(stage, phase, enqueue, TaskId)`; if T01 wins, remove the stage-order claim from D5 and document `STAGE_ASSIGNMENT` as a worker-layer gate only. Add an ordering test covering at least one kind whose stage and group disagree.

**Affected paths:** `compiler/src/routing.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `docs/tasks/T01_COMPILER_CONTRACT.md`, `docs/tasks/M1_TARGET_ACCEPTANCE.md`, `compiler/tests/c06_routing.rs`.

---

### A23-F08 (P1) — Limit accounting is not closed (`macros`, `stage_queue_bound`, zero quota)

**Evidence.**

- `Arenas::allocated_total()` lists every arena except `macros` (`bus.rs:679-707`). `total_records()` therefore omits existing macro records (`bus.rs:1021-1023`), while the current batch's `new_macros` is added (`commit.rs:2039`, `:2064-2086`).
- `stage_queue_bound` is declared (`limits.rs:67`), validated (`:154-174`), hashed (`contract.rs:659-661`), and snapshotted (`snapshot.rs:109-111`), but `commit.rs:1425-1438` calls `check_backpressure_capacity` with `max_queue_len` and stage `0`; no `stage_queues` exist. `check_backpressure_capacity`'s own doc claims `stage_queue_bound[stage]` (`commit.rs:2217-2225`).
- `dispatch_quota` clamps a configured zero to one (`routing.rs:251-253`), while `Limits::validate` says zero is invalid (`limits.rs:154-158`). `CompilerBus::new` is public and does not validate (`bus.rs:974-990`), so an invalid config can silently run.
- Rule `limits.every-configured-bound-enforced` (`contract.rs:63`).

**Impact.** A job can exceed `max_records_total` by repeatedly appending macros; a stage-specific queue bound is never enforced; a zero quota is accepted by the infallible constructor and silently treated as one. These are limit-field claims, not just missing tests.

**Minimal reproduction (text only).** Append one `Macro` record, then assert `bus.total_records()` and `ensure_total_records(0)` agree with `bus.arenas.allocated_total()`; the macro slot is absent. Separate: build a bus with `max_queue_len = 4096` but `stage_queue_bound[1] = 1`, enqueue enough stage-1 tasks, and observe no stage-1 rejection. Build `CompilerBus::new` with `max_inflight_per_tick = 0`; `dispatch_quota` returns 1.

**Required fix / invariant.** Include `macros` in total accounting; either implement per-stage queues/bounds or remove the field from the "enforced" set; reject a zero quota in every constructor or remove the clamp. Add tests for macro totals, stage bounds, and zero-quota construction.

**Affected paths:** `compiler/src/bus.rs`, `compiler/src/limits.rs`, `compiler/src/commit.rs`, `compiler/src/routing.rs`, `compiler/tests/c07_limits.rs`, `compiler/tests/c09_readiness.rs`.

---

### A23-F09 (P1) — `vf01.refs-resolve` is an allocated-bounds check, not a liveness check; continuation references are unchecked

**Evidence.**

- `resolve` in `chips/verify/vf_store.rs:209-246` returns `Ok` when `id.index() < bounds.<arena>`; it never calls `is_live`/`get`.
- `Vf01Chip::compute` checks task parent/continuation only against `bounds.tasks`/`bounds.continuations` (`:346-360`), results only against bounds (`:364-386`), and spans only against source length (`:389-409`).
- `TaskDraft.continuation` is `Option<ContinuationId>` (`task.rs:1802-1810`); the Enqueue validation checks destination kind/owner/parent/depth but never the continuation (`commit.rs:823-862`). `check_continuation_committed` exists but has no runtime call site (`commit.rs:2141-2153`).
- The c19 header itself records that "tombstone liveness stays deferred" (`compiler/tests/c19_vf01.rs:5-11`).

**Impact.** A tombstoned record referenced by a task payload, parent, continuation, result, or span is accepted by VF01 as "resolved"; a task can be enqueued with a dangling continuation. The rule name and T01 §3 checked-access/liveness claim overstate the check.

**Minimal reproduction (text only).**

```rust
let limits = bus.limits();
let node = bus.arenas.nodes.alloc(node_record, &limits).unwrap();
bus.arenas.nodes.remove(node).unwrap(); // tombstone; allocated remains
// Seed a task payload referring to the tombstoned NodeId(node), then run VF01.
let input = project_vf01_input(&bus, vf01_task).unwrap();
assert!(matches!(Vf01Chip.compute(&input)[..], [Proposal::Complete { .. }])); // currently passes
```

**Required fix / invariant.** Use checked `get`/`is_live` for every reference in VF01, including task state diagnostic IDs, and validate `TaskDraft.continuation` in commit. Add tombstoned-reference negatives and a dangling-continuation Enqueue test.

**Affected paths:** `compiler/src/chips/verify/vf_store.rs`, `compiler/src/commit.rs`, `compiler/src/task.rs`, `compiler/tests/c19_vf01.rs`, `compiler/tests/c03_task.rs`.

---

### A23-F10 (P2) — Trace semantics are too coarse for replay/evidence

**Evidence.**

- `TraceRecord` stores only `tick`, `selected`, outcome/resolution names, `ready_len`, and four counters (`snapshot.rs:952-971`); `Trace::record` fills `enqueued/completed/failed` from `CommitReport` counts but drops `commit.waiting`, `commit.rejoined`, `commit.patches`, and `commit.appended` (`commit.rs:498-516`; `snapshot.rs:986-1008`).
- `CommitFailed` records `failed = 1` regardless of the dispatched batch size and drops the `CommitError`/diagnostic IDs (`snapshot.rs:1000`).
- `Joined` records readied/failed counts but not the parent/child IDs (`snapshot.rs:1001-1007`); `bus.report` for the same tick records no `readied` IDs either (`bus.rs:856-865`).
- `Trace` has no reader or diff (`snapshot.rs:979-1064`); D7 (`M1_TARGET_ACCEPTANCE.md:371`) is not implemented.

**Impact.** Two executions with different completed task IDs, different appended records, different patches, or different join parents can produce the same trace hash when counts and the first selected task match. A trace-only comparison cannot report the first divergence. This is a trace/evidence gap even at quota 1.

**Required fix / invariant.** Either define `Trace` as a coarse summary and record a separate canonical semantic trace (IDs for dispatched/completed/failed/appended/patched/joined tasks and records), or extend `TraceRecord` with those IDs and a trace diff. Add a test where two runs have equal counts but different IDs and require unequal trace hashes.

**Affected paths:** `compiler/src/snapshot.rs`, `compiler/src/commit.rs`, `compiler/src/routing.rs`, `compiler/src/bus.rs`, `compiler/tests/c05_codec.rs`, `compiler/tests/c06_routing.rs`.

---

### A23-F11 (P2) — Contract vocabulary mismatch: `SYMBOL_KIND_NAMES` says `"variable"`, the codec uses `"object"`

**Evidence.**

- `compiler/src/contract.rs:404` defines `SYMBOL_KIND_NAMES = ["function", "variable"]`; this list is hashed by `FrozenSchema::encode` (`contract.rs:564`).
- `compiler/src/snapshot.rs:1855-1867` encodes `SymbolKind::Object` as `"object"` and decodes only `"object"`.
- `compiler/src/bus.rs:325-332` defines `SymbolKind::{Function, Object}`; all chip use sites use `Object`.
- `compiler/tests/freeze.rs:205-275` cross-checks several enum name lists but not `SYMBOL_KIND_NAMES`.

**Impact.** The frozen contract's normative name list cannot be used to decode an encoded symbol record. A future consumer that follows the contract (`"variable"`) will reject valid snapshots, and a wire-rename to `"variable"` would not change the contract hash. This is a canonical-encoding/contract-consistency defect.

**Minimal reproduction (text only).** `assert_eq!(SYMBOL_KIND_NAMES[1], "variable");` passes; `encode_symbol` for `SymbolKind::Object` emits a length-prefixed `"object"`, and `decode_symbol` rejects `"variable"`.

**Required fix / invariant.** Make the normative list and the codec share one name function (or test their equality); decide whether `Object` or `Variable` is the frozen vocabulary; bump the contract version/hash if the name changes. Add `SYMBOL_KIND_NAMES` to the enum-name cross-check in `freeze.rs`.

**Affected paths:** `compiler/src/contract.rs`, `compiler/src/snapshot.rs`, `compiler/src/bus.rs`, `compiler/tests/freeze.rs`.

---

### A23-F12 (P2) — Frozen hash does not cover `ParseContext`, later-wave kind names, or numeric wire tags

**Evidence.**

- `FrozenSchema::encode` hashes `TASK_STATE_NAMES`, `RESULT_VALUE_NAMES`, `PROPOSAL_NAMES`, many `*_NAMES`/field lists, the foundation registry, and Gate 1 slice kinds (`contract.rs:529-586`, `:706-753`).
- It does not encode `ParseContext` names/ordinals (`task.rs:1586-1607`); `decode_continuation` maps context ordinals 0-9 (`snapshot.rs:2462-2474`). Reordering `ParseContext` would change wire meaning without changing the contract hash.
- It hashes only `TaskKindRegistry::foundation()` plus `is_gate1_slice_kind` entries (`contract.rs:529-541`, `:719-739`); later-wave kind names/status are not hashed, although their raw codes appear in `STAGE_ASSIGNMENT`.
- `RecordRef::wire_tag` numeric tags are not encoded directly; `RECORD_KINDS` positional order and `M1_STORE_FAMILY_ARENA` names/ordinals cover part of the inventory (`contract.rs:681-701`), while `freeze.rs:390-468` only tests tags in Rust.
- A02-F09 already reported the general numeric-tag/later-kind gap; this audit adds the concrete `ParseContext` case and the `SymbolKind` name mismatch above.

**Impact.** A change to `ParseContext` order, a later-wave task-kind rename, or a wire-tag change can leave the frozen hash unchanged while old snapshots decode differently. The hash remains a shape fingerprint, not a complete wire-compatibility guard.

**Required fix / invariant.** Decide and document whether numeric tags, `ParseContext`, and later-wave kind names are hash-covered. If yes, encode them and add a freeze test; if no, add them to `hash_excludes` and state that old byte streams are not guaranteed compatible. Keep the existing hash-scope claim precise.

**Affected paths:** `compiler/src/contract.rs`, `compiler/src/task.rs`, `compiler/src/snapshot.rs`, `compiler/tests/freeze.rs`, `compiler/contracts/CONTRACT_VERSION`.

---

### A23-F13 (P2) — Snapshot omission/self-consistency holes: `Task.id` and hash-only config

**Evidence.**

- `Snapshot::capture` writes the arena key as `w.u32(id.index())` for each task (`snapshot.rs:574-599`); it never writes `Task.id` itself (`task.rs:1758-1761`). No validation checks `task.id == key`.
- `Snapshot` writes `config_hash(bus.config())` (`snapshot.rs:548`), not the full config; there is no `Snapshot::from_bytes`/decoder, so the artifact is not a restorable configuration/state image.
- `Snapshot` has no consistency checks between `control.selected`, `tasks.active`, and `wires.selected`, nor between `store_versions`, `patch_log`, and typed append counts.

**Impact.** Mutating a task's `id` field changes an observable field but not the snapshot hash; a malformed bus can have divergent selected views without a verification failure. The snapshot is a deterministic fingerprint of the encoded subset, not a full state image. This matters if Stage B treats the snapshot as replay evidence.

**Minimal reproduction (text only).**

```rust
let id = bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
let before = Snapshot::capture(&bus).bytes().to_vec();
bus.arenas.tasks.get_mut(id).unwrap().id = TaskId::from_index(999);
assert_eq!(before, Snapshot::capture(&bus).bytes()); // currently equal
```

**Required fix / invariant.** Remove `Task.id` if it is redundant, or encode it and validate `task.id == arena key`; add selected-view and version/patch consistency checks; either add a full snapshot decoder/restore path or narrow the C05 "snapshot" claim to "hash fingerprint for the encoded subset".

**Affected paths:** `compiler/src/snapshot.rs`, `compiler/src/task.rs`, `compiler/src/bus.rs`, `compiler/src/chips/verify/vf_store.rs`.

---

### A23-F14 (P2) — Several decoders reserve a length prefix before checking remaining bytes

**Evidence.**

- `decode_node` converts a `u64` child count to `usize` and immediately calls `Vec::with_capacity(child_count)` (`snapshot.rs:1701-1703`).
- `decode_type` does the same for `params` (`:1837-1839`); `decode_instruction` for operands (`:2295-2297`); `decode_artifact` for `raw_offsets` (`:2384-2387`).
- By contrast, `decode_continuation` bounds the reservation with `child_count.min(r.remaining() / 4)` (`:2486-2490`), and `decode_macro` caps at 1024 (`:2097-2105`).

**Impact.** A malformed snapshot with a huge count can force a huge allocation/OOM before the first missing-element `Truncated` error, violating the "no panic/unchecked allocation on decoders" robustness expectation. This is a codec security/robustness risk, not a determinism defect for well-formed snapshots.

**Required fix / invariant.** Bound every count-based reservation by `remaining()/element_size` (or a configured maximum), then read the elements with checked errors. Add property/fuzz tests for arbitrary byte strings and for `u64::MAX` count fields.

**Affected paths:** `compiler/src/snapshot.rs`, `compiler/src/codec.rs`, `compiler/tests/c05_codec.rs`.

---

## 4. Unproven claims and hypotheses

These are not asserted as code defects until a contract decision is made.

### H-A — Snapshot canonicality across registration/declaration/queue order

`push_manifests` iterates `bus.registrations.iter()` in registration order (`snapshot.rs:290-315`); `push_store_schema` iterates fields in declaration order (`:339-348`); `tasks.ready` is encoded in stored order (`:560-571`). Two buses with disjoint manifests registered in opposite order, or the same schema fields declared in a different order, can hash differently although scheduling behavior is unchanged (manifest lookup is by ID; routing is by kind). Similarly, a manually permuted ready queue can hash differently even though `select_batch` sorts by schedule key.

**Decision needed:** either sort these collections by stable key before encoding (canonical), or declare registration/declaration/queue order part of the semantic initial state and test it. The config options are already deliberately sorted (`target.rs:1277-1280`), so the asymmetry should be intentional.

### H-B — `literal_identity_key` vs `Snapshot::hash`

`literal_identity_key` excludes magnitude/spelling/radix/sign and is documented as the "hash-relevant symbolic projection" (`snapshot.rs:1084-1093`, `:1607-1620`). `Snapshot::capture` includes the literal body, and `Snapshot::hash` hashes those bytes (`:870-878`). The test at `c05_codec.rs:653-686` confirms both facts. No production code calls `literal_identity_key` (only the test). The contract must say whether snapshot hashes are lossless state identity (magnitudes included) or semantic identity (symbolic key). Today the comment names two different hash domains without a defined consumer.

### H-C — "No hidden state" vs public bus fields and handler closures

`CompilerBus` exposes mutable public fields (`bus.rs:936-968`); tests seed fixtures by direct arena/task mutation (for example `c08_gate1.rs:112-148`, `c19_vf01.rs:298-435`), and `TickHandler` is an arbitrary `Fn` (`routing.rs:129-131`). These are deliberate integration/trust boundaries, but they mean identical bus state is not sufficient for determinism if a handler reads host/global state. Stage B needs a declared rule: either all semantic writes go through commit/checked APIs, or the public fields are documented as a trusted host exception and the handler cannot be used for replay evidence without a purity check.

### H-D — Replay is not restoration

`Snapshot` and `Trace` are write-only artifacts: `Snapshot` has `capture`/`bytes`/`hash` (`snapshot.rs:526-878`), `Trace` has `to_bytes`/`hash` (`:979-1064`), and there is no `from_bytes`, decoder, or replay driver. Existing "replay" tests run a scenario twice from fresh construction (`c05_codec.rs:101-112`, `c08_gate1.rs:748-800`, `c17_vf12.rs:653-667`, `c19_vf01.rs:626-632`). Pin data (`CompilerPins`, `bus.rs:922-933`) is not encoded in either artifact. This may be acceptable if "replay" means "re-execute the same input", but it cannot validate a serialized snapshot or a recorded trace. Decide and document.

### H-E — Host-path exclusion in D4

D4 says the snapshot excludes "host paths" (`M1_TARGET_ACCEPTANCE.md:368`). `SourceRecord.name` (`snapshot.rs:643-650`) and diagnostic messages (`:903-924`) can contain host-supplied path text. If the rule means "no absolute workspace paths", a normalization test is needed; if it means "no memory addresses/wall clock", the current code is likely fine.

### H-F — Quota>1 authority

`COMPILER_DEVELOPMENT_GUARDRAILS.md:187-198` says quota >1 is a measured post-freeze change and is inert until then; `Limits::try_new` accepts it and `propagate_inner` executes it today. This review treats that as a documentation/authority mismatch unless the integrator explicitly declares the quota>1 path as an inert foundation shell. If it is inert, it must not be reachable through the normal public `CompilerConfig`/`CompilerBus` path without a separate gate.

---

## 5. Required invariants and Stage-B tests

### 5.1 Invariants to freeze

1. **Determinism:** For identical `(initial bus snapshot, config, pins)` the next-tick `Snapshot.bytes`, `Trace.bytes`, and `TickReport` outcome are byte/value identical. This must include the full dispatched batch and every wire payload.
2. **Canonical ordering:** Every encoded collection has a documented order (ID order, sorted key, or explicit semantic insertion order). If insertion order is semantic, it must be part of the tested initial state; if not, encode sorted.
3. **Snapshot completeness:** Every mutable bus field is either encoded or listed in a versioned exclusion list. `AppendRecords` handles/bodies, config (or a full hash contract), pins, and self-IDs are explicitly classified.
4. **Restore/replay:** If a snapshot is called a replay artifact, provide `Snapshot::from_bytes` + a replay driver and prove `restore -> one tick` equals a fresh run. If not, rename/limit the claim to a hash fingerprint.
5. **Store version monotonicity:** Every successful store mutation bumps the store version exactly once per defined revision unit; a `StorePatch` version is accepted only against the exact pre-batch revision; typed appends/names cannot bypass it. `ResultRecord.version` is either meaningful or removed.
6. **Tombstone/liveness:** A removed ID is never live, is never reused, and every reference check uses `is_live`/checked `get`, not only `index < allocated`. `StorePatch` target family/store/liveness is validated before acceptance or the operation is explicitly rejected.
7. **Task/ID integrity:** Enqueue ordinals are unique and monotonic; `task.id == arena key`; `ChipId::NONE` and other sentinels are rejected at registration/routing; `control.selected`/`tasks.active`/`wires.selected` agree within a tick.
8. **Scheduling:** The frozen key is implemented verbatim (including stage if D5 wins); dispatch order, commit order, and record-ID allocation order are documented and tested at quota 1 and quota>1; a canonical semantic projection exists for quota equivalence.
9. **Lifecycle:** Exactly one `TickRecord` per tick for every `TickOutcome` and every error path that advances the tick; no task remains `Running` at latch; every task reaches terminal/wait/progress exactly once.
10. **Limit closure:** `max_records_total` counts all arenas; every `stage_queue_bound` entry either enforces or is explicitly reserved; `max_inflight_per_tick = 0` is rejected by every public constructor; `max_const_bits` has one declared enforcement layer.
11. **Hash/contract consistency:** Every normative enum name list is shared with its codec; `ParseContext`, later-wave kind names, and numeric tags are either hashed or declared excluded; `SYMBOL_KIND_NAMES` equals the codec names.
12. **No hidden state:** Semantic writes occur only through commit/checked host APIs (or public fields are explicitly a trusted integration exception); handler purity is stated as a trust boundary and cannot be used as replay evidence without a deterministic projection.

### 5.2 Tests to add

| Test | What it must prove | Primary paths |
|---|---|---|
| `append_wire_body_sensitivity` | Two `AppendRecords` proposals with same count but different family/index/body produce different snapshot bytes | `c05_codec.rs`, `snapshot.rs` |
| `quota2_dispatch_record` | `dispatched.len`, `metrics.dispatched`, `selected`, and trace identify both quota-2 tasks | `c06_routing.rs`, `routing.rs` |
| `quota2_commit_dispatch_order` | Cross-group quota>1 commit/ID order follows the frozen dispatch/commit key | `c06_routing.rs`, `commit.rs` |
| `quota1_semantic_projection_equivalence` | quota 1 and quota N produce equal semantic projection, with scheduler fields excluded | new T13/VF fixture |
| `error_tick_record` | Every advancing tick, including idle-join queue error and selection overflow, has exactly one report entry (or no advance) | `c06_routing.rs` |
| `store_version_after_append` | Typed append bumps the owning store; stale patch after append is rejected | `c08_gate1.rs`, `c05_codec.rs` |
| `names_version` | New name bumps `Names` once; duplicate name does not | `c11_lex.rs`, `c05_codec.rs` |
| `tombstone_materialized` | Live target becomes non-live; later refs fail; store version bumps; wrong-family target rejected | `c03_task.rs`, `c19_vf01.rs` |
| `stage_order` | A stage-2 verification kind is selected before a stage-4 semantic kind when both are ready (if D5 wins) | `c06_routing.rs` |
| `snapshot_restore_replay` | Decode/restore snapshot, run one tick, compare to fresh run | new `c05`/T13 harness |
| `trace_divergence` | Different completed/appended/join IDs produce different trace hashes; diff reports first tick | `c05_codec.rs` |
| `pins_digest` | Same state + different pin effects are distinguishable (or explicitly not part of trace) | new replay harness |
| `limit_total_macros` | Repeated macro appends respect `max_records_total` | `c07_limits.rs` |
| `stage_queue_bound` | A stage queue exceeding its own bound is rejected or the field is documented as reserved | `c07_limits.rs`, `c09_readiness.rs` |
| `zero_quota_rejected` | `CompilerBus::new`/`CompilerConfig::new` cannot silently accept zero quota | `c02_target.rs`, `c06_routing.rs` |
| `vf01_tombstone_liveness` | Payload/parent/continuation/result/span refs to tombstoned IDs fail | `c19_vf01.rs` |
| `continuation_enqueue_check` | Dangling `TaskDraft.continuation` is rejected | `c03_task.rs` |
| `enum_name_consistency` | `SYMBOL_KIND_NAMES` and all enum name lists equal codec names | `freeze.rs` |
| `hash_vocabulary` | `ParseContext`, later-wave kind names, and tags change the hash or are excluded | `freeze.rs` |
| `snapshot_order_canonical` | Disjoint manifests/schema declared in different orders hash equal (or order is declared semantic) | `c05_codec.rs` |
| `codec_bounded_allocation` | Huge count prefixes fail with `Truncated` without unbounded reservation | `c05_codec.rs` |
| `task_self_id` | Mutating `Task.id` is either rejected or changes the snapshot | `c05_codec.rs`, `c01_arena.rs` |

---

## 6. Affected paths

**Core implementation**

- `compiler/src/snapshot.rs` — snapshot coverage, wire arms, trace, record encoders/decoders.
- `compiler/src/routing.rs` — selection key, quota/batch reporting, tick records, error paths.
- `compiler/src/commit.rs` — commit order, versions, patch/tombstone semantics, continuation validation, capacity.
- `compiler/src/bus.rs` — `allocated_total`, `TickRecord`, public state, constructors.
- `compiler/src/task.rs` — `Task`/`ScheduleKey`, `AppendBatch`, continuation, result version.
- `compiler/src/contract.rs` — enum name lists, hash coverage, rules.
- `compiler/src/manifest.rs` — `STAGE_ASSIGNMENT`, `stage_of`, registry ordering.
- `compiler/src/limits.rs` — stage bounds, zero-quota validation.
- `compiler/src/target.rs` — config constructor/duplicate-option canonicalization.
- `compiler/src/ids.rs` — sentinel policy.
- `compiler/src/chips/verify/vf_store.rs` — reference liveness.
- `compiler/src/codec.rs` — reader allocation/robustness.

**Tests**

- `compiler/tests/c05_codec.rs`
- `compiler/tests/c06_routing.rs`
- `compiler/tests/c08_gate1.rs`
- `compiler/tests/c09_readiness.rs`
- `compiler/tests/c17_vf12.rs`
- `compiler/tests/c19_vf01.rs`
- `compiler/tests/freeze.rs`

**Contracts/design**

- `docs/tasks/T01_COMPILER_CONTRACT.md` — §3/§4/§7.1 C01/C05 claims.
- `docs/tasks/M1_TARGET_ACCEPTANCE.md` — D1/D2/D4/D5/D7/D8.
- `docs/tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md` — quota>1 authority.
- `docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md` — deterministic ordering/limits.
- `compiler/contracts/CONTRACT_VERSION` and `compiler/src/contract.rs` — version/hash scope.

---

## 7. Checks actually run, and unrun checks

### 7.1 Static checks used for this report

```sh
git rev-parse HEAD
git status --short
wc -l compiler/src/{snapshot,codec,contract,ids,task,commit,routing,bus,manifest}.rs \
      compiler/tests/{c05_codec,c06_routing,c08_gate1,c09_readiness,c17_vf12,c19_vf01,freeze}.rs
grep -R "HashMap|HashSet" compiler/src
grep -R "store_versions|touched_stores|allocated_total|stage_queue_bound|SYMBOL_KIND_NAMES|ParseContext" compiler/src
```

A small read-only Python table check was used only to confirm that `STAGE_ASSIGNMENT` has 69 unique kind rows and covers all 69 `TaskKind` constants; it was not used to modify any file.

### 7.2 Not run (do not infer)

- No `cargo test`, `cargo fmt`, or `cargo clippy` was run for this review. The full suite was deliberately not run.
- Existing `/39` test results are cited only as coverage evidence; they were not re-executed here. The A02 review reports targeted foundation suites passing at the same revision, but that is not evidence for the new findings above.
- No Linux probe, AArch64 cross-compile/link/run, H04 candidate driver, H05 isolation, GCC torture run, or target execution was performed.
- No snapshot restore/replay driver exists to run.

---

## 8. Open questions for the integrator

1. **Scheduler order:** Does Stage B implement T01's three-key `(phase, enqueue, id)` order, or M1 D5's four-key stage-first order? `STAGE_ASSIGNMENT` currently does not participate in selection.
2. **Quota>1 status:** Is the quota>1 dispatch path an inert `/39` shell (and therefore must not be reachable through normal public configuration), or is it an accepted Stage-B profile despite the guardrail text?
3. **Snapshot meaning:** Is `Snapshot` a hash-only fingerprint or a restorable state image? If restorable, a decoder/restore/replay contract is required; if not, the C05/evidence wording should be narrowed.
4. **Store revision unit:** Is a store version bumped per commit batch, per record, or per patch operation? Should typed appends and `StorePatch` share one revision stream? Without a decision, the version guard cannot be tested.
5. **Tombstone/StorePatch semantics:** Does `StorePatch` mutate a store, or is it only an audit log? If only a log, Stage B chips must not use it for semantic state; if it mutates, target liveness/family and apply semantics must be frozen.
6. **Reference policy:** Which references must be existence/liveness-checked at commit, which only at verification, and which are intentionally opaque? `TaskDraft.continuation`, `StorePatch.target`, `ResultValue::Diagnostic`, and record-internal links currently have different answers.
7. **Canonical collection order:** Are manifest registration order, schema declaration order, and ready-queue order semantic? If not, snapshot encoding should sort them.
8. **Hash scope:** Should numeric wire tags, `ParseContext`, later-wave kind names, and `SYMBOL_KIND_NAMES` be hash-covered, or are they explicitly outside the frozen artifact?
9. **Host paths / pins:** What exactly does D4's "host paths" exclusion mean, and must replay evidence include a canonical `CompilerPins` digest?
10. **`Task.id`:** Is the self-ID redundant (remove) or part of the invariant (`task.id == arena key`, encode and check)?

---

## 9. Bottom line

The quota-1 execution path is a credible deterministic baseline for the encoded subset, and the arena/commit design is materially better than a hash-map-driven prototype. The `/39` claims are not yet a replay contract:

- the canonical snapshot drops `AppendRecords` bodies;
- the scheduler record drops quota>1 work;
- store versions do not follow typed appends;
- tombstones and continuation references are not materialized/validated;
- stage order is documented but not implemented;
- limit accounting has concrete omissions;
- and "replay" is currently tested by re-running, not by restoring.

Stage B should not treat the current snapshot/trace as authoritative evidence until F01–F09 are fixed or explicitly narrowed in the contract, and H-A/H-B/H-D/H-F are decided. No target, ABI, compiler, or torture claim is made or supported by this review.
