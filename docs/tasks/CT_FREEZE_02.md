# CT Freeze 02 — CT05–CT09 Loop / Fixpoint / Budget / Oscillation / Progress Semantics

**Status: DRAFT ONLY — proposal, not a freeze, not an accepted contract, not owner/integrator-approved. No chip code, no interface, no schema, no test is authorized by this document. `/5` remains current; `/6` remains unfrozen.**

- **Scope:** CT05 ProposalValidate, CT06 TaskCommit, CT07 ResultResume, CT08 HostRequest, CT09 PhaseAdvance only. CT10 ProgressBudgetChip owns budget *detection*; this proposal describes only how CT05–CT09 *carry* Progress outcomes. CT01–CT04, CT10–CT14 are out of scope except as named interfaces.
- **Authority:** [T01_COMPILER_CONTRACT.md](T01_COMPILER_CONTRACT.md) (integrator-owned; C01–C06 frozen artifacts) and [T02_CONTROL_CHIPS.md](T02_CONTROL_CHIPS.md) (control/scheduling/diagnostic owner) govern. On any conflict, T01 + T02 control. This file decides nothing; every normative sentence below requires T01/T02 `/6` co-freeze.
- **Conventions:** `OPEN-02-NN` = open question owned by T01/T02 co-freeze. `H6`/`H9` cross-references point at the T02-owned checklist (rev 30–35), not restated as resolved. "Must" below means "the candidate must, if selected" — not "the implementation must today".

## 1. Definitions (candidate)

1. **Task outcome** (per dispatched task, per tick): exactly one of `Complete` / `Fail` / `AwaitHost` / `AwaitChildren` / `Progress`. `Running` is never a latch outcome. (T01 C03 five-outcome set; T02 H6 checklist.)
2. **Loop**: any multi-tick path where a task returns without terminal state and is re-selected: `Progress` reinsert, `AwaitHost` resume (CT08 → CT02), `AwaitChildren` join (CT07). Single-tick same-task re-entry is forbidden (T01 §4: newly enqueued work is next-tick visible).
3. **Fixpoint**: the scheduler predicate under which a stage/phase is quiescent: no `Ready` task in any canonical `stage_queues[stage]` and no `Waiting` task whose resume condition is committable this tick. `tasks.ready` is a derived view, never the authority (T02 CT09 row).
4. **Budget**: any finite bound that forces a non-terminal loop to terminate: `max_task_progress` (per-task Progress count), `max_proposals_per_tick` (per-batch proposal volume), `max_inflight_per_tick` (dispatch count), `stage_queue_bound` (reinsert capacity), diagnostic/record capacity. No wall-clock participates (T02 CT10 row).
5. **Oscillation**: a non-converging loop the fixpoint predicate never reaches: Progress↔re-dispatch without state advance, join↔re-wait without child progress, phase-advance denied every tick while work exists, or two tasks re-enqueueing each other. The candidate must make every loop either advance or exhaust a named budget.
6. **Progress**: the `Progress` outcome — "not done, not blocked, reschedule me next tick". It carries no semantic store write, reinserts the task into its **own** canonical `stage_queues[stage_of(task.kind)]` with `ready_tick = tick+1` and a persisted progress count, and is selectable next tick. Applied only by the commit path, never by a chip body.

## 2. Per-chip candidate semantics (CT05–CT09)

### CT05 ProposalValidate — loop/budget-carrying validation (decision only, no mutation)

- CT05 validates each task's outcome tag and its budget-carrying fields: `Progress { count, reason }` shape well-formedness, `AwaitChildren` child-ref scope (committed IDs or same-task `OwnBatch` transient keys only; no cross-task drafts, no wire/address refs — T02 rev 26 A/B, T01 confirmation pending), `AwaitHost` request-draft well-formedness, and the `(StoreId, field, record)` cross-task write-conflict predicate over the ordered batch.
- A `Progress` proposal **must not** carry a `StorePatch`/`AppendRecords` payload in the same task's proposal vector. Mixed Progress+write vectors are rejected (candidate rule; error family open — see OPEN-02-01).
- CT05 does **not** decide whether the progress count is exceeded; it checks shape only. Exceedance (`count + 1 > max_task_progress`) is enforced at commit pre-apply (CT06), so validation stays order-independent.
- A dispatched task with an empty proposal vector (`proposals == []`) is **not** silently `Completed`. CT05 flags it; the explicit outcome rule is unresolved (T02 H6.5 / OB-2 — see OPEN-02-02).

### CT06 TaskCommit — the single ordered atomic commit; Progress reinsert; budget enforcement

- CT06 *proposes* the whole-batch commit in `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal index)` order; the commit path/backend applies **one ordered atomic commit** per tick (validate → capacity preflight → infallible apply). New tasks / reinserts are visible next tick only.
- **Budget pre-apply order (candidate, no-mutation pass before any apply):** (a) per-task Progress exceedance check (`count+1 > max_task_progress` → that task's outcome becomes `Fail` with a named diagnostic, not silent drop); (b) whole-batch proposal budget (`checked_add(proposals.len(), total_drafts) <= max_proposals_per_tick`; `total_drafts` definition open); (c) per-stage reinsert capacity (`queue_after[stage] = stage_queues[stage].len() + new_tasks + progress_reinserts + join_reinserts`, checked arithmetic, `BackpressureCapacity` on exceed). Any rejection commits **no semantic state**; the H6 bounded recovery (rev-30 selected mechanism, T01 realization pending) then runs so no dispatched task remains `Running`.
- **Progress reinsert (candidate):** a non-exceeded `Progress` outcome reinserts exactly one entry for that task into its own `stage_queues[stage_of(task.kind)]` with `ready_tick = tick+1` and persisted `count+1`. Never into the derived `tasks.ready`. Exactly-once per dispatch; a second reinsert for the same task in the same batch is a batch-construction fault.
- **Progress vs semantic writes:** a tick whose batch contains a `Progress` outcome for task T commits no semantic-store mutation *from T*; other tasks' validated writes still commit atomically as one batch. (Contrast: a *failed batch* commits nothing from anyone and activates recovery.)

### CT07 ResultResume — join fixpoint (decision only; commit-apply realizes)

- CT07 computes the join decision over **committed terminal state only**; it consumes no child result (T02 rev 34 OPEN-02 direction). Sibling policy candidate: **await-all** (rev-30 selected direction, co-freeze pending): all committed children terminal before any parent decision; all `Completed` → parent `Ready` (+ one own-stage reinsert, next tick); any `Failed` → parent `Failed` exactly once, never `Ready`.
- **Join fixpoint contribution:** a `Waiting` parent is in no stage queue and is not dispatched. The parent rejoins the fixpoint predicate only via its own reinsert (`Ready`) or terminal outcome (`Failed`). Join evaluation must be bounded (no unbounded scan over children/queues); the bound mechanism is open (see OPEN-02-04).
- `WaitSet` is the sole awaited-child-ID source; same-task `OwnBatch` keys are transient wire/proposal inputs resolved to committed IDs before persistent state (T02 rev 26; T01 confirmation pending). A join that would wait on a non-committed, cross-task, or already-consumed child is rejected, never parked silently.
- Result consumption stays with the parent's own next-tick atomic commit (`ResultAlreadyConsumed` guarded) or, only if co-freeze requires, transfers at join into explicit parent-owned durable state. Replay between join and parent execution must neither double-consume nor lose results (`join_then_replay_retry` fixture required, T13 H6-M15).

### CT08 HostRequest — parking a loop without spinning it

- CT08 forms a persistent `HostRequestDraft` proposal (no I/O, no direct store write); the commit path persists the request and the task becomes `Waiting(host_request)`. A `Waiting`-on-host task is **not** in any stage queue, is **not** dispatched, and consumes **no** Progress count while parked.
- Legitimate host waits must never be misjudged as non-advancing loops: Progress-budget accounting pauses while `Waiting(host_request)`; resume re-enters with its prior Progress count preserved (candidate; see OPEN-02-05 for the resume-count rule).
- Duplicate/out-of-order host responses are deduplicated by request ID (CT02 import side); a response that arrives for a cancelled or already-failed waiter must not resurrect it (cancel precedence open — T13 H6-M13/M14, see OPEN-02-06).

### CT09 PhaseAdvance — the phase-level fixpoint gate (observation only, no compilation)

- CT09 advances the phase only when **all** canonical `stage_queues` are empty of `Ready` **and** no `Waiting` task remains resumable (join children all terminal with the join committable, or host response committed). It never advances while a `Ready` or resumable-`Waiting` task remains in any stage, and it never performs compilation work itself.
- **Denied-advance is not failure:** if the fixpoint predicate is false, CT09 proposes `Wait` (stay in phase); the tick still latches normally. A phase that is denied every tick while work exists is by definition *not oscillating* — oscillation requires the fixpoint predicate to flap (see §4).
- Phase advance reads completion facts (committed terminal states + empty queues), never wires, drafts, or scheduler-batch (`in_flight`) state.

## 3. Budget taxonomy (candidate; names/defaults/codes NOT frozen)

| Budget | Owner that enforces | Candidate exceedance effect | Notes |
|---|---|---|---|
| `max_task_progress` | CT06 pre-apply (CT10 detects) | task outcome becomes `Fail` + named diagnostic | per-task persisted count; pause while `Waiting` (candidate) |
| `max_proposals_per_tick` | CT06 pre-apply | whole-batch reject-before-apply → H6 recovery | per-dispatch vs whole-batch interaction open (T02 H9.3) |
| `max_inflight_per_tick` (quota) | dispatcher pre-dispatch | `SelectionBatchOverflow`, tasks stay `Ready` | sole per-tick dispatch-count bound (rev-31 default) |
| `stage_queue_bound` | CT06 pre-apply | `BackpressureCapacity`, whole-batch reject → H6 recovery | includes progress+join reinserts in the checked sum (T02 H9.2) |
| diagnostic/record capacity | commit path per-task attempt | `DiagnosticId::NONE` sentinel fallback, still exactly-once `Failed` | no pre-reservation (rev-30 H6 mechanism) |

All arithmetic is checked/saturating; overflow never panics and never wraps silently. Exact limit names, defaults (other than quota default 1), and error-code spellings are **not** invented here — T01/T02 `/6` co-freeze.

## 4. Oscillation guards (candidate invariants, not proven)

- **O1 — Progress must advance or exhaust.** A task that returns `Progress` on consecutive ticks must either change its observable continuation state (cursor/operands/scope advanced) or exhaust `max_task_progress` and `Fail`. A task returning `Progress` with byte-identical continuation state more than once is a candidate *non-advancing-loop* signal; the exact detection chip (CT10) and the fail-vs-keep-going policy are out of scope here — CT05–CT09 only guarantee the count is carried honestly and the budget is enforced (see OPEN-02-03).
- **O2 — No Progress↔Wait flap.** A task must not alternate `Progress` / `AwaitChildren` (or `AwaitHost`) across ticks without an intervening child/host state change. CT07/CT08 decisions are functions of committed state; identical committed state must yield an identical decision (determinism), so flapping implies either committed-state change (legitimate) or a determinism bug.
- **O3 — Fixpoint monotonicity per phase.** Within a phase, the set of unfinished tasks (non-terminal + waiting-resumable) must not grow without bound: enqueues are bounded by `stage_queue_bound` + `max_tasks_total`; reinserts are bounded by surviving-task counts. Unbounded task creation without consumption is rejected at pre-apply, never latched.
- **O4 — Phase gate stability.** CT09's advance/deny decision is a pure function of committed completion facts + canonical queue emptiness at latch. The same latch state must yield the same gate decision on replay. A phase that advances and then regresses without new enqueued work is a protocol fault.

## 5. Latch post-conditions involving loops (candidate; assert at latch, before `reset_wires`)

- Every dispatched task has exactly one outcome in `{Completed, Failed, Waiting, Ready}`; `Ready` via `Progress` has `ready_tick = tick+1` and exactly one own-stage queue entry; no dispatched task is `Running`; `tasks.in_flight = ∅`.
- A `Waiting` task (host or children) is in no stage queue. A `Progress` task is in exactly one (its own). A terminal task is in none.
- Progress counts are persisted per task and survive replay identically (scheduler snapshot deterministic; canonical semantic projection excludes scheduler-only registers — T02 quota-1 equivalence note).

## 6. Proposed fixture list (none implemented; T13 owns verification)

1. `progress_advances_then_completes` — N× `Progress` with advancing cursor, then `Complete`; asserts count persistence + own-stage reinsert each tick.
2. `progress_exhausts_to_fail` — `Progress` past `max_task_progress`; asserts exactly-once `Fail` + named diagnostic, never silent `Completed`, never `Running`.
3. `progress_does_not_carry_writes` — mixed Progress+write vector rejected at CT05/commit pre-apply; asserts no partial store mutation.
4. `empty_proposal_has_explicit_rule` — dispatched `proposals == []`; asserts the co-frozen explicit outcome (rule per OPEN-02-02), never silent success.
5. `host_wait_parks_progress_count` — `AwaitHost` → response → resume; asserts count preserved across the park and no budget consumed while `Waiting`.
6. `join_fixpoint_all_complete_vs_any_failed` — await-all both branches; asserts `Ready`+reinsert vs exactly-once `Failed`-never-`Ready`.
7. `join_then_replay_retry` — replay between join and parent commit; asserts no double-consume, no loss (T13 H6-M15).
8. `phase_gate_denies_then_advances` — work remaining → deny; all done → advance; asserts gate purity on replay.
9. `oscillation_flap_rejected` — Progress↔Await alternation without committed-state change; asserts deterministic identical decision (no flap).

## 7. OPEN items (all unresolved; nothing below is decided)

- **OPEN-02-01 — Mixed Progress+write vector error family.** Which error (validation reject vs commit `CommitError` vs protocol fault), and does it fail the task or the whole batch? *Owner: T01/T02 co-freeze.*
- **OPEN-02-02 — Empty-proposal outcome rule (OB-2).** The explicit outcome for a dispatched `proposals == []` task on the normal path is unresolved; the candidate outcome set is not shown reachable from an empty vector. *Owner: T01/T02 co-freeze.*
- **OPEN-02-03 — Non-advancing-loop detection policy.** Where the Progress-with-identical-continuation signal is detected (CT10 candidate), its threshold, and whether it fails fast or keeps rescheduling within budget. CT05–CT09 carry the count; they do not judge advancement. *Owner: T02 (CT10 role) + T01.*
- **OPEN-02-04 — Join scan bound.** The explicit bound that keeps CT07/commit-apply join evaluation bounded (per-parent child-count cap? per-tick join-attempt cap?) and its error. *Owner: T01/T02/T05 co-freeze.*
- **OPEN-02-05 — Progress-count lifecycle across waits.** Whether `AwaitHost`/`AwaitChildren` preserve, pause, or reset the per-task Progress count on resume, and where the count is stored (task record vs continuation). *Owner: T01/T02 co-freeze.*
- **OPEN-02-06 — Cancel vs tick-budget vs join precedence.** Ready/waiting-task disposition on cancel ticks, cancel-vs-budget precedence, repeated-cancel idempotence, and the single terminal report record. (T13 H6-M13/M14.) *Owner: T01/[INT].*
- **OPEN-02-07 — `total_drafts` definition.** Exact scope of the whole-batch commit-budget sum (`proposals.len() + total_drafts`) and its interaction with quota Q (per-dispatch cap vs summed batch cap; ADR-0002 §6 open item). *Owner: T01/T02 co-freeze.*
- **OPEN-02-08 — Numeric inventory.** Exact limit names/defaults/codes (`max_task_progress`, `stage_queue_bound`, `max_proposals_per_tick`, `max_tasks_total`) and `StageId` set/ordinals. Nothing invented here. *Owner: T01/T02 `/6` co-freeze.*
- **OPEN-02-09 — Dispatcher `Ready→Running` vs atomic-commit boundary; `in_flight` clear owner/order; no-residual proof.** The four T01 architecture-critical items (T02 checklist items 1–4) apply unchanged to every loop path above; this proposal selects none. *Owner: T01 (T02 lists alternatives only).*

## 8. Non-goals / not claimed

- No chip implementation, interface change, schema change, hash-rule change, or test is authorized or claimed.
- No throughput claim for quota > 1; quota 1 remains the baseline equivalence mode (T02 §quota policy).
- No wall-clock, no threads, no concurrent bus writes; sequential single-writer execution only.
- More than 99% / torture-gate claims are out of scope ([T00_GCC_TORTURE_GATE.md](T00_GCC_TORTURE_GATE.md) gate rules apply separately).
