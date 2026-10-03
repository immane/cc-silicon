# LLM Parallel Development and Handoff Protocol

## 1. Ordering

**Wave 0 (must be done first)**: T00/H00–H03 and target probes, can run in parallel with T01/C01–C06. Implementation tasks are not dispatched until the T01 extension is approved and types/task/schema are compiled and frozen. During this period other implementers may study language rules / write test designs, but must not create incompatible interfaces on their own.

**Wave 1**: T02 control; T03 preprocessing; T04 lexical; T06 symbol/type; T08 layout/constant; T11 ABI foundation; T13 contract verification. Jointly read the frozen contract and each test with synthetic records.

**Wave 2**: T05 parsing; T07 semantic; T08 initialization; T09 IR; T11 basic instructions/stack/output; T12 extension analysis. Local parts may be implemented in parallel; integration requires real upstream artifacts and does not substitute mock success.

**Wave 3**: integrate M1/M2 closed loops; T10 local optimization; T11 complete ABI/allocation/floating point; T12 builtins/asm; T13 CFG/MI/ABI verification.

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

Before the storage extension is approved / the target is frozen, only dispatch the design, probe, and interface work for `H00–H03` and `C01–C06`; do not let other implementers assume that the protocol already exists. After the foundation is frozen, micro-batches may be launched as follows, with each row further split into one to five chips per implementer:

| Parallelizable task pool | Initial tasks | Independent fixture input | Integration waiting for |
|---|---|---|---|
| control | CT03–07, CT10 | manual task graph/proposals | C03/C06 |
| source/pp | PP01–04, PP06–08 | source bytes/pp tokens | source response protocol |
| literal | LX05–10, LX11–14 | pp spelling/target model | PP complete output |
| type | TY13–17, TY21–29 | manual type records | declaration/sem consumers |
| scope | TY01–09 | manual scope/name records | PA04/14/28 |
| layout | CL08–14 | manual types/members/pack | aggregate declarations |
| backend ABI | CG01–05 | manual signature/layout | TY28 and IR18 |
| verifier | VF01–04, VF07–08 | positive/negative task/store/CFG snapshots | real records from each stage |

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
