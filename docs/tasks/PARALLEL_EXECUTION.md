# LLM Parallel Development and Handoff Protocol

## 1. Ordering

**Wave 0 (must be done first)**: T00/H00–H03 and target probes, can run in parallel with T01/C01–C06. The target probes in this wave gate **Part B / target-dependent work only**: they are not a precondition for drafting, reviewing, or approving the **symbolic Part A** contract, and the T01 `/6` freeze does not wait for the Linux probe to run. Implementation tasks are not dispatched until the T01 `/6` language types/task/result/schema are compiled and frozen (the storage extension itself is already approved, ADR-0001); **no chip implementation begins before that T01 schema freeze**. During this period other implementers may study language rules / write test designs, but must not create incompatible interfaces on their own.

**Wave placement is a plan, not evidence.** Naming a task in a wave (including Wave 0's "target probes") does not mean its handler exists, its chip is installed, or the Linux substrate/toolchain is provisioned. Part B execution waits for the substrate and a probe reporting `verified=true`; until then, target-dependent rows stay blocked rather than passing on symbolic or macOS values. `t01-c01-c06/9` is the current frozen artifact; wave placement alone authorizes no implementation.

**Host-task state (2026-10-05).** Wave 0's Host tasks are not a completed baseline. H00 is scaffold only (`tools/torture/**`; no corpus, no frozen lock). H01's probe harness (`tools/torture/probe/**`) exists but has **never run**: no report or hash exists, H01 is not complete, and C02 stays unverified. The probe→attestation handoff is T01-integrator-owned (`attest` validates caller-supplied data/hash, not provenance), while H07 owns the reference-oracle side, including the ABI classification the report leaves `unresolved`. H04's candidate driver is **absent**: it is required not only for torture flag handling but also for the M1 Part A evidence commands (snapshot/trace/interpret), so Part A evidence collection waits on it even though Part A contract drafting does not. H05's isolated execution (per-instance directories, timeouts/output/resource limits, cancellation, crash classification) is **absent** as well; it is required for any torture run and the final gate (H05–H10), not for M1 Part A.

**Wave 1**: T02 control; T03 preprocessing; T04 lexical; T06 symbol/type; T08 layout/constant; T11 ABI foundation (target-dependent **Part B**, blocked until the probe); T13 contract verification. Jointly read the frozen contract and each test with synthetic records.

**Wave 2**: T05 parsing; T07 semantic; T08 initialization; T09 IR; T13 typed-AST verification (**VF06**; M1: after committed T07 `SemRecord`s and before T09 lowering, registration still open); T11 basic instructions/stack/output (target-dependent **Part B**, probe-gated); T12 extension analysis. Local parts may be implemented in parallel; integration requires real upstream artifacts and does not substitute mock success.

**Wave 3**: integrate M1/M2 closed loops; T10 local optimization; T11 complete ABI/allocation/floating point (target-dependent **Part B**, probe-gated); T12 builtins/asm; T13 CFG/MI/ABI verification.

**Wave 4**: T00 full-scale failure closure; dispatch new chip tasks by failure cluster; complex GNU/IEEE/long double; optimization analysis; iterate verification until M5.

Each wave is not "all tasks run simultaneously": if a request/result field that a chip depends on is not frozen, or the required helper does not exist, the implementer can only submit contract suggestions/test designs and must not fill in another owner's files on their own.

## 2. File Ownership

- The integrator exclusively owns the root Cargo/workspace, application bus/type schema, task enums, motherboard routing, global registration, shared tests entry point, and final metric configuration.
- A single-chip owner only changes their own `compiler/src/chips/<group>/<snake_name>.rs`, independent test fixture, and corresponding manifest; the package lead manages the group's common entry point.
- When multiple chips run in parallel, do not jointly modify `mod.rs`; first submit the file list, and the integrator generates the module/registration and manifest aggregation.
- Prefer an independent worktree/branch and verify the contract hash before merging; in a single shared workspace, resetting/overwriting other implementers' changes is prohibited.
- Only shared mechanical helpers may be reused: checked ID access, bitvector operations, intern, serialization. Language rules should be independent chips and must not be stuffed into a giant helper that turns the chips into decoration.

## 3. Ready-to-Copy Work Assignment Template

```text
You are responsible for <task package>/<ChipId> <ChipName> of the Silicon C compiler.
First read docs/tasks/README.md, T01_COMPILER_CONTRACT.md, PARALLEL_EXECUTION.md,
then read the task document containing the chip. Obey the frozen contract <version/hash>.

Only permitted to modify:
  compiler/src/chips/<group>/<snake_name>.rs
  compiler/tests/chips/<group>/<snake_name>/**
  compiler/contracts/chips/<ChipId>.*

Implement: that row's Input→Output, all functionality, all specified tests, and the unified DoD.
The chip must be ZST and only read/write manifest fields; do not change semantic state when input does not match;
no I/O, no cross-chip calls, no implicit state; cross-tick tasks may only remain in the bus.
Must not modify Bus, Task enums, Cargo, or mod.rs; if an interface is missing, first submit a change request.
Test special-casing, returning success for unimplemented behavior, and invoking GCC/Clang to compile the candidate program are prohibited.

Deliver: code, exact read/write manifest, independent tests, test commands and real results,
limitations/unimplemented list, dependencies/integration steps. Do not yet claim full-compiler or torture pass.
```

Granularity: one implementer is responsible for one to five strongly related chips, avoiding a situation where a single LLM directly builds the entire parser/backend. For difficult rules (macro hide-set, declarator, ABI, floating point), start with one owner per chip + an independent reviewer.

### Initial Work Assignment Recommendations

Before the T01 `/6` language schema is frozen, only dispatch the design, probe, and interface work for `H00–H03` and `C01–C06` (the storage extension is already approved and the target identity frozen; the live gate is the `/6` language-schema freeze); do not let other implementers assume that the protocol already exists. The symbolic Part A contract work above proceeds without waiting for the Linux probe; only target-dependent Part B rows wait on the substrate and a probe reporting `verified=true`. After the foundation is frozen, micro-batches may be launched as follows, with each row further split into one to five chips per implementer. (These rows are recommendations and assign no implementation; no row is dispatchable before the T01 schema freeze.)

| Parallelizable task pool | Initial tasks | Independent fixture input | Integration waiting for |
|---|---|---|---|
| control | CT03–07, CT10 | manual task graph/proposals | C03/C06 |
| source/pp | PP01–04, PP06–08 | source bytes/pp tokens | source response protocol |
| literal | LX05–10, LX11–14 | pp spelling/target model | PP complete output |
| type | TY13–17, TY21–29 | manual type records | declaration/sem consumers |
| scope | TY01–09 | manual scope/name records | PA04/14/28 |
| layout | CL08–14 | manual types/members/pack | aggregate declarations |
| backend ABI | CG01–05 | manual signature/layout | TY28 and IR18 |
| verifier | VF01–04, VF06–08 | positive/negative task/store/CFG snapshots; VF06 manual checked-node/typed-fact snapshots | real records from each stage (VF06: committed T07 `SemRecord`s) |

After all these chips are "complete" under fixtures, they still have to pass real upstream integration; accept M1 first, then spread the remaining parsing, semantic, IR, and complex backend tasks. Do not treat full parallelism as a means to bypass contract/integration ordering.

## 4. Unified Handoff Record

```text
Chip IDs / files:
Contract version/hash:
Task input/result schema:
Reads / Writes (field paths):
Child request kinds:
Progress / waiting / completion / error rules:
Tests actually run + results:
Unsupported cases:
Torture feature/instance IDs linked (if any):
Integration prerequisites:
```

The reviewer checks semantic rules and test independence, not just whether it compiles. The test expected value cannot be computed directly by the implementation function (self-verification); it needs a specification, hand calculation, or reference evidence from the same target.

## 5. Integration Gates

1. Before merging schema/helper changes, batch-recompile dependent packages and do not let interfaces diverge.
2. Check the chip registry for unique IDs, manifest coverage, routing coverage, ZST, prohibition of cross-chip calls and Host access; static lint is not a formal proof.
3. Single-chip tests → pairwise signal propagation → stage snapshot → full chain → torture regression.
4. New enqueues execute next tick; test the trace before and after commit, with completion/fault exactly once; patches from the wrong owner are rejected.
5. When the optimizer changes IR it must invalidate analyses; the typed IR consumed by codegen must pass verification.
6. New failures introduced by parallel development have an owner and a minimal reproduction; do not use "other packages are not implemented yet" to hide one's own regression.

## 6. Things That Should Not Be Parallelized Prematurely

- Each inventing their own type layout, AST, or IR format; each implementing name resolution; multiple LLMs modifying the motherboard or bus.
- Running the entire hundreds-of-chips pipeline every tick; runtime threads concurrently writing the bus; unverified out-of-order fusion.
- Building a thousand stubs at the start to make the build all-green while having no first program closed loop.
- For the 99% goal, deleting cases / turning off options / counting unsupported as PASS; using reference GCC output directly as candidate output.

LLM parallelism is only responsible for source development; the runtime's first version is strictly sequential with fixed routing. Verify the architecture with M1 first, then expand the task pool.

## 7. Revision Record

Separate counter from the CDR/M1-proposal revisions; a bare "rev" number does not denote the same change in every document.

| Date | Revision and summary | Author |
|---|---|---|
| 2026-10-05 | Clarity-only edits (no assignment, ownership, or authorization change). §1: stated that Wave 0's target probes gate **Part B / target-dependent work only** and are not a precondition for drafting, reviewing, or approving the symbolic Part A contract or for the T01 `/6` freeze; restated that **no chip implementation begins before the T01 schema freeze**; added that **wave placement is a plan, not evidence** of installed handlers/toolchain or a provisioned substrate, that Part B execution waits for the substrate and a probe reporting `verified=true`, and that `/5` (`t01-c01-c06/5`) is current while `/6` is unfrozen. §3 heading text: noted the Part A/Linux-probe split and that the rows assign no implementation and are not dispatchable before the T01 schema freeze. Doc-only; no freeze, no code. | DeepSeek doc integrator |
| 2026-10-05 | **H00–H10 ownership/gate audit fixes** (docs-only; no assignment, authorization, or freeze change). §1: added the **Host-task state** paragraph — H00 is scaffold only, H01's probe harness has never run (no report/hash; C02 unverified), the probe→attestation handoff is T01-integrator-owned with H07 owning reference-oracle/ABI classification, and the absent H04 candidate driver also gates the M1 Part A evidence commands. | H00–H10 audit (doc-only) |
| 2026-10-05 | **T11 Part-B probe-gating annotation** (docs-only; no assignment, authorization, or freeze change). §1 Wave 2/3: annotated the remaining `T11` entries as target-dependent **Part B**, probe-gated, completing the Wave-1 annotation (OB-37). Companion edits: T11 gained a probe-gated qualifier over the concrete CG01–CG39 ABI values (UNVERIFIED until a probe reports `verified=true`/C02, with the probe's assembly-evidence-only fields and uncovered HVA/`x18`/relocation/TLS/features called out), and the T11 catalog row no longer reads as ELF output ownership (assembly emission; ELF object via the Host assembler). | T11 Part-B audit (doc-only) |
| 2026-10-05 | **VF06 placement, H05 host-state completion, and `/6` gate wording** (docs-only; no assignment, authorization, or freeze change). §1: added **VF06** to **Wave 2** at its fixed M1 position (after committed T07 `SemRecord`s, before T09 lowering; registration still open) and added **VF06** to the initial **verifier** pool row with its manual checked-node/typed-fact fixture input and committed-T07 integration dependency; recorded that **H05** (isolated execution/limits) is absent and is required for any torture run/final gate (H05–H10), not for M1 Part A; clarified that implementation waits on the T01 `/6` language types/task/result/schema freeze (the storage extension is already approved, ADR-0001), removing the stale "T01 extension is approved" phrasing. | VF06/H05/`/6` docs audit |
