# cc-silicon

A reusable Rust framework for the **Silicon-Based Software Architecture Paradigm**.

Instead of object call chains, `cc-silicon` models software as a synchronous
digital circuit:

- **Input Pins** sample external signals once per tick
- A flat **System Bus** stores all state (registers + ephemeral wires)
- Stateless **Logic Chips** perform isolated deductions
- A **Motherboard Clock** drives deterministic ticks

This crate is deliberately domain-free. It provides the paradigm, not a product.
Applications define their own pins, wires, bus, and chips — then wire the chips
into a motherboard. A complete worked circuit lives in
[`examples/counter.rs`](examples/counter.rs).

---

## Why this framework exists

Traditional code is often organised around classes, mutable object graphs, and
implicit control flow. That style can scale, but it frequently introduces:

- hidden state transitions,
- hard-to-replay bugs,
- tight coupling across modules,
- a high context load for human and AI contributors.

`cc-silicon` explores a different approach: software architecture inspired by
FPGA/ASIC design principles. The result is a codebase that is highly
deterministic, easy to reason about, and friendly to AI-assisted development.

---

## Terminology

This project uses the following terms consistently:

| Term | Meaning |
|---|---|
| **Input Pins** | External inputs sampled and frozen for the current tick only |
| **System Bus** | The single flat state carrier: registers plus wires |
| **Registers** | State that persists across ticks |
| **Wires** | Per-tick ephemeral signals, reset at tick start |
| **Logic Chips** | Stateless transition units |
| **Motherboard** | Deterministic scheduler of chip layers |
| **Backend** | A realization of the semantic core on a substrate (CPU/GPU/HDL/…) |
| **Tick** | One full sample → propagate → latch cycle |

When discussing portability, **SFL (Silicon Formal Language)** is the semantic
source of truth, while Rust/C/CUDA/HDL implementations are backend realizations.
The formal multi-backend contract is documented in
[docs/architecture/SFL_CONTRACT.md](docs/architecture/SFL_CONTRACT.md).

---

## The paradigm in one screen

```text
External I/O ──▶ InputPins (frozen snapshot)
                   │
                   ▼
        ┌───────────────────────┐
        │     Motherboard       │
        │ layer[0..N] pipeline  │
        └───────────────────────┘
                   │
                   ▼
        SystemBus · Registers + Wires
                   │
                   ▼
          Backend (cpu / gpu / …)
```

Clock lifecycle per tick:

1. **Sampling** — the host polls I/O into `pins` (at the boundary, outside the core).
2. **Combinational propagation** — chips read pins/bus and write wires.
3. **Sequential latching** — the motherboard commits edge state for the next tick.

The first and third phases are explicit hook points ([`Bus`]); the middle phase
is the backend's job.

---

## The framework API

Everything lives behind a small set of traits and types.

### `Bus` — the single source of truth

```rust
pub trait Bus: 'static {
    type Pins: Clone + 'static;
    type Wires: Default + Clone + 'static;

    fn wires(&self) -> &Self::Wires;
    fn wires_mut(&mut self) -> &mut Self::Wires;

    // hooks with sensible defaults
    fn reset_wires(&mut self) { *self.wires_mut() = Self::Wires::default(); }
    fn latch(&mut self, _pins: &Self::Pins) {}
    fn tick_count(&self) -> u64 { 0 }
    fn advance_tick(&mut self) {}
}
```

An application implements `Bus` on one flat struct holding all registers plus an
embedded wire bundle.

### `LogicChip` — a stateless transition unit

```rust
pub trait LogicChip<B: Bus> {
    fn tick(&self, pins: &B::Pins, bus: &mut B);
}
```

Chips are zero-field unit structs. They never call each other; data flows only
through the bus. `tick` returns `()` — errors are modelled as "blown fuse"
signals on the bus, never as panics.

For applications that want stronger enforcement, prefer `RestrictedChip` with
`silicon_chip!` and `Motherboard::install_projected`. Its computation receives
only a read-only input projection and returns a typed proposal; a separate
`ChipAdapter` projects bus fields and commits the proposal. The macro only
declares unit structs and compile-fail doctests verify that fields, bus access,
and input mutation are rejected. Adapters remain trusted application code, and
Rust cannot prove absence of global I/O or nondeterminism; enforce those with
crate boundaries, static linting, and deterministic replay tests.

### `Motherboard` — the clock driver

```rust
let mut mb = Motherboard::<MyBus>::new(2);
mb.install(0, DecodeChip);
mb.install(1, MutateChip);
mb.clock_tick(&pins, &mut bus);
```

`clock_tick` runs the three phases in order and is the only entity allowed to
invoke a chip or to reset/latch the bus.

### `Backend` — the realization layer

```rust
pub trait Backend<B: Bus> {
    fn name(&self) -> &str;
    fn execute_layers(
        &mut self,
        layers: &[Vec<Box<dyn LogicChip<B>>>],
        pins: &B::Pins,
        bus: &mut B,
    ) { /* reference: run every chip in order */ }
}
```

`CpuBackend` is the deterministic scalar reference. Replace it with
`Motherboard::with_backend(...)` to batch, fuse, offload, or emulate work while
preserving observable meaning.

### `Clock` — wall-clock sampling

Host-boundary helper that measures elapsed nanoseconds between ticks and clamps
surprises such as a suspended process.

### `Testbench` / `simulate` — headless verification

Run deterministic pin sequences and assert properties of the resulting bus.

### Restricted chips and static checks

For stronger separation between computation and bus mutation, implement
`RestrictedChip` and install it with `Motherboard::install_projected`. The chip
receives only an immutable input projection and returns a typed proposal; a
separate `ChipAdapter` builds that projection and commits the proposal. The
`silicon_chip!` macro declares a unit-struct chip, and compile-fail doctests
cover stateful declarations, direct bus access, and input mutation.

The optional AST-based chip linter in `tools/chip-lint` checks a source
directory for common violations, including legacy `LogicChip` implementations,
stateful chip structs, opaque macros, unsafe blocks, and common host or
nondeterministic APIs. Run it with:

```bash
cargo test --manifest-path tools/chip-lint/Cargo.toml
cargo run --manifest-path tools/chip-lint/Cargo.toml -- <chip-source-directory>
```

The linter is a conservative aid, not a proof of purity: Rust aliases, method
dispatch, and effects hidden in called helpers or dependencies may require
review. Keep adapters and helper functions auditable, isolate host I/O, and use
deterministic replay tests. The legacy `LogicChip` API remains available for
compatibility; use `RestrictedChip` for the stricter path.

---

## Quick start

The full program is in [`examples/counter.rs`](examples/counter.rs); run it with:

```bash
cargo run --example counter
```

The essential shape is: define `Pins`, `Wires`, and a `Bus`; implement
`LogicChip` for each unit struct; assemble layers; tick.

```rust
use cc_silicon::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
struct Pins { pulse: bool }

#[derive(Clone, Debug, Default)]
struct Wires { edge: bool }

#[derive(Clone, Debug)]
struct MyBus { value: u32, wires: Wires, prev_pulse: bool, tick_count: u64 }

impl Bus for MyBus {
    type Pins = Pins;
    type Wires = Wires;

    fn wires(&self) -> &Wires { &self.wires }
    fn wires_mut(&mut self) -> &mut Wires { &mut self.wires }

    fn latch(&mut self, pins: &Pins) { self.prev_pulse = pins.pulse; }
    fn tick_count(&self) -> u64 { self.tick_count }
    fn advance_tick(&mut self) { self.tick_count = self.tick_count.wrapping_add(1); }
}

struct EdgeChip;
impl LogicChip<MyBus> for EdgeChip {
    fn tick(&self, pins: &Pins, bus: &mut MyBus) {
        bus.wires.edge = pins.pulse && !bus.prev_pulse;
    }
}

struct CountChip;
impl LogicChip<MyBus> for CountChip {
    fn tick(&self, _pins: &Pins, bus: &mut MyBus) {
        // Overflow is part of the chip's contract: wrap explicitly.
        if bus.wires.edge { bus.value = bus.value.wrapping_add(1); }
    }
}

fn main() {
    let mut mb = Motherboard::<MyBus>::new(2);
    mb.install(0, EdgeChip);
    mb.install(1, CountChip);

    let mut bus = MyBus { value: 0, wires: Wires::default(), prev_pulse: false, tick_count: 0 };
    for tick in 0..10u64 {
        let pins = Pins { pulse: tick % 2 == 0 };
        mb.clock_tick(&pins, &mut bus);
    }
    assert_eq!(bus.value, 5);
}
```

---

## Repository layout

```text
src/
  lib.rs            crate root and public re-exports
  bus.rs            Bus trait (System Bus: registers + wires)
  chip.rs           LogicChip trait
  motherboard.rs    Motherboard pipeline + clock_tick driver
  backend.rs        Backend trait + CpuBackend reference
  clock.rs          wall-clock sampling helper (host boundary)
  sim.rs            simulate() + Testbench for headless verification
  prelude.rs        convenient re-exports
examples/
  counter.rs        a complete minimal circuit
tests/
  paradigm.rs       framework-level tests on a neutral domain
docs/
  architecture/     paradigm spec + SFL contract/schema
  design/           blueprint + getting-started guide
tools/
  chip-lint/        AST-based checks for chip source
  torture/          offline corpus-lock integrity and provenance verifier
```

---

## Design rules

To preserve silicon semantics, applications built on this framework should obey
the physical purity rules:

- **No global mutable state** outside the bus passed during a tick.
- **No chip-to-chip calls** — communicate only through bus fields.
- **No implicit control flow** — model errors as blown-fuse wires, not panics.
- **No privilege escalation** — a chip touches only fields relevant to its duty.
- **Wires are per-tick** — never assume a wire survives a tick boundary.
- **Prefer testbenches and property tests** over anecdotal unit tests.

Many of these are enforced structurally by Rust: `&mut Bus` gives one exclusive
writer, `&Pins` is read-only, and zero-field unit structs cannot hide state.
`rustc` is therefore a *partial design-rule checker* — it cannot verify business
logic, which still requires tests and, where it matters, formal methods.

---

## Testing and verification

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --example counter
cargo test --manifest-path tools/chip-lint/Cargo.toml
```

The paradigm maps cleanly onto a Mealy/Moore machine:

```text
S(t+1) = F(S(t), I(t))
```

where `S` is the full bus snapshot and `F` is the fixed layer-ordered chip
pipeline. The functional reading is a **conditional property**, not something
the public API guarantees: [`LogicChip`](src/chip.rs) accepts arbitrary `tick`
implementations, and the default [`Backend::execute_layers`](src/backend.rs)
just calls them, so neither trait can enforce purity, termination, or absence
of panics. The equation holds for chips, bus hooks, adapters, and backends that
satisfy four conditions: **purity** (no hidden state, I/O, or chip-to-chip
calls), **determinism** for fixed inputs, **termination**, and an **explicit
error/overflow contract** (every fault becomes a bus value, never a panic).
Under those conditions deterministic replay, fuzzing, and property-based
testing are natural fits; replay tests exercise the property on chosen traces
and are evidence for it, not a proof that it holds for every input.

---

## Backend outlook

The CPU backend is complete and is the reference for all behavior. The SFL
contract defines the boundary for later realizations:

| Backend | Status | Notes |
|---|---|---|
| CPU scalar (reference) | Stable | This crate |
| SIMD / accelerated CPU | Future | Must remain semantically equivalent |
| GPU | Future | Batch/fuse chips; emulate or reject the rest |
| FPGA / ASIC (HLS) | Research | Fixed-width, synthesizable subsets only |
| Quantum-oriented | Speculative | Interface-level orchestration and reversible sub-circuits |

The rule is constant: backends realize meaning, they never redefine it. Any
behavior a backend cannot represent must be rejected or emulated explicitly —
silent semantic drift is forbidden.

---

## Documentation

| Document | Purpose |
|---|---|
| [docs/architecture/SILICON_PARADIGM_SPEC.md](docs/architecture/SILICON_PARADIGM_SPEC.md) | The paradigm, principles, and primitives |
| [docs/architecture/SFL_CONTRACT.md](docs/architecture/SFL_CONTRACT.md) | Multi-backend semantic contract |
| [docs/architecture/SFL_SCHEMA_DRAFT.md](docs/architecture/SFL_SCHEMA_DRAFT.md) | Structured document shape for tooling |
| [docs/design/ARCHITECTURAL_BLUEPRINT.md](docs/design/ARCHITECTURAL_BLUEPRINT.md) | How to build a system on cc-silicon |
| [docs/design/GETTING_STARTED.md](docs/design/GETTING_STARTED.md) | Step-by-step walkthrough |

---

## License

MIT — see [LICENSE](LICENSE).
