# M1 Store Sole-Writer Audit: Field-Level Table (M1 Proposal §8 + T01 §5)

Status: **read-only documentation audit; doc-only.** This file records a
field-level sole-writer review of the M1 store declarations. It is **not** an
ADR, not a contract approval, not a `/6` freeze, and not implementation
authorization. `/5` (`t01-c01-c06/5`) remains current; `/6` is unfrozen; the M1
proposal, CDR, and task packages remain DRAFT/PENDING. No code, test, schema, or
interface is changed by this file. No compiler capability, pass rate, or probe
result is claimed.

Date: 2026-10-05.
Basis: the working tree (including the uncommitted/untracked M1 documents), not
Git HEAD alone. Section references are stable; any line numbers cited are as of
this audit and may drift.

Sources:

- [T01_COMPILER_CONTRACT.md](../tasks/T01_COMPILER_CONTRACT.md) §2 (register
  partitions), §4 (task/completion protocol), §5 (per-chip table inheritance
  rules).
- [M1_PART_A_CONTRACT_PROPOSAL.md](../tasks/M1_PART_A_CONTRACT_PROPOSAL.md) §8
  (proposed append fields, owners, write sets), §14 (ownership boundaries),
  §24.11 (OB open-blocker ledger).
- [CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md](../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)
  §3.2 (IDs/record references/stores and fields), §B (single-owner model).
- [T02_CONTROL_CHIPS.md](../tasks/T02_CONTROL_CHIPS.md) (envelope, CT chips),
  [T03_PREPROCESS_CHIPS.md](../tasks/T03_PREPROCESS_CHIPS.md) §7,
  [T04_LEX_CHIPS.md](../tasks/T04_LEX_CHIPS.md),
  [T05_PARSE_CHIPS.md](../tasks/T05_PARSE_CHIPS.md),
  [T06_SYMBOL_TYPE_CHIPS.md](../tasks/T06_SYMBOL_TYPE_CHIPS.md),
  [T07_SEMANTIC_CHIPS.md](../tasks/T07_SEMANTIC_CHIPS.md),
  [T08_CONSTANT_LAYOUT_INIT_CHIPS.md](../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md),
  [T09_IR_LOWER_CHIPS.md](../tasks/T09_IR_LOWER_CHIPS.md).
- [COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md)
  (manifest shape, validation rules, commit-time enforcement), with the
  foundation field set verified in `compiler/src/manifest.rs`
  (`StoreSchema::foundation`).

## 1. Scope and vocabulary

The question: for every M1 store field, is there **one** declared writer, and
does each writing chip have the **field-level read/write manifest** that T01 §5
requires?

- **Sole writer.** M1 proposal §8 declares one owning group per append field.
  A group owner is necessary but not sufficient: T01 §5 says the package
  envelope is only the upper bound of permissions, each chip must refine its
  input IDs into a field list before implementation, and "a permission to read
  or write the whole bus is not an adequate contract". `M1-WS-04` (M1 vertical
  acceptance §6.4) audits exactly this across the M1 chip set.
- **Per-chip manifest.** The `ChipManifest { reads, writes, ... }` of
  `COMPILER_SFL_MANIFEST.md` §2: `store.field` `FieldPath`s validated against
  the store schema. A package-level envelope or a group-level owner statement is
  **not** a per-chip manifest.
- **Structural block (`UNREGISTRABLE`).** In the frozen `/5` code, the
  foundation schema declares only `config.*`, `control.*`,
  `sources.{bytes,name,span_root,expansion}`, `tasks.{active.*,queue.ready,
  results,completed}`, `diagnostics.entries`, `artifacts.fragments`, and
  `wires.*`. The group-owned stores (`pp`, `lex`, `parse`, `symbols`, `types`,
  `sem`, `constants`, `layout`, `init`, `ir`, `opt`, `machine`, `ext`) start
  with **no** declared fields, `StoreId::Names` does not exist yet, and
  `sources.spans` / `sources.expansions` / `tasks.continuations` are not
  declared. Until the two-tier `StoreSchema::foundation + M1AppendSchema` seed
  (accepted in principle, CDR rev 50) is implemented, any manifest reading or
  writing those paths is rejected as an undeclared field. Per-chip manifests for
  most M1 append families are therefore **not registrable today**: the
  documentation gap and the code gap coincide.
- **Flag tokens used below:** `NO-OWNER` (no single writer named),
  `NO-MANIFEST` (no per-chip field-level manifest),
  `OWNER-PENDING` (owner named but owner/integrator sign-off open),
  `UNREGISTRABLE` (field not in the `/5` store schema),
  `HOST-BOUNDARY` (not a chip; needs explicit host write attribution),
  `OUT-OF-M1` (no M1 field/writer because the feature is deferred).

## 2. Table A: declared M1 append fields (M1 proposal §8)

The 19 rows of M1 §8. "Manifest state" reports what exists in the task packages
today; every candidate is unfrozen and owner/T01 co-freeze remains pending
(OB-12, OB-32).

| Store.field | Record / Draft | Declared sole writer (M1 §8) | Per-chip manifest state | Flags |
|---|---|---|---|---|
| `sources.spans` | `SpanRecord` / `SpanDraft` | **T03 only** (T04 reuses the committed PP span; T05 uses token ranges; no shared-writer carveout) | PP01 candidate only (T03 §7); no per-chip split for PP02–PP28; OB-24 open | `NO-MANIFEST` (partial), `UNREGISTRABLE` |
| `sources.expansions` | `ExpansionRecord` / `ExpansionDraft` | T03 | same as `sources.spans`; M1 produces no expansion | `NO-MANIFEST`, `UNREGISTRABLE` |
| `names.entries` | interned `NameId` (no per-record body, no `CommittedPatch`) | T04 (job-global; T03/T06 read-only; mandatory allowlist) | T04 candidate reads/writes; per-chip split and append semantics open (OB-32) | `NO-MANIFEST` (partial), `UNREGISTRABLE` |
| `pp.tokens` | `PpTokenRecord` / `PpTokenDraft` | T03 (owns the span T04 references) | PP01 candidate only; other producers open | `NO-MANIFEST` (partial), `UNREGISTRABLE` |
| `lex.tokens` | `TokenRecord` / `TokenDraft` | T04 (span = committed T03 PP span; no span write) | T04 candidate upper bound; per-chip manifests explicitly still T01 co-freeze (T04 §manifest) | `NO-MANIFEST`, `UNREGISTRABLE` |
| `lex.literals` | `LiteralRecord` / `LiteralDraft` | **T04 sole writer** (consumed by T08 via committed `RecordRef::Literal`) | same T04 candidate; exact fields selected in principle (rev 45) but manifest split open | `NO-MANIFEST`, `UNREGISTRABLE` |
| `parse.nodes` | `NodeRecord` / `NodeDraft` | T05 (first/last `TokenId` range; no span write) | **none**; OB-32 records missing manifests for `PA01–PA38` | `NO-MANIFEST`, `UNREGISTRABLE` |
| `tasks.continuations` | `ContinuationRecord` / `ContinuationDraft` | T05 | **none**; ordered fields accepted (CDR rev 43) but write manifest open | `NO-MANIFEST`, `UNREGISTRABLE` |
| `symbols.scopes` | `ScopeRecord` / `ScopeDraft` | T06 | **none**; lifecycle candidate only; manifests co-freeze | `NO-MANIFEST`, `UNREGISTRABLE` |
| `symbols.scope_events` | `ScopeEventRecord` / `ScopeEventDraft` | T06 | **none**; arena/field append itself open (T06 item 6d/6f) | `NO-MANIFEST`, `UNREGISTRABLE` |
| `symbols.symbols` | `SymbolRecord` / `SymbolDraft` | T06 | **none** | `NO-MANIFEST`, `UNREGISTRABLE` |
| `types.records` | `TypeRecord` / `TypeDraft` | T06, `ChipId`-keyed allowlist (`TY13` canonical `int`, `TY17` function type; no structural dedup) | allowlist **mechanism** selected; rows/seed/hash absent (T06 item 6d; OB-48) | `NO-MANIFEST`, `UNREGISTRABLE` |
| `sem.records` | `SemRecord` / `SemDraft` | T07 | **none**; carrier (`SemId`+family vs `NodeId`-keyed) and store/field manifest open (OB-19/20; T07 §2) | `NO-MANIFEST`, `UNREGISTRABLE`, `OWNER-PENDING` |
| `constants.records` | `ConstRecord` / `ConstDraft` | **T08 sole M1 writer** (T04 writes no constant) | exact request/result carriers open (OB-6..OB-9, OB-22); no per-chip manifest itemized | `NO-MANIFEST`, `UNREGISTRABLE` |
| `ir.functions` | `FunctionRecord` / `FunctionDraft` | T09 | **none**; writer manifest explicitly open (T09 §writer manifest) | `NO-MANIFEST`, `UNREGISTRABLE` |
| `ir.blocks` | `BlockRecord` / `BlockDraft` | T09 | same | `NO-MANIFEST`, `UNREGISTRABLE` |
| `ir.values` | `ValueRecord` / `ValueDraft` | T09 | same | `NO-MANIFEST`, `UNREGISTRABLE` |
| `ir.instructions` | `InstructionRecord` / `InstructionDraft` | T09 (owns IR `Constant` emission) | same | `NO-MANIFEST`, `UNREGISTRABLE` |
| `artifacts.fragments` | `ArtifactRecord` / `ArtifactDraft` (total 8-kind) | **T03** produces the M1 kinds; CT14/host only reads and publishes the Host write request | T03 artifact candidate exists; field **is** in the `/5` foundation, but the M1 record shape (`source`/`raw_offsets`) is unfrozen; CT14→CT08→Host chain unstated (OB-35/36) | `NO-MANIFEST` (partial), `HOST-BOUNDARY` |

Note: M1 §8 fixes the **append areas only**; per-chip **read** sets (e.g.
`lex.tokens.kind`) must still each be declared in `StoreSchema` and are not
itemized anywhere yet.

## 3. Table B: M1-relevant fields outside M1 §8

These are written or read on the M1 path but have no row in M1 §8.

| Field | Declared writer/owner today | Single owner? | Per-chip manifest | Flags |
|---|---|---|---|---|
| `config.{target,dialect,options,limits}` | read-only after init; writes rejected at registration and commit (C04) | n/a (no writer) | n/a | OK (read-only) |
| `control.{phase,tick,job_state,budget,selected_task,enqueue_ordinal}` | control chips + dispatcher/commit path (T02); `PipelineMetrics`/report fields proposed | partly; the `Ready→Running` vs atomic-commit boundary and `in_flight` clear owner are open T01 decisions (OB-1) | T02 package envelope only; per-chip manifests pending (T02; DOC-04) | `NO-MANIFEST`, `OWNER-PENDING` |
| `sources.{bytes,name,span_root,expansion}` | CT02 source import via the commit path (T02 envelope); foundation source-root/span-map fields stay T01/integration (CDR §3.2) | CT02 named for import; `sources.expansion` vs `expansions` boundary recorded but no chip manifest | none; source-import satisfaction flag undefined (OB-40) | `NO-MANIFEST`, `HOST-BOUNDARY` |
| `tasks.active.*` | read-only default read set (T01 §5); dispatcher applies selection state | dispatcher/backend | not a chip write | OK for reads; write boundary open (OB-1) |
| `tasks.queue.ready` | derived view; canonical `stage_queues[stage]` written by commit path | owner/clear order open (H9; OB-1/OB-2) | none | `NO-OWNER`, `NO-MANIFEST` |
| `tasks.results` | commit path/backend (T01 §4; T02: queues/results written only by the commit path) | **no chip owner**; `ResultValue` additions (`DraftRecords`) absent from the `M1AppendSchema` seed (OB-42); exactly-once `consumed` open (OB-50) | none (no chip can own it) | `NO-OWNER`, `NO-MANIFEST` |
| `tasks.completed` | commit path/backend | same as `tasks.results` | none | `NO-OWNER`, `NO-MANIFEST` |
| `diagnostics.entries` | commit path applies CT11 `DiagnosticProposal`; `Fail`/`AwaitHost` do **not** consult a producer manifest (manifest §5) | CT11 named, but `DiagnosticDraft` vs `DiagnosticProposal` naming open (OB-32); capacity/sentinel mechanism selected (CDR rev 42) but codes/encoding open | none | `NO-MANIFEST`, `OWNER-PENDING` |
| `wires.proposals.self` | each worker adapter (T02 single-writer statement) | dispatcher/adapters named | per-chip manifests still required | `NO-MANIFEST` |
| `wires.selected_task` | dispatcher sole writer; CT03 adapter writes the canonical `wires.selection`/`wires.selected` | yes (T02) | not a language chip write | OK |
| `layout.*` | **no append field declared**; T08 package envelope says "write constants/layout/init stores" (upper bound only) | **no field owner** | **none**; no layout chip in M1 (`M1-CL-04` gap) | `NO-OWNER`, `NO-MANIFEST`, `OUT-OF-M1` |
| `init.*` | same as `layout.*` | **no field owner** | **none** | `NO-OWNER`, `NO-MANIFEST`, `OUT-OF-M1` |
| `opt.*`, `machine.*`, `ext.*` | no M1 fields or chips (Part B / deferred) | none | none | `OUT-OF-M1` |

## 4. Flag register

### 4.1 `layout` / `init` — no single owner and no per-chip manifest

- T01 §2 lists `layout` and `init` as partitions; T08's package grants
  `chips/constants/`, `chips/layout/`, `chips/initializers/` and "write
  constants/layout/init stores and proposals" — a package-level upper bound
  with no field-level rows anywhere.
- M1 §8 declares no `layout.*` or `init.*` append field, and M1 §5.6/§10 records
  all layout/initializer chips as unexercised gaps, not passes.
- Per T01 §5, the envelope is an upper bound and each chip must refine to a
  field list or pause for the integrator. No layout/init field list exists, so
  if such a chip were implemented from the envelope as written it would hold a
  whole-store permission — the exact failure `M1-WS-04`/T01 §5 prohibits.
- Action needed before any layout/init chip is dispatched: freeze field-level
  rows (e.g. a `layout.records`-style append area and an `init` plan field — not
  invented here) or keep the stores explicitly outside M1.

### 4.2 `results` — no chip owner, no manifest, omitted from the `/6` seed

- `tasks.results` is semantic state (the canonical comparison projection
  includes results), but it is written by the commit path/backend, not by any
  chip (T01 §4: "the commit path/backend writes persistent queues/results";
  T02: "queues/results/arenas are written only by the commit path/integration").
- The `ResultValue` wire additions (`DraftRecords`) are explicitly **not** in
  the `M1AppendSchema` §12.12 list, so the proposed `/6` seed hash would not pin
  them (OB-42).
- Exactly-once result consumption remains open (OB-50; OPEN-02), and the
  stage-edge result delivery has no frozen envelope.
- Consequence: no per-chip manifest can cover results, and `M1-WS-04` does not
  audit the area. An explicit integration-owned write declaration (not a chip
  manifest) is required for the commit path, and the `/6` seed must include the
  result wire shapes.

### 4.3 `diagnostics` — named chip, but no field-level manifest and no commit gate

- `diagnostics.entries` is in the `/5` foundation schema and is applied by the
  commit path from CT11 proposals, but it has no M1 §8 row and no per-chip
  manifest.
- The `DiagnosticDraft` vs `DiagnosticProposal` name conflict is an open
  co-freeze item (OB-32); diagnostic numeric codes are open throughout.
- Commit-time enforcement is asymmetric: only `StorePatch` checks the producer's
  registered manifest; `Complete`, `Fail`, and `AwaitHost` perform no
  producer-registration or accepted-kind lookup (`COMPILER_SFL_MANIFEST.md` §5).
  A task whose owning chip has no registered manifest can therefore still emit
  diagnostics. If diagnostics are to be manifest-gated, that gate must be
  added explicitly; otherwise the exception must be stated for diagnostics.
- The selected failure mechanism (per-task diagnostic attempt with
  `DiagnosticId::NONE` fallback, CDR rev 42) fixes capacity behavior but not the
  field-level writer/encoding of the diagnostic records.

### 4.4 `host` — outside the chip-manifest model; chain unstated

- Host actions are not chips and cannot hold a `ChipManifest`. The M1 path
  nevertheless depends on host-side writes: source import (CT02), artifact
  persistence (CT14 → Host), and the `AwaitHost` wait/resume.
- OB-35: proposed `read-source`/`write-artifact`/`invoke-toolchain` TaskKinds
  duplicate the frozen `HostRequestKind` and have no valid chip owner; the
  CT14→CT08→Host finalization chain is unstated.
- OB-36: CT14 `ArtifactFinalize` is absent from the M1 T02 chip list even though
  the successful-run terminal state depends on it; the Part A evidence commands
  have no M1 Host-task scope.
- OB-40: `HostRequestRecord.satisfied` has no setter/consumer and no
  `SourceImported` carrier. OB-41: no rule names who observes a satisfied host
  request and reinserts the waiting task exactly once.
- Until the host boundary is defined, `sources.*` import and
  `artifacts.fragments` persistence have no single attributable writer:
  CT02/CT08/CT14 must receive field-level manifests, and the Host itself needs
  an explicit non-chip write declaration (or all host-visible mutations must be
  expressed as commit-path applies of chip proposals).

### 4.5 Cross-cutting blockers

- OB-12: per-chip field-level read/write manifests plus sole-chip writers for
  every `/6` append family are open (T01 + T03–T09 owners).
- OB-32: per-chip field-level manifests missing for `LX01–LX18` and
  `PA01–PA38`; `names.entries` append semantics; `DiagnosticDraft` vs
  `DiagnosticProposal` naming.
- `M1-WS-04` (M1 vertical §6.4, gate G10) cannot pass in the current state:
  most M1 chips have no field-level manifest, and several fields have no single
  owner at all (Table B).
- Structural: group-owned store fields are not in the `/5` `StoreSchema`, so the
  manifests cannot even be registered until the `M1AppendSchema` seed is
  implemented (CDR rev 50 two-tier model, accepted in principle, not
  implemented).

## 5. Non-claims

- This audit did not run tests, Cargo checks, or any compiler; it made no code
  or schema change and claims no implementation status.
- Owner rows quoted from M1 §8/CDR are proposals or accepted-in-principle
  directions, not `/6` freezes; "selected" does not mean implemented.
- The flag list is not claimed exhaustive; it covers the fields named in the
  audit request (layout, init, results, diagnostics, host) plus the M1 §8 rows.
- Line-level citations may drift as the DRAFT documents are revised; section
  references remain the authority.
