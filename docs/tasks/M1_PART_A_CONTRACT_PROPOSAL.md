# M1 Part A Contract Proposal: Typed Record Materialization for the Frontend + IR

Status: **DRAFT — not accepted/frozen as an application.** The `/6` contract
itself is frozen (see rev 41): no owner sign-off has been obtained for the
application direction and no owner approval may be inferred from
this document. Every reviewer listed in §15.3 remains **conditional**.

This document is a proposal. It does **not** freeze any interface, does **not**
change `compiler/contracts/CONTRACT_VERSION`, does **not** patch the compiler,
and does **not** authorize language chips. Everything below is a request for the
T01 integrator to accept, amend, or reject. Where a concrete field or type shape
cannot be safely determined from the current task documents, it is marked
`RESIDUAL — integrator must accept before /6` and is not presented as settled.

**Current state (read this first).** This proposal document is at **rev 40** (the
OPEN-03 T01/T07/T08 co-freeze fix; see the rev-40 paragraph below and the `§16`
rev-40 row), with the preceding rev-39 content being the
docs-only integration of the then-current companion state — **CDR rev 53**, ADR-0002
Revision 17, and the then-newly updated T02 rev 32 / T03 rev 53–54 / T04 rev 47 /
T05–T09 / T13 packages plus the M1 vertical rev 27 / target rev 29 acceptance; see
the `§16` rev-39 row and the `§24` ledger. The current-state M1
pointer/range advances to **rev 21–40** (the operative revision
content is rev 40; the rev-38 content is the previous integration snapshot, see
the `§16` rev-38 row and the `§23` ledger). **Pointer
correction (rev 40; no revision advance):** the companion state has since
advanced — the CDR is at **rev 55**, which **points at M1 proposal rev 40
(current)** as its shared resulting file (the rev-53 rows named rev 38 and the
rev-54 row named the expected rev 39); **T02 is at rev 35,
T03 at rev 56, T04 at rev 47 + task revisions 1–3 (current task revision 3), T13
at rev 11, M1 target at rev 37, and M1 vertical at rev 34**; ADR-0002 stays
**Revision 17**. These values are as of 2026-10-05 and each document's own
revision record is authoritative for any later change. This note corrects only
the present-tense pointer; it advances no revision, changes no acceptance
status, and freezes nothing. The
per-revision paragraphs below are **historical snapshots in (mostly)
reverse-chronological order**; a paragraph describing an earlier revision is
**as-of that revision** and must not be read as the current pointer or status.
The rev-40 paragraph is the latest snapshot below (it immediately follows this
current-state paragraph), the rev-39 paragraph is the next historical snapshot,
and the rev-38 paragraph the one after that; the current pointer is recorded in
the `§16` rev-40 row and the `§24` ledger. Status is
unchanged from the latest revision:
this proposal is **DRAFT**, [`/5`](../architecture/SFL_CONTRACT.md) stays current
(the `/5` artifact is `t01-c01-c06/5`), the `/6` contract stays **unfrozen**,
ADR-0002 stays **PROPOSED**, H6 is a
**selected quota>1-capable bounded recovery mechanism to be frozen in `/6`**
(user, CDR rev 42; not owner/T01 signoff, not implemented), H9
(in-flight scheduling ownership) remains an **accepted-in-principle removal
direction pending T01 integrator acceptance** with its residual proof (mutation
boundary/`in_flight` clear order/residual no-`Running` proof) still open, and the
rev-51 delegated candidate default (sole `max_inflight_per_tick`/quota bound; the
redundant `max_dispatches_per_tick` dropped from the candidate) is recorded but
**not** an owner/T01 signoff or freeze. A/B/C and H1 remain in principle only, D–I
remain pending, every owner's
incomplete bundle and the T01 co-freeze remain pending, and **every T01
integrator and owner sign-off is pending**. There is **no code** and **no Part B
substrate** (no assembler/linker/probe/runner). The rev-53 delegated candidate
default (`Lx08CandidateType` M1 vocabulary = the closed one-member set
**`{ Int }`**, M1 literals `2`/`3` = target-independent `Int` with **no bit
width**) and the newest T03/T05/T06/T07/T08/T09/T13 candidate sections are
recorded in `§24` as **candidates only** — not owner/T01 signoffs and not a
freeze.

**Revision 40 — OPEN-03 fix: M1 constant-expression handoff co-freeze (2026-10-05;
doc-only; T01/T07/T08 co-freeze; no code, no `/6` contract freeze).** Closes the
documentation-review OPEN-03 gap and the `§24.10`/OB-9 open item: the M1
committed constant-expression **input path** and the `ConstantRequest`/
`ConstantResult` M1 **variants** are **selected and frozen for the M1 `/6`
co-freeze** — the sem-stage per-use `const.evaluate` request covers both the
leaf-literal use (`ConstantRequest::Literal { literal, node, required_kind }`) and
the binary additive use (`ConstantRequest::Binary { node, op: ConstExprOp, lhs,
rhs, required_kind }`), with the committed `BinaryExpression` `NodeId`, T07's
checked operator (`Add` in M1), and the committed T04 `LiteralRecord` refs of
both operands; T08 commits **exactly one** `ConstRecord` for the folded result;
T09 consumes the same committed `RecordRef::Const` and does not re-fold. A real
T07→T08→T09 fixture (`M1-CL-05`, M1 frontend acceptance rev 31) replaces the
hand-built-`5` evidence path; `M1-CL-03` is clarified as a conversion-unit
fixture only. Still open: the result envelope (`ResultValue` carrier, OB-6),
non-legal coupling (OB-7), `ConstRecord` identity/reuse (OB-8), and the exact
wire tags/payload-variant spellings/task-kind spellings/numeric codes.
Companion edits: T07 rev 4, T08 task-package revision 5, T09 rev 49, M1
frontend acceptance rev 31, and CDR rev 55 (which records the same co-freeze).
No ADR/T01/manifest/compiler/test edit by this
revision; `/5` current; `/6` unfrozen; M1 DRAFT; no code.

**Revision 39 integrates the current companion state (CDR rev 53, ADR-0002
Revision 17, the newly updated T02 rev 32 / T03 rev 53–54 / T04 rev 47 /
T05–T09 / T13 packages, and the M1 vertical rev 27 / target rev 29 acceptance
docs) into this proposal (docs-only; no new user decision, no acceptance, no `/6`
change; see the `§16` rev-39 row and the new `§24` ledger).** It advances the
current-state M1 pointer/range to **`rev 21–38`** and records the newest
candidate status without inventing schema or freezing. **(1) CDR rev 53
delegated candidate (T04 rev 47 cross-reference in T08 rev 2):** for the
**exercised M1 literal subset** the `Lx08CandidateType` M1 vocabulary is the
**closed one-member set `{ Int }`**, with M1 literals `2`/`3` represented as
**target-independent `Int` with no bit width**; literal forms **outside** the
exercised subset (non-decimal radix, non-`None` suffix, character/string, any
not-yet-specified category) must **not silently default to `Int`** and are
**explicitly unsupported/deferred**; future categories **append without
reinterpretation** — a **delegated candidate default, not** an owner/T04 or T01
`[INT]` signoff, **not** a `/6` freeze, not the complete future C candidate set,
no numeric tags; the complete member set/encodings remain **open**. **(2) T03
rev 53–54:** the delegated **`raw_offsets` boundary semantics** for the M1
single-source `Normalized` artifact (a primary location map, **not** complete
provenance; `CRLF`→`LF`/inserted-terminal-`LF` boundary rules; exact error
codes open) and the new **M1 PP01 producer/consumer contract candidate**
(single-source `Normalized` input only; four normalization outputs; the
`source_scoped_one_hop` source-provenance rule with equality **deferred**; a
delegated `PpRequest`/`PpResult` typing/timing candidate; field-scoped manifest;
remaining co-freeze blockers listed) — both **candidates only**. **(3) T05/T06
TU carrier + exactly-once still open:** the T05 item-F and T06 item-5 TU-carrier
candidate default supplies the committed TU `NodeId` from the **existing generic
`ResultValue::Record(RecordRef::Node(root_node_id))`** payload of the committed
`TaskState::Completed(ResultId)` — **no** new typed result variant/family and no
task-kind-only derivation — with the **T05 item-F point 5 / T06 item-7
exactly-once clarification**: `Completed(ResultId)` **alone is not sufficient**
to drive the stage edge; the candidate requires a **result-consumption**
condition (consume + enqueue in one commit-visible unit) or, failing that, a
**T01-defined equivalent persistent delivery claim**, with the exact
consume/enqueue envelope, retry/error recovery, and T06 enqueue-payload task
typing **open** T05/T01/T06 co-freeze. **(4) T07/T08/T09/T13 newest candidate
sections:** the T07 `SemRecord` field-set candidate checklist and the M1-only
constant-evaluator candidate; the T08 H1-aligned payload + M1-only evaluator
candidate; the T09 M1-only IR-shape checklist and delegated rev-47 rule
defaults; and the T13 M1 fixture/registration checklist plus the explicit
H6/H9 verification matrix (H6-M01..H6-M16, **required pending fixtures**) — all
**candidate/prospective `/6`**, none frozen. **(5) T02 rev 32 H6/H9 checklist:**
a compact T02-owned **remaining co-freeze contract checklist** converting each
open H6/H9 point into an exact question plus a testable invariant (H6.1–H6.6,
H9.1–H9.4), explicitly **proposed, not signoff/freeze**, and leaving the four
T01 architecture-critical dispatcher/commit items to T01. **(6) Audit findings
still open (recorded in `§24`):** the **T04 reciprocal token↔literal
`RecordLink`** relation is a **mutual/reciprocal cycle** that requires
**pre-reserved IDs before links resolve** (the append-reference mechanism must
allocate IDs before a reciprocal pair can be checked) — an **open T04/T01 `/6`
co-freeze blocker**, not a resolved mechanism; the **T08 `§G2` signed-range
overflow formula** remains an **unselected draft proposal/reference** (T08
records it as not adopted; the formula/enforcement split, symbolic-vs-probe
gating, and signed-representative wording stay open); the **M1 `PP-08`
exact-map/failure semantics** (mandatory-map invariants for the single-source
`Normalized` artifact; missing-source/bad-map rejected before mutation; the
exact classification/error codes open); and a **H9 clearing conflict plus the
T13 no-residual evidence gap** (the `in_flight` clear ownership/order,
"clearing in-flight is not itself a transition", and the all-paths
no-`Running`/no-residual proof remain **open**, with the T13 H6/H9 matrix rows
marked **required pending fixtures**, **not** a proof). The rev-50 two-tier hash
scope, the rev-49 H11/T09/T01 direction, and the rev-51 sole-per-tick-bound
delegated candidate default remain current; `/5` is current, `/6` **unfrozen**,
M1 **DRAFT**, ADR-0002 **PROPOSED**, there is **no code**, and there is **no
Part B substrate**. The CDR is at **rev 53**, ADR-0002 at **Revision 17**, T02 at
**rev 32**, M1 target at **rev 29**, and M1 vertical at **rev 27**. No schema is
invented and nothing is frozen.

**Revision 38 integrates the current companion state (CDR rev 51, ADR-0002
Revision 17, updated T02–T09/T13, and the M1 vertical/target acceptance docs)
into this proposal (docs-only; no new user decision, no acceptance, no `/6`
change; see the `§16` rev-38 row and the new `§23` ledger).** It advances the
current-state M1 pointer/range to **`rev 21–37`** and records seven things.
**(1) CDR rev 51 delegated candidate:** `max_inflight_per_tick`/quota is the
**sole per-tick dispatch-count bound** and the redundant
`max_dispatches_per_tick` is **dropped from the candidate limit inventory and its
validation** (with the duplicate `DispatchBudgetExceeded` dispatcher failure
dropped from the candidate inventory), grounded in the rev-33/34 H9 direction and
the rev-49 audit duplication finding — a **delegated candidate default, not** an
owner/T01 `[INT]` signoff, **not** a schema freeze; ADR-0002 Revision 17 and T02
rev 31 mirror it; and the H9 mutation boundary / `in_flight` clear order /
residual no-`Running` proof remain **open**, with H6 mechanism unchanged.
**(2) Delegated `ParseContext` 10-member candidate** (T05 package records it):
`ParseContext { TranslationUnit, ExternalDecl, Specifier, Declarator,
ParameterList, Block, Expression, Assignment, Unary, Primary }`, with `context`
**semantically distinct** from `production: TaskKind` and not derived/elided
absent an explicit frozen one-to-one mapping; the mapping and discriminants
remain **open/T01 co-freeze**.
**(3) T04 `LiteralRecord.token` delegated candidate:** `Some(committed TokenId)`
when the committed literal came from a single committed source C token (M1 `2`
and `3`), `None` for an explicitly synthesized literal with no single source
token; any same-batch token↔literal relation uses the generic typed `RecordLink` /
append-reference mechanism and is resolved to durable committed IDs before
persistence — **not** the task-level `ContinuationRef::OwnBatch`/`ChildRef::OwnBatch`;
the exact mechanism/local-link spelling remains open.
**(4) T08 opening payload aligned** to `ConstantRequest { literal:
RecordRef::Literal, node, required_kind }` → `ConstantResult { value:
RecordRef::Const, legality }`, with the sem→T08 const→T09 IR order corrected (T07
consumes no T08 output), and the exact payload/encoding/tag/task-typing/numeric
codes still **open/T01 co-freeze**.
**(5) M1 vertical acceptance artifact scope corrected:** the M1 exercised
artifact-map path is **only** the single-source `Normalized` artifact; the other
map-mandatory kinds (`Spliced`/`CommentFree`/`Preprocessed`) are schema-declared
but **not produced/asserted** in M1, and the **source-provenance/equality**
relation remains **pending/deferred** (an audit-identified mismatch now resolved at
the document level, with the `raw_offsets` semantics / token linkage / candidate
closed set still open T03/T04/T01 blockers — not claimed all resolved).
**(6) M1 target acceptance rev 29** aligns the IR28 committed `FunctionEnd`
terminal-result completion fact with the T01 phase-2b hook and the rev-50
**two-tier hash scope** (frozen `/6` seed hashed; post-seed runtime declarations
excluded); **Part B remains unavailable**.
**(7) T01 readiness audit** is summarized in `§23` as an ordered freeze-blocker/
dependency checklist (owner-signed schemas/payloads/manifests; families/tags/
ordinals/canonical encoder; T05 TU/context; T06 lifecycle/allowlist; T07
`SemRecord`/VF06; T08 const; T09 hook; pipeline H6/H9; hash/snapshot tests) — a
concise ordered checklist, not the whole audit. The rev-50 two-tier hash scope and
the rev-49 H11/T09/T01 direction remain current; `/5` is current, `/6`
**unfrozen**, M1 **DRAFT**, there is **no code**, and there is **no Part B
substrate**. The CDR is at rev 51, ADR-0002 at Revision 17, and T02 at rev 31.
No schema is invented and nothing is frozen.

**Rev-38 pointer correction note (same rev 38; historical snapshot above
preserved).** The "CDR rev 51" figures in the rev-38 record and row above are
accurate **as of the rev-38 integration snapshot** (what rev 38 integrated at
authoring time), and are retained as history. The **current** companion CDR is
**rev 52** — a read-only cross-owner audit/reconciliation with **no matching
proposal revision** — which **points at M1 proposal rev 38** as the shared
resulting file. This note corrects only the present-tense pointer; it advances no
revision, changes no acceptance status, and freezes nothing.


(docs-only; no new user decision, no acceptance, no `/6` change; see the `§16`
rev-37 row and the new `§22` ledger).** It adds a **proposal-side record** of the
user/authority responses already recorded in the CDR, and advances the
current-state M1 pointer/range to **`rev 21–36`** (this header/§16/§18.7/§19.6/
§20.6/§21). It records five things. **(1) User-accepted selected subdecisions
rev 42–46** (per-subdecision only; overall bundles still PENDING): A/T02/T13/T01
— H6 is retained and its **quota>1-capable bounded recovery mechanism is selected
to be frozen in `/6`** (rejecting the §9B row-A deferral), with pre-dispatch
errors leaving tasks `Ready`, a failed semantic batch commit mutating no semantic
state, a deterministic bounded recovery processing the dispatched tasks **once in
dispatch order** to `Failed`, **no pre-reservation of N diagnostics**, a per-task
diagnostic attempt with the `DiagnosticId::NONE` fallback, and a state guard
against duplicate transition; D/T02/T05/T01 — the join is a **commit-apply
invariant with no new CT07 committed carrier/family**, **await-all** children
terminal before the parent decision; D/T05 — the `ContinuationRecord` **exact
ordered fields** (`production`, `cursor`, `context`, `binding_power`, `scope`,
`parent`, `partial_children`, `next_child_ordinal`, `previous`, **no `awaited`**)
and the formal `/6` T01 §4 `WaitSet`-only supersession (**no `/5` edit**); D/T01
— the exact `OwnBatch` pre-apply validation; F/T06/T07/T09/T01 — **no
`FunctionContextId`**, `SemRecord` = the committed materialization of
`CheckedNode` (one per `NodeId`) with an explicit committed typed link consumed by
T09; F/T07 — `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2,
Void=3 }` with M1 `NonLvalue` and M1-only `EffectMask(0)`; F/T07/T13/T01 — VF06
`TypedAstInvariant` **is in M1** (exact registration open) and the **M1-minimal
conversion scope** (non-M1 conversions unsupported/deferred, no variant list
invented); E/T06/T01 — a deterministic bounded scan of committed `types.records`
returning the **lowest matching `TypeId`** (no hidden cache/index) and the File
`Enter` as a deterministic `parse.TranslationUnit → symbol_type.scope-enter` stage
edge after the committed TU (no job-bootstrap); C/G/T03/T04/T07/T08/T01 — a hashed
`limits.max_const_bits` origin with M1 cap/default 128 and config rejection `>128`,
`required_kind` as the per-use constant-expression requirement, `legality` as a
result payload field, the total eight-`ArtifactKind` map rule with M1 scope only
single-source `Normalized`, the exact ordered `LiteralRecord` fields plus
`LiteralKind`/`LiteralSuffix`/radix scope, `RequiredKind` =
`IntegerConstantExpression`, `ConstLegality` ∈ {`Legal`,`NotConstantExpression`,
`Unsupported`}, and the mandatory artifact-map invariants. **(2) Rev 47
user-delegated candidate defaults** (optional-`ArtifactKind` map policy, T05
parse-depth, T06 namespace/lookup, T09 noncritical rule defaults) are recorded as
**selected under user-delegated integration default, clearly labeled not owner or
T01 `[INT]` signoff** (not owner acceptance, not T01 `[INT]`, not a freeze).
**(3) Rev 48 code audit discrepancy and the unresolved hash conflict as it existed
then:** the `/5` code facts are **24** `RECORD_KINDS` / **24** `RecordRef`
variants (tags 0–23) with `RecordFamily` **absent**; the draft 19/27/27 counts are
**proposed/unverified**; and the `M1AppendSchema` hash-scope conflict between
`CONTRACT_VERSION`/`contract.rs`/`README.md` (`hash_excludes=…group-declared-store
-fields…`) and `COMPILER_SFL_MANIFEST.md` §4 is **real and unresolved**, with the
three hash-scope options then **unselected and unrecommended**. **(4) Rev 49 user
H11/T09/T01 direction + H9 read-only audit:** the `TerminatorMissing` trigger
**reuses the committed IR28 `FunctionEnd` terminal result**
(`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic
completion fact, checked by a **T01-owned typed phase-2b commit-apply validation
hook** — **no** new `CompletedFunction` marker family/ID/arena/`RecordRef` tag/
`RecordFamily` ordinal/snapshot encoder/`ResultValue` variant (**not a frozen `/6`
hook, no code**), with the earlier new-marker direction **superseded for the
operative direction but preserved as history**; and the H9 residual blockers
(quota>1 pipeline unimplemented; dispatcher `Ready→Running`/`in_flight`-vs-commit
relationship, `in_flight` clear ownership/order, and the all-paths no-`Running`
proof open; the `max_dispatches_per_tick`-vs-`max_inflight_per_tick` limit conflict
deferred to `/6`; fan-out fixtures and T13 VF02/VF03/VF04/VF13 pending). **(5)
Rev 50 two-tier hash scope:** the user accepts the **two-tier model** —
`StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed participating
in the `/6` contract hash**, while **post-seed runtime `StoreSchema::declare()`
extensions stay hash-excluded**, captured/validated via runtime snapshot/schema
mechanisms; at `/6` integration T01 must **atomically** update the
`COMPILER_SFL_MANIFEST.md` §4 wording, the `hash_excludes` semantic token (scoped
to post-seed runtime declarations), and `FrozenSchema::encode`/contract code plus
the freeze test, **preserving the frozen `/5` hash and history** — this settles the
conceptual boundary but **accepts no exact `M1AppendSchema` contents/counts, freezes
nothing, changes no `/5`, and authorizes no code**; the numeric inventory and
freeze-test implementation remain pending and T01 still co-freezes the M1 seed
values after the owners. **Present status is unchanged:** `/5` is current, `/6` is
**unfrozen**, M1 is **DRAFT**, there is **no code**; every owner bundle is
incomplete and the T01 co-freeze remains pending; and there is **no Part B
substrate**. The old marker-family direction is **historical/superseded**;
`quota > 1` is **not** the M1 baseline (`quota = 1` is the semantic-comparison
baseline); and the Part A symbolic model remains probe-independent.

**Revision 34 completes the H9 integration across CDR/ADR-0002/T02 (a docs-only
consistency completion; see the `§16` rev-34 row).** Rev 33 recorded the user's
in-principle H9 removal direction and the D1–D3 corrections in **this proposal**,
and its `§16` rev-33 row forward-referenced the matching edits "via CDR rev 35,
ADR-0002 §1.1 via Revision 14, and T02 Rev 28"; those companion edits had **not**
yet been applied (this proposal's partial text was ahead of the CDR/ADR/T02
render). Rev 34 records their actual application: **the CDR** now records H9 in
its header/§2/§11/§13 and §A/§9/§10 plus the T02 owner-amendment row (CDR rev 35),
with the **D3 correction** — the CDR §13 frontend claim — applied as CDR rev 36
(the M1 frontend acceptance is at **Rev 25** and carries **no** H6/CT06 text;
**T02** is where the operative H6 text lives); **ADR-0002** records H9 and
advances §1.1 (**Revision 14**); and **T02** records H9 in its in-flight-bound
bullet/CT06 row and table (**Rev 28**). Every current-state M1 pointer/range
advances to the current **`rev 21–34`** (this header/§18.6/§18.7/§19.6/§19.10/
§20.1/§20.6/§20.10/§21, the CDR header/§2/§11/§13, ADR-0002 §1.1, and T02 Rev 28).
The rev-33 D1/D2 corrections (pointer-list repair; §20 status key) remain
operative; D3's operative effect is now the CDR §13 correction. H9 stays a
`/6` **working-basis direction only, pending T01 integrator acceptance**; the
remaining H9 `[INT]` items (the dispatcher-vs-commit `Ready → Running`
relationship and the residual-set semantics) stay open. It changes no other
acceptance status: M1 stays DRAFT, ADR-0002 stays PROPOSED, `/5` stays current,
H1 remains accepted in principle only, all T01/owner sign-offs remain pending, and
no chip or freeze is authorized.**

**Revision 33 recorded a new explicit user in-principle decision (2026-10-04) on
removing `max_inflight_total` from the `/6` candidate (see the `§16` rev-33 row),
integrates the H6 rev-32 audit defects D1–D3 (see the `§16` rev-33 row and the
`§21` correction note), and advances every current-state M1 pointer/range from
`rev 21–32` to the current `rev 21–33`. **H9 (removal direction accepted in
principle):** the user accepts removing `max_inflight_total` from the `/6`
candidate because sequential per-tick dispatch is already bounded by
`max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; `Waiting`
tasks are not in-flight; and any future cross-tick `Running` needs a separate CDR.
This is a `/6` **working-basis direction only, pending T01 integrator acceptance**;
`/5` is not changed, no bound/config/hash is frozen, and the H6 recovery semantics
are retained. `tasks.in_flight` remains an ephemeral per-tick scheduler batch only
if the candidate says so, and it clears at latch only after every dispatched task
has a terminal/`Waiting`/`Progress` outcome (or H6 recovery). The dispatch batch
bound remains the per-tick `max_inflight_per_tick` quota; the dispatcher's
`Ready → Running` pre-worker mutation is distinguished from the one ordered
atomic semantic commit; the remaining H9/H6 `[INT]` blockers are retained (see
§18.6). The `§16` rev-32 row's pointer list is corrected (D1) to name the actual
locations (this header, §18.7 H10, §20.6 Finding 6) rather than
§18.4/§19.2/§19.10/§20.1/§20.8/§20.10, and the stale §19.6 F6 pointer is advanced
to `rev 21–32` (it now advances to `rev 21–33`). The `§20` status key misattribution
(D2) and the CDR §13 frontend claim (D3) are corrected. It changes no other
acceptance status: M1 stays DRAFT, ADR-0002 stays PROPOSED, `/5` stays current,
H1 remains accepted in principle only, all T01/owner sign-offs remain pending, and
no chip or freeze is authorized.**

**Revision 35 is a docs-only consistency cleanup of stale active candidate prose
(no new decision, no acceptance, no `/6` change; see the `§16` rev-35 row).** It
removes remaining contradictions where the **current operative** text still
presented the rev-22/23 candidate as active despite the rev-33 H9 removal
direction: the **§4 integrator summary** for T02/T13/ADR-0002 and the **§4 H6/H9
summaries** (which had stated the phase/owner for `max_inflight_total` as
"undefined" without recording that the removal direction superseded it) are
rewritten to state the **current** status — H9 removal direction accepted in
principle, pending T01 integrator acceptance, with the dispatcher-
`Ready→Running`-vs-atomic-commit and residual-set items still open; H6 direction
accepted in principle, **implementation BLOCKED** — while the rev-23 framing is
retained explicitly as **superseded history, not overwritten**. The companion
CDR (rev 37: the §A9/§I dispatcher error lists no longer name
`InflightQuotaExceeded` as an active candidate), ADR-0002 (Revision 15: the §6
in-flight-bound paragraph stale-candidate cleanup), and T02 (rev 29: the
in-flight-bound bullet) apply the same cleanup. H6/H9 statuses are unchanged;
A/B/C and H1 remain in principle only; D–I remain pending. Every current-state M1
pointer/range advances to the current **`rev 21–35`** (this header/§16/§18.6/
§18.7/§19.6/§19.10/§20.1/§20.6/§20.10/§21, the CDR header/§2/§11/§13, ADR-0002
§1.1, and T02 rev 29). M1 stays DRAFT; ADR-0002 stays PROPOSED; `/5` stays
current; all T01/owner sign-offs remain pending; no chip or freeze is
authorized.**

**Revision 32 records a new explicit user in-principle decision (2026-10-04) on
the H6 batch-failure recovery direction (see the `§16` rev-32 row): after an
atomic semantic batch commit fails, the user selects the **deterministic
all-dispatched failure recovery direction** in principle — every dispatched task,
in **dispatch order**, transitions **exactly once** to `Failed`; a committed
`DiagnosticId` is attached when diagnostic/record capacity allows, otherwise the
`TaskState::Failed(DiagnosticId::NONE)` sentinel; **every** dispatched task leaves
`Running`; and **clearing the in-flight set is not itself a transition**. This
**generalizes the verified `/5` `fail_selected` single-task semantics** (the
single-task sentinel obligation is **not weakened**). It is a `/6`
**working-basis direction only**: the exact atomic/bounded implementation requires
**T01 integrator approval**, and the batch failure-atomicity details, the diagnostic
budget/state mechanism, and the T02/T13 owner fixtures/sign-offs remain **co-freeze
and pending**. H6 therefore moves from *no selected alternative* to *direction
accepted in principle, implementation BLOCKED pending T01*; the mechanism is **not**
implemented, not frozen, and no no-`Running` batch guarantee is claimed. It advances
every current-state M1 pointer/range from `rev 21–31` to the current `rev 21–32`
(this header and §18.7 H10 plus §20.6 Finding 6 — **corrected in rev 33**, which
also advances the stale §19.6 F6 pointer to `rev 21–32`; the CDR
header/§2/§11/§13 via CDR rev 34, and ADR-0002 §1.1 via Revision 13). It changes no
other acceptance status: M1 stays DRAFT, ADR-0002 stays PROPOSED, `/5` stays
current, H9 stays BLOCKED, H1 remains accepted in principle only, all T01/owner
sign-offs remain pending, and no chip or freeze is authorized.**

**Revision 31 is a docs-only history-gap repair (see the `§16` rev-31 row):
it supplies the missing historical rev-29 row (the §16 table had jumped from
rev 28 to rev 30 even though rev 29 was an operative docs-only
historical-documentation correction recorded in the header and the CDR §12
rev-29 row), adds the rev-31 row, and advances every current-state M1
pointer/range from `rev 21–30` to the current `rev 21–31` (this header,
§18.7/§19.6/§20.6, the CDR header/§2/§11/§13, and ADR-0002 §1.1 — Revision 12,
with the CDR Rev 32 row recording the same repair). It changes no acceptance
status: M1 stays DRAFT, ADR-0002 stays PROPOSED, `/5` stays current, H6/H9 stay
BLOCKED, H1 remains accepted in principle only, all T01/owner sign-offs remain
pending, and no chip or freeze is authorized.**

**Revision 30 records two explicit user in-principle decisions (2026-10-04) as a
new revision layer (see the `§16` rev-30 row): (A) the Guardrails §6.1 **narrow
interpretation** — a same-task `OwnBatch(DraftRef)` may exist only as a
**transient wire/proposal input before commit**, validated and resolved to a
committed `TaskId`/record before any persistent state, with no durable
cursor/`WaitSet`/join pointing at a draft/wire/address; the guardrail text is
**unamended** and **T01 integrator implementation-confirmation is pending**; and
(B) awaited children — `TaskState::Waiting(WaitSet)` is the **sole**
awaited-child-ID source and `ContinuationRecord` carries **no duplicate `awaited`**,
with the **T01 §4 supersession accepted in principle for `/6` only** (**no `/5`
edit**) and **T01 integrator + T05 owner acceptance pending**. Both are accepted
in principle only, **not a contract or freeze**; row D remains **PENDING
overall**; M1 stays DRAFT, ADR-0002 PROPOSED, `/5` current, all T01/owner
sign-offs pending, and no chip or freeze is authorized.**

**Revision 29 applies the docs-only historical-documentation correction (see the
`§16` rev-29 row): it corrects the rev-28 rows' "historical revision rows are
unchanged" claim (rev 28 did correct the CDR §12 rev-26 history row; the
historical rev-27 row is preserved verbatim, and other historical rows are
unchanged), records the ADR-0002 Revision 6 (rev-23) H10 chronology erratum
(rev 23 corrected the pointer to rev 21–23, with rev 24/25 extending it to
rev 21–24; ADR-0002 Revision 10), and advances every current-state M1
pointer/range from `rev 21–28` to `rev 21–29` (this proposal
`§16`/`§18.7`/`§19.6`/`§20.6`, the CDR header/§2/§11/§13, and ADR-0002 §1.1).
It changes no acceptance status: M1 stays DRAFT, ADR-0002 stays PROPOSED, `/5`
stays current, H1 remains accepted in principle only, all T01/owner sign-offs
remain pending, and no chip or freeze is authorized.**
Revision 28 applied the docs-only pointer reconciliation (see the `§16` rev-28
row): the CDR §12 rev-26 history item (6) before→after range was corrected to
`rev 21–24→rev 21–26`, and every current-state M1 pointer/range was advanced to
`rev 21–28`. It changed no acceptance status.
Revision 27 applied the rev-26 audit corrections (doc-only; see the `§16` rev-27
row): the CDR §12 rev-26 history item (6) pointer wording, the CDR §2 dashboard
note, and the CDR §13 update scope; this proposal's `§20.9` D1 cross-reference and
`§20.10` residual register (H11); and residual `frozen` wording at the stage
ordinals, the versioned contract input, and the `NamePlan`/`ResolveTable`/
`RecordDraft` interface. It changed no acceptance status.
Revision 26 had integrated the rev-25 independent audit (main findings 1–8 plus
the nested CDR §C/§D/§E request items, enumerated as items 1–12 in the CDR); its
point-by-point disposition is the `§20` ledger. Revision 25 had integrated the
rev-24 independent audit
F1–F9 and the verified `/5` `routing.rs` single-task failure guarantee (`§19`
ledger). Revision 24 records the user's in-principle acceptance (2026-10-04) of
the H1 literal-handoff allocation split (`接受拆分（推荐）`); this changes no other
pending status, is not a freeze, and is not code/chip authorization. Revision 23
integrated the independent rev-22 audit H2–H11 (T03/T04, T05,
T06, T07, T08, T09, T02/T13 pipeline, ADR-0002); its point-by-point disposition is
the new **§18 ledger**. Revision 22 had integrated the **accumulated rev-21
read-only review findings** (T03/T04, T05, T06, T07, T08, T09, and the T02/T13
pipeline + ADR-0002 review), recorded in the **§17 ledger**; no finding in §17 or
§18 is a sign-off and nothing is frozen. Revision 21
applied the **user's in-principle decision of 2026-10-04** (recorded
in the [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)) and the
rev-20 reviewer findings / independent audit (T03/T04/T05/T06/T07/T08/T09). The
user selected the rev-21 `/6` contract-revision **working basis in principle**
(direction only): (1) **one writer per store/field, no shared-writer carveout** —
`sources.spans` is **T03-only**, T04/Token reuse committed T03 PP spans with **no
span writes**, and AST `Node` uses a first/last `TokenId` range with **no T05
span write**; (2) a **deterministic bounded sequential stage pipeline** with a
`quota = 1` semantic-comparison-projection baseline and `quota > 1` permitted only
after measured before/after tick counts and separate integrator acceptance; and
(3) the **T04→T08 typed handoff via a committed T04-owned `LiteralRecord`**
preserving the semantic information `node`, `required_kind`, `legality`, and the
`LX08` candidate type as explicit co-freeze field requirements, with **T08 the
sole `constants.records` writer**. **H1: on 2026-10-04 the user also explicitly
accepted the recommended allocation split in principle (`接受拆分（推荐）`)** as the
`/6` revision working basis: the per-literal committed `LiteralRecord` carries the
lexical facts + `LX08` candidate type, the post-parse sem-stage per-use
`ConstantRequest` carries `node`/`required_kind`, and the `ConstantResult` carries
`legality`. This is **not a freeze** and **not code/chip authorization**; the exact
schema/variants and T01 integrator acceptance + T03/T04/T08 owner co-freeze/
sign-off remain pending (§5, §8, §18.1; CDR §C2/§F4). Revision 20 had integrated the revision-19 re-review findings
(independent audit H1/H2 plus T03/T04/T05/T06/T07/T08/T09 owner residuals);
revision 19 had integrated the revision-18 owner-review findings and the user's
**binding direction on pipeline scheduling** (§3.12, §6.2.1, §7, §10.5, §12.18,
§13): cross-tick compiler work is explicitly modeled as staged pipelines with
committed-ID cursors/joins; the scheduler is designed for a measured, bounded
in-flight quota `> 1` over independent stages; and `quota = 1` is retained
**only as the baseline equivalence mode**, not as the final optimization
profile. The pipeline design is consistent with
[ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md), which
remains **PROPOSED** and authorizes nothing. **No owner sign-off has been
obtained; all owners remain conditional; the in-principle user decision is not a
freeze.** Exact field-level manifests, enum variants, and the pipeline `/6`
amendments stay blockers. **DRAFT — not freeze-ready**; the compiler is not
implemented.

What revision 22 integrates (summary; the point-by-point ledger is §17):

- **T03/T04:** single-file T03-only span reuse is exact only for
  one-PP-token→one-C-token; adjacent-string (`LX14`) and `LX16`
  synthesized/pasted token provenance has **no** carried span and needs a typed
  T04 provenance carrier (blocker). The `source_scoped_one_hop` rule is
  **one-source only** and cannot model include expansion or a multi-source
  artifact (M1 limited; typed multi-source map is a blocker). Non-M1
  `ArtifactKind`s have no M1 writer and are not silently assigned.
  **H1:** the statement that `LiteralRecord` is a **lexical candidate fact
  without `node`/`required_kind`** (with those on the sem-stage per-use request and
  `legality` on the result) was the rev-22 proposed allocation; the user **accepted
  it in principle on 2026-10-04** (rev 24) as the `/6` working basis. Exact schema +
  T01/T03/T04/T08 co-freeze/sign-off remain pending (§5/§8; CDR §C2).
- **T05:** `ContinuationRecord.awaited` deletion **conflicts with the frozen
  T01 §4 wording** and is recorded as an explicit `/6` supersession request, not
  a silent removal; `ContinuationRefInvalid` is added to the error inventory;
  join resume-count is added to capacity preflight; resume stage is
  `stage_of(parent.kind)`; CT07 needs a concrete committed decision carrier;
  child-failure outcomes are reconciled; same-batch token links are
  committed-only; node-token-range and T04 PP-span-provenance invariants are
  stated as verifiable; the terminal diagnostic-capacity/no-`Running` mechanism is
  **BLOCKED** (rev 23, H6), not reconciled; parse-depth counts only parser-chain frames;
  empty-TU EOF range / synthetic nodes / `ErrorRecovery` raw-token location are
  stated. No T05 span write.
- **T06:** stale `NodeRecord.span` wording replaced by token-range links;
  canonical `TypeId` reuse lookup mechanism is a blocker; `RedeclarationConflict`
  is a **chip diagnostic**, not a `CommitError`; duplicate `(parent,kind)` scope
  detection is same-batch-explicit; the File-scope `Enter` runs as a T06 task
  after the T05 `TranslationUnit` is committed, pinning a committed `NodeId`
  (rev 23, H3; bootstrap order a T06/`[INT]` decision); the namespace-validation
  claim is narrowed to what a carrier can enforce; `NORMATIVE_RULES`/test-list
  commas are fixed.
- **T07:** `CheckedNode`/`SemRecord` identity is pinned to one committed record
  per checked node; the `FunctionContextId` conflict with the T07 package is
  surfaced (selected proposal direction, integrator pending); TY27 role,
  `ConversionOp` domain (incl. FP/pointer qualifiers), role→T09-chip mapping and
  `(op,role)` enforcement are co-freeze blockers; VF06 `Return`/
  `FunctionDefinition` rows are no longer contradictory; duplicate/missing
  conversion-role commit errors are explicitly absent (chip diagnostics); the
  sem→const order is fixed (vertical-acceptance T07↔T08 cycle corrected), and the
  separation of the two constant-request payloads is the **H1 allocation accepted
  in principle on 2026-10-04** (rev 24), with the exact shapes still pending.
- **T08:** the lex-stage literal request carries only the committed
  `LiteralRecord` + candidate type (no `node`/`required_kind`) — **the H1
  allocation accepted in principle (rev 24)**; `legality`'s committed
  carrier and the `LX08` stage/type carrier are exact-shape blockers; `max_const_bits`
  numeric carrier / `<=128` constraint / restricted chip-config access are
  blockers; the signed-representative wording for unsigned magnitudes is clarified.
- **T09:** the `CompletedFunction` marker has **no** approved
  family/store/arena/encoder/tag and is an explicit blocker (preferred over
  inventing a family); the phase-2b "at function end" wording is replaced by the
  marker trigger, but the trigger is only a **selected direction with the marker
  family/schema unresolved** (rev 23, H11); the rejection→rule-id list remains
  **unresolved** — both `ir.op-immediate-type` aliases and both
  `ir.terminator-missing*` candidates must be reconciled (rev 23, H8), and all
  IR rule/op statements are prospective `/6`; folded-`int5` ownership is reworded
  (T08 computes the `ConstRecord` value, T09 emits the IR `Constant`; the
  `folded_int5_part_a_producer` name is retired for `folded_int5_ir_constant`,
  rev 23 H7); `Constant` missing-immediate vs missing-result precedence is stated.
  **[Rev-49 note (user-accepted recommendation; T01/T09 co-freeze pending; not a
  `/6` freeze; no code): the marker-family residual in the T09 bullet above is
  superseded for the operative direction by the rev-49 `FunctionEnd`-
  terminal-result reuse (no new marker family); history preserved. See §18.8.]**
- **T02/T13 pipeline + ADR-0002 (current status; see §4 H6/H9 below for the
  historical rev-23 framing):** CT07 needs a committed decision carrier and is not
  decorative; **H9:** the user accepts **removing `max_inflight_total`** from the
  `/6` candidate in principle (rev 33, 2026-10-04), so the earlier one-phase bound
  with a separate `max_inflight_total`/`InflightQuotaExceeded` carrier is a
  **historical rev-21/22 candidate, not the current operative direction** (the
  rev-23 "single phase/owner for `max_inflight_total`" blocker is superseded by the
  removal direction); the remaining H9 `[INT]` items — the dispatcher
  `Ready→Running`-vs-atomic-commit relationship and the in-flight residual-set
  semantics — stay **open**, pending T01 integrator acceptance, with no bound
  frozen; **H6:** the batch-failure recovery direction is accepted in principle
  (rev 32), while the terminal diagnostic-capacity/no-`Running` **implementation**
  is **BLOCKED** (rev 23; implementation BLOCKED pending T01); the own-batch
  `DraftRef` vs the guardrail "cursors/joins
  never point at a draft" conflict is an authority-reconciliation request that
  **rev 30 (user decision A, 2026-10-04) resolves in principle** as the narrow
  interpretation (transient wire/proposal input only; guardrail unamended; T01
  implementation-confirmation pending); "accepted" wording is softened to
  selected/direction;
  the dispatcher is the sole `wires.selected` writer; `dispatch_cursor` is set per
  worker invocation; VF04's batch audit is proposed-not-existing; T13 is added to
  the pending-amendment list (T13 itself is **not** edited).
  **Rev 38 (CDR rev 51 / ADR-0002 Revision 17 / T02 rev 31; §23.1):** the
  current candidate limit inventory is narrowed by the rev-51 **delegated
  candidate default** — `max_inflight_per_tick`/quota is the **sole per-tick
  dispatch-count bound**; the redundant `max_dispatches_per_tick` is dropped from
  the candidate limit inventory/validation (**not** an owner/T01 `[INT]` signoff,
  **not** a freeze). The H9 mutation boundary, the `in_flight` clear
  ownership/order relative to the H6 recovery, and the all-paths no-`Running`
  residual proof remain **open**; the H6 mechanism is unchanged.
- **Global:** the CDR dashboard records A/B/C as in-principle only and D–I
  pending (row H wording corrected to "marker-based trigger direction selected;
  marker family/schema unresolved"); exact hash/inventory claims are prospective
  only; T13 is a pending amendment; `M1AppendSchema` rows mark proposed vs
  unresolved distinctly.
  **[Rev-49 note: the row-H marker wording above is superseded for the operative
  direction by the rev-49 `FunctionEnd`-terminal-result reuse (no new marker
  family); history preserved. See §18.8.]**

What revision 23 integrates (summary; the point-by-point ledger is §18):

- **H1 (user-accepted in principle, 2026-10-04):** the literal-handoff
  **semantic-information preservation** and the **allocation split** are now both
  user-accepted in principle: per-literal `LiteralRecord` (lexical facts + `LX08`
  type), sem-stage `ConstantRequest` (`node`/`required_kind`), `ConstantResult`
  (`legality`). This is a **revision working basis, not a freeze**; the exact
  schema/variants and T03/T04/T08/T01 co-freeze/sign-off remain pending (§5, §8,
  §13, CDR §C2). Do not read it as frozen.
- **H2 (doc/blocked):** resume reinserts into `stage_queues[stage_of(parent.kind)]`
  (not `parent.continuation.production`) and the realization must handle an absent
  `continuation` (§5, §7, §8, CDR §D3).
- **H3 (doc/blocked):** the File-scope `Enter` is a **T06 task after the T05
  `TranslationUnit` is committed**, pinning a committed `NodeId`; the bootstrap
  order is a T06/`[INT]` `/6` decision, **not** a job-bootstrap action (§5,
  CDR §E2).
- **H4 (doc/blocked):** commit-apply join — all committed children `Completed` →
  parent `Ready`; any committed child `Failed` → parent `Failed` exactly once,
  **never** `Ready`; the sibling policy is **await-all** (accepted, CDR rev 42); the fate of
  non-terminal siblings is an open co-freeze item (the earlier fail-fast/sibling-cancellation framing is stale).
- **H5 (doc):** the same-batch `TokenDraft` node-link branch is removed; node
  `first_token`/`last_token` are **committed `TokenId`s only** (§5, §7, CDR §13).
- **H6 (direction accepted in principle; implementation BLOCKED):** clearing the
  in-flight set does not clear `Running`; the user selected the batch-failure
  recovery direction in principle (rev 32: fail every dispatched task exactly once
  in dispatch order with the optional-diagnostic/`DiagnosticId::NONE` sentinel), but
  the whole no-`Running`/terminal diagnostic **implementation** is **BLOCKED**
  pending T01 integrator approval and only the invariant is stated (§7, T02,
  ADR-0002 §6, CDR §A).
  **[Rev-42 note (user-accepted recommendation; T01 co-freeze pending; not a `/6`
  freeze; no code): the "implementation BLOCKED" framing above is qualified by
  the rev-42 selected quota>1-capable bounded recovery mechanism to be frozen in
  `/6` (H6 wording promoted to the rev-42 formula everywhere operative: pre-
  dispatch errors leave tasks `Ready`; failed batch commits no semantic state;
  dispatch-order `Failed` recovery; no N pre-reservation; `NONE` fallback; state
  guard); exact atomic realization + fixtures remain open.]**
- **H7 (doc):** `folded_int5_part_a_producer`/`ir.folded-int5-part-a-producer` is
  renamed to `folded_int5_ir_constant`/`ir.folded-int5-constant-emission`, with the
  T08-computes-`ConstRecord`-value / T09-emits-IR-`Constant` split (§5, §13, CDR §H).
- **H8 (blocked/inventory):** all present-tense "frozen/hashed/pinned" IR terms are
  made **prospective `/6`**; both `ir.op-immediate-type` aliases and both
  `ir.terminator-missing*` candidates are inventoried as **unresolved**; the
  dangling `§3.6` reference is corrected to `§3 item 6` (§5, §12.12, §13, CDR §H).
- **H9 (removal direction accepted in principle; remaining `[INT]` items open):**
  **current status (rev 33, 2026-10-04):** the user accepts **removing
  `max_inflight_total`** from the `/6` candidate in principle — sequential per-tick
  dispatch is already bounded by `max_inflight_per_tick` (the dispatch batch
  bound), the per-stage `stage_queue_bound`, and `max_tasks_total`; `TaskState::Waiting`
  is not in-flight; and `tasks.in_flight` is an **ephemeral per-tick scheduler
  batch only** cleared at latch only after every dispatched task has a
  terminal/`Waiting`/`Progress` outcome (or the H6 recovery); the dispatcher's
  **pre-worker `Ready→Running` mutation is distinguished from** the one ordered
  atomic semantic commit. This is a `/6` **working-basis direction only, pending
  T01 integrator acceptance**: no bound/config/hash is frozen, `InflightQuotaExceeded`
  is removed from the candidate, and the remaining H9 `[INT]` items — the
  dispatcher `Ready→Running`-vs-ordered-atomic-commit relationship and the
  residual-set semantics of the in-flight set — stay **open** (§6.2.1, §7, T02,
  ADR-0002 §6, CDR §A). **Historical rev-23 framing (superseded):** the rev-23
  entry recorded the single phase/owner for `max_inflight_total`, the residual-set
  semantics, and the dispatcher mutation-vs-commit relationship as **undefined
  `/6` decisions, not resolved**; that one-phase-owner framing is superseded by the
  rev-33 removal direction above (the residual/commit-relationship items remain open
  as stated). The historical record is retained here, not overwritten.
- **H10 (doc):** stale rev-21 references are corrected to rev 21–26 (later
  extended to rev 21–28 in rev 28 and to rev 21–29 in rev 29) (CDR §11/§12/
  §13, ADR-0002 §1.1/§8, this proposal) with T13 recorded as **not edited**;
  `AwaitChildren` is added to §2.4 and to the §3 one-transition list.
- **H11 (doc):** "trigger fixed" becomes "**marker-based trigger direction
  selected; marker family/schema unresolved**" (CDR row H/§H1, M1 target §3.2,
  this proposal §5/§16).
  **[Rev-49 note: the marker-family wording above is superseded for the operative
  direction by the rev-49 `FunctionEnd`-terminal-result reuse (no new marker
  family); history preserved. See §18.8.]**

What revision 21 corrects (summary; details inline):

- **Single owner per field; no shared-writer carveout (user, in principle).**
  `sources.spans` is **T03-only**; T04 `TokenRecord.span` is a reference to the
  committed T03 PP span (T04 writes no spans); AST `NodeRecord` carries a
  first/last `TokenId` range (T05 writes no `sources.spans`). The rev-20 C6
  shared T03+T04 writer and the pending T05 span writer are removed (§5, §6.1,
  §7, §8, §15).
- **Committed T04-owned `LiteralRecord` handoff (user, in principle).** Preserve
  `node`/`required_kind`/`legality`/`LX08` type as explicit co-freeze fields;
  T08 remains the sole `constants.records` writer (§5, §8, §C).
- **Deterministic bounded sequential stage pipeline (user, in principle).** The
  scheduler is a staged pipeline with one ordered atomic commit; `quota = 1`
  (semantic projection) is the baseline and `quota > 1` requires measurement +
  integrator acceptance (§3.12, §6.2.1, §10.5, §11, §12.18).
- **T05 (`awaited` removed; refs validated pre-apply; join in commit apply).**
  `ContinuationRecord.awaited` is deleted in favor of `WaitSet`-only;
  `ContinuationRef`/`ChildRef` are validated in the no-mutation pass before apply;
  `Task.continuation` direction is fixed; the join reinserts into the task's own
  `stage_queues` in commit apply (§5, §6.1, §6.3, §7, §13).
- **T06 (`Identifier`-leaf `decl`, Block boundary `at`, symbol-error class).**
  (§5, §7, §13).
- **T07 (no T09-`FunctionRecord` cycle; `SemRecord` for `Return`/
  `FunctionDefinition`; explicit conversion ops; VF06 matrix; split const
  requests).** (§5, §6.4, §7, §13).
- **T08/T09 (`max_const_bits` formula; explicit `TerminatorMissing` trigger;
  `Constant` immediate = result type; rule id per rejection; `scope_events`
  inventory).** (§5, §6.4, §7, §12, §13).
- **T03 (`Preprocessed` not M1-produced; total `requires_map`/source
  equality/hash/tests).** (§5, §8, §13).

Earlier revisions (retained context):

- **Baseline equivalence is a canonical projection, not byte-identical
  snapshots** (audit H1). The new scheduler snapshot encodes stage queues, the
  in-flight set, `dispatch_cursor`, `stage_assignment_version`, and
  `PipelineMetrics`; it therefore **cannot** be byte-identical to the rev-18
  single-active-task snapshot. The quota = 1 gate is restated as a **canonical
  semantic comparison projection** (semantic records/results/diagnostics plus
  the semantic trace, *excluding* scheduler-only registers); the new scheduler
  snapshot must separately be deterministic and replay-identical (§10.5, §12.18;
  ADR-0002 §2.8; `M1-REC-03`/`D8`/`G15`).
- **Batch dispatch set fixed** (audit H2): `dispatched_tasks` is the ordered
  in-flight set at quota `> 1`, or the singleton `tasks.active` view at quota 1,
  everywhere in §7 — not only in the algorithm header.
- **A real T05 cross-tick handoff protocol** (finding 3): `ContinuationRef`
  (own-batch continuation draft or committed `ContinuationId`),
  `Proposal::AwaitChildren`, the task/parent state machine, resume, and the
  continuation chain are specified. The earlier "next-tick committed IDs" prose
  is replaced; residual owner sign-off remains.
- **`ArtifactKind` total variant set** (finding 4): the checked-in
  `{Preprocessed, Assembly, Object, Snapshot, Trace}` and the added
  `{Normalized, Spliced, CommentFree}` are resolved with a **total**
  `requires_map`; `Preprocessed` (PP28 output) vs. `Normalized` (PP01 buffer) is
  explicit.
- **CL02/CL03 ids disambiguated** (finding 5): fixture `M1-CL-02` is executed by
  chip `CL03`; chip `CL02` (`ConstantUnaryChip`) is unexercised. The Part A
  symbolic `CL03` operation and the probe-gated target-width projection are
  separated with explicit owners; T08 owns `ConstRecord.value`, T09 owns IR
  `Constant` emission.
- **ADR-0002 contradictions removed** (finding 6); **`Selection`/batch/report
  shapes chosen and absent `report.rs` types marked proposed** (finding 7);
  **`Progress` canonical queue source** (finding 8); and **an exact same-tick
  cross-task write-conflict predicate** (finding 9) are integrated consistently
  across this document, T02, and ADR-0002.
- **Error classification** (cross-cutting): every failure is classified as a
  `CommitError`, a **chip diagnostic** (e.g. `ConstOverflow`/`ConstUnsupported`/
  `ParseCursorDidNotAdvance`/`UnsupportedNode`), or a `ManifestError`
  (`StoreOwnerViolation`, stage assignment), with a precise inventory of what is
  **proposed** to be hashed at `/6` versus not (§6.4, §12.12).

Current authority and state (re-read 2026-10-04, checked-in tree):

- T01 foundation is `t01-c01-c06/5`, hash
  `61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5`
  (`compiler/contracts/CONTRACT_VERSION`, `compiler/src/contract.rs`), verified
  by `compiler/tests/freeze.rs`.
- The `/5` artifact is **foundation only**: C01–C06 envelope, target/config
  schema, manifest lint, snapshot/trace, and a routing shell. There are **no C
  language chips**, no worker handlers, no parser, no code generation, and no
  compiler. This proposal does not claim otherwise.
- Target **identity** is frozen (`aarch64-unknown-linux-gnu`, ELF, LP64,
  little-endian, AAPCS64). Every concrete scalar size/alignment, `long double`
  format, `wchar_t` signedness, and ABI register/save-area value remains
  **UNVERIFIED**. The Linux probe substrate is planned and not provisioned; no
  probe has run; no pass rate and no >99% claim is made or implied.
- The M1 source is a hand-written fixture, not a GCC torture case. M1 Part A is
  target-independent frontend + IR; M1 Part B (AArch64 assembly/ELF/run) remains
  blocked by design until the probe verifies, and stays out of this proposal.
- **Hash scope (rev 37; user, CDR rev 50).** The `/5` frozen hash stays current.
  For `/6`, the user accepts the **two-tier model**: `StoreSchema::foundation +
  M1AppendSchema` is the **frozen `/6` seed participating in the `/6` contract
  hash**, while **post-seed runtime `StoreSchema::declare()` extensions stay
  hash-excluded** and are captured/validated through runtime snapshot/schema
  mechanisms. This settles the **conceptual** boundary only: no exact
  `M1AppendSchema` contents/counts are accepted, nothing is frozen, no `/5` change
  or code is authorized, and the numeric inventory/freeze test remain pending T01
  co-freeze after the owners (§22.5). The `/5` code facts (rev 48 audit) are
  **24** `RECORD_KINDS` / **24** `RecordRef` variants (tags 0–23), `RecordFamily`
  **absent**; the draft 19/27/27 counts remain **proposed/unverified**.
  **Rev 38:** at this snapshot the [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)
  is at **rev 51**, [ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md)
  at **Revision 17**, and [T02](T02_CONTROL_CHIPS.md) at **rev 31**; the rev-50
  two-tier scope and the rev-49 H11/T09/T01 direction remain current, and the
  rev-51 delegated candidate default (sole `max_inflight_per_tick`/quota bound)
  freezes nothing (see §23). **Pointer correction (same rev 38):** the CDR has since
  advanced to **rev 52**, which **points at this proposal rev 38**; the **rev 51**
  above is the CDR state at the rev-38 integration snapshot, not the current CDR
  revision.

Precedence follows `AGENTS.md` §2: explicit user instruction, accepted ADR,
accepted engineering contract, architecture design, then implementation. This
draft is below all of those and is superseded by any accepted T01 freeze.

Related documents: [T01](T01_COMPILER_CONTRACT.md),
[docs/tasks/README.md](README.md), [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md),
[TASK_TEMPLATE.md](TASK_TEMPLATE.md), [T02](T02_CONTROL_CHIPS.md)–[T13](T13_VERIFICATION_CHIPS.md),
[M1 vertical-slice acceptance (frontend half)](M1_VERTICAL_SLICE_ACCEPTANCE.md),
[M1 target acceptance](M1_TARGET_ACCEPTANCE.md),
[ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md),
[SILICON_PARADIGM_SPEC.md](../architecture/SILICON_PARADIGM_SPEC.md),
[SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md),
[compiler/README.md](../../compiler/README.md),
[COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md).

---

## 1. Purpose and the gap this proposal closes

`/5` states the rule but has no mechanism for it:

- T01 §4: *"writing a store/generating a record" means generating an append
  proposal submitted by the commit chip; a worker must not bypass commit to
  directly modify a shared arena.*
- T01 §7.1 C01: language stores `pp/lex/parse/symbols/types/nodes/consts/…` are
  `ReservedArena` placeholders.
- T01 §7.1 C03/C05: patch and result payloads carry only `RecordRef`s; the
  mechanical commit does not materialize record bodies; reserved-store bodies
  are not encoded.

Consequently there is today **no path** by which a stateless restricted chip can
cause a typed C-frontend record to exist. The installed `ReservedArena` can
allocate an ID but cannot store a body. `StorePatch.value: Option<RecordRef>`
and every `ResultValue` variant reference *already-materialized* records only.
`InternTable::intern` (`compiler/src/intern.rs`) and `CompilerBus::alloc_source`
(`compiler/src/bus.rs`) are bus methods no `RestrictedChip` may call.

This proposal defines the smallest deterministic, all-or-nothing protocol that
lets an M1 chip propose typed PP/token/AST/scope/symbol/type/sem/const/IR,
span/expansion, and name records without touching the bus and without allocating
IDs itself. It covers **M1 Part A only**.

**Non-goals (explicitly excluded):** M1 Part B (AArch64 assembly, ELF,
assembler, linker, runner); Layout, Init, Machine, VReg, Opt, Ext record stores;
optimization; GNU/builtin handling; concurrency. Target ABI **numbers** are a
non-goal: no Part A record may embed an unverified width (§5/O1).

---

## 2. Discrepancies to resolve before this can freeze

### 2.1 `sem` partition has no ID, no arena, no record family

T01 §2 declares a `sem` partition, but T01 §3's required-ID list omits `SemId`,
and `compiler/src/ids.rs`, `RECORD_KINDS`, `snapshot.rs`, and `bus.rs` have no
`sem` entry. `StoreId::Sem` exists in `task.rs` but has no arena. See §5/§12,
including `RecordRef::Sem`.

### 2.2 Task README code layout vs actual flat layout

`docs/tasks/README.md` §6 sketches subdirectories; the T01 status below it
records the actual flat layout (`compiler/src/{arena,ids,task,bus,commit,...}.rs`).
This proposal assumes the flat layout and proposes group-owned record-body files
under `compiler/src/records/<group>.rs` (`RESIDUAL`: exact path).

### 2.3 `RestrictedChip`/`ChipAdapter` model vs the `RoutingShell` shell

T01 §4.1 requires new chips to be zero-field `RestrictedChip`s committed by an
application `ChipAdapter`. `RoutingShell::propagate` (`compiler/src/routing.rs`)
instead fuses selection, direct state mutation, and `commit_proposals`. §10
resolves this.

### 2.4 Documented envelope vs proposal vocabulary

T01 §4 lists `Proposal = Enqueue | Complete | Fail | AwaitHost | StorePatch`.
This proposal adds `AppendRecords`, `Progress`, **and `AwaitChildren`**, and result
variant `DraftRecords`. That is a normative change (§12). (`AwaitChildren` is the
join/await-children proposal; §6.3 lists the full transition set.)

### 2.5 Frozen-hash coverage vs the AGENTS freeze rule

`contract.rs::FrozenSchema::encode` fingerprints `RECORD_KINDS` **labels**, the
foundation task/result enums, rule identifiers, target profile, limits, and the
**foundation** `StoreSchema` only. Group-declared store fields and record-body
field shapes are hash-excluded (README/`contract.rs` header; snapshot-covered
instead). `AGENTS.md` §5 item 2 nevertheless requires shared AST/type/IR models to be
frozen before parallel implementation, and `COMPILER_SFL_MANIFEST.md` §4
currently claims "adding fields or rules changes the frozen contract hash" — a
direct contradiction with `contract.rs`/README. §12 proposes a concrete `/6`
mechanism (an `M1AppendSchema` section proposed to be hashed at `/6`) and records the required document
reconciliation. **Rev 37 (user, CDR rev 50; `§22.5`):** the **conceptual** scope of
this discrepancy is now **settled** by the accepted **two-tier model** —
`StoreSchema::foundation + M1AppendSchema` is the frozen `/6` seed that
participates in the `/6` contract hash, while post-seed runtime
`StoreSchema::declare()` extensions stay hash-excluded (captured/validated via
runtime snapshot/schema mechanisms). The rev-48 audit recorded the conflict as
**real and unresolved** as it existed then; the rev-50 decision selects the
two-tier option at the conceptual level but **freezes no value** and leaves the
exact `M1AppendSchema` contents/counts, dual-inventory encoding, numeric-value
inclusion, and the freeze/self-consistency test as pending T01 `/6` co-freeze.

### 2.6 Materialization of foundation families has no store path `[audit2 N1]`

`SpanRecord`/`ExpansionRecord` are real `TypedArena`s in `bus.rs` but have no
`StoreId`; the `InternTable` is not a store at all. The `/5` manifest
write-authorization model is keyed by `StoreId`/field, so spans, expansions, and
names cannot be authorized or audited today. §8 resolves this without inventing
per-family stores unnecessarily.

---

## 3. Proposed decisions (integrator-selected defaults; item 12 is user-binding direction)

Items 1–11 are integrator-selected draft defaults (not owner-confirmed). Item 12
is the user's **binding** pipeline-scheduling direction and is cross-referenced by
[ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md); it still
requires the `/6` freeze before implementation.

1. **Closed typed `RecordDraft` enum + `AppendRecords { task, batch }`.** No
   type erasure, no `Any`, no `dyn`. No chip or adapter allocates arena IDs.
2. **Closed typed link enumeration; central pre-mutation validation.**
   `RecordLink` carries an expected `RecordFamily` and a `LinkTarget`; the
   envelope validates range, family, committed existence, and source/TU
   ownership for every link **before the first mutation**. Cross-task **draft**
   references are never allowed; cross-task **committed** references are the
   normal data flow.
3. **Two-phase atomic commit.** Validation and reference resolution are fully
   performed in a **no-mutation pass** that constructs final typed records;
   capacity preflight follows; apply performs only infallible pushes and state
   transitions.
4. **Fully typed apply via closed exhaustive dispatch.** Heterogeneous
   `TypedArena<I,T>` fields are pushed by an exhaustive `match` over
   `ResolvedDraft`; no `Result`, no trait object, no `arena(store)` accessor.
5. **Explicit one-transition invariant.** Every dispatched task must produce
   exactly one of `Complete`/`Fail`/`AwaitHost`/`AwaitChildren`/`Progress` each
   tick; no task may remain `Running`. (The invariant is stated; the *mechanism*
   guaranteeing it on a terminal capacity-failure path is BLOCKED — §18.4/H6. The
   user **selected the recovery direction in principle** on 2026-10-04: fail every
   dispatched task exactly once in dispatch order with the same
   optional-diagnostic/`DiagnosticId::NONE` sentinel semantics, so every task
   leaves `Running`; the exact atomic/bounded implementation still requires T01
   integrator approval (§3 item 12, §7, §10.5, §18.4). **H9 (rev 33):** the user
   also accepts in principle **removing `max_inflight_total`**; the per-tick
   `max_inflight_per_tick` dispatch bound, the stage queues, and `max_tasks_total`
   already bound work, `Waiting` is not in-flight, and `tasks.in_flight` is an
   **ephemeral per-tick scheduler batch only** that clears at latch after every
   dispatched task has a terminal/`Waiting`/`Progress` outcome or H6 recovery —
   pending T01 integrator acceptance, no frozen bound.)
   **Rev 37 (rev-42/rev-49; §22.1/§22.4):** the H6 **quota>1-capable bounded
   recovery mechanism is now selected to be frozen in `/6`** (not merely a
   direction): pre-dispatch errors before state mutation leave tasks `Ready`, a
   failed semantic batch commit commits no semantic state, a deterministic bounded
   recovery mutation processes the dispatched tasks once in dispatch order to
   `Failed` with no pre-reservation of N diagnostics and a per-task
   `DiagnosticId::NONE` fallback, and a state guard prevents duplicate transition.
   It is **not implemented or frozen**; the exact atomic-commit realization requires
   T01 integrator approval and the H9 numeric inventories plus T02/T13 fixtures/
   sign-offs remain pending.
   `[audit2 N3; rev 23 H10; rev 32 H6 direction; rev 33 H9 removal direction; rev 42 H6 mechanism; rev 49 H9 audit blockers]`
   **Rev 38 (rev-51; §23.1):** the H9 candidate limit inventory is narrowed by a
   **delegated candidate default** — `max_inflight_per_tick`/quota is the **sole
   per-tick dispatch-count bound** and the redundant `max_dispatches_per_tick`
   (with its `DispatchBudgetExceeded` failure) is **dropped from the candidate**;
   **not** an owner/T01 `[INT]` signoff, **not** a freeze; the H9 mutation
   boundary / `in_flight` clear order / residual no-`Running` proof stay **open**.
6. **Typed migration limited to M1 Part A stores.** Exactly **19 draft
   families** = 18 arena-backed (Span, Expansion, PpToken, Token, Literal, Node,
   Scope, Symbol, Type, Sem (`SemId`), Const, Function, Block, Value, Instruction,
   ScopeEvent (`ScopeEventId`), Artifact, Continuation) + Name (interned,
   `InternTable`-backed). No Layout/Init/Machine/VReg/Ext.
7. **Root compiler chips are `RestrictedChip`s; an application-owned
   driver/backend owns selection application, invocation, commit, and
   reporting.** `[audit2 N6]`
8. **Explicit bounded multi-tick `Progress`.** `[audit2 N5]`
9. **Symbolic Part A types/consts; records carry no unverified width.**
10. **One `/6` freeze of the full M1 family list, append schema, and proposals.**
11. **Names are an intern store (idempotent, `InternTable`-backed), not an
    arena; spans/expansions are NEW append fields on the existing `sources`
    store, both owned by T03.** `sources.spans` has the single owner **T03**;
    T04 `TokenRecord.span` reuses the committed T03 PP span and writes no spans;
    AST `NodeRecord` carries a first/last `TokenId` range and T05 writes no
    `sources.spans`. `[audit2 N1, audit3 AD2; user in-principle decision
    2026-10-04]`
12. **Staged pipelines with committed-ID cursors/joins (user binding direction,
    2026-10-04).** Cross-tick work is an explicit **pipeline of stage tasks**,
    not an ad-hoc continuation: every task kind is assigned to exactly one
    `StageId`; each stage owns a bounded ready queue; independent stage tasks may
    overlap across ticks; the dispatcher runs a bounded ordered
    `SelectionBatch` under a configured `max_inflight_per_tick` quota, executes
    the batch **sequentially** on the single CPU backend, and performs **one
    ordered atomic commit** per tick over committed IDs. Cursors, stage
    queues, awaited child sets, and join points live in the bus as committed IDs
    or **own-task-batch draft keys**; the canonical
    ready-queue source is `stage_queues` (`tasks.ready` is a derived quota-1
    view). **Accepted in principle (user, 2026-10-04; user decision A; narrow
    interpretation):** the own-task `OwnBatch` draft key is the user's **narrow
    interpretation** of
    [Guardrails](COMPILER_DEVELOPMENT_GUARDRAILS.md) §6.1. A same-task
    `OwnBatch(DraftRef)` may exist **only as a transient wire/proposal input
    before commit**; commit validates and resolves it to a committed
    `TaskId`/record **before any persistent state**, and **no durable
    cursor/`WaitSet`/join may point at a draft, wire, or address**. The guardrail
    text is **unamended**; **T01 integrator implementation-confirmation remains
    pending** (M1 proposal §17.7 PIPE-5/§19.4, T02, ADR-0002 §2.1 invariant 6).
    This is a revision working basis, **not a freeze**.
    `quota = 1` is the **baseline equivalence mode only** and is compared
    by the **canonical semantic projection** of §10.5 (not a byte-identical
    snapshot); the pipeline is designed for a measured bounded quota `> 1` over
    independent stages, and no throughput claim is made until measured.
    Same-tick writes to one `(StoreId, field, record)` conflict unless the field
    is declared append-only and the appends are semantically independent; a
    conflict rejects the whole batch before apply (§7 `CrossTaskWriteConflict`).
    **Batch-failure recovery (user direction in principle, 2026-10-04; H6):** when
    the one ordered atomic commit fails, the user selected (in principle) the
    deterministic recovery direction that fails **every** dispatched task
    **exactly once** in dispatch order — a committed `DiagnosticId` when
    diagnostic/record capacity allows, else the
    `TaskState::Failed(DiagnosticId::NONE)` sentinel — so **every** dispatched task
    leaves `Running`; clearing the in-flight set is **not itself** a transition.
    This generalizes the verified `/5` `fail_selected` single-task sentinel
    obligation (unchanged). Exact atomic/bounded implementation is a **T01
    integrator** decision and the diagnostic budget/state mechanism plus T02/T13
    owner fixtures/sign-offs remain **co-freeze and pending** (§7, §10.5, §18.4).
    **Rev 37 (rev-42/rev-49; §22.1/§22.4):** the mechanism is now **selected for
    `/6` freeze** — pre-dispatch errors before state mutation leave tasks `Ready`,
    a failed semantic batch commit commits no semantic state, a deterministic
    bounded recovery mutation processes the dispatched tasks **once in dispatch
    order** to `Failed`, **no pre-reservation of N diagnostics**, a per-task
    diagnostic attempt with the `DiagnosticId::NONE` fallback, and a state guard
    against duplicate transition; **not implemented or frozen**, and the H9 audit
    blockers (quota>1 pipeline unimplemented; `Ready→Running`/`in_flight`-vs-commit
    relationship, `in_flight` clear ownership/order, all-paths no-`Running` proof
    open; `max_dispatches_per_tick`-vs-`max_inflight_per_tick` conflict deferred to
    `/6`) remain open.
    **In-flight total bound removal (user direction in principle, 2026-10-04;
    H9):** the user accepts in principle **removing `max_inflight_total`** from
    the `/6` candidate: sequential per-tick dispatch is already bounded by
    `max_inflight_per_tick` (the dispatch batch bound), the per-stage
    `stage_queue_bound`, and `max_tasks_total`; `TaskState::Waiting` tasks are
    **not** in-flight; and any future cross-tick `Running` mode needs a **separate
    CDR**. `tasks.in_flight` therefore remains an **ephemeral per-tick scheduler
    batch only** (candidate), cleared at latch only after every dispatched task
    has a terminal/`Waiting`/`Progress` outcome (or H6 recovery). The dispatcher's
    pre-worker `Ready → Running` mutation is distinct from the one ordered atomic
    semantic commit. This is a `/6` **working-basis direction only, pending T01
    integrator acceptance**; no bound/config/hash is frozen, and the residual
    H9 `[INT]` items (the `Ready→Running`-vs-commit relationship and residual-set
    semantics) stay open (§18.6).
    **Rev 38 (rev-51; §23.1):** the candidate limit inventory is narrowed by the
    rev-51 **delegated candidate default** — `max_inflight_per_tick`/quota is the
    **sole per-tick dispatch-count bound** and the redundant
    `max_dispatches_per_tick` (with its `DispatchBudgetExceeded` failure) is
    **dropped from the candidate**; **not** an owner/T01 `[INT]` signoff and
    **not** a freeze, and the H9 mutation boundary / `in_flight` clear order /
    residual no-`Running` proof remain **open**.
    `[T05, T07, user pipeline direction; ADR-0002]`

---

## 4. M1 Part A fixture, mode, and config

Canonical source (`M1-SRC-000`), LF-terminated, no BOM, 28 bytes:

```text
int main(void){return 2+3;}\n
```

0-based half-open spans (from [M1 vertical-slice acceptance (frontend half)](M1_VERTICAL_SLICE_ACCEPTANCE.md) §3):

```text
0..3 int | 3 sp | 4..8 main | 8..9 ( | 9..13 void | 13..14 ) | 14..15 {
15..21 return | 21 sp | 22..23 2 | 23..24 + | 24..25 3 | 25..26 ;
26..27 } | 27..28 \n
```

Boundary variants: `M1-SRC-001` (no final LF, 27), `M1-SRC-002` (CRLF, 29),
`M1-SRC-003` (spaced, 33), `M1-SRC-004` (`2+ +3`, 30).

Config: an **explicit** dialect and options, e.g. `-std=c11 -O0`, passed through
`CompilerConfig::try_new` (external fixtures cannot use the now-`pub(crate)`
`new`); no fixture may rely on `CompilerConfig::default()`. `[audit11]` The
`TargetSpec` stays `aarch64_unknown_linux_gnu_unverified()`; its numeric fields
are read but never asserted in Part A. `ensure_codegen_ready()` must keep failing
closed. Part A records are defined symbolically so this holds structurally, not
only by test discipline. `M1-SE-01` ("type `int`, non-lvalue") is
target-independent under symbolic types and is in scope at `T01 freeze`; the
bit-pattern fixtures (`M1-LX-04/05`, `M1-TY-01`, `M1-CL-02`, `M1-NEG-07/18`)
remain `T01 + target probe`, while `M1-TY-07` (symbolic `int` promotion, no
concrete width) is Part A at `T01 freeze`. `[audit2 N13]`

Semantic invariants the records must express (from
[M1 target acceptance](M1_TARGET_ACCEPTANCE.md) §3.1–§3.2): one TU, one function
`main` of type `int (void)`, exactly one entry block with exactly one return
terminator, return operand type `int`, result the folded `int 5` (the closed op
table may retain `Add` for verifier/future but the M1 fixture does not emit it),
no memory/call/volatile/atomic/float/aggregate operations.

---

## 5. Proposed M1 Part A IDs and typed final records

Reused IDs: `SourceId, SpanId, ExpansionId, PpTokenId, TokenId, NameId,
ScopeId, SymbolId, TypeId, NodeId, ConstId, FunctionId, BlockId, ValueId,
InstructionId, ContinuationId, ArtifactId, TaskId, ResultId, DiagnosticId, ChipId`.

New IDs (proposed; append-only wire tags): `SemId` (`RecordRef::Sem`),
`ScopeEventId` (`RecordRef::ScopeEvent`), and `LiteralId`
(`RecordRef::Literal`, T04-owned decoded literal). With `ArtifactId` already at
wire tag 23, `RecordRef` gains `Sem` (**wire tag 24**), `ScopeEvent` (**wire tag
25**), and `Literal` (**wire tag 26**), preserving wire tags 0–23;
`Continuation`/`Artifact` keep their existing wire tags (`Artifact` = tag 23).
This is the sole authority for the **wire-tag** inventory; the §12.1/§12.17 and
§6.2 lists are restatements and must match it exactly. **`RecordFamily` ordinals
are a distinct closed-enum order and are NOT claimed to equal
the `RecordRef` wire tags** (finding 7, T05); `M1AppendSchema` is **proposed to
pin** both inventories independently at `/6` (not pinned today — rev 26). Parse state reuses
`ContinuationId`/`RecordRef::Continuation` (no new ID/tag). Stage scheduling adds
**no** `RecordRef` family: `StageId`, stage queues, cursors, and joins are
scheduling registers keyed by committed `TaskId`/`ContinuationId`/`ResultId`
(§10.5).

> **Reviewer-driven candidate shapes (illustrative), not frozen and not
> owner-approved.** These incorporate the T03/T04/T05/T06/T07/T08/T09
> revision-18/19 reviewer findings plus the rev-20 re-review findings (audit
> H1–H2, findings 3–9) as an integrated candidate for a second owner sign-off;
> each shape still needs the named owner's sign-off (§15.3) and is not an
> implementation claim. No approval is implied.

```rust
// PROPOSAL ONLY — none of this exists today. Illustrative, not frozen.

pub struct SpanRecord {                 // TypedArena<SpanId, _> already in bus.rs
    pub source: SourceId,               // raw physical SourceRecord.bytes
    pub start: u64, pub end: u64,       // half-open raw-byte offsets (u64; <= max_source_bytes)
    pub expansion: Option<ExpansionId>,
}
// **Rev 22 delta note (T03/T04 finding 5):** the checked-in `/5` `SpanRecord`
// offsets are narrower (`u32`); this proposal widens them to `u64`
// (`<= max_source_bytes`). The CDR §3.2 delta now records that widening and the
// `ArtifactRecord` shape change explicitly. `sources.span_root` is the existing
// source-root span field (T01 foundation) and grants no general span write.
pub struct ExpansionRecord {            // TypedArena<ExpansionId, _> already in bus.rs
    pub parent: Option<ExpansionId>, pub spelling: SpanId, pub expanded: SpanId,
    pub ordinal: u32,
}
// `ExpansionRecord` carries **no** own `source` field (T03). Its source is
// derived: commit resolves `spelling` and `expanded` each **one hop** to a live
// committed `SpanRecord` and requires both spans' `source` to equal the task's
// declared `SourceId` (`SpanSourceMismatch`); the parent chain is dereferenced
// one level only, never recursively walked. Same-batch `SpanDraft` targets are
// resolved through this task's own batch (candidate-only; the own-task-batch key
// applies the user's in-principle narrow interpretation of Guardrails §6.1 —
// §17.7 PIPE-5; T01 implementation-confirmation pending). A committed
// `Spelling`/`Expanded` link that is a `Span` is checked directly; a non-`Span`
// family is `CommittedFamilyMismatch`. §13 test
// `expansion_spelling_expanded_one_hop_source`.
// NameRecord deleted (T03 R1): interning identity is NameId; bytes live in
// InternTable. T04 owns names.entries; T03/T06 read only, no writes.

pub enum PpTokenKind { Identifier, PpNumber, CharConstant, StringLiteral, Punctuator, HeaderName, Eof }
pub struct PpTokenRecord {              // T03 candidate (second sign-off pending)
    pub kind: PpTokenKind,
    pub span: SpanId,
    pub spelling: Option<Vec<u8>>,      // None only for Eof; append order authoritative
}
// Exactly one Eof, zero-length at raw logical/source end; no head/ordinal.

pub struct TokenRecord {
    pub kind: TokenKind, pub span: SpanId, pub name: Option<NameId>,
    pub literal: Option<LiteralId>, pub flags: TokenFlags,
}
// `span` is a **committed T03-owned PP `SpanId`** (the span of the source PP
// token this C token derives from). T04 writes **no** `SpanRecord` and never
// appends to `sources.spans`; it reuses the committed span. The user selected
// this single-owner model on 2026-10-04 (§8, §15.1).
// **Rev 22 limitation (T03/T04 finding 1):** a single committed PP span is exact
// only when one C token is derived one-to-one from one PP token (the M1
// single-file fixture). It is **incomplete** for `LX14 AdjacentStringChip`
// (a merged token whose provenance is a *sequence* of PP token spans) and
// `LX16 TokenLocationChip` (synthesized/pasted/directive tokens whose location
// is a computed macro origin, not an existing PP span). M1 has neither adjacent
// string concatenation nor macro/paste, so the fixture is unaffected; the
// general contract needs a typed T04 provenance carrier (e.g. a committed
// token-origin record with a `Vec<SpanId>`/synthetic-origin tag) or a T04-owned
// span field. This is a **T03/T04 `/6` blocker**; no carrier is invented here.
// `literal` references a **committed T04-owned `LiteralRecord`**, not a
// `ConstId` (T08 owns `constants.records`). This is the representable T04→T08
// handoff: T08 reads `RecordRef::Literal` (a committed record) and commits the
// `ConstRecord`, so `Payload` stays `RecordRef`-only and T08 remains the single
// `constants.records` writer. See `LiteralRecord` below and §8.

/// T04-owned decoded literal (raw lexical fact; **not** a constant value).
pub struct LiteralRecord {              // Arenas.literals (new); (Lex, "literals")
    pub token: Option<TokenId>,        // back-link when formed from a token
    pub kind: LiteralKind,             // Integer / Character / String (M1: Integer only)
    pub radix: u8,                     // 10/16/8/2
    pub suffix: LiteralSuffix,         // candidate suffix surface
    pub value: Vec<u8>,                // big-endian magnitude bytes; no host width
    pub negative: bool,                // explicit sign only; canonical sign invariant
    pub spelling: Vec<u8>,             // exact source spelling
    // **PROPOSED, UNFROZEN (rev 25 F3; H1 allocation accepted in principle
    // 2026-10-04).** The `LX08` candidate type is allocated to this per-literal
    // record by the H1 split; the field name/type below is illustrative and the
    // exact enum/type encoding is a T04/T08/T01 `/6` co-freeze item, NOT a frozen
    // shape. `Lx08CandidateType` is a prospective type placeholder (the closed
    // candidate set is not enumerated here).
    pub candidate_type: Lx08CandidateType,
}
// **H1 (user-accepted in principle 2026-10-04) — this struct reflects the
// accepted allocation, not a frozen shape.** The user decision accepted the
// **semantic-information preservation requirement** (`node`, `required_kind`,
// `legality`, `LX08` type) via a committed T04-owned `LiteralRecord`, **T08 as the
// sole `constants.records` writer**, and the **recommended allocation split**
// (`接受拆分（推荐）`): `LiteralRecord` is the per-literal lexical candidate fact
// (spelling, radix, suffix, magnitude, sign, and the `LX08` candidate type), with
// `node`/`required_kind` on the sem-stage per-use `ConstantRequest` and `legality`
// on the result (`ConstantResult`). The rationale is that no AST exists at the lex
// stage. **This is a revision working basis, not a freeze and not code/chip
// authorization**; the exact field/enum/tag shapes still require T01 integrator
// acceptance plus T03/T04/T08 co-freeze/sign-off before `/6` (recorded in §18.1).
// T08 consumes the committed literal, evaluates it under
// `ConstRecord { ty, value: i128 }` and the checked-op rule, never writes
// `constants.records`, and does not re-derive the `LX08` candidate type;
// `Payload` stays `RecordRef`-only. The exact enum variants, the carrier of the
// candidate type, and the committed `legality` carrier are **T04/T08 `/6`
// co-freeze blockers**. No field/family/tag is frozen here.
// **Rev 38 (T04 rev 46; §23.3) — `token` semantics delegated candidate.** The
// meaning of the already-accepted `token: Option<TokenId>` field is clarified by
// a **delegated candidate default** (T04 package rev 46; not an owner/T04 or T01
// `[INT]` signoff, not a freeze): `token = Some(committed TokenId)` when the
// committed record came from a **single committed source C token** (M1 `2` and
// `3`), and `token = None` for an **explicitly synthesized literal with no
// single originating source C token** (a deliberate condition, not a
// "not-yet-resolved" placeholder). A draft/wire/address is **never** persisted.
// A same-batch token↔literal relation is expressed through the **generic typed
// `RecordLink` / append-reference mechanism** and resolved to durable committed
// IDs before persistence — **not** the task-level
// `ContinuationRef::OwnBatch`/`ChildRef::OwnBatch`. The exact local-link
// spelling/error handling, the synthetic-literal policy beyond M1, and the
// source-span/error-code rules remain **open T04/T01 co-freeze**.

pub enum NodeKind {
    TranslationUnit, FunctionDefinition, Declaration, TypeSpecifier, Declarator,
    ParameterList, Block, Return, ExpressionStatement, BinaryExpression,
    UnaryExpression, IntegerConstant, Identifier, ErrorRecovery,
}
pub struct NodeRecord {                 // T05 candidate
    pub kind: NodeKind,
    pub first_token: Option<TokenId>,   // committed token-level range start
    pub last_token: Option<TokenId>,    // committed token-level range end (>= first)
    pub parent: Option<NodeId>,         // committed parent; acyclic
    pub ordinal: u32,                   // unique per parent (continuation.next_child_ordinal)
}
// **Token-range nodes (user, in principle 2026-10-04):** a node's span is derived
// one hop from `first_token`/`last_token` -> committed `TokenRecord.span` ->
// `SpanRecord.source/start/end`; T05 appends **no** `sources.spans`. `None`/`None`
// is allowed only for `ErrorRecovery` (no token range); otherwise both are `Some`
// and `first <= last` in committed token order. A leaf (e.g. `Identifier`,
// `IntegerConstant`) has `first == last`. Synthetic/fabricated node ranges are not
// representable. No ty/first_child/next_sibling; typing is SemRecord-only; parent
// graph acyclic.
// **Rev 22 corrections (T05 findings 7, 8, 9, 12):**
// (a) **Committed tokens only.** A `NodeDraft` (T05) cannot carry a same-batch
//     `TokenDraft` (T04): a batch is single-task/single-owner and T04 owns
//     `TokenDraft`, so node `first_token`/`last_token` are **committed `TokenId`s
//     only** for M1; the rev-21 "same-batch TokenDraft" wording is withdrawn.
// (b) **Enforced token-range invariants (verifiable):** both endpoints must
//     exist, be committed tokens of the task's declared source, and satisfy
//     `first <= last` in committed token order; a leaf has `first == last`; a
//     non-leaf range must include every committed token boundary it spans (the
//     exact "range covers descendants" predicate is a T05 blocker). Tests:
//     `node_token_range`, `node_token_range_committed_source`,
//     `node_leaf_token_eq`. **Rev 25 F7 / rev 26:** the earlier draft name
//     `node_token_range_same_source` is **superseded** by the active
//     `node_token_range_committed_source` (same-source, committed tokens only) and
//     is **distinct** from the retired `node_token_range_same_batch_source` (the
//     withdrawn same-batch `TokenDraft` branch, H5). Only `node_token_range`,
//     `node_token_range_committed_source`, and `node_leaf_token_eq` are active, so
//     the §5/§13/§17.2 inventories agree.
// (c) **Verifiable PP-span provenance, not merely same-source:** the check must
//     confirm each committed `TokenRecord.span` resolves to the committed T03 PP
//     token it derived from (a recorded token→PP-token provenance), not merely
//     that the span has the same `source`. Until T04 exposes that verifiable
//     link, the M1 check is explicitly documented as **same-source-only** and the
//     stronger provenance check is a T04 `/6` blocker.
// (d) **Empty TU / synthetic / recovery nodes:** the EOF token is zero-length at
//     the raw end (the TranslationUnit node's range is the empty range at EOF);
//     synthetic nodes are not representable except `ErrorRecovery`; the
//     `ErrorRecovery` diagnostic location is carried as a raw committed `TokenId`
//     (not a fabricated node span).

pub enum ScopeKind { File, Block }   // M1 closed set; see deferral note below
pub struct ScopeRecord { pub parent: Option<ScopeId>, pub kind: ScopeKind }
pub enum ScopeEventKind { Enter, Exit }
pub struct ScopeEventRecord {           // append-only lifecycle; order = ScopeEventId
    pub scope: ScopeId, pub kind: ScopeEventKind, pub at: NodeId,
}
// Cumulative validation over committed + new events (added to the §7 phase-2b
// algorithm, i.e. **after** resolution, never before phase 2): exactly one Enter
// per ScopeRecord, at most one Exit, Exit strictly after Enter in `ScopeEventId`
// (append) order; `event.at` must be a live committed Node or a same-batch
// `NodeDraft` that resolves to a committed Node in this batch; a second Enter, a
// second Exit, an Exit-before-Enter, or a dangling `at` is
// ScopeLifecycleViolation (pre-mutation, per §6.4). `ScopeEventId` order is the
// authority; no runtime ordinal is stored.
// ScopeDraft cardinality and scope identity (T06, finding; DOC-13 correction):
// each **new** `ScopeDraft` in a batch must receive exactly one matching Enter
// event (same batch or already committed) and may receive at most one Exit; a
// ScopeDraft with no Enter is ScopeLifecycleViolation. A scope is identified by
// its **owner lexical node** -- the resolved `at` of its single Enter (the
// committed `TranslationUnit` node for the file scope; the `Block` node for a
// block scope) -- **not** by `(parent, kind)`: sibling blocks (e.g. two function
// bodies) may share `(parent, kind)` and are legal distinct scopes. The only
// duplicate-creation rejection is a new ScopeDraft whose resolved owner node is
// already claimed by another ScopeRecord (committed or same-batch); the claim is
// keyed on the owner node, not on `(parent, kind)`. The check never keys on a
// **live** arena record, because a closed scope **retains its record** (TY02, close
// visibility), so close neither blocks later siblings nor releases the owner
// claim. **File-scope policy (rev 23, H3):** the file scope is
// opened by a **T06 task that runs after the T05 `TranslationUnit` node is
// committed** (the `symbol_type` stage follows `parse`); its Enter pins a
// **committed `NodeId`** reference and has **no Exit** until job end (M1 never
// closes it). This is **not** a job-bootstrap action: the bootstrap ordering (how
// the first T06 task is scheduled once the TU node is committed) is a T06/`[INT]`
// `/6` decision, left open. A file-scope Enter emitted before the TU node is
// committed is not representable (it would violate the committed-Node rule).
// **OPEN-01 candidate (docs-only; §24.14, not accepted):** the candidate M1
// startup commits the TU **root** early -- in the `parse.TranslationUnit` task's
// first commit-visible batch, after the full token stream (incl. EOF) is
// committed and before the external-declaration loop -- and the file-Enter edge
// fires on that committed root, with the parse task `Waiting` so the
// `symbol_type` task can dispatch; parse then resumes with the committed file
// scope. The recorded fallback is the explicit M1 no-scope parse path plus the
// `M1-START-01` no-prebuilt-scope startup fixture. Bootstrap wiring beyond the
// edge and every encoding/numeric detail remain T06/`[INT]` `/6` co-freeze.
// **Block scope (rev 21):** Enter and Exit
// both reference the **`Block` node** (distinguished by `kind`); the Block node's
// token range supplies the boundary (`first_token` = `{`, `last_token` = matching
// `}`), so `at` is a real node and no fabricated `{`/`}` boundary node is
// introduced (the node shape plus the token range is authoritative).
// M1 scope-kind deferrals (explicit): `Function` (prototype) scope, `member`
// scope, `tag` scope, and label-as-scope are NOT modeled in M1. `(void)`
// therefore opens no prototype scope. The generic rule above allows any number
// of block scopes (nested or sibling); the **M1 positive fixture** (one TU, one
// function `main`) exercises exactly one file scope and one block scope, which
// is a **fixture cardinality, not a structural limit**. Function-scope/parameter
// scoping for `main` is carried by the block scope plus `SymbolRecord.decl`; a
// distinct function scope is a T06 `/6` deferral. **Labels:** the `Label`
// `SymbolKind` is enumerated for namespace
// completeness only and is **not exercised by M1**.
// T06 conflict model (finding): M1 defines **no general type-compatibility
// model**; `TypeCompatibilityChip`/`CompositeTypeChip` (TY21/TY22) are deferred
// beyond M1. Declaration conflicts are therefore **chip-level** obligations of
// the declaration/lookup chips, not inferred by commit. The only *structural*
// conflict the commit rejects pre-mutation is an exact duplicate append keyed by
// `(name, scope, kind)` in the same batch (`SymbolConflict`, a structural
// duplicate), and a redefinition that the declaration chip itself reports as
// `RedeclarationConflict`; neither is a compatibility judgement by the commit.
// §13 named tests cover the structural duplicate and the chip-reported case.

pub enum SymbolKind { Object, Function, Typedef, EnumConst, Label, StructTag, UnionTag, EnumTag }
// Closed namespace mapping, **proposed to be** hashed and validated at `/6`
// (not hashed today; not a free-form field):
//   Object|Function|Typedef|EnumConst -> Ordinary
//   StructTag|UnionTag|EnumTag        -> Tag
//   Label                             -> Label
// `SymbolRecord` stores **no** `namespace` field: the namespace is derived by the
// closed function above, and commit validates that any append attributed to a
// symbol uses the derived namespace. A stored-field alternative would be a
// T06 `/6` owner decision; the derived form is proposed. **Rev 22 narrowing (T06
// finding 7):** because no record carries a `namespace` value, "commit validates
// that an append uses the derived namespace" is vacuous — commit can check only
// the derived mapping from the record's `kind`, and a *lookup* namespace is a
// **chip** decision, not a commit check. The namespace-validation claim is
// therefore narrowed to "commit validates `kind` is in the closed set and the
// derived mapping is a function"; any stronger cross-namespace check needs a
// carrier or is chip-local. **A lookup whose
// qualified name resolves in a different namespace than the one queried is a
// miss (`NoSuchSymbol`), not a `SymbolConflict`**; the commit never infers an
// absent namespace and never fabricates a tag/member lookup (T06, finding).
// Member namespace and the fourth namespace beyond M1's Label kind are
// deferred; `Label` is enumerated but not exercised (above).
pub enum Linkage { None, Internal, External }
pub enum StorageDuration { None, Static, Automatic, Thread, Allocated }
pub struct SymbolRecord {
    pub name: NameId, pub scope: ScopeId, pub kind: SymbolKind,
    pub ty: Option<TypeId>,             // None only for Label
    pub linkage: Linkage, pub storage: StorageDuration, pub decl: NodeId,
}
// Point of declaration (preserves acceptance `M1-TY-06`). **Identity and
// visibility are separated (DOC-12; C11 6.2.1p7):** `SymbolRecord.decl` remains
// the `Identifier` `NodeKind` leaf that names the symbol (the `Identifier` node
// of the declarator for `main`) — the **stable identity/diagnostic location**,
// not the `Declarator`/`Declaration`/`FunctionDefinition` wrapper and not an
// arbitrary descendant. The lookup **visibility boundary** is the completion of
// the **complete declarator**: the declaration tuple
// `(source: SourceId, start: u64, end: u64, decl: NodeId)` derives `start`/`end`
// from the **completion point** of the enclosing complete declarator (the span
// of its `last_token`, the declarator's completion token), not from the
// identifier leaf and not from the declarator's full range (a full range would
// expose the symbol inside its own declarator, e.g. in an array bound). The
// completion token is reached from the committed `decl` leaf through the
// committed AST parent links (exact T05 `NodeKind` ownership/token-range
// semantics remain the T05 `/6` item); rev 22, T06 finding 1: `NodeRecord` has
// no `span` field:
//   declarator NodeRecord.last_token -> committed TokenRecord.span ->
//   SpanRecord.source/start/end,
// with `decl` = the leaf NodeId kept as the stable identity component. The
// identifier leaf's own token range is retained for diagnostics only and is
// **not** the visibility boundary.
// **Contrast test (`int n[n]` vs initializer; C11 6.2.1p7).**
// `int n = 3; void f(void) { int n[n]; }`: the array bound lies inside the
// inner declarator, so the inner `n` is not yet visible and the bound resolves
// to the **outer** `n`; a leaf-position rule would wrongly expose the inner
// `n`. `void f(void) { int n = n; }`: the initializer follows the completed
// declarator, so its `n` resolves to the **declared inner** `n`
// (self-initialization; the indeterminate value is a separate concern). Both
// spellings place the identifier leaf before the second `n`, so the leaf
// position alone cannot distinguish them.
// Deterministic lookup rule (TY03): the lookup
// walks only the **active scope chain** (`scope` upward to the file scope, never
// a closed/sibling scope), and given a name and query point
// `(source, qstart, qend, qNode)`, returns the ordinary-namespace symbol whose
// tuple `(source, start, end, decl)` — declarator-completion `start`/`end`,
// identifier-leaf `decl` — is `<=` the query tuple under the total order
// `(source, start, end, NodeId)` and is the greatest such. **Tie rule:** if two
// visible symbols share a tuple, the higher `SymbolId` is selected only when
// both are in the same scope; otherwise the innermost active scope wins. If none
// qualifies, the lookup is missing. This makes "missing before the point of
// declaration" (M1-TY-06) expressible without a stored extra tuple. General
// `TypeCompatibilityChip` recursion is **deferred beyond M1** and is not the
// mechanism that establishes declaration identity. Integration order fixture
// TY01 -> TY07 -> TY08 -> TY03 (TY08 `TypedefRegisterChip` is **not** exercised
// by the M1 fixture; see §9/§15.4).

pub enum IntRank { Short, Int, Long, LongLong }   // no Char rank; see CharKind
pub enum CharKind { Plain, Signed, Unsigned }     // all char spellings; independent of target plain-char signedness
pub enum TypeKind {                     // symbolic; no target widths
    Void, Bool,
    Char(CharKind),
    Int { rank: IntRank, signed: bool },
    Function { result: TypeId, params: Vec<TypeId>, prototype: bool, variadic: bool },
}
pub struct TypeRecord { pub kind: TypeKind }
// One consistent, enforceable M1 type-identity policy (T06): the canonical
// scalar `int` `TypeId` is produced by the **chip-specific rule**
// `type.canonical-int-single-producer` owned by TY13; the M1 function `TypeId`
// is produced by TY17 (`type.function-type-single-producer`). Reuse policy: a
// later TY13/TY17 task **reads and reuses the committed canonical id** rather
// than appending a duplicate; commit validates family/existence only and never
// performs structural dedup, so reuse is the producer's obligation, with §13
// tests (`canonical_int_reused_id`, `function_type_reused_id`). **Rev 22 blocker
// (T06 finding 2):** the concrete **reuse lookup mechanism** is not defined — how
// a TY13/TY17 task deterministically finds an existing committed canonical
// `TypeId` (an index/keyed field, a scan bound, or a committed lookup table) is
// absent, so "reads and reuses" is not yet actionable. This is a T06 `/6`
// blocker; no cache or hidden index is invented here. There is no
// structural-equality identity for `int`; general `TypeCompatibilityChip`
// recursion is **deferred beyond M1** and must not be advertised as the
// mechanism that establishes `int` identity.
// The manifest registry (not the commit) enforces a **field-scoped owner
// allowlist** on `types.records`: each row is keyed by **`ChipId`** (not a group
// key) as `(ChipId, StoreId, field)` with a declared `TypeKind` constraint, so
// the rule is chip-specific rather than a broad "TY13+TY17 may write
// `types.records`" grant. The allowlist rows are **proposed to be part of the
// `/6` hashed `M1AppendSchema` seed/signature** (not hashed today — rev 26); a non-allowlisted or kind-mismatched writer is
// `ManifestError::StoreOwnerViolation { chip, store, field, expected_kind }`
// emitted at `ManifestRegistry::register`, never by the commit.
// `TypeRecord` has no `canonical_key`.

pub enum ValueCategory { Lvalue = 0, NonLvalue = 1, FunctionDesignator = 2, Void = 3 }  // byte discriminants **proposed** (unfrozen; pinned at `/6`)
pub struct EffectMask(pub u32);
// Closed M1 encoding (T07): bit 0..=31 each denote one named effect class; only
// the empty mask `0` is legal in M1. Any nonzero mask is `EffectMaskUnsupported
// { task, mask }`. The per-bit *names/semantics* are **not yet assigned**; the
// claim "each bit denotes one named effect class" is a reservation of the
// encoding space, not a frozen class list. No bit assignment is hashed until an
// owner publishes the effect classes; a nonzero mask must not be interpreted by
// commit. No host pointer/address is in the mask.
pub enum ConversionOp {
    Identity, IntegerPromotion, UsualArithmetic, Assignment,
    // rev 21: explicit signedness/kind so IR07 cannot lose sign/zero extend:
    SignExtend, ZeroExtend, Truncate, IntToFloat, FloatToInt,
    PointerToInt, IntToPointer, ToBool,
}   // exact set a T06/T07/T09 co-freeze item
pub enum ConversionRole {           // closes the (node, role) rule subject (finding)
    Result, Operand0, Operand1, Return, Condition, Assignment, Argument, Cast,
}
pub struct ConversionPlan { pub op: ConversionOp, pub role: ConversionRole, pub from: TypeId, pub to: TypeId }   // the ONE shared conversion type; T06 CastPlan (TY29)/ArgumentPlans (TY28) are role-labelled instances
// T09 `IR07 ConversionLowerChip` must emit the matching `sext`/`zext`/`trunc`/
// int-float/pointer/truth op from `op`, not merely change the type. The exact
// permitted `(op, role)` set and the VF06 `(NodeKind, role, op)` completeness
// matrix are T06/T07/T09 co-freeze blockers (CDR §F).
// **Rev 22 findings (T07 findings 3, 6):**
// (a) `ConversionOp` has **no** `FloatToFloat` (float↔double/long double) and
//     does not carry **pointer qualifier** changes (e.g. adding/removing
//     `const`); the TY29/TY27 domains include those, so the enum domain is
//     incomplete. (b) `ConversionRole` has no explicit `TY27`-instance role
//     mapping: the mapping from the TY27 `AssignmentConversionChip` (and
//     TY28/TY29) instances to a `ConversionRole` and to the T09 lowering chip
//     (`IR07`) is not defined. (c) the `(op, role)` **permitted/forbidden**
//     enforcement (which pairings are legal) is a co-freeze blocker, not the
//     current uniqueness-only check. These are T06/T07/T09 `/6` blockers.
// M1 restricts the exercised `(op, role)` set: `(Identity, Return)` for the
// return conversion and `(Identity, Result)` on an explicitly recorded no-op;
// `IntegerPromotion`/`UsualArithmetic`/`Assignment` and the remaining roles are
// **enumerated but not exercised by the M1 fixture** and stay future-facing. The
// role is a required field, not inferred from `Vec` index.
pub struct SemRecord {                          // exactly one per NodeId (T07)
    pub node: NodeId, pub ty: TypeId, pub category: ValueCategory,
    pub effects: EffectMask, pub conversions: Vec<ConversionPlan>,
}
// `conversions` semantics (pinned): a **non-empty** entry is the explicit
// conversion/plan the semantic owner requires at this node; an **empty** vector
// means "no conversion plan was emitted" and is distinct from an explicit
// `ConversionOp::Identity` entry (a legal recorded no-op). Commit enforces **at
// most one** `ConversionPlan` per `(node, role)` for the roles M1 exercises; it
// does not invent missing conversions. VF06 verifies *required completeness*
// against the pinned `(NodeKind, role, op)` matrix (CDR §F3): in M1 the only
// possible required plan is a `Return`/`Return` plan when the operand type differs
// from the function return type; `int`/`int` requires **no** operand conversion
// (so `2+3` records none, consistent with `M1-SE-02`). A required conversion
// missing for a matrix row is a VF06 failure; VF06's scope is exactly that matrix,
// not unspecified "all nodes". Deterministic lookup is by `Vec` index (append
// order), never a map. `ConversionPlan`/role and `ConversionOp` are co-frozen
// T06/T07/T09 and **proposed to be hashed at `/6`** (not hashed today — rev 26);
// the exact matrix/op set is a co-freeze blocker.
// **Every checked node gets exactly one `SemRecord`, including the `Return` node
// and the `FunctionDefinition` node (rev 21).** This **removes the rev-20
// contradiction** in which the Return node was said to have no `SemRecord` while
// the checked return had to be persisted. The `FunctionDefinition` node's
// `SemRecord.ty` is the checked function signature `TypeId`; the `Return` node's
// `SemRecord` carries the checked return operand type and its `ConversionPlan`.
// **T07 does NOT consume a T09-created `FunctionRecord` (rev 21):** `FunctionRecord`
// is created by T09 `IR01` in the **ir** stage, *after* **sem**, so using it would
// be a temporal cycle. T09 reads the committed T07 `SemRecord`s (present at sem
// time). **Rev 22 blockers (T07 finding 1):** (a) **CheckedNode vs SemRecord
// identity:** the T07 package's `CheckRequest -> CheckedNode` has no separate
// store or family in this proposal; `SemRecord` is defined as the committed
// materialization of `CheckedNode` (one per checked node), so the two names must
// be reconciled (same family vs a distinct `CheckedNode` family) — a T07 `/6`
// blocker. (b) **T09 consumption link/request:** the proposal says T09 reads
// committed `SemRecord`s but defines **no** typed request/link by which a T09
// lowering task obtains the `SemRecord` for a `NodeId` (no `SemRecord` link in
// `LowerRequest`/`Parameter`/IR records). This is a T07/T09 `/6` co-freeze
// blocker; no link is invented here.
// `FunctionContext` is not a separate allocatable family and has **no
// `FunctionContextId`**; a parse `ContinuationRecord` alone is **not** a
// `FunctionContext` and is never a proxy for it. The exact carrier shape / whether
// a `FunctionContextId` exists is a hard T07 `/6` owner blocker (CDR §F1).
// **Rev 22 conflict surfaced (T07 finding 2):** the T07 task package still
// specifies `CheckRequest(NodeId, ScopeId, FunctionContextId)`, which *requires* a
// `FunctionContextId`; this proposal selects the **no-`FunctionContextId`**
// direction as a draft but the task package is unedited. The proposal direction
// is selected, not authoritative; reconciling it requires either a T07 package
// amendment or a T07 `/6` owner decision. Neither is claimed here.
// DuplicateSemRecord if a NodeId already has one; lookup scans the sem arena in
// append order. qualifiers/atomic/bitfield/EffectGraph are deferred beyond M1;
// **SE26 effect-graph behavior is NOT claimed as exercised by M1** (the fixture
// has no effect graph), and `SE26` remains catalog-only for M1.
// **T07/T08 pipeline ordering (rev 21):** the sem stage precedes the const stage.
// The split is **two distinct** constant requests:
// `const.literal-decode` (T04→T08, committed `RecordRef::Literal` + `LX08` candidate
// type) and `const.evaluate` (T07→T08, the semantic variant set
// `ConstantRequest::Literal { literal: RecordRef::Literal, node, required_kind }` /
// `ConstantRequest::Binary { node, op, lhs, rhs, required_kind }` — the literal
// references are part of the H1 accepted-in-principle shape; the M1 variant set
// and the binary committed operand/operator path are frozen by the OPEN-03
// co-freeze, see the carrier block below). Both use
// committed references only
// (never a same-tick cross-stage draft), so no `T07→T08→T07` cycle exists.
// **H1 (accepted in principle 2026-10-04): that split is the accepted `/6`
// revision working basis, not a freeze.** Under it the two requests do not both
// hand `node`/`required_kind`: only `const.evaluate` (sem stage) does;
// `const.literal-decode` carries the committed literal + candidate type, and
// `legality` lives in the result (§8/§18.1, CDR §C2/§F4). The sem-before-const
// ordering is selected, so the sem→const handoff is a committed next-tick
// reference with **no cycle**. The exact kind names/vocabulary remain a T07/T08
// co-freeze item.
// **Return/FunctionDefinition have SemRecords** (rev 21); `FunctionRecord` is a
// T09 IR record for IR purposes only.

pub struct ConstRecord { pub ty: TypeId, pub value: i128 }
// T08, success-only host-neutral carrier (not a target width/overflow oracle).
// **T08 owns `ConstRecord.value`** (single writer, §8); T09 owns IR `Constant`
// emission (`InstructionRecord` with `op: Constant`, `immediate: Some(ConstId)`).
// `value` is the **canonical mathematical signed integer value** of the
// constant; an unsigned interpretation is a type-level fact, not a second stored
// value (**canonical-unsigned sign invariant:** `value` is always the signed
// representative; unsigned-ness lives only in `ty`; **rev 22 clarification (T08
// finding 4):** for an unsigned mathematical magnitude that fits the signed range
// `value` holds that **positive** signed magnitude (e.g. `0xFFFFFFFF` as `4294967295`,
// not `-1`); it is **not** a two's-complement reinterpretation. A magnitude above
// `2^(max_const_bits-1)-1` is not representable and is `ConstOverflow` (§G2), not
// a fabricated negative). Bounded representability
// (rev 21, exact): `max_const_bits` defaults to **128**; `value` is representable
// iff `-(2^(max_const_bits-1)) <= value <= 2^(max_const_bits-1) - 1` (for 128 this
// is the `i128` range `[i128::MIN, i128::MAX]`). Every M1 constant op is checked
// (`checked_add`/`checked_sub`/`checked_mul`/`checked_neg`); a mathematical result
// outside the bound (including an unsigned magnitude `> 2^(max_const_bits-1)-1`
// such as `2^128-1`, which cannot be stored as a signed `i128`) is a **chip
// diagnostic** `ConstOverflow` (not a `CommitError`), carried through the chip's
// `DiagnosticDraft`/`Fail` path. An op that would require an unverified target
// width/signedness (e.g. a conversion on a concrete width) fails closed as the
// **chip diagnostic** `ConstUnsupported`. Checked arithmetic is a **chip-level**
// obligation (in CL03/CL05; `CL02` `ConstantUnaryChip` is unexercised by M1), not
// a commit validation pass; **no `ConstOverflow`/`ConstUnsupported` variant exists
// in `CommitError`** (§6.4). `max_const_bits`, the formula, and its
// stage/error/rule/test are **proposed to be** pinned in `M1AppendSchema` at
// `/6` (`const.max-const-bits`/`limits.max_const_bits`) — **prospective `/6`,
// not pinned today** (rev 25 F1) — and would be enforced at the const stage.
// **Rev 22 blockers (T08 finding 3):** (a) the **numeric carrier** of
// `max_const_bits` is not fixed (a `Limits` field vs a hashed schema constant);
// (b) the required **`max_const_bits <= 128`** constraint (since `value: i128`)
// is asserted but not enforced at config validation; (c) a zero-field
// `RestrictedChip` has no bus access, so the const-stage chip **cannot read
// config** directly — the bound must be delivered in the task input projection or
// enforced by the adapter/commit. The exact carrier, validation, and projection
// are T08/`/6` blockers.
// **Hash: proposed for the `/6` hash, NOT pinned today (rev 25 F1):** the
// `ConstRecord` shape, `value: i128`, the checked-op rule, and
// `limits.max_const_bits`. M1 success values are `2`, `3`, and the folded `5`;
// the mathematical additive fixture has no overflow, so **no impossible
// i128-overflow fixture is invented** for M1. A genuine `i128` boundary vector
// (`i128::MAX + 1` or `2^128-1`) is a **width-dependent synthetic negative
// input** and is a chip-diagnostic negative unit vector, not an M1 acceptance row.
// Full fixed-width constant folding remains **Part B / probe-gated**; the earlier
// `/7` nomenclature is withdrawn (no `/7` is proposed by this draft).

// **M1 constant-expression handoff — OPEN-03 co-freeze decision (T01/T07/T08;
// user-directed, 2026-10-05; doc-only; no code).** The documentation-review
// OPEN-03 gap is closed at the document level: the M1 `const.evaluate` input
// path and the request/result variants below are **selected and frozen for the
// M1 `/6` co-freeze**, covering **both** the leaf-literal use and the binary
// additive use. Every record input arrives as a **committed** reference (the H1
// allocation is preserved: committed T04 `LiteralRecord`; T08 sole
// `constants.records` writer; record-reference payloads). Exact wire tags
// (`RecordRef::Literal`/`RecordRef::Const`), payload-variant spellings,
// task-kind spellings, and all numeric codes remain `/6` hash-inventory items;
// the *shapes below* are the frozen M1 handoff and must not be re-decided per
// implementation. The separate T04-only `const.literal-decode` flow carries the
// committed `RecordRef::Literal` and does **not** carry `node`/`required_kind`.
pub enum ConstantRequest {              // sem-stage per-use request (T07->T08 `const.evaluate`)
    Literal(ConstantLiteralRequest),    // leaf literal use (M1 exercised: `2`, `3`)
    Binary(ConstantBinaryRequest),      // binary use (M1 exercised: additive `2+3`)
}
pub struct ConstantLiteralRequest {
    pub literal: RecordRef,             // committed `RecordRef::Literal` (T04-owned)
    pub node: NodeId,                   // committed per-use AST node (M1: IntegerConstant)
    pub required_kind: RequiredKind,    // M1: IntegerConstantExpression
}
pub enum ConstExprOp {                  // T07 checked constant-expression operator
    Add,                                // M1 closed exercised subset; append-only beyond M1
}
pub struct ConstantBinaryRequest {
    pub node: NodeId,                   // committed per-use AST node (M1: BinaryExpression)
    pub op: ConstExprOp,                // T07 checked operator; M1 closed exercised subset { Add }
    pub lhs: RecordRef,                 // committed `RecordRef::Literal` (source-order lhs)
    pub rhs: RecordRef,                 // committed `RecordRef::Literal` (source-order rhs)
    pub required_kind: RequiredKind,    // M1: IntegerConstantExpression
}
// **Binary-request invariants (M1, frozen).** `node` is the committed
// `BinaryExpression` use node; `lhs`/`rhs` are the committed T04 literals of its
// two operand leaves in source order; `op` is T07's checked operator fact
// (`ConstExprOp::Add` in M1) and is **not** re-derived by T08; the committed `+`
// token remains the lexical provenance inside the node's committed token range
// (`M1-PA-07`). T07 emits the request after the sem checks (SE02/SE07); it is
// committed before T08 runs (sem -> const next-tick handoff; no
// `T07->T08->T07` cycle). T08's M1 evaluator accepts only the exercised `Add`
// op with two committed integer/no-suffix/decimal literals; anything else is
// `ConstLegality::Unsupported` (fail closed; never a silent fold and never a
// hand-built value). `ConstExprOp` is append-only beyond M1.
pub struct ConstantResult {             // T08 result; carries the `Const` ref + `legality`
    pub value: RecordRef,               // committed `RecordRef::Const` (T08 sole writer)
    pub legality: ConstLegality,        // M1: Legal | NotConstantExpression | Unsupported
}
// **Result/handoff invariants (M1, frozen).** The binary request produces
// **exactly one** committed `ConstRecord` for the folded result (`ty` = symbolic
// `int`; `value` = the mathematical `+5` for `2+3`) — no per-operand
// `ConstRecord` and no join. T09 consumes that committed `ConstRecord` (the same
// `ConstId` carried by the `ConstantResult` as a committed `RecordRef::Const`)
// and emits the IR `Constant` without recomputation (H7); the T08->T09 delivery
// carries the committed `RecordRef::Const`, not a bare projected index. The
// exact result envelope (`ResultValue` variant spelling/tag) and task-kind
// spellings remain T01 `/6` shared-interface items; the M1 fixture asserts the
// semantic identity (same `ConstId`), not the envelope encoding. `legality`
// stays a result payload field with no separate record family (rev 44). The
// earlier `ConstantRequest(node, required_kind)` ->
// `ConstantResult(bits/float/symbol+addend, legality)` spelling is superseded
// for M1.

pub enum ArtifactKind {
    // Checked-in variants (retained, unchanged):
    Preprocessed,   // PP28 final preprocessed artifact (can be lexed again); PP01 output is NOT this
    Assembly, Object, Snapshot, Trace,
    // New append-only variants added by M1 (finding 4):
    Normalized, Spliced, CommentFree,
}
// **Total variant set = 8.** `Normalized` (PP01 newline-normalized bytes) is
// distinct from `Preprocessed` (PP28 final artifact): different stages and
// different payloads. `requires_map(kind)` is a **total** predicate **proposed
// to be hashed at `/6`** (prospective; not pinned today — rev 25 F1):
//   map-mandatory (source: Some, full raw_offsets): Normalized, Spliced,
//     CommentFree, Preprocessed;
//   map-optional (raw_offsets must be empty; source is Option<SourceId>,
//     validated when Some — rev-47 delegated candidate default): Assembly,
//     Object, Snapshot, Trace.
// **M1-produced set (rev 21, authoritative):** the M1 fixture emits exactly
// `Normalized`/`Spliced`/`CommentFree` (map-mandatory). `Preprocessed` is a
// **declared** map-mandatory kind but is **NOT produced by the M1 fixture** (an
// explicit unexercised gap, not a pass); it is distinct from `Normalized` (PP01
// buffer). The retained `Assembly`/`Object`/`Snapshot`/`Trace` are target/host
// artifacts and are map-optional. **Writer ownership (rev 22, T03/T04 finding
// 3):** within M1 `artifacts.fragments` is T03-only and only
// `Normalized`/`Spliced`/`CommentFree` are produced; `Preprocessed` (PP28) is
// declared-not-produced by M1; `Assembly` is a Part B T11 output (probe-gated);
// `Object`/`Snapshot`/`Trace` are Host/Part B outputs. None of those four has an
// M1 writer — they are explicitly out of M1 scope, not silently assigned, and
// their Part B owner remains a blocker (not frozen). A map-mandatory kind with no
// source is
// `ArtifactSourceMissing { task, artifact }`. `artifact_kind_name` and its hash
// inventory gain the three new names; this classification requires T03
// second sign-off (§15.3) and is a `/6` blocker, not a frozen claim.
pub struct ArtifactRecord {             // reuses ArtifactId / StoreId::Artifacts / Arenas.artifacts
    pub kind: ArtifactKind,
    pub source: Option<SourceId>,       // raw source the map refers to (Some for map-mandatory; map-optional: validated when Some — rev 47)
    pub bytes: Vec<u8>,
    pub raw_offsets: Vec<u64>,          // len == bytes.len() + 1, checked; index = output boundary
}
// T03 source/map rule (pre-mutation; rev-47 delegated candidate default):
// when `source` is `Some`, it must equal the task's declared
// `RecordRef::Source` (`ArtifactSourceMismatch`); a map-mandatory kind requires
// `Some(source)` (`ArtifactSourceMissing` when absent) and the map must satisfy
// `bytes.len() <= max_source_bytes`,
// `raw_offsets.len() == checked_add(bytes.len(), 1)`, `raw_offsets[0] == 0`,
// non-decreasing entries, and `raw_offsets[last] <= source.bytes.len()`; any
// failure is `ArtifactMapInvalid { task, artifact, reason }`. A map-optional
// kind requires empty `raw_offsets` (non-empty rejected) and may carry a
// validated `Some(source)` — the same equality rule — or `None`; there is no
// source-payload-equals-`bytes` requirement. Every logical
// offset produced by T03 is remapped to a **raw physical** `SpanRecord` offset
// through `raw_offsets` before a `SpanDraft` is formed, so stored spans are
// always raw `SourceRecord.bytes` offsets.

pub enum ParseContext { TranslationUnit, ExternalDecl, Specifier, Declarator, ParameterList, Block, Expression, Assignment, Unary, Primary }
// Parse state **extends the existing `ContinuationRecord`** (`ContinuationId`
// arena); no competing `ParseContinuation` struct is defined. The extension is
// field-by-field with the mapping from the checked-in fields, but the **exact
// field order and wire encoding remain a T05 `/6` sign-off item** (not
// finalized by this revision): `[T05 finding]`
//   existing `resume_kind: TaskKind`  -> `production: TaskKind`  (renamed; same type)
//   `awaited: Vec<TaskId>`             -> **rev-21 removes it in favor of
//                                        `TaskState::Waiting(WaitSet)`. Rev 22
//                                        flagged that this CONFLICTS with the frozen
//                                        T01 §4 wording ("a continuation must
//                                        record ... awaited child IDs"). Rev 30
//                                        (user decision B, 2026-10-04) accepts in
//                                        principle the supersession: keep
//                                        `WaitSet` only (option b) and amend
//                                        T01 §4 in `/6` ONLY (no `/5` edit). The
//                                        guardrail-adjacent T01 authority is still
//                                        pending: **T01 integrator + T05 owner
//                                        acceptance**. The alternative (a)
//                                        (retain `awaited`, drop `WaitSet.children`)
//                                        is not selected.**
//   existing `scope: Option<ScopeId>` -> `scope: Option<ScopeId>` (retained)
//   new `cursor: TokenId`             (EOF allowed; committed TokenId)
//   new `context: ParseContext`
//   new `binding_power: u16`
//   new `parent: Option<NodeId>`      (committed parent; None only for the TU root)
//   new `partial_children: Vec<NodeId>` (sorted, contiguous ordinals)
//   new `next_child_ordinal: u32`     (the uniqueness source for NodeDraft.ordinal)
//   new `previous: Option<ContinuationId>` (committed predecessor; acyclic chain)
// **Direction (rev 21):** `Task.continuation: Option<ContinuationId>` points from
// a task to its current/next parse frame; `previous` points back to the
// predecessor frame (committed-only, acyclic). The successor Enqueue's
// `ContinuationRef::OwnBatch` resolves to the `ContinuationId` minted for that
// task's draft in the same commit.
// `RecordFamily` **ordinals and RecordRef wire tags are distinct inventories**
// (finding 7): the `Continuation` family ordinal and its wire tag need not be
// equal and are **proposed to be pinned separately at `/6`** (not pinned today).
// **Node children/counter exactness (T05, finding):** the root node has
// `parent: None`; every other node has `parent: Some(p)`. Under a given committed
// parent, child ordinals are exactly `0..n-1` contiguous; root siblings are the
// children of the TranslationUnit node (never `parent: None` siblings).
// `partial_children` must equal the parent's committed children plus this batch's
// new children, sorted by ordinal; `next_child_ordinal == max(ordinal)+1`. A gap,
// a duplicate, or a `next_child_ordinal` inconsistent with `partial_children` is
// NodeOrdinalNotUnique / a structural parse-state error. Ordinal uniqueness is
// cumulative across committed + same-batch (a new NodeDraft colliding with a
// committed or same-batch sibling is `NodeOrdinalNotUnique`).
// **Token-range source checks (rev 22):** a Node's `first_token`/`last_token`
// are **committed `TokenId`s only** (dereferenced **one hop** to the committed
// `SpanId`); the rev-21 "same-batch `TokenDraft`" form is withdrawn (T05 finding
// 7: T05 cannot carry a T04 `TokenDraft` in its own single-owner batch). **All**
// of the referenced tokens' spans must have the same `source` as the task's
// declared source (`SpanSourceMismatch`). Because T04 `TokenRecord.span` is
// itself the committed T03 PP span, this checks the node range against the source
// without any T05 span write. Committed token→span linkage is intentional and
// defined; the stronger "span derived from the recorded T03 PP token" provenance
// check is a T04 `/6` blocker (finding 9).
// Cross-tick handoff protocol (T05; the pipeline mechanism is fully specified
// here, replacing the earlier "next-tick committed IDs" prose). A parse task that
// cannot finish in one tick emits, in a single batch:
//   1. `AppendRecords { task, batch: [ContinuationDraft] }` — the continuation is
//      **append-only**, owned by that task, `previous` pointing at the task's own
//      committed continuation (committed ID) or `None`;
//   2. `Enqueue(TaskDraft { kind: <parse stage kind>, payload: RecordRef-only,
//      owner, parent: Some(self), continuation: ContinuationRef::OwnBatch(DraftRef) })`
//      — the successor's continuation ref is **own-batch only**; a cross-task
//      draft ref is unrepresentable;
//   3. and either `AwaitChildren { task, children }` (if it must resume after the
//      successor) or `Progress { task, ordinal }` (if not) or `Complete`.
// **Pre-apply validation (rev 21):** in the no-mutation pass, every
// `ContinuationRef` (`Committed` existence/family; `OwnBatch(k)` range/family) and
// `ChildRef` (`Committed` existence as a child; `OwnBatch(k)` own-Enqueue index) is
// validated **before** phase 4 apply; a violation rejects the batch
// (`AwaitChildrenRefInvalid`/`ContinuationRefInvalid`; exact names a `/6` item).
// Commit phase 4 materializes the ContinuationDraft (allocating `ContinuationId`),
// resolves the successor's `ContinuationRef::OwnBatch` from the own-task resolve
// table to the just-committed `ContinuationId`, applies the Enqueue (successor
// visible next tick with `continuation: Some(committed_id)`), then applies
// `AwaitChildren`/`Progress`. No same-tick consumption; no cross-task draft ref.
// `AwaitChildren { task, children: Vec<ChildRef> }` with
// `ChildRef = Committed(TaskId) | OwnBatch(u32)` (the `u32` indexes this task's
// own `Enqueue` list, same-task only). Commit resolves `OwnBatch` children to the
// committed child TaskIds after applying its own Enqueues, then sets the parent
// `TaskState::Waiting(WaitSet { children, host_request: None })` (existing
// `WaitSet`). **Resume (rev 21):** the **commit-apply phase** (not CT09/CT10)
// performs the join: when every committed child in the `WaitSet` is `Completed`
// (or any child `Failed`), the parent returns to `Ready` and is reinserted into
// its **own `stage_queues[stage_of(parent.kind)]`** before
// latch (selectable next tick); a failed child propagates the parent `Failed`
// exactly once. **OPEN-02 result delivery (selected draft direction; T01/T05/T02
// `/6` co-freeze):** the join consumes no child result; the child results stay
// committed and unconsumed, and consumption stays with the parent's own next-tick
// atomic commit (same commit-visible consume unit as the TU stage edge,
// §24.3/OB-11) — or, only if `/6` requires consuming at join, each result is
// transferred at join into parent-owned durable state in the same commit as the
// `Ready` transition. Exactly-once consumption must coincide with exactly-once
// semantic processing; replay/retry between join and parent execution must
// neither double-consume nor lose results (required fixture
// `join_then_replay_retry`). **Rev 22 corrections (T05 findings 4/5/6):** (a) the resume stage
// is `stage_of(parent.kind)` — the parent task's **kind** — **not**
// `continuation.production`, and the realization must handle a parent whose
// `continuation` is absent; (b) the candidate's child-failure wording is
// contradictory ("all `Completed` *or any `Failed`* -> `Ready`" vs "a failed
// child propagates `Failed`"); selected draft direction: any committed child
// `Failed` fails the parent exactly once (never `Ready`), all `Completed` resumes
// `Ready`, and the sibling policy is await-all (accepted, CDR rev 42); the fate of
// non-terminal siblings is a T05/T02 `/6` open item; (c) **CT07 decision carrier:** because
// commit-apply performs the join, CT07 needs a concrete committed decision
// carrier (a join-decision record/result consumed by commit-apply) or it must be
// defined as a commit-apply invariant and removed from the join path. Otherwise
// CT07 is decorative and the join is re-derived; this is a T02/T05 `/6` blocker
// and no carrier is invented here. (d) a join resume adds reinserts that the
// phase-3 queue bound must count (see §7 `join_reinserts`). **Chain:** `previous`
// links are committed-only and acyclic
// (`ContinuationCycle` is a structural commit error).
// **No extra-tick scheduler path is required or claimed for M1**; M1 uses
// own-batch continuation resolution plus Progress/AwaitChildren. `Payload` stays
// `RecordRef`-only. Exact residual: the exact field order/wire encoding and the
// two-tick flow still need T05 sign-off (the `awaited`-removal, pre-apply
// validation, and commit-apply reinsertion are selected, not the encodings).
// **Parse errors are chip diagnostics, not CommitErrors (T05, finding):**
// `ParseCursorDidNotAdvance` and `ParseDepthExceeded` are emitted by the parse
// chip through the `Fail`/`DiagnosticDraft` path. The parse nesting bound **reuses
// the existing `limits.max_task_depth`/TaskDepth budget** (mapped to
// `ParseDepthExceeded`); no separate `max_parse_depth` limit is introduced in M1.
// **Rev 22 clarification (T05 finding 11):** the parse depth must count only the
// parser's own continuation/child chain (parse frames), not the total number of
// bus tasks across stages, or an unrelated stage would trip `ParseDepthExceeded`;
// the exact counting predicate and the error-vs-depth ordering are a T05 `/6`
// blocker.
// PA20 `UnaryExpressionChip` in M1 exercises **only the unary-`+` path**
// (`M1-PA-08`, `2+ +3`); the `sizeof`/`alignof` **type-name** variants
// (`M1-PA-10` abstract declarator) are **not exercised and must not be conflated
// with the M1 unary path**.

pub enum IrOp { Constant, Add, Return }         // closed op table (see invariants)
pub struct FunctionRecord { pub symbol: SymbolId, pub signature: TypeId, pub entry: BlockId, pub linkage: Linkage }
pub struct BlockRecord { pub function: FunctionId, pub ordinal: u32 }   // instructions derived by ascending InstructionId
pub struct ValueRecord { pub ty: TypeId }       // producer = unique Instruction.result
pub struct InstructionRecord {
    pub op: IrOp, pub block: BlockId,
    pub operands: Vec<ValueId>,
    pub immediate: Option<ConstId>,             // Constant only
    pub result: Option<ValueId>,                // Constant/Add: Some; Return: None
}
// Op table (**prospective `/6`: proposed, NOT frozen/hashed**, T09; H8):
//   `ir.op.constant`: 0 operands, immediate Some, result Some, non-terminator;
//   `ir.op.add`: 2 operands, immediate None, result Some, non-terminator
//     (verifier/future only; the M1 fixture emits the folded constant);
//   `ir.op.return`: 1 operand for M1 main, immediate None, result None, the unique
//     terminator.
// Unlisted op => `UnsupportedIrOp { task, op }`; an implemented node whose
// lowering is absent => the **chip diagnostic** `UnsupportedNode { task,
// node_kind }` (T09), emitted through the `Fail`/`DiagnosticDraft` path, never a
// silent no-op. Exact `committed` vs `terminated` (cumulative over committed +
// same-batch): a Block is `committed` once created, and once a terminator is
// appended it is `terminated`; no instruction may be appended to a terminated
// Block (`BlockTerminated`); the terminator must be the greatest `InstructionId`
// in a terminated block (`TerminatorNotLast`). **`TerminatorMissing` trigger
// (rev 23 H11 history + rev 49 operative direction):** the rev-23 marker-based
// *direction* and its unresolved `CompletedFunction` marker family/schema are
// **historical/superseded**. **Rev 49 (user, CDR rev 49; §22.4) selects the
// operative direction:** the trigger **reuses the committed terminal result of the
// IR28 `FunctionEnd` task** — the `TaskState::Completed(ResultId)` produced by the
// task whose **kind is `FunctionEnd`** — as the **deterministic function-completion
// fact**, checked by a **T01-owned typed phase-2b commit-apply validation hook**
// that verifies the function's **entry block is terminated**. This adds **no** new
// marker record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot
// encoder and **no** new `ResultValue` variant. It is **not a frozen `/6`
// hook/schema and authorizes no code**: the exact hook contract, its hash impact,
// the result typing/commit ordering, and the T09/T01 co-freeze remain **open**.
// (Historical rev-23 wording, preserved: T09 `IR28 FunctionEndChip` was *intended*
// to commit an explicit **function-completion marker**, whose record family/store/
// backing arena/`RecordRef` tag/snapshot encoder/`RecordFamily` variant were
// undefined; that direction is superseded for the operative direction.) IR01/IR02
// commit Function + entry Block before instructions. A Block referenced by an instruction must be
// committed and open (`BlockMissing`). Op arity is `OpArityMismatch`; a
// non-`Constant` op carrying an `immediate`, or a `Constant` without one, is
// `OpImmediateMismatch`; a result/terminator mismatch (e.g. `Return` with a
// result, `Constant` without one) is `OpResultMismatch`; the `immediate`'s
// `ConstId` must exist and, for `Constant`, its `ConstRecord.ty` must equal the
// instruction's **result `ValueRecord.ty`** (`OpImmediateTypeMismatch`; the
// expected type is the selected draft direction — no host-width or
// lowering-context inference).
// **Every rejection carries a stable rule id (rev 21):** the rule ids are
// `ir.op-arity`, `ir.op-immediate`, `ir.op-immediate-type`, `ir.op-result`,
// `ir.block-missing`, `ir.block-terminated`, `ir.terminator-not-last`,
// `ir.terminator-missing`, `ir.value-unique-producer`. **H8 — the rule/alias
// inventory is UNRESOLVED, not fixed:** the `NORMATIVE_RULES` inventory in
// §12.12 contains **both** `ir.op-immediate-type` **and**
// `ir.op-immediate-type-result-ty` (aliases to be collapsed at `/6`) and **both**
// `ir.terminator-missing` **and** `ir.terminator-missing-explicit-commit-marker`
// (candidate ids to be collapsed at `/6`); §12.12 also had a missing comma before
// `node.token-range-no-span-write` that is corrected here. No rule id is hashed
// until `/6`.
// `Constant` missing-immediate vs missing-result precedence (T09 finding 4): a
// `Constant` instruction requires an `immediate` (`OpImmediateMismatch`) and a
// `result` (`OpResultMismatch`); when both are absent the **immediate** rule is
// evaluated first in the canonical op-table order. This order is a T09 `/6`
// blocker, not frozen.
// **Ownership split (T08/T09; rev 23 H7):** T08 **computes** the `ConstRecord`
// value (the folded `int 5`); T09 only **emits** the IR `Constant`
// (`InstructionRecord`) that references the already-computed `ConstRecord`. The
// old rule/test name `ir.folded-int5-part-a-producer` /
// `folded_int5_part_a_producer` implied T09 computes the value and is **retired**;
// the name is now `ir.folded-int5-constant-emission` / `folded_int5_ir_constant`
// (the renaming is a `/6` inventory item). The
// Part A T09 constant emission is independent of the target probe. The probe-gated
// fixture is named by **fixture ID only**: fixture `M1-CL-02`, executed by chip
// `CL03` (`ConstantBinaryChip`), asserts the target-width bit pattern
// `0x00000005`; **chip `CL02` (`ConstantUnaryChip`) is unexercised by M1**. The
// earlier "`CL02` chip vs `M1-CL-02`" shorthand is corrected: fixture
// `M1-CL-02` ≠ chip `CL02`/`CL03`; the target-probe row is owned by T08/T11 and
// does not gate the Part A folded-5 IR result.
```

`TypeKind::Int { rank, signed }` and `ConstRecord { ty, value }` deliberately
exclude target width and bit pattern; concrete width/bit-pattern is Part B and
probe-gated (see M1_TARGET_ACCEPTANCE IR-6). The type-identity policy is the
**chip-specific** canonical-`int`/function-type single-producer rule above; there
is **no** commit-side structural dedup, and structural compatibility is deferred
beyond M1.

Reviewer-driven changes vs rev 14: PpToken `spelling` inline bytes + closed
`PpTokenKind` (T03); `NodeRecord` parent+ordinal with SemRecord-only typing
(T05/T07); `ScopeEventRecord` scope lifecycle (T06); symbolic `TypeKind` with
`Bool` and no `canonical_key` (T06); `SemRecord` with `conversions`/`ValueCategory`
and `ConversionRole` (T07); `InstructionRecord.immediate`;
`BlockRecord` function+ordinal with derived instructions; `ValueRecord` without
`def` (T09). **Rev 20 additions:** `ConversionRole`, `TerminatorMissing`/
`OpImmediateMismatch`/`OpImmediateTypeMismatch`, total `ArtifactKind`
`requires_map`, `ContinuationRef`/`AwaitChildren`, chip-diagnostic error
classification, and the cross-task write-conflict predicate. **Rev 21 additions
(user in principle + reviewer corrections):** `TokenRecord.span` = committed T03
PP span; `NodeRecord.first_token`/`last_token` (no T05 span write); `awaited`
removed from the continuation (`WaitSet`-only) and `task.continuation` direction
fixed; `ConversionOp` signedness ops (`SignExtend`/`ZeroExtend`/`Truncate`/…);
`SemRecord` for `Return`/`FunctionDefinition` and no T09-`FunctionRecord` cycle;
`ConstRecord`/`max_const_bits` signed-range formula; `Preprocessed` not
M1-produced; marker-based `IR28` `TerminatorMissing` trigger direction (marker
family/schema unresolved, H11); `Constant` immediate =
result type; rule id per IR rejection. Remaining owner questions are in
§15.3/§15.4.

---

## 6. Proposed envelope additions

### 6.1 Draft wrappers (fully enumerated) `[audit2 N7]`

Convention: a field that denotes a record reference is `RecordLink`; an optional
reference is `Option<RecordLink>`; a list of references is `Vec<RecordLink>`.
Non-reference fields keep their final scalar/enum type. `RecordLink` is
`{ expect: RecordFamily, target: LinkTarget }` and `LinkTarget` is
`Committed(RecordRef) | Draft(DraftRef)` — one uniform representation, so
`RecordDraft::links()` can enumerate **every** reference field with its expected
family. The `RecordLink<Family>` notation in the table below is **documentation
shorthand only**: the actual field type is the non-generic `RecordLink` above
(one closed enum; no Rust generics, no type parameters). `[audit3 AD3]`

| Draft | Fields (references shown as links) |
|---|---|
| `NameDraft` | `bytes: Vec<u8>` (T04-only interning identity; no links) |
| `SpanDraft` | `source: RecordLink<Source>` (committed only), `start: u64`, `end: u64`, `expansion: Option<RecordLink<Expansion>>` |
| `ExpansionDraft` | `parent: Option<RecordLink<Expansion>>`, `spelling: RecordLink<Span>`, `expanded: RecordLink<Span>`, `ordinal: u32` |
| `PpTokenDraft` | `kind: PpTokenKind`, `span: RecordLink<Span>`, `spelling: Option<Vec<u8>>` (no `head`/`ordinal`) |
| `TokenDraft` | `kind`, `span: RecordLink<Span>` (**committed T03 PP span; T04 writes no span**), `name: Option<RecordLink<Name>>`, `literal: Option<RecordLink<Literal>>`, `flags` |
| `LiteralDraft` | `token: Option<RecordLink<Token>>`, `kind: LiteralKind`, `radix: u8`, `suffix`, `value: Vec<u8>`, `negative: bool`, `spelling: Vec<u8>`, `candidate_type` (**PROPOSED/UNFROZEN**; field-symmetric with `LiteralRecord.candidate_type`, carrying the `LX08` candidate type; exact enum/type encoding is a T04/T08/T01 `/6` co-freeze item) (T04-owned) |
| `NodeDraft` | `kind: NodeKind`, `first_token: Option<RecordLink<Token>>`, `last_token: Option<RecordLink<Token>>` (**token range; no span link**), `parent: Option<RecordLink<Node>>`, `ordinal: u32` |
| `ScopeDraft` | `parent: Option<RecordLink<Scope>>`, `kind` (no chip ordinal) |
| `ScopeEventDraft` | `scope: RecordLink<Scope>`, `kind: ScopeEventKind`, `at: RecordLink<Node>` (order = `ScopeEventId`) |
| `SymbolDraft` | `name: RecordLink<Name>`, `scope: RecordLink<Scope>`, `kind` (namespace derived from kind), `ty: Option<RecordLink<Type>>`, `linkage`, `storage`, `decl: RecordLink<Node>` |
| `TypeDraft` | `kind: TypeKindDraft` (no `canonical_key`); `Char(CharKind)`, `Int { rank, signed }`, `Bool`, `Void`, `Function { result: RecordLink<Type>, params: Vec<RecordLink<Type>>, prototype: bool, variadic: bool }` |
| `SemDraft` | `node: RecordLink<Node>`, `ty: RecordLink<Type>`, `category`, `effects`, `conversions: Vec<ConversionPlanDraft>` |
| `ConversionPlanDraft` | `{ op: ConversionOp, role: ConversionRole, from: RecordLink<Type>, to: RecordLink<Type> }` |
| `ConstDraft` | `ty: RecordLink<Type>`, `value: i128` |
| `FunctionDraft` | `symbol: RecordLink<Symbol>`, `signature: RecordLink<Type>`, `entry: RecordLink<Block>`, `linkage` |
| `BlockDraft` | `function: RecordLink<Function>`, `ordinal` (instructions derived) |
| `ValueDraft` | `ty: RecordLink<Type>` |
| `InstructionDraft` | `op`, `block: RecordLink<Block>`, `operands: Vec<RecordLink<Value>>`, `immediate: Option<RecordLink<Const>>`, `result: Option<RecordLink<Value>>` |
| `ArtifactDraft` | `kind: ArtifactKind`, `source: Option<RecordLink<Source>>` (**total** `requires_map`: map-mandatory Normalized/Spliced/CommentFree/Preprocessed require `Some`; map-optional Assembly/Object/Snapshot/Trace accept a validated `Some` or `None` — rev-47 delegated candidate default), `bytes: Vec<u8>`, `raw_offsets: Vec<u64>` (len = bytes+1 for map-mandatory, empty for map-optional; full map rule in §5) |
| `ContinuationDraft` | **extends the existing `ContinuationRecord`** (no competing `ParseContinuation` struct): `production: TaskKind` (renamed from `resume_kind`), `cursor: RecordLink<Token>`, `context`, `binding_power: u16`, `scope: Option<RecordLink<Scope>>`, `parent: Option<RecordLink<Node>>`, `partial_children: Vec<RecordLink<Node>>`, `next_child_ordinal: u32`, `previous: Option<RecordLink<Continuation>>` (committed, acyclic). **No `awaited` field (rev 21): the awaited children live only in `TaskState::Waiting(WaitSet)`.** |

`RecordDraft::links()` is **recursive over nested draft links**: it enumerates
every reference field, including the two `from`/`to` type links inside each
`ConversionPlanDraft` nested in `SemDraft.conversions`, and every link in
`partial_children`. A nested type link is validated exactly like a
top-level link (family, committed-existence, or same-batch draft resolution).
`[T07]` `ContinuationRef`/`ChildRef` are **not** `RecordLink`s; they are
validated separately in the no-mutation pass before apply (rev 21, §5/§7).

Source/TU rule (generalized one-hop, `source_scoped_one_hop`). A task is
source-scoped iff, after the batch scan, any of these known span-bearing draft
fields is present: `SpanDraft` (direct), `ExpansionDraft.spelling`/`expanded`,
`PpTokenDraft.span`, `TokenDraft.span`, `NodeDraft.first_token`/`last_token`
(token-range links), **or `ArtifactDraft.source`**; or any such field is a
committed link to a live `Span`/`Expansion`/`Token` (direct dereference only — a
committed `Node`'s range is **not** recursively walked). A source-scoped task must
carry exactly one
`RecordRef::Source` in `task.payload` (0 -> `SourceNotDeclared`, >1 ->
`MultipleSourceRefs`); it must be live; and every directly referenced
Span/Expansion (draft or committed) must resolve to that `SourceId`
(`SpanSourceMismatch`). In addition, an `ArtifactDraft.source` that is `Some`
must equal that declared `SourceId` (`ArtifactSourceMismatch`) — a map-optional
artifact may carry a validated `Some(source)` or `None` (rev-47 delegated
candidate default) — and a map-mandatory artifact kind with no source is
`ArtifactSourceMissing`.
`NameDraft` is T04-only and job-global. No arbitrary graph recursion.
`[audit2 N7, audit5 C, audit6 C1, T03 R4, T03 rev18]`

> **Rev 22 limit (T03/T04 finding 2) — the rule is one-source, not
> multi-source.** It requires exactly one `RecordRef::Source` and cannot model
> include expansion across multiple `SourceId`s (T03 `PP17 IncludeResolveChip` /
> `PP18 IncludeEnterExitChip`) or a multi-source `Preprocessed` artifact. **M1
> scope is explicitly the single-file fixture** (`M1-SRC-000`, no `#include`), so
> the rule is exact for M1 and **incomplete as a general contract**. The general
> solution is a typed multi-source map (a committed source-map/provenance record
> keyed over a set of `SourceId`s) with the artifact map defined over it; this is
> a **T03/integrator `/6` blocker**, and it is not silently generalized here.
> Alternatives: (a) per-include `RecordRef` sets in the task payload with
> ownership checks; (b) a committed `SourceMap`/`OriginChain` record family
> (adds a family/arena). Neither is selected.

### 6.2 Envelope types

```rust
// PROPOSAL ONLY.

/// Closed family tag, total over every `RecordRef` variant (24 existing +
/// `Sem` + `ScopeEvent` + `Literal` = 27 variants). **The enum ordinals here and
/// the `RecordRef` wire tags are two distinct closed inventories** (finding 7):
/// this enum's discriminants are **proposed to be** pinned separately from wire
/// tags 0–26, and `M1AppendSchema` is **proposed to hash** both at `/6` (not
/// pinned/hashed today — rev 25 F1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecordFamily {
    Source, Span, Expansion, Name, PpToken, Token, Literal, Node, Scope, ScopeEvent, Symbol,
    Type, Sem, Const, Layout, Init, Function, Block, Value, Instruction, VReg,
    Continuation, Task, Result, Diagnostic, HostRequest, Artifact,
}
// 27 variants (24 existing + Sem + ScopeEvent + Literal).
impl RecordRef {
    pub const fn family(self) -> RecordFamily;                 // exhaustive
    pub const fn make(family: RecordFamily, index: u32) -> RecordRef;  // exhaustive
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DraftRef(pub u32);                                   // scoped to one task's AppendBatch

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkTarget { Committed(RecordRef), Draft(DraftRef) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordLink { pub expect: RecordFamily, pub target: LinkTarget }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordDraft {
    Span(SpanDraft), Expansion(ExpansionDraft), Name(NameDraft),
    PpToken(PpTokenDraft), Token(TokenDraft), Literal(LiteralDraft),
    Node(NodeDraft), Scope(ScopeDraft),
    ScopeEvent(ScopeEventDraft), Symbol(SymbolDraft), Type(TypeDraft), Sem(SemDraft),
    Const(ConstDraft), Function(FunctionDraft), Block(BlockDraft), Value(ValueDraft),
    Instruction(InstructionDraft), Artifact(ArtifactDraft), Continuation(ContinuationDraft),
}

impl RecordDraft {
    pub fn family(&self) -> RecordFamily;
    pub fn target(&self) -> (StoreId, &'static str);
    /// Every reference field, **recursively** over nested draft links (including
    /// the `from`/`to` links inside each `ConversionPlanDraft` nested in
    /// `SemDraft.conversions`). Validation only.
    pub fn links(&self) -> Vec<RecordLink>;
    /// Total after validation; task-scoped. All links already checked against
    /// `table`/`names` for this task's batch.
    pub fn resolve(&self, task: TaskId, table: &ResolveTable, names: &NamePlan) -> ResolvedDraft;
    /// Canonical wire bytes for the snapshot (deterministic, no addresses).
    pub fn encode(&self, w: &mut Writer);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendBatch { pub records: Vec<RecordDraft> }        // empty is rejected

/// (TaskId, DraftRef) -> (family, resolved ref). Built in the no-mutation pass.
pub struct ResolveTable { /* BTreeMap<(TaskId, u32), (RecordFamily, RecordRef)> */ }

/// Global commit-order name plan (see §7 phase 2a).
pub struct NamePlan {
    pub existing: std::collections::BTreeMap<Vec<u8>, NameId>,
    pub new_names: Vec<Vec<u8>>,      // first-seen order; predicted id = from_index(intern.len()+k)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedDraft {
    Span(SpanRecord), Expansion(ExpansionRecord),
    Name { bytes: Vec<u8>, existing: Option<NameId> },   // None => create one intern entry
    PpToken(PpTokenRecord), Token(TokenRecord), Literal(LiteralRecord), Node(NodeRecord), Scope(ScopeRecord),
    ScopeEvent(ScopeEventRecord), Symbol(SymbolRecord), Type(TypeRecord), Sem(SemRecord),
    Const(ConstRecord), Function(FunctionRecord), Block(BlockRecord),
    Value(ValueRecord), Instruction(InstructionRecord), Artifact(ArtifactRecord),
    Continuation(ContinuationRecord),
}

/// Per-final-record deterministic encoders for the snapshot. The serializer
/// calls the matching encoder for each *arena entry* (a final record), **not**
/// the commit-time `ResolvedDraft`. `RecordDraft::encode` (above) is the separate
/// wire encoding for `AppendRecords` proposals. `Name` is not an arena body; the
/// intern table is encoded by the existing intern serialization.
pub fn encode_span(r: &SpanRecord, w: &mut Writer);
pub fn encode_expansion(r: &ExpansionRecord, w: &mut Writer);
pub fn encode_pp_token(r: &PpTokenRecord, w: &mut Writer);
pub fn encode_token(r: &TokenRecord, w: &mut Writer);
pub fn encode_literal(r: &LiteralRecord, w: &mut Writer);
pub fn encode_node(r: &NodeRecord, w: &mut Writer);
pub fn encode_scope(r: &ScopeRecord, w: &mut Writer);
pub fn encode_symbol(r: &SymbolRecord, w: &mut Writer);
pub fn encode_type(r: &TypeRecord, w: &mut Writer);
pub fn encode_sem(r: &SemRecord, w: &mut Writer);
pub fn encode_const(r: &ConstRecord, w: &mut Writer);
pub fn encode_function(r: &FunctionRecord, w: &mut Writer);
pub fn encode_block(r: &BlockRecord, w: &mut Writer);
pub fn encode_value(r: &ValueRecord, w: &mut Writer);
pub fn encode_instruction(r: &InstructionRecord, w: &mut Writer);
pub fn encode_scope_event(r: &ScopeEventRecord, w: &mut Writer);
pub fn encode_artifact(r: &ArtifactRecord, w: &mut Writer);   // includes raw_offsets
pub fn encode_continuation(r: &ContinuationRecord, w: &mut Writer);

/// Exhaustive mapping from a draft family to its **backing** `TypedArena`
/// capacity. Returns `None` for `Name` (intern table, not an arena) and for any
/// family that has no M1 draft. This is the only correct key for the per-arena
/// preflight: `StoreId` is *not* 1:1 with an arena. The preflight loop only sees
/// families that have a draft (`Name` is excluded), so it must map the 18 M1
/// arena families to `Some` and treat any `None` as `InternalMissingArena`.
pub fn arena_allocated(bus: &CompilerBus, family: RecordFamily) -> Option<u32>;

/// The single family -> arena push used by the infallible apply pass. Exhaustive
/// over the 18 M1 arena families; `Name` is handled by `intern_reserved`.
pub fn push_arena(bus: &mut CompilerBus, family: RecordFamily, body: ResolvedDraft);

/// A single chip output. `RestrictedChip::Output` may be this type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalBatch { pub proposals: Vec<Proposal> }
```

Backing-arena mapping for capacity (BLK-1). Each draft family maps to exactly one
backing `TypedArena`; `Name` maps to the `InternTable` (no arena). The preflight
uses `arena_allocated(family)` and the apply uses the same match:

| Draft family | Backing arena | `(StoreId, field)` label |
|---|---|---|
| Span | `Arenas.spans` | `(Sources, "spans")` |
| Expansion | `Arenas.expansions` | `(Sources, "expansions")` |
| PpToken | `Arenas.pp_tokens` | `(Pp, "tokens")` |
| Token | `Arenas.tokens` | `(Lex, "tokens")` |
| Literal | `Arenas.literals` (new) | `(Lex, "literals")` |
| Node | `Arenas.nodes` | `(Parse, "nodes")` |
| Scope | `Arenas.scopes` | `(Symbols, "scopes")` |
| Symbol | `Arenas.symbols` | `(Symbols, "symbols")` |
| Type | `Arenas.types` | `(Types, "records")` |
| Sem | `Arenas.sem` | `(Sem, "records")` |
| Const | `Arenas.consts` | `(Constants, "records")` |
| Function | `Arenas.functions` | `(Ir, "functions")` |
| Block | `Arenas.blocks` | `(Ir, "blocks")` |
| Value | `Arenas.values` | `(Ir, "values")` |
| Instruction | `Arenas.instructions` | `(Ir, "instructions")` |
| ScopeEvent | `Arenas.scope_events` (new) | `(Symbols, "scope_events")` |
| Artifact | `Arenas.artifacts` (existing) | `(Artifacts, "fragments")` |
| Continuation | `Arenas.continuations` (existing) | `(Tasks, "continuations")` |
| Name | `InternTable` (not an arena) | `(Names, "entries")` |

`StoreId` is **not** 1:1 with an arena: `Sources` backs the source bytes plus
spans/expansions, `Symbols` backs scopes/symbols, and `Ir` backs four arenas. The
`max_records_per_arena` bound is therefore checked per **family** (backing arena)
and never per `StoreId`:
`checked_add(arena_allocated(family), new_body_count(family)) <=
max_records_per_arena` (a `None` mapping is `InternalMissingArena`). `Name` is
excluded from `counts`/`arena_bodies` and bounded only by
`max_intern_entries`/`max_intern_bytes`. `[audit4 BLK-1, audit5 E]`

`ProposalBatch.proposals.len()` is bounded by `limits.max_proposals_per_tick`;
the commit additionally enforces `proposals.len() + total_drafts <=
max_proposals_per_tick`. `[audit2 N14]`

### 6.2.1 Pipeline scheduling registers (proposed; user binding direction)

The staged pipeline is part of the envelope, not a driver-only concern. It adds
**no** `RecordRef` family and **no** new record body; it adds explicit bus
registers and a versioned stage-assignment section **proposed to be hashed at
`/6`** (mirroring
[ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) §2).

```rust
// PROPOSAL ONLY.
/// Closed stage tag. Ordinals are **proposed** to be pinned with
/// `stage_assignment_version` at `/6`; not frozen today.
pub struct StageId(pub u16);
pub const STAGE_NAMES: &[&str] = &[
    "source_pp", "lex", "parse", "symbol_type", "sem", "const", "ir",
    "machine", "artifact",
]; // exact set/names RESIDUAL until /6

/// Versioned, total mapping (kind -> stage ordinal), **proposed to be hashed at
/// `/6`** (not hashed today). Re-frozen, not mutated.
pub struct StageAssignment { pub version: u32, /* kind -> StageId */ }

/// One selected task. `dispatch_ordinal` is its position in the batch (0-based),
/// so the routed worker invocation is addressable without a global cursor.
pub struct SelectionEntry {
    pub task: TaskId, pub chip: ChipId, pub layer: u16, pub dispatch_ordinal: u32,
}
/// **Canonical selection wire value at every quota**: a bounded, ordered batch
/// written by **CT03's adapter only** to `wires.selection`. At quota 1 it holds
/// at most one entry; at quota `> 1` the ordered dispatched set. `len <=
/// max_inflight_per_tick` is enforced pre-dispatch. **`SelectionBatchOverflow`
/// and `DuplicateSelection` are dispatcher/scheduling failures (pre-dispatch,
/// before any store mutation), not `CommitError`s (rev 21).**
pub struct SelectionBatch { pub entries: Vec<SelectionEntry> }

/// Snapshot-encoded delivery metrics. **Canonical location selected:** encoded
/// as `bus.report.metrics` (a dedicated register inside `report`), not a
/// floating view; the exact inner encoding is a `/6` item.
pub struct PipelineMetrics {
    pub ticks: u64, pub dispatches: u64,
    pub dispatches_this_tick: u32,
    pub per_stage_dispatches: Vec<u64>,     // indexed by stage ordinal
    pub per_stage_queue_hwm: Vec<u32>,
    pub queued_per_stage: Vec<u32>,
    pub inflight_hwm: u32,
    pub commits: u64, pub rejected_batches: u64, pub idle_ticks: u64,
    pub backpressure_events: u64, pub cancellations: u64,
    pub ticks_at_quota: Vec<u64>,           // histogram indexed by quota value
}
```

- **Canonical queue source (finding 8):** `stage_queues[stage]` are the
  **single canonical** bounded ready queues (one `Vec<TaskId>` per stage ordinal,
  each bounded by `stage_queue_bound[stage]`). `tasks.ready` is a **derived
  read-only quota-1 compatibility view** and is never a second write target:
  `Progress`/resume/enqueue reinsert into `stage_queues[stage_of(task.kind)]`,
  and the view is rebuilt deterministically. A second writer to `tasks.ready` is
  rejected by the manifest (`StoreOwnerViolation`) at `/6`.
- `tasks.in_flight: Vec<TaskId>` (bounded, ordered) is the **ephemeral per-tick
  scheduler batch only** (rev 33 H9): it is populated by the dispatcher from the
  tick's `SelectionBatch`, used to address each worker invocation, and cleared at
  latch **only after every dispatched task has a terminal/`Waiting`/`Progress`
  outcome** (or the H6 recovery), so no residual in-flight set survives.
  `tasks.active`/`control.selected` are the **quota = 1 compatibility
  projections**, not retired.
- `wires.selected: Option<TaskId>` is a **singular, optional compatibility
  projection** of the batch, not the canonical selection; the canonical
  `TickRecord` field is the ordered `dispatched: Vec<TaskId>`. At quota 1 only,
  `wires.selected = dispatched.first()`; at quota `> 1` it is `None` (or the
  first entry only if a consumer explicitly opts into the projection). The
  proposed `Selection`/`TickReport`/`Resolution` types in `report.rs` **do not
  exist today** (crate has no `report.rs`); they are proposed relocations, not
  existing types.
- **`dispatch_cursor` semantics (finding 7):** `control.dispatch_cursor: u32`
  is the 0-based **position of the task whose worker is currently invoked** in
  `tasks.in_flight` (equivalently the current `SelectionEntry.dispatch_ordinal`).
  It is set immediately before ticking that task's routed worker layer and is
  valid only for that invocation (the projection maps
  `tasks.in_flight[dispatch_cursor]` to one task); it is cleared after each
  worker invocation and at latch. The worker sees exactly one task.
- `stage_assignment_version` and the `(TaskKind → StageId)` table are **proposed
  to be** pinned in `M1AppendSchema` at `/6` (not pinned today — rev 25 F1);
  changing a kind's stage is a normative re-freeze.
- **Single writer per wire (rev 21):** `wires.selection: SelectionBatch` is
  written by **CT03's adapter only**; `wires.selected: Option<TaskId>` is written
  by the **dispatcher only** (quota-1 projection); `wires.proposals` is written by
  each routed worker's adapter only; `bus.report` is written by the driver's
  **step-6 sole append site only**. The canonical report shape is
  `TickRecord { dispatched: Vec<TaskId>, metrics: PipelineMetrics, selected:
  Option<TaskId> }` with `dispatched` canonical and `selected` derived.
- **Bounds enforced pre-mutation; all quotas before any state mutation (rev 21;
  rev 33 H9):** `entries.len() <= max_inflight_per_tick` (`SelectionBatchOverflow`),
  `dispatch_count <= max_dispatches_per_tick` (`DispatchBudgetExceeded`),
  `queue_after[stage] <= stage_queue_bound[stage]` (`BackpressureCapacity`), and
  `proposals.len() + total_drafts <= max_proposals_per_tick`. The
  queue/dispatch/proposal bounds are checked in the **no-mutation pass** before
  apply; the selection-size/duplicate checks are checked by the dispatcher before
  any task is marked `Running`. All use checked/saturating arithmetic; exhaustion
  is a structured error, never a panic or silent drop. **Classification (rev 21):
  `BackpressureCapacity` is a `CommitError`; `SelectionBatchOverflow`/
  `DuplicateSelection`/`DispatchBudgetExceeded` are
  dispatcher/scheduling failures, not `CommitError`s; `StageUnassigned`/
  `StageLayerMismatch`/`StoreOwnerViolation` are `ManifestError`s.**
  **[Rev-51/54 note: `DispatchBudgetExceeded` above is the historical rev-21/22
  name, dropped from the candidate with `max_dispatches_per_tick` (sole bound
  `max_inflight_per_tick`); see the rev-38 note below. Not a `/6` candidate.]**
  **Rev 38 (CDR rev 51 / ADR-0002 Revision 17 / T02 rev 31; §23.1):** the
  rev-21/22 candidate `max_dispatches_per_tick` bound (and its
  `DispatchBudgetExceeded`) is **dropped from the candidate limit inventory and
  validation** by a **delegated candidate default** — `max_inflight_per_tick`/quota
  is the **sole per-tick dispatch-count bound**; **not** an owner/T01 `[INT]`
  signoff, **not** a freeze, and the H9 residual proof stays open. The historical
  candidate text above is preserved, not overwritten.
  **Rev 23 (H9) — the in-flight scheduling ownership was BLOCKED, not
  resolved.** The **rev-21/22 candidate** had committed the dispatcher to a
  single-phase `max_inflight_per_tick` check plus a separate residual
  `max_inflight_total` bound (`InflightQuotaExceeded`). **Rev 33 (user direction in
  principle, 2026-10-04; H9):** the user accepts **removing `max_inflight_total`**
  from the `/6` candidate because sequential per-tick dispatch is already bounded
  by `max_inflight_per_tick`, the per-stage `stage_queue_bound`, and
  `max_tasks_total`, and `TaskState::Waiting` is not in-flight. The dispatch batch
  bound remains the per-tick `max_inflight_per_tick` quota; `tasks.in_flight`
  remains an **ephemeral per-tick scheduler batch only** (candidate), cleared at
  latch only after every dispatched task has a terminal/`Waiting`/`Progress`
  outcome (or H6 recovery), so **no residual in-flight set survives the latch**.
  The dispatcher's **pre-worker `Ready → Running`** mutation is distinguished from
  the **one ordered atomic semantic commit**. This is a `/6` **working-basis
  direction only, pending T01 integrator acceptance**; no bound/config/hash is
  frozen, and the remaining H9 `[INT]` items (the dispatcher `Ready→Running`-vs-
  ordered-atomic-commit relationship and the residual-set semantics) stay open
  (§18.6).
- **Canonical queues exclusion (rev 21):** the canonical `stage_queues` already
  exclude the tasks the dispatcher removed this tick; the phase-3 bound must
  therefore **not** subtract them again.

### 6.2.2 Extended deterministic commit order

The commit order is the T01 §4 order extended with the **dispatch ordinal** so a
multi-inflight tick is still totally ordered:

```text
ordered = sort by (dispatch_ordinal, enqueue_ordinal, TaskId, proposal_index)
```

`dispatch_ordinal` is the position of the task in the tick's ordered
`SelectionBatch` (itself ordered by `(stage ordinal, phase priority, enqueue
ordinal, TaskId)`). A cross-task batch-scoped conflict is therefore rejected in
this canonical order, independent of any internal iteration order
(`commit.cross-task-check-order`). `[ADR-0002 §2.1, user pipeline direction]`

### 6.3 Proposal/result extensions

```rust
// PROPOSAL ONLY.
/// Reference to a continuation from an `Enqueue`: committed, or an own-task
/// batch draft key. **Accepted in principle (rev 30, user decision A):** the
/// own-task-batch key is a transient wire/proposal input only and is resolved to
/// a committed `ContinuationId` in commit phase 4 before any persistent state; no
/// durable cursor/WaitSet/join points at a draft/wire/address. Guardrails §6.1
/// text is unamended; T01 implementation-confirmation pending (§17.7 PIPE-5).
pub enum ContinuationRef { Committed(ContinuationId), OwnBatch(DraftRef) }
/// A child task awaited by the same task: a committed TaskId or an index into
/// this task's own `Enqueue` list (same-task only).
pub enum ChildRef { Committed(TaskId), OwnBatch(u32) }

pub enum Proposal {
    Enqueue(TaskDraft),                                   // TaskDraft.continuation: Option<ContinuationRef>
    Complete { task: TaskId, value: ResultValue },
    Fail { task: TaskId, diagnostic: DiagnosticDraft },
    AwaitHost { task: TaskId, request: HostRequestDraft },
    AwaitChildren { task: TaskId, children: Vec<ChildRef> },  // joins; sets Waiting(WaitSet)
    StorePatch(StorePatch),
    AppendRecords { task: TaskId, batch: AppendBatch },   // at most one per task per batch
    Progress { task: TaskId, ordinal: u64 },              // reschedule for tick+1
}
pub enum ResultValue {
    Empty, Ack, Record(RecordRef), Records(Vec<RecordRef>), Diagnostic(DiagnosticId),
    DraftRecords(Vec<DraftRef>),   // resolved to Records in the same commit
}
```

Coexistence semantics `[audit2 N14]`: per dispatched task per batch, exactly one
of `{Complete, Fail, AwaitHost, AwaitChildren, Progress}`; at most one
`AppendRecords`; `Enqueue` may accompany any of the five (a task may spawn
children and then await them); `StorePatch` may accompany a transition;
`AppendRecords` and `StorePatch(Append)` may not target the same
`(store, field)`; `Progress`/`AwaitChildren` are incompatible with
`Complete`/`Fail`/`AwaitHost`. `AwaitChildren.children` `OwnBatch(k)` must index
this task's own `Enqueue` list (`k < own Enqueue count`, same task); a
cross-task index is unrepresentable. `TaskDraft.continuation` is
`Option<ContinuationRef>`; `Task.continuation` remains a committed
`Option<ContinuationId>` after resolution. `Payload` stays `RecordRef`-only
(children enqueue next tick and reference committed records). Cross-task
**committed** references are normal; cross-task **draft** references are
impossible.

### 6.4 Structured errors (classified)

Every failure is classified into exactly one carrier (cross-cutting finding):

- **`CommitError`** — reject-before-apply invariants checked by the commit path
  (below).
- **Chip `DiagnosticDraft`** — emitted by a chip through `Fail`/the diagnostic
  proposal path, never a `CommitError`: `ConstOverflow`, `ConstUnsupported`,
  `ParseCursorDidNotAdvance`, `ParseDepthExceeded`, `UnsupportedIrOp`,
  `UnsupportedNode`, and every language-rule violation owned by a stage chip.
- **`ManifestError`** — emitted by `ManifestRegistry::register` (not commit):
  `StoreOwnerViolation { chip, store, field, expected_kind }`, `StageUnassigned`,
  `StageLayerMismatch`. These are registration-time and are **proposed to be
  hashed in `M1AppendSchema` at `/6`** (not hashed today — rev 26), never runtime `CommitError`s.
- **`ConfigError`** — construction/validation (separate family, §10.2/§12.8).

```text
# --- CommitError: commit-path reject-before-apply (stable codes at /6) ---
AppendNotDeclared { chip, store, field }
AppendFieldNotDeclared { store, field }
AppendNotPermittedForKind { chip, kind }
DuplicateAppendBatch { task }
EmptyAppendBatch { task }
DuplicateAppendMechanism { task, store, field }
DraftRefOutOfRange { task, ordinal }
DraftFamilyMismatch { task, ordinal, expected, found }
CommittedRefMissing { task, reference }
CommittedFamilyMismatch { task, reference, expected }
SourceNotDeclared { task, reference }
MultipleSourceRefs { task, count }
SpanSourceMismatch { task, expected, found }
ResultDraftRefWithoutBatch { task, ordinal }
DuplicateTaskTransition { task }
TaskNotTransitioned { task }
AwaitChildrenRefInvalid { task, ordinal }          # OwnBatch index >= own Enqueue count
ContinuationRefInvalid { task, reference }         # Committed missing / OwnBatch out of range (rev 22, T05 finding 1)
NonAdvancingProgress { task, previous, ordinal }
ProgressLimit { task, limit }
DraftCapacity(ArenaError)
InternCapacity(InternError)
InternalMissingArena { family: RecordFamily }
ArtifactMapInvalid { task, artifact, reason }
ArtifactSourceMissing { task, artifact }
ArtifactSourceMismatch { task, artifact, expected, found }
SymbolConflict { task, name, scope, kind }               # exact structural duplicate only
# RedeclarationConflict is NOT a CommitError (rev 22, T06 finding 3): it is a
# chip diagnostic emitted by the declaration/redeclaration chip through the
# Fail/DiagnosticDraft path (see §6.4 classification and §13). Removed from this
# block.
EffectMaskUnsupported { task, mask }
ScopeLifecycleViolation { task, scope, reason }
DuplicateSemRecord { task, node }
DuplicateConversionRole { task, node, role }   # rev 22, T07 finding 5: at most one plan per (node,role)
# (A *missing required* conversion is a VF06 completeness failure, not a CommitError.)
NodeCycle { task, node }
NodeOrdinalNotUnique { task, parent, ordinal }
ContinuationCycle { task, continuation }
ValueProducerMissing { task, value }
ValueProducerDuplicate { task, value }
BlockMissing { task, block }
BlockTerminated { task, block }
TerminatorMissing { task, block }
OpArityMismatch { task, op, expected, found }
OpImmediateMismatch { task, op, reason }
OpImmediateTypeMismatch { task, op, expected, found }
OpResultMismatch { task, op, reason }
TerminatorNotLast { task, block }
# --- cross-task (batch) conflict predicate, phase 1 (finding 9) ---
CrossTaskWriteConflict { task_a, task_b, store, field, record }
# --- pipeline scheduling (ADR-0002; stable codes fixed at /6) ---
# CommitError (commit-path reject-before-apply):
BackpressureCapacity { stage, reason }    # enqueue/queue bound, phase 3
# ManifestError (registration-time; listed for inventory only):
StageUnassigned { kind }
StageLayerMismatch { kind, stage, layer }
# Dispatcher/scheduling failures (pre-dispatch, before any store mutation;
# NOT CommitError and NOT ManifestError):
# rev 33 (H9): `InflightQuotaExceeded` is REMOVED from the candidate with
# `max_inflight_total` (per-tick dispatch is bounded by `max_inflight_per_tick`,
# the stage queues, and `max_tasks_total`; `Waiting` is not in-flight; the
# ephemeral `tasks.in_flight` clears at latch). Retained here only as the
# historical rev-21/22 name; pending T01 integrator acceptance.
# rev 51/54: `DispatchBudgetExceeded` is likewise DROPPED from the candidate
# with `max_dispatches_per_tick` (`max_inflight_per_tick` is the sole per-tick
# dispatch-count bound); retained here only as the historical rev-21/22 name.
# HISTORICAL-ONLY (rev-51/54): the line below is not an operative candidate.
DispatchBudgetExceeded { limit }
DuplicateSelection { task }
SelectionBatchOverflow { limit }
```

The last group is the **pipeline** family; numeric codes are fixed at `/6`.
`StageUnassigned`/`StageLayerMismatch`/`StoreOwnerViolation` are **classified as
ManifestError** (registration-time); `BackpressureCapacity` is the only new
**CommitError**; the remaining dispatcher/scheduling names are a **separate
pre-mutation failure category**, not `CommitError`s (rev 21). **Rev 33 (H9):**
`InflightQuotaExceeded` is **removed** with `max_inflight_total` (clean dispatch
bound remains `max_inflight_per_tick`), pending T01 integrator acceptance. None is
listed in
the frozen hash unless `M1AppendSchema` explicitly lists it (§12.12); the names
above are proposal vocabulary, and the set proposed to be hashed at `/6` is exactly the
`NORMATIVE_RULE` ids plus the enum-code inventory recorded in
`M1AppendSchema`.

`ForeignDraftRef` is dropped: with `DraftRef` scoped to one task it is
unrepresentable. (No orphan doc comment — the earlier misplaced comment is
removed.) `[audit2 N7]`

---

## 7. Commit algorithm (proposed)

Extend `commit_proposals` (`compiler/src/commit.rs`), preserving its
all-or-nothing goal and the T01 §4 order extended with the **dispatch ordinal**
(§6.2.2). Validation and reference resolution are fully separated from mutation;
apply performs only infallible pushes and state transitions.

```text
# deterministic order across all dispatched tasks' proposals
ordered = sort by (dispatch_ordinal, enqueue_ordinal, TaskId, proposal index)

# budgets
check_proposal_budget(proposals.len() + total_drafts, limits.max_proposals_per_tick)
# runtime dispatched set (`dispatched_tasks`) is the ordered in-flight set at
# quota > 1, or the singleton `{tasks.active}` at quota == 1 (baseline). The
# one-transition check below runs for every dispatched task even when
# `proposals` is empty. (Audit H2: this definition is used everywhere, not only
# in the header.)
# ---- phase 1: structural validation (no mutation) ----
appended: Map<TaskId, BatchIdx>                 # one AppendRecords per task
transitions: Map<TaskId, TransitionKind>         # exactly one per dispatched task
counts: Map<RecordFamily, u32>                    # arena-body drafts only (Name excluded)
# Cross-task write-conflict predicate (finding 9; rev 21 scope). A write key is
#   WriteKey = (StoreId, field, RecordScope)
# where RecordScope::Field is used for an append to a declared append-only field,
# and RecordScope::Record(RecordRef) is used for a patch that targets an existing
# record (StorePatch.target for Replace/Tombstone). Two different tasks in one
# batch writing the same key CONFLICT (CrossTaskWriteConflict) unless BOTH hold:
# (i) the field is declared append-only in M1AppendSchema, and (ii) the two
# appends are semantically independent under that field's hashed
# `commutative`/`independent` rule. This predicate is for two tasks of the SAME
# owner group appending one append-only field, or two patches; it is NOT a
# mechanism to bless a second owner group (that is `StoreOwnerViolation`). A patch
# to the same RecordRef always conflicts (no commutative patch rule in M1). The
# commit hashes the canonical ordered append, so a permitted same-owner append is
# still deterministic.
write_keys: Map<WriteKey, (TaskId, WriteKind)>
for tagged in ordered:
    task Running; chip owns task; manifest registered; kind accepted     # existing
    match proposal:
      AppendRecords{task,batch}:
        if batch.records.is_empty(): EmptyAppendBatch
        if appended.contains(task): DuplicateAppendBatch
        appended.insert(task, idx)
        for d in batch.records:
            (store,field) = d.target()
            manifest.declares_write(store,field)?                        # field-scoped
            schema.has_field((store,field))?                             # M1 schema, §12
            key = (store, field, Field)
            if let Some((other, _)) = write_keys.get(key), other != task:
                if !(append_only(store,field) && independent(store,field)):
                    CrossTaskWriteConflict { task_a: other, task_b: task, .. }
            write_keys.insert(key, (task, Append))
            if d.family() != RecordFamily::Name: counts[d.family()] += 1
            if any StorePatch in same task/batch targets (store,field): DuplicateAppendMechanism
        # ... (source-scope scan unchanged) ...
        if source_scoped_one_hop(batch, task.payload): require_unique_source(task.payload, source)
      StorePatch(p):
        existing validate_patch (version, owner, field, shape)
        key = (p.store, p.field, Record(p.target or private key))
        if let Some((other, _)) = write_keys.get(key), other != task:
            CrossTaskWriteConflict { task_a: other, task_b: task, .. }
        write_keys.insert(key, (task, Patch))
      Progress|Complete|Fail|AwaitHost|AwaitChildren:
        if transitions.insert(task, kind) already present: DuplicateTaskTransition
      Enqueue: existing validate (payload RecordRef-only)
        # rev 21: validate TaskDraft.continuation ContinuationRef:
        #   Committed(id) -> ContinuationId exists & live
        #   OwnBatch(k)   -> k in this task's own AppendRecords draft range & family
        #                    Continuation
        #   violation -> ContinuationRefInvalid (batch rejected pre-apply)
      AwaitChildren: (also) validate each ChildRef (rev 21, pre-apply):
        #   Committed(id) -> committed child of this task
        #   OwnBatch(k)   -> k < this task's own Enqueue count (AwaitChildrenRefInvalid)
# exactly one transition for each dispatched task, even with an empty proposal
# vector (not only for tasks that appended):
for task in dispatched_tasks(bus):          # in-flight set (quota>1) | {active} (quota=1)
    if !transitions.contains(task): TaskNotTransitioned(task)   # batch rejected atomically

# (Cumulative cross-record semantic validation is **phase 2b**, AFTER resolve;
#  it never runs before phase 2 and never over unresolved drafts -- cross-cutting
#  finding: validators must name their exact phase.)

# ---- phase 2: resolve (no mutation; builds final typed records) ----
# 2a. global name plan in commit order across ALL append batches:
#     seed `existing` from the live InternTable; walk every batch in `ordered`
#     order; first-seen unseen bytes -> new_names (predicted = from_index(base+k)).
# 2b. predicted arena ids per **backing arena / family**: next[family] =
#     arena_allocated(family); `RecordRef::make(family, next[family]++)`.
#     Never keyed by StoreId: Sources/Symbols/Ir are multi-arena.
# 2c. build table for all drafts; then validate every link of every draft:
#       Committed(r): r.family()==expect, r exists & live, source/TU ok
#       Draft(k):     k<len, table[(task,k)].0==expect
#     Direct source/TU checks (one hop): SpanDraft.source = Committed(Source);
#     ExpansionDraft.spelling/expanded; PpTokenDraft.span; TokenDraft.span
#     (= the committed T03 PP span); NodeDraft.first_token/last_token;
#     ArtifactDraft.source. A committed Span/Expansion/Token link is
#     dereferenced one level to its `source`; a committed Node range is checked
#     through its tokens, never recursively walked. All must equal the task's
#     declared SourceId (SpanSourceMismatch). An ArtifactDraft whose `source` is Some must
#     equal that declared SourceId (ArtifactSourceMismatch); a map-mandatory kind
#     with no source is ArtifactSourceMissing; a map-optional kind must have empty
#     raw_offsets (non-empty rejected) and may carry a valid Some(source) or None
#     (rev-47 delegated candidate default).
# 2d. resolved[(task,i)] = d.resolve(task, &table, &name_plan)      # total
# resolved_table[(task,i)] = (family, ref) for all drafts, including reused names

# ---- phase 2b: cumulative cross-record semantic validation (no mutation) ----
# Runs over committed state + this batch's **resolved** final records/refs
# (phase 2). Each rejection names a stable structured error and a §13 test.
# (a) Scope lifecycle (T06): build the per-scope event list from committed
#     ScopeEvents plus new ScopeEventDrafts in `ordered`. Scan bound: one pass
#     over that list, BTreeMap<ScopeId, ...> (never a hash map). For each scope:
#     Enter count == 1, Exit count <= 1, Exit strictly after Enter in
#     `ScopeEventId` order; every `event.at` resolves to a live committed Node
#     (or a same-batch NodeDraft resolved above); a new ScopeDraft with no Enter,
#     a duplicate scope-owner creation (a second ScopeRecord claiming the same
#     resolved owner `at` node; never keyed on `(parent, kind)` or on a live
#     arena record), or the File-scope Exit policy violation
#     is ScopeLifecycleViolation. `event.at` is validated, not ignored.
# (b) Node graph (T05): every NodeDraft.parent resolves (committed or same-batch)
#     and the parent chain is acyclic (NodeCycle); `ordinal` is unique per parent
#     **cumulatively** across committed siblings + same-batch siblings
#     (NodeOrdinalNotUnique); `partial_children`/`next_child_ordinal` are exactly
#     consistent (contiguous 0..n-1, no gap/duplicate). **H5:** node
#     `first_token`/`last_token` are **committed `TokenId`s only**; the same-batch
#     `TokenDraft` branch is removed. Each committed TokenId is dereferenced one
#     hop to its SpanId, and each token's span source must equal the Node's
#     declared source (SpanSourceMismatch). (Parent may be committed or same-batch
#     `NodeDraft`; token links may not.)
# (c) Sem (T07): exactly one SemRecord per NodeId cumulatively
#     (DuplicateSemRecord); `effects` must be the frozen empty mask or
#     EffectMaskUnsupported; `ConversionPlan.role` is required and each
#     `(node, role)` has at most one plan; the from/to type links are validated
#     recursively (via `links()`).
# (d) IR (T09): a Block referenced by instructions is committed and open
#     (BlockMissing / BlockTerminated); each ValueId has exactly one producing
#     Instruction.result (ValueProducerMissing / ValueProducerDuplicate); the
#     terminator is the greatest InstructionId in a terminated block and nothing
#     follows (TerminatorNotLast); an unterminated entry block is
#     TerminatorMissing only under the **marker-based trigger direction** (H11:
#     the rev-23 IR28 function-completion marker family/schema was unresolved; the
#     commit does not infer "function end"). **Rev 37 (rev-49 operative direction;
#     §22.4):** the trigger **reuses the committed IR28 `FunctionEnd` terminal
#     result** (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the
#     deterministic completion fact, checked by a **T01-owned typed phase-2b
#     commit-apply validation hook** (entry block terminated); **no** new marker
#     family/ID/tag/ordinal/encoder/`ResultValue`; not a frozen `/6` hook and no
#     code. op arity/immediate/result/terminator match
#     the **prospective `/6`** op table (OpArityMismatch / OpImmediateMismatch /
#     OpImmediateTypeMismatch / OpResultMismatch; H8 — no op table is hashed yet).
#     An unlisted op (UnsupportedIrOp) and an implemented node
#     with no lowering (UnsupportedNode) are **chip diagnostics**, not commit
#     errors.
# (e) Symbols (T06): the only structural conflict commit rejects is an exact
#     duplicate append keyed by `(name, scope, kind)` (SymbolConflict); general
#     compatibility and `RedeclarationConflict` are chip-reported (no M1
#     compatibility model). A lookup attributed to a different namespace is a
#     miss, not a conflict, and commit never infers an absent namespace.
# (f) Types (T06): `types.records` writers are checked against the **chip-id
#     keyed** owner allowlist at manifest registration
#     (`ManifestError::StoreOwnerViolation`); commit runs no structural dedup and
#     reuses the producer's committed id.
# (g) Continuations (T05): `previous` links are committed-only and acyclic
#     (ContinuationCycle); `partial_children`/`next_child_ordinal` consistent.
# Const arithmetic is NOT re-validated here (T08): checked arithmetic is a
# chip-level obligation; the commit never recomputes a constant value.

# ---- phase 3: capacity preflight (nothing mutated yet) ----
arena_bodies  = sum of counts                      # excludes Name
audit_patches = arena_bodies                       # one patch per new arena body
proposal_patches = number of StorePatch proposals
# per backing TypedArena (family), NOT per StoreId: Sources/Symbols/Ir are each
# multi-arena; Name is never added to `counts` (phase 1 skips it). `None` from
# arena_allocated is a structured internal error, never a silent pass.
for family, n in counts:
    used = arena_allocated(family).ok_or(InternalMissingArena { family })?
    checked_add(used, n)? <= max_records_per_arena
ensure_total_records((tasks+results+diags+requests) + proposal_patches
                     + arena_bodies + audit_patches)
# new_name_bytes = checked sum of bytes.len() over the unique NamePlan.new_names
new_name_bytes = checked_sum(NamePlan.new_names[*].len())
ensure_intern(NamePlan.new_names.len() as u32, new_name_bytes)   # names: intern limits only
# artifact map preflight (no mutation), total requires_map (rev-47 delegated
# candidate default):
#   map-mandatory kind: bytes.len() <= max_source_bytes,
#   raw_offsets.len() == checked_add(bytes.len(), 1), raw_offsets[0] == 0,
#   non-decreasing, raw_offsets[last] <= source.bytes.len(); else ArtifactMapInvalid;
#   map-optional kind: raw_offsets must be empty, else rejected; source may be
#   Some (validated/equal to the declared payload source) or None
#   (synthetic example: a Trace artifact with source = Some(declared SourceId)
#    and raw_offsets = [] is accepted; a non-empty raw_offsets is rejected).
ensure_diagnostics(new_diagnostics)
# per-stage queue / in-flight / dispatch budgets (pipeline, ADR-0002), all
# checked/saturating and enforced BEFORE any mutation; the canonical queue is
# `stage_queues`, and `progress_reinserts` counts `Progress`/resume reinserts
# into the task's OWN stage queue (never `tasks.ready`, which is a derived view):
#   queue_after[stage] = checked_add(stage_queues[stage].len(),
#                                    new_tasks[stage] + progress_reinserts[stage]
#                                    + join_reinserts[stage])   # rev 22, T05 finding 3
#   queue_after[stage] <= stage_queue_bound[stage], else BackpressureCapacity
#   # rev 33 (H9, user direction in principle, 2026-10-04): `max_inflight_total`
#   # is REMOVED from the candidate. Per-tick dispatch is bounded by the
#   # `max_inflight_per_tick` dispatch batch (checked by the dispatcher
#   # pre-dispatch as `SelectionBatch.entries.len()` -> `SelectionBatchOverflow`),
#   # the per-stage `stage_queue_bound`, and `max_tasks_total`; `Waiting` is not
#   # in-flight; and `tasks.in_flight` is an EPHEMERAL per-tick scheduler batch
#   # cleared at latch only after every dispatched task has a terminal/Waiting/
#   # Progress outcome (or H6 recovery), so no residual in-flight set survives and
#   # no separate total bound is needed. Still `[INT]`: the dispatcher's pre-worker
#   # `Ready -> Running` mutation vs the one ordered atomic semantic commit, and the
#   # residual-set semantics, remain T01 integrator decisions (see §18.6). This is
#   # a working basis only; no bound is frozen.
#   # Rev 51/54: `max_dispatches_per_tick`/`DispatchBudgetExceeded` is DROPPED from
#   # the candidate (delegated default) — `max_inflight_per_tick` is the sole per-tick
#   # dispatch bound, so `SelectionBatchOverflow` is the sole per-tick dispatch-count
#   # error. The historical `dispatch_count <= max_dispatches_per_tick,
#   # else DispatchBudgetExceeded` line below is superseded, retained as history.
#   dispatch_count <= max_dispatches_per_tick, else DispatchBudgetExceeded
#   proposals.len() + total_drafts <= max_proposals_per_tick
# At quota == 1 the single global `max_queue_len` check is the specialization;
# `stage_queues` already excludes the tasks the dispatcher removed this tick; do
# NOT subtract it again.
for Progress: ordinal > task.progress_ordinal and
              task.progress_count + 1 <= limits.max_task_progress

# ---- phase 4: apply (infallible only; no Result) ----
for (task, chip, i, d) in ordered_drafts:
    match resolved[(task,i)]:
      ResolvedDraft::Name { existing: Some(id), .. } =>     # reused: no body, no patch
          resolved_table[(task,i)] = (RecordFamily::Name, RecordRef::Name(id))
      ResolvedDraft::Name { bytes, existing: None } =>
          let id = bus.intern_reserved(bytes);              # infallible (phase 3 reserved)
          resolved_table[(task,i)] = (RecordFamily::Name, RecordRef::Name(id));   # no arena body/patch
      ResolvedDraft::Span(r)      => push_arena(StoreId::Sources, "spans", bus.arenas.spans.push(r))
      ResolvedDraft::Expansion(r) => push_arena(StoreId::Sources, "expansions", bus.arenas.expansions.push(r))
      ResolvedDraft::PpToken(r)   => push_arena(StoreId::Pp, "tokens", bus.arenas.pp_tokens.push(r))
      ResolvedDraft::Token(r)     => push_arena(StoreId::Lex, "tokens", bus.arenas.tokens.push(r))
      ResolvedDraft::Literal(r)   => push_arena(StoreId::Lex, "literals", bus.arenas.literals.push(r))
      ResolvedDraft::Node(r)      => push_arena(StoreId::Parse, "nodes", bus.arenas.nodes.push(r))
      ResolvedDraft::Scope(r)     => push_arena(StoreId::Symbols, "scopes", bus.arenas.scopes.push(r))
      ResolvedDraft::Symbol(r)    => push_arena(StoreId::Symbols, "symbols", bus.arenas.symbols.push(r))
      ResolvedDraft::Type(r)      => push_arena(StoreId::Types, "records", bus.arenas.types.push(r))
      ResolvedDraft::Sem(r)       => push_arena(StoreId::Sem, "records", bus.arenas.sem.push(r))
      ResolvedDraft::Const(r)     => push_arena(StoreId::Constants, "records", bus.arenas.consts.push(r))
      ResolvedDraft::Function(r)  => push_arena(StoreId::Ir, "functions", bus.arenas.functions.push(r))
      ResolvedDraft::Block(r)     => push_arena(StoreId::Ir, "blocks", bus.arenas.blocks.push(r))
      ResolvedDraft::Value(r)     => push_arena(StoreId::Ir, "values", bus.arenas.values.push(r))
      ResolvedDraft::Instruction(r)=> push_arena(StoreId::Ir, "instructions", bus.arenas.instructions.push(r))
      ResolvedDraft::ScopeEvent(r) => push_arena(StoreId::Symbols, "scope_events", bus.arenas.scope_events.push(r))
      ResolvedDraft::Artifact(r)   => push_arena(StoreId::Artifacts, "fragments", bus.arenas.artifacts.push(r))
      ResolvedDraft::Continuation(r)=> push_arena(StoreId::Tasks, "continuations", bus.arenas.continuations.push(r))
    # push_arena pushes the record, appends exactly one CommittedPatch for the new
    # arena body, and sets resolved_table[(task,i)] to the new RecordRef.
    Complete{value: DraftRecords(refs)}: value = Records(refs.map(|r| resolved_table[(task,r)].1))
    Progress{..}: task.progress_ordinal=ordinal; task.progress_count+=1;
                  task.state=Ready; task.ready_tick=tick+1;
                  stage_queues[stage_of(task.kind)].push(task)   # CANONICAL queue source
    # Enqueue: resolve the new task's `continuation` ContinuationRef against the
    # committed resolve table. A Committed(id) is used as-is; an OwnBatch(k) maps
    # to the ContinuationId materialized for this task's own draft k in this same
    # phase. Then push the task (visible next tick). payload stays RecordRef-only.
    # AwaitChildren{task, children}: for each ChildRef, Committed(id) is used as-is;
    # OwnBatch(k) maps to the TaskId of this task's own k-th Enqueue applied above
    # (index checked pre-apply; AwaitChildrenRefInvalid if k >= own Enqueue count).
    # Set task.state = Waiting(WaitSet { children: resolved, host_request: None });
    # there is NO continuation.awaited field (rev 21).
    # ---- join realization (rev 21/23; CT07 is decision-only) ----
    # For every committed Waiting parent (rev 23, H2/H4):
    #   - ALL of the WaitSet children Completed  -> state=Ready and reinsert into
    #     stage_queues[stage_of(parent.kind)]  (the parent TASK KIND; H2 — NOT
    #     parent.continuation.production) BEFORE latch (visible next tick);
    #   - ANY committed child Failed             -> state=Failed exactly once and
    #     NEVER Ready.
    # The realization must handle a parent whose `continuation` is absent (H2).
    # ---- OPEN-02 result delivery (selected draft direction; T01/T05/T02 /6) ----
    # The join decides on child TERMINAL STATE only; it must not consume a child
    # result and leave the parent without a durable way to obtain it next tick
    # (exactly-once consumption != exactly-once semantic processing). Selected
    # direction: consumption stays with the parent's OWN atomic commit — child
    # ResultRecords remain committed and unconsumed at the join, and the parent's
    # next-tick commit consumes each child result it reads in the same
    # commit-visible unit as its own semantic writes (`ResultAlreadyConsumed`
    # guarded), so consumption and semantic processing coincide. The join must
    # leave the child result handles durably reachable by the parent; the exact
    # carrier is a T01/T05 /6 co-freeze item (never a wire/draft/address; not
    # invented here). Alternative admissible realization (only if the /6
    # co-freeze requires consuming at join): transfer each child result at join,
    # in the same commit as the Ready transition, into explicit parent-owned
    # durable state co-frozen by T01/T05/T02. Either way, replay/retry between
    # the join and the parent execution must neither double-consume nor lose
    # results (required fixture `join_then_replay_retry`; cf. the TU stage-edge
    # consume/enqueue envelope, §24.3/OB-11).
    # The sibling policy is await-all (accepted, CDR rev 42); the fate of non-terminal
    # siblings stays a T05/T02 /6 open item (H4). This is done here in
    # commit apply (the only pre-latch mutation site), not by CT09/CT10 and never
    # via the derived `tasks.ready`.
    ... existing Enqueue/Complete/Fail/AwaitHost/StorePatch apply ...
clear control.selected = None; tasks.active = None; tasks.in_flight.clear()
# rev 33 (H9): `tasks.in_flight` is cleared ONLY AFTER every dispatched task has a
# terminal/`Waiting`/`Progress` outcome (or the H6 failure recovery has transitioned
# every dispatched task exactly once). It is an ephemeral per-tick scheduler batch:
# no residual in-flight set survives the latch, so `max_inflight_total` is removed
# from the candidate (per-tick dispatch is bounded by `max_inflight_per_tick`, the
# stage queues, and `max_tasks_total`; `Waiting` is not in-flight).
# quota == 1: `tasks.in_flight` holds at most the single `tasks.active`
# wires.selected is the compatibility projection (None at quota > 1); the
# canonical report field is the ordered dispatches. `wires.selection`/selected
# is NOT cleared here: the snapshot captures it; reset_wires clears next tick.
bump store_versions for every store that received a new arena body or a patch;
bump StoreId::Names only if >=1 genuinely new name was interned
record TickReport (ordered dispatches + metrics) into bus.report
```

Properties:

- **Pre-mutation semantic validations (T03/T04/T05/T06/T07/T08/T09).** All run in
  the no-mutation pass (phase 1 structural / **phase 2b cumulative, after
  resolve**) and reject the batch atomically: artifact map validity (mandatory:
  length == bytes+1, first == 0, non-decreasing, last <= source bytes; optional:
  empty map, with a validated `Some(source)` allowed — rev-47 delegated
  candidate default) **and artifact source equality** with the task payload
  source (`ArtifactSourceMismatch`/`ArtifactSourceMissing`); Node `parent` exists
  and the parent graph is acyclic (no self/cycle) with `ordinal` unique per parent
  **cumulatively** and **committed-only** token links (H5: the
  `first_token`/`last_token` span sources are matched, no same-batch `TokenDraft`);
  exactly one `ScopeEventRecord::Enter` and at most one `::Exit` per
  `ScopeRecord`, Exit after Enter in `ScopeEventId` order, with every `event.at`
  checked and the new-`ScopeDraft` cardinality + File-Enter policy enforced;
  exactly one `SemRecord` per `NodeId` (`DuplicateSemRecord`), `effects` empty
  (`EffectMaskUnsupported`), and `ConversionPlan.role` present with at most one
  plan per `(node, role)`; each `ValueId` has exactly one producing
  `Instruction.result`; a Block referenced by instructions is committed and open;
  a terminated block's unique terminator is the greatest `InstructionId` and
  nothing follows it; an unterminated entry block is `TerminatorMissing` only under
  the **marker-based trigger direction** (H11; the marker family/schema is
  unresolved, so the commit does not infer "function end" and the mechanism is not
  claimed fixed). **Rev 37 (rev-49 operative direction; §22.4):** the trigger
  reuses the **committed IR28 `FunctionEnd` terminal result**
  (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) with a **T01-owned
  typed phase-2b commit-apply validation hook** (entry block terminated), adding no
  new marker family/ID/tag/ordinal/encoder/`ResultValue`; not a frozen `/6` hook
  and no code. IR op arity/immediate/result/terminator match the **prospective
  `/6`**
  op table (`OpImmediateMismatch`/`OpImmediateTypeMismatch`/`OpResultMismatch`);
  the only commit-side symbol conflict is an exact `(name, scope, kind)`
  structural duplicate (general compatibility is chip-reported); `TypeRecord`
  identity is the **chip-specific** canonical-`int`/function-type single-producer
  rule with **no** structural dedup or compatibility used as identity. **Checked
  constant arithmetic and `UnsupportedIrOp`/`UnsupportedNode`/parse errors are
  chip-level diagnostics**, not commit validation (`ConstOverflow`/
  `ConstUnsupported`/`ParseCursorDidNotAdvance`/`ParseDepthExceeded` are not
  `CommitError`s). `[T03, T04, T05, T06, T07, T08, T09; cross-cutting finding]`

- **No fallible step in apply.** Every `Result`-returning check (family, range,
  existence, source/TU, capacity, intern, progress, queue) happens in phases
  1–3. `resolve` is total after validation; apply has no `Result`.
- **Audit patches only for new arena bodies.** A new arena body costs one arena
  slot and one audit patch; a reused name costs nothing; a new name costs one
  intern entry and **no** audit patch (the intern table plus snapshot is its
  provenance). `audit_patches == arena_bodies`; names are excluded from
  `max_records_per_arena` and from `max_records_total`. `[audit2 N2, N9]`
- **Per-arena capacity is keyed by the backing arena (BLK-1).** The preflight
  computes `checked_add(arena_allocated(family), new_body_count(family)) <=
  max_records_per_arena` for the 18 M1 arena families. `arena_allocated` returns
  `None` only for `Name`/non-draft families, which never enter `counts`, and a
  `None` in the loop is `InternalMissingArena`. `StoreId::Sources`, `Symbols`, and
  `Ir` each map to multiple arenas and must never be the capacity key.
  `[audit4 BLK-1, audit5 E]`
- **Name prediction is exact and idempotent (T03 R8).** `NamePlan.new_names[k]`
  is pushed by `intern_reserved` in the same global commit order as phase 2a and
  resolves to `NameId::from_index(intern.len()+k)`, so predicted and applied IDs
  match; duplicate spellings in one batch materialize exactly once (the second
  occurrence resolves to the first `NameId`). `NamePlan`/`resolve` are
  task-scoped: `RecordDraft::resolve(task, table, names)` only sees this task's
  batch.
- **`intern_reserved` is genuinely infallible.** It assigns
  `NameId::from_index(entries.len())`, inserts into `index`, and updates
  `total_bytes`; its return equals the phase-2a predicted id for that name. It is
  safe because (i) phase 3 reserved `max_intern_entries`/`max_intern_bytes` for
  exactly `new_names` (using the checked `new_name_bytes`); (ii) the commit holds
  `&mut CompilerBus` and apply runs single-threaded with no chip invocation or
  re-entry; (iii) no other reachable path mutates the intern table between
  preflight and apply. A debug assertion may re-check the reserved capacity; it
  must not be semantic control flow. A committed `Name` reference is checked via
  `InternTable::get`; `StoreId::Names` bumps only when ≥1 new name is interned,
  and a reused name produces no bump and no `CommittedPatch`. `[audit3 BL5, AD4]`
- **One transition per dispatched task.** `dispatched_tasks(bus)` is the
  ordered in-flight set (`tasks.in_flight`) at quota `> 1`, or the singleton
  `{bus.tasks.active}` at quota 1 (audit H2: same definition as the §7 header,
  the selection path, and T02/ADR-0002); any such task without exactly one
  transition — including when the proposal vector is empty — rejects the batch
  atomically (`TaskNotTransitioned`). **Verified `/5` single-task obligation
  (read from the frozen `compiler/src/routing.rs`):** on a commit failure
  `fail_selected` **always** transitions the selected task to
  `TaskState::Failed`; if diagnostic/record capacity permits it attaches a
  committed `DiagnosticId`, otherwise it uses the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel, so the task is **never** left
  stranded or `Running` (it also clears the singleton
  `tasks.active`/`control.selected`). This single-task semantic obligation is
  preserved and is not weakened.
  **The batch (`quota > 1`) recovery direction is user-selected in principle
  (rev 32, 2026-10-04; H6); the mechanism is not established and remains `[INT]`
  pending.** The rev-21 "intent" that the integration fails every dispatched task
  exactly once through a capacity-respecting failure path and therefore leaves no
  task `Running` was **unproven**; clearing the in-flight set does **not** clear
  `TaskState`/`Running`, because `TaskState` is separate record state, so removing a
  task from `tasks.in_flight` leaves any `Running` `TaskState` untouched. "The
  cleared in-flight set guarantees no task remains `Running`" is **withdrawn as
  unproven**, and **clearing the in-flight set is not itself a transition.** On
  2026-10-04 the user **selected the recovery direction in principle**: on a failed
  atomic semantic batch commit, transition **every** dispatched task, in
  **dispatch order**, **exactly once** to `TaskState::Failed` — attaching a
  committed `DiagnosticId` when diagnostic/record capacity allows, otherwise the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel — so **every** dispatched task
  leaves `Running`. This **generalizes** the verified `/5` `fail_selected`
  single-task transition with the same optional-diagnostic/sentinel semantics (a
  proposed generalization, **not** verified). **Still blocked / co-freeze:** the
  **exact atomic/bounded implementation** requires **T01 integrator approval**, and
  the batch failure-atomicity details, the diagnostic budget/state mechanism, and
  the T02/T13 owner fixtures/sign-offs remain **pending**. Any chosen design must
  uphold **no task remains `Running`, exactly-one transition per dispatched task,
  bounded diagnostics, and failure atomicity**. Only the **invariant** "no
  dispatched task remains `Running` at latch" is stated; no batch mechanism is
  claimed implemented or resolved (see §18.4 H6, §19.2, and §20). Single-task
  behavior is `/5`-verified; the batch behavior is **proposed only**. Idle (`no ready task`,
  empty in-flight set) and a foundation no-op complete without this path; an
  unregistered/unsupported kind or a kind with no handler fails explicitly (T01
  `routing.unsupported-fails`). `[audit3 BL1, audit H2; rev 21; rev 23 H6; rev 25 F2; rev 32 H6 direction in principle]`
- **Selection is integration-owned and total.** The dispatcher applies the
  ordered batch before propagation (workers read `tasks.in_flight[dispatch_cursor]`
  at quota > 1, or `tasks.active` at quota 1); the **canonical** `stage_queues`
  already exclude the tasks the dispatcher removed this tick, so the queue bound
  is `checked_add(stage_queues[stage].len(), new_tasks[stage] + progress_reinserts[stage])`
  (`tasks.ready` is a derived view, never a second write target). **At quota = 1
  only**, if the tick produces no valid transition the integration's single-task
  failure transition resolves the task (the verified `/5` `fail_selected`
  obligation above: the task **always** becomes `Failed`, with a diagnostic
  attached when capacity allows and the `TaskState::Failed(DiagnosticId::NONE)`
  sentinel otherwise, so it is never stranded). A `StorePatch`-only or empty
  proposal set is therefore not a stranded state **at quota = 1**. **At quota > 1
  this completion guarantee is BLOCKED (H6):** the batch fan-out is not an
  established mechanism (see the one-transition property above), so no
  no-`Running`/non-stranded claim is made for the batch. `[audit3 BL2, finding 8; rev 26]`
  **[Rev-42 note (user-accepted recommendation; T01 co-freeze pending; not a `/6`
  freeze; no code): the "BLOCKED" framing above is qualified by the rev-42
  selected quota>1-capable bounded recovery mechanism to be frozen in `/6`
  (pre-dispatch errors leave tasks `Ready`; failed batch commits no semantic
  state; dispatch-order `Failed` recovery; no N pre-reservation; `NONE`
  fallback; state guard); exact atomic realization + fixtures remain open.]**
- **Cross-task write conflicts (finding 9; rev 21 scope).** Two different
  dispatched tasks may write the same `WriteKey` only when the target field is
  declared append-only in `M1AppendSchema` and the appends are semantically
  independent under that field's hashed `independent`/`commutative` rule;
  otherwise the whole batch is rejected pre-apply (`CrossTaskWriteConflict`).
  **Rev 21 scope:** because each field has one owner group, this covers two tasks
  of the **same** owner group appending one append-only field (`sources.spans` is
  T03-only; `lex.tokens` T04-only; `constants.records` T08-only;
  `names.entries` idempotent; `parse.nodes` T05-only; `artifacts.fragments`
  T03-only) and **every** patch. It does **not** permit a second owner group
  (that is `StoreOwnerViolation`). A patch to the same `RecordRef` always
  conflicts (no commutative patch rule in M1). The audit owner is a **proposed**
  VF04 `AccessContractChip` extension (batch conflict check) plus VF13 replay over
  dispatch order; **the VF04 batch-conflict extension is not defined today, so it
  is a residual, not a defined verifier**; **VF12 `IrInterpretChip` is not the
  conflict verifier** and must not be cited unless explicitly extended.
  `[finding 9; rev 21]`
- **Canonical equivalence projection (audit H1).** Because the new scheduler
  snapshot encodes stage queues, the in-flight set, `dispatch_cursor`,
  `stage_assignment_version`, and `PipelineMetrics`, the quota = 1 baseline gate
  is defined as a **canonical semantic comparison projection**, not a
  byte-identical snapshot: compare semantic records, results, and diagnostics
  (the `RecordRef`-reachable typed records) and the **semantic trace** (tick,
  ordered dispatched `TaskId`s, per-task outcome, produced records, diagnostics)
  while **excluding scheduler-only registers** (stage queues, in-flight
  set/count, `dispatch_cursor`, `stage_assignment_version`, `PipelineMetrics`).
  Separately, the new scheduler snapshot must be **deterministic and
  replay-identical run-to-run** at every tested quota. This projection is the M1
  definition; a future "no new scheduler snapshot" variant is not claimed.
- **Tick-start reset, budget gate, terminal halt, and end-of-tick active state
  (BLK-2).** The root `Motherboard::clock_tick` runs `reset_wires` (hence
  `wires.selected` is `Default`); the driver then clears
  `control.selected`/`tasks.active`, applies the budget/cancel gate
  (`pins.tick_budget_reached` or `control.tick >= max_ticks` ->
  `budget_exhausted = true` + `job_state = Failed` + one terminal
  `BudgetExhausted` record; `pins.cancel` -> `Cancelled` terminal) before any
  selection, then selects. Once terminal, `CompilerDriver::step` refuses to tick
  again and returns the stored terminal record. The dispatcher writes the ordered
  `wires.selection: SelectionBatch` and, at quota 1 only, sets the compatibility
  projection `wires.selected = Some(task)`; the canonical report field is the
  ordered `TickRecord.dispatched`, so `Snapshot::capture` records the batch and
  `reset_wires` clears it next tick. After apply, the commit clears
  `control.selected`/`tasks.active` and the in-flight set. **The verified `/5`
  single-task failure path** (`routing.rs` `fail_selected`) **always** transitions
  the task to `TaskState::Failed`; it attaches a committed `DiagnosticId` when
  diagnostic/record capacity allows and otherwise the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel, so the task is not stranded.
  **The batch failure path is BLOCKED (H6, rev 25/26):** "every
  dispatched task has exactly one transition" and "no stale `Running`/active task
  survives a tick" are the stated **invariant**, not an established mechanism;
  clearing the in-flight set does not clear `TaskState`, so a `Running` task state
  can survive. The `/6` alternatives are recorded in §7 (generalize the verified
  deterministic all-dispatched fail transition with the same
  optional-diagnostic/`DiagnosticId::NONE` sentinel semantics, or another
  T01-approved atomic terminal path, e.g. reserve bounded terminal
  diagnostic slots before dispatch). No stale-`Running` guarantee is asserted for
  the batch. `[audit4 BLK-2, audit5 A, B, audit6 BLK-6.1..6.3; rev 26]`
  **[Rev-42 note (user-accepted recommendation; T01 co-freeze pending; not a `/6`
  freeze; no code): the "BLOCKED (H6, rev 25/26)" framing above is qualified by
  the rev-42 selected quota>1-capable bounded recovery mechanism to be frozen
  in `/6` (pre-dispatch errors leave tasks `Ready`; failed batch commits no
  semantic state; dispatch-order `Failed` recovery; no N pre-reservation;
  `NONE` fallback; state guard); exact atomic realization + fixtures remain
  open.]**
- **Bounded.** Batch size is bounded by the proposal budget; resolution is a flat
  single pass; record graphs are ID-valued and never recursive at commit.
- **Apply order.** Phase 4 materializes every append body (and allocates every
  new name) before resolving `Complete{DraftRecords}` or applying `Progress`, so
  a result may reference any draft in its own batch and the resolve table is
  complete at that point. `[audit2 N14]`

`SELECTED` (integrator default, §15.2.5): committed-reference existence is checked
for **both** append links and `StorePatch` references (`CommittedRefMissing`),
pre-mutation.

---

## 8. Proposed append fields, owners, and write sets

The M1 append schema is **proposed to be** a **closed, hashed** section of the
`/6` contract
(§12) — **prospective `/6`, not closed/hashed/frozen today** (rev 25 F1) — not
merely runtime `StoreSchema::declare` (which is hash-excluded today).
The runtime bus constructor (`CompilerBus::try_new`, which calls the in-crate
`new`) would seed `StoreSchema` from the `/6` M1 section, and
`ManifestRegistry::register` would validate against it, so runtime declarations
cannot diverge. `[audit2 N11, audit11; rev 25 F1]`

| Store | Append field | Record | Draft variant | Owning group(s) |
|---|---|---|---|---|
| `sources` | `spans` | `SpanRecord` | `SpanDraft` | **T03 only (user, in principle 2026-10-04)**; T04 reuses the committed PP span on `TokenRecord` and writes no span; T05 writes no span (token-range nodes) |
| `sources` | `expansions` | `ExpansionRecord` | `ExpansionDraft` | T03 |
| `names` | `entries` | interned `NameId` entries | `NameDraft` | T04 (job-global; T03/T06 read-only; mandatory allowlist) |
| `pp` | `tokens` | `PpTokenRecord` | `PpTokenDraft` | T03 (owns the `span` referenced by T04) |
| `lex` | `tokens` | `TokenRecord` | `TokenDraft` | T04 (span = committed T03 PP span; no span write) |
| `lex` | `literals` | `LiteralRecord` (decoded literal; carries the `LX08` candidate type) | `LiteralDraft` | **T04** (sole writer; consumed by T08 via the committed `RecordRef::Literal`) |
| `parse` | `nodes` | `NodeRecord` (first/last `TokenId` range) | `NodeDraft` | T05 (no span write) |
| `tasks` | `continuations` | `ContinuationRecord` (parse state) | `ContinuationDraft` | T05 |
| `symbols` | `scopes` | `ScopeRecord` | `ScopeDraft` | T06 |
| `symbols` | `scope_events` | `ScopeEventRecord` | `ScopeEventDraft` | T06 |
| `symbols` | `symbols` | `SymbolRecord` | `SymbolDraft` | T06 |
| `types` | `records` | `TypeRecord` | `TypeDraft` | T06 chip-id-keyed owner allowlist (TY13 canonical `int`, TY17 function type; single-producer policy; `StoreOwnerViolation` at manifest registration) |
| `sem` | `records` | `SemRecord` | `SemDraft` | T07 |
| `constants` | `records` | `ConstRecord` | `ConstDraft` | **T08 (sole M1 writer)**; the decoded T04 literal arrives by the **typed `ConstantRequest` family** below (no T04 write) |
| `ir` | `functions` | `FunctionRecord` | `FunctionDraft` | T09 |
| `ir` | `blocks` | `BlockRecord` | `BlockDraft` | T09 |
| `ir` | `values` | `ValueRecord` | `ValueDraft` | T09 |
| `ir` | `instructions` | `InstructionRecord` | `InstructionDraft` | T09 (owns IR `Constant` emission) |
| `artifacts` | `fragments` | `ArtifactRecord` (total `ArtifactKind`) | `ArtifactDraft` | **T03** produces Normalized/Spliced/CommentFree/Preprocessed fragments; **CT14 ArtifactFinalize/host** only reads and publishes the Host write request — CT14 does **not** append `artifacts.fragments` |

`[§8 read-only audit, 2026-10-05; doc-only annotation; no owner, scope, or
claim change]` The sole-owner rows above match the §6.2 backing-arena mapping,
the §12 items 2/4/17 inventory, and the §15.3 conditional matrix (no
shared-writer carveout). Two backing-arena facts are made explicit for this
table: `lex.literals` (T04) and `symbols.scope_events` (T06) map to the **new**
`Arenas.literals` / `Arenas.scope_events` (neither exists in `/5`), and the
`sem.records` row presumes the `SemId` + `Arenas.sem` + `SemDraft` +
`RecordRef::Sem` backing whose selection remains **OPEN** (OB-19; T07 §2
S1/S7) — the `NodeId`-keyed family-less alternative would remove this row and
its family/tag/arena. Mapping order/counts remain the OB-13 T01 `/6` item.

Store decisions `[audit2 N1]`:

- **Spans/expansions are NEW append fields on the existing `sources` store**
  (`sources.spans`, `sources.expansions`). The backing `TypedArena`s already
  exist in `bus.rs` (`Arenas.spans`, `Arenas.expansions`), as do `StoreId::Sources`
  and the `RecordRef` variants. The fields are **not** in
  `StoreSchema::foundation()`, which declares `sources.bytes`, `sources.name`,
  `sources.span_root`, `sources.expansion`; the `/6` `M1AppendSchema` adds the two
  new fields, and `sources.expansion` vs `sources.expansions` must be kept
  distinct. No `StoreId::Spans`/`Expansions` is invented. `[audit3 AD2]`
- **Names use one new `StoreId::Names`** (field `entries`) that maps to the
  **`InternTable`**, not a `TypedArena`; it is **appended at the end** of
  `StoreId::ALL` so existing indices 0–19 do not shift. A committed `Name`
  reference is checked via `InternTable::get(id)` (the private
  `entries`/`index`/`total_bytes` are reachable only through it). Names are
  idempotent intern entries: no per-record body, **no `CommittedPatch`**, bounded
  only by `max_intern_entries`/`max_intern_bytes` (not `max_records_per_arena`).
  The `StoreId::Names` version slot exists only for the store-version/manifest
  model; it bumps when ≥1 genuinely new name is interned and does **not** bump
  for a reused name. `[audit3 AD4]`
- `sources.spans` is **T03-only (rev 21; user, in principle 2026-10-04).** T04
  does **not** write it: `TokenRecord.span` is a reference to the **committed T03
  PP span**. T05 does **not** write it: `NodeRecord` carries a first/last
  `TokenId` range and the node span is derived. There is **no shared-writer
  carveout and no T05 span blocker**; the Guardrail is unamended. The exact
  T03 `SpanRecord` fields and the T05 token-range derivation are still owner
  sign-off items. `sources.span_root` is the existing source-root span field and
  grants no general span write.
  `[rev 20 C6 withdrawn; user in-principle decision; T03/T04/T05 fields pending]`
- **T04/T08 `constants.records` handoff (user-selected direction + accepted H1
  allocation).** `constants.records` has the **single M1 writer T08** (itemized:
  `ConstDraft` only; no T04 write). `LiteralId`/`RecordRef::Literal` (wire tag 26),
  `LiteralRecord` (`lex.literals`, T04-owned, §5/§8) is a **committed typed
  literal family**. T04 commits the `LiteralRecord`; T08 references it by
  `RecordRef::Literal` (a committed record, so `Payload` stays `RecordRef`-only)
  and commits the proposed, unfrozen `ConstantResult { value: RecordRef::Const,
  legality: ConstLegality }` (rev 25 F3; the exact field/type naming is a
  co-freeze item). The handoff **must
  preserve** the semantic information `node`, `required_kind`, `legality`, and the
  **`LX08` candidate type** as **explicit co-freeze requirements**.
  **H1 — the allocation is user-accepted in principle (2026-10-04).** The
  sem-stage per-use `ConstantRequest { literal: RecordRef::Literal, node,
  required_kind }` (proposed, unfrozen — rev 25 F3), `legality` on the
  `ConstantResult`, and the `LX08` candidate type on the per-literal committed
  `LiteralRecord` (`candidate_type`, proposed/unfrozen). This is
  the **`/6` revision working basis**, **not a freeze and not code/chip
  authorization**; the exact `LiteralRecord`/`ConstantRequest` enum variants and
  carrier shapes remain a T04/T08 `/6` co-freeze item plus T01 integrator
  acceptance. The task kinds are the two flows `const.literal-decode` (T04) and
  `const.evaluate` (T07), both stage `const` (rev 21); a shared T04+T08 writer is
  **not** selected. `[T04/T08 finding; user in principle; rev 21; rev 23 H1;
  user allocation accepted-in-principle 2026-10-04 rev 24; rev 25 F3 concrete
  unfrozen carriers; rev 26 §20]`
- **Artifact intermediate buffers (T03 R3; rev20 resolves the enum contradiction).**
  The checked-in enum is `{Preprocessed, Assembly, Object, Snapshot, Trace}`; M1
  **adds** `{Normalized, Spliced, CommentFree}`, for a **total 8-variant**
  `ArtifactKind` with a **total `requires_map`**: Normalized/Spliced/CommentFree/
  Preprocessed are map-mandatory; Assembly/Object/Snapshot/Trace are map-optional.
  `ArtifactRecord` has `source: Option<SourceId>` and ordered `raw_offsets: Vec<u64>`
  (len = `bytes.len() + 1` for mandatory kinds; empty for optional), bounds-checked
  by `max_source_bytes` and snapshot-encoded. Mandatory: `source` must be `Some`
  and equal the task payload source (`ArtifactSourceMismatch`/
  `ArtifactSourceMissing`); logical offsets remap through `raw_offsets` to raw
  `SourceRecord.bytes` offsets before any `SpanDraft` is formed. Optional
  (rev-47 delegated candidate default): `raw_offsets` must be empty, and
  `source` may be `Some` — validated equal to the task payload source — or
  `None`; no source-payload-equals-`bytes` requirement. `artifact_kind_name`
  gains "normalized"/"spliced"/"comment_free"; the classification is a **`/6`
  blocker** requiring T03 second sign-off (§15.3), not a frozen claim.
  `Preprocessed` (PP28 output) ≠ `Normalized` (PP01 buffer) and, per the rev-21
  fix, is **declared but not produced by the M1 fixture** (an explicit gap); the
  M1-produced kinds are exactly `Normalized`/`Spliced`/`CommentFree`. Artifact
  production is T03; CT14/host only reads and publishes, never appends
  `artifacts.fragments`.
- **Parse continuation / cross-tick pipeline (T05).** Parse state reuses the
  existing `ContinuationId`/`RecordRef::Continuation` and
  `Arenas.continuations` (`tasks.continuations`), not a new family.
  `ContinuationDraft` carries `production` (from `resume_kind`), `cursor: TokenId`
  (may be the EOF token), `context`, `binding_power`, `scope`, `parent`,
  `partial_children`, `next_child_ordinal`, and `previous` (committed-only,
  acyclic). **There is no `awaited` on the continuation (rev 21); the single
  awaited-child source is `TaskState::Waiting(WaitSet)`.** `Payload` stays
  `RecordRef`-only. The handoff is the own-batch continuation +
  `AwaitChildren`/`Progress` protocol of §5/§6.3: commit validates
  `ContinuationRef`/`ChildRef` pre-apply, materializes the continuation, resolves
  the successor's `ContinuationRef::OwnBatch`, applies `Enqueue`, and applies
  `AwaitChildren`/`Progress`; **the join reinsertion is performed in commit apply
  into the parent's own `stage_queues[stage]`, and CT07 is decision-only**; the
  chain is committed-only. **No extra-tick scheduler path is required or claimed.**
  `ParseCursorDidNotAdvance`/`ParseDepthExceeded` are **chip diagnostics**; the
  depth bound reuses `max_task_depth` (no separate `max_parse_depth`); PA20's M1
  path is unary-`+` only (sizeof/alignof type-name variants are not exercised);
  PA38 preserves the following function; tests `M1-NEG-08/09/11`, `M1-REC-05`.
  The exact fields/two-tick flow remain a **T05 sign-off blocker**; the mechanism
  is concrete, not claimed resolved.

Read sets remain field-level per chip (e.g. `lex.tokens.kind`) and must each be
declared in `StoreSchema`; the M1 frozen section fixes the **append** areas only.

---

## 9. Proposed M1 task-kind code allocation

`TaskKind = group<<12 | local`, with `local 0..=15` reserved for
foundation/protocol kinds; group owners claim from `16` (matches
`TaskKind::RESERVED_LOCAL_MAX = 15`). Proposed minimum M1 Part A kinds (all
`RESIDUAL`; names are proposals, not existing registrations):

| Group | Local range (proposed) | M1 kinds (catalog refs) |
|---|---|---|
| Control (0) | 16–31 | select, guard, validate, commit, resume, phase-advance, progress-budget, diagnostic-commit, recovery-select, artifact-finalize (T02) |
| Host (1) | 16–23 | read-source, write-artifact, invoke-toolchain (T02/Host) |
| Preprocess (2) | 16–39 | normalize, splice, comment, pp-scan, directive, diagnostic, emit (PP01–04, PP26, PP28); **PP08 `MacroUndefChip` is unexercised by M1 and must carry an explicit gap gate**, not a pass; **`PP26` `PpDiagnosticChip` is diagnostic-only via `M1-NEG-01`/`M1-NEG-02` (its exact role for those fixtures remains a T03/T01 co-freeze item)**, and `PP02`/`PP03` are exercised **identity-only** on the M1 canonical path (splice/comment-removal semantics deferred, T03 rev 54/55) |
| Lex (3) | 16–47 | classify, identifier, keyword, punctuator, integer-value, **literal-record (T04-owned `LiteralRecord`)**, type-select, publish, location, error (LX01–09, LX12, LX13, LX16–18; **`LX09`/`LX12`/`LX13`/`LX18` are diagnostic-only via `M1-NEG-04`/`M1-NEG-05`/`M1-NEG-06`, and `LX10`/`LX11`/`LX14`/`LX15` remain deferred non-M1 forms**) |
| Parse (4) | 16–63 | TU, external-decl, specifiers, declarator, function-declarator, primary, **unary (PA20; unary-`+` M1 path only)**, binary, compound, jump/return, expr-stmt, recovery (PA01–09, PA16, PA18, PA20, PA22, PA24, PA28, PA32, PA34, PA38; **`PA18` only via the mode-sensitive `M1-NEG-19` call-expression negative**) |
| Symbol/type (5) | 16–79 | scope-enter, scope-exit, lookup, symbol-declare, redeclaration, linkage, builtin-type, function-type, declarator-bind, promotion, arithmetic-conv, assignment-conv (TY01–03, TY07, TY09–10, TY13, TY17, TY20, TY25–27); **TY08 `TypedefRegisterChip` is NOT exercised by M1** (no typedef); **TY14 `QualifiedTypeChip` is NOT exercised by M1** (no qualifier in the fixture); `TY31 DeclarationConstraintChip` is exercised by M1 **only** through the constraint negatives `M1-NEG-12`/`M1-NEG-13`, not by the positive `M1-TY-*` rows, and its wider positive-path scope stays deferred beyond M1 |
| Semantic (6) | 16–47 | literal, binary-arithmetic, return, function-definition (SE02, SE07, SE21, SE29); **SE26 effect-sequencing is catalog-only and NOT exercised by M1** (deferred `EffectGraph`); SE01 only as the negative-fixture owner (`M1-NEG-14` undeclared identifier, mode-independent; `M1-NEG-19` implicit-declaration call, dialect-policy) |
| Constant/layout (7) | 16–31 | constant-context, constant-binary, constant-cast (CL01, CL03, CL05; chip `CL02` unexercised), **`const.literal-decode` (T04→T08 committed-`Literal` handoff)** and **`const.evaluate` (T07→T08 semantic constant request)** |
| IR (8) | 16–47 | function-begin, block-create, constant-lower (Part A folded `int 5`), return-lower, function-end (IR01–03, IR19, IR28) |
| Verification (12) | 16–31 | store-invariant, task-invariant, wire-lifetime, access-contract, token-ast-invariant, typed-ast-invariant, ir-interpret, replay-compare, evidence-classify (VF01–VF06, VF12–VF14); **VF07 `CfgInvariantChip`/VF08 `IrInvariantChip`/VF09 `SsaInvariantChip` are out of M1** (T13 rev 8; no M1 CFG/SSA verification registration — recorded gap, not a pass) |

`SE01` is **not** on the M1 positive path; under symbolic `int` the M1 fixture
`M1-SE-01` is owned by `SE02`, not `SE01`. `SE01 NameExpressionChip` is retained
only as the owner of the negative fixtures `M1-NEG-14` (undeclared identifier,
mode-independent) and `M1-NEG-19` (implicit-declaration call, dialect-policy);
it is catalog-only for M1 and is not listed as an M1 semantic rule. Local codes
register in `TaskKindRegistry` (runtime) and route in `RoutingTable`.

---

## 10. Integration design: driver, dispatcher, chips, commit

### 10.1 Existing APIs (checked-in; reused, not modified)

- `cc_silicon::RestrictedChip`, `ChipAdapter`, `ProjectedChip`, `silicon_chip!`
  (`src/chip.rs`); `Motherboard::{new, with_backend, install, install_projected,
  layers, clock_tick, backend_name}` (`src/motherboard.rs`); `Backend::{name,
  execute_layers}` and `CpuBackend` (`src/backend.rs`); `Bus::{reset_wires,
  latch, tick_count, advance_tick}` (`src/bus.rs`).
- `compiler::{commit_proposals, CommitError, CompilerBus, CompilerPins,
  CompilerWires, RoutingShell, RoutingTable, TaskKindRegistry}`.

Constraining facts (verified):

- `Motherboard::clock_tick` runs `reset_wires` → `backend.execute_layers(layers,
  pins, bus)` → `bus.latch` → `bus.advance_tick`; there is no post-propagation
  hook, and `Backend::execute_layers` returns `()`.
- `CompilerBus` does not override `Bus::latch`; `advance_tick` bumps
  `control.tick`.
- T02 (`T02_CONTROL_CHIPS.md`, fixed topology line) states: *"TaskSelect →
  TaskGuard → the corresponding worker sub-pipeline → ProposalValidate →
  TaskCommit → ResultResume/PhaseAdvance. Route by task rather than traversing
  the entire catalog every tick."*

### 10.2 New required application integration (proposed)

- `compiler/src/driver.rs`: `CompilerDriver` + `CompilerBackend`.
  - `CompilerBackend` implements `cc_silicon::Backend<CompilerBus>` and overrides
    `execute_layers(&mut self, layers, pins, bus)`. The root
    `Motherboard::clock_tick` has already run phase 0 `reset_wires`, so `wires` is
    `Default`. Exact order (the **quota = 1 baseline** specialization; §10.5
    generalizes steps 2–5 to a bounded ordered `SelectionBatch` at any quota and
    is the authority for multi-inflight behavior):
    0. **tick start**: `control.selected = None`, `tasks.active = None`
       (mirrors `RoutingShell::propagate`); `wires.selected` is already `None`
       from `reset_wires`.
    1. **budget/cancel gate** (preserves `RoutingShell` semantics; makes the job
       terminal). First, if `pins.tick_budget_reached` set
       `control.budget_exhausted = true`. If `pins.cancel`, set
       `control.cancel_requested = true`, `control.job_state = Failed`, set the
       tick outcome to `Cancelled`, and go to 6. Else if
       `control.tick >= limits().max_ticks` or `control.budget_exhausted`, set
       `control.budget_exhausted = true`, `control.job_state = Failed`, set the
       tick outcome to `BudgetExhausted`, and go to 6. This gate **never appends**
       a record (step 6 is the sole append site) and performs no selection or
       commit. `max_ticks == 0` sets a terminal outcome at tick 0.
    2. **CT03 via its adapter**: tick the installed CT03 `ProjectedChip` in the
       dedicated first control layer — `for chip in &layers[SELECT_LAYER] {
       chip.tick(pins, bus); }` (exactly one chip there). Its `ChipAdapter::read`
       builds the projection and `commit` writes the **canonical**
       `bus.wires.selection: SelectionBatch` (§6.2.1); at quota 1 it has ≤ 1
       entry, and the compatibility projection `wires.selected` is set from it.
       **CT03 is never invoked by a direct `compute` call and never receives a
       hand-built projection.**
    3. **apply selection (dispatcher)**: read `wires.selection: SelectionBatch`
       in order; for each entry set `TaskState::Running`, record it in
       `tasks.in_flight` and `control.dispatch_cursor`, remove it from its
       `stage_queues[stage]`, and set the compatibility projection
       `wires.selected` (quota 1 only); set the tick outcome to `Executed`; an
       empty batch sets `Idle` and leaves `wires.selected = None`. Either way
       proceed to 6 (no append here).
    4. **routed invocation**: for each selected entry in dispatch order, tick
       `layers[GUARD_LAYER]` (TaskGuard) and then **only**
       `layers[WORKER_BASE + offset]`, where `offset` comes from
       `RoutingTable::lookup(kind).layer`, with `control.dispatch_cursor` set to
       that entry's position in `tasks.in_flight` so the worker projection sees
       exactly that task. The rule `routing.one-chip-per-layer` (enforced at route
       registration: a worker layer must be `>= WORKER_BASE` and distinct from
       `SELECT_LAYER`/`GUARD_LAYER`, and each layer carries at most one routed
       chip) guarantees only the routed worker runs; non-routed worker layers are
       not ticked (a worker has no work unless it owns the dispatched task), and
       each worker adapter also gates on the current `dispatch_cursor`. CT03's
       layer is not re-ticked.
    5. `commit_proposals(bus, take(bus.wires.proposals))`; the commit validates and
       applies, subsuming T02 ProposalValidate/TaskCommit. **On error at quota 1
       the frozen `/5` `routing.rs` `fail_selected` ALWAYS transitions the single
       task to `TaskState::Failed`** — attaching a committed `DiagnosticId` when
       diagnostic/record capacity allows, and otherwise using the
       `TaskState::Failed(DiagnosticId::NONE)` sentinel so the task is not stranded
       — and clears `control.selected`/`tasks.active`; that single-task obligation
       is preserved and `/5`-verified. The batch (quota `> 1`) failure path — "fail
       every dispatched
       task exactly once and clear the in-flight set so no `Running` survives" —
       is **BLOCKED (H6, rev 25/26)**: clearing the in-flight set does not clear
       `TaskState`, and the fan-out mechanism is an open `[INT]` `/6` decision
       (§7; alternatives: generalize the verified deterministic all-dispatched
       fail transition with the same optional-diagnostic/`DiagnosticId::NONE`
       sentinel semantics, or another T01-approved atomic terminal path, e.g.
       reserve bounded terminal diagnostic slots before dispatch). The driver
       clears
       `control.selected`/`tasks.active`/`tasks.in_flight` (it does **not** clear
       `wires.selection`/`wires.selected`); it makes **no** unsupported claim that
       no task remains `Running`.
       **[Rev-42 note (user-accepted recommendation; T01 co-freeze pending; not a
       `/6` freeze; no code): the "BLOCKED (H6, rev 25/26)" batch framing above
       is qualified by the rev-42 selected quota>1-capable bounded recovery
       mechanism to be frozen in `/6`; exact atomic realization + fixtures
       remain open.]**
    6. **sole append site**: choose the tick outcome from the post-commit
       `job_state` — if the budget/cancel gate set `BudgetExhausted`/`Cancelled`
       preserve it; else `job_state == Finished` -> `Finished`,
       `job_state == Failed` -> `JobFailed`, otherwise the step-3 outcome
       (`Idle`/`Executed`/`CommitFailed`). Append **exactly one** `bus.report`
       entry whose canonical field is the ordered `TickRecord.dispatched` set and
       per-stage metrics (captured after backend execution, before the next tick's
       `reset_wires`; empty on idle/budget/cancel); `TickRecord.selected` is only
       the quota-1 compatibility projection. Set `bus.report.last_outcome = record`
       (the guard later returns this exact record). Then the root `Motherboard`
       latches and advances. No other step appends, so the budget/cancel and idle
       paths produce exactly one record each.
  - **Terminal halt (API).** `Motherboard` is **private** to `CompilerDriver`;
    `step` is the only public clock entry, so a direct `clock_tick` cannot bypass
    this guard. Because `Motherboard::clock_tick` returns `()`,
    `step(&mut self, pins: &CompilerPins, bus: &mut CompilerBus) ->
    TickOutcomeRecord` first evaluates the terminal predicate
    `control.budget_exhausted || control.cancel_requested ||
    matches!(control.job_state, JobState::Finished | JobState::Failed)`. If
    terminal, it returns `bus.report.last_outcome` **without** calling
    `Motherboard::clock_tick` and **without** appending (repeated host attempts
    are idempotent). Otherwise it calls `clock_tick` exactly once and returns the
    record step 6 appended this tick. Initialization (`JobState::Idle`/`Running`)
    is not terminal. `TickOutcome` gains `Finished` and `JobFailed`: step 6
    records `Finished` when `job_state == Finished` and `JobFailed` when
    `job_state == Failed` **as it stands before step 6** (e.g. cancel or a
    semantic failure), while budget/cancel keep their distinct
    `BudgetExhausted`/`Cancelled` outcomes. `JobState::Finished` is future T02
    CT14 behavior (not set by the foundation today). A `Failed` set **after**
    step 6 by the defensive `advance_tick` overflow cannot change this tick's
    outcome; it is observable only via the `tick_overflow` flag/snapshot (no
    separate `JobFailed` reason field is proposed).
  - **Report bound and fallible construction.** `bus.report.trace` holds at most
    `max_ticks` non-terminal records plus one terminal record
    (`BudgetExhausted`/`Cancelled`/`Finished`/`JobFailed`), so its bound is
    `checked_add(max_ticks, 1)` (i.e. `trace.len() <= max_ticks + 1`). Because the
    infallible `CompilerConfig::new`/`CompilerBus::new` cannot reject an invalid
    config, add `CompilerConfig::try_new(...) -> Result<Self, ConfigError>` and
    `CompilerBus::try_new(config) -> Result<Self, ConfigError>`; `validate()`
    gains `ConfigError::MaxTicksOverflow` (stable code `config.max_ticks_overflow`)
    when `max_ticks.checked_add(1).is_none()` (i.e. `max_ticks >= u64::MAX`).
    `CompilerBus::try_new` revalidates the config as defense (same error).
    `CompilerConfig::new`/`CompilerBus::new` become **`pub(crate)`** (not public
    deprecated) and are used only in-crate with known-valid inputs (`Default`
    uses `Limits::fixture()`, `max_ticks = 1<<20`); all external callers/tests go
    through `try_new`, which rejects before the job starts.
    `CompilerConfig::default()` cannot overflow; only a custom config can.
  - **Tick advance and `tick_overflow` (defensive-only).** The root `Motherboard`
    always calls `CompilerBus::advance_tick` after the backend, so the driver
    cannot intercept it. `advance_tick` returns `()`, so it must use checked
    arithmetic and, on the impossible `checked_add` failure, set
    `control.tick_overflow = true` and `control.job_state = Failed` (a plain
    register; **no allocation, diagnostic, or report append** in the infallible
    advance) and never wrap. Because step 6 already appended this tick's record
    and set `last_outcome` before `advance_tick` runs, `advance_tick` **cannot**
    produce a terminal report record; the next `CompilerDriver::step` sees the
    terminal `job_state`/`tick_overflow`, returns the existing `last_outcome`
    unchanged, and appends nothing. The host distinguishes the cause by
    inspecting `control.tick_overflow` (encoded by `Snapshot::capture`); no
    `JobFailed` overflow record or overflow diagnostic is claimed. `tick_overflow`
    is therefore redundant with the `job_state == Failed` guard term (kept only
    for observability, not a separate predicate). With a validated `max_ticks` the
    threshold is `< u64::MAX`, so overflow is unreachable in a permitted run; the
    flag is defensive. The terminal gate fires at `control.tick >= max_ticks`, so
    `max_ticks == 0` yields exactly one terminal record; the terminal tick still
    performs one root `latch`/`advance_tick`, so `control.tick` ends at `T + 1`
    (for `max_ticks = M` the terminal tick begins at `M` and ends at `M + 1`), and
    `step` guards every further tick/report.
  - `wires.selection`/`wires.selected` lifecycle: the canonical
    `wires.selection: SelectionBatch` is written on dispatch and preserved for
    `Snapshot::capture`; `wires.selected` is the quota-1 compatibility projection
    only. `reset_wires` at the next tick start clears both. On
    budget/cancel/error `wires.selected` remains `None` (or the successfully
    dispatched singleton, if the failure happened after dispatch).
  - `CompilerDriver` wraps `Motherboard::<CompilerBus>::with_backend(...)`,
    installs CT03 and each routed `ProjectedChip` at its registered layer (one
    chip per layer), and calls `clock_tick`.
  - **No root change is required**: `Backend::execute_layers` receives `layers`
    and may tick any subset/order; `install_projected` installs the chips. This is
    the exact app-owned technique (no direct `compute`, no root-framework change).
- **Selection shape (exact `/6` proposal).** The canonical selection value is
  `SelectionBatch { entries: Vec<SelectionEntry> }` on `wires.selection`
  (§6.2.1); `SelectionEntry { task, chip, layer, dispatch_ordinal }`. `report.rs`
  **does not exist today** (the crate has no `report.rs`), and the old
  `Selection { task: Option<TaskId>, chip, layer }` is **not** retained as a
  runtime type: it is superseded by `SelectionEntry`, with
  `wires.selected: Option<TaskId>` only as the **quota-1 compatibility
  projection**. `SelectionBatch.entries.len() <= max_inflight_per_tick`; the
  quota = 1 batch has at most one entry. `[audit4 BLK-3, finding 7]`
- `compiler/src/report.rs` (**proposed relocation; no such file exists today**):
  `TickOutcomeRecord`/`TickReport`/`Resolution` relocated here from `routing.rs`;
  `TickOutcome` gains `Finished` and `JobFailed`.
  `CompilerBus.report` is the **canonical persistent trace** (`last_outcome`,
  `trace: Vec<TickRecord>`), bounded by `max_ticks + 1` (at most `max_ticks`
  non-terminal records plus one terminal record), with `last_outcome` set to the
  exact appended record, and encoded+hashed by `Snapshot::capture`. The canonical
  `TickRecord` field is the ordered `dispatched: Vec<TaskId>` set (plus per-stage
  metrics); `TickRecord.selected` is only the quota-1 compatibility projection
  (from `wires.selected`). The host-side `snapshot::Trace` becomes a derived
  view/accumulator over `bus.report`, not a competing source.
  `[audit3 BL4, audit4 C1, audit6 BLK-6.1, finding 7]`
- **Exactly one selection path.** CT03 `TaskSelectChip` is a `RestrictedChip`
  invoked only through its `ProjectedChip`/`ChipAdapter` (step 1); the dispatcher
  reads `wires.selection` and applies it (step 2). CT03 is excluded from the
  routed invocation (step 3), so there is no duplicate selection and CT03 never
  mutates `control`/`tasks`. Fixed control layers are ordered
  `SELECT_LAYER < GUARD_LAYER < WORKER_BASE`; `TaskGuard` is a ticked decision
  chip. `routing.one-chip-per-layer` applies to the **worker** layers only; the
  fixed `SELECT_LAYER`/`GUARD_LAYER` are outside the routing uniqueness set. T02's
  ProposalValidate/TaskCommit are realized by the commit path's validate/apply
  phases; CT05 is validation-only and CT06 proposes the commit. This split is
  recorded as the **selected draft direction (user/integrator; not an accepted
  contract)** in `T02_CONTROL_CHIPS.md` (which states the
  decision-only/backend-commit realization). `[audit3 BL3, audit4 BLK-3, audit6 C2, selected AB1a/AB1b; rev 25 F9: not accepted]`
- `RoutingShell` is demoted to a `#[cfg(test)]`/`pub(crate)` test helper (or
  removed after tests migrate); it must not be a second runtime path. Its
  `TickOutcome`/`TickReport`/`Resolution` types move to `report.rs`.
  `routing.rs` is therefore in the `/6` inventory. `[audit2 N6, N13]`

### 10.3 Where each step happens

| Step | Owner | Where |
|---|---|---|
| Wire reset | root `Motherboard` | `clock_tick` phase 0 (clears `wires`, incl. `wires.selected`) |
| Tick-start reset | app `CompilerBackend` | `control.selected=None`, `tasks.active=None` before selection |
| Budget/cancel gate | app `CompilerBackend` | `pins.tick_budget_reached`/`max_ticks` -> terminal `BudgetExhausted`; `pins.cancel` -> terminal `Cancelled`; no selection |
| Selection decision | T02 CT03 `ProjectedChip` | adapter `read`+`compute`+`commit` writes canonical `wires.selection: SelectionBatch` |
| Selection application | app dispatcher | reads `wires.selection`; sets `control.selected`/`tasks.active`/`tasks.in_flight`/`control.dispatch_cursor` and `wires.selected` (quota-1 projection) |
| Worker/control invocation | `CompilerBackend` + `ProjectedChip` | tick GUARD_LAYER + only the routed worker layer (one chip per layer), `dispatch_cursor` per task |
| Typed proposal build | `ChipAdapter::commit` | writes `wires.proposals` only |
| Proposal commit | `CompilerBackend` | `commit_proposals` (subsumes ProposalValidate/TaskCommit) |
| Failure resolution | `CompilerBackend` (`fail_selected`) | **quota 1 (verified `/5`):** the single task **always** becomes `TaskState::Failed` — a committed `DiagnosticId` when capacity allows, else the `TaskState::Failed(DiagnosticId::NONE)` sentinel (never stranded). **quota `> 1`:** the batch fan-out ("fail every dispatched task once; clear in-flight") is **BLOCKED** (H6) — clearing the in-flight set does not clear `TaskState`; alternatives (generalize the deterministic all-dispatched fail transition with the same optional-diagnostic/sentinel semantics, or another T01-approved atomic terminal path) in §7/§10.5. Keeps `wires.selection` **[Rev-42 note: "BLOCKED" qualified by the rev-42 selected quota>1-capable bounded recovery mechanism to be frozen in `/6` (user-accepted recommendation; T01 co-freeze pending; not a `/6` freeze; no code)]** |
| Report/trace | `CompilerBackend` + `CompilerDriver::step` | canonical `bus.report` with ordered `dispatched`; `len <= max_ticks + 1` (one terminal); snapshot-encoded |
| Latch / tick advance | root `Motherboard` | `clock_tick` phase 2 |

### 10.4 Integration decisions (selected) and remaining technical residuals

The recommended integration decisions are **selected draft defaults**
(user in-principle direction 2026-10-04, doc-level only; **not accepted**; no
code, no root change, compiler still not implemented):

- **AB1a (selected):** CT03 is invoked only through its
  `ProjectedChip`/`ChipAdapter` (writing `wires.selection`); the dispatcher
  applies the selection, and CT03 is excluded from the routed invocation.
- **AB1b (selected):** CT05 `ProposalValidateChip` is validation-only; CT06
  `TaskCommitChip` may propose a commit decision, but the commit
  path/`CompilerBackend` performs the mutations. No commit side-effect inside a
  chip.
- **AB2 (selected):** `TickOutcome`/`TickReport`/`Resolution` and the
  `fail_selected` failure path move to `report.rs`/`driver.rs`; `bus.report` is
  canonical snapshot state encoded by `Snapshot::capture`.

These are recorded in [T02_CONTROL_CHIPS.md](T02_CONTROL_CHIPS.md) so the semantic
topology stays `TaskSelect → TaskGuard → worker sub-pipeline → ProposalValidate →
TaskCommit → ResultResume/PhaseAdvance` while the layer/runtime realization is
fixed: CT03 decision-only via adapter; dispatcher applies selection; CT05
validation-only; CT06 proposes, backend commits; `CompilerDriver` owns a private
`Motherboard`; the custom `Backend` ticks CT03's `ProjectedChip` exactly once,
applies selection, invokes guard + the exact routed worker, collects proposals,
`commit_proposals` validates/materializes, and `bus.report` is canonical.

Feasible with the current public APIs and **no root-framework change**. The
still-open items are the §15.3 owner schema sign-offs (T03/T04/T05/T06/T07/T08/
T09) and the §15.4 blocking questions; the §15.2 integrator defaults are closed
as draft defaults. None is an integration blocker.

Residual limitation (T01 §4.1): adapters still receive `&mut CompilerBus`;
field-level isolation is a review/lint obligation until schema-generated private
projections exist.

### 10.5 Pipeline scheduling: staged, bounded, sequential multi-inflight (proposed)

This subsection is the M1-side realization of
[ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) (still
**PROPOSED**). It makes cross-tick work an explicit **pipeline** and turns on
bounded in-flight scheduling as a first-class strategy, while leaving the
generic framework untouched.

**Baseline and optimization profile.** `max_inflight_per_tick` defaults to **1**.
The quota = 1 path (`SelectionBatch` of length ≤ 1, `tasks.active`, one routed
worker) is the **baseline equivalence mode**. Because the new scheduler snapshot
encodes scheduler-only registers, equivalence is the **canonical semantic
comparison projection** (semantic records/results/diagnostics + semantic trace,
excluding stage queues, in-flight set, `dispatch_cursor`,
`stage_assignment_version`, and `PipelineMetrics`), **not** a byte-identical
snapshot; the new scheduler snapshot must separately be deterministic and
replay-identical run-to-run. The pipeline is **designed** for a measured bounded
quota `> 1` over independent stages; a quota `Q > 1` is acceptable only after
before/after tick counts are measured on the frozen corpus and reviewed (§2.8 of
ADR-0002). This document does **not** claim any throughput improvement, and it
does **not** freeze a single-active-task architecture as final.

**Dispatch loop (extends §10.2, at any quota).** The single-threaded
`CompilerBackend` performs, in order:

1. tick-start reset clears the in-flight set, `dispatch_cursor`, and wires; the
   budget/cancel gate runs **once**, before selection (cancel is not re-evaluated
   between dispatches, §2.7 of ADR-0002).
2. CT03's installed `ProjectedChip` is ticked **exactly once** and its adapter
   writes a bounded ordered `SelectionBatch` to `wires.selection` (CT03 stays
   decision-only; still one selection path).
3. the dispatcher applies the whole batch in order: mark each task `Running`,
   record it in `tasks.in_flight`, set `control.dispatch_cursor`, set the
   quota-1 projection `wires.selected`, and remove it from its canonical
   `stage_queues[stage]`. The batch is validated
   **before** any dispatch: no duplicate task (`DuplicateSelection`) and
   `entries.len() <= max_inflight_per_tick` (`SelectionBatchOverflow`).
4. for each dispatched task in order, tick the guard layer and **only** that
   task's routed worker layer, with `dispatch_cursor` set so the worker projection
   sees exactly that task; workers write proposals to wires only.
5. collect all proposals into one ordered log and call `commit_proposals`
   **once** (one ordered atomic commit, §6.2.2/§7). **Verified `/5`
   obligation:** the frozen `routing.rs` `fail_selected` single-task
   commit-failure path **always** transitions that task to
   `TaskState::Failed` — a committed `DiagnosticId` when
   diagnostic/record capacity allows, else the
   `TaskState::Failed(DiagnosticId::NONE)` sentinel (never stranded) — and
   clears the singleton active/selected state; this semantic obligation is
   preserved. **The batch failure recovery direction is user-selected in
   principle (rev 32, 2026-10-04; H6); the mechanism remains `[INT]`/co-freeze:**
   "fail every dispatched task exactly once and clear the in-flight set so no
   `Running` survives" is **not** an established mechanism, and clearing the
   in-flight set does **not** clear `TaskState`/`Running`, so a `Running` task state
   can survive (clearing the in-flight set is **not itself** a transition). The user
   **selected the deterministic all-dispatched failure recovery direction in
   principle**: transition every dispatched task, in **dispatch order**, **exactly
   once** to `TaskState::Failed` — a committed `DiagnosticId` when
   diagnostic/record capacity allows, else the
   `TaskState::Failed(DiagnosticId::NONE)` sentinel — so every task leaves
   `Running`. This generalizes the verified `/5` single-task transition with the
   same optional-diagnostic/sentinel semantics. **The exact atomic/bounded
   implementation requires T01 integrator approval**; the batch failure-atomicity
   details, the diagnostic budget/state mechanism, and the T02/T13 owner
   fixtures/sign-offs remain **pending**. The chosen design must uphold no task
   remains `Running`, exactly-one transition per dispatched task, bounded
   diagnostics, and failure atomicity. No no-`Running` claim is made for the batch
   beyond the selected direction. **H9 (rev 33, user direction in principle,
   2026-10-04):** the user accepts in principle **removing `max_inflight_total`**
   from the `/6` candidate — per-tick dispatch is already bounded by
   `max_inflight_per_tick` (the dispatch batch bound), the per-stage
   `stage_queue_bound`, and `max_tasks_total`; `TaskState::Waiting` is **not**
   in-flight; and `tasks.in_flight` is an **ephemeral per-tick scheduler batch**
   cleared at latch only after every dispatched task has a terminal/`Waiting`/
   `Progress` outcome (or H6 recovery), so **no residual in-flight set survives
   the latch**. The dispatcher's pre-worker `Ready → Running` mutation is distinct
   from the one ordered atomic semantic commit; pending **T01 integrator
   acceptance**, no frozen bound.
6. the sole report-append site appends one `TickRecord` recording the ordered
   dispatched set and the per-stage metrics.

**Cross-tick overlap via committed IDs and own-task-batch draft keys.** Independent
stage tasks may be in flight in the same tick or across ticks, but a task's
`Payload` may reference only **committed** records; new tasks/results become
visible next tick. A continuation, stage cursor, awaited child set, or join is
expressed over committed `TaskId`/`ResultId`/`ContinuationId` **or an own-task
batch draft key** (`ContinuationRef::OwnBatch`, `ChildRef::OwnBatch`), which
commit resolves in the same tick; it may never point at another task's draft, a
wire, or an address (the full protocol is specified in §5/§6.3). **Accepted in
principle (user, 2026-10-04; user decision A; narrow interpretation):** the
own-task-batch candidate now applies the user's **narrow reading** of Guardrails
§6.1 — the `OwnBatch(DraftRef)` key is a **transient wire/proposal input only**,
validated and resolved to committed IDs **before persistent state**, and no durable
cursor/`WaitSet`/join points at a draft/wire/address; the guardrail text is
**unamended** and **T01 integrator implementation-confirmation is pending**
(§3.12, §17.7 PIPE-5, §19.4, CDR §A). When a parent's
committed child results are all present, the join resumes the parent (T02
CT07/CT09). This is the mechanism by which a parse (or any stage) task that cannot
finish in one tick hands off to a next-tick stage task. **No extra-tick scheduler
path is required or claimed for M1.**

**Cross-task write conflicts.** A same-tick write to one `(StoreId, field,
record)` by two different tasks is rejected pre-apply (`CrossTaskWriteConflict`)
unless the field is declared append-only and the appends are semantically
independent under a hashed rule (finding 9; §7). **Rev 21 scope:** this covers two
tasks of the *same* owner group (`sources.spans` T03-only; `constants.records`
T08-only; `names.entries` T04; `parse.nodes` T05; `artifacts.fragments` T03) and
every patch; it does not permit a second owner group. CT14 does not append
`artifacts.fragments`.

**Fairness, backpressure, cancel, failure, replay.**
- Fairness is a deterministic policy (e.g. one entry per non-empty stage in stage
  order, then fill remaining quota by the global order) **proposed** as a versioned
  contract input to be pinned at `/6` (selected draft, not frozen today); any
  persistent fairness cursor is explicit, snapshot-encoded
  bus state (`pipeline.fairness-deterministic`).
- Backpressure is a pre-mutation per-stage check (§7 phase 3); a stage whose
  queues are at capacity is throttled deterministically and reports a structured
  metric/error, never a panic or silent overflow (`pipeline.backpressure-bounded`).
- Cancel is a once-per-tick terminal gate; a mid-tick failure is handled by the
  single atomic commit; a failed batch commits nothing.
- Replay: for identical input/config/pins/initial state, the
  `stage_assignment_version`, dispatch order, proposal order, commit outcome,
  snapshot hash, and per-tick report are identical; the quota is config and is
  replay-visible (`replay.dispatch-order-determinism`).

**Gate chip waves on the freeze.** All pipeline scheduling is **inert until the
`/6` schema/routing/snapshot/limits/T02/T13 amendments are frozen** (§12.18).
No chip wave (F1–F6) may depend on the pipeline registers before that freeze;
quota `> 1` work is a post-freeze, measured, integrator-accepted change. The
T13 verification additions (VF02/VF03/VF04 extended with the batch write-conflict
audit, and VF13; **VF12 is `IrInterpretChip` and is unrelated unless explicitly
extended**) and their fixtures are part of the same freeze.

**No root change.** The loop uses only the existing public APIs exactly as §10.2
does (tick a subset/order of layers inside `Backend::execute_layers`). The root
framework's no-threads, no-concurrent-write, fixed-array, `#![forbid(unsafe_code)]`
rules are preserved; multiple dispatches are sequential calls, not threads.

---

## 11. Phasing and dependency order (M1 Part A)

| Phase | Scope | Depends on | Gate |
|---|---|---|---|
| F1 | Control select/guard/commit/phase + source import + diagnostics; `AppendRecords`/`Progress`/`DraftRef`/`RecordLink`/`NamePlan`/`intern_reserved`; `sources.spans`/`sources.expansions`/`names.entries`; first language store (`pp`) | accepted `/6` | mechanism tests green; no language rule yet |
| F2 | Preprocessing (PP01–04, PP28) + Lex (LX01–08, LX17–18) | F1 | M1-PP-*, M1-LX-* (target-independent rows only) |
| F3 | Scope/Symbol/Type (TY01–03, 07, 09–10, 13, 17, 20, 25–27) | F2 names/spans | M1-TY-* (no numeric width assertions) |
| F4 | Parse (PA01–09, 16, 20, 22, 24, 28, 32, 34, 38) incl. `Node` | F2 tokens, F3 | M1-PA-* |
| F5 | Semantic (SE02, 07, 21, 29; SE26 catalog-only/not exercised) + Constant (CL01, 03, 05) | F4 AST, F3 types, T04 committed `LiteralRecord` | M1-SE-*, M1-CL-01/03 (both Part A, **no probe dependency**). Fixture `M1-CL-02` is the **probe-gated target-width row executed by chip `CL03`**; chip `CL02` (`ConstantUnaryChip`) is **unexercised**. T08 owns the `ConstRecord` value; T09 owns IR `Constant` emission. |
| F6 | IR lower (IR01–03, 19, 28; Part A folded `int 5`) + IR interpreter (**VF12 `IrInterpretChip`**) | F5 | IR-1..IR-9 invariants; modeled return `5` |
| F7 | Target code (T11) + ELF/run | F6 **and** verified probe | **blocked**; fail-closed remains |

F7 out of scope. G12 blocks every fixture asserting a concrete width/alignment/
ABI value until `verified=true`: `M1-LX-04/05`, `M1-TY-01`, `M1-CL-02`,
`M1-NEG-07/18` stay blocked; `M1-SE-01` and `M1-TY-07` (symbolic `int`
promotion, no concrete width) are not in that list under symbolic types.

Pipeline gate: F1–F6 may not implement or depend on the §6.2.1 pipeline
scheduling registers (stage queues, in-flight set, `SelectionBatch`, metrics)
until the `/6` freeze of §12.18 lands. Quota = 1 is the baseline equivalence
mode; a measured quota `> 1` is post-freeze. `[user pipeline direction, ADR-0002]`

Recommended: one `/6` envelope freezes the full M1 family list, append schema,
and proposals before F1; F1–F6 add record bodies, chips, tests, manifests.

---

## 12. Version bump requirements (`/6` change inventory)

The draft is inert; `/5` remains current until an accepted freeze lands. The
following are the changes **known today**; this inventory is **not claimed
exhaustive** — group owners will add record bodies and tests. `[audit2 N10]`

1. `compiler/src/task.rs`: `Proposal` gains
   `AppendRecords`/`Progress`/`AwaitChildren` (`PROPOSAL_NAMES`); `ResultValue`
   gains `DraftRecords` (`RESULT_VALUE_NAMES`); `TaskDraft.continuation` becomes
   `Option<ContinuationRef>` and `Task.continuation` stays committed
   `Option<ContinuationId>`; `ChildRef`/`ContinuationRef` added; `StoreId` gains
   `Names` (appended at end; `ALL` 21, `COUNT`, `index`, `from_index`, `name`,
   `parse`); `RECORD_KINDS` gains `sem`, `scope_events`, and `literals`;
   `Task` gains `progress_ordinal: u64` and `progress_count: u32`; `StageId`,
   `SelectionEntry`/`SelectionBatch`, and the pipeline scheduling registers
   (§6.2.1) are added; `TickRecord` gains the ordered `dispatched: Vec<TaskId>`
   with `selected` retained only as the quota-1 projection.
   `[audit2 N5, N10, T04/T05/T06 finding, finding 7, user pipeline direction]`
2. `compiler/src/ids.rs`: `SemId`, `ScopeEventId`, and `LiteralId`;
   `RecordRef::Sem` (tag 24), `RecordRef::ScopeEvent` (tag 25), and
   `RecordRef::Literal` (tag 26) appended after the existing 24 (tags 0–23, incl.
   `Artifact` at 23, do not renumber); `RecordFamily` (27 variants), and
   `RecordRef::family()`, `RecordRef::make()`. **Family ordinals and wire tags are
   separate inventories** and are **proposed to be** hashed separately at `/6`
   (not hashed today — rev 25 F1). `[T04/T06 finding]`
3. `compiler/src/intern.rs`: crate-private `intern_reserved(bytes) -> NameId` that
   is **lookup-first**: if `bytes` already exists it returns the existing
   `NameId`; otherwise it assigns `NameId::from_index(entries.len())`, inserts
   into `index`, and updates `total_bytes`. The phase-3 preflight reserves
   capacity for exactly the new names, so the mutation path performs no fallible
   check (idempotent duplicate spellings materialize once, single-writer
   preflight). Its return equals the phase-2a predicted id. `[audit3 BL5, T03 R8]`
4. `compiler/src/bus.rs`: `sem` arena (`TypedArena<SemId, SemRecord>`),
   `literals` arena (`TypedArena<LiteralId, LiteralRecord>`), **and the
   `scope_events` arena (`TypedArena<ScopeEventId, ScopeEventRecord>`)** — all
   three added to `allocated_total`; the rev-20 inventory omitted
   `scope_events` (rev 21 fixes it); `total_records` unchanged (**names are not arena
   records and are not added to it** — intern has its own bounds); `ensure_intern`
   preflight using the exact checked `new_name_bytes`; `arena_allocated(family)`
   per-backing-arena capacity helper; `CompilerWires.selection` becomes the
   canonical `SelectionBatch` with `wires.selected: Option<TaskId>` retained only
   as the quota-1 projection; `tasks` gains `in_flight`, canonical per-stage
   `stage_queues` with bounds (`tasks.ready` is a derived view),
   `dispatch_cursor`, and `stage_assignment_version`; a canonical bounded
   `report` register plus the snapshot-encoded `PipelineMetrics` register;
   `ArtifactKind` total 8-variant set with total `requires_map`;
   `StoreSchema` seeded from the frozen M1 schema; `StoreId::Names` bumps only
   when ≥1 new name is interned (reused names: no bump, no `CommittedPatch`);
   `advance_tick` becomes non-wrapping (checked) and sets `control.tick_overflow`
   (`bool`, snapshot-encoded in `Control`) plus `job_state = Failed` on the
   impossible overflow (the root `Motherboard` always calls it, so the driver
   cannot intercept; it cannot append a report because step 6 already ran).
   `[audit3 BL4, BL5, AD4, audit4 BLK-1, BLK-3, audit8 BLK-8.3, audit9 BLK-9.2, audit10 BLK-10.1, findings 4/7/8]`
5. `compiler/src/commit.rs`: phase split; `ResolvedDraft` dispatch; new
   `CommitError` variants; one-audit-patch-per-new-arena-body; `CommitReport`
   gains `appended: Vec<(StoreId, RecordRef)>` (or `appended_count`),
   `progressed: Vec<TaskId>`, `new_names: u32`, preserving `patches` semantics.
   `[audit2 N9]`
6. `compiler/src/snapshot.rs`: `RecordRef::Sem` tag 24,
   `RecordRef::ScopeEvent` tag 25, and `RecordRef::Literal` tag 26; encode
   **all** typed M1 arena bodies via the per-final-record encoders in §6.2
   (`encode_span`, …, `encode_instruction`, `encode_literal`,
   `encode_scope_event`, `encode_artifact`, `encode_continuation`) — **not** the
   commit-time `ResolvedDraft`; `artifact_kind_name` gains
   "normalized"/"spliced"/"comment_free" (total 8) and the total `requires_map`
   is encoded; `AppendRecords`/`Progress`/`AwaitChildren` wire encoding via
   `RecordDraft::encode`; encode
   `Task.progress_ordinal`/`progress_count`; encode the canonical bounded
   `CompilerBus.report`; encode the pipeline scheduling registers and
   `PipelineMetrics` (§6.2.1); host `Trace` becomes a derived view; `TraceRecord`
   gains `appended`, `progressed`, `new_names`, the ordered `dispatched` set, and
   per-stage metrics, with their byte encoding.
   `[audit3 BL4, BL6, audit4 C1, C2, C4, findings 4/7, user pipeline direction]`
7. `compiler/src/codec.rs`: `RecordDraft::encode` canonical writer usage and
   tests. `[audit2 N10]`
8. `compiler/src/limits.rs` + `compiler/src/target.rs`: proposed
   `max_task_progress: u32`; `max_inflight_per_tick` (**default 1**, the sole
   per-tick dispatch-count bound; `max_dispatches_per_tick` dropped per rev 51),
   per-stage `stage_queue_bound`,
   `max_const_bits`, and optional fairness-weight bounds; a
   new `ConfigError::MaxTicksOverflow` with stable
   diagnostic code `config.max_ticks_overflow` (plus `Display`/`to_diagnostic`
   arm) and `ConfigError` variants for invalid quota/queue/fairness values (stable
   codes); `CompilerConfig::validate()` errors when
   `checked_add(max_ticks, 1).is_none()`; new `CompilerConfig::try_new(...) ->
   Result<Self, ConfigError>` (canonicalize + validate) and
   `CompilerBus::try_new(config) -> Result<Self, ConfigError>` (revalidates as
   defense); `CompilerConfig::new`/`CompilerBus::new` become **`pub(crate)`**
   in-crate constructors fed known-valid constants (not public/deprecated), with
   all external tests migrated to `try_new`:
   `compiler/tests/{c03_task,c05_codec,c06_routing,c07_limits}.rs` (the
   `CompilerConfig::new`/`CompilerBus::new` call sites) and any `compiler/README.md`
   examples; `Default` sites are unaffected.
   `[audit7 BLK-7.2, audit8 BLK-8.2, audit9 BLK-9.1, audit10 BLK-10.2, user pipeline direction]`
9. `compiler/src/manifest.rs`: validate against the M1 append schema (proposed to
   be frozen at `/6`);
   **mandatory** field-scoped store-owner allowlist (§12.17) keyed by
   **`ChipId`** (`(ChipId, StoreId, field)` + record-kind constraint) and
   enforced in `ManifestRegistry::register`
   (`ManifestError::StoreOwnerViolation`, rule
   `manifest.store-owner-allowlist-mandatory`); the allowlist rows/seed/signature
   are **proposed to be** hashed at `/6` (not hashed today — rev 25 F1); plus
   chip→stage declaration validation
   (total `kind → stage` coverage, unique stage ordinals, stage matches routed
   layer; `ManifestError::StageUnassigned`/`StageLayerMismatch`).
   `[audit2 N10, T04/T06 finding, user pipeline direction]`
10. `compiler/src/routing.rs`: relocate `TickOutcome`/`TickReport`/`Resolution`
    (with `TickOutcome` gaining `Finished`/`JobFailed`) and `fail_selected` to
    `report.rs`/`driver.rs`; demote `RoutingShell` to test-only/deprecated;
    enforce `routing.one-chip-per-layer` in `RoutingTable::register` — a worker
    route layer must be `>= WORKER_BASE` and distinct from
    `SELECT_LAYER`/`GUARD_LAYER`, and no two routes may claim the same layer.
    `[audit3 AD1, audit5 D, audit7 BLK-7.5, audit8 BLK-8.1]`
11. New `compiler/src/materialize.rs` (`RecordDraft`, drafts, `AppendBatch`,
    `DraftRef`, `LinkTarget`, `RecordLink`, `RecordFamily`, `ResolveTable`,
    `NamePlan`, `ResolvedDraft`, `ProposalBatch`), `compiler/src/driver.rs`
    (`CompilerDriver` with a **private** `Motherboard`; `step` is the only public
    clock entry; terminal predicate `budget_exhausted || cancel_requested ||
    job_state ∈ {Finished, Failed}`; **step 6 is the sole report append site**;
    checked tick advance; tick-start reset; budget/cancel terminal gate using
    existing `CompilerPins.tick_budget_reached`/`cancel` +
    `control.budget_exhausted`; CT03/worker subset tick; commit;
    `wires.selected`), `compiler/src/report.rs`.
12. `compiler/src/contract.rs`: bump `CONTRACT_VERSION`/`CONTRACT_HASH`; add
    `NORMATIVE_RULES` (e.g. `storage.typed-record-materialization`,
    `commit.append-batch.one-per-task`,
    `commit.append-batch.family-validation-central`,
    `commit.append-batch.no-mutation-before-resolve`,
    `commit.append-batch.existence-checked`,
    `commit.append-batch.source-ownership`,
    `commit.append-batch.empty-rejected`,
    `commit.append-batch.single-mechanism`,
    `commit.append-batch.per-arena-capacity`,
    `commit.per-arena-capacity-checked-add`,
    `commit.tick-start-active-reset`,
    `commit.source-scoped-unique`,
    `driver.tick-budget-enforced`,
    `driver.terminal-halt`,
    `driver.terminal-outcome-finished-jobfailed`,
    `driver.report-bound-max-ticks-plus-one`,
    `driver.single-report-append`,
    `driver.tick-advance-non-wrapping`,
    `driver.wires-selected-record`,
    `driver.checked-tick-advance`,
    `driver.terminal-job-state`,
    `driver.motherboard-private`,
    `config.max-ticks-overflow-rejected`,
    `config.max-ticks-overflow-code`,
    `config.try-new-validating`,
    `bus.try-new-validating`,
    `api.infallible-new-pub-crate`,
    `storage.tick-overflow-defensive`,
    `driver.tick-overflow-observable-only`,
    `limits.max-ticks-no-overflow`,
    `routing.one-chip-per-layer`,
    `routing.worker-layer-range`,
    `commit.name-intern.commit-only-idempotent`,
    `commit.progress.next-tick-bounded`,
    `commit.task.one-transition-per-dispatch`,
    `commit.result-draft-resolution`,
    `commit.append-audit-patch-per-new-body`,
    `snapshot.typed-record-bodies-encoded`,
    `snapshot.final-record-encoders`,
    `snapshot.report-canonical-bounded-by-max-ticks-plus-one`,
    `sem.store.record-family`,
    `names.store.intern-identity`, `commit.existence-checked-append-and-patch`,
    `limits.max-task-progress-bounded`, `proposal.batch-bound-enforced`,
    `schema.m1-single-freeze`, `schema.symbolic-int-ranks`,
    `schema.sources-expansion-distinct`,
    `artifact.total-requires-map`, `artifact.map-bounds-checked`,
    `artifact.source-equals-payload`, `artifact.logical-to-raw-remap`,
    `artifact.kind-name-inventory`,
    `scope.event-cumulative-lifecycle`, `scope.event-at-validated`,
    `scope.draft-cardinality`, `scope.file-enter-policy`,
    `symbol.namespace-derived-closed`, `symbol.namespace-miss-not-conflict`,
    `symbol.point-of-declaration-order`,
    `type.canonical-int-single-producer`, `type.function-type-single-producer`,
    `type.reused-id-policy`, `type.no-structural-dedup`,
    `sem.one-record-per-node`, `sem.conversion-at-most-one`, `sem.conversion-role`,
    `sem.effects-empty`, `sem.return-and-function-semrec`,
    `sem.no-t09-functionrecord-cycle`, `sem.checked-return-persisted`,
    `sem.conversion-op-signedness`,
    `const.math-signed-value`, `const.canonical-signed-value`,
    `const.checked-arithmetic-chip-level`, `const.chip-diagnostic`,
    `const.max-const-bits`, `const.max-const-bits-signed-range`,
    `const.unsupported-unverified-width`,
    `ir.op.constant`, `ir.op.add`, `ir.op.return`, `ir.block-committed-vs-terminated`,
    `ir.terminator-greatest-ordered`, `ir.terminator-missing`,
    `ir.terminator-missing-explicit-commit-marker`,
    `ir.op-immediate`, `ir.op-immediate-type`, `ir.op-immediate-type-result-ty`,
    `ir.unsupported-node`, `ir.rejection-rule-id`, `ir.folded-int5-constant-emission`,
    *(`ir.terminator-missing` / `ir.terminator-missing-explicit-commit-marker` and
    `ir.op-immediate-type` / `ir.op-immediate-type-result-ty` are **alias pairs left
    unresolved**: the `/6` inventory must collapse each to one id — H8.)*
    `lex.literal-record`, `literal.t04-t08-handoff`,
    `literal.handoff-lexical-candidate-plus-sem-per-use-request`,
    `artifact.m1-produced-set`,
    `artifact.source-equals-payload`,
    `symbol.identifier-leaf-decl`, `scope.block-node-boundary-at`,
    `manifest.store-owner-allowlist-mandatory`, `manifest.allowlist-chip-id-keyed`,
    `pipeline.stage-assignment-versioned`, `pipeline.per-stage-queues`,
    `pipeline.dispatch-order`, `pipeline.fairness-deterministic`,
    `pipeline.backpressure-bounded`, `pipeline.committed-id-cursors`,
    `pipeline.own-batch-continuation`, `pipeline.await-children`,
    `pipeline.progress-canonical-stage-queue`,
    `pipeline.join-reinsert-commit-apply`, `pipeline.wait-set-only`,
    `pipeline.single-writer-per-wire`, `pipeline.selection-errors-not-commiterror`,
    `pipeline.inflight-quota`, `pipeline.quota-baseline-one`,
    `commit.batch-ordered-atomic`, `commit.one-transition-per-dispatch`,
    `commit.cross-task-check-order`, `commit.cross-task-write-conflict`,
    `commit.failure-capacity-atomic`,
    `replay.dispatch-order-determinism`, `replay.scheduler-snapshot-deterministic`,
    `baseline.canonical-projection`,
    `node.token-range-no-span-write`, `token.reuse-committed-pp-span`)
    **and an `M1AppendSchema` section proposed to be hashed at `/6`**.
    `M1AppendSchema` is the single section proposed to be hashed at `/6`; it must include, exactly:
    (a) every appended `(StoreId, field, RecordFamily, backing-arena)` pair from
    §8 (19 draft families: **18 arena-backed** + `Name`; wire tags 0–26;
    `RecordFamily` ordinals separate);
    (b) the per-family record-field identifiers and field order, pinned after
    §15.3 owner sign-off;
    (c) the closed enum-code inventories (`PpTokenKind`, `NodeKind`, `ScopeKind`,
    `ScopeEventKind`, `SymbolKind` + derived `Namespace`, `IntRank`, `CharKind`,
    `TypeKind`, `ValueCategory`, `EffectMask`, `ConversionOp`, `ConversionRole`,
    `LiteralKind`, `LiteralSuffix`, `IrOp`,
    `ArtifactKind` (total 8 + `requires_map`), `ParseContext`, `Linkage`,
    `StorageDuration`);
    (d) the §9 task-kind codes and their stage assignments;
    (e) the mandatory store-owner allowlist rows keyed by `ChipId` with a
    seed/signature proposed to be hashed at `/6`;
    (f) the limits (`max_inflight_per_tick`, `stage_queue_bound`,
    `max_const_bits`, `max_task_progress`, …) — **Rev 38 (rev-51):** the redundant
    `max_dispatches_per_tick` is **dropped from the candidate limit inventory**
    (`max_inflight_per_tick`/quota is the sole per-tick dispatch-count bound;
    delegated candidate default, not a freeze);
    (g) the per-final-record snapshot bit-encodings (the `encode_*` functions);
    (h) **all** `NORMATIVE_RULES` ids listed above.
    The section must be internally consistent: the family, tag, and rule counts
    stated in this document must equal the encoded counts. `[audit2 N11, selected, user pipeline direction]`
13. `compiler/src/lib.rs`/`prelude.rs`: export new modules/types.
14. `ReservedArena` → `TypedArena` for the M1 Part A families and the C05
    snapshot change from reserved-ID-only to typed-body encoding; update T01
    §7.1 and `compiler/README.md` explicitly.
15. Tests/docs: `compiler/tests/**`, `compiler/README.md`,
    `COMPILER_SFL_MANIFEST.md`. Tests: `StoreSchema::foundation() ⊕
    M1AppendSchema == CompilerBus::try_new(...).schema` byte-for-byte (AD5);
    per-backing-arena capacity for `sources.spans`/`symbols.scopes`/`ir.*`
    (BLK-1); tick-start active reset and end-of-tick `tasks.active == None`
    (BLK-2); CT03 invoked exactly once through its adapter, no direct `compute`
    (BLK-3); canonical `bus.report` snapshot/hash and host-`Trace` view (C1);
    `Task` progress fields encoded (C4); payload `SourceId` uniqueness (C3);
    per-final-record encoding round-trip (C2). Audit-5: budget/cancel gate and
    `report.trace.len() <= max_ticks + 1` with exactly one terminal record,
    terminal halt, and repeated `step` calls returning the stored terminal
    record without appending (A); `wires.selected` recorded (B);
    conditional source rule (C); one-chip-per-layer (D); `InternalMissingArena`
    and checked-add capacity (E). Audit-6/7: single append site — `max_ticks == 0`
    yields exactly **one** terminal record (no double append) and repeated
    terminal `step` calls do not append; `checked_add(max_ticks, 1)` config
    rejection and checked tick advance (no wrap); terminal `job_state`
    (`Finished`/`Failed`) halts `step`; `Motherboard` private / `step`-only;
    worker-layer range `>= WORKER_BASE`; source-scoped checked after the batch
    scan. Audit-8: step 6 records `Finished`/`JobFailed` from post-commit
    `job_state` and sets `last_outcome`; `CompilerConfig::try_new` rejects
    `max_ticks` overflow before construction; `advance_tick` is non-wrapping.
    Audit-9: `CompilerConfig::try_new`/`CompilerBus::try_new` reject
    `max_ticks = u64::MAX`; the terminal tick ends at `control.tick == max_ticks + 1`
    (early terminal `T + 1`), after which `step` never advances or appends.
    Audit-10: `control.tick_overflow` is defensive-only (no terminal report; the
    guard returns the stored `last_outcome`; the flag/snapshot observe it);
    `CompilerConfig::new`/`CompilerBus::new` are `pub(crate)` and tests migrate to
    `try_new`.
    `[audit3 AD5, audit4 BLK-1..3/C1..C4, audit5 A..E, audit6/7 BLK-6.x/7.x, audit8 BLK-8.1..8.3, audit9 BLK-9.1/9.2, audit10 BLK-10.1/10.2]`

16. T01 §4 terminology and **symbolic-model** correction (**`/6` doc action only;
    do not edit the frozen T01 §5/`/5` here**): (i) reword "the control chip
    selects ready tasks" and "the commit chip writes persistent queues/results"
    to "the control chip **computes** the selection; the integration/dispatcher
    applies it" and "the **commit path/backend** writes persistent queues/results",
    consistent with T01 §4.1 (a `RestrictedChip` cannot touch the bus); and
    (ii) align T01's integer-value wording ("target bit-width + bit-pattern +
    signedness", T01 §3) with the M1 Part A **symbolic** rank/signedness model,
    recording that concrete width/bit-pattern is Part B/probe-gated. Recorded in
    this proposal's `/6` inventory only; no T01/`/5` edit is made now.
    `[cross-cutting finding]`

17. Cross-integration schema additions (one `/6` freeze; no half-defined records):
    - `ids.rs`: new `SemId`, `ScopeEventId`, `LiteralId`; `RecordRef` appends
      `Sem` (tag 24), `ScopeEvent` (25), `Literal` (26), preserving tags 0–23.
      Parse state reuses the existing `Continuation` family/tag; `Artifact` keeps
      its existing tag. `RecordFamily` ordinals are a **separate** inventory from
      the wire tags and are **proposed to be pinned separately at `/6`** (not pinned today).
    - `task.rs`: `RECORD_KINDS` gains `sem`, `scope_events`, `literals`;
      `StoreId` gains only `Names`; `symbols.scope_events`, `tasks.continuations`,
      `lex.literals`, and `artifacts.fragments` are append fields (not new
      stores).
    - `bus.rs`: new `sem`, `scope_events`, and `literals` typed arenas (reuse
      `continuations`, `artifacts`); `ArtifactKind` total 8-variant set with total
      `requires_map`; `ArtifactRecord` gains `source`/`raw_offsets`;
      `arena_allocated` extends to the new families.
    - `snapshot.rs`: `encode_literal`, `encode_scope_event`,
      `encode_artifact` (with `raw_offsets`), `encode_continuation` appended;
      `RecordRef` tags 24–26; `artifact_kind_name` total 8.
    - Hash: the `M1AppendSchema` section is **proposed to pin** the appended
      `(StoreId, field, RecordFamily)` pairs and, after §15.3 sign-off, the
      record-field ids and closed enum variants; `NORMATIVE_RULES` would then gain
      the task/report/source/literal/cross-task rules. **This is prospective `/6`,
      not a present-tense hashed claim (rev 25 F1).**
    No family is listed in the hash until its owner signs off (§15.3); unresolved
    shapes stay blockers, not frozen placeholders. **Proposed for pinning at `/6`:** the `IrOp`
    op table (arity/immediate/result/terminator), the closed enum variants
    (`PpTokenKind`, `NodeKind`, `ScopeKind`, `SymbolKind` with derived namespace,
    `TypeKind`/char, `ValueCategory`/`ConversionOp`/`ConversionRole`,
    `LiteralKind`/`LiteralSuffix`, `ArtifactKind` total 8 + `requires_map`,
    `ParseContext`, `Linkage`, `StorageDuration`), and the widened
    `SpanRecord.start/end: u64` (`<= max_source_bytes`). The counts (consistent
    with **§3 item 6** and §8; the earlier `§3.6` reference was dangling — H8):
    **19 draft families** = **18 arena-backed**
    (`Span, Expansion, PpToken, Token, Literal, Node, Scope, Symbol, Type, Sem,
    Const, Function, Block, Value, Instruction, ScopeEvent, Artifact,
    Continuation`) + `Name` (intern table, excluded). Wire tags are `0–26`
    (27 `RecordRef` variants). `names.entries` is T04-only, `lex.literals` is
    T04-only, and `types.records` is keyed by **`ChipId`** (TY13 canonical `int`,
    TY17 function type): the **mandatory** schema store-owner allowlist in
    `ManifestRegistry::register` rejects any other writer
    (`ManifestError::StoreOwnerViolation`, rule
    `manifest.store-owner-allowlist-mandatory`); the earlier "optional VF04
    fallback" is removed. `ConstRecord` (`value: i128`,
    mathematical signed value, checked ops) is **proposed to be** pinned in the
    `/6` hash — **not pinned today** (rev 25 F1); checked
    arithmetic is chip-level (`const.checked-arithmetic-chip-level`) and its
    overflow/unsupported results are **chip diagnostics**, not commit errors. The
    point-of-declaration order (`symbol.point-of-declaration-order`), the
    wrong-namespace-is-a-miss rule (`symbol.namespace-miss-not-conflict`), the
    chip-specific single-producer type policy
    (`type.canonical-int-single-producer`/`type.function-type-single-producer`),
    and the Sem/IR/artifact/literal rules above are **proposed to be** pinned in
    the same `/6` hashed section. Also **proposed for the same `/6` hashed
    section** (rev 21): **`sources.spans` is
    T03-only** (T04/T05 write no spans; `token.reuse-committed-pp-span`/
    `node.token-range-no-span-write`); the **M1-produced `ArtifactKind` set is
    `Normalized`/`Spliced`/`CommentFree`** with `Preprocessed` declared-not-
    produced; **every checked node including `Return`/`FunctionDefinition` has a
    `SemRecord`** with no T09-`FunctionRecord` cycle; the **`max_const_bits`
    signed-range formula** (default 128); and the **`Constant` immediate =
    result-type** rule. `[T03/T04/T05/T06/T07/T08/T09 rev20/21, cross-cutting]`

18. **Pipeline scheduling amendments (user in-principle direction; ADR-0002).** One
    `/6` freeze pins, together with `M1AppendSchema`:
    - **schema/`task.rs`:** `StageId`/`STAGE_NAMES`; `SelectionEntry`/
      `SelectionBatch`; `ContinuationRef`/`ChildRef`/`Proposal::AwaitChildren`;
      extended commit-order key `(dispatch_ordinal, enqueue_ordinal, TaskId,
      proposal index)`; in-flight/`dispatch_cursor` fields; canonical
      `stage_queues`; the pipeline errors (§6.4) — **only `BackpressureCapacity`
      and `CrossTaskWriteConflict` are `CommitError`; the selection-size/duplicate/
      inflight/dispatch-budget errors are dispatcher/scheduling failures; stage
      errors are `ManifestError`s**.
    - **routing:** `RoutingTable::register` enforces total `kind → stage`
      coverage, unique stage ordinals, worker layer `>= WORKER_BASE`, and that the
      declared stage matches the routed layer.
    - **snapshot:** encode `stage_assignment_version`, canonical stage queues,
      in-flight set, `dispatch_cursor`, and the `PipelineMetrics` register.
    - **limits:** `max_inflight_per_tick` (default 1; sole per-tick dispatch bound —
      `max_dispatches_per_tick` dropped per rev 51),
      `stage_queue_bound`, plus `ConfigError` variants for
      invalid quota/queue/fairness values; all bounds enforced pre-mutation.
      **Rev 33 (H9):** `max_inflight_total` is **removed** from the candidate
      (per-tick dispatch is bounded by `max_inflight_per_tick`, the stage queues,
      and `max_tasks_total`; `Waiting` is not in-flight; `tasks.in_flight` is an
      ephemeral per-tick batch cleared at latch after every task's outcome) —
      pending T01 integrator acceptance, no bound frozen.
    - **`T02_CONTROL_CHIPS.md`:** batch semantics for CT03 (bounded batch),
      CT05 (whole-batch validation incl. batch-scoped cross-task checks and the
      write-conflict predicate), CT06 (whole commit), CT07 (joins over committed
      IDs incl. resumed `AwaitChildren`), CT09 (advance only when all
      stage queues are empty/waiting), CT10 (batch progress/budget; canonical
      stage-queue reinsert), CT13 (in-flight cancel set).
    - **`T13_VERIFICATION_CHIPS.md`:** extend VF02 (one transition per dispatched
      task across the batch, no dead-wait), VF03 (wire lifetime with a batch
      selection), **VF04** (batch write-conflict access audit), and VF13 (replay
      across dispatch order); **VF12 is `IrInterpretChip` and is unrelated to the
      pipeline unless explicitly extended** — do not assign it pipeline checks.
      Add fixtures for quota=1 canonical-projection equivalence, multi-inflight
      determinism, cross-task conflict ordering, and backpressure.
    - **gate:** no chip wave F1–F6 may implement or depend on these registers
      until this freeze lands; quota `> 1` is a post-freeze, measured,
      integrator-accepted change. The `/6` inventory in
      [ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) §3
      is the companion list.

**Hash-coverage decision (adopted by this proposal).** The M1 append schema is
encoded as a new `M1AppendSchema` section inside `FrozenSchema::encode`, and
the runtime bus constructor (`CompilerBus::try_new`) seeds `StoreSchema` from it
so manifest registration cannot diverge. This deliberately changes the current rule that group-declared
fields are hash-excluded; the integrator must reconcile
`COMPILER_SFL_MANIFEST.md` §4 (which already claims adding fields changes the
hash) with `contract.rs`/`compiler/README.md` (which exclude them) as a required
document change. `StoreSchema::declare` remains for non-frozen group fields; it
is not sufficient for M1. `[audit2 N11, audit11]`

**Rev 37 (rev-50 two-tier hash scope; §22.5).** The user accepts the **two-tier
model**: `StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed that
participates in the `/6` contract hash**, while **post-seed runtime
`StoreSchema::declare()` extensions stay hash-excluded** and are captured/validated
through runtime snapshot/schema mechanisms. At `/6` integration T01 must
**atomically** update the `COMPILER_SFL_MANIFEST.md` §4 wording, the
`hash_excludes` semantic token (scoped to **post-seed runtime declarations**, not
the frozen M1 seed), and `FrozenSchema::encode`/contract code plus the freeze test,
**preserving the frozen `/5` hash and history**. This settles the conceptual
boundary only: **no exact `M1AppendSchema` contents/counts are accepted, nothing is
frozen, no `/5` change or code is authorized**, and the numeric inventory,
dual-inventory encoding (wire tags vs `RecordFamily` ordinals), numeric-value
inclusion, and the seed-equality/self-consistency test remain pending T01 co-freeze
after the owners.

**Rev 38 (companion state; §23.6).** At this snapshot the [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)
is at **rev 51**, [ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md)
is at **Revision 17**, and [T02](T02_CONTROL_CHIPS.md) is at **rev 31**; the M1
target acceptance is at **rev 29** (which aligns the IR28 committed `FunctionEnd`
terminal result + T01 phase-2b hook with the two-tier hash scope) and the M1
vertical acceptance at **rev 27** (single-source `Normalized` only). The rev-50
two-tier scope and the rev-49 H11/T09/T01 direction remain **current**; the
rev-51 delegated candidate default is **not** a hash-scope change and freezes
nothing. `/5` stays current and `/6` stays unfrozen. **Pointer correction (same
rev 38):** the CDR has since advanced to **rev 52**, which **points at this
proposal rev 38**; the **rev 51** above is the CDR state at the rev-38 integration
snapshot, not the current CDR revision.

**Rev 39 (companion state; §24; pointer-corrected in place).** The current companion [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)
is at **rev 55** (which points at proposal **rev 40 (current)** as its shared resulting file),
[ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) is at
**Revision 17**, [T02](T02_CONTROL_CHIPS.md) is at **rev 35** (checklist precision + T13 H6-M link fix),
[T03](T03_PREPROCESS_CHIPS.md) is at **rev 56**
(`raw_offsets` boundary map + M1 PP01 contract candidate + DOC-06 PP03/PP04 scan-state candidate), [T04](T04_LEX_CHIPS.md)
at **rev 47 + task revisions 1–3** (`Lx08CandidateType { Int }`; reciprocal token↔literal links),
[T13](T13_VERIFICATION_CHIPS.md) at **rev 11** (H6/H9 verification matrix, H6-M01..H6-M16), and the M1
target/vertical acceptance at rev 37/rev 34 (values as of 2026-10-05; each document's own revision
record is authoritative). All are **candidate/prospective** and freeze nothing;
`/5` stays current and `/6` stays unfrozen.

What does not by itself require a bump (runtime, hash-excluded,
snapshot-covered): registering `TaskKind`s/routes at runtime; adding group chip
modules, manifests, and tests; adding non-M1 record body files.

`RESIDUAL`: whether `/6` freezes all Part A families at once (recommended) or an
envelope `/6` plus family increments.

---

## 13. Test plan

Per the DoD (T01 §5) and M1 gates G7–G13. Every failure is a structured
diagnostic; no panic, no unbounded loop, no partial commit.

Normal: single/multi-draft batch; intra-batch forward ref and cycle
(`Function.entry ↔ Block.function`); cross-tick committed ref; `Complete` with
`DraftRecords`; same-batch `NameDraft` resolving to a newly interned name; a
`NameDraft` deduping to an existing `NameId` (no body, no audit patch, predicted
ID reused); a `SpanDraft` referencing a committed `Source`; `Progress`
rescheduling to `Ready` next tick; `StorePatch` accompanying a transition.

Boundary: batch at the proposal budget; **per-backing-arena** capacity for
multi-arena stores (`sources.spans`/`sources.expansions`, `symbols.scopes`/
`symbols.symbols`, each `ir.*`) independent of the `StoreId` (BLK-1); intern
entries/bytes at capacity; `max_records_total` at bound (arena bodies + audit
patches); `DraftRef == len`; empty batch rejected; `progress_count ==
max_task_progress`; queue at bound with a `Progress` reinsert.

Invalid (fail before mutation): undeclared append field; field not in the chip's
`writes`; wrong owner/task attribution; wrong link family
(`DraftFamilyMismatch`); missing committed reference (`CommittedRefMissing`);
`SpanDraft.source` not a committed `Source` (`SourceNotDeclared`); a source-scoped
task payload with zero or more than one `RecordRef::Source`
(`SourceNotDeclared`/`MultipleSourceRefs`); span/expansion
source mismatch (`SpanSourceMismatch`); `AppendRecords`+`StorePatch(Append)` on
the same field (`DuplicateAppendMechanism`); `DraftRecords` without a batch;
second `AppendRecords`; two transitions; a dispatched task with no transition
(`TaskNotTransitioned`); non-increasing progress ordinal; stale `StorePatch`
version.

Capacity/atomicity: each failure asserts **no** arena growth, **no** intern
growth, **no** patch-log growth, unchanged store versions, and the task
terminated exactly once. A `StorePatch`-only **or empty** proposal set for the
dispatched task must yield `TaskNotTransitioned` before any mutation and must
not strand it `Running`; the `Progress` queue bound uses checked arithmetic and
does not double-count the dispatcher's removal. After every tick `tasks.active`
and `control.selected` are `None` (BLK-2); CT03 is ticked exactly once through
its adapter and never via `compute` (BLK-3).

Write-scope: M1-WS-01..05 adapted to append areas; `config` read-only;
undeclared registers/fields byte-for-byte unchanged after a rejected batch. The
store-owner allowlist is **mandatory** (no optional `VF04` fallback) and keyed by
**`ChipId`**: a non-T04 `names.entries`/`lex.literals` writer and a
non-chip-allowlisted `types.records` writer are rejected by
`manifest.store-owner-allowlist-mandatory`
(`ManifestError::StoreOwnerViolation`). **`sources.spans` is T03-only (rev 21):**
a manifest declaring T04/T05 as a `sources.spans` writer is a `StoreOwnerViolation`
(no shared-writer carveout); the cross-task conflict rule covers only two tasks of
the *same* owner group appending one append-only field or patches to the same
record. (`node.token-range-no-span-write`, `token.reuse-committed-pp-span`.)

Replay/determinism: identical source/config/state ⇒ identical IDs, name-intern
order, body bytes, snapshot hash, and trace hash across ≥3 runs; snapshot
encodes wires, **every** typed M1 body via the per-final-record encoders, name
entries, `Task` `progress_ordinal`/`progress_count`, the canonical `bus.report`
(bounded by `max_ticks + 1`, one terminal record), and audit order; the runtime
`StoreSchema` seed equals `foundation() ⊕ M1AppendSchema`.

Tooling: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets
-- -D warnings`; `cargo test --workspace` plus the nested compiler package and
`freeze.rs`; `tools/chip-lint` over the new chip tree.

Driver lifecycle (§10.2, audit-5/6/7/8/9/10): (A) `pins.tick_budget_reached` or
`control.tick >= max_ticks` sets `control.budget_exhausted = true` and a
`BudgetExhausted` tick outcome; `pins.cancel` sets a `Cancelled` outcome. **Step 1
never appends**; step 6 is the sole append site and selects the outcome from
post-commit `job_state` (`Finished` -> `Finished`, `Failed` -> `JobFailed`, else
`Idle`/`Executed`), setting `last_outcome`. `max_ticks == 0` yields exactly one
terminal record at tick 0; the trace is bounded by `checked_add(max_ticks, 1)`;
`CompilerConfig::try_new`/`CompilerBus::try_new` reject `max_ticks` overflow
before construction; `CompilerConfig::new`/`CompilerBus::new` are `pub(crate)`.
`advance_tick` is non-wrapping and, on the impossible overflow, sets
`control.tick_overflow` + `job_state = Failed`; because it runs after step 6 it
**cannot** append a report, so the next `step` returns the existing `last_outcome`
without appending and the overflow is observable only via the `tick_overflow`
flag/snapshot. The terminal tick performs one root `latch`/`advance_tick`, leaving
`control.tick == max_ticks + 1` (early terminal `T + 1`); after terminal,
`CompilerDriver::step` returns the stored terminal record and never ticks or
appends again (repeated calls are idempotent).
(B) the canonical `TickRecord.dispatched` set is recorded after dispatch, and at
quota 1 `wires.selected == Some(task)`/`TickRecord.selected` is the compatibility
projection (`None` at quota > 1, on idle/budget/cancel); cleared by the next
`reset_wires`. (C) **`source_scoped_one_hop` test (T03 rev18):** a batch is
source-scoped iff the batch scan finds any of `SpanDraft`, `ExpansionDraft`
(`spelling`/`expanded`), `PpTokenDraft.span`, `TokenDraft.span`,
`NodeDraft.first_token`/`last_token`, or `ArtifactDraft.source`, **or** a
committed direct link to a live `Span`/`Expansion`/`Token`; then the task must
carry exactly one `RecordRef::Source`, and every directly referenced
Span/Expansion/Token (draft or committed) must equal it; a committed
`Span`/`Expansion`/`Token` link is dereferenced one level, a committed `Node`
range is checked through its tokens (not walked), and `ArtifactDraft.source` must
equal the payload source. A non-source task (control/host/IR/verification) with
zero sources passes; zero/multiple, a span/expansion mismatch, and an artifact
source mismatch are rejected (`SourceNotDeclared`/`MultipleSourceRefs`/
`SpanSourceMismatch`/`ArtifactSourceMismatch`). (D) `RoutingTable::register` rejects
a worker layer `< WORKER_BASE` or equal to `SELECT_LAYER`/`GUARD_LAYER`, and a
second route claiming the same worker layer (`SELECT_LAYER < GUARD_LAYER <
WORKER_BASE`), and only the routed worker layer is ticked. (E) `arena_allocated`
returning `None` inside the preflight loop is `InternalMissingArena`; `Name` can
never enter `counts`. (F) pipeline (ADR-0002): at quota 1 the batch has ≤ 1 entry
and the **canonical semantic comparison projection** (not a byte-identical
snapshot) equals the rev-18 single-active-task result; the new scheduler snapshot
is separately replay-identical run-to-run; at quota `> 1`
the ordered dispatched set, in-flight bound, per-stage queue bound
(`BackpressureCapacity`), dispatch budget (`DispatchBudgetExceeded`, **dropped by
the rev-51/54 candidate**), and
`ManifestError::StageUnassigned`/`StageLayerMismatch` are enforced before
mutation; a same-tick cross-task **write** conflict is rejected pre-apply in
`(dispatch_ordinal, enqueue_ordinal, TaskId, proposal_index)` order
(`CrossTaskWriteConflict`) and the result is iteration-order independent; the
audit owner is **VF04 extended with the batch conflict check** (VF12 is
`IrInterpretChip`, not the conflict verifier). No
throughput claim is made by any of these tests.

Rev-21 named tests (each mapped to its error/rule; owners stay conditional):
- **T03/T04:** `artifact_map_len_first_monotonic_last` (`ArtifactMapInvalid`);
  `bytes` limit; `artifact_source_equals_payload` (`ArtifactSourceMismatch`);
  `artifact_map_mandatory_requires_source` (`ArtifactSourceMissing`);
  `artifact_map_optional_empty` (Assembly/Object/Snapshot/Trace: empty
  `raw_offsets` required; a synthetic `Trace` carrying a valid `Some(source)`
  plus empty offsets is **accepted**, and a non-empty optional map is rejected;
  exact error classification/numeric codes open — rev-47 delegated candidate
  default); total
  `ArtifactKind::requires_map` = 8 variants; `m1_produced_artifact_set`
  (`Normalized`/`Spliced`/`CommentFree`; `Preprocessed` not produced);
  `logical_offset_remapped_to_raw_span`; committed `Artifact` source read-back;
  one-hop committed-span dereference (`PpToken`/`Token` spans) source match;
  `token_reuses_committed_pp_span` (T04 writes no `sources.spans`); duplicate-name
  idempotence (one intern entry); `expansion_spelling_expanded_one_hop_source`;
  `literal_record_committed`/`literal_t04_t08_handoff` asserting that the handoff
  **preserves the semantic information** `node`/`required_kind`/`legality`/`LX08`
  type (T08 reads `RecordRef::Literal`, commits `ConstRecord`; T04 never writes
  `constants.records`). **H1:** the test asserts the **accepted-in-principle**
  allocation (per-literal `LiteralRecord` = lexical facts + `LX08`; sem-stage
  `ConstantRequest` = `node`/`required_kind`; result = `legality`) as the working
  basis, and must not be read as freezing the exact fields.
  **Rev 38 (§23.3/§23.5):** `artifact_source_equals_payload` is **not** asserted in
  M1 — the source-provenance/equality relation is **deferred**, and the M1
  exercised artifact-map path is **only** the single-source `Normalized` artifact
  (`Spliced`/`CommentFree`/`Preprocessed` schema-declared but **not
  produced/asserted**); `m1_produced_artifact_set`/`artifact_map_optional_empty`
  are prospective. The exact `raw_offsets` semantics, the token↔literal linkage,
  and the `Lx08CandidateType` closed set remain **open T03/T04/T01 blockers**.
- **T05:** `node_children_by_parent_sorted_unique_ordinal`
  (`NodeOrdinalNotUnique`); `node_parent_acyclic` (`NodeCycle`);
  `node_partial_children_counter_consistent`; `node_token_range` (first/last
  `TokenId`; T05 writes no span); `node_token_range_committed_source`
  (`SpanSourceMismatch`; **H5**: committed-only — the same-batch `TokenDraft`
  branch/test is removed);
  `parse_cursor_non_advance` (`ParseCursorDidNotAdvance`, chip diagnostic);
  `parse_depth` (`ParseDepthExceeded`, chip diagnostic, reuses `max_task_depth`);
  `continuation_own_batch_resolution` (`ContinuationRef::OwnBatch`);
  `continuation_ref_validated_preapply` (`ContinuationRefInvalid`);
  `await_children_own_batch` (`AwaitChildren`; `AwaitChildrenRefInvalid` for a
  cross-task/out-of-range index); `no_continuation_awaited_field` (WaitSet-only);
  `join_reinsert_stage_queue` (commit-apply reinsertion, CT07 decision-only);
  `join_then_replay_retry` (OPEN-02: replay/retry between the join and the parent
  execution neither double-consumes nor loses child results; consumption stays
  with the parent's own atomic commit, or the results are in parent-owned durable
  state);
  `continuation_chain_acyclic` (`ContinuationCycle`); PA38 recovery
  (`M1-NEG-08/09/11`, `M1-REC-05`); PA20 unary-`+` path only.
  **Rev 38 (§23.2):** `parse_context_vocabulary` is a **delegated candidate**
  (10-member `ParseContext`; exact discriminants/mapping/unknown-tag errors open);
  no test freezes it.
- **T06:** `scope_event_cumulative_lifecycle` (committed + new; duplicate Enter,
  duplicate Exit, Exit-before-Enter -> `ScopeLifecycleViolation`);
  `scope_new_draft_cardinality`; `scope_file_enter_policy`;
  `scope_block_node_boundary_at`; `scope_event_at_validated`;
  `symbol_identifier_leaf_decl` (the `Identifier` leaf, not a wrapper; identity/
  diagnostic location, **not** the visibility boundary);
  `symbol_declarator_visibility_contrast` (DOC-12; C11 6.2.1p7: `int n = 3;
  void f(void) { int n[n]; }` bound resolves to the outer `n`; `int n = n;`
  initializer resolves to the declared inner `n`);
  `symbol_point_of_declaration_lookup` (before -> missing, preserves `M1-TY-06`);
  `symbol_namespace_miss_not_conflict`; `symbol_structural_duplicate`
  (`SymbolConflict`; no compatibility model); `char` fixture (`CharKind`);
  `canonical_int_reused_id`/`function_type_reused_id`
  (`type.canonical-int-single-producer`); `ty08_non_m1`.
- **T07:** `sem_one_record_per_node` (`DuplicateSemRecord`); `return_sem_record`
  and `function_definition_sem_record` (every checked node incl. `Return`/
  `FunctionDefinition`); `no_t09_functionrecord_cycle` (T09 reads T07
  `SemRecord`s); append-order Sem lookup; `conversion_plan_nested_links`
  (`links()` recursion); `conversion_role_required`;
  `conversion_at_most_one_plus_vf_completeness` (pinned `(NodeKind, role, op)`
  matrix); `conversion_op_signedness`; `effect_mask_empty`
  (`EffectMaskUnsupported`); `checked_return_persisted`;
  `const_literal_vs_semantic_request` (two request kinds, committed-ID staging).
- **T08:** `const_checked_arithmetic` (`ConstOverflow`, **chip diagnostic**);
  `const_unsupported_unverified_width` (`ConstUnsupported`, **chip diagnostic**);
  `const_max_const_bits` (`i128::MIN`/`i128::MAX` representable, synthetic
  `i128::MAX + 1`/`2^128-1` = `ConstOverflow`); `const_canonical_signed_value`;
  feasible vectors `2`,`3`,`5` (no invented i128 overflow);
  `constants_writer_handoff` (`manifest.store-owner-allowlist-mandatory` for
  `constants.records`); `const_overflow_not_commiterror` (no such `CommitError`).
- **T09:** `ir_op_table_hashed` (**prospective `/6`** — the op table is not
  claimed hashed yet, H8); `ir_block_committed_vs_terminated`
  (`BlockMissing`/`BlockTerminated`); `ir_terminator_greatest_ordered`
  (`TerminatorNotLast`); `ir_terminator_missing` (marker-based trigger
  **direction only**; the `IR28` function-completion marker family/schema is
  unresolved, H11. **Rev 37/rev-49:** the operative trigger is the **committed IR28
  `FunctionEnd` terminal result** (`TaskState::Completed(ResultId)`) plus a
  **T01-owned typed phase-2b commit-apply validation hook**; the marker-based form
  is superseded/historical; the hook's exact contract and VF fixtures remain
  T01/T13 pending); `ir_value_unique_producer`
  (`ValueProducerMissing`/`ValueProducerDuplicate`); `OpArityMismatch`/
  `OpImmediateMismatch`/`OpImmediateTypeMismatch`/`OpResultMismatch`;
  `constant_immediate_result_type` (immediate type = result `ValueRecord.ty`);
  `ir_rejection_rule_id` (the alias pairs `ir.op-immediate-type*` /
  `ir.terminator-missing*` are unresolved, H8);
  `UnsupportedIrOp`/`UnsupportedNode` (chip diagnostics);
  `folded_int5_ir_constant` (H7; T08 computes the `ConstRecord` value, T09 emits
  the IR `Constant`) separate from the probe-gated fixture
  `M1-CL-02` (chip `CL03`); `Unsupported` negative interpreter case (VF12).
- **Cross-record/cross-task:** `manifest.store-owner-allowlist-mandatory` rejects
  a non-T04 `names.entries`/`lex.literals` writer, a non-T03 `sources.spans`
  writer, and a non-chip-allowlisted `types.records` writer (`StoreOwnerViolation`;
  the batch-conflict audit is a **proposed** VF04 extension, not defined today);
  `cross_task_write_conflict` (`CrossTaskWriteConflict`; same-owner-group
  append-only appends allowed, same-`RecordRef` patches rejected);
  `selection_errors_not_commiterror`;
  `quota1_canonical_projection_equivalence`; `scheduler_snapshot_replay_identical`.
  `[T03/T04/T05/T06/T07/T08/T09 rev20/21; audit H1/H2, findings 3–9]`

Gated/not-run: every fixture asserting a concrete width/alignment/ABI value stays
blocked on the probe; the only Part B check in scope is the negative fail-closed
path (`N1`). No probe, no execution, no pass rate is claimed.

---

## 14. Ownership boundaries

- **Integrator only:** workspace/Cargo, `compiler/src/{bus,task,commit,snapshot,
  contract,manifest,ids,arena,limits,intern,routing,codec,materialize,driver,
  report}.rs`, the frozen M1 append schema, `TaskKindRegistry`/`RoutingTable`
  wiring, `CONTRACT_VERSION`/hash, shared tests.
- **Group owners:** their record body + Draft wrapper (proposed
  `compiler/src/records/<group>.rs`), the chip's `RestrictedChip` + `Adapter`
  under `compiler/src/chips/<group>/`, its manifest `reads`/`writes`, and its
  tests; change requests for shared files (`PARALLEL_EXECUTION.md` §2).
- **No** group edits another group's store, the envelope, the commit path, or the
  driver.

---

## 15. Pre-freeze acceptance checklist

`SELECTED` items are doc-level decisions recorded by the integrator (user
"continue", 2026-10-04); `FROZEN-PROPOSAL` items still require the named owner's
sign-off before `/6`. Nothing here is implemented.

### 15.1 Selected integration contract decisions (selected/direction only, 2026-10-04; not accepted)

1. `SELECTED` **AB1a:** CT03 is invoked only through its
   `ProjectedChip`/`ChipAdapter` (writing `wires.selection`); the dispatcher
   applies it, with CT03 excluded from the routed invocation. Recorded in T02.
2. `SELECTED` **AB1b:** CT05 is validation-only; CT06 proposes the commit
   decision and the commit path/`CompilerBackend` performs the mutations (no
   commit side-effect inside a chip). Recorded in T02.
3. `SELECTED` **AB2:** relocate `TickOutcome`/`TickReport`/`Resolution` and
   `fail_selected` to `report.rs`/`driver.rs`; `bus.report` is canonical snapshot
   state encoded by `Snapshot::capture`.
4. `SELECTED` **single owner (user, in principle 2026-10-04):** `sources.spans`
   is **T03-only**; the rev-20 C6 shared T03/T04 writer is **withdrawn**. T04
   `TokenRecord.span` reuses the committed T03 PP span and T04 writes no span;
   `NodeRecord` carries a first/last `TokenId` range and T05 writes no span.
   `sources.expansions` stays T03-only. The Guardrail is unamended; T03/T04/T05
   exact fields remain owner sign-off items.

### 15.2 Integrator-selected draft defaults (closed as draft defaults; no owner schema change)

Promoted to concrete draft defaults by the integrator (user "continue",
2026-10-04); doc-level and not implemented. They change no group-owner record
schema. Items 15–18 additionally record the **user's in-principle pipeline
direction** (rev-21 working basis, direction only), but like every item here they
are inert until `/6` and implement nothing.

5. `SELECTED` **Committed-reference existence** is checked for **both**
   `AppendRecords` links and `StorePatch` references (`CommittedRefMissing`),
   pre-mutation and existence-only (no field-type inference beyond `expect`).
6. `SELECTED` **One `/6` full M1 schema freeze** (all families, append fields,
   task codes, and rules in a single version) rather than per-family increments.
7. `SELECTED` **Bounded `max_task_progress: u32`** added to `Limits` and enforced
   at commit (`ProgressLimit`), in addition to `max_ticks`.
8. `SELECTED` **Stable error family/codes**: the §6.4 `CommitError` variants,
   `ConfigError::MaxTicksOverflow` (`config.max_ticks_overflow`), and internal
   `InternalMissingArena` (`DiagGroup::Internal`); exact numeric codes are fixed
   at `/6` from the §6.4 list with no new error families.
9. `SELECTED` **Task-kind codes** follow §9 (`group<<12 | local`, local `>= 16`),
   registered in `TaskKindRegistry` and routed in `RoutingTable`.
10. `SELECTED` **Symbolic integer ranks / no target widths** for Part A records
    (`TypeKind::Int { rank, signed }`, `ConstRecord { ty, value }`); concrete
    width/bit-pattern only behind the probe gate.
11. `SELECTED` **`commit_proposals` enforces the `ProposalBatch` bound**
    (`proposals.len() + total_drafts <= max_proposals_per_tick`); adapters
    declare, commit enforces.
12. `SELECTED` **`NamePlan`/`ResolveTable`/`RecordDraft` interface** is **selected as**
    the §6.2 sketch (a selected draft, **not** a freeze): opaque
    `NamePlan`/`ResolveTable` internals; `RecordDraft`
    exposes `family`/`target`/`links`/`resolve`/`encode`; closed `ResolvedDraft`.
13. `SELECTED` **`sources.expansion`** remains the existing singular source-map
    metadata field; **`sources.expansions`** is the distinct append area for
    `ExpansionRecord`.
14. `SELECTED` **Terminal/report rules** as §10.2 (single append site at step 6;
    `trace.len() <= checked_add(max_ticks, 1)`; terminal predicate
    `budget_exhausted || cancel_requested || job_state ∈ {Finished, Failed}`;
    defensive-only `tick_overflow`; `CompilerDriver::step` returns the stored
    terminal record).
15. `SELECTED` **Pipeline quota baseline** (§10.5, ADR-0002):
    `max_inflight_per_tick` defaults to **1** and the quota = 1 path must equal
    the rev-18 single-active-task result under the **canonical semantic
    comparison projection** (semantic records/results/diagnostics + semantic
    trace minus scheduler-only fields), with the new scheduler snapshot separately
    deterministic/replay-identical. The pipeline is a core
    scheduling strategy; quota `> 1` is a **measured, post-freeze,
    integrator-accepted** change. No throughput claim is made. This does **not**
    freeze a single-active-task architecture as final.
16. `SELECTED` **Committed-ID cursors/joins plus own-task-batch draft keys**
    (§10.5): continuations, stage cursors, and joins are over committed
    `TaskId`/`ResultId`/`ContinuationId` or an **own-task batch** draft key
    (`ContinuationRef::OwnBatch`, `ChildRef::OwnBatch`); never a
    foreign task's draft, wire, or address. **Accepted in principle (rev 30, user
    decision A):** the own-task-batch key is a **transient wire/proposal input
    only**, validated and resolved to committed IDs **before persistent state** —
    the user's **narrow interpretation** of Guardrails §6.1 (guardrail text
    unamended; **T01 implementation-confirmation pending**). `TaskDraft.continuation` is
    `Option<ContinuationRef>`; `Proposal::AwaitChildren` sets `Waiting(WaitSet)`
    (the **only** awaited-child source; no `continuation.awaited` — **accepted in
    principle, user decision B; T01/T05 acceptance pending**); the join
    reinsertion is performed in commit apply. No extra-tick scheduler path is
    required or claimed.
17. `SELECTED` **Pipeline is inert until `/6`** (§12.18): chip waves F1–F6 may not
    implement or depend on the pipeline registers before the freeze; T02/T13
    batch amendments and the pipeline fixtures are in the same freeze.
18. `SELECTED` **Store-owner allowlist is mandatory** (not optional) and keyed by
    **`ChipId`**: a non-allowlisted writer is rejected
    (`ManifestError::StoreOwnerViolation`; seed/signature proposed to be hashed
    at `/6`). The batch
    write-conflict audit is a **proposed** VF04 extension, not defined today.
19. `SELECTED` **Total `ArtifactKind`/`requires_map`** and the committed
    `LiteralId` T04→T08 handoff **direction + H1 allocation** are adopted as the
    rev-21/rev-24 shape: the M1-produced set is
    `Normalized`/`Spliced`/`CommentFree` (`Preprocessed` declared but not
    produced), and the handoff preserves the semantic information
    `node`/`required_kind`/`legality`/`LX08` type. The **H1 allocation split** is
    user-accepted in principle (2026-10-04): `LiteralRecord` = lexical facts +
    `LX08`; `ConstantRequest` = `node`/`required_kind`; `ConstantResult` =
    `legality`. **Rev 25 F3** drafts the concrete (unfrozen) carriers
    `LiteralRecord.candidate_type`, `ConstantRequest { literal, node,
    required_kind }`, and `ConstantResult { value, legality }`. Exact enum
    variants remain T03/T04 owner `/6` sign-off items; this
    is **not a freeze** (§8, §18.1, §19.3, CDR §C2).

### 15.3 Reviewer sign-off matrix (not owner-approved; second review required)

**Rev 22–25:** the accumulated read-only review findings (rev 22), the independent
rev-22 audit H2–H11 (rev 23), the user's in-principle H1 allocation acceptance
(rev 24), and the rev-24 independent audit F1–F9 (rev 25) are added as further
conditions; see the `§17`, `§18`, and `§19` ledgers. No row
becomes a sign-off, and the exact field/enum/rule shapes remain `/6` blockers. All
owners remain **conditional**. **H1** (the literal-handoff allocation) is
user-accepted in principle; T03/T04/T08 still require the enumerated co-freeze and
sign-off before any `/6`.

The rev-19/second-round reviewer reports (T03, T04, T05, T06, T07, T08, T09) and
the rev-20 independent audit are integrated as this revision's candidate. Each is
**conditional** — fixes are incorporated, but the named owner must re-review and
sign off; none is an approval. Exact field-level manifests and enum variants
remain `/6` blockers.

| Owner | Rev-21 conditions | Integrated candidate | Status / pending |
|---|---|---|---|
| T03 | total `ArtifactKind` + total `requires_map`; `Preprocessed` not M1-produced; artifact source equality; pre-mutation map checks; `ExpansionRecord` one-hop source; committed Artifact source read-back; idempotent intern; **sole `sources.spans` owner** | §5 `ArtifactKind` total/`ArtifactRecord`/`ExpansionRecord`; §6.1 `source_scoped_one_hop`; §7 phase 1/2b/3; §8 T03-only spans; §13 named tests | **conditional** — T03 field manifest, exact map shape, and `SpanRecord` fields are `/6` blockers; the single-owner decision is in principle, not signed |
| T04 | decoded-literal handoff representable and preserving the semantic info `node`/`required_kind`/`legality`/`LX08` type (**H1 allocation accepted in principle 2026-10-04**); owner allowlist for `lex.literals`/`names.entries`; **`TokenRecord.span` reuses the committed T03 PP span and writes no span** | §5 `LiteralRecord`/`TokenRecord`; §8 `lex.literals`/T03-only spans; §12.9; §13 `literal_record_committed`/`token_reuses_committed_pp_span` | **conditional** — `LiteralRecord`/`LiteralSuffix` variants, the **H1 carrier field shapes**, and the T04→T08 request shape are `/6` blockers; shared `constants.records` writer not selected |
| T05 | exact field order/encoding (not finalized); `awaited` removed (`WaitSet`-only); `ContinuationRef`/`ChildRef` validated pre-apply; `task.continuation` direction; join reinsertion in commit apply; `partial_children`/counter; **token-range nodes (no span write)**; parse errors as chip diagnostics; PA20 unary-only | §5 `ContinuationRecord`/`NodeRecord`; §6.1/§6.3 `ContinuationRef`/`ChildRef`/`AwaitChildren`; §7 phase 1/2b/4; §8; §9/§11 PA20; §13 named tests | **conditional** — exact fields, wire encoding, and two-tick flow are **hard `/6` owner blockers**; the span blocker is closed by the token-range decision, but the loop/join realization is not signed |
| T06 | **`Identifier`-leaf `decl`** (identity/diagnostic; visibility from the complete declarator, DOC-12); active-scope-chain lookup + tie rule; `ScopeDraft` cardinality + File-Enter policy + **Block-node boundary `at`** + `ScopeEventId` order; namespace miss≠conflict; symbol-error classification; chip-id-keyed `int` ownership + reuse; TY08 non-M1; no compatibility model | §5 `SymbolRecord`/`ScopeEventRecord`/`TypeRecord`; §7 phase 2b; §8 chip-id allowlist; §12.17; §13 named tests | **conditional** — lifecycle/field encodings, char representation, enum variants, and allowlist seed are `/6` blockers |
| T07 | `ConversionRole` field; **T07-owned committed `SemRecord` carrier for `Return`/`FunctionDefinition` (no T09 `FunctionRecord` cycle)**; one shared `ConversionPlan` with explicit `sext`/`zext`/`trunc`; pinned `(NodeKind, role, op)` VF06 matrix; split `const.literal-decode`/`const.evaluate`; `EffectMask` claims | §5 `ConversionRole`/`SemRecord`/`EffectMask`; §6.1 `links()`; §6.4 `EffectMaskUnsupported`; §9/§13 | **conditional** — exact carrier shape / `FunctionContextId` choice, conversion-op set, and const-request vocabulary are hard `/6` blockers |
| T08 | `ConstOverflow`/`ConstUnsupported` are chip diagnostics (not `CommitError`); `max_const_bits = 128` + signed-range formula/hash/test; canonical signed value; no `/7`; `M1-CL-02`=`CL03`/`M1-NEG-16`=`CL03`; T04 handoff representable (**H1 allocation accepted in principle**; exact shapes pending); itemized `constants.records` owner | §5 `ConstRecord`/`LiteralRecord`; §6.4 classification; §7 (no const commit check); §8 single-writer; §11 F5 | **conditional** — exact codes/enforcement, the **H1 carrier field shapes**, and T04→T08 request shape are `/6` blockers; shared writer not selected |
| T09 | op semantics in `M1AppendSchema`/`NORMATIVE_RULES` (**prospective `/6`**); **marker-based `TerminatorMissing` trigger direction (family/schema unresolved, H11)**; `Constant` immediate = result type; rule id per rejection (**alias pairs unresolved, H8**); `UnsupportedNode` chip diagnostic; committed-vs-terminated cumulative; T08/T09 folded-5 ownership split (`folded_int5_ir_constant`, H7); `scope_events` inventory; VF12 correction | §5 op table/terminators; §6.4 classification; §11 F5/F6; §12.4; §13 named tests | **conditional** — exact marker record shape, op table, and IR fields are `/6` blockers |
| Pipeline (user) | staged pipelines; own-batch + committed cross-tick join; bounded quota `> 1`; quota-1 **canonical projection** baseline (in principle); ordered single commit; canonical stage queues; **single writer per wire**; selection errors not `CommitError`; inert until `/6` | §3.12, §6.2.1/§6.2.2, §6.3, §7, §10.5, §11, §12.18, §13; ADR-0002 | **PROPOSED design / conditional** — pipeline `/6` amendments and T02/T13 batch changes must be frozen before any chip wave; quota `> 1` requires measurement + integrator acceptance |

### 15.4 Remaining blocking questions (not safely defaultable)

- **H1 (T01 + T03/T04/T08 co-freeze; user-accepted in principle):** the
  literal-handoff **allocation** of `node`/`required_kind`/`legality`/`LX08` across
  per-literal `LiteralRecord` / sem-stage `ConstantRequest` / `ConstantResult` is
  **user-accepted in principle (2026-10-04)** as the `/6` working basis (see
  §18.1); the exact field/enum shapes remain T03/T04/T08 co-freeze + T01
  integrator sign-off items and must not be read as frozen (§8, CDR §C2).
- T03/T04: exact PP/T04 field manifests; total `ArtifactKind`/`requires_map`
  second sign-off; `ArtifactRecord.raw_offsets` map shape; `LiteralRecord`/
  `LiteralSuffix`/`LiteralKind` variants.
- T04→T08: the `LiteralId`-backed carrier shapes (**H1 allocation accepted in
  principle; exact shapes pending**;
  single-writer handoff selected; shared writer not selected); still needs
  T04/T08 sign-off.
- T05: exact `ContinuationRecord` field order/wire encoding and the two-tick
  `ContinuationRef`/`AwaitChildren` flow; the `ContinuationRef`/`ChildRef`
  validation error names; `NodeKind` final list; `partial_children`/counter
  exactness. **The T05 `sources.spans` blocker is closed by the token-range
  decision; it is no longer a blocking question.** (`decl` node choice is now
  pinned to the `Identifier` leaf at the T05/T06 boundary; the visibility
  boundary is the complete declarator's completion token (DOC-12).)
- T06: `ScopeEventRecord` lifecycle fields; `ScopeDraft` cardinality/File-Enter
  policy field encodings; `SymbolRecord` enum variants; plain/signed/unsigned
  `char` representation; allowlist rows/seed. (**H3:** the File-scope `Enter`
  ordering — a T06 task after the T05 `TranslationUnit` is committed, pinning a
  committed `NodeId` — and the bootstrap order are a T06/`[INT]` `/6` decision.)
  (The `Identifier`-leaf `decl` identity, the complete-declarator visibility
  boundary, and the Block-node boundary `at` are selected, not open.)
- T07: the exact committed `SemRecord` carrier shape and whether a
  `FunctionContextId` exists; `EffectGraph` deferral; the permitted
  `ConversionOp`/`ConversionRole` set and the exact `(NodeKind, role, op)` VF06
  matrix; the exact constant-request kind names. (The no-T09-`FunctionRecord` rule
  and `Return`/`FunctionDefinition` `SemRecord` presence are selected, not open.)
  `M1-NEG-14` (undeclared identifier, mode-independent) and `M1-NEG-19`
  (implicit-declaration call, dialect-policy) are owned by SE01 (negative,
  catalog-only); `M1-SE-01` by SE02.
- T08/T09: the exact `ConstRecord` diagnostic codes/enforcement; the **marker
  family/schema for the `IR28` function-completion marker** (**H11**: the trigger
  is a selected direction only); the **prospective `/6`** `IrOp` op table and the
  per-rejection rule ids (**H8**: both `ir.op-immediate-type` aliases and both
  `ir.terminator-missing*` candidates are unresolved aliases); Part A folded-`int5`
  IR emission (`folded_int5_ir_constant`, **H7**: T08 computes the `ConstRecord`
  value, T09 emits the IR `Constant`) vs the probe-gated fixture `M1-CL-02` (chip
  `CL03`). (The `max_const_bits = 128` formula and the `Constant` immediate =
  result-type rule are selected as draft, not open.)
- Pipeline (user/ADR-0002): the exact stage set/names, fairness policy, queue
  representation, `SelectionEntry`/`SelectionBatch` final shape, backpressure
  semantics (defer/reject), cancel re-check, metrics register placement, and the
  exact dispatcher/`CommitError`/`ManifestError`/`ConfigError` codes — all frozen
  together at `/6`; quota `> 1` additionally requires measured evidence and
  integrator acceptance. **H6 is an explicit blocker;** the no-`Running`/terminal
  diagnostic mechanism is an **undefined `/6` decision**, not resolved (only its
  recovery direction is accepted in principle). **H9 (rev 33, superseding the
  rev-23 one-phase-owner blocker):** the user accepts **removing
  `max_inflight_total`** in principle (per-tick dispatch is bounded by
  `max_inflight_per_tick`, the stage queues, and `max_tasks_total`; `Waiting` is
  not in-flight; `tasks.in_flight` is an ephemeral per-tick batch), pending T01
  integrator acceptance; the residual-set semantics of the in-flight set and the
  dispatcher `Ready→Running`-vs-ordered-atomic-commit relationship stay **open**.
- Cross-cutting: exact `NORMATIVE_RULES` id set and the
  `CommitError`/chip-diagnostic/`ManifestError`/dispatcher classification are
  frozen at `/6`; no rule name may be claimed hashed before that.
- Any Part A record carrying target width/bit-pattern becomes target-dependent and
  probe-gated (default symbolic, §15.2.10).

Addressed in this draft revision (BL1–BL6, AD1–AD5): `dispatched_tasks` (ordered
in-flight set at quota > 1, singleton `tasks.active` at quota 1) and
empty-vector check (BL1); post-dispatch queue bound with checked arithmetic
(BL2); CT03 invoked exactly once through its adapter / CT06 decision-only
(BL3, AB1a/AB1b); canonical `bus.report` (BL4); exact `new_name_bytes`/
`intern_reserved`/`StoreId::Names` bump rules (BL5); exhaustive final-record
encoding (BL6); `Selection` + relocated `fail_selected` (AD1); new `sources` fields and
shared writer map (AD2); `RecordLink<…>` shorthand clarified (AD3); `Names`
maps to `InternTable`, no `CommittedPatch` (AD4); schema-seed equality test
(AD5).

Addressed in this draft revision (audit 4): per-backing-arena capacity via
`arena_allocated(family)` for the 18 M1 families, never `StoreId` (BLK-1);
tick-start reset of `control.selected`/`tasks.active` and end-of-tick active
clear (BLK-2); CT03 invoked only through its installed
`ProjectedChip`/`ChipAdapter` writing `wires.selection`, no direct `compute`
(BLK-3); canonical bounded `bus.report` with host `Trace` as a derived view and
`Task` progress fields in the snapshot (C1/C4); per-final-record encoders
distinct from `RecordDraft::encode` (C2); unique payload `SourceId`
(`MultipleSourceRefs`) (C3); "any of AB1a/AB1b/AB2" wording (C5); shared
`sources.spans` ownership stated as an acceptance decision (C6; selected in
revision 13); all
changes/tests enumerated in §12/§13 (C7).

Addressed in this draft revision (audit 5): driver-side tick budget/cancel gate
(`pins.tick_budget_reached`/`max_ticks` -> `BudgetExhausted`, `pins.cancel` ->
`Cancelled`); report bound `trace.len() <= checked_add(max_ticks, 1)` with
exactly one terminal record and a terminal halt (A); `wires.selected` set on
dispatch and captured by the snapshot, `None` on no-selection/budget/cancel (B);
conditional source rule for source-scoped batches only, with `SourceNotDeclared`/
`MultipleSourceRefs`/`SpanSourceMismatch` (C); `routing.one-chip-per-layer`
enforced at route registration (D); `arena_allocated` `None` ->
`InternalMissingArena` with checked-add capacity (E).

Addressed in this draft revision (audit 6): report bound is exactly
`checked_add(max_ticks, 1)` with one terminal record and an explicit terminal
halt (`CompilerDriver::step` returns the stored terminal record without ticking
after `budget_exhausted`/`cancel_requested`/terminal `job_state`; `max_ticks == 0`
yields one record) (BLK-6.1); `control.budget_exhausted = true` on both the
`pins.tick_budget_reached` and `control.tick >= max_ticks` branches (BLK-6.2); `TickRecord.selected` derived
from the tick's `wires.selected` after backend execution (BLK-6.3); source-scoped
determination after the batch scan (C1); fixed layer order
`SELECT_LAYER < GUARD_LAYER < WORKER_BASE` with worker-only uniqueness (C2);
residual #19 + tests/inventory (C3).

Addressed in this draft revision (audit 7): step 1 sets the budget/cancel
outcome/flags/`job_state` but **never appends**; step 3 sets the idle/executed
outcome and never appends; step 6 is the sole append site, so `max_ticks == 0`
produces exactly one terminal record and repeated terminal `step` calls append
nothing (BLK-7.1); `checked_add(max_ticks, 1)` is rejected at config validation
when it overflows and the tick advance is checked (BLK-7.2); the terminal
predicate also halts on `job_state ∈ {Finished, Failed}` (BLK-7.3); `Motherboard`
is private to `CompilerDriver` with `step` the only clock entry (BLK-7.4); worker
route layers must be `>= WORKER_BASE` and distinct from
`SELECT_LAYER`/`GUARD_LAYER` (BLK-7.5); the source-scoped check runs after the
batch scan (BLK-7.6); "Addressed"/"proposed default" wording (BLK-7.7).

Addressed in this draft revision (audit 8): `TickOutcome` gains `Finished` and
`JobFailed`, and step 6 selects the outcome from post-commit `job_state`
(budget/cancel preserve `BudgetExhausted`/`Cancelled`; `Finished` -> `Finished`;
`Failed` -> `JobFailed`; else `Idle`/`Executed`), setting `last_outcome` to the
exact appended record (BLK-8.1); `ConfigError::MaxTicksOverflow` +
`CompilerConfig::validate()` wired into construction (the current `validate()`
has no call site) (BLK-8.2); `CompilerBus::advance_tick` non-wrapping with a
terminal structured failure on impossible overflow, noting the root
`Motherboard` owns the call (BLK-8.3) — **superseded by audit 10 / revision 11**:
`tick_overflow` is defensive-only, appends no report, and is observable only via
the flag/snapshot.

Addressed in this draft revision (audit 9): fallible construction
(`CompilerConfig::try_new`/`CompilerBus::try_new` return `Result`, the infallible
`new` demoted to `pub(crate)`, `Default` known-valid) rejects an invalid
`max_ticks` before the job starts (BLK-9.1); `control.tick_overflow` (`bool`,
snapshot-encoded) is set by the non-wrapping `advance_tick` on the impossible
overflow with `job_state = Failed`, observable via the flag/snapshot only (no
terminal report) (BLK-9.2); the terminal tick performs one root
`latch`/`advance_tick` leaving `control.tick == T + 1`; no separate `JobFailed`
reason field (separate outcomes only); `Default` `max_ticks = 1<<20`.

Addressed in this draft revision (audit 10): `tick_overflow` is defensive-only —
because step 6 already appended the tick's record before the root
`advance_tick`, the overflow cannot emit a terminal report; the next `step`
returns the existing `last_outcome` and the host observes the cause via the
`tick_overflow` flag/snapshot (BLK-10.1); `CompilerConfig::new`/`CompilerBus::new`
become `pub(crate)` (not public/deprecated) with `Default` using validated
constants, `CompilerBus::try_new` revalidating, and tests migrated in
`c03_task.rs`/`c05_codec.rs`/`c06_routing.rs`/`c07_limits.rs` (BLK-10.2);
`ConfigError::MaxTicksOverflow` code `config.max_ticks_overflow`.

Addressed in this rev-20 revision (independent audit H1/H2 + findings 3–9):
baseline equivalence restated as the **canonical semantic comparison projection**
(scheduler-only registers excluded) with a separately deterministic scheduler
snapshot (H1); `dispatched_tasks` is the batch/in-flight set, not a stale
singleton (H2); a fully specified T05 own-batch `ContinuationRef` +
`Proposal::AwaitChildren`/`Progress` protocol with `Waiting(WaitSet)` resume and
committed-only chain (finding 3); total 8-variant `ArtifactKind` + total
`requires_map`, `Preprocessed`≠`Normalized` (finding 4); `M1-CL-02` is executed
by chip `CL03` and chip `CL02` is unexercised, with T08 owning `ConstRecord` and
T09 owning IR `Constant` (finding 5); ADR-0002 stale "quota 1 only" language and
VF12 ownership corrected (finding 6); exact `SelectionEntry`/`SelectionBatch`
and `wires.selected` compatibility projection, absent `report.rs` marked
proposed (finding 7); canonical `stage_queues` queue source for `Progress`,
enforced in-flight/proposal bounds (finding 8); and the
`CrossTaskWriteConflict` predicate keyed by `(StoreId, field, record)` with
VF04-extended verifier (finding 9). Owner matrices, residuals, fixture gates, and
revision records are aligned; all owners remain **conditional**.

Proposed defaults (adopt or override): closed `RecordDraft` incl.
Node/Span/Expansion/Name/Literal and full draft table (§6.1); central
pre-mutation validation incl. source/TU ownership; one-transition-per-dispatched-task
(incl. empty proposal vector); `intern_reserved` commit-only idempotent interning;
NEW `sources.spans`/`sources.expansions` fields and names as `StoreId::Names`
mapped to `InternTable`; T04-owned `lex.literals` for the representable
T04→T08 handoff; app-owned `CompilerDriver`/`CompilerBackend`/dispatcher
with no root change; canonical bounded `bus.report` with ordered `dispatched`;
per-backing-arena capacity; CT03 through its adapter; exhaustive
per-final-record encoders for all M1 bodies; bounded `Progress` into canonical
`stage_queues` with checked queue arithmetic; symbolic Part A
types; audit patches only for new arena bodies; `M1AppendSchema` proposed to be
hashed at `/6` with a
seed-equality test; `RecordRef::make`/`family` exhaustive; `SE01` is the
`M1-NEG-14`/`M1-NEG-19` negative-fixture owner only (`M1-SE-01` is owned by
`SE02`); staged pipelines with a
canonical-projection quota-1 baseline and a measured bounded quota `> 1`
(ADR-0002).

---

## 16. Non-claims and revision record

- This is a **proposal**. It is not accepted, not frozen, and changes no existing
  document or code.
- The compiler is not implemented; there is no C parser, no IR, no codegen, no
  probe, no pass rate, and no >99% result.
- Target identity is frozen. Every concrete ABI value remains **UNVERIFIED** and
  is never asserted by Part A. Every proposed new runtime ABI number remains
  **UNVERIFIED**; no value is fabricated here. Referenced interfaces such as
  `intern_reserved`, `RecordDraft`, `NamePlan`, `CompilerBackend`,
  `CompilerBus.report`, `SelectionEntry`/`SelectionBatch`, `StageId`,
  `ContinuationRef`/`ChildRef`/`AwaitChildren`, `LiteralRecord`,
  `ConversionRole`, the pipeline scheduling
  registers/`PipelineMetrics`, `arena_allocated`, the per-final-record
  encoders, and the terminal-halt `CompilerDriver::step` do **not** exist today
  (§1); nor do the proposed `trace.len() <= checked_add(max_ticks, 1)` terminal
  behavior, `TickOutcome::Finished`/`JobFailed`, `ConfigError::MaxTicksOverflow`,
  `CompilerConfig::try_new`/`CompilerBus::try_new`, `control.tick_overflow`, or
  the non-wrapping `CompilerBus::advance_tick`. `report.rs` does not exist today;
  any `Selection`/`TickReport`/`Resolution` relocation into it is proposed.
- No GCC/Clang was used to derive any expected value; no reference output is used
  as an oracle.
- `/5` (`t01-c01-c06/5`) remains the current checked-in version. This document
  does not change `CONTRACT_VERSION`.
- No root `cc-silicon` framework change is proposed; the CPU `Vec` arena
  extension remains application-scoped, and the root crate stays
  `#![forbid(unsafe_code)]` with fixed-layout / no-heap bus data (topology
  construction allocates; the default tick hot path does not;
  [ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) §2).

| Date | Change | Authority |
|---|---|---|
| 2026-10-04 | Initial draft proposal for M1 Part A record materialization; design only, no code, no freeze | M1 preparation session |
| 2026-10-04 | Audit revision: closed B1–B7 and O1–O7; two-phase atomic resolve/apply; Node/Span/Expansion/Name materialization; `RecordRef::Sem`; app-owned driver/dispatcher; bounded `Progress`; symbolic Part A types; `/6` inventory and checklist | M1 preparation session, incorporating independent audit |
| 2026-10-04 | Second-audit revision: `sources.spans`/`sources.expansions` + single `StoreId::Names`; fully enumerated draft wrappers and `links`/`resolve`/`encode`; audit patch only for new arena bodies; one-transition-per-dispatched-task; exact intern preflight/`intern_reserved` invariant; `Progress` fields/queue rules; T02 no-duplicate-select + `report.rs`; hashed `M1AppendSchema`; widened `/6` inventory; illustrative-shape marking; accounting/report fields | M1 preparation session, incorporating second independent audit |
| 2026-10-04 | Third-audit revision: BL1 singleton `dispatched_tasks` + empty-vector check; BL2 post-dispatch checked queue bound (no `selected_removed`); BL3 CT03 invoked once/CT06 decision-only (AB1a/AB1b); BL4 serializable `bus.report` in snapshot; BL5 exact `new_name_bytes`/`intern_reserved`/`Names` bump rules; BL6 exhaustive `encode_record`; AD1 `Selection` + relocated `fail_selected`; AD2 new `sources` fields + shared writer map; AD3 `RecordLink<…>` shorthand; AD4 `Names`→`InternTable`/no `CommittedPatch`; AD5 schema-seed equality test | M1 preparation session, incorporating third independent audit |
| 2026-10-04 | Fourth-audit revision: BLK-1 per-backing-arena capacity via `arena_allocated(family)`; BLK-2 tick-start `selected`/`active` reset + end-of-tick clear; BLK-3 CT03 via installed `ProjectedChip` writing `wires.selection`; C1 canonical bounded `bus.report`/derived `Trace`; C2 per-final-record encoders; C3 unique payload `SourceId`; C4 `Task` progress fields in snapshot; C5 AB wording; C6 `sources.spans` ownership as an acceptance decision; C7 `/6` inventory/checklist/tests | M1 preparation session, incorporating fourth independent audit |
| 2026-10-04 | Fifth-audit revision: (A) driver tick budget/cancel gate + one bounded report entry per tick; (B) `wires.selected` set on dispatch and snapshot-captured; (C) conditional source-scoped rule with `MultipleSourceRefs`; (D) `routing.one-chip-per-layer`; (E) `InternalMissingArena` + checked-add capacity | M1 preparation session, incorporating fifth independent audit |
| 2026-10-04 | Sixth-audit revision: (BLK-6.1) report bound `checked_add(max_ticks, 1)` with exactly one terminal record; (BLK-6.2) `budget_exhausted = true` on both pin and `max_ticks` branches; (BLK-6.3) `CompilerDriver::step` terminal halt returning the stored terminal record; `TickRecord.selected` from `wires.selected`; C1 source-scoped determined after batch scan; C2 `SELECT_LAYER < GUARD_LAYER < WORKER_BASE`; C3 residual #19 + tests/inventory | M1 preparation session, incorporating sixth independent audit |
| 2026-10-04 | Seventh-audit revision: (BLK-7.1) single report append site (step 1/3 never append; step 6 only) with `max_ticks == 0` one-record regression; (BLK-7.2) `checked_add(max_ticks, 1)` config rejection + checked tick advance; (BLK-7.3) terminal `job_state` in the halt guard; (BLK-7.4) `Motherboard` private / `step`-only entry; (BLK-7.5) worker layer range `>= WORKER_BASE`; (BLK-7.6) source-scoped after batch scan; (BLK-7.7) "Addressed"/"proposed default" wording | M1 preparation session, incorporating seventh independent audit |
| 2026-10-04 | Eighth-audit revision: (BLK-8.1) `TickOutcome::Finished`/`JobFailed` selected from post-commit `job_state`, `last_outcome` assignment; (BLK-8.2) `ConfigError::MaxTicksOverflow` + `CompilerConfig::validate()` wired into construction; (BLK-8.3) non-wrapping app `CompilerBus::advance_tick` (root owns the call) | M1 preparation session, incorporating eighth independent audit |
| 2026-10-04 | Ninth-audit revision: (BLK-9.1) `CompilerConfig::try_new`/`CompilerBus::try_new` fallible constructors; (BLK-9.2) `control.tick_overflow` flag (later corrected to defensive-only in the tenth-audit revision), terminal tick ends at `T+1`; removed `JobFailed` reason wording | M1 preparation session, incorporating ninth independent audit |
| 2026-10-04 | Tenth-audit revision: `tick_overflow` defensive-only (no terminal report; guard returns `last_outcome`; flag/snapshot observe); `CompilerConfig::new`/`CompilerBus::new` become `pub(crate)` with `try_new` revalidating; `ConfigError::MaxTicksOverflow` code `config.max_ticks_overflow`; exact test call-site migration | M1 preparation session, incorporating tenth independent audit |
| 2026-10-04 | Eleventh-audit revision: §4 fixture config uses `CompilerConfig::try_new`; schema-seeding prose names the public `CompilerBus::try_new` runtime path (in-crate `pub(crate) new`); audit-8 "terminal structured failure on impossible overflow" marked **superseded by audit 10 / revision 11** (defensive-only `tick_overflow`) while preserving the historical statement | M1 preparation session, incorporating eleventh independent audit |
| 2026-10-04 | Selected integration decisions: AB1a/AB1b/AB2 **selected as a draft direction (user/integrator; not an accepted contract, not owner-approved)** and recorded in T02 (CT03 decision-only via adapter + dispatcher applies selection; CT05 validation-only; CT06 proposes commit, backend mutates; `TickReport`/`fail_selected` relocation, canonical `bus.report`); C6 `sources.spans` field-scoped shared T03/T04 writers (later withdrawn in rev 21). Proposal remains DRAFT; remaining technical/schema residuals must close before `/6`. No code implemented | User, M1 integration decision session |
| 2026-10-04 | Integration-default promotion: integrator-selected draft defaults (§15.2) and `FROZEN-PROPOSAL` owner sign-off table (§15.3, T03/T05/T06/T07/T09). T01 §4 terminology correction added to `/6` inventory only. Still DRAFT/not freeze-ready; no code | User, continue; M1 integration session |
| 2026-10-04 | Reviewer integration: T03/T05/T06/T07/T09 reports folded into §5/§6.1/§8 (inline PpToken spelling + closed kind; Node parent/ordinal + `NodeKind`; `ScopeEvent` lifecycle; `SymbolRecord`/namespaces; no `canonical_key` + `Bool`; `SemRecord` conversions/`ValueCategory`/`EffectMask`; `InstructionRecord.immediate`; derived `BlockRecord`; `ValueRecord` without `def`; artifact map; parse frame); §15.3 conditional sign-off matrix; §12 additions (Sem/ScopeEvent/ParseFrame ids, tags 24–26, arenas, snapshot, hash). Conditional, second sign-off required; still DRAFT | User, continue; T03/T05/T06/T07/T09 review integration |
| 2026-10-04 | Second-round integration (rev 16): Artifact/Continuation/ScopeEvent first-class in `RecordDraft`/`ResolvedDraft`/arena map; `SourceMapEntry` boundary map; one-hop source-scope algorithm; lookup-first idempotent intern; Span offsets `u64`; T05 `ParseContinuation`/`NodeKind`; T06 namespaces/enums; T07 `ConversionPlanDraft`/Sem policy; T08 `ConstRecord`/CL gates; T09 op table; C6 T03/T04 selected with T05 pending; §15.3 T08 row; acceptance IR-2/op-table, SE01 mapping, CL-02/CL-03 gates. All owners conditional; not freeze-ready; no code | User, continue; second-round review integration |
| 2026-10-04 | Rev 17 omission fixes: `NodeKind`/`NodeRecord.token`; `ScopeEventRecord` no ordinal + lifecycle rule; `SymbolRecord.namespace` + `Linkage`/`StorageDuration`/`CharKind`/tag kinds; Sem Return/Function policy + one-per-NodeId; IR op table + Block/Value/terminator invariants; `SpanDraft` `u64`; Phase-4 push arms + pre-mutation validations; direct committed-span dereference in phase 2c; Continuation (reuse `ContinuationId`) replaces `ParseFrameId`; artifact `raw_offsets`; §12 hash/variants. Conditional; not freeze-ready; no code | User, continue; second-round omission fixes |
| 2026-10-04 | Rev 18 owner-review integration: Artifact map phase 2c/3 + `ArtifactMapInvalid`; continuation unified into `ContinuationRecord` + two-tick handoff + parse errors; `ScopeLifecycleViolation` cumulative; namespace derived from kind; `IntRank` no Char; single-`int` producer; `DuplicateSemRecord`/deterministic lookup; `M1-SE-01`=SE02, `M1-SE-02`=SE07/SE02 and SE01 removed from M1; T04/T08 shared `constants.records`; IR block/op/terminator structured errors; store-owner allowlist; §13 tests; acceptance IR/folded-5/F5. Conditional; not freeze-ready; no code | User, continue; rev17 review integration |
| 2026-10-04 | Rev 19 integration: user binding direction making cross-tick work an explicit staged pipeline with committed-ID cursors/joins and a bounded in-flight quota (`quota = 1` baseline equivalence only; measured `> 1` optimization profile), consistent across §3.12/§6.2.1/§6.2.2/§7/§10.5/§11/§12.18/§13 and ADR-0002 (PROPOSED); T03 map-mandatory/optional + `ArtifactSourceMismatch`/`ArtifactSourceMissing` + generalized `source_scoped_one_hop`; T05 continued-record field mapping + committed-ID next-tick stage/join + parse error sites + child scan + hard span-authority blocker + PA20; T06 point-of-declaration + cumulative ScopeEvent in the algorithm + closed namespace + single-producer int + mandatory allowlist + char fixture; T07 SE01/NEG-14 + FunctionContext + `links()` recursion + conversion/EffectMask semantics; T08 chip-level checked arithmetic + const bounds/errors + Part A/probe CL02 split + constants handoff; T09 hashed op semantics + committed/terminated + `UnsupportedNode` + folded-`int5`; corrected family/tag/rule inventories and §15 statuses. All owners still **conditional**; DRAFT, not freeze-ready; no code, commit, or push | User, rev18 owner-review integration + pipeline direction |
| 2026-10-04 | Rev 20 re-review integration (DRAFT, no code/commit/push): baseline equivalence restated as the canonical semantic comparison projection + separately deterministic scheduler snapshot (audit H1); `dispatched_tasks` batch definition everywhere (H2); fully specified T05 `ContinuationRef`/`AwaitChildren`/`Progress` handoff and `Waiting(WaitSet)` resume (finding 3); total `ArtifactKind`/`requires_map` with `Preprocessed`≠`Normalized` (finding 4); `M1-CL-02`=`CL03`/chip `CL02` unexercised + T08/T09 ownership split (finding 5); ADR-0002/VF12 corrections (finding 6); exact `SelectionEntry`/`SelectionBatch` + `wires.selected` projection + absent `report.rs` marked proposed (finding 7); canonical `stage_queues` `Progress` source + enforced bounds (finding 8); `CrossTaskWriteConflict` predicate + VF04-extended verifier (finding 9); new committed `LiteralId` family for the representable T04→T08 handoff; `ConversionRole`; chip-diagnostic vs `CommitError` vs `ManifestError` classification; §13 tests, §15 matrices/residuals, and revision records aligned. All owners still **conditional**; DRAFT, not freeze-ready | User rev19 re-review integration |
| 2026-10-04 | **Rev 21 (DRAFT, no code/commit/push):** user in-principle decision (2026-10-04, direction only) + rev-20 reviewer/audit corrections. **Single owner:** `sources.spans` = T03-only; T04 `TokenRecord.span` reuses committed T03 PP span (no span write); `NodeRecord.first_token`/`last_token` (T05 no span write); C6 withdrawn; §6.1/§7/§8/§13 aligned. **Literal handoff:** committed T04 `LiteralRecord` preserves `node`/`required_kind`/`legality`/`LX08` type; T08 sole `constants.records`; §F corrected. **T05:** `awaited` removed (`WaitSet`-only); `ContinuationRef`/`ChildRef` validated pre-apply; `task.continuation` direction; join reinserted in commit apply; `Progress` stage-queue target; atomic-failure capacity semantics. **T06:** `Identifier`-leaf `decl`; Block-node boundary `at`; symbol-error classification; ChipId allowlist; TY08 non-M1. **T07:** no T09-`FunctionRecord` cycle; `SemRecord` for `Return`/`FunctionDefinition`; `ConversionOp` signedness ops; `(NodeKind, role, op)` VF06 matrix; split `const.literal-decode`/`const.evaluate`. **T08:** `max_const_bits = 128` signed-range formula/enforcement/hash/test; `M1-NEG-16`=`CL03`; symbolic Part A vs probe. **T09:** explicit `IR28` `TerminatorMissing` trigger; `Constant` immediate = result type; rule id per rejection; `scope_events` in the bus inventory. **Pipeline:** single writer per wire; canonical report; selection errors reclassified; `quota > 1` measurement + integrator acceptance. **T03:** M1-produced set `Normalized`/`Spliced`/`CommentFree` (`Preprocessed` not produced). All owners remain **conditional**; T01 integrator acceptance pending; `/5` current; DRAFT | User in-principle decision + M1 rev-21 review-integration subagent |
| 2026-10-04 | **Rev 22 (DRAFT, no code/commit/push; not a freeze):** integrated the accumulated rev-21 read-only review findings (T03/T04, T05, T06, T07, T08, T09, and the T02/T13 pipeline + ADR-0002 review) as documentation corrections, explicit blockers, and pending owner amendment requests; full ledger in §17. Key doc corrections: lex `LiteralRecord` is a candidate fact with **no `node`/`required_kind`** (sem-stage per-use request; `legality` is a result); `source_scoped_one_hop` explicitly limited to the **single-source** M1 fixture with a typed multi-source map blocker; non-M1 `ArtifactKind`s have **no M1 writer**; LX14/LX16 provenance and T04 PP-span provenance are blockers; `SpanRecord` `u32→u64` delta recorded; `ContinuationRefInvalid` and `DuplicateConversionRole` added to `CommitError`; `RedeclarationConflict` removed from the `CommitError` block (chip diagnostic); `awaited`-removal surfaced as an explicit T01 §4 supersession request; resume stage corrected to `stage_of(parent.kind)`; join reinserts counted in preflight; committed-only node token links; terminal diagnostic-capacity/no-`Running` reconciliation; `max_const_bits` carrier/`<=128`/restricted-config blockers; `CompletedFunction` marker is an explicit blocker; IR rejection→rule-id list reconciled (missing comma fixed); folded-`int5` wording corrected; in-flight bound de-duplicated (dispatcher single-phase); own-batch draft-key vs guardrail reconciliation request; "accepted" wording softened to selected/direction. All owners/integrator **pending**; `/5` current; ADR-0002 PROPOSED; no `/6`, no chip, no sign-off | M1 rev-22 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 23 (DRAFT, no code/commit/push; not a freeze):** integrated the independent rev-22 audit H2–H11; full ledger in the new `§18`. **H1 (critical, carried forward):** reframed the rev-22 literal-handoff split as a **proposed allocation requiring explicit user acknowledgement** + T03/T04/T08/T01 co-freeze (the preservation requirement and committed-`LiteralRecord` direction remain user-accepted); removed the claim that the split is accepted/"corrected"; the CDR carries the pending user decision. **H2** resume stage `stage_of(parent.kind)` + absent-`continuation` handling; **H3** File-scope `Enter` is a T06 task after the T05 `TranslationUnit` is committed, pinning a committed `NodeId` (bootstrap order a T06/`[INT]` decision, not job-bootstrap); **H4** all-`Completed`→`Ready`, any-`Failed`→`Failed` once, never `Ready`, sibling-cancellation policy stays a blocker; **H5** same-batch `TokenDraft` node-link branch removed (committed `first_token`/`last_token` only); **H6** no-`Running`/terminal diagnostic mechanism marked **BLOCKED** (invariant stated, not resolved); **H7** `folded_int5_part_a_producer`→`folded_int5_ir_constant` (T08 computes the `ConstRecord` value, T09 emits the IR `Constant`); **H8** all IR rule/op/field terms made **prospective `/6`**, both `ir.op-immediate-type` aliases and both `ir.terminator-missing*` candidates inventoried unresolved, dangling `§3.6`→`§3 item 6`; **H9** `max_inflight_total` phase/owner, residual-set semantics, and dispatcher `Ready→Running`-vs-ordered-atomic-commit marked **BLOCKED**; **H10** stale rev-21 references corrected (CDR §11/§12/§13, ADR-0002, this proposal) with T13 recorded as **not edited**, and `AwaitChildren` added to §2.4 and the §3 one-transition list; **H11** "trigger fixed"→"marker-based trigger direction selected; marker family/schema unresolved" (CDR row H/§H1, M1 target §3.2, §5). All owners/integrator **pending**; `/5` current; ADR-0002 PROPOSED; no `/6`, no chip, no sign-off; H1 needs the user **(superseded by rev 24: user accepted the H1 allocation in principle)** | M1 rev-23 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 24 (DRAFT, no code/commit/push; not a freeze):** records the user's explicit acceptance in principle (`接受拆分（推荐）`, 2026-10-04) of the recommended **H1 literal-handoff allocation split** as the `/6` revision working basis: committed T04-owned `LiteralRecord` = per-literal lexical facts + `LX08` candidate type; post-parse sem-stage `ConstantRequest` = per-use `node`/`required_kind`; `ConstantResult` = `legality`; typed handoff with T08 the sole `constants.records` writer. **Not a freeze and not code/chip authorization**; **T01 integrator acceptance and T03/T04/T08 owner co-freeze/sign-off remain pending**, as do the exact variants/schema (owner/integrator). Updated the intro, §5 `LiteralRecord`, §8 handoff, §13 test, §15.2.19/§15.3/§15.4, §17.1/§17.4/§17.5/§17.8, and §18/§18.1 so no stale "pending user decision" for this split remains; all other blockers (H6/H8/H9, D–I, T13) unchanged. **Correction (rev 25 F3):** rev 24 recorded the **allocation**, not concrete carrier shapes — the distinct proposed `ConstantRequest`/`ConstantResult` carriers and the `LiteralRecord.candidate_type` field were **drafted in rev 25**, not previously edited. `/5` current; ADR-0002 PROPOSED; M1 DRAFT | User in-principle decision (2026-10-04); M1 rev-24 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 25 (DRAFT, no code/commit/push; not a freeze):** integrated the **rev-24 independent audit F1–F9** plus the **verified `/5` `routing.rs` `fail_selected` single-task failure guarantee**; point-by-point ledger in the new `§19`. **F1** — present-tense "pinned/hashed" claims in §5 `ConstRecord`/`requires_map` and §12.17 are marked **proposed to be hashed at `/6`** (not hashed today), and §12.17 is added to ledger coverage; **F2** — §7/§10.2/§10.3/§10.5 preserve the verified `/5` single-task failure semantics, state that clearing in-flight does **not** clear `TaskState`/`Running`, and mark the batch fan-out **BLOCKED** with `/6` alternatives (reserve bounded terminal diagnostic slots before dispatch, or another T01-approved atomic terminal path); **F3** — `LiteralRecord` gains a clearly proposed/unfrozen `candidate_type`, distinct proposed `ConstantRequest { literal, node, required_kind }` / `ConstantResult { value, legality }` carriers are drafted, and the rev-24 history is corrected to "drafted in rev 25"; **F4** — own-batch `DraftRef`/guardrail interpretations are candidate-only and explicitly linked to the open authority reconciliation (no "never another task" statement resolves Guardrails §6.1); **F5** — the CDR no longer reads the terminator trigger as explicit; **F6** — all rev 21–23 pointers are rev 21–24; **F7** — the `node_token_range_same_source` test inventory is reconciled with the retired same-batch test; **F8** — CDR row H/§3.2 say proposed `/6` hash (not hashed); **F9** — T02 "selected contract decisions" and the historical AB1a/AB1b/AB2 "accepted" wording are selected draft direction only, not accepted. M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; H6 BLOCKED; H1 exact shapes pending; T03–T09/T13 not edited | M1 rev-25 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 26 (DRAFT, no code/commit/push; not a freeze):** integrated the **rev-25 independent audit findings 1–8** plus the nested CDR §C/§D/§E request items (`C1`/`C2`/`C3`, `D1`/`D2`/`D3`, `E`); point-by-point ledger in the new `§20`. **Finding 1** — the frozen `/5` `fail_selected` single-task sentinel semantics are documented exactly and the batch (`quota > 1`) fan-out stays **BLOCKED** (H6); **Finding 2** — `ConstantRequest` references the committed `LiteralRecord`; **Finding 3** — `LiteralDraft`/`LiteralRecord.candidate_type` added; **Finding 4** — the node-token-range test cross-reference is §17.2 (T05) and the retired same-batch test name is distinguished; **Finding 5** — remaining present-tense hash/pin claims are prospective `/6` only; **Finding 6** — current-state pointers are rev 21–26; **Finding 7** — the §C closure criterion requires (pending) owner acceptance of the exact shapes; **Finding 8** — the no-valid-transition statement is qualified to `quota = 1` with `quota > 1` **BLOCKED** (H6); nested `D1`/`D2` mark the own-batch direction/validation as **candidate-only** linked to the open Guardrails §6.1 reconciliation, and CDR items 9–12 are reconciled in §20.9. No acceptance, no `/6`, no code/chip; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending; T03–T09/T13 not edited | M1 rev-26 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 27 (DRAFT, no code/commit/push; not a freeze):** applied the **rev-26 audit corrections** (doc-only; no acceptance-status change). **(1)** CDR §12 rev-26 history item (6) corrected to `rev 25→26 and rev 21–25→rev 21–26`; CDR operative pointers verified at rev 21–26. **(2)** CDR §13 now names which documents were updated through which revision (M1 proposal/CDR/ADR-0002/M1 target acceptance through rev 26; T02 and the M1 frontend acceptance through rev 25). **(3)** CDR §2 dashboard note renamed `Rev 23–26` and extended with §19/§20. **(4)** this proposal's §20.9 D1 cross-reference corrected from the wrong `Finding 2` to the T05 continuation/own-batch items (§17.2/§18.3/§19.4, CDR §D1). **(5)** §20.10 residual register now lists the **H11** `CompletedFunction` marker-family blocker. **(6)** residual unqualified `frozen` wording at the stage ordinals (§6.2.1), the versioned fairness contract input (§10.5), and the `NamePlan`/`ResolveTable`/`RecordDraft` interface (§15.2 item 12) is qualified as proposed/selected draft, not a freeze. No acceptance, no `/6`, no code/chip; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending; T03–T09/T13 not edited | M1 rev-27 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 28 (DRAFT, no code/commit/push; not a freeze; docs-only pointer reconciliation).** Corrected the CDR §12 rev-26 history item (6) before→after range to `rev 21–24→rev 21–26` (the rev-27 row had recorded `rev 21–25→rev 21–26`; the rev-25 F6 `rev 21–23→rev 21–24` is the correct predecessor) and advanced every current-state M1 pointer/range to **rev 21–28**: this proposal's header/§18.7 H10/§19.6 F6/§20.6 Finding 6, the CDR header/§2/§11/§13, and ADR-0002 §1.1 (Revision 9). No schema, interface, field, enum, rule, task-kind, chip, or `/6` change; the correction changed the CDR §12 rev-26 history row, the historical rev-27 row is preserved verbatim, and other historical rows are unchanged. M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending; T03–T09/T13 not edited | M1 rev-28 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 29 (DRAFT, no code/commit/push; not a freeze; docs-only historical-documentation correction).** *(Historical row supplied in rev 31; this proposal's rev-29 edits were operative and described in the header and in the CDR §12 rev-29 row, but the §16 row was omitted at the time — see the rev-31 row.)* Corrected the rev-28 rows' "historical revision rows are unchanged" claim: rev 28 did correct the CDR §12 rev-26 history row; the historical rev-27 row is preserved verbatim, and other historical rows are unchanged (the CDR §12 rev-28 row and this §16 rev-28 row were corrected in place). Recorded the ADR-0002 Revision 6 (rev-23) H10 chronology erratum: rev 23 corrected the pointer to **rev 21–23**, with rev 24/25 extending it to **rev 21–24** (ADR-0002 Revision 10; the Revision 6 text is preserved). Advanced every current-state M1 pointer/range from **rev 21–28** to **rev 21–29** (this proposal header/§16/§18.7/§19.6/§20.6, the CDR header/§2/§11/§13, and ADR-0002 §1.1). No schema, interface, field, enum, rule, task-kind, chip, or `/6` change. M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending; T03–T09/T13 not edited | M1 rev-29 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 30 (DRAFT, no code/commit/push; not a freeze; records two explicit user in-principle decisions).** **(A, Guardrails §6.1 narrow interpretation)** the user accepts in principle that a same-task `OwnBatch(DraftRef)` may exist **only as a transient wire/proposal input before commit**, with commit validating/resolving it to a committed `TaskId`/record **before any persistent state**, so no durable cursor/`WaitSet`/join points at a draft/wire/address; the guardrail text is **unamended** and **T01 integrator implementation-confirmation is pending**. **(B, awaited children)** the user accepts in principle that `TaskState::Waiting(WaitSet)` is the **sole** awaited-child-ID source and `ContinuationRecord` carries **no duplicate `awaited`**; the request to supersede/clarify **T01 §4 in `/6`** is accepted in principle with **no `/5` edit**, and **T01 integrator + T05 owner acceptance remain pending**. Updates this proposal's header/§3 item 12/§5 continuation comment/§6.3 `ContinuationRef`/§8/§10.5/§15.2 item 16/§17.2 T05-2/§17.7 PIPE-5/§18.6/§18.9/§19.4/§19.10/§20 intro/§20.9/§20.10. The CDR records the same decisions as **rev 31**; ADR-0002 as **Revision 11**; T02 as **rev 26**. Batch **H6/H9**, **CT07**, **H8/H11**, all other rows, and every exact-shape blocker unchanged; row D remains **PENDING overall**. M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; H1 accepted in principle only; all T01/owner sign-offs pending; T03–T09/T13 not edited. | User in-principle decisions (2026-10-04); M1 rev-30 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 31 (DRAFT, no code/commit/push; not a freeze; docs-only history-gap repair).** Repairs the §16 revision-record gap identified by the rev-30 audit (**F1**): §16 had jumped from **Rev 28** to **Rev 30** while the header, §18.7, §19.6, §20.6, and the CDR §12 rev-29 row all stated that **rev 29** was a docs-only historical-documentation correction that updated this §16. This revision **adds the missing historical Rev 29 row** (above, describing exactly the rev-29 edits already recorded in the header/CDR, no new content) and **adds this Rev 31 row**, so the revision record no longer skips a revision. It also advances every current-state M1 pointer/range from **rev 21–30** (proposal pointer rev 30) to the current **rev 21–31** (this proposal header/§18.7/§19.6/§20.6, the CDR header/§2/§11/§13, and ADR-0002 §1.1 — Revision 12), and adds the CDR **Rev 32** row for the same repair. No schema, interface, field, enum, rule, task-kind, chip, or `/6` change; the historical **Rev 28** and **Rev 30** rows above are preserved verbatim; no acceptance status changes. M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; H6/H9 BLOCKED; H1 accepted in principle only; all T01/owner sign-offs pending; T03–T09/T13 not edited | M1 rev-31 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 32 (DRAFT, no code/commit/push; not a freeze; records a new explicit user in-principle decision, direction only).** Records the user's 2026-10-04 **in-principle acceptance of the H6 batch-failure recovery direction**: after an atomic semantic batch commit fails, transition **every** dispatched task, in **dispatch order**, **exactly once** to `TaskState::Failed` — a committed `DiagnosticId` when diagnostic/record capacity allows, else the `TaskState::Failed(DiagnosticId::NONE)` sentinel — so **every** dispatched task leaves `Running`; **clearing the in-flight set is not itself a transition**. This **generalizes the verified `/5` `fail_selected` single-task sentinel obligation**, which is preserved and **not weakened**. This is a `/6` **working-basis direction only**: the **exact atomic/bounded implementation requires T01 integrator approval**, and the batch failure-atomicity details, the diagnostic budget/state mechanism, and the **T02/T13 owner fixtures/sign-offs remain co-freeze and pending**; the mechanism is **not** implemented or frozen and no no-`Running` batch guarantee is claimed. Edits: header; §3 item 5/§3 item 12; §7 one-transition property; §10.5 step 5; §16; §17.7 PIPE-4; §18.4 H6; §19.2 F2; §19.6/§19.10; §20.1/§20.6/§20.8/§20.10. H6 therefore moves from "no selected alternative" to "direction accepted in principle, implementation **BLOCKED** pending T01". No other acceptance status changes; **H9 stays BLOCKED**; H1 accepted in principle only; D–I remain pending; row D remains PENDING overall. Advances every current-state M1 pointer/range from **rev 21–31** to **rev 21–32** (this proposal header/§18.7 H10/§20.6 Finding 6 — **the rev-32 row originally named §18.4/§19.2/§19.6/§19.10/§20.1/§20.8/§20.10, which was inaccurate; corrected in rev 33, which also advances the stale §19.6 F6 pointer to `rev 21–32`**, the CDR header/§2/§11/§13 via CDR rev 34, and ADR-0002 §1.1 — Revision 13). M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; all T01/owner sign-offs pending; T03–T09/T13 not edited | User in-principle decision (2026-10-04); M1 rev-32 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 33 (DRAFT, no code/commit/push; not a freeze; records a new explicit user in-principle decision, direction only, and integrates the H6 rev-32 audit defects D1–D3).** Records the user's 2026-10-04 **in-principle acceptance of removing `max_inflight_total` from the `/6` candidate (H9)**: sequential per-tick dispatch is already bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; `Waiting` tasks are not in-flight; a future cross-tick `Running` mode would need a separate CDR. This is a `/6` **working-basis direction only, pending T01 integrator acceptance**; `/5` is unchanged and no bound/config/hash is frozen. The dispatch batch bound remains the per-tick `max_inflight_per_tick` quota; `tasks.in_flight` remains an ephemeral per-tick scheduler batch only (candidate) that clears at latch only after every dispatched task has a terminal/`Waiting`/`Progress` outcome (or H6 recovery); the dispatcher's `Ready→Running` pre-worker mutation is distinguished from the ordered atomic semantic commit; H6 recovery semantics are retained. **D1 (pointer-list repair):** the §16 rev-32 and CDR rev-34 history lists named `§18.4/§19.2/§19.10/§20.1/§20.8/§20.10`; the actual rev-32 locations were this header, §18.7 H10, and §20.6 Finding 6, and the §19.6 F6 pointer was still `rev 21–31` (now advanced to `rev 21–32`; this rev advances everything to `rev 21–33`). **D2 (M1 §20 status-key repair):** H1 is `ACCEPTED-IN-PRINCIPLE` at **Rev 24**, and A/B at M1 proposal **Rev 30** / CDR **Rev 31** — the §20 status key misattributed H1 to rev-30 A/B; the acceptance dates are unchanged. **D3 (CDR §13 repair):** the CDR §13 claim that the M1 frontend acceptance carries rev-26 H6/CT06 text was wrong — the frontend is at **Rev 25** with no H6/CT06 content; T02 carries the operative H6 text (Rev 27 prior state, updated to Rev 28 by the H9 integration). Edits: header; §3 item 5/§3 item 12; §6.2.1 (H9 removal + `tasks.in_flight` ephemerality + dispatch-vs-commit distinction); §7 phase-3; §10.5 step 5; §12 item 8/§12.10 limits/hash inventory; §16; §17.7 PIPE-1; §18.6 H9; §18.9; §19.6 F6; §19.10; §20 status key; §20.1/§20.6; §20.10; §21 (new D1–D3 correction note). Advances every current-state M1 pointer/range from **rev 21–32** to **rev 21–33** (this header/§18.6/§18.7/§19.6/§19.10/§20.1/§20.6/§20.10/§21, the CDR header/§2/§11/§13 via CDR rev 35, ADR-0002 §1.1 via Revision 14, and T02 Rev 28). H9 moves from *BLOCKED (phase/owner undefined)* to *removal direction accepted in principle, pending T01 integrator acceptance*; the remaining H9 `[INT]` items (the dispatcher-vs-commit `Ready→Running` relationship, residual-set semantics) stay open. No other acceptance status changes; H1 accepted in principle only; D–I remain pending; row D remains PENDING overall. M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; all T01/owner sign-offs pending; T03–T09/T13 not edited. **Correction (rev 34):** the companion CDR/ADR/T02 edits named here were **forward-referenced but not yet applied** at rev 33 (the proposal text was ahead of the CDR/ADR/T02 render); they are applied in **rev 34** (CDR rev 35/36, ADR Revision 14, T02 Rev 28), and the proposal pointer advances to **rev 21–34**. | User in-principle decision (2026-10-04); M1 rev-33 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 34 (DRAFT, no code/commit/push; not a freeze; docs-only consistency completion; no new decision).** Completes the H9 integration across [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)/[ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md)/[T02](T02_CONTROL_CHIPS.md) and applies the **D3** correction there, since rev 33 had applied H9/D1–D3 **only in this proposal** while its `§16` rev-33 row forward-referenced the not-yet-applied companion edits. **Records (no new decision, direction only, pending T01 integrator acceptance):** the H9 removal direction is now written into the **CDR** (CDR rev 35: header/§2 dashboard row A + note, §A5/A status honesty, §9 H9, §10 narrative/row A/authority, §11/§13; and the T02 owner-amendment row), the **ADR** (Revision 14: §1.1 pointer + §6 in-flight-bound/§5/§6 operative text), and **T02** (Rev 28: the in-flight-bound bullet, the CT06 row, and the pipeline-corrections list). **D3:** the CDR §13 frontend claim is corrected (**CDR rev 36**) — the M1 frontend acceptance is at **Rev 25** and carries **no** H6/CT06 text; the operative H6 batch-failure text is carried by **T02**. D1/D2 are the rev-33 proposal-internal repairs and remain operative. Every current-state M1 pointer/range advances from **rev 21–33** to the current **rev 21–34** (this header/§16/§18.6/§18.7/§19.6/§19.10/§20.1/§20.6/§20.10/§21, the CDR header/§2/§11/§13, ADR-0002 §1.1, and T02 Rev 28). No schema, interface, field, enum, rule, task-kind, chip, or `/6` change; H9 stays a working-basis direction pending T01 integrator acceptance and its remaining `[INT]` items stay open; H1 accepted in principle only; D–I remain pending; row D remains PENDING overall. M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; all T01/owner sign-offs pending; T03–T09/T13 not edited | M1 rev-34 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 35 (DRAFT, no code/commit/push; not a freeze; docs-only consistency cleanup of stale active candidate prose; no new decision).** Fixes the remaining contradictions where the **current operative** text still presented the rev-22/23 candidate as active despite the rev-33 H9 removal direction. **§4 integrator summary (T02/T13/ADR-0002):** the entry stated the one-phase bound and the "single phase/owner for `max_inflight_total` … BLOCKED" as the *current* summary; it now states the **removal direction** as current, marks the rev-21/22 one-phase candidate `max_inflight_total`/`InflightQuotaExceeded` as **historical superseded**, distinguishes the still-open H9 `[INT]` items, and keeps H6 accurate (**direction accepted in principle, implementation BLOCKED**). **§4 H9 summary:** now states the removal direction as current (pending T01 integrator acceptance; residual-set and dispatcher-mutation-vs-commit items open) with the rev-23 "undefined phase/owner" framing retained as **superseded history**. **§4 H6 summary:** records the direction accepted in principle (rev 32) while the implementation stays **BLOCKED**. **Companion cleanup:** [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md) rev 37 removes `InflightQuotaExceeded` from the §A9/§I active dispatcher-error candidate lists; [ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) Revision 15 rewrites the §6 in-flight-bound paragraph so the removal direction is the opening operative statement and the rev-22/23 framing is marked superseded; [T02](T02_CONTROL_CHIPS.md) rev 29 does the same for the in-flight-bound bullet. No acceptance status changes: H9 removal direction remains in principle only (**pending T01 integrator acceptance**, no bound/config/hash frozen), the residual H9 `[INT]` items stay open, H6 stays **implementation BLOCKED**, A/B/C and H1 remain in principle only, and D–I remain pending (row D PENDING overall). Every current-state M1 pointer/range advances from **rev 21–34** to the current **rev 21–35** (this header/§16/§18.6/§18.7/§19.6/§19.10/§20.1/§20.6/§20.10/§21, the CDR header/§2/§11/§13, ADR-0002 §1.1 — Revision 15, and T02 rev 29). Historical rev rows are preserved, not overwritten. M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; all T01/owner sign-offs pending; T03–T09/T13 not edited | M1 rev-35 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 36 (DRAFT, no code/commit/push; not a freeze; docs-only header/audit correction; no new decision).** Adds a concise **current-state pointer paragraph at the very top of the header** so top-down readers see the operative revision (**rev 35**) and the current pointer/range (**rev 21–35**) first, and states explicitly that the per-revision paragraphs below are **historical reverse-chronological snapshots**; an earlier paragraph (e.g. rev 34) is therefore unambiguously **as-of that revision**, not the current pointer. Fixes two typo/corruption defects from the independent audit: §19.3 F3 `candidate-type type` → `candidate-type encoding`, and §21 D1 `In in addition` → `In addition`. No current-state range change: the pointer stays **rev 21–35** (this header/§16/§18.7/§19.6/§20.6/§21, the CDR header/§2/§11/§13, ADR-0002 §1.1 — Revision 16, and T02 rev 29). Companion edits: [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md) rev 38 corrects the §13 lead-in to read the M1 proposal as updated through **rev 36** and fixes the `per-stage stage queues` typo; [ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) Revision 16 clarifies the §3 `bus.rs` amendment row so the preflight bound is the **per-tick `max_inflight_per_tick`/quota only** (no separate total in-flight bound). No acceptance status changes: H6 = direction accepted in principle, **implementation BLOCKED**; H9 = removal direction accepted in principle, **pending T01 integrator acceptance**, residual-set and dispatcher-mutation-vs-commit `[INT]` items open; A/B/C and H1 remain in principle only; D–I remain pending (row D PENDING overall). M1 stays DRAFT; ADR-0002 PROPOSED; `/5` current; all T01/owner sign-offs pending; T03–T09/T13 not edited | M1 rev-36 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 37 (DRAFT, no code/commit/push; not a freeze; docs-only mirror of the explicit CDR rev 42–50 decisions; no new user decision, no acceptance, no `/6` change).** Records in this proposal the authority decisions already recorded in the [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md) rev 42–50 (plus the owner-package syncs), as a proposal-side ledger (`§22`) and header/current-state update, without changing any accepted decision or inventing schema. **(1) User-accepted selected subdecisions rev 42–46 (per-subdecision only; overall bundles A/C/D/E/F/G PENDING):** A/T02/T13/T01 H6 scope+mechanism (retain H6; **freeze a quota>1-capable bounded recovery mechanism in `/6`**, rejecting the §9B row-A deferral; pre-dispatch errors leave tasks `Ready`; a failed semantic batch commit mutates no semantic state; deterministic bounded recovery transitions dispatched tasks once in dispatch order to `Failed`; no pre-reservation of N diagnostics; per-task diagnostic attempt with `DiagnosticId::NONE` fallback; state guard against duplicate); D/T02/T05/T01 join = **commit-apply invariant, no new CT07 committed carrier/family**, **await-all**; D/T05 the `ContinuationRecord` exact ordered fields (**no `awaited`**) and the formal `/6` T01 §4 `WaitSet`-only supersession (**no `/5` edit**); D/T01/T05 the exact OwnBatch pre-apply validation; F/T06/T07/T09/T01 **no `FunctionContextId`**, `SemRecord` = committed `CheckedNode` materialization one per `NodeId` with an explicit committed typed link consumed by T09; F/T07 `ValueCategory {Lvalue=0,NonLvalue=1,FunctionDesignator=2,Void=3}` (M1 `NonLvalue`) and M1-only `EffectMask(0)`; F/T07/T13/T01 VF06 `TypedAstInvariant` **in M1** (exact registration open) and the **M1-minimal conversion scope**; E/T06/T01 deterministic bounded committed-`types.records` lowest-`TypeId` scan (no hidden cache) and the File `Enter` `parse.TranslationUnit → symbol_type.scope-enter` stage edge after the committed TU (no job-bootstrap); C/G **`max_const_bits`** hashed-`Limits` origin (cap 128, config rejects `>128`, task-input projection), `required_kind` = per-use constant-expression requirement, `legality` = result payload field, total eight-`ArtifactKind` map rule (M1 scope only single-source `Normalized`), exact ordered `LiteralRecord` fields + `LiteralKind`/`LiteralSuffix`/radix scope, `RequiredKind` = `IntegerConstantExpression`, `ConstLegality` ∈ {`Legal`,`NotConstantExpression`,`Unsupported`}, mandatory artifact-map invariants. **(2) Rev 47 user-delegated candidate defaults** (optional-`ArtifactKind` map policy; T05 parse-depth; T06 namespace/lookup; T09 shorter rule aliases + immediate-before-result + target type = result `ValueRecord.ty`) recorded as **selected under user-delegated integration default, clearly labeled NOT owner or T01 `[INT]` signoff** (not owner acceptance, not T01 `[INT]`, not a freeze). **(3) Rev 48 code audit discrepancy + unresolved hash conflict as it existed then:** `/5` code facts **24** `RECORD_KINDS` / **24** `RecordRef` variants (tags 0–23), `RecordFamily` **absent**; draft 19/27/27 counts **proposed/unverified**; the `CONTRACT_VERSION`/`contract.rs`/`README.md` vs `COMPILER_SFL_MANIFEST.md` §4 hash-scope conflict **real and unresolved**, three options then unselected/unrecommended. **(4) Rev 49 user H11/T09/T01 direction + H9 read-only audit:** `TerminatorMissing` reuses the committed IR28 `FunctionEnd` terminal result (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) with a **T01-owned typed phase-2b commit-apply validation hook**; **no** new marker family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder/`ResultValue` variant (**not a frozen `/6` hook, no code**); the earlier `CompletedFunction` new-marker direction is **superseded for the operative direction, preserved as history**; H9 residual blockers (quota>1 pipeline unimplemented; `Ready→Running`/`in_flight`-vs-commit relationship, `in_flight` clear ownership/order, all-paths no-`Running` proof open; `max_dispatches_per_tick`-vs-`max_inflight_per_tick` limit conflict deferred to `/6`; fan-out fixtures + T13 VF02/VF03/VF04/VF13 pending). **(5) Rev 50 two-tier hash scope:** the user accepts `StoreSchema::foundation + M1AppendSchema` as the **frozen `/6` seed participating in the `/6` contract hash**, with **post-seed runtime `declare()` extensions hash-excluded** (runtime snapshot/schema capture/validation); at `/6` T01 must **atomically** update the manifest §4 wording, the `hash_excludes` semantic token (**post-seed runtime declarations**), and `FrozenSchema::encode`/contract code plus the freeze test, **preserving the frozen `/5` hash/history** — **no exact `M1AppendSchema` contents/counts accepted, nothing frozen, no `/5` change, no code**; numeric inventory and freeze-test remain pending; T01 still co-freezes the M1 seed values after the owners. Present status reflected: `/5` current, `/6` **unfrozen**, M1 **DRAFT**, **no code**, owners' bundles incomplete + T01 co-freeze pending, **no Part B substrate**; old marker-family direction historical/superseded; `quota > 1` **not** the M1 baseline; Part A symbolic model probe-independent. Every current-state M1 pointer/range advances from **rev 21–35** to the current **rev 21–36** (this header/§16/§18.7 H10/§19.6 F6/§20.6 Finding 6/§21; the CDR is at rev 50, ADR-0002 §1.1 stays **Revision 16**, and T02 is at rev 30). No schema, interface, field, enum, rule, task-kind, chip, or `/6` change; the historical rev 21–36 ledger is preserved verbatim; no acceptance status changes; the CDR, ADR, task packages, and code are not edited | M1 rev-37 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 38 (DRAFT, no code/commit/push; not a freeze; docs-only integration of the current companion state — CDR rev 51, ADR-0002 Revision 17, updated T02–T09/T13, M1 vertical/target acceptance; no new user decision, no acceptance, no `/6` change).** Records, via a new `§23` ledger and this row, the current relevant status and exact latest items without inventing schema or freezing. **(1) CDR rev 51 delegated candidate** (mirrored by ADR-0002 Revision 17 and T02 rev 31): `max_inflight_per_tick`/quota is the **sole per-tick dispatch-count bound** and the redundant `max_dispatches_per_tick` (with its `DispatchBudgetExceeded`) is **dropped from the candidate limit inventory and validation** — a **delegated candidate default, not** an owner/T01 `[INT]` signoff, **not** a schema freeze; the H9 mutation boundary / `in_flight` clear order / residual no-`Running` proof remain **open**, and the H6 mechanism is unchanged. **(2) Delegated ParseContext 10-member candidate** (`TranslationUnit, ExternalDecl, Specifier, Declarator, ParameterList, Block, Expression, Assignment, Unary, Primary`), recorded in the T05 package; `context` is **semantically distinct** from `production: TaskKind`; mapping/discriminants **open**. **(3) T04 `LiteralRecord.token` delegated candidate:** `Some(committed TokenId)` for a single committed source token (M1 `2`/`3`), `None` for an explicitly synthesized no-single-token literal; same-batch relation uses the generic typed `RecordLink`/append-ref (not `ContinuationRef::OwnBatch`/`ChildRef::OwnBatch`); exact mechanism **open**. **(4) T08 opening payload aligned** to `ConstantRequest { literal: RecordRef::Literal, node, required_kind }` → `ConstantResult { value: RecordRef::Const, legality }`, with the sem→T08 const→T09 IR order corrected; exact payload/encoding/tag/typing/codes **open**. **(5) M1 vertical acceptance artifact scope corrected** — the exercised artifact-map path is **only** single-source `Normalized`; the other map-mandatory kinds are schema-declared but **not produced/asserted**; source-provenance/equality **deferred**; an audit-identified mismatch is resolved at the document level, with the `raw_offsets` semantics / token linkage / candidate closed set still **open T03/T04/T01 blockers** (not all resolved). **(6) M1 target acceptance rev 29** aligns the IR28 committed `FunctionEnd` terminal result + T01 phase-2b hook with the rev-50 two-tier hash scope; **Part B remains unavailable**. **(7) T01 readiness audit summarized in `§23`** as an ordered freeze-blocker/dependency checklist (owner-signed schemas/payloads/manifests; families/tags/ordinals/canonical encoder; T05 TU/context; T06 lifecycle/allowlist; T07 `SemRecord`/VF06; T08 const; T09 hook; pipeline H6/H9; hash/snapshot tests). The rev-50 two-tier hash scope and rev-49 H11/T09/T01 direction remain current. Every current-state M1 pointer/range advances from **rev 21–36** to the current **rev 21–37** (this header/§16/§18.7 H10/§19.6 F6/§20.6 Finding 6/§21; the CDR is at rev 51, ADR-0002 §1.1 at **Revision 17 / rev 21–36**, T02 at rev 31, M1 target at rev 29, M1 vertical at rev 27). No schema, interface, field, enum, rule, task-kind, chip, or `/6` change; the historical rev 21–37 ledger is preserved verbatim; no acceptance status changes; the CDR, ADR, task packages, and code are not edited. | M1 rev-38 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 39 (DRAFT, no code/commit/push; not a freeze; docs-only integration of the current companion state — CDR rev 53, ADR-0002 Revision 17, newly updated T02 rev 32 / T03 rev 53–54 / T04 rev 47 / T05–T09 / T13 packages, M1 vertical rev 27 / target rev 29 acceptance; no new user decision, no acceptance, no `/6` change).** Records, via a new `§24` ledger and this row, the newest candidate status and open audit findings without inventing schema or freezing. **(1) CDR rev 53 delegated candidate:** for the exercised M1 literal subset the `Lx08CandidateType` M1 vocabulary is the **closed one-member set `{ Int }`**, M1 literals `2`/`3` = **target-independent `Int` with no bit width**; forms outside the exercised subset must **not silently default to `Int`** and are **explicitly unsupported/deferred**; future categories **append without reinterpretation** — a delegated candidate default, **not** an owner/T04 or T01 `[INT]` signoff, **not** a `/6` freeze, no numeric tags; complete member set/encodings **open** (T04 rev 47; T08 rev-2 cross-reference). **(2) T03 rev 53–54:** the delegated M1 single-source `Normalized` **`raw_offsets` boundary map** (primary location map, not complete provenance; error codes open) and the new **M1 PP01 producer/consumer contract candidate**; both candidates only. **(3) T05/T06 TU carrier + exactly-once still open:** the committed TU `NodeId` is supplied by the existing generic `ResultValue::Record(RecordRef::Node(root_node_id))` of the committed `TaskState::Completed(ResultId)` — no new typed result/family, no task-kind-only derivation — and the **exactly-once delivery claim is not established by `Completed(ResultId)` alone**; the candidate requires result-consumption + enqueue in one commit-visible unit or a **T01-defined equivalent persistent delivery claim**, with consume/enqueue envelope, retry/error recovery, and T06 enqueue-payload task typing **open** T05/T01/T06 co-freeze. **(4) Newest T07/T08/T09/T13 candidate sections:** T07 `SemRecord` field-set checklist + M1-only constant evaluator; T08 H1-aligned payload + M1-only evaluator; T09 M1-only IR-shape checklist + delegated rev-47 rule defaults; T13 M1 fixture/registration checklist + explicit H6/H9 verification matrix (H6-M01..H6-M16, required pending fixtures — **same-rev-39 correction: this row initially read H6-M12; T13 rev 5 already extends the matrix through H6-M16**) — all candidate/prospective `/6`, none frozen. **(5) T02 rev 32 H6/H9 checklist:** a proposed remaining co-freeze contract checklist (H6.1–H6.6, H9.1–H9.4) stating each open point as an exact question + testable invariant, **proposed, not signoff/freeze**, four T01 architecture-critical items left to T01. **(6) Audit findings recorded as open in `§24`:** the T04 **reciprocal token↔literal `RecordLink` cycle** requires **pre-reserved IDs before links resolve** (open T04/T01 `/6` blocker); the T08 **`§G2` signed-range overflow formula** remains an **unselected draft proposal/reference**; the M1 **`PP-08` exact-map/failure semantics** (mandatory-map invariants; missing-source/bad-map rejected pre-mutation; classification/codes open); and the **H9 clearing conflict + T13 no-residual evidence gap** (the `in_flight` clear ownership/order and the all-paths no-`Running`/no-residual proof remain open; the T13 matrix rows are **required pending fixtures, not a proof**). Every current-state M1 pointer/range advances from **rev 21–37** to the current **rev 21–38** (this header/§16/§18.7 H10/§19.6 F6/§20.6 Finding 6/§21; the CDR is at **rev 53**, ADR-0002 §1.1 stays **Revision 17 / rev 21–36**, T02 at **rev 32**, M1 target at rev 29, M1 vertical at rev 27). No schema, interface, field, enum, rule, task-kind, chip, or `/6` change; the historical rev 21–38 ledger is preserved verbatim; no acceptance status changes; the CDR, ADR, task packages, and code are not edited. | M1 rev-39 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 41 (`/6` freeze execution; user-confirmed 2026-10-05; code + contract + tests, no language chips).** Freezes `t01-c01-c06/6` (hash `60935783b7b46cc62fc6fff64c532e840e7019a544055c594093d72dce0bf6d8`): RecordRef 24–26, RecordFamily 27 ordinals, Proposal +3 wire tags 5–7 (`ResultValue` unchanged), `StoreId::Names`, 9-field continuation, `ChildRef`/`ContinuationRef`, 4 scheduler limits + `try_new`, 6 `ConfigError` codes, `StoreOwnerViolation` + allowlist skeleton, span-u64, tags/arms/`encode_*` + round-trip decode, new reserved arenas + `in_flight` + `report` snapshot coverage, `Task.progress_count/ordinal` persisted with exceedance-to-`Fail`, commit-apply join/host-join/reservation core, quota dispatcher + cancel precedence + bounded recovery, `m1-append/1` seed, 16 new rule IDs (64 total), two-tier token. T01/owner co-freeze signatures still pending; numerics/spellings/encodings from the UNDECIDED lists deferred to named follow-ups; `SelectionBatchOverflow` stays on `CommitError` (carrier open); records-track typed apply explicitly rejected pending; `new()` stays `pub`. M1 application stays DRAFT; no language chips authorized. | `/6` freeze (user confirmation 2026-10-05) |
| 2026-10-05 | **Rev 40 (DRAFT, no code/commit/push; not a `/6` contract freeze; docs-only OPEN-03 fix / T01/T07/T08 co-freeze).** Closes the documentation-review OPEN-03 gap and the `§24.10`/OB-9 open item: the M1 committed constant-expression **input path** and the `ConstantRequest`/`ConstantResult` M1 **variants** are **selected and frozen for the M1 `/6` co-freeze** — the sem-stage per-use `const.evaluate` request covers the leaf-literal use (`ConstantRequest::Literal { literal, node, required_kind }`) and the binary additive use (`ConstantRequest::Binary { node, op: ConstExprOp, lhs, rhs, required_kind }`) with the committed `BinaryExpression` `NodeId`, T07's checked operator (`Add` in M1), and the committed T04 `LiteralRecord` refs; T08 commits **exactly one** `ConstRecord` for the folded result; T09 consumes the same committed `RecordRef::Const` and does not re-fold. A real T07→T08→T09 fixture (`M1-CL-05`, M1 frontend acceptance rev 31) replaces the hand-built-`5` evidence path; `M1-CL-03` is clarified as a conversion-unit fixture only. Still open: the result envelope (`ResultValue` carrier, OB-6), non-legal coupling (OB-7), `ConstRecord` identity/reuse (OB-8), and the exact wire tags/payload-variant spellings/task-kind spellings/numeric codes. Companion edits: T07 rev 4, T08 task-package revision 5, T09 rev 49, and M1 frontend acceptance rev 31. No ADR/T01/manifest/compiler/test edit by this revision (the CDR records the same co-freeze as rev 55); `/5` current; `/6` unfrozen; M1 DRAFT; no code. | OPEN-03 fix (documentation review 2026-10-05); T01/T07/T08 co-freeze |

---

## 17. Rev-22 integration ledger (accumulated rev-21 read-only review findings)

This section is the point-by-point disposition of every accumulated rev-21
read-only review finding. **Status key:** `RESOLVED-DOC` = corrected here as a
documentation-only fix that needs no authority decision; `BLOCKED` = an
authority-sensitive or exact-shape decision whose alternatives are recorded and
which remains an explicit `/6`/owner blocker (never silently chosen);
`PENDING-OWNER` = the fix needs an edit to the named task package, recorded in
CDR §13. Nothing here is a sign-off; `/5` stays current, M1 stays **DRAFT**,
ADR-0002 stays **PROPOSED**, and the T01 integrator plus every owner remain
**pending**.

### 17.1 T03 / T04 (preprocess + lex)

- **T03/T04-1 — single-file span reuse vs LX14/LX16 provenance:**
  `RESOLVED-DOC` (M1 limit) + `BLOCKED`. `TokenRecord.span` reusing one committed
  T03 PP span is exact only for a one-PP-token→one-C-token derivation (the M1
  single-file fixture). It does not cover `LX14 AdjacentStringChip` (a merged
  token whose provenance is a *sequence* of PP spans) or `LX16 TokenLocationChip`
  (synthesized/pasted/directive tokens with a computed macro origin). M1 has
  neither adjacency-concatenation nor macro/paste, so it is unaffected. **Blocker:**
  the general typed T04 provenance carrier (e.g. a committed token-origin record
  with `Vec<SpanId>`/synthetic-origin tag, or a T04-owned span field) is a T03/T04
  `/6` co-freeze item. See §5 `TokenRecord`.
- **T03/T04-2 — `source_scoped_one_hop` is one-source, not multi-source:**
  `RESOLVED-DOC` (M1 limit) + `BLOCKED`. The rule requires exactly one
  `RecordRef::Source`; it cannot model include expansion across multiple
  `SourceId`s (T03 `PP17`/`PP18`) or a multi-source `Preprocessed` artifact. M1 is
  the single-file fixture, so the rule is exact for M1 and **incomplete as a
  general contract**. **Blocker:** a typed multi-source map (committed
  source-map/provenance keyed over a set of `SourceId`s) with the artifact map
  defined over it; T03/integrator `/6`. Alternatives recorded in §6.1.
- **T03/T04-3 — non-M1 `ArtifactKind` writers:** `RESOLVED-DOC` +
  `PENDING-OWNER`/`BLOCKED`. Within M1 `artifacts.fragments` is T03-only and only
  `Normalized`/`Spliced`/`CommentFree` are produced; `Preprocessed` is
  declared-not-produced. `Assembly` (Part B, T11/probe-gated) and
  `Object`/`Snapshot`/`Trace` (Host/Part B) have **no M1 writer**; they are
  explicitly out of M1 scope, not silently assigned. The Part B owner remains
  unfrozen. See §5 `ArtifactKind`.
- **T03/T04-4 — `LiteralRecord` shape / `node`+`required_kind` placement:**
  `RESOLVED-DOC` + **user-accepted in principle 2026-10-04**. The lex-stage
  `LiteralRecord` is a per-literal lexical candidate fact
  (spelling/radix/suffix/magnitude + `LX08` candidate type) with **no `node`** and
  **no `required_kind`**; those belong to the **sem-stage per-use**
  `ConstantRequest`, and `legality` to the result. No `NodeId` is required before
  parse. See §5 `LiteralRecord` and CDR §C2/§F4. **Remaining `BLOCKED`:** the exact
  committed `legality` carrier and the `LX08` type/stage carrier field shapes are
  T04/T08 `/6` co-freeze items.
- **T03/T04-5 — CDR delta omissions:** `RESOLVED-DOC`. CDR §3.2 now records
  `SpanRecord.start/end` `u32→u64`, `ArtifactRecord {kind,bytes}→
  {kind,source,bytes,raw_offsets}`, the `sources.span_root`/`sources.expansion`
  writer boundary, and the artifact-map authority.
- **T03/T04-6 — M1 gate coverage:** `RESOLVED-DOC`. `M1-PP-08`, PP28 artifact
  production, committed-span reuse / no-T04-span-write, and the T04→T08 literal
  handoff are named gates in `M1_VERTICAL_SLICE_ACCEPTANCE.md` G1/G2.
- **T03/T04-7 — package amendments:** `PENDING-OWNER` (CDR §13); T03/T04 packages
  are not edited.

### 17.2 T05 (parse)

- **T05-1 — `ContinuationRefInvalid` referenced but absent:** `RESOLVED-DOC`.
  Added to the `CommitError` inventory (§6.4) with `# Committed missing / OwnBatch
  out of range`.
- **T05-2 — deleting `ContinuationRecord.awaited` conflicts with frozen T01 §4:**
  `ACCEPTED-IN-PRINCIPLE` (user decision B, 2026-10-04) + `PENDING` (T01/T05
  acceptance). The user accepts in principle that `TaskState::Waiting(WaitSet)` is
  the **sole** awaited-child-ID source and `ContinuationRecord` carries no duplicate
  `awaited`; option (b) — keep `WaitSet` only and amend T01 §4 — is selected **in
  principle for `/6`**, with **no `/5` edit**. Option (a) (retain `awaited`, drop
  `WaitSet.children`) is not selected. **T01 integrator and T05 owner acceptance
  remain pending**; the formal T01 §4 amendment is a `/6` item. See §5 continuation
  mapping and CDR §D4.
- **T05-3 — join resume count missing from capacity preflight:** `RESOLVED-DOC`.
  `join_reinserts[stage]` is added to the §7 phase-3 `queue_after[stage]` bound.
- **T05-4 — resume stage uses `continuation.production`:** `RESOLVED-DOC` (rev 23
  H2). Resume reinserts into `stage_queues[stage_of(parent.kind)]` (the parent
  **task kind**), and the realization must handle a parent whose `continuation` is
  **absent**; §5/§7/§8 and CDR §D3 are aligned.
- **T05-5 — CT07 join carrier absent / decorative:** `BLOCKED`. CT07 needs a
  concrete committed join-decision carrier consumed by commit-apply, or must be
  removed from the join path and the join defined as a commit-apply invariant.
  T02/T05 `/6`; no carrier invented here.
- **T05-6 — child-failure parent/sibling outcome contradictory:** `RESOLVED-DOC`
  (direction) + `BLOCKED` (policy). Selected draft direction: any committed child
  `Failed` fails the parent exactly once (never `Ready`); all `Completed` resumes
  `Ready`. Fail-fast vs await-all-siblings / sibling cancellation is a T05/T02
  `/6` blocker.
- **T05-7 — same-batch `TokenDraft` links impossible across T04/T05:** `RESOLVED-DOC`.
  A batch is single-task/single-owner, so node `first_token`/`last_token` are
  **committed `TokenId`s only**; the rev-21 same-batch wording is withdrawn.
- **T05-8 — enforce Node token-range invariants, tests:** `RESOLVED-DOC`.
  Invariants (endpoints exist, same declared source, `first <= last` in committed
  token order, leaf `first == last`, range covers spanned boundaries) and tests
  (`node_token_range`, `node_token_range_committed_source`,
  `node_leaf_token_eq`) are stated in §5/§13. **Rev 23 H5:** the
  `node_token_range_same_batch_source` test is retired with the same-batch
  `TokenDraft` branch (committed links only). The exact "range covers descendants"
  predicate stays a T05 `/6` item.
- **T05-9 — T04 PP-span provenance must be verifiable:** `BLOCKED` (T04).
  Committed `TokenRecord.span` must resolve to the recorded T03 PP token, not
  merely share a `source`. Until T04 exposes that link, the M1 check is explicitly
  same-source-only.
- **T05-10 — terminal diagnostic capacity vs no-`Running`:** `BLOCKED` (rev 23
  **H6**). The earlier `RESOLVED-DOC` reconciliation is **withdrawn**: clearing the
  dispatched/in-flight set does **not** clear a task's `Running` state, so the
  no-`Running` claim is not established. The whole mechanism (implicit terminal via
  job state vs explicit bounded transitions vs a capacity-respecting failure path)
  is a T01/T02 `/6` blocker; only the invariant is stated.
- **T05-11 — parse depth vs task depth / error order:** `RESOLVED-DOC`
  (clarification) + `BLOCKED`. Parse depth counts only parser-chain frames, not
  all bus tasks; exact predicate and error-vs-depth order are a T05 `/6` item.
- **T05-12 — empty-TU EOF range / synthetic nodes / `ErrorRecovery`:** `RESOLVED-DOC`.
  EOF token is zero-length at the raw end; synthetic nodes are not representable
  except `ErrorRecovery`; the recovery diagnostic location is a raw committed
  `TokenId`.
- **No T05 span write:** unchanged and preserved (T05 writes `parse.nodes` +
  `tasks.continuations` only).

### 17.3 T06 (scope/symbol/type)

- **T06-1 — stale `NodeRecord.span`:** `RESOLVED-DOC`. Point-of-declaration now
  derives from the node **token range** (`first_token`/`last_token` →
  `TokenRecord.span` → `SpanRecord`), not a `NodeRecord.span`.
- **T06-2 — canonical `TypeId` reuse lookup mechanism absent:** `BLOCKED`. How a
  TY13/TY17 task deterministically finds an existing committed canonical `TypeId`
  is undefined; T06 `/6`, no hidden cache/index invented.
- **T06-3 — `RedeclarationConflict` in the `CommitError` block:** `RESOLVED-DOC`.
  Removed from the `CommitError` list; it is a chip diagnostic via
  `Fail`/`DiagnosticDraft`.
- **T06-4 — missing test / unsupported claim:** `RESOLVED-DOC` (claim softened) +
  `PENDING`. The canonical-int-reuse claim now names the missing lookup mechanism
  as a blocker; a dedicated test cannot be specified until it is defined.
- **T06-5 — same-batch duplicate `(parent,kind)` scopes:** `RESOLVED-DOC`. The
  scope-duplicate rule is explicit that it is cumulative over committed + new
  same-batch scopes.
- **T06-6 — File-scope `Enter` point:** `RESOLVED-DOC` + `BLOCKED` (rev 23 **H3**).
  The File `Enter` `at` is the T05 `TranslationUnit` node with no Exit in M1, but
  it must be emitted by a **T06 task after the `TranslationUnit` is committed**,
  pinning a **committed `NodeId`** — **not** a job-bootstrap action. The bootstrap
  ordering (how the first T06 task is scheduled once the TU node is committed) is a
  T06/`[INT]` `/6` decision.
- **T06-7 — namespace-validation claim lacks a carrier:** `RESOLVED-DOC`
  (narrowed) + `BLOCKED`. Commit can only validate the derived `kind`→namespace
  function; a stronger lookup-namespace check needs a carrier or is chip-local.
- **T06-8 — `NORMATIVE_RULES`/test-list punctuation + duplicate ids:** `RESOLVED-DOC`
  (missing comma/stray paren fixed in §12.12) + `BLOCKED` (rev 23 **H8**: **both**
  `ir.op-immediate-type` **and** `ir.op-immediate-type-result-ty` are unresolved
  aliases, and **both** `ir.terminator-missing` **and**
  `ir.terminator-missing-explicit-commit-marker` are unresolved candidates; the
  `/6` inventory must collapse each pair).
- **T06-9 — package amendment pending; TY08 non-M1:** `PENDING-OWNER` (CDR §13).
  `TY08` remains non-M1.

### 17.4 T07 (semantics)

- **T07-1 — T09 `SemRecord` link/request missing; `CheckedNode` vs `SemRecord`
  identity:** `BLOCKED`. No typed request/link by which a T09 lowering task
  obtains a `SemRecord` for a `NodeId` is defined; and `CheckedNode` has no
  separate store/family. T07/T09 `/6` co-freeze.
- **T07-2 — `FunctionContextId` conflict:** `RESOLVED-DOC` (surfaced) +
  `PENDING`. The proposal selects the no-`FunctionContextId` direction, but the
  T07 package still requires one (`CheckRequest(..., FunctionContextId)`). Requires
  a T07 package amendment or a T07 `/6` owner decision; not authoritative.
- **T07-3 — `ConversionOp` domain / TY27 role / role→T09 mapping / `(op,role)`
  enforcement:** `BLOCKED`. Missing `FloatToFloat` and pointer-qualifier
  conversions; no TY27-instance→role mapping; the permitted/forbidden `(op,role)`
  set is unresolved. T06/T07/T09 `/6`.
- **T07-4 — VF06 `Return`/`FunctionDefinition` rows contradictory and stage
  unwired:** `RESOLVED-DOC` (CDR §F3 table rows reconciled) + `BLOCKED`. The
  matrix is still a T07/T13 `/6` item and no VF06 stage binding is claimed.
- **T07-5 — commit errors for duplicate/missing conversion role:** `RESOLVED-DOC`.
  `DuplicateConversionRole` added to `CommitError`; a *missing required* plan is a
  VF06 completeness failure, not a `CommitError`.
- **T07-6 — contradictory constant-request payloads; backwards sem/const
  dependency:** `RESOLVED-DOC` (ordering) + **H1 accepted in principle**
  (2026-10-04). The literal-decode request carries no `node`/`required_kind` and
  the sem request does; that payload split is now the **accepted-in-principle
  allocation** (working basis, not a freeze). The sem-precedes-const ordering (no
  T07↔T08 cycle) is separately selected and corrected in the vertical-acceptance
  doc.
- **T07-7 — test/rule inventory mismatches:** `BLOCKED` (`/6` inventory
  reconciliation).

### 17.5 T08 (constants)

- **T08-1 — lex-stage literal request does not carry `node`/`required_kind`:**
  **user-accepted in principle 2026-10-04** (H1 allocation). The accepted reading
  is two distinct typed payloads: a lex candidate fact and a sem per-use request.
  It is a **revision working basis, not a freeze**; the exact payload shapes still
  require T03/T04/T08/T01 co-freeze.
- **T08-2 — `legality` committed carrier / `LX08` type+stage unresolved:**
  `BLOCKED` (T04/T08 `/6` co-freeze).
- **T08-3 — `max_const_bits` numeric carrier / `<=128` / restricted config
  access:** `BLOCKED`. The carrier, the `<=128` config-validation constraint, and
  how a zero-field chip receives the bound are unresolved (T08 `/6`).
- **T08-4 — signed-representative wording for unsigned magnitudes:** `RESOLVED-DOC`.
  Unsigned magnitudes that fit the signed range are stored as the **positive**
  value (not a two's-complement reinterpretation); an out-of-range magnitude is
  `ConstOverflow`.
- **T08-5 — package amendment pending:** `PENDING-OWNER` (CDR §13).

### 17.6 T09 (IR)

- **T09-1 — `CompletedFunction` marker has no family/store/arena/encoder/tag;
  stale "at function end":** `BLOCKED` (preferred) + `RESOLVED-DOC`. The marker is
  left an explicit blocker rather than inventing a family; the phase-2b wording is
  restated as **marker-triggered, but as a selected *direction* only** (rev 23
  **H11**: the marker family/schema is unresolved — do not read "trigger fixed").
  **[Rev-49 note: the marker-family residual above is superseded for the
  operative direction by the rev-49 `FunctionEnd`-terminal-result reuse (no new
  marker family); history preserved. See §18.8.]**
- **T09-2 — rejection→rule-id list vs `NORMATIVE_RULES`:** `RESOLVED-DOC`
  (missing `ir.terminator-missing` added; missing comma/stray paren fixed in
  §12.12) + `BLOCKED` (rev 23 **H8**: **both** `ir.op-immediate-type` and
  `ir.op-immediate-type-result-ty`, and **both** `ir.terminator-missing` and
  `ir.terminator-missing-explicit-commit-marker`, are unresolved aliases; the exact
  hashed inventory is a `/6` item and no rule is claimed hashed now).
- **T09-3 — folded-`int5` ownership wording:** `RESOLVED-DOC`. T08 computes the
  `ConstRecord` value; T09 only emits the IR `Constant`; the rule/test names are
  reworded (`ir.folded-int5-constant-emission` / `folded_int5_ir_constant`) as a
  `/6` inventory item.
- **T09-4 — `Constant` missing-immediate vs missing-result precedence:**
  `RESOLVED-DOC` (stated: immediate evaluated first) + `BLOCKED` (exact canonical
  order is a T09 `/6` item).

### 17.7 T02 / T13 pipeline + ADR-0002

- **PIPE-1 — in-flight bound duplicated/misclassified:** `RESOLVED-DOC` (the
  duplicate commit-side check is withdrawn) **+ H9 removed in principle
  (rev 33)**. The rev-23 H9 blocker — the phase/owner for `max_inflight_total`,
  the residual-set semantics, and the dispatcher-`Ready→Running`-vs-ordered-atomic
  -commit relationship — is **partly resolved in principle**: on 2026-10-04 the
  user accepted **removing `max_inflight_total`** from the `/6` candidate because
  per-tick dispatch is already bounded by `max_inflight_per_tick`, the stage
  queues, and `max_tasks_total`, and `Waiting` is not in-flight; `tasks.in_flight`
  is an **ephemeral per-tick scheduler batch** cleared at latch after every
  dispatched task's outcome. The dispatch batch bound remains the per-tick
  `max_inflight_per_tick` quota. This is a **working-basis direction only, pending
  T01 integrator acceptance**; the dispatch-`Ready→Running`-vs-ordered-atomic-commit
  relationship and the residual-set semantics remain `[INT]` (§18.6).
- **PIPE-2 — add T13 VF02/VF03/VF04/VF13 pending amendment:** `PENDING-OWNER`
  (CDR §13).
- **PIPE-3 — CT07 decision carrier absent:** `BLOCKED` (same as T05-5). It must
  not be left decorative or replaced by an unbounded scan.
- **PIPE-4 — failure diagnostic capacity terminal path leaves `Running`:**
  `ACCEPTED-IN-PRINCIPLE` (recovery direction) + `BLOCKED` (implementation) —
  the earlier reconciliation is withdrawn; clearing the in-flight set does not
  clear `Running`, and clearing the in-flight set is **not itself** a transition.
  **Rev 32 (user direction in principle, 2026-10-04):** the batch failure-recovery
  **direction** is selected — fail every dispatched task exactly once in dispatch
  order, attaching a committed `DiagnosticId` when capacity allows (else the
  `DiagnosticId::NONE` sentinel), so every task leaves `Running` (generalizing the
  verified `/5` single-task semantics). The **exact atomic/bounded implementation
  still requires T01 integrator approval** and the diagnostic budget/state
  mechanism plus T02/T13 owner fixtures/sign-offs remain **pending** (same as
  §18.4/T05-10).
- **PIPE-5 — own-batch `DraftRef` vs accepted guardrail "cursors/joins never point
  at a draft":** `ACCEPTED-IN-PRINCIPLE` (user decision A, 2026-10-04) +
  `PENDING` (T01 implementation-confirmation). The user accepts in principle the
  **narrow interpretation**: a same-task `OwnBatch(DraftRef)` may exist only as a
  **transient wire/proposal input before commit**; commit validates/resolves it to a
  committed `TaskId`/record before persistent state; no durable cursor/`WaitSet`/
  join points at a draft/wire/address; the guardrail text is **unamended**. The
  remaining item is the **T01 integrator confirmation that the implementation
  matches** this reading. Alternatives are recorded in §3.12.
- **PIPE-6 — "accepted" wording in M1 §15.1/§10.4:** `RESOLVED-DOC`. Softened to
  selected/direction; §15.1 heading and §10.4 text updated.
- **PIPE-7 — dispatcher sole setter of `wires.selected`:** `RESOLVED-DOC`
  (confirmed in §6.2.1/§10.5/ADR-0002).
- **PIPE-8 — `dispatch_cursor` per worker invocation:** `RESOLVED-DOC`
  (confirmed: set immediately before ticking each task's worker layer, cleared
  after the invocation/at latch).
- **PIPE-9 — VF04 batch audit proposed-not-existing:** `RESOLVED-DOC` (kept as a
  proposed/residual extension, never cited as a defined verifier). **Quota policy
  preserved:** `quota = 1` is the semantic-comparison-projection baseline;
  `quota > 1` requires measured before/after tick counts **and** separate
  integrator acceptance.

### 17.8 Global consistency

- **G-1 — CDR dashboard:** `RESOLVED-DOC`. A/B/C are recorded as **in-principle
  direction only**; D–I are **PENDING**; row H now reads "marker-based trigger
  direction selected; marker family/schema unresolved" (**H11**) with the other
  unresolved encodings (rule-id aliases / immediate-vs-result precedence)
  preserved; row C records the **H1 allocation as accepted in principle
  (2026-10-04)**.
  **[Rev-49 note: the row-H marker wording above is superseded for the operative
  direction by the rev-49 `FunctionEnd`-terminal-result reuse (no new marker
  family); history preserved. See §18.8.]**
- **G-2 — T13 pending amendment:** `RESOLVED-DOC` (added to CDR §13; **T13 is not
  edited**, rev 23 H10).
- **G-3 — exact hash/inventory claims prospective only; no `/6` hashed claim:**
  `RESOLVED-DOC`. Every `M1AppendSchema`/hash statement is marked proposed or
  unresolved; no rule is claimed hashed before `/6`.
- **G-4 — `M1AppendSchema` proposed vs unresolved:** `RESOLVED-DOC`. The schema
  lists mark proposed shapes and unresolved shapes distinctly (CDR §I2/§9).

### 17.9 Residual register (must not be read as resolved)

`BLOCKED` items above are the open `/6`/owner decisions. No `/6` is authorized by
this revision; no chip may implement or depend on any item here; no owner or
integrator sign-off is implied. The T03–T09 and T13 task packages are **not
edited**; their amendments are requested in CDR §13.

---

## 18. Rev-23/24 integration ledger (independent rev-22 audit H2–H11; H1 accepted in principle rev 24)

This section is the point-by-point disposition of the independent **rev-22 audit**.
**Status key:** `RESOLVED-DOC` = corrected here as a documentation-only fix that
needs no authority decision; `BLOCKED` = an authority-sensitive or exact-shape
decision whose alternatives are recorded and which remains an explicit `/6`/owner
blocker (never silently chosen); `PENDING-USER` = needs an explicit user decision;
`ACCEPTED-IN-PRINCIPLE` = the user has accepted the working basis but it is **not**
a freeze and still needs owner/integrator co-freeze/sign-off;
`PENDING-OWNER` = the fix needs an edit to the named task package, recorded in CDR
§13. Nothing here is a sign-off; `/5` stays current, M1 stays **DRAFT**, ADR-0002
stays **PROPOSED**, and the T01 integrator plus every owner remain **pending**.

### 18.1 H1 — literal-handoff allocation (`ACCEPTED-IN-PRINCIPLE` 2026-10-04 + co-freeze)

- **H1.** The user's 2026-10-04 decision accepted the **semantic-information
  preservation requirement** (`node`, `required_kind`, `legality`, `LX08`) via a
  committed T04-owned `LiteralRecord`, with T08 the sole `constants.records`
  writer. On the **same date the user then explicitly accepted the recommended
  allocation split in principle (`接受拆分（推荐）`)**: the per-literal committed
  `LiteralRecord` carries the lexical facts + `LX08` candidate type, the post-parse
  sem-stage `ConstantRequest` carries the per-use `node`/`required_kind`, and the
  `ConstantResult` carries `legality`. **Status: `ACCEPTED-IN-PRINCIPLE`** — a
  revision working basis, **not a freeze** and **not code/chip authorization**.
  T01 integrator acceptance and T03/T04/T08 owner co-freeze/sign-off remain
  **pending**, and the exact field/enum/tag shapes are a `/6` co-freeze item.
  Recorded across §5 `LiteralRecord`, §8 handoff, §13 tests, §15.2.19/§15.3, and
  CDR §C2/§C3/§9/§10 (rev 24).

### 18.2 H2 — resume stage and absent continuation (`RESOLVED-DOC`)

- **H2.** Resume reinserts into `stage_queues[stage_of(parent.kind)]` — the parent
  **task kind** — **not** `stage_of(parent.continuation.production)`, and the
  realization must handle a parent whose `continuation` is **absent**. Fixed in §5
  (continuation comment), §7 (join realization), §8, and CDR §D3.

### 18.3 H3, H4, H5 — parse/join/token-link details (`RESOLVED-DOC`; H3 bootstrap `BLOCKED`; H4 policy selected rev 42)

- **H3 — File-scope `Enter` ordering:** `RESOLVED-DOC` + `BLOCKED` (bootstrap).
  The File `Enter` is emitted by a **T06 task after the T05 `TranslationUnit` is
  committed**, pinning a **committed `NodeId`**; it is **not** a job-bootstrap
  action. The bootstrap order is a T06/`[INT]` `/6` decision. Fixed in §5 (scope
  policy), §17.3 T06-6, CDR §E2.
- **H4 — join outcome:** `RESOLVED-DOC` (direction) + **policy selected (rev 42)**.
  All committed children `Completed` → parent `Ready`; any committed child `Failed`
  → parent `Failed` exactly once, **never** `Ready`. **Rev 42 (user, 2026-10-05;
  CDR §D; §22.1) selected the sibling policy — await-all** (all committed children
  terminal before the parent decision) and the **join as a commit-apply invariant
  with no new CT07 committed carrier/family**; the earlier fail-fast/
  sibling-cancellation framing is **stale**, and the fate of non-terminal siblings
  remains an open co-freeze item. Fixed in §5, §7 (join realization), §8, §17.2
  T05-6, CDR §D3. **OPEN-02 result
  delivery:** `RESOLVED-DOC` (direction) + co-freeze. The join must not consume a
  child result without a durable parent-side handoff; the selected draft direction
  keeps consumption with the parent's own atomic commit (fallback: transfer at
  join into parent-owned durable state), and `join_then_replay_retry` is required;
  the exact carrier/envelope remains T01/T05/T02 `/6` co-freeze (OB-4). Fixed in
  §5, §7, §13, §24.
- **H5 — same-batch `TokenDraft` node link:** `RESOLVED-DOC`. The branch is
  **removed**: node `first_token`/`last_token` are **committed `TokenId`s only**;
  the `node_token_range_same_batch_source` test is retired. Fixed in §5, §7 phase
  2b, §13, §17.2 T05-7/T05-8, CDR §13.

### 18.4 H6 — no-`Running`/terminal diagnostic mechanism (`ACCEPTED-IN-PRINCIPLE` direction rev 32; mechanism selected rev 42; not frozen/implemented)

- **H6.** Clearing the in-flight set does **not** clear a task's `Running` state;
  the rev-22 "reconciliation" claim is **withdrawn**. Only the invariant "no
  dispatched task remains `Running` at latch" is stated. **Rev 32 (user direction
  in principle, 2026-10-04)** selected the batch failure-recovery **direction** —
  on a failed atomic semantic batch commit, transition **every** dispatched task,
  in **dispatch order**, **exactly once** to `TaskState::Failed`, attaching a
  committed `DiagnosticId` when diagnostic/record capacity allows (else the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel), so every dispatched task
  leaves `Running`; **clearing the in-flight set is not itself a transition**;
  this generalizes the verified `/5` `fail_selected` single-task sentinel
  obligation (unchanged). **Rev 42 (user, 2026-10-05; CDR §A5; §22.1) selected the
  mechanism to be frozen in `/6`:** pre-dispatch errors before any state mutation
  leave the affected tasks `Ready`; a semantic batch commit failure commits **no**
  semantic state; a **deterministic bounded recovery mutation** then processes the
  dispatched tasks **once in dispatch order** to `Failed`; **no pre-reservation of
  N diagnostics**; a **per-task diagnostic attempt** with the `DiagnosticId::NONE`
  fallback when capacity is insufficient; and a **state guard** prevents the
  duplicate transition. It is **not implemented or frozen**: the exact
  atomic-commit realization requires **T01 integrator approval**, and the H9
  limits/stages/hash/error inventories plus the T02/T13 owner fixtures/sign-offs
  are **pending**. Fixed in §3 item 5/§3 item 12, §7 (one-transition property),
  §10.5 step 5, §17.7 PIPE-4, T02 (§Rev 22/23/25/26/27 bullets, CT06 row),
  ADR-0002 §2.1 invariant 4/§2.4 step 6/§2.7/§6, CDR §A5/§9/§10.

### 18.5 H7, H8 — IR naming and prospective-`/6` inventory (`RESOLVED-DOC` + `BLOCKED`)

- **H7 — folded-`int5` rename:** `RESOLVED-DOC`. `folded_int5_part_a_producer` /
  `ir.folded-int5-part-a-producer` is renamed to `folded_int5_ir_constant` /
  `ir.folded-int5-constant-emission`, with the T08-computes-`ConstRecord`-value /
  T09-emits-IR-`Constant` split stated. Fixed in §5, §13, §17.6 T09-3, CDR §H.
- **H8 — prospective `/6` + unresolved aliases:** `RESOLVED-DOC` + `BLOCKED`. All
  present-tense "frozen/hashed/pinned" IR terms are made **prospective `/6`**
  (op table, `ir.op_table_hashed` test, IR fields); **both** `ir.op-immediate-type`
  aliases (`ir.op-immediate-type`, `ir.op-immediate-type-result-ty`) and **both**
  `ir.terminator-missing*` candidates (`ir.terminator-missing`,
  `ir.terminator-missing-explicit-commit-marker`) are inventoried as **unresolved**
  and must be collapsed at `/6`; the dangling `§3.6` reference is corrected to
  `§3 item 6`. Fixed in §5, §12.12, §13, §17.6 T09-2, CDR §H.

### 18.6 H9 — in-flight scheduling ownership (`ACCEPTED-IN-PRINCIPLE` removal direction 2026-10-04 + remaining `[INT]`)

- **H9 (current status: removal direction accepted in principle; remaining `[INT]`
  items open).** **Current status (rev 33, user direction in principle,
  2026-10-04):** the user accepts **removing `max_inflight_total`** from the `/6`
  candidate — sequential per-tick dispatch is already bounded by
  `max_inflight_per_tick` (the dispatch batch bound), the per-stage
  `stage_queue_bound`, and `max_tasks_total`; `Waiting`
  tasks are not in-flight; and `tasks.in_flight` is an **ephemeral per-tick
  scheduler batch** cleared at latch only after every dispatched task has a
  terminal/`Waiting`/`Progress` outcome (or H6 recovery), so no residual in-flight
  set survives and a separate total bound is unnecessary. A future cross-tick
  `Running` mode would need a **separate CDR**. The dispatcher's pre-worker
  `Ready → Running` mutation is distinguished from the one ordered atomic semantic
  commit. **This is a working-basis direction only, pending T01 integrator
  acceptance**, and the remaining `[INT]` items (the dispatcher-mutation-vs-
  ordered-atomic-commit relationship and the residual-set semantics) stay open. No
  bound/config/hash is frozen. **Historical rev-23 framing (superseded, retained):**
  the single phase/owner for `max_inflight_total`, the residual-set
  semantics of the in-flight set, and the relationship between the dispatcher's
  `Ready → Running` mutation and the ordered atomic commit were **undefined `/6`
  decisions**, not resolved; the rev-22 "single-phase in-flight bound, fixed here"
  claim was withdrawn. The rev-23 one-phase-owner blocker is superseded by the
  removal direction above; the residual-set and dispatcher-mutation-vs-commit items
  remain open. Fixed in §6.2.1, §7 phase-3, T02, ADR-0002 §6, CDR §A/§9.
  Distinct from the hard/committed-ID cursor rules: the `Guardrails §6.1`
  own-batch **narrow interpretation is accepted in principle** (user decision A,
  2026-10-04; §17.7 PIPE-5/§19.4), with T01 implementation-confirmation pending.

### 18.7 H10 — stale rev-21 references and `AwaitChildren` (`RESOLVED-DOC`)

- **H10.** Stale rev-21 references were corrected to rev 21–26 (rev 23 corrected
  rev 21 → 21–23; rev 24/25 extended the aligned set; rev 26 extends it to 21–26)
  in CDR §11/§12/§13,
  ADR-0002 §1.1/§8, and this proposal; **T13 is recorded as not edited** alongside
  T03–T09. `AwaitChildren` is added to §2.4 (proposal additions) and to the §3
  item-5 one-transition set. Fixed in §2.4, §3, §16, §17, CDR §11/§12/§13,
  ADR-0002. **Rev 26:** every operative rev 21–24 pointer is now rev 21–26.
  **Rev 28:** the current-state pointer is **rev 21–28** (the rev-26 statement
  above is historical; see the §16 rev-28 row). **Rev 29:** the current-state
  pointer is **rev 21–29** (the rev-28 statement above is historical; see the §16
  rev-29 row). **Rev 30:** the current-state proposal pointer is **rev 30**
  (ADR-0002 §1.1 is **rev 21–30**); see the §16 rev-30 row. **Rev 31:** the
  current-state M1 pointer/range is **rev 21–31** (this proposal pointer rev 31;
  ADR-0002 §1.1 **rev 21–31**) — a docs-only history-gap repair that also supplies
  the missing historical rev-29 row; see the §16 rev-31 row. **Rev 32:** the
  current-state M1 pointer/range is **rev 21–32** (this proposal pointer rev 32;
  ADR-0002 §1.1 **rev 21–32**, Revision 13) — the H6 recovery-direction record; see
  the §16 rev-32 row. **Rev 33:** the current-state M1 pointer/range is
  **rev 21–33** (this proposal pointer rev 33; the H9 `max_inflight_total` removal
  direction + the D1–D3 corrections) — see the §16 rev-33 row and §21. **Rev 34:**
  the current-state M1 pointer/range is **rev 21–34** (this proposal pointer
  rev 34; the H9 cross-document completion in CDR/ADR-0002/T02 and the D3 CDR
  correction) — see the §16 rev-34 row. **Rev 35:** the current-state M1
  pointer/range is the current **rev 21–35** (this proposal pointer rev 35; the
  stale-active-prose cleanup in §4/§18.6/§18.7 and companion CDR rev 37/ADR
  Revision 15/T02 rev 29) — see the §16 rev-35 row. **Rev 36:** the current-state
  M1 pointer/range is **unchanged at rev 21–35** (this proposal document is at
  rev 36 for the header current-state pointer and the §19.3/§21 typo fixes;
  companion CDR rev 38/ADR Revision 16) — see the §16 rev-36 row. **Rev 37:** the
  current-state M1 pointer/range is the current **rev 21–36** (this proposal
  document rev 37; the proposal-side mirror of the explicit CDR rev 42–50
  decisions, with the CDR at rev 50, ADR-0002 at Revision 16, and T02 rev 30) —
  see the §16 rev-37 row and §22. **Rev 38:** the current-state M1 pointer/range is
  the current **rev 21–37** (this proposal document rev 38; the docs-only
  integration of the then-current companion state at the rev-38 snapshot — CDR rev 51;
  the *current* CDR is rev 52 and points at this proposal rev 38; ADR-0002 Revision 17,
  T02 rev 31, M1 target rev 29, M1 vertical rev 27) — see the §16 rev-38 row and §23.
  **Rev 39:** the current-state M1 pointer/range is the current **rev 21–38** (this
  proposal document rev 39; the docs-only integration of the current companion
  state — CDR **rev 53**, ADR-0002 Revision 17, T02 rev 32, T03 rev 53–54, T04
  rev 47, T05–T09/T13 updated, M1 target rev 29, M1 vertical rev 27) — see the
  §16 rev-39 row and §24. The current CDR (**rev 53**) points at proposal **rev
  38** as its parallel expected file until the CDR is next updated.
  **Pointer correction (same rev 39; historical — see the rev-40 header pointer for the current values):** the then-current CDR rev 54 pointed at proposal rev 39
  (expected); T02 was at rev 33; T03 at rev 54; T04 at rev 47 + task revisions 1–2; T13 at rev 5; M1 vertical
  was at rev 28; no revision advance, no acceptance change, nothing frozen.

### 18.8 H11 — trigger wording (rev-23 marker wording; operative direction superseded by rev-49 `FunctionEnd` terminal result) (`RESOLVED-DOC`)

- **H11 (historical rev-23 correction; operative direction superseded by rev 49).**
  "trigger fixed" became "**marker-based trigger direction selected;
  marker family/schema unresolved**". Fixed in CDR dashboard row H/§H1/§10 row H,
  M1 target §3.2, and §5/§16. **Rev 49 (user, 2026-10-05; §22.4) selected the
  operative direction:** the `TerminatorMissing` trigger **reuses the committed
  IR28 `FunctionEnd` terminal result** — the `TaskState::Completed(ResultId)`
  produced by the task whose kind is `FunctionEnd` — as the **deterministic
  function-completion fact**, checked by a **T01-owned typed phase-2b commit-apply
  validation hook** (the function's entry block is terminated). This adds **no**
  new marker record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot
  encoder and **no** new `ResultValue` variant; the earlier new-marker
  (`CompletedFunction`) direction is **superseded for the operative direction but
  preserved as history**. The selected direction is **not a frozen `/6` hook/schema
  and authorizes no code**: the exact hook contract, its hash impact, the result
  typing/commit ordering, and the T09/T01 co-freeze remain **open**, and **overall
  H stays PENDING**.

### 18.9 Residual register (must not be read as resolved)

The remaining open `/6`/owner decisions above are: the **H6** exact
atomic-commit realization/implementation (the rev-42 mechanism is **selected** to
be frozen in `/6`; it is **not implemented/frozen** — §18.4), the **H9** in-flight
scheduling-ownership
`[INT]` items (the `max_inflight_total` **removal direction** is
`ACCEPTED-IN-PRINCIPLE` from rev 33 — pending T01 integrator acceptance — while the
dispatcher-`Ready→Running`-vs-ordered-atomic-commit relationship and residual-set
semantics stay open), the **H8** rule-alias inventory, and the **H11** exact hook
contract/hash impact/result typing/commit ordering (the rev-49
`FunctionEnd`-terminal-result direction is selected, **no** new marker family;
overall H stays **PENDING**). The **H1** allocation is now
`ACCEPTED-IN-PRINCIPLE` (user, 2026-10-04) but **not frozen** — its exact shapes
remain a T01 + T03/T04/T08 co-freeze/sign-off item. No `/6` is authorized by this
revision; no chip may implement or depend on any item here; no owner or integrator
sign-off is implied. The T03–T09 and T13 task packages are **not edited**; their
amendments are requested in CDR §13.

---

## 19. Rev-25 integration ledger (rev-24 independent audit F1–F9 + verified `/5` failure guarantee)

This section is the point-by-point disposition of the **rev-24 independent
audit** F1–F9 plus the verified frozen `/5` failure guarantee. **Status key:**
`RESOLVED-DOC` = corrected here as a documentation-only fix that needs no
authority decision (it does not settle any open owner/integrator choice);
`BLOCKED` = an authority-sensitive or exact-shape decision whose alternatives are
recorded and which remains an explicit `/6`/owner blocker (never silently chosen);
`CANDIDATE-ONLY` = a proposed reading that is explicitly **not** a resolution of
an accepted rule and is linked to an open reconciliation request; `PENDING-OWNER`
= the fix needs an edit to the named task package, recorded in CDR §13;
`ACCEPTED-IN-PRINCIPLE` = a user-accepted-in-principle working basis that is **not**
a freeze (rev 30 adds decisions A/B under this status). Nothing here is
a sign-off; `/5` stays current, M1 stays **DRAFT**, ADR-0002 stays **PROPOSED**,
H6's rev-42 mechanism is **selected but not implemented/frozen**, and the T01
integrator plus every owner remain **pending**.
The concrete edits are in §5, §7, §10.2/§10.3/§10.5, §12.17, §13, §15.2,
§17.5/§17.7/§17.8, §18.3/§18.7, CDR §A/§C2/§F4/§H1/§9/§13, ADR-0002
§1.1/§2.1/§2.4/§2.7/§2.8/§6/§8, T02, and the two M1 acceptance documents.

### 19.1 F1 — present-tense `pinned`/`hashed` claims (`RESOLVED-DOC`)

- **F1.** In §5 the `ConstRecord` `max_const_bits`/`Hash-pinned` sentences and
  the `requires_map` "total, hashed" wording, and in §12.17 the `ConstRecord`
  "is pinned in the hash" / "are pinned in the same hashed section" sentences,
  are now marked **proposed to be pinned/hashed at `/6` — prospective, not pinned
  today**. The same present-tense claims in the CDR (§1, §3.2/§3.3, §B5, §C1,
  §E4, §G2, §H3, §I2, §5, dashboard) are qualified. **§12.17 is now within ledger
  coverage** (this item), satisfying the audit request that the cross-integration
  schema additions be explicitly tracked. No `/6` hash exists yet.

### 19.2 F2 + verified `/5` — single-task failure guarantee and selected batch mechanism (`RESOLVED-DOC` + `ACCEPTED-IN-PRINCIPLE` direction rev 32 + mechanism selected rev 42; not frozen/implemented)

- **Verified `/5` guarantee (read from the frozen `compiler/src/routing.rs`).** On
  a commit failure the `RoutingShell` calls `fail_selected`, which transitions the
  selected task to `TaskState::Failed` with a committed diagnostic when
  diagnostic/record capacity allows (else the `DiagnosticId::NONE` sentinel so the
  task is **not stranded**), and clears the singleton `tasks.active` /
  `control.selected`. This **single-task** semantic obligation is preserved by the
  proposal and is **not weakened**.
- **F2 (`ACCEPTED-IN-PRINCIPLE` direction rev 32 + mechanism selected rev 42; not
  frozen/implemented).** The
  rev-21 wording "on failure, fail every dispatched task exactly once and clear the
  in-flight set; no `Running` survives" was a **batch generalization that is not
  established**. Clearing `tasks.in_flight` does **not** clear `TaskState`, so a
  `Running` task state can survive, and clearing the in-flight set is **not itself**
  a transition. §7 (one-transition property), §10.2 step 5, §10.3, and §10.5 step 5
  preserve the `/5` single-task obligation, state the above, and make **no**
  no-`Running` claim for the batch beyond the selected direction. **Rev 32 (user
  direction in principle, 2026-10-04):** the user selected the **deterministic
  all-dispatched failure recovery direction** — every dispatched task, in
  **dispatch order**, **exactly once** to `TaskState::Failed`, a committed
  `DiagnosticId` when diagnostic/record capacity allows (else the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel), so every task leaves
  `Running`; this generalizes the verified `/5` single-task transition. **Rev 42
  (user, 2026-10-05; CDR §A5; §22.1):** the **quota>1-capable bounded batch
  recovery mechanism is selected to be frozen in `/6`** — pre-dispatch errors
  before any state mutation leave the affected tasks `Ready`; a semantic batch
  commit failure commits **no** semantic state; a deterministic bounded recovery
  mutation then processes the dispatched tasks once in dispatch order to `Failed`;
  no pre-reservation of N diagnostics; a per-task diagnostic attempt with the
  `DiagnosticId::NONE` fallback when capacity is insufficient; and a state guard
  prevents the duplicate transition. **The exact atomic-commit realization is
  `[INT]` (T01 integrator approval) and is not implemented/frozen**, and the H9
  limits/stages/hash/error inventories plus the T02/T13 owner fixtures/sign-offs
  remain **pending**. The selected design must uphold **no task remains `Running`,
  exactly-one transition per dispatched task, bounded diagnostics, and failure
  atomicity**. The frozen `/5` file is **not edited**.

### 19.3 F3 — `LiteralRecord`/`ConstantRequest` carriers and rev-24 history (`RESOLVED-DOC` + `BLOCKED`)

- **F3.** §5 now gives `LiteralRecord` a clearly proposed, **unfrozen**
  `candidate_type` field (the `LX08` candidate type allocated to the per-literal
  record by H1), and drafts **distinct proposed** `ConstantRequest { literal:
  RecordRef::Literal, node, required_kind }` and `ConstantResult { value:
  RecordRef::Const, legality }` carriers. The **exact enum/type encoding**
  (`RequiredKind`, `ConstLegality`, the candidate-type encoding) is explicitly
  **pending co-freeze**. CDR §C2/§F4 resolve the earlier §C2-vs-§F4 ambiguity
  (`ConstantRequest { node, required_kind }` vs `{ literal, node, required_kind }`)
  in favor of the request **referencing the committed literal**; CDR §13's T04 row
  withdraws "without asserting which carrier wins". The rev-24 history entries in
  this proposal and the CDR are corrected to state that the **proposed carrier
  shapes were drafted in rev 25**, not edited in rev 24.

### 19.4 F4 — own-batch `DraftRef` vs Guardrails §6.1 (`ACCEPTED-IN-PRINCIPLE` — narrow interpretation)

- **F4.** The own-task-batch `ContinuationRef::OwnBatch`/`ChildRef::OwnBatch`
  candidate now applies the **user's in-principle narrow interpretation** of
  [Guardrails](COMPILER_DEVELOPMENT_GUARDRAILS.md) §6.1 (rev 30, 2026-10-04): a
  same-task `OwnBatch(DraftRef)` may exist **only as a transient wire/proposal
  input before commit**; commit validates/resolves it to a committed `TaskId`
  **before any persistent state**; and **no durable cursor/`WaitSet`/join may point
  at a draft, wire, or address**. The guardrail text is **unamended** and is **not
  edited**; the remaining open item is the **T01 integrator
  implementation-confirmation** that the implementation matches this reading
  (§3.12, §17.7 PIPE-5, §18.6, CDR §A, T02, ADR-0002 §2.1 invariant 6). This is an
  accepted-in-principle working basis, **not a freeze**.

### 19.5 F5 — CDR §9 trigger wording (rev-25 marker correction; rev-49 supersedes the marker direction) (`RESOLVED-DOC`)

- **F5 (historical rev-25 correction; operative direction superseded by rev 49).**
  CDR §9 no longer lists the terminator trigger as an explicit
  `IR28` trigger "selected"; it stated the **marker-based `TerminatorMissing`
  trigger direction, with the marker family/schema unresolved (H11)**, and noted
  that this supersedes any "explicit trigger selected" reading. **Rev 49 (user,
  2026-10-05; §22.4):** the operative direction is the **`FunctionEnd`
  terminal-result reuse** — the `TerminatorMissing` trigger reuses the committed
  IR28 `FunctionEnd` terminal result (`TaskState::Completed(ResultId)`, task kind
  `FunctionEnd`) as the deterministic function-completion fact, checked by a
  **T01-owned typed phase-2b commit-apply validation hook**; this adds **no** new
  marker record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot
  encoder or `ResultValue` variant, and the earlier `CompletedFunction`
  family/store/arena/encoder/tag direction is **superseded for the operative
  direction and preserved as history**. The exact hook contract, its hash impact,
  the result typing/commit ordering, and the T09/T01 co-freeze remain **open**
  (overall H stays **PENDING**).

### 19.6 F6 — rev 21–23 pointers (`RESOLVED-DOC`)

- **F6.** Operative rev 21–23 pointers were corrected to **rev 21–24** in rev 25:
  this proposal (§16 intro, §18.7 H10), CDR §13, and ADR-0002 §1.1/§8. The
  historical rev-23 statements remain historical. **Rev 26:** the current-state
  pointer is extended to **rev 21–26** (see §20). **Rev 28:** the current-state
  pointer is extended to **rev 21–28** (see the §16 rev-28 row). **Rev 29:** the
  current-state pointer is extended to **rev 21–29** (see the §16 rev-29 row).
  **Rev 30:** the current-state proposal pointer is **rev 30** (ADR-0002 §1.1
  **rev 21–30**; see the §16 rev-30 row). **Rev 31:** the current-state
  M1 pointer/range is **rev 21–31** (ADR-0002 §1.1 **rev 21–31**); see the §16
  rev-31 row. **Rev 32:** the current-state M1 pointer/range is **rev 21–32**
  (ADR-0002 §1.1 **rev 21–32**, Revision 13); see the §16 rev-32 row. *(The rev-32
  pass left this F6 paragraph at **rev 21–31** — audit defect **D1**, corrected in
  rev 33.)* **Rev 33:** the current-state M1 pointer/range is **rev 21–33** (this
  proposal pointer rev 33; ADR-0002 §1.1 **rev 21–33**, Revision 14); see the §16
  rev-33 row and §21. **Rev 34:** the current-state M1 pointer/range is
  **rev 21–34** (this proposal pointer rev 34; ADR-0002 §1.1 **rev 21–34**,
  Revision 14, whose operative record of the H9 direction is unchanged); see the
  §16 rev-34 row. **Rev 35:** the current-state M1 pointer/range is the current
  **rev 21–35** (this proposal pointer rev 35; ADR-0002 §1.1 **rev 21–35**,
  Revision 15); see the §16 rev-35 row. **Rev 36:** the current-state M1
  pointer/range is **unchanged at rev 21–35** (this proposal document is at rev 36;
  ADR-0002 §1.1 **rev 21–35**, Revision 16); see the §16 rev-36 row. **Rev 37:**
  the current-state M1 pointer/range is the current **rev 21–36** (this proposal
  document rev 37; ADR-0002 §1.1 **rev 21–35**, Revision 16, unchanged); see the
  §16 rev-37 row and §22. **Rev 38:** the current-state M1 pointer/range is the
  current **rev 21–37** (this proposal document rev 38; ADR-0002 §1.1 **rev 21–36**,
  Revision 17); see the §16 rev-38 row and §23. **Rev 39:** the current-state M1
  pointer/range is the current **rev 21–38** (this proposal document rev 39; the
  CDR at **rev 53**, which points at proposal rev 38; ADR-0002 §1.1 **rev 21–36**,
  Revision 17; T02 at rev 32); see the §16 rev-39 row and §24.
  **Pointer correction (same rev 39; historical — see the rev-40 header pointer for the current values):** the then-current CDR rev 54 pointed at proposal rev 39
  (expected); T02 was at rev 33; T03 at rev 54; T04 at rev 47 + task revisions 1–2; T13 at rev 5; M1 vertical
  was at rev 28; no revision advance, no acceptance change, nothing frozen.

### 19.7 F7 — `node_token_range_same_source` vs the retired same-batch test (`RESOLVED-DOC`)

- **F7.** §5's token-range test list is reconciled with §13/§17.2: the active
  tests are `node_token_range`, `node_token_range_committed_source`,
  `node_leaf_token_eq`; the earlier draft name `node_token_range_same_source` is
  **superseded** by `node_token_range_committed_source` and is **distinct** from
  the retired `node_token_range_same_batch_source` (the withdrawn same-batch
  `TokenDraft` branch, H5), which is not an active name. The §5/§13/§17.2
  inventories now agree. **Rev 26** corrects the earlier §17.5 cross-reference
  (T08) to §17.2 (T05) and separates the two test names.

### 19.8 F8 — CDR row H / §3.2 hash wording (`RESOLVED-DOC`)

- **F8.** CDR dashboard row H and the §3.2 `ir.functions/blocks/values/
  instructions` row now say **proposed per-op rules for the `/6` hash (not hashed
  today)** rather than "hashed", consistent with F1/H8.

### 19.9 F9 — T02 "selected contract decisions" and historical AB1a/AB1b/AB2 (`RESOLVED-DOC`)

- **F9.** T02's "Runtime/layer realization (selected contract decisions,
  2026-10-04)" is restated as **selected draft direction, not an accepted contract
  and not owner/integrator-approved**. The historical proposal revision entry that
  said "AB1a/AB1b/AB2 accepted" now says **selected as a draft direction (not
  accepted)**; §10.2 likewise records the split as the selected draft direction,
  not an accepted contract. No AB1a/AB1b/AB2 interface is authorized.

### 19.10 Residual register (must not be read as resolved)

The **H6**
exact atomic-commit realization/implementation (the failure-recovery
**direction** is `ACCEPTED-IN-PRINCIPLE` from the rev-32 user decision, and the
**mechanism is selected (rev 42) to be frozen in `/6`** — fail every dispatched
task exactly once in dispatch order with the optional-diagnostic/
`DiagnosticId::NONE` sentinel, so every task leaves `Running`; **not implemented/
frozen**: the exact atomic-commit realization requires T01 integrator approval and
the H9 limits/stages/hash/error inventories plus T02/T13 fixtures/sign-offs remain
pending), the **H9** in-flight
scheduling-ownership `[INT]` items (the `max_inflight_total` **removal direction**
is `ACCEPTED-IN-PRINCIPLE` from rev 33 — pending T01 integrator acceptance — while
the dispatcher-`Ready→Running`-vs-ordered-atomic-commit relationship and the
residual-set semantics stay open), and
the **H8** rule-alias inventory remain **open/`BLOCKED`** (or, for H9's removal
direction, accepted-in-principle only). The **H1** allocation is
`ACCEPTED-IN-PRINCIPLE` (user, 2026-10-04) but **not frozen**; its exact
field/enum/tag shapes (including the proposed `candidate_type`,
`ConstantRequest`, and `ConstantResult`) remain a T01 + T03/T04/T08 co-freeze item.
The own-batch `DraftRef` vs Guardrails §6.1 is an **accepted-in-principle
narrow interpretation** (user decision A, 2026-10-04; §19.4) with the **T01
implementation-confirmation pending**, not a freeze. No `/6` is authorized;
no chip may implement or depend on any item here; no owner or integrator sign-off
is implied. The T03–T09 and T13 task packages are **not edited**; their amendments
are requested in CDR §13.

---

## 20. Rev-26 integration ledger (rev-25 independent audit findings 1–8 + nested C/D/E items)

This section is the point-by-point disposition of the **rev-25 independent audit**
of rev 25. The audit's **main findings are numbered 1–8** below; it also raised
**nested request-level items** against CDR §C (`C1`/`C2`/`C3`), CDR §D
(`D1`/`D2`/`D3`), and CDR §E (`E`), cross-referenced in §20.9. (The CDR §12
rev-26 record enumerates the same audit as **items 1–12**; §20.9 reconciles the two
enumerations and records the extra items 9–12.) **Status key:** `RESOLVED-DOC` =
corrected as a documentation-only fix that needs no authority decision (it settles
no open owner/integrator choice); `BLOCKED` = an authority-sensitive or exact-shape
decision whose alternatives are recorded and which remains an explicit `/6`/owner
blocker (never silently chosen); `CANDIDATE-ONLY` = a proposed reading that is
explicitly **not** a resolution of an accepted rule and is linked to an open
reconciliation request (rev 30 narrows the own-batch item to
`ACCEPTED-IN-PRINCIPLE` — decision A — so `CANDIDATE-ONLY` no longer applies to
it); `PENDING-OWNER` = the fix needs an edit to the named task
package, recorded in CDR §13; `ACCEPTED-IN-PRINCIPLE` = a user-accepted working
basis that is **not** a freeze (**the H1 literal-handoff allocation, accepted at
Rev 24**; the **A/B decisions, accepted at M1 proposal Rev 30 / CDR Rev 31**; the
**H6 batch-failure recovery direction**, accepted at rev 32; and the **H9
`max_inflight_total` removal direction**, accepted at rev 33 — **correction D2
(rev 33):** the earlier status key misattributed H1 to the rev-30 A/B decisions;
the acceptance dates are unchanged). **This ledger does
not claim the audit is fully
resolved:** H6 (batch no-`Running`/terminal **implementation** — the rev-42
**mechanism is selected** to be frozen in `/6`, but it is **not implemented/
frozen** and the exact atomic-commit realization plus the H9
limits/stages/hash/error inventories and
T02/T13 fixtures/sign-offs remain pending), H8 (rule-alias inventory),
H9 (from rev 33 the `max_inflight_total` **removal direction** is
`ACCEPTED-IN-PRINCIPLE`, pending T01 integrator acceptance, while the remaining
in-flight scheduling-ownership `[INT]` items stay open), the H1 exact shapes, and
the T01 §4
`awaited` supersession's T01/T05 acceptance (§20.10). Nothing here is a sign-off;
`/5` stays current, M1 stays **DRAFT**, ADR-0002 stays **PROPOSED**, and the T01
integrator plus every owner remain **pending**.

### 20.1 Finding 1 — `/5` single-task sentinel guarantee documented; batch recovery mechanism selected rev 42 (`RESOLVED-DOC` + `ACCEPTED-IN-PRINCIPLE` direction rev 32 + mechanism selected rev 42; not frozen/implemented)

- **Finding 1.** The frozen `/5` `compiler/src/routing.rs` `fail_selected`
  **single-task** obligation is documented exactly: the failed task **always**
  transitions to `TaskState::Failed`, attaching a committed `DiagnosticId` when
  diagnostic/record capacity allows and otherwise the
  `TaskState::Failed(DiagnosticId::NONE)` sentinel (so it is **never** stranded or
  left `Running`), and the singleton `tasks.active`/`control.selected` is cleared.
  The **batch** (`quota > 1`) fan-out ("fail every dispatched task exactly once so
  no `Running` survives") was **not** an established mechanism: clearing
  `tasks.in_flight` does **not** clear `TaskState`, so a `Running` task state can
  survive, and clearing the in-flight set is **not itself** a transition. **Rev 32
  (user direction in principle, 2026-10-04):** the batch recovery **direction**
  was selected — every dispatched task, in **dispatch order**, transitions
  **exactly once** to `TaskState::Failed`, a committed `DiagnosticId` when
  diagnostic/record capacity allows (else the `DiagnosticId::NONE` sentinel), so
  every task leaves `Running`; this generalizes the verified `/5` single-task
  semantics (unchanged). **Rev 42 (user, 2026-10-05; CDR §A5; §22.1):** the
  **quota>1-capable bounded recovery mechanism is selected to be frozen in `/6`** —
  pre-dispatch errors before any state mutation leave the affected tasks `Ready`; a
  semantic batch commit failure commits **no** semantic state; a deterministic
  bounded recovery mutation then processes the dispatched tasks once in dispatch
  order to `Failed`; no pre-reservation of N diagnostics; a per-task diagnostic
  attempt with the `DiagnosticId::NONE` fallback when capacity is insufficient; and
  a state guard prevents the duplicate transition. **It is not implemented/frozen:**
  the exact atomic-commit realization requires **T01 integrator approval**, and the
  H9 limits/stages/hash/error inventories plus the T02/T13 owner fixtures/sign-offs
  are **pending**. **H9 (rev 33, user direction
  in principle, 2026-10-04):** separately, the user accepts **removing
  `max_inflight_total`** from the `/6` candidate — per-tick dispatch is already
  bounded by `max_inflight_per_tick`, the stage queues, and `max_tasks_total`;
  `Waiting` is not in-flight; `tasks.in_flight` is an ephemeral per-tick scheduler
  batch cleared at latch after every dispatched task's outcome; a future
  cross-tick `Running` mode needs a separate CDR (pending T01 integrator
  acceptance; no bound frozen). **Operative
  locations:** §3 item 5/§3 item 12, §6.2.1, §7 (one-transition property + selection
  bullet + phase-3), §10.2 step 5, §10.3, §10.5 step 5, §18.4 H6, §18.6 H9, §19.2,
  §19.10, §21; CDR §A5/§9/§10; ADR-0002 §2.1 invariant 4, §2.4 step 6, §2.7, §2.8
  item 3, §6; T02 CT06 row + Rev 25/Rev 27/Rev 28 bullets; M1 target acceptance §7.
  The frozen `/5` file is **not
  edited**.

### 20.2 Finding 2 — `ConstantRequest` references the committed literal (`RESOLVED-DOC` + `BLOCKED`)

- **Finding 2.** The earlier §C2-vs-§F4 ambiguity (`ConstantRequest { node,
  required_kind }` vs `{ literal, node, required_kind }`) is resolved in favor of
  the request **referencing the committed T04-owned `LiteralRecord`**; the literal
  is not re-embedded. **Operative locations:** §5 `ConstantRequest { literal:
  RecordRef::Literal, node, required_kind }`, §8 handoff, §19.3; CDR §C2/§C3/§F4
  and the §13 T04 row (which withdraws "without asserting which carrier wins").
  **Remaining `BLOCKED`:** the exact `RequiredKind`/`ConstLegality`/candidate-type
  encodings remain a T04/T08/T01 `/6` co-freeze item.

### 20.3 Finding 3 — `LiteralDraft`/`LiteralRecord.candidate_type` added (`RESOLVED-DOC` + `BLOCKED`)

- **Finding 3.** The per-literal record now carries the proposed/unfrozen
  `candidate_type` field (the `LX08` candidate type allocated by the H1 split), and
  the `LiteralDraft` wrapper is field-symmetric. The lex-stage record still carries
  **no `node`/`required_kind`**. **Operative locations:** §5 `LiteralRecord`
  (`candidate_type`), §6.1 draft-wrapper table (`LiteralDraft` row), §13, §19.3;
  CDR §C2/§C3 and the §13 T04/T08 rows. **Remaining `BLOCKED`:** the exact
  enum/type encoding (`Lx08CandidateType`) is a T04/T08/T01 `/6` co-freeze item.

### 20.4 Finding 4 — test references fixed (`RESOLVED-DOC`)

- **Finding 4.** The node-token-range cross-reference is §17.2 (T05), **not**
  §17.5 (T08), and `node_token_range_same_source` is distinguished from the
  retired same-batch `node_token_range_same_batch_source` test. The active test
  names are `node_token_range`, `node_token_range_committed_source`, and
  `node_leaf_token_eq`. **Operative locations:** §5 token-range test list, §13,
  §17.2 T05-8, §19.7; CDR §13.

### 20.5 Finding 5 — hash/pin claims are prospective (`RESOLVED-DOC`)

- **Finding 5.** Every present-tense "hashed"/"pinned"/"frozen" claim is now
  **proposed for the `/6` hash, not pinned today**: §5 `ConstRecord`/`requires_map`,
  §12.17, §19.1; CDR §1, §3.2/§3.3, §B5, §C1, §E4, §G2, §H3, §I2, §5, the
  dashboard, and row H/§3.2/§13. No `/6` hash exists yet.

### 20.6 Finding 6 — current revision pointers updated (`RESOLVED-DOC`)

- **Finding 6.** The rev-26 current-state pointer was **rev 21–26**; it was
  advanced to **rev 21–28** in rev 28 and to **rev 21–29** in rev 29 (see the §16
  rev-28/rev-29 rows), and the proposal pointer to **rev 30** in rev 30 (ADR-0002
  §1.1 **rev 21–30**; see the §16 rev-30 row), then to **rev 31** in rev 31 (the
  current-state M1 pointer is **rev 21–31**; ADR-0002 §1.1 **rev 21–31**; see the
  §16 rev-31 row, which also supplies the missing historical rev-29 row), then
  to **rev 32** in rev 32 (the current-state M1 pointer is **rev 21–32**; ADR-0002
  §1.1 **rev 21–32**, Revision 13; see the §16 rev-32 row), and then to **rev 33**
  in rev 33 (the current-state M1 pointer is **rev 21–33**; ADR-0002 §1.1
  **rev 21–33**, Revision 14; see the §16 rev-33 row and §21), and then to **rev 34**
  in rev 34 (the current-state M1 pointer is **rev 21–34**; ADR-0002 §1.1
  **rev 21–34**, Revision 14; the CDR at rev 35/36; T02 at rev 28; see the §16
  rev-34 row), and then to **rev 35** in rev 35 (the current-state M1 pointer is
  the current **rev 21–35**; ADR-0002 §1.1 **rev 21–35**, Revision 15; the CDR at
  rev 37; T02 at rev 29; see the §16 rev-35 row), and **rev 35 remains the current
  range in rev 36** (this proposal pointer rev 36 for the header current-state
  pointer and typo fixes; CDR at rev 38; ADR-0002 §1.1 **rev 21–35**, Revision 16;
  T02 at rev 29; see the §16 rev-36 row), and the range advances to the current
  **rev 21–36** in rev 37 (this proposal document rev 37; the proposal-side mirror
  of the explicit CDR rev 42–50 decisions; CDR at rev 50; ADR-0002 §1.1
  **rev 21–35**, Revision 16; T02 at rev 30; see the §16 rev-37 row and §22), and
  the range advances to the current
  **rev 21–37** in rev 38 (this proposal document rev 38; the docs-only integration
  of the then-current companion state at the rev-38 snapshot; CDR at rev 51 at that
  snapshot — the *current* CDR is rev 52 and points at this proposal rev 38; ADR-0002 §1.1 **rev 21–36**,
  Revision 17; T02 at rev 31; M1 target at rev 29; M1 vertical at rev 27; see the
  §16 rev-38 row and §23), and
  the range advances to the current
  **rev 21–38** in rev 39 (this proposal document rev 39; the docs-only integration
  of the current companion state — CDR at **rev 53**, which points at proposal rev
  38; ADR-0002 §1.1 **rev 21–36**, Revision 17; T02 at rev 32; M1 target at rev 29;
  M1 vertical at rev 27; see the §16 rev-39 row and §24).
  **Pointer correction (same rev 39; historical — see the rev-40 header pointer for the current values):** the then-current CDR rev 54 pointed at proposal rev 39
  (expected); T02 was at rev 33; T03 at rev 54; T04 at rev 47 + task revisions 1–2; T13 at rev 5; M1 vertical
  was at rev 28; no revision advance, no acceptance change, nothing frozen.
  Historical
  rev-23/24/25 statements remain historical. **Operative locations:** CDR §11/§13;
  ADR-0002 §1.1/§8; this proposal §16, §18.7 H10, §19.6. The §20 pointer itself
  resolves (see the §16 rev-26 row).

### 20.7 Finding 7 — CDR closure wording (`RESOLVED-DOC`)

- **Finding 7.** The §C closure criterion no longer reads as if owner-accepted
  shapes follow from the in-principle decision; **owner acceptance of the exact
  `LiteralRecord`/`ConstantRequest` shapes is required and pending** (T01
  integrator + T03/T04/T08 co-freeze/sign-off), and the H1 allocation is
  accepted-in-principle only. **Operative locations:** CDR §C closure criterion
  (and the analogous §A/§D/§E/§F/§G/§H closure criteria); §15.2.19/§15.3; §18.1.

### 20.8 Finding 8 — quota-one no-valid-transition statement qualified (`RESOLVED-DOC` + `ACCEPTED-IN-PRINCIPLE` + mechanism selected rev 42; not frozen/implemented)

- **Finding 8.** The "tick produces no valid transition" statement is qualified:
  **at `quota = 1` only**, the verified `/5` single-task failure transition
  resolves the task (task → `Failed`; committed `DiagnosticId` when capacity
  allows, else the `DiagnosticId::NONE` sentinel), so a `StorePatch`-only or empty
  proposal set is not a stranded state at quota 1. **At `quota > 1`** the
  completion/no-`Running` guarantee is **not** claimed beyond the **rev-42 selected
  recovery mechanism** (deterministic bounded recovery: fail every dispatched task
  exactly once in dispatch order; a committed `DiagnosticId` when capacity allows,
  else the sentinel; pre-dispatch errors leave tasks `Ready`; no semantic state on
  a failed batch commit; no N-diagnostic pre-reservation; state guard) whose exact
  atomic-commit realization remains `[INT]`/co-freeze and which is **not
  implemented/frozen**. **Operative
  locations:** §7 selection bullet + one-transition property, §10.5 step 5, §18.4
  H6, §19.2, §20.1.

### 20.9 Nested request-level items (C1/C2/C3, D1/D2/D3, E) and CDR items 9–12

- **C1 (CDR §C — artifact inventory/source map/hash):** `RESOLVED-DOC` (hash
  qualification) + `BLOCKED`. The total 8-variant `ArtifactKind`/`requires_map` is
  **proposed to be hashed at `/6`**, not hashed today (Finding 5); the M1-produced
  set and the non-M1 writers remain as recorded (§17.1). Cross-refs: §5
  `ArtifactKind`, CDR §C1.
- **C2 (CDR §C — literal record + typed handoff):** `ACCEPTED-IN-PRINCIPLE`
  (allocation) + `BLOCKED` (exact shapes). The committed `LiteralRecord` carries
  the lexical facts + `LX08` candidate type, the sem-stage `ConstantRequest`
  carries `node`/`required_kind` and references the committed literal, and the
  `ConstantResult` carries `legality`; exact variants remain a T04/T08 `/6`
  co-freeze. Cross-refs: Findings 2/3/7; CDR §C2/§C3; §18.1.
- **C3 (CDR §C — single T08 `constants.records` writer):** `RESOLVED-DOC` +
  `BLOCKED`. T04 never writes `constants.records`; the `LX08` carrier is
  **assigned** to `LiteralRecord.candidate_type` (only its exact encoding pending).
  Cross-refs: Finding 2; CDR §C3/§13; §19.3.
- **D1 (CDR §D — exact continuation mapping):** `ACCEPTED-IN-PRINCIPLE` (direction,
  user decision A) + `PENDING` (exact fields/encoding, T05 acceptance). The
  `Task.continuation: Option<ContinuationId>` direction, the
  `ContinuationRecord.previous` committed-only back-link, and the
  `ContinuationRef::OwnBatch` successor resolution now apply the user's **narrow
  interpretation** of
  [Guardrails](COMPILER_DEVELOPMENT_GUARDRAILS.md) §6.1 (rev 30, 2026-10-04): the
  own-task draft key is a **transient wire/proposal input only**, validated and
  resolved to committed IDs **before persistent state**, so no durable
  cursor/`WaitSet`/join points at a draft/wire/address; the guardrail text is **not
  edited** and **T01 integrator implementation-confirmation is pending**
  (§3.12, §17.7 PIPE-5, §19.4; T02; ADR-0002 §2.1 invariant 6). Cross-refs:
  §17.2 T05-1/T05-2/T05-4, §18.3 H4/H5, §19.4; CDR §D1. Not a freeze.
- **D2 (CDR §D — own-batch validation):** `ACCEPTED-IN-PRINCIPLE` (own-batch
  narrow reading, user decision A) + `PENDING` (exact encodings, T05/T01
  acceptance). The own-batch key is a **transient wire/proposal input only**,
  resolved to committed IDs before persistent state (guardrail text unamended;
  T01 implementation-confirmation pending). The pre-apply
  `ContinuationRef`/`ChildRef`
  validation phases are selected draft direction; the `awaited`-vs-T01 §4
  supersession is **accepted in principle** (user decision B: `WaitSet`-only,
  formal `/6` amendment, no `/5` edit; T01/T05 acceptance pending), and the CT07
  question is **settled by rev 42** — the join is a **commit-apply invariant with
  no new CT07 committed carrier/family** (the exact encodings and the parse
  contract/tests remain open). Cross-refs: §17.2
  T05-2/T05-5, §17.7 PIPE-3/PIPE-5; CDR §D2/§D4.
- **D3 (CDR §D — parent waiting/resume):** `RESOLVED-DOC` (direction) + **policy
  selected (rev 42)**. All committed children `Completed` → parent `Ready`; any
  committed child `Failed` → parent `Failed` exactly once, **never** `Ready`; the
  resumed parent re-enters `stage_queues[stage_of(parent.kind)]` and the
  realization handles an absent `continuation` (H2). **Rev 42 (user, 2026-10-05;
  CDR §D; §22.1) selected the sibling policy — await-all** (all committed children
  terminal before the parent decision); the earlier fail-fast/sibling-cancellation
  framing is **stale**, and the fate of non-terminal siblings remains an open
  co-freeze item (H4). Cross-refs: §18.3, §17.2 T05-4/T05-6; CDR §D3.
- **E (CDR §E — T06 scopes/symbols/types):** `RESOLVED-DOC` + `BLOCKED`. The
  File-scope `Enter` is a **T06 task after the T05 `TranslationUnit` is
  committed**, pinning a committed `NodeId` (H3); the bootstrap order is a
  T06/`[INT]` `/6` decision, **not** a job-bootstrap action. The canonical
  `TypeId` reuse-lookup mechanism and the namespace-carrier decision remain
  blockers; the Finding-7 closure wording applies. Cross-refs: §17.3 T06-2/T06-6,
  §18.3 H3; CDR §E2.
- **CDR items 9–12 (enumeration reconciliation).** (9) The T02 "selected contract
  decisions" and the historical AB1a/AB1b/AB2 wording are **selected draft
  direction, not accepted** (`RESOLVED-DOC`; §19.9; T02; ADR-0002 §1.1). (10) The
  own-batch key vs Guardrails §6.1 is now an **accepted-in-principle narrow
  interpretation** (user decision A, 2026-10-04; §17.7 PIPE-5/§19.4; D1/D2 above)
  with the T01 implementation-confirmation pending — no longer an open request.
  (11) The `LX08` carrier is **assigned**
  to `LiteralRecord.candidate_type` (only its exact encoding pending)
  (`RESOLVED-DOC` + `BLOCKED`; CDR §9/§13; §19.3). (12) The H11 trigger
  wording and the rev pointers were rechecked (rev-23 `RESOLVED-DOC`; the
  marker-family part is **superseded by the rev-49 `FunctionEnd`-terminal-result
  direction** — **no** new marker family; the exact hook contract/hash impact/
  result typing/commit ordering and the T09/T01 co-freeze remain open; §18.8;
  §19.5; §20.6).

### 20.10 Residual register (must not be read as resolved)

- **`BLOCKED`:** H6 (batch no-`Running`/terminal **implementation** — the rev-42
  **mechanism is selected** to be frozen in `/6`: every dispatched task fails
  exactly once in dispatch order with a committed `DiagnosticId` when capacity
  allows, else the `DiagnosticId::NONE` sentinel, so every task leaves `Running`;
  pre-dispatch errors leave tasks `Ready`; a failed semantic batch commit commits
  no semantic state; no N-diagnostic pre-reservation; state guard. It is **not
  implemented/frozen**: the exact atomic-commit realization requires T01 integrator
  approval and the H9 limits/stages/hash/error inventories plus T02/T13
  fixtures/sign-offs remain co-freeze); H8 (IR rule-alias
  inventory); H9 (in-flight scheduling ownership); **H11 (the exact hook
  contract/hash impact/result typing/commit ordering and the T09/T01 co-freeze
  remain open — the rev-49 direction reuses the committed IR28 `FunctionEnd`
  terminal result (`TaskState::Completed(ResultId)`) checked by a T01-owned typed
  phase-2b commit-apply validation hook; **no** new marker record
  family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder or
  `ResultValue` variant; the earlier `CompletedFunction` marker direction is
  superseded for the operative direction and preserved as history; overall H stays
  PENDING)**;
  the H1 exact field/enum/tag shapes; the T01 §4 `awaited` supersession's **T01/T05
  acceptance** (the supersession itself is accepted in principle, user decision B);
  the D1/D2/D3 exact encodings/policies; and the E
  bootstrap ordering + canonical-`TypeId` reuse lookup.
- **`ACCEPTED-IN-PRINCIPLE` only:** the H6 batch failure-recovery **direction**
  (rev 32, 2026-10-04) — **extended by the rev-42 selected mechanism to be frozen
  in `/6`** (deterministic all-dispatched fail-once in dispatch order with the
  optional-diagnostic/`DiagnosticId::NONE` sentinel semantics, pre-dispatch `Ready`
  retention, no semantic state on a failed batch commit, no N-diagnostic
  pre-reservation, state guard; **not a freeze and not implemented**, exact
  atomic-commit realization T01-pending and T02/T13 owner fixtures/sign-offs
  pending); the **H9 `max_inflight_total` removal direction**
  (rev 33, 2026-10-04: remove the residual in-flight total bound from the `/6`
  candidate because per-tick dispatch is already bounded by
  `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`, and
  `Waiting` is not in-flight; a future cross-tick `Running` mode needs a separate
  CDR — **not a freeze, pending T01 integrator acceptance**, and no bound is
  frozen); the H1 literal-handoff allocation (not a
  freeze; owner/integrator co-freeze/sign-off pending); the own-batch `DraftRef`
  **Guardrails §6.1 narrow interpretation** (user decision A: transient
  wire/proposal input only, resolved to committed IDs before persistent state,
  guardrail unamended; **T01 implementation-confirmation pending**); and the
  awaited-child **`WaitSet`-only** decision (user decision B: sole source; no
  duplicate `awaited`; formal T01 §4 supersession in `/6`, **no `/5` edit**;
  **T01/T05 acceptance pending**).
- No `/6` is authorized by this revision; no chip may implement or depend on any
  item here; no owner or integrator sign-off is implied. The T03–T09 and T13 task
  packages are **not edited**; their amendments are requested in CDR §13.

---

## 21. Rev-33 corrections to the rev-32 record (D1–D3)

This section is the point-by-point correction of the three rev-32 H6 audit defects
integrated by rev 33. Nothing here is a sign-off; `/5` stays current, M1 stays
**DRAFT**, ADR-0002 stays **PROPOSED**, and the T01 integrator plus every owner
remain **pending**.

- **D1 — pointer-list repair (`RESOLVED-DOC`).** The rev-32 revision records (this
  proposal `§16` and CDR `§12`) listed the rev-32 edits as
  `§18.4/§19.2/§19.10/§20.1/§20.8/§20.10`. The **actual** rev-32 operative
  pointer locations were **this proposal header, §18.7 H10, and §20.6 Finding 6**
  (the other listed sections were edited for content but are not the current-state
  pointer anchors). The historical statements at those sections are preserved; the
  rev-32 `§16` row now names the actual locations (`rev 33` corrected row).
  In addition, `§19.6 F6` still read the current-state M1 pointer as `rev 21–31`; it
  is advanced to `rev 21–32`, then to `rev 21–33` in rev 33 (see §19.6), then to
  `rev 21–34` in rev 34, and to the current **`rev 21–35`** in rev 35. The current
  M1 proposal pointer is **rev 36** (the range stays **rev 21–35**); ADR-0002 §1.1
  is **Revision 16 / rev 21–35** (Revision 14 recorded the rev-34 pointer before
  the Rev 15 cleanup; Revision 16 clarifies the §3 `bus.rs` amendment row without
  changing the pointer range). **Rev 37:** the current M1 proposal document is
  **rev 37** and the pointer/range advances to **rev 21–36** (the proposal-side
  mirror of the explicit CDR rev 42–50 decisions; ADR-0002 §1.1 stays
  **Revision 16 / rev 21–35**; see §22). **Rev 38:** the current M1 proposal
  document is **rev 38** and the pointer/range advances to **rev 21–37** (the
  docs-only integration of the then-current companion state at the rev-38 snapshot —
  CDR rev 51; the *current* CDR is rev 52 and points at this proposal rev 38 — / ADR-0002 Revision 17 / T02 rev 31 / M1
  target rev 29 / M1 vertical rev 27; ADR-0002 §1.1 is **Revision 17 / rev 21–36**;
  see §23). **Rev 39:** the current M1 proposal document is **rev 39** and the
  pointer/range advances to **rev 21–38** (the docs-only integration of the current
  companion state — CDR **rev 53**, which points at proposal rev 38; ADR-0002
  Revision 17 / rev 21–36 / T02 rev 32 / T03 rev 53–54 / T04 rev 47 / M1 target
  rev 29 / M1 vertical rev 27; see §24).
  **Pointer correction (same rev 39; historical — see the rev-40 header pointer for the current values):** the then-current CDR rev 54 pointed at proposal rev 39
  (expected); T02 was at rev 33; T03 at rev 54; T04 at rev 47 + task revisions 1–2; T13 at rev 5; M1 vertical
  was at rev 28; no revision advance, no acceptance change, nothing frozen.
- **D2 — M1 §20 status-key repair (`RESOLVED-DOC`).** The `§20` status key
  previously read "the H1 allocation, from rev 30 decisions A/B", misattributing
  the H1 allocation. **H1 was accepted in principle at Rev 24**; the **A/B
  decisions were accepted in principle at M1 proposal Rev 30 / CDR Rev 31**. The
  user policy acceptance dates are unchanged; only the attribution is corrected
  (see the §20 status key).
- **D3 — CDR §13 frontend claim (`RESOLVED-DOC`; recorded in the CDR).** The CDR
  §13 sentence that the [M1 frontend acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md)
  carries a rev-26 H6/CT06 text is wrong: the frontend is at the latest **Rev 25**
  and has **no H6/CT06 content**. The operative H6 batch-failure text is carried by
  [T02](T02_CONTROL_CHIPS.md) (its Rev 27 record, updated to **Rev 28** by the H9
  integration). The frontend needs no H6/CT06 amendment for this change (see the
  CDR §13 corrected paragraph). **Rev 34:** the CDR §13 correction is actually
  applied as **CDR rev 36** (rev 33 recorded D3 here but the CDR edit was
  forward-referenced and not yet applied); the CDR §13 paragraph now states the
  frontend is at **Rev 25** with no H6/CT06 content and that T02 carries the
  operative H6 batch-failure text.

---

## 22. Rev-37 integration ledger (proposal-side mirror of CDR rev 42–50; historical as of rev 38)

This section is the **rev-37 historical** point-by-point proposal-side record of the
authority decisions
already recorded in the [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)
rev 42–50 and the owner task packages, mirrored here in rev 37 (the current
companion-state ledger is the **§23** rev-38 ledger). It records **no new
user decision**; it does **not** accept any bundle wholesale, does **not** freeze any
interface, does **not** change `compiler/contracts/CONTRACT_VERSION`, and does **not**
authorize any chip or code. **Status key:** `USER-ACCEPTED-SUBDECISION` = a named
per-subdecision authority response recorded in the CDR (overall bundle still
PENDING); `DELEGATED-CANDIDATE-DEFAULT` = selected by the M1 integration agent under
explicit user delegation, **explicitly not** an owner or T01 `[INT]` signoff;
`READ-ONLY-AUDIT` = a read-only finding recorded as fact, not a signature;
`USER-DIRECTION` = an explicit user direction (still not a `/6` freeze);
`USER-HASH-SCOPE-DECISION` = the rev-50 user scope decision (conceptual boundary
only). Nothing here is a sign-off; `/5` stays current, the `/6` contract stays
**unfrozen**, M1 stays **DRAFT**, ADR-0002 stays **PROPOSED**, and the T01 integrator
plus every owner remain **pending**.

### 22.1 User-accepted selected subdecisions rev 42–46 (`USER-ACCEPTED-SUBDECISION`)

- **Rev 42 (2026-10-05; overall bundles still PENDING).** **(a) A/T02/T13/T01 —
  H6 scope + mechanism.** H6 is retained and its **quota>1-capable bounded batch
  recovery mechanism is selected to be frozen in `/6`**, explicitly rejecting the
  §9B row-A deferral to a later CDR: pre-dispatch errors before any state mutation
  leave the affected tasks `Ready`; a semantic batch commit failure commits **no**
  semantic state; a **deterministic bounded recovery mutation** then processes the
  dispatched tasks **once in dispatch order** to `Failed`; **no pre-reservation of N
  diagnostics**; a **per-task diagnostic attempt** with the `DiagnosticId::NONE`
  fallback when capacity is insufficient; and a **state guard** prevents the
  duplicate transition. This is a selected mechanism only; the H9
  limits/stages/hash/error inventories and all fixtures remain PENDING. **(b)
  D/T02/T05/T01 — join/CT07.** The join is a **commit-apply invariant** with **no
  new CT07 committed carrier/family**, and the sibling policy is **await-all** (all
  committed children terminal before the parent decision). **(c) F/T06/T07/T09/T01
  — checked carrier.** **No `FunctionContextId`**; `SemRecord` is the committed
  materialization of `CheckedNode`, **one per `NodeId`**, with an **explicit
  committed typed link consumed by T09**. **(d) E/T06/T01 — canonical `TypeId`
  reuse.** A **deterministic bounded scan of committed `types.records`** returns the
  **lowest matching `TypeId`**, with **no hidden cache/index** (T05 upstream
  dependency acknowledged).
- **Rev 43 (2026-10-05; rows D/E still PENDING).** **D/T05** — the candidate
  `ContinuationRecord` **exact ordered fields** (`production: TaskKind`,
  `cursor: TokenId`, `context: ParseContext`, `binding_power: u16`,
  `scope: Option<ScopeId>`, `parent: Option<NodeId>`, `partial_children: Vec<NodeId>`,
  `next_child_ordinal: u32`, `previous: Option<ContinuationId>`), with **no
  `awaited`** and committed-ID references only (the `RecordRef` numeric wire tags and
  `RecordFamily` ordinals remain **separate inventories**; numeric encodings are a
  T01 `/6` detail; `ParseContext` vocabulary still needs the T05/T01 encoding). **D/
  T01/T05** — the formal `/6` **T01 §4 supersession** (`TaskState::Waiting(WaitSet)`
  is the only awaited-child source; continuation has no `awaited`; **`/5` unchanged**,
  no `/5` edit). **D/T02/T05/T01** — the exact **OwnBatch pre-apply validation**
  (committed continuation exists/live; OwnBatch continuation index within the same
  task's `AppendRecords` range/family `Continuation` in phase 1; committed child
  valid; OwnBatch child index within the same task Enqueue list in phase 2b; errors
  `ContinuationRefInvalid`/`AwaitChildrenRefInvalid`, whole-batch `CommitError`,
  before any mutation). **E/T06/T01** — the File `Enter` is a deterministic
  `parse.TranslationUnit → symbol_type.scope-enter` stage edge **after** the
  committed TU, carrying the committed `NodeId`, with **no job-bootstrap**; the T05
  committed-TU carrier remains an upstream dependency and is **not** a TU schema
  signoff.
- **Rev 44 (2026-10-05; overall C/G still PENDING).** **C/H1/T07/T08/T01** —
  `ConstantRequest.required_kind` is the **per-use constant-expression requirement**
  (e.g. the M1 integer constant expression), distinct from the lexical
  `LiteralRecord.candidate_type`, duplicating no implicit target type;
  `ConstantResult.legality` is a **result payload field** and adds **no** extra
  committed record family. **G/T08/T01** — `max_const_bits` origin is a **hashed
  `Limits` value** (`limits.max_const_bits` participates in the frozen-contract
  hash), M1 cap/default **128**, `config` **rejects** values **> 128**, and the bound
  reaches the zero-field T08 chip through an **explicit task-input projection**.
  **C/T03/T01** — accept `ArtifactRecord { kind, source, bytes, raw_offsets }` with
  the **total eight-`ArtifactKind`** map rule (map-mandatory
  `Normalized`/`Spliced`/`CommentFree`/`Preprocessed`; map-optional
  `Assembly`/`Object`/`Snapshot`/`Trace`), M1 **exercised scope only single-source
  `Normalized`**; other producers and the multi-source map deferred.
- **Rev 45 (2026-10-05; overall C/G still PENDING).** **C/T04/T08/T01** — the exact
  ordered `LiteralRecord` fields `token: Option<TokenId>`, `kind: LiteralKind`,
  `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` (big-endian magnitude),
  `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`
  (no `node`/`required_kind`); the M1 enum/scope `LiteralKind {Integer, Character,
  String}` (only `Integer` produced), `LiteralSuffix {None, U, L, UL, LL, ULL}`
  (only `None` produced), radix `{2, 8, 10, 16}` (M1 decimal only), and a symbolic
  `Lx08CandidateType` (M1 literals 2/3 = `Int`, no bit width; complete member
  set/numeric encodings remain open, not invented). **C/T07/T08/T01** —
  `RequiredKind` M1 enum only `IntegerConstantExpression`; `ConstLegality` values
  `Legal`/`NotConstantExpression`/`Unsupported` (future purposes via appended
  variants/new rules, not lexical-candidate reuse). **C/T03/T01** — mandatory
  artifact-map invariants `raw_offsets.len() == bytes.len()+1`, first `== 0`,
  monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds require a
  valid source; the optional-kind map rule, source-versus-payload equality, and
  exact artifact error classification/numeric codes remain open.
- **Rev 46 (2026-10-05; overall F still PENDING).** **F/T07** —
  `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` with the M1
  fixture producing **`NonLvalue`**; M1 allows **only `EffectMask(0)`** (a nonzero
  mask is a typed unsupported/diagnostic; the effect bit classes are
  reserved/unassigned). **F/T07/T13/T01** — VF06 **`TypedAstInvariant` is in M1**,
  executing **after the committed T07 `SemRecord`s and before T09 lowering**,
  checking M1 typed-fact/required-conversion completeness (the **exact registered
  task kind/stage/phase/interface remains T01/T13 co-freeze/open**). **F/T06/T07/
  T09/T01** — select the **M1-minimal conversion scope only** (freeze only the
  M1-fixture-needed conversion behavior incl. the identity/no-conversion rule; integer
  promotions, float conversions incl. `FloatToFloat`, pointer-qualifier conversions,
  and other non-M1 conversions are **explicitly unsupported/deferred** to a later
  append/contract revision); **no** `ConversionOp`/`ConversionRole` closed variant
  list, pairing, role→chip mapping, or numeric encoding is invented.

### 22.2 Rev 47 delegated candidate defaults (`DELEGATED-CANDIDATE-DEFAULT`)

- **Rev 47 (2026-10-05).** Under the user's explicit delegation ("for non-critical
  decisions adopt the recommended choice directly; ask only for critical decisions"),
  the **M1 integration agent** selected four low-risk **candidate defaults**, each
  **selected under user-delegated integration default** and **clearly labeled NOT an
  owner or T01 `[INT]` signoff** (not owner acceptance, not T01 `[INT]`, not a
  freeze): **(1) optional-`ArtifactKind` map policy** (map-optional kinds have empty
  `raw_offsets`; `source` stays `Option<SourceId>` and must be valid when `Some`;
  mandatory-map kinds require a valid `source` plus the accepted rev-45 invariants;
  **no** source-payload-equals-bytes requirement; source-provenance/equality rule
  deferred; exact numeric error codes open); **(2) T05 parse depth** (reuse
  `limits.max_task_depth`; count parser continuation/child frames only; detect the
  limit before any child enqueue for the descent; excess = the `ParseDepthExceeded`
  chip diagnostic; exact `ParseContext`/request-result variants pending); **(3) T06
  namespace/symbol lookup** (closed `SymbolKind`→namespace mapping with no namespace
  field; wrong-namespace lookup is a **miss not a conflict**; deterministic TY03
  active-scope-chain lookup ordering candidates by `(source,start,end,NodeId)`,
  greatest ≤ the query point, same-scope tie to the higher `SymbolId`, else the
  innermost active scope; T05 `NodeKind`/token-range dependency and exact event
  encoding/lifecycle/allowlist rows remain co-freeze); **(4) T09 noncritical rule
  defaults** (shorter aliases `ir.op-immediate-type` and `ir.terminator-missing`;
  `Constant` validates the **immediate before the result** when both are missing; the
  target type equals the result `ValueRecord.ty`; all prospective `/6`, and the
  `CompletedFunction` marker family stays unresolved with no family invented).
  **[Rev-49 note: the marker-family residual above is superseded for the
  operative direction by the rev-49 `FunctionEnd`-terminal-result reuse (no new
  marker family); history preserved. The rev-47 default itself is unchanged.]**
  Overall rows C/D/E/H remain **PENDING**; all public/shared schema awaits owner/T01
  co-freeze; no conflicting user-selected rev 42–46 decision is overridden.

### 22.3 Rev 48 code audit discrepancy and unresolved hash conflict (`READ-ONLY-AUDIT`)

- **Rev 48 (2026-10-05; as it existed then).** A read-only audit (docs-only; no
  code/T01/manifest/other-doc edit; no hash-scope option selected; no freeze)
  recorded the checked-in `/5` code facts: the frozen contract is `t01-c01-c06/5`
  (`61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5`);
  `compiler/src/contract.rs` encodes **24** `RECORD_KINDS`; `compiler/src/ids.rs`
  `RecordRef` has **24** variants (tags **0–23**, `Artifact` = 23); and
  `RecordFamily` is **absent** from the code. The draft candidate figures — **19
  draft families / 27 `RecordRef` variants / 27 `RecordFamily` ordinals** — are
  **proposed/unverified**, and the future `/6` count is **not** claimed to equal the
  current 24. The `M1AppendSchema` **hash-scope conflict** between
  `CONTRACT_VERSION`/`contract.rs`/`README.md`
  (`hash_excludes=…group-declared-store-fields…`) and `COMPILER_SFL_MANIFEST.md` §4
  is **real and unresolved**, and the three hash-scope options (**hash
  `M1AppendSchema`** + update the exclusion token/freeze assertion/manifest; **do
  not hash** + correct the manifest; **explicit two-tier hash-seed-vs-runtime-
  declarations**) were then **unselected and unrecommended** (no authority). The
  audit further required `/6` to specify dual-inventory encoding (wire tags vs
  ordinals), numeric-value inclusion in the hash, and the schema self-consistency
  mechanism. This added no owner acceptance and no signature.

### 22.4 Rev 49 H11/T09/T01 terminal result + T01 phase-2b hook and H9 residual blockers (`USER-DIRECTION` + `READ-ONLY-AUDIT`)

- **Rev 49 (2026-10-05).** The user recorded an explicit critical **H11/T09/T01
  direction**: the `TerminatorMissing` trigger **reuses the committed terminal
  result of the IR28 `FunctionEnd` task** — the `TaskState::Completed(ResultId)`
  produced by the task whose **kind is `FunctionEnd`** — as the **deterministic
  function-completion fact**, checked by a **T01-owned typed phase-2b commit-apply
  validation hook** (the function's **entry block is terminated**, i.e. its greatest
  `InstructionId` is a terminator). This adds **no** new marker record
  family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder and **no**
  new `ResultValue` variant; the earlier new-marker (`CompletedFunction`) direction
  is **superseded for the operative direction** but **preserved as history**. The
  selected direction is **not a frozen `/6` hook/schema and authorizes no code**: the
  exact hook contract, its hash impact (likely no new record family, but
  **rule/hook hashing remains a T01 decision**), the result typing/commit ordering,
  and the T09/T01 co-freeze remain **open**, and **overall H stays PENDING**.
  Separately, **read-only H9 audit findings** were recorded as **unresolved
  blockers** without changing the selected H6 mechanism or the H9 removal direction:
  the `/5` code proves only the quota=1 single-task `fail_selected`, so the proposed
  **quota>1 pipeline is unimplemented**; H9 still needs the **exact dispatcher
  pre-worker `Ready→Running`/`in_flight`-population relationship to the single
  ordered atomic commit**, the **exact `in_flight` clear ownership/order relative to
  the bounded H6 recovery**, and a **no-`Running`/no-residual proof for all
  success/error/empty-proposal paths**; the
  **`max_dispatches_per_tick`-vs-`max_inflight_per_tick` internal proposed-limit
  conflict** must be resolved in `/6` (not decided now); the H6/H9 fan-out fixtures
  and T13 VF02/VF03/VF04/VF13 remain **pending**; the H9 no-residual guarantee is
  **conditional on the H6 recovery implementation**, and the quota=1 M1 baseline
  stays **separated** from the quota>1 optimization.

### 22.5 Rev 50 two-tier hash scope (`USER-HASH-SCOPE-DECISION`)

- **Rev 50 (2026-10-05; conceptual boundary only).** The user recorded an explicit
  critical **hash-scope decision** accepting the **two-tier model**:
  `StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed and
  participates in the `/6` contract hash**, while **post-seed runtime
  `StoreSchema::declare()` extensions remain excluded** from the frozen hash and are
  **captured/validated through runtime snapshot/schema mechanisms**. This settles the
  **conceptual future `/6` boundary** (the three hash-scope options are no longer
  open at the conceptual level), but it **accepts no exact `M1AppendSchema`
  contents/counts, freezes nothing, changes no `/5`, and authorizes no code**: the
  numeric inventory and freeze-test implementation remain **pending**, and **T01
  must still co-freeze the M1 seed values after the owners**. At `/6` integration T01
  must **atomically** update the normative
  [COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md) §4
  wording, the `hash_excludes` semantic description/token (scoped to **post-seed
  runtime declarations**, not the frozen M1 seed), and `FrozenSchema::encode`/
  contract code plus the freeze test, **preserving the frozen `/5` hash and history**.
  This is the user's own `[USER]` scope decision: **not** a row-I closure, **not** a
  `/6` freeze, and **not** a T01 signature beyond that scope decision.

### 22.6 Present status and residual register (must not be read as resolved)

- **Present status:** `/5` (`t01-c01-c06/5`) is **current**; the `/6` contract is
  **unfrozen**; M1 Part A remains **DRAFT — not accepted, not freeze-ready**; there is
  **no code** (no parser, IR, codegen, probe, or pass rate). Every owner bundle
  (T02–T09, T13) is **incomplete** and the **T01 co-freeze remains pending**; every
  owner and T01 integrator sign-off remains **pending**. There is **no Part B
  substrate** (no assembler/linker/probe/runner), and Part B stays out of this
  proposal. The old **marker-family direction is historical/superseded** (rev 49);
  `quota > 1` is **not** the M1 baseline (`quota = 1` is the semantic-comparison
  baseline); and the Part A symbolic model remains **probe-independent**. A/B/C and
  the H1 allocation remain **in principle only**; H6 is a **selected mechanism to be
  frozen in `/6`** (not implemented); H9 remains an **accepted-in-principle removal
  direction pending T01 integrator acceptance** with its residual `[INT]` blockers
  open; D–I remain **PENDING** (row D PENDING overall; from rev 44 the overall pending
  set is C–I). The rev-47 delegated candidate defaults are **not** owner or T01
  `[INT]` signoffs; the rev-48/rev-49 audits are **not** signatures; and the rev-50
  hash-scope decision is the user's **`[USER]` scope decision** only. No `/6` is
  authorized by rev 37; no chip may implement or depend on any item here; and the
  [CDR](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md), ADR-0002, the task
  packages, and the compiler code are **not edited** by this revision.

---

## 23. Rev-38 integration ledger (rev-38 snapshot used CDR rev 51; the rev-39 pointer correction recorded the then-current CDR rev 54 pointing at this proposal rev 39 (expected) — see the rev-40 header pointer for the current values; ADR-0002 Revision 17, updated T02–T09/T13, M1 acceptance)

This section is the point-by-point proposal-side record of the companion
state integrated by rev 38 (**at that snapshot the CDR was rev 51**; the CDR has
since advanced — rev 52 was the read-only cross-owner audit/reconciliation that
**pointed at this proposal rev 38**, and the rev-39 pointer correction recorded the
then-current CDR **rev 54**, which **pointed at this proposal rev 39 (expected)** as
the shared resulting file; see the rev-40 header pointer for the current values — a
pointer correction, no revision advance). It
records **no new user decision**; it does **not**
accept any bundle wholesale, does **not** freeze any interface, does **not**
change `compiler/contracts/CONTRACT_VERSION`, and does **not** authorize any chip
or code. **Status key:** `DELEGATED-CANDIDATE-DEFAULT` = selected by the M1
integration agent under explicit user delegation, **explicitly not** an owner or
T01 `[INT]` signoff and **not** a `/6` freeze; `DOC-ALIGNMENT` = an operative text
alignment with an already-selected candidate, not a new decision and not a
freeze; `READ-ONLY-AUDIT` = a read-only finding recorded as fact, not a signature.
Nothing here is a sign-off; `/5` stays current, the `/6` contract stays
**unfrozen**, M1 stays **DRAFT**, ADR-0002 stays **PROPOSED**, and the T01
integrator plus every owner remain **pending**.

### 23.1 Rev-51 delegated candidate: sole per-tick dispatch-count bound (`DELEGATED-CANDIDATE-DEFAULT`)

- **Rev 51 (CDR rev 51; ADR-0002 Revision 17; T02 rev 31).** Under the user's
  explicit 2026-10-05 delegation, the M1 integration agent selected — **under
  user-delegated integration default, not as an owner or T01 `[INT]` signoff, not a
  schema or `/6` freeze** — that **`max_inflight_per_tick`/quota is the sole
  per-tick dispatch-count bound**, and that the redundant **`max_dispatches_per_tick`
  is dropped from the candidate limit inventory and its validation** (the
  now-duplicate `DispatchBudgetExceeded` dispatcher failure is likewise dropped from
  the candidate inventory). Grounded in the rev-33/34 H9 direction (per-tick dispatch
  is already bounded by `max_inflight_per_tick`/quota, the stage queues, and
  `max_tasks_total`) and the rev-49 audit duplication finding. It **selects no**
  dispatcher-`Ready→Running`/`in_flight` atomic-boundary or clear-order item, changes
  neither the selected H6 mechanism nor the H9 removal direction, and resolves none
  of the residual proof/defaults/fixtures. **Still open:** the exact dispatcher
  mutation boundary; the `in_flight` clear ownership/order relative to the bounded
  H6 recovery; a proof that no `Running`/residual set remains at latch for all
  success/error/empty-proposal paths; the exact limit names/defaults/error-code
  inventories (**T01/T02 `/6` co-freeze**); and the **T02/T13 tests**. `quota = 1`
  remains the baseline equivalence mode; `quota > 1` remains measured and separately
  accepted only.

### 23.2 Delegated `ParseContext` 10-member candidate (`DELEGATED-CANDIDATE-DEFAULT`)

- The T05 package records, under explicit user delegation and **not** as a T05 owner
  signoff or T01 `[INT]` acceptance, the proposed closed vocabulary
  `ParseContext { TranslationUnit, ExternalDecl, Specifier, Declarator,
  ParameterList, Block, Expression, Assignment, Unary, Primary }`. `context` is a
  **grammar-category/entry constraint at the saved cursor**, **semantically distinct**
  from `production: TaskKind` (the registered parser task to resume), and the two are
  **not** derived/elided from each other absent an explicit frozen one-to-one mapping.
  `Expression`/`Assignment` may use `binding_power`. **Open T05/T01 co-freeze:** the
  parser continuation/task→`context` mapping, whether each member is exercised, and
  all canonical discriminants/encoding/unknown-tag errors; the `RecordRef`/`RecordFamily`
  numeric encodings remain separate inventories.

### 23.3 T04 `LiteralRecord.token` delegated candidate (`DELEGATED-CANDIDATE-DEFAULT`)

- The T04 package (rev 46) records the **meaning** of the already-accepted
  `token: Option<TokenId>` field: `token = Some(committed TokenId)` **iff** the
  committed `LiteralRecord` was emitted from a **single committed C token** (the M1
  source literals `2` and `3`), and `token = None` is reserved for an **explicitly
  synthesized literal with no single originating source C token** (a deliberate
  condition, not a "not-yet-resolved" placeholder). A **draft/wire/address is never
  persisted**. Any same-batch token↔literal relation must use the T01-frozen **generic
  typed `RecordLink` / append-reference mechanism** (proposal §3.2/§5), resolved to
  durable committed IDs before persistence — **not** the task-level
  `ContinuationRef::OwnBatch`/`ChildRef::OwnBatch`, which belong to the
  continuation/task-child protocol. **Open T04/T01 co-freeze:** the exact local-link
  spelling/wire-tag/error handling, the synthetic-literal policy beyond M1, the
  source-span relation/error codes, and all numeric encodings. Not an owner/T04 or
  T01 `[INT]` signoff, not a freeze, no code.

### 23.4 T08 opening payload aligned; sem→const→IR order (`DOC-ALIGNMENT`)

- The [T08](T08_CONSTANT_LAYOUT_INIT_CHIPS.md) opening protocol line is aligned to the
  already-selected H1 allocation (CDR rev 50 / proposal rev 36):
  `ConstantRequest { literal: RecordRef::Literal, node, required_kind }` →
  `ConstantResult { value: RecordRef::Const, legality }`. T04 commits the
  `LiteralRecord` in `lex.literals`; the sem-stage per-use `const.evaluate` request
  **references** the committed literal (`RecordRef::Literal`, not re-embedded,
  `Payload` stays `RecordRef`-only); `required_kind` is the M1
  `IntegerConstantExpression` per-use requirement; the `ConstantResult` payload carries
  a committed `RecordRef::Const` result reference (the value lives in the T08-owned
  `ConstRecord`; T08 is the sole `constants.records` writer) plus `legality`
  (`Legal`/`NotConstantExpression`/`Unsupported`, no separate legality family). The
  dependency order is corrected: **T07 sem checks → T08 const → T09 IR** (T07 consumes
  no T08 output; T09 consumes the committed `SemRecord` plus the T08 constant). This is
  an **alignment of operative text with an already-selected candidate, not a new user
  decision and not a freeze**; the exact request/result payload variant spellings, the
  `RecordRef::Literal`/`RecordRef::Const` encoding/tag, the task typing, and all numeric
  codes remain **T01/T07/T08 `/6` co-freeze** items.

### 23.5 M1 vertical acceptance artifact scope corrected (`DOC-ALIGNMENT` + remaining blockers)

- The [M1 vertical acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-PP-08`/G1 scope now
  states the M1 **exercised** artifact-map path is **only** the single-source
  `Normalized` artifact (with a valid `source` and the mandatory `raw_offsets`
  invariants); `Spliced`/`CommentFree`/`Preprocessed` are **schema-declared but not
  produced and not asserted** in this M1 slice, and the **source-provenance/equality**
  relation is **deferred** (rev-47 delegated default: no source-payload-bytes-equality
  requirement). This **resolves an audit-identified document-level mismatch** (the
  earlier text asserted broader producers/equality than the accepted subdecisions
  support); it does **not** claim all artifact blockers resolved. **Remaining open
  T03/T04/T01 blockers:** the exact `raw_offsets` semantics/error mapping and numeric
  codes, the token↔literal linkage (provenance carrier / verifiable PP-span link), the
  artifact error classification, and the `Lx08CandidateType`/`LiteralKind`/
  `LiteralSuffix`/radix **candidate closed set**. Not a freeze, no code.

### 23.6 M1 target acceptance rev 29: `FunctionEnd` result + phase-2b hook + two-tier hash (`DOC-ALIGNMENT`)

- The [M1 target acceptance](M1_TARGET_ACCEPTANCE.md) rev 29 aligns its operative text
  with CDR rev 49/50 and the T09 operative amendments: the `TerminatorMissing` trigger
  reuses the **committed IR28 `FunctionEnd` terminal result**
  (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic
  completion fact, checked by a **T01-owned typed phase-2b commit-apply validation
  hook** for entry-block termination — **no** new marker record family/ID/arena/
  `RecordRef` tag/`RecordFamily` ordinal/snapshot encoder/`ResultValue` variant. It also
  records the rev-50 **two-tier `/6` hash boundary accepted conceptually** (frozen seed
  `foundation + M1AppendSchema` hashed; post-seed runtime `declare()` declarations
  excluded) with **no `/6` hash/seed values or freeze**. The exact hook ordering/typing/
  hash treatment and T09/T01 co-freeze remain **open**; the T13 fixture is **pending**.
  **Part B remains unavailable** (no assembler/linker/probe/runner), and no target
  acceptance is passed.

### 23.7 T01 readiness audit — concise ordered freeze-blocker/dependency checklist

- This is a **summary**, not the whole audit. Ordered so each item blocks the next
  freeze step; all are **open**:
  1. **Owner-signed schemas/payloads/manifests** — each owner's exact field/enum/tag
     shapes and manifest rows signed off (T02–T09, T13; overall bundles still PENDING).
  2. **Families/tags/ordinals/canonical encoder** — pin the wire-tag inventory
     (`RecordRef` tags) **separately** from `RecordFamily` ordinals; define the
     canonical encoder and dual-inventory encoding; `/5` code facts are **24**/24
     (tags 0–23) with `RecordFamily` **absent**, and the 19/27/27 draft counts remain
     **proposed/unverified**.
  3. **T05 TU/context** — the committed-`TranslationUnit` carrier and the
     `ParseContext` final mapping/encoding (item 23.2) remain open.
  4. **T06 lifecycle/allowlist** — the `Identifier`-leaf `decl` lifecycle, the
     scope-event exact lifecycle/encoding/allowlist rows, and the `ChipId`-keyed
     allowlist seed/hash remain open.
  5. **T07 `SemRecord`/VF06** — the exact `SemRecord` field encoding, the full
     conversion matrix, and the VF06 registration (task kind/stage/phase/interface)
     remain open.
  6. **T08 const** — the exact `ConstantRequest`/`ConstantResult` payload spellings,
     `RecordRef` encoding/tag, task typing, the `max_const_bits` projection field/
     diagnostic code/hash encoding, and the signed-range overflow formula remain open.
  7. **T09 hook** — the phase-2b commit-apply validation hook's exact typed
     contract/details and its `/6` hash treatment remain open.
  8. **Pipeline H6/H9** — the exact dispatcher mutation boundary, the `in_flight`
     clear ownership/order relative to the H6 recovery, the all-paths no-`Running`/
     residual proof, and the limit/stage/hash/error inventories remain open (23.1).
  9. **Hash/snapshot tests** — the `M1AppendSchema` seed-equality/self-consistency
     test, the numeric-value inclusion in the hash, and the snapshot/freeze tests
     remain to be implemented after the owners accept their shapes (rev-50 two-tier
     scope accepted **conceptually only**).
  No item above is a signoff; nothing is frozen; T01 still co-freezes the M1 seed
  values after the owners.

### 23.8 Present status and residual register (must not be read as resolved)

- **Present status:** `/5` (`t01-c01-c06/5`) is **current**; `/6` is **unfrozen**; M1
  Part A is **DRAFT — not accepted, not freeze-ready**; there is **no code**; there is
  **no Part B substrate**. Every owner bundle (T02–T09, T13) is **incomplete** and the
  **T01 co-freeze remains pending**. The rev-50 two-tier hash scope and the rev-49
  H11/T09/T01 direction remain **current**; the rev-51 delegated candidate default
  (23.1) and the rev-38 delegated `ParseContext`/`LiteralRecord.token` candidates
  (23.2/23.3) are **not** owner/T01 `[INT]` signoffs and freeze nothing. At this
  snapshot the CDR is at **rev 51**, ADR-0002 at **Revision 17**, T02 at **rev 31**,
  the M1 target acceptance at **rev 29**, and the M1 vertical acceptance at **rev 27**.
  **Pointer correction (same rev 38; historical — see the rev-40 header pointer for the current values):** that **rev-51** value was
  the CDR state at the rev-38 integration snapshot; the then-current CDR was **rev 54**,
  which **pointed at M1 proposal rev 39 (expected)** (the rev-52 read-only cross-owner
  audit/reconciliation pointed at rev 38; the later CDR advances have no matching
  proposal revision). No `/6` is authorized by rev 38; no chip may
  implement or depend on any item here;
  and the CDR, ADR-0002, the task packages, and the compiler code are **not edited** by
  this revision.

---

## 24. Rev-39 integration ledger (rev-39 snapshot: CDR rev 53; current companion as of 2026-10-05: CDR rev 55, T02 rev 35, T03 rev 56, T04 rev 47 + task revisions 1–3, T13 rev 11, M1 vertical rev 34; ADR-0002 Revision 17; M1 target rev 37)

This section is the point-by-point proposal-side record of the companion state
integrated by rev 39. The **current CDR is rev 55** (which points at this proposal
**rev 40 (current)** as its shared resulting file; values as of 2026-10-05). It records
**no new user decision**; it does **not** accept any bundle wholesale, does **not**
freeze any interface, does **not** change `compiler/contracts/CONTRACT_VERSION`, and
does **not** authorize any chip or code. **Status key:**
`DELEGATED-CANDIDATE-DEFAULT` = selected by the M1 integration agent under explicit
user delegation, **explicitly not** an owner or T01 `[INT]` signoff and **not** a
`/6` freeze; `DOC-ALIGNMENT` = an operative text alignment with an already-selected
candidate, not a new decision and not a freeze; `READ-ONLY-AUDIT` = a read-only
finding recorded as fact, not a signature; `OPEN-BLOCKER` = an unresolved
`/6`/owner co-freeze item recorded explicitly, not resolved. Nothing here is a
sign-off; `/5` stays current, the `/6` contract stays **unfrozen**, M1 stays
**DRAFT**, ADR-0002 stays **PROPOSED**, and the T01 integrator plus every owner
remain **pending**.

### 24.1 CDR rev 53 / T04 rev 47: `Lx08CandidateType` M1 vocabulary `{ Int }` (`DELEGATED-CANDIDATE-DEFAULT`)

- **CDR rev 53 (T04 rev 47; T08 task-package revision 2 cross-reference).** Under the
  user's explicit 2026-10-05 delegation, the M1 integration agent selected — **under
  user-delegated integration default, not as an owner/T04 or T01 `[INT]` signoff** —
  that for the **exercised M1 integer literal subset** the symbolic
  `Lx08CandidateType` vocabulary is the **closed one-member set `{ Int }`**, with M1
  literals `2`/`3` represented as **target-independent `Int` with no bit width** (a
  symbolic candidate value, **not** a target type, ABI type, or `TypeId`, and
  duplicating no implicit target type). Literal forms **outside** the exercised
  subset (non-decimal radix, non-`None` suffix, character/string literals, or any
  not-yet-specified category) **must not silently default to `Int`** and are
  classified as an **explicit unsupported/deferred** result/diagnostic until their
  categories/rules are specified; future C candidate support **appends symbolic
  members/rules without reinterpreting** the selected `Int` member or repurposing
  `Lx08CandidateType`. This refines only the **meaning** of the already-selected
  `candidate_type`; it changes **no** field order/type/count, is **not** the complete
  future C candidate vocabulary, defines **no** numeric tag/encoding, and is **not** a
  `/6` freeze. **Still open:** the complete member set and all numeric encodings
  (`T04/T08/T01` `/6` co-freeze).

### 24.2 T03 rev 53–54: `raw_offsets` boundary map + M1 PP01 contract candidate (`DELEGATED-CANDIDATE-DEFAULT` + `OPEN-BLOCKER`)

- **T03 rev 53.** Under the same delegation, the delegated candidate default for the
  M1 **single-source `Normalized`** artifact's `raw_offsets` is recorded: `raw_offsets[i]`
  is the **raw-source boundary for output boundary `i`** (output `[a,b)` maps to raw
  `[raw_offsets[a], raw_offsets[b])`; retained bytes map corresponding boundaries;
  `CRLF`→`LF` maps the output `LF` start to the raw `CR` start and the end to after
  the raw `LF`; an inserted terminal `LF` maps both boundaries to raw `EOF`
  (zero-width); identity/final-`LF`/`CRLF`/whitespace M1 cases deterministic). This is
  a **primary location map, not complete provenance** for deleted splice ranges / macro
  / paste / multi-source (deferred). **Open:** exact error behavior/codes and any
  general transform policy (`T03/T01` co-freeze).
- **T03 rev 54.** A compact **M1 PP01 producer/consumer contract candidate** for
  single-source `Normalized` input only: exact source input (one live
  `RecordRef::Source`, source-scoped one-source limit), normalization outputs
  (identity / inserted terminal `LF` / `CRLF`→`LF` / whitespace variant),
  `ArtifactRecord` publication under the accepted rev-44 shape + rev-45 mandatory-map
  invariants + rev-53 `raw_offsets` boundary convention, PP-token/span interactions
  (PP04 maximal munch; T03-only `sources.spans`/`sources.expansions`; raw remap before
  `SpanDraft`; T04 reuses the committed PP span), the `source_scoped_one_hop`
  source-provenance rule with equality **deferred** (no source-payload-bytes equality
  per the rev-47 delegated default), a delegated `PpRequest`/`PpResult` typing/timing
  candidate, a field-scoped read/write manifest, and the remaining co-freeze blockers.
  It **does not** extend M1 to splice/comment/macro/multi-source.
- **Open:** the artifact diagnostics/classification and numeric codes, the exact
  `PpRequest`/`PpResult` variants, the `TaskKind` local codes, the ref/provenance
  carrier, and the hash inventory (`T03/T01` co-freeze). Not a T03/T01 signoff and not
  a `/6` freeze.

### 24.3 T05/T06 committed-`TranslationUnit` carrier + exactly-once still open (`DELEGATED-CANDIDATE-DEFAULT` + `OPEN-BLOCKER`)

- **TU carrier (T05 item F; T06 item 5; integration-selected candidate default under
  the same delegation).** The committed TU `NodeId` (`root_node_id`) is supplied by the
  **existing generic `ResultValue::Record(RecordRef::Node(root_node_id))`** payload of
  the committed `TaskState::Completed(ResultId)` of the registered
  `parse.TranslationUnit` task — **no** new typed `TranslationUnit` result
  variant/family, **no** new `RecordValue`/`RecordRef`/`RecordFamily` family, and
  **no** task-kind-only derivation. Only after **both** the committed `ResultId` and
  the referenced committed `NodeId` are **commit-visible** does the deterministic
  `parse.TranslationUnit -> symbol_type.scope-enter` stage edge enqueue the file-scope
  `Enter`, carrying that committed `NodeId`.
- **Exactly-once claim still open (`OPEN-BLOCKER`).** The candidate explicitly states
  that the committed `TaskState::Completed(ResultId)` fact is **not sufficient** to
  drive the edge: `ResultId` is a durable handle, not a consumed-delivery claim, and
  matching on task kind or completion status alone cannot distinguish an
  already-delivered completion from a fresh one. The stage edge is therefore stated as
  a **result-consumption** condition over the registered TU task's own successful,
  **still-unconsumed** `ResultRecord` whose value is the generic
  `ResultValue::Record(RecordRef::Node(root_node_id))`; consumption + enqueue must be a
  **single commit-visible unit**, firing exactly once per result. If the `/6` frozen
  commit machinery **cannot** atomically combine result consumption with task enqueue,
  **T01 must define an equivalent persistent delivery claim** (a durable
  consumed/delivered marker or equivalent) so exactly-once delivery is still enforced;
  this document invents and freezes no such mechanism. **Open (`T05/T01/T06`
  co-freeze):** the exact `TaskKind` identity/registration, the exact `ResultValue`
  typing, duplicate-edge suppression, the precise trigger/commit order, the exact
  consume/enqueue envelope, the retry/error recovery of the stage edge, and the T06
  enqueue-payload task typing. Not a T05/T06/T01 signoff and not a `/6` freeze.
- **OPEN-01 cross-reference (§24.14; candidate only).** The §24.14 early TU-root
  commit-order candidate would trigger this edge on the **committed root append**
  (in the parse task's first batch) rather than on terminal task completion, and
  would make exactly-once **structural** for this edge (one root append per TU),
  so the `ResultRecord.consumed` claim would not be required for the File-Enter
  edge (it would remain available for other result hand-offs). This entry's
  completion-trigger carrier remains the current candidate until §24.14 is
  accepted; OB-11/OB-50 stay open either way.

### 24.4 T07/T08/T09/T13 newest candidate sections (`DOC-ALIGNMENT` + `DELEGATED-CANDIDATE-DEFAULT`)

- **T07 (rev 1–3).** The `SemRecord` field-set **candidate checklist** (S1–S7; each row
  `candidate` or `OPEN`) assembled from the accepted subdecisions; the selected
  **sem → const → ir** stage order (T07 produces the per-use `ConstantRequest`; T08
  folds; T09 consumes the committed `SemRecord` **plus** the T08 constant; T07 reads
  no T08-produced `Const` fact on this path, so there is no `T07→T08→T07` cycle); the
  M1-only constant-context metadata and the identity/no-conversion M1 scope. **Open:**
  the exact `SemRecord` field encoding, the complete conversion matrix, the VF06
  registration, and all numeric encodings.
- **T08 (task-package revisions 1–3; rev-44/45 rows).** The operative H1-aligned
  opening payload `ConstantRequest { literal: RecordRef::Literal, node, required_kind }`
  → `ConstantResult { value: RecordRef::Const, legality }`; the **M1-only constant
  evaluator candidate** for `2`/`3`/folded `2+3`; `max_const_bits` accepted
  source/projection/bound; and the **`§G2` formula conflict left unselected** (24.7).
- **T09 (rev 47/48).** The **M1-only IR-shape checklist** (one-function/entry-block/
  `Constant(int5)`/`Return`/`FunctionEnd`-completion-result; candidate
  `FunctionRecord`/`BlockRecord`/`ValueRecord`/`InstructionRecord`; `Constant`/`Return`/
  `Add` op-table entries; the IR28 `FunctionEnd` terminal-result completion fact with
  the T01-owned typed phase-2b hook and **no** marker family/ID/tag/ordinal/encoder/
  `ResultValue` variant); the delegated rev-47 rule defaults (shorter
  `ir.op-immediate-type`/`ir.terminator-missing` aliases; immediate-before-result
  validation; expected type = result `ValueRecord.ty`). **Open:** FunctionEnd result
  typing, commit-hook validation order/error, IR record fields, task/result typing,
  writer manifest, hash/rules/fixtures.
- **T13 (rev 3/4/5/42/46).** The M1 verification fixture/registration checklist; the
  **explicit H6/H9 verification matrix** (H6-M01..H6-M16: latch zero-`Running` +
  empty-`in_flight` on success/failure, pre-dispatch `Ready`, empty-proposal vector,
  `StorePatch`-only batch, quota-1 path, semantic-hash-unchanged-before-recovery,
  exactly-one-outcome-in-dispatch-order, diagnostic-vs-sentinel order, reset/wire
  lifetime "clearing in-flight is not itself a transition", whole-batch no-mutation,
  quota-1 projection vs scheduler snapshot) — every row is a **required pending
  fixture**, **not** implemented/defined/signoff and **not** a proof (24.8); plus the
  VF06-in-M1 role (registration open) and the VF04 batch write-conflict audit
  **proposed/residual, not defined today**. All candidate/prospective `/6`; none
  frozen.

### 24.5 T02 rev 32: H6/H9 remaining co-freeze contract checklist (`DOC-ALIGNMENT` + `OPEN-BLOCKER`)

- **T02 rev 32.** A compact, T02-owned **remaining co-freeze contract checklist**
  converts each open H6/H9 point recorded in the T02 package into an **exact question
  plus a testable invariant**, so T01/T02 can co-freeze with the smallest sufficient
  question set: **H6.1** pre-dispatch error path (affected tasks remain `Ready`, no
  store mutation, structured tick diagnostic not a `CommitError`); **H6.2** semantic
  batch-commit failure (semantic hash equals pre-tick value; recovery in dispatch
  order); **H6.3** exactly-once `Failed` + state guard (count of `Failed` transitions
  per dispatched task exactly 1); **H6.4** diagnostic capacity + `DiagnosticId::NONE`
  (no task left `Running` or stranded); **H6.5** empty proposal vector (exactly one
  terminal outcome, never `Running`); **H6.6** `/5` single-task obligation preserved;
  **H9.1** numeric limit inventory (`max_inflight_per_tick` default 1, stable codes);
  **H9.2** `stage_queue_bound` preflight (checked before mutation); **H9.3**
  `max_proposals_per_tick` whole-batch; **H9.4** stage set/names (`StageUnassigned`/
  `StageLayerMismatch`). It **does not decide** the four T01 architecture-critical
  dispatcher/commit items (`in_flight` clear ownership/order; failure-recovery
  mutation primitive; no-residual/no-`Running` guarantee; the dispatcher
  `Ready→Running`-vs-`in_flight`-vs-single-atomic-commit boundary); for those it
  presents only **viable alternatives and required properties**. Exact stage
  set/names and all numeric error codes remain **open**. All checklist material is
  **proposed, not signoff, not freeze**.

### 24.6 T04 reciprocal token↔literal `RecordLink` cycle — pre-reserved IDs required (`READ-ONLY-AUDIT` + `OPEN-BLOCKER`)

- **Finding.** The T04 same-batch token↔literal relation must be expressed through the
  T01-frozen **typed `RecordLink` / append-reference** mechanism
  (`RecordLink { expect: RecordFamily, target: LinkTarget }`,
  `LinkTarget = Committed(RecordRef) | Draft(DraftRef)`; `RecordDraft::links()`), **not**
  a raw draft index/handle and **not** the task-level
  `ContinuationRef::OwnBatch`/`ChildRef::OwnBatch` (which belong to the
  continuation/task-child protocol). A read-only audit notes that the `TokenRecord`
  ↔`LiteralRecord` pair is **reciprocal/mutual**: `TokenRecord.literal` references a
  `LiteralRecord`, and a `LiteralRecord` produced from a token references exactly that
  token, so a **directly reciprocal `RecordLink` cycle** between the two drafts is
  possible. Resolving such a cycle requires the **arena IDs for both records to be
  pre-reserved (allocated) before either link is resolved**, so that each link resolves
  to a durable committed ID rather than to the other's unresolved draft. The current
  `RecordDraft::links()` candidate resolves links against the batch draft table in
  phase 2 (`resolved_table[(task,i)] = (family, ref)`); the audit records that
  **pre-reservation of the predicted IDs must therefore precede link resolution** for
  a reciprocal pair, and that the exact mechanism (ID pre-reservation order, the
  reciprocal-link check, and how the cycle is broken/witnessed) has **not** been
  designed. **Status:** `OPEN-BLOCKER` — an **open T04/T01 `/6` co-freeze** item; no
  mechanism is invented here, and the T04 package records the local-link spelling,
  wire-tag, consistency predicate, and error handling as still open.

### 24.7 T08 `§G2` signed-range overflow formula conflict — unselected (`READ-ONLY-AUDIT` + `OPEN-BLOCKER`)

- **Finding.** The CDR `§G2` signed-range overflow formula
  `-(2^(max_const_bits-1)) <= v <= 2^(max_const_bits-1)-1` was, in an earlier draft,
  phrased as if it were an operative selected step. The T08 package (task-package
  revision 3, with a **same-revision correction note and no new revision**) recorded
  that phrasing as a **wording error**; that same-revision note is preserved as
  history, but its own `max_const_bits` sentence — that the exercised M1 results
  (`2`, `3`, `5`) are **representable for every accepted valid `max_const_bits`
  value** (any cap `<= 128`, including the default `128`) — was itself overbroad
  and is **superseded** (review
  [DOC-14](../reviews/2026-10-05_DOCUMENTATION_REVIEW.md); T08 task-package
  revision 4). At a budget of `1` the magnitude `2` already needs two bits and `5`
  needs three, before any sign representation, so those values cannot be
  representable there. **Corrected position (DOC-14; no acceptance change, no
  freeze, no code):** M1 **success acceptance is limited to a sufficient
  budget** (e.g. the default `128`); an **insufficient budget** must produce a
  **typed overflow/capacity result** — the accepted `ConstOverflow`
  chip-diagnostic classification, whose exact trigger formula remains open —
  **not** a `Legal` success or a committed `ConstRecord`. The configuration
  range and minimum are **unchanged**: valid values `<= 128` remain valid and
  **no minimum is raised**. The candidate settles **neither** the general overflow
  bound formula, **nor** the signed/unsigned representation, **nor** the
  representation carrier, **nor** the chip-vs-commit enforcement. The `§G2` formula
  and the `ConstRecord { ty, value: i128 }` carrier are retained **only as unselected
  draft proposals/references, not chosen rules**. The **signed-representative wording
  for unsigned magnitudes** also remains open. **Status:** `OPEN-BLOCKER` — the
  formula/enforcement split, symbolic-vs-probe gating, and representation remain open
  `[OWNER:T08]`/`[INT]` items; `ConstOverflow`/`ConstUnsupported` remain chip
  diagnostics.

### 24.8 M1 `PP-08` exact-map/failure semantics; H9 clearing conflict and T13 no-residual evidence (`READ-ONLY-AUDIT` + `OPEN-BLOCKER`)

- **M1 `PP-08` exact-map/failure semantics.** The [M1 vertical acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md)
  `M1-PP-08` (rev 27) identifies the **single-source `Normalized`** artifact as the only
  M1 **exercised** artifact-map path: it asserts a **valid `source`** and the rev-45
  mandatory-map invariants (`raw_offsets.len() == bytes.len()+1`, first `== 0`,
  monotonic nondecreasing, last `<= source length`) and requires **missing-source/bad-map
  rejected before mutation**; `Spliced`/`CommentFree`/`Preprocessed` are
  schema-declared but **unexercised** (and `PP08 MacroUndefChip` carries an explicit
  unexercised gap gate, never a pass), and the source-provenance/equality relation is
  **deferred** (rev-47 delegated default: no source-payload-bytes-equality requirement).
  **Open:** the exact `raw_offsets` error mapping and **failure classification/numeric
  codes**, the token↔literal linkage, and the `Lx08CandidateType`/`LiteralKind`/
  `LiteralSuffix`/radix candidate closed set (`T03/T04/T01` co-freeze). The
  document-level mismatch is **resolved at the document level**, but the audit records
  that not all artifact blockers are resolved.
- **H9 clearing conflict.** The H9 removal direction (removing `max_inflight_total`) is
  accepted in principle pending T01 integrator acceptance, and the rev-31/rev-32
  candidate narrows the limit inventory to the **sole** `max_inflight_per_tick`/quota
  bound (with `max_dispatches_per_tick` dropped). A read-only audit records a **clearing
  conflict still open**: the exact `in_flight` clear ownership/order relative to the
  bounded H6 recovery is undefined, and the statement "clearing the in-flight set is
  **not itself** a transition" (and does not clear `TaskState`/`Running`) must be
  reconciled with the selected **exactly-once `Failed`** recovery so that clearing
  neither strands a task nor double-transitions one. The dispatcher
  `Ready→Running`/`in_flight`-population relationship to the single ordered atomic
  commit also remains open. **Status:** `OPEN-BLOCKER` (`T01`/`T02` co-freeze).
- **T13 no-residual evidence gap.** The T13 H6/H9 verification matrix rows
  (H6-M01..H6-M16 as of T13 rev 11) are **required pending fixtures**, **not implemented, not defined,
  not signoff**, and **not** a proof: the all-paths no-`Running`/no-residual guarantee
  (success, semantic batch failure, pre-dispatch error, and empty proposal vector) is
  **conditional on the H6 recovery realization** and would be **verified, not
  asserted**; the H9 numeric inventories and fixture encodings/registration remain
  **UNRESOLVED (`T01`/`T02`/`T13`)**. The audit therefore records that the no-residual
  claim currently has **no test evidence** and must not be read as established.

### 24.9 Present status and residual register (must not be read as resolved)

- **Present status:** `/5` (`t01-c01-c06/5`) is **current**; `/6` is **unfrozen**; M1
  Part A is **DRAFT — not accepted, not freeze-ready**; there is **no code**; there is
  **no Part B substrate**. Every owner bundle (T02–T09, T13) is **incomplete** and the
  **T01 co-freeze remains pending**. The rev-50 two-tier hash scope, the rev-49
  H11/T09/T01 direction, and the rev-51 sole-per-tick-bound delegated candidate default
  remain **current**; the rev-53 delegated `Lx08CandidateType { Int }` default (24.1)
  and all newest T03/T05/T06/T07/T08/T09/T13 candidate sections (24.2–24.4) are **not**
  owner/T01 `[INT]` signoffs and freeze nothing. The **current CDR is rev 55** (which
  points at this proposal **rev 40 (current)** as its shared resulting file),
  ADR-0002 is at **Revision 17**, T02 at **rev 35**, T03 at **rev 56**, T04 at
  **rev 47 + task revisions 1–3**, T13 at **rev 11** (H6-M01..H6-M16 matrix), the M1 target acceptance at
  **rev 37**, and the M1 vertical acceptance at **rev 34** (values as of 2026-10-05; each
  document's own revision record is authoritative). No CDR revision beyond
  **rev 55** exists at this writing; the CDR's next revision is **not contemporaneous**
  and is not named here. No `/6` is authorized
  by rev 39; no chip may implement or depend on any item here; and the CDR, ADR-0002,
  the task packages, and the compiler code are **not edited** by this revision.

### 24.10 Constant-result carrier / legality coupling (post-rev-39 audit; binary handoff resolved by rev 40; remaining `OPEN-BLOCKER`, `T01`/`T07`/`T08`/`T09`)

- **OPEN-03 binary request + operand/operator handoff + T08→T09 delivery — RESOLVED at the document level by rev 40 (T01/T07/T08 co-freeze, user-directed 2026-10-05; OB-9).** The `2+3` input path is frozen: `ConstantRequest::Binary { node, op: ConstExprOp, lhs, rhs, required_kind }` with the committed `BinaryExpression` `NodeId`, T07's checked `Add` operator, and the committed T04 `LiteralRecord` refs of the operands; T08 commits **one** `ConstRecord` (`ty` = symbolic `int`, `value` = `+5`); T09 consumes the same committed `RecordRef::Const` and does not re-fold. See §5 and the T07/T08/T09/M1-vertical revisions.
- **Remaining open (not resolved by rev 40; `OPEN-BLOCKER`, `T01`/`T07`/`T08`/`T09`).** `ConstantResult` still has **no defined `ResultValue`/envelope carrier** (new variant vs existing-variant encoding is a T01 shared-interface decision and is where `legality` lives); it is undecided whether `value` is always present or optional for non-legal legality, whether non-legal outcomes commit any `ConstRecord`, and how T09 must refuse a non-legal constant; `ConstRecord` identity/reuse is unstated. Also logged in T07 §2. No carrier is invented here.

### 24.11 Open-blocker summary table (post-rev-39 audit; all `OPEN`, none frozen)

This table consolidates the still-open `/6` co-freeze blockers surfaced by the
post-rev-39 read-only audits. Every row is **`OPEN`** — no row is a decision, a
signoff, or a freeze — except the rows explicitly marked resolved by the
**rev-40 OPEN-03 co-freeze** (OB-9; OB-25's delivery carrier); each names its
owner/co-freeze set. It changes no accepted
subdecision. **Extent:** the first batch (OB-1..OB-17) came from the initial
post-rev-39 audits; the second batch (OB-18..OB-34) came from the follow-up
T01-§4/T03/T04-T05/T06-T07/T08-T09/limits/snapshot/acceptance audits, each
verified against the current working tree. Owner cells reflect the audited
co-freeze sets and may be more precise than the §9C matrix's row-level owner lists
(the §9C matrix remains the authority for signatures). The third batch
(OB-35..OB-38) came from the Host/artifact-finalize, T11 Part-B, T00-gate,
cross-reference, and guardrails audits. The fourth batch (OB-39..OB-52) came from
the post-rev-39 Host/bootstrap, M1AppendSchema, metrics, acceptance-traceability,
kind-inventory, `/5`-contract, relocation-protocol, package-status,
conversion-scope, and pointer audits. The fifth batch (OB-53) records the
documentation-review OPEN-01 TU-root/scope startup-order candidate (§24.14;
still `OPEN`). See §24.15 for the Group A (scheduler/commit boundary)
user-accepted recommendations (2026-10-05), which dispose OB-1/2/3/5/26/28/29/39/40/41/44 as recommendations only.
See §24.16 for the Group B (result/diagnostic carriers) user-accepted
recommendations (2026-10-05), which dispose OB-4/6/7/8/27/30/31/50 as
recommendations only. See §24.17 for the Group C (records/links/schema/hash)
user-accepted recommendations (2026-10-05), which dispose
OB-10/12/13/14/15/19/20/21/22/23/24/32/34/42/43/48/49/52 as recommendations only.
See §24.18 for the Group D/E/F (semantics/host/acceptance/limits) user-accepted
recommendations (2026-10-05), which dispose
OB-11/16/33/35/36/37/38/45/51/52/53 as recommendations only (OB-52 narrows the
§24.17 hygiene entry; OB-11 follows the Group B OB-50 direction).
See §24.19 for the OB-17/OB-18 user-accepted recommendations (2026-10-05),
which dispose OB-17/OB-18 as recommendations only.

| # | Blocker | Owner / co-freeze | Where |
|---|---|---|---|
| OB-1 | Scheduler `Ready→Running` vs the one ordered atomic commit; sole `in_flight` clear owner/order; all-paths no-`Running`/no-residual proof | T01 `[INT]` (+T02/T13) | §24.8; T02 T01-critical items 1/2/4; ADR §2/§6 |
| OB-2 | Empty-proposal outcome rule for a dispatched task | T01 `[INT]` (+T02) | T02 H6.5; T13 H6-M04 |
| OB-3 | Cancel / tick-budget task disposition and latch assertion; cancel-vs-budget precedence | T01 `[INT]` (+T13) | T13 H6-M13/M14 |
| OB-4 | Join resume exactly-once + result delivery (OPEN-02: no consume at join without a durable parent-side handoff; consumption stays with the parent's own atomic commit or the results transfer to parent-owned durable state; `join_then_replay_retry` required) + consume/enqueue atomic envelope; fate of non-terminal siblings | T01/T05/T02 | T05 item F; T13 H6-M15; proposal §5/§7/§13/§18.3 |
| OB-5 | `Progress` reinsert exactly-once and `max_task_progress` exceedance outcome (per-task `Fail` vs batch reject) | T02/T01 | T13 H6-M16 |
| OB-6 | `ConstantResult` `ResultValue`/envelope carrier and where `legality` lives | T01/T07/T08 | §24.10; T07 §2 |
| OB-7 | Non-legal legality value coupling (`value` optional? commit a `ConstRecord`? T09 refusal rule) | T07/T08/T09 | §24.10; T07 §2 |
| OB-8 | `ConstRecord` identity/reuse invariant and possible `literal`/`node` back-link | T07/T08 | §24.10 |
| OB-9 | **RESOLVED by proposal rev 40 (OPEN-03 co-freeze, 2026-10-05):** the frozen `ConstantRequest::Binary { node, op: ConstExprOp, lhs, rhs, required_kind }` carries the committed expression/operator/operand inputs; T08 commits one `ConstRecord`; T09 consumes the committed `RecordRef::Const` (same `ConstId`). Exact wire spellings/tags remain `/6`. | T07/T08/T09 | §24.10; §5 carrier block |
| OB-10 | T04 reciprocal token↔literal same-batch link needs whole-batch ID pre-reservation before link resolution | T04/T01 | T04 task revision 2; §24.6 |
| OB-11 | T05/T06 TU `parse.TranslationUnit → symbol_type.scope-enter` exactly-once atomic consume+enqueue (or T01-equivalent persistent claim) | T01/T05/T06 | T05 item F; T06 item 7; §24.3 |
| OB-12 | Per-chip field-level read/write manifests + sole-chip writers for every `/6` append family | T01 + T03–T09 owners | §12.9; each task package |
| OB-13 | `/6` counts/inventories (families/refs/ordinals/stores) and store↔family↔arena mapping order | T01 `[INT]` | §12.1–12.2; §8 |
| OB-14 | Canonical encoder + numeric-value hash + typed-body snapshot for the M1 seed; two-tier `foundation + M1AppendSchema` boundary implementation | T01 `[INT]` | §12.12/§12.15; CDR row I |
| OB-15 | `SpanRecord.start/end` `u32→u64` and `encode_span` u64 snapshot encoding (version/compat) | T01 + T03 | §12.17; CDR §3.2 |
| OB-16 | Limits validation inventory (`max_inflight_per_tick`/`stage_queue_bound`/`max_const_bits`/`max_task_progress` names/defaults/codes; `try_new`) | T01/T02/T08 | §12.8/§12.18 |
| OB-17 | ADR-0002 H6 status reconciliation and `DispatchBudgetExceeded` removal propagated everywhere operative | T01/integrator | ADR-0002; CDR §A9; M1 §7 |
| OB-18 | T01 §4 `/6` supersession catalog incomplete: `Fault` representation, queue membership (`Waiting` not queued; `Progress` reinsert), latch/in-flight clear (ADR §2.4 step 1 now records the tick-start-clear supersession; the clear owner/order remains open), and the §I4 catalog missing the ordering-rule extension, five-outcome set, Proposal additions, continuation fields, Progress/latch items; §3-vs-§4 mis-attribution | T01 `[INT]` | CDR §I4; T01 §4; ADR §2.4 |
| OB-19 | `SemRecord` backing decision (`SemId`+`sem.records`+`SemDraft`+`RecordRef::Sem` vs `NodeId`-keyed family-less) — gates the 27-variant/tag inventory | T07/T01 | T07 §1 S1/S7; proposal §5/§8 |
| OB-20 | `SemRecord` field set/placement/order and the typed T09 link carrier; `TypeId` reuse "matching" predicate and scan bound; scope-event encoding and `symbols.scope_events` allowlist rows | T06/T07/T01 | T07 §1/§4; T06 items 1/4/6 |
| OB-21 | Scope-event duplicate-rule wording (committed vs committed-or-same-batch) and file-Exit violation placement | T06 | T06 item 6b; proposal §5 |
| OB-22 | `ConstantResult` carrier/legality and `TypeId`/constant enum numeric encodings already listed as OB-6..OB-9; add: `ConstRecord` value carrier `i128`-vs-other and formula/enforcement/symbolic-vs-probe split | T08/T01 | CDR §G2; T08 rev3 |
| OB-23 | VF06 completeness domain: whether non-M1 required conversions are VF06 failures or unsupported diagnostics; and VF06 depends on the unresolved `SemRecord` shape | T07/T13/T01 | T13 VF06 checklist items 3/4; CDR §F3 |
| OB-24 | T03 PP01 candidate unresolved: request/result payload variant+plural encoding, chunk-state bus field, commit grouping, per-chip manifest split, same-batch span-link vs committed span, artifact/limit failure classification, `max_source_bytes` scope, artifact-rejection atomicity, provenance carriers | T03/T01 | T03 rev54; proposal OB-12 |
| OB-25 | **Delivery carrier RESOLVED by proposal rev 40 (OPEN-03 co-freeze):** the T08→T09 handoff carries the committed `RecordRef::Const` (same `ConstId`), not a bare projected index. Remaining open: IR03 emitter attribution and the IR03 symbolic-vs-bit-pattern scope. | T08/T09/T01 | T09 M1-only IR shape checklist; CDR §7; §24.10 |
| OB-26 | `FunctionEnd` phase-2b hook rejection vs H6: task-scoped failure vs whole-batch `TerminatorMissing` rejection, and how the "already-committed" completion fact can exist if rejection rolls back the batch | T09/T01 | T09 FunctionEnd; OB-1 |
| OB-27 | `EffectMaskUnsupported` carrier: `CommitError` (proposal §6.4) vs chip `DiagnosticDraft` (CDR rev 46) vs VF06 failure (T13) — exactly one must be chosen | T07/T13/T01 | proposal §6.4; CDR §F; T13 VF06 checklist item 5 |
| OB-28 | "phase 2b" label collision across `ContinuationRef`/`ChildRef` validation, predicted-ID reservation, and cumulative semantic validation; per-task collection pass to know own `AppendRecords`/`Enqueue` sets | T05/T01 | T05 item C; proposal §7 |
| OB-29 | `ChildRef::Committed` "valid committed child of this task" predicate and its error name undefined | T05/T02 | T05 item C |
| OB-30 | `ParseResult.next_cursor` carrier vs `ResultValue` closed set (new variant vs `ContinuationDraft.cursor`); parse request/result/`NodeKind` inventory and `ParseContext`→PA mapping | T05/T01 | T05 items G/H |
| OB-31 | T01 §4 `operands` requirement vs the accepted 9-field `ContinuationRecord`; sibling/await-all stale-open text in the proposal | T05/T01 | T01 §4; proposal §5 |
| OB-32 | Per-chip field-level manifests missing for `LX01–LX18` and `PA01–PA38`; `names.entries` append semantics; `DiagnosticDraft` vs `DiagnosticProposal` naming | T04/T05/T01 | T04 manifest/co-freeze checklist; T05; T01 §5 |
| OB-33 | Limits validation inventory stale lists (`max_dispatches_per_tick` still in §12.8/§12.18), per-limit validation predicates/`ConfigError` codes, `try_new`, and checked arithmetic on `proposals.len()+total_drafts`/`progress_count+1` | T01/T02/T08 | §12.8/§12.18; CDR §3.3 |
| OB-34 | Canonical encoder/hash implementation gaps: no seed section, manifest §4 vs `contract.rs` token conflict, numeric-value hashing partial, per-record encoder layer absent, u64 span encoding, seed-equality test boundary, decoder-vs-round-trip decision | T01 `[INT]` | CDR §I/§12; OB-14/OB-15 |
| OB-35 | Host/task taxonomy collision: `read-source`/`write-artifact`/`invoke-toolchain` proposed as group-1 `TaskKind`s duplicate the frozen `HostRequestKind`; no valid chip owner; the CT14→CT08→Host finalization chain is unstated | T01/T02 | proposal §9; T02 CT08/CT14 |
| OB-36 | CT14 `ArtifactFinalize` M1 scope: absent from the M1 T02 chip list yet the successful-run terminal `JobState::Finished` depends on it; Part A evidence commands (`--emit-*`) and the Part B target-output command (`-S -o`) write files with no M1 Host-task scope | T02/T01 | M1 vertical §7/§8; target §9 |
| OB-37 | Part B scope labeling: T11 prerequisite "target ABI frozen" conflates identity with probe-verified values; wave entries list T11 without a probe annotation; CG set incomplete/non-exhaustive for the M1 path | T11/T01 | T11 prerequisite (opening); PARALLEL_EXECUTION §1; M1 target P4/P5 |
| OB-38 | Cross-reference hygiene: T00-vs-M1-end-to-end pointer (`M1_TARGET_ACCEPTANCE`), `T00 §1.1`→§1 item 1, T01 §6→§7.1 C02 citation, and the "M1 vertical/frontend acceptance" naming split | integrator | M1 vertical §1; M1 target §3; proposal §24 |
| OB-39 | M1 job bootstrap `JobState::Idle→Running` undefined: `/5` initializes `Idle` and never sets `Running` (only `routing.rs` sets `Failed`); `bootstrap_task` sets no job state; CT01 has no §9 M1 kind/stage; no named transition/tick order | T01 `[INT]` (+T02) | proposal §10.2/§10.5; T02 CT01/CT02; `compiler/src/bus.rs` |
| OB-40 | Source-import satisfaction flag undefined: `/5` creates `HostRequestRecord.satisfied = false` and never sets it `true`; no setter/consumer, no request-ID binding, no `SourceImported` carrier | T01/T02 | T01 §4; T02 CT02/CT08; proposal §5/§9; `compiler/src/commit.rs` |
| OB-41 | Host-wait exit undefined: `AwaitHost` sets `Waiting(host_request)` but no rule names who observes `satisfied`, consumes the response, and reinserts the task exactly once into `stage_queues[stage_of(task.kind)]` | T01/T02 | proposal §5/§6.3/§7; T02 CT02/CT07/CT08/CT13 |
| OB-42 | `M1AppendSchema` §12.12 (a)–(h) omits the `ResultValue` wire additions (`DraftRecords`, reused `Record` payload typing), so the `/6` seed hash does not pin them | T01 `[INT]` | proposal §6.3/§12.12/§12.15; T01 C05; CDR row I; cf. OB-18 |
| OB-43 | `M1AppendSchema` omits the `Proposal` wire additions `AppendRecords`/`Progress`/`AwaitChildren` (spellings, coexistence, encodings) | T01 `[INT]` (+T02/T05) | proposal §6.3/§12.12/§12.17; CDR §I4 |
| OB-44 | `PipelineMetrics` field-level writer/encoding undefined: `bus.report.metrics` selected but no per-field owner/byte encoding; `/5` has no report/metrics register | T01/T02 | proposal §6.2.1/§12.9/§12.17; T02 CT03/CT06/CT10 |
| OB-45 | M1 vertical §6.2 `M1-UNS-01..09` rows lack the `Chip(s)` and `Gate` columns used by other fixture tables | T01/integrator (+T02/T03/T05/T06/T09/T13) | M1 vertical §6.2 |
| OB-46 | **RESOLVED (docs-only chip-ID audit pass, 2026-10-05):** §9 now lists the M1-exercised diagnostic-only chips — Preprocess `PP26` (`M1-NEG-01/02`), Lex `LX09`/`LX12`/`LX13`/`LX16`/`LX18`, Parse `PA18` (`M1-NEG-19`) — and marks `PP02`/`PP03` identity-only; the exact `PP26` role for the malformed fixtures remains a T03/T01 co-freeze item. | T03/T04/T05/T01 | proposal §9; M1 vertical §7 |
| OB-47 | **RESOLVED (docs-only chip-ID audit pass, 2026-10-05):** §9 now lists the M1 T13 set `VF01–VF06, VF12–VF14` (matching M1 vertical §7 and T13 rev 8) and no longer lists the out-of-M1 `VF07`/`VF08`; the M1 IR/CFG checker references are recorded as an explicit gap (M1 vertical §8; T13 rev 8). | T13/T01 | proposal §9; M1 vertical §7; T13 rev 8 |
| OB-48 | `StoreOwnerViolation` absent from `/5`: `ManifestError` has no such variant; the mandatory `ChipId`-keyed allowlist/seed/hash and `tasks.ready` second-writer rejection are proposal-only | T01 `[INT]` | proposal §6.4/§12.9/§12.12(e); `compiler/src/manifest.rs`; M1-WS-04 |
| OB-49 | T01 §4's deterministic reserved-ID/local-reference relocation protocol is absent (only predicted IDs in `commit.rs`); no named reservation/apply-map protocol or hashed rule; OB-10 covers only the T04 reciprocal pair | T01 `[INT]` (+T04/T05) | T01 §4; proposal §7 phases 2a/2b/3; cf. OB-10/OB-28 |
| OB-50 | Exactly-once status conflict: T05 item F point 5 / T06 item 7 present `ResultRecord.consumed` as a candidate durable claim while §24.3/OB-11 keep exactly-once open and `/5` `consume_result` clones the whole value with no partial claim API | T05/T06/T01 | T05 item F point 5; T06 item 7; §24.3; OB-11 |
| OB-51 | rev-46 "M1-minimal conversion scope" conflicts with the M1 fixture: rev 46 defers integer promotions, but M1 exercises `TY25` (`M1-TY-07`), `TY26` (`M1-TY-08`), `TY27` (`M1-TY-09`) and §9/T06 list them as M1 kinds | T06/T07/T13/T01 | proposal §9/§22.1; T06 M1 qualifier; M1 vertical M1-TY-07/08/09; CDR row F rev 46 |
| OB-52 | E-bootstrap pointer staleness: text still reads "the bootstrap order is a T06/`[INT]` `/6` decision" after the accepted File `Enter` stage edge; only "bootstrap wiring beyond the stage edge" remains open; also the proposal header `rev 21–38` / CDR "rev 39 (expected)" pointers are unreconciled | integrator/T01 | proposal header; §18.3 H3; §17.3 T06-6; §20.10; CDR row E; T06 items 2/5 |
| OB-53 | OPEN-01 TU-root commit order vs scope-query startup ring: early TU-root commit order candidate (root committed in the TU parse task's first batch after lex completes; the parse task is `Waiting` while `symbol_type.scope-enter` dispatches; parse resumes with the committed file scope), fallback explicit M1 no-scope parse path, and the `M1-START-01` no-prebuilt-scope startup fixture | T01/T05/T06 | §24.14; §5 file-scope policy; T05 item F point 6; T06 item 8 |

### 24.12 DOC-15 consistency correction — optional-`ArtifactKind` source policy (same rev 39; no revision advance)

- **DOC-15 (docs-only consistency correction; no new decision, no revision
  advance, no `/6` change, no code).** The 2026-10-05 documentation review
  ([DOC-15](../reviews/2026-10-05_DOCUMENTATION_REVIEW.md)) found that this
  proposal's commit algorithm and draft-validation/schema prose still rejected
  **any** `source` on a map-optional artifact and required `source: None`, while
  the already-recorded **rev-47 delegated candidate default** (CDR §2 row B /
  §9C; T03 rev 47; this proposal §22.2 item 1) allows a map-optional kind to
  carry a **valid `Some(source)`** with **empty `raw_offsets`**. This correction
  syncs §5 `ArtifactKind`/`ArtifactRecord` and the `ArtifactDraft` validation
  table, §6.1 `source_scoped_one_hop`, §7 phase-2c/phase-3 algorithm text, the
  §7 pre-mutation property, the §12 artifact summary, and the §13
  `artifact_map_optional_empty` test description to that default: **map-optional
  kinds require empty `raw_offsets`; `Some(source)` is validated against the
  task's declared payload source (`ArtifactSourceMismatch`) rather than
  rejected; `None` remains allowed; no source-payload-equals-`bytes`
  requirement.** The §13 description adds the synthetic **`Trace`** example: a
  `Trace` artifact with a valid `Some(source)` and empty offsets is **accepted**,
  while a non-empty optional map is rejected. `Trace` remains outside the M1
  produced/asserted scope (no M1 writer), so this is a declared-total-schema
  contract-test consistency fix, not an M1 production claim. The rev-47 default
  remains a **delegated candidate default — not** a T03/T01 owner signoff and
  **not** a freeze; the exact optional-kind error classification/numeric codes
  stay **open**, and overall artifact rows remain pending co-freeze. No other
  document is edited by this correction.

### 24.13 DOC-12 consistency correction — declarator-completion visibility vs identifier-leaf identity (same rev 39; no revision advance)

- **DOC-12 (docs-only consistency correction; no new decision, no revision
  advance, no `/6` change, no code).** The 2026-10-05 documentation review
  ([DOC-12](../reviews/2026-10-05_DOCUMENTATION_REVIEW.md)) found that the
  candidate point-of-declaration rule used the **`Identifier` leaf position** as
  the declaration point, while C11 6.2.1p7 starts an ordinary identifier's scope
  **just after the completion of its declarator**. This correction separates the
  two roles: `SymbolRecord.decl` **remains the `Identifier` leaf** as the stable
  **identity/diagnostic** location (unchanged), while the TY03 lookup
  **visibility tuple** `(source, start, end, decl)` derives `start`/`end` from
  the **completion point of the complete declarator** (the declarator's
  `last_token` span; not the declarator's full range and not the leaf), with
  `decl` kept as the leaf `NodeId`. The exact T05 `NodeKind` ownership/token-range
  semantics remain the T05 `/6` item. This syncs the §5 `SymbolRecord`
  point-of-declaration comment and the §13 named tests: the new contrast test
  `symbol_declarator_visibility_contrast` asserts that in `int n = 3; void
  f(void) { int n[n]; }` the bound resolves to the **outer** `n` (the inner
  declarator is not complete), while in `void f(void) { int n = n; }` the
  initializer resolves to the **declared inner** `n`;
  `symbol_identifier_leaf_decl` is marked identity/diagnostic-only. The rule
  remains a **delegated candidate default — not** an owner/T01 signoff and
  **not** a freeze; `M1-TY-06` is preserved (the tuple shape is unchanged).
  Mirrored in the T06 package (§Operative amendment items 4/6b/6c/6f, the TY03
  row, and the revision record) and the CDR §E1 request text.

### 24.14 OPEN-01 resolution candidate — early TU-root commit order + `M1-START-01` no-prebuilt-scope startup fixture (`DELEGATED-CANDIDATE-DEFAULT` + `OPEN-BLOCKER`)

- **OPEN-01 (documentation review, 2026-10-05).** The review records the
  **startup ring**: the §5 file-scope policy opens the file scope by a T06 task
  that runs after the committed `TranslationUnit` node, while T05 parsing depends
  on scope/typedef queries (T05 opening text; `M1-PA-04`'s PA04 query; the T06
  `TY01 → TY07 → TY08 → TY03` order) and `M1-PA-05`/`M1-TY-05` exercise block
  scope enter/exit during parse. If the TU root were only committed when the full
  TU parse completes, the startup would be circular. The review asks for an
  explicit **early TU-root commit order** or an explicit **M1 no-scope parse
  path**, plus a **no-prebuilt-scope startup test**.
- **Selected candidate resolution — early TU-root commit order (delegated
  candidate default under the same explicit user delegation, 2026-10-05; NOT a
  T05/T06 owner signoff, NOT T01 `[INT]` acceptance, and NOT frozen).** The
  deterministic M1 startup order is:
  1. **Precondition — lex complete.** The TU's whole token stream, first
     committed token through the unique committed EOF token, is committed in the
     `lex` stage (T04 `LX17` ordered publication; M1: 12 tokens + one EOF). No
     parse task runs before this.
  2. **Bootstrap.** The job bootstrap (T02 CT01/CT02; the exact M1
     `Idle→Running` transition/kind remains open — OB-39) enqueues exactly one
     registered `parse.TranslationUnit` task in the `parse` stage. The bootstrap
     creates **no** scope and seeds nothing.
  3. **Early TU-root commit (the task's first commit-visible batch).** The task
     appends exactly one `NodeRecord { kind: TranslationUnit, parent: None,
     ordinal: 0, first_token: <first committed token>, last_token: <committed
     EOF> }` (empty TU: the empty range at EOF, `first == last == EOF`) and
     carries the external-declaration loop in its continuation.
  4. **File-Enter edge, exactly once.** After the root `NodeRecord` is
     commit-visible, the deterministic `parse.TranslationUnit ->
     symbol_type.scope-enter` edge (rev 43, accepted direction) enqueues exactly
     one T06 scope-enter task carrying the **committed** root `NodeId`. The edge
     fires once per committed root append (one root per TU; duplicate-root
     rejection is a T05 pre-commit invariant); it is not a job-bootstrap action
     and never carries a same-batch draft. Candidate realization: the same first
     batch enqueues the child (structural exactly-once via one atomic commit), or
     a T01-owned commit-apply edge hook observes the committed root append
     (mirroring the rev-49 H11 hook style); the mechanism remains co-freeze.
  5. **Stage-order guard — the TU parse task waits.** After its first batch the
     `parse.TranslationUnit` task is `Waiting(WaitSet { children: [scope-enter
     child] })`, not `Ready`: dispatch order is `(stage ordinal, phase priority,
     enqueue ordinal, TaskId)`, so a ready `parse` task (stage 2) would always
     precede the `symbol_type` scope-enter task (stage 3). The wait is what lets
     the file scope be created before the parse continues; the task resumes in
     `stage_queues[stage_of(parent.kind)]` (parse) only after the child is
     terminal (all `Completed` → `Ready`; any `Failed` → `Failed` once, never
     `Ready`).
  6. **File-scope commit.** The T06 task creates exactly one `ScopeRecord {
     parent: None, kind: File }` and appends exactly one `Enter` with `at` = the
     committed TU-root `NodeId`; **no Exit** in M1. **No prebuilt scope**: the job
     bootstrap, the Host, and the fixture seed create none.
  7. **Parse continues with the committed file scope.** On resume the TU parse
     task carries `scope: Some(file ScopeId)`; external declarations are parsed
     with scope/typedef visibility preserved (T05 opening text), and parse-time
     T06 interactions are awaited child tasks (never guessed). The
     `TY01 → TY07 → TY08 → TY03` order for the non-M1 typedef fixture is
     preserved; M1 itself has no typedef (`TY08` unexercised).
  8. **Terminal completion unchanged.** When the full TU parse finishes, the task
     completes with the existing generic `TaskState::Completed(ResultId)` /
     `ResultValue::Record(RecordRef::Node(root_node_id))` (T05 item F points 1–2
     preserved) for downstream consumers that need the complete TU. Only the
     File-Enter edge trigger moves from terminal completion to the early root
     append.
- **Compatibility.** The accepted rev-43 direction (deterministic
  `parse.TranslationUnit -> symbol_type.scope-enter` edge after the committed TU,
  carrying the committed `NodeId`, no job-bootstrap) and H3 are preserved. The
  T05 item F / T06 item 5 candidate trigger ("after task completion") is
  **qualified for the File-Enter edge only**; T06 item 7's `ResultRecord.consumed`
  candidate is **not required for this edge** (exactly-once is structural from
  the single root append) but remains available for other result hand-offs.
  OB-11/OB-50 stay open until co-freeze; this entry proposes a resolution path
  and closes neither. `M1-PA-04`'s PA04 query, `M1-PA-05`/`M1-TY-05` block scope
  operations, and `M1-TY-06` are preserved; no M1 acceptance row is weakened and
  no no-scope carve-out is needed on the selected path. The stale bootstrap-order
  pointer flagged by OB-52 is superseded **for the candidate order** only; the
  bootstrap wiring beyond the edge remains open.
- **Fallback — explicit M1 no-scope parse path (if the early root commit is
  rejected).** `ContinuationRecord.scope` stays `None`; the M1 parse performs no
  scope/typedef query required for a parse decision before the file scope exists
  (no typedef names exist; the `M1-PA-04` ordinary-identifier classification is
  grammar-determined or deferred to the `symbol_type` stage after the file
  Enter, never guessed from the string); parse-emitted block scope enter/exit
  requests are **deferred** to the `symbol_type` stage after the file Enter (the
  ordering rule for deferred requests is co-freeze); the file Enter keeps the
  accepted late-commit edge and the T05 item F / T06 item 7 carriers. This
  fallback explicitly limits the M1 parse path and must not be read as a general
  scope-free parser.
- **No-prebuilt-scope startup test (`M1-START-01`; required pending fixture;
  registration with T13/M1 vertical acceptance is an integration action).**
  Fresh bus from job bootstrap with `M1-SRC-000`; assert: (1) before the TU-root
  commit there are **zero** `ScopeRecord`s and **zero** `ScopeEventRecord`s and
  no bootstrap/Host-created scope; (2) the registered `parse.TranslationUnit`
  task is the first semantic task and commits the root `NodeRecord`
  (`parent: None`, range first committed token..committed EOF); (3) the stage
  edge enqueues **exactly one** `symbol_type.scope-enter` carrying the committed
  root `NodeId`, and re-observation on later ticks enqueues no second Enter;
  (4) exactly one file `ScopeRecord` and one `Enter` with `at` = the committed
  root `NodeId`, no file `Exit`; (5) under the selected path the TU parse task is
  `Waiting` while the scope-enter task dispatches and resumes only after the file
  `Enter` is committed, with `ScopeEventId` order file Enter before any block
  Enter; under the fallback, no parse decision depends on a scope query before
  the file Enter; (6) identical snapshots/pins replay to the identical record
  order; (7) a pre-file-scope scope/typedef query (outside M1) is a typed
  unsupported/diagnostic, never a guessed classification. The fixture is **not
  implemented, not run, and not evidence**.
- **Still open (T05/T01/T06 `/6` co-freeze).** The edge-enqueue realization
  (same-batch link vs T01 commit-apply hook) and its exact payload encoding; the
  duplicate-root pre-commit validation; the root-range/descendant invariant for
  a root committed before its children (the T05 "range covers descendants"
  blocker); the bootstrap task kind/stage (OB-39); and all numeric encodings.
  This entry is a delegated candidate default, not a T05/T06 owner signoff, not
  T01 `[INT]` acceptance, and not a `/6` freeze. `/5` remains current, M1 remains
  DRAFT, ADR-0002 remains PROPOSED, and no code is authorized.

### 24.15 Group A (scheduler/commit boundary) user-accepted recommendations (2026-10-05)

- **Group A scope (docs-only; no code, no commit, no freeze).** This subsection
  records 11 user-accepted Group A (scheduler/commit boundary) recommendations
  accepted 2026-10-05. Each is a **user-accepted recommendation; T01 co-freeze
  pending; not a `/6` freeze; no code**. None changes `/5`, none freezes `/6`,
  none authorizes code, and all OB rows and historical text above remain
  verbatim except the §24.11 pointer sentence.
- **OB-1 — dispatcher pre-worker mutation vs semantic commit; `in_flight` clear
  ownership (user-accepted recommendation; T01 co-freeze pending; not a `/6`
  freeze; no code):** the dispatcher's per-batch `Ready→Running` transition plus
  in-flight population stays a distinct pre-worker mutation outside the semantic
  commit; the sole clear owner is the dispatcher/backend clearing `in_flight`
  at latch after H6 recovery; rationale: this keeps the scheduling mutation
  boundary explicit so the one ordered atomic semantic commit is never conflated
  with dispatch, and the clear is not itself a transition.
- **OB-2 — empty-proposal outcome (user-accepted recommendation; T01 co-freeze
  pending; not a `/6` freeze; no code):** a dispatched task with an empty
  proposal vector gets a per-task `Fail` with a diagnostic; rationale: this
  rules out silent `Completed` and rules out leaving the task `Running`.
- **OB-3 — cancel vs tick-budget precedence (user-accepted recommendation; T01
  co-freeze pending; not a `/6` freeze; no code):** cancel takes precedence
  over tick-budget; the cancel tick performs no new
  selection/dispatch/commit, tasks are otherwise untouched, repeated cancel is
  idempotent, and exactly one terminal report record is emitted; rationale: this
  gives cancel a single deterministic tick semantic with no new work admitted
  on the cancel path.
- **OB-5 — `Progress` reinsert vs `max_task_progress` exceedance (user-accepted
  recommendation; T01 co-freeze pending; not a `/6` freeze; no code):**
  `max_task_progress` exceedance is a per-task `Fail` (chip-level diagnostic);
  `Progress` reinsert is exactly-once into the task's own stage queue with
  `ready_tick = tick + 1`, mutually exclusive with `Fail`; rationale: this
  separates the per-task quota breach from the requeue path so a task never
  both requeues and fails on the same tick.
- **OB-26 — `FunctionEnd` phase-2b hook failure vs H6 (user-accepted
  recommendation; T01 co-freeze pending; not a `/6` freeze; no code):** a
  `FunctionEnd` phase-2b hook failure rejects the whole batch as a
  `TerminatorMissing` `CommitError`, then H6 recovery applies; the
  "already-committed" phrasing is corrected to "proposed completion inside the
  failing batch"; rationale: this resolves the rollback contradiction — there
  is no durable completion fact when the batch that proposed it is rejected.
- **OB-28 — "phase 2b" label collision rename (user-accepted recommendation;
  T01 co-freeze pending; not a `/6` freeze; no code):** rename the colliding
  "phase 2b" labels to P1 structural-refs (including a frozen P1.0 per-task
  inventory sub-pass producing own `AppendRecords` ranges + `Enqueue` counts),
  P2a–2d resolve, P2E cumulative-semantic, and P2F completion-hook, preserving
  rev-43 semantics; rationale: this removes the cross-reference collision while
  making the per-task inventory sub-pass explicit.
- **OB-29 — `ChildRef::Committed` predicate + error name (user-accepted
  recommendation; T01 co-freeze pending; not a `/6` freeze; no code):** the
  `ChildRef::Committed` predicate is: `TaskId` exists AND a committed `Enqueue`
  record shows `parent == this task` AND live; committed-child failures map to
  `AwaitChildrenRefInvalid` (payload extended to carry the reference);
  rationale: this gives the predicate an auditable committed-parent link plus
  liveness, with one named error carrying the offending reference.
- **OB-39 — job bootstrap `Idle→Running` ownership (user-accepted
  recommendation; T01 co-freeze pending; not a `/6` freeze; no code):** a
  CT01-owned explicit `Idle→Running` transition exactly once (duplicate start is
  a diagnostic); `Finished` stays CT14-owned; rationale: this names the missing
  bootstrap transition and its owner without moving terminal-job ownership.
- **OB-40 — source-import satisfaction bit (user-accepted recommendation; T01
  co-freeze pending; not a `/6` freeze; no code):** the commit-apply import path
  validates `HostResponse.request` against the committed request ID, appends
  `sources` records, and flips `satisfied := true` in the same atomic commit;
  duplicates/out-of-order get a named diagnostic; rationale: this binds the
  satisfaction flag to the committed request inside the atomic commit so the
  flag can never be set without the matching response.
- **OB-41 — host-wait exit / host join (user-accepted recommendation; T01
  co-freeze pending; not a `/6` freeze; no code):** a commit-apply host-join
  mirroring CT07 — CT02/CT08 decision-only; commit-apply observes committed
  `satisfied` + response, transitions `Waiting(host)→Ready` exactly once and
  reinserts into `stage_queues[stage_of(task.kind)]`; cancel winds down via the
  CT13 path; rationale: this keeps the wait exit single-owner and exactly-once
  while leaving cancel on its existing wind-down path.
- **OB-44 — `PipelineMetrics` deferral (user-accepted recommendation; T01
  co-freeze pending; not a `/6` freeze; no code):** defer full
  `PipelineMetrics` to Part B / post-`/6`; `/6` freezes only
  `TickRecord.dispatched` + `selected` as canonical scheduler-observable facts,
  and §6.2.1 is amended accordingly with a re-entry criterion; rationale: this
  keeps `/6` to the two scheduler facts needed for replay/observability while
  the full metrics writer/encoding set waits for Part B.

### 24.16 Group B (result/diagnostic carriers) user-accepted recommendations (2026-10-05)

- **Group B scope (docs-only; no code, no commit, no freeze).** This subsection
  records 8 user-accepted Group B (result/diagnostic carriers) recommendations
  accepted 2026-10-05. Each is a **user-accepted recommendation; T01/owner
  co-freeze pending; not a `/6` freeze; no code**. None changes `/5`, none
  freezes `/6`, none authorizes code, and all OB rows and historical text above
  remain verbatim except the §24.11 pointer sentence.
- **OB-4 — join decides child terminal state only and consumes nothing
  (user-accepted recommendation; T01/owner co-freeze pending; not a `/6`
  freeze; no code):** the join decides child terminal state only and consumes
  nothing (parent-owns-consumption); under await-all the parent stays `Waiting`
  until all committed children are terminal; non-terminal siblings are waited
  on, not cancelled (the fail-fast framing stays superseded); rationale: this
  keeps result consumption with the parent's own atomic commit so the join never
  needs a durable parent-side handoff, and await-all waits out slow siblings
  instead of cancelling them.
- **OB-50 — File-Enter structural exactly-once vs `consumed`-bool hand-offs
  (user-accepted recommendation; T01/owner co-freeze pending; not a `/6`
  freeze; no code):** the File-Enter edge uses structural exactly-once
  (committed TU-root append cardinality, no result consumed); other
  completion-triggered hand-offs reuse the `ResultRecord.consumed` bool with
  per-result-kind consumer fencing (edge-scoped consumption, whole-value clone
  acceptable); rationale: this keeps the File-Enter edge free of result
  consumption while giving the remaining hand-offs one durable claim bit with
  per-kind fencing instead of a new claim mechanism.
- **OB-6 — `ConstantResult` carrier, no new variant (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  no new `ResultValue` variant; Legal constant results complete with
  `Record(RecordRef::Const)`; this explicitly amends the rev-44 "legality as
  result payload field" selection to outcome-carried legality; rationale: this
  reuses the existing generic record carrier for the Legal path and moves
  legality out of the payload into the completion outcome.
- **OB-7 — non-legal constant coupling (user-accepted recommendation;
  T01/owner co-freeze pending; not a `/6` freeze; no code):** value absent on
  non-legal outcomes; no `ConstRecord` is committed for non-legal results;
  non-legal outcomes `Fail` with the accepted chip diagnostics
  (`ConstUnsupported`/`ConstOverflow` and a `NotConstantExpression`-family code
  to be named); T09 refuses non-legal constants by the missing-`ConstId` rule
  plus its existing immediate/`ty` checks; rationale: this keeps non-constant
  or unsupported evaluations from materializing a value record while giving T09
  a structural refusal (no `ConstId`) layered on its existing checks.
- **OB-8 — `ConstRecord` identity, one per request (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  one `ConstRecord` per successful `const.evaluate` request (no dedup, no
  literal/node back-link in M1); provenance stays in the `ConstantRequest`;
  rationale: this keeps the M1 identity rule trivially auditable (request count
  equals committed value count) and leaves dedup and back-links out of M1.
- **OB-27 — nonzero `EffectMask` carrier (user-accepted recommendation;
  T01/owner co-freeze pending; not a `/6` freeze; no code):** nonzero
  `EffectMask` is a chip `DiagnosticDraft` via `Fail` (CDR-rev-46-literal), not
  a `CommitError` and not VF06-only; gives VF06 a committed negative case to
  verify; rationale: this classifies unexpected effects as a semantic chip
  diagnostic on the failing task while supplying the committed negative fixture
  VF06 needs.
- **OB-30 — parse `next_cursor` reuses `ContinuationDraft.cursor`
  (user-accepted recommendation; T01/owner co-freeze pending; not a `/6`
  freeze; no code):** parse `next_cursor` reuses `ContinuationDraft.cursor`
  (no new `ResultValue` variant); T05 must specify the continuation read path,
  manifest scope, and the rule for tasks completing without an appended
  continuation; rationale: this avoids a new result-variant family for the
  cursor while leaving the read path, manifest ownership, and the
  no-continuation completion rule as explicit T05 co-freeze items.
- **OB-31 — T01 §4 continuation supersession entry (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  a formal `/6` supersession entry replacing the T01 §4 continuation sentence
  (`operands` superseded by `partial_children`/`next_child_ordinal`; `awaited`
  superseded by `WaitSet`-only); all pre-rev-42 fail-fast/sibling-cancellation
  sentences labeled superseded history; rationale: this retires the stale T01
  §4 sentence and the stale fail-fast/sibling-cancellation framing in one
  catalogued supersession instead of silent edits.

### 24.17 Group C (records/links/schema/hash) user-accepted recommendations (2026-10-05)

- **Group C scope (docs-only; no code, no commit, no freeze).** This subsection
  records 17 user-accepted Group C (records/links/schema/hash) recommendations
  accepted 2026-10-05. Each is a **user-accepted recommendation; T01/owner
  co-freeze pending; not a `/6` freeze; no code**. None changes `/5`, none
  freezes `/6`, none authorizes code, and all OB rows and historical text above
  remain verbatim except the §24.11 pointer sentence.
- **OB-10/OB-49 — T01 reserved-ID protocol generalizing the T04-rev-2 condition
  (user-accepted recommendation; T01/owner co-freeze pending; not a `/6`
  freeze; no code):** generalize the T04-rev-2 condition to a T01 reserved-ID
  protocol — whole-batch stable-ID pre-reservation/prediction (P1.0 per-task
  inventory, 2a name plan, 2b predicted `RecordRef` per family) before any typed
  `RecordLink` validation/resolution, then resolve; rationale: the reciprocal
  token↔literal cycle resolves without topological order once every ID in the
  batch is predicted before any link is checked.
- **OB-12 — append-family owners + allowlist mechanism first, per-chip splits
  wave-gated (user-accepted recommendation; T01/owner co-freeze pending; not a
  `/6` freeze; no code):** freeze append-family owners (§8) + the allowlist
  mechanism first; per-chip field splits land wave-gated with their tests under
  an explicit "group-owner-only, no chip dispatch" interim gate; rationale: the
  owner + mechanism freeze unblocks integration while per-chip splits stay
  gated to their own waves with tests instead of blocking the seed.
- **OB-32 — `DiagnosticDraft` name fix + M1-exercised manifests only
  (user-accepted recommendation; T01/owner co-freeze pending; not a `/6`
  freeze; no code):** fix the diagnostic name once as `DiagnosticDraft` (update
  T01 §5 and T02 CT11 references); write field-level manifests only for
  M1-exercised chips, all others OUT-OF-M1 with explicit gap gates; rationale:
  one naming fix removes the `DiagnosticDraft` vs `DiagnosticProposal` split
  while the manifest scope stays proportional to what M1 exercises.
- **OB-48 — `StoreOwnerViolation` + allowlist rows as written (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  implement as written — new `ManifestError::StoreOwnerViolation` (+ stage
  errors), (`ChipId`,`StoreId`,field,kind) allowlist rows with seed/signature
  in the `M1AppendSchema` hash; `tasks.ready` gets zero allowlisted chip
  writers; rationale: the allowlist becomes hash-pinned seed with a named
  violation while the ready queue stays free of chip writers.
- **OB-13 — single-freeze closed inventory (user-accepted recommendation;
  T01/owner co-freeze pending; not a `/6` freeze; no code):** single-freeze
  closed inventory — all families, tags+ordinals, stores, and the mapping table
  frozen byte-for-byte with a seed-equality test; rationale: one atomic freeze
  keeps the family/tag/store/map inventories from drifting across partial
  freezes, with the equality test as the gate.
- **OB-14 — two-tier minimal seed (user-accepted recommendation; T01/owner
  co-freeze pending; not a `/6` freeze; no code):** two-tier minimal seed —
  inventories + `TickRecord.dispatched`/`selected` + encode presence; full
  metrics/report bodies deferred to Part B with re-entry criterion (aligned with
  the OB-44 deferral); rationale: `/6` pins only the minimal
  scheduler-observable seed while the full metrics/report bodies wait for Part B
  under the same deferral.
- **OB-34 — atomic manifest-§4/token fix + numeric-symbolic carve-out +
  round-trip gate (user-accepted recommendation; T01/owner co-freeze pending;
  not a `/6` freeze; no code):** fix the manifest-§4/token contradiction
  atomically at `/6`; numeric-symbolic carve-out (enum names + field order
  hashed, magnitudes/widths excluded until probe-gated Part B); snapshot
  `encode_*` implemented with round-trip tests now, hash covers inventories +
  encoder names; rationale: the contradiction is repaired in one atomic update
  while round-trip (decode + re-encode identity) gates the encoders and
  unverified magnitudes stay out of the hash until the probe gates Part B.
- **OB-42 — Group-B reuse for `ResultValue` wires (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  adopt Group-B reuse — drop `DraftRecords` from `M1AppendSchema`;
  Legal=`Record`, multi=`Records`, non-legal=`Fail`, cursor via continuation;
  retract §6.3/§12.1 candidate text accordingly; rationale: the result-wire set
  reuses the accepted Group-B carriers instead of hashing a new `DraftRecords`
  variant, with the stale candidate text retracted.
- **OB-43 — explicit wire-inventory item + supersession catalog (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  append an explicit wire-inventory item (`PROPOSAL_NAMES` 8 entries +
  coexistence rule ids + wire tags + `push_wires` arms) plus the T01 §4/§7.1
  supersession catalog entries; rationale: the `AppendRecords`/`Progress`/
  `AwaitChildren` wire additions are pinned as an inventoried set with their
  supersession entries instead of unlisted prose.
- **OB-15 — `SpanRecord.start/end` `u64` widening (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  widen `SpanRecord.start/end` to `u64` with fixed little-endian u64 snapshot
  encoding; byte-compat break handled by `/6` version bump + migration test;
  rationale: the widened span bounds get one fixed encoding while the compat
  break is handled explicitly by version + migration instead of silent
  reinterpretation.
- **OB-19 — `SemId` + `sem.records` family/arena + `RecordRef::Sem`
  (user-accepted recommendation; T01/owner co-freeze pending; not a `/6`
  freeze; no code):** new `SemId` + `sem.records` family/arena +
  `RecordRef::Sem` (uniform record/link/encoder treatment, explicit T09 link
  target); rationale: sem records get the same family/arena/ref/link/encoder
  standing as every other family with T09 consuming the typed link.
- **OB-20 — `SemRecord` shape + `TypeId` reuse scan + scope-event order +
  allowlist rows (user-accepted recommendation; T01/owner co-freeze pending;
  not a `/6` freeze; no code):** `SemRecord` keeps `ty` carrying the signature
  plus a separate plan-holder field; `TypeId` reuse =
  structural-equality-on-`TypeKind` scan of all committed `types.records`,
  lowest-id wins, no cache; scope events field order (`scope`,`kind`,`at`) with
  `ChipId`-keyed allowlist rows in the seed; T09 typed link targets
  `RecordRef::Sem`; rationale: the sem shape, the cache-free canonical-reuse
  scan, the fixed scope-event order, and the typed T09 link are pinned together
  so none drifts independently.
- **OB-21 — committed-or-same-batch wording + file-Exit reason (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  committed-or-same-batch wording; file-Exit is a `ScopeLifecycleViolation`
  reason (single rule covers intra-batch doubles); rationale: one wording
  covers the same-batch scope-event case while one violation reason covers
  both inter- and intra-batch file-Exit doubles.
- **OB-22 — `i128` carrier with sufficient-budget representability only
  (user-accepted recommendation; T01/owner co-freeze pending; not a `/6`
  freeze; no code):** `i128` carrier for M1 with "sufficient-budget
  representability" only; general formula, BigInt, enforcement split, and
  symbolic-vs-probe gating deferred post-M1; rationale: M1 commits to the
  carrier plus the minimal representability claim while the formula/general
  carrier/enforcement/gating questions stay post-M1.
- **OB-23 — VF06 fails only missing M1-minimal required conversions
  (user-accepted recommendation; T01/owner co-freeze pending; not a `/6`
  freeze; no code):** VF06 fails only missing M1-minimal required conversions;
  non-M1 conversions are chip unsupported diagnostics outside VF06; rationale:
  the VF06 completeness domain stays exactly the M1-minimal required set while
  non-M1 conversions remain explicitly unsupported rather than VF06 failures.
- **OB-24 — split PP01/PP04 commits + continuation-carried chunk cursor +
  `max_source_bytes` scope (user-accepted recommendation; T01/owner co-freeze
  pending; not a `/6` freeze; no code):** split commits (PP01 commits
  artifact/spans, PP04 commits tokens in later ticks with committed-span refs);
  chunk cursor carried in the continuation (no new bus field);
  `max_source_bytes` bounds source bytes and artifact bytes as stated;
  rationale: the commit split keeps span refs committed before tokens use them,
  the cursor needs no new bus field, and the byte bound keeps its stated scope.
- **OB-52 hygiene — reconcile E-bootstrap pointer staleness (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  reconcile the E-bootstrap pointer staleness when touching those paragraphs
  (only "bootstrap wiring beyond the stage edge" stays open); rationale: the
  accepted File `Enter` stage edge replaces the stale bootstrap-order wording
  so only the wiring beyond the edge remains open.

### 24.18 Group D/E/F (semantics/host/acceptance/limits) user-accepted recommendations (2026-10-05)

- **Group D/E/F scope (docs-only; no code, no commit, no freeze).** This
  subsection records 11 user-accepted Group D/E/F (semantics/host/acceptance/
  limits) recommendations accepted 2026-10-05. Each is a **user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code**.
  None changes `/5`, none freezes `/6`, none authorizes code, and all OB rows
  and historical text above remain verbatim except the §24.11 pointer sentence.
- **OB-51 — TY25–TY27 admitted as identity-only in M1 (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  admit TY25–TY27 as identity-only in M1 (the int→int no-op corner); the
  rev-46 "integer promotions deferred" means non-identity promotions, not the
  int→int row; use one consistent scope phrase across §9, §22.1, CDR row
  F/§F2–F3, the T06 qualifier, T07-S5, the T09 scope bullet, and T13 VF06
  items 3/6; VF06 missing-conversion negatives stay synthetic (per OB-23);
  rationale: this resolves the rev-46/M1-fixture contradiction without
  widening the M1 conversion scope, keeping the exercised set exactly the
  identity corner while non-identity promotions stay deferred and VF06
  negatives stay synthetic.
- **OB-11 — structural File-Enter + `consumed`-bool hand-offs (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  structural File-Enter (committed TU-root append cardinality, no result
  consumed) + other completion-triggered hand-offs reuse
  `ResultRecord.consumed` with per-kind consumer fencing (Group B OB-50
  direction); rationale: this keeps the File-Enter edge free of result
  consumption while giving the remaining hand-offs one durable claim bit with
  per-kind fencing instead of a new claim mechanism.
- **OB-35 — Host/task taxonomy layer split (user-accepted recommendation;
  T01/owner co-freeze pending; not a `/6` freeze; no code):** layer split —
  group-1 `TaskKind`s name schedulable T02-owned units (CT08 family),
  `HostRequestKind` names wire-protocol payloads; freeze an explicit
  kind↔request mapping table with disambiguated names plus the
  CT14→CT08→Host chain and the exactly-once request rule; rationale: this
  separates the schedulable-unit namespace from the wire-protocol namespace so
  no chip owner is invented for Host payloads and the finalization chain has
  one stated path with one request rule.
- **OB-36 — CT14 as M1 terminal gate only (user-accepted recommendation;
  T01/owner co-freeze pending; not a `/6` freeze; no code):** CT14 in M1 as
  terminal gate only — validates no-incomplete + fragments present, emits the
  `WriteArtifact` Host request proposal, never writes files; `Finished` is
  observed via the Group A host-join; Part A `--emit-*` are
  Host-materialized snapshots/traces, Part B `-S -o` is the Host write after
  CT14; freeze which fragments are mandatory in M1; rationale: this keeps
  file writes on the Host side with CT14 as a gate, so M1 needs no chip-owned
  file scope while the mandatory-fragment list stays explicit.
- **OB-37 — minimal split-label docs-only (user-accepted recommendation;
  T01/owner co-freeze pending; not a `/6` freeze; no code):** minimal
  split-label docs-only — prerequisite as two bullets (identity frozen per
  T01 §6 vs values UNVERIFIED per §7.1 C02 + probe), wave rows keep the
  Part-B tag, vertical CG list labeled M1-path subset non-exhaustive with the
  T11 catalog authoritative; rationale: this stops the identity/values
  conflation at the prerequisite while keeping the wave/CG labels honest
  about Part-B scope.
- **OB-38 — narrow cross-reference retarget (user-accepted recommendation;
  T01/owner co-freeze pending; not a `/6` freeze; no code):** retarget
  narrowly — corpus/target-identity cites point at T01 §6 (identity) +
  §7.1 C02 (UNVERIFIED/attest status), suite list at T00 §1 items 1–3; keep
  the "vertical = frontend half, end-to-end = target doc" alias line;
  rationale: the cites point at the exact identity vs values sources so the
  alias line no longer papers over the split.
- **OB-45 — M1-UNS `Chip(s)`/`Gate` columns (user-accepted recommendation;
  T01/owner co-freeze pending; not a `/6` freeze; no code):** add
  `Chip(s)`|`Gate` columns to the M1-UNS table with explicit TBD (T01
  co-freeze) where unknown; Gate = G8 (+G12 for concrete-value assertions,
  +mode/policy for UNS-05/08); no invented ownership; rationale: the table
  gains the same chip/gate traceability as the other fixture tables while
  unknowns stay explicit TBDs instead of invented owners.
- **OB-52 — narrow reword preserving history (user-accepted recommendation;
  T01/owner co-freeze pending; not a `/6` freeze; no code):** narrow reword
  preserving history — File-Enter edge accepted (rev 43, no job-bootstrap);
  only bootstrap wiring beyond the edge + T05 TU carrier + kind/stage remain
  open; close the header/CDR-pointer leg as superseded by the rev-40
  correction (narrowing the §24.17 OB-52 hygiene entry); rationale: this keeps
  the accepted edge while shrinking the open set to the wiring beyond it,
  retiring the already-corrected pointer leg instead of re-litigating it.
- **OB-53 — registration/mechanism split for M1-START-01 (user-accepted
  recommendation; T01/owner co-freeze pending; not a `/6` freeze; no code):**
  split registration from mechanism — register the M1-START-01
  shape/asserts with T13/vertical now as test design only; edge realization
  + payload + range invariant + bootstrap kind/stage stay T01/T05/T06 `/6`
  items; rationale: test design (shape/asserts/registration) lands now while
  the mechanism stays co-freeze, so fixture design neither blocks on nor
  pre-empts the edge realization.
- **OB-16/OB-33 — limits shape/`try_new` skeleton now, numerics deferred
  (user-accepted recommendation; T01/owner co-freeze pending; not a `/6`
  freeze; no code):** freeze the shape/`try_new` skeleton now — field names,
  defaults (`max_inflight_per_tick=1`), bound-check phases (pre-dispatch vs
  pre-apply), checked-arithmetic requirement, sole-bound/
  `BackpressureCapacity`/per-task-`Fail` architecture; retire stale
  `max_dispatches_per_tick` lines to history; defer numeric `ConfigError`
  codes, `total_drafts` definition, Q-scaling, and the const trigger formula
  to a named follow-up co-freeze; rationale: the skeleton
  (names/defaults/phases/arithmetic/architecture) freezes now while numerics
  and derived definitions wait for the named follow-up, and stale
  sole-bound-violating lines become history instead of operative text.

### 24.19 OB-17/OB-18 user-accepted recommendations (2026-10-05)

- **Scope (docs-only; no code, no commit, no freeze).** This subsection records
  the user-accepted OB-17/OB-18 recommendations accepted 2026-10-05. Each is a
  **user-accepted recommendation; T01/integrator co-freeze pending; not a `/6`
  freeze; no code**. None changes `/5`, none freezes `/6`, none authorizes
  code, and all OB rows and historical text above remain verbatim except the
  §24.11 pointer sentence.
- **OB-17 — mechanical cleanup per checklist C17-1..C17-9 (user-accepted
  recommendation; T01/integrator co-freeze pending; not a `/6` freeze; no
  code):** (a) qualify/remove stale `DispatchBudgetExceeded` operative lines
  to history-with-rev-51/54-notes (M1 §6/§6.4/§13, CDR §A9/§I3, ADR-0002 §6;
  sole per-tick bound `max_inflight_per_tick`/quota via
  `SelectionBatchOverflow`); (b) promote H6 wording to the rev-42 formula
  everywhere operative (pre-dispatch errors leave tasks `Ready`; a failed
  semantic batch commit commits no semantic state; deterministic bounded
  recovery processes dispatched tasks once in dispatch order to `Failed`; no
  pre-reservation of N diagnostics; per-task `DiagnosticId::NONE` fallback;
  state guard); (c) attach rev-49 supersession pointers to marker-family
  residual sentences (operative direction: reuse the committed IR28
  `FunctionEnd` terminal result with a T01-owned typed phase-2b commit-apply
  validation hook; no new marker family); (d) dispatcher-failure carrier/code
  inventory: the classification is fixed (`CommitError` vs
  dispatcher/scheduling failure vs `ManifestError`) but the carrier
  type/family/numeric codes stay open — recorded as an open T01/integrator
  inventory item to be added at `/6` co-freeze; rationale: stale operative
  lines become qualified history, one H6 formula reads operative everywhere,
  marker residuals point at the rev-49 direction, and the failure vehicle has
  one open inventory slot instead of an implied carrier.
- **OB-18 — `/6` supersession catalog seed per checklist C18-1..C18-5
  (user-accepted recommendation; T01/integrator co-freeze pending; not a `/6`
  freeze; no code):** extend CDR §I4 with entries S1–S12 (S1 continuation
  fields; S2 dispatch ordering; S3 commit order; S4 `Proposal`/`ResultValue`
  deltas; S5 consume envelope; S6 queue membership; S7 latch/in-flight clear;
  S8 five-outcome set + T01 §7.1 C03 update; S9 reserved-ID protocol; S10
  `Fault` representation; S11 Group A/B/C decided entries; S12 mis-attribution
  fix); apply the Group A/B/C decided entries as the first catalogued
  supersessions; fix the §3-vs-§4 mis-attribution (the symbolic-width citation
  belongs to **T01 §3:59-60**, not T01 §4); resolve the S4 `DraftRecords`
  conflict against the OB-42 Group-B reuse direction (Legal=`Record`,
  multi=`Records`, non-legal=`Fail`, cursor via continuation) before freezing
  the seed list; rationale: the §I4 catalog becomes a seeded, ordered,
  attributable list whose first supersessions are the already-decided Group
  A/B/C entries, with the citation and the wire-conflict settled before any
  seed freeze.
