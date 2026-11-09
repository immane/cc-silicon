# Mass-Parallel Compiler Development Dispatch Runbook

| Field | Value |
|---|---|
| Status | **Dispatch planning, not a contract freeze or implementation authorization** |
| Updated | 2026-11-10 |
| Scope | Compiler application, Host tooling, verification, and integration; does not extend root-framework responsibilities |
| Goal | Provide 100–150 parallel task slots when interfaces and file isolation permit, while prioritizing a real execution closed loop |
| Checked baseline | `t01-c01-c06/55`; re-read the version, hash, and workspace state at dispatch time |
| This document does not provide | Started agents, completed implementations, rerun tests, target probe results, or a GCC pass rate |

**Next dispatch:** use [Section 13](#13-next-dispatch-from-53) for the concrete
post-`/53` work orders, ownership, release gates, and handoffs. Sections 5–8 are
long-term capacity pools, not instructions to launch 100 implementations now.

## 1. Authority and Usage Boundaries

This document implements the dispatch requirement to "dispatch as many subtasks in
parallel as possible, up to hundreds." It refines
[WAVE_DISPATCH.md](WAVE_DISPATCH.md) and does not replace
[PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md),
[TASK_TEMPLATE.md](TASK_TEMPLATE.md), or
[COMPILER_DEVELOPMENT_GUARDRAILS.md](COMPILER_DEVELOPMENT_GUARDRAILS.md).
The [T01](T01_COMPILER_CONTRACT.md) integrator still freezes shared protocols.
The user's request to write this plan does not approve unaccepted ADRs, candidate
schemas, target values, arbitrary network downloads, or actually starting agents.
The existing WAVE_DISPATCH R1/R2/R3 rules apply within their original authorization
scope; this document does not expand that authority.

The architecture basis is the
[paradigm specification](../architecture/SILICON_PARADIGM_SPEC.md), the
[SFL contract](../architecture/SFL_CONTRACT.md), the
[design blueprint](../design/ARCHITECTURAL_BLUEPRINT.md), and the accepted
[ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md).
The CPU dynamic arena is only a compiler-application extension. Chips remain
stateless, perform no I/O, and make no cross-chip calls; state is explicit in the
bus; parallel development does not change propagation order or commit rules.

**Task counts are not chip counts, nor are they a number that can be coded
simultaneously.** The counts in this document are task-pool budgets, not an
established work-order list; a task becomes dispatchable only after exact
inputs/outputs, files, and verification commands are filled in. Do not split
duplicate producers, create empty shells, or add unneeded abstractions just to
reach a headcount.

The letters A–E label dispatch stages in this document. They do not renumber the
Wave 1–5 sequence in [WAVE_DISPATCH.md](WAVE_DISPATCH.md).

## 2. Current Facts and Remaining Scope

The following are static inspection conclusions from source, the frozen marker,
and documentation. They are not a record of test execution in this change.

| Area | Checked baseline | Remaining focus |
|---|---|---|
| Root framework | Bus, chips, Motherboard, CPU backend, ticks, and test utilities exist | Keep it generic; do not push C rules down into the framework |
| Application contract | [CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION) is `/53` | Subsequent small-batch freezes, dependency revalidation, registration, and permission consistency |
| Preprocessing | Several implemented slices with bounded capability | Driver integration, real header loading, recursive include, once state, complete macro-conditional rules |
| Lexing | Integer M1 subset plus float, character, and string slices | Integer radix/suffix/type, adjacent strings, dialects, recovery, and downstream integration |
| Parsing | [pa_tu.rs](../../compiler/src/chips/parse/pa_tu.rs) is still limited to the M1 shape; some productions are Ack-only checks | General declarators, expressions, statements, initialization, and real AST production |
| Types and semantics | M1 subset exists | Scopes, compatibility/conversions, lvalues, pointers, calls, effects, and control-flow legality |
| Constants/layout/initialization | Folding and some conditional checks exist | Target widths, layout, bitfields, initialization plans, VLA |
| IR | [ir_function.rs](../../compiler/src/chips/ir_lower/ir_function.rs) produces local loads/stores, integer expression trees, ordered assignments, and return | Multiple functions, parameters/calls, general memory, CFG, dynamic stack, and verification |
| Target backend | `/48` pins a probe-attested Debian 12 arm64/GCC 12.2.0 baseline; the host-only emitter covers local integer programs | Calls/ABI, general codegen, machine chips, and the official T00 reference substrate remain open |
| Host | [candidate](../../compiler/src/bin/candidate.rs) has evidence output, bounded leaf includes, `-E/-D/-U`, and prototype `--emit-asm`; H05 process supervision exists in `tools/runner` | `-I`, multiple sources, formal `-S/-c`, official test driver, statistics, and CI; H05 is not a kernel sandbox |
| Acceptance | Corpus lock/probe scaffolding exists; no final acceptance evidence | Freeze corpus/environment/options/denominator, run, and retain failures |

The earlier audit found parse drafts and duplicate reservations. B-INT-00 parked
the unregistered drafts; do not reactivate them or treat Ack-only productions as
AST producers. Re-read the working tree before each dispatch rather than assuming
the earlier uncommitted state still exists. Older index descriptions such as
`/5`, `/9`, or "no chips" cannot serve directly as current facts. Historical test
counts in the ledger are prior-session evidence, not checks rerun by this update.

## 3. Dispatch Goals and Critical Paths

Priority: correctness and contract consistency first; a real M1 execution closed
loop takes precedence over optional optimization and broad GNU shells.

```text
Audit existing work ──→ Collect contract requirements in parallel ──→ Integrator freezes
                                                                          │
                                                        100+ independent implementations
                                                                          │
                                              Independent tests/review → Batch integration
                                                                          │
                                                   Real upstream input → Vertical closed loop

Linux AArch64 environment → probe → H07 ABI classification → integrator attestation
                                                                  │
Frontend real source → IR → minimal machine chain → assembly → external assembler/
linker → execution returns 5
```

The two critical paths advance independently: large-scale frontend work must not
hold up the target environment, and the target probe must not block symbolic Part A
design or already-frozen implementation. Parts that depend on concrete target
values, ABI, or machine execution remain subject to the probe gate.

Development parallelism does not authorize concurrent runtime mutation of the bus.
Quota 1 remains the comparison baseline; quota>1, out-of-order execution, fusion,
and batched predicted-ID schemes are not implemented by hitchhiking and require
separate contracts and differential acceptance.

## 4. Organization, Files, and Resource Isolation

| Role | Responsibility and write permission |
|---|---|
| Overall integrator, one logical writer | Shared records, IDs, task/result, bus, manifest aggregation, routing, `mod.rs`, Cargo, version/hash, and shared test entry points |
| Domain reviewers, assigned as needed | Consolidate rule and interface requirements and review semantics and dependencies; must not modify shared protocols themselves |
| Implementers, one owner per work order | Modify only the implementation paths named exactly in the work order |
| Independent testers | Modify only assigned test/fixture paths; expected values are independent of the implementation |
| Verification and integration reviewers | Check evidence, permissions, registration, real chains, and regressions; propose fixes but do not overstep by editing others' work |

Independent worktrees/branches are recommended; one implementation file has only
one active writer. Audits may read the shared workspace, but large-scale
implementation must not share mutable build artifacts, test temporary directories,
or the same output file. Test entry points belong to the integrator; how a new test
file is actually compiled and run must be stated in the work order.

The maximum active count is the minimum of: ready tasks, platform agent limit,
available resource quota, and review capacity. 100–150 is a design capacity; there
is no promise that current tooling can launch that many at once. Model sessions,
CPU builds, and target runs are throttled separately: research and coding can run
at scale, while full compilation and target execution use bounded queues. Each work
order states time, output, retry, and resource budgets; on reaching a limit it
returns blocked or a defect rather than retrying indefinitely.

## 5. Stage A: 110 Tasks That Can Be Prepared Now

This stage prioritizes read-only audits, protocol requirements, and independent
test design. Host code is implemented only when its existing authorization and
protocol meet the gates; candidate code must not preempt unfrozen language
interfaces.

The scopes below are planning task numbers, not ChipId/TaskKind values, and are not
assigned to actual agents.

| Planning task ID | Count | Split and deliverable |
|---|---:|---|
| A-AUD-01..20 | 20 | Partition audits by framework, application foundation, language groups, Host, and acceptance tooling; identify existing implementation, registration, tests, integration, shells, and conflicts |
| A-PTS-01..24 | 24 | Eight rule and contract-requirement documents each for PA/TY/SE; divide declarations, expressions, statements, scopes, conversions, calls, and effects by responsibility |
| A-CLI-01..16 | 16 | Eight for constants/layout/initialization and eight for IR; artifact fields, versions, effects, CFG, and consumer requirements |
| A-TST-01..24 | 24 | Four independent acceptance designs each for preprocessing/lexing, parsing, types, semantics, constants/layout, and IR |
| A-HST-01..10 | 10 | One gap/dependency/acceptance design each for H00..H09; H10 standards/ABI/random tests are folded into independent test tasks |
| A-ABI-01..08 | 8 | Evidence and protocol requirements for target environment, integer model, arguments, returns, stack, variadics, floating point/aggregates, and the assembly boundary |
| A-REV-01..08 | 8 | Read-only review of duplicate producers, type conflicts, lifetimes, wait/cancel, permissions, determinism, security, and acceptance definitions |
| **Total** | **110** | Not all must run simultaneously; each enters the ready queue after exact file/rule assignment |

An audit deliverable includes at minimum code/test paths and line locations, rule
scope, whether the candidate installs it, and differences from the ledger. Contract
deliverables propose only the smallest change and conflicts; they do not duplicate a
private type system. Review tasks may read the current contract first and then
consume this stage's deliverables; missing input means an explicit wait, not a
fabricated conclusion.

**Exit:** a trusted baseline, clear contract change requests, independent fixtures,
and exact next-stage work orders. The integrator merges duplicate requirements and
freezes in batches; 110 research reports are not themselves a capability-growth
metric.

## 6. Stage B: Up to 132 Candidate Production Packages After Freeze

Eight control packages are made explicit on top of the previously discussed
124-package budget, so scheduling and commit gaps are not hidden inside the Host
driver. These are not 132 ready implementations; every package must be verified
item by item against Section 9.

| Planning task ID | Packages | Catalog/task scope and split principle |
|---|---:|---|
| B-CTL-01..08 | 8 | T02: start/source response, selection/guard, validation, commit, recovery/wait, progress/phase, diagnostics/cancel, artifacts; check existing routing/commit first to avoid duplicate implementation |
| B-PPL-01..12 | 12 | T03/T04: six preprocessing integration/gap packages and six integer, string, dialect, location/recovery packages; repair and wire existing slices first |
| B-PAR-01..18 | 18 | T05: six declarator/declaration, six expression, and six statement/recovery packages; complex rules have one file owner each |
| B-TYP-01..20 | 20 | T06: six scope/symbol, seven type-construction, and seven compatibility/conversion packages |
| B-SEM-01..20 | 20 | T07: eight expression/value-category, six declaration/call, and six statement/effect packages |
| B-CON-01..16 | 16 | T08: six constant, five layout, and five initialization packages; target-specific subpackages wait for the target model |
| B-IRL-01..18 | 18 | T09: six value/memory, six CFG/control-flow, and six function/call/stack/finalization packages |
| B-VER-01..12 | 12 | T13 and independent tests: four record/AST/type, four CFG/IR, and four replay/evidence/interaction packages |
| B-HST-01..08 | 8 | H00/H01, H02/H03, H04, H05, H06, H07, H08, H09/H10; large packages may split into mutually exclusive implementation and review work orders |
| **Total** | **132** | Packages may split into implementation, test, and review tasks, but the same semantic capability must not be counted twice |

Exact binding still uses the
[T02–T13 catalog](README.md#3-documents-and-task-packages) and
[CHIP_PLAN](CHIP_PLAN.md) as entry points rather than deriving new ChipIds from this
table. If a reasonably fused existing worker already covers several numbers, first
determine whether the responsibilities are independent; do not hard-split for
headcount.

### 6.1 Unblocking Upstream/Downstream Waits

When interfaces are frozen, consumers can develop independently against synthetic
tokens, AST, types, SemRecords, IR, and similar fixtures. Fixtures substitute only
for upstream input, not for this task's semantics; protocols, ordering, ownership,
and error shapes must match the frozen contract. When real upstream output arrives,
fixtures must be replaced for interaction testing.

Implementation deliveries are counted at three separate levels:

1. **Independent implementation pass:** semantics, boundary, error, replay, and
   write-scope tests hold under synthetic inputs.
2. **Integration pass:** the worker is registered, consumes real upstream artifacts,
   and provenance and consume-once rules hold.
3. **Closed-loop pass:** real source yields the IR required by the feature or the
   target execution result.

An independent pass must not impersonate an integration pass; Ack certification must
not impersonate AST/IR production; an Unsupported shell is only an explicit gap and
must not count as semantic completion or acceptance PASS.

### 6.2 Three Vertical Integration Trains

| Train | Ordered scope | Layers that must be connected |
|---|---|---|
| V1 | Integer expressions, parentheses/precedence, local variables, declaration/initialization, assignment and conversions | PA→TY→SE→CL/IR→verification; add real execution once the target is unlocked |
| V2 | if/else, loops, break/continue, short-circuit evaluation, and CFG | Real conditions and branches; constant interpretation must not impersonate all control flow |
| V3 | Function parameters/calls/returns, global variables, pointers, and arrays | Joint type/semantic/memory/ABI acceptance; do not substitute the Rust host ABI for the target |

The three trains freeze their common base first, then allow overlap for work that
depends on frozen interfaces; integration advances producers before consumers. For
target semantics that are not yet unlocked, report only Part A evidence and an
explicit blocker.

## 7. Stage C: Target Environment and Backend Parallel Pool

H01 fixes the Linux AArch64 board/VM/CI, sysroot, GCC, assembler/linker, runner, and
versions, runs the probe, and produces a report/hash. H07 classifies the unresolved
ABI evidence in the report; the T01 integrator reviews provenance and completes
attestation. Hash validation alone does not prove that a report is authentic.

Before the probe, machine-record, instruction, and ABI-rule research, oracle
preparation, and mixed-link test preparation may proceed in parallel, but production
code generation must not be unlocked by a synthetic target or by macOS arm64 values.

After probe evidence covers the required fields and the protocol is frozen, build a
target task pool from [T11 CG01–CG39](T11_TARGET_CHIPS.md), grouped by ABI,
integer/addressing, branch/memory access, stack/registers, floating
point/aggregates, and assembly output. Each group gets independent tests and review;
the actual parallel count depends on the frozen schema and file isolation. When this
work overlaps with Stage B, reuse the H01/H07 owners rather than dispatching the
same work twice.

**Integrate the minimum critical path first:** Constant/Return IR → own target
assembly → external assembler/linker → designated runner, execute the program, and
verify exit code 5. The candidate must not hand C/AST/IR to GCC/Clang to generate the
candidate program. Do not wait for all 39 tasks before accepting the minimal path;
an IR interpretation of 5 is not completion of the execution closed loop.

H05 isolation, timeouts, output/resource limits, cancellation, and anomaly
classification must precede large-scale execution of untrusted programs. `-S`/`-c`
open only after the real target is ready and the relevant acceptance passes; while
closed, they continue to be rejected explicitly.

## 8. Stage D/E: Complex C, Optimization, GNU, and Acceptance Convergence

| Stage | Task pool | Gate and exit |
|---|---|---|
| D | Aggregates/enums/layout/bitfields, complex initialization, floating point, variadics, complex declarators, VLA, old-style functions, volatile/atomic | Each type/semantic/IR/ABI interface frozen; complete interaction, mixed-link, and execution evidence |
| E-OPT | Ready subset of T10 OP01–OP22 | First freeze IR versions, analyses, and rewrite protocols; legality checks, invalidation, and semantic regression precede performance |
| E-EXT | Ready subset of T12 EX01–EX39 plus corpus-driven additions | ABI, sysroot, dialect, and census dependencies satisfied; unknown extensions fail explicitly and are not claimed as complete GNU support |
| E-GATE | H02/H03/H06/H08/H09/H10 and owners for each failure cluster | Official plan, frozen denominator, reproducible commands, retained failures, and stable continuous regression |

H00 corpus acquisition/locking and H02/H03 official DejaGnu plan integration should
be prepared early in parallel with Stages B/C, not started only after complex C is
complete. Downloads follow authorization and the on-demand hash-locked policy; do
not vendor test assets into the MIT core without permission. Reference baselines and
candidate results remain separate.

The final gate follows [T00](T00_GCC_TORTURE_GATE.md): the compile, execute, and ieee
per-suite instance rates and full-configuration file rates, plus both pooled rates,
must each exceed 99%; per-configuration rates are reported separately. An empty
denominator does not count as a pass. Unsupported, crash, wrong code, timeout, and
missing instances remain in the denominator and never become PASS; infrastructure
errors retain instances and block the final conclusion. H09 requires the same
profile to meet the gate stably across three consecutive runs.

## 9. Dispatch Gates, States, and Work Orders

### 9.1 Readiness Gates for Each Coding Work Order

If any condition below is missing, reassign the work to design/audit or mark it
blocked; do not bypass the gate with candidate-local constants:

1. Exact binding to the current contract version/hash; required fields, task/result,
   and error codes are frozen.
2. A single owner, exact allowed paths, test entry points, record producers, and
   permissions are explicit.
3. Input can come from a real producer or a valid synthetic fixture, and dependency
   and acceptance levels are explicit.
4. Task guard, stage, category, progress/wait/complete/failure/cancel behavior are
   testable.
5. Target/corpus gates are satisfied; otherwise the corresponding implementation
   stays blocked, and a shell delivery cannot count as semantic completion.
6. Verification commands actually exist and apply to the package, and resource
   budgets and expected complexity are explicit.

### 9.2 States and Ledger

```text
PROPOSED → NEEDS_FREEZE → READY → RUNNING → REVIEW → UNIT_ACCEPTED
                                                    ↓
                                             INTEGRATION_PENDING
                                                    ↓
                                               INTEGRATED
                                                    ↓
                                           CLOSED_LOOP_ACCEPTED
```

Side states are `BLOCKED_CONTRACT`, `BLOCKED_PROBE`, `BLOCKED_CORPUS`,
`BLOCKED_RESOURCE`, `DEFECT`, and `UNSUPPORTED_SHELL`. Target-independent tasks are
not globally blocked by the probe. Timeouts, defects, withdrawals, and re-dispatches
retain their history and must not be silently removed from statistics.

The integrator owns the ledger; workers do not compete to modify a shared ledger. It
records at least work-order ID, chip/rule, owner, paths, dependencies, version/hash,
state, acceptance level, test commands/results, known limitations, blocker reason,
repair round, integration batch, and raw evidence location. If the ledger format or
automation needs new tooling, review it first as an independent tool task; this
document does not claim that a scheduler or machine-executable ledger already exists.

### 9.3 Copyable Work-Order Template

```text
Work order ID / role / priority:
Chip IDs / rule scope / explicit non-goals:
Contract version/hash / frozen schema references:
Base revision / input worktree state:
Owned implementation paths / owned test paths:
Forbidden shared paths:
Task kind / payload / result / error codes:
Exact reads / writes / record producers:
Guard / phase / capability / dialect / target:
Progress / wait / resume / complete / fail / cancel:
Dependencies / synthetic fixtures / independent oracle:
Complexity / resource limits / timeout / retry budget:
Exact verification commands / test discovery mechanism:
Real-upstream integration / closed-loop acceptance:
Delivery evidence / known unsupported / failure instance links:
```

A work order permits "deliver an evidence-backed implementation" or "return a
structured DEFECT"; it does not permit unilateral extension of shared protocols. A
DEFECT includes missing fields/conflicts, a minimal reproduction, affected tasks, the
minimal change request, and independent work that can continue.

## 10. Batch Integration, Versions, and Failure Handling

1. Freeze preparation first consolidates cross-group requirements; the integrator
   checks schemas, producers, permissions, stages, and result carriers, and updates
   version/hash and dependent tests under authorized rules. Whether a pure
   registration/wiring change affects the hash depends on the actual hash scope; an
   unchanged hash does not prove unchanged behavior.
2. Work orders pin a baseline; when a freeze changes, the integrator lists affected
   work orders, pauses related deliveries, and re-dispatches; not every task should
   blindly follow latest.
3. Each train absorbs about 10–20 delivery packages after review; each batch checks
   registration uniqueness, routing, ZST, field permissions, contracts, and relevant
   tests, with full regression at regular intervals and before release.
4. A successful build does not prove that files not included in compilation were
   tested. Check that modules and test entry points actually cover the delivery; an
   uninstalled worker is not candidate support.
5. Localize failures to the delivery package and exclude or fix it on an isolated
   integration branch; do not reset/overwrite the shared user workspace or rewrite
   others' history. Committing or pushing still requires the corresponding explicit
   authorization.
6. Non-critical packages may leave the train and be re-dispatched; a missing critical
   producer blocks its consumers' real-chain acceptance. Do not turn necessary rules
   into Unsupported just to make the train green and then declare closed-loop success.
7. When, within the cumulative reviewed implementation deliveries of a stage, work
   orders needing rework / reviewed implementation work orders >20%, stop dispatching
   new implementations and report; design review and failure localization may
   continue. Do not compute a defect rate when there are no reviewed deliveries. A
   retry does not erase the fact that a work order previously had a defect.

Large-scale coding throughput may exceed serial integration capacity. Prioritize
contract and independent-review quality; when the review queue reaches the limit set
by the freeze package, stop adding coding slots and assign independent review/testing
instead of expanding shared write permissions.

## 11. Verification and Progress Reporting

Each semantic chip covers normal, boundary, invalid, unsupported/recovery, replay,
non-matching task, and undeclared-fields-unchanged cases; cross-stage rules add real
producer/consumer tests. Expected values come from the specification, hand
calculation, or same-target reference evidence, not from the implementation under
test.

The root package, `compiler/`, and the torture tool are not currently one Cargo
workspace. Run the applicable checks in the corresponding directory for the actual
change; do not run only the root package and declare the whole repository passing:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cargo fmt --manifest-path compiler/Cargo.toml --all -- --check
cargo clippy --manifest-path compiler/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path compiler/Cargo.toml --workspace

cargo fmt --manifest-path tools/torture/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/torture/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path tools/torture/Cargo.toml --workspace
```

These are examples for later implementation work orders, not evidence that this
document ran them. The integrator verifies the actual tool entry points for
chip-lint, probe self-tests, target execution, and contract validation before writing
them into work orders; do not invent validator paths. Pure documentation changes use
content, link, count, and boundary-consistency checks and do not require fabricated
runtime acceptance.

Each train reports: ready/active/blocked/awaiting-review counts, separate
independent/integration/closed-loop pass counts, unfrozen requirements,
target/corpus blockers, defect rate, raw test evidence, and inputs to the next train.
Performance reports are separate from semantic passes; token consumption, file count,
and chip count are not correctness metrics.

## 12. Start Sequence and Stop Conditions

Executors should complete the following startup gates before filling concurrent
slots:

1. Record the baseline and existing uncommitted changes, confirm ownership, and do
   not overwrite candidate work.
2. Expand the Stage A budget into exact, mutually exclusive work orders; immediately
   run audits, rule research, and test design in parallel.
3. Advance the H01/H07 real-environment evidence path and H05 isolation in parallel,
   stating the external permissions required.
4. The integrator freezes the minimal target chain and V1 common interfaces first;
   continuously add Stage B/C work orders that pass the gates to READY.
5. Dispatch exactly as many truly ready work orders as exist, reaching large-scale
   parallelism when resources allow; when short, add independent tests and reviews
   rather than coding unfrozen interfaces merely to keep 100 agents active.
6. Accept the minimal real M1 run first, then expand the integration surface through
   V1/V2/V3 and complex C.

Stop the corresponding dispatch and report when contract conflicts, workspace
ownership conflicts, unauthentic probe evidence, an incomplete acceptance
denominator, untrusted evidence, runaway resources, or the defect threshold is
triggered. Only a global shared-safety/contract problem blocks an entire stage; a
local problem blocks only its dependency closure. Final completion is judged by real
functionality and the T00 gate, not by all agents exiting.

## 13. Next Dispatch from `/53`

### 13.1 Binding Baseline and Dispatch Status

This section records the completed `/54` schema → `/55` implementation
dispatch. The bounded direct-call behavior is accepted for the covered subset;
the wider ABI and remaining language work below are still unstarted plans.
Work-order IDs are scheduling labels, never ChipIds or TaskKinds.

| Item | Required binding |
|---|---|
| Starting artifact | `t01-c01-c06/55` |
| Starting hash | `e19cf5c1dd86c7ca50da0fe429e25f4acf2ecbcfd8cd18da7efe1fd5d09d7652` |
| Known committed baseline | `/55` integration batch; capture actual HEAD and worktree state at dispatch |
| Existing closed loop | Constant return, `/49`–`/53` locals/expressions/assignments, and `/55` same-TU acyclic direct calls with 0–8 `int` arguments; modeled and native AArch64 evidence |
| Implemented call path | General parser, parameterized signatures/parameter symbols, `SE_CALL_CHIP`, explicit-callee IR, multi-function VF12 frames, multi-function AArch64 emission, candidate orchestration |
| Remaining call limits | Recursion, variadics, external/indirect calls, stack-passed arguments, wider scalar/FP/aggregate ABI, two-way reference mixed-link boundary |
| Target boundary | Linux AArch64 pinned probe baseline only; no Darwin ABI, official T00 baseline, or universal ABI claim |
| Numbering | Allocate the next amendment only after checking T01; `/54` is the direct-call common schema; the deferred restore package moves to `/56` |

Read [T01](T01_COMPILER_CONTRACT.md), [parallel rules](PARALLEL_EXECUTION.md),
[task template](TASK_TEMPLATE.md), [source slice](M1_SOURCE_LOCALS_SLICE.md),
[target slice](M1_TARGET_EMIT_SLICE.md), and the applicable T05/T06/T07/T09/T11/T13
packages before accepting a work order. ADR-0003's single-task atomic parser and
ADR-0004's ordered successor representation remain binding; changing either needs
an explicit accepted amendment, not a private worker decision.

### 13.2 First Release: Bounded Direct Calls

The proposed next closed loop is a same-TU, acyclic direct-call subset:

```c
int add(int a, int b) { return a + b; }
int main(void) { return add(2, 3); }
```

Required acceptance is modeled integer 5 and independently emitted assembly that
assembles, links, and executes as 5. This source is **not supported at `/54`**;
`/54` only freezes the AST/IR carriers.

The freeze review must accept or narrow the following scope before coding:

- Signed canonical `int` parameters/results; 0–8 named arguments, with `(void)`
  for zero parameters. No unspecified/old-style parameter lists.
- Function definitions precede uses; no separate prototype/redeclaration merging
  in this release. `main(void)` is the entry; other functions are defined in the
  same TU. Duplicate definitions and out-of-scope names are errors.
- One straight-line block per function, pure parameter reads/arithmetic, direct
  calls, and return. Preserve all existing `/53` local/assignment programs.
  Extending parameter assignment or adding multiple local declarations is not an
  implicit requirement of the call slice.
- Calls can occur in covered expression trees; argument and parameter ordering is
  explicit. Nested calls and values live across calls are tested. A deterministic
  evaluation order is permitted for this pure subset; it is not a general C
  sequencing claim or permission to reorder future side effects.
- Reject recursion/cyclic call graphs, indirect calls, unresolved/external calls,
  variadics, more than eight arguments, non-int types, pointers, aggregates, and
  stack-passed arguments explicitly. A diagnostic is not a supported-case pass.
- Bind the exact AAPCS64 revision and relevant probe fields for integer argument
  and return registers, stack alignment, preserved registers, and LR handling.
  Uncovered evidence blocks only its dependent target work; it cannot be invented.
- Define finite function, argument, nesting, frame, and interpreted-work budgets.
  Existing tick/task limits do not bound recursion inside one chip computation.
  Failure must be typed and deterministic, not a stack overflow or panic.

### 13.3 Freeze Gate: One Integrator, Parallel Input Review

`CALL-F0` is the sole shared-contract writer. It begins as `NEEDS_FREEZE` and
collects read-only reviews from the future parser, semantics, IR/VF, and ABI owners
in parallel. Reviewers return findings to the dispatcher; they do not each create
a competing contract document or implementation type.

The release packet must contain all of the following before dependent coding is
marked `READY`:

| Contract obligation | Decision/output owned by CALL-F0 |
|---|---|
| AST | Append-only kinds; exact function/parameter/callee/argument child roles, token spans, parent links, atomic append and depth failure behavior |
| Symbols/types | Function-specific signatures, parameter ordinal and scope, declaration visibility, duplicate rules, canonical type reuse |
| Semantic tasks | Exact payload/result, call fact and effects, guard/stage, unique chip IDs, field manifests, diagnostics, reuse and invalid-input behavior |
| IR | Explicit callee identity, ordered arguments, parameter-to-value binding, result ownership, function/block/value membership and call-graph rules |
| Interpretation | Entry selection, isolated per-call values/locals, return continuation, depth/work accounting and structured failure; state persisting across ticks lives in the bus |
| Target | Covered scalar ABI, caller/callee save obligations, all incoming arguments saved before scratch-register use, aligned frames, LR preservation, frame-size rejection |
| Persistence | Matching snapshots/codecs, schema tables, draft materialization/commit checks, version/hash and dependent freeze tests |
| Integration | Per-function producer order, symbol-to-function resolution without dangling committed records, routing/registration/allowlist, test discovery |

Do not overload `InstructionRecord.local` with a callee or assume existing
`FunctionRecord` fields already encode parameters. Avoid introducing a full new
machine pipeline just for this slice; retain the explicit host-only emitter
classification unless separately authorized. Temporary pure evaluation scratch is
allowed; hidden persistent interpreter/call state is not.

Freeze exit: accepted T01 text and implementation schema agree, the artifact hash
recomputes, affected codec/commit/contract tests pass, synthetic producer/consumer
fixtures compile, and every work order pins the same commit/version/hash. A
schema fixture passing does not establish the call feature itself. If a decision
conflicts with an accepted ADR, pause for that decision rather than silently
changing the architecture.

### 13.4 Exclusive Coding Lanes After Freeze

`compiler/` paths below are exact ownership boundaries, not permission to edit a
whole directory. Each lane gets one worktree and one writer. CALL-F0 owns all
shared changes; other lanes submit change requests when a required field is absent.

| Work order | Exclusive production files | Exclusive test delivery | Depends on / unit exit |
|---|---|---|---|
| CALL-P1 Parser | `compiler/src/chips/parse/pa_tu.rs` | Proposed new `compiler/tests/call_parse.rs` | F0; real multi-function AST, parameters, calls, punctuation, atomic failure, bounded nesting |
| CALL-S1 Binding/types/semantics | `compiler/src/chips/types/ty_types.rs`, `compiler/src/chips/symbols/ty_scope.rs`, `compiler/src/chips/symbols/ty_symbol.rs`, `compiler/src/chips/semantic/se_function.rs`, `se_return.rs`, `se_identifier.rs` in that same semantic directory; proposed new `compiler/src/chips/semantic/se_call.rs` | Proposed new `compiler/tests/call_semantic.rs` | F0; exact scoped parameter/callee binding, signatures, arity/types, effects, fact reuse; no whole-phase helper |
| CALL-I1 IR lowering | `compiler/src/chips/ir_lower/ir_function.rs` | Proposed new `compiler/tests/call_ir.rs`; existing `compiler/tests/c15_ir.rs` | F0; parameter values and direct calls, stable IDs, function-local ownership, argument order, no dropped operations |
| CALL-V1 Verification/modeling | `compiler/src/chips/verify/vf_interpret.rs`, `vf_syntax.rs`, `vf_invariant.rs` in that same verify directory | Proposed new `compiler/tests/call_verify.rs`; existing `compiler/tests/c17_vf12.rs` | F0; malformed AST/IR/call rejection, bounded explicit frames, independent values/locals and correct nested return |
| CALL-A1 AArch64 emitter | `compiler/src/bin/candidate/target_m1.rs` | Existing `compiler/tests/c55_target_m1.rs` and inline emitter tests | F0 plus covered target evidence; multi-function symbols, argument moves, LR and live-value preservation, legal frames |
| CALL-T1 Independent acceptance | No production changes | Proposed new `compiler/tests/call_source.rs`, `compiler/tests/call_abi.rs` | F0 fixture specification; hand-derived expected values, source replay, native and two-way mixed-link evidence |
| CALL-R1 Independent review | Read-only | Findings returned to integrator, no shared report writes | Review F0 and deliveries; invariants, source-to-IR meaning, ABI/evidence independence |
| CALL-F0 Integration | Shared files listed below; `compiler/src/bin/candidate.rs` | Existing `compiler/tests/c56_source_locals.rs`, `h04_candidate.rs`, `c42_wiring.rs`, `freeze.rs`; any schema-dependent fixture migrations | Owns freeze, per-function orchestration, registration, integration and final evidence |

The abbreviated filenames in S1/V1 refer only to the explicitly named directory,
not similarly named files elsewhere. New paths above are **proposed, not existing
test entry points**. The integrator must confirm availability and Cargo discovery
before assigning them. Existing tests not assigned in this table remain read-only
to lane writers; F0 handles necessary compatibility migrations.

CALL-F0 exclusively owns `compiler/src/{bus,task,manifest,contract,records,commit,
snapshot,routing,limits}.rs`, all `mod.rs` files, Cargo files/locks, contract
markers/manifests, shared registry tests, and dispatch/status documentation.
No worker may allocate a shared ID, change a result tag, add a dependency, or
reactivate parked drafts on its own. A manifest method inside an assigned worker
is that worker's delivery; aggregation/allowlist changes belong to F0.

This deliberately keeps binding/type/semantic edits together: splitting them into
more writers immediately would contend on shared function/signature assumptions.
Split S1 only after a concrete file-disjoint handoff and independent task contracts
are frozen. Parser depth fixes stay with P1, lowering depth fixes with I1, and
host traversal fixes with F0; a parallel “resource fix” writer must not edit them.

### 13.5 Launch Sequence, Capacity, and Integration

| Release gate | Work allowed concurrently | Exit / next release |
|---|---|---|
| G0 Baseline captured | F0 inspection; P1/S1/I1/V1/A1 read-only interface reviews; T1 independent case design | Actual HEAD, clean/dirty paths, artifact, resources, toolchain and ownership recorded |
| G1 Contract frozen | P1, S1, I1, V1, A1 code against frozen synthetic inputs; T1 tests; R1 review; F0 wiring | Independent deliveries pass; no private schemas or overlapping writers |
| G2 Pairwise integration | F0 integrates P1→S1→I1; V1 and A1 validate real produced IR; T1 runs source cases | Real producers/consumers agree; synthetic-only status remains separate |
| G3 Closed loop | F0 target queue; T1 native/mixed-link checks; R1 evidence review | Required valid source cases model, emit, assemble/link/run; invalid/unsupported cases fail correctly |
| G4 Batch accepted | F0 final regression and documentation; next-wave interface review can overlap | Logical commits only when authorized; record exact evidence, blockers and next ready work |

At G0, implementation of the new call feature is blocked, but review/test design
can start immediately. At G1, five production lanes become eligible together;
the integrator, independent tester, and reviewer are separate roles, not five
additional feature implementations. These are logical lanes, **not a promise of
eight simultaneous model sessions**.

Initial operational caps: at most four active child sessions, two concurrent
builds, one full regression, and one native target job. The integrator can own a
lane or queue ready work when tool capacity is lower. Increase to five production
writers only after available agent capacity, memory, and review throughput are
confirmed. Worktree builds use separate target/output/temp directories. A review
backlog of three deliveries pauses new coding launches; review proceeds first.

Workers return a handoff after one bounded iteration (default 45 minutes; one
targeted retry). Timeout/resource exhaustion returns `BLOCKED_RESOURCE` or
`DEFECT`, not an unbounded retry. The dispatcher may set different limits in the
work order before launch. These are execution budgets, not completion-time
estimates; gates, rather than a calendar date, determine when parallelism expands.

F0 merges schema-compatible deliveries in dependency order, never by completion
order alone. A test lane can submit failing acceptance cases before the producer
exists, but they remain integration-pending, not a passing delivery. Any schema
change pauses its dependency closure, updates the freeze and fixtures, then
rebases/retests affected lanes. Unrelated Host work can continue. Do not reset
another writer's files or silently discard a failed package.

### 13.6 Required Acceptance Matrix

| Family | Required cases / independently checked property |
|---|---|
| Regressions | Canonical constant return and every `/53` local expression/assignment case retain their modeled/native meaning |
| Arity/order | `(void)`, 1, 2, and 8 int parameters; 9 rejected; distinct positional values detect reversal, dropped and duplicated arguments |
| Scope | Same parameter spelling in different functions, repeated identical signatures, duplicate names/definitions, unknown callee and use-before-definition |
| Evaluation | Nested `add(inc(2),inc(3))` → 7; `inc(2)+inc(3)` → 7; caller values survive two calls; no shared local/value map between invocations |
| Failure | Arity/type mismatch; missing/dangling IDs; wrong function/block/value owner; missing argument/result; malformed call shape; recursion/cycle rejected |
| Limits | Argument, AST/traversal depth, function count, call depth, interpreted-work and frame limits at boundary and just above; no partial append or panic |
| Arithmetic | Defined signed-int examples, negative results and repeated literals; overflow/divide-by-zero diagnosed in the model, not compared as portable C behavior |
| Determinism/isolation | Identical initial input/config/pins yields identical records/results/trace; nonmatching task and foreign write proposals cannot mutate unrelated stores |
| Target | Multiple function labels; distinct argument registers; aligned SP; LR restored; live values preserved; frame too large rejected before artifact success |
| ABI counterpart | Candidate callee + reference caller, and candidate caller + reference callee, for the covered int subset; independent reference compilation never compiles the candidate function under test |

The eight-argument fixture must use a noncommutative expression as well as a sum.
Use full modeled integers or an independent counterpart assertion to distinguish
results that share the low eight exit-status bits. Linux process exit 254 is not
by itself proof of modeled -2. Keep reference source, candidate source, commands,
assembly, executable status and provenance separate.

External calls remain unsupported in normal source input. If two-way mixed-link
checks need a test-only IR/artifact fixture, freeze its narrow boundary and have
A1/F0 generate it through the same emitter; do not secretly broaden the frontend
or count fixture-only linkage as source support. If that boundary is unavailable,
record ABI acceptance as blocked, not complete.

The current literal-fold path excludes octal zero. Do not claim source-level
zero/divide-by-zero coverage from IR fixtures; either close that prerequisite
under its own work order or retain the explicit source limitation.

### 13.7 Verification Commands and Evidence

Existing entry points, to run from the repository root after relevant changes:

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test freeze
cargo test --locked --manifest-path compiler/Cargo.toml --test c13_ty
cargo test --locked --manifest-path compiler/Cargo.toml --test c39_sefunc
cargo test --locked --manifest-path compiler/Cargo.toml --test c15_ir
cargo test --locked --manifest-path compiler/Cargo.toml --test c17_vf12
cargo test --locked --manifest-path compiler/Cargo.toml --test c55_target_m1
cargo test --locked --manifest-path compiler/Cargo.toml --test c56_source_locals
cargo test --locked --manifest-path compiler/Cargo.toml --bin candidate
cargo fmt --manifest-path compiler/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path compiler/Cargo.toml
git diff --check
```

After the proposed test files are installed, run the corresponding `--test
call_parse`, `call_semantic`, `call_ir`, `call_verify`, `call_source`, and `call_abi`
targets; these commands are not valid entry points until those files exist.
Run root fmt/clippy/workspace tests at the batch gate and standalone tool checks
when those tools change. Preserve command exit status: capture the command to a
log and check success before summarizing; do not pipe to grep/awk and infer that
an empty failure listing or summed pass count establishes a successful run.

Native runs bind the pinned Linux arm64 image digest/toolchain from the probe
evidence, record assembler/linker/runtime versions, and use bounded Host process
supervision. Check architecture before running: existing Linux tests can return
early on non-AArch64 GCC, so a green host test is not native execution evidence.
Do not silently pull a new image, download a corpus, or use the Rust host ABI.

Each handoff states baseline and delivery commits (if authorized), exact files,
task kind/guard/read/write fields, supported rules, actual commands and statuses,
raw evidence paths, skipped tests, blockers and integration prerequisites. F0
alone updates [DISPATCH_LEDGER.md](DISPATCH_LEDGER.md) with role/session, state,
unit/integration/closed-loop levels and evidence; no pending item becomes complete
merely because another lane's build passed.

### 13.8 Independent Host Work and Subsequent Release Trains

The following are dependency-gated queues, not automatically ready work orders.
They may overlap CALL development only with disjoint files and a reviewed shared
contract. Keep one feature writer per parser, lowerer, interpreter and emitter.

| Queue | Scope / initial owner boundary | Gate before implementation / release evidence |
|---|---|---|
| HOST-RUN | H05 supervision in `tools/runner/src/{lib,main}.rs`, `tools/runner/tests/runner.rs` | Existing runner contract; audit/fix timeout, output, cancellation and descendants with tests; never advertise filesystem/network sandboxing |
| HOST-CORPUS | H00 offline lock/provenance in `tools/torture/src/verify.rs`, `tools/torture/tests/verify.rs` | Freeze authorized corpus revision/inventory/hash/licensing before acquisition; offline validation may proceed without candidate calls |
| HOST-DRIVER | H02/H03 orchestration, H06 aggregation, H04 formal compile/link flag path | First freeze instance/result schemas and single-owner exact paths; these handlers are not assumed implemented; H04 overlaps F0's candidate file and waits for handoff |
| LANG-LEX | Decimal/octal/hex integer decoding/type rules and the zero-literal gap | Identify actual producer/fold owners and exact files first; frozen integer model; source + const + VF boundary tests, no duplicate folder |
| CFG | if/else first, then loops/break/continue and short-circuit | Freeze condition types, branch/terminator verification, per-function block order and interpreter step budgets; reuse authoritative `successors`; source→CFG→native closed loops |
| MEMORY | Multiple locals/globals, address/load/store, pointers/arrays | Freeze object identity/lifetime, target size/alignment, address/effect rules and static-data/relocation boundary; layout/type schemas alone do not implement memory |
| ABI-EXT | Stack arguments, then wider scalars/floats/aggregates/variadics | Separate probe/spec evidence and call/frame contracts; two-way mixed linking per supported family; do not extend the int-only acceptance claim |
| RESTORE | Deferred snapshot restore/quota>1/VF13/StorePatch package | Its own accepted contract, unique amendment allocation and differential evidence; not a dependency for source-code development parallelism |
| COMPLEX | Aggregate layout/init, VLA, old-style/GNU features | Frozen per-family interfaces and corpus-driven demand; independent language/ABI tests and real producers, not stub count |
| OPT | T10 legal rewrites and analyses | Verified IR semantics/version guards/invalidation; differential semantic validation before performance claims |
| T00-GATE | Frozen corpus plus H02/H03/H06/H09 complete chain | Compile-only vs execute/IEEE retained separately; frozen denominators/options/environment; reproducible runs and failure owners; apply T00 thresholds only to actual evidence |

CFG and MEMORY preparation may run concurrently with CALL, but their coding may
not touch the active call parser/lowerer/emitter worktrees. To overlap production,
the integrator must first freeze a file-disjoint extension boundary or finish and
hand off the contested files. Do not perform a broad refactor merely to increase
agent count. HOST tasks can use spare slots once exact ownership/resource and
security gates are satisfied; they do not consume arbitrary network permissions.

Later releases repeat F0 freeze → file-disjoint production → independent review →
real producer integration → native/Host acceptance. The whole A–G objective is
not complete until its remaining Host/corpus/general-codegen obligations pass;
CALL acceptance only adds one bounded capability. More than 99% remains a T00
goal, not a result supplied by this dispatch procedure.
