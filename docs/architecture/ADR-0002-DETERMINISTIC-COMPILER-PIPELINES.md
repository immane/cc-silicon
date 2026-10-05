# ADR-0002: Deterministic Staged Compiler Pipelines with Bounded In-Flight Tasks

Status: **PROPOSED — requires integrator/owner acceptance.**

This ADR is a **proposal only**. It does **not** supersede the frozen
`t01-c01-c06/5` contract, does **not** amend `docs/tasks/T01_COMPILER_CONTRACT.md`
or any other accepted document, does **not** change
`compiler/contracts/CONTRACT_VERSION`, and does **not** authorize any
implementation, C language chip, code generation, or test execution. The `/5`
artifact and the accepted decisions recorded in
[ADR-0001](ADR-0001-COMPILER-DYNAMIC-ARENA.md) remain current until the T01
integrator accepts an amendment and publishes a new freeze.

**Status reconciliation (docs-only; not a freeze, not acceptance).** This ADR's
invariant/§6 wording still calls the H6 batch no-`Running`/terminal-diagnostic
mechanism "BLOCKED/in-principle only". That wording is **superseded as operative
direction** by the user's 2026-10-05 CDR rev 42 decision (mirrored in T02 rev
30/33 and T13): the quota>1-capable bounded batch recovery mechanism is
**selected to be frozen in `/6`** (pre-dispatch errors leave tasks `Ready`; a
failed semantic batch commit commits no semantic state; a deterministic bounded
recovery transitions dispatched tasks once in dispatch order to `Failed`, with a
per-task diagnostic and the `DiagnosticId::NONE` sentinel fallback; a state guard
prevents duplicates). The historical "BLOCKED" rows below are **retained as
history**; what remains genuinely open is the **exact atomic realization** — the
dispatcher `Ready→Running` vs the one ordered atomic commit boundary, the
`in_flight` clear ownership/order relative to that recovery, and the all-paths
no-residual proof — which is the T01 `[INT]` item. This note changes no ADR
invariant text and freezes nothing.

Scope: **compiler application only** (`compiler/`, crate
`cc-silicon-compiler`). The root `cc-silicon` framework — `#![forbid(unsafe_code)]`,
its flat bus data (fixed arrays, no heap, no threads), and the
`docs/architecture/SILICON_PARADIGM_SPEC.md` / `SFL_CONTRACT.md` semantic core —
is **unchanged** (the fixed-layout / no-heap rule is a bus-data constraint, not
a whole-framework guarantee; [ADR-0001](ADR-0001-COMPILER-DYNAMIC-ARENA.md) §2).
Nothing here requires a root-framework change.

Cross-references: [ADR-0001](ADR-0001-COMPILER-DYNAMIC-ARENA.md),
[T01](../tasks/T01_COMPILER_CONTRACT.md),
[PARALLEL_EXECUTION.md](../tasks/PARALLEL_EXECUTION.md),
[T02](../tasks/T02_CONTROL_CHIPS.md), [T13](../tasks/T13_VERIFICATION_CHIPS.md),
[M1 Part A draft](../tasks/M1_PART_A_CONTRACT_PROPOSAL.md) §6.2.1, §6.2.2, §7,
§10.5, §12.18,
[M1 vertical-slice acceptance (frontend half)](../tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md),
[M1 target acceptance](../tasks/M1_TARGET_ACCEPTANCE.md).

This ADR is the scheduling authority. The M1 Part A draft realizes it in
[§6.2.1](../tasks/M1_PART_A_CONTRACT_PROPOSAL.md) (pipeline scheduling
registers), §6.2.2 (extended commit order), §7 (batch commit algorithm), §10.5
(staged bounded sequential driver loop), §11 (chip waves gated on the freeze),
and §12.18 (the `/6` amendment inventory); [T02](../tasks/T02_CONTROL_CHIPS.md)
records the per-chip batch semantics. If this ADR and the M1 draft disagree, this
ADR is the proposal of record for scheduling and the M1 draft is updated to
match, but **neither is accepted** until the T01 integrator freezes them together
(the `/5` contract remains current).

> **Pointer note (docs-only; no acceptance, no freeze).** The M1-draft revision
> range cited below (e.g. §1.1 "rev 21–35") is the range at this ADR's last
> revision and is **not** the current M1 proposal revision. The current companion
> state (as of 2026-10-05; each document's own revision record is authoritative
> for any later change) is: M1 proposal **rev 40** (pointed by CDR **rev 55**),
> T02 **rev 35**, T03 **rev 56**, T04 **rev 47 + task revisions 1–3**, T13
> **rev 11**, M1 target acceptance **rev 37**, M1 vertical acceptance **rev 34**; ADR-0002
> remains at **Revision 17**. This note changes no ADR decision and freezes
> nothing.

---

## 1. Context

### 1.1 Current contract baseline

The checked-in `/5` foundation and the **pre-pipeline** M1 draft were a
**single-active-task** runtime; the M1 draft (rev 21–35) and the user's 2026-10-04
in-principle direction now select the staged pipeline below. The historical
baseline that this ADR generalizes:

- T01 §4 fixes the selection order as `(phase priority, enqueue ordinal, TaskId)`
  and states that a task enqueued in tick `T` is executable in tick `T+1`.
- The pre-pipeline M1 draft defined `dispatched_tasks(bus)` as the singleton
  `bus.tasks.active` when it is `Some`, else the empty set. **Rev 21 generalizes
  this** to the ordered in-flight set at quota `> 1` (singleton view at quota 1);
  `tasks.active` survives only as the quota-1 compatibility projection.
- M1 draft §10.2 step 3 now selects a bounded ordered batch through CT03's
  `ProjectedChip`/`ChipAdapter`; the dispatcher applies the whole batch, the
  backend ticks the guard and the routed worker per dispatched task, and one
  `commit_proposals` call materializes the tick.
- [T02](../tasks/T02_CONTROL_CHIPS.md) fixes the topology
  `TaskSelect → TaskGuard → worker sub-pipeline → ProposalValidate → TaskCommit
  → ResultResume/PhaseAdvance` and the **selected-draft** AB1a/AB1b/AB2 realizations
  (a selected draft direction, **not** an accepted contract)
  (CT03 decision-only, CT05 validation-only, CT06 proposes and the backend
  commits, `TickReport`/`TickOutcome`/`Resolution` in `report.rs`, `CT07`
  decision-only with the join realized in commit apply).
- [PARALLEL_EXECUTION.md](../tasks/PARALLEL_EXECUTION.md) §6 explicitly warns
  against "running the entire hundreds-of-chips pipeline every tick" and says the
  first runtime version is "strictly sequential with fixed routing. Verify the
  architecture with M1 first, then expand the task pool."

This is the **quota = 1 baseline** runtime. It is deterministic, auditable, and
is the baseline against which any throughput change must be proven. The selected
staged pipeline generalizes it; quota 1 remains the **baseline equivalence mode
only** (compared by the canonical projection in §2.1/§2.8), not the final
architecture. **The user's 2026-10-04 selection is an in-principle working basis,
not a freeze; this ADR remains PROPOSED.**

### 1.2 The pressure to change

A compiler front end is a staged pipeline (preprocess → lex → parse →
symbol/type → sem → constants → IR → machine → artifacts). With one dispatch per
tick, independent work in different stages is serialized: tick `T` may only run
one preprocess task even when a ready lex task exists and consumes no
preprocess output. The cost is tick count, not correctness. The proposal is to
allow a **bounded number of independent in-flight tasks per tick**, executed
sequentially by the same single-threaded backend, while preserving every
existing determinism, atomicity, exactly-once, and write-scope guarantee.

### 1.3 What this is not

- It is **not** runtime threading, a work-stealing scheduler, or concurrent bus
  writes. The single CPU reference backend still owns the tick and invokes chips
  one at a time.
- It is **not** permission to run the whole chip catalog every tick. Dispatch
  remains routed and bounded; only the quota per tick changes.
- It is **not** an SFL semantic-core change and does **not** weaken ADR-0001.
- It is **not** measured. No throughput number is claimed here.

---

## 2. Decision (proposed)

Adopt a **deterministic sequential staged pipeline** in the compiler
application, with a configured **in-flight quota** `max_inflight_per_tick`. The
default quota is **1**, which reproduces the current single-active-task
semantics exactly.

### 2.1 Invariants (must hold at every quota)

1. **Single writer, sequential.** One thread; the backend invokes chips one at a
   time. No concurrent bus mutation. This preserves the root framework's
   no-threads rule; multiple dispatches are sequential `tick` calls inside
   `Backend::execute_layers`, exactly the technique M1 draft §10.2 already uses.
2. **Deterministic dispatch order.** The dispatches of a tick are a pure
   function of the bus snapshot and pins, ordered by an extension of the T01
   rule: `(stage ordinal, phase priority, enqueue ordinal, TaskId)`. No
   hash-map iteration order, address, or wall-clock input may affect it.
3. **Per-dispatch exactly-one transition.** Every dispatched task produces
   exactly one of `Complete` / `Fail` / `AwaitHost` / `AwaitChildren` / `Progress`
   in the tick. No dispatched task may remain `Running` at latch. (This is the
   stated invariant; the *mechanism* that guarantees it on the terminal
   capacity-failure path is BLOCKED, §6 — H6. The user **selected the recovery
   direction in principle** on 2026-10-04: fail every dispatched task exactly once
   in dispatch order with the same optional-diagnostic/`DiagnosticId::NONE`
   sentinel semantics; the exact atomic/bounded implementation requires T01
   integrator approval.) This generalizes the M1 draft
   one-transition rule from a singleton to the in-flight set.
4. **One ordered atomic commit per tick.** All proposals from all dispatches are
   written to wires and, at the end of propagation, handed to a **single**
   `commit_proposals` pass in the deterministic dispatch order. Commit validates
   the whole batch before any mutation and applies only infallible appends. A
   failed batch commits nothing. **The verified `/5` `routing.rs` `fail_selected`
   single-task obligation is preserved:** the failed task **always** transitions to
   `TaskState::Failed` — a committed `DiagnosticId` is attached when
   diagnostic/record capacity allows, otherwise the
   `TaskState::Failed(DiagnosticId::NONE)` sentinel is used, so it is never
   stranded or `Running`. **Batch failure-recovery direction (user, in principle,
   rev 32 of the M1 proposal / Revision 13, 2026-10-04; H6):** on a failed atomic
   semantic batch commit, the user selects the direction that **every** dispatched
   task, in **dispatch order**, transitions **exactly once** to
   `TaskState::Failed` — a committed `DiagnosticId` when diagnostic/record capacity
   allows, otherwise the `TaskState::Failed(DiagnosticId::NONE)` sentinel — so
   **every** dispatched task leaves `Running`; **clearing the in-flight set is not
   itself a transition**, and clearing it does **not** clear `TaskState`/`Running`.
   This generalizes the verified `/5` single-task transition (unchanged). **The
   implementation remains BLOCKED/co-freeze:** the exact atomic/bounded mechanism
   requires **T01 integrator approval**, and the batch failure-atomicity details,
   the diagnostic budget/state mechanism, and the T02/T13 owner
   fixtures/sign-offs remain pending (§6). Single-task behavior is verified; the
   batch behavior is **proposed only**.
5. **No same-tick cross-task draft references.** A task's payload references
   committed records only; `AppendRecords` draft refs stay task-scoped (M1 draft
   §6.2). Therefore two dispatches in the same tick cannot consume each other's
   yet-uncommitted output; new tasks and results become visible in tick `T+1`.
   This is what makes the in-flight set safe under a single commit.
6. **Committed-ID cross-tick cursor/joins.** Continuations, cursors, awaited
   child sets, and join points are expressed over **committed IDs**
   (`TaskId`, `ResultId`, `RecordRef`) and live in the bus. No continuation or
   cursor may point at a draft, a wire, or an address. Cross-tick joins resume a
   parent only when all its committed child results are present. **Rev 22
   reconciliation request (T02 finding 5):** the M1 candidate also introduces
   **same-task own-batch** draft keys (`ContinuationRef::OwnBatch`,
   `ChildRef::OwnBatch`), which are not foreign drafts but do not fit the literal
   "never point at a draft" wording (accepted
   [Guardrails](../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md) §6.1). **Accepted in
   principle (user, 2026-10-04; decision A — narrow interpretation):** a same-task
   `OwnBatch(DraftRef)` may exist **only as a transient wire/proposal input before
   commit**; the commit validates and resolves it to a committed `TaskId` **before
   any persistent state**, and **no durable cursor/`WaitSet`/join may point at a
   draft, wire, or address**. The guardrail text is **unamended**; **T01 integrator
   implementation-confirmation remains pending**. This is `ACCEPTED-IN-PRINCIPLE`
   only, not a freeze, and does not amend §6.1.
7. **Explicit stage state.** Canonical per-stage ready queues
   (`stage_queues[stage]`), the in-flight set, the fairness cursor (if any), the
   `dispatch_cursor`, and the stage-assignment version are bus registers,
   reset/latched by the normal clock lifecycle, and included in the
   deterministic snapshot. No hidden scheduler state. `tasks.ready` is a derived
   compatibility view, not a second write target; `Progress`/resume/enqueue
   reinsert into the task's own stage queue.
8. **Bounded and fail-explicit.** Quota (`max_inflight_per_tick`), per-stage queue
   depth (`stage_queue_bound`), dispatch count, task total (`max_tasks_total`), and
   proposal budget are configured limits enforced before
   mutation; exhaustion yields a structured diagnostic, never a panic or a
   silent drop. **Rev 33/34 (H9, direction in principle, pending T01):** the
   separate **in-flight total** bound (`max_inflight_total`) is **removed** from the
   candidate — per-tick dispatch is already bounded by `max_inflight_per_tick`, the
   per-stage queues, and `max_tasks_total`, and `Waiting` is not in-flight.
9. **Same-tick write conflicts are predicates, not assumptions.** Two different
   dispatched tasks may write the same `(StoreId, field, record)` key only when
   the field is declared append-only and the appends are semantically
   independent under a hashed rule; otherwise the whole batch is rejected before
   apply (`CrossTaskWriteConflict`). **Rev 21 scope (user, in principle):** each
   field has exactly one owner group, so this covers two tasks of the *same*
   owner group appending one append-only field, or two patches; it is **not** a
   shared-writer carveout and never permits a second owner group
   (`StoreOwnerViolation`). A patch to the same record always conflicts (no
   commutative patch rule in M1). The commit hashes the canonical ordered append,
   so a permitted same-owner append stays deterministic. The audit owner is a
   **proposed** VF04 (`AccessContractChip`) extension with the batch conflict
   check; that extension is **not defined today** (residual); VF12 is
   `IrInterpretChip` and is not the conflict verifier unless explicitly
   extended.

### 2.2 Stages and per-stage queues (proposed shape)

Define a closed `StageId` newtype and a fixed, versioned pipeline order, for
example:

```text
Stage order (proposal; exact names/number are RESIDUAL):
  Source/PP → Lex → Parse → Symbol/Type → Sem → Const → IR → Machine → Artifact
```

- Each task kind is assigned to exactly one stage by a **stage-assignment
  table**. Assignment is closed, total over registered kinds, and **proposed to
  be hashed** into the `/6` contract (prospective; not hashed today — rev 25 F1).
- Each stage owns an explicit ready queue (`stage_queues[stage]`) with its own
  bound. The current single `tasks.ready` is a derived quota=1 view of the
  canonical per-stage `stage_queues`.
- A worker's routed layer is unchanged; routing is `(kind → worker layer)`.
  Stage is a scheduling/queue concept, not a second routing mechanism.

> **RESIDUAL (reduced in rev 3).** Whether the stages mirror the T01 §2 store
> partitions exactly, and whether the canonical queues are one partitioned vector
> or physically separate vectors, are integration decisions. **Selected:** the
> canonical queue source is `stage_queues`, and `tasks.ready` survives only as a
> derived read-only compatibility view (no second writer).

### 2.3 Stage-assignment versioning

- Carry an explicit `stage_assignment_version` in the bus/config and encode it in
  the deterministic snapshot.
- The `(TaskKind → StageId)` table and stage ordinals are **proposed to be**
  frozen as a hashed section of the T01 contract together with the version at
  `/6` (prospective; not frozen/hashed today — rev 25 F1).
- A task is scheduled by the stage assignment in force for its snapshot. Changing
  a kind's stage changes scheduling and is a normative change requiring a version
  bump, re-freeze, and dependent-test rerun — not a runtime mutation.
- A chip may not move itself between stages; the manifest declares the stage and
  the registry validates total coverage.

### 2.4 Dispatch loop and selection model

Keep the selected AB1a/AB1b decision (CT03 decision-only; dispatcher applies):

1. Tick-start reset clears the wires (`wires.selection` is cleared by
   `reset_wires`). **Rev 17 reconciliation / OB-18:** the earlier wording that
   tick-start reset also cleared the in-flight set and `dispatch_cursor` is
   **superseded** — a next-tick-start `in_flight` clear leaves a residual entry
   across the latch boundary. Per T02 rev 33 (and this ADR's own §6), the
   in-flight set is cleared **at the current latch only after** every dispatched
   task has a terminal/`Waiting`/`Progress` outcome or the H6 recovery; the exact
   clear owner/order remains the T01 `[INT]` decision. The tick-start reset is a
   **wire reset only**.
2. Budget/cancel gate as in M1 draft §10.2 step 1, evaluated once per tick before
   any selection (so cancel is not re-evaluated between dispatches; see §6).
3. Tick CT03's installed `ProjectedChip` **once**. Its adapter writes a bounded,
   ordered `SelectionBatch { entries: Vec<SelectionEntry> }` (at most one entry
   per eligible stage, up to quota; `SelectionEntry { task, chip, layer,
   dispatch_ordinal }`) to `wires.selection`. CT03 remains decision-only and
   never touches `control`/`tasks`. **`wires.selection` has CT03's adapter as its
   only writer.**
4. The dispatcher applies the whole batch: for each entry in order, mark the task
   `Running`, set `control.dispatch_cursor` to that entry's position, record it in
   the in-flight set, and remove it from its canonical `stage_queues[stage]`.
   There is still **exactly one selection path** and no duplicate selection.
   `wires.selected: Option<TaskId>` is only a quota-1 compatibility projection
   **whose sole writer is the dispatcher**; the canonical report field is the
   ordered `dispatched: Vec<TaskId>` set. **Single writer per wire:** the
   dispatcher writes `wires.selected`; each worker adapter writes only
   `wires.proposals`; the driver's step-6 sole append site writes `bus.report`.
5. For each dispatched task in order, tick the guard layer and exactly the routed
   worker layer, with `dispatch_cursor` set so the worker projection sees exactly
   that task. The worker writes proposals to wires only.
6. Collect all proposals into one ordered log and call `commit_proposals` once
   (one ordered atomic commit). **Verified `/5` obligation:** the frozen
   `routing.rs` `fail_selected` single-task failure **always** transitions that
   task to `TaskState::Failed` — attaching a committed `DiagnosticId` when
   diagnostic/record capacity allows, otherwise the
   `TaskState::Failed(DiagnosticId::NONE)` sentinel (never stranded). **On failure
   the batch recovery direction is user-selected in principle (rev 32 /
   Revision 13, 2026-10-04; H6):** every dispatched task, in **dispatch order**,
   fails **exactly once** — a committed `DiagnosticId` when capacity allows,
   otherwise the `DiagnosticId::NONE` sentinel — so every task leaves `Running`;
   clearing the in-flight set is **not itself** a transition (clearing it does not
   clear `TaskState`/`Running`). This generalizes the verified single-task
   transition. **The implementation remains BLOCKED/co-freeze** (§6): the exact
   atomic/bounded mechanism requires **T01 integrator approval**, and the batch
   failure-atomicity details, the diagnostic budget/state mechanism, and the
   T02/T13 owner fixtures/sign-offs remain pending; the alternative (another
   T01-approved atomic terminal path, e.g. reserve bounded terminal diagnostic
   slots before dispatch) is retained as an implementation option.
   **[Rev-42 note (user-accepted recommendation; T01 co-freeze pending; not a
   `/6` freeze; no code): the "BLOCKED" framing above is qualified by the
   rev-42 selected quota>1-capable bounded recovery mechanism to be frozen in
   `/6` (pre-dispatch errors leave tasks `Ready`; failed batch commits no
   semantic state; dispatch-order `Failed` recovery; no N pre-reservation;
   `NONE` fallback; state guard); exact atomic realization + fixtures remain
   open.]**
7. The sole report-append site (M1 draft §10.2 step 6) appends one
   `TickRecord` for the tick, now recording the ordered dispatched set and the
   per-stage metrics.

> **RESIDUAL (reduced in rev 3).** Whether CT03 computes the fair subset or only
> enumerates eligible tasks while the dispatcher chooses, the final fairness
> weights, and the exact inner `PipelineMetrics` encoding remain integration
> decisions. The `SelectionBatch`/`SelectionEntry` shape and the
> `dispatch_cursor`-as-position-of-current-task semantics are **selected** (see
> step 3/4/5), not residual.

### 2.5 Fairness

Selection must be deterministic **and** starvation-free across non-empty stages:

- A documented policy (for example one entry per non-empty stage in stage order,
  then fill the remaining quota by the global order) is frozen with the contract.
- Any persistent fairness cursor is explicit bus state and snapshot-encoded.
- Fairness policy is a versioned contract input; changing it is a normative
  change and a replay-visible difference.

> **RESIDUAL.** Round-robin vs. weighted vs. one-per-stage, and whether a
> persistent cursor is needed at all, are open.

### 2.6 Backpressure

- The commit preflight checks every per-stage queue bound
  (`checked_add(queue_len, new_tasks + progress_reinserts) <= bound`) before any
  mutation, replacing the single global `max_queue_len` check or complementing
  it.
- A stage whose downstream queues are at capacity is throttled deterministically
  (documented selection policy), so the pipeline cannot overflow silently.
- Backpressure is reported as a deterministic metric, not a panic.

> **RESIDUAL.** Whether backpressure *defers* dispatch, *rejects* with a
> structured capacity error, or both, and whether upstream throttling is computed
> per stage or globally, are open.

### 2.7 Cancel, fail, and replay

- **Cancel.** Once per tick, before selection, a cancel pin sets the job terminal
  exactly as today; no new dispatches occur. Any dispatch already applied before
  a mid-tick failure is handled by the single atomic commit and the terminal
  record. Repeated cancel is idempotent.
- **Fail.** A worker `Fail` is one transition for that dispatch. Because all
  proposals commit atomically, a failure does not produce partial appends. The
  parent/subtree failure propagation continues to use committed IDs and existing
  fault-propagation rules. **Rev 25/26 F2:** this is the single-task path. **Batch
  failure-recovery direction (user, in principle, rev 32 / Revision 13, 2026-10-04;
  H6):** on a failed atomic semantic batch commit, every dispatched task, in
  **dispatch order**, fails **exactly once** — a committed `DiagnosticId` when
  capacity allows, else the `TaskState::Failed(DiagnosticId::NONE)` sentinel — so
  no dispatched task is left `Running`; **clearing the in-flight set is not itself
  a transition** (clearing it does not clear `TaskState`/`Running`). The verified
  `/5` single-task obligation is
  preserved: the failed task **always** becomes `TaskState::Failed` with a
  committed `DiagnosticId` when capacity allows, else the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel (never stranded); the batch
  behavior is **proposed only** and its exact atomic/bounded implementation is
  **BLOCKED/co-freeze** (§2.1 invariant 4, §2.4 step 6, §6).
  **[Rev-42 note (user-accepted recommendation; T01 co-freeze pending; not a
  `/6` freeze; no code): the "proposed only / BLOCKED" batch framing above is
  qualified by the rev-42 selected quota>1-capable bounded recovery mechanism
  to be frozen in `/6`; exact atomic realization + fixtures remain open.]**
- **Replay.** For identical input, config, pins, and initial state, the stage
  assignment version, dispatch order, proposal order, commit outcome, snapshot
  hash, and per-tick report are identical across runs. The quota is part of the
  config and is replay-visible.

> **RESIDUAL.** Whether a single dispatch's commit failure fails the whole tick
> batch or only that dispatch, and whether cancel is re-checked between
> dispatches, are open. The **batch recovery direction** ("whole batch is atomic,
> each dispatched task is failed exactly once in dispatch order with the optional
> diagnostic/`DiagnosticId::NONE` sentinel, so none remains `Running`") is
> **user-selected in principle** (rev 32 / Revision 13, 2026-10-04; H6) —
> **direction only, not an accepted mechanism and not frozen**: the exact
> atomic/bounded implementation is **BLOCKED/co-freeze** pending **T01 integrator
> approval** and the T02/T13 owner fixtures/sign-offs, and clearing the in-flight
> set alone still does not clear `TaskState`/`Running`. The
> verified `/5` single-task `fail_selected` obligation (task → `Failed`, a
> committed `DiagnosticId` when capacity allows, else the
> `TaskState::Failed(DiagnosticId::NONE)` sentinel, never stranded) is preserved
> (see §2.1 invariant 4, §2.4 step 6, §6).
> **[Rev-42 note (user-accepted recommendation; T01 co-freeze pending; not a
> `/6` freeze; no code): the "direction only / BLOCKED" batch framing above is
> qualified by the rev-42 selected quota>1-capable bounded recovery mechanism
> to be frozen in `/6`; exact atomic realization + fixtures remain open.]**

### 2.8 Metrics and acceptance evidence

Add deterministic, snapshot-encoded metrics to the canonical report register.
**Selected (rev 3):** the metrics live in `bus.report.metrics` (a dedicated
register inside `report`), not a floating `bus.metrics` view; only the inner
encoding remains residual:

- ticks; total dispatches; dispatch count for the max-quota tick;
- per-stage dispatch counts and per-stage ready-queue high-water marks;
- commits, rejected batches, idle ticks, backpressure events, cancellations;
- in-flight high-water mark; number of ticks at quota 1 / 2 / … (histogram).

Acceptance evidence required before any quota increase is claimed (all on the
frozen corpus/config, none claimed today):

1. **Quota=1 equivalence (baseline).** For every existing M1 fixture, the
   staged driver at quota 1 must equal the `/5` + M1 single-active-task design
   under the **canonical semantic comparison projection** (semantic
   records/results/diagnostics plus the semantic trace, excluding scheduler-only
   registers: stage queues, in-flight set, `dispatch_cursor`,
   `stage_assignment_version`, `PipelineMetrics`). Byte-identical snapshots are
   **not** required and are impossible when the scheduler registers are encoded;
   separately, the new scheduler snapshot must be deterministic and
   replay-identical run-to-run (§2.8 item 2). This is the gate that proves the new
   machinery is a strict generalization.
2. **Replay determinism.** Identical inputs yield identical ids, orders, hashes,
   and (for the scheduler) identical `stage_assignment_version`, dispatch order,
   queue state, and metrics across repeated runs at each tested quota.
3. **Exactly-one transition per dispatched task** and **atomic batch commit**
   (failed batch mutates nothing). The "no `Running` survives a tick" invariant is
   **stated**. The verified `/5`
   `routing.rs` `fail_selected` single-task obligation (the failed task becomes
   `TaskState::Failed` — a committed `DiagnosticId` when capacity allows, else the
   `TaskState::Failed(DiagnosticId::NONE)` sentinel, never stranded) is
   **preserved**. **Batch failure-recovery direction (user, in principle, rev 32 /
   Revision 13, 2026-10-04; H6):** every dispatched task fails exactly once in
   dispatch order with the optional-diagnostic/`DiagnosticId::NONE` sentinel, so
   none remains `Running`; clearing the in-flight set is **not itself** a
   transition. **The implementation is BLOCKED/co-freeze** (exact atomic/bounded
   mechanism requires T01 integrator approval; batch failure-atomicity details,
   diagnostic budget/state, and T02/T13 owner fixtures/sign-offs pending); the
   alternative is recorded in §6. Single-task
   behavior is verified; the batch behavior is **proposed only**.
4. **Cross-task conflict determinism and predicate.** Two same-tick writes to the
   same `(StoreId, field, record)` key are either (a) permitted when the field is
   declared append-only and the appends are semantically independent under a
   hashed rule, or (b) rejected in the canonical order
   (`CrossTaskWriteConflict`) independently of any internal iteration order. The
   batch is rejected before apply in case (b).
5. **Write-scope and manifest audit** hold for every dispatched task.
6. **Measured increase only.** A quota `Q > 1` is acceptable only after before/after
   tick counts are measured on the frozen corpus and reviewed. No unmeasured
   throughput claim is permitted.

### 2.9 Baseline quota = 1; measured quota > 1 is the optimization profile

`max_inflight_per_tick` **defaults to 1** and the quota=1 path is the required
baseline: for every existing M1 fixture it must equal the `/5` + M1
single-active-task design under the **canonical semantic comparison projection**
(§2.8 item 1), while the new scheduler snapshot stays separately
deterministic/replay-identical. Quota 1 is the
**baseline equivalence mode only**; it is **not** the final optimization profile
and this ADR does **not** freeze a single-active-task architecture as final.

The proposal enables larger quotas; it does not assert they help. A quota `Q > 1`
is a later, **measured**, integrator-accepted change (see §2.8 item 6 and
[PARALLEL_EXECUTION.md](../tasks/PARALLEL_EXECUTION.md) §6, "verify the
architecture with M1 first, then expand the task pool"). No throughput claim is
made by this ADR or by the M1 draft; only the measured before/after tick counts on
the frozen corpus may support one. Multiple in-flight tasks are still executed
**sequentially** by the single CPU backend with one ordered atomic commit; this is
never runtime threading.

---

## 3. Required `/6` amendments (proposal inventory)

This section lists the **exact** documents/artifacts the integrator would have
to amend to accept and implement this ADR. It is an inventory, not an edit; no
existing file is changed by this proposal. It extends, and does not replace, the
M1 Part A §12 `/6` inventory.

| Artifact | Required amendment (proposal) |
|---|---|
| **`compiler/src/bus.rs`** | Replace the singleton `tasks.active: Option<TaskId>` with a bounded in-flight set (`tasks.in_flight`/`in_flight_count`) plus `dispatch_cursor`; make canonical per-stage ready queues (`stage_queues`, bounded) the single write target with `tasks.ready` a derived read-only view; add `stage_assignment_version`; `CompilerWires.selection` becomes the canonical bounded ordered `SelectionBatch` with `wires.selected` only a quota-1 projection; add batch-aware report/metrics registers (`report.dispatched`, `report.metrics`); extend per-limit preflight (per-stage queue bounds and the per-tick `max_inflight_per_tick`/quota bound only — **no separate total in-flight bound** (H9 removal direction) and **no separate `max_dispatches_per_tick`/dispatch-budget bound** (rev 17 candidate default; the quota is the sole per-tick dispatch-count bound)). `control.selected`/`tasks.active` remain quota-1 compatibility views. |
| **`compiler/src/task.rs`** | Add `StageId` and `STAGE_NAMES` (or equivalent); add `SelectionEntry`/`SelectionBatch` (`SelectionEntry { task, chip, layer, dispatch_ordinal }`) replacing the old singular `Selection`; add `ContinuationRef`/`ChildRef`/`Proposal::AwaitChildren`; extend the deterministic commit order key with the dispatch ordinal; keep `Proposal`/`ResultValue`/`StorePatch` semantics; add explicit in-flight/`dispatch_cursor` fields; keep exactly-one-transition as a batch-level invariant. |
| **Selection (T02/CT03 + dispatcher)** | Preserve AB1a: CT03 stays decision-only and writes the canonical `wires.selection: SelectionBatch`; the dispatcher applies the **whole** ordered batch and sets the quota-1 `wires.selected` projection; exactly one selection path and no duplicate selection. CT03 is excluded from routed invocation. Selection policy (fairness) becomes a versioned contract input. |
| **`compiler/src/report.rs`** | `TickOutcome::Executed` carries the ordered dispatched set (not one task); add `TickReport.dispatched: Vec<TaskId>` as the canonical field with `selected` retained only as the quota-1 projection; add per-stage/metrics fields; keep the single append site, the `checked_add(max_ticks, 1)` bound with exactly one terminal record, and the terminal-halt guard. **`report.rs` does not exist today**; the relocation is proposed. |
| **`compiler/src/limits.rs` + `compiler/src/target.rs`** | Add `max_inflight_per_tick` (default **1**), per-stage queue bound(s), and optional fairness-weight bounds; add `ConfigError` variants for invalid quota/queue/fairness values (stable diagnostic codes); wire validation into `CompilerConfig::try_new`/`CompilerBus::try_new`. **Rev 33/34 (H9):** the separate **in-flight total bound** (`max_inflight_total`/`InflightQuotaExceeded`) is **removed from the candidate** in principle (pending T01 integrator acceptance) — per-tick dispatch is bounded by `max_inflight_per_tick`, the stage queues, and `max_tasks_total`. **Rev 17 (integration-selected candidate default under user delegation, 2026-10-05):** the redundant **`max_dispatches_per_tick` is dropped** from the candidate limit inventory and its validation — `max_inflight_per_tick`/quota is the **sole per-tick dispatch-count bound** (grounded in the rev-33/34 H9 direction and the rev-49 audit finding that the two limits duplicated each other; **not** an owner/T01 `[INT]` sign-off or a schema freeze; T01/T02 must confirm/co-freeze the exact names/defaults/codes at `/6`). |
| **`compiler/src/contract.rs` (schema)** | Add a hashed `StageAssignment` section (kind → stage, stage ordinals, `stage_assignment_version`) and the `NORMATIVE_RULES` ids for the new pipeline (e.g. `pipeline.stage-assignment-versioned`, `pipeline.per-stage-queues`, `pipeline.dispatch-order`, `pipeline.fairness-deterministic`, `pipeline.backpressure-bounded`, `pipeline.committed-id-cursors`, `pipeline.own-batch-continuation`, `pipeline.await-children`, `pipeline.progress-canonical-stage-queue`, `pipeline.inflight-quota`, `pipeline.quota-baseline-one`, `commit.batch-ordered-atomic`, `commit.one-transition-per-dispatch`, `commit.cross-task-check-order`, `commit.cross-task-write-conflict`, `replay.dispatch-order-determinism`, `replay.scheduler-snapshot-deterministic`, `baseline.canonical-projection`). One `/6` freeze pins all shapes together with the M1 `M1AppendSchema`. |
| **`compiler/src/manifest.rs` + `COMPILER_SFL_MANIFEST.md`** | Each chip manifest declares its `stage`; registration validates total kind→stage coverage, unique stage ordinals, and that the stage matches the routed layer. Reconcile the manifest doc's hash-coverage claim (§4) with `contract.rs` so the stage assignment is actually hashed. |
| **`docs/tasks/T02_CONTROL_CHIPS.md`** | Batch semantics: CT03 selects a bounded batch; CT04 guards per dispatch; CT05 validates the whole ordered batch including batch-scoped cross-task checks; CT06 proposes the whole commit; CT07 resumes joins across in-flight/children over committed IDs; CT09 advances phase only when all stage queues are empty/waiting; CT10 checks progress/budget over the batch (no wall-clock); CT12 recovery is deterministic per dispatch; CT13 cancel handles an in-flight set. Update the runtime-realization paragraph. |
| **`docs/tasks/T13_VERIFICATION_CHIPS.md`** | Extend VF02 (exactly-one transition per dispatched task across the batch; no dead-wait), VF03 (wire lifetime with a batch selection), VF04 (add the batch write-conflict access audit), and VF13 (replay compare across dispatch order). **VF12 is `IrInterpretChip`; it is not the cross-task conflict verifier and must not be extended/cited for pipeline checks unless explicitly extended.** Add fixtures for multi-in-flight determinism, cross-task conflict ordering, quota=1 canonical-projection equivalence, and backpressure. **Rev 32 (H6 direction, 2026-10-04):** add the pending batch-verification amendment for the **all-dispatched failure recovery** — verify that on a failed atomic semantic batch commit every dispatched task transitions exactly once to `Failed` in dispatch order (a committed `DiagnosticId` when capacity allows, else the `DiagnosticId::NONE` sentinel) so no dispatched task remains `Running`, and that clearing the in-flight set is not itself a transition. |
| **`docs/tasks/M1_*`** | M1 default remains quota 1; existing fixture traces are unchanged at quota 1 (baseline-equivalence gate). Add multi-in-flight fixtures/gates (e.g. M1-P`*`, M1-BP`*`, M1-REC`*`), extend G9 (replay) and G10 (write-scope/atomic commit) to the batch, and state that the M1 end-to-end loop must be re-run before any quota > 1 is claimed. Update the M1 Part A §12 `/6` inventory accordingly. |

Also implied, and to be recorded in the same freeze (not a separate change):
the T01 §4 wording reconciliation already listed in the M1 Part A §12.16
("the control chip **computes** the selection; the integration/dispatcher applies
it") must extend from one selection to the ordered batch.

The M1 Part A draft carries the companion, more granular inventory at
[§12.18](../tasks/M1_PART_A_CONTRACT_PROPOSAL.md) and the scheduling registers at
§6.2.1; the two inventories must be reconciled and frozen together. The M1
owner-review items (T03/T04/T05/T06/T07/T08/T09) remain **conditional** and are
not ADRs; none is accepted by this proposal, and no owner sign-off may be
inferred.

---

## 4. Consequences

### 4.1 Positive

- Independent stages can make progress in the same tick, reducing tick count
  without threads or concurrent bus writes.
- Determinism, atomicity, exactly-once, and write-scope guarantees are preserved
  as invariants, not weakened; quota 1 equals the rev-18 design under the
  canonical semantic comparison projection, with the scheduler snapshot
  separately deterministic.
- Per-stage queues make backpressure and starvation explicit and testable.
- Stage-assignment versioning makes scheduling changes replay-visible and forces
  a freeze/rerun instead of a silent behavior change.
- The same-tick write-conflict predicate (§2.1 invariant 9) makes shared append
  areas auditable instead of assumed safe.

### 4.2 Negative / cost

- More state in the bus (queues, in-flight set, cursor, version, metrics) and
  more snapshot/encoding surface, so **byte-identical** quota-1 snapshots are no
  longer available; equivalence is semantic (§2.8).
- A larger batch makes the single atomic commit stricter: one bad dispatch can
  reject the whole tick. This is the price of a single ordered commit and must be
  validated by tests.
- Contract churn: `/6` grows beyond the M1 Part A inventory in §3.
- Risk of misreading "in-flight" as parallelism; §5 must be enforced by lint and
  review, and no threads are introduced.

### 4.3 Neutral

- Root `cc-silicon` is untouched; SFL core, backend boundary, and the ADR-0001
  storage extension are unchanged.
- No new external dependency is required.
- No probe/codegen gate is affected; the target path stays fail-closed.

---

## 5. Alternatives considered

1. **Keep the single-active-task runtime, no staged pipeline.** Simplest, fully
   frozen, no `/6` churn. The proposal retains quota 1 as the **default and the
   baseline equivalence mode**, but does **not** treat the single-active-task
   runtime as the final architecture: the staged pipeline is the selected design
   and quota `> 1` is the measured optimization profile. This alternative is
   rejected as the permanent design, not as the baseline.
2. **Runtime threads with concurrent bus writes.** Rejected. Violates the
   paradigm no-threads/no-race invariant, ADR-0001's explicit constraints, and
   the determinism requirements. Any future parallel runtime needs a separate
   ADR with conflict checking and a deterministic commit order.
3. **One commit per dispatch.** Rejected as the default: it breaks the single
   ordered atomic commit and makes partial-tick state observable. Retained only
   as a residual question.
4. **Same-tick child visibility (re-entrancy).** Rejected. It would allow
   cross-task draft references and order-dependent semantics; next-tick
   visibility is preserved.
5. **One global queue, no stages.** Rejected for now: it does not express
   backpressure or fairness per stage. Canonical stage queues are the proposal;
   the exact backing representation is residual.
6. **Assume shared append areas are safe.** Rejected: shared appends require the
   explicit `(StoreId, field, record)` conflict predicate and an independent /
   commutative rule hashed with the contract; otherwise the batch is rejected.

---

## 6. Unresolved choices (must not be invented)

These are deliberately left open. None may be treated as frozen by this
proposal. Items that rev 3 **selected** are recorded in §2 and are no longer
listed here.

- **One ordered atomic commit** is the selected default for `Q` dispatches. On
  commit failure, the **batch failure-recovery direction is user-selected in
  principle** (rev 32 / Revision 13, 2026-10-04): fail every dispatched task
  exactly once in dispatch order with the optional-diagnostic/
  `DiagnosticId::NONE` sentinel, so no task remains `Running`, with clearing the
  in-flight set **not itself** a transition. This is **direction only — not an
  accepted mechanism and not frozen**; the exact atomic/bounded implementation is
  an `[INT]` `/6` decision (H6 block below). The verified `/5`
  single-task `fail_selected` obligation (task → `Failed`, a committed
  `DiagnosticId` when capacity allows, else the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel, never stranded) is preserved.
  The residual is only whether a future bypass needs a
  per-dispatch commit (not proposed). **Rev 21 proposed (superseded by the H6
  block below — rev 25 F2):** the failure path preflights capacity for the
  dispatched-task count and either fails all of them or ends the job in one
  terminal capacity failure — no partial failure; **this is not an established
  mechanism.** **The
  join reinsertion is also performed here in commit apply**: all committed
  children `Completed` → the parent returns to `Ready` and re-enters its own
  `stage_queues[stage_of(parent.kind)]`; any committed child `Failed` → the parent
  is failed exactly once and is never set `Ready` (H4). The realization handles an
  absent `continuation` (H2). CT07 is decision-only. The exact error names/codes
  remain open.
- **CT07 decision carrier (rev 22, open).** Because commit-apply realizes the
  join, CT07 is decorative unless it emits a concrete committed join-decision
  carrier consumed by commit-apply, or is removed from the join path. Unresolved.
- **No-`Running`/terminal diagnostic mechanism (rev 23/25/26 direction; rev 32
  selected direction, implementation BLOCKED — H6).**
  Clearing the in-flight set does **not** by itself clear a task's
  `TaskState`/`Running` state, and **clearing the in-flight set is not itself a
  transition**, so the "ends the job in one terminal capacity failure with no task
  left `Running`" claim above is **not established**. **On 2026-10-04 the user
  selected the batch failure-recovery direction in principle (rev 32):** on a
  failed atomic semantic batch commit, **every** dispatched task, in **dispatch
  order**, transitions **exactly once** to `TaskState::Failed` — a committed
  `DiagnosticId` when diagnostic/record capacity allows, otherwise the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel — so **every** dispatched task
  leaves `Running`. This **generalizes the verified deterministic `/5` single-task
  fail transition** with the same optional-diagnostic/sentinel semantics. **The
  implementation remains BLOCKED/co-freeze:** only the invariant "no dispatched
  task remains `Running` at latch" is stated, and the **exact atomic/bounded
  mechanism requires T01 integrator approval**, with the batch failure-atomicity
  details, the diagnostic budget/state mechanism, and the T02/T13 owner
  fixtures/sign-offs pending. Do
  not read the rev-21 wording as resolved. **The verified `/5` `routing.rs`
  `fail_selected` single-task obligation is preserved:** the failed task
  **always** becomes `TaskState::Failed`, attaching a committed `DiagnosticId` when
  diagnostic/record capacity allows and otherwise the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel (never stranded). **Alternative
  retained for `/6` (exact design `[INT]`):** another T01-approved atomic terminal
  path (e.g. reserve bounded terminal diagnostic slots **before** dispatch).
  **[Rev-42 note (user-accepted recommendation; T01 co-freeze pending; not a
  `/6` freeze; no code): the "implementation BLOCKED" framing of this H6 item
  is qualified by the rev-42 selected quota>1-capable bounded recovery
  mechanism to be frozen in `/6` (pre-dispatch errors leave tasks `Ready`;
  failed batch commits no semantic state; dispatch-order `Failed` recovery; no
  N pre-reservation; `NONE` fallback; state guard); exact atomic realization +
  fixtures remain open.]**
- **In-flight bound (rev 33/34 H9 removal direction accepted in principle;
  remaining `[INT]` items open).** **Current operative direction:** the `/6`
  candidate **removes `max_inflight_total`** (and with it `InflightQuotaExceeded`)
  because sequential per-tick dispatch is already bounded by
  `max_inflight_per_tick`/quota — enforced once by the dispatcher pre-dispatch
  (`SelectionBatchOverflow`) — the per-stage `stage_queue_bound`, and
  `max_tasks_total`; `TaskState::Waiting` is not in-flight; and a future cross-tick
  `Running` mode would need a **separate CDR**. The dispatcher owns the
  `Ready→Running` + in-flight mutation, and that **pre-worker `Ready→Running`
  mutation is distinguished from** the one ordered atomic semantic commit. The
  dispatch **batch bound remains the per-tick `max_inflight_per_tick` quota**;
  `tasks.in_flight` remains an **ephemeral per-tick scheduler batch only**
  (candidate) cleared at latch only after every dispatched task has a
  terminal/`Waiting`/`Progress` outcome (or the H6 recovery). **This is a
  working-basis direction only, pending T01 integrator acceptance**; no
  bound/config/hash is frozen, `InflightQuotaExceeded` is removed from the
  candidate, and the **residual-set semantics of the in-flight set and the
  dispatcher-`Ready→Running`-vs-ordered-atomic-commit relationship remain open
  `[INT]` items** (not resolved; exact error code numbers also remain open).
  **Historical rev-22/23 framing (superseded, retained):** the earlier candidate
  asserted a one-phase `max_inflight_per_tick` check with a separate residual
  `max_inflight_total` bound (`InflightQuotaExceeded`), and the rev-23 audit marked
  the phase/owner for `max_inflight_total`, the residual-set semantics, and the
  dispatcher `Ready→Running`-vs-atomic-commit relationship **BLOCKED, not resolved**
  (Revision 6). That one-phase-owner blocker is superseded by the removal direction
  above; the residual-set and dispatcher-mutation-vs-commit items stay open.
  **Rev 17 (integration-selected candidate default under user delegation,
  2026-10-05):** under the user's explicit 2026-10-05 delegation ("for
  non-critical decisions adopt the recommended choice directly; ask only for
  critical decisions"), the **M1 integration agent** selected — **under
  user-delegated integration default, not as an owner or T01 `[INT]` signoff** —
  that **`max_inflight_per_tick`/quota is the sole per-tick dispatch-count bound**
  and that the redundant **`max_dispatches_per_tick` is dropped from the candidate
  limit inventory and its validation** (mirroring CDR rev 51). This is grounded in
  the rev-33/34 H9 direction and the rev-49 audit finding that the two limits
  duplicated each other; it selects **no** dispatcher-`Ready→Running`/`in_flight`
  atomic-boundary or clear-order item, changes neither the selected H6 mechanism
  nor the H9 removal direction, and **T01/T02 must confirm/co-freeze the exact
  names/defaults/codes at `/6`**, with the T02/T13 tests pending.
- **Own-batch `DraftRef` vs Guardrails §6.1 (accepted in principle — narrow
  interpretation, user decision A, 2026-10-04).** A same-task `OwnBatch(DraftRef)`
  may exist **only as a transient wire/proposal input before commit**; commit
  validates/resolves it to a committed `TaskId` **before any persistent state**;
  no durable cursor/`WaitSet`/join points at a draft, wire, or address. The
  §6.1 text is **unamended**; the only remaining item is the **T01 integrator
  implementation-confirmation** (§2.1 invariant 6). Not a freeze.
- **Error classification (rev 21, selected).** `BackpressureCapacity` and
  `CrossTaskWriteConflict` are `CommitError`s; `SelectionBatchOverflow`,
  `DuplicateSelection`, and `DispatchBudgetExceeded` are
  **dispatcher/scheduling failures** (pre-dispatch, before any store mutation),
  not `CommitError`s; `StoreOwnerViolation`/`StageUnassigned`/`StageLayerMismatch`
  are `ManifestError`s. **Rev 33/34 (H9):** `InflightQuotaExceeded`/
  `max_inflight_total` is **removed from the candidate** (pending T01 integrator
  acceptance). **Rev 17 (integration-selected candidate default under user
  delegation, 2026-10-05):** with `max_dispatches_per_tick` dropped as the redundant
  duplicate of the sole `max_inflight_per_tick`/quota bound, the
  `DispatchBudgetExceeded` dispatcher failure is **dropped from the candidate
  inventory**; the per-tick dispatch-count bound is enforced by
  `max_inflight_per_tick`/quota (`SelectionBatchOverflow`). Not an owner/T01
  `[INT]` sign-off, no schema freeze; T01/T02 confirm/co-freeze the exact
  names/codes at `/6`. Exact codes remain open.
- The **fairness policy** (round-robin / weighted / one-per-stage) and whether a
  persistent fairness cursor is semantic state.
- Whether **in-flight tasks may be parked across ticks** (`Running` while the
  job advances) or must always complete their single transition within the tick;
  the selected M1 behavior is single-transition, and `AwaitChildren` uses
  `Waiting`, not `Running`.
- The **exact stage set and names**, and whether they mirror the T01 §2 store
  partitions one-to-one.
- Whether CT03 or the dispatcher computes the fair subset (the
  `SelectionEntry`/`SelectionBatch` shape and the
  `dispatch_cursor`-as-position-of-current-task semantics are **selected**).
- **Backpressure semantics**: defer vs. reject vs. both, and per-stage vs.
  global throttle computation.
- Whether **cancel** is re-checked between dispatches or only once per tick
  (selected: once per tick, matching M1).
- The exact inner encoding of `bus.report.metrics` (the register placement in
  `report` is **selected**).
- Whether `stage_assignment_version` is config-only or bus state, and the
  migration policy for an in-progress job.
- The interaction between `Q` and `max_proposals_per_tick` (per-dispatch cap vs.
  summed batch cap) and the exact capacity-error codes; the bounds are selected,
  the numeric codes are not.
- Exact `CommitError`/chip-diagnostic/`ManifestError`/`ConfigError` names and
  numeric codes for the new pipeline rules (must be frozen together at `/6`, no
  new error families unless the integrator accepts them).

---

## 7. Non-claims

- This ADR is **PROPOSED**; it authorizes nothing and supersedes nothing.
  `t01-c01-c06/5` remains current.
- No code, chip, driver, report, queue, metric, or test described here exists
  today. The compiler is **not implemented**; there is no parser, IR, codegen,
  probe, or pass rate.
- No throughput, tick-count, or performance improvement is claimed. Quota 1 is
  the only baseline; increases require measured evidence and acceptance.
- Target identity is frozen by ADR-0001, but every concrete ABI value remains
  **UNVERIFIED**; nothing here changes that or the codegen fail-closed gate.
- No root-framework change, no unsafe code, no threads, and no new dependency
  are proposed. The `/5` hash and all accepted task documents remain unedited by
  this proposal.

---

## 8. Revision record

| Date | Change | Authority |
|---|---|---|
| 2026-10-04 | Initial proposal: deterministic sequential staged pipeline with bounded in-flight tasks per tick; quota=1 baseline; per-stage queues and versioned stage assignment; one ordered atomic commit; committed-ID cross-tick joins; fairness/backpressure/cancel/fail/replay; metrics/acceptance evidence; exact `/6` amendment inventory. PROPOSED only; no existing document changed | Subagent draft for integrator/owner review |
| 2026-10-04 | Revision 2: aligned with M1 Part A rev 19 (user binding direction). Clarified quota 1 is the **baseline equivalence mode only**, not the final optimization profile; added measured `Q > 1` optimization profile; linked the M1 §6.2.1/§6.2.2/§7/§10.5/§12.18 realization; recorded that the M1 owner reviews are conditional and that no owner sign-off is implied. Still **PROPOSED**; no code, no freeze, no root-framework change | User pipeline direction; M1 rev 19 integration |
| 2026-10-04 | Revision 3 (rev-20 re-review): removed stale "quota 1 only"/"status quo"/"only adds an option" language; quota-1 equivalence is the **canonical semantic comparison projection** (scheduler-only registers excluded) with a separately deterministic scheduler snapshot; selected `SelectionEntry`/`SelectionBatch`, `dispatch_cursor` semantics, canonical `stage_queues` queue source, `report.metrics`/`report.dispatched`, and the `CrossTaskWriteConflict` predicate + VF04-extended verifier; corrected the T13 table (VF12 is `IrInterpretChip`, not the pipeline/interpreter-conflict verifier); aligned §6 open decisions with the selected defaults; added T04 to the conditional owner list. Still **PROPOSED**; no code, no freeze, no root-framework change | User rev19 re-review integration; M1 rev 20 |
| 2026-10-04 | Revision 4 (rev-21): recorded the user's **in-principle** direction (2026-10-04, direction only, not a freeze); removed the stale "single-active-task runtime" wording in §1.1 and restated the quota-1 singleton as a projection of the ordered in-flight set; scoped invariant 9 (write conflicts) to same-owner-group appends/same-record patches and rejected a shared-writer carveout; added **single writer per wire** (CT03 adapter → `wires.selection`; dispatcher → `wires.selected`; worker adapter → `wires.proposals`; driver step 6 → `bus.report`); selected the **commit-apply join reinsertion** (CT07 decision-only) and the **error classification** (`BackpressureCapacity`/`CrossTaskWriteConflict` = `CommitError`; selection/inflight/dispatch-budget = dispatcher/scheduling failures; stage errors = `ManifestError`); noted the VF04 batch-conflict audit as a proposed/residual extension. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current | User in-principle direction (2026-10-04); M1 rev-21 review-integration subagent |
| 2026-10-04 | **Revision 5 (rev-22).** Integrated the accumulated T02/T13 pipeline review findings without changing the selected direction: flagged the **own-task-batch draft-key vs Guardrails §6.1** wording conflict as an explicit authority-reconciliation request (invariant 6); recorded the **single-phase in-flight bound** (dispatcher `SelectionBatchOverflow`; `InflightQuotaExceeded` = `max_inflight_total` only) and the dispatcher-owned `Ready→Running` mutation; recorded the **CT07 committed join-decision carrier** requirement (decision-only is decorative otherwise); reconciled the **terminal diagnostic-capacity path** with the no-`Running` invariant; confirmed the single writers (`wires.selection`/`wires.selected`/`wires.proposals`/`bus.report`) and `dispatch_cursor`-per-invocation; kept VF04's batch audit proposed/residual (not defined today; VF12 unrelated). Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; all owners/integrator pending | M1 rev-22 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 6 (rev-23).** Integrated the independent rev-22 audit H2–H11: **H6** — the no-`Running`/terminal diagnostic mechanism is downgraded to **BLOCKED** (clearing the in-flight set does not clear `Running`; only the invariant is stated; invariant 3/§2.4 step 6/§6); **H9** — the single phase/owner for `max_inflight_total`, the residual-set semantics, and the dispatcher `Ready→Running`-vs-atomic-commit relationship are marked **BLOCKED**, not "resolved to a single phase" (§6); **H4** — join semantics: all committed children `Completed` → parent `Ready`, any child `Failed` → parent `Failed` once, never `Ready` (§6); **H2** — resume reinserts into `stage_queues[stage_of(parent.kind)]` and handles an absent `continuation`; **H10** — stale rev-21 reference corrected to rev 21–24 (extended by rev 25 F6) and `AwaitChildren` added to the one-transition invariant; **H11** — the `TerminatorMissing` "trigger fixed" wording is out of scope here but the pipeline references were audited. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; all owners/integrator pending | M1 rev-23 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 7 (rev-25).** Integrated the rev-24 independent audit **F2/F4/F6** and the verified `/5` failure guarantee: **F2** — §2.1 invariants 3/4, §2.4 step 6, §2.7, §2.8 item 3, and §6 now preserve the frozen `/5` `routing.rs` `fail_selected` **single-task** obligation (failed task → `Failed` with diagnostic when capacity allows) and mark the **batch** no-`Running`/terminal mechanism **BLOCKED** (clearing the in-flight set does not clear `TaskState`/`Running`), with explicit `/6` alternatives (reserve bounded terminal diagnostic slots before dispatch, or another T01-approved atomic terminal path); **F4** — §2.1 invariant 6 strengthens the own-task-batch key vs Guardrails §6.1 note to **candidate-only, not a resolution**; **F6** — §1.1 pointer is rev 21–24. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H6 BLOCKED; all owners/integrator pending | M1 rev-25 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 8 (rev-26).** Integrated the rev-25 independent audit items 1–12 (main findings 1–8 plus nested C/D/E request items; M1 proposal `§20`): **item 1 (H6)** — §2.1 invariant 4, §2.4 step 6, §2.7, §2.8 item 3, and §6 now state the verified `/5` `fail_selected` **single-task** semantics exactly (the task **always** becomes `TaskState::Failed` — a committed `DiagnosticId` when capacity allows, else the `TaskState::Failed(DiagnosticId::NONE)` sentinel, never stranded) and record the batch fan-out as **BLOCKED** with the `/6` alternatives (generalize the deterministic all-dispatched fail transition with the same optional-diagnostic/sentinel semantics, or another T01-approved atomic terminal path), with §2.7/§6 explicitly marking the all-dispatched fail-exactly-once reading **candidate only, not accepted/default/established**; **item 6** — §1.1 pointer is **rev 21–26**; **items 9/10** — AB1a/AB1b/AB2 remain selected **draft direction** and the own-task-batch key vs Guardrails §6.1 remains an **open reconciliation request**. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending | M1 rev-26 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 9 (rev-28).** Docs-only pointer reconciliation: advanced the §1.1 M1-draft pointer to **rev 21–28**; no ADR decision, invariant, or interface text changed. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending | M1 rev-28 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 10 (rev-29).** Docs-only historical-documentation correction. **Erratum:** Revision 6 (rev-23) recorded H10 as correcting the stale rev-21 reference to **rev 21–24**; per the M1 proposal `§18.7` the rev-23 correction was to **rev 21–23**, with rev 24/25 extending the aligned set to **rev 21–24** (the Revision 6 text is preserved above; this erratum supplies the correct chronology — no approval is implied by either statement). Advanced the §1.1 M1-draft pointer from **rev 21–28** to **rev 21–29**. No ADR decision, invariant, or interface text changed. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending | M1 rev-29 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 11 (rev-30).** Records the two explicit user in-principle decisions (2026-10-04) in the operative text; no acceptance of a contract or `/6` freeze. **§2.1 invariant 6** and **§6** now state the **narrow interpretation** of Guardrails §6.1 (**decision A**): a same-task `OwnBatch(DraftRef)` may exist **only as a transient wire/proposal input before commit**, validated and resolved to a committed `TaskId` **before any persistent state**, so no durable cursor/`WaitSet`/join points at a draft/wire/address; the §6.1 text is **unamended** and **T01 implementation-confirmation is pending**. The awaited-child **`WaitSet`-only** decision (**decision B**: `TaskState::Waiting(WaitSet)` is the sole source, no duplicate `ContinuationRecord.awaited`, T01 §4 supersession in `/6` only with no `/5` edit, T01/T05 acceptance pending) is cross-recorded in the M1 proposal and CDR (its T05 text is owned by those documents). Advanced the §1.1 M1-draft pointer to **rev 21–30**. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending | User in-principle decisions (2026-10-04); M1 rev-30 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 12 (rev-31).** Docs-only history-gap repair (M1 proposal rev-31). Advanced the §1.1 M1-draft pointer from **rev 21–30** to **rev 21–31**, matching the M1 proposal's new current revision, whose §16 revision record had skipped the operative **rev 29** docs-only historical-documentation correction (now supplied) and which adds the **rev 31** row. No ADR decision, invariant, or interface text changed; the Revision 10/11 rows are preserved verbatim. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending | M1 rev-31 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 13 (rev-32).** Records the user's **2026-10-04 in-principle acceptance of the H6 batch-failure recovery direction** in the operative text (matching M1 proposal rev 32 and CDR rev 34); no acceptance of a contract or `/6` freeze. **§2.1 invariant 4**, **§2.4 step 6**, **§2.7** (Fail + RESIDUAL), **§2.8 item 3**, and **§6** now state the selected direction: on a failed atomic semantic batch commit, **every** dispatched task, in **dispatch order**, transitions **exactly once** to `TaskState::Failed` — a committed `DiagnosticId` when diagnostic/record capacity allows, otherwise the `TaskState::Failed(DiagnosticId::NONE)` sentinel — so **every** dispatched task leaves `Running`; **clearing the in-flight set is not itself a transition** and does not clear `TaskState`/`Running`. This **generalizes the verified `/5` `fail_selected` single-task sentinel obligation**, which is **preserved and not weakened**. **Direction only:** the **exact atomic/bounded implementation requires T01 integrator approval**, the batch failure-atomicity details and the diagnostic budget/state mechanism remain **co-freeze**, and the **T02/T13 owner fixtures/sign-offs are pending**; the mechanism is **not** implemented or frozen, and the alternative (another T01-approved atomic terminal path, e.g. reserve bounded terminal diagnostic slots before dispatch) is retained. **H9 unchanged (BLOCKED).** Advanced the §1.1 M1-draft pointer from **rev 21–31** to **rev 21–32**. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H1 accepted in principle only; all owners/integrator pending | User in-principle decision (2026-10-04); M1 rev-32 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 14 (rev-33/34).** Records the user's **2026-10-04 in-principle acceptance of the H9 in-flight scheduling-ownership direction** in the operative text (matching M1 proposal rev 33/34, CDR rev 35/36, T02 rev 28); no acceptance of a contract or `/6` freeze. **§6** (in-flight bound + error classification) now states the selected direction: accept **removing `max_inflight_total`** from the `/6` candidate because sequential per-tick dispatch is already bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; `TaskState::Waiting` is not in-flight; a future cross-tick `Running` mode would need a **separate CDR**. The dispatch **batch bound remains the per-tick `max_inflight_per_tick` quota**; `tasks.in_flight` remains an **ephemeral per-tick scheduler batch only** (candidate) cleared at latch only after every dispatched task has a terminal/`Waiting`/`Progress` outcome (or the H6 recovery); the dispatcher's **pre-worker `Ready→Running` mutation is distinguished from** the one ordered atomic semantic commit; `InflightQuotaExceeded` is **removed from the candidate and the dispatcher-error list**. **Direction only:** **pending T01 integrator acceptance**, no bound/config/hash is frozen, and the residual-set semantics of the in-flight set and the dispatcher-`Ready→Running`-vs-ordered-atomic-commit relationship remain **open `[INT]` items**. H6 unchanged (direction accepted in principle, implementation BLOCKED). Advanced the §1.1 M1-draft pointer from **rev 21–32** to the current **rev 21–34**. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H1 accepted in principle only; all owners/integrator pending | User in-principle decision (2026-10-04); M1 rev-33/34 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 15 (rev-35).** Docs-only consistency cleanup of **stale operative candidate prose** (no new decision, no acceptance, no `/6` change; matches M1 proposal rev 35, CDR rev 37, T02 rev 29). **§6 in-flight bound:** the operative paragraph still opened by stating the rev-22/23 *candidate* (`max_inflight_per_tick` checked once with a separate residual `max_inflight_total` bound denoted by `InflightQuotaExceeded`) **before** saying it was removed, which read as a contradiction; it is rewritten so the **removal direction is the current operative statement** and the rev-22/23 one-phase-owner framing is explicitly marked **superseded history retained**. §2.1 invariant 8 and §5 already carried the removal direction and are unchanged. H6/H9 statuses unchanged: H9 removal direction **accepted in principle only, pending T01 integrator acceptance**, with the residual-set and dispatcher-mutation-vs-commit `[INT]` items **open**; H6 **direction accepted in principle, implementation BLOCKED**. Advanced the §1.1 M1-draft pointer from **rev 21–34** to the current **rev 21–35**. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H1 accepted in principle only; all owners/integrator pending | M1 rev-35 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Revision 16 (rev-36).** Docs-only **amendment-scope clarification** of the §3 inventory (no new decision, no acceptance, no `/6` change; matches M1 proposal rev 36 and CDR rev 38). The **§3 `compiler/src/bus.rs`** amendment row said the per-limit preflight would extend with a generic "in-flight bound", which could be misread as re-introducing a **total** in-flight bound after the rev-33/34 H9 removal direction; it now reads the **per-tick `max_inflight_per_tick`/quota bound only**, explicitly noting there is **no separate total in-flight bound**. No other §3 row, invariant, or decision text changed. The §1.1 M1-draft pointer is **unchanged at the current rev 21–35** (the shared M1 proposal advances to rev 36 for its own header current-state pointer and typo fixes; the current-state range does not change). H6/H9 statuses unchanged: H9 removal direction **accepted in principle only, pending T01 integrator acceptance**, with the residual-set and dispatcher-mutation-vs-commit `[INT]` items **open**; H6 **direction accepted in principle, implementation BLOCKED**. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` current; H1 accepted in principle only; all owners/integrator pending | M1 rev-36 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Revision 17 (CDR rev 51 mirror).** Records an **integration-selected candidate default under explicit user delegation** (docs-only; mirrors CDR rev 51; M1 proposal stays rev 36). Under the user's explicit **2026-10-05 delegation** ("for non-critical decisions adopt the recommended choice directly; ask only for critical decisions"), the **M1 integration agent** — **under user-delegated integration default, not as an owner or T01 `[INT]` signoff** — selected that **`max_inflight_per_tick`/quota is the sole per-tick dispatch-count bound** and that the redundant **`max_dispatches_per_tick` is dropped from the candidate limit inventory and its validation** (with the now-duplicate `DispatchBudgetExceeded` dispatcher failure dropped from the candidate inventory). Recorded in **§3** (`compiler/src/bus.rs` and `compiler/src/limits.rs` + `compiler/src/target.rs` rows) and **§6** (in-flight bound + error classification). Grounded in the rev-33/34 H9 direction (per-tick dispatch is already bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`) and the rev-49 audit finding that the two limits duplicated each other. It selects **no** dispatcher-`Ready→Running`/`in_flight` atomic-boundary or clear-order item, changes neither the selected H6 mechanism nor the H9 removal direction, and resolves none of the residual proof/defaults/fixtures. **T01/T02 must confirm/co-freeze the exact names/defaults/codes at `/6`, and the T02/T13 tests remain pending.** This is a **delegated candidate default**, **not** an owner/T01 `[INT]` sign-off, **not** a schema freeze, and **not** code. H9 `max_inflight_total`-removal direction **accepted in principle only, pending T01 integrator acceptance**, with the residual-set and dispatcher-mutation-vs-commit `[INT]` items **open**; H6 **direction accepted in principle, implementation BLOCKED** (its exact atomic/bounded realization, in-flight clear order relative to H6, residual proof, exact defaults/error codes/fixtures unresolved). Quota `max_inflight_per_tick = 1` remains the **baseline equivalence mode**; quota `> 1` remains **measured and separately accepted only**. Still **PROPOSED**; no code, no freeze, no root-framework change; `/5` unchanged; H1 accepted in principle only; all owners/integrator pending | CDR rev-51 mirror subagent (DeepSeek doc integrator) |

---

## 9. References

- [ADR-0001: Compiler Dynamic Arena Extension and Frozen Target Identity](ADR-0001-COMPILER-DYNAMIC-ARENA.md)
- [T01: Compiler Bus and Protocol Freeze](../tasks/T01_COMPILER_CONTRACT.md) §4, §5, §7.1
- [PARALLEL_EXECUTION.md](../tasks/PARALLEL_EXECUTION.md) §2, §5, §6
- [T02: Control, Scheduling, and Diagnostic Chips](../tasks/T02_CONTROL_CHIPS.md)
- [T13: Verification Chips, Replay, and Differential Checking](../tasks/T13_VERIFICATION_CHIPS.md)
- [M1 Part A Contract Proposal](../tasks/M1_PART_A_CONTRACT_PROPOSAL.md) §6.2, §7, §10, §12, §15
- [M1 Vertical-Slice Acceptance (frontend half)](../tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md) §6.3, §8
- [M1 Target Acceptance](../tasks/M1_TARGET_ACCEPTANCE.md)
- [SILICON_PARADIGM_SPEC.md](SILICON_PARADIGM_SPEC.md)
- [SFL_CONTRACT.md](SFL_CONTRACT.md)
