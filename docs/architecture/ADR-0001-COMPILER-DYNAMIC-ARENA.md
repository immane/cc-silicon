# ADR-0001: Compiler Dynamic Arena Extension and Frozen Target Identity

Status: **Accepted** (user-approved T01 integration decision, 2026-10-04)

This ADR records an explicit, application-scoped extension. It does **not**
weaken or rewrite the framework specification. The root `cc-silicon` crate
remains a domain-free, `#![forbid(unsafe_code)]` framework; its fixed-array /
no-heap constraint applies to **bus data**, not to the whole crate (the scope
note closing §2 distinguishes bus storage, topology initialization, and the
tick hot path). `docs/architecture/SILICON_PARADIGM_SPEC.md` is unchanged.

## 1. Context

The framework bus is specified as a flat struct of primitives, fixed-size
arrays, and plain enums: no heap, no interior mutability, no threads. A
general-purpose C compiler must accept inputs of unknown length, so it cannot
be expressed with fixed arrays alone.

`docs/tasks/T01_COMPILER_CONTRACT.md` proposed a CPU storage extension. It was
a pending-approval proposal; dependent implementation was blocked until an
explicit decision. That decision has now been made and is recorded here.

## 2. Decision: application-scoped dynamic arenas

The compiler application (`compiler/`, crate `cc-silicon-compiler`) may use
append-only `Vec` arenas, intern tables, and work queues **inside its own
`CompilerBus`**, subject to all of the following:

1. **All semantic state still lives in the bus.** Arenas are bus storage, not a
   private backend cache. There is no separate mutable object graph.
2. **Stable newtype IDs only.** Records reference each other through typed ID
   newtypes, never through `Rc`/`Arc`/`RefCell`/`Mutex`, raw pointers, or heap
   addresses used as identities. An address is never a semantic ID.
3. **No ID reuse.** Arenas are append-only; deletion leaves a tombstone and an
   ID is never reallocated.
4. **Checked access.** Out-of-bounds, tombstoned, and sentinel IDs return
   structured `ArenaError` values. Panics are never semantic control flow.
5. **Configured bounds.** Input size, arena size, queue size, task depth, tick
   budget, and proposal count are configured explicitly; exhaustion produces a
   structured capacity diagnostic.
6. **Deterministic ordering.** Scheduling, ID allocation, diagnostics, and
   serialization use explicit ordering (insertion order, sorted keys, ordinal
   counters), never hash-map iteration order.
7. **The compiler is not heapless per tick and not directly
   hardware-synthesizable.** The framework's bus staying fixed-layout is not a
   claim about user chips. A bounded hardware realization must define capacity
   and exhaustion behavior separately and provide a differential; the CPU
   dynamic arena is not claimed to be portable across arbitrary HDL.
8. **Diagnostics, not panics.** Recoverable faults are typed diagnostics;
   unimplemented behavior fails explicitly and is never reported as success.

Scope: this extension applies to the compiler application crate only. The
framework's resource claims are scoped and must not be conflated:

- **Bus storage — fixed-layout, no heap.** Applications implement `Bus` as a
  flat struct of primitives, fixed-size arrays, and plain enums, with no
  interior mutability and no threads. This is the constraint stated in
  `docs/architecture/SILICON_PARADIGM_SPEC.md` §3.2.
- **Topology initialization — allocates.** `Motherboard` stores its pipeline as
  `Vec<Vec<Box<dyn LogicChip<B>>>>` plus a boxed `Backend`; `new`,
  `with_backend`, `push_layer`, and `install_chip` allocate. That memory is
  construction-time topology, not semantic bus state.
- **Tick hot path — no framework allocation.** After construction, layers are
  never resized during a tick, and `clock_tick` only resets wires, iterates the
  pre-built layers, and latches. This claim covers the framework's own driver;
  user chips and custom backends may still allocate.

The "fixed arrays / no heap" rule is therefore a bus-data constraint, not a
whole-framework resource guarantee. Examples (e.g. `examples/counter.rs`) keep
fixed-layout bus data but allocate while assembling their `Motherboard`.

## 3. Decision: frozen target identity and probe policy

Target identity is frozen:

- triple `aarch64-unknown-linux-gnu`
- object format ELF
- data model LP64
- little-endian
- ABI AAPCS64

Freezing identity does **not** verify concrete values. Until a probe runs:

- every scalar size/alignment, `long double` format, `wchar_t` signedness, and
  ABI register/save-area value is marked **UNVERIFIED**;
- macOS/Darwin arm64 values must not be used anywhere;
- the AAPCS64 variadic model uses a register save area, not the Darwin arm64
  stack-only model.

Probe/runner policy:

- the future probe and runner substrate is a Linux CI/VM; it is **planned and
  not yet provisioned**;
- the GCC corpus is fetched on demand with a hash lock;
- a reference-only DejaGnu baseline is **authorized** as an oracle, is **not
  yet available**, and is **never candidate compiler evidence**.

No runner, reference baseline, or probe result is claimed to exist today.

Verification is attestation-only. What is unforgeable is the *private verified
state representation*: the verified state can only be produced by validating a
canonical, hash-signed probe report containing every required field (including
`wchar_t.encoding`). `attest` validates caller-supplied report data and its
hash; it does not establish authenticity or physical provenance, and the report
file is read at a point in time (TOCTOU). Trust in the report source is an
integration responsibility.

Every configured resource bound (source bytes, records total, tasks total, task
depth, diagnostics, queue length, per-arena capacity, proposals per tick, ticks,
intern entries/bytes) is enforced with a structured failure before any mutation.
Commit binds every inner task ID to the enclosing task, validates Enqueue parent
references (including earlier predicted sibling IDs) and rejects dangling
parents, is field-scoped against each chip's registered write manifest, is
owner/task-kind attributed, rejects writes to the read-only `config` store, and
performs only infallible appends after a capacity preflight, so a failed batch
commits nothing and a commit failure never strands a task in `Running`.

## 4. Consequences

- `docs/tasks/T01_COMPILER_CONTRACT.md` section 1 status changes from
  pending to approved; the framework specification is not rewritten.
- The compiler application contract is compiled and frozen under
  `compiler/`, with version and content hash in
  `compiler/contracts/CONTRACT_VERSION` and `compiler/src/contract.rs`.
  The current frozen version is `t01-c01-c06/5`; the hash is a fingerprint of
  normative shapes and rule identifiers, not a source-code hash and not a
  semantic-equivalence proof.
- This ADR authorizes only the storage extension and the target identity. It
  does not authorize C language chips, does not claim any compiler exists, and
  does not establish any torture pass rate.

## 5. Revision record

| Date | Change | Authority |
|---|---|---|
| 2026-10-04 | CPU dynamic arena extension approved; target identity frozen; concrete values and probe remain unverified | User, T01 integration session |
| 2026-10-04 | Protocol repair: inner-task binding and preflight-atomic commit, dangling-parent rejection, precise probe-attestation wording, `wchar_t.encoding` required, full wire/reserved snapshot coverage, validated manifest registration, structurally-immutable config; contract bumped to /3 | User, T01 integration session |
| 2026-10-04 | Follow-up repair: single terminal/wait transition per task per batch (`AwaitHost` shares the guard), lossless `usize` proposal-budget comparison, propagation entry-path coverage for pre-populated malformed wires; contract bumped to /4 | User, T01 integration session |
| 2026-10-04 | Audit repair: `.gitignore` no longer hides `compiler/contracts/target/`; source content hashes computed internally; Enqueue destinations require a registered kind-accepting chip; accurate `ResultAlreadyConsumed` error; contract bumped to /5 | User, T01 integration session |
| 2026-10-05 | DOC-07 wording fix: scoped the framework resource claims to fixed-layout bus data, `Vec`/`Box` topology allocation at construction/installation, and no framework allocation on the default tick hot path, replacing the crate-wide "no-heap" description of the framework and examples | Documentation review DOC-07 (doc-only) |
