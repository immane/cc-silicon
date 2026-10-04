# Silicon C Compiler: LLM Parallel-Task Master Plan

Status: task design; the compiler is not yet implemented and the GCC pass rate has not yet been measured. This document does not promise that parallel code generation alone can reach 99%+.

## 1. Goals and Boundaries

Using `cc-silicon` as the framework, the planned compiler application is written in Rust. Language rules and transformations belong in zero-field `RestrictedChip`s that compute from narrow immutable projections and return typed proposals; application adapters project and commit against the explicit `CompilerBus`. Hundreds or thousands of chips are permitted, but the chip count is not a success metric.

Final acceptance: after freezing the revision, target platform, test inventory, and option matrix, the compile, execute, and execute/ieee rates of the GCC C torture suite must **each exceed 99%**, and a per-file full-matrix rate above 99% must also be reached. Gaming the score by skipping unimplemented features, modifying source code, hiding timeouts, or delegating to another C compiler is prohibited. See [T00](T00_GCC_TORTURE_GATE.md) for the exact definition.

The recommended primary acceptance target is **aarch64-unknown-linux-gnu / ELF / LP64 / little-endian / AAPCS64**. The current macOS is a development host, not the runtime environment for that target; a Linux VM/CI or a trusted cross toolchain + runner must be established first. The ABI, object format, and long double of macOS arm64 must not be mixed in. The target is only a recommendation; after T00 freezes it, it must not be changed without review.

Language evolution: minimal C subset → common C11 semantics → GNU C compatibility and torture gaps. To satisfy old tests, compatibility modes such as old-style functions and implicit declarations must also be explicitly supported; the standard mode must not be silently relaxed because of this. Modern C features and target-related extensions are driven by a frozen corpus list; do not claim "complete C" or "support for any GCC version" in advance.

## 2. Specification Sources and Conflict Handling

Required reading:

- [Paradigm specification](../architecture/SILICON_PARADIGM_SPEC.md): state/logic isolation, no cross-chip calls, clock lifecycle.
- [SFL contract](../architecture/SFL_CONTRACT.md): observable state, backend/Host boundary, determinism, and differential verification.
- [SFL schema draft](../architecture/SFL_SCHEMA_DRAFT.md): read/write sets, phase, capability declaration.
- [Design blueprint](../design/ARCHITECTURAL_BLUEPRINT.md): design the bus, single responsibility, topology, and test bench first.
- [Getting started](../design/GETTING_STARTED.md): the current framework API; do not assume that the new interfaces in the tasks already exist.

**Priority**: approved semantic contract > frozen task interface > single-chip implementation. The CPU dynamic arena is an **approved application-scoped extension** to "fixed arrays / no heap" (see [ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) and T01); the framework itself remains fixed-array/heap-free. The extension is frozen in `compiler/` (C01–C06); it does not authorize C language chips or prove any compiler capability. The original design's descriptions that the compiler can guarantee no hidden state, automatic correctness, and automatic parallelism cannot be used as proof: the current trait does not enforce ZST, read/write permissions, or determinism.

Distinguish two kinds of backend: `cc-silicon::Backend` is the compute backend that executes chips (CPU in the first version); the compiler target/codegen backend is the **target of the generated program** (AArch64 in the first version). The two must not be conflated.

## 3. Documents and Task Packages

| Task | Deliverable / content | Prerequisite |
|---|---|---|
| [T00](T00_GCC_TORTURE_GATE.md) | GCC corpus freeze, runner, denominator, failure attribution, 99% gate | None; environment configuration frozen |
| [T01](T01_COMPILER_CONTRACT.md) | Bus, IDs, task protocol, storage extension, target model | None; extension approved |
| [T02](T02_CONTROL_CHIPS.md) | Scheduling, task/result commit, diagnostics, verification chips | T01 |
| [T03](T03_PREPROCESS_CHIPS.md) | Source stage, preprocessing tokens, macros, include, conditional compilation | T01 |
| [T04](T04_LEX_CHIPS.md) | Final tokens, integer/float/string decoding | T01; interface depends on T03 |
| [T05](T05_PARSE_CHIPS.md) | Expressions, declarators, statements, GNU syntax | T01; interface depends on T04/T06 |
| [T06](T06_SYMBOL_TYPE_CHIPS.md) | Namespaces, scope, type construction/conversion/compatibility | T01 |
| [T07](T07_SEMANTIC_CHIPS.md) | Expression/statement legality, side effects, atomic/volatile | T01; interface depends on T05/T06 |
| [T08](T08_CONSTANT_LAYOUT_INIT_CHIPS.md) | Constant expressions, layout, initialization, VLA | T01; interface depends on T06 |
| [T09](T09_IR_LOWER_CHIPS.md) | AST→explicit CFG/IR, storage, calls, dynamic stack | T01; interface depends on T07/T08 |
| [T10](T10_OPTIMIZE_CHIPS.md) | Optimization legality, local optimization, data flow, optional SSA | T01; interface depends on T09 |
| [T11](T11_TARGET_CHIPS.md) | AArch64 ABI, instructions, allocation, ELF assembly output | T01; interface depends on T09 |
| [T12](T12_GNU_BUILTIN_CHIPS.md) | GNU extensions, builtins, inline assembly, helper calls | T00 list + T01 |
| [T13](T13_VERIFICATION_CHIPS.md) | Contract/IR/ABI verification, differential and replay chips | T01; interface depends on each phase |
| [Parallel notes](PARALLEL_EXECUTION.md) | Dependency waves, file ownership, handoff template, integration discipline | Required reading for all implementers |
| [Per-chip template](TASK_TEMPLATE.md) | Executable task header, exact field binding, three concrete examples of type promotion/declarator/variadic | T01 interface frozen |
| [M1 frontend acceptance](M1_VERTICAL_SLICE_ACCEPTANCE.md) | Planned M1 frontend fixtures for `int main(void){return 2+3;}` (design only; no implementation, no execution, no pass rate) | T01 interface binding; target probe for target-dependent fixtures |
| [M1 target acceptance](M1_TARGET_ACCEPTANCE.md) | Planned M1 vertical closed-loop end-to-end acceptance, including the Linux-probe-gated target half (design only; all checks NOT RUN) | T01, T09, T11; planned (not provisioned) Linux probe substrate for Part B |

The tables for T02–T13 are work lists with one row per chip: each row's protocol inputs, outputs, functionality, and tests are that chip's concrete objectives; together with the unified contract in T01 they form the complete task. Cross-phase dependencies are **data/task-protocol dependencies**, not permission to call other chips directly. The tables are an initial decomposition and may be further divided when there is evidence; there is no requirement to duplicate identical chips just to pad the count.

The current list contains **331 unique chip tasks** in total, plus 11 Host tasks and 6 contract-foundation tasks. They are planned tasks, not implemented capabilities. Each chip must fill in the exact field binding from TASK_TEMPLATE; these fields are not yet frozen, so it must not be claimed that all of them can now be coded simultaneously without dependencies.

| Chip group | Count | ID range |
|---|---:|---|
| Control | 14 | CT01–CT14 |
| Preprocessing | 28 | PP01–PP28 |
| Lexical | 18 | LX01–LX18 |
| Parsing | 38 | PA01–PA38 |
| Symbol/type | 34 | TY01–TY34 |
| Semantic | 30 | SE01–SE30 |
| Constant/layout/initialization | 26 | CL01–CL26 |
| IR lowering | 29 | IR01–IR29 |
| Optimization | 22 | OP01–OP22 |
| Target code | 39 | CG01–CG39 |
| GNU/builtins | 39 | EX01–EX39 |
| Verification | 14 | VF01–VF14 |

## 4. Milestones and Acceptance

| Milestone | Must implement | Gate |
|---|---|---|
| M0 Foundation freeze | T00 reference GCC baseline; T01 schema; empty task→complete; progress/budget | Corpus/environment hash; protocol compiles; replay consistent; unimplemented must not be reported as success |
| M1 First vertical closed loop | `int main(void){return 2+3;}`; lexical/parse/type/IR/ABI/assembly | Candidate generates code itself, external assembler/linker, run returns 5 |
| M2 Integers and control flow | Type conversions, local/global, functions, pointers, arrays, loops, short-circuit | Layered fixtures all green; full torture suite starts reporting, denominator not changed according to failures |
| M3 Complex C | aggregate, initialization, bitfield, VLA, floating point, variadic, old-style C | All catalog chips have an implementation or an explicit gap; ABI mixed-linking verification; coverage statistics |
| M4 GNU/torture closure | T12 gaps, target features, optimization options and instruction semantics | compile/execute/ieee at 90%→95%→98% respectively; every failure has an owner |
| M5 Final gate | Full matrix, per-file, supplementary standard tests, regression and performance budget | All items defined by T00 >99%; no unattributed results; stable repeated execution |

The M1 milestone is elaborated in two planned acceptance documents: the target-independent frontend fixtures in [M1_VERTICAL_SLICE_ACCEPTANCE.md](M1_VERTICAL_SLICE_ACCEPTANCE.md) and the end-to-end closed-loop plan, including the Linux-probe-gated target half, in [M1_TARGET_ACCEPTANCE.md](M1_TARGET_ACCEPTANCE.md). Both are draft/planned design artifacts: they report no implemented chip, no executed test, no probe, and no pass rate.

Do not treat the M1/M2 small-subset pass rate as the final GCC pass rate. All catalog chips must enter the coverage ledger, and unimplemented items must be recorded; optional optimizations have lower priority than code-generation correctness. Whether complex ISA/language extensions become final mandatory items is decided by the frozen corpus; do not work backward and cut tests according to implementation capability.

## 5. Unified Definition of Done for Each Chip

1. Zero-field `RestrictedChip`; `compute(&Input) -> Output` has no bus access. A small `ChipAdapter` owns the manifest-scoped projection and proposal commit, and the application installs the projected chip through the motherboard. No cross-chip calls, I/O/environment/clock access, or hidden cache.
2. Has a manifest: unique ID, task kind, exact reads/writes, phase, category, backend class, target/mode conditions, dependencies, test paths. Do not merely write "reads/writes the whole bus".
3. On receiving a task not belonging to this chip: do not modify semantic state; for the same snapshot and input: the result is the same. After accepting a task, advance, wait for an explicit child task/Host request, or produce a structured error; it must not spin idly.
4. Downstream requests/output proposals are written to wires in the current tick; cross-tick continuations, queues, diagnostics, results, and cursors must be in registers. A response is consumed once, and each task completes once.
5. At minimum: normal, boundary, invalid, error-recovery/unsupported, replay, and unauthorized-field-unchanged tests; when there is an interaction requirement, add cross-chip tests. The cases listed in the tables must be tested and do not replace these general items.
6. Test independently with synthetic fixtures; input records may be mocked, but the semantics under test must not be mocked. Full integration must replace all stubs; faking a successful return is prohibited.
7. Compute width/alignment/floating point according to the frozen target model; do not use Rust host types or host `sizeof` to impersonate C target semantics.
8. Submit tests, contracts, and known limitations; `fmt/clippy/test` pass. Implemented state does not mean the chip has coverage in torture.

## 6. Code Locations (original layout sketch; current state in the T01 status below)

The original plan kept the current framework independent and created an application crate under `compiler/` that depends on the root crate; the proposed layout was:

```text
compiler/src/bus/            # owned by the integrator
compiler/src/chips/<group>/  # one snake_case.rs per chip
compiler/src/motherboard/    # owned by the integrator; only dispatches, hides no language rules
compiler/src/host/           # I/O, toolchain, runner boundary
compiler/tests/chips/        # single-chip snapshots
compiler/tests/integration/  # phase chains
compiler/contracts/         # frozen protocols and per-chip manifests
tools/torture/              # peripheral test infrastructure, not the semantic core
```

Whether to establish a Cargo workspace is decided by the T01 integrator; implementers must not each modify the root `Cargo.toml`. When this document was first added it only added task documents and created none of these directories; the T01 integrator has since created `compiler/` with the flat layout recorded in the T01 status below, while the group chip directories (`compiler/src/chips/<group>/`) and host code remain future work and do not exist yet.

T01 status (2026-10-04): the T01 integrator created `compiler/` as a nested
standalone package (path dependency on the root crate, its own lock file) and
implemented the C01–C06 foundation with a flat module layout
(`compiler/src/{arena,ids,task,bus,commit,...}.rs`) rather than the subdirectory
sketch above. The root framework remains domain-free; the root package defaults
are unchanged. Group chip files (`compiler/src/chips/<group>/`) and host code
remain future work and do not exist yet.
