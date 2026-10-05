# M1 NEG/UNS Fixture Audit — 2026-10-05

## Status and scope

- **Status:** read-only audit record. It is **not** an ADR, contract approval,
  interface freeze, acceptance result, or implementation authorization. No
  audited file was modified; no code, commit, or push.
- **Snapshot:** the working tree at 2026-10-05 ~08:08 local. The tree was
  **under concurrent documentation editing** during this audit; findings were
  re-verified against the snapshot below, and the concurrent changes are
  recorded in §0.1.
- **Audited object:** `docs/tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md`
  (**rev 32**, published during this audit): §6.1 `M1-NEG-01..19`, §6.2
  `M1-UNS-01..09`, §7 owner/readiness rows, §8 gates G7/G8, §9 non-normative
  items.
- **Cross-checked:** T01–T08 task packages, `M1_PART_A_CONTRACT_PROPOSAL.md`
  (rev 40 header; §9 allocation; §24 OB ledger), the CDR,
  `M1_TARGET_ACCEPTANCE.md`, and the frozen `/5` foundation
  (`compiler/src/diagnostic.rs`, `compiler/src/routing.rs`,
  `compiler/tests/c06_routing.rs`).
- **Method:** line-by-line cross-check of **owner chip**, **diagnostic
  class/obligation**, **gate**, and **finite recovery** for every NEG/UNS row.
  No fixture was executed; no Cargo check was run.

### 0.1 Concurrent-edit record

The following changes landed while this audit was running and are reflected in
the findings below:

1. `M1_VERTICAL_SLICE_ACCEPTANCE.md` advanced to **rev 32** ("§7 chip-list and
   gate-scope alignment", `:509`). The §7 rows now include `PP26`
   (malformed-only), `LX09`/`LX12`/`LX13`/`LX18` (diagnostic-only), `PA18`
   (`M1-NEG-19` only), and scope `TY31` to the `M1-NEG-12`/`M1-NEG-13`
   constraint negatives; `PP02`/`PP03` are marked **identity-only**; G4/G7/G8
   are re-scoped. This resolves the **vertical-side** owner-list findings
   (F1/F3/F5/F8 below); the corresponding proposal §9 allocation and the T03
   PP26 role definition remain open.
2. `M1_PART_A_CONTRACT_PROPOSAL.md` §9 corrected the stale `M1-NEG-14`
   dialect-policy label; it now correctly splits `M1-NEG-14` (mode-independent)
   from `M1-NEG-19` (dialect-policy) (`:3154`). The stale-label defect is no
   longer present.
3. `T02_CONTROL_CHIPS.md`, `T07_SEMANTIC_CHIPS.md`, and
   `M1_TARGET_ACCEPTANCE.md` received small edits (T02's CT11 row moved to
   `:118`; `M1_TARGET_ACCEPTANCE.md` N2/N4 moved to `:413`/`:415`; T07 advanced
   to rev 5/6 with unrelated `SemRecord`-checklist corrections). Line anchors
   here are as of the snapshot and may drift again.

## Overall result

- The NEG table's gates are coherent, and the rev-32 §7 alignment closes the
  owner-list omissions **inside the M1 vertical**. Three gaps remain open:
  **TY31**, **SE01**, and **PP26** — now as cross-file/definition issues rather
  than vertical omissions.
- The UNS table still has no owner-chip and no gate columns (`OB-45`,
  proposal §24) and no per-row termination wording; G8 (`:432`) now scopes
  the mode/policy rows (`M1-UNS-05/06/08`).
- The **diagnostic class vocabulary and numeric codes** the NEG rows depend on
  remain unfrozen; the expected-result column is a prose obligation, not yet an
  executable assertion.
- Per-row **finite-recovery endpoints** remain absent for many NEG rows even
  though G7 (`:431`) now requires a finite recovery for the normative rows.

## 1. Findings index

| # | Finding | Level | Status |
|---|---|---|---|
| F1 | `TY31`: vertical rev 32 exercises it via `M1-NEG-12/13`, but proposal §9 still defers it and its kind allocation has no TY31 entry | high | open (proposal side) |
| F2 | `SE01`: catalog-only owner carries executable M1 gates on `M1-NEG-14/19` | high | open |
| F3 | `PP26`: now in the vertical (malformed-only), but still absent from proposal §9 (`OB-46`) and its T03 definition does not match the assigned role | high | open (proposal/T03 side) |
| F4 | Diagnostic class vocabulary and numeric codes are unfrozen; no NEG/UNS row names a frozen class | medium | open, T01 `/6` |
| F5 | Diagnostic-only chips `LX09`/`LX12`/`LX13`/`LX18`/`PA18` are in vertical rev 32 but still absent from proposal §9 (only `LX16` is recorded, as `OB-46`) | medium | open (proposal side) |
| F6 | UNS rows lack owner-chip and gate columns (`OB-45`) and per-row termination wording | medium | open |
| F7 | Finite-recovery wording absent from 11 of 19 NEG rows despite G7's finite-recovery requirement | medium | open |
| F8 | `PP02`/`PP03` scope: vertical rev 32 reconciled to identity-only; T03 rev 55 still says M1 does not exercise them | low | mostly closed; wording tension |
| F9 | `M1-UNS-09` is already satisfied by the frozen `/5` foundation but has no gate/owner column | observation | recorded |

## 2. M1-NEG per-fixture audit

Owner status is measured against the rev-32 §7 M1 rows
(`M1_VERTICAL_SLICE_ACCEPTANCE.md:369-375`): T03 `PP01–PP05, PP09, PP17,
PP19, PP25–PP26, PP28` (PP02/PP03 identity-only; PP26 malformed-only); T04
`LX01–LX09, LX12, LX13, LX16–LX18` (LX09/LX12/LX13/LX18 diagnostic-only);
T05 `PA01–PA09, PA16, PA18, PA20, PA22, PA24, PA28, PA32, PA34, PA38` (PA18
only via `M1-NEG-19`); T06 `TY01–TY03, TY07, TY09, TY10, TY13, TY17, TY20,
TY25–TY27` plus `TY31` **negative-only** via `M1-NEG-12/13`; T07 `SE02, SE07,
SE21, SE29` (`SE01` catalog-only).

| Fixture | Row chip(s) | Owner in §7 rev-32 M1 rows? | Diagnostic obligation as written | Gate | Finite recovery | Flags |
|---|---|---|---|---|---|---|
| `M1-NEG-01` | PP03, PP26 | PP03 yes; PP26 yes (malformed-only) | one unterminated-comment diagnostic | T01 freeze | explicit (consumes to EOF, finite) | F3 (proposal/T03 role) |
| `M1-NEG-02` | PP05, PP26 | yes (PP26 malformed-only) | unknown or unsupported directive diagnostic | T01 freeze | **absent** | F3, F7 |
| `M1-NEG-03` | PP19 | yes | structural conditional diagnostic at EOF | T01 freeze | implicit (frame drained) | — |
| `M1-NEG-04` | LX01, LX18 | yes (LX18 diagnostic-only) | one unknown-character diagnostic at the `@` span | T01 freeze | explicit (advance one byte, no loop) | F5 (proposal §9) |
| `M1-NEG-05` | LX09 | yes (diagnostic-only) | one malformed-float-syntax diagnostic | T01 freeze | **absent** | F5, F7 |
| `M1-NEG-06` | LX12, LX13 | yes (diagnostic-only) | unterminated-literal diagnostic with span | T01 freeze | explicit (finite recovery) | F5 |
| `M1-NEG-07` | LX07, LX08 | yes | range or constraint diagnostic | T01 + target probe | **absent** | F7 |
| `M1-NEG-08` | PA32, PA34, PA38 | yes | one expected-`;` diagnostic | T01 freeze | explicit (sync at `}`, block closes) | — |
| `M1-NEG-09` | PA28, PA38 | yes | unterminated-compound diagnostic at EOF | T01 freeze | implicit (EOF, scope cleanup once) | — |
| `M1-NEG-10` | PA09, PA38 | yes | malformed parameter list diagnostic | T01 freeze | explicit (to `{` or `;`) | — |
| `M1-NEG-11` | PA22, PA34, PA38 | yes | expected-operator-or-`;` diagnostic | T01 freeze | explicit (at `;`, one error) | — |
| `M1-NEG-12` | PA02, TY31 | PA02 yes; TY31 negative-only | constraint diagnostic (initializer on function) | T01 freeze | **absent** | F1, F7 |
| `M1-NEG-13` | TY17, TY31 | TY17 yes; TY31 negative-only | named `void` parameter is a constraint violation | T01 freeze | **absent** | F1, F7 |
| `M1-NEG-14` | SE01 (catalog-only) | **no** | one undeclared-identifier diagnostic (mode-independent) | T01 freeze | **absent** | F2, F7 |
| `M1-NEG-15` | SE21 | yes | mode/policy-sensitive `return;` in `int` function | T01 + dialect policy | **absent** | F7; excluded by G7 |
| `M1-NEG-16` | CL03, SE07 | yes | signed overflow; non-normative per §9 | T01 + overflow policy | **absent** | F7; excluded by G7 |
| `M1-NEG-17` | CL03 | yes | division-by-zero diagnostic; no crash | T01 freeze | **absent** (only "no crash") | F7 |
| `M1-NEG-18` | CL03 | yes | shift count at/above operand width yields a diagnostic | T01 + target probe | **absent** | F7; concrete assertion probe-gated (G7) |
| `M1-NEG-19` | PA18, SE01 | PA18 yes (`M1-NEG-19` only); SE01 **no** | mode-sensitive implicit-declaration constraint diagnostic | T01 + dialect policy | **absent** | F2, F7; excluded by G7 |

G7 (`:431`) now states the normative set explicitly: `M1-NEG-01`–`M1-NEG-14`
and `M1-NEG-17`–`18` must produce exactly one primary diagnostic with the
expected location **and a finite recovery**; `M1-NEG-15`/`16`/`19` are excluded
until their policy is declared; `M1-NEG-18`'s concrete-width diagnostic
assertion is probe-gated.

### 2.1 Diagnostic class

No NEG row names a diagnostic **class identifier**; each row states a count and
a location ("one expected-`;` diagnostic", "unterminated-literal diagnostic with
span"). The class/category vocabulary and all numeric codes are explicitly
deferred:

- `M1_VERTICAL_SLICE_ACCEPTANCE.md` §9 (`:476-480`): the unsupported-result
  encoding (a distinct `Unsupported` diagnostic class versus `Failed` with a
  specific code) "is a T01 protocol decision and is not fixed here"; `:481-482`:
  "Fixtures assert diagnostic category and location, not message text or numeric
  codes."
- `T04_LEX_CHIPS.md:153`: the exact diagnostic class/category vocabulary,
  numeric codes, and hash encoding remain **T01 `/6`** details.
- `T05_PARSE_CHIPS.md` item G.7 and item H: numeric codes remain T01 `/6`
  details; `T06_SYMBOL_TYPE_CHIPS.md` item 6f: symbol/scope error classification
  is a T01 co-freeze item; `T07_SEMANTIC_CHIPS.md` §4: diagnostic codes and the
  rule-id inventory are T01/T07/T13 open; `T08_CONSTANT_LAYOUT_INIT_CHIPS.md`
  "still open": numeric diagnostic codes remain `/6`.
- The frozen `/5` foundation (`compiler/src/diagnostic.rs:37-54`) provides only
  the coarse `DiagGroup { Protocol, Arena, Config, Target, Manifest, Task,
  Unsupported, Internal }` with `DiagnosticCode { group, code: u16 }`; there is
  **no language (preprocess/lex/parse/symbol/sem/const) diagnostic family**, so
  even the family for a "syntax error" or "undeclared identifier" is not frozen
  today. `DiagnosticDraft::unsupported` is `DiagGroup::Unsupported` code `1`
  (`:116-118`).

Consequence: the NEG expected-result column cannot be converted into an
executable assertion until T01 `/6` freezes the diagnostic class vocabulary; the
rows assert no code by design, but they also do not yet reference a frozen
class. Proposed names used elsewhere must not be read as frozen
(`ArtifactSourceMissing`/`ArtifactSourceMismatch`/`ArtifactMapInvalid` in T03;
`ConstOverflow`/`ConstUnsupported` in T08; `ParseDepthExceeded`,
`ContinuationRefInvalid`, `AwaitChildrenRefInvalid` in T05;
`ScopeLifecycleViolation`, `SymbolConflict`, `RedeclarationConflict`,
`NoSuchSymbol` in T06; `UnsupportedNode`, `UnsupportedIrOp` in T09).

The "Chip(s)" column is also not a diagnostic-emitter manifest: it mixes
detecting and recovery chips, and the diagnostic record writer is T02 `CT11
DiagnosticCommitChip` (`T02_CONTROL_CHIPS.md:118`, listed in §7 rev 32 at
`M1_VERTICAL_SLICE_ACCEPTANCE.md:375`), which appears in no §6.1 row.

### 2.2 Finite recovery

§6 preamble (`M1_VERTICAL_SLICE_ACCEPTANCE.md:295-298`) and G7 (`:431`) require
every normative malformed case to produce a structured diagnostic **and a finite
recovery**. Per-row recovery endpoints are explicit for `M1-NEG-01`, `04`, `06`,
`08`, `10`, `11` (and implicit for `03`, `09`); they are **absent** for
`M1-NEG-02`, `05`, `07`, `12`, `13`, `14`, `15`, `16`, `17`, `18`, `19`. The
gate-level obligation covers these rows, but the row-level expected result
cannot be checked for termination/progress without a stated synchronization
point (F7).

### 2.3 Gates

Gates are present and coherent per row, including the symbolic/probe split
(`M1-NEG-07/18` are `T01 + target probe`) and the mode-sensitive tier
(`M1-NEG-15/19`). `M1-NEG-16` is non-normative per §9. G7 now excludes the
policy rows and probe-gates `M1-NEG-18`'s concrete assertion. The gate problem
is not the gate column itself but that two gated rows name owners outside M1
(F1, F2).

## 3. M1-UNS per-fixture audit

The §6.2 table (`M1_VERTICAL_SLICE_ACCEPTANCE.md:324-340`) has only three
columns: Fixture, Variant, Expected frontend outcome. It has **no owner-chip and
no gate column** (`OB-45`, proposal §24), and no per-row termination
wording. G8 (`:432`) supplies the general obligation and now scopes
`M1-UNS-05`/`06` to an "explicitly declared modeled outcome" and
`M1-UNS-05`/`08` to a declared mode/policy.

| Fixture | Variant | Row chip(s) | Row gate | Expected encoding | Owner path (audit-derived, not document text) | Flags |
|---|---|---|---|---|---|---|
| `M1-UNS-01` | `float`/`double`/`long double` use | — | — | explicit unsupported result | T04 `LX09`/`LX10`; T06/T07/T08 positive support deferred beyond M1 | F4, F6 |
| `M1-UNS-02` | `struct`/`union`/`enum` | — | — | explicit unsupported result | T05 `PA11`/`PA12`; T06 `TY18`/`TY19` | F4, F6 |
| `M1-UNS-03` | pointer/array declarator | — | — | explicit unsupported result; no silent mis-parse | T05 `PA05`–`PA08` | F4, F6 |
| `M1-UNS-04` | variadic `int f(int, ...)` | — | — | explicit unsupported result | T05 `PA09` | F4, F6 |
| `M1-UNS-05` | old-style definition with empty parameter list | — | — | explicit unsupported or explicitly modeled no-prototype function | T05 `PA02`/`PA09`; G8 mode/policy scope | F4, F6 |
| `M1-UNS-06` | `#include` or function-like macro | — | — | explicit unsupported unless a no-op capability is declared | T03 `PP06`/`PP17`; no no-op capability declared in M1; G8 conditional | F4, F6 |
| `M1-UNS-07` | GNU attribute, `typeof`, statement expression, inline `asm` | — | — | explicit unsupported result | T05 `PA36`/`PA37`; T12 | F4, F6 |
| `M1-UNS-08` | `void main(void)` | — | — | mode/policy decision; must not be accepted as conforming | T06 `TY31` (negative-only per rev 32); G8 mode/policy scope | F1-adjacent, F4, F6 |
| `M1-UNS-09` | a task kind that is not implemented | — | — | foundation `control.unsupported` path must fail explicitly | T02/C06; **already implemented in `/5`** (see F9) | F6, F9 |

The "owner path" column is this audit's cross-reference to the chip catalogs; it
is **not** in the audited document. The document assigns no owner for any UNS
row, so no UNS row can be scheduled, gated, or reported as a chip gap/pass as
written.

## 4. Flags

### F1 — TY31: vertical rev 32 vs proposal §9

- Vertical rev 32 (§7 T06 row, `M1_VERTICAL_SLICE_ACCEPTANCE.md:372`): `TY31`
  "is exercised by M1 **only** through the constraint-negative fixtures
  `M1-NEG-12`/`M1-NEG-13` (G7), not by the positive `M1-TY-*` rows, and its
  wider positive-path scope stays deferred"; G4 (`:428`) repeats the
  negative-only scope. This resolves the vertical's own former self-contradiction.
- Proposal §9 (`M1_PART_A_CONTRACT_PROPOSAL.md:3153`) still says
  "`TY31 DeclarationConstraintChip` is deferred beyond M1", and the §9
  Symbol/type kind list has no TY31 entry. `OB-46` (proposal §24) does not
  cover TY31.
- Remaining defect: align proposal §9 with rev 32 (add the TY31 negative kind
  or a negative-only note). As written, a reader of the allocation table alone
  still sees TY31 as out of M1.

### F2 — SE01: catalog-only owner vs executable gate

- `M1-NEG-14` (chips `SE01 (catalog-only; M1-NEG-14 owner)`, gate `T01 freeze`)
  and `M1-NEG-19` (chips `PA18, SE01 (catalog-only; M1-NEG-19 owner)`, gate
  `T01 + dialect policy`) (`M1_VERTICAL_SLICE_ACCEPTANCE.md:317,322`).
- §7 rev 32: "`SE01` is catalog-only and owned by the negative fixtures
  `M1-NEG-14`/`M1-NEG-19`" (`:373`); the T07 M1 chip list is `SE02, SE07,
  SE21, SE29`. Proposal §9 (`:3159-3164`): "`SE01 NameExpressionChip` ... is
  catalog-only for M1 and is not listed as an M1 semantic rule." T07's chip
  table includes SE01 (`T07_SEMANTIC_CHIPS.md:13`), but the M1 scope does not.
- The stale proposal §9 label calling `M1-NEG-14` the dialect-policy fixture
  was corrected during this audit (concurrent edit); the current snapshot
  correctly splits `M1-NEG-14` (mode-independent) from `M1-NEG-19`
  (dialect-policy). That wording defect is **no longer present**.
- Remaining defect: neither row can be executed or reported after `T01 freeze`
  while its owner is catalog-only. The rows need an explicit catalog-only/gap
  annotation (not a pass), or SE01 must be added to the M1 chip list; the
  mode-sensitive gate on `M1-NEG-19` does not resolve the owner gap.

### F3 — PP26: now in the vertical, still absent from proposal §9 and mismatched to its T03 definition

- `M1-NEG-01` (chips `PP03, PP26`) and `M1-NEG-02` (chips `PP05, PP26`)
  (`M1_VERTICAL_SLICE_ACCEPTANCE.md:304-305`). Vertical rev 32 §7 T03 row now
  lists `PP25–PP26` and says "`PP26` only by the malformed fixtures
  `M1-NEG-01`/`M1-NEG-02`" (`:369`). That closes the vertical-side omission.
- Proposal §9 Preprocess row is unchanged: "(PP01–04, PP28)" with no PP26
  (`M1_PART_A_CONTRACT_PROPOSAL.md:3150`); this remains `OB-46` (§24).
- T03 still defines PP26 as `PpDiagnosticChip` for `Error/WarningDirective →
  Diagnostic` (`#error`/`#warning`) with acceptance "not emitted in inactive
  regions; correct span" (`T03_PREPROCESS_CHIPS.md:34`). That does not describe
  the owner role the fixtures assign it (an unterminated-comment diagnostic and
  an unknown-directive diagnostic). Per the same chip table the relevant chips
  are PP03 (unterminated comment; acceptance includes "unterminated", `:11`)
  and PP05 (directive recognition, `:13`).
- Result: either PP26 is a misassignment, or the intended generic PP diagnostic
  emitter role must be specified in T03 and the chip added to the §9 allocation.
  The vertical's malformed-only note does not by itself define that role.

### F4 — Diagnostic class vocabulary and numeric codes are unfrozen

- Every NEG row's expected result is a diagnostic **count + prose category**
  with no class identifier; §9 states fixtures assert category and location,
  not codes (`M1_VERTICAL_SLICE_ACCEPTANCE.md:481-482`), and the UNS result
  encoding (`Unsupported` class vs `Failed` + code) is explicitly unfixed
  (`:476-480`).
- The category vocabulary itself is deferred in every owner package (T04
  `:153`; T05 items G.7/H; T06 item 6f; T07 §4; T08 "still open"), and the
  frozen `/5` `DiagGroup` has no language family
  (`compiler/src/diagnostic.rs:37-54`). Only `DiagGroup::Unsupported` code `1`
  exists for the foundation path (`:116-118`).
- Consequence for the gate: `T01 freeze` is the correct gate, but until T01
  `/6` freezes the language diagnostic class vocabulary, the NEG expected
  results remain non-executable as written. This is a readiness fact, not a
  claim that the fixtures are wrong; it should be stated in the NEG table's
  legend so the rows are not mistaken for executable tests today.

### F5 — Diagnostic-only chips in vertical rev 32 but absent from proposal §9

Vertical rev 32 adds the M1-exercised diagnostic-only members
(`M1_VERTICAL_SLICE_ACCEPTANCE.md:369-371`): T03 `PP26`; T04
`LX09`/`LX12`/`LX13`/`LX18`; T05 `PA18` (`M1-NEG-19` only). The proposal §9
allocation still reads: Preprocess "(PP01–04, PP28)" (`:3150`), Lex
"(LX01–08, LX17–18)" (`:3151`), Parse "(PA01–09, PA16, PA20, PA22, PA24,
PA28, PA32, PA34, PA38)" (`:3152`). `OB-46` (§24) records only the missing
`LX16` and `PP26`; the rev-32 diagnostic-only `LX09`/`LX12`/`LX13`/`PA18`
additions are not recorded anywhere in the allocation ledger. Align §9 with
rev 32 (and update `OB-46` accordingly).

### F6 — UNS table structure (`OB-45`) and per-row termination

- `M1-UNS-01..09` have no `Chip(s)` and no `Gate` columns
  (`M1_VERTICAL_SLICE_ACCEPTANCE.md:324-340`), already recorded as `OB-45`
  (proposal §24). G8 (`:432`) supplies the general obligation and the
  mode/policy scoping, but there is no per-row owner to schedule or per-row
  gate to report.
- No UNS row states a termination/recovery endpoint; the general §6 preamble
  ("Every unsupported case must produce an explicit unsupported or failed
  result", `:295-298`) and G8 cover it, but the rows should state that
  termination is a typed failure, not a hang. `M1_TARGET_ACCEPTANCE.md` N2/N4
  (`:413`/`:415`) show the intended typed outcomes for the language and control
  cases.

### F7 — Finite-recovery wording absent in 11 NEG rows

Explicit/absent status is listed in §2 above. The rows with no recovery endpoint
are `M1-NEG-02`, `05`, `07`, `12`, `13`, `14`, `15`, `16`, `17`, `18`, `19`.
For the mode/policy and non-normative rows (`15`, `16`, `19`) the expected
result is not fixed anyway; for the others the expected result should state the
synchronization point or "recovery to EOF" so finite progress is checkable
under G7.

### F8 — `PP02`/`PP03` scope: mostly reconciled

Vertical rev 32 marks `PP02`/`PP03` "exercised **identity-only** on the M1
canonical path ... splice/comment-removal semantics deferred, T03 rev 54/55"
(`M1_VERTICAL_SLICE_ACCEPTANCE.md:369`), and §5.1 keeps `M1-PP-02`/`M1-PP-03`
as identity rows (`:173-174`). This resolves the earlier conflict with T03 rev
54's scope guard (`T03_PREPROCESS_CHIPS.md:305-307`). The remaining wording
tension: T03 rev 55 still says "M1 does **not** exercise PP02/PP03"
(`:319`) while the vertical exercises them identity-only; a one-line
cross-reference would remove the ambiguity. `M1-NEG-01` remains the only
unterminated-comment exercise, and T03 rev 55 §2.5 anchors its beyond-M1 PP03
contract on that fixture (`:377`).

### F9 — Observation: `M1-UNS-09` is already satisfied by the frozen `/5` foundation

`TaskKind::CONTROL_UNSUPPORTED` (`compiler/src/task.rs:153`) resolves to
`Resolution::Unsupported` and produces `Proposal::Fail` with
`DiagnosticDraft::unsupported` (`compiler/src/routing.rs:222,287-293`); the
frozen test `unsupported_task_fails_explicitly` asserts the task ends
`TaskState::Failed` with one committed diagnostic
(`compiler/tests/c06_routing.rs:85-105`). The M1 vertical row is a planned
fixture, so this is not a pass claim, but the row's expected outcome is already
current `/5` behavior rather than T01-freeze-gated language work. The row
should say so and should not be counted as an M1 language-chip pass.

## 5. Recommended doc-only follow-ups

1. **TY31 (F1):** align proposal §9 with vertical rev 32 — add a TY31
   negative-only kind/note or record the vertical decision in the allocation
   ledger.
2. **SE01 (F2):** add an explicit "owner catalog-only; not executed in M1; gap,
   not pass" annotation to `M1-NEG-14/19` (or admit SE01 to M1), and keep the
   corrected mode-independent/dialect split.
3. **PP26 (F3):** remove or replace the PP26 assignment in `M1-NEG-01/02` with
   the detecting/recovery chips (`PP03`, `PP05`), or define PP26's generic PP
   diagnostic role in T03 and add it to proposal §9 (`OB-46`).
4. **Diagnostic class (F4):** add a legend note that the NEG/UNS diagnostic
   category vocabulary and numeric codes are T01 `/6`-unfrozen and that rows
   assert category/location only; name the expected class per row once frozen.
5. **Allocation alignment (F5):** add the rev-32 diagnostic-only chips to
   proposal §9 and update `OB-46`.
6. **UNS table (F6):** add `Chip(s)` and `Gate` columns (`OB-45`), mark the
   mode-sensitive rows (`M1-UNS-05/06/08`) and the TY31-negative
   `M1-UNS-08`, and state the typed termination.
7. **Recovery wording (F7):** add the synchronization/EOF endpoint to the 11
   rows that lack one.
8. **UNS-09 (F9):** annotate it as an already-implemented foundation path
   (`/5`), not a language-chip M1 pass.

## 6. Verification record and limitations

- `git diff --check` on the working tree: **clean** (exit 0) at the snapshot;
  the new untracked audit file was additionally checked for trailing whitespace
  and tabs (none found).
- No Cargo `fmt`/`clippy`/`test`, no fixture execution, no candidate compile,
  and no Linux/AArch64 probe was run. This audit reports documentation
  consistency only and does not establish compiler capability or acceptance.
- The working tree was concurrently edited during the audit (see §0.1); line
  numbers and §7 content are as of the stated snapshot and may drift again.
- This audit does not modify the audited documents and does not claim to have
  exhausted all NEG/UNS defects.
