# Compilation Walkthrough

This page follows one tiny C program through the **planned** compiler.
Almost every step below is still unimplemented — the point is to show
what the finished chain should look like, so that later progress is
easy to place on the map.

The program:

```c
int main(void) { return 2 + 3; }
```

The expected end result: an AArch64 executable that exits with code 5.

## The journey, stage by stage

```text
source text
  |  (T03) preprocess: handle includes, macros, conditionals
  v
preprocessing tokens
  |  (T04) lex: decode numbers, strings, operators into final tokens
  v
C tokens
  |  (T05) parse: build the syntax tree for `return 2 + 3;`
  v
syntax tree
  |  (T06) symbols/types: resolve `main`, type `2 + 3` as int
  v
typed tree
  |  (T07) semantics: check the return statement is legal
  v
checked tree
  |  (T08) constants/layout: fold `2 + 3` to the constant 5
  v
  |  (T09) IR lowering: emit explicit control-flow instructions
  v
intermediate representation
  |  (T10) optimize (optional): simplify, within legality rules
  v
  |  (T11) target emission: AArch64 instructions, AAPCS64 calling rules
  v
assembly -> (host assembler/linker) -> executable -> exit code 5
```

Target-dependent steps (the second half of T11) are **probe-gated**:
they stay disabled until a Linux probe measures the real ABI values
and an attestation accepts them. The frontend half (through IR) is
target-independent and is the first implementation goal.

## What "chips doing work" looks like

Pick one concrete moment: the lexer has produced tokens and the parser
chip is scheduled. In slow motion:

1. **Dispatch.** The motherboard selects the ready parse task and
   invokes the responsible chip with a frozen view of the bus.
2. **Compute.** The chip reads only its declared inputs (the token
   records) and returns a proposal, e.g. "append these syntax-tree
   nodes to the AST arena".
3. **Commit.** The single commit path checks the proposal — is the
   task still running, is this chip allowed to write those records,
   does the budget allow it — and only then applies it.
4. **Latch.** At tick end, the new nodes become visible registers;
   downstream chips (symbol resolution, type checking) become
   schedulable on later ticks.
5. **Snapshot.** Every tick's observable state encodes to canonical
   bytes, so the whole compilation can be replayed and diffed.

If anything goes wrong — bad syntax, an unsupported construct, a full
arena — the outcome is a **structured diagnostic**, never a panic used
as control flow and never a faked success.

## Where the host steps in

Three moments in this walkthrough cross the host boundary:

- Reading the `.c` file from disk (a `read_source` request).
- Invoking the external assembler and linker (an `invoke_toolchain`
  request) — the compiler emits assembly; it does not assemble it.
- Running the executable to check the exit code (test harness, not a
  chip).

Each crossing is an explicit request with a frozen answer, so the
trace shows exactly what the outside world contributed.

## What you can run today

The walkthrough above is the plan. What actually executes today is the
framework's counter example, which demonstrates the tick discipline on
a trivial circuit:

```bash
cargo run --example counter
```

It implements a small bus, two chips, two layers, and ticks — the same
mechanics the compiler will one day ride on, with none of the language
content. Full setup is in
[design/GETTING_STARTED.md](../design/GETTING_STARTED.md).

## Sources

- Frontend acceptance fixtures (planned):
  [tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md](../tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md)
- End-to-end acceptance, including the probe-gated target half:
  [tasks/M1_TARGET_ACCEPTANCE.md](../tasks/M1_TARGET_ACCEPTANCE.md)
- Per-stage chip tables (planned work, not code):
  [tasks/README.md](../tasks/README.md) §3
