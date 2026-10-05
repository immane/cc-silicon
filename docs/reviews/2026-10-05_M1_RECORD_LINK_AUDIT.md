# M1 Record-Link Audit — 2026-10-05

## Status and scope

- **Status: audit record only.** This is not an ADR, not an owner sign-off, not
  T01 `[INT]` acceptance, not a `/6` freeze, and it authorizes no interface,
  store, field, enum, rule, hash, task kind, chip, or code.
- **Date:** 2026-10-05.
- **Baseline:** the working tree at `HEAD 237cc3b` plus the uncommitted
  documentation in progress (M1 proposal, CDR, T04–T08 packages). Line numbers
  are those of the audit-time workspace and may drift.
- **Scope:** the record-link contract (`RecordLink { expect, target }`,
  `LinkTarget`, `RecordDraft::links()`, `ResolveTable`) at
  [M1_PART_A_CONTRACT_PROPOSAL.md](../tasks/M1_PART_A_CONTRACT_PROPOSAL.md)
  §3 item 2, §6.1, and §8, plus the record-reference sites in
  [T04](../tasks/T04_LEX_CHIPS.md)–[T08](../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md).
  Proposal §7 is read as the normative home of the checks because §3/§6.1/§8
  delegate to it.
- **Method:** read-only audit of the documents. The frozen `/5` statements are
  taken from [compiler/README.md](../../compiler/README.md) and the CDR rev-48
  read-only code facts; no code was executed or changed, and no contract file
  was edited.
- **Dimensions:** for each site — **family** (expected `RecordFamily` vs actual),
  **liveness** (committed target exists and is live), **range** (draft/committed
  index bounds, scalar range invariants), **order** (resolution order,
  pre-reservation, phase placement), **cycle** (required acyclicity), and
  **atomicity** (whole-batch reject before mutation, infallible apply).

## Sites audited

| Site | Location |
|---|---|
| S1 umbrella rule | proposal §3 item 2 (`877–882`) |
| S2 draft wrappers / `links()` / source-TU rule | proposal §6.1 (`1948–2021`) |
| S3 append fields, owners, write sets | proposal §8 (`2994–3124`) |
| S4 normative checks | proposal §7 phases 1–4 (`2480–2841`) |
| S5 token↔literal | T04 (`72–98`, `137–144`) |
| S6 continuation / task-child | T05 (`63–80`) |
| S7 scope / symbol / type | T06 (`23–68`) |
| S8 sem / conversions | T07 (`62–109`) |
| S9 constants | T08 (`5–15`, `27–51`) |
| S10 result values | proposal §6.3 (`2353–2372`), §7 apply (`2734`), §13 (`4040–4061`) |

## Per-site × dimension matrix

Legend: `Y` = rule present and located; `P` = partial/open (see note);
`–` = not applicable; `N` = missing (finding reference).

| Site | family | liveness | range | order | cycle | atomicity |
|---|---|---|---|---|---|---|
| S1 §3 item 2 | Y | Y | Y | Y | N (per-family only; RL-01/RL-02) | Y |
| S2 §6.1 | Y | Y (via §7 2c) | P (draft `k<len`; node token range) | Y | P (**scope parent RL-01**; Function↔Block coherence RL-02) | Y |
| S3 §8 | Y (ownership) | – | – | P (append/commit order) | P (continuation only) | Y |
| S4 §7 | Y | Y | P (RL-05) | Y | Y (Node, Continuation) | Y |
| S5 T04 | Y | Y (candidate) | P (OB-10 co-freeze) | Y | Y (reciprocal pair intended) | Y |
| S6 T05 | Y | Y (`ChildRef` predicate OB-29) | Y | Y (OB-28 label) | Y (ContinuationCycle) | Y |
| S7 T06 | Y | Y | – | Y | **N (RL-01)** | Y |
| S8 T07 | Y | Y | Y (per node/role) | Y | – (type graph beyond M1) | Y |
| S9 T08 | P (typed refs, no `expect`) | **N (RL-04)** | – | Y | – | Y |
| S10 result values | P | **N (RL-03/RL-04)** | **N phase (RL-03)** | Y (apply order stated) | – | P (depends on RL-03) |

## Findings

### RL-01 — Scope parent chain has no acyclicity / file-root rule (missing)

- **Severity:** high.
- **Location:** proposal §6.1 `ScopeDraft.parent: Option<RecordLink<Scope>>`
  (`1969`); §7 phase 2b(a) scope lifecycle (`2590–2599`); §6.4 error list
  (`2428–2430`); T06 items 6/6b (`27`, `40`).
- **Evidence:** `ScopeDraft.parent` receives only the generic link checks
  (family, committed existence/liveness) from §7 phase 2c (`2570–2572`). The
  cumulative phase-2b(a) check validates Enter/Exit counts, `event.at`, owner
  uniqueness, and the file-scope policy — it never walks `parent`. Error names
  exist for the two other parent/previous chains (`NodeCycle`, `ContinuationCycle`,
  `2428`/`2430`) but none for scopes. `ScopeDraft.parent` is also not marked
  committed-only, so a same-batch `A.parent → B`, `B.parent → A` cycle resolves
  through the predicted-ID table and passes all current checks.
- **Impact:** the committed scope graph can be cyclic or have no file root, while
  T06's selected lookup "walks the active scope chain only" (T06 `13`) with no
  documented bound or visited set. A cycle makes that walk unbounded/undefined
  and can make deterministic lookup order non-terminating. The M1 fixture (one
  file scope, one block scope) does not expose it.
- **Recommended documentation remedy (not applied):** add a cumulative
  phase-2b(a) rule that the `parent` chain is acyclic, rejects self-parent, and
  terminates at exactly one `File` scope with `parent: None`; add an error name
  (e.g. `ScopeParentCycle`/`ScopeParentInvalid`) to §6.4; state whether
  `ScopeDraft.parent` may be same-batch or is committed-only; add a §13 invalid
  case and a T13 fixture note.
- **Authority:** T06 + T01 `[INT]` co-freeze. Not present in the current
  OB-1…OB-53 register.

### RL-02 — Function↔Block reciprocal coherence is not validated (missing)

- **Severity:** medium.
- **Location:** proposal §6.1 `FunctionDraft.entry` / `BlockDraft.function`
  (`1976–1977`); §5 `FunctionRecord.entry` / `BlockRecord.function` (`1836–1837`);
  §7 phase 2b(d) (`2615–2633`); §13 normal case (`4040–4041`); T09 (`77–78`).
- **Evidence:** the two fields are validated only as links (family, liveness).
  Phase 2b(d) validates instruction→Block (`BlockMissing`/`BlockTerminated`),
  value producers, and terminator rules; it does not check that `entry` resolves
  to a block whose `function` is the owning function, that a block's `function`
  resolves to a function whose `entry` includes it, that a block is the entry of
  at most one function, or that `BlockRecord.ordinal` is unique per function.
  The mutual resolution cycle itself is intended and tested: §13 `4040–4041`
  lists "intra-batch forward ref and cycle (`Function.entry ↔ Block.function`)",
  and the predicted-ID pass resolves it without topological ordering.
- **Impact:** commit can accept an inconsistent IR graph (e.g. `Function.entry`
  pointing at a block owned by another function); the M1 single-function/single-block
  fixture hides it, but the structural contract and the T13 CFG checks generalize.
  The rev-49 `TerminatorMissing` hook checks only that the entry block is
  terminated, not that it is the right block for the function.
- **Recommended documentation remedy (not applied):** add phase-2b(d) reciprocal
  checks with named errors (candidates: `FunctionEntryMismatch`,
  `BlockFunctionMismatch`, `BlockOrdinalNotUnique`), plus a §13 invalid case; or
  explicitly record that Function↔Block coherence is a chip-owned invariant and
  is not commit-validated (with the rationale), so the `RecordLink` "central
  validation" claim is not overread.
- **Authority:** T09 + T01. Not present in the OB register.

### RL-03 — `ResultValue::DraftRecords` has no assigned pre-apply validation phase (missing)

- **Severity:** medium.
- **Location:** proposal §6.3 `DraftRecords(Vec<DraftRef>)` (`2355`); §6.4
  `ResultDraftRefWithoutBatch { task, ordinal }` (`2405`); §7 apply
  `resolved_table[(task,r)].1` (`2734`); §7 properties "No fallible step in
  apply" (`2834–2836`) and "Apply order" (`2981–2986`); §13 invalid case
  (`4061`); §6.4 `ForeignDraftRef` note (`2474`); OB-42 (`6458`).
- **Evidence:** `DraftRecords` is not a `RecordLink`, is not enumerated by
  `RecordDraft::links()`, and is not covered by the phase-2c link validation.
  `ResultDraftRefWithoutBatch` is named in the error list and §13 names the
  invalid case, but **no phase or predicate is stated for it** (the identifier
  occurs only in the error list). Apply resolves the refs by direct indexing in
  the pass that §7 declares infallible. The ordering property (`2981–2986`) says
  phase 4 materializes every body before resolving `DraftRecords` and that a
  result may reference "any draft in its own batch" — but the pre-apply check
  that enforces "own batch" and index range is exactly the missing piece. §6.4's
  claim that `ForeignDraftRef` is unrepresentable (`2474`) holds only because
  `DraftRef` is intended to be task-scoped; the type itself does not carry the
  task.
- **Impact:** if the check is not specified (and later implemented), apply either
  panics on a key-miss (violating "no panic" / "apply has no `Result`") or
  silently resolves a foreign/out-of-range draft. The result contract is also
  ambiguous about expected family per element (none is carried) and about which
  error/phase applies.
- **Recommended documentation remedy (not applied):** assign
  `ResultDraftRefWithoutBatch` to a named pre-apply phase (phase 1 or 2c) with
  the predicate `r < len(enclosing task's AppendBatch.records)` and resolution
  in the enclosing task's table; state in §6.3 that a result's `DraftRef`s are
  scoped to `Complete.task` and that this is what makes cross-task draft refs
  unrepresentable; keep OB-42 for the separate hash-scope omission.
- **Authority:** T01 `[INT]`.

### RL-04 — Task-payload / result committed references have no liveness rule (missing)

- **Severity:** medium.
- **Location:** proposal §3 item 2 "for every link" (`877–882`); §6.1 source/TU
  rule (`1991–2008`); §6.3 "Payload stays `RecordRef`-only" (`2369–2372`); §7
  phase 1 `Enqueue: existing validate (payload RecordRef-only)` (`2545`); T04
  `115–117`; T05 `109`, `117`; T06 item 7b (`60`); T07 `83`; T08 `12`, `33`,
  `51`; [compiler/README.md](../../compiler/README.md) `214–221`;
  T01 C03 (`154`).
- **Evidence:** the only payload-reference liveness rule is the source-scoped
  single `RecordRef::Source` (exactly one, live, `1998–2001`). `Enqueue` payload
  validation is shape-only. Other payload/result refs — `ConstantRequest`
  `RecordRef::Literal`/`node` refs (T08 `33`, `51`), the TU-root `NodeId` in the
  scope-enter enqueue (T05 `109`, `117`), T06 lookup refs (T06 `46`), and result
  values `Record(RecordRef)`/`Records(Vec<RecordRef>)` — have no commit-side
  existence/liveness rule and no documented exemption. The frozen `/5` README
  exempts only **store-patch** refs (`219–221`); the proposal separately selects
  patch-ref checking (`2988–2990`), leaving payload/result refs unaddressed.
- **Impact:** a task can be enqueued (or completed) with a payload/result ref to
  a missing or tombstoned record; the consuming chip's behavior is unspecified
  (typed miss vs panic vs fabricated default). Because `Payload` is
  `RecordRef`-only, the liveness obligation should be stated once rather than
  left to each chip.
- **Recommended documentation remedy (not applied):** either (a) add a pre-apply
  check that every committed `RecordRef` in `TaskDraft.payload` and in
  `ResultValue::{Record,Records}` exists and is live, with a named `CommitError`,
  or (b) explicitly document that payload/result refs are producer-owned
  obligations that the mechanical commit does not check, plus the child-side
  typed-miss rule. Option (a) matches the §3 "central validation" direction;
  option (b) must be stated, not implied.
- **Authority:** T01 + T04–T08 co-freeze. Not present in the OB register.

### RL-05 — Committed-reference dereference/range predicate for multi-arena families is unnamed (secondary)

- **Severity:** low–medium.
- **Location:** proposal §7 phase 2c `Committed(r): … r exists & live` (`2571`);
  §6.2 `family()`/`make()` (`2041–2044`); `arena_allocated` (`2131`); §8
  multi-arena notes (`2652–2654`).
- **Evidence:** "exists & live" is required but the per-family dereference is not
  specified. Families sharing one store have arena-local indexes (`sources`
  spans/expansions; `symbols` scopes/symbols/scope_events; `ir`
  functions/blocks/values/instructions). `arena_allocated(family)` maps
  family→arena for allocation; the total `resolve(ref) -> Option`/`is_live(ref)`
  predicate (including tombstone handling) is implied but unnamed.
- **Impact:** range/existence checking can be implemented inconsistently across
  families; snapshot/replay visibility of tombstoned slots depends on it.
- **Recommended documentation remedy (not applied):** name the per-family
  dereference/liveness predicate in §7 phase 2c (or cross-reference the existing
  arena checked access) and add it to the `/6` co-freeze inventory.
- **Authority:** T01 `[INT]`.

## Rules already covered (do not re-flag)

- Cross-task **draft** refs impossible; cross-task **committed** refs are the
  normal flow (§3 item 2; §6.3 `2370–2372`).
- Generic link family/liveness/range: `expect` match, committed existence,
  `Draft(k) k < len` (§7 2c `2570–2572`); pre-mutation and whole-batch atomicity
  (§7 phases 1–3; "No fallible step in apply" `2834–2836`).
- Predicted-ID pre-reservation before link validation, so intra-batch mutual
  references need no topological ordering (§7 phase 2b; T04 `88–90`).
- Node `parent` acyclicity + ordinal uniqueness + committed-only token range
  `first <= last` (§7 2b(b) `2600–2609`; §5 `1210–1224`).
- Continuation `previous` committed-only + `ContinuationCycle`; OwnBatch
  phase-1/phase-2b index checks (§7 2b(g) `2643–2644`; T05 `73–80`).
- Scope lifecycle (Enter/Exit order, `event.at` liveness, owner-node identity,
  DOC-13 fix) (§7 2b(a) `2590–2599`; T06 `40`, `130`).
- Sem per-node uniqueness, conversion role uniqueness, recursive nested link
  validation (§7 2b(c) `2610–2614`).
- IR instruction→Block `BlockMissing`/`BlockTerminated`, value producer,
  terminator rules, and the rev-49 entry-termination hook direction
  (§7 2b(d) `2615–2633`).
- T04 token↔literal reciprocal cycle is an intended, resolvable case with a
  defined fallback (T04 `88–91`, `142`); the open item is the T01 co-freeze of
  the whole-batch pre-reservation property (OB-10).
- Apply order: all append bodies materialized before `Complete{DraftRecords}`
  resolution (§7 `2981–2986`).

## Recommended integration actions

1. Register RL-01, RL-02, RL-04, and RL-05 in the proposal's OB register (or the
   active review tracking) so the missing rules are not lost; RL-03 should be
   linked to OB-42 as its validation-phase counterpart.
2. Decide each finding as either a commit-side rule (preferred for RL-01,
   RL-02, RL-03, RL-04a) or an explicit documented exemption (RL-04b, RL-05),
   and record the decision with owner/authority.
3. Add the corresponding §13 invalid/boundary tests and T13 fixture notes when
   the rules are drafted.
4. Do not treat this audit as a `/6` freeze input by itself; the rules remain
   prospective until the owning task packages and T01 co-freeze them.

## Verification record and limitations

- Read-only audit; no contract, task package, code, or test file was modified.
- `git diff --check` was run after writing this file (result recorded below).
  This file is untracked, so it was additionally checked for trailing
  whitespace/CR/tab characters directly.
- The audit did not run Cargo, the candidate compiler, or any target probe; it
  makes no compiler-capability, target, or pass-rate claim.
- Line numbers refer to the audit-time workspace and may drift after later
  documentation revisions.
