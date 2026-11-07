# A-AUD-04 — Parsing, Symbol/Type, and Semantic Implementation Audit

| Field | Value |
|---|---|
| Status | **READ-ONLY AUDIT COMPLETE — REPORT ONLY**; no source, test, contract, Cargo, `mod.rs`, or shared-index edits; no commit or push |
| Date | 2026-11-07 |
| Task | Stage A `A-AUD-04` under [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) |
| Base revision | `430ae29162efef90f86667f384097964d611604f` (`wave4-development`) |
| Frozen contract | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` |
| Report path | `docs/reviews/dispatch/A04_PARSE_TYPE_SEMANTICS.md` |
| Scope | T05 parsing, T06 symbol/type, T07 semantics; frozen slices, untracked parse candidates, registration, tests, real-source driver integration, and collisions |

This report separates **implemented behavior** (code plus tests actually exercised), **frozen contract**, **Ack-only certification** (validation that writes no language record), **explicit `Unsupported`**, **unregistered drafts**, **proposed/planned contract text**, and **unverified claims**. Source comments are treated as claims, not proof.

---

## 1. Checked Baseline and Workspace State

### 1.1 Revision and uncommitted state

At audit start the workspace contained user work; this audit did not modify it.

| State | Paths |
|---|---|
| Modified (user) | `compiler/src/chips/parse/mod.rs` — adds `pa_cast`, `pa_decl_spec`, `pa_ext_dispatch`, `pa_init`, `pa_stmt`; does **not** add `pa_expr_ext` or `pa_typedef_query` |
| Untracked parse drafts (user) | `pa_cast.rs`, `pa_decl_spec.rs`, `pa_expr_ext.rs`, `pa_ext_dispatch.rs`, `pa_init.rs`, `pa_stmt.rs`, `pa_typedef_query.rs` |
| Untracked outside this scope | `compiler/src/chips/lex/lx_error.rs`, `lx_extension.rs`; `docs/tasks/{CT_FREEZE_01..03,H04_I_FREEZE,OP_FREEZE_01..03,VF13_FREEZE}.md` |
| Concurrent dispatch artifacts | `docs/reviews/dispatch/A01_FRAMEWORK_BASELINE.md`, `A05_IR_TARGET_HOST.md`, `docs/tasks/DISPATCH_LEDGER.md` appeared during the audit and were not authored or verified by this task |

The frozen marker and `contract.rs` both state `/39`. The candidate driver prints `candidate evidence (t01-c01-c06/39)`.

### 1.2 Checks actually run by this audit

| Check | Exact command | Result |
|---|---|---|
| Compiler library check | `cargo check --manifest-path compiler/Cargo.toml --lib` | **PASS**; `137` warnings, all dead-code warnings from the five newly wired draft modules |
| Compiler clippy (lib) | `cargo clippy --manifest-path compiler/Cargo.toml --lib -- -D warnings` | **FAIL**; `137 previous errors` (dead code promoted to errors) |
| Scoped frozen tests | `cargo test --manifest-path compiler/Cargo.toml --test c12_parse --test c13_ty --test c14_se --test c34_parse --test c35_expr --test c36_recovery --test c39_sefunc` | **PASS**; `63 passed, 0 failed` (5 + 8 + 9 + 12 + 10 + 11 + 8) |
| Draft-local tests (compiled modules only) | `cargo test --manifest-path compiler/Cargo.toml --lib -- "chips::parse::pa_cast"` etc. | **PASS**; 47 tests total: `pa_cast` 15, `pa_decl_spec` 9, `pa_ext_dispatch` 5, `pa_init` 6, `pa_stmt` 12 |
| Candidate driver, canonical source | `compiler/target/debug/candidate <(printf 'int main(void){return 2+3;}\n')` | **PASS**; exit 0; `interpret: value=05 negative=false` |
| Candidate driver, non-M1 operator `2*3` | same driver, source `int main(void){return 2*3;}` | **ACCEPTED — DEFECT**; exit 0; `value=05` (should be 6 or explicit `Unsupported`) |
| Candidate driver, non-M1 operator `2-3` | same driver, source `int main(void){return 2-3;}` | **ACCEPTED — DEFECT**; exit 0; `value=05` (should be negative 1 or explicit `Unsupported`) |
| Candidate driver, malformed delimiters | same driver, source `int main(void]{return 2*3;)` | **ACCEPTED — DEFECT**; exit 0; `value=05` |
| Candidate driver, fold sanity | same driver, source `int main(void){return 9+9;}` | `value=12` (hex magnitude, i.e. 18); confirms literal values reach the fold |

The driver output prints magnitude bytes in hex (`05` = 5, `12` = 0x12 = 18). These are **Part A symbolic IR-interpretation probes on macOS**, not target compilation/execution and not GCC torture evidence.

Not run: full workspace tests, full clippy targets, formatting, docs validators, GCC torture, Linux/AArch64 probe, target execution, and tests in `pa_expr_ext.rs`/`pa_typedef_query.rs` (those modules are not compiled).

### 1.3 Contract status versus implementation

`/39` freezes the **code/schema slices that exist**, not the whole T05/T06/T07 language contracts. The task packages still say:

- T05 overall bundle remains **PENDING** (`docs/tasks/T05_PARSE_CHIPS.md:54`), with item A–H candidate/co-freeze text.
- T06 CDR row **E remains PENDING** (`docs/tasks/T06_SYMBOL_TYPE_CHIPS.md:7` and `:133`).
- T07/CDR row **F remains PENDING** (`docs/tasks/T07_SEMANTIC_CHIPS.md:50`).

Therefore this audit treats frozen slices as **implemented slices**, not as completed parsers/type systems. Proposed M1 lifecycle text in T05/T06/T07 is **not** counted as implemented capability.

---

## 2. Summary Ledger

| Group | Catalog rows | Rows with some code | Real record/AST producer | Ack-only / explicit `Unsupported` | Draft-only | No file/kind |
|---|---:|---:|---:|---:|---:|---:|
| T05 parse (PA01–PA38) | 38 | 30 nominal (13 frozen + 17 draft) | **PA01 only** | 12 frozen Ack-only; 3 draft files / 9 rows are explicit-`Unsupported` or empty shells | 7 files / 17 unique rows | PA06, PA11–PA13, PA17–PA19, PA24 |
| T06 symbol/type (TY01–TY34) | 34 | 9 nominal (TY01,02,03,07,13,17,25,26,27) | TY01/02/07/13/17/25–27 produce records; TY03 is query-only | — | 0 | 25 rows |
| T07 semantic (SE01–SE30) | 30 | 4 nominal (SE02, SE07, SE21, SE29) | 4; plus VF06 verifier | — | 0 | 26 rows |

“Rows with some code” is not a quality or percentage metric; most T05 rows are validation-only and produce no AST or symbol/type/semantic record.

---

## 3. T05 Parsing — Per-Capability Status

Status legend: **AST** = produces committed AST; **ACK** = frozen validation returning `ResultValue::Ack`, writes nothing; **UNS** = explicit `Unsupported`; **DRAFT** = unregistered candidate; **ABSENT** = no file or task kind.

### 3.1 Frozen, registered T05 slices

| Row(s) | Status / real role | Implementation evidence | Registration / driver | Test evidence |
|---|---|---|---|---|
| PA01 TranslationUnit | **AST**, M1 fixed shape only | `compiler/src/chips/parse/pa_tu.rs:116-151` manifest; `:153-168` 13-kind table; `:193-233` shape gates; `:250-355` nine-node pre-order append and `Complete(Record(Node(root)))` | `PARSE_TU` local 16 / `PA_TU_CHIP = ChipId(7)`; allowlist `parse.nodes`; **candidate driver integrated** | `compiler/tests/c12_parse.rs:255,276,361,444,487`; 5/5 passed |
| PA02 ExternalDeclaration | **ACK** | `compiler/src/chips/parse/pa_external.rs:193-249` manifest + `Complete(Ack)`; `:261-320` `{` vs `;` classification; reads continuation | local 17 / `ChipId(44)`; not in driver | `compiler/tests/c34_parse.rs:408,424,515`; c34 12/12 passed |
| PA03 DeclarationSpecifiers | **ACK**, only one `int` token | `compiler/src/chips/parse/pa_specifier.rs:212-286` | local 18 / `ChipId(45)`; not in driver | `compiler/tests/c34_parse.rs:455,492`; c34 |
| PA05/PA07/PA09 (fused declarator) | **ACK**, only `main(void)`; `()` is a typed DEFECT | `compiler/src/chips/parse/pa_declarator.rs:162-365`; pure cores `:374-510` | local 19 / `ChipId(46)`; not in driver | `compiler/tests/c34_parse.rs:492,515`; c34 |
| PA14 DeclarationFinish | **ACK**, only `main(void);`, no declare task emitted | `compiler/src/chips/parse/pa_pod.rs:246-440`; parser `:455-557` | local 25 / `ChipId(50)`; not in driver | `compiler/tests/c36_recovery.rs:273,287,306`; c36 11/11 passed |
| PA16/PA22 (fused primary/binary) | **ACK**, only one integer and `<int> + <int>` | `compiler/src/chips/parse/pa_binary.rs:392-505`; pure core `:520-613` | locals 22/23 / `ChipId(48)`; not in driver | `compiler/tests/c35_expr.rs:282,314,375`; c35 10/10 passed |
| PA20 UnaryExpression | **ACK**, only `+<int>` / `-<int>`; no unary AST | `compiler/src/chips/parse/pa_unary.rs:339-390`; core `:403-439` | local 24 / `ChipId(49)`; not in driver | `compiler/tests/c35_expr.rs:339,360,375`; c35 |
| PA28/PA32 (fused block/return) | **ACK**, only 7-token block and 5-token return | `compiler/src/chips/parse/pa_block.rs:327-480`; cores `:491-545` | locals 20/21 / `ChipId(47)`; not in driver | `compiler/tests/c34_parse.rs:559,580,608`; c34 |
| PA38 ParseRecovery | **ACK**, delimiter synchronization only; no scope cleanup, no continuation drop | `compiler/src/chips/parse/pa_recovery.rs:314-362`; core `:417-503` | local 26 / `ChipId(51)`; not in driver | `compiler/tests/c36_recovery.rs:345,354,384,411`; c36 |

**Key result:** only PA01 produces AST. PA02/03/05/07/09/14/16/20/22/28/32/38 do not append any parse or symbol record and do not enqueue child tasks. Their manifests declare `writes: vec![]`. They cannot satisfy T05’s “independent AST snapshot and child-task replay” acceptance on their own.

### 3.2 T05 draft / candidate files

| File | Claimed rows | Claimed locals / chip | Compiled under `mod.rs` | Registered | Actual M1 behavior | In-file tests |
|---|---|---|---|---|---|---|
| `pa_cast.rs` | PA21, PA23, PA25 | 27,28,29 / `ChipId(58)` (file lines `:140,146,152,158`) | Yes | No | All productions reject as `Unsupported`; pure classifiers only locate geometry (`:636-815`). `compute_*` has an unreachable `Ok -> Complete(Ack)` arm (`:586-620`) | 15/15 passed |
| `pa_decl_spec.rs` | PA08, PA10 | 27,28 / `ChipId(58)` (`:121,126,131`) | Yes | No | PA08 accepts an **absent** suffix and Acks; any `[...]` is `Unsupported`. PA10 accepts only bare `int`; all abstract declarators `Unsupported` (`:540-575`, `:589-728`) | 9/9 passed |
| `pa_expr_ext.rs` | PA15, PA26 | 27,28 / `ChipId(58)` (`:140,146,152`) | **No** | No | Uncompiled/unregistered. PA15 core accepts `= <int>` and Acks; PA26 always `Unsupported` (`:650-698`, `:730-759`) | 11 tests present but **not compiled/run** |
| `pa_ext_dispatch.rs` | PA33–PA37 | 27–31 / `ChipId(58)` (`:103-121`) | Yes | No | Every production always fails `Unsupported`; it never completes (`:279-352`) | 5/5 passed |
| `pa_init.rs` | PA15 | 27 / `ChipId(58)` (`:116,122`) | Yes | No | Accepts exactly `= <int>` and Acks; no InitTree record (`:301-407`) | 6/6 passed |
| `pa_stmt.rs` | PA27, PA29–PA31 | 27–30 / `ChipId(58)` (`:117-135`) | Yes | No | PA27 accepts exactly the 7-token block / 5-token return and Acks; PA29/30/31 always `Unsupported` (`:614-654`, `:667-798`). `compute_if/switch/loop` also contain unreachable `Ok -> Ack` arms | 12/12 passed |
| `pa_typedef_query.rs` | PA04 | 27 / `ChipId(58)` (`:123,129`) | **No** | No | Uncompiled/unregistered. Projection always sets `typedef_visible: false`; a visible typedef is `Unsupported` (`:204-273`, `:318-345`) | 10 tests present but **not compiled/run** |

All seven drafts are untracked; none is in `task.rs`, `manifest.rs`, `contract.rs`, `chips/mod.rs`, or a shared test entry. Only the five listed as compiled are reachable enough for dead-code analysis; **`pa_expr_ext.rs` and `pa_typedef_query.rs` are not in the build at all.**

### 3.3 T05 rows with no implementation

- **PA06 PointerDeclarator**: no file/task; `pa_declarator.rs` rejects pointer declarators (`:597-610`).
- **PA11 AggregateSpecifier, PA12 EnumSpecifier, PA13 MemberDeclaration**: no file/task/kind.
- **PA17 PostfixExpression, PA18 CallExpression, PA19 MemberSubscript**: no file/task/kind. This blocks calls, `[]`, `.`, `->`, `++/--` entirely.
- **PA24 AssignmentExpression**: no file/task/kind. Assignment syntax cannot be produced.

### 3.4 T05 shape and integration findings

1. **The AST vocabulary is frozen at eight kinds**: `TranslationUnit, FunctionDefinition, Specifiers, Declarator, Compound, Return, BinaryAdd, IntLiteral` (`compiler/src/bus.rs:545-562`). There is no `Declaration`, `Identifier`, `ExpressionStatement`, `Assignment`, `Unary`, `Call`, `If`, `Loop`, `Initializer`, or `Parameter` node. Draft-local shapes such as `CastNodeShape`, `InitNodeShape`, `LocalStmtShape` are not bus records.
2. **`PaTuChip` is the only AST producer**, and it accepts one fixed 13-token/9-node tree (`pa_tu.rs:153-168`, `:250-355`). There is no general external-declaration loop; PA02’s dispatch is not wired into PA01.
3. **No parse continuation is ever produced.** `ContinuationRecord` exists (`compiler/src/task.rs:1557-1576`) and PA02 reads one (`pa_external.rs:114-184`), but no chip emits `ContinuationDraft`, no chip enqueues children, and no `next_cursor` result carrier is frozen (OB-30). Parse-depth accounting (`ParseDepthExceeded`) is only mentioned in comments; only the generic task-depth limit exists (`compiler/src/bus.rs:1059-1068`, `compiler/src/commit.rs:848-856`).
4. **Real-source parser accepts non-M1 syntax and can produce wrong values.** `pa_tu.rs:201-233` checks token *kinds* at all 13 positions but spellings at only `int`, `main`, `void`, `return`. It does not check `(`, `)`, `{`, `}`, `+`, or `;`. Consequently:
   - `int main(void){return 2*3;}` is accepted as `BinaryAdd` and interpreted as 5, not 6.
   - `int main(void){return 2-3;}` is accepted as `BinaryAdd` and interpreted as 5, not -1.
   - `int main(void]{return 2*3;)` is accepted despite unmatched delimiters.
   The PA16/PA22 validators correctly reject `*`/`-` in `c35_expr.rs:375-395`, but that validation is **not on the candidate driver path**.
5. **VF05 does not catch the operator-spelling defect.** It verifies parent/children, token ranges, per-kind child counts, name/literal presence, and literal token origin (`compiler/src/chips/verify/vf_syntax.rs:5-14`, `:212-300`) but has no operator/delimiter spelling table. The driver runs VF05 and still returns success for the malformed probes.
6. **PA14 certifies a point-of-declaration registration but does not emit a T06 declare task** (`pa_pod.rs:246-440`). PA02/PA03/PA05 likewise only return `Ack`; the wiring layer that would consume their results does not exist.
7. **No AST node carries an operator, type, value, or declaration flag.** `NodeRecord` is `{kind,parent,children,first_token,last_token,name,literal}` (`bus.rs:526-541`); consumers re-derive semantics from fixed shapes.

---

## 4. T06 Symbol/Type — Per-Capability Status

### 4.1 Implemented / frozen rows

| Row(s) | Status | Implementation evidence | Driver use | Test evidence |
|---|---|---|---|---|
| TY01 ScopeEnter | **Implemented subset**: file scope on `TranslationUnit`, block scope on `Compound` | `compiler/src/chips/symbols/ty_scope.rs:3-10`; enter logic `:255-330`; one `Scope` + one `Enter`; duplicate guards | File enter only | `compiler/tests/c13_ty.rs:513-620`; c13 8/8 passed |
| TY02 ScopeExit | **Implemented subset**: block exit only; file never exits; one exit after enter | `ty_scope.rs:349-430` | Not used by driver | `c13_ty.rs:513-620` |
| TY03 OrdinaryNameLookup | **Implemented subset**: active scope chain, function/object only, highest same-scope `SymbolId`; **no query point / no point-of-declaration ordering** | `compiler/src/chips/symbols/ty_symbol.rs:153-231` projection; `:340-380` compute | Route registered; not exercised by driver | `c13_ty.rs:622-737` |
| TY07 SymbolDeclare | **Implemented subset**: only a **function** type in a **file** scope; hardcodes `Function`, `External`, `Static`; block-scope declarations explicitly `Unsupported` | `ty_symbol.rs:293-338`; guards `:112-140` | Declares `main` | `c13_ty.rs:622-737` |
| TY13 BuiltinType | **Implemented subset**: canonical `int {Int,true}` only; lowest-ID reuse scan | `compiler/src/chips/types/ty_types.rs:24-31`, `:52-57`, `:143-233` | Used (`int`) | `c13_ty.rs:453-512` |
| TY17 FunctionType | **Implemented subset**: `int(void)`, zero params, prototype, non-variadic only | `ty_types.rs:196-220` | Used (`int(void)`) | `c13_ty.rs:453-512` |
| TY25 IntegerPromotion / TY26 ArithmeticConversion / TY27 AssignmentConversion | **Identity-only**: all operands must be `int`; returns first `TypeId`; no `ConversionPlan` | `compiler/src/chips/types/ty_conv.rs:3-10`, `:153-178` | Routes registered; not exercised by driver | `c13_ty.rs:738-792` |

Registration in the frozen slice: `TY_TYPE_CHIP=8`, `TY_SCOPE_CHIP=9`, `TY_SYMBOL_CHIP=10`, `TY_CONV_CHIP=11`; stage 3; allowlist rows exist; test `c13_ty.rs:382-452` freezes them.

### 4.2 Missing T06 rows

No implementation file/task kind exists for:

- **TY04 TagNameLookup, TY05 LabelNameLookup, TY06 MemberNameLookup** — no tag/label/member spaces in the schema.
- **TY08 TypedefRegister, TY09 Redeclaration, TY10 LinkageResolve, TY11 StorageDuration, TY12 TentativeDefinition** — no typedef symbol, compatibility model, linkage resolver, or tentative-definition flow.
- **TY14 QualifiedType, TY15 PointerType, TY16 ArrayType, TY18 AggregateType, TY19 EnumType** — `TypeKind` has only `Void, Bool, Char, Int, Function` (`bus.rs:298-323`); no pointer/array/aggregate/enum/qualified variants.
- **TY20 DeclaratorBind** — absent. The `ty_symbol.rs` header mentions TY04/TY10/TY20, but the code implements only `declare` and ordinary `lookup`; the header is an overclaim.
- **TY21 TypeCompatibility, TY22 CompositeType, TY23 LvalueConversion, TY24 ArrayFunctionDecay** — absent.
- **TY28 ArgumentConversion, TY29 ExplicitCast, TY30 AtomicType, TY31 DeclarationConstraint, TY32 InlineLinkage, TY33 EffectiveType, TY34 RestrictContract** — absent. PA21 cast correctly refuses because `ConversionOp`/type-name decisions do not exist (`pa_cast.rs:5-31`, `:764-815`).

### 4.3 Schema and contract divergences

1. `SymbolKind` is only `{Function, Object}` (`bus.rs:327-332`). T06’s proposed namespace closure needs `Typedef`, `EnumConst`, `Label`, `StructTag`, `UnionTag`, `EnumTag` (`T06_SYMBOL_TYPE_CHIPS.md` operative item 3). None exists.
2. `SymbolRecord.decl` is a `NodeId` documented as “M1: the `Declarator` node” (`bus.rs:375-376`, `ty_symbol.rs:328`). T06 item 6b proposes the `Identifier` leaf plus complete-declarator visibility (`T06_SYMBOL_TYPE_CHIPS.md:36-38`). The frozen `/13` choice and the pending T06 candidate conflict; V1 local variables need an explicit decision.
3. Lookup has no source/start/end/query-node tuple (`ty_symbol.rs:153-164`), so it cannot implement C11 6.2.1p7 visibility or the required `int n[n]` contrast. It is a name+scope-chain lookup only.
4. File scope records `owner: None` (`ty_scope.rs:271`) while T06 item 6b identifies a scope by its owner lexical node (TU for file scope). Block scope does record `owner: Some(compound)`. This is another frozen-slice versus pending-candidate divergence.
5. `TyConvChip` records **no conversion fact** in `SemRecord`; identity is represented by the absence of a plan. That is a valid M1 slice choice, but it cannot support V1 assignment/return conversions without a schema extension.

---

## 5. T07 Semantics — Per-Capability Status

### 5.1 Implemented / frozen rows

| Row(s) | Status | Implementation evidence | Driver use | Test evidence |
|---|---|---|---|---|
| SE02 LiteralExpression | **Implemented subset**: `IntLiteral` only, canonical `int`, `NonLvalue`, `EffectMask(0)`, exactly one `SemRecord` per node with reuse | `compiler/src/chips/semantic/se_literal.rs:3-10`, `:189-230` | Yes | `compiler/tests/c14_se.rs:453-503`; c14 9/9 passed |
| SE07 ArithmeticBinary | **Implemented subset**: `BinaryAdd` only; two-phase Sem append + real `const_fold` child + await; no conversion plan | `compiler/src/chips/semantic/se_binary.rs:3-12`, `:192-246` | Yes | `c14_se.rs:504-591` |
| SE21 ReturnStatement | **Implemented subset**: one `BinaryAdd`/`int` operand and identity `int` return; no non-identity plan | `compiler/src/chips/semantic/se_return.rs:3-10`, `:138-230` | Yes | `c14_se.rs:592-650` |
| SE29 FunctionDefinition | **Implemented subset**: `int main(void)` only; single TY17 signature, declared `main`, checked `Return` child; appends signature-carrying SemRecord; reuses existing record; K&R/mismatch `Unsupported` | `compiler/src/chips/semantic/se_function.rs:3-14`, `:270-345` | **No** — `/39` registry is not in `pp_emit_slice`; `candidate.rs` does not register `SeFuncChip` | `compiler/tests/c39_sefunc.rs:476,540,582,616,671,704,811,822`; 8/8 passed |
| VF06 TypedAstInvariant (T13 row, M1 companion) | **Implemented subset**: checks `IntLiteral`, `BinaryAdd`, `Return` for one int-typed, `NonLvalue`, zero-effect `SemRecord`; skips `FunctionDefinition` fact | `compiler/src/chips/verify/vf_invariant.rs:2-11`, `:180-245`; `VF06_CHIP=15` | Yes | `c14_se.rs:651-705` |

Registration: `SE_LIT_CHIP=12`, `SE_BIN_CHIP=13`, `SE_RET_CHIP=14`, `VF06_CHIP=15`, `SE_FUNC_CHIP=57`; stage 4; allowlist rows exist; `c14_se.rs:402-452` and `c39_sefunc.rs:476-539` freeze them.

### 5.2 Missing T07 rows

No implementation file/task kind exists for **SE01, SE03–SE06, SE08–SE20, SE22–SE28, SE30**. Consequences:

- No name binding (`SE01`), no unary arithmetic (`SE03`), no logical/short-circuit (`SE04`), no address/dereference (`SE05/06`).
- No pointer arithmetic/comparison (`SE08/10`), no assignment or compound assignment (`SE11/12`), no increment (`SE13`), no conditional/comma (`SE14/15`).
- No calls (`SE16`), member/subscript (`SE17/18`), `sizeof/alignof` (`SE19`), `_Generic` (`SE20`).
- No control-flow legality or jump checks (`SE22`, `SE24`, `SE25`), no switch/case (`SE23`), no effect graph/sequencing (`SE26`), no volatile/atomic plans (`SE27/28`), no alignment (`SE30`).
- `ValueCategory` has four variants but M1 emits only `NonLvalue`; `EffectMask` is only `0` (`bus.rs:424-438`). Nonzero effects are a typed failure, not a modeled effect.

### 5.3 Semantic record production findings

1. `SemRecord` is `{node, ty, category, effects}` (`bus.rs:504-513`). It has **no conversion-plan field**, no function-context field, and no effect detail. One record per checked node is enforced worker-side and tested.
2. Only IntLiteral/BinaryAdd/Return/FunctionDefinition get SemRecords. `Compound`, `Declarator`, `Specifiers`, and `TranslationUnit` do not; VF06 checks only three of the four produced kinds.
3. SE29 is frozen at `/39` but not on the candidate driver’s registry path; its only evidence is `c39_sefunc.rs`, which seeds the PP layer and uses real LX/PA/TY/SE chips.
4. No SE chip emits a `ConversionPlan`; identity is “no plan.” V1 assignment/return conversions will require a contract/schema change before code.

---

## 6. M1 Shape Limits and Real-Upstream Integration

### 6.1 What the real driver actually runs

The candidate driver (`compiler/src/bin/candidate.rs:215-293`) installs the PP→LX→PA_TU→VF05→TY→SE→VF06→IR→VF12→VF01 chain and uses `TaskKindRegistry::pp_emit_slice()` plus `StoreSchema::pp_macro_slice()`. On canonical source it runs:

- full real preprocessing and lexing,
- `PaTuChip` (the only AST producer),
- VF05 token/AST invariants,
- TY canonical int, function type, file scope enter, `main` declare,
- SE literal, binary, return,
- VF06, IR function, VF12 interpretation, VF01 store invariant.

It does **not** run PA02/03/05/14/16/20/28/32/38, SE29, block scope enter/exit, symbol lookup, or identity conversions. Those routes are registered in tests or manifests but are not invoked by the production driver. The driver is a **Part A canonical-source evidence path**, not a closed-loop target execution and not a general parser.

### 6.2 Shape limits

| Limit | Evidence | Effect |
|---|---|---|
| Exactly 13 tokens and 9 nodes | `pa_tu.rs:153-168`, `:193-233`, `:250-355` | No general translation unit, declarations, statements, or expressions |
| 8 `NodeKind` variants only | `bus.rs:545-562` | Local variables, assignment, calls, control flow cannot be represented |
| Token spelling checked at only 4 positions | `pa_tu.rs:201-233` | Non-M1 operators/malformed delimiters accepted (verified wrong values) |
| No `next_cursor`/scalar result carrier | OB-30, draft headers; `ContinuationRecord` exists but no producer | No resumable parser |
| No child parse tasks/continuations | grep: no `ContinuationDraft`/`Enqueue` in parse chips | No nesting/precedence recursion; no await-all joins |
| No automatic scope/declare fan-out | `pa_pod.rs` certifies only; `ty_scope`/`ty_symbol` are dispatched by tests/driver | Parse cannot create scopes or symbols from syntax |
| No commit-side node-link validation | `pa_tu.rs:7-12` note; VF05 only | Link coherence is worker/test-enforced |
| No target widths in types | `ty_types.rs:24-31`; `TypeKind` symbolic | Part A only; ABI/layout remain probe-gated |

### 6.3 Verified wrong-acceptance defect

This is the highest-severity finding in this audit because it is on the only real-source AST path:

- Source `int main(void){return 2*3;}` → candidate succeeds and interprets **5**.
- Source `int main(void){return 2-3;}` → candidate succeeds and interprets **5**.
- Source `int main(void]{return 2*3;)` → candidate succeeds and interprets **5**.

The parser table accepts any `Punctuator` in the operator/delimiter positions; `BinaryAdd` is the only expression kind, and T08/T09 fold as addition. The PA22 validator that would reject `*`/`-` (`pa_binary.rs:574-613`; tested at `c35_expr.rs:375-395`) is bypassed by `PaTuChip` and the driver. This is a wrong-code acceptance, not merely an `Unsupported` gap. It must be fixed before any V1 closed-loop evidence is accepted.

---

## 7. Draft, Collision, and Owner Map

### 7.1 Collision matrix

Every parse draft uses the next-free-looking values without cross-file arbitration. Frozen head: `PARSE` locals 16–26, next free local 27; frozen chip IDs up to 57, next free 58.

| Local | Claimants |
|---:|---|
| 27 | `pa_cast` PA21 (`:140`), `pa_decl_spec` PA08 (`:121`), `pa_expr_ext` PA15 (`:140`), `pa_ext_dispatch` PA33 (`:103`), `pa_init` PA15 (`:116`), `pa_stmt` PA27 (`:117`), `pa_typedef_query` PA04 (`:123`) — **7 claimants** |
| 28 | `pa_cast` PA23 (`:146`), `pa_decl_spec` PA10 (`:126`), `pa_expr_ext` PA26 (`:146`), `pa_ext_dispatch` PA34 (`:106`), `pa_stmt` PA29 (`:121`) — **5 claimants** |
| 29 | `pa_cast` PA25 (`:152`), `pa_ext_dispatch` PA35 (`:109`), `pa_stmt` PA30 (`:125`) — 3 |
| 30 | `pa_ext_dispatch` PA36 (`:112`), `pa_stmt` PA31 (`:129`) — 2 |
| 31 | `pa_ext_dispatch` PA37 (`:115`) — 1 |

| Chip ID | Claimants |
|---:|---|
| 58 | all seven parse drafts, plus the out-of-scope lex draft `compiler/src/chips/lex/lx_extension.rs:39` (`LX15_CHIP`) |

If the integrator attempts to register more than one claimant, `TaskKindRegistry::register` and `ManifestRegistry::register` reject duplicate kinds/names/IDs loudly (see `compiler/src/task.rs:1181-1210` and `compiler/src/manifest.rs:1600-1629`); the collision cannot silently coexist.

### 7.2 Duplicate capabilities and latent-success risks

- **PA15 initializer is duplicated** by `pa_init.rs` and `pa_expr_ext.rs`: both claim local 27 / `parse.initializer` / `ChipId(58)`, with different input structs and projectors. `pa_expr_ext` is uncompiled; `pa_init` is compiled but unregistered. Only one may survive.
- **PA27 statement dispatch duplicates PA28/PA32 acceptance**: it accepts the same 7-token block and 5-token return shapes that the frozen PA28/PA32 chip already accepts, under a new unregistered kind. It adds no AST.
- **Latent Ack-on-Ok arms**: `pa_cast.rs:586-620` and `pa_stmt.rs:626-654` contain `Ok -> Complete(Ack)` arms even though their parsers currently always return `Unsupported`. If a future edit lifts only the parser, the chip would report success without producing a node or semantic fact. This violates “no fabricated success” and must be removed or tied to a real producer.
- **Ack-only success is not AST production**: `pa_decl_spec` PA08 accepts an absent array suffix; PA10 accepts bare `int`; `pa_init` accepts `= <int>`; `pa_typedef_query` always returns ordinary identifier. Registering these without parent/node integration would create successful tasks with no parser output.
- **No shared fixture/test entry** exists for any draft. Five compiled drafts have in-file tests (47 passed); two uncompiled drafts have 21 tests that never run. No `compiler/tests/c40+` file references them.
- **No manifest/docs registration** exists for the drafts: no task kind, stage row, routing row, allowlist row, or contract hash update.

### 7.3 Owner map

| Area | Current owner / required owner | Exact paths |
|---|---|---|
| Shared task kinds, stages, IDs, allowlist, contract/hash | T01 integrator only | `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/src/ids.rs`, `compiler/contracts/CONTRACT_VERSION` |
| Parse module and group exports | T01 integrator (mod.rs/chips/mod.rs) | `compiler/src/chips/parse/mod.rs`, `compiler/src/chips/mod.rs` |
| PA01 parser correctness | Parse owner; currently solo writer of `pa_tu.rs` | `compiler/src/chips/parse/pa_tu.rs`, `compiler/tests/c12_parse.rs` |
| PA02/03/05/14/16/20/28/32/38 validators | Parse owners per file | the frozen parse files and their `c34`–`c36` tests |
| PA04/08/10/15/21/23/25/26/27/29–31/33–37 drafts | Unassigned candidate authors; one writer per file | the seven untracked parse files |
| Scope/symbol/type records | T06 owner | `compiler/src/chips/symbols/*.rs`, `compiler/src/chips/types/*.rs`, `compiler/tests/c13_ty.rs` |
| Semantic records/effects | T07 owner | `compiler/src/chips/semantic/*.rs`, `compiler/tests/c14_se.rs`, `compiler/tests/c39_sefunc.rs` |
| Typed AST/store verification | T13 owner | `compiler/src/chips/verify/vf_syntax.rs`, `vf_invariant.rs`, `compiler/tests/c14_se.rs` |
| Candidate driver / H04 | H04 + T01 integrator | `compiler/src/bin/candidate.rs`, `compiler/tests/h04_candidate.rs` |

No human/agent names are recorded for the untracked drafts; the owner map is by role and file, not by inferred authorship.

---

## 8. Ranked Gaps and Risks

### 8.1 V1 — integer expressions, parentheses/precedence, local variables, declaration/initialization, assignment/conversions

| Rank | Gap / risk | Evidence | Consequence |
|---|---|---|---|
| **V1-1 Critical** | PaTu accepts non-M1 operators/malformed delimiters and produces wrong values | verified driver probes; `pa_tu.rs:201-233`; `vf_syntax.rs:212-300` | Wrong-code acceptance on the only real AST path; all V1 evidence is suspect until fixed |
| **V1-2 Critical** | No AST/record vocabulary for declarations, identifiers, assignment, initializers, statements | `bus.rs:545-562`; `NodeRecord` fields | V1 local variables and assignment cannot be represented without a T01/T05 schema freeze |
| **V1-3 Critical** | `TySymbolChip` rejects non-function types and block scopes; `decl` is a `Declarator`, not the T06 candidate `Identifier` leaf | `ty_symbol.rs:112-140`, `:293-338`; `bus.rs:375-376`; T06:36-38 | No local object symbol; no point-of-declaration visibility; C11 6.2.1p7 tests impossible |
| **V1-4 High** | No parse continuations, child tasks, cursor result, or parse depth | `ContinuationRecord` exists but no producer; OB-30; draft headers | No precedence recursion, declarator nesting, or multi-declaration parsing |
| **V1-5 High** | No conversion plan/fact in `SemRecord`; conversions are identity-only | `ty_conv.rs:153-178`; `bus.rs:504-513` | Assignment/return conversions cannot be recorded or verified |
| **V1-6 High** | Initializer capability duplicated and unregistered; no InitTree node kind | `pa_init.rs` vs `pa_expr_ext.rs`; `pa_init.rs:116-122`; collision matrix | Cannot select an implementation or test entry; success would be Ack-only |
| **V1-7 Medium** | Frozen validators are Ack-only and not wired to each other or to T06 | `writes: vec![]` manifests; `pa_pod.rs:246-440`; `pa_external.rs:193-249` | “Parsed” does not mean AST/symbol produced; integration work is unstarted |
| **V1-8 Medium** | Frozen `/13` file scope owner is `None`; T06 candidate says owner node = TU | `ty_scope.rs:271`; T06 item 6b | Scope identity/lifecycle tests may diverge from the intended contract; decide at freeze |

### 8.2 V2 — if/else, loops, break/continue, short-circuit, CFG

| Rank | Gap / risk | Evidence | Consequence |
|---|---|---|---|
| **V2-1 Critical** | No control-flow node kinds, task kinds, semantic checks, or CFG | `bus.rs:545-562`; `pa_stmt.rs` PA29–31 reject; no SE22–25 | Entire V2 is unrepresentable |
| **V2-2 High** | No comparison/logical/unary operators or precedence beyond `+` | `pa_binary.rs:574-613`; `pa_unary.rs:403-439`; no SE03/04/09/10 | Conditions and short-circuit expressions cannot be formed |
| **V2-3 High** | No effect/sequence model | `EffectMask(0)` only (`bus.rs:435-438`); no SE26 | Volatile/short-circuit/sequencing semantics cannot be checked |
| **V2-4 Medium** | `pa_stmt` draft duplicates PA28/32 and contains latent Ack arms | `pa_stmt.rs:614-654`, `:667-798`; collision matrix | Registration ambiguity and possible fabricated success |
| **V2-5 Medium** | No parse-depth/continuation nesting for loops and nested blocks | comments only; no producer | V2 nesting and recovery cannot be tested on the real path |

### 8.3 V3 — parameters/calls/returns, globals, pointers, arrays

| Rank | Gap / risk | Evidence | Consequence |
|---|---|---|---|
| **V3-1 Critical** | `TypeKind` lacks pointer/array/aggregate/enum/qualified types | `bus.rs:298-323`; missing TY14–19 | Pointer/array/aggregate programs cannot be typed |
| **V3-2 Critical** | No declarator/abstract/call/postfix parse | no PA06/08/10/17/18/19/24 implementation; drafts limited | Function parameters, calls, `[]`, `.`, `->` cannot be parsed |
| **V3-3 High** | No object/global symbol resolution, linkage, storage, tentative definitions | `ty_symbol.rs:293-338` hardcodes Function/External/Static; missing TY08–12/TY31–32 | Globals and parameters lack symbols and ABI-facing linkage facts |
| **V3-4 High** | No argument/assignment/cast conversions or pointer arithmetic semantics | `ty_conv.rs`; missing TY21–29; missing SE05/06/08/10/16 | Calls and pointer code cannot be checked; PA21 remains explicit `Unsupported` |
| **V3-5 Expected blocker** | Target ABI/widths/layout unverified; Part B probe not run | `CONTRACT_VERSION`: `target_verification=unverified`; T11 probe-gated | No ABI/codegen claim may be made from Part A evidence |

### 8.4 Cross-cutting risks

| Rank | Risk | Evidence |
|---|---|---|
| **X-1 High** | Current dirty workspace fails `clippy -D warnings` with 137 dead-code errors because `parse/mod.rs` wires five unregistered drafts | exact clippy run in §1.2; warnings located in the five draft files |
| **X-2 High** | `/39` includes SE29 and VF14, but the candidate driver uses `pp_emit_slice()` and does not register `SeFuncChip`/SE29 | `compiler/src/task.rs:725-737`; `compiler/src/bin/candidate.rs:215-293` |
| **X-3 High** | Tests are fixture-chip chains with seeded PP/tokens; only the canonical driver probe is real source | `c13_ty.rs:129-180`, `c14_se.rs` seed PP; `c34`–`c36` seed tokens; driver probe canonical only |
| **X-4 Medium** | Draft/driver registration is not automatic; success in `c34`–`c36`/`c39` does not prove production registration | test installers register local subsets; no shared test entries for drafts |
| **X-5 Medium** | Proposed T05/T06/T07 contract text can be mistaken for frozen behavior | T05:54, T06:7/133, T07:50 all PENDING |

---

## 9. Recommended Stage-B Work Orders

All orders obey the Stage-B readiness gates: no coding against an unfrozen shared interface. IDs are recommendations, not assigned tasks. Paths are exact; the integrator still owns shared files.

### 9.1 Immediate, no semantic freeze required

| ID | Owner role | Exact owned paths | Goal / acceptance | Gate |
|---|---|---|---|---|
| `B-INT-00` | T01 integrator | `compiler/src/chips/parse/mod.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/tests/freeze.rs` | Arbitrate the seven drafts: assign unique `PARSE` locals/ChipId or park them; remove compiled-but-unregistered modules from `mod.rs` or fully register them; restore `clippy -D warnings`; choose PA15 winner (`pa_init` vs `pa_expr_ext`); no draft-body rewrite | Workspace ownership; no schema change |
| `B-V1-FIX-01` | Parse owner | `compiler/src/chips/parse/pa_tu.rs`, `compiler/tests/c12_parse.rs`, `compiler/tests/h04_candidate.rs` | Make PA01 reject every non-M1 token spelling: delimiters, operator, semicolon, EOF; add negative driver cases for `2*3`, `2-3`, unmatched `]`/`)`; expected results are `Unsupported`/failure, not 5 | Can start immediately on the frozen parser |
| `B-V1-DRV-00` | H04/integrator | `compiler/src/bin/candidate.rs`, `compiler/tests/h04_candidate.rs` | Add a fail-closed negative fixture matrix to the candidate driver; ensure no malformed source reaches IR interpretation | Depends on `B-V1-FIX-01` |

### 9.2 V1 work orders (blocked on V1 schema/contract freeze)

| ID | Owner role | Exact owned paths | Goal / acceptance | Gate |
|---|---|---|---|---|
| `B-V1-SCH-01` | T01 integrator + T05/T06/T07 reviewers | `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/ids.rs`, `compiler/src/records.rs`, `compiler/src/contract.rs`, `compiler/contracts/CONTRACT_VERSION`, `docs/tasks/T05_PARSE_CHIPS.md`, `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md`, `docs/tasks/T07_SEMANTIC_CHIPS.md` | Freeze V1 AST kinds/fields, object symbol kind, block-scope declaration, point-of-declaration lookup tuple, conversion facts, and V1 task/stage entries | **BLOCKED_CONTRACT** until integrator freeze |
| `B-V1-PAR-01` | Parse owner | `compiler/src/chips/parse/pa_external.rs`, `pa_specifier.rs`, `pa_declarator.rs`, `pa_pod.rs`, `pa_init.rs`, `compiler/tests/c40_v1_parse.rs` | Produce real AST for `int main(void){int x; return x;}`-class input and `int x = 3;`; no Ack-only success; emit scope/declare child requests | `B-V1-SCH-01` |
| `B-V1-PAR-02` | Parse owner | `compiler/src/chips/parse/pa_binary.rs`, `pa_unary.rs`, new `compiler/src/chips/parse/pa_assignment.rs`, `compiler/tests/c40_v1_expr.rs` | Real precedence/parenthesized expressions and assignment; keep PA21 `Unsupported` until TY29 freezes | `B-V1-SCH-01` |
| `B-V1-TY-01` | T06 owner | `compiler/src/chips/symbols/ty_scope.rs`, `ty_symbol.rs`, `compiler/src/chips/types/ty_types.rs`, `compiler/tests/c40_v1_ty.rs` | Block-scope object declare, lookup with query point, `decl` decision, shadowing contrast (`int n[n]` vs `int n = n`), file/body scope lifecycle | `B-V1-SCH-01`; resolves V1-3/V1-8 |
| `B-V1-SE-01` | T07 owner | `compiler/src/chips/semantic/se_literal.rs`, `se_binary.rs`, `se_return.rs`, new `se_name.rs`, `se_assignment.rs`, `se_unary.rs`, `compiler/tests/c40_v1_se.rs` | Type/category/conversion facts for names, assignment, identity return; one Sem per node; no fabricated conversions | `B-V1-SCH-01` |
| `B-V1-VER-01` | T13 owner | `compiler/src/chips/verify/vf_syntax.rs`, `vf_invariant.rs`, `compiler/tests/c40_v1_verify.rs` | Extend per-kind AST table and conversion-completeness checks; add negative fixtures for wrong operator, missing conversion, wrong scope link | `B-V1-SCH-01` |
| `B-V1-DRV-01` | H04/integrator | `compiler/src/bin/candidate.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/tests/h04_candidate.rs` | Register and route V1 kinds; run a real source V1 chain through AST→type→semantic; explicit failure for non-V1 | `B-V1-SCH-01`; after `B-V1-PAR/TY/SE` |

### 9.3 V2 work orders (blocked on control-flow schema freeze)

| ID | Owner role | Exact owned paths | Goal / acceptance | Gate |
|---|---|---|---|---|
| `B-V2-SCH-01` | T01 integrator + T05/T07 | `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/contracts/CONTRACT_VERSION` | Freeze If/While/For/Do/Switch/Break/Continue/Goto/ExpressionStatement/Comparison/Logical nodes, effect/sequence facts, CFG records | **BLOCKED_CONTRACT** |
| `B-V2-PAR-01` | Parse owner | `compiler/src/chips/parse/pa_stmt.rs`, new `compiler/src/chips/parse/pa_switch.rs`, `compiler/src/chips/parse/pa_ext_dispatch.rs`, `compiler/tests/c41_v2_stmt.rs` | Real condition/loop/switch/jump parse with restructuring; delete the duplicate PA27 acceptance or make it the sole producer | `B-V2-SCH-01`; resolves V2-1/V2-4 |
| `B-V2-SE-01` | T07 owner | new `compiler/src/chips/semantic/se_condition.rs`, `se_jump.rs`, `se_switch.rs`, `se_sequence.rs`, `compiler/tests/c41_v2_se.rs` | Scalar conditions, loop/switch legality, short-circuit effects, sequencing facts | `B-V2-SCH-01` |
| `B-V2-IR-01` | T09 owner (coordination only) | `compiler/src/chips/ir_lower/` (T09-owned), `compiler/tests/c41_v2_ir.rs` | CFG lowering and verification for V2 facts; no code before IR freeze | IR/V2 contracts; outside this audit’s implementation scope |

### 9.4 V3 work orders (blocked on type/declarator/call schema freeze)

| ID | Owner role | Exact owned paths | Goal / acceptance | Gate |
|---|---|---|---|---|
| `B-V3-SCH-01` | T01 integrator + T06/T11 | `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md` | Freeze pointer/array/aggregate/enum/qualified types, parameter/call nodes, linkage/storage/object symbol model, ABI-facing signature shape | **BLOCKED_CONTRACT**; ABI values blocked on probe |
| `B-V3-PAR-01` | Parse owner | `compiler/src/chips/parse/pa_declarator.rs`, `pa_decl_spec.rs`, new `compiler/src/chips/parse/pa_call.rs`, new `postfix` file under `compiler/src/chips/parse/`, `compiler/tests/c42_v3_parse.rs` | Pointers, arrays, parameters, calls, member/subscript; no guessing typedefs; parse every declarator nesting through explicit frames | `B-V3-SCH-01` |
| `B-V3-TY-01` | T06 owner | `compiler/src/chips/types/ty_types.rs`, new `ty_pointer.rs`, `ty_array.rs`, `ty_aggregate.rs`, `ty_compat.rs`, `ty_args.rs`, `compiler/tests/c42_v3_ty.rs` | Build/canonicalize pointer/array/aggregate types; compatibility and argument conversion plans; no host widths | `B-V3-SCH-01`; target values probe-gated |
| `B-V3-SE-01` | T07 owner | `compiler/src/chips/semantic/se_function.rs`, new `se_call.rs`, `se_name.rs`, `compiler/tests/c42_v3_se.rs` | Parameters, calls, returns, global references, pointer/array category and effects | `B-V3-SCH-01` |
| `B-V3-ABI-01` | T11/H07 + T01 integrator | T11/T01 target contract paths (no implementation path authorized here) | Probe-backed ABI classification, widths, argument/return convention; remains `BLOCKED_PROBE` | H01/H07 probe + attestation |

### 9.5 Suggested sequence

1. `B-INT-00` and `B-V1-FIX-01` now (they fix real defects/hygiene without new semantics).
2. `B-V1-SCH-01` freeze V1 AST/symbol/conversion shape; then `B-V1-TY-01`, `B-V1-PAR-01/02`, `B-V1-SE-01`, `B-V1-VER-01`.
3. `B-V1-DRV-01` after producers are integrated; accept V1 only when the driver runs a real V1 source through the chain and negative fixtures fail closed.
4. Repeat the freeze→producer→consumer→verifier pattern for V2 and V3; keep Part B blocked until probe evidence.

---

## 10. Unrun Checks and Open Questions

### 10.1 Unrun / unverified

- Full repository and compiler test suites were **not** run; only the seven scoped binaries and five draft-local modules listed in §1.2.
- `cargo fmt --all -- --check`, full clippy `--all-targets`, docs link/contract validators, and root-package tests were **not** run by this audit.
- `pa_expr_ext.rs` (11 tests) and `pa_typedef_query.rs` (10 tests) are not compiled or run; their compile correctness is unverified.
- No GCC torture corpus, official driver, Linux/AArch64 probe, target assembler/linker/runner, or ABI execution was run. The candidate driver output is symbolic IR interpretation on macOS.
- The concurrent `DISPATCH_LEDGER.md` reports a compiler test count and formatting/clippy results from another session; this audit did not independently reproduce those claims.
- The claim in slice documents that “full §5 suite green at commit” is historical; only current scoped tests were checked here.

### 10.2 Open questions for the integrator / Stage-B owners

1. **PA15 ownership:** should `pa_init.rs` be the single PA15 implementation, with `pa_expr_ext.rs` parked/merged, or vice versa? Their APIs and registrations conflict.
2. **PA01 evolution:** is `PaTuChip` a permanent fixed-shape shortcut to be hardened, or a temporary producer to be replaced by a real PA01 external-declaration loop before V1? The wrong-acceptance defect must be fixed either way.
3. **AST schema ownership:** approve or reject the `NodeKind` additions needed for V1 (Declaration, Identifier, Assignment, Initializer, ExpressionStatement, parentheses/unary); without them V1 is not representable.
4. **`SymbolRecord.decl`:** keep the frozen `/13` `Declarator`-node choice, or adopt T06 item 6b’s `Identifier` leaf plus complete-declarator visibility? The lookup/visibility implementation cannot satisfy both.
5. **Conversion representation:** freeze a `ConversionPlan`/conversion-fact field for `SemRecord` before V1, or explicitly keep V1 identity-only and remove assignment/conversion from V1 claims.
6. **Draft registration policy:** should the five compiled-but-unregistered drafts stay in `mod.rs` while unregistered? Current state fails clippy with 137 dead-code errors; the alternatives are register, park, or remove from `mod.rs` (without deleting user work).
7. **SE29/driver alignment:** is `/39` SE29 intended to be outside the candidate driver, or should `candidate.rs` move to `se_function_slice()` and register SE29 for the canonical function-definition fact?
8. **PA02/03/05/14/16/20/28/32/38 production integration:** these frozen validators are only exercised through tests; who owns the wiring that turns their `Ack` results into AST/symbol child tasks?
9. **Cross-group ID claim:** `lx_extension.rs:39` also claims `ChipId(58)`; is the ID arbitration across lex and parse drafts, not just within parse?
10. **T05/T06/T07 candidate text:** the operative amendments/item checklists are still PENDING; which items are accepted as V1 requirements and which remain research notes?

---

## 11. Evidence Index

Primary files read for this report:

- Task contracts: `docs/tasks/T05_PARSE_CHIPS.md`, `T06_SYMBOL_TYPE_CHIPS.md`, `T07_SEMANTIC_CHIPS.md`, `PA_SLICE.md`, `PA_DECL_SLICE.md`, `PA_EXPR_SLICE.md`, `PA_RECOVERY_SLICE.md`, `TY_SLICE.md`, `SE_SLICE.md`, `SE_FUNC_SLICE.md`, `PARALLEL_EXECUTION.md`, `MASS_PARALLEL_DISPATCH.md`.
- Frozen implementation: `compiler/src/bus.rs`, `task.rs`, `manifest.rs`, `commit.rs`, `chips/parse/*.rs` (frozen files), `chips/symbols/*.rs`, `chips/types/*.rs`, `chips/semantic/*.rs`, `chips/verify/vf_syntax.rs`, `chips/verify/vf_invariant.rs`, `bin/candidate.rs`.
- Drafts: the seven untracked `chips/parse/pa_*.rs` files.
- Tests: `compiler/tests/c12_parse.rs`, `c13_ty.rs`, `c14_se.rs`, `c34_parse.rs`, `c35_expr.rs`, `c36_recovery.rs`, `c39_sefunc.rs`, `h04_candidate.rs`.
- Baseline: `compiler/contracts/CONTRACT_VERSION`, `compiler/src/contract.rs`.

No source, test, contract, Cargo, `mod.rs`, or shared-index file was modified by this audit. The only created file is this report.
