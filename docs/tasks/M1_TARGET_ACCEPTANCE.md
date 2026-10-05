# M1: Target Acceptance Plan — First Vertical Closed Loop

Status: **planned document; M1 DRAFT, not implemented; target acceptance not
passed.** No compiler, assembler, linker, runner, or test has been executed for
M1. Every check and probe below is explicitly **NOT RUN**. This document claims
no pass rate and no >99% result; it is a plan for what must be built and
verified. `/5` remains current; the two-tier `/6` hash boundary is accepted
**conceptually** (rev 50: the frozen `/6` seed `foundation + M1AppendSchema` is
hashed, with post-seed runtime `declare()` declarations excluded) but **no `/6`
hash/seed values or freeze exist**. Part B (target-dependent) acceptance is
**unavailable**; H6's mechanism direction is selected but its implementation
remains pending; partial decisions exist (CDR rev 42–55, incl. the rev-47/51/53
delegated candidate defaults; current CDR rev 55) with the overall owner
bundles pending.

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
  the root framework remains `#![forbid(unsafe_code)]`, and its fixed-array /
  no-heap rule applies to bus data, not to the whole crate — topology
  construction allocates, the default tick hot path does not
  ([ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) §2).
- Target **identity** is frozen: `aarch64-unknown-linux-gnu`, ELF, LP64,
  little-endian, AAPCS64.
- Substrate plan: Linux CI/VM (planned, **not provisioned**).
- Corpus acquisition: fetch-on-demand, hash-locked, **no vendoring**.
- Reference-only DejaGnu baseline is **authorized** as an oracle, **not
  available**, and **never candidate compiler evidence**.

### 1.3 Not frozen, not verified, not implemented

This subsection records scoping facts. The **Part B** (target-dependent) items
among them do not gate the **Part A** symbolic contract or its probe-independent
fixtures (§3); Part A check execution is separately gated by the language-record
schema freeze and the front-end/IR chip implementations (§3, §8).

- Concrete target values (scalar sizes/alignments, `long double` format,
  `wchar_t` signedness/encoding, ABI register/save-area values) are **UNVERIFIED**
  until a Linux probe runs. They are a **Part B-only** concern: the Part A symbolic
  contract and fixtures use symbolic ranks/signedness and do not depend on them
  (§3.2 IR-6).
- Assembler, linker, sysroot, runner, and execution substrate are **unresolved**
  (see [T00_H00_IMPLEMENTATION_STATUS.md](T00_H00_IMPLEMENTATION_STATUS.md)).
  They are **planned Part B expectations, not available facilities**.
- Language-record schemas and per-group task/result payload variants are
  **not frozen**; their stores are owned by the relevant task groups.
- There is **no C compiler, no language chips, and no code generation**. The
  compiler package currently contains only the contract foundation envelope.
- The target-codegen path must remain **fail-closed** while the target is
  unverified (§5). This restriction applies to the **Part B** target-codegen
  path only; it does not constrain the Part A symbolic contract or its
  probe-independent fixtures.

### 1.4 Verified current state (read 2026-10-04; nothing re-run here)

| Fact | Current evidence |
|---|---|
| Arena extension approved | [ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) (Status: Accepted); [T01](T01_COMPILER_CONTRACT.md) §1 |
| C01–C06 envelope records a frozen artifact version/hash, backed by a freeze test | `version=t01-c01-c06/5`, hash `61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5` in [CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION) and [T01](T01_COMPILER_CONTRACT.md) §7.1; freeze test at [compiler/tests/freeze.rs](../../compiler/tests/freeze.rs). Integrator reran the compiler package checks, freeze-hash example, and root checks on 2026-10-04; the independent audit is pending. |
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

The frozen C01–C06 version/hash (`t01-c01-c06/5`) is the **foundation envelope
only**: it is not the M1 language schema and not the `/6` contract revision,
which remain **not frozen** (§3.2). The frozen target is the **target identity
only** (`aarch64-unknown-linux-gnu`, ELF, LP64, little-endian, AAPCS64); the
concrete target values and toolchain/sysroot/runner are **not validated** by that
freeze and stay **UNVERIFIED** until the Linux probe (§1.3, §4).

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
| Constant/layout | Integer constants `2`, `3`; the folded `5` (the `Add` form is optional/verifier-only); **`int` is symbolic rank + signedness in Part A** — concrete `int` size/alignment is probe-gated Part B (`M1-CL-02`). The Part A folded-`int 5` producer (**T09 owns IR `Constant`; T08 owns `ConstRecord.value`**) is separate from the probe-gated fixture `M1-CL-02` (**executed by chip `CL03`**; chip `CL02` `ConstantUnaryChip` is unexercised). | T08 |
| IR lower | Typed CFG/IR with the invariants in §3.2 | T09 |
| IR interpret (verification-only) | Modeled return value `5` | T13 |
| Target code | Candidate emits AArch64 GNU/Linux assembly (not Mach-O, not Darwin ABI) | T11 |
| Assemble | External assembler produces a valid AArch64 ELF object | H01/Host |
| Link | External linker (or link-only driver invocation) on the candidate object only | H01/Host |
| Run | Process exits with status **5** | H01/Host runner |

Compile gate: the output at the compile step must be an assembled **object**
([T00](T00_GCC_TORTURE_GATE.md) §1 item 1); emitting `-S` only is not a pass.

The `Target code`, `Assemble`, `Link`, and `Run` rows are **planned Part B
expectations, not available facilities**: the assembler, linker, and runner are
not provisioned (§1.3, §4), so those rows cannot be exercised until the Part B
gate opens. The preceding rows (preprocess through IR interpret) are Part A and
are target-independent, **except the concrete target-value/layout clause of the
Constant/layout row, which is Part B/probe-gated**.

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
| A3 | The returned operand is fully checked (value category, type `int`); no unchecked node reaches lowering. Every checked node — **including the `Return` node and the `FunctionDefinition` node** — has exactly one committed T07-owned `SemRecord`; the `FunctionDefinition` `SemRecord` carries the checked signature `TypeId`. T09 reads those committed `SemRecord`s without re-inferring. **T07 does not consume a T09-created `FunctionRecord`** (it is created in the later **ir** stage, so using it would be a temporal cycle); the **selected draft direction** is that there is **no** `FunctionContextId` (accepted subdecision, CDR rev 42; T07 rev 1 amended `CheckRequest` to `(NodeId, ScopeId)`), and a parse `ContinuationRecord` alone is not a proxy. The exact `SemRecord` field/carrier encoding (including the typed T09 link) remains an open T07/T01 `/6` co-freeze item (overall F bundle pending) |
| A4 | `2`, `3`, and `2+3` are `int`; no implicit conversion; no host width or host `sizeof` leakage |
| A5 | Tokens and spans are in range and ordered; a single EOF; no fabricated macro provenance |

### 3.2 IR invariants

Record field names and the exact IR schema are **not frozen**; the following are
the semantic obligations the T09 owner must make expressible and the T13
verifier must check. Chip references are catalog IDs only, not type bindings.
**T13 scope (T13 rev 8):** `VF07`–`VF11` are **out of the M1 T13 chip set**, so
the `VF07`/`VF08`/`VF09` references below are **catalog checkers only** and the
M1 IR/CFG invariant checks are a **recorded gap, not a pass**.

| # | Invariant |
|---|---|
| IR-1 | One IR function for `main`; ABI-neutral signature `int (void)`; external linkage; no aggregate or variadic |
| IR-2 | Exactly one basic block, the entry block; exactly one terminator (a return), which is the **greatest-ordered `InstructionId` in the block**; **no instruction appended after the terminator** |
| IR-3 | No branch/loop/switch edges; no dangling, unreachable, or duplicate-terminator blocks |
| IR-4 | **Type predicates (each checkable from committed records).** (a) The `Return` instruction's single operand is a committed `ValueId` whose `ValueRecord.ty` is the canonical symbolic `int` `TypeId` (`TypeKind::Int { rank: Int, signed: true }`, IR-6) — the same `TypeId` as the result type of `FunctionRecord.signature`. (b) Operand/result type consistency is the closed op-table type rule: every operand and result `ValueRecord.ty` in the M1 function is that canonical symbolic `int`, and for `Constant` the immediate `ConstRecord.ty` equals the result `ValueRecord.ty` (`ir.op-immediate-type`). (c) Memory/effect carriers are absent, or present as exactly the empty mask `0` (`EffectMask(0)`; any nonzero mask is `EffectMaskUnsupported`) — "provably empty" means the exact `0` check, never an unchecked assertion. VF08 `IrInvariantChip` is the checker (catalog ID only). |
| IR-5 | The M1 reference fixture lowers to a folded `int` constant `5`; the closed op table may retain an `int` add of `2`/`3` for verifier/future tests, but it is not required by this fixture; identical input must deterministically yield the same form |
| IR-6 | **Type vs value (Part A symbolic model).** Integer **types** carry symbolic rank + signedness (`TypeKind::Int { rank, signed }`; identity = the canonical `TypeId`), with **no** concrete target width/bit-pattern and no `sizeof`/alignment. Integer **values** are mathematical (`ConstRecord { ty, value }`): `value` is the canonical signed representative (unsigned-ness lives only in `ty`), computed with checked arithmetic — **no host overflow/wrap** and no target bit-pattern projection; overflow/unsupported is the chip diagnostic `ConstOverflow`/`ConstUnsupported` inside the `max_const_bits` bound. Concrete width/bit-pattern and layout are Part B and probe-gated (G12). |
| IR-7 | No memory operations, calls, volatile/atomic, floating point, or aggregate copy appear; legal-falloff handling must not synthesize a spurious `0` return for the explicit-return case |
| IR-8 | Function/block/value/instruction IDs are stable, in production order, never address-derived; no tombstones on the happy path |
| IR-9 | **Non-SSA is legal; the verifier must not require or assume SSA form.** M1 IR is checked by the non-SSA invariants only; SSA-only checks (defs dominate uses, phi edge count/type, single definition in SSA form) apply only to IR explicitly in SSA form (T10/VF09), and a non-SSA function must not be reported as an SSA-form violation (T13 VF09 negative fixture). |

M1 op table (**prospective `/6`; not frozen**): `Constant` (0 operands,
`immediate` some, result some, non-terminator); `Add` (2 operands, result some,
non-terminator; optional, used only for verifier/future — the fixture emits the
folded constant); `Return` (1 operand for M1 `main`; result none; the unique
terminator). Each op is **proposed to be** pinned with a `NORMATIVE_RULE` id in
the future `/6` hashed section (`ir.op.constant`/`ir.op.add`/`ir.op.return`); no
rule/op/field is claimed hashed or frozen before that freeze (H8). An unimplemented node whose
lowering is absent is `UnsupportedNode` (structured, never a no-op) and an
unlisted op is `UnsupportedIrOp`; **both are chip diagnostics** carried through
the chip's `Fail`/`DiagnosticDraft` path, not `CommitError`s. A `Block` is
*committed* once created and *terminated* once a terminator is appended; no
instruction may be appended to a terminated `Block`; each `ValueId` has exactly
one producing `Instruction.result`; the terminator is the greatest-ordered
`InstructionId` and nothing follows it; an unterminated entry block is
`TerminatorMissing` — **rev 49 (user, 2026-10-05) selects the operative trigger
direction**: the deterministic function-completion fact is the **committed
terminal result of the reused IR28 `FunctionEnd` task** (`TaskState::Completed(ResultId)`;
task kind `FunctionEnd`), and a **T01-owned typed phase-2b commit-apply check**
then validates **entry-block termination** (the entry block has a terminator as
its greatest `InstructionId`) when that committed result is applied. This adds
**no** new marker record family/ID/arena/`RecordRef` tag/`RecordFamily`
ordinal/snapshot encoder and **no** new `ResultValue` variant. It is a selected
**direction, not a frozen `/6` hook/schema and authorizes no code**: the exact
hook ordering/typing, hash treatment, result typing, and T09/T01 co-freeze remain
**open**, and the **T13 fixture** exercising this completion fact is **pending**.
The **earlier marker-based direction** (an explicit T09 `IR28`
function-completion marker record) is **superseded for the operative direction**
and preserved only as history;
a non-`Constant` op with an `immediate` (or a `Constant` without one) is
`OpImmediateMismatch`, and a mis-typed `immediate` is `OpImmediateTypeMismatch`
(for `Constant`, the `ConstRecord.ty` must equal the instruction's result
`ValueRecord.ty`; rev 21). `ConstRecord` carries a mathematical signed value
(the exact carrier is open; `i128` is an unselected draft, not selected, per CDR
rev 54); checked arithmetic is a **chip-level** obligation (T08; `CL03`/`CL05`;
`CL02` unexercised) with `ConstOverflow`/`ConstUnsupported` as **chip
diagnostics** bounded by `max_const_bits`; the commit does not re-validate the
value.

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

Everything in §4 (concrete target values, ABI register/save-area numbers,
assembler/linker/runner acceptance) is **Part B-only**: it scopes the
target-dependent half of M1 and does **not** gate the Part A symbolic contract or
its fixtures, which are probe-independent (§3).

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
| B8 | The stack pointer is aligned to the probe-confirmed `abi.stack_align` value (fixture proposal `16`, **UNVERIFIED**) at every public interface / call boundary and at return |
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
   `long double` format, `wchar_t` signedness/encoding (the T01 C02 attestation
   requires `wchar_t.encoding`).
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
with a content hash, as required by [T01](T01_COMPILER_CONTRACT.md) §7.1 (C02).
A hashed evidence report is necessary but not sufficient: the H01 normalizer
leaves the four AAPCS64 fields `unresolved` and does not emit
`verified`/`report_hash`, while `attest` requires resolved numeric values plus a
caller-supplied `verified=true` and matching `report_hash`; the T01 integrator
owns that step after H07 classifies the ABI evidence
([T00](T00_GCC_TORTURE_GATE.md) §4.1).

---

## 5. Fail-Closed Rule for Target Code Generation

- While the target is unverified, the configuration must **refuse** code
  generation with a structured target diagnostic. This is the current recorded
  behavior described in [compiler/README.md](../../compiler/README.md) and
  [T01](T01_COMPILER_CONTRACT.md) §7.1; this plan does not re-run it.
- This restriction is **Part B-only**: it scopes target code generation. It does
  **not** block the Part A symbolic contract or its probe-independent fixtures
  (§3), which perform no target code generation.
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
| D5 | Scheduling and queue order | Stable; new enqueues are visible next tick; results are consumed exactly once; selection order is `(stage ordinal, phase priority, enqueue ordinal, TaskId)` |
| D6 | ELF artifact across runs | **Semantic** equality only (identical structural/symbol summaries and identical exit status). Do **not** require byte-identical ELF; build-id or timestamps may legitimately differ. Record this distinction. |
| D7 | Two-trace comparison | First divergence is reported, or none |
| D8 | Pipeline quota determinism | At `max_inflight_per_tick = 1` the result equals the single-active-task design under the **canonical semantic comparison projection** (semantic records/results/diagnostics + semantic trace, excluding scheduler-only registers: stage queues, in-flight set, `dispatch_cursor`, `stage_assignment_version`, `PipelineMetrics`); it is **not** a byte-identical snapshot, and the new scheduler snapshot must be deterministic/replay-identical run-to-run. At any tested quota the dispatch order, proposal order, commit order, and snapshot hash are identical across repeated runs; a same-tick **write** conflict on a `(StoreId, field, record)` key is rejected in the canonical `(dispatch_ordinal, enqueue_ordinal, TaskId, proposal index)` order. No throughput claim. |

These follow the deterministic serialization and replay obligations in
[T01](T01_COMPILER_CONTRACT.md) §7 (C05) and
[T13](T13_VERIFICATION_CHIPS.md).

---

## 7. Structured Failures and Negative Paths

Every failure must be a typed diagnostic; the task must terminate
`Completed`/`Failed` exactly once or explicitly wait, never spin. No partial
artifact may be committed on failure. Error inputs are never counted as passes.
**Rev 25 F2:** at the M1 baseline (`max_inflight_per_tick = 1`) the frozen `/5`
`compiler/src/routing.rs` `fail_selected` **single-task** guarantee applies and is
**verified**: the failed task **always** transitions to `TaskState::Failed`,
attaching a committed `DiagnosticId` when diagnostic/record capacity allows and
otherwise using the `TaskState::Failed(DiagnosticId::NONE)` sentinel, so the task
is **never** stranded or left `Running`. The
**batch** (`quota > 1`) no-`Running`/terminal fan-out was historically recorded as
**BLOCKED** (H6) and is **not** part of M1 acceptance; clearing the in-flight set
does not clear `TaskState`/`Running` (see ADR-0002 §6 and the M1 proposal
§7/§19.2/§20). **Current status (updated 2026-10-05, rev 29):** the H6
**mechanism direction** has been explicitly selected (the bounded dispatch-order
fail transition with the same optional-diagnostic/`DiagnosticId::NONE` sentinel
semantics; CDR rev 42, with the T02/T13 updates), but the mechanism's
**implementation and `/6` schema realization remain blocked** pending the `/6`
freeze; clearing the in-flight set still does not clear `TaskState`/`Running`.
Partial authority decisions were recorded across CDR rev 42–55 (selected
subdecisions and delegated candidate defaults; current CDR rev 55 — revs 52/54
are read-only audits), but the **overall owner bundles
remain pending**; the H6 **mechanism direction** is selected while its
**implementation remains pending**. **H9 (in-flight scheduling ownership):** the
`max_inflight_total` removal direction (CDR rev 35) and the sole-`max_inflight_per_tick`/quota
delegated candidate default (rev 51) are current, but the dispatcher
`Ready→Running`/`in_flight` commit boundary, the clear order, and the all-paths
no-`Running`/no-residual proof remain **open** (T02 H9.1–H9.4; T13 H6-M01..H6-M16);
no no-residual guarantee is claimed. The prior `/6` alternatives (generalize the
deterministic all-dispatched fail transition, or another T01-approved atomic
terminal path) are recorded there. No `/6` freeze, implementation, or acceptance
is claimed here.

| # | Input / condition | Expected structured outcome | Owner |
|---|---|---|---|
| N1 | Codegen requested while the target is unverified | Structured target/config diagnostic; refuse to emit | T01/C02 |
| N2 | Unsupported construct for the M1 subset (for example a floating type or a function call in the return expression) | Explicit `Unsupported`/`UnsupportedNode`/`UnsupportedIrOp` diagnostic; no code emitted, no silent truncation | T07/T09/T11 |
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

Gate scoping: the `Blocking` gates below are **not** all of one kind. **P2/P3**
gate **chip work** (they must clear before chips are implemented or runnable
acceptance is claimed); **P7/P8** gate **candidate evidence collection** — the
Part A §9 commands need the H04-owned candidate CLI (snapshot/trace/interpret),
and Part B execution additionally needs the H05 runner; **P4/P5** gate
**target-dependent (Part B) chip work/execution and acceptance**. None of
**P4/P5, P7, or P8** gates approval or freeze-readiness of the **Part A
symbolic contract**, whose readiness depends on its **own contract decisions and
owner/integrator sign-offs** (P2/P3 are that schema/wiring work), not on the
Linux probe
([PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) Wave 0: the target probes "gate
Part B / target-dependent work only … not a precondition for drafting, reviewing,
or approving the symbolic Part A contract, and the T01 `/6` freeze does not wait
for the Linux probe to run"). The **mixed** rows **P7/P8/P15/P18** state their
Part A/Part B scope in the table below; their Part B portions open only after
**P4/P5**.

| ID | Owner | Deliverable needed for M1 | Current state | Gate |
|---|---|---|---|---|
| P0 | Integrator | Durable arena/target decision | [ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) present (Accepted) | Recorded |
| P1 | Integrator | C01–C06 envelope compiles with a recorded version/hash and freeze test | Recorded `t01-c01-c06/5`; compiler tests and hash recomputation rerun on 2026-10-04 | Recorded |
| P2 | Integrator + group owners | Freeze language-record schemas and per-group task/result variants | Placeholders only | **Blocking for chip work** |
| P3 | Integrator | Registration and motherboard wiring | Routing shell only; no handlers | **Blocking for chip work** |
| P4 | H01 | Linux CI/VM substrate, assembler, linker, sysroot, runner | Unresolved; not provisioned | **Blocking Part B** |
| P5 | H01 (report) / H07 (classification) | Target probe producing a hashed report; H07 classifies the report's `unresolved` ABI fields; the T01 integrator attests | Harness exists but has **never run**; no report/hash; ABI classification fields unresolved | **Blocking Part B**; C02 attestation is T01-integrator-owned |
| P6 | H00 | Corpus lock and license/provenance records | Scaffold only; lock draft | Required for T00, not for the M1 executable |
| P7 | H04 | Candidate driver flags `-E/-S/-c/-o/-I/-D/-U/-std` plus the Part A snapshot/trace/interpret flags (§9) | Part A present (`compiler/src/bin/candidate.rs` + `h04_candidate` 6 tests; `-E`/`-I`/`-D`/`-U`/multi-source and `-S`/`-c` emission still absent) | **Blocking for Part A** (driver/emit flags); `-S`/`-c` target emission **Part B** |
| P8 | H05 | Isolated execution, limits, timeouts, cancellation/crash classification | Absent | **Blocking Part B** for the run stage; Part A checks do not require the isolated target runner |
| P9 | T02 | Job bootstrap, selection, phase advance, commit/diagnostics | Shell only | **Blocking** |
| P10 | T03 | Candidate preprocessing pass (record the no-external-`-E` decision) | Not implemented | Blocking or explicitly bypassed |
| P11 | T04 | Lexer for the fixture | Not implemented | **Blocking** |
| P12 | T05 | Parser for `main`, `(void)`, block, `return` | Not implemented | **Blocking** |
| P13 | T06 | `int`, `void` parameter list, function type, `main` | Not implemented | **Blocking** |
| P14 | T07 | Return/type legality and constant-expression checks | Not implemented | **Blocking** |
| P15 | T08 | Integer constants (Part A) and `int` layout (`M1-CL-02`, probe-gated) | Not implemented | **Blocking Part A** for symbolic constants; `int` layout **Part B** |
| P16 | T09 | IR lower for this fixture and the frozen IR schema | Not implemented | **Blocking** |
| P17 | T11 | AArch64 assembly/ELF emission and target verification | Not implemented | **Blocking Part B** |
| P18 | T13 | IR/CFG/interpreter/replay/outcome verification (Part A) and ABI verification (Part B) | Not implemented | **Blocking Part A** for IR/CFG/replay evidence; ABI verification **Part B** |
| P19 | T12, T10 | Not required for this fixture; absence must be explicit | Not implemented | Non-goal |

Probe → attestation handoff (ownership): the H01 report is evidence, not
verification. H07 owns the reference-oracle/target-capability classification
(the report deliberately leaves `abi.gp_arg_regs`, `abi.fp_arg_regs`,
`abi.stack_align`, and `abi.variadic_register_save_area` as `unresolved`); the
T01 integrator owns the `TargetSpec::attest` step and is the only owner who may
mark the target verified after incorporating a real report hash. `attest`
validates caller-supplied data and hash, not authenticity or physical provenance
(TOCTOU), so no probe has been attested and C02 remains unverified.

Wave ordering (see [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md)): Wave 0 is
P0–P6; P7/P8 (H04/H05) are the evidence/run host tasks (P7 gates Part A
evidence, P8 gates the Part B run); Wave 1 is T02/T03/T04/T06/T08/T13 plus
T11-foundation (target-dependent **Part B**, blocked until the probe); Wave 2
adds T05/T07/T09, VF06 typed-AST verification (M1: after committed T07
`SemRecord`s, before T09 lowering), and T11 instructions; Wave 3 is M1
integration.

---

## 9. Evidence to Collect (planned; NOT RUN)

All commands below are **planned only**. They are not a run record.

Part A (target-independent, after P2/P3; **no target code generation** — the
fail-closed rule in §5 refuses `-S`/assembly emission while the target is
unverified, so Part A produces only snapshots/IR and interpreted results; the
snapshot/trace/interpret flags are candidate-driver flags **owned by H04/P7**,
spelled illustratively until the driver contract freezes):

```text
candidate -std=c11 -O0 --emit-ir-snapshot --emit-trace ...
candidate --interpret-ir main.ir          # expect modeled return 5
candidate ... invalid_type.c              # expect structured diagnostic, no output
```

Part B (Linux substrate, after P4/P5; the first stage that emits target code):

```text
candidate -std=c11 -O0 -S -o main.s main.c
<frozen-assembler> -o main.o main.s
<frozen-link-command> -o a.out main.o     # never passes main.c to any C compiler
<header/symbol/section inspection> main.o
<header/symbol/section inspection> a.out
./a.out ; echo $?                         # expect 5
```

Repeat the Part A chain three times and compare snapshot and trace hashes;
repeat the Part B chain three times and compare the assembly hashes (D3).
Compare ELF output semantically only (D6).

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
- [tools/torture/probe/README.md](../../tools/torture/probe/README.md) — H01 probe harness (present; never run) and its report normalization
- [ADR-0001-COMPILER-DYNAMIC-ARENA.md](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) — approved storage extension and frozen target identity
- [ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) — PROPOSED deterministic staged pipeline / bounded in-flight scheduling
- [M1_PART_A_CONTRACT_PROPOSAL.md](M1_PART_A_CONTRACT_PROPOSAL.md) — M1 Part A record/envelope proposal (DRAFT)
- [SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md) — semantic core and boundaries
- [SILICON_PARADIGM_SPEC.md](../architecture/SILICON_PARADIGM_SPEC.md) — paradigm specification
- [ARCHITECTURAL_BLUEPRINT.md](../design/ARCHITECTURAL_BLUEPRINT.md) — application design guide
- [compiler/README.md](../../compiler/README.md) — foundation package status and limitations
- [compiler/contracts/CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION) — recorded foundation artifact
- [compiler/contracts/target/aarch64-linux-probe.txt](../../compiler/contracts/target/aarch64-linux-probe.txt) — unverified probe fixture

---

## 12. Revision record

| Date | Change | Authority |
|---|---|---|
| 2026-10-04 | IR-5/IR-6 clarified: the M1 fixture lowers to a folded `int 5` (the `Add` form is optional/verifier-only), and Part A integer records are symbolic rank+signedness with concrete width/bit-pattern deferred to Part B/probe (G12). Planned; no implementation, no execution. | M1 integration session |
| 2026-10-04 | Rev 19: A3 records the persisted checked-Return/`FunctionContext`; the op table is hash-pinned per op with `UnsupportedIrOp`/`UnsupportedNode` and committed-vs-terminated Block terms; the folded-`int5` producer is separated from the probe-gated `M1-CL-02` (chip vs fixture id); D5 order and new D8 pipeline-quota determinism added; references include the PROPOSED ADR-0002. Planned; no implementation, no execution. | M1 rev 19 integration |
| 2026-10-04 | Rev 20: `ConstOverflow`/`ConstUnsupported`/`UnsupportedNode`/`UnsupportedIrOp` classified as **chip diagnostics** (not commit errors) with `max_const_bits`; added `TerminatorMissing`/`OpImmediateMismatch`/`OpImmediateTypeMismatch`; D8 restated as the **canonical semantic comparison projection** (not byte-identical) with a separately deterministic scheduler snapshot; `M1-CL-02` is executed by chip `CL03` and chip `CL02` is unexercised; A3 uses the committed `FunctionId`/`FunctionRecord` handoff (no `FunctionContextId`). Planned; no implementation, no execution. | M1 rev 20 re-review integration |
| 2026-10-04 | Rev 21: **A3 corrected** — the T07→T09 handoff uses the committed T07 `SemRecord`s (including `Return`/`FunctionDefinition`), **not** a T09-created `FunctionRecord` (temporal cycle removed); §3.2 fixes the `TerminatorMissing` trigger to the explicit T09 `IR28` function-completion marker and the `Constant` immediate = result-type rule. Aligned with the user's in-principle decisions recorded in the CDR. Planned; no implementation, no execution, no freeze. | M1 rev-21 review-integration subagent |
| 2026-10-04 | Rev 22: A3 softens the `FunctionContextId` statement to a **selected draft direction** and flags the T07-package conflict (task amendment/integrator pending); integrated the accumulated read-only rev-21 findings (the `CompletedFunction` marker family is an explicit blocker; the T09 rejection→rule-id inventory, folded-`int5` ownership, and `Constant` missing-immediate-vs-result precedence are recorded; the sem→const order is preserved). Planned; no implementation, no execution, no freeze; `/5` current; all owners/integrator pending. | M1 rev-22 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | Rev 23: integrated the independent rev-22 audit. **H11** — `TerminatorMissing` is restated as a **marker-based trigger *direction* with the marker family/schema unresolved** (not "trigger fixed"); **H8** — the M1 op table and per-op `NORMATIVE_RULE` ids are stated as **prospective `/6`**, not frozen. Planned; no implementation, no execution, no freeze; `/5` current; all owners/integrator pending. | M1 rev-23 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | Rev 24: recorded cross-doc that the user accepted the **H1 literal-handoff allocation split** in principle (2026-10-04) in the CDR/M1 proposal (working basis, not a freeze). No target-acceptance row is affected (the split is frontend/const-stage, target-independent); H8/H11 target rows are unchanged. Planned; no implementation, no execution, no freeze; `/5` current. | M1 rev-24 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | Rev 25: integrated the rev-24 independent audit **F2** and the verified `/5` failure guarantee — §7 now states that the M1 baseline (`quota = 1`) uses the frozen `/5` single-task `fail_selected` guarantee, while the batch no-`Running`/terminal fan-out is **BLOCKED** (H6) and not part of M1 acceptance. Other audit findings are proposal/CDR/T02/ADR-only for this target-acceptance document; no target row changes. Planned; no implementation, no execution, no freeze; `/5` current; H6 BLOCKED. | M1 rev-25 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | Rev 26: integrated the rev-25 independent audit findings 1–8 plus the nested CDR §C/§D/§E request items at the target-acceptance level (point-by-point ledger in M1 proposal §20). §7 (the M1 baseline `quota = 1` uses the frozen `/5` single-task `fail_selected` sentinel guarantee; the `quota > 1` batch fan-out stays **BLOCKED**/H6) is unchanged, and no target row changes because the audit fixes are schema/proposal/CDR/ADR/T02-level. Planned; no implementation, no execution, no freeze; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending. | M1 rev-26 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-04 | Rev 27: docs-only label correction (no acceptance, status, or target-value change). §7's failure-guarantee paragraph is relabelled from "Rev 26 F2" to **Rev 25 F2**: the H6/sentinel text was integrated at rev 25 and rev 26 did not alter §7 (the rev-25 record already names F2, and the rev-26 record states §7 was unchanged). Planned; no implementation, no execution, no freeze; `/5` current; H6 BLOCKED; H1 accepted in principle only; all owners/integrator pending. | M1 rev-27 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | Rev 28: **clarity-only** edit from read-only audit findings; no acceptance criterion, status, or target-value change. (1) §8 now scopes the gates: P2/P3 block **chip work**, P4/P5 block **Part B** target-dependent acceptance, and none of P2–P5 gates freeze-readiness of the **Part A symbolic contract**, which depends on its own contract decisions/sign-offs, not the Linux probe. (2) §1.3/§4/§5 state that concrete target values and the fail-closed codegen restriction are **Part B-only**; the Part A symbolic contract/fixtures are probe-independent. (3) §1.4 states the `t01-c01-c06/5` version/hash is the **foundation envelope only** (not the M1 language schema, not `/6`), and the frozen target is **identity only** (not validated target values/toolchain). (4) §1.3/§2 state that assembler/linker/runner and the assembly/link/run rows are **planned Part B expectations, not available facilities**. **Same-revision correction (2026-10-05):** this row's date is corrected from 2026-10-04 to 2026-10-05 (the rev-28 edit was performed 2026-10-05); no rev 29 is added because the change is a same-revision date/status correction, not a new revision. Current status (2026-10-05): M1 DRAFT; `/5` current; `/6` unfrozen; Part B unavailable; no implementation or acceptance pass claimed. H6: the **mechanism direction** is explicitly selected in CDR rev 42 (bounded dispatch-order fail transition) with the T02/T13 updates, but **implementation/`/6` schema realization remains blocked** pending `/6`; the historical "H6 BLOCKED" wording above (revs 25–28) describes the pre-rev-42 state. Partial authority responses exist in CDR rev 42–47 (selected subdecisions and delegated candidate defaults), while the **full bundle sign-offs and `/6` remain pending**. H1 accepted in principle only. | M1 rev-28 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | Rev 29: **current-operative reconciliation** with CDR rev 49/50 and the T09 operative amendments; no acceptance criterion is added or relaxed. (1) §3.2 `TerminatorMissing` now states the **current operative direction** — the function-completion fact is the **committed terminal result of the reused IR28 `FunctionEnd` task** (`TaskState::Completed(ResultId)`, task kind `FunctionEnd`), validated by a **T01-owned typed phase-2b commit-apply check** for **entry-block termination** — adding **no** new marker record family/ID/arena/`RecordRef` tag/`RecordFamily` ordinal/snapshot encoder or new `ResultValue` variant; the **earlier marker-based wording is superseded for the operative direction and preserved only as history**. This remains a **selected direction, not a frozen `/6` hook/schema/code authorization**: exact hook ordering/typing/hash treatment and T09/T01 co-freeze stay **open**, and the **T13 fixture is pending**. (2) Header status and (3) §7 current status note that CDR rev 42–53 record **partial** decisions (selected subdecisions and delegated candidate defaults; current CDR rev 54) while the **overall owner bundles remain pending**; H6's **mechanism direction is selected but its implementation is pending**. (4) Status records the rev-49/50 **two-tier `/6` hash boundary accepted conceptually** (the frozen seed `foundation + M1AppendSchema` is hashed; post-seed runtime `declare()` declarations excluded) with **no `/6` hash/seed values or freeze**; `/5` remains current. Current status (2026-10-05): M1 DRAFT; `/5` current; `/6` unfrozen; Part B unavailable; **target acceptance not passed**; no implementation claimed. The rev-28 row above (including its same-revision date correction) is preserved verbatim as history. | M1 rev-29 review-integration subagent (DeepSeek doc integrator) |
| 2026-10-05 | Rev 30: **post-rev-39 audit integration** (docs-only; no acceptance change, no implementation, no freeze). (1) `int` layout clause qualified to Part B; §7 P2–P5 gate scoping corrected (Part A vs Part B); H9 sentence added; header/§7 pointer to CDR rev 42–53 / current rev 54; rev-29 row pointer corrected to 42–53. (2) `ConstRecord` carrier `i128` marked unselected draft; `FunctionContextId` clause updates to the accepted subdecision; T00 `§1.1`→§1 item 1; T01 `§6`→§7.1 (C02) citation; T11 wave/Part-B annotation cross-referenced. Planned; no implementation, no execution, no freeze; `/5` current, `/6` unfrozen, M1 DRAFT. | Post-rev-39 read-only audit; docs integration |
| 2026-10-05 | Rev 31: **DOC-02 fix** (docs-only; no acceptance change, no implementation, no freeze). §9 Part A no longer invokes `-S`/emits `main.s`: it runs only snapshot/trace emission and IR interpretation, consistent with the §5 fail-closed rule for target code generation. `candidate -std=c11 -O0 -S -o main.s main.c` and the assembly-hash comparison move to Part B (after P4/P5), ahead of assembling and linking. | Documentation review 2026-10-05 (DOC-02) |
| 2026-10-05 | Rev 32: **IR-1..IR-9 precision/testability audit** (docs-only; no acceptance change, no implementation, no freeze). IR-4 restated as committed-record-checkable predicates: the `Return` operand's `ValueRecord.ty` is the canonical symbolic `int` `TypeId` (the result type of `FunctionRecord.signature`); operand/result consistency is the op-table type rule including the `Constant` immediate check; effects are absent or exactly `EffectMask(0)` (VF08 is the checker). IR-6 separates symbolic integer **types** (rank + signedness, no width/bit-pattern) from mathematical integer **values** (`ConstRecord`, canonical signed representative, checked arithmetic with no host overflow; concrete width/bit-pattern Part B/probe-gated, G12). IR-9 reworded: non-SSA is legal and the SSA-only check is conditional on SSA form (T10/VF09), so a non-SSA function must not be flagged as an SSA violation. IR-1/2/3/5/7/8 audited against the symbolic model and the `FunctionEnd` completion-fact hook and needed no change; in particular IR-2's greatest-`InstructionId` terminator predicate matches the T01-owned phase-2b entry-termination check, and IR-7's explicit-return/no-spurious-`0` rule matches IR28 legal-falloff handling. | IR precision/testability audit (2026-10-05) |
| 2026-10-05 | Rev 33: **H00–H10 ownership/gate audit fixes** (docs-only; no acceptance change, no implementation, no freeze). P5 ownership split into **H01 (probe run/report)** and **H07 (reference-oracle/ABI classification of the report's `unresolved` fields)** with the `TargetSpec::attest` step remaining T01-integrator-owned; added the probe → attestation handoff note (`attest` validates caller-supplied data/hash, not authenticity/provenance; no report exists, so no attestation has occurred); §11 adds the H01 probe harness reference. The P7/P8 mixed-row scoping and the §9 H04-flag note were already recorded by the concurrent gate-scope edit and are preserved. | H00–H10 audit (doc-only) |
| 2026-10-05 | Rev 34: **Part A/Part B scoping sync** (docs-only; no acceptance change, no implementation, no freeze). (1) §8 scopes the mixed rows **P7/P8/P15/P18**: P7 candidate driver/emit flags are **Part A** (`-S`/`-c` target emission **Part B**), P8 isolated execution is **Part B** for the run stage (Part A checks need no isolated runner), P15 symbolic constants are **Part A** while `int` layout is probe-gated **Part B** (`M1-CL-02`), and P18 IR/CFG/interpreter/replay/outcome evidence is **Part A** while ABI verification is **Part B**; the gate-scoping paragraph states the mixed rows and that their Part B portions open only after **P4/P5**. (2) §4.3 B8 is checked against the probe-confirmed `abi.stack_align` value (fixture proposal `16`, **UNVERIFIED**) instead of an assumed 16-byte alignment. (3) §1.3/§4.5 add the required `wchar_t` **encoding** probe field (`wchar_t.encoding`; T01 C02 attestation). (4) §9 records the snapshot/trace/interpret flags as candidate-driver flags **owned by H04/P7** (the P7 row carries them); the rev-31 DOC-02 placement keeps the `-S` invocation only in the Part B command list. (5) Revision record: the Rev 29 row now precedes the Rev 30 row and the duplicate table header is removed. Current status: M1 DRAFT; `/5` current; `/6` unfrozen; Part B unavailable; no implementation or acceptance pass claimed. | M1 target Part A/Part B scoping sync (2026-10-05) |
| 2026-10-05 | Rev 35: **Part A/Part B split wording** (docs-only; no acceptance change, no implementation, no freeze). (1) §1.3 no longer classifies every listed item as a Part B scoping fact and no longer claims that none of them gates Part A check execution: the Part B items do not gate the Part A symbolic contract or its probe-independent fixtures, while Part A check execution is separately gated by the language-record schema freeze and the front-end/IR chip implementations (§3, §8). (2) The §8 wave summary annotates T11-foundation as target-dependent **Part B**, blocked until the probe, matching [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) Wave 1. The rev-34 gate-scope fixes, the rev-31 DOC-02 `-S` placement, the probe-confirmed B8 check, and the §9 H04/P7 flag attribution are preserved. Current status: M1 DRAFT; `/5` current; `/6` unfrozen; Part B unavailable; no implementation or acceptance pass claimed. | Part A/Part B split audit (2026-10-05) |
| 2026-10-05 | Rev 36: **VF06/H04–H05 wave-placement sync and `/6` gate-scoping correction** (docs-only; no acceptance change, no implementation, no freeze). §8 wave ordering now places **P7/P8 (H04/H05)** as the evidence/run host tasks (P7 gates Part A evidence; P8 gates the Part B run) and adds **VF06 typed-AST verification** to Wave 2 at its fixed M1 position (after committed T07 `SemRecord`s, before T09 lowering), matching [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md). §8 scoping corrected so only the probe/substrate and Part B host gates (**P4/P5, P7, P8**) are stated not to gate the Part A symbolic contract's approval/freeze-readiness; **P2/P3 are that schema/wiring freeze work itself**, so the earlier "none of P2–P5" phrasing was over-broad. No gate value or acceptance criterion changes. | VF06/H04–H05/`/6` docs audit |
| 2026-10-05 | Rev 37: **chip-ID audit follow-up** (docs-only; no acceptance change, no implementation, no freeze). §3.2 preamble now records that `VF07`–`VF11` are **out of the M1 T13 chip set** (T13 rev 8), so the `VF08`/`VF09` references in IR-4/IR-9 are **catalog checkers only** and the M1 IR/CFG invariant checks are a **recorded gap, not a pass** (mirrored in M1 vertical rev 34 §8). | Chip-ID audit (docs-only); T13 rev 8 |
| 2026-10-06 | Rev 38: **H04 Part A driver present** (host tooling; no acceptance change, no freeze, no contract bump). The P7 status cell now records the Part A candidate driver (`compiler/src/bin/candidate.rs`, evidence `compiler/tests/h04_candidate.rs` 6 tests, doc [H04_CANDIDATE_DRIVER.md](H04_CANDIDATE_DRIVER.md)): source-bytes M1 pipeline with snapshot/trace/interpret evidence, `-S`/`-c` fail-closed, invalid input diagnosed with exit 1. `-E`/`-I`/`-D`/`-U`/multi-source and target emission remain absent (Part B). §9 commands stay planned-illustrative until the H04 driver contract freezes. | H04 Part A implementation |
