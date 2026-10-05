# M1-REC / M1-WS Fixture Testability Audit (read-only)

| Field | Value |
|---|---|
| **Date** | 2026-10-05 |
| **Status** | Read-only audit record. Not a fix, not an acceptance, not a contract change, not a `/6` freeze. No reviewed file was modified; this report is the only artifact added. |
| **Scope** | `M1-REC-01..05` and `M1-WS-01..05` (`docs/tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md` §6.3/§6.4), gates `G9`/`G10`/`G15` (same file §8), and the determinism/projection/access machinery they depend on. |
| **Baseline** | Working tree at audit time. `/5` is current: `t01-c01-c06/5`, hash `61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5` (`compiler/contracts/CONTRACT_VERSION`). `/6` is unfrozen; [ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) is **PROPOSED**, [CDR-M1-0001](../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md) is a pending decision request, the M1 proposal is DRAFT. |
| **Method** | Read-only document and source inspection. No Cargo command, test, probe, or compiler run. Line numbers are as of this audit and may drift. |
| **Non-claims** | No compiler capability, pass, probe result, or `/6` acceptance is claimed. Findings concern whether the planned fixtures are executable and well-defined; a "not testable under `/5`" finding is not a code defect claim. |

## 1. Testability matrix

"Executable against `/5`" means the frozen C01–C06 protocol surface can express the scenario and an assertion today. It does not mean the full M1 fixture passes; the M1 language chips (T03–T09) do not exist.

| Fixture | Stated expectation | `/5` today | `/6` dependency | Verdict |
|---|---|---|---|---|
| `M1-REC-01` | Replay: identical record IDs, diagnostic order, per-tick trace | `Snapshot::capture` + host `Trace` replay test exists (`compiler/tests/c05_codec.rs:85–95`), but only for the synthetic C06 scenario; `/5` `Trace` is a scheduler trace (`selected`, `ready_len`), not a semantic trace | CDR A7: host `Trace` must become a **derived view**; semantic-trace encoding undefined | Split: protocol-level replay testable; M1-level and `/6` projection blocked (RW-01) |
| `M1-REC-02` | Child fails during parse/type: parent fails once, one diagnostic per error, no duplicate completion | Commit protocol covers `DuplicateCompletion`/`Fail`; `TaskNotTransitioned` does not exist in `/5` (`compiler/src/commit.rs`); no parse/type chips | None beyond M1 chips for the flow; the "dispatched task must transition" invariant is `/6`-proposed | Protocol subset testable; M1 flow symbolic |
| `M1-REC-03` | T+1 visibility; selection order `(stage ordinal, phase priority, enqueue ordinal, TaskId)`; quota-1 canonical projection | T+1 and `(phase priority, enqueue ordinal, TaskId)` selection implemented/tested (`routing.rs:198–216`, `task.rs:479–482`, `compiler/tests/c06_routing.rs:190–209`); no `stage ordinal`, no `max_inflight_per_tick`, no scheduler snapshot | Projection encoder, `StageAssignment`, scheduler snapshot all proposed | Partly testable; quota-1 gate blocked (RW-01, RW-05) |
| `M1-REC-04` | Malformed bytes through lexer: bounded ticks, no panic | No lexer chips (T04 planned) | T04 chips; T13 gate | Not executable |
| `M1-REC-05` | Recovery after a bad token does not swallow the next function | No parser chips (T05/PA38 planned) | T04/T05 chips; T13 gate | Not executable |
| `M1-WS-01` | Patch to an undeclared field rejected; no mutation | Implemented: registration rejects unknown fields (`compiler/tests/c03_task.rs:329–340`); `UndeclaredStoreField` commit branch is defense in depth; validation runs before apply | None for the protocol part | Testable at protocol level (needs an explicit commit-branch test) |
| `M1-WS-02` | Wrong owner / stale version rejected; failed batch commits nothing | Implemented and tested: `ChipNotOwner`, `PatchOwnerMismatch`, `WriteNotDeclared`, `StaleVersion`, `ReadOnlyStore` (`c03_task.rs:235–326`); validation pass precedes apply (`commit.rs:350–486`) | None for the protocol part | Testable |
| `M1-WS-03` | Worker receives a kind it does not own: semantic state unchanged except for the **scheduling diagnostic** | No scheduling diagnostic exists (`DiagGroup` has no scheduling/routing family, `diagnostic.rs:36–70`); kind/owner check is patch-only; non-patch proposals bypass it; the mismatch is unreachable through the checked `Enqueue` path; the route-entry chip is never compared with the task owner | CDR A9 names a "structured tick diagnostic" class for scheduling failures but leaves names/codes open; the WS-03 scenario is not in its list | Undefined diagnostic + behavior gap (RW-02) |
| `M1-WS-04` | Manifest audit: exact reads/writes/phase/category/backend; no whole-store grant | `ManifestError` has no `StoreOwnerViolation` (proposal OB-48); whole-store prohibition and the M1 chip set do not exist | T02/T01 manifest/allowlist extension | `/6`-blocked; static audit only |
| `M1-WS-05` | Adapter projection: changing a declared output field succeeds; undeclared registers/fields byte-for-byte unchanged | No adapters or projected chips are installed in the compiler crate; manifest `reads` are declarative and unenforced (T01 §4.1 residual); the oracle is declaration-relative | Adapter/manifest binding, snapshot-diff tooling | Circular oracle; not executable (RW-03) |

## 2. Findings

### RW-01 (High) — The quota-1 "canonical semantic comparison projection" is circular/underdefined

Applies to `M1-REC-03`, `G15`, `D8` (`docs/tasks/M1_TARGET_ACCEPTANCE.md:359`), ADR-0002 §2.8 item 1/§2.9, CDR A4, and M1 proposal §10.5/§7.

1. **The comparator contains scheduler output while claiming to exclude it.** The projection is defined as "semantic records/results/diagnostics + semantic trace, excluding scheduler-only registers: stage queues, in-flight set, `dispatch_cursor`, `stage_assignment_version`, `PipelineMetrics`" (M1 §6.3, ADR-0002:413–425). The M1 proposal then defines the semantic trace as "(tick, ordered dispatched `TaskId`s, per-task outcome, produced records, diagnostics)" (`M1_PART_A_CONTRACT_PROPOSAL.md:2946–2947`). Tick assignment and dispatch order are produced by the excluded registers; record IDs are allocated in commit-apply order, which is scheduler order. So the projection either keeps scheduler-derived data (fails to isolate semantics) or excludes it and leaves the "semantic trace" without a defined content.
2. **The `/5` side of the comparison does not exist.** The baseline is "the `/5` + M1 single-active-task design" — a planned reference implementation, not the frozen C06 shell. In `/5` there is no `stage_queues`, in-flight set, `dispatch_cursor`, `stage_assignment_version`, or `PipelineMetrics` to exclude, and the only trace (`snapshot.rs:728–833`) records scheduler-visible fields (`selected`, `ready_len`, outcome counters) and **no** produced record/diagnostic IDs. The projection rule is therefore enumerated against `/6` registers only and has no executable meaning on the `/5` side.
3. **No projection encoder is specified anywhere.** The candidate test is only a name (`quota1_canonical_projection_equivalence`, CDR:1007–1011). There is no artifact, hash, or comparison function for the "semantic projection"; `grep semantic compiler/src/snapshot.rs` returns nothing. The fixture cannot be written as a concrete assertion until the projection is encoded and frozen.
4. **This is not merely "not implemented yet".** A comparator whose exclusion list is defined by the same scheduler it validates, and whose "semantic" side is populated from scheduler-generated ordering, cannot prove "the new machinery is a strict generalization" (ADR-0002:421–422) without an independently frozen semantic vocabulary. CDR A4 itself warns the exclusion list "must not become a blanket escape hatch"; as written it is also not independently checkable.

### RW-02 (High) — `M1-WS-03`'s "scheduling diagnostic" is undefined, and the expected behavior does not hold for non-patch proposals

1. **No scheduling diagnostic exists in `/5`.** `DiagGroup` contains `Protocol`, `Arena`, `Config`, `Target`, `Manifest`, `Task`, `Unsupported`, `Internal` (`compiler/src/diagnostic.rs:36–70`) — no scheduling/routing family. The concrete carriers are:
   - routing failures (`Unregistered`/`Unsupported`) → `DiagnosticDraft::unsupported` = `Unsupported(1)` (`routing.rs:287–310`, `diagnostic.rs:115–118`);
   - commit failures → `CommitError::to_diagnostic` = `Protocol(1)` (`commit.rs:297–304`), including `TaskKindNotAccepted`.
   Neither is a "scheduling diagnostic", and no code, group, or record type is defined under that name.
2. **The owner/kind check only fires for patch proposals.** `TaskKindNotAccepted` is raised inside `validate_patch` (`commit.rs:608–617`). The `Complete`, `Fail`, and `AwaitHost` arms validate only the inner task ID (`commit.rs:441–470`); the apply pass does not re-check the owner manifest. A task whose owner does not accept its kind can therefore `Complete` successfully and mutate semantic state — directly contradicting `M1-WS-03`'s "semantic state is unchanged except for the scheduling diagnostic". This is the same gap recorded as DOC-01 in the 2026-10-05 documentation review.
3. **The mismatch state is not reachable through the checked protocol.** `Enqueue` validates `destination.accepts_kind(draft.kind)` (`commit.rs:410–419`), so a producer cannot create an owner/kind-mismatched task. It can enter only through `bootstrap_task`/direct arena access (`bus.rs:519–528`), i.e. the trusted host/test boundary. Separately, the routing shell never compares the route entry's `chip` with `task.owner`; it tags the proposal with `owner` (`routing.rs:265–317`), so "a worker receives a kind it does not own" is not even expressible as a routing state in `/5`.
4. **`/6` does not close it.** CDR A9 (`:973–992`) separates `ManifestError`, a pre-dispatch "structured tick diagnostic" for scheduling failures (`SelectionBatchOverflow`, `DuplicateSelection`), and `CommitError` (`BackpressureCapacity`, `CrossTaskWriteConflict`). It states that exact names/numeric codes remain a `/6` item, and the WS-03 scenario (owner/kind mismatch on a dispatched task) is not among the listed scheduling errors. The `/6` TickRecord/`PipelineMetrics` surface does not define a per-task scheduling diagnostic either.
5. **Ambiguity in "semantic state".** Even on the patch path, the failure transition changes the task record to `Failed` and may commit a diagnostic; the fixture does not say whether task lifecycle and diagnostics are inside or outside "semantic state". As written, `M1-WS-03` has no deterministic, checkable expected result.

### RW-03 (Medium–High) — `M1-WS-05`'s adapter-projection oracle is declaration-relative (circular)

`M1-WS-05` asserts that "changing a declared output field succeeds; undeclared registers and undeclared fields stay byte-for-byte unchanged". "Declared" is the chip manifest/adapter under test, so the fixture's oracle is the same declaration it is supposed to validate:

- A behavior-vs-declaration comparison passes whenever the adapter writes exactly what it declares, even if the declaration itself is too broad (e.g. declares a whole store, or declares the wrong owner's field). `M1-WS-04` is a static audit against the same manifest/schema and cannot supply an independent behavioral oracle; the mandatory `ChipId`-keyed allowlist and whole-store rejection (`StoreOwnerViolation`) are proposal-only and absent from `/5` (`manifest.rs:297–383`; OB-48).
- `/5` provides no adapter binding to test. The compiler crate installs no `ChipAdapter`/`ProjectedChip`; T01 §4.1 records that "adapters still receive `&mut CompilerBus`; field-level isolation is a review/lint obligation until schema-generated private projections exist" (`M1_PART_A_CONTRACT_PROPOSAL.md:3416–3418`). Manifest `reads` are declarative metadata; nothing enforces them.
- "Undeclared registers stay byte-for-byte unchanged" could in principle use a `Snapshot` diff (`snapshot.rs` encodes wires), but with no adapter to run and no expected-write-set oracle, the assertion is untestable today and circular at `/6` unless an independent per-fixture write set is defined.

### RW-04 (Medium) — Readiness tiers overstate what a `/5`/T01 freeze unblocks

The readiness table (`M1_VERTICAL_SLICE_ACCEPTANCE.md:389–392`) assigns all `M1-WS-*`/`M1-REC-*` rows to the "Record-bound … T01 freeze" tier. Actual prerequisites differ:

- `M1-REC-04`/`M1-REC-05` require T04/T05 chip behavior (lexer/parser recovery), not only record binding.
- `M1-REC-03` and `G15` require the `/6` scheduler and projection (ADR-0002 PROPOSED); they are not runnable against `/5` at all.
- `M1-WS-03` requires the diagnostic definition of RW-02; `M1-WS-04`/`M1-WS-05` require T02, the M1 chip set, the manifest allowlist extension, and adapter projections.
- Only `M1-WS-01`/`M1-WS-02` (protocol-level) and the T+1/selection-order parts of `M1-REC-03` and the replay parts of `M1-REC-01` are expressible against `/5` today.

This matters because the tier label is used to schedule work: a contributor could believe these fixtures are blocked on a schema freeze when they are in fact blocked on unimplemented chips and an unfrozen projection.

### RW-05 (Medium) — Cross-version projection vocabulary is undefined

The projection compares "the `RecordRef`-reachable typed records" across two designs whose record vocabularies differ: `/5` encodes 24 `RECORD_KINDS`/24 `RecordRef` variants with `RecordFamily` absent (CDR rev 48 audit), while `/6` proposes additional families/tags (e.g. `Literal`, tags 24/25/26) and a `RecordFamily` inventory. No common semantic vocabulary, version mapping, or projection schema exists, and the `/5` `Trace` is explicitly slated to become "a derived view" (CDR A7). Until that mapping is frozen, "semantic records/results/diagnostics" is not a well-defined comparison domain.

## 3. Recommendations (doc-only; each needs T01/owner co-freeze)

1. **RW-01:** freeze an explicit projection artifact (schema + encoder + comparison function) before `M1-REC-03`/`G15`/`D8` can be run. Separate the comparator into (a) semantic records/results/diagnostics, (b) an ordered semantic trace with an explicitly frozen field list, and (c) a scheduler snapshot compared separately. Do not include dispatch order or tick assignment in the "semantic" side unless they are named as scheduler invariants, and state the cross-version record mapping. Re-state the gate as `/6`-blocked, not "T01 freeze".
2. **RW-02:** define the scheduling/ownership diagnostic carrier (group, code, whether it is a committed `DiagnosticRecord` or a tick diagnostic) and extend the owner/kind acceptance check to all proposal kinds (`Complete`/`Fail`/`AwaitHost`), or explicitly scope `M1-WS-03` to patch proposals and say so. Define whether task lifecycle/diagnostics count as "semantic state". Add a negative test for a non-patch proposal from a non-owning chip.
3. **RW-03:** give `M1-WS-05` an independent expected-write-set oracle per fixture (derived from the semantic task, not from the adapter's own manifest) and pair it with `M1-WS-04`; record that `/5` has no adapter projection binding and that read enforcement is a review/lint obligation.
4. **RW-04:** split the readiness rows: `/5`-executable protocol parts (`M1-WS-01/02`, replay hash of `M1-REC-01`, T+1/order of `M1-REC-03`) vs chip-blocked (`M1-REC-04/05`, `M1-WS-04`) vs `/6`-pipeline-blocked (`M1-REC-03` projection, `G15`).
5. **RW-05:** freeze the `/5`↔`/6` record mapping as part of the same projection freeze; make the "host `Trace` becomes a derived view" change explicit in the `/6` inventory.

## 4. Verification record

- `git diff --check` → exit 0 (no whitespace errors or conflict markers in tracked modifications).
- The new report is untracked, so `git diff --check` does not cover it; `git diff --no-index --check /dev/null docs/reviews/2026-10-05_M1_REC_WS_FIXTURE_AUDIT.md` reported no whitespace errors (exit 1 is the expected "files differ" result).
- No Cargo fmt/clippy/test was run; no probe, compiler, or fixture was executed.
- No reviewed file was edited; `/5` remains current and `/6` remains unfrozen.
