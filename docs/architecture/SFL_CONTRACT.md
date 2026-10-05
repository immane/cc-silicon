# SFL Contract for Multi-Backend Silicon Systems

This document defines the closest practical boundary for a project-wide Silicon
Formal Language (SFL) contract. Its goal is not to describe any single
implementation, but to define the smallest semantic core that can host future
Rust, C, CUDA, HDL, and quantum-oriented backends.

The guiding rule is simple:

- SFL defines meaning.
- Backends define realizations.
- Adapters define how awkward or partial realizations are isolated.

If a code path cannot be represented directly by a backend, it must be pushed into
an adapter layer or declared as backend-specific extension logic.

---

## 1. Scope and Purpose

SFL is the project-level contract for every executable artifact. It must be able
to describe:

- the runtime itself,
- input sampling,
- state mutation,
- rendering and presentation,
- backend selection and fallback,
- simulation and testing harnesses,
- hardware-oriented lowering targets,
- and future experimental execution models.

This contract deliberately allows odd or inconvenient mappings. The goal is not
to make every backend elegant; the goal is to keep them semantically comparable.

---

## 2. Contract Layers

### 2.1 Semantic Core

The semantic core is the part every backend must preserve exactly. It consists of:

- `Pins`: frozen external inputs for one tick,
- `Bus`: the full observable machine state for one tick,
- `Wires`: ephemeral per-tick signals, reset at tick start,
- `Chip`: a pure state transform with explicit read and write sets,
- `Motherboard`: ordered tick orchestration,
- `Tick`: one complete sample → propagate → latch cycle,
- `Verifier`: tests or tooling that check semantic equivalence.

`cc-silicon` names these `Bus::Pins`, `Bus`, `Bus::Wires`, `LogicChip`,
`Motherboard`, `clock_tick`, and `Testbench`. `LogicChip` is the base chip
interface; `RestrictedChip` is the stricter form, receiving only a read-only
input projection and returning a typed proposal that a `ChipAdapter` commits.
The semantic core must remain backend-agnostic: no CUDA, HDL, or quantum detail
may leak into the meaning of the core contract.

### 2.2 Backend Realization Layer

Each backend may implement the contract differently, but it must advertise:

- what it can execute directly,
- what it can emulate,
- what it must lower through adapters,
- what it cannot represent and must reject.

This layer is where backend-specific constraints are allowed. In cc-silicon it is
the [`Backend`] trait.

### 2.3 Adapter Layer

Adapters are the compatibility boundary between SFL and a backend. Adapters may:

- reshape data for a backend,
- batch multiple chips into one device kernel,
- emulate unsupported behavior on the host,
- split one SFL chip into multiple backend operations,
- or fuse several SFL chips into one backend operation.

Adapters may not change SFL meaning.

---

## 3. Canonical State Model

Every executable state must be representable as one of the following:

- `Pins`: immutable per tick,
- `Bus`: mutable per tick, persisted across ticks,
- `Wire`: ephemeral per tick, reset at tick start,
- `BackendState`: private realization state, if needed,
- `HostState`: UI, file I/O, command-line, and environment scaffolding.

### 3.1 Bus Rule

The bus is the canonical machine state for semantic comparison. If a value is
visible to application logic, rendering, replay, or verification, it must either
live in the bus or be derived from the bus.

### 3.2 Private Backend State Rule

A backend may keep private caches, device handles, contexts, compiled kernels, or
hardware-specific descriptors. These are allowed only if they do not change the
observable SFL meaning. Examples: a GPU context and buffers, HDL simulation
handles, host-side command queues, quantum circuit compilation caches.
Observable scheduling and ordering decisions are semantic state: they must live
in the bus, never in private backend storage.

### 3.3 No Hidden Semantic State

If hidden state changes meaning, it is not allowed. If hidden state only improves
performance or manages the device, it is allowed.

---

## 4. Chip Contract

Every chip in SFL must declare:

- `reads`: the bus and pins fields it consumes,
- `writes`: the bus and wire fields it may modify,
- `phase`: when in the tick it runs,
- `determinism`: whether output is fully deterministic for fixed inputs,
- `backend_class`: whether it is portable, batchable, emulable, or device-specific.

### 4.1 Semantic Chip Form

A chip is a pure state transition:

```text
C_i(I_t) : Bus_t → Bus_{t+1}
```

This is a semantic statement, not a literal API requirement. The implementation
may mutate a shared `&mut Bus`, but the observable meaning must match the same
input–output relation.

### 4.2 Chip Categories

All chips must fall into one of these categories:

- `portable`: directly representable on all target backends,
- `batchable`: can be grouped with neighbors for a backend kernel,
- `emulable`: can run on the host when the backend cannot represent it,
- `device_specific`: intentionally limited to one backend family,
- `experimental`: allowed only behind an explicit feature or config gate.

### 4.3 Chip Compatibility Rule

If a new chip cannot be expressed in the current backend class, the backend must
do one of the following:

1. emulate it on the host,
2. lower it through a backend adapter,
3. reject it with a clear unsupported-capability error,
4. or mark it experimental and keep it out of the default pipeline.

Silent semantic drift is forbidden.

---

## 5. Motherboard Contract

The motherboard is the only entity allowed to execute tick ordering. It must
preserve:

- strict layer order,
- wire reset at tick start,
- all chip execution within a tick,
- latching at tick end,
- deterministic replay for identical input streams.

### 5.1 Ordering Boundary

The motherboard defines the only legal ordering boundary for chips. Backend
implementations may fuse, batch, or emulate work internally, but the observable
tick order must remain equivalent to the SFL layer order.

### 5.2 Device Mapping Boundary

The motherboard may remain abstract while a backend decides how to realize a
layer:

- one chip per kernel,
- one layer per kernel,
- one tick per command buffer,
- one chip group per HDL module,
- or one reversible sub-circuit per quantum-adjacent unit.

The mapping is backend-specific; the semantics are not.

---

## 6. Backend Capability Contract

Every backend must publish a capability matrix. Suggested dimensions:

- supports pins sampling: yes/no,
- supports the bus as flat memory: yes/no,
- supports wire reset semantics: yes/no,
- supports per-chip execution: yes/no,
- supports layer batching: yes/no,
- supports host fallback: yes/no,
- supports deterministic replay: yes/no,
- supports cross-backend diff testing: yes/no,
- supports hardware synthesis: yes/no,
- supports reversible sub-graphs: yes/no.

### 6.1 Practical Backend Families

#### CPU Reference Backend

The reference backend is the source of truth for implementation convenience. It
should be the most complete, easiest to debug, and most faithful version.
`cc-silicon`'s `CpuBackend` is this backend.

#### CPU Optimization Backend

The optimized CPU backend may vectorize or simplify execution, but must remain
semantically equivalent to the reference.

#### GPU Backend

The GPU backend may batch chip execution, scan large arrays, or accelerate
well-structured kernels. It is not required to map every chip one-to-one. It must
either emulate unsupported chips or decline them explicitly.

#### HDL / FPGA / ASIC Backend

The HDL backend is the natural home for fixed-width, static, synthesizable
subsets. It may reject dynamic or non-synthesizable constructs, but it must state
those limits explicitly and preserve SFL semantics for the accepted subset.

#### Quantum-Oriented Backend

The quantum-oriented backend is currently an interface-level and reversible-subset
target. It may only support orchestration, reversible transforms,
amplitude-independent control flow, and explicitly bounded experimental modules.
It does not imply that the full runtime becomes quantum-native.

---

## 7. Boundary Classes

To keep the contract honest, code must be classified into one of the following
boundary classes.

### 7.1 Core Portable Boundary

Code in this class must be available everywhere the project runs. Examples: bus
definitions, tick semantics, deterministic chip logic, replay and property tests,
pure rendering from bus state.

### 7.2 Backend-Adaptation Boundary

Code in this class may vary by backend, but only through clearly documented
capabilities. Examples: GPU kernels, HDL modules, CPU vectorization, quantum
compiler passes.

### 7.3 Host Boundary

Code in this class is not part of the semantic contract. Examples: terminal I/O,
environment-variable selection, logging, panic hooks, startup/shutdown guards,
wall-clock sampling.

### 7.4 Experimental Boundary

Code in this class may be awkward, partial, or backend-specific, but it must be
opt-in and must not silently change default semantics.

---

## 8. Verification Contract

The SFL contract is incomplete unless it is testable. Required verification modes:

- property-based tests on tick invariants,
- deterministic replay tests,
- backend diff tests against the reference backend,
- unsupported-capability tests for backend boundaries,
- and, where applicable, synthesis checks.

### 8.1 Semantic Equality

For the accepted SFL subset, all backends must satisfy:

```text
∀ t ≥ 0 : State(backend, t) = State(SFL, t)
```

### 8.2 Partial Support Rule

If a backend only supports a subset, the unsupported part must be:

- rejected before execution, or
- emulated in a documented way.

It must never fail by silently producing different meaning.

---

## 9. Design Rule Summary

1. SFL defines all semantic meaning.
2. The bus is the canonical observable state.
3. Chips are pure transforms with explicit reads and writes.
4. The motherboard defines legal tick order.
5. Backends may realize the semantics differently.
6. Adapters are the formal place where awkward compatibility lives.
7. Unsupported behavior must be explicit, not accidental.

This is the boundary that makes a project broad enough to include Rust, CUDA, HDL,
and future quantum-oriented experiments, while still being strict enough to
preserve one shared meaning.

## 10. Schema Draft Entry Point

For a more execution-oriented representation of this contract, see
[SFL_SCHEMA_DRAFT.md](SFL_SCHEMA_DRAFT.md). The schema draft turns the semantic
rules above into a structured document shape that can be validated, lowered, and
diff-tested by tooling.
