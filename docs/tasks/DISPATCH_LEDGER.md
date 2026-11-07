# Dispatch Ledger

| Field | Value |
|---|---|
| Status | **Active dispatch record; not a contract freeze or completion claim** |
| Updated | 2026-11-07 |
| Master dispatcher | Integrator role |
| Plan | [MASS_PARALLEL_DISPATCH.md](MASS_PARALLEL_DISPATCH.md) |
| Contract baseline | `t01-c01-c06/39` |
| Branch / HEAD | `wave4-development` / `430ae29` |
| Workspace note | Uncommitted user candidate files and freeze drafts are present and are not part of this dispatch batch |

## 1. Baseline Checks

| Check | Result | Notes |
|---|---|---|
| Root package tests | **PASS**, 21 tests across 3 suites | Observed in this dispatch session |
| Compiler package tests | **PASS**, 654 tests across 44 suites at the start; **682 tests across 49 suites after B1/B3** | Observed in this dispatch session; warnings remain from unregistered parse/lex drafts. The final run includes c40/c41/c42/c43/c44 and the updated c11 hex expectation. |
| Root/compiler/torture/chip-lint formatting | **PASS** | `cargo fmt --all -- --check` and all three manifest-path checks after B1/B3 |
| Root tests after B1/B3 | **PASS**, 21 tests | `cargo test --workspace` |
| Torture package after B1/B3 | **PASS** | `cargo test --locked --manifest-path tools/torture/Cargo.toml` |
| Chip-lint after B1-05 | **PASS**, 16 tests | `cargo test --locked --manifest-path tools/chip-lint/Cargo.toml`; clippy clean |
| Clippy | **FAIL in the current dirty workspace** | `cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings` fails with 137 errors, mostly dead-code/`vec_init_then_push` in uncommitted parse drafts. This is not a HEAD verdict; committed HEAD CI remains separately unverified. |
| Target probe | **NOT RUN / UNVERIFIED** | No ABI or execution claim |
| GCC torture corpus | **NOT FROZEN / NOT RUN** | No pass-rate claim |

The compiler warnings are not accepted as a green baseline signal. They identify
draft files that are compiled through `parse/mod.rs` but not registered or integrated.

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
| A-PTS-07 | T13 verification and evidence requirements | `ses_e4b30c6abffdjF9dd1iHy9meKZ` | `docs/reviews/dispatch/A17_VERIFICATION_REQUIREMENTS.md` | REPORT PRESENT — worker hit capacity after write; integrator review pending |
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
| D-02 | **P0** | SE29 `FunctionDefinition` `SemRecord` is not dispatched by the candidate; VF06 checks only three node kinds. M1 A3 is not complete. | A05 §3.2 G3; `candidate.rs:252-289`, `vf_invariant.rs:180-184` | B2 after A17 review |
| D-03 | **P0** | Candidate panics (exit 101) on non-UTF-8 argv; no structured driver error. | A10 §1.3 F-01; A24 L-01 | B1-02 |
| D-04 | **P1** | Current dirty-tree clippy gate fails with 142 diagnostics from five declared parse drafts; four additional drafts are not compiled at all. | A13 §5-6 | Integrator freezes draft disposition |
| D-05 | **P1** | Duplicate candidate local 27 / `ChipId(58)` across several drafts; PA15 has two implementations (`pa_init.rs`, `pa_expr_ext.rs`). | A04 §7; A13 §5 | A12/A13 ownership map |
| D-06 | **P1** | All 65 chips use local `Worker`/`handle(&CompilerBus)`, not `RestrictedChip`/`ChipAdapter`; contract has two competing chip boundaries. | A02 F01 | Contract freeze decision |
| D-07 | **P1** | H01 probe never ran; H07 classification and attestation handoff absent; four ABI fields unresolved. | A05 §2.3; A09 | Target track, blocked |
| D-08 | **P1** | H05 isolated execution is absent; no timeout/resource/cancellation/crash boundary. | A10; A24 H-01 | H05 work order after security review |
| D-09 | **P1** | `ConstRecord` has no `ty`; documented immediate-type rule cannot be enforced. A14 proposes a tagged v2 record. | A05 G4; A14 §4.2 | Schema freeze |
| D-10 | **P2** | Status docs still claim `/6`, `/7`, `/9`, `/16`, "no chips", "torture not in CI", and stale H04/VF06 status. | A01 §3.3; A13 §8 | B1-03 |
| D-11 | **P2** | `-std`/`-O` are validated but not propagated; exit code 2 conflates usage/unsupported/Part B refusal; output writes are non-atomic. | A10 §1.3 F-02..F-04, F-12 | B2 candidate hardening |
| D-12 | **P0** | `AppendRecords` wire proposals encode only the draft count in snapshots; different append batches with equal counts hash identically. This contradicts `snapshot.wire-payloads-encoded`. | A23 A23-F01; `snapshot.rs:446-455` | Contract freeze B2 |
| D-13 | **P1** | Scheduler order uses task-group ordinal, not `STAGE_ASSIGNMENT`; T01 three-key order conflicts with M1 D5 four-key stage-first order. | A23 finding 5; `routing.rs:34-38` | Contract decision B2 |
| D-14 | **P1** | `allocated_total()` omits macros; `stage_queue_bound` is unused; zero quota is clamped; typed appends do not bump `store_versions`; `ResultRecord.version` is always 0. | A23 findings 14-16; A02 F04 | Foundation fix B2 |
| D-15 | **P1** | `Snapshot`/`Trace` have no restore/decoder/diff; "replay" tests re-run fresh buses and pins are not recorded. | A23 finding 2 | Replay contract B2 |
| D-16 | **P1** | Diagnostic codes are ad hoc: `Task:4` has 339 sites and `Unsupported:1` 106; capacity, invalid, and verification faults lack distinct groups/codes; spans/task attribution are rare. | A18 §1 | Diagnostic CCR B2 |
| D-17 | **P0** | `#if X` does not macro-expand identifiers; a `-DX=1` probe takes the false branch, silently selecting the wrong conditional branch. | A03 §2.3, probe in §1.2 | **FIXED / VERIFIED** by B3-01 for object-like macros; function-like invocations remain explicit Unsupported |
| D-18 | **P0** | Integer `010` is classified/decode as decimal 10 instead of octal 8; radix/suffix/type/range handling is absent. | A03 §2.5 | **FIXED / VERIFIED** for decimal/octal/hex by B3-02; suffix/type/range remain future |
| D-19 | **P1** | Candidate runs only PP01–04/19/09/05/26/28; `#define`, `#include`, `#line`, `#pragma` still fail at PP05. | A03 §2.2 | PP driver orchestration B3 |
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
| B2-00 | Determinism/limits/replay cluster D-12..D-15: append snapshot bodies, scheduler order conflict, limit accounting, store versions, replay restore/diff. | `docs/tasks/CONTRACT_CHANGE_REQUEST_B2_DETERMINISM_LIMITS.md` | **CCR SUBMITTED** — mandatory `/40`-class package + deferred replay/restore; decision pending |
| B2-01 | M1 A3 closure D-02: wire SE29, extend VF06 function-signature checking, consume FunctionDefinition fact in IR; state `ConstRecord.ty` dependency. | `docs/tasks/CONTRACT_CHANGE_REQUEST_B2_M1_A3_SE29.md` | **CCR SUBMITTED** — no new shared shapes; proposed `/40`; decision pending |

## 3D. Stage A Exit Summary

All 24 Stage A reports are present. The reports are independent read-only audits or
design packages; presence is not acceptance of their proposals. The integrator has
triaged the critical path as follows.

### Confirmed critical path

1. **Finish B1 safe fixes** (PA01 exact gate, candidate argv, docs, wiring guards).
2. **Resolve B2 contract decisions** (determinism/limits/replay; M1 A3 SE29) and freeze
   the smallest `/40` delta. D-12 (append snapshot bodies) and D-02 (SE29/VF06/IR) are
   the highest-value correctness items.
3. **Fix PP/lex wrong-result defects** before claiming V1 evidence:
   D-17 (`#if` macro expansion), D-18 (octal/radix), D-19 (driver PP orchestration),
   D-20 (literal token kinds).
4. **Then** choose the canonical PA15 implementation and allocate distinct parse/lex
   IDs; only after that wire V1 AST/type/semantic/IR producers.
5. **Target track stays blocked** until H01 probe + H07 classification + T01
   attestation. Do not implement ABI values or `-S`/`-c` from proposals.
6. **H05 stays blocked on an accepted isolation design** (A10/A24); no untrusted
   execution before then.

### Provisional Integrator Decisions (not frozen)

| ID | Decision / recommendation | Status |
|---|---|---|
| ID-PARSER-01 | Use A06's single-task `PARSE_TU` bootstrap for V1 only. Do not add `NodeLinkRecord`, `SetFrame`, or a taskified parser in V1. Before V2, freeze either (a) the same single parser extended to V2 or (b) a taskified parser; retire the losing path. Never run two AST producers. A06 independently recommends this bootstrap; A07 records the V2 conflict. | Recommendation pending owner/T01 acceptance |
| ID-IR-01 | Prefer A15's `InstructionRecord.successors` + `Branch`/`CondBranch` over A07's separate `CfgEdgeRecord`; keep the edge family only as a documented fallback. | Recommendation pending IR freeze |
| ID-TARGET-01 | All concrete ABI/scalar values remain probe-gated. No target codegen or `-S`/`-c` from proposal values. | Existing accepted boundary |
| ID-CHIP-01 | Do not add new chips until the Worker-vs-RestrictedChip template decision (A02 F01) is resolved; B1/B3 fixes may continue because they touch existing files. | Recommendation pending contract decision |
| ID-VERSION-01 | Both B2 CCRs propose `/40`. Recommended sequence: `/40` = M1 A3 SE29 closure (no new shared shapes), `/41` = determinism/limits mandatory package, `/42` = tagged constants (`ConstRecord.ty`). Exact hash remains integrator-computed after owner acceptance. | Recommendation pending owner/T01 acceptance |

### Stage B3 candidate queue (not dispatched)

| Order | Candidate scope | Gate |
|---|---|---|
| B3-01 | PP19 `#if` macro expansion + independent conditional tests | **COMPLETE / INTEGRATOR VERIFIED** — c22 8/8, c43 18/18, c23 10/10, c24 12/12; original `-DX=1` probe now selects the true branch (`ses_e4b00efb5ffeWgELj4JDqqo9pX`) |
| B3-02 | Integer radix fix (octal/hex/decimal) + independent numeric tests | **COMPLETE / INTEGRATOR IMPLEMENTED** — worker hit capacity; integrator changed `lx_decode.rs`/`lx_classify.rs`, updated c11 hex expectation, added c44; c11 8/8, c44 3/3 |
| B3-03 | Candidate driver PP orchestration for `#define`/`#undef`/`#include`/`#line`/`#pragma` | H04 include policy + T03/T02 co-freeze |
| B3-04 | Canonical PA15 selection; allocate distinct parse IDs; park duplicate | Integrator ID/task-kind freeze |
| B3-05 | `ConstRecord.ty` / tagged constant v2 | B2-00/B2-01 CCR + version bump |
| B3-06 | Diagnostic taxonomy migration | Separate diagnostic CCR |
| B3-07 | H05 safe runner (per-instance dirs, timeout, output/resource limits, cancellation, crash class) | Accepted A24-SEC-01 design |
| B3-08 | H07 probe-report classifier over synthetic assembly fixtures | H01 report schema frozen; no real probe claim |

## 3E. B2 Decision Draft (not a freeze)

Recommended answers to the CCR open questions, pending owner/T01 sign-off:

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
| B2-00 OD-6 | Replay meaning | `/41` fingerprint + recorded-input replay; restore deferred |
| B2-00 OD-7 | Pins | Trace digest + Host envelope |
| B2-00 OD-8 | `Task.id` | Encode and test divergence |
| B2-00 OD-9 | Freeze granularity | **Rejected as written**: use `/40` B2-01, `/41` B2-00, `/42` tagged constants |
| B2-00 OD-10 | Dispatch ordinal | Keep out of D-13; quota>1 in `/42+` |

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

- Collect A1 reports, then A2 reports.
- Classify each finding as fact, defect, proposal, or unverified.
- Update the ownership map and produce the first Stage-B freeze package.
- Start implementation only for packages whose readiness gates are satisfied.
- Run package-specific `fmt`, `clippy`, and tests before any integration claim.
