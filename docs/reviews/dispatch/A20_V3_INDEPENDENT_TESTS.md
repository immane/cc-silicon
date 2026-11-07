# A20 — V3 Independent Acceptance Suite: Functions, Calls, Returns, Globals, Pointers, Arrays, and the ABI Boundary

| Field | Value |
|---|---|
| Task | A-TST-03 (Stage A, V3 independent acceptance design) |
| Status | **DESIGN ONLY — NOT EXECUTED** |
| Date | 2026-11-07 |
| Contract baseline | `t01-c01-c06/39` (`compiler/contracts/CONTRACT_VERSION`), hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` |
| Target baseline | Identity frozen: `aarch64-unknown-linux-gnu`, ELF, LP64, little-endian, AAPCS64; **all concrete values UNVERIFIED**; probe substrate not provisioned |
| Deliverable | This design report only; no test, chip, contract, Cargo, `mod.rs`, index, or source file is changed |
| Scope source | [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) §6.2 V3: "Function parameters/calls/returns, global variables, pointers, and arrays — joint type/semantic/memory/ABI acceptance; do not substitute the Rust host ABI for the target" |

**Status honesty.** This document designs a suite; it does not claim any V3 capability. No V3 test exists, no V3 chip is implemented, no probe has run, and no target execution has occurred. The existing test files inspected for this design were read, not executed. No build or test command was run for this report. The dispatch ledger's baseline (654 compiler tests across 44 suites) is recorded in [DISPATCH_LEDGER.md](../../tasks/DISPATCH_LEDGER.md) from the dispatch session; it is not re-run or re-claimed here.

---

## 1. Purpose and non-goals

### 1.1 Purpose

Design an **independent acceptance suite** for the V3 vertical train:

- function definitions, prototypes, parameters, calls (direct and indirect), and returns;
- global variables, static storage duration, tentative definitions, linkage, and constant initialization;
- pointers, pointer arithmetic, qualifiers, pointers to functions and arrays;
- fixed and incomplete arrays, parameter adjustment, subscripting, decay, and lvalue conversion;
- argument and return conversions that cross the function boundary;
- aliasing and effect facts at the semantic/IR level;
- the target ABI boundary, including mixed-link tests against an independent same-target reference compiler.

The suite must be usable both as:

1. a **Part A** frontend/IR acceptance suite (target-independent; symbolic types and records); and
2. a **Part B** target acceptance suite (probe-gated; assembly, object, link, execution, and ABI).

### 1.2 Non-goals

This report does **not**:

- implement chips, tests, fixtures, schemas, task kinds, routing, or Cargo changes;
- freeze any V3 record, type, task, diagnostic, or ABI value;
- run the probe, GCC, the candidate driver, or the target toolchain;
- provide a GCC torture pass rate or a formal-correctness claim;
- substitute macOS arm64, the Rust host ABI, or the Rust `usize`/pointer model for the frozen target model;
- authorize a chip owner to invent a private V3 interface.

### 1.3 Why the suite is not executable today

Static inspection of the checked baseline shows that V3 cannot be represented yet:

- `NodeKind` is the eight-variant M1 set (`TranslationUnit`, `FunctionDefinition`, `Specifiers`, `Declarator`, `Compound`, `Return`, `BinaryAdd`, `IntLiteral`); there is no parameter list, call, subscript, address/deref, global declaration, pointer declarator, or array declarator node (`compiler/src/bus.rs:545-565`).
- `TypeKind` has only `Void`, `Bool`, `Char`, `Int`, and `Function`; there is no pointer, array, aggregate, or qualified type record (`compiler/src/bus.rs:298-324`).
- `IrOp` has only `Constant` and `Return`; there is no address, load, store, call, pointer arithmetic, or argument/return move (`compiler/src/bus.rs:442-452`).
- `layouts` and `inits` are reserved, untyped arenas (`compiler/src/bus.rs`, `Arenas`); there is no global-initialization or array-layout record.
- The implemented parse/semantic/IR slices are M1-shaped; the target directory/chips (`chips/target/aarch64/`) do not exist.
- The H04 candidate driver drives only the M1 pipeline and refuses `-S`/`-c` while the target is unverified (`compiler/src/bin/candidate.rs:1-18`).
- The existing acceptance files `c12`–`c19` and `c34`–`c39` pin M1 and Wave-3 slices; none exercises V3. They are useful style/pattern references, not V3 evidence.

These are static inspection facts at the checked `/39` baseline. The workspace also contains uncommitted parallel work; it is not used as V3 evidence and must not be overwritten. Therefore every V3 case in this report is **DESIGN / BLOCKED**, not PASS.

---

## 2. Authority, inputs, and citation map

Precedence used: explicit user instruction, accepted ADR, accepted engineering contract, architecture design, existing implementation (`AGENTS.md` §2). The V3 scope is planning text, not implementation evidence.

| Topic | Source |
|---|---|
| V3 definition and joint acceptance | `docs/tasks/MASS_PARALLEL_DISPATCH.md:212` |
| Stage A task numbers / A-TST-03 | `docs/tasks/MASS_PARALLEL_DISPATCH.md:140`, `docs/tasks/DISPATCH_LEDGER.md:62` |
| Acceptance levels (independent/integration/closed-loop) | `docs/tasks/MASS_PARALLEL_DISPATCH.md:194-204` |
| Required normal/boundary/invalid/replay/undeclared-write coverage | `docs/tasks/MASS_PARALLEL_DISPATCH.md:377-381`; `docs/tasks/README.md:105-114` |
| Test-oracle independence | `docs/tasks/PARALLEL_EXECUTION.md:85`; `COMPILER_DEVELOPMENT_GUARDRAILS.md` §7 |
| Target identity and probe gate | `docs/tasks/T01_COMPILER_CONTRACT.md:121-128`; `docs/tasks/T11_TARGET_CHIPS.md:5-7` |
| ABI mandatory bidirectional mixed-link tests | `docs/tasks/T11_TARGET_CHIPS.md:53-55` |
| T05 parser rows for declarators, calls, subscripts, globals | `docs/tasks/T05_PARSE_CHIPS.md:11-19,24-31,38-42` |
| T06 pointer/array/type/conversion rows | `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md:5,89-107,109-111` |
| T07 call/assignment/decay/lvalue/return/effect rows | `docs/tasks/T07_SEMANTIC_CHIPS.md:13,16-33,41,56-60` |
| T08 layout/global initialization/address-constant rows | `docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md:5,61-82` |
| T09 memory/call/return/CFG rows | `docs/tasks/T09_IR_LOWER_CHIPS.md:9-27,43-49` |
| T13 verification/interpreter/replay rows | `docs/tasks/T13_VERIFICATION_CHIPS.md:70-85` |
| Part A / Part B split and evidence commands | `docs/tasks/M1_TARGET_ACCEPTANCE.md:408-418,498-521` |
| Current record/type/IR shapes | `compiler/src/bus.rs:290-324,442-452,504-514,545-565` |
| Existing test style and helpers | `compiler/tests/c12_parse.rs` through `c19_vf01.rs`; `c34_parse.rs` through `c39_sefunc.rs` |
| H04 Part A driver limits | `docs/tasks/H04_CANDIDATE_DRIVER.md`; `compiler/src/bin/candidate.rs:1-18` |
| Contract-change gate | `COMPILER_DEVELOPMENT_GUARDRAILS.md` §3 |
| Commands required for future work orders | `docs/tasks/MASS_PARALLEL_DISPATCH.md:383-399` |

No command from the "future commands" section was executed for this report.

---

## 3. V3 boundary

### 3.1 In scope

| Area | V3 scope |
|---|---|
| Functions | Prototype and definition; zero and N parameters; `(void)`; parameter names; `return` with and without expression where legal; direct calls; calls through function pointers; recursion (self and mutual); forward declarations; function/array pointer declarators. |
| Globals | File-scope declarations and definitions; `extern`; `static` internal linkage; tentative definitions; constant initializers; zero initialization; address constants; global arrays; global function-pointer tables. |
| Pointers | Address-of, dereference, null pointer constant, `void *` round trip, pointer qualifiers, pointer arithmetic within an array, one-past-end, pointer subtraction, pointers to functions and arrays, pointer-to-pointer output parameters. |
| Arrays | Fixed bound, constant-expression bound, incomplete array completed by initializer, string initialization, multidimensional arrays, subscripting, parameter adjustment, decay, array/function decay exceptions. |
| Conversions | Lvalue conversion; array-to-pointer and function-to-pointer decay; assignment conversion for pointer qualifiers and null/`void *`; argument conversion to prototype parameters; return conversion; `_Bool` conversion; narrow-integer argument extension at the ABI boundary. |
| Effects | Store effects through pointers/globals; call effects; conservative aliasing at `-O0`; alias-sensitive results when two pointer parameters may alias. |
| ABI | Scalar and pointer parameters/returns; GP-register and stack argument boundaries; callee-saved preservation; global data symbols; function-pointer interop; `main` entry; ELF/AArch64 identity; mixed-link in both directions. |

### 3.2 Deferred / out of V3

The following are C11 or GNU features but are **not** V3 acceptance scope; they must fail as explicit `Unsupported` in V3 rather than silently pass, and their later acceptance belongs to Stage D/T12:

- structures, unions, bitfields, flexible array members, anonymous members;
- floating-point parameters/returns/arithmetic, `long double`, `__int128`;
- variadic functions and `va_list`;
- variable-length arrays (VLA) and `alloca`;
- `_Atomic`, volatile access semantics, fences;
- `_Thread_local`/TLS, weak symbols, symbol visibility beyond default external/internal;
- inline assembly, nested functions, computed `goto`, `typeof`, vector/complex extensions;
- `setjmp`/`longjmp` and non-local control transfer;
- optimization passes; V3 correctness is `-O0` only.

**Open boundary question Q1.** T11's mandatory ABI tests name small/large structs, HFA, variadic, `long double`, and `__int128` (`T11_TARGET_CHIPS.md:53-55`). Those are not in the V3 language scope defined by the dispatch plan, but they are part of the full AAPCS64 boundary. This report recommends: V3 covers **scalar/pointer/array-pointer ABI only**, and aggregate/HFA/variadic/FP/`long double`/`__int128` mixed-link moves to Stage D. The integrator must confirm before V3 Part B freezes.

### 3.3 Part A / Part B split

- **Part A**: target-independent. Source → PP/LX/PA/TY/SE/CL/IR records, scopes/symbols/types, conversions, effects, initialization plans, IR shape, diagnostics, replay, protocol/manifest checks. No concrete widths, sizes, alignments, offsets, addresses, registers, relocations, object format, link, or run.
- **Part B**: probe-gated. Concrete target layout, ABI classification, argument/return locations, stack frames, assembly, external assembler/linker, ELF object, mixed-link, execution, exit codes, and target validation.
- **Part A is not a substitute for Part B**, and an IR interpreter result is not target execution. Conversely, Part B does not excuse missing Part A semantic facts: a target run that happens to print the expected exit code is not acceptance if the frontend facts/diagnostics are wrong.

The probe blocks only concrete target assertions, not drafting/reviewing/approving the symbolic Part A contract (`PARALLEL_EXECUTION.md:5-9`; `M1_TARGET_ACCEPTANCE.md:408-418`).

---

## 4. Test architecture and fixture ownership

### 4.1 Acceptance levels

| Level | Meaning | Typical mechanism |
|---|---|---|
| `L0` | Pure chip core | `compute(&input)` / pure helper with synthetic typed records; no Bus. |
| `L1` | Chip + Bus tick | `bootstrap_task` → `RoutingShell::clock_tick_with` / `drive_task`; real commit, manifest, stage/layer. |
| `L2` | Producer→consumer chain | Real PP/LX/PA/TY/SE/CL/IR tasks from source; symbolic Part A facts. |
| `L3` | Part B single-TU closed loop | Candidate `-S`/`-c` → external assembler/linker → target runner; exit/result checks. |
| `L4` | Part B mixed-link/ABI | Candidate object + independent reference object; two-way calls, globals, pointers. |
| `G` | Gate/meta | Kind/stage/registry/allowlist/manifest/replay/negative-protocol assertions. |

A case may be `L2` for structure and `L3`/`L4` for its value. The matrix uses `L2→L3` and `L3/L4` accordingly.

### 4.2 Oracle discipline (non-negotiable)

Every expected value must have one of these bases:

1. **C11 rule** — cite the clause; no implementation-derived expected value.
2. **Hand calculation** — show the arithmetic or expansion in the test comment, e.g. `1+2+3+4=10`.
3. **Same-target reference evidence** — only for well-defined programs, only for Part B, only from the pinned H01/H07 target environment; the reference compiler must never compile the candidate program itself.
4. **Frozen target probe** — concrete widths/alignments/ABI fields after `verified=true` and C02 attestation; before that, `PROBE` rows are expectations, not verified values.
5. **Frozen contract** — task/record/diagnostic identity; the test must not accept "any failure".

Prohibited oracle sources:

- running the implementation under test to obtain the expected value;
- regenerating golden snapshots/IR from the candidate;
- reading implementation constants to fill the case table;
- using macOS arm64 or Rust host `sizeof`/pointer width as a substitute;
- treating an `Unsupported` result as a pass for a V3-required feature;
- asserting output for undefined or unspecified behavior from one reference compiler run.

Each case table entry carries `Basis` and `Oracle` columns so reviewers can reject self-verifying tests.

### 4.3 Fixture ownership

| Artifact | Owner | Rule |
|---|---|---|
| `compiler/tests/v3_*.rs` | Independent tester | Only the assigned tester writes; expected tables are independent of implementation. |
| `compiler/tests/fixtures/v3/**` | Independent tester | Static `.c` and metadata sources; no runtime generation from candidate output. |
| `compiler/tests/v3_common/mod.rs` | Independent tester + integrator review | Shared harness only; no language semantics, no golden generation. |
| `compiler/src/chips/**` | Chip owners | One writer per chip; V3 tests do not edit chip logic. |
| Contract, task/result enums, schema, IDs, routing, allowlists, `mod.rs`, Cargo | Integrator | A V3 schema change is a `CONTRACT_CHANGE_REQUEST`; no chip-local private version. |
| Probe report, toolchain, runner, reference GCC pin | H01/H07/T01 integrator | Evidence provenance; not a test-writer task. |
| Test entry/discovery and CI registration | Integrator | Cargo auto-discovers top-level `tests/*.rs`, but CI inclusion and any feature gate are integrator-owned. |

**Fixture provenance.** Each fixture `.c` file should carry a header comment with its case ID, C11 basis, expected result, oracle type, and target gate; the Rust case table is the executable copy of the expected value. No fixture or expected value may be generated by the candidate, and `target/v3-reference/**` artifacts are regenerated by the pinned reference toolchain rather than committed. V3 fixtures are authored for this suite; no GCC torture source is copied into them.

**Uncommitted-work note.** The workspace currently has a modified `compiler/src/chips/parse/mod.rs` and untracked lex/parse candidate files. V3 work must treat them as user work, not overwrite them, and must not use them as V3 evidence (`MASS_PARALLEL_DISPATCH.md:64-69`).

### 4.4 Proposed test layout (does not exist yet)

```text
compiler/tests/
  v3_common/mod.rs              # shared harness; no semantics
  v3_calls.rs                   # V3-FN-*, V3-AL-* call/return/effect cases
  v3_globals.rs                 # V3-GL-* global/static/tentative/init cases
  v3_pointers_arrays.rs         # V3-PT-* pointer/array/decay/lvalue cases
  v3_negative.rs                # V3-NEG-*, V3-UNS-* diagnostics/recovery
  v3_protocol.rs                # V3-PRO-* task/manifest/write-scope
  v3_replay.rs                  # V3-RPL-* snapshot/trace/replay
  v3_abi_exec.rs                # V3-ABI-*; #[ignore] probe-gated Part B
  v3_mixed_link.rs              # V3-MIX-*; #[ignore] probe-gated Part B
  fixtures/v3/
    calls/  globals/  pointers/  arrays/  effects/
    negative/  unsupported/  abi/  mixed/
    README.md                   # fixture provenance/oracle notes
```

No `Cargo.toml`, `mod.rs`, or shared index change is required for new top-level integration test files; cargo auto-discovers them. A future V3 **contract freeze** must land before these files reference `v3_slice()`-style APIs; otherwise the test crate will not compile. Do not commit compiling references to unfrozen V3 kinds.

### 4.5 Harness shape

A minimal, dependency-free table harness is sufficient:

```rust
enum Expect {
    Exit(i32),                                  // Part B
    Diagnostic { group: DiagGroup, code: u16 }, // frozen identity
    Unsupported { feature: &'static str },      // explicitly out of V3
}

struct Case {
    id: &'static str,
    source: &'static str,   // fixtures/v3/... path
    expect: Expect,
    level: Level,
    gate: Gate,             // PartA / ProbePartB / PartAAndPartB
    basis: &'static str,    // C11 clause / HAND / REF / PROBE / CONTRACT
    oracle: &'static str,
}
```

Rules for the harness:

- One `#[test]` per family, iterating its case table; the panic message includes the case ID. An optional `V3_CASE=<case-id>` environment filter isolates one case without changing the default all-cases run. Part B gets one `#[test]` per executable scenario so subprocess failure is localized.
- Expected values are literals in the test source; they are never computed by calling the implementation.
- `Diagnostic` requires a frozen group/code/rule identity. If the V3 contract has not frozen the code, the case is `BLOCKED_CONTRACT`, not "any failure passes".
- `Unsupported` is legal only for a feature explicitly outside V3; a V3-required feature returning `Unsupported` is `FAIL`.
- Part B tests use `#[ignore = "probe-gated: requires H01 verified report"]`; `cargo test` reports them ignored, never passed. Running with `--ignored` without a verified probe must fail closed, not silently skip.
- Part A candidate evidence uses the H04 driver only after V3 integration; its flags are H04-owned and illustrative until the H04 contract freezes.

---

## 5. Case matrix

Legend:

- **Gate**: `A` = Part A only; `B` = probe-gated Part B only; `A/B` = structural Part A plus value/ABI Part B.
- **Oracle**: `C11` = clause citation; `HAND` = hand calculation shown; `CONTRACT` = frozen task/diagnostic/record rule; `PROBE` = frozen target probe; `EQ` = equality/determinism; `REF` = same-target reference counterpart/evidence for a well-defined program.
- Every row is **DESIGN, NOT RUN**. `B` rows are blocked by the probe in addition to implementation.
- Matrix size: **179 cases** (FN 22, GL 24, PT 31, AL 10, NEG 22, UNS 14, PRO 16, RPL 10, ABI 20, MIX 10).

### 5.1 Functions, parameters, calls, returns, recursion (`V3-FN-*`)

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-FN-01 | `calls/add.c`: `int add(int a,int b){return a+b;} int main(void){return add(2,3);}` | exit 5 | C11 6.5.2.2p2-4, 6.9.1; HAND `2+3=5` | C11+HAND | L2→L3, A/B |
| V3-FN-02 | `calls/void_params.c`: `int seven(void){return 7;} int main(void){return seven();}` | exit 7 | C11 6.5.2.2, 6.9.1p10 (`(void)` means no parameters) | C11+HAND | L2→L3, A/B |
| V3-FN-03 | `calls/identity.c`: `int id(int x){return x;} int main(void){return id(7);}` | exit 7 | C11 6.5.2.2p4 (argument converted as if by assignment) | C11+HAND | L2→L3, A/B |
| V3-FN-04 | `calls/nested.c`: `add(add(1,2),3)` | exit 6 | C11 6.5.2.2; HAND `(1+2)+3=6`; evaluation order unspecified but no side effects | C11+HAND | L2→L3, A/B |
| V3-FN-05 | `calls/fnptr_local.c`: `int (*p)(int)=inc; return p(5);` with `inc` adding 1 | exit 6 | C11 6.3.2.1p4, 6.5.2.2p1 | C11+HAND | L2→L3, A/B |
| V3-FN-06 | `calls/fnptr_param.c`: `int apply(int (*f)(int),int x){return f(x);} apply(inc,5)` | exit 6 | C11 6.5.2.2p1, 6.7.6.3p8 (function parameter adjusted to pointer) | C11+HAND | L2→L3, A/B |
| V3-FN-07 | `calls/fnptr_global.c`: global `int (*g)(int)` assigned `inc`, then `dec`; call both | exit 2 | C11 6.5.2.2, 6.7.9 (address constant) | C11+HAND | L2→L3, A/B |
| V3-FN-08 | `calls/return_ptr.c`: `int *idp(int *p){return p;} int x=3; return *idp(&x);` | exit 3 | C11 6.5.2.2p1, 6.5.3.2 | C11+HAND | L2→L3, A/B |
| V3-FN-09 | `calls/output_param.c`: `void set(int *p){*p=7;} int main(void){int x=0; set(&x); return x;}` | exit 7 | C11 6.5.2.2, 6.8.6.4 (void return) | C11+HAND | L2→L3, A/B |
| V3-FN-10 | `calls/recursion_sum.c`: `int sum(int n){if(n==0)return 0;return n+sum(n-1);}` main returns `sum(5)` | exit 15 | C11 6.5.2.2, 6.8.6.4; HAND `5+4+3+2+1=15` | C11+HAND | L2→L3, A/B |
| V3-FN-11 | `calls/mutual_recursion.c`: `is_even`/`is_odd` with forward prototype; main returns `is_odd(7)` | exit 1 | C11 6.5.2.2, 6.9.1; HAND trace 7→6→5→4→3→2→1→0 | C11+HAND | L2→L3, A/B |
| V3-FN-12 | `calls/forward_decl.c`: prototype `int add(int,int);` before definition; call after | exit 5 | C11 6.9.1, 6.2.7 (compatible declarations) | C11+HAND | L2→L3, A/B |
| V3-FN-13 | `calls/eight_args.c`: `sum8(1,...,8)` | exit 36 | C11 6.5.2.2; HAND sum 1..8; ABI boundary 8 GP registers | HAND+PROBE | L3, B |
| V3-FN-14 | `calls/nine_args.c`: `sum9(1,...,9)` | exit 45 | C11 6.5.2.2; HAND sum 1..9; 9th argument on stack | HAND+PROBE | L3, B |
| V3-FN-15 | `calls/bool_return.c`: `_Bool f(void){return 2;} main returns f()` | exit 1 | C11 6.3.1.2p1 (scalar to `_Bool`), 6.8.6.4p3 | C11+HAND | L2→L3, A/B |
| V3-FN-16 | `calls/long_return.c`: `long f(void){return 5;} main returns (int)f()` | exit 5 | C11 6.3.1.3, 6.8.6.4; LP64 `long` width is target-dependent | C11+PROBE | L2→L3, A/B |
| V3-FN-17 | `calls/main_falloff.c`: `int main(void){}` | exit 0 | C11 5.1.2.2.3 (reaching `}` returns 0) | C11+HAND | L3, B |
| V3-FN-18 | `calls/main_argv.c`: `int main(int argc,char **argv){return (argc==3 && argv[3]==0) ? 0 : 1;}` run with two args | exit 0 | C11 5.1.2.2.1p2 (`argv[argc]` null) | C11+HAND | L3, B |
| V3-FN-19 | `calls/designator_forms.c`: call `f()`, `(&f)()`, `(*f)()` and sum | exit 21 for `7+7+7` | C11 6.3.2.1p4, 6.5.2.2, 6.5.3.2 | C11+HAND | L2→L3, A/B |
| V3-FN-20 | `calls/recursive_ptr.c`: `int sum(int *p,int n)` recursively walking `p+1`; main returns `sum(a,4)` with `{1,2,3,4}` | exit 10 | C11 6.5.2.2, 6.5.6, 6.5.2.1; HAND 1+2+3+4 | C11+HAND | L2→L3, A/B |
| V3-FN-21 | `calls/no_prototype.c`: `int f(); int f(int x){return x;} int main(void){char c=65;return f(c);}` (call sees no prototype; `char` is promoted and the definition has an `int` parameter) | exit 65 | C11 6.5.2.2p6 (default argument promotions), 6.7.6.3 (no-prototype function type) | C11+HAND | L2→L3, A/B (mode policy Q2) |
| V3-FN-22 | `calls/tail_recursion.c`: bounded tail recursion with depth 16 and base 0 | exit 16 | C11 6.5.2.2; HAND; no tail-call optimization requirement | C11+HAND | L3, B |

### 5.2 Globals, static storage, linkage, initialization (`V3-GL-*`)

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-GL-01 | `globals/init_int.c`: `int g=7; int main(void){return g;}` | exit 7 | C11 6.7.9p4 (constant initializer), 6.9.2 | C11+HAND | L2→L3, A/B |
| V3-GL-02 | `globals/zero.c`: `int g; int main(void){return g;}` | exit 0 | C11 6.7.9p10 (static objects zero-initialized) | C11+HAND | L2→L3, A/B |
| V3-GL-03 | `globals/store.c`: `int g; int main(void){g=5;return g;}` | exit 5 | C11 6.5.16.1, 6.9.2 | C11+HAND | L2→L3, A/B |
| V3-GL-04 | `globals/static.c`: `static int s=4; int main(void){return s;}` | exit 4 | C11 6.2.2p3 (internal linkage) | C11+HAND | L2→L3, A/B |
| V3-GL-05 | `globals/tentative.c`: `int t; int t=9; main returns t` | exit 9 | C11 6.9.2p2 (tentative definition merging) | C11+HAND | L2→L3, A/B |
| V3-GL-06 | `globals/array_init.c`: `int a[3]={1,2,3}; main returns a[1]` | exit 2 | C11 6.7.9 (array initialization in element order) | C11+HAND | L2→L3, A/B |
| V3-GL-07 | `globals/partial_init.c`: `int a[3]={1}; main returns a[2]` | exit 0 | C11 6.7.9p21 (remaining subobjects zero) | C11+HAND | L2→L3, A/B |
| V3-GL-08 | `globals/string_init.c`: `char s[4]="abc"; char t[3]="abc"; main returns (s[2]=='c') && (s[3]==0) && (t[2]=='c')` | exit 1 (exact-fit array has no terminating null; do not read `t[3]`) | C11 6.7.9p14 (string literal initialization; terminator only if there is room) | C11+HAND | L2→L3, A/B |
| V3-GL-09 | `globals/ptr_to_global.c`: `int x=4; int *p=&x; main returns *p` | exit 4 | C11 6.6p7 (address constant), 6.7.9p4 | C11+HAND | L2→L3, A/B |
| V3-GL-10 | `globals/ptr_offset.c`: `int a[3]={1,2,3}; int *p=&a[1]; main returns *p` | exit 2 | C11 6.6p9 (address constant via `&`), 6.5.3.2 | C11+HAND | L2→L3, A/B |
| V3-GL-11 | `globals/string_ptr.c`: `char *s="abc"; main returns s[1]=='b'` | exit 1 | C11 6.7.9p4, 6.3.2.1p3; string object/rodata relocation is target-dependent | C11+PROBE | L3, B |
| V3-GL-12 | `globals/fnptr_table.c`: `int (*t[2])(int)={inc,dec}; main returns t[0](5)` | exit 6 | C11 6.6p9 (function address constant), 6.5.2.2 | C11+HAND | L3, B |
| V3-GL-13 | `globals/extern_def.c`: `extern int e; int e=9; main returns e` | exit 9 | C11 6.2.2p4, 6.9.2 | C11+HAND | L2→L3, A/B |
| V3-GL-14 | `globals/const.c`: `const int c=3; main returns c` | exit 3 | C11 6.7.9p4; `const` does not imply read-only storage | C11+HAND | L2→L3, A/B |
| V3-GL-15 | `globals/ptr_qualifiers.c`: `int g=3; int *const p=&g; const int *q=&g; main returns *p+*q` | exit 6 | C11 6.5.16.1p1, 6.7.9; HAND 3+3 | C11+HAND | L2→L3, A/B |
| V3-GL-16 | `globals/tentative_twice.c`: `int t; int t; main returns t` | exit 0 | C11 6.9.2p2 (one tentative definition yields one definition) | C11+HAND | L2→L3, A/B |
| V3-GL-17 | `globals/incomplete_tentative.c`: `int a[]; int main(void){a[0]=5;return a[0];}` | exit 5 | C11 6.9.2p2 (incomplete tentative array completed as one element) | C11+HAND | L2→L3, A/B |
| V3-GL-18 | `globals/completed_size.c`: `int a[]={1,2,3}; main returns sizeof a` | exit 12 | C11 6.7.9p22, 6.5.3.4; concrete `sizeof(int)==4` is probe-gated | C11+PROBE | L3, B |
| V3-GL-19 | `globals/const_expr.c`: `int g=1+2; main returns g` | exit 3 | C11 6.6 (arithmetic constant expression), 6.7.9p4 | C11+HAND | L2→L3, A/B |
| V3-GL-20 | `globals/write_through_ptr.c`: `int x=1; int *p=&x; *p=5; main returns x` | exit 5 | C11 6.5.3.2, 6.5.16.1; alias to same object | C11+HAND | L2→L3, A/B |
| V3-GL-21 | `globals/array_of_ptrs.c`: `int a=1,b=2; int *p[2]={&a,&b}; main returns *p[1]` | exit 2 | C11 6.6p9, 6.7.9 | C11+HAND | L3, B |
| V3-GL-22 | `globals/cross_tu_const.c`: one TU defines `int g=7;`, another has `extern int g;` and reads it | exit 7 | C11 6.2.2, 6.9.2; mixed-link Part B | C11+REF | L3/L4, B |
| V3-GL-23 | `globals/static_cross_tu.c`: TU A has `static int s=3; int a(void){return s;}`; TU B has `static int s=4; int b(void){return s;}`; one `main` calls `a()+b()` | exit 7; no link collision | C11 6.2.2p3 (internal linkage); HAND 3+4 | C11+HAND | L4, B |
| V3-GL-24 | `globals/bss_data.c`: one initialized global (`7`) and one zero global (`0`); main returns `g1-g2` | exit 7 | C11 6.7.9p10; `.data`/`.bss` placement is Part B | C11+PROBE | L3, B |

### 5.3 Pointers, arrays, decay, lvalue conversion (`V3-PT-*`)

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-PT-01 | `pointers/address_deref.c`: `int x=3; int *p=&x; main returns *p` | exit 3 | C11 6.5.3.2p2-3 | C11+HAND | L2→L3, A/B |
| V3-PT-02 | `arrays/subscript.c`: `int a[3]={10,20,30}; main returns a[1]` | exit 20 | C11 6.5.2.1p2 (E1[E2] equals *(E1+E2)) | C11+HAND | L2→L3, A/B |
| V3-PT-03 | `arrays/ptr_subscript.c`: `int *p=a; main returns p[2]` | exit 30 | C11 6.5.2.1p2, 6.5.6 | C11+HAND | L2→L3, A/B |
| V3-PT-04 | `arrays/commutative.c`: `main returns 1[a]` with `a[1]==20` | exit 20 | C11 6.5.2.1p2 | C11+HAND | L2→L3, A/B |
| V3-PT-05 | `pointers/arithmetic_diff.c`: `int a[4]={1,2,3,4}; int *p=a,*q=a+2; main returns ((q-p)==2) && (*(p+2)==3)` | exit 1 | C11 6.5.6p8-9; pointer difference type `ptrdiff_t` | C11+HAND | L2→L3, A/B |
| V3-PT-06 | `arrays/address_of_array.c`: `int a[3]={1,2,3}; int (*pa)[3]=&a; main returns (*pa)[1]` | exit 2 | C11 6.5.3.2p3, 6.3.2.1p3 (`&a` has pointer-to-array type) | C11+HAND | L2→L3, A/B |
| V3-PT-07 | `pointers/and_deref.c`: `int x=3; int *p=&x; int *q=&*p; main returns *q` | exit 3 | C11 6.5.3.2p3 (`&*p` neither evaluates `*` nor `&`) | C11+HAND | L2→L3, A/B |
| V3-PT-08 | `arrays/decay_call.c`: `int f(int *p){return p[1];} int a[3]={1,2,3}; main returns f(a)` | exit 2 | C11 6.3.2.1p3 (array-to-pointer conversion), 6.5.2.2 | C11+HAND | L2→L3, A/B |
| V3-PT-09 | `arrays/sizeof_decay.c`: `int a[3]; int *p=a; main returns (sizeof a==3*sizeof(int)) && (sizeof p==sizeof(int*))` | exit 1 | C11 6.5.3.4p2-4; concrete sizes are probe-gated | C11+PROBE | L3, B |
| V3-PT-10 | `arrays/string_decay.c`: `char s[4]="abc"; char *p="abc"; main returns (s[2]=='c') && (p[1]=='b')` | exit 1 | C11 6.3.2.1p3, 6.7.9p14; string-literal address is Part B | C11+PROBE | L3, B |
| V3-PT-11 | `arrays/multidim.c`: `int m[2][3]={{1,2,3},{4,5,6}}; main returns m[1][2]` | exit 6 | C11 6.5.2.1, 6.7.9 | C11+HAND | L2→L3, A/B |
| V3-PT-12 | `arrays/ptr_to_row.c`: `int (*p)[3]=m; main returns p[1][0]` | exit 4 | C11 6.5.6 (element stride is array size), 6.5.2.1 | C11+HAND | L3, B |
| V3-PT-13 | `arrays/param_adjust.c`: `int f(int a[10]){return sizeof a==sizeof(int*);} int main(void){int x[10];return f(x);}` | exit 1 | C11 6.7.6.3p7 (array parameter adjusted to pointer); concrete pointer size probe | C11+PROBE | L3, B |
| V3-PT-14 | `arrays/param_static.c`: `int f(int a[static 3]){return a[1];}` called with a 3-element array | exit 2 | C11 6.7.6.3p7 (static/qualifier inside parameter brackets) | C11+HAND | L2→L3, A/B |
| V3-PT-15 | `arrays/param_ptr_to_array.c`: `int f(int (*p)[3]){return p[1][1];}` called with `m[2][3]` | exit 5 | C11 6.7.6.3p7, 6.5.6 | C11+HAND | L2→L3, A/B |
| V3-PT-16 | `pointers/declarator_binding.c`: `int seven(void){return 7;} int (*get(void))(void){return seven;} main returns get()()` | exit 7 | C11 6.7.6.3 (declarator binding), 6.5.2.2 | C11+HAND | L2→L3, A/B |
| V3-PT-17 | `arrays/ptr_vs_array.c`: type-only assertions for `int *a[3]` (array of pointers) vs `int (*p)[3]` (pointer to array); no execution value asserted | distinct TypeIds/structural types; `sizeof` values Part B | C11 6.7.6.2, 6.7.6.3 | CONTRACT | L2/G, A |
| V3-PT-18 | `pointers/qualifier_discard.c`: `int x=3; const int *cp=&x; int *p=cp;` (invalid); `pointers/qualifier_add.c`: `int *q=&x; const int *cq=q;` (valid) | invalid: constraint diagnostic; valid: exit 3 | C11 6.5.16.1p1 (left pointed-to qualifiers) | C11 | L2, A |
| V3-PT-19 | `pointers/null.c`: `int *p=0; main returns p==0` | exit 1 | C11 6.3.2.3p3, 6.5.9 | C11+HAND | L2→L3, A/B |
| V3-PT-20 | `pointers/void_roundtrip.c`: `int x=3; void *v=&x; int *p=v; main returns *p` | exit 3 | C11 6.3.2.3p1, 6.5.16.1 | C11+HAND | L2→L3, A/B |
| V3-PT-21 | `pointers/lvalue_conversion.c`: `const int x=3; int y=x; main returns y` | exit 3; `x` keeps type `const int`, the initializer use carries an lvalue-conversion fact removing `const`, and `y` has type `int` | C11 6.3.2.1p2; qualifier removal is a conversion fact | CONTRACT+C11 | L2→L3, A/B |
| V3-PT-22 | `arrays/decay_exceptions.c`: `int a[3];` assert `sizeof a` no decay, `&a` no decay, `a` in `sizeof` is array; `sizeof &a` pointer | Part A type facts; Part B sizes 12/8 | C11 6.3.2.1p3, 6.5.3.4 | CONTRACT+PROBE | L2/L3, A/B |
| V3-PT-23 | `pointers/one_past.c`: `int a[3]={1,2,3}; int *p=a+3; main returns p==a+3` | exit 1 (do not dereference `p`) | C11 6.5.6p8 | C11+HAND | L2→L3, A/B |
| V3-PT-24 | `pointers/designator_sum.c`: `(&seven)()` plus `(*seven)()` | exit 14 | C11 6.3.2.1p4, 6.5.3.2 | C11+HAND | L2→L3, A/B |
| V3-PT-25 | `arrays/function_param.c`: `int f(int g(void)){return g();}` call `f(seven)` | exit 7 | C11 6.7.6.3p8 (function parameter adjusted to pointer) | C11+HAND | L2→L3, A/B |
| V3-PT-26 | `arrays/constant_bound.c`: `int a[2+1]; main returns sizeof a` | exit 12 | C11 6.7.6.2p1 (constant-expression bound), 6.5.3.4; `sizeof(int)==4` probe | C11+PROBE | L3, B |
| V3-PT-27 | `pointers/single_object_past.c`: `int x; int *p=&x; main returns (p+1)==(&x+1)` | exit 1 | C11 6.5.6p7 (single object behaves as one-element array; one-past valid) | C11+HAND | L2→L3, A/B |
| V3-PT-28 | `pointers/restrict_type.c`: `int f(int *restrict p){return *p;} int main(void){int x=3;return f(&x);}` | exit 3; no no-alias optimization claim | C11 6.7.3.1; TY34 later | CONTRACT+C11 | L2→L3, A/B |
| V3-PT-29 | `pointers/integer_cast.c`: `int *p=(int*)0; main returns p==0` | exit 1 (null pointer conversion is defined) | C11 6.3.2.3p3 | C11+HAND | L2→L3, A/B |
| V3-PT-30 | `pointers/remaining_exceptions.c`: `sizeof(f)` where `f` is a function; `sizeof(void)` in C11 mode | constraint diagnostic; GNU `sizeof(void)==1` is mode-gated, not V3/C11 | C11 6.5.3.4p1; dialect policy Q2 | C11 | L2, A |
| V3-PT-31 | `arrays/incomplete_extern.c`: `extern int a[]; int a[3]={1,2,3}; main returns a[2]` | exit 3 | C11 6.2.7 (composite type), 6.9.2 | C11+HAND | L2→L3, A/B |

### 5.4 Aliasing and effects (`V3-AL-*`)

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-AL-01 | `effects/distinct.c`: `int f(int *p,int *q){*p=1;*q=2;return *p;}` with `&x,&y` | exit 1 | C11 6.5.16, 6.8 (sequenced statements) | C11+HAND | L2→L3, A/B |
| V3-AL-02 | `effects/aliased.c`: same function called with `&x,&x` | exit 2 | C11 6.5.16; both stores sequenced | C11+HAND | L2→L3, A/B |
| V3-AL-03 | `effects/global_ptr.c`: `int g; int *p=&g; *p=5; main returns g` | exit 5 | C11 6.5.3.2, 6.5.16.1 | C11+HAND | L2→L3, A/B |
| V3-AL-04 | `effects/repeated_call.c`: `int g; void inc(void){g=g+1;} int main(void){inc();inc();return g;}` | exit 2 | C11 6.5.2.2 (call sequence point), 6.8.3 (compound statement) | C11+HAND | L2→L3, A/B |
| V3-AL-05 | `effects/indirect_store.c`: `void set(int **pp,int *p){*pp=p;} int x; int *q; set(&q,&x); *q=7; return x` | exit 7 | C11 6.5.2.2, 6.5.3.2 | C11+HAND | L2→L3, A/B |
| V3-AL-06 | `effects/store_fact.c`: `int f(int *p){*p=1;return *p;}` | exit 1; the `SemRecord` for the call/body must record a non-`EffectMask(0)` store effect | C11 6.5.16; T07 effect classes must be frozen | CONTRACT | L2, A |
| V3-AL-07 | `effects/alias_unknown.c`: `int f(int *p,int *q){int a=*p;*q=9;return a+*p;}` called with distinct and aliased pointers | distinct exit 0, aliased exit 9 | C11 6.5.16; conservative alias required at `-O0` | C11+HAND | L2→L3, A/B |
| V3-AL-08 | `effects/call_effects.c`: `int g; void set(void){g=5;} int main(void){g=0;set();return g;}`; caller IR must not assume `g` is unchanged | exit 5 | C11 6.5.2.2; T09 call effects; no optimization at `-O0` | CONTRACT+C11 | L2→L3, A/B |
| V3-AL-09 | `effects/volatile_deferred.c`: `volatile int v;` access | explicit `Unsupported`/deferred, not a silent non-volatile access | V3 scope; T07 SE27 later | CONTRACT | L2, A |
| V3-AL-10 | `effects/atomic_deferred.c`: `_Atomic int a;` | explicit `Unsupported`/deferred | V3 scope; T07 SE28 later | CONTRACT | L2, A |

### 5.5 Invalid C and constraint diagnostics (`V3-NEG-*`)

All `V3-NEG` cases require: exactly one primary diagnostic with the frozen group/code/rule; no output artifact; no partial semantic record; finite recovery. A test that accepts "any failure" is non-conforming.

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-NEG-01 | `negative/undeclared_call.c`: call to an undeclared function in C11 mode | constraint diagnostic; no implicit-int success | C11 6.5.2.2p1, 6.5.1p2; mode policy Q2 | C11 | L2, A |
| V3-NEG-02 | `negative/arg_count.c`: `int f(int); f();` | constraint diagnostic | C11 6.5.2.2p2 | C11 | L2, A |
| V3-NEG-03 | `negative/arg_type.c`: `int f(int*); f(1);` | constraint diagnostic | C11 6.5.2.2p2, 6.5.16.1 | C11 | L2, A |
| V3-NEG-04 | `negative/return_type.c`: `int *f(void){int x; return x;}` | constraint diagnostic | C11 6.8.6.4p3, 6.5.16.1 | C11 | L2, A |
| V3-NEG-05 | `negative/array_assign.c`: `int a[3]; int *p; a=p;` | constraint diagnostic (array not modifiable lvalue) | C11 6.3.2.1p1, 6.5.16.1p1 | C11 | L2, A |
| V3-NEG-06 | `negative/deref_nonptr.c`: `int x; *x;` | constraint diagnostic | C11 6.5.3.2p2 | C11 | L2, A |
| V3-NEG-07 | `negative/call_nonfn.c`: `int x; x();` | constraint diagnostic | C11 6.5.2.2p1 | C11 | L2, A |
| V3-NEG-08 | `negative/return_void.c`: `void f(void){return 1;}` and `int g(void){return;}` | two constraint diagnostics | C11 6.8.6.4p1 | C11 | L2, A |
| V3-NEG-09 | `negative/duplicate_param.c`: `int f(int a,int a){return a;}` | constraint diagnostic | C11 6.9.1 (unique parameter names) | C11 | L2, A |
| V3-NEG-10 | `negative/global_redef.c`: `int g=1; int g=2;` | constraint diagnostic | C11 6.9p5 | C11 | L2, A |
| V3-NEG-11 | `negative/nonconst_global_init.c`: `int g=f();` at file scope | constraint diagnostic | C11 6.7.9p4 | C11 | L2, A |
| V3-NEG-12 | `negative/excess_array_init.c`: `int a[2]={1,2,3};` | constraint diagnostic | C11 6.7.9p2 | C11 | L2, A |
| V3-NEG-13 | `negative/zero_bound.c`: `int a[0];` and `int b[-1];` | constraint diagnostic in C11 | C11 6.7.6.2p1 | C11 | L2, A |
| V3-NEG-14 | `negative/qualifier_discard.c`: `const int *cp; int *p=cp;` | constraint diagnostic; reverse valid | C11 6.5.16.1p1 | C11 | L2, A |
| V3-NEG-15 | `negative/array_of_functions.c`: `int a[3](void);`; `int f(void)[3]` | constraint diagnostics | C11 6.7.6.2p1, 6.7.6.3p1 | C11 | L2, A |
| V3-NEG-16 | `negative/void_object.c`: `void v;`; `void f(void, int);` | constraint diagnostics | C11 6.7.6.2, 6.7.6.3 (void restriction) | C11 | L2, A |
| V3-NEG-17 | `negative/function_incompat.c`: `int f(void); int f(int);` | constraint diagnostic (incompatible declarations) | C11 6.2.7, 6.7.6.3p15 | C11 | L2, A |
| V3-NEG-18 | `negative/incomplete_object.c`: `extern int a[];` with no completion and `sizeof a` before any completing declaration | constraint diagnostic where completeness is required | C11 6.5.3.4p1, 6.7p7, 6.9.2 | C11 | L2, A |
| V3-NEG-19 | `negative/ptr_from_int.c`: `int *p=1;` | constraint diagnostic (only null pointer constant allowed) | C11 6.3.2.3p3, 6.5.16.1 | C11 | L2, A |
| V3-NEG-20 | `negative/duplicate_function_def.c`: two definitions of the same external function in one TU | constraint diagnostic | C11 6.9p5 | C11 | L2, A |
| V3-NEG-21 | `negative/const_assign.c`: `const int x=1; x=2;` | constraint diagnostic (not a modifiable lvalue) | C11 6.3.2.1p1, 6.5.16.1p1 | C11 | L2, A |
| V3-NEG-22 | `negative/array_bound_nonconst.c`: `int a[1/0];` | constraint diagnostic (constant expression not representable / division by zero) | C11 6.6p4, 6.7.6.2p1 | C11 | L2, A |

**Explicitly excluded UB/unspecified rows (do not assert output):**

- dereference of null or invalid pointer (`*p` with `p==0`);
- out-of-bounds array access and pointer arithmetic beyond one-past;
- pointer subtraction between different array objects;
- unsequenced modifications (`f(i++, i++)`, `i = i++`);
- reading an uninitialized automatic object;
- falling off a non-void non-`main` function and using the value;
- relational comparison of pointers to different objects;
- signed integer overflow;
- `memcpy` overlap; `restrict` violations;
- any implementation-defined conversion (pointer↔integer, `char` signedness negative values, `long double`).

These are not V3 acceptance cases; a test may compile them to check a diagnostic required by another rule, but must not use one reference result as a universal oracle.

### 5.6 Unsupported features and recovery (`V3-UNS-*`)

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-UNS-01 | `unsupported/knr.c`: old-style definition `int f(a) int a; {return a;}` | explicit `Unsupported` if V3 excludes it; never silently treated as `(void)` | V3 scope decision Q2; C11 still permits old-style definitions | CONTRACT | L2, A |
| V3-UNS-02 | `unsupported/variadic.c`: `int f(int,...);` | explicit `Unsupported` (T12/Stage D) | V3 scope; T11 variadic ABI later | CONTRACT | L2, A |
| V3-UNS-03 | `unsupported/vla.c`: `int f(int n){int a[n];return sizeof a;}` | explicit `Unsupported` (T08 CL15 later) | V3 scope | CONTRACT | L2, A |
| V3-UNS-04 | `unsupported/aggregate.c`: struct/union/bitfield parameter or global | explicit `Unsupported` (Stage D) | V3 scope | CONTRACT | L2, A |
| V3-UNS-05 | `unsupported/float.c`: `double f(double x)` or float global | explicit `Unsupported` (Stage D) | V3 scope | CONTRACT | L2, A |
| V3-UNS-06 | `unsupported/atomic_volatile.c`: `_Atomic` or volatile access semantics | explicit `Unsupported`/deferred | V3 scope; T07 later | CONTRACT | L2, A |
| V3-UNS-07 | `unsupported/tls.c`: `_Thread_local int t;` | explicit `Unsupported` (T11 CG34 later) | V3 scope | CONTRACT | L2, A |
| V3-UNS-08 | `unsupported/gnu.c`: inline asm, nested function, computed goto, `typeof` | explicit `Unsupported`/T12 | V3 scope; T12 | CONTRACT | L2, A |
| V3-UNS-09 | `unsupported/generic.c`: `_Generic`/vector/complex | explicit `Unsupported`/T12 | V3 scope | CONTRACT | L2, A |
| V3-UNS-10 | `recovery/bad_call_then_good.c`: invalid call in `f`, then valid `g` in same TU | one diagnostic for `f`; `g` still checked and committed; finite ticks | C11 constraint; T05 PA38 recovery; task progress | CONTRACT | L2, A |
| V3-UNS-11 | `recovery/bad_global_then_good.c`: `int bad=f(); int good=3;` | diagnostic for `bad`; no partial init record; `good` plan committed | C11 6.7.9p4; atomic task failure | CONTRACT | L2, A |
| V3-UNS-12 | `recovery/bad_declarator_then_good.c`: malformed parameter list followed by a valid function | parser sync at `;`/`}`; valid function still checked; no scope leak | C11 6.9.1; T05 PA38 | CONTRACT | L2, A |
| V3-UNS-13 | `recovery/unsupported_no_artifact.c`: unsupported feature in an otherwise valid program | typed diagnostic; no output object/assembly; no fabricated success | DoD; `AGENTS.md` §4.2/§6 | CONTRACT | L2, A |
| V3-UNS-14 | `recovery/no_spin.c`: a task that cannot progress (dangling callee/type) | one typed terminal failure; no repeated selection; tick budget respected | T01 §4; C06 task progress rules | CONTRACT | L1, A |

### 5.7 Protocol, non-matching tasks, and undeclared writes (`V3-PRO-*`)

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-PRO-01 | Dispatch a V3 task kind to a chip that does not accept it (pure `compute` and Bus `drive_task`) | typed `Task` diagnostic/`DriveError`; no semantic state change | T01 §4; `docs/tasks/README.md` §5 item 3 | CONTRACT | L0/L1, A |
| V3-PRO-02 | Wrong payload arity/family (e.g. `Node` where `Type` expected; empty payload) | typed `Task` failure; nothing appended | T01 §4; existing c12/c14 pattern | CONTRACT | L0/L1, A |
| V3-PRO-03 | Dangling `Node`/`Type`/`Symbol`/`Function`/`Block`/`Value` refs | typed failure; no partial record | T01 §4; VF01 | CONTRACT | L1, A |
| V3-PRO-04 | Manifest missing a declared write for a V3 chip | `ManifestRegistryError`/`ManifestError` at registration; chip not installed | C04; `compiler/src/manifest.rs:434-740` | CONTRACT | G, A |
| V3-PRO-05 | `StorePatch` to a field not in the chip's write manifest | `CommitError::WriteNotDeclared`/`UndeclaredStoreField`; no mutation | C04/C06; `compiler/src/commit.rs:171-186` | CONTRACT | L1, A |
| V3-PRO-06 | Two chips claim the same store field; a chip claims a kind twice | `ManifestError::StoreOwnerViolation` for the field; `ManifestError::DuplicateKindClaim` for the kind; second registration rejected | T06 item 1; `compiler/src/manifest.rs:522,1623` | CONTRACT | G, A |
| V3-PRO-07 | Wrong phase, layer, backend class, or category | registration/driver rejection; no task execution | T01 §5; C04 | CONTRACT | G/L1, A |
| V3-PRO-08 | Duplicate `Complete`/`Fail` in one batch | `CommitError::DuplicateCompletion`; whole batch rejected | C03/C06; `compiler/src/commit.rs` | CONTRACT | L1, A |
| V3-PRO-09 | Consume a child result twice | `ResultAlreadyConsumed`; no duplicate effect | C03; `compiler/src/commit.rs` | CONTRACT | L1, A |
| V3-PRO-10 | Await-all join: one child fails while others complete | parent `Failed` exactly once, never `Ready`; no partial parent record | T05 item A; T02 join rules | CONTRACT | L1, A |
| V3-PRO-11 | Failed commit mid-batch | no semantic state committed; dispatched task transitions to `Failed` once; not left `Running` | C06/H6; T13 H6-M02/M07 | CONTRACT | L1, A |
| V3-PRO-12 | Chip shape/I/O/cross-chip lint | unit struct ZST; no `std::fs/process/env/time`; no chip calls; chip-lint passes | T01 §4.1; `tools/chip-lint` | CONTRACT | G, A |
| V3-PRO-13 | Read-scope audit limitation | manifest read set is reviewed; VF04 read monitoring is not a proof | T13 VF04 caveat | CONTRACT | G, A |
| V3-PRO-14 | Non-matching task snapshot equality | snapshot before/after a rejected task is byte-identical | T01 §4; `docs/tasks/README.md` §5 item 3 | EQ | L1, A |
| V3-PRO-15 | A V3 chip declares a task kind it does not implement or a store it does not own | registration rejection; no silent accept | T01 §5; C04 | CONTRACT | G, A |
| V3-PRO-16 | Missing test path in manifest | `ManifestError::MissingTests`; chip not registered | C04; `compiler/src/manifest.rs:724` | CONTRACT | G, A |

### 5.8 Replay and determinism (`V3-RPL-*`)

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-RPL-01 | Fresh Bus, same V3 source/state/config/pins, run twice | identical `Snapshot::capture` bytes and trace hashes | T01 C05; README replay gate | EQ | L2, A |
| V3-RPL-02 | H04 candidate binary run twice on the same V3 fixture | byte-identical evidence report; if a future report carries scheduler-only fields, compare the declared canonical projection and record the exclusion | H04; M1 D1/D2/D8 | EQ | L2, A |
| V3-RPL-03 | ID allocation across repeated runs | identical `SymbolId`/`TypeId`/`FunctionId`/`BlockId`/`ValueId` sequences | T01 §3; no address/hash-map ordering | EQ | L2, A |
| V3-RPL-04 | Failed task then retry | no partial records; retry result identical to a fresh run | C06; T13 H6 | EQ+CONTRACT | L1, A |
| V3-RPL-05 | Per-tick trace comparison | first divergence reported with tick/task; none for identical input | T13 VF13; M1 D7/D8 | EQ | L2, A |
| V3-RPL-06 | Part B repeated assembly emission | assembly text hash identical across runs; object hash only if H01 toolchain is deterministic | T00 H09; M1 D3 | EQ+PROBE | L3, B |
| V3-RPL-07 | Part B repeated execution | exit code/result identical across at least three runs | T00 H09 | EQ+REF | L3/L4, B |
| V3-RPL-08 | Quota/scheduler projection | canonical semantic projection ignores scheduler-only registers; separate scheduler snapshot deterministic | M1 D8; T13 VF13 | CONTRACT+EQ | L2, A |
| V3-RPL-09 | Interpreter vs target execution for well-defined V3 cases | same observable result; divergence is a defect | T13 VF12; M1 D8 | CONTRACT+REF | L2/L3, A/B |
| V3-RPL-10 | Snapshot content | no host path, pointer address, timestamp, hash-map order, or environment value appears in semantic snapshot | T01 §3; C05 | CONTRACT | L2, A |

### 5.9 ABI boundary, probe-gated (`V3-ABI-*`)

Every target-valued row here is `B` and requires `target_verification=verified` plus a C02-attested probe report. Concrete values are **expectations**, not verified values, until then. `V3-ABI-17`/`V3-ABI-18` are scope-boundary rows (`A`/not applicable), not target-value assertions.

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-ABI-01 | Candidate defines `int cand_add(int,int)`, reference GCC caller calls it | exit 5 | AAPCS64 scalar args/return; C11 6.5.2.2 | REF+PROBE | L4, B |
| V3-ABI-02 | Reference GCC defines `ref_add`, candidate caller calls it | exit 5 | AAPCS64 scalar args/return | REF+PROBE | L4, B |
| V3-ABI-03 | Pointer argument and pointer return, both directions | exit 3 via `*idp(&x)` | AAPCS64 pointer representation (LP64) | REF+PROBE | L4, B |
| V3-ABI-04 | 8 integer arguments `sum8(1..8)` | exit 36 | AAPCS64 first 8 GP registers; HAND sum | REF+PROBE | L3, B |
| V3-ABI-05 | 9 integer arguments `sum9(1..9)` | exit 45; 9th argument on stack | AAPCS64 stack argument; HAND sum | REF+PROBE | L3, B |
| V3-ABI-06 | Stack alignment at call boundary; `TargetVerifyChip` synthetic frame/SP invariant | reject misaligned SP; accepted frame meets `abi.stack_align` | T11 CG32/CG33/CG38; probe `abi.stack_align` | PROBE | L4, B |
| V3-ABI-07 | Callee-saved registers preserved across a candidate call while reference caller keeps values live | correct result; no clobbered live value | AAPCS64 callee-saved set; T11 CG06/CG26 | REF+PROBE | L4, B |
| V3-ABI-08 | Global data symbol interop: candidate defines, reference reads; reference defines, candidate reads | expected values 7 and 9 | ELF symbol/relocation; T11 CG34/CG36 | REF+PROBE | L4, B |
| V3-ABI-09 | Function pointer passed across link boundary, both directions | expected `inc(5)=6` | AAPCS64 indirect call; T11 CG08 | REF+PROBE | L4, B |
| V3-ABI-10 | Narrow argument extension: `char`/`short` parameter called from reference | correct positive value; extension policy from probe/AAPCS64 | AAPCS64 parameter passing; T11 CG07 | REF+PROBE | L4, B |
| V3-ABI-11 | Signed/unsigned high-bit values: `unsigned` 32/64 comparison and return | expected 1 for high bit set | C11 6.3.1.3/6.5.8; target width probe | REF+PROBE | L3, B |
| V3-ABI-12 | LP64 model: `sizeof(long)==8`, `sizeof(pointer)==8`, `sizeof(size_t)==8`, `sizeof(ptrdiff_t)==8` | all 1 | probe fixture fields; T01 §6 | PROBE | L3, B |
| V3-ABI-13 | `main(int argc,char **argv)` with runner-provided args | `argc==3`, `argv[3]==NULL`; exit 0 | C11 5.1.2.2.1p2; H01 runner | REF+PROBE | L3, B |
| V3-ABI-14 | ELF object inspection: ELF64, little-endian, AArch64, no Mach-O directives/prefixes | header/identity matches frozen target | T01 §6; T11 §1 | PROBE | L3, B |
| V3-ABI-15 | PIE/PIC global addressing as configured by H01 | correct global access under the selected relocation model | T11 CG17/CG34; H01 config | PROBE | L4, B |
| V3-ABI-16 | `x18` platform-register policy | expected policy from H07/probe; no fixture yet; unresolved until classified | T11 probe qualifier; T01 §6 | PROBE | L4, B |
| V3-ABI-17 | TLS `_Thread_local` | deferred/unsupported in V3; T11 CG34 later | V3 scope; T11 probe qualifier | CONTRACT | L2, A |
| V3-ABI-18 | Aggregate/HFA/large-struct/`long double`/`__int128`/variadic mixed-link | **not V3**; Stage D/T12, listed to prevent scope leakage | T11 mandatory tests; V3 boundary Q1 | CONTRACT | — |
| V3-ABI-19 | Large frame/stack probe and deep bounded recursion | correct result; no stack corruption; configured resource limit honored | T11 CG32/CG33; C11 recursion semantics | REF+PROBE | L4, B |
| V3-ABI-20 | `main` returns 5; runner observes exit status 5 | exit 5 | C11 5.1.2.2.3; H05 runner | REF+PROBE | L3, B |

### 5.10 Mixed-link (`V3-MIX-*`)

Rule: the candidate compiles its own C source; the reference GCC compiles only the **independent counterpart** and test harness. Never pass candidate C source to GCC/Clang. Both objects are produced separately and linked by the external linker.

| ID | Input / fixture | Expected | Basis | Oracle | Level/gate |
|---|---|---|---|---|---|
| V3-MIX-01 | Candidate `cand_add`; reference `main` calls it | exit 5 | C11 6.5.2.2; AAPCS64 | REF+PROBE | L4, B |
| V3-MIX-02 | Reference `ref_add`; candidate `main` calls it | exit 5 | C11 6.5.2.2; AAPCS64 | REF+PROBE | L4, B |
| V3-MIX-03 | Mutual recursion: candidate `cand_even` calls reference `ref_odd`; reference `main` starts at `cand_even(7)` | exit 0 for even(7) | C11 6.5.2.2; HAND trace | REF+PROBE | L4, B |
| V3-MIX-04 | Candidate defines `int cand_g=7;` reference reads it; reference defines `int ref_g=9;` candidate reads it | exit 16 (7+9) | C11 6.2.2/6.9.2; ELF data symbols | REF+PROBE | L4, B |
| V3-MIX-05 | Function-pointer table split across objects: candidate table holds reference function; reference table holds candidate function | expected sum 15 | C11 6.5.2.2; ELF relocations; HAND 6+9=15 | REF+PROBE | L4, B |
| V3-MIX-06 | Array/pointer parameter crosses boundary: candidate passes `int[4]={1,2,3,4}` to reference `sum(const int *,int)`, which reads through the pointer and returns the sum | exit 10 | C11 6.5.2.2, 6.5.2.1; AAPCS64 pointer; HAND 1+2+3+4 | REF+PROBE | L4, B |
| V3-MIX-07 | Internal `static` symbols with same spelling in candidate and reference objects | no duplicate-symbol link error; values independent | C11 6.2.2p3; ELF local symbols | REF+PROBE | L4, B |
| V3-MIX-08 | `main` in candidate with reference helper; and `main` in reference with candidate helper | both link and exit expected values | C11 5.1.2.2; ELF entry | REF+PROBE | L4, B |
| V3-MIX-09 | Repeat mixed-link build/run three times | assembly text/exit stable; object bytes only if toolchain deterministic | T00 H09; M1 D3 | EQ+PROBE | L4, B |
| V3-MIX-10 | Reference counterpart compiled with pinned GCC/version/flags and `noinline` where needed; candidate source never passed to GCC | provenance check: command log contains no candidate `.c` in GCC invocation | T00/AGENTS §6; `T11_TARGET_CHIPS.md:53-55` | CONTRACT | L4, B |

---

## 6. Required-dimension traceability

| Required dimension | Cases |
|---|---|
| Normal | V3-FN-01..09, V3-GL-01..20, V3-PT-01..31, V3-AL-01..08 |
| Boundary | V3-FN-13/14/17/18/21/22, V3-GL-16/17/18/24, V3-PT-09/13/22/23/26/27, V3-ABI-04/05/12/19 |
| Invalid | V3-NEG-01..22 |
| Unsupported/recovery | V3-UNS-01..14 |
| Replay | V3-RPL-01..10 |
| Non-matching task | V3-PRO-01/02/07/14/15 |
| Undeclared write | V3-PRO-04/05/06/13/16 |
| Recursion | V3-FN-10/11/20/22, V3-ABI-19 |
| Pointer/array decay | V3-PT-06/07/08/09/10/22/23/24/25 |
| Lvalue conversion | V3-PT-21/22, V3-GL-14/15 |
| Argument/return conversion | V3-FN-03/15/16/21, V3-PT-13/18/19/20/25, V3-ABI-10/11 |
| Global initialization | V3-GL-01..24, V3-MIX-04 |
| Aliasing/effects | V3-AL-01..10, V3-AL-06/08 IR call/load/store facts |
| Mixed-link | V3-MIX-01..10 |
| ABI boundary | V3-ABI-01..20, V3-MIX-01..10 |

---

## 7. Integration ladder

The suite should grow in this order; a later rung cannot mask a failed earlier rung.

| Rung | Content | Exit criterion | Evidence |
|---|---|---|---|
| I1 | Minimal V3 call: `add(2,3)` | L0/L1 call/return chips pass; one function/one call in IR; no ABI claim | chip tests + Bus snapshot |
| I2 | Pairwise: parse→type, type→sem, sem→IR for call/pointer/array | both sides of each handoff use real committed records; no private projection | stage snapshots |
| I3 | Multi-function single TU: globals, recursion, pointer/array traversal | L2 frontend chain produces committed symbols/types/sem/IR; replay equal | Part A evidence |
| I4 | Multi-TU frontend: `extern`, `static`, global init | cross-TU linking deferred; per-TU symbols correct; no shared mutable frontend state | two-bus integration |
| I5 | Part B single TU: `add`, globals, arrays, pointers | candidate `-S` → external asm/link → runner exit expected; ABI-01..14 pass | assembly/object/run artifacts |
| I6 | Part B mixed-link | MIX-01..10 pass; both directions | link map/command log/exit |
| I7 | Full V3 replay/regression | all Part A/B rows pass or are explicitly blocked; 3-run stability | evidence report |

**Integration discipline:** tests must consume real upstream records; synthetic fixtures may substitute for upstream input only, never for the chip's own semantics (`MASS_PARALLEL_DISPATCH.md:187-204`). A chip passing L0 does not imply L2 or L3.

---

## 8. Replay protocol

1. **Fresh-state run.** Create a new `CompilerBus` from the same `CompilerConfig`, target identity, dialect, options, and limits; seed only the declared source/pp fixtures.
2. **Capture.** Record `Snapshot::capture(&bus)` bytes, trace hash, committed `TaskId`/`ResultId`/`DiagnosticId` sequences, and the per-stage record tables.
3. **Repeat.** Run the same case from a second fresh bus. Compare byte-for-byte; no normalization is allowed for semantic fields.
4. **First divergence.** On mismatch, report tick, task, chip, proposal index, and first differing record/field; do not regenerate a golden from the new run.
5. **Scheduler projection.** Compare the canonical semantic projection and the separate scheduler snapshot per M1 D8; do not conflate them (`M1_TARGET_ACCEPTANCE.md` §4.3 D8).
6. **Candidate binary.** Run the H04 candidate twice; compare the evidence report bytes and hashes. The report must not contain timestamps, absolute paths, hostnames, or random IDs.
7. **Part B.** Compare assembly text hashes across runs; object bytes only under a deterministic H01 toolchain. Execute at least three times and compare exit/result; H05 isolation and timeouts apply.
8. **Failure injection.** Repeat a negative case; the same diagnostic identity, same missing artifacts, and same snapshot hash (before/after failure) must result.

A replay pass is evidence of determinism for the tested input, not a formal proof of global determinism.

---

## 9. Negative, recovery, and protocol execution rules

### 9.1 Negative cases

For every `V3-NEG`/`V3-UNS` case:

1. Assert exactly one primary diagnostic with the frozen group/code/rule ID and source location.
2. Assert no output artifact (no `.s`, `.o`, executable, or success report) was produced.
3. Assert no partial semantic record for the rejected declaration/expression; completed earlier records remain.
4. Assert the task reaches a terminal state (`Failed` with diagnostic) and is not left `Running`/`Waiting`.
5. Assert recovery is finite: the next tick count is bounded; no silent spin.
6. Do not accept a different error group/code or a generic "failed" as a pass.

If a V3 diagnostic code is not frozen, the case is `BLOCKED_CONTRACT`; implementation may not choose the expected code after the fact.

### 9.2 Unsupported feature cases

`Unsupported` is a typed, explicit gap. It is only acceptable when the V3 contract says the feature is out of scope. For a V3-required feature, `Unsupported` is a failure. The test must also assert that the unsupported feature does not produce a fabricated result, partial record, or no-op success.

### 9.3 Protocol cases

- **Non-matching task:** call the chip's pure core with a foreign task kind/payload, then dispatch the same task through the Bus. The pure result must be a typed `Task` failure; the Bus must reject before mutation; the snapshot must be unchanged.
- **Undeclared write:** mutate the manifest to remove a declared write and expect registration failure; add a write outside the owner allowlist and expect `StoreOwnerViolation`; commit a `StorePatch` to an undeclared field and expect `WriteNotDeclared`/`UndeclaredStoreField`; assert no record changed.
- **Read scope:** a manifest test can prove the declared read set, not the actual reads. State this limitation; VF04 read monitoring is not implemented today and is not a proof.

---

## 10. ABI-gated and mixed-link procedure

### 10.1 Probe gate

Before any `V3-ABI-*`/`V3-MIX-*` run:

1. H01 has provisioned the Linux AArch64 substrate/toolchain/runner and run the probe.
2. The probe report has `verified=true` and a report hash; H07 has classified every `unresolved` ABI field; the T01 integrator has attested C02.
3. `CompilerConfig::ensure_codegen_ready` succeeds for the frozen target identity.
4. H04 `-S`/`-c` emission and H05 isolation/timeouts are available.
5. The test records probe report hash, toolchain versions, GCC reference version, flags, sysroot, runner, and environment.

V3-specific probe fields that must be resolved for these rows: `sizeof`/`alignof` for `int`, `long`, pointer, `size_t`, and `ptrdiff_t`; `char.signed` (positive-only V3 char cases do not depend on it); `abi.gp_arg_regs`; `abi.stack_align`; endianness/LP64/ELF identity; and the relocation/PIE policy. `abi.fp_arg_regs`, HFA/HVA, variadic save area, `long double`, and `__int128` are not used by this V3 subset (Q1). `x18`, relocation/TLS forms, and target features remain explicitly unresolved even after a passing probe until H07 classification or a fixture extension (T11 probe qualifier).

Until then, `V3-ABI-*`/`V3-MIX-*` are `BLOCKED_PROBE`; they must not run with macOS arm64 values, Rust host sizes, or a synthetic target.

### 10.2 Reference counterpart rules

- GCC/Clang may compile only the **reference counterpart** and test harness, never the candidate C source.
- The candidate object comes from candidate-generated assembly assembled by the external assembler; the candidate source is not delegated to GCC.
- Reference code should use `noinline` where the call boundary is essential; flags are pinned by H01.
- The reference result is not a universal C oracle; expected values come from C11/hand calculation. Reference evidence is used for ABI compatibility and same-target behavior.
- The mixed-link test log must show separate commands for candidate assembly/object and reference compile; a violation is a test defect.

### 10.3 Required two-way matrix

For each of scalar int, pointer, global data, function pointer, and array/pointer parameter:

- candidate callee + reference caller;
- reference callee + candidate caller;
- link, run, compare expected result.

This satisfies T11's mandatory candidate-callee/reference-caller and reverse requirement (`T11_TARGET_CHIPS.md:53-55`) for the V3 scalar/pointer subset. Aggregate/HFA/variadic/`long double`/`__int128` variants are explicitly deferred (Q1).

---

## 11. What Part A evidence can cover

| V3 family | Part A evidence | Not Part A |
|---|---|---|
| Functions/calls/returns | AST nodes for prototypes/definitions/parameters/calls/returns; scopes and symbols; function types; argument/return conversion plans; recursion/CFG; IR call/return shape; interpreter result if a V3 interpreter is frozen | Register/stack argument locations; return-register materialization; call-clobber preservation; actual exit status |
| Globals | File-scope symbols; linkage/storage; tentative merging; initializer plan structure (constant, zero, string, symbolic address); array bound completion; duplicate/conflict diagnostics | `.data`/`.bss` bytes; concrete offsets/alignment; relocation encoding; process startup |
| Pointers/arrays | Pointer/array/function types; declarator binding; decay and lvalue conversion; qualifier compatibility; subscript/address/deref IR; element-index semantics; one-past validity | `sizeof`/`_Alignof` concrete values; byte strides; address representation; pointer↔integer implementation-defined behavior |
| Effects/aliasing | Lvalue/modifiable-lvalue facts; store/call effect facts; conservative alias decision at `-O0`; effect-mask classes if frozen | Register clobbers; machine-level memory ordering; volatile/atomic hardware semantics |
| Negative/recovery | Constraint diagnostics that are target-independent; recovery sync; finite progress; no partial records | ABI-level rejection (e.g. stack alignment, relocation overflow) |
| Replay | Snapshot/trace determinism for the frontend/IR chain; stable IDs | Deterministic object bytes/runner behavior |

**Part A current status.** Even Part A V3 cannot run today: V3 parse/type/sem/IR producers, records, and task kinds do not exist. The H04 driver's Part A pipeline is M1-only and rejects V3 source. Part A V3 evidence becomes possible only after the V3 contract freeze and the corresponding Stage B implementations; it remains **evidence of frontend/IR structure, not target execution**.

If a V3 IR interpreter is added (VF12 extension), it may evaluate well-defined V3 programs with a frozen memory model and report modeled values. That is verification-only: it does not prove ABI, assembly, linking, or target execution. `Unsupported` in the interpreter is never equal to a pass (`T13_VERIFICATION_CHIPS.md:81`).

---

## 12. Future commands and paths

These are **planned** work-order commands, not executed here. Exact H01 toolchain paths and probe report fields must be filled by H01/H07/T01 before Part B.

### 12.1 Part A test suites (after V3 freeze/implementation)

```sh
cargo fmt --manifest-path compiler/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings

cargo test --locked --manifest-path compiler/Cargo.toml --test v3_calls
cargo test --locked --manifest-path compiler/Cargo.toml --test v3_globals
cargo test --locked --manifest-path compiler/Cargo.toml --test v3_pointers_arrays
cargo test --locked --manifest-path compiler/Cargo.toml --test v3_negative
cargo test --locked --manifest-path compiler/Cargo.toml --test v3_protocol
cargo test --locked --manifest-path compiler/Cargo.toml --test v3_replay
```

One case only (for failure localization; the family test reads `V3_CASE`):

```sh
V3_CASE=V3-FN-01 cargo test --locked --manifest-path compiler/Cargo.toml \
  --test v3_calls -- --nocapture
```

### 12.2 Part A candidate evidence (H04, after V3 pipeline integration)

```sh
cargo build --locked --manifest-path compiler/Cargo.toml --bin candidate
./compiler/target/debug/candidate -std=c11 -O0 \
  --emit-ir-snapshot --emit-trace --interpret-ir \
  -o target/v3-evidence/V3-FN-01.report \
  compiler/tests/fixtures/v3/calls/add.c
```

Repeat twice and compare report hashes; the expected interpreted value is the case table literal, never read from the report. The exact H04 flag spelling is H04-owned and illustrative until the H04 contract freezes; `-S`/`-c` remain refused until Part B opens.

### 12.3 Part B single-TU (probe verified; H01/H05 filled)

```sh
# Environment values are H01-pinned; placeholders must not be invented.
export CC_SILICON_V3_PROBE_REPORT=<H01-report-path>
export CC_SILICON_AARCH64_CC=<H01-pinned-cross-cc>
export CC_SILICON_AARCH64_RUNNER=<H01-pinned-runner>

cargo test --locked --manifest-path compiler/Cargo.toml \
  --test v3_abi_exec -- --ignored --nocapture
```

The future `v3_abi_exec` test must fail closed if the probe report is missing/unverified. It must not substitute macOS values.

### 12.4 Part B mixed-link (probe verified; H01/H05 filled)

```sh
cargo test --locked --manifest-path compiler/Cargo.toml \
  --test v3_mixed_link -- --ignored --nocapture
```

The test log must contain separate candidate assembly/object commands and reference GCC commands; a candidate `.c` passed to GCC is a test defect.

### 12.5 Proposed paths (not created)

```text
compiler/tests/v3_common/mod.rs
compiler/tests/v3_calls.rs
compiler/tests/v3_globals.rs
compiler/tests/v3_pointers_arrays.rs
compiler/tests/v3_negative.rs
compiler/tests/v3_protocol.rs
compiler/tests/v3_replay.rs
compiler/tests/v3_abi_exec.rs          # #[ignore] Part B
compiler/tests/v3_mixed_link.rs        # #[ignore] Part B
compiler/tests/fixtures/v3/calls/add.c
compiler/tests/fixtures/v3/globals/init_int.c
compiler/tests/fixtures/v3/pointers/address_deref.c
compiler/tests/fixtures/v3/arrays/subscript.c
compiler/tests/fixtures/v3/effects/aliased.c
compiler/tests/fixtures/v3/negative/arg_count.c
compiler/tests/fixtures/v3/unsupported/vla.c
compiler/tests/fixtures/v3/abi/sum8.c
compiler/tests/fixtures/v3/mixed/cand_add.c
target/v3-evidence/                     # generated, not committed
target/v3-reference/                    # generated reference objects, not committed
```

---

## 13. Dependencies and blockers

| ID | Blocker | Current state | Needed to unblock | Owner |
|---|---|---|---|---|
| B1 | V3 contract freeze (types, nodes, task kinds, IR ops, effect classes, diagnostics) | `t01-c01-c06/39` foundation only; no V3 schema | Complete `CONTRACT_CHANGE_REQUEST`; integrator freeze and hash bump | T01 integrator + T05/T06/T07/T08/T09 |
| B2 | V3 parser productions (general declarators, calls, globals, arrays, initializers) | M1/limited Wave-3 parse slices only | PA05–PA10, PA14–PA19, PA27–PA28, PA32/PA38 work | T05 |
| B3 | V3 types/conversions/lookup | M1 `int`/`int(void)` and identity conversions only | TY07, TY10–TY20, TY21–TY29 (pointer/array/object/compatibility/argument/return) | T06 |
| B4 | V3 semantic checks | M1 literal/binary/return/function-def only | SE01–SE11, SE16–SE19, SE21, SE25–SE29; effect classes | T07 |
| B5 | V3 layout/global initialization | `layouts`/`inits` reserved; no plan records | CL08–CL14, CL16–CL24, CL26; static data/relocations | T08 |
| B6 | V3 IR | `Constant`/`Return` only | IR01–IR07, IR10, IR14, IR18–IR19, IR23, IR28 | T09 |
| B7 | V3 verification/interpreter | VF12 M1 return-5 only; VF07–VF09 not M1 | VF01/VF05/VF06/VF07/VF08/VF12/VF13 extension | T13 |
| B8 | Target code generation/assembly | No `chips/target/` implementation | CG01–CG09, CG15–CG18, CG26–CG39 | T11 |
| B9 | Target probe/attestation | Harness never run; `target_verification=unverified`; substrate not provisioned | H01 run; H07 classification; T01 attest; C02 | H01/H07/T01 |
| B10 | Host driver/runner/isolation | H04 Part A M1 only; `-S`/`-c` refused; H05 absent | H04 V3 integration; H05 isolated runner; H01 toolchain | Host/H04/H05 |
| B11 | Reference oracle | Authorized, not available; no DejaGnu baseline | H07/pinned GCC reference; provenance record | H07 |
| B12 | Shared test entry/CI | Cargo auto-discovery exists; no V3 files | Integrator assignment and CI verification | Integrator |
| B13 | Uncommitted user work | `parse/mod.rs` modified; lex/parse drafts untracked | Preserve; reconcile in an authorized worktree/branch | Integrator/chip owners |
| B14 | Effect-mask classes | M1 only allows `EffectMask(0)`; nonzero is unsupported | Freeze V3 effect classes and diagnostics | T07/T01 |
| B15 | Layout-dependent Part A value oracle | No target layout; pointer/array value interpretation needs a model | Freeze symbolic/element-index interpreter model or defer values to Part B | T08/T13/T01 |
| B16 | Full corpus/denominator | Not frozen/fetched | T00 H00/H02/H03 | T00 |

No V3 test may bypass B1–B6 by embedding a private type/AST/IR; that is a contract defect, not progress.

---

## 14. Open questions

| ID | Question | Recommended default (not a decision) | Owner |
|---|---|---|---|
| Q1 | Does V3 ABI include aggregates/HFA/variadic/`long double`/`__int128`? | No; V3 is scalar/pointer/array-pointer; defer to Stage D/T12. | Integrator + T11/T12 |
| Q2 | Which language modes does V3 accept? C11 only, or `gnu11`/old-style compatibility? | C11 primary; old-style/no-prototype explicitly typed `Unsupported` until a mode freeze. | T01/T05/T07 |
| Q3 | What is the V3 effect-mask class inventory (load/store/call/volatile/atomic)? | Freeze minimal classes: memory read, memory write, call; volatile/atomic deferred. | T07/T01 |
| Q4 | How does Part A model array/pointer layout for `sizeof` and pointer arithmetic? | Structural type/IR only; concrete values Part B; interpreter uses element-index model with no byte claims. | T08/T13/T01 |
| Q5 | Which global initialization/linkage model is frozen: tentative, common/no-common, PIE/PIC, visibility? | C11 tentative + default external/internal; common extension off; PIE decision from H01. | T06/T08/T11 |
| Q6 | What exact task kinds, record fields, rule IDs, and diagnostic codes does V3 freeze? | Freeze with V3 contract change request; tests use exact codes, not "any failure". | T01 integrator |
| Q7 | Is a V3 IR interpreter model in scope for Part A values? | Yes, verification-only, typed element-index memory; no ABI claim; `Unsupported` is never pass. | T13 |
| Q8 | How are probe-gated tests represented and counted? | `#[ignore]` plus `--ignored`; ignored/skipped never counted as PASS; record `BLOCKED_PROBE`. | Integrator/H01 |
| Q9 | What is the host contract for `main(argc, argv)` and runner exit status? | H01/H05 define argv encoding and runner; tests use C11 guarantees only. | H01/H05 |
| Q10 | What recursion/stack limit behavior is expected? | Test bounded recursion only; stack exhaustion is not C-defined; report resource limit separately. | T11/T02 |
| Q11 | Are pointer↔integer casts, pointer provenance, and cross-object relational comparisons in V3? | No normative assertions; implementation-defined/unspecified cases excluded. | T06/T07 |
| Q12 | What reference GCC revision/flags are pinned for Part B? | H01/H07 freeze; reference-only, never candidate evidence. | H01/H07 |
| Q13 | Are global function-pointer tables and relocations in V3 or Stage D? | Keep basic function-pointer tables in V3; TLS/weak/advanced visibility deferred. | T08/T11 |
| Q14 | Is `restrict` accepted as a type with no optimization assumption? | Accept type only; no no-alias optimization in V3. | T06/T10 |
| Q15 | Does V3 include `_Bool`, `long`, unsigned, and `char` conversion cases? | Yes for conversion/ABI boundary, with positive values only where signedness is unverified. | T06/T07/T11 |
| Q16 | Is optimization in scope? | No; V3 correctness is `-O0`; optimization differential is later. | T10/T00 |

---

## 15. Risks and controls

| Risk | Control |
|---|---|
| Host ABI leaks into expected values (`sizeof(long)`, pointer width, `usize`) | Every concrete target assertion is `B`/`PROBE`; Part A uses symbolic types only; test review rejects host-derived literals. |
| Reference compiler used to compile the candidate | Mixed-link procedure separates candidate assembly/object from reference C; command log audit; no candidate `.c` in GCC invocation. |
| Golden snapshots become implementation-derived | No golden regeneration from candidate; structural assertions and hand-written expecteds; snapshot used for equality only. |
| Probe-gated tests counted as PASS when ignored | `#[ignore]`/`BLOCKED_PROBE` accounting; final report counts ignored separately; `--ignored` run fails closed without verified probe. |
| V3 tests coupled to M1 node indices | Locate nodes by kind/name/tree walk, as H04 does; never reuse fixed M1 indices. |
| Additive `NodeKind`/`TypeKind`/`IrOp` changes break c12–c39 | Integrator-owned contract change; rerun dependent slices; old expectations preserved or explicitly superseded. |
| Uncommitted lex/parse drafts overwritten | Worktree/branch isolation; only assigned files edited; no reset/overwrite. |
| Scope creep into V1/V2/Stage D | V3 boundary table; unsupported-feature cases; Q1/Q2/Q3 decisions. |
| Loose diagnostic oracle ("any failure passes") | Exact frozen group/code/rule required; otherwise `BLOCKED_CONTRACT`. |
| Part A interpreted value mistaken for target execution | Explicit non-claim; Part A/Part B columns; no ABI claim from VF12. |
| Layout-dependent pointer values asserted without a model | Q4; structural Part A; values Part B or frozen interpreter model. |
| Excessive test runtime (many Part B programs) | Bounded case count, shared harness, H05 timeouts; performance not correctness. |

---

## 16. Acceptance gates and reporting

### 16.1 Gate definitions

- **V3-A (Part A)**: every `A`/`A/B` structural case passes at `L0`–`L2`; all `B` value portions are explicitly `BLOCKED_PROBE`, not pass. This gate is target-independent and does not claim ABI correctness.
- **V3-B (Part B)**: every `B`/`A/B` value case passes at `L3`/`L4` with a verified probe, attested target, pinned toolchain, and H05 runner; mixed-link both directions; three stable runs.
- **V3 accepted**: V3-A and V3-B both pass; no required case is `Unsupported`, `SKIP`, `BLOCKED`, timeout, crash, or missing from the denominator.

### 16.2 Per-case statuses

`DESIGN` → `BLOCKED_CONTRACT` / `BLOCKED_PROBE` / `READY` → `PASS` / `FAIL` / `UNSUPPORTED` / `TIMEOUT` / `CRASH`.

Only `PASS` after an executed assertion counts as a pass. `UNSUPPORTED` is legal only for an out-of-V3 feature; for a required V3 feature it is `FAIL`. `BLOCKED_*`, `SKIP`, and `IGNORED` are not passes and must appear in the report with reasons.

### 16.3 Report fields

Future execution reports must include:

- contract version/hash, probe report hash/status, target identity;
- source/fixture paths and case IDs;
- exact command lines, environment, toolchain/GCC versions, sysroot, runner;
- pass/fail/blocked/unsupported/timeout counts;
- Part A and Part B results separately;
- known limitations and excluded UB/implementation-defined cases;
- raw evidence locations (`target/v3-evidence/...`) and first-divergence report.

No pass rate, ABI claim, or "V3 complete" statement may be inferred from this design document.

### 16.4 Executed evidence ledger for this task

| Item | Status |
|---|---|
| This report | DESIGN ONLY; no V3 test was run |
| V3 test files | NOT CREATED (proposed paths in §12.5) |
| V3 chips/schema | NOT IMPLEMENTED (checked `/39` baseline) |
| Target probe | NOT RUN; `target_verification=unverified` |
| Part A V3 chain | BLOCKED by contract and frontend/IR implementation |
| Part B single-TU | BLOCKED by probe, T11, H01/H04/H05 |
| Part B mixed-link | BLOCKED by probe, T11, H01/H05, reference oracle |
| Commands in §12 | NOT EXECUTED |

---

## 17. Summary

This report defines the independent V3 acceptance suite in three layers:

1. **Part A** — symbolic frontend/IR acceptance: calls/returns/params, globals/linkage/init plans, pointer/array types and decay, lvalue/argument/return conversions, effects, diagnostics, protocol, and replay.
2. **Part B** — probe-gated target acceptance: concrete layout, ABI argument/return/stack behavior, assembly/ELF/link/run, and two-way mixed-link.
3. **Protocol/negative/replay** — non-matching tasks, undeclared writes, manifest ownership, recovery, deterministic snapshots, and no-fabricated-success rules.

Nothing in V3 is implemented or run today. The immediate blockers are the V3 contract freeze (types/nodes/tasks/IR/effects/diagnostics), the V3 language/IR chips, the target probe, T11, and the H01/H04/H05/H07 toolchain/runner. Once frozen, the suite can grow through the I1–I7 ladder, with `V3-A` accepted independently of the probe and `V3-B` accepted only after the probe reports `verified=true` and the target is attested.
