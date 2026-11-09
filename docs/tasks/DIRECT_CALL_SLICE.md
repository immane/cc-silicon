# Direct-Call Slice — `/54` Schema and `/55` Implementation Plan

| Field | Value |
|---|---|
| Status | **`/54` schema frozen; `/55` source implementation dispatched** |
| Baseline | `t01-c01-c06/54`, hash `850adb9a2e4d4409072c9ad12586cd14aed5e5b305cbacc68a675b935f081e39` |
| Dispatch | [MASS_PARALLEL_DISPATCH §13](MASS_PARALLEL_DISPATCH.md#13-next-dispatch-from-53) |
| Goal | Same-TU, acyclic direct calls with canonical `int` parameters/result and a native AArch64 closed loop |
| Non-claims | No recursion, variadics, external/indirect calls, more than eight arguments, stack-passed arguments, general ABI, `-S`/`-c`, object writer, corpus result, or torture rate |

## 1. Frozen `/54` Shape

`/54` freezes carriers only; it adds no producer and proves no call behavior.

- `NodeKind` appends `ParameterList`, `Parameter`, `Call`.
- `FunctionDefinition` keeps its existing three children
  `[Specifiers, Declarator, Compound]`; a parameterized function `Declarator`
  has one `ParameterList` child, while `(void)` and local declarators keep no
  children.
- `ParameterList.children` are 1–8 `Parameter` nodes in declaration order;
  each `Parameter` carries one name and an implicit canonical `int` type.
- `Call.children` are the callee `Identifier` followed by argument expressions
  in source order. Parentheses create no node.
- `FunctionRecord.parameters` lists parameter symbols in declaration order.
- `InstructionRecord.callee` names the called function symbol for `IrOp::Call`;
  `operands` are arguments in source order and `result` is the return value.
- `SEMANTIC_CALL_EXPR` is `SEMANTIC` local 25; `SE_CALL_CHIP = ChipId(61)`;
  stage 4; one `sem.records` allowlist row.
- Snapshot codecs append the parameter list and callee presence fields.

Parameter values bind implicitly at function entry from the caller's ordered
arguments. No hidden call stack, global mutable state, or chip-to-chip call is
introduced; interpreter state is local scratch for one task computation.

## 2. `/55` Source Subset

```c
int add(int a, int b) { return a + b; }
int main(void) { return add(2, 3); }
```

Additional accepted shapes:

```c
int inc(int x) { return x + 1; }
int main(void) { return add(inc(2), inc(3)); }   /* 7 */

int add(int a, int b) { return a + b; }
int main(void) { int x = 2; return add(x, 3); }  /* 5 */
```

Rules:

- Definitions appear before use. A callee must be an earlier file-scope
  function definition. This makes recursive and mutually recursive calls
  unreachable without a separate call-graph analysis.
- Zero parameters are written `(void)`; up to eight parameters are accepted.
  A zero-parameter definition may not use the empty `()` form.
- Parser budgets are fixed at 32 functions per TU, 8 parameters/arguments per
  function/call, 32 levels of expression/parenthesis nesting, and 4096 AST
  nodes per TU; the boundary is accepted and the next value fails without an
  append.
- Every parameter, argument, local, and result is canonical signed `int`.
  Duplicate parameter names, arity mismatch, and non-int arguments fail.
- Functions keep one straight-line block. The `/49`–`/53` local declaration,
  expression, and ordered-assignment forms remain valid. Calls may appear in
  expressions and arguments.
- Literal/literal `+` folding remains as before. Generic-program literal
  constants are materialized in source order before semantic checks; T09 pairs
  literal IDs to that prefix rather than guessing by value.
- Calls, parameters, and return values have no observable side effects in this
  slice. Argument evaluation is left-to-right for this pure subset; this is a
  closed implementation rule, not a general C sequencing claim.

## 3. Work Orders

All implementation work orders start from the `/54` schema commit. The
integrator (`CALL-F0`) owns shared files and integration; each lane owns only
the paths below.

| ID | Owner paths | Deliverable |
|---|---|---|
| CALL-P1 | `compiler/src/chips/parse/pa_tu.rs`, proposed `compiler/tests/call_parse.rs` | Generic multi-function parser producing the frozen shapes; preserve canonical/`/49`–`/53` trees; atomic failure; argument/parameter/function/nesting limits |
| CALL-S1 | `compiler/src/chips/types/ty_types.rs`, `compiler/src/chips/symbols/ty_scope.rs`, `compiler/src/chips/symbols/ty_symbol.rs`, `compiler/src/chips/semantic/{se_function,se_return,se_identifier,se_assignment}.rs`, new `se_call.rs` | Parameterized signatures, block-scope parameter symbols, enclosing-scope resolution, call checking, call facts; no shared schema edits |
| CALL-I1 | `compiler/src/chips/ir_lower/ir_function.rs`, proposed `compiler/tests/call_ir.rs` | Parameter bindings, ordered calls, callee symbols, nested expression lowering, per-function ownership |
| CALL-V1 | `compiler/src/chips/verify/{vf_interpret,vf_syntax,vf_invariant}.rs`, proposed `compiler/tests/call_verify.rs` | Multi-function AST/IR verification, explicit bounded call frames, per-call locals, typed failure |
| CALL-A1 | `compiler/src/bin/candidate/target_m1.rs`, `compiler/tests/c55_target_m1.rs` | Multi-function labels, `w0`–`w7` arguments, `w0` result, LR/save/restore, aligned frames, call clobbers, frame bounds |
| CALL-T1 | `compiler/tests/call_source.rs`, `compiler/tests/call_abi.rs` | Independent source/native and two-way ABI cases with hand-derived values |
| CALL-F0 | `compiler/src/bin/candidate.rs`, shared schema/registry files, contract/hash/docs | Freeze maintenance, orchestration, registration, integration, final evidence |

`CALL-S1` may add `se_call` to `mod.rs` in its worktree for compilation; the
integrator resolves module aggregation while merging. No other lane edits
`mod.rs`, `bus.rs`, `task.rs`, `manifest.rs`, `contract.rs`, `snapshot.rs`,
Cargo files, or candidate orchestration.

## 4. Acceptance

| Case | Expected |
|---|---|
| `add(2,3)` | modeled 5; native exit 5 |
| `add(inc(2),inc(3))` | modeled 7; native exit 7 |
| `add(x,3)` with `x=2` | modeled 5 |
| Eight distinct parameter values | a non-commutative result exposes reversal/drops |
| Nine parameters, wrong arity, unknown/later callee, duplicate parameter | explicit failure; no partial records |
| Recursion or mutual recursion | rejected by definition-order rule |
| Malformed Call/parameter AST | VF05/VF06/fail-closed diagnostics |
| Call depth/work/frame over limit | typed failure, never timeout/panic/success |
| Canonical and `/49`–`/53` sources | unchanged modeled and native behavior |
| Two-way ABI | candidate callee with reference caller and candidate caller with reference callee for the covered int subset |

Native checks use the pinned Linux AArch64 substrate and external
assembler/linker only; reference GCC must never compile the candidate function
under test. Linux-gated tests that return early on non-AArch64 hosts are not
execution evidence.

## 5. Verification Commands

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test freeze
cargo test --locked --manifest-path compiler/Cargo.toml --test call_parse
cargo test --locked --manifest-path compiler/Cargo.toml --test call_semantic
cargo test --locked --manifest-path compiler/Cargo.toml --test call_ir
cargo test --locked --manifest-path compiler/Cargo.toml --test call_verify
cargo test --locked --manifest-path compiler/Cargo.toml --test call_source
cargo test --locked --manifest-path compiler/Cargo.toml --test call_abi
cargo test --locked --manifest-path compiler/Cargo.toml --test c55_target_m1
cargo test --locked --manifest-path compiler/Cargo.toml --test c56_source_locals
cargo test --locked --manifest-path compiler/Cargo.toml
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings
```

The `call_*` targets are planned and do not exist at `/54`. The final `/55`
amendment, hash, test counts, and native evidence are recorded only after the
real producers and closed loop pass.
