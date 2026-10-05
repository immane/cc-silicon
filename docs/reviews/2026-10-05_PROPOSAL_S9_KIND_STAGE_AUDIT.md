# Proposal §9 Task-Kind → Stage Mapping Audit — 2026-10-05

## Status and scope

- Status: review record only. This is not an ADR, contract approval, interface
  freeze, task-kind registration, or implementation authorization.
- No source, task-package, contract, or proposal file was modified. This
  document is the only artifact.
- Date: 2026-10-05.
- Baseline: working tree at audit time. `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md`
  is **untracked** and was **concurrently edited during this audit** (observed
  mtime `2026-10-05 08:05`; line numbers shifted between reads). Line numbers
  below are **audit-time** and may drift; section headings and quoted text are
  the stable anchors.
- Scope: proposal **§9 "Proposed M1 task-kind code allocation"** (audit-time
  lines 3139–3164), plus the stage model it must satisfy: §6.2.1 `STAGE_NAMES`/
  `StageAssignment`, §12.9/§12.12(d)/§12.18 validation and hash requirements.
  Cross-checked against `compiler/src/task.rs`, `compiler/src/routing.rs`,
  [ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) §2.2/§2.3,
  T02 H9.4, T13 rev 5/6, and M1 vertical acceptance §7.
- Method: direct document and source reading. No runtime tests. The proposal is
  DRAFT, `/6` is unfrozen, and no compiler code is implemented; the findings are
  document/schema-level, not runtime defects.

## Verification evidence

| Check | Result |
|---|---|
| `git diff --check` | exit 0, no output (tracked changes only; the proposal is untracked and therefore **not** covered by this command) |
| `git diff --no-index --check /dev/null docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md` | no whitespace warnings (exit 1 is the expected "differences exist" for a whole-file addition, not a whitespace error) |
| `compiler/src/task.rs:576–587` | frozen `HostRequestKind { ReadSource, WriteArtifact, InvokeToolchain, Cancel }` confirmed |
| `compiler/src/task.rs:26–104` | `TaskGroup` has `CONTROL=0`, `HOST=1`, `VERIFICATION=12`; group names differ from stage names (`constant_layout_init` vs `const`, `ir_lower` vs `ir`) |
| `compiler/src/task.rs:123–124,159–167` | local codes `0..=15` reserved; `is_foundation()` covers only `control.noop/unsupported/start_job/import_source` (local 0–3) |
| `compiler/src/routing.rs:94–97` | `/5` `RoutingTable::register` rejects foundation/reserved-local kinds; no stage concept exists in `/5` |
| proposal §6.2.1 (audit-time lines 2194–2201, 2261–2263) | `STAGE_NAMES` is a **closed 9-name** list `{source_pp, lex, parse, symbol_type, sem, const, ir, machine, artifact}`; `StageAssignment` is declared a "Versioned, **total** mapping (kind -> stage ordinal)" |
| proposal §12.9 (audit-time line ≈3701) and §12.18 | `RoutingTable::register` must enforce **total `kind → stage` coverage** (`ManifestError::StageUnassigned`/`StageLayerMismatch`) |
| proposal §12.12(d) (audit-time line 3825) | `M1AppendSchema` must include "the §9 task-kind codes and their stage assignments" |
| T02 `H9.4` (line 75) | "Task kind → exactly one `StageId`; registration rejects unassigned (`StageUnassigned`) … every registered kind covered" |
| T13 rev 5/6 (lines 9, 12, 21, 75, 87) | VF06 and batch-fixture "exact registered task kind/stage/phase/interface" remain **T01/T13 co-freeze/open**; VF07–VF11 are out of M1 |

## Findings

### P9-01 — §9 fixes no kind→stage assignment; Control/Host/Verification have no stage in the closed candidate (high)

- Location: proposal §9 (audit-time lines 3139–3164); §6.2.1 `STAGE_NAMES`/
  `StageAssignment` (lines 2194–2201, 2261–2263); §12.9; §12.12(d); §12.18.
- Problem: §9 allocates kinds by **group only**; it has no stage column and no
  kind→stage text anywhere. The stage model requires the assignment to be
  **total over registered kinds** and hashed in `M1AppendSchema` item (d). The
  closed 9-stage candidate contains **no control, host, or verification stage**:
  - **Control (0)**, 10 kinds at local 16–31 — no stage. `artifact-finalize`
    (CT14) plausibly belongs to the `artifact` stage but is not assigned;
    `select/guard/validate/commit/resume/phase-advance/progress-budget/`
    `diagnostic-commit/recovery-select` are driver/commit-path realizations
    (§10.2/§10.4) with no stage.
  - **Host (1)**, 3 kinds at local 16–23 — no stage (and see P9-02).
  - **Verification (12)**, 6 kinds at local 16–31 — no stage (and see P9-04).
  - Conversely, `machine` and `artifact` stages have no §9 group owner.
  - Groups 2–8 correspond to `source_pp…ir` only **by convention**: the group
    names in the frozen `TaskGroup` (`preprocess`, `symbol_type`, `semantic`,
    `constant_layout_init`, `ir_lower`) differ from the stage names, and no
    table states the correspondence.
- Impact: under the proposed validation, any registered kind from these rows is
  rejected by `StageUnassigned`; `M1AppendSchema` item (d) cannot be completed as
  written, so the `/6` stage-assignment hash is blocked. This is a `/6`
  co-freeze blocker, not an implementation detail.
- Existing records: OB-35 (Host collision), OB-39 (CT01 "no §9 M1 kind/stage"),
  OB-46/OB-47 (§9 chip omissions), CDR Request A2 ("exact stage set/names"
  open), T02 H9.4, ADR-0002 §2.2/§2.3. The **Control/Host/Verification
  stage-mapping gap itself is not recorded** in the §24.11 OB table.

### P9-02 — Host group duplicates the frozen `HostRequestKind` (confirmed; already OB-35)

- Location: §9 Host row (audit-time line 3149); `compiler/src/task.rs:576–587`;
  §5 `AwaitHost` path; T02 CT08 row.
- Evidence: the frozen `/5` envelope represents host interaction as
  `HostRequestKind { ReadSource, WriteArtifact, InvokeToolchain, Cancel }`
  carried by `HostRequestDraft`/`HostRequestRecord` and created via
  `Proposal::AwaitHost`. §9 proposes `read-source`, `write-artifact`,
  `invoke-toolchain` as **group-1 `TaskKind`s**: the same three names (hyphen
  vs snake case), `Cancel` absent. Chips must not perform I/O (AGENTS.md §3.7;
  T01 §4 boundary), and no group-1 chip owner exists (the request chip CT08 is
  under Control). Because the row's local codes 16–23 are in the group-owner
  routable range, registering them would additionally require stages (P9-01).
- Status: already recorded as **OB-35**; this audit confirms it against the
  frozen code. No replacement kind was invented and no fix was applied.

### P9-03 — Control row mixes non-routed control-plane functions with routable-kind codes (medium)

- Location: §9 Control row (audit-time line 3148); §10.2/§10.4; T02 chip table
  CT01–CT14; `compiler/src/routing.rs:94–97`.
- Evidence: §9 lists 10 Control entries at local 16–31 (the group-owner routable
  range), while §10.2/§10.4 state CT03 is invoked only through its adapter
  outside the routed-worker path and CT05/CT06 are realized by the commit path
  (validation-only / propose-only). `/5` `RoutingTable::register` rejects only
  foundation/reserved-local kinds (local ≤ 15); nothing exempts control-plane
  functions from the proposed total stage mapping. The row is also not a
  complete CT01–CT14 inventory: CT01/CT02 (partially covered by the frozen
  `control.start_job`/`control.import_source` at local 0–3), **CT08**, and
  **CT13** have no §9 entry.
- Impact: unstated which §9 Control entries are registered routable `TaskKind`s
  (needing exactly one `StageId`) versus driver-realized functions (not task
  kinds). That ambiguity propagates into the `M1AppendSchema` (d) hash
  inventory and the `StageUnassigned` validation surface.

### P9-04 — Verification row has no stage; its inventory conflicts with the T13 M1 set (medium)

- Location: §9 Verification row (audit-time line 3157); T13 rev 5/6 (lines 9,
  12, 21, 75, 87); M1 vertical acceptance §7.
- Evidence: no verification stage exists in `STAGE_NAMES`, and T13 repeatedly
  states the "exact registered task kind/stage/phase/interface" for VF06 and the
  batch fixtures remains T01/T13 co-freeze/open — so no stage assignment is
  derivable. VF06's fixed M1 placement ("after committed T07 `SemRecord`s,
  before T09 lowering") cannot be expressed as one of the nine stages. The row
  lists VF05–VF08/VF12–VF13, but the T13 M1 set is
  **VF01–VF06/VF12/VF13/VF14**: VF01–VF04 and VF14 are omitted (**OB-47**), and
  VF07 `CfgInvariantChip`/VF08 `IrInvariantChip` are listed although T13 line 87
  says VF07–VF11 are out of M1 (no CFG/SSA/machine/ABI stage). M1 vertical §7
  says the verification chips cover "all M1 stages", which is not a single-stage
  assignment either.

## Cross-reference summary

| Issue | Existing record | Gap not covered by that record |
|---|---|---|
| Control/Host/Verification kinds have no stage; no such stage exists | none directly; CDR Request A2 keeps the stage set open; T02 H9.4 requires coverage | the §9-specific mapping gap and its effect on `M1AppendSchema` (d) |
| Host row duplicates `HostRequestKind`, no chip owner | OB-35 | confirmed against frozen code; `Cancel` absent; stage consequence |
| CT01 has no §9 kind/stage | OB-39 | CT08/CT13 also absent; control-plane vs task-kind ambiguity |
| §9 verification row omits VF01–VF04/VF14 | OB-47 | row also lists out-of-M1 VF07/VF08; no stage assignment |
| §9 chip-row omissions (PP26, LX16) | OB-46 | (out of this audit's scope; noted for completeness) |

## Minimal fix suggestion (not applied; no new kinds)

This audit chose the report path: the proposal is untracked and was concurrently
edited during the audit, and the OB table numbering/revision records are
integration-owned, so applying an edit here risks collision with parallel
audits. If the integration pass wants the minimal doc fix, two text-local edits
suffice and introduce **no new task kind and no new stage**:

1. Under §9, after the table, add:

   > **Stage assignment (T01 co-freeze; no new kinds).** The total
   > `(TaskKind → StageId)` assignment required by §6.2.1/§12.9/§12.18 and
   > listed in `M1AppendSchema` item (d) is **not fixed by this table**. Groups
   > 2–8 correspond by convention to the `source_pp`…`ir` stages; the
   > **Control**, **Host**, and **Verification** rows have **no stage** in the
   > closed `STAGE_NAMES` candidate (which also has no §9 group for
   > `machine`/`artifact`). Which of these entries are registered/routed kinds —
   > and therefore must carry exactly one `StageId` or be rejected by
   > `StageUnassigned` — versus driver/control-plane-realized functions
   > (CT03/CT05/CT06 per §10.2/§10.4) is a T01 `[INT]` co-freeze item. The Host
   > row additionally duplicates the frozen `HostRequestKind`
   > (`read-source`/`write-artifact`/`invoke-toolchain`) with no chip owner
   > (OB-35); `Cancel` is absent. No replacement or additional kind is proposed
   > here.

2. Add one OB row to §24.11, e.g.:

   > | OB-54 | §9 kind→stage mapping absent for Control/Host/Verification: no
   > stage exists in the closed 9-name `STAGE_NAMES` candidate for those groups
   > (nor for the frozen Control foundation kinds if routed), while §6.2.1/
   > §12.9/§12.18 require total kind→stage coverage (`StageUnassigned`) and
   > `M1AppendSchema` (d) must hash "the §9 task-kind codes and their stage
   > assignments"; `machine`/`artifact` have no §9 group; T02 H9.4 requires
   > every registered kind covered; T13 rev 5/6 leaves the VF registration/
   > stage open | T01 `[INT]` (+T02/T13) | proposal §9/§6.2.1/§12.12(d)/§12.18;
   > ADR-0002 §2.2/§2.3; T02 H9.4; T13 |

## Verification and limitations

- `git diff --check`: exit 0 (tracked changes only). The untracked proposal and
  this untracked report are not covered by that command; the no-index check
  above found no whitespace warnings in the proposal.
- No file other than this report was created or modified by this audit.
- No `cargo` build/test/clippy was run (document/source-read audit only); no
  runtime behavior, compiler capability, or `/6` status is claimed.
- The proposal was edited concurrently during the audit; re-derive line numbers
  from section anchors before applying any suggested text.
- This report does not close, reclassify, or sign off any OB row; P9-01 is a new
  observation offered for the integration pass, and P9-02 is a confirmation of
  the already-recorded OB-35.
