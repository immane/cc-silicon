# CT_FREEZE_03 — T02 CT10–CT14 Freeze Proposal (DRAFT ONLY, OPEN)

Status: **DRAFT ONLY — OPEN, NOT FROZEN, NOT ACCEPTED.**
No contract, schema, bound, hash, chip, interface, or test is frozen or authorized by this file.
No chip code is authorized. `/5` remains current.

Scope: T02 control-tail chips **CT10–CT14 only**:

- CT10 ProgressBudgetChip
- CT11 DiagnosticCommitChip
- CT12 RecoverySelectChip
- CT13 CancelJobChip
- CT14 ArtifactFinalizeChip

CT01–CT09, T01, T13, CDR, and any other package are **not** edited or decided by this file.
This file references their selected directions unchanged; it reopens none of them.
T01 integrator acceptance is required before any `/6` freeze.

## 1. Proposal boundary

1. Each chip is a zero-field decision/proposal-only `RestrictedChip`; no chip mutates tasks, queues, stores, diagnostics, artifacts, or `tasks.in_flight` directly.
2. All mutations run through the single designated commit/cancel path in deterministic order; chips emit typed proposals only.
3. `reset_wires` at tick start clears **only wires**; it is not an `in_flight`-lifecycle step.
4. Quota 1 (`max_inflight_per_tick = 1`) is the baseline equivalence mode under the canonical semantic comparison projection. Quota > 1 is post-`/6`, measured, integrator-accepted. No throughput claim is made here.

## 2. Batch (CT10–CT14 view)

1. CT10 checks progress/budget **over the dispatched batch**; no wall-clock.
2. A `Progress` outcome reschedules to next tick within `max_task_progress` and reinserts into the task's **own canonical `stage_queues[stage_of(task.kind)]`** (never the derived `tasks.ready`); the reinsert counts toward the `stage_queue_bound` pre-apply check (candidate; T01 decision pending).
3. CT11 proposes diagnostics as `diagnostics`-append proposals applied by the commit path in stable ordinal order with deduplication; no direct store write.
4. CT12 proposes an explicit recovery strategy by phase; it does not swallow the original diagnostic and does not itself mutate task state.
5. CT13 proposes cancel for the **in-flight set plus ready tasks** (see §5); the designated cancel path applies it.
6. CT14 is read-only over `artifacts.fragments` (plus completion facts) and emits a Host write **request proposal** via the request path; it performs no I/O and never appends `artifacts.fragments`. Missing fragments or incomplete/failed tasks must not report success.

## 3. Atomic commit (CT10–CT14 view)

1. One ordered atomic semantic batch commit per tick is the sole candidate mutation site (T01 decision pending). A failed semantic batch commit applies **no semantic state**; the bounded H6 recovery (rev-30 selected mechanism, to be frozen in `/6`) is the only post-failure mutation path.
2. Pre-dispatch errors (`SelectionBatchOverflow`, `DuplicateSelection` candidate set) occur before any state mutation and leave affected tasks `Ready`; they are structured tick diagnostics, not `CommitError`. `DispatchBudgetExceeded` / `InflightQuotaExceeded` are **not** in the operative candidate set.
3. `BackpressureCapacity` / proposal-budget rejections in the no-mutation pre-apply pass are semantic-batch-failure path: no partial queue/budget mutation leaks, and the H6 recovery (or T01-approved equivalent) runs so no dispatched task remains `Running`.
4. CT10 `Progress`, CT11 diagnostic appends, CT07 join reinserts, and new-task publishes all commit through this same ordered path and become visible next tick.
5. An empty proposal vector on the normal path requires an explicit named outcome rule (still unresolved, OB-2); it must never be silently treated as success and never leave the task `Running`. On the failed-batch path it is covered by the same bounded recovery like every other dispatched task.

## 4. `in_flight` ownership (candidate, T01 decision required)

1. `tasks.in_flight` is an **ephemeral per-tick scheduler batch only** (candidate): populated by the dispatcher pre-worker (`Ready→Running` + in-flight population, distinguished from the one ordered atomic semantic commit), observed during guard/worker/validate/commit/join/progress/cancel handling, and cleared **at the current latch only after** every dispatched task has a terminal/`Waiting`/`Progress` outcome or the H6 recovery.
2. The clear is **not itself a transition** and does not clear `TaskState`/`Running`.
3. Clearing at the **next tick start is rejected as latch-residual-incompatible**; a residual entry across the latch boundary violates the no-residual invariant.
4. The exact clear owner/order (clear at latch after recovery vs. clear inside recovery as final step) and the dispatcher-`Ready→Running`-vs-atomic-commit boundary are **critical T01 selections**; this file selects none.
5. Testable invariant (proposed): over every path (success, semantic batch failure, pre-dispatch error, empty proposal vector, cancel tick, tick-budget tick, join-only tick, `Progress`-only tick) assert **at latch, before `reset_wires`**: every dispatched task has exactly one outcome and no task is `Running`; `tasks.in_flight = ∅`; and a next-tick-start observer sees the same empty set.

## 5. Cancel (CT13 focus, OPEN)

1. Cancel is evaluated **once per tick before selection**; repeated cancel is idempotent with exactly one terminal report record.
2. A cancel tick performs **no new selection, dispatch, or commit**.
3. The ready/waiting task disposition under cancel, Host-wait wind-down with diagnostics/trace retention, and the **cancel-vs-tick-budget precedence** are **unresolved (T01/[INT], OB-3)**; this file freezes no disposition rule.
4. Cancel interaction with the latch post-condition: the cancel-tick path must still satisfy `tasks.in_flight = ∅` at latch (see §4.5); pre-dispatch-error/cancel/tick-budget affected tasks are **not** in the dispatched set and keep their state/`ready_tick` except as the frozen disposition rule (once selected) prescribes.
5. CT13 remains **decision-only**: it proposes the cancel decision for the in-flight set and ready tasks; the designated cancel path applies it, including any Host-wait wind-down. No direct task/queue mutation by the chip.

## 6. OPEN items for T01/T02 `/6` co-freeze

1. [OPEN] Exact `in_flight` clear owner/order (T01 critical decision).
2. [OPEN] Dispatcher `Ready→Running` vs. single ordered atomic commit boundary (T01 critical decision).
3. [OPEN] H6 recovery primitive realization and diagnostic-capacity mechanism (rev-30 mechanism selected for `/6`; exact atomic/bounded realization needs T01 approval).
4. [OPEN] Empty-proposal-vector normal-path outcome rule (OB-2).
5. [OPEN] Cancel disposition + cancel-vs-tick-budget precedence + tick-budget rule (OB-3; T01/[INT]).
6. [OPEN] Exact limit names/defaults/codes (`max_inflight_per_tick`, `stage_queue_bound`, `max_tasks_total`, `max_task_progress`), `StageId` set/ordinals, `total_drafts` definition, and error-code inventory.
7. [OPEN] T02/T13 batch fixtures and sign-offs (including cancel-tick, tick-budget-tick, join-only-tick, `Progress`-only-tick, `join_then_replay_retry`).

No item above is decided, frozen, or signed off by this file. A model/subagent note is not a human `[OWNER]`/`[INT]` response.
