# A16 — T02 Control Protocol Requirements (A-PTS-06)

| Field | Value |
|---|---|
| Status | **DESIGN ONLY — contract-requirement package. Not a freeze, not a schema change, not implementation authorization.** No source, test, contract, Cargo, `mod.rs`, shared-index, or other non-report file was modified; no commit, push, or agent was started; no test suite was run. |
| Task | A-PTS-06 under [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) Stage A |
| Date | 2026-11-07 |
| Scope | T02 control, commit, join, cancellation, progress, and phase advancement: CT01–CT14 |
| Frozen baseline | `t01-c01-c06/39`; hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` ([CONTRACT_VERSION](../../../compiler/contracts/CONTRACT_VERSION)) |
| Write scope of this task | This report only: `docs/reviews/dispatch/A16_CONTROL_PROTOCOL_REQUIREMENTS.md` |
| Non-claims | No chip, schema, manifest, routing row, test, probe, corpus run, pass rate, throughput, or freeze is claimed. The report does not authorize Stage B coding. |

## 0. Status vocabulary and reading rules

Every claim in this report carries one of these markers. A marker is not decoration: it is the boundary the integrator must preserve when turning this package into a freeze.

| Marker | Meaning |
|---|---|
| `[FROZEN /39]` | A normative shape or rule in the current `t01-c01-c06/39` artifact (inherited from `/6`–`/9` and later slices). Changing it requires an integrator freeze, a version bump, a hash recomputation, and dependent-test revalidation. |
| `[IMPLEMENTED /39]` | Code in `compiler/src/` implements the behavior, with at least one test in `compiler/tests/`. Implemented is not the same as complete or universally proven. |
| `[PARTIAL /39]` | Code implements one path (usually quota 1) or a projection; a named gap remains. |
| `[ABSENT]` | No type, field, chip, wire, or test exists today. |
| `[PROPOSED A16]` | A candidate requirement/recommendation in this report. It is not frozen and authorizes nothing. |
| `[CONFLICT]` | Two authoritative or candidate sources make incompatible statements; the freeze work order must resolve it. |
| `[DEFERRED]` | Explicitly outside this package's freeze (for example quota > 1 performance work). |

Authority order used here is the `AGENTS.md` §2 order: explicit user instruction > accepted ADR > accepted engineering contract > architecture design > existing implementation. [CT_FREEZE_01](../../tasks/CT_FREEZE_01.md), [CT_FREEZE_02](../../tasks/CT_FREEZE_02.md), and [CT_FREEZE_03](../../tasks/CT_FREEZE_03.md) are drafts; they do not outrank `/39`. Where a draft and `/39` disagree, this report records the conflict and recommends a resolution; it does not silently rewrite either.

## 1. Goal, scope, and gap statement

### 1.1 Goal

Define the **smallest shared control protocol** that lets the T02 chips be implemented against the frozen `/39` task/result envelope without private state, without a second scheduler, and without losing quota-1 determinism. The package must make the following testable:

1. task/result/continuation/WaitSet/queue/ledger shapes;
2. dispatch ordering and the dispatch/commit boundary;
3. the task guard;
4. proposal validation;
5. the one ordered atomic commit;
6. await-all join and result delivery;
7. failure closure and bounded recovery;
8. cancel precedence and idempotence;
9. the progress budget;
10. phase/stage gates;
11. recovery selection;
12. exactly-once completion and consumption.

### 1.2 In scope

- T02 CT01–CT14 semantic interfaces and state transitions.
- The quota-1 routing shell as the baseline equivalence mode.
- The frozen `/39` envelopes that the control loop must reuse.
- Conflicts with the candidate freeze drafts `CT_FREEZE_01..03`.
- Ordered freeze items and Stage B implementation work orders with exact paths.

### 1.3 Out of scope

- C language rules, type/constant/layout/IR semantics, target ABI, and code generation.
- Host file I/O, subprocess execution, and artifact persistence.
- Quota > 1 throughput claims and performance measurements (`[DEFERRED]`; the code may contain a bounded quota path, but this package does not authorize a quota > 1 acceptance claim).
- Any schema, contract, source, test, Cargo, or `mod.rs` edit.
- Any test execution or pass-rate claim.

### 1.4 Executive summary

1. `/39` already freezes `Task`, `TaskState`, `WaitSet`, `ContinuationRecord`, `ResultValue`, `ResultRecord`, `Proposal`, and `TaskDraft`, and provides the stable `CommitError` carrier. The control package should **reuse them** and add **no committed record family, no `ResultValue` variant, and no `TaskState` variant** for the M1 control loop.
2. The current runtime is a **quota-1 baseline routing shell**, not the T02 chip topology. It already implements select → `Running` → worker proposals → one commit → recovery/join → report; the code contains a bounded batch path, but its report projection is quota-1 and its commit order lacks the dispatch ordinal. CT01–CT14 chips do not exist; the shell acts as their driver.
3. The largest conflicts are the selection/commit order key (group-first vs stage/dispatch-first), the `SelectionBatchOverflow` carrier (`CommitError` vs dispatcher error), the `Stage`/`stage_of` vocabulary, `ContinuationRef`/continuation materialization, progress-plus-write coexistence, and `in_flight` clear ownership.
4. The smallest queue/ledger design is: keep `tasks.ready` canonical at quota 1, keep `tasks.in_flight` ephemeral, optionally add a bounded `tasks.waiting` index, and extend `TickRecord` with per-dispatch outcomes only if the integrator accepts the snapshot impact. `stage_queues` are a quota > 1 extension, not a quota-1 requirement.
5. Exactly-once completion is **already structurally guarded** by `Running` state, `DuplicateCompletion`, `ResultRecord.consumed`, the join `Waiting` state, and `try_fail_task`'s `Running` guard. The remaining gaps are consumer identity on result consumption, progress-failure capacity preflight, and dangling join children.
6. The join is a **commit-apply invariant**: await-all, terminal-state-only, no child-result consumption. Freeze that and add the `join_then_replay_retry` fixture.
7. Cancel is pre-selection, pre-budget, idempotent, and job-terminal. Ready tasks stay `Ready` in the current implementation; that disposition must be frozen or replaced by an explicit `Cancelled` state. Silence is not acceptable.
8. H6 recovery is implemented in the shell: dispatch-order `Failed`, committed diagnostic or `DiagnosticId::NONE`, state guard, in-flight clear. The exact latch owner and the all-paths no-residual proof remain open.
9. `CT_FREEZE_01..03` are proposals, not freezes. Several recommendations contradict `/39` and each other (continuation field list, progress-plus-write prohibition, `SelectionBatchOverflow` classification, `in_flight` clear point). They must be reconciled by the freeze work orders in §8 before any CT chip code.
10. No CT chip implementation is authorized by this report. The Stage B work orders in §9 are ordered after the relevant freeze item, not before it.

## 2. Frozen baseline: what `/39` actually provides

### 2.1 Task/result/continuation/WaitSet shapes

The following are the frozen shapes in `compiler/src/task.rs`. They are the **only** shared envelope the control package may rely on.

| Shape | Field | Type | Writer / owner | Persistence | Requirement / invariant | Status |
|---|---|---|---|---|---|---|
| `Task` (`task.rs:1757-1781`) | `id` | `TaskId` | commit/bootstrap | `arenas.tasks` | Stable; never reused; dense index; `u32::MAX` is `NONE` | `[FROZEN /39]` |
| | `kind` | `TaskKind` | producer | tasks | Registered; group/local encoding stable; reserved local `0..=15` | `[FROZEN /39]` |
| | `payload` | `Payload { refs: Vec<RecordRef> }` | producer | tasks | Committed `RecordRef`s only; no draft/wire/address | `[FROZEN /39]` |
| | `owner` | `ChipId` | producer | tasks | Must equal the routed chip; mismatch is `ChipNotOwner`/`OwnerMismatch` | `[FROZEN /39]` |
| | `parent` | `Option<TaskId>` | producer | tasks | Dangling parent rejected; depth bounded by `max_task_depth` | `[FROZEN /39]` |
| | `continuation` | `Option<ContinuationId>` | producer | tasks | Committed ID only; `ContinuationRef` is **not** wired into this field (CT_FREEZE_01 OPEN-10's pointer recommendation is aligned; OPEN-02's `ContinuationRef` migration is the conflict) | `[FROZEN /39]`, `[CONFLICT]` with CT_FREEZE_01 OPEN-02 |
| | `state` | `TaskState` | dispatcher/commit/join/recovery | tasks | Exactly one outcome per dispatch; `Running` never at latch | `[FROZEN /39]` |
| | `enqueue_ordinal` | `u64` | commit/bootstrap | tasks | Monotonic; part of the scheduling and commit keys | `[FROZEN /39]` |
| | `ready_tick` | `u64` | commit/bootstrap/reinsert | tasks | Selectable only when `ready_tick <= control.tick`; new tasks use `tick + 1` | `[FROZEN /39]` |
| | `progress_count` | `u32` | commit | tasks | `count + 1 <= max_task_progress`; zero disables `Progress` | `[FROZEN /39]` |
| | `progress_ordinal` | `u64` | commit | tasks | Strictly increasing per reschedule | `[FROZEN /39]` |
| `TaskState` (`task.rs:1676-1707`) | `Ready` | — | dispatcher/join/progress | tasks | Selectable when `ready_tick <= tick`; in no waiting set | `[FROZEN /39]` |
| | `Running` | — | dispatcher | tasks | Only the dispatcher sets `Ready → Running`; transient for one tick | `[FROZEN /39]` |
| | `Waiting(WaitSet)` | `WaitSet` | commit apply | tasks | Sole awaited-child/host source; in no ready queue | `[FROZEN /39]` |
| | `Completed(ResultId)` | `ResultId` | commit apply | tasks | Terminal; exactly one result | `[FROZEN /39]` |
| | `Failed(DiagnosticId)` | `DiagnosticId` | commit apply/recovery/join | tasks | Terminal; committed ID or `NONE` sentinel | `[FROZEN /39]` |
| `WaitSet` (`task.rs:1709-1716`) | `children` | `Vec<TaskId>` | commit apply | tasks | Committed child IDs; existing code requires `child.parent == Some(parent)`; may be empty only for host-only wait | `[FROZEN /39]` |
| | `host_request` | `Option<HostRequestId>` | commit apply | tasks | Exactly one host request for a host wait; never a draft | `[FROZEN /39]` |
| `ContinuationRecord` (`task.rs:1546-1576`) | `production` | `TaskKind` | append path | `arenas.continuations` | Frozen 9-field shape; **no `awaited`**; no draft/wire/address | `[FROZEN /39]` |
| | `cursor` | `TokenId` | append path | continuations | Committed token cursor (EOF allowed) | `[FROZEN /39]` |
| | `context` | `ParseContext` | append path | continuations | Closed 10-member vocabulary, discriminants 0–9 | `[FROZEN /39]` |
| | `binding_power` | `u16` | append path | continuations | Precedence level | `[FROZEN /39]` |
| | `scope` | `Option<ScopeId>` | append path | continuations | Committed scope or none | `[FROZEN /39]` |
| | `parent` | `Option<NodeId>` | append path | continuations | Committed parent node; `None` only for the TU root | `[FROZEN /39]` |
| | `partial_children` | `Vec<NodeId>` | append path | continuations | Committed nodes; contiguous ordinals | `[FROZEN /39]` |
| | `next_child_ordinal` | `u32` | append path | continuations | `max(ordinal) + 1` | `[FROZEN /39]` |
| | `previous` | `Option<ContinuationId>` | append path | continuations | Committed predecessor; acyclic chain | `[FROZEN /39]` |
| `ResultValue` (`task.rs:1727-1739`) | `Empty` / `Ack` / `Record(RecordRef)` / `Records(Vec<RecordRef>)` / `Diagnostic(DiagnosticId)` | — | commit apply | `arenas.results` | Five variants; **no `DraftRecords`** | `[FROZEN /39]` |
| `ResultRecord` (`task.rs:1741-1754`) | `task` | `TaskId` | commit apply | results | Producing task | `[FROZEN /39]` |
| | `kind` | `TaskKind` | commit apply | results | Equals the producing task kind | `[FROZEN /39]` |
| | `value` | `ResultValue` | commit apply | results | Tagged payload; no draft variant | `[FROZEN /39]` |
| | `version` | `u64` | commit apply | results | Store/protocol version; currently `0` at creation | `[FROZEN /39]` |
| | `consumed` | `bool` | consumer | results | Exactly-once consumption; `ResultAlreadyConsumed` | `[FROZEN /39]` |
| `Proposal` (`task.rs:1869-1990`) | `Enqueue(TaskDraft)` | 0 | worker adapter | wires | Visible next tick; parent/destination checked | `[FROZEN /39]` |
| | `Complete { task, value }` | 1 | worker adapter | wires | Exactly one transition; result produced once | `[FROZEN /39]` |
| | `Fail { task, diagnostic }` | 2 | worker adapter | wires | Exactly one transition; diagnostic committed once | `[FROZEN /39]` |
| | `AwaitHost { task, request }` | 3 | worker adapter | wires | Exactly one transition; request persisted once | `[FROZEN /39]` |
| | `StorePatch(StorePatch)` | 4 | worker adapter | wires | Field-scoped, owner/version checked | `[FROZEN /39]` |
| | `AppendRecords { task, batch }` | 5 | worker adapter | wires | At most one per task per batch; 1:1 bodies | `[FROZEN /39]` |
| | `Progress { task, ordinal }` | 6 | worker adapter | wires | Reinsert next tick; ordinal advances; count bounded | `[FROZEN /39]` |
| | `AwaitChildren { task, children }` | 7 | worker adapter | wires | Non-empty; committed children or own-batch keys | `[FROZEN /39]` |
| `TaskDraft` (`task.rs:1791-1810`) | `kind` / `payload` / `owner` / `parent` / `continuation` | — | producer | wire only | `continuation` is `Option<ContinuationId>`; **not** `ContinuationRef` | `[FROZEN /39]`, `[CONFLICT]` |
| `StorePatch` (`task.rs:1812-1835`) | `owner` / `task` / `version` / `store` / `field` / `op` / `target` / `value` | — | worker adapter | wires → `patch_log` | Version guard; field declared; config read-only | `[FROZEN /39]` |
| `AppendBatch` (`task.rs:1848-1867`) | `records: Vec<RecordDraft>` / `bodies: Vec<G1DraftBody>` | — | worker adapter | wires → arenas | 1:1 positional; at most one batch per task | `[FROZEN /39]` |

### 2.2 Control, queue, and ledger shapes

The following are the frozen control registers and report shapes in `compiler/src/bus.rs`.

| Shape | Field | Type | Writer / owner | Persistence | Requirement / invariant | Status |
|---|---|---|---|---|---|---|
| `Control` (`bus.rs:786-820`) | `tick` | `u64` | motherboard/driver | control | Lamport tick; wraps only at `u64::MAX` | `[FROZEN /39]` |
| | `job_state` | `JobState { Idle, Running, Finished, Failed }` | CT01/CT13/CT09 apply path | control | `Idle` before start; terminal after finish/cancel | `[FROZEN /39]` |
| | `stage` | `Stage { Init, Preprocess, Lex, Parse, SymbolsTypes, Semantic, LayoutInit, Ir, Optimize, Target, Done }` | CT09 apply path | control | Reporting/phase label; **not** the dispatch key today | `[FROZEN /39]`, `[CONFLICT]` with `stage_of` |
| | `selected` | `Option<TaskId>` | dispatcher | control | Quota-1 projection; not the canonical batch | `[FROZEN /39]` |
| | `next_enqueue_ordinal` | `u64` | commit/bootstrap | control | Monotonic | `[FROZEN /39]` |
| | `next_diagnostic_ordinal` | `u64` | commit/recovery | control | Monotonic | `[FROZEN /39]` |
| | `budget_exhausted` | `bool` | driver | control | Latched by tick budget | `[FROZEN /39]` |
| | `cancel_requested` | `bool` | driver | control | Latched by cancel pin; never cleared today | `[FROZEN /39]`, `[PARTIAL]` |
| `TaskStore` (`bus.rs:822-843`) | `ready` | `Vec<TaskId>` | dispatcher/commit/bootstrap | tasks | Single canonical queue at quota 1; enqueue order | `[FROZEN /39]`, `[PARTIAL]` vs `stage_queues` |
| | `active` | `Option<TaskId>` | dispatcher | tasks | Quota-1 projection of the dispatch | `[FROZEN /39]` |
| | `in_flight` | `Vec<TaskId>` | dispatcher | tasks | Ephemeral per-tick batch; empty at latch | `[FROZEN /39]`, `[PARTIAL]` clear owner |
| `TickMetrics` (`bus.rs:845-854`) | `dispatched` | `u32` | driver | report | Count only; full metrics deferred | `[FROZEN /39]` |
| `TickRecord` (`bus.rs:856-865`) | `dispatched` | `Vec<TaskId>` | driver step 6 | report | Canonical dispatch order; quota-1 projects one | `[FROZEN /39]` |
| | `metrics` | `TickMetrics` | driver | report | Count only | `[FROZEN /39]` |
| | `selected` | `Option<TaskId>` | driver | report | Quota-1 projection | `[FROZEN /39]` |
| `CompilerWires` (`bus.rs:902-911`) | `proposals` | `Vec<TaggedProposal>` | each worker adapter | wires | Emission order; no cross-worker writes | `[FROZEN /39]` |
| | `selected` | `Option<TaskId>` | dispatcher | wires | Quota-1 projection | `[FROZEN /39]` |
| | `phase_advance_requested` | `bool` | CT09 adapter (proposed) | wires | Unused today | `[FROZEN /39]`, `[PARTIAL]` |
| `CompilerPins` (`bus.rs:922-933`) | `start_job` | `bool` | Host | pins | Unused by the shell today | `[FROZEN /39]`, `[ABSENT]` handler |
| | `cancel` | `bool` | Host | pins | Consumed by `clock_tick`/`clock_tick_with` | `[FROZEN /39]` |
| | `host_responses` | `Vec<HostResponse>` | Host | pins | Not processed by the shell today | `[FROZEN /39]`, `[ABSENT]` CT02 |
| | `tick_budget_reached` | `bool` | Host | pins | Latches `budget_exhausted` | `[FROZEN /39]` |
| `TickReport` / `TickOutcome` (`routing.rs:151-200`) | `Idle` / `BudgetExhausted` / `Cancelled` / `Executed` / `CommitFailed` / `Joined` | — | driver | return value | Proposed relocation to `report.rs`; no `report.rs` exists | `[IMPLEMENTED /39]` (shell-local) |
| `CommitReport` (`commit.rs:497-516`) | `enqueued` / `completed` / `failed` / `waiting` / `rejoined` / `patches` / `appended` | — | commit apply | return value | One per successful commit | `[IMPLEMENTED /39]` |

### 2.3 Current quota-1 routing behavior

`RoutingShell::propagate_inner` (`compiler/src/routing.rs:301-540`) is the de facto driver:

1. Clears `control.selected`, `tasks.active`, `wires.selected` (`:306-309`).
2. If `control.cancel_requested`, sets `JobState::Failed`, clears selection/active/in-flight, returns `Cancelled` (`:311-324`). This is before budget and selection.
3. If `tick >= max_ticks`, sets `budget_exhausted` and returns `BudgetExhausted` (`:325-332`); if already exhausted, repeats (`:333-339`).
4. Selects a batch by `select_batch(bus, quota)` (`:340-341`); quota defaults to 1 (`dispatch_quota`, `:245-253`).
5. If the batch is empty, calls `poll_await_joins` and returns `Idle` or `Joined` (`:342-367`).
6. Rejects an oversized batch with `CommitError::SelectionBatchOverflow` before mutation (`:368-376`).
7. Marks each dispatched task `Running`, pushes it to `tasks.in_flight`, removes it from `tasks.ready`, and sets the quota-1 projections (`:377-404`).
8. Resolves each kind through the bus routing table and emits a proposal: no-op `Complete(Empty)`, unsupported `Fail`, registered worker proposals via the handler, unregistered `Fail` (`:406-476`).
9. Injects `empty_proposal_fail` for any dispatched task without a transition (`:477-507`; helper at `commit.rs:2817-2827`).
10. Calls `commit_proposals` once (`:508-509`); on success clears `in_flight` (`:510-517`); on failure calls `fail_selected` in dispatch order, clears `in_flight`, returns `CommitFailed` (`:519-533`).
11. `clock_tick` / `clock_tick_with` reset wires, apply pins, propagate, append one `TickRecord`, latch, advance (`:593-677`).

The selection key is `(phase_priority(kind) = group.raw(), enqueue_ordinal, TaskId)` (`routing.rs:34-38`, `task.rs:1783-1789`, `routing.rs:223-243`). It is **not** stage-ordinal-first and it does not consider `dispatch_ordinal`.

The M1 host driver `compiler/src/bin/candidate.rs` bootstraps each task kind directly (`candidate.rs:315-333`) and calls `clock_tick_with`; it does not exercise CT01/CT02. This is an integration shortcut, not a control-chip implementation, and it is why the control package must not assume a real `start_job`/source-response path exists.

### 2.4 Current commit behavior

`commit_proposals` (`compiler/src/commit.rs:530-1761`):

- Checks the proposal budget first (`:538-539`; `check_proposal_budget`, `:518-528`).
- Orders proposals by `(task.enqueue_ordinal, task.index(), proposal index)` (`:541-551`). **No dispatch ordinal is available.**
- Validates read-only, then preflights capacity, then applies. The apply pass is declared infallible (`:1440-1760`); `try_fail_task` is the explicit fallback for progress-limit failures and a missing resolved wait map (`:1660-1676`, `:1695-1697`, `:2829-2869`).
- Enforces exactly one transition per enclosed task (`completing` set; `DuplicateCompletion`; `TaskNotTransitioned` at `:1260-1270`).
- Binds inner task IDs (`bind_inner_task`, `:1763-1773`), owner/chip attribution, manifest acceptance, write declarations, schema fields, store versions, patch shape (`validate_patch`, `:1848-1972`).
- Resolves `ChildRef::Committed`/`OwnBatch` and `ContinuationRef` (`:1286-1313`, `:2121-2224`); `OwnBatch` must name this parent (`:1301-1308`).
- Simulates the await-all join against post-batch child states (`:1315-1388`): all terminal + all completed → reinsert; any failed → fail once; a batch-failed child defers to apply; non-terminal → stay waiting.
- Preflights proposal + draft budget (`:1390-1397`), capacity (`:1398-1424`), and backpressure (`:1425-1438`). The backpressure call currently uses `bus.tasks.ready.len()` and `max_queue_len` with stage `0` (`:1430-1438`), because `stage_queues` do not exist.
- Applies in order (`:1440-1760`); `Complete` pushes one result; `Fail` pushes one diagnostic; `AwaitHost` pushes one request + `Waiting`; `AppendRecords` materializes typed bodies; `Progress` updates ordinal/count and reinserts; `AwaitChildren` sets `Waiting`; `StorePatch` appends to `patch_log`; join execution reinserts/fails waiters (`:1719-1758`).
- Never consumes child results at join (`:1719-1728`), matching T02 rev 34.

### 2.5 Current join, progress, cancel, and phase behavior

| Area | Current behavior | Evidence | Gap |
|---|---|---|---|
| Join | Await-all over committed terminal states; all `Completed` → `Ready` + one reinsert at `tick+1`; any `Failed` → `Failed` once with the first failed child's diagnostic; non-terminal → stay `Waiting`; no result consumption | `commit.rs:1315-1388`, `:1719-1758`, `:2421-2528`; `c03_task.rs:1174-1285`; `c09_readiness.rs:207-304` | No `tasks.waiting` index; scans all tasks/waiters; a dangling committed child stays `Waiting` forever instead of faulting; no explicit `join_then_replay_retry` fixture |
| Progress | `Progress { task, ordinal }` must strictly advance; `count + 1 <= max_task_progress`; zero disables; reinsert at `tick+1`; exceedance routes to per-task `Fail` | `commit.rs:1198-1228`, `:1660-1676`, `:2257-2337`; `c03_task.rs:1116-1172` | Progress-failure diagnostics are not included in the capacity preflight; `Progress` may coexist with `AppendRecords`/`StorePatch` today, contrary to CT_FREEZE_02 §2; non-advancing detection is absent |
| Cancel | Pre-selection, pre-budget; sets `JobState::Failed`; clears selection/active/in-flight; leaves ready tasks `Ready`; repeated cancel repeats the outcome; one report record | `routing.rs:311-324`, `:602-633`; `c06_routing.rs:288-326` | `cancel_requested` is never cleared; ready/waiting disposition is not frozen; CT13 chip is absent; no mid-tick in-flight cancel path exists |
| Phase | `control.stage` exists but is never advanced by the shell; `wires.phase_advance_requested` is never set; no CT09 | `bus.rs:739-784`, `:902-911`; `routing.rs` (no stage writes) | No gate, no mapping from `stage_of` (0–6) to `Stage` (0–10), no `STAGE_COUNT` reconciliation |
| Recovery | `fail_selected` transitions each dispatched task in dispatch order via `try_fail_task`; diagnostic or `NONE`; state guard only `Running`; clears in-flight after | `routing.rs:565-591`; `commit.rs:2829-2869` | No CT12; H6 recovery is in the shell, not the commit path; exact latch clear owner open; no all-path proof |
| Report | One `TickRecord` per tick via `record_tick`; quota-1 projects `selected` only; `dispatched` is `selected.into_iter()` | `routing.rs:679-695`; `bus.rs:856-865`; `snapshot.rs:808-823` | Quota > 1 would report only the first task; no per-dispatch outcomes; `TickRecord` has no tick number |
| Result consumption | `consume_result` checks `consumed` and flips it; no consumer identity | `commit.rs:2091-2119` | Any caller can consume; exactly-once is boolean, not consumer-scoped |

### 2.6 What is absent

The following are **not** in `/39` and must not be assumed by any work order:

- `SelectionBatch`, `SelectionEntry`, `wires.selection`, `control.dispatch_cursor`, `DuplicateSelection` (T02/ADR proposal; absent).
- `stage_queues: [Vec<TaskId>; N]`, per-stage backpressure, `PipelineMetrics`, `fairness` cursor (proposal; absent).
- `CrossTaskWriteConflict` (proposal; absent; the current commit uses `InvalidPatchShape` for the per-task append/patch conflict).
- `TickRecord.outcomes`, `TickRecord.tick` (proposal; absent).
- `TaskState::Cancelled` (proposal option; absent).
- `ContinuationRef` in `TaskDraft.continuation`, `G1DraftBody::Continuation`, continuation materialization (type exists, path absent).
- CT01–CT14 chip files, `compiler/src/chips/control/`, `compiler/src/report.rs`, `compiler/src/driver.rs` (absent).
- A service-chip manifest with zero `task_kinds` (validator rejects `NoTaskKinds`; absent).

### 2.7 Reconciliation summary: `/39` quota-1 routing vs A16

| Aspect | `/39` quota-1 baseline | A16 proposal | Freeze action |
|---|---|---|---|
| Selection key | `(group priority, enqueue ordinal, TaskId)` | `(stage ordinal, group priority, enqueue ordinal, TaskId)` | F1; if the integrator chooses the current key, record it explicitly and drop the stage-first claim |
| Selection result | `Vec<TaskId>` from `select_batch` | `SelectionBatch`/`SelectionEntry` (wire-only) | F1/F6; quota-1 projection remains `selected` |
| Commit key | `(enqueue ordinal, TaskId, proposal index)` | `(dispatch ordinal, enqueue ordinal, TaskId, proposal index)` | F1; add dispatch ordinal to the batch envelope |
| Queue | `tasks.ready` only | `tasks.ready` canonical + optional `tasks.waiting`; `stage_queues` deferred | F3; no second writable queue |
| Stage | `Stage` label, never advanced | `stage_of` scheduling authority + CT09 gate; `Stage` derived | F3; publish the mapping |
| Task guard | `drive_task` identity checks | CT04 decision with 10 checks | F6; no mutation |
| Validation | `commit_proposals` internal | CT05 decision + commit defense-in-depth | F1/F2/F6; one validation authority |
| Commit | one ordered atomic batch | same, with dispatch ordinal and explicit preflight | F1/F3; no fallible apply |
| Join | commit-apply + idle poll | CT07 decision, same commit-apply invariant | F5; no new carrier, no consume |
| Progress | `Proposal::Progress` + commit counters | CT10 decision, same counters/budget | F4; freeze writes-with-progress |
| Cancel | shell pre-selection | CT13 decision + driver apply | F4; freeze disposition |
| Recovery | shell `fail_selected` | H6 recovery + separate CT12 language recovery | F4/F7; do not conflate |
| Report | `TickRecord { dispatched, metrics, selected }` | optional `tick`/`outcomes`; otherwise reuse | F8; snapshot impact explicit |
| Chips | none | CT01–CT14 service interfaces | F6; no synthetic task kinds |

The reconciliation preserves the `/39` invariants: no same-tick re-entry, one transition per dispatched task, exactly-once completion/consumption, await-all join, no chip mutation, deterministic ordering, and bounded recovery. The changes are additive or explicitly deferred; none silently weakens the frozen contract.

## 3. Proposed contract requirements (A16 candidate)

The following rules are the package's normative proposal. Each rule is testable and names its status. The rules deliberately reuse the `/39` envelope; they do not add a committed record family.

### 3.0 Minimal-shape decision

| Shared shape | Decision | New committed record/field? |
|---|---|---|
| `Task` | Reuse exactly; no control-specific fields | No |
| `TaskState` | Reuse the five frozen variants; no `Cancelled` for M1 | No |
| `ResultValue`/`ResultRecord` | Reuse the five variants and the `consumed` flag | No |
| `ContinuationRecord` | Reuse the frozen 9 fields; materialization deferred or separately frozen | No (defer) |
| `WaitSet` | Reuse `children + host_request` | No |
| Queue | Reuse `tasks.ready`, `tasks.active`, `tasks.in_flight`; optional `tasks.waiting` index | Optional index only |
| Ledger | Reuse `Task.state`, `ResultRecord`, `DiagnosticRecord`, `TickRecord`; optional `TickRecord.outcomes` | Optional report field only |
| Control decisions (`SelectionBatch`, `GuardDecision`, `ValidatedBatch`, `CommitDecision`, `JoinDecision`, `ProgressDecision`, `PhaseDecision`, `CancelDecision`, `RecoveryDecision`) | Wire-only decision types; no durable record | No |
| Task kinds | No new control task kinds for the loop; CT01–CT14 are driver-invoked services (recommended) | No |
| Stage/queue schema | `stage_of` scheduling authority; `Stage` derived; `stage_queues` deferred | No at quota 1 |

This table is the "smallest sufficient" answer: the control package adds decisions and observability, not a second semantic store.

### 3.1 Design rules

| ID | Rule | Status |
|---|---|---|
| A16-R01 | Control chips are **decision-only**. No chip mutates `CompilerBus`, `TaskStore`, `Control`, stores, queues, diagnostics, or artifacts. Every mutation is performed by the driver/commit path from a typed decision. | `[PROPOSED A16]` (matches T02 package envelope) |
| A16-R02 | Reuse `Task`, `TaskState`, `WaitSet`, `ContinuationRecord`, `ResultValue`, `ResultRecord`, `Proposal`, `TaskDraft`, and `StorePatch` as frozen. Do **not** add a committed record family, a `ResultValue` variant, or a `TaskState` variant for the quota-1 control loop. | `[PROPOSED A16]`; current `/39` shape |
| A16-R03 | A newly enqueued task is visible in the next tick only (`ready_tick = tick + 1`). No same-tick re-entry. | `[FROZEN /39]`; `commit.rs:1445`, `routing.rs:230` |
| A16-R04 | One transition (`Complete`/`Fail`/`AwaitHost`/`AwaitChildren`/`Progress`) per dispatched task per batch. A dispatched task with an empty or transition-free proposal vector fails with `TaskNotTransitioned`; it is never silently completed and never left `Running`. | `[FROZEN /39]` / `[IMPLEMENTED /39]`; `commit.rs:1260-1270`, `routing.rs:477-507` |
| A16-R05 | The selection order is total and deterministic. The proposed key is `(stage ordinal, group priority, enqueue ordinal, TaskId)`; the current key is `(group priority, enqueue ordinal, TaskId)`. The freeze must choose one key and publish it as a rule ID. | `[PROPOSED A16]`, `[CONFLICT]` with `/39` |
| A16-R06 | The commit order is `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal_index)`. The current order is `(enqueue_ordinal, TaskId, proposal_index)` because no dispatch ordinal exists. | `[PROPOSED A16]`, `[CONFLICT]` with `/39` |
| A16-R07 | The dispatcher's `Ready → Running` + in-flight population is a distinct pre-worker mutation, not part of the semantic commit. The semantic commit is one ordered atomic batch. | `[PROPOSED A16]`; T02 H9 direction, `[PARTIAL]` |
| A16-R08 | A commit failure commits **no semantic state**. The driver then runs a deterministic bounded recovery that transitions every dispatched task exactly once, in dispatch order, to `Failed` (committed diagnostic or `DiagnosticId::NONE`). Clearing `in_flight` is not a transition. | `[IMPLEMENTED /39]` in the shell; `[PROPOSED A16]` as the frozen mechanism; T02 H6 |
| A16-R09 | The join is a commit-apply invariant: await-all, terminal-state-only, no child-result consumption, no new CT07 carrier. All `Completed` → `Ready` + one reinsert; any `Failed` → `Failed` once; non-terminal → `Waiting`. | `[FROZEN /39]` / `[IMPLEMENTED /39]`; T02 rev 30/34 |
| A16-R10 | Cancel is evaluated once per tick, before budget and selection. A cancel tick performs no new selection, dispatch, or commit; it is idempotent; it leaves ready tasks `Ready` (current) or applies the frozen disposition; it produces exactly one terminal report record and `tasks.in_flight = ∅` at latch. | `[IMPLEMENTED /39]` / `[PROPOSED A16]` disposition |
| A16-R11 | `Progress` is the only reschedule mechanism. The ordinal strictly advances; the count is bounded by `max_task_progress`; zero disables; exceedance fails the task once. Whether a `Progress` vector may also carry atomic `AppendRecords`/`StorePatch` is a freeze decision (A16-P04 recommends allow). | `[IMPLEMENTED /39]`; `[CONFLICT]` with CT_FREEZE_02 on writes |
| A16-R12 | The phase gate advances only when no `Ready` task exists in any canonical queue and no `Waiting` task remains. It never performs compilation and never sees wires/drafts/in-flight as authority. | `[PROPOSED A16]`; T02 CT09 |
| A16-R13 | Recovery selection (CT12) is decision-only: it preserves the original diagnostic, emits at most one recovery `Enqueue` or a `Fail`, is bounded, and never re-enters the failed task in the same tick. | `[PROPOSED A16]`; T02 CT12 |
| A16-R14 | Every control decision is replay-visible: identical initial state/config/pins yields identical decisions, dispatch order, commit order, join outcomes, phase outcomes, cancel outcomes, and report. No hash-map iteration or address dependence. | `[FROZEN /39]` determinism; `[PROPOSED A16]` control |
| A16-R15 | Single writer per field/queue: the driver owns selection/in-flight/report; the commit path owns task states/stores/results/diagnostics/requests; worker adapters own only their own proposal slots. No chip writes a shared field. | `[PROPOSED A16]`; Guardrails §2/§6 |
| A16-R16 | Every failure is typed: `CommitError` for reject-before-apply commit invariants; `ManifestError` for registration; chip `DiagnosticDraft` for language/task faults; dispatcher/scheduling errors for pre-dispatch failures (carrier decision open). No panic control flow. | `[PROPOSED A16]`; M1 proposal §6.4 |

### 3.2 Task and task-state requirements

**A16-T01 — Task envelope.** Keep `Task` exactly as frozen. The control package may not add a `phase`, `stage`, `dispatch_ordinal`, `waiting_on`, or `owner_group` field to `Task`; those are derived from `kind`/`state` or held in wire/ledger state. `[PROPOSED A16]`

**A16-T02 — State invariant.** A task is in exactly one of `Ready`, `Running`, `Waiting`, `Completed`, `Failed`. `Running` is only legal while it is in `tasks.in_flight` during the current tick. A task not in `in_flight` must not be `Running` at latch. `[FROZEN /39]` + `[PROPOSED A16]` invariant.

**A16-T03 — Transition matrix.** The only legal transitions are:

| From | Event | To | Guard |
|---|---|---|---|
| `Ready` | dispatcher dispatch | `Running` | `ready_tick <= tick`, not cancelled/budget-exhausted, node in `tasks.ready` |
| `Running` | `Complete` | `Completed(result)` | task kind matches result; one transition |
| `Running` | `Fail` | `Failed(diag)` | one transition; diagnostic committed or `NONE` (recovery only) |
| `Running` | `AwaitHost` | `Waiting(host_request)` | one transition; one request |
| `Running` | `AwaitChildren` | `Waiting(children)` | one transition; non-empty; committed children |
| `Running` | `Progress` | `Ready` (`ready_tick = tick+1`) | ordinal advances; count bound |
| `Waiting(children)` | join all `Completed` | `Ready` (`ready_tick = tick+1`) | all committed children terminal |
| `Waiting(children)` | join any `Failed` | `Failed(child_diag)` | all committed children terminal; first failed |
| `Waiting(host)` | host response / CT02 | `Ready` (`ready_tick = tick+1`) | request satisfied once; proposed |
| `Running` | commit failure / H6 | `Failed(diag or NONE)` | exactly once; dispatch order |
| any | cancel | unchanged (current) | job terminal; no dispatch |

There is no `Completed → *`, `Failed → *`, `Ready → Completed` (without dispatch), or `Waiting → Running` (without a `Ready` reinsert) transition. `[PROPOSED A16]`

**A16-T04 — `ready_tick`.** A task is selectable only when `state == Ready` and `ready_tick <= control.tick`. Enqueue/reinsert uses `tick + 1`. Bootstrap may use `tick` (integration-only). `[FROZEN /39]`

### 3.3 Result and completion-ledger requirements

**A16-L01 — Result identity.** A `Complete` produces exactly one `ResultRecord` with `task` = the completed task, `kind` = its `TaskKind`, `value` = the proposal value, `version` = the store/protocol version at commit, `consumed = false`. `[FROZEN /39]`

**A16-L02 — Consumption.** A result is consumed at most once. The consumer must be the parent/owning task's next atomic commit, not the join. The current `consumed: bool` is exactly-once but not consumer-scoped; for M1 the protocol must require the parent as the only consumer, and a future `consumer: Option<TaskId>` field may be added only by a freeze. `[FROZEN /39]` boolean; `[PROPOSED A16]` consumer contract.

**A16-L03 — No result for non-`Complete`.** `Fail`, `AwaitHost`, `AwaitChildren`, and `Progress` produce no `ResultRecord`. A `Diagnostic` result variant is a diagnostic reference, not a result value for a failed task. `[FROZEN /39]`

**A16-L04 — Completion ledger.** The authoritative completion facts are `Task.state` plus the committed `ResultRecord`/`DiagnosticRecord`. `CommitReport` is a transient report, not a second ledger. `TickRecord` is the per-tick dispatch ledger; if per-dispatch outcomes are required for audit, add `outcomes` to `TickRecord` (freeze item F3), not a new record family. `[PROPOSED A16]`

**A16-L05 — No silent completion.** A dispatched task never becomes `Completed` without an explicit `Complete` proposal. The empty-vector rule is the explicit `Fail` in A16-R04. `[FROZEN /39]`

### 3.4 Continuation and WaitSet requirements

**A16-C01 — WaitSet is the sole awaited-child source.** `TaskState::Waiting(WaitSet)` is the only place a task's awaited children live. `ContinuationRecord` has no `awaited` field. `[FROZEN /39]`

**A16-C02 — Committed IDs only in durable state.** Every `WaitSet.children` entry and every `Task.continuation` is a committed ID. `ChildRef::OwnBatch`/`ContinuationRef::OwnBatch` are transient wire/proposal inputs only; commit resolves them before persistent state. `[FROZEN /39]` for `ChildRef`; `[PARTIAL]` for `ContinuationRef`.

**A16-C03 — Continuation materialization.** The frozen `ContinuationRecord` currently has **no append path**: `G1DraftBody` has no `Continuation` variant, and `StoreSchema` declares `tasks.active.continuation` as a task field but **no continuation-record append field**. For the quota-1 control loop, keep `Task.continuation = None` and do not use `ContinuationRef`; if a parser slice needs continuations, freeze an explicit `G1DraftBody::Continuation(ContinuationRecord)` plus a declared append field, allowlist row, snapshot body, and `TaskDraft.continuation: Option<ContinuationRef>` migration. Do not invent a private continuation map. `[PROPOSED A16]`; `[ABSENT]` path.

**A16-C04 — Continuation field inventory.** Keep the frozen 9 fields (`production`, `cursor`, `context`, `binding_power`, `scope`, `parent`, `partial_children`, `next_child_ordinal`, `previous`). Do not replace them with the generic 9-field list in CT_FREEZE_01 OPEN-09 (`operands`, `frame`, `progress_count`, `stage_hint`, `resume_kind`); that list is incompatible with `/39`. `[CONFLICT]` with CT_FREEZE_01; recommendation: keep `/39`.

### 3.5 Queue and dispatch-batch requirements

**A16-Q01 — Canonical ready queue at quota 1.** `tasks.ready` is the single canonical ready set for the quota-1 baseline. Its vector order is insertion order and is **not** semantic: `select_batch` collects eligible IDs and sorts them by the dispatch key (`routing.rs:223-243`). The dispatcher removes dispatched tasks; commit reinserts `Progress`/join tasks. `[IMPLEMENTED /39]`; `[PROPOSED A16]` freeze as canonical at quota 1.

**A16-Q02 — `stage_queues` are deferred.** Per-stage `stage_queues` are a quota > 1 extension, not a quota-1 requirement. If the integrator wants them for the pipeline freeze, they replace `tasks.ready` as canonical and `tasks.ready` becomes a derived read-only projection; that is a larger schema change (task store, snapshot, limits, backpressure) and must be a separate freeze item F3. Do not add both as writable queues. `[PROPOSED A16]`; `[CONFLICT]` with T02/ADR direction.

**A16-Q03 — Waiting index.** Add `tasks.waiting: Vec<TaskId>` (ascending `TaskId`) maintained by the commit apply/join path, or document the current full scan as a bounded small-profile exception. The index makes the join and phase gate bounded and discoverable; it is a single-writer field. `[PROPOSED A16]`; current code scans (`commit.rs:2421-2528`).

**A16-Q04 — In-flight set.** `tasks.in_flight` is an ephemeral per-tick `Vec<TaskId>` in dispatch order. It is populated by the dispatcher before the first worker and cleared after every dispatched task has a terminal/`Waiting`/`Progress` outcome or after H6 recovery. It is not a transition and does not clear `TaskState::Running`. `Waiting` tasks are never in-flight. `[FROZEN /39]` / `[IMPLEMENTED /39]`; exact clear owner open.

**A16-Q05 — Dispatch batch.** A `SelectionBatch`/`SelectionEntry` shape is **proposed** for CT03: `SelectionEntry { task: TaskId, chip: ChipId, layer: u16, dispatch_ordinal: u32 }`, `SelectionBatch { entries: Vec<SelectionEntry> }`. At quota 1 it has at most one entry; at quota Q it has at most Q. It is wire-only, bounded, and ordered. `SelectionBatchOverflow` and `DuplicateSelection` are pre-dispatch scheduling failures; the current `SelectionBatchOverflow` is carried on `CommitError` (`commit.rs:244-258`) and must be re-homed or explicitly accepted. `[PROPOSED A16]`; `[CONFLICT]` with `/39` carrier.

**A16-Q06 — Dispatch order.** CT03 selects at most one entry per eligible stage, ordered by `(stage ordinal, group priority, enqueue ordinal, TaskId)`; the dispatcher applies the batch in that order. At quota 1 the projection is `tasks.active`/`control.selected`/`wires.selected`. At quota Q the canonical report is the ordered `dispatched` list; `selected` is `None` or the first entry only if a consumer explicitly opts into the projection. `[PROPOSED A16]`.

**A16-Q07 — No duplicate dispatch.** A task may appear at most once in a `SelectionBatch`; a duplicate is `DuplicateSelection` before any `Ready → Running` mutation. `[PROPOSED A16]`; `[ABSENT]` error/type.

### 3.6 Control ledger and report requirements

**A16-L10 — One report append per tick.** The driver's step-6 append site is the only writer of `bus.report`. Exactly one `TickRecord` per tick, bounded by `max_ticks + 1`. `[IMPLEMENTED /39]` (`routing.rs:679-695`; `bus.rs:992-1005`).

**A16-L11 — Report content.** `TickRecord.dispatched` is the ordered dispatch batch. `TickRecord.selected` is the quota-1 projection. `TickRecord.metrics.dispatched` is the count. If per-dispatch outcomes are needed (for exactly-once audit and quota > 1), extend `TickRecord` with `tick: u64` and `outcomes: Vec<DispatchOutcome>` where `DispatchOutcome { task: TaskId, outcome: TaskOutcomeKind }` and `TaskOutcomeKind` is one of `Completed(ResultId)`, `Failed(DiagnosticId)`, `Waiting`, `ReadyProgress`, `ReadyJoin`, `Recovered(Option<DiagnosticId>)`. This is a snapshot/contract freeze item (F3), not a current requirement. `[PROPOSED A16]`.

**A16-L12 — Report determinism.** The report order is dispatch order, then `TaskId` for joins, then ascending `DiagnosticId` for diagnostics. No hash-map iteration. `[PROPOSED A16]`.

### 3.7 Task-guard requirements (CT04)

**A16-G01 — Guard is decision-only.** `GuardDecision { task, chip, stage, layer, verdict }` with `GuardVerdict = Admit | Wait { reason } | Reject { code }`. The guard reads the task, its route, the manifest, the task state, the stage assignment, and the store-version snapshot; it does not mutate. `[PROPOSED A16]`

**A16-G02 — Required checks.** The guard must check, in this order:
1. task exists and is live (`UnknownTask`);
2. task `state == Ready` and `ready_tick <= tick` (`WrongState`);
3. task owner equals the routed chip (`WrongOwner`);
4. route exists and the chip registry has the manifest (`UnregisteredChip`);
5. the chip accepts the task kind (`KindNotAccepted`);
6. `stage_of(kind)` exists and equals the routed layer (`StageUnassigned`/`StageLayerMismatch`);
7. the task is not already in `in_flight` (`DuplicateDispatch`);
8. the in-flight quota is not exceeded (`InFlightQuota`);
9. job state is `Running`, cancel is not requested, budget is not exhausted (`JobNotRunning`/`CancelPending`/`BudgetExhausted`);
10. for patches, the worker's `StorePatch.version` equals the current `store_versions` at commit (`StaleVersion`; commit re-checks).
`[PROPOSED A16]`; current `drive_task` checks 1, 3–6 and the handler checks 1, 3–6 (`chips/mod.rs:309-419`); it does not check 7–9 or a version snapshot.

**A16-G03 — Wrong task.** If the guard rejects, the task does not enter the worker and the driver leaves it `Ready` for a pre-dispatch failure (no transition) or fails it explicitly if the guard is evaluated after dispatch. The choice must be published; the current shell has no CT04. `[PROPOSED A16]`

### 3.8 Proposal-validation requirements (CT05)

**A16-V01 — Validation is decision-only.** `ValidatedBatch { entries: Vec<ValidatedEntry> }` with `ValidatedEntry { task, chip, dispatch_ordinal, transition: Option<TransitionKind>, proposal_indices: Vec<u32>, write_claims: Vec<WriteClaim> }`. CT05 returns the decision; the commit still re-validates for direct callers. `[PROPOSED A16]`

**A16-V02 — One validation authority.** Factor the read-only validation pass into a pure `validate_batch(bus, proposals) -> Result<ValidatedBatch, CommitError>` helper used by both CT05 and `commit_proposals`. Do not let the chip and the commit diverge into two validators. The commit remains the only mutator. `[PROPOSED A16]`

**A16-V03 — Required checks.** The validation pass must check:
1. proposal budget: `proposals.len() + total_drafts <= max_proposals_per_tick` (lossless `usize`);
2. one transition per enclosed task; empty/transition-free vector is `TaskNotTransitioned`;
3. inner task binding (`InnerTaskMismatch`);
4. chip registered, owner, kind accepted;
5. result/transition tag matches the task kind (the current commit only sets `ResultRecord.kind = task.kind`; a kind-specific result registry is absent);
6. store field declared, owner allowlist, config read-only, patch shape, stale version;
7. append family/bodies 1:1, M1-subset gate, predicted IDs, at most one `AppendRecords` per task;
8. append vs patch same `(store, field)` conflict;
9. `ChildRef`/`ContinuationRef` range, ownership, and committed/live checks;
10. progress ordinal advance and count bound;
11. cross-task write conflict `(StoreId, field, record)` in canonical order; patches to the same record always conflict; same-owner append-only fields only if a hashed rule permits;
12. duplicate `Enqueue` destination/kind authorization.
`[PROPOSED A16]`; current implementation covers 1–4, 6–11 in `commit_proposals`; 5 is partial; 12 is covered by `validate_append_authorization`/`validate_patch`.

**A16-V04 — Error carrier.** Validation failures are `CommitError` (reject-before-apply) when they are commit invariants. Pre-dispatch selection failures are dispatcher/scheduling failures, not `CommitError`; the current `SelectionBatchOverflow` carrier is the known `/6` open item (T01 §7.1: "carried on `CommitError` although proposal §6.2.1 classifies dispatch-count failures as dispatcher failures"; C17-9). `[CONFLICT]`; freeze item F1.

**A16-V05 — No mutation during validation.** The validation pass must not mutate `TaskState`, queues, stores, `control`, `patch_log`, or store versions. Any cache used for validation must be local and deterministic. `[PROPOSED A16]`; current validation is read-only.

### 3.9 Atomic-commit requirements (CT06)

**A16-K01 — One ordered atomic commit.** All proposals for one tick commit in one ordered batch. The commit path performs: validate (read-only) → capacity preflight (no mutation) → infallible apply. A failure before apply commits no semantic state. `[FROZEN /39]` / `[IMPLEMENTED /39]`

**A16-K02 — Commit order.** `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal_index)`. `dispatch_ordinal` is the 0-based position in the ordered `SelectionBatch`; `enqueue_ordinal` is the task's global enqueue ordinal; `TaskId` breaks ties; `proposal_index` is the emission index within the task. The current order omits `dispatch_ordinal` (`commit.rs:541-551`). `[PROPOSED A16]`; `[CONFLICT]` with `/39`.

**A16-K03 — Capacity preflight.** Before any apply, check the total-record budget, per-arena capacity, task total, diagnostic total, proposal + draft budget, and queue/backpressure. All arithmetic is checked/saturating. A preflight failure is a whole-batch reject; the H6 recovery then runs. `[PROPOSED A16]`; current `check_capacity`/`check_backpressure_capacity` implement most of it.

**A16-K04 — No fallible step after the first mutation.** The apply pass must be infallible. The current exceptions to audit are: `Progress` limit failures call `try_fail_task` inside apply (`commit.rs:1667-1676`) and can allocate a diagnostic after preflight; `intern` can fail if the preflight prediction is wrong (`commit.rs:1643-1655`, `debug_assert`). Freeze must either include these allocations in the preflight or use the `NONE` sentinel explicitly. `[PROPOSED A16]`; `[PARTIAL]` current.

**A16-K05 — Apply order.** Apply in the same order as validation/commit; enqueues create predicted IDs in that order; patches update `patch_log` and `store_versions`; results/diagnostics/host requests append once; `AppendRecords` materializes 1:1; `Progress` reinserts once; `AwaitChildren` sets `Waiting`; join execution runs after the proposal apply. `[FROZEN /39]` / `[IMPLEMENTED /39]`

**A16-K06 — Commit failure recovery.** On any commit failure, the driver runs H6 recovery (A16-R08). The recovery is not part of the failed atomic commit; it is a separate deterministic mutation. It must leave no dispatched task `Running` and no `in_flight` entry at latch. `[PROPOSED A16]`; current `fail_selected` (`routing.rs:565-591`).

**A16-K07 — Commit report.** `CommitReport` is returned on success; on failure no `CommitReport` is returned and no semantic state is applied. The driver may append a failure report/diagnostic through the explicit recovery path, not by fabricating a successful report. `[PROPOSED A16]`

### 3.10 Join requirements (CT07)

**A16-J01 — Await-all.** A parent resumes only when all committed children are terminal. All `Completed` → `Ready` + one reinsert at `tick+1`; any `Failed` → `Failed` exactly once, reusing the first failed child's `DiagnosticId` in `WaitSet.children` order; a non-terminal child → stay `Waiting`. `[FROZEN /39]` / `[IMPLEMENTED /39]`

**A16-J02 — No consumption at join.** The join reads child state only. Child results remain committed and unconsumed; the parent consumes them in its own next atomic commit. `[FROZEN /39]` direction (T02 rev 34); `[IMPLEMENTED /39]`

**A16-J03 — No new carrier.** The join is a commit-apply invariant. Do not add a CT07 committed join-decision record/family. If CT07 is implemented as a chip, its output is wire-only and commit-apply consumes it. `[PROPOSED A16]`; T02 rev 30.

**A16-J04 — Dangling child.** A committed `WaitSet` child that does not exist or is not a child of the parent is a **protocol fault**; fail the parent once with a typed diagnostic, never park it `Waiting` forever. The current `poll_await_joins` treats a missing child as non-terminal and leaves the parent `Waiting` (`commit.rs:2475-2490`); that is a silent-spin defect. `[PROPOSED A16]`; `[PARTIAL]` current.

**A16-J05 — Bounded join.** Use `tasks.waiting` (A16-Q03) or a documented bound. The join scan is O(waiters × children) per poll and the commit simulation scans all waiters per batch; the pcr13 small-profile exception (`c09_readiness.rs:616-639`) is not a general bound. Freeze an explicit bound or index. `[PROPOSED A16]`

**A16-J06 — Replay retry.** Add `join_then_replay_retry`: replay between the join and the parent commit must neither double-consume nor lose child results. `[PROPOSED A16]`; required by T02 rev 34/T13 H6-M15.

### 3.11 Failure-closure and recovery requirements (H6 / CT12)

**A16-F01 — H6 scope.** H6 is the scheduler-level batch-failure recovery: after a failed atomic commit, every dispatched task transitions exactly once, in dispatch order, to `Failed`. It is distinct from language-level CT12 recovery (parse synchronization/termination). `[PROPOSED A16]`

**A16-F02 — Diagnostic attempt.** With no pre-reservation, each task's diagnostic attempt checks the remaining diagnostic/record capacity at that attempt. A committed `DiagnosticId` when capacity allows; otherwise `TaskState::Failed(DiagnosticId::NONE)`. The sentinel is not a committed record. `[PROPOSED A16]`; current `try_fail_task` (`commit.rs:2829-2869`).

**A16-F03 — State guard.** Only a `Running` task may transition; a second attempt is a no-op/return `None`. The count of `Failed` transitions per dispatched task is exactly one. `[IMPLEMENTED /39]`; `try_fail_task` checks `Running`.

**A16-F04 — No semantic state on failure.** The pre-recovery semantic store/record hash must equal the pre-commit hash. The recovery transitions and any recovery diagnostics are the only post-failure mutations. `[PROPOSED A16]`; T13 H6-M07.

**A16-F05 — In-flight clear.** The clear is not a transition and does not clear `TaskState::Running` by itself. It happens only after every dispatched task has a terminal/`Waiting`/`Progress` outcome or after H6 recovery, and leaves no residual entry at latch. The exact owner (commit path vs driver latch) is a T01 decision; the current shell clears at the end of `propagate` (`routing.rs:512`, `:521`), before `latch`. `[CONFLICT]` / `[PARTIAL]`; freeze item F4.

**A16-F06 — CT12 recovery.** `RecoveryDecision { failed: TaskId, diagnostic: DiagnosticId, strategy: RecoveryStrategy }`; `RecoveryStrategy = TerminateTask | Enqueue(RecoveryTaskDraft) | Resume { cursor: TokenId }`. The original diagnostic is retained; the recovery task has its own diagnostic if it fails; the recovery is bounded by `max_task_progress`/`max_ticks` and must not re-enter the failed task in the same tick. `[PROPOSED A16]`

### 3.12 Cancel requirements (CT13)

**A16-X01 — Precedence.** Cancel is evaluated once per tick before budget accounting, selection, and dispatch. A cancel tick performs no new selection, dispatch, or commit. `[IMPLEMENTED /39]`; `routing.rs:311-324`; `c06_routing.rs:288-309`.

**A16-X02 — Idempotence.** Repeated cancel ticks repeat `Cancelled` without further state change and append exactly one terminal report record per tick, not one per repeated request. `[IMPLEMENTED /39]`; `c06_routing.rs:305-308`, `:311-326`.

**A16-X03 — Disposition.** Current: `JobState::Failed`; `control.selected = None`; `tasks.active = None`; `tasks.in_flight.clear()`; ready tasks stay `Ready`; waiting tasks stay `Waiting`. Freeze this disposition or add an explicit `TaskState::Cancelled` and a waning rule; do not leave it implicit. `[PROPOSED A16]`; `c06_routing.rs:174-187` pins the ready-stays-`Ready` behavior.

**A16-X04 — Host waits.** A cancel tick winds down host waits by leaving `HostRequestRecord.satisfied = false` and not resurrecting a waiter on a late response. The current shell never processes host responses, so this is `[ABSENT]`; CT02/CT08 must define it. `[PROPOSED A16]`

**A16-X05 — No artifact.** Cancel never finalizes artifacts and never reports success. `[PROPOSED A16]`; CT14 missing-fragment rule.

### 3.13 Progress-budget requirements (CT10)

**A16-P01 — Only reschedule.** `Progress { task, ordinal }` is the only non-terminal reschedule. It reinserts the task into its own canonical queue at `tick+1`, increments `progress_count`, and persists `progress_ordinal`. `[FROZEN /39]` / `[IMPLEMENTED /39]`

**A16-P02 — Ordinal.** `ordinal` must be strictly greater than the stored `progress_ordinal`; a non-advancing ordinal is `NonAdvancingProgress` and fails the task once (current) or rejects the batch (alternative). `[IMPLEMENTED /39]` per-task fail; freeze the choice. `[CONFLICT]` (CT_FREEZE_02 says pre-apply/CT05, current applies per task).

**A16-P03 — Count.** `progress_count + 1 <= max_task_progress`; `max_task_progress == 0` disables `Progress`; exceedance is `ProgressLimit` and fails the task once. `[FROZEN /39]` / `[IMPLEMENTED /39]`

**A16-P04 — Writes with Progress.** The frozen commit allows `Progress` to coexist with `AppendRecords`/`StorePatch` from the same task (the coexistence rules forbid `Progress` with another transition, but not with writes; `task.rs:1869-1890`). CT_FREEZE_02 §2 proposes the opposite ("A Progress proposal must not carry a StorePatch/AppendRecords payload in the same task's proposal vector"). This is a direct conflict. Recommendation: **allow atomic writes with `Progress`** for M1 (a parser may append a node and advance its cursor in one step), require the ordinal to advance, and let the `max_task_progress` bound stop pathological loops; if the integrator prefers the prohibition, it must be frozen as a new rule and all progress fixtures updated. `[CONFLICT]`; freeze item F2.

**A16-P05 — Count across waits.** `AwaitHost`/`AwaitChildren` do not reset `progress_count`; a resumed task keeps its prior count. This matches the current code (no reset on wait) but is not explicitly frozen. `[PROPOSED A16]`; CT_FREEZE_02 OPEN-02-05.

**A16-P06 — No wall clock.** No `Instant`, `SystemTime`, thread, or environment read participates in progress or scheduling. `[FROZEN /39]` (chips have no I/O; Guardrails §2.8).

### 3.14 Phase/stage-gate requirements (CT09)

**A16-H01 — Gate predicate.** Advance the phase/stage only when:
- no `Ready` task exists in any canonical queue;
- no `Waiting` task remains (host or children);
- no `in_flight` task exists;
- no dispatched task is `Running` at latch.
A `Waiting` task is not "done"; it blocks the gate until it resumes or the job is cancelled/budget-exhausted. `[PROPOSED A16]`; resolves the "empty/waiting" ambiguity in T02 CT09.

**A16-H02 — No compilation.** CT09 reads completion facts/queues only; it never parses, type-checks, lowers, or optimizes. `[PROPOSED A16]`; T02 CT09.

**A16-H03 — Authority.** The canonical queues/task states are the authority; `wires.phase_advance_requested`/`Done` is advisory and must not bind the gate. `[PROPOSED A16]`; CT_FREEZE_01 OPEN-16 recommendation (a).

**A16-H04 — Stage vocabulary.** `STAGE_ASSIGNMENT` ordinals are 0–6 (`manifest.rs:1444-1514`), `Stage` has 11 variants (`bus.rs:739-784`), and `STAGE_COUNT` is 11 (`limits.rs:13-21`). The freeze must publish one mapping. Recommendation: `stage_of(kind)` is the **scheduling** authority; `control.stage` is a derived/observable job phase; define a fixed `stage_of → Stage` mapping (and leave unused `Stage` variants inert) rather than inventing per-task `Stage` fields. If the integrator prefers a 7-member `Stage`, that is a larger schema change and must be a separate freeze item. `[CONFLICT]`; freeze item F3.

**A16-H05 — Terminal state.** `Stage::Done` only when every task is terminal (`Completed`/`Failed`) and no ready/waiting/in-flight task remains. `JobState::Finished` when all terminal and none failed; `JobState::Failed` if any failed or the job was cancelled. The current shell never sets `Finished` or `Stage::Done`; CT09/CT14 must. `[PROPOSED A16]`; `[ABSENT]`.

**A16-H06 — No regression.** A stage that advances must not regress to an earlier stage without a new enqueue/contract event. A replay of the same latch state yields the same gate decision. `[PROPOSED A16]`

### 3.15 CT01/CT02/CT08/CT11/CT14 requirements

| Chip | Requirement | Status |
|---|---|---|
| CT01 JobStart | Validate the frozen config; `Idle → Running` exactly once; consume `start_job`; produce/bootstrap the first source task (`PREPROCESS_NORMALIZE` or the frozen first task kind) with the committed `Source` ref; duplicate start is a typed diagnostic, never a second job. `bootstrap_task` is integration-only (`bus.rs:1122-1139`). | `[ABSENT]` handler; `[PROPOSED A16]` |
| CT02 SourceResponse | Validate by `HostRequestId`; import immutable bytes/hash/span root as `sources` append proposals applied by commit; out-of-order/duplicate responses are typed; identical content replays identically; no direct store write. Current `CompilerPins.host_responses` is unused. | `[ABSENT]`; `[PROPOSED A16]` |
| CT08 HostRequest | Form a persistent `HostRequestDraft` proposal; commit persists it and sets `Waiting(host_request)`; exactly one request per wait; no I/O. Current `AwaitHost` commits the request; no CT08 chip. | `[PARTIAL /39]` via `Proposal::AwaitHost`; `[PROPOSED A16]` chip |
| CT11 DiagnosticCommit | Propose severity/span/code/notes in stable ordinal order, deduplicate, and let commit append `diagnostics`; no direct store write. Current `Fail` commits one diagnostic per failure; no CT11 dedupe. | `[PARTIAL /39]`; `[PROPOSED A16]` |
| CT12 RecoverySelect | A16-F06. | `[ABSENT]`; `[PROPOSED A16]` |
| CT14 ArtifactFinalize | Validate that no tasks are incomplete; read `artifacts.fragments`; emit a Host write request proposal; perform no I/O; missing fragments/failed tasks must not report success; never append `artifacts.fragments`. | `[ABSENT]`; `[PROPOSED A16]` |

### 3.16 Proposed control task/result table

Because the frozen `ResultValue` has exactly five variants and no decision variant, the control loop has two options. This table records the **recommended service form** and the alternative task-worker form.

| Chip | Recommended form | Input (fields) | Output / decision (fields) | Persistence | Consumer | Status |
|---|---|---|---|---|---|---|
| CT01 JobStart | service/bootstrap | `StartJob { config_hash, source: Option<RecordRef::Source> }` | `JobReady { first: TaskDraft }` + `Enqueue` | `control.job_state`, `sources` | dispatcher | `[ABSENT]` |
| CT02 SourceResponse | service | `HostResponse { request: HostRequestId, bytes: Vec<u8> }` | `SourceImported { source: SourceId }` | `sources` (via commit) | commit | `[ABSENT]` |
| CT03 TaskSelect | service | `ReadyQueues { ready: &[TaskId], tick, quota }` | `SelectionBatch { entries }` | wire only | dispatcher | `[ABSENT]` |
| CT04 TaskGuard | service | `SelectedTask { task, route, versions }` | `GuardDecision` | wire only | dispatcher | `[ABSENT]` |
| CT05 ProposalValidate | service | `Proposals { tagged: &[TaggedProposal] }` | `ValidatedBatch` | wire only | commit | `[ABSENT]` |
| CT06 TaskCommit | service | `ValidatedBatch` | `CommitDecision` | wire only | commit path | `[ABSENT]` |
| CT07 ResultResume | service | `ChildResults { parent, committed children }` | `JoinDecision` | wire only; commit applies | commit apply | `[ABSENT]` |
| CT08 HostRequest | service | `AwaitSource`/`Artifact` | `HostRequestDraft` | `host_requests` | Host | `[ABSENT]` |
| CT09 PhaseAdvance | service | `PhaseFacts { queues, states, tick }` | `PhaseDecision` | `control.stage` | driver | `[ABSENT]` |
| CT10 ProgressBudget | service | `TickFacts { task, progress_count, ordinal, limits }` | `Progress`/`Fail` | task counters | commit | `[ABSENT]` |
| CT11 DiagnosticCommit | service | `DiagnosticProposals` | `DiagnosticDecision { drafts }` | `diagnostics` | commit | `[ABSENT]` |
| CT12 RecoverySelect | service | `RecoverableFault { task, diagnostic, phase }` | `RecoveryDecision` | wire only | driver/commit | `[ABSENT]` |
| CT13 CancelJob | service | `CancelPin { cancel_requested }` | `CancelDecision` | `control.cancel_requested`/`job_state` | driver | `[ABSENT]` |
| CT14 ArtifactFinalize | service | `CompletedStages { tasks, fragments }` | `HostRequestDraft` | `host_requests` | Host | `[ABSENT]` |

**Task-worker alternative.** If the integrator requires every control chip to own a `TaskKind` (the current `ChipManifest` rejects `NoTaskKinds`; variant at `manifest.rs:452-456`, check at `:679-683`), then each service needs a new control kind at local `16+`, a `control_slice()` registry, a stage row, a layer, a manifest, an allowlist entry (if it writes), and tests. That is **fourteen new task kinds** and a manifest/registry freeze; it is not the smallest design and it risks the CT03-selects-itself recursion. Recommendation: extend the manifest with a decision/service capability that permits an empty `task_kinds` list, or invoke control services as driver-owned pure functions outside the `ManifestRegistry`. Do not invent synthetic task kinds to satisfy a validator. `[PROPOSED A16]`; `[CONFLICT]` with current `ChipManifest` validation.

### 3.17 Proposed wire-only decision shapes (field level)

These are **proposal-only** Rust-like shapes for the control decisions. They are not committed records, not store fields, and not in `/39`. A decision is consumed by the driver/commit path in the same tick; it is not durable. If the integrator prefers chip-local types, these are the minimum fields that must survive the projection.

| Shape | Field | Type | Consumer | Notes |
|---|---|---|---|---|
| `SelectionEntry` | `task` | `TaskId` | dispatcher | committed task |
| | `chip` | `ChipId` | dispatcher | routed chip |
| | `layer` | `u16` | dispatcher | must equal `stage_of(kind)` |
| | `dispatch_ordinal` | `u32` | dispatcher/commit | 0-based batch position |
| `SelectionBatch` | `entries` | `Vec<SelectionEntry>` | dispatcher | `len <= max_inflight_per_tick`; no duplicate task |
| `GuardDecision` | `task` | `TaskId` | dispatcher | |
| | `chip` | `ChipId` | dispatcher | |
| | `stage` | `u8` | dispatcher | `stage_of(kind)` |
| | `layer` | `u16` | dispatcher | routed layer |
| | `verdict` | `GuardVerdict` | dispatcher | `Admit`/`Wait`/`Reject` |
| `GuardVerdict` | `Admit` / `Wait { reason }` / `Reject { code }` | enum | dispatcher | reason/code typed |
| `ValidatedEntry` | `task` | `TaskId` | CT06/commit | |
| | `chip` | `ChipId` | CT06/commit | |
| | `dispatch_ordinal` | `u32` | CT06/commit | |
| | `transition` | `Option<TransitionKind>` | CT06/commit | exactly one for a dispatched task |
| | `proposal_indices` | `Vec<u32>` | CT06/commit | emission indices |
| | `write_claims` | `Vec<WriteClaim>` | CT06/commit | conflict predicate input |
| `ValidatedBatch` | `entries` | `Vec<ValidatedEntry>` | CT06/commit | consistent with the dispatch batch |
| `WriteClaim` | `store` | `StoreId` | commit | |
| | `field` | `&'static str` | commit | declared field |
| | `record` | `Option<RecordRef>` | commit | append/patch target |
| `CommitDecision` | `ordered` | `Vec<OrderedProposal>` | commit path | canonical order |
| | `capacity` | `CapacityPlan` | commit path | no-mutation preflight |
| | `recovery` | `RecoveryPolicy` | driver | H6 policy |
| `OrderedProposal` | `dispatch_ordinal` | `u32` | commit | |
| | `enqueue_ordinal` | `u64` | commit | |
| | `task` | `TaskId` | commit | |
| | `proposal_index` | `u32` | commit | |
| `JoinDecision` | `parent` | `TaskId` | commit apply | |
| | `verdict` | `JoinVerdict` | commit apply | `StayWaiting`/`Ready`/`Fail` |
| `JoinVerdict` | `StayWaiting` / `Ready` / `Fail { diagnostic: DiagnosticId }` | enum | commit apply | no consume |
| `ProgressDecision` | `task` | `TaskId` | commit | |
| | `verdict` | `ProgressVerdict` | commit | `Reinsert { ordinal, ready_tick }` / `Fail { code }` |
| `PhaseDecision` | `from` | `Stage` | driver | current stage |
| | `verdict` | `PhaseVerdict` | driver | `Hold` / `Advance { to: Stage }` |
| | `blocking` | `Vec<TaskId>` | driver | committed ready/waiting tasks (diagnostic) |
| `CancelDecision` | `verdict` | `CancelVerdict` | driver | `CancelJob`/`NoCancel` |
| | `affected_in_flight` | `Vec<TaskId>` | driver | empty in the pre-selection path |
| | `affected_ready` | `Vec<TaskId>` | driver | diagnostic/audit |
| `RecoveryDecision` | `failed` | `TaskId` | driver/commit | original task |
| | `diagnostic` | `DiagnosticId` | driver/commit | original diagnostic retained |
| | `strategy` | `RecoveryStrategy` | driver/commit | bounded |
| `RecoveryStrategy` | `TerminateTask` / `Enqueue(RecoveryTaskDraft)` / `Resume { cursor: TokenId }` | enum | driver/commit | no same-tick re-entry |
| `DiagnosticDecision` | `drafts` | `Vec<DiagnosticDraft>` | commit | stable order; dedupe key `(code, span, task)` |
| `DispatchOutcome` (optional) | `task` | `TaskId` | report | only if F8 accepts |
| | `outcome` | `TaskOutcomeKind` | report | completed/failed/waiting/ready/recovered |

## 4. State machine and ordering rules

### 4.1 Task state machine

```text
                         (bootstrap or Enqueue, ready_tick = tick+1)
                                         |
                                         v
   +------------------------------- Ready -------------------------------+
   |                                 |                                   |
   | cancel (job terminal)           | dispatch (guard admit)            |
   |                                 v                                   |
   |                              Running                                |
   |        +----------------+--------+---------+---------------+        |
   |        |                |                  |               |        |
   |   Complete            Fail             AwaitHost       AwaitChildren  Progress
   |        |                |                  |               |        |
   |        v                v                  v               v        v
   |  Completed(result)  Failed(diag)  Waiting(host)   Waiting(children) Ready(tick+1)
   |                                          |               |
   |                                          | host response | join all Completed
   |                                          +--> Ready      +--> Ready(tick+1)
   |                                                          |
   |                                                          | join any Failed
   |                                                          +--> Failed(diag)
   |
   +-- commit failure / H6: Running -> Failed(diag or NONE), exactly once
```

The diagram is a requirement sketch, not a claim that every edge is implemented. The implemented edges are the `/39` ones in §2. The proposed host-response edge is `[ABSENT]` (CT02). The `Ready`-under-cancel edge is the current frozen test behavior.

### 4.2 Job state machine

| From | Event | To | Guard |
|---|---|---|---|
| `Idle` | CT01 start | `Running` | config valid; no duplicate start |
| `Running` | CT09/CT14 completion | `Finished` | all tasks terminal, none failed, artifacts finalized |
| `Running` | any failed task / CT13 cancel / tick budget | `Failed` | terminal; no further dispatch |
| `Finished`/`Failed` | any start/cancel/select | same | idempotent; no re-entry |

Current `/39`: `Idle` is the default; cancel sets `Failed`; the shell does not set `Running` or `Finished`; `start_job` is unused. `[PARTIAL /39]`.

### 4.3 Phase/stage gate

The gate is a pure function of committed facts:

```text
advance = (no Ready task in the canonical ready set)
       && (no Waiting task)
       && (tasks.in_flight is empty)
       && (no task is Running)
```

If `advance`, CT09 proposes the next `Stage`; otherwise it holds. `Waiting` always blocks; a host wait is an external dependency, not progress. The gate never reads `wires`, drafts, or `in_flight` as a work source; it reads them only to assert emptiness. `[PROPOSED A16]`

### 4.4 Ordering rules

| ID | Order | Scope | Current `/39` | Proposed A16 |
|---|---|---|---|---|
| O1 | `(stage ordinal, group priority, enqueue ordinal, TaskId)` | selection | `(group priority, enqueue ordinal, TaskId)` (`routing.rs:223-243`) | stage-first key; freeze F1 |
| O2 | batch order = O1 order | dispatch application | batch order; quota 1 one task | keep |
| O3 | worker emission order = dispatch order; proposal index within a task | proposal collection | dispatch order; worker order | keep |
| O4 | `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal_index)` | commit | `(enqueue_ordinal, TaskId, proposal_index)` (`commit.rs:541-551`) | add dispatch ordinal; freeze F1 |
| O5 | apply order = O4 order | atomic apply | same as current commit order | keep with O4 |
| O6 | ascending `TaskId` | join scan | `waiter_children.sort_unstable()`; `poll_await_joins` sorts (`commit.rs:1339-1341`, `:2450-2452`) | keep; add `tasks.waiting` index |
| O7 | dispatch order | H6 recovery | `fail_selected` iterates `dispatched` (`routing.rs:583-587`) | keep |
| O8 | one append after commit/recovery, before latch | report | `record_tick` (`routing.rs:679-695`) | keep |
| O9 | after all outcomes/recovery, at current latch | `in_flight` clear | end of `propagate` (`routing.rs:512`, `:521`) | freeze owner F4 |
| O10 | after commit/join, before latch | phase gate | absent | CT09 |
| O11 | parent commit order | result consumption | `consume_result` caller-defined | parent-only contract |
| O12 | once per tick, before O1 | cancel | `routing.rs:311-324` | keep |

### 4.5 Reference tick sequence (proposed)

```text
1. reset wires (wire-only; not an in_flight lifecycle step)
2. sample pins (start, cancel, host responses, tick budget)
3. cancel gate (precedence over budget); if cancel: report + latch + advance; return
4. budget gate; if exhausted: report + latch + advance; return
5. CT03 select -> ordered SelectionBatch (quota 1: one entry)
6. dispatcher: preflight batch; Ready -> Running; populate in_flight; remove from ready
7. for each dispatched task in order: CT04 guard; routed worker; collect proposals
8. CT05 validate the whole batch (read-only)
9. CT06 propose the ordered commit
10. commit path: capacity preflight; one atomic apply; report
11. if commit failed: H6 recovery (dispatch order, exactly once); no semantic state
12. CT07 join decisions (commit-apply; no consumption)
13. CT09 phase gate (committed facts only)
14. CT13 cancel decision (already applied at step 3; no second evaluation)
15. step-6 report append (one TickRecord)
16. assert in_flight empty; latch (in_flight clear owner per F4); advance tick
```

Steps 5–14 are the target topology; `/39` implements a reduced form of 1–10, 12, and 15–16 in `RoutingShell::propagate_inner`.

### 4.6 Exactly-once obligations (test requirements, not a proof)

| Obligation | Guard | Evidence | Gap |
|---|---|---|---|
| One dispatch per task per tick | `select_batch` returns distinct ready tasks; `tasks.ready` remove; `in_flight` | `routing.rs:377-404` | quota > 1 duplicate selection absent (`DuplicateSelection`) |
| One transition per task per batch | `completing` set; `DuplicateCompletion` | `commit.rs:1198-1244`, `:1260-1270` | none for the current path |
| Empty vector fails once | `empty_proposal_fail`; `TaskNotTransitioned` | `routing.rs:477-507`; `commit.rs:2817-2827` | freeze the rule (CT_FREEZE_02 calls it open) |
| One result per `Complete` | apply pushes exactly one `ResultRecord` | `commit.rs:1455-1465` | none |
| Result consumed once | `ResultRecord.consumed`; `consume_result` | `commit.rs:2091-2119` | no consumer identity |
| Join readies/fails once | only `Waiting` parents; simulation + poll | `commit.rs:1315-1388`, `:2421-2528` | dangling child stays waiting |
| Progress reinsert once | `completing` set; `reinsert_ready` removes duplicates | `commit.rs:1198-1228`, `:2297-2310` | progress-fail capacity not preflighted |
| Recovery fails once | `try_fail_task` only `Running` | `commit.rs:2829-2869` | exact latch owner open |
| Cancel idempotent | `cancel_requested` latched; no dispatch | `routing.rs:311-324`; `c06_routing.rs:288-309` | ready/waiting disposition |
| Report once | sole `record_tick` append | `routing.rs:679-695` | quota > 1 reports first only |

## 5. Ownership and producer-consumer matrix

### 5.1 Field-level writer matrix

| Field / shape | Sole writer | Phase | Readers | Forbidden writers | Notes |
|---|---|---|---|---|---|
| `control.job_state` | driver/control-apply from CT01/CT09/CT13 decisions | job start/cancel/finish | all | chips, workers | one writer per transition; no chip direct write |
| `control.stage` | driver/CT09 apply | phase gate | report, CT14 | workers | derived from committed facts |
| `control.selected` | dispatcher | dispatch | quota-1 consumers | chips, workers | projection only |
| `control.next_enqueue_ordinal` | commit/bootstrap | enqueue | scheduler | workers | monotonic |
| `control.next_diagnostic_ordinal` | commit/recovery | fail | diagnostics | workers | monotonic |
| `control.budget_exhausted` | driver | budget gate | all | workers | latched |
| `control.cancel_requested` | driver | cancel gate | all | workers | latched; lifecycle decision open |
| `tasks.ready` | dispatcher remove; commit reinsert; bootstrap | dispatch/commit | CT03/CT09 | workers | canonical at quota 1 |
| `tasks.active` | dispatcher | dispatch | quota-1 | workers | projection |
| `tasks.in_flight` | dispatcher populate/clear | dispatch/recovery | driver | workers | ephemeral; not a transition |
| `tasks.waiting` (proposed) | commit apply/join | wait/resume | CT07/CT09 | workers | ascending `TaskId` |
| `Task.state` | dispatcher, commit apply, join, recovery | dispatch/commit/join/recovery | all | workers | one transition per phase; `Running` guard |
| `Task.progress_count/ordinal` | commit apply | progress | CT10/commit | workers | monotonic |
| `ResultRecord` | commit apply | complete | parent | workers, join | no join consumption |
| `DiagnosticRecord` | commit apply, recovery, CT11 apply | fail | all | workers | stable ordinal |
| `HostRequestRecord` | commit apply (AwaitHost/CT08) | host wait | Host/CT02 | workers | `satisfied` once |
| `ContinuationRecord` | commit append (proposed) | append | parser | workers | no path today |
| `WaitSet` | commit apply | await | join | workers | sole child source |
| `TickRecord`/`bus.report` | driver step 6 | report | tests/replay | workers | one per tick |
| `CommittedPatch`/`store_versions` | commit apply | patch | all | workers | version guard |
| `ArtifactRecord` | commit append | append | CT14 | CT14 (read-only) | CT14 never appends |
| `wires.proposals` | each worker adapter (own task) | worker | commit | other workers | emission order |
| `wires.selected` | dispatcher | dispatch | quota-1 | workers | projection |
| `wires.phase_advance_requested` | CT09 adapter (proposed) | phase | driver | workers | advisory; unused today |

### 5.2 Chip producer-consumer matrix

| Producer | Output | Consumer | Dependency type | Status |
|---|---|---|---|---|
| CT01 | `JobReady`/first `TaskDraft` | dispatcher/commit | task enqueue | `[ABSENT]` |
| CT02 | `SourceImported` | commit/pipeline | sources append | `[ABSENT]` |
| CT03 | `SelectionBatch` | dispatcher | scheduler decision | `[ABSENT]` |
| CT04 | `GuardDecision` | dispatcher | dispatch decision | `[ABSENT]` |
| CT05 | `ValidatedBatch` | CT06/commit | validation decision | `[ABSENT]` |
| CT06 | `CommitDecision` | commit path | ordered commit | `[ABSENT]` |
| CT07 | `JoinDecision` | commit apply | join decision | `[ABSENT]` (join is commit-apply today) |
| CT08 | `HostRequestDraft` | Host/CT02 | host request | `[PARTIAL]` via `AwaitHost` |
| CT09 | `PhaseDecision` | driver | phase gate | `[ABSENT]` |
| CT10 | `Progress`/`Fail` | commit | progress decision | `[PARTIAL]` via `Proposal::Progress` |
| CT11 | `DiagnosticDecision` | commit | diagnostic appends | `[PARTIAL]` via `Fail` |
| CT12 | `RecoveryDecision` | driver/commit | recovery | `[ABSENT]` |
| CT13 | `CancelDecision` | driver | cancel | `[PARTIAL]` shell |
| CT14 | `HostRequestDraft` | Host | artifact write request | `[ABSENT]` |

### 5.3 Forbidden edges

- Any chip calling another chip or the motherboard (`AGENTS.md` §3.3; Guardrails §2.3).
- Any chip reading `std::fs`, `std::env`, `std::process`, `std::time`, `std::thread`, `std::net`, or a clock/RNG.
- Any worker writing `TaskState`, queues, stores, `control`, diagnostics, results, or artifacts directly.
- Any second writer to `tasks.ready`/`stage_queues`; `tasks.ready` is not a second write target for `stage_queues`.
- Any durable cursor/`WaitSet`/join pointing at a draft, wire, or address.
- Any join consuming a child result; any `ResultValue::DraftRecords` (not in `/39`).
- Any phase advance from a wire/Done signal alone; any compile work inside CT09.
- Any task `Running` at latch; any residual `in_flight` entry at latch.
- Any "success" for an empty/transition-free/unimplemented worker; any test special-casing or GCC/Clang delegation.

## 6. Failure, cancel, and replay case matrix

The "Current `/39`" column is behavior observed/implemented in source; it is not a claim that the case is tested or proven. `[ABSENT]` means no path.

| # | Case | Required behavior (A16) | Current `/39` | Gap / action |
|---|---|---|---|---|
| C01 | Empty selection | No dispatch; `Idle`; no mutation; one report record | `select_batch` empty → `poll_await_joins` → `Idle`/`Joined` (`routing.rs:342-367`) | freeze the `Idle`/`Joined` distinction |
| C02 | Oversized selection | Pre-dispatch `SelectionBatchOverflow`; affected tasks stay `Ready`; no mutation | `CommitError::SelectionBatchOverflow` before `Ready→Running` (`routing.rs:368-376`; `commit.rs:244-258`) | carrier conflict; move to dispatcher error or freeze on `CommitError` |
| C03 | Duplicate selection | Pre-dispatch `DuplicateSelection`; no mutation | `[ABSENT]` | add type/error and test |
| C04 | Wrong owner/kind/route | Guard rejects; no worker; no mutation | `drive_task`/handler checks owner/kind/route; `CommitError` on commit | CT04/CT05 formalization |
| C05 | Wrong stage/layer | Guard/driver rejects; no worker | `DriveError::StageLayerMismatch`; handler fail (`chips/mod.rs:325-334`, `:379-391`) | freeze as guard check |
| C06 | Stale patch version | Reject before apply; no partial | `CommitError::StaleVersion` (`commit.rs:1848-1972`) | guard/version snapshot |
| C07 | Commit validation failure | No semantic state; H6 recovery in dispatch order; no `Running`; in-flight empty | `fail_selected` (`routing.rs:565-591`); tests `c06_routing.rs:349-369` | exact latch owner; all-path proof |
| C08 | Capacity/backpressure failure | Whole-batch reject before apply; H6 recovery | `check_capacity`/`check_backpressure_capacity` (`commit.rs:1390-1438`); `try_fail_task` | progress-fail diagnostics not preflighted |
| C09 | Proposal budget exceeded | Whole-batch reject before apply; H6 recovery | `check_proposal_budget` (`commit.rs:538-539`, `:1394-1397`) | per-dispatch vs whole-batch interaction open (T02 H9.3) |
| C10 | Empty/transition-free proposal vector | Per-task `TaskNotTransitioned` `Fail`; never success; never `Running` | routing injects `empty_proposal_fail`; commit rejects (`routing.rs:477-507`; `commit.rs:2817-2827`) | freeze the explicit rule (CT_FREEZE_02 calls it open) |
| C11 | Duplicate completion | Whole-batch `DuplicateCompletion`; no mutation; H6 recovery | `commit.rs:1198-1244`; `c09_readiness.rs:229-247` | none |
| C12 | Progress ordinal non-advancing | Per-task `NonAdvancingProgress` fail or whole-batch reject; freeze one | per-task fail at apply (`commit.rs:1210-1215`, `:1667-1670`) | conflict with CT_FREEZE_02; freeze |
| C13 | Progress limit exceeded | Per-task `ProgressLimit` fail exactly once; diagnostic or `NONE` | `check_progress_limit`; `try_fail_task` | preflight diagnostic capacity |
| C14 | Progress + writes | Allow atomically or reject; freeze one | current allows writes with `Progress` | direct conflict with CT_FREEZE_02 §2 |
| C15 | Join all `Completed` | `Ready` + one reinsert at `tick+1`; no consumption | commit/poll (`commit.rs:1719-1728`, `:2516-2522`); `c03_task.rs:1210-1257` | add `join_then_replay_retry` |
| C16 | Join any `Failed` | `Failed` once with first failed child diagnostic; never `Ready` | commit/poll (`commit.rs:1729-1758`); `c03_task.rs:1259-1285` | none |
| C17 | Join non-terminal sibling | Stay `Waiting`; do not fail early | `commit.rs:1343-1377`; `c09_readiness.rs:306-350` | none |
| C18 | Join dangling child | Typed fault; fail once; never park forever | missing child → `all_terminal = false` → stays `Waiting` (`commit.rs:2475-2490`) | fix per A16-J04 |
| C19 | Cancel pre-selection | No dispatch/commit; `JobState::Failed`; ready stays `Ready`; in-flight empty; one report | `routing.rs:311-324`; `c06_routing.rs:288-326` | freeze disposition |
| C20 | Cancel vs tick budget | Cancel wins | `routing.rs:311-339`; `c06_routing.rs:288-309` | none |
| C21 | Repeated cancel | Idempotent; one terminal report per tick | `c06_routing.rs:305-308` | `cancel_requested` lifecycle |
| C22 | Tick budget | No dispatch; `budget_exhausted`; repeated ticks stay exhausted | `routing.rs:325-339`; `c06_routing.rs:145-171` | job state not set; freeze |
| C23 | Phase gate with work | Hold; no compile; latch normally | `[ABSENT]` CT09 | implement + test |
| C24 | Phase gate at quiescence | Advance one stage; `Done` only all terminal | `[ABSENT]` CT09 | implement + mapping |
| C25 | Quota > 1 partial failure | Every dispatched task fails once; report all; no `Running` | `fail_selected` iterates batch; report projects first only (`routing.rs:525-532`, `:679-695`) | T13 pending; report shape |
| C26 | Replay identical pins | Identical decisions/order/report/snapshot | deterministic sort; snapshot encodes in_flight/report (`snapshot.rs:802-823`) | `stage_queues`/quota > 1 replay tests |
| C27 | Result double consume | `ResultAlreadyConsumed`; no second value | `consume_result` (`commit.rs:2091-2119`); `c03_task.rs:155-180` | no consumer identity |
| C28 | Late host response after cancel | Do not resurrect; no dispatch; no artifact | `[ABSENT]` CT02/CT08 | define |
| C29 | Duplicate start job | Typed diagnostic; no second job | `[ABSENT]` CT01 | define |
| C30 | Unsupported/unregistered worker | Explicit `Fail`/`Unsupported`; never success | `routing.rs:421-474`; `c06_routing.rs:86-130` | none |

### 6.1 Existing test mapping

| Existing test | Cases covered | Notes |
|---|---|---|
| `c03_task.rs:119-180` | completion exactly once, result consume once | C10/C27 baseline |
| `c03_task.rs:400-465` | failed batch commits nothing, AwaitHost transition guard | C07/C08 |
| `c03_task.rs:550-745` | unknown task, non-running completion, dangling/forward parent, destination auth | C04/C07 |
| `c03_task.rs:829-919` | proposal transitions/wire tags, five `ResultValue` variants | A16-R02 |
| `c03_task.rs:921-962` | nine-field continuation, no `awaited` | A16-C04 |
| `c03_task.rs:978-1058` | error codes, backpressure checked arithmetic | C02/C08 |
| `c03_task.rs:1060-1152` | child refs, progress ordinal/limit/reinsert | C12/C13/C15 |
| `c03_task.rs:1174-1285` | await-all join, no consumption, any-failed | C15/C16/C17 |
| `c06_routing.rs:62-130` | noop/unsupported/unregistered | C30 |
| `c06_routing.rs:132-171` | idle, budget | C01/C22 |
| `c06_routing.rs:173-187` | cancel pin, ready stays Ready | C19 |
| `c06_routing.rs:189-255` | next-tick enqueue, commit failure recovery | C03/C07 |
| `c06_routing.rs:256-369` | malformed wires, cancel precedence, one report, in-flight empty | C07/C19/C21 |
| `c06_routing.rs:371-394` | quota selection | O1/O2 |
| `c09_readiness.rs:107-205` | limits/hash, append-only reject, in-flight empty | C08/C10 |
| `c09_readiness.rs:207-304` | idle join drain, closure | C16/C17 |
| `c09_readiness.rs:306-350` | await-all slow sibling | C17 |
| `c09_readiness.rs:351-466` | own-batch parent, draft index, append/patch conflict | C04/C14 |
| `c09_readiness.rs:500-555` | wrong layer on driver path | C05 |
| `c09_readiness.rs:616-639` | small-profile bounded ticks | performance note, not a general bound |

### 6.2 Replay requirements

- Identical initial semantic state, config, and pins must produce an identical snapshot and report. `[FROZEN /39]` (C05 snapshot; `snapshot.rs`).
- Scheduler-only registers (`stage_queues`, `in_flight`, `dispatch_cursor`, `PipelineMetrics`, `stage_assignment_version`) are outside the **canonical semantic comparison projection** but must themselves be deterministic/replay-identical. `[PROPOSED A16]`; T02/M1 proposal.
- The `report` and `in_flight` are snapshot-encoded today (`snapshot.rs:802-823`); adding fields changes the snapshot. Any ledger extension must update `Snapshot::capture` and the freeze test.
- A cancel/budget/recovery path must be replayable from the same pins; no wall-clock or environment input.

## 7. Conflicts with candidate freeze drafts

### 7.1 CT_FREEZE_01 (CT01–CT04)

| Draft item | Draft position | `/39` fact | A16 assessment / required action |
|---|---|---|---|
| OPEN-01 spawn payload typing | Recommend typed per-kind payload structs | `Payload` is `Vec<RecordRef>` only (`task.rs:1245-1263`) | **Conflict.** Do not add a payload enum for M1; use a typed `decode` adapter per kind (the `ConstantRequest::decode` precedent) or a new freeze. Recommend decode adapter; freeze F2. |
| OPEN-02 `Enqueue` + `ContinuationRef` | Recommend transient `OwnBatch` | `TaskDraft.continuation: Option<ContinuationId>`; `ContinuationRef` unused (`task.rs:1791-1810`, `:1660-1674`) | **Conflict.** Either migrate the field to `Option<ContinuationRef>` or drop `ContinuationRef`. No materializer exists. Freeze F2. |
| OPEN-03 CT04 `AwaitChildren` guard depth | Shape-only; commit resolves | Commit resolves `OwnBatch` and checks parent (`commit.rs:1286-1313`) | Aligned; CT04 must not resolve. |
| OPEN-04 join result carrier | Boundary; not decided | Join does not consume (`commit.rs:1719-1728`) | Aligned; freeze no-consume + `join_then_replay_retry`. |
| OPEN-05 `WaitSet` shape | `children + host_request`, no continuation dup | Exactly that (`task.rs:1709-1716`) | Aligned. |
| OPEN-06 `WaitSet` writer | Commit-apply only | Commit apply only (`commit.rs:1476-1491`, `:1678-1698`) | Aligned. |
| OPEN-07 await-all vs await-any | Await-all | Commit/poll await-all (`commit.rs:1343-1385`, `:2456-2505`) | Aligned. |
| OPEN-08 reinsertion tick | Next tick | `ready_tick = tick+1` (`commit.rs:1445`, `:2517`) | Aligned. |
| OPEN-09 9-field inventory | Candidate generic list | Frozen parse-oriented 9 fields (`task.rs:1546-1576`) | **Conflict.** Reject the generic list; keep `/39`. |
| OPEN-10 pointer vs inline | `Option<ContinuationId>` | Frozen pointer | Aligned. |
| OPEN-11 absent continuation | Registry-gated `None` | `None` is unconditionally legal; no registry gate | **Conflict.** For M1 keep `None` legal; add a `ContinuationPolicy` table only by freeze. |
| OPEN-12 `ContinuationDraft` path | `AppendRecords` only | `G1DraftBody` has no `Continuation`; only `tasks.active.continuation` is declared, no append field | **Conflict.** Add the body/store/schema/snapshot/allowlist or defer. Recommend defer for M1; freeze F2. |
| OPEN-13 phase value set | Stage ordinal as priority | `phase_priority = group.raw()`; `stage_of` separate | **Conflict.** Freeze F1 chooses stage-first or group-first. |
| OPEN-14 phase storage | Derived from kind registry | No per-task phase field; `control.stage` is job-level | Aligned for per-task; `control.stage` remains observable. |
| OPEN-15 selection key | Collapse to stage-first | Current group-first | **Conflict.** Same as OPEN-13. |
| OPEN-16 CT09 `Done` authority | Advisory; queues decide | No CT09; `phase_advance_requested` unused | Aligned recommendation; implement. |

### 7.2 CT_FREEZE_02 (CT05–CT09)

| Draft item | Draft position | `/39` fact | A16 assessment / required action |
|---|---|---|---|
| CT05 Progress+write prohibition | A `Progress` proposal must not carry `StorePatch`/`AppendRecords`; mixed vectors rejected | Frozen coexistence allows `StorePatch`/`AppendRecords` to accompany a transition; `Progress` may coexist (`task.rs:1869-1890`); no check in commit | **Direct conflict.** Recommend allowing atomic writes with `Progress` for M1 (A16-P04); if the draft wins, add the check and update fixtures. Freeze F2. |
| CT05 Progress shape-only; exceedance at CT06 | Shape in CT05, exceedance at commit | Progress ordinal/limit checked in commit (`commit.rs:1198-1228`) | Aligned if CT05 is added; current no CT05. |
| CT05 empty vector "unresolved" | OB-2 open | `/39` has explicit `TaskNotTransitioned` per-task fail (`routing.rs:477-507`; `commit.rs:2817-2827`) | **Resolved by implementation.** Freeze the explicit rule; do not keep OB-2 open. |
| CT06 budget order | (a) progress exceedance → per-task Fail; (b) proposal budget; (c) per-stage capacity | Current proposal budget first (`:538`), progress per-task at apply (`:1667`), capacity/backpressure after validation (`:1390-1438`) | **Conflict/order gap.** Freeze a total no-mutation preflight; include progress-fail diagnostic capacity. |
| CT06 Progress reinsert | Own stage queue; exactly once | `tasks.ready`; `reinsert_ready`; `DuplicateCompletion` | Aligned except queue shape. |
| CT06 Progress no writes | A tick with `Progress` for T commits no semantic mutation from T | Current allows writes from T | **Conflict.** Same as CT05 row. |
| CT07 await-all/no consume | Await-all; no consume; bounded | Aligned; scan is unbounded | Add `tasks.waiting`/bound; freeze F5. |
| CT07 "already-consumed child" | Reject a join on a consumed child | Join reads state, not results; `consumed` is on the result, not the task | **Contradiction.** Clarify: the join must not require an unconsumed result; consumption is orthogonal. Reject the wording. |
| CT08 host wait pause | Progress count paused; no wall clock | `AwaitHost` sets `Waiting`; no resume path; no progress reset | **Partial.** CT02/CT08 must implement resume; freeze count-preserve (A16-P05). |
| CT09 gate "empty/waiting" | Advance when queues empty/waiting | No CT09 | **Ambiguous.** Freeze "no Ready and no Waiting" (A16-H01). |
| CT09 budget taxonomy | `max_task_progress`, `max_proposals_per_tick`, `max_inflight_per_tick`, `stage_queue_bound`, diagnostic/record capacity | All names exist (`limits.rs`); `stage_queue_bound` is not per-stage enforced | Aligned names; per-stage enforcement open. |
| CT09 oscillation guards | O1–O4 | Ordinal/limit exist; no non-advancing detection; join deterministic | O1 detection open; O2–O4 testable. |
| CT09 fixtures | Nine fixtures | Some existing tests cover parts | Map to §6.1; add missing. |

### 7.3 CT_FREEZE_03 (CT10–CT14)

| Draft item | Draft position | `/39` fact | A16 assessment / required action |
|---|---|---|---|
| CT10 progress/budget over batch | No wall clock | Commit enforces count/ordinal; no CT10 | Aligned conceptually; CT10 is absent. |
| CT10 reinsert own stage queue | Own `stage_queues` | `tasks.ready` | **Conflict** with queue shape; defer `stage_queues` or freeze F3. |
| CT11 diagnostics | Append proposals, stable order, dedupe | `Fail` commits one diagnostic per failure; no dedupe chip | **Partial.** CT11 may be decorative; freeze whether dedupe is required. |
| CT12 recovery | Explicit strategy by phase; no swallow | No CT12; `PaRecoveryChip` is a parse worker | **Absent.** Implement A16-F06; do not conflate with H6. |
| CT13 cancel | In-flight set + ready tasks; designated path | Shell applies pre-selection; ready stays Ready; no in-flight at cancel | **Partial.** Freeze disposition; CT13 decision-only. |
| CT14 artifacts | Read-only; Host request; no append | No CT14 | **Absent.** Implement. |
| §3.2 `SelectionBatchOverflow` not `CommitError` | Dispatcher/scheduling failure | `/39` carries it on `CommitError` (`commit.rs:244-258`; T01 §7.1 known open) | **Known conflict.** Freeze carrier F1; do not silently change both. |
| §3.3 `BackpressureCapacity` | `CommitError` | `CommitError` (`commit.rs:228-243`) | Aligned. |
| §4.1 in-flight pre-worker | Dispatcher populates | Dispatcher populates (`routing.rs:392-399`) | Aligned. |
| §4.3 clear at current latch | Clear at latch only | Shell clears at end of `propagate` (`routing.rs:512`, `:521`), before `latch`; no `latch` override | **Nuance/conflict.** Freeze owner/order; current observable passes but is not a latch override. |
| §5.1 cancel once per tick | Before selection | Aligned (`routing.rs:311-324`) | Freeze. |
| §5.2 no dispatch/commit | Aligned | Aligned | Freeze. |
| §5.3 disposition unresolved | Ready/waiting disposition | Ready stays Ready; waiting stays Waiting | **Open.** Freeze or add `Cancelled`. |
| §5.4 cancel latch post-condition | in_flight empty | Shell clears | Aligned observable. |
| §6 open items 1–7 | Various | See F1/F3/F4/F5 | Freeze work orders. |

### 7.4 Cross-draft conflicts

| Topic | CT_FREEZE_01 | CT_FREEZE_02 | CT_FREEZE_03 | `/39` | A16 recommendation |
|---|---|---|---|---|---|
| Selection key | stage-first (OPEN-13/15) | assumes stage queues | batch order `(stage, phase, enqueue, TaskId)` | group-first | stage-first with group tie-break; F1 |
| `SelectionBatchOverflow` | not `CommitError` (via M1 proposal) | not `CommitError` | not `CommitError` | `CommitError` | move to dispatcher error or freeze on `CommitError`; F1 |
| Progress + writes | — | forbidden | — | allowed | allow atomically; F2 |
| Continuation fields | generic 9-field list | assumes continuations | — | parse 9-field | keep `/39`; defer materialization; F6 |
| `in_flight` clear | — | — | at latch | end of propagate | freeze latch owner; F4 |
| Stage queues | — | own stage queues | own stage queues | single ready | keep single at quota 1; defer stage queues; F3 |
| Cancel disposition | — | — | unresolved | ready stays Ready | freeze ready-stays-Ready or add state; F4 |
| CT07 carrier | no new carrier | no new carrier | — | join in commit | freeze; F5 |

### 7.5 Conflict-resolution rules

1. `/39` wins until an integrator freeze says otherwise; a draft recommendation is not authority.
2. A conflict that changes a frozen shape/rule requires a version bump, hash recomputation, and dependent-test revalidation.
3. A conflict that changes only a decision/wire (no committed record) still requires a freeze item because replay/snapshot and tests may change.
4. Do not resolve a conflict by adding a private chip-local type that duplicates a shared shape.
5. If two drafts disagree with each other and `/39`, freeze the smallest shape that preserves quota-1 correctness first; defer the larger pipeline shape.

## 8. Ordered freeze work orders

These are **freeze** work orders, not implementation authorizations. The integrator owns shared files; chip owners own only their proposed chip files after the freeze. Each order names exact paths and acceptance. None is started by this report.

**"`/39` protocol" in this report** means the integrator's serial freeze procedure evidenced by the `/39` SE29 slice ([SE_FUNC_SLICE.md](../../tasks/SE_FUNC_SLICE.md) §3–§6): verify the current head and free local/chip IDs linearly; add the kind to the cumulative registry; add the stage row and routed layer; add the manifest/allowlist row; add the hashed rule IDs; bump `CONTRACT_VERSION` and recompute the hash; update the affected pins (`freeze.rs`, dependent tests, both READMEs, T01 §7.1, `CHIP_PLAN.md`); run the slice acceptance. A proposal that bypasses this procedure is not a freeze, even if it compiles.

### F1 — Dispatch/commit ordering and error carriers

- **Owner:** T01 integrator (shared schema/routing).
- **Exact paths:** `compiler/src/routing.rs`; `compiler/src/commit.rs`; `compiler/src/task.rs`; `compiler/src/contract.rs`; `compiler/contracts/CONTRACT_VERSION`; `compiler/tests/c03_task.rs`; `compiler/tests/c06_routing.rs`; `compiler/tests/freeze.rs`.
- **Decision:** choose the selection key (recommend `(stage ordinal, group priority, enqueue ordinal, TaskId)`), the commit key (recommend `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal_index)`), and the `SelectionBatchOverflow` carrier (recommend a dispatcher/scheduling error type; if `CommitError` is kept, record the exception explicitly).
- **Acceptance:** deterministic selection fixtures for mixed-group stages; quota-1 differential before/after; duplicate selection rejected pre-dispatch; commit order test with two dispatch ordinals; `SelectionBatchOverflow` reachable exactly once; hash/version/freeze test updated; no behavior change for no-op/unsupported/unregistered.
- **Depends:** none. **Blocks:** F2, F3, B-CTL-02, B-CTL-03.

### F2 — Task/result/continuation/WaitSet wiring and coexistence

- **Owner:** T01 integrator.
- **Exact paths:** `compiler/src/task.rs`; `compiler/src/records.rs`; `compiler/src/commit.rs`; `compiler/src/bus.rs`; `compiler/src/manifest.rs`; `compiler/src/snapshot.rs`; `compiler/tests/c03_task.rs`; `compiler/tests/c05_codec.rs`; `compiler/tests/c09_readiness.rs`.
- **Decision:** freeze empty-vector `TaskNotTransitioned`; freeze `Progress` + writes (recommend allow atomically); freeze `TaskDraft.continuation` (`ContinuationId` vs `ContinuationRef`); freeze continuation materialization (recommend defer for M1, `None` only); freeze result consumer contract (parent-only).
- **Acceptance:** empty vector fails once; progress+append/patch test per decision; `ContinuationRef` either wired or explicitly unused; result consume-once test; snapshot encodes any new field; no `DraftRecords`.
- **Depends:** F1. **Blocks:** B-CTL-03, B-CTL-04, B-CTL-06.

### F3 — Queue, ledger, and phase/stage vocabulary

- **Owner:** T01 integrator.
- **Exact paths:** `compiler/src/bus.rs`; `compiler/src/limits.rs`; `compiler/src/manifest.rs`; `compiler/src/routing.rs`; `compiler/src/snapshot.rs`; `compiler/src/contract.rs`; `compiler/tests/c06_routing.rs`; `compiler/tests/freeze.rs`.
- **Decision:** keep `tasks.ready` canonical at quota 1 (recommend) or add `stage_queues` with a derived `ready`; add `tasks.waiting` or document the scan; define the `Stage`/`stage_of`/`STAGE_COUNT` mapping; decide `TickRecord.tick`/`outcomes`.
- **Acceptance:** queue audit (no second writer); bounded join; phase gate tests; stage/layer agreement; snapshot/replay; `max_ticks + 1` report bound; `stage_queue_bound` interpretation documented.
- **Depends:** F1. **Blocks:** B-CTL-05, B-CTL-08.

### F4 — Cancel, H6 recovery, and progress budget

- **Owner:** T01 integrator + T02 owner.
- **Exact paths:** `compiler/src/routing.rs`; `compiler/src/commit.rs`; `compiler/src/bus.rs`; `compiler/src/limits.rs`; `compiler/tests/c06_routing.rs`; `compiler/tests/c03_task.rs`; `compiler/tests/c09_readiness.rs`.
- **Decision:** freeze cancel disposition (`Ready` stays `Ready` or add `Cancelled`), `cancel_requested` lifecycle, cancel-vs-budget precedence, H6 diagnostic policy, in-flight clear owner/order, progress-fail capacity preflight, progress count across waits.
- **Acceptance:** every path (success, commit failure, pre-dispatch error, empty vector, cancel, budget, join-only, progress-only) asserts no `Running`, `in_flight = ∅` at latch, one report; recovery hash pre/post; `DiagnosticId::NONE` branch; repeated cancel; no wall clock.
- **Depends:** F1/F2. **Blocks:** B-CTL-05, B-CTL-07.

### F5 — Join, result delivery, and host wait

- **Owner:** T01 integrator + T05/T02 owners.
- **Exact paths:** `compiler/src/commit.rs`; `compiler/src/task.rs`; `compiler/src/bus.rs`; `compiler/tests/c03_task.rs`; `compiler/tests/c09_readiness.rs`; new `compiler/tests/c43_control_join.rs`.
- **Decision:** freeze await-all/no-consume; dangling-child fault; `tasks.waiting`/bound; host-response resume; `join_then_replay_retry`; CT08 request shape.
- **Acceptance:** all-completed/any-failed/non-terminal/dangling; no double consume; replay retry; host wait pause/resume; one request per wait.
- **Depends:** F2/F3. **Blocks:** B-CTL-04.

### F6 — Control service interfaces and manifests

- **Owner:** T01 integrator + T02 owner.
- **Exact paths:** `compiler/src/manifest.rs`; `compiler/src/task.rs`; `compiler/src/bus.rs`; `compiler/src/chips/mod.rs`; `compiler/contracts/COMPILER_SFL_MANIFEST.md`; `compiler/tests/c04_manifest.rs`; `compiler/tests/freeze.rs`.
- **Decision:** decide whether CT01–CT14 are driver-invoked services (recommend) or task workers; if services, extend the manifest/validator to allow `task_kinds: []` for a decision/service capability or invoke them outside `ManifestRegistry`; if workers, freeze 14 new control kinds at local `16+`, a `control_slice()`, stage rows, layers, allowlists, and tests.
- **Acceptance:** no synthetic task kinds; manifest registration; unique chip IDs; stage/layer agreement; field-scoped reads/writes; zero-field worker check; control service decision types tested.
- **Depends:** F1. **Blocks:** all B-CTL.

### F7 — Diagnostic, recovery, and artifact-finalize interfaces

- **Owner:** T01 integrator + T02/T13 owners.
- **Exact paths:** `compiler/src/bus.rs`; `compiler/src/task.rs`; `compiler/src/commit.rs`; `compiler/src/manifest.rs`; `compiler/tests/c03_task.rs`; new `compiler/tests/c45_control_recovery.rs`; new `compiler/tests/c46_control_cancel.rs`.
- **Decision:** CT11 dedupe/order; CT12 recovery strategy shape and bound; CT14 artifact finalize/Host request; late host response after cancel; missing fragments.
- **Acceptance:** original diagnostic retained; recovery bounded; no artifact on failure/cancel; no I/O; host request exactly once; dedupe stable.
- **Depends:** F4/F5. **Blocks:** B-CTL-06, B-CTL-07.

### F8 — Ledger/report and acceptance-freeze pins

- **Owner:** T01 integrator.
- **Exact paths:** `compiler/src/bus.rs`; `compiler/src/snapshot.rs`; `compiler/src/contract.rs`; `compiler/src/routing.rs`; `compiler/contracts/CONTRACT_VERSION`; `compiler/tests/freeze.rs`; `compiler/tests/c05_codec.rs`; `compiler/tests/c06_routing.rs`.
- **Decision:** finalize `TickRecord`/`TickMetrics` shape, per-dispatch outcomes (if any), report bound, and the new rule IDs/hash/version.
- **Acceptance:** deterministic snapshot round-trip; report one per tick; quota-1 projection; quota > 1 report (if accepted); freeze test green; all affected pins updated by the integrator.
- **Depends:** F1–F7. **Blocks:** B-CTL-08.

## 9. Ordered implementation work orders (Stage B)

These are **proposed** work orders. They may start only after the named freeze item is accepted. They do not authorize any file edit by themselves. Each work order names exact proposed paths; the integrator owns `mod.rs`, registration, and shared test entry points.

### B-CTL-01 — CT01 job start + CT02 source response

- **Owned files:** `compiler/src/chips/control/ct01_job_start.rs`; `compiler/src/chips/control/ct02_source_response.rs` (proposed; directory absent).
- **Tests:** `compiler/tests/c40_control_start.rs` (proposed).
- **Reads:** `control.job_state`, `config`, `sources`; `CompilerPins.start_job`/`host_responses`.
- **Writes:** `control.job_state` only through the driver; `sources` only through commit.
- **Acceptance:** duplicate start diagnostic; first task bootstrap; out-of-order/duplicate source; identical bytes replay; no direct store write; `cargo test --manifest-path compiler/Cargo.toml --test c40_control_start`.
- **Freeze deps:** F5/F6.

### B-CTL-02 — CT03 selection + CT04 guard

- **Owned files:** `compiler/src/chips/control/ct03_task_select.rs`; `compiler/src/chips/control/ct04_task_guard.rs`.
- **Tests:** `compiler/tests/c41_control_select.rs`.
- **Reads:** `tasks.ready`, `tasks.in_flight`, `control.tick`, `config`, `routing`, `registrations`, `stage_of`.
- **Writes:** none; decisions only (`SelectionBatch`, `GuardDecision`).
- **Acceptance:** deterministic batch; one entry per stage; `SelectionBatchOverflow`/`DuplicateSelection` pre-dispatch; wrong owner/kind/stage/layer/state rejected; no mutation; `cargo test --manifest-path compiler/Cargo.toml --test c41_control_select`.
- **Freeze deps:** F1/F6.

### B-CTL-03 — CT05 validation + CT06 commit decision

- **Owned files:** `compiler/src/chips/control/ct05_proposal_validate.rs`; `compiler/src/chips/control/ct06_task_commit.rs`.
- **Tests:** `compiler/tests/c42_control_commit.rs`.
- **Reads:** tagged proposals, task states, manifests, schema, store versions, limits.
- **Writes:** none; decisions only (`ValidatedBatch`, `CommitDecision`).
- **Acceptance:** one transition per task; empty vector fails; duplicate completion; cross-task/append/patch conflicts; predicted refs; canonical commit order; no mutation; `cargo test --manifest-path compiler/Cargo.toml --test c42_control_commit`.
- **Freeze deps:** F1/F2/F6.

### B-CTL-04 — CT07 join + CT08 host request

- **Owned files:** `compiler/src/chips/control/ct07_result_resume.rs`; `compiler/src/chips/control/ct08_host_request.rs`.
- **Tests:** `compiler/tests/c43_control_join.rs`.
- **Reads:** committed task states, `WaitSet`, `host_requests`, artifacts.
- **Writes:** none; decisions only (`JoinDecision`, `HostRequestDraft`); commit applies the wait/request.
- **Acceptance:** await-all all-completed/any-failed/non-terminal/dangling; no consumption; replay retry; host request once; `cargo test --manifest-path compiler/Cargo.toml --test c43_control_join`.
- **Freeze deps:** F5/F6.

### B-CTL-05 — CT09 phase advance + CT10 progress budget

- **Owned files:** `compiler/src/chips/control/ct09_phase_advance.rs`; `compiler/src/chips/control/ct10_progress_budget.rs`.
- **Tests:** `compiler/tests/c44_control_phase.rs`.
- **Reads:** canonical queues/task states, `control.stage`, `limits`.
- **Writes:** none; decisions only (`PhaseDecision`, `Progress`/`Fail`); commit/driver applies.
- **Acceptance:** gate holds with ready/waiting; advances at quiescence; `Done` only all terminal; progress ordinal/count/zero/reinsert; no wall clock; `cargo test --manifest-path compiler/Cargo.toml --test c44_control_phase`.
- **Freeze deps:** F3/F4/F6.

### B-CTL-06 — CT11 diagnostics + CT12 recovery

- **Owned files:** `compiler/src/chips/control/ct11_diagnostic_commit.rs`; `compiler/src/chips/control/ct12_recovery_select.rs`.
- **Tests:** `compiler/tests/c45_control_recovery.rs`.
- **Reads:** failed task/diagnostic, phase facts, config/limits.
- **Writes:** none; decisions only (`DiagnosticDecision`, `RecoveryDecision`); commit applies.
- **Acceptance:** stable diagnostic order/dedupe; original diagnostic retained; one recovery enqueue/fail; bounded; no same-tick re-entry; `cargo test --manifest-path compiler/Cargo.toml --test c45_control_recovery`.
- **Freeze deps:** F7.

### B-CTL-07 — CT13 cancel + CT14 artifact finalize

- **Owned files:** `compiler/src/chips/control/ct13_cancel_job.rs`; `compiler/src/chips/control/ct14_artifact_finalize.rs`.
- **Tests:** `compiler/tests/c46_control_cancel.rs`.
- **Reads:** cancel pin, task states, artifacts fragments, completion facts.
- **Writes:** none; decisions only (`CancelDecision`, `HostRequestDraft`); driver/commit applies.
- **Acceptance:** cancel precedence/idempotence; ready/waiting disposition; no spurious artifact; missing fragments/failed tasks fail; no I/O; `cargo test --manifest-path compiler/Cargo.toml --test c46_control_cancel`.
- **Freeze deps:** F4/F7.

### B-CTL-08 — Driver integration, report, and recovery wiring

- **Owned files:** `compiler/src/routing.rs` (driver); proposed `compiler/src/driver.rs`/`compiler/src/report.rs` if the integrator accepts the relocation; `compiler/src/chips/control/mod.rs` (integrator-owned; not a chip owner edit).
- **Tests:** `compiler/tests/c47_control_driver.rs`; update `compiler/tests/c06_routing.rs`, `c09_readiness.rs`, `freeze.rs`.
- **Acceptance:** one tick record; ordered dispatch; in-flight empty at latch; no `Running`; H6 recovery; join drain; phase gate; cancel/budget; quota-1 projection; replay; `cargo test --manifest-path compiler/Cargo.toml --test c47_control_driver`.
- **Freeze deps:** F1–F8.

### Implementation sequencing

1. **Serial bootstrap:** F1 → F2 → F3 → F4 → F5 → F6 → F7 → F8. Do not parallelize shared-schema edits.
2. **Wave 1 control loop:** B-CTL-02 (select/guard), B-CTL-03 (validate/commit), B-CTL-04 (join/host), B-CTL-05 (phase/progress), B-CTL-07 (cancel/finalize), B-CTL-06 (diagnostics/recovery), B-CTL-01 (start/source), B-CTL-08 (integration).
3. **Integration:** B-CTL-08 after all others; run the targeted tests, then the existing `c03`/`c06`/`c09` regression subset; do not run the full suite in this Stage A task.
4. **Quota > 1:** separate, measured, post-freeze change; not part of these work orders.

## 10. Open questions

| ID | Question | Options | Recommendation | Owner |
|---|---|---|---|---|
| A16-OQ-01 | Selection key | (a) `(stage, group, enqueue, TaskId)`; (b) current `(group, enqueue, TaskId)` | (a) stage-first with group tie-break; freeze a new rule ID | T01 integrator |
| A16-OQ-02 | `SelectionBatchOverflow` carrier | (a) dispatcher error; (b) keep on `CommitError` | (a) dispatcher error; if (b), document the exception | T01 integrator |
| A16-OQ-03 | Control chips as services or task workers | (a) services with a decision capability; (b) 14 new task kinds; (c) pure driver functions | (a) service capability; (c) acceptable fallback | T01/T02 |
| A16-OQ-04 | Queue shape | (a) single `tasks.ready` at quota 1; (b) `stage_queues` canonical + derived `ready` | (a) for quota 1; (b) separate quota > 1 freeze | T01/T02 |
| A16-OQ-05 | `Stage`/`stage_of` mapping | (a) `stage_of` scheduling authority, `Stage` derived; (b) collapse `Stage` to 7; (c) map table | (a) with a fixed map table | T01 integrator |
| A16-OQ-06 | Continuation materialization | (a) defer, `None` only; (b) add `G1DraftBody::Continuation` + store/schema/allowlist/snapshot; (c) drop `ContinuationRef` | (a) for M1; (b) when T05 needs it | T01/T05 |
| A16-OQ-07 | `TaskDraft.continuation` | (a) keep `Option<ContinuationId>`; (b) migrate to `Option<ContinuationRef>` | (a) if continuations deferred; (b) only with materialization | T01/T05 |
| A16-OQ-08 | Progress + writes | (a) allow atomically; (b) prohibit (CT_FREEZE_02) | (a) for M1; (b) only with a new rule and fixture updates | T01/T02 |
| A16-OQ-09 | Empty proposal rule | (a) freeze current `TaskNotTransitioned`; (b) keep OB-2 open | (a); the implementation already has a rule | T01/T02 |
| A16-OQ-10 | Join dangling child | (a) typed fault + fail once; (b) stay `Waiting` (current) | (a); (b) is a silent-spin defect | T01/T05 |
| A16-OQ-11 | Join bound | (a) `tasks.waiting` index; (b) documented scan bound | (a) | T01/T02 |
| A16-OQ-12 | Cancel disposition | (a) ready stays `Ready`, job terminal; (b) add `TaskState::Cancelled`; (c) fail all | (a) for `/39` compatibility; (b) only by freeze | T01/T02 |
| A16-OQ-13 | `cancel_requested` lifecycle | (a) latched terminal; (b) one-shot/resumable | (a) terminal | T01/T02 |
| A16-OQ-14 | Result consumer identity | (a) parent-only protocol; (b) add `consumer: Option<TaskId>` | (a) for M1; (b) if a second consumer appears | T01 |
| A16-OQ-15 | `TickRecord` outcomes | (a) add `tick` + `outcomes`; (b) keep minimal | (a) if audit/quota > 1 requires; (b) for M1 | T01/T13 |
| A16-OQ-16 | `stage_queue_bound` enforcement | (a) global `max_queue_len`; (b) per-stage bounds with `stage_queues` | (a) at quota 1; (b) with stage queues | T01 |
| A16-OQ-17 | `max_proposals_per_tick` | (a) whole-batch sum; (b) per-dispatch cap; (c) both | (c) both with a checked sum; freeze the interaction | T01/T02 |
| A16-OQ-18 | `total_drafts` definition | (a) all draft bodies in the batch; (b) per-task; (c) per-stage | (a) whole `AppendRecords` bodies; freeze | T01 |
| A16-OQ-19 | Non-advancing progress | (a) rely on `max_task_progress`; (b) add a progress reason/state hash | (a) for M1; (b) later | T02/T13 |
| A16-OQ-20 | `in_flight` clear owner | (a) commit path after recovery; (b) driver latch; (c) end of propagate (current) | (b) at latch with an explicit driver assertion; (c) if documented as the latch step | T01 |
| A16-OQ-21 | CT11 dedupe | (a) commit dedupes; (b) CT11 dedupes before commit | (b) if CT11 exists; otherwise (a) | T02/T13 |
| A16-OQ-22 | CT12 recovery loop | (a) one recovery attempt per fault; (b) bounded retries | (a) + `max_task_progress`/`max_ticks` bound | T02 |
| A16-OQ-23 | Host wait resume | (a) CT02 response → `Ready` next tick; (b) CT08 re-request | (a); freeze request/response identity | T02/T03 |
| A16-OQ-24 | `phase_advance_requested` | (a) keep as advisory wire; (b) remove | (a) advisory; CT09 recomputes | T01/T02 |
| A16-OQ-25 | Quota > 1 report | (a) extend `TickRecord`; (b) first-task projection only | (a) before any quota > 1 claim | T01/T13 |

## 11. Rejected alternatives

| Alternative | Why rejected | Consequence if forced |
|---|---|---|
| Chips mutate the bus directly | Violates `AGENTS.md` §3.3/§3.7, Guardrails §2, and T02's decision-only envelope; breaks replay/audit | A full schema/contract rewrite; not acceptable |
| Add a new CT07 committed join carrier/family | T02 rev 30 selected the commit-apply invariant; a new family adds hash/schema/allowlist cost with no semantic need | Freeze F5 would be larger; reject |
| Consume child results at join | T02 rev 34 selected no-consume; the parent's own commit owns consumption; double-consume risk | `join_then_replay_retry` fails; reject |
| Fail-fast sibling policy | Await-all is the selected direction; fail-fast can lose sibling diagnostics and is not deterministic under interleaving | Reopen T02; reject |
| Clear `in_flight` at next tick start | Leaves a residual entry across latch; contradicts the no-residual invariant (T02 rev 33) | Latch assertion fails; reject |
| Reserve N diagnostics before dispatch | H6 selected no pre-reservation; per-task attempt with `NONE` fallback | Larger preflight; not selected |
| `DispatchBudgetExceeded` / `max_dispatches_per_tick` / `max_inflight_total` | Removed from the candidate; `max_inflight_per_tick` is the sole per-tick dispatch bound | Reintroduces duplicate bounds; reject |
| `ResultValue::DraftRecords` | Not in `/39`; the five-variant set is frozen and test-pinned; predicted refs are checked at commit | Breaks `c03_task.rs:898-919`; reject |
| A per-task `phase` field | Derived from kind/`stage_of`; a second writer cannot desync | Extra state with no reader; reject |
| A second writable `tasks.ready` or `stage_queues` projection | Two writable queues diverge; `tasks.ready` must be canonical or derived, never both | Split-brain scheduler; reject |
| `TaskState::Cancelled` for M1 | Not frozen; current tests pin ready-stays-`Ready`; a new state adds snapshot/hash/transition cost | Only with a freeze and fixture update; not selected now |
| `SelectionBatch`/`ControlDecision` as a committed record family | Decisions are wire-only; committing them would pollute the semantic store and snapshot | No semantic need; reject |
| Synthetic control task kinds to satisfy `NoTaskKinds` | CT03 selecting its own selection task is recursive; 14 kinds add registry/hash cost | Prefer a service capability or driver functions |
| Whole-bus manifest ("reads/writes the bus") | Violates T01 §5 and Guardrails §2.5 | Reject |
| Wall-clock/tick budget inside language semantics | T01 §4: wall-clock does not participate in language semantics | Reject |
| Same-tick re-entry of newly enqueued work | T01 §4 next-tick rule | Reject |
| Duplicate `awaited` in `ContinuationRecord` | WaitSet is the sole source; duplicate source causes divergence | Reject |
| `ContinuationRecord` generic 9-field list from CT_FREEZE_01 OPEN-09 | Incompatible with the frozen parse-oriented 9 fields | Reject or reopen with T05 |
| Join on a consumed child result | The join reads task state, not results; "already-consumed" is orthogonal | Clarify the draft; reject the wording |
| Treating the empty vector as success | Explicitly prohibited by `/39` and DoD; hides unimplemented work | Reject |
| Quota > 1 with concurrent writes/threads | T01/Guardrails: sequential single-writer only; parallel LLM development is not runtime parallelism | Reject; quota > 1 remains sequential |

## 12. Verification and non-claims

### 12.1 Checks performed for this report

- Read the required documents: `AGENTS.md`; `docs/tasks/README.md`; `T01_COMPILER_CONTRACT.md`; `T02_CONTROL_CHIPS.md`; `PARALLEL_EXECUTION.md`; `COMPILER_DEVELOPMENT_GUARDRAILS.md`; `TASK_TEMPLATE.md`; `MASS_PARALLEL_DISPATCH.md` §6/§9/§10; `CT_FREEZE_01..03`; `ADR-0002`; relevant `M1_PART_A_CONTRACT_PROPOSAL.md` sections.
- Read the source: `compiler/src/task.rs`, `bus.rs`, `commit.rs`, `routing.rs`, `manifest.rs`, `limits.rs`, `chips/mod.rs`, `snapshot.rs`, `contract.rs`, `records.rs`, `ids.rs`.
- Read the tests: `compiler/tests/c03_task.rs`, `c06_routing.rs`, `c09_readiness.rs`, `freeze.rs` (targeted sections).
- Cross-checked line references against the working tree; no source/test/contract file was modified.
- `git status --porcelain` was used only to confirm the report is a new untracked file and that existing user changes are untouched.
- At inspection time the workspace also contained pre-existing uncommitted parse/lex candidate files and freeze drafts (for example `compiler/src/chips/parse/mod.rs` modified, plus untracked `compiler/src/chips/parse/pa_*.rs` and `docs/tasks/CT_FREEZE_0*.md`). They are outside A16, were not treated as completed capability, and were not modified.

### 12.2 Checks not performed

- No `cargo test`, `cargo fmt`, `cargo clippy`, or contract validator was run. This is a documentation-only task; the Stage B work orders name the targeted commands but do not claim they pass.
- No probe, corpus, target execution, or pass-rate measurement.
- No schema/contract hash recomputation; no freeze; no commit/push.
- No implementation of any CT chip, queue, wire, or ledger.

### 12.3 What this report does not authorize

- It does not authorize any chip implementation, schema change, manifest row, routing row, test, Cargo change, `mod.rs` change, or shared-index change.
- It does not freeze `CT_FREEZE_01..03` or accept any of their recommendations.
- It does not claim quota > 1 correctness, throughput, or a pass rate.
- It does not replace the integrator's freeze authority or the T00 acceptance gate.

## Appendix A. Source evidence index

| Evidence | Path | Used for |
|---|---|---|
| Task envelope | `compiler/src/task.rs:1245-1263`, `:1546-1576`, `:1645-1674`, `:1676-1810`, `:1848-1990` | §2.1, §3 |
| Control/queue/report | `compiler/src/bus.rs:710-969`, `:992-1005`, `:1122-1210`, `:1236-1255` | §2.2, §3.5, §3.6 |
| Routing shell | `compiler/src/routing.rs:34-38`, `:81-269`, `:271-695` | §2.3, §6 |
| Commit | `compiler/src/commit.rs:95-313`, `:497-528`, `:530-1761`, `:1763-1972`, `:1974-2337`, `:2339-2528`, `:2817-2869` | §2.4, §2.5, §3.8–§3.11 |
| Manifest/stage | `compiler/src/manifest.rs:384-547`, `:784-985`, `:1444-1573` | §2.6, §3.14 |
| Limits | `compiler/src/limits.rs:13-343` | §3.13, §3.14 |
| Snapshot/trace | `compiler/src/snapshot.rs:532-599` (control/tasks), `:802-823` (in-flight/report), `:952-1070` (`TraceRecord`/`Trace`) | §2.2, §3.6 |
| Contract/freeze | `compiler/src/contract.rs:38-44`, `:46-241`, `:645-742`; `compiler/contracts/CONTRACT_VERSION`; `compiler/tests/freeze.rs` | Baseline, §8 |
| Tests | `compiler/tests/c03_task.rs:119-180`, `:400-465`, `:550-745`, `:829-962`, `:978-1285`; `c06_routing.rs:62-394`; `c09_readiness.rs:107-639`; `c07_limits.rs` (configured bounds) | §6 |
| T02 | `docs/tasks/T02_CONTROL_CHIPS.md` §1–§5 | Requirements, conflicts |
| CT freezes | `docs/tasks/CT_FREEZE_01.md`, `CT_FREEZE_02.md`, `CT_FREEZE_03.md` | §7 |
| ADR-0002 | `docs/architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md` §2.1–§2.7, §6 | Open items |
| M1 proposal | `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md` §6.2.1, §6.2.2, §6.3, §6.4, §10.5 | Proposed pipeline shapes |
| Dispatch plan | `docs/tasks/MASS_PARALLEL_DISPATCH.md` §6, §9, §10; `DISPATCH_LEDGER.md` | Work-order format, gates |

## Appendix B. Minimal-shape checklist for the integrator

Use this as the freeze checklist. A checked item means "the shape is frozen as stated"; it does not mean "implemented".

- [ ] Task: reuse `Task` exactly; no new field for control (`A16-T01`).
- [ ] TaskState: reuse five variants; no `Cancelled` for M1 unless separately frozen (`A16-X03`).
- [ ] Result: reuse five `ResultValue` variants; no `DraftRecords` (`A16-L01`, §11).
- [ ] Continuation: reuse 9 frozen fields; no generic 9-field replacement; materialization deferred or explicitly added (`A16-C03/C04`).
- [ ] WaitSet: reuse `children + host_request`; sole awaited-child source (`A16-C01`).
- [ ] Queue: `tasks.ready` canonical at quota 1; `in_flight` ephemeral; `waiting` index optional (`A16-Q01–Q04`).
- [ ] Ledger: `Task.state` + `ResultRecord` + `DiagnosticRecord`; `TickRecord` per tick; optional `outcomes` (`A16-L04`, `A16-L11`).
- [ ] Dispatch order: one key, published as a rule ID (`A16-R05`, F1).
- [ ] Commit order: one key, published as a rule ID (`A16-R06`, F1).
- [ ] Guard: 10 checks, decision-only (`A16-G02`).
- [ ] Validation: one authority, read-only, 12 checks (`A16-V02/V03`).
- [ ] Atomic commit: validate → preflight → infallible apply; no partial state (`A16-K01–K05`).
- [ ] Join: await-all, terminal-only, no consume, dangling fault (`A16-J01–J05`).
- [ ] Failure closure: H6 dispatch-order, diagnostic or `NONE`, state guard, no `Running`, in-flight empty (`A16-F01–F05`).
- [ ] Cancel: pre-selection, pre-budget, idempotent, one report, disposition frozen (`A16-X01–X05`).
- [ ] Progress: ordinal advance, count bound, zero disables, writes decision, count across waits (`A16-P01–P06`).
- [ ] Phase gate: no ready/waiting/in-flight; no compile; mapping frozen (`A16-H01–H06`).
- [ ] Recovery: CT12 decision-only, original diagnostic retained, bounded (`A16-F06`).
- [ ] Ownership: single writer per field; no chip mutation (`A16-R15`, §5).
- [ ] Replay: identical snapshot/report/order; no wall clock/hash-map (`A16-R14`).
