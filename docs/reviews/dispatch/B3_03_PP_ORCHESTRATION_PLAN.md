# B3-03 — Candidate Driver PP Orchestration Plan

| Field | Value |
|---|---|
| Status | **READ-ONLY DESIGN / WORK-ORDER REPORT** — no implementation, no contract freeze, no schema change, no source/test edit |
| Date | 2026-11-09 |
| Task | B3-03 `#define` / `#undef` / `#include` / `#line` / `#pragma` orchestration in `compiler/src/bin/candidate.rs` |
| Authority | [AGENTS.md](../../../AGENTS.md); [DISPATCH_LEDGER.md](../../tasks/DISPATCH_LEDGER.md) D-19/B3-03/§5; [H04_CANDIDATE_DRIVER.md](../../tasks/H04_CANDIDATE_DRIVER.md); [T03_PREPROCESS_CHIPS.md](../../tasks/T03_PREPROCESS_CHIPS.md); [T02_CONTROL_CHIPS.md](../../tasks/T02_CONTROL_CHIPS.md); [B2-00 CCR](../../tasks/CONTRACT_CHANGE_REQUEST_B2_DETERMINISM_LIMITS.md) |
| Baseline observed | Branch `wave4-development`, HEAD `316a3d4`; the review began on `/41` and ended after the uncommitted `/42` B2-02 integration. The ledger now records `/42` (hash `a9a1da92…abf1a5`) as accepted/implemented/verified, but the worktree changes are uncommitted. |
| Write scope | **This file only.** No source, tests, contracts, ledger, T01, or other file was modified. |
| Non-claims | No full M1 acceptance, no target/ABI/probe result, no GCC torture rate, no target execution, no full preprocessor claim, no new test/probe capability. Prior test counts are cited as recorded evidence unless §11 says this session re-ran them. |

**Integrator numbering note (2026-11-09).** `c45` is already used by the frozen `/42` tagged-constant acceptance (`compiler/tests/c45_const_ty.rs`). Treat this report's proposed `c45_pp_driver.rs` as **`c46_pp_driver.rs`**, its `c46_pp_include_driver.rs` as **`c47_pp_include_driver.rs`**, and row labels `c45`/`c46` as `c46`/`c47` respectively. No design decision changes.

**Post-report implementation (2026-11-09).** W1/B3-03a is implemented and verified as a host-only slice: source `#define`/`#undef` are processed through PP06/PP07/PP08 with ordered text segments, a single PP19 conditional boundary, directive removal, and a continuation guard. Evidence: `compiler/tests/c46_pp_driver.rs` 10/10; see [PP_DRIVER_SLICE.md](../../tasks/PP_DRIVER_SLICE.md). W2/B3-03b is implemented as a **bounded subset only**: quoted headers beside the including file, canonical sandboxing, PP17/PP18 per-line splice, header `#define`/`#undef` and text; angle/system/`-I`, nested includes, guards/`once`, and after-conditional includes remain refused. Evidence: `c47_pp_include_driver` 12/12; policy: [B3_03B_LEAF_INCLUDE_POLICY.md](../../tasks/B3_03B_LEAF_INCLUDE_POLICY.md). W3 line/pragma remains open. This note does not freeze the wider H04 seam.

## 0. Reading conventions

| Tag | Meaning |
|---|---|
| **[IMPL]** | Implemented behavior observed in source code (and, where stated, in a test run during this review). |
| **[FROZEN]** | A frozen contract/slice rule recorded in a task document or T01. Frozen does **not** mean installed or end-to-end. |
| **[PROPOSED]** | Work order / design proposal in this report. Not authorized, not frozen. |
| **[UNVERIFIED]** | Reasoned from source or cited from prior documents but not executed in this review, or dependent on an unstable baseline. |

The report deliberately separates "the chip exists and passes its slice tests" from "the candidate driver can drive it". Most of B3-03 is the second gap.

## 1. Bottom line

1. **The PP worker layer is substantially present; the driver orchestration is not.** PP06/PP07/PP08 (macro definitions), PP17/PP18 (include), PP23 (line), and PP25 (pragma) exist and have frozen-slice tests, but the candidate driver registers/routes none of them (`compiler/src/bin/candidate.rs:223-294`, `:387-474`). PP05 still rejects every non-`#error` directive (`compiler/src/chips/preprocess/pp_directive.rs:541-581`). This is D-19.
2. **`#define`/`#undef` are the closest win, but a simple "register the chips" change is unsafe.** The driver runs one fixed PP sequence: PP01→PP02→PP03→PP04→PP19→PP09→PP05 (`candidate.rs:581-689`). A source `#define` therefore cannot affect a later `#if` (PP19 already ran) or a later use (PP09 already ran), and the accepted directive line would still be handed to the lexer because PP05 does not remove it (`candidate.rs:678-688`, `:699-717`).
3. **`#include` is blocked twice.** H04 include policy is a **DRAFT ONLY** document ([H04_I_FREEZE.md](../../tasks/H04_I_FREEZE.md):3-11), and frozen PP18 has a single-header-per-task contract: it fails if any include line in the payload resolves to a different header (`compiler/src/chips/preprocess/pp_enter.rs:324-330`). A file with two distinct headers cannot be processed by one PP18 call.
4. **`#line` and `#pragma` cannot be completed by orchestration alone.** PP23 and PP25 are validation/classification workers that complete `Ack` and write nothing (`pp_line.rs:215-243`; `pp_pragma.rs:28-47`, `:476-500`). There is no logical-location carrier, no `PragmaRecord`, no once-set, and no pack stack. Implementing their effects requires a separate T01/T03 record/store freeze — explicitly outside this work order.
5. **Do not implement on a mixed mid-integration tree.** During this review the tree moved from `/41` to the uncommitted `/42` B2-02 integration: `compiler/src/contract.rs` and `compiler/contracts/CONTRACT_VERSION` changed underneath the review; an intermediate `h04_candidate` run failed one version assertion before the `/42` test update landed, while the PP chip suites re-run in this review passed. The ledger now records `/42` as accepted/implemented/verified, so B3-03 must start from a clean committed `/42` (or `/41`) baseline, not from a worktree that still mixes B2-02 edits.

## 2. Current driver behavior

### 2.1 Command surface and limits

| Area | Observed behavior | Source |
|---|---|---|
| Accepted flags | `-std=c11`, `-O0`, `-D`, `-U`, `-E`, `-o`, `--emit-ir-snapshot`, `--emit-trace`, `--interpret-ir` | `candidate.rs:91-215` |
| `-D` | Bare `-D NAME` defines `1`; `-D NAME=value` scans `value`; empty value stays defined; last `-D`/`-U` for a name wins by commit order | `candidate.rs:145-181`, `:488-572`; `macro_flags.rs:78-141` |
| `-U` | Commits an `undefined: true` tombstone | `candidate.rs:560-568`; `macro_flags.rs:118-127` |
| `-E` | Runs the shared PP prefix, then PP28; PP28 strips consumed directive lines and emits no line markers | `candidate.rs:694-697`; `emit_helper.rs:38-62`; `pp_emit.rs:24-28`, `:391-395` |
| `-I`, `-I<dir>` | Explicitly refused with exit 2 ("deferred") | `candidate.rs:143-144`; [H04_CANDIDATE_DRIVER.md](../../tasks/H04_CANDIDATE_DRIVER.md):32 |
| `-S`, `-c` | Explicitly refused with exit 2 (Part B, target unverified) | `candidate.rs:107-110` |
| Multi-source | Exactly one positional source; `N != 1` exits 2 | `candidate.rs:195-201` |
| `-std`/`-O` propagation | Validated but **not stored**; config hardcodes `Dialect::C11`, `OptLevel::O0` | `candidate.rs:65-78`, `:119-142`, `:586-592`; A10 F-02 |
| Exit codes | `0` evidence/emit; `1` candidate diagnostic; `2` driver/usage/refusal/non-UTF-8 | `candidate.rs:925-1016` |

Host read failures are currently mapped to the pipeline error path (exit 1), not cleanly separated from source diagnostics; A10 F-04 records this as an unresolved H04 taxonomy issue. B3-03 must not invent a new exit-code taxonomy without the H04 freeze.

### 2.2 Installed PP workers and routes

`install()` registers these PP workers: `PpNormalizeChip`, `PpSpliceChip`, `PpCommentChip`, `PpScanChip`, `PpConditionalChip`, `PpInvokeChip`, `PpSubstituteChip`, `PpDirectiveChip`, `PpDiagnosticChip`, `PpEmitChip` (`candidate.rs:223-251`). It does **not** register PP06/PP07/PP08/PP16/PP17/PP18/PP23/PP24/PP25/PP27.

The route table covers `PREPROCESS_NORMALIZE`, `SPLICE`, `COMMENT`, `SCAN`, `CONDITIONAL`, `MACRO_INVOKE`, `MACRO_SUBSTITUTE`, `DIRECTIVE`, `DIAGNOSTIC`, `EMIT` only (`candidate.rs:256-294`). `build_workers()` has the same subset (`candidate.rs:387-474`). Consequently, even a manually enqueued `PREPROCESS_MACRO_DEFINE` task would have no registered worker route in the candidate binary.

### 2.3 Fixed pipeline order

`pp_prefix()` runs:

```text
PP01 normalize -> PP02 splice -> PP03 comment -> PP04 scan
-> PP19 conditional -> PP09 macro invoke -> PP05 directive dispatch
```

(candidate.rs:581-689; the PP19 call is at `:653-663`, PP09 at `:667-677`, PP05 at `:680-687`).

This order is correct for the directive-free M1 source and for `-D`/`-U` seeds, because the seeds are committed before PP19/PP09. It is **not** correct for source directives:

- source `#define X 2` has not been processed when PP09 expands a later `X`;
- source `#define X 2` has not been processed when PP19 evaluates a later `#if X`;
- source `#undef X` cannot affect a later use;
- the `#define` line itself remains in the returned token stream.

### 2.4 Directive tokens leak into the evidence path

PP05 is called and its result is discarded; `pp_prefix` returns `expanded_tokens` unchanged (`candidate.rs:678-688`). Those tokens are then passed to `LX_INTERN`/`LX_CLASSIFY` and parsing (`candidate.rs:699-747`). PP09 explicitly passes directive lines through verbatim (`pp_invoke.rs:11-14`, `:658-671`), and PP05 itself only completes `Ack` or fails; it has no filter proposal (`pp_directive.rs:234-249`).

The `-E` path hides this because PP28 strips directive lines (`pp_emit.rs:24-28`, `:391-395`). The evidence path does not. A source `#define` accepted by a future PP05 must therefore be removed from the token stream before lexing, or the parser will see `#`, `define`, etc.

### 2.5 Observed directive probes

With the current `compiler/target/debug/candidate` binary, all four directives fail at PP05:

| Probe source | Exit | Message (abridged) |
|---|---|---|
| `#define X 2` + `int main(void){return X;}` | 1 | unsupported directive `define`: macros are deferred to the macro chips (PP06 definition, PP09-PP16 expansion) |
| `#include "h.h"` + `main` | 1 | unsupported directive `include`: header inclusion is deferred to PP17/PP18 |
| `#line 5 "x.c"` + `main` | 1 | deferred to PP23 |
| `#pragma once` + `main` | 1 | deferred to PP25 |
| `-E` on the `#define` source | 1 | same PP05 `define` message |

This matches source `pp_directive.rs:541-581` and A03 §2.2. It is not evidence that PP06/PP08/PP17/PP18/PP23/PP25 are broken; it is evidence that the driver never reaches them.

## 3. Existing PP capability inventory

Test counts below are the frozen-slice records (T01 `/21`..`/31`). Entries marked "run now" were executed during this review on the then-current mid-`/42` tree; they are chip-level results, not B3-03 acceptance.

| Chip | Kind | Implemented scope | Installed in candidate? | Evidence |
|---|---|---|---|---|
| PP01 normalize | `PREPROCESS_NORMALIZE` | Newline/CRLF/missing-final-LF normalization | **Yes** | c10; A03 7 tests |
| PP02 splice | `PREPROCESS_SPLICE` | Backslash-newline deletion, map composition to raw | **Yes** | c16; A03 |
| PP03 comment | `PREPROCESS_COMMENT` | Comment replacement; literal/header-name protection; whole-input scan, no rev-55 chunk state | **Yes** | c16/c20; A03 |
| PP04 scan | `PREPROCESS_SCAN` | Maximal-munch pp-tokens, `HeaderName`, M1 punctuators; non-ASCII/UCN/lone `\` deferred | **Yes** | c16/c20; A03 |
| PP05 directive | `PREPROCESS_DIRECTIVE` | Line grouping, `#error` fan-out, null directive no-op; **all other directives `Unsupported`** | **Yes** | c21 8 passed (run now) |
| PP06 define | `PREPROCESS_MACRO_DEFINE` | Object/function-like parse, fresh `MacroRecord`, PP07 child on collision | **No** | c23 10 passed (run now) |
| PP07 redefine | `PREPROCESS_MACRO_REDEFINE` | Benign-equivalence verifier, no writes | **No** (child only) | c23 |
| PP08 undef | `PREPROCESS_MACRO_UNDEF` | Tombstone writer, ignore-unknown | **No** | c23 |
| PP09 invoke | `PREPROCESS_MACRO_INVOKE` | Object/function-like non-variadic fan-out/stitch; directive lines verbatim; variadic use `Unsupported` to PP16 | **Yes** | c24 12 passed (run now) |
| PP10/11/13/14/15 | folded helpers | Argument collection/prescan/stringify/paste/rescan inside PP12 | n/a | c24 |
| PP12 substitute | `PREPROCESS_MACRO_SUBSTITUTE` | Substitution, prescan, `#`/`##`, blue paint | **Yes** | c24 |
| PP16 variadic | `PREPROCESS_VARIADIC_MACRO` | Variadic definition use; GNU comma swallowing deferred | **No** | c26 10 passed (run now) |
| PP17 resolve | `PREPROCESS_INCLUDE_RESOLVE` | Pure resolve over committed sources; exact then basename; quote = angle; ambiguous/not-loaded fail | **No** | c25 8 passed (run now) |
| PP18 enter | `PREPROCESS_INCLUDE_ENTER` | One-level token splice; requires scanned header; nested includes pass through | **No** | c25 |
| PP19 conditional | `PREPROCESS_CONDITIONAL` | Conditional stack; one-shot whole stream; keeps active non-conditional directives; object-like `#if` expansion; function-like `#if` unsupported | **Yes** | c22 8, c43 18 (run now c22) |
| PP20/21/22 | folded helpers | `defined`, expression parse/evaluate inside PP19 | n/a | c22 |
| PP23 line | `PREPROCESS_LINE_DIRECTIVE` | Validate `#line`/GNU marker, return `LogicalLocation`; complete `Ack`, no writes | **No** | c28 10 passed (run now) |
| PP24 builtin | `PREPROCESS_MACRO_BUILTIN` | `__FILE__`, `__LINE__`, `__COUNTER__`, target predefined set; `__LINE__` is physical fallback; date/time unsupported | **No** | c27 9 passed (run now) |
| PP25 pragma | `PREPROCESS_PRAGMA_DIRECTIVE` | Classify `once`/`pack`/opaque, `_Pragma` decode; complete `Ack`, no writes | **No** | c29 9 passed (run now) |
| PP26 diagnostic | `PREPROCESS_DIAGNOSTIC` | `#error` message product; always fails | **Yes** | c21 |
| PP27 expand map | `PREPROCESS_EXPAND_MAP` | Query-only origin chains; no producer/carrier | **No** | c30 10 passed (run now) |
| PP28 emit | `PREPROCESS_EMIT` | Directive strip, no gluing, marker-free `Preprocessed` artifact, primary map only | **Yes** | c31 11 passed (run now) |

### 3.1 What "works" means here

- **[IMPL] PP06/PP08 work at chip level.** `pp_define.rs:62-142` projects one post-`#` line; `:291-372` appends a fresh `MacroRecord` or enqueues PP07; `pp_undef.rs:161-260` appends a tombstone or acknowledges. c23 passed 10/10 in this review.
- **[IMPL] PP09/PP12 expand object-like and function-like non-variadic macros** from the committed table (`pp_invoke.rs:25-41`); c24 passed 12/12. This means the macro table and expansion machinery are not the gap.
- **[IMPL] PP17/PP18 work for the tested single-header case.** c25 passed 8/8. PP18 splices scanned header tokens and keeps one EOF.
- **[IMPL] PP23/PP25 validate their inputs.** c28/c29 passed. They do not persist anything.
- **[UNVERIFIED] Source-defined macro → PP09 expansion is not tested end-to-end in the driver.** c24 tests the order "define PP06 first, then PP09 over the original stream"; the driver has no such order.

## 4. What is missing, per directive

### 4.1 `#define`

**Present [IMPL/FROZEN]:** PP06 parses one post-`#` line and either appends a fresh `MacroRecord` or enqueues PP07 for a redefinition check (`pp_define.rs:62-142`, `:291-372`; frozen `/23`). PP09 expands later uses from the committed table (`pp_invoke.rs:25-41`).

**Missing [IMPL gaps]:**

1. PP06/PP07 are not imported, registered, routed, or in the worker registry (`candidate.rs:24-31`, `:223-294`, `:387-474`).
2. No host line loop fans out one PP06 task per `#define` line. PP05 sees the whole stream and fails first (`pp_directive.rs:315-320`, `:548-552`).
3. No ordering: `pp_prefix` runs PP09 once over the whole stream before any source define exists (`candidate.rs:667-687`).
4. No directive removal: the `#define` line would remain in the token stream if PP05 were bypassed (`candidate.rs:678-688`; `pp_invoke.rs:11-14`).
5. No safe behavior for defines that affect later conditionals. PP19 runs before PP06 (`candidate.rs:653-663`), and its macro table is a snapshot at dispatch (`pp_conditional.rs:118-151`). A naive reorder can silently select the wrong `#if` branch.

**Consequence:** a source `#define X 2` cannot currently affect either `-E` or the evidence model. The chip can do the work; the driver cannot sequence it.

### 4.2 `#undef`

**Present [IMPL/FROZEN]:** PP08 accepts one post-`#` line, validates the single identifier operand, and appends a tombstone when the name is defined; unknown/undefined is a no-op (`pp_undef.rs:161-260`; frozen `/23`). PP09's latest-wins projection reads the tombstone (`pp_invoke.rs:151-155`).

**Missing:**

- Same registration/routing/line-fan-out/ordering/removal gaps as `#define`.
- `#undef` must be processed before the PP09 expansion of the following text segment. A single whole-stream PP09 cannot express "defined before line N, undefined after line N".
- **[UNVERIFIED]** There is no test that drives PP08 followed by PP09 and asserts the spelling stays unexpanded; c23 tests the tombstone lifecycle, not expansion after undef.

### 4.3 `#include`

**Present [IMPL/FROZEN, single-level]:** PP17 resolves one `HeaderName` against committed source names (exact then basename; quote and angle identical; ambiguous → fail) (`pp_resolve.rs:5-12`, `:179-255`; frozen `/25`). PP18 replaces include lines with the header's already-scanned tokens and keeps a single EOF (`pp_enter.rs:261-370`; frozen `/25`).

**Missing [IMPL gaps]:**

1. PP17/PP18 are not installed or routed (`candidate.rs:223-294`, `:387-474`).
2. No host loader: PP17 only looks at **already committed** sources; it never reads a file (`pp_resolve.rs:5-12`; [PP_INCLUDE_SLICE.md](../../tasks/PP_INCLUDE_SLICE.md):13-19). The driver has no include path policy and refuses `-I` (`candidate.rs:143-144`).
3. **PP18's frozen contract is one header per task.** Its compute loop resolves every include line and fails if any resolves to a header other than the payload's trailing `Source` (`pp_enter.rs:304-335`). A source with `#include "a.h"` and `#include "b.h"` cannot be passed to one PP18 call. Either the host calls PP18 with a single include-line payload and mechanically splices the returned header tokens, or T03 must amend PP18. This is a blocking interface conflict, not a driver detail.
4. PP18 requires the header's tokens to be scanned **before** entry (`pp_enter.rs:271-277`; [PP_INCLUDE_SLICE.md](../../tasks/PP_INCLUDE_SLICE.md):67-70). The host must run PP01→PP04 for the header first.
5. No include stack, depth bound, `#pragma once`, include guards, search paths, `include_next`, or multi-level fixpoint ([PP_INCLUDE_SLICE.md](../../tasks/PP_INCLUDE_SLICE.md):105-114; [T03](../../tasks/T03_PREPROCESS_CHIPS.md):38). Nested include lines inside spliced headers pass through verbatim (`pp_enter.rs:13-15`).
6. Header conditionals need a second PP19 pass after stitching ([PP_INCLUDE_SLICE.md](../../tasks/PP_INCLUDE_SLICE.md):113-114). That is control-loop scope, not one driver call.
7. Source identity is unresolved: the driver names the input by file stem/basename (`candidate.rs:596-606`), while PP17 exact/basename matching can become ambiguous when two headers share a basename (A10 D-I-12, line 345; A10 F-06). H04 must freeze a name scheme or an explicit ambiguity failure.

**Consequence:** a leaf, literal, already-loaded header can be made to work after the H04 policy and the PP18 single-line workaround; real nested/system-header inclusion cannot be claimed.

### 4.4 `#line`

**Present [IMPL/FROZEN validation only]:** PP23 accepts `#line number "file"?` and GNU `# lineno "file" flags?`, validates the range/string escapes, and returns a `LogicalLocation` through the pure `line_location` helper (`pp_line.rs:215-243`). It completes `Ack` with **no bus write** (`pp_line.rs:44-45`, `:215-230`).

**Missing:**

- No logical-location carrier on the bus (no store/field/record). The comment states the wiring layer must persist it, but no such persistence exists (`pp_line.rs:241-243`).
- PP24 therefore uses the physical-line fallback for `__LINE__` (`pp_builtin.rs:18-21`, `:311-315`), and diagnostics have no logical file/line remapping.
- Even if the driver registers PP23, it can only validate then discard. Accepting `#line` and ignoring its effect would be a silent wrong result.

**Consequence:** B3-03 can route/validate `#line` only as a fail-closed staging step; applying it requires a separate T01/T03 carrier freeze (new schema).

### 4.5 `#pragma`

**Present [IMPL/FROZEN validation only]:** PP25 decodes `_Pragma("...")` and classifies `#pragma` parameters as `Once`, `PackPush`, `PackPop`, or `Opaque` (`pp_pragma.rs:190-195`, `:330-403`). It completes `Ack` and writes nothing (`pp_pragma.rs:28-47`, `:476-500`).

**Missing:**

- No `PragmaRecord` family/store, no once-set, no pack stack. The chip header explicitly records this deferral (`pp_pragma.rs:28-47`).
- PP17/PP18 never consult `once`; repeated inclusion is not suppressed.
- Layout has no pack consumer; accepting `pack` and ignoring it can produce a wrong ABI later.

**Consequence:** `once`, `pack`, and `_Pragma` effects cannot be implemented by driver orchestration. Opaque pragmas can be ignored per C11 6.10.6p1, but only if that policy is explicitly frozen; recognized `once`/`pack` should fail closed until their carriers land.

### 4.6 Cross-cutting gaps

| Gap | Evidence | Effect |
|---|---|---|
| Directive-bearing token stream reaches the lexer | `candidate.rs:678-688`, `:699-717`; `pp_invoke.rs:11-14` | Evidence path fails on any accepted directive; `-E` hides it via PP28 |
| One-shot PP19 before source macros | `candidate.rs:653-663`; `pp_conditional.rs:118-151` | `#if X` after source `#define X` takes the undefined branch silently |
| One-shot PP09 before/after all defines | `candidate.rs:667-677` | Use-before-define and redefinition order cannot be expressed |
| Raw physical-line grouping for directives | `pp_directive.rs:355-403`, `:427-438`; `pp_conditional.rs:443-...`; `pp_enter.rs:505-...`; `pp_splice.rs:6-9`, `:164` | A `\`-continued `#define`/`#include` is split at the physical newline; the later tokens are treated as a new line. **Reasoned from source; not executed.** |
| Include-prefix disagreement on a leading same-line comment | A03 §8b, `pp_comment.rs:200-208`, `pp_scan.rs:293-307` vs `pp_directive.rs:473-517` | `/*c*/ #include <a//b>` may have `//b>` treated as a comment before PP05 sees the directive. A host classifier must not copy the wrong predicate. |
| Duplicated directive predicates | A03 §15 (PP05/PP09/PP16/PP18/PP19 copies) | A host line classifier becomes a sixth copy unless T03 freezes a shared helper or accepts a drift test. |
| Fixed drain budget | `candidate.rs:62`, `:328-343`; PP05 fan-out `pp_directive.rs:327-350` | A batch of more than ~14 `#error` lines can exhaust `DRAIN_BUDGET=16` and surface "drain budget exhausted" instead of the first `#error`. **Reasoned; not tested.** |

## 5. Exact host / task / projection seams

### 5.1 Host seam

The host already owns: input bytes (`std::fs::read` at `candidate.rs:585`), source allocation (`:604-606`), `-D/-U` record commit (`:488-572`), output persistence (`:956-1009`), and the trace recorder (`step` at `:311-372`). It does not yet own a directive cursor or a header loader.

**Proposed host responsibilities [PROPOSED]:**

- group committed pp-tokens into raw directive lines using the frozen PP05 predicate (or a T03-approved shared helper);
- maintain an ordered cursor over the post-PP19 stream;
- enqueue existing tasks PP06/PP07/PP08/PP17/PP18/PP23/PP25/PP05 with the exact payload shapes in §5.4;
- remove consumed directive lines from the token stream;
- resolve/load headers only under the frozen H04 policy; never sample the environment, never shell out, never call GCC/Clang;
- fail closed on anything not explicitly supported.

The host must **not** evaluate macro replacement, conditionals, `#include` path semantics, or pragma semantics itself. Those remain chip rules. If the host must duplicate the directive-line recognition predicate, T03/T01 must freeze that as an orchestration predicate with a drift test.

### 5.2 Task seam

`step()` bootstraps exactly one task, drains all tasks to quiescence, and returns that task's result or diagnostic (`candidate.rs:311-372`). It is usable once per directive line, but has three properties the work order must address:

1. `DRAIN_BUDGET` is fixed at 16 (`candidate.rs:62`); PP05 can fan out one PP26 child per `#error` line (`pp_directive.rs:327-350`).
2. `quiescent()` checks **all** tasks in the arena (`candidate.rs:304-309`); sequential steps are fine because prior tasks are terminal, but a persistent cursor in the bus would change this model.
3. Each step records a trace record (`candidate.rs:336`); the host loop cursor is not a bus record. Replay is therefore recorded-input re-execution, not restore (B2-00 OD-6; `/43` for restore).

**No new task kind is needed** for the bounded work order. All directives use existing frozen kinds. Any persistent conditional stack/include stack/once-set would need a new carrier and is out of scope.

### 5.3 Projection seam

Chips read only committed arena state and never call another chip. The driver must guarantee this order:

| Order | Requirement | Why |
|---|---|---|
| 1 | PP01→PP04 scan complete | All later chips project pp-tokens/spans/sources |
| 2 | `-D`/`-U` seeds committed | PP19/PP09 see the command-line table |
| 3 | PP06/PP08 before the PP09 segment that follows | PP09 reads the latest macro table |
| 4 | leading PP06/PP08 before PP19 on the suffix | PP19's one-shot table must include source defines that precede conditionals |
| 5 | PP17 after the header source is committed; PP18 after the header is scanned | PP17 is committed-source-only; PP18 requires scanned header tokens |
| 6 | PP18 per include line (bounded workaround) | PP18 fails on a second distinct header in the same payload |
| 7 | PP05 only over remaining `#error`/unknown lines | PP05 fails on every other directive name |
| 8 | PP28 only after the final directive-free stream | PP28 strips directives and emits the `-E` artifact |

`-D`/`-U` direct host commit (`candidate.rs:488-572`) is a precedent, not a reason to bypass PP06 for source defines: PP06 owns function-like parsing, span adjacency, redefinition compatibility, and the `Pp/macros` allowlist write.

### 5.4 Payload conventions (important)

The frozen chips do not share one payload shape. A host dispatcher must use exactly these shapes:

| Chip | Payload | Source/test precedent |
|---|---|---|
| PP05 directive | All committed pp-token refs (stream includes EOF) | `pp_directive.rs:69-86`; c21 |
| PP06 define | One post-`#` line's refs; first token Identifier `define`, no EOF | `pp_define.rs:78-103`; c23 `refs[1..4]` |
| PP07 redefine | Post-`#` line refs + exactly one trailing `RecordRef::Macro` | `pp_redefine.rs:70-...` |
| PP08 undef | One post-`#` line's refs; first token Identifier `undef`, one identifier operand | `pp_undef.rs:161-260`; c23 `refs[1..3]` |
| PP09 invoke | All active pp-token refs (stream includes EOF) | `pp_invoke.rs:97-122`; c24 |
| PP17 resolve | Exactly one `HeaderName` pp-token ref | `pp_resolve.rs:46-70`; c25 `main_refs[2]` |
| PP18 enter | Full stream refs + exactly one trailing `RecordRef::Source`; header must be scanned | `pp_enter.rs:48-90`; [PP_INCLUDE_SLICE.md](../../tasks/PP_INCLUDE_SLICE.md):46-61 |
| PP19 conditional | All committed pp-token refs | `pp_conditional.rs:93-...`; c22 |
| PP23 line | Full directive-line refs including `#`/`%:`; no trailing EOF in tests | `pp_line.rs:86-...`; c28 `scan_line_refs` |
| PP25 pragma | Post-`#` refs (first Identifier `pragma`) **or** the four `_Pragma ( StringLiteral )` tokens | `pp_pragma.rs:82-...`; c29 removes the leading `#` |
| PP28 emit | Final pp-token refs in payload order | `pp_emit.rs:302-...`; c31 |

A single generic "dispatch line" helper that passes the same refs to every chip will fail protocol validation.

## 6. Blocking contract / coordination gates

### G1 — H04 include policy must be frozen before host header reads

[H04_I_FREEZE.md](../../tasks/H04_I_FREEZE.md) is **DRAFT ONLY — OPEN** and needs H01 + T01 sign-off (`:3-11`). It authorizes nothing today. The candidate currently refuses `-I` (`candidate.rs:143-144`), and PP17 resolves only committed sources (`pp_resolve.rs:5-12`).

Before any host header read, the freeze must settle at least:

1. **Search order and quote/angle divergence.** The draft proposes quote: including-file directory → `-I` → H01 system roots; angle: `-I` → H01 roots (`H04_I_FREEZE.md:32-43`). A no-`-I` B3-03 subset still needs an explicit decision: quote-relative only; angle fail closed until H01 roots.
2. **Source identity/name scheme.** PP17 exact/basename matching must agree with the host-committed source name; duplicate basenames must not silently misresolve (A10 D-I-12 line 345; A10 F-06). A minimal policy could be "commit under the header spelling; a second distinct file with the same spelling fails as ambiguous", but this is **not frozen**.
3. **Failure taxonomy and exit codes.** Not-found, ambiguous, empty, malformed, escaped root, and `include_next` need explicit classes; H04 must also reconcile A10 F-03/F-04 (exit 2 overload and host-read-as-exit-1).
4. **Sandbox/root pinning.** Absolute paths, `..`, symlink escapes, size limits, and environment variables must be defined (H04_I_FREEZE.md:44-48; AGENTS §9; A24).
5. **Depth/retry/once.** Nested include depth, host retry bound, `#pragma once` identity, and cancellation are T02/limits items (`H04_I_FREEZE.md:57-71`, `:94-98`).

Until G1 is accepted, B3-03 must not read a header file, must not add `-I`, and must keep the current explicit refusal.

### G2 — T03/T01 co-freeze of the directive-routing seam

The precise seam is unresolved and is the main design decision:

- **Option A (host routing; recommended for a bounded B3-03):** host groups lines and dispatches existing PP06/PP07/PP08/PP05 tasks. No chip behavior change, no schema. Cost: a sixth directive predicate copy; needs a T03-approved shared predicate or a drift test.
- **Option B (complete PP05 fan-out):** change PP05 from "reject every non-`#error`" to a dispatcher that fans out macro/include/line/pragma tasks. No schema, but it changes frozen `/21` behavior and needs a T03 contract amendment, new acceptance, and likely new manifests/allowlist review.

Either option must freeze:

1. which directive names are consumed by the host/PP05 and which are passed through;
2. the exact payload table in §5.4;
3. directive removal from the token stream before LX/PA;
4. fail-closed behavior for `#line`/`#pragma` carriers, nested includes, `once`, and `pack`;
5. the logical-line rule for `\`-continued directives (see G3/defect DEF-B303-02);
6. whether the host may recognize directives at all, or whether that violates the "semantic rules in chips" rule (AGENTS §3.3-3.6). This is a real interface conflict; do not resolve it by silent host duplication.

Related documentation conflict: [PP_CONDITIONAL_SLICE.md](../../tasks/PP_CONDITIONAL_SLICE.md):46-51 says any non-conditional directive line is an explicit `Unsupported` failure, while the implementation keeps active non-conditional directive lines (`pp_conditional.rs:246-260`) and T01's `/22` text says active non-conditional lines pass through. B3-03's host loop depends on the implemented behavior; T03/T01 should reconcile the package text before freezing the orchestration seam.

### G3 — T02 co-freeze of the control loop

Full correctness needs persistent conditional/include state across ticks: T03 requires "conditional stack management and include waits persist across ticks" ([T03](../../tasks/T03_PREPROCESS_CHIPS.md):38). T02 owns the control/scheduler side, including host fetch (`AwaitHost`/CT02), per-stage queues, cancellation, and limits ([T02](../../tasks/T02_CONTROL_CHIPS.md):3-12, `:15-26`, `:36-39`; [PP_INCLUDE_SLICE.md](../../tasks/PP_INCLUDE_SLICE.md):105-114).

A bounded synchronous host loop can be accepted as **H04 host tooling** only if T02/T01 agree that it is not a second scheduler and that the host cursor is not semantic bus state. Any persistent conditional stack, include stack, depth/once set, or async fetch requires T02 co-freeze and likely new state.

### G4 — Clean, frozen baseline

B3-03 implementation must start from a committed `/42` tree (now recorded frozen in the ledger) or from `/41`. During this review the tree moved from `/41` to uncommitted `/42`: `compiler/src/contract.rs` and `compiler/contracts/CONTRACT_VERSION` changed under the review, and an intermediate `h04_candidate` run failed its `/41` version assertion before the test was updated. Do not mix B2-02 constant-shape work with B3-03 PP orchestration.

### G5 — No new schema, chip, task, or config by B3-03

The bounded work order needs no new record family, store, field, task kind, stage row, chip ID, limit, or config field. In particular:

- `#line` logical locations, `#pragma once`/pack state, include stack/depth, and persistent conditional state are **new carriers**; each needs a separate T01/T03 CCR and freeze.
- No new chip ID may be allocated before the Worker-vs-`RestrictedChip` decision (ledger ID-CHIP-01; D-06).

If implementation discovers that a carrier is unavoidable, stop and file a CCR; do not add a private record/store or host-only semantic cache.

### G6 — Ownership and collision control

`compiler/src/bin/candidate.rs` is integrator-owned (A12 line 82; A10 `:692`, `:698-704`). `compiler/src/chips/preprocess/*` belongs to T03 owners. New host helper modules must be disjoint and assigned one writer each; tests must have integrator-assigned numbers. Do not run two writers on `candidate.rs`.

### G7 — Security and sandbox

Header loading is new host file I/O. It must use configured roots, reject traversal/absolute/escaped paths, enforce size limits, avoid environment sampling, and keep diagnostics free of unrelated file contents (AGENTS §9; H04_I_FREEZE.md:44-48; A24). This is a gate, not a follow-up.

### G8 — H04 flag/document freeze

H04 flag spellings are explicitly illustrative until the H04 contract freezes ([H04_CANDIDATE_DRIVER.md](../../tasks/H04_CANDIDATE_DRIVER.md):20-33, `:60-65`). `-std`/`-O` are validated but not propagated (`candidate.rs:65-78`, `:586-592`; A10 F-02). Do not claim dialect-dependent directive behavior (trigraphs, `%:`, `include_next`) while the dialect is hardcoded. The H04 evidence sample still names `/19`; the driver tests now expect `/42`. Documentation reconciliation is an acceptance item, not part of the code change.

### 6.1 Structured defects

| ID | Priority | Defect | Evidence | Disposition |
|---|---|---|---|---|
| DEF-B303-01 | **P0** | PP18 cannot process two distinct headers in one payload; it fails on any include line resolving to a different header. | `pp_enter.rs:304-335` | Host must call PP18 per include line and splice; or T03 amends PP18. Gate G2. |
| DEF-B303-02 | **P0** | Directive grouping uses raw physical lines, so a `\`-continued directive is split; the later tokens look like a new line. | `pp_directive.rs:355-403`, `:427-438`; `pp_conditional.rs:443-...`; `pp_enter.rs:505-...`; `pp_splice.rs:6-9`, `:164` | Fail closed on continued directives until T03 freezes logical-line grouping. **Reasoned; not executed.** |
| DEF-B303-03 | **P0** | PP19 runs before source PP06/PP08; a later `#if X` can silently take the wrong branch. | `candidate.rs:653-663`; `pp_conditional.rs:118-151` | Bounded driver must process leading defines first and reject macro mutations after the first conditional; full interleaving is T02/T03. |
| DEF-B303-04 | **P0** | Directive tokens remain in the evidence token stream after PP05. | `candidate.rs:678-688`, `:699-717`; `pp_invoke.rs:11-14`; `pp_directive.rs:234-249` | Host must remove consumed directive lines before PP09 output is lexed. |
| DEF-B303-05 | **P0** | PP05 rejects all non-`#error` directives; PP06/07/08/17/18/23/25 are not installed/routed. | `pp_directive.rs:541-581`; `candidate.rs:223-294`, `:387-474` | B3-03 plumbing; Gate G2. |
| DEF-B303-06 | **P1** | No logical-location or pragma carrier; PP23/PP25 cannot affect downstream semantics. | `pp_line.rs:44-45`, `:241-243`; `pp_pragma.rs:28-47`; `pp_builtin.rs:18-21` | Separate T01/T03 CCR; B3-03 fail-closed. |
| DEF-B303-07 | **P1** | Include policy, source identity, sandbox, and failure taxonomy are not frozen. | H04_I_FREEZE.md:3-11, `:32-48`, `:334-349`; A10 F-03/F-04/F-06 | Gate G1. |
| DEF-B303-08 | **P2** | Fixed `DRAIN_BUDGET=16` can misfire on many `#error` children. | `candidate.rs:62`, `:328-343`; `pp_directive.rs:327-350` | Replace with a checked/split budget or prove the bound; typed failure, never a false diagnostic. |
| DEF-B303-09 | **P2** | Exit 2 conflates usage/unsupported/refusal; host read failures exit 1. | `candidate.rs:945-1016`; A10 F-03/F-04 | H04 freeze; do not invent a taxonomy in B3-03. |
| DEF-B303-10 | **P1** | The working tree is not a committed baseline; `/42` B2-02 changes are uncommitted. | `git status`; `contract.rs:45` | Gate G4. |
| DEF-B303-11 | **P1** | PP19 package prose says non-conditional directives fail, but the implementation keeps active non-conditional directives; the host loop depends on the implementation. | [PP_CONDITIONAL_SLICE.md](../../tasks/PP_CONDITIONAL_SLICE.md):46-51 vs `pp_conditional.rs:246-260` | T03/T01 reconcile the text before the orchestration seam freezes. |

## 7. Proposed bounded implementation sequence

The sequence is deliberately split so that work which does not need header I/O can proceed after the seam decision, while `#include` waits for H04. **None of this is authorized by this report.**

### W0 — Freeze the seam and split the package (no code)

| Field | Content |
|---|---|
| Owner | T01 integrator, with T03 and T02 owners |
| Inputs | This report; A03 B-PPL-01/B-PPL-07; H04_I_FREEZE; T03/T02 |
| Deliverable | A recorded decision (ledger/CCR or accepted work order) selecting host routing vs PP05 extension; freezing the payload table, directive-removal rule, logical-line fail-closed rule, and exit behavior; splitting B3-03 into B3-03a (define/undef), B3-03b (leaf include), B3-03c (line/pragma carriers or fail-closed) |
| Acceptance | Decision recorded; `candidate.rs` owner and helper-module boundaries assigned; clean committed `/42` (or `/41`) baseline named |
| Non-goals | No code, no schema, no test edits |

### W1 — Driver plumbing + `#define`/`#undef` (B3-03a)

**Preconditions:** W0 seam decision; clean frozen baseline; T03 accepts the host directive predicate or PP05 extension; no include/line/pragma effects in scope.

**Files [PROPOSED]:**

| File | Change |
|---|---|
| `compiler/src/bin/candidate/pp_driver.rs` (new) | Host-only pure helper: raw-line grouping, directive-name classification, segment/cursor construction, continuation guard, conditional-order guard. No macro/conditional evaluation. |
| `compiler/src/bin/candidate.rs` (integrator) | Import/register/route PP06/PP07/PP08; replace the fixed PP19→PP09→PP05 block with the ordered segment loop; keep `-E` through PP28; keep `-D/-U`. |
| `compiler/tests/c45_pp_driver.rs` (new) | End-to-end driver tests in §8, rows PPD-01..PPD-17. |

**Bounded behavior [PROPOSED]:**

1. Run PP01→PP04 over the source.
2. Commit `-D`/`-U` seeds (existing).
3. Structurally group committed pp-tokens into raw directive lines. **Fail closed** on a directive whose tokens span more than one physical line (DEF-B303-02).
4. Walk the stream in order:
   - text segment → PP09 with the current macro table;
   - `#define` → PP06 with post-`#` refs; drain PP07 child if any;
   - `#undef` → PP08 with post-`#` refs;
   - null `#` line → drop;
   - `#error`/`#warning`/unknown → PP05 on the line refs (preserves the frozen taxonomy);
   - `#include`/`#line`/`#pragma` → explicit `Unsupported` in W1.
5. At the **first conditional directive**, finish the preceding segments, then run PP19 once on the remaining refs with the current macro table. Continue the walk over PP19's active output. **Fail closed** if any source `#define`/`#undef` appears after the first conditional (DEF-B303-03).
6. Remove every consumed directive line; keep exactly one EOF; hand the resulting directive-free stream to LX/PA/IR (evidence path) or PP28 (`-E`).

**Why this order:** text before the first define sees the old table; text after a define sees the new table; leading defines can drive conditionals because PP19 runs after them; conditionals cannot silently miss post-conditional defines because that case is rejected.

**Failure behavior:** PP06/PP07/PP08/PP19/PP05 diagnostics are candidate diagnostics (exit 1); guard violations are explicit `Unsupported` (exit 1); no partial evidence; no silent expansion or removal; no fallback.

**Tests:** see §8 PPD-01..PPD-17. Essential assertions: `-E` exact bytes; evidence `value=02` for `#define X 2` + `return X;`; directive tokens absent before LX; use-before-define not expanded; non-benign redefine fails; inactive define dropped; source define after `#if` rejected; `#error` still fails; deterministic rerun.

**Acceptance commands (on the frozen baseline):**

```sh
cargo test --locked --manifest-path compiler/Cargo.toml \
  --test c45_pp_driver --test c23_macro --test c22_conditional \
  --test c24_expand --test h04_candidate
cargo fmt --all -- --check
```

**Non-goals:** includes, line/pragma effects, nested/multi-level PP, variadic USE, new schema, full M1.

### W2 — Leaf `#include` (B3-03b)

**Preconditions:** G1 H04 include policy accepted; G2 PP18 single-header seam accepted (host per-line PP18 or T03 amendment); W1 merged.

**Files [PROPOSED]:**

| File | Change |
|---|---|
| `compiler/src/bin/candidate/pp_include.rs` (new) | Host-only: literal HeaderName extraction, frozen path policy, sandbox/read/size checks, source commit/name, PP01-04 header scan, PP17 resolve, PP18 single-line entry, header-directive processing. |
| `compiler/src/bin/candidate.rs` (integrator) | Import/register/route PP17/PP18; call the include hook at include positions; keep W1 loop for header directives. |
| `compiler/tests/c46_pp_include_driver.rs` (new) | End-to-end tests in §8, rows PPD-18..PPD-24, PPD-27. |

**Bounded behavior [PROPOSED]:**

- Only literal `#include "..."` / `<...>` header names; macro-generated names remain `Unsupported`.
- Quote includes: resolve relative to the including source under the frozen H04 policy. Angle includes: fail closed until H01 system roots are frozen.
- Read exactly once, under the configured root/size limit; commit the source under the frozen name; run PP01→PP04 exactly once.
- Call PP17 on the `HeaderName`; if it resolves to a different source than the host-loaded one, fail closed.
- Call PP18 with a payload containing the single include line plus the trailing `Source`; take the returned header tokens (drop the header's EOF) and splice them mechanically into the working stream. This is the bounded workaround for DEF-B303-01; it must be T03-approved.
- Process header directives with W1 rules, but **reject** header conditionals/guards, nested includes, `#pragma once`, and `include_next` in W2.
- Require includes to appear before the first conditional in the main source; otherwise fail closed.

**Failure behavior:** missing/ambiguous header, malformed name, path escape, nested include, header guard/conditional, `once`, and `include_next` are typed diagnostics (exit 1) or the frozen driver-error class; no partial output; no fallback; no external preprocessor.

**Tests:** see §8 PPD-18..PPD-24. Essential: plain header `-E`; header `#define` used by main → evidence value; missing header; ambiguous same-spelling headers; nested include refused; guard/conditional refused; `once` refused; traversal refused; determinism.

**Acceptance commands:**

```sh
cargo test --locked --manifest-path compiler/Cargo.toml \
  --test c46_pp_include_driver --test c25_include --test c45_pp_driver
cargo fmt --all -- --check
```

**Non-goals:** `-I`, system headers, multi-source, nested include, guards, once, header conditionals, `include_next`, target emission, cross-source provenance.

### W3 — `#line`/`#pragma` handling (B3-03c; optional and carrier-free)

**Preconditions:** T03/T01 accepts the validation-only/fail-closed policy; T02 does not require new state. If a carrier is required, stop and file a separate CCR.

**Options:**

| Option | Behavior | Recommendation |
|---|---|---|
| W3-fail-closed (default) | Do not register PP23/PP25. Leave PP05's existing explicit `Unsupported` messages. | **Recommended until carriers freeze.** Honest; no false capability. |
| W3-validate-only | Register PP23/PP25; validate the line/pragma; complete `Ack`; then fail closed with "logical-location/PragmaRecord carrier not frozen" for `#line`, `once`, `pack`. Drop opaque pragmas per C11 6.10.6p1 if that policy is frozen. | Acceptable only with an explicit T03/T01 decision. |

**Files (only if W3-validate-only):** `candidate.rs` registration/routes; `pp_driver.rs` dispatch; `compiler/tests/c47_pp_line_pragma.rs` (new).

**Failure behavior:** malformed `#line`/`_Pragma` → PP23/PP25 typed error (exit 1); valid `#line`/`once`/`pack` → explicit "carrier not frozen" (exit 1); never silently apply or silently ignore a recognized effect.

**Non-goals:** persisting a logical location, changing `__LINE__`/`__FILE__`, once-suppression, pack layout, new records/stores.

### W4 — Full control loop (not B3-03)

Nested includes, include stack/depth, include guards, `#pragma once`, conditionals interleaved with source defines/includes, async host fetch, per-stage limits, and full provenance require T03/T02 co-freeze and new explicit state. Do not start under B3-03. This is the boundary recorded by [PP_INCLUDE_SLICE.md](../../tasks/PP_INCLUDE_SLICE.md):105-114 and T03:38.

## 8. Acceptance test matrix

All expected values are hand-derived from the frozen PP rules and the existing driver's PP28 spacing convention; none is a reference-compiler oracle. Rows marked W2/W3 are gated.

| ID | Stage | Input | Expected | File |
|---|---|---|---|---|
| PPD-01 | W1 | `#define X 2` + `int main(void){return X;}` | `-E`: `int main ( void ) { return 2 ; }\n`; evidence `value=02` | c45 |
| PPD-02 | W1 | use `X` before `#define X 2` | `-E` leaves `X`; evidence fails typed | c45 |
| PPD-03 | W1 | two identical `#define X 1` | success; one macro record; PP07 benign | c45 |
| PPD-04 | W1 | `#define X 1` then `#define X 2` | exit 1; message names replacement | c45 |
| PPD-05 | W1 | `#define X 2` then `#undef X` then use `X` | `-E` leaves `X` | c45 |
| PPD-06 | W1 | `#undef X` then `#define X 2` then use | `-E` expands `2` | c45 |
| PPD-07 | W1 | `#if 0` + `#define X 2` + `#endif` + use `X` | define dropped; `-E` leaves `X` | c45 |
| PPD-08 | W1 | leading `#define X 1` + `#if X` + M1 body + `#endif` | success; true branch | c45 |
| PPD-09 | W1 | `#if 1` + `#define X 1` + `#endif` + use `X` | explicit `Unsupported`; no wrong branch | c45 |
| PPD-10 | W1 | active `#error boom` | exit 1; message `boom`; no evidence | c45 |
| PPD-11 | W1 | unknown directive `#foo` | exit 1; PP05 taxonomy message | c45 |
| PPD-12 | W1 | `#define X \` + newline + `2` | explicit `Unsupported` (continuation guard) | c45 |
| PPD-13 | W1 | `#define X 2` + M1 body | evidence succeeds; no `#`/`define` reaches LX/PA | c45 |
| PPD-14 | W1 | `#define F(x) x` + `F(2+3)` use | `-E` expands; no variadic claim | c45 |
| PPD-15 | W1 | `#define V(...) 1` + `V(2)` use | explicit `Unsupported` naming PP16 | c45 |
| PPD-16 | W1 | any PPD-01/08 run twice | byte-identical report/`-E`; snapshot/trace equal | c45 |
| PPD-17 | W1 | existing `-D`/`-U` cases | h04 macro-flag tests still pass | h04/c45 |
| PPD-18 | W2 | `#include "h.h"` where `h.h` is `int h = 1;` | `-E` contains `int h = 1 ;`; no include line | c46 |
| PPD-19 | W2 | `h.h` = `#define X 2`; main includes then returns `X` | evidence `value=02` | c46 |
| PPD-20 | W2 | missing `h.h` | exit 1 typed; no output file | c46 |
| PPD-21 | W2 | two different `h.h` candidates | explicit ambiguous/refusal; no silent pick | c46 |
| PPD-22 | W2 | header includes another header | explicit depth `Unsupported` | c46 |
| PPD-23 | W2 | header with `#ifndef` guard | explicit `Unsupported` until W4 | c46 |
| PPD-24 | W2 | header with `#pragma once` | explicit `Unsupported` until carrier | c46 |
| PPD-25 | W3 | valid `#line 5 "f.h"` | PP23 validates; explicit carrier `Unsupported` | c47 |
| PPD-26 | W3 | `#line 0` / malformed `_Pragma` | PP23/PP25 typed error | c47 |
| PPD-27 | W2 | `#include "../outside.h"` | explicit sandbox/refusal; file not read | c46 |
| PPD-28 | W2 | `<stdio.h>` | explicit `Unsupported` (no H01 roots) | c46 |

Additional acceptance checks:

- **No external compiler delegation:** no `std::process::Command` in the PP driver path; no GCC/Clang/`cpp` invocation.
- **No partial output:** `-o` writes only after the final success.
- **No silent fallback:** every unsupported directive/path is a typed message.
- **Clang/gcc flags:** `-S`/`-c` remain refused; `-I` remains refused until G1.

## 9. Failure behavior requirements

| Situation | Required behavior |
|---|---|
| PP06 malformed `#define` | Typed `Fail` (exit 1); no partial macro table |
| PP07 non-benign redefine | Typed `Fail` naming the differing part; no append |
| PP08 malformed `#undef` | Typed `Fail`; no tombstone |
| PP05 `#error` | Typed diagnostic with the joined message; no evidence |
| PP05 `#warning`/unknown | Explicit `Unsupported`; never silent |
| PP17 not loaded/ambiguous/empty | Typed `Fail`; no fallback |
| PP18 missing scan | Typed `Fail` ("header tokens not scanned"); host ordering bug |
| Nested include / depth > 1 | Explicit `Unsupported` until W4 |
| Header conditional/guard / `once` | Explicit `Unsupported` until carriers/loop |
| `#line`/`pack` valid but no carrier | Explicit "carrier not frozen" (exit 1); never apply or silently ignore |
| Continuation directive | Explicit `Unsupported` until logical-line co-freeze |
| Source macro mutation after first conditional | Explicit `Unsupported`; never take a potentially wrong branch |
| Drain/limit exhaustion | Typed error (exit 1); no infinite spin; no silent drop |
| Host read failure | Follow the frozen H04 exit taxonomy when it exists; until then, keep the current explicit error and do not claim H06 classification |
| Any internal invariant failure | Typed protocol/internal diagnostic; no panic, no fabricated success |

Exit codes remain the current convention unless H04 freezes a replacement: `0` success; `1` candidate diagnostic; `2` driver/usage/refused feature/non-UTF-8. Do not add a third semantic class ad hoc.

## 10. Non-goals

- `-I`, system/sysroot headers, multi-source compilation, `-S`/`-c`, linking, target execution.
- Nested/multi-level include, include stack/depth, `#include_next`, `#pragma once`, include guards, header conditionals.
- Logical-line grouping for `\`-continued directives (until T03 co-freeze).
- `#line`/`__LINE__`/`__FILE__` logical semantics; diagnostics remapping.
- `#pragma pack` layout effects and any `PragmaRecord`/once-set carrier.
- Variadic macro USE, GNU `, ## __VA_ARGS__` swallowing, function-like macro invocation in `#if`/`#elif`.
- Macro-generated `#include` names.
- New record families, stores, fields, task kinds, stage rows, chip IDs, limits, config fields, contract rules, or version bumps.
- Full C preprocessing, full M1 acceptance, GCC torture rate, target ABI/probe claims, formal correctness.
- Fallback to GCC/Clang/`cpp` or any external preprocessor for candidate output.

## 11. Verification record

### 11.1 Repository state observed

```text
$ git branch --show-current
wave4-development
$ git log -1 --oneline
316a3d4 feat: freeze /41 with determinism, limits, and replay closure
$ git status --porcelain=v1
 M compiler/src/bin/candidate.rs
 M compiler/src/bus.rs
 M compiler/src/chips/constant_layout_init/fold.rs
 M compiler/src/chips/ir_lower/ir_function.rs
 M compiler/src/chips/parse/mod.rs
 M compiler/src/chips/verify/vf_evidence.rs
 M compiler/src/chips/verify/vf_interpret.rs
 M compiler/src/contract.rs
 M compiler/src/snapshot.rs
 M compiler/tests/c05_codec.rs
 M compiler/tests/c08_gate1.rs
 M compiler/tests/c09_readiness.rs
 M compiler/tests/c14_se.rs
 M compiler/tests/c15_ir.rs
 M compiler/tests/c17_vf12.rs
 M compiler/tests/c38_vf14.rs
 M compiler/tests/c42_wiring.rs
?? docs/tasks/CONTRACT_CHANGE_REQUEST_B2_TAGGED_CONSTANTS.md
```

The list changed during the review. `compiler/src/contract.rs:45` declared `t01-c01-c06/42` at one observation; `compiler/contracts/CONTRACT_VERSION` moved from `/41` to `/42` (`hash=a9a1da92…abf1a5`) during the review. The ledger was subsequently updated to record `/42` as accepted/implemented/verified, but the `/42` integration changes remain uncommitted in the worktree. This is still not a B3-03 baseline until committed/clean.

### 11.2 Commands run and exact results

| # | Command | Result |
|---|---|---|
| 1 | `compiler/target/debug/candidate` on `#define X 2` source | exit 1; "unsupported directive `define`: macros are deferred to the macro chips (PP06 definition, PP09-PP16 expansion)" |
| 2 | same on `#include "h.h"` | exit 1; "unsupported directive `include`: header inclusion is deferred to PP17/PP18" |
| 3 | same on `#line 5 "x.c"` | exit 1; "deferred to PP23" |
| 4 | same on `#pragma once` | exit 1; "deferred to PP25" |
| 5 | `candidate -E` on the `#define` source | exit 1; same `define` message |
| 6 | `cargo test --locked --manifest-path compiler/Cargo.toml --test h04_candidate --test c23_macro --test c25_include --test c28_line --test c29_pragma` (first attempt) | **compile failure**, 13 errors, because the in-progress `/42` `ConstRecord` shape (fields `ty`/`value`/`negative`) did not match `snapshot.rs`/`vf_interpret.rs`; no tests ran |
| 7 | `cargo test --locked --manifest-path compiler/Cargo.toml --test h04_candidate` (intermediate, before the `/42` test update) | **17 passed, 1 failed**; `candidate_models_m1_return_5` expected `t01-c01-c06/41` but the binary emitted `/42`; 0.96 s |
| 7b | `cargo test --locked --manifest-path compiler/Cargo.toml --test h04_candidate --test freeze` (final observation after `/42`) | **h04 18 passed; freeze 15 passed** |
| 8 | `cargo test --locked --manifest-path compiler/Cargo.toml --test c23_macro --test c25_include --test c28_line --test c29_pragma --test c22_conditional --test c24_expand` | **all passed**: c23 10, c25 8, c28 10, c29 9, c22 8, c24 12 |
| 9 | `cargo test --locked --manifest-path compiler/Cargo.toml --test c21_directive --test c22_conditional --test c26_variadic --test c27_builtin --test c30_expand_map --test c31_emit` | **all passed**: 8, 8, 10, 9, 10, 11 |
| 10 | `cargo test --locked --manifest-path compiler/Cargo.toml --test freeze` | **15 passed** |

**Interpretation:** rows 8-10 confirm the chip-level PP inventory on the then-current `/42` tree. They are **not** B3-03 acceptance and do not prove driver orchestration. Row 7 is an intermediate `/41`-vs-`/42` version-bump failure, not a B3-03 regression; row 7b is the final post-`/42` observation. The first compile failure in row 6 shows the tree was not a stable baseline earlier in the review.

### 11.3 Not run

- Full `cargo test --workspace`; full compiler suite (the ledger records 723/50 for `/41` and 684/51 for `/42`; neither was re-run here).
- `cargo fmt --all -- --check`, clippy (D-04 parse-draft clippy remains a known red gate).
- GCC torture, H01 probe, H07 classification, target execution, ABI/attestation.
- No B3-03 driver tests exist; none were created by this report.
- The multi-line directive split and PP18 multi-header behavior were **reasoned from source**, not executed.

## 12. Decisions required before implementation

| # | Decision | Owner | Recommended default |
|---|---|---|---|
| 1 | Host routing vs PP05 fan-out extension | T01 + T03 | Host routing for a bounded B3-03, with a shared/T03-approved predicate; PP05 extension only as a separate freeze |
| 2 | Logical-line grouping for continued directives | T03 | Fail closed in B3-03; fix in T03 |
| 3 | PP18 multi-header contract | T03 | Host single-line PP18 splice for B3-03; consider a T03 amendment later |
| 4 | Minimal H04 include policy | H01 + T01 + H04 | Quote-relative leaf includes only; angle/system fail closed; no `-I`; explicit name scheme/ambiguity |
| 5 | Exit taxonomy for host I/O/include failures | H04 | Freeze before include work; do not invent |
| 6 | `#line`/`pragma` validation-only vs fail-closed | T03 + T01 | Fail closed until carriers; no validation-only claim unless explicitly accepted |
| 7 | Opaque pragma policy | T03 | Ignore opaque per C11 only if explicitly frozen; `once`/`pack` fail closed |
| 8 | B3-03 split and test numbering | Integrator | Approve B3-03a/b/c split and `c45`/`c46`/`c47` numbering |
| 9 | Commit/clean the `/42` baseline | T01 integrator | Start from a clean committed `/42`; never the current mixed worktree |
| 10 | Drain budget | Integrator/T02 | Checked per-step budget or split; no false "drain exhausted" diagnostic |

## 13. Source index

| Topic | Path / lines |
|---|---|
| Driver flags and refusals | `compiler/src/bin/candidate.rs:91-215`, `:143-144`, `:195-201` |
| Driver PP install/routes | `compiler/src/bin/candidate.rs:217-300`, `:387-474` |
| Driver `-D`/`-U` commit | `compiler/src/bin/candidate.rs:488-572`; `compiler/src/bin/candidate/macro_flags.rs:78-141` |
| Driver PP order | `compiler/src/bin/candidate.rs:581-689` |
| Directive leak into LX/PA | `compiler/src/bin/candidate.rs:678-688`, `:699-717` |
| `step`/drain budget | `compiler/src/bin/candidate.rs:62`, `:304-372` |
| `-E` path | `compiler/src/bin/candidate.rs:694-697`; `compiler/src/bin/candidate/emit_helper.rs:38-62` |
| PP05 dispatch/taxonomy | `compiler/src/chips/preprocess/pp_directive.rs:298-351`, `:541-581` |
| PP05 line grouping | `compiler/src/chips/preprocess/pp_directive.rs:355-403`, `:427-438` |
| PP06 define | `compiler/src/chips/preprocess/pp_define.rs:62-142`, `:291-372` |
| PP08 undef | `compiler/src/chips/preprocess/pp_undef.rs:161-260` |
| PP09 invoke | `compiler/src/chips/preprocess/pp_invoke.rs:5-41`, `:307-...`, `:658-671` |
| PP19 conditional | `compiler/src/chips/preprocess/pp_conditional.rs:118-151`, `:246-260`, `:700-765` |
| PP17 resolve | `compiler/src/chips/preprocess/pp_resolve.rs:5-12`, `:179-255` |
| PP18 enter | `compiler/src/chips/preprocess/pp_enter.rs:13-15`, `:261-330`, `:370-378`, `:505-...`, `:580-...` |
| PP23 line | `compiler/src/chips/preprocess/pp_line.rs:44-45`, `:215-243` |
| PP24 builtin | `compiler/src/chips/preprocess/pp_builtin.rs:18-21`, `:311-315` |
| PP25 pragma | `compiler/src/chips/preprocess/pp_pragma.rs:28-47`, `:190-195`, `:476-500` |
| PP28 emit | `compiler/src/chips/preprocess/pp_emit.rs:24-28`, `:302-...`, `:391-395` |
| PP02 splice/map | `compiler/src/chips/preprocess/pp_splice.rs:6-9`, `:164` |
| H04 flags/include deferral | [H04_CANDIDATE_DRIVER.md](../../tasks/H04_CANDIDATE_DRIVER.md):20-33, `:60-65` |
| H04 include draft | [H04_I_FREEZE.md](../../tasks/H04_I_FREEZE.md):3-11, `:24-48`, `:57-84`, `:334-349` |
| PP include freeze | [PP_INCLUDE_SLICE.md](../../tasks/PP_INCLUDE_SLICE.md):9, `:13-19`, `:46-72`, `:105-114` |
| PP macro/expand/conditional freezes | [PP_MACRO_DEFINE_SLICE.md](../../tasks/PP_MACRO_DEFINE_SLICE.md):9, `:45-89`; [PP_EXPAND_SLICE.md](../../tasks/PP_EXPAND_SLICE.md):30-41, `:133-141`; [PP_CONDITIONAL_SLICE.md](../../tasks/PP_CONDITIONAL_SLICE.md):20-23, `:121-126` |
| PP line/pragma/emit freezes | [PP_LINE_SLICE.md](../../tasks/PP_LINE_SLICE.md):9, `:107`; [PP_PRAGMA_SLICE.md](../../tasks/PP_PRAGMA_SLICE.md):9, `:109`; [PP_EMIT_SLICE.md](../../tasks/PP_EMIT_SLICE.md):9, `:122-126` |
| T03 scope/scheduling | [T03_PREPROCESS_CHIPS.md](../../tasks/T03_PREPROCESS_CHIPS.md):3-5, `:38`, `:65-74` |
| T02 control loop | [T02_CONTROL_CHIPS.md](../../tasks/T02_CONTROL_CHIPS.md):3-12, `:15-26`, `:36-39`, `:59-100` |
| Determinism/replay boundary | [B2-00 CCR](../../tasks/CONTRACT_CHANGE_REQUEST_B2_DETERMINISM_LIMITS.md):30-56, `:631-648`, `:790-801` |
| Prior PP audit findings | [A03_PREPROCESS_LEX.md](A03_PREPROCESS_LEX.md):41, `:74`, `:93-113`, `:222-243`, `:280-291` |
| Host/H04 findings | [A10_HOST_H04_H05_PLAN.md](A10_HOST_H04_H05_PLAN.md):28-34, `:68-79`, `:169`, `:334-349`, `:692-704` |
| Ownership | [A12_OWNERSHIP_AND_DISPATCH_RISK.md](A12_OWNERSHIP_AND_DISPATCH_RISK.md):82 |
