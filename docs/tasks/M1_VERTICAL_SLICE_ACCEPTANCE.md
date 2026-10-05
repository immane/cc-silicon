# M1 Vertical-Slice Acceptance (Frontend Half): Fixtures for `int main(void){return 2+3;}`

Status: **planned test design, not implemented capability.**

This document specifies the independent frontend fixtures for the M1 vertical
slice. It is a design artifact for the preprocessing, lexical, parse/declarator,
symbol/type, integer-semantics, and constant-evaluation stages of
`int main(void){return 2+3;}`.

The following are explicitly true of this document:

- **No tests were executed.** Every expected result below is hand-derived from
  ISO C rules and the frozen target *identity*; nothing here is a measured
  result.
- **It is not evidence that any chip, bus, or compiler exists.** The compiler
  application is planned work under `docs/tasks/`; this document does not create
  code, interfaces, or directories.
- **Language schemas and record IDs remain provisional.** The C01–C06
  foundation envelope is recorded as `t01-c01-c06/5`, but it does not freeze
  language-store record bodies or per-group task/result payloads. IDs such as
  `TokenId`, `TypeId`, `NodeId`, `ConstId`, and `DiagnosticId`, and catalog
  labels such as `PP04`, `LX08`, `PA22`, do not become binding for language
  fixtures until their owning contracts are frozen.
- **The M1 language mode is declared per fixture; it is not inferred as the
  compiler default.** See section 4.
- **Concrete ABI values are unverified.** Target identity is frozen
  (`aarch64-unknown-linux-gnu`, ELF, LP64, little-endian, AAPCS64), but every
  scalar width/alignment, `long double` format, and ABI register value remains
  UNVERIFIED until the planned Linux probe reports `verified=true`
  ([ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md)).
  The Linux probe substrate is **planned and not provisioned**; no probe has
  run. The probe (and every integration/downstream/Part B gate) constrains
  **only** the fixtures or assertions that depend on concrete target values or
  on a runnable end-to-end artifact — it does **not** gate the symbolic Part A
  fixture path, the semantic expectations, or the `/6` schema freeze. Symbolic
  Part A fixtures (for example `M1-CL-03`, `M1-TY-11`) are target-independent
  and are blocked only by the T01 freeze, not by the probe.
- **No GCC/Clang candidate compilation and no pass-rate claim.** No reference
  compiler output is used to derive an expected value, and M1 fixtures are not
  the T00 GCC torture rate.

This is a **draft/planned** acceptance document. It may be superseded when T01
freezes, but it must not be read as a claim that the described behavior is
implemented.

---

## 1. Scope

This document covers the M1 frontend only:

- translation phases 1–3 (preprocessing) for the canonical source,
- final C tokenization of that source,
- parsing and declarator binding for the single function definition,
- symbol/type construction for `main` and its function type,
- integer semantics of `2 + 3`,
- constant evaluation of the integer constant expression,
- malformed/unsupported/replay/write-scope robustness cases.

Downstream IR, ABI, and assembly acceptance is mentioned only as a gate
(`M1-INT-01`); it is owned by T09/T11 and is not specified here. The full M1
end-to-end milestone definition lives in
[docs/tasks/README.md](README.md) section 4 and
[M1_TARGET_ACCEPTANCE.md](M1_TARGET_ACCEPTANCE.md).

Out of scope for this fixture set: every C feature not needed by the canonical
source. Unsupported-feature fixtures in section 6.2 exist only to prove that
out-of-M1 input fails explicitly, not to enumerate the feature catalog.

## 2. Authority and precedence

Read together with:

- repository rules: [AGENTS.md](../../AGENTS.md)
- compiler task master plan: [docs/tasks/README.md](README.md)
- contract and protocol: [T01_COMPILER_CONTRACT.md](T01_COMPILER_CONTRACT.md)
- parallel/handoff discipline: [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md)
- per-chip template: [TASK_TEMPLATE.md](TASK_TEMPLATE.md)
- acceptance gate: [T00_GCC_TORTURE_GATE.md](T00_GCC_TORTURE_GATE.md)
- framework semantics: [SILICON_PARADIGM_SPEC.md](../architecture/SILICON_PARADIGM_SPEC.md),
  [SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md),
  [SFL_SCHEMA_DRAFT.md](../architecture/SFL_SCHEMA_DRAFT.md)
- design: [ARCHITECTURAL_BLUEPRINT.md](../design/ARCHITECTURAL_BLUEPRINT.md)
- storage/target decision: [ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md)

Precedence follows AGENTS.md section 2: explicit user instruction, applicable
accepted ADR, accepted engineering contract, architecture design, then existing
implementation. Because T01 is not frozen, this document cannot bind chip
interfaces; if a fixture conflicts with a later T01 freeze, the freeze wins and
the fixture is updated in one place.

Expected values are derived from ISO C clause behavior (cited informally, for
example "ISO C 6.7.6.3"), not from any vendored standard text. No ISO C standard
document is stored in this repository.

## 3. Canonical source and boundary variants

| Fixture | Exact bytes | Length | Role |
|---|---|---:|---|
| `M1-SRC-000` | `int main(void){return 2+3;}` + `\n` | 28 | Canonical M1 source (matches the milestone spelling) |
| `M1-SRC-001` | canonical without the final `\n` | 27 | Missing-final-newline boundary |
| `M1-SRC-002` | canonical with `\r\n` | 29 | Line-ending-normalization boundary |
| `M1-SRC-003` | `int main(void) { return 2 + 3; }` + `\n` | 33 | Whitespace-invariance boundary; must yield the same semantics as the canonical source with different spans |
| `M1-SRC-004` | `int main(void){return 2+ +3;}` + `\n` | 30 | Two separate `+` punctuators, never a `++` token |

Byte layout of `M1-SRC-000`, 0-based half-open spans:

```text
0..3 int | 3 sp | 4..8 main | 8..9 ( | 9..13 void | 13..14 ) | 14..15 {
15..21 return | 21 sp | 22..23 2 | 23..24 + | 24..25 3 | 25..26 ;
26..27 } | 27..28 \n
```

Whitespace and newline handling is part of the preprocessing fixtures; span
values are location records, not semantic tokens, and must never determine
semantics (T03 acceptance, T04 tail note).

## 4. M1 language-mode policy

To keep the default dialect unsettled, every fixture in this document declares
its mode explicitly. No fixture may rely on `CompilerConfig::default()` or any
other implicit dialect choice.

- The M1 frontend fixtures are written against a **declared** mode in the
  ISO C family. The canonical source
  `int main(void){return 2+3;}` is intentionally chosen so that the frontend
  result is invariant across C89, C99, C11, C17, and C23 (and their GNU
  variants) for the specific facts listed below. It does not follow that the
  compiler's default is C11 or any other value; [T00](T00_GCC_TORTURE_GATE.md)
  section 2 requires the default to be confirmed by reference-release probes.
- Invariant across those modes for this source: keyword identity of `int`,
  `void`, and `return`; `(void)` as a prototype with zero parameters; decimal
  integer literal type selection for 2 and 3; integer promotion and usual
  arithmetic conversion of `int + int`; and `2 + 3 == 5`.
- Mode-sensitive fixtures in section 6 are labeled `T01 + dialect policy` and
  may not be assigned an expected result until the mode is declared and the
  default-dialect question is settled. They exist to force an explicit choice,
  not to assume one.
- The fixtures assert **no** macro dialect default, predefined-macro set, or
  command-line flag default.

## 5. Frontend expected semantics

The Gate column means:

- `T01 freeze` — executable only after the T01 schema/types/task protocol
  freeze; a row that names a pending T03/T01 co-freeze in its Depends column
  additionally requires that co-freeze.
- `T01 + target probe` — additionally requires the AArch64 probe to be verified.
  The probe applies **only** to the target-dependent assertion in the row (a
  concrete width, alignment, bit pattern, signedness, or ABI value); the
  symbolic/structural part of the same row remains gated by the T01 freeze
  alone. A probe-gated row is therefore not wholly blocked by the probe: its
  target-independent expectations are exercised as soon as T01 freezes.
- `T01 + dialect policy` — additionally requires an explicit declared mode.
- `T01 + overflow policy` — outside the normative pass criteria (section 9).

Probe and integration gates constrain target-dependent assertions and a runnable
Part B artifact **only**. They do not gate the Part A symbolic fixtures, the
hand-derived semantic expectations, or the unfrozen `/6` schema; Part B
substrate (assembler/linker/sysroot/runner) is unavailable and out of this
document's fixture scope.

Each row is independent: it feeds the chip under test from a hand-built input
record and does not require the full upstream chain. Full-chain behavior is the
single integration row `M1-INT-01`.

### 5.1 Preprocessing (owner T03)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-PP-01` | PP01 | `M1-SRC-000` (28 raw bytes) | normalized bytes equal input; byte-to-location map is identity; one logical line; final LF present. Exact map-boundary assertion under the rev-53 delegated candidate (see note below): `raw_offsets == [0, 1, …, 28]`, `raw_offsets.len() == 29`, every output boundary `i` mapping to raw boundary `i` | T01 source/span records; T03 rev-53 map-boundary candidate (pending T03/T01 co-freeze) | T01 freeze |
| `M1-PP-01b` | PP01 | `M1-SRC-001` (27 raw bytes) | LF appended (27 to 28 output bytes); "newline inserted" provenance; content otherwise identical. Exact map-boundary assertion under the rev-53 delegated candidate (pending T03/T01 co-freeze): `raw_offsets == [0, 1, …, 27, 27]`, `raw_offsets.len() == 29`, the inserted output LF range `[27, 28)` mapping to the **zero-width** raw range `[27, 27)` at raw EOF, and retained boundaries `0..=27` mapping to raw `0..=27` | T01 records; T03 rev-53 map-boundary candidate (pending T03/T01 co-freeze) | T01 freeze |
| `M1-PP-01c` | PP01 | `M1-SRC-002` (29 raw bytes) | CRLF mapped to LF (29 to 28 output bytes); logical columns map to physical bytes. Exact map-boundary assertion under the rev-53 delegated candidate (pending T03/T01 co-freeze): `raw_offsets == [0, 1, …, 26, 27, 29]`, `raw_offsets.len() == 29`, retained boundaries `0..=26` mapping to raw `0..=26`, and the collapsed output LF range `[27, 28)` mapping to the raw range `[27, 29)` (raw `CR` start to after the raw `LF`) | T01 records; T03 rev-53 map-boundary candidate (pending T03/T01 co-freeze) | T01 freeze |
| `M1-PP-02` | PP02 | normalized canonical | identity; zero line-splice records (no backslash-newline) | PP01 output | T01 freeze |
| `M1-PP-03` | PP03 | spliced bytes | identity; comment-free; logical line count 1 | PP02 output | T01 freeze |
| `M1-PP-04` | PP04 | comment-free bytes | pp-tokens in order with spans: `int`[0,3) `main`[4,8) `(`[8,9) `void`[9,13) `)`[13,14) `{`[14,15) `return`[15,21) `2`[22,23) `+`[23,24) `3`[24,25) `;`[25,26) `}`[26,27) EOF[28,28); maximal munch. **EOF[28,28)** is qualified as the **`M1-SRC-000` (canonical, 28-byte, identity-map)** coordinate = the raw source end; for a CRLF or inserted-EOF-LF variant the EOF raw coordinate is the **map raw boundary** (for `M1-SRC-001` raw EOF 27, for `M1-SRC-002` raw EOF 29), not the normalized output length | PP03 output; rev-53 map-boundary candidate for non-canonical variants (pending T03/T01 co-freeze) | T01 freeze |
| `M1-PP-05` | PP05, PP09, PP17, PP19, PP25 | pp-token sequence | no directive, macro, include, conditional frame, or pragma recognized; no child requests | T01 records | T01 freeze |
| `M1-PP-06` | PP28 | final pp tokens | emits exactly the 12 tokens above plus EOF; no accidental token joining; origin chain length 1 | PP row above | T01 freeze |
| `M1-PP-07` | PP04, PP28 | `M1-SRC-004` | the body scans as `return`, `2`, `+`, `+`, `3`, `;`; the two plus signs (separated by a space) stay separate punctuators and are never glued into `++` | T01 records; T03 PP28 acceptance | T01 freeze |
| `M1-PP-08` | PP01 (single-source `Normalized`) / artifact map | normalized canonical buffer | the M1 **exercised artifact-map path is only the single-source `Normalized` artifact** (rev 44); for that artifact the accepted subdecisions require a **valid `source`** and the mandatory map invariants `raw_offsets.len() == bytes.len()+1`, first `== 0`, non-decreasing, last `<= source length` (rev 45); a logical offset remaps to a raw `SourceRecord` offset. A **missing `source` or invalid map is rejected before mutation** is a **planned required contract test** (G1), not a current or frozen behavior: the exact **fault classification/diagnostic** (`ArtifactSourceMissing`/`ArtifactSourceMismatch`/`ArtifactMapInvalid` are **proposed names, not frozen**) and the **atomic artifact behavior** (whether the rejected proposal commits nothing and leaves `artifacts.fragments` unchanged) remain a **T03/T01 co-freeze** item and are **not asserted as pass** here. `Spliced`/`CommentFree`/`Preprocessed` are **schema-declared but not produced (and not asserted) in this M1 slice**; the map-optional empty-`raw_offsets` rule is a **rev-47 delegated candidate pending T03/T01 co-freeze**, and **source-provenance/equality** is deferred (no source-payload-bytes-equality requirement) | T01 records; T03 artifact-map acceptance (rev 44/45/47); T03/T01 co-freeze of the pre-mutation fault classification/atomicity | T01 freeze |

Preprocessing gap gate: chip **PP08 `MacroUndefChip`** is **unexercised** by M1 (the
fixture has no `#undef`) and must be recorded as an explicit gap with a gate, not
as a pass. Note `M1-PP-08` (the artifact-map fixture) is **not** chip `PP08`.
The total `ArtifactKind` map classification is a selected subdecision; the
map-optional empty-`raw_offsets` behavior is a delegated candidate pending T03/T01
co-freeze, while exact validation/error details remain open. **M1 artifact-map exercised scope is only the single-source
`Normalized` artifact (rev 44)** — the `ArtifactRecord { kind, source: Option<SourceId>,
bytes, raw_offsets }` shape and the total eight-`ArtifactKind` map rule are
accepted subdecisions (map-mandatory `Normalized`/`Spliced`/`CommentFree`/`Preprocessed`;
map-optional `Assembly`/`Object`/`Snapshot`/`Trace`), but the other map-mandatory
producers (`Spliced`/`CommentFree`/`Preprocessed`) and the multi-source map are
**declared schema obligations that are not produced or asserted in M1** (deferred),
not M1 passes. **`Preprocessed` (PP28) is declared but not produced by the M1
fixture** (rev 21): chip PP28 itself is exercised by `M1-PP-06`'s token
re-emission; the `Preprocessed` artifact kind is the explicit unexercised gap,
not a pass.

Artifact-map boundary note (rev-53 delegated candidate; not a freeze). The exact
`raw_offsets` values asserted in `M1-PP-01`/`01b`/`01c` above follow the **T03
rev-53 integration-agent-selected candidate default under explicit user
delegation** ([T03_PREPROCESS_CHIPS.md](T03_PREPROCESS_CHIPS.md) rev 53): `raw_offsets[i]`
is the raw-source boundary for output boundary `i`, output `[a,b)` maps to raw
`[raw_offsets[a], raw_offsets[b])`, an inserted terminal `LF` maps both boundaries
to raw EOF (zero width), and a `CRLF`→`LF` collapse maps the output `LF` start to
the raw `CR` start and its end after the raw `LF`. The asserted values are
grounded in the fixture byte lengths stated in section 3 — `M1-SRC-000` 28,
`M1-SRC-001` 27, `M1-SRC-002` 29 — and were checked against those lengths; no
fixture raw bytes exist on disk yet, so these remain hand-derived. This candidate
is **not** a T03/T01 owner signoff and **not** a `/6` freeze: the exact
`raw_offsets` semantics/error mapping and numeric codes stay a **T03/T01 co-freeze**
item, and the assertions above are **planned**, not accepted or executed. No
acceptance pass is claimed.

### 5.2 Tokenization (owner T04)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-LX-01` | LX01, LX03, LX04 | 12 pp-tokens | kinds: keyword `int`, identifier `main`, punct `(`, keyword `void`, punct `)`, punct `{`, keyword `return`, integer `2`, punct `+`, integer `3`, punct `;`, punct `}`, unique EOF | PP04 output | T01 freeze |
| `M1-LX-02` | LX02 | `main` | one interned name; identical spelling yields the same name record; `main` is not coerced to a keyword | T01 name store | T01 freeze |
| `M1-LX-03` | LX03 | `int`, `void`, `return` | keywords in every ISO C89 to C23 and GNU variant listed in section 4; classification is mode-invariant for these three spellings | T01 keyword records (classification mode-invariant) | T01 freeze |
| `M1-LX-04` | LX05, LX06, LX07, LX08 | token `2` | decimal radix; no suffix; value 2; the first representable decimal no-suffix candidate is `int` in every mode listed in section 4 (the full candidate list is dialect-scoped: `int`, `long`, `long long` for C99+ and GNU C89+; `int`, `long`, `unsigned long` for strict ISO C89/C90); value fits `int`; signed `int`; bit pattern `0x00000002` (probe applies only to the concrete bit pattern/width; the radix/suffix/value/first-representable-candidate facts are symbolic and T01-freeze-gated) | frozen target integer model | T01 + target probe |
| `M1-LX-05` | LX05, LX06, LX07, LX08 | token `3` | same as `M1-LX-04`, value 3, bit pattern `0x00000003` (probe applies only to the concrete bit pattern/width) | frozen target integer model | T01 + target probe |
| `M1-LX-06` | LX16, LX17 | 12 tokens + EOF | tokens published in source order; spans preserved; exactly one EOF; stable token IDs | T01 token records | T01 freeze |
| `M1-LX-07` | LX01, LX04, LX17 | `M1-SRC-003` | same token kinds as `M1-LX-01`, spellings preserved, only spans differ; whitespace does not change meaning | PP output of variant | T01 freeze |

### 5.3 Parsing and declarator (owner T05)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-PA-01` | PA01 | token range | exactly one external declaration, then EOF; explicit progress each step; no idle spin | T04 tokens | T01 freeze |
| `M1-PA-02` | PA02 | `int main(void){` | classified as a function definition (specifier + declarator + body), not a declaration | tokens; specifier service | T01 freeze |
| `M1-PA-03` | PA03 | `int` | specifier bundle: type specifier `int`; no storage class, qualifier, function specifier, or alignment | T06 TY13 | T01 freeze |
| `M1-PA-04` | PA05, PA06, PA07, PA09 | `main ( void )` | declarator tree: direct declarator naming `main` with one function suffix; params = prototype with zero parameters; variadic false; no pointer chain. `(void)` is the ISO C 6.7.6.3 special "no parameters" form, not one `void` parameter | PA04 query that `main` is an ordinary identifier and not a typedef | T01 freeze |
| `M1-PA-05` | PA28 | `{ return 2+3; }` | one block; scope enters at `{` and exits at `}`; scope balance returns to zero | T06 TY01/TY02 | T01 freeze |
| `M1-PA-06` | PA32 | `return 2+3 ;` | return statement with an expression present (not `return;`) | PA24 | T01 freeze |
| `M1-PA-07` | PA16, PA22 | `2+3` | binary node, additive `+`, lhs integer constant `2`, rhs integer constant `3`, additive precedence level; one operator so associativity is not observable | T04 tokens | T01 freeze |
| `M1-PA-08` | PA20, PA22 | `2+ +3` | valid: the second `+` is unary plus; no diagnostic; no `++` token exists | T04 tokens | T01 freeze |
| `M1-PA-09` | PA09 | `main()` variant | an empty identifier list is "no prototype information" for a declaration and must not be silently equated with `(void)` for later call checking | declared mode; T06 TY17 | T01 + dialect policy |

### 5.4 Symbol and type (owner T06)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-TY-01` | TY13 | `int` | canonical signed integer scalar; target rank between `short` and `long`; width and alignment only from the frozen target model (probe currently unverified; the probe applies only to the concrete width/alignment, not to the symbolic "canonical signed integer scalar" identity) | frozen target integer model | T01 + target probe |
| `M1-TY-02` | TY17 | `int (void)` | function type: result `int`, parameters empty, prototype true, variadic false; not old-style | `M1-TY-01` | T01 freeze |
| `M1-TY-03` | TY20 | base `int` + declarator | declared type is a function returning `int` with an empty prototype; must not become pointer-to-function or array | `M1-PA-04`, `M1-TY-01/02` | T01 freeze |
| `M1-TY-04` | TY07, TY10 | specifiers + declarator | register `main` in the ordinary namespace at file scope, kind function, linkage external by default, type from `M1-TY-03` | scope service; `M1-TY-03` | T01 freeze |
| `M1-TY-05` | TY01, TY02 | body scope operations | scope tree: file scope and one function-body block scope; `(void)` declares zero parameters and opens **no** prototype scope; every exit balances; records retained for typed AST use. **H3:** the File-scope `Enter` is emitted by a **T06 task after the T05 `TranslationUnit` node is committed**, pinning a committed `NodeId` (not job-bootstrap); the bootstrap order is a T06/`T01 integrator` `/6` decision | scope service | T01 freeze |
| `M1-TY-06` | TY03 | lookup `main` | found in file scope after declaration with the function type; missing before the point of declaration. The point of declaration is defined by the `(source, raw-span start/end, NodeId)` tuple and the deterministic lookup rule (`M1` proposal §5); this row is preserved, not weakened | `M1-TY-04` | T01 freeze |
| `M1-TY-07` | TY25 | types `int`, `int` | integer promotion of `int` yields `int`, unchanged; the promotion fact is **symbolic** and reads no target width, so the row itself asserts no concrete target value and is **Part A** (a concrete-width bit-pattern assertion, were one added, would be probe-gated) | `M1-TY-01` (symbolic `int`) | T01 freeze |
| `M1-TY-08` | TY26 | types `int`, `int` | usual arithmetic conversion yields common type `int`, no inserted casts | `M1-TY-07` | T01 freeze |
| `M1-TY-09` | TY27 | source `int`, destination `int` | return conversion is identity; no diagnostic | `M1-TY-01/03` | T01 freeze |
| `M1-TY-10` | TY09 | first declaration of `main` | no prior declaration, no merge, one definition, no duplicate-definition diagnostic | `M1-TY-04` | T01 freeze |
| `M1-TY-11` | TY13 | a `char`-typed declaration/expression (separate boundary fixture, not the canonical source) | `CharKind` `Plain`/`Signed`/`Unsigned` are distinct symbolic kinds; plain-`char` signedness is **not** asserted (probe-gated); the fixture exercises the closed `CharKind` encoding only | `M1-TY-01`; T06 `CharKind` | T01 freeze (sign-independent); concrete plain-`char` signedness is `T01 + target probe` |

### 5.5 Integer and expression semantics (owner T07)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-SE-01` | SE02 | literal nodes `2`, `3` | each has type `int` and value category non-lvalue (an integer constant is not an lvalue); no string or enum involvement | `M1-LX-04/05`, `M1-TY-01` | T01 freeze |
| `M1-SE-02` | SE07, SE02 | `2+3` | operand types `int`, `int`; result type `int`; no inserted conversions; non-lvalue; signed addition is well-defined because 2 + 3 is representable | `M1-TY-07/08` | T01 freeze |
| `M1-SE-03` | SE21 | `return <expr>` in `int(void)` | function is non-void and the expression is present; conversion `int` to `int` is identity and valid | `M1-TY-09` | T01 freeze |
| `M1-SE-04` | SE29 | function definition | zero parameters, not K&R; body checked; definition consistent with `M1-TY-03` | `M1-TY-03`, `M1-PA-02` | T01 freeze |
| `M1-SE-05` | SE26 (catalog-only) | checked additive tree | the frontend model has no side effects and no unsequenced modifications; the program is well-defined. **`SE26`/`EffectGraph` is deferred and this row does NOT exercise an effect graph**; `M1` has no sequenced-effect requirement beyond this observation (T07 rev18) | `M1-SE-02` | T01 freeze |

### 5.6 Constant evaluation (owner T08)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-CL-01` | CL01 | `2+3` context | legal integer constant expression: integer constants and additive operator only, no assignment, comma, or call | `M1-SE-02` | T01 freeze |
| `M1-CL-02` | **CL03** | add of 2 and 3 | **probe-gated target-width row:** arithmetic in target `int` width; value 5; type `int`; bit pattern `0x00000005`; no overflow. Fixture `M1-CL-02` is executed by chip **`CL03`** (`ConstantBinaryChip`) | `M1-CL-01`, target model | T01 + target probe |
| `M1-CL-03` | CL05 | `int` to `int` on 5 | **Part A symbolic** identity conversion; value 5; type `int`; **no target width**; independent of `M1-CL-02`. **Conversion-unit fixture only:** its `5` is hand-built and is **not** evidence of constant evaluation — the real T07→T08→T09 evaluation handoff is `M1-CL-05` | hand-built symbolic `int 5` (conversion-unit input; not `M1-CL-02` and not the evaluation handoff) | T01 freeze |
| `M1-CL-04` | — | coverage note | `CL02`, `CL04`, `CL06`, `CL07`, `CL20`–`CL26`, and all layout/initializer chips are unexercised by M1 and must be reported as gaps, not passes. Chip `CL02` (`ConstantUnaryChip`) is **not** the probe-gated fixture `M1-CL-02`; that fixture is executed by chip `CL03` | — | not applicable |
| `M1-CL-05` | SE02, SE07 → T08 const evaluator (Part A symbolic) → IR03, IR19 | committed `2+3` expression (T04 literals + T05 nodes + T07 checked facts) | **real T07→T08→T09 constant handoff (OPEN-03 co-freeze, proposal rev 40):** T07 emits `ConstantRequest::Binary` with the committed `BinaryExpression` node, the checked additive operator, and the committed `2`/`3` `LiteralRecord` refs; T08 decodes both committed literals, folds with checked addition (**no hand-built value**), commits **exactly one** `ConstRecord` (`ty` symbolic `int`, `value` `+5`), and returns `ConstantResult { value: RecordRef::Const(c), legality: Legal }`; T09 consumes the same committed `ConstRecord` (`c`) and emits IR `Constant` (`immediate: Some(c)`, **no refold**) + `Return`. Assert: the `ConstId` consumed by T09 equals the one committed by T08; no per-operand `ConstRecord`; the IR `Add` op is not emitted by M1 | `M1-SE-02`; T04 committed literals; T05 nodes; T08/T09 interfaces | T01 freeze (Part A symbolic; no target width) |

Part A / probe split (T08/T09 rev18; rev20 clarification): the symbolic constant
path (`CL01`, `CL03`, `CL05`) is Part A and target-independent; only fixture
`M1-CL-02` asserts a concrete target width/bit-pattern and is probe-gated (chip
`CL03`). The **T09 Part A folded-`int 5` producer is separate** from `M1-CL-02`
and does not gate on the probe; **T08 owns `ConstRecord.value` and T09 owns IR
`Constant` emission**. The **chip IDs `CL02`/`CL03`** and the **fixture IDs
`M1-CL-02`/`M1-CL-03`** are distinct: `M1-CL-02` is run by `CL03`, and chip
`CL02` is unexercised.

**Real constant handoff (OPEN-03 co-freeze, proposal rev 40).** `M1-CL-05` is the
**real T07→T08→T09 fixture**: the folded `int 5` must be produced by T08
evaluating the **committed** `2+3` inputs (the frozen `ConstantRequest::Binary`
carrying the committed `BinaryExpression` node, the checked additive operator, and
the committed operand `LiteralRecord` refs) and consumed by T09 as the same
committed `ConstRecord` (`ConstId`); a hand-built `5` (as in `M1-CL-03`) is a
conversion-unit input only and does **not** satisfy this handoff. `M1-CL-05` is
Part A symbolic (no target width) and gated by `T01 freeze` alone; it does not
assert the probe-gated `M1-CL-02` bit pattern.

## 6. Robustness fixtures

Every malformed case must produce a structured diagnostic and a finite recovery.
Every unsupported case must produce an explicit unsupported or failed result.
Neither may panic, loop, silently succeed, or fabricate a value
([AGENTS.md](../../AGENTS.md) section 4.2; T01 section 4).

### 6.1 Malformed input

| Fixture | Source variant | Chip(s) | Hand-derived expected result | Gate |
|---|---|---|---|---|
| `M1-NEG-01` | `... 3; } /*` unterminated comment | PP03, PP26 | one unterminated-comment diagnostic; recovery consumes to EOF; finite | T01 freeze |
| `M1-NEG-02` | `#foo` line before the source | PP05, PP26 | unknown or unsupported directive diagnostic; macro text must not become a new directive | T01 freeze |
| `M1-NEG-03` | `#if 1` with no `#endif` | PP19 | structural conditional diagnostic at EOF; conditional frame drained | T01 freeze |
| `M1-NEG-04` | `int main(void){return 2@3;}` | LX01, LX18 | one unknown-character diagnostic at the `@` span; error token preserves spelling; advance exactly one byte; no loop | T01 freeze |
| `M1-NEG-05` | `int main(void){return 1e+foo;}` | LX09 | maximal munch makes `1e+foo` one pp-number; one malformed-float-syntax diagnostic; it is not split into `1`, `e`, `+`, `foo` | T01 freeze |
| `M1-NEG-06` | unterminated `"abc` or `'a` | LX12, LX13 | unterminated-literal diagnostic with span; finite recovery | T01 freeze |
| `M1-NEG-07` | `int main(void){return 18446744073709551616;}` | LX07, LX08 | value exceeds every type in the decimal no-suffix candidate list; range or constraint diagnostic; no wrap or truncation (probe applies only to the concrete candidate-list widths; the "exceeds every candidate" fact and the diagnostic are symbolic) | T01 + target probe |
| `M1-NEG-08` | `int main(void){return 2+3}` (missing `;`) | PA32, PA34, PA38 | one expected-`;` diagnostic; recovery synchronizes at `}`; block closes; no phantom second declaration | T01 freeze |
| `M1-NEG-09` | `int main(void){return 2+3;` (missing `}`) | PA28, PA38 | unterminated-compound diagnostic at EOF; scope cleanup exactly once | T01 freeze |
| `M1-NEG-10` | `int main(void{return 2+3;}` | PA09, PA38 | malformed parameter list diagnostic; `void {` is not accepted as a parameter; recovery to `{` or `;` | T01 freeze |
| `M1-NEG-11` | `int main(void){return 2 3;}` | PA22, PA34, PA38 | expected-operator-or-`;` diagnostic; recovery at `;`; one error only | T01 freeze |
| `M1-NEG-12` | `int f(void){return 2+3;} = 0;` or `int f(void) = 0;` | PA02, TY31 | function definition or declaration cannot carry an object initializer; constraint diagnostic | T01 freeze |
| `M1-NEG-13` | `int f(void x);` | TY17, TY31 | a named parameter of type `void` is a constraint violation; only bare `void` as the sole item is the no-parameter form | T01 freeze |
| `M1-NEG-14` | `int main(void){return x+3;}` | SE01 (catalog-only; `M1-NEG-14` owner) | `x` is an undeclared identifier used as an object operand, not a call, so the implicit-function-declaration rule does **not** apply in any declared mode: one undeclared-identifier diagnostic is required in C89/C99/C11/C17/C23 and their GNU variants (mode-independent). This is **not** a dialect-policy fixture; the implicit-declaration dialect case is `M1-NEG-19`. SE01 is retained **only** as this negative fixture's owner and is not on the M1 positive path (`M1-SE-01` is owned by SE02) | T01 freeze |
| `M1-NEG-15` | `int main(void){return;}` | SE21 | mode/policy-sensitive: returning without a value from an `int` function. Must not be silently accepted or rejected without a declared policy | T01 + dialect policy |
| `M1-NEG-16` | `int main(void){return 2147483647+1;}` | **CL03** (`ConstantBinaryChip`), SE07 | signed-overflow behavior is deliberately outside this fixture's normative pass criteria; see section 9. If treated as an integer constant expression it is out of range; otherwise it is undefined absent a wrap policy. Chip `CL02` (`ConstantUnaryChip`) is **not** used here (rev 21 corrects the malformed chip list) | T01 + overflow policy |
| `M1-NEG-17` | `int main(void){return 5/0;}` | CL03 | division-by-zero diagnostic; not a valid constant expression; no crash | T01 freeze |
| `M1-NEG-18` | `int main(void){return 1<<40;}` | CL03 | shift count at or above the operand width yields a diagnostic; no host shift is used (probe applies only to the concrete operand width; the diagnostic rule is symbolic and T01-freeze-gated) | T01 + target probe |
| `M1-NEG-19` | `int main(void){return f(3);}` | PA18, SE01 (catalog-only; `M1-NEG-19` owner) | mode-sensitive implicit-declaration case, moved here from the `x+3` object form: `f` is undeclared and is the callee of a call expression. Under C89/GNU89 the call implicitly declares `int f()` (C89 3.3.2.2; typically with a warning), while strict C99 and later removed that rule, so the call is a constraint violation requiring a diagnostic. Expected result chosen only after the mode is declared; never silently accepted as conforming | T01 + dialect policy |

### 6.2 Unsupported features

Each row must yield an explicit unsupported or failed result, never a
successful compile and never a fabricated artifact. The concrete result
encoding is provisional (section 9).

| Fixture | Variant | Expected frontend outcome |
|---|---|---|
| `M1-UNS-01` | any `float`, `double`, or `long double` use, e.g. `return 2+3.0;` | explicit unsupported result; the conversion is well-defined C but outside the M1 subset |
| `M1-UNS-02` | `struct`, `union`, or `enum` use | explicit unsupported result |
| `M1-UNS-03` | pointer or array declarator, e.g. `int main(int argc, char **argv)` | explicit unsupported result; must not be silently mis-parsed as M1 syntax |
| `M1-UNS-04` | variadic `int f(int, ...)` | explicit unsupported result |
| `M1-UNS-05` | old-style definition with an empty parameter list in a definition | explicit unsupported or an explicitly modeled no-prototype function; must not be conflated with `(void)` |
| `M1-UNS-06` | `#include` or a function-like macro | explicit unsupported result unless a no-op capability is separately declared; no silent ignore |
| `M1-UNS-07` | GNU attribute, `typeof`, statement expression, or inline `asm` | explicit unsupported result |
| `M1-UNS-08` | `void main(void)` | mode/policy decision; ISO C requires `int main`, so this must not be silently accepted as conforming |
| `M1-UNS-09` | a task kind that is not implemented | the foundation `control.unsupported` path must fail explicitly, never return success |

### 6.3 Replay and determinism

| Fixture | Scenario | Hand-derived expected result |
|---|---|---|
| `M1-REC-01` | same input, config, and initial state replayed | identical record IDs, diagnostic order, and per-tick trace; scheduling and ID allocation use only documented ordering, never hash-map iteration or addresses |
| `M1-REC-02` | a child task fails during parse or type construction | the parent fails exactly once; exactly one diagnostic per primary error; no duplicate completion |
| `M1-REC-03` | new task enqueued in tick T | ready in T+1 by default; no same-tick re-entry; selection order is `(stage ordinal, phase priority, enqueue ordinal, TaskId)`; at `max_inflight_per_tick = 1` the result equals the single-active-task design under the **canonical semantic comparison projection** (semantic records/results/diagnostics + semantic trace, excluding scheduler-only registers: stage queues, in-flight set, `dispatch_cursor`, `stage_assignment_version`, `PipelineMetrics`) — **not** a byte-identical snapshot — while the new scheduler snapshot is separately replay-identical |
| `M1-REC-04` | malformed or illegal bytes streamed through the lexer | bounded ticks, no infinite loop, no panic; progress is finite |
| `M1-REC-05` | recovery after a bad token followed by a valid function | recovery synchronizes to a declared boundary and does not swallow the following function |

### 6.4 Write-scope and access contract

| Fixture | Scenario | Hand-derived expected result |
|---|---|---|
| `M1-WS-01` | a worker proposes a patch to an undeclared store field | the proposal is rejected; no store mutation occurs |
| `M1-WS-02` | a worker proposes a patch with a wrong owner or stale version | the proposal is rejected; a failed batch commits nothing |
| `M1-WS-03` | a worker receives a task kind it does not own | semantic state is unchanged except for the scheduling diagnostic |
| `M1-WS-04` | manifest audit across the M1 chip set | every chip declares exact reads/writes, phase, category, and backend class; a chip must not be granted read or write of a whole store (`VF04`, `T01` section 5) |
| `M1-WS-05` | adapter projection test | changing a declared output field succeeds; undeclared registers and undeclared fields stay byte-for-byte unchanged |

## 7. Owners, dependencies, and readiness gates

Owner packages and the producer facts they consume (data dependencies, never
chip-to-chip calls):

| Phase | Owner package | Primary chips | Depends on |
|---|---|---|---|
| Preprocessing | T03 | PP01–PP05, PP09, PP17, PP19, PP25–PP26, PP28; `PP02`/`PP03` are exercised **identity-only** on the M1 canonical path (`M1-PP-02`/`M1-PP-03`; splice/comment-removal semantics deferred, T03 rev 54/55), and `PP26` only by the malformed fixtures `M1-NEG-01`/`M1-NEG-02` | T01 source/pp records; CT02/CT08 host import; CT06 commit path |
| Tokenization | T04 | LX01–LX09, LX12, LX13, LX16–LX18 (the M1-exercised members of `LX09`–`LX18`; `LX09`/`LX12`/`LX13`/`LX18` are diagnostic-only via `M1-NEG-04/05/06`, while `LX10`/`LX11`/`LX14`/`LX15` remain explicitly deferred non-M1 forms) | T03 output; target integer model for the probe-gated `M1-LX-04/05` bit-pattern part only (the M1-tokenized literal record is symbolic) |
| Parsing / declarator | T05 | PA01–PA09, PA16, PA18, PA20, PA22, PA24, PA28, PA32, PA34, PA38 (`PA18` only via the mode-sensitive `M1-NEG-19` call-expression negative) | T04 tokens; T06 TY01/TY02/TY17; PA04 query |
| Symbol / type | T06 | M1 chips TY01–TY03, TY07, TY09, TY10, TY13, TY17, TY20, TY25–TY27; `TY08` (`TypedefRegisterChip`) is **not** exercised by M1 (no typedef in the fixture), and the T01 `TY01→TY07→TY08→TY03` ordering fixture that includes `TY08` is therefore a **non-M1 supporting fixture**, not an M1 gate (resolved consistently); `TY14` (`QualifiedTypeChip`) is **not** exercised by M1 (the canonical source has no qualifiers; `M1-PA-03`'s "no qualifier" fact is a parse-side specifier-bundle fact, not a TY14 invocation); `TY31` (`DeclarationConstraintChip`) is exercised by M1 **only** through the constraint-negative fixtures `M1-NEG-12`/`M1-NEG-13` (G7), not by the positive `M1-TY-*` rows, and its wider positive-path scope stays deferred | T04 names; T01 type/symbol stores |
| Integer semantics | T07 | SE02, SE07, SE21, SE29; `SE26`/`EffectGraph` is deferred and **not exercised**; `SE01` is catalog-only and owned by the negative fixtures `M1-NEG-14`/`M1-NEG-19` | T05 AST; T06 types (rev 22: **not** T08 constants — the sem stage precedes the const stage; T07 emits the per-use `const.evaluate` request that T08 consumes) |
| Constant evaluation | T08 | CL01, CL03, CL05 | T06 types; T07 checked facts / the committed T07 `SemRecord`; T04 committed `LiteralRecord`; target model for the probe-gated `CL03`/`M1-CL-02` bit-pattern path only (`CL01`/`CL05` and the folded-`int5` value are Part A symbolic) |
| Control, diagnostics, commit | T02 | CT02–CT08, CT11, CT12 (`CT02`/`CT08` cover host source import/request only) | T01 task/result protocol |
| Replay, access, evidence | T13 | VF01–VF06, VF12, VF13, VF14 (`VF07`–`VF11` are **out of M1**; see the VF07–VF09 gap note below) | all M1 stages |
| Integration downstream | T09 (Part A) | IR01–IR03, IR19, IR28 | T01 freeze (symbolic IR; no probe) |
| Integration downstream | T11 (Part B, probe-gated) | CG04, CG09, CG10, CG33, CG35, CG37, CG38 (CG29 if virtual registers are used; catalog refs, non-exhaustive) | T01 freeze + probe-verified ABI + frozen IR |

**Chip-list scope note (rev 32; no acceptance change, no freeze).** The "Primary
chips" column names the M1-exercised chips only. `PP26` and
`LX09`/`LX12`/`LX13`/`LX18` are exercised only through the malformed fixtures
(`M1-NEG-01`/`02`, `M1-NEG-04/05/06`); `PP02`/`PP03` run **identity-only** on the
M1 canonical path (`M1-PP-02`/`M1-PP-03`), with splice/comment-removal semantics
deferred beyond M1 (T03 rev 54/55); `PA18` is exercised only by the mode-sensitive
`M1-NEG-19`; `TY31` only by the constraint negatives `M1-NEG-12`/`M1-NEG-13`;
`CT02`/`CT08` cover host source import/request only. `LX10`/`LX11`/`LX14`/`LX15`
remain explicitly deferred non-M1 forms, not M1 primary chips. `VF12` and the T09
set `IR01–IR03, IR19, IR28` were already listed and are unchanged.

**Rev 22 dependency-order correction (T07 finding 6).** The rev-21 table made
T07 depend on "T08 constant context" while T08 depends on "T07 checked facts",
which is a **cycle** and backwards relative to the selected sem→const stage order.
Corrected: the **sem** stage (T07) precedes the **const** stage (T08); T07 emits
the per-use `const.evaluate` request, T08 consumes the committed T07 `SemRecord`
and the committed T04 `LiteralRecord`. T07 does not depend on T08.

Readiness tiers:

| Tier | Fixtures | Blocked by |
|---|---|---|
| Text-level expectations that can be written now but not executed | source bytes, token spellings, spans, diagnostic category, exit value | T01 freeze to bind records; no harness exists |
| Record-bound | `M1-LX-*`, `M1-TY-*`, `M1-SE-*`, `M1-CL-*`, `M1-WS-*`, `M1-REC-*` | T01 freeze |
| Target-dependent | `M1-LX-04/05`, `M1-NEG-07/18`, `M1-TY-01`, `M1-CL-02` | T01 freeze plus probe `verified=true` for the target-dependent assertion only (`M1-TY-07` is Part A symbolic and is **not** in this tier) |
| Mode-sensitive | `M1-PA-09`, `M1-NEG-15/19`, `M1-UNS-05/08` | T01 freeze plus declared/dialect policy |
| Policy-sensitive (non-normative) | `M1-NEG-16` | signed-overflow policy (section 9) |
| Integration | `M1-INT-01` | T01 freeze, target probe, T09/T11 interfaces |

For every "Target-dependent" row, the probe blocks only the concrete
target-value assertion in that row; the symbolic/structural expectations of the
same fixture are gated by the T01 freeze alone. The probe does **not** block the
Part A symbolic fixtures (`M1-CL-03`, `M1-TY-11`, `M1-TY-07`, and the symbolic parts of
`M1-LX-04/05`, `M1-TY-01`) and does **not** block the `/6` schema freeze.
Integration and Part B gates (G12, G14) cover target-dependent assertions and a
runnable end-to-end artifact; Part B substrate is unavailable and out of this
document's fixture scope.

## 8. M1 frontend acceptance gates

M1 frontend fixtures are accepted only when all applicable gates below hold.
Passing a gate does not imply the full compiler works and is not a torture rate.

| Gate | Requirement | Owner |
|---|---|---|
| G1 | `M1-PP-01`–`M1-PP-08` produce the declared token stream, provenance, and **artifact map** (including the boundary and whitespace variants); `M1-PP-08` exercises the **single-source `Normalized`** artifact-map path only, asserting a **valid `source`** and the full mandatory `raw_offsets` invariants for that artifact (rev 44/45), with the exact `M1-PP-01`/`01b`/`01c` boundary values under the **rev-53 delegated candidate** (pending T03/T01 co-freeze, not asserted as frozen); the **missing-`source`/invalid-map rejection-before-mutation** is a **planned required contract test** whose exact **fault classification/diagnostic** (proposed names, not frozen) and **atomic artifact behavior** remain a T03/T01 co-freeze item and is **not asserted as a pass**; the source-provenance/equality relation is **deferred** (rev 47 delegated default: no source-payload-bytes-equality requirement) and is **not** asserted as accepted/frozen; `Spliced`/`CommentFree`/`Preprocessed` and PP28 artifact production are **not** required in this slice (schema-declared, unexercised); chip `PP08` `MacroUndefChip` carries an explicit unexercised **gap gate** (never a pass) | T03 |
| G2 | `M1-LX-01`–`M1-LX-07` produce the declared token kinds, spellings, spans, and literal records in the declared mode; the committed T03 PP span is **reused with no T04 span write** (`token_reuses_committed_pp_span`); the committed T04 `LiteralRecord` is the representable T04→T08 handoff (`literal_record_committed`/`literal_t04_t08_handoff`) | T04 |
| G3 | `M1-PA-01`–`M1-PA-08` produce the declared declarator tree, AST shape, and scopes; `(void)` is zero parameters; PA20 is present in the unary path | T05 |
| G4 | `M1-TY-01`–`M1-TY-11` produce the declared types, symbol, linkage, and scope tree; the concrete numeral assertions of `M1-TY-01` (width/alignment) and the concrete plain-`char` signedness part of `M1-TY-11` are probe-gated, while the remaining expectations (`M1-TY-07` promotion, the `M1-TY-11` `CharKind` encoding, etc.) are symbolic `T01 freeze`; `M1-TY-06` point-of-declaration is preserved; `TY31` is scoped to the `M1-NEG-12`/`M1-NEG-13` constraint negatives (G7), not the positive `M1-TY-*` rows | T06 |
| G5 | `M1-SE-01`–`M1-SE-05` produce the declared typed facts and value category for `2+3` in an `int` function; `M1-SE-01` is owned by SE02 and `SE26`/`EffectGraph` is not exercised | T07 |
| G6 | `M1-CL-01`–`M1-CL-03` and `M1-CL-05` produce the constant value 5 of type `int` for the integer constant expression; only `M1-CL-02` is probe-gated and the Part A folded-`int5` producer is separate. `M1-CL-05` requires the folded `5` to come from the **real T07→T08 evaluation** of the committed `2+3` inputs and to be consumed by T09 as the same committed `ConstRecord`; a hand-built `5` (`M1-CL-03`) does not satisfy the handoff | T08 |
| G7 | each malformed fixture whose expected result is normative (`M1-NEG-01`–`M1-NEG-14`, `M1-NEG-17`–`18`) produces exactly one primary diagnostic with the expected location and a finite recovery; the mode/policy-sensitive `M1-NEG-15`/`M1-NEG-19` and the policy-sensitive non-normative `M1-NEG-16` are excluded until their policy is declared (section 9), and `M1-NEG-18`'s concrete-width diagnostic assertion is probe-gated (its symbolic rule is `T01 freeze`) | T02, T03, T04, T05, T06, T07, T08 |
| G8 | each unsupported fixture produces an explicit unsupported or failed result — or, for `M1-UNS-05`/`06`, the explicitly declared modeled outcome — never a silent success and never a fabricated artifact; the mode/policy-sensitive `M1-UNS-05`/`M1-UNS-08` and the conditional `M1-UNS-06` no-op capability are scoped to a declared mode/policy before their expected outcome is fixed, and none may be silently accepted | T02, T13 |
| G9 | replay fixtures are tick-for-tick identical for identical input, config, and initial state (the new scheduler snapshot is separately replay-identical); the quota-1 **cross-design** comparison in `M1-REC-03` uses the canonical semantic comparison projection (G15) and is **not** a byte-identical snapshot; the batch/quota>1 replay extension (`VF13`) is required-pending and is not part of quota-1 M1 acceptance | T13 |
| G10 | write-scope and access fixtures hold; a failed batch commits nothing | T01, T02, T13 |
| G11 | no expected value in this document was produced by compiling the candidate with GCC or Clang, and no reference output substitutes for the fixed target | all |
| G12 | the target-dependent assertion of any fixture that asserts a concrete width, alignment, bit pattern, signedness, or ABI value is blocked until the AArch64 probe reports `verified=true`; the symbolic/structural expectations of the same fixture are blocked only by the T01 freeze | T01, T11 |
| G13 | the mode for every fixture is declared; no fixture depends on an inferred default dialect | all |
| G14 | integration `M1-INT-01` is separately recorded: candidate-generated object, external assembler/linker, process exit status 5 | T09, T11 |
| G15 | pipeline baseline: at `max_inflight_per_tick = 1` the result equals the single-active-task design under the **canonical semantic comparison projection** (semantic records/results/diagnostics + semantic trace, scheduler-only registers excluded), with the new scheduler snapshot separately deterministic/replay-identical; the pipeline scheduling amendments are frozen before any chip wave depends on them, and a quota `> 1` requires measured before/after tick counts | T02, T13 |

G14 is listed only so the frontend gates are not confused with the M1
end-to-end milestone. It is a **Part B** gate: the Linux probe/toolchain,
assembler, linker, sysroot, and runner are **not provisioned**, so G14 is
not runnable today and constrains only the runnable end-to-end artifact, not the
symbolic Part A fixtures or the `/6` freeze. Its detailed acceptance belongs to
the T09/T11 task packages, not to this document. G15 does **not** claim a
throughput improvement; it is the gate that keeps the pipeline inert (except
quota 1) until the freeze and the measurement exist.

**H6/H9 batch scope (current status).** The H6 mechanism direction is **selected**
(CDR rev 42) but its implementation and `/6` realization remain **pending**; the
H9 `max_inflight_total` removal direction and the sole-`max_inflight_per_tick`
delegated candidate default (CDR rev 35/51) leave the dispatcher
`Ready→Running`/`in_flight` boundary, the clear order, and the all-paths
no-`Running`/no-residual proof **open**. The T13 H6/H9 batch fixtures
(VF02/VF03/VF04/VF13, H6-M01..H6-M16) are **required-pending** and are **not** part
of quota-1 M1 acceptance. The rev-25 row's "batch no-`Running` fan-out stays
BLOCKED (H6)" describes the **pre-rev-42** state.

**VF06 coverage (recorded gap).** VF06 `TypedAstInvariant` is **in M1** (T07/T13
rev 46: after the committed T07 `SemRecord`s, before T09 lowering, checking M1
typed-fact/required-conversion completeness), but its **exact registration and
fixtures remain T01/T13 co-freeze/pending** — a **recorded gap, not an M1 pass**
(the same pending status applies to the VF02/VF03/VF04/VF13 batch amendments).

**VF07–VF09 scope (recorded gap).** VF07 `CfgInvariantChip`, VF08
`IrInvariantChip`, and VF09 `SsaInvariantChip` are **out of the M1 T13 chip
set** (T13 rev 8: VF07–VF11 out of M1 — no M1 CFG/SSA/machine/ABI verification
stage is registered). `M1_TARGET_ACCEPTANCE.md` §3.2 names VF08 (and the VF09
non-SSA guard) only as **catalog checkers** for the M1 IR invariants; no M1
registration or fixture exists for them, so the M1 IR/CFG invariant checks are a
**recorded gap, not a pass**.

## 9. Explicitly non-normative items

The following are deliberately outside this fixture's normative pass criteria.
They must still be recorded and must not be counted as passes for the frontend.

- **Signed-overflow behavior.** `M1-NEG-16` (`2147483647+1`) has no normative
  expected value here. Whether M1 defines a wrap policy (`-fwrapv`) or treats
  the case as undefined or a constraint failure is a contract decision. Until
  that decision is recorded, the fixture asserts only that the frontend neither
  crashes nor fabricates a value.
- **Unsupported result encoding.** Section 6.2 requires an explicit
  unsupported or failed result, but the concrete encoding (a distinct
  `Unsupported` diagnostic class versus `Failed` with a specific code) is a T01
  protocol decision and is not fixed here. Fixtures must not assert a specific
  encoding until T01 freezes it.
- **Diagnostic wording and codes.** Fixtures assert diagnostic category and
  location, not message text or numeric codes.
- **Concrete target widths, alignment, and ABI values.** These remain
  unverified and are not asserted by any fixture before the probe.
- **Optimization behavior.** Whether the backend or optimizer folds `2+3` is
  not part of the frontend fixture; `M1-CL-02` asserts the semantic constant
  value only.

## 10. Non-claims

- No test in this document has been run; the compiler application does not yet
  implement these chips.
- Nothing here is an interface, type, record, or manifest freeze. All schema
  names and IDs are provisional until the T01 integrator publishes a freeze.
- No default dialect is asserted for the compiler.
- Target identity is frozen, but every concrete ABI value remains unverified.
- No GCC or Clang was used to compile the candidate source, and no reference
  compiler output was used as an oracle.
- No pass rate, coverage percentage, or M1-complete claim is made. Passing the
  M1 frontend gates is necessary but not sufficient for the M1 vertical slice,
  and neither is the T00 GCC torture rate.
- Unexercised constant, layout, and initialization chips are recorded as gaps
  (`M1-CL-04`), not as passes.

## 11. Revision record

| Date | Change | Authority |
|---|---|---|
| 2026-10-05 | Rev 34: **chip-ID audit follow-up** (docs-only; no acceptance change, no implementation, no freeze). Recorded `TY14` (`QualifiedTypeChip`) as **not exercised by M1** (no qualifier in the fixture; the `M1-PA-03` "no qualifier" fact is parse-side, not a TY14 invocation) in the §7 T06 row, and recorded `VF07`–`VF09` as **out of the M1 T13 chip set** (T13 rev 8) with the `M1_TARGET_ACCEPTANCE.md` §3.2 IR-invariant references scoped to **catalog checkers only** — a recorded gap, not a pass. | Chip-ID audit (docs-only); T13 rev 8 |
| 2026-10-05 | Rev 33: **lines 1–250 fixture-scope corrections** (docs-only; no acceptance change, no implementation, no freeze). `M1-LX-04/05`: the asserted literal fact is now the mode-invariant **first representable decimal no-suffix candidate `int`** (all section-4 modes), with the full candidate list stated as dialect-scoped (`int`/`long`/`long long` for C99+/GNU C89+; `int`/`long`/`unsigned long` for strict ISO C89/C90). `M1-LX-03` dependency clarified (mode-invariant classification; gate stays `T01 freeze`). `M1-LX-06` input spelled `12 tokens + EOF`. `M1-PP-07` wording corrected (the two plus signs are separated by a space). `M1-PP-08` map invariant spelled `bytes.len()+1`; `M1-PP-01`/`01b`/`01c` map length spelled `raw_offsets.len() == 29`; the `T01 freeze` legend now notes any pending T03/T01 co-freeze named in Depends. `Preprocessed`/PP28 clarified (chip PP28 is exercised by `M1-PP-06`; the `Preprocessed` artifact kind is the gap). Section-3 lengths (28/27/29/33/30), the byte layout, the PP-04 spans, and the map arrays were re-verified and are unchanged. | Lines 1–250 fixture audit (byte lengths/PP-04 spans/raw_offsets arity/LX candidate dialect scope/M1-SRC-004 30B); T01 co-freeze pending |
| 2026-10-05 | Rev 32: **§7 chip-list and gate-scope alignment** (docs-only; no acceptance change, no implementation, no freeze). Preprocessing: added `PP26` (malformed-only) and marked `PP02`/`PP03` **identity-only** on the M1 canonical path. Tokenization: added the M1-exercised `LX09`/`LX12`/`LX13`/`LX18` (diagnostic-only subset of `LX09`–`LX18`; `LX10`/`LX11`/`LX14`/`LX15` stay deferred). Parsing: added `PA18` (mode-sensitive `M1-NEG-19` only). Symbol/type: scoped `TY31` to the `M1-NEG-12`/`M1-NEG-13` constraint negatives. Control: widened to `CT02`–`CT08`. Gates: G4 (probe-gated `M1-TY-11` plain-`char` signedness; `TY31` negative-only), G7 (policy rows `M1-NEG-15`/`16`/`19` excluded), G8 (`M1-UNS-05`/`06`/`08` mode/policy scope), G9 (replay vs cross-design projection; batch replay extension pending). `VF12` and `IR01–IR03, IR19, IR28` were already present and are unchanged. | §7/G4/G7/G8/G9 chip-list audit; T01 co-freeze pending |
| 2026-10-05 | Rev 31: **OPEN-03 fix** (docs-only; no acceptance-criterion change, no implementation, no freeze). Added fixture `M1-CL-05`, the **real T07→T08→T09 constant handoff** for the committed `2+3`: T07 emits the frozen `ConstantRequest::Binary` (committed `BinaryExpression` node, checked additive operator, committed `2`/`3` `LiteralRecord` refs); T08 folds the committed operands (no hand-built value) and commits exactly one `ConstRecord`; T09 consumes the same committed `ConstRecord` (`ConstId`) without refolding. Clarified `M1-CL-03` as a **conversion-unit fixture only** (its hand-built `5` is not evaluation evidence), added the §5.6 handoff note, and updated G6 to require the real evaluation path. | OPEN-03 fix (documentation review 2026-10-05); T01/T07/T08 co-freeze |
| 2026-10-05 | Rev 30: **DOC-03 fix** (docs-only; no acceptance change, no implementation, no freeze). `M1-NEG-14` (`return x+3;`) now requires a mode-independent undeclared-identifier diagnostic and is gated `T01 freeze`: an undeclared object operand is not an implicit-function-declaration case, so the GNU89 rule does not apply. The implicit-declaration dialect test moves to new `M1-NEG-19` (`return f(3);`, a call expression with an undeclared callee), gated `T01 + dialect policy`; the §7 mode-sensitive tier and the SE01 owner note are updated accordingly. | Documentation review 2026-10-05 (DOC-03) |
| 2026-10-05 | Rev 29: **Part A target-neutrality and scope clarity** (no acceptance-criterion change, no implementation, no freeze). (1) `M1-TY-07` reclassified from the target-dependent tier to **Part A** (`T01 freeze`): the `int` promotion assertion is symbolic and reads no target width; the probe-gated list is now `M1-LX-04/05`, `M1-NEG-07/18`, `M1-TY-01`, `M1-CL-02`. (2) Added an explicit **H6/H9 batch-scope** note (mechanism selected but pending; H9 boundary/clear/no-residual open; T13 batch fixtures required-pending, not part of quota-1 M1 acceptance) and a **VF06 coverage** note (VF06 in M1 per T07/T13 rev 46, registration/fixtures pending — a recorded gap, not a pass). (3) The rev-26 row's `M1-TY-01/07` probe qualifier is marked superseded for `M1-TY-07`. Planned; no implementation, no execution, no freeze; `/5` current, `/6` unfrozen, M1 DRAFT. | Post-rev-39 read-only audit; docs integration |
| 2026-10-04 | Initial planned M1 frontend fixture/acceptance design; no implementation, no execution, no freeze. | M1 preparation session |
| 2026-10-04 | `M1-TY-05` corrected: `(void)` declares zero parameters and opens no prototype scope (T06 review integration); still planned, no implementation. | M1 integration session |
| 2026-10-04 | Rev 19: added `M1-TY-11` (`char` kinds, sign-independent Part A), `M1-PP-08` (artifact source/map), preserved `M1-TY-06` via the point-of-declaration rule, clarified `M1-SE-05`/`SE26` not exercised, `M1-NEG-14` owned by SE01 (catalog-only), Part A/probe CL02 split, G3/G4/G5/G6 wording, and G15 pipeline baseline gate. Planned; no implementation, no execution. | M1 rev 19 integration |
| 2026-10-04 | Rev 20: `M1-CL-02` is stated as the probe-gated fixture executed by chip `CL03` (chip `CL02` unexercised); `M1-CL-03` is explicitly Part A symbolic with no target width; T08 owns `ConstRecord.value` and T09 owns IR `Constant`; `M1-REC-03`/G15 restated as the **canonical semantic comparison projection** (not byte-identical) with a separately deterministic scheduler snapshot; TY08 ordering fixture marked **non-M1**; PP08 given an explicit gap gate; artifact map classification is map-mandatory only for M1-produced kinds and remains a T03 `/6` item. Planned; no implementation, no execution. | M1 rev 20 re-review integration |
| 2026-10-04 | Rev 21: corrected `M1-NEG-16` to be executed by chip **`CL03`** (`ConstantBinaryChip`) and `SE07` (not chip `CL02`); stated that `Preprocessed` is **not produced by the M1 fixture** (M1-produced kinds = `Normalized`/`Spliced`/`CommentFree`), an explicit gap; aligned with the user's in-principle single-owner/`LiteralRecord` handoff/pipeline decisions recorded in the CDR. Planned; no implementation, no execution, no freeze. | M1 rev-21 review-integration subagent |
| 2026-10-04 | Rev 22: G1 extended to `M1-PP-08` + PP28 artifact production; G2 extended with committed-PP-span reuse/no-T04-span-write and the literal T04→T08 handoff; §7 corrected the **T07↔T08 dependency cycle** to the sem→const order. Planned; no implementation, no execution, no freeze. | M1 rev-22 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | Rev 23: integrated the independent rev-22 audit. **H3** — `M1-TY-05` records the File-scope `Enter` as a **T06 task after the T05 `TranslationUnit` is committed**, pinning a committed `NodeId` (bootstrap order a T06/integrator `/6` decision, not job-bootstrap). The literal-handoff semantic-information split was a **proposed** allocation (audit H1); the user later explicitly accepted the recommended split in principle on 2026-10-04 (rev 24) as a `/6` working basis, **not a freeze** — the per-literal `LiteralRecord` carries the lexical facts + `LX08` type, the sem-stage `ConstantRequest` carries `node`/`required_kind`, and the `ConstantResult` carries `legality`, with T01 + T03/T04/T08 co-freeze/sign-off still pending. Planned; no implementation, no execution, no freeze; `/5` current. | M1 rev-23 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | Rev 24: recorded the user's in-principle acceptance of the recommended H1 literal-handoff allocation split (committed T04 `LiteralRecord` = per-literal lexical facts + `LX08`; sem-stage `ConstantRequest` = `node`/`required_kind`; `ConstantResult` = `legality`; T08 sole `constants.records` writer). Working basis only — not a freeze, not code/chip authorization; exact schema and T01 + T03/T04/T08 owner co-freeze/sign-off pending. Planned; no implementation, no execution; `/5` current. | M1 rev-24 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | Rev 25: integrated the rev-24 independent audit F1–F9 cross-doc. No M1 frontend fixture row changes (the audit fixes are schema/proposal/CDR/ADR/T02-level); the H1 allocation remains accepted in principle with the exact `ConstantRequest { literal, node, required_kind }`/`ConstantResult { value, legality }`/`LiteralRecord.candidate_type` shapes **drafted and unfrozen** (rev 25 F3), and `G10`'s "a failed batch commits nothing" is unchanged while the batch no-`Running` fan-out stays **BLOCKED** (H6). Planned; no implementation, no execution, no freeze; `/5` current; ADR-0002 PROPOSED. | M1 rev-25 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | Rev 26: **clarity-only**, no acceptance-criteria change, no implementation claim, no freeze. Clarified the opening target-model caveat and the §5 Gate legend so the Linux probe and the integration/Part B gates are stated to constrain **only** target-dependent assertions and a runnable end-to-end artifact — not the symbolic Part A fixtures, the hand-derived semantic expectations, or the `/6` schema freeze. Added the inline "probe applies only to the target-dependent part" qualifier to the `T01 + target probe` rows (`M1-LX-04/05`, `M1-TY-01/07`, `M1-NEG-07/18`) (**superseded for `M1-TY-07`, which was later reclassified to Part A** — see the rev-28/rev-29 rows), the §7 readiness-tier note, and G12; restated G14 as a **Part B** gate whose substrate is not provisioned. Planned; no implementation, no execution, no freeze; `/5` current, `/6` unfrozen, M1 DRAFT. | M1 rev-26 docs-clarity subagent (DeepSeek doc integrator) |
| 2026-10-05 | Rev 27: **artifact-map scope alignment** to the CDR rev 44/45/47 accepted subdecisions (no Part A/Part B gate change, no unrelated preprocessor/token criterion change, no implementation claim, no freeze). §5.1 `M1-PP-08` now identifies the **single-source `Normalized`** artifact as the only M1 **exercised** artifact-map path and no longer asserts `Spliced`/`CommentFree`/`Preprocessed` producers or `source`-equality: it requires a **valid `source`** and the rev-45 mandatory-map invariants for the exercised artifact, and records `Spliced`/`CommentFree`/`Preprocessed` as **schema-declared but unexercised** with the rev-47 delegated default (optional-kind raw_offsets empty, `source` optional/valid if `Some`, **no** source-payload-bytes-equality requirement). The §5.1 gap paragraph and G1 restated the same scope. The unresolved **source-provenance relation / source equality** is stated as a **T01/T03 `/6` contract dependency** (exact provenance/equality details deferred), **not** an accepted/frozen or proven behavior. Planned; no implementation, no execution, no freeze; `/5` current, `/6` unfrozen, M1 DRAFT. | M1 rev-27 artifact-handoff integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | Rev 28: integrated the read-only audit caveats (no Part A/Part B gate change, no non-map grammar requirement change, no implementation claim, no freeze; **no acceptance pass claimed**). (1) `M1-PP-08`/G1: the **missing-`source`/invalid-map rejection-before-mutation** is stated as a **planned required contract test**, not current/frozen behavior; the exact **fault classification/diagnostic** (`ArtifactSourceMissing`/`ArtifactSourceMismatch`/`ArtifactMapInvalid` = proposed names, not frozen) and **atomic artifact behavior** remain **T03/T01 co-freeze** items and are **not asserted as pass**. (2) Added the **exact map-boundary assertions required by T03 rev 53** to `M1-PP-01`/`01b`/`01c` (identity `[0,1,…,28]`; inserted-LF `[0,1,…,27,27]` zero-width at raw EOF; CRLF collapse `[0,1,…,26,27,29]`), each qualified as the **rev-53 delegated candidate pending T03/T01 co-freeze**, grounded in and checked against the section-3 fixture lengths (28/27/29); added a §5.1 boundary note. (3) Resolved the **`M1-PP-04` EOF coordinate ambiguity**: `EOF[28,28)` is qualified to the canonical `M1-SRC-000` (28-byte identity) raw source end; for CRLF/inserted-LF variants the EOF raw coordinate is the **map raw boundary** (27/29), not the normalized output length. Planned; no implementation, no execution, no freeze; `/5` current, `/6` unfrozen, M1 DRAFT. | M1 rev-28 review-integration subagent (DeepSeek doc integrator) |
