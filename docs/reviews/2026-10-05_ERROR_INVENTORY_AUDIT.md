# Error and Diagnostic Inventory Audit — 2026-10-05

## Status and scope

- Status: **read-only audit record**. This document is not an ADR, contract
  approval, `/6` freeze, sign-off, or implementation authorization. It changes
  no accepted decision and closes no OB item.
- Scope: the structured error and diagnostic inventory across the compiler
  application and the current contract/proposal documents:
  `compiler/src/*.rs`, `compiler/tests/*.rs`, T01, the M1 Part A proposal, the
  CDR, ADR-0002, and the T02–T09/T13 packages.
- Baseline: the working tree at audit time, including uncommitted and untracked
  documents. Line numbers correspond to that version and may drift.
- This audit **edited no reviewed file** and added no code. The only file added
  is this record.

## Method and evidence

- Variant counts were extracted mechanically from the current sources
  (`compiler/src/commit.rs`, `manifest.rs`, `limits.rs`, `arena.rs`, `task.rs`,
  `target.rs`, `routing.rs`, `intern.rs`, `diagnostic.rs`, `bus.rs`,
  `contract.rs`, `snapshot.rs`) and cross-checked by hand.
- Proposed names and counts were extracted from the M1 Part A proposal §6.4
  block (lines 2390–2459) and classified against the §6.4 prose and the
  T01/T02/CDR/ADR error-classification text.
- No `cargo fmt`/`clippy`/`test` run was performed (documentation-only audit);
  no probe or candidate compiler was executed.

## 1. Summary

- Implemented error enums: **11 enums / 69 variants** — `CommitError` 20,
  `LimitError` 8, `ArenaError` 4, `ManifestError` 13,
  `ManifestRegistryError` 2, `SchemaError` 2, `RegistryError` 5,
  `ConfigError` 2, `ProbeError` 9, `RouteError` 2, `InternError` 2.
- The proposal's §6.4 block adds **46 proposed `CommitError` names** on top of
  the 20 implemented variants (the parent inventory's "20 + 46"). It also
  proposes **3 `ManifestError` names**, `ConfigError` additions
  (`MaxTicksOverflow` plus unnamed quota/queue/fairness variants), and
  dispatcher/scheduling failure names that are explicitly *not* `CommitError`s.
- **Per-variant diagnostic codes are not assigned anywhere.** The
  implementation maps every variant of a family to one coarse
  `DiagnosticCode { group, code }`; the documents say stable codes are "fixed
  at `/6`". The single concrete stable code string in any document is
  `config.max_ticks_overflow` (proposed, not implemented).
- **`DiagnosticDraft` vs `DiagnosticProposal`** is an open naming conflict:
  code and proposal use `DiagnosticDraft`; T01 §5 and T02 CT11 say
  `DiagnosticProposal`; T04 records the choice as an open T01 `/6` item
  (OB-32).
- **`EffectMaskUnsupported` carrier is unresolved** (OB-27): the proposal §6.4
  block lists it as a `CommitError` while T07/CDR rev 46 classify a nonzero
  mask as a chip `DiagnosticDraft`/typed unsupported diagnostic, and T13/VF06
  treats it as a verification check; the same §6.4 section simultaneously says
  chip diagnostics are "never a `CommitError`".
- The dispatcher/scheduling failures have **no carrier type, family, or code**
  in code or in the proposed stable-code inventory; only two names remain
  operative (`SelectionBatchOverflow`, `DuplicateSelection`), and
  `DispatchBudgetExceeded` still appears unqualified in several documents
  despite its rev-51/54 removal.

## 2. Current implementation inventory

### 2.1 Diagnostic mapping

| Error type | File | Variants | `to_diagnostic` mapping | Wrapped by |
|---|---|---|---|---|
| `CommitError` | `compiler/src/commit.rs:91–213` | 20 | all → `(DiagGroup::Protocol, 1)` (`commit.rs:299–305`) | returned by `commit_proposals`; converted by `fail_selected` |
| `LimitError` | `compiler/src/limits.rs:80–133` | 8 | none | `CommitError::Limit` (`commit.rs:313–317`) |
| `ArenaError` | `compiler/src/arena.rs:45–76` | 4 | none | `CommitError::Capacity` (`commit.rs:307–311`), `LimitError::Arena` |
| `ManifestError` | `compiler/src/manifest.rs:297–383` | 13 | all → `(DiagGroup::Manifest, 1)` (`manifest.rs:449–454`) | `ManifestRegistryError` |
| `ManifestRegistryError` | `compiler/src/manifest.rs:459–469` | 2 | none | returned by `ManifestRegistry::register` |
| `SchemaError` | `compiler/src/manifest.rs:117–128` | 2 | none | `StoreSchema::declare` |
| `RegistryError` | `compiler/src/task.rs:196–224` | 5 | all → `(DiagGroup::Task, 2)` (`task.rs:254–256`) | `TaskKindRegistry::register` |
| `ConfigError` | `compiler/src/target.rs:1096–1107` | 2 | `TargetUnverified` → `(Target, 1)`; `DuplicateOption` → `(Config, 1)` (`target.rs:1111–1122`) | `validate`/`ensure_codegen_ready`/`require_verified` |
| `ProbeError` | `compiler/src/target.rs:721–769` | 9 | none | `TargetSpec::attest`/`ProbeReport` parsing |
| `RouteError` | `compiler/src/routing.rs:41–52` | 2 | none | `RoutingTable::register` |
| `InternError` | `compiler/src/intern.rs:17–30` | 2 | none | `InternTable::intern` |

The generic chip-diagnostic path is `DiagnosticDraft::unsupported` →
`(DiagGroup::Unsupported, 1)` (`diagnostic.rs:116–118`), emitted by the routing
shell for unsupported, unregistered, and registered-without-handler task kinds
(`routing.rs:287–310`). The commit-failure recovery diagnostic is built from
`CommitError::to_diagnostic` with the task attached (`routing.rs:347–381`).

### 2.2 Observations

1. **Coarse per-family codes.** All 20 `CommitError` variants share
   `(Protocol, 1)`, all 13 `ManifestError` variants share `(Manifest, 1)`, and
   all 5 `RegistryError` variants share `(Task, 2)`. `DiagnosticCode` is
   structured, but tooling cannot match a variant without parsing the message,
   despite the `diagnostic.rs:34–35` rationale ("so tooling can match without
   parsing prose"). No document freezes these coarse codes as final.
2. **The `diagnostic.rs` module claim is stronger than the code.** Lines 4–7
   state that "Every error type in this crate can be mapped to a
   [`DiagnosticDraft`]". Seven error types (`ArenaError`, `LimitError`,
   `InternError`, `RouteError`, `ProbeError`, `SchemaError`,
   `ManifestRegistryError`) have no `to_diagnostic`; some are reachable only
   through wrapping.
3. **Unused groups.** `DiagGroup::Arena` and `DiagGroup::Internal` are defined
   (`diagnostic.rs:40–41`, `53`) and hashed (`contract.rs:118–128`) but no code
   path produces them. The proposal selects `DiagGroup::Internal` for the
   proposed `InternalMissingArena` (§15.2 item 8, line 4320–4321).
4. **Duplicate dangling-parent carrier.** `CommitError::DanglingParent`
   (`commit.rs:128–131`, raised at `commit.rs:427`) and
   `LimitError::DanglingParent` (`limits.rs:129–132`, raised at
   `bus.rs:464`, wrapped as `CommitError::Limit`) describe the same condition
   through two carriers.
5. **No dispatcher exists.** `/5` has `RoutingShell::select` selecting one task
   (`routing.rs:198–216`); there is no `SelectionBatch`, no dispatcher error
   type, and no `SelectionBatchOverflow`/`DuplicateSelection` code.
6. **Snapshot visibility.** Committed diagnostics encode the group name and the
   `u16` code (`snapshot.rs:682–687`), so a later per-variant code assignment
   is replay-visible. The `/5` contract hash includes only the 8 `DiagGroup`
   names (`contract.rs:195`), not the per-variant code numbers.

## 3. Proposed inventory in the contract documents

### 3.1 `CommitError`: 20 implemented + 46 proposed

- The §6.4 block (proposal lines 2392–2442, plus the cross-task entry at 2442
  and the pipeline entry at 2445) lists **46 names** that are new relative to
  the implemented 20. Appendix A reproduces the list.
- The 46 include two wrappers of existing types (`DraftCapacity(ArenaError)`,
  `InternCapacity(InternError)`), one disputed carrier (`EffectMaskUnsupported`,
  §6), and two pipeline entries (`CrossTaskWriteConflict`,
  `BackpressureCapacity`).
- **The 20 implemented variants are not re-listed** in the §6.4 block. §15.2
  item 8 (lines 4319–4322) says the exact numeric codes are "fixed at `/6`
  from the §6.4 list"; it is ambiguous whether the implemented 20 receive
  stable codes from that list or keep the current coarse mapping (finding
  ERR-07).
- The block header says "stable codes at /6" but assigns no numeric code.

### 3.2 `ManifestError`: 13 implemented + 3 proposed

- Proposed additions: `StoreOwnerViolation { chip, store, field,
  expected_kind }`, `StageUnassigned { kind }`, `StageLayerMismatch { kind,
  stage, layer }` (proposal lines 2384–2387, 2446–2448; T06 lines 9, 44;
  T02/ADR stage validation).
- All three are **absent from `/5`** (`manifest.rs` has no such variants and no
  stage concept). OB-48 records `StoreOwnerViolation`; the same absence applies
  to `StageUnassigned`/`StageLayerMismatch`.
- The proposal says these are "proposed to be hashed in `M1AppendSchema` at
  `/6`" (line 2387), but the `M1AppendSchema` item list (a)–(h) does not
  contain an error-code inventory (finding ERR-06).

### 3.3 `ConfigError`: 2 implemented + N proposed

- Implemented: `TargetUnverified`, `DuplicateOption` (`target.rs:1096–1107`).
- Proposed: `MaxTicksOverflow` with stable code `config.max_ticks_overflow`
  (proposal lines 3287–3296, 3662–3680; `validate()` rejects
  `max_ticks.checked_add(1).is_none()`), plus "variants for invalid
  quota/queue/fairness values (stable diagnostic codes)" with **no names and no
  codes** (ADR §3 line 486; proposal line 3669).
- The validation inventory (`try_new`, per-limit predicates, codes) is open:
  OB-16, OB-33; T02 H9.1 (line 72) says "Exact names/defaults/codes … open;
  not invented here".
- The string `config.max_ticks_overflow` is the **only concrete stable code in
  any document**. The code model is `DiagnosticCode { group: DiagGroup, code:
  u16 }`; the proposal assigns no numeric value, and it is not stated whether
  the dotted token is the diagnostic code, a rule id, or both (finding
  ERR-08).

### 3.4 Dispatcher/scheduling failures

- Operative candidate set after T02 rev 31/33 and CDR rev 51/54:
  **`SelectionBatchOverflow`, `DuplicateSelection`** — pre-dispatch, before any
  store mutation, "a structured tick diagnostic, not a `CommitError`"
  (T02 line 61, T02 line 102; CDR §A9 lines 978–988; ADR lines 674–688).
- Removed from the candidate: `InflightQuotaExceeded` (with
  `max_inflight_total`, rev 33/35) and `DispatchBudgetExceeded` (with
  `max_dispatches_per_tick`, rev 51/54).
- **No carrier type/family is defined** for these failures, and they are not
  listed in any proposed stable-code inventory; ADR §6 says "no new error
  families unless the integrator accepts them" (ADR lines 711–713) without
  naming the carrier to use (finding ERR-03).
- Stale `DispatchBudgetExceeded` occurrences not qualified by the rev-51/54
  removal:
  - proposal §6.4 block, line 2456 (the adjacent note at 2451–2455 covers only
    `InflightQuotaExceeded`);
  - proposal §13 test description, line 4140;
  - CDR §A9 "Candidate errors" list, line 1001 (the rev-54 note is in the
    earlier bullet at 984–988, not in this list);
  - CDR §I3, lines 1911–1915 (only a rev-37 `InflightQuotaExceeded` note).
  The proposal §7 pseudocode (lines 2693–2698), T02 (rev 33), and ADR
  (Revision 17) already carry the removal annotation.

### 3.5 Chip diagnostics

Named in the documents, none implemented and none coded:

| Name | Where classified | Notes |
|---|---|---|
| `ConstOverflow`, `ConstUnsupported` | proposal §6.4 lines 2381–2382; T08 line 43 | `ConstOverflow` explicitly "chip diagnostic via `Fail`/`DiagnosticDraft`, not a `CommitError`" |
| `ParseCursorDidNotAdvance`, `ParseDepthExceeded` | T05 line 143; proposal §6.4 | numeric codes/hash remain T01 `/6` details (T05 line 143) |
| `UnsupportedIrOp`, `UnsupportedNode` | proposal §6.4; T09 | `UnsupportedNode` also a T09 rule-id item |
| `RedeclarationConflict` | T06 lines 9, 42; CDR §I3 line 1907 | removed from the `CommitError` block (rev 22, T06 finding 3) |
| `NoSuchSymbol` | T06 line 42; CDR §I3 line 1917 | a lookup **miss result**, not an error family |
| `EffectMaskUnsupported` | proposal §6.4 line 2423 vs T07/CDR rev 46 vs T13 | carrier disputed — see §6 |
| VF06 `TypedAstInvariant` failure | T13 lines 24, 27, 39 | whether non-M1 required conversions are VF06 failures or unsupported diagnostics is OB-23 |

No per-class code, `DiagnosticId` rule-id inventory, or `DiagGroup` assignment
is defined for any of these; T07 line 106 records the concrete
unsupported/diagnostic codes as open, and T13 line 39 records the H9 numeric
inventories as pending. The only implemented generic diagnostic is
`DiagnosticDraft::unsupported` (`(Unsupported, 1)`).

## 4. Codes: assigned vs open

| Carrier | Assigned today (code) | Assigned in docs | Status |
|---|---|---|---|
| `CommitError` (20 implemented) | `(Protocol, 1)` shared | none | per-variant codes open; ambiguity ERR-07 |
| `CommitError` (46 proposed) | n/a | none; "stable codes at `/6`" | all open |
| `ManifestError` (13 implemented) | `(Manifest, 1)` shared | none | per-variant codes open |
| `ManifestError` (+3 proposed) | n/a | none; proposed hashed in `M1AppendSchema` (line 2387) | names open to freeze; inventory item missing (ERR-06) |
| `ConfigError` (2 implemented) | `(Target, 1)`, `(Config, 1)` | none | per-variant codes open |
| `ConfigError::MaxTicksOverflow` | n/a | `config.max_ticks_overflow` (string) | name assigned; numeric encoding open (ERR-08) |
| `ConfigError` quota/queue/fairness | n/a | "stable diagnostic codes" | names and codes open (OB-16/OB-33) |
| Dispatcher failures (2 operative) | n/a | none | carrier, names, codes open (ERR-03) |
| Chip diagnostics (named) | generic `(Unsupported, 1)` only | none | all open (T07 line 106, T05 line 143) |
| `InternalMissingArena` | n/a | `DiagGroup::Internal` selected (§15.2 item 8) | group selected; numeric code open |
| `DiagGroup` names | hashed in `/5` (`contract.rs:195`) | — | 8 names frozen; adding a family changes the hash |

**Hash consequence:** the `/5` contract hash (`6187…63bb5`) covers the 8
`DiagGroup` names but not any per-variant code. Assigning stable per-variant
codes at `/6` therefore requires an explicit hash-scope decision; the
`M1AppendSchema` list (a)–(h) (proposal lines 3802–3823) does not include an
error-code inventory, although §6.4 (lines 2469–2472) and §15.2 item 8 imply
one (finding ERR-06).

## 5. `DiagnosticDraft` vs `DiagnosticProposal`

| Location | Name used |
|---|---|
| `compiler/src/diagnostic.rs`, `commit.rs`, `snapshot.rs`; `Proposal::Fail` | `DiagnosticDraft` |
| M1 Part A proposal §6.3/§6.4/§7 | `DiagnosticDraft` |
| T01 §5 (line 106) | `DiagnosticProposal` ("`Fail/DiagnosticProposal`") |
| T02 CT11 row (line 110) | `DiagnosticProposal` ("DiagnosticProposals → DiagnosticIds") |
| T04 manifest bullet (line 155) | open item: "`DiagnosticDraft` vs `DiagnosticProposal`" |
| OB-32 (proposal line 6433) | open co-freeze item |

Exactly one name must be frozen at `/6`; the implementation and the proposal
consistently use `DiagnosticDraft`, and T01/T02 are the conflicting texts
(finding ERR-04).

## 6. `EffectMaskUnsupported` carrier

Three incompatible classifications are recorded (OB-27, proposal line 6428):

1. **`CommitError`** — proposal §6.4 block line 2423
   (`EffectMaskUnsupported { task, mask }`), repeated at lines 1429, 2612,
   2810, 4232; `M1_TARGET_ACCEPTANCE.md` line 179 uses the name without a
   carrier.
2. **Chip `DiagnosticDraft`** — CDR rev 46 / T07: M1 allows only
   `EffectMask(0)`; a nonzero mask is a "typed unsupported/diagnostic" whose
   concrete path is OPEN (T07 lines 56, 75, 100, 106); effect bit classes are
   reserved/unassigned.
3. **VF06 verification failure** — T13 lines 24, 27: VF06 must not accept a
   nonzero mask as a pass; OB-23 asks whether non-M1 required conversions are
   VF06 failures or unsupported diagnostics.

The §6.4 block contradicts its own classification prose: the prose says chip
diagnostics are "never a `CommitError`" (line 2380), while the block below
lists `EffectMaskUnsupported` as a `CommitError`. Exactly one carrier must be
chosen before `/6` (finding ERR-02).

## 7. Findings

| # | Severity | Finding | Evidence | Tracked by |
|---|---|---|---|---|
| ERR-01 | Medium | `diagnostic.rs` claims every error type maps to a `DiagnosticDraft`; 7 types have no mapping | `diagnostic.rs:4–7` vs table §2.1 | new (this audit) |
| ERR-02 | High | `EffectMaskUnsupported` carrier is a three-way contradiction, including an internal §6.4 contradiction | proposal §6.4 lines 2380 vs 2423; T07 lines 56/75/100/106; T13 lines 24/27 | OB-27 |
| ERR-03 | High | Dispatcher/scheduling failures have no carrier type, family, or code; the classification is fixed but the vehicle is not | T02 line 61; CDR §A9 lines 978–988; ADR lines 711–713 | new; adjacent to OB-1/OB-17 |
| ERR-04 | Medium | `DiagnosticDraft` vs `DiagnosticProposal` naming conflict | T01 line 106; T02 line 110; T04 line 155 | OB-32 |
| ERR-05 | Medium | `DispatchBudgetExceeded` remains unqualified in the proposal §6.4 block and §13, and in CDR §A9/§I3 lists, after rev-51/54 removal | proposal lines 2456, 4140; CDR lines 1001, 1911–1915 | OB-17/OB-33 (partially) |
| ERR-06 | Medium | Error-code inventory is absent from the hashed `M1AppendSchema` (a)–(h) list, though §6.4/§15.2 item 8 say codes are stable/hashed at `/6` | proposal lines 2387, 2469–2472, 3802–3823, 4319–4322 | new; adjacent to OB-14/OB-42/OB-43 |
| ERR-07 | Medium | The 20 implemented `CommitError` variants are not re-listed in §6.4; unclear whether they receive stable per-variant codes at `/6` | proposal lines 2391–2442, 4319–4322; `commit.rs:91–213` | new |
| ERR-08 | Medium | `config.max_ticks_overflow` is a dotted string "stable code" while the code model is `DiagnosticCode { group, u16 }`; no numeric value and no rule-id/diagnostic-code relationship is defined | proposal lines 3289, 4319–4322; `diagnostic.rs:72–86` | OB-16/OB-33 (partially) |
| ERR-09 | Low | Coarse shared family codes cannot distinguish variants despite the stated tooling rationale; if `/6` adopts per-variant codes, the mapping code changes; if not, the rationale should be corrected | `commit.rs:299–305`; `manifest.rs:449–454`; `task.rs:254–256`; `diagnostic.rs:34–35` | new |
| ERR-10 | Low | `DiagGroup::Arena`/`Internal` are hashed but unused; `InternalMissingArena` is selected to `Internal` yet unimplemented | `diagnostic.rs:40–53`; `contract.rs:118–128`; proposal line 4320 | new |
| ERR-11 | Low | Duplicate dangling-parent carriers (`CommitError::DanglingParent` vs `LimitError::DanglingParent` wrapped by `CommitError::Limit`); the canonical one is not stated | `commit.rs:128–131,427`; `limits.rs:129–132`; `bus.rs:447–464` | new |
| ERR-12 | Low | No direct negative-test match found for `CommitError::UnknownResult`, `CommitError::Capacity`, and `RouteError`; `UndeclaredStoreField` is documented as unreachable defense-in-depth | `compiler/tests/c03_task.rs:332`; `compiler/tests/c06_routing.rs`; `compiler/tests/c07_limits.rs` | new |

## 8. Verification record

- `git diff --check` on the tracked working tree: **clean** (exit 0) before and
  after adding this record.
- This record itself was checked for whitespace errors with
  `git diff --no-index --check /dev/null <file>`: **clean**.
- No `cargo fmt`/`clippy`/`test` run; no code, tests, contract hash, or task
  package changed by this audit.
- Counts in §1–§3 were re-derived mechanically from the working tree at audit
  time; document line references may drift with later edits.

## Appendix A — the 46 proposed `CommitError` names (proposal §6.4, lines 2392–2445)

```text
AppendNotDeclared              AppendFieldNotDeclared        AppendNotPermittedForKind
DuplicateAppendBatch           EmptyAppendBatch              DuplicateAppendMechanism
DraftRefOutOfRange             DraftFamilyMismatch           CommittedRefMissing
CommittedFamilyMismatch        SourceNotDeclared             MultipleSourceRefs
SpanSourceMismatch             ResultDraftRefWithoutBatch    DuplicateTaskTransition
TaskNotTransitioned            AwaitChildrenRefInvalid       ContinuationRefInvalid
NonAdvancingProgress           ProgressLimit                 DraftCapacity(ArenaError)
InternCapacity(InternError)    InternalMissingArena          ArtifactMapInvalid
ArtifactSourceMissing          ArtifactSourceMismatch        SymbolConflict
EffectMaskUnsupported          ScopeLifecycleViolation       DuplicateSemRecord
DuplicateConversionRole        NodeCycle                     NodeOrdinalNotUnique
ContinuationCycle              ValueProducerMissing          ValueProducerDuplicate
BlockMissing                   BlockTerminated               TerminatorMissing
OpArityMismatch                OpImmediateMismatch           OpImmediateTypeMismatch
OpResultMismatch               TerminatorNotLast             CrossTaskWriteConflict
BackpressureCapacity
```

Classified in the same block but **not** `CommitError`s:
`StageUnassigned`, `StageLayerMismatch` (ManifestError);
`SelectionBatchOverflow`, `DuplicateSelection`, `DispatchBudgetExceeded`
(dispatcher/scheduling; the last is removed from the candidate).

## Appendix B — primary references

- `compiler/src/commit.rs`, `manifest.rs`, `limits.rs`, `arena.rs`, `task.rs`,
  `target.rs`, `routing.rs`, `intern.rs`, `diagnostic.rs`, `bus.rs`,
  `contract.rs`, `snapshot.rs`.
- [T01_COMPILER_CONTRACT.md](../tasks/T01_COMPILER_CONTRACT.md) §4–§7.
- [M1_PART_A_CONTRACT_PROPOSAL.md](../tasks/M1_PART_A_CONTRACT_PROPOSAL.md)
  §6.3/§6.4/§7/§10.2/§12/§13/§15.2/§24.
- [CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md](../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)
  §A9/§I3/§9C.
- [ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md)
  §3/§6/§7.
- [T02_CONTROL_CHIPS.md](../tasks/T02_CONTROL_CHIPS.md),
  [T04_LEX_CHIPS.md](../tasks/T04_LEX_CHIPS.md),
  [T05_PARSE_CHIPS.md](../tasks/T05_PARSE_CHIPS.md),
  [T06_SYMBOL_TYPE_CHIPS.md](../tasks/T06_SYMBOL_TYPE_CHIPS.md),
  [T07_SEMANTIC_CHIPS.md](../tasks/T07_SEMANTIC_CHIPS.md),
  [T08_CONSTANT_LAYOUT_INIT_CHIPS.md](../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md),
  [T13_VERIFICATION_CHIPS.md](../tasks/T13_VERIFICATION_CHIPS.md).
- [COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md) §5.
- [2026-10-05_DOCUMENTATION_REVIEW.md](2026-10-05_DOCUMENTATION_REVIEW.md)
  (separate review; DOC-01/DOC-11 touch adjacent enforcement gaps).
