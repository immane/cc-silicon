# CONTRACT_CHANGE_REQUEST — M1 Part A Shared Schema and Deterministic Pipeline

| Field | Value |
|---|---|
| **ID** | `CDR-M1-0001` |
| **Title** | M1 Part A shared-schema materialization + deterministic staged-pipeline amendments |
| **Date** | 2026-10-04 |
| **Author** | M1 review-integration subagent (draft prepared for the T01 integrator and the user; **not** a reviewer sign-off) |
| **Target contract version/hash** | `t01-c01-c06/5` — `61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5` (read-only M1 audit re-confirmed 2026-10-05: `/5` is current; `compiler/contracts/CONTRACT_VERSION` carries this exact version/hash; **no `/6` freeze and no new hash exists**) |
| **Status** | **PARTIALLY ACCEPTED IN PRINCIPLE — user selected the recommended baseline as the `/6` contract-revision working basis (decisions A/B/C direction) on 2026-10-04.** This is **not** an accepted contract, **not** a `/6` freeze, and authorizes **no** interface, store, field, enum, rule, hash, task kind, chip, or code. `/5` remains current. As of the prior revision (rev 41; this status text dates from rev 40/41), T01 integrator acceptance and all owner sign-offs (T02–T09, T13) were pending. **Current truth (rev 55):** selected subdecisions/authority responses are recorded in rev 42/43/44/45/46 (§10, §12 rev 42/43/44/45/46); **rev 47 adds integration-agent selected candidate defaults under explicit user delegation** (2026-10-05) for the optional-artifact-map, T05 parse-depth, T06 namespace/lookup, and T09 noncritical rule items (§9A/§9B/§9C/§10); those delegated candidate defaults are **not** owner or T01 `[INT]` sign-offs and remain **pending final `/6` co-freeze**; **rev 48 records read-only T01 audit findings (docs-only, no code/T01/manifest/other-doc edit, no hash-scope option selected, no freeze)** — the current frozen `/5` is `t01-c01-c06/5` (`6187…63bb5`) with `compiler/src/contract.rs` encoding **24** `RECORD_KINDS` and `compiler/src/ids.rs` `RecordRef` carrying **24** variants (tags 0–23), `RecordFamily` **absent**; **rev 49 records the user's explicit 2026-10-05 critical H11/T09/T01 direction** (the `TerminatorMissing` trigger reuses the committed IR28 `FunctionEnd` terminal result, `TaskState::Completed(ResultId)`, as the deterministic function-completion fact, checked by a T01-owned typed phase-2b commit-apply validation hook; **no** new marker record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder/`ResultValue` variant; not a frozen `/6` hook, no code authorization) **and records read-only H9 audit findings as unresolved blockers** (§9A/§9B/§9C/§10, §12 rev 49); **rev 50 records the user's explicit 2026-10-05 critical HASH-SCOPE decision** ([USER], §3.3/§9/§9A/§9B/§9C/§10, §12 rev 50): the user **accepts the two-tier model** — `StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed** and **participates in the `/6` contract hash**, while **post-seed runtime `StoreSchema::declare()` extensions remain excluded** from the frozen hash and are captured/validated through runtime snapshot/schema mechanisms. This **resolves the conceptual future `/6` boundary** but **authorizes/completes no actual `/6` schema values, hash, tag/count inventory, code, or `/5` change**; the **`/6` integration must atomically** update the normative `COMPILER_SFL_MANIFEST.md` §4 wording, the `hash_excludes` semantic description/token (scoped to **post-seed runtime declarations**, not the frozen M1 seed), and `FrozenSchema::encode`/contract code plus the freeze test, while **preserving the frozen `/5` hash and history**; it **does not itself accept exact `M1AppendSchema` contents/counts nor freeze**, and **T01 must still co-freeze those values after the owners** — the **numeric inventory and test implementation remain pending** and **row I / overall CDR stay open**; **rev 51 records an integration-selected candidate default under explicit user delegation (2026-10-05)**: `max_inflight_per_tick`/quota is the **sole per-tick dispatch-count bound**, and the redundant `max_dispatches_per_tick` is **dropped from the candidate limit inventory and its validation** (grounded in the rev-35/38 H9 direction and the rev-49 audit finding that the two limits duplicated each other; **not** an owner/T01 `[INT]` sign-off, **not** a schema freeze; T01/T02 must confirm/co-freeze the exact names/defaults/codes at `/6`, and T02/T13 tests remain pending). All prior pointers were **rev 51**; M1 proposal **rev 36** / ADR **Revision 16**; the earlier new-marker direction is marked **superseded for operative direction**; overall H stays **pending**, rows C–I stay pending, and the H9 removal direction and H6 mechanism remain unchanged. **Current truth (rev 52):** the CDR is at **rev 52**; **rev 52 records the cross-owner audit/reconciliation (read-only, docs-only)** mirroring the CDR's already-selected items against the now-completed owner packages — **T02 rev 30+31**; **T03/T04 (T04 rev 46, incl. the `LiteralRecord.token` candidate and the same-revision correction from the typed `RecordLink` mechanism away from any `OwnBatch` confusion)**; **T05 `ParseContext` candidate**; **T06**; **T07 sem→const dependency correction**; **T08 exact candidate shape**; **T09 H11**; **T13**; **M1 vertical acceptance rev 27**; **M1 target acceptance rev 29**; and **ADR-0002 Revision 17** — with **no owner package called a `/6` sign-off** (§9C/§10/§12 rev 52). The **rev-51 sole-per-tick-bound** candidate default (`max_inflight_per_tick`/quota) and the **rev-50 two-tier hash-scope** decision remain **selected, with exact implementation/sign-off still pending**; rows C–I remain **PENDING overall**. The shared **M1 proposal is being advanced in parallel to rev 38** (expected resulting file) and the **CDR points at rev 38**, with **ADR-0002 Revision 17** and **T02 rev 31**; the CDR remains an **accepted-decision ledger/work queue, not an executable schema**, and **no chip coding precedes the `/6` freeze** (next integration stage after exact owner artifacts is T01's serial `/6` seed assembly with two-tier-hash self-consistency checking; Part B remains independent). **Current truth (rev 53):** the CDR is at **rev 53**; **rev 53 records the integration-agent selected candidate default under explicit user delegation (2026-10-05)** for the M1 exercised literal subset: the `Lx08CandidateType` M1 vocabulary is the one-member closed set **`{ Int }`**, the M1 literals `2` and `3` are represented as **target-independent `Int` with no bit width**, literal forms **outside** the exercised M1 subset must **never silently default to `Int`** and are **explicitly unsupported/deferred** until their categories/rules are specified, and future categories **append without reinterpretation**; this is a **delegated candidate default, not an owner/T04 or T01 `[INT]` sign-off, not a `/6` freeze, not the complete future C candidate vocabulary, and not the T04/T08 owner/T01 `[INT]` signoff or `/6` freeze**; the **complete future `Lx08CandidateType` member set and numeric encodings remain open**. **Current truth (rev 54):** the CDR is at **rev 54**; **rev 54 is a docs-only read-only-audit + historical-supersession cleanup (2026-10-05; no new decision, no owner/T01 signoff, no `/6` freeze, no code)** that (a) re-marks the rev-21 §G2 `max_const_bits` signed-range/`i128` representable-set **formula**, the `ConstRecord.value: i128` **carrier**, and the chip-level **enforcement** wording as **historical and superseded — not selected** (the T08 correction marked them unselected), retaining only the rev-44 **origin/cap/projection** and leaving the exact **formula/carrier/enforcement open**; (b) qualifies the rev-49 H9 `max_dispatches_per_tick`-vs-`max_inflight_per_tick` conflict text as **historical/superseded** by the rev-51 sole-`max_inflight_per_tick` candidate (history retained), while the **dispatcher `Ready→Running`/`in_flight` commit-boundary and clear-order items stay unresolved**; and (c) records as **open** the latest read-only findings — the **T04 reciprocal token↔literal same-batch link needs a cycle-safe ID reservation**, the **T05/T06 TU `parse.TranslationUnit -> symbol_type.scope-enter` edge needs exactly-once atomic consumption/dedupe**, the **T13 no-residual fixtures are incomplete**, and the **M1 vertical PP-08 failure semantics + exact artifact-map arrays are pending**. Points at M1 proposal **rev 39** (expected), **ADR-0002 Revision 17**, **T02 rev 32**; `/5` current, **M1 DRAFT**, **no `/6` freeze**, no code. Rows **C–I remain `PENDING overall`**. | A **real unresolved hash-scope conflict** exists between `CONTRACT_VERSION`/`contract.rs`/README (`hash_excludes=…group-declared-store-fields…`) and `COMPILER_SFL_MANIFEST.md` §4 ("adding fields or rules changes the frozen contract hash") over the proposed-to-be-hashed `M1AppendSchema` at `/6`, and the CDR's draft counts (19 families / 27 refs / 27 ordinals) are marked **proposed/unverified** against the **`/5` code fact** of 24/24 (§3.2/§3.3, §9, §9A/§9B/§9C, §12 rev 48); the three hash-scope options remain **unselected** and no recommendation is made without authority. The remaining full-bundle sign-offs (T02–T09, T13) and the overall rows C–I **remain pending**; rows C–I and the exact field/enum/rule shapes are still open. **Rev 22 integrated the accumulated rev-21 read-only review findings; rev 23 integrated the independent rev-22 audit H2–H11; rev 24 records the user's in-principle acceptance of the H1 literal-handoff allocation split; rev 25 integrates the rev-24 independent audit F1–F9 and the verified `/5` `routing.rs` single-task failure guarantee** (T03/T04, T05, T06, T07, T08, T09, T02/T13 pipeline, ADR-0002). The point-by-point disposition is the M1 proposal `§17` (rev 22), `§18` (rev 23/24), `§19` (rev 25), and `§20` (rev 26); the owner-facing amendments are §13. No finding is a sign-off. **Rev 26 integrates the rev-25 independent audit items 1–12; like rev 25 it does not change the contract, task kinds, or any code; H6 (batch no-`Running`/terminal mechanism) remains BLOCKED (superseded by the rev-42 H6 scope+mechanism selection below), the H1 shapes remain accepted in principle only (exact shapes pending), and all T01 integrator/owner sign-offs remain pending.** **Rev 27 applies the rev-26 audit corrections (this §12 rev-26 history item (6), the §2 dashboard note, and the §13 update scope; M1 proposal §20.9/§20.10 and residual `frozen` wording); it changes no acceptance status and adds no `/6` claim.** **Rev 28 is a docs-only pointer reconciliation (no acceptance, schema, interface, task-kind, chip, or `/6` change): it corrects the §12 rev-26 history item (6) before→after range to `rev 21–24→rev 21–26` (the rev-27 row had recorded `rev 21–25→rev 21–26`), and advances every current-state M1 pointer/range to `rev 21–28` (this CDR's header/§2/§11/§13, the M1 proposal §16/§18.7/§19.6/§20.6, and ADR-0002 §1.1).** **Rev 29 is a docs-only historical-documentation correction (no acceptance, schema, interface, task-kind, chip, or `/6` change): it corrects the rev-28 rows' "historical revision rows are unchanged" claim (rev 28 did correct the CDR §12 rev-26 history row; the historical rev-27 row is preserved verbatim, and other historical rows are unchanged), records the ADR-0002 Revision 6 (rev-23) H10 chronology erratum (rev 23 corrected the pointer to rev 21–23, with rev 24/25 extending it to rev 21–24; ADR-0002 Revision 10), and advances every current-state M1 pointer/range from `rev 21–28` to `rev 21–29` (this CDR's header/§2/§11/§13, the M1 proposal §16/§18.7/§19.6/§20.6, and ADR-0002 §1.1).** **H1 (user-accepted in principle, 2026-10-04): the split of `node`/`required_kind`/`legality`/`LX08` across a per-literal `LiteralRecord`, a sem-stage `ConstantRequest`, and a `ConstantResult` is recorded as the accepted-in-principle `/6` revision working basis; it is not a freeze and still requires T01 integrator acceptance plus T03/T04/T08 owner co-freeze/sign-off, with the exact schema owner/integrator pending (§C2).** **Rev 31 records two further explicit user in-principle decisions (2026-10-04) as a new revision layer (accepted in principle only — not a contract and not a freeze): (A, Guardrails §6.1 narrow interpretation) a same-task `OwnBatch(DraftRef)` may exist only as a **transient wire/proposal input before commit**; commit validates/resolves it to a committed `TaskId`/record before any persistent state, and no durable cursor/`WaitSet`/join may point at a draft, wire, or address; the guardrail text is **unamended** and T01 integrator implementation-confirmation is pending; (B, awaited children) `TaskState::Waiting(WaitSet)` is the **sole** awaited-child-ID source and `ContinuationRecord` carries no duplicate `awaited`; the user accepts in principle the request to supersede T01 §4 in `/6` (**no `/5` edit**), with T01 integrator and T05 owner acceptance pending. Row D remains **PENDING overall**. No batch H6/H9, CT07, H8/H11, other row, or exact-shape blocker changes.** **Rev 32 is a docs-only history-gap repair (F1/F2; no acceptance, schema, interface, task-kind, chip, or `/6` change): the M1 proposal rev 31 supplies its missing historical rev-29 §16 row; §13 is rewritten to disambiguate the M1-proposal rev 30 (A/B decision record) from the CDR rev 30 (scope/label correction) and CDR rev 31 (A/B decision record); and every current-state M1 pointer/range advances to `rev 21–31` (this CDR's header/§2/§11/§13, the M1 proposal header/§16/§18.7/§19.6/§20.6, and ADR-0002 §1.1 — Revision 12).** **Rev 33 is a docs-only audit-defect correction (no acceptance, schema, interface, task-kind, chip, or `/6` change): it restricts §13's "made no change" claim to CDR rev 31 and the rev-32 history-gap repair, states that CDR rev 30 did change the M1 target acceptance (recorded as its rev 27), labels the same-numbered revision counters by document in the §2 dashboard note, and adds this row. The current M1 proposal stays rev 31 and the CDR becomes rev 33; §13 reports M1 target acceptance rev 27, T02/frontend rev 25, ADR-0002 Rev 12/rev 31, M1 proposal rev 31, CDR rev 33. A/B/H1 remain in-principle only; row D remains PENDING; `/5` current; all approvals pending.** **Rev 34 records a new explicit user in-principle decision (2026-10-04) on the H6 batch-failure recovery direction (no acceptance of a contract, schema, interface, task-kind, chip, or `/6` freeze): after an atomic semantic batch commit fails, transition every dispatched task, in dispatch order, exactly once to `TaskState::Failed` — a committed `DiagnosticId` when diagnostic/record capacity allows, else the `TaskState::Failed(DiagnosticId::NONE)` sentinel — so every dispatched task leaves `Running`; clearing the in-flight set is not itself a transition. This generalizes the verified `/5` `fail_selected` single-task sentinel obligation, which is preserved and not weakened. It is a `/6` working-basis direction only: the exact atomic/bounded implementation requires T01 integrator approval and the batch failure-atomicity details, the diagnostic budget/state mechanism, and the T02/T13 owner fixtures/sign-offs remain co-freeze and pending. H6 therefore moves from "no selected alternative" to "direction accepted in principle, implementation BLOCKED pending T01"; the mechanism is not implemented or frozen. H9 stays BLOCKED; A/B/C and H1 remain in-principle only; D–I remain PENDING (row D stays PENDING overall); `/5` remains current; all T01/owner sign-offs remain pending and no `/6` is authorized. Every current-state M1 pointer/range advances from `rev 21–31` to `rev 21–32` (this CDR's header/§2/§11/§13, the M1 proposal header/§16/§18.4/§19.6/§19.10/§20.1/§20.6/§20.8/§20.10, and ADR-0002 §1.1 — Revision 13); the CDR becomes rev 34.** **Rev 35 records a new explicit user in-principle decision (2026-10-04) on the H9 in-flight scheduling ownership, direction only (no acceptance of a contract, schema, interface, task-kind, chip, or `/6` freeze): the user accepts in principle REMOVING `max_inflight_total` from the `/6` candidate, because sequential per-tick dispatch is already bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; `Waiting` tasks are not in-flight; and a future cross-tick `Running` mode would need a separate CDR. The dispatch batch bound remains the per-tick `max_inflight_per_tick` quota; `tasks.in_flight` remains an ephemeral per-tick scheduler batch only (candidate) that clears at latch only after every dispatched task has a terminal/`Waiting`/`Progress` outcome (or the H6 recovery); the dispatcher's `Ready→Running` pre-worker mutation is distinguished from the ordered atomic semantic commit; H6 recovery semantics are retained. This is a `/6` working-basis direction only, PENDING T01 integrator acceptance; no bound/config/hash is frozen and `/5` is unchanged. CDR edits: header status; §2 dashboard note (title/ledger + row A + counter prose); §A5; §A status honesty; §9 H9; §10 narrative/row A/authority; §11/§13 (incl. the [T02](T02_CONTROL_CHIPS.md) owner-amendment row, T02 rev 28). H9 moves from "BLOCKED (phase/owner undefined)" to "removal direction accepted in principle, pending T01 integrator acceptance"; the remaining H9 `[INT]` items (the dispatcher-`Ready→Running`-vs-ordered-atomic-commit relationship and the residual-set semantics) stay open. H6 stays "direction accepted in principle, implementation BLOCKED"; D–I remain PENDING (row D PENDING overall); the M1 proposal is at rev 33 and the CDR becomes rev 35.** **Rev 36 is a docs-only D3 correction to §13 (no acceptance, schema, interface, task-kind, chip, or `/6` change): the §13 claim that the [M1 frontend acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md) carried a rev-26 H6/CT06 text is wrong — the frontend is at the latest Rev 25 and contains NO H6/CT06 content, and [T02](T02_CONTROL_CHIPS.md) is where the operative H6 batch-failure text lives (rev 25 H6/sentinel bullet and the rev 27 H6-direction bullet, updated to T02 rev 28 by the H9 integration). Every current-state M1 pointer/range advances from `rev 21–33` to the current `rev 21–34` (this CDR's header/§2/§11/§13, the M1 proposal header/§16/§18.6/§18.7/§19.6/§19.10/§20.6/§20.10, ADR-0002 §1.1 — Revision 14, and T02 rev 28); the CDR becomes rev 36.** **Rev 37 is a docs-only consistency cleanup of stale active candidate prose (no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change; matches M1 proposal rev 35, ADR-0002 Revision 15, T02 rev 29): the §A9 and §I dispatcher/scheduling error carriers still listed `InflightQuotaExceeded` as an active `/6` candidate, contradicting the rev-35 H9 removal direction; both lists are corrected to state that `InflightQuotaExceeded` is removed from the candidate with `max_inflight_total` (retained only as the historical rev-21/22 name) and the per-tick dispatch bound stays `SelectionBatchOverflow` on `max_inflight_per_tick`. §3.3's Limits row, §A5, §9, §10 row A, and the §13 T02 row already carried the removal direction and are unchanged. Every current-state M1 pointer/range advances from `rev 21–34` to the current `rev 21–35` (this CDR's header/§2/§11/§13, the M1 proposal header/§16/§18.x/§19.6/§19.10/§20.6/§20.10, ADR-0002 §1.1 — Revision 15, and T02 rev 29). H9 remains "removal direction accepted in principle, pending T01 integrator acceptance"; H6 stays "direction accepted in principle, implementation BLOCKED"; A/B/C and H1 remain in-principle only; D–I remain PENDING (row D PENDING overall); `/5` current; M1 DRAFT; ADR-0002 PROPOSED; all T01/owner sign-offs pending; no freeze/chip authorization.** **Rev 38 is a docs-only lead-in/typo correction (no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change; matches M1 proposal rev 36 and ADR-0002 Revision 16): the §13 opening lead-in now reads the M1 proposal as updated through **rev 36** (it had said rev 34 while continuing through rev 35), and the `per-stage stage queues` typo is fixed. The current-state M1 pointer/range stays `rev 21–35` (unchanged; the M1 proposal document revision advances to rev 36), the ADR-0002 §3 `bus.rs` amendment row now states the per-tick `max_inflight_per_tick`/quota bound only (no separate total in-flight bound), and the CDR becomes rev 38.** **Rev 39 is a docs-only addition of the proposed `/6` co-freeze work queue in §9A (no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change; and no task package, proposal, or ADR edited). It records the three separate gates (Gate 1 T01 shared types/task/schema freeze before any chip code, then Wave 1 assigned-chips-only; Gate 2 Wave 2 on real upstream contracts/artifacts; Gate 3 the full M1 end-to-end run as a separate Part B gate requiring a Linux probe substrate/toolchain/assembler/linker/sysroot/runner, none claimed available), the parallel owner-drafting/review tracks vs the serial freeze dependencies, and the minimum work items A–I with exact authorities; every item is a proposed question/authority assignment, not accepted schema. `/5` stays current, M1 stays DRAFT, ADR-0002 stays PROPOSED, A/B/C and the H1 allocation stay in-principle only, H6/H9 stay directions only, rows D–I remain PENDING, and all T01 integrator/owner sign-offs remain pending; this CDR becomes rev 39 while the shared M1 proposal stays rev 36 and the current-state M1 pointer/range stays rev 21–35.** **Rev 40 is a docs-only addition of the proposed decision-ready recommendation appendix in §9B (no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change; no task package, proposal, or ADR edited). Per row/authority it states one concise proposed recommendation plus its tradeoff/open dependency — C/H1 (lexical-facts + symbolic `LX08` `LiteralRecord`, ref-based `ConstantRequest`/`ConstantResult`; defer target widths/multi-source/Part B writers; T01 owns the `RecordRef`-tag vs `RecordFamily`-ordinal inventories/hash), D/T05 (continuation fields per §D1, no `awaited`, commit-apply join invariant, no new CT07 family, await-all-then-parent; both flagged as open T02/T05 choices), A/T02/T13 (quota-1 M1 baseline preserving the `/5` single-task `fail_selected` sentinel and the dispatcher `Ready→Running` writer, explicit empty-inflight paths/tests; the multi-task H6 mechanism is recommended to be deferred to a separate CDR as a material scope decision that T01 must explicitly accept and reconcile with — and must not erase — the user's accepted-in-principle H6 direction), E/T06 (`Identifier`-leaf `decl`, File `Enter` only after the committed TU via deterministic `parse`→`symbol_type` scheduling, derived namespace, chip-id allowlist, bounded committed-arena canonical-`TypeId` lowest-id scan with no cache), F/T07 (no `FunctionContextId` pending a T07 package amendment, `SemRecord` = committed `CheckedNode` + explicit committed typed T09 link, co-frozen conversion matrix incl. `FloatToFloat`/qualifier domain as a proposal, VF06 bound or explicitly out of M1, empty-only `EffectMask`), H/T09 (`Constant`/`Add`/`Return` op-table proposal, shorter-id alias choices, immediate-before-result, T08-computes/T09-emits folded `int5`; marker family left unresolved — no family selected, absent basis stated), and I/T01 (serialize only after owner acceptance, reconcile the hash-source contradiction, explicit numeric tags vs family ordinals, count/self-consistency tests); every recommendation is proposed for the named authority to accept/amend/reject, none is a sign-off/freeze/implementation authorization/user decision, and it explicitly does not withdraw the H6 direction. It defers the dependency/gate ordering to §9A rather than duplicating it. `/5` stays current, M1 stays DRAFT, ADR-0002 stays PROPOSED, A/B/C and the H1 allocation stay in-principle only, H6/H9 stay directions only, rows D–I remain PENDING, and all T01 integrator/owner sign-offs remain pending; this CDR becomes rev 40 while the shared M1 proposal stays rev 36 and the current-state M1 pointer/range stays rev 21–35.** **Rev 41 is a docs-only addition of the proposed per-decision response matrix in §9C (no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change; no task package, proposal, or ADR edited). It is a procedural response form only: per decision bundle (T01-global, C/H1, D, A, E, F, G, H) it names the exact authority and permits `Accept recommendation / Amend (state replacement) / Reject (reason) / Defer (dependency/date)` with a `Status / signature` cell that stays `PENDING` until that named human authority fills it; it cites §9A/§9B and the detailed §C–§I rows rather than duplicating technical content. Bundle A carries the explicit H6 scope question (preserve the user's in-principle H6 direction; any deferral requires explicit T01 acceptance and, if it would change the user's direction, returns to the user — the form presumes no outcome). The matrix is not schema and implies no sign-off: rows D–I remain PENDING. The protocol states only the named authority may fill its status/signature, a reviewer/model analysis never counts, and T01 records accepted shapes in `/6` only after owner sign-offs; until then `/5` is current and no code. `/5` stays current, M1 stays DRAFT, ADR-0002 stays PROPOSED, A/B/C and the H1 allocation stay in-principle only, H6/H9 stay directions only, all T01 integrator/owner sign-offs remain pending; this CDR becomes rev 41 while the shared M1 proposal stays rev 36 and the current-state M1 pointer/range stays rev 21–35.** **Rev 42 records the user's explicit 2026-10-05 decisions on selected subdecisions (user self-identifies as holding all named T01/owner roles and responds directly in each role; these are human authority responses for the selected subdecisions only). (1) A/T02/T13/T01 H6 scope: the user preserves the in-principle direction and chooses to freeze a quota>1-capable bounded batch recovery mechanism in `/6`, explicitly rejecting the §9B row-A recommendation to defer the H6 mechanism to a later CDR; the separately-selected mechanism is: pre-dispatch errors before state mutation leave tasks `Ready`; a semantic batch commit failure commits no semantic state; then a deterministic bounded recovery mutation processes the dispatched tasks once in dispatch order to `Failed`; no pre-reservation of N diagnostics; per-task diagnostic attempt with `DiagnosticId::NONE` if capacity is insufficient; a state guard prevents duplicate transition. This is a selected H6 mechanism only — it does **not** claim the whole A bundle, the H9 limits/stages/hash/error inventories, or any fixtures complete. (2) D/T02/T05/T01: accept the join as a commit-apply invariant with no new CT07 committed carrier/family, and await-all children terminal before the parent decision; the continuation fields/wire encoding, the `WaitSet` `/6` formal T01 §4 supersession, the `OwnBatch` exact preapply errors, the parse contract/tests, and the rest of the D bundle remain PENDING. (3) F/T06/T07/T09/T01: accept no `FunctionContextId`; `SemRecord` is the committed materialization of `CheckedNode`, one per `NodeId`, with an explicit committed typed link consumed by T09; the conversion matrix, VF06/T13, effects/value category, diagnostics/tests, and the remaining F bundle remain PENDING. (4) E/T06/T01 (T05 upstream dependency acknowledged): accept the deterministic bounded scan of committed `types.records` returning the lowest matching `TypeId` with no hidden cache/index; the bootstrap and the rest of the E bundle remain PENDING. This is a per-subdecision acceptance record only: authority entries carry role labels and the 2026-10-05 date, all unselected proposals remain proposal-only, every overall bundle status stays PENDING where other items are unresolved, and it authorizes no code and no `/6` freeze. `/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED, the broad rows remain PENDING, and no chip code is authorized. This CDR becomes rev 42 while the shared M1 proposal stays rev 36 and the current-state M1 pointer/range stays rev 21–35.** **Rev 43 records further explicit user decisions of 2026-10-05 on selected subdecisions only (user self-identifies as holding all named T01/owner roles; no accepted contract, no `/6` freeze, no code/chip authorization; shared M1 proposal stays rev 36, ADR-0002 stays Revision 16, current-state M1 pointer/range stays rev 21–35). (1) D/T05 continuation shape: accept the candidate `ContinuationRecord` exact ordered fields/shapes (`production: TaskKind`, `cursor: TokenId`, `context: ParseContext`, `binding_power: u16`, `scope: Option<ScopeId>`, `parent: Option<NodeId>`, `partial_children: Vec<NodeId>`, `next_child_ordinal: u32`, `previous: Option<ContinuationId>`), with **no `awaited`** and durable references to committed IDs only; the `RecordRef` numeric wire tags and `RecordFamily` ordinals are separate inventories (numeric encodings remain a T01 `/6` freeze detail) and the `ParseContext` vocabulary still needs the T05/T01 final encoding. (2) D/T01/T05: formally accept, in the `/6` inventory, the T01 §4 supersession of the old continuation-`awaited` wording — `TaskState::Waiting(WaitSet)` is the **only** awaited-child source, continuation has **no `awaited`**, and T01 §5 is unchanged (**no `/5` edit**). (3) D/T02/T05/T01 OwnBatch: accept the exact pre-apply proposal — a committed continuation exists/live; an OwnBatch continuation index lies within the same task's `AppendRecords` range and family `Continuation` (phase 1); a committed child is a valid committed child; an OwnBatch child index lies within the same task's Enqueue list (phase 2b, after the task's own Enqueues are known); errors `ContinuationRefInvalid`/`AwaitChildrenRefInvalid`, whole-batch `CommitError`, before any mutation (numerical codes/hash remain T01 `/6` details). (4) E/T06/T01 bootstrap: accept the File Enter trigger as a deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge **after** the committed TU, carrying the committed `NodeId`, with **no job-bootstrap**; T05's actual committed-TU carrier remains an upstream pending dependency and this stage-edge direction is not a TU schema signoff. Every overall D/E bundle status stays PENDING where other items are unresolved, and no `/6` freeze or implementation is authorized; this CDR becomes rev 43.** **Rev 44 records further explicit user decisions of 2026-10-05 on selected subdecisions only (user self-identifies as holding all named T01/owner roles; no accepted contract, no `/6` freeze, no code/chip authorization; shared M1 proposal stays rev 36, ADR-0002 stays Revision 16, current-state M1 pointer/range stays rev 21–35). (1) C/H1/T07/T08/T01 `required_kind` meaning: `ConstantRequest.required_kind` is a **per-use constant-expression requirement** (e.g. the M1 integer constant expression), distinct from the lexical `LiteralRecord.candidate_type`; it does not duplicate any implicit target type. `ConstantResult.legality` is a **result payload field** and adds **no** extra committed record family; the exact `ConstLegality` variants remain open. (2) G/T08/T01 `max_const_bits` origin: the origin is a **hashed `Limits` value** (the `limits.max_const_bits` member participates in the frozen-contract hash), with the M1 cap/default **128**; `config` **rejects** values **> 128**, and the bound reaches the zero-field T08 chip through an **explicit task-input projection** (accepted source/projection/bound subdecision). The **exact** wire field, diagnostic numeric code, and hash encoding remain `/6` details, and the accepted bound/cap is **not** a claim that every valid config value must equal 128. (3) C/T03/T01 artifact shape: accept `ArtifactRecord { kind, source, bytes, raw_offsets }` with the **total eight-`ArtifactKind`** scheme, where `Normalized`/`Spliced`/`CommentFree`/`Preprocessed` **require maps** and `Assembly`/`Object`/`Snapshot`/`Trace` **may omit** them; the M1 **exercised scope is only single-source `Normalized`** and the other producers and the multi-source map are deferred (selected M1 map shape/scope). The **exact** `raw_offsets` invariant/error mapping and all enum numeric codes remain open. Every overall C/G bundle status stays PENDING where other items are unresolved, and no `/6` freeze or implementation is authorized; this CDR becomes rev 44.** **Rev 45 records further explicit user decisions of 2026-10-05 on selected C/G subdecisions only (user self-identifies as holding all named T01/owner roles; no accepted contract, no `/6` freeze, no code/chip authorization; shared M1 proposal stays rev 36, ADR-0002 stays Revision 16, current-state M1 pointer/range stays rev 21–35). (1) C/T04/T08/T01 `LiteralRecord` exact ordered fields: `token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` (big-endian magnitude), `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`, with no `node`/`required_kind`. (2) C/T04/T08/T01 M1 enum/scope: `LiteralKind {Integer, Character, String}` (only `Integer` produced in M1); `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced); radix domain {2, 8, 10, 16} (M1 decimal only); `Lx08CandidateType` is symbolic/target-independent with M1 literals 2 and 3 as `Int` and no bit width — its complete member set/numeric encodings remain open (not invented here). (3) C/T07/T08/T01 `RequiredKind`/`ConstLegality`: `RequiredKind` M1 enum only `IntegerConstantExpression`; `ConstLegality` result-field values `Legal`, `NotConstantExpression`, `Unsupported`; future C constant-expression purposes require appended variants/new rules, not repurposing the lexical candidate type. (4) C/T03/T01 mandatory-map invariant: accept `raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, and mandatory kinds require a valid source. The optional-kind map rule, source-versus-payload equality, and exact artifact error classification/numeric codes remain open because the prompt allowed either an empty optional map or another explicit rule and the user made no choice between them. Every overall C/G bundle status stays PENDING where other items are unresolved, and no `/6` freeze or implementation is authorized. **Rev 46 (user, 2026-10-05) records further F-row per-subdecision acceptances only:** F/T07 `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` with the M1 fixture producing `NonLvalue` and M1 allowing only `EffectMask(0)` (nonzero is a typed unsupported/diagnostic, bit classes reserved/unassigned); F/T07/T13/T01 VF06 `TypedAstInvariant` **is in M1** (executes after committed T07 `SemRecord`s and before T09 lowering, checking M1 typed-fact/required-conversion completeness; exact registered task kind/stage/phase/interface still T01/T13 co-freeze/open); and F/T06/T07/T09/T01 selects the **M1-minimal conversion scope only** (freeze only the M1-fixture-needed conversion behavior including identity/no-conversion; integer promotions, float conversions incl. `FloatToFloat`, pointer qualifier, and other non-M1 conversions are explicitly unsupported/deferred to a later append/contract revision; no `ConversionOp`/`ConversionRole` closed variant list, pairing, role→chip mapping, or numeric encoding is invented here — all remain open `/6` co-freeze details). Every overall F bundle status stays PENDING and rows C–I remain pending; no `/6` freeze or implementation is authorized; this CDR becomes rev 46.** **Rev 47 (2026-10-05; no accepted contract, no `/6` freeze, no owner or T01 `[INT]` signoff, no code/chip authorization; no task package, proposal, or ADR edited; shared M1 proposal stays rev 36, ADR-0002 stays Revision 16, current-state M1 pointer/range stays rev 21–35).** Under the user's explicit **2026-10-05 delegation** ("for non-critical decisions adopt the recommended choice directly; ask only for critical decisions"; per the user's current instruction), the **M1 integration agent** selected — **under user-delegated integration default, not as an owner or T01 `[INT]` signoff** — four **low-risk candidate defaults** for the remaining open items: **(1) optional `ArtifactKind` map policy** — if `requires_map(kind) == false`, `raw_offsets` **must be empty**; `source` stays `Option<SourceId>` and must be valid when `Some`; mandatory-map kinds require a valid `source` plus the accepted rev-45 invariants; **no** requirement that the source payload bytes equal the artifact bytes (normalization transforms); any exact source-provenance/equality rule is **deferred**; exact numeric error codes stay open; **(2) T05 parse depth** — reuse `limits.max_task_depth`, count **parser continuation/child frames only** (not total bus tasks), detect the limit **before any child enqueue for the descent**, and treat excess as the `ParseDepthExceeded` **chip diagnostic**; the exact `ParseContext` encoding and parse request/result variants stay pending; **(3) T06 namespace/symbol lookup** — derive a **closed namespace from `SymbolKind`** (`Object`/`Function`/`Typedef`/`EnumConst` → `Ordinary`; `StructTag`/`UnionTag`/`EnumTag` → `Tag`; `Label` → `Label`) with **no namespace field**, wrong-namespace lookup is a **miss not a conflict**, and the deterministic **TY03** lookup walks the **active scope chain only**, ordering candidate declarations by `(source,start,end,NodeId)` and picking the greatest ≤ the query point, with a same-scope tie going to the higher `SymbolId` and otherwise the innermost active scope; the T05 `NodeKind`/token-range dependency and the exact event encoding/lifecycle/allowlist rows stay **remaining co-freeze**; **(4) T09 noncritical rule defaults** — choose the **shorter** aliases `ir.op-immediate-type` and `ir.terminator-missing`; for `Constant` with **both** immediate and result missing validate the **immediate first**; the target type equals the result `ValueRecord.ty`; all remain **prospective `/6`** with owner/T01 acceptance and hashing pending, and the `CompletedFunction` marker family stays **unresolved with no family invented**. These selections are **selected under user-delegated integration default** dated **2026-10-05**; they are **not** evidence of T03/T05/T06/T09 owner signoff (unless the user separately already accepted that exact subdecision) and **not** T01 `[INT]` acceptance, and all public/shared schema awaits owner/T01 co-freeze. Overall rows C/D/E/H remain PENDING; no conflicting user-selected rev 42–46 decision is overridden. `/5` remains current; M1 remains DRAFT; ADR-0002 remains PROPOSED. This CDR becomes **rev 49** while the shared M1 proposal stays **rev 36** and the current-state M1 pointer/range stays **rev 21–35**. **Rev 50–53 later revisions (read the CDR body/§12 for the operative record):** rev 50 records the user's two-tier hash-scope decision; rev 51 the integration-selected sole-per-tick-bound candidate default; rev 52 the read-only cross-owner audit/reconciliation; and **rev 53 records the integration-agent selected candidate default under explicit user delegation (2026-10-05)** for the **M1 exercised literal subset** — `Lx08CandidateType` M1 vocabulary = the closed one-member set **`{ Int }`**, M1 literals `2`/`3` = **target-independent `Int`, no bit width**; forms **outside** the exercised M1 subset must **not silently default to `Int`** and are **explicitly unsupported/deferred** until their categories/rules exist; future categories **append without reinterpretation**; **not** an owner/T04 or T01 `[INT]` signoff, **not** a `/6` freeze, **not** the complete future C candidate set, no numeric tags; the complete `Lx08CandidateType` member set/numeric encodings remain open. M1 proposal **rev 40** (current; revs 38/39 historical), **ADR-0002 Revision 17**, **T02 rev 35** (as of 2026-10-05; see its revision record); `/5` current; M1 DRAFT; no code/freeze; rows **C–I PENDING overall**. **Rev 55 records the OPEN-03 T01/T07/T08 constant-expression handoff co-freeze (frozen `ConstantRequest::Literal`/`Binary` M1 variants; committed expression/operand/operator input path; the real T07→T08→T09 fixture `M1-CL-05`; proposal rev 40), while the exact wire tags/numeric codes/result-envelope spelling remain `/6`.** |
| **Scope** | Compiler application only (`compiler/`, crate `cc-silicon-compiler`). Root `cc-silicon` framework, SFL core, ADR-0001, and `/5` are untouched. The user acceptance is **doc-level only** for the revision working basis; it does not edit `/5`, add a chip, or turn on the pipeline. |
| **Related requests** | [Guardrails](COMPILER_DEVELOPMENT_GUARDRAILS.md) (accepted policy), [ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) (PROPOSED), [M1 Part A proposal rev 40](M1_PART_A_CONTRACT_PROPOSAL.md) (DRAFT, expected rev 40, not freeze-ready; §17/§18/§19/§20/§21 finding ledgers, plus the rev-38 §23 and rev-39 §24 ledgers and the rev-40 OPEN-03 co-freeze record) |

This is a **decision request**, not an accepted contract, not an ADR, and not a
freeze. It does **not** authorize `/6`, any bus/schema edit, any chip wave, or any
code. It exists because the accumulated rev 18–20 reviews and the accepted
[Compiler Development Guardrails](COMPILER_DEVELOPMENT_GUARDRAILS.md) leave a set
of shared-interface and ownership decisions that cannot be made unilaterally by a
group owner and cannot be safely defaulted.

Authority order is unchanged (`AGENTS.md` §2): explicit user instruction >
accepted ADR > accepted engineering contract > architecture design > implementation.
The [Guardrails](COMPILER_DEVELOPMENT_GUARDRAILS.md) sit at the user-instruction
level for their policy; where they describe a shared interface they are
subordinate to the applicable accepted contract until it is amended. This CDR
would amend that contract only after the acceptance record below is filled.

**Non-claims.** The compiler is not implemented; there is no parser, IR, codegen,
probe, or pass rate. [ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md)
is marked **PROPOSED** throughout and authorizes nothing. No owner review has
signed; every owner position below is **conditional**. No throughput claim exists
and quota `> 1` is not authorized here. The `sources.spans` dual-writer candidate
is **rejected as non-conforming** (see §B).

**User acceptance in principle (2026-10-04, explicit user instruction).** The
user selected the following as the **rev-21 `/6` contract-revision working
basis** (decision A/B/C direction); this is a decision to *prepare a revision*,
not a freeze. It does **not** accept rows D–I, does **not** accept the exact
field/enum/rule/hash shapes, and does **not** authorize code or chips:

1. **One writer per store/field; no shared-writer carveout.** `sources.spans` has
   the **single owner T03** (source/PP boundary). T04/PpToken/Token **reuse
   committed T03 PP spans and write no spans**; AST `Node` records a **first/last
   `TokenId` range** and T05 writes **no** `sources.spans`. T03 is also the sole
   owner of `sources.expansions`. The Guardrails §2.7 one-owner rule stands
   unamended.
2. **Deterministic bounded sequential stage pipeline.** Cross-tick work is an
   explicit staged pipeline over committed IDs, executed **sequentially** by the
   single CPU backend with **one ordered atomic commit per tick**. `quota = 1`
   (semantic-comparison projection) is the correctness baseline; `quota > 1` is
   permitted **only after measured before/after tick counts and separate
   integrator acceptance**.
3. **T04→T08 typed handoff via a committed T04-owned `LiteralRecord`.** The
   handoff must preserve the **semantic information** `node`, `required_kind`,
   `legality`, and the `LX08` candidate type as explicit **co-freeze field
   requirements**. **T08 is the sole `constants.records` writer**; T04 never
   writes `constants.records`. On **2026-10-04 the user also explicitly accepted
   the recommended allocation split in principle (`接受拆分（推荐）`)**: the
   committed T04-owned `LiteralRecord` carries the **per-literal lexical facts +
   the `LX08` candidate type**; the post-parse sem-stage `ConstantRequest` carries
   the **per-use `node`/`required_kind`**; and the `ConstantResult` carries
   **`legality`**. This is recorded as the **accepted-in-principle `/6` revision
   working basis** — **not** a freeze and **not** code/chip authorization; the
   exact variants/schema and T01 integrator acceptance plus T03/T04/T08 owner
   co-freeze/sign-off remain pending (see §C2 and §9).

The acceptance record (§10) marks exactly these three rows as **accepted in
principle**; it does **not** mark all of A–I accepted.

---

## 0. Required CDR fields (guardrails §3.4) — location index

| Required field | Where addressed |
|---|---|
| ID/title, date, author, target version/hash | Header table + §12 |
| Concrete requirement/motivation and smallest sufficient change | §1, §2 |
| Exact current vs proposed shape of every affected shared interface/store/field/enum/schema/file | §3 |
| Affected chips, task kinds, manifests, dependent tests to rerun | §7 |
| Read/write manifest, phase, ownership impact | §3, §B, §7 |
| Determinism/replay and snapshot/hash impact | §5 |
| Chip-local alternative considered and why insufficient | §6 |
| Migration/freeze plan, `/6` inventory item | §8 |
| Acceptance record (partial, in principle) | §10 |
| User in-principle acceptance + pending owner amendment requests | §10, §13 |

---

## 1. Motivation

Rev 18–20 converged on a **candidate** shared schema for M1 Part A
([M1 Part A proposal](M1_PART_A_CONTRACT_PROPOSAL.md)) plus a **PROPOSED**
pipeline scheduler ([ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md),
[M1 §6.2.1/§10.5/§12.18](M1_PART_A_CONTRACT_PROPOSAL.md)). The candidate is
explicitly **DRAFT — not freeze-ready**. Three classes of problem remain:

1. **Ownership conflict with an accepted guardrail — resolved by user decision
   to the single-owner model.** The rev-20 candidate selected a **shared writer**
   for `sources.spans` (`T03` **and** `T04`, M1 §15.1 item 4 / §8). The accepted
   [Guardrails §2.7](COMPILER_DEVELOPMENT_GUARDRAILS.md) require **exactly one
   owner per store/field/record family** and call a second writer a
   `StoreOwnerViolation` class defect. A shared-writer carveout therefore cannot
   be defaulted by the integrator. On 2026-10-04 the **user selected the
   single-owner model in principle**: `sources.spans` is **T03-only**, T04/Token
   writers reuse committed T03 spans and write none, and AST `Node` uses a
   first/last `TokenId` range with **no T05 span write**. The guardrail is
   **not** amended. This CDR still records the rejected alternatives (§B) and
   keeps the guardrail unamended.

2. **Cross-group shared shapes that no single owner may invent.** The M1 candidate
   adds IDs, `RecordRef`/`RecordFamily` variants, `StoreId::Names`, new arenas,
   new `Proposal`/`ResultValue` variants, an `M1AppendSchema` **proposed to be
   hashed at `/6`** (not hashed today — rev 25 F1/F8), and pipeline
   registers. These are integrator-owned shared interfaces
   ([PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) §2). Group owners may submit
   requests but may not create private incompatible versions.

3. **Unresolved protocol inside the candidate.** The rev-20 candidate itself states
   that the T05 own-batch continuation/`AwaitChildren` protocol, the T07→T09
   function handoff, the T03/T04→T08 literal handoff, the artifact-map shape, the
   `ConstantRequest` shape, and the pipeline fairness/backpressure/metrics are
   **owner/`/6` blockers**, not settled. This CDR lists them as explicit decision
   points rather than pretending they are resolved.

The smallest sufficient change is: **decide ownership and shared shapes first**
(§A–§I), then freeze them in one `/6` envelope with the `M1AppendSchema`
(§8, §11). Until then, the candidate stays draft text and `/5` stays current.

---

## 2. What this CDR asks for (summary dashboard)

| # | Decision | Recommended default | Decision authority | Freeze impact | User 2026-10-04 |
|---|---|---|---|---|---|
| A | Pipeline scheduling model, stage set, fairness/backpressure/cancel/metrics, quota policy | Adopt ADR-0002 shape, **inert**, quota default 1; keep `<5>` semantics until `/6` | Integrator; **user** for quota policy/direction | New bus/task/report/limits snapshot surface | **Accepted in principle (direction only)**: deterministic bounded **sequential** staged pipeline; `quota = 1` semantic-projection baseline; `quota > 1` only after measurement + integrator acceptance. Exact stage set/fairness/backpressure/metrics/errors remain open. **§6.1 own-batch narrow interpretation also accepted in principle (user decision A, 2026-10-04):** a same-task `OwnBatch(DraftRef)` may exist only as a **transient wire/proposal input before commit**; commit validates/resolves it to a committed `TaskId` before persistent state; no durable cursor/`WaitSet`/join may point at a draft/wire/address; guardrail text **unamended**; **T01 integrator implementation-confirmation pending**. **H6 batch-failure recovery direction also accepted in principle (user, rev 34, 2026-10-04):** on a failed atomic semantic batch commit, every dispatched task fails **exactly once** in **dispatch order** (a committed `DiagnosticId` when capacity allows, else the `DiagnosticId::NONE` sentinel), so every task leaves `Running`; clearing the in-flight set is **not itself** a transition — generalizing the verified `/5` single-task sentinel semantics. **Exact atomic/bounded implementation requires T01 integrator approval; batch failure-atomicity details, diagnostic budget/state, and T02/T13 owner fixtures/sign-offs remain co-freeze/pending; mechanism not implemented/frozen.** **H9 in-flight scheduling ownership direction also accepted in principle (user, rev 35, 2026-10-04):** the user accepts **removing `max_inflight_total`** from the `/6` candidate because sequential per-tick dispatch is already bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; `Waiting` is not in-flight; and a future cross-tick `Running` mode would need a separate CDR. The dispatch batch bound stays the per-tick `max_inflight_per_tick` quota; `tasks.in_flight` is an ephemeral per-tick scheduler batch only (clears at latch after each dispatched task has a terminal/`Waiting`/`Progress` outcome or H6 recovery); the dispatcher's `Ready→Running` pre-worker mutation is distinguished from the ordered atomic semantic commit. **Pending T01 integrator acceptance; no bound/config/hash frozen.** **Rev 49 read-only H9 audit (unresolved blockers, no change to the selected H6 mechanism or the H9 removal direction):** `/5` proves only the quota=1 single-task `fail_selected` transition, so the proposed quota>1 pipeline is **unimplemented**; H9 still needs the exact dispatcher pre-worker `Ready→Running`/`in_flight`-population relationship to the single ordered atomic commit, the exact `in_flight` clear ownership/order relative to the bounded H6 recovery, and a proof that no `Running`/residual set remains at latch for **all** success/error/empty-proposal paths; an **internal proposed-limit conflict** (`max_dispatches_per_tick` is listed/checked separately from `max_inflight_per_tick` although the latter is described as the sole dispatch bound) must be resolved in `/6` and is **not** decided here; and the H6/H9 fan-out fixtures plus T13 VF02/VF03/VF04/VF13 remain pending. The H9 no-residual guarantee is **conditional on the H6 recovery implementation**, and the quota=1 M1 baseline stays **separated** from the quota>1 optimization. No freeze/code; exact stages/version carrier/errors/hashes remain pending. **Rev 51 integration-selected candidate default under user delegation (2026-10-05; not owner/T01 `[INT]` signoff):** `max_inflight_per_tick`/quota is the **sole per-tick dispatch-count bound**; the redundant `max_dispatches_per_tick` is **dropped from the candidate limit inventory and validation** (grounded in the rev-35/38 H9 direction and the rev-49 audit duplication finding; T01/T02 must confirm/co-freeze exact names/defaults/codes at `/6`, T02/T13 tests pending). |
| B | One writer per store/field (reject `sources.spans` dual owner) | Single owner per field; other groups reuse **committed** records (no span-service task needed) | **User** (guardrail) + integrator + T03/T04/T05 owners | Removes the shared-writer carveout; no new field | **Accepted in principle**: one writer per field; `sources.spans` = **T03 only**; T04/Token reuse committed T03 PP spans, no span writes; AST `Node` = first/last `TokenId` range, no T05 span write; guardrail unamended. **Rev 47 delegated candidate default:** the optional-`ArtifactKind` map policy (map-optional kinds have empty `raw_offsets`; `source` valid when `Some`; no source-payload-equals-bytes requirement; source-provenance/equality rule deferred) is **selected under user-delegated integration default (2026-10-05)** — **not** a T03/T04 owner signoff, pending final `/6` co-freeze. |
| C | T03/T04/T08 artifact + literal handoff | Single T08 `constants.records` writer via a committed literal record | T03/T04/T08 owners + integrator | New `lex.literals` field, `ArtifactKind` total set | **Accepted in principle (handoff direction + H1 allocation split, 2026-10-04)**: committed T04-owned `LiteralRecord` (per-literal lexical facts + `LX08` candidate type); sem-stage `ConstantRequest` (`node`/`required_kind`); `ConstantResult` (`legality`); typed handoff; T08 sole writer. **H1 allocation is an accepted-in-principle working basis — not a freeze, not code/chip authorization**; exact variants/schema and T01 + T03/T04/T08 owner co-freeze/sign-off pending. **Further selected subdecisions (user, rev 44, 2026-10-05; no freeze, overall C still PENDING):** **C/H1/T07/T08/T01** — `ConstantRequest.required_kind` is a **per-use constant-expression requirement** (e.g. M1 integer constant expression), distinct from the lexical `LiteralRecord.candidate_type`, and duplicates no implicit target type; `ConstantResult.legality` is a **result payload field** with **no** extra committed record family (exact `ConstLegality` variants open); **C/T03/T01** — accept `ArtifactRecord { kind, source, bytes, raw_offsets }` with the total eight-`ArtifactKind` scheme (Normalized/Spliced/CommentFree/Preprocessed require maps; Assembly/Object/Snapshot/Trace may omit), M1 exercised scope only single-source `Normalized`, other producers/multi-source map deferred (exact `raw_offsets` invariant/error mapping and enum numeric codes open). **Further selected subdecisions (user, rev 45, 2026-10-05; no freeze, overall C still PENDING):** **C/T04/T08/T01** — the exact ordered `LiteralRecord` fields `token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` (big-endian magnitude), `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType` (no `node`/`required_kind`), the M1 enum/scope `LiteralKind {Integer, Character, String}` (only `Integer` produced), `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced), radix domain {2,8,10,16} (M1 decimal only), and symbolic `Lx08CandidateType` with M1 literals 2/3 as `Int` and no bit width; **C/T07/T08/T01** — `RequiredKind` M1 enum only `IntegerConstantExpression` and `ConstLegality` result values `Legal`/`NotConstantExpression`/`Unsupported`; **C/T03/T01** — mandatory-map invariants `raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds require a valid source. **Still open (rev 45):** the `Lx08CandidateType` complete member set/numeric encodings (not invented), the optional-kind artifact-map rule, source-versus-payload equality, and the exact artifact error classification/numeric codes. **Rev 53 delegated candidate default (2026-10-05; integration-agent selected under explicit user delegation, not an owner/T04 or T01 `[INT]` signoff, not a freeze):** for the exercised M1 literal subset the `Lx08CandidateType` M1 vocabulary is the **closed one-member set `{ Int }`**, with M1 literals `2`/`3` = **target-independent `Int`, no bit width**; forms **outside** the exercised M1 subset must **not silently default to `Int`** and are **explicitly unsupported/deferred**; future categories **append without reinterpretation**; **not** the complete future C candidate vocabulary, no numeric tags; the complete `Lx08CandidateType` member set/encodings remain open; overall C stays PENDING |
| D | T05 continuation/join protocol | Own-batch refs resolved at commit; single awaited-child representation | T05 owner + integrator | Continuation fields, `AwaitChildren`, task states | **PENDING overall.** **Sub-decisions accepted by the user, 2026-10-04:** (A) same-task `OwnBatch(DraftRef)` is transient wire/proposal input only, resolved to committed IDs at commit; (B) `TaskState::Waiting(WaitSet)` is the sole awaited-child-ID source and `ContinuationRecord` carries no duplicate `awaited`, superseding T01 §4 in `/6` (no `/5` edit). **Further sub-decisions accepted by the user, 2026-10-05 (rev 43):** (C) the candidate `ContinuationRecord` **exact ordered fields/shapes** (`production: TaskKind`, `cursor: TokenId`, `context: ParseContext`, `binding_power: u16`, `scope: Option<ScopeId>`, `parent: Option<NodeId>`, `partial_children: Vec<NodeId>`, `next_child_ordinal: u32`, `previous: Option<ContinuationId>`), no `awaited`, durable committed-ID references only; (D) the formal T01 §4 supersession recorded `/6`-only; (E) the exact OwnBatch pre-apply validation (committed continuation exists/live; OwnBatch continuation index within the same task `AppendRecords` range and family `Continuation` phase 1; committed child valid; OwnBatch child index within the same task Enqueue list phase 2b) with whole-batch `CommitError` before any mutation. **Still PENDING:** `RecordRef` wire-tag values and `RecordFamily` ordinals (separate inventories; numeric encodings a T01 `/6` freeze detail), the `ParseContext` vocabulary final encoding, the OwnBatch numerical error codes/hash, and the parse request/result/test shapes. **Rev 47 delegated candidate default:** T05 parse depth reuses `limits.max_task_depth`, counts parser continuation/child frames only, detects the limit before any child enqueue for the descent, and reports excess as the `ParseDepthExceeded` chip diagnostic (selected under user-delegated integration default, 2026-10-05; **not** a T05 owner signoff; exact `ParseContext` encoding/request-result variants still pending). |
| E | T06 scope/symbol/type details | Point-of-declaration tuple; cumulative scope lifecycle; chip-keyed int producer | T06 owner + integrator | `symbols.scope_events`, `types.records` allowlist | **Pending overall** (exact `decl` node/lifecycle/allowlist seed open). **Sub-decision accepted by the user, 2026-10-05 (rev 43):** the File Enter trigger is a deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge **after** the committed TU, carrying the committed `NodeId`, with **no job-bootstrap**. **Still PENDING:** T05's actual committed-TU carrier is an upstream dependency and acceptance of the stage-edge direction is **not** a TU schema signoff; the `ParseContext`-adjacent bootstrap wiring and the rest of the E bundle remain open. **Rev 47 delegated candidate default:** T06 namespace/symbol lookup derives a closed namespace from `SymbolKind` (`Object`/`Function`/`Typedef`/`EnumConst` → `Ordinary`; `StructTag`/`UnionTag`/`EnumTag` → `Tag`; `Label` → `Label`) with no namespace field, wrong-namespace lookup is a miss not a conflict, and the deterministic TY03 lookup walks the active scope chain only (order candidates by `(source,start,end,NodeId)`, pick the greatest ≤ query point; same-scope tie higher `SymbolId`, else innermost active scope) — selected under user-delegated integration default (2026-10-05), **not** a T06 owner signoff; T05 `NodeKind`/token-range dependency and exact event encoding/lifecycle/allowlist rows remain co-freeze. |
| F | T07 committed checked facts + conversion plan alignment | T07-owned committed carrier (no T09 `FunctionRecord` cycle); one shared conversion-plan type | T07 + T06 + T09 owners + integrator | `sem.records`, conversion types | **Pending overall** (exact carrier + conversion matrix remain hard owner blockers). **Subdecisions selected (user, rev 42, 2026-10-05; no freeze):** **F/T06/T07/T09/T01** — no `FunctionContextId`; `SemRecord` is the committed materialization of `CheckedNode`, one per `NodeId`, with an explicit committed typed link consumed by T09. **Further subdecisions selected (user, rev 46, 2026-10-05; no freeze, overall F still PENDING):** **F/T07** — `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }`; M1 fixture produces `NonLvalue`; M1 allows only `EffectMask(0)` (nonzero is a typed unsupported/diagnostic; effect bit classes reserved/unassigned). **F/T07/T13/T01** — VF06 `TypedAstInvariant` **is in M1**, executing after committed T07 `SemRecord`s and before T09 lowering, checking M1 typed-fact/required-conversion completeness (exact registered task kind/stage/phase/interface still T01/T13 co-freeze/open). **F/T06/T07/T09/T01** — select the **M1-minimal conversion scope only** (freeze only M1-fixture-needed conversion behavior incl. identity/no-conversion; integer promotions, float conversions incl. `FloatToFloat`, pointer qualifier, and other non-M1 conversions are explicitly unsupported/deferred to a later append/contract revision); no `ConversionOp`/`ConversionRole` closed variant list, pairing, role→chip mapping, or numeric encoding invented here — remain open `/6` co-freeze details. **Still PENDING:** VF06 actual registration/task kind/stage/phase/interface and all conversion/effect encodings. |
| G | T08 const diagnostics/bounds/CL02-vs-`M1-CL-02` | Chip diagnostics; symbolic Part A; no `/7` | T08 owner + integrator | `max_const_bits`, `ConstRecord` | **Pending overall** (exact formula/bounds/enforcement open). **Subdecisions selected (user, rev 44, 2026-10-05; no freeze):** **G/T08/T01** — `max_const_bits` origin is a **hashed `Limits` value** (`limits.max_const_bits` participates in the frozen-contract hash), M1 cap/default **128**, `config` **rejects** values **> 128**, and the bound reaches the zero-field T08 chip via an **explicit task-input projection**. Exact wire field/diagnostic numeric code/hash encoding remain `/6`; the accepted bound/cap does **not** require every valid config value to equal 128. **Still PENDING:** the exact formula/enforcement split, symbolic-vs-probe gating, and remaining G items. **Rev 45 selected subdecisions (user, 2026-10-05; no freeze, overall G still PENDING):** **C/T07/T08/T01** — `RequiredKind` M1 enum only `IntegerConstantExpression`; `ConstLegality` result values `Legal`/`NotConstantExpression`/`Unsupported`; future C constant-expression purposes require appended variants/new rules, not repurposing the lexical candidate type |
| H | T09 IR op table/terminator rules | **Proposed** per-op rules, **not hashed today**; to be hashed at `/6` (rev 25 F8/rev 26); **rev 49: reuse the committed IR28 `FunctionEnd` terminal-result fact (no new marker)** | T09 owner + integrator | `ir.*` records, IR rules | **Pending** (H11: **rev 49 selected the `FunctionEnd`-terminal-result direction** — the `TerminatorMissing` trigger reuses the committed terminal result of the IR28 `FunctionEnd` task (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic completion fact, checked by a T01-owned typed phase-2b commit-apply validation hook; **no** `CompletedFunction` record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder or new `ResultValue` variant; the earlier new-marker direction is **superseded for operative direction** but preserved as history. The hook is **not** a frozen `/6` hook/schema and authorizes no code; exact hook contract, hash impact, result typing/commit ordering, and T09/T01 co-freeze remain open. Other unresolved items: the rejection→rule-id inventory incl. both `ir.op-immediate-type` aliases and both `ir.terminator-missing*` candidates, and the `Constant` missing-immediate-vs-result precedence. Exact rules are prospective `/6`, not frozen; overall H stays pending). **Rev 47 delegated candidate default:** choose the shorter aliases `ir.op-immediate-type` and `ir.terminator-missing`, validate the immediate before the result when `Constant` is missing both, and set the target type equal to the result `ValueRecord.ty` — selected under user-delegated integration default (2026-10-05), **not** a T09 owner signoff or T01 `[INT]` acceptance; all prospective `/6`, hashes/owner acceptance pending. |
| I | Cross-doc consistency, C05 snapshot, `M1AppendSchema`, error families, T01 §4 supersession | Doc-only reconciliation; no `/5` edit | Integrator | Hash scope + version | **Pending.** **Rev 48 read-only audit:** the `/5` code facts are **24** `RECORD_KINDS` / **24** `RecordRef` variants (tags 0–23), `RecordFamily` **absent**; the `M1AppendSchema` hash-scope conflict between `CONTRACT_VERSION`/`contract.rs`/README (`group-declared-store-fields` excluded) and `COMPILER_SFL_MANIFEST.md` §4 is **confirmed real and unresolved**; the draft 19/27/27 counts are **proposed/unverified**; the three hash-scope options are **unselected**, no recommendation, no freeze. **Rev 49:** records the user's explicit 2026-10-05 critical H11/T09/T01 direction (reuse the committed IR28 `FunctionEnd` terminal result, no new marker family/ID/arena/tag/ordinal/encoder/`ResultValue`; T01-owned typed phase-2b commit-apply validation hook; not a frozen `/6` hook, no code) and the read-only H9 audit findings as unresolved blockers (quota>1 pipeline unimplemented; `Ready→Running`/`in_flight`-commit relationship and clear ownership/order open; no-`Running` proof for all paths; `max_dispatches_per_tick`-vs-`max_inflight_per_tick` conflict deferred to `/6`; fan-out fixtures/VF02/VF03/VF04/VF13 pending); the earlier new-marker direction is **superseded for operative direction** and preserved as history; no hash-scope option selected; no freeze. **Rev 50 (user `[USER]`, 2026-10-05, critical hash-scope decision):** the user **accepts the two-tier hash-scope model** — `StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed** and **participates in the `/6` contract hash**; **post-seed runtime `StoreSchema::declare()` extensions stay excluded** from the frozen hash and are captured/validated through **runtime snapshot/schema mechanisms**. At `/6` integration T01 must **atomically** update the normative `COMPILER_SFL_MANIFEST.md` §4 wording, the **`hash_excludes` semantic description/token** (now scoped to **post-seed runtime declarations**, not the frozen M1 seed), and `FrozenSchema::encode`/contract code plus the freeze test, **preserving the frozen `/5` hash and history**. **No acceptance of exact `M1AppendSchema` contents/counts, no freeze, no code, no `/5` change:** the numeric inventory and test implementation remain pending; **T01 still must co-freeze the M1 seed values after the owners**. Row I and the overall CDR remain **PENDING**. **Rev 51 integration-selected candidate default under user delegation (2026-10-05; not owner/T01 `[INT]` signoff):** the duplicate `max_dispatches_per_tick` limit is **dropped** — `max_inflight_per_tick`/quota is the sole per-tick dispatch-count bound (per the rev-35/38 H9 direction and the rev-49 audit duplication finding); no exact names/defaults/codes accepted, T01/T02 confirm/co-freeze at `/6`, T02/T13 tests pending. |

**Legend:** `[USER]` = user amendment/acceptance required; `[INT]` = T01
integrator acceptance; `[OWNER]` = named group-owner sign-off (all conditional);
`[DOC]` = documentation reconciliation.

**Rev 23–54 dashboard note (global consistency).** Only **A/B/C** (plus the rev-31 A-narrow/B sub-decisions recorded under rows A and D, the rev-35 H9 direction under row A, and the **rev-50 user hash-scope decision under row I**) carry the user's
2026-10-04 in-principle **direction**; they are **not accepted contracts** and their
exact shapes remain open. For **C**, the user separately accepted the **H1
allocation split in principle** on 2026-10-04 as the `/6` revision working basis
(committed `LiteralRecord` = per-literal lexical facts + `LX08` type; sem-stage
`ConstantRequest` = `node`/`required_kind`; `ConstantResult` = `legality`); that is
still **not a freeze**, and the exact schema plus owner sign-off remain pending.
**Rev 34** additionally records the **H6 batch-failure recovery direction** as
**accepted in principle** (row A): every dispatched task fails exactly once in
dispatch order with the optional-diagnostic/`DiagnosticId::NONE` sentinel, so every
task leaves `Running`; clearing the in-flight set is not itself a transition. That
is a **direction only** — the exact atomic/bounded implementation requires **T01
integrator approval** and the batch failure-atomicity details, the diagnostic
budget/state mechanism, and the T02/T13 owner fixtures/sign-offs remain co-freeze
and pending. **Rev 35** additionally records the **H9 in-flight scheduling
ownership direction** as **accepted in principle** (row A): remove
`max_inflight_total` from the `/6` candidate because sequential per-tick dispatch
is already bounded by `max_inflight_per_tick`/quota, the stage queues, and
`max_tasks_total`; `Waiting` is not in-flight; a future cross-tick `Running` mode
needs a separate CDR. The dispatch batch bound stays the per-tick
`max_inflight_per_tick` quota and `tasks.in_flight` is an ephemeral per-tick
scheduler batch only; that is a **direction only** — pending **T01 integrator
acceptance**, no bound/config/hash frozen, and the remaining H9 `[INT]` items (the
dispatcher-`Ready→Running`-vs-ordered-atomic-commit relationship and the
residual-set semantics) stay open. **C–I remain PENDING** (C and G carry selected
partial subdecisions only). The dashboard must not
be read as acceptance
of anything but the A/B/C direction (plus the C allocation split, the H6/H9
directions, the rev-42 H6 scope+mechanism and D/F/E subdecisions, the rev-43
D continuation-shape/T01-§4-supersession/OwnBatch-preapply and E File-Enter
subdecisions, and the rev-44 C `required_kind`/`legality` + artifact shape,
T03/T01 and G `max_const_bits` origin/cap/projection subdecisions, plus the rev-45
C `LiteralRecord` exact-ordered-fields/enum-scope + `Lx08CandidateType`
symbolic/no-bit-width selections, the C/T07/T08/T01 `RequiredKind`/`ConstLegality`
values, the C/T03/T01 mandatory-map invariants, the rev-46 F
`ValueCategory`/`EffectMask` + VF06-in-M1 + M1-minimal-conversion-scope
selections, and the rev-47 **integration-agent selected candidate defaults under
explicit user delegation** on the optional-artifact-map, T05 parse-depth, T06
namespace/lookup, and T09 noncritical rule items — the last of which are **not**
owner or T01 `[INT]` sign-offs), and the **rev-48 read-only T01 audit findings**
(the current `/5` code fact of **24** `RECORD_KINDS` / **24** `RecordRef` variants,
`RecordFamily` **absent**; the unresolved `M1AppendSchema` hash-scope conflict; and
the draft 19/27/27 counts marked **proposed/unverified** — docs-only, no option
selected, no freeze). **The rev-49 updates** add the **user's explicit 2026-10-05
critical H11/T09/T01 direction** (the `TerminatorMissing` trigger reuses the
committed IR28 `FunctionEnd` terminal result — `TaskState::Completed(ResultId)` with
task kind `FunctionEnd` — as the deterministic function-completion fact, checked by a
T01-owned typed phase-2b commit-apply validation hook; **no** new
`CompletedFunction` record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/
snapshot encoder or new `ResultValue` variant; the earlier new-marker direction is
**superseded for operative direction** but preserved as history; not a frozen `/6`
hook/schema, no code authorization; the exact hook contract, hash impact, result
typing/commit ordering, and T09/T01 co-freeze remain open) and the **read-only H9
audit findings recorded as unresolved blockers** (the `/5`-verified `fail_selected`
proves only a quota=1 single-task transition; the proposed quota>1 pipeline is
unimplemented; the dispatcher pre-worker `Ready→Running`/in_flight-population
relationship to the single ordered atomic commit, the in_flight clear ownership/order,
the no-`Running`/residual-set proof for all paths, the internal
`max_dispatches_per_tick`-vs-`max_inflight_per_tick` limit conflict, and the pending
fan-out fixtures/VF02/VF03/VF04/VF13 remain open; the H9 removal direction and the H6
mechanism are **unchanged**; no freeze/code). This CDR
is not a `/6` freeze. The accumulated rev-21 review
findings are integrated in the M1 proposal `§17` (rev 22); the independent rev-22
audit H2–H11 is integrated in `§18` (rev 23/24); the rev-24 independent audit
F1–F9 and the verified `/5` failure guarantee are integrated in `§19` (rev 25); the
rev-25 independent audit items 1–12 are integrated in `§20` (rev 26). **Rev 27**
applied the rev-26 audit corrections;
**rev 28** is a docs-only pointer reconciliation, **rev 29** a docs-only
historical-documentation correction, **rev 30** a docs-only scope/label
correction, **rev 31** records the two user in-principle decisions
(A/A-narrow and B) recorded across rows A/D and §D/§I/§9/§10/§13, **rev 34**
records the H6 batch-failure recovery direction (row A/§A/§9/§10/§13), **rev 35**
records the H9 in-flight scheduling-ownership direction (remove
`max_inflight_total`; row A/§A/§9/§10/§13), **rev 36** is a docs-only **D3**
correction to §13 (the frontend acceptance carries no H6/CT06 text; T02 carries
it), **rev 37** is a docs-only consistency cleanup of stale active candidate
prose (the §A9/§I dispatcher error lists no longer name `InflightQuotaExceeded`
as an active candidate), and **rev 38** is a docs-only lead-in/typo correction
(the §13 opening now reads the M1 proposal as updated through **rev 36**, and the
`per-stage stage queues` typo is fixed), and **rev 39** is the docs-only addition of
the proposed `/6` co-freeze work queue in §9A (three separate gates, parallel
owner-drafting/review tracks vs serial freeze dependencies, and the minimum work
items A–I with exact authorities; no acceptance/schema/interface/chip/`/6` claim);
**rev 40** is the docs-only addition of the proposed **decision-ready
recommendation appendix in §9B** (per-row recommendation + tradeoff/open
dependency, all marked accept/amend/reject; no acceptance/schema/interface/chip/`/6`
claim); and **rev 41** is the docs-only addition of the proposed **per-decision
response matrix in §9C** (bundle → exact authority → `Accept/Amend/Reject/Defer`
with a `PENDING` status/signature cell, citing §9A/§9B/§C–§I; no new decision, no
acceptance/schema/interface/chip/`/6` claim; not schema; rows D–I remain PENDING);
and **rev 42** records the user's explicit **2026-10-05 per-subdecision
acceptances** (A/T02/T13/T01 H6 scope + mechanism, D/T02/T05/T01 CT07 invariant +
await-all, F/T06/T07/T09/T01 no-`FunctionContextId`/`SemRecord` carrier, E/T06/T01
lowest-id committed scan), with the authority entries recorded per role label and
date while **every overall bundle stays PENDING** where other items are unresolved;
**rev 42 qualifies the §9B row-A H6 deferral as a historical proposal the user
rejected on 2026-10-05** and requires the `/6` freeze of the selected H6 mechanism
rather than a later-CDR deferral; it authorizes no code and no `/6` freeze; **rev 43**
records further explicit **2026-10-05** user subdecisions on rows **D** and **E**
(D/T05 `ContinuationRecord` exact ordered fields/shapes with no `awaited`; D/T01/T05
formal `/6` T01 §4 supersession; D/T02/T05/T01 exact OwnBatch pre-apply validation;
E/T06/T01 File Enter deterministic `parse.TranslationUnit -> symbol_type.scope-enter`
stage edge), again per role label and date, with both overall bundles staying
PENDING and no code/`/6` freeze; **rev 44** records further explicit **2026-10-05**
user subdecisions on rows **C** and **G** (**C/H1/T07/T08/T01**: `required_kind` is
the per-use constant-expression requirement (e.g. M1 integer constant expression),
distinct from the lexical `LiteralRecord.candidate_type` and no duplicate implicit
target type; `ConstantResult.legality` is a result payload field with no extra
committed family; **G/T08/T01**: `max_const_bits` origin is a hashed `Limits` value,
M1 cap/default 128, config rejects > 128, explicit task-input projection; **C/T03/T01**:
`ArtifactRecord { kind, source, bytes, raw_offsets }` + total eight-`ArtifactKind`
map rule, M1 exercised scope only single-source `Normalized`), again per role label
and date, with the overall C/G bundles staying PENDING and no code/`/6` freeze; none
adds a
new finding ledger, so `§20` (rev 26) remains the latest ledger and the current M1
candidate revision is the **M1 proposal rev 39** (parallel expected resulting file;
it was **rev 36** at CDR revs 38–51, **rev 37** at the intervening proposal-only
mirror, and **rev 38** at CDR revs 52–53); this CDR is at
**CDR rev 54** (rev 32 was the docs-only history-gap repair of the same change,
rev 33 was a docs-only audit-defect correction, rev 34 is the H6
recovery-direction record, rev 35 is the H9 removal-direction record, rev 36
is the D3 §13 correction, rev 37 is the stale-active-prose cleanup, rev 38
is the lead-in/typo correction, rev 39 is the §9A work-queue addition, rev 40
is the §9B decision-aid addition, rev 41 is the §9C response-matrix addition,
rev 42 is the per-subdecision acceptance record, rev 43 is the further D/E
per-subdecision acceptance record, rev 44 is the further C/G
per-subdecision acceptance record, and rev 45 is the further C `LiteralRecord`
exact-ordered-fields/enum-scope + `Lx08CandidateType` symbolic + `RequiredKind`/
`ConstLegality` values + artifact mandatory-map-invariant subdecision record, and
rev 46 is the further F `ValueCategory`/`EffectMask` + VF06-in-M1 +
M1-minimal-conversion-scope subdecision record, and rev 47 is the
integration-agent **selected candidate defaults under explicit user delegation**
(optional-artifact-map, T05 parse-depth, T06 namespace/lookup, T09 noncritical
rule items; not owner or T01 `[INT]` sign-offs), and rev 48 is the docs-only
**read-only T01 audit findings** record (current `/5` code facts **24/24**,
`RecordFamily` absent; the unresolved `M1AppendSchema` hash-scope conflict; the
draft 19/27/27 counts marked **proposed/unverified**; no hash-scope option
selected, no code/T01/manifest/other-doc edit, no freeze), and rev 49 records the
user's explicit **2026-10-05 critical H11/T09/T01 direction** (reuse the committed
IR28 `FunctionEnd` terminal result as the deterministic completion fact, checked by
a T01-owned typed phase-2b commit-apply validation hook; **no** new marker record
family/ID/arena/tag/ordinal/encoder/`ResultValue`; earlier new-marker direction
**superseded for operative direction**, preserved as history; not a frozen `/6`
hook, no code) and the **read-only H9 audit findings as unresolved blockers** (H9
removal direction and H6 mechanism unchanged; no freeze/code), and rev 50 is the
**user's explicit critical HASH-SCOPE two-tier decision record** (frozen `/6`
`foundation + M1AppendSchema` seed hashed; post-seed runtime `declare()`
declarations hash-excluded; conceptual boundary settled, no `/6` values/hash/counts/
code//5 change; row I stays pending), and **rev 51** is the
**integration-selected candidate default under explicit user delegation** record
(`max_inflight_per_tick`/quota as the sole per-tick dispatch-count bound; the
redundant `max_dispatches_per_tick` dropped from the candidate limit inventory and
validation; not an owner/T01 `[INT]` signoff and not a schema freeze; T01/T02
confirm/co-freeze at `/6`, T02/T13 tests pending). **The rev-51
update** records that **integration-selected candidate default** under the user's
explicit delegation (2026-10-05): it drops the duplicate `max_dispatches_per_tick`
and keeps `max_inflight_per_tick`/quota as the sole per-tick dispatch-count bound,
grounded in the rev-35/38 H9 direction and the rev-49 audit finding that the two
limits duplicated each other; it changes **no** acceptance status, resolves **no**
dispatcher `Ready→Running`/`in_flight` atomic-boundary or clear-order item, and
keeps the H6 mechanism and H9 removal direction as selected; `/5` current, M1
DRAFT, ADR-0002 PROPOSED, rows C–I pending. **The rev-50
update** adds the **user's explicit 2026-10-05 critical HASH-SCOPE decision**
(`[USER]`): the two-tier model is **accepted** — `StoreSchema::foundation +
M1AppendSchema` is the **frozen `/6` seed and participates in the `/6` contract
hash**,
while **post-seed runtime `StoreSchema::declare()` extensions remain excluded** from
the frozen hash and are captured/validated through **runtime snapshot/schema
mechanisms**; at `/6` integration T01 must **atomically** update the
`COMPILER_SFL_MANIFEST.md` §4 wording, the `hash_excludes` semantic
description/token (scoped to **post-seed runtime declarations**, not the frozen M1
seed), and `FrozenSchema::encode`/contract code plus the freeze test, **preserving
the frozen `/5` hash and history**; the conceptual future `/6` boundary is thus
settled, but **no actual `/6` schema values, hash, tag/count inventory, code, or
`/5` change is authorized or completed**, the selected boundary **does not itself
accept exact `M1AppendSchema` contents/counts nor freeze**, and **T01 must still
co-freeze the M1 seed values after the owners** — the numeric inventory and test
implementation remain pending and **row I stays PENDING**. Note the separate counters:
because both documents use the same revision numbers, a bare "rev 30/31/32/33/34/35/36/37/38/39/40/41/42/43/44/45/46/47/48/49/50/51/52/53/54" is
ambiguous — **M1 proposal rev 30** is the A/B decision record, while **CDR rev 30**
is the scope/label correction and **CDR rev 31** is the A/B decision record;
likewise **M1 proposal rev 31** is the proposal's own docs-only history-gap repair,
while **CDR rev 31** is the A/B decision record, **CDR rev 32** is the history-gap
repair, and **CDR rev 33** is the audit-defect correction; **M1 proposal rev 32**
is the H6 recovery-direction record, matching **CDR rev 34**; **M1 proposal rev 33**
is the H9 removal-direction + D1–D3 record (the proposal became rev 34 when the
cross-document H9 completion was recorded); **CDR rev 35** is the CDR H9
removal-direction record; **M1 proposal rev 35** is the stale-active-prose cleanup,
matching **CDR rev 37**; **M1 proposal rev 36** is the header
current-state-pointer + typo correction, matching **CDR rev 38**; **CDR rev 39**
has **no matching proposal revision** (the shared M1 proposal stays **rev 36**);
**CDR rev 40** likewise has **no matching proposal revision** (the §9B
decision-aid is CDR-only); **CDR rev 41** likewise has **no matching proposal
revision** (the §9C response matrix is CDR-only); **CDR rev 42** likewise has
**no matching proposal revision** (the per-subdecision acceptance record is
CDR-only); **CDR rev 43** likewise has
**no matching proposal revision** (the further D/E per-subdecision acceptance
record is CDR-only); **CDR rev 44** likewise has
**no matching proposal revision** (the further C/G per-subdecision acceptance
record is CDR-only); **CDR rev 45** likewise has **no matching proposal
revision** (the further C `LiteralRecord` exact-ordered-fields/enum-scope +
`Lx08CandidateType` symbolic/no-bit-width + `RequiredKind`/`ConstLegality` values +
artifact mandatory-map-invariant subdecision record is CDR-only, and the shared M1
proposal stays **rev 36**); and **CDR rev 46** likewise has **no matching proposal
revision** (the further F `ValueCategory`/`EffectMask` + VF06-in-M1 +
M1-minimal-conversion-scope subdecision record is CDR-only, and the shared M1
proposal stays **rev 36**); and **CDR rev 47** likewise has **no matching proposal
revision** (the integration-agent selected candidate defaults under explicit user
delegation are CDR-only, and the shared M1
proposal stays **rev 36**); and **CDR rev 48** likewise has **no matching proposal
revision** (the read-only T01 audit findings record is CDR-only, and the shared M1
proposal stays **rev 36**); and **CDR rev 49** likewise has **no matching proposal
revision** (the critical H11/T09/T01 `FunctionEnd`-terminal-result direction record
and the read-only H9 audit-findings record are CDR-only, and the shared M1
proposal stays **rev 36**); and **CDR rev 50** likewise has **no matching proposal
revision** (the user's critical hash-scope two-tier decision record is CDR-only, and
the shared M1 proposal stays **rev 36**); and **CDR rev 51** likewise has **no
matching proposal revision** (the integration-selected candidate default dropping
`max_dispatches_per_tick` in favour of the sole `max_inflight_per_tick`/quota bound
is CDR-only; not an owner/T01 `[INT]` signoff; the shared M1 proposal stays
**rev 36**); and **CDR rev 52** likewise has **no matching proposal revision** (the
read-only cross-owner audit/reconciliation record is CDR-only; **no owner package is
called a `/6` sign-off**; the parallel **M1 proposal was at rev 38** at the time, and the
CDR points at it; the CDR is not an executable schema); and **CDR rev 53** likewise has
**no matching proposal revision** (the integration-agent selected candidate default
under explicit user delegation for the M1 exercised literal subset — the closed
one-member `Lx08CandidateType { Int }` M1 vocabulary, M1 literals 2/3 as
target-independent `Int` with no bit width, no silent `Int` defaulting for forms
outside the exercised M1 subset, and append-only future categories — is CDR-only;
**not** an owner/T04 or T01 `[INT]` signoff, **not** a `/6` freeze, **not** the
complete future C candidate vocabulary, no numeric tags; the parallel **M1 proposal
was at rev 38** then, and the CDR pointed at it); and **CDR rev 54** likewise has
**no matching proposal revision** (the docs-only read-only-audit +
historical-supersession cleanup — the §G2 signed-range/`i128`/enforcement
supersession, the H9 duplicate-limit-conflict supersession by rev-51, and the open
T04 reciprocal-link / T05-T06 TU-edge / T13 no-residual / vertical PP-08 findings —
is CDR-only; **not** an owner/T01 `[INT]` signoff, **not** a `/6` freeze, no
architecture-critical choice; the parallel **M1 proposal is expected at rev 39**,
and the CDR points at it). Read every bare "rev
31/32/33/34/35/36/37/38/39/40/41/42/43/44/45/46/47/48/49/50/51/52/53/54" against its
document. The C allocation acceptance is recorded in
§C2/§C3 and the §12 revision record (rev 24). **Rev 31 (user, in principle)**
records: (A) the **Guardrails §6.1 narrow interpretation** — same-task
`OwnBatch(DraftRef)` is transient wire/proposal input only, resolved to committed
IDs before persistent state; no durable cursor/`WaitSet`/join may point at a
draft/wire/address; guardrail unamended; **T01 implementation-confirmation
pending**; and (B) the awaited-child **`WaitSet`-only** decision — `WaitSet` is the
sole source and `ContinuationRecord` carries no duplicate `awaited`, with the
T01 §4 supersession accepted in principle for `/6` (**no `/5` edit**) and
**T01 + T05 acceptance pending**. Both are **accepted in principle only**;
**row D remains PENDING overall**. **Rev 32** is a docs-only history-gap repair
(no new acceptance): it mirrors M1 proposal rev 31 (which supplied the proposal's
missing historical rev-29 row) and rewrites §13 to disambiguate the proposal/CDR
revision counters; it advances the current M1 pointer to **rev 21–31** and adds no
new finding ledger. **Rev 33** is a docs-only audit-defect correction (no new
acceptance): it restricts the §13 "made no change" claim to CDR rev 31 and the
rev-32 history-gap repair, notes that **CDR rev 30 did change the M1 target
acceptance** (its rev 27), labels the bare revision numbers by document, and adds
this row; it does not advance the M1 pointer (still **rev 21–31**) and adds no new
finding ledger. **Rev 34** records the **H6 batch-failure recovery direction**
(user in principle, 2026-10-04; no new acceptance beyond the direction): every
dispatched task fails exactly once in dispatch order with the optional-diagnostic/
`DiagnosticId::NONE` sentinel, so every task leaves `Running`; clearing the
in-flight set is not itself a transition. It advances the current M1 pointer to
**rev 21–32** and adds no new finding ledger. **Rev 35** records the **H9 in-flight
scheduling-ownership direction** (user in principle, 2026-10-04; no new acceptance
beyond the direction): remove `max_inflight_total` from the `/6` candidate because
sequential per-tick dispatch is already bounded by `max_inflight_per_tick`/quota,
the stage queues, and `max_tasks_total`; `Waiting` is not in-flight; the dispatch
batch bound stays the per-tick `max_inflight_per_tick` quota and `tasks.in_flight`
is an ephemeral per-tick scheduler batch only; a future cross-tick `Running` mode
needs a separate CDR. It advances the current M1 pointer to **rev 21–33** and adds
no new finding ledger. **Rev 36** is a docs-only **D3** correction (no new
acceptance): the §13 frontend claim is corrected — the M1 frontend acceptance is at
**Rev 25** and contains **no** H6/CT06 text, and **T02** carries the operative H6
batch-failure text. It advances the current M1 pointer to **rev 21–34**. **Rev 37**
is a docs-only consistency cleanup of stale active candidate prose (no new
acceptance): the §A9 and §I dispatcher/scheduling error carriers no longer name
`InflightQuotaExceeded` as an active candidate — it is removed with
`max_inflight_total`, retained only as the historical rev-21/22 name, and the
per-tick dispatch bound stays `SelectionBatchOverflow` on `max_inflight_per_tick`.
It advances the current M1 pointer to **rev 21–35**. **Rev 38** is a docs-only
lead-in/typo correction (no new acceptance): the §13 opening now reads the M1
proposal as updated through **rev 36**, and the `per-stage stage queues` typo is
fixed; the current-state pointer stays **rev 21–35** (the shared M1 proposal is
**rev 36**); **rev 39** is a docs-only addition of the proposed `/6` co-freeze work
queue in §9A (three separate gates, parallel owner-drafting/review tracks vs serial
freeze dependencies, and the minimum work items A–I with exact authorities), adding
no acceptance, schema, interface, task-kind, chip, or `/6` claim; every §9A item is
a proposed question/authority assignment, not accepted schema, and the detailed
§C–§I/§13 owner rows remain the source of truth. **Rev 40** is a docs-only addition
of the proposed **decision-ready recommendation appendix in §9B** (a per-row recommendation +
tradeoff/open dependency for accept/amend/reject by the named authority; no
acceptance, schema, interface, task-kind, chip, or `/6` claim); §9B explicitly
does **not** erase the user's accepted-in-principle H6 direction, and the
current-state pointer **stays rev 21–35** (the shared M1 proposal stays **rev 36**).
**Rev 41** is a docs-only addition of the proposed **per-decision response matrix in
§9C** (bundle → exact authority → `Accept recommendation / Amend / Reject / Defer`
with a `PENDING` status/signature cell, citing §9A/§9B/§C–§I; a procedural form, not
schema; no new decision, acceptance, schema, interface, task-kind, chip, or `/6`
claim); it preserves the user's in-principle H6 direction and requires any H6
deferral to return to the user, and the current-state pointer **stays rev 21–35**
(the shared M1 proposal stays **rev 36**). **A subsequent correction to that same
rev-41 §9C form** (no new revision) makes multi-authority rows unambiguous: every
listed human authority must **independently** respond/sign for its own acceptance
(one `Authority — disposition — date` sub-entry each; no group or single-owner
response represents another), a row stays **`PENDING`** until **all** listed
authorities (and T01 `[INT]` where listed) have separately signed, and T01 records
`/6` acceptance only after every owner signoff. **Rev 42** records the user's
explicit **2026-10-05 per-subdecision acceptances** (user self-identifies as all
named T01/owner roles and responds directly in each role). The selected
subdecisions are: **A/T02/T13/T01 H6 scope + mechanism** (freeze a
quota>1-capable bounded batch recovery mechanism in `/6`; pre-dispatch errors
before state mutation leave tasks `Ready`; a semantic batch commit failure commits
no semantic state; a deterministic bounded recovery mutation processes the
dispatched tasks once in dispatch order to `Failed`; no pre-reservation of N
diagnostics; per-task diagnostic attempt with `DiagnosticId::NONE` when capacity
is insufficient; a state guard prevents duplicate transition), **D/T02/T05/T01**
(join as a commit-apply invariant, no new CT07 committed carrier/family; await-all
children terminal before the parent decision), **F/T06/T07/T09/T01** (no
`FunctionContextId`; `SemRecord` is the committed materialization of `CheckedNode`,
one per `NodeId`, with an explicit committed typed link consumed by T09), and
**E/T06/T01** (deterministic bounded scan of committed `types.records` returning
the lowest matching `TypeId`, no hidden cache/index; T05 upstream dependency
acknowledged). Rev 42 **rejects the §9B row-A recommendation to defer the H6
mechanism to a later CDR** (retained there as a historical proposal only) and
records the `/6` freeze of the selected H6 mechanism instead. Every **overall
bundle status stays PENDING** where other items are unresolved; the unselected
proposals remain proposal-only, and no code or `/6` freeze is authorized. **Rev 43**
records further explicit **2026-10-05** user subdecisions on rows **D** and **E**
(no contract/schema/interface/task-kind/chip/`/6` claim): **D/T05** accepts the
candidate `ContinuationRecord` exact ordered fields/shapes (no `awaited`; durable
committed-ID references only), **D/T01/T05** formally accepts the `/6`-only T01 §4
supersession (`WaitSet` sole awaited-child source; T01 §5 unchanged), **D/T02/T05/T01**
accepts the exact OwnBatch pre-apply validation (phase-1 continuation ref within the
same task `AppendRecords` range/family `Continuation`; phase-2b child ref within the
same task Enqueue list; whole-batch `CommitError` before any mutation), and
**E/T06/T01** accepts the File Enter trigger as a deterministic
`parse.TranslationUnit -> symbol_type.scope-enter` stage edge after the committed TU
carrying the committed `NodeId` with no job-bootstrap. The `RecordRef` numeric wire
tags and `RecordFamily` ordinals remain **separate inventories** (their exact tag
values are **not** accepted; numeric encodings are a T01 `/6` freeze detail), the
`ParseContext` vocabulary final encoding, the OwnBatch numerical error codes/hash,
the parse request/result/test shapes, T05's committed-TU upstream carrier, and the
rest of the D/E bundles remain **PENDING**; both overall bundles stay PENDING and no
`/6` freeze or implementation is authorized. **Rev 44**
records further explicit **2026-10-05** user subdecisions on rows **C** and **G**
(no contract/schema/interface/task-kind/chip/`/6` claim): **C/H1/T07/T08/T01** accepts
that `ConstantRequest.required_kind` means the **per-use constant-expression
requirement** (e.g. the M1 integer constant expression), distinct from the lexical
`LiteralRecord.candidate_type`, and does not duplicate an implicit target type, and
that `ConstantResult.legality` is a **result payload field** with **no** extra
committed record family (the exact `ConstLegality` variants remain **open**);
**G/T08/T01** accepts that `max_const_bits` origin is a **hashed `Limits` value**
(M1 cap/default **128**; `config` **rejects** values **> 128**; explicit
**task-input projection** to the restricted T08 chip — an accepted
source/projection/bound subdecision whose exact wire field, diagnostic numeric code,
and hash encoding remain `/6` details, and the cap is **not** a claim that every
valid config value must equal 128); and **C/T03/T01** accepts
`ArtifactRecord { kind, source, bytes, raw_offsets }` with the **total eight-`ArtifactKind`**
scheme (map-mandatory `Normalized`/`Spliced`/`CommentFree`/`Preprocessed`;
map-optional `Assembly`/`Object`/`Snapshot`/`Trace`), the M1 **exercised scope only
single-source `Normalized`** with other producers and the multi-source map deferred
(a selected M1 map shape/scope), while the exact `raw_offsets` invariant/error
mapping and all enum numeric codes remain **open**. The overall C/G bundles stay
PENDING and no code or `/6` freeze is authorized. The
owner-facing amendments are §13.

**Rev 48 (2026-10-05) — read-only T01 audit findings (docs-only; no code/T01/
manifest/other-doc edit, no hash-scope option selected, no `/6` freeze).** A
read-only verification against the checked-in `/5` code confirmed, as **`/5`
facts, not proposals**: the frozen contract is `t01-c01-c06/5`
(`61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5`);
`compiler/src/contract.rs` encodes **24** `RECORD_KINDS`; `compiler/src/ids.rs`
`RecordRef` has **24** variants (wire tags **0–23**, `Artifact` = 23); and a
`RecordFamily` type is **absent** from the code. These **`/5` code counts** are
recorded against the CDR's own candidate figures (19 draft families / 27
`RecordRef` variants / 27 `RecordFamily` ordinals) as **proposed/unverified** —
the draft counts are **not** asserted to be correct and the future `/6` count is
**not** claimed to equal the current 24. The audit also confirmed a **real
unresolved hash-scope conflict** (not merely prose): `CONTRACT_VERSION`,
`compiler/src/contract.rs`, and `compiler/README.md` carry
`hash_excludes=runtime-registrations, routing-content,
group-declared-store-fields, chip-logic` (so `group-declared-store-fields` is
**excluded** from the hash), while `COMPILER_SFL_MANIFEST.md` §4 states that
"adding fields or rules changes the frozen contract hash"; the proposed
`M1AppendSchema` is currently **proposed to be hashed at `/6`** (not hashed
today). Three hash-scope options exist — **hash `M1AppendSchema`** (requires an
update to the exclusion token + a freeze assertion + manifest wording), **do not
hash it** (requires correcting the manifest wording), or an **explicit two-tier
split between a hash seed and runtime declarations** — and **none is selected or
recommended here** (no authority to choose). The audit further requires `/6` to
specify **dual-inventory encoding** (`RecordRef` wire tags kept separate from
`RecordFamily` ordinals — the structural subdecision accepted at rev 43),
**inclusion of numeric values in the hash**, and a **schema self-consistency
mechanism**, with the **actual counts/values frozen only after the owner shapes
are accepted**. This is a **critical `/6` hash-scope choice with no
implementation authorization**. No owner acceptance is added and no signature is
recorded; the overall rows C–I remain **PENDING**.

**Rev 49 (2026-10-05) — user's explicit critical H11/T09/T01 direction + read-only
H9 audit findings (docs-only; no code/other-doc/test edit, no `/6` freeze).** The
**user** recorded an explicit **2026-10-05 critical decision** for **H11/T09/T01**:
the `TerminatorMissing` trigger **reuses the committed terminal result of the IR28
`FunctionEnd` task** — the `TaskState::Completed(ResultId)` produced by the task
whose kind is `FunctionEnd` — as the **deterministic function-completion fact**; a
**T01-owned typed phase-2b commit-apply validation hook** checks the function's
**entry block termination** (greatest `InstructionId` is a terminator) when that
committed result is applied. This **adds no new marker record family/ID/arena/
`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder and no new `ResultValue`
variant**. The earlier **new-marker (`CompletedFunction`) direction** is marked
**superseded for the operative direction** but is **preserved as history**. The
selected direction is **not a frozen `/6` hook/schema and authorizes no code**; the
**exact hook contract, its hash impact** (likely **no** new record family, but the
rule/hook hashing is still a **T01 decision**), **result typing/commit ordering**, and
the **T09/T01 co-freeze** all remain **open**. Overall **H stays pending** and rows
C–I stay pending. The **read-only H9 audit findings** are recorded **as unresolved
blockers, without changing the selected H6 mechanism or the H9
`max_inflight_total`-removal direction**: the `/5` code proves **only** the quota=1
single-task `fail_selected` transition, so the proposed **quota>1 pipeline is not
implemented**; H9 still needs the **exact dispatcher pre-worker `Ready→Running`/
`in_flight`-population relationship to the single ordered atomic commit**, the
**exact `in_flight` clear ownership/order relative to the bounded H6 recovery**, and
a **proof that no `Running`/residual set remains at latch for all
success/error/empty-proposal paths**; there is an **internal proposed-limit
conflict** (`max_dispatches_per_tick` is listed/checked separately from
`max_inflight_per_tick` although the latter is described as the sole dispatch bound)
that must be **resolved in `/6`**, not decided now; and the **H6/H9 fan-out fixtures**
and **T13 VF02/VF03/VF04/VF13** remain **pending**. The H9 no-residual guarantee is
**conditional on the H6 recovery implementation**, and the **quota=1 M1 baseline is
preserved and separated** from the quota>1 optimization. **No freeze/code**; exact
stages/version carrier/errors/hashes remain pending. This is a **user instruction
recording a selected direction plus a read-only audit of open blockers** — the audit
is **not a signature** and no hash-scope option is selected.

**Rev 50 (2026-10-05) — user's explicit critical HASH-SCOPE decision (two-tier
model; docs-only; no `/5`/T01/manifest/code/test edit, no `/6` freeze).** The
**user** recorded an explicit **2026-10-05 critical decision** resolving the
conceptual future `/6` hash boundary: the user **accepts the two-tier model**.
**(a) Frozen seed:** `StoreSchema::foundation + M1AppendSchema` is the **frozen
`/6` seed** and **participates in the `/6` contract hash**. **(b) Runtime
declarations excluded:** post-seed runtime `StoreSchema::declare()` extensions
**remain excluded** from the frozen contract hash and are instead
**captured/validated through runtime snapshot/schema mechanisms**. This **resolves
the conceptual `/6` boundary** (the three-option hash-scope question is **no longer
open at the conceptual level**), but it **authorizes or completes no actual `/6`
schema values, hash, tag/count inventory, code, or `/5` change**: the **selected
boundary does not itself accept exact `M1AppendSchema` contents/counts, or freeze
anything**, and **T01 must still implement it as integration authority and
co-freeze the M1 seed values after the owners** (numeric inventory and test
implementation **remain pending**). **At `/6` integration, T01 must update
atomically:** the normative [COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md)
**§4 wording**; the **`hash_excludes` semantic description/token** — now scoped to
**post-seed runtime declarations**, **not** the frozen M1 seed (the current
`group-declared-store-fields` token is to be re-described, not silently reinterpreted);
and `FrozenSchema::encode`/contract code plus the freeze test — **while preserving
the frozen `/5` hash and history** (`t01-c01-c06/5`, `6187…63bb5`). Per `AGENTS.md`
§9C authority, this is a **`[USER]` instruction recording the selected conceptual
scope**; it is **not** a row-I closure, **not** a `/6` freeze, and **not** a T01
signature beyond the user's own scope decision; `/5` remains current, M1 remains
DRAFT, ADR-0002 remains PROPOSED, and **row I / the overall CDR stay PENDING**.

**Rev 51 (2026-10-05) — integration-selected candidate default under explicit user
delegation (sole per-tick dispatch bound; docs-only; no `/5`/T01/manifest/code/test
edit, no `/6` freeze).** Under the user's explicit delegation ("for non-critical
decisions adopt the recommended choice directly; ask only for critical decisions"),
the **M1 integration agent** selected — **under user-delegated integration default,
not as an owner or T01 `[INT]` signoff** — that **`max_inflight_per_tick`/quota is
the sole per-tick dispatch-count bound** and that the redundant
**`max_dispatches_per_tick` is dropped** from the candidate limit inventory and its
validation. This is grounded in the rev-35/rev-38 H9 direction (per-tick dispatch
bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`)
and in the rev-49 audit finding that the two limits duplicated each other. It
**resolves no** dispatcher-`Ready→Running`/`in_flight` atomic-boundary or
clear-order item and changes neither the H6 mechanism nor the H9 removal direction.
**T01/T02 must confirm/co-freeze the exact names/defaults/codes at `/6`, and the
T02/T13 tests remain pending.** This is a **delegated candidate default**, **not**
an owner/T01 `[INT]` sign-off, **not** a schema freeze, and **not** code; `/5`
remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED, and rows C–I remain
**PENDING**.

**Rev 52 (2026-10-05) — cross-owner audit/reconciliation (read-only; docs-only; no
code/T01/manifest/other-doc/test edit, no `/5` change, no `/6` freeze; no owner
package is called a `/6` sign-off).** This revision mirrors the CDR's
already-selected items against the now-completed owner packages and records the
reconciliation as **subitems A–E**.

**A — Package state mirrors the CDR's selected items (documentation alignment
only).** Read-only reconciliation confirms each package records the same
already-selected candidate text the CDR selected, **without any package being a
`/6` sign-off**:
- **T02** — **rev 30** (sync of the CDR-rev-42/43/44/45/46 user subdecisions: H6
  scope+mechanism, CT07 commit-apply invariant / no new carrier / await-all, H9
  removal re-affirmed) and **rev 31** (mirror of the CDR-rev-51 integration-selected
  candidate default: sole per-tick `max_inflight_per_tick`/quota bound, redundant
  `max_dispatches_per_tick` dropped). **T02 revision is at rev 31.**
- **T03/T04** — **T03 rev 44/45/47** (artifact shape/map scope, mandatory-map
  invariants, delegated optional-map default); **T04 rev 45/46** including the
  integration-selected **`LiteralRecord.token`** candidate default
  (`Some(committed TokenId)` for M1 source literals; `None` reserved for an
  explicitly synthesized literal with no single originating source token) and the
  **same-revision correction** that any same-batch token↔literal relation is
  expressed through the T01-frozen typed **`RecordLink`**/append-reference
  mechanism **resolved to durable committed IDs before persistence**, **not**
  `ContinuationRef::OwnBatch`/`ChildRef::OwnBatch` (those belong to the
  continuation/task-child protocol, so the earlier `OwnBatch`-as-record-link
  wording is corrected — **no `OwnBatch` confusion for record links**). Still open:
  the exact `RecordLink`/`LinkTarget` local-link spelling/wire tag/error handling,
  the synthetic-literal policy beyond M1, and the source-span relation/error codes.
- **T05** — the operative amendments record the accepted continuation/join/
  `OwnBatch` pre-apply subdecisions plus the integration-selected **`ParseContext`**
  candidate default (parse depth reuses `limits.max_task_depth`, parser
  continuation/child frames only, detect before child enqueue,
  `ParseDepthExceeded`); the **`ParseContext` vocabulary final encoding remains a
  T05/T01 `/6` co-freeze item**.
- **T06** — the operative amendment records the canonical-`TypeId` lowest-matching
  committed-arena scan, the File Enter `parse.TranslationUnit ->
  symbol_type.scope-enter` stage edge, and the delegated namespace/miss/TY03
  active-scope-chain lookup defaults; `Identifier`-leaf `decl` and lifecycle/event
  encodings stay pending.
- **T07** — includes the **sem→const correction**: the stale opening dependency
  clause that named T08 constants as a T07 upstream input is corrected — under the
  selected **sem→const** M1 order T07 produces the per-use `ConstantRequest`, then
  T08 computes/folds constants, and T09 consumes the committed `SemRecord` plus the
  T08 constant (no `T07→T08→T07` cycle). It also records the F carrier/`ValueCategory`/
  `EffectMask`/VF06-in-M1/M1-minimal-conversion-scope subdecisions.
- **T08** — the operative request/result shape is aligned to the selected H1
  allocation/exact candidate shape (`ConstantRequest { literal: RecordRef::Literal,
  node, required_kind }`, `ConstantResult` carries a committed `RecordRef::Const`
  plus `legality`; `RequiredKind` M1 `IntegerConstantExpression`; `ConstLegality`
  `Legal`/`NotConstantExpression`/`Unsupported`; no separate legality family), with
  payload-variant spellings, reference encoding/tag, task typing, and all numeric
  codes still `/6` T01/T07/T08 co-freeze items.
- **T09** — records the **H11** direction: `TerminatorMissing` reuses the committed
  IR28 `FunctionEnd` terminal result (`TaskState::Completed(ResultId)`, task kind
  `FunctionEnd`) as the deterministic completion fact, checked by the T01-owned
  typed phase-2b commit-apply validation hook; **no** marker record
  family/ID/arena/tag/ordinal/encoder/`ResultValue`; the marker-based wording is
  superseded for the operative direction and preserved as history.
- **T13** — records the H6 batch-verification amendment (selected mechanism) and
  VF06-in-M1 (`TypedAstInvariant` after committed T07 `SemRecord`s, before T09
  lowering), with VF02/VF03/VF04/VF13 and the exact VF06 registration still
  pending.
- **Acceptance docs / ADR** — the **[M1 vertical acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md)
  is at rev 27** (artifact-map scope alignment; no Part A/Part B gate change) and
  the **[M1 target acceptance](M1_TARGET_ACCEPTANCE.md) is at rev 29** (current
  operative `TerminatorMissing` `FunctionEnd` reconciliation), and
  **[ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) is at
  Revision 17** (the CDR-rev-51 sole-per-tick-bound mirror).

**B — M1 proposal/target/vertical alignment (no `/6` freeze).** The M1 proposal,
target acceptance, and vertical acceptance now align on **H11**, the
**hash boundary**, and the **Part A-vs-B** split, and the proposal is being
advanced in parallel to **rev 38** (expected resulting file). The `/5` frozen
contract is **preserved** and there is **no `/6` freeze**; the alignment is
documentation-level only.

**C — T01 audit still leaves `/6` not freeze-ready.** The read-only T01 audit
records the following as still **not freeze-ready**: exact record/tag/**family**
inventories; field bodies/ref encoding; task/request/result typing; writer
manifests; the TU-carrier/**`ParseContext`** final encoding; `SourceId`/token
provenance/raw-offset mapping and **`Lx08CandidateType`**; **`SemRecord`/VF06**;
const overflow/projection; the IR hook contract; the scheduler **H6/H9** atomic
boundaries/in_flight clear order/no-residual fixtures; and the
snapshot/canonical-encoder/hash tests. The audit implies **no new user-critical
decision**.

**D — Selected items remain selected; exact implementation/sign-off pending.** The
**rev-51 delegated `max_inflight_per_tick`-only** candidate default and the
**rev-50 hash-scope** decision remain **selected**; their exact implementation and
sign-off are **pending**. Rows **C–I remain PENDING overall**.

**E — CDR role and next integration stage.** This CDR is an **accepted-decision
ledger/work queue, not an executable schema**. After the owners produce the exact
artifacts, the **next integration stage is T01's serial `/6` seed assembly**,
self-consistency-checked against the **frozen two-tier hash model**
(`foundation + M1AppendSchema` hashed seed; post-seed runtime `declare()` excluded).
**No chip coding precedes the `/6` freeze**; **Part B remains independent**. This
is a read-only audit/reconciliation record: `/5` remains current, M1 remains DRAFT,
ADR-0002 remains PROPOSED, and no code, interface, task-kind, chip, or `/6` freeze
is authorized.


(no contract/schema/interface/task-kind/chip/`/6` claim): C/T04/T08/T01** fixes
the **exact ordered `LiteralRecord` fields** (`token: Option<TokenId>`,
`kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>`
big-endian magnitude, `negative: bool`, `spelling: Vec<u8>`,
`candidate_type: Lx08CandidateType`; no `node`/`required_kind`) and the **M1
enum/scope** — `LiteralKind {Integer, Character, String}` (only `Integer` produced
in M1), `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced), radix
domain {2,8,10,16} (M1 decimal only), and a **symbolic/target-independent**
`Lx08CandidateType` with M1 literals 2 and 3 as `Int` and **no bit width**; its
**complete member set/numeric encodings remain open and are deliberately not
invented**. **C/T07/T08/T01** fixes `RequiredKind` (M1 enum only
`IntegerConstantExpression`) and the `ConstLegality` result-field values
(`Legal`, `NotConstantExpression`, `Unsupported`), with future C constant-expression
purposes requiring **appended variants/new rules**, not a repurposing of the lexical
candidate type. **C/T03/T01** accepts the **mandatory-map invariants**
(`raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last
`<= source.bytes.len()`, and mandatory kinds require a valid source); the
**optional-kind map rule, source-versus-payload equality, and the exact artifact
error classification/numeric codes remain open** because the prompt allowed an empty
optional map or another explicit rule and the user made no choice between them. The
overall C/G bundles stay PENDING and no code or `/6` freeze is authorized.

**Rev 46 records further explicit 2026-10-05 user subdecisions on row F
(no contract/schema/interface/task-kind/chip/`/6` claim): F/T07** fixes
`ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }`, with the M1
fixture producing **`NonLvalue`**, and fixes that M1 allows **only `EffectMask(0)`**
— a **nonzero** mask is a **typed unsupported/diagnostic**, and the effect **bit
classes are reserved/unassigned**. **F/T07/T13/T01** fixes that VF06
**`TypedAstInvariant` is in M1**: it **executes after the committed T07
`SemRecord`s and before T09 lowering**, checking **M1 typed-fact/required-conversion
completeness**; its **exact registered task kind/stage/phase/interface remains
T01/T13 co-freeze/open** and is not asserted here. **F/T06/T07/T09/T01** selects the
**M1-minimal conversion scope only** — freeze **only** the conversion behavior the
M1 fixture actually needs, including the **identity/no-conversion rule**; **integer
promotions, float conversions (incl. `FloatToFloat`), pointer-qualifier conversions,
and other non-M1 conversions are explicitly unsupported and deferred** to a later
**append/contract revision** (distinct from being silently omitted). No exact
`ConversionOp`/`ConversionRole` closed variant list, pairing, role→chip mapping, or
numeric encoding is invented here; those remain **open `/6` co-freeze details**. The
overall F bundle stays PENDING and rows C–I remain pending; no code or `/6` freeze is
authorized.

---

## 3. Exact current vs proposed shared interfaces (delta matrix)

Current = checked-in `/5` and accepted documents. Proposed = rev-21 candidate.
"Requested" is what this CDR asks the authority to accept, amend, or reject.

### 3.1 Bus, task, and report envelope

| Shared interface | Current (`/5`) | Rev-21 candidate | Requested decision | Authority |
|---|---|---|---|---|
| `CompilerBus.tasks.active` | singleton `Option<TaskId>` | quota-1 projection of an in-flight set; canonical `tasks.in_flight: Vec<TaskId>` | whether to add in-flight set + `dispatch_cursor`, and whether `active` stays a compatibility view | `[INT]` + `[USER]` (pipeline) |
| READY queue | single `tasks.ready` | canonical `stage_queues[stage]`, `ready` derived read-only | add per-stage queues or keep single queue; who writes | `[INT]` |
| `CompilerWires.selection` | singular selection value | canonical `SelectionBatch { entries: Vec<SelectionEntry> }`; `wires.selected` quota-1 projection | exact `SelectionEntry`/`SelectionBatch` shape and `dispatch_cursor` semantics | `[INT]` |
| `CompilerBus.report` | report types live in `routing.rs`; **no `report.rs`** | canonical bounded `bus.report` trace + `report.metrics`; `report.rs`/`driver.rs` proposed | approve relocation and canonical report register | `[INT]` |
| `TickOutcome` | existing variants | gains `Finished`/`JobFailed` | approve new variants | `[INT]` |
| `Proposal` | `Enqueue \| Complete \| Fail \| AwaitHost \| StorePatch` | adds `AppendRecords`, `Progress`, `AwaitChildren` | approve the three additions or a different mechanism | `[INT]` + `[OWNER]` |
| `ResultValue` | `Empty \| Ack \| Record \| Records \| Diagnostic` | adds `DraftRecords` | approve same-commit draft resolution | `[INT]` |
| `TaskDraft.continuation` | committed `TaskId`-style ref | `Option<ContinuationRef>` (`Committed`/`OwnBatch`) | approve own-batch continuation ref; **user in-principle narrow interpretation 2026-10-04 (transient wire/proposal input only, resolved to committed IDs before persistent state; guardrail unamended)** | `[INT]` + `[OWNER]` (T05) |
| `StoreId::Names` | absent | appended at end (indices 0–19 do not shift) mapped to `InternTable` | approve new store id and its non-arena mapping | `[INT]` |

### 3.2 IDs, record references, and stores/fields

| Shared interface | Current (`/5`) | Rev-21 candidate | Requested decision | Authority |
|---|---|---|---|---|
| New IDs | — | `SemId`, `ScopeEventId`, `LiteralId` | approve, or express without new IDs | `[INT]` + `[OWNER]` |
| `RecordRef` tags | **24 variants, tags 0–23** (`Artifact` = 23) — **verified `/5` code fact** (rev 48 read-only audit: `compiler/src/ids.rs`, 24 variants) | append `Sem`=24, `ScopeEvent`=25, `Literal`=26; tags 0–23 unchanged | approve append-only tags; **draft 27-variant count is proposed/unverified — not asserted correct; future `/6` count not claimed to equal the current 24** | `[INT]` |
| `RecordFamily` | **absent from the code** — **verified `/5` code fact** (rev 48 read-only audit: no `RecordFamily` type in `compiler/src/`) | 27 variants; ordinals **separate** from wire tags | approve dual inventory and exhaustive `family`/`make`; **the 27-ordinal figure is proposed/unverified** | `[INT]` |
| `sources.spans` | no span append field | append field; rev-21 selects **T03 sole owner** (T04/Token reuse committed PP spans, AST token range) | **single owner T03**; no T04/T05 span write (§B) | `[USER]` + `[INT]` + `[OWNER]` |
| `SpanRecord.start/end` | `u32` offsets | widened to `u64` (`<= max_source_bytes`); rev 22 records the widening the rev-21 delta omitted | approve widening (T03 finding 5) | `[INT]` + `[OWNER]` T03 |
| `sources.span_root` / `sources.expansion` writers | foundation source-root/span-map fields; no general span write | `span_root` stays T01 foundation (no general span write); `sources.expansion` singular metadata vs `sources.expansions` T03 append area; rev 22 records the writer boundary | approve writer boundary (T03 finding 5) | `[INT]` + `[OWNER]` T03 |
| `sources.expansions` | placeholder | T03-only append field | approve single owner | `[INT]` + `[OWNER]` T03 |
| `sources.expansion` vs `sources.expansions` | `sources.expansion` exists (foundation) | keep distinct: singular metadata vs plural append area | approve naming/field split | `[INT]` |
| `names.entries` | intern table, not a store | `StoreId::Names` + `intern_reserved`, no `CommittedPatch` | approve intern-as-store model and bump rule | `[INT]` |
| `lex.literals` | absent | T04-owned `LiteralRecord`; **accepted in principle (H1, 2026-10-04):** a per-literal **lexical candidate fact** carrying spelling/radix/suffix/magnitude and the **`LX08` candidate type** (no `node`/`required_kind`); `node`/`required_kind` on the sem-stage `ConstantRequest`, `legality` in the result. **Not a freeze**; exact variants co-freeze. **Rev 44:** `required_kind` means the **per-use constant-expression requirement** (e.g. M1 integer constant expression), distinct from `candidate_type`, no duplicate implicit target type; `legality` is a **result payload field** with no extra committed family (exact `ConstLegality` variants open). **Rev 45 (selected subdecision):** exact ordered `LiteralRecord` fields `token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` (big-endian magnitude), `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType` (no `node`/`required_kind`), and the M1 enum/scope `LiteralKind {Integer, Character, String}` (only `Integer` produced) / `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced) / radix {2,8,10,16} (M1 decimal only) / symbolic `Lx08CandidateType` (M1 literals 2,3 = `Int`, no bit width; full member set/numeric encodings still open) **Rev 53 (delegated candidate default, 2026-10-05; not an owner/T04 or T01 `[INT]` signoff):** the M1 exercised literal subset uses the **closed one-member `Lx08CandidateType { Int }`** (M1 literals 2/3 = target-independent `Int`, no bit width; no silent `Int` defaulting outside the subset — those are explicit unsupported/deferred; future categories append without reinterpretation; not the complete future C candidate vocabulary, no numeric tags) | approve field/owner; settle the exact `LiteralRecord`/`ConstantRequest` variants as a T04/T08 `/6` co-freeze; see §C/§F4 | `[INT]` + `[OWNER]` T04/T08 |
| `sem.records` | no `sem` id/arena/record family | `SemId` + `SemRecord` | approve `sem` family | `[INT]` + `[OWNER]` T07 |
| `constants.records` | placeholder | **single writer T08** | confirm single writer and T04 handoff (§C) | `[INT]` + `[OWNER]` T08 |
| `types.records` | placeholder | `ChipId`-keyed allowlist (TY13 int, TY17 function type); no structural dedup | approve allowlist and reuse producer ownership | `[INT]` + `[OWNER]` T06 |
| `symbols.scope_events` | absent | T06 append field | approve lifecycle event field | `[INT]` + `[OWNER]` T06 |
| `tasks.continuations` | existing `ContinuationRecord` | extended field-by-field (parse state); **rev 43: the ordered fields/shapes are user-accepted** | approve the ordered fields **accepted 2026-10-05**; fix the exact wire tags/`RecordRef`/`RecordFamily` ordinals encoding (§D) | `[INT]` + `[OWNER]` T05 |
| `artifacts.fragments` | existing `ArtifactRecord` | total `ArtifactKind` (8) + `source`/`raw_offsets` | approve total set and map rule (§C); **rev 44 accepted subdecision** (8-kind total, map rule, M1 scope single-source `Normalized`); **rev 45** accepts the mandatory-map invariants, with the optional-kind map rule still open | `[INT]` + `[OWNER]` T03 |
| `ArtifactRecord` shape | `{kind, bytes}` | `{kind, source: Option<SourceId>, bytes, raw_offsets: Vec<u64>}` with total `requires_map`; rev 22 records the shape change and the map authority the rev-21 delta omitted. **Rev 44 (accepted subdecision):** the `{kind, source, bytes, raw_offsets}` shape and the **total eight-`ArtifactKind`** map rule are **selected** (map-mandatory `Normalized`/`Spliced`/`CommentFree`/`Preprocessed`; map-optional `Assembly`/`Object`/`Snapshot`/`Trace`), M1 exercised scope only single-source `Normalized`; exact `raw_offsets` invariant/error mapping and enum numeric codes still open. **Rev 45 (accepted subdecision):** the mandatory-map invariants `raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds require a valid source; the optional-kind map rule, source-versus-payload equality, and exact error classification/numeric codes remain open | approve shape + who authorizes the map (T03 finding 5) | `[INT]` + `[OWNER]` T03 |
| `ArtifactKind` writers | `{Preprocessed,Assembly,Object,Snapshot,Trace}` | M1-produced `{Normalized,Spliced,CommentFree}` (T03-only); `Preprocessed` declared-not-produced; `Assembly`/`Object`/`Snapshot`/`Trace` have **no M1 writer** (Part B/Host) | approve M1 scope; Part B owner is a blocker (T03 finding 3) | `[INT]` + `[OWNER]` T03/T11 |
| `ir.functions/blocks/values/instructions` | placeholder | T09 records; **proposed** op table, **not hashed today**; to be hashed at `/6` (rev 25 F8/rev 26) | approve op table/terminator rules (§H) | `[INT]` + `[OWNER]` T09 |

### 3.3 Hash, limits, and schema

| Shared interface | Current (`/5`) | Rev-21 candidate | Requested decision | Authority |
|---|---|---|---|---|
| Hash scope | `hash_excludes=…group-declared-store-fields…`; group fields hash-excluded (verified `/5` fact — rev 48: present in `CONTRACT_VERSION`, `contract.rs`, `README.md`) | new `M1AppendSchema` section inside `FrozenSchema::encode`, **proposed to be hashed at `/6`** (not hashed today — rev 25 F1/F8) | **Rev 50 (`[USER]`, 2026-10-05, critical hash-scope decision): the two-tier model is accepted** — `StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed participating in the `/6` contract hash**, and **post-seed runtime `declare()` extensions stay excluded** from the frozen hash (captured/validated by runtime snapshot/schema mechanisms). At `/6` integration T01 must **atomically** update [COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md) §4 wording, the `hash_excludes` **semantic description/token** (scoped to **post-seed runtime declarations**, not the frozen M1 seed), and `FrozenSchema::encode`/contract code plus the freeze test, **preserving the frozen `/5` hash and history**. **No `/6` values/hash/counts/code//5 change; row I stays PENDING; T01 still co-freezes the M1 seed values after the owners.** Superseded option context (rev 48): the former three unselected options were hash `M1AppendSchema`, do-not-hash + manifest fix, or explicit two-tier split — the **conceptual scope is now settled as the two-tier split**, but the **numeric inventory and self-consistency/freeze test implementation remain pending** | `[USER]` + `[INT]` |
| `StoreSchema` | `foundation()` + runtime `declare` | seeded from frozen `M1AppendSchema`; seed-equality test | approve seed model; **rev 50 (`[USER]`, 2026-10-05):** the **seed (`foundation + M1AppendSchema`) is hashed** while **post-seed runtime `declare()` extensions are hash-excluded** and validated via runtime snapshot/schema mechanisms; exact seed contents/counts and the seed-equality/self-consistency test remain **T01 `/6` co-freeze after owners** | `[INT]` |
| Limits | existing set | `max_inflight_per_tick` (default 1), `stage_queue_bound`, `max_const_bits`, `max_task_progress`; **rev 35 (H9): `max_inflight_total` is removed from the `/6` candidate in principle** (per-tick dispatch is bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; pending T01 integrator acceptance); **rev 44 (accepted subdecision): `max_const_bits` origin is a hashed `Limits` value** (`limits.max_const_bits` participates in the frozen-contract hash), M1 cap/default 128, `config` rejects values > 128, explicit task-input projection to the restricted T08 chip (exact wire field/diagnostic code/hash encoding remain `/6`); **rev 51 (integration-selected candidate default under user delegation, 2026-10-05): `max_dispatches_per_tick` is dropped from the candidate limit inventory and its validation — `max_inflight_per_tick`/quota is the sole per-tick dispatch-count bound** (grounded in the rev-35/rev-38 H9 direction and the rev-49 audit finding that the two limits duplicated each other; **not** an owner/T01 `[INT]` sign-off or a schema freeze; T01/T02 must confirm/co-freeze the exact names/defaults/codes at `/6`) | approve additions and bounds | `[INT]` + `[OWNER]` for const bound |
| `ConfigError` | existing | `MaxTicksOverflow` + quota/queue/fairness variants | approve family additions and codes | `[INT]` |
| `CommitError` | existing | adds append-batch/cross-task/pipeline variants | approve classification and codes | `[INT]` |
| `ManifestError` | `StoreOwnerViolation` | `StageUnassigned`, `StageLayerMismatch`; `ChipId`-keyed allowlist | approve registration-time family | `[INT]` |
| T01 §4 wording | "control chip selects", "commit chip writes", `tasks.active` | "control chip computes; dispatcher applies"; backend writes | **doc-only `/6` inventory**, no `/5` edit here | `[INT]` `[DOC]` |
| `advance_tick` | infallible | non-wrapping + `control.tick_overflow` (defensive) | approve or keep `advance_tick` unchanged | `[INT]` |

> **Counts to reconcile before freeze (do not assume they are right).** The rev-20
> candidate states 19 draft families = 18 arena-backed + `Name`; 27 `RecordRef`
> variants / wire tags 0–26; a separate `RecordFamily` ordinal inventory. The
> **rev-48 read-only audit confirms the checked-in `/5` code fact is 24
> `RECORD_KINDS` / 24 `RecordRef` variants (tags 0–23), with no `RecordFamily`
> type**; the 19/27/27 figures are therefore **proposed/unverified** draft counts,
> **not** established, and the future `/6` count is **not** claimed to equal the
> current 24. The artifact, id, store, and rule counts are asserted in multiple
> places and must be machine-checked by `M1AppendSchema` self-consistency tests
> before `/6`.

---

## A. Pipeline scheduling `[INT]` `[USER]` `[OWNER:T02,T13]`

**Request A1 — preserve `/5` as current; adopt ADR-0002 only as PROPOSED.**
[ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) is
**PROPOSED** and authorizes nothing. `/5` (the checked-in foundation; T01 §4 order
`(phase priority, enqueue ordinal, TaskId)`) remains current. The user selected
the **deterministic bounded sequential stage pipeline** as the rev-21 working
basis **in principle** (2026-10-04), but the pipeline may be *designed* now and is
**inert until `/6`**; no chip wave may implement or depend on the pipeline
registers before the freeze.

**Request A2 — stage tasks plus explicit committed cursor/join state.** Cross-tick
work is an explicit staged pipeline: each task kind is assigned to exactly one
`StageId`; each stage has a bounded ready queue; continuations/fairness cursors/
awaited-child sets/joins are bus registers over committed IDs (or own-task-batch
draft keys, §D) and are snapshot-encoded. **Decide:** exact stage set/names
(candidate: `source_pp, lex, parse, symbol_type, sem, const, ir, machine,
artifact`), whether stages mirror T01 §2 store partitions one-to-one, and whether
the canonical queue is one partitioned vector or separate vectors.

**Request A3 — sequential, no threads, bounded multi-dispatch.**
`max_inflight_per_tick` defaults to **1**; multiple in-flight tasks are executed
**sequentially** by the single CPU backend, never concurrently. Confirm the
guardrail rule that quota `> 1` is a **measured, post-freeze, integrator-accepted**
change only (guardrails §6.4; ADR-0002 §2.9).

**Request A4 — quota-1 baseline is a semantic projection, not byte equality.**
The scheduler snapshot encodes stage queues, in-flight set, `dispatch_cursor`,
`stage_assignment_version`, and `PipelineMetrics`; therefore byte-identical
quota-1 snapshots are impossible. Define the quota-1 gate as the **canonical
semantic comparison projection** (semantic records/results/diagnostics + semantic
trace, excluding scheduler-only registers), with a separately deterministic/
replay-identical scheduler snapshot. **Decide the exact exclusion list** — it must
not become a blanket escape hatch that lets semantic drift pass.

**Request A5 — one ordered atomic commit per tick.** All dispatched tasks'
proposals are sorted by `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal
index)`, validated as a batch, capacity-preflighted, then applied infallibly. A
failed batch commits nothing. **Rev 25/26 F2:** "each dispatched task fails exactly
once" was the **intended** batch generalization, **not** an established mechanism.
The frozen `/5` `routing.rs` `fail_selected` single-task obligation is
**preserved and verified**: the failed task **always** transitions to
`TaskState::Failed` — it attaches a committed `DiagnosticId` when diagnostic/record
capacity allows and otherwise the `TaskState::Failed(DiagnosticId::NONE)` sentinel,
so it is never stranded or `Running`. **Rev 34 (user direction in principle,
2026-10-04):** the user **selected the batch-failure recovery direction in
principle** — on a failed atomic semantic batch commit, transition **every**
dispatched task, in **dispatch order**, **exactly once** to `TaskState::Failed`,
attaching a committed `DiagnosticId` when diagnostic/record capacity allows and
otherwise the `TaskState::Failed(DiagnosticId::NONE)` sentinel, so **every**
dispatched task leaves `Running`; **clearing the in-flight set is not itself a
transition**. This **generalizes the verified deterministic single-task fail
transition** with the same optional-diagnostic/`DiagnosticId::NONE` sentinel
semantics (a proposed generalization, not verified). **Rev 42 (user authority
decision, 2026-10-05; H6 scope + mechanism selected):** the user **rejected** the
§9B row-A proposal to defer the H6 mechanism to a later CDR and selected a
**quota>1-capable bounded batch recovery mechanism to be frozen in `/6`**:
**pre-dispatch errors before any state mutation leave the affected tasks `Ready`**;
a **semantic batch commit failure commits no semantic state**; then a
**deterministic bounded recovery mutation processes the dispatched tasks once in
dispatch order to `Failed`**; **no pre-reservation of N diagnostics**; a
**per-task diagnostic attempt** uses `DiagnosticId::NONE` when capacity is
insufficient; and a **state guard prevents the duplicate transition**. This is an
**explicitly selected H6 mechanism** (not a mere direction). **Still `[INT]`/co-freeze:**
the **exact atomic-commit realization requires T01 integrator approval**, and
the H9 limits/stages/hash/error numeric inventories and
the T02/T13 owner fixtures/sign-offs remain **pending**; the batch behavior is a
**selected mechanism awaiting `/6` freeze**, not implemented code. The alternative
(a different T01-approved atomic terminal path, e.g. reserve bounded terminal
diagnostic slots before dispatch) is retained as an implementation option and is
**not** the selected mechanism (the user explicitly selected the
no-pre-reservation recovery). Single-task behavior is `/5`-verified; the batch
behavior is **selected in `/6` but not implemented**. **Rev 35 (user direction in principle, 2026-10-04;
H9):** separately, the user accepts **removing `max_inflight_total`** from the `/6`
candidate — sequential per-tick dispatch is already bounded by
`max_inflight_per_tick`/quota, the per-stage queues, and `max_tasks_total`;
`Waiting` tasks are not in-flight; and a future cross-tick `Running` mode would
require a **separate CDR**. The dispatch **batch bound stays the per-tick
`max_inflight_per_tick` quota**; `tasks.in_flight` remains an **ephemeral per-tick
scheduler batch only** (candidate), populated from the tick's `SelectionBatch` and
cleared at latch **only after every dispatched task has a terminal/`Waiting`/
`Progress` outcome or the H6 recovery has transitioned every dispatched task
exactly once**. The dispatcher's **pre-worker `Ready→Running` mutation** is
**distinguished from the one ordered atomic semantic commit**. This is a
**direction only, pending T01 integrator acceptance**; no bound/config/hash is
frozen, and the residual H9 `[INT]` items (the dispatcher-`Ready→Running`-vs-
ordered-atomic-commit relationship and the residual-set semantics) stay open.

**Request A6 — fairness, backpressure, cancel, fail, replay, limits, metrics.**
Decide: fairness policy (one-per-stage vs round-robin vs weighted) and whether a
persistent fairness cursor is semantic state; backpressure semantics (defer vs
reject vs both) and per-stage vs global computation; cancel re-check (recommended
once per tick); commit-failure scope (whole batch, recommended); and the inner
`PipelineMetrics` encoding and its register location (`bus.report.metrics`
selected, inner shape residual). All limits are checked/saturating before
mutation; exhaustion is a structured diagnostic, never a panic or silent drop.

**Request A7 — reconcile `SelectionBatch` / report / wires / CT07 / transitions.**
The following are **not yet consistent** and must be closed together:
- `SelectionBatch`/`SelectionEntry` shape vs the removed singular `Selection`; the
  candidate states `report.rs` **does not exist today**, so all relocations are
  proposed, not existing.
- `wires.selection` (canonical batch) vs `wires.selected` (quota-1 projection) vs
  `control.selected`/`tasks.active` (compatibility views) — declare the single
  writer of each.
- `TickReport`/`TickOutcome`/`Resolution` and the `fail_selected` path vs the
  canonical `bus.report`; host `Trace` must become a derived view.
- CT07 resume/join semantics vs `AwaitChildren`/`WaitSet` (duplicate
  representation, §D) and who reinserts a resumed parent into `stage_queues`
  (CT07 decision vs the commit-apply realization; **not** CT09/CT10). Rev 21
  selects: the **join decision is realized in the commit-apply phase**, which
  reinserts the resumed parent into its own `stage_queues[stage]` before latch,
  so it is selectable next tick; CT07 is decision-only (rev 21, §D3).
- CT09 phase advance condition (all stage queues empty/waiting) vs CT10
  progress/budget over the batch; CT12 recovery per dispatch; CT13 cancel of an
  in-flight set, idempotent.
- Exactly-one-transition-per-dispatched-task invariant, including empty proposal
  vectors, at every quota (`TaskNotTransitioned`).

**Request A8 — single writer per wire and one canonical report shape.** Declare
exactly one writer per wire: `wires.selection: SelectionBatch` is written by
**CT03's adapter only**; `wires.selected: Option<TaskId>` is the **dispatcher's**
quota-1 projection; `wires.proposals` is written by each routed worker's adapter
only; `bus.report` is written **only** by the driver's step 6 (sole append site).
The canonical report shape is `TickRecord { dispatched: Vec<TaskId>, metrics:
PipelineMetrics, selected: Option<TaskId> }` with `dispatched` canonical and
`selected` a derived quota-1 view. No other component writes any of these.

**Request A9 — selection errors are not `CommitError`; categories fixed.** The
pipeline bounds split into three carriers, none of which is invented as a
`CommitError`:
- **`ManifestError`** (registration-time): `StoreOwnerViolation` (keyed by
  `ChipId`), `StageUnassigned`, `StageLayerMismatch`.
- **Dispatcher/scheduling failure** (pre-dispatch, before any store mutation;
  fails the affected/selected tasks and yields a structured tick diagnostic, not a
  `CommitError`): `SelectionBatchOverflow`, `DuplicateSelection`,
  `DispatchBudgetExceeded`. **Rev 35 (H9):** `InflightQuotaExceeded` is **removed
  from the candidate** with `max_inflight_total` (retained only as the historical
  rev-21/22 name; pending T01 integrator acceptance); the per-tick dispatch bound
  stays `SelectionBatchOverflow` on `max_inflight_per_tick`. **Rev 54:** with the
  rev-51 delegated default, `DispatchBudgetExceeded` **and** its
  `max_dispatches_per_tick` bound are likewise **dropped from the candidate**
  (retained as the historical rev-21/22 name); the operative pre-dispatch error
  candidate set is `SelectionBatchOverflow` and `DuplicateSelection` only.
- **`CommitError`** (commit-path reject-before-apply): `BackpressureCapacity`
  (enqueue/queue bound checked in phase 3) and `CrossTaskWriteConflict`.
The exact names/numeric codes remain a `/6` item; the *classification* is fixed
here so selection-time and commit-time errors are not conflated.

**Status honesty for A.** The rev-20 own-batch continuation/`AwaitChildren`
protocol is **not claimed fully solved**; it is an owner-blocked mechanism. Open
decisions: exact stage set/names; fairness policy; backpressure defer/reject;
cancel re-check; metrics encoding; quota-`>1` measurement; `SelectionEntry` final
shape; whether a persistent fairness cursor is semantic state; whether in-flight
tasks may park across ticks (selected M1 behavior: single transition, `Waiting` not
`Running`). Candidate errors: `StageUnassigned`, `StageLayerMismatch`,
`BackpressureCapacity`,
`DuplicateSelection`, `SelectionBatchOverflow`, `CrossTaskWriteConflict`. **Rev 35
(H9):** `InflightQuotaExceeded`/`max_inflight_total` is **removed from the `/6`
candidate** in principle (pending T01 integrator acceptance), so it is no longer
listed as a candidate error; the per-tick dispatch bound stays
`SelectionBatchOverflow` on `max_inflight_per_tick`. **Rev 54:** with the rev-51
delegated default, `DispatchBudgetExceeded` and its `max_dispatches_per_tick`
bound are likewise **dropped from the candidate** (historical rev-21/22 name
only), so the operative pre-dispatch error set here is `SelectionBatchOverflow`
and `DuplicateSelection`.
Candidate tests: `quota1_canonical_projection_equivalence`,
`scheduler_snapshot_replay_identical`, multi-inflight determinism, cross-task
conflict ordering, backpressure bound. **Closure criterion:** an accepted list of
stage names + fairness/backpressure/cancel rules + error-code inventory + the
quota-`>1` measurement plan, all frozen in one `/6` and gated by T02/T13 owner
review. **Rev 22/23 (H6, H9):** the in-flight bound is **not** claimed resolved. The
residual-set semantics of the in-flight set (what remains after
a partial or terminal capacity failure) and the relationship between the
dispatcher's `Ready → Running` mutation (claimed all-or-nothing per batch) and the
ordered atomic commit are **explicit `/6` blockers** (`§18.6`); no single
phase/owner is asserted. Separately, clearing the in-flight set does **not** by
itself clear a task's `Running` state, so the whole no-`Running`/terminal
diagnostic mechanism is **BLOCKED pending T01** (the invariant "no task remains
`Running` at latch" is stated, not a resolved mechanism). **Rev 34 (user
direction in principle, 2026-10-04):** the user selected the **batch-failure
recovery direction** — every dispatched task, in dispatch order, transitions
exactly once to `Failed` (a committed `DiagnosticId` when capacity allows, else
the `DiagnosticId::NONE` sentinel), so every task leaves `Running`; clearing the
in-flight set is not itself a transition. This **generalizes the verified `/5`
single-task `fail_selected` sentinel semantics** (unchanged). **H6 therefore
moved from "no selected alternative" to "direction accepted in principle,
implementation BLOCKED pending T01".** **Rev 42 (user authority decision,
2026-10-05; H6 scope + mechanism selected):** the user **rejected the §9B row-A
deferral** and **selected an explicit quota>1-capable bounded batch recovery
mechanism to freeze in `/6`** — pre-dispatch errors before any state mutation
leave the affected tasks `Ready`; a semantic batch commit failure commits **no**
semantic state; then a **deterministic bounded recovery mutation processes the
dispatched tasks once in dispatch order to `Failed`**; **no pre-reservation of N
diagnostics**; a **per-task diagnostic attempt** uses `DiagnosticId::NONE` when
capacity is insufficient; a **state guard prevents the duplicate transition**.
**H6 therefore moves from "implementation BLOCKED pending T01" to "selected
mechanism to be frozen in `/6`":** the **exact atomic-commit realization, the
H9 numeric inventories, and the T02/T13 owner fixtures/sign-offs remain
`[INT]`/co-freeze and pending** (M1 proposal §7/§10.5/§18.4/§19.2/§20.1); the
mechanism is **not implemented**. **Rev 35
(H9):** the user also accepts **removing `max_inflight_total`** in principle — the
per-tick `max_inflight_per_tick` dispatch bound, the stage queues, and
`max_tasks_total` already bound work; `Waiting` is not in-flight; `tasks.in_flight`
is an ephemeral per-tick scheduler batch only; and the dispatcher's
pre-worker `Ready→Running` mutation is distinguished from the ordered atomic
commit. **H9 therefore moves from "BLOCKED (phase/owner undefined)" to "removal
direction accepted in principle, pending T01 integrator acceptance"**; the
dispatcher-`Ready→Running`-vs-ordered-atomic-commit relationship and the
residual-set semantics stay **open**. **Rev 31 (user decision A):** the
**own-task-batch draft-key vs Guardrails §6.1** *narrow interpretation* is now
**accepted in principle** (2026-10-04): a same-task `OwnBatch(DraftRef)` may exist
only as a **transient wire/proposal input before commit**; commit validates and
resolves it to a committed `TaskId` before any persistent state, and no durable
cursor/`WaitSet`/join may point at a draft, wire, or address. The guardrail text
is **unamended**; **T01 integrator implementation-confirmation remains pending**
(`§17.7`; M1 proposal §3.12/§19.4). `quota = 1` remains the
semantic-comparison-projection baseline; `quota > 1` requires measured before/after
tick counts **and** separate integrator acceptance.

---

## B. Ownership — single owner per field; the user-selected model `[USER]` `[INT]` `[OWNER:T03,T04,T05]`

**Request B1 — one writer per store/field/family (user-selected).** The accepted
[Guardrails §2.7](COMPILER_DEVELOPMENT_GUARDRAILS.md) bind: *"Exactly one owner per
store/field/record family. A second writer is a `StoreOwnerViolation` class
defect, not a shortcut."* The rev-20 candidate (§15.1 item 4 "C6", §8) selected
`sources.spans` as a **shared T03+T04** writer plus a pending T05 AST-span
writer. On **2026-10-04 the user selected the single-owner model in principle**:
**there is no shared-writer carveout, and the Guardrail is not amended.** The
decided field owners are:

- `sources.spans` — **T03 only** (source/PP boundary; the sole span producer).
- `sources.expansions` — T03 only.
- `pp.tokens` — T03; `names.entries`, `lex.tokens`, `lex.literals` — T04.
- `parse.nodes`, `tasks.continuations` — T05.
- `symbols.scopes`, `symbols.scope_events`, `symbols.symbols` — T06.
- `types.records` — chip-id-keyed single writer per row (T06 policy).
- `sem.records` — T07; `constants.records` — T08; `ir.*` — T09;
  `artifacts.fragments` — T03.

**Request B2 — T04/Token reuse committed T03 PP spans; no span writes.**
`PpTokenRecord.span` is written by T03. `TokenRecord.span` (T04) is a reference to
the **already-committed T03 `SpanId`** (the PP token's span); T04 creates no
`SpanRecord` and `lex.tokens` carries no new span identity. T04's own decoded
positions (literal spelling offsets, diagnostics) are derived from the committed
span, not appended to `sources.spans`.

**Request B3 — AST `Node` uses a first/last `TokenId` range; no T05 span write.**
`NodeRecord` carries `first_token: Option<TokenId>` and `last_token:
Option<TokenId>` (a token-level range) instead of a `span: SpanId`. T05 writes
only `parse.nodes` and `tasks.continuations`; it **never** appends to
`sources.spans`. A node's raw span is derived one hop: `first_token`/`last_token`
→ committed `TokenRecord.span` → `SpanRecord.source/start/end`. This removes the
T05 span-owner blocker entirely (the rev-20 "hard `/6` blocker" is closed by
decision, pending T05 field sign-off). Synthetic/fabricated node ranges are not
representable; parser diagnostics use the token-level range.

**Request B4 — cross-task conflict check scope.** The `CrossTaskWriteConflict`
predicate is needed only when multiple dispatches in one tick are enabled. Because
each field has exactly one owner, a same-field conflict can arise only between
**two tasks of the same owner group** or between **two patches to the same
record**; it is **not** a mechanism to bless shared ownership. A second *owner
group* naming the same field remains a `StoreOwnerViolation`. Same-field,
single-task-per-tick is the baseline; two same-owner-group append tasks to one
append-only field are permitted only when the appends are independent (ordered
append, no cross-record dependency), and any two patches to the same
`RecordRef` conflict. The batch-conflict audit owner is a **proposed** VF04
`AccessContractChip` extension; because that extension is **not defined today**,
the audit is recorded as a residual, not a defined verifier (§A7/§I).

**Request B5 — align owner allowlist and storage append semantics.** The
`ManifestRegistry` allowlist must be keyed by `ChipId` with a declared record-kind
constraint, not by group, and must exactly match the single-owner map; the
allowlist seed/signature is **proposed to be hashed at `/6`** (§8, §I; not hashed
today — rev 25 F1). Confirm the `StoreId`→arena mapping
is not 1:1 (`Sources`/`Symbols`/`Ir` are multi-arena) and that per-arena capacity
is keyed by backing arena, never `StoreId`.

### Span-ownership alternatives (recorded; S3 rejected, S1/S0 selected in principle)

| Option | Description | Guardrail conformance | Status |
|---|---|---|---|
| **S1 (selected in principle)** | **Single span owner = T03.** T04 and T05 do **not** request or write spans: T04 reuses the committed PP span on its `TokenRecord`; T05 derives node ranges from committed `TokenId`s. | Conforms, **no** extra span-service task and **no** T04/T05 span write. | **User-selected**; T03 owner sign-off pending |
| **S2** | Split by field (`pp_spans`/`lex_spans`/`ast_spans`), each single-writer. | Conforms, but adds fields/arenas and splits one global span sequence. | Rejected for M1 (S1 is simpler and needs no new field) |
| **S3** | Shared `sources.spans` writer with a `CrossTaskWriteConflict` predicate. | **Non-conforming** with guardrails §2.7 unless the user amends the guardrail. | **Rejected** (guardrail unamended) |
| **S0 (selected sub-option)** | Token-boundary AST nodes (first/last `TokenId`; spans derived) vs full-range node spans. | Full-range spans require a T05 span writer; token-boundary conforms with **no** T05 span write. | **User-selected**: token-boundary |

**Authority classification:**
- `[USER]` — the single-owner decision and the no-carveout rule (2026-10-04).
- `[INT]` — field/arena naming, owner-allowlist mechanism, `StoreId` mapping.
- `[OWNER]` — T03/T04/T05 acceptance of the exact fields, the committed-span
  reuse protocol, and the token-range node derivation. All pending.

**Closure criterion:** one writer named per appended field, the T03 span
producer + T04 committed-span reuse + T05 token-range derivation specified and
tested, the `ChipId` allowlist rows matching, and the guardrail unamended.

---

## C. T03 / T04 / T08 — artifacts, literals, and the const handoff `[OWNER:T03,T04,T08]` `[INT]`

**Request C1 — complete `ArtifactKind` inventory, classification, source map,
hash, tests, and PP08.** *(rev 21 fixes the `Preprocessed`/M1 ambiguity.)*
- Confirm the **total 8-variant** set: checked-in `{Preprocessed, Assembly,
  Object, Snapshot, Trace}` plus `{Normalized, Spliced, CommentFree}`.
- Confirm the **total `requires_map`** predicate is a **total** function, **proposed to be hashed at `/6`** (not hashed today — rev 25 F1):
  map-mandatory `Normalized`, `Spliced`, `CommentFree`, `Preprocessed`;
  map-optional `Assembly`, `Object`, `Snapshot`, `Trace`.
- **Fix `Preprocessed` M1 status (rev 21).** `Preprocessed` (PP28's re-lexable
  final artifact) is **declared but NOT produced by the M1 fixture**. The
  M1-produced artifact kinds are exactly `Normalized`, `Spliced`, `CommentFree`
  (all map-mandatory). `Preprocessed` remains a declared map-mandatory kind and is
  an **explicit unexercised gap** for M1, not a pass; it is distinct from
  `Normalized` (PP01 buffer). This is the single authoritative statement; the
  draft's internal ambiguity is removed.
- Confirm `ArtifactRecord` shape: `source: Option<SourceId>`,
  `bytes: Vec<u8>`, `raw_offsets: Vec<u64>` with `len == bytes.len()+1`,
  `[0] == 0`, non-decreasing, `last <= source.bytes.len()` for mandatory kinds;
  empty for optional. The **source equality** is enforced pre-mutation as
  `ArtifactSourceMismatch`/`ArtifactSourceMissing`.
- **Rev 45 — mandatory-map invariants accepted (user subdecision, 2026-10-05).**
  The user accepts the **mandatory-map invariants**
  `raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing,
  last `<= source.bytes.len()`, and that **mandatory kinds require a valid
  source**. **Still open:** the **optional-kind map rule** (empty vs another
  explicit rule — the prompt allowed either and the user did not choose), the
  **source-versus-payload equality** rule, and the **exact artifact error
  classification/numeric codes** (`ArtifactSourceMismatch`/
  `ArtifactSourceMissing` remain proposed names, not frozen). No enum numeric code
  is accepted.
- Confirm the map + `requires_map` + `artifact_kind_name` (total 8) are
  **proposed to be hash-pinned** in `M1AppendSchema` (prospective `/6`; not pinned
  today), and that logical offsets remap to raw `SourceRecord.bytes` before any
  `SpanDraft` is formed.
- Confirm `PP08 MacroUndefChip` is an **explicit unexercised gap gate** (not a
  pass). `M1-PP-08` is the artifact-map fixture and is **not** chip `PP08`; both
  facts are recorded, and `M1-PP-08` has a gate column (`T01 freeze`).
- **Closure:** owner-accepted variant list, total `requires_map`, source-equality
  and map checks, hash entry, `M1-PP-08` test, PP08 gap gate.

**Request C2 — literal record and typed handoff preserving T08 semantics
(user-selected direction + accepted allocation; H1 accepted in principle).** The
T08 task contract defines `ConstantRequest(node, required_kind)` →
`ConstantResult(bits/float/symbol+addend, legality)`; T04's `LX08` selects a
**typed candidate type**. On **2026-10-04 the user selected the typed
committed-`LiteralRecord` handoff in principle**: T04 commits a `LiteralRecord` in
`lex.literals`, and T08 consumes `RecordRef::Literal` and commits the `ConstRecord`;
the handoff must preserve the semantic information `node`, `required_kind`,
`legality`, and the `LX08` candidate type as co-freeze requirements. On the **same
date the user also explicitly accepted the recommended allocation split in
principle (`接受拆分（推荐）`, H1)** as the `/6` revision working basis.

> **H1 — allocation accepted in principle, not frozen.** The allocation of the
> four items across (a) a per-literal `LiteralRecord`, (b) a sem-stage per-use
> `ConstantRequest`, and (c) a `ConstantResult` is now recorded as the
> **user-accepted-in-principle revision working basis**:
> - the committed T04-owned `LiteralRecord` is a **per-literal lexical candidate
>   fact** carrying spelling/radix/suffix/magnitude/sign and the **`LX08`
>   candidate type** (no `node`/`required_kind` — no AST exists at the lex stage);
> - the post-parse sem-stage per-use `ConstantRequest` **carries the committed
>   `RecordRef::Literal` plus `node` and `required_kind`** — so the two flows do
>   **not** both hand `node`/`required_kind`: only `const.evaluate` (sem stage)
>   does, while the T04-only `const.literal-decode` flow carries just the
>   committed literal;
> - the `ConstantResult` carries the committed `RecordRef::Const` reference and
>   `legality`.
>
> **Rev 25 F3 — ambiguity resolved:** the rev-22 `ConstantRequest { node,
> required_kind }` reading and the H1 row's `ConstantRequest { literal, node,
> required_kind }` reading were in conflict. The operative proposed shape is the
> latter (a request references the committed literal; the literal is not
> re-embedded). The exact field/enum/type encoding (`RequiredKind`, the `legality`
> carrier, the candidate-type field) is **still unfrozen**.
>
> This is a **revision working basis, not a freeze** and **not code/chip
> authorization**. The exact `LiteralRecord`/`LiteralSuffix`/`LiteralKind`/
> `ConstantRequest` enum variants, where the candidate type is stored, and the
> committed `legality` carrier remain a **T04/T08 `/6` co-freeze blocker**; the
> shape/schema stays **owner/integrator pending**, as do T01 integrator acceptance
> and T03/T04/T08 owner sign-off. Whatever shape is frozen must keep `Payload`
> **`RecordRef`-only** and **T08 the single `constants.records` writer**.
> `[rev 23 H1; user accepted-in-principle 2026-10-04; rev 25 F3]`

> **Rev 45 — `LiteralRecord` exact ordered fields + M1 enum/scope accepted as
> selected subdecisions (user, 2026-10-05).** These are **selected shapes to be
> frozen in `/6`**, not a `/6` freeze, not an accepted contract, and not code/chip
> authorization; the full-bundle C sign-offs remain pending.
> - **C/T04/T08/T01 — exact ordered `LiteralRecord` fields:**
>   `token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`,
>   `suffix: LiteralSuffix`, `value: Vec<u8>` (big-endian magnitude),
>   `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`.
>   There is **no `node` and no `required_kind`** (those stay on the sem-stage
>   `ConstantRequest`).
> - **C/T04/T08/T01 — M1 enum/scope:** `LiteralKind {Integer, Character, String}`
>   with **only `Integer` produced in M1**; `LiteralSuffix {None, U, L, UL, LL,
>   ULL}` with **only `None` produced in M1**; radix domain `{2, 8, 10, 16}` with
>   **M1 decimal only**; `Lx08CandidateType` is **symbolic/target-independent**
>   with M1 literals 2 and 3 typed `Int` and **no bit width**.
> - **Still open (deliberately not invented):** the **complete `Lx08CandidateType`
>   member set and all numeric encodings/tags**. The prompt did not enumerate them,
>   so none are asserted here; they remain a T04/T08/T01 co-freeze item.
>
> `[rev 45; user selected 2026-10-05]`

> **Rev 53 — `Lx08CandidateType` M1 vocabulary: integration-agent selected candidate
> default under explicit user delegation (2026-10-05; not an owner/T04 or T01 `[INT]`
> signoff, not a `/6` freeze, no code).** Grounded in a **read-only review of the
> exercised M1 integer literal subset**, for the **exercised M1 literal subset**
> `Lx08CandidateType` is the **closed one-member set `{ Int }`** and the M1 literals
> `2`/`3` are **target-independent `Int` with no bit width** (the `candidate_type`
> value on the committed `LiteralRecord`; not a target type/ABI type/`TypeId`). Literal
> **forms outside the exercised M1 subset** must **not silently default to `Int`** and
> are **explicitly unsupported/deferred** until their categories/rules are specified;
> future categories **append symbolic members/rules without reinterpretation**. This
> **does not** claim the **complete future C candidate vocabulary**, defines **no
> numeric tag/encoding**, and is **not** the T04/T08 owner or T01 `[INT]` signoff or
> `/6` freeze; the **complete `Lx08CandidateType` member set/numeric encodings remain
> open**. `[rev 53; delegated candidate default 2026-10-05]`

**Request C3 — single T08 `constants.records` writer.** Confirm that T04 **never**
writes `constants.records` and that T08 evaluates the committed literal under
the `ConstRecord` shape and the checked-op rule, so T08 does not
re-select the `LX08` candidate type. **Rev 54:** the `ConstRecord { ty, value: i128 }`
spelling below is **historical** — the exact value **carrier** is an **open `/6`
co-freeze** item (§G2); whether it is `i128` or another representation is **not
selected**. Under the user-accepted-in-principle H1
allocation the `LX08` candidate type is carried on the committed `LiteralRecord`
and `required_kind` on the sem-stage `ConstantRequest`, which references the
committed `RecordRef::Literal` (rev 25 F3); the **exact carrier shape**
(`LiteralRecord` fields — incl. the proposed `candidate_type` — and
`ConstantRequest` fields, and whether `required_kind` is re-derived) remains the
**T04/T08 `/6` co-freeze item** (shape/schema owner/integrator pending), together
with T01 integrator acceptance. **Rev 45 (selected subdecisions, user,
2026-10-05):** `RequiredKind` M1 enum is **only `IntegerConstantExpression`**, and
the `ConstLegality` result-field values are **`Legal`, `NotConstantExpression`,
`Unsupported`**. Future C constant-expression purposes must add **appended variants
and/or new rules**; the lexical candidate type (`Lx08CandidateType`) is **not** to
be repurposed for them. The `LiteralRecord` ordered fields and M1 enum/scope are
fixed as in the rev-45 block above; the **complete `Lx08CandidateType` member
set/numeric encodings remain open**.

### Literal/const handoff alternatives

| Option | Description | Impact | Status |
|---|---|---|---|
| **H1 (user-selected in principle; allocation accepted in principle 2026-10-04)** | T04 commits `LiteralRecord` in `lex.literals`; T08 reads it and commits `ConstRecord`; the handoff preserves `node`/`required_kind`/`legality`/`LX08`. **Accepted allocation (working basis):** `LiteralRecord` carries the per-literal lexical facts + `LX08` type; `ConstantRequest { literal: RecordRef::Literal, node, required_kind }` carries the sem-stage per-use items; `legality` in the result. | Adds `LiteralId`/`RecordRef::Literal`/`lex.literals`; keeps `Payload` `RecordRef`-only; requires T04/T08 to co-freeze the exact request shape. | **User-selected direction + accepted allocation in principle**; still requires T01 integrator + T03/T04/T08 owner co-freeze/sign-off; exact schema owner/integrator pending (H1) |
| **H2** | T04 commits a raw literal fact (spelling/radix/kind); T08 performs decoding **and** `LX08` selection. | Moves candidate-type selection to the const stage; splits `LX08` behavior across stages. | Rejected for M1 (conflates T04/T08 ownership) |
| **H3** | A dedicated committed `ConstantRequest` record (not `lex.literals`) carries literal + node + `required_kind`; T08 writes both it and `constants.records`. | One more family/arena; T08-owned request fact. | Not selected (extra family without a demonstrated need) |
| **Rejected** | Shared T04+T08 `constants.records` writer. | Two writers, contradicts guardrails. | **Rejected** |

**Closure criterion:** the user's H1 allocation is recorded here as accepted in
principle (2026-10-04); **owner acceptance of the exact `LiteralRecord`/
`ConstantRequest` shapes is still required and pending** (a T04/T08 co-freeze plus
T01 integrator item), the semantic information must be preserved, one
`constants.records`
writer; exact wire tags; a typed multi-source map decision; and `M1-CL-*` tests
rerun. **Rev 22/23/24:** the one-source limit and the non-M1 artifact writers are
named blockers (`§17.1`); **the H1 allocation is no longer a pending user
decision, but it is still not frozen** — the exact enum/schema shapes remain a
T04/T08 `/6` co-freeze item plus T01 integrator acceptance. **Rev 25 F3:** the
`ConstantRequest` shape is fixed in principle as `{ literal: RecordRef::Literal,
node, required_kind }` (the request references the committed literal) and the
`LiteralRecord` carries a proposed, unfrozen `candidate_type` field; the exact
enum/type encodings remain pending co-freeze. The rev-24 history is corrected to
state that the **proposed carrier shapes were drafted in rev 25**, not edited in
rev 24. **Rev 45:** the exact ordered `LiteralRecord` fields, the M1
`LiteralKind`/`LiteralSuffix`/radix scope, the symbolic `Lx08CandidateType` (M1
2/3 = `Int`, no bit width), `RequiredKind` (`IntegerConstantExpression` only), and
the `ConstLegality` values (`Legal`/`NotConstantExpression`/`Unsupported`) are
**selected subdecisions**; the complete `Lx08CandidateType` member set/numeric
encodings, the optional-kind artifact-map rule, source-versus-payload equality, and
the exact error classification/numeric codes remain **open**, and the overall C
bundle stays PENDING.

---

## D. T05 — continuation, join, and resume `[OWNER:T05]` `[INT]`

**Request D1 — exact continuation mapping (rev 21; `awaited` removed).** The
existing `ContinuationRecord` (`resume_kind`, `awaited`, `scope`) is extended
field-by-field to `production` (rename), `cursor: TokenId`, `context:
ParseContext`, `binding_power: u16`, `scope`, `parent`, `partial_children`,
`next_child_ordinal`, `previous`. **Rev 21 removes `awaited` from the
continuation entirely** (see D4). **Direction is fixed:** `Task.continuation:
Option<ContinuationId>` points from a task to its **current/next parse frame**;
`ContinuationRecord.previous: Option<ContinuationId>` points **back** to the
predecessor frame (committed-only, acyclic). The successor's
`ContinuationRef::OwnBatch` in an `Enqueue` resolves to the `ContinuationId`
minted for that same task's draft. **The exact field order and wire encoding
remain a T05 `/6` sign-off item** (not finalized; do not read this as resolved).
**Accepted in principle (user, 2026-10-04; user decision A; narrow interpretation):** the `OwnBatch` successor resolution and the "committed-only" predecessor reading now apply the user's **narrow interpretation** of [Guardrails](COMPILER_DEVELOPMENT_GUARDRAILS.md) §6.1: a same-task `OwnBatch(DraftRef)` may exist only as a **transient wire/proposal input before commit**, and commit validates/resolves it to a committed `TaskId`/record **before any persistent state**; no durable cursor/`WaitSet`/join may point at a draft, wire, or address. The guardrail text is **unamended**; **T01 integrator implementation-confirmation remains pending** (M1 proposal §3.12/§17.7 PIPE-5/§19.4/§20, T02, ADR-0002 §2.1 invariant 6). Still not a freeze.
Confirm parse state reuses `ContinuationId`/`Arenas.continuations` and adds no
family.

**Request D2 — own-batch reference resolution and phase-1/2 validation of
`ContinuationRef`/`ChildRef` before apply.** `ContinuationRef`
(`Committed`/`OwnBatch`) and `ChildRef` (`Committed`/`OwnBatch`) are same-task
only. Rev 21 requires **explicit validation in the no-mutation pass, before
apply**, of:
- `ContinuationRef::Committed(id)` — the `ContinuationId` exists and is live;
- `ContinuationRef::OwnBatch(k)` — `k` is within this task's own `AppendRecords`
  draft range and the draft family is `Continuation`;
- `ChildRef::Committed(id)` — the `TaskId` exists and is a committed child of this
  task;
- `ChildRef::OwnBatch(k)` — `k` indexes this task's own `Enqueue` list.
A violation is `AwaitChildrenRefInvalid`/`ContinuationRefInvalid` (names a `/6`
item) and rejects the batch. Commit phase 4 materializes the continuation and
resolves the successor's `OwnBatch` ref **before** applying the Enqueue, so no
same-tick cross-task draft reference exists; a cross-task index is
unrepresentable. **Accepted in principle (user, 2026-10-04; user decision A; narrow interpretation):** this own-batch key candidate, the same-task-only direction, and the "cross-task index is unrepresentable" reading now apply the user's **narrow interpretation** of [Guardrails](COMPILER_DEVELOPMENT_GUARDRAILS.md) §6.1: the own-task draft key is a **transient wire/proposal input only**, validated and resolved to a committed `TaskId` before persistent state, and no durable cursor/`WaitSet`/join points at a draft/wire/address. The guardrail text is **unamended**; **T01 integrator implementation-confirmation remains pending** (M1 proposal §3.12/§17.7 PIPE-5/§19.4/§20, T02, ADR-0002 §2.1 invariant 6). The pre-apply validation phases and exact error names remain T05 `/6` items.

**Request D3 — parent waiting/resume: realized in commit apply; reinsert into the
canonical stage queue (H2, H4).** On resume: **when all committed children of a
`Waiting` parent are `Completed`, the parent returns to `Ready`; when any
committed child `Failed`, the parent becomes `Failed` exactly once and is
**never** set `Ready`.** Rev 21 fixes **where this happens**: the **commit-apply
phase** performs the join reinsertion (it is the only component that mutates task
state before latch), and **CT07 is decision-only** (it computes/marks the join but
mutates nothing). The resumed parent is reinserted into its **own canonical
`stage_queues[stage_of(parent.kind)]`** (the parent task's **kind** — **H2**; not
`stage_of(parent.continuation.production)`), not `tasks.ready` (a derived view)
and not by CT09/CT10. The realization must **handle a parent whose `continuation`
is absent** (a parent task kind with no parse continuation is still resumable by
its kind). The parent is selected next tick. The exact **fail-fast vs
await-all-siblings / sibling-cancellation** policy remains an explicit T05/T02
`/6` blocker (not resolved here); the direction above (all `Completed` → `Ready`,
any `Failed` → `Failed`, never `Ready`) is the selected draft direction.
**Rev 42 (user authority decision, 2026-10-05; D subdecisions selected):** the
user accepts the join as a **commit-apply invariant** with **no new CT07 committed
carrier/family**, and **await-all children terminal before the parent decision**
(all committed children `Completed` before the parent decision; any `Failed`
fails the parent once). These two subdecisions are accepted; the **continuation
fields/wire encoding, the `WaitSet` `/6` formal T01 §4 supersession, the
`OwnBatch` exact preapply errors, the parse contract/tests, and the remaining D
items stay PENDING**.

**Request D4 — one source of awaited children: `WaitSet` only (rev 21).** The
rev-20 candidate had **both** `ContinuationRecord.awaited: Vec<TaskId>` and
`TaskState::Waiting(WaitSet { children })`. Rev 21 **deletes `awaited` from
`ContinuationRecord`/`ContinuationDraft`**; the **only** source of truth for
awaited children is `TaskState::Waiting(WaitSet { children: Vec<TaskId>,
host_request: Option<HostRequestId> })`. The join reads `WaitSet`, not a
continuation field, so a continuation cannot disagree with the task wait set. This
is the selected representation; no mirror is defined. **Accepted in principle
(user, 2026-10-04; user decision B):** `TaskState::Waiting(WaitSet)` is the **sole**
awaited-child-ID source and `ContinuationRecord` carries **no duplicate `awaited`**;
the user accepts in principle the request to supersede/clarify **T01 §4** in `/6`
(**no `/5` edit**; the `/5` conflict therefore still requires the formal `/6`
amendment). **T01 integrator and T05 owner acceptance remain pending.**

**Request D5 — progress, cumulative child ordinals, parse depth, PA20.**
Confirm: `Progress { ordinal }` strictly increases and is bounded by
`max_task_progress` (`NonAdvancingProgress`/`ProgressLimit`); `Progress`
reinserts into the task's **own `stage_queues[stage]`** (never `tasks.ready`) and
is selectable next tick; child ordinals are cumulative across committed +
same-batch siblings (`NodeOrdinalNotUnique`); `partial_children`/
`next_child_ordinal` are exact; parse depth reuses the existing
`limits.max_task_depth` mapped to `ParseDepthExceeded` (chip diagnostic, no new
limit); `ParseCursorDidNotAdvance` is a chip diagnostic; PA20's M1 path is
**unary-`+` only** (sizeof/alignof type-name variants not exercised).

**Request D6 — T05 span authority resolved by §B; no span write.** The T05 task
contract is `ParseRequest(production, token_cursor, scope_id, context,
binding_power)` → `ParseResult(node/declarator, next_cursor)`. Rev 21 selects
token-boundary AST nodes (§B): T05 writes **only** `parse.nodes` and
`tasks.continuations` and never `sources.spans`; `NodeRecord` carries
`first_token`/`last_token`. The `ContinuationRef`/`AwaitChildren`/`Progress`/
record-draft additions still require a T05 task-kind/result contract delta and
manifest/tests alignment, which remains a T05 `/6` sign-off item.

**Closure criterion:** T05 owner signs the exact `ContinuationRecord` fields/order/
encoding, the own-batch/`ChildRef` validation phases, the `WaitSet`-only
representation, join reinsertion in commit apply, and the parse task-contract
delta; tests `continuation_own_batch_resolution`, `await_children_own_batch`,
`continuation_ref_validated_preapply`, `continuation_chain_acyclic`,
`node_children_by_parent_sorted_unique_ordinal`, `node_token_range`,
`join_reinsert_stage_queue`, `parse_cursor_non_advance`, `parse_depth` pass under
`/6` fixtures. **Rev 22/23/31:** the `awaited`-vs-T01 §4 supersession is now
**accepted in principle** (user decision B, 2026-10-04: `WaitSet`-only; supersede
T01 §4 in `/6`; no `/5` edit) with **T01/T05 acceptance pending**; the CT07
committed join-decision carrier, the verifiable T04 PP-span provenance, and
the exact child-failure/sibling-cancellation policy remain named blockers (`§17.2`,
`§18.3`); node token links are **committed-only** (no same-batch `TokenDraft`); the
resume stage is `stage_of(parent.kind)` (**H2**), the realization handles an absent
`continuation`, all-`Completed` resumes `Ready` while any-`Failed` fails the parent
once and never sets `Ready` (**H4**), and join reinserts are counted in the
phase-3 bound. **Rev 42 (user authority decision, 2026-10-05):** the join as a
**commit-apply invariant** with **no new CT07 committed carrier/family** and
**await-all children terminal before the parent decision** are now **accepted
subdecisions**; the CT07 carrier question is therefore **resolved by selection
(no new carrier/family)**, while the continuation field/order/encoding, the
`WaitSet` `/6` formal T01 §4 supersession, the preapply error names, and the parse
contract/tests remain **pending**.

---

## E. T06 — scopes, symbols, types `[OWNER:T06]` `[INT]`

**Request E1 — exact `decl` node and active-scope lookup (rev 21; DOC-12
correction).** `decl` is fixed to the **`Identifier` `NodeKind` leaf** that
names the symbol (e.g. the `Identifier` node for `main`), **not** the
`Declarator`/`Declaration`/`FunctionDefinition` wrapper and not an arbitrary
descendant; it is the **identity/diagnostic location**. The lookup **visibility**
boundary follows C11 6.2.1p7: the point-of-declaration tuple is derived from the
**completion point of the complete declarator** (the declarator's `last_token`
span, reached from the `Identifier` leaf through the committed AST parent links)
→ `TokenRecord.span` → `(source, start, end)`, plus the leaf `NodeId`:
`(source, start, end, decl)`; the identifier leaf's own token range is **not** the
visibility boundary, and the declarator's full range is **not** used. Because
`NodeRecord` now carries a `first_token`/`last_token` range (§B), the
completion-token span resolves one hop to `TokenRecord.span`; the exact T05
`NodeKind` ownership/token-range semantics remain a T05 `/6` item. TY03 walks
only the **active scope chain** (`scope` upward to the file scope, never a
closed/sibling scope); tie rule: same scope → higher `SymbolId`; else innermost
active scope. The required `int n[n]` array-bound vs initializer contrast test is
recorded in the T06 package and the M1 proposal §13. Preserve `M1-TY-06`.

**Request E2 — scope lifecycle: new scopes, File Enter, boundary `at`, order
(rev 21; DOC-13 correction).** `ScopeEventRecord` semantics: exactly one `Enter` per `ScopeRecord`,
at most one `Exit`, Exit strictly after Enter in `ScopeEventId` order; `event.at`
resolves to a committed or same-batch Node; each new `ScopeDraft` gets exactly one
Enter and at most one Exit. A scope is identified by its **owner lexical node**
(the resolved `at` of its single Enter — the `TranslationUnit` node for the file
scope, the `Block` node for a block scope), **not** by `(parent, kind)`: sibling
blocks/functions may share `(parent, kind)` and are legal; only a duplicate
creation claiming an already-owned node is `ScopeLifecycleViolation`. A closed
scope retains its record, so the check never keys on a live arena record. **Rev 21 fixes the boundary representation without
inventing a `{`/`}` node:** the file scope's Enter `at` is the
**`TranslationUnit` node** and has **no Exit** in M1; a block scope's Enter/Exit
both reference the **`Block` node** (distinguished by `kind`), whose token range
supplies the boundary — `first_token` is the `{` and `last_token` is the
matching `}`. No fabricated boundary node is introduced; the existing node shape
plus the token range is authoritative. **H3 (ordering):** the file-scope `Enter`
must be emitted by a **T06 task that runs after the T05 `TranslationUnit` node is
committed** (`symbol_type` stage follows `parse`), and it **pins a committed
`NodeId` reference** — not a same-batch draft and not an uncommitted node. The
file-scope `Enter` is therefore **not a job-bootstrap action**; **rev 43**
accepts the trigger itself as the deterministic `parse.TranslationUnit ->
symbol_type.scope-enter` stage edge after the committed TU (no job-bootstrap),
so only the **bootstrap wiring beyond that accepted stage edge** (how the first
T06 task is scheduled once the TU node is committed) remains a **T06/`[INT]`
`/6` decision**, left open here. This matches the
scope-lifecycle rule that `event.at` resolves to a committed Node. The block-node
boundary `at` may resolve to a committed or same-batch `Block` node (T06). Exact
`ScopeEventRecord`/`ScopeDraft`
field order remains a T06 `/6` sign-off item.

**Request E3 — namespace miss semantics.** Confirm a lookup whose qualified name
resolves in a different namespace is a **miss** (`NoSuchSymbol`), not a conflict,
and the namespace is **derived** from `SymbolKind` (no stored field). Confirm
deferred namespaces (member, fourth namespace) and that `Label` is enumerated but
not exercised.

**Request E4 — canonical `int` reuse producer ownership and `types.records`
writer scope.** Confirm the chip-specific single-producer policy (`TY13` canonical
`int`, `TY17` function type), reuse of the committed canonical id by later tasks,
**no** commit-side structural dedup, and the chip-id-keyed allowlist enforcing it
at `ManifestRegistry::register` with `ManifestError::StoreOwnerViolation`. The
allowlist row is `(ChipId, StoreId = Types, field = "records", kind-constraint)`;
it is **proposed to be hashed at `/6`** via the allowlist seed/signature (not
hashed today — rev 25 F1). Later `TY13`/`TY17` tasks reuse
the committed id rather than appending a duplicate. The exact row set, seed, and
hash form remain T06 `/6` blockers. **`StoreOwnerViolation` is a `ManifestError`,
never a runtime `CommitError`.** **Rev 42 (user authority decision, 2026-10-05;
E subdecision selected):** the user accepts the **canonical `TypeId` reuse
mechanism** as a **deterministic bounded scan of the committed `types.records`
arena returning the lowest matching `TypeId`**, with **no hidden cache/index**.
The bootstrap and the rest of the E bundle remain pending; the T05 upstream
committed-`TranslationUnit` contract is acknowledged as an **upstream
dependency**.

**Request E5 — symbol diagnostics classification and deferrals (rev 21).**
Classify explicitly: `SymbolConflict` (exact `(name, scope, kind)` structural
duplicate) is a **commit-path pre-mutation rejection**; `RedeclarationConflict`
and any compatibility judgement are **chip diagnostics** (T06 `Fail`/
`DiagnosticDraft`), never `CommitError`s; a wrong-namespace lookup is a **miss**
(`NoSuchSymbol`, a chip result), never a `SymbolConflict`. Confirm no M1 general
compatibility model (`TY21`/`TY22` deferred). Confirm deferrals: `Function`
prototype scope, member/tag/label-as-scope, `CharKind` symbolic only (plain-char
signedness probe-gated), and **`TY08` (`TypedefRegisterChip`) unexercised by M1
in every document** (the `TY01→TY07→TY08→TY03` fixture is explicitly non-M1).

**Closure criterion:** owner signs `SymbolRecord`/`ScopeRecord`/`ScopeEventRecord`/
`TypeRecord` fields and enum variants, the lookup/lifecycle rules, the allowlist
rows, and the deferrals; tests `scope_event_cumulative_lifecycle`,
`scope_new_draft_cardinality`, `scope_file_enter_policy`, `scope_event_at_validated`,
`symbol_point_of_declaration_lookup`, `symbol_namespace_miss_not_conflict`,
`canonical_int_reused_id`, `function_type_reused_id`, char fixture. **Rev 22/23:**
the canonical `TypeId` reuse lookup mechanism and the namespace-carrier decision
are named blockers (`§17.3`); `RedeclarationConflict` is a chip diagnostic; the
point-of-declaration visibility derives from the **complete declarator's**
completion token (`last_token`) while the `Identifier` leaf stays the
identity/diagnostic location (DOC-12); and the **File-scope
`Enter` runs as a T06 task after the T05 `TranslationUnit` is committed, pinning a
committed `NodeId`** (H3), with the trigger accepted as the rev-43 deterministic
stage edge and only the bootstrap **wiring beyond that stage edge** a
T06/`[INT]` `/6` decision (not job-bootstrap).

---

## F. T07 — checked facts and conversion-plan alignment `[OWNER:T07,T06,T09]` `[INT]`

**Request F1 — T07-owned committed semantic carrier; do NOT consume a
T09-created `FunctionRecord` (rev 21).** The rev-20 candidate routed the
T07→T09 handoff through the committed `FunctionId`/`FunctionRecord`. `FunctionRecord`
is created by T09 `IR01` (the **ir** stage), **after** the **sem** stage, so any
T07 consumption of it is a temporal cycle. **Rev 21 removes that path.** The
selected candidate is:
- **Every checked node gets exactly one committed `SemRecord`, including the
  `Return` node and the `FunctionDefinition` node.** This **eliminates the
  rev-20 contradiction** in which the Return node was said to have no
  `SemRecord` while the checked return had to be persisted.
- The `FunctionDefinition` node's `SemRecord` carries the **checked function
  signature `TypeId`** (and the return conversion); the `Return` node's
  `SemRecord` carries the checked return operand type and its `ConversionPlan`.
- T09 reads the committed `SemRecord`s (T07-owned, present at sem time) and does
  **not** require a T09-created `FunctionRecord` to obtain semantic facts. The
  `FunctionRecord` remains a T09 IR record created in the ir stage for IR
  purposes only.

Alternatives considered: **F1b** (a T07 task result keyed by `NodeId`) is
workable but less inspectable/hashable; **F1c** (T09 creates `FunctionRecord`
first) is **rejected** (temporal cycle; makes IR an input to semantics). The
**exact carrier shape, whether a `FunctionContextId` exists, and the `SemRecord`
field encoding remain hard T07 `/6` owner blockers** — the choice above is a
candidate, not a signed-off shape. **Rev 42 (user authority decision, 2026-10-05;
F subdecisions selected):** the user accepts **no `FunctionContextId`** and that
`SemRecord` is the **committed materialization of `CheckedNode`, one per
`NodeId`**, with an **explicit committed typed link consumed by T09**. The
conversion matrix, VF06/T13, effects/value category, diagnostics/tests, and the
remaining F bundle stay **PENDING**.

**Request F2 — one shared conversion-plan type with explicit signedness ops
(rev 21).** There is **one** shared type `ConversionPlan { op: ConversionOp,
role: ConversionRole, from: TypeId, to: TypeId }`, co-frozen T06/T07/T09. T06's
`CastPlan` (TY29) and `ArgumentPlans` (TY28) are **role-labelled instances of the
same shape** (`role: Cast` / `role: Argument`), not a second type. Rev 21 extends
`ConversionOp` so IR lowering cannot lose signedness: the enumerated set is
`{Identity, IntegerPromotion, UsualArithmetic, Assignment, SignExtend, ZeroExtend,
Truncate, IntToFloat, FloatToInt, PointerToInt, IntToPointer, ToBool}` (exact set
a co-freeze item). T09 `IR07 ConversionLowerChip` must emit the matching
`sext`/`zext`/`trunc`/int-float/pointer/truth operation from the `op` — **not**
merely change the type (T06 note: "must not merely change the type in a way that
makes the IR lose sign/zero extend"). `ConversionRole` remains the required
subject of the `(node, role)` uniqueness rule. **Rev 46 (user decision,
2026-10-05; M1-minimal conversion scope selected):** only the conversion behavior
the **M1 fixture actually needs** is frozen, including the **identity/no-conversion
rule**; **integer promotions, float conversions (incl. `FloatToFloat`),
pointer-qualifier conversions, and other non-M1 conversions are explicitly
unsupported and deferred to a later append/contract revision** (explicitly
unsupported, not silently omitted). **No** exact `ConversionOp`/`ConversionRole`
closed variant list, `(op, role)` pairing, role→T09-chip mapping, or numeric
encoding is frozen or invented here; the enumerated set above remains a proposed
candidate and stays an open `/6` co-freeze detail.

**Request F3 — required `(NodeKind, role, op)` VF06 table (rev 21).** Distinguish
*uniqueness* (commit: **at most one** plan per `(node, role)`) from
*completeness* (VF06: a **required** conversion must exist). The candidate
required-conversion matrix, to be co-frozen, is:

| NodeKind | role | required plan | permitted `op` |
|---|---|---|---|
| `Return` | `Return` | required **only** when the operand type differs from the function return type; an explicit recorded `Identity` plan may be present when the owner records a no-op | `Identity`, `SignExtend`, `ZeroExtend`, `Truncate`, `IntToFloat`, `PointerToInt`, `IntToPointer`, `ToBool` |
| `BinaryExpression` | `Operand0`/`Operand1` | required **only** when the operand type differs from the usual-arithmetic common type | `IntegerPromotion`, `UsualArithmetic` |
| `Assignment`-family (future) | `Assignment` | required when source type differs from destination | `Assignment` |
| `Identifier` / `IntegerConstant` | `Result` | **optional** (M1 records no conversion when the type is already `int`) | `Identity` (recorded no-op only) |
| `FunctionDefinition` | **signature carrier (not `Return`)** | the node's `SemRecord.ty` carries the checked signature; **no** `Return`-role conversion is required on the `FunctionDefinition` node | `Identity` only if an explicit no-op is recorded |

**Rev 22 correction (T07 finding 4):** the `FunctionDefinition` row no longer
duplicates the `Return` role (the rev-21 table made `Return`/`Return` and
`FunctionDefinition`/`Return` contradictory). The `FunctionDefinition` node's
`SemRecord.ty` is the signature; the `Return` node owns the `Return`-role plan.
**VF06 stage binding is unwired:** no VF06 task kind/phase/layer is defined to
consume this matrix, so the matrix is a proposed verification obligation, not an
implemented verifier. The exact permitted-op set and the matrix remain a
T07/T13 `/6` co-freeze blocker. **Rev 46 (user decision, 2026-10-05; VF06 selected
in M1):** VF06 **`TypedAstInvariant` is in M1** — it executes **after** the
committed T07 `SemRecord`s and **before** T09 lowering and checks **M1
typed-fact/required-conversion completeness**. The **exact registered task
kind/stage/phase/interface remains a T01/T13 co-freeze/open item** and is not
asserted here; "VF06 is unwired" therefore now means **the M1 role is fixed but
its registration is still open**, not that VF06 is out of M1.

The matrix must not over-enforce: M1 `2+3` has `int` operands and `int` result, so
**no** operand conversions are required (consistent with `M1-SE-02`). VF06's
scope is exactly this table; the **exact permitted-op set is a T07/T13 co-freeze
blocker**.

**Request F4 — separate the T04 literal request from the semantic
constant-expression request (rev 21; H1 allocation accepted in principle).** The
candidate defines two distinct T08 request kinds and their stage dependencies;
**the split itself is the H1 allocation accepted in principle by the user on
2026-10-04** (working basis, not a freeze) — recorded here as the accepted
allocation, with exact variants still owner/integrator pending:
- **`const.literal-decode`** (T04→T08): carries the committed
  `RecordRef::Literal` (under the accepted allocation: a lexical candidate fact
  with the `LX08` candidate type; **no `node`/`required_kind`**) and asks T08 to
  produce the `ConstRecord`. Stage: **const**, depends on the committed
  `lex.literals`.
- **`const.evaluate`** (T07→T08): the **semantic** constant-expression request
  (`ConstantRequest::Literal { literal: RecordRef::Literal, node, required_kind }`
  / `ConstantRequest::Binary { node, op: ConstExprOp, lhs, rhs, required_kind }` —
  rev 25 F3 resolves the earlier `{ node, required_kind }` reading; the request
  references the committed literal(s), and the binary variant carries the committed
  expression node, T07's checked operator, and the committed operand refs; the M1
  variant set is frozen by the **rev-55 OPEN-03 T01/T07/T08 co-freeze**) for ICE
  legality/evaluation (`M1-CL-01`). Stage:
  **const**, depends on the committed T07 `SemRecord`/`NodeId` and the committed
  `lex.literals`.
**Accepted split (working basis, 2026-10-04):** under this allocation the two
requests do **not** both hand `node`/`required_kind` — only `const.evaluate` (sem
stage) does; `const.literal-decode` carries the committed literal + candidate type,
and `legality` is preserved in the **result**. Both use **committed** references
only (no same-tick cross-stage draft ref). The sem-before-const ordering (so
T07→T08 is a committed-ID next-tick handoff and no `T07→T08→T07` same-tick cycle
exists) is a separate selected direction. The exact request-kind names and
required-kind vocabulary remain a T07/T08 `/6` co-freeze blocker; this
accepted-in-principle allocation is **not a freeze and not code/chip
authorization**.

**Closure criterion:** owner-signed T07 committed semantic carrier (no T09
`FunctionRecord` cycle; every checked node including `Return`/`FunctionDefinition`
has a `SemRecord`), one shared conversion-plan type with explicit sign/zero/trunc
ops, the `(NodeKind, role, op)` VF06 matrix, the two constant-request kinds and
their stage ordering, and IR07 sign/zero-extend evidence. Tests
`checked_return_persisted`, `function_sem_carrier_committed`,
`conversion_plan_nested_links`, `conversion_role_required`,
`conversion_at_most_one_plus_vf_completeness`, `conversion_op_signedness`,
`effect_mask_empty`. **Rev 22:** the T09 `SemRecord` request/link, the
`CheckedNode`↔`SemRecord` family decision, the `FunctionContextId` choice, and the
`ConversionOp` FP/pointer-qualifier domain are named blockers (`§17.4`); VF06's
stage is unwired. **Rev 42 (user authority decision, 2026-10-05):** the
`FunctionContextId` choice (no `FunctionContextId`) and the
`CheckedNode`↔`SemRecord` family decision (`SemRecord` is the committed
materialization of `CheckedNode`, one per `NodeId`, with an explicit committed
typed link consumed by T09) are now **accepted subdecisions**; the `ConversionOp`
FP/pointer-qualifier domain, the VF06 matrix/stage, effects/value category,
diagnostics/tests, and the remaining F bundle stay **pending**. **Rev 46 (user
authority decision, 2026-10-05):** the `ValueCategory` discriminants (M1 fixture =
`NonLvalue`), the M1-only `EffectMask(0)` rule (nonzero typed
unsupported/diagnostic; bit classes reserved/unassigned), VF06 `TypedAstInvariant`
**in M1** (after committed T07 `SemRecord`s, before T09 lowering, M1
typed-fact/required-conversion completeness), and the **M1-minimal conversion
scope** (only M1-fixture-needed behavior incl. identity/no-conversion; integer
promotions, float conversions incl. `FloatToFloat`, pointer qualifier, and other
non-M1 conversions explicitly unsupported/deferred; no exact `ConversionOp`/
`ConversionRole` closed list, pairing, role→chip mapping, or numeric encoding
invented) are **accepted subdecisions**; the **exact VF06 registration**, the full
conversion matrix, and all conversion/effect numeric encodings stay **pending**.

---

## G. T08 — constants `[OWNER:T08]` `[INT]`

**Request G1 — chip diagnostics, distinct from `CommitError`.** Confirm
`ConstOverflow` and `ConstUnsupported` are **chip diagnostics** via the
`Fail`/`DiagnosticDraft` path; there is **no** `ConstOverflow`/`ConstUnsupported`
variant in `CommitError`, and the commit does **not** recompute constant values.
The classification is fixed; the exact diagnostic codes remain `/6`.

**Request G2 — `max_const_bits` (rev 21 request, rev 54 supersession).** The
following **rev-21 request text is preserved as historical wording** but is
**explicitly superseded as an unselected draft**: the T08 correction marked the
signed-range/`i128` representable-set formula, the `ConstRecord.value: i128`
carrier, and the chip-level enforcement split as **not selected** (deferred to
`/6` co-freeze), so no part of the rev-21 pin below is normative today.

> **Historical rev-21 request (superseded, unselected draft — retained for
> history, not operative):**
> - **Default:** `max_const_bits = 128`.
> - **Representable set:** `ConstRecord.value: i128` holds the **signed**
>   mathematical representative; the value is representable iff
>   `-(2^(max_const_bits-1)) <= v <= 2^(max_const_bits-1) - 1`. For
>   `max_const_bits = 128` this is exactly the `i128` range
>   `[i128::MIN, i128::MAX]`. An unsigned mathematical value above
>   `2^(max_const_bits-1)-1` (e.g. `2^128-1`) is **not** representable and is
>   `ConstOverflow` (chip diagnostic).
> - **Enforcement:** checked arithmetic is a **chip-level** obligation in
>   `CL03`/`CL05` (`CL02` unexercised); the commit performs no overflow
>   recomputation. The bound is applied **before** the `ConstRecord` is committed.
> - **Hash:** `limits.max_const_bits` and the rule `const.max-const-bits` are
>   **proposed to be hash-pinned** in `M1AppendSchema` (prospective `/6`; not pinned
>   today); the `ConstRecord { ty, value: i128 }` shape and
>   `const.checked-arithmetic-chip-level`/`const.chip-diagnostic` are likewise
>   proposed, not yet pinned.
> - **Tests:** `const_max_const_bits` (boundary `i128::MIN`/`i128::MAX`
>   representable; synthetic `i128::MAX + 1`/`2^128-1` = `ConstOverflow`),
>   `const_checked_arithmetic`, `const_overflow_not_commiterror`. The synthetic
>   overflow is a **chip-diagnostic negative unit vector**, never an M1 acceptance
>   row.

**Request G2 accepted content only (rev 44, 2026-10-05).** The **only** accepted
`max_const_bits` subdecision is that its **origin** is a **hashed `Limits` value**
(`limits.max_const_bits` participates in the frozen-contract hash), with M1
cap/default **128**, `config` **rejects** values **> 128**, and the bound reaching
the zero-field T08 chip through an **explicit task-input projection**. The accepted
bound/cap does **not** claim every valid config value equals 128. **Still OPEN
(`/6` co-freeze, not resolved here):** the exact representable-set **formula**, the
**numeric/wire carrier** (including whether `ConstRecord.value` uses `i128` or
another representation), the **enforcement** split (chip-level vs commit-level and
where the bound is applied), the symbolic-vs-probe gating, and all hash/encoding
and test details. The rev-21 formula/enforcement wording above is **not** operative
and must not be implemented as if selected.

**Request G3 — canonical unsigned/signed domain (historical; superseded).** The
rev-21 request that `ConstRecord.value` is always the **signed** representative
with unsigned-ness only in `ty`, and that an unsigned value exceeding the signed
range is `ConstOverflow` "per G2", **depends on the superseded G2 formula** and is
likewise marked **not selected**. The signed-vs-unsigned canonical-value question,
the overflow classification, and the value carrier remain **open** `/6`
co-freeze items.

**Request G4 — `CL02` chip vs `M1-CL-02` fixture `CL03`; symbolic vs probe (rev
21).** Confirm fixture `M1-CL-02` is executed by chip **`CL03`
(`ConstantBinaryChip`)** and asserts the probe-gated bit pattern `0x00000005`;
chip **`CL02` (`ConstantUnaryChip`) is unexercised by M1**. Confirm `M1-CL-03` is
Part A **symbolic** (no target width) and independent of `M1-CL-02`, with T08
owning `ConstRecord.value` and T09 owning IR `Constant` emission. **Rev 21 also
corrects the malformed fixture chip list: `M1-NEG-16` (`2147483647+1`) is
executed by `CL03` (`ConstantBinaryChip`) and `SE07`, not by chip `CL02`**; the
`CL02` mention is removed.

**Request G5 — no `/7`.** The earlier `/7` nomenclature is withdrawn; this CDR
requests no `/7`. Full fixed-width folding remains Part B / probe-gated.

**Closure criterion:** owner-signed `ConstRecord`/`max_const_bits` semantics
(default/formula/enforcement/hash/test), chip-diagnostic classification, symbolic
Part A path, CL02/CL03/fixture disambiguation (incl. the `M1-NEG-16` fix), and
tests. **Rev 22:** the `max_const_bits` numeric carrier, the `<=128`
config-validation constraint, the restricted-chip config projection, and the
committed `legality` carrier plus the exact `LiteralRecord.candidate_type`
encoding (the assigned `LX08` carrier) are named blockers (`§17.5`). **Rev 44 (user
authority decision, 2026-10-05):** `max_const_bits` origin is a **hashed `Limits`
value** (`limits.max_const_bits` participates in the frozen-contract hash), M1
cap/default **128**, `config` **rejects** values **> 128**, with an **explicit
task-input projection** to the restricted T08 chip (exact wire field/diagnostic
numeric code/hash encoding remain `/6`); the accepted bound/cap does **not** claim
every valid config value equals 128. **Rev 45 (user authority decision,
2026-10-05):** `RequiredKind` M1 enum is **only `IntegerConstantExpression`**, and
the `ConstLegality` result-field values are **`Legal`/`NotConstantExpression`/
`Unsupported`**; future C constant-expression purposes require **appended
variants/new rules**, not a repurposing of the lexical candidate type. The exact
`RequiredKind`/`ConstLegality` numeric encodings, the formula/enforcement split,
and symbolic-vs-probe gating remain **pending** (`§C`). **Rev 54 (docs-only
supersession record, 2026-10-05):** the rev-21 G2/G3 signed-range/`i128`
representable-set formula, `ConstRecord.value: i128` carrier, and chip-level
enforcement wording are recorded as **historical and superseded — not selected**
(the T08 correction marked them unselected); only the rev-44 origin/cap/projection
remains accepted, and the exact formula, carrier, and enforcement stay **open** `/6`
co-freeze items. No new decision, no `/6` freeze, no code.

---

## H. T09 — IR `[OWNER:T09]` `[INT]`

**Request H1 — `TerminatorMissing` trigger direction (rev 21; H11; rev 49 critical
direction).** The rev-20 candidate said "an unterminated entry block at **function
end** is `TerminatorMissing`" without defining "function end". **Rev 49 (user,
2026-10-05; explicit critical H11/T09/T01 decision) selects the operative
direction:** the `TerminatorMissing` trigger **reuses the committed terminal result
of the IR28 `FunctionEnd` task** — the `TaskState::Completed(ResultId)` produced by
the task whose **kind is `FunctionEnd`** — as the **deterministic
function-completion fact**; a **T01-owned typed phase-2b commit-apply validation
hook** then checks the function's **entry block is terminated** (has a terminator as
its greatest `InstructionId`) when that committed result is applied. This direction
**adds no new marker record family/ID/arena/`RecordRef` tag/`RecordFamily`
ordinal/snapshot encoder and no new `ResultValue` variant**. **The earlier
new-marker (`CompletedFunction`) direction is superseded for the operative
direction** (preserved as history). **This selected direction is not a frozen `/6`
hook/schema and authorizes no code:** the **exact hook contract**, its **hash
impact** (likely no new record family, but **rule/hook hashing remains a T01
decision**), the **result typing/commit ordering**, and the **T09/T01 co-freeze**
all remain **open**. `UnsupportedNode`/`UnsupportedIrOp` remain **chip diagnostics**
(missing terminator is a committed-graph invariant, while an unsupported node/op is
a chip language decision).

**Request H2 — `Constant` immediate expected type (rev 21).**
`OpImmediateTypeMismatch` requires the `immediate`'s `ConstId` to have the op's
expected type. **Rev 21 fixes the expected type for `Constant`:** the instruction's
**result `ValueRecord.ty`**; the check is `ConstRecord.ty ==
ValueRecord.ty` (and the `ConstId` exists). There is **no** host width and no
lowering-context inference. Checked pre-mutation in phase 2b.

**Request H3 — complete IR `NORMATIVE_RULES`; every rejection has a rule id.**
Confirm the proposed per-op rules for the `/6` hash `ir.op.constant` (0 operands, immediate some,
result some, non-terminator), `ir.op.add` (2 operands, immediate none, result
some, non-terminator; verifier/future only), `ir.op.return` (1 operand for M1
`main`, immediate none, result none, unique terminator), plus
`ir.block-committed-vs-terminated`, `ir.terminator-greatest-ordered`,
`ir.terminator-missing`, `ir.op-immediate`, `ir.op-immediate-type`,
`ir.unsupported-node`, `ir.folded-int5-constant-emission` (H7; T08 computes the
`ConstRecord` value, T09 emits the IR `Constant`). **Rev 21 requires that
every IR rejection carry a stable rule id**, e.g. `OpArityMismatch` →
`ir.op-arity`, `OpImmediateMismatch` → `ir.op-immediate`,
`OpImmediateTypeMismatch` → `ir.op-immediate-type`, `OpResultMismatch` →
`ir.op-result`, `BlockMissing` → `ir.block-missing`, `BlockTerminated` →
`ir.block-terminated`, `TerminatorNotLast` → `ir.terminator-not-last`,
`ValueProducerMissing`/`ValueProducerDuplicate` → `ir.value-unique-producer`.
Confirm no op/rule is claimed hashed before the `/6` freeze. **H8:** the
rejection→rule-id inventory is **unresolved**, not fixed. Both
`ir.op-immediate-type` aliases (`ir.op-immediate-type` and
`ir.op-immediate-type-result-ty`) are unresolved candidates, and both
`ir.terminator-missing*` candidates (`ir.terminator-missing` and
`ir.terminator-missing-explicit-commit-marker`) are unresolved; the `/6`
inventory must pick one of each.

**Request H4 — counts/tags/arena inventories (incl. `scope_events`).** Reconcile
the IR record counts, the `RecordRef` tags (24/25/26 append-only), and the
`RecordFamily` ordinals (separate inventory) with §3 and §8. **The bus inventory
must list the `scope_events` arena alongside `sem` and `literals`** (the rev-20
`bus.rs` entry omitted it). Confirm the folded `int 5` Part A producer is separate
from the probe-gated `M1-CL-02`.

**Closure criterion:** owner-signed op table + explicit terminator/completion
**fact** rule + immediate-type rule + a rule id for every rejection + IR negative
cases; tests `ir_op_table_hashed`, `ir_block_committed_vs_terminated`,
`ir_terminator_greatest_ordered`, `ir_terminator_missing`,
`ir_value_unique_producer`,
`OpArityMismatch`/`OpImmediateMismatch`/`OpImmediateTypeMismatch`/`OpResultMismatch`,
`folded_int5_ir_constant` (H7 rename; the old `folded_int5_part_a_producer`/`ir.folded-int5-part-a-producer` name implies T09 computes the value and is retired). **Rev 22/23:** the
rejection→rule-id inventory is
**unresolved** — both `ir.op-immediate-type` aliases and both
`ir.terminator-missing*` candidates must be reconciled at `/6` (H8); folded-`int5`
wording is corrected (T08 computes the value, T09 emits IR — **H7**); `Constant`
missing-immediate-vs-result precedence is stated (`§17.6`). **Rev 49 (user,
2026-10-05; critical H11/T09/T01 direction):** the **trigger reuses the committed
IR28 `FunctionEnd` terminal result** (`TaskState::Completed(ResultId)`, task kind
`FunctionEnd`) as the deterministic function-completion fact, checked by a
**T01-owned typed phase-2b commit-apply validation hook**; **no** new marker
record family/ID/arena/tag/ordinal/encoder/`ResultValue`; the earlier new-marker
(`CompletedFunction`) direction is **superseded for the operative direction**
(preserved as history). The **exact hook contract, hash impact** (likely no new
record family; **rule/hook hashing still a T01 decision**), **result
typing/commit ordering**, and **T09/T01 co-freeze** remain **open**; overall **H
stays pending**. All IR rule/op/field
statements here are **prospective `/6`**, not frozen.

---

## I. Cross-document consistency `[INT]` `[DOC]`

**Request I1 — C05 snapshot.** Confirm the snapshot changes from
reserved-ID-only to **typed-body encoding** via per-final-record encoders
(distinct from `RecordDraft::encode`), encodes `Task.progress_ordinal`/
`progress_count`, the canonical bounded `bus.report`, and the pipeline registers +
`PipelineMetrics`. Confirm `sources` byte hashes remain internally recomputed and
that reserved-store bodies not yet frozen remain excluded from the hash but visible
in the snapshot.

**Request I2 — `M1AppendSchema`.** Confirm the single section **proposed to be hashed at `/6`** (not hashed today — rev 25 F1) includes:
(a) appended `(StoreId, field, RecordFamily, backing-arena)` pairs; (b) record
field ids/order after owner sign-off; (c) closed enum inventories; (d) task-kind
codes + stage assignments; (e) chip-id-keyed owner allowlist rows with seed/
signature; (f) limits; (g) per-final-record encodings; (h) all `NORMATIVE_RULES`
ids. The section must be **self-consistent**: encoded family/tag/rule/count values
must equal the document counts, enforced by a test
(`StoreSchema::foundation() ⊕ M1AppendSchema == CompilerBus::try_new(...).schema`).
This **changes the current hash scope** (`hash_excludes=…group-declared-store-fields…`).
**Rev 22:** every `M1AppendSchema` row/inventory must be marked **proposed** vs
**unresolved** distinctly, and no row may be claimed hashed or frozen before the
`/6` freeze; the `§17` ledger lists the unresolved rows. **Rev 50 qualifier
(user, 2026-10-05):** under the accepted **two-tier model** this section is the
**frozen `/6` seed** (`StoreSchema::foundation + M1AppendSchema`) that
**participates in the `/6` contract hash**, while **post-seed runtime
`declare()` extensions stay hash-excluded** and are captured/validated via
runtime snapshot/schema mechanisms; the exact contents/counts, the
`hash_excludes` re-description, and the seed-equality/freeze test remain **T01
`/6` co-freeze after the owners** (no `/6` values/hash yet).

**Request I3 — exact error families (rev 21; selection errors reclassified).**
Confirm the classification inventory:
- `CommitError` (commit-path reject-before-apply): the §6.4 list including
  `BackpressureCapacity` and `CrossTaskWriteConflict`.
- **chip diagnostic** (`Fail`/`DiagnosticDraft`): `ConstOverflow`,
  `ConstUnsupported`, `ParseCursorDidNotAdvance`, `ParseDepthExceeded`,
  `UnsupportedIrOp`, `UnsupportedNode`, `RedeclarationConflict`, and stage
  language-rule violations.
- `ManifestError` (registration-time): `StoreOwnerViolation` (keyed by `ChipId`),
  `StageUnassigned`, `StageLayerMismatch`.
- **dispatcher/scheduling failure** (pre-dispatch, before any store mutation; not
  a `CommitError`): `SelectionBatchOverflow`, `DuplicateSelection`.
  **Rev 37 (H9):** `InflightQuotaExceeded` is **removed
  from the candidate** with `max_inflight_total` (historical rev-21/22 name only;
  pending T01 integrator acceptance). **Rev 54:** with the rev-51 delegated
  default, `DispatchBudgetExceeded` and its `max_dispatches_per_tick` bound are
  likewise **dropped from the candidate** (historical rev-21/22 name only).
- `ConfigError`: `MaxTicksOverflow`, invalid quota/queue/fairness.
`SymbolConflict` is commit-path; `NoSuchSymbol` is a chip lookup result (miss),
not an error family. Confirm numeric codes are fixed at `/6` and that no new
family is invented beyond the dispatcher/scheduling classification above.

**Request I4 — T01 §4 supersession inventory only (do not edit `/5`).** Record
in the `/6` inventory (not in `/5`): the T01 §4 wording reconciliation
("the control chip **computes** the selection; the integration/dispatcher applies
it"; "the **commit path/backend** writes persistent queues/results"), the
symbolic-vs-concrete-width wording, and — **accepted in principle by the user
(2026-10-04, decision B)** — the awaited-child clarification that
`TaskState::Waiting(WaitSet)` is the sole awaited-child-ID source and
`ContinuationRecord.awaited` is removed. **No edit to `/5`/T01 §5 is made or
authorized by this CDR;** the T01 §4 change is a formal `/6` amendment only, with
**T01 integrator and T05 owner acceptance pending**. **Rev 54 completeness note
(read-only audit): the §I4 catalog above is incomplete and one entry is
mis-attributed.** It omits: the extended ordering rule
`(stage ordinal, phase priority, enqueue ordinal, TaskId)` and commit order
`(dispatch_ordinal, enqueue ordinal, TaskId, proposal index)`; the five-outcome
per-dispatch set `{Complete, Fail, AwaitHost, AwaitChildren, Progress}` (plus the
failed-batch regime) replacing the three-outcome set at T01 §7.1 C03; the
`Proposal` additions `AppendRecords`/`Progress`/`AwaitChildren` (and
`ResultValue::DraftRecords`); the `continuation: Option<ContinuationRef>` typing and
the exact continuation fields; the `Progress`/queue-membership rule (`Progress` →
`Ready` + one own-stage entry; `Waiting` in no stage queue); the latch/in-flight
clear allowance (which contradicts ADR-0002 §2.4 step 1's tick-start clear); and
the missing `Fault` representation. It **mis-attributes** the
symbolic-vs-concrete-width wording to T01 §4 when the operative text is **T01 §3,
line 59**. These additions are recorded here as **open catalog corrections**
(consolidated as OB-18 in the M1 proposal §24.11), not as accepted amendments; the
`/6` freeze must reconcile T01 §4, T01 §7.1, and ADR-0002 §2.4 together.

**Request I5 — documentation reconciliation.** Resolve the direct contradiction
between [COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md)
§4 ("adding fields or rules changes the frozen contract hash") and
[contract.rs](../../compiler/src/contract.rs) /
[compiler/README.md](../../compiler/README.md) (group-declared fields are
hash-excluded), consistent with the adopted `M1AppendSchema` decision. **Rev 48
read-only audit:** this is a **real unresolved conflict, not merely prose** — the
`/5` artifacts carry `hash_excludes=…group-declared-store-fields…` while the
manifest asserts adding fields/rules changes the hash, and the `M1AppendSchema`
is **proposed to be hashed at `/6`**. The **three hash-scope options remain
unselected and unrecommended** (no authority): (a) **hash `M1AppendSchema`**
(requires updating the exclusion token, a freeze assertion, and the manifest
wording); (b) **do not hash** it (requires correcting the manifest wording); or
(c) an **explicit two-tier split** between a hash seed and runtime declarations.
The `/6` must also specify **dual-inventory encoding** (wire tags vs ordinals),
**numeric-value inclusion in the hash**, and the **schema self-consistency
mechanism**; actual counts/values are frozen only after the owner shapes are
accepted. **Rev 50 qualifier (user, 2026-10-05):** the user accepted the
**two-tier split** (option (c)) at the **conceptual** level, so the rev-48
"three options remain unselected and unrecommended" wording above is
**historical**: `foundation + M1AppendSchema` is the **frozen `/6` seed
hashed**, and **post-seed runtime `declare()` declarations are hash-excluded**
and captured/validated via runtime snapshot/schema mechanisms. The
contradiction is therefore resolved in **concept** only; the manifest §4
wording, the `hash_excludes` semantic description/token (**post-seed runtime
declarations**, not the M1 seed), and `FrozenSchema::encode`/contract code plus
the freeze test must still be updated **atomically** at `/6`, with the numeric
inventory and self-consistency test pending and `/5` preserved.

**Closure criterion:** the counts reconcile, the seed-equality test is specified,
the hash-scope change is accepted, the error inventory is fixed, and the
doc-only `/6` items are listed without touching `/5`.

---

## 4. Ownership conflict analysis (explicit)

| Conflict | Accepted rule | Candidate | CDR position |
|---|---|---|---|
| `sources.spans` T03+T04 (+T05) shared writer | Guardrails §2.7: one owner per store/field | rev-20 §15.1 "C6" shared selection | **User selected the single owner in principle (2026-10-04): T03 sole `sources.spans`; T04/Token reuse committed T03 PP spans; AST `Node` = first/last `TokenId` range, no T05 span write. Guardrail unamended; S3 rejected.** |
| `constants.records` T04+T08 (rev 18) → single T08 | one owner per store | rev-20 committed `LiteralRecord` handoff | **User accepted the handoff direction and the H1 allocation split in principle (2026-10-04)** — `LiteralRecord` = per-literal lexical facts + `LX08` type; sem-stage `ConstantRequest` = `node`/`required_kind`; `ConstantResult` = `legality`; T08 sole writer. Exact variants still T04/T08 co-freeze; **not a freeze**, exact schema and T01/owner sign-off pending. |
| `types.records` TY13+TY17 | one owner per store/field | chip-id-keyed allowlist, single producer per row | **Accept direction**, pending E4 allowlist rows. |
| `names.entries`/`lex.literals` | one owner per field | T04-only | **Accept direction**, pending C2. |
| `parse.nodes` / `sources.spans` from T05 | one owner | T05 `parse.nodes` sole; no span write | **Resolved in principle: `parse.nodes` T05-only, token-range nodes, no span write** (§B). |
| Cross-task conflict predicate | must not justify shared ownership | rev-20 predicate | **Accept only for same-owner-group appends and same-record patches**, not to bless a carveout; the VF04 batch-conflict audit is a proposed/residual extension. |

**No other multi-owner carveout is proposed or accepted by this CDR.**

---

## 5. Determinism / replay and snapshot / hash impact

1. **Scheduler registers change snapshots.** Adding stage queues, in-flight set,
   `dispatch_cursor`, `stage_assignment_version`, and `PipelineMetrics` makes
   byte-identical quota-1 snapshots impossible. Quota-1 equivalence is therefore
   the **canonical semantic comparison projection** (§A4), with the scheduler
   snapshot separately deterministic and replay-identical run-to-run.
2. **`M1AppendSchema` changes hash scope.** The current contract
   [hash-excludes group-declared store fields](../../compiler/contracts/CONTRACT_VERSION).
   The candidate hashes the M1 append schema. This is a normative change and a
   `/6` item; it must be reconciled with
   [COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md)
   §4 ([I5](#i-cross-document-consistency-int-doc)). **Rev 50 qualifier (user,
   2026-10-05):** the conceptual scope is settled as the **two-tier model** —
   `foundation + M1AppendSchema` is the **frozen `/6` seed participating in the
   hash**, while **post-seed runtime `declare()` extensions stay excluded**; the
   manifest §4 wording, the `hash_excludes` token re-description, and
   `FrozenSchema::encode`/freeze-test updates remain **T01 `/6` integration
   work**, with the exact contents/counts pending.
3. **Append-only wire tags.** New `RecordRef` tags 24/25/26 preserve tags 0–23;
   `RecordFamily` ordinals are a separate inventory. Both are **proposed to be
   pinned at `/6`** (not pinned today — rev 25 F1) and self-consistency tested.
4. **Stable IDs only.** No hash-map iteration, address, or wall-clock input may
   affect dispatch, id allocation, diagnostics, or serialization (ADR-0001 §2.6,
   guardrails §2.8/§4.1).
5. **Replay evidence required:** identical input/config/initial state ⇒ identical
   IDs, name-intern order, body bytes, dispatch order, commit order, and snapshot
   hash across ≥3 runs; `M1-REC-03`, `D8`, `G15` are the named gates.
6. **No throughput claim.** Quota `> 1` additionally requires measured before/after
   tick counts on the frozen corpus and separate integrator acceptance.

---

## 6. Chip-local alternative and why it is insufficient

A group owner can implement its chips against a **private** local draft/arena and
avoid the shared envelope. This is **insufficient** because:

- `RestrictedChip` has no bus access; a record body can only be materialized by the
  shared `AppendRecords`/commit path. A private path would bypass the single commit
  and the `/5` rule that a chip may only propose
  ([T01 §4](T01_COMPILER_CONTRACT.md), [guardrails §2.1/§2.3](COMPILER_DEVELOPMENT_GUARDRAILS.md)).
- Cross-stage data flow is only through committed typed records and task
  requests/results; a private type/ID/arena would be an incompatible second
  contract ([PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) §2).
- The deterministic snapshot/hash and replay gate require one canonical shared
  schema; per-chip private stores cannot be audited or hashed consistently.
- Ownership/allowlist enforcement is a registration-time shared mechanism.

Therefore the change **must** be an integrator-accepted shared-interface change.

---

## 7. Affected chips, task kinds, manifests, and tests

| Area | Chips / kinds affected | Manifests to update | Tests to (re)run |
|---|---|---|---|
| Pipeline | CT03–CT14; all routed workers via `dispatch_cursor`; VF02/VF03/VF04/VF13 | every chip manifest gains `stage` | quota-1 projection, multi-inflight determinism, cross-task conflict, backpressure, replay |
| T03 | PP01–PP04, PP28; artifact map | `sources.spans` (owner TBD), `sources.expansions`, `pp.tokens`, `artifacts.fragments` | `M1-PP-*`, artifact map tests |
| T04 | LX01–LX08, LX16, LX17 | `lex.tokens`, `lex.literals`, `names.entries` | `M1-LX-*`, `literal_record_committed`, `literal_t04_t08_handoff` |
| T05 | PA01–PA09, PA16, PA20, PA22, PA24, PA28, PA32, PA34, PA38 | `parse.nodes`, `tasks.continuations`, span field (TBD) | `M1-PA-*`, continuation/join tests |
| T06 | TY01–TY03, TY07, TY09, TY10, TY13, TY17, TY20, TY25–TY27 | `symbols.*`, `types.records` | `M1-TY-*`, scope/symbol tests |
| T07 | SE02, SE07, SE21, SE29 (SE26/SE01 catalog/negative) | `sem.records` | `M1-SE-*`, conversion/sem tests |
| T08 | CL01, CL03, CL05 (CL02 unexercised) | `constants.records` | `M1-CL-01/03`, const tests, probe-gated `M1-CL-02` |
| T09 | IR01–IR03, IR19, IR28 | `ir.*` | IR invariants, op table, folded-5 |
| T13 | VF01–VF06, VF12–VF14 | verification manifests | VF02/VF03/VF04/VF13 batch extensions |

Task-kind codes (`group<<12 | local`, local `>= 16`) and stage assignments are
`RESIDUAL` and must be registered in `TaskKindRegistry`/`RoutingTable` at `/6`.

---

## 8. Compatibility, migration, and freeze plan

1. **No in-place mutation of `/5`.** All changes are a new `/6` envelope; `/5`
   stays current until acceptance. No existing file is edited by this CDR.
2. **Single `/6` freeze (recommended).** Freeze all Part A families, the append
   schema, task codes, rules, limits, and pipeline registers in one version
   (`schema.m1-single-freeze`); the alternative (envelope `/6` + family increments)
   is a residual to decide.
3. **Migration steps (after acceptance only):** update `CONTRACT_VERSION`/hash;
   add explicit structs/enums per §3; seed `StoreSchema` from `M1AppendSchema`;
   migrate external tests to `try_new`; re-run dependent packages
   ([PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) §5).
4. **Inertness.** No chip wave F1–F6 may implement or depend on the pipeline
   registers before the freeze; quota `> 1` is post-freeze and measured.
5. **Rollback.** If a decision is rejected, the affected candidate stays draft and
   `/5` is unaffected; no partial `/6` is published.
6. **`/6` inventory item.** This CDR is the inventory entry; it extends (does not
   replace) the [ADR-0002 §3](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md)
   and [M1 §12.18](M1_PART_A_CONTRACT_PROPOSAL.md) lists, which must be reconciled.

---

## 9. Unresolved items (do not pretend there is enough information to freeze)

*The following remain open. Items resolved **in principle** by the user on
2026-10-04 are marked, but their **exact fields/encodings** remain owner blockers;
"resolved in principle" is not "frozen".*

- **H1 (accepted in principle; co-freeze/sign-off pending) — literal-handoff
  allocation:** which of `node`/`required_kind`/`legality`/`LX08` live on the
  per-literal committed `LiteralRecord`, on the sem-stage per-use
  `ConstantRequest`, and on the `ConstantResult`. On **2026-10-04 the user
  accepted the recommended split in principle**: `LiteralRecord` carries the
  per-literal lexical facts + `LX08` candidate type; the sem-stage
  `ConstantRequest` carries `node`/`required_kind`; the `ConstantResult` carries
  `legality`. This is a **revision working basis, not a freeze**; the exact
  variants/schema remain **T01 integrator acceptance + T03/T04/T08 owner
  co-freeze/sign-off** items.
- **H6 (selected scope + mechanism; `/6` freeze pending, not implemented) —
  no-`Running`/terminal diagnostic mechanism:** clearing the in-flight set does not
  by itself clear a task's `Running` state, and **clearing the in-flight set is not
  itself a transition**. Only the invariant "no dispatched task remains `Running`
  at latch" is stated. The frozen `/5` `routing.rs` `fail_selected` **single-task**
  behavior is **verified and preserved** — the failed task **always** becomes
  `TaskState::Failed`, attaching a committed `DiagnosticId` when capacity allows or
  the `TaskState::Failed(DiagnosticId::NONE)` sentinel otherwise (never stranded).
  **Rev 34 (user direction in principle, 2026-10-04):** the user **selected the
  batch-failure recovery direction** — every dispatched task, in **dispatch
  order**, transitions **exactly once** to `Failed`, a committed `DiagnosticId`
  when diagnostic/record capacity allows (else the `DiagnosticId::NONE` sentinel),
  so **every** dispatched task leaves `Running`; this **generalizes** the verified
  `/5` single-task transition. **Rev 42 (user authority decision, 2026-10-05;
  H6 scope + mechanism selected):** the user **rejected** the §9B row-A deferral
  to a later CDR and selected an explicit **quota>1-capable bounded batch recovery
  mechanism to freeze in `/6`** — **pre-dispatch errors before any state mutation
  leave the affected tasks `Ready`**; a **semantic batch commit failure commits no
  semantic state**; a **deterministic bounded recovery mutation then processes the
  dispatched tasks once in dispatch order to `Failed`**; **no pre-reservation of N
  diagnostics**; a **per-task diagnostic attempt** with `DiagnosticId::NONE` when
  capacity is insufficient; and a **state guard prevents the duplicate
  transition**. **The mechanism is selected; it is not implemented and no `/6`
  freeze has occurred.** The **exact atomic-commit realization / T01 integrator
  approval, the H9 numeric inventories, and the T02/T13 owner fixtures/sign-offs
  remain co-freeze and pending**. The withdrawn hypothesis that clearing the
  in-flight set alone clears `Running` remains **withdrawn as unproven**.
- **H9 (removal direction accepted in principle; remaining `[INT]` items open) —
  in-flight scheduling ownership:** the single phase/owner for
  `max_inflight_total`, the residual-set semantics of the in-flight set, and the
  relationship between the dispatcher's `Ready → Running` mutation and the ordered
  atomic commit were **undefined `/6` decisions**, not resolved. **Rev 35 (user
  direction in principle, 2026-10-04):** the user accepts **removing
  `max_inflight_total`** from the `/6` candidate — sequential per-tick dispatch is
  already bounded by `max_inflight_per_tick` (the dispatch batch bound), the
  per-stage `stage_queue_bound`, and `max_tasks_total`; `TaskState::Waiting` is not
  in-flight; `tasks.in_flight` is an **ephemeral per-tick scheduler batch only**
  cleared at latch only after every dispatched task has a terminal/`Waiting`/
  `Progress` outcome (or the H6 recovery), so no residual in-flight set survives and
  a separate total bound is unnecessary. A future cross-tick `Running` mode would
  need a **separate CDR**. The dispatcher's pre-worker `Ready → Running` mutation is
  distinguished from the one ordered atomic semantic commit. **This is a
  working-basis direction only, pending T01 integrator acceptance**; no
  bound/config/hash is frozen, and the remaining `[INT]` items (the
  dispatcher-mutation-vs-ordered-atomic-commit relationship and the residual-set
  semantics) stay open. The single phase/owner for `max_inflight_total` is no
  longer asserted; the per-tick `max_inflight_per_tick` bound is the only dispatch
  bound in the candidate. **Rev 49 (read-only H9 audit; unresolved blockers; the H9
  removal direction and the H6 mechanism are unchanged, no freeze/code):** the `/5`
  code proves **only** the **quota=1 single-task** `fail_selected` transition, so
  the proposed **quota>1 pipeline is not implemented**; H9 still needs (a) the
  **exact relationship** of the dispatcher pre-worker `Ready→Running`/`in_flight`
  population to the **single ordered atomic commit**, (b) the **exact `in_flight`
  clear ownership/order** relative to the **bounded H6 recovery**, and (c) a **proof
  that no `Running`/residual set remains at latch for all success/error/
  empty-proposal paths**. An **internal proposed-limit conflict** was recorded: the
  candidate listed/checked `max_dispatches_per_tick` **separately** from
  `max_inflight_per_tick` even though the latter is described as the **sole**
  dispatch bound. **Rev 54 (docs-only):** this rev-49 conflict finding is
  **historical and superseded** by the **rev-51** integration-selected candidate
  default (see below), which **drops** the redundant `max_dispatches_per_tick` and
  keeps `max_inflight_per_tick`/quota as the sole per-tick dispatch-count bound; the
  rev-49 wording is **retained as history only** and is **no longer an open
  candidate conflict**, while the underlying **dispatcher `Ready→Running`/`in_flight`
  commit-boundary and clear-order items remain unresolved**. The
  **H6/H9 fan-out fixtures** and **T13 VF02/VF03/VF04/VF13** remain **pending**, and
  the H9 no-residual guarantee is **conditional on the H6 recovery implementation**;
  the **quota=1 M1 baseline** is preserved and **separated** from the quota>1
  optimization. **No freeze/code**; the exact stages/version carrier/errors/hashes
  remain pending. **Rev 51 (integration-selected candidate default under user
  delegation, 2026-10-05; not an owner/T01 `[INT]` signoff):** the duplicate
  `max_dispatches_per_tick` is **dropped** from the candidate limit inventory and
  validation — **`max_inflight_per_tick`/quota is the sole per-tick dispatch-count
  bound** — grounded in the rev-35/38 H9 direction and the rev-49 audit duplication
  finding; the exact names/defaults/codes remain **T01/T02 `/6` co-freeze** and the
  T02/T13 tests remain **pending**. This changes neither the H6 mechanism nor the H9
  removal direction and resolves **no** dispatcher-`Ready→Running`/`in_flight`
  atomic-boundary or clear-order item.
- **Pipeline:** exact stage set/names; fairness policy + persistent cursor;
  backpressure defer/reject and per-stage vs global; cancel re-check; metrics
  inner encoding; quota-`>1` measurement plan; `SelectionEntry` final shape;
  parking-vs-single-transition. The *direction* (deterministic bounded sequential
  pipeline, quota-1 semantic projection baseline) is accepted in principle; none
  of these concrete choices is.
- **Ownership:** resolved **in principle** (T03 sole `sources.spans`; T04/Token
  reuse committed spans; AST token range; guardrail unamended). Remaining: T03/T04/
  T05 exact manifest fields and the T04 committed-span reuse protocol.
- **T03/T04/T08:** `raw_offsets` final shape;
  `LiteralRecord`/`LiteralSuffix`/`LiteralKind` variants; the `ConstantRequest`
  shape that preserves `node`/`required_kind`/`legality`/`LX08` type. (`Preprocessed`
  M1 status, the handoff direction, and the H1 carrier split are accepted in
  principle; the exact enum variants are not.) **Rev 45 (user subdecisions,
  2026-10-05; no freeze, still PENDING overall):** the exact ordered
  `LiteralRecord` fields (`token`, `kind`, `radix`, `suffix`, `value` big-endian,
  `negative`, `spelling`, `candidate_type`; no `node`/`required_kind`), the M1
  `LiteralKind {Integer, Character, String}` (only `Integer` produced) /
  `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced) / radix
  `{2,8,10,16}` (M1 decimal only) scope, the symbolic `Lx08CandidateType` (M1
  literals 2 and 3 = `Int`, no bit width), `RequiredKind` M1 enum only
  `IntegerConstantExpression`, and the `ConstLegality` values `Legal` /
  `NotConstantExpression` / `Unsupported` are **selected**. **Still open:** the
  complete `Lx08CandidateType` member set/numeric encodings (not invented here),
  the optional-kind artifact-map rule, source-versus-payload equality, and the
  exact artifact error classification/numeric codes. **Rev 53 (delegated candidate
  default, 2026-10-05; integration-agent selected under explicit user delegation, not
  an owner/T04 or T01 `[INT]` signoff, not a freeze):** for the **exercised M1 literal
  subset** the `Lx08CandidateType` M1 vocabulary is the **closed one-member set
  `{ Int }`** (M1 literals 2/3 = target-independent `Int`, no bit width); forms
  **outside** the exercised M1 subset must **not silently default to `Int`** and are
  **explicitly unsupported/deferred**; future categories **append without
  reinterpretation**; **not** the complete future C candidate vocabulary, no numeric
  tags; the complete `Lx08CandidateType` member set/encodings remain open.
- **T05:** exact `ContinuationRecord` field order/wire encoding; the two constant
  flows; `NodeKind` final list; the exact `ContinuationRef`/`ChildRef` validation
  error names; the parse task-contract delta. (`awaited` removal/`WaitSet`-only
  representation is **accepted in principle** — user decision B, 2026-10-04 — with
  the T01 §4 `/6` supersession (no `/5` edit) and **T01/T05 acceptance pending**;
  token-range nodes and commit-apply join reinsertion are selected; the fields are
  not.) **Rev 42 (user authority decision, 2026-10-05):** the join as a
  **commit-apply invariant with no new CT07 committed carrier/family** and
  **await-all children terminal before the parent decision** are **accepted
  subdecisions**; the continuation fields/wire encoding, the `WaitSet` `/6` formal
  T01 §4 supersession, the `OwnBatch` exact preapply errors, and the parse
  contract/tests remain **pending**. **Rev 43 (user authority decision, 2026-10-05):**
  the `ContinuationRecord` **exact ordered fields/shapes** (`production: TaskKind`,
  `cursor: TokenId`, `context: ParseContext`, `binding_power: u16`,
  `scope: Option<ScopeId>`, `parent: Option<NodeId>`, `partial_children: Vec<NodeId>`,
  `next_child_ordinal: u32`, `previous: Option<ContinuationId>`; no `awaited`; durable
  committed-ID references only), the **formal `/6` T01 §4 supersession** (`WaitSet`
  sole awaited-child source; no `/5` edit), and the **exact OwnBatch pre-apply
  validation** (phase-1 continuation ref within the same task `AppendRecords`
  range/family `Continuation`; phase-2b child ref within the same task Enqueue list;
  whole-batch `CommitError` before any mutation) are **accepted subdecisions**; the
  `RecordRef` numeric wire-tag values and `RecordFamily` ordinals (separate
  inventories), the `ParseContext` vocabulary final encoding, the OwnBatch numerical
  error codes/hash, and the parse request/result/test shapes remain **pending**.
- **T06:** lifecycle field shapes; `char` representation; enum variants; allowlist
  rows/seed. (The `Identifier`-leaf `decl` node and the Block-node boundary `at`
  are selected; the field encodings are not.) **Rev 42 (user authority decision,
  2026-10-05):** the canonical `TypeId` reuse mechanism — a **deterministic
  bounded scan of the committed `types.records` returning the lowest matching
  `TypeId`, no hidden cache/index** — is an **accepted subdecision**; the bootstrap
  scheduling, the T05 upstream committed-`TranslationUnit` contract, and the field
  encodings remain **pending**. **Rev 43 (user authority decision, 2026-10-05):**
  the File Enter trigger as a deterministic
  `parse.TranslationUnit -> symbol_type.scope-enter` stage edge **after** the
  committed TU, carrying the committed `NodeId`, with **no job-bootstrap**, is an
  **accepted subdecision**; the bootstrap wiring beyond the stage edge and the T05
  committed-TU carrier remain **pending** (the accepted stage-edge direction is not
  a TU schema signoff).
- **T07:** the exact committed semantic carrier shape and whether a
  `FunctionContextId` exists; the `ConversionOp`/`ConversionRole` permitted set;
  the exact `(NodeKind, role, op)` VF06 matrix; the exact constant-request kind
  names/vocabulary. (`Return`/`FunctionDefinition` SemRecord presence and the
  no-T09-`FunctionRecord` rule are selected; the shapes are not.) **Rev 42 (user
  authority decision, 2026-10-05):** **no `FunctionContextId`** and `SemRecord` =
  committed materialization of `CheckedNode`, one per `NodeId`, with an explicit
  committed typed link consumed by T09 are **accepted subdecisions**; the
  conversion matrix, VF06/T13, effects/value category, and diagnostics/tests
  remain **pending**. **Rev 46 (user authority decision, 2026-10-05):**
  `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` (M1
  fixture = `NonLvalue`) and the M1-only `EffectMask(0)` rule (nonzero = typed
  unsupported/diagnostic; effect bit classes reserved/unassigned); VF06
  `TypedAstInvariant` **is in M1** (after committed T07 `SemRecord`s, before T09
  lowering, M1 typed-fact/required-conversion completeness); and the **M1-minimal
  conversion scope** (freeze only M1-fixture-needed behavior incl.
  identity/no-conversion; integer promotions, float conversions incl.
  `FloatToFloat`, pointer qualifier, and other non-M1 conversions **explicitly
  unsupported/deferred** to a later append/contract revision; no
  `ConversionOp`/`ConversionRole` closed list, pairing, role→chip mapping, or
  numeric encoding invented) are **accepted subdecisions**. **Still open:** the
  **exact VF06 registration** (task kind/stage/phase/interface), the full future
  conversion matrix, and all conversion/effect numeric encodings.
- **T08/T09:** the exact diagnostic codes; the **function-completion fact/trigger**
  shape; the `IrOp` per-rejection rule ids; exact record fields. (`max_const_bits =
  128` origin/cap and the `Constant` immediate = result-type rule and
  the `M1-NEG-16` chip fix are selected; **the rev-21 signed-range/`i128` formula is
  historical/superseded — not selected (rev 54)**, and the encodings are not.) **Rev 49 (user,
  2026-10-05; critical H11/T09/T01 direction):** the `TerminatorMissing` trigger
  **reuses the committed terminal result of the IR28 `FunctionEnd` task**
  (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the **deterministic
  function-completion fact**, checked by a **T01-owned typed phase-2b commit-apply
  validation hook** (entry-block termination); **no** new marker record family/ID/
  arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder and **no** new
  `ResultValue` variant. The earlier **new-marker (`CompletedFunction`) direction is
  superseded for the operative direction** (preserved as history). **Not a frozen
  `/6` hook/schema, no code authorization:** the exact hook contract, its **hash
  impact** (likely no new record family, but **rule/hook hashing remains a T01
  decision**), **result typing/commit ordering**, and the **T09/T01 co-freeze** stay
  **open**; overall **H stays pending**.
- **Cross-cutting:** exact `NORMATIVE_RULES` set; error-code numbers; counts/tags/
  arena reconciliation; `M1AppendSchema` self-consistency test; docs §I5. The
  **bus inventory must list `scope_events`** alongside `sem`/`literals`.
  **Rev 48 read-only audit:** the checked-in `/5` code fact is **24**
  `RECORD_KINDS` / **24** `RecordRef` variants (tags 0–23) with **no
  `RecordFamily`** type; the CDR's 19/27/27 draft counts are **proposed/unverified**
  and the future `/6` count is **not** claimed to equal 24. The **hash-scope
  conflict is real and unresolved** (see §I5); the three hash-scope options remain
  **unselected/unrecommended**; `/6` must specify **dual-inventory encoding**
  (wire tags vs ordinals — the rev-43 accepted structural subdecision),
  **numeric-value inclusion in the hash**, and a **schema self-consistency
  mechanism**, with actual counts/values frozen only after owner-shape
  acceptance. No owner acceptance, no signature, no freeze. **Rev 50 (`[USER]`,
  2026-10-05, critical hash-scope decision):** the user **accepts the two-tier
  model** — `StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed
  and participates in the `/6` contract hash**, while **post-seed runtime
  `StoreSchema::declare()` extensions remain excluded** from the frozen hash and
  are captured/validated through **runtime snapshot/schema mechanisms**. This
  **resolves the conceptual `/6` hash boundary**, but it **accepts no exact
  `M1AppendSchema` contents/counts and freezes nothing**; at `/6` integration T01
  must **atomically** update the `COMPILER_SFL_MANIFEST.md` §4 wording, the
  `hash_excludes` semantic description/token (scoped to **post-seed runtime
  declarations**, not the frozen M1 seed), and `FrozenSchema::encode`/contract
  code plus the freeze test, **preserving the frozen `/5` hash and history**. The
  numeric inventory and the self-consistency/freeze test implementation remain
  **pending**, and **T01 still co-freezes the M1 seed values after the owners**;
  **row I and the overall CDR stay PENDING**, `/5` current, no code. **Rev 49 read-only H9
  audit (unresolved blockers; no option selected; no freeze):** the `/5` code proves
  only the **quota=1 single-task** transition, so the quota>1 pipeline is
  unimplemented; the dispatcher `Ready→Running`/`in_flight`-vs-ordered-atomic-commit
  relationship, the `in_flight` clear ownership/order relative to the H6 recovery,
  and the no-`Running`/residual-set proof for all paths stay **open**; the
  `max_dispatches_per_tick`-vs-`max_inflight_per_tick` proposed-limit conflict is
  **deferred to `/6`** (not decided); the H6/H9 fan-out fixtures and T13
  VF02/VF03/VF04/VF13 stay **pending**. No owner acceptance, no signature, no freeze.
  **Rev 51 (integration-selected candidate default under user delegation,
  2026-10-05; not an owner/T01 `[INT]` signoff):** the duplicate
  `max_dispatches_per_tick` limit is **dropped** from the candidate limit inventory
  and validation, leaving **`max_inflight_per_tick`/quota as the sole per-tick
  dispatch-count bound**; exact names/defaults/codes remain **T01/T02 `/6`
  co-freeze** and the T02/T13 tests remain **pending**. No freeze/code; rows C–I
  stay **PENDING**.
- **Any Part A record carrying a target width/bit-pattern becomes probe-gated**
  (default symbolic).
- **Post-rev-39 OB register (cross-reference only; not restated here).** The
  still-open blockers surfaced by the post-rev-39 read-only audits are
  consolidated as **OB-1..OB-53** in the
  [M1 Part A proposal](M1_PART_A_CONTRACT_PROPOSAL.md) §24.11 open-blocker summary
  table. That register is the **proposal-side index** for those blockers,
  including the Host/bootstrap items (OB-35/OB-36/OB-39–OB-41), the
  `M1AppendSchema` wire-addition omissions (OB-42/OB-43), the `PipelineMetrics`
  field encoding (OB-44), the §9 task-kind-code inventory gaps (OB-45–OB-47),
  `StoreOwnerViolation` (OB-48), the reserved-ID relocation protocol (OB-49), the
  exactly-once status conflict (OB-50), the conversion-scope conflict (OB-51),
  and the pointer-staleness / OPEN-01 entries (OB-52/OB-53). Every row is
  **`OPEN`**; §9C remains the authority for signatures. This cross-reference adds
  no decision, acceptance, or freeze.

**Rev 22 additional unresolved items (from the `§17` ledger; all are `/6`/owner
blockers, none frozen):**

- **T03/T04:** the typed T04 provenance carrier for `LX14`/`LX16`
  (merged/synthesized token spans); the typed multi-source map for include
  expansion / multi-source `Preprocessed`; the Part B owner of
  `Assembly`/`Object`/`Snapshot`/`Trace`.
- **T05:** the T01 §4 `awaited` supersession is **accepted in principle** (user
  decision B, 2026-10-04: `WaitSet`-only; formal `/6` T01 §4 amendment, no `/5`
  edit; **T01/T05 acceptance pending**); the CT07 committed
  join-decision carrier (**now: no new CT07 carrier/family — join is a
  commit-apply invariant, rev 42**); the exact child-failure/sibling-cancellation policy
  (all-`Completed`→`Ready` / any-`Failed`→`Failed` once is only the selected
  direction, H4; **await-all-children-terminal is the accepted subdecision,
  rev 42**); the verifiable T04 PP-span provenance; the exact
  terminal-capacity no-`Running` form (**rev 42: the user selected the quota>1-capable
  bounded recovery mechanism to freeze in `/6` — pre-dispatch errors leave tasks
  `Ready`, batch commit failure commits no semantic state, the deterministic
  bounded recovery mutation transitions dispatched tasks once in dispatch order,
  no pre-reservation, per-task `DiagnosticId::NONE` attempt, state guard; the
  exact atomic realization remains pending**); the parse-depth
  counting/order predicate; the resume stage `stage_of(parent.kind)` with
  absent-`continuation` handling (H2); committed-only node token links (H5).
- **T06:** the canonical `TypeId` reuse lookup mechanism; the namespace-carrier
  decision.
- **T07:** the T09 `SemRecord` request/link and `CheckedNode`↔`SemRecord` family
  decision; the `FunctionContextId` choice (T07 package still requires one);
  the `ConversionOp` FP/pointer-qualifier domain and the `(op,role)` permitted
  set plus the role→T09-chip mapping.
- **T08:** the `max_const_bits` numeric carrier, the `<=128` config-validation
  constraint, and the restricted-chip config projection; the committed
  `legality` carrier and the exact encoding of the `LiteralRecord.candidate_type`
  (the assigned `LX08` carrier — the carrier is **not** in doubt; only its
  enum/type encoding is pending). The accepted-in-principle H1
  split places `LX08` on the `LiteralRecord` and `legality` on the result; the
  exact committed carrier shapes remain `/6` co-freeze.
- **T09:** the function-completion fact/trigger shape (**rev 49: the user selected
  reuse of the committed IR28 `FunctionEnd` terminal result** — `TaskState::Completed(ResultId)`,
  task kind `FunctionEnd` — checked by a **T01-owned typed phase-2b commit-apply
  validation hook**; **no** new marker family/ID/arena/tag/ordinal/encoder/
  `ResultValue`; the earlier new-marker (`CompletedFunction`) direction is
  **superseded for the operative direction** and preserved as history; the exact
  hook contract, hash impact (rule/hook hashing still a T01 decision), result
  typing/commit ordering, and T09/T01 co-freeze remain **open** — H11); the
  rejection→rule-id inventory reconciliation —
  **both** `ir.op-immediate-type` aliases and **both** `ir.terminator-missing*`
  candidates are unresolved (H8); the `Constant` missing-immediate-vs-result
  precedence. All IR rule/op/field statements are **prospective `/6`**.
- **Pipeline:** the own-task-batch draft-key vs Guardrails §6.1 **narrow
  interpretation is accepted in principle** (user decision A, 2026-10-04:
  transient wire/proposal input only; resolved to committed IDs before persistent
  state; guardrail unamended) — only the **T01 integrator
  implementation-confirmation is pending**; the **H9 removal direction** is
  **accepted in principle** (rev 35, 2026-10-04: remove `max_inflight_total`;
  per-tick dispatch is bounded by `max_inflight_per_tick`/quota, the stage queues,
  and `max_tasks_total`; `Waiting` is not in-flight; `tasks.in_flight` is an
  ephemeral per-tick scheduler batch only; pending T01 integrator acceptance), while
  the residual-set
  semantics and the dispatcher `Ready→Running`-vs-atomic-commit relationship
  remain **open `[INT]` items** (the rev-22 "single-phase in-flight bound, fixed
  here" claim is
  withdrawn); the in-flight bound is **not** claimed resolved and no bound is
  frozen.
- **Global:** no `/6` hashed claim; every `M1AppendSchema` row marks proposed vs
  unresolved distinctly; T13 is a pending amendment.

---

## 9A. Proposed `/6` co-freeze work queue (rev 39)

**This section is a proposed question list and authority assignment map — not
accepted schema, not a freeze, and not a chip authorization.** Every item below is
a **question to resolve** or an **authority assignment**, grounded only in the six
completed read-only reports already integrated (their point-by-point disposition is
the M1 proposal `§17`–`§21`; the detailed owner-facing rows in §C–§I and the §13
amendment table remain the **source of truth** for content). Nothing here restates,
narrows, or overrides those rows; where this queue and a §C–§I owner row differ,
the owner row controls. **No new user decision is made or required by this
queue:** the six reports surface no question that is not already covered by the
accepted-in-principle choices (A/B/C, the H1 allocation, A-narrow/B, H6/H9
directions) or by an already-recorded `[INT]`/`[OWNER]` blocker. A model or
subagent review does **not** count as an owner or T01 integrator sign-off; every
`[OWNER]`/`[INT]` item below stays pending until the named human role records it.
All contents are **proposed**.

**Reading the queue.** Each item names the grounded reports, the exact open
questions, and the authority. `[INT]` = T01 integrator decision; `[OWNER:*]` =
named group-owner sign-off; `[DOC]` = documentation reconciliation; `[USER]` =
already-used user in-principle direction (no new decision requested).

### 9A.1 Phase gates and dependency ordering

The three gates below are **separate**; passing one does not imply the next.

1. **Gate 1 — before any chip code (T01 freeze).** T01 must freeze the **shared
   types, task/result envelope, and append schema** first (the T01 integrator owns
   the bus/type schema, task enums, routing, and registration per
   [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) §2). Only **after** that frozen
   contract exists may the repo's **Wave 1** dispatch, and even then **only the
   assigned chips** of Wave 1 (`T02, T03, T04, T06, T08, T11, T13` per
   [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) §1) may be dispatched. No chip
   wave may implement or depend on any pipeline register or un-frozen shape before
   this gate (§8 item 4). **The freezes are serial:** T01 consolidates and hashes
   only **after** the upstream owner shapes are accepted (§9A.3).
2. **Gate 2 — Wave 2 needs real upstream contracts/artifacts.** Wave 2
   (`T05, T07, T08 init, T09, T11, T12`;
   [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) §1) requires the **real committed
   upstream outputs** (frozen request/result fields and committed records), not
   only fixtures; mocks do not substitute. Local parts may be drafted in parallel,
   but integration is gated on the real artifacts.
3. **Gate 3 — full M1 end-to-end run is a separate Part B gate.** A full M1
   end-to-end **executed** run additionally requires a **Linux probe
   substrate/toolchain/assembler/linker/sysroot/runner**, which **is not
   available** (T01 §6: every concrete ABI value is **UNVERIFIED** until an actual
   Linux probe runs; the probe fixture is recorded but unrun). Part A is
   **symbolic** and probe-independent. **No claim is made that the substrate,
   toolchain, assembler, linker, sysroot, or runner exists or is provisioned.**
   Gate 3 is **Part B**, separate from the Part A `/6` freeze.

### 9A.2 Parallel owner-drafting / review tracks (may proceed now)

These are **genuinely parallel** because they are owner-authored shape proposals
and reviews that do not mutate the frozen bus, and they can be drafted while
shared-schema decisions are still being formed. Drafting is not freezing, and a
draft from one track must not invent another track's shape.

| Track | Reports / rows | May proceed in parallel because | Not permitted |
|---|---|---|---|
| C + E (artifacts/literals + scope/symbol/type) | H1/row C (`§17.1`, `§18.1`), T06/E (`§17.3`) | Different stores/families and no shared carrier between them; each owner drafts its own enum/field questions | No mutual field references, no private family reuse |
| A + T13 (pipeline + verifier) | pipeline T02/T13 (`§17.7`, `§18.4`/`§18.6`) | T13's batch-verification fixtures can be designed while the T01 shared scheduler schema is still draft | T13 may not define a verifier interface; VD/CT chips may not bind a stage until `/6` |
| H (IR rule inventory) | T09/H (`§17.6`, `§18.5`/`§18.8`) | The `(rejection → rule-id)` inventory and marker question can be inventoried (§H3/§H8/H11) while the shared marker family is still open | No rule may be claimed hashed; no marker family may be invented before T01 consolidation |

### 9A.3 Serial freeze dependencies (must wait, in order)

- **T01 consolidates and hashes only after the upstream owner shapes are
  accepted.** The `M1AppendSchema` self-consistency / seed-equality test
  (§I2, §3.3) is a **late, serial** step; it cannot run until the per-family enum
  inventories, tags, limits, and rule ids are accepted by their owners and handed
  to T01.
- **Coding follows the frozen types.** No implementation of any new shared type,
  task kind, record family, or rule begins before Gate 1.
- **Wave 2 follows real upstream outputs** (Gate 2), not fixtures.
- **Gate 3 (Part B execution)** follows the Part A `/6` freeze and the
  provisioning of the probe substrate; it is not a chip-drafting dependency.

### 9A.4 Work items (minimum set; all proposed, all authority-assigned)

| # | Grounded in | Open questions to resolve (proposed) | Authority |
|---|---|---|---|
| **A** | pipeline T02/T13 report (`§17.7`, `§18.4`/`§18.6`) + row A | **A/T02/T13:** (a) **H6** — **scope now decided (rev 42, 2026-10-05): freeze the quota>1-capable bounded mechanism in `/6` (deferral rejected)**; the remaining work is to settle its **exact atomic-commit realization**, explicitly including the **pre-dispatch vs semantic-commit** error paths (pre-dispatch errors before state mutation leave tasks `Ready`; a semantic batch commit failure commits no semantic state; the deterministic bounded recovery mutation then processes dispatched tasks once in dispatch order to `Failed`; no pre-reservation of N diagnostics; per-task `DiagnosticId::NONE` attempt; state guard) and how the exactly-once dispatch-order `Failed` transition is realized atomically; (b) **H9** pin the **exact mutation boundary / sole writers** (dispatcher pre-worker `Ready→Running` vs the one ordered atomic semantic commit), the **residual-inflight exhaustive paths** (every path leaving no residual in-flight task, incl. empty proposal vectors), and the **limit/stage/hash/error numeric inventory** (`max_inflight_per_tick`, `stage_queue_bound`, `max_tasks_total`, `max_task_progress`, stage set, config/commit/selection error codes); (c) **fixtures and CT07/sibling policy coordination** — the T02/T13 batch fixtures and the CT07 committed join-decision carrier / sibling-failure policy must be co-designed so they do not diverge (**the CT07-invariant/no-new-carrier subdecision is accepted, rev 42**). | H6 scope+mechanism `[USER]`+`[OWNER:T02,T13]`+`[INT]` accepted 2026-10-05 (rev 42); `[INT]` for (a)/(b) remaining; `[OWNER:T02,T13]` for fixtures (c); CT07/sibling `[OWNER:T02,T05]`; removal direction `[USER]` already. **Rev 49 read-only H9 audit (unresolved blockers; no change to the selected H6 mechanism or the H9 removal direction, no freeze/code):** `/5` proves only the **quota=1 single-task** `fail_selected`, so the proposed **quota>1 pipeline is not implemented**; (b) now explicitly also needs the **dispatcher `Ready→Running`/`in_flight`-population relationship to the single ordered atomic commit**, the **exact `in_flight` clear ownership/order relative to the bounded H6 recovery**, and a **no-`Running`/no-residual proof for all success/error/empty-proposal paths**; the **`max_dispatches_per_tick`-vs-`max_inflight_per_tick` internal proposed-limit conflict** must be **resolved in `/6`** (not decided now); the **H6/H9 fan-out fixtures** and **T13 VF02/VF03/VF04/VF13** remain **pending**; the no-residual guarantee is **conditional on the H6 recovery implementation**, and the **quota=1 M1 baseline stays separated** from the quota>1 optimization. **Rev 51 integration-selected candidate default under user delegation (2026-10-05; not an owner/T01 `[INT]` signoff):** `max_inflight_per_tick`/quota is the **sole per-tick dispatch-count bound**; the redundant `max_dispatches_per_tick` is **dropped** from the candidate limit inventory and validation (grounded in the rev-35/38 H9 direction + the rev-49 audit duplication finding); exact names/defaults/codes remain **T01/T02 `/6` co-freeze**, T02/T13 tests **pending**. |
| **B** | H1/row C report (`§17.1`, `§18.1`) + row C | **C/T03/T04/T07/T08:** settle `ArtifactRecord`/artifact-map shape; `LiteralRecord` variants + `LX08` (`Lx08CandidateType`) encoding; `ConstantRequest`/`ConstantResult` shapes and `legality` carrier; the `RequiredKind` enum vocabulary; the `max_const_bits` numeric **carrier** (bound to the literal/const request or config projection); the **writer allowlist** rows; and the `M1AppendSchema` **hash/tag inventories** for these families. | `[OWNER:T03,T04,T08]` (+`T07` for `ConstantRequest`); `[INT]` for hash/tag inventory and enum consolidation; allocation `[USER]` already. **Rev 44 accepted subdecisions (2026-10-05):** **C/H1/T07/T08/T01** — `required_kind` = per-use constant-expression requirement (e.g. M1 integer constant expression), distinct from `candidate_type`, no duplicate implicit target type; `legality` = result payload field, no extra committed family (exact `ConstLegality` variants open); **C/T03/T01** — `ArtifactRecord { kind, source, bytes, raw_offsets }` + total eight-`ArtifactKind` map rule, M1 scope only single-source `Normalized`. **Rev 45 accepted subdecisions (2026-10-05):** **C/T04/T08/T01** — exact ordered `LiteralRecord` fields (`token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` big-endian magnitude, `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`; no `node`/`required_kind`), M1 `LiteralKind {Integer, Character, String}` (only `Integer` produced) / `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced) / radix `{2,8,10,16}` (M1 decimal only), symbolic `Lx08CandidateType` (M1 2/3 = `Int`, no bit width); **C/T07/T08/T01** — `RequiredKind` M1 enum only `IntegerConstantExpression`, `ConstLegality` values `Legal`/`NotConstantExpression`/`Unsupported`, future purposes via appended variants/new rules; **C/T03/T01** — mandatory-map invariants (`len == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds need a valid source). Remaining items stay `[OWNER]`/`[INT]` pending, incl. the **complete `Lx08CandidateType` member set/numeric encodings**, the **optional-kind artifact-map rule**, source-versus-payload equality, and exact artifact error codes. **Rev 47 delegated candidate default (2026-10-05; user-delegated integration default, not a T03/T04/T07/T08 owner signoff or T01 `[INT]` acceptance):** the **optional-kind map policy** is selected — if `requires_map(kind) == false`, `raw_offsets` **must be empty**; `source` stays `Option<SourceId>` and must be valid when `Some`; mandatory-map kinds require a valid `source` plus the accepted rev-45 invariants; **no** source-payload-equals-artifact-bytes requirement (normalization transforms); any exact source-provenance/equality rule **deferred**; exact numeric error codes stay open. |
| **D** | T05/D report (`§17.2`, `§18.3`/`§18.4`) + row D | **D/T05/T01/T02:** the **exact `ContinuationRecord` wire shape/tags** (**now, rev 43: the ordered fields/shapes are user-accepted; the `RecordRef` numeric wire tags and `RecordFamily` ordinals stay separate inventories and their exact numeric encodings remain a T01 `/6` freeze detail**); the **B supersession** in **T01 §4** (`WaitSet`-only, `/6`-only amendment) and its catalog entry (**now, rev 43: formally accepted `/6`-only; no `/5` edit**); the **A-narrow preapply checks/error codes** (`ContinuationRefInvalid`/`AwaitChildrenRefInvalid` names and phases; **now, rev 43: the exact pre-apply proposal — phase-1 continuation ref within the same task `AppendRecords` range/family `Continuation`, phase-2b child ref within the same task Enqueue list, whole-batch `CommitError` before any mutation — is user-accepted; the numerical codes/hash remain T01 `/6` details**); **CT07 carrier** or remove CT07 from the join path (**now: no new CT07 carrier/family — join is a commit-apply invariant, rev 42**); the **sibling-failure policy** (**now: await-all children terminal before the parent decision, rev 42**); the **parse contract/tests** (**still PENDING**). | `[OWNER:T05]` + `[INT]`; T01 §4 supersession T01 `[INT]` + `[OWNER:T05]` accepted 2026-10-05 (rev 43); continuation fields `[OWNER:T05]` and preapply `[OWNER:T05,T02]` + T01 `[INT]` accepted 2026-10-05 (rev 43); CT07/sibling `[OWNER:T02,T05]` + T01 `[INT]` accepted 2026-10-05 (rev 42); decisions A/B `[USER]` already; residual `[INT]` = tags/ordinals/encoder/`ParseContext`/error numeric codes/hash and parse request-result-test shapes. **Rev 47 delegated candidate default (2026-10-05; user-delegated integration default, not a T05/T01/T02 signoff):** T05 parse depth reuses `limits.max_task_depth`, counts parser continuation/child frames only (not total bus tasks), detects the limit **before any child enqueue for the descent**, and reports excess as the `ParseDepthExceeded` **chip diagnostic**; the exact `ParseContext` encoding and parse request/result variants stay pending. |
| **E** | T06/E report (`§17.3`, `§18.3`) + row E | **E/T06/T01/T05:** the **scope record lifecycle/bootstrap** — how the first T06 task is scheduled **after** the committed TU (H3; not job-bootstrap) (**now, rev 43: the File Enter trigger is user-accepted as a deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge after the committed TU, carrying the committed `NodeId`, no job-bootstrap; the bootstrap wiring beyond the stage edge remains `[INT]`**); the **decl/lookup tie semantics** (Identifier-leaf `decl`, active-chain tie rule); the **canonical `TypeId` reuse explicit mechanism** (T06-2) (**now: deterministic bounded committed-arena scan returning the lowest matching `TypeId`, no cache/index, rev 42**); the **namespace mapping/manifest allowlist** rows and tests. | `[OWNER:T06]` + `[INT]`; lowest-id reuse mechanism accepted 2026-10-05 (rev 42); File Enter stage edge accepted 2026-10-05 (rev 43); bootstrap ordering `[INT]`; T05 upstream committed-TU carrier `[OWNER:T05]` **pending** (the accepted stage-edge direction is not a TU schema signoff); namespace/allowlist rows/tests pending. **Rev 47 delegated candidate default (2026-10-05; user-delegated integration default, not a T06/T05 owner signoff or T01 `[INT]` acceptance):** namespace is derived as a **closed function of `SymbolKind`** (`Object`/`Function`/`Typedef`/`EnumConst` → `Ordinary`; `StructTag`/`UnionTag`/`EnumTag` → `Tag`; `Label` → `Label`) with **no namespace field**; a wrong-namespace lookup is a **miss, not a conflict**; the deterministic **TY03** lookup walks the **active scope chain only**, orders candidate declarations by `(source,start,end,NodeId)`, picks the greatest ≤ query point, same-scope tie to the higher `SymbolId`, else innermost active scope. The T05 `NodeKind`/token-range dependency and the exact event encoding/lifecycle/allowlist rows remain **remaining co-freeze**. **Rev 54 (read-only, open; no mechanism selected):** the accepted File Enter edge still needs an **exactly-once consumed TU** with **atomic consumption/dedupe** so the committed TU triggers File Enter **once** (no duplicate/lost edge) — an open `/6` co-freeze item (shared with bundle **D**). |
| **F** | T07/F report (`§17.4`, `§18.5`) + row F | **F/T06/T07/T09/T13:** resolve the **`FunctionContextId` conflict** (**now: no `FunctionContextId`, rev 42**); decide **`SemRecord` == `CheckedNode`** identity and the **typed T09 consumption link** (**now: `SemRecord` is the committed materialization of `CheckedNode`, one per `NodeId`, explicit committed typed link consumed by T09, rev 42**); complete the **`ConversionOp`/`ConversionRole` permitted matrix** and its **IR emission**; bind **VF06** to a stage (**now, rev 46: VF06 `TypedAstInvariant` is selected in M1; the exact registered kind/stage/phase/interface is still open**); pin the **`EffectMask`/`ValueCategory`** encodings; fix diagnostics/rules/tests. **Rev 46 selected subdecisions (2026-10-05):** **F/T07** — `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` with M1 fixture = `NonLvalue`, and M1 only `EffectMask(0)` (nonzero typed unsupported/diagnostic; bit classes reserved/unassigned); **F/T07/T13/T01** — VF06 `TypedAstInvariant` is **in M1**, executing after committed T07 `SemRecord`s and before T09 lowering, checking M1 typed-fact/required-conversion completeness (exact registered kind/stage/phase/interface still T01/T13 open); **F/T06/T07/T09/T01** — **M1-minimal conversion scope only** (freeze only M1-fixture-needed behavior incl. identity/no-conversion; integer promotions, float conversions incl. `FloatToFloat`, pointer qualifier, and other non-M1 conversions **explicitly unsupported/deferred** to a later append/contract revision; no `ConversionOp`/`ConversionRole` closed list, pairing, role→chip mapping, or numeric encoding invented — open `/6` co-freeze details). | `[OWNER:T07,T06,T09]` carrier decisions accepted 2026-10-05 (rev 42); **rev 46** `ValueCategory`/`EffectMask` `[OWNER:T07]` + VF06-in-M1 `[OWNER:T07,T13]` + M1-conversion-scope `[OWNER:T06,T07,T09]` accepted, exact VF06 registration `[INT]`/`[OWNER:T13]` pending; `[INT]` consolidation |
| **G** | T08 report (`§17.5`) + row G | **G/T08/T01:** the `max_const_bits` **bound/carrier/diagnostic enforcement** (default/formula/`<=128` config constraint/restricted-chip config projection), and the split between **Part A symbolic** vs **probe-gated target bits** (no target bit pattern in Part A). | `[OWNER:T08]` + `[INT]`; probe gating `[INT]`. **Rev 44 accepted subdecision (2026-10-05):** origin is a hashed `Limits` value, M1 cap/default 128, config rejects > 128, explicit task-input projection to the restricted T08 chip; exact wire field/diagnostic code/hash encoding remain `/6`; formula/enforcement/symbolic-vs-probe stay `[OWNER]`/`[INT]` pending. **Rev 45 accepted subdecisions (2026-10-05):** **C/T07/T08/T01** — `RequiredKind` M1 enum only `IntegerConstantExpression` and `ConstLegality` values `Legal`/`NotConstantExpression`/`Unsupported`, future C constant-expression purposes via appended variants/new rules (not lexical-candidate reuse); exact `ConstLegality`/`RequiredKind` numeric encodings remain `[INT]` pending. **Rev 54 (docs-only supersession record):** the rev-21 signed-range/`i128` representable-set formula, the `ConstRecord.value: i128` carrier, and the chip-level enforcement wording are **historical and superseded — not selected**; only the rev-44 origin/cap/projection remains accepted, and the exact formula/carrier/enforcement stay open `/6` co-freeze items. |
| **H** | T09/H report (`§17.6`, `§18.5`/`§18.8`) + row H | **H/T09/T01/T13/T08:** the **function-completion fact/trigger** (H11 — **rev 49: reuse the committed IR28 `FunctionEnd` terminal result, no new marker**); the **collapse rule aliases** — pick one of each pair (`ir.op-immediate-type` vs `…-result-ty`; `ir.terminator-missing` vs `…-explicit-commit-marker`) (H8); the **canonical `Constant` validation precedence** (missing-immediate vs missing-result); the **hash and verifier fixtures**. | `[OWNER:T09]` + `[INT]` for hash/count; fixtures `[OWNER:T13]`; folded-`int5` split `[OWNER:T08,T09]`. **Rev 47 delegated candidate default (2026-10-05; user-delegated integration default, not a T09 owner signoff or T01 `[INT]` acceptance):** choose the **shorter** aliases `ir.op-immediate-type` and `ir.terminator-missing`; when `Constant` is missing **both** immediate and result, validate the **immediate first**; the target type equals the result `ValueRecord.ty`. All remain **prospective `/6`** (owner/T01 acceptance and hashing pending). **Rev 49 (user, 2026-10-05; explicit critical H11/T09/T01 direction):** select **reuse of the committed IR28 `FunctionEnd` terminal result** (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic function-completion fact, checked by a **T01-owned typed phase-2b commit-apply validation hook**; **no** new `CompletedFunction` record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder or new `ResultValue` variant; the earlier new-marker direction is **superseded for the operative direction** and preserved as history. **Not a frozen `/6` hook/schema, no code:** the exact hook contract, hash impact (likely no new record family; rule/hook hashing still a T01 decision), result typing/commit ordering, and T09/T01 co-freeze remain **open**; overall H stays **pending**. |
| **I** | Global report (`§17.8`) + row I | **I/T01:** one `/6` envelope; resolve the **`M1AppendSchema` vs `COMPILER_SFL_MANIFEST.md` §4 hash-source contradiction** (§I5); reconcile **IDs/tags/`RecordFamily` ordinal counts/self-consistency**; catalogue **all T01 §4 supersessions** (the A-narrow/B awaited-child supersession plus the "control computes / dispatcher applies" and "commit path/backend writes" wording) for the `/6` inventory only. **Rev 48 read-only audit (docs-only; no option selected):** the `/5` code fact is **24** `RECORD_KINDS` / **24** `RecordRef` variants (tags 0–23), `RecordFamily` **absent**; the 19/27/27 draft counts are **proposed/unverified**; the hash-scope conflict is **real and unresolved** with the three options (hash `M1AppendSchema`; don't hash + fix manifest; explicit two-tier seed-vs-runtime) **unselected/unrecommended**; `/6` must specify **dual-inventory encoding**, **numeric-value inclusion in the hash**, and the **self-consistency mechanism**, freezing actual counts/values only after owner-shape acceptance. **Rev 50 (`[USER]`, 2026-10-05, critical hash-scope decision):** the two-tier model is **accepted** — `foundation + M1AppendSchema` is the **frozen `/6` seed hashed**, **post-seed runtime `declare()` extensions excluded** and captured/validated via runtime snapshot/schema mechanisms; at `/6` integration T01 must **atomically** update the manifest §4 wording, the `hash_excludes` semantic description/token (**post-seed runtime declarations**, not the M1 seed), and `FrozenSchema::encode`/contract code + freeze test, **preserving the `/5` hash/history**; **no `/6` values/hash/counts/code//5 change**, and T01 still co-freezes the M1 seed values after owners; the **numeric inventory and test implementation remain pending**; row I stays PENDING. | `[USER]` for the rev-50 conceptual hash scope + `[INT]` + `[DOC]`; T01 §4 catalog `[INT]` (no `/5` edit); hash-scope **conceptual choice now settled `[USER]`**, exact numeric inventory/implementation still `[INT]` critical, no implementation authorization |

**Authority honesty.** The `[OWNER:*]` entries are **pending human sign-offs**
for their **remaining open items** (as of the prior revision rev 41 this blanket
statement held; **current truth rev 46:** selected owner subdecisions now carry
authority responses recorded in rev 42/43/44/45 — see §9C/§10/§12 rev 42/43/44/45 — while every
row's remaining items and the overall rows stay pending);
the `[INT]` entries are pending T01 integrator decisions. No report, ledger,
audit, subagent, or model review is a sign-off. The `[USER]` entries reference the
**already-recorded** 2026-10-04 in-principle directions; this queue requests **no
new user decision**. **Rev 42 update (2026-10-05):** the user, self-identifying as
all named T01/owner roles, selected a small set of subdecisions (A H6
scope+mechanism, D CT07-invariant+await-all, F carrier decisions, E lowest-id
scan); they are recorded per role label and date in §9C/§10 and **do not close**
row A/D/E/F or any other row, whose remaining items stay `[INT]`/`[OWNER]`
pending. **Rev 43 update (2026-10-05):** the user further selected **D**
subdecisions (the `ContinuationRecord` exact ordered fields/shapes, the formal `/6`
T01 §4 supersession, and the Exact OwnBatch pre-apply validation) and the **E**
File Enter deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage
edge; these are recorded per role label and date in §9C/§10 and likewise **do not
close** row D/E; the `RecordRef` numeric tag values / `RecordFamily` ordinals (kept
as **separate inventories**), the `ParseContext` vocabulary, the OwnBatch numerical
error codes, and the T05 committed-TU upstream all stay `[INT]`/`[OWNER]` pending.
**Rev 44 update (2026-10-05):** the user further selected **C** subdecisions
(`required_kind` per-use constant-expression requirement + distinct `candidate_type`,
`legality` as a result payload field with no extra committed family, and the
`ArtifactRecord { kind, source, bytes, raw_offsets }` + total eight-`ArtifactKind`
map rule with M1 scope only single-source `Normalized`) and the **G** subdecision
(`max_const_bits` origin a hashed `Limits` value, M1 cap/default 128, config rejects
> 128, explicit task-input projection to the restricted T08 chip); these are
recorded per role label and date in §9C/§10 and likewise **do not close** row C/G;
the exact `ConstLegality` variants, the `RequiredKind` enum codes, the artifact map
boundary formula/error classification, the `raw_offsets` detailed invariants, the
numeric IDs/tags, and all enum numeric codes stay `[INT]`/`[OWNER]` pending.
**Rev 45 update (2026-10-05):** the user further selected **C**/**G** subdecisions —
the exact ordered `LiteralRecord` fields and M1 `LiteralKind`/`LiteralSuffix`/radix
scope, the symbolic `Lx08CandidateType` (M1 literals 2/3 = `Int`, no bit width), the
`RequiredKind` M1 enum `IntegerConstantExpression`, the `ConstLegality` values
`Legal`/`NotConstantExpression`/`Unsupported` (future purposes via appended
variants/new rules), and the mandatory artifact-map invariants; these are recorded
per role label and date in §9C/§10 and likewise **do not close** row C/G; the
**complete `Lx08CandidateType` member set/numeric encodings**, the **optional-kind
artifact-map rule**, source-versus-payload equality, the exact artifact error
classification/numeric codes, and all enum numeric codes stay `[INT]`/`[OWNER]`
pending.
**Rev 46 update (2026-10-05):** the user further selected **F** subdecisions — the
`ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` discriminants
(M1 fixture = `NonLvalue`), the M1-only `EffectMask(0)` rule (nonzero typed
unsupported/diagnostic; bit classes reserved/unassigned), the VF06
`TypedAstInvariant` **in M1** (after committed T07 `SemRecord`s, before T09 lowering,
M1 typed-fact/required-conversion completeness; exact registration still open), and
the **M1-minimal conversion scope** with all non-M1 conversions explicitly
deferred/unsupported (no `ConversionOp`/`ConversionRole` closed list, pairing, role→chip
mapping, or numeric encodings invented); these are recorded per role label and date in
§9C/§10 and likewise **do not close** row F; the VF06 exact registered task
kind/stage/phase/interface and all conversion/effect encodings stay `[INT]`/`[OWNER]`
pending.
**Rev 47 update (2026-10-05):** under the user's explicit **delegation** ("for
non-critical decisions adopt the recommended choice directly"), the **M1
integration agent** selected four **candidate defaults** for the remaining open
items — the optional-`ArtifactKind` map policy (B/C), T05 parse depth (D), T06
namespace/symbol lookup (E), and the T09 noncritical rule defaults (H). These are
**selected under user-delegated integration default**, **not** an `[OWNER:*]` or
T01 `[INT]` sign-off, and **not** evidence of T03/T05/T06/T09 owner acceptance;
they are recorded separately (see §9B/§9C/§10) and every **remaining** item and the
**overall** rows B/C/D/E/H stay `[INT]`/`[OWNER]` **pending** pending final `/6`
co-freeze. The `CompletedFunction` marker family (H) stays unresolved.
**Rev 48 update (2026-10-05):** a **read-only T01 audit** (docs-only; no code/T01/
manifest/other-doc edit, no hash-scope option selected, no `/6` freeze) confirmed
the `/5` code facts — **24** `RECORD_KINDS` / **24** `RecordRef` variants (tags
0–23), `RecordFamily` **absent** — against the CDR's **proposed/unverified**
19/27/27 draft counts, and confirmed the `M1AppendSchema` **hash-scope conflict**
between `CONTRACT_VERSION`/`contract.rs`/README and `COMPILER_SFL_MANIFEST.md` §4
as **real and unresolved**; this is a **critical `/6` hash-scope choice** with
**no implementation authorization**, and no owner/T01 signoff is supplied by the
audit. Every overall row and the `M1AppendSchema` schema remain **pending**.
**Rev 49 update (2026-10-05):** the user's explicit **critical H11/T09/T01
direction** selects **reuse of the committed IR28 `FunctionEnd` terminal result**
(`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic
function-completion fact, checked by a **T01-owned typed phase-2b commit-apply
validation hook**, with **no** new marker record family/ID/arena/`RecordRef` tag/
`RecordFamily` ordinal/snapshot encoder or `ResultValue` variant (the earlier
new-marker direction is **superseded for the operative direction**, preserved as
history; not a frozen `/6` hook/schema, no code; exact hook contract/hash impact
(likely no new record family; rule/hook hashing still a T01 decision)/result
typing/commit ordering/T09/T01 co-freeze open); and the **read-only H9 audit
findings** are recorded as **unresolved blockers** without changing the selected
H6 mechanism or the H9 removal direction (quota>1 pipeline unimplemented;
`Ready→Running`/`in_flight`-commit relationship and `in_flight` clear ownership/order
open; no-`Running`/residual proof for all paths; `max_dispatches_per_tick`-vs-
`max_inflight_per_tick` conflict **deferred to `/6` at the time — now historical,
superseded by the rev-51 drop of `max_dispatches_per_tick`, see below**; fan-out
fixtures/VF02/VF03/VF04/
VF13 pending). The audit is **not a signature**; every overall row and the
`M1AppendSchema` schema remain **pending**; overall H stays **pending**.
**Rev 50 update (2026-10-05):** the **user's explicit critical hash-scope
decision** (`[USER]`) accepts the **two-tier model** — `foundation + M1AppendSchema`
is the **frozen `/6` seed and participates in the `/6` contract hash**, while
**post-seed runtime `declare()` extensions remain excluded** and are
captured/validated via **runtime snapshot/schema mechanisms**; at `/6` integration
T01 must **atomically** update the manifest §4 wording, the `hash_excludes` semantic
description/token (**post-seed runtime declarations**, not the M1 seed), and
`FrozenSchema::encode`/contract code + freeze test, **preserving the `/5`
hash/history**. This **settles the conceptual boundary** (no longer open at the
conceptual level) but **accepts no exact `M1AppendSchema` contents/counts and
freezes nothing**; T01 still co-freezes the M1 seed values after the owners, the
**numeric inventory and test implementation remain pending**, and **row I / every
overall row stay `PENDING`**; no code, no `/5` change, no `/6` freeze.
**Rev 51 update (2026-10-05):** under the user's explicit **delegation** for
non-critical decisions, the **M1 integration agent** selected a **candidate
default** (not an owner/T01 `[INT]` signoff, not a freeze) — `max_inflight_per_tick`/
quota is the **sole per-tick dispatch-count bound** and the redundant
`max_dispatches_per_tick` is **dropped** from the candidate limit inventory and
validation (grounded in the rev-35/38 H9 direction and the rev-49 audit duplication
finding); exact names/defaults/codes remain **T01/T02 `/6` co-freeze** and the
T02/T13 tests remain **pending**. It resolves no dispatcher-`Ready→Running`/
`in_flight` atomic-boundary or clear-order item and leaves **row A / every overall
row `PENDING`**; no code, no `/5` change, no `/6` freeze.
**Rev 53 update (2026-10-05):** under the user's explicit **delegation** for
non-critical decisions, the **M1 integration agent** selected a **candidate default**
(not an owner/T04 or T01 `[INT]` signoff, not a freeze) for the **M1 exercised literal
subset**: the `Lx08CandidateType` M1 vocabulary is the closed one-member set
**`{ Int }`** and M1 literals `2`/`3` are **target-independent `Int` with no bit
width** (the `candidate_type` value on the committed `LiteralRecord`; not a target
type/ABI type/`TypeId`); forms **outside** the exercised M1 subset (non-decimal radix,
non-`None` suffix, character/string literals, or any unspecified category) **must not
silently default to `Int`** and are **explicitly unsupported/deferred** until their
categories/rules exist; future categories **append symbolic members/rules without
reinterpreting** the selected `Int` member; and **no** numeric tag/encoding is
asserted. This is **not** the complete future C candidate vocabulary, **not** T04/T08
owner or T01 `[INT]` signoff, and **not** a `/6` freeze; the **complete
`Lx08CandidateType` member set and all numeric encodings remain open**. It changes no
field order/type/count, leaves every overall row **`PENDING`**, and authorizes no code
and no `/6` freeze.
**Rev 54 update (2026-10-05):** docs-only **read-only-audit + historical-supersession
cleanup** (not a signoff, not a `/6` freeze, no code): the §G2 `max_const_bits`
signed-range/`i128`/enforcement rev-21 wording is marked **historical/superseded — not
selected** (only the rev-44 origin/cap/projection retained; formula/carrier/enforcement
open); the rev-49 H9 duplicate-limit conflict is marked **historical/superseded by
rev-51** (dispatch/clear-order still open); and the open findings — **T04 reciprocal
token↔literal cycle-safe ID reservation**, **T05/T06 TU-edge exactly-once atomic
consumption/dedupe**, **T13 no-residual fixtures incomplete**, and **M1 vertical PP-08
failure semantics + exact artifact-map arrays pending** — are recorded without any
architecture-critical choice. Every overall row stays **`PENDING`**; `/5` current, no
code.

---

## 9B. Decision-ready recommendation appendix (rev 40)

**This section is a proposed-default decision aid — not accepted schema, not a
freeze, and not a chip authorization.** It restates, per row, a **single concise
recommendation** the named authority may **accept, amend, or reject**, plus the
one-line tradeoff/open dependency that makes the choice non-trivial. It is
**grounded only in the six completed read-only reports already integrated** (H1/row
C, T05/row D, T02/T13 rows H6–H9, T06/row E, T07/row F, T09/row H), whose
point-by-point disposition is the M1 proposal `§17`–`§21` and whose detailed owner
rows in §C–§I/§13 remain the **source of truth**. Where this appendix and a §C–§I
owner row differ, **the owner row controls**; where §9A and this appendix differ,
**§9A's authority assignment controls**. As of when it was added (rev 40) it did
**not** reproduce the six reports, state a new finding, or change any acceptance
status; **current truth (rev 49):** the appendix itself still only *records* (does
not decide) statuses, but since rev 42/43/44/45/46 it now **carries named partial
subdecision acceptances** in rows C/D/E/F (see §10 and §12 rev 42/43/44/45/46), and
since **rev 49** it carries the user's selected **H11 `FunctionEnd`-terminal-result
direction** in row H plus the **read-only H9 audit blockers** (see §10 and §12
rev 49), while the
**remaining full-bundle sign-offs and the overall rows C–I stay `PENDING`**.

**What is authority vs what is merely grounded in a draft.** Two classes must not
be conflated. **(a) Already-recorded user in-principle direction** (2026-10-04):
the A/B/C direction, the H1 allocation split, the A-narrow/B sub-decisions, and the
H6/H9 directions. These are **not restated as decided here**, are **not
weakened**, and in particular the user's **accepted-in-principle H6 direction must
not be erased** — nothing below may be read as withdrawing it. **(b) Proposed
defaults** recommended in this appendix: every row recommendation below is a
**proposal for the authorized owners/T01 integrator to accept, amend, or reject**,
grounded in the existing drafts, **not** an authority decision and **not** a
sign-off. **No new user decision was made or requested by this appendix when it was
added in rev 40**, and at that time no row's status changed: `/5` remained current,
M1 remained **DRAFT**, ADR-0002 remained **PROPOSED**, T01 integrator acceptance and
**all** owner sign-offs remained **pending**, and **rows D–I remained PENDING**.
**Rev 42 (2026-10-05) subsequently records only named partial subdecisions** (the
per-subdecision acceptances itemized in §10 and §12 rev 42), each marked in the row
below; **the overall rows D–I remain pending** because their other items are
unresolved. **Rev 43 (2026-10-05) adds only further named partial D/E
subdecisions** (the exact `ContinuationRecord` ordered fields/shapes, the formal `/6`
T01 §4 supersession, the exact `OwnBatch` pre-apply validation, and the File Enter
deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge),
itemized in §10 and §12 rev 43 and marked in rows D/E below; **the overall rows D–I
still remain pending** because their other items are unresolved, and the `RecordRef`
numeric tag values / `RecordFamily` ordinals (kept as separate inventories) and the
`ParseContext` vocabulary stay open. **Rev 44 (2026-10-05) adds only further named
partial C/G subdecisions** (the `required_kind` per-use constant-expression meaning
and `legality` result-payload placement; the `ArtifactRecord` shape + total
eight-`ArtifactKind` map rule with M1 scope only single-source `Normalized`; and the
`max_const_bits` hashed-`Limits` origin / cap 128 / config `> 128` rejection /
task-input projection), itemized in §10 and §12 rev 44 and marked in row C below
(G is carried by row C's const items and §G); **the overall rows C–I still remain
pending** because their other items are unresolved, and the exact `ConstLegality`
variants, `RequiredKind` codes, artifact-map boundary formula/error classification,
`raw_offsets` invariants, numeric IDs/tags, and all enum numeric codes stay open.
**Rev 45 (2026-10-05) adds only further named partial C/G subdecisions** (the exact
ordered `LiteralRecord` fields + M1 `LiteralKind`/`LiteralSuffix`/radix scope, the
symbolic `Lx08CandidateType` with M1 literals 2/3 = `Int` and no bit width, the
`RequiredKind` M1 enum `IntegerConstantExpression`, the `ConstLegality` values
`Legal`/`NotConstantExpression`/`Unsupported`, and the mandatory artifact-map
invariants), itemized in §10 and §12 rev 45 and marked in row C below; **the overall
rows C–I still remain pending**, and the **complete `Lx08CandidateType` member
set/numeric encodings**, the optional-kind artifact-map rule, source-versus-payload
equality, and the exact artifact error classification/numeric codes stay open.
**Rev 46 (2026-10-05) adds only further named partial F subdecisions** (the
`ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` discriminants
with M1 fixture = `NonLvalue`; the M1-only `EffectMask(0)` rule with nonzero typed
unsupported/diagnostic and reserved/unassigned bit classes; the VF06
`TypedAstInvariant` **in M1** after committed T07 `SemRecord`s and before T09
lowering; and the **M1-minimal conversion scope** with non-M1 conversions explicitly
deferred), itemized in §10 and §12 rev 46 and marked in row F below; **the overall
rows C–I still remain pending**, and the exact VF06 registration plus all
conversion/effect encodings stay open.

| Row / authority | Proposed default recommendation (accept/amend/reject) | Tradeoff / open dependency |
|---|---|---|
| **C — H1 literal/const handoff** `[OWNER:T03,T04,T08]` `[INT]` | Keep the accepted-in-principle split: committed `LiteralRecord` = per-literal lexical facts + symbolic `LX08` candidate type (no `node`/`required_kind`); `ConstantRequest = { literal: RecordRef::Literal, node, required_kind }` (ref-based, not re-embedded); `ConstantResult` carries `legality`. Defer target-specific widths, multi-source maps, and Part B (`Assembly`/`Object`/`Snapshot`/`Trace`) writers out of M1. **Rev 44 selected (2026-10-05) — subdecisions only:** `required_kind` is the **per-use constant-expression requirement** (e.g. the M1 integer constant expression), **distinct** from the lexical `LiteralRecord.candidate_type`, and duplicates no implicit target type; `ConstantResult.legality` is a **result payload field** with **no** extra committed record family; and the `ArtifactRecord { kind, source, bytes, raw_offsets }` shape + the **total eight-`ArtifactKind`** map rule (map-mandatory `Normalized`/`Spliced`/`CommentFree`/`Preprocessed`; map-optional `Assembly`/`Object`/`Snapshot`/`Trace`) with M1 **exercised scope only single-source `Normalized`** and other producers/multi-source map deferred. **Rev 45 selected (2026-10-05) — subdecisions only:** the exact ordered `LiteralRecord` fields (`token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` big-endian magnitude, `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`; **no** `node`/`required_kind`); the M1 `LiteralKind {Integer, Character, String}` (only `Integer` produced) / `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced) / radix `{2,8,10,16}` (M1 decimal only) scope; the symbolic/target-independent `Lx08CandidateType` with M1 literals 2 and 3 as `Int` and **no bit width**; `RequiredKind` M1 enum only `IntegerConstantExpression`; `ConstLegality` values `Legal`/`NotConstantExpression`/`Unsupported` (future C constant-expression purposes via **appended variants/new rules**, not lexical-candidate reuse); and the mandatory artifact-map invariants (`len == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds need a valid source). **Still proposal/pending in this row:** the **complete `Lx08CandidateType` member set/numeric encodings** (deliberately not invented), the `ConstLegality`/`RequiredKind` enum numeric codes, the artifact map boundary formula/error classification, the **optional-kind artifact-map rule**, source-versus-payload equality, the `raw_offsets` detailed invariants, and all enum numeric codes. **Rev 47 delegated candidate default (2026-10-05) — selected under user-delegated integration default, not an owner/T01 signoff:** the optional-`ArtifactKind` map rule is selected (map-optional kinds have empty `raw_offsets`; `source` valid when `Some`; no source-payload-equals-bytes requirement; source-provenance/equality rule deferred; exact numeric error codes open). | **Unresolved:** exact `RequiredKind` semantics/vocabulary, the `legality` carrier domain, artifact-map invariants, and the `max_const_bits` projection/carrier. **T01 owns** the explicit **`RecordRef` wire-tag vs `RecordFamily` ordinal** inventories and their hashing in `M1AppendSchema` (§3.2/§3.3, §I2). **Rev 44:** the `required_kind`/`legality`/`ArtifactRecord`-shape subdecisions are **selected**; the `max_const_bits` origin (hashed `Limits` value; M1 cap/default 128; config rejects > 128; explicit task-input projection) is the companion **G** subdecision (see §G) — its exact wire field/diagnostic code/hash encoding stay open, and the cap is **not** a claim that every valid config value must equal 128. |
| **D — T05 continuation/join** `[OWNER:T05]` `[INT]` | Adopt the §D1 continuation field list as the recommendation (cite §D1; do not re-list here) with **no `awaited`** on the continuation (`WaitSet` only); resolve `OwnBatch(DraftRef)` in the **pre-apply** pass; realize the join as a **commit-apply invariant** with **no new CT07 family**; on resume, require **all** committed children `Completed` **then** parent decision (`Ready`/`Failed` once). | **Two subdecisions resolved on 2026-10-05 (rev 42):** the CT07 carrier question is **settled — no new CT07 committed carrier/family** (join is a commit-apply invariant), and the sibling policy question is **settled — await-all, i.e. all committed children terminal before the parent decision** (any `Failed` fails the parent once). This rev-40 row is retained **as a historical proposal**; those two are **no longer live open choices**. **Further resolved on 2026-10-05 (rev 43):** the `ContinuationRecord` **exact ordered fields/shapes** (no `awaited`; durable committed-ID references only), the **formal `/6` T01 §4 supersession** (`WaitSet` sole awaited-child source; no `/5` edit), and the **exact `OwnBatch` pre-apply validation** (phase-1 continuation ref within the same task `AppendRecords` range/family `Continuation`; phase-2b child ref within the same task Enqueue list; whole-batch `CommitError` before any mutation). **Still open:** the `RecordRef` numeric wire-tag values and `RecordFamily` ordinals (separate inventories), the `ParseContext` vocabulary final encoding, the OwnBatch numerical error codes/hash, and the parse request/result/test shapes. Parse depth reuses the existing `limits.max_task_depth` mapped to parser-chain frames (no new limit). **Rev 47 delegated candidate default (2026-10-05) — selected under user-delegated integration default, not a T05 owner/T01 `[INT]` signoff:** parse depth counts **parser continuation/child frames only** (not total bus tasks), detects the limit **before any child enqueue for the descent**, and reports excess as the `ParseDepthExceeded` chip diagnostic; exact `ParseContext` encoding/request-result variants stay pending. |
| **A — pipeline / T02 / T13 (H6–H9)** `[INT]` `[OWNER:T02,T13]` | Recommend the **quota-1 M1 baseline**: preserve the verified `/5` single-task `fail_selected` semantics (task always reaches `Failed`, committed `DiagnosticId` else `DiagnosticId::NONE`), the dispatcher as the sole pre-worker `Ready→Running` writer, and **explicit empty-inflight / empty-proposal paths** with tests so no task is stranded. | **Historical proposal (rev 40) — superseded on 2026-10-05.** This recommendation proposed **deferring the multi-task batch H6 mechanism to a separate CDR**. **Rev 42 records that the user rejected that deferral on 2026-10-05 and selected instead to freeze a quota>1-capable bounded batch recovery mechanism in `/6`** (the selected mechanism is recorded in §A and §10). The deferral text is retained here **only as a historical proposal**; it is **no longer a live default**. The remaining separator still holds: **M1-required** choices (empty-inflight paths, dispatcher writer, quota-1 projection) are distinct from the selected **M1** batch recovery, while quota `>1` execution, fairness/backpressure, the stage set, limits, hash, and numeric error codes remain **T01 decisions** (§9A row A). |
| **E — T06 scope/symbol/type** `[OWNER:T06]` `[INT]` | `decl` = the **`Identifier` leaf** node; file-scope `Enter` emitted by a T06 task **only after the TU node is committed**, via a **deterministic `parse`→`symbol_type` stage scheduling**; namespace **derived** from `SymbolKind` (no stored field); `types.records` writer = **`ChipId`-keyed allowlist**; canonical `TypeId` reuse by a **bounded scan of the committed arena (lowest matching id, no cache/index)**. **Rev 42 selected (2026-10-05):** the **lowest-matching committed-arena scan / no-cache subdecision** (deterministic bounded scan of committed `types.records` returning the **lowest matching `TypeId`**, with **no hidden cache/index**). **Rev 43 selected (2026-10-05):** the File Enter trigger is a deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge **after** the committed TU, carrying the committed `NodeId`, with **no job-bootstrap**. **Still proposal/pending in this row:** the `Identifier`-leaf `decl`, the bootstrap wiring beyond the accepted stage edge, the namespace carrier, the `ChipId`-keyed allowlist, and the exact row set/seed; **T05's committed-TU carrier is an upstream pending dependency and the accepted stage-edge direction is not a TU schema signoff**. | **Explicit upstream dependency:** the file-scope `Enter` needs the committed T05 `TranslationUnit`; the bootstrap **wiring beyond the accepted rev-43 stage edge** needs **T01 bootstrap acceptance** (the trigger direction itself is accepted). Exact row set/seed and field encodings remain T06 `/6`. **T05 upstream dependency acknowledged** for the selected scan and File-Enter subdecisions. **Rev 47 delegated candidate default (2026-10-05) — selected under user-delegated integration default, not a T06/T05 owner or T01 `[INT]` signoff:** the closed `SymbolKind`→namespace mapping, the miss-not-conflict rule, and the deterministic TY03 active-scope-chain lookup (`(source,start,end,NodeId)` ordering, greatest ≤ query point, same-scope tie higher `SymbolId`, else innermost active scope) are selected; the T05 `NodeKind`/token-range dependency and exact event encoding/lifecycle/allowlist rows stay co-freeze. |
| **F — T07 checked facts / conversions** `[OWNER:T07,T06,T09]` `[INT]` | No `FunctionContextId` (**requires a T07 package amendment**); `SemRecord` **is** the committed `CheckedNode` plus an **explicit committed typed T09 consumption link**; the full future `(NodeKind, role, op)` conversion matrix remains a **T06/T07/T09 co-freeze proposal** (**rev 46: only the M1-minimal conversion scope is selected now; the full matrix incl. `FloatToFloat`/pointer-qualifier is deferred, not silently frozen**); `EffectMask` **empty-only** for M1. **Rev 42 selected (2026-10-05):** **no `FunctionContextId`**; `SemRecord` = the **committed materialization of `CheckedNode`, one per `NodeId`**, with an **explicit committed typed T09 link**. **Rev 46 selected (2026-10-05) — subdecisions only:** `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` with the M1 fixture producing `NonLvalue`; M1 allows only `EffectMask(0)` (nonzero typed unsupported/diagnostic; effect bit classes reserved/unassigned); VF06 `TypedAstInvariant` **is in M1** (after committed T07 `SemRecord`s, before T09 lowering, checking M1 typed-fact/required-conversion completeness); the **M1-minimal conversion scope only** is selected — freeze only M1-fixture-needed conversion behavior incl. identity/no-conversion, with integer promotions, float conversions (incl. `FloatToFloat`), pointer qualifier, and other non-M1 conversions **explicitly unsupported/deferred** to a later append/contract revision. **Still proposal/open in this row:** the full future conversion `(NodeKind, role, op)` matrix, the exact VF06 registration (task kind/stage/phase/interface), and the exact `ConversionOp`/`ConversionRole` closed variant list/pairings/role→T09-chip mapping/numeric encodings. | **Open:** the **exact VF06 registration** (task kind/stage/phase/interface) is **T01/T13 co-freeze/open** even though VF06 `TypedAstInvariant` is now **in M1**; `ValueCategory`/`EffectMask` are selected (rev 46); the exact `ConversionOp`/role→T09-chip mapping and all conversion/effect numeric encodings stay `/6`. **T05 upstream dependency acknowledged** for the selected carrier subdecision; the full conversion matrix and diagnostics/tests remain pending. |
| **H — T09 IR op table** `[OWNER:T09]` `[INT]` `[OWNER:T13]` | Adopt the proposed `Constant`/`Add`/`Return` op-table rows (arity/immediate/result/terminator as in §H3), choose the **shorter** id of each unresolved alias pair (`ir.op-immediate-type`, `ir.terminator-missing`), and make `Constant` validation check **immediate-before-result**; keep the split **T08 computes folded `int5` / T09 emits IR `Constant`**. | **Rev 49 (user, 2026-10-05; critical H11/T09/T01 direction):** the `TerminatorMissing` trigger **reuses the committed IR28 `FunctionEnd` terminal result** (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic function-completion fact, checked by a **T01-owned typed phase-2b commit-apply validation hook**; **no** new marker family/store/arena/tag/encoder/`ResultValue` (the earlier `CompletedFunction` new-marker direction is **superseded for the operative direction**, preserved as history); **not a frozen `/6` hook/schema, no code**; exact hook contract/hash impact (likely no new record family; rule/hook hashing still a T01 decision)/result typing/commit ordering/T09-T01 co-freeze **open**; overall H stays **pending**. VF fixture amendments (incl. the batch/terminator fixtures) are **T13** work. All IR rules stay **prospective `/6`**, not hashed. **Rev 47 delegated candidate default (2026-10-05) — selected under user-delegated integration default, not a T09 owner or T01 `[INT]` signoff:** the shorter aliases `ir.op-immediate-type` and `ir.terminator-missing` are selected, `Constant` validates the immediate before the result when both are missing, and the target type equals the result `ValueRecord.ty`; all prospective `/6`, hashes/acceptance pending. |
| **I — cross-document / T01** `[INT]` `[DOC]` | Serialize the shared schema **only after the owners accept their shapes**; reconcile the **`M1AppendSchema` vs `COMPILER_SFL_MANIFEST.md` §4 hash-source contradiction**; keep **explicit numeric wire tags separate from `RecordFamily` ordinals** (dual inventory); add **count/self-consistency tests** (`StoreSchema::foundation() ⊕ M1AppendSchema == try_new(...).schema`). **Rev 48 read-only audit:** the `/5` code fact is **24** `RECORD_KINDS` / **24** `RecordRef` variants (tags 0–23), `RecordFamily` **absent**; the 19/27/27 draft counts are **proposed/unverified** (future `/6` count not claimed equal to 24); the hash-scope conflict is **real and unresolved** with three options (hash `M1AppendSchema`; don't hash + correct manifest; explicit two-tier hash-seed-vs-runtime-declarations) **unselected/unrecommended**; `/6` must include **dual-inventory encoding**, **numeric values in the hash**, and the **self-consistency mechanism**. **Rev 50 (`[USER]`, 2026-10-05):** the **two-tier hash scope is accepted** — `foundation + M1AppendSchema` = the **frozen `/6` seed hashed**; **post-seed runtime `declare()` extensions excluded** and captured/validated via runtime snapshot/schema mechanisms; at `/6` integration T01 must **atomically** update the manifest §4 wording, the `hash_excludes` semantic description/token (**post-seed runtime declarations**, not the M1 seed), and `FrozenSchema::encode`/contract code + freeze test, **preserving the `/5` hash/history**; **conceptual boundary settled, but no exact `/6` values/counts, no freeze, no code, no `/5` change**; T01 still co-freezes the M1 seed values after the owners. | **Depends on every owner row above.** No `/6` hashed claim; no `/5` edit; the T01 §4 supersessions are catalogued for the `/6` inventory only. **Hash-scope conceptual choice now settled `[USER]` (rev 50); the exact numeric inventory, seed contents, and self-consistency/freeze-test implementation remain critical `[INT]`; no implementation authorization.** |

**Dependency ordering and gates (refer, do not duplicate).** The three separate
gates — **Gate 1** (T01 shared-type/schema freeze before any chip code; then Wave 1
chips only), **Gate 2** (Wave 2 needs real committed upstream artifacts, not
fixtures), and **Gate 3** (full M1 end-to-end requires a Linux
probe/toolchain/assembler/linker/sysroot/runner and is **Part B**, separate from the
Part A `/6` freeze) — are stated in **§9A.1**. This appendix adds only the ordering
that follows from the recommendations above: **owner drafting proceeds in
parallel**; **T01 serial integration and hashing wait for the owner shapes**
(§9A.3); **chip code only after the `/6` freeze** (Gate 1); **Wave 2 only after the
upstream artifacts** (Gate 2); and **Part B probe/toolchain/assembler/linker/sysroot/
runner is a separate gate** (Gate 3). These gates **already exist in §9A** and are
**not re-specified** here.

**Authority honesty (repeated for this appendix).** Every recommendation above is
**proposed** for the named `[OWNER:*]`/`[INT]` authority to **accept, amend, or
reject**; none is a sign-off, a freeze, an implementation authorization, or a new
user decision. The already-recorded user in-principle directions (A/B/C, H1
allocation, A-narrow/B, H6/H9) are **referenced, not modified**. **Rev 42
(2026-10-05):** the user **rejected the row-A deferral recommendation** and
selected the H6 mechanism recorded in §A/§10; the deferral text above is retained
**as a historical proposal only** and is **no longer a live default**. The **row-D
tradeoff** is likewise corrected: the CT07 carrier question (**no new CT07
committed carrier/family**) and the sibling policy (**await-all**) are settled
2026-10-05 subdecisions, no longer live open choices. The user's 2026-10-05
acceptances are **only the named partial subdecisions** and are now **marked in
the affected rows above**: the **D** subdecisions (CT07 invariant + await-all), the
**F** carrier subdecisions (no `FunctionContextId`; `SemRecord` = committed
`CheckedNode`, one per `NodeId`, with a committed typed T09 link), and the **E**
subdecision (lowest-matching committed-arena scan, no cache/index) — recorded in
§9C/§10/§12 rev 42. **Rev 43 (2026-10-05):** the **row-D** tradeoff is further
corrected — the exact `ContinuationRecord` ordered fields/shapes (no `awaited`), the
formal `/6` T01 §4 supersession (no `/5` edit), and the exact `OwnBatch` pre-apply
validation are settled 2026-10-05 subdecisions; the **row-E** File Enter
deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge (after
the committed TU; no job-bootstrap) is also settled, while T05's committed-TU
carrier is a **pending upstream dependency**. **Rev 44 (2026-10-05):** the **row-C**
tradeoff is further corrected — the `required_kind` per-use constant-expression
meaning (distinct from `candidate_type`, no duplicate implicit target type), the
`legality` result-payload placement (no extra committed family), and the
`ArtifactRecord { kind, source, bytes, raw_offsets }` shape + total eight-`ArtifactKind`
map rule (M1 scope only single-source `Normalized`) are settled subdecisions, and the
companion **G** subdecision settles the `max_const_bits` origin (hashed `Limits`
value; M1 cap/default 128; config rejects > 128; explicit task-input projection).
**Rev 45 (2026-10-05):** the **row-C** tradeoff is further corrected — the exact
ordered `LiteralRecord` fields, the M1 `LiteralKind`/`LiteralSuffix`/radix scope, the
symbolic `Lx08CandidateType` (M1 literals 2/3 = `Int`, no bit width), the
`RequiredKind` M1 enum (`IntegerConstantExpression`) and `ConstLegality` values
(`Legal`/`NotConstantExpression`/`Unsupported`), and the mandatory artifact-map
invariants are settled subdecisions. **Rev 46 (2026-10-05):** the **row-F** tradeoff
is further corrected — the `ValueCategory` discriminants (M1 fixture = `NonLvalue`),
the M1-only `EffectMask(0)` rule (nonzero typed unsupported/diagnostic; bit classes
reserved/unassigned), the VF06 `TypedAstInvariant` **in M1** (after committed T07
`SemRecord`s, before T09 lowering), and the **M1-minimal conversion scope** (non-M1
conversions explicitly deferred) are settled subdecisions; the **exact VF06
registration** (task kind/stage/phase/interface) and all conversion/effect numeric
encodings remain open. **Rev 47 (2026-10-05):** under the user's explicit
delegation, the **M1 integration agent selected four candidate defaults** for the
remaining open items (optional-`ArtifactKind` map policy, T05 parse depth, T06
namespace/symbol lookup, T09 noncritical rule defaults); these are **selected under
user-delegated integration default, not an `[OWNER:*]`/`[INT]` sign-off**, are
recorded in the rows above and separately in §9C/§10, and leave every **overall**
row B/C/D/E/H **pending** pending final `/6` co-freeze. Every **unselected** recommendation
and every **other** item
in those rows remains **proposal-only**, and **the overall rows C–I remain
pending**; the remaining rows above are proposal-only in full. The `RecordRef`
numeric tag values and `RecordFamily` ordinals remain **separate inventories** (the
rev-43/44/45 acceptances imply **no** exact tag values), the **complete
`Lx08CandidateType` member set/numeric encodings**, the optional-kind artifact-map
rule, source-versus-payload equality, the exact artifact error classification, the
`ConstLegality`/`RequiredKind` numeric codes, the artifact-map boundary formula, the
`raw_offsets` invariants / enum numeric codes stay open, and the `ParseContext`
vocabulary final encoding stays open. **Rev 48 (2026-10-05):** a **read-only T01
audit** (docs-only) recorded the `/5` code facts (**24** `RECORD_KINDS` / **24**
`RecordRef` variants, tags 0–23; `RecordFamily` **absent**), marked the CDR's
**19/27/27 draft counts proposed/unverified**, and confirmed the `M1AppendSchema`
**hash-scope conflict** as **real and unresolved** (three options **unselected/
unrecommended**); it supplies **no sign-off** and **no** implementation
authorization. **Rev 49 (2026-10-05):** the user recorded the **critical
H11/T09/T01 direction** — reuse the committed IR28 `FunctionEnd` terminal result
(`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic
function-completion fact, checked by a **T01-owned typed phase-2b commit-apply
validation hook**, with **no** new marker family/store/arena/tag/encoder/
`ResultValue` (the earlier new-marker direction is **superseded for the operative
direction**, preserved as history; not a frozen `/6` hook/schema, no code; exact
hook contract/hash impact (likely no new record family; rule/hook hashing still a
T01 decision)/result typing/commit ordering/T09-T01 co-freeze open); and a
**read-only H9 audit** recorded **unresolved blockers** (quota>1 pipeline
unimplemented; dispatcher `Ready→Running`/`in_flight`-commit relationship and
`in_flight` clear ownership/order open; no-`Running`/residual proof for all paths;
`max_dispatches_per_tick`-vs-`max_inflight_per_tick` conflict deferred to `/6`;
fan-out fixtures/VF02/VF03/VF04/VF13 pending) — the audit supplies **no sign-off**
and changes neither the selected H6 mechanism nor the H9 removal direction. The
overall **rows C–I**, including **H**, remain **pending**. **Rev 50 (2026-10-05):**
the user's **critical hash-scope decision** (`[USER]`) accepts the **two-tier
model** — `foundation + M1AppendSchema` = the **frozen `/6` seed participating in
the `/6` contract hash**; **post-seed runtime `declare()` extensions stay
excluded** and are captured/validated via **runtime snapshot/schema mechanisms**;
at `/6` integration T01 must **atomically** update the manifest §4 wording, the
`hash_excludes` semantic description/token (**post-seed runtime declarations**, not
the M1 seed), and `FrozenSchema::encode`/contract code + freeze test, preserving
the `/5` hash/history. This is a **`[USER]` conceptual-scope record only** — it
accepts **no exact `M1AppendSchema` contents/counts, freezes nothing, changes no
`/5` and no code**, and leaves the numeric inventory/self-consistency-test
implementation to **T01 co-freeze after the owners**; **row I and the overall rows
C–I remain `PENDING`**. **Rev 51 (2026-10-05):** under the user's **delegation** for
non-critical decisions, the **M1 integration agent** selected a **candidate
default** (not an owner/T01 `[INT]` signoff, not a freeze) making
`max_inflight_per_tick`/quota the **sole per-tick dispatch-count bound** and
**dropping** the redundant `max_dispatches_per_tick` from the candidate limit
inventory and validation (grounded in the rev-35/38 H9 direction and the rev-49
audit duplication finding); exact names/defaults/codes remain **T01/T02 `/6`
co-freeze** and the T02/T13 tests remain **pending**. It changes no H6/H9 direction
and resolves no dispatcher-`Ready→Running`/`in_flight` atomic-boundary or
clear-order item; **every overall row stays `PENDING`**, `/5` current, no code.
**Rev 53 (2026-10-05):** under the user's explicit delegation for non-critical
decisions, the **M1 integration agent** selected a **candidate default** for the M1
exercised literal subset — the closed one-member `Lx08CandidateType { Int }` M1
vocabulary, M1 literals `2`/`3` as target-independent `Int` with no bit width, **no
silent `Int` defaulting** for forms outside the exercised M1 subset (those are
explicitly unsupported/deferred), and **append-only** future categories. This is
**selected under user-delegated integration default — not an owner/T04 or T01 `[INT]`
signoff, not a `/6` freeze, not the complete future C candidate vocabulary, no numeric
tags**; the complete `Lx08CandidateType` member set/numeric encodings and all
public/shared schema remain open. Every overall row (including C–I) stays `PENDING`;
`/5` current, no code.
**Rev 54 (2026-10-05):** docs-only **read-only-audit + historical-supersession
cleanup** — the §G2 signed-range/`i128`/enforcement rev-21 wording is marked
**historical/superseded — not selected** (only the rev-44 origin/cap/projection
retained), the rev-49 H9 duplicate-limit conflict is marked **historical/superseded
by rev-51** (dispatch/clear-order open), and the open **T04 reciprocal-link /
T05-T06 TU-edge / T13 no-residual / vertical PP-08** findings are recorded. It makes
**no architecture-critical choice**, is **not an owner/T01 `[INT]` signoff**, **not a
`/6` freeze**, and authorizes no code; every overall row stays `PENDING`. `/5`
current.

---

## 9C. Proposed per-decision response matrix (rev 41; rev-43/44/45 per-subdecision updates below; rev-47 delegated candidate-default note below; rev-49 H11/H9 note below; rev-50 hash-scope note below; rev-51 sole-per-tick-bound note below; rev-52 cross-owner audit/reconciliation note below; rev-53 Lx08CandidateType M1-vocabulary note below; rev-54 read-only audit/supersession note below)

**This section is a procedural response form — not accepted schema, not a freeze,
not a chip authorization, and not a new decision request.** It adds **no** technical
content: the recommendation, tradeoff, and open dependency for each bundle live in
**§9B** (recommendation appendix) and the detailed owner rows in **§C–§I** (source
of truth) and **§13** (owner amendments); the authority assignment and work items
live in **§9A**. §9C only lets each named authority respond **per decision bundle**
(`Accept recommendation` / `Amend (state replacement)` / `Reject (reason)` /
`Defer (dependency/date)`) instead of accepting or rejecting the whole document.
**No new user decision is requested by this form.** Filling a row is a sign-off by
that authority; leaving it untouched changes nothing. `/5` remains current, M1
remains DRAFT, ADR-0002 remains PROPOSED, and every status/signature below starts
and stays **PENDING** until the named human authority fills it.

**Bundles and citation.** Each row cites the recommendations in **§9B** and the
detailed technical rows in **§C–§I**; the matrix deliberately does **not** restate
them. `Accept recommendation` means the §9B recommendation for that bundle minus
any stated amendment; `Amend` requires the **replacement** text to be written in
the row; `Reject` requires a **reason**; `Defer` requires the **dependency or date**
that unblocks it.

**Per-subdecision acceptances (rev 42, extended at rev 43, rev 44, rev 45, and rev 46).** A bundle may be accepted **partially, at
the subdecision level**, while its **overall status stays `PENDING`**. Each such
acceptance is recorded as a role-labelled sub-entry with its date, naming the exact
**selected subdecision** and the **remaining open items**. Subdecisions not
selected remain **proposal-only**. A bundle whose row shows only partial
acceptances still has its `Status / signature` cell as **`PENDING`** and is **not**
a wholesale closure of row A/C/D/E/F/G or any other row. Only the roles named for a
**selected** subdecision carry an acceptance sub-entry; a role that co-freezes a
bundle's other items (for example `[OWNER:T04]` for the exact `LiteralRecord`
fields/enum variants) is **not** shown as having accepted those unaddressed items.

**Per-authority signature — every listed authority signs for itself.** Where a
bundle names more than one authority (for example `[OWNER:T03]`, `[OWNER:T04]`,
`[OWNER:T07]`, `[OWNER:T08]` + T01 `[INT]`), **each named authority must respond
and sign independently, for its own acceptance only.** No group response, no single
owner's response, and no T01 `[INT]` response represents, covers, or implies any
other listed authority. A row stays **`PENDING`** until **every** listed required
authority for that bundle — **and T01 `[INT]` where the row lists it** — has
separately signed (an `Amend` still leaves the row `PENDING` until the stated
replacement text is written and that authority signs). To keep the table from
inflating, use **one sub-entry per authority** in the `Response` and
`Status / signature` cells rather than duplicating the bundle row; duplicate the
row per authority only if a sub-entry becomes unreadable. Sub-entries are written
as **`Authority — disposition — date`** (for example `[OWNER:T03] — Accept
recommendation — 2026-10-06`), one line per authority, and a `Status / signature`
cell is `PENDING` unless **all** of its authorities have signed, at which point it
records the same per-authority entries.

| # | Decision bundle | Exact authority | Response (one sub-entry per authority) | Status / signature (one sub-entry per authority; `PENDING` until all signed) |
|---|---|---|---|---|
| **T01-global** | `/6` envelope, hash scope, source of truth, `RecordRef`-tag vs `RecordFamily`-ordinal inventory, `M1AppendSchema` self-consistency, and the **T01 §4 supersessions** (control-computes/dispatcher-applies; commit-path/backend writes; awaited-child `WaitSet` supersession) — see §9B row **I**, §9A row **I**, §3.3, §I2/§I4 | T01 integrator `[INT]` + user `[USER]` (hash scope, rev 50) | T01 `[INT]` — **ACCEPT (subdecision only), 2026-10-05 (rev 43):** the awaited-child `WaitSet` T01 §4 supersession is formally accepted as a `/6`-inventory amendment (**no `/5` edit**; T01 §5 unchanged); the `RecordRef` numeric wire tags and `RecordFamily` ordinals remain **separate inventories** and their exact numeric encodings, plus the `ParseContext` canonical encoding, remain **PENDING**. **Rev 48 read-only audit (not a response):** confirms the `/5` code fact (**24** `RECORD_KINDS` / **24** `RecordRef` variants, tags 0–23; `RecordFamily` **absent**), the **proposed/unverified** 19/27/27 draft counts, and the **real unresolved** `M1AppendSchema` hash-scope conflict with the three options **unselected/unrecommended**; no signature. **Rev 50 (`[USER]`, 2026-10-05; critical hash-scope decision):** the **user accepts the two-tier model** — `foundation + M1AppendSchema` = the **frozen `/6` seed participating in the `/6` contract hash**; **post-seed runtime `declare()` extensions excluded** and captured/validated via runtime snapshot/schema mechanisms; at `/6` integration T01 must **atomically** update the manifest §4 wording, the `hash_excludes` semantic description/token (**post-seed runtime declarations**, not the M1 seed), and `FrozenSchema::encode`/contract code + freeze test, preserving the **`/5` hash/history**. This record is the **user's own scope decision** (`[USER]`), **not** a T01 `[INT]` signature and **not** a row-I closure: exact `/6` values/counts, the seed contents, and the self-consistency/freeze-test implementation remain **T01 `[INT]` co-freeze after the owners**, the **numeric inventory/test implementation remain PENDING**, and no code/`/5` change is authorized. | **PENDING** (T01 `[INT]`; only the T01 §4 supersession subdecision accepted by T01, and the rev-50 hash-scope **conceptual** scope accepted by `[USER]`; encoders/tags/numeric seed/test still open; rev-48 audit adds no signoff) |
| **C/H1** | T03/T04/T08 literal/const handoff + `ArtifactRecord`/artifact-map + `lex.literals`/`ConstantRequest`/`ConstantResult` shapes — see §9B row **C**, §9A row **B**, §C, §F4 | `[OWNER:T03]`, `[OWNER:T04]`, `[OWNER:T07]`, `[OWNER:T08]` + T01 `[INT]` | `[OWNER:T03]` — **ACCEPT (subdecision only), 2026-10-05 (rev 44):** `ArtifactRecord { kind, source, bytes, raw_offsets }` + total eight-`ArtifactKind` map rule (map-mandatory `Normalized`/`Spliced`/`CommentFree`/`Preprocessed`; map-optional `Assembly`/`Object`/`Snapshot`/`Trace`), M1 scope only single-source `Normalized`; exact `raw_offsets` invariant/error mapping and enum numeric codes remain PENDING. **rev 45:** the mandatory-map invariants (`raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds require a valid source); the **optional-kind map rule, source-versus-payload equality, and exact artifact error classification/numeric codes remain PENDING**.<br>`[OWNER:T04]` — **ACCEPT (subdecision only), 2026-10-05 (rev 45):** the exact ordered `LiteralRecord` fields (`token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` big-endian magnitude, `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`; no `node`/`required_kind`) and the M1 enum/scope `LiteralKind {Integer, Character, String}` (only `Integer` produced) / `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced) / radix `{2,8,10,16}` (M1 decimal only); the **complete `Lx08CandidateType` member set/numeric encodings remain PENDING** (not invented here).<br>`[OWNER:T07]` — **ACCEPT (subdecision only), 2026-10-05 (rev 44):** `ConstantRequest.required_kind` is the per-use constant-expression requirement (e.g. M1 integer constant expression), distinct from lexical `candidate_type`; remaining const items PENDING. **rev 45:** `RequiredKind` M1 enum only `IntegerConstantExpression`; `ConstLegality` values `Legal`/`NotConstantExpression`/`Unsupported`; numeric encodings PENDING.<br>`[OWNER:T08]` — **ACCEPT (subdecision only), 2026-10-05 (rev 44):** `required_kind` per-use meaning; `ConstantResult.legality` is a result payload field with no extra committed family (exact `ConstLegality` variants PENDING); remaining items PENDING. **rev 45:** `ConstLegality` values `Legal`/`NotConstantExpression`/`Unsupported`; `RequiredKind` only `IntegerConstantExpression`; future C constant-expression purposes via appended variants/new rules (not lexical-candidate reuse); the symbolic `Lx08CandidateType` (M1 literals 2/3 = `Int`, no bit width); numeric encodings PENDING.<br>T01 `[INT]` — **ACCEPT (subdecision only), 2026-10-05 (rev 44):** the `required_kind`/`legality` meanings and the artifact shape/map-rule subdecisions; tags/enum codes/invariants PENDING. **rev 45:** the `LiteralRecord` ordered fields + enum-scope, the `RequiredKind`/`ConstLegality` values, and the mandatory-map invariants; the `Lx08CandidateType` full member set/numeric tags, optional-map policy, and error codes PENDING. | **PENDING** (each of T03/T04/T07/T08 + T01 signs separately; rev-44/45 partial subdecision acceptances only, no wholesale C closure) |
| **D** | T05 continuation/join: continuation fields, `WaitSet` `/6` sole-source supersession, `OwnBatch` commit resolution, CT07 carrier, sibling policy — see §9B row **D**, §9A row **D**, §D | `[OWNER:T05]`, `[OWNER:T02]`, T01 `[INT]` | `[OWNER:T05]` — **ACCEPT (subdecision only), 2026-10-05:** join is a commit-apply invariant with **no new CT07 committed carrier/family**; await-all children terminal before the parent decision; **further, rev 43:** the `ContinuationRecord` exact ordered fields/shapes (no `awaited`; committed-ID references only), the formal `/6` T01 §4 supersession, and the exact OwnBatch pre-apply validation (phase-1/phase-2b; whole-batch `CommitError` before mutation). **Still PENDING:** `RecordRef` tag values/`RecordFamily` ordinals (separate inventories), the `ParseContext` vocabulary, OwnBatch numerical codes/hash, parse request/result/test shapes.<br>`[OWNER:T02]` — **ACCEPT (subdecision only), 2026-10-05:** same CT07-invariant + await-all subdecisions; **rev 43:** same OwnBatch pre-apply acceptance; no wholesale D closure.<br>T01 `[INT]` — **ACCEPT (subdecision only), 2026-10-05:** same two rev-42 subdecisions; **rev 43:** the formal `/6` T01 §4 supersession (`WaitSet` sole awaited-child source; no `/5` edit) and the OwnBatch pre-apply acceptance; the T01 tags/encoder/error-number items remain PENDING; no wholesale D closure. | **PENDING overall** — partial subdecision acceptances recorded per authority; remaining D items unresolved (each of T05/T02 + T01 signs separately) |
| **A** | Pipeline scheduling (T05/T02 batch + H6/H9): `quota = 1` baseline, H6 bounded atomic recovery details, H9 removal/sole-writer boundary — see §9B row **A**, §9A row **A**, §A. **Includes the explicit H6 scope question:** whether the multi-task batch mechanism is M1 or deferred to a separate CDR; any **deferral requires explicit T01 acceptance**, and if the response would change the **user's** in-principle H6 direction it must **return to the user** — the form does not presume that outcome. | `[OWNER:T02]`, `[OWNER:T13]` + T01 `[INT]`; H6 user direction `[USER]` already | `[OWNER:T02]` — **ACCEPT (subdecision only), 2026-10-05:** H6 **scope** = preserve the user's in-principle direction and **freeze a quota>1-capable bounded batch recovery mechanism in `/6`** (rejecting the §9B row-A deferral to a later CDR); H6 **mechanism** = pre-dispatch errors before state mutation leave tasks `Ready`; semantic batch commit failure commits no semantic state; a deterministic bounded recovery mutation then processes the dispatched tasks once in dispatch order to `Failed`; no pre-reservation of N diagnostics; per-task diagnostic attempt with `DiagnosticId::NONE` when capacity is insufficient; a state guard prevents duplicate transition. **H9/other pipeline inventories remain PENDING.**<br>`[OWNER:T13]` — **ACCEPT (subdecision only), 2026-10-05:** same H6 scope + mechanism; **fixtures and H9 inventories remain PENDING**.<br>T01 `[INT]` — **ACCEPT (subdecision only), 2026-10-05:** same H6 scope + mechanism (`[INT]`); did **not** accept the deferral; H9 limits/stages/hash/error inventories remain PENDING.<br>`[USER]` — **DIRECTION CONFIRMED, 2026-10-05:** user preserves the in-principle direction and chose the `/6` freeze of the mechanism (supersedes the §9B row-A deferral recommendation). | **PENDING overall** — partial H6 subdecision acceptances only; H9/other pipeline items unresolved (each of T02/T13 + T01 signs separately) |
| **E** | T06 scope/symbol/type: `decl` leaf, file-`Enter` bootstrap dependency, namespace carrier, `types.records` allowlist, canonical-`TypeId` reuse — see §9B row **E**, §9A row **E**, §E | `[OWNER:T05]` (committed TU upstream), `[OWNER:T06]`, T01 `[INT]` | `[OWNER:T05]` — **PENDING (upstream dependency):** the committed T05 `TranslationUnit` upstream contract remains pending; the accepted stage-edge direction is **not** a TU schema signoff.<br>`[OWNER:T06]` — **ACCEPT (subdecision only), 2026-10-05:** deterministic bounded scan of committed `types.records` returning the **lowest matching `TypeId`**, with **no hidden cache/index**; **further, rev 43:** the File Enter trigger is a deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge after the committed TU, carrying the committed `NodeId`, no job-bootstrap. Bootstrap wiring and the rest of the E bundle remain PENDING.<br>T01 `[INT]` — **ACCEPT (subdecision only), 2026-10-05:** same lowest-id committed-scan acceptance; **rev 43:** same File Enter stage-edge acceptance; bootstrap remains PENDING. | **PENDING overall** — partial subdecision acceptance; bootstrap/T05-upstream/remaining E items unresolved (each of T05/T06 + T01 signs separately) |
| **F** | T07 checked facts + conversion plan: `FunctionContextId`, `SemRecord`↔`CheckedNode`, `ConversionOp`/role matrix, VF06 stage, `EffectMask` — see §9B row **F**, §9A row **F**, §F | `[OWNER:T06]`, `[OWNER:T07]`, `[OWNER:T09]`, `[OWNER:T13]`, T01 `[INT]` | `[OWNER:T06]` — **ACCEPT (subdecision only), 2026-10-05:** no `FunctionContextId`; `SemRecord` is the committed materialization of `CheckedNode`, one per `NodeId`, with an explicit committed typed link consumed by T09 (T05 upstream dependency acknowledged). **rev 46:** the **M1-minimal conversion scope** (`ConversionOp`/role lists/pairings/role→chip mapping/numeric encodings not invented; non-M1 conversions explicitly deferred); conversion encodings/etc. remain PENDING.<br>`[OWNER:T07]` — **ACCEPT (subdecision only), 2026-10-05:** same three carrier decisions. **rev 46:** `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` (M1 fixture = `NonLvalue`); M1 only `EffectMask(0)` (nonzero typed unsupported/diagnostic; bit classes reserved/unassigned); VF06 `TypedAstInvariant` in M1; M1-minimal conversion scope — all accepted subdecisions only, exact VF06 registration and conversion encodings PENDING.<br>`[OWNER:T09]` — **ACCEPT (subdecision only), 2026-10-05:** same three carrier decisions. **rev 46:** same M1-minimal conversion scope (T09 consumes only the frozen M1 behavior); conversion encodings PENDING.<br>`[OWNER:T13]` — **ACCEPT (subdecision only), 2026-10-05 (rev 46):** VF06 `TypedAstInvariant` **is in M1**, executing after committed T07 `SemRecord`s and before T09 lowering, checking M1 typed-fact/required-conversion completeness; the **exact registered task kind/stage/phase/interface remains T01/T13 co-freeze/open** and VF06/matrix verification fixtures remain PENDING.<br>T01 `[INT]` — **ACCEPT (subdecision only), 2026-10-05:** same three carrier decisions. **rev 46:** same `ValueCategory`/`EffectMask`/VF06-in-M1/M1-minimal-conversion-scope subdecisions; exact VF06 registration and conversion/effect encodings remain PENDING. | **PENDING overall** — partial carrier/effect/subdecision acceptances; exact VF06 registration, the full conversion matrix, conversion/effect encodings, and diagnostics/tests unresolved (each of T06/T07/T09/T13 + T01 signs separately) |
| **G** | T08 constants: `max_const_bits` bound/carrier/constraint, symbolic-vs-probe split, chip diagnostics — see §9B row **C** (const items), §9A row **G**, §G | `[OWNER:T08]`, T01 `[INT]` | `[OWNER:T08]` — **ACCEPT (subdecision only), 2026-10-05 (rev 44):** `max_const_bits` origin is a hashed `Limits` value; M1 cap/default 128; config rejects > 128; explicit task-input projection to the restricted T08 chip; exact wire field/diagnostic code/hash encoding and the formula/enforcement/symbolic-vs-probe split remain PENDING. **rev 45:** `RequiredKind` M1 enum only `IntegerConstantExpression`; `ConstLegality` values `Legal`/`NotConstantExpression`/`Unsupported`; future purposes via appended variants/new rules; numeric encodings PENDING.<br>T01 `[INT]` — **ACCEPT (subdecision only), 2026-10-05 (rev 44):** same `max_const_bits` origin/cap/projection subdecision; remaining G items PENDING. **rev 45:** same `RequiredKind`/`ConstLegality` value selections; numeric encodings PENDING. **rev 54:** the rev-21 signed-range/`i128` formula, `ConstRecord.value: i128` carrier, and chip-level enforcement wording are marked **historical/superseded — not selected**; only the rev-44 origin/cap/projection remains accepted (exact formula/carrier/enforcement open). | **PENDING** (T08 + T01 sign separately; rev-44/45 partial subdecision acceptances only, no wholesale G closure) |
| **H** | T09 IR op table/terminator: repeated rule ids, `Constant` immediate/result, folded `int5` split, **function-completion fact/trigger** — see §9B row **H**, §9A row **H**, §H | `[OWNER:T09]`, `[OWNER:T08]`, `[OWNER:T13]`, T01 `[INT]` | `[OWNER:T09]` — **DIRECTION RECORDED (user, 2026-10-05, rev 49; critical H11/T09/T01):** the `TerminatorMissing` trigger **reuses the committed IR28 `FunctionEnd` terminal result** (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic function-completion fact, checked by a **T01-owned typed phase-2b commit-apply validation hook**; **no** new marker record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder or `ResultValue` variant; the earlier new-marker (`CompletedFunction`) direction is **superseded for the operative direction** (preserved as history). **Not a frozen `/6` hook/schema, no code;** exact hook contract, hash impact (likely no new record family; rule/hook hashing still a T01 decision), result typing/commit ordering, and T09/T01 co-freeze remain **PENDING**.<br>`[OWNER:T08]` — <br>`[OWNER:T13]` — <br>T01 `[INT]` — | **PENDING** (each of T09/T08/T13 + T01 signs separately; rev-49 records the user's selected H11 direction only, overall H stays pending) |

**Response protocol.** Only the **named human authority** for a bundle may fill its
`Response` and `Status / signature` cells, and **each listed authority fills its
own sub-entry**: one authority's response is only that authority's own acceptance
and **does not** represent, cover, or stand in for any other listed authority (no
group response, no delegation, no single-owner proxy). A reviewer, audit, subagent,
ledger, or model analysis — including this CDR, §9A, §9B, and §9C themselves —
**never counts** as an owner or T01 integrator response. A bundle row remains
**`PENDING`** until **every** listed required authority for that bundle (and T01
`[INT]` where the row lists it) has **separately** signed. T01 records an accepted
shape in `/6` **only after** the owning `[OWNER:*]` bundle(s) and the T01 `[INT]`
bundle are **signed**, and T01's own `[INT]` sign-off is itself an independent
sub-entry that no owner sign-off supplies; until then **`/5` is current and no
code** may implement or depend on the shape. `Amend` rows are not accepted bundles
until the replacement text is written
and the authority signs. A bundle sitting in `Defer` blocks no other bundle (the
tracks in §9A.2 are parallel); it only blocks T01 consolidation/hashing of the
shapes it covers. The H6 scope question in bundle **A** stays with the user if the
response would change the user's accepted-in-principle direction; **on
2026-10-05 the user answered it — rejecting deferral and selecting the `/6`
freeze of the mechanism (rev 42)**. All rows **C–I remain PENDING overall** and
this form implies **no** wholesale sign-off; the rev-42 per-subdecision acceptances
(A H6 scope+mechanism, D CT07-invariant+await-all, F carrier decisions, E
lowest-id scan), the **rev-43** D subdecisions (the exact `ContinuationRecord`
ordered fields/shapes, the formal `/6` T01 §4 supersession, and the exact `OwnBatch`
pre-apply validation) plus the **rev-43** E File Enter deterministic
`parse.TranslationUnit -> symbol_type.scope-enter` stage edge, the **rev-44**
C/G subdecisions (the `required_kind` per-use meaning + `legality` result-payload
placement; the `ArtifactRecord` shape + total eight-`ArtifactKind` map rule with M1
scope only single-source `Normalized`; and the `max_const_bits` hashed-`Limits`
origin / cap 128 / config `> 128` rejection / task-input projection), and the
**rev-45** C/G subdecisions (the exact `LiteralRecord` ordered fields + M1
`LiteralKind`/`LiteralSuffix`/radix scope + symbolic `Lx08CandidateType`;
`RequiredKind` only `IntegerConstantExpression`; `ConstLegality` values
`Legal`/`NotConstantExpression`/`Unsupported`; and the mandatory artifact-map
invariants), and the **rev-46** F subdecisions (the `ValueCategory` discriminants
with M1 fixture = `NonLvalue`; the M1-only `EffectMask(0)` rule with nonzero typed
unsupported/diagnostic and reserved/unassigned bit classes; VF06
`TypedAstInvariant` **in M1** after committed T07 `SemRecord`s and before T09
lowering; and the **M1-minimal conversion scope** with non-M1 conversions explicitly
deferred) are recorded
above and do **not** close row A/C/D/E/F/G or any other row.

**Rev 47 — delegated candidate defaults (separate note; NOT role signoffs).** This
is a **clearly separated** record, added 2026-10-05. Under the user's explicit
delegation ("for non-critical decisions adopt the recommended choice directly; ask
only for critical decisions"), the **M1 integration agent** selected four low-risk
**candidate defaults** — the **optional-`ArtifactKind` map policy** (bundle **C/H1**:
map-optional kinds have empty `raw_offsets`; `source` stays `Option<SourceId>` and
must be valid when `Some`; mandatory-map kinds require a valid `source` plus the
accepted rev-45 invariants; **no** source-payload-equals-bytes requirement, any
exact source-provenance/equality rule deferred; exact numeric error codes open),
the **T05 parse-depth default** (bundle **D**: reuse `limits.max_task_depth`, count
parser continuation/child frames only, detect before any child enqueue for the
descent, excess = `ParseDepthExceeded` chip diagnostic; exact `ParseContext`
encoding/request-result variants pending), the **T06 namespace/symbol-lookup
default** (bundle **E**: closed `SymbolKind`→namespace mapping with no namespace
field, wrong-namespace lookup is a miss not a conflict, deterministic TY03
active-scope-chain lookup; T05 `NodeKind`/token-range dependency and exact event
encoding/lifecycle/allowlist rows still co-freeze), and the **T09 noncritical rule
defaults** (bundle **H**: shorter aliases `ir.op-immediate-type` and
`ir.terminator-missing`, `Constant` validates immediate before result when both are
missing, target type equals the result `ValueRecord.ty`; all prospective `/6`, and
the `CompletedFunction` marker family stays unresolved with no family invented).
These entries are **selected under user-delegated integration default** and are
**explicitly NOT role signoffs**: they are **not** evidence of T03/T05/T06/T09
owner acceptance (unless the user separately already accepted that exact
subdecision), **not** T01 `[INT]` acceptance, and **not** a `/6` freeze. The
`Response` and `Status / signature` cells above are **not** populated by this note
and **no bundle status changes**: bundles **C/H1**, **D**, **E**, and **H** remain
**`PENDING` overall**, and all public/shared schema awaits owner/T01 co-freeze. No
conflicting user-selected rev 42–46 decision is overridden.

**Rev 48 — read-only T01 audit findings (separate note; NOT a role signoff).**
This is a **clearly separated** record, added 2026-10-05. A **read-only** audit
(docs-only; no code, T01, manifest, or other-doc edit; no hash-scope option
selected; no `/6` freeze) recorded, as **`/5` facts**, that the frozen contract is
`t01-c01-c06/5` (`6187…63bb5`), `compiler/src/contract.rs` encodes **24**
`RECORD_KINDS`, `compiler/src/ids.rs` `RecordRef` has **24** variants (tags
**0–23**), and `RecordFamily` is **absent** from the code; it marked the CDR's
**19 draft families / 27 `RecordRef` variants / 27 ordinals** as
**proposed/unverified** (the future `/6` count is **not** claimed to equal the
current 24), and confirmed the `M1AppendSchema` **hash-scope conflict** between
`CONTRACT_VERSION`/`contract.rs`/`README.md` (`hash_excludes=…
group-declared-store-fields…`) and `COMPILER_SFL_MANIFEST.md` §4 as **real and
unresolved**, with the three options (**hash `M1AppendSchema`** + update
exclusion token/freeze assertion/manifest; **do not hash** + correct manifest;
**explicit two-tier hash-seed-vs-runtime-declarations**) left **unselected and
unrecommended**. The `/6` must further specify **dual-inventory encoding** (wire
tags separate from ordinals — the rev-43 accepted structural subdecision),
**numeric-value inclusion in the hash**, and the **schema self-consistency
mechanism**, with actual counts/values frozen only after owner-shape acceptance.
This is a **critical `/6` hash-scope choice with no implementation
authorization**. The `Response` and `Status / signature` cells are **not**
populated by this note, **no bundle status changes** (rows C–I remain
**`PENDING` overall**), no owner acceptance or signature is added, and no
conflicting user-selected rev 42–47 decision is overridden.

**Rev 49 — user critical H11/T09/T01 direction + read-only H9 audit findings
(separate note; the audit is NOT a role signoff).** Added 2026-10-05, docs-only; no
code/other-doc/test edit; no `/6` freeze. **(a) H11/T09/T01:** the **user** recorded
an explicit critical decision — the `TerminatorMissing` trigger **reuses the committed
terminal result of the IR28 `FunctionEnd` task** (`TaskState::Completed(ResultId)`,
task kind `FunctionEnd`) as the **deterministic function-completion fact**, checked by
a **T01-owned typed phase-2b commit-apply validation hook** (entry-block
termination); **no** new marker record family/ID/arena/`RecordRef` tag/`RecordFamily`
ordinal/snapshot encoder or new `ResultValue` variant. The earlier new-marker
(`CompletedFunction`) direction is **superseded for the operative direction** but
**preserved as history**. This is **not a frozen `/6` hook/schema and authorizes no
code**; the **exact hook contract, its hash impact** (likely **no** new record
family, but **rule/hook hashing remains a T01 decision**), **result
typing/commit ordering**, and **T09/T01 co-freeze** remain **open**, and **overall H
stays PENDING**. **(b) H9 read-only audit (unresolved blockers; NOT a signature; no
hash-scope option selected):** the `/5` code proves **only** the quota=1 single-task
`fail_selected`, so the **proposed quota>1 pipeline is not implemented**; H9 still
needs the **exact dispatcher `Ready→Running`/`in_flight`-population relationship to
the single ordered atomic commit**, the **exact `in_flight` clear ownership/order
relative to the bounded H6 recovery**, and a **no-`Running`/no-residual proof for all
success/error/empty-proposal paths**; the **`max_dispatches_per_tick`-vs-
`max_inflight_per_tick` internal proposed-limit conflict** was recorded to be
**resolved in `/6`** (not decided then) — **now historical/superseded**: the
**rev-51** integration-selected default **drops** `max_dispatches_per_tick` and keeps
`max_inflight_per_tick`/quota as the sole per-tick bound (see the rev-51 record
below), so this conflict is **no longer open** while the dispatch/clear-order items
remain open; the **H6/H9 fan-out fixtures** and **T13 VF02/VF03/VF04/VF13**
remain **pending**. The H9 no-residual guarantee is **conditional on the H6 recovery
implementation**, the selected **H6 mechanism and the H9 removal direction are
unchanged**, and the **quota=1 M1 baseline stays separated** from the quota>1
optimization; no freeze/code; exact stages/version carrier/errors/hashes remain
pending. The `Response`/`Status / signature` cells are not populated by this note
(except the recorded H11 direction sub-entry), **no bundle status changes** (rows
C–I remain **`PENDING` overall**), and no conflicting user-selected rev 42–48
decision is overridden.

**Rev 50 — user critical HASH-SCOPE decision (separate note; `[USER]` scope
record, NOT a T01 `[INT]` signature).** Added 2026-10-05, docs-only; no
`/5`/T01/manifest/other-doc/code/test edit; no `/6` freeze. The **user** records an
explicit **critical decision** accepting the **two-tier hash-scope model**:
`StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed participating
in the `/6` contract hash**, while **post-seed runtime `StoreSchema::declare()`
extensions stay excluded** from the frozen hash and are **captured/validated via
runtime snapshot/schema mechanisms**. This **settles the conceptual future `/6`
boundary** (the three hash-scope options are no longer open at the conceptual
level), but it **accepts no exact `M1AppendSchema` contents/counts, freezes
nothing, changes no `/5`, and authorizes no code**; at `/6` integration T01 must
**atomically** update the `COMPILER_SFL_MANIFEST.md` §4 wording, the `hash_excludes`
semantic description/token (scoped to **post-seed runtime declarations**, not the
frozen M1 seed), and `FrozenSchema::encode`/contract code plus the freeze test,
**preserving the frozen `/5` hash and history**; the **numeric inventory and
self-consistency/freeze-test implementation remain pending**, and **T01 must still
co-freeze the M1 seed values after the owners**. The `Response`/`Status / signature`
cells are **not** populated by this note (only the `[USER]` scope is recorded in the
T01-global row's authority column), **no bundle status changes** (rows C–I,
including I, remain **`PENDING` overall**), and no conflicting user-selected
rev 42–49 decision is overridden; `/5` remains current, M1 remains DRAFT, ADR-0002
remains PROPOSED.

**Rev 51 — integration-selected candidate default under explicit user delegation
(separate note; NOT a role signoff).** Added 2026-10-05, docs-only; no
`/5`/T01/manifest/other-doc/code/test edit; no `/6` freeze. Under the user's
explicit **delegation** ("for non-critical decisions adopt the recommended choice
directly; ask only for critical decisions"), the **M1 integration agent** selected —
**under user-delegated integration default, not as an owner or T01 `[INT]` signoff**
— that **`max_inflight_per_tick`/quota is the sole per-tick dispatch-count bound**
and that the redundant **`max_dispatches_per_tick` is dropped** from the candidate
limit inventory and validation, grounded in the rev-35/38 H9 direction and the
rev-49 audit finding that the two limits duplicated each other. It selects **no**
dispatcher-`Ready→Running`/`in_flight` atomic-boundary or clear-order item, changes
neither the H6 mechanism nor the H9 removal direction, and accepts **no** exact
names/defaults/codes (those remain **T01/T02 `/6` co-freeze**, with T02/T13 tests
**pending**). The `Response`/`Status / signature` cells are **not** populated by this
note and **no bundle status changes** (rows C–I remain **`PENDING` overall**); no
conflicting user-selected rev 42–50 decision is overridden; `/5` remains current, M1
remains DRAFT, ADR-0002 remains PROPOSED, and no code is authorized.

**Rev 52 — cross-owner audit/reconciliation (separate note; read-only, NOT a role
signoff).** Added 2026-10-05, docs-only; no `/5`/T01/manifest/other-doc/code/test
edit; no `/6` freeze. A **read-only** reconciliation mirrors the CDR's
already-selected items against the now-completed owner packages and records: **(A)**
the package state mirrors the selected items — **T02 rev 30+31**, **T03/T04** (T04
rev 46 incl. the `LiteralRecord.token` candidate and the same-revision correction
to the typed `RecordLink` mechanism, with no `OwnBatch` confusion for record links),
**T05 `ParseContext` candidate**, **T06**, **T07 sem→const correction**, **T08**
exact candidate shape, **T09 H11**, **T13**, **[M1 vertical
acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md) rev 27** and **[M1 target
acceptance](M1_TARGET_ACCEPTANCE.md) rev 29**, and **ADR-0002 Revision 17** — with
**no owner package called a `/6` sign-off**; **(B)** the M1 proposal/target/vertical
docs now align on H11, the hash boundary, and the Part A-vs-B split, and the
proposal is being advanced in parallel to **rev 38** (expected resulting file),
with `/5` preserved and **no `/6` freeze**; **(C)** the T01 audit still leaves `/6`
**not freeze-ready** (exact record/tag/family inventories, field bodies/ref
encoding, task/request/result typing, writer manifests, TU carrier/`ParseContext`
final encoding, `SourceId`/token provenance/raw-offset mapping and
`Lx08CandidateType`, `SemRecord`/VF06, const overflow/projection, the IR hook
contract, the scheduler H6/H9 atomic boundaries/in_flight clear order/no-residual
fixtures, and the snapshot/canonical-encoder/hash tests), with **no new
user-critical decision implied**; **(D)** the **rev-51 `max_inflight_per_tick`-only**
candidate default and the **rev-50 hash-scope** decision remain **selected**, with
exact implementation/sign-off **pending**, and **rows C–I remain PENDING overall**;
and **(E)** the CDR is an **accepted-decision ledger/work queue, not an executable
schema** — the **next integration stage after the owners produce exact artifacts is
T01's serial `/6` seed assembly**, self-consistency-checked against the **frozen
two-tier hash model**, and **no chip coding precedes the freeze**, while **Part B
remains independent**. The `Response`/`Status / signature` cells are **not**
populated by this note and **no bundle status changes** (rows C–I remain
**`PENDING` overall**); no conflicting user-selected rev 42–51 decision is
overridden; `/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED, and
no code is authorized.

**Rev 53 — integration-agent selected candidate default under explicit user delegation
(the `Lx08CandidateType` M1 vocabulary; separate note; NOT a role signoff).** Added
2026-10-05, docs-only; no `/5`/T01/manifest/other-doc/code/test edit; no `/6` freeze.
Under the user's explicit **delegation** ("for non-critical decisions adopt the
recommended choice directly; ask only for critical decisions"), grounded in a
**read-only review of the exercised M1 integer literal subset**, the **M1 integration
agent** selected — **under user-delegated integration default, not as an owner/T04 or
T01 `[INT]` signoff** — that for the **exercised M1 literal subset** the
`Lx08CandidateType` M1 vocabulary is the **closed one-member set `{ Int }`**, with the
M1 literals `2` and `3` represented as **target-independent `Int` with no bit width**
(the `candidate_type` value on the committed `LiteralRecord`; **not** a target type,
ABI type, or `TypeId`); that literal **forms outside the exercised M1 subset** must
**not silently default to `Int`** and are classified as an **explicit
unsupported/deferred** result/diagnostic until their categories/rules are specified;
and that future categories **append symbolic members/rules without reinterpreting** the
selected `Int` member. This refines only the **meaning** of the already-selected
`Lx08CandidateType`, changes **no** field order/type/count, and **does not** claim to be
the **complete future C candidate vocabulary**, does **not** define any **numeric
tag/encoding**, is **not** a **T04/T08 owner or T01 `[INT]` signoff**, and is **not** a
`/6` freeze. The `Response`/`Status / signature` cells are **not** populated by this
note and **no bundle status changes** (rows C–I remain **`PENDING` overall**); the
**complete `Lx08CandidateType` member set and all numeric encodings remain open**; no
conflicting user-selected rev 42–52 decision is overridden; `/5` remains current, M1
remains DRAFT, ADR-0002 remains PROPOSED, and no code is authorized.

**Rev 54 — read-only audit notes + historical-supersession cleanup (separate note;
NOT a role signoff).** Added 2026-10-05, docs-only; no
`/5`/T01/manifest/other-doc/code/test edit; no `/6` freeze. This note records the
latest read-only findings and the supersession cleanups; it resolves **no**
architecture-critical choice and adds no signoff:

- **§G2 historical supersession.** The rev-21 `max_const_bits` signed-range/`i128`
  representable-set **formula**, the `ConstRecord.value: i128` **carrier**, and the
  chip-level **enforcement** wording are marked **historical and superseded — not
  selected** (the T08 correction marked them unselected). Only the **rev-44
  origin/cap/projection** (`max_const_bits` is a hashed `Limits` value; M1
  cap/default 128; `config` rejects `> 128`; explicit task-input projection) remains
  accepted. The exact **formula/carrier/enforcement** stay **open** `/6` co-freeze
  items (bundle **G**).
- **H9 duplicate-limit conflict — historical/superseded.** The rev-49
  `max_dispatches_per_tick`-vs-`max_inflight_per_tick` conflict text is qualified as
  **historical/superseded** by the **rev-51** sole-`max_inflight_per_tick` candidate
  (retained as history); the **dispatcher `Ready→Running`/`in_flight`
  commit-boundary and clear-order items remain unresolved** (bundle **A**).
- **T04 (read-only, open).** The reciprocal token↔literal same-batch link between
  the T04 `LiteralRecord.token` and its originating token needs a **cycle-safe ID
  reservation** mechanism at `/6` (a same-batch reciprocal reference can otherwise
  form a cycle before commit). **Open** `/6` co-freeze (bundle **C**); no mechanism
  selected here.
- **T05/T06 TU edge (read-only, open).** The T05 `parse.TranslationUnit ->`
  `symbol_type.scope-enter` edge (rev 43 File Enter) needs an **exactly-once**
  consumed TU with **atomic consumption/dedupe** so the committed TU triggers File
  Enter **once** (no duplicate/lost edge). **Open** `/6` co-freeze (bundles **D/E**);
  no mechanism selected here.
- **T13 no-residual fixtures (read-only, open).** The T13 **no-residual
  `Running`/in-flight** fixtures are **incomplete** (they do not yet cover all
  success/error/empty-proposal and fan-out paths, nor the H6 recovery +
  `in_flight` clear order). **Open** (bundle **A**/T13); pending.
- **M1 vertical PP-08 (read-only, open).** The vertical **PP-08** fixture's failure
  **semantics** and its **exact artifact-map arrays** are **pending** (the failure
  path and the exact `raw_offsets`/map arrays are not yet fixed). **Open**; pending.

These findings add **no** decision, no owner/T01 signoff, and no `/6` freeze; the
`Response`/`Status / signature` cells are **not** populated, and **rows C–I remain
`PENDING` overall**. No conflicting user-selected rev 42–53 decision is overridden;
`/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED, and no code is
authorized.

---

## 10. Acceptance record (partial, in principle)

**Partial acceptance in principle only.** On **2026-10-04** the **user** selected
the recommended baseline (decisions A/B/C direction) as the **rev-21 `/6`
contract-revision working basis**; on the same date the user also explicitly
accepted the recommended **H1 literal-handoff allocation split** in principle (row
C), and later recorded two further in-principle decisions (rev 31):
**A-narrow** — the Guardrails **§6.1 narrow interpretation** (same-task
`OwnBatch(DraftRef)` is a transient wire/proposal input only, resolved to committed
IDs before persistent state; no durable cursor/`WaitSet`/join points at a
draft/wire/address; guardrail unamended; T01 implementation-confirmation pending)
— and **B** — `TaskState::Waiting(WaitSet)` is the sole awaited-child-ID source,
`ContinuationRecord` carries no duplicate `awaited`, and the T01 §4 supersession is
accepted in principle for `/6` only (no `/5` edit), with T01 + T05 acceptance
pending. **Rev 34** additionally records the user's **in-principle acceptance of
the H6 batch-failure recovery direction** (row A): on a failed atomic semantic
batch commit, every dispatched task fails **exactly once** in **dispatch order** (a
committed `DiagnosticId` when diagnostic/record capacity allows, else the
`DiagnosticId::NONE` sentinel), so every task leaves `Running`, with clearing the
in-flight set **not itself** a transition — generalizing the verified `/5`
single-task `fail_selected` sentinel semantics (unchanged). **Rev 35** additionally
records the user's **in-principle acceptance of the H9 in-flight
scheduling-ownership direction** (row A): **remove `max_inflight_total`** from the
`/6` candidate because sequential per-tick dispatch is already bounded by
`max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; `Waiting`
is not in-flight; the dispatch batch bound stays the per-tick
`max_inflight_per_tick` quota; `tasks.in_flight` is an ephemeral per-tick scheduler
batch only (clears at latch after every dispatched task has a terminal/`Waiting`/
`Progress` outcome or H6 recovery); the dispatcher's `Ready→Running` pre-worker
mutation is distinguished from the ordered atomic semantic commit; a future
cross-tick `Running` mode needs a separate CDR. This is also a **direction only**:
**pending T01 integrator acceptance**, no bound/config/hash frozen, and the
remaining H9 `[INT]` items stay open.

**Rev 42 — user explicit per-subdecision acceptances (2026-10-05, authority
decisions).** On **2026-10-05** the **user** — who self-identifies as holding
**all** the named T01/owner roles (T01 `[INT]`, `[OWNER:T02]`, `[OWNER:T05]`,
`[OWNER:T06]`, `[OWNER:T07]`, `[OWNER:T09]`, `[OWNER:T13]`) and responded
**directly in each role** — explicitly accepted the following **selected
subdecisions only**, each recorded with its role label and date. These are
**human authority responses for the selected subdecisions**; they are **not**
wholesale bundle acceptances, **not** accepted contracts, **not** a `/6` freeze,
and authorize **no** code:

1. **A/T02/T13/T01 — H6 scope + mechanism.** The user preserves the
   in-principle direction and chooses to **freeze a quota>1-capable bounded batch
   recovery mechanism in `/6`**, explicitly **rejecting** the §9B row-A proposal to
   defer the H6 mechanism to a later CDR. Selected mechanism: pre-dispatch errors
   before state mutation leave tasks `Ready`; a semantic batch commit failure
   commits **no** semantic state; then a **deterministic bounded recovery
   mutation** processes the dispatched tasks **once in dispatch order** to
   `Failed`; **no pre-reservation of N diagnostics**; a **per-task diagnostic
   attempt**, using `DiagnosticId::NONE` when capacity is insufficient; a **state
   guard** prevents duplicate transition. This is an **explicitly selected H6
   mechanism**, but it does **not** claim the whole A bundle, the H9
   limits/stages/hash/error inventories, or any fixtures complete.
2. **D/T02/T05/T01 — join/CT07.** Accept the join as a **commit-apply
   invariant** with **no new CT07 committed carrier/family**, and **await-all
   children terminal before the parent decision**. This does **not** close the
   continuation fields/wire encoding, the `WaitSet` `/6` formal T01 §4
   supersession, the `OwnBatch` exact preapply errors, the parse contract/tests,
   or the rest of the D bundle.
3. **F/T06/T07/T09/T01 — checked carrier.** Accept **no `FunctionContextId`**;
   `SemRecord` is the **committed materialization of `CheckedNode`, one per
   `NodeId`**, with an **explicit committed typed link consumed by T09**. This does
   **not** close the conversion matrix, VF06/T13, effects/value category,
   diagnostics/tests, or the remaining F bundle.
4. **E/T06/T01 — canonical `TypeId` reuse.** Accept a **deterministic bounded
   scan of committed `types.records`**, returning the **lowest matching `TypeId`**,
   with **no hidden cache/index** (T05 upstream dependency acknowledged). This does
   **not** close the bootstrap or the remaining E bundle.

All **unselected** proposals remain **proposal-only**. `quota > 1` execution,
fairness/backpressure, the stage set, limits, hash, and numeric error codes remain
open. This is a **user instruction recording selected subdecisions**, **not** an
accepted contract, **not** a `/6` freeze, and **not** a code/chip authorization.
**T01 integrator acceptance remains pending for every other item; all remaining
owner sign-offs remain pending; `/5` remains current; M1 remains DRAFT; ADR-0002
remains PROPOSED; the broad rows D–I remain PENDING (as of that revision; from
rev 44 the overall pending set is C–I).**

**Rev 43 — further user explicit per-subdecision acceptances (2026-10-05,
authority decisions).** On **2026-10-05** the **user** — self-identifying as
holding **all** the named T01/owner roles (T01 `[INT]`, `[OWNER:T02]`,
`[OWNER:T05]`, `[OWNER:T06]`, `[OWNER:T07]`, `[OWNER:T09]`, `[OWNER:T13]`) and
responding **directly in each role** — additionally accepted the following
**selected subdecisions only**, each recorded with role label and **2026-10-05**
date; these are **human authority responses for the selected subdecisions**, not
wholesale bundle acceptances, not accepted contracts, not a `/6` freeze, and they
authorize **no** code:

1. **D/T05 — continuation exact shape.** Accept the candidate `ContinuationRecord`
   **exact ordered fields/shapes**: `production: TaskKind`, `cursor: TokenId`,
   `context: ParseContext`, `binding_power: u16`, `scope: Option<ScopeId>`,
   `parent: Option<NodeId>`, `partial_children: Vec<NodeId>`,
   `next_child_ordinal: u32`, `previous: Option<ContinuationId>`. There is **no
   `awaited`**; durable references are **committed IDs only**. The `RecordRef`
   numeric wire tags and the `RecordFamily` ordinals are **separate inventories**
   (their exact tag values are **not** accepted here — the canonical
   encoder/wire-enum numeric encodings remain a **T01 `/6` freeze detail**), and
   the `ParseContext` vocabulary still needs the **T05/T01 final encoding**.
2. **D/T01/T05 — T01 §4 supersession (formal).** Formally accept, **in the `/6`
   inventory**, the supersession of the T01 §4 old continuation-`awaited` wording:
   `TaskState::Waiting(WaitSet)` is the **only** awaited-child source, the
   continuation carries **no `awaited`**, and **`/5` is unchanged** (T01 §5
   unchanged; the change is a formal `/6` amendment only, no `/5` edit).
3. **D/T02/T05/T01 — OwnBatch pre-apply.** Accept the exact pre-apply proposal:
   a committed continuation exists/live; an OwnBatch continuation index lies within
   the same task's `AppendRecords` range and family `Continuation` (**phase 1**); a
   committed child is a valid committed child; an OwnBatch child index lies within
   the same task's Enqueue list (**phase 2b**, after the task's own Enqueues are
   known); exception errors are `ContinuationRefInvalid` and
   `AwaitChildrenRefInvalid`, whole-batch `CommitError`, checked **before any
   mutation**. The **numerical error codes/hash remain T01 `/6` details**.
4. **E/T06/T01 — File Enter trigger.** Accept the File Enter trigger as a
   deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge
   **after** the committed TU, carrying the committed `NodeId`, with **no
   job-bootstrap**. **T05's actual committed-TU carrier remains an upstream pending
   dependency**, and this accepted **stage-edge direction is not a TU schema
   signoff**.

All **unselected** proposals remain **proposal-only**. **Every overall D/E bundle
stays PENDING** where other items are unresolved: the `RecordRef` tag values /
`RecordFamily` ordinals (separate inventories; exact values not accepted), the
`ParseContext` enum and canonical encoder/wire numeric encodings, the OwnBatch error
numeric codes/hash, the parse request/result/test shapes, and the T05 committed-TU
upstream all remain **open**. This is a **user instruction recording selected
subdecisions**, **not** an accepted contract, **not** a `/6` freeze, and **not** a
code/chip authorization. **T01 integrator acceptance remains pending for every other
item; all remaining owner sign-offs remain pending; `/5` remains current; M1 remains
DRAFT; ADR-0002 remains PROPOSED; the broad rows D–I remain PENDING (as of that revision; from
rev 44 the overall pending set is C–I).**

**Rev 44 — further user explicit per-subdecision acceptances (2026-10-05,
authority decisions).** On **2026-10-05** the **user** — self-identifying as
holding **all** the named T01/owner roles (T01 `[INT]`, `[OWNER:T02]`,
`[OWNER:T03]`, `[OWNER:T04]`, `[OWNER:T05]`, `[OWNER:T06]`, `[OWNER:T07]`,
`[OWNER:T08]`, `[OWNER:T09]`, `[OWNER:T13]`) and responding **directly in each
role** — additionally accepted the following **selected subdecisions only**, each
recorded with role label and **2026-10-05** date; these are **human authority
responses for the selected subdecisions**, not wholesale bundle acceptances, not
accepted contracts, not a `/6` freeze, and they authorize **no** code:

1. **C/H1/T07/T08/T01 — `required_kind` meaning + `legality` placement.**
   `ConstantRequest.required_kind` denotes the **per-use constant-expression
   requirement** (for example the M1 integer constant expression), **distinct** from
   the lexical `LiteralRecord.candidate_type`; the request does **not** duplicate an
   implicit target type. `ConstantResult.legality` is a **result payload field** and
   introduces **no** extra committed record family. The exact `ConstLegality` variants
   remain **open** (a T04/T08/T01 co-freeze item).
2. **G/T08/T01 — `max_const_bits` origin.**
   The origin of `max_const_bits` is a **hashed `Limits` value**
   (`limits.max_const_bits` participates in the frozen-contract hash); the M1
   cap/default is **128**; `config` **rejects** values **> 128**; the bound reaches
   the zero-field T08 chip through an **explicit task-input projection**. This is an
   accepted **source/projection/bound** subdecision; the **exact** wire field, the
   diagnostic numeric code, and the hash encoding remain **`/6` details**, and the
   accepted bound/cap is **not** a claim that every valid config value must equal 128.
3. **C/T03/T01 — artifact shape/scope.** Accept `ArtifactRecord { kind, source,
   bytes, raw_offsets }` with the **total eight-`ArtifactKind`** scheme, where
   `Normalized`/`Spliced`/`CommentFree`/`Preprocessed` **require maps** and
   `Assembly`/`Object`/`Snapshot`/`Trace` **may omit** them. The M1 **exercised
   scope is only single-source `Normalized`**; the other producers and the
   multi-source map are **deferred**. This is the **selected M1 map shape/scope**;
   the **exact** `raw_offsets` invariant/error mapping and all enum numeric codes
   remain **open**.

All **unselected** proposals remain **proposal-only**. **Every overall C/G bundle
stays PENDING** where other items are unresolved: the exact `ConstLegality` variants,
the `RequiredKind` enum codes, the artifact-map boundary formula/error
classification, the `raw_offsets` detailed invariants, numeric IDs/tags, the
`max_const_bits` wire field/diagnostic code/hash encoding, the formula/enforcement
split, and the symbolic-vs-probe gating all remain **open**. This is a **user
instruction recording selected subdecisions**, **not** an accepted contract, **not**
a `/6` freeze, and **not** a code/chip authorization. **T01 integrator acceptance
remains pending for every other item; all remaining owner sign-offs remain pending;
`/5` remains current; M1 remains DRAFT; ADR-0002 remains PROPOSED; the broad rows
C–I remain PENDING.**

**Rev 45 — further user explicit per-subdecision acceptances (2026-10-05,
authority decisions).** On **2026-10-05** the **user** — self-identifying as
holding **all** the named T01/owner roles (T01 `[INT]`, `[OWNER:T03]`,
`[OWNER:T04]`, `[OWNER:T07]`, `[OWNER:T08]`) and responding **directly in each
role** — additionally accepted the following **selected subdecisions only**, each
recorded with role label and **2026-10-05** date; these are **human authority
responses for the selected subdecisions**, not wholesale bundle acceptances, not
accepted contracts, not a `/6` freeze, and they authorize **no** code:

1. **C/T04/T08/T01 — `LiteralRecord` exact ordered fields.** Accept the ordered
   fields `token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`,
   `suffix: LiteralSuffix`, `value: Vec<u8>` (big-endian magnitude),
   `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`.
   There is **no `node` and no `required_kind`**.
2. **C/T04/T08/T01 — M1 enum/scope.** Accept `LiteralKind {Integer, Character,
   String}` with **only `Integer` produced in M1**; `LiteralSuffix {None, U, L, UL,
   LL, ULL}` with **only `None` produced in M1**; radix domain `{2, 8, 10, 16}`
   with **M1 decimal only**; and a **symbolic/target-independent**
   `Lx08CandidateType` with M1 literals 2 and 3 as `Int` and **no bit width**. The
   **complete `Lx08CandidateType` member set and all numeric encodings remain
   open** — they are deliberately **not invented** here.
3. **C/T07/T08/T01 — `RequiredKind`/`ConstLegality`.** Accept `RequiredKind` M1
   enum only `IntegerConstantExpression`, and `ConstLegality` result-field values
   `Legal`, `NotConstantExpression`, `Unsupported`. Future C constant-expression
   purposes require **appended variants/new rules**, not a repurposing of the
   lexical candidate type.
4. **C/T03/T01 — mandatory artifact-map invariants.** Accept
   `raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last
   `<= source.bytes.len()`, and that **mandatory kinds require a valid source**. The
   **optional-kind map rule, source-versus-payload equality, and exact artifact
   error classification/numeric codes remain open** because the prompt allowed an
   empty optional map or another explicit rule and the user made no choice between
   them.

All **unselected** proposals remain **proposal-only**. **Every overall C/G bundle
stays PENDING** where other items are unresolved: the complete `Lx08CandidateType`
member set/numeric tags, the `ConstLegality`/`RequiredKind` numeric codes, the
optional-kind artifact-map rule, source-versus-payload equality, the exact artifact
error classification/numeric codes, and the `max_const_bits` wire
field/diagnostic code/hash encoding all remain **open**. This is a **user
instruction recording selected subdecisions**, **not** an accepted contract, **not**
a `/6` freeze, and **not** a code/chip authorization. **T01 integrator acceptance
remains pending for every other item; all remaining owner sign-offs remain pending;
`/5` remains current; M1 remains DRAFT; ADR-0002 remains PROPOSED; the broad rows
C–I remain PENDING.**

**Rev 46 — further user explicit per-subdecision acceptances (2026-10-05,
authority decisions).** On **2026-10-05** the **user** — self-identifying as
holding **all** the named T01/owner roles (T01 `[INT]`, `[OWNER:T06]`,
`[OWNER:T07]`, `[OWNER:T09]`, `[OWNER:T13]`) and responding **directly in each
role** — additionally accepted the following **selected subdecisions only**, each
recorded with role label and **2026-10-05** date; these are **human authority
responses for the selected subdecisions**, not wholesale bundle acceptances, not
accepted contracts, not a `/6` freeze, and they authorize **no** code:

1. **F/T07 — `ValueCategory` + `EffectMask`.** Accept
   `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }`, with
   the M1 fixture producing **`NonLvalue`**; M1 allows **only `EffectMask(0)`** — a
   **nonzero** mask is a **typed unsupported/diagnostic**, and the effect **bit
   classes are reserved/unassigned**.
2. **F/T07/T13/T01 — VF06 in M1.** Accept that VF06 **`TypedAstInvariant` is in
   M1**: it executes **after the committed T07 `SemRecord`s and before T09
   lowering**, checking **M1 typed-fact/required-conversion completeness**. The
   **exact registered task kind/stage/phase/interface remains T01/T13
   co-freeze/open** and is not asserted here.
3. **F/T06/T07/T09/T01 — M1-minimal conversion scope.** Select the **M1-minimal
   conversion scope only**: freeze **only** the conversion behavior the M1 fixture
   actually needs, including the **identity/no-conversion rule**. **Integer
   promotions, float conversions (incl. `FloatToFloat`), pointer-qualifier
   conversions, and other non-M1 conversions are explicitly unsupported and
   deferred** to a later **append/contract revision** (distinct from silently
   omitting them). **No** exact `ConversionOp`/`ConversionRole` closed variant list,
   pairing, role→chip mapping, or numeric encoding is invented here; all remain
   **open `/6` co-freeze details**.

All **unselected** proposals remain **proposal-only**. **Every overall F bundle
stays PENDING** where other items are unresolved: the exact VF06 registration (task
kind/stage/phase/interface), the full future conversion matrix, and all
`ConversionOp`/`ConversionRole`/`ValueCategory`/`EffectMask` numeric encodings remain
**open**. This is a **user instruction recording selected subdecisions**, **not** an
accepted contract, **not** a `/6` freeze, and **not** a code/chip authorization.
**T01 integrator acceptance remains pending for every other item; all remaining
owner sign-offs remain pending; `/5` remains current; M1 remains DRAFT; ADR-0002
remains PROPOSED; the broad rows C–I remain PENDING.**

**Authority and scope of the rev-46 acceptances (2026-10-05).** Authority: explicit
user instruction (`AGENTS.md` §2, highest precedence), 2026-10-05. The user
self-identifies as holding **all** the named T01/owner roles and responded
**directly in each role**; each acceptance above is recorded with its role label and
the **2026-10-05** date. Scope: **document-level selected subdecisions only on row
F** — **F/T07** the `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2,
Void=3 }` discriminants with the M1 fixture producing `NonLvalue`, and the
M1-only `EffectMask(0)` rule (nonzero is a typed unsupported/diagnostic; the effect
bit classes are reserved/unassigned); **F/T07/T13/T01** VF06 `TypedAstInvariant` is
**in M1**, executing after committed T07 `SemRecord`s and before T09 lowering and
checking M1 typed-fact/required-conversion completeness; and **F/T06/T07/T09/T01**
the **M1-minimal conversion scope only** (freeze only M1-fixture-needed conversion
behavior incl. identity/no-conversion; integer promotions, float conversions incl.
`FloatToFloat`, pointer qualifier, and other non-M1 conversions are **explicitly
unsupported/deferred** to a later append/contract revision). These acceptances do
**not** cover the **exact VF06 registration** (task kind/stage/phase/interface — still
T01/T13 co-freeze/open) or any `ConversionOp`/`ConversionRole` closed variant list,
pairing, role→T09-chip mapping, or numeric encoding (all deliberately **not
invented** and still open). This record does **not** amend the Guardrail, does
**not** edit `/5`/T01 §4/§5, does **not** add a field/enum/rule/hash, does **not**
authorize a chip wave, and does **not** turn on the pipeline. **M1 remains DRAFT;
ADR-0002 remains PROPOSED; `/5` remains current; the overall F bundle and the broad
rows C–I remain PENDING.**

**Rev 47 — integration-agent selected candidate defaults under explicit user
delegation (2026-10-05; recorded separately from authority acceptance).** This
record is **not** an authority acceptance. On 2026-10-05 the user explicitly
delegated to the assistant the adoption of its recommended choice for
**non-critical** decisions (asking only for critical decisions). Acting under that
delegation — and **not** as an owner or T01 `[INT]` signer — the **M1 integration
agent** selected four **low-risk candidate defaults**, each marked **selected under
user-delegated integration default** and dated **2026-10-05**: **(1)** the
optional-`ArtifactKind` map policy (`requires_map(kind) == false` → empty
`raw_offsets`; `source` remains `Option<SourceId>` valid when `Some`; mandatory-map
kinds require a valid `source` plus the accepted rev-45 invariants; **no**
source-payload-equals-bytes requirement; source-provenance/equality rule deferred;
exact numeric error codes open); **(2)** the T05 parse-depth default (reuse
`limits.max_task_depth`, count parser continuation/child frames only, detect before
any child enqueue for the descent, excess = `ParseDepthExceeded` chip diagnostic;
exact `ParseContext`/request-result variants pending); **(3)** the T06
namespace/symbol-lookup default (closed `SymbolKind`→namespace mapping, no namespace
field, miss-not-conflict, deterministic TY03 active-scope-chain lookup with
`(source,start,end,NodeId)` ordering / greatest ≤ point / same-scope higher
`SymbolId` / else innermost; T05 `NodeKind`/token-range dependency and exact event
encoding/lifecycle/allowlist rows still co-freeze); and **(4)** the T09 noncritical
rule defaults (shorter aliases `ir.op-immediate-type` and `ir.terminator-missing`;
immediate-before-result when `Constant` misses both; target type = result
`ValueRecord.ty`; all prospective `/6`; `CompletedFunction` marker family
unresolved, no family invented). **These are candidate defaults selected under
delegated integration default, not a user-authored technical decision, not owner
signoffs, and not T01 `[INT]` acceptance;** they are **not** evidence of
T03/T05/T06/T09 owner acceptance unless the user separately already accepted that
exact subdecision. **Overall rows C/D/E/H remain PENDING**, all public/shared schema
awaits owner/T01 co-freeze, `/5` remains current, M1 remains DRAFT, ADR-0002 remains
PROPOSED, and no code is authorized. This record does **not** amend the Guardrail,
does **not** edit `/5`/T01 §4/§5, and does **not** override any user-selected
rev 42–46 decision.

**Rev 48 — read-only T01 audit findings (2026-10-05; docs-only, not an authority
acceptance).** A **read-only** audit verified the checked-in `/5` code and
recorded, as **facts**, that: the frozen contract is `t01-c01-c06/5`
(`61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5`);
`compiler/src/contract.rs` encodes **24** `RECORD_KINDS`; `compiler/src/ids.rs`
`RecordRef` has **24** variants (tags **0–23**, `Artifact` = 23); and there is
**no** `RecordFamily` type in the code. The CDR's candidate figures — **19 draft
families / 27 `RecordRef` variants / 27 `RecordFamily` ordinals** — are
**proposed/unverified** draft counts, **not** established facts, and the future
`/6` count is **not** claimed to equal the current 24. The audit confirmed a
**real unresolved hash-scope conflict** (not just prose): `CONTRACT_VERSION`,
`compiler/src/contract.rs`, and `compiler/README.md` exclude
`group-declared-store-fields` from the hash, while `COMPILER_SFL_MANIFEST.md` §4
says adding fields/rules changes the frozen hash, and the proposed
`M1AppendSchema` is **proposed to be hashed at `/6`**. The **three hash-scope
options** — hash `M1AppendSchema` (update exclusion token + freeze assertion +
manifest); do not hash (correct manifest wording); or an explicit two-tier
hash-seed-vs-runtime-declarations split — are **not selected and not recommended**
here (no authority). The audit requires `/6` to specify **dual-inventory encoding**
(`RecordRef` wire tags separate from `RecordFamily` ordinals — the structural
subdecision accepted at rev 43), **inclusion of numeric values in the hash**, and
a **schema self-consistency mechanism**, freezing actual counts/values only after
the owner shapes are accepted. This is a **critical `/6` hash-scope choice with no
implementation authorization**. It adds **no owner acceptance and no signature**;
M1 remains DRAFT, ADR-0002 remains PROPOSED, `/5` remains current, and rows C–I
remain **PENDING**.

**Rev 49 — user critical H11/T09/T01 direction + read-only H9 audit findings
(2026-10-05; the H11 record is a user instruction; the H9 audit is read-only, not an
authority acceptance).** **(a) H11/T09/T01:** the **user** recorded an explicit
**2026-10-05 critical decision** — the `TerminatorMissing` trigger **reuses the
committed terminal result of the IR28 `FunctionEnd` task**
(`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the **deterministic
function-completion fact**, checked by a **T01-owned typed phase-2b commit-apply
validation hook** (entry-block termination); **no** new marker record
family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder and **no** new
`ResultValue` variant. The earlier **new-marker (`CompletedFunction`) direction is
superseded for the operative direction** but **preserved as history**. This is **not
a frozen `/6` hook/schema and authorizes no code:** the **exact hook contract, its
hash impact** (likely **no** new record family, but **rule/hook hashing remains a T01
decision**), **result typing/commit ordering**, and **T09/T01 co-freeze** remain
**open**, and **overall H stays PENDING**. **(b) H9 read-only audit:** the `/5` code
proves **only** the quota=1 single-task `fail_selected`, so the proposed **quota>1
pipeline is not implemented**; H9 still needs the **exact dispatcher
`Ready→Running`/`in_flight`-population relationship to the single ordered atomic
commit**, the **exact `in_flight` clear ownership/order relative to the bounded H6
recovery**, and a **no-`Running`/no-residual proof for all success/error/
empty-proposal paths**; the **`max_dispatches_per_tick`-vs-`max_inflight_per_tick`
internal proposed-limit conflict** was recorded to be **resolved in `/6`** (not
decided then) — **rev 54: historical/superseded** by the rev-51 drop of
`max_dispatches_per_tick` (sole `max_inflight_per_tick`/quota bound), so it is **no
longer an open conflict**;
the **H6/H9 fan-out fixtures** and **T13 VF02/VF03/VF04/VF13** remain **pending**. The
H9 no-residual guarantee is **conditional on the H6 recovery implementation**, the
**selected H6 mechanism and the H9 removal direction are unchanged**, and the
**quota=1 M1 baseline stays separated** from the quota>1 optimization; no freeze/code;
exact stages/version carrier/errors/hashes remain pending. The audit is **not a
signature**; M1 remains DRAFT, ADR-0002 remains PROPOSED, `/5` remains current, and
rows C–I remain **PENDING**.

**Rev 50 — user critical HASH-SCOPE decision (separate note; `[USER]` scope
record, NOT a T01 `[INT]` signature).** Added 2026-10-05, docs-only; no
`/5`/T01/manifest/other-doc/code/test edit; no `/6` freeze. The **user** recorded an
explicit **critical decision** accepting the **two-tier hash-scope model**:
`StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed** and
**participates in the `/6` contract hash**, while **post-seed runtime
`StoreSchema::declare()` extensions remain excluded** from the frozen hash and are
**captured/validated through runtime snapshot/schema mechanisms**. This **resolves
the conceptual future `/6` boundary** — the three hash-scope options are **no longer
open at the conceptual level** — but it **authorizes or completes no actual `/6`
schema values, hash, tag/count inventory, code, or `/5` change**: the **selected
boundary does not itself accept exact `M1AppendSchema` contents/counts and freezes
nothing**, and **T01 must implement it as integration authority and still co-freeze
the M1 seed values after the owners** (numeric inventory and test implementation
**remain pending**). At `/6` integration T01 must **atomically** update the
normative [COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md)
§4 wording, the **`hash_excludes` semantic description/token** (scoped to **post-seed
runtime declarations**, not the frozen M1 seed), and `FrozenSchema::encode`/contract
code plus the freeze test, **preserving the frozen `/5` hash and history**. Per
the **CDR §9C response protocol** (and `AGENTS.md` §2 authority order, which
places explicit user instruction first), this is the **user's own `[USER]` scope
decision**; it is **not** a
row-I closure, **not** a `/6` freeze, and **not** a T01 signature beyond that scope
decision. The `Response`/`Status / signature` cells are **not** populated by this
note (only the `[USER]` scope is recorded in the T01-global row's authority), **no
bundle status changes** (rows C–I, including I, remain **`PENDING` overall**), and
no conflicting user-selected rev 42–49 decision is overridden; `/5` remains current,
M1 remains DRAFT, ADR-0002 remains PROPOSED.

**Rev 51 — integration-selected candidate default under explicit user delegation
(2026-10-05; NOT an owner or T01 `[INT]` signoff).** Under the user's explicit
delegation for non-critical decisions, the **M1 integration agent** selected a
**candidate default** — **`max_inflight_per_tick`/quota is the sole per-tick
dispatch-count bound**, and the redundant **`max_dispatches_per_tick` is dropped**
from the candidate limit inventory and validation — grounded in the rev-35/38 H9
direction and the rev-49 audit finding that the two limits duplicated each other.
This is a **delegated candidate default**, **not** an owner/T01 `[INT]` signoff,
**not** a schema freeze, and **not** code; it selects no dispatcher-`Ready→Running`/
`in_flight` atomic-boundary or clear-order item and changes neither the H6 mechanism
nor the H9 removal direction. The exact names/defaults/codes remain **T01/T02 `/6`
co-freeze** and the T02/T13 tests remain **pending**; **row A and every overall row
stay PENDING**; `/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED.

**Rev 52 — cross-owner audit/reconciliation (read-only; NOT an authority
acceptance).** Added 2026-10-05, docs-only; no `/5`/T01/manifest/other-doc/code/test
edit; no `/6` freeze. A **read-only** reconciliation confirmed that the completed
owner packages mirror the CDR's already-selected items — **T02 rev 30+31**;
**T03/T04** (T04 rev 46 incl. the `LiteralRecord.token` candidate and the
same-revision correction to the typed `RecordLink` mechanism, with no `OwnBatch`
confusion for record links); **T05 `ParseContext` candidate**; **T06**; **T07
sem→const correction**; **T08 exact candidate shape**; **T09 H11**; **T13**; **M1
vertical acceptance rev 27**; **M1 target acceptance rev 29**; and **ADR-0002
Revision 17** — and that the M1 proposal/target/vertical docs align on H11, the hash
boundary, and the Part A-vs-B split, with the proposal being advanced in parallel to
**rev 38** (expected resulting file). It records that the T01 audit still leaves
`/6` **not freeze-ready** (exact record/tag/family inventories, field bodies/ref
encoding, task/request/result typing, writer manifests, TU carrier/`ParseContext`
final encoding, `SourceId`/token provenance/raw-offset mapping and
`Lx08CandidateType`, `SemRecord`/VF06, const overflow/projection, the IR hook
contract, the scheduler H6/H9 atomic boundaries/in_flight clear order/no-residual
fixtures, and the snapshot/canonical-encoder/hash tests), with **no new
user-critical decision implied**; that the **rev-51 sole-per-tick-bound** and
**rev-50 hash-scope** decisions remain **selected** with exact
implementation/sign-off pending and **rows C–I PENDING overall**; and that the CDR
is an **accepted-decision ledger/work queue, not an executable schema**, whose
**next integration stage after exact owner artifacts is T01's serial `/6` seed
assembly** self-consistency-checked against the frozen two-tier hash model, with
**no chip coding before freeze** and **Part B independent**. **No owner package is
called a `/6` sign-off.** It adds **no owner acceptance and no signature**; M1
remains DRAFT, ADR-0002 remains PROPOSED, `/5` remains current, and rows C–I remain
**PENDING**.

**Rev 53 — integration-agent selected candidate default under explicit user delegation
(2026-10-05; NOT an owner or T01 `[INT]` signoff).** Under the user's explicit
delegation for non-critical decisions, and grounded in a read-only review of the
exercised M1 integer literal subset, the **M1 integration agent** selected a **candidate
default** for the **M1 exercised literal subset**: the `Lx08CandidateType` M1
vocabulary is the **closed one-member set `{ Int }`**, the M1 literals `2` and `3` are
represented as **target-independent `Int` with no bit width** (the `candidate_type`
value on the committed `LiteralRecord`; not a target type/ABI type/`TypeId`); literal
**forms outside the exercised M1 subset** must **not silently default to `Int`** and are
**explicitly unsupported/deferred** until their categories/rules are specified; and
future categories **append symbolic members/rules without reinterpreting** the selected
`Int` member. This **refines only the meaning** of the already-selected
`Lx08CandidateType`; it changes **no** field order/type/count, is **not** an
owner/T04 or T01 `[INT]` signoff, **not** a `/6` freeze, **not** the complete future C
candidate vocabulary, and asserts **no** numeric tag/encoding; the **complete
`Lx08CandidateType` member set and all numeric encodings remain open**. Rows C–I
remain **PENDING overall**; `/5` remains current, M1 remains DRAFT, ADR-0002 remains
PROPOSED, and no code is authorized.

**Rev 54 — read-only audit notes + historical-supersession cleanup (2026-10-05;
docs-only; NOT an authority acceptance).** Added docs-only; no
`/5`/T01/manifest/other-doc/code/test edit; no `/6` freeze. **(a) §G2 supersession:**
the rev-21 `max_const_bits` signed-range/`i128` representable-set formula, the
`ConstRecord.value: i128` carrier, and the chip-level enforcement wording are recorded
as **historical and superseded — not selected** (the T08 correction marked them
unselected), with only the rev-44 origin/cap/projection retained; the exact
formula/carrier/enforcement stay **open** (row **G**). **(b) H9 duplicate-limit
conflict:** the rev-49 conflict text is qualified as **historical/superseded** by the
rev-51 sole-`max_inflight_per_tick` candidate (retained as history), while the
dispatcher `Ready→Running`/`in_flight` commit-boundary and clear-order items stay
**unresolved** (row **A**). **(c) Latest read-only findings (all open, no choice
made):** the **T04 reciprocal token↔literal same-batch link** needs a **cycle-safe ID
reservation** (row **C**); the **T05/T06 `parse.TranslationUnit ->`
`symbol_type.scope-enter` TU edge** needs an **exactly-once atomic consumption/dedupe**
(rows **D/E**); the **T13 no-residual fixtures** were incomplete at this revision and
are now **extended by T13 rev 5** (H6-M13..H6-M16; current T13 rev 11), still **required/pending**
(row **A**/T13);
and the **M1 vertical PP-08 failure semantics and exact artifact-map arrays** are
**pending**. This record adds **no** decision, owner/T01 signoff, or `/6` freeze; rows
C–I remain **PENDING overall**; `/5` remains current, M1 remains DRAFT, ADR-0002
remains PROPOSED, and no code is authorized.

Rows
C–I are **PENDING** (row C carries the accepted-in-principle H1 allocation and the
rev-44/rev-45 partial subdecisions but is **not** fully accepted; row D carries the
A-narrow/B sub-decisions but is **not**
fully accepted; row A's H6/H9 branches are selected directions, not completed
mechanisms — with a rev-51 integration-selected candidate default dropping the
duplicate `max_dispatches_per_tick` in favour of the sole `max_inflight_per_tick`/
quota bound; row F carries the rev-42 carrier subdecisions and the rev-46
`ValueCategory`/`EffectMask` + VF06-in-M1 + M1-minimal-conversion-scope subdecisions
but is **not** fully accepted; row G carries the rev-44 `max_const_bits` origin and
the rev-45 `RequiredKind`/`ConstLegality` value subdecisions; row H carries the
rev-49 selected `FunctionEnd`-terminal-result H11 direction but is **not** fully
accepted). Owner
reviews (T02–T09, T13) are **conditional**; no reviewer signature is asserted or
implied.

| # | Decision | Required from | Status | Date | Reference |
|---|---|---|---|---|---|
| A | Pipeline scheduling model / stage set / quota policy | T01 integrator + user | **ACCEPTED IN PRINCIPLE (user, direction only):** deterministic bounded **sequential** staged pipeline; `quota = 1` semantic-comparison-projection baseline; `quota > 1` only after measurement + integrator acceptance. **A-narrow (rev 31):** Guardrails §6.1 narrow interpretation — same-task `OwnBatch(DraftRef)` is transient wire/proposal input only, resolved to committed IDs before persistent state; no durable cursor/`WaitSet`/join points at a draft/wire/address; guardrail unamended; **T01 implementation-confirmation pending.** **H6 batch-failure recovery direction (rev 34):** on a failed atomic semantic batch commit, every dispatched task fails **exactly once** in **dispatch order** (a committed `DiagnosticId` when capacity allows, else the `DiagnosticId::NONE` sentinel), so every task leaves `Running`; clearing the in-flight set is **not itself** a transition — generalizes the verified `/5` single-task sentinel semantics. **Exact atomic/bounded implementation requires T01 integrator approval; batch failure-atomicity details, diagnostic budget/state, and T02/T13 owner fixtures/sign-offs remain co-freeze/pending.** **H9 in-flight scheduling-ownership direction (rev 35):** accept **removing `max_inflight_total`** from the `/6` candidate — per-tick dispatch is bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; `Waiting` is not in-flight; `tasks.in_flight` is an ephemeral per-tick scheduler batch only; the dispatcher's `Ready→Running` pre-worker mutation is distinguished from the ordered atomic commit. **Pending T01 integrator acceptance; no bound frozen.** Exact stage set/fairness/backpressure/metrics/errors pending. **Integrator acceptance pending.** **Partial acceptance (user, rev 42, 2026-10-05):** H6 scope = **freeze a quota>1-capable bounded batch recovery mechanism in `/6`** (rejecting the §9B row-A deferral to a later CDR); H6 mechanism = pre-dispatch errors before state mutation leave tasks `Ready`; semantic batch commit failure commits no semantic state; deterministic bounded recovery mutation processes dispatched tasks **once in dispatch order** to `Failed`; **no pre-reservation of N diagnostics**; per-task diagnostic attempt with `DiagnosticId::NONE` when capacity insufficient; state guard prevents duplicate transition — selected subdecisions only; H9/other pipeline inventories remain PENDING. **Rev 49 read-only H9 audit (unresolved blockers, not an acceptance; no change to the selected H6 mechanism or the H9 removal direction):** `/5` proves only the quota=1 single-task `fail_selected` transition, so the proposed quota>1 pipeline is **unimplemented**; the exact dispatcher `Ready→Running`/`in_flight`-population-vs-single-ordered-atomic-commit relationship, the exact `in_flight` clear ownership/order relative to the bounded H6 recovery, and a no-`Running`/no-residual proof for all success/error/empty-proposal paths remain **open**; the `max_dispatches_per_tick`-vs-`max_inflight_per_tick` internal proposed-limit conflict is **deferred to `/6`**; the H6/H9 fan-out fixtures and T13 VF02/VF03/VF04/VF13 remain **pending**; the H9 no-residual guarantee is **conditional on the H6 recovery implementation**; no freeze/code. **Rev 51 integration-selected candidate default under user delegation (2026-10-05; not owner/T01 `[INT]` signoff):** `max_inflight_per_tick`/quota is the **sole per-tick dispatch-count bound** and the redundant `max_dispatches_per_tick` is **dropped** from the candidate limit inventory and validation (per the rev-35/38 H9 direction + rev-49 audit duplication finding); exact names/defaults/codes remain **T01/T02 `/6` co-freeze**, T02/T13 tests **pending**. | 2026-10-04 / 2026-10-05 | §A, §2, §5, §9C |
| B | One-writer rule; span owner | User (guardrail) + T01 integrator + T03/T04/T05 | **ACCEPTED IN PRINCIPLE (user):** one writer per field; `sources.spans` = **T03 only**; T04/Token reuse committed T03 PP spans, no span writes; AST `Node` = first/last `TokenId` range, no T05 span write; guardrail unamended. **Integrator + owner sign-offs pending.** | 2026-10-04 | §B, §2, §5 |
| C | Artifact map + literal/const handoff | T03/T04/T08 + T01 integrator | **ACCEPTED IN PRINCIPLE (user, handoff direction + H1 allocation split, 2026-10-04):** committed T04-owned `LiteralRecord` carrying the per-literal lexical facts + `LX08` candidate type; sem-stage `ConstantRequest` carrying `node`/`required_kind`; `ConstantResult` carrying `legality`; T08 sole `constants.records` writer. **Working basis only — not a freeze, not code/chip authorization. Exact variants/schema and T01 + T03/T04/T08 owner co-freeze/sign-off pending.** See §C2/§F4/§9. **Further partial acceptance (user, rev 44, 2026-10-05 subdecisions only):** **C/H1/T07/T08/T01** — `required_kind` = per-use constant-expression requirement (e.g. M1 integer constant expression), distinct from lexical `candidate_type`, no duplicate implicit target type; `ConstantResult.legality` is a result payload field with no extra committed family (exact `ConstLegality` variants open); **C/T03/T01** — `ArtifactRecord { kind, source, bytes, raw_offsets }` + total eight-`ArtifactKind` map rule (map-mandatory Normalized/Spliced/CommentFree/Preprocessed; map-optional Assembly/Object/Snapshot/Trace), M1 exercised scope only single-source `Normalized`. **Still PENDING:** exact `RequiredKind` codes, `ConstLegality` variants, artifact map boundary formula/error classification, `raw_offsets` invariants, enum numeric codes, and T04's exact `LiteralRecord` fields/variants (not covered by rev 44). **Further partial acceptance (user, rev 45, 2026-10-05 subdecisions only):** **C/T04/T08/T01** — exact ordered `LiteralRecord` fields (`token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` big-endian magnitude, `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`; no `node`/`required_kind`), the M1 `LiteralKind {Integer, Character, String}` (only `Integer` produced) / `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced) / radix `{2,8,10,16}` (M1 decimal only) scope, and symbolic `Lx08CandidateType` (M1 literals 2/3 = `Int`, no bit width); **C/T07/T08/T01** — `RequiredKind` M1 enum only `IntegerConstantExpression`, `ConstLegality` values `Legal`/`NotConstantExpression`/`Unsupported` (future purposes via appended variants/new rules); **C/T03/T01** — mandatory-map invariants (`raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds require a valid source). **Still PENDING (rev 45):** the complete `Lx08CandidateType` member set/numeric encodings, the optional-kind artifact-map rule, source-versus-payload equality, and the exact artifact error classification/numeric codes. **Rev 47 delegated candidate default (2026-10-05; integration-agent selected under user delegation, not an authority acceptance):** the optional-kind map policy (map-optional kinds have empty `raw_offsets`; `source` valid when `Some`; no source-payload-equals-bytes requirement; source-provenance/equality rule deferred; exact numeric error codes open) is selected; **overall C stays PENDING.** **Rev 53 delegated candidate default (2026-10-05; integration-agent selected under explicit user delegation, not an authority acceptance):** for the **exercised M1 literal subset** the `Lx08CandidateType` M1 vocabulary is the **closed one-member set `{ Int }`**, with M1 literals `2`/`3` = **target-independent `Int`, no bit width**; **forms outside the exercised M1 subset must not silently default to `Int`** and are **explicitly unsupported/deferred**; future categories **append without reinterpretation**; this is **not** an owner/T04 or T01 `[INT]` signoff, **not** a `/6` freeze, **not** the complete future C candidate vocabulary, and asserts **no numeric tags**; the complete `Lx08CandidateType` member set/encodings remain open; **overall C stays PENDING.** | 2026-10-04 / 2026-10-05 | §C, §2, §9C |
| D | T05 continuation/join protocol | T05 + T01 integrator | **PENDING overall** (exact fields/encoding, CT07 carrier, sibling policy). **A-narrow/B sub-decisions accepted in principle (user, rev 31):** own-batch `DraftRef` transient-only, resolved to committed IDs before persistent state; `TaskState::Waiting(WaitSet)` sole awaited-child source with no duplicate `ContinuationRecord.awaited`, superseding T01 §4 in `/6` (no `/5` edit). **T01 + T05 acceptance pending.** **Partial acceptance (user, rev 42, 2026-10-05):** join is a commit-apply invariant with **no new CT07 committed carrier/family**; **await-all children terminal before the parent decision** — accepted subdecisions only. **Further partial acceptance (user, rev 43, 2026-10-05):** the candidate `ContinuationRecord` **exact ordered fields/shapes** (`production: TaskKind`, `cursor: TokenId`, `context: ParseContext`, `binding_power: u16`, `scope: Option<ScopeId>`, `parent: Option<NodeId>`, `partial_children: Vec<NodeId>`, `next_child_ordinal: u32`, `previous: Option<ContinuationId>`; no `awaited`; durable committed-ID references only); the **formal `/6` T01 §4 supersession** (`WaitSet` sole awaited-child source; no `/5` edit); and the **exact OwnBatch pre-apply validation** (phase-1 continuation ref within the same task `AppendRecords` range/family `Continuation`; phase-2b child ref within the same task Enqueue list; whole-batch `CommitError` before any mutation). **Still PENDING:** `RecordRef` wire-tag values / `RecordFamily` ordinals (separate inventories), the `ParseContext` vocabulary final encoding, OwnBatch numerical error codes/hash, and parse request/result/test shapes. **Rev 47 delegated candidate default (2026-10-05; integration-agent selected under user delegation, not an authority acceptance):** T05 parse depth reuses `limits.max_task_depth`, counts parser continuation/child frames only, detects the limit before any child enqueue for the descent, and reports excess as the `ParseDepthExceeded` chip diagnostic; **overall D stays PENDING.** | 2026-10-04 / 2026-10-05 | §D, §9C |
| E | T06 scope/symbol/type details | T06 + T01 integrator | PENDING overall (`Identifier`-leaf `decl` and Block-node boundary selected as draft; field encodings pending). **Partial acceptance (user, rev 42, 2026-10-05):** deterministic bounded scan of committed `types.records` returning the **lowest matching `TypeId`**, **no hidden cache/index** — accepted subdecision only. **Further partial acceptance (user, rev 43, 2026-10-05):** the File Enter trigger is a deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge **after** the committed TU, carrying the committed `NodeId`, **no job-bootstrap**. **Still PENDING:** T05's committed-TU carrier is an upstream dependency (the accepted stage-edge direction is **not** a TU schema signoff); bootstrap wiring and the rest of E remain open. **Rev 47 delegated candidate default (2026-10-05; integration-agent selected under user delegation, not an authority acceptance):** the closed `SymbolKind`→namespace mapping (no namespace field; miss-not-conflict) and the deterministic TY03 active-scope-chain lookup are selected; T05 `NodeKind`/token-range dependency and exact event encoding/lifecycle/allowlist rows stay co-freeze; **overall E stays PENDING.** | 2026-10-05 | §E, §9C |
| F | T07 committed checked carrier + conversion alignment | T07/T06/T09 + T01 integrator | PENDING overall (no T09-`FunctionRecord` cycle + `SemRecord` presence selected as draft; exact carrier/conversion matrix pending). **Partial acceptance (user, rev 42, 2026-10-05):** **no `FunctionContextId`**; `SemRecord` is the committed materialization of `CheckedNode`, **one per `NodeId`**, with an **explicit committed typed link consumed by T09** — accepted subdecisions only; conversion matrix, VF06/T13, effects/value category, diagnostics/tests remain PENDING. **Further partial acceptance (user, rev 46, 2026-10-05 subdecisions only):** **F/T07** — `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` (M1 fixture = `NonLvalue`) and M1 only `EffectMask(0)` (nonzero typed unsupported/diagnostic; effect bit classes reserved/unassigned); **F/T07/T13/T01** — VF06 `TypedAstInvariant` **is in M1**, executing after committed T07 `SemRecord`s and before T09 lowering, checking M1 typed-fact/required-conversion completeness (exact registered task kind/stage/phase/interface still T01/T13 co-freeze/open); **F/T06/T07/T09/T01** — **M1-minimal conversion scope only** (freeze only M1-fixture-needed behavior incl. identity/no-conversion; integer promotions, float conversions incl. `FloatToFloat`, pointer qualifier, and other non-M1 conversions **explicitly unsupported/deferred** to a later append/contract revision; no `ConversionOp`/`ConversionRole` closed list, pairing, role→chip mapping, or numeric encoding invented). **Still PENDING:** the exact VF06 registration and all conversion/effect numeric encodings plus the full future conversion matrix. | 2026-10-05 | §F, §9C |
| G | T08 const diagnostics/bounds | T08 + T01 integrator | PENDING overall (`max_const_bits = 128` origin/cap accepted; **the rev-21 signed-range/`i128` formula is historical/superseded — not selected, rev 54**; exact formula/carrier/enforcement open). **Partial acceptance (user, rev 44, 2026-10-05 subdecisions only):** `max_const_bits` origin is a **hashed `Limits` value**; M1 cap/default **128**; `config` **rejects** values **> 128**; explicit **task-input projection** to the restricted T08 chip (accepted source/projection/bound subdecision; the cap does **not** require every valid config value to equal 128). **Still PENDING:** exact wire field/diagnostic numeric code/hash encoding, the formula/enforcement split, symbolic-vs-probe gating. **Further partial acceptance (user, rev 45, 2026-10-05 subdecisions only):** **C/T07/T08/T01** — `RequiredKind` M1 enum only `IntegerConstantExpression`; `ConstLegality` values `Legal`/`NotConstantExpression`/`Unsupported`; future C constant-expression purposes via appended variants/new rules, not lexical-candidate reuse; numeric encodings still PENDING. | 2026-10-05 | §G, §9C |
| H | T09 IR op/terminator rules | T09 + T01 integrator | PENDING (**rev 49: the user selected the `FunctionEnd`-terminal-result trigger direction** — `TerminatorMissing` reuses the committed IR28 `FunctionEnd` terminal result (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic completion fact, checked by a T01-owned typed phase-2b commit-apply validation hook, with **no** new marker record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder or `ResultValue` variant; the earlier new-marker direction is **superseded for operative direction** and preserved as history; **not a frozen `/6` hook/schema, no code**; exact hook contract/hash impact (rule/hook hashing still a T01 decision)/result typing/commit ordering/T09-T01 co-freeze open; overall H stays pending). `Constant` immediate = result type selected as draft; rule ids/fields pending; both `ir.op-immediate-type` aliases and both `ir.terminator-missing*` candidates unresolved (H8). **Rev 47 delegated candidate default (2026-10-05; integration-agent selected under user delegation, not an authority acceptance):** the shorter aliases `ir.op-immediate-type` and `ir.terminator-missing` are selected, `Constant` validates the immediate before the result when both are missing, and the target type equals the result `ValueRecord.ty`; all prospective `/6`. | 2026-10-05 | §H, §9C |
| I | Cross-doc consistency, snapshot, hash, errors, T01 §4 | T01 integrator + user `[USER]` (hash scope) | PENDING. **Rev 48 read-only audit (not an acceptance):** confirmed `/5` code facts (**24** `RECORD_KINDS`/`RECORD_REF` variants, tags 0–23; `RecordFamily` absent), the **proposed/unverified** 19/27/27 draft counts, and the **real unresolved** `M1AppendSchema` hash-scope conflict with three **unselected/unrecommended** options; no signature. **Rev 49 (not an acceptance):** records the user's critical H11/T09/T01 direction (reuse the committed IR28 `FunctionEnd` terminal result, no new marker family/ID/arena/tag/ordinal/encoder/`ResultValue`; T01-owned typed phase-2b commit-apply validation hook; not a frozen `/6` hook, no code) and the read-only H9 audit findings as unresolved blockers; no hash-scope option selected, no freeze. **Rev 50 (`[USER]`, 2026-10-05; critical hash-scope decision):** the user **accepts the two-tier model** — `StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed participating in the `/6` contract hash**, while **post-seed runtime `declare()` extensions stay excluded** and are captured/validated via runtime snapshot/schema mechanisms; at `/6` integration T01 must **atomically** update the manifest §4 wording, the `hash_excludes` semantic description/token (**post-seed runtime declarations**, not the M1 seed), and `FrozenSchema::encode`/contract code + freeze test, **preserving the `/5` hash/history**. This **settles the conceptual boundary** but **accepts no exact `M1AppendSchema` contents/counts, freezes nothing, changes no `/5`, and authorizes no code**; the **numeric inventory/test implementation remain pending** and T01 still co-freezes the M1 seed values after the owners. Row I and the overall CDR remain **PENDING**. | 2026-10-05 | §I, §9C, §3.3 |

**Authority and scope of the in-principle acceptance.** Authority: explicit user
instruction (`AGENTS.md` §2, highest precedence), 2026-10-04. Scope: **document-
level revision working basis** for rows A/B/C, plus the rev-31 A-narrow/B
sub-decisions (rows A/D) and the rev-34 H6 batch-failure recovery **direction**
(row A); it does **not** amend the Guardrail
(the §6.1 text is unamended — the own-batch narrow interpretation is an explicit
user in-principle reading of it, still subject to T01 implementation-confirmation),
does **not** edit `/5`/T01 §4 (the awaited supersession is a formal `/6` amendment
only), does **not** add a
field/enum/rule/hash, does **not** authorize a chip wave, and does **not** turn on
the pipeline. The H9 removal direction is
**not** an accepted mechanism: it is pending **T01 integrator acceptance**,
it freezes no bound/config/hash, and the dispatcher-`Ready→Running`-vs-ordered-
atomic-commit relationship and the residual-set semantics stay open. `quota > 1` additionally
requires measured before/after evidence and
separate integrator acceptance.

**Authority and scope of the rev-42 acceptances (2026-10-05).** Authority:
explicit user instruction (`AGENTS.md` §2, highest precedence), 2026-10-05. The
user self-identifies as holding **all** the named T01/owner roles
(T01 `[INT]`, `[OWNER:T02]`, `[OWNER:T05]`, `[OWNER:T06]`, `[OWNER:T07]`,
`[OWNER:T09]`, `[OWNER:T13]`) and responded **directly in each role**; each
acceptance above is recorded with its role label and the **2026-10-05** date.
Scope: **document-level selected subdecisions only** — H6 scope+mechanism (row A),
the CT07 commit-apply-invariant + await-all (row D), the no-`FunctionContextId` /
`SemRecord`-carrier decisions (row F), and the lowest-`TypeId` committed scan
(row E). The H6 mechanism is now an **explicitly selected mechanism to be frozen
in `/6`** (not a later-CDR deferral), but it does **not** authorize code and does
**not** freeze `/6` yet; `quota > 1` execution, the H9 limits/stages/hash/error
inventories, fixtures, and every other unselected item remain open. This record
does **not** amend the Guardrail, does **not** edit `/5`/T01 §4, does **not** add a
field/enum/rule/hash, does **not** authorize a chip wave, and does **not** turn on
the pipeline. **M1 remains DRAFT; ADR-0002 remains PROPOSED; `/5` remains current;
the broad rows C–I remain PENDING.**

**Authority and scope of the rev-43 acceptances (2026-10-05).** Authority: explicit
user instruction (`AGENTS.md` §2, highest precedence), 2026-10-05. The user
self-identifies as holding **all** the named T01/owner roles (T01 `[INT]`,
`[OWNER:T02]`, `[OWNER:T05]`, `[OWNER:T06]`, `[OWNER:T07]`, `[OWNER:T09]`,
`[OWNER:T13]`) and responded **directly in each role**; each acceptance above is
recorded with its role label and the **2026-10-05** date. Scope:
**document-level selected subdecisions only on rows D and E** — the `ContinuationRecord`
exact ordered fields/shapes (D/T05; no `awaited`), the formal `/6` T01 §4
supersession (D/T01/T05; no `/5` edit), the exact OwnBatch pre-apply validation
(D/T02/T05/T01; whole-batch `CommitError` before any mutation), and the File Enter
deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge
(E/T06/T01; committed `NodeId`; no job-bootstrap). The `RecordRef` numeric wire tags
and `RecordFamily` ordinals remain **separate inventories**; their exact tag values
are **not** accepted (numeric encodings are a T01 `/6` freeze detail), the
`ParseContext` vocabulary final encoding, the OwnBatch numerical error codes/hash,
and the parse request/result/test shapes remain open, and T05's committed-TU
carrier is a **pending upstream dependency** (the accepted stage-edge direction is
not a TU schema signoff). This record does **not** amend the Guardrail, does **not**
edit `/5`/T01 §4/§5, does **not** add a field/enum/rule/hash, does **not** authorize
a chip wave, and does **not** turn on the pipeline. **M1 remains DRAFT; ADR-0002
remains PROPOSED; `/5` remains current; both overall D/E bundles and the broad rows
D–I remain PENDING (at rev 43; from rev 44 the overall pending set is C–I).**

**Authority and scope of the rev-44 acceptances (2026-10-05).** Authority: explicit
user instruction (`AGENTS.md` §2, highest precedence), 2026-10-05. The user
self-identifies as holding **all** the named T01/owner roles (T01 `[INT]`,
`[OWNER:T02]`, `[OWNER:T03]`, `[OWNER:T04]`, `[OWNER:T05]`, `[OWNER:T06]`,
`[OWNER:T07]`, `[OWNER:T08]`, `[OWNER:T09]`, `[OWNER:T13]`) and responded
**directly in each role**; each acceptance above is recorded with its role label and
the **2026-10-05** date. Scope: **document-level selected subdecisions only on rows
C and G** — the `required_kind` per-use constant-expression meaning and the
`legality` result-payload placement (C/H1/T07/T08/T01), the `ArtifactRecord` shape +
total eight-`ArtifactKind` map rule with M1 scope only single-source `Normalized`
(C/T03/T01), and the `max_const_bits` hashed-`Limits` origin / M1 cap-default 128 /
config `> 128` rejection / explicit task-input projection (G/T08/T01). These
acceptances do **not** cover the exact `ConstLegality` variants, the `RequiredKind`
enum codes, the artifact-map boundary formula/error classification, the
`raw_offsets` detailed invariants, any numeric IDs/tags or enum numeric codes, the
`max_const_bits` wire field/diagnostic numeric code/hash encoding, the
formula/enforcement split, or the symbolic-vs-probe gating — all of which remain
open; the accepted `max_const_bits` cap is **not** a claim that every valid config
value must equal 128, and `[OWNER:T04]`'s exact `LiteralRecord` fields/enum variants
are **not** claimed accepted. This record does **not** amend the Guardrail, does
**not** edit `/5`/T01 §4/§5, does **not** add a field/enum/rule/hash, does **not**
authorize a chip wave, and does **not** turn on the pipeline. **M1 remains DRAFT;
ADR-0002 remains PROPOSED; `/5` remains current; both overall C/G bundles and the
broad rows C–I remain PENDING.**

**Authority and scope of the rev-45 acceptances (2026-10-05).** Authority: explicit
user instruction (`AGENTS.md` §2, highest precedence), 2026-10-05. The user
self-identifies as holding **all** the named T01/owner roles (T01 `[INT]`,
`[OWNER:T03]`, `[OWNER:T04]`, `[OWNER:T07]`, `[OWNER:T08]`) and responded
**directly in each role**; each acceptance above is recorded with its role label and
the **2026-10-05** date. Scope: **document-level selected subdecisions only on rows
C and G** — the exact ordered `LiteralRecord` fields and M1 enum/scope
(`LiteralKind`/`LiteralSuffix`/radix), the symbolic `Lx08CandidateType` (M1 literals
2/3 = `Int`, no bit width), `RequiredKind` (M1 `IntegerConstantExpression` only),
`ConstLegality` (`Legal`/`NotConstantExpression`/`Unsupported`), and the mandatory
artifact-map invariants. These acceptances do **not** cover the **complete
`Lx08CandidateType` member set/numeric encodings** (deliberately not invented), the
**optional-kind artifact-map rule**, source-versus-payload equality, the exact
artifact error classification/numeric codes, or any `ConstLegality`/`RequiredKind`
numeric encoding — all of which remain open. This record does **not** amend the
Guardrail, does **not** edit `/5`/T01 §4/§5, does **not** add a field/enum/rule/hash,
does **not** authorize a chip wave, and does **not** turn on the pipeline. **M1
remains DRAFT; ADR-0002 remains PROPOSED; `/5` remains current; both overall C/G
bundles and the broad rows C–I remain PENDING.**

**Conditional owner review status:** T02 `conditional`; T03 `conditional`; T04
`conditional`; T05 `conditional`; T06 `conditional`; T07 `conditional`; T08
`conditional`; T09 `conditional`; T13 `conditional`. **None is a sign-off.**

---

## 11. References (relative links)

- [T01: Compiler Bus and Protocol Freeze](T01_COMPILER_CONTRACT.md)
- [T02: Control, Scheduling, and Diagnostic Chips](T02_CONTROL_CHIPS.md)
- [T03: Source Normalization and Preprocessing Chips](T03_PREPROCESS_CHIPS.md)
- [T04: Final C Token and Literal Chips](T04_LEX_CHIPS.md)
- [T05: Taskified Parsing Chips](T05_PARSE_CHIPS.md)
- [T06: Scope, Symbol, and Type Chips](T06_SYMBOL_TYPE_CHIPS.md)
- [T07: Expression, Statement Semantics, and Side-Effect Chips](T07_SEMANTIC_CHIPS.md)
- [T08: Constant, Layout, Initialization, and Dynamic Object Chips](T08_CONSTANT_LAYOUT_INIT_CHIPS.md)
- [T09: AST-to-CFG/IR Chips](T09_IR_LOWER_CHIPS.md)
- [T13: Verification Chips, Replay, and Differential Checking](T13_VERIFICATION_CHIPS.md)
- [Compiler Development Guardrails (accepted)](COMPILER_DEVELOPMENT_GUARDRAILS.md)
- [LLM Parallel Development and Handoff Protocol](PARALLEL_EXECUTION.md)
- [M1 Part A Contract Proposal (rev 39 expected, DRAFT — current; §23 ledger)](M1_PART_A_CONTRACT_PROPOSAL.md)
- [M1 Vertical-Slice Acceptance (frontend half)](M1_VERTICAL_SLICE_ACCEPTANCE.md)
- [M1 Target Acceptance](M1_TARGET_ACCEPTANCE.md)
- [docs/tasks README](README.md)
- [Per-chip Task Template](TASK_TEMPLATE.md)
- [ADR-0001: Compiler Dynamic Arena Extension and Frozen Target Identity](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md)
- [ADR-0002: Deterministic Staged Compiler Pipelines (PROPOSED)](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md)
- [SILICON_PARADIGM_SPEC.md](../architecture/SILICON_PARADIGM_SPEC.md)
- [SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md)
- [SFL_SCHEMA_DRAFT.md](../architecture/SFL_SCHEMA_DRAFT.md)
- [`compiler/contracts/CONTRACT_VERSION`](../../compiler/contracts/CONTRACT_VERSION)
- [`compiler/contracts/COMPILER_SFL_MANIFEST.md`](../../compiler/contracts/COMPILER_SFL_MANIFEST.md)
- [`compiler/README.md`](../../compiler/README.md)

---

## 12. Revision record

| Date | Change | Authority |
|---|---|---|
| 2026-10-04 | Initial `CDR-M1-0001` decision request: required CDR fields; ownership rejection of the `sources.spans` dual-writer carveout; decision requests A–I; span/handoff alternatives; determinism/hash impact; migration/freeze plan; blank acceptance record. **REQUESTED — awaiting acceptance.** No file edited; no code/commit/push; `/5` current; ADR-0002 PROPOSED | M1 review-integration subagent draft for T01 integrator/user decision |
| 2026-10-04 | **Rev 21.** Recorded the **user's in-principle acceptance** (2026-10-04) of the rev-21 working basis for decisions A/B/C direction only: (A) deterministic bounded **sequential** stage pipeline with `quota = 1` semantic-projection baseline and `quota > 1` only after measurement + integrator acceptance; (B) **no shared-writer carveout** — `sources.spans` is **T03-only**, T04/Token reuse committed T03 PP spans with **no span writes**, AST `Node` uses a first/last `TokenId` range with **no T05 span write**, guardrail unamended; (C) T04→T08 typed handoff via a committed T04-owned `LiteralRecord` preserving `node`/`required_kind`/`legality`/`LX08` candidate type as explicit co-freeze fields with **T08 sole `constants.records` writer**. Integrated the rev-20 reviewer findings/audit with concrete corrections: T05 (`awaited` removed in favor of `WaitSet`-only; `ContinuationRef`/`ChildRef` validated pre-apply; `task.continuation` direction; join reinsertion in commit apply; `Progress` stage-queue target), T06 (`Identifier`-leaf `decl`; Block-node boundary `at`; classification of symbol errors; `ChipId` allowlist; TY08 non-M1), T07 (no T09-`FunctionRecord` cycle; `SemRecord` for `Return`/`FunctionDefinition`; one `ConversionPlan` with explicit `sext`/`zext`/`trunc`; VF06 `(NodeKind, role, op)` matrix; separated `const.literal-decode` vs `const.evaluate`), T08 (`max_const_bits` formula/enforcement/hash/test; `M1-NEG-16` chip fix; symbolic Part A vs probe), T09 (explicit `IR28` `TerminatorMissing` trigger; `Constant` immediate = result type; rule id per rejection; `scope_events` in the bus inventory), pipeline (single writer per wire; canonical report; selection-error reclassification; VF04 batch audit as proposed/residual), and T03 (`Preprocessed` not produced by M1; total `requires_map`/source equality/hash/tests). **T01 integrator acceptance and all owner sign-offs remain pending; `/5` current; no code/chip/freeze.** | User in-principle decision (2026-10-04); M1 rev-21 review-integration subagent |
| 2026-10-04 | Added **§13 pending owner amendment requests**: the task packages T03–T09 were **not edited**; the corrections that require their text are recorded as pending owner amendments. No code/commit/push | M1 rev-21 review-integration subagent |
| 2026-10-04 | **Rev 22.** Integrated the accumulated rev-21 read-only review findings (T03/T04, T05, T06, T07, T08, T09, T02/T13 pipeline + ADR-0002) as documentation corrections, explicit blockers, and pending owner amendments; point-by-point ledger in M1 proposal `§17`. CDR changes: header status + `Rev 22` note; dashboard row H stale wording corrected (actual unresolved encodings preserved) with an explicit "A/B/C direction only, D–I pending" note; §3.1/§3.2 delta rows added (`SpanRecord` `u32→u64`, `ArtifactRecord` shape, `sources.span_root`/`expansion` writer boundary, artifact map authority, non-M1 `ArtifactKind` writers, `lex.literals` candidate-fact correction); §C2/§F3/§F4 corrected (lex literal carries no `node`/`required_kind`; VF06 `FunctionDefinition` row no longer duplicates `Return` and VF06 stage is unwired; the two constant requests do not both hand `node`/`required_kind`); §9 sets expanded with the new blockers; §13 adds **T13** and expands T03–T09; closure criteria in §A/§C/§D/§E/§F/§G/§H unchanged in authority but now name the new `§17` blockers. **No acceptance, no `/6`, no code/chip; `/5` current; all owners/integrator pending** | M1 rev-22 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 23.** Integrated the independent rev-22 audit H2–H11 (H1 kept open); point-by-point ledger in M1 proposal `§18`. CDR changes: **H1** — the rev-22 literal-handoff split is now explicitly a **proposed interpretation pending explicit user acknowledgement** + T03/T04/T08/T01 co-freeze (§C2/§C3, header, dashboard row C, §3.2 `lex.literals`, §9); **H2** resume stage `stage_of(parent.kind)` + absent-continuation handling (§D3); **H3** File-scope `Enter` is a T06 task after T05 `TranslationUnit` committed, pinning a committed `NodeId`; bootstrap order a T06/`[INT]` decision (§E2); **H4** all-`Completed`→`Ready`, any-`Failed`→`Failed` once, never `Ready`; sibling policy stays a blocker (§D3); **H5** same-batch `TokenDraft` link branch withdrawn, committed `first_token`/`last_token` only; **H6** terminal capacity/no-`Running` mechanism marked **BLOCKED** (invariant stated, not resolved) (§A); **H7** `folded_int5_part_a_producer`→`folded_int5_ir_constant` with the T08-computes/T09-emits split; **H8** both `ir.op-immediate-type` aliases and both `ir.terminator-missing*` candidates inventoried unresolved; all IR rule/op/field terms made **prospective `/6`**; **H9** `max_inflight_total` phase/owner, residual-set semantics, and dispatcher `Ready→Running` vs ordered atomic commit marked **BLOCKED** (§A); **H10** stale rev-21 references corrected (this §11/§12/§13, ADR-0002, and the M1 proposal) with T13 included as **not edited**, and `AwaitChildren` added to the proposal §2.4 additions + one-transition invariant; **H11** "trigger fixed"→"marker-based trigger direction selected; marker family/schema unresolved" (dashboard row H, §H1). **No acceptance, no `/6`, no code/chip; `/5` current; all owners/integrator pending; H1 requires the user** (superseded by rev 24: the user accepted the H1 allocation in principle) | M1 rev-23 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 24.** Recorded the user's explicit acceptance in principle (`接受拆分（推荐）`, 2026-10-04) of the recommended **H1 literal-handoff allocation split** as the `/6` revision working basis: committed T04-owned `LiteralRecord` = per-literal lexical facts + `LX08` candidate type; post-parse sem-stage `ConstantRequest` = per-use `node`/`required_kind`; `ConstantResult` = `legality`; typed handoff with T08 the sole `constants.records` writer. This is **not** an accepted contract, **not** a `/6` freeze, and **not** code/chip authorization; **T01 integrator acceptance and T03/T04/T08 owner co-freeze/sign-off remain pending**, as do the exact variants/schema (owner/integrator). Updated the CDR header, dashboard (row C + note), §3.2 `lex.literals`, §C2/§C3, §F4, §4, §9, §10, and §13 so no stale "pending user decision" for this split remains; all other blockers (H6/H8/H9, rows D–I, T13) unchanged. **Correction (rev 25 F3):** rev 24 recorded the **allocation**; the distinct proposed `ConstantRequest { literal, node, required_kind }`/`ConstantResult { value, legality }` carrier shapes and the `LiteralRecord.candidate_type` field were **drafted in rev 25**, not edited in rev 24. `/5` current; M1 DRAFT; ADR-0002 PROPOSED | User in-principle decision (2026-10-04); M1 rev-24 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 25.** Integrated the rev-24 independent audit **F1–F9** plus the **verified `/5` failure guarantee**; point-by-point ledger in M1 proposal `§19`. CDR changes: **F1** — present-tense "pinned/hashed" claims in §1, §3.2/§3.3, §C1, §E4, §G2, §H3, §I2, §5, and the dashboard are marked **proposed to be hashed at `/6`**, not hashed today; **F8** — row H and §3.2 state **proposed** per-op rules for the `/6` hash; **F2** — §A5 and the CDR now preserve the frozen `/5` `routing.rs` `fail_selected` single-task obligation and mark the batch fan-out **BLOCKED** (H6; clearing in-flight does not clear `TaskState`; alternatives in proposal §7/§10.5); **F3** — §C2/§F4 resolve the `ConstantRequest { literal, node, required_kind }` ambiguity and §13's T04 row withdraws "without asserting which carrier wins"; **F5** — §9 now states the marker-based trigger *direction* (family/schema unresolved); **F6** — rev 21–23 pointers are rev 21–24; **F9** — T02's "selected contract decisions" and the historical AB1a/AB1b/AB2 "accepted" wording are selected draft direction only. **No acceptance, no `/6`, no code/chip; `/5` current; all owners/integrator pending; H6 remains BLOCKED.** | M1 rev-25 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 26.** Integrated the **rev-25 independent audit items 1–12**; point-by-point disposition is M1 proposal `§20`. CDR changes: **(1, H6)** §A5/§9 now state the verified `/5` `routing.rs` `fail_selected` **single-task** semantics exactly — the failed task **always** becomes `TaskState::Failed`, attaching a committed `DiagnosticId` when capacity allows or the `TaskState::Failed(DiagnosticId::NONE)` sentinel otherwise (never stranded); the **batch** (`quota > 1`) fan-out is **BLOCKED** with `/6` alternatives (generalize the deterministic all-dispatched fail transition with the same optional-diagnostic/sentinel semantics, or another T01-approved atomic terminal path); **(2)** `ConstantRequest` references the committed `LiteralRecord` (proposal §5/§20); **(3)** `LiteralDraft` gains the proposed/unfrozen `candidate_type` for field symmetry; **(4)** the node-token-range test cross-reference is §17.2 (T05), not §17.5, and `node_token_range_same_source` is distinguished from the retired same-batch test; **(5)** remaining present-tense `hashed`/`pinned` claims (proposal §5/§12.17, CDR row H/§3.2/§13) are prospective `/6` only; **(6)** stale rev pointers are rev 25→26 and rev 21–24→rev 21–26 (CDR §11/§13, ADR-0002, proposal §16/§18.7/§19.6); **(7)** the §C closure criterion no longer says owner-accepted shapes follow — **owner acceptance is required and pending**; **(8)** row H/§3.2 say proposed per-op rules **not hashed today**; **(9)** T02/AB1a wording stays selected draft direction, **not accepted**; **(10)** the own-batch key vs Guardrails §6.1 stays an open reconciliation request (not resolved); **(11)** §13/§9 state the `LX08` carrier is **assigned** to `LiteralRecord.candidate_type` (only its exact encoding pending), removing the carrier-unknown wording; **(12)** H11 and rev pointers rechecked. **No acceptance, no `/6`, no code/chip; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending.** | M1 rev-26 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 27.** Applied the **rev-26 audit corrections** (doc-only; no acceptance-status change). **(1)** §12 rev-26 history item (6) corrected to `rev 25→26 and rev 21–25→rev 21–26` (it had said `rev 24→25 and rev 21–24→rev 21–25`, duplicating the rev-25 F6 pattern and off by one); all operative CDR pointers verified at rev 21–26. **(2)** §13 now states precisely which documents were updated through which revision: M1 proposal/CDR/ADR-0002/M1 target acceptance through rev 26; T02 and the M1 frontend acceptance through rev 25 (rev 26 made no substantive change there). **(3)** §2 dashboard note renamed `Rev 23–26` and extended with the §19 (rev 25) and §20 (rev 26) ledgers. **(4)** M1 proposal §20.9 D1 cross-reference corrected from the wrong `Finding 2` to the T05 continuation/own-batch items (§17.2/§18.3/§19.4, CDR §D1). **(5)** M1 proposal §20.10 residual register now lists the **H11** `CompletedFunction` marker-family blocker. **(6)** Residual unqualified `frozen` wording at proposal stage ordinals, the versioned contract input, and the `NamePlan`/`ResolveTable`/`RecordDraft` interface is qualified as proposed/selected draft, not a freeze. **No acceptance, no `/6`, no code/chip; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending.** | M1 rev-27 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 28 (docs-only pointer reconciliation; no acceptance, schema, interface, task-kind, chip, or `/6` change).** Corrected the §12 rev-26 history item (6) before→after range to `rev 21–24→rev 21–26` (the rev-27 row had recorded `rev 21–25→rev 21–26`, off by one on the before-state; the rev-25 F6 `rev 21–23→rev 21–24` remains the correct predecessor). Advanced every current-state M1 pointer/range to **rev 21–28**: the header `Related requests` link and §11 proposal link (rev 26 → rev 28), this §2 dashboard note title/ledger (`Rev 23–26` → `Rev 23–28`), and §13; the M1 proposal header/§16/§18.7/§19.6/§20.6; and ADR-0002 §1.1 (Revision 9). The correction changed the §12 rev-26 history row, while the historical rev-27 row is preserved verbatim and other historical rows are unchanged; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending. | M1 rev-28 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 29 (docs-only historical-documentation correction; no acceptance, schema, interface, task-kind, chip, or `/6` change).** Corrected the rev-28 rows' "historical revision rows are unchanged" claim: rev 28 did correct the §12 rev-26 history row; the historical rev-27 row is preserved verbatim, and other historical rows are unchanged (this CDR's §12 rev-28 row and the M1 proposal §16 rev-28 row are corrected in place). Recorded the ADR-0002 Revision 6 (rev-23) H10 chronology erratum: rev 23 corrected the pointer to **rev 21–23**, with rev 24/25 extending it to **rev 21–24** (ADR-0002 Revision 10; the Revision 6 text is preserved). Advanced every current-state M1 pointer/range from **rev 21–28** to **rev 21–29**: the header `Related requests` link and §11 proposal link (rev 28 → rev 29), this §2 dashboard note title/ledger (`Rev 23–28` → `Rev 23–29`), and §13; the M1 proposal header/§16/§18.7/§19.6/§20.6; and ADR-0002 §1.1 (Revision 10). `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending. | M1 rev-29 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 30 (docs-only scope/label correction; no acceptance, schema, interface, task-kind, chip, or `/6` change).** Corrected the M1 target acceptance §7 failure-guarantee label from "Rev 26 F2" to **Rev 25 F2** (the H6/sentinel paragraph was integrated at rev 25 and rev 26 did not alter §7), and recorded that correction as the target-acceptance **rev 27** history row. §13's per-document revision scope now states the M1 target acceptance was updated through **rev 27**. No current-state pointer advances: the M1 proposal remains **rev 29** (DRAFT) and the CDR's `/6` working basis is unchanged. `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending. | M1 rev-30 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 31 (records two further explicit user in-principle decisions; no acceptance of a contract, schema, interface, task-kind, chip, or `/6` freeze).** **(A, Guardrails §6.1 narrow interpretation)** the user accepts in principle that a same-task `OwnBatch(DraftRef)` may exist only as a **transient wire/proposal input before commit**, and that commit validates/resolves it to a committed `TaskId`/record before any persistent state; no durable cursor/`WaitSet`/join may point at a draft, wire, or address; the guardrail text is **unamended** and **T01 integrator implementation-confirmation is pending**. **(B, awaited children)** the user accepts in principle that `TaskState::Waiting(WaitSet)` is the **sole** awaited-child-ID source and `ContinuationRecord` carries no duplicate `awaited`; the request to **supersede/clarify T01 §4 in `/6`** is accepted in principle with **no `/5` edit**, and **T01 integrator + T05 owner acceptance remain pending**. CDR edits: header status; §2 dashboard rows A/D and the `Rev 23–31` note; §3.1 `TaskDraft.continuation`; §A status honesty; §D1/§D2 (narrow interpretation replaces "candidate-only/open reconciliation"), §D4 (B), §D closure criterion; §I4 (T01 §4 inventory includes B); §9; §10 narrative/rows/authority; §11/§12/§13. **Row D stays PENDING overall** (exact fields/encoding, CT07 carrier, sibling policy). Batch **H6/H9**, **CT07**, **H8/H11**, all other rows, and every exact-shape blocker are unchanged. M1 proposal at **rev 30**, ADR-0002 at **Revision 11**, T02 at **rev 26**; M1 target/frontend acceptance unchanged. `/5` current; M1 DRAFT; ADR-0002 PROPOSED; H1 accepted in principle only; all T01/owner sign-offs pending; no freeze/chip authorization. | User in-principle decisions (2026-10-04); M1 rev-31 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 32 (docs-only history-gap repair; no acceptance, schema, interface, task-kind, chip, or `/6` change).** Repairs the §12/§16 history gap identified by the rev-31 audit (**F1/F2**). **(F1)** The M1 proposal §16 revision record had jumped from **Rev 28** to **Rev 30** while the header/§18.7/§19.6/§20.6 and this CDR's §12 rev-29 row recorded that **rev 29** was an operative docs-only historical-documentation correction that updated the proposal §16; the M1 proposal **rev 31** now supplies the missing historical **Rev 29** row and adds its **Rev 31** row. **(F2)** Rewrote the **§13** opening to disambiguate the separate revision counters: **M1 proposal rev 30** = the revision that records the user's in-principle decisions A/B, whereas **CDR rev 30** = the docs-only scope/label correction and **CDR rev 31** = the A/B decision record; §13 now also states the proposal is at **rev 31** and the CDR at **rev 32**. Advanced every current-state M1 pointer/range from the proposal pointer **rev 30** (`rev 21–30`) to the current **rev 21–31**: this CDR's header `Related requests` link and §11 proposal link (rev 30 → rev 31), the §2 dashboard note (`Rev 23–31` → `Rev 23–32`; current M1 candidate **rev 31**), and §13; the M1 proposal header/§16/§18.7/§19.6/§20.6; and ADR-0002 §1.1 (**Revision 12**, rev 21–30 → **rev 21–31**). Historical **Rev 30** and **Rev 31** rows above are preserved verbatim; no acceptance status changes. `/5` current; M1 DRAFT; ADR-0002 PROPOSED; H6/H9 BLOCKED; H1 accepted in principle only; all T01/owner sign-offs pending; no freeze/chip authorization. | M1 rev-32 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 33 (docs-only audit-defect correction; no acceptance, schema, interface, task-kind, chip, or `/6` change).** Fixes the one **rev-31/32 audit defect** in **§13** and the related counter ambiguity. **(1)** §13's M1 target-acceptance paragraph incorrectly lumped the CDR rev 30/31 corrections with the history-gap repair as having "made no change" to the M1 target acceptance, contradicting the §12 rev-30 row (which *did* change it: the §7 failure-guarantee label "Rev 26 F2" → **Rev 25 F2**, recorded as the target acceptance's **rev 27**). The "made no change" claim is now restricted to **CDR rev 31** and the **rev-32 history-gap repair**, and §13 states explicitly that **CDR rev 30 did change the M1 target acceptance, recorded as its rev 27**. **(2)** The §2 dashboard note now labels the bare revision numbers by document (**M1 proposal rev 31** vs **CDR rev 31**) and extends the same-number-counter warning to rev 31 (and the new rev 33). **(3)** Adds this **Rev 33** history row and the corresponding §2 note text; the current M1 proposal stays **rev 31** and the CDR becomes **rev 33**. §13 now reports the accurate current counters: M1 target acceptance **rev 27**, T02/frontend **rev 25**, ADR-0002 **Rev 12/rev 31**, M1 proposal **rev 31**, CDR **rev 33**. No current-state M1 pointer advances (**rev 21–31** unchanged); no task package is edited. A/B/H1 remain in-principle only; row D remains PENDING; `/5` current; all approvals pending; no freeze/chip authorization. | M1 rev-33 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 34 (records a new explicit user in-principle decision, direction only; no acceptance of a contract, schema, interface, task-kind, chip, or `/6` freeze).** Records the user's 2026-10-04 **in-principle acceptance of the H6 batch-failure recovery direction**: after an atomic semantic batch commit fails, transition **every** dispatched task, in **dispatch order**, **exactly once** to `TaskState::Failed` — a committed `DiagnosticId` when diagnostic/record capacity allows, else the `TaskState::Failed(DiagnosticId::NONE)` sentinel — so **every** dispatched task leaves `Running`; **clearing the in-flight set is not itself a transition**. This **generalizes the verified `/5` `fail_selected` single-task sentinel obligation**, which is preserved and **not weakened**; the alternative (another T01-approved atomic terminal path, e.g. reserve bounded terminal diagnostic slots before dispatch) is retained as an implementation option. It is a `/6` **working-basis direction only**: the **exact atomic/bounded implementation requires T01 integrator approval**, and the batch failure-atomicity details, the diagnostic budget/state mechanism, and the **T02/T13 owner fixtures/sign-offs remain co-freeze and pending**; the mechanism is **not** implemented or frozen. CDR edits: header status; §2 dashboard note (title/ledger + row A + counter prose); §A5; §A status honesty; §9 H6 (+ the T05 terminal-capacity item); §10 narrative/row A/authority; §11/§12/§13. H6 moves from "no selected alternative" to "direction accepted in principle, implementation BLOCKED pending T01"; **H9 stays BLOCKED**; A/B/C and H1 remain in-principle only; **D–I remain PENDING** (row D stays PENDING overall); every current-state M1 pointer/range advances from **rev 21–31** to **rev 21–32**; M1 proposal at **rev 32**, ADR-0002 at **Revision 13 (rev 32)**, T02 at **rev 27**; the CDR becomes **rev 34**. `/5` current; M1 DRAFT; ADR-0002 PROPOSED; all T01/owner sign-offs pending; no freeze/chip authorization. | User in-principle decision (2026-10-04); M1 rev-34 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 35 (records a new explicit user in-principle decision, direction only; no acceptance of a contract, schema, interface, task-kind, chip, or `/6` freeze).** Records the user's 2026-10-04 **in-principle acceptance of the H9 in-flight scheduling-ownership direction**: accept **removing `max_inflight_total`** from the `/6` candidate because sequential per-tick dispatch is already bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; `Waiting` tasks are not in-flight; and a future cross-tick `Running` mode would need a **separate CDR**. The dispatch batch bound remains the per-tick `max_inflight_per_tick` quota; `tasks.in_flight` remains an **ephemeral per-tick scheduler batch only** (candidate), populated from the tick's `SelectionBatch` and cleared at latch **only after every dispatched task has a terminal/`Waiting`/`Progress` outcome (or the H6 recovery has transitioned every dispatched task exactly once)**; the dispatcher's pre-worker `Ready→Running` mutation is **distinguished from** the one ordered atomic semantic commit; the **H6 recovery semantics are retained**. It is a `/6` **working-basis direction only, pending T01 integrator acceptance**: `/5` is unchanged and no bound/config/hash is frozen. CDR edits: header status; §2 dashboard note (title/ledger + row A + counter prose); §A5; §A status honesty (candidate-error list; H9); §9 H9 (+ the §9 pipeline item); §10 narrative/row A/authority; §11/§12/§13 (incl. the [T02](T02_CONTROL_CHIPS.md) owner-amendment row, T02 rev 28). H9 moves from "BLOCKED (phase/owner undefined)" to "removal direction accepted in principle, pending T01 integrator acceptance"; the remaining H9 `[INT]` items (the dispatcher-`Ready→Running`-vs-ordered-atomic-commit relationship and the residual-set semantics) stay open. H6 stays "direction accepted in principle, implementation BLOCKED"; A/B/C and H1 remain in-principle only; **D–I remain PENDING** (row D stays PENDING overall); every current-state M1 pointer/range advances from **rev 21–32** to **rev 21–33**; M1 proposal at **rev 33** (its own rev-33 H9 record), ADR-0002 at **Revision 14**, T02 at **rev 28**; the CDR becomes **rev 35**. `/5` current; M1 DRAFT; ADR-0002 PROPOSED; all T01/owner sign-offs pending; no freeze/chip authorization. | User in-principle decision (2026-10-04); M1 rev-35 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 36 (docs-only D3 correction; no acceptance, schema, interface, task-kind, chip, or `/6` change).** Corrects the §13 claim that the [M1 frontend acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md) carried a rev-26 H6/CT06 text: the frontend is at the latest **Rev 25** and contains **no H6/CT06 content**, and **[T02](T02_CONTROL_CHIPS.md)** is where the operative H6 batch-failure text lives (its rev-25 H6/sentinel bullet and rev-27 H6-direction bullet, with the H9 addition at T02 rev 28). The frontend therefore requires no H6/CT06 amendment. This also records the completion of the H9 cross-document integration in the CDR (rev 35), ADR-0002 (**Revision 14**) and T02 (**rev 28**), matching the M1 proposal's rev-34 record. Every current-state M1 pointer/range advances from **rev 21–33** to the current **rev 21–34** (this CDR's header/§2/§11/§13); M1 proposal at **rev 34**, ADR-0002 at **Revision 14**, T02 at **rev 28**; the CDR becomes **rev 36**. A/B/C and H1 remain in-principle only; H6/H9 are directions only; **D–I remain PENDING** (row D stays PENDING overall). `/5` current; M1 DRAFT; ADR-0002 PROPOSED; all T01/owner sign-offs pending; no freeze/chip authorization. | M1 rev-36 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 37 (docs-only consistency cleanup of stale active candidate prose; no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change).** Corrects the two remaining H9 contradictions in the CDR where the **current operative** error-carrier lists still named `InflightQuotaExceeded` as an active candidate despite the rev-35 removal direction: the **§A9** dispatcher/scheduling bullet and the **§I** dispatcher/scheduling bullet. Both now state `InflightQuotaExceeded` is **removed from the candidate** with `max_inflight_total` (retained only as the historical rev-21/22 name) and the per-tick dispatch bound stays `SelectionBatchOverflow` on `max_inflight_per_tick`. §3.3's Limits row, §A5, §9, §10 row A, and the §13 T02 row already carried the removal direction and are unchanged. Every current-state M1 pointer/range advances from **rev 21–34** to the current **rev 21–35** (this CDR's header/§2/§11/§13); M1 proposal at **rev 35**, ADR-0002 at **Revision 15**, T02 at **rev 29**; the CDR becomes **rev 37**. **H6/H9 statuses unchanged:** H6 = direction accepted in principle, **implementation BLOCKED**; H9 = removal direction accepted in principle, **pending T01 integrator acceptance**, with the residual-set and dispatcher-mutation-vs-commit `[INT]` items open; A/B/C and H1 remain in-principle only; **D–I remain PENDING** (row D stays PENDING overall). `/5` current; M1 DRAFT; ADR-0002 PROPOSED; all T01/owner sign-offs pending; no freeze/chip authorization. | M1 rev-35 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 38 (docs-only lead-in/typo correction; no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change; matches M1 proposal rev 36 and ADR-0002 Revision 16).** Corrects the §13 opening lead-in, which said the **M1 proposal was updated through rev 34** while its own list continued through rev 35: it now reads **rev 36**. Fixes the §13 `per-stage stage queues` typo (now `per-stage queues`) and cross-references the M1 proposal rev-36 header current-state pointer and its typo fixes. The current-state M1 pointer/range **stays rev 21–35** (the range is unchanged; only the M1 proposal document revision advances to rev 36); the CDR becomes **rev 38**. This is the CDR side of the same docs-only audit correction as the M1 proposal rev 36, whose other half is the ADR-0002 **Revision 16** §3 `bus.rs` amendment-row clarification (the preflight bound is the **per-tick `max_inflight_per_tick`/quota only**, with no separate total in-flight bound). **H6/H9 statuses unchanged:** H6 = direction accepted in principle, **implementation BLOCKED**; H9 = removal direction accepted in principle, **pending T01 integrator acceptance**, with the residual-set and dispatcher-mutation-vs-commit `[INT]` items open; A/B/C and H1 remain in-principle only; **D–I remain PENDING** (row D stays PENDING overall). `/5` current; M1 DRAFT; ADR-0002 PROPOSED; all T01/owner sign-offs pending; no freeze/chip authorization. | M1 rev-36 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 39 (docs-only addition of the proposed `/6` co-freeze work queue; no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change; no task package, proposal, or ADR edited; matches M1 proposal rev 36 and ADR-0002 Revision 16).** Adds **§9A**, a concise proposed `/6` co-freeze work queue grounded only in the six completed read-only reports already integrated (H1/row C, T05/D, pipeline T02/T13, T06/E, T07/F, T09/H), whose point-by-point disposition is the M1 proposal §17–§21 and whose detailed owner rows remain the §C–§I/§13 source of truth. §9A labels **all contents as proposed questions/authority assignments, not accepted schema**, and states no new user decision is made or required. It records: **(1)** Gate 1 — before any chip code, T01 must freeze the shared types/task/result/schema, after which the repo's [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) **Wave 1** may dispatch **only the assigned chips**; **(2)** Gate 2 — **Wave 2** depends on **real upstream contracts/artifacts** (fixtures/mocks do not substitute); **(3)** Gate 3 — a full M1 **end-to-end executed run** additionally needs a **Linux probe substrate/toolchain/assembler/linker/sysroot/runner** and remains a **separate Part B gate** (none claimed available; Part A is symbolic and probe-independent). It separates the genuinely parallel owner-drafting/review tracks (**C+E**, **A/T13**, and H's rule inventory) from the serial freeze dependencies (T01 consolidates and hashes only after upstream owner shapes are accepted; coding follows the frozen types; Wave 2 follows upstream outputs), and gives the **minimum work items A–I** with **exact authority** (**A/T02/T13** H6 bounded atomic recovery incl. pre-dispatch-vs-semantic-commit error paths + H9 exact mutation boundary/sole writers/residual-inflight exhaustive paths/limit-stage-hash-error numeric inventory + fixtures/CT07/sibling policy; **C/T03/T04/T07/T08** `ArtifactRecord`/map, `LiteralRecord`/`LX08`, `ConstantRequest`/`Result`/legality, `RequiredKind`, `max_const_bits` carrier, writer allowlist, `M1AppendSchema` hash/tag inventories; **D/T05/T01/T02** `ContinuationRecord` wire shape/tags, B supersession T01 §4, A-narrow preapply checks/error codes, CT07 carrier or remove, sibling-failure policy, parse contract/tests; **E/T06/T01/T05** scope record lifecycle/bootstrap after committed TU, decl/lookup tie semantics, canonical `TypeId` reuse mechanism, namespace mapping/manifest allowlist and tests; **F/T06/T07/T09/T13** `FunctionContextId` conflict, `SemRecord`==`CheckedNode` typed T09 link, `ConversionOp`/`Role`/permitted matrix/IR emission, VF06 binding, `EffectMask`/`ValueCategory` encodings, diagnostics/rules/tests; **G/T08/T01** `max_const_bits` bound/carrier/diagnostic enforcement and Part-A symbolic vs probe-gated target bits; **H/T09/T01/T13/T08** `CompletedFunction` marker family/store/arena/tag/encoder, rule-alias collapse, canonical `Constant` validation precedence, hash and verifier fixtures; **I/T01** one `/6` envelope, `M1AppendSchema` vs `COMPILER_SFL_MANIFEST.md` §4 hash-source contradiction, IDs/tags/`RecordFamily` ordinal counts/self-consistency, and all T01 §4 supersessions catalogued). Every item is **proposed**; a model/subagent review does **not** count as an owner or T01 sign-off. Updated the header status cell, the §2 dashboard note title/ledger (`Rev 23–38` → `Rev 23–39`), the header `Related requests` and §11 proposal links (rev 35 → **rev 36**, matching the actual shared proposal revision), and §13 (the CDR's own list and the render-time line: **rev 38 → rev 39**). The current-state M1 pointer/range **stays rev 21–35** and the shared M1 proposal **stays rev 36**; no task package is edited. H6/H9 statuses unchanged (H6 = direction accepted in principle, implementation BLOCKED; H9 = removal direction accepted in principle, pending T01 integrator acceptance, residual-set/dispatcher-mutation-vs-commit `[INT]` items open); A/B/C and H1 remain in-principle only; **D–I remain PENDING** (row D PENDING overall); `/5` current; M1 DRAFT; ADR-0002 PROPOSED; all T01/owner sign-offs pending; no freeze/chip authorization. | M1 rev-39 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 40 (docs-only addition of the proposed decision-ready recommendation appendix; no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change; no task package, proposal, or ADR edited; matches M1 proposal rev 36 and ADR-0002 Revision 16).** Adds **§9B**, inserted after §9A and before §10 without renumbering any existing section. §9B is a **proposed-default decision aid**: per row/authority it states one concise **recommendation** (accept/amend/reject) plus the single **tradeoff/open dependency**, grounded only in the six completed read-only reports already integrated (H1/row C, T05/row D, T02/T13 rows H6–H9, T06/row E, T07/row F, T09/row H), whose detailed owner rows remain the §C–§I/§13 source of truth. It records: **C/H1** — lexical-facts + symbolic `LX08` `LiteralRecord`, ref-based `ConstantRequest = { literal: RecordRef::Literal, node, required_kind }` and `ConstantResult` `legality`, defer target widths/multi-source/Part B writers, unresolved `RequiredKind`/legality-carrier/artifact-map/`max_const_bits`, with T01 owning the `RecordRef`-tag vs `RecordFamily`-ordinal inventories/hash; **D/T05** — continuation fields per §D1 (cited, not re-listed), no `awaited`, commit-apply join invariant with no new CT07 family, await-all-children-then-parent, both fail-fast-vs-await-all and CT07-carrier flagged as open T02/T05 choices, parse depth via `max_task_depth`/parser frames; **A/T02/T13** — recommend the quota-1 M1 baseline preserving the `/5` single-task `fail_selected`/sentinel and the dispatcher `Ready→Running` sole writer with explicit empty-inflight/empty-proposal paths + tests, and **flag the recommendation to defer the multi-task H6 mechanism to a separate CDR as a material scope decision T01 must explicitly accept and reconcile with — and not erase — the user's accepted-in-principle H6 direction** (not stated as decided); stage/limits/hash/error-code remain T01 decisions; **E/T06** — `Identifier`-leaf `decl`, File `Enter` only after the committed TU via deterministic `parse`→`symbol_type` scheduling, derived namespace, chip-id allowlist, bounded committed-arena canonical-`TypeId` lowest-id scan with no cache, with explicit upstream T05 dependency and T01 bootstrap acceptance; **F/T07** — no `FunctionContextId` (requires a T07 package amendment), `SemRecord` = committed `CheckedNode` + explicit committed typed T09 link, conversion matrix co-frozen T06/T07/T09 including `FloatToFloat`/qualifier domain as a proposal (not silently frozen), VF06 bound or explicitly out of M1 by T07/T13, empty-only `EffectMask`, `ValueCategory` pinned only with T07 acceptance; **H/T09** — `Constant`/`Add`/`Return` op-table proposal, shorter-id alias choices, immediate-before-result validation, T08-computes-folded-`int5`/T09-emits-IR, and marker options left **unresolved with the absent basis stated** (no family selected); VF fixture amendments are T13 work; **I/T01** — serialize only after owner acceptance, reconcile the `M1AppendSchema` vs `COMPILER_SFL_MANIFEST.md` §4 hash-source contradiction, explicit numeric tags vs family ordinals, count/self-consistency tests. §9B **refers** to the §9A gates (Gates 1–3) instead of duplicating them, states every recommendation is **proposed** for the named authority, and states explicitly that **no new user decision is made or required**, no recommendation is a sign-off/freeze/implementation authorization, and the accepted-in-principle H6 direction is **referenced, not modified or withdrawn**. Updated the header status cell (appended a rev-40 sentence), the §2 dashboard note title (`Rev 23–39` → `Rev 23–40`) and its ledger + current-CDR-pointer prose (rev 39 → **rev 40**; §9B noted), the bare-counter list (`…/39` → `…/39/40`), and §13 (the CDR's own list and the render-time line: **rev 39 → rev 40**). The current-state M1 pointer/range **stays rev 21–35** and the shared M1 proposal **stays rev 36**; no section renumbered; no task package is edited. H6/H9 statuses unchanged (H6 = direction accepted in principle, implementation BLOCKED; H9 = removal direction accepted in principle, pending T01 integrator acceptance, residual-set/dispatcher-mutation-vs-commit `[INT]` items open); A/B/C and H1 remain in-principle only; **D–I remain PENDING** (row D PENDING overall); `/5` current; M1 DRAFT; ADR-0002 PROPOSED; all T01/owner sign-offs pending; no freeze/chip authorization. | M1 rev-40 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-04 | **Rev 41 (docs-only addition of the proposed per-decision response matrix; no new decision, no acceptance, schema, interface, task-kind, chip, or `/6` change; no task package, proposal, or ADR edited; matches M1 proposal rev 36 and ADR-0002 Revision 16).** Adds **§9C**, inserted after §9B and before §10 without renumbering any existing section. §9C is a **procedural response form only, not schema**: per decision bundle (**T01-global** `/6` envelope/hash/source-of-truth/tag-vs-ordinal/self-consistency + T01 §4 supersessions; **C/H1** T03/T04/T07/T08 + `[INT]`; **D** T05/T02 + `[INT]` incl. `WaitSet` `/6`, `OwnBatch`, CT07, sibling policy; **A** T02/T13 + `[INT]` incl. `quota = 1` and the explicit H6 scope question; **E** T05/T06 + `[INT]`; **F** T06/T07/T09/T13 + `[INT]`; **G** T08 + `[INT]`; **H** T09/T08/T13 + `[INT]` incl. the undecided marker) it names the **exact authority** and permits `Accept recommendation / Amend (state replacement) / Reject (reason) / Defer (dependency/date)`, with a **`Status / signature`** cell that stays **`PENDING`** until that named human authority fills it. **Correction to the same rev-41 procedural form (review fix, no new revision):** because a bundle row may list several authorities while holding one response/signature cell, §9C now requires **one sub-entry per listed authority** in the `Response` and `Status / signature` cells (**`Authority — disposition — date`**), states that **every** listed human authority must **independently** respond/sign for **its own** acceptance and that **no group or single-owner response represents another**, keeps a row **`PENDING`** until **all** listed required authorities (and T01 `[INT]` where listed) have **separately** signed, offers **duplicate-row-per-authority** as a fallback without inflating the table by default, and affirms that T01 records acceptance in `/6` **only after** every owner signoff (T01's own `[INT]` being a separate sub-entry). It **cites §9A/§9B and the detailed §C–§I rows** instead of duplicating technical content, requests **no new user decision**, and presumes no outcome on the H6 scope question (bundle A: any deferral requires explicit T01 acceptance; if it would change the user's in-principle H6 direction it returns to the user). Protocol: only the named human authority may fill its status/signature; a reviewer/model analysis never counts; T01 records accepted shapes in `/6` only after the owning `[OWNER:*]` + `[INT]` sign-offs; until then **`/5` is current and no code**. Rows **D–I remain PENDING** with no implied sign-off. Edits: this §12 row; the header status; the §2 dashboard ledger/counter note (rev 41, CDR at rev 41) and the render-time line: **rev 40 → rev 41**; §9C; and §13. The current-state M1 pointer/range **stays rev 21–35** and the shared M1 proposal **stays rev 36**; no section renumbered; no task package is edited. H6/H9 statuses unchanged (H6 = direction accepted in principle, implementation BLOCKED; H9 = removal direction accepted in principle, pending T01 integrator acceptance); A/B/C and H1 remain in-principle only; **D–I remain PENDING** (row D PENDING overall); `/5` current; M1 DRAFT; ADR-0002 PROPOSED; all T01/owner sign-offs pending; no freeze/chip authorization. | M1 rev-41 docs-consistency subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 42 (records the user's explicit per-subdecision authority decisions of 2026-10-05; no accepted contract, no `/6` freeze, no code/chip authorization; no task package, proposal, or ADR edited; matches M1 proposal rev 36 and ADR-0002 Revision 16).** The **user** — self-identifying as holding **all** named T01/owner roles (T01 `[INT]`, `[OWNER:T02]`, `[OWNER:T05]`, `[OWNER:T06]`, `[OWNER:T07]`, `[OWNER:T09]`, `[OWNER:T13]`) and responding **directly in each role** — explicitly accepted the following **selected subdecisions only**, each recorded with role label and **2026-10-05** date: **(1) A/T02/T13/T01 — H6 scope + mechanism.** The user preserves the in-principle direction and chooses to **freeze a quota>1-capable bounded batch recovery mechanism in `/6`**, explicitly **rejecting** the §9B row-A proposal to defer the H6 mechanism to a later CDR. Selected mechanism: pre-dispatch errors before state mutation leave tasks `Ready`; a semantic batch commit failure commits **no** semantic state; a **deterministic bounded recovery mutation** then processes the dispatched tasks **once in dispatch order** to `Failed`; **no pre-reservation of N diagnostics**; a **per-task diagnostic attempt** using `DiagnosticId::NONE` when capacity is insufficient; a **state guard** prevents the duplicate transition. This is an explicit selected H6 mechanism, but does **not** claim the whole A bundle, the H9 limits/stages/hash/error inventories, or any fixtures complete. **(2) D/T02/T05/T01:** accept the join as a **commit-apply invariant** with **no new CT07 committed carrier/family**, and **await-all children terminal before the parent decision**; continuation fields/wire encoding, the `WaitSet` `/6` formal T01 §4 supersession, `OwnBatch` exact preapply errors, parse contract/tests, and the rest of D remain **PENDING**. **(3) F/T06/T07/T09/T01:** accept **no `FunctionContextId`**; `SemRecord` is the committed materialization of `CheckedNode`, **one per `NodeId`**, with an **explicit committed typed link consumed by T09**; conversion matrix, VF06/T13, effects/value category, diagnostics/tests, and the rest of F remain **PENDING**. **(4) E/T06/T01 (T05 upstream dependency acknowledged):** accept a deterministic bounded scan of committed `types.records` returning the **lowest matching `TypeId`**, **no hidden cache/index**; bootstrap and the rest of E remain **PENDING**. CDR edits: header status; §2 dashboard note (ledger/counter prose, bare-counter ambiguity, rev-42 paragraph); §A5 and §A status honesty (H6 scope+mechanism selected); §D3/§D closure; §E4; §F1/§F closure; §9 (H6, T05, T06, T07 items); §9A (row A H6 scope decided + D/E/F accepted subdecisions; authority-honesty note); §9B (row A deferral marked historical/rejected; **same-rev-42 consistency correction:** the §9B introductory no-status-change/all-signoffs-pending statement is qualified as rev 40's then-current state with rev 42 recording only named partial subdecisions and overall rows D–I still pending; rows **E**/**F** now explicitly mark their rev-42 selected subdecisions and keep bootstrap/other E items and conversion/VF06/effects/etc. as proposal/open; the §9B closing now describes the D/F/E selected subdecisions as marked in rows D/E/F above with the overall rows D–I still pending; rows **A**/**D** historical/superseded wording verified correct; closing note); §9C (per-subdecision acceptance support + D/A/E/F rows); §10 (rev-42 narrative + rows A/D/E/F + rev-42 authority/scope paragraph); §11/§12/§13. **All unselected proposals remain proposal-only; every overall bundle status stays PENDING where other items are unresolved; no row is closed wholesale; `/5` remains current; M1 DRAFT; ADR-0002 PROPOSED; broad rows D–I remain PENDING; no chip code.** The current-state M1 pointer/range **stays rev 21–35** and the shared M1 proposal **stays rev 36**; this CDR becomes **rev 42**. | User authority decisions (2026-10-05); M1 rev-42 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 43 (records further explicit user subdecision decisions of 2026-10-05 on rows D and E only; no accepted contract, no `/6` freeze, no code/chip authorization; no task package, proposal, or ADR edited; matches M1 proposal rev 36 and ADR-0002 Revision 16).** The **user** — self-identifying as holding **all** named T01/owner roles and responding **directly in each role** — accepted these **selected subdecisions only**, each recorded with role label and **2026-10-05** date: **(1) D/T05 — continuation exact shape.** Accept the candidate `ContinuationRecord` **exact ordered fields/shapes** (`production: TaskKind`, `cursor: TokenId`, `context: ParseContext`, `binding_power: u16`, `scope: Option<ScopeId>`, `parent: Option<NodeId>`, `partial_children: Vec<NodeId>`, `next_child_ordinal: u32`, `previous: Option<ContinuationId>`), with **no `awaited`** and durable references to committed IDs only; the `RecordRef` numeric wire tags and `RecordFamily` ordinals remain **separate inventories** and their exact numeric encodings remain a **T01 `/6` freeze detail**, while the `ParseContext` vocabulary still needs the T05/T01 final encoding. **(2) D/T01/T05 — T01 §4 supersession.** Formally accept, in the `/6` inventory, the supersession of the T01 §4 old continuation-`awaited` wording: `TaskState::Waiting(WaitSet)` is the **only** awaited-child source, the continuation has **no `awaited`**, and **`/5` is unchanged** (T01 §5 unchanged; no `/5` edit). **(3) D/T02/T05/T01 — OwnBatch pre-apply.** Accept the exact pre-apply proposal: a committed continuation exists/live; an OwnBatch continuation index lies within the same task's `AppendRecords` range and family `Continuation` (phase 1); a committed child is a valid committed child; an OwnBatch child index lies within the same task's Enqueue list (phase 2b, after the task's own Enqueues are known); errors `ContinuationRefInvalid`/`AwaitChildrenRefInvalid`, whole-batch `CommitError`, before any mutation (numerical codes/hash remain T01 `/6` details). **(4) E/T06/T01 — File Enter trigger.** Accept the File Enter trigger as a deterministic `parse.TranslationUnit -> symbol_type.scope-enter` stage edge **after** the committed TU, carrying the committed `NodeId`, with **no job-bootstrap**; T05's actual committed-TU carrier remains an **upstream pending dependency** and acceptance of the stage-edge direction is **not a TU schema signoff**. CDR edits: header status; §2 dashboard note (ledger/counter prose, bare-counter ambiguity, rev-43 paragraph) and dashboard rows D/E; §9A rows D/E; §9B rows D/E; §9C per-subdecision support + D/E rows; §10 rev-43 narrative + rows D/E + rev-43 authority/scope paragraph; §12 (this row) and §13. **Every overall D/E bundle stays PENDING** where other items are unresolved (the `RecordRef` tag values/`RecordFamily` ordinals distinct-inventory note implies no accepted tag values; `ParseContext` enum, encoder/wire numeric encodings, OwnBatch error numeric codes/hash, parse request/result/test shapes, and the T05 committed-TU upstream carry all remain pending); the unselected proposals remain proposal-only; no row is closed wholesale; **no code and no `/6` freeze is authorized**; `/5` remains current, M1 DRAFT, ADR-0002 PROPOSED. The current-state M1 pointer/range **stays rev 21–35** and the shared M1 proposal **stays rev 36**; this CDR becomes **rev 43**. **Same-rev43 correction (no new revision/pointer):** the header Status opening's blanket "T01 integrator acceptance and **all** owner sign-offs remain pending" and §9B's "does **not** ... change any acceptance status" (and §9A's blanket "`[OWNER:*]` entries are pending human sign-offs") read as pre-rev-42 current truth; they are now qualified as **as-of rev 40/41 historical**, with the current truth stated: selected subdecisions carry authority responses recorded in **rev 42/43** (§9C/§10), while the remaining full-bundle sign-offs (T02–T09, T13) and the overall rows D–I remain **pending**; `/5` remains current, M1 DRAFT, ADR-0002 PROPOSED, no code and no `/6` freeze. | User authority decisions (2026-10-05); M1 rev-43 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 44 (records further explicit user subdecision decisions of 2026-10-05 on rows C and G only; no accepted contract, no `/6` freeze, no code/chip authorization; no task package, proposal, or ADR edited; matches M1 proposal rev 36 and ADR-0002 Revision 16).** The **user** — self-identifying as holding **all** named T01/owner roles and responding **directly in each role** — accepted these **selected subdecisions only**, each recorded with role label and **2026-10-05** date: **(1) C/H1/T07/T08/T01 — `required_kind` meaning + `legality` placement.** `ConstantRequest.required_kind` denotes the **per-use constant-expression requirement** (for example the M1 integer constant expression), **distinct** from the lexical `LiteralRecord.candidate_type`, and does **not** duplicate an implicit target type; `ConstantResult.legality` is a **result payload field** with **no** extra committed record family, and the exact `ConstLegality` variants remain open. **(2) G/T08/T01 — `max_const_bits` origin.** The origin is a **hashed `Limits` value** (`limits.max_const_bits` participates in the frozen-contract hash); the M1 cap/default is **128**; `config` **rejects** values **> 128**; the bound reaches the zero-field T08 chip via an **explicit task-input projection**. This is an accepted source/projection/bound subdecision; the **exact** wire field, diagnostic numeric code, and hash encoding remain `/6` details, and the accepted cap is **not** a claim that every valid config value must equal 128. **(3) C/T03/T01 — artifact shape/scope.** Accept `ArtifactRecord { kind, source, bytes, raw_offsets }` with the **total eight-`ArtifactKind`** scheme (map-mandatory `Normalized`/`Spliced`/`CommentFree`/`Preprocessed`; map-optional `Assembly`/`Object`/`Snapshot`/`Trace`), the M1 **exercised scope only single-source `Normalized`**, other producers and the multi-source map deferred; the **exact** `raw_offsets` invariant/error mapping and all enum numeric codes remain open. Every overall C/G bundle stays PENDING where other items are unresolved (the exact `ConstLegality` variants, `RequiredKind` enum codes, artifact map boundary formula/error classification, `raw_offsets` invariants, numeric IDs/tags, and enum numeric codes all remain open), and this record authorizes no code and no `/6` freeze; the CDR becomes rev 44 while the shared M1 proposal stays rev 36 and the current-state M1 pointer/range stays rev 21–35. | User explicit subdecision instruction (2026-10-05); M1 rev-44 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 45 (records further explicit user subdecision decisions of 2026-10-05 on rows C and G only; no accepted contract, no `/6` freeze, no code/chip authorization; no task package, proposal, or ADR edited; matches M1 proposal rev 36 and ADR-0002 Revision 16).** The **user** — self-identifying as holding **all** named T01/owner roles (T01 `[INT]`, `[OWNER:T03]`, `[OWNER:T04]`, `[OWNER:T07]`, `[OWNER:T08]`) and responding **directly in each role** — accepted these **selected subdecisions only**, each recorded with role label and **2026-10-05** date: **(1) C/T04/T08/T01 — `LiteralRecord` exact ordered fields.** `token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` (big-endian magnitude), `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`; **no `node`/`required_kind`**. **(2) C/T04/T08/T01 — M1 enum/scope.** `LiteralKind {Integer, Character, String}` with **only `Integer` produced in M1**; `LiteralSuffix {None, U, L, UL, LL, ULL}` with **only `None` produced in M1**; radix `{2, 8, 10, 16}` with **M1 decimal only**; `Lx08CandidateType` is **symbolic/target-independent** with M1 literals 2 and 3 as `Int` and **no bit width** — its **complete member set/numeric encodings are deliberately not invented and remain open**. **(3) C/T07/T08/T01 — `RequiredKind`/`ConstLegality`.** `RequiredKind` M1 enum only `IntegerConstantExpression`; `ConstLegality` result-field values `Legal`, `NotConstantExpression`, `Unsupported`; future C constant-expression purposes require **appended variants/new rules**, not a repurposing of the lexical candidate type. **(4) C/T03/T01 — mandatory artifact-map invariants.** `raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds require a valid source; the **optional-kind map rule, source-versus-payload equality, and exact artifact error classification/numeric codes remain open** because the prompt allowed an empty optional map or another explicit rule and the user made no choice between them. Every overall C/G bundle stays PENDING where other items are unresolved (the complete `Lx08CandidateType` member set/numeric tags, the `RequiredKind`/`ConstLegality` numeric codes, the optional-kind artifact-map rule, source-versus-payload equality, the exact artifact error classification/numeric codes, and all enum numeric codes remain open), and this record authorizes no code and no `/6` freeze; the CDR becomes rev 45 while the shared M1 proposal stays rev 36 and the current-state M1 pointer/range stays rev 21–35. | User explicit subdecision instruction (2026-10-05); M1 rev-45 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 46 (records further explicit user subdecision decisions of 2026-10-05 on row F only; no accepted contract, no `/6` freeze, no code/chip authorization; no task package, proposal, or ADR edited; matches M1 proposal rev 36 and ADR-0002 Revision 16).** The **user** — self-identifying as holding **all** named T01/owner roles (T01 `[INT]`, `[OWNER:T06]`, `[OWNER:T07]`, `[OWNER:T09]`, `[OWNER:T13]`) and responding **directly in each role** — accepted these **selected subdecisions only**, each recorded with role label and **2026-10-05** date: **(1) F/T07 — `ValueCategory` + `EffectMask`.** `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` with the M1 fixture producing **`NonLvalue`**; M1 allows **only `EffectMask(0)`** (a nonzero mask is a **typed unsupported/diagnostic**; the effect **bit classes are reserved/unassigned**). **(2) F/T07/T13/T01 — VF06 in M1.** VF06 **`TypedAstInvariant` is in M1**: it executes **after** the committed T07 `SemRecord`s and **before** T09 lowering, checking **M1 typed-fact/required-conversion completeness**; the **exact registered task kind/stage/phase/interface remains T01/T13 co-freeze/open** and is not asserted. **(3) F/T06/T07/T09/T01 — M1-minimal conversion scope.** Select the **M1-minimal conversion scope only** — freeze **only** the conversion behavior the M1 fixture needs, including the **identity/no-conversion rule**. **Integer promotions, float conversions (incl. `FloatToFloat`), pointer-qualifier conversions, and other non-M1 conversions are explicitly unsupported/deferred** to a later **append/contract revision** (distinct from silently omitting them); **no** exact `ConversionOp`/`ConversionRole` closed variant list, pairing, role→chip mapping, or numeric encoding is invented (all remain open `/6` co-freeze details). Every overall F bundle status stays **PENDING** and the broad rows C–I remain PENDING; this record does **not** amend the Guardrail, does **not** edit `/5`/T01 §4/§5, does **not** add a field/enum/rule/hash, does **not** authorize a chip wave, and does **not** turn on the pipeline. **M1 remains DRAFT; ADR-0002 remains PROPOSED; `/5` remains current;** the CDR becomes **rev 46**, the shared M1 proposal stays **rev 36**, and the current-state M1 pointer/range stays **rev 21–35**. | User explicit subdecision instruction (2026-10-05); M1 rev-46 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 47 (records integration-agent selected candidate defaults under explicit user delegation on 2026-10-05; no accepted contract, no `/6` freeze, no owner or T01 `[INT]` signoff, no code/chip authorization; no task package, proposal, or ADR edited; matches M1 proposal rev 36 and ADR-0002 Revision 16; current-state M1 pointer/range stays rev 21–35).** Under the user's explicit delegation ("for non-critical decisions adopt the recommended choice directly; ask only for critical decisions"; per the user's 2026-10-05 instruction), the **M1 integration agent** selected four **low-risk candidate defaults** for the remaining open items, each marked **selected under user-delegated integration default** and dated **2026-10-05**: **(1) optional `ArtifactKind` map policy** — `requires_map(kind) == false` ⇒ empty `raw_offsets`; `source` stays `Option<SourceId>` and must be valid when `Some`; mandatory-map kinds require a valid `source` plus the accepted rev-45 invariants; **no** source-payload-equals-artifact-bytes requirement (normalization transforms); any exact source-provenance/equality rule deferred; exact numeric error codes stay open. **(2) T05 parse depth** — reuse `limits.max_task_depth`; count parser continuation/child frames only (not total bus tasks); detect the limit **before any child enqueue for the descent**; excess is the `ParseDepthExceeded` chip diagnostic; exact `ParseContext` encoding/request-result variants stay pending. **(3) T06 namespace/symbol lookup** — derive a closed namespace from `SymbolKind` (`Object`/`Function`/`Typedef`/`EnumConst` → `Ordinary`; `StructTag`/`UnionTag`/`EnumTag` → `Tag`; `Label` → `Label`) with no namespace field; wrong-namespace lookup is a **miss not a conflict**; the deterministic **TY03** lookup walks the **active scope chain only**, orders candidate declarations by `(source,start,end,NodeId)`, picks the greatest ≤ the query point, same-scope tie to the higher `SymbolId`, else the innermost active scope; the T05 `NodeKind`/token-range dependency and exact event encoding/lifecycle/allowlist rows remain **remaining co-freeze**. **(4) T09 noncritical rule defaults** — choose the shorter aliases `ir.op-immediate-type` and `ir.terminator-missing`; for `Constant` with both immediate and result missing validate the **immediate first**; the target type equals the result `ValueRecord.ty`; all remain **prospective `/6`** with owner/T01 acceptance and hashing pending; the `CompletedFunction` marker family stays **unresolved** with **no family invented**. These defaults are **selected under user-delegated integration default, not evidence of T03/T05/T06/T09 owner signoff** (unless the user separately already accepted that exact subdecision) and **not** T01 `[INT]` acceptance; **overall rows C/D/E/H remain PENDING** and all public/shared schema awaits owner/T01 co-freeze. No conflicting user-selected rev-42/43/44/45/46 decision is overridden. This record does **not** amend the Guardrail, does **not** edit `/5`/T01 §4/§5, does **not** add a field/enum/rule/hash, does **not** authorize a chip wave, and does **not** turn on the pipeline; `/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED. Recorded in §9A (rows B/D/E/H), §9B (rows C/D/E/H), §9C (the separated delegated-candidate-default note), §10 (the separate delegation record), and §13. This CDR becomes **rev 47** while the shared M1 proposal stays **rev 36** and the current-state M1 pointer/range stays **rev 21–35**. | User explicit delegation instruction (2026-10-05); M1 rev-47 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 48 (records read-only T01 audit findings on 2026-10-05; docs-only; no accepted contract, no `/6` freeze, no owner or T01 `[INT]` signoff, no code/chip authorization; no task package, T01/manifest/other-doc edited; no hash-scope option selected or recommended; matches M1 proposal rev 36 and ADR-0002 Revision 16; current-state M1 pointer/range stays rev 21–35).** A **read-only** audit verified the checked-in `/5` code and recorded, as **facts**: the frozen contract is `t01-c01-c06/5` (`61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5`); `compiler/src/contract.rs` encodes **24** `RECORD_KINDS`; `compiler/src/ids.rs` `RecordRef` has **24** variants (tags **0–23**, `Artifact` = 23); and **no** `RecordFamily` type exists in the code. The CDR's candidate figures — **19 draft families / 27 `RecordRef` variants / 27 `RecordFamily` ordinals** — are marked **proposed/unverified** (the future `/6` count is **not** claimed to equal the current 24). The audit confirmed a **real unresolved hash-scope conflict** (not merely prose): `CONTRACT_VERSION`, `contract.rs`, and `README.md` exclude `group-declared-store-fields` from the hash, while `COMPILER_SFL_MANIFEST.md` §4 states adding fields/rules changes the frozen hash, and the proposed `M1AppendSchema` is **proposed to be hashed at `/6`**. The **three options** — hash `M1AppendSchema` (update exclusion token + freeze assertion + manifest); do not hash (correct manifest wording); or an explicit two-tier hash-seed-vs-runtime-declarations split — are **not selected and not recommended** here (no authority). The audit requires `/6` to specify **dual-inventory encoding** (`RecordRef` wire tags separate from `RecordFamily` ordinals — the structural subdecision accepted at rev 43), **inclusion of numeric values in the hash**, and a **schema self-consistency mechanism**, freezing actual counts/values only after owner-shape acceptance. This is a **critical `/6` hash-scope choice with no implementation authorization**. Recorded in the header, §2 dashboard note/row I, §3.2/§3.3, §9, §9A (row I + authority honesty), §9B (row I + authority), §9C (separated audit note + T01-global row), §10 (separate audit record + table row I), and §13. It adds **no owner acceptance and no signature**; M1 remains DRAFT; ADR-0002 remains PROPOSED; `/5` remains current; rows C–I remain PENDING. This CDR becomes **rev 48** while the shared M1 proposal stays **rev 36** and the current-state M1 pointer/range stays **rev 21–35**. | Read-only T01 audit (2026-10-05); M1 rev-48 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 49 (records the user's explicit 2026-10-05 critical H11/T09/T01 direction plus read-only H9 audit findings; docs-only; no accepted contract, no `/6` freeze, no code/chip authorization; no task package, T01/manifest/other-doc/test edited; no hash-scope option selected; matches M1 proposal rev 36 and ADR-0002 Revision 16; current-state M1 pointer/range stays rev 21–35).** The **user** recorded an explicit **critical decision** for **H11/T09/T01**: the `TerminatorMissing` trigger **reuses the committed terminal result of the IR28 `FunctionEnd` task** — the `TaskState::Completed(ResultId)` produced by the task whose **kind is `FunctionEnd`** — as the **deterministic function-completion fact**, checked by a **T01-owned typed phase-2b commit-apply validation hook** (the function's entry block is terminated, i.e. its greatest `InstructionId` is a terminator). This **adds no new marker record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder and no new `ResultValue` variant**; the earlier **new-marker (`CompletedFunction`) direction is superseded for the operative direction** but **preserved as history**. The selected direction is **not a frozen `/6` hook/schema and authorizes no code**; the **exact hook contract**, its **hash impact** (likely **no** new record family, but **rule/hook hashing is still a T01 decision**), **result typing/commit ordering**, and the **T09/T01 co-freeze** remain **open**, and **overall H stays pending**. Separately, **read-only H9 audit findings** are recorded **as unresolved blockers, without changing the selected H6 mechanism or the H9 `max_inflight_total`-removal direction**: the `/5` code proves **only** the quota=1 single-task `fail_selected` transition, so the proposed **quota>1 pipeline is not implemented**; H9 still needs the **exact dispatcher pre-worker `Ready→Running`/`in_flight`-population relationship to the single ordered atomic commit**, the **exact `in_flight` clear ownership/order relative to the bounded H6 recovery**, and a **no-`Running`/no-residual proof for all success/error/empty-proposal paths**; an **internal proposed-limit conflict** (`max_dispatches_per_tick` listed/checked separately from `max_inflight_per_tick` although the latter is described as the sole dispatch bound) must be **resolved in `/6`** (not decided now); the **H6/H9 fan-out fixtures** and **T13 VF02/VF03/VF04/VF13** remain **pending**; the H9 no-residual guarantee is **conditional on the H6 recovery implementation**, and the **quota=1 M1 baseline stays separated** from the quota>1 optimization. The audit is **not a signature**; no freeze/code; exact stages/version carrier/errors/hashes remain pending. Recorded in the header, §2 dashboard note/rows A/H/I, §9 (H6/H9/T08-T09/cross-cutting), §9A (row A/row H/authority honesty), §9B (row H + authority), §9C (H row + separated rev-49 note), §10 (row A/row H/row I + separate record), §H1/H closure, §13 (T09 amendment + rev-49 note), and this §12. Rows C–I remain PENDING. This CDR becomes **rev 49** while the shared M1 proposal stays **rev 36** and the current-state M1 pointer/range stays **rev 21–35**. | User explicit critical H11/T09/T01 decision + read-only H9 audit (2026-10-05); M1 rev-49 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 50 (records the user's explicit 2026-10-05 critical HASH-SCOPE decision; docs-only; no accepted contract, no `/6` freeze, no code/chip authorization; no task package, T01/manifest/code/test/other-doc edited; no `/5` change; matches M1 proposal rev 36 and ADR-0002 Revision 16; current-state M1 pointer/range stays rev 21–35).** The **user** recorded an explicit **critical decision** ([USER]) accepting the **two-tier hash-scope model**: `StoreSchema::foundation + M1AppendSchema` is the **frozen `/6` seed and participates in the `/6` contract hash**, while **post-seed runtime `StoreSchema::declare()` extensions remain excluded** from the frozen hash and are **captured/validated through runtime snapshot/schema mechanisms**. This **resolves the conceptual future `/6` boundary** — the former three hash-scope options (hash `M1AppendSchema`; do-not-hash + manifest fix; explicit two-tier split) are **no longer open at the conceptual level**, the **two-tier split being selected** — but it **authorizes or completes no actual `/6` schema values, hash, tag/count inventory, code, or `/5` change**. The **selected boundary does not itself accept exact `M1AppendSchema` contents/counts and freezes nothing**; **T01 must implement it as integration authority and still co-freeze the M1 seed values after the owners** (the numeric inventory and freeze-test implementation remain pending). At `/6` integration T01 must **atomically** update the normative [COMPILER_SFL_MANIFEST.md](../../compiler/contracts/COMPILER_SFL_MANIFEST.md) §4 wording, the **`hash_excludes` semantic description/token** (scoped to **post-seed runtime declarations**, not the frozen M1 seed), and `FrozenSchema::encode`/contract code plus the freeze test, **preserving the frozen `/5` hash and history** (`t01-c01-c06/5`, `6187…63bb5`). Per the **CDR §9C response protocol** (and `AGENTS.md` §2 authority order) this is the **user's own `[USER]` scope decision**, **not** a row-I closure, **not** a `/6` freeze, and **not** a T01 `[INT]` signature beyond the user's scope decision; it adds no owner/T01 signoff. Recorded in the header status; §2 dashboard note (title + list + rev-50 paragraph) and row I; §3.3 Hash-scope/`StoreSchema` rows; §9 cross-cutting; §9A row I + authority-honesty rev-50 update; §9B row I + authority rev-50 update; §9C T01-global row authority + separated rev-50 note; §10 separate rev-50 record + table row I; §13 rev-disambiguation + rev-50 note; and §12. `/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED, overall row I and rows C–I remain **PENDING**, and no freeze/chip authorization. This CDR becomes **rev 50** while the shared M1 proposal stays **rev 36** and the current-state M1 pointer/range stays **rev 21–35**. | User explicit critical hash-scope decision (2026-10-05) + M1 rev-50 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 51 (records an integration-selected candidate default under explicit user delegation: the sole per-tick dispatch-count bound; docs-only; no accepted contract, no `/6` freeze, no owner or T01 `[INT]` signoff, no code/chip authorization; no task package, T01/manifest/code/test/other-doc edited; no `/5` change; matches M1 proposal rev 36 and ADR-0002 Revision 16; current-state M1 pointer/range stays rev 21–35).** Under the user's explicit **2026-10-05 delegation** ("for non-critical decisions adopt the recommended choice directly; ask only for critical decisions"), the **M1 integration agent** selected — **under user-delegated integration default, not as an owner or T01 `[INT]` signoff** — that **`max_inflight_per_tick`/quota is the sole per-tick dispatch-count bound**, and that the redundant **`max_dispatches_per_tick` is dropped from the candidate limit inventory and its validation**. This is grounded in the rev-35/rev-38 H9 direction (per-tick dispatch is bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`) and in the rev-49 audit finding that the two limits duplicated each other. It **selects no** dispatcher-`Ready→Running`/`in_flight` atomic-boundary or clear-order item and changes neither the selected H6 mechanism nor the H9 removal direction. **T01/T02 must confirm/co-freeze the exact names/defaults/codes at `/6`, and the T02/T13 tests remain pending.** This is a **delegated candidate default**, **not** an owner/T01 `[INT]` signoff, **not** a schema freeze, and **not** code; `/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED, and rows C–I remain **PENDING**. Recorded in the header status; §2 dashboard note (title + list + rev-51 paragraph) and rows A/I; §3.3 Limits row; §9 H9 + cross-cutting; §9A row A + authority-honesty rev-51 update; §9B authority rev-51 update; §9C H/A row-adjacent separated rev-51 note; §10 separate rev-51 record + table row A; §13 rev-disambiguation + rev-51 note; and this §12. This CDR becomes **rev 51** while the shared M1 proposal stays **rev 36** and the current-state M1 pointer/range stays **rev 21–35**. | User explicit delegation (2026-10-05); M1 rev-51 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 52 (records a read-only cross-owner audit/reconciliation; docs-only; no accepted contract, no `/6` freeze, no owner or T01 `[INT]` signoff, no code/chip authorization; no task package, T01/manifest/code/test/other-doc edited; no `/5` change; points at M1 proposal rev 38 (parallel expected resulting file), ADR-0002 Revision 17, T02 rev 31).** A **read-only** reconciliation mirrors the CDR's already-selected items against the now-completed owner packages: **(A)** package state — **T02 rev 30+31**; **T03/T04** (T04 rev 46 incl. the `LiteralRecord.token` candidate and the same-revision correction to the typed `RecordLink` mechanism, with no `OwnBatch` confusion for record links); **T05 `ParseContext` candidate**; **T06**; **T07 sem→const correction**; **T08 exact candidate shape**; **T09 H11**; **T13**; **M1 vertical acceptance rev 27**; **M1 target acceptance rev 29**; **ADR-0002 Revision 17** — with **no owner package called a `/6` sign-off**; **(B)** the M1 proposal/target/vertical docs now align on H11, the hash boundary, and the Part A-vs-B split, with the proposal being advanced in parallel to **rev 38**, `/5` preserved, and **no `/6` freeze**; **(C)** the T01 audit still leaves `/6` **not freeze-ready** (exact record/tag/family inventories, field bodies/ref encoding, task/request/result typing, writer manifests, TU carrier/`ParseContext` final encoding, `SourceId`/token provenance/raw-offset mapping and `Lx08CandidateType`, `SemRecord`/VF06, const overflow/projection, the IR hook contract, the scheduler H6/H9 atomic boundaries/in_flight clear order/no-residual fixtures, and the snapshot/canonical-encoder/hash tests), with **no new user-critical decision implied**; **(D)** the **rev-51 `max_inflight_per_tick`-only** candidate default and the **rev-50 hash-scope** decision remain selected, with exact implementation/sign-off pending, and rows **C–I remain PENDING overall**; **(E)** the CDR is an accepted-decision ledger/work queue, **not an executable schema** — the **next integration stage after exact owner artifacts is T01's serial `/6` seed assembly**, self-consistency-checked against the frozen two-tier hash model, with **no chip coding before freeze** and **Part B independent**. Recorded in the header status; §2 dashboard note (title + list + rev-52 paragraph + bare-rev disambiguation); §9C separated rev-52 note; §10 separate rev-52 record; §13 rev-disambiguation + rev-52 note + render pointer; and this §12. This CDR becomes **rev 52** while the shared M1 proposal is expected at **rev 38**, ADR-0002 is at **Revision 17**, and T02 is at **rev 31**. | Read-only cross-owner audit (2026-10-05); M1 rev-52 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 53 (records an integration-agent selected candidate default under explicit user delegation — the `Lx08CandidateType` M1 vocabulary; docs-only; no accepted contract, no `/6` freeze, no owner or T01 `[INT]` signoff, no code/chip authorization; no task package, T01/manifest/code/test/other-doc edited; no `/5` change; points at M1 proposal rev 38 (current), ADR-0002 Revision 17, T02 rev 31).** Under the user's explicit **2026-10-05 delegation** ("for non-critical decisions adopt the recommended choice directly; ask only for critical decisions"), grounded in a **read-only review of the exercised M1 integer literal subset**, the **M1 integration agent** selected — **under user-delegated integration default, not as an owner/T04 or T01 `[INT]` signoff, not a schema freeze** — that for the **exercised M1 literal subset** the `Lx08CandidateType` M1 vocabulary is the **closed one-member set `{ Int }`**, with the M1 literals `2` and `3` represented as **target-independent `Int` with no bit width** (the `candidate_type` value on the committed `LiteralRecord`; **not** a target type, ABI type, or `TypeId`, and duplicating no implicit target type); that literal **forms outside the exercised M1 subset** (non-decimal radix, non-`None` suffix, character/string literals, or any not-yet-specified category) **must not silently default to `Int`** and are classified as an **explicit unsupported/deferred** result/diagnostic until their categories/rules are specified; and that future C candidate support **appends symbolic members/rules without reinterpreting** the selected `Int` member or repurposing `Lx08CandidateType`. This **refines only the meaning** of the already-selected `Lx08CandidateType`; it changes **no** field order/type/count, is **not** the **complete future C candidate vocabulary**, defines **no numeric tag/encoding**, is **not** a **T04/T08 owner or T01 `[INT]` signoff**, and is **not** a `/6` freeze. The **complete `Lx08CandidateType` member set and all numeric encodings remain open** (a T04/T08/T01 `/6` co-freeze item). Recorded in the header status; §2 dashboard note (title + list + rev-53 paragraph + bare-rev disambiguation); §9A authority-honesty rev-53 update; §9B authority rev-53 update; §9C separated rev-53 note + section-note list; §10 separate rev-53 record + status row C; §13 rev-disambiguation + rev-53 note + render pointer; and this §12. `/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED, rows C–I remain **PENDING overall**, and no code/freeze is authorized. This CDR becomes **rev 53** while the shared M1 proposal is at **rev 38** (current), ADR-0002 is at **Revision 17**, and T02 is at **rev 31**. | Integration-agent selected candidate default under explicit user delegation (2026-10-05); M1 rev-53 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 54 (read-only audit notes + historical-supersession cleanup; docs-only; no accepted contract, no `/6` freeze, no owner or T01 `[INT]` signoff, no code/chip authorization; no task package, T01/manifest/code/test/other-doc edited; no `/5` change; points at M1 proposal rev 39 (expected), ADR-0002 Revision 17, T02 rev 32/33).** This revision makes **no architecture-critical choice** and adds **no signoff**. **(1) §G2 supersession:** the rev-21 `max_const_bits` **signed-range/`i128` representable-set formula**, the `ConstRecord.value: i128` **carrier**, and the **chip-level enforcement** wording are re-marked as **historical and superseded — not selected** (the T08 correction marked them unselected); §G2/§G3 retain the rev-21 request as **historical text only**, and the accepted content is restated as **only** the rev-44 **origin/cap/projection** (`max_const_bits` is a hashed `Limits` value; M1 cap/default 128; `config` rejects `> 128`; explicit task-input projection). The exact **formula, numeric/wire carrier, and enforcement split** remain **open `/6` co-freeze** (row **G**). **(2) H9 duplicate-limit conflict:** the rev-49 `max_dispatches_per_tick`-vs-`max_inflight_per_tick` conflict text is qualified as **historical/superseded** by the **rev-51** sole-`max_inflight_per_tick` candidate (history retained), while the **dispatcher `Ready→Running`/`in_flight` commit-boundary and clear-order items** remain **unresolved** (row **A**). **(3) Latest read-only findings (all open, no choice made):** the **T04 reciprocal token↔literal same-batch link** needs a **cycle-safe ID reservation** (row **C**); the **T05/T06 `parse.TranslationUnit -> symbol_type.scope-enter` TU edge** needs an **exactly-once atomic consumption/dedupe** (rows **D/E**); the **T13 no-residual fixtures** are **incomplete** (row **A**/T13); and the **M1 vertical PP-08 failure semantics + exact artifact-map arrays** are **pending**. Recorded in the header status; §2 dashboard note (title + list + rev-54 paragraph); §G2/§G3 + §G closure criterion; §9 H9; §9C separated rev-54 note + section-note list; §10 separate rev-54 record; §13 rev-disambiguation + rev-54 note + render pointer; and this §12. `/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED, rows C–I remain **PENDING overall**, and no code/freeze is authorized. This CDR becomes **rev 54** while the shared M1 proposal is expected at **rev 39**, ADR-0002 is at **Revision 17**, and T02 is at **rev 32**. | Read-only audit/reconciliation (2026-10-05); M1 rev-54 docs-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | **Rev 55 (OPEN-03 M1 constant-expression handoff co-freeze; docs-only; T01/T07/T08 co-freeze; no accepted contract, no `/6` freeze, no owner or T01 `[INT]` signoff, no code/chip authorization; no `/5` change; points at M1 proposal rev 40, ADR-0002 Revision 17, T02 rev 33, T07 rev 4, T08 task-package revision 5, T09 rev 49, M1 vertical rev 31).** Records the directed OPEN-03 fix (documentation review 2026-10-05) closing the §F4/§24.10 binary-input gap: the M1 `const.evaluate` request variants are **frozen for the M1 `/6` co-freeze** as `ConstantRequest::Literal { literal: RecordRef::Literal, node, required_kind }` and `ConstantRequest::Binary { node, op: ConstExprOp, lhs: RecordRef::Literal, rhs: RecordRef::Literal, required_kind }` — every record input is committed (the committed `BinaryExpression` `NodeId`, T07's checked operator `Add` in M1, and the committed T04 `LiteralRecord` refs in source order); T08 decodes the committed operands, folds with checked addition, and commits **exactly one** `ConstRecord`; T09 consumes the same committed `RecordRef::Const` (`ConstId`) without re-folding. The real T07→T08→T09 fixture is `M1-CL-05`; `M1-CL-03` is a conversion-unit fixture only (its hand-built `5` is not evaluation evidence). Remaining open: the `ConstantResult` `ResultValue`/envelope carrier and `legality` placement, non-legal coupling, `ConstRecord` identity/reuse, and all exact wire tags/payload-variant spellings/task-kind spellings/numeric codes. Updates §F4 and this §12 row; the task packages and proposal carry the corresponding revisions. No other schema/interface/field/enum/rule/hash change; `/5` remains current, M1 remains DRAFT, ADR-0002 remains PROPOSED, rows C–I remain **PENDING overall**, and no code/freeze is authorized. | OPEN-03 fix (documentation review 2026-10-05); T01/T07/T08 co-freeze |

---

## 13. Pending owner amendment requests (task packages not edited)

**Revision-number disambiguation (M1 proposal vs this CDR).** The proposal and the
CDR maintain **separate** revision counters, and a bare "rev 30", "rev 31",
"rev 32", "rev 33", "rev 34", "rev 35", "rev 36", "rev 37", "rev 38", "rev 39", "rev 40", "rev 41",
"rev 42", "rev 43", "rev 44", "rev 45", "rev 46", "rev 47",
"rev 48", "rev 49", "rev 50", "rev 51", "rev 52", "rev 53", or
"rev 54"
does **not** denote the same change in both. Specifically: the **M1 Part A proposal
rev 30** is the revision that **records the user's two in-principle decisions A/B**
(2026-10-04); this **CDR rev 30** is a **docs-only scope/label correction** (the M1
target acceptance §7 failure-guarantee label "Rev 26 F2" → **Rev 25 F2**), and the
**CDR rev 31** is the revision that **records the same A/B decisions** on the CDR
side (a new revision layer, accepted in principle only). Likewise the **M1
proposal rev 31** is the proposal's own docs-only **history-gap repair**, while
**CDR rev 31** is the A/B decision record, **CDR rev 32** is the history-gap
repair, and **CDR rev 33** is a docs-only audit-defect correction. The **M1
proposal rev 32** is the **H6 batch-failure recovery-direction record** (2026-10-04),
matching **CDR rev 34**; the **M1 proposal rev 33** is the **H9 removal-direction
+ D1–D3 record**, matching **CDR rev 35** (the proposal advanced to **rev 34** when
the cross-document H9 completion was recorded, and the CDR advanced to **rev 36**
with the D3 §13 correction). The **M1 proposal rev 35** is the **stale-active-prose
cleanup**, matching **CDR rev 37**; and the **M1 proposal rev 36** is the
**header current-state-pointer + typo correction**, matching **CDR rev 38** (both
keep the current-state range at **rev 21–35**). The **CDR rev 39** has **no
matching M1 proposal revision**: it is the CDR-only addition of the proposed `/6`
co-freeze work queue (§9A) and adds no revision to the shared proposal, which
stays at **rev 36**. The **CDR rev 40** likewise has **no matching M1 proposal
revision**: it is the CDR-only addition of the proposed decision-ready
recommendation appendix (§9B). The **CDR rev 41** likewise has **no matching M1
proposal revision**: it is the CDR-only addition of the proposed per-decision
response matrix (§9C). The **CDR rev 42** likewise has **no matching M1 proposal
revision**: it is the CDR-only per-subdecision acceptance record. The **CDR rev 43**
likewise has **no matching M1 proposal revision**: it is the CDR-only further D/E
per-subdecision acceptance record. The **CDR rev 44**
likewise has **no matching M1 proposal revision**: it is the CDR-only further C/G
per-subdecision acceptance record. The **CDR rev 45** likewise has **no matching M1
proposal revision**: it is the CDR-only further C `LiteralRecord`
exact-ordered-fields/enum-scope + `Lx08CandidateType` symbolic/no-bit-width +
`RequiredKind`/`ConstLegality` values + mandatory artifact-map-invariant
per-subdecision acceptance record. The **CDR rev 46** likewise has **no matching M1
proposal revision**: it is the CDR-only further F `ValueCategory`/`EffectMask` +
VF06-in-M1 + M1-minimal-conversion-scope per-subdecision acceptance record. The
**CDR rev 47** likewise has **no matching M1
proposal revision**: it is the CDR-only **integration-agent selected candidate
defaults under explicit user delegation** record (optional-`ArtifactKind` map
policy, T05 parse depth, T06 namespace/symbol lookup, T09 noncritical rule
defaults; **not** owner or T01 `[INT]` signoffs) (the shared M1
proposal stays **rev 36**). The **CDR rev 48** likewise has **no matching M1
proposal revision**: it is the CDR-only **read-only T01 audit findings** record
(checked-in `/5` code facts **24**/`24`, `RecordFamily` absent; the unresolved
`M1AppendSchema` hash-scope conflict; the 19/27/27 draft counts marked
**proposed/unverified**; no hash-scope option selected or recommended; no freeze).
The **CDR rev 49** likewise has **no matching M1 proposal revision**: it is the
CDR-only **critical H11/T09/T01 `FunctionEnd`-terminal-result direction** record
(reuse the committed IR28 `FunctionEnd` terminal result; no new marker family/ID/
arena/tag/ordinal/encoder/`ResultValue`; T01-owned typed phase-2b commit-apply
validation hook; earlier new-marker direction superseded for operative direction and
preserved as history; not a frozen `/6` hook, no code) plus the **read-only H9 audit
findings** (quota>1 pipeline unimplemented; `Ready→Running`/`in_flight`-commit
relationship and clear ownership/order open; no-`Running`/residual proof for all
paths; `max_dispatches_per_tick`-vs-`max_inflight_per_tick` conflict deferred to
`/6`; fan-out fixtures/VF02/VF03/VF04/VF13 pending; H9 removal direction and H6
mechanism unchanged; no freeze/code) (the shared M1 proposal stays **rev 36**). The
**CDR rev 50** likewise has **no matching M1 proposal revision**: it is the CDR-only
**user critical hash-scope two-tier decision** record (frozen `/6`
`foundation + M1AppendSchema` seed hashed; post-seed runtime `declare()`
declarations hash-excluded and captured/validated via runtime snapshot/schema
mechanisms; conceptual boundary settled but no `/6` values/hash/counts/code/`/5`
change; T01 still co-freezes the M1 seed values after owners; row I stays PENDING)
(the shared M1 proposal stays **rev 36**). The **CDR rev 51** likewise has **no
matching M1 proposal revision**: it is the CDR-only **integration-selected candidate
default under explicit user delegation** record — `max_inflight_per_tick`/quota is
the **sole per-tick dispatch-count bound** and the redundant
`max_dispatches_per_tick` is **dropped** from the candidate limit inventory and
validation (grounded in the rev-35/38 H9 direction and the rev-49 audit duplication
finding; **not** an owner/T01 `[INT]` signoff; exact names/defaults/codes remain
T01/T02 `/6` co-freeze; T02/T13 tests pending) (the shared M1 proposal stays
**rev 36**). The **CDR rev 52** likewise has **no matching M1 proposal revision**:
it is the CDR-only **read-only cross-owner audit/reconciliation** record —
**T02 rev 30+31**, **T03/T04** (T04 rev 46 incl. the `LiteralRecord.token` candidate
and the correction to the typed `RecordLink` mechanism, no `OwnBatch` confusion for
record links), **T05 `ParseContext` candidate**, **T06**, **T07 sem→const
correction**, **T08 exact candidate shape**, **T09 H11**, **T13**, **M1 vertical
acceptance rev 27**, **M1 target acceptance rev 29**, and **ADR-0002 Revision 17**
all mirror the already-selected items with **no owner package called a `/6`
sign-off**; the M1 proposal/target/vertical docs align on H11/hash boundary/Part
A-vs-B; the T01 audit still leaves `/6` **not freeze-ready** (no new user-critical
decision); the rev-51 and rev-50 selections stand with exact implementation/sign-off
pending (**rows C–I PENDING overall**); and the CDR is an **accepted-decision
ledger/work queue, not an executable schema**, whose next integration stage is
T01's serial `/6` seed assembly against the frozen two-tier hash model, with no chip
coding before freeze and Part B independent (the **M1 proposal is at rev
38**, current). Read
every bare
"rev 30/31/32/33/34/35/36/37/38/39/40/41/42/43/44/45/46/47/48/49/50/51/52/53" below against its document.

The **M1 proposal** was updated through **rev 36**: rev 27 applied the rev-26 audit
corrections; rev 28 is a docs-only pointer reconciliation; rev 29 is a docs-only
historical-documentation correction; rev 30 records the user's in-principle
decisions A/B; rev 31 is a docs-only **history-gap repair** that supplies the
missing historical rev-29 row (§16 had jumped rev 28 → rev 30) and advances every
current-state M1 pointer to **rev 21–31**; rev 32 records the **H6 batch-failure
recovery direction** (in principle, direction only) and advances the pointer to
**rev 21–32**; rev 33 records the **H9 `max_inflight_total` removal direction** and
the **D1–D3** corrections (pointer-list repair, §20 status key, and the CDR §13
D3 issue) and advances the pointer to **rev 21–33**; rev 34 is a docs-only
consistency completion that applies the H9 integration in this CDR/ADR-0002/T02
and the **D3** CDR §13 correction, advancing the pointer to
**rev 21–34**; rev 35 is a docs-only consistency cleanup of stale active
candidate prose (the T02 in-flight-bound bullet no longer opens with the rev-22
candidate; see the T02 rows), advancing the pointer to
**rev 21–35**; and rev 36 is a docs-only header/audit correction — a
**current-state pointer** is added at the top of the proposal so top-down readers
see rev 35 / current **rev 21–35** first and earlier paragraphs are marked
historical, the §21 `In in addition` and §19.3 `candidate-type type` typos are
fixed, and the pointer is re-stated as the current **rev 21–35** (no change to
the current-state range).
The **CDR** was updated through
**rev 54**: rev 27 applied the rev-26 audit corrections; rev 28 is a docs-only
pointer reconciliation; rev 29 is a docs-only historical-documentation correction;
rev 30 is the docs-only scope/label correction; rev 31 records decisions A/B; rev
32 is the CDR side of the **same docs-only history-gap repair** (rev 32 mirrors
the M1 proposal rev 31: it re-points the current-state CDR links to the M1 proposal
**rev 31** and rewrites this §13 opening to disambiguate the revision counters);
rev 33 is a docs-only **audit-defect correction** of this §13 target-acceptance
paragraph and the §2 same-number-counter note; rev 34 records the **H6
batch-failure recovery direction** (matching the M1 proposal rev 32) and advances
the pointer to **rev 21–32**; rev 35 records the **H9 in-flight
scheduling-ownership direction** — accept removing `max_inflight_total`; per-tick
dispatch is bounded by `max_inflight_per_tick`/quota, the stage queues, and
`max_tasks_total`; `Waiting` is not in-flight; `tasks.in_flight` is an ephemeral
per-tick scheduler batch only; the dispatcher's pre-worker `Ready→Running`
mutation is distinguished from the ordered atomic semantic commit (matching the M1
proposal rev 33/34) — and advances the pointer to **rev 21–33**; rev 36 is the
docs-only **D3 correction** of this §13 frontend paragraph (see below), advancing
the pointer to **rev 21–34**; rev 37 is a docs-only consistency cleanup of
stale active candidate prose — the §A9/§I dispatcher error lists no longer name
`InflightQuotaExceeded` as an active candidate — advancing the pointer to
**rev 21–35**; rev 38 is the docs-only **lead-in/typo correction** — this §13
opening lead-in now reads the M1 proposal as updated through **rev 36** (it had
said **rev 34** while continuing through rev 35), the §13 `per-stage stage queues`
typo is fixed, and the current-state pointer remains **rev 21–35** (the shared M1
proposal advances to **rev 36** for its header current-state pointer and typo
fixes; no change to the range); and rev 39 is the docs-only **proposed `/6`
co-freeze work queue** added in §9A (three separate gates, the parallel
owner-drafting/review tracks vs the serial freeze dependencies, and the minimum
work items A–I with exact authorities) — no acceptance, schema, interface,
task-kind, chip, or `/6` claim; the current-state pointer remains **rev 21–35** and
the shared M1 proposal remains **rev 36**; and rev 40 is the docs-only **proposed
decision-ready recommendation appendix** added in §9B (per-row recommendation +
tradeoff/open dependency, all for accept/amend/reject, with no acceptance,
schema, interface, task-kind, chip, or `/6` claim; it does not erase the
accepted-in-principle H6 direction), keeping the current-state pointer at
**rev 21–35** and the shared M1 proposal at **rev 36**; and rev 41 is the docs-only
**proposed per-decision response matrix** added in §9C (bundle → exact authority →
`Accept recommendation / Amend / Reject / Defer` with a `PENDING` status/signature
cell, citing §9A/§9B/§C–§I; a procedural form, not schema; no new decision, no
acceptance, schema, interface, task-kind, chip, or `/6` claim; it preserves the
user's in-principle H6 direction and requires any H6 deferral to return to the
user), keeping the current-state pointer at **rev 21–35** and the shared M1 proposal
at **rev 36**; and rev 42 records the user's explicit **2026-10-05 per-subdecision
authority decisions** (A H6 scope+mechanism, D CT07-invariant+await-all, F carrier
decisions, E lowest-id scan), each recorded per role label and date, with every
overall bundle staying **PENDING** where other items are unresolved, and explicitly
**rejects the §9B row-A H6 deferral** in favour of freezing the selected mechanism
in `/6` (no acceptance, no `/6` freeze, no code; keeps the current-state pointer at
**rev 21–35** and the shared M1 proposal at **rev 36**); and rev 43 records further
explicit user **2026-10-05 D/E subdecisions** — the `ContinuationRecord` exact
ordered fields/shapes (no `awaited`; durable committed-ID references only), the formal
`/6` T01 §4 supersession (`WaitSet` sole awaited-child source; no `/5` edit), the
exact OwnBatch pre-apply validation (phase-1/phase-2b; whole-batch `CommitError`
before any mutation), and the File Enter trigger as a deterministic
`parse.TranslationUnit -> symbol_type.scope-enter` stage edge after the committed TU
carrying the committed `NodeId` with no job-bootstrap (no acceptance of the
`RecordRef` numeric tag values / `RecordFamily` ordinals — separate inventories — or
of the `ParseContext`/encoder/OwnBatch error numeric codes; T05 committed-TU upstream
remains pending; no `RecordField` tag values, no `/6` freeze, no code; keeps the
current-state pointer at **rev 21–35** and the shared M1 proposal at **rev 36**); and
rev 44 records further explicit user **2026-10-05 C/G subdecisions** — the
`ConstantRequest.required_kind` **per-use constant-expression requirement** meaning
(e.g. M1 integer constant expression) distinct from the lexical
`LiteralRecord.candidate_type` and with no duplicate implicit target type, the
`ConstantResult.legality` **result payload field** placement with no extra committed
family (exact `ConstLegality` variants open), the `ArtifactRecord { kind, source,
bytes, raw_offsets }` shape + **total eight-`ArtifactKind`** map rule
(Normalized/Spliced/CommentFree/Preprocessed map-mandatory; Assembly/Object/Snapshot/
Trace map-optional) with M1 **exercised scope only single-source `Normalized`** and
other producers/multi-source map deferred, and the `max_const_bits` **hashed `Limits`
value** origin with M1 cap/default **128**, `config` **rejects > 128**, and an
**explicit task-input projection** to the restricted T08 chip (the exact wire field,
diagnostic numeric code, and hash encoding remain `/6`; the cap is **not** a claim
that every valid config value must equal 128; no exact `ConstLegality` variants,
`RequiredKind` codes, artifact map boundary formula/error classification,
`raw_offsets` invariants, or numeric tags/IDs are accepted; keeps the
current-state pointer at **rev 21–35** and the shared M1 proposal at **rev 36**); and
rev 45 records further explicit user **2026-10-05 C/G subdecisions** — the exact
ordered `LiteralRecord` fields (`token`, `kind`, `radix`, `suffix`, `value`
big-endian magnitude, `negative`, `spelling`, `candidate_type`; no
`node`/`required_kind`), the M1 `LiteralKind {Integer, Character, String}` (only
`Integer` produced) / `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None`
produced) / radix `{2,8,10,16}` (M1 decimal only) scope, the symbolic
`Lx08CandidateType` (M1 literals 2/3 = `Int`, no bit width; complete member
set/numeric encodings **not invented and still open**), `RequiredKind` M1 enum only
`IntegerConstantExpression`, the `ConstLegality` values
`Legal`/`NotConstantExpression`/`Unsupported` (future C constant-expression purposes
via appended variants/new rules, not lexical-candidate reuse), and the mandatory
artifact-map invariants (`raw_offsets.len() == bytes.len()+1`, first `== 0`,
monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds require a
valid source; the optional-kind map rule, source-versus-payload equality, and exact
artifact error classification/numeric codes **remain open**; no `/6` freeze, no code;
keeps the current-state pointer at **rev 21–35** and the shared M1 proposal at
**rev 36**); and rev 46 records further explicit user **2026-10-05 F subdecisions** —
the `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }`
discriminants (M1 fixture = `NonLvalue`), the M1-only `EffectMask(0)` rule (nonzero
is a typed unsupported/diagnostic; effect bit classes reserved/unassigned), VF06
`TypedAstInvariant` **in M1** (after committed T07 `SemRecord`s, before T09 lowering,
checking M1 typed-fact/required-conversion completeness; the exact registered task
kind/stage/phase/interface remains T01/T13 co-freeze/open), and the **M1-minimal
conversion scope** (freeze only M1-fixture-needed behavior incl. identity/no-conversion;
integer promotions, float conversions incl. `FloatToFloat`, pointer qualifier, and
other non-M1 conversions explicitly unsupported/deferred to a later append/contract
revision; no `ConversionOp`/`ConversionRole` closed list, pairing, role→chip mapping,
or numeric encoding invented; no `/6` freeze, no code; keeps the current-state pointer
at **rev 21–35** and the shared M1 proposal at **rev 36**); and rev 47 records the
**integration-agent selected candidate defaults under explicit user delegation**
(2026-10-05) — the optional-`ArtifactKind` map policy (map-optional kinds have empty
`raw_offsets`; `source` valid when `Some`; no source-payload-equals-bytes
requirement; source-provenance/equality rule deferred; exact numeric error codes
open), the T05 parse-depth default (reuse `limits.max_task_depth`, count parser
continuation/child frames only, detect before any child enqueue for the descent,
excess = `ParseDepthExceeded` chip diagnostic; exact `ParseContext`/request-result
variants pending), the T06 namespace/symbol-lookup default (closed `SymbolKind`
→namespace mapping, no namespace field, miss-not-conflict, deterministic TY03
active-scope-chain lookup with `(source,start,end,NodeId)` ordering / greatest ≤
point / same-scope higher `SymbolId` / else innermost; T05 `NodeKind`/token-range
dependency and exact event encoding/lifecycle/allowlist rows still co-freeze), and
the T09 noncritical rule defaults (shorter aliases `ir.op-immediate-type` and
`ir.terminator-missing`; immediate-before-result when `Constant` misses both;
target type = result `ValueRecord.ty`; all prospective `/6`; `CompletedFunction`
marker family unresolved, no family invented) — these are **selected under
user-delegated integration default**, **not** owner or T01 `[INT]` signoffs and
**not** evidence of T03/T05/T06/T09 owner acceptance (unless the user separately
already accepted that exact subdecision); **overall rows C/D/E/H remain PENDING**,
all public/shared schema awaits owner/T01 co-freeze, no user-selected rev-42–46
decision is overridden, no `/6` freeze and no code; keeps the current-state pointer
at **rev 21–35** and the shared M1 proposal at **rev 36**); and rev 48 records the
**read-only T01 audit findings** (2026-10-05) — the checked-in `/5` code facts
(**24** `RECORD_KINDS` / **24** `RecordRef` variants, tags 0–23; `RecordFamily`
**absent**), the **`M1AppendSchema` hash-scope conflict** between
`CONTRACT_VERSION`/`contract.rs`/`README.md` and `COMPILER_SFL_MANIFEST.md` §4
confirmed **real and unresolved**, and the CDR's 19/27/27 draft counts marked
**proposed/unverified** — with the three hash-scope options **unselected and
unrecommended**, the requirement that `/6` specify **dual-inventory encoding**,
**numeric-value inclusion in the hash**, and a **schema self-consistency
mechanism**, and no owner acceptance or signature; docs-only, no
code/T01/manifest/other-doc edit, no `/6` freeze; keeps the current-state pointer
at **rev 21–35** and the shared M1 proposal at **rev 36**); and rev 49 records the
user's explicit **2026-10-05 critical H11/T09/T01 direction** — the `TerminatorMissing`
trigger **reuses the committed IR28 `FunctionEnd` terminal result**
(`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic
function-completion fact, checked by a **T01-owned typed phase-2b commit-apply
validation hook** (entry-block termination); **no** new marker record
family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder and **no** new
`ResultValue` variant; the earlier new-marker (`CompletedFunction`) direction is
**superseded for the operative direction** and preserved as history; **not a frozen
`/6` hook/schema, no code**, with the exact hook contract, hash impact (likely no new
record family; rule/hook hashing still a T01 decision), result typing/commit
ordering, and T09/T01 co-freeze remaining **open** and overall **H staying pending**
— and the **read-only H9 audit findings** recorded as **unresolved blockers** (the
`/5` code proves only the quota=1 single-task `fail_selected`, so the quota>1
pipeline is unimplemented; the dispatcher `Ready→Running`/`in_flight`-population
relationship to the single ordered atomic commit, the `in_flight` clear
ownership/order relative to the bounded H6 recovery, and the no-`Running`/residual
proof for all paths remain open; the `max_dispatches_per_tick`-vs-
`max_inflight_per_tick` internal proposed-limit conflict is deferred to `/6`; the
H6/H9 fan-out fixtures and T13 VF02/VF03/VF04/VF13 remain pending; the H9
no-residual guarantee is conditional on the H6 recovery implementation; the H9
removal direction and H6 mechanism are unchanged) — docs-only, no
code/T01/manifest/other-doc/test edit, no hash-scope option selected, no `/6` freeze;
keeps the current-state pointer at **rev 21–35** and the shared M1 proposal at
**rev 36**; and rev 50 records the user's explicit **2026-10-05 critical
HASH-SCOPE decision** (`[USER]`, two-tier model) — `StoreSchema::foundation +
M1AppendSchema` is the **frozen `/6` seed participating in the `/6` contract hash**,
while **post-seed runtime `StoreSchema::declare()` extensions remain excluded** from
the frozen hash and are **captured/validated via runtime snapshot/schema
mechanisms**; at `/6` integration T01 must **atomically** update the
`COMPILER_SFL_MANIFEST.md` §4 wording, the **`hash_excludes` semantic
description/token** (scoped to **post-seed runtime declarations**, not the frozen M1
seed), and `FrozenSchema::encode`/contract code plus the freeze test, **preserving
the frozen `/5` hash and history**; this **settles the conceptual `/6` boundary**
(the three former options are no longer open at the conceptual level) but
**accepts no exact `M1AppendSchema` contents/counts, freezes nothing, changes no
`/5`, and authorizes no code** — the **numeric inventory and freeze-test
implementation remain pending** and **T01 still co-freezes the M1 seed values after
the owners**; docs-only, no `/5`/T01/manifest/other-doc/code/test edit, no `/6`
freeze; **row I / overall rows C–I remain PENDING**; keeps the current-state pointer
at **rev 21–35** and the shared M1 proposal at **rev 36**; and rev 51 records the
**integration-selected candidate default under explicit user delegation**
(2026-10-05) — `max_inflight_per_tick`/quota is the **sole per-tick dispatch-count
bound** and the redundant `max_dispatches_per_tick` is **dropped** from the
candidate limit inventory and validation, grounded in the rev-35/38 H9 direction
and the rev-49 audit finding that the two limits duplicated each other; **not** an
owner/T01 `[INT]` signoff and **not** a schema freeze; the exact names/defaults/codes
remain **T01/T02 `/6` co-freeze** and the T02/T13 tests remain **pending**; it
selects no dispatcher-`Ready→Running`/`in_flight` atomic-boundary or clear-order item
and changes neither the H6 mechanism nor the H9 removal direction; docs-only, no
`/5`/T01/manifest/other-doc/code/test edit, no `/6` freeze; **row A / overall rows
C–I remain PENDING**; keeps the current-state pointer at **rev 21–35** and the shared
M1 proposal at **rev 36**. And rev 52 records the **read-only cross-owner
audit/reconciliation** (2026-10-05) — the completed owner packages mirror the
already-selected items (**T02 rev 30+31**; **T03/T04** incl. the T04-rev-46
`LiteralRecord.token` candidate and the correction to the typed `RecordLink`
mechanism with no `OwnBatch` confusion for record links; **T05 `ParseContext`
candidate**; **T06**; **T07 sem→const correction**; **T08 exact candidate shape**;
**T09 H11**; **T13**; **M1 vertical acceptance rev 27**; **M1 target acceptance
rev 29**; **ADR-0002 Revision 17**) with **no owner package called a `/6`
sign-off**; the M1 proposal/target/vertical docs align on H11, the hash boundary,
and the Part A-vs-B split and the proposal is being advanced in parallel to **rev
38**; the T01 audit still leaves `/6` **not freeze-ready** (exact
record/tag/family inventories, field bodies/ref encoding, task/request/result
typing, writer manifests, TU carrier/`ParseContext` final encoding, `SourceId`/
token provenance/raw-offset mapping and `Lx08CandidateType`, `SemRecord`/VF06,
const overflow/projection, the IR hook contract, the scheduler H6/H9 atomic
boundaries/in_flight clear order/no-residual fixtures, and the
snapshot/canonical-encoder/hash tests) with **no new user-critical decision**; the
rev-51 sole-per-tick-bound and rev-50 hash-scope decisions remain **selected** with
exact implementation/sign-off pending and **rows C–I PENDING overall**; and the CDR
is an **accepted-decision ledger/work queue, not an executable schema** whose next
integration stage is T01's serial `/6` seed assembly against the frozen two-tier
hash model, with **no chip coding before freeze** and **Part B independent**;
docs-only, no `/5`/T01/manifest/other-doc/code/test edit, no `/6` freeze. And rev 53 records
the **integration-agent selected candidate default under explicit user delegation**
(2026-10-05) for the M1 exercised literal subset — the `Lx08CandidateType` M1
vocabulary is the **closed one-member set `{ Int }`**, M1 literals `2`/`3` = **Int,
target-independent, no bit width**; forms **outside** the exercised M1 subset must
**not silently default to `Int`** and are **explicitly unsupported/deferred**; future
categories **append without reinterpretation**; **not** an owner/T04 or T01 `[INT]`
signoff, **not** a `/6` freeze, **not** the complete future C candidate vocabulary, no
numeric tags; the complete `Lx08CandidateType` member set/numeric encodings remain
open; docs-only, no `/5`/T01/manifest/other-doc/code/test edit, no `/6` freeze; rows
C–I remain **PENDING overall**; M1 proposal **rev 38** (current), ADR-0002 **Revision
17**, T02 **rev 31**. And rev 54 is the docs-only **read-only-audit +
historical-supersession cleanup** (2026-10-05): it re-marks the rev-21 §G2
`max_const_bits` signed-range/`i128` formula + `ConstRecord.value: i128` carrier +
chip-level enforcement as **historical/superseded — not selected** (retaining only
the rev-44 origin/cap/projection; formula/carrier/enforcement open), qualifies the
rev-49 H9 duplicate-limit conflict as **historical/superseded by rev-51** (history
retained; dispatch/clear-order open), and records as **open** the **T04 reciprocal
token↔literal cycle-safe ID reservation**, the **T05/T06 TU-edge exactly-once atomic
consumption/dedupe**, the **T13 no-residual fixtures incomplete**, and the **M1
vertical PP-08 failure semantics + exact artifact-map arrays pending**; no new
decision, no owner/T01 signoff, no `/6` freeze, no code; M1 proposal **rev 39**
(expected), ADR-0002 **Revision 17**, T02 **rev 32**. A later review correction to that same rev-41 §9C form (no new
revision) requires **one sub-entry per listed authority** in the `Response` and
`Status / signature` cells — every listed human authority responds/signs
independently for its own acceptance with no group or single-owner proxy, a row
stays **`PENDING`** until **all** listed authorities (and T01 `[INT]` where listed)
have separately signed, and T01 records `/6` acceptance only after every owner
signoff.
The render-time CDR revision is **rev 55**; the shared M1 proposal is at **rev 40** (earlier snapshots were rev 36–39).
ADR-0002 was updated through **rev 29** and now also **Revision 11 (rev-30)**
(the A/B in-principle decisions in its operative text, §1.1 M1 pointer
**rev 21–30**), **Revision 12 (rev-31)** (the §1.1 M1 pointer advanced to
**rev 21–31**, the ADR side of the history-gap repair; Revisions 9/10 are the prior
pointer and historical-documentation corrections), **Revision 13 (rev-32)** (the
H6 recovery direction + §1.1 pointer **rev 21–32**), **Revision 14 (rev-33/34)**
(the H9 in-flight-bound removal direction in §6/§5 and the §1.1 M1 pointer advanced
to **rev 21–34**), **Revision 15 (rev-35)** (the §6 in-flight-bound paragraph
stale-candidate cleanup + §1.1 M1 pointer **rev 21–35**), and **Revision 16
(rev-36)** (the §3 `bus.rs` amendment-row clarification that the preflight bound is
the **per-tick `max_inflight_per_tick`/quota only**, with no separate total
in-flight bound; §1.1 pointer unchanged at **rev 21–35**) and **Revision 17
(rev-51)** (the integration-selected candidate default: `max_inflight_per_tick`/
quota is the sole per-tick dispatch-count bound and the redundant
`max_dispatches_per_tick` is dropped; **not** an owner/T01 `[INT]` sign-off or a
schema freeze); the ADR is therefore at
**Revision 17**, and **CDR rev 54** points at it (as did CDR revs 52/53), and the current **CDR rev 55** points at it as well.
The
[M1 target acceptance](M1_TARGET_ACCEPTANCE.md) was updated through **rev 29**:
rev 26 made no substantive change to §7, and rev 27 is a docs-only correction of
the §7 failure-guarantee label from "Rev 26 F2" to **Rev 25 F2** (the H6/sentinel
paragraph was integrated at rev 25); rev 28 is a clarity-only gate-scoping edit
(no acceptance-criterion change), and rev 29 is the current-operative
reconciliation with the CDR-rev-49/50 decisions (the operative `TerminatorMissing`
`FunctionEnd` direction; partial decisions only; the two-tier hash boundary
accepted conceptually; no target acceptance passed). That target-acceptance rev 27
was made by
**CDR rev 30** (the docs-only scope/label correction) — so CDR rev 30 **did**
change the M1 target acceptance, recorded as its rev 27. By contrast, the
subsequent CDR docs-only revisions — **CDR rev 31** (the A/B decision record),
the **CDR rev 32** history-gap repair, and the **CDR rev 36** D3 correction —
made **no** change to the M1 target
acceptance (they touch only the CDR's own pointers and §13 disambiguation); the
target acceptance is at **rev 37** as of 2026-10-05 (subsequent docs-only revisions 30–37; see its own revision record).
[T02](T02_CONTROL_CHIPS.md) and the
[M1 frontend acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md): **T02** was updated
through **rev 25**, and additionally carries a **rev 26** row recording the A/B
in-principle decisions in its operative pipeline text, a **rev 27** row recording
the **H6 batch-failure recovery direction** in its operative CT06/pipeline text,
a **rev 28** row recording the **H9 `max_inflight_total` removal direction**
in its in-flight-bound bullet/CT06 row, and a **rev 29** row cleaning the stale
active candidate prose in that bullet (all still not an accepted contract or
freeze), and additionally a **rev 30** row syncing the T02-role 2026-10-05 user
subdecisions (H6 scope+mechanism, CT07 commit-apply invariant/no new carrier/
await-all, H9 re-affirmed) and a **rev 31** row mirroring the CDR-rev-51
sole-per-tick-bound candidate default (`max_inflight_per_tick`/quota sole bound;
redundant `max_dispatches_per_tick` dropped), and a **rev 32** row mirroring the
CDR-rev-54 read-only cleanup, so **T02 is at rev 35** as of 2026-10-05 (subsequent docs-only revisions 33–35; see its own revision record). The
**[M1 frontend acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md)** was
updated through **rev 27** (rev 26 is a clarity-only target-model/gate-legend edit
and rev 27 is the artifact-map scope alignment) and has **no applicable A/B,
H6/sentinel, H6-direction,
or H9 acceptance row** — it contains **no H6/CT06 content** — so its rev-27 record
remains operative (**D3 correction, rev 36:** an earlier §13 sentence wrongly
stated that the frontend carried a rev-26 H6/CT06 text; the frontend is at
**Rev 27** and the operative H6 batch-failure text lives in **T02**, not the
frontend). **T02 is where the H6 operative text lives.** **Current scope update (2026-10-05):** the frontend acceptance is at **rev 34** and T02 is at **rev 35** (each document's own revision record is authoritative for later changes); the operative H6 batch-failure text remains in **T02**, not the frontend. Rev 26–53 of this CDR made
no substantive change to the T02/frontend documents beyond the T02 rev-26 A/B
cross-record, the T02 rev-27 H6-direction cross-record, the T02 rev-28 H9
cross-record, and the T02 rev-29 stale-active-prose cleanup. **Rev 54 note:** the
CDR-rev-54 read-only cleanup (§G2 supersession, H9 duplicate-limit-conflict
supersession, and the open T04/T05-T06/T13/vertical findings) makes no
architecture-critical change and is mirrored as a **T02 rev 32/33** cross-record; the
corresponding T02 task-package text remains an **unedited, pending owner amendment**
(no task package is edited by this CDR). **Rev 42 note:** the
user's 2026-10-05 selection of the **H6 scope + mechanism** (freeze in `/6`; the
pre-dispatch/commit-failure/recovery-mutation/no-pre-reservation/state-guard
mechanism), the **D** CT07-invariant+await-all, the **F** carrier decisions, and
the **E** lowest-id scan are recorded in this CDR; the corresponding **T02/T05/
T06/T07/T09/T13 task-package text remains an unedited, pending owner amendment**
(no task package is edited by this CDR). **Rev 43 note:** the further 2026-10-05
**D/T05** `ContinuationRecord` exact ordered fields/shapes (no `awaited`), the
**D/T01/T05** formal `/6` T01 §4 supersession, the **D/T02/T05/T01** exact OwnBatch
pre-apply validation, and the **E/T06/T01** File Enter deterministic
`parse.TranslationUnit -> symbol_type.scope-enter` stage edge are recorded in this
CDR; the corresponding **T02/T05/T06** task-package text (and the T01 §4 catalog
entry) remains an **unedited, pending owner amendment** (no task package is edited
by this CDR; the T01 §4 supersession is a `/6`-inventory item only). **Rev 44 note:**
the further 2026-10-05 **C/H1/T07/T08/T01** `required_kind` per-use
constant-expression meaning + `legality` result-payload placement, the **C/T03/T01**
`ArtifactRecord { kind, source, bytes, raw_offsets }` shape + total
eight-`ArtifactKind` map rule (M1 scope only single-source `Normalized`), and the
**G/T08/T01** `max_const_bits` hashed-`Limits` origin / M1 cap-default 128 / config
`> 128` rejection / explicit task-input projection are recorded in this CDR; the
corresponding **T03/T04/T07/T08** task-package text (and the T01 hash/projection
detail) remains an **unedited, pending owner amendment** (no task package is edited
by this CDR). **Rev 45 note:** the further 2026-10-05 **C/T04/T08/T01** exact
`LiteralRecord` ordered fields + M1 `LiteralKind`/`LiteralSuffix`/radix scope +
symbolic `Lx08CandidateType`, the **C/T07/T08/T01** `RequiredKind`
(`IntegerConstantExpression`) and `ConstLegality`
(`Legal`/`NotConstantExpression`/`Unsupported`) values, and the **C/T03/T01**
mandatory artifact-map invariants are recorded in this CDR; the corresponding
**T03/T04/T07/T08** task-package text remains an **unedited, pending owner
amendment** (no task package is edited by this CDR; the exact `Lx08CandidateType`
member set/numeric encodings, the optional-kind artifact-map rule,
source-versus-payload equality, and the exact artifact error classification/numeric
codes remain open). **Rev 46 note:** the further 2026-10-05 **F/T07**
`ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` discriminants
(M1 fixture produces **`NonLvalue`**), the **F/T07** M1-only `EffectMask(0)` rule
(nonzero mask is a **typed unsupported/diagnostic**; effect bit classes
reserved/unassigned), the **F/T07/T13/T01** VF06 `TypedAstInvariant` **in M1**
(after committed T07 `SemRecord`s, before T09 lowering, M1
typed-fact/required-conversion completeness; exact registered task
kind/stage/phase/interface still **T01/T13 co-freeze/open**), and the
**F/T06/T07/T09/T01** **M1-minimal conversion scope** (freeze only M1-fixture-needed
behavior incl. identity/no-conversion; integer promotions, float conversions incl.
`FloatToFloat`, pointer qualifier, and other non-M1 conversions **explicitly
unsupported/deferred** to a later append/contract revision; **no** exact
`ConversionOp`/`ConversionRole` closed list, pairing, role→chip mapping, or numeric
encoding invented) are recorded in this CDR; the corresponding
**T06/T07/T09/T13** task-package text (and the VF06 exact registration) remains an
**unedited, pending owner amendment** (no task package is edited by this CDR; the
exact VF06 registration and all `ConversionOp`/`ConversionRole`/`ValueCategory`/
`EffectMask` numeric encodings remain open). **Rev 47 note:** the integration-agent
**selected candidate defaults under explicit user delegation** (2026-10-05) — the
optional-`ArtifactKind` map policy (**T03/T04**), T05 parse depth (**T05**), T06
namespace/symbol lookup (**T06**), and the T09 noncritical rule defaults (**T09**) —
are recorded in this CDR as **delegated candidate defaults, not owner signoffs**;
the corresponding **T03/T04/T05/T06/T09** task-package text remains an **unedited,
pending owner amendment** (no task package is edited by this CDR; the exact
`ParseContext`/request-result variants, the T05 `NodeKind`/token-range dependency,
the exact event encoding/lifecycle/allowlist rows, the exact numeric error codes,
and the `CompletedFunction` marker family remain open). **Rev 48 note:** the
**read-only T01 audit findings** (2026-10-05) are recorded in this CDR as
**facts/proposals only** — the checked-in `/5` code facts (**24** `RECORD_KINDS` /
**24** `RecordRef` variants, tags 0–23; `RecordFamily` **absent**), the
**`M1AppendSchema` hash-scope conflict** confirmed **real and unresolved**, and the
CDR's 19/27/27 draft counts marked **proposed/unverified**; the three hash-scope
options stay **unselected/unrecommended**, and the `/6` must specify
**dual-inventory encoding**, **numeric-value inclusion in the hash**, and a
**schema self-consistency mechanism**. This audit **edits no task package, T01,
manifest, or other document** and adds **no owner acceptance or signature**; the
corresponding schema/hash detail remains an **unedited, pending owner/T01
amendment**. **Rev 49 note:** the **user's critical H11/T09/T01 direction**
(2026-10-05) — the `TerminatorMissing` trigger **reuses the committed IR28
`FunctionEnd` terminal result** (`TaskState::Completed(ResultId)`, task kind
`FunctionEnd`) as the deterministic function-completion fact, checked by a
**T01-owned typed phase-2b commit-apply validation hook**, with **no** new marker
record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder or
`ResultValue` variant (the earlier new-marker direction is **superseded for the
operative direction**, preserved as history) — is recorded in this CDR as a
**selected direction** (not a frozen `/6` hook/schema, no code); the corresponding
**T09/T01** task-package text remains an **unedited, pending owner amendment**, with
the exact hook contract, hash impact (rule/hook hashing still a T01 decision),
result typing/commit ordering, and T09/T01 co-freeze **open**. The **read-only H9
audit findings** (2026-10-05) are recorded as **unresolved blockers** — quota>1
pipeline unimplemented; dispatcher `Ready→Running`/`in_flight`-commit relationship and
`in_flight` clear ownership/order open; no-`Running`/residual proof for all paths;
`max_dispatches_per_tick`-vs-`max_inflight_per_tick` conflict deferred to `/6`;
H6/H9 fan-out fixtures and T13 VF02/VF03/VF04/VF13 pending — with the **H9 removal
direction and H6 mechanism unchanged**, the audit **not a signature**, and the
corresponding **T02/T13** task-package text an **unedited, pending owner amendment**.
**Rev 50 note:** the **user's critical hash-scope decision** (`[USER]`, 2026-10-05)
— the **two-tier model** (`foundation + M1AppendSchema` = the **frozen `/6` seed
hashed**; **post-seed runtime `declare()` extensions excluded** and
captured/validated via runtime snapshot/schema mechanisms) — is recorded in this
CDR as a **conceptual-scope decision only**; the **`/6` integration-time** updates
(T1 `COMPILER_SFL_MANIFEST.md` §4 wording, the `hash_excludes` semantic
description/token scoped to **post-seed runtime declarations**, and
`FrozenSchema::encode`/contract code + freeze test, preserving the `/5` hash/history)
are **T01 integration work**, and the corresponding **T01 schema/hash/freeze-test
detail** remains an **unedited, pending owner/T01 amendment** (no task package,
T01, manifest, code, or test is edited by this CDR; the numeric inventory and
test implementation remain pending, and T01 still co-freezes the M1 seed values
after the owners). This record is **not** a row-I closure, **not** a `/6` freeze,
and **not** a T01 `[INT]` signature beyond the user's scope decision. **Rev 53 note:**
the **integration-agent selected candidate default under explicit user delegation**
(2026-10-05, not a role signoff) — the **`Lx08CandidateType` M1 vocabulary** for the
exercised M1 literal subset (closed one-member set **`{ Int }`**; M1 literals `2`/`3`
= **target-independent `Int`, no bit width**; forms **outside** the exercised M1
subset must **not silently default to `Int`** and are **explicitly
unsupported/deferred**; future categories **append without reinterpretation**) — is
recorded in this CDR as a **delegated candidate default**, **not** an owner/T04 or T01
`[INT]` signoff, **not** a `/6` freeze, **not** the complete future C candidate
vocabulary, and asserts **no numeric tag/encoding**; the corresponding **T04/T08**
task-package detail (the complete `Lx08CandidateType` member set and all numeric
encodings) remains an **unedited, pending owner/T01 co-freeze amendment**. The task
packages
**T03–T09 and T13 were not edited** (not
authorized) and therefore still contain the rev-18/19 wording. The following
amendments are **requested** and must be applied by the owning reviewer before
`/6`; they are not accepted here. Each is a documentation amendment only (no
interface is authorized by this CDR).

| Task doc | Requested amendment | Why |
|---|---|---|
| [T03](T03_PREPROCESS_CHIPS.md) | State explicitly that `sources.spans` is **T03-only** (independently of the T01-owned store partition) and that T04/T05 write no spans. **Rev 22:** state the M1 one-source limit of `source_scoped_one_hop` and the multi-source/include gap; state that non-M1 `ArtifactKind`s (`Assembly`/`Object`/`Snapshot`/`Trace`) have no M1 writer, and that `SpanRecord` offsets are proposed `u64`. **Rev 44 (accepted subdecision, 2026-10-05):** record `ArtifactRecord { kind, source, bytes, raw_offsets }` with the **total eight-`ArtifactKind`** map rule (Normalized/Spliced/CommentFree/Preprocessed map-mandatory; Assembly/Object/Snapshot/Trace map-optional) and the M1 **exercised scope only single-source `Normalized`**; the exact `raw_offsets` invariant/error mapping and enum numeric codes remain open. **Rev 45 (accepted subdecision, 2026-10-05):** record the **mandatory-map invariants** (`raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds require a valid source); the optional-kind map rule, source-versus-payload equality, and exact artifact error classification/numeric codes remain open. **Rev 47 (delegated candidate default, 2026-10-05; not a T03 owner signoff):** record the selected **optional-`ArtifactKind` map policy** — map-optional kinds have empty `raw_offsets`; `source` valid when `Some`; no source-payload-equals-bytes requirement; exact numeric error codes open. | Removes the rev-20 shared-writer ambiguity; records the one-source limit, the unassigned artifact writers, and the selected M1 artifact shape/map scope at the package level. |
| [T04](T04_LEX_CHIPS.md) | State that `TokenRecord.span` **references the committed T03 PP span** and that T04 creates no `SpanRecord`; **rev 25 F3:** record the `LX08` candidate type as allocated **in principle** to the per-literal `LiteralRecord` (the proposed `candidate_type` field), with the exact enum/type encoding **unfrozen** and a T04/T08 `/6` co-freeze item — the earlier "without asserting which carrier wins" wording is withdrawn as superseded by the H1 allocation. **Rev 22–24:** under the **H1 allocation accepted in principle (2026-10-04)**, `LiteralRecord` is a lexical candidate fact carrying the per-literal lexical facts + `LX08` candidate type and **no `node`/`required_kind`** (those are on the sem-stage `ConstantRequest`, which also references the committed literal — rev 25 F3; `legality` on the result) — record it as the accepted-in-principle working basis, **not a freeze**; state that `LX14` adjacent-string and `LX16` synthesized/pasted token provenance needs a typed T04 carrier; and that committed `TokenRecord.span` must expose a verifiable PP-token provenance, not merely same-source. **Rev 44 (2026-10-05):** the rev-44 selected subdecisions do **not** cover the exact `LiteralRecord` fields/enum variants (or any lexical enum), which remain a **T04 co-freeze open item** — no T04 acceptance is claimed. **Rev 45 (accepted subdecision, 2026-10-05):** record the **exact ordered `LiteralRecord` fields** (`token: Option<TokenId>`, `kind: LiteralKind`, `radix: u8`, `suffix: LiteralSuffix`, `value: Vec<u8>` big-endian magnitude, `negative: bool`, `spelling: Vec<u8>`, `candidate_type: Lx08CandidateType`; no `node`/`required_kind`) and the M1 enum/scope `LiteralKind {Integer, Character, String}` (only `Integer` produced) / `LiteralSuffix {None, U, L, UL, LL, ULL}` (only `None` produced) / radix `{2,8,10,16}` (M1 decimal only); the **complete `Lx08CandidateType` member set/numeric encodings remain open** (not invented). **Rev 53 (delegated candidate default, 2026-10-05; not a T04 owner signoff):** record the selected **`Lx08CandidateType` M1 vocabulary** for the **exercised M1 literal subset** — the closed one-member set **`{ Int }`**, M1 literals `2`/`3` = **target-independent `Int`, no bit width**; forms **outside** the exercised M1 subset must **not silently default to `Int`** and are **explicitly unsupported/deferred** until their categories/rules exist; future categories **append without reinterpretation**; **not** the complete future C candidate vocabulary, no numeric tags; remaining members/encodings stay a T04/T08/T01 `/6` co-freeze item. | Aligns T04 with the single-owner and literal-handoff directions; surfaces the provenance blockers **and the H1 working-basis allocation**. |
| [T05](T05_PARSE_CHIPS.md) | State that AST `NodeRecord` uses a **first/last `TokenId` range** and that T05 writes no `sources.spans`; remove any `awaited`-on-continuation wording; state that join reinsertion happens in commit apply into the task's own `stage_queues`. **Rev 22/23/31:** record the `awaited`-vs-T01 §4 supersession as **accepted in principle** (user decision B, 2026-10-04: `TaskState::Waiting(WaitSet)` is the sole awaited-child-ID source, no duplicate `ContinuationRecord.awaited`; the T01 §4 supersession is a formal `/6` amendment only, **no `/5` edit**; **T05 owner + T01 integrator acceptance pending**); node token links are **committed-only** (no same-batch `TokenDraft` — the branch is withdrawn, H5); resume stage is `stage_of(parent.kind)` and the realization handles an absent `continuation` (H2); all-`Completed`→`Ready` and any-`Failed`→`Failed` once, never `Ready`, with fail-fast/sibling-cancel policy still open (H4); CT07's committed join-decision carrier is open; parse depth counts parser-chain frames. **Rev 47 (delegated candidate default, 2026-10-05; not a T05 owner signoff):** record the parse-depth default — reuse `limits.max_task_depth`, count parser continuation/child frames only, detect before any child enqueue for the descent, excess = `ParseDepthExceeded` chip diagnostic; exact `ParseContext`/request-result variants still open. | Removes the span blocker; records the accepted-in-principle T01 §4 supersession (T05/T01 acceptance pending) and the remaining open T05 protocol items. |
| [T06](T06_SYMBOL_TYPE_CHIPS.md) | State the exact `Identifier`-leaf `decl` node and that the Block node's token range supplies the scope boundary `at`; classify `SymbolConflict` (commit) vs `RedeclarationConflict`/`NoSuchSymbol` (chip); state the `ChipId`-keyed allowlist and TY08 non-M1 status consistently. **Rev 22/23:** the point-of-declaration derives from the **token range** (no `NodeRecord.span`); the **File-scope `Enter` is a T06 task after the T05 `TranslationUnit` is committed, pinning a committed `NodeId`**, with the bootstrap order a T06/`[INT]` `/6` decision (not job-bootstrap, H3); define the canonical `TypeId` **reuse lookup mechanism**; the namespace validation is chip-level/narrowed (no namespace carrier); fix the `NORMATIVE_RULES`/test-list duplication. **Rev 47 (delegated candidate default, 2026-10-05; not a T06 owner signoff):** record the selected namespace/symbol-lookup default — closed `SymbolKind`→namespace mapping (no field), miss-not-conflict, deterministic TY03 active-scope-chain lookup (`(source,start,end,NodeId)` ordering, greatest ≤ point, same-scope higher `SymbolId`, else innermost); T05 `NodeKind`/token-range dependency and exact event encoding/lifecycle/allowlist rows still open. | Removes ambiguity the CDR would otherwise hide; surfaces the reuse-lookup, namespace-carrier, and file-scope-ordering blockers. |
| [T07](T07_SEMANTIC_CHIPS.md) | State that the checked return/function facts are carried by T07-owned committed `SemRecord`s (including `Return`/`FunctionDefinition`) and are **not** routed through a T09-created `FunctionRecord`; add the explicit `ConversionOp` signedness set and the VF06 `(NodeKind, role, op)` matrix. **Rev 22:** reconcile `CheckedNode`↔`SemRecord` identity and define the T09 consumption link; resolve the `FunctionContextId` requirement (`CheckRequest` still names one); add `FloatToFloat`/pointer-qualifier ops and the role→T09-chip mapping; fix the VF06 `FunctionDefinition` row and mark VF06 stage as unwired. **Rev 45 (accepted subdecision, 2026-10-05):** record `RequiredKind` M1 enum only `IntegerConstantExpression` and the `ConstLegality` values `Legal`/`NotConstantExpression`/`Unsupported`; future C constant-expression purposes require appended variants/new rules, not a repurposing of the lexical candidate type. **Rev 46 (accepted subdecisions, 2026-10-05):** record `ValueCategory { Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3 }` (M1 fixture produces **`NonLvalue`**) and that M1 allows **only `EffectMask(0)`** (a nonzero mask is a **typed unsupported/diagnostic**; the effect **bit classes are reserved/unassigned**); record that VF06 **`TypedAstInvariant` is in M1** (executes after committed T07 `SemRecord`s and before T09 lowering, checking M1 typed-fact/required-conversion completeness; the exact registered task kind/stage/phase/interface is a **T01/T13 co-freeze/open** item); record the **M1-minimal conversion scope** — freeze only the conversion behavior the M1 fixture needs, including the identity/no-conversion rule, while integer promotions, float conversions incl. `FloatToFloat`, pointer-qualifier conversions, and other non-M1 conversions are **explicitly unsupported/deferred** to a later append/contract revision; do **not** invent an exact `ConversionOp`/`ConversionRole` closed variant list, pairing, role→chip mapping, or numeric encoding. | Removes the sem→ir cycle and the signedness-loss risk; surfaces the carrier/link and domain blockers. |
| [T08](T08_CONSTANT_LAYOUT_INIT_CHIPS.md) | State the `ConstantRequest` protocol as the literal-decode (T04) and semantic (T07) flows; record `max_const_bits`/formula/enforcement; keep `ConstOverflow`/`ConstUnsupported` as chip diagnostics. **Rev 22–24:** record the **H1 allocation accepted in principle (2026-10-04)** — the lex-stage literal request carries the committed `LiteralRecord` (per-literal lexical facts + `LX08` type) and **no `node`/`required_kind`**; `node`/`required_kind` are on the sem-stage `ConstantRequest` and `legality` on the result — as a **working basis, not a freeze**; define the exact committed `legality` carrier and the exact `LiteralRecord.candidate_type` encoding (the assigned `LX08` carrier — the carrier is **not** in doubt; only its enum/type encoding is pending) as part of the T04/T08 `/6` co-freeze; define the `max_const_bits` numeric carrier, the `<=128` config constraint, and how a restricted chip receives the bound; clarify the signed-representative wording for unsigned magnitudes. **Rev 44 (accepted subdecisions, 2026-10-05):** `ConstantRequest.required_kind` means the **per-use constant-expression requirement** (e.g. M1 integer constant expression), distinct from the lexical `LiteralRecord.candidate_type`, with no duplicate implicit target type; `ConstantResult.legality` is a **result payload field** with no extra committed family (exact `ConstLegality` variants remain open); and `max_const_bits` origin is a **hashed `Limits` value** with M1 cap/default **128**, `config` **rejects > 128**, and an **explicit task-input projection** (exact wire field/diagnostic numeric code/hash encoding remain `/6`; the cap is not a claim that every valid config value equals 128). **Rev 45 (accepted subdecisions, 2026-10-05):** record `RequiredKind` M1 enum only `IntegerConstantExpression`, and the `ConstLegality` result-field values `Legal`/`NotConstantExpression`/`Unsupported`; future C constant-expression purposes require appended variants/new rules, not a repurposing of the lexical candidate type; numeric encodings remain open. | Aligns T08 with the handoff and bounds directions; surfaces the carrier/config blockers **and the H1 working-basis allocation plus the rev-44 `required_kind`/`legality`/`max_const_bits` and rev-45 `RequiredKind`/`ConstLegality` subdecisions**. |
| [T09](T09_IR_LOWER_CHIPS.md) | State the `TerminatorMissing` trigger **direction** (H11); the `Constant` immediate = result-type rule; a rule id per rejection **as prospective `/6`**. **Rev 22/23:** reconcile the rejection→rule-id list with `NORMATIVE_RULES` — **both** `ir.op-immediate-type` aliases and **both** `ir.terminator-missing*` candidates are unresolved (H8); reword folded-`int5` to `folded_int5_ir_constant` (T08 computes the value, T09 emits IR — H7); state `Constant` missing-immediate vs missing-result precedence. **Rev 47 (delegated candidate default, 2026-10-05; not a T09 owner signoff):** record the selected shorter aliases `ir.op-immediate-type`/`ir.terminator-missing`, immediate-before-result precedence when `Constant` misses both, and target type = result `ValueRecord.ty`; all prospective `/6`. **Rev 49 (accepted direction, 2026-10-05; user's critical H11/T09/T01 decision; not a frozen `/6` hook/schema and no code):** record that the `TerminatorMissing` trigger **reuses the committed IR28 `FunctionEnd` terminal result** (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`) as the deterministic function-completion fact, checked by a **T01-owned typed phase-2b commit-apply validation hook** (entry-block termination), with **no** new marker record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder or `ResultValue` variant; the earlier new-marker (`CompletedFunction`) direction is **superseded for the operative direction** (preserved as history); exact hook contract, hash impact (rule/hook hashing still a T01 decision), result typing/commit ordering, and T09/T01 co-freeze remain open. | Removes the undefined "function end" trigger direction; records the selected `FunctionEnd`-terminal-result fact and surfaces the unresolved rule inventory. |
| [T13](T13_VERIFICATION_CHIPS.md) | State the pending batch-verification amendment: VF02 (exactly-one transition per dispatched task across the batch; no dead-wait), VF03 (wire lifetime with a batch selection), **VF04** (batch write-conflict access audit — proposed/residual, **not** defined today), and VF13 (replay across dispatch order); add fixtures for quota=1 canonical-projection equivalence, multi-inflight determinism, cross-task conflict ordering, and backpressure; **VF12 is `IrInterpretChip` and is unrelated unless explicitly extended.** **Rev 34 (H6 direction, 2026-10-04):** add the pending batch-verification amendment for the **all-dispatched failure recovery** — that on a failed atomic semantic batch commit **every** dispatched task is verified to transition **exactly once** to `Failed` in dispatch order (a committed `DiagnosticId` when capacity allows, else the `TaskState::Failed(DiagnosticId::NONE)` sentinel), so **no dispatched task remains `Running`** (the no-`Running` sentinel), and that **clearing the in-flight set is not itself a transition**. **Rev 42 (H6 mechanism selected, 2026-10-05):** amend the batch fixtures for the selected mechanism — pre-dispatch errors before state mutation leave tasks `Ready`, a semantic batch commit failure commits no semantic state, the deterministic bounded recovery mutation transitions dispatched tasks once in dispatch order, no pre-reservation of N diagnostics, per-task `DiagnosticId::NONE` when capacity is insufficient, and the state guard preventing duplicate transition. **Rev 46 (VF06 selected in M1, 2026-10-05):** state that VF06 **`TypedAstInvariant` is in M1**, running **after** the committed T07 `SemRecord`s and **before** T09 lowering, checking **M1 typed-fact/required-conversion completeness**; the **exact registered task kind/stage/phase/interface remains a T01/T13 co-freeze/open** item, and the VF06 registration/verification interface is **not** fixed here. | The pipeline `/6` freeze requires the T13 verification packages to be amended together; the H6 scope+mechanism is user-selected (rev 42) but the T13 fixtures/sign-offs remain **pending**; **T13 was not edited** (H10/rev 34); the VF06-in-M1 subdecision (rev 46) is recorded here with its exact registration still open and T13 fixtures/sign-off pending. |
| [T02](T02_CONTROL_CHIPS.md) | **Rev 26 (updated in this revision; T02-owner acceptance and T01 implementation-confirmation pending):** record the A/B in-principle decisions in the CT07/own-batch operative text — a same-task `OwnBatch(DraftRef)` is a **transient wire/proposal input only**, validated and resolved to committed IDs **before persistent state**, so no durable cursor/`WaitSet`/join points at a draft/wire/address (guardrail §6.1 text unamended); `TaskState::Waiting(WaitSet)` is the **sole** awaited-child-ID source and `ContinuationRecord` carries no duplicate `awaited`, with the **T01 §4 supersession a formal `/6` amendment only (no `/5` edit)**. **Rev 27 (updated in this revision; T02-owner acceptance and T01 implementation-confirmation pending):** record the **H6 batch-failure recovery direction** in the CT06/pipeline operative text — every dispatched task fails exactly once in dispatch order (a committed `DiagnosticId` when capacity allows, else the `DiagnosticId::NONE` sentinel), so every task leaves `Running`; clearing the in-flight set is not itself a transition; the exact atomic/bounded implementation requires T01 integrator approval and the batch failure-atomicity details/diagnostic budget-state/T02+T13 owner fixtures remain co-freeze/pending. **Rev 28 (updated in this revision; T02-owner acceptance and T01 integrator acceptance pending):** record the **H9 `max_inflight_total` removal direction** — accept removing `max_inflight_total` from the `/6` candidate because sequential per-tick dispatch is bounded by `max_inflight_per_tick`/quota, the stage queues, and `max_tasks_total`; `Waiting` is not in-flight; `tasks.in_flight` is an ephemeral per-tick scheduler batch only (cleared at latch after each dispatched task has a terminal/`Waiting`/`Progress` outcome or H6 recovery); the dispatcher's `Ready→Running` pre-worker mutation is distinguished from the ordered atomic semantic commit; a future cross-tick `Running` mode needs a separate CDR — direction only, no bound/config/hash frozen. **Rev 29 (updated in this revision; docs-only cleanup):** the T02 in-flight-bound bullet now opens with the **removal direction** as the current operative statement and marks the rev-22/23 one-phase-owner framing as **superseded history retained**, so no stale candidate prose remains active. **Rev 42 (owner-amendment request; H6 scope+mechanism selected 2026-10-05):** record in the T02 CT06/pipeline operative text the selected H6 mechanism — pre-dispatch errors before state mutation leave tasks `Ready`; a semantic batch commit failure commits no semantic state; the deterministic bounded recovery mutation transitions dispatched tasks once in dispatch order to `Failed`; no pre-reservation of N diagnostics; per-task diagnostic attempt with `DiagnosticId::NONE` when capacity is insufficient; state guard prevents duplicate transition; the `/6` freeze is of this selected mechanism (no later-CDR deferral). | The A/B decisions are user-accepted in principle (2026-10-04) but not owner/integrator-approved; T02 must not read them as a freeze, and the T01 §4 supersession still requires T01 integrator and T05 owner acceptance. The H6 scope+mechanism is user-selected (rev 42) but is not an implemented or frozen mechanism and remains a pending T02 amendment; the H9 removal direction is pending T01 integrator acceptance and the residual H9 `[INT]` items stay open. |

If an owner disagrees, the affected statement reverts to its prior blocker
status; no task package is edited by this CDR (T03–T09 and **T13** are all
unedited) and no claim is made that these amendments are accepted. The M1 proposal
`§17` (rev 22), `§18` (rev 23/24), `§19` (rev 25), and `§20` (rev 26) is the authoritative
per-finding disposition.
