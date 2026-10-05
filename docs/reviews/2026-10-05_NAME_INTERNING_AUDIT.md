# Name Interning Audit — 2026-10-05

## Status and scope

- Status: read-only audit record. It is not an ADR, contract approval, `/6`
  freeze, owner sign-off, or implementation authorization.
- Date: 2026-10-05.
- Baseline: the current working tree, including uncommitted and untracked
  documentation (the frozen `/5` code is committed). Line numbers are
  working-tree positions at audit time and may drift after later edits.
- Scope: name interning across the frozen `/5` implementation and the current
  M1 documentation set. Checklist items: dedup, deterministic IDs,
  store-vs-table, relocation, `SymbolRecord.name`, keyword/name count
  (1-vs-4), and `TokenRecord.literal` typing.
- Method: cross-read of code and documents; no execution, no code change, no
  edit to any audited file. This report is the only file added.
- Frozen code inspected: `compiler/src/intern.rs`, `ids.rs`, `task.rs`,
  `bus.rs`, `commit.rs`, `contract.rs`, `manifest.rs`, `snapshot.rs`,
  `limits.rs`; tests `c01_arena.rs`, `c07_limits.rs`.
- Documents inspected: [T01](../tasks/T01_COMPILER_CONTRACT.md),
  [M1 proposal](../tasks/M1_PART_A_CONTRACT_PROPOSAL.md),
  [CDR](../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md),
  [T04](../tasks/T04_LEX_CHIPS.md), [T06](../tasks/T06_SYMBOL_TYPE_CHIPS.md),
  [M1 frontend acceptance](../tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md),
  [compiler README](../../compiler/README.md).

Overall: the `/5` interning mechanics (dedup, first-seen deterministic IDs,
snapshot order) are consistent and tested. The new M1 documents are internally
inconsistent on three points (`TokenRecord.literal` typing, keyword/name count,
`names.entries` store-vs-table tense) and correctly leave the relocation
protocol open (`OB-49`). `SymbolRecord.name` is consistent. Details below.

## Verdict summary

| # | Item | Verdict | Severity |
|---|---|---|---|
| NI-01 | `TokenRecord.literal` typing | Inconsistent across three documents | Medium |
| NI-02 | Keyword/name count 1-vs-4 | Acceptance row contradicts T04 rule | Medium |
| NI-03 | "same name record" wording (`M1-LX-02`) | Stale: no `NameRecord` exists | Low |
| NI-04 | Store-vs-table (`names.entries`) | `/5` cannot express it; proposal-only store; T04 tense mixes current/future | Medium |
| NI-05 | Relocation | Design exists, protocol open (`OB-49`/`OB-10`); one unstated cross-batch case | Open blocker |
| NI-06 | Dedup and deterministic IDs | Verified consistent; two wording/clarification items | Low |
| NI-07 | `SymbolRecord.name` | Verified consistent | None |

---

## NI-01 — `TokenRecord.literal` typing is inconsistent (Medium)

The field is spelled in three incompatible ways across four places:

1. Final record (proposal §5):
   `pub literal: Option<LiteralId>` — `M1_PART_A_CONTRACT_PROPOSAL.md:1123–1126`.
2. Draft form (proposal §6.1):
   `literal: Option<RecordLink<Literal>>` — `M1_PART_A_CONTRACT_PROPOSAL.md:1966`.
3. T04 "C token fields (candidate view)" presents the **record** but uses the
   **draft link** type:
   `literal: Option<RecordLink<Literal>>` with the comment "typed draft link,
   resolved pre-apply" — `T04_LEX_CHIPS.md:124–128`.
4. T04 task-revision-1 history says `Option<TokenId> literal` — the wrong
   family entirely (`T04_LEX_CHIPS.md:179`).

The committed-vs-draft split is handled correctly for the sibling fields:
`LiteralRecord.token: Option<TokenId>` (proposal:1149) versus
`LiteralDraft.token: Option<RecordLink<Token>>` (proposal:1967), and for
`TokenRecord.span`/`name` versus `TokenDraft.span`/`name` (proposal:1123–1125,
1966). `TokenRecord.literal` is the only field where the final record view uses
a draft-link type.

Recommendation: state the final record field as `Option<LiteralId>` (or an
equivalent committed `RecordRef::Literal`) and keep `RecordLink<Literal>` only
on `TokenDraft`; correct the revision-1 history text to
`Option<RecordLink<Literal>>`/`Option<LiteralId>` as appropriate. This matters
for the `/6` record schema and the T04/T08 handoff.

## NI-02 — Keyword/name count 1-vs-4 (Medium)

- `M1-LX-02` asserts the M1 fixture has **one** interned name:
  "one interned name; identical spelling yields the same name record"
  (`M1_VERTICAL_SLICE_ACCEPTANCE.md:217`).
- T04 says identifier **and keyword** tokens intern their spelling:
  `name: Option<NameId>, // interned identifier name (identifier/keyword tokens)`
  (`T04_LEX_CHIPS.md:127`) and "Identifier (and keyword) tokens intern their
  spelling ... identical spelling yields the same stable `NameId`"
  (`T04_LEX_CHIPS.md:148`).
- The fixture has the keyword tokens `int`, `void`, `return` plus the
  identifier `main` (`M1_VERTICAL_SLICE_ACCEPTANCE.md:216`, and `M1-LX-03`
  at line 218 names the three keyword spellings).

Under T04's rule the fixture interns **four** distinct spellings
(`int`, `void`, `return`, `main`); under `M1-LX-02` it interns one. No document
exempts keyword spellings from interning, and the LX03 input is `Name/mode`
(`T04_LEX_CHIPS.md:11`), which reads as an already-interned name. Either:

- the acceptance row is corrected to the intended count (e.g. four interned
  spellings, or "one identifier name plus the three keyword spellings"), or
- T04 explicitly exempts keyword spellings from interning and drops the
  "keyword tokens" claim for `TokenRecord.name`.

Until one is chosen, an implementation following T04 fails the `M1-LX-02`
oracle, or an implementation following `M1-LX-02` violates the T04 rule.

## NI-03 — Stale "same name record" wording (Low)

`M1-LX-02` says "identical spelling yields the same name record"
(`M1_VERTICAL_SLICE_ACCEPTANCE.md:217`). There is no name record body:
`NameRecord deleted (T03 R1): interning identity is NameId; bytes live in
InternTable` (`M1_PART_A_CONTRACT_PROPOSAL.md:1112–1113`); T04 itself says
"same stable `NameId`" (`T04_LEX_CHIPS.md:148`). Recommendation: replace "same
name record" with "same `NameId`", and consider "T01 name store" in the same
row (line 217) since `/5` has no name store (see NI-04).

## NI-04 — Store-vs-table: `names.entries` (Medium)

Frozen `/5` facts:

- `StoreId::ALL` has 20 stores and **no** `Names` (`task.rs:650–671`);
  `StoreSchema::foundation()` declares no `names` field (`manifest.rs:175–204`).
- The intern table is a public bus field, not an arena/store:
  `pub intern: InternTable` (`bus.rs:356–357`), with the checked wrapper
  `intern_name` (`bus.rs:486–489`).
- `ManifestError` has no `StoreOwnerViolation` (`manifest.rs:297–383`); a chip
  manifest declaring `names.entries` would fail `UnknownWriteField` today.
- `FrozenSchema::encode` hashes the `StoreId::ALL` labels
  (`contract.rs:196–198`), so appending a store necessarily changes the `/6`
  contract hash; `StoreVersions` is sized by `StoreId::COUNT`.
- `commit_proposals` bumps store versions only for `StorePatch`-touched stores
  (`commit.rs:541–559`); names would need a new bump path.
- `RECORD_KINDS` already includes `"names"` (`task.rs:746–771`), so only the
  store inventory changes, not the record-family list.

The proposal resolves this deliberately: new `StoreId::Names` appended at the
end, field `entries`, mapped to `InternTable` (not a `TypedArena`), no
`CommittedPatch`, version bump only when at least one new name is interned
(`M1_PART_A_CONTRACT_PROPOSAL.md:3037–3046`; CDR rows at
`CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md:793,807`). That is a
proposed `/6` interface (`OB-13`, `OB-32`, `OB-48`), not current behavior.

The defect is tense/terminology, not the direction:

- T04 states as operative that it reads "the committed `names.entries`" and
  writes "`names.entries` (own intern append)" with a "mandatory allowlist"
  (`T04_LEX_CHIPS.md:154–155`), although no `/5` `FieldPath` exists for it and
  T04's own status says the exact manifest/allowlist is a `/6` co-freeze item.
- The vocabulary oscillates between "the `InternTable` is not a store at all"
  (`M1_PART_A_CONTRACT_PROPOSAL.md:858–864`), "Names are an intern store"
  (item 11, lines 933–939), and the §8 table's store row `names | entries`
  (line 3009).

Recommendation: in T04 mark the `names.entries` read/write/allowlist lines as
`/6`-conditional (or refer to the proposed `StoreId::Names`), and define once
that `StoreId::Names` is a store identifier for authorization/versioning that
maps to the non-arena `InternTable`. Note that adding it changes the `/6`
frozen hash (store labels are hashed) even though indices 0–19 do not shift.

## NI-05 — Relocation (Open blocker; no new defect, one unstated case)

- T01 §4 requires: "When a proposal needs to reference a new record, C01/C03
  must freeze a deterministic reserved-ID or local-reference relocation
  protocol" (`T01_COMPILER_CONTRACT.md:82`).
- T01 C01/C03 explicitly record it as absent and cite `OB-49`
  (`T01_COMPILER_CONTRACT.md:153,155`); `OB-49` confirms "only predicted IDs in
  `commit.rs`; no named reservation/apply-map protocol or hashed rule"
  (`M1_PART_A_CONTRACT_PROPOSAL.md:6514`). In `/5`, `commit_proposals` predicts
  only earlier `Enqueue` parent **task** IDs (`commit.rs:373–438`); record
  references in patches are not existence-checked.
- The proposal's name path is a deterministic reserved-ID design:
  `NamePlan` built globally in commit order across all batches
  (`M1_PART_A_CONTRACT_PROPOSAL.md:2563–2566,2085–2088`), phase 2b predicted
  arena IDs before phase 2c link validation, phase 3 `ensure_intern` preflight,
  phase 4 infallible `intern_reserved` (`:2648–2662,2707–2713,2856–2866`,
  `intern_reserved` spec at `:3612–3618`). It matches T04 revision 2's required
  whole-batch pre-reservation property at the design level
  (`T04_LEX_CHIPS.md:88–91,140–142`), and `OB-10`/`OB-28` track the remaining
  co-freeze.
- Unstated case: phase 2c validates `Committed(r)` as "exists & live"
  (`:2570–2572`). A reference to a name newly planned **earlier in the same
  commit** is neither live nor a draft link of the referencing task, so the
  written rule would reject it. M1 stage order avoids this (T04 commits names
  before T06 symbol tasks), but the general contract does not say whether such
  predicted-name references are allowed or forbidden.

Recommendation: keep `OB-49` open; when the T01 relocation protocol is frozen,
either define predicted-name references explicitly (with the phase-2c check
extended) or state that a name interned in one task's batch is only referenceable
by later tasks after commit.

## NI-06 — Dedup and deterministic IDs (Verified; two clarifications)

Verified in `/5` code:

- Dedup is lookup-first: `index.get(bytes)` returns the existing `NameId`
  before any capacity check (`intern.rs:66–69`), so re-interning an existing
  spelling succeeds even at entry capacity.
- IDs are first-seen order: `NameId::from_index(self.entries.len())`
  (`intern.rs:82`), with a `BTreeMap<Vec<u8>, NameId>` index (byte-ordered) and
  `iter()` in ascending ID order (`intern.rs:50–54,128–133`). No hash-map
  iteration or address participates.
- Tests: `intern_is_deterministic` (`c01_arena.rs:68–82`) and
  `intern_capacity_is_structured` (`:84–96`). The snapshot encodes intern bytes
  in ID order (`snapshot.rs:396–400`), so replay recreates the same IDs.
- Sentinel safety: the capacity check `entries.len() as u32 >= limit` runs
  before assigning `from_index(entries.len())`, so the largest assignable index
  is `limit - 1 <= u32::MAX - 1`; `NameId::NONE` (`u32::MAX`) is never
  allocated for any `u32` limit.
- Proposal consistency: predicted ID `from_index(intern.len() + k)`
  (`:2087`) matches `intern_reserved`'s `from_index(entries.len())`
  (`:2857,3614`); duplicate spellings materialize once and reused names produce
  no patch (`:2849–2855,2709–2713`; test `duplicate-name idempotence (one intern
  entry)` at `:4162–4163`).

Clarifications (Low):

1. The proposal says `NamePlan`/`resolve` are "task-scoped ... only sees this
   task's batch" (`:2853–2855`) while phase 2a is explicitly global across all
   batches (`:2564–2566`). A spelling first seen in an earlier batch must
   resolve through the global plan (or rely on lookup-first
   `intern_reserved`); the text should say so explicitly.
2. `intern.rs:75` uses an unchecked `total_bytes + bytes.len()` whereas the
   proposal uses `checked_sum` (`:2660–2661`). The overflow is unreachable in
   practice, but a checked add would match the no-panic convention.

## NI-07 — `SymbolRecord.name` (Verified consistent)

- Final record: `SymbolRecord { name: NameId, ... }`
  (`M1_PART_A_CONTRACT_PROPOSAL.md:1340–1344`; T06 item 6a,
  `T06_SYMBOL_TYPE_CHIPS.md:29`).
- Draft form: `SymbolDraft.name: RecordLink<Name>`
  (`M1_PART_A_CONTRACT_PROPOSAL.md:1971`; T06 line 29).
- `RecordRef::Name`/`RecordFamily::Name`/`InternTable::get` mapping is coherent
  (`ids.rs:117–118`; proposal `:3037–3046`).

Caveat (already documented): `NameDraft` is T04-only, so T06 can only reference
committed names; the writer allowlist is proposed (`OB-32`, `OB-48`). No
change requested.

## Related open blockers

- `OB-10` — T04 reciprocal token↔literal same-batch link needs whole-batch ID
  pre-reservation (`M1_PART_A_CONTRACT_PROPOSAL.md:6475`).
- `OB-13` — `/6` counts/inventories and store↔family↔arena mapping
  (`:6478`).
- `OB-28` — "phase 2b" label collision and per-task collection pass
  (`:6493`).
- `OB-32` — per-chip manifests; `names.entries` append semantics (`:6497`).
- `OB-48` — `StoreOwnerViolation` absent from `/5` (`:6513`).
- `OB-49` — T01 §4 relocation protocol absent (`:6514`).

## Verification commands

- `git diff --check` — clean (exit 0) with the pre-existing working tree; the
  new report is untracked, so it was additionally checked directly with
  `git diff --no-index --check /dev/null docs/reviews/2026-10-05_NAME_INTERNING_AUDIT.md`
  (no whitespace errors).
- No `cargo` checks were run: this audit changes no code. The `/5` code facts
  above were read from the committed sources; the M1 documents are proposed
  candidates, not implemented behavior.
