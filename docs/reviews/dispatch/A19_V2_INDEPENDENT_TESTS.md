# A19 — V2 Independent Acceptance Suite: if/else, loops, break/continue, short-circuit, CFG

| Field | Value |
|---|---|
| Dispatch task | A-TST-02 (Stage A, Batch A2) |
| Status | **Design only — no implementation, no test execution, no probe, no freeze** |
| Contract baseline inspected | `t01-c01-c06/39` (`07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24`) |
| V2 source | [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) §6.2 |
| Primary authority | [AGENTS.md](../../../AGENTS.md), [T01](../../tasks/T01_COMPILER_CONTRACT.md), [PARALLEL_EXECUTION.md](../../tasks/PARALLEL_EXECUTION.md), [COMPILER_DEVELOPMENT_GUARDRAILS.md](../../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md) |
| Related packages | [T05](../../tasks/T05_PARSE_CHIPS.md), [T07](../../tasks/T07_SEMANTIC_CHIPS.md), [T09](../../tasks/T09_IR_LOWER_CHIPS.md), [T13](../../tasks/T13_VERIFICATION_CHIPS.md) |
| Target | `aarch64-unknown-linux-gnu` identity; concrete values **UNVERIFIED**; Part B is probe-gated |
| Write scope used | This report only. No source, test, contract, Cargo, `mod.rs`, or shared-index edit. |
| Executed evidence | **None.** No `cargo` command was run for this report. All statements below are design or read-only static inspection. |

---

## 1. Executive summary

This report designs an independent acceptance suite for the V2 integration train:
`if`/`else`, loops, `break`/`continue`, short-circuit `&&`/`||`, and explicit CFG.
It defines the layer split, file ownership, fixtures, expected values, negative and
replay cases, future commands, and pass/fail criteria. The design contains
**91 concrete fixtures** and **112 matrix rows** (16 parser, 20 negative, 20
semantic, 29 CFG, 27 integration/replay/target).

The suite is **not runnable today**. The frozen `/39` artifact has no V2 AST kinds,
task kinds, effects, CFG ops, CFG verifier registration, or multi-block interpreter.
The current M1 parser candidate explicitly rejects every `if`/`switch`/`for`/`while`/
`do` input as `Unsupported` (`compiler/src/chips/parse/pa_stmt.rs:31-34,71-74`), and
the current IR set is only `Constant` and `Return` (`compiler/src/bus.rs:442-447`).
V2 also needs the V1 base (local variables, initializers, assignment, loads/stores),
which is not integrated in the frozen artifact.

The design therefore has four gates:

1. **V2-0 contract freeze** — AST, semantic facts, effects, CFG/IR terminators,
   verifier registrations, diagnostics, manifests, and hash.
2. **V1 integration pass** — stateful loops require V1 locals and assignment.
3. **V2 implementation plus L1/L2/L3A acceptance** — synthetic, real-chain, and
   symbolic closed-loop evidence.
4. **L3T target execution** — only after H01/H07 probe and H04/H05 Host isolation
   exist; currently `BLOCKED_PROBE`/`BLOCKED_HOST`, never a pass by substitution.

A case is accepted only when its expected value is independently derived from C11
or a hand calculation, not from the implementation under test. Target-independent
V2 evidence cannot be replaced by, or promoted to, target execution evidence.

---

## 2. Scope, authority, and non-claims

### 2.1 V2 definition

[MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) §6.2 defines
V2 as:

> `if/else, loops, break/continue, short-circuit evaluation, and CFG` — real
> conditions and branches; constant interpretation must not impersonate all
> control flow.

This report interprets that definition as:

- statement forms `if`, `if`/`else`, `while`, `for`, `do`/`while`;
- `break` and `continue` with C11 target binding;
- short-circuit `&&` and `||`, including conditional RHS effects and `int` 0/1
  result normalization;
- explicit CFG blocks, conditional/unconditional terminators, loop back edges,
  join points, and CFG invariants;
- unreachable and dead code handling without suppressing semantic checks;
- real producer/consumer integration through the PP/LX/PA/TY/SE/IR chain.

### 2.2 In scope

- Target-independent (Part A) acceptance for the V2 feature set.
- L1 synthetic chip tests, L2 real-chain integration, L3A source-to-CFG symbolic
  closed loop.
- Negative, unsupported, recovery, replay, non-matching-task, and undeclared-write
  cases required by [T01 §5](../../tasks/T01_COMPILER_CONTRACT.md) and
  [MASS_PARALLEL §11](../../tasks/MASS_PARALLEL_DISPATCH.md).
- Target execution acceptance design, explicitly probe-gated.

### 2.3 Out of scope / non-goals

- `switch`/`case`/`default`, `goto`/labels, function calls, parameters, pointers,
  arrays, aggregates, floating point, variadics, VLAs, volatile/atomic semantics,
  old-style definitions, and GNU extensions. `switch`/`goto` appear only as
  explicit-unsupported boundary cases.
- Optimization and branch folding. Acceptance pins `OptLevel::O0`; branch folding
  requires the T10 legality/version/validation gates and is not V2 acceptance.
- AArch64 instruction selection, ABI classification, assembly emission, linking,
  and runtime execution. Those are L3T and probe-gated.
- Any claim that V2 is implemented, that a test passed, that a probe ran, or that
  a GCC torture rate exists.

### 2.4 Authority order

Per [AGENTS.md](../../../AGENTS.md) §2 and
[COMPILER_DEVELOPMENT_GUARDRAILS.md](../../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md)
§1: correctness, contract compliance, ownership, determinism, then speed. The
frozen `/39` contract is current; V2 candidate names in this report are **not**
frozen interfaces. Where this report proposes a shape, the integrator must freeze
it through the V2-0 contract amendment before any implementation or test compiles
against it.

### 2.5 Relationship to A07/A15/A17

A-PTS-02 (`docs/reviews/dispatch/A07_V2_CONTROL_FLOW_CONTRACT.md`) owns the V2
contract-requirement enumeration; A-PTS-05 (`A15_IR_CONTRACT_REQUIREMENTS.md`)
owns IR/CFG requirements; A-PTS-07 (`A17_VERIFICATION_REQUIREMENTS.md`) owns
verification requirements. This report owns the **independent acceptance design**
and must be reconciled with those reports at the V2-0 freeze. It does not
duplicate or pre-empt their contract decisions.

---

## 3. Baseline facts from read-only inspection

These are static facts about the inspected workspace, not test results.

| Fact | Evidence | Consequence for V2 |
|---|---|---|
| Frozen artifact is `/39`. | `compiler/contracts/CONTRACT_VERSION` | Every shared V2 shape needs a new authorized amendment/hash. |
| `NodeKind` has only 8 M1 variants: `TranslationUnit`, `FunctionDefinition`, `Specifiers`, `Declarator`, `Compound`, `Return`, `BinaryAdd`, `IntLiteral`. | `compiler/src/bus.rs:545-562` | No `If`, loop, `Break`, `Continue`, logical-op, or statement-expression node exists. |
| `IrOp` has only `Constant` and `Return`. | `compiler/src/bus.rs:442-447` | No branch, jump, compare, arithmetic, load/store, or join ops. |
| `InstructionRecord` has `op`, `block`, `operands`, `immediate`, `result`; no successor/edge field. | `compiler/src/bus.rs:483-497` | CFG edge representation must be frozen. |
| `SemRecord` has `node`, `ty`, `category`, `effects`; M1 permits only `EffectMask(0)`. | `compiler/src/bus.rs:504-521`; [T07](../../tasks/T07_SEMANTIC_CHIPS.md) rev 46 | V2 side effects and short-circuit effects need a nonzero-effect amendment. |
| `TypeKind` has `Void`, `Bool`, `Char`, `Int`, `Function`; no aggregate/pointer. | `compiler/src/bus.rs:298-323` | A non-scalar condition negative must use a synthetic `void` fixture; aggregate-condition tests wait for train D. |
| The frozen `TaskKind` registry's `PARSE` locals stop at 26 (`PARSE_RECOVERY`); `IR_LOWER` has only local 16 (`IR_FUNCTION`). Candidate files claim further locals but are not registered. | `compiler/src/task.rs:485,282` | No V2 task kind is registered or staged. |
| `STAGE_ASSIGNMENT` has no V2 control kind. | `compiler/src/manifest.rs:1444-1515` | V2 routing/stage/layer rows are absent. |
| `STORE_OWNER_ALLOWLIST` has no V2 CFG/control writer row. | `compiler/src/manifest.rs:784-990` | V2 writers would be rejected at registration. |
| The M1 parser candidate rejects all `if`/`switch`/`for`/`while`/`do` input as `Unsupported`. | `compiler/src/chips/parse/pa_stmt.rs:31-34,71-74` | Candidate files are not V2 evidence and must not be overwritten. |
| VF12 models one entry block with `Constant` + `Return` only and fails multi-block/extra ops. | `compiler/src/chips/verify/vf_interpret.rs:6-13,268-327` | VF12 must be extended or a new CFG interpreter frozen for L3A. |
| VF07 `CfgInvariantChip` is planned but out of M1; no registration exists. | [T13](../../tasks/T13_VERIFICATION_CHIPS.md):76,87 | V2 needs a VF07 registration/freeze; tests may also assert CFG invariants locally. |
| Target probe has not run; concrete ABI values are unverified; H04/H05 are not available. | `compiler/contracts/CONTRACT_VERSION`; [PARALLEL_EXECUTION.md](../../tasks/PARALLEL_EXECUTION.md) §1 | L3T stays `BLOCKED_PROBE`/`BLOCKED_HOST`. |
| The workspace has uncommitted parse/lex candidate files and freeze drafts. | `git status` at inspection time | They are read-only user work; V2 implementation must be a separate frozen work order. |

**Baseline conclusion:** no V2 test in this design can execute against `/39`. The
suite is a specification for the post-freeze implementation; running it today
would fail at compile time (missing task kinds/ops) or produce expected
`Unsupported` failures, not V2 acceptance.

---

## 4. Test architecture

### 4.1 Acceptance levels

| Level | Meaning | Allowed substitutes | Not a pass |
|---|---|---|---|
| **L0** | Contract/schema shape: kinds, stages, manifests, allowlists, snapshot codecs, hash. | Compile-time checks and registration rejection. | Registration success alone is not semantic correctness. |
| **L1** | Independent chip pass: pure projection/synthetic committed records; the semantic rule under test is never mocked. | Synthetic tokens, AST, SemRecords, IR records with hand-built expected facts. | A stub/Ack/`Unsupported` shell is not L1 success. |
| **L2** | Integration pass: real upstream producers; no seeded PP/PA/SE output for the feature under test. | Only the Host source bytes may be seeded. | Hand-built IR presented as parser/semantic evidence is not L2. |
| **L3A** | Part A closed loop: real source → committed CFG → symbolic interpretation/CFG verification → expected value. | Extended VF12 or frozen CFG interpreter; target-independent. | An IR constant folded to the expected answer without CFG is not L3A. |
| **L3T** | Target closed loop: candidate assembly/object → external assembler/linker → runner → exit status/assertion. | Only after H01/H07 probe and H04/H05 isolation. | Part A evidence, macOS execution, or an IR interpretation is not L3T. |

The V2 acceptance gate requires **L1 + L2 + L3A**. L3T is required only after the
probe substrate exists; until then it is `BLOCKED_PROBE`, not `PASS`.

### 4.2 Test files, fixtures, and ownership

| Path | Contents | Owner | Notes |
|---|---|---|---|
| `compiler/tests/v2_support/mod.rs` | Shared harness: bus setup, V2 registration, real-chain driver, snapshot/replay helpers, CFG walk helpers, assertion helpers. **No `#[test]` functions.** | Independent test owner (A-TST-02 design) | Not an integration-test target; loaded by `mod v2_support;`. |
| `compiler/tests/v2_support/fixtures.rs` | Inline `&'static [u8]` C source fixtures and expected-value tables. No file I/O. | Independent test owner | Single source of truth for all V2 fixtures; implementation owners must not edit expectations. |
| `compiler/tests/v2_parse.rs` | L1 parser/AST acceptance: if/else, dangling else, loop forms, jump nodes, scopes, recovery. | Independent test owner | Test target auto-discovered by Cargo. |
| `compiler/tests/v2_sem.rs` | L1 semantic acceptance: conditions, jump binding, short-circuit effects, evaluation order, unreachable checks. | Independent test owner | Test target auto-discovered. |
| `compiler/tests/v2_cfg.rs` | L1/L2 CFG and IR acceptance: terminators, edges, joins, VF07/VF08 positives and negatives. | Independent test owner | Test target auto-discovered. |
| `compiler/tests/v2_e2e.rs` | L1/L2/L3A integration, replay, non-matching-task, and undeclared-write cases. | Independent test owner | Test target auto-discovered; contains no probe-gated code. |
| `compiler/tests/v2_target.rs` | L3T probe-gated target execution cases. | Independent test owner | Created **only after** H01/H07 and H04/H05 exist; not part of the pre-probe gate, so absent Host APIs cannot break `v2_e2e` compilation. |

Ownership rules:

- The integrator owns `compiler/src/task.rs`, `manifest.rs`, `bus.rs`, chip IDs,
  stage rows, allowlists, registrations, `mod.rs`, Cargo files, and the contract
  version/hash. The V2 test author must not edit them.
- The V2 test author owns only the paths above (five targets plus two support
  files) after the integrator acknowledges the test entry points. `v2_target.rs`
  is deferred until the probe/Host gate is satisfied.
- Implementation owners must not change expected values, fixtures, or test
  assertions to make a delivery pass. A semantic dispute goes to the integrator.
- Fixtures are inline byte strings, so tests do not require new Cargo features,
  external files, environment variables, or Host I/O. This keeps L1/L2 hermetic.
- If a reviewer requires reviewable `.c` fixtures later, put them under
  `compiler/tests/fixtures/v2/` and use `include_bytes!`; that is a test-owner
  change, not an implementation shortcut.

### 4.3 Shared harness requirements

The harness must be thin and must not hide language rules. It may seed source
bytes, drive tasks, capture snapshots, and traverse committed records; it must not
compute C semantics, expected values, or CFG decisions. Proposed API:

```rust
pub struct V2Ast {
    pub bus: CompilerBus,
    pub tu: NodeId,
    pub function: NodeId,
}

pub struct V2Sem {
    pub bus: CompilerBus,
    pub tu: NodeId,
    pub function: NodeId,
}

pub struct V2Cfg {
    pub bus: CompilerBus,
    pub tu: NodeId,
    pub function: NodeId,
    pub ir: FunctionId,
}

pub fn new_bus() -> CompilerBus;                    // frozen fixture limits/config
pub fn install_v2(bus: &mut CompilerBus);           // all V2 chips, routes, stages
pub fn run_to_ast(source: &[u8]) -> V2Ast;          // real PP + LX + PA only
pub fn run_to_sem(source: &[u8]) -> V2Sem;          // real PP + LX + PA + TY + SE
pub fn run_to_cfg(source: &[u8]) -> V2Cfg;          // full real chain to IR
pub fn expect_part_a_value(chain: &V2Cfg, expected: u64);
pub fn assert_cfg_invariants(chain: &V2Cfg);        // independent local graph checks
pub fn run_vf07(chain: &V2Cfg) -> Proposal;         // frozen verifier task
pub fn snapshot(bus: &CompilerBus) -> Snapshot;
pub fn assert_replay(source: &[u8], ticks: usize);
pub fn expect_failure(report: &TickReport, group: DiagGroup);
pub fn assert_no_partial_state(before: &Snapshot, after: &Snapshot);
```

`run_to_ast`, `run_to_sem`, and `run_to_cfg` must drive real producer tasks and
must not seed PP tokens, AST nodes, SemRecords, or IR for positive cases. L1
tests may call pure `compute`/projection helpers directly with synthetic records;
that is the only place mocks are allowed, and only for the input side.

Because Cargo compiles `v2_support` separately into each test target, helpers used
by only some targets can otherwise trip `dead_code` under `-D warnings`. The
support module should carry a narrow, documented `#![allow(dead_code)]` (or
per-item allows), explicitly scoped to test-harness plumbing; this must not be
used to silence warnings in V2 chip code or tests' assertions.

### 4.4 Real producer/consumer chain

The L2/L3 chain is:

```text
source bytes
  -> PP01 normalize -> splice -> comment -> scan        (real PP artifacts)
  -> LX intern -> classify -> decode                    (real tokens/literals)
  -> V2 PA if/loop/jump productions                     (real AST nodes)
  -> TY scopes/declarations/conversions                 (real symbols/types)
  -> T07 condition/jump/short-circuit checks            (real SemRecords)
  -> T09 if/loop/short-circuit/jump/function-end         (real CFG/IR)
  -> VF06/VF07 typed/CFG verification                   (real verifier tasks)
  -> VF12 extended CFG interpretation                   (L3A expected value)
```

The current M1 tests seed PP tokens for convenience; V2 must not copy that
shortcut for positive integration cases. L2 starts from source bytes and uses the
real PP/LX chain, as the `/16` source-bytes test already demonstrates
(`compiler/tests/c16_pp.rs:332-390`).

### 4.5 Oracle policy

Expected values come from:

1. **C11 rules** — the clause citations in §7 and the fixture catalog.
2. **Hand evaluation** — arithmetic and control-flow traces written in this
   report, independent of the compiler.
3. **Frozen CFG contract** — structural properties such as “every block has one
   terminator” and “branch targets exist,” not a dump of the implementation's
   block list.

The suite must not:

- read expected values from the implementation under test;
- compare against a reference compiler's generated code or IR for candidate
  acceptance (T00 reference evidence, even if later available, is not candidate
  evidence);
- assert diagnostic message text; it asserts the frozen diagnostic group and
  stable rule ID;
- treat `Unsupported` as a pass for an in-scope V2 construct.

Unspecified/undefined C behavior is not used as a positive fixture. For example,
`i++ + i++` is not asserted; V2 fixtures use sequenced `&&`/`||`/assignment forms
where C11 defines the result. The T07 rule “do not fix UB to a wrong result” is
respected.

### 4.6 Replay and negative-state methodology

Every test that mutates a bus uses this pattern:

1. capture `Snapshot::capture(&bus)` before the tick or chain;
2. run the task/chain;
3. assert the expected terminal state;
4. for negative cases, assert the snapshot after the rejected tick equals the
   before-snapshot (no partial append, no task mutation, no diagnostic duplicate).

Replay tests run a fresh bus twice with identical source, config, limits, and
pins, and compare snapshots after each tick plus the final IR. A failed run is
also replayed; diagnostic order/codes, task states, and snapshots must match.
Dispatch-order replay (VF13/H6) is a separate integration obligation and is not
claimed by basic V2 replay.

Every harness-driven loop and task chain has a configured tick/step bound. Bound
exhaustion, a task left `Running`/`Waiting` forever, or a test-runner timeout is
a **failure**, never a pass. This is especially important for `continue`/`break`
miscompilations that would otherwise produce an infinite loop (`F-LP-09`,
`F-LP-14`).

---

## 5. V2-0 contract freeze prerequisites

All rows are `BLOCKED_CONTRACT` until the integrator freezes them. Candidate
names are descriptive, not API commitments.

| ID | Required freeze item | Why V2 needs it | Owner |
|---|---|---|---|
| V2-0.1 | `NodeKind` additions for `If`, `While`, `For`, `DoWhile`, `Break`, `Continue`, `LogicalAnd`, `LogicalOr` (or a frozen generic binary-op node with an explicit operator field). | Parser/AST acceptance cannot name or traverse V2 statements. | T05 + T01 |
| V2-0.2 | Exact child order/optional-child encoding for `if`-then/else, loop init/condition/step/body, and jump leaves; token ranges for multi-token statements. | With only `children: Vec<NodeId>`, optional `else` and omitted `for` clauses are ambiguous. | T05 + T01 |
| V2-0.3 | Parser task kinds, stage rows, chip IDs, manifests, and allowlist rows for V2 productions. | No V2 kind/stage/owner exists today. | T05 + T01 |
| V2-0.4 | Semantic facts for condition truth conversion, `break`/`continue` target, loop context, short-circuit sequencing, and unreachable-code checking. | `SemRecord` has no jump-target/control fact carrier. | T07 + T01 |
| V2-0.5 | `EffectMask` nonzero bit allocation and semantics for assignment, increment, calls (future), volatile (future), and control sequencing. | Current M1 permits only `EffectMask(0)`; V2 loop state requires effects. | T07 + T01 |
| V2-0.6 | IR `IrOp`/terminator additions for conditional branch, unconditional jump, compare/truth, and function end; V1-provided arithmetic/assignment/load/store ops are consumed where already frozen. Exact successor/edge representation must be frozen. | Current IR has only `Constant`/`Return` and no edge field. | T09 + T01 |
| V2-0.7 | CFG invariants: unique entry, block ownership, terminator uniqueness/last, target existence, cross-function prohibition, reachability/dead-block policy, join-slot definition. | Needed by VF07 and by the local CFG assertions. | T09 + T13 + T01 |
| V2-0.8 | VF07 `CfgInvariantChip` registration: task kind, stage, layer, input/result carrier, rule IDs. | VF07 is out of M1 and unregistered. | T13 + T01 |
| V2-0.9 | VF08 `IrInvariantChip` registration or an explicitly frozen equivalent for operand/type/join checks. | CFG correctness alone does not prove operand typing. | T13 + T01 |
| V2-0.10 | VF12 extension (or a new frozen CFG interpreter) covering V2 control ops, locals/loads/stores, arithmetic, assignment, and explicit `Unsupported` for uncovered ops. The `Record(Const)` result is a **modeled value, not a PASS verdict**. | Current VF12 fails any multi-block/extra-op function. | T13 + T01 |
| V2-0.11 | Diagnostic groups/rule IDs for parse, semantic, unsupported, CFG, and IR control errors; no message-text contract. | Tests must assert stable fail reasons. | T01 + package owners |
| V2-0.12 | Snapshot codecs for new node/IR/CFG records and the contract hash bump. | C05 coverage/replay otherwise cannot see V2 state. | T01 |

The V2-0 freeze must use the complete `CONTRACT_CHANGE_REQUEST` fields from
[COMPILER_DEVELOPMENT_GUARDRAILS.md](../../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md)
§3.4, including affected chips/tests, read/write manifest impact, determinism and
snapshot/hash impact, and the migration/freeze plan. It must also state whether
`switch`, `goto`, `?:`, and comma are explicitly unsupported in V2 or belong to
another train. This report assumes they are out of scope; if the integrator
chooses otherwise, the unsupported boundary cases in §7.2 must be reclassified,
not silently passed.

---

## 6. Fixture catalog

All sources below are wrapped as `int main(void){ ... }`. The harness config is
`Dialect::C11`, `OptLevel::O0`, the frozen unverified `aarch64-unknown-linux-gnu`
identity, and `Limits::fixture()`. Expected values are Part A hand calculations
over symbolic `int`; they do not depend on a concrete target width. L3T must
produce the same exit status on the verified target, and only those target
observations are probe-gated. No fixture uses implementation-defined or undefined
behavior, and all expected exit values are in 0..91, safely below the POSIX
8-bit status range; status truncation is not exercised.

### 6.1 `if`/`else` fixtures

- **F-IF-01** `if(1)return 7;return 9;` — expected **7**. Basis: 6.8.4.1.
- **F-IF-02** `if(0)return 7;return 9;` — expected **9**. Basis: 6.8.4.1.
- **F-IF-03** `if(1)return 7;else return 9;` — expected **7**. Basis: 6.8.4.1.
- **F-IF-04** `if(0)return 7;else return 9;` — expected **9**. Basis: 6.8.4.1.
- **F-IF-05** `if(1)if(0)return 1;else return 2;return 3;` — expected **2**. `else` binds the inner `if`. Basis: 6.8.4.1p3.
- **F-IF-06** `if(1)if(1)return 1;else return 2;return 3;` — expected **1**. Basis: 6.8.4.1p3.
- **F-IF-07** `if(0)if(1)return 1;else return 2;return 3;` — expected **3**. The outer condition is false; no outer `else` exists. Basis: 6.8.4.1p3.
- **F-IF-08** `if(1){if(0)return 1;}else return 2;return 3;` — expected **3**. Braces force the `else` to bind the outer `if`. Basis: 6.8.2, 6.8.4.1p3.
- **F-IF-09** `if(0)return 1;else if(1)return 2;else return 3;` — expected **2**. Basis: 6.8.4.1.
- **F-IF-10** `if(0)return 1;else if(0)return 2;else return 3;` — expected **3**. Basis: 6.8.4.1.
- **F-IF-11** `if(0);else;return 4;` — expected **4**. Empty statements are valid. Basis: 6.8.3.
- **F-IF-12** `if(0)return 1;` — expected **0**. Reaching `}` of `main` returns 0. Basis: 5.1.2.2.3.
- **F-IF-13** `if(1)return 1;else return 2;return 3;` — expected **1**; the trailing `return 3` is unreachable but valid. Basis: 6.8.4.1, 6.8.3.
- **F-IF-14** `int x=0;if((x=x+1)==1)return x;return 9;` — expected **1**. The condition assignment is evaluated once. Basis: 6.8.4.1, 6.5.16p3.
- **F-IF-15** `int x=2;if(x)return 5;return 6;` — expected **5**. Nonzero scalar is true. Basis: 6.8.4.1p2.
- **F-IF-16** `int x=0;if(x)return 5;return 6;` — expected **6**. Zero is false. Basis: 6.8.4.1p2.

### 6.2 Loop fixtures

- **F-LP-01** `int i=0,s=0;while(i<3){s=s+i;i=i+1;}return s;` — expected **3**. Basis: 6.8.5.1.
- **F-LP-02** `int s=0;while(0){s=1;}return s;` — expected **0**; zero-iteration boundary. Basis: 6.8.5.1.
- **F-LP-03** `int s=0;for(int i=0;i<3;i=i+1){s=s+i;}return s;` — expected **3**. Basis: 6.8.5.3.
- **F-LP-04** `int i=0,s=0;for(;i<3;){s=s+i;i=i+1;}return s;` — expected **3**; omitted init/step. Basis: 6.8.5.3.
- **F-LP-05** `int i=0;for(;;){i=i+1;if(i==3)break;}return i;` — expected **3**; omitted condition is “true”. Basis: 6.8.5.3p2, 6.8.6.3.
- **F-LP-06** `int i=0,s=0;do{s=s+i;i=i+1;}while(i<3);return s;` — expected **3**. Basis: 6.8.5.2.
- **F-LP-07** `int i=0;do{i=i+1;}while(0);return i;` — expected **1**; `do` executes once. Basis: 6.8.5.2.
- **F-LP-08** `int s=0;for(int i=0;i<5;i=i+1){if(i==2)continue;s=s+i;}return s;` — expected **8** (`0+1+3+4`). Basis: 6.8.6.2.
- **F-LP-09** `int i=0,s=0;for(;i<3;i=i+1){if(i==1)continue;s=s+1;}return i*10+s;` — expected **32** (`i==3`, `s==2`); `continue` must execute the `for` step. Basis: 6.8.6.2p2, 6.8.5.3p5.
- **F-LP-10** `int i=0,s=0;while(i<5){i=i+1;if(i==2||i==4)continue;s=s+i;}return s;` — expected **9** (`1+3+5`). Basis: 6.8.6.2p2.
- **F-LP-11** `int i=0,s=0;do{i=i+1;if(i<3)continue;s=s+i;}while(i<5);return s;` — expected **12** (`3+4+5`); `do` `continue` goes to the condition. Basis: 6.8.6.2p2, 6.8.5.2.
- **F-LP-12** `int s=0;for(int i=0;i<2;i=i+1){for(int j=0;j<2;j=j+1){s=s+1;}}return s;` — expected **4**. Basis: 6.8.5.3.
- **F-LP-13** `int s=0;for(int i=0;i<3;i=i+1){for(int j=0;j<3;j=j+1){if(j==1)continue;if(i==2)break;s=s+1;}}return s;` — expected **4**. Hand trace: `i=0`: `j=0` makes `s=1`, `j=1` continues, `j=2` makes `s=2`; `i=1`: `j=0` makes `s=3`, `j=1` continues, `j=2` makes `s=4`; `i=2`: `j=0` breaks before `s=s+1`. Return 4. Basis: 6.8.6.2/6.8.6.3.
- **F-LP-14** `int i=0;for(;i<10;i=i+1){if(i==2)break;}return i;` — expected **2**; `break` skips the step. Basis: 6.8.6.3p2.
- **F-LP-15** `int i=9,s=0;for(int i=0;i<2;i=i+1){s=s+i;}return i*10+s;` — expected **91**; the `for` declaration shadows the outer `i`. Basis: 6.2.1p4, 6.8.5.3p1.
- **F-LP-16** `int i=0;while((i=i+1)<3);return i;` — expected **3**; the condition is re-evaluated each iteration. Basis: 6.8.5.1.
- **F-LP-17** `int i=0;while(i<3)i=i+1;return i;` — expected **3**; empty/one-statement body. Basis: 6.8.5.1.
- **F-LP-18** `int i=0,s=0;for(i=0;i<3;i=i+1)s=s+i;return s*10+i;` — expected **33**. Basis: 6.8.5.3.
- **F-LP-19** `int i=0,s=0;do{s=s+i;i=i+1;}while(i<3);return s*10+i;` — expected **33**. Basis: 6.8.5.2.

### 6.3 Short-circuit fixtures

- **F-SC-01** `return 2&&3;` — expected **1**. Basis: 6.5.13p3.
- **F-SC-02** `return 0&&3;` — expected **0**. Basis: 6.5.13p3.
- **F-SC-03** `return 2||0;` — expected **1**. Basis: 6.5.14p3.
- **F-SC-04** `return 0||0;` — expected **0**. Basis: 6.5.14p3.
- **F-SC-05** `return 1&&2&&3;` — expected **1**. Basis: 6.5.13p3.
- **F-SC-06** `return 0||0||4;` — expected **1**. Basis: 6.5.14p3.
- **F-SC-07** `return 1||0&&0;` — expected **1**; `&&` binds tighter than `||`. Basis: 6.5.13, 6.5.14, C11 grammar.
- **F-SC-08** `int x=0,y=0;if((x=1)&&(y=2))return x+y;return 0;` — expected **3**. Basis: 6.5.13p3, 6.5.16p3.
- **F-SC-09** `int x=0,y=0;if((x=0)&&(y=2))return 99;return x*10+y;` — expected **0**; RHS `y=2` is not evaluated. Basis: 6.5.13p3.
- **F-SC-10** `int x=0,y=0;if((x=1)||(y=2))return x*10+y;return 99;` — expected **10**; RHS `y=2` is not evaluated. Basis: 6.5.14p3.
- **F-SC-11** `int x=0,y=0;if((x=0)||(y=2))return x*10+y;return 99;` — expected **2**. Basis: 6.5.14p3.
- **F-SC-12** `int a=0,b=0,c=0;if((a=1)&&((b=0)||(c=3)))return a+b+c;return 9;` — expected **4**. Basis: 6.5.13p3, 6.5.14p3.
- **F-SC-13** `int a=0,b=0;if((a=0)&&((b=1)||(a=2)))return 9;return a*10+b;` — expected **0**; the whole inner `||` is not evaluated. Basis: 6.5.13p3.
- **F-SC-14** `int i=0;while(i<3&&(i=i+1)){}return i;` — expected **3**; the RHS is skipped when `i<3` is false. Basis: 6.5.13p3, 6.8.5.1.
- **F-SC-15** `int x=0;if((x=1)&&(x=x+1))return x;return 9;` — expected **2**; LHS is sequenced before RHS. Basis: 6.5.13p3.
- **F-SC-16** `int x=0;if((x=0)||(x=x+2))return x;return 9;` — expected **2**; LHS is sequenced before RHS. Basis: 6.5.14p3.
- **F-SC-17** `int x=0;if((x=0)&&(x=x+1))return 9;return x;` — expected **0**; RHS not evaluated. Basis: 6.5.13p3.
- **F-SC-18** `int x=0;if((x=1)||(x=x+2))return x;return 9;` — expected **1**; RHS not evaluated. Basis: 6.5.14p3.

### 6.4 Unreachable and semantic-check fixtures

- **F-UN-01** `return 1;return 2;` — expected **1**; the second return is unreachable but valid. Basis: 6.8.3, 6.9.1p12.
- **F-UN-02** `if(0)return 1;return 2;` — expected **2**. Basis: 6.8.4.1.
- **F-UN-03** `while(0)return 1;return 2;` — expected **2**. Basis: 6.8.5.1.
- **F-UN-04** `while(1){break;}return 1;return 2;` — expected **1**. Basis: 6.8.6.3.
- **F-UN-05** `if(0){break;}return 1;` — expected **diagnostic**; `break` outside loop/switch is a constraint violation even though unreachable. Basis: 6.8.6.3p1.
- **F-UN-06** `if(0){continue;}return 1;` — expected **diagnostic**; `continue` outside loop. Basis: 6.8.6.2p1.
- **F-UN-07** `if(0){return x;}return 1;` — expected **diagnostic**; undeclared `x` is a constraint violation even in dead code. Basis: 6.2.1 (scope), 6.5.1 (primary expressions).
- **F-UN-08** `return 1;break;` — expected **diagnostic**; unreachable `break` outside loop/switch. Basis: 6.8.6.3p1.
- **F-UN-09** `while(0){continue;}return 1;` — expected **1**; `continue` is inside the `while` body and is valid, though unreachable. Basis: 6.8.6.2p1.
- **F-UN-10** `while(1){break;return 1;}return 2;` — expected **2**; `return 1` is unreachable. Basis: 6.8.6.3, 6.8.3.

### 6.5 Evaluation-order fixtures

- **F-EFF-01** `int i=0,s=0;for(i=0;i<3;i=i+1)s=s+i;return s*10+i;` — expected **33**. If the step ran before the body, `s` would be 6; the fixture distinguishes.
- **F-EFF-02** `int i=0,s=0;while(i<3){s=s+i;i=i+1;}return s*10+i;` — expected **33**.
- **F-EFF-03** `int i=0,s=0;do{s=s+i;i=i+1;}while(i<3);return s*10+i;` — expected **33**.
- **F-EFF-04** `int x=0;while(x<3){x=x+1;}return x;` — expected **3**; body effect persists across iterations.
- **F-EFF-05** `int x=0;if((x=0)&&(x=x+1))return 9;return x;` — expected **0**; short-circuit suppresses the RHS effect.
- **F-EFF-06** `int x=0;if((x=1)||(x=x+2))return x;return 9;` — expected **1**; short-circuit suppresses the RHS effect.
- **F-EFF-07** `int x=0;if((x=1)&&(x=x+1))return x;return 9;` — expected **2**; both effects occur in sequence.
- **F-EFF-08** `int x=0;if((x=0)||(x=x+2))return x;return 9;` — expected **2**; both effects occur in sequence.

### 6.6 Syntax and semantic negative fixtures

These are wrapped as `int main(void){ ... }`; each expected outcome is a typed
failure, not a fabricated success.

- **F-NEG-01** `if 1 return 1;` — missing parentheses; parse failure. Basis: 6.8.4.1 grammar.
- **F-NEG-02** `if(1 return 1;` — missing `)`; parse failure.
- **F-NEG-03** `else return 1;` — `else` without `if`; parse failure.
- **F-NEG-04** `if(1)return 1;else return 2;else return 3;` — duplicate `else`; parse failure.
- **F-NEG-05** `while()return 1;` — missing condition; parse failure. Basis: 6.8.5.1.
- **F-NEG-06** `for()return 1;` — malformed `for` header; parse failure. Basis: 6.8.5.3.
- **F-NEG-07** `do{return 1;}` — missing `while` clause; parse failure. Basis: 6.8.5.2.
- **F-NEG-08** `do{return 1;}while(0)` — missing trailing `;`; parse failure. Basis: 6.8.5.2p1.
- **F-NEG-09** `while(1)return 1` — missing statement `;`; parse failure.
- **F-NEG-10** `if(1){return 1;` — missing `}`; parse failure.
- **F-NEG-11** `if(1)return 1` — missing `;` at EOF; parse failure.
- **F-NEG-12** `for(int i=0 i<1;i=i+1){}` — missing `;` in the `for` header; parse failure.
- **F-NEG-13** `break;` — outside loop/switch; semantic failure. Basis: 6.8.6.3p1.
- **F-NEG-14** `continue;` — outside loop; semantic failure. Basis: 6.8.6.2p1.
- **F-NEG-15** `for(int i=0;i<1;i=i+1);return i;` — `i` out of scope after the loop; semantic failure. Basis: 6.2.1p4, 6.8.5.3p1.
- **F-NEG-16** `if(0){break;}return 1;` — unreachable `break` is still invalid. Basis: 6.8.6.3p1.
- **F-NEG-17** `switch(1){case 1:return 1;}return 2;` — V2 scope boundary: explicit `Unsupported`, never a silent pass. (If a later train supports `switch`, this row is reclassified, not passed as V2.) Basis: 6.8.4.2.
- **F-NEG-18** `goto L;L:return 1;` — V2 scope boundary: explicit `Unsupported`, never a silent pass. (If a later train supports `goto`, reclassify.) Basis: 6.8.6.1.
- **F-NEG-19** `if(1)return 1 return 2;` — missing `;` after the first return; recovery must report one diagnostic, must not silently accept or swallow the second statement. Basis: 6.8.3, 6.8.6.3.
- **F-NEG-20** `if(1){return 1; return 2;` — unbalanced brace; parse failure; no fabricated IR.

### 6.7 Synthetic CFG fixtures

These are constructed directly from frozen V2 IR constructors; they have no C
source and are L1 verifier tests.

| ID | Construction | Expected |
|---|---|---|
| SCFG-POS-IF | Entry condition block with true/false successors; both arms join; one return. | VF07/VF08 accept; both arms reachable. |
| SCFG-POS-WHILE | Entry condition block; body edge back to condition; false edge to exit. | VF07 accept; exactly one back edge to the condition. |
| SCFG-POS-FOR | Init block; condition block; body; step; step back to condition; exit. | VF07 accept; `continue` edge targets step. |
| SCFG-POS-DO | Body block; condition block; true edge back to body; false edge to exit. | VF07 accept; `continue` targets condition. |
| SCFG-POS-SC | LHS block; true/false successors; RHS block conditional; join slot. | VF07/VF08 accept; RHS block has no entry path bypassing the LHS true edge. |
| SCFG-NEG-DANGLING | Conditional terminator names a missing `BlockId`. | VF07 `Fail` with a stable dangling-target rule. |
| SCFG-NEG-CROSS-FN | Branch target is a block owned by a different function. | VF07 `Fail` with a stable cross-function rule. |
| SCFG-NEG-NO-ENTRY | Function entry names a missing block. | VF07 `Fail`. |
| SCFG-NEG-DUP-TERMINATOR | A block has two terminators or an instruction after its terminator. | VF07 `Fail`. |
| SCFG-NEG-APPEND-AFTER-END | Append an instruction to an already terminated block. | Commit or VF07 `Fail`; no silent append. |
| SCFG-NEG-JOIN-UNDEF | Join value slot is defined on only one incoming path. | VF08 `Fail` (or frozen equivalent). |
| SCFG-NEG-WRONG-TYPE | Branch condition or returned value has a non-`int`/wrong type. | VF08 `Fail`. |
| SCFG-DEAD-BLOCK | An unreachable block is present but no reachable edge enters it. | VF07 accepts or reports it explicitly; it must not fail solely because the block is unreachable. |

---

## 7. Case matrix

Legend: **Gate** lists additional blockers beyond V2-0. `L1`/`L2`/`L3A`/`L3T`
are the acceptance levels from §4.1. The **Basis** column is the independent
oracle: a C11 clause, a hand calculation, or the frozen CFG contract. The
**Fixture** column is the input; the **Expected** column is the pass condition.

### 7.1 Parser / AST acceptance (L1)

| ID | Fixture | Expected structural fact | Basis | Gate |
|---|---|---|---|---|
| P-01 | F-IF-01/02 | `If` node with one then-child and no else-child; parent/child reciprocity; token range covers the whole `if` statement. | C11 6.8.4.1 grammar; T05 PA29 | — |
| P-02 | F-IF-03/04 | `If` node with then and else children in frozen order. | 6.8.4.1 | — |
| P-03 | F-IF-05 | `else` is the else-child of the **inner** `If`; outer `If` has no else-child. | 6.8.4.1p3 | — |
| P-04 | F-IF-08 | Braces make the `else` the else-child of the outer `If`; inner `If` has no else. | 6.8.4.1p3 | — |
| P-05 | F-IF-09/10 | Right-nested `else if` chain; each `else` binds the immediately preceding `if`. | 6.8.4.1 | — |
| P-06 | F-IF-11 | Empty then/else statements are `Empty`/expression statements, not omitted children. | 6.8.3 | — |
| P-07 | F-LP-01/02/16/17 | `While` node with condition and body children; empty/one-statement body preserved. | 6.8.5.1 | — |
| P-08 | F-LP-03/18 | `For` node with init/condition/step/body; declaration-in-init is parsed in the loop scope. | 6.8.5.3p1 | V1 |
| P-09 | F-LP-04/05 | Omitted `for` clauses are represented explicitly (absent, not guessed); omitted condition is the canonical “true” form. | 6.8.5.3p2 | — |
| P-10 | F-LP-06/07 | `DoWhile` node with body then condition and the trailing `;` consumed. | 6.8.5.2 | — |
| P-11 | F-LP-12/13 | Nested loop nodes preserve parent/child nesting and inner-loop scope. | 6.8.5.3 | V1 |
| P-12 | F-LP-08/10/13 | `Break`/`Continue` are jump nodes with no resolved target at parse time; target resolution is semantic. | 6.8.6.2/6.8.6.3 | — |
| P-13 | F-LP-15 | `for` declaration scope shadows the outer `i` only inside the loop. | 6.2.1p4, 6.8.5.3p1 | V1 |
| P-14 | F-IF-12 | `main` with no reachable return parses as a complete function body; falloff is semantic, not a parse error. | 5.1.2.2.3 | — |
| P-15 | F-UN-01/10 | Unreachable statements are still parsed and retained; no parser “dead-code skip.” | 6.8.3 | — |
| P-16 | F-NEG-19 | Recovery after a missing `;` synchronizes at a delimiter, emits exactly one diagnostic, and does not swallow the following statement. | T05 PA38; 6.8.3 | — |

Parser test functions (proposed names; one per row):

```text
v2_parse.rs
  p01_if_without_else_shape
  p02_if_else_shape
  p03_dangling_else_binds_nearest
  p04_braces_bind_outer_else
  p05_else_if_chain
  p06_empty_then_else
  p07_while_shape_and_empty_body
  p08_for_full_header_and_scope
  p09_for_omitted_clauses
  p10_do_while_shape
  p11_nested_loops_and_scopes
  p12_break_continue_are_jump_nodes
  p13_for_declaration_shadows_outer
  p14_main_falloff_is_not_parse_error
  p15_unreachable_statements_retained
  p16_missing_semicolon_recovery
```

### 7.2 Parser / semantic negative acceptance (L1)

| ID | Fixture | Expected failure | Basis | Gate |
|---|---|---|---|---|
| N-01 | F-NEG-01 | Parse diagnostic; no `If` node committed for this function. | 6.8.4.1 grammar | — |
| N-02 | F-NEG-02 | Parse diagnostic; no partial acceptance. | 6.8.4.1 grammar | — |
| N-03 | F-NEG-03 | Parse diagnostic; `else` cannot start a statement. | 6.8.4.1 grammar | — |
| N-04 | F-NEG-04 | Parse diagnostic for the second `else`; first `if/else` may be retained but no IR is emitted. | 6.8.4.1 grammar | — |
| N-05 | F-NEG-05 | Parse diagnostic; empty `while` condition is not an implicit true. | 6.8.5.1 | — |
| N-06 | F-NEG-06 | Parse diagnostic; malformed `for` header. | 6.8.5.3 | — |
| N-07 | F-NEG-07 | Parse diagnostic; missing `while` clause. | 6.8.5.2 | — |
| N-08 | F-NEG-08 | Parse diagnostic; missing trailing `;`. | 6.8.5.2p1 | — |
| N-09 | F-NEG-09 | Parse diagnostic; missing statement terminator. | 6.8.3 | — |
| N-10 | F-NEG-10/20 | Parse diagnostic; unbalanced brace; no fabricated IR. | 6.8.2 | — |
| N-11 | F-NEG-11 | Parse diagnostic; truncated `if` at EOF. | 6.8.4.1 | — |
| N-12 | F-NEG-12 | Parse diagnostic; malformed `for` header. | 6.8.5.3 | — |
| N-13 | F-NEG-13 | Semantic diagnostic; `break` target missing. | 6.8.6.3p1 | — |
| N-14 | F-NEG-14 | Semantic diagnostic; `continue` target missing. | 6.8.6.2p1 | — |
| N-15 | F-NEG-15 | Semantic diagnostic; `i` out of scope after the loop. | 6.2.1p4 | V1 |
| N-16 | F-NEG-16 | Semantic diagnostic even though the `break` is unreachable. | 6.8.6.3p1 | — |
| N-17 | F-NEG-17 | Explicit `Unsupported`; never a pass, never a silent switch acceptance. | V2 scope; 6.8.4.2 | — |
| N-18 | F-NEG-18 | Explicit `Unsupported`; never a pass. | V2 scope; 6.8.6.1 | — |
| N-19 | F-UN-07 | Undeclared `x` is diagnosed even in dead code. | 6.2.1, 6.5.1 | V1 |
| N-20 | F-UN-05/06 | Unreachable invalid jump is diagnosed, not skipped by constant evaluation. | 6.8.6.2/6.8.6.3 | — |

Negative-test acceptance checks:

- the task becomes `Failed` exactly once, or the batch is rejected as a whole;
  it never becomes `Completed(Ack)` and never remains `Running`;
- exactly one primary diagnostic per primary error (recovery may add a note only
  if the frozen taxonomy allows it);
- no AST/SemRecord/IR append for the rejected function;
- the snapshot after the rejection equals the before-snapshot for all persistent
  semantic stores;
- no panic, no infinite retry, no swallowed error.

### 7.3 Semantic acceptance (L1)

| ID | Fixture | Expected semantic fact | Basis | Gate |
|---|---|---|---|---|
| S-01 | F-IF-15/16 | Condition truth requires a scalar type; integer 0 is false, nonzero is true. | 6.8.4.1p2 | — |
| S-02 | F-IF-14 | Condition expression has type `int`; its side effect is evaluated once. | 6.8.4.1, 6.5.16p3 | V1 |
| S-03 | F-SC-01/02 | `&&` result is `int` 0/1; operands must be scalar. | 6.5.13p3 | — |
| S-04 | F-SC-03/04 | `\|\|` result is `int` 0/1; operands must be scalar. | 6.5.14p3 | — |
| S-05 | F-SC-15/17 | LHS sequenced before RHS for `&&`; RHS has a conditional effect. | 6.5.13p3 | V1 |
| S-06 | F-SC-16/18 | LHS sequenced before RHS for `\|\|`; RHS has a conditional effect. | 6.5.14p3 | V1 |
| S-07 | F-SC-09/10 | RHS effect is absent when short-circuit selects the LHS. | 6.5.13/6.5.14 | V1 |
| S-08 | F-SC-08/11 | RHS effect is present when the LHS requires it. | 6.5.13/6.5.14 | V1 |
| S-09 | F-LP-08/10 | `continue` target is the nearest enclosing loop, not an `if` or block. | 6.8.6.2p2 | — |
| S-10 | F-LP-09 | `continue` in `for` targets the step expression. | 6.8.6.2p2, 6.8.5.3p5 | — |
| S-11 | F-LP-11 | `continue` in `do` targets the condition. | 6.8.6.2p2, 6.8.5.2 | — |
| S-12 | F-LP-13 | Nested `break`/`continue` bind the innermost loop; outer loop state is unaffected except through body completion. | 6.8.6.2/6.8.6.3 | — |
| S-13 | F-LP-14 | `break` exits the loop without running the `for` step. | 6.8.6.3p2 | — |
| S-14 | F-LP-16 | Loop condition is re-evaluated each iteration and its effects persist. | 6.8.5.1 | V1 |
| S-15 | F-EFF-01/02/03 | `for` init once, condition before body, body, step; `while`/`do` order matches C11. | 6.8.5.3p5, 6.8.5.1, 6.8.5.2 | — |
| S-16 | F-IF-12 | `main` falloff is allowed and yields 0; the semantic stage records the implicit return, not an error. | 5.1.2.2.3 | — |
| S-17 | F-UN-01/09 | Unreachable statements are still checked and may be retained; no semantic skip. | C11 constraint clauses | — |
| S-18 | F-UN-05/06/07 | Invalid jump/name use in dead code is diagnosed. | 6.8.6.2/6.8.6.3, 6.2.1 | — |
| S-19 | F-SC-07 | `&&`/`\|\|` precedence is reflected in the typed AST, not re-derived by T09. | C11 grammar; T09 accepted dependency | — |
| S-20 | F-LP-15 | `for` declaration is a new scope; the shadowed outer object remains distinct. | 6.2.1p4 | V1 |

Semantic test functions (proposed):

```text
v2_sem.rs
  s01_condition_scalar_truth
  s02_condition_effect_once
  s03_logical_and_result_type
  s04_logical_or_result_type
  s05_and_sequencing
  s06_or_sequencing
  s07_short_circuit_suppresses_rhs
  s08_short_circuit_runs_rhs
  s09_continue_binds_loop
  s10_continue_for_targets_step
  s11_continue_do_targets_condition
  s12_nested_jump_binding
  s13_break_skips_for_step
  s14_loop_condition_reevaluated
  s15_loop_evaluation_order
  s16_main_falloff_semantics
  s17_unreachable_checked
  s18_dead_code_constraints
  s19_logical_precedence_typed
  s20_for_scope_shadowing
```

### 7.4 CFG / IR acceptance (L1/L2/L3A)

Structural assertions are graph properties, not a dump of one implementation's
block numbering. IDs may differ across implementations; their stability across
replay is required.

| ID | Input | Expected CFG/IR property | Basis | Gate |
|---|---|---|---|---|
| C-01 | F-IF-03/04 | Conditional block with true and false successors; both arms reach a common join; exactly one return on each path. | 6.8.4.1; V2-0.6/0.7 | — |
| C-02 | F-IF-01/02 | Conditional block; false successor reaches the after/merge path; no else block required. | 6.8.4.1 | — |
| C-03 | F-LP-01/02 | `while` condition block; true edge to body; body back edge to condition; false edge to exit. | 6.8.5.1 | — |
| C-04 | F-LP-03/04 | `for` init before condition; body then step; step back to condition; exit on false. | 6.8.5.3p5 | — |
| C-05 | F-LP-06/07 | `do` body before condition; true edge back to body; false edge to exit. | 6.8.5.2 | — |
| C-06 | F-LP-09 | `continue` edge targets the `for` step block; the step then reaches the condition. | 6.8.6.2p2 | — |
| C-07 | F-LP-10/11 | `continue` edge targets the condition block in `while`/`do`. | 6.8.6.2p2 | — |
| C-08 | F-LP-08/14 | `break` edge targets the loop exit, not the step/condition. | 6.8.6.3p2 | — |
| C-09 | F-LP-13 | Nested loops: inner break/continue edges target inner exit/step; outer edges remain distinct. | 6.8.6.2/6.8.6.3 | — |
| C-10 | F-SC-08/09 | `&&`: LHS block has a conditional edge to the RHS block and a bypass edge to the merge; join value is defined on both paths. | 6.5.13p3; V2-0.7 | — |
| C-11 | F-SC-10/11 | `\|\|`: LHS block has a conditional edge to the RHS block and a bypass edge to the merge; join value is defined on both paths. | 6.5.14p3 | — |
| C-12 | F-SC-05/06 | Chained `&&`/`\|\|` preserves right-nested conditional evaluation; no RHS block is entered unconditionally. | 6.5.13/6.5.14 | — |
| C-13 | All positive CFGs | Every block has exactly one terminator; the terminator is the last instruction; no ordinary instruction follows it. | C11 control semantics; V2-0.7 | — |
| C-14 | SCFG-NEG-DANGLING | VF07 rejects a branch to a missing block; no panic and no silent fallthrough. | V2-0.7 | VF07 |
| C-15 | SCFG-NEG-CROSS-FN | VF07 rejects a branch to another function's block. | V2-0.7 | VF07 |
| C-16 | SCFG-NEG-NO-ENTRY | VF07 rejects a missing/duplicate entry block. | V2-0.7 | VF07 |
| C-17 | SCFG-NEG-DUP-TERMINATOR | VF07 rejects a second terminator or an append after one. | V2-0.7 | VF07 |
| C-18 | SCFG-NEG-APPEND-AFTER-END | Append after an ended block is rejected at commit or VF07; no partial append. | V2-0.7 | VF07 |
| C-19 | SCFG-DEAD-BLOCK | An unreachable block is allowed or explicitly reported; it does not make reachable CFG verification fail. | 6.8.3; V2-0.7 | VF07 |
| C-20 | F-UN-01/10 | Unreachable code after a terminator is not emitted in the same block; dead blocks, if retained, are marked/reported. | 6.8.3; V2-0.7 | — |
| C-21 | F-IF-13 | Both arms return: no reachable merge edge after the `if`; the trailing statement is unreachable. | 6.8.4.1 | — |
| C-22 | F-IF-12 | `main` falloff: the path reaching the end returns 0, not an undefined/absent terminator. | 5.1.2.2.3 | — |
| C-23 | F-IF-11, F-LP-02 | Empty then/else/body blocks are legal and terminate correctly. | 6.8.3, 6.8.5.1 | — |
| C-24 | All positive CFGs | Block and instruction IDs are stable across replay; block ownership and ordinals are deterministic. | T01 determinism; V2-0.12 | — |
| C-25 | SCFG-POS-* | VF07 accepts well-formed if/while/for/do/short-circuit CFGs. | V2-0.7 | VF07 |
| C-26 | SCFG-NEG-JOIN-UNDEF | VF08 (or frozen equivalent) rejects a join slot not defined on every incoming path. | V2-0.9 | VF08 |
| C-27 | SCFG-NEG-WRONG-TYPE | VF08 rejects a non-`int`/wrong-type condition or result. | V2-0.9 | VF08 |
| C-28 | F-SC-15/16 | Join/effect facts show LHS before RHS and exactly one evaluation of each side. | 6.5.13p3, 6.5.14p3 | VF12 |
| C-29 | F-UN-05/06/07 | CFG lowering is not attempted for a semantic failure; no half-built function is committed. | T09; V2-0.6 | — |

CFG test functions (proposed):

```text
v2_cfg.rs
  c01_if_else_edges_and_join
  c02_if_without_else_edges
  c03_while_back_edge
  c04_for_step_and_back_edge
  c05_do_condition_after_body
  c06_continue_targets_for_step
  c07_continue_targets_condition
  c08_break_targets_loop_exit
  c09_nested_loop_targets
  c10_logical_and_conditional_rhs
  c11_logical_or_conditional_rhs
  c12_chained_short_circuit
  c13_single_terminator_last
  c14_dangling_target_rejected
  c15_cross_function_target_rejected
  c16_entry_block_required
  c17_duplicate_terminator_rejected
  c18_append_after_terminator_rejected
  c19_unreachable_block_policy
  c20_dead_code_after_terminator
  c21_both_arms_return
  c22_main_falloff_returns_zero
  c23_empty_blocks
  c24_stable_ids_replay
  c25_vf07_positive
  c26_join_definition_required
  c27_operand_type_required
  c28_short_circuit_effect_facts
  c29_no_cfg_on_semantic_failure
```

### 7.5 Integration, replay, negative, and target acceptance

The protocol rows E-13..E-19 are parameterized over the installed V2 worker
registry: each V2 chip must be exercised for wrong owner, wrong kind/production,
non-`Running` state, interleaved unrelated work, and unauthorized writes. Each
case also asserts an unchanged snapshot. This is not satisfied by testing only
one representative chip.

| ID | Case | Expected | Level | Gate |
|---|---|---|---|---|
| E-01 | F-IF-01 real chain | AST/Sem/CFG/IR committed; Part A returns **7**. | L2+L3A | V1 |
| E-02 | F-IF-04 real chain | Part A returns **9**. | L2+L3A | V1 |
| E-03 | F-IF-05 real chain | Dangling `else` structure plus Part A **2**. | L2+L3A | V1 |
| E-04 | F-LP-01 real chain | While CFG plus Part A **3**. | L2+L3A | V1 |
| E-05 | F-LP-08 real chain | For `continue` CFG plus Part A **8**. | L2+L3A | V1 |
| E-06 | F-LP-11 real chain | Do-while `continue` CFG plus Part A **12**. | L2+L3A | V1 |
| E-07 | F-LP-13 real chain | Nested break/continue plus Part A **4**. | L2+L3A | V1 |
| E-08 | F-SC-08 real chain | Short-circuit effects plus Part A **3**. | L2+L3A | V1 |
| E-09 | F-SC-09 real chain | Suppressed RHS effect plus Part A **0**. | L2+L3A | V1 |
| E-10 | F-UN-01 real chain | Unreachable code accepted; Part A **1**. | L2+L3A | V1 |
| E-11 | F-IF-01/F-LP-08 replay | Two fresh runs produce identical per-tick snapshots and final IR. | L2+L3A | — |
| E-12 | F-NEG-19 replay | Two failed runs produce identical diagnostics, task states, and snapshots; no partial state. | L2 | — |
| E-13 | Wrong-owner task | `drive_task` returns `DriveError::OwnerMismatch`; task stays `Ready`; no result/diagnostic. | L1 | — |
| E-14 | Wrong-kind/production task | `DriveError::KindNotClaimed` or a typed `Task`/`Protocol` failure; no append. | L1 | — |
| E-15 | Non-`Running` task | `Ready`/`Completed`/`Waiting` task is rejected on the `Task` channel; no append. | L1 | — |
| E-16 | Unrelated task interleaved | An unrelated `CONTROL_NOOP` completes without mutating V2 semantic state; replay unchanged. | L2 | — |
| E-17 | Manifest undeclared write | Registration with an extra/unauthorized write is rejected (`StoreOwnerViolation`/`UnknownWriteField`). | L0 | — |
| E-18 | Commit-time undeclared write | A crafted `StorePatch`/append outside the V2 chip manifest is rejected; snapshot unchanged. | L0 | — |
| E-19 | Read-only config write | A V2 chip writing `config` is rejected with `ReadOnlyStore`; no partial state. | L0 | — |
| E-20 | Failure isolation | A malformed V2 task fails; a following valid task still runs and reaches a terminal state; no spin. | L2 | — |
| E-21 | Quota-1 lifecycle | Each enqueue is executable next tick; completion exactly once; no same-tick re-entry. | L2 | — |
| E-22 | F-IF-01 target execution | Compiled/linked program exits **7** on the verified target. | L3T | PROBE/HOST |
| E-23 | F-LP-01 target execution | Exits **3**. | L3T | PROBE/HOST |
| E-24 | F-SC-08 target execution | Exits **3**. | L3T | PROBE/HOST |
| E-25 | F-LP-13 target execution | Exits **4**. | L3T | PROBE/HOST |
| E-26 | F-UN-01 target execution | Exits **1**. | L3T | PROBE/HOST |
| E-27 | Target replay | Three consecutive target runs produce the same exit status and no nondeterministic output. | L3T | PROBE/HOST |

Integration test functions (proposed):

```text
v2_e2e.rs
  e01_if_true_real_chain
  e02_if_false_real_chain
  e03_dangling_else_real_chain
  e04_while_real_chain
  e05_for_continue_real_chain
  e06_do_continue_real_chain
  e07_nested_break_continue_real_chain
  e08_short_circuit_both_sides_real_chain
  e09_short_circuit_suppressed_real_chain
  e10_unreachable_real_chain
  e11_replay_identical_source
  e12_replay_failed_source
  e13_wrong_owner_task_rejected
  e14_wrong_kind_or_production_rejected
  e15_non_running_task_rejected
  e16_unrelated_task_does_not_mutate_v2
  e17_manifest_undeclared_write_rejected
  e18_commit_undeclared_write_rejected
  e19_config_write_rejected
  e20_failure_isolation_and_progress
  e21_quota1_next_tick_lifecycle
```

L3T rows live in a separate target, created only after the probe/Host gate:

```text
v2_target.rs
  e22_target_if_exit_7
  e23_target_while_exit_3
  e24_target_short_circuit_exit_3
  e25_target_nested_exit_4
  e26_target_unreachable_exit_1
  e27_target_replay_three_runs
```

`v2_target.rs` is not created before H01/H07 and H04/H05 exist; E-22..E-27 remain
`BLOCKED_PROBE` in the ledger. This keeps the pre-probe `v2_e2e` target free of
missing Host APIs and avoids `#[ignore]`-as-pass ambiguity. Once the probe exists,
`v2_target` is table-driven over **all** §6 value fixtures with their expected
values; E-22..E-27 are the minimum representative rows, not the whole target
matrix.

---

## 8. Exact future commands and pass/fail criteria

### 8.1 Available target-independent commands

Run from the repository root. These commands are **not run by this report**;
they are the commands the future V2 work order must use after V2-0 and the V2
implementation land.

```sh
cargo fmt --manifest-path compiler/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings

cargo test --locked --manifest-path compiler/Cargo.toml --test v2_parse
cargo test --locked --manifest-path compiler/Cargo.toml --test v2_sem
cargo test --locked --manifest-path compiler/Cargo.toml --test v2_cfg
cargo test --locked --manifest-path compiler/Cargo.toml --test v2_e2e

# Only after H01/H07 and H04/H05 exist; the file must not exist before then:
# cargo test --locked --manifest-path compiler/Cargo.toml --test v2_target

# One case while debugging (example):
cargo test --locked --manifest-path compiler/Cargo.toml --test v2_parse \
  p03_dangling_else_binds_nearest -- --exact

# Only when a V2-0 amendment changes the frozen hash:
cargo test --locked --manifest-path compiler/Cargo.toml --test freeze
```

Do **not** run the full `cargo test --workspace` as V2 evidence; the V2 suite is
targeted per file. The root package and `tools/torture` are separate packages
and are not V2 acceptance inputs.

Expected command result for each pre-probe V2 test target:

```text
test result: ok. N passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The exact `N` is the number of `#[test]` functions in the target, not the number
of matrix rows. A matrix row may be covered by one test with several assertions.
`v2_target` is separate and is not run, created, or counted before the probe/Host
gate; an absent target is `BLOCKED_PROBE`, not a pass.

### 8.2 Probe-gated target commands

There is **no authorized command today** for L3T:

- H04 candidate driver is absent as a complete driver in the checked baseline;
- H05 isolated execution/timeout/resource limits are absent;
- H01/H07 have no probe report or verified target;
- `ensure_codegen_ready` fails closed while the target is unverified.

The V2 target work order must copy the exact H04/H05 commands from their
approved acceptance work orders; this report must not invent a candidate CLI.
Until then, E-22..E-27 are `BLOCKED_PROBE`, not pass.

### 8.3 Pass/fail criteria

**Pass requires all of the following:**

1. V2-0 contract freeze is accepted, hashed, and recorded; the test code compiles
   against the frozen names, not local guesses.
2. `v2_parse`, `v2_sem`, `v2_cfg`, and `v2_e2e` each exit 0 with zero failed and
   zero ignored target-independent tests.
3. Every matrix row in §7 is mapped to at least one test function/assertion; a
   missing row is a coverage failure.
4. Every positive L1/L2/L3A case produces the exact expected AST fact, semantic
   fact, CFG property, and Part A value.
5. Every negative case produces the frozen diagnostic group/rule, exactly one
   terminal failure, no partial AST/Sem/IR, and an unchanged snapshot.
6. Every replay case produces byte-identical snapshots for the same input/config/
   pins; no hash-map iteration, address, or timing dependence.
7. No in-scope `Unsupported` is counted as success; no test special-cases a
   fixture filename/spelling; no expected value is read from the implementation.
8. `fmt` and `clippy -D warnings` are clean on the V2 worktree. The dirty
   workspace's pre-existing candidate-file clippy failures are not accepted as a
   V2 green signal; V2 work must use a clean branch/worktree.
9. L3T rows in `v2_target.rs` pass on the verified Linux AArch64 target, or are
   reported `BLOCKED_PROBE` while the file does not exist; Part A evidence must
   not be promoted to L3T.

**Fail conditions (any one):**

- a V2 construct is silently accepted with wrong control flow;
- a branch/loop/join is missing from the CFG or the expected value is produced
  only by constant folding without a CFG;
- `break`/`continue` binds the wrong target, especially across `if`/nested loops;
- a dangling `else` binds the outer `if`;
- RHS side effects run when short-circuit says they must not, or are skipped when
  they must run;
- an unreachable invalid construct is not diagnosed;
- an unauthorized write is accepted or leaves partial state;
- a non-matching task mutates state or completes successfully;
- replay differs;
- a target test is claimed without a verified probe/runner;
- a blocked target row is omitted, ignored, or counted as pass without the
  `BLOCKED_PROBE` label.

---

## 9. Dependencies and blockers

| ID | Blocker | Current state | Work needed | Owner |
|---|---|---|---|---|
| B-01 | V2-0 contract freeze | Absent; `/39` has no V2 shapes. | Accept V2 shapes, diagnostics, manifests, stages, hash. | T01 integrator + T05/T07/T09/T13 |
| B-02 | V1 base | Frozen artifact is M1 `return 2+3`; stateful loops need locals/assignment. | Complete V1 L1/L2/L3A integration. | V1 train |
| B-03 | T05 parser if/loop/jump | Candidate `pa_stmt` rejects all V2 forms. | Implement PA29/PA31/PA32 (+ PA27/PA28 integration), dangling-else resolution, `for` scopes, recovery. | T05 owner |
| B-04 | T06 scopes/declarations | No local object/scope integration for loop declarations. | Freeze/implement loop/if block scopes and `for` declaration scope. | T06 owner |
| B-05 | T07 semantic control/effects | `EffectMask(0)` only; no jump-target carrier. | Freeze/implement condition checks, jump binding, sequencing, nonzero effects, unreachable checking. | T07 owner |
| B-06 | T09 CFG lowering | Only `Constant`/`Return`, single block. | Implement IR12/IR13/IR20/IR21/IR23/IR28, terminators, joins, dead-code policy. | T09 owner |
| B-07 | VF07/VF08 | VF07 out of M1; VF08 unregistered. | Freeze/register CFG/IR verifiers and fixtures. | T13 owner |
| B-08 | VF12 extension | Single-block `Constant`+`Return` only. | Extend to V2 CFG/locals/arithmetic/assignment; explicit `Unsupported` outside coverage. | T13 owner |
| B-09 | Target probe | No report; identity only. | H01/H07 probe, attestation, Linux AArch64 substrate. | H01/H07 + T01 |
| B-10 | Host execution | H04/H05 absent. | Candidate driver, isolation, timeouts, resource limits, runner. | H04/H05 |
| B-11 | Diagnostics | No V2 control diagnostic IDs. | Freeze group/rule IDs; tests assert them. | T01 + owners |
| B-12 | Dirty workspace | Uncommitted parse/lex drafts; clippy fails. | Use a clean V2 worktree/branch; do not overwrite candidates. | Integrator |

Until B-01 and B-02 are resolved, V2 L1 tests cannot compile or run; until B-03
through B-08 are resolved, L2/L3A cannot run; until B-09/B-10 are resolved, L3T
cannot run. A blocked row is reported blocked, never passed.

---

## 10. Open questions for the V2-0 freeze

1. **AST child encoding.** Does `NodeRecord.children` encode optional `else` by
   count/order, or does V2 add explicit role/operator fields? The answer determines
   P-03/P-04 and every CFG test.
2. **Jump target carrier.** Is `break`/`continue` binding stored in `SemRecord`, a
   new `SemControlRecord`, or a resolved `JumpTarget` child node? Tests must know
   where to assert.
3. **`EffectMask` bits.** Which bits mean assignment/store/loop-condition
   sequencing, and is nonzero `EffectMask` allowed for V2 or only for a later
   effects train? V2 cannot pass loop fixtures without a decision.
4. **Terminator/edge shape.** Are CFG successors encoded in `InstructionRecord`,
   a `TerminatorRecord`, or `BlockRecord` edges? Exact shape must be frozen before
   VF07/tests.
5. **Dead-block policy.** Are unreachable blocks retained and marked, or omitted?
   The suite permits either but requires a reachable-code check and an explicit
   report; the freeze must choose the observable contract.
6. **VF12 coverage.** Extend VF12 or introduce a V2-specific CFG interpreter?
   Which ops/locals/assignments are covered, and what is the result carrier?
7. **`switch`/`goto`/`?:`/comma.** Are they explicit V2 `Unsupported` boundaries
   (this report's assumption), or do they belong to V2/V3? Reclassify N-17/N-18 if
   needed.
8. **`for` declaration scope.** Is `for (int i = ...)` mandatory V2 or deferred?
   F-LP-15/F-NEG-15 assume mandatory C11 scope.
9. **`main` falloff.** Confirm the implicit `return 0` is in V2 semantic/IR scope;
   otherwise F-IF-12 is deferred.
10. **Condition types.** V2 only checks scalar `int`/`_Bool` for now; pointer/array
    conditions wait for V3. Confirm that a synthetic `void` condition is the only
    L1 non-scalar negative.
11. **Diagnostic taxonomy.** Which group/rule IDs represent parse, semantic,
    unsupported, CFG, and IR failures? Tests must not assert message text.
12. **O0 branch policy.** Confirm no constant folding of `if`/loops at O0; if the
    integrator allows folding, the CFG expectations in C-01..C-12 need a
    version/legality contract and the “constant interpretation” warning must be
    addressed explicitly.
13. **Test entry point ownership.** Confirm the four pre-probe test targets, the
    deferred `v2_target` target, and the `v2_support` module are accepted as
    test-owned paths; no Cargo or `mod.rs` edit is needed for auto-discovery.
14. **Probe exit-status normalization.** Confirm the runner reports the target
    `main` return as the observed status (including the low-8-bit POSIX
    convention); all fixture expectations are 0..255 (max 91), so no truncation
    ambiguity is intended.

---

## 11. Execution evidence

**No tests, builds, probes, or corpus runs were executed for this report.**

- No `cargo test`, `cargo fmt`, `cargo clippy`, `cargo run`, assembler, linker,
  runner, or probe command was run.
- No V2 test file or fixture exists; the four pre-probe test targets, the
  deferred `v2_target` target, and the support module are not created by this
  design task.
- The expected values in §6/§7 are hand calculations and C11 clause applications,
  not observed compiler output.
- The baseline statements in §3 are read-only source/document inspection at
  `t01-c01-c06/39`; they are not implementation or acceptance evidence.
- Any future claim must record exact command, worktree, contract version/hash,
  target profile, and raw output; a probe-gated target row remains
  `BLOCKED_PROBE` until the verified substrate exists.

---

## 12. Handoff checklist

- [ ] V2-0 freeze accepted and hashed; all candidate names replaced by frozen
      names.
- [ ] V1 integration pass recorded before V2 L2/L3A is attempted.
- [ ] `compiler/tests/v2_support/{mod.rs,fixtures.rs}` and the four pre-probe
      test targets created by the independent test owner only.
- [ ] `compiler/tests/v2_target.rs` created only after the probe/Host gate.
- [ ] Every §7 row mapped to a test function/assertion and listed in the V2
      coverage ledger.
- [ ] Positive fixtures run through the real PP→LX→PA→TY→SE→IR chain; no seeded
      upstream output.
- [ ] Negative fixtures assert one terminal failure, no partial state, and an
      unchanged snapshot.
- [ ] Replay tests compare per-tick snapshots and final IR for fresh buses.
- [ ] Non-matching-task and undeclared-write tests exercise driver, registration,
      commit, and snapshot-unchanged paths.
- [ ] Target rows remain `BLOCKED_PROBE` until H01/H07 and H04/H05 evidence exists.
- [ ] No implementation owner edits test expectations to make a delivery pass.

---

## 13. Revision record

| Date | Change | Author |
|---|---|---|
| 2026-11-07 | Initial V2 independent acceptance design (A-TST-02). Defines levels, files, harness, fixture catalog, case matrix, commands, pass/fail criteria, blockers, and open questions. Design only; no code, tests, probe, or execution. | A-TST-02 subagent |
