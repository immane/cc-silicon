# A17: T13 Verification and Evidence Contract Requirements

| Field | Value |
|---|---|
| Task | Stage A, `A-PTS-07` (T13 verification and evidence requirements) |
| Status | **Design proposal only. Not a contract freeze, not implementation authorization.** |
| Baseline | `t01-c01-c06/39` (`07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24`), branch `wave4-development`, HEAD `430ae29` |
| Scope | Compiler application (`compiler/`) verification records, task/result carriers, acceptance precedence, quotas, provenance, replay/differential requirements, and ordered freeze/implementation work orders |
| Write scope of this task | This file only. No source, tests, contracts, Cargo, `mod.rs`, shared indexes, or user files were edited. |
| Checks performed | Read-only static inspection of the paths cited below. No test, probe, corpus run, or pass rate was produced by this task. |
| Authority order | Explicit user instruction > accepted ADR > accepted engineering contract > architecture design > implementation (`AGENTS.md` §2). |

This document does not edit [T13](../../tasks/T13_VERIFICATION_CHIPS.md), [T01](../../tasks/T01_COMPILER_CONTRACT.md), `compiler/`, or any freeze artifact. It records requirements for the integrator to accept, reject, or split. Every proposed task kind, stage, chip ID, record family, numeric rule code, and limit below is **PROPOSED**, not frozen, unless explicitly marked **FROZEN `/39`**.

---

## 1. Summary and Recommendation

### 1.1 What this package recommends

1. **One shared verification record family, not nine.** Add a single `Verification` family with three record variants: `Fact`, `Result`, and `Evidence`. One family, one `VerificationId`, one `RecordRef` tag, one snapshot encoder. Do not create separate fact/result/evidence arenas.
2. **Verdicts stay on the existing result carrier.** `Ack` means PASS and `Proposal::Fail` plus one committed diagnostic means non-PASS. The `Verification::Result` record is durable audit/provenance, not a second verdict carrier. Do not add a `ResultValue::Verification` variant; `ResultValue` is frozen in `compiler/src/task.rs:1722-1739`.
3. **Facts are recorded only for the primary failure.** A PASS appends one `Result`; a non-PASS appends one `Result`, one primary `Fact`, and one diagnostic. This keeps record growth bounded and matches the existing "one primary diagnostic" rule.
4. **Evidence is Host-fed and typed.** Add `Verification::Evidence` with explicit `gate`, four `EvidenceSlot`s, and provenance hashes. This closes `DEFECT-VF14-01`/`DEFECT-VF14-02` from the frozen VF14 slice without reinterpreting `/39` tags.
5. **Precedence is total and shared:** `Invalid > Missing > Undecodable > Fail > Pass`. `Unsupported` is a typed `Fail` subtype, never a sixth level and never PASS. Within one level, the earliest canonical subject wins.
6. **VF13 replay is the first new verifier to freeze.** A draft already exists and M1 gates G9/G15 need it. It should freeze before VF02/VF03/VF04/VF07–VF11 because it establishes the semantic-projection/scheduler-snapshot split the others consume.
7. **Transient latch checks are Host harnesses, not ordinary chips.** The H6 batch latch state (`in_flight`, pre-recovery hash, wire reset) is not observable after latch under `/39`; VF02/VF03 cannot verify it from a later task. Split steady-state chip facts from Host latch evidence.
8. **VF07–VF11 stay out of M1.** CFG/SSA/machine/ABI records do not exist at `/39`; freezing their verifiers now would invent producer schemas. Freeze them only after T09/T10/T11 and the probe.
9. **No new `Limits` field.** Verification facts are bounded by `max_records_total`, diagnostics by `max_diagnostics`, replay by `max_ticks`, and evidence slots are fixed at four. A separate verification quota would duplicate existing bounds.
10. **A verification PASS is not a T00 PASS.** VF14 classifies one evidence instance. The suite rate belongs to T00/H09. No verifier may count `Unsupported`, missing, undecodable, crash, or timeout as PASS.

### 1.2 Frozen versus proposed

| Item | Status at `/39` | This document |
|---|---|---|
| `ResultValue` five-variant set (`Empty`, `Ack`, `Record`, `Records`, `Diagnostic`) | **FROZEN** (`compiler/src/task.rs:1722-1739`) | Reused unchanged. No new variant proposed. |
| `RecordRef` 28 variants, wire tags 0–27 | **FROZEN** (`compiler/src/ids.rs:119-178`, `:215-255`) | One append proposed: `Verification` tag 28. |
| `RECORD_KINDS` 28 entries | **FROZEN** (`compiler/src/task.rs:2185-2214`) | One append proposed: `verification`. |
| `RecordFamily` 28 ordinals | **FROZEN** (`compiler/src/ids.rs:343-400`) | One append proposed: `Verification` ordinal 28. |
| `StoreId` 21 entries; `Names` frozen index 20 | **FROZEN** (`compiler/src/task.rs:2081-2105`) | One append proposed: `Verification` index 21. |
| `DiagGroup` 9 variants | **FROZEN** (`compiler/src/diagnostic.rs:37-71`) | One append proposed: `Verification`; open alternative is a `Task` code subrange. |
| VF01, VF05, VF06, VF12, VF14 registrations | **FROZEN** (`/19`, `/18`, `/14`, `/17`, `/38`) | Preserved unchanged; no retrofit in `/40`. |
| VF02, VF03, VF04, VF07–VF11, VF13 | **NOT IMPLEMENTED / NOT FROZEN** | Proposed requirements only. |
| `Snapshot::capture` whole-state encoder and `Trace` | **FROZEN** (`compiler/src/snapshot.rs:522-1064`) | Reused; a semantic projection and scheduler snapshot are proposed as views, not new whole-state encoders. |
| `HostTestEvidence` record family, frozen instance schema, boolean pass/fail carrier | **ABSENT** (`DEFECT-VF14-01`, `DEFECT-VF14-02`, `VF_EVIDENCE_SLICE.md` §7) | Proposed `Verification::Evidence`. |
| H6/H9 batch fixtures (H6-M01..M16) | **REQUIRED / PENDING** (`T13` rev 4/5) | Preserved as pending; split into steady-state chip facts and Host latch evidence. |
| Target ABI/machine values | **UNVERIFIED** (`compiler/contracts/CONTRACT_VERSION`) | VF10/VF11 remain probe-gated; no ABI PASS is claimable. |

### 1.3 Non-claims

- No verification chip, task kind, record family, stage row, chip ID, rule code, or hash is frozen by this document.
- No test was run, no probe was executed, no torture case was compiled, and no pass rate is claimed.
- The VF13 draft remains **DRAFT ONLY**; this document does not promote it.
- The VF01/VF05/VF06/VF12/VF14 slices remain at their frozen scopes even where the T13 catalog row is broader. A frozen slice is not evidence that the full row is complete.

---

## 2. Current Baseline (static inspection, not test evidence)

### 2.1 Frozen verification chips at `/39`

| Chip | Task kind (group/local) | Stage | ChipId | Source | Test | Result carrier | Writes |
|---|---|---:|---:|---|---|---|---|
| VF01 `StoreInvariantChip` | `verification.store_invariant` (12/19) | 6 | 22 | `compiler/src/chips/verify/vf_store.rs` | `compiler/tests/c19_vf01.rs` | `Ack` / `Fail` | none |
| VF05 `TokenAstInvariantChip` | `verification.token_ast_invariant` (12/18) | 2 | 21 | `compiler/src/chips/verify/vf_syntax.rs` | `compiler/tests/c18_vf05.rs` | `Ack` / `Fail` | none |
| VF06 `TypedAstInvariantChip` | `verification.typed_invariant` (12/16) | 4 | 15 | `compiler/src/chips/verify/vf_invariant.rs` | `compiler/tests/c14_se.rs` | `Ack` / `Fail` | none |
| VF12 `IrInterpretChip` | `verification.ir_interpret` (12/17) | 6 | 20 | `compiler/src/chips/verify/vf_interpret.rs` | `compiler/tests/c17_vf12.rs` | `Record(Const)` (modeled value, not PASS) | none |
| VF14 `EvidenceClassifyChip` | `verification.evidence_classify` (12/20) | 6 | 56 | `compiler/src/chips/verify/vf_evidence.rs` | `compiler/tests/c38_vf14.rs` | `Ack` / `Fail` | none |

Stage rows are in `compiler/src/manifest.rs:1444-1514`; chip reservations are in `compiler/src/manifest.rs:1022-1170`. The next free `VERIFICATION` local code is 21 (T13 `/38` says owners start at 21), and the next free `ChipId` is 58 after `SE_FUNC_CHIP = 57`.

### 2.2 Existing carriers the design must reuse

| Carrier | Location | Relevant property |
|---|---|---|
| `ResultValue` | `compiler/src/task.rs:1722-1739` | Five variants; frozen note forbids a `DraftRecords` variant and says the set is unchanged. |
| `ResultRecord` | `compiler/src/task.rs:1741-1754` | `task`, `kind`, `value`, `version`, `consumed`; exactly-once consumption. |
| `DiagnosticRecord` / `DiagnosticDraft` | `compiler/src/diagnostic.rs:78-170` | Severity, `(DiagGroup, u16)`, message, optional span/task. |
| `Snapshot` | `compiler/src/snapshot.rs:522-879` | Whole-bus canonical bytes; includes control, queues, tasks, results, diagnostics, language stores, in-flight set, report, patches, versions, wires. |
| `Trace` / `TraceRecord` | `compiler/src/snapshot.rs:952-1064` | Per-tick `tick`, `selected`, `outcome`, `resolution`, `ready_len`, `enqueued`, `completed`, `failed`. |
| `ArtifactKind::Snapshot` / `Trace` | `compiler/src/bus.rs:79-105` | Byte artifacts with `source` optional and `raw_offsets` empty for these kinds; usable as Host-fed replay inputs. |
| `TaskStore` | `compiler/src/bus.rs:826-843` | Only `ready`, `active`, `in_flight`; no stage queues, dispatch cursor, or `PipelineMetrics`. |
| `TickMetrics` / `TickRecord` | `compiler/src/bus.rs:845-865` | Only `dispatched` count and selected projection; no per-stage metrics. |
| `Limits` | `compiler/src/limits.rs:30-101` | `max_records_total`, `max_diagnostics`, `max_ticks`, `max_proposals_per_tick`, `max_inflight_per_tick = 1`, `stage_queue_bound`. |

### 2.3 What does not exist at `/39`

- No `Verification` store, record family, ID, `RecordRef` variant, or snapshot encoder.
- No `HostTestEvidence` record family, frozen test-instance schema, or boolean pass/fail evidence carrier.
- No `dispatch_cursor`, `stage_assignment_version`, `stage_queues`, or `PipelineMetrics` on the bus, despite being named in the M1 projection/exclusion discussion.
- No typed machine, ABI, layout, init, or vreg record bodies; those stores are reserved/absent (`compiler/src/bus.rs`; `compiler/src/task.rs:2081-2105`).
- No semantic-projection or scheduler-snapshot function; `Snapshot::capture` is whole-state only.
- No per-tick projection history; `Trace` is Host-side, not a bus record.
- No VF02/VF03/VF04/VF07/VF08/VF09/VF10/VF11/VF13 implementation.

---

## 3. Goal, Scope, and Boundaries

### 3.1 Goal

Define the smallest shared verification fact/result/evidence contract that:

- covers store/ID integrity, syntax, typed AST, semantic facts, IR/CFG, ABI/machine output, replay, differential checks, and evidence classification;
- keeps verification results separate from producer implementation self-checks;
- gives every non-PASS a typed, deterministic, attributable reason;
- gives every PASS a bounded, replayable provenance record;
- can be frozen in small batches without rewriting the five frozen VF slices.

### 3.2 In scope

- Shared record shapes, task/result mapping, ownership, precedence, quotas, provenance, replay/differential rules.
- The registration requirements for the unimplemented VF chips.
- Conflict analysis against `/39` and the frozen VF slices.
- Ordered freeze and implementation work orders.

### 3.3 Out of scope

- Implementing chips, tests, records, or schema changes.
- Changing `compiler/`, `Cargo`, `mod.rs`, `CONTRACT_VERSION`, or any shared index.
- Proving that any verification rule is complete for all C programs.
- Target probe execution, ABI attestation, torture execution, or a pass rate.
- Formal proof; the T13 proof-obligation direction remains separate.

---

## 4. Separation Model: Verification Is Not a Self-Check

The contract must make the following separations explicit and testable:

| Separation | Rule |
|---|---|
| Producer vs verifier | A producer chip writes its own records/results. A verifier chip reads only committed records, results, diagnostics, and Host-fed Evidence. It never reads a producer's private output slot, draft, wire, or helper state. |
| Task ownership | A verification task kind belongs to `TaskGroup::VERIFICATION`; its `owner` is a VF chip, not the producer chip. Its result/diagnostic are attached to the verifier task, never to the producer task. |
| Write scope | A verifier writes only `verification.records` (its own facts/result) and never a producer store. Frozen VF01/VF05/VF06/VF12/VF14 are zero-write and stay zero-write. |
| Evidence | A producer `Ack`, a producer test, or a producer diagnostic is not evidence. Evidence is a Host-fed `Verification::Evidence` record with instance and provenance hashes, or the frozen VF14 six-ref seeded form. |
| Independent expected values | Verifier tests derive expected values from the specification, hand calculation, or same-target reference evidence, not from the verifier or producer implementation (`PARALLEL_EXECUTION.md` §4, `AGENTS.md` §4.7). |
| No helper delegation | A verifier may use mechanical helpers (IDs, bitvectors, interning, serialization) but must not call the producer's language-rule helper to decide the outcome. |
| Unsupported | `Unsupported` is a typed `Fail` subtype. It is never equal, never PASS, and never removed from a denominator. |
| Final rate | VF14 classifies one instance. T00/H09 owns the final compile/execute/ieee and per-file rates. A VF chip `Ack` is not a suite pass. |

A useful litmus test: if deleting the verifier chip would still leave the producer's tests green, the verifier was not adding independent evidence. The verifier must be able to fail when a producer's committed output is mutated independently of the producer's code.

---

## 5. Proposed Shared Verification Records (`/40+`)

### 5.1 One family, three variants

**PROPOSED:** add one committed family `verification` with these variants:

```text
VerificationRecord =
    Fact(VerificationFact)
  | Result(VerificationResult)
  | Evidence(TestEvidence)
```

Why one family:

- It minimizes schema cost: one `VerificationId`, one `RecordRef::Verification`, one `RecordFamily::Verification`, one `StoreId::Verification`, one snapshot encoder/decoder, one manifest field.
- It still gives typed, field-level records through the variant payload.
- It avoids three new ID families/arenas for one domain and avoids a second writer group for evidence.

**Rejected alternative:** three separate families (`VerificationFact`, `VerificationResult`, `TestEvidence`). It is clearer at the type level but triples the ID/arena/encoder/hash surface for no consumer benefit at M1. See §17.

### 5.2 Store, ID, and tag placement

| Item | Proposed value | Frozen constraint |
|---|---|---|
| Store | `StoreId::Verification` at index 21, fields `records` and `evidence` | `StoreId::Names` stays index 20 (`compiler/src/task.rs:2071-2078`); append-only after it. |
| ID | `VerificationId` (newtype `u32`, `u32::MAX` sentinel) | Same arena discipline as existing IDs; no reuse. |
| `RecordRef` | `RecordRef::Verification(VerificationId)` at wire tag 28 | Tags 0–27 are frozen; append-only. |
| `RecordFamily` | `RecordFamily::Verification` at ordinal 28 | Ordinals are a separate closed inventory; do not derive from wire tags. |
| `RECORD_KINDS` | append `"verification"` | Keep names, not discriminant positions. |
| Snapshot | one arena section with variant name + fields | Encode variant names, not raw discriminants. |
| Store owner | `TaskGroup::VERIFICATION` owns `records`; Host boundary owns `evidence` | See §7.3; this is the one owner-model ambiguity that must be clarified. |

The new family also requires updates in `compiler/src/ids.rs`, `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/commit.rs` (typed append materialization), `compiler/src/snapshot.rs`, `compiler/src/contract.rs`, and `compiler/tests/freeze.rs`. These are integrator-owned.

### 5.3 `VerificationOutcome`

```text
VerificationOutcome =
    Pass        // 0, lowest precedence
  | Fail        // 1
  | Undecodable // 2
  | Missing     // 3
  | Invalid     // 4, highest precedence
```

| Outcome | Meaning | Typical carrier |
|---|---|---|
| `Pass` | Every required fact is present, decodable, and positive; no observed failure anywhere. | `Ack` |
| `Fail` | A decoded fact is negative (wrong type, wrong range, divergence, failing evidence). | `Proposal::Fail` + diagnostic |
| `Undecodable` | A present carrier has no frozen meaning for this rule (unknown enum, `Record`/`Records` where a boolean is required). | `Proposal::Fail` + diagnostic |
| `Missing` | A required fact, record, stage, or provenance field is absent. | `Proposal::Fail` + diagnostic |
| `Invalid` | Protocol/gate/shape failure before classification: wrong task kind, non-`Running`, malformed payload, dangling subject, invalid gate. | `Proposal::Fail` + diagnostic |

`Unsupported` is not a sixth outcome. It is `Fail` with `DiagGroup::Unsupported` (or `DiagGroup::Verification` with an unsupported local code) and must never be reported as PASS.

### 5.4 `VerificationClass` and rule IDs

Classes align with the T13 verifier rows, not with abstract language phases:

| Class | Verifier(s) | Rule range (candidate) | Covers |
|---|---|---|---|
| `StoreId` | VF01 | `0x01xx` | ID ownership, dense bounds, record tags, span bounds, reserved stores |
| `Task` | VF02 | `0x02xx` | Ready/Waiting/Complete exclusivity, graph acyclicity, exactly-once completion |
| `Wire` | VF03 | `0x03xx` | Wire reset, proposal lifetime, persistence not on wire, latch |
| `Access` | VF04 | `0x04xx` | Manifest read/write/phase/task-guard audit; batch write-conflict residual |
| `Syntax` | VF05 | `0x05xx` | Token ranges/order, parent/child reciprocity, required fields, unique EOF |
| `TypedAst` | VF06 | `0x06xx` | Typed-fact completeness, value category, effects, required conversions |
| `SemanticFacts` | VF06/VF07 boundary | `0x07xx` | Call/return/lvalue/effect facts not covered by the typed-AST row |
| `IrCfg` | VF07, VF08 | `0x08xx` | Unique terminator, symmetric edges, target existence, operand/result types, memory width/effects |
| `Ssa` | VF09 | `0x09xx` | Dominance, phi edge count/type, single definition; non-SSA must not be falsely flagged |
| `Machine` | VF10 | `0x0Axx` | Use/def/classes/ties, fixed registers, spill slots, frame consistency |
| `Abi` | VF11 | `0x0Bxx` | Args/returns/SP alignment/callee saves/target identity |
| `Replay` | VF13 | `0x0Cxx` | Semantic projection equality, scheduler snapshot equality, first divergence |
| `Differential` | VF13/Host | `0x0Dxx` | Before/after optimization, cross-design, reference ABI/behavior, negative drift |
| `Evidence` | VF14 | `0x0Exx` | Gate completeness, stage prefix, undecodable carrier, missing/fail precedence |
| `Provenance` | all | `0x0Fxx` | Missing/invalid provenance, replay/evidence hash mismatch, contract mismatch |

`VerificationRuleId` is a `u16` with `(class << 8) | local`. The mapping from ID to stable string name is a new frozen table in `compiler/src/contract.rs` (candidate name `VERIFICATION_RULE_NAMES`). Existing VF01/VF05/VF06/VF12/VF14 rule strings keep their `vfNN.*` names to avoid churn; new rules use `vf.<class>.<rule>`. The numeric inventory is **OPEN** and requires T01 sign-off; do not invent final codes in chip code before the freeze.

### 5.5 `VerificationFact`

| Field | Type | Required | Meaning |
|---|---|---|---|
| `rule` | `VerificationRuleId` | yes | Stable rule identity; one rule per fact. |
| `subject` | `RecordRef` | yes | The committed record the rule is about. For a global invariant, the offending record, not the verifier task. |
| `outcome` | `VerificationOutcome` | yes | `Fail`, `Missing`, `Undecodable`, or `Pass` (Pass facts optional). |
| `detail` | `Option<DiagnosticId>` | no | Committed diagnostic with message/location. Required for non-PASS in practice; absent for a Pass fact. |

A `Fact` is a **primary** fact only in the minimal design. Additional per-rule facts are allowed but bounded by the normal arena/total limits. A verifier that emits a `Fact` with `Pass` and no diagnostic is valid but optional; the aggregate `Result` is authoritative.

### 5.6 `VerificationResult`

| Field | Type | Required | Meaning |
|---|---|---|---|
| `task` | `TaskId` | yes | The verifier task that produced the result. |
| `verifier` | `ChipId` | yes | The VF chip ID; must match the task owner. |
| `class` | `VerificationClass` | yes | The verifier family. |
| `subject` | `Option<RecordRef>` | no | Primary subject; `None` only for a global check. |
| `outcome` | `VerificationOutcome` | yes | Aggregate verdict; must agree with the `Ack`/`Fail` proposal and the primary fact. |
| `facts` | `Vec<VerificationId>` | yes (may be empty) | Fact records referenced by this result; each must resolve to a `Fact` variant. |
| `first_failure` | `Option<VerificationId>` | no | Required when `outcome != Pass` or when `facts` contains a non-PASS fact. Must be in `facts`. |
| `tick` | `u64` | yes | Deterministic bus tick at result production; not wall-clock. |
| `snapshot_hash` | `[u8; 32]` | yes | Whole-snapshot hash at task dispatch, before the verifier appends. Projected by the adapter; a chip cannot compute it. |
| `trace_hash` | `Option<[u8; 32]>` | no | Semantic-trace hash when a trace is part of the evidence. |
| `comparison` | `Option<ComparisonSummary>` | no | Required for `Replay` and `Differential`; forbidden otherwise. |
| `evidence` | `Option<VerificationId>` | no | Required for the Host evidence classifier; must resolve to an `Evidence` variant. |

The `Result` is **audit/provenance**, not a second verdict. A consumer that needs the verdict reads the task state (`Completed`/`Failed`) and the `Result.outcome` must agree. A disagreement is an `Invalid` verifier defect, not a PASS.

### 5.7 `TestEvidence`

| Field | Type | Required | Meaning |
|---|---|---|---|
| `case` | `NameId` | yes | Frozen test-instance / upstream case name. |
| `source_hash` | `[u8; 32]` | yes | Hash of the exact test source bytes. |
| `target` | `NameId` | yes | Frozen target identity label, e.g. `aarch64-unknown-linux-gnu`. |
| `dialect` | `NameId` | yes | Frozen `-std`/GNU mode label. |
| `options_hash` | `[u8; 32]` | yes | Hash of the exact option vector, including include paths and defines. |
| `gate` | `EvidenceGate` (`Compile`, `Link`, `Run`, `Check`) | yes | Required pipeline prefix. |
| `compile` | `EvidenceSlot` | yes | Compile-stage evidence. |
| `link` | `EvidenceSlot` | yes | Link-stage evidence. |
| `run` | `EvidenceSlot` | yes | Run-stage evidence. |
| `check` | `EvidenceSlot` | yes | Assertion/check-stage evidence. |
| `toolchain_hash` | `[u8; 32]` | yes | Assembler/linker/sysroot/reference-runner versions and flags. |
| `environment_hash` | `[u8; 32]` | yes | OS/VM/sysroot/environment identity. |
| `command_hash` | `[u8; 32]` | yes | Exact candidate/reference command lines. |
| `log_hash` | `[u8; 32]` | yes | Raw stdout/stderr/log digest; raw logs stay Host-side. |
| `probe_report_hash` | `Option<[u8; 32]>` | conditional | Required for any ABI/machine/target-execution claim; absent only for compile-only symbolic evidence. |

`EvidenceSlot`:

```text
EvidenceSlot =
    Missing
  | Pass
  | Fail { diagnostic: Option<DiagnosticId> }
  | Undecodable { carrier: RecordRef }
```

`EvidenceGate` and `EvidenceSlot` replace raw bytes/booleans with typed, snapshot-stable names. This directly addresses `DEFECT-VF14-02` (no frozen boolean carrier) without treating `Record`/`Records` as pass.

### 5.8 `ComparisonSummary`

| Field | Type | Required | Meaning |
|---|---|---|---|
| `kind` | `ComparisonKind` (`Replay`, `Differential`) | yes | Which comparison produced this summary. |
| `cycles` | `u64` | yes | Number of ticks/steps compared. |
| `semantic_chain_a` | `[u8; 32]` | yes | Rolling digest of the canonical semantic projection for run A. |
| `semantic_chain_b` | `[u8; 32]` | yes | Rolling digest for run B. |
| `scheduler_chain_a` | `Option<[u8; 32]>` | replay only | Rolling digest of the scheduler snapshot for run A. |
| `scheduler_chain_b` | `Option<[u8; 32]>` | replay only | Rolling digest for run B. |
| `baseline` | `Option<RecordRef>` | differential | Baseline artifact/evidence record. |
| `candidate` | `Option<RecordRef>` | differential | Candidate artifact/evidence record. |
| `first_divergence` | `Option<Divergence>` | no | Required when any chain differs. |

```text
Divergence {
    track: Semantic | Scheduler,
    tick: u64,
    subject: Option<RecordRef>,
    offset: u64, // byte offset in the canonical projection, deterministic
}
```

A rolling chain, not a per-tick snapshot array, is the minimal carrier: it detects divergence and identifies the first tick when the Host supplies per-tick digests or re-runs to localize. The exact localization mechanism is **OPEN** (§16); do not require the chip to store `max_ticks` snapshots.

### 5.9 Schema cost

| Change | File(s) | Effect |
|---|---|---|
| `VerificationId`, `RecordRef::Verification`, `RecordFamily::Verification` | `compiler/src/ids.rs` | New variant/tag 28/ordinal 28; exhaustive matches updated. |
| `VerificationRecord` and payloads; `Arenas.verification` | `compiler/src/bus.rs` | New typed arena; body encoding. |
| `StoreId::Verification`, `RECORD_KINDS` append, task kinds/registries | `compiler/src/task.rs` | New store index 21; new kinds; new slice registries. |
| Chip IDs, allowlist, stage rows, slice predicates | `compiler/src/manifest.rs` | New `ChipId`s 58+; `STORE_OWNER_ALLOWLIST` rows; `STAGE_ASSIGNMENT` rows. |
| `DiagGroup::Verification` and codes | `compiler/src/diagnostic.rs` | New group; snapshot name/hash. |
| Typed append materialization | `compiler/src/commit.rs` | `Verification` family append/predicted-ref handling. |
| Snapshot encoder/decoder; projection views | `compiler/src/snapshot.rs` | New arena section; semantic/scheduler projection functions. |
| Rule table, names, version/hash | `compiler/src/contract.rs`, `compiler/contracts/CONTRACT_VERSION` | New normative rules; hash bump; `/39` preserved as history. |
| Freeze pins | `compiler/tests/freeze.rs` | Version/hash/rule/registry counts. |
| Manifest docs | `compiler/contracts/COMPILER_SFL_MANIFEST.md`, `docs/tasks/T01_COMPILER_CONTRACT.md` §7.1 | New store/fields; no silent schema divergence. |

This is a larger change than a chip-local slice. It must be one integrator-owned freeze, not a chip owner's edit.

---

## 6. Task, Result, and Verifier Table

### 6.1 Frozen VF carriers (do not change in `/40`)

| VF | Task kind | Payload | PASS carrier | Non-PASS carrier | Note |
|---|---|---|---|---|---|
| VF01 | `verification.store_invariant` | empty | `Ack` | `Fail` | Global snapshot check; no writes. |
| VF05 | `verification.token_ast_invariant` | `[Node]` (TU) | `Ack` | `Fail` | Closed 8-kind M1 tree. |
| VF06 | `verification.typed_invariant` | `[Node]` (TU) | `Ack` | `Fail` | M1 checked set `{IntLiteral, BinaryAdd, Return}`. |
| VF12 | `verification.ir_interpret` | `[Function]` | `Record(Const)` | `Fail` | `Record` is a modeled value, **not** a PASS. |
| VF14 | `verification.evidence_classify` | six refs `[instance, gate, compile, link, run, check]` | `Ack` | `Fail` | Frozen M1-scope carrier map; `Record`/`Records` are `Undecodable`. |

### 6.2 Proposed VF registrations

All rows are **PROPOSED**; IDs/stages are candidate allocations, not frozen.

| VF | Proposed task kind (group/local) | Proposed stage | Proposed ChipId | Payload | Result | Writes | Freeze |
|---|---|---:|---:|---|---|---|---|
| VF13 | `verification.replay_compare` (12/21) | 6 | 58 | `[baseline artifacts, candidate artifacts]` (proposed 2 or 4 `Artifact` refs) | `Ack` / `Fail` + `Result{comparison}` | `Verification/records` | First new freeze (draft exists). |
| VF14-Host | `verification.evidence_classify_host` (12/22) | 6 | 59 | `[Verification::Evidence]` | `Ack` / `Fail` + `Result{evidence}` | `Verification/records` | After H04/H05 evidence schema. |
| VF02 | `verification.task_invariant` (12/23) | 6 | 60 | empty (global task graph) | `Ack` / `Fail` + `Result` | `Verification/records` | After T02/H6 freeze for batch rows; steady-state can freeze earlier. |
| VF03 | `verification.wire_lifetime` (12/24) | 6 | 61 | `[pre snapshot artifact, post snapshot artifact]` | `Ack` / `Fail` + `Result` | `Verification/records` | After pre/post capture decision; see §13. |
| VF04 | `verification.access_contract` (12/25) | 6 | 62 | empty or `[ChipTrace]` (no trace family today) | `Ack` / `Fail` + `Result` | `Verification/records` | After T02 patch-log/manifest audit; batch conflict remains residual. |
| VF07 | `verification.cfg_invariant` (12/26) | 6 | 63 | `[Function]` | `Ack` / `Fail` + `Result` | `Verification/records` | Non-M1; after T09 CFG freeze. |
| VF08 | `verification.ir_invariant` (12/27) | 6 | 64 | `[Function]` | `Ack` / `Fail` + `Result` | `Verification/records` | Non-M1; after T09 wider IR. |
| VF09 | `verification.ssa_invariant` (12/28) | 7 (candidate) | 65 | `[Function]` | `Ack` / `Fail` + `Result` | `Verification/records` | Non-M1; after T10 SSA freeze. |
| VF10 | `verification.machine_invariant` (12/29) | 8 (candidate) | 66 | `[Function]` | `Ack` / `Fail` + `Result` | `Verification/records` | Non-M1; after T11 machine records. |
| VF11 | `verification.abi_invariant` (12/30) | 8/9 (candidate) | 67 | `[Function]` | `Ack` / `Fail` + `Result` | `Verification/records` | Non-M1; probe-gated. |

`STAGE_COUNT` is 11 (`compiler/src/limits.rs:21`), so ordinals 6–10 exist numerically, but the meaning of stages 7–10 is not frozen. The integrator must assign non-M1 stages with T09/T10/T11, not by this table.

### 6.3 Result/verdict rules per class

| Class | PASS requires | FAIL examples | Missing examples | Undecodable examples |
|---|---|---|---|---|
| `StoreId` | All refs/parents/continuations resolve; spans in bounds; reserved stores empty; no tombstone misuse. | Dangling ref, cross-family ID, span escape. | No source for a span; absent required arena. | Unknown record tag; unsupported HostRequest ref in M1. |
| `Syntax` | Parent/child reciprocity, ordered non-overlapping ranges, per-kind fields, unique EOF. | Wrong child count, overlapping siblings, bad literal origin. | Missing token/literal/EOF. | Unknown token/kind outside the frozen set. |
| `TypedAst` | Every lowerable node has one typed fact; type/category/effects legal; required conversions present. | Wrong type, lvalue misuse, nonzero effect in M1, missing required conversion. | Missing `SemRecord`; missing signature. | Unchecked node; non-M1 conversion shape (explicit unsupported). |
| `SemanticFacts` | Calls/returns/effects consistent with typed AST and target model. | Wrong call signature, missing return conversion, effect mismatch. | Missing callee facts. | Unsupported ABI/aggregate fact. |
| `IrCfg` | Unique terminator, symmetric edges, existing targets, legal op domain, operand/result types. | Dangling branch, re-terminated block, wrong signedness. | Missing block/function. | Op outside covered set. |
| `Ssa` | Defs dominate uses, phi arity/types, single definition. | Non-dominating use, missing incoming, duplicate def. | Missing phi/edge. | Non-SSA function (must be explicitly out of scope, not FAIL by accident). |
| `Machine` | Use/def/classes/ties/fixed regs/spills/frame consistent. | earlyclobber conflict, call-live unsaved. | Missing liveness/frame. | Feature/instruction outside covered set. |
| `Abi` | Args/returns/SP/saves/target identity match the probe-verified model. | Darwin plan on Linux, variadic misalignment. | Missing ABI plan/probe hash. | Unverified ABI value. |
| `Replay` | Semantic chain and scheduler chain both equal; zero divergence. | Any chain mismatch; first divergence recorded. | One run absent/incomplete; no ticks; no provenance. | Malformed artifact/trace. |
| `Differential` | Declared equivalence holds on the covered domain; no unsupported. | Semantic/output divergence; wrong mixed-link result. | Baseline/candidate absent. | Unsupported op; non-comparable target. |
| `Evidence` | Required gate prefix present and passing; no present failure anywhere; provenance complete. | Compile-ok/run-fail; earliest failing stage. | Required stage missing. | `Record`/`Records` where a boolean is required; invalid gate. |

---

## 7. Verifier Ownership and Producer/Consumer Matrix

### 7.1 Producer -> verifier -> consumer

| Fact/record | Producer | Verifier(s) | Consumer | Notes |
|---|---|---|---|---|
| Sources, spans, expansions | T03 PP | VF01, VF05 | VF13, T00 | Committed only. |
| PP tokens, macros | T03 PP | VF01, VF05 | VF13, T00 | VF05 uses committed C tokens/literals, not raw PP. |
| C tokens, literals | T04 LX | VF01, VF05 | VF06, VF12, VF13 | Literal origin links are checked reverse-only in VF05. |
| AST nodes | T05 PA | VF05, VF06 | VF12, VF13 | No parent/child trust from walk order. |
| Types, symbols, scopes | T06 TY | VF01, VF06 | VF12, VF13 | VF06 checks the M1 canonical `int` and typed facts. |
| `SemRecord`s | T07 SE | VF06 | T09, VF13 | One per checked node; no conversion field at `/14`. |
| Constants | T08 CL | VF01, VF12 | T09, VF13 | T09 consumes the committed `Const`; VF12 models it. |
| IR functions/blocks/values/instructions | T09 IR | VF01, VF07, VF08, VF12 | VF13, T10/T11 | M1 IR is one entry block, `Constant` + `Return`. |
| Optimization output | T10 OP | VF08, VF09, VF12 (differential) | VF13 | No T10 pipeline at `/39`; blocked. |
| Machine/ABI/frame/assembly | T11 CG | VF10, VF11 | T00 runner | No machine/ABI records at `/39`; probe-gated. |
| Task/result/diagnostic graph | T02 CT / commit | VF02, VF13 | T00, audit | VF02 steady-state only; transient latch is Host. |
| Wires/proposals/reset | T02 CT | VF03 | T00, audit | Pre/post snapshots are Host-fed artifacts. |
| Patch log/manifests/routing | T01/T02 | VF04 | T00, audit | Read-set monitoring remains a known limitation. |
| Host compile/link/run/check evidence | H04/H05/H07/H09 | VF14-Host | T00 | Host owns I/O, isolation, timeouts, and raw logs. |

### 7.2 Verifier -> writes -> consumers

| Verifier | Writes | Consumers | Forbidden writes |
|---|---|---|---|
| VF01 | none | audit, VF13 | all producer stores |
| VF05 | none | audit, VF13 | all producer stores |
| VF06 | none | T09 gate, audit, VF13 | `Sem/records` |
| VF12 | none | differential harness, audit | `Ir/*`, `Constants/*` |
| VF14 | none (frozen) | T00, Host | evidence slot records |
| VF13 | `Verification/records` (`Fact`, `Result`) | audit, T00 | producer stores, evidence |
| VF14-Host | `Verification/records` (`Result`) | T00 | `Verification/evidence` (Host owns it) |
| VF02 | `Verification/records` (`Fact`, `Result`) | audit, T00 | `Tasks/*`, `Wires/*` |
| VF03 | `Verification/records` (`Fact`, `Result`) | audit, T00 | `Wires/*`, `Tasks/*` |
| VF04 | `Verification/records` (`Fact`, `Result`) | audit, T00 | manifests, patch log |
| VF07/VF08 | `Verification/records` (`Fact`, `Result`) | T10/T11 gate, audit | `Ir/*` |
| VF09 | `Verification/records` (`Fact`, `Result`) | T10 gate, audit | `Opt/*` |
| VF10/VF11 | `Verification/records` (`Fact`, `Result`) | T00, audit | `Machine/*`, `Ext/*` |

### 7.3 Ownership and allowlist

`STORE_OWNER_ALLOWLIST` currently authorizes multiple chips in the same group to append one field (for example `SE_LIT_CHIP`, `SE_BIN_CHIP`, `SE_RET_CHIP`, `SE_FUNC_CHIP` all write `Sem/records`; `LX12_CHIP`/`LX13_CHIP` write `Lex/literals`; `PP_SCAN_CHIP`, `PP09_CHIP`, `PP12_CHIP`, `PP16_CHIP`, `PP24_CHIP` write `Pp/tokens`). The guardrail phrase "exactly one owner per store/field/record family" (`COMPILER_DEVELOPMENT_GUARDRAILS.md` §2.7) and the T02 phrase "each field has exactly one owner" must be read consistently with this existing practice.

**Required clarification:** define "owner" as the **owner group** for append-only fields, with per-`(chip, kind)` allowlist rows. If the integrator instead requires one chip writer per field, the single `Verification/records` field is invalid and three per-chip fields (or three families) are required. This is `OPEN` (§16, OQ-5). The recommended interpretation is owner-group ownership, matching the frozen `Sem/records` practice.

The `Verification/evidence` field is different: it is Host-fed, not chip-written. It must be appended through the trusted Host boundary (T01 C01's trusted integration/host boundary) or a future Host-group chip with its own allowlist row. A verifier chip must never write it.

`check_store_owner_allowlist` only enforces rows for manifests that claim a known slice kind (`compiler/src/manifest.rs:1371-1435`). Adding VF kinds without adding `is_vf_*_slice_kind` predicates would silently skip the check. Every new VF kind must have a slice predicate and an allowlist row.

---

## 8. Acceptance Levels, Precedence, and Failure Rules

### 8.1 Verification levels

| Level | Name | Requirement | Carrier |
|---|---|---|---|
| L0 | Protocol | Task kind/state/payload shape valid; subject resolvable; gate decodable; provenance present. | `Invalid` -> `Fail` |
| L1 | Presence | Every required fact/record/stage is present. | `Missing` -> `Fail` |
| L2 | Decodability | Every present carrier has a frozen meaning for this rule. | `Undecodable` -> `Fail` |
| L3 | Rule | Decoded facts satisfy the rule. | `Fail` -> `Fail` |
| L4 | Pass | All required facts hold; no observed failure; no unsupported. | `Ack` |

Only L4 may be reported as PASS. A verifier that cannot reach L0/L1/L2 must not skip to PASS. `Unsupported` is L3 `Fail` with an unsupported code.

### 8.2 Precedence

Across classes, the higher rank wins regardless of subject order:

```text
Invalid (4) > Missing (3) > Undecodable (2) > Fail (1) > Pass (0)
```

Within one rank, the earliest canonical subject wins:

1. pipeline stage ordinal (lower stage first);
2. record family/ID ascending order (`StoreId` canonical order, then `RecordRef` wire tag, then arena index);
3. field slot order for evidence (`compile`, `link`, `run`, `check`);
4. fact index for same-subject facts.

`Missing` is evaluated only over **required** facts/stages. A missing optional slot is ignored (VF14's "missing past the gate is not required" rule). `Undecodable` and `Fail` are evaluated over **all present** slots, including slots past the gate (VF14's "present failures count anywhere" rule). This is the generalization of `classify_vf14_evidence` (`compiler/src/chips/verify/vf_evidence.rs:224-259`) and must not diverge from it for VF14-compatible inputs.

### 8.3 Carrier mapping

| Carrier | Meaning for a verification fact/result | Must not be read as |
|---|---|---|
| `Ack` | PASS (L4). | A T00 suite pass. |
| `Proposal::Fail` + `Diagnostic` | Non-PASS with typed reason. | A producer success. |
| `ResultValue::Diagnostic` | Committed failure evidence in an evidence slot. | A missing slot. |
| `ResultValue::Empty` | Absent slot. | PASS. |
| `ResultValue::Record` | A produced record; for VF12 it is the modeled value. | PASS unless the task kind explicitly says PASS. |
| `ResultValue::Records` | Several produced records. | PASS unless the task kind explicitly says PASS. |
| `Unsupported` diagnostic | Explicit unsupported capability (L3 Fail). | Equal, skip, or PASS. |

For VF14-compatible evidence slots, `Record`/`Records` are `Undecodable` (frozen `/38` rule). The new `Verification::Evidence` variant must be added as a decodable evidence source in the new classifier, but must not reinterpret existing tags or make legacy `Record`/`Records` pass.

### 8.4 Failure rules

1. **Exactly one terminal transition per verification task.** `Complete` or `Fail`, never both; no `Progress`/`Await*` in the M1 verifier set. Batch/Progress only if a later H6 freeze explicitly authorizes it.
2. **Exactly one primary diagnostic per non-PASS.** Additional diagnostics are allowed only for independent secondary faults, not for restating the same failure.
3. **No silent pass.** A verifier that reads zero required subjects is `Missing`, not PASS. Global invariants (VF01 reserved stores, VF04 manifest audit) may pass vacuously only when the class declares a global/vacuous rule explicitly.
4. **No unsupported-as-pass.** `Unsupported` never maps to `Ack`.
5. **Capacity is a failure, not a pass.** If appending the `Result`/`Fact` or the diagnostic would exceed limits, the verifier must `Fail` with a capacity diagnostic; it must not silently drop the audit record.
6. **Provenance is required for PASS.** A PASS without `snapshot_hash` (and, for replay/differential, comparison chains) is `Missing`/`Invalid`, not PASS.
7. **Mismatch is invalid.** `Result.outcome` disagreeing with the task state or the primary fact is an `Invalid` verifier defect.
8. **Determinism.** Same committed input and same config produce the same verdict, same primary fact, same diagnostic order, and same `Result` bytes. No hash-map iteration, pointer address, wall-clock, PID, or absolute path in semantic fields.
9. **Loud negatives.** Wrong task kind, non-`Running`, malformed payload, dangling subject, unknown rule, or wrong chip owner fails loudly and never mutates producer state.
10. **No producer writes.** A verifier read error must not be repaired by writing the producer store.

### 8.5 Delivery acceptance levels

The dispatch ledger's states map to verification evidence as follows:

| State | Verification evidence required |
|---|---|
| `UNIT_ACCEPTED` | Non-frozen schema; independent fixtures for normal/boundary/invalid/unsupported/replay/write-scope; expected values independent; no self-verification. |
| `INTEGRATION_PENDING` | Real producer chain exists; verifier consumes committed records, not fixtures; provenance captured. |
| `INTEGRATED` | Kind/chip/stage/manifest/allowlist/route frozen; snapshot/hash/replay deterministic; no undeclared writes. |
| `CLOSED_LOOP_ACCEPTED` | Real source -> IR -> target object -> external assembler/linker -> run evidence; VF14 PASS on the frozen instance; no missing denominator; probe report hash present for ABI/machine claims. |

A verifier passing on synthetic fixtures is `UNIT_ACCEPTED`, not `INTEGRATED` and not a T00 pass.

---

## 9. Quotas

No new `Limits` field is proposed. Quotas are constraints on coverage, scheduling, and evidence completeness, bounded by existing limits.

| ID | Quota | Minimum / rule | Failure if not met |
|---|---|---|---|
| Q1 | Fixture quota per verifier | ≥1 normal, ≥1 boundary, ≥1 invalid, ≥1 unsupported/recovery, ≥1 replay, ≥1 unauthorized-field-unchanged; plus every T13 row case. | `UNIT_ACCEPTED` withheld. |
| Q2 | Rule coverage | Every frozen `VerificationRuleId` has ≥1 positive and ≥1 negative/mutation fixture. No sampling. | Freeze rejected. |
| Q3 | Evidence quota | Every frozen instance × every configuration in the frozen matrix classified exactly once. Missing/undecodable/fail/crash/timeout stay in the denominator. Empty denominator = FAIL. | VF14/gate FAIL. |
| Q4 | Replay quota | ≥2 identical runs per replay fixture; ≥3 consecutive runs for the final gate profile (H09); ≥1 forced semantic drift and ≥1 scheduler-only drift negative per track. | VF13 PASS withheld. |
| Q5 | Differential quota | 100% of frozen corpus instances for the final gate; per optimization class ≥1 before/after pair and ≥1 unsupported case. Unsupported never equal. | Differential PASS withheld. |
| Q6 | Batch quota | H6-M01..H6-M16 all required, no sampling; quota 1 and quota >1 (when enabled) both covered. | Batch acceptance withheld. |
| Q7 | Resource quota | Facts bounded by `max_records_total`; diagnostics by `max_diagnostics`; replay cycles by `max_ticks`; evidence slots fixed at 4; no unbounded per-tick scan. | Capacity diagnostic; FAIL. |
| Q8 | Provenance quota | Every PASS has contract/snapshot provenance; every replay/differential PASS has both chains; every ABI/machine PASS has `probe_report_hash`. | `Missing`/`Invalid`; never PASS. |
| Q9 | Scheduling quota | Verification tasks obey `max_inflight_per_tick`; no separate dispatch quota. Global verifiers run at defined checkpoints, not every tick. No duplicate verifier for the same `(kind, subject, version)`. | Dispatcher/duplicate-selection error; not a semantic PASS. |
| Q10 | No-empty-pass | A subject-scoped verifier with zero subjects is `Missing`; only an explicitly global rule may pass vacuously. | MISSING -> FAIL. |
| Q11 | Evidence retention | Raw logs/artifacts retained by Host by hash; no raw paths, credentials, or environment secrets in the bus record. | Security defect; evidence rejected. |

**Scheduling detail.** A whole-store verifier such as VF01 is O(records). The contract must prohibit running it every tick; it runs at a checkpoint or Host request. This is a correctness/performance guardrail, not a new quota. The exact checkpoint (stage barrier vs Host request) is `OPEN`.

---

## 10. Provenance

### 10.1 Verification provenance (semantic)

| Field | Where | Rule |
|---|---|---|
| Contract version/hash | Global config/snapshot | Must equal the frozen artifact under test. A mismatch is `Invalid`. |
| Whole-snapshot hash | `VerificationResult.snapshot_hash` | Captured at dispatch before the verifier appends; adapter-projected; a chip cannot compute it. |
| Semantic trace hash | `VerificationResult.trace_hash` | Optional except replay/differential. |
| Verifier identity | `task`, `verifier`, `class` | Must match the task owner and kind. |
| Subject | `subject`, `Fact.subject` | Committed record refs; no draft/wire/address. |
| Rule | `Fact.rule` | Stable ID + frozen name; one rule per fact. |
| Tick | `tick` | Bus tick, not wall-clock. |
| Diagnostic | `Fact.detail` | Committed diagnostic with code/span/task. |

### 10.2 Host evidence provenance

| Field | Rule |
|---|---|
| `case` + `source_hash` | Frozen instance and exact source bytes; upstream GCC case provenance/licence retained by Host/T00. |
| `target` + `dialect` + `options_hash` | Frozen target identity, mode, and exact option vector; no inferred defaults. |
| `toolchain_hash` | Assembler, linker, sysroot, reference compiler, runner versions/flags. |
| `environment_hash` | OS/VM/CI/sysroot identity; no secrets. |
| `command_hash` | Exact candidate/reference command lines. |
| `log_hash` | Raw stdout/stderr/log digest; raw logs stay Host-side. |
| `probe_report_hash` | Required for ABI/machine/target-execution claims; absent only for symbolic/compile-only evidence. |
| Timestamps / PIDs / paths | **Excluded** from semantic comparison and the bus record. Host may retain them outside the comparison projection. |

### 10.3 Determinism and exclusions

- No host pointer, hash-map iteration, wall-clock, PID, random seed, absolute path, or environment dump participates in a semantic hash.
- Stable ordering: `StoreId::ALL` order, `RecordRef` wire tag, arena index, field slot order.
- Host non-semantic fields are explicitly excluded from the semantic projection; the exclusion list is a co-freeze item (§11.3).
- A PASS with missing provenance is `Missing`, not PASS. This prevents an unverifiable "green" record.

---

## 11. Replay Requirements (VF13)

### 11.1 Inputs and outputs

**Input:** two runs of the same pipeline from identical initial state, config, and pins, at `max_inflight_per_tick = 1` for the quota-1 subset. The Host supplies per-tick canonical projections or rolling digests as `Artifact` records (`ArtifactKind::Snapshot` and `ArtifactKind::Trace`), or a frozen equivalent. The chip does not run the pipeline and does not read files.

**Output:** `Ack` on equality, `Fail` on divergence, plus one `VerificationResult` with `class = Replay` and `comparison = Some(ComparisonSummary)`.

### 11.2 Two separate tracks

| Track | Compared surface | Excluded |
|---|---|---|
| Semantic projection | Language-store records, committed results, diagnostics, terminal task facts, intern table, semantic trace. | Scheduler-only registers: stage queues, in-flight set, dispatch cursor, `stage_assignment_version`, `PipelineMetrics`, ready/active scheduling views, per-tick wires, store versions, enqueue/diagnostic ordinals. |
| Scheduler snapshot | `control`, `tasks.ready`, `tasks.active`, `tasks.in_flight`, stage queues (when they exist), dispatch cursor (when it exists), `stage_assignment_version` (when it exists), `report`/`PipelineMetrics` (when they exist), store versions, wires. | Language-store bodies, results, diagnostics (covered by the semantic track). |

Both tracks must hold for PASS. A scheduler-only drift is a scheduler-track `Fail`, even if the semantic track is equal. The VF13 draft's `R1-4` "scheduler-only drift does not fail the semantic track" is a track-independence assertion, **not** an overall PASS; this document clarifies that overall PASS requires both.

The exact inclusion/exclusion list is `OPEN` (§16, OQ-8). The table above is the recommended starting point, grounded in `M1_VERTICAL_SLICE_ACCEPTANCE.md` §6.3/§G9/§G15, not a frozen list.

### 11.3 Tick-for-tick requirement

- Comparing only the final `Snapshot` is necessary but not sufficient; it misses an intermediate divergence that later converges.
- The Host must capture per-tick projection digests or artifacts. The minimal carrier is a rolling chain: `chain_t = H(chain_{t-1} || canonical_projection_t)`, with `chain_0` a frozen domain separator. Store `semantic_chain_*` and `scheduler_chain_*` in `ComparisonSummary`.
- `first_divergence.tick` is found by comparing per-tick digests; the Host may need to retain the first divergent tick's raw projection or re-run to localize. The chip validates chain length, tick order, and first mismatch; it does not store `max_ticks` snapshots.
- `cycles` must equal both runs' tick counts; a short/absent run is `Missing`, not PASS.

### 11.4 Replay pass/fail rules

| Case | Verdict |
|---|---|
| Both chains equal for the full tick count; provenance complete | `Pass` (`Ack`) |
| Semantic chain differs | `Fail`; semantic divergence at first differing tick |
| Scheduler chain differs only | `Fail`; scheduler divergence at first differing tick |
| One run incomplete / no tick count | `Missing` |
| Malformed/undecodable artifact | `Undecodable` |
| Wrong contract/target/projection version | `Invalid` |
| Non-covered shape / unsupported projection | `Fail` (unsupported), never equal |
| Forced semantic drift in a negative fixture | Must `Fail`; a PASS is a verifier defect |

### 11.5 H6-M12 status

`H6-M12` ("Quota1 semantic projection vs separate scheduler snapshot") remains **REQUIRED / PENDING**. A quota-1 VF13 freeze implements the subset but does **not** satisfy H6-M12 or the quota>1/cross-dispatch-order extension. The batch replay fixture remains `T01/T02/T13` co-freeze pending.

### 11.6 Existing harness gap

`compiler/tests/c17_vf12.rs:654` and `compiler/tests/c19_vf01.rs:626` compare a **final** `Snapshot::capture` across two runs; they do not capture per-tick projections. VF13 must not claim tick-for-tick replay by reusing that pattern unchanged. A new Host harness is required.

---

## 12. Differential Requirements

### 12.1 Differential kinds

| ID | Kind | Baseline | Candidate | Equality oracle | Status |
|---|---|---|---|---|---|
| D1 | Before/after optimization IR semantics | T09 IR | T10 rewritten IR | VF12 modeled values on the covered subset; `Unsupported` never equal | Blocked: no T10 pipeline at `/39`. |
| D2 | Cross-design quota-1 vs single-active | Single-active reference projection | Quota-1 pipeline projection | Semantic projection equality; scheduler snapshot separate | M1 G9/G15; reference trace not implemented. |
| D3 | Reference ABI / behavior | Reference GCC object/run | Candidate object/run | Two-way mixed link + exit/output/assertion; reference only oracle, never candidate codegen | Part B; probe/toolchain not provisioned. |
| D4 | Evidence differential | Frozen expected instance outcome | VF14 classification | Per-instance outcome; missing/undecodable/fail stay in denominator | Depends on H04/H05/H07/H09. |
| D5 | Negative differential | Intentionally drifted projection | Original projection | Must diverge; PASS on drift is a verifier defect | Required for VF13/VF14 tests. |

### 12.2 Rules

1. A differential result is `Pass` only when the declared equivalence holds on the covered domain and no `Unsupported` was encountered.
2. A divergence is `Fail` with `first_divergence`; it is never averaged away or dropped.
3. A reference compiler may generate the reference counter-half for ABI mixed-link, but must not compile the candidate program or supply candidate output (`T00`, `AGENTS.md` §6).
4. `Unsupported` is never equal, even if both sides are unsupported.
5. Differential provenance includes both artifacts/commands/toolchains and the target/probe hash for ABI claims.
6. No differential result may be used as a T00 pass-rate substitution; it is evidence for a specific rule.
7. D1 is blocked until T10 freezes IR versions, analyses, and rewrite legality. D3 is blocked until the Linux probe reports `verified=true`. D4 is blocked until the Host evidence path exists.

---

## 13. Conflicts with `/39` and Existing VF Slices

Severity: **B** = blocks freeze; **H** = hardening/coverage gap; **I** = integration detail.

| ID | Severity | Conflict / gap | Evidence | Required resolution |
|---|---|---|---|---|
| X-01 | B | No `Verification` store/family/ID/`RecordRef` tag exists; adding one changes `StoreId::ALL`/`COUNT`, `RECORD_KINDS`, `RecordFamily::ALL`, snapshot encoding, and the `/39` hash. | `compiler/src/task.rs:2081-2105`, `:2185-2214`; `compiler/src/ids.rs:119-178`, `:343-400`; `compiler/src/snapshot.rs:528-863` | One integrator-owned schema freeze; `/39` preserved as history; all freeze pins updated. |
| X-02 | B | VF14 has no `HostTestEvidence` family, no frozen instance schema, and no boolean carrier; it accepts any committed record as the instance and maps `Record`/`Records` to `Undecodable`. | `compiler/src/chips/verify/vf_evidence.rs:26-35`, `:263-270`, `:288-319`; `VF_EVIDENCE_SLICE.md` §7 | Add `Verification::Evidence`; add a new Host-evidence classifier kind; keep legacy `/38` semantics for old payloads. |
| X-03 | B | VF14's six-ref payload and a new `[Evidence]` payload cannot share one frozen kind under the kind-shape strict rule. | `compiler/src/chips/verify/vf_evidence.rs:345-351`; `vf_evidence_classify` local 20 frozen | New kind `verification.evidence_classify_host`; do not mutate the frozen kind shape. |
| X-04 | B | VF06 `/14` checks only `{IntLiteral, BinaryAdd, Return}`; `SemRecord` has no conversions field. T13's M1 checklist requires `FunctionDefinition` signature, `Return` operand type, required-conversion completeness, and unchecked-node rejection. | `compiler/src/chips/verify/vf_invariant.rs:5-11`, `:180-234`; `compiler/src/bus.rs:496-513`; `T13` VF06 checklist items 2–5 | T07/T01 must freeze a conversion fact (new field or new record) and extend VF06 in a separate behavior-change freeze. Do not claim the T13 VF06 row is complete today. |
| X-05 | H | VF01 `/19` defers tombstone liveness, record-tag validation, cross-TU IDs, and HostRequest refs. T13's VF01 row says "record tags and lifetime". | `compiler/src/chips/verify/vf_store.rs:9-14`, `:332-410`; `VF01_STORE_SLICE.md` §4 | Keep the frozen scope explicit; add lifetime/tag rules in a later VF01 extension with new hash rules. |
| X-06 | H | VF05 `/18` is closed over the 8-kind M1 tree; wider syntax needs a new slice and new hash rules. | `compiler/src/chips/verify/vf_syntax.rs:11-12`, `:224-233`; `VF05_SYNTAX_SLICE.md` §4 | New syntax kinds require a new VF05 slice; the closed match is the tripwire. |
| X-07 | B | VF12 `/17` returns `Record(Const)` (a modeled value), not a verdict. A generic "Record = PASS" rule would misclassify it. | `compiler/src/chips/verify/vf_interpret.rs:321-324`; `T13` VF12 qualifier | Verdict mapping must be task-kind-aware; `Record` is PASS only when the kind says so; VF14 evidence slots treat `Record` as `Undecodable`. |
| X-08 | B | VF13 draft assumes scheduler fields (`dispatch_cursor`, `stage_queues`, `stage_assignment_version`, `PipelineMetrics`) that do not exist. | `docs/tasks/VF13_FREEZE.md:25-56`; `compiler/src/bus.rs:826-865`; `compiler/src/limits.rs:21` | Quota-1 VF13 can compare current control/tasks/report/trace; full scheduler snapshot waits for T02/H6. |
| X-09 | B | VF13 draft's "tick-for-tick" is not implemented by the existing final-snapshot replay tests. | `compiler/tests/c17_vf12.rs:654`; `compiler/tests/c19_vf01.rs:626`; `VF13_FREEZE.md:58-79` | Add Host per-tick digest capture; do not claim tick-for-tick from final snapshots. |
| X-10 | B | VF02/VF03 H6 latch invariants are transient: `in_flight` is cleared at latch, `wires` reset at tick start, and no pre-recovery snapshot exists. A later chip cannot observe them. | `compiler/src/bus.rs:832-843`; `compiler/src/limits.rs:30-101`; `T13` H6-M01..M16 | Split steady-state chip facts from Host latch evidence; or freeze a new latch snapshot carrier (rejected for now as too large). |
| X-11 | B | VF04 batch write-conflict audit is proposed/residual, not defined; read-set monitoring has a known limitation; there is no `ChipTrace` record family. | `T13` VF04 row; `T02_CONTROL_CHIPS.md:19,37`; `compiler/src/manifest.rs:1365-1435` | Freeze write-only manifest/patch audit first; batch conflict remains residual and must not be claimed. |
| X-12 | B | `StoreOwnerViolation` wording says one owner per field, but `STORE_OWNER_ALLOWLIST` already grants multiple same-group chips one append field. | `COMPILER_DEVELOPMENT_GUARDRAILS.md` §2.7; `T02_CONTROL_CHIPS.md:19`; `compiler/src/manifest.rs:784-978` | Clarify owner-group vs chip-level ownership before freezing `Verification/records`; recommended owner-group interpretation. |
| X-13 | B | VF07–VF11 are out of M1, and no CFG/SSA/machine/ABI record bodies exist (`layouts`/`inits`/`vregs` are reserved; `Machine`/`Ext` stores have no typed records). | `T13` M1-scope qualifier; `M1_VERTICAL_SLICE_ACCEPTANCE.md:470-477`; `compiler/src/bus.rs`; `compiler/src/task.rs:2081-2105` | Keep non-M1; freeze verifiers only after T09/T10/T11 producer schemas. |
| X-14 | B | Target ABI/machine values are unverified; VF11/VF10 and any ABI/machine PASS are blocked. | `compiler/contracts/CONTRACT_VERSION` (`target_verification=unverified`); `T11` probe-gated qualifier | No VF10/VF11 PASS before `verified=true` and probe report hash. |
| X-15 | I | `DiagGroup` has no verification group; current VF failures use `Task, 4`, conflating protocol faults with rule failures. | `compiler/src/diagnostic.rs:37-71`; `compiler/src/chips/verify/vf_store.rs:253` | Append `DiagGroup::Verification` (recommended) or freeze a `Task` code subrange; update snapshot names/hash. |
| X-16 | I | `Snapshot::capture` is whole-state; there is no semantic projection or scheduler snapshot function. `Trace` is Host-side, not a bus record. | `compiler/src/snapshot.rs:522-879`, `:952-1064` | Add projection/snapshot functions in `snapshot.rs` (integrator-owned); store per-tick digests as artifacts or result fields. |
| X-17 | I | `ArtifactKind::Snapshot`/`Trace` can carry replay inputs, but no Host path currently exports two runs' per-tick artifacts. | `compiler/src/bus.rs:79-105`; `compiler/src/bin/candidate.rs` | Host work order under H04/H05; no chip I/O. |
| X-18 | I | New VF kinds need slice predicates in `check_store_owner_allowlist`; otherwise the allowlist check is skipped for unclaimed kinds. | `compiler/src/manifest.rs:1371-1435` | Add `is_vf_<name>_slice_kind` predicates and rows with each freeze. |
| X-19 | I | Stage 6 is the only implemented verification stage; VF05 stage 2 and VF06 stage 4 share producer stages. There is no separate "verification stage" beyond ordinal 6. | `compiler/src/manifest.rs:1444-1514` | Keep M1 verifiers at their frozen or proposed 6; assign non-M1 stages with producer owners. |
| X-20 | I | `ResultValue` is explicitly frozen unchanged; a `Verification` result variant would contradict that note. | `compiler/src/task.rs:1722-1739` | Do not add a variant; use `Ack`/`Fail` + audit record. |

---

## 14. Ordered Freeze Work Orders

Every order is **proposal-only** until the T01 integrator accepts it. Do not edit shared files from a chip worktree.

| Order | Target | Scope | Exact paths (primary) | Exit gate |
|---|---|---|---|---|
| FW-0 | none | Accept/decline this requirements package; deduplicate with A15 (IR), A16 (control), A18 (diagnostics). | `docs/reviews/dispatch/A17_VERIFICATION_REQUIREMENTS.md`; `docs/tasks/DISPATCH_LEDGER.md` (integrator update only) | Integrator review note; no freeze. |
| FW-1 | next `/40+` | Shared verification schema: `Verification` family/store/ID/tag; `VerificationRecord`/`Fact`/`Result`/`Evidence`; outcome lattice; rule table; `DiagGroup::Verification`; `ResultValue` unchanged; no VF chip yet. | `compiler/src/ids.rs`, `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/diagnostic.rs`, `compiler/src/snapshot.rs`, `compiler/src/commit.rs`, `compiler/src/contract.rs`, `compiler/contracts/COMPILER_SFL_MANIFEST.md`, `compiler/contracts/CONTRACT_VERSION`, `compiler/tests/freeze.rs`, `docs/tasks/T01_COMPILER_CONTRACT.md` §7.1 | Schema round-trip, snapshot hash/replay, freeze pins, no `ResultValue` change. |
| FW-2 | next+1 | VF13 quota-1 replay: kind/chip/stage, artifact inputs, `ComparisonSummary`, semantic/scheduler projection views, H6-M12 still pending. | `compiler/src/chips/verify/vf_replay.rs`, `compiler/src/chips/verify/mod.rs`, `compiler/src/chips/mod.rs`, `compiler/src/snapshot.rs`, `compiler/src/manifest.rs`, `compiler/src/task.rs`, `compiler/src/contract.rs`, `compiler/tests/c40_vf13.rs` (rename to actual ordinal), `compiler/tests/freeze.rs`, `docs/tasks/T13_VERIFICATION_CHIPS.md`, `docs/tasks/VF13_FREEZE.md` (do not edit in place without owner sign-off; promote to an accepted freeze doc) | Positive equal, semantic drift, scheduler-only drift, malformed/missing, replay determinism, H6-M12 explicitly pending. |
| FW-3 | next+2 | VF14 Host-evidence extension: `Verification::Evidence`, `EvidenceGate`/`EvidenceSlot`, new classifier kind; legacy `/38` preserved. | `compiler/src/chips/verify/vf_evidence.rs` (new sibling path, not the frozen kind), `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/tests/cXX_vf14_host.rs`, `docs/tasks/VF_EVIDENCE_SLICE.md` (new revision), `docs/tasks/T13_VERIFICATION_CHIPS.md` | Evidence positive, compile-ok/run-fail, missing, undecodable, invalid gate, precedence, provenance required. |
| FW-4 | next+3 | VF02/VF03 steady-state + H6 latch fixtures: task graph, wire lifetime; split transient Host evidence from chips. | `compiler/src/chips/verify/vf_task.rs`, `compiler/src/chips/verify/vf_wire.rs`, `compiler/src/manifest.rs`, `compiler/src/task.rs`, `compiler/tests/cXX_vf02.rs`, `compiler/tests/cXX_vf03.rs`, `docs/tasks/T02_CONTROL_CHIPS.md` (co-freeze), `docs/tasks/T13_VERIFICATION_CHIPS.md` | H6-M01..M16 covered as chip steady-state and Host latch evidence; no claim of transient chip observation. |
| FW-5 | next+4 | VF04 access contract: write-manifest/patch-log audit; batch conflict residual. | `compiler/src/chips/verify/vf_access.rs`, `compiler/src/manifest.rs`, `compiler/src/commit.rs`, `compiler/tests/cXX_vf04.rs`, `docs/tasks/T02_CONTROL_CHIPS.md` | Unauthorized write/wrong owner/wrong phase rejected; read-set limitation documented. |
| FW-6 | next+5 | VF07/VF08 CFG/IR (non-M1). | `compiler/src/chips/verify/vf_cfg.rs`, `compiler/src/chips/verify/vf_ir.rs`, `compiler/src/bus.rs` (only if T09 freezes new IR), `compiler/tests/cXX_vf07.rs`, `compiler/tests/cXX_vf08.rs`, `docs/tasks/T09_IR_LOWER_CHIPS.md` | IR/CFG producer freeze first; then positive/dangling/terminator/type/effect cases. |
| FW-7 | next+6 | VF09 SSA (non-M1). | `compiler/src/chips/verify/vf_ssa.rs`, `compiler/tests/cXX_vf09.rs`, `docs/tasks/T10_OPTIMIZE_CHIPS.md` | T10 SSA/dominance freeze; non-SSA not falsely flagged. |
| FW-8 | next+7 | VF10/VF11 machine/ABI (non-M1, probe-gated). | `compiler/src/chips/verify/vf_machine.rs`, `compiler/src/chips/verify/vf_abi.rs`, `compiler/tests/cXX_vf10.rs`, `compiler/tests/cXX_vf11.rs`, `docs/tasks/T11_TARGET_CHIPS.md`, `compiler/contracts/target/aarch64-linux-probe.txt` | Probe `verified=true`, ABI report hash, machine/ABI producer records frozen. |
| FW-9 | later | Optional hardening: VF01 lifetime/tags, VF05 wider syntax, VF06 conversion completeness, VF12 target widths. | `docs/tasks/VF01_STORE_SLICE.md`, `VF05_SYNTAX_SLICE.md`, `VF06` checklist in `T13`, `VF12_INTERPRET_SLICE.md`, corresponding `compiler/src/chips/verify/vf_*.rs` + tests | Separate behavior-change freezes; do not bundle with FW-1. |

**Order rationale:** FW-1 establishes the shared schema; FW-2 and FW-3 consume it for the two M1 gate-facing verifiers; FW-4/FW-5 close control invariants; FW-6–FW-8 wait for producer schemas; FW-9 is optional hardening. FW-3 may run in parallel with FW-2 once FW-1 is frozen, but the integrator should serialize the first schema bump.

---

## 15. Ordered Implementation Work Orders

These are **post-freeze** work orders. A worker may not start before the named freeze is accepted. Paths are exact where the repository already defines the convention; `cXX_*` names are assigned by the integrator at the freeze ordinal.

| Order | Package | Owned implementation paths | Owned test paths | Reads/writes | Acceptance | Blockers |
|---|---|---|---|---|---|---|
| IW-1 | Shared verification schema (integrator only) | `compiler/src/ids.rs`, `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/diagnostic.rs`, `compiler/src/snapshot.rs`, `compiler/src/commit.rs`, `compiler/src/contract.rs` | `compiler/tests/freeze.rs`, `compiler/tests/c04_manifest.rs` (if store schema), `compiler/tests/c05_codec.rs` (if codec) | Schema only | Round-trip, hash/replay, freeze pins, no undeclared store field | FW-1 |
| IW-2 | VF13 replay chip | `compiler/src/chips/verify/vf_replay.rs` | `compiler/tests/cXX_vf13.rs` | Reads `Artifact`/`Snapshot`/`Trace` artifacts + config; writes `Verification/records` | Equal, semantic drift, scheduler-only drift, first divergence, malformed/missing, determinism | FW-2; Host per-tick artifact path (H04/H05) |
| IW-3 | Host replay harness | `compiler/src/bin/candidate.rs` or a new Host test helper under `compiler/tests/` (integrator chooses), `compiler/tests/cXX_vf13.rs` | same | Host capture of two runs' per-tick projections; no chip I/O | Two identical runs equal; forced drift diverges; artifacts/hashes retained | H04/H05; FW-2 |
| IW-4 | VF14 Host evidence classifier | `compiler/src/chips/verify/vf_evidence_host.rs` (new file; do not edit frozen `vf_evidence.rs` semantics) | `compiler/tests/cXX_vf14_host.rs` | Reads `Verification::Evidence`; writes `Verification/records` | Positive, run-fail, missing, undecodable, invalid gate, precedence, provenance | FW-3; H04/H05 evidence schema |
| IW-5 | VF02 task invariant | `compiler/src/chips/verify/vf_task.rs` | `compiler/tests/cXX_vf02.rs` | Reads tasks/results/diagnostics/in_flight/report; writes `Verification/records` | Mutual exclusivity, acyclic parent/child, exactly-once, dead-wait, duplicate complete | FW-4; H6 steady-state scope |
| IW-6 | VF03 wire lifetime | `compiler/src/chips/verify/vf_wire.rs` | `compiler/tests/cXX_vf03.rs` | Reads pre/post artifacts, wires, tasks; writes `Verification/records` | Reset, proposal lifetime, persistent continuation not on wire, idle after warmup | FW-4; pre/post capture decision |
| IW-7 | VF04 access contract | `compiler/src/chips/verify/vf_access.rs` | `compiler/tests/cXX_vf04.rs` | Reads `patch_log`, manifests, routing, task owners; writes `Verification/records` | Unauthorized write, wrong owner/phase/task; read-set limitation | FW-5; T02 patch log stable |
| IW-8 | VF07/VF08 CFG/IR | `compiler/src/chips/verify/vf_cfg.rs`, `compiler/src/chips/verify/vf_ir.rs` | `compiler/tests/cXX_vf07.rs`, `compiler/tests/cXX_vf08.rs` | Reads IR/types/consts; writes `Verification/records` | Terminator/edge/target, operand/result type, memory width/effects, legal op domain | FW-6; T09 wider IR |
| IW-9 | VF09 SSA | `compiler/src/chips/verify/vf_ssa.rs` | `compiler/tests/cXX_vf09.rs` | Reads SSA/dominance; writes `Verification/records` | Dominance, phi arity/type, single def, non-SSA not falsely flagged | FW-7; T10 SSA |
| IW-10 | VF10/VF11 machine/ABI | `compiler/src/chips/verify/vf_machine.rs`, `compiler/src/chips/verify/vf_abi.rs` | `compiler/tests/cXX_vf10.rs`, `compiler/tests/cXX_vf11.rs` | Reads machine/ABI/frame records; writes `Verification/records` | Use/def/classes/ties/spills/frame; args/returns/SP/saves/target identity; probe hash | FW-8; T11 + probe |
| IW-11 | Host evidence pipeline | Host-owned H04/H05/H07/H09 paths (assigned by A-HST-01/A-HST-02; not invented here) | H04/H05 acceptance suites | Host I/O, isolation, limits, evidence import | EvidenceRecord provenance complete; no missing denominator | H04/H05/H07/H09 |

**Ownership rule:** `compiler/src/chips/verify/mod.rs`, `compiler/src/chips/mod.rs`, `compiler/src/ids.rs`, `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/snapshot.rs`, `compiler/src/commit.rs`, `compiler/src/contract.rs`, `compiler/contracts/*`, and `compiler/tests/freeze.rs` are integrator-owned. A chip owner edits only its `compiler/src/chips/verify/vf_*.rs` and its assigned test file.

---

## 16. Open Questions

| ID | Question | Owner | Recommended default |
|---|---|---|---|
| OQ-1 | One `VerificationRecord` union vs three families? | T01/T13 | One union family; smallest schema. |
| OQ-2 | New `DiagGroup::Verification` vs a `Task` code subrange? | T01/T13 | New group at the end; clearer separation. |
| OQ-3 | Numeric `VerificationRuleId` inventory vs interned rule names? | T01/T13 | Numeric `(class << 8) | local` with a frozen name table; stable and compact. |
| OQ-4 | Should frozen VF01/05/06/12/14 emit `Result` audit records? | T13/T01 | No in `/40`; retrofit only as a separate behavior-change freeze. |
| OQ-5 | Owner-group vs chip-level ownership of `Verification/records`? | T01/T02 | Owner-group, matching `Sem/records` practice; per-chip allowlist rows. |
| OQ-6 | Host evidence written via trusted boundary vs a Host-group chip? | T01/H04 | Trusted Host boundary now; a chip only if H04 requires it. |
| OQ-7 | VF03 pre/post capture: Host artifact vs new `TickSnapshotRecord`? | T13/T02 | Host artifact (`ArtifactKind::Snapshot`/`Trace`); avoid a large new record. |
| OQ-8 | Exact semantic projection inclusion/exclusion list (task states, patch log, store versions, artifacts, wires)? | T13/T02 | Start from §11.2; freeze the list; compare both tracks separately. |
| OQ-9 | Scheduler snapshot content when stage queues/dispatch cursor/PipelineMetrics do not exist? | T02/T01 | Compare current control/tasks/report/trace at quota 1; extend after T02/H6. |
| OQ-10 | Per-tick replay localization: rolling chain vs per-tick digest list vs raw artifacts? | T13/H04 | Rolling chain plus Host-retained first-divergent artifact; no `max_ticks` snapshots in the bus. |
| OQ-11 | VF02/VF03 transient H6 checks: chip vs Host latch harness? | T02/T13 | Host latch harness for transient; chip for steady-state. |
| OQ-12 | VF06 conversion completeness carrier: extend `SemRecord` vs new conversion record? | T07/T01 | New typed conversion fact; do not silently widen `SemRecord` without a freeze. |
| OQ-13 | VF12 target-width interpretation: stay symbolic vs Part B extension? | T09/T13 | Stay symbolic; target execution is T11/Part B, not VF12. |
| OQ-14 | New VF stage ordinals (6 vs 7/8/9)? | T01/T09/T10/T11 | 6 for M1-adjacent; assign non-M1 with producer stages. |
| OQ-15 | New chip IDs 58+ / local codes 21+? | T01 | Candidate table §6.2; integrator assigns at freeze. |
| OQ-16 | `Unsupported` mapping: Fail subtype vs separate label? | T13/T01 | Fail subtype with an unsupported code; never PASS. |
| OQ-17 | `Result.outcome` audit vs task-state verdict mismatch handling? | T13/T01 | `Invalid` verifier defect; never repair by rewriting the result. |
| OQ-18 | Evidence `case` as `NameId` vs a new `TestInstanceId`? | T01/H04 | `NameId` + `source_hash` + `options_hash`; no new ID family. |
| OQ-19 | Does a global verifier pass vacuously? | T13 | Only when the rule explicitly says global/vacuous; otherwise `Missing`. |
| OQ-20 | Should verification tasks run every tick or at checkpoints? | T02/T13 | Checkpoints/Host request; no per-tick whole-store scan; counts against `max_inflight_per_tick`. |
| OQ-21 | Cross-TU/multi-file evidence: one record per instance/config or per TU? | H04/T13 | One record per `(case, target, dialect, options)` instance; source hash covers the TU set. |
| OQ-22 | Raw log retention and secret redaction in evidence provenance? | A24/H05/T13 | Hash in bus; raw logs Host-side with access control; no secrets/paths in records. |

---

## 17. Rejected Alternatives

| ID | Alternative | Why rejected |
|---|---|---|
| RA-1 | No new records; diagnostics only. | Cannot represent evidence provenance, replay chains, or queryable PASS audit; diagnostics exist only on failure. |
| RA-2 | Three separate families (`Fact`, `Result`, `Evidence`). | Triples ID/arena/encoder/hash surface for one domain; one union family is smaller and still typed. |
| RA-3 | Add `ResultValue::Verification` variant. | `ResultValue` is frozen unchanged (`compiler/src/task.rs:1722-1739`); a variant changes snapshot tags and every consumer. `Ack`/`Fail` + audit record is sufficient. |
| RA-4 | Retrofit frozen VF01/05/06/12/14 in the first schema freeze. | Causes behavior/hash churn and breaks `/39` replay; a separate behavior-change freeze is safer. |
| RA-5 | Make VF02/VF03/VF04/VF13 transient latch checks ordinary post-latch chips. | The latch state is cleared/reset and not observable after the tick; a later chip would fabricate a pass. Use Host latch evidence or a new carrier. |
| RA-6 | Byte-identical whole-state snapshot comparison for replay. | M1 G9/G15 explicitly require the semantic projection with the scheduler snapshot separate, not byte-identical whole state. |
| RA-7 | Wall-clock, timestamps, PIDs, or absolute paths in semantic provenance. | Violates determinism and leaks host state; use bus ticks and hashes. |
| RA-8 | Count VF PASS as T00 PASS. | VF14 classifies one instance; T00/H09 owns the suite rate; a verifier pass is not a torture rate. |
| RA-9 | Accept producer `Ack`/self-tests as verification facts. | Defeats independent verification; the verifier must recompute from committed records. |
| RA-10 | Treat `Unsupported` as equal/PASS. | T13 and `AGENTS.md` §4.2/§6 prohibit fabricated success; unsupported stays explicit. |
| RA-11 | Use `Record`/`Records` as boolean evidence. | VF14 `/38` freezes them as `Undecodable`; a boolean carrier must be typed (`EvidenceSlot`). |
| RA-12 | Add a new `max_verification_facts` limit. | Duplicates `max_records_total`/`max_diagnostics`; no evidence a separate bound is needed. |
| RA-13 | Freeze VF07–VF11 before T09/T10/T11. | Would invent CFG/SSA/machine/ABI schemas; violates "freeze shared producers first". |
| RA-14 | Run VF10/VF11 before the Linux probe. | ABI/machine PASS would rely on unverified values or macOS arm64; prohibited. |
| RA-15 | One global store owned by all groups. | Violates owner-group discipline; `Verification/records` is owned by VERIFICATION and `Verification/evidence` by Host. |
| RA-16 | Per-tick full snapshot array in the bus. | Unbounded record growth; rolling chains plus Host-retained artifacts are smaller. |
| RA-17 | Let VF13 run the pipeline twice inside one chip. | Chips do not execute other tasks or I/O; replay is a Host harness plus a pure comparison chip. |

---

## 18. Targeted Checks for This Report

This task was read-only. The following checks were actually performed; no compiler test, probe, corpus run, or pass-rate measurement was performed.

| Check | Result |
|---|---|
| Baseline version/hash read from `compiler/contracts/CONTRACT_VERSION` and `compiler/src/contract.rs` | `/39`, `07f4eade…f2fa24` |
| Frozen VF kinds/stages/chip IDs read from `compiler/src/task.rs` and `compiler/src/manifest.rs` | VF01 19/22/6, VF05 18/21/2, VF06 16/15/4, VF12 17/20/6, VF14 20/56/6 |
| `ResultValue`, `RecordRef`, `RECORD_KINDS`, `RecordFamily`, `StoreId` inventories read | 5 / 28 / 28 / 28 / 21; no `Verification` entry |
| Snapshot/Trace structure read | Whole-state `Snapshot`; per-tick `Trace`; no semantic projection/scheduler snapshot |
| TaskStore/TickMetrics read | `ready`, `active`, `in_flight`; `dispatched` count only; no stage queues/dispatch cursor/PipelineMetrics |
| VF13 draft status read | `DRAFT ONLY`; H6-M12 required/pending; final-snapshot replay harness, not tick-for-tick |
| H6/H9 matrix status read | H6-M01..M16 required/pending; no implementation claim |
| Report path written | `docs/reviews/dispatch/A17_VERIFICATION_REQUIREMENTS.md` only |
| Source/tests/contracts/Cargo/mod.rs modified | none |
| Commit/push/agents started | none |

No link validator or contract validator exists for this new report path; the document uses only existing repository paths or explicitly marked proposed paths. Any future freeze must run the applicable `compiler/` checks named in `AGENTS.md` §6 and `MASS_PARALLEL_DISPATCH.md` §11 before claiming a result.

---

## 19. Bottom Line

The smallest defensible T13 verification contract is:

- **one** `Verification` record family with `Fact`, `Result`, and `Evidence`;
- **one** total precedence lattice: `Invalid > Missing > Undecodable > Fail > Pass`;
- **one** verdict carrier pair: `Ack` for PASS, `Fail` + one diagnostic for non-PASS;
- **one** evidence record with typed gate/slots/provenance;
- **one** replay comparison summary with separate semantic and scheduler chains;
- **zero** changes to the frozen `ResultValue` set or the five frozen VF slices;
- **zero** claims that any of this is frozen, implemented, tested, or a T00 pass rate.

The integrator should freeze FW-1 (shared schema) and FW-2 (VF13 quota-1) first, then FW-3 (Host evidence), then the control and non-M1 verifiers in producer order. VF01/VF05/VF06/VF12 hardening should not be bundled with the new schema.
