# Type/Conversion Documentation Audit — 2026-10-05

**Status.** Read-only audit record. Findings only: this file applies no fix, closes no open blocker, and is **not** an authority decision, accepted contract, ADR, `/6` freeze, or implementation authorization. The audited documents remain the authority for their own statuses.

**Scope.** The type/conversion documentation set: T06 (scope/symbol/type chips), T07 (semantic chips), T13 (verification chips), the M1 Part A contract proposal, the M1 pipeline/schema CDR, and the M1 vertical/target acceptance plans. Audit topics: (1) symbolic `TypeRecord`; (2) canonical `TypeId` reuse scan predicate; (3) `TY25`–`TY27` vs CDR rev 46; (4) identity-plan recorded-vs-open; (5) `ConversionPlan` shape; (6) VF06 domain.

**Baseline.** Working tree as read 2026-10-05 07:55–08:10, including the uncommitted/untracked documents (`M1_PART_A_CONTRACT_PROPOSAL.md`, `CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md`, T02–T09/T13 packages, M1 acceptance docs, `docs/reviews/2026-10-05_DOCUMENTATION_REVIEW.md`). Several audited files were being edited concurrently during the audit; citations use section/item names first, with line numbers only as a reading aid.

**Method.** Read-only cross-document consistency inspection; no code, tests, or probe executed. Already-recorded open items were used as the reference set to avoid double-counting: proposal §24 OB-20, OB-23, OB-27, OB-51; proposal §17.3 T06-2/T06-4, §17.4 T07-3/T07-4; T07 §2 S5/S6 and §4; T13 §2 items 3/5/6.

**Overall result.** The core shapes are mutually consistent (`TypeRecord { kind: TypeKind }`; one shared `ConversionPlan { op, role, from, to }`; committed `TypeId` vs draft `RecordLink<Type>`; symbolic no-width `int`/`int(void)`). The material inconsistencies are: the accepted canonical-`TypeId` scan has no defined predicate or scan bound while several records present it as actionable; the identity-plan recorded-vs-open question is stated inconsistently and is not tracked as an open item; and the VF06 M1 fixture/domain text conflicts with the selected M1-minimal conversion scope (partly tracked as OB-23) and with the per-node typed-fact obligation.

## Findings

### TC-01 — Canonical scan: "lowest matching" predicate and scan bound undefined; stale "mechanism absent" text

- Severity: **medium**. Status: open; predicate/bound already recorded in OB-20, but the accepted-subdecision records do not carry that qualifier and two proposal sections still say the mechanism itself is undefined.
- Locations: CDR §E4 (rev 42) and row E / §9A / §9B / §13 ("deterministic bounded scan of the committed `types.records` returning the lowest matching `TypeId`, no hidden cache/index"); T06 §Operative amendment items 1, 6d, 6f; proposal §5 `TypeRecord` reuse-policy comment ("the concrete reuse lookup mechanism is not defined", rev-22 blocker) and §17.3 T06-2/T06-4; proposal §24 OB-20.
- Issue: "matching" is undefined. The same proposal states there is **no structural-equality identity for `int`** (§5), and the §7 pre-mutation properties say `TypeRecord` identity is the chip-specific single-producer rule with **no structural dedup or compatibility used as identity**; the only body available to a scan is `kind: TypeKind`, so the scan predicate is not derivable from the accepted text. The scan bound is also undefined. T06 item 1/6d and CDR row E list only the allowlist rows/seed/hash as remaining co-freeze, so the accepted subdecision reads as actionable; conversely proposal §5 and §17.3 T06-2/T06-4 still read as pre-rev-42 text ("mechanism absent/undefined") without marking the mechanism-kind supersession.
- Recommendation: record the matching predicate and scan bound explicitly as remaining T06/T01 co-freeze in T06 item 1/6d and CDR row E (cross-reference OB-20); mark §5 / §17.3 T06-2/T06-4 text as superseded for the mechanism kind by rev 42.

### TC-02 — Identity plan: recorded vs open

- Severity: **medium**. Status: open; not tracked by any OB row.
- Locations: proposal §5 `ConversionPlan` comment ("M1 restricts the exercised `(op, role)` set: `(Identity, Return)` for the return conversion and `(Identity, Result)` on an explicitly recorded no-op"); CDR §F3 table (`Return` row: "required **only** when the operand type differs from the function return type; an explicit recorded `Identity` plan may be present when the owner records a no-op"; `Identifier`/`IntegerConstant` row: "**optional** (M1 records no conversion when the type is already `int`)"); T07 §2 S5/S6 and §4 "function/return facts" row (whether an explicit no-op `Identity` plan is recorded is **OPEN**); T13 §2 item 3(a)/(b); M1 vertical `M1-SE-02` ("no inserted conversions"), `M1-SE-03`/`M1-TY-09` ("identity").
- Issue: the proposal sentence can be read as asserting that the M1 fixture exercises both `(Identity, Return)` and `(Identity, Result)` with an explicitly recorded no-op, while the CDR F3 `Result` row says M1 records no conversion when the type is already `int` and T07 S6 says the recorded-vs-absent question is OPEN. No document states whether the M1 fixture's `conversions` vector is empty or contains an `Identity` entry, and no OB row tracks it.
- Recommendation: state the M1 fixture behavior once (empty vector vs explicit `Identity` entry) and mark the question as a `/6` item; align proposal §5 wording with T07 S6 / CDR F3; add the item to OB-20 or a new OB row (T06/T07/T09).

### TC-03 — `TY25`–`TY27` vs rev 46: conflict remains open (OB-51); Part B vs deferred non-M1 conflated

- Severity: **medium-high**. Status: already recorded as OB-51 (open); this audit confirms it and adds wording details.
- Locations: T06 rows `TY25`/`TY26`/`TY27` and the "M1 Part A qualifier" after the table; M1 vertical `M1-TY-07`/`M1-TY-08`/`M1-TY-09` (T01 freeze); CDR row F rev 46 / §F2; proposal §24 OB-51; T09 "M1 conversion scope".
- Issue: rev 46 defers integer promotions (and other non-M1 conversions) to a later append/contract revision, while M1 still exercises `TY25` (`M1-TY-07`: "integer promotion of `int` yields `int`, unchanged"), `TY26` (`M1-TY-08`: "usual arithmetic conversion yields common type `int`, no inserted casts"), and `TY27` (`M1-TY-09`: return identity) as M1 `T01 freeze` rows. T06's qualifier reinterprets the exercised cases as "symbolic identity", but the boundary between the in-scope identity/no-conversion rule and the deferred promotion/common-type behavior is not defined, and the M1 acceptance rows keep the promotion/common-type descriptions. In addition, T06's qualifier lumps "probe-gated Part B" (target-dependent, same M1 scope) with "non-M1 conversions" (deferred to a later revision), which are different dispositions.
- Recommendation: keep OB-51 open; add the Part B-vs-deferred distinction to T06's qualifier; make one authoritative statement whether `M1-TY-07`/`M1-TY-08` are "identity/no-conversion facts" (in M1-minimal scope) or must be reclassified/deferred.

### TC-04 — `ConversionPlan` one-type statement not carried into T06; role→chip mapping open while `M1-TY-09` attributes `TY27`

- Severity: **low-medium**. Status: partially recorded (proposal §17.4 T07-3; CDR §F2).
- Locations: proposal §5 `ConversionOp`/`ConversionRole`/`ConversionPlan` block; CDR §F2; T06 rows `TY27`/`TY28`/`TY29`; M1 vertical `M1-TY-09`; proposal §17.4 T07-3.
- Issue: the proposal and CDR define `ConversionPlan` as the one shared type, with T06 `CastPlan` (`TY29`) and `ArgumentPlans` (`TY28`) as role-labelled instances; the T06 rows present `ConversionPlan`/`ArgumentPlans`/`CastPlan` as three outputs without that reconciliation. `M1-TY-09` attributes the return conversion to `TY27`, while the `TY27`-instance→role mapping and the role→T09-chip mapping are named open blockers (T07-3). `TY25`/`TY26` outputs (`PromotedType`/`CommonType`) are not `ConversionPlan`, so their M1 no-conversion outcome has no stated plan/role representation.
- Recommendation: add the one-shared-type note to the T06 rows or the M1 qualifier; state the `TY25`/`TY26` M1 output as "no plan emitted (identity)" pending the role mapping; cross-reference T07-3.

### TC-05 — VF06 completeness fixture contradicts the M1-minimal conversion scope

- Severity: **medium-high**. Status: OB-23 records the domain question, but T13 §2 item 3(c) already asserts one answer.
- Locations: T13 §2 item 3(c) vs item 6; CDR §F3 rev-46 note; proposal §5 `SemRecord` conversions comment ("in M1 the only possible required plan is a `Return`/`Return` plan when the operand type differs from the function return type"); OB-23.
- Issue: item 3(c) requires a VF06 verification failure for a missing required conversion using non-M1 examples (a `Return` whose operand type differs from the return type; `BinaryExpression` operands differing from the usual-arithmetic common type). Rev 46 defers exactly those conversions, and item 6 says the VF06 M1 fixture set must not assert the future matrix. OB-23 explicitly asks whether non-M1 required conversions are VF06 failures or unsupported diagnostics; 3(c) answers "verification failure" without authority.
- Recommendation: qualify 3(c) on the OB-23 decision, or restrict M1 VF06 fixtures to M1-permitted required conversions (none beyond the identity case) and place the non-M1 examples in unsupported-diagnostic fixtures.

### TC-06 — VF06 domain statement in the proposal is narrower than rev 46 / T13 item 2

- Severity: **medium**. Status: not recorded as an OB item.
- Locations: proposal §5 comment "VF06's scope is exactly that matrix, not unspecified 'all nodes'"; T13 §2 item 2 (typed fact for every checked `NodeId` the lowering path consumes); CDR rev-46 VF06 definition ("M1 typed-fact/required-conversion completeness").
- Issue: rev 46 gives VF06 two obligations (typed-fact completeness per checked node and required-conversion completeness against the matrix). The proposal sentence limits VF06's scope to the conversion matrix and explicitly excludes "all nodes"; as written it conflicts with T13 item 2's per-node typed-fact check.
- Recommendation: reconcile the sentence — conversion completeness is matrix-scoped; typed-fact completeness covers every checked `NodeId` the lowering path consumes.

### TC-07 — VF06 acceptance phrase "missing promotion" is non-M1

- Severity: **low**. Status: related to OB-23 / TC-05.
- Locations: T13 VF06 table row `Specified Acceptance` ("missing promotion, wrong lvalue, unchecked call") and the appended rev-46 note; T13 §2 item 6.
- Issue: "missing promotion" is a deferred non-M1 conversion and must not be a required M1 VF06 behavior ("must not over-enforce"); the appended rev-46 note does not remove the phrase.
- Recommendation: mark the phrase non-M1/future in the row.

### TC-08 — `EffectMaskUnsupported` carrier ambiguity reaches VF06

- Severity: **low**. Status: already recorded as OB-27.
- Locations: T13 §2 item 5 ("a nonzero effect mask is a typed unsupported/diagnostic and must not be accepted as a pass"); proposal §6.4 (`CommitError`); CDR rev 46 (chip typed unsupported/diagnostic); OB-27.
- Issue: OB-27 requires exactly one carrier among `CommitError`, chip `DiagnosticDraft`, and VF06 failure; T13 item 5's wording implies VF06 rejection without choosing the carrier. VF06's domain depends on the choice.
- Recommendation: keep OB-27 open; once chosen, restate item 5 against the chosen carrier.

### TC-09 — `TypeKind` M1 subset vs the T06 full-corpus type list

- Severity: **low**. Status: not recorded.
- Locations: T06 opening paragraph (type records cover scalar, pointer, array, function, struct/union, enum, qualified/atomic, GNU vector/complex); T06 §Operative amendment item 6a `TypeKind`; proposal §5 `TypeKind`; M1 target IR-6.
- Issue: the M1 `TypeKind` closed set is `{Void, Bool, Char, Int, Function}`; the package's opening states a much larger corpus scope. The M1 set is not labelled as a subset to which pointer/array/aggregate/qualified variants must be appended later (consistent with the "explicitly unsupported, not silently omitted" principle), and `TypeKindDraft` is referenced in the T06/proposal draft tables without a definition.
- Recommendation: label `TypeKind` as the M1 closed subset; state that later variants append (no reinterpretation); define or explicitly mark `TypeKindDraft` as shorthand.

### TC-10 — CDR §F closure criterion includes non-M1 conversion-op coverage

- Severity: **low**. Status: not recorded.
- Locations: CDR §F closure criterion (`conversion_op_signedness`, `conversion_at_most_one_plus_vf_completeness`, IR07 sign/zero-extend evidence) vs the rev-46 M1-minimal scope note in the same section.
- Issue: the closure criterion still lists signedness-op/IR07 evidence without a rev-46 future/non-M1 qualifier, while the same section says the enumerated `ConversionOp` set and `(op, role)` pairing are not frozen. A reader can treat the closure criterion as current `/6` scope.
- Recommendation: mark those closure tests future/non-M1, or state the closure criterion is superseded for M1 scope by rev 46.

## Recorded-elsewhere summary

| Finding | Already recorded? | Where |
|---|---|---|
| TC-01 predicate/bound | yes (partially) | OB-20; not in T06 items 1/6d or CDR row E |
| TC-02 identity recorded-vs-open | no | T07 S6/§4 only |
| TC-03 TY25–27 vs rev 46 | yes | OB-51 |
| TC-04 ConversionPlan one-type/role mapping | yes (partially) | T07-3; CDR F2; not in T06 rows |
| TC-05 VF06 completeness fixture | yes (partially) | OB-23; T13 item 3(c) over-specifies |
| TC-06 VF06 domain scope | no | — |
| TC-07 "missing promotion" | related | OB-23 / T13 item 6 |
| TC-08 EffectMask carrier | yes | OB-27 |
| TC-09 TypeKind M1 subset | no | — |
| TC-10 CDR §F closure criterion | no | — |

## Verification record and limitations

- `git diff --check` was run on the working tree after this file was written; no whitespace errors were reported by the command.
- This file itself was checked for trailing whitespace and CRLF line endings (none).
- No code, tests, cargo command, or Linux/AArch64 probe was run; no `/5` or `/6` artifact was touched.
- Several audited documents were under concurrent edit during the audit (file mtimes 08:07–08:08); line numbers may drift. Every citation was checked against the reading at 2026-10-05 ~07:55–08:10.
- This file does not claim to be exhaustive, does not fix or close any finding, and does not change any document's status.
