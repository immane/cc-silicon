# Architecture

This page explains the parts of the system and how they interact. It
uses the circuit analogy throughout: if you know roughly how a
clocked digital circuit works, you already know the shape of this
project.

## The circuit, in 30 seconds

```text
                +-------------------------------+
                |            BUS                |
                |  registers (persist)          |
                |  wires (reset every tick)     |
                +---^-------------------^-------+
                    |                   |
        +-----------+--------+  +-------+--------------+
        | Motherboard        |  | Chips (one job each) |
        | - freezes pins     |  | - read pins + bus    |
        | - resets wires     |  | - never call others  |
        | - runs layers      |  | - emit proposals     |
        |   in order         |  |   onto wires         |
        | - latches state    |  |                      |
        | - advances tick    |  |                      |
        +-----------+--------+  +-------+--------------+
                    |                   |
              +-----v-------------------v-------+
              |  Backend (executes chips:       |
              |  CPU reference today)           |
              +-------------------------------+
```

One **tick** is one full cycle: sample and freeze the external inputs
(**pins**), reset the per-tick **wires**, run every chip in layer order,
then **latch** the results into persistent **registers** and advance
the clock. The motherboard owns this ordering; it is the only entity
allowed to invoke a chip.

## The five framework pieces (`src/`)

| Piece | Role | Circuit analogy |
|---|---|---|
| `Bus` | The single flat carrier of all shared state: registers plus one wire bundle | The board's traces and flip-flops |
| `LogicChip` / `RestrictedChip` | One stateless deduction per tick | A combinational logic gate |
| `Motherboard` | Layer pipeline and clock driver; the only legal tick ordering | The clock tree plus scheduler |
| `Backend` | Executes the chips (batching, offload, or emulation allowed if observable meaning is preserved) | The silicon substrate |
| `Clock` / `Testbench` / `simulate` | Wall-clock sampling and headless deterministic replay | The lab bench |

Two chip APIs exist. The legacy `LogicChip` receives the whole bus;
the stricter `RestrictedChip` receives only a read-only **projection**
prepared by a `ChipAdapter` and returns a typed **proposal** that the
adapter commits. New work should prefer `RestrictedChip` (declared with
the `silicon_chip!` macro). A known limitation, stated openly: Rust's
type system cannot prove a chip avoids global state, I/O, or
nondeterminism, so those properties are enforced by crate boundaries,
an AST linter (`tools/chip-lint`), and deterministic replay tests —
not by the compiler.

## The compiler foundation (`compiler/`)

The nested `compiler/` package is an **application** built on the
framework: it defines the compiler's bus, vocabulary, and rules of the
game. Its six frozen components (C01–C06) are:

1. **Storage (C01).** Typed record arenas addressed only by stable ID
   types. An ID from one arena can never be mistaken for another, and
   deleted slots leave tombstones instead of being silently reused.
2. **Task protocol (C03).** Work items move through
   Ready → Running → Waiting → Completed/Failed. Each task completes
   exactly once; each result is consumed exactly once.
3. **Proposals and commit.** Chips do not edit shared state. They emit
   change requests (enqueue, complete, fail, await host, store patch,
   append records, progress, await children), and one commit path
   checks ordering, ownership, and exactly-once rules before applying
   them. Think of it as: chips write memos, one clerk updates the
   ledger.
4. **Manifests (C04).** Every chip declares its identity: what it
   reads, what it writes, which phase it runs in, and which backend it
   supports. A validator checks these declarations; it cannot inspect
   chip logic.
5. **Snapshots (C05).** Every observable store encodes to canonical
   bytes. Replay the same inputs and you get byte-identical snapshots —
   this is what makes debugging and verification possible.
6. **Routing and dispatch (C06).** A selector matches each ready task
   to the chip responsible for it, within a per-tick proposal budget
   and bounded recovery.

The frozen identity is `t01-c01-c06/6`
(see [CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION)).
The hash pins the normative shapes and rule IDs — not the source code,
not the chip logic, and not a proof of correctness.

## The host boundary

File access, environment reads, subprocess execution, and artifact
persistence all live in **host code**, outside the chips. When a chip
needs a file read or a toolchain invocation, it emits a host request
and waits; the host's answer is frozen into the bus on a later tick.
This keeps the semantic core free of I/O and keeps every external
effect explicit in the trace.

## The two meanings of "backend"

This project uses the word twice, and confusing them causes real
misunderstandings:

- **`cc-silicon::Backend`** — the mechanism that *executes chips*
  (the CPU reference today; GPU/HDL tomorrow, maybe).
- **Compiler target backend** — the machine code the *compiled
  program* is generated for (AArch64 Linux first).

The framework backend is implemented. The AArch64 target's *identity*
is frozen but its concrete ABI values are **unverified** until a Linux
probe attests them, and code generation stays fail-closed until then.

## Sources

- Framework API walkthrough:
  [design/GETTING_STARTED.md](../design/GETTING_STARTED.md)
- Design rules:
  [design/ARCHITECTURAL_BLUEPRINT.md](../design/ARCHITECTURAL_BLUEPRINT.md)
- Formal contract: [architecture/SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md)
- Schema draft: [architecture/SFL_SCHEMA_DRAFT.md](../architecture/SFL_SCHEMA_DRAFT.md)
- Compiler contract: [tasks/T01_COMPILER_CONTRACT.md](../tasks/T01_COMPILER_CONTRACT.md)
