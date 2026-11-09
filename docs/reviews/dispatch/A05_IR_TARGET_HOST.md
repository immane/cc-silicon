# A-AUD-05: IR Lowering, Verification, Target Model, Candidate Driver, and Torture/Probe Audit

| Field | Value |
|---|---|
| Status | **AUDIT COMPLETE — READ-ONLY.** No source, test, contract, Cargo, `mod.rs`, ledger, or shared index was modified. No commit or push. The only repository file written is this report (ignored build output and test temp dirs are not source changes). |
| Date | 2026-11-07 |
| Task | Stage A, A-AUD-05 under [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) |
| Checked revision | `430ae29162efef90f86667f384097964d611604f` (2026-11-07T14:52:26+08:00, `docs: add mass-parallel compiler dispatch plan`), branch `wave4-development` |
| Contract baseline | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24`; `target_verification=unverified` |
| Workspace state | At audit start: 18 entries — 1 modified (`compiler/src/chips/parse/mod.rs`, +5 `mod` lines) and 17 untracked user files (2 lex drafts, 7 parse drafts, 8 freeze/deferred drafts). Before this report was written, `docs/tasks/DISPATCH_LEDGER.md` appeared concurrently (19 entries); it was not touched by this audit. Status hash at `2026-11-07T07:07:13Z`: `d8c13a184191aa4ff35119ff8c80f846433a7ef796397acf2d8a47ce8b03994a`. |
| Read-only commands | Targeted only; see §2.4. The real Linux AArch64 probe, target assembly/link/run, full compiler suite, formatting, Clippy, and any GCC torture run were **not run by this audit**. |

**Label discipline used below:** **IMPLEMENTED** = code exists and the cited targeted check was actually run in this audit; **FROZEN CONTRACT** = part of the `/39` artifact (the artifact hash covers normative shapes/rule IDs, not source semantics); **PROPOSED/PLANNED** = task catalog, draft, or unaccepted contract; **UNVERIFIED CLAIM** = requires a probe, target execution, corpus, or a test not run here.

---

## 1. Executive Result

The repository currently has a **working, deterministic M1 Part A evidence path for the single fixture `int main(void){return 2+3;}`**. The candidate binary drives real source bytes through the implemented preprocessing, lexing, parsing, typing, semantic, const-fold, minimal IR-lowering, and verification slices and prints snapshot/trace hashes plus `interpret: value=05 negative=false`. The cited targeted tests pass.

It does **not** have a target compiler. There is no T11 target-code directory, no AArch64 instruction selection, no machine IR, no register allocation/frame layout, no assembly emitter, no object/ELF writer, no assembler/linker integration, no isolated runner, and no target execution. The target identity is frozen but every concrete value remains unverified; the H01 probe harness has never run; `-S`/`-c` are explicitly refused while the target is unverified.

Consequently:

- **M1 Part A:** substantial and demonstrated for the M1 fixture, with recorded gaps against the M1 acceptance text (notably the `FunctionDefinition` `SemRecord`/SE29 path and the `ConstRecord.ty` rule).
- **M1 Part B / real closed loop:** **not achieved and blocked by target evidence** (H01 → H07 → T01 attestation → T11 → assembler/linker → runner).
- **M1 acceptance overall:** **not passed**; `docs/tasks/M1_TARGET_ACCEPTANCE.md` itself records every check as NOT RUN.
- **GCC torture pass rates:** none; no corpus lock, no official driver integration, no isolated runner, no denominator.

---

## 2. Capability and Evidence

### 2.1 Implemented slices relevant to this audit

| Area | Classification | Implementation and exact locations | Targeted evidence actually run in this audit | Scope limit / what it does not prove |
|---|---|---|---|---|
| T09 IR lowering (fused M1 slice) | **IMPLEMENTED** (M1 slice; `/15`) | `compiler/src/chips/ir_lower/ir_function.rs:1-13` documents IR01/02/03/19 scope; projection at `:55-255`; worker/manifest at `:257-305`; pure lowering at `:307-386`. It reads the `FunctionDefinition`, declarator, one symbol, canonical `int`, committed `Return` + `BinaryAdd` `SemRecord`s, and exactly one folded `ConstRecord`; appends one `Function`, one `Block`, one `Value`, and exactly `Constant` + `Return` (`:321-384`). It performs no arithmetic and carries the fold's `ConstId` verbatim (`:226-241`, `:363-376`). | `cargo test --manifest-path compiler/Cargo.toml --test c15_ir` → **7 passed**; the M1 test is `ir_lowerrs_m1_cl05_without_refolding` at `compiler/tests/c15_ir.rs:476-577`. | Only the M1 function shape. No general CFG, memory operations, calls, variables, aggregate copies, `FunctionEnd`/IR28, or `TerminatorMissing` hook. The `Add` IR op is not in the frozen `IrOp` enum (`compiler/src/bus.rs:442-447`). `c15` seeds PP records directly (`compiler/tests/c15_ir.rs:180-249`), so it does not prove preprocessing. |
| VF06 typed-AST invariant | **IMPLEMENTED** (M1 checked set; `/14`) | `compiler/src/chips/verify/vf_invariant.rs:1-12`; projection `:39-127`; check at `:170-239`. It checks exactly `IntLiteral`, `BinaryAdd`, and `Return` (`:180-184`) for exactly one committed `SemRecord`, canonical `int`, `NonLvalue`, and `EffectMask(0)` (`:188-233`), then completes `Ack`. | `cargo test --manifest-path compiler/Cargo.toml --test c14_se` → **9 passed**; VF06 test at `compiler/tests/c14_se.rs:651-704`. | It does **not** check `FunctionDefinition`, `Declarator`, `Specifiers`, `Compound`, or the translation-unit node. Its generic `sem.ty == int` rule would also reject a `FunctionDefinition` `SemRecord` carrying a function-signature `TypeId`. No conversion-completeness matrix exists in the frozen `SemRecord` shape (`compiler/src/bus.rs:496-513`). |
| VF12 symbolic IR interpreter | **IMPLEMENTED** (M1 shape model; `/17`) | `compiler/src/chips/verify/vf_interpret.rs:1-14`; projection `:47-199`; shape checks `:267-320`; returns the committed `RecordRef::Const` at `:321-324`. | `cargo test --manifest-path compiler/Cargo.toml --test c17_vf12` → **6 passed**; `vf12_models_return_5` at `compiler/tests/c17_vf12.rs:486-518`. | It does **not** evaluate arithmetic or independently derive `5`; it verifies the single entry block / `Constant` then `Return` shape and returns the already-folded `ConstId`. Its file header says “no arithmetic” (`:5-13`). It is not target execution. |
| VF05 token-AST invariant | **IMPLEMENTED** (M1 8-kind set; `/18`) | `compiler/src/chips/verify/vf_syntax.rs:1-12`; projection `:40-124`; checks `:184-306`; closed node table at `:224-233`; unique EOF at `:194-211`. | `cargo test --manifest-path compiler/Cargo.toml --test c18_vf05` → **6 passed**; positive test `vf05_accepts_m1_tree` at `compiler/tests/c18_vf05.rs:300-330`. | Closed M1 AST only. Wider syntax requires a new slice and hash rules. |
| VF01 store invariant | **IMPLEMENTED** (M1 ownership/bounds; `/19`) | `compiler/src/chips/verify/vf_store.rs:1-15`; projection `:113-207`; checks `:322-415`; reserved stores at `:333-335`; payload/result links `:336-387`; spans `:388-410`. | `cargo test --manifest-path compiler/Cargo.toml --test c19_vf01` → **7 passed**; `vf01_accepts_healthy_bus` at `compiler/tests/c19_vf01.rs:482-500`. | Tombstone liveness is deferred; `HostRequest` references are out of M1 scope; this is a snapshot-shape check, not a lifetime proof. |
| VF14 evidence classifier | **IMPLEMENTED chip; not wired to candidate** (`/38`) | `compiler/src/chips/verify/vf_evidence.rs:220-259` classification; carrier map `:263-270`; projection `:329-412`; compute `:487-528`. PASS → `Ack`; fail/missing/undecodable/invalid stage → one typed `Fail`. | `cargo test --manifest-path compiler/Cargo.toml --test c38_vf14` → **10 passed**; e.g. `compile_ok_run_fail_is_fail_run_never_pass` at `compiler/tests/c38_vf14.rs:216-254`. | The H06 evidence producer does not exist. The chip classifies caller-supplied pins; it does not obtain compile/link/run evidence itself. Carrier limitations `DEFECT-VF14-01/02` are documented at `vf_evidence.rs:24-36`. |
| VF02/VF03/VF04, VF07–VF11, VF13 | **ABSENT (planned)** | T13 table rows at `docs/tasks/T13_VERIFICATION_CHIPS.md:68-83`; VF07–VF09 are explicitly outside M1 (`M1_TARGET_ACCEPTANCE.md:174-176`); the untracked `docs/tasks/VF13_FREEZE.md:1-15` is a **DRAFT ONLY** proposal for quota-1 VF13 and freezes nothing. | No chip or test exists to run. | No task-graph, wire-lifetime, access-contract, CFG, typed-IR, SSA, machine-IR, ABI, or replay verifier. VF08 is a catalog checker only, not an M1 pass. |
| Candidate driver, Part A | **IMPLEMENTED (partial H04)** | `compiler/src/bin/candidate.rs:1-19` scope; flag parsing `:88-212`; registration/routing `:214-295`; bounded step/drain `:299-366`; real PP prefix `:572-680`; full evidence run `:690-889`; output `:939-957`. It uses `TargetSpec::aarch64_unknown_linux_gnu_unverified()` and `Limits::fixture()` (`:577-583`). | Direct run: `compiler/target/debug/candidate <(printf 'int main(void){return 2+3;}\n')` → exit 0, `candidate evidence (t01-c01-c06/39)`, snapshot `833659dd…`, trace `8b466fad…`, `interpret: value=05 negative=false`. Three repeated runs produced identical snapshot/trace values. `cargo test --manifest-path compiler/Cargo.toml --test h04_candidate` → **18 passed**. | Source-bytes-to-evidence only. No target code. The direct run used process-substitution input `/dev/fd/11`, so the recorded input path is not `main.c`; the fixture bytes are identical. |
| Candidate `-E` / `-D` / `-U` | **IMPLEMENTED** | `candidate.rs:108-110` `-E`; `:142-178` `-D`/`-U`; `:479-563` macro seed commit; `:685-688` PP28 emit; helper `compiler/src/bin/candidate/emit_helper.rs:38-62`; macro parser `compiler/src/bin/candidate/macro_flags.rs`. | Direct `-E` run on the M1 fixture → `int main ( void ) { return 2 + 3 ; }\n`, exit 0. H04 tests `candidate_emit_golden` (`h04_candidate.rs:60-72`), `candidate_define_*` (`:243-335`), etc. all pass. | `-E` is host-side emission through PP28; it is not the frozen target preprocessor claim and does not solve `-I`/system headers. |
| Candidate target emission refusal | **IMPLEMENTED negative path** | `candidate.rs:104-107` returns a driver error for `-S`/`-c` before reading source. | Direct `-S` run → exit 2, `-S refused: target emission is fail-closed while the target is unverified (Part B, probe-gated)`; no output file. H04 test at `h04_candidate.rs:195-209`. | This is a driver-level early refusal, not the `TargetSpec::require_verified()` / `ConfigError::TargetUnverified` structured path. That path exists (`compiler/src/target.rs:647-656`, `:1344-1347`) but is not exercised by `candidate` because target emission is absent. |
| `-I` and multi-source | **ABSENT** | `candidate.rs:140-141` rejects `-I`; `:192-197` requires exactly one input. `docs/tasks/H04_I_FREEZE.md` (untracked draft) proposes search-path/multi-source policy but is **DRAFT ONLY / OPEN** (`H04_I_FREEZE.md:1-15`). | No success test exists; the H04 suite does not exercise `-I`. | No include resolution, no system/sysroot search, no quoting/multi-TU orchestration. |
| SE29 function-definition chip | **IMPLEMENTED but NOT wired into candidate** (`/39`) | `compiler/src/chips/semantic/se_function.rs:1-20`; projection `:51-265`; compute `:306-346`; appends/reuses one signature-carrying `SemRecord` at `:324-345`. Chip `SE_FUNC_CHIP = ChipId(57)` (`compiler/src/manifest.rs:1162-1168`); task kind `SEMANTIC_FUNCTION_DEF` (`compiler/src/task.rs:266-273`). | `cargo test --manifest-path compiler/Cargo.toml --test c39_sefunc` → **8 passed**. | `candidate.rs` imports/routes do not include `SeFuncChip` (`candidate.rs:23-29`, `:252-289`, `:802-831`). Therefore the candidate evidence run never commits a `FunctionDefinition` `SemRecord`, and T09 does not consume one. See §3.2 gap G3. |
| T08 const fold (M1) | **IMPLEMENTED** (frozen `/7`) | `compiler/src/chips/constant_layout_init/fold.rs:1-16`; worker `:108-149`; checked fold `:151+`; SE binary enqueues the child at `compiler/src/chips/semantic/se_binary.rs:229-241`. | Exercised indirectly by the candidate and by `c15_ir` (`compiler/tests/c15_ir.rs:479-505`, asserts the fold is `vec![5]` and the IR carries the same single `ConstId`). | Integer decimal/add M1 subset only. `ConstRecord` has no `ty` field (`compiler/src/bus.rs:596-602`). |
| T13 VF06/VF12/VF05/VF01 registration and stages | **FROZEN CONTRACT** (slices `/14`, `/17`, `/18`, `/19`) | Task kinds and stage rows in `compiler/src/task.rs:276`, `:515-531`; chip IDs in `compiler/src/manifest.rs:1023-1042`; `/14`–`/19` descriptions in `docs/tasks/T01_COMPILER_CONTRACT.md:299-386`. | Registration/stage assertions in the targeted tests (`c14_se.rs:401-451`, `c17_vf12.rs:456-484`, `c18_vf05.rs:264-298`, `c19_vf01.rs:449-480`), all passing. | The artifact hash covers shapes/rule IDs, not the semantic behavior inside the chips. |
| T11 AArch64 target code | **ABSENT** | No `compiler/src/chips/target/` directory. `grep -rn 'AArch64\|assembly\|\.global\|\.text' compiler/src` finds only `lex/string` target-width comments, `preprocess/pp_builtin`, `snapshot`/`bus` artifact kind names, and `target.rs`; no CG chip, no `CG0x` constants, no assembly production. `ArtifactKind::Assembly` exists (`compiler/src/bus.rs:88`; `compiler/src/contract.rs:481`) but has no producer. | No test exists to run. | T11 CG01–CG39 (`docs/tasks/T11_TARGET_CHIPS.md:11-51`) are planned work, not implemented capability. |
| Assemble / link / run | **ABSENT** | No runner, assembler/linker wrapper, or per-instance isolation in `tools/torture/src/**`; only the H00 lock verifier and H01 probe harness exist. | No test exists to run. | H05 and H02/H03 are required; see §4. |
| GCC torture acceptance | **ABSENT / UNRUN** | No corpus lock, no DejaGnu integration, no denominator, no runner, no report. `tools/torture` is the H00 verifier scaffold (`tools/torture/README.md:5-8`). | No torture command run. | No compile/execute/ieee instance or file rate exists. |

### 2.2 Catalog coverage at a glance (implemented vs planned)

- **T09 IR lowering:** 29 catalog rows (`docs/tasks/T09_IR_LOWER_CHIPS.md:7-37`). Implemented: one fused worker covering the M1 scope of **IR01/IR02/IR03/IR19** only (`ir_function.rs:1-13`). Absent: IR04–IR18 and IR20–IR29, including memory/address/conversion/assignment/call/CFG/loops/switch/bitfield/aggregate/VLA/atomic/function-end.
- **T13 verification:** 14 catalog rows (`docs/tasks/T13_VERIFICATION_CHIPS.md:68-83`). Implemented: **VF01, VF05, VF06, VF12, VF14** (5). Absent: **VF02, VF03, VF04, VF07, VF08, VF09, VF10, VF11, VF13** (9).
- **T11 target code:** 39 catalog rows (`docs/tasks/T11_TARGET_CHIPS.md:11-51`). Implemented: **0**. Only the target identity/config model exists (`target.rs`), not ABI, MI, allocation, frame, assembly, or target verification.
- **T13 M1 gap note:** VF07/VF08/VF09 are explicitly outside the M1 chip set and are recorded as a gap, not a pass (`M1_TARGET_ACCEPTANCE.md:174-176`). VF12 is a shape/identity model, not an arithmetic interpreter.

### 2.3 Target model and probe state

| Item | Classification | Evidence | Consequence |
|---|---|---|---|
| Frozen target identity | **FROZEN CONTRACT** | `compiler/src/target.rs:426-451` (`aarch64-unknown-linux-gnu`, ELF, LP64, little, AAPCS64); `compiler/contracts/CONTRACT_VERSION:5-6`. | Identity may be used symbolically, but not as proof of concrete values. |
| Proposed concrete scalar/ABI values | **UNVERIFIED CLAIM** | `TargetSpec::aarch64_unknown_linux_gnu_unverified` hardcodes expected sizes/alignments and ABI counts (`target.rs:583-620`) but sets `verification=UNVERIFIED` (`:607-619`). `compiler/contracts/target/aarch64-linux-probe.txt:1-6` is explicitly an unverified proposal. | `sizeof(int)=4`, `stack_align=16`, `gp/fp_arg_regs=8`, `long double=binary128`, `wchar_t=utf32`, etc. are **not** verified facts. |
| Private verified state / attestation | **IMPLEMENTED mechanism, never applied to real evidence** | `VerificationState` private representation `target.rs:361-417`; `require_verified` `:647-656`; `TargetSpec::attest` `:666-716`; environment `ensure_codegen_ready` `:1344-1347`. Attestation validates required fields, identity, `verified=true`, and an inline `report_hash` over the canonical unsigned body (`:858-899`). | The mechanism is test-covered with a synthetic signed report, but no real probe report has been attested. `attest` does not prove physical provenance or plausibility (C02 notes this; e.g. zero/oversized ABI counts pass if hashed). |
| H01 probe harness | **IMPLEMENTED harness, NEVER RUN** | `tools/torture/probe/run-probe.sh:1-255`; guard/pins `:67-125`; reference-only compilation `:199-216`; normalization `:247-250`; normalizer `tools/torture/probe/normalize/normalize.py:50-99`; CI workflow `.github/workflows/t00-target-probes.yml:1-50` is manual-only for the real probe. | A harness in the tree and 57 self-test checks are not a probe result. No report or report hash exists in the repository (only synthetic fixture reports under `tools/torture/probe/tests/fixtures/expected/`). |
| Probe report → attestation handoff | **UNRESOLVED GAP** | The normalizer deliberately leaves `abi.gp_arg_regs`, `abi.fp_arg_regs`, `abi.stack_align`, and `abi.variadic_register_save_area` as `unresolved` (`normalize.py:91-99`, `:397-400`), and emits an external `report.sha256`, not the `verified=true` + inline `report_hash` fields required by `ProbeReport::verify_verified` (`target.rs:874-899`). T00 assigns classification to H07 and attestation to the T01 integrator (`docs/tasks/T00_GCC_TORTURE_GATE.md:92-106`). | Even a successful H01 probe cannot be passed directly to `TargetSpec::attest`; H07 must classify the four ABI fields from assembly evidence and the integrator must produce/attest the required report form. HVA, `x18`, relocations/TLS, and target features have no probe fixture at all (`T11_TARGET_CHIPS.md:7`). |
| Probe self-check semantics | **NON-GATING** | `tools/torture/probe/README.md:158-177`: `abi.*.ok` values are copied through; a failed self-check prints `0` but the probe still exits 0 and the report still hashes. | A valid `report.sha256` does **not** mean the ABI round-trip checks passed. Any consumer must require every `abi.*.ok=1`. |

### 2.4 Commands actually run (and actual outcomes)

All commands were run from the repository root. No source file was created or edited by this audit.

```text
git status --porcelain=v1
git log -1 --format='%H %ad %s' --date=iso-strict
cat compiler/contracts/CONTRACT_VERSION
cargo check --manifest-path compiler/Cargo.toml --all-targets
cargo test --manifest-path compiler/Cargo.toml --test c14_se
cargo test --manifest-path compiler/Cargo.toml --test c15_ir
cargo test --manifest-path compiler/Cargo.toml --test c17_vf12
cargo test --manifest-path compiler/Cargo.toml --test c18_vf05
cargo test --manifest-path compiler/Cargo.toml --test c19_vf01
cargo test --manifest-path compiler/Cargo.toml --test c37_const_branch
cargo test --manifest-path compiler/Cargo.toml --test c38_vf14
cargo test --manifest-path compiler/Cargo.toml --test c39_sefunc
cargo test --manifest-path compiler/Cargo.toml --test h04_candidate
compiler/target/debug/candidate <(printf 'int main(void){return 2+3;}\n')
compiler/target/debug/candidate -S <(printf 'int main(void){return 2+3;}\n')
compiler/target/debug/candidate -E <(printf 'int main(void){return 2+3;}\n')
bash tools/torture/probe/run-probe.sh --self-test
cargo test --manifest-path tools/torture/Cargo.toml --test verify
cargo test --manifest-path tools/torture/Cargo.toml --test cli
git status --porcelain=v1 | sha256sum
```

Actual summarized results:

```text
cargo check ................................ Finished `dev` profile (warnings only)
c14_se .................................... 9 passed, 0 failed
c15_ir .................................... 7 passed, 0 failed
c17_vf12 .................................. 6 passed, 0 failed
c18_vf05 .................................. 6 passed, 0 failed
c19_vf01 .................................. 7 passed, 0 failed
c37_const_branch .......................... 10 passed, 0 failed
c38_vf14 .................................. 10 passed, 0 failed
c39_sefunc ................................ 8 passed, 0 failed
h04_candidate ............................. 18 passed, 0 failed
probe self-tests (normalizer/guard) ....... 57 passed, 0 failed
torture verifier tests .................... 51 passed, 0 failed
torture CLI tests ......................... 7 passed, 0 failed
```

Direct candidate result:

```text
$ compiler/target/debug/candidate <(printf 'int main(void){return 2+3;}\n')
candidate evidence (t01-c01-c06/39)
input: /dev/fd/11
snapshot: 833659dd3c53954267c175c07a693739311c7693e1af5044ded3a52664621c8f
trace: 8b466fadf535de4033807ef7ff7c192c82e5100c37d2eada0ea5fc06ff3a54a8
interpret: value=05 negative=false
exit=0

$ compiler/target/debug/candidate -S <(printf 'int main(void){return 2+3;}\n')
candidate: -S refused: target emission is fail-closed while the target is unverified (Part B, probe-gated)
... usage ...
exit=2

$ compiler/target/debug/candidate -E <(printf 'int main(void){return 2+3;}\n')
int main ( void ) { return 2 + 3 ; }
exit=0
```

Three repeated candidate runs with the same source and same process-substitution path produced identical snapshot and trace values. This is run-to-run determinism for this fixture, not a target result.

### 2.5 Documentation drift found during the audit

These are current-file inconsistencies; they do not change code behavior, but they make status claims unreliable:

| Stale statement | Current fact |
|---|---|
| `docs/tasks/T00_GCC_TORTURE_GATE.md:107-117` says H04 has **6 tests** and `-E`/`-D`/`-U` remain deferred. | `docs/tasks/H04_CANDIDATE_DRIVER.md:5,52-58` records 18 tests and present `-E`/`-D`/`-U`; code and this audit's 18-test run confirm it. |
| `docs/tasks/M1_TARGET_ACCEPTANCE.md:461` (P7) says the candidate has 6 tests and `-E`/`-I`/`-D`/`-U` are still absent. | `-E`, `-D`, `-U` are present; only `-I`/multi-source and target emission are absent. |
| `docs/tasks/PARALLEL_EXECUTION.md:9` says H04 is “absent”. | H04 Part A is present; H04 is still partial for the full flag matrix. |
| `docs/tasks/T13_VERIFICATION_CHIPS.md:12,21-29` says VF06 registration is still open. | `/14` froze `verification.typed_invariant` (stage 4, chip 15) and `/39` lists it; code and targeted tests match the frozen slice. The T13 catalog text needs reconciliation. |
| `docs/tasks/H04_CANDIDATE_DRIVER.md:41` shows evidence header `t01-c01-c06/19`. | The binary actually prints `t01-c01-c06/39` (verified above). |

---

## 3. M1 Closed-Loop Gap Analysis

### 3.1 What the current M1 Part A evidence actually proves

For the exact fixture bytes `int main(void){return 2+3;}\n`, the current candidate:

1. Reads source bytes from the host and runs the implemented PP01→PP02→PP03→PP04, PP19 conditional, PP09 macro-invoke, PP05 directive, lex intern/classify/decode, PA TU, TY int/function/scope/declare, SE literal/binary/return, VF05, VF06, IR function lowering, VF12, and VF01 slices (`candidate.rs:572-889`; routes `:252-289`).
2. Produces exactly one folded `ConstRecord` `[5]` through the real T08 fold child (c15 asserts this at `compiler/tests/c15_ir.rs:479-505`; the candidate's `interpret: value=05` reads that record at `candidate.rs:876-887`).
3. Lowers to one `Function`, one entry `Block`, one `Value`, `Constant`, and `Return`; the `Constant` names the one `ConstId`; `Return` is the greatest instruction ID (`ir_function.rs:321-384`; c15 assertions `:500-564`).
4. Passes VF06, VF05, VF12, and VF01 for this shape and emits deterministic snapshot/trace hashes.
5. Refuses `-S`/`-c` with exit 2 while the target is unverified.

It proves a deterministic **source→symbolic IR→modeled constant** chain for the fixture. It does **not** prove:

- target code generation, ABI correctness, assembly validity, ELF structure, linking, or exit status `5`;
- general C parsing, types, conversions, control flow, memory, calls, or optimization;
- complete M1 A3 typed-fact coverage (see G3 below);
- that VF12 independently computes `5` (it only returns the already-folded `ConstId`);
- that the candidate's single-fixture behavior generalizes beyond the closed M1 node/op/vocabulary sets.

### 3.2 Stage-by-stage gaps against `M1_TARGET_ACCEPTANCE.md`

| # | Stage / required M1 property | Current state | Gap / blocker |
|---|---|---|---|
| G1 | Preprocess: candidate's own preprocessing; no external `-E` (`M1_TARGET_ACCEPTANCE.md:122-124`) | Implemented minimal PP prefix over real source; `-E` is host-only PP28 emission. | No `-I`/system headers, no recursive include; the M1 fixture does not need them. |
| G2 | Lex/parse/type/semantics for `main`, `(void)`, `return` (`:125-128`) | Implemented M1 subset; candidate walks real tokens/nodes. | General declarators/expressions/statements absent; uncommitted parse drafts are not registered (`parse/mod.rs` has `mod` rows; `candidate.rs` does not install those workers). |
| G3 | A3: every checked node, **including `FunctionDefinition`**, has exactly one committed T07 `SemRecord`; T09 consumes it without re-inferring (`M1_TARGET_ACCEPTANCE.md:165`) | `candidate` never dispatches `SE_FUNC` (`candidate.rs:252-289`, `:802-831`). `IrFunctionChip` finds the symbol by scanning the declarator (`ir_function.rs:120-144`) and checks only `Return`/`BinaryAdd` sem facts (`:204-222`). VF06 checks only the three expression/statement kinds (`vf_invariant.rs:180-184`). | **Part A acceptance gap.** Wiring SE29 into the candidate is not sufficient by itself: VF06's blanket `ty == int` rule must distinguish the function-signature fact, and `IrFunctionChip` should consume the committed `FunctionDefinition` fact rather than re-inferring the symbol. |
| G4 | Constant/layout: folded `int 5`; T09 emits only IR `Constant`; `ConstRecord.ty == ValueRecord.ty` (`M1_TARGET_ACCEPTANCE.md:183,219-226`; `T09_IR_LOWER_CHIPS.md:81`) | Fold computes `5`; IR sets `ValueRecord.ty = int` (`ir_function.rs:362`). | Frozen `ConstRecord` has only `value` and `negative` (`compiler/src/bus.rs:596-602`); it has **no `ty` field**, so the documented immediate-type rule is not implementable in `/39`. This needs a `/6` schema decision or an explicit rule revision. |
| G5 | IR-1..IR-9, one function/block, terminator greatest ID, no CFG edges, no memory/calls/effects, non-SSA legal (`M1_TARGET_ACCEPTANCE.md:178-226`) | Minimal Constant+Return shape satisfies the positive M1 fixture. | VF07/VF08/VF09 are explicitly outside M1 and are a recorded gap (`M1_TARGET_ACCEPTANCE.md:174-176`). `IR28 FunctionEnd` and the T01 phase-2b `TerminatorMissing` hook do not exist (`T09_IR_LOWER_CHIPS.md:12,59-98`; OB-26 unresolved). |
| G6 | IR interpreter models return `5` without target widths (`M1_TARGET_ACCEPTANCE.md:228-236`) | VF12 checks shape and returns the folded `ConstId`. | It is a symbolic shape/identity model, not an arithmetic interpreter. A wrong fold value would still be returned as “the modeled value”; only the H04 test's textual `05` assertion and the fold tests catch that. |
| G7 | Target code: candidate emits AArch64 GNU/Linux assembly (`M1_TARGET_ACCEPTANCE.md:132,140-145`) | No T11 chip, no assembly text, no machine IR, no ABI plan. | Blocked by target verification and T11 implementation. |
| G8 | Assemble/link/run: object, ELF, exit `5` (`M1_TARGET_ACCEPTANCE.md:133-138,259-313`) | No assembler/linker wrapper, no runner, no H05 isolation. | H01 substrate, H05 runner, and H04 `-S`/`-c` are all absent; `-S`/`-c` are currently refused. |
| G9 | Target verification: probe report, H07 classification, T01 attestation (`M1_TARGET_ACCEPTANCE.md:314-339,475-482`) | Harness exists but never ran; no report/hash; four ABI fields unresolved; no attestation. | **The primary blocker.** `target_verification=unverified`; `TargetSpec::require_verified` fails closed. |
| G10 | Replay/determinism for Part A (`M1_TARGET_ACCEPTANCE.md:361-372`) | Candidate three-run snapshot/trace equality observed; H04 test checks repeatability. | No frozen expected Part A golden in the repository beyond test assertions; no VF13 replay chip; target D3/D6 not applicable yet. |
| G11 | Negative paths N1-N10 (`M1_TARGET_ACCEPTANCE.md:414-425`) | `-S` refusal and invalid-input diagnostic work; VF01/VF05/VF06/VF12 have negative tests. | N1 is a driver early-refusal, not `TargetSpec::require_verified`; N8 (ABI/target negative) is absent with T11; N2 target unsupported path absent. |
| G12 | H04 Part A commands in `M1_TARGET_ACCEPTANCE.md:494-523` | `--emit-ir-snapshot`/`--emit-trace`/`--interpret-ir` select evidence sections and always compile the source. | `candidate --interpret-ir main.ir` (illustrative in §9) is **not** implemented: the driver takes a `.c` source and runs the full pipeline. `--emit-ir-snapshot` prints a hash, not an IR dump. |

### 3.3 Minimal real closed loop still required

In dependency order:

1. **H01:** provision a native Linux AArch64 runner with pinned reference GCC, assembler, linker, sysroot, glibc, and runner identity; run `tools/torture/probe/run-probe.sh`; preserve `raw/`, `normalized/report.txt`, `normalized/report.sha256`, evidence blobs, and toolchain versions.
2. **H07:** classify the four unresolved ABI fields from the captured assembly evidence; require every `abi.*.ok=1`; record HVA/`x18`/relocation/TLS/feature limitations; produce the canonical `verified=true` + inline `report_hash` form expected by `TargetSpec::attest`.
3. **T01 integrator:** call `TargetSpec::attest` with the real report; only then mark C02 verified and update the target-verification status by the authorized contract process. Do not hardcode values.
4. **T11:** implement at minimum the ABI/constant/return/frame/assembly path for `int main(void){return 5;}` (CG01–CG05, CG09–CG11, CG15–CG18, CG29–CG38 as needed), emitting GNU/Linux AArch64 assembly text with `.text`, `.global main`, `.type main, %function`, and `.size main, .-main`; no Mach-O/Darwin markers.
5. **H04:** implement `-S` and `-c` only after C02 attestation; invoke the frozen assembler/linker; produce an ELF object and executable; never pass C source/AST/IR to GCC/Clang.
6. **H05:** isolate execution per instance with timeouts/output/resource limits and crash/signal classification; verify normal exit status `5` (not signal, low 8 bits).
7. **Verification:** B1–B6 ELF structure, B7–B11 AAPCS64/function checks, D1–D3 determinism, N1–N2 target negative paths; three repeated runs.
8. **Part A completion:** decide whether M1 A3 requires SE29 wiring; if yes, wire `SeFuncChip` into the candidate and adjust VF06 for the function-signature `SemRecord`; resolve the `ConstRecord.ty` rule; record the expected snapshot/trace (or a canonical semantic projection) as frozen M1 evidence.

**Claims blocked by the unverified target (do not state as fact until TGT-01 → TGT-02 → TGT-03):**

- all concrete scalar sizes/alignments, `char`/`wchar_t` signedness/encoding, `long double` format, and `__int128` layout;
- AAPCS64 GP/FP argument-register counts, stack alignment, variadic register-save-area behavior, aggregate/HFA/HVA classification, and the `x18` policy;
- AArch64 assembly syntax acceptance, instruction/immediate ranges, relocations, TLS, target features, and ELF object/executable structure;
- link defaults (PIE/non-PIE, startup objects, loader path, symbol visibility), runner exit-code/signal mapping, and the M1 “exit status 5” result;
- any T11 CG01–CG39 implementation acceptance, any `-S`/`-c` success claim, and the M1 Part B rows P4/P5/P17/P8;
- any GCC torture compile/execute/ieee or pooled rate, and any “target verified”/C02 claim.

M1 Part A symbolic results are **not** blocked by the unverified target, but they also cannot substitute for any item in this list.

---

## 4. H00–H10 Host-Task Status

Classification key: **Scaffold only** = useful tooling exists but the task acceptance is not met; **Present, never run** = harness exists, no real evidence; **Partial** = some acceptance items work; **Absent** = no implementation.

| ID | Status | Evidence actually inspected/run | Missing items before acceptance |
|---|---|---|---|
| **H00** | **Scaffold only; verifier tests pass** | `tools/torture/src/{lock,verify,path,digest,diagnostic}.rs`; README status `tools/torture/README.md:5-8`; status doc `docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md:3-13,109-132`. Ran `cargo test ... --test verify` → 51 pass; `--test cli` → 7 pass. | No authorized corpus fetch, no asset tree, no frozen lock, no license/legal sign-off; no download/production lock path. |
| **H01** | **Present, never run** | `tools/torture/probe/run-probe.sh`, `lib/guard.sh`, `normalize/normalize.py`, `src/scalar-layout.c`, `src/abi-args.c`, `tests/run-tests.sh`; manual CI workflow `.github/workflows/t00-target-probes.yml`. Ran `run-probe.sh --self-test` → 57 pass. | No Linux AArch64 substrate provisioned; no pinned GCC/assembler/linker/sysroot/glibc/runner; no real report or report hash; four ABI fields deliberately unresolved; no H07 classification; no attestation. |
| **H02** | **Absent** | No DejaGnu wrapper, board file, or candidate-under-test driver in `tools/torture/src`/`tests`; only lock verification. `T00_GCC_TORTURE_GATE.md:115-119` says H02–H03 are planned only. | Official driver integration, candidate wrapper, compile=assemble behavior, execute/ieee runs, `.sum/.log` capture. |
| **H03** | **Absent** | No instance-manifest normalizer, option expansion, auxiliary-source resolution, or denominator tool. | Frozen instance manifest, option/dialect/header/aux/target-predicate expansion, dual enumeration equality. |
| **H04** | **Partial (Part A present)** | `compiler/src/bin/candidate.rs`; `compiler/src/bin/candidate/{emit_helper,macro_flags}.rs`; `compiler/tests/h04_candidate.rs` (18 pass). Direct M1 evidence, `-E`, `-D`, `-U`, and `-S` refusal all verified. Doc `H04_CANDIDATE_DRIVER.md:5-8,20-33,52-65`. | `-I`, multi-source, quoting edges, torture flag matrix, `-S`/`-c` success (probe-gated), assembler/linker invocation, object/executable production. H04_I draft remains unaccepted. |
| **H05** | **Absent** | No runner/isolation/limits/timeout/cancellation implementation in `tools/torture`; `M1_TARGET_ACCEPTANCE.md:462` (P8) records absent. | Per-instance directories, timeouts, output/resource limits, cancellation, crash/signal classification, malicious-hang test. |
| **H06** | **Absent** | No aggregate report library/CLI, no JSON/HTML schema, no dual-denominator logic. `Vf14Chip` is a classifier chip, not the Host aggregator (`vf_evidence.rs:1-36`). | Per-instance evidence record, compile/link/run/check stratification, unknown-rule handling, fake-PASS self-test, retained commands/evidence. |
| **H07** | **Absent** | No reference-oracle runner/classifier implementation; only the ownership boundary is documented (`T00_GCC_TORTURE_GATE.md:92-106`, `M1_TARGET_ACCEPTANCE.md:475-482`). | Reference compiler baseline on the same target; oracle fixtures; classification of H01's four unresolved ABI fields; mixed-link/oracle evidence. |
| **H08** | **Absent** | No minimizer, bisect, or snapshot-localization tool. | Failure minimization preserving the original predicate, token/AST/IR/asm localization, retained originals. |
| **H09** | **Absent** | `.github/workflows/ci.yml` runs framework/compiler/torture unit checks; it does **not** run a frozen torture corpus, and no release gate or three-consecutive-run profile exists. | Full/smoke/nightly corpus regression, denominator freeze, 3-run identical profile, new-failure merge block, final >0.99 gate. |
| **H10** | **Absent** | No supplementary standard/ABI test suite, random valid-C generator, or undefined-behavior screening. | Self-owned execution tests, ABI/mixed-link tests, random valid C, UB screening. |

**Note on the dispatch ledger:** `docs/tasks/DISPATCH_LEDGER.md` appeared in the workspace during this audit (concurrent writer). It records a full-suite compiler result; this audit did **not** run the full compiler suite and does not adopt that result as its own evidence.

---

## 5. Recommended Target/Host Work Orders

These are recommendations, not authorization; all touch integration-owned or new files and require the named owner/review. No path below was created by this audit.

### 5.1 Target-evidence and minimal target chain (critical path)

| Order | Owner | Exact paths (existing or proposed) | Gate / acceptance |
|---|---|---|---|
| **TGT-01** | H01 | Existing: `tools/torture/probe/run-probe.sh`, `tools/torture/probe/normalize/normalize.py`, `tools/torture/probe/src/{scalar-layout,abi-args}.c`; outputs under `T00_PROBE_OUTDIR` (outside the repo). | Native Linux AArch64; pinned GCC version+binary SHA-256, assembler, linker, sysroot, glibc, runner; report + `report.sha256`; every required field present; every `abi.*.ok=1`; no macOS/Darwin value. |
| **TGT-02** | H07 | New H07-owned `tools/torture/probe/classify/` plus docs/evidence notes; consumes the H01 report. | Four ABI fields resolved from captured assembly evidence; no guessing; HVA/`x18`/relocation/TLS/features explicitly marked unprobed; emit the `verified=true` + inline `report_hash` form required by `compiler/src/target.rs:874-899`; independent review. |
| **TGT-03** | T01 integrator | `compiler/src/target.rs` (`TargetSpec::attest` call path), `compiler/tests/c02_target.rs`, `compiler/contracts/CONTRACT_VERSION`, `compiler/contracts/target/aarch64-linux-probe.txt`. | Real report accepted by `attest`; `verification.is_verified()` true; synthetic `signed_report` cannot count as acceptance; contract-version/hash update only through the authorized freeze process. |
| **TGT-04** | T11 + integrator | New `compiler/src/chips/target/aarch64/` (CG01–CG05 first: `cg01_scalar_abi.rs`, `cg02_aggregate_abi.rs`, `cg03_argument_location.rs`, `cg04_return_location.rs`, `cg05_variadic_abi.rs`), with integration-owned `mod.rs`, task kinds, routing, allowlist. | ABI classifications match probe-confirmed values; scalar/int return path for `int (void)`; negative fixtures for wrong class/register count; no Darwin model. |
| **TGT-05** | T11 + integrator | New `compiler/src/chips/target/aarch64/` (minimum CG09–CG11, CG15–CG18, CG29–CG38 as applicable), especially `cg10_constant_select.rs`, `cg09_return_sequence.rs`, `cg32_frame_layout.rs`, `cg33_prologue_epilogue.rs`, `cg35_assembly_instruction.rs`, `cg37_assembly_function.rs`, `cg38_target_verify.rs`. | Candidate emits GNU/Linux AArch64 assembly for `int main(void){return 5;}`; assembler accepts; object/executable ELF with `main`; no unassigned vregs; stack alignment/callee-save/return checks; B1–B11. |
| **TGT-06** | H04 + integrator | `compiler/src/bin/candidate.rs` (target emission branch), `compiler/src/bin/candidate/` helpers if needed, integration-owned toolchain invocation. | `-S` and `-c` open only after TGT-03; `-S` writes assembly, `-c`/assembler produces object; `TargetSpec::require_verified` is the gate; no GCC/Clang C compilation; no fallback; `-S`/`-c` remain fail-closed before attestation. |
| **TGT-07** | H05 | New `tools/torture/runner/` (or an explicitly approved host location); per-instance dirs, timeout, output/resource limits, cancellation, crash classification, exit-code propagation. | Executes the TGT-05 executable in isolation; normal exit status `5`; signals/timeouts classified; malicious hang test; identical basenames do not collide. |

### 5.2 Host tasks H00–H10

| Order | Owner | Exact paths (existing or proposed) | Gate |
|---|---|---|---|
| **HST-00** | H00 | `tools/torture/src/**`, `tools/torture/tests/**`, lock path under `tools/torture/lock/` (proposed); external asset tree outside the MIT core. | Authorized fetch; all asset hashes/license/provenance; frozen lock verifies offline; H00 exit code 0 only after freeze; no vendoring. |
| **HST-01** | H02/H03 | New `tools/torture/dejagnu/` (wrapper/board) and `tools/torture/manifest/` (instance expansion). | Official compile/execute/ieee drivers; compile produces an object; execute/ieee run; two independent enumerations identical; frozen denominator before execution. |
| **HST-02** | H05 | `tools/torture/runner/` (see TGT-07). | Isolation/limits/cancellation/crash classification; H05 self-test; no test can block the suite. |
| **HST-03** | H06 | New `tools/torture/report/` (JSON/HTML aggregation). | Dual denominators, suite/config strata, failure reasons, exclusions, unknown rules; self-test rejects fake PASS/skip/dropped evidence; per-instance command/evidence retained. |
| **HST-04** | H07 | New `tools/torture/oracle/` plus `tools/torture/probe/classify/`. | Reference compiler baseline on the same target; oracle fixtures; reference failure investigated rather than converted to candidate PASS; mixed-link checks. |
| **HST-05** | H08 | New `tools/torture/minimize/`. | Bisect/minimize preserving original predicate and original tests; token/AST/IR/asm localization; originals retained. |
| **HST-06** | H09 | New `.github/workflows/torture-regression.yml`; profile under `tools/torture/profiles/`. | Frozen profile; 3 consecutive identical runs; compile/execute/ieee instance and file rates each >0.99 plus pooled rates; per-configuration reported; new failures block merge. |
| **HST-07** | H10 | New `tools/torture/supplemental/`. | Self-owned execution/ABI tests, random valid C, UB screening; clearly separate from the torture rate. |

**Sequencing rule:** H00/H01 may run in parallel with Part A work; T11 target code, H04 `-S`/`-c`, H05 execution, and every target-dependent acceptance row stay **BLOCKED_PROBE** until TGT-01 → TGT-02 → TGT-03 complete. Do not unlock T11 from a synthetic or macOS value.

---

## 6. Unrun Checks and Open Questions

### 6.1 Checks not run by this audit

- Real H01 probe (`run-probe.sh` with real pins) — impossible on this macOS host; no Linux AArch64 substrate.
- Any `TargetSpec::attest` call with a real report; any C02 verified claim.
- Any T11 assembly emission, external assembler, linker, ELF inspection, or execution/exit-code test.
- Any `-S`/`-c` success path (the current paths are refusal-only).
- H02/H03 DejaGnu, H05 runner/isolation, H06 aggregation, H07 oracle, H08 minimization, H09 regression gate, H10 supplemental tests.
- GCC corpus acquisition/lock, torture compile/execute/ieee, or any pass rate.
- Full compiler package test suite; full root/compiler/torture workspace tests; `cargo fmt --all -- --check`; Clippy with `-D warnings`.
- Multi-source or `-I` compilation; quoting edge cases.
- Three-run target determinism (D3/D6) and target negative fixtures.
- Any “M1 acceptance passed” or “probe verified” claim.

The targeted test build emitted **137 compiler warnings**, largely dead code from the uncommitted parse/lex drafts now reachable through `parse/mod.rs`. This is not a Clippy result and not a green-baseline claim. The `cargo check` run itself completed with warnings only.

### 6.2 Open questions

1. **A3 / SE29 integration:** Is `FunctionDefinition` `SemRecord` mandatory for the accepted M1 Part A evidence? If yes, `candidate` must dispatch `SeFuncChip`, and VF06 must check the function-signature fact separately from the `int` expression rule. Current code does neither.
2. **`ConstRecord.ty`:** The documented `ir.op-immediate-type` rule requires `ConstRecord.ty == ValueRecord.ty`, but the frozen `ConstRecord` has no `ty` field. Is this a `/6` schema change, or is the rule to be restated for the `/39` slice?
3. **IR28 / `TerminatorMissing`:** The selected direction reuses a committed `FunctionEnd` terminal result plus a T01-owned phase-2b hook, but OB-26 (task-scoped vs whole-batch rejection and rollback ordering) remains unresolved and no IR28 chip exists. What is the minimum M1 negative fixture, and who owns the hook?
4. **VF12 independence:** Is returning the already-folded `ConstId` sufficient for the M1 “modeled return 5” gate, or must the verifier independently evaluate the covered arithmetic? Current evidence cannot distinguish a wrong fold from a correct one without the fold tests/H04 assertion.
5. **Probe report schema:** Who transforms the H01 normalized report (`probe.status=observed`, external `report.sha256`, four `unresolved` ABI fields) into the `verified=true` + inline `report_hash` form required by `TargetSpec::attest`? Is H07 allowed to synthesize the hash-bearing attestation input, and what review signs it?
6. **ABI evidence coverage:** HVA, `x18`, relocations/TLS, and target-feature availability have no probe fixture; after a passing H01 report, these remain unverified. Which M1 target features are actually required, and which must stay explicitly unsupported?
7. **`-S`/`-c` error contract:** Should `candidate` keep the driver-level early refusal, or route target requests through `TargetSpec::require_verified()` to produce the frozen `ConfigError::TargetUnverified`/`Target` diagnostic? The current behavior is fail-closed but does not exercise the structured target path.
8. **`--interpret-ir` naming:** The current flag selects an evidence section over a source compile; it does not accept an IR artifact. Should the flag be renamed, or should an IR-file mode be added with a frozen IR serialization?
9. **H04/H01 documentation status:** Multiple task documents still describe the pre-`-E`/`-D`/`-U` candidate and 6 tests. Which document is the authoritative H04 status after `/39`?
10. **VF14/H06 ownership:** VF14 is implemented but has no Host evidence producer. Does H06 own the evidence-pin construction, and should VF14 be wired into the candidate/H06 path before any torture run?

---

## 7. Bottom Line

| Claim | Verdict |
|---|---|
| Deterministic M1 Part A evidence for `int main(void){return 2+3;}` exists | **Yes, for the implemented subset; targeted tests run here pass.** |
| Complete M1 Part A acceptance (including A3 `FunctionDefinition` fact) | **No; SE29 is not wired and VF06/T09 do not consume the function-definition fact.** |
| AArch64 target compiler / assembly emission | **No; T11 is absent.** |
| Object, link, and executable exit status 5 | **No; no assembler/linker/runner integration.** |
| Target probe run and target values verified | **No; harness never ran, no report/hash, C02 unverified.** |
| M1 real closed loop passed | **No; Part B is blocked by the unverified target and absent T11/H05.** |
| GCC torture pass rate | **None measured; no corpus lock, driver, denominator, or runner.** |
| H00/H01/H04 complete | **No: H00 scaffold only; H01 never run; H04 Part A partial.** |
| H02/H03/H05–H10 implemented | **No; absent.** |
