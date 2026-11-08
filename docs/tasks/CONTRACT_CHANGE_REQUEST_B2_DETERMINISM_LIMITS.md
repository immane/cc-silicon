# CONTRACT_CHANGE_REQUEST — B2-00: Determinism, Limits, and Replay Closure (D-12..D-15)

| Field | Value |
|---|---|
| **ID** | `B2-00` |
| **Title** | Determinism/limits/replay cluster D-12..D-15: append snapshot bodies, scheduler order, limit accounting, store versions, replay evidence |
| **Date** | Submitted 2026-11-07; accepted 2026-11-09 |
| **Author** | B2-00 subagent (draft prepared for the T01 integrator and the affected owners; acceptance recorded by the T01 integrator) |
| **Status** | **ACCEPTED 2026-11-09 — mandatory package frozen as `t01-c01-c06/41`; deferred restore package is `/43`.** `/40` was allocated to B2-01 (M1 A3 closure). The original draft text below is preserved; the version remap and accepted decisions are recorded immediately after the version note. |
| **Baseline contract** | `t01-c01-c06/40` — `c6961ac65fb3fb7e4ad4fdb666c47584b94042949cb6ed17111a4d81b3b10c14` (`compiler/contracts/CONTRACT_VERSION`) |
| **Proposed artifact** | `t01-c01-c06/41` for the mandatory minimal package; `t01-c01-c06/43` for the deferred replay/restore package (after B2-02 `/42` tagged constants). The exact hash is recomputed at integration. |
| **Baseline revision** | `wave4-development` / `58428ac` |
| **Scope** | Compiler application foundation (`compiler/`, crate `cc-silicon-compiler`). Root `cc-silicon` framework, SFL core, target values, and chip logic are unchanged. |
| **Related** | [T01](T01_COMPILER_CONTRACT.md) §§3–4, §7.1 C01/C05; [M1 target acceptance](M1_TARGET_ACCEPTANCE.md) D1–D8; [M1 proposal](M1_PART_A_CONTRACT_PROPOSAL.md) §6.2.1/§10.5; [A23](../reviews/dispatch/A23_DETERMINISM_REVIEW.md); [A02](../reviews/dispatch/A02_CONTRACT_FOUNDATION.md); [A16](../reviews/dispatch/A16_CONTROL_PROTOCOL_REQUIREMENTS.md); [A17](../reviews/dispatch/A17_VERIFICATION_REQUIREMENTS.md); [Guardrails](COMPILER_DEVELOPMENT_GUARDRAILS.md) §3; [Dispatch Ledger](DISPATCH_LEDGER.md) §3A/§3C |
| **Write scope of this task** | This file only. |

**Non-claims.** This is a decision request, not an accepted contract, not an ADR, and not a
freeze. It does not authorize any bus/schema edit or any implementation. No test,
probe, corpus run, target run, or pass rate was produced by this task. The only
test evidence cited is the pre-existing evidence recorded by the cited reviews
and the dispatch ledger; it was not re-run. The compiler remains incomplete and
no target/ABI/torture claim is made.

**Version note.** The Guardrails §3.4 template still names `t01-c01-c06/5` as
"today". The current artifact at acceptance is `t01-c01-c06/40`
(`CONTRACT_VERSION`). This CCR originally proposed `/40`/`/41`; because B2-01
took `/40` for the M1 A3 closure, the mandatory B2-00 package is now `/41` and
the deferred restore package is `/43` (B2-02 tagged constants owns `/42`).

## Accepted version remap and decisions (T01 integrator, 2026-11-09)

| Original draft label | Accepted label |
|---|---|
| Mandatory package `/40` | **`/41`** |
| Deferred restore/projection package `/41` | **`/43`** |
| `B2-40-01..10` inventory items | **`B2-41-01..10`** (mandatory) |
| `B2-41-01..05` inventory items | **`B2-43-01..05`** (deferred) |

Accepted defaults for the open decisions (all implementation now targets `/41`):

| ID | Accepted decision |
|---|---|
| OD-1 | Stage-first selection key `(stage, group/phase, enqueue ordinal, TaskId)`; `STAGE_ASSIGNMENT` is the scheduler authority; T01 §4 is amended. |
| OD-2 | Full canonical append handles and bodies in the snapshot; names for family/body variants; stored order preserved. |
| OD-3 | `stage_queue_bound` is a validated, hashed, snapshotted **reserved** bound in `/41`; per-stage enforcement is deferred to `/43`. |
| OD-4 | Remove the zero-quota clamp; `propagate` returns `CommitError::Limit(LimitError::InvalidInflightQuota { value: 0 })` with no dispatch and no commit. |
| OD-5 | `ResultRecord.version` is a 1-based global commit ordinal stored in `Control.next_commit_ordinal`; failed commits do not advance it. |
| OD-6 | `/41` replay is **recorded-input re-execution**, not bus restoration; full restore is `/43`. |
| OD-7 | `TraceRecord` carries a canonical `pins_digest`; the exact pins stay in the Host replay envelope. |
| OD-8 | Encode the redundant `Task.id` field in the snapshot and test divergence; do not remove the field. |
| OD-9 | One mandatory `/41` package for all D-12..D-15 minimal items; restore/projection stays `/43`. |
| OD-10 | D8 quota>1 commit order stays out of `/41`; it is `/43` work. |

Acceptance is a decision record only; implementation and test evidence are
recorded by the `/41` integration run. The original draft's `/40` wording below
is preserved as history and must be read through this remap.

---

## 0. Required Guardrails §3.4 fields — location index

| Required field | Where addressed |
|---|---|
| ID/title, date, author, target contract version/hash | Header table |
| Concrete requirement/motivation and smallest sufficient change | §1, §2, §3–§6 |
| Exact current vs proposed shape of every affected shared interface, store, field, enum, schema, and file | §3.2, §4.3, §5.1–§5.5, §6.2–§6.4 |
| Affected chips, task kinds, manifests, dependent tests to rerun | §7 |
| Read/write manifest, phase, and ownership impact | §8 |
| Determinism/replay and snapshot/hash impact | §9 |
| Chip-local alternative considered and why insufficient | §10 |
| Migration/freeze plan, inventory item, integrator acceptance record (who/when) | §11 |
| Open decisions | §12 |
| Explicit exclusions / adjacent findings | §13 |

---

## 1. Motivation and finding map

The Stage-B determinism contract depends on four things being true at the same
time:

1. the canonical snapshot distinguishes every observable bus state, including
   pending `AppendRecords` work;
2. the scheduler consumes the declared stage assignment, so the pipeline has
   the order the acceptance rows claim;
3. every configured limit and every store revision is accounted for, with no
   silent clamp and no stale-write hole; and
4. replay evidence records the pinned inputs and can compare traces, instead
   of only re-running a fresh bus.

`/39` is credible for run-to-run determinism of the encoded subset, but it is
not yet a replay contract. The four findings below are already registered in
the dispatch ledger and in A23; this CCR packages their shared-interface
changes for an integrator decision.

| Ledger ID | Priority | Finding | Primary evidence | A23/A02 reference |
|---|---|---|---|---|
| **D-12** | **P0** | `AppendRecords` wire proposals encode only the draft count in snapshots; different append batches with equal counts hash identically. | `compiler/src/snapshot.rs:446-455`; `compiler/src/task.rs:1862-1867`; `compiler/src/records.rs:25-96` | A23-F01; A02-F06; rule `snapshot.wire-payloads-encoded` (`contract.rs:125`) |
| **D-13** | P1 | Scheduler order uses the task-group ordinal, not `STAGE_ASSIGNMENT`; T01 §4's three-key order conflicts with M1 D5's four-key stage-first order. | `compiler/src/routing.rs:34-38`, `:223-243`; `compiler/src/task.rs:1783-1789`; `compiler/src/manifest.rs:1444-1527` | A23-F07; A16-R05/Q06; T01 `:78`; M1 target `:369` |
| **D-14** | P1 | `allocated_total()` omits macros; `stage_queue_bound` is unused; a zero quota is silently clamped to one; typed appends do not bump store versions; `ResultRecord.version` is always `0`. | `compiler/src/bus.rs:679-707`; `compiler/src/commit.rs:1425-1438`, `:1455-1462`, `:1493-1658`, `:1700-1718`; `compiler/src/routing.rs:251-253`; `compiler/src/limits.rs:54-67`; `compiler/src/task.rs:2071-2078` | A23-F05/F08; A02-F04 |
| **D-15** | P1 | `Snapshot`/`Trace` have no restore/decoder/diff; "replay" tests re-run fresh buses; pins are not recorded. | `compiler/src/snapshot.rs:522-1064`; `compiler/tests/c05_codec.rs:101-112`; `c17_vf12.rs:653-667`; `c19_vf01.rs:625-632` | A23-F10/F13, H-D; A17 §11.6; A02-F06 |

**Why this cannot be fixed chip-locally.** All four findings live in
integrator-owned foundation files: `snapshot.rs`, `routing.rs`, `commit.rs`,
`bus.rs`, `task.rs`, `limits.rs`, `manifest.rs`, and `contract.rs`. Chips are
zero-field, stateless proposal producers; they neither schedule, commit, nor
serialize. A chip-local workaround would either create a private second source
of truth or silently weaken a frozen invariant. §10 records the rejected
alternatives.

**Smallest sufficient change.** One integrator-owned `/40` freeze that:

- encodes append handles and bodies in the existing wire arm (D-12);
- freezes one total scheduler key and implements it (D-13);
- closes the five concrete limit/version holes (D-14); and
- makes replay evidence explicit and pin-aware, while narrowing the snapshot
  claim if full restore is not in `/40` (D-15 minimal).

Full `Snapshot` restore, per-tick semantic/scheduler projections, and quota>1
reporting are **explicitly deferred** to `/41` (§2, §6.6, §13).

---

## 2. Package decision: mandatory minimal vs optional/deferred

### 2.1 Two-package split

| Package | Defects | Content | Why this boundary |
|---|---|---|---|
| **`/40` — mandatory minimal** | D-12, D-13, D-14, D-15 minimal | Append wire bodies; one scheduler key; macro total; zero-quota fail-closed; typed-append/name store versions; `ResultRecord.version`; stage-bound contract narrowing; `Task.id` encoding; trace pin digest; trace decoder/diff; replay-test correction | Restores truthful canonical state, truthful limit claims, and a usable first-divergence replay check without inventing a restorable bus image |
| **`/41` — optional/deferred** | D-15 remainder, A23-F02/F03/F06/F09/F12/F14 | Full `Snapshot::from_bytes` + `CompilerBus::restore` + continue-tick; full config image; VF13 semantic/scheduler projections; quota>1 dispatch/report/commit order; per-stage queues/bounds; `StorePatch` materialization/tombstone liveness; full pin bytes in bus records; full hash-scope coverage | These require new state, new projections, or new policy decisions that are not needed to make the `/40` claims true |

### 2.2 Recommended freeze order

1. **Policy decisions first (§12):** D-13 scheduler key; D-15 snapshot meaning
   (fingerprint vs restorable image); D-14 stage-bound status; D-14 result
   version semantics.
2. **Shape freeze (`/40`):** new rule IDs, `G1_DRAFT_BODY_NAMES`,
   `APPEND_BATCH_FIELDS`, append-family/store map, trace/pin field lists,
   `Task.id` encoding, `Control.next_commit_ordinal` if selected.
3. **Implementation and dependent tests:** D-13, then D-14, then D-12, then
   D-15 minimal; rerun `c05`/`c06`/`c07`/`c08`/`c09`/`freeze` and all slice
   suites that append records.
4. **Hash/version:** recompute `CONTRACT_HASH`, bump `CONTRACT_VERSION` to
   `/40`, preserve `/39` as history.
5. **`/41` deferred package:** only after `/40` is accepted and measured.

Nothing in steps 1–5 is authorized by this document.

### 2.3 Item-level classification

| Defect | Sub-item | Classification | Freeze |
|---|---|---|---|
| D-12 | Append handles and bodies in snapshots | **Mandatory minimal** | `/40` |
| D-13 | Stage-first scheduler key (or explicit opposite decision) | **Mandatory decision + implementation** | `/40` |
| D-14 | `macros` in `allocated_total()` | **Mandatory minimal** | `/40` |
| D-14 | `stage_queue_bound` false enforcement claim | **Mandatory contract narrowing**; actual per-stage enforcement deferred | `/40` clause, `/41` enforcement |
| D-14 | Zero-quota silent clamp | **Mandatory minimal** | `/40` |
| D-14 | Typed-append / `Names` store versions | **Mandatory minimal** | `/40` |
| D-14 | `ResultRecord.version` semantics | **Mandatory minimal** (shape decision open) | `/40` |
| D-15 | Pin digest in replay evidence | **Mandatory minimal** | `/40` |
| D-15 | Trace IDs + decoder + first-divergence diff | **Mandatory minimal** | `/40` |
| D-15 | Snapshot `Task.id` completeness | **Mandatory minimal** | `/40` |
| D-15 | Full config image in snapshot | Optional/deferred | `/41` |
| D-15 | `Snapshot::from_bytes` + restore + continue | Optional/deferred | `/41` |
| D-15 | VF13 semantic/scheduler projections | Optional/deferred | `/41` |
| D-15 | Quota>1 dispatch/report/commit order | Optional/deferred | `/41` |
| D-15 | Full pin bytes in a bus record | Optional/deferred | `/41` |

The `/40` package is therefore not "all of D-15"; it is the minimal set that
makes the current snapshot/trace/limit claims true and gives replay a real
first-divergence check. The restore/projection package remains `/41`.

---

## 3. D-12 — `AppendRecords` wire proposal bodies

### 3.1 Current shape

`Snapshot::capture` routes every wire proposal through `push_wires`
(`compiler/src/snapshot.rs:393-487`). The `AppendRecords` arm is:

```rust
crate::task::Proposal::AppendRecords { task, batch } => {
    w.u32(task.index());
    // ... comment: records-track pending ...
    w.u64(batch.records.len() as u64);
}
```

The batch itself is:

```rust
pub struct AppendBatch {
    pub records: Vec<RecordDraft>,   // handle family + own-batch index
    pub bodies: Vec<G1DraftBody>,    // typed bodies, 1:1 positional
}
pub struct RecordDraft { pub family: RecordFamily, pub index: DraftRef }
```

`RecordDraft` and `G1DraftBody` are defined in `compiler/src/records.rs:25-96`.
`G1DraftBody` currently has 18 variants (`Literal`, `Const`, `Artifact`,
`Token`, `Name`, `Node`, `Type`, `Symbol`, `Scope`, `ScopeEvent`, `Sem`,
`Function`, `Block`, `Value`, `Instruction`, `Span`, `PpToken`, `Macro`).
Every body variant already has a canonical per-record encoder in
`snapshot.rs` (`encode_literal_record`, `encode_const`, `encode_artifact_record`,
`encode_token`, `encode_node`, `encode_type`, `encode_symbol`, `encode_scope`,
`encode_scope_event`, `encode_sem`, `encode_function`, `encode_block`,
`encode_value`, `encode_instruction`, `encode_span_record`, `encode_pp_token`,
`encode_macro`); `Name` is spelling bytes interned through `InternTable`.

Consequence (A23-F01): two proposals with the same draft count but different
family, handle index, or body produce **identical** snapshot bytes. The
normative rule `snapshot.wire-payloads-encoded` (`compiler/src/contract.rs:125`)
and T01 C05's "full wire proposal payloads" claim (`T01:880`) are false for the
largest new wire arm. The only existing snapshot test for this arm uses an
empty batch (`compiler/tests/c05_codec.rs:633-650`), so the hole is untested.

### 3.2 Proposed shape (wire bytes and tables)

**Recommended:** encode both handle and full body, in batch order, without
validating 1:1 length in the encoder. A malformed mid-tick wire state is still
observable state and must be distinguishable; commit remains the validator
(`append.bodies-match-records`).

```text
Proposal::AppendRecords (wire tag 5):
    u32  task.index()
    u64  records.len()
    records.len() × {
        str  family_name          // RecordFamily::name(), canonical
        u32  draft.index.index()  // DraftRef position
    }
    u64  bodies.len()
    bodies.len() × {
        str  body_name            // G1_DRAFT_BODY_NAMES, canonical
        <canonical body bytes>    // existing per-record encoder
    }
```

`Name` body bytes are `Writer::bytes(spelling)`; all other variants call the
existing `encode_*` function. The body-name order in `G1_DRAFT_BODY_NAMES`
must be the declaration order in `records.rs`; the hash fingerprints the
names, not just the ordinal.

**New hashed tables / field lists (proposed):**

```rust
pub const G1_DRAFT_BODY_NAMES: [&str; 18] = [
    "literal", "const", "artifact", "token", "name", "node",
    "type", "symbol", "scope", "scope_event", "sem", "function",
    "block", "value", "instruction", "span", "pp_token", "macro",
];
pub const APPEND_BATCH_FIELDS: [&str; 2] = ["records", "bodies"];
```

**Rules (proposed, hashed):**

| Rule ID | Meaning |
|---|---|
| `snapshot.append-batch-handles-encoded` | Every `RecordDraft.family` and `.index` is in the snapshot bytes |
| `snapshot.append-batch-bodies-encoded` | Every `G1DraftBody` variant body is in the snapshot bytes via its canonical encoder |
| `snapshot.append-batch-order-preserved` | Handles and bodies encode in stored batch order (no sorting) |

The umbrella rule `snapshot.wire-payloads-encoded` stays; the three new rules
are its concrete coverage clauses for the `AppendRecords` arm.

**Why full bodies, not a body hash.** A body hash would make the snapshot
sensitive but not lossless; it cannot support the `/41` restore path or an
independent re-encode check. The full body is the same canonical bytes already
used by the record encoders, so it adds no second serialization format. The
snapshot grows by the encoded pending drafts only; that work is already bounded
by `max_proposals_per_tick` and the per-arena capacities, so no new limit is
introduced.

### 3.3 Affected interfaces / files

| File | Impact |
|---|---|
| `compiler/src/snapshot.rs` | `push_wires` append arm; new body-name constant; no new record encoder |
| `compiler/src/contract.rs` | Hash `G1_DRAFT_BODY_NAMES`, `APPEND_BATCH_FIELDS`; new rules |
| `compiler/src/records.rs` | No shape change; names must match the existing 18 variants |
| `compiler/tests/c05_codec.rs` | Replace the empty-batch-only check with sensitivity across every variant/family/index |
| `compiler/tests/freeze.rs` | Cross-check the 18 names against the `G1DraftBody` variant set |

No `Task`/`Proposal`/`AppendBatch` shape change; no new wire tag; no new limit.

### 3.4 Acceptance

- Same-count/different-family, same-count/different-index, and same-count/
  different-body snapshots all differ.
- Every one of the 18 `G1DraftBody` variants round-trips through its existing
  encoder and changes the snapshot.
- An empty batch still encodes deterministically and differs from a non-empty
  batch.
- A malformed handle/body length mismatch is encoded as-is, then rejected by
  commit before mutation (no snapshot validation is invented).

---

## 4. D-13 — Scheduler order: group-first vs stage-first

### 4.1 Current shape and conflict

Implementation (`compiler/src/routing.rs:34-38`, `:223-243`;
`compiler/src/task.rs:1783-1789`):

```rust
pub const fn phase_priority(kind: TaskKind) -> u16 {
    kind.group().raw() as u16          // task-group ordinal, not stage
}
pub fn schedule_key(&self, phase_priority: u16) -> (u16, u64, u32) {
    (phase_priority, self.enqueue_ordinal, self.id.index())
}
```

`select_batch` collects eligible `(phase, enqueue_ordinal, TaskId)` keys and
sorts them. `STAGE_ASSIGNMENT` (`compiler/src/manifest.rs:1444-1514`) and
`stage_of` (`:1517-1527`) exist and are hashed (`contract.rs:741-745`), but
`stage_of` is consumed only by manifest registration and worker-layer checks
(`manifest.rs:1536-1573`; `chips/mod.rs:325-334`, `:379-391`), never by
selection.

The contract conflict is explicit:

| Source | Required order | Status |
|---|---|---|
| T01 §4 (`T01:78`) | `(phase priority, enqueue ordinal, TaskId)` | Frozen `/39`; `phase priority` is implemented as group ordinal |
| M1 target D5 (`M1_TARGET_ACCEPTANCE.md:369`) | `(stage ordinal, phase priority, enqueue ordinal, TaskId)` | M1 acceptance row; not implemented |
| M1 target D8 (`:372`) | commit conflict order `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal index)` | Quota>1 acceptance row; not implemented |
| ADR-0002 §6.2.1 (`docs/architecture/ADR-0002-...:145`) | stage-first four-key | PROPOSED |
| T02 CT03 (`T02_CONTROL_CHIPS.md:9`, `:17`) | stage-first four-key | PROPOSED |
| M1 proposal §6.2.1 (`M1_PART_A_CONTRACT_PROPOSAL.md:2345-2349`) | stage-first four-key | DRAFT |
| Implementation `/39` | group-first three-key | Current |

Concrete disagreement: a ready `VERIFICATION_TOKEN_AST_INVARIANT` (stage 2,
group 12) and a ready `SEMANTIC_LITERAL_EXPR` (stage 4, group 6) are selected
semantic-first today; the declared stage order is verification-first. The
current behavior therefore cannot support a "stage-priority pipeline" claim.

### 4.2 Recommended decision

**Adopt M1 D5's four-key order and amend T01 §4:**

```text
selection key = (stage ordinal, group/phase priority, enqueue ordinal, TaskId)
```

Rationale:

- `STAGE_ASSIGNMENT` is the only declared kind→stage authority and is already
  hashed; ignoring it makes the table misleading and the worker-layer check
  the only consumer.
- Four independent documents (M1 target, ADR-0002, T02, M1 proposal) already
  use the stage-first order; T01 §4's three-key order predates the stage table
  and is the outlier.
- A stage-4 semantic task running before a stage-2 verification task is
  semantically backwards for the verification checkpoints the M1 gate expects.

If the integrator instead keeps group-first, then D5, ADR-0002, T02, and the
M1 proposal must all be amended to say `STAGE_ASSIGNMENT` is a worker-layer
gate only, and the "stage-priority" pipeline claim must be removed. That is a
larger documentation correction than adopting the four-key order.

### 4.3 Proposed shape

```rust
/// Stage priority used by the scheduler. `None` sorts after every assigned
/// stage (sentinel `STAGE_COUNT`) so selection stays total; dispatch still
/// fails the task explicitly through the existing unregistered/unsupported
/// path.
pub const fn stage_priority(kind: TaskKind) -> u8 {
    match stage_of(kind) {
        Some(stage) => stage,
        None => STAGE_COUNT as u8,
    }
}

pub fn schedule_key(&self, stage: u8, phase: u16) -> (u8, u16, u64, u32) {
    (stage, phase, self.enqueue_ordinal, self.id.index())
}
```

`select_batch` sorts `(stage, phase, enqueue_ordinal, TaskId)` and keeps the
existing quota slice. `phase_priority` stays the group ordinal (name unchanged
to avoid a public rename; doc clarified). `control.stage` remains a reporting
label; it is not the scheduler key. The `Stage` enum (`bus.rs:739-784`, 0..10)
and the `stage_of` ordinals (0..6 rows today) are distinct vocabularies and
must be documented as such; no mapping between them is invented here.

**Complexity note.** `stage_of` is currently a linear scan over 69 rows
(`manifest.rs:1517-1527`), and selection would call it for every ready task
every tick. `/40` must replace that hot-path lookup with a `const` match, a
sorted binary search, or a precomputed kind→stage table; the frozen rows and
their hash coverage stay identical. The intended selection cost is
O(n log n) for the sort plus O(1) per kind lookup, never O(n×69) per tick.

**Rules (proposed, hashed):**

| Rule ID | Change |
|---|---|
| `task.select.stage-phase-enqueue-taskid` | New canonical selection rule |
| `task.select.phase-enqueue-taskid` | Remove from `NORMATIVE_RULES` (preserve `/39` as history) |
| `dispatch.stage-assignment-authoritative` | `STAGE_ASSIGNMENT` is the scheduler authority, not only a registration gate |

**Coverage.** `STAGE_ASSIGNMENT` currently has 69 unique rows covering the 69
`TaskKind` constants (A23 static check). Every new kind must add one row when
its manifest is registered; `check_stage_assignment` already enforces this for
manifests. Foundation kinds have stage 0 rows. A task bootstrapped with an
unassigned kind sorts last via the sentinel and still fails explicitly as
unregistered/unsupported; it is never silently scheduled as stage 0.

### 4.4 Affected interfaces / files

| File | Impact |
|---|---|
| `compiler/src/routing.rs` | `phase_priority` doc; `select_batch` key; `select` unchanged |
| `compiler/src/task.rs` | `Task::schedule_key` signature (stage + phase) |
| `compiler/src/manifest.rs` | No table change; `stage_of` becomes scheduler-consumed |
| `compiler/src/contract.rs` | Rule rename/addition in `NORMATIVE_RULES` |
| `compiler/tests/c06_routing.rs` | New `stage_order` test with stage/group disagreement |
| `compiler/tests/c09_readiness.rs` | Any ordering-dependent fixture; unassigned-kind fallback |

### 4.5 Acceptance

- `VERIFICATION_TOKEN_AST_INVARIANT` is selected before
  `SEMANTIC_LITERAL_EXPR` when both are ready at quota 1.
- Equal-stage tasks still order by group, enqueue ordinal, and `TaskId`.
- An unassigned kind sorts last and still fails explicitly (no stage-0
  default).
- The existing quota-1 selection fixtures remain deterministic.
- No commit-order change is included in D-13; D8's `dispatch_ordinal` key is
  `/41` quota>1 work (§13).

---

## 5. D-14 — Limit accounting and store versions

D-14 contains five independent holes. Each is mandatory for the `/40`
truthfulness claim; the exact shape choice is listed in §12.

### 5.1 `allocated_total()` omits macros

**Current.** `Arenas::allocated_total()` (`bus.rs:679-707`) sums every arena
except `macros` (`Arenas::macros` is declared at `bus.rs:621`). Therefore
`CompilerBus::total_records()` (`bus.rs:1020-1023`) omits pre-existing macro
records, while `commit_proposals` adds only the current batch's `new_macros`
(`commit.rs:2064-2086`). Repeated macro batches can exceed `max_records_total`
while `ensure_total_records` reports success.

**Proposed.** Add `self.macros.allocated()` to `allocated_total()`. No shape,
snapshot, or limit change. This makes `total_records()` and
`ensure_total_records(0)` agree with the macro arena.

**Rules/tests.** New `limits.total-records-includes-macros`; `c07_limits`
appends macros and asserts the total grows and the configured bound rejects
the next batch. `c23_macro` remains the producer-side coverage.

### 5.2 `stage_queue_bound` is unused

**Current.** `stage_queue_bound` is declared (`limits.rs:61-67`), validated
(`:160-167`), hashed (`contract.rs:659-661`), and snapshotted
(`snapshot.rs:109-111`). `commit.rs:1425-1438` calls
`check_backpressure_capacity` with `max_queue_len` and `stage = 0`; no
per-stage queues exist. The function doc (`commit.rs:2217-2225`) claims it
checks `stage_queue_bound[stage]`, which is false.

**Recommended `/40` decision: contract narrowing, not implementation.**
Keep the field as a validated, hashed, snapshotted **reserved** bound; record
that the enforced queue bound at quota 1 is `max_queue_len` through
`check_backpressure_capacity`; split the normative rule so the contract no
longer claims enforcement:

| Current rule | Proposed rules |
|---|---|
| `limits.every-configured-bound-enforced` | `limits.declared-bounds-enforced` (lists the bounds actually enforced on checked entry points) |
| — | `limits.stage-queue-bound-reserved` (explicitly not enforced until per-stage queues land) |

This is the smaller change and matches A16-Q02 ("`stage_queues` are a quota>1
extension, not a quota-1 requirement"). Per-stage queues or a projected
per-stage count are `/41` work (§6.6/§13). The `check_backpressure_capacity`
doc must stop claiming stage enforcement; its `stage` parameter remains a
reporting label for future use, or is removed when the projection is deleted.

**Alternative (not recommended for `/40`).** Project each ready task's
`stage_of(kind)` plus the batch's new/progress/join tasks into per-stage
counts and compare to `stage_queue_bound` without adding `stage_queues`. It is
feasible, but it is an O(queue) scan per commit with an unassigned-kind
bucket decision, and it duplicates the stage-queue semantics that `/41` will
own. `max_queue_len` is already the real bound today.

**Tests.** `c07_limits` asserts `stage_queue_bound` changes the config
hash/snapshot (already `c09` pcr01) but does **not** reject a ready queue that
exceeds a per-stage entry; the test name and assertion must say "reserved".
`freeze.rs` checks the rule list contains the reservation rule.

### 5.3 Zero quota is silently clamped

**Current.** `Limits::validate` rejects `max_inflight_per_tick == 0`
(`limits.rs:154-158`), but `CompilerBus::new`/`CompilerConfig::new` are
infallible and do not validate (`bus.rs:975-990`; `target.rs:1236-1255`).
`RoutingShell::dispatch_quota` then applies `.max(1)`
(`routing.rs:251-253`), so an invalid config silently runs at quota 1.

**Proposed.**

- Remove the `.max(1)` clamp. `dispatch_quota` returns the configured value
  verbatim (`0` included).
- `propagate_inner` checks `quota == 0` before selection and returns a typed
  `CommitError::Limit(LimitError::InvalidInflightQuota { value: 0 })`; no
  dispatch and no commit occur.
- `CompilerBus::try_new`/`CompilerConfig::try_new` remain the validated
  production path; `new` is documented as a trusted integration boundary (as
  are the public bus fields), not a validation bypass.
- `bootstrap_task`/`commit_proposals` may additionally call
  `limits.validate()` as a checked-entry-point guard so an invalid bus cannot
  acquire runnable work; the exact guard placement is an open decision
  (§12, OD-4).

**Rules/tests.** `dispatch.zero-quota-rejected` (or
`limits.zero-quota-fail-closed`); `c06_routing`/`c07_limits` assert
`dispatch_quota == 0`, `propagate` returns the typed error, no task is
dispatched, and `Limits::try_new` still rejects zero.

### 5.4 Typed appends do not bump store versions

**Current.** `commit_proposals` records `touched_stores` only for
`Proposal::StorePatch` (`commit.rs:1700-1718`). The typed `AppendRecords` arms
(`:1493-1658`) append records but never insert the owning store; `intern_name`
(`bus.rs:1096-1100`) and the `Name` body (`commit.rs:1643-1656`) do not bump
`StoreId::Names`, despite the comment at `task.rs:2071-2078`. A later
`StorePatch` computed against the unchanged version passes
`validate_patch` (`commit.rs:1898-1905`) after the append already changed the
store: a stale-write hole.

**Proposed canonical map.** Reuse the existing family→store/field mapping in
`validate_append_authorization` (`commit.rs:1791-1846`) as one function
(`fn append_store(body: &G1DraftBody) -> StoreId` or a table) so validation
and apply cannot drift:

| `G1DraftBody` | `StoreId` | Declared field |
|---|---|---|
| `Literal` | `Lex` | `literals` |
| `Const` | `Constants` | `records` |
| `Artifact` | `Artifacts` | `fragments` |
| `Token` | `Lex` | `tokens` |
| `Name` | `Names` | `entries` |
| `Node` | `Parse` | `nodes` |
| `Type` | `Types` | `records` |
| `Symbol` | `Symbols` | `symbols` |
| `Scope` | `Symbols` | `scopes` |
| `ScopeEvent` | `Symbols` | `scope_events` |
| `Sem` | `Sem` | `records` |
| `Function` | `Ir` | `functions` |
| `Block` | `Ir` | `blocks` |
| `Value` | `Ir` | `values` |
| `Instruction` | `Ir` | `instructions` |
| `Span` | `Sources` | `spans` |
| `PpToken` | `Pp` | `tokens` |
| `Macro` | `Pp` | `macros` |

**Revision unit.** Bump each mutated store **exactly once per successful
commit batch**, matching the existing `StorePatch` behavior. A batch that
appends only `Const` records bumps `Constants` once, regardless of record
count. A batch that appends `PpToken` and `Macro` bumps `Pp` once.

**Names.** A `Name` body bumps `Names` **iff** the intern table length grew
(a genuinely new name). A duplicate lookup returns the existing ID and does
not bump. `CompilerBus::intern_name` (the trusted Host/checked entry point)
applies the same rule: record `len` before, call `intern`, bump once iff the
length grew. Exactly one logical bump per new name, whichever path interns it.

**Rules/tests.** `commit.append-bumps-store-version`,
`commit.names-bump-on-new-only`. Tests: typed append changes the owning store
version; a stale patch after append is rejected; a new name bumps `Names`
exactly once; a duplicate name does not bump; two appends to different stores
bump each once.

### 5.5 `ResultRecord.version` is always `0`

**Current.** `commit.rs:1455-1462` creates every result with `version: 0`,
while the field doc (`task.rs:1750-1752`) says "store/protocol version the
result was produced against". A16-L01 repeats the same intended meaning. The
snapshot encodes the field (`snapshot.rs:898`), so the zeros are observable
but meaningless.

**Recommended.** Add a global commit ordinal and define the field:

```rust
pub struct Control {
    // ... existing fields ...
    pub next_commit_ordinal: u64,   // starts at 0
}
```

On a successful `commit_proposals` batch, increment `next_commit_ordinal` and
use the new value (1-based) for every `ResultRecord.version` created by that
batch. A failed commit does not increment. The field is then a deterministic,
monotonic protocol revision, not a per-store vector. Snapshot encodes
`next_commit_ordinal`.

**Alternatives (open decision OD-5).**

- **Tick:** set `version = control.tick`; no new field, but multiple commits in
  one tick share a value and the doc must be renamed "commit tick".
- **Remove:** delete `ResultRecord.version`; smallest shape but removes a
  frozen field and any future protocol-revision metadata.

**Rules/tests.** `commit.result-version-meaningful`; `c03_task`/`c05_codec`
assert two successful commits produce different versions, a failed commit does
not consume an ordinal, and replay produces the same version sequence.

---

## 6. D-15 — Replay evidence: pins, trace comparison, restore

### 6.1 The model decision

Current wording overclaims. `Snapshot` is described as encoding "every
observable store" (`snapshot.rs:4-16`) and T01 C05 says replay is "consistent
per tick" (`T01:138-139`). In fact:

- `Snapshot` has no `from_bytes`/restore and writes `config_hash` only
  (`snapshot.rs:548`; A23-F13);
- `Trace` has no parser or diff (`snapshot.rs:979-1064`; A23-F10/H-D);
- the existing "replay" tests construct two fresh buses and rerun
  (`c05_codec.rs:101-112`, `c08_gate1.rs:748-800`, `c17_vf12.rs:653-667`,
  `c19_vf01.rs:625-632`);
- `CompilerPins` (`bus.rs:913-933`) is never recorded in either artifact.

**Recommended `/40` model — recorded-input re-execution, not bus restoration.**

```text
ReplayEvidence (Host-owned envelope; not a new bus record in /40):
    contract_version
    contract_hash
    config: Vec<u8>                 // encode_config(config)
    initial_snapshot_hash: [u8; 32]
    pins_digests: Vec<[u8; 32]>     // one per tick, in tick order
    trace_hash: [u8; 32]
```

The Host retains the exact `CompilerPins` sequence and source/config inputs.
`Snapshot` is a **canonical state fingerprint** for the encoded subset, not a
restorable bus image, until `/41` adds restore. The C05/T01 wording must be
narrowed accordingly. This is the minimal honest contract and matches M1
D1/D2 (identical snapshot/trace hashes) rather than inventing a restore
requirement.

If the integrator decides a restorable image is mandatory for M1 evidence,
then §6.6's restore package moves into `/40`; that is an explicit open
decision (OD-6) with a larger schema/decoder/test cost.

### 6.2 Pins

**Current.** `CompilerPins { start_job, cancel, host_responses, tick_budget_reached }`
is applied by `clock_tick`/`clock_tick_with` but never stored or hashed.
Two runs with identical bus state and different pins are indistinguishable in
the current evidence.

**Proposed.** Add a canonical pin digest:

```rust
pub fn pins_digest(pins: &CompilerPins) -> [u8; 32];
```

Canonical encoding, in field declaration order:

```text
u8 start_job
u8 cancel
u8 tick_budget_reached
u64 host_responses.len()
host_responses.len() × { u32 request.index(); bytes(response.bytes) }
```

`host_responses` preserves the Host-supplied order (input identity, not a
sorted set). Add `pins_digest` to `TraceRecord` and require the trace recorder
to receive the pins (`Trace::record(bus, report, pins)` or
`record_with_pins`). The Host envelope above stores the exact pins; the digest
is the bus-side/replay-side identity.

**Alternative.** Add `pins_digest` to `TickRecord` so `Snapshot` (which
encodes `report`) is self-contained. This is not recommended for `/40`
because it changes a bus shape for an input already owned by the Host; it is
listed in OD-7.

### 6.3 Trace decoder, ID sensitivity, and first divergence

**Current.** `TraceRecord` stores `tick`, `selected`, `outcome`,
`resolution`, `ready_len`, and four counts (`snapshot.rs:952-971`). It drops
`commit.waiting`, `commit.rejoined`, `commit.patches`, `commit.appended`, the
actual completed/failed IDs, and the `CommitError` on `CommitFailed`
(`snapshot.rs:986-1008`). `Trace` has only `to_bytes`/`hash`
(`:1036-1064`). Two runs with equal counts but different IDs can hash
identically; D7's "first divergence" is not implemented.

**Proposed `TraceRecord`:**

```rust
pub struct TraceRecord {
    pub tick: u64,
    pub pins_digest: [u8; 32],
    pub selected: Option<TaskId>,
    pub outcome: &'static str,          // stable name
    pub resolution: &'static str,       // stable name
    pub error: Option<DiagnosticCode>,  // CommitFailed, stable (group, code)
    pub ready_len: u32,
    pub enqueued: Vec<TaskId>,
    pub completed: Vec<(TaskId, ResultId)>,
    pub failed: Vec<(TaskId, DiagnosticId)>,
    pub waiting: Vec<TaskId>,
    pub rejoined: Vec<TaskId>,
    pub appended: Vec<RecordRef>,
    pub patches: u32,
}
```

**Proposed `Trace` API:**

```rust
pub fn from_bytes(bytes: &[u8]) -> Result<Trace, CodecError>;
pub fn diff(&self, other: &Trace) -> TraceDiff;

pub struct TraceDiff { pub first_divergence: Option<TraceDivergence> }
pub struct TraceDivergence { pub index: usize, pub tick: u64 }
```

`diff` compares records in tick order and returns the first index/tick whose
canonical bytes differ, or `None`. `selected` is the quota-1 dispatch
projection; all lists are in canonical order: apply order for
`enqueued`/`completed`/`failed`/`appended`, and `TaskId` order for
`waiting`/`rejoined` as produced by commit. No hash-map iteration.

**Complexity note.** `from_bytes` is O(retained records + payload bytes) and
must bound every `Vec` reservation by the reader's remaining bytes (or a
configured maximum), not by an unchecked count prefix (A23-F14). `diff` is
O(records) with per-field comparison; it must not re-serialize the whole trace
for every record.

**Rules/tests.** `trace.canonical-ids-recorded`,
`trace.decoder-round-trip`, `trace.first-divergence`; `c05_codec` tests equal
counts/different IDs produce different trace hashes and a correct first
divergence, and truncation/unknown-name inputs fail with `CodecError` (bounded
reservations per A23-F14). Quota>1 `dispatched`/multi-outcome remains `/41`.

### 6.4 Snapshot completeness in `/40`

| Item | Current | Proposed `/40` | Rationale |
|---|---|---|---|
| `AppendRecords` handles/bodies | Count only (`snapshot.rs:446-455`) | Full handles + bodies (§3) | D-12 |
| `Task.id` self-ID | Key only; `Task.id` field omitted (`snapshot.rs:574-599`) | Encode `task.id` after the key; test mutating it changes the snapshot | A23-F13: observable field must participate |
| Config | `config_hash` only (`:548`) | Keep hash; replay envelope carries `encode_config` bytes | Restore is `/41`; hash is collision-resistant identity |
| `Task.id == arena key` | Not checked | Encode both; do not add panic control flow; test divergence is visible | State completeness without inventing a validator |
| Pins | Absent | Trace digest (§6.2) + Host envelope | D-15 |
| Reserved-store bodies | IDs only (`push_reserved`) | Unchanged | Schema not frozen for layouts/inits/vregs |
| `Task.id` removal alternative | — | OD-8: remove redundant field or encode it | If removed, snapshot need not grow |

**Exact snapshot placement (proposed).**

- Control section (`snapshot.rs:532-545`): write `u64 next_commit_ordinal`
  immediately after `next_diagnostic_ordinal` when OD-5 selects the commit
  ordinal; no other control position changes.
- Task section (`snapshot.rs:574-599`): after the arena key `w.u32(id.index())`,
  write `u32 task.id.index()`, then the existing kind/owner/ordinal/state
  fields. The key remains the arena position; `task.id` is the record field.
- Trace bytes (separate artifact, `snapshot.rs:1037-1057`): write contract
  version, record count, then each record in the §6.3 field order;
  `pins_digest` comes immediately after `tick`.

No full `Snapshot::from_bytes` is added in `/40`; that is `/41` (§6.6).
`Snapshot::capture` remains read-only and deterministic.

### 6.5 Replay tests must stop claiming restoration

**Current.** Tests named "replay" run the scenario twice from fresh buses
(`c05`, `c08`, `c17`, `c19`). They prove run-to-run determinism, not restore.

**Proposed `/40` test changes.**

- Rename/annotate the existing tests as "run-to-run determinism".
- Add `replay_evidence_is_pin_sensitive`: same bus state, different pins →
  different `pins_digest` and replay envelope.
- Add `trace_decoder_round_trip` and `trace_first_divergence` (§6.3).
- Add `snapshot_task_self_id` (§6.4).
- Add a Host-envelope fixture that records `encode_config` + pins digests +
  trace hash and asserts a second identical run produces the same envelope.
- Do **not** claim restore/continue until the `/41` driver exists.

### 6.6 Deferred `/41` restore/projection package

| Item | Why deferred | Required before claim |
|---|---|---|
| `Snapshot::from_bytes` full decode | Large decoder surface; current capture is not a restorable image | Section-by-section decoder, version/tag checks, bounded allocations, config image |
| `CompilerBus::restore` + continue-tick replay | Needs config, manifests/registry/schema reconstruction, `Task.id` validation | Restore API + `restore -> one tick` equals fresh-run test |
| Full config in snapshot | Only needed for restore; hash + Host config suffices for `/40` | Replace `config_hash` with `push_config`; hash rule |
| VF13 semantic/scheduler projections | New view contract, not required for D1/D2 | A17 §11 tracks; H6-M12 |
| Quota>1 dispatch/report/commit order | A23-F02/F03; guardrail quota>1 is post-freeze | `dispatch_ordinal`, batch outcome, D8 commit key |
| Per-stage queues/bounds | A16-Q02; needs new queue state | Stage queue schema + limits |
| `StorePatch` materialization/tombstone liveness | A02-F02/SB-03, not D-14 | Store/family/liveness validation |
| Full pin bytes in bus | Host envelope suffices at `/40` | `TickRecord.pins` shape decision |

---

## 7. Affected chips, task kinds, manifests, and tests

### 7.1 Chips and task kinds

**No chip source file changes.** The changes are integrator-owned foundation
changes. All registered chips are nevertheless affected at runtime:

- every kind in `STAGE_ASSIGNMENT` (69 rows) is scheduled by the new key;
- every chip that emits `AppendRecords` is affected by the store-version bump
  and snapshot body encoding: PP normalize/splice/comment/scan/directive/
  conditional/macro/invoke/substitute/include/variadic/builtin/line/pragma/
  expand_map/emit; LX intern/classify/decode/float/string; PA TU/decl/expr/
  recovery; TY scope/symbol/type; SE literal/binary/return/function_def; CL
  fold/branch; IR function; and the read-only VF chips that consume them;
- no chip gains a new read/write manifest field; no new chip ID, kind, stage
  row, or allowlist row is created.

Specific ordering fixture: `VERIFICATION_TOKEN_AST_INVARIANT` (stage 2,
group 12) must precede `SEMANTIC_LITERAL_EXPR` (stage 4, group 6).

### 7.2 Manifests / tables

| Table | Change |
|---|---|
| `STAGE_ASSIGNMENT` | No content change; becomes scheduler authority; every new kind must add a row |
| `STORE_OWNER_ALLOWLIST` | No change |
| `M1_STORE_FAMILY_ARENA` | No change; the append-family→store function must agree with it |
| `NORMATIVE_RULES` | Add/rename the rules listed in §3–§6; remove the disproven `task.select.phase-enqueue-taskid` and replace `limits.every-configured-bound-enforced` |
| `FrozenSchema::encode` | Add `G1_DRAFT_BODY_NAMES`, `APPEND_BATCH_FIELDS`, append-family/store map, `TRACE_RECORD_FIELDS`, `PIN_RECORD_FIELDS`, `TASK_RECORD_FIELDS` (or equivalent), `CONTROL_RECORD_FIELDS` if `next_commit_ordinal` is added |

### 7.3 Tests to add or change

| Test | Proves | Primary file |
|---|---|---|
| `append_wire_body_sensitivity` | Same-count/different family/index/body snapshots differ; all 18 variants | `c05_codec.rs` |
| `stage_order` | Stage-2 VF before stage-4 SE; tie-break order | `c06_routing.rs` |
| `unassigned_stage_fails_explicitly` | Sentinel sort + explicit failure, not stage 0 | `c06_routing.rs` |
| `zero_quota_rejected` | No clamp; typed error; no dispatch | `c06_routing.rs`, `c07_limits.rs` |
| `limit_total_macros` | Macro records count toward `max_records_total` | `c07_limits.rs` |
| `stage_queue_bound_reserved` | Field is config only; no false enforcement claim | `c07_limits.rs` |
| `store_version_after_append` | Typed append bumps owning store; stale patch rejected | `c08_gate1.rs`, `c05_codec.rs` |
| `names_version` | New name bumps `Names` once; duplicate does not | `c11_lex.rs`, `c05_codec.rs` |
| `result_version` | Two commits differ; failed commit does not advance | `c03_task.rs`, `c05_codec.rs` |
| `snapshot_task_self_id` | Mutating `Task.id` changes snapshot (or the field is removed) | `c05_codec.rs` |
| `trace_decoder_round_trip` | `Trace::from_bytes(to_bytes)` identity; truncation/unknown names fail | `c05_codec.rs` |
| `trace_first_divergence` | Equal counts/different IDs differ; first tick reported | `c05_codec.rs` |
| `pins_digest` | Same state/different pins distinguishable | `c05_codec.rs` |
| `replay_evidence_is_pin_sensitive` | Envelope records config/pins/trace | `c05_codec.rs`, `c09_readiness.rs` |
| `enum_name_consistency` extension | `G1_DRAFT_BODY_NAMES` matches the 18 variants; trace/pin field lists match codecs | `freeze.rs` |

Existing "replay" tests in `c08`, `c17`, `c19` must be renamed/annotated as
run-to-run determinism or converted to the envelope model; they must not claim
restore.

### 7.4 Rerun commands (for the implementation package, not this CCR)

```sh
cargo test --manifest-path compiler/Cargo.toml \
  --test c01_arena --test c02_target --test c03_task --test c04_manifest \
  --test c05_codec --test c06_routing --test c07_limits --test c08_gate1 \
  --test c09_readiness --test freeze
cargo test --manifest-path compiler/Cargo.toml \
  --test c10_pp01 --test c11_lex --test c12_parse --test c13_ty \
  --test c14_se --test c15_ir --test c16_pp --test c17_vf12 \
  --test c18_vf05 --test c19_vf01 --test c20_ppscan --test c21_directive \
  --test c22_conditional --test c23_macro --test c24_expand --test c25_include \
  --test c26_variadic --test c27_builtin --test c28_line --test c29_pragma \
  --test c30_expand_map --test c31_emit --test c32_float --test c33_string \
  --test c34_parse --test c35_expr --test c36_recovery --test c37_const_branch \
  --test c38_vf14 --test c39_sefunc
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

The dirty-tree clippy failure recorded by the ledger is not a HEAD verdict;
the implementation must run on a clean committed baseline and report exact
results. This CCR ran none of these commands.

---

## 8. Read/write manifest, phase, and ownership impact

| Aspect | Impact |
|---|---|
| **Read/write manifests** | No chip manifest `reads`/`writes` change. `Snapshot::capture` is read-only. `commit_proposals` gains internal `touched_stores` inserts for typed appends; `intern_name` bumps `Names` on new names. These are integrator/commit-owned writes, not chip writes. |
| **Phase** | All affected work remains `phase=propagation`. No phase-advance change; `control.stage` is untouched. |
| **Ownership** | `snapshot.rs`, `contract.rs`, `routing.rs`, `commit.rs`, `bus.rs`, `task.rs`, `limits.rs`, `manifest.rs`, and `freeze.rs` are integrator-owned ([PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) §2). No chip owner may edit them. The CCR itself is a shared-interface change and requires the acceptance record in §11. |
| **Single writer** | `Tasks.ready` remains dispatcher/commit-owned; no second queue. Store versions are commit-owned (plus the checked `intern_name` host path). `report` remains driver step-6 sole append. Trace is Host-side. |
| **New state** | If OD-5 chooses the commit ordinal, `Control.next_commit_ordinal` is new bus state, snapshot-encoded. If OD-7 chooses the bus report form, `TickRecord.pins_digest` is new state. No new arena, ID, task kind, or chip ID. |
| **Limits** | No new `Limits` field. `stage_queue_bound` is retained and explicitly reserved in `/40`. `max_records_total` accounting is corrected. |

---

## 9. Determinism, replay, and snapshot/hash impact

### 9.1 Determinism

- Append handles/bodies encode in stored batch order; no sorting or
  hash-map iteration is introduced.
- Stage-first selection is a total, stable sort over `(stage, group, enqueue,
  TaskId)`; ties are fully broken by `TaskId`.
- `Trace` lists use commit/apply order and ascending IDs exactly as the
  commit path produces them.
- `pins_digest` encodes `host_responses` in supplied order; the Host must
  supply a deterministic order (it is input identity).
- No address, wall-clock, PID, random seed, or absolute path is added to any
  semantic field. Existing host-path wording (A23 H-E) remains a separate
  open item and is not changed here.

### 9.2 Replay

- `/40` replay is **recorded-input re-execution**, not restoration: same
  config/source/pins + same trace must reproduce the same snapshot and trace
  hashes.
- `Trace::from_bytes` + `diff` make D7's first divergence mechanical.
- Pins are no longer invisible.
- `/41` adds restore/continue and the VF13 semantic/scheduler projection.

### 9.3 Snapshot and trace bytes

- D-12 changes the `AppendRecords` arm and therefore every snapshot containing
  that wire arm; expected snapshot bytes/hashes change.
- `Task.id` encoding changes snapshots that contain tasks (all of them).
- `ResultRecord.version` (if changed) and `Control.next_commit_ordinal` change
  result/control bytes.
- `TraceRecord` field changes alter trace bytes/hashes.
- Stage-first selection can change which task gets which record IDs in mixed-
  stage fixtures; expected snapshots in those fixtures must be re-baselined.
- No existing golden hash constant is changed by this CCR; `freeze.rs` pins the
  contract hash and must be updated only with the `/40` freeze.

### 9.4 Contract hash / version

The current hash is `07f4...fa24` for `/39`. The `/40` hash will change because
`NORMATIVE_RULES` and the hashed table/name lists change. The integrator must:

1. apply the accepted shape changes;
2. recompute `CONTRACT_HASH` from `FrozenSchema::encode`;
3. set `CONTRACT_VERSION` and `compiler/contracts/CONTRACT_VERSION` to `/40`
   with the new hash;
4. preserve `/39` as history (no overwrite of the historical value);
5. update `freeze.rs` version/hash/rule/name assertions.

`/41` is a separate artifact and must not be mixed into `/40`.

---

## 10. Chip-local alternatives and why they are insufficient

| Tempting chip-local fix | Why insufficient |
|---|---|
| Encode append bodies inside the producing chip | Chips return proposals, not snapshot bytes; `Snapshot` is C05/integrator-owned. A chip-side body hash would create a second serialization format and still not make the wire lossless. |
| Avoid `AppendRecords`; use `StorePatch` for typed records | `StorePatch` is logged, not materialized (`commit.rs:1700-1713`; A02-F02), so state would not change; it also loses the typed draft/ID prediction protocol. |
| Let a chip set a stage in its payload and sort on it | Scheduling is the routing shell's responsibility; a payload field duplicates `STAGE_ASSIGNMENT`, is not authoritative, and can diverge per chip. Chips cannot mutate `tasks.ready`. |
| Count macros in the chip that produces them | `max_records_total` is a bus-wide bound over all arenas; a chip sees only its own proposals and cannot account for other producers or existing records. |
| Enforce `stage_queue_bound` in a chip | The queue is a shared dispatcher/commit structure; chips cannot read or mutate it consistently. Backpressure is a commit preflight invariant. |
| Bump store versions in the chip | Only commit knows which bodies were applied atomically; a chip-side bump would be uncommitted, duplicated, or lost on rollback. |
| Set `ResultRecord.version` in the chip | The result is created by commit, not the chip; a chip-side counter would be private, schedule-dependent state. |
| Implement replay/restore in a chip | Chips are stateless, I/O-free, and cannot see pins; snapshot/trace are Host/C05 artifacts. |
| Record pins in a chip | Pins are driver inputs; a chip that read them would violate the no-hidden-state/no-I/O boundary. |

The correct owner for every item is the integrator-owned foundation, not a
chip. This is exactly the Guardrails §3.2/§3.3 condition ("multi-chip/shared
interface changes are contract change requests, never unilateral cross-file
edits").

---

## 11. Migration / freeze plan, inventory, and acceptance record

### 11.1 Inventory items

The Guardrails §3.4 text says "its `/6` inventory item"; the `/6` label is
historical (the artifact is `/39` today). The successor inventory items are:

| Item | Scope | Owner | Paths | Acceptance | Depends on | Package |
|---|---|---|---|---|---|---|
| `B2-40-01` | `AppendRecords` handle/body encoding; `G1_DRAFT_BODY_NAMES`; sensitivity tests | T01/C05 integrator | `snapshot.rs`, `contract.rs`, `records.rs`, `c05_codec.rs`, `freeze.rs` | Same-count/different-body snapshots differ; all 18 variants | OD-2 | `/40` |
| `B2-40-02` | Stage-first selection key; `stage_priority`; rule rename; order test | T01/routing + T02 review | `routing.rs`, `task.rs`, `manifest.rs`, `contract.rs`, `c06_routing.rs` | VF stage-2 before SE stage-4; ties stable | OD-1 | `/40` |
| `B2-40-03` | Macro total accounting | T01/limits | `bus.rs`, `c07_limits.rs` | Macro append counts; bound rejects | none | `/40` |
| `B2-40-04` | Zero-quota fail-closed; remove clamp | T01/routing | `routing.rs`, `c06_routing.rs`, `c07_limits.rs` | Typed error, no dispatch | OD-4 | `/40` |
| `B2-40-05` | Append-family→store map; typed append + Names version bumps | T01/commit | `commit.rs`, `bus.rs`, `contract.rs`, `c08`/`c11`/`c05` | Stale patch rejected; names bump once | OD-3 | `/40` |
| `B2-40-06` | `ResultRecord.version` semantics | T01/commit | `task.rs`, `commit.rs`, `snapshot.rs`, `contract.rs`, `c03`/`c05` | Nonconstant, replay-stable | OD-5 | `/40` |
| `B2-40-07` | `stage_queue_bound` reserved/rule narrowing | T01/limits | `limits.rs`, `commit.rs`, `contract.rs`, `c07_limits.rs`, `freeze.rs` | No false enforcement claim | OD-3 | `/40` |
| `B2-40-08` | Snapshot `Task.id`; trace pin digest; trace ID fields; decoder/diff | T01/C05/T13 | `snapshot.rs`, `bus.rs`, `contract.rs`, `c05_codec.rs`, `freeze.rs` | D7 first divergence; pin sensitivity | OD-6/OD-7 | `/40` |
| `B2-40-09` | Replay envelope + test correction; C05 wording narrowing | T01/T13/Host | `snapshot.rs`, `c05`/`c08`/`c17`/`c19`, T01/A17 docs | No restore claim; envelope stable | B2-40-08 | `/40` |
| `B2-40-10` | Contract version/hash bump; docs and freeze test | T01 integrator | `contract.rs`, `CONTRACT_VERSION`, `freeze.rs`, T01/M1 docs | `/40` hash consistent; `/39` preserved | all `/40` | `/40` |
| `B2-41-01` | Snapshot full decode/restore/continue | T01/C05 | `snapshot.rs`, `bus.rs`, tests | `restore -> one tick` equals fresh run | `/40` accepted | `/41` |
| `B2-41-02` | VF13 semantic/scheduler projections | T13/T01 | VF13 contract + tests | A17 §11 tracks | `/40` | `/41` |
| `B2-41-03` | Quota>1 dispatch/report/commit order | T02/T01/T13 | `routing.rs`, `commit.rs`, `bus.rs`, `snapshot.rs` | D8 canonical order | `/40` | `/41` |
| `B2-41-04` | Per-stage queues/bounds | T02/T01 | `bus.rs`, `limits.rs`, `commit.rs`, `routing.rs` | Stage bound enforced | `/40` | `/41` |
| `B2-41-05` | `StorePatch`/tombstone materialization + VF01 liveness | T01/T02/T13 | `commit.rs`, `arena.rs`, `vf_store.rs` | A02-F02/A23-F06/F09 | `/40` | `/41` |

### 11.2 Migration steps

1. **Accept this CCR** with the §11.3 record. No edit before acceptance.
2. **Freeze `/40` shapes** (rules/tables/field lists) and recompute the hash.
   Do not partially apply D-12/D-13/D-14/D-15-minimal on `/39`.
3. **Implement in item order** `B2-40-01..09`; keep `/39` behavior as history
   where a test needs a legacy assertion.
4. **Rerun** the §7.4 command set on a clean committed baseline; report exact
   results and any unrun checks.
5. **Publish `/40`** with the new version/hash and updated T01/M1/A16 wording.
6. **Only then** start `/41`.

### 11.3 Integrator acceptance record (required before any edit)

| Authority | Name / role | Date | Decision |
|---|---|---|---|
| T01 integrator | **T01 integrator (master dispatcher, user-authorized)** | 2026-11-09 | **ACCEPTED** mandatory `/41` with OD-1..OD-10 defaults; version remap recorded above |
| C05/snapshot owner | T01 integrator acting for C05 | 2026-11-09 | **ACCEPTED** full append bodies, trace decoder/diff, `Task.id`, pin digest |
| T02/routing owner | T01 integrator acting for T02 | 2026-11-09 | **ACCEPTED** stage-first selection and zero-quota fail-closed |
| T13/VF13 owner (`/43`) | T13 track | 2026-11-09 | **DEFERRED** to `/43` with restore/projection |
| User | User instruction to proceed as master dispatcher | 2026-11-09 | **AUTHORIZED** the `/41` dispatch and freeze work |

**Acceptance is filled with a real who/when decision (2026-11-09).** The
original "no edit before acceptance" gate is satisfied.

### 11.4 Post-acceptance implementation evidence (2026-11-09)

The mandatory package was implemented as `t01-c01-c06/41` (hash
`500abb528a1e62622f6d327456d2e27f6bf1abdc1992e6b991e9dd3e0e832966`):

- **D-12**: `snapshot.rs` encodes every append handle and body in stored
  order; `contract.rs` freezes `G1_DRAFT_BODY_NAMES`, `APPEND_BATCH_FIELDS`,
  and `APPEND_BODY_STORE_NAMES`.
- **D-13**: stage-first `(stage, phase, enqueue, TaskId)` selection;
  `STAGE_ASSIGNMENT` is authoritative; `stage_of` is a bounded binary search;
  zero quota returns `CommitError::Limit(LimitError::InvalidInflightQuota)`
  before selection or mutation.
- **D-14**: macros count toward `max_records_total`; typed append bodies bump
  the owning store once per successful batch; `Names` bumps only on new names;
  `ResultRecord.version` is a 1-based global commit ordinal
  (`Control.next_commit_ordinal`); `stage_queue_bound` is explicitly reserved.
- **D-15**: `TraceRecord` carries `pins_digest`, canonical ID lists, and the
  commit error; `Trace::from_bytes`/`diff` provide bounded round-trip and
  first divergence; `Task.id` is encoded; `ReplayEvidence` is a Host-owned
  envelope. Replay is recorded-input re-execution, not restoration.

Tests: `c05_codec` 29, `c06_routing` 19, `c07_limits` 20, `c03_task` 48,
`c08_gate1` 22, `c11_lex` 11, `freeze` 15, `c20_ppscan` 8,
`h04_candidate` 18; full compiler suite 723 tests / 50 suites; root 21,
torture 73, chip-lint 16; formatting clean. Compiler clippy remains red
only in the D-04 parse drafts (137 pre-existing diagnostics, none in changed
files). Three candidate runs were byte-identical and emitted `/41` with
`value=05`.

Deferred to `/43`: full `Snapshot::from_bytes`/restore/continue, VF13
semantic/scheduler projections, quota>1 dispatch/commit order, per-stage
queues/bounds, `StorePatch` materialization/tombstone liveness, and full pin
bytes in bus records. D-09 tagged constants remain B2-02 `/42`. No target,
probe, torture, or full-M1 acceptance claim is made.

---

## 12. Open decisions

| ID | Decision | Accepted | Alternatives (preserved) | Authority |
|---|---|---|---|---|
| **OD-1** | D-13 selection key | Stage-first `(stage, group, enqueue, TaskId)`; amend T01 §4; `STAGE_ASSIGNMENT` authoritative | Keep group-first; amend M1 D5/ADR-0002/T02/M1 proposal; document stage table as worker-layer only | T01 integrator + T02 + M1 owner |
| **OD-2** | D-12 body encoding | Full canonical bodies, names for handle family/body variant, stored order; encode malformed length mismatch as-is | Body hash; numeric body tags; validate in encoder | T01/C05 |
| **OD-3** | D-14 `stage_queue_bound` | Reserve the field in `/41`; enforce via stage queues/projection in `/43`; split the rule | Implement projected per-stage counts now | T01 integrator |
| **OD-4** | D-14 zero quota | Remove clamp; typed fail-closed at `propagate`; validated entry points + `try_new` production path | Make `new` fallible; panic; keep clamp | T01/routing |
| **OD-5** | D-14 `ResultRecord.version` | 1-based global commit ordinal (`Control.next_commit_ordinal`) | `control.tick`; remove the field | T01/commit |
| **OD-6** | D-15 snapshot meaning | `/41` fingerprint + recorded-input replay; full restore `/43` | Move full restore into `/41`; keep the current overclaim | T01/C05 + T13 |
| **OD-7** | D-15 pins | Digest in `TraceRecord`; exact pins in Host envelope | `TickRecord.pins_digest`; full pin bytes in bus | T01/C05 |
| **OD-8** | D-15 `Task.id` | Encode `task.id` in `/41` and test divergence | Remove the redundant field; leave omitted | T01/C05 |
| **OD-9** | Freeze granularity | One `/41` for all mandatory items; `/43` deferred | Split `/41` into smaller freezes (higher coordination cost) | T01 integrator |
| **OD-10** | D8 commit order | Keep out of D-13; freeze with quota>1 in `/43` | Add `dispatch_ordinal` now | T02/T01/T13 |

---

## 13. Explicit exclusions and adjacent findings

These are **not** part of D-12..D-15 and are not authorized by this CCR. They
must not be silently bundled into the `/40` edit.

| Finding | Why excluded | Track |
|---|---|---|
| A23-F02/F03 (quota>1 report and commit origin) | Requires dispatch-ordinal/batch outcome; quota>1 is post-freeze | `/41` (B2-41-03) |
| A23-F04 (error path skips `TickRecord` while latching/advancing) | Tick lifecycle/error-report policy, not D-15 restore | T02/driver |
| A23-F06 (`StorePatch` replace/tombstone never applied) | A02-F02 materialization gap; no D-14 fix | `/41` (B2-41-05) |
| A23-F09 (VF01 liveness/continuation) | Verification-track liveness policy | T13/VF track |
| A23-F11 (`SYMBOL_KIND_NAMES` says "variable", codec says "object") | Vocabulary/hash consistency; adjacent P2 | hash/docs track |
| A23-F12 (`ParseContext`/later kind names/numeric tags not hashed) | Hash-scope policy; partially touched by new tables only | hash-scope track |
| A23-F14 (decoder unbounded reservations) | Applies to any new decoder; the `/40` trace decoder must use checked bounds, but the existing snapshot decoders are not rewritten here | codec hardening |
| A02-F01/F02/F03/F05/F07/F08/F10/F11 | Chip boundary, reference materialization, profile assembly, allowlist, optional artifacts, doc drift, target scalars, constructor metadata | separate Stage-B items (A02 SB-01..SB-10) |
| D-01..D-11 | Other ledger findings | their assigned B1/B2 items |

---

## 14. Evidence status and non-claims

- **Checks run by this CCR:** none. This is a documentation-only change request.
- **Tests NOT run:** `cargo test`, `cargo fmt`, `cargo clippy`, target probe,
  AArch64 cross-compile/link/run, torture corpus, H04/H05.
- **Evidence cited:** A02's targeted 168 + 14 tests at the same baseline
  (A02 §7.1), the dispatch ledger's workspace baseline, and the static
  inspections in A23/A16/A17. These are prior evidence, not re-executed here.
- **No claim:** universal correctness, restore capability, quota>1 equivalence,
  target/ABI validity, or torture pass rate.
- **No implementation:** this file does not change any source, test, contract,
  Cargo, `mod.rs`, ledger, or report.

---

## 15. Revision record

| Date | Change | Author |
|---|---|---|
| 2026-11-07 | Initial B2-00 draft: D-12 append bodies; D-13 stage-first vs group-first decision; D-14 macro total, stage-bound reservation, zero-quota fail-closed, typed append/name versions, result version; D-15 pin-aware trace, decoder/diff, snapshot `Task.id`, deferred restore; mandatory/deferred split; open decisions; freeze/inventory plan. No acceptance, no freeze, no edit. | B2-00 subagent |
