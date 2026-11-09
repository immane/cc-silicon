# A01 Framework Baseline Audit

| Field | Value |
|---|---|
| Status | Read-only audit complete; source/test/contract files were not modified |
| Date | 2026-11-07 |
| Task | Stage A `A-AUD-01`: root `cc-silicon` framework and stated trust boundaries |
| Repository | `/Volumes/Nayuki/Development/Rust/cc-silicon` |
| Branch / HEAD | `wave4-development` / `430ae29162efef90f86667f384097964d611604f` |
| HEAD commit | `docs: add mass-parallel compiler dispatch plan`, 2026-11-07T14:52:26+08:00 |
| Scope | `src/`, `tests/`, `examples/`, root `README.md`, architecture/SFL/design status claims, and the compiler application's use of the root framework |
| Out of scope | Full compiler semantics, Host/torture/probe execution, GCC results, accepted-ADR changes |
| Deliverable | This file only (`docs/reviews/dispatch/A01_FRAMEWORK_BASELINE.md`) |

## 1. Checked Revision, Workspace State, and Evidence Classes

### 1.1 Workspace state at audit time

The worktree was dirty before this audit. The dirty files are compiler-application
work and draft task documents; they are outside the root framework's `src/`,
`tests/`, and `examples/`. They were inspected only where relevant and were not
modified.

```text
 M compiler/src/chips/parse/mod.rs
?? compiler/src/chips/lex/lx_error.rs
?? compiler/src/chips/lex/lx_extension.rs
?? compiler/src/chips/parse/pa_cast.rs
?? compiler/src/chips/parse/pa_decl_spec.rs
?? compiler/src/chips/parse/pa_expr_ext.rs
?? compiler/src/chips/parse/pa_ext_dispatch.rs
?? compiler/src/chips/parse/pa_init.rs
?? compiler/src/chips/parse/pa_stmt.rs
?? compiler/src/chips/parse/pa_typedef_query.rs
?? docs/tasks/CT_FREEZE_01.md
?? docs/tasks/CT_FREEZE_02.md
?? docs/tasks/CT_FREEZE_03.md
?? docs/tasks/H04_I_FREEZE.md
?? docs/tasks/OP_FREEZE_01.md
?? docs/tasks/OP_FREEZE_02.md
?? docs/tasks/OP_FREEZE_03.md
?? docs/tasks/VF13_FREEZE.md
```

`git diff --stat` showed only `compiler/src/chips/parse/mod.rs` (5 insertions:
new `mod` declarations). The untracked parse/lex files are user work and are not
counted as framework evidence. Root `src/`, `tests/`, `examples/`, and
`Cargo.toml` were clean. Build artifacts under `target/` are ignored and may
have changed when the targeted checks below were run.

Other sessions created additional files under `docs/reviews/dispatch/`
(`A05_IR_TARGET_HOST.md`, `A09_MINIMAL_TARGET_CHAIN_PLAN.md`,
`A10_HOST_H04_H05_PLAN.md`, `A13_TESTS_CI_DOCS.md`) and
`docs/tasks/DISPATCH_LEDGER.md` while this audit was running. They are not part
of A01 and were not read or modified here.

### 1.2 Artifact identity

- Root package: `cc-silicon` 0.1.0, Rust 2021, no runtime dependencies
  (`Cargo.toml:1-15`).
- Pinned toolchain: `rust-toolchain.toml:1-3` (1.99.0).
- Compiler application contract: `t01-c01-c06/39`, hash
  `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24`
  (`compiler/contracts/CONTRACT_VERSION`; `compiler/src/contract.rs:38-44`).
- The root framework itself has no separate frozen contract version. `/39` is
  the compiler package's artifact, not a semantic proof for root `src/`.
- Several documents still cite older artifacts; see Finding F-13.

### 1.3 Evidence classes used below

- **Implemented** — present in source and, where stated, exercised by a command
  recorded in §6.
- **Implemented with limits** — present, but the listed guarantee is enforced
  only by convention/review/tests, not by the type system.
- **Frozen contract** — normative document or hash; not necessarily implemented.
- **Proposed/planned** — task catalogs, ADRs, acceptance plans, or dispatch
  plans.
- **Unverified** — a claim not checked by this audit and not proven by a
  command recorded here.

## 2. Implementation Status

### 2.1 Root framework inventory

| Capability | Status | File:line evidence | Boundary / limitation |
|---|---|---|---|
| Root unsafe-code policy | Implemented | `src/lib.rs:18` | `#![forbid(unsafe_code)]` applies to this crate; dependencies and application crates are not covered |
| `Bus` trait: `Pins`, `Wires`, register/wire access | Implemented | `src/bus.rs:27-67` | Flat/no-heap/no-interior-mutability wording at `src/bus.rs:22-26` is policy; trait bounds do not enforce it |
| Wire reset | Implemented default | `src/bus.rs:45-47`; called at `src/motherboard.rs:114-115` | Overridable by any `Bus` implementation; the framework cannot prove all wires reset |
| Latch and tick hooks | Implemented hooks | `src/bus.rs:54`, `src/bus.rs:60-67`; called at `src/motherboard.rs:120-122` | Default latch is a no-op; default tick count is constant 0 |
| `LogicChip` | Implemented | `src/chip.rs:29-37` | Receives `&B::Pins` and `&mut B`; no ZST/read-set/write-set/phase/determinism enforcement |
| `RestrictedChip` | Implemented | `src/chip.rs:49-63`; size assertion at `src/chip.rs:53` | Size-0 is checked; global state, I/O, and effects are not checked |
| `ChipAdapter` / `ProjectedChip` | Implemented | `src/chip.rs:84-124` | Adapter is the trusted boundary; framework cannot inspect its field accesses (`src/chip.rs:80-83`) |
| `silicon_chip!` | Implemented | `src/chip.rs:211-232` | Unit struct, ZST `const` assert (`:224`), `SameAs` impl-target check (`:225`); compile-fail doctests were not run |
| `Motherboard` layers/install | Implemented | `src/motherboard.rs:20-72` | `layers` is public (`:23`); `install_chip` panics on a bad layer (`:57-64`); topology allocation is construction-time |
| `clock_tick` lifecycle | Implemented | `src/motherboard.rs:113-123` | Reset wires → backend propagation → latch/advance hooks; pins are supplied by the caller; no transaction/double buffer |
| `Backend` / `CpuBackend` | Implemented | `src/backend.rs:23-53` | Only `name` plus provided/trait `execute_layers`; no capability matrix or differential contract |
| `Clock` | Implemented | `src/clock.rs:16-68` | Host wall-clock helper; no reference in `tests/` or `examples/`; not integrated with `Motherboard` |
| `simulate` | Implemented | `src/sim.rs:13-22` | Driver only; no assertions or invariant checking |
| `Testbench` | Implemented | `src/sim.rs:28-64` | Public fields; wraps driver state; not a verifier |
| Counter example | Implemented | `examples/counter.rs:55-159`; run in §6 | Demonstrates `LogicChip`, layered propagation, reset, edge detection; no test assertion in the example |
| Framework test inventory | Implemented test file | `tests/paradigm.rs`; `cargo test --test paradigm -- --list` reported 15 tests | 6 exact tests were run and passed; full 15, property tests, and doctests were not run |
| Optional AST chip linter | Implemented separate package | `tools/chip-lint/src/lib.rs:46-107`; tests in `tools/chip-lint/tests/lint.rs` | Scans source text; not part of root crate; coverage limits in Finding F-11 |
| SFL per-chip manifest (`reads`, `writes`, `phase`, `deterministic`, `backend_class`) | Not implemented in root; partial in compiler app | `docs/architecture/SFL_CONTRACT.md:119-125`; compiler fields at `compiler/src/manifest.rs:384-406` | Root `LogicChip`/`RestrictedChip` expose no manifest; compiler `ChipManifest` is declarative and validated as a lint, not read-enforcement |
| SFL backend capability matrix | Not implemented | `docs/architecture/SFL_CONTRACT.md:197-208`; `src/backend.rs:23-43` | No capability query, reject/emulate API, or backend diff test |
| Pins sampling/freezing | Not implemented as a framework helper | `src/bus.rs:28-32`; `src/motherboard.rs:111-112` | Caller must freeze pins; `Clock` only samples elapsed time |
| Root snapshot/replay | Not implemented in root | `src/bus.rs:60-67`; `src/sim.rs:13-22` | Only an application-defined tick count; compiler snapshot facility lives in `compiler/` |
| Compiler runtime via root `Motherboard`/`RestrictedChip` | Not used | `compiler/src/bus.rs:34`; `compiler/src/routing.rs:22,602-675`; `compiler/src/chips/mod.rs:137-150` | Compiler shares only `cc_silicon::Bus`; its runtime is `Worker` + `RoutingShell` |
| Frozen compiler contract | Frozen artifact | `compiler/contracts/CONTRACT_VERSION`; `compiler/src/contract.rs:38-44`; T01 amendments through `/39` | Hash covers normative shapes/rule IDs, not source code or semantic equivalence (`T01:876-883`) |

### 2.2 Contract, plan, and claim classification

**Implemented and exercised by this audit (targeted only):** root `Bus` accessors
and defaults, wire reset, latch-driven edge detection, `Motherboard::clock_tick`,
`Backend` custom-install path, `ProjectedChip` adapter installation, `Testbench`,
and `cargo check --lib` / `cargo run --example counter`.

**Implemented but not exercised by this audit:** `Clock` (no test reference),
`Motherboard::push_layer`, `Motherboard::default`, `install_chip` panic path,
`debug`/`Display` implementations, doctests (including the compile-fail checks),
full property tests, and compiler application workers.

**Frozen contract (compiler application, not root):** `t01-c01-c06/39` and the
T01 /6–/39 amendment sequence. The root framework is not a versioned part of
this artifact.

**Proposed/planned, not implemented evidence:** T02–T13 language chips and the
complete C pipeline, ADR-0002 staged pipelines, mass-parallel work orders,
Linux AArch64 probe/attestation, GCC torture acceptance, and multi-backend
realizations.

**Documented but absent as root mechanisms:** SFL chip manifests with enforced
read sets, backend capability matrices, a graph validator, snapshot/replay,
pins sampling/freezing, transactional latch semantics, and synthesis/proof
tooling. Some of these exist partially in `compiler/`; none are root guarantees.

**Unverified claims from README/docs:** the `/39` test suite passes, the M1
frontend loop is closed end to end, the ABI probe harness self-tests pass, the
torture lock schema/verifier pass, and macros/slices listed at `README.md:13`
are "working" in the sense of real end-to-end execution. This audit did not run
those suites.

## 3. Trust-Boundary and Documentation Findings

### 3.1 Root enforcement boundaries

**F-01 — The legacy `LogicChip` boundary is fully privileged.**
`LogicChip::tick` receives the whole `&mut B` (`src/chip.rs:29-37`). There is no
ZST check, field-scoped view, manifest, phase check, or determinism check on
this path. The docs call chips "stateless" and "single responsibility"
(`src/chip.rs:11-17`), but that is a design policy. Root tests and
`examples/counter.rs` use this path. Impact: any framework user selecting the
legacy API gets no mechanical isolation. The README trust table acknowledges
that the root tick driver cannot guarantee purity, termination, or panic-freedom
of user chips (`README.md:390`).

**F-02 — `RestrictedChip` is narrower but not a complete sandbox.**
The chip body gets only a read-only projection and returns a typed output
(`src/chip.rs:39-63`), and the `ProjectedChip` wrapper enforces the chip's
zero-size assertion at construction/installation (`src/chip.rs:99-108`;
`src/motherboard.rs:80-88`). However:

- The `ChipAdapter` is explicitly trusted and the framework cannot inspect which
  bus fields it reads or writes (`src/chip.rs:80-90`).
- `ProjectedChip` stores an adapter `A` and only asserts `C`'s size; there is no
  `size_of::<A>() == 0` check and no adapter-state restriction
  (`src/chip.rs:93-108`). An adapter can hide fields or use interior mutability
  and still sit behind the "restricted" path.
- The ZST check proves only size zero, not absence of statics, globals, I/O,
  wall-clock use, or nondeterminism; the source says so
  (`src/chip.rs:45-48`).
- A manual `RestrictedChip` implementation can call `compute` directly without
  ever constructing a `ProjectedChip`; the installation-time assertion is not
  reached in that use. The `silicon_chip!` declaration path does add a
  module-level size assertion (`src/chip.rs:224`), but direct trait use does not.

**F-03 — Motherboard/Backend ordering is a policy, not a sealed API.**
`Motherboard::layers` is public (`src/motherboard.rs:23`), so callers can
insert, reorder, or obtain references to chips without using `install_chip`.
`Bus::reset_wires` and `Bus::latch` are public trait hooks, and `clock_tick` is
public (`src/bus.rs:45-54`; `src/motherboard.rs:113-123`). A custom `Backend`
receives the whole layer slice and mutable bus and may call, skip, repeat, or
reorder chips; the default implementation is only the reference path
(`src/backend.rs:27-42`). The docs state that the motherboard is the only legal
invocation/reset/latch entity, but the README correctly admits that public Rust
methods do not prevent bypass (`README.md:277-279`). This is an architectural
trust boundary, not a mechanically enforced one.

**F-04 — `latch` is an application hook, not a commit transaction.**
`clock_tick` runs wires reset → backend propagation → `bus.latch(pins)` →
`bus.advance_tick()` (`src/motherboard.rs:113-123`). There is no double-buffered
old-state snapshot: an earlier chip's register write is visible to later chips
in the same tick. `latch` is whatever the application implements; its default is
a no-op (`src/bus.rs:54`). The README describes this correctly
(`README.md:213-217`), but the phrase "registers written by later layers are
visible only after the tick completes" (`src/motherboard.rs:17-19`) is easy to
misread as a latch commit barrier. Consequence: callers must not assume a
generic end-of-tick transaction. The compiler application makes the divergence
concrete: `CompilerBus` commits through its routing/commit path during
propagation and leaves `Bus::latch` at the default no-op
(`compiler/src/bus.rs:1236-1255`).

**F-05 — "Frozen pins" is not enforced.**
`Bus::Pins` is only bounded by `Clone + 'static` (`src/bus.rs:28-32`), and chips
receive `&Pins`. Rust references prevent ordinary mutation but do not prevent
interior mutability (`Cell`, `RefCell`, atomics, etc.). There is no framework
sampling/freezing helper; `Clock` measures wall-clock deltas but never
constructs pins or synchronizes with the motherboard (`src/clock.rs:16-68`;
`src/motherboard.rs:111-112`). Deterministic replay therefore depends on the
application owning honest, immutable pin data. The README trust table says the
root driver does not guarantee purity of hooks/chips/backends
(`README.md:390`).

**F-06 — The "Lamport clock" is application-owned and optional.**
`Bus::tick_count` defaults to 0 and `Bus::advance_tick` defaults to no-op
(`src/bus.rs:60-67`). The motherboard calls `advance_tick`, but the framework
never reads `tick_count` and does not maintain a tick counter itself. The
`lamport_clock_advances` test only works because the test bus implements both
hooks (`tests/paradigm.rs:64-69`, `:211-215`). A framework user that forgets them
gets a constant-0 clock with no compile-time warning. The root README notes the
defaults (`README.md:209-211`), but the trait's "monotonic tick counter" wording
overstates the guarantee for default users.

**F-07 — The flat/no-heap bus constraint is documented but not type-enforced.**
`src/bus.rs:22-26` and `SILICON_PARADIGM_SPEC.md:67-69` require a flat bus with
no heap/interior mutability/threads. The trait itself only requires
`Pins: Clone + 'static` and `Wires: Default + Clone + 'static`
(`src/bus.rs:27-32`). ADR-0001 explicitly grants the compiler application a
dynamic-arena exception (`docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md:12-66`),
and `CompilerBus` is dynamic (`compiler/src/bus.rs:904-957`). That exception is
accepted, but the root `Bus` rustdoc and the blueprint's "no Vec/HashMap/Box"
rule (`docs/design/ARCHITECTURAL_BLUEPRINT.md:61-70`) do not point at it. A new
application can violate the generic bus rule without any compiler or clippy
error; only review/tests catch it.

### 3.2 Compiler application boundary

**F-08 — The compiler runtime does not use the root `Motherboard`,
`RestrictedChip`, `ProjectedChip`, `Backend`, or `Clock`.**
Static search of `compiler/src`, `compiler/tests`, and `compiler/README.md`
found only `cc_silicon::Bus` (used at `compiler/src/bus.rs:34` and
`compiler/src/routing.rs:22`). The actual runtime is:

- `Worker` with `handle(&self, task, bus: &CompilerBus) -> Vec<Proposal>`
  (`compiler/src/chips/mod.rs:136-150`);
- `WorkerRegistry` with a zero-size check (`compiler/src/chips/mod.rs:169-178`);
- `drive_task`/`handler_for` identity/stage checks
  (`compiler/src/chips/mod.rs:309-330`, `:364`);
- `RoutingShell::clock_tick`/`clock_tick_with`, which implement their own
  reset → propagate → latch/advance lifecycle
  (`compiler/src/routing.rs:602-675`).

T01 §4.1 says the `RestrictedChip` path "already landed" and that "new chips in
the compiler application must use RestrictedChip" (`T01:86`), with
`ChipAdapter`/`ProjectedChip`/`Motherboard::install_projected` as the integration
path (`T01:86-101`). The source contradicts that mandate: compiler chips use
`Worker`, not `RestrictedChip`, and the compiler driver uses `RoutingShell`, not
`Motherboard`. The README partially acknowledges the deviation ("not a full
RestrictedChip installation chain", `README.md:404-406`), but T01 §4.1 is still
normative-looking. This is a contract/implementation ownership conflict that
must be resolved before relying on the framework's "restricted" guarantee for
compiler work.

**F-09 — Compiler read isolation is declarative, not enforced.**
`Worker::handle` receives the full read-only `&CompilerBus`
(`compiler/src/chips/mod.rs:149`). Individual workers then build narrow
projections manually (for example `project_fold_input` at
`compiler/src/chips/constant_layout_init/fold.rs:67-100` and the `PpInput`
projector at `compiler/src/chips/preprocess/pp_normalize.rs:46-99`). Those
projectors are application code and may read any public bus field. The
`ChipManifest.reads` list (`compiler/src/manifest.rs:384-406`) is declarative;
the driver checks identity, stage/layer, registration, owner, and kind but not
the actual read set (`compiler/src/chips/mod.rs:309-330`). Commit checks writes
against manifests, but not reads. T01's grading table explicitly says this
cannot be proven by an ordinary trait and needs generated private views or
structured patches (`T01:90-101`). The README trust table also says adapter
writes/projections are not automatically verified (`README.md:391`).

**F-10 — There are two tick drivers and no differential test between them.**
Root `Motherboard::clock_tick` (`src/motherboard.rs:113-123`) and compiler
`RoutingShell::clock_tick` (`compiler/src/routing.rs:602-675`) implement similar
reset/latch/advance sequences but differ in return type, cancel handling, tick
recording, and propagation logic. Root tests exercise only `Motherboard`;
compiler tests exercise only `RoutingShell`. No such differential test was found
that compares their observable lifecycle on an equivalent bus. A future change
to root wire-reset/latch semantics, or to compiler cancellation/latch behavior,
can drift silently. This is the most concrete integration risk from F-04 and
F-08.

**F-11 — The AST chip linter does not cover the compiler's adapter/projector
surface, and its deny list can be bypassed by fully qualified paths.**
`tools/chip-lint/src/lib.rs` scans `RestrictedChip::compute` and inherent
`XChip::compute` bodies (`:109-208`, `:247-283`, `:425-460`). It does not scan
`Worker::handle`, projection functions, or other adapter code. It treats any
same-file top-level function call as resolved (`:112-119`, `:312-336`) but does
not visit the callee body. Its `DENIED_PATH_PREFIXES`/`DENIED_IDENTIFIERS`
(`:22-43`) cover `std::fs`, `std::process`, `std::env`, `std::time`, `std::thread`,
`std::net`, `std::io`, `std::sync`, `rand`, and a few identifiers, but a fully
qualified path such as `::cc_silicon::Clock::new(...).sample_delta_ns()` is not
denied. The root `Clock` is re-exported in the prelude
(`src/prelude.rs:11-17`), so wall-clock use can hide in adapter/helper code. The
README correctly calls the linter a conservative aid, not a proof
(`README.md:320-323`); the compiler guardrails require stricter AST/HIR/lint
behavior for unknown macros and indirect calls (`T01:98-101`). This gap matters
because compiler CI runs chip-lint over `compiler/src/chips`
(`.github/workflows/ci.yml:60-61`).

**F-12 — The root `Backend` contract lacks the SFL capability matrix and
differential evidence.**
`Backend` requires only `name` and provides a default `execute_layers`
(`src/backend.rs:23-43`). SFL requires every backend to publish a capability
matrix covering pins, flat bus, wire reset, per-chip/layer execution, host
fallback, deterministic replay, cross-backend diff, hardware synthesis, and
reversible subgraphs (`SFL_CONTRACT.md:197-208`). Root has only `CpuBackend`;
the README lists SIMD/GPU/FPGA/quantum as future (`README.md:695-701`). This is
acceptable as a plan, but installing a custom backend currently gives no
API-level way to declare what it supports or to make it reject unsupported
chips. The root test `CountingBackend` simply replicates CPU order
(`tests/paradigm.rs:229-274`); there is no backend diff test.

### 3.3 Documentation/status contradictions

**F-13 — Artifact identity is inconsistent across authoritative-looking docs.**
The contract markers and source say `/39`
(`compiler/contracts/CONTRACT_VERSION`; `compiler/src/contract.rs:38-44`), and
the T01 amendment trail ends at `/39` (`T01:846-872`). Yet:

- T01 §7.1's "Frozen artifact" sentence still says `/6` (`T01:145`);
- README's architecture diagram says `frozen /16` (`README.md:142`);
- README's template-status paragraph says current `/9` (`README.md:396`);
- ADR-0001 §4 still says "current frozen version is `t01-c01-c06/5`"
  (`docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md:125`).

Only the contract file and source constants are reliable current identity. This
drift can make a worker target an obsolete schema. The README itself warns that
some linked docs still describe `/5` or `/6` (`README.md:408-413`), but it does
not fix its own `/16` and `/9` references.

**F-14 — README understates implemented language slices and overstates the
Gate 1 restriction.**
`README.md:340` says "Apart from the initial fold subset, language work remains
planned", but the compiler tree already contains PP/LX/PA/TY/SE/IR/verify
workers and tests `c10`–`c39` (for example `compiler/src/chips/preprocess`,
`compiler/src/chips/lex`, `compiler/src/chips/parse`,
`compiler/src/chips/semantic`, `compiler/src/chips/verify`; T01 /10–/39 records
the corresponding freezes). The row is about the *complete* pipeline, so its
first sentence is true, but its second sentence is false. Similarly,
`compiler/README.md:7` says the package "contains no C language chips" and
`:10` says language work "builds on this foundation"; both are stale relative
to source. `docs/tasks/README.md:3` says "the compiler is not yet implemented"
without distinguishing "no complete compiler" from "many partial slices exist".
This underreporting harms trust: a reader can conclude that no language logic
exists when partial logic does.

`README.md:381-384` says "Gate 1 materializes `Literal` and `Const` bodies only.
Other append families are explicitly rejected". At `/39`, typed materialization
also covers Artifact, Name/Token, Span/PpToken, Node, Type/Symbol/Scope/Sem,
IR records, Macro, and others (T01 /10–/31 and the corresponding compiler record
types). That paragraph must be read as historical `/7`-era Gate 1 text, not
current status.

**F-15 — README's CI/probe claims do not match the current workflow.**
`README.md:658-665` says `tools/torture` is not wired into CI and that the probe
self-test is the only automatic probe check. `.github/workflows/ci.yml:33-58`
does run torture formatting, clippy, and tests, in addition to root, chip-lint,
and compiler checks. The probe workflow remains manual for the real probe
(`.github/workflows/t00-target-probes.yml`), but the torture-CI sentence is
stale. README lines 342-343 also claim probe self-tests and lock schema/verifier
tests pass; those are external claims not verified by this audit.

**F-16 — The accepted ADR's absolute limit wording conflicts with the compiler
status table.**
ADR-0001 says every configured bound is enforced before mutation
(`docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md:108-116`), while T01's
C01 status says public mutable stores and direct pushes bypass global budgets
and only the checked entry points are covered (`T01:876`). The root README's
trust table agrees that global budgets/ownership for direct public-store
mutation are not guaranteed (`README.md:393`). This is a documentation conflict
about a real trust boundary; the conservative statement (T01/README) should win
until direct-store enforcement exists.

### 3.4 What is actually proven vs merely documented

- **Proven by source structure:** the root crate exposes the listed traits and
  types; the compiler can implement `Bus`; `ProjectedChip` requires `C` to be a
  ZST when installed; `Motherboard` invokes the backend's `execute_layers`.
- **Proven by targeted runs (only):** the listed six tests and the example run
  in §6.1.
- **Not proven:** full root suite, doctests, property tests, compiler `/39`
  suite, chip-lint suite, probe, torture, target ABI, and semantic correctness
  of any C slice.
- **Not mechanically proven even if tests pass:** actual read sets, absence of
  global state, absence of wall-clock/I/O in adapters, backend equivalence,
  latch transactionality, or deterministic behavior for arbitrary application
  code.

## 4. Concrete Risks to Compiler Development

| ID | Risk | Why it matters | Mitigation direction |
|---|---|---|---|
| R-01 | Root isolation mechanisms are not on the compiler's execution path (F-08) | The compiler's `Worker`/`RoutingShell` runtime can evolve independently of the root `RestrictedChip`/`Motherboard` guarantees that T01 cites; a root test pass says nothing about compiler worker isolation | Resolve T01 §4.1 vs the `/8+` Worker template; add an integration test that exercises the chosen path |
| R-02 | Two tick drivers can drift (F-10, F-04) | Reset/latch/advance ordering, cancel handling, and tick recording are duplicated; a future change can change replay or commit timing silently | Extract a shared lifecycle helper or add a differential test over an equivalent bus |
| R-03 | Compiler worker read sets are declarative (F-09) | A worker can read unrelated state, creating hidden dependencies and snapshot/replay divergence while still passing manifest validation | Generate private projections or structured read patches; add negative read-set tests/review |
| R-04 | Linter/adapters can hide global or wall-clock behavior (F-11) | `Worker::handle`, projectors, helper bodies, and fully qualified `cc_silicon::Clock` are outside the current lint surface; nondeterminism can evade CI | Extend chip-lint coverage and deny fully qualified host paths; keep replay tests |
| R-05 | Status/contract drift (F-13, F-14, F-15) | Workers may implement against `/6`, `/16`, or `/9` while source expects `/39`; reviewers may under- or over-estimate implemented language work | Single-source status table; update T01, ADR-0001, README, compiler README in one batch with owner approval |
| R-06 | Dynamic bus profile ambiguity (F-07) | A contributor can put `Vec`/interior mutability in a generic bus or mistake the compiler exception for a framework-wide relaxation | State the ADR-0001 exception next to the root `Bus` contract and blueprint; add a lint/review check |
| R-07 | Backend capability contract absent (F-12) | If the compiler later selects a custom backend, silent skipping/reordering is possible; SFL §6 cannot be satisfied | Define capability/unsupported behavior before any non-CPU backend; add backend diff tests |
| R-08 | Pins/tick determinism is app-owned (F-05, F-06) | Interior-mutable pins or forgotten tick hooks can break replay without a compile error | Constrain pins in application schema; require explicit tick/latch implementations or helpers; test replay |
| R-09 | Verification claims are broader than executed evidence (F-15, §6) | Downstream reports may cite `/39` or M1 as "tested" when only targeted root checks ran | Keep the evidence ledger exact; never convert a plan/README claim into a pass rate |

## 5. Recommended Follow-Through Work Orders (Proposed, Not Dispatched)

These are recommendations for the integrator; they assign no implementation and
do not authorize a contract change by themselves. Shared interfaces remain
integrator-owned.

| Proposed ID | Objective | Exact owned paths | Dependency notes / gate |
|---|---|---|---|
| A01-FU-01 | Documentation truth pass: one current artifact identity, accurate partial-slice status, correct CI/probe claims, ADR exception note | `README.md`; `docs/tasks/T01_COMPILER_CONTRACT.md` (status sentences only); `docs/tasks/README.md`; `compiler/README.md`; `docs/design/ARCHITECTURAL_BLUEPRINT.md`; `docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md` (version-note only) | T01 integrator/doc owner approval; no source or schema change. Gate: every artifact reference resolves to `/39` or is explicitly historical; no false "all tests pass" claim |
| A01-FU-02 | Decide and document the compiler runtime path: retain `Worker`/`RoutingShell` as the contract, or migrate to `RestrictedChip`/`Motherboard`; reconcile T01 §4.1 | `compiler/src/chips/mod.rs`; `compiler/src/routing.rs`; `docs/tasks/T01_COMPILER_CONTRACT.md` §4.1 | Depends on A01-FU-01 and an integrator decision; a shared-interface change requires a `CONTRACT_CHANGE_REQUEST` before editing |
| A01-FU-03 | Add a differential tick-lifecycle test between root `Motherboard` and compiler `RoutingShell` | `compiler/tests/c40_tick_lifecycle.rs` (new); optionally `tests/framework_boundaries.rs` (new) | Depends on A01-FU-02; test-only, no shared interface edit |
| A01-FU-04 | Harden chip-lint: fully qualified host paths, `cc_silicon::Clock`, helper bodies, `Worker::handle`/projector coverage, unknown macros | `tools/chip-lint/src/lib.rs`; `tools/chip-lint/tests/lint.rs` | Independent; add tests for each new finding; keep the "not a proof" documentation |
| A01-FU-05 | Add framework boundary tests for `Clock`, latch/advance ordering, public `layers`, stateful adapter, and a backend that reorders/skips chips | `tests/framework_boundaries.rs` (new) | Independent; test-only. Do not claim Clock determinism beyond what the test establishes |
| A01-FU-06 | Define the `Backend` capability/unsupported contract and backend differential tests | `src/backend.rs`; `docs/architecture/SFL_CONTRACT.md` | Depends on an accepted contract/ADR; not needed before an actual non-CPU backend is proposed |
| A01-FU-07 | Move or lint-close the wall-clock helper: relocate `Clock` to a Host crate or deny all reachable forms in chips | `src/clock.rs`; `tools/chip-lint/src/lib.rs`; optionally `compiler/src/host/clock.rs` (new) | Depends on the host-boundary decision; a public-API move needs contract/ADR approval |
| A01-FU-08 | Make the flat-bus/ADR exception explicit and consider a mechanical guard | `src/bus.rs` (rustdoc only, unless a contract change is approved); `docs/design/ARCHITECTURAL_BLUEPRINT.md`; `docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md` | Depends on A01-FU-01; documentation first; a type-level/API guard is a separate contract change |

## 6. Unrun Checks and Open Questions

### 6.1 Checks actually run in this audit

| Command | Result |
|---|---|
| `rustc --version` / `cargo --version` | `rustc 1.99.0` / `cargo 1.99.0` |
| `cargo check --lib` | Passed (`Finished dev profile`) |
| `cargo test --test paradigm -- --list` | 15 tests listed, 0 benchmarks; no test executed in this step |
| `cargo test --test paradigm -- --exact wires_reset_every_tick` | 1 passed |
| `cargo test --test paradigm -- --exact latching_enables_edge_detection` | 1 passed |
| `cargo test --test paradigm -- --exact projected_chip_adapter_commits_only_its_declared_output` | 1 passed |
| `cargo test --test paradigm -- --exact testbench_runs_headless` | 1 passed |
| `cargo test --test paradigm -- --exact host_can_install_a_custom_backend` | 1 passed |
| `cargo test --test paradigm -- --exact lamport_clock_advances` | 1 passed |
| `cargo run --example counter` | Ran 40 ticks; reported `final value: 7`, `tick_count: 40` |
| Static search of `tests/` and `examples/` for `Clock` | No test/example references found; `Clock` is untested by the repository's visible test files |

No full-suite, property-test, doctest, clippy, compiler, chip-lint, torture, or
probe command was run. No pass rate or verified ABI claim is made here.

### 6.2 Not run / not established

- Full root suite: `cargo test` (all 15 tests and doctests), including all
  `proptest` cases.
- Compile-fail doctests in `src/chip.rs:134-210`; their presence is verified,
  their execution is not.
- `cargo fmt --all -- --check`, root clippy, compiler tests, chip-lint tests,
  torture tests, and the AArch64 probe.
- Any compiler `/39` test (including `c10`–`c39`, `freeze.rs`, `h04_candidate`).
- Any end-to-end C compilation, source-to-executable run, GCC torture rate, or
  verified target ABI.
- Any full review of the uncommitted `compiler/src/chips/parse` user work; it is
  outside this audit's root-framework scope.

### 6.3 Open questions for the integrator

1. **Runtime authority:** Is T01 §4.1's `RestrictedChip` mandate still
   normative, or does the `/8+` `Worker`/`RoutingShell` template supersede it?
   The answer determines whether the compiler must migrate or T01 must be
   corrected.
2. **Root driver scope:** Is `Motherboard`/`Backend` intended only for generic
   examples and non-compiler applications, or is the compiler expected to use
   them later? There is currently no documented integration decision.
3. **Bus profile:** Should the root `Bus` trait remain a policy-only flat-bus
   contract, or should a new API/lint enforce fixed-layout data outside the
   approved compiler exception? If it stays policy-only, the exception must be
   more visible.
4. **Backend contract:** Is the SFL capability matrix a must-have before any
   non-CPU backend, and who owns the first differential test?
5. **Host boundary:** Should `Clock` move out of the root crate into a Host
   crate, or is it acceptable for the root framework to ship a wall-clock helper
   that chips can reach through re-exports?
6. **Compiler dynamic pins/wires:** Are `CompilerPins.host_responses: Vec<_>`
   and `CompilerWires.proposals: Vec<_>` within the accepted ADR-0001 dynamic
   area scope, or do they need a separate explicit storage decision?
7. **Documentation ownership:** Who is authorized to update T01 §7.1,
   ADR-0001 §4, README, and compiler README so the `/39` identity and
   partial-slice status are consistent?
8. **Lint scope:** Should CI's chip-lint cover only semantic `compute` bodies or
   also workers' `handle`/projection adapters? The current split leaves the
   trusted boundary outside the lint.

## 7. Bottom Line

The root framework is a small, coherent execution kernel: `Bus`, `LogicChip`,
`RestrictedChip`/`ProjectedChip`, `Motherboard`, `Backend`/`CpuBackend`,
`Clock`, and `simulate`/`Testbench` exist in source, and the six targeted tests
plus the counter example ran successfully. The source and README are unusually
candid that most isolation and determinism guarantees are policy/test-enforced
rather than type-enforced.

The material gap is not missing root types; it is **which boundary the compiler
actually uses**. The compiler application shares only `cc_silicon::Bus` and runs
its own `Worker`/`RoutingShell` lifecycle. It does not use the root
`Motherboard`, `RestrictedChip`, `ProjectedChip`, `Backend`, or `Clock`, despite
T01 §4.1 naming that path. Compiler read sets are declarative, the chip-lint does
not cover adapters/projectors, and two tick drivers have no differential test.
Documentation also drifts between `/6`, `/16`, `/9`, `/5`, and the actual `/39`,
which can mislead parallel workers.

Recommended first action: resolve the runtime-authority question (A01-FU-02)
after the documentation truth pass (A01-FU-01), then add the two-driver
differential test (A01-FU-03) and lint hardening (A01-FU-04). Do not treat this
audit's six targeted passes as a full framework or compiler acceptance.
