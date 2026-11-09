# CT Freeze 01 (DRAFT ONLY): T02 CT01–CT04 Control-Flow Freeze Proposal

Status: **DRAFT ONLY — no code, no freeze, no sign-off.** New file only; no other file edited, nothing committed, no chip implemented.

Scope: freeze proposal for **CT01–CT04 only** (T02_CONTROL_CHIPS.md rows CT01 JobStart, CT02 SourceResponse, CT03 TaskSelect, CT04 TaskGuard). CT05–CT14 are out of scope except where their interfaces bound CT01–CT04 (CT06 commit path, CT07 join/WaitSet, CT09 phase advance).

References:

- [T02_CONTROL_CHIPS.md](T02_CONTROL_CHIPS.md) — CT01–CT04 rows, fixed topology (TaskSelect → TaskGuard → worker → ProposalValidate → TaskCommit → ResultResume/PhaseAdvance), decision-only CT03, per-dispatch CT04 guard, H6/H9 rev 30–35 notes.
- [TASK_TEMPLATE.md](TASK_TEMPLATE.md) — per-chip header binding (ID/task kind/payload fields/result tag, reads/writes, preconditions, transition states, fixtures).
- `/6` foundation: [T01_COMPILER_CONTRACT.md](T01_COMPILER_CONTRACT.md) §4 task/completion protocol + §7.1 `t01-c01-c06/6` amendment (five-outcome set `AppendRecords`/`Progress`/`AwaitChildren`, per-task empty-proposal `Fail`, bounded dispatch-order recovery, await-all join without consumption, Progress reinsert with count/ordinal, 9-field continuation; formal T01 §4 supersession entries catalogued, **not signed**).

Conventions: every undecided item is marked **OPEN** with numbered options plus a recommendation. "Candidate" = proposed text for co-freeze, not frozen. Nothing here authorizes implementation.

## 1. Task-spawn / join payload typing (CT01, CT02, CT04 scope; join delivery itself is CT07/T01-owned)

Frozen `/5` baseline (T01 §4): `Task { id, kind, payload, owner, parent, continuation, state }`; `Proposal = Enqueue | Complete | Fail | AwaitHost | StorePatch`. `/6` candidate adds `AppendRecords` / `Progress` / `AwaitChildren` outcomes (catalogued, unsigned).

- **OPEN-01 — spawn payload type for CT01's first source task and CT02's import.** Options: (a) opaque `RecordRef` payload resolved by C01 reserved-ID protocol; (b) typed per-kind struct (e.g. `StartJob { config: ConfigId }`, `ImportSource { request: HostRequestId, bytes_hash }`) carried in the `TaskKind` payload enum. Recommendation: (b) — TASK_TEMPLATE §1 requires explicit payload fields per chip; opaque refs push validation into CT04 guard instead of the type registry.
- **OPEN-02 — `Enqueue` child-spec typing (parent → child spawn).** Options: (a) `Enqueue { kind, payload, parent: TaskId, continuation: ContinuationRef }` with `ContinuationRef = Committed(ContinuationId) | OwnBatch(DraftRef)` (M1-proposal direction, transient wire-only per Guardrails §6.1 narrow interpretation, user decisions A/B in principle); (b) committed-IDs-only `Enqueue` (no `OwnBatch` anywhere, even transient). Recommendation: (a) — matches T02 rev 26/30 accepted-in-principle direction; (b) stays as fallback if T01 rejects transient drafts.
- **OPEN-03 — join input typing (CT04 guard view of `AwaitChildren`).** CT07 join is a commit-apply invariant with no new carrier (T02 rev 30 settled); CT04 only guards the `AwaitChildren { task, children: Vec<ChildRef> }` proposal shape with `ChildRef = Committed(TaskId) | OwnBatch(u32)` same-task-only. Options: (a) CT04 validates `ChildRef` shape/range only, commit resolves; (b) CT04 fully resolves `OwnBatch` → committed IDs. Recommendation: (a) — resolution is a commit-apply (CT06 path) responsibility; CT04 stays decision-only per topology.
- **OPEN-04 — result-delivery carrier at join.** T02 rev 34 direction: join consumes no child result; consumption stays with the parent's own atomic commit or transfers to parent-owned durable state; fixture `join_then_replay_retry` required. The exact carrier/envelope is T01/T05/T02 `/6` co-freeze — recorded here as a boundary, not decided.

## 2. WaitSet semantics (CT04 guard + CT07 boundary)

Baseline: `TaskState = Ready | Running | Waiting(child/request_ids) | Completed(result_id) | Failed(diag_id)` (T01 §4); `/6` candidate: `Waiting(WaitSet { children, host_request })` is the **sole** awaited-child-ID source, `ContinuationRecord` carries no duplicate `awaited` (formal `/6` amendment only, T01+T05 acceptance pending).

- **OPEN-05 — `WaitSet` exact shape.** Candidate: `WaitSet { children: Vec<TaskId>, host_request: Option<HostRequestId> }`, committed IDs only (no drafts/wires/addresses persist). Options: (a) freeze as above; (b) add `continuation: Option<ContinuationId>` pointer into `WaitSet`. Recommendation: (a) — `Task.continuation` already links the record; duplicating it in `WaitSet` reintroduces the dual-source bug the `/6` amendment removes.
- **OPEN-06 — who writes `WaitSet`.** Options: (a) commit-apply only, from validated `AwaitChildren`/`AwaitHost` proposals (CT07/CT08 decision-only); (b) CT07/CT08 chips write directly. Recommendation: (a) — matches "chips propose, commit path mutates" (T02 package envelope) and the rev-30 CT07 invariant.
- **OPEN-07 — await-all vs await-any.** Settled direction (T02 rev 30, CDR rev 42): **await-all** — parent resumes only when all committed children terminal; all `Completed` → `Ready`; any `Failed` → parent `Failed` once, never `Ready`. Remaining open: CT07/T05 fixtures + sign-offs (noted, not reopened).
- **OPEN-08 — `Waiting` + `stage_queues` interaction.** Direction (T02 rev 21+): a `Waiting` parent is in no stage queue; on resume the commit path reinserts into `stage_queues[stage_of(parent.kind)]` (parent kind, H2; absent continuation handled). Options for the reinsertion tick: (a) selectable next tick (default new-task rule); (b) same-tick re-dispatch. Recommendation: (a) — preserves "new tasks visible next tick" (T01 §4).

## 3. Continuation fields (CT01–CT04 spawn/resume view)

Baseline (T01 §4): "a continuation must record the production state, cursor, operands, scope, and awaited child IDs." `/6` candidate: **9-field continuation** with `WaitSet`-only awaited children (i.e. no duplicate `awaited` field in the record); formal supersession catalogued, unsigned. The exact 9-field inventory is **not enumerated in the frozen contract** — the list below is a candidate reconstruction for co-freeze, not a claim.

- **OPEN-09 — field inventory.** Candidate 9 fields: `production`, `cursor`, `operands`, `scope`, `frame`, `next_child_ordinal`, `progress_count`, `stage_hint`, `resume_kind`. (Sources: T01 §4 sentence for the first four; M1-proposal `ordinal`/`Progress`-count/`stage_of(parent.kind)` needs for the rest.) Options: (a) freeze these 9 with stated types; (b) freeze a smaller record and keep the rest in task-owned frames. Recommendation: (a) with T05 (parser, heaviest continuation user per TASK_TEMPLATE Example B) confirming field 5–9 necessity before freeze.
- **OPEN-10 — `Task.continuation: Option<ContinuationId>` vs inline record.** M1-proposal direction (rev 21): task points at an optional committed continuation; successor spawn uses transient `ContinuationRef::OwnBatch`. Options: (a) pointer form; (b) inline record in `Task`. Recommendation: (a) — pointer form is what makes the transient-`OwnBatch` resolution meaningful; inline forces duplication on every spawn.
- **OPEN-11 — absent-continuation handling.** Direction (T02 H2): realization handles an absent `continuation`. Options: (a) `None` = fresh start, always legal; (b) `None` legal only for spawn kinds listed in the kind registry. Recommendation: (b) — (a) lets a resume-without-state silently restart; registry-gating is one checked lookup in CT04.
- **OPEN-12 — `ContinuationDraft` materialization path.** M1-proposal direction: `AppendRecords { task, batch: [ContinuationDraft] }` validated pre-apply, materialized at commit phase 4. Options: (a) continuations materialize only via `AppendRecords`; (b) also via `StorePatch`. Recommendation: (a) — continuations are new records (append), not patches to existing ones.

## 4. Phase encoding (CT01/CT03/CT09 boundary)

Baseline: workers are `phase=propagation` (T01 §5, TASK_TEMPLATE §1 header); CT09 advances phase only when all canonical `stage_queues` are empty/waiting (T02 CT09 row). Dispatch order candidate: `(stage ordinal, phase priority, enqueue ordinal, TaskId)` (T02 batch bullet); `/6` adds `STAGE_ASSIGNMENT` per-kind enforcement + stage/layer agreement (T01 §7.1 `/7` note as precedent pattern).

- **OPEN-13 — phase value set.** Options: (a) numeric priority per stage (`STAGE_ASSIGNMENT` ordinal doubles as phase priority); (b) separate `Phase` enum (`Source < Lex < Parse < SymbolType < … < Target`) independent of `StageId`. Recommendation: (a) — one ordering source avoids stage-vs-phase skew; CT09 gating reads the same `stage_queues` it already checks.
- **OPEN-14 — where phase lives.** Options: (a) derived from task kind via registry (`stage_of(kind)`), no per-task phase field; (b) stored per-task `phase` field set at spawn. Recommendation: (a) — derived data cannot desync; (b) adds a second writer (spawner vs registry) with no reader benefit.
- **OPEN-15 — CT03 selection key.** Candidate: `(stage ordinal, phase priority, enqueue ordinal, TaskId)`; under OPEN-13 option (a) the first two keys collapse to one. Options: (a) collapse; (b) keep both keys distinct for future phase-within-stage use. Recommendation: (a) — freeze the minimal key; reintroduce a separate phase key only with a demonstrated same-stage multi-phase case.
- **OPEN-16 — CT09 vs CT03 `Done` signal.** CT03 emits Wait/Done when no task is ready (decision only); CT09 advances the phase. Options: (a) `Done` is advisory, CT09 recomputes from `stage_queues`; (b) `Done` binds CT09. Recommendation: (a) — single authority (canonical queues) avoids a stale-wire advance.

## Open-item index

| ID | Topic | Recommendation |
|---|---|---|
| OPEN-01 | Spawn payload typing (CT01/CT02) | Typed per-kind structs |
| OPEN-02 | `Enqueue` child spec + transient `OwnBatch` | (a) transient wire-only `OwnBatch`, fallback (b) |
| OPEN-03 | CT04 `AwaitChildren` guard depth | Shape-only; commit resolves |
| OPEN-04 | Join result-delivery carrier | T01/T05/T02 co-freeze (boundary) |
| OPEN-05 | `WaitSet` shape | `children + host_request`, no continuation dup |
| OPEN-06 | `WaitSet` writer | Commit-apply only |
| OPEN-07 | Sibling policy | await-all (settled direction; fixtures pending) |
| OPEN-08 | `Waiting` reinsertion tick | Next tick |
| OPEN-09 | 9-field inventory | Candidate list; T05 confirms 5–9 |
| OPEN-10 | Continuation pointer vs inline | `Option<ContinuationId>` pointer |
| OPEN-11 | Absent continuation | Registry-gated `None` |
| OPEN-12 | `ContinuationDraft` path | `AppendRecords`-only |
| OPEN-13 | Phase value set | Stage ordinal as priority |
| OPEN-14 | Phase storage | Derived from kind registry |
| OPEN-15 | CT03 selection key | Collapsed minimal key |
| OPEN-16 | CT09 `Done` authority | Advisory; queues decide |

## Non-goals / explicitly not decided here

- CT05–CT14 internals; H6 recovery mechanism internals (selected elsewhere, rev 30); numeric limit defaults (`max_inflight_per_tick`, `stage_queue_bound`, `max_tasks_total`, `max_task_progress`); exact `StageId` set/names/ordinals; error-code inventory; hash-scope reconciliation (C05 open item). All remain with their owners (T01 integrator primary).
