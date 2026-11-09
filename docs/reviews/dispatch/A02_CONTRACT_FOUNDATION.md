# A02 — Compiler Contract Foundation Audit

**Status:** Complete — read-only audit; no source, test, contract, Cargo, or `mod.rs` file was changed.
**Date:** 2026-11-07
**Audited revision:** `430ae29162efef90f86667f384097964d611604f` (`wave4-development`, 2026-11-07)
**Contract baseline:** `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` (`compiler/contracts/CONTRACT_VERSION`)
**Target state:** identity `aarch64-unknown-linux-gnu` / ELF / LP64 / little-endian / AAPCS64; all concrete values still `target_verification=unverified`; no probe has run.

**Workspace state at audit time:** dirty, preserving user work:

- modified: `compiler/src/chips/parse/mod.rs` (adds `pa_cast`, `pa_decl_spec`, `pa_ext_dispatch`, `pa_init`, `pa_stmt`);
- untracked chip candidates: `compiler/src/chips/lex/{lx_error,lx_extension}.rs`, `compiler/src/chips/parse/{pa_cast,pa_decl_spec,pa_expr_ext,pa_ext_dispatch,pa_init,pa_stmt,pa_typedef_query}.rs`;
- untracked planning/freeze drafts: `docs/tasks/{CT_FREEZE_01,CT_FREEZE_02,CT_FREEZE_03,H04_I_FREEZE,OP_FREEZE_01,OP_FREEZE_02,OP_FREEZE_03,VF13_FREEZE}.md` and `docs/tasks/DISPATCH_LEDGER.md`.

No foundation file (`bus.rs`, `commit.rs`, `task.rs`, `records.rs`, `ids.rs`, `snapshot.rs`, `contract.rs`, `manifest.rs`, `target.rs`, routing/limits/diagnostic/intern/codec) was modified by this audit.

---

## 1. Executive assessment

`/39` is a real, tested foundation **envelope**, not just a document: arenas and IDs are checked, task/proposal envelopes and an atomic commit preflight exist, the routing shell is deterministic at quota 1, snapshots/hashes are deterministic for the encoded state, manifests are validated, and the target codegen gate fails closed. The frozen version/hash is internally consistent: `freeze.rs` recomputes the hash and checks the version file.

It is **not yet a clean Stage-B chip foundation**. The largest gaps are:

1. **Chip boundary:** 65 `impl Worker` sites exist in the current workspace (including uncommitted candidates), and the trait's `handle(&CompilerBus)` owns projection; **no compiler chip implements `RestrictedChip`**, and no `ChipAdapter`/`Motherboard::install_projected` path is used. T01 §4.1/AGENTS §3.3 require `RestrictedChip`; the contract grading table permits an adapter trust boundary, so this is an unresolved contract choice rather than an obvious code bug. Stage B must not copy two competing patterns.
2. **Reference/continuation materialization:** `AppendRecords` has no `Source`, `Expansion`, or `Continuation` body; `StorePatch` is only logged, never materialized; payload/patch/continuation references are not existence-checked; `resolve_continuation_ref` is defined but never called. T01 §4's reserved-ID/local-reference protocol is absent.
3. **Bus profile assembly:** `CompilerBus::new` installs only the foundation registry/schema. Gate 1 and every later slice kind/schema must be installed manually through public fields in tests. There is no frozen `CompilerProfile`/registration generator, so a Stage B work order can run against a bus that does not accept its kinds.
4. **Limits are not all enforced:** `stage_queue_bound` is validated/hashed/snapshotted but never applied; `Arenas::allocated_total()` omits `macros`, so `max_records_total` undercounts existing macro records.
5. **Snapshot/replay completeness:** pre-commit `AppendRecords` wires encode only the draft count, not bodies/families; two different append batches can produce identical snapshots. A non-commit error path skips the per-tick record while still latching/advancing.
6. **Store-owner/single-writer enforcement:** the allowlist and the “zero writers to `tasks.ready`” invariant are wave-gated and not enforced for foundation/control manifests or at commit.
7. **Document/version drift:** `CONTRACT_VERSION` and `contract.rs` use `post-seed-declarations`, while `compiler/README.md` still carries `group-declared-store-fields`; `PARALLEL_EXECUTION.md` says `/9` is current; guardrails say `/5` is current; T01 C04/C05 status rows still call the hash-scope reconciliation open.

The targeted foundation suites pass; those successes must not be read as proof of the missing guarantees above.

---

## 2. Capability table — what `/39` actually provides

| Capability | Implemented evidence | Tests / evidence run or referenced | Actual guarantee / limit |
|---|---|---|---|
| Append-only arenas, stable IDs, checked access | `compiler/src/arena.rs:245-330` (`TypedArena` alloc/get/remove/push), `:347-409` (`ReservedArena`), `compiler/src/ids.rs:57-95` (newtype IDs), `:119-178` (`RecordRef`), `:226-257` (wire tags), `:343-400` (`RecordFamily`) | `c01_arena` 7 tests (ran); `c03_task` tag/family tests; `freeze.rs` mappings (ran) | Dense IDs, tombstones, no reuse, sentinel `u32::MAX`, per-arena capacity. Direct public `bus.arenas` mutation bypasses global budgets by documented design. |
| Task / result / proposal envelope | `compiler/src/task.rs:1676-1781` (`TaskState`/`Task`), `:1727-1754` (`ResultValue`/`ResultRecord`), `:1893-1990` (`Proposal` + tags + transition predicate), `:1861-1867` (`AppendBatch`), `:1556-1674` (continuation/child refs) | `c03_task` 46 tests (ran); `c08`; `c09` | Envelope exists; concrete per-group payload enums are **not** frozen. `ContinuationRef` exists as a type but is not accepted by `TaskDraft.continuation`. |
| Atomic commit validation and apply | `compiler/src/commit.rs:530-560` (entry + deterministic order), `:553-800` (validation), `:800-1388` (per-proposal validation/join simulation), `:1390-1438` (capacity preflight), `:1440-1761` (infallible apply), `:1848-1923` (patch validation) | `c03_task` 46, `c08_gate1` 21, `c09_readiness` 14 (ran) | Validation failure commits no semantic state; apply uses only infallible pushes after preflight. References inside patches/payloads are generally **not** existence-checked. `StorePatch` is logged, not applied to a store. |
| Routing / quota-1 shell | `compiler/src/routing.rs:212-253` (selection + quota), `:276-540` (propagate, recovery), `:602-695` (clock/reset/report), `:95-98` (foundation routes rejected), `:256-268` (resolution) | `c06_routing` 15 tests (ran); `c09` pcr09/pcr13 | Deterministic `(phase priority, enqueue ordinal, TaskId)` selection; quota 1 baseline; no-op/unsupported/unregistered fail explicitly; commit failure is caught and the task is failed (no stranded `Running`). `CONTROL_START_JOB`/`CONTROL_IMPORT_SOURCE` are unroutable by this shell. |
| Snapshot / trace / config hash | `compiler/src/snapshot.rs:528-863` (`Snapshot::capture`), `:871-878` (hash), `:986-1064` (`Trace`), `:393-487` (wires), `compiler/src/codec.rs:11-187` (writer/SHA-256) | `c05_codec` 21 tests (ran); `c09` pcr01 | Deterministic for every encoded field; full foundation record bodies where typed; reserved-store bodies remain IDs-only. `AppendRecords` draft bodies are not encoded. |
| Limits / budgets | `compiler/src/limits.rs:30-80` (fields), `:154-174` (validation), `compiler/src/bus.rs:1021-1094` (total/source/diagnostic checks), `compiler/src/commit.rs:1974-2088` (per-arena/total preflight) | `c07_limits` 18 tests (ran); `c08` total-budget test | Most checked entry points enforce their bounds before mutation. `stage_queue_bound` is unused; `max_records_total` omits existing `macros`; `max_const_bits` is chip-enforced only. Direct public-store mutation bypasses global checks. |
| Manifests / registration | `compiler/src/manifest.rs:383-430` (manifest), `:671-751` (structural validation), `:1371-1435` (allowlist), `:1444-1514` (stage assignment), `:1583-1632` (registry); bus stores `kinds`/`schema`/`registrations` | `c04_manifest` 15 tests (ran); `c08`/`c09` registration/route tests | Structural lint works; exact write sets are enforced at commit for `StorePatch` and `AppendRecords`. Read sets and store-owner allowlist are only partially enforced (wave-gated). |
| Canonical codec / diagnostics | `compiler/src/codec.rs:11-187` (canonical writer + SHA-256), `:240-344` (checked reader); `compiler/src/diagnostic.rs:37-176` (groups/drafts/records) | `c05_codec` 21 (ran); `c03_task` error-code tests | No panic-control-flow decoders; structured errors. Diagnostic numeric codes are frozen by code/comments and partially by tests. |
| Target model / verification state | `compiler/src/target.rs:381-423` (private `VerificationState`), `:583-716` (`TargetSpec`/`attest`), `:1086-1347` (`CompilerConfig`), `compiler/contracts/target/aarch64-linux-probe.txt` | `c02_target` 11 tests (ran) | Identity fixed; concrete values unverified; `attest` validates report data/hash/identity, not provenance; `ensure_codegen_ready` fails closed. Only 13 of 21 scalar records are probe-filled. |
| Contract hash / freeze artifact | `compiler/src/contract.rs:38-44` (version/hash), `:51-...` (`NORMATIVE_RULES`), `:515-756` (`FrozenSchema::encode`), `:781-783` (`compute_contract_hash`), `compiler/contracts/CONTRACT_VERSION` | `freeze.rs` 14 tests (ran) | Hash covers declared shapes/rule IDs and the M1 seed schema, not source code, numeric wire tags, or semantic equivalence. Version/hash file is consistent with code. |
| Worker/chip integration | `compiler/src/chips/mod.rs:136-150` (`Worker` trait), `:152-188` (`WorkerRegistry` ZST check), `:309-357` (`drive_task`), `:364-419` (`handler_for`) | `c08` 21, `c09` 14 (ran) | 65 `impl Worker` sites in the workspace (including uncommitted candidates); workers are ZST and pure `compute` methods exist, but the trait gives `handle` a full `&CompilerBus` and no `RestrictedChip`/`ChipAdapter` path exists. |
| Bus profile / registration assembly | `compiler/src/bus.rs:974-990` (`CompilerBus::new` = foundation registry/schema); tests set `bus.kinds`/`bus.schema` manually (`c09_readiness.rs:39-48`) | `c08`/`c09` install Gate 1 manually | No single `/39` profile constructor; later-wave kinds/schemas are opt-in public mutation, not a frozen runtime assembly. |

---

## 3. Evidence classification

### 3.1 Implemented and mechanically enforced

- Append-only arenas with checked access, tombstones, sentinel handling, per-arena capacity (`arena.rs:276-330`, `:347-409`).
- Deterministic commit ordering and whole-batch validation before the apply pass (`commit.rs:541-551`, `:1390-1438`); exactly-one transition per dispatched task (`commit.rs:1260-1284`); inner task binding (`commit.rs:1764-1773`); parent prediction for Enqueue (`commit.rs:823-862`).
- Exactly-once completion and result consumption (`commit.rs:800-815`, `:2091-2102`).
- Config read-only at registration and commit (`manifest.rs:710-721`, `commit.rs:1867-1871`).
- Quota-1 selection and recovery from commit failure without stranded `Running` tasks (`routing.rs:223-243`, `:488-534`, `:577-591`).
- Snapshot/trace/config hash determinism for the encoded fields (`snapshot.rs`, `codec.rs`); contract hash recomputation (`freeze.rs`).
- Private target verification state and fail-closed codegen gate (`target.rs:381-423`, `:648-656`, `:1345-1347`).

### 3.2 Frozen in the contract/hash but not fully enforced by code

- **`RestrictedChip` boundary / no-bus chip logic:** T01 §4.1 (`T01_COMPILER_CONTRACT.md:84-101`) and AGENTS §3.3 require `RestrictedChip`; the implemented `Worker` path is not one. The contract grading table itself admits the adapter read/write set is not provable, but Stage B still needs one frozen pattern.
- **Reserved-ID/local-reference relocation:** T01 §4 (`T01:75-82`) and the C01/C03 status rows (`T01:876-880`) acknowledge it is absent; code confirms it.
- **“Every configured bound enforced”:** `contract.rs:63` and T01 C01 row; `stage_queue_bound` and `max_records_total` macro omission contradict it.
- **“Full wire proposal payloads”:** `T01:880` and rule `snapshot.wire-payloads-encoded` (`contract.rs:125`); `AppendRecords` bodies are absent from `snapshot.rs`.
- **“Zero allowlisted writers to `tasks.ready`”:** `manifest.rs:780-783`; only a table-inspection test (`c04:238-256`), not registration/commit enforcement.
- **“Exactly one TickRecord per tick”:** `routing.rs:596-601`; non-commit error paths skip the record while still latching/advancing.

### 3.3 Proposed / planned, not implemented

- T02 control chips (start job, source response, selection/guard, commit/recovery, phase/cancel) — `T02_CONTROL_CHIPS.md` rows CT01–CT14; the routing shell has no installed handlers and `START_JOB`/`IMPORT_SOURCE` are unroutable.
- Host H04/H05 tooling, source include loading, isolated execution, cancellation/timeouts, corpus/runner, and all torture acceptance.
- T03–T13 breadth: real preprocessing/lexing/parsing/types/semantics/layout/IR/ABI codegen/optimization/GNU builtins/verification beyond the bounded slices.
- Target probe (Part B), attestation handoff (H07/T01), all concrete ABI/scalar values, and any execution claim.
- Numeric hash coverage for wire tags/later-wave kind names; full `RestrictedChip`/adapter migration; stage queues and per-stage bounds; source/expansion/continuation materialization; optional artifact kinds; `StorePatch` materialization.
- Full C compiler and GCC torture pass rate — not claimed anywhere in `/39`.

### 3.4 Unverified or overstated claims

- `compiler/README.md:8` “no language chips” and its `c01..c07 + freeze` layout (`:123`) are stale; 65 `impl Worker` sites (including uncommitted candidates) and `c39` exist.
- `compiler/README.md:40` still lists `hash_excludes=...group-declared-store-fields...`; `CONTRACT_VERSION:11` and `contract.rs:798` use `post-seed-declarations`; `README:236-241` says the reconciliation is open.
- `COMPILER_SFL_MANIFEST.md:84-100` claims the two-tier change was applied atomically to `README.md`; it was not.
- `PARALLEL_EXECUTION.md:7` says `/9` is the current frozen artifact; T01/MASS_PARALLEL use `/39`.
- `COMPILER_DEVELOPMENT_GUARDRAILS.md:8-14` and `:250-259` still say `/5` is current and `/6` is proposed/inert.
- `docs/tasks/README.md:133-139` still says group chip files do not exist.
- `c09_readiness.rs:599-614` is named “declares its mechanical reads” but checks only 4 fields; `FoldChip` also reads `parse.nodes` (`fold.rs:76-99`) without declaring it (`fold.rs:122-131`).
- `c05_codec.rs:239-300` tests other wire payloads but not `AppendRecords` body sensitivity; `:632-651` compares only an empty batch.
- `c07_limits.rs:73-88` tests total records with sources only; it does not cover macro records or `stage_queue_bound`.
- Target concrete values remain proposal values; only 13 of 21 scalars are probe-filled by `attest` (`target.rs:327-341`, `:666-714`).

---

## 4. Enforcement gaps and protocol risks

Severity is relative to a clean Stage-B freeze: **P0** must be decided/fixed before dispatching Stage B chip work; **P1** must be fixed or explicitly narrowed in the freeze artifact; **P2** is lower-risk cleanup that can be deferred with an owner.

### A02-F01 (P0) — The chip boundary is not the frozen `RestrictedChip` path

**Evidence.**
- `compiler/src/chips/mod.rs:136-150` defines `Worker::handle(&self, task: TaskId, bus: &CompilerBus) -> Vec<Proposal>`; `:152-188` registers workers; `:309-357` drives them.
- `compiler/src/chips/constant_layout_init/fold.rs:142-149` implements `handle` by projecting from the full bus; the pure `compute` at `:153` is a second method, not the trait boundary.
- Repository grep: 65 `impl Worker for` sites in `compiler/src/chips` (including uncommitted candidates), 0 `RestrictedChip`, 0 `ChipAdapter`, 0 `install_projected` in `compiler/src`.
- T01 §4.1 (`T01_COMPILER_CONTRACT.md:84-101`) says new compiler chips must use `RestrictedChip`; AGENTS §3.3 and Guardrails §2.1 say zero-field chips compute from `&Input`.

**Impact.** All current and future Stage B chips copy an adapter+chip hybrid that can read any bus field from `handle`. The manifest read set is documentation/review only. The 65 worker sites (some uncommitted) make either migration or an explicit contract amendment urgent; two patterns must not coexist.

**Smallest fix.** Integrator decision: either (a) freeze `Worker` as the adapter boundary in T01, rename/document it as the application adapter, and require a narrow typed `Input` plus a separate projection function; or (b) add `RestrictedChip`/`ChipAdapter` wrappers and install through `Motherboard::install_projected`. In both cases add a negative test that a chip computation cannot receive `&CompilerBus`.

**Concrete read-set example.** `FoldChip` reads `bus.arenas.nodes` for the binary-node liveness check (`fold.rs:76-99`) but its manifest reads omit `parse.nodes` (`fold.rs:122-131`). `c09_readiness.rs:599-614` checks only `active.kind`, `lex.literals`, `constants.records`, and `config.limits`. This is a direct violation of “no undeclared accesses” that no test catches.

### A02-F02 (P0) — Reference, source, expansion, and continuation materialization is incomplete

**Evidence.**
- `G1DraftBody` (`compiler/src/records.rs:55-95`) has no `Source`, `Expansion`, or `Continuation` variant; the commit apply arms (`commit.rs:1498-1657`) and `CapacityPlan` (`commit.rs:1927-1972`) have no corresponding counts.
- `TaskDraft.continuation` is `Option<ContinuationId>` with an explicit comment that `ContinuationRef` migration is future work (`task.rs:1802-1810`); the Enqueue validation does not check it (`commit.rs:823-862`).
- `ContinuationRef`, `resolve_continuation_ref`, `check_continuation_committed`, `RecordLink`, `PendingDraft`, `NamePlan`, `ResolvedTable`, and predicted-range helpers exist (`task.rs:1660-1674`, `commit.rs:2140-2215`, `:2530-2807`), but `resolve_continuation_ref` has only its definition and no call site; `check_token_literal_reciprocal` is exercised only by tests.
- `StorePatch` is validated and appended to `patch_log` (`commit.rs:1848-1923`, `:1700-1713`) but no store is mutated; the module comment says materialization is group-owned (`commit.rs:23-26`).
- Record/payload references are generally opaque; the `Complete` prediction check covers only selected families and even then `_ => None` leaves `Continuation` and several other families unchecked (`commit.rs:886-957`).

**Impact.** T02 CT02 cannot import a `Source` through a chip proposal, T03 cannot create `Expansion` provenance, and T05 cannot create parse `Continuation`s; T09/T05 work that assumes a durable continuation is blocked. Any chip that emits `StorePatch` expecting state change observes only a log/version bump. This is the central `/39` protocol gap and matches the C01/C03 status rows.

**Smallest fix.** Either implement a frozen materialization/link protocol for `Source`, `Expansion`, `Continuation`, and `StorePatch` (typed bodies, predicted IDs, link validation, capacity, tests), or explicitly narrow T01 §4 and Stage B work orders to forbid these families until a later amendment. At minimum, validate `TaskDraft.continuation` and `RecordRef`s that the contract claims are checked.

### A02-F03 (P0) — The runtime bus is not assembled from the `/39` profile

**Evidence.**
- `CompilerBus::new` initializes `TaskKindRegistry::foundation()` and `StoreSchema::foundation()` (`compiler/src/bus.rs:974-990`).
- Gate 1/PP/LX/PA/TY/SE/IR/VF slices are installed by tests through public fields, e.g. `bus.kinds = TaskKindRegistry::m1_slice(); bus.schema = StoreSchema::m1_slice();` (`compiler/tests/c09_readiness.rs:39-48`).
- `TaskKindRegistry` slice constructors are cumulative and disjoint from the default; `CompilerBus::default()` cannot register `FoldChip` without replacement.
- No registration generator or `CompilerProfile` type exists in `compiler/src`.

**Impact.** A Stage B work order can run with a bus that silently lacks its task kind or schema field, or two integration tests can use different profiles. The `/39` artifact does not define one executable runtime profile.

**Smallest fix.** Add an integrator-owned `CompilerProfile` (or `CompilerBus::for_contract()`) that installs the exact `/39` registry/schema/route set; add a test asserting the profile's kind/schema inventory; keep the public fields for trusted host integration but not for normal setup.

### A02-F04 (P1) — Configured limits are not all enforced

**Evidence.**
- `stage_queue_bound` is declared (`limits.rs:67`), validated (`:154-174`), hashed (`contract.rs:659-661`), and snapshotted (`snapshot.rs:109-111`), but `commit.rs:1425-1438` calls `check_backpressure_capacity` with `bus.limits().max_queue_len` and stage `0`; no `stage_queues` exist. The code comment admits the projection.
- `Arenas::allocated_total()` (`bus.rs:679-707`) omits `self.macros.allocated()`; `CompilerBus::total_records()` (`:1021-1023`) therefore omits pre-existing macro records. `commit_proposals` adds only this batch's `new_macros` (`:2064-2086`), so repeated macro batches can exceed `max_records_total`.
- `max_const_bits` is checked in `FoldChip` (`fold.rs:216` and related), but `G1DraftBody::Const` commit materialization only counts records (`commit.rs:1068-1070`); a different authorized const producer would not be checked at commit.
- Rule `limits.every-configured-bound-enforced` (`contract.rs:63`) and T01 C01 row overstate the current guarantee.
- `c07_limits.rs:73-88` covers sources only; `:417-445` covers report bounds; there is no macro-total or stage-bound test.

**Impact.** Jobs can exceed the configured total/stage budget while `ensure_total_records` reports success; limit claims are not trustworthy for acceptance. The macro omission is a concrete code defect; the stage bound is a known projection that must be made explicit.

**Smallest fix.** Include `macros` in `allocated_total`; add a regression test with macro appends. Either implement per-stage queues/bounds or remove `stage_queue_bound` from the “enforced” claim and document it as a reserved configuration field; add a c07 negative test for the chosen behavior. Decide whether `max_const_bits` is chip-only (explicit rule) or commit-enforced.

### A02-F05 (P1) — Store-owner allowlist and single-writer invariants are wave-gated

**Evidence.**
- `check_store_owner_allowlist` returns `Ok(())` when a manifest claims no slice kind (`manifest.rs:1371-1408`); only slice-kind manifests are checked.
- `validate_patch` checks the chip’s write set and schema field, not the allowlist (`commit.rs:1882-1896`); `AppendRecords` authorization is likewise manifest/schema-only (`commit.rs:1791-1846`).
- `tasks.ready` is declared in `StoreSchema::foundation` (`manifest.rs:204-207`) and the allowlist has zero `tasks.queue.ready` rows (`manifest.rs:780-783`), but a control manifest with `writes: [tasks.queue.ready]` would register and commit.
- `c04_manifest.rs:238-256` only inspects the table; `:259-267` explicitly accepts a non-slice manifest without the allowlist.

**Impact.** The “zero allowlisted writers to `tasks.ready`” and single-writer claims are not mechanically true for foundation/control chips — exactly the T02 area that writes control/queue state. A malformed control chip can bypass the store-owner model.

**Smallest fix.** Enforce the allowlist at registration and commit for all manifests (or add explicit deny rows for `tasks.queue.ready`/`wires`); add negative tests for a control manifest writing `tasks.queue.ready` and for an unauthorized append. If wave-gating is intentional, narrow the T01 C04 claim and the allowlist comment so they do not assert universal enforcement.

### A02-F06 (P1) — Snapshot/trace/report completeness overclaims

**Evidence.**
- `push_wires` encodes only `batch.records.len()` for `AppendRecords` (`snapshot.rs:446-455`); neither families, record handles, nor bodies are encoded. `c05_codec.rs:239-300` tests Complete/Enqueue/Fail/AwaitHost payloads, not append bodies; `:632-651` compares empty append batches.
- `RoutingShell::clock_tick` records a tick only when `propagate` returns `Ok` (`routing.rs:627-633`); `propagate_inner` can return `Err` from idle join backpressure (`:347-366`) or the defensive selection guard while `clock_tick` still latches/advances (`:602-633`). The documented “exactly one TickRecord per tick” (`:596-601`) is therefore false on those paths.
- `CompilerBus::report` is public (`bus.rs:961-968`); direct `report.push` bypasses `push_tick_record`’s `max_ticks + 1` bound (`bus.rs:997-1005`) and the declared sole-append-site rule.
- `snapshot.wire-payloads-encoded` (`contract.rs:125`) and T01 C05’s “full wire proposal payloads” overstate what is encoded.

**Impact.** Two different pre-commit append batches can be indistinguishable in a snapshot; replay from a mid-tick snapshot loses work; an error tick can disappear from the bounded trace; the report bound can be bypassed. This is a determinism/replay risk for T13 and Host evidence.

**Smallest fix.** Encode append handles and bodies (or a canonical body hash) in `push_wires` and add a sensitivity test; record an error tick or amend the one-record claim; make `report` private/append-only or document the public field as a trusted host exception. Add a test for the error-tick lifecycle.

### A02-F07 (P1) — Optional artifact kinds are declared but not producible

**Evidence.**
- `ArtifactKind` has 8 variants and `requires_map` distinguishes mandatory/optional (`bus.rs:77-105`).
- `commit_proposals` rejects every non-map-mandatory artifact with “only map-mandatory artifacts are produced” (`commit.rs:1151-1162`).
- `check_map` implements the optional empty-offset branch (`bus.rs:122-162`), but no commit path reaches it; `c05_codec.rs:560` only tests the encoder.
- T01 `/10` declares the total 8-kind set and optional-kind rule; `/31` exercises only `Preprocessed`.

**Impact.** Stage B target/Host output (`Assembly`, `Object`, `Snapshot`, `Trace`) cannot use the frozen artifact path; T11/T02 must add a contract amendment or a second output path. The declared kind set is misleading as a production capability.

**Smallest fix.** Either allow optional kinds through `AppendRecords` with empty maps and add a commit test, or mark them extension-only in the contract and make the commit rejection the documented behavior. T11 output design must not assume the rejected path.

### A02-F08 (P1) — Freeze hash and version documentation are inconsistent

**Evidence.**
- `CONTRACT_VERSION:11` and `contract.rs:798` use `hash_excludes=... post-seed-declarations ...`.
- `compiler/README.md:40` still shows `group-declared-store-fields`; `:236-241` says the reconciliation is open; `:274` uses the old “fields added after foundation” wording.
- `COMPILER_SFL_MANIFEST.md:84-100` says the two-tier change was applied atomically to `README.md`, `FrozenSchema::encode`, and `freeze.rs`; the code is two-tier, the README is not.
- T01 C04/C05 status rows (`T01:876-880`) still say the hash-scope reconciliation is open.
- `PARALLEL_EXECUTION.md:7` says `/9` is current; `COMPILER_DEVELOPMENT_GUARDRAILS.md:8-14`/`:250-259` say `/5` is current; `docs/tasks/README.md:133-139` says group chip files do not exist.
- `freeze.rs:22-49` checks the version file and hash, not README/T01/PARALLEL/guardrail consistency.

**Impact.** Stage B agents read contradictory version numbers and hash-scope rules; the “atomic” freeze claim is false; a future hash-scope change could silently update code while docs remain stale. The hash itself recomputes correctly (`freeze.rs` passed), so this is a documentation/authority risk, not a hash corruption.

**Smallest fix.** Update all current-state docs to `/39` and `post-seed-declarations` (or generate the status block from `CONTRACT_VERSION`); add a freeze test that asserts the current version/hash token appears in README/T01/PARALLEL, or explicitly mark those docs historical. Reconcile T01 C04/C05 rows with the code.

### A02-F09 (P1) — The frozen hash does not cover numeric wire tags or all later-wave kind names

**Evidence.**
- `FrozenSchema::encode` hashes `RECORD_KINDS` names (`contract.rs:520`), `RecordFamily` names/ordinals (`:681-687`), `PROPOSAL_NAMES` (`:546`), and the M1 seed mapping/proposal tags (`:689-701`), but not `RecordRef::wire_tag` numbers, `TaskState` encoding tags, or later-wave `TaskKind` names/codes beyond the `STAGE_ASSIGNMENT` raw rows.
- `freeze.rs:390-468` cross-checks tags in a test, but a tag change does not change `CONTRACT_VERSION`’s hash.
- T01 C05 row (`T01:880`) admits numeric-value hashing remains an open item; `contract.rs:14-16` says the hash is not a source/semantic proof.

**Impact.** A wire-tag or later-wave kind-name change can leave the “frozen” hash unchanged; external artifacts/replays could be incompatible while the freeze test still passes. This is a real freeze-scope gap for Stage B serialization.

**Smallest fix.** Decide whether numeric tags and later-wave kind names are part of the frozen hash. If yes, encode them in `FrozenSchema::encode` and bump the version; if no, add them to `hash_excludes` and state the boundary in T01/C05. Extend `freeze.rs` accordingly.

### A02-F10 (P2) — Target attestation verifies only 13 of 21 scalar records

**Evidence.**
- `MEASURED_SCALARS` lists 13 kinds (`target.rs:327-341`); `TargetSpec::attest` fills only those plus `char.signed`, `wchar_t.signed`, `longdouble.format`, and the ABI/wchar fields (`:666-714`). The other 8 scalar records retain `aarch64_unknown_linux_gnu_unverified` proposal values.
- `ensure_codegen_ready` checks only `VerificationState` (`:1345-1347`); `ProbeReport::status` is presence-only and does not check identity (`:895-899`).
- `c02_target.rs:136-204` attests a caller-built signed report; it does not test the unmeasured scalar values.

**Impact.** A “verified” target can carry unverified `_Bool`, `signed/unsigned char`, `unsigned short/int/long/long long/__int128` values. Target codegen must not treat the whole `TargetSpec` as probed. The probe has not run in any case, so no codegen claim is currently valid.

**Smallest fix.** Derive the eight companion scalars from measured kinds where the C model guarantees it, or add them to `REQUIRED_PROBE_FIELDS` and `attest`; document the derivation and add a test asserting the verified target’s full scalar inventory. Keep `ensure_codegen_ready` fail-closed until the real probe runs.

### A02-F11 (P2) — Constructor and metadata sharp edges

**Evidence.**
- `CompilerConfig::new`/`CompilerBus::new` are public and infallible (`target.rs:1226-1250`, `bus.rs:974-990`); `try_new` validates (`target.rs:1257-1274`, `:1370-1375`).
- `CompilerConfig::try_new` canonicalizes before `validate`; `canonicalize` sorts/dedups (`target.rs:1277-1280`), so `ConfigError::DuplicateOption` is unreachable through the public constructors (`:1311-1318`); `c02_target.rs:293-303` only tests the error’s code string.
- `ResultRecord.version` is always `0` on completion (`commit.rs:1455-1462`) although the field claims the store/protocol version produced against (`task.rs:1750-1752`).
- Shell-generated `Fail` proposals do not attach `.with_task` (`routing.rs:421-474`); only the commit-failure recovery path does (`:584-587`).
- `TaskKind::CONTROL_START_JOB`/`CONTROL_IMPORT_SOURCE` cannot be routed (`routing.rs:95-98`) and are not built-in-handled (`:256-268`); T02 CT01/CT02 must use pins, built-ins, or a contract change.
- `ChipManifest.dependencies` is always empty in the 65 chips and is not validated (`manifest.rs:407-409`, `:671-751`).
- `TaskStore`/`report` comments claim latch/sole-append discipline, but the store is public and clearing happens in the shell before the no-op `latch`.

**Impact.** Integration code may construct an invalid bus/config; duplicate options silently merge; metadata (version/task/dependencies) is misleading; T02 control cannot dispatch its foundation kinds through the frozen shell. These are lower-risk than F01–F03 but should be assigned before integration.

**Smallest fix.** Require `try_new` or validate in `new`; add a duplicate-option test; set/remove `ResultRecord.version`; attach the task to shell diagnostics; define built-in handlers or an explicit foundation-route exception; populate/validate dependencies or remove the field.

---

## 5. Minimal Stage-B freeze checklist

| Gate | Required freeze decision / fix | Owner | Primary paths | Depends on |
|---|---|---|---|---|
| SB-01 Chip boundary | Choose `RestrictedChip` + adapter or explicitly amend T01 to freeze `Worker` as the adapter; require narrow typed input and no `&CompilerBus` in semantic compute; add a negative test | T01 integrator + T02 | `compiler/src/chips/mod.rs`, `compiler/src/routing.rs`, `T01_COMPILER_CONTRACT.md` §§4/4.1, `AGENTS.md` §3.3 | none; blocks all chip work |
| SB-02 Bus profile | Add one `/39` profile/registration constructor; assert kind/schema/route inventory; stop relying on manual public-field assignment in tests | T01 integrator | `compiler/src/bus.rs`, `compiler/src/manifest.rs`, `compiler/src/task.rs`, `compiler/tests/c08_gate1.rs`, `c09_readiness.rs` | SB-01 for worker installation semantics |
| SB-03 Reference/continuation protocol | Implement materialization/link validation for `Source`, `Expansion`, `Continuation`, and `StorePatch`, or explicitly narrow T01 §4 and Stage B scopes; validate `TaskDraft.continuation` | T01 + T02/T03/T05/T09 | `compiler/src/commit.rs`, `task.rs`, `records.rs`, `ids.rs`, `c03_task.rs` | SB-02 |
| SB-04 Limits closure | Include `macros` in total records; decide/implement `stage_queue_bound`; decide const-bit scope; add c07 tests | T01 integrator / limits track | `compiler/src/bus.rs`, `limits.rs`, `commit.rs`, `compiler/tests/c07_limits.rs` | SB-02 |
| SB-05 Store-owner enforcement | Enforce allowlist/single-writer for all manifests or explicitly narrow the claim; add negative tests for `tasks.queue.ready`/`wires` writes | T01 integrator + T02/C04 | `compiler/src/manifest.rs`, `commit.rs`, `compiler/tests/c04_manifest.rs` | SB-04 |
| SB-06 Snapshot/trace closure | Encode `AppendRecords` handles/bodies (or a canonical body hash); record/define error ticks; make `report` bounded and append-only; add c05/c06 tests | C05/T13 integrator | `compiler/src/snapshot.rs`, `routing.rs`, `bus.rs`, `compiler/tests/c05_codec.rs`, `c06_routing.rs` | SB-03 for body shapes |
| SB-07 Artifact kinds | Allow optional artifact kinds with empty maps or declare them extension-only; align T11/T02 output plans | T01 + T11/T02 | `compiler/src/commit.rs`, `bus.rs`, `compiler/tests/c10_pp01.rs`/`c31_emit.rs` | SB-03 |
| SB-08 Version/hash/docs | Reconcile README/T01/PARALLEL/guardrails/docs README with `/39`; decide numeric-tag/later-kind hash scope; add consistency assertions | Docs integrator + T01 | `compiler/contracts/CONTRACT_VERSION`, `compiler/README.md`, `compiler/src/contract.rs`, `T01_COMPILER_CONTRACT.md`, `PARALLEL_EXECUTION.md`, `COMPILER_DEVELOPMENT_GUARDRAILS.md`, `docs/tasks/README.md`, `compiler/tests/freeze.rs` | SB-04/SB-06 for final hash inventory |
| SB-09 Target boundary | Document/probe the 8 unmeasured scalars; keep codegen fail-closed; complete probe→H07→attest handoff | T01 + H01/H07/C02 | `compiler/src/target.rs`, `compiler/contracts/target/aarch64-linux-probe.txt`, `compiler/tests/c02_target.rs` | real Linux probe substrate |
| SB-10 Constructor hardening | Require validated config/bus construction; fix duplicate-option behavior; relocate `CompilerBus::try_new` next to `new` | T01 integrator | `compiler/src/target.rs`, `compiler/src/bus.rs`, `compiler/tests/c02_target.rs` | none |

---

## 6. Recommended work orders and dependencies

| Work order | Scope | Owner | Paths | Depends on | Exit criterion |
|---|---|---|---|---|---|
| WO-A02-01 | Chip boundary decision and contract text | T01 integrator, T02 review | `compiler/src/chips/mod.rs`, `T01_COMPILER_CONTRACT.md` §4.1, `AGENTS.md` §3.3 | none | One frozen pattern; negative test; no `&CompilerBus` in semantic compute |
| WO-A02-02 | Profile/registration constructor | T01 integrator | `compiler/src/bus.rs`, `manifest.rs`, `task.rs`, `c08`/`c09` | WO-A02-01 | `CompilerBus::for_contract()` (or equivalent) installs `/39` profile; inventory test |
| WO-A02-03 | Reference/link materialization or explicit deferral | T01 + T02/T03/T05/T09 | `compiler/src/commit.rs`, `task.rs`, `records.rs`, `ids.rs`, `c03_task.rs` | WO-A02-02 | Every claimed reference checked; source/expansion/continuation have a path or a documented prohibition |
| WO-A02-04 | Limits correction | T01/limits track | `compiler/src/bus.rs`, `limits.rs`, `commit.rs`, `c07_limits.rs` | WO-A02-02 | Macro total test; stage-bound decision implemented/tested; const-bit rule explicit |
| WO-A02-05 | Store-owner enforcement | T01 + T02/C04 | `manifest.rs`, `commit.rs`, `c04_manifest.rs` | WO-A02-04 | Control-manifest queue/wire write rejected; negative tests |
| WO-A02-06 | Snapshot/trace/report completeness | C05/T13 integrator | `snapshot.rs`, `routing.rs`, `bus.rs`, `c05_codec.rs`, `c06_routing.rs` | WO-A02-03 | Append-body sensitivity test; error-tick test; report bound test |
| WO-A02-07 | Artifact-kind production decision | T01 + T11/T02 | `commit.rs`, `bus.rs`, `c10_pp01.rs`, `c31_emit.rs` | WO-A02-03 | Optional kinds accepted or explicitly extension-only; T11 plan aligned |
| WO-A02-08 | Freeze-doc reconciliation and hash-scope decision | Docs integrator + T01 | `CONTRACT_VERSION`, `contract.rs`, `README.md`, T01, PARALLEL, guardrails, docs README, `freeze.rs` | WO-A02-04/06 | `/39` and one hash-scope token everywhere; freeze test asserts it |
| WO-A02-09 | Target scalar/attestation completion | H01/H07/C02 + T01 | `target.rs`, probe fixture, `c02_target.rs` | real probe substrate | All 21 scalars either probed or explicitly derived; `verified` claim scoped |
| WO-A02-10 | Constructor/metadata cleanup | T01 integrator | `target.rs`, `bus.rs`, `routing.rs`, `c02_target.rs` | WO-A02-02 | Validated construction path; duplicate-option test; result version/task attachment/dependencies resolved |

Dependency graph in short: **WO-01 → WO-02 → {WO-03, WO-04, WO-10} → {WO-05, WO-06, WO-07} → WO-08**; **WO-09** runs on the independent target-probe critical path and does not block frontend Stage B.

---

## 7. Checks actually run, and unrun checks

### 7.1 Ran by this audit

```sh
cargo test --locked --manifest-path compiler/Cargo.toml \
  --test c01_arena --test c02_target --test c03_task --test c04_manifest \
  --test c05_codec --test c06_routing --test c07_limits --test c08_gate1 \
  --test c09_readiness
```

Result: **168 passed, 0 failed, 0 ignored** across 9 test binaries (7 + 11 + 46 + 15 + 21 + 15 + 18 + 21 + 14).

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test freeze -- --test-threads=1
```

Result: **14 passed, 0 failed, 0 ignored**. The test build emitted **137 warnings**, all dead-code/unused warnings from the uncommitted candidate modules compiled through `parse/mod.rs`; this is not a clean clippy baseline. No foundation test failed.

### 7.2 Not run (do not infer)

- Full `cargo test --workspace` / compiler `c10`–`c39` / `h04_candidate`; only the targeted 182 tests above were run by A02.
- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`; the dirty workspace is known to fail clippy on uncommitted drafts per the dispatch ledger.
- Root `cc-silicon` package tests; not part of A02.
- Linux target probe, H01 harness, H07 attestation, any AArch64 cross-compile/link/run.
- GCC torture corpus, H04 candidate driver, H05 isolation, M1 end-to-end execution.
- Any re-run of the `/39` hash after a change (none was made); the hash was only checked with the freeze test.

The existing `docs/tasks/DISPATCH_LEDGER.md` reports 654 compiler tests across 44 suites from the dispatch session; A02 did **not** re-run that set and does not use it as evidence for this audit.

---

## 8. Open questions for the integrator

1. **Chip boundary:** Is `Worker` the accepted adapter contract, or must Stage B migrate all 65 chips to `RestrictedChip`/`ChipAdapter`? What is the smallest contract amendment that preserves the existing work?
2. **Materialization scope:** Are `Source`, `Expansion`, and `Continuation` part of `/40`, or explicitly deferred? If deferred, how do T02 source import, T03 expansion provenance, and T05 parse framing proceed?
3. **`StorePatch` semantics:** Is it intended only as an audit/version log, or must it mutate a store? If only a log, Stage B chips must not use it for state.
4. **Stage bounds:** Is `stage_queue_bound` a real future bound or a placeholder? Should the “every configured bound” rule be narrowed until `stage_queues` exist?
5. **Artifact kinds:** Are `Assembly`/`Object`/`Snapshot`/`Trace` frozen for Stage B, or extension-only? The current commit rejects them.
6. **Hash scope:** Should numeric wire tags and later-wave kind names participate in the frozen hash, or is the two-tier seed intentionally name/shape-only?
7. **Target scalar coverage:** Must a “verified” target have all 21 scalars probed/derived, or is the 13-scalar probe set the intended contract?
8. **Doc generation:** Should the README/T01/PARALLEL status blocks be generated from `CONTRACT_VERSION` so `/5`, `/9`, and “no chips” drift cannot recur?

---

## 9. Bottom line

`/39` is a credible, tested **protocol envelope** and a reasonable base for Stage B **only if** the freeze resolves the chip-boundary choice, the reference/continuation/source materialization gap, the runtime profile assembly, the limit enforcement claims, and the snapshot/replay completeness claims. The remaining findings are either explicit deferrals that must be documented as such or lower-risk cleanups. No test run here supports a target, ABI, compiler, or torture claim.
