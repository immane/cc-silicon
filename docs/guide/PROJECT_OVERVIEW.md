# Project Overview

## The goal in one paragraph

We are building a **deterministic C compiler** structured like a
synchronous digital circuit: external input is frozen once per tick,
small stateless units called **chips** each perform one isolated piece
of reasoning, and all shared state advances through one explicit,
replayable path. The end goal is a compiler whose behavior is exactly
reproducible — same inputs, same configuration, same result, bit for
bit — and whose acceptance is measured against the GCC C torture suite.

## Why chips instead of ordinary function calls?

In a conventional compiler, one phase calls the next directly, and any
module can keep private caches or reach into shared objects. That is
flexible, but it makes two things hard:

1. **Reproducibility.** Hidden state, call order, and pointer addresses
   can leak into the output. Two runs of "the same" compilation can
   then disagree in subtle ways.
2. **Parallel authorship.** If every module can touch everything, 331
   separately authored pieces cannot be developed independently without
   constant integration conflicts.

The chip model answers both with physical discipline:

- All shared state lives in **one flat bus** that everyone can see.
- Chips are **stateless**: they carry no fields and keep nothing
  between ticks.
- Chips **never call each other**. They communicate only through the
  bus, and every change goes through a single commit path.
- Time advances in **ticks** with a fixed order: sample inputs, reset
  per-tick signals, run chips in layer order, latch the new state.

Nothing about this guarantees correctness by itself. It only guarantees
that whatever happens is **visible and replayable**: every intermediate
state can be snapshotted, and replaying the same inputs yields byte-
identical snapshots. Correctness still has to be built and tested chip
by chip.

## The three things in this repository

| Piece | Where | State |
|---|---|---|
| Generic execution framework (bus, chips, motherboard, backend, clock, testbench) | `src/` | **Implemented**, with tests and a runnable counter example |
| Compiler contract foundation (storage, IDs, task protocol, target model, manifest check, snapshots, dispatcher) | `compiler/` | **Implemented and frozen** as `t01-c01-c06/6`; language behavior still unwritten |
| C language pipeline (preprocessing through target emission, ~331 planned chips) | `docs/tasks/T02`–`T13` | **Planned only** — task descriptions and acceptance plans, no code |

A common misunderstanding is worth clearing up early: the frozen
contract is **an agreement about shapes, names, and protocol rules**,
not a working compiler. Freezing it means later authors cannot
silently redefine the shared vocabulary; it does not mean any C code
can be compiled yet. See [Project status](PROJECT_STATUS.md).

## How a compilation is supposed to flow

At a high level, the planned flow looks like this:

```text
C source files
      |
      v
Host boundary (reads files, runs tools, keeps artifacts)
      |
      v
Compiler pipeline: preprocess -> lex -> parse -> symbols/types
      -> semantics -> constants/layout -> IR -> optimize
      -> AArch64 emission
      |
      v
Assembled object + run result
```

Each pipeline stage is a group of chips. A chip never performs file
I/O or calls a toolchain itself; it **asks the host** through an
explicit request, and the host's frozen answer comes back on a later
tick. Likewise, a chip never edits shared records directly; it submits
a **proposal** (a change request), and the commit path validates and
applies it. Walk through a concrete program in
[Compilation walkthrough](COMPILATION_WALKTHROUGH.md).

## What success looks like

Final acceptance is defined in
[tasks/T00_GCC_TORTURE_GATE.md](../tasks/T00_GCC_TORTURE_GATE.md):
after freezing the GCC revision, target, test list, and option matrix,
the compile, execute, and execute/ieee pass rates must each exceed 99%,
with no gaming (no skipping hard tests, no editing test sources, no
delegating to another C compiler). That gate has **not been run**; the
corpus has not even been fetched yet.

The nearer milestone is M1: compile `int main(void){return 2+3;}`
through the candidate's own pipeline and get exit code 5. That is also
**still ahead of us** — it is the first visible proof that the whole
chain connects.

## Sources

- Task master plan: [tasks/README.md](../tasks/README.md)
- Acceptance gate: [tasks/T00_GCC_TORTURE_GATE.md](../tasks/T00_GCC_TORTURE_GATE.md)
- Paradigm specification:
  [architecture/SILICON_PARADIGM_SPEC.md](../architecture/SILICON_PARADIGM_SPEC.md)
- Frozen contract identity:
  [compiler/contracts/CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION)
