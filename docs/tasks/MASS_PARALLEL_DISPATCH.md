# Mass-Parallel Compiler Development Dispatch Plan

| Field | Value |
|---|---|
| Status | **Dispatch planning, not a contract freeze or implementation authorization** |
| Date | 2026-11-07 |
| Scope | Compiler application, Host tooling, verification, and integration; does not extend root-framework responsibilities |
| Goal | Provide 100–150 parallel task slots when interfaces and file isolation permit, while prioritizing a real execution closed loop |
| Checked baseline | `t01-c01-c06/53`; re-read the version, hash, and workspace state at dispatch time |
| This document does not provide | Started agents, completed implementations, rerun tests, target probe results, or a GCC pass rate |

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
| Application contract | [CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION) is `/39` | Subsequent small-batch freezes, dependency revalidation, registration, and permission consistency |
| Preprocessing | Several implemented slices with bounded capability | Driver integration, real header loading, recursive include, once state, complete macro-conditional rules |
| Lexing | Integer M1 subset plus float, character, and string slices | Integer radix/suffix/type, adjacent strings, dialects, recovery, and downstream integration |
| Parsing | [pa_tu.rs](../../compiler/src/chips/parse/pa_tu.rs) is still limited to the M1 shape; some productions are Ack-only checks | General declarators, expressions, statements, initialization, and real AST production |
| Types and semantics | M1 subset exists | Scopes, compatibility/conversions, lvalues, pointers, calls, effects, and control-flow legality |
| Constants/layout/initialization | Folding and some conditional checks exist | Target widths, layout, bitfields, initialization plans, VLA |
| IR | [ir_function.rs](../../compiler/src/chips/ir_lower/ir_function.rs) mainly produces Constant/Return | Variables, memory access, CFG, calls, dynamic stack, and verification |
| Target backend | AArch64 identity is fixed; concrete target values are unverified | Probe, ABI classification, machine records, instructions/frames/registers/assembly output |
| Host | [candidate](../../compiler/src/bin/candidate.rs) has evidence output and `-E/-D/-U` | `-I`, multiple sources, `-S/-c`, isolation, official test driver, statistics, and CI |
| Acceptance | Corpus lock/probe scaffolding exists; no final acceptance evidence | Freeze corpus/environment/options/denominator, run, and retain failures |

At inspection time the workspace also contained uncommitted parse and lex candidate
files and freeze drafts. Some files are not in a module, and some only Ack the old
M1 shapes or explicitly report Unsupported. They enter audit first; they do not
count as completed capability, and new tasks must not overwrite them. Older index
descriptions such as `/5`, `/9`, or "no chips" cannot serve directly as current
facts.

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
