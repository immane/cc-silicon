# M1: Target Acceptance Plan — First Vertical Closed Loop

Status: **planned document; M1 is not implemented.** No compiler, assembler,
linker, runner, or test has been executed for M1. Every check and probe below is
explicitly **NOT RUN**. This document claims no pass rate and no >99% result; it
is a plan for what must be built and verified.

This document is a task design artifact. It does **not** freeze any interface,
does **not** patch the root framework, and does **not** authorize C language
chips. Mutable contract type names and record shapes are deliberately not
pinned here; they are owned by the relevant task groups.

References: [T00](T00_GCC_TORTURE_GATE.md), [T01](T01_COMPILER_CONTRACT.md),
[T09](T09_IR_LOWER_CHIPS.md), [T11](T11_TARGET_CHIPS.md),
[T13](T13_VERIFICATION_CHIPS.md), [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md),
[TASK_TEMPLATE.md](TASK_TEMPLATE.md),
[ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md),
[SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md), and
[compiler/README.md](../../compiler/README.md).

---

## 1. Scope, Authority, and Current State

### 1.1 Milestone definition

M1 is the first vertical closed loop (see the milestone table in
[docs/tasks/README.md](README.md)):

> `int main(void){return 2+3;}`; lexical/parse/type/IR/ABI/assembly; the
> candidate generates the code itself, an external assembler/linker is used, and
> the run returns `5`.

The M1 source is a hand-written fixture, not a GCC torture case. Passing M1 must
never be reported as a torture pass rate or as evidence toward the >99% gate.

### 1.2 Frozen by decision (authority: user, T01 integration, 2026-10-04)

- CPU dynamic arena extension is **approved for the compiler application only**;
  the root framework remains fixed-array, heap-free, `#![forbid(unsafe_code)]`
  ([ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md)).
- Target **identity** is frozen: `aarch64-unknown-linux-gnu`, ELF, LP64,
  little-endian, AAPCS64.
- Substrate plan: Linux CI/VM (planned, **not provisioned**).
- Corpus acquisition: fetch-on-demand, hash-locked, **no vendoring**.
- Reference-only DejaGnu baseline is **authorized** as an oracle, **not
  available**, and **never candidate compiler evidence**.

### 1.3 Not frozen, not verified, not implemented

- Concrete target values (scalar sizes/alignments, `long double` format,
  `wchar_t` signedness, ABI register/save-area values) are **UNVERIFIED** until a
  Linux probe runs.
- Assembler, linker, sysroot, runner, and execution substrate are **unresolved**
  (see [T00_H00_IMPLEMENTATION_STATUS.md](T00_H00_IMPLEMENTATION_STATUS.md)).
- Language-record schemas and per-group task/result payload variants are
  **not frozen**; their stores are owned by the relevant task groups.
- There is **no C compiler, no language chips, and no code generation**. The
  compiler package currently contains only the contract foundation envelope.
- The target-codegen path must remain **fail-closed** while the target is
  unverified (§5).

### 1.4 Verified current state (read 2026-10-04; nothing re-run here)

| Fact | Current evidence |
|---|---|
| Arena extension approved | [ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) (Status: Accepted); [T01](T01_COMPILER_CONTRACT.md) §1 |
| C01–C06 envelope records a frozen artifact version/hash, backed by a freeze test | `version=t01-c01-c06/2`, hash `c50009bc…827af96` in [CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION) and [T01](T01_COMPILER_CONTRACT.md) §7.1; freeze test at [compiler/tests/freeze.rs](../../compiler/tests/freeze.rs). Integrator reran the compiler package checks, freeze-hash example, and root checks on 2026-10-04; the independent audit is pending. |
| Language-store record schemas are placeholders, not frozen | [T01](T01_COMPILER_CONTRACT.md) §7.1 (C01–C03 rows); [compiler/README.md](../../compiler/README.md) |
| Target identity frozen; concrete values unverified; codegen refuses until probed | [T01](T01_COMPILER_CONTRACT.md) §6; [aarch64-linux-probe.txt](../../compiler/contracts/target/aarch64-linux-probe.txt); [compiler/README.md](../../compiler/README.md) |
| No worker handlers or language chips exist | [T01](T01_COMPILER_CONTRACT.md) §7.1 (C06 row); `compiler/src/chips/` does not exist |
| Compiler package is covered by root CI via explicit manifest path | [.github/workflows/ci.yml](../../.github/workflows/ci.yml) |
| Probe/runner/reference baseline unresolved | [T00_H00_IMPLEMENTATION_STATUS.md](T00_H00_IMPLEMENTATION_STATUS.md) |

Claim discipline: the C01–C06 foundation is described as **recorded frozen**,
not as a universally approved parser or code generator. Only the storage
extension, the target identity, and the C01–C06 envelope are in scope of that
freeze. Everything else in [T01](T01_COMPILER_CONTRACT.md) remains a proposal
until it is compiled and tested by its owner.

---

## 2. M1 End-to-End Expectations

Fixture, byte-exact (LF-terminated, no BOM):

```c
int main(void){return 2+3;}
```

For M1, pin an **explicit** dialect and options (for example `-std=c11 -O0`).
Do not assume the reference default dialect; [T00](T00_GCC_TORTURE_GATE.md) §2
requires the default to be confirmed by release probes (H07).

| Stage | Expected result | Owner |
|---|---|---|
| Preprocess | Candidate's own preprocessing pass runs; no external `-E`, no `gcc -E`. The file has no directives, but the stage must be exercised or explicitly bypassed with a recorded decision. | T03 |
| Lex | Final token stream for the fixture | T04 |
| Parse | One function definition `main` with parameter list `(void)` (no parameters) and a `return` statement | T05 |
| Symbol/type | `int` scalar, `void` parameter list, function type `int (void)`, external `main` | T06 |
| Semantic | Return operand checked; no implicit conversion needed; type is `int` | T07 |
| Constant/layout | Integer constants `2`, `3`; either a folded `5` or an integer add; `int` layout from the frozen target model | T08 |
| IR lower | Typed CFG/IR with the invariants in §3.2 | T09 |
| IR interpret (verification-only) | Modeled return value `5` | T13 |
| Target code | Candidate emits AArch64 GNU/Linux assembly (not Mach-O, not Darwin ABI) | T11 |
| Assemble | External assembler produces a valid AArch64 ELF object | H01/Host |
| Link | External linker (or link-only driver invocation) on the candidate object only | H01/Host |
| Run | Process exits with status **5** | H01/Host runner |

Compile gate: the output at the compile step must be an assembled **object**
([T00](T00_GCC_TORTURE_GATE.md) §1.1); emitting `-S` only is not a pass.

Non-goals: no optimization (T10) and no GNU/builtin handling (T12) are required
for this fixture; their absence must be explicit, not a silent no-op.

---

## 3. Target-Independent Acceptance (runnable without a Linux probe)

These checks exercise only the domain-free front end and IR in Rust. They may be
planned on the dev host because they do not test AArch64 execution. They are
blocked on the language-record schema freeze **and** on the corresponding
front-end/IR chip implementations (§8). **All are NOT RUN.**

### 3.1 Front-end and typed-AST invariants

| # | Invariant the T05/T06/T07 owners must make checkable |
|---|---|
| A1 | Exactly one translation unit and one function definition `main`; no other definitions |
| A2 | `main` has type `int (void)`; `(void)` means **no** parameters |
| A3 | The returned operand is fully checked (value category, type `int`); no unchecked node reaches lowering |
| A4 | `2`, `3`, and `2+3` are `int`; no implicit conversion; no host width or host `sizeof` leakage |
| A5 | Tokens and spans are in range and ordered; a single EOF; no fabricated macro provenance |

### 3.2 IR invariants

Record field names and the exact IR schema are **not frozen**; the following are
the semantic obligations the T09 owner must make expressible and the T13
verifier must check. Chip references are catalog IDs only, not type bindings.

| # | Invariant |
|---|---|
| IR-1 | One IR function for `main`; ABI-neutral signature `int (void)`; external linkage; no aggregate or variadic |
| IR-2 | Exactly one basic block, the entry block; exactly one terminator (a return); **no instruction appended after the terminator** |
| IR-3 | No branch/loop/switch edges; no dangling, unreachable, or duplicate-terminator blocks |
| IR-4 | Return operand type is `int`; operand/result types are consistent; memory-effect fields are absent or provably empty |
| IR-5 | The result is either a folded `int` constant `5` **or** an `int` add of `2` and `3`; both forms are accepted; identical input must deterministically yield the same form |
| IR-6 | Integer records carry target bit width + bit pattern + signedness; no host overflow is used to compute the value |
| IR-7 | No memory operations, calls, volatile/atomic, floating point, or aggregate copy appear; legal-falloff handling must not synthesize a spurious `0` return for the explicit-return case |
| IR-8 | Function/block/value/instruction IDs are stable, in production order, never address-derived; no tombstones on the happy path |
| IR-9 | Non-SSA is permitted and must not be mis-flagged as SSA by the verifier |

### 3.3 IR interpreter gate (verification-only)

- Execute the supported IR subset with no memory image and no host input; assert
  the modeled return value is `5`.
- Any unsupported operation must be reported as `Unsupported`, never silently
  equal. None is expected for this fixture.
- This gate proves the candidate's own lowering independently of AArch64. It
  does **not** replace final target execution and must not be reported as the
  target result.

### 3.4 Directive-free preprocessing

The fixture needs no directives, so the T03 stage may be a documented minimal
pass-through. Whatever the decision, record it and ensure an **external
preprocessor is never used for the end-to-end claim**
([T00](T00_GCC_TORTURE_GATE.md) §3; [T11](T11_TARGET_CHIPS.md) preamble).

---

## 4. Linux-Probe-Gated Acceptance (must await the probe)

These checks **cannot** be accepted from the macOS host and must wait for a
provisioned Linux CI/VM plus a frozen cross toolchain, sysroot, and runner. They
are listed as the exact expectations to check once the gate opens. **All are NOT
RUN.**

### 4.1 ELF structural assertions

These are derivable from the frozen target identity and the public AArch64 ELF
ABI, not from any macOS fact.

| # | Assertion | Observation |
|---|---|---|
| B1 | ELF magic, 64-bit class, little-endian data encoding, current version | header inspection |
| B2 | `e_machine` is AArch64, for both object and executable | header inspection |
| B3 | `-c` output is a relocatable object; linked output is a valid AArch64 Linux executable | header inspection |
| B4 | `.text` exists with allocate + execute flags | section inspection |
| B5 | `main` exists as a defined global function symbol (not `_main`, not local) | symbol inspection |
| B6 | No Mach-O magic or Mach-O containers appear anywhere in the pipeline | file/header inspection |

### 4.2 Assembly-text no-Darwin assertions

Scan the candidate's emitted assembly. It must define `main` and must **not**
contain Mach-O or Darwin markers, including:

- a leading-underscore `main` symbol or calls to underscore-prefixed symbols;
- `__TEXT` / `__text` sections or Mach-O section directives;
- Mach-O-only directives or local-label conventions;
- the Darwin arm64 stack-only variadic model.

It must use GNU-assembler / Linux AArch64 syntax (for example `.text`,
`.global main`, `.type main, %function`, `.size main, .-main`), or the documented
equivalent accepted by the frozen assembler. The identity rules come from
[T11](T11_TARGET_CHIPS.md).

### 4.3 AAPCS64 function-level assertions

| # | Assertion |
|---|---|
| B7 | The `int` return value is delivered in the AAPCS64 integer result register, and equals `5` at the return point |
| B8 | The stack pointer is 16-byte aligned at every public interface / call boundary and at return |
| B9 | If a frame is emitted, callee-saved registers are preserved, the stack pointer is restored, and the link register is not corrupted |
| B10 | A leaf function may omit a frame; both forms are accepted if B7–B9 hold |
| B11 | `main` is an ordinary AAPCS64 function invoked by the C runtime; not `_main`, not the Darwin variadic model |

Concrete numbers (`gp_arg_regs`, `fp_arg_regs`, `stack_align`,
`variadic_register_save_area`, `sizeof(int)`, …) stay **UNVERIFIED** until the
probe; B7–B9 are checked against probe-confirmed values, not assumed ones.

### 4.4 Run — expected exit status 5

- Execute the produced ELF on the frozen substrate (native Linux preferred; a
  documented emulator such as `qemu-aarch64` is acceptable only if its exit-code
  propagation is itself verified and recorded).
- Pass criterion: the process terminated **normally** (not by signal) and its
  exit status is exactly `5` (the low 8 bits of `main`'s return).
- The program must not write output that would corrupt the runner's
  classification.
- The compile gate object from §2 must exist and pass B1–B5; `-S` only is not a
  pass.

### 4.5 Values that must await the probe

Do not accept, and do not substitute from macOS arm64:

1. Every required probe field: scalar sizes/alignments, `char` signedness,
   `long double` format, `wchar_t` signedness.
2. ABI values: general/FP argument register counts, stack alignment, variadic
   register save area.
3. Assembler/linker acceptance of the emitted directives, pseudo-instructions,
   operand ranges, and relocations.
4. Link defaults: PIE vs non-PIE, startup object selection, dynamic loader path,
   symbol visibility/PIC behavior.
5. Runner exit-code propagation and signal mapping.
6. The reference-only baseline's default dialect and options on the same target
   (oracle only, never candidate evidence).
7. Availability and exact names/flags of inspection tools on the substrate.

The concept "target concrete values are verified" means a recorded probe report
with a content hash, as required by [T01](T01_COMPILER_CONTRACT.md) §6.

---

## 5. Fail-Closed Rule for Target Code Generation

- While the target is unverified, the configuration must **refuse** code
  generation with a structured target diagnostic. This is the current recorded
  behavior described in [compiler/README.md](../../compiler/README.md) and
  [T01](T01_COMPILER_CONTRACT.md) §7.1; this plan does not re-run it.
- Consequence: the **target-dependent half of M1 (Part B) is blocked by design**
  until the probe runs. Only Part A (target-independent) plus the fail-closed
  negative path can be accepted before then.
- No macOS/Darwin ABI, object-format, or `long double` value may be used as a
  substitute at any point ([T01](T01_COMPILER_CONTRACT.md) §6;
  [T11](T11_TARGET_CHIPS.md) preamble).

---

## 6. Replay and Determinism Semantics

| # | Check | Pass criterion |
|---|---|---|
| D1 | Two full compiles from identical initial state, configuration, and source | Identical canonical snapshot hash and trace hash |
| D2 | At least three consecutive runs | All three hashes equal |
| D3 | Same IR to emitted assembly | Byte-identical assembly artifact |
| D4 | Snapshot content | Excludes addresses, host paths, and wall-clock data; includes contract version and configuration hash |
| D5 | Scheduling and queue order | Stable; new enqueues are visible next tick; results are consumed exactly once |
| D6 | ELF artifact across runs | **Semantic** equality only (identical structural/symbol summaries and identical exit status). Do **not** require byte-identical ELF; build-id or timestamps may legitimately differ. Record this distinction. |
| D7 | Two-trace comparison | First divergence is reported, or none |

These follow the deterministic serialization and replay obligations in
[T01](T01_COMPILER_CONTRACT.md) §7 (C05) and
[T13](T13_VERIFICATION_CHIPS.md).

---

## 7. Structured Failures and Negative Paths

Every failure must be a typed diagnostic; the task must terminate
`Completed`/`Failed` exactly once or explicitly wait, never spin. No partial
artifact may be committed on failure. Error inputs are never counted as passes.

| # | Input / condition | Expected structured outcome | Owner |
|---|---|---|---|
| N1 | Codegen requested while the target is unverified | Structured target/config diagnostic; refuse to emit | T01/C02 |
| N2 | Unsupported construct for the M1 subset (for example a floating type or a function call in the return expression) | Explicit `Unsupported` diagnostic; no code emitted, no silent truncation | T07/T09/T11 |
| N3 | Malformed input (syntax error) | Parse-family diagnostic; no fabricated IR | T05/T13 |
| N4 | Explicitly unsupported task kind | `Failed` with an unsupported diagnostic | T02/C06 |
| N5 | Registered task kind with no installed handler | `Failed` with "no handler installed" | T02/C06 |
| N6 | Unregistered task kind | `Failed` with "not registered" | T02/C06 |
| N7 | Invalid store patch (wrong owner, undeclared field, stale version, bad shape) | Structured commit error; **nothing committed** (atomic) | T01/CT06 |
| N8 | Target verification negative fixture (wrong stack alignment, clobbered callee-save, unassigned virtual register) | Rejected validated artifact | T11/T13 |
| N9 | Budget or tick limit reached | Reported budget exhaustion; no idle spin | T02 |
| N10 | Review rule | No filename or source-text special-casing; no "return success for unimplemented" | Reviewer |

---

## 8. Task Owners and Dependencies

Owner names follow [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) and
[T00](T00_GCC_TORTURE_GATE.md). "Integrator" is the T01 contract integrator.
Gate values describe the current state, not a promise.

| ID | Owner | Deliverable needed for M1 | Current state | Gate |
|---|---|---|---|---|
| P0 | Integrator | Durable arena/target decision | [ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) present (Accepted) | Recorded |
| P1 | Integrator | C01–C06 envelope compiles with a recorded version/hash and freeze test | Recorded `t01-c01-c06/2`; compiler tests and hash recomputation rerun on 2026-10-04 | Recorded |
| P2 | Integrator + group owners | Freeze language-record schemas and per-group task/result variants | Placeholders only | **Blocking** for chip work |
| P3 | Integrator | Registration and motherboard wiring | Routing shell only; no handlers | **Blocking** |
| P4 | H01 | Linux CI/VM substrate, assembler, linker, sysroot, runner | Unresolved; not provisioned | **Blocking Part B** |
| P5 | H01/H07 | Target probe producing a hashed record | Not run | **Blocking Part B** |
| P6 | H00 | Corpus lock and license/provenance records | Scaffold only; lock draft | Required for T00, not for the M1 executable |
| P7 | H04 | Candidate driver flags `-E/-S/-c/-o/-I/-D/-U/-std` | Absent | **Blocking** |
| P8 | H05 | Isolated execution, limits, timeouts, cancellation/crash classification | Absent | **Blocking** for run |
| P9 | T02 | Job bootstrap, selection, phase advance, commit/diagnostics | Shell only | **Blocking** |
| P10 | T03 | Candidate preprocessing pass (record the no-external-`-E` decision) | Not implemented | Blocking or explicitly bypassed |
| P11 | T04 | Lexer for the fixture | Not implemented | **Blocking** |
| P12 | T05 | Parser for `main`, `(void)`, block, `return` | Not implemented | **Blocking** |
| P13 | T06 | `int`, `void` parameter list, function type, `main` | Not implemented | **Blocking** |
| P14 | T07 | Return/type legality and constant-expression checks | Not implemented | **Blocking** |
| P15 | T08 | Integer constants and `int` layout | Not implemented | **Blocking** |
| P16 | T09 | IR lower for this fixture and the frozen IR schema | Not implemented | **Blocking** |
| P17 | T11 | AArch64 assembly/ELF emission and target verification | Not implemented | **Blocking Part B** |
| P18 | T13 | IR/CFG/ABI verification, interpreter, replay, outcome classification | Not implemented | **Blocking** for evidence |
| P19 | T12, T10 | Not required for this fixture; absence must be explicit | Not implemented | Non-goal |

Wave ordering (see [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md)): Wave 0 is
P0–P6; Wave 1 is T02/T03/T04/T06/T08/T11-foundation/T13; Wave 2 adds T05/T07/T09
and T11 instructions; Wave 3 is M1 integration.

---

## 9. Evidence to Collect (planned; NOT RUN)

All commands below are **planned only**. They are not a run record.

Part A (target-independent, after P2/P3):

```text
candidate -std=c11 -O0 -S -o main.s main.c
candidate --emit-ir-snapshot --emit-trace ...
candidate --interpret-ir main.ir          # expect modeled return 5
candidate ... invalid_type.c              # expect structured diagnostic, no output
```

Part B (Linux substrate, after P4/P5):

```text
<frozen-assembler> -o main.o main.s
<frozen-link-command> -o a.out main.o     # never passes main.c to any C compiler
<header/symbol/section inspection> main.o
<header/symbol/section inspection> a.out
./a.out ; echo $?                         # expect 5
```

Repeat the Part A and Part B chains three times and compare snapshot, trace, and
assembly hashes; compare ELF output semantically only (D6).

Foundation checks already recorded by the compiler package (not re-run here) are
listed in [compiler/README.md](../../compiler/README.md) and
[T01](T01_COMPILER_CONTRACT.md) §7.1.

---

## 10. Non-Claims and Review Rules

- This document is a **plan**. It reports no executed test, no probe, no pass
  rate, and no compiler.
- The >99% GCC torture gates in [T00](T00_GCC_TORTURE_GATE.md) are not met,
  measured, or implied. M1 is a single hand-written fixture.
- The target identity is frozen; concrete ABI, toolchain, sysroot, and runner
  values are not. Nothing here may be used to claim otherwise.
- The candidate compiles C itself. GCC/Clang may be used only as a reference
  oracle and must never generate the candidate program; no external
  preprocessor may be used for the end-to-end claim.
- No behavior may be special-cased by filename or test text, and unimplemented
  behavior must never be reported as success.

---

## 11. References

- [docs/tasks/README.md](README.md) — milestones and acceptance
- [T00_GCC_TORTURE_GATE.md](T00_GCC_TORTURE_GATE.md) — corpus, runner, denominator
- [T01_COMPILER_CONTRACT.md](T01_COMPILER_CONTRACT.md) — bus, IDs, protocol, target model
- [T09_IR_LOWER_CHIPS.md](T09_IR_LOWER_CHIPS.md) — AST-to-CFG/IR lowering
- [T11_TARGET_CHIPS.md](T11_TARGET_CHIPS.md) — AArch64 ABI, MI, assembly
- [T13_VERIFICATION_CHIPS.md](T13_VERIFICATION_CHIPS.md) — verification, replay, differential
- [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) — waves, ownership, handoff
- [TASK_TEMPLATE.md](TASK_TEMPLATE.md) — per-chip header and field binding
- [T00_H00_IMPLEMENTATION_STATUS.md](T00_H00_IMPLEMENTATION_STATUS.md) — corpus tooling status
- [ADR-0001-COMPILER-DYNAMIC-ARENA.md](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) — approved storage extension and frozen target identity
- [SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md) — semantic core and boundaries
- [SILICON_PARADIGM_SPEC.md](../architecture/SILICON_PARADIGM_SPEC.md) — paradigm specification
- [ARCHITECTURAL_BLUEPRINT.md](../design/ARCHITECTURAL_BLUEPRINT.md) — application design guide
- [compiler/README.md](../../compiler/README.md) — foundation package status and limitations
- [compiler/contracts/CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION) — recorded foundation artifact
- [compiler/contracts/target/aarch64-linux-probe.txt](../../compiler/contracts/target/aarch64-linux-probe.txt) — unverified probe fixture
