# Dispatch Ledger

| Field | Value |
|---|---|
| Status | **Active dispatch record; not a contract freeze or completion claim** |
| Updated | 2026-11-09 |
| Master dispatcher | Integrator role |
| Plan | [MASS_PARALLEL_DISPATCH.md](MASS_PARALLEL_DISPATCH.md) |
| Contract baseline | `t01-c01-c06/48` — hash `820230c41c37ec4ce12d7517172b767d837211b4ef638c6b7a2f2d111c1ad58e` |
| Branch / HEAD | `wave4-development` / `fdfe066` (code through the M1 integer-IR emitter and frame hardening committed; this status-doc commit follows) |
| Workspace note | `/42`–`/48`, B-INT-00, B3-03a/B, the target probe, and H05 are committed; no push was performed |

## 1. Baseline Checks

| Check | Result | Notes |
|---|---|---|
| Root package tests | **PASS**, 21 tests across 3 suites | Observed in the `/40` integration session |
| Compiler package tests | **PASS**, **730 tests across 61 suites (731 on native AArch64 Linux) after the M1 integer-IR slice** | Observed in the M1 locals/arithmetic session. Adds four candidate-bin target-emitter tests on top of the M1 constant-return prototype. |
| Root/compiler/torture/chip-lint formatting | **PASS** | `cargo fmt --all -- --check` plus all three manifest-path checks on 2026-11-09 |
| Root tests after B3-03a | **PASS**, 21 tests | `cargo test --workspace --locked` |
| Torture package after B3-03a | **PASS**, 73 tests | `cargo test --locked --manifest-path tools/torture/Cargo.toml` |
| Chip-lint after B3-03a | **PASS**, 16 tests | `cargo test --locked --manifest-path tools/chip-lint/Cargo.toml`; clippy clean |
| Compiler clippy after B3-03a | **PASS** | `cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings`; D-04 closed by B-INT-00 |
| Target probe | **PINNED PROBE-ATTESTED BASELINE** | Debian 12 aarch64 + GCC 12.2.0; `c48_target_probe` 3/3; global `target_verification=probe-attested`; official T00 CI/bare-metal baseline and corpus remain open |
| GCC torture corpus | **NOT FROZEN / NOT RUN** | No pass-rate claim |

The current `/48` schema baseline is green for the configured root, compiler,
chip-lint, and torture checks. This is a repository check result, not target
execution or a GCC pass rate.

## 2. Stage A Batch A1

All A1 tasks are read-only on source. Each worker may write only its assigned report.
No worker may modify source, tests, contracts, Cargo files, `mod.rs`, shared indexes,
or user work; no worker may commit or push.

| Task | Scope | Worker session | Report | State |
|---|---|---|---|---|
| A-AUD-01 | Root framework and trust-boundary audit | `ses_e4b347361ffekZh6RXys5cjGWE` | `docs/reviews/dispatch/A01_FRAMEWORK_BASELINE.md` | COMPLETE — reviewed |
| A-AUD-02 | Compiler contract foundation audit | `ses_e4b34735bffes5dUV1kaErgZg0` | `docs/reviews/dispatch/A02_CONTRACT_FOUNDATION.md` | COMPLETE — reviewed |
| A-AUD-03 | Preprocess and lex capability audit | `ses_e4b3472c5ffe1Qorhnt3v5IJpf` | `docs/reviews/dispatch/A03_PREPROCESS_LEX.md` | COMPLETE — reviewed; found `#if` and octal defects |
| A-AUD-04 | Parse/type/semantic capability audit | `ses_e4b3472c3ffeo70B4V7LyweVho` | `docs/reviews/dispatch/A04_PARSE_TYPE_SEMANTICS.md` | COMPLETE — reviewed; found P0 wrong acceptance |
| A-AUD-05 | IR/target/Host/tooling audit | `ses_e4b3472c1ffeCIbOGINEaBxJFT` | `docs/reviews/dispatch/A05_IR_TARGET_HOST.md` | COMPLETE — reviewed; found M1 A3 gap |
| A-PTS-01 | V1 contract requirements | `ses_e4b3472bfffe2n6oWqgnzavfco` | `docs/reviews/dispatch/A06_V1_CONTRACT_REQUIREMENTS.md` | COMPLETE — reviewed |
| A-PTS-02 | V2 control-flow contract requirements | `ses_e4b3472beffehEObPsJFlaHoQv` | `docs/reviews/dispatch/A07_V2_CONTROL_FLOW_CONTRACT.md` | COMPLETE — reviewed (freeze F-V2-00..05) |
| A-PTS-03 | V3 function/memory/ABI contract requirements | `ses_e4b3472beffdR690UGdog6RfWi` | `docs/reviews/dispatch/A08_V3_FUNCTIONS_MEMORY_ABI_CONTRACT.md` | COMPLETE — reviewed (26 conflicts; probe-gated) |
| A-ABI-01 | Minimal target-chain evidence plan | `ses_e4b3472bdffeEEc3gioqmeqPt7` | `docs/reviews/dispatch/A09_MINIMAL_TARGET_CHAIN_PLAN.md` | COMPLETE — reviewed |
| A-HST-01 | H04/H05 Host acceptance plan | `ses_e4b3472bbffe68YWmIkBQLAIHN` | `docs/reviews/dispatch/A10_HOST_H04_H05_PLAN.md` | COMPLETE — reviewed; found candidate Host defects |
| A-TST-01 | V1 independent acceptance design | `ses_e4b3472b9ffeYxP28bFiDNUrKz` | `docs/reviews/dispatch/A11_V1_INDEPENDENT_TESTS.md` | COMPLETE — reviewed (113 case rows) |
| A-REV-01 | Ownership and dispatch-risk review | integrator synthesis (two worker capacity failures) | `docs/reviews/dispatch/A12_OWNERSHIP_AND_DISPATCH_RISK.md` | COMPLETE — integrator-authored from A04/A13 + source checks |

## 3. Stage A Batch A2

Second read-only wave; same write restriction and English-report requirement. It must
not duplicate A1 files or edit any shared source.

| Task | Scope | Worker session | Report | State |
|---|---|---|---|---|
| A-AUD-06 | Tests, CI, and documentation-status audit | `ses_e4b30c6e6ffej9Q4ycCDWSEc1N` | `docs/reviews/dispatch/A13_TESTS_CI_DOCS.md` | COMPLETE — reviewed |
| A-PTS-04 | Constants, layout, and initialization contract requirements | `ses_e4b30c6e5ffeoGAOvCl05owaVE` | `docs/reviews/dispatch/A14_CONST_LAYOUT_INIT_CONTRACT.md` | COMPLETE — reviewed |
| A-PTS-05 | IR, CFG, memory, effects, and versioning contract requirements | `ses_e4b30c6acffeHz3otvCtC3f3wX` | `docs/reviews/dispatch/A15_IR_CONTRACT_REQUIREMENTS.md` | COMPLETE — reviewed |
| A-PTS-06 | T02 control/commit/join/cancel requirements | `ses_e4b30c6abffewzdszkmEnseEF7` | `docs/reviews/dispatch/A16_CONTROL_PROTOCOL_REQUIREMENTS.md` | COMPLETE — reviewed |
| A-PTS-07 | T13 verification and evidence requirements | `ses_e4b30c6abffdjF9dd1iHy9meKZ` | `docs/reviews/dispatch/A17_VERIFICATION_REQUIREMENTS.md` | COMPLETE — reviewed (worker hit capacity after writing the full report) |
| A-CLI-01 | Diagnostic/error taxonomy and ownership requirements | `ses_e4b30c6abffcIR5nNES0MCqAwz` | `docs/reviews/dispatch/A18_DIAGNOSTIC_TAXONOMY.md` | COMPLETE — reviewed |
| A-TST-02 | V2 independent acceptance design | `ses_e4b30c6aaffevkjrn6N4dN5hUj` | `docs/reviews/dispatch/A19_V2_INDEPENDENT_TESTS.md` | COMPLETE — reviewed |
| A-TST-03 | V3 independent acceptance design | `ses_e4b30c6aaffdiGUBSxpqe8l6K6` | `docs/reviews/dispatch/A20_V3_INDEPENDENT_TESTS.md` | COMPLETE — reviewed (179-case matrix) |
| A-HST-02 | T00 corpus/H02/H03 official-driver plan | `ses_e4b30c6aaffc5gBq127vww5WOY` | `docs/reviews/dispatch/A21_TORTURE_PLAN.md` | COMPLETE — reviewed |
| A-ABI-02 | Target integer/floating-point model requirements | `ses_e4b30c6a9ffeRwGy520AJ5lNJ4` | `docs/reviews/dispatch/A22_TARGET_MODEL_REQUIREMENTS.md` | COMPLETE — reviewed |
| A-REV-02 | Determinism, replay, and invariant review | `ses_e4b30c6a9ffdf7ZNWoLbqUtHOP` | `docs/reviews/dispatch/A23_DETERMINISM_REVIEW.md` | COMPLETE — reviewed |

| A-REV-03 | Security, sandbox, and secret-handling review | `ses_e4b30c6a8ffeNQsnzkoEW9JW1l` | `docs/reviews/dispatch/A24_SECURITY_REVIEW.md` | COMPLETE — reviewed |

## 3A. Findings Register

| ID | Priority | Finding | Evidence | Disposition |
|---|---|---|---|---|
| D-01 | **P0** | Candidate accepts non-M1 operators/delimiters and misinterprets them: `return 2*3;` and `return 2-3;` both return 5; malformed `main(void]{...;)` is accepted. `PaTuChip` checks token kinds and only four spellings, not punctuator spellings. | A04 §1.2, §6.3; `pa_tu.rs:193-233` | **FIXED / VERIFIED** by B1-01 |
| D-02 | **P0** | SE29 `FunctionDefinition` `SemRecord` is not dispatched by the candidate; VF06 checks only three node kinds. M1 A3 is not complete. | A05 §3.2 G3; `candidate.rs:252-289`, `vf_invariant.rs:180-184` | **FIXED / VERIFIED by `/40`** — candidate dispatches SE29; VF06 checks four M1 kinds; T09 consumes the committed fact. Evidence: `c43_a3` 5/5, `c14_se` 16/16, `c15_ir` 13/13, `h04_candidate` 18/18, candidate `value=05` |
| D-03 | **P0** | Candidate panics (exit 101) on non-UTF-8 argv; no structured driver error. | A10 §1.3 F-01; A24 L-01 | **FIXED / VERIFIED** by B1-02 |
| D-04 | **P1** | Current clippy gate fails with 137 diagnostics from five declared parse drafts; four additional drafts are not compiled at all. `/40` changed files have zero diagnostics. | A13 §5-6; `/40` clippy run | **FIXED / VERIFIED by B-INT-00** — all nine unregistered drafts parked; clippy clean; see [disposition](B_INT_00_DRAFT_DISPOSITION.md) |
| D-05 | **P1** | Duplicate candidate local 27 / `ChipId(58)` across several drafts; PA15 has two implementations (`pa_init.rs`, `pa_expr_ext.rs`). | A04 §7; A13 §5 | **DISPOSITIONED by B-INT-00** — no parked draft is compiled/registered; `pa_init.rs` is the future PA15 candidate, `pa_expr_ext.rs` is non-canonical; IDs stay unfrozen under B3-04 |
| D-06 | **P1** | All 65 chips use local `Worker`/`handle(&CompilerBus)`, not `RestrictedChip`/`ChipAdapter`; contract has two competing chip boundaries. | A02 F01 | Contract freeze decision |
| D-07 | **P1** | H01 probe never ran; H07 classification and attestation handoff absent; four ABI fields unresolved. | A05 §2.3; A09 | **FIXED for the pinned probe baseline 2026-11-09** — containerized Debian GCC 12.2.0 report, H07 classification, global `TargetSpec::aarch64_unknown_linux_gnu_probe_attested`, `c48_target_probe` 3/3. Official T00 CI/bare-metal baseline and corpus remain open |
| D-08 | **P1** | H05 isolated execution is absent; no timeout/resource/cancellation/crash boundary. | A10; A24 H-01 | **IMPLEMENTED / VERIFIED (host tooling)** — `tools/runner` provides per-instance dirs, cleared env, timeout, output caps, cancellation, Linux `prlimit`/process-group termination, and outcome classification. `docs/tasks/H05_ISOLATED_RUNNER.md`; 10 Linux tests / 8 macOS tests |
| D-09 | **P1** | `ConstRecord` has no `ty`; documented immediate-type rule cannot be enforced. A14 proposes a tagged v2 record. | A05 G4; A14 §4.2 | **FIXED / VERIFIED by `/42`** — `ConstRecord { ty, value: ConstValue::Integer(..) }`; T09/VF12 enforce `ir.op-immediate-type`; `c45_const_ty` 8/8 |
| D-10 | **P2** | Status docs still claim `/6`, `/7`, `/9`, `/16`, "no chips", "torture not in CI", and stale H04/VF06 status. | A01 §3.3; A13 §8 | **FIXED / VERIFIED** by B1-03; dated audit retained with banner |
| D-11 | **P2** | `-std`/`-O` are validated but not propagated; exit code 2 conflates usage/unsupported/Part B refusal; output writes are non-atomic. | A10 §1.3 F-02..F-04, F-12 | B2 candidate hardening |
| D-12 | **P0** | `AppendRecords` wire proposals encode only the draft count in snapshots; different append batches with equal counts hash identically. This contradicts `snapshot.wire-payloads-encoded`. | A23 A23-F01; `snapshot.rs` | **FIXED / VERIFIED by `/41`** — handles and full bodies encode in stored order; c05 sensitivity covers all 18 body variants |
| D-13 | **P1** | Scheduler order uses task-group ordinal, not `STAGE_ASSIGNMENT`; T01 three-key order conflicts with M1 D5 four-key stage-first order. | A23 finding 5; `routing.rs` | **FIXED / VERIFIED by `/41`** — stage-first `(stage, group, enqueue, TaskId)`; `STAGE_ASSIGNMENT` authoritative; c06 19/19 |
| D-14 | **P1** | `allocated_total()` omits macros; `stage_queue_bound` is unused; zero quota is clamped; typed appends do not bump `store_versions`; `ResultRecord.version` is always 0. | A23 findings 14-16; A02 F04 | **FIXED / VERIFIED by `/41`** — macro total, store/Names bumps, typed zero-quota failure, 1-based result version; `stage_queue_bound` explicitly reserved to `/49` |
| D-15 | **P1** | `Snapshot`/`Trace` have no restore/decoder/diff; "replay" tests re-run fresh buses and pins are not recorded. | A23 finding 2 | **FIXED / VERIFIED minimal by `/41`** — pin digests, trace decoder/first divergence, `Task.id`, `ReplayEvidence`; full restore is `/49` |
| D-16 | **P1** | Diagnostic codes are ad hoc: `Task:4` has 339 sites and `Unsupported:1` 106; capacity, invalid, and verification faults lack distinct groups/codes; spans/task attribution are rare. | A18 §1 | Diagnostic CCR B2 |
| D-17 | **P0** | `#if X` does not macro-expand identifiers; a `-DX=1` probe takes the false branch, silently selecting the wrong conditional branch. | A03 §2.3, probe in §1.2 | **FIXED / VERIFIED** by B3-01 for object-like macros; function-like invocations remain explicit Unsupported |
| D-18 | **P0** | Integer `010` is classified/decode as decimal 10 instead of octal 8; radix/suffix/type/range handling is absent. | A03 §2.5 | **FIXED / VERIFIED** for decimal/octal/hex by B3-02; suffix/type/range remain future |
| D-19 | **P1** | Candidate runs only PP01–04/19/09/05/26/28; `#define`, `#include`, `#line`, `#pragma` still fail at PP05. | A03 §2.2 | **PARTIALLY FIXED by B3-03a/B3-03b** — source `#define`/`#undef` and bounded quoted `#include "..."` work; `#line`/`#pragma`, `-I`, angle/system, nested includes, and guards/`once` remain fail-closed. Evidence: `c46_pp_driver` 10/10, `c47_pp_include_driver` 12/12; see [PP_DRIVER_SLICE](PP_DRIVER_SLICE.md) and [leaf-include policy](B3_03B_LEAF_INCLUDE_POLICY.md) |
| D-20 | **P1** | Integrated lexer accepts only M1 decimal integers; `TokenKind` lacks Character/String/Float, so float/char/string/adjacent-string tokens cannot flow. | A03 §2.4 | Lex token model B3 |
| D-21 | **P1** | Parser architecture conflict: A06 proposes a single-task V1 parser, while A07 requires multi-task AST child links, task self-continuation/frame updates, and V2 control regions. `/39` `NodeRecord.children` cannot be populated across tasks. | A07 §1; A06 implementation orders | Freeze one parser/continuation strategy before V1/V2 |

## 3B. Stage B Batch B1: Safe, No-New-Schema Work

B1 is intentionally limited to fixes/tests/docs that do not require a new frozen
schema. Each worker has an exclusive file set and must not touch user drafts, the
ledger, or other reports.

| Order | Scope | Owned paths | Acceptance | State / session |
|---|---|---|---|---|
| B1-01 | Tighten PA01 exact-shape validation: verify every punctuator spelling, not only token kind; reject `2*3`, `2-3`, and malformed delimiters before AST append. Add independent regression tests. | `compiler/src/chips/parse/pa_tu.rs`, new `compiler/tests/c40_pa_guard.rs` | `cargo test --manifest-path compiler/Cargo.toml --test c12_parse --test c40_pa_guard`; candidate rejects the three D-01 inputs; existing M1 fixture still returns 5 | **COMPLETE / INTEGRATOR VERIFIED** — c12 5/5, c40 4/4, h04 18/18; all three D-01 probes exit 1, canonical still `value=05` (`ses_e4b0fb052ffeQB8EOeXxn2xrxD`) |
| B1-02 | Harden candidate argv handling: use `args_os`, reject non-UTF-8 arguments with a structured exit-2 diagnostic instead of a Rust panic; add a Unix regression test. Do not change flag semantics. | `compiler/src/bin/candidate.rs`, new `compiler/tests/c41_candidate_argv.rs` | `cargo test --manifest-path compiler/Cargo.toml --test h04_candidate --test c41_candidate_argv`; non-UTF-8 argv exits 2, no backtrace | **COMPLETE / INTEGRATOR VERIFIED** — c41 2 passed, h04 18 passed (`ses_e4b0fb051ffewR31XnMnJUB56g`) |
| B1-03 | Correct stale current-status documentation to `/39` and observed CI/test facts; add a dated-audit banner instead of rewriting history. | `README.md`, `compiler/README.md`, `docs/guide/{PROJECT_STATUS,PROJECT_OVERVIEW,ARCHITECTURE,GLOSSARY,ROADMAP}.md`, `docs/tasks/README.md`, `docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md`, `docs/reviews/2026-10-05_CI_GATING_AUDIT.md` | No current-status page says `/6`, `/7`, or `/16`; no claim that torture is outside CI; changed local links resolve; `git diff --check` | **COMPLETE / INTEGRATOR APPLIED** — explore retry was read-only, so the integrator applied the reconciliation; stale scans and link checks pass |
| B1-04 | Ownership/dispatch-risk review; two worker attempts failed on capacity, integrator synthesized from A04/A13 + source checks. | `docs/reviews/dispatch/A12_OWNERSHIP_AND_DISPATCH_RISK.md` | Collision/ownership table, safe Stage-B batch boundaries, blockers | **COMPLETE** — integrator-authored |
| B1-05 | Add pair of unit tests for chip-lint `lint_tree` (nested fixture, missing/empty dir, syntax error, deterministic sorted order). | `tools/chip-lint/tests/lint.rs`, new fixtures under `tools/chip-lint/tests/fixtures/` | `cargo test --manifest-path tools/chip-lint/Cargo.toml`; `cargo clippy ... -D warnings` | **COMPLETE / INTEGRATOR VERIFIED** — 16 tests passed, clippy clean (`ses_e4b0fb034ffeP4BLAJfbg45g70`) |
| B1-06 | Add an orphan-source reachability guard with an explicit four-file draft allowlist and a check that every `mod` declaration resolves. | new `compiler/tests/c42_wiring.rs` | `cargo test --manifest-path compiler/Cargo.toml --test c42_wiring`; `cargo clippy ... --test c42_wiring -- -D warnings` | **COMPLETE / INTEGRATOR VERIFIED** — 1 test passed (`ses_e4b0e3c8cffe1sJzGW4ljBlD6J`) |

## 3C. Stage B2 Contract Change Requests

| Order | Scope | Output | State / session |
|---|---|---|---|
| B2-00 | Determinism/limits/replay cluster D-12..D-15: append snapshot bodies, scheduler order, limit accounting, store versions, replay evidence. | `docs/tasks/CONTRACT_CHANGE_REQUEST_B2_DETERMINISM_LIMITS.md` | **ACCEPTED / IMPLEMENTED / VERIFIED** — frozen as `/41` (hash `500abb52…832966`); c05 29, c06 19, c07 20, c03 48, c08 22, c11 11; full 723/50. Restore/quota>1/VF13 deferred to `/49` |
| B2-01 | M1 A3 closure D-02: wire SE29, extend VF06 function-signature checking, consume FunctionDefinition fact in IR; state `ConstRecord.ty` dependency. | `docs/tasks/CONTRACT_CHANGE_REQUEST_B2_M1_A3_SE29.md` | **ACCEPTED / IMPLEMENTED / VERIFIED** — frozen as `/40` (hash `c6961ac6…10c14`); `c14_se` 16, `c15_ir` 13, `c43_a3` 5, `h04_candidate` 18; candidate `value=05` |
| B2-02 | Tagged, typed constants D-09: `ConstRecord.ty`, closed integer payload, fold type resolution, T09/VF12 immediate-type enforcement, snapshot codec, `/42` freeze. | `docs/tasks/CONTRACT_CHANGE_REQUEST_B2_TAGGED_CONSTANTS.md` | **ACCEPTED / IMPLEMENTED / VERIFIED** — frozen as `/42` (hash `a9a1da92…abf1a5`); `c45_const_ty` 8, `c08_gate1` 22, `c15_ir` 13, `c17_vf12` 6, `c38_vf14` 10; candidate `value=05` |

## 3D. Stage A Exit Summary

All 24 Stage A reports are present. The reports are independent read-only audits or
design packages; presence is not acceptance of their proposals. The integrator has
triaged the critical path as follows.

### Confirmed critical path

1. **Finish B1 safe fixes** (PA01 exact gate, candidate argv, docs, wiring guards).
2. **B2 contract decisions.** B2-01 (M1 A3 SE29) is **accepted, implemented, and
   verified as `/40`**; B2-00 (determinism/limits/replay) is **accepted,
   implemented, and verified as `/41`**; B2-02 (tagged, typed constants, D-09)
   is **accepted, implemented, and verified as `/42`**; `/43` parser/AST and
   `/44` IR/CFG and `/45` constant v3 are **frozen schemas**. The next schema
   next schema freeze is the deferred
   restore/quota>1/VF13/`StorePatch` package at `/49`; H05/torture are host/tooling work without a language-schema bump.
3. **Fix PP/lex wrong-result defects** before claiming V1 evidence:
   D-17 (`#if` macro expansion), D-18 (octal/radix), D-19 (driver PP orchestration),
   D-20 (literal token kinds).
4. **Then** choose the canonical PA15 implementation and allocate distinct parse/lex
   IDs; only after that wire V1 AST/type/semantic/IR producers.
5. **Target track:** one containerized H01 probe + H07 classification + T01
   attestation test now pass for a pinned Debian GCC 12.2.0 report. The official
   T00 reference baseline/CI substrate remains open; do not implement `-S`/`-c`
   or claim global target verification from the provisional report.
6. **H05 process supervision is implemented**; it is not a kernel/namespace
   sandbox and still requires H02/H03 orchestration plus the target runner.
   No untrusted target execution is claimed.

### Provisional Integrator Decisions (not frozen)

| ID | Decision / recommendation | Status |
|---|---|---|
| ID-PARSER-01 | Use A06's single-task `PARSE_TU` bootstrap for V1 only. Do not add `NodeLinkRecord`, `SetFrame`, or a taskified parser in V1. Before V2, freeze either (a) the same single parser extended to V2 or (b) a taskified parser; retire the losing path. Never run two AST producers. A06 independently recommends this bootstrap; A07 records the V2 conflict. | Recommendation pending owner/T01 acceptance |
| ID-IR-01 | Prefer A15's `InstructionRecord.successors` + `Branch`/`CondBranch` over A07's separate `CfgEdgeRecord`; keep the edge family only as a documented fallback. | Recommendation pending IR freeze |
| ID-TARGET-01 | All concrete ABI/scalar values remain probe-gated. No target codegen or `-S`/`-c` from proposal values. | Existing accepted boundary |
| ID-CHIP-01 | Do not add new chips until the Worker-vs-RestrictedChip template decision (A02 F01) is resolved; B1/B3 fixes may continue because they touch existing files. | Recommendation pending contract decision |
| ID-VERSION-01 | **Accepted 2026-11-09:** `/40` = M1 A3 SE29 closure (frozen), `/41` = B2-00 determinism/limits/replay mandatory package (frozen), `/42` = B2-02 tagged, typed constants (frozen), `/43` = parser/AST common base (frozen), `/44` = IR/CFG schema (frozen), `/45` = constant v3 float/address (frozen), `/46` = type/layout/init common base (frozen), `/47` = diagnostic taxonomy (frozen), `/48` = probe-attested target baseline (frozen), `/49` = deferred restore/quota>1/VF13/`StorePatch` package. `compiler/contracts/CONTRACT_VERSION` records `/48` hash `820230c4…ad58e`. | Accepted by T01 integrator |

### Stage B3 candidate queue (not dispatched)

| Order | Candidate scope | Gate |
|---|---|---|
| B3-01 | PP19 `#if` macro expansion + independent conditional tests | **COMPLETE / INTEGRATOR VERIFIED** — c22 8/8, c43 18/18, c23 10/10, c24 12/12; original `-DX=1` probe now selects the true branch (`ses_e4b00efb5ffeWgELj4JDqqo9pX`) |
| B3-02 | Integer radix fix (octal/hex/decimal) + independent numeric tests | **COMPLETE / INTEGRATOR IMPLEMENTED** — worker hit capacity; integrator changed `lx_decode.rs`/`lx_classify.rs`, updated c11 hex expectation, added c44; c11 8/8, c44 3/3 |
| B3-03 | Candidate driver PP orchestration for `#define`/`#undef`/`#include`/`#line`/`#pragma` | **B3-03a + B3-03b IMPLEMENTED / VERIFIED** — source `#define`/`#undef` through PP06/PP07/PP08; bounded quoted single-level `#include "..."` beside the including file through PP17/PP18 with canonical sandboxing. `c46_pp_driver` 10/10, `c47_pp_include_driver` 12/12. W3 line/pragma, `-I`, angle/system, nesting, guards/`once` remain blocked. Design: `docs/reviews/dispatch/B3_03_PP_ORCHESTRATION_PLAN.md`; slices: `docs/tasks/PP_DRIVER_SLICE.md`, `docs/tasks/B3_03B_LEAF_INCLUDE_POLICY.md` |
| B3-04 | Canonical PA15 selection; allocate distinct parse IDs; park duplicate | **PARTIALLY DISPOSITIONED by B-INT-00** — both PA15 drafts are parked; `pa_init.rs` is the future candidate, `pa_expr_ext.rs` non-canonical. Wiring still requires the ID/task-kind freeze |
| B3-05 | `ConstRecord.ty` / tagged constant v2 | **COMPLETE / INTEGRATOR VERIFIED** — B2-02 `/42` frozen; `c45_const_ty` 8/8; candidate `value=05` |
| B3-06 | Diagnostic taxonomy migration | Separate diagnostic CCR |
| B3-07 | H05 safe runner (per-instance dirs, timeout, output/resource limits, cancellation, crash class) | **COMPLETE / VERIFIED** — `tools/runner`; `cargo test --manifest-path tools/runner/Cargo.toml` 8 macOS / 10 Linux; see [H05](H05_ISOLATED_RUNNER.md) |
| B3-08 | H07 probe-report classifier over synthetic assembly fixtures | **IMPLEMENTED / REAL RUN VERIFIED** — `tools/torture/probe/classify.py` derives the four ABI fields from `abi-args.s`; `c48_target_probe` 3/3. Official T00 baseline/substrate still open |
| B3-09 | M1 AArch64 assembly prototype (`candidate --emit-asm`) | **COMPLETE / VERIFIED** — constant-return IR emits GNU assembly; native AArch64 Linux assemble/link/run exits 5; `c55_target_m1` 1 macOS / 2 Linux. Not general T11 codegen; `-S`/`-c` still refused |
| B3-10 | M1 automatic-local/integer-arithmetic IR emitter (host-only) | **IMPLEMENTED / VERIFIED at IR level** — `target_m1` accepts automatic `int` locals, `LocalLoad`/`LocalStore`, `Add`/`Sub`/`Mul`/`Div`/`Rem`/`Neg`, and one `Return`; 14 candidate-bin unit tests macOS/Linux; native AArch64 `gcc` executes the add, all-operator, and signed-constant fixtures; frames are bounded to the `sub sp` immediate. No source-to-IR producer yet |

## 3E. B2 Decision Record (B2-01 `/40`, B2-00 `/41`, B2-02 `/42` accepted)

B2-01 was **accepted on 2026-11-09** and implemented as `/40`; B2-00 was
**accepted on 2026-11-09** and implemented as `/41`; B2-02 was **accepted on
2026-11-09** and implemented as `/42`. The rows below are the accepted
defaults. The deferred restore package is `/49`.

| CCR | Decision | Recommendation |
|---|---|---|
| B2-01 O-1 | `ConstRecord.ty` split | Separate B2-02 tagged-constant CCR |
| B2-01 O-2 | Function fact value category | Keep `NonLvalue` for M1 |
| B2-01 O-3 | T09 function link | Node-keyed `SemRecord.node == FunctionDefinition` |
| B2-01 O-4 | Deferred `ConstRecord.ty` visibility | Document in T01/M1; no fake rule marker |
| B2-01 O-5 | VF06 function count | Require exactly one `FunctionDefinition`; fail closed |
| B2-01 O-6 | T09 re-check | Re-check category/effects cheaply |
| B2-01 O-7 | SE29 reuse validation | Optional hardening, not a D-02 blocker |
| B2-01 O-8 | Candidate registry | Use `se_function_slice()` |
| B2-01 O-9 | Zero/multiple definitions | Driver `Err` for M1; typed diagnostic in V1 |
| B2-01 O-10 | Golden evidence | Capture a new `/40` golden; never reuse `/39` |
| B2-01 O-11 | Future `Verification` family | Separate CCR |
| B2-01 O-12 | Interim IR type rule | Add `ir.constant-result-type-from-sem` now |
| B2-00 OD-1 | Scheduler key | Stage-first `(stage, group, enqueue, TaskId)`; amend T01/M1 |
| B2-00 OD-2 | Append snapshot bodies | Full canonical bodies with names/order |
| B2-00 OD-3 | `stage_queue_bound` | Reserve in `/41`; enforce with stage queues/projection |
| B2-00 OD-4 | Zero quota | Remove clamp; typed fail-closed |
| B2-00 OD-5 | `ResultRecord.version` | 1-based global commit ordinal |
| B2-00 OD-6 | Replay meaning | `/41` fingerprint + recorded-input replay; restore deferred `/49` |
| B2-00 OD-7 | Pins | Trace digest + Host envelope |
| B2-00 OD-8 | `Task.id` | Encode and test divergence |
| B2-00 OD-9 | Freeze granularity | **Rejected as written**: use `/40` B2-01, `/41` B2-00, `/42` tagged constants, `/43` parser/AST, `/44` IR/CFG (frozen), `/45` constant v3 (frozen), `/46` type/layout/init (frozen), `/47` diagnostics (frozen), `/48` probe baseline (frozen), `/49` deferred restore |
| B2-00 OD-10 | Dispatch ordinal | Keep out of D-13; quota>1 in `/49` |
| B2-02 T-01 | Constant type field | `ConstRecord.ty` is a committed T06 `TypeId` |
| B2-02 T-02 | Payload family | Closed `/42` tag `Integer` only; float/address append-only future tags |
| B2-02 T-03 | Integer carrier | Big-endian minimal magnitude + sign; no host width |
| B2-02 T-04 | Zero canonicality | Zero is exactly `[0]` with `negative = false` |
| B2-02 T-05 | Binary type source | Committed `SemRecord.ty` of the request node; missing/ambiguous fails |
| B2-02 T-06 | Literal type source | Unique committed canonical `int`; missing/ambiguous fails |
| B2-02 T-07 | Payload/type compatibility | Integer payload requires `TypeKind::Int`; target range stays gated |
| B2-02 T-08 | T09 immediate check | `ConstRecord.ty == ValueRecord.ty` (`ir.op-immediate-type`) |
| B2-02 T-09 | VF12 check | Same immediate type as the returned value |
| B2-02 T-10 | Shape scope | No new task kind/chip/stage/allowlist/arena/store/record family |

## 3F. `/40` Freeze Record (2026-11-09)

| Field | Value |
|---|---|
| Artifact | `t01-c01-c06/40` |
| Hash | `c6961ac65fb3fb7e4ad4fdb666c47584b94042949cb6ed17111a4d81b3b10c14` |
| Authority | B2-01 accepted by the T01 integrator; R1 auto-bump; `/39` preserved as history |
| Contract text | [T01 §7.1 `/40` amendment](T01_COMPILER_CONTRACT.md); [CCR B2-01](CONTRACT_CHANGE_REQUEST_B2_M1_A3_SE29.md) §11 |
| New normative rules | `vf06.function-signature-fact`, `ir.function-consumes-committed-sem`, `ir.constant-result-type-from-sem` |
| Shape impact | None: no new task kind, chip, stage, allowlist row, store, field, record family, `ResultValue`, or `Proposal`; allowlist remains 33 rows; `SemRecord` remains `{node, ty, category, effects}` |
| Implementation | Candidate SE29 dispatch + registry switch; VF06 four-kind per-type table + unique `int(void)` signature projection; T09 consumes `SemRecord.ty` with node-keyed link and `symbol.ty` cross-check; interim IR constant/return type rule |
| Tests | `c14_se` 16, `c15_ir` 13, `c39_sefunc` 8, `c43_a3` 5, `h04_candidate` 18; `c17_vf12` 6 and `c19_vf01` 7 fixtures aligned to SE29; full compiler suite 700 tests / 50 suites; root 21; torture 73; chip-lint 16; formatting clean |
| Determinism evidence | Three direct candidate runs on `int main(void){return 2+3;}` were byte-identical, exited 0, and emitted `candidate evidence (t01-c01-c06/40)` and `interpret: value=05 negative=false` |
| Explicit deferrals | D-09 (`ConstRecord.ty`/`ir.op-immediate-type`) to B2-02; O-7 SE29 reuse-hardening not in `/40`; B2-00 determinism/limits/replay to `/41` |
| Non-claims | No full M1 acceptance, no target ABI/probe result, no GCC torture rate, no target execution; clippy remains red only in the D-04 parse drafts |

## 3G. `/41` Freeze Record (2026-11-09)

| Field | Value |
|---|---|
| Artifact | `t01-c01-c06/41` |
| Hash | `500abb528a1e62622f6d327456d2e27f6bf1abdc1992e6b991e9dd3e0e832966` |
| Authority | B2-00 accepted by the T01 integrator with OD-1..OD-10; R1 auto-bump; `/40` preserved as history |
| Contract text | [T01 §7.1 `/41` amendment](T01_COMPILER_CONTRACT.md); [CCR B2-00](CONTRACT_CHANGE_REQUEST_B2_DETERMINISM_LIMITS.md) accepted remap/§11 |
| New normative rules | `snapshot.append-batch-handles-encoded`, `snapshot.append-batch-bodies-encoded`, `snapshot.append-batch-order-preserved`, `snapshot.task-self-id-encoded`, `trace.pins-digest-recorded`, `trace.canonical-ids-recorded`, `trace.decoder-round-trip`, `trace.first-divergence`, `replay.recorded-input-model`, `task.select.stage-phase-enqueue-taskid`, `dispatch.stage-assignment-authoritative`, `limits.declared-bounds-enforced`, `limits.stage-queue-bound-reserved`, `limits.total-records-includes-macros`, `limits.zero-quota-fail-closed`, `commit.append-bumps-store-version`, `commit.names-bump-on-new-only`, `commit.result-version-meaningful` |
| Removed rules | `task.select.phase-enqueue-taskid`, `limits.every-configured-bound-enforced` (replaced, not aliased) |
| Shape impact | No new task kind, chip, stage row, allowlist row, arena, store, or record family; `Control.next_commit_ordinal` and the richer `TraceRecord` are the only new observable fields; `stage_queue_bound` stays reserved |
| Implementation | Append handles/bodies in snapshots; stage-first selection + zero-quota typed error; macro total; typed-append and Names store versions; 1-based result commit ordinal; `Task.id`; pin digest; bounded trace decoder/diff; Host `ReplayEvidence` |
| Tests | `c05_codec` 29, `c06_routing` 19, `c07_limits` 20, `c03_task` 48, `c08_gate1` 22, `c11_lex` 11, `freeze` 15, `c20_ppscan` 8, `h04_candidate` 18; full compiler suite 723 tests / 50 suites; root 21; torture 73; chip-lint 16; formatting clean |
| Determinism evidence | Three direct candidate runs on `int main(void){return 2+3;}` were byte-identical, exited 0, and emitted `candidate evidence (t01-c01-c06/41)` and `interpret: value=05 negative=false` |
| Explicit deferrals | Full `Snapshot::from_bytes`/restore, VF13 projections, quota>1 dispatch/commit order, per-stage queues/bounds, `StorePatch` materialization, full pin bytes → `/49` (was `/43` before the schema train); D-09 tagged constants → `/42` |
| Non-claims | No full M1 acceptance, no target ABI/probe result, no GCC torture rate, no target execution; clippy remains red only in the D-04 parse drafts |

## 3H. `/42` Freeze Record (2026-11-09)

| Field | Value |
|---|---|
| Artifact | `t01-c01-c06/42` |
| Hash | `a9a1da92b5e7559cd6b3cfc025553dcd5148b96632cd769381e084bda9abf1a5` |
| Authority | B2-02 accepted by the T01 integrator with T-01..T-10; R1 auto-bump; `/41` preserved as history |
| Contract text | [T01 §7.1 `/42` amendment](T01_COMPILER_CONTRACT.md); [CCR B2-02](CONTRACT_CHANGE_REQUEST_B2_TAGGED_CONSTANTS.md) §11 |
| New normative rules | `const.record-type-committed`, `const.record-type-from-sem`, `const.integer-payload-type-int`, `const.zero-canonical`, `ir.op-immediate-type`, `snapshot.const-record-typed-body` |
| Shape impact | `ConstRecord` body is now `{ty, value}` with closed `ConstValue::Integer`; new hashed tables `CONST_RECORD_FIELDS`, `CONST_VALUE_NAMES`, `INTEGER_CONST_FIELDS`; no new task kind, chip, stage row, allowlist row, arena, store, or record family |
| Implementation | Typed fold resolution from committed `SemRecord.ty` (binary) / unique canonical `int` (literal); canonical integer codec; T09 immediate-type check; VF12 same-type check; VF14 integer gate decode; candidate integer evidence extraction |
| Tests | `c45_const_ty` 8, `c08_gate1` 22, `c15_ir` 13, `c17_vf12` 6, `c38_vf14` 10, `c05_codec` 29, `freeze` 15; full compiler suite 684 tests / 51 suites after B-INT-00 parking |
| Determinism evidence | Three direct candidate runs on `int main(void){return 2+3;}` were byte-identical, exited 0, and emitted `candidate evidence (t01-c01-c06/42)` and `interpret: value=05 negative=false` |
| Explicit deferrals | Float/address constant tags; full signed-range formula; `Snapshot::from_bytes`/restore, VF13, quota>1, `StorePatch` materialization → `/49` (was `/43` before the schema train) |
| Non-claims | No full M1 acceptance, no target ABI/probe result, no GCC torture rate, no target execution |

## 3I. `/43` Freeze Record (2026-11-09)

| Field | Value |
|---|---|
| Artifact | `t01-c01-c06/43` |
| Hash | `637a0484a4e1870b8e66b04dfd5ad79921d0ad1736972ce7f6a90243bce8df30` |
| Authority | ADR-0003 accepted by the T01 integrator; R1 auto-bump; `/42` preserved as history |
| Contract text | [T01 §7.1 `/43` amendment](T01_COMPILER_CONTRACT.md); [ADR-0003](../architecture/ADR-0003-PARSER-ARCHITECTURE-AND-V1-AST.md) |
| New names | `declaration`, `identifier`, `expression_statement`, `assignment`, `binary_sub`, `binary_mul`, `binary_div`, `binary_rem`, `unary_plus`, `unary_minus` in `NODE_KIND_NAMES` |
| Shape impact | `NodeKind` append only; `NodeRecord` unchanged; no new task kind, chip, stage, allowlist, arena, store, or record family; no `NodeLinkRecord`/extended `ParseContext` in V1 |
| Implementation | Single-task V1 parser decision recorded; append-only `NodeKind` inventory; snapshot encode/decode names; VF05 fails closed on V1 kinds until its shape table lands |
| Tests | `c49_ast_schema` 2; full compiler suite 710 tests / 55 suites |
| Explicit deferrals | V1 parser producer, VF05 V1 shape table, V2 taskified parser choice, SemRecord conversion/constant/symbol extensions |
| Non-claims | No V1 parser, full C AST, target codegen, probe, or torture result |

## 3J. `/44` Freeze Record (2026-11-09)

| Field | Value |
|---|---|
| Artifact | `t01-c01-c06/44` |
| Hash | `73131cdfd251802c33756a7434d1b89aa2ae71f8a62c0fe8098ecdd57acae0f7` |
| Authority | ADR-0004 accepted by the T01 integrator; R1 auto-bump; `/43` preserved as history |
| Contract text | [T01 §7.1 `/44` amendment](T01_COMPILER_CONTRACT.md); [ADR-0004](../architecture/ADR-0004-IR-CFG-SCHEMA.md) |
| New names | `local_load`, `local_store`, `add`, `sub`, `mul`, `div`, `rem`, `neg`, `branch`, `cond_branch`; `INSTRUCTION_RECORD_FIELDS` appends `local`, `successors` |
| Shape impact | `InstructionRecord` appends `local`/`successors`; `EFFECT_WRITES_LOCAL`; no task kind, chip, stage, allowlist, arena, store, or record family |
| Implementation | Append-only op/field schema, snapshot round-trip, contract hash; M1 producers use `local: None, successors: vec![]` |
| Tests | `c50_ir_schema` 2; full compiler suite 712 tests / 56 suites |
| Explicit deferrals | V1/V2 lowering producers, VF07/VF08 CFG/local verification, memory/call/object payloads |
| Non-claims | No new IR producer, CFG verifier, target codegen, or torture result |

## 3K. `/45` Freeze Record (2026-11-09)

| Field | Value |
|---|---|
| Artifact | `t01-c01-c06/45` |
| Hash | `c4e84ccbd756b15faa9415192636ed93105d339e005e3b9e9d18a3f8cf30bac8` |
| Authority | ADR-0005 accepted by the T01 integrator; R1 auto-bump; `/44` preserved as history |
| Contract text | [T01 §7.1 `/45` amendment](T01_COMPILER_CONTRACT.md); [ADR-0005](../architecture/ADR-0005-CONSTANT-V3.md) |
| New tags/tables | `ConstValue::Float`/`Address`; `FLOAT_CONST_FIELDS`, `ADDRESS_CONST_FIELDS`; rules `const.float-format-bits`, `const.address-symbolic`, `snapshot.const-v3-tags` |
| Shape impact | `ConstValue` append only; no new task kind, chip, stage, allowlist, arena, store, or record family |
| Implementation | Canonical float/address codec, format bit-length validation, VF14 non-integer gate rejection; no producer |
| Tests | `c51_const_v3` 3; full compiler suite 715 tests / 57 suites |
| Explicit deferrals | Float/address producers, T06 float/pointer type compatibility, layout/init records |
| Non-claims | No target width, layout, codegen, probe, or torture result |

## 3L. `/46` Freeze Record (2026-11-09)

| Field | Value |
|---|---|
| Artifact | `t01-c01-c06/46` |
| Hash | `3416b176c23f3802e2c57978db74f89bcc7719337c968c0c12c9d7b19aa6972f` |
| Authority | ADR-0006 accepted by the T01 integrator; R1 auto-bump; `/45` preserved as history |
| Contract text | [T01 §7.1 `/46` amendment](T01_COMPILER_CONTRACT.md); [ADR-0006](../architecture/ADR-0006-TYPE-LAYOUT-INIT-SCHEMA.md) |
| New names | `pointer`/`array`/`aggregate`/`enum`; qualifier/array-bound/aggregate-member/enumerator tables; `G1DraftBody::Layout`/`Init` |
| Shape impact | `TypeKind` append; layouts/inits become typed arenas; snapshot bodies replace reserved counts; no new task kind, chip, stage, or allowlist row |
| Implementation | Type/layout/init canonical codecs and append handles; no producer |
| Tests | `c52_type_schema` 3, `c53_layout_init_schema` 2; full compiler suite 720 tests / 59 suites |
| Explicit deferrals | T06 type producers, T08 layout/init producers, target byte layouts, VF07/layout validation |
| Non-claims | No target layout, codegen, probe, or torture result |

## 3M. `/47` Freeze Record (2026-11-09)

| Field | Value |
|---|---|
| Artifact | `t01-c01-c06/47` |
| Hash | `3c3d722def942d229e95d15f8e6758ee88f9a71720b170b3751bebf38245f593` |
| Authority | ADR-0007 accepted by the T01 integrator; R1 auto-bump; `/46` preserved as history |
| Contract text | [T01 §7.1 `/47` amendment](T01_COMPILER_CONTRACT.md); [ADR-0007](../architecture/ADR-0007-DIAGNOSTIC-TAXONOMY.md) |
| New groups/codes | `lex`, `parse`, `type`, `const`, `ir`, `codegen`, `host`; `DIAGNOSTIC_CODES` named registry; rules `diag.taxonomy-stable`, `diag.code-unique`, `diag.span-task-attribution` |
| Shape impact | Group/code registry append; `DiagnosticRecord` shape unchanged; no task kind, chip, stage, allowlist, arena, or store |
| Implementation | Named code constants and structured helpers; registry hashed by name/group/code |
| Tests | `c54_diag_taxonomy` 3; full compiler suite 723 tests / 60 suites |
| Explicit deferrals | Full `Task:4` call-site migration, per-fault span/task enrichment, diagnostic UI/renderer |
| Non-claims | No full migration, no target codegen, no torture result |

## 3N. `/48` Freeze Record (2026-11-09)

| Field | Value |
|---|---|
| Artifact | `t01-c01-c06/48` |
| Hash | `820230c41c37ec4ce12d7517172b767d837211b4ef638c6b7a2f2d111c1ad58e` |
| Authority | T01 integrator acceptance; R1 auto-bump; `/47` preserved as history |
| Contract text | [T01 §7.1 `/48` amendment](T01_COMPILER_CONTRACT.md); [probe run record](../reviews/target-probe/2026-11-09_DOCKER_AARCH64_PROBE.md) |
| Baseline | Debian 12 arm64 image `sha256:268fb24c…3ff8`, GCC 12.2.0, binutils 2.40; attested report hash `57eadb28…c4f90` |
| Shape impact | `target_verification=probe-attested`, `probe_substrate=docker-linux-arm64`; target model values unchanged |
| Implementation | `TargetSpec::aarch64_unknown_linux_gnu_probe_attested()`; fixture + `c48_target_probe`; H07 classifier |
| Tests | `c48_target_probe` 3; full compiler suite 725 tests / 61 suites (726 on native AArch64 Linux) |
| Explicit deferrals | Official T00 CI/bare-metal reference baseline, corpus freeze, HVA/mixed-linking, target codegen |
| Non-claims | No target execution, codegen, or torture rate |

## 4. Dispatch Rules for This Batch

1. A worker may write only its report path; reports are English markdown.
2. A worker must cite source paths/lines and distinguish implemented behavior, frozen
   contract, proposed work, and unverified claims.
3. No report may claim a test, probe, corpus run, pass rate, or implementation that
   was not actually performed.
4. A worker that discovers a blocking interface conflict returns a structured defect
   in its report; it must not edit shared schemas.
5. The master dispatcher reviews every report before a Stage-B freeze, deduplicates
   requirements, and assigns implementation work only to frozen interfaces.
6. User-owned uncommitted files are read-only for this entire batch.

## 5. Review and Next Gate

- Stage A reports are collected and classified; B1 safe fixes and B3-01/B3-02 are
  verified. B2-01 is accepted, implemented, and verified as `/40`; B2-00 is
  accepted, implemented, and verified as `/41`; B2-02 is accepted, implemented,
  and verified as `/42`.
- **Next freeze train:** `/49` remains the deferred
  restore/quota>1/VF13/`StorePatch` package. Each needs an explicit integrator
  decision, implementation, dependent tests, and a recomputed hash.
- **Next integration blockers:** a source-to-IR producer for automatic locals
  and arithmetic beyond the M1 constant-return path, general T11 codegen,
  official T00 corpus/runner/aggregation, B3-03c
  `#line`/`#pragma`, wider H04 `-I`/angle/system/multi-source freeze, canonical
  PA15 allocation/wiring (B3-04), and full legacy diagnostic migration. H05
  process supervision is implemented; namespace sandboxing and target
  orchestration remain future work. B-INT-00 closed the draft/clippy blocker;
  B3-03a/B3-03b implemented source macros and bounded quote include.
- Start implementation only for packages whose readiness gates are satisfied, and run
  package-specific `fmt`, tests, and attributed clippy before any integration claim.
