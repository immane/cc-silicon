# A-AUD-03 — Preprocessing and Lexical Implementation Audit

| Field | Value |
|---|---|
| Status | **READ-ONLY AUDIT COMPLETE — REPORT ONLY**; no source, test, contract, Cargo, `mod.rs`, or shared-index edits; no commit or push |
| Date | 2026-11-07 |
| Task | Stage A `A-AUD-03` under [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) |
| Base revision | `430ae29162efef90f86667f384097964d611604f` (`wave4-development`) |
| Frozen contract | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` |
| Report path | `docs/reviews/dispatch/A03_PREPROCESS_LEX.md` |
| Scope | T03 preprocessing (PP01–PP28), T04 lexical (LX01–LX18), frozen slices, untracked lex drafts, registration, tests, real-upstream integration, and candidate collisions |

This report separates **implemented behavior** (code plus tests actually exercised), **frozen contract**, **explicit `Unsupported`**, **Ack-only validation**, **draft/unregistered work**, **proposed/planned work**, and **unverified claims**. Source comments and slice execution records are treated as claims, not proof.

---

## 1. Checked Baseline and Workspace State

### 1.1 Revision, contract, and concurrent user/concurrent-agent state

At audit start the workspace already contained user work; this audit did not modify it. During the audit, additional concurrent-agent files appeared (not authored or verified here).

| State | Paths |
|---|---|
| HEAD / branch | `430ae29162efef90f86667f384097964d611604f` / `wave4-development` |
| Contract marker | `compiler/contracts/CONTRACT_VERSION` = `t01-c01-c06/39`; hash matches `compiler/src/contract.rs:38-44` |
| Modified at audit start | `compiler/src/chips/parse/mod.rs` |
| Untracked at audit start | `compiler/src/chips/lex/lx_error.rs`, `compiler/src/chips/lex/lx_extension.rs`; seven `compiler/src/chips/parse/pa_*.rs` drafts; `docs/tasks/{CT_FREEZE_01..03,H04_I_FREEZE,OP_FREEZE_01..03,VF13_FREEZE}.md` |
| Appeared during audit (concurrent work) | `compiler/src/bin/candidate.rs` modified (argv UTF-8 hardening only); `compiler/src/chips/parse/pa_tu.rs` modified; `tools/chip-lint/tests/lint.rs` modified; `compiler/tests/c40_pa_guard.rs`, `c41_candidate_argv.rs`, `c42_wiring.rs`; `docs/tasks/DISPATCH_LEDGER.md`; dispatch reports `A01`, `A02`, `A04`–`A24` (this report is `A03`) |
| Wiring guard observed | `compiler/tests/c42_wiring.rs:27-32` explicitly parks `lex/lx_error.rs` and `lex/lx_extension.rs` as unwired user drafts; this is a concurrent test, not one this audit ran |
| Files inspected for this audit | `compiler/src/chips/preprocess/*` (20 files + `mod.rs`), `compiler/src/chips/lex/*` (8 wired files + 2 drafts + `mod.rs`), `compiler/src/{manifest,task,routing,bus,records,commit}.rs` as needed, `compiler/tests/c10,c11,c16,c20..c33`, `compiler/tests/c37` (read only), PP/LX slice docs |

The working tree remained live while this audit ran; the table records observed states, not an exhaustive final snapshot. Later concurrent files (for example `pa_tu.rs`/`c40_pa_guard.rs`) do not change the PP/LX source findings above; the candidate probes were rerun after the concurrent `candidate.rs` argv change.

The T03/T04 prose still contains stale `/5`/`/6` status language (for example `docs/tasks/T03_PREPROCESS_CHIPS.md:69-74`). The `/39` marker and code are the current facts used here; the task packages as a whole are **not** signed off as complete T03/T04 contracts.

### 1.2 Checks actually run by this audit

| Check | Exact command | Result |
|---|---|---|
| Scoped PP/LX acceptance tests | `cargo test --manifest-path compiler/Cargo.toml --test c10_pp01 --test c11_lex --test c16_pp --test c20_ppscan --test c21_directive --test c22_conditional --test c23_macro --test c24_expand --test c25_include --test c26_variadic --test c27_builtin --test c28_line --test c29_pragma --test c30_expand_map --test c31_emit --test c32_float --test c33_string` | **PASS**; 17 binaries, **162 passed, 0 failed** (counts: 7, 8, 8, 8, 8, 8, 10, 12, 8, 10, 9, 10, 9, 10, 11, 12, 14) |
| Inline lex unit tests | `cargo test --manifest-path compiler/Cargo.toml --lib -- chips::lex` | **PASS**; **60 passed, 0 failed**, 112 filtered out |
| Library check | `cargo check --manifest-path compiler/Cargo.toml --lib` | **PASS**; 137 warnings (dead code from concurrent parse drafts; no lex draft is in the build) |
| Candidate binary build | `cargo build --manifest-path compiler/Cargo.toml --bin candidate` | **PASS**; 137 warnings |
| Candidate M1 probe | `compiler/target/debug/candidate <m1.c>` | exit 0; `interpret: value=05 negative=false` |
| Candidate `-E` M1 probe | `compiler/target/debug/candidate -E <m1.c>` | exit 0; `int main ( void ) { return 2 + 3 ; }` |
| Candidate `#define` probe | `compiler/target/debug/candidate <define.c>` | exit 1; `unsupported directive 'define'` |
| Candidate `#include` probe | `compiler/target/debug/candidate <include.c>` | exit 1; `unsupported directive 'include'` |
| Candidate float probe | source `return 2.5;` | exit 1; `non-decimal pp-number is outside the M1 exercised subset` |
| Candidate adjacent-string probe | source `char *s="a" "b";` | exit 1; `string/character/header-name pp-token is outside the M1 exercised subset` |
| Candidate `#if` macro probe | `-DX=1` + `#if X ... #else #error X_was_not_expanded #endif` | exit 1; the `#else`/`#error` branch was taken, proving `#if X` did not expand the `-D X` macro |

The probe sources and binary are Part A symbolic front-end checks on macOS, not target compilation/execution and not GCC torture evidence. The candidate binary was rebuilt after the concurrent argv hardening; PP/LX code was not modified by that change.

### 1.3 Evidence legend used below

| Tag | Meaning |
|---|---|
| **IMPL** | Committed semantic implementation exists (not merely a stub) |
| **REG-T** | Frozen task kind / chip ID / stage / allowlist / slice registry exists, and the slice test registers and routes it |
| **REG-D** | Also registered/routed by `compiler/src/bin/candidate.rs` (the real driver) |
| **REAL** | Test input comes from real PP01→PP02→PP03→PP04 production over source bytes, not hand-seeded PP records |
| **UNS** | Out-of-scope forms are explicit `Unsupported` / typed `Fail` rather than silent success |
| **ACK** | Validation returns `Ack` and writes no language record; consumer must recompute through a helper |
| **FOLD** | Catalog rows folded into a sibling chip as helpers; no independent task |
| **DRAFT** | Untracked and not compiled/registered; in-file tests are not run by normal `cargo test` |
| **ABSENT** | No implementation, task kind, or file |

---

## 2. Executive Summary

1. **The frozen PP/LX slices are real but narrow.** PP01–PP28 and LX01–LX13 are implemented and their 17 scoped test binaries pass (162 tests), plus 60 inline lex tests. Registration, stage rows, chip IDs, and store allowlists exist for every frozen slice that writes a store.
2. **The preprocessing pipeline is not integrated beyond token scanning.** The candidate driver runs only PP01→PP02→PP03→PP04→PP19→PP09→PP05→PP26/PP28 and LX intern/classify/decode. PP06/PP07/PP08/PP16/PP17/PP18/PP23/PP24/PP25/PP27 are absent from the driver. `#define`, `#undef`, `#include`, `#line`, `#pragma` still fail at PP05 as `Unsupported`; source macros and headers are therefore not usable end-to-end.
3. **`#if` does not macro-expand identifiers.** PP19 reads the committed table only for `defined`/`#ifdef`/`#ifndef`; a plain identifier in `#if` evaluates to `0`. The probe `-DX=1` + `#if X` took the false branch. This is not merely unsupported; it can silently select the wrong conditional branch.
4. **Lexing is still M1-integer-only in the integrated path.** LX classify rejects every non-decimal pp-number and every string/character/header token; `TokenKind` has no Character/String/Float members. Float, char, string, adjacent-string, and GNU literal features exist only as isolated Ack-only or literal-record slices, not as a usable PP→LX chain.
5. **Integer decoding has a silent wrong-value case.** `010` is classified as an integer (all digits), then decoded by `decimal_magnitude`, yielding decimal 10 instead of octal 8. Radix, suffix, candidate type, and range diagnostics (LX05–LX08) are otherwise absent.
6. **Provenance and location are mostly placeholders.** PP12/PP16 synthesize tokens with the invocation-name span; no chip writes `ExpansionRecord`; PP27 is a query over records that only fixtures create. PP23/PP25 are Ack-only, PP24 uses physical line/source names and `__COUNTER__ = 0`, and LX16/LX17 have no dedicated chips.
7. **Two lex drafts are not integration candidates yet.** `lx_error.rs` (LX18) always returns `Unsupported` in `handle`; `lx_extension.rs` (LX15) is pure-classification only and Ack-only. Both are uncompiled and unregistered; `lx_extension.rs` uses `ChipId(58)`, which collides with seven concurrent parse drafts.

No GCC torture pass rate, target execution, or Linux/AArch64 evidence is claimed.

---

## 3. Per-Capability Status

### 3.1 Preprocessing — source, comments, and tokens

| Row(s) | Implementation | Registration | Test evidence | Real upstream | Status / notes |
|---|---|---|---|---|---|
| PP01 SourceNormalize | `pp_normalize.rs:201-233` (CRLF/insert LF/raw map); manifest `:104-138` | `PP01_CHIP=3` (`manifest.rs:994`), stage 1 (`manifest.rs:1449`), allowlist `manifest.rs:791-796`; **REG-D** `candidate.rs:220-247, 252-289, 606-613` | `c10_pp01.rs:76-330` | **REAL** (source bytes) | **IMPL/REG-T/REG-D**; lone CR is `Unsupported`; no trigraphs, no dialect read |
| PP02 LineSplice | `pp_splice.rs:192-208`; manifest `:99-131` | `PP_SPLICE_CHIP=17`; stage 1; allowlist `manifest.rs:900-905`; **REG-D** | `c16_pp.rs:210-226, 332-419` | **REAL** | **IMPL/REG-T/REG-D**; huge/backslash EOF cases limited to unit tests |
| PP03 CommentReplace | `pp_comment.rs:210-276`; literal/header protection `:278-473`; manifest `:103-135` | `PP_COMMENT_CHIP=18`; stage 1; allowlist `manifest.rs:906-911`; **REG-D** | `c16_pp.rs:228-252`; `c20_ppscan.rs:263-289` | **REAL** | **IMPL/REG-T/REG-D**; whole-input scan only; rev-55 chunk state not adopted; no dialect; a block comment before a line-start `#` is not treated as whitespace by the include-prefix lookahead (`:301-314`), so a following header containing `//` can be corrupted |
| PP04 PpTokenScan | `pp_scan.rs:466-599`; full punctuator table `:263-270`; include prefix `:322-374`; manifest `:99-126` | `PP_SCAN_CHIP=19`; stage 1; allowlist `manifest.rs:912-923`; **REG-D** | `c16_pp.rs:254-296`; `c20_ppscan.rs:146-261` | **REAL** | **IMPL/REG-T/REG-D/UNS**; lone `\`, `@`, `$`, backtick, non-ASCII, UCN deferred; no C23 separators; no dialect; same leading-comment disagreement with PP05's walk-back |

### 3.2 Preprocessing — directives, macros, conditionals

| Row(s) | Implementation | Registration | Test evidence | Real upstream | Status / notes |
|---|---|---|---|---|---|
| PP05 DirectiveDispatch | `pp_directive.rs:298-351`, raw-line predicate `:440-517`, taxonomy `:541-581`; manifest `:176-202` | `PP05_CHIP=23`; stage 1; **REG-D** | `c21_directive.rs:219-369` | **REAL** (`scan_source`) | **IMPL/REG-T/REG-D/UNS**; only `#error` dispatches to PP26; every implemented macro/include/line/pragma body is still `Unsupported` at PP05 |
| PP06 MacroDefinition | `pp_define.rs:291-583`; fresh append `:324-349`; manifest `:147-172` | `PP06_CHIP=26`; stage 1; allowlist `manifest.rs:924-929`; **not REG-D** | `c23_macro.rs:272-339` | **REAL** scan, manual per-line fan-out | **IMPL/REG-T**; no host/controller fan-out; duplicate parameter names not rejected; no dialect |
| PP07 MacroRedefinition | `pp_redefine.rs:418-492`; reparse `:268-416`; manifest `:156-181` | `PP07_CHIP=27`; stage 1; **not REG-D** | `c23_macro.rs:341-397` | **REAL**, manual child | **IMPL/REG-T**; replacement comparison is spelling-only (whitespace-insensitive by frozen design) |
| PP08 MacroUndef | `pp_undef.rs:159-254`; manifest `:102-135` | `PP08_CHIP=28`; stage 1; allowlist `manifest.rs:930-935`; **not REG-D** | `c23_macro.rs:397-447` | **REAL**, manual fan-out | **IMPL/REG-T**; no controller |
| PP09 MacroInvocation | `pp_invoke.rs:426-471` dispatch, `:478-539` scan, `:332-423` stitch; manifest `:235-263` | `PP09_CHIP=29`; stage 1; allowlist `manifest.rs:936-941`; **REG-D** | `c24_expand.rs:312-483, 539` | **REAL** | **IMPL/REG-T/REG-D/UNS**; variadic use fails naming PP16; nested variadic inside non-variadic args remains unsupported; no provenance |
| PP10 MacroArgumentCollect | **FOLD** helper `pp_substitute.rs:404-454, 539-560` | None (folded by `/24` §7) | Exercised through PP12 in `c24_expand.rs:313-348` | Real PP stream via PP12 | **FOLD/IMPL**; no independent task/carrier |
| PP11 MacroArgumentExpand | **FOLD** helper `pp_substitute.rs:653-702` | None | `c24_expand.rs:331-348` | Real PP stream via PP12 | **FOLD/IMPL** |
| PP12 MacroSubstitute | `pp_substitute.rs:786-893` substitution, `:1049-1075` rescan, `:281-353` compute; manifest `:200-225` | `PP12_CHIP=30`; stage 1; allowlist `manifest.rs:942-947`; **REG-D** | `c24_expand.rs:313-483` | **REAL** | **IMPL/REG-T/REG-D/UNS**; synthesized tokens all reuse the invocation-name span; no `ExpansionRecord` append |
| PP13 MacroStringify | **FOLD** helper `pp_substitute.rs:704-730` | None | `c24_expand.rs:350-378` | Real PP stream | **FOLD/IMPL**; escape rule is simple quote/backslash only |
| PP14 MacroPaste | **FOLD** helper `pp_substitute.rs:732-780` | None | `c24_expand.rs:350-378` | Real PP stream | **FOLD/IMPL**; pastes are validated by the full scanner and must form one token |
| PP15 MacroRescan | **FOLD** helper `pp_substitute.rs:1045-1075` | None | `c24_expand.rs:378-403` | Real PP stream | **FOLD/IMPL**; macro-count+2 breaker is a circuit breaker |
| PP16 VariadicMacro | `pp_variadic.rs:583-628` stream dispatch, `:634-719` stitch, `:1221-1293` single mode; manifest `:486-515` | `PP16_CHIP=33`; stage 1; allowlist `manifest.rs:948-953`; **not REG-D** | `c26_variadic.rs:280-545` | **REAL**, manual define | **IMPL/REG-T/UNS**; `__VA_OPT__` C23 only; GNU `, ## __VA_ARGS__` comma swallowing is **not implemented and not rejected** (empty tail retains the comma; a non-empty tail generally fails as a multi-token paste at `pp_variadic.rs:2127-2177`); nested variadic inside non-variadic arguments deferred |
| PP17 IncludeResolve | `pp_resolve.rs:179-256`; manifest `:105-130` | `PP17_CHIP=31`; stage 1; **not REG-D** | `c25_include.rs:284-389` | **REAL** scan of header | **IMPL/REG-T**; quote/angle identical; basename suffix policy; no host fetch; no search paths; no `#pragma once` |
| PP18 IncludeEnter | `pp_enter.rs:261-354`, header splice `:356-487`; manifest `:206-232` | `PP18_CHIP=32`; stage 1; **not REG-D** | `c25_include.rs:389-513` | **REAL**, one header source | **IMPL/REG-T/UNS**; single-pass, one resolved header per task; no recursion/stack/depth/once; `include_next` and macro header names unsupported |
| PP19 ConditionalDirective | `pp_conditional.rs:191-269` stack, `:296-390` directive step, `:620-922` evaluator; manifest `:133-159` | `PP19_CHIP=25`; stage 1; **REG-D** | `c22_conditional.rs:205-439`; `c23_macro.rs:447-490` | **REAL** | **IMPL/REG-T/REG-D/UNS**; **no macro expansion in `#if`**; i128 exact arithmetic; `&&`/`||` always evaluate both sides; target-width PP semantics deferred |
| PP20 DefinedOperator | **FOLD** in `pp_conditional.rs:888-921` | None | `c22_conditional.rs:260-280`; `c23_macro.rs:447-490` | Real PP stream | **FOLD/IMPL**; reads committed macro table for `defined` only |
| PP21 PpExpressionParse | **FOLD** `ExprParser` `pp_conditional.rs:639-922` | None | `c22_conditional.rs:280-297` | Real PP stream | **FOLD/IMPL**; T03 row says “macro-processed, then evaluated”; implementation evaluates raw identifiers as `0` |
| PP22 PpExpressionEvaluate | **FOLD** `pp_conditional.rs:620-637, 924-980` | None | `c22_conditional.rs:280-355` | Real PP stream | **FOLD/IMPL**; i128, not PP `intmax_t`/`uintmax_t`; `shift_left` uses `checked_shl`, which does **not** detect value overflow |

### 3.3 Preprocessing — line, builtins, pragma, diagnostics, map, emit

| Row(s) | Implementation | Registration | Test evidence | Real upstream | Status / notes |
|---|---|---|---|---|---|
| PP23 LineDirective | `pp_line.rs:243-270` location, `:278-391` parsing; manifest `:176-200` | `PP23_CHIP=35`; stage 1; **not REG-D**; **ACK** (no carrier) | `c28_line.rs:153-465` | **REAL** scan line | **IMPL/REG-T/ACK**; validates `#line`/GNU marker but persists nothing; `__LINE__`/`__FILE__` remain physical |
| PP24 BuiltinMacro | `pp_builtin.rs:302-340` table; manifest `:210-238` | `PP24_CHIP=34`; stage 1; allowlist `manifest.rs:954-959`; **not REG-D** | `c27_builtin.rs:230-403` | **REAL** scan | **IMPL/REG-T/UNS**; `__DATE__`/`__TIME__` unsupported; `__COUNTER__` always seed 0; `__LINE__` physical; `__FILE__` physical name; closed target-macro table |
| PP25 PragmaDispatch | `pp_pragma.rs:453-499` dispatch, `:271-404` decode/classify; manifest `:150-173` | `PP25_CHIP=36`; stage 1; **not REG-D**; **ACK** (no carrier) | `c29_pragma.rs:142-452` | **REAL** scan | **IMPL/REG-T/ACK**; `once`/`pack` classified but not persisted or consulted; `_Pragma` simple escapes only |
| PP26 PpDiagnostic | `pp_diagnostic.rs:145-185`; manifest `:83-105` | `PP26_CHIP=24`; stage 1; **REG-D** | `c21_directive.rs:279-327` | **REAL** | **IMPL/REG-T/REG-D/UNS**; only `#error`; `#warning` has no wire; diagnostic carries no span |
| PP27 ExpansionSourceMap | `pp_expand_map.rs:280-356` chain; `:448-474` compute; manifest `:411-437` | `PP27_CHIP=37`; stage 1; **not REG-D**; **ACK** (no carrier) | `c30_expand_map.rs:178-478, 545-580` | Real scan for tokens, **synthetic `ExpansionRecord` fixtures** | **IMPL/REG-T/ACK**; no chip writes `ExpansionRecord`; `G1DraftBody` has no Expansion variant; chain cannot be produced from real expansion |
| PP28 PreprocessedEmit | `pp_emit.rs:400-542` serializer; `:300-374` compute; manifest `:262-289` | `PP28_CHIP=38`; stage 1; allowlist `manifest.rs:960-965`; **REG-D** | `c31_emit.rs:209-716`; candidate `-E` M1 | **REAL** scan | **IMPL/REG-T/REG-D**; no `#line`/GNU markers; primary map only; directive strip keys on `span.expansion.is_none()` but no producer writes expansion links |

### 3.4 Lexical — tokenize/classify/decode

| Row(s) | Implementation | Registration | Test evidence | Real upstream | Status / notes |
|---|---|---|---|---|---|
| LX01 TokenClassify | `lx_classify.rs:196-310`; manifest `:160-185` | `LX_CLASSIFY_CHIP=5`; stage 2; allowlist `manifest.rs:803-808`; **REG-D** | `c11_lex.rs:290-325`; `c20_ppscan.rs:392-445` | Positive path seeded PP tokens; driver real M1; real PP04 only for rejection | **IMPL/REG-T/REG-D/UNS**; only decimal-digit pp-numbers become `Integer`; all floats/literals/headers rejected |
| LX02 IdentifierDecode | `lx_intern.rs:113-168`; manifest `:79-102` | `LX_INTERN_CHIP=4`; stage 2; allowlist `manifest.rs:797-802`; **REG-D** | `c11_lex.rs:264-288` | Driver real M1; test seeded | **IMPL/REG-T/REG-D**; first-seen order; no UCN/encoding/dialect |
| LX03 KeywordClassify | C11 table `lx_classify.rs:27-78`; role in `:230-237` | `LX_CLASSIFY_CHIP=5`; **REG-D** | `c11_lex.rs:255-262` | Driver real M1 | **IMPL/REG-T/REG-D**; no GNU keywords/aliases; no dialect; typedef names stay identifiers (consistent with T04) |
| LX04 PunctuatorDecode | Generic `TokenKind::Punctuator` in `lx_classify.rs:250` | `LX_CLASSIFY_CHIP=5`; **REG-D** | `c11_lex.rs:290-315` | Driver real M1 | **PARTIAL**; every punctuator maps to one generic kind; spelling carries identity; no operator enum/digraph normalization |
| LX05 IntegerRadix | `lx_decode.rs:149-226` (decimal only) | `LX_DECODE_CHIP=6`; stage 2; allowlist `manifest.rs:809-814`; **REG-D** | `c11_lex.rs:327-375` | Driver real `2`,`3` | **PARTIAL/REG-T/REG-D/UNS**; no binary/octal/hex decode; **leading-zero `010` is decoded as decimal 10** (static finding, no test) |
| LX06 IntegerSuffix | No suffix parser; non-digit pp-number hits classify `Unsupported` (`lx_classify.rs:238-248`) | `LX_DECODE_CHIP=6` only for decimal digits | `c11_lex.rs:327-375` | Driver only suffix-free M1 | **ABSENT**; no `U`/`L`/`UL`/`LL`/`ULL` |
| LX07 IntegerValue | `decimal_magnitude` `lx_decode.rs:208-226` | `LX_DECODE_CHIP=6` | No test beyond `2`,`3`; `c11_lex.rs:355-375` | Driver M1 | **PARTIAL**; arbitrary decimal accumulation, but no range/type diagnostics; >u64/int128 unverified |
| LX08 IntegerTypeSelect | `LiteralRecord.candidate_type = Int` `lx_decode.rs:185-194` | `LX_DECODE_CHIP=6` | `c11_lex.rs:355-375` | Driver M1 | **PARTIAL**; only `Int`, no width/type; T08 fold rejects non-decimal/non-`None`/non-`Int` (`fold.rs:252-258`, `fold_branch.rs:499-505`) |
| LX09 FloatSyntax | `lx_float_syntax.rs:238-424`; compute `:206-229`; manifest `:172-195` | `LX09_CHIP=39`; stage 2; **not REG-D**; **ACK** | `c32_float.rs:210-307` | Seeded pp-number records; **no PP04→LX09 dispatch** | **IMPL/REG-T/ACK**; decimal/hex syntax, `f/F/l/L`; no float literal record; no GNU/C23 suffix set |
| LX10 FloatValue | `lx_float_value.rs:539-630` core; `:219-247` compute; manifest `:183-208` | `LX10_CHIP=40`; stage 2; **not REG-D**; **ACK** | `c32_float.rs:327-395, 441-511` | Seeded pp-number records | **IMPL/REG-T/ACK**; correctly rounded binary32/binary64 only; long double/binary128 explicit `Unsupported`; no published bits |
| LX11 EscapeDecode | `lx_escape.rs:327-348` core; `:482-514` compute; manifest `:446-471` | `LX11_CHIP=41`; stage 2; **not REG-D**; **ACK** | `c33_string.rs:318-394`; inline tests `lx_escape.rs:527-817` | Seeded literal records | **IMPL/REG-T/ACK**; no `CodeUnits` carrier; UTF-8 source/target assumption differs from LX12/LX13 narrow copies |
| LX12 CharacterLiteral | `lx_char.rs:408-467` compute; `:102-292` parsing/folding; manifest `:372-397` | `LX12_CHIP=42`; stage 2; allowlist `manifest.rs:966-971`; **not REG-D** | `c33_string.rs:394-459`; inline tests `lx_char.rs:469-606` | Seeded `[Token, PpToken]` | **IMPL/REG-T**; `TokenKind` has no Character, so token kind unchecked; `u8'` unsupported; multichar truncation implementation-defined; `radix:16`/`candidate_type:Int` explicit placeholders |
| LX13 StringLiteral | `lx_string.rs:643-717` compute; `:333-480` decode; manifest `:605-632` | `LX13_CHIP=43`; stage 2; allowlist `manifest.rs:972-977`; **not REG-D** | `c33_string.rs:460-547`; inline tests `lx_string.rs:719-811` | Seeded `[Token, PpToken]` | **IMPL/REG-T**; `TokenKind` has no String, token kind unchecked; `radix:0` is outside the frozen `{2,8,10,16}` domain; `candidate_type:Int` is a non-consumable placeholder; wchar UTF-32 required |
| LX14 AdjacentString | **ABSENT**; no file/task/kind | None | No test; `c31_emit.rs:360-400` only shows separate spelling preservation | PP04 emits separate string tokens | **ABSENT**; blocks all adjacent-string C |
| LX15 LiteralExtension | **DRAFT** `lx_extension.rs:252-294` compute; `:162-211` projection; manifest `:216-250` | `ChipId(58)`, LEX local 24 (not frozen, not staged, not allowlisted); **not compiled** | In-file tests `lx_extension.rs:302-432` are **not compiled/run** | Pure classification only | **DRAFT**; GNU imaginary `i/I/j/J` only; strict ISO rejects; Ack-only; overlaps LX06/LX09 suffix territory (`1F`, `1.5ll` classified `UnknownSuffix`) |
| LX16 TokenLocation | **No dedicated chip**; committed PP span reuse in `lx_classify.rs:284-289`; projection `:138-143` | `LX_CLASSIFY_CHIP=5` | `c11_lex.rs:316-323` | Driver real M1 | **PARTIAL**; `CHIP_PLAN.md:122` calls `/11` span reuse “DONE”; no physical/logical/macro origin mapping, no `#line`, no macro chain |
| LX17 TokenPublish | **No dedicated chip**; classify appends in order `lx_classify.rs:279-308`; literal decoders append separately | `LX_CLASSIFY_CHIP=5` / `LX_DECODE_CHIP=6` | `c11_lex.rs:290-325` | Driver M1 | **PARTIAL**; no sole publisher/fan-in; `CHIP_PLAN.md:123` marks “PARTIAL” |
| LX18 LexError | **DRAFT** `lx_error.rs:136-226` pure recovery; `:318-353` compute; `handle` `:303-315` always `Unsupported`; manifest `:278-301` | `ChipId(59)`, LEX local 25 (not frozen, not staged); **not compiled** | In-file tests `lx_error.rs:356-523` are **not compiled/run** | None | **DRAFT**; finite-advance pure core exists, but integrated `handle` is a loud `Unsupported`; no host byte-window pin; no recovery plan carrier |

### 3.5 Cross-cutting behavior matrix

| Requirement | Current behavior | Evidence | Status |
|---|---|---|---|
| Macro definition | PP06 parses and appends `MacroRecord`; only reachable by a manual per-line task | `pp_define.rs:291-583`; `c23_macro.rs:272-339` | **Implemented, not orchestrated** |
| Macro redefinition | PP07 benign-equivalence check (spelling-based) | `pp_redefine.rs:418-492`; `c23_macro.rs:341-397` | **Implemented, not orchestrated** |
| Macro undef | PP08 tombstone append | `pp_undef.rs:159-254`; `c23_macro.rs:397-447` | **Implemented, not orchestrated** |
| Macro expansion | PP09 fan-out + PP12 prescan/`#`/`##`/blue paint | `pp_invoke.rs:426-539`; `pp_substitute.rs:281-353`; `c24_expand.rs:312-483` | **Implemented inside one invocation; no source-level lineage** |
| Variadic macros | PP16 stream + single modes; `__VA_OPT__`; empty tail legal | `pp_variadic.rs:583-719, 1221-1293`; `c26_variadic.rs:318-503` | **Implemented; GNU comma swallowing absent** |
| Conditional inclusion | Stack, `#if/#elif/#else/#endif`, `defined`/`#ifdef` table | `pp_conditional.rs:191-390`; `c22_conditional.rs:205-355`; `c23_macro.rs:447-490` | **Implemented; `#if` macro expansion absent** |
| Include resolution | Committed-source lookup, exact then basename; quote=angle | `pp_resolve.rs:179-256`; `c25_include.rs:284-389` | **Implemented; no host fetch/search path** |
| Include entry | One-level token splice; nested include lines pass through | `pp_enter.rs:261-354`; `c25_include.rs:389-513` | **Implemented; not recursive/once-aware** |
| Line directive | Validates `#line`/GNU marker, returns `LogicalLocation` to caller | `pp_line.rs:243-391`; `c28_line.rs:189-391` | **Ack-only; no carrier** |
| Pragma | Classifies `once`, `pack(push/pop)`, opaque unknown | `pp_pragma.rs:361-499`; `c29_pragma.rs:180-326` | **Ack-only; no carrier/effect** |
| `#error` | PP26 fails with joined message | `pp_diagnostic.rs:145-185`; `c21_directive.rs:279-327` | **Implemented; no span; no `#warning`** |
| Expansion map | Rebuilds chains from committed spans/`ExpansionRecord` | `pp_expand_map.rs:280-356`; `c30_expand_map.rs:216-478` | **Query only; no producer** |
| Preprocessed emit | Strips directives, separates tokens, emits `Preprocessed` artifact | `pp_emit.rs:400-542`; `c31_emit.rs:252-716` | **Implemented; no line markers; primary map only** |
| Integer decode | Decimal digits only; `LiteralRecord` with `Int` | `lx_decode.rs:149-226`; `c11_lex.rs:327-375` | **M1-only; octal/radix/suffix/type gaps** |
| Float decode | Syntax + binary32/64 value check; Ack-only | `lx_float_syntax.rs:238-424`; `lx_float_value.rs:539-630`; `c32_float.rs:210-511` | **Isolated validation; no literal/carrier** |
| Char/string decode | Char and string `LiteralRecord`s; escape validation | `lx_char.rs:408-467`; `lx_string.rs:643-717`; `c33_string.rs:394-547` | **Isolated; not reachable from classify** |
| Adjacent strings | Not merged; PP28 keeps separate tokens | `c31_emit.rs:360-400`; no LX14 file | **Absent** |
| Token locations | PP spans raw; synthesized tokens name span; no expansion records | `pp_scan.rs:179-232`; `pp_substitute.rs:1077-1113`; `pp_expand_map.rs:280-356` | **Physical only; provenance incomplete** |
| Lexer recovery | Scanner/classifier fail whole task; LX18 draft only | `pp_scan.rs:580-597`; `lx_classify.rs:242-257`; `lx_error.rs:303-315` | **Absent in integrated path** |
| Dialect gates | Only PP24 `__STDC_VERSION__` reads dialect; most chips ignore it | `pp_builtin.rs:371-387`; `pp_variadic.rs:35-39`; `lx_char.rs:113-116` | **Mostly absent** |

---

## 4. Draft Files and Collision Assessment

### 4.1 `compiler/src/chips/lex/lx_error.rs` (LX18, draft)

- **Scope:** pure finite-advance recovery (`recover_at`, `split_error_stream`) over a Host-supplied byte window; `compute` Acks a valid offset; `handle` always returns `Unsupported` because the Host pin is unwired (`:278-315`).
- **Registration:** none. `ChipId(59)` (`:45`) and LEX local 25 (`:49`) are unassigned in `manifest.rs`/`task.rs`; no stage row, no allowlist row, no `mod.rs` entry.
- **Tests:** in-file tests (`:356-523`) include a 4 KiB pseudorandom sweep and exhaustive 2-byte alphabet; they are **not compiled** because `lex/mod.rs` does not declare the module. The concurrent wiring guard parks it (`c42_wiring.rs:27-32`).
- **Assessment:** the pure core is useful, but the chip is not an integration candidate until (a) the Host byte-window/task pin is frozen, (b) a recovery-plan carrier or consumer exists, and (c) ChipId/local/stage/allowlist are allocated. It cannot currently improve lexer recovery in the driver.

### 4.2 `compiler/src/chips/lex/lx_extension.rs` (LX15, draft)

- **Scope:** pure extension-suffix classification (`classify_extension_suffix`, `:93-142`) and a chip that Acks GNU imaginary spellings in GNU mode, rejects them in strict ISO, and rejects unknown suffixes (`:252-294`). Ack-only; no `LiteralRecord`.
- **Registration:** none. `ChipId(58)` (`:39`) and LEX local 24 (`:43`) are unassigned; no stage/allowlist/`mod.rs`.
- **Tests:** in-file tests (`:302-432`) cover GNU imaginary, identifier protection, plain numbers, unknown suffixes, determinism; **not compiled/run**.
- **Assessment:** useful classification core, but the file is not a full LX15 implementation: no imaginary literal value/type, no suffix combinations, no `TokenKind`/`LiteralRecord` carrier, and no dispatch gate. It also overlaps LX06/LX09: `1F`, `1.5ll`, `0x1p+2u` are classified as `UnknownSuffix` even though some are standard float/integer-suffix shapes owned by other chips. Routing all pp-numbers to LX15 would misclassify valid forms.

### 4.3 Collision matrix

| Resource | Claimants | Existing/frozen allocation | Risk |
|---|---|---|---|
| `ChipId(58)` | `lx_extension.rs:39`; `pa_cast.rs:158`; `pa_decl_spec.rs:131`; `pa_expr_ext.rs:152`; `pa_ext_dispatch.rs:121`; `pa_init.rs:122`; `pa_stmt.rs:135`; `pa_typedef_query.rs:129` | No manifest constant; next free after `SE_FUNC_CHIP=57` | **Direct collision among 8 drafts.** Only the integrator can assign 58; lx_extension must not assume it. |
| `ChipId(59)` | `lx_error.rs:45` only | No manifest constant | Currently unique, but still a “next free” guess; integrator must allocate. |
| LEX local 24 | `lx_extension.rs:43` | Free; tests assert `stage_of(LEX local 24) == None` (`c32_float.rs:181`, `c33_string.rs:247`) | If LX14/another chip takes 24, collision; adding LX15 requires changing frozen test assertions (integrator-owned). |
| LEX local 25 | `lx_error.rs:49` | Free; tests do not reserve it | If LX14/LX15 take 25, collision; local numbering must be assigned as a block. |
| `core::chips::scan` paste validator | PP12 uses `crate::chips::scan` (`pp_substitute.rs:762`) | Full PP04 scanner (`pp_scan.rs:466`) | Paste validation inherits scanner `Unsupported` policy: a paste producing a non-ASCII/UCN token fails; may be correct fail-closed but is a hidden coupling. |
| Directive predicate copies | PP05, PP09, PP16, PP18, PP19 each copy `group_raw_lines`/walk-back/`line_index` | No shared helper by design | Drift risk: a fix to one copy (e.g. leading comments before `#`) will not reach the others. |
| Include-prefix copies | PP03 (`pp_comment.rs:293-388`) and PP04 (`pp_scan.rs:281-374`) | Shared predicate is copied | `c20_ppscan.rs:291-391` drift test covers a subset; no generic agreement test. |
| Float parts shapes | LX09 `FloatParts` (`lx_float_syntax.rs:69-79`) and LX10 `FloatParts` (`lx_float_value.rs:69-83`) | Deliberate local copies | Divergence risk; `c32_float.rs:394-441` pins the current agreement. |
| Escape decoders | LX11 (`lx_escape.rs:225-348`), LX12 (`lx_char.rs:137-264`), LX13 (`lx_string.rs:330-480`) | Deliberate local copies | Known behavioral deltas documented in `lx_escape.rs:54-67`; convergence is a T04/T01 co-freeze item. |

### 4.4 Other draft/duplication observations

- The two lex drafts are explicitly listed as unwired in the concurrent `c42_wiring.rs:27-32`; any Stage-B integration must remove them from that list in the same integrator-owned change.
- No other untracked lex file exists. LX14 has no draft; LX16/LX17 have no dedicated file; their catalog claims are satisfied only by classify-side behavior.
- The concurrent parse drafts use `ChipId(58)` seven times; this is the largest ID-allocation hazard for `lx_extension.rs`. The Stage-B integrator must allocate a single monotonic chip-ID block, not “next free” guesses.

---

## 5. Gaps and Risks Ranked by Dependency

### P0 — blocks real preprocessing/lexing

1. **No source-level preprocessing orchestration.** The driver (`candidate.rs:220-289, 572-680`) registers only PP01–PP05, PP09, PP12, PP19, PP26, PP28 and LX intern/classify/decode. PP06/PP07/PP08/PP16/PP17/PP18/PP23/PP24/PP25/PP27 are not registered/routed; PP05 still fails `#define`, `#undef`, `#include`, `#line`, `#pragma` as `Unsupported` (`pp_directive.rs:541-581`). Probe: `#define X 2` and `#include "h.h"` exit 1 at PP05. This blocks headers, macros, line control, pragmas, and real `-E`.
2. **`#if` does not macro-expand.** T03:5 and PP21 require “first macro-processed, then evaluated”; `PpConditional` only consults the macro table for `defined`/`#ifdef`/`#ifndef` (`pp_conditional.rs:888-921`), and plain identifiers become `0` (`:869-872`). Probe `-DX=1` + `#if X` took the false branch. This can silently choose wrong code, not just reject.
3. **Lexer integration is M1-integer-only.** `LxClassifyChip` rejects non-decimal pp-numbers and all string/char/header tokens (`lx_classify.rs:238-258`); `TokenKind` has no Character/String/Float (`bus.rs:246-257`; `lx_classify.rs:230-260`). Probes: float and adjacent-string sources exit 1. LX09–LX13 are isolated tasks with no PP→LX dispatch.
4. **Integer literal semantics are incomplete and one case is wrong.** `010` is accepted by classify (all digits) and decoded by `decimal_magnitude` (`lx_decode.rs:167-175, 208-226`), yielding 10 instead of octal 8. No radix (`0b`, octal, hex), suffix (`U/L/UL/LL/ULL`), candidate type beyond `Int`, target width, or range diagnostics. Downstream T08 fold rejects anything except decimal/`None`/`Int` (`fold.rs:252-258`; `fold_branch.rs:499-505`; `c37_const_branch.rs:113-124, 176-183`).

### P1 — blocks large parts of C / diagnostics

5. **Expansion provenance has no producer.** PP12/PP16 synthesize tokens with the invocation-name span (`pp_substitute.rs:1077-1113`; `pp_variadic.rs` materialize), no chip writes `ExpansionRecord`, and `G1DraftBody` has no Expansion variant (`records.rs:55-95`). PP27 can therefore only consume fixture records; it cannot trace real macros, `#`/`##`, nested includes, or blue paint.
6. **Location, line, and pragma carriers are placeholders.** PP23 Acks and returns `LogicalLocation` only (`pp_line.rs:243-270`); PP24 `__LINE__`/`__FILE__` use physical source/line (`pp_builtin.rs:304-340, 438-486`), `__COUNTER__` is always 0 (`:318-324`), and date/time are unsupported; PP25 classifies `once`/`pack` but persists nothing (`pp_pragma.rs:453-499`). LX16 has no chip.
7. **Adjacent strings are absent (LX14).** No file/task/kind; PP28 preserves separate string tokens (`c31_emit.rs:360-400`), and classify rejects them. This blocks a very common C construct and torture cases.
8. **Include semantics are single-level.** PP18 splices one resolved header and passes nested includes through (`pp_enter.rs:261-354`); no include stack, depth limit, `#pragma once`, `include_next`, search paths, or Host fetch. `-I` is explicitly deferred (`candidate.rs:140-141`). This blocks real headers and transitively many torture tests.
8b. **Include-prefix recognition disagrees with directive recognition on a leading comment.** PP03/PP04 only accept `[ \t]*#` before `include` (`pp_comment.rs:301-314`, `pp_scan.rs:293-307`), while PP05's walk-back accepts a same-line `/*...*/` before `#` (`pp_directive.rs:473-517`). For `/*c*/ #include <a//b>`, PP03 can treat `//b>` as a comment before PP05 sees the directive; that is a real header-corruption defect, not just a deferred feature.
9. **Lexer recovery is absent.** The scanner/classifier fail the whole task on the first bad byte/token (`pp_scan.rs:580-597`; `lx_classify.rs:242-257`); LX18 is a draft with an always-`Unsupported` `handle` (`lx_error.rs:303-315`). This impairs diagnostic robustness and fuzz/negative cases.

### P2 — correctness/conformance/contract

10. **PP integer model is not the C PP model.** i128 exact arithmetic, no signed/unsigned conversions, always-evaluated `&&`/`||` (T03 PP21 says short-circuit), and `shift_left` uses `checked_shl`, which checks only the shift amount, not value overflow (`pp_conditional.rs:966-980`). Target-width PP semantics are explicitly deferred in the frozen slice.
11. **GNU/dialect gates are mostly absent.** No trigraphs (`pp_normalize.rs`); no GNU `$` identifiers/UCN (`pp_scan.rs:580-594`); GNU comma swallowing is not implemented and not rejected: an empty tail retains the comma, while a non-empty tail generally fails as a multi-token paste (`pp_variadic.rs:35-39, 2127-2177`); LX03 has no GNU keywords; LX12 rejects `u8'` (`lx_char.rs:113-116`); LX13 accepts `u8"` without dialect gating (`lx_string.rs:191-204`); LX15 is uncompiled.
12. **Float semantics stop at binary32/binary64.** LX10 has no binary128/x87 and no float `LiteralRecord`/`FloatBits` carrier (`lx_float_value.rs:102-108, 219-247`); long double is explicit `Unsupported`; only unit-test bits, no integrated consumer.
13. **PP diagnostics are thin.** `#warning` has no wire and is `Unsupported` (`pp_directive.rs:541-547`); PP26 diagnostics carry no span (`pp_diagnostic.rs:178-184`); malformed directives generally fail the whole task rather than produce recoverable diagnostics.
14. **PP28 directive detection is latent-unsafe without expansion records.** It treats a `#`/`%:` with `span.expansion.is_none()` as a directive (`pp_emit.rs:440-473`). PP12-synthesized `#` tokens have no expansion link, so a macro-generated `#` at line start could be stripped as a directive. This is a static risk; no test covers it.
15. **Duplicated predicates/helpers drift.** PP05/PP09/PP16/PP18/PP19 copy the directive walk-back/grouping; PP03/PP04 copy the include-prefix scanner; LX11/12/13 copy escape decoders. This is a maintenance hazard and a future contract-consistency bug source.
16. **Draft ID/local collisions.** `lx_extension` (58) collides with seven parse drafts; `lx_error` (59) and locals 24/25 are not integrator-allocated. See §4.3.

---

## 6. Contract vs Implementation Conflicts and Unverified Claims

### 6.1 Conflicts requiring an integrator decision

| Contract text | Implementation | Conflict |
|---|---|---|
| `T03:5`: PP expressions are “first macro-processed, then evaluated” | `pp_conditional.rs:869-872` maps every identifier to 0; no macro expansion | **Semantic conflict.** Either PP19 must expand macros in `#if`/`#elif`, or T03 must explicitly narrow the rule (and the corpus impact must be accepted). |
| T03 PP21: “logical short-circuit” | `pp_conditional.rs:686-702` evaluates both sides; comment says fail-closed | **Conflict.** Decide whether `#if 0 && 1/0` must succeed (short-circuit) or fail closed. |
| T03 PP22: PP maximum integer domain, signed/unsigned conversion/overflow policy | i128 exact arithmetic; no signed/unsigned model; shift overflow not detected | **Conflict/deferral.** The frozen `/22` slice defers target width; the T03 row remains broader. |
| T03 rev 55 §3: scan state persists across chunks/ticks | `/20` freeze explicitly does not adopt chunk persistence; PP03/PP04 are whole-input | **Superseded/not adopted**, but T03 rev-55 candidate text still reads as planned contract. |
| T03 PP16: GNU comma swallowing and mode rejection | Empty tail retains the comma; non-empty tail generally fails as a multi-token paste (`pp_variadic.rs:2127-2177`); no dialect read | **Unimplemented and not rejected.** Either implement dialect-gated behavior or make GNU use explicit `Unsupported`. |
| T03 PP28 / `pp_emit.rs` comment: macro-generated `#` is never an introducer | No producer of `ExpansionRecord`; synthesized `#` has `span.expansion=None` | **Latent conflict.** The discriminator cannot work with the current producer set. |
| T04 `LiteralRecord` radix domain `{2,8,10,16}` | LX13 writes `radix: 0` for strings; LX12 writes `16` for chars | **Schema extension unresolved.** `radix:0` is an explicit marker, not a base; T08 must not consume it. |
| T04 `Lx08CandidateType` M1 `{Int}`; non-M1 forms must not silently default to `Int` | LX12/LX13 write `candidate_type: Int` as an explicit placeholder | **Temporary, documented**; downstream must not treat char/string records as typed. Needs co-freeze. |

### 6.2 Unverified claims (not counted as capabilities)

- `CHIP_PLAN.md:122-123` marks LX16 “DONE” and LX17 “PARTIAL”; there is no dedicated chip for either. The only evidence is committed-span reuse/ordered classify append (`lx_classify.rs:279-323`).
- Slice execution records claim “full C11 punctuator table”, “correct rounding”, “all five origin cases”, and “target macros”; tests cover subsets and unit-pinned patterns, not universal correctness.
- PP24 “replayable” `__DATE__`/`__TIME__` is not implemented; it is an explicit `Unsupported`.
- PP27’s five origin cases are tested with synthetic `ExpansionRecord` fixtures (`c30_expand_map.rs:217-456`), not with records produced by PP12/PP16.
- PP28’s “re-lex guarantee” is tested over its local corpus; it is not a proof for every pp-token spelling or for macro-generated `#`.
- T03/T04 prose still contains `/5`/`/6` status statements; `/39` is the authoritative frozen marker, but the total T03/T04 contracts are not signed off here.

---

## 7. Recommended Stage-B Work Orders

All work orders are **proposals**, not authorization. Any change to `TokenKind`, `RecordFamily`, `G1DraftBody`, `TaskKind`, chip IDs, stage rows, allowlists, or contract hash is **integrator-owned** and must start as `NEEDS_FREEZE`/`BLOCKED_CONTRACT`, not as a chip-local edit. Each work order owns only the paths listed; shared paths are forbidden.

| ID / priority | Scope | Owned implementation paths | Owned test path (proposed) | Depends on | Exit criterion |
|---|---|---|---|---|---|
| **B-PPL-01** / P0 | Host preprocessing orchestration: scan lines, fan out PP06/PP08 before PP09, route PP23/PP25, invoke PP17/PP18 with a Host header loader, keep PP05 for `#error`; register existing PP/LX chips in the driver | `compiler/src/bin/candidate/pp_pipeline.rs` (new); `compiler/src/bin/candidate/pp_include.rs` (new, if split). `candidate.rs` wiring is **integrator-owned** | `compiler/tests/c50_pp_pipeline.rs` (new; integrator adds entry) | `/39`; B-PPL-02 for correct `#if` | Real source with `#define`, `#undef`, `#include`, `#line`, `#pragma` reaches the corresponding frozen chip; `-E` emits its expansion; all failures are typed |
| **B-PPL-02** / P0 | Conditional expression semantics: macro-expand `#if`/`#elif` operands; decide/implement short-circuit; PP `intmax_t`/`uintmax_t` model or explicit narrowing; fix shift-overflow check | `compiler/src/chips/preprocess/pp_conditional.rs` | `compiler/tests/c51_conditional_expr.rs` (new) | `/22` slice + T03/T01 ruling on short-circuit/PP-int | `#if X` honors `-D X=1`/source `#define X`; operator matrix, signed/unsigned boundaries, overflow, and short-circuit cases pinned |
| **B-PPL-03** / P0 | Integer literals LX05–LX08: radix `2/8/10/16`, suffix parsing, big-magnitude decode, candidate type/range diagnostics; fix leading-zero octal | `compiler/src/chips/lex/lx_decode.rs`; optionally split new `lx_integer_radix.rs`, `lx_integer_suffix.rs`, `lx_integer_type.rs` (one owner per file) | `compiler/tests/c52_integer_literals.rs` (new) | `/11` + T04/T08 candidate-type co-freeze; T08 fold gate update is integrator/T08-owned | Exact values and candidates for `010`, `0x10`, `0b101`, `1U`, `1LL`, `ULL`, boundary `2147483648`/`0xffffffff`; ranges fail typed, not wrap |
| **B-PPL-04** / P0 | Literal token/dispatch integration: extend `TokenKind` (plus any co-frozen `TokenFlags`) or route literal pp-tokens; make classify emit or hand off float/char/string tokens; add LX14 adjacent-string merge | `compiler/src/chips/lex/lx_classify.rs`; new `compiler/src/chips/lex/lx_adjacent.rs` (`LX14_TASK_KIND` etc.); contract `bus.rs`/`task.rs`/`manifest.rs` changes are integrator-owned | `compiler/tests/c53_literal_dispatch.rs` (new) | B-PPL-03 for integer; T04/T01 co-freeze for TokenKind/LX14 carrier | PP04 output with `"a" "b"`, `L"x"`, `'a'`, `2.5`, `2` flows through LX with typed tokens/literals; adjacent merge produces one literal/one terminator |
| **B-PPL-05** / P1 | Expansion provenance producer/consumer: add `ExpansionDraft` materialization (or approved alternative), write `ExpansionRecord` from PP12/PP16, consume in PP27; preserve raw/prescan/paste/stringify spans | Contract: `compiler/src/records.rs`, `compiler/src/commit.rs`, `compiler/src/bus.rs` (integrator); chips: `pp_substitute.rs`, `pp_variadic.rs`, `pp_expand_map.rs` | `compiler/tests/c54_expansion_provenance.rs` (new) | T01 materialization/ref freeze; ADR-0002 if pipeline semantics change | PP12/PP16 real expansion produces durable expansion frames; PP27 rebuilds `#`/`##`/prescan/blue-paint chains from them; no fixture records needed |
| **B-PPL-06** / P1 | Logical location + pragma carriers: persist PP23 `LogicalLocation`, make PP24 read logical `__LINE__`/`__FILE__`, persist PP25 `once`/`pack`, consult once in PP17/PP18; give LX16 a real location chip if the carrier is frozen | `pp_line.rs`, `pp_builtin.rs`, `pp_pragma.rs`, `pp_resolve.rs`, `pp_enter.rs`; new `lx_location.rs` only if contract owner approves | `compiler/tests/c55_line_pragma_carriers.rs` (new) | T01 store/record freeze; PP23/PP25 catalog | `#line` changes `__LINE__`/`__FILE__`; `#pragma once` suppresses repeat include; pack state is available to layout; diagnostics carry logical spans |
| **B-PPL-07** / P1 | Include pipeline: recursive include stack/depth, `#pragma once`, `include_next` policy, Host file loading/search paths, one-task-per-include orchestration | `compiler/src/bin/candidate/pp_include.rs` (new Host); `pp_resolve.rs`, `pp_enter.rs`; `candidate.rs` wiring integrator-owned | `compiler/tests/c56_include_pipeline.rs` (new) | B-PPL-01; T02 AwaitHost/Host contract; parser/driver include policy | Nested includes resolve recursively with depth limit and once; missing/ambiguous headers typed; `-I` path policy explicit |
| **B-PPL-08** / P1 | Float literal carrier/dispatch: publish `FloatBits`/float `LiteralRecord`, connect classify/PP04 to LX09/LX10, add `L`/binary128 policy and long-double target model | `lx_float_syntax.rs`, `lx_float_value.rs`, `lx_classify.rs` (routing); contract carrier integrator-owned | `compiler/tests/c57_float_literals.rs` (new) | B-PPL-04; T04/T08/T11 target co-freeze; probe for `long double` | Float literals from real source reach a durable literal record; f32/f64 exact; binary128 explicitly unsupported or probe-gated |
| **B-PPL-09** / P2 | Register and complete the two lex drafts: allocate IDs/locals/stage/allowlist; LX15 produce a real extension literal record; LX18 host byte-window pin + recovery plan carrier | `lx_extension.rs`, `lx_error.rs`; `compiler/tests/c58_lex_extension_recovery.rs` (new); shared registration integrator-owned | c58 | B-PPL-04; T04/T01 co-freeze; unique chip-ID block | LX15/LX18 manifests register, are routed, and their tests run; recovery advances finitely on arbitrary bytes; no duplicate ID with parse drafts |
| **B-PPL-10** / P2 | Dialect gates: trigraphs (PP01), GNU identifiers/UCN, GNU comma swallowing, GNU keywords, `u8` char/string gates, C23 separators, dialect test matrix | `pp_normalize.rs`, `pp_scan.rs`, `pp_conditional.rs`, `pp_variadic.rs`, `lx_classify.rs`, `lx_char.rs`, `lx_string.rs` | `compiler/tests/c59_dialect_gates.rs` (new) | B-PPL-02/03/04; `Dialect` model frozen | Each gate is either implemented or explicit `Unsupported`; no silent acceptance/rejection in the wrong mode |
| **B-PPL-11** / P2 | PP diagnostics/recovery: `#warning` wire or explicit same-tick diagnostic, spans on PP26 errors, per-line recovery rather than whole-task failure | `pp_diagnostic.rs`, `pp_directive.rs`, new `pp_recovery.rs` if needed | `compiler/tests/c60_pp_diagnostics.rs` (new) | T01 diagnostic wire freeze | `#error`/`#warning` carry spans and structured codes; malformed lines recover finitely; no panic |
| **B-PPL-12** / P2 | Independent conformance tests: PP03 rev-55 matrix subset, PP/USE cases, literal decoding vs hand-derived oracle, chunk-invariance policy | Test-only: `compiler/tests/c61_pp_lex_conformance.rs`; fixtures under `compiler/tests/fixtures/pp_lex/` | self | B-PPL-01..10; no reference-compiler-as-oracle for candidate output | Each planned PP03-T row and literal edge is either tested or explicitly marked unsupported; expected values are hand-derived/spec-derived |

**Forbidden shared paths for every work order:** `compiler/src/chips/{preprocess,lex}/mod.rs`, `compiler/src/chips/mod.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/src/routing.rs`, `compiler/src/bus.rs`/`records.rs`/`commit.rs` (unless the work order explicitly names the contract owner), `compiler/src/bin/candidate.rs`, `compiler/Cargo.toml`, `compiler/contracts/CONTRACT_VERSION`, and `compiler/tests/freeze.rs`. The integrator wires modules, registrations, IDs, stages, allowlists, and test entry points.

---

## 8. Unrun Checks and Open Questions

### 8.1 Checks not run by this audit

- Full workspace test suite, full clippy, `cargo fmt --check`, chip-lint, doc link/count validators, contract validators.
- `c37_const_branch` and concurrent `c41`/`c42` tests (c37 was read only for the T08 literal gate; the others were not executed).
- `lx_error.rs`/`lx_extension.rs` compilation and in-file tests (modules are not declared; ordinary `cargo test` cannot see them).
- Any Linux/AArch64 probe, target compilation, assembler/linker, or execution; no `-S`/`-c` acceptance.
- GCC torture suite, `-E` differential against GCC, official DejaGnu driver, corpus lock, or pass-rate measurement.
- Large integer > u64/int128 decode, octal/radix/suffix matrix, binary128/long-double conversion, adjacent-string matrix, recursive include matrix, `#pragma once`/pack effects, `#warning`, chunk invariance, or dialect matrix.
- Full PP03 rev-55 `PP03-T01..T20` matrix; only the subset in `c16`/`c20` is covered. Chunk persistence is not adopted by the frozen `/20` slice.
- Full macro corpus (mutual recursion, nested variadic inside non-variadic args, GNU comma swallowing, placemarker edge cases); only the c24/c26 fixtures.
- Source-level `#define`/`#include`/`#line`/`#pragma` end-to-end except the negative candidate probes above.

### 8.2 Open questions for the integrator/contract owners

1. Is PP19’s lack of `#if` macro expansion a deliberate narrowing of T03 PP21, or a defect to fix? The current behavior silently takes the false branch for `#if MACRO`.
2. Must `#if 0 && 1/0` short-circuit per T03 PP21, or fail closed per the frozen `/22` implementation?
3. What is the required PP integer model (`intmax_t`/`uintmax_t`, signedness, shift/overflow)? Is the i128 model a permanent implementation choice or a temporary slice?
4. Who owns `ExpansionRecord` materialization? `G1DraftBody` has no Expansion variant and no chip appends one, so PP27’s frozen rule is currently unrealizable from real expansion.
5. When do PP23/PP25 get durable carriers, and who consumes `once`/`pack`? Is a `LogicalLocation` store a T03/T01 co-freeze item?
6. What is the `TokenKind` plan for Character/String/Float (new variants versus routing by PpToken kind)? What are the numeric encodings and `TokenFlags`?
7. Is `radix: 0` for strings a permanent domain extension or an explicit marker that T08 must reject? Same for `candidate_type: Int` placeholders on char/string records.
8. Who allocates `ChipId(58)`/`59` and LEX locals 24/25 given the seven parse drafts on 58? What is the monotonic allocation rule?
9. How should PP28 distinguish a macro-generated `#` from a directive if no `ExpansionRecord` producer exists? Is a token flag/`TokenFlags` field the intended carrier?
10. Who fetches headers (T02 `AwaitHost`/CT02), what is the quote/angle search path, and is `include_next` T12 or T03?
11. Is rev-55 chunk invariance required for large sources, or is whole-input scanning with `max_source_bytes` the accepted model?
12. Should GNU `, ## __VA_ARGS__` be implemented dialect-gated, rejected explicitly in GNU mode, or accepted with a documented wrong-output limitation?
13. What are the target `long double` and `wchar_t` models for float/string literal records, and when does the Linux probe unlock them?
14. Should `__COUNTER__`/`__DATE__`/`__TIME__` remain explicit unsupported until frozen carriers exist, or be host-seeded per compilation?
15. What diagnostic span/code shape should PP26 `#error` and PP05 malformed directives carry? Is `#warning` a T03 or T13 responsibility?

---

## 9. Bottom Line

The repository contains a substantial, tested **frozen slice** of T03/T04: source normalization/splice/comment/token scan, directive/conditional/macro/include/line/pragma/builtin/emit workers, and integer/float/char/string decoding workers. That is not the same as a working preprocessor/lexer:

- Source macros and includes are not orchestrated and still fail in the driver.
- `#if` does not expand macros and can select the wrong branch.
- The integrated lexer only accepts M1 decimal integers; strings, chars, floats, and adjacent strings fail.
- Integer octal is silently decoded as decimal; radix/suffix/type/range are missing.
- Provenance, logical locations, pragma effects, and lexer recovery are placeholders or drafts.

The smallest critical-path sequence is **B-PPL-01 (orchestration) → B-PPL-02 (`#if` semantics) → B-PPL-03 (integer literals) → B-PPL-04 (literal dispatch/LX14)**, with **B-PPL-05/06/07** required before diagnostics, `#line`, pragma, and real headers can be trusted.
