# A11 — V1 Independent Acceptance Suite (Design)

| Field | Value |
|---|---|
| Task | A-TST-01 (Stage A) |
| Status | **DESIGN ONLY — NOT EXECUTED** |
| Date | 2026-11-07 |
| Owner | Independent tester (A-TST-01) |
| Master plan | [`docs/tasks/MASS_PARALLEL_DISPATCH.md`](../../tasks/MASS_PARALLEL_DISPATCH.md) §5 and §6.2 (V1) |
| V1 definition | Integer expressions, parentheses/precedence, local variables, declaration/initialization, assignment and conversions; PA → TY → SE → CL/IR → verification; real execution once the target is unlocked |
| Inspected frozen baseline | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24`; target identity frozen, concrete target values **UNVERIFIED** |
| Write scope | This report only. No source, test, contract, Cargo, `mod.rs`, index, or shared-file changes. |
| Evidence | No V1 test was authored or run. No candidate binary was run. No probe, assembler, linker, runner, or target execution occurred. All expected values below are hand-derived from C11 rules or arithmetic. |

> **Claim discipline.** This document is a test design. It is not an implementation, a
> contract freeze, a probe report, a pass rate, or evidence that any V1 chip exists.
> Every command in §6 is a **future** command unless explicitly marked otherwise.
> The only performed work was read-only inspection of the repository plus writing this
> report.

---

## 1. Scope, V1 Profile, and Acceptance Levels

### 1.1 What V1 must demonstrate

The V1 train is the first multi-feature vertical integration after the M1 constant
expression closed loop. It must connect, for real source input:

```text
source bytes → PP → LX → PA → TY → SE → CL/T08 → IR/T09 → verification
```

and, once the target is unlocked, continue through:

```text
IR → target code → external assembler → external linker → runner
```

The V1 feature list is deliberately narrow but not identical to M1. M1 proves one
folded `2+3` function. V1 must prove that the same pipeline carries **values that are
not compile-time constants** through local storage, initialization, assignment, and
back out through `return`.

### 1.2 V1-core profile proposed by this suite

The master plan names the feature families but does not freeze their exact grammar,
record shapes, or conversion matrix. To make acceptance testable, this suite proposes
the following **V1-core** profile. The integrator/T01 owner must accept or amend it
before these cases become normative (§7.5).

| Area | V1-core requirement | Notes |
|---|---|---|
| Language mode | Explicit `C11`; no implicit default dialect | Every fixture is compiled with `-std=c11` declared by the harness/config; no GNU extension |
| Integer types | `int` only, signed, symbolic in Part A | No concrete width/bit-pattern assertion without the probe; no `char`/`short`/`long`/`unsigned`/`_Bool` |
| Literals | Decimal integer literals that fit the symbolic `int` candidate | Existing T04 M1 lexical subset; no suffixes, no other radices in V1-core |
| Unary operators | Unary `+`, unary `-` | C11 6.5.3 |
| Binary operators | `+`, `-`, `*`, `/`, `%` | C11 6.5.5, 6.5.6; left-associative; usual arithmetic conversions on `int`/`int` only |
| Grouping | Parentheses for expression grouping | Parentheses are grouping only and need not be AST nodes |
| Primary expressions | Integer literal, local identifier | Identifier must resolve to an object in scope; no calls, no string literals |
| Local declarations | `int name;` and `int name = assignment-expression;` inside the `main` body | At least one declaration form; multiple declarators are a boundary case |
| Local scope | `main` body block scope; file scope for `main` | Nested blocks/shadowing are a boundary case |
| Statements | `return assignment-expression;` and `assignment-expression;` | Expression statements are needed for ordinary assignment; `return;` is invalid in `int main` |
| Assignment | Simple `=` only; right-associative; left operand must be a modifiable lvalue; result is the assigned value, not an lvalue | C11 6.5.16 |
| Conversions | Identity `int`→`int` for assignment, initialization, and return; lvalue-to-value conversion when reading an object | M1-minimal conversion scope; no narrowing, widening, sign-change, pointer, or float conversions |
| Determinism | Identical source/config/initial state yields identical record IDs, order, snapshot, and trace | T01 C05; no hash-map iteration or address-derived IDs |
| Failure | Invalid input is a typed diagnostic and a terminal task; unsupported input is explicit `Unsupported`/`Failed`; no panic, no silent success, no fabricated artifact | AGENTS.md §4.2; T01 §4; T05/T07/T09 DoD |

**Why this subset.** The V1 feature name "integer expressions" is broader than the
minimal arithmetic subset used here. This design chooses a small, fully hand-checkable
core so the first V1 acceptance is meaningful: literals, `+ - * / %`, unary `+`/`-`,
parentheses, `int` locals, assignment, and identity conversion. If the integrator defines
V1 to include bitwise/comparison/shift operators, `unsigned`, or wider/narrower integer
types, those rows must be added to the same matrix and will require the target
width/conversion schema and (for concrete bit patterns) the probe. They are not silently
omitted from V1 scope; they are explicit open scope in §7.5 Q1/Q2.

### 1.3 V1-boundary cases (must pass or be explicitly unsupported)

These are valid C11 or natural extensions of the named V1 scope. The implementation
may choose to include them in V1 or to reject them explicitly as out of profile; it
must not misparse them into a different successful program.

| Boundary | C11 basis | Suite treatment |
|---|---|---|
| Multiple declarators: `int a = 2, b = 3;` | 6.7p1 | Expected 6; if not in V1, explicit `Unsupported` |
| Nested block and shadowing: `int a=2; { int a=3; } return a;` | 6.2.1p4/p7 | Expected 2; if not in V1, explicit `Unsupported` |
| Expression initializer: `int a = 2*3+4;` | 6.7.9 | Expected 10; if initializer is literal-only, explicit `Unsupported` |
| Assignment chain: `a = b = 4;` | 6.5.16p1/p3 | Expected both 4, result 8; if not in V1, explicit `Unsupported` |
| Assignment expression as value: `return (a=5)+1;` | 6.5.16p3 | Expected 6; if not in V1, explicit `Unsupported` |
| Declaration after statement: `int a=1; a=2; int b=3;` | 6.8.2 | Profile decision; expected 5 if supported, explicit `Unsupported` otherwise |
| Uninitialized local never read: `int a; return 0;` | 6.2.4p6 (read is the UB-relevant act) | Expected 0; no read of `a` |
| Self-initialization `int a = a;` | 6.2.1p7 | **Scope-only** assertion: initializer `a` binds to the inner declaration; no value/execution claim (reading the indeterminate value is not a normative V1 test) |

### 1.4 V1 deferred / explicit unsupported

The following are outside V1-core. They must fail explicitly, never become a silently
different success. They are **not** expected to be accepted by this suite.

- `char`, `short`, `long`, `long long`, `unsigned`, `_Bool`, `float`, `double`, `long double`.
- Pointers, arrays, structs, unions, enums, typedefs, function pointers.
- Function calls, multiple functions, global variables, `static`/`extern`/`register`, qualifiers (`const`, `volatile`, `restrict`, `_Atomic`).
- `if`, `switch`, loops, `goto`, labels, `break`, `continue`, compound statements with control flow.
- `++`, `--`, compound assignment (`+=`, `-=`, ...), casts, `sizeof`, `_Alignof`, `_Generic`, compound literals.
- Variadic functions, K&R definitions, inline assembly, attributes.
- Optimizations other than `-O0`.
- Any program whose result depends on signed overflow, division by zero, uninitialized indeterminate values, unspecified evaluation order, or host ABI/width.

### 1.5 Acceptance levels

| Level | Meaning | What a pass requires |
|---|---|---|
| **A1 — pure unit** | Worker core/helper called with a synthetic narrow projection; no bus, no commit | Exact proposal/result shape; exact value/diagnostic; no writes; deterministic |
| **A2 — dispatch/commit** | One task through `RoutingShell`/`drive_task` and the frozen commit path | Exactly one terminal transition; declared writes only; undeclared state byte-identical; diagnostic attached correctly |
| **A3 — producer/consumer integration** | One bus, real upstream records from PP→LX→PA→TY→SE→CL→IR; synthetic input only where a real producer does not yet exist, and then explicitly marked | Real record handoff (same IDs, same values); no synthetic semantic facts on the accepted path; expected final semantic value/records |
| **A4 — pipeline determinism** | Full V1 chain run at least twice from identical initial state/config/input | Identical snapshot bytes and trace bytes; bounded ticks; negative fixture also deterministic |
| **A5A — Part A closed loop** | V1 IR interpreter (target-independent, symbolic) | Expected mathematical return value; unsupported never modeled as a value; interpreter result is not a substitute for Part B |
| **A5B — Part B closed loop** | Candidate-generated target assembly/object → external assembler/linker → runner | Exact alive process exit status (0..=127 fixtures); ELF/AArch64 structural facts; probe-confirmed ABI values where asserted |

A test may be listed at multiple levels. A lower-level pass must not be reported as a
higher-level pass. In particular, an A1 chip test is not an integration pass, and an
A5A interpreter result is not target execution evidence.

### 1.6 Independence and anti-cheating rules

1. **Expected values are not computed by the implementation under test.** Test support
   tables contain hand-written constants and hand-derived AST/record shapes. A test must
   not call `parse_binary_expression`, a semantic chip, a constant evaluator, or an IR
   lowerer to derive its own expectation.
2. **No external compiler oracle.** GCC/Clang may not compile the candidate fixture for
   an expected value, and reference output must not substitute for the fixed target.
   The C11 clause and hand arithmetic are the oracle.
3. **No success by silence.** Unsupported or invalid input must produce an explicit typed
   failure and no accepted artifact. A missing diagnostic is a failure of the test, not a
   pass.
4. **No implementation-specific over-specification.** Where C11 permits multiple legal
   evaluations or lowerings, the acceptance assertion is the observable semantic value
   plus required invariants, not a particular instruction sequence. Exact IR records are
   asserted only after the V1 IR shape is frozen (§7.1).
5. **No filename/source special-casing.** Each fixture is a distinct semantic case; the
   implementation must not branch on the fixture name, spelling, or line number.
6. **No fake integration.** Synthetic upstream records may substitute only for a missing
   upstream producer at A1/A2; A3 must use the real producer output when it exists.
7. **Unrun is reported as unrun.** A design row is not a test result.
8. **C11 citations are informal.** No ISO C standard text is vendored in this
   repository; clause numbers identify the rule used for the hand derivation, not a
   quoted normative text. If a clause citation is disputed, the expected value is
   re-derived and the table is corrected before the case becomes normative.

---

## 2. Current Baseline and V1 Gap

### 2.1 Frozen and implemented baseline inspected

The current frozen artifact is `t01-c01-c06/39`. It includes the T01 foundation and the
implemented M1/PP/parse/semantic/IR slices through `c39_sefunc.rs`. Relevant existing
coverage:

| Area | Existing files / tests | V1 relevance |
|---|---|---|
| M1 constant chain | `c14_se.rs`, `c15_ir.rs`, `c17_vf12.rs` | `2+3` → folded 5 → IR `Constant`/`Return`; baseline for E01 |
| Parse declaration shapes | `c12_parse.rs`, `c34_parse.rs` | `int main(void)` and a fixed `{ return 2+3; }`; **not** general local declarations |
| Parse expression shapes | `c35_expr.rs` | exactly `2`, `2+3`, `+3`, `-3`, and a 4-token boundary; **no** general precedence, `*`, `/`, `%`, identifiers, parentheses, assignment |
| Parse recovery | `c36_recovery.rs` | delimiter sync and finite advance; reusable mechanism, but the V1 productions are absent |
| Type/scope/symbol | `c13_ty.rs` | canonical `int`, `int(void)`, file scope, `main` declaration; **no** object/block-scope declaration |
| Semantic checks | `c14_se.rs`, `c39_sefunc.rs` | literals, `2+3`, return identity, function definition; **no** identifier/lvalue/assignment/declaration |
| Constants | Gate 1 fold + `c37_const_branch.rs` | literal/`Add` fold and branch selection; **no** `-`, `*`, `/`, `%`, unary |
| IR | `c15_ir.rs` | `Constant` + `Return` only; **no** locals, assignment, arithmetic |
| Verification | `c17_vf12.rs`, `c18_vf05.rs`, `c19_vf01.rs`, `c38_vf14.rs` | M1 symbolic interpreter/store/token/AST invariants; V1 extensions absent |
| Candidate driver | `compiler/tests/h04_candidate.rs` | Part A evidence flags work for the M1 pipeline; no V1 pipeline wiring |
| Snapshot/trace | `c05_codec.rs`, per-slice replay tests | Canonical snapshot/trace machinery exists; new V1 records/kinds need encoding and hash participation |

### 2.2 What V1 needs that the frozen baseline does not have

This is a **readiness** statement, not a defect claim. The frozen baseline is doing
exactly what its slice documents say.

| V1 need | Current fact | Consequence for this suite |
|---|---|---|
| V1 task kinds/stages | Only the kinds in `TaskKindRegistry::se_function_slice()`; no local declaration, initializer, assignment, general arithmetic, or V1 IR kinds | Most A2/A3 cases are `BLOCKED-SCHEMA` |
| AST node kinds | `NodeKind` is the closed M1 set (`TranslationUnit`, `FunctionDefinition`, `Specifiers`, `Declarator`, `Compound`, `Return`, `BinaryAdd`, `IntLiteral`) | No identifier expression, assignment, general arithmetic, declaration/initializer node; parser expectations blocked |
| Local symbol/type support | `TySymbolChip` accepts exactly `[Declarator Node, Function Type, File Scope]`, rejects non-function types and block scopes | No local `Object` symbol, no `Automatic` storage, no block-scope declaration; A3 local cases blocked |
| Semantic facts | `SemRecord { node, ty, category, effects }` in the frozen slice; no `conversions` field | No recorded assignment/initializer/return conversion plan; identity is currently represented by absence |
| Constant evaluator | `ConstExprOp` has only `Add`; `ConstantRequest` is literal/binary-add across committed literals | No V1 `-`, `*`, `/`, `%`, unary constant evaluation |
| IR | `IrOp` has only `Constant` and `Return`; `FunctionRecord` has no local/frame representation | No local initialization, load, store, assignment, or arithmetic IR; A3/IR expectations blocked |
| Snapshot/hash | Current encoders cover the frozen M1 record set; V1 kinds/records are not encoded | New V1 records must be added to snapshot/hash/version before A4 can bind |
| V1 verifier | VF12 models the M1 `Constant`/`Return` shape only | No V1 interpreter or V1 typed-IR invariant |
| Candidate driver | `candidate` installs the M1 pipeline | V1 source cannot reach a V1 evidence run without H04 integration |
| Target | `target_verification=unverified`; `ensure_codegen_ready` fails closed; no assembler/linker/runner substrate | All A5B cases are `BLOCKED-PROBE` / `BLOCKED-HOST` |

### 2.3 Uncommitted work at design time

`git status` at inspection time showed:

- modified: `compiler/src/chips/parse/mod.rs`;
- untracked candidate parse files: `pa_cast.rs`, `pa_decl_spec.rs`, `pa_expr_ext.rs`,
  `pa_ext_dispatch.rs`, `pa_init.rs`, `pa_stmt.rs`, `pa_typedef_query.rs`;
- untracked dispatch/review and freeze drafts elsewhere.

These files are **not** a frozen contract, are not registered, and include candidate
IDs (several claim `ChipId(58)` / `PARSE` local 27) that the integrator has not
arbitrated. Other concurrent uncommitted files (candidate-driver edits, chip-lint test
edits, new candidate tests, freeze drafts) may also be present in the shared workspace;
they are not V1 frozen evidence and were not edited here. This suite must not use any of
them as an oracle, must not assume their shapes, and must not edit or overwrite them. If
they land in a later freeze, the V1 suite is re-baselined against that freeze rather than
against the drafts.

---

## 3. Test Architecture and Fixture Ownership

### 3.1 Layered architecture

```text
A1 pure unit
  └─ narrow synthetic projection → compute → proposal/result
A2 dispatch/commit
  └─ one task → routing → worker → commit → task terminal + store delta
A3 producer/consumer integration
  └─ real PP→LX→PA→TY→SE→CL→IR chain on one bus
A4 pipeline determinism
  └─ full chain repeated; snapshot bytes + trace bytes equality
A5A Part A closed loop
  └─ V1 IR interpreter returns the expected mathematical value
A5B Part B closed loop
     └─ candidate assembly/object → external tools → process exit status
```

Each layer has a different failure meaning. A failure at A1 is not automatically an
integration defect; a green A1 is not evidence that the chain works. Reports must state
the highest level actually executed.

### 3.2 Proposed future test paths

No V1 test file exists today. The suite will live entirely under the compiler package's
auto-discovered integration-test tree. Cargo auto-discovers `compiler/tests/*.rs`;
`compiler/tests/v1_support/` is a non-target support directory. The integrator must
confirm the entry points before the first run; the test author must not edit
`compiler/Cargo.toml`.

| Path | Purpose | Owner |
|---|---|---|
| `compiler/tests/v1_support/mod.rs` | Bus construction, V1 installation, tick driver, snapshot/trace helpers, diagnostic assertions | Independent V1 test owner |
| `compiler/tests/v1_support/expected.rs` | Hand-written expected values, AST shapes, variable states, exit statuses | Independent V1 test owner |
| `compiler/tests/v1_support/sources.rs` | Exact fixture byte access (`include_bytes!`) and fixture manifest | Independent V1 test owner |
| `compiler/tests/v1_expr.rs` | V1-EXPR cases (A1/A3/A5A where available) | Independent V1 test owner |
| `compiler/tests/v1_locals_decl.rs` | V1-LOC cases | Independent V1 test owner |
| `compiler/tests/v1_assignment_conversion.rs` | V1-ASN cases | Independent V1 test owner |
| `compiler/tests/v1_negative.rs` | V1-NEG invalid cases | Independent V1 test owner |
| `compiler/tests/v1_unsupported_recovery.rs` | V1-UNS/V1-REC cases | Independent V1 test owner |
| `compiler/tests/v1_integration.rs` | V1-INT real-chain cases | Independent V1 test owner |
| `compiler/tests/v1_replay.rs` | V1-REP snapshot/trace cases | Independent V1 test owner |
| `compiler/tests/v1_write_scope.rs` | V1-WS non-matching-task and undeclared-write cases | Independent V1 test owner |
| `compiler/tests/v1_closed_loop.rs` | V1-CL Part A and Part B cases; Part B tests `#[ignore]` until the probe/host gate opens | Independent V1 test owner |
| `compiler/tests/fixtures/v1/*.c` | Immutable byte-exact source fixtures | Independent V1 test owner |

The fixture set is deliberately one file per semantic case, not one large program. This
keeps the first failing rule attributable and prevents one unsupported construct from
masking all later V1 behavior.

### 3.3 Fixture ownership and change control

| Artifact | Owner | Rules |
|---|---|---|
| Fixture sources under `compiler/tests/fixtures/v1/` | Independent V1 test owner | ASCII, LF-terminated, no BOM, no trailing spaces; exact bytes listed in the fixture manifest; a change requires updating the expected table in the same change |
| Fixture naming and bytes | Independent V1 test owner | `v1_<group>_<NN>.c` (for example `v1_expr_01.c`, `v1_loc_01.c`, `v1_asn_01.c`). Unless the matrix states otherwise, the exact bytes are `int main(void){` + the body shown in §4 + `}` + `\n`, with no added spaces. For the explicitly unterminated fixtures (NEG-12/REC-05) the file ends at the stated point and omits the final `}`. Where the matrix gives a complete program (for example UNS-09) that text is the exact fixture. The `sources.rs` manifest binds each file with `include_bytes!`; tests must not build fixtures by runtime concatenation. |
| Expected tables under `v1_support/expected.rs` | Independent V1 test owner | Hand-derived only; no implementation call may populate them; a chip owner may not edit them to match a defect |
| V1 test files under `compiler/tests/v1_*.rs` | Independent V1 test owner | May use public chip/bus APIs; must not edit chip files, registries, schemas, or shared helpers |
| V1 chip implementations | T05/T06/T07/T08/T09/T13 owners | Not edited by the test owner; interface gaps become contract-change requests, not private test fixtures |
| Shared registries/schemas/snapshot/hash | T01 integrator | New kinds, records, stage rows, allowlist rows, snapshot encoders, and version/hash updates are integrator-owned |
| Candidate driver | H04 owner | V1 pipeline wiring is a host task; test owner invokes the binary only |
| Target probe/assembler/linker/runner | H01/H05/H07 owners | The test owner consumes frozen commands; no invented ABI values or tool names |

### 3.4 Harness responsibilities (mechanical only)

The shared support module must contain only mechanical orchestration:

- `new_v1_bus()`: `CompilerConfig::new(TargetSpec::aarch64_unknown_linux_gnu_unverified(), Dialect::C11, OptLevel::O0, vec![], Limits::fixture())`.
- `install_v1(bus)`: register the **frozen** V1 kind registry/schema/routes/workers for the current contract; fail loudly if the contract version is not the expected one.
- `run_until_terminal(bus, workers)`: tick with `RoutingShell::clock_tick_with` and a bounded tick budget; return the bus, `Trace`, and per-tick reports.
- `snapshot(bus)`: `Snapshot::capture(bus)`; `trace_bytes(trace)`: `Trace::to_bytes()`.
- `diagnostic_groups`, `task_state`, `store_allocated`, `record_values`: read-only accessors.
- No language rule may live in the support module. If a helper starts deciding precedence, scope, or conversion behavior, the test is hiding the implementation and must be redesigned.

### 3.5 Oracle and fixture discipline

- Each expected value table entry cites the C11 clause used and shows the hand
  calculation in a comment or adjacent table.
- AST expectations are written as hand-built expected trees/shapes, not produced by the
  parser. Where the final `NodeKind` names are not frozen, the expectation is expressed
  as a semantic shape (operator, operands, grouping) until the freeze names it.
- Record expectations are written as counts/fields/relative ordering, not addresses or
  allocator internals. Exact numeric IDs are asserted only where the frozen contract
  makes dense allocation order normative, and even then the expected ID is derived from
  production order, not read back from a similar run.
- Negative tests assert the diagnostic category/location and the absence of an accepted
  artifact, not message text or numeric codes that are still unfrozen.
- A test that must use a synthetic upstream record carries a comment naming the missing
  producer and the level at which it is acceptable (A1/A2 only). It is not counted as A3.

---

## 4. Case Matrix

Legend for **Readiness**:

- `BASELINE-M1` — the exact semantic case is already implemented by the frozen M1 slice,
  but the V1 suite still has to author/run it.
- `BLOCKED-SCHEMA` — cannot be bound or implemented until the named T01/T05/T06/T07/T08/T09/T13
  schema/kind/record/conversion/IR freeze lands (see §7.1).
- `BLOCKED-PROBE` — the semantic value is target-independent, but the asserted concrete
  width/bit-pattern/ABI/codegen/exit status needs a verified target probe.
- `BLOCKED-HOST` — needs the H04 driver wiring, H05 isolation/runner, or the external
  assembler/linker substrate.
- `DESIGN-ONLY` — the expected value is fixed by this design, but the test cannot be
  authored against a frozen interface yet.

A row may have more than one readiness tag; the suite must report the highest level
actually reached.

### 4.1 Integer expressions, parentheses, and precedence

Inputs are the body of `int main(void){ ... }`; the surrounding function is fixed and
byte-exact. The expected result is the value returned by `main` (and, where shown, the
AST shape before constant folding).

| ID | Fixture / expression | Expected result (pass criterion) | Basis / oracle | Level | Readiness |
|---|---|---|---|---|---|
| V1-EXPR-01 | `return 2+3;` | `5`; AST `Add(IntLiteral 2, IntLiteral 3)` | C11 6.5.6; hand arithmetic | A1/A3/A5A | `BASELINE-M1` (value/interpreter); A3 real-source PP wiring still pending; V1 suite still to author |
| V1-EXPR-02 | `return 2+3*4;` | `14`; AST `Add(2, Mul(3,4))` | C11 6.5.5 higher precedence than 6.5.6; hand | A1/A3/A5A | `BLOCKED-SCHEMA` (general `Mul`) |
| V1-EXPR-03 | `return (2+3)*4;` | `20`; AST `Mul(Add(2,3),4)`; parentheses disappear as grouping | C11 6.5.1/6.5.5/6.5.6; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-04 | `return 2*(3+4);` | `14`; AST `Mul(2,Add(3,4))` | C11 6.5.1; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-05 | `return 10-3-2;` | `5`; AST `Sub(Sub(10,3),2)` (left-associative) | C11 6.5.6 grammar/left grouping; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-06 | `return 2*3+4*5;` | `26`; AST `Add(Mul(2,3),Mul(4,5))` | C11 6.5.5/6.5.6; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-07 | `return 100/7;` | `14`; quotient truncates toward zero | C11 6.5.5p5–p6; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-08 | `return 100%7;` | `2`; `(100/7)*7 + 100%7 == 100` | C11 6.5.5p5–p6; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-09 | `return -7/2;` | `-3`; unary minus binds first, division truncates toward zero | C11 6.5.3/6.5.5p6; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-10 | `return -7%2;` | `-1`; `(-7/2)*2 + (-7%2) == -7` | C11 6.5.5p6; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-11 | `return +2*3;` | `6`; unary plus is a no-op on `int`; `Mul` binds after unary | C11 6.5.3/6.5.5; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-12 | `return -(2+3)*4;` | `-20`; unary applies to the parenthesized sum, then multiplication | C11 6.5.1/6.5.3/6.5.5; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-13 | `return 2+3-4+5;` | `6`; additive operators group left-to-right | C11 6.5.6 grammar; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-14 | `return 2*(3*(4+1));` | `30`; nested parentheses and multiplication | C11 6.5.1/6.5.5; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-15 | `return ((2));` | `2`; redundant grouping does not change the value | C11 6.5.1; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-16 | `return 2+ +3;` | `5`; the two `+` characters are separate tokens; the second is unary plus | C11 6.4 lexical separation; 6.5.3/6.5.6; hand | A1/A3/A5A | `BLOCKED-SCHEMA` (PA16 unary composition) |
| V1-EXPR-17 | `return 0*7;` | `0`; zero boundary | C11 6.5.5; hand | A1/A3/A5A | `BLOCKED-SCHEMA` |
| V1-EXPR-18 | `return 2147483647+1;` | **No normative value.** Signed overflow is undefined (or policy-defined); must not crash or fabricate a value; report as non-normative. | C11 6.5p5 (undefined on exceptional result); policy open | A1/A3 | `DESIGN-ONLY` (non-normative; excluded from pass count) |

**AST-shape oracle.** The expected trees above come from the C11 grammar and
precedence/associativity rules, not from the parser. Parentheses are grouping and do not
require an AST node. If the frozen V1 `NodeKind` set uses different names, the table is
translated to those names without changing the tree.

**Constant folding is not over-specified.** A constant-only expression may be folded by
T08/CL (as M1 does for `2+3`) or lowered as arithmetic if the frozen V1 IR chooses that
form. The acceptance assertion is the semantic value at A5A/A5B plus, if a `ConstRecord`
is produced, the rule that T09 consumes the same `ConstId` and does not refold. Exact
record counts are asserted only after the V1 IR shape is frozen (§7.1).

### 4.2 Local variables, declarations, and initialization

| ID | Fixture / body | Expected result (pass criterion) | Basis / oracle | Level | Readiness |
|---|---|---|---|---|---|
| V1-LOC-01 | `int a; a=2; return a;` | `2`; one `Object` symbol `a` in the `main` block, type `int`, storage `Automatic`, linkage `None`; declaration before first read | C11 6.7, 6.5.16, 6.8.2; hand | A2/A3/A5A | `BLOCKED-SCHEMA` (local symbol/IR) |
| V1-LOC-02 | `int a=4; return a;` | `4`; initializer runs once before the read; symbol facts as above | C11 6.7.9; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-LOC-03 | `int a=2; int b=3; return a+b*4;` | `14`; two block-scope `Object` symbols; `a` and `b` initialized before use | C11 6.7/6.7.9/6.5.5/6.5.6; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-LOC-04 | `int a; int b; a=2; b=3; return a*10+b;` | `23`; assignments update the declared objects; reads see the new values | C11 6.5.16; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-LOC-05 | `int a=2, b=3; return a*b;` | `6`; multiple declarators in one declaration | C11 6.7p1/6.7.9; hand | A1/A2/A3/A5A | `BLOCKED-SCHEMA`; boundary decision |
| V1-LOC-06 | `int a=2; { int a=3; } return a;` | `2`; inner `a` shadows only inside the nested block; outer `a` is unchanged after the block | C11 6.2.1p4/p7, 6.2.4; hand | A2/A3/A5A | `BLOCKED-SCHEMA`; boundary decision |
| V1-LOC-07 | `int a; return 0;` | `0`; declaration is valid; `a` is never read, so no indeterminate-value claim | C11 6.7, 6.2.4p6 (read is the relevant act); hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-LOC-08 | `int a=5; a=a+1; return a;` | `6`; read-modify-write through the local | C11 6.5.16; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-LOC-09 | `int a=2; int b=a+3; return b;` | `5`; `b`'s initializer sees the already-declared `a` | C11 6.2.1p7/6.7.9; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-LOC-10 | `int a=2*3+4; return a;` | `10`; expression initializer | C11 6.7.9/6.5.5/6.5.6; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-LOC-11 | `int a=(2+3)*4; return a;` | `20`; parenthesized expression initializer | C11 6.7.9/6.5.1; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-LOC-12 | `int a=1; { int b=a+1; return b; }` | `2`; inner block sees the outer `a`; return from the nested block | C11 6.2.1p4, 6.8.6.4; hand | A2/A3/A5A | `BLOCKED-SCHEMA`; boundary decision |
| V1-LOC-13 | `int a=2; { int b=3; } return a;` | `2`; inner `b` is not visible after its block | C11 6.2.1p4; hand | A2/A3 | `BLOCKED-SCHEMA`; boundary decision |
| V1-LOC-14 | `int a = a; return 0;` | **Scope-only:** the initializer `a` resolves to the just-declared inner `a`, not an outer/undeclared name. Do not claim a value; reading the indeterminate value is not a V1 pass case. | C11 6.2.1p7 (point of declaration after the declarator); value is indeterminate/UB-relevant | A1/A2 | `BLOCKED-SCHEMA` + `DESIGN-ONLY` (non-normative value) |
| V1-LOC-15 | `int a=1; a=2; int b=3; return a+b;` | `5` if mixed declarations/statements are in the V1 profile; otherwise explicit `Unsupported` | C11 6.8.2 block-item order; profile decision | A1/A2/A3/A5A | `PROFILE-DECISION` (§7.5 Q3) |

**Semantic facts to assert for local cases.**

- The local symbol is an ordinary-namespace `Object`, not a `Function`; `ty = int`;
  `storage = Automatic`; `linkage = None`; `scope` is the `main` body block; `decl` is
  the identifier leaf (exact field/encoding after T06 freeze).
- The symbol is visible only after the complete declarator (C11 6.2.1p7). The
  initializer of its own declaration is in scope (LOC-14), while an outer use before the
  declaration is not (NEG-01/NEG-07).
- The local has exactly one initialization execution per declaration occurrence; it is
  not re-evaluated on each read.
- A read of an initialized local observes the most recent assignment/initialization;
  V1 must not exhibit stale or duplicated evaluation.
- The enclosing function's symbol remains in file scope; local scopes nest under the
  file scope, not under each other's closed siblings.
- No global object symbol is created for a local.

### 4.3 Assignment and conversions

| ID | Fixture / body | Expected result (pass criterion) | Basis / oracle | Level | Readiness |
|---|---|---|---|---|---|
| V1-ASN-01 | `int a; a=7; return a;` | `7`; assignment stores once; later read sees `7` | C11 6.5.16; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-02 | `int a; int b; a=b=4; return a+b;` | `8`; `a=b=4` parses `a=(b=4)` (right-associative); both variables end at `4`; assignment result is `4` | C11 6.5.16p1/p3; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-03 | `int a; return (a=5)+1;` | `6`; assignment expression has the assigned value; `+1` sees `5` | C11 6.5.16p3; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-04 | `int a=1; int b=2; a=b; return a*10+b;` | `22`; identity assignment copies `b` to `a`; `b` remains `2` | C11 6.5.16.1/6.3.1.3; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-05 | `int a=7; int b; b=a; return b;` | `7`; `int`→`int` assignment is identity; no narrowing/widening | C11 6.5.16.1/6.3.1.3; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-06 | `return 5;` | `5`; return expression `int`→`int` identity | C11 6.8.6.4/6.3.1.3; hand | A1/A2/A5A/A5B | `BLOCKED-SCHEMA` (single-literal return form; M1 parser accepts only the fixed `2+3` block shape) |
| V1-ASN-07 | `int a=5; return a;` | `5`; lvalue conversion of `a` to its value, then identity return | C11 6.3.2.1p2/6.8.6.4; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-08 | `int a=5; return a+0;` | `5`; `int`+`int` usual arithmetic conversion yields `int`; no inserted cast changes the value | C11 6.3.1.8; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-09 | `int a=5; return a=6;` | `6`; the assignment expression is `int`, non-lvalue; return consumes its value | C11 6.5.16p3/6.8.6.4; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-10 | `int a=1; int b=2; int c=3; a=b=c=2; return a*10+b;` | `22`; three-level right-associative chain; all three end at `2`; result of each assignment is the assigned value | C11 6.5.16p1/p3; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-11 | `int a; a=2+3*4; return a;` | `14`; assignment RHS uses precedence; store happens after RHS evaluation | C11 6.5.5/6.5.6/6.5.16; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |
| V1-ASN-12 | `int a=5; int b=6; a=b; b=7; return a*10+b;` | `67`; assignment copies the value (`a=6`), later `b=7` does not change `a`; `a*10+b = 67` | C11 6.5.16p2/p3; hand | A2/A3/A5A | `BLOCKED-SCHEMA` |

**Conversion facts to assert.**

- `int`→`int` assignment, initialization, and return are identity conversions. If the
  frozen V1 `SemRecord` records a `ConversionPlan`, the expected entry is
  `(Identity, Assignment/Result/Return, int, int)` per the frozen role vocabulary; if
  the frozen contract keeps M1's "identity = no plan", the expected vector is empty.
  The test must use the frozen representation, not invent one.
- Reading an object applies lvalue conversion (C11 6.3.2.1p2) except where the grammar
  requires an lvalue (assignment left operand).
- The assignment expression is **not an lvalue** (C11 6.5.16p3). This is the basis for
  NEG-04.
- No implicit narrowing, sign extension, or target-width wrap is asserted in Part A;
  any such assertion is probe-gated Part B.

### 4.4 Invalid programs

Every invalid case must produce a typed diagnostic, a terminal task, no accepted IR or
artifact, and no panic/loop. The exact diagnostic group/code is frozen later; the table
states the C11 basis and the required category/absence.

| ID | Fixture / body | Expected result (pass criterion) | Basis / oracle | Level | Readiness |
|---|---|---|---|---|---|
| V1-NEG-01 | `return x;` | One undeclared-identifier diagnostic at `x`; no symbol, no IR function, no artifact | C11 6.5.1p2 (constraint: identifier must be declared); hand | A1/A2/A3 | `BLOCKED-SCHEMA` (semantic identifier path) |
| V1-NEG-02 | `return x+1;` | One undeclared-identifier diagnostic at `x`; `+1` does not fabricate `x=0` | C11 6.5.1p2; hand | A1/A2/A3 | `BLOCKED-SCHEMA` |
| V1-NEG-03 | `return 2=3;` | One constraint diagnostic: left operand of `=` is not a modifiable lvalue; no IR/artifact | C11 6.5.16p2; hand | A1/A2 | `BLOCKED-SCHEMA` |
| V1-NEG-04 | `int a=1; return (a=5)=6;` | One constraint diagnostic: the result of an assignment is not an lvalue; no IR/artifact | C11 6.5.16p3; hand | A1/A2 | `BLOCKED-SCHEMA` |
| V1-NEG-05 | `int a=1; int a=2; return a;` | One redeclaration/conflict diagnostic for the second `a` in the same block scope; the first declaration remains, but no accepted artifact | C11 6.7p3 (no linkage: at most one declaration in a scope/name space); hand | A1/A2/A3 | `BLOCKED-SCHEMA` (local symbol path) |
| V1-NEG-06 | `return;` | One constraint diagnostic: `return` with no expression in an `int` function; no IR/artifact | C11 6.8.6.4p1; hand | A1/A2 | `BLOCKED-SCHEMA` (return form) |
| V1-NEG-07 | `int a=1; return b;` | One undeclared-identifier diagnostic at `b`; no fallback to `0` | C11 6.5.1p2; hand | A1/A2/A3 | `BLOCKED-SCHEMA` |
| V1-NEG-08 | `int a=; return a;` | One syntax diagnostic at the missing initializer expression; recovery to `;`; no accepted artifact | C11 6.7.9 (initializer requires an assignment expression); grammar | A1/A2 | `BLOCKED-SCHEMA` (declaration/initializer parser) |
| V1-NEG-09 | `int a=1 return a;` | One expected-`;` diagnostic before `return`; finite recovery; no accepted artifact | C11 6.7/6.8.2 grammar; hand | A1/A2 | `BLOCKED-SCHEMA` |
| V1-NEG-10 | `int a=(2+3; return a;` | One expected-`)` diagnostic; recovery to `;`; no accepted artifact | C11 6.5.1 parenthesized primary grammar | A1/A2 | `BLOCKED-SCHEMA` |
| V1-NEG-11 | `return 2 3;` | One expected-operator-or-`;` diagnostic at the second `3`; recovery at `;`; no accepted artifact | C11 6.5 grammar; hand | A1/A2 | `BLOCKED-SCHEMA` |
| V1-NEG-12 | `int a=1; return a;` with missing final `}` | One unterminated-compound diagnostic at EOF; finite cleanup; no accepted artifact | C11 6.8.2 compound-statement grammar | A1/A2 | `BLOCKED-SCHEMA` |
| V1-NEG-13 | `return 5/0;` | **Non-normative.** Division by zero is undefined (C11 6.5.5p5); V1 may reject it as an invalid constant expression or leave it to runtime, but must not crash or fabricate a value. Not counted as a pass. | C11 6.5.5p5; policy open | A1/A3 | `DESIGN-ONLY` (non-normative) |

For V1-NEG-01/02/05/07, the expected diagnostic group is `Semantic` once the frozen
taxonomy exposes that group for the V1 path. For V1-NEG-03/04/06, the expected group is
also semantic/constraint. For syntax cases (NEG-08..12), the exact parse/syntax diagnostic
group and numeric code are unfrozen; the test asserts "one primary diagnostic, correct
location, no success" and binds the group after the freeze. It does not assert message
text.

### 4.5 Unsupported features and recovery

Unsupported rows are outside the V1 profile. They must fail explicitly with
`DiagGroup::Unsupported` (or the frozen explicit-unsupported result), never be accepted
as a different program, and never produce a partial accepted artifact.

| ID | Fixture / body | Expected result (pass criterion) | Basis / oracle | Level | Readiness |
|---|---|---|---|---|---|
| V1-UNS-01 | `return 2+3.0;` | Explicit `Unsupported`; no double value, no coercion to `int` | V1 profile excludes floating types; C11 conversion is well-defined but out of profile | A1/A2/A3 | `BLOCKED-SCHEMA` (V1 reject path) |
| V1-UNS-02 | `int *p; return 0;` | Explicit `Unsupported` for the pointer declarator | V1 profile excludes pointers | A1/A2 | `BLOCKED-SCHEMA` |
| V1-UNS-03 | `int a[2]; return 0;` | Explicit `Unsupported` for the array declarator | V1 profile excludes arrays | A1/A2 | `BLOCKED-SCHEMA` |
| V1-UNS-04 | `return f(3);` | Explicit `Unsupported` for the call expression | V1 profile excludes calls | A1/A2 | `BLOCKED-SCHEMA` |
| V1-UNS-05 | `if (1) return 1; return 0;` | Explicit `Unsupported` for control flow | V1 profile excludes `if`/loops | A1/A2 | `BLOCKED-SCHEMA` |
| V1-UNS-06 | `int a=1; a++; return a;` | Explicit `Unsupported` for increment | V1 profile excludes `++`/`--` | A1/A2 | `BLOCKED-SCHEMA` |
| V1-UNS-07 | `int a=1; a+=2; return a;` | Explicit `Unsupported` for compound assignment | V1 profile excludes compound assignment | A1/A2 | `BLOCKED-SCHEMA` |
| V1-UNS-08 | `return (int)3;` | Explicit `Unsupported` for the cast; `sizeof(int)` is the same unsupported class and can be a sibling fixture | V1 profile excludes casts and `sizeof` | A1/A2 | `BLOCKED-SCHEMA` |
| V1-UNS-09 | `int g; int main(void){return 0;}` | Explicit `Unsupported` for the global object; a second function is the same unsupported class | V1 is a single-`main` local-variable slice | A1/A2/A3 | `BLOCKED-SCHEMA` |
| V1-UNS-10 | `const int a=1; return a;` | Explicit `Unsupported` for the qualifier; `static int a;` / `register int a;` are the same unsupported storage-class class | V1 profile is plain automatic `int` locals | A1/A2 | `BLOCKED-SCHEMA` |
| V1-REC-01 | `int a=; int b=2; return b;` | Exactly one primary syntax diagnostic at the first `=`; recovery resumes at the next `;` and sees `int b=2;` as the next construct; TU still fails; no accepted IR/artifact; finite ticks | C11 grammar + recovery contract; hand | A1/A2 | `BLOCKED-SCHEMA` |
| V1-REC-02 | `int a=1; a = ; return a;` | Exactly one primary diagnostic at the missing assignment RHS; recovery at `;`; no accepted artifact | C11 6.5.16/6.8.2 grammar | A1/A2 | `BLOCKED-SCHEMA` |
| V1-REC-03 | `int a=(2+3; return a;` | Exactly one expected-`)` diagnostic; recovery does not swallow the following `return`/`;`; no accepted artifact | C11 6.5.1 grammar | A1/A2 | `BLOCKED-SCHEMA` |
| V1-REC-04 | `return 2 3;` | Exactly one expected-operator-or-`;` diagnostic; recovery at `;`; no accepted artifact | C11 6.5 grammar | A1/A2 | `BLOCKED-SCHEMA` |
| V1-REC-05 | `int a=1; return a;` (missing `}`) | Exactly one unterminated-compound diagnostic at EOF; cleanup once; no accepted artifact; bounded ticks | C11 6.8.2 grammar | A1/A2 | `BLOCKED-SCHEMA` |
| V1-REC-06 | V1-REC-01 fixture (malformed initializer followed by a valid declaration/`return` in the same block) | Recovery is finite (bounded ticks), no panic, exactly one terminal failure per task; the parser may continue scanning but must not produce a successful TU/IR | T01 finite-progress contract; T05 recovery | A1/A2 | `BLOCKED-SCHEMA` |

Recovery pass criteria are stricter than "does not crash":

1. exactly one terminal transition for the failed task (no stranded `Running` task);
2. one primary diagnostic for the seeded error (secondary diagnostics are allowed only
   where the frozen contract says so; the test records the exact count);
3. no accepted IR function or artifact for the failed TU;
4. bounded tick count and an explicit `Failed`/`Unsupported` outcome;
5. the next construct after the recovery point is not silently swallowed (REC-01/REC-03);
6. no fabricated value or partial successful result.

### 4.6 Producer/consumer integration

A3 cases use one bus and real upstream artifacts. Hand-built semantic records are not
allowed on the accepted path. The chain is PP (real source bytes) → LX → PA → TY → SE →
CL/T08 → IR/T09; the test records the actual IDs handed from each producer to its
consumer.

| ID | Fixture | Expected integration result | Basis / oracle | Level | Readiness |
|---|---|---|---|---|---|
| V1-INT-01 | V1-EXPR-01 `return 2+3;` | Real chain yields `5`. In the M1 baseline form T08 folds one `ConstRecord` with magnitude `5` and T09 consumes the same `ConstId` (`Constant` then `Return`, no refold). If the frozen V1 IR instead lowers arithmetic, the value must still be `5`, the IR must not double-evaluate, and any `ConstRecord` that is produced must be consumed without refold. | M1-CL-05 precedent; C11 6.5.6; hand | A3/A4 | `BASELINE-M1` for the folded form; alternate V1 lowering is `BLOCKED-SCHEMA` |
| V1-INT-02 | V1-LOC-03 `int a=2; int b=3; return a+b*4;` | Real chain yields `14`; two block-scope `Object` symbols `a`,`b`; initialization before reads; final value 14; no global object | C11 6.7/6.7.9/6.5.5/6.5.6; hand | A3/A4 | `BLOCKED-SCHEMA` |
| V1-INT-03 | V1-ASN-02 `a=b=4; return a+b;` | Real chain yields `8`; `b` assigned once, then `a` once; both reads see `4`; assignment results are non-lvalues; identity conversions | C11 6.5.16p1/p3/6.5.16.1; hand | A3/A4 | `BLOCKED-SCHEMA` |
| V1-INT-04 | V1-NEG-01 `return x;` | Real chain produces one undeclared diagnostic; no `x` symbol; no IR function/artifact | C11 6.5.1p2; hand | A3 | `BLOCKED-SCHEMA` |
| V1-INT-05 | V1-UNS-01 `return 2+3.0;` | Real chain fails with explicit `Unsupported`; no accepted IR/artifact; no partial success | V1 profile; hand | A3 | `BLOCKED-SCHEMA` |
| V1-INT-06 | V1-REC-01 malformed initializer then `int b=2;` | Real chain fails with one primary diagnostic; recovery does not fabricate a second TU/function; no accepted artifact; bounded ticks | C11 grammar + T05 recovery; hand | A3 | `BLOCKED-SCHEMA` |
| V1-INT-07 | V1-LOC-01 `int a; a=2; return a;` | Real chain yields `2`; local symbol has `Automatic` storage, block scope, no linkage; no disk/memory I/O in chips | C11 6.7/6.5.16; hand | A3/A4 | `BLOCKED-SCHEMA` |
| V1-INT-08 | Any accepted V1 fixture | Producer/consumer identity checks: T04 `LiteralRecord` fields (`kind=Integer`, `radix=10`, `suffix=None`, correct magnitude/negative), T05 node ranges are committed tokens, T06 `int` `TypeId` is canonical/reused, T07 `SemRecord` is one per checked node, any produced T08 `ConstRecord` is the same `ConstId` consumed by T09, T09 does not refold | C11 + frozen record contract; hand | A3 | `BLOCKED-SCHEMA` |
| V1-INT-09 | Negative fixture V1-NEG-05 redeclaration | Real chain has one `a` symbol (the first) and one conflict diagnostic; no duplicate symbol accepted; later `return a` does not become a second valid definition | C11 6.7p3; hand | A3 | `BLOCKED-SCHEMA` |

**A3 pass criteria.**

- The fixture bytes are the real input; PP is not seeded with a hand-built token stream.
- Every upstream artifact consumed by the next stage is the actual committed record from
  the previous stage; the test records the IDs and asserts identity/equality.
- The accepted path has no synthetic semantic facts. If a producer is missing, the case
  is temporarily A1/A2 only and is marked as such; it is not counted as A3.
- The final semantic value and the expected variable states are exact. If the frozen V1
  IR shape permits more than one legal lowering, exact instruction counts are not
  asserted; the test asserts the frozen invariants (initialization before first read,
  assignment updates visible, no duplicated side effects, terminator last).
- A failed batch commits no accepted semantic state (§4.8 W04–W07).

### 4.7 Replay, snapshot, and trace determinism

| ID | Scenario | Expected result (pass criterion) | Basis / oracle | Level | Readiness |
|---|---|---|---|---|---|
| V1-REP-01 | V1-EXPR-02 run twice from identical source/config/initial state | `Snapshot::capture` bytes equal; `Trace::to_bytes` equal | T01 C05; hand/contract | A4 | `BLOCKED-SCHEMA` |
| V1-REP-02 | V1-LOC-03 run three consecutive times | All three snapshot bytes and trace bytes equal; stable IDs/order; no address/host data | T01 C05; deterministic contract | A4 | `BLOCKED-SCHEMA` |
| V1-REP-03 | V1-NEG-01 run twice | Failure trace equal; both runs fail with the same primary diagnostic and no accepted artifact; bounded ticks | T01 C05 + failure atomicity | A4 | `BLOCKED-SCHEMA` |
| V1-REP-04 | V1-EXPR-02 with `2` changed to `3` (`3+3*4`) | Semantics change to `15` (hand); snapshot must differ from REP-01; no hash collision | C11 6.5.5/6.5.6; hand | A4 | `BLOCKED-SCHEMA` |
| V1-REP-05 | Same V1 run from two different process invocations (candidate or test harness) | Snapshot/trace bytes equal for the same fixture/config; no wall-clock, temp path, or address influence | T01 C05; deterministic contract | A4 | `BLOCKED-SCHEMA` (candidate V1 driver) |
| V1-REP-06 | New task enqueued in tick T | Executable in T+1 by default; no same-tick re-entry; dispatch order is the frozen `(stage, phase, enqueue ordinal, TaskId)` order | T01 §4; contract | A2/A4 | `BLOCKED-SCHEMA` (V1 tasks) |
| V1-REP-07 | A completed child result is consumed | Exactly once; a second consume yields `ResultAlreadyConsumed`; no duplicate completion | T01 C03; contract | A2 | `BLOCKED-SCHEMA` (V1 tasks) |

**A4 pass criteria.**

- Equality is byte equality of the canonical snapshot and trace, not object equality or a
  manually chosen subset.
- A snapshot hash may be printed as evidence, but no expected hash is embedded in the test
  table before a contract freeze defines the full V1 record set. The expected result is
  "identical across runs", plus the semantic values of §4.1–4.3.
- Different inputs must produce different snapshots; sensitivity is part of determinism,
  not just repeatability.
- Negative fixtures must terminate within the configured tick budget; a timeout is a
  failure, not a pass.

### 4.8 Non-matching task and undeclared-write cases

These cases target the chip/commit contract, not V1 language semantics. They are
designed now and can be partially authored against the existing T01 bus; V1-specific
counterpart tests are blocked only by the V1 manifest/kind rows.

| ID | Scenario | Expected result (pass criterion) | Basis / oracle | Level | Readiness |
|---|---|---|---|---|---|
| V1-WS-01 | V1 chip core receives a task with a foreign kind/production (e.g., an assignment core receives a parse `2+3` task) | Exactly one `Fail` on the `Task` channel; no semantic proposal; no store mutation | T01 §4; chip DoD "non-matching task" | A1 | `BLOCKED-SCHEMA` (V1 chip exists) |
| V1-WS-02 | V1 chip core receives a non-`Running` task state | Exactly one `Fail` on the `Task` channel; no writes; deterministic | T01 §4; DoD | A1 | `BLOCKED-SCHEMA` |
| V1-WS-03 | An unregistered V1 task kind is dispatched through the shell | Task becomes `Failed` with an explicit "not registered" diagnostic; no semantic store writes; no idle spin | T01 C06; contract | A2 | `BLOCKED-SCHEMA` (V1 kind) |
| V1-WS-04 | Manifest registration of a V1 chip with an undeclared store field or no allowlist ownership row | `ManifestRegistryError::Validation` wrapping `ManifestError::UnknownWriteField` (field absent from the schema) or `ManifestError::StoreOwnerViolation` (no allowlist row); no bus mutation | T01 C04; manifest contract | A1 | `AVAILABLE MECHANISM`; V1 rows later |
| V1-WS-05 | Commit a `StorePatch` to an undeclared store field | `CommitError::UndeclaredStoreField`; batch rejects before mutation; pre/post snapshot bytes equal | T01 C03/CT06; commit contract | A2 | `AVAILABLE MECHANISM`; V1 patch later |
| V1-WS-06 | Commit a patch to a declared field by a chip that does not own it | `CommitError::WriteNotDeclared`; batch rejects before mutation; pre/post snapshot bytes equal | T01 C03/CT06 | A2 | `AVAILABLE MECHANISM`; V1 patch later |
| V1-WS-07 | Commit a patch with a stale store version | `CommitError::StaleVersion`; batch rejects before mutation; snapshot bytes equal | T01 C03/CT06 | A2 | `AVAILABLE MECHANISM`; V1 patch later |
| V1-WS-08 | Valid V1 commit, then compare the semantic projection before/after | Only the chip's declared store fields/versions change; all language stores/fields outside the declared write manifest remain byte-for-byte unchanged. Protocol-owned task/result/wire registers are compared separately under the frozen projection, since a valid commit must transition tasks. | T01 C04/C05; DoD "unauthorized-field-unchanged" | A2 | `BLOCKED-SCHEMA` (V1 chip) |
| V1-WS-09 | Test-local routing maps a V1 kind to a worker that does not claim that kind; drive the task and compare the pre/post snapshot | Worker fails on the `Task` channel; language semantic stores are unchanged; only protocol-owned task/diagnostic registers change; no partial semantic record | T01 §4; chip DoD "non-matching task"; T02 routing | A2 | `BLOCKED-SCHEMA` (V1 kind/route) |

**A2 write-scope pass criteria.**

- The error variant is exact (or the frozen equivalent); "it failed" is not enough.
- The snapshot captured before the rejected batch and after it is byte-identical,
  including task states, diagnostics, and store versions. A rejected batch may not leave
  a `Running` task or a partial record.
- For WS-08, the test uses the canonical semantic projection; scheduler-only registers
  are excluded only where the frozen contract explicitly says so.

### 4.9 Closed-loop evidence

Part A is target-independent and uses the V1 IR interpreter. Part B is probe-gated and
uses candidate-generated assembly plus external tools. No Part A result is a Part B pass.

| ID | Fixture | Expected result (pass criterion) | Basis / oracle | Level | Readiness |
|---|---|---|---|---|---|
| V1-CL-A01 | `return 5;` | Interpreter returns mathematical `5`; no unsupported op | C11 6.8.6.4/5.1.2.2p3; hand | A5A | `BLOCKED-SCHEMA` (V1 interpreter; M1 VF12 covers only the M1 shape) |
| V1-CL-A02 | V1-EXPR-02 `2+3*4` | Interpreter returns `14` | C11 6.5.5/6.5.6; hand | A5A | `BLOCKED-SCHEMA` |
| V1-CL-A03 | V1-LOC-03 locals | Interpreter returns `14`; variable states `a=2,b=3` at return | C11 6.7/6.5.5/6.5.6; hand | A5A | `BLOCKED-SCHEMA` |
| V1-CL-A04 | V1-ASN-02 assignment chain | Interpreter returns `8`; `a=4,b=4` | C11 6.5.16; hand | A5A | `BLOCKED-SCHEMA` |
| V1-CL-A05 | V1-NEG-01 undeclared | Interpreter/verifier must not produce a value; typed failure; no accepted IR | C11 6.5.1p2; hand | A5A | `BLOCKED-SCHEMA` |
| V1-CL-B01 | `return 5;` | Candidate-generated AArch64 assembly/object; external assembler/linker; process exits `5` | C11 5.1.2.2p3 + frozen Linux substrate; hand | A5B | `BLOCKED-PROBE` + `BLOCKED-HOST` |
| V1-CL-B02 | V1-EXPR-02 `2+3*4` | Process exits `14` | same; hand | A5B | `BLOCKED-PROBE` + `BLOCKED-HOST` |
| V1-CL-B03 | V1-LOC-03 locals | Process exits `14` | same; hand | A5B | `BLOCKED-PROBE` + `BLOCKED-HOST` |
| V1-CL-B04 | V1-ASN-02 assignment chain | Process exits `8` | same; hand | A5B | `BLOCKED-PROBE` + `BLOCKED-HOST` |
| V1-CL-B05 | `return 0;` | Process exits `0` (boundary success) | C11 5.1.2.2p3; hand | A5B | `BLOCKED-PROBE` + `BLOCKED-HOST` |
| V1-CL-B06 | `return 127;` | Process exits `127` (boundary non-zero status in the chosen 0..=127 range) | Target/OS exit-status contract; hand | A5B | `BLOCKED-PROBE` + `BLOCKED-HOST` |
| V1-CL-B07 | V1-NEG-01 undeclared | No object/executable; diagnostic; no successful run | C11 constraint + Part B fail-closed | A5B | `BLOCKED-PROBE` + `BLOCKED-HOST` |
| V1-CL-B08 | V1-CL-B01 artifact | ELF magic/class/endianness, `e_machine=AArch64`, `.text` allocate+execute, defined global `main`, no Mach-O markers | Frozen target identity/ABI; not probe values for ELF structure | A5B | `BLOCKED-HOST` (substrate absent; probe for ABI details) |
| V1-CL-B09 | V1-CL-B01 repeated three times | Each run exits `5`; semantic ELF equality only (not byte-identical build ID/timestamps) | T01 C05 + target acceptance D6 | A5B | `BLOCKED-PROBE` + `BLOCKED-HOST` |

**Exit-status scope.** C11 5.1.2.2p3 makes `return` from `main` equivalent to `exit` with
the returned value; the host mapping for values other than 0/`EXIT_SUCCESS`/
`EXIT_FAILURE` is implementation-defined. The suite therefore asserts exact statuses
only for the chosen 0..=127 fixtures on the frozen Linux substrate, and records this as a
target-behavior assertion, not a universal C11 guarantee.

**Part B pass criteria.**

- The candidate emits its own assembly; no GCC/Clang invocation compiled the candidate
  program. External assembler/linker are allowed and required by the milestone.
- The compiled object is an AArch64 ELF, not Mach-O; `main` is a defined global.
- The process terminates normally (not by signal) with the exact expected status.
- Interpreted result alone does not satisfy Part B.
- Any concrete ABI/stack/register assertion uses probe-confirmed values, never the macOS
  host or a guessed number.

### 4.10 Matrix summary

| Group | Design rows | Normative rows | Non-normative / profile rows | Coverage |
|---|---:|---:|---:|---|
| V1-EXPR | 18 | 17 | 1 (EXPR-18 overflow) | values, precedence, parentheses, unary, AST shape |
| V1-LOC | 15 | 13 | 2 (LOC-14 value, LOC-15 profile) | declarations, initializers, locals, nested scope |
| V1-ASN | 12 | 12 | 0 | assignment, lvalue, identity conversion, return |
| V1-NEG | 13 | 12 | 1 (NEG-13 division by zero) | undeclared, non-lvalue, redeclaration, syntax; EXPR-18 carries the overflow policy case |
| V1-UNS | 10 | 10 | 0 | explicit unsupported outside V1 |
| V1-REC | 6 | 6 | 0 | finite recovery and no accepted artifact |
| V1-INT | 9 | 9 | 0 | real producer/consumer handoffs |
| V1-REP | 7 | 7 | 0 | deterministic snapshot/trace |
| V1-WS | 9 | 9 | 0 | non-matching task and write scope |
| V1-CL | 14 | 14 | 0 | A5A interpreter and A5B target loop |

Counts in this table are design counts; they are not executed tests and must not be
reported as a pass count.

---

## 5. Execution Protocols

### 5.1 A1 pure-unit protocol

1. Build the narrow input projection directly in the test (tokens/nodes/records) with
   the exact fields under test; no bus mutation is required.
2. Call the worker's pure `compute`/core once.
3. Assert the **entire** proposal slice: one expected `Complete`/`Fail`/`AppendRecords`
   shape, no extra diagnostic, no write proposal.
4. For a non-matching task/state, assert exactly one `Fail` on the task channel and no
   semantic proposal.
5. Run the pure core twice with identical input and assert identical proposals (A1
   determinism is a subset of A4).

A1 must not use a helper that parses the fixture to derive the expected tree. The input
projection is the fixture; the expected output is the hand table.

### 5.2 A2 dispatch/commit protocol

1. Create the frozen V1 bus/config; install exactly the V1 kind registry, schema, routes,
   and workers under test.
2. Bootstrap one task with a fixed payload; capture the pre-tick snapshot.
3. Tick with `RoutingShell` until terminal or the explicit budget; capture the post-tick
   snapshot.
4. Assert exactly one terminal transition, the expected diagnostic/result, the expected
   store delta, and no `Running` residue.
5. For a rejected batch, assert the exact commit error and pre/post snapshot equality
   for all undeclared fields.

### 5.3 A3 integration protocol

1. Read the fixture bytes from `compiler/tests/fixtures/v1/`; do not synthesize the
   source token stream.
2. Run PP (normalize/splice/comment/scan/directives/conditional/macro/emit as required by
   the test) on the real bytes; assert the committed token spelling/order matches the
   hand table.
3. Run LX (intern/classify/decode) and assert literal records and token kinds.
4. Run PA and assert the expected AST/semantic shape (operator tree, declaration,
   initializer, assignment).
5. Run TY, SE, CL, IR in producer order; after each stage assert the consumer received
   the real committed ID/record from the previous stage.
6. Assert the final semantic value and variable states from `expected.rs`.
7. Capture and report the actual IDs, record counts, and snapshot hash as evidence, but
   do not use them to derive the expected value.

### 5.4 A4 replay protocol

1. Run the full chain twice (or three times for REP-02) from a fresh bus/config/input.
2. Capture `Snapshot::capture(bus)` and `Trace::to_bytes()` after each run.
3. Assert byte equality; for a negative run, also assert equal diagnostic category/order
   and bounded tick count.
4. Run the sensitivity variant (REP-04) and assert the snapshot differs and the semantic
   value changes exactly as hand-calculated.
5. Report the first divergence (tick/record field) if equality fails; do not dump the
   whole snapshot into the report.

### 5.5 A5A interpreter protocol

1. Lower the accepted V1 fixture to IR through the real chain.
2. Invoke the V1 symbolic interpreter with no memory image and no host input.
3. Assert the modeled return value and final local states; an unsupported op is a typed
   failure, never a fabricated value.
4. Record that this is Part A evidence, not target execution.

### 5.6 A5B target protocol

1. Require a probe report with `verified=true` and the frozen identity; otherwise mark
   `BLOCKED-PROBE` and do not run.
2. Run the candidate with `-std=c11 -O0 -S` (or the frozen equivalent) to emit assembly;
   ensure the candidate itself generated it, with no GCC/Clang invocation.
3. Assemble and link with the frozen external tools; inspect ELF/AArch64 structure.
4. Run under H05 isolation with the frozen timeout/environment; assert alive exit status.
5. Repeat three times for status determinism; compare ELF semantically, not byte-for-byte.
6. Record the exact tool versions, flags, runner, and probe hash in the evidence record.

---

## 6. Exact Future Commands and Test Paths

### 6.1 New files (not created by this task)

```text
compiler/tests/v1_support/mod.rs
compiler/tests/v1_support/expected.rs
compiler/tests/v1_support/sources.rs
compiler/tests/v1_expr.rs
compiler/tests/v1_locals_decl.rs
compiler/tests/v1_assignment_conversion.rs
compiler/tests/v1_negative.rs
compiler/tests/v1_unsupported_recovery.rs
compiler/tests/v1_integration.rs
compiler/tests/v1_replay.rs
compiler/tests/v1_write_scope.rs
compiler/tests/v1_closed_loop.rs
compiler/tests/fixtures/v1/README.md
compiler/tests/fixtures/v1/*.c
```

No `compiler/Cargo.toml`, `compiler/src/**`, `compiler/contracts/**`, or `mod.rs` change
is permitted for this suite. Cargo auto-discovers top-level `tests/*.rs`; the support
directory is not a test target. If the integrator later adds explicit `[[test]]`
entries, that is an integrator-owned integration change, not a test-owner edit.

### 6.2 Formatting and static checks (future, after tests exist)

Run from the repository root:

```sh
cargo fmt --manifest-path compiler/Cargo.toml --all -- --check
cargo clippy --manifest-path compiler/Cargo.toml --workspace --all-targets -- -D warnings
```

Run the root/package checks only in their applicable directories; do not claim the whole
repository from one package.

### 6.3 V1 suite commands (future)

```sh
cargo test --manifest-path compiler/Cargo.toml --test v1_expr
cargo test --manifest-path compiler/Cargo.toml --test v1_locals_decl
cargo test --manifest-path compiler/Cargo.toml --test v1_assignment_conversion
cargo test --manifest-path compiler/Cargo.toml --test v1_negative
cargo test --manifest-path compiler/Cargo.toml --test v1_unsupported_recovery
cargo test --manifest-path compiler/Cargo.toml --test v1_integration
cargo test --manifest-path compiler/Cargo.toml --test v1_replay
cargo test --manifest-path compiler/Cargo.toml --test v1_write_scope
cargo test --manifest-path compiler/Cargo.toml --test v1_closed_loop -- --skip v1_part_b
```

After the probe/host gate opens, the Part B subset is explicit:

```sh
cargo test --manifest-path compiler/Cargo.toml --test v1_closed_loop v1_part_b -- --ignored --nocapture
```

The suite must print a level summary (A1/A2/A3/A4/A5A/A5B passes, not-run and blocked
counts). A command that exits 0 while skipping blocked tests is not a full V1 pass.

### 6.4 Candidate Part A evidence (future; requires H04 V1 wiring)

```sh
cargo run --quiet --manifest-path compiler/Cargo.toml --bin candidate -- \
  -std=c11 -O0 --emit-ir-snapshot --emit-trace --interpret-ir \
  compiler/tests/fixtures/v1/v1_expr_02.c
```

Expected: exit 0, snapshot/trace evidence, modeled return `14` (for the `2+3*4`
fixture), and no target emission. For a negative fixture, expected exit 1 with a
structured diagnostic and no evidence artifact. These flags already exist for the M1
driver; the V1 pipeline wiring is the missing H04 work.

### 6.5 Candidate Part B closed loop (future; blocked until probe/host gate)

The exact tool names/flags must come from the H01/H07 freeze. The following is the
planned shape, not a runnable command today:

```sh
# 1. candidate emits AArch64 assembly itself
cargo run --quiet --manifest-path compiler/Cargo.toml --bin candidate -- \
  -std=c11 -O0 -S -o "$OUT/main.s" compiler/tests/fixtures/v1/v1_loc_03.c

# 2. external assembler (name/flags frozen by H01)
"$FROZEN_AARCH64_AS" -o "$OUT/main.o" "$OUT/main.s"

# 3. external linker (name/flags frozen by H01); never pass C source to a C compiler
"$FROZEN_AARCH64_LD" "$OUT/main.o" -o "$OUT/main"

# 4. isolated run (H05) and exact status check
"$FROZEN_RUNNER" "$OUT/main"
status=$?
test "$status" -eq 14
```

Do not replace the placeholders with macOS `as`/`ld`, a Darwin ABI, a host `int` width,
or a guessed stack alignment. If the probe has not verified the target, the correct
result is `BLOCKED-PROBE`, not a substituted macOS result.

### 6.6 Evidence to retain

For each executed case, retain:

- fixture name and exact bytes/hash;
- contract version/hash and compiler revision;
- test command and full output;
- level reached (A1..A5B), pass/fail/not-run;
- diagnostic category/order and first divergence for negative/replay failures;
- snapshot/trace hashes and semantic projection;
- target probe hash, tool versions, runner, and exit status for Part B;
- any synthetic upstream substitute and why it was allowed at that level.

---

## 7. Dependencies, Blockers, and Open Questions

### 7.1 Schema/contract blockers (V1 Part A)

These blockers are about missing **frozen interfaces**, not about missing test effort.
The owner column names the package that must freeze the shape; the T01 integrator owns
the actual shared-file change and version/hash bump.

| ID | Missing shape | Current fact | Needed for | Owner | Cases blocked |
|---|---|---|---|---|---|
| B1 | V1 task kinds, stage rows, routing, allowlist rows | Registry has only the `/39` M1/PP/PA/TY/SE/CL/IR slices; no local declaration, initializer, assignment, identifier, general arithmetic, or V1 IR kinds | Every A2/A3 V1 case | T01 + T05/T06/T07/T08/T09/T13 | Most non-baseline cases |
| B2 | `NodeKind` and parser productions | Closed M1 set of 8 kinds; `pa_tu`/`pa_block` accept fixed shapes; `pa_binary` accepts only `2+3`; `pa_unary` only `+3`/`-3`; no identifier expression, assignment, declaration/initializer, `*`/`/`/`%` | V1-EXPR/LOC/ASN parse shapes and A3 | T05 | EXPR-02..18, LOC-*, ASN-* |
| B3 | Local object symbol/type path | `TySymbolChip` requires a `Function` type, a `Declarator` node, and a `File` scope; block-scope declarations explicitly deferred | Local `Object`/`Automatic` symbols and lookup | T06 | LOC-*, ASN-*, INT-02/03/07/09 |
| B4 | Semantic lvalue/assignment/conversion facts | Frozen `SemRecord` has `{node,ty,category,effects}` and M1 produces `NonLvalue`; no `conversions` field or assignment/declaration facts | Assignment legality, lvalue conversion, identity conversion evidence | T07 | ASN-*, NEG-03/04, INT-03 |
| B5 | Constant evaluator op set | `ConstExprOp` only `Add`; `ConstantRequest` is literal/binary-add only; no `-`, `*`, `/`, `%`, unary | Constant-only V1 expressions and initializers | T08 | EXPR-02..18, LOC-10/11 |
| B6 | IR locals/assignment/arithmetic | `IrOp` only `Constant`/`Return`; `FunctionRecord` has no locals/frame; `BlockRecord` single entry; no load/store/arithmetic | Local initialization/assignment and V1 IR verification | T09 | LOC-*, ASN-*, INT-02/03/07, CL-A02/A03/A04 |
| B7 | Snapshot/hash/version for V1 records/kinds | Encoders cover the frozen `/39` record set; new kinds/enum variants/records must be encoded and hashed | A4 replay, A3 evidence | T01 integrator | REP-*, INT-* |
| B8 | V1 verifier/interpreter | VF12 models only the M1 `Constant`/`Return` shape; VF06's typed matrix is M1-Minimal; VF01's reserved-store check is M1-shaped | A3 verification, A5A interpreter | T13 + T01 | INT-*, CL-A* |
| B9 | Candidate V1 pipeline | `candidate` installs the M1 PP/LX/PA/TY/SE/IR chain; V1 source cannot reach evidence | A5A via driver, REP-05 | H04 | CL-A*, REP-05 |
| B10 | Declaration/initializer/assignment request/result carriers | `Payload` is `RecordRef`-only; `next_cursor` carrier (OB-30) and V1 child-task shapes are open | Parser continuations and integration | T05 + T01 | EXPR/LOC/ASN/INT |
| B11 | Conversion plan representation | M1 identity is "absence of a plan"; the frozen `SemRecord` has no `conversions`; role/op encodings are open | ASN conversion assertions | T06/T07/T09 + T01 | ASN-04/05/07/08, INT-03 |

**Do not work around B1–B11 with private test-local types.** A test may not invent a
parallel AST, conversion plan, or IR shape to make an expected result pass. If an
interface is missing, the case stays blocked and the blocker is reported.

### 7.2 Target/probe blockers (V1 Part B)

| ID | Blocker | Current fact | Cases blocked |
|---|---|---|---|
| P1 | Target probe not run | `target_verification=unverified`; no probe report/hash; `ensure_codegen_ready` fails closed | CL-B01..B09 |
| P2 | No target toolchain/substrate | Assembler, linker, sysroot, runner, and Linux CI/VM are planned, not provisioned | CL-B01..B09 |
| P3 | No H05 isolated execution | Timeouts, resource limits, cancellation, crash classification absent | CL-B01..B09 |
| P4 | No H04 V1 driver/emission | Candidate Part A pipeline is M1; `-S`/`-c` fail closed | CL-A via driver, CL-B |
| P5 | Concrete ABI/layout values unverified | `int` size/alignment, frame layout, register save area, stack alignment; probe/H07 attestation pending | Any concrete-width/stack/ABI assertion in an expanded ASN case and in CL-B |

The probe blocks **only** concrete target assertions and the runnable target artifact.
It does **not** block Part A symbolic values (all normative expression results here are
small non-negative values that fit any conforming `int`; the non-normative overflow
fixture makes no width claim) or the C11 semantic expectations. Do not use macOS arm64
values as a substitute.

### 7.3 Host/tooling blockers

- No V1 pipeline registration in the candidate driver (H04); the current driver is M1.
- No isolated runner (H05); target execution would be untrusted and unclassified.
- No frozen assembler/linker/sysroot command lines; placeholders in §6.5 must be filled
  by H01/H07 before Part B.
- No V1 evidence classifier/interpretation registration (H04/T13) for the V1 IR shape.
- The repository's `compiler/` package is a nested standalone package; run commands with
  `--manifest-path compiler/Cargo.toml`, not from the root only.

### 7.4 Tests that can be authored before the freeze

These are **design-ready** and can be written against existing frozen mechanisms, even
though their V1-specific counterparts remain blocked:

| Area | Cases | Why it can be authored now |
|---|---|---|
| Commit/manifest write-scope | V1-WS-04..07 | T01 C03/C04/CT06 are frozen; the tests use test-local adversarial workers/patches and exact `CommitError` variants |
| A1 non-matching-state checks | V1-WS-01/02 pattern | The pattern is frozen by the chip DoD; the V1 chip instance is future |
| Negative diagnostic shape | V1-NEG-01/03/05/06 pattern | The semantic diagnostic groups exist; exact V1 path/kind is future |
| Snapshot/trace harness | V1-REP pattern | `Snapshot`/`Trace` APIs are frozen; the V1 chain is future |
| M1 baseline regression | V1-EXPR-01, V1-CL-A01 (M1 form) | M1 chain exists for the fixed `2+3` shape; reuse it as the baseline before adding V1 cases. V1-ASN-06 is not an M1 baseline because the M1 block parser accepts only the fixed `2+3` return shape. |

Writing the test file before the freeze is allowed only if it has no private V1 schema
and fails loudly with `BLOCKED-SCHEMA`/`not supported` rather than fabricating a pass.
The safest division is: author the harness and M1 baseline now; author V1-specific
cases after the V1 freeze.

### 7.5 Open questions for the integrator/T01 owner

These must be answered before the corresponding rows become normative. Recommended
answers are given; the integrator may amend them.

1. **Integer type scope.** Is V1 `int`-only (recommended) or does it include
   `char`/`short`/`long`/`unsigned`? If broader, the probe and conversion matrix become
   mandatory for those cases, and this suite's "Part A symbolic" rows must be split.
2. **Arithmetic op set.** Does V1 require `+ - * / %` and unary `+`/`-` (recommended), or
   only additive? If only additive, EXPR-02/04/06..14/17 move to an explicit deferred
   set, not to a pass.
3. **Mixed declarations/statements.** Does V1 support `int a=1; a=2; int b=3;` per C11
   6.8.2 (recommended), or require all declarations first? If not supported, the case
   must be an explicit `Unsupported`, not a misparse.
4. **Nested blocks/shadowing.** Does V1 support a nested compound statement and local
   shadowing (recommended as a boundary), or only one function-body scope? The same
   explicit-unsupported rule applies.
5. **Multiple declarators.** Does V1 support `int a=2, b=3;`? Recommended yes, because it
   is ordinary declaration syntax; otherwise mark V1-LOC-05 explicitly deferred.
6. **Expression statements.** Does V1 support `a = 2;` as a statement? It is needed for
   ordinary local assignment; recommended yes with no side-effect-free-expression
   requirement. If not, assignment can only appear in initializers/return, which would
   make the "assignment" feature incomplete.
7. **Conversion representation.** On the frozen `SemRecord`, is identity conversion
   recorded as an explicit `Identity` plan or as the M1 "no plan" absence? The test must
   not assert both; the freeze must choose one. Recommended: freeze the M1-compatible
   "no plan" for identity, or a single explicit `Identity` entry; do not leave both
   accepted silently.
8. **Local IR representation.** May a non-address-taken local be held in a virtual
   register/value, or must it have a stack slot? Multiple legal lowerings exist. The
   V1 IR contract should state which invariants are observable (initialization before
   read, assignment visibility, no duplicate side effects, terminator last). Exact
   instruction counts should be frozen only if the contract intends one canonical
   lowering.
9. **Constant folding policy.** For constant subexpressions, must T08 fold, or may T09
   emit arithmetic? The M1 path folds `2+3`; V1 should state whether folding is a
   mandatory semantic step, an optional optimization, or a recorded implementation
   choice. The value must be correct either way; no double evaluation/refold.
10. **Diagnostic taxonomy for syntax.** The frozen `DiagGroup` currently has no distinct
    syntax group; parse failures use task/unsupported channels. V1 should freeze the
    syntax diagnostic group/code and the primary-vs-secondary recovery rule.
11. **Uninitialized locals.** Confirm that V1 does not assign a defined value to an
    uninitialized local and does not test reads of indeterminate values. Only
    "declared but never read" is in this suite.
12. **`main` return status range.** Confirm the closed-loop status fixtures 0..=127 and
    the Linux low-8-bit behavior; do not extend to negative/large values without a
    target-OS decision.

### 7.6 Readiness summary by case group

| Group | Design complete | A1/A2 now | A3 now | A4 now | A5A now | A5B now | Primary blockers |
|---|---:|---:|---:|---:|---:|---:|---|
| V1-EXPR | yes | only E01 | no (M1 value only; real-PP wiring pending) | no | only M1 form | no | B1,B2,B5,B6,B7,B8; P1–P5 for A5B |
| V1-LOC | yes | no | no | no | no | no | B1,B2,B3,B4,B6,B7,B8; P1–P5 |
| V1-ASN | yes | no | no | no | no | no | B1,B2,B3,B4,B6,B7,B8,B11; P1–P5 |
| V1-NEG | yes | pattern only | no | no | no | no | B1,B2,B3,B4; B10 for parse carriers |
| V1-UNS | yes | explicit-reject pattern | no | no | no | no | B1,B2; V1 reject paths |
| V1-REC | yes | pattern only | no | no | no | no | B1,B2,B10 |
| V1-INT | yes | no | no | no | no | no | B1–B11; H04 for driver |
| V1-REP | yes | harness pattern | no | no | no | no | B1–B8; H04 for REP-05 |
| V1-WS | yes | WS-04..07 | no | no | no | no | B1/B2 for V1 chip; T01 mechanisms exist |
| V1-CL | yes | no | no | no | no | no | B1–B11; P1–P5; H04/H05 |

"Now" means with the current frozen `/39` mechanism, not that the V1 test file exists.

---

## 8. Acceptance Gates and Evidence Ledger

### 8.1 V1 acceptance gates

| Gate | Requirement | Owner |
|---|---|---|
| V1-G1 | V1 profile (§1.2–1.4) and all schema/kind/record/conversion/IR shapes are frozen with a new contract version/hash; the V1 test suite pins that version and fails on mismatch | T01 integrator + owners |
| V1-G2 | All V1-core A1/A2 cases pass; normal, boundary, invalid, unsupported, and recovery categories are represented | T05/T06/T07/T08/T09 |
| V1-G3 | A3 producer/consumer cases pass on real upstream artifacts; synthetic substitutes are absent from the accepted path | T05–T09 + Integrator |
| V1-G4 | A4 replay cases pass with byte-identical snapshot/trace for identical input and a detectable difference for a changed literal | T13 + Integrator |
| V1-G5 | Every invalid/unsupported case fails explicitly with a typed diagnostic and no accepted artifact; recovery is finite and does not swallow the next construct | T02/T05/T07 |
| V1-G6 | Write-scope cases pass: exact `Manifest`/`CommitError` variants and pre/post snapshot equality for rejected batches | T01/T02 |
| V1-G7 | The expected-value tables are independent: no implementation call, GCC/Clang output, or reference output is used as the oracle; review sign-off | Independent tester + reviewer |
| V1-G8 | Part A interpreter returns the hand-derived value for the accepted fixtures; unsupported IR never becomes a value | T13 + T09 |
| V1-G9 | Part B target artifacts are probe-verified, assembled/linked externally, run under H05, and exit with the exact status; interpreter is not reported as target execution | T11 + H01/H05/H07 |
| V1-G10 | No test special-cases filenames, no unimplemented behavior is reported as success, and no unsupported case is removed from the denominator | Reviewer |

A V1 report must state, separately: A1/A2 pass count, A3 pass count, A4 pass count,
A5A pass count, A5B pass count, blocked/not-run count, and the exact contract version.
Passing V1 is not a T00 torture rate and is not a claim of full C11 support.

### 8.2 Evidence ledger (this task)

| Check | Status | Evidence |
|---|---|---|
| Read `AGENTS.md`, `docs/tasks/README.md`, T01, PARALLEL_EXECUTION, GUARDRAILS, T05–T09, MASS_PARALLEL_DISPATCH, M1 frontend/target acceptance | **DONE (read-only)** | File reads; no execution |
| Inspect existing tests `c12..c19`, `c34..c39` | **DONE (read-only)** | File reads; no test run |
| Inspect current bus/task/manifest/schema/snapshot/candidate state | **DONE (read-only)** | File reads; no build/test run |
| Author V1 test files | **NOT DONE** | This task is design-only |
| Source/test/contract files changed | **NONE** | Only this report was created; `git status` shows it as untracked. Pre-existing uncommitted files were not touched. |
| Run V1 tests | **NOT RUN** | No V1 test file exists |
| Run candidate Part A evidence | **NOT RUN** | No V1 pipeline; no command executed |
| Run target probe | **NOT RUN** | Probe substrate not provisioned; no report/hash |
| Assemble/link/run Part B | **NOT RUN** | No target toolchain/runner; fail-closed |
| Claim a pass rate or V1 completion | **NOT MADE** | No executed evidence |

### 8.3 What this report does not claim

- It does not claim any V1 chip, task kind, AST node, conversion plan, IR op, or
  diagnostic exists.
- It does not claim any test passed, any snapshot hash, any exit code, or any probe
  result.
- It does not freeze or amend T01, T05–T09, T13, the CDR, the M1 proposal, or any
  contract artifact.
- It does not authorize code changes or assign implementation ownership.
- It does not claim that the V1 profile in §1.2 is accepted; it is a proposed test
  profile for the integrator/T01 owner to confirm or amend.

---

## 9. Revision and Cross-References

### 9.1 Cross-references

- [AGENTS.md](../../../AGENTS.md) — repository rules, correctness, validation, security.
- [docs/tasks/README.md](../../tasks/README.md) — milestones, chip groups, definition of done.
- [T01_COMPILER_CONTRACT.md](../../tasks/T01_COMPILER_CONTRACT.md) — bus, IDs, task protocol, target identity.
- [PARALLEL_EXECUTION.md](../../tasks/PARALLEL_EXECUTION.md) — ownership, waves, handoff.
- [COMPILER_DEVELOPMENT_GUARDRAILS.md](../../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md) — correctness priority, chip isolation, review checklist.
- [T05_PARSE_CHIPS.md](../../tasks/T05_PARSE_CHIPS.md) — parse productions, continuations, recovery.
- [T06_SYMBOL_TYPE_CHIPS.md](../../tasks/T06_SYMBOL_TYPE_CHIPS.md) — scope/symbol/type/lookup, point of declaration.
- [T07_SEMANTIC_CHIPS.md](../../tasks/T07_SEMANTIC_CHIPS.md) — checked facts, value category, conversions, effects.
- [T08_CONSTANT_LAYOUT_INIT_CHIPS.md](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md) — constant evaluator, layout, initialization.
- [T09_IR_LOWER_CHIPS.md](../../tasks/T09_IR_LOWER_CHIPS.md) — IR schema/lowering, FunctionEnd, verification.
- [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) — A-TST-01 scope and V1 train definition.
- [M1_VERTICAL_SLICE_ACCEPTANCE.md](../../tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md) — M1 fixture/acceptance precedent.
- [M1_TARGET_ACCEPTANCE.md](../../tasks/M1_TARGET_ACCEPTANCE.md) — Part A/B split, probe and target gates.

### 9.2 Revision record

| Date | Change | Author |
|---|---|---|
| 2026-11-07 | Initial A-TST-01 design: V1 profile, layered architecture, fixture ownership, expression/local/assignment/invalid/unsupported/recovery/integration/replay/write-scope/closed-loop matrices, future paths/commands, and schema/probe blocker register. No tests run; no source edits. | Independent tester (A-TST-01) |
