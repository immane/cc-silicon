# Architectural Blueprint: Building a System on cc-silicon

**Paradigm:** Silicon-Based Software Architecture
**Audience:** Application authors, AI coding agents, and reviewers
**References:** [SILICON_PARADIGM_SPEC.md](../architecture/SILICON_PARADIGM_SPEC.md), [SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md)

This blueprint describes how to build a silicon application on top of the
`cc-silicon` framework. The framework supplies the primitives; the application
supplies the domain types and the pipeline topology.

---

## 1. Design the Bus First

The bus is the machine. Design it before writing any chip.

### 1.1 Separate registers from wires

```rust
#[derive(Clone, Debug, Default)]
struct MyWires {
    // ephemeral: valid only for the current tick
    request: bool,
    result: u32,
}

#[derive(Clone, Debug)]
struct MyBus {
    // registers: persist across ticks
    accumulator: u32,
    phase: Phase,
    tick_count: u64,

    // per-tick signal bundle, reset in one assignment
    wires: MyWires,
}
```

### 1.2 Implement the `Bus` trait

```rust
impl Bus for MyBus {
    type Pins = MyPins;      // frozen external inputs
    type Wires = MyWires;    // ephemeral signals

    fn wires(&self) -> &MyWires { &self.wires }
    fn wires_mut(&mut self) -> &mut MyWires { &mut self.wires }

    // Optional hook: commit edge/phase state for the next tick.
    fn latch(&mut self, pins: &MyPins) {
        self.prev_request = pins.request;
    }

    // Optional hook: advance the Lamport clock.
    fn tick_count(&self) -> u64 { self.tick_count }
    fn advance_tick(&mut self) { self.tick_count = self.tick_count.wrapping_add(1); }
}
```

### 1.3 Field rules

- Every field is a primitive, a fixed-size array, a plain enum, or `Option` of a
  flat type. No `Vec`, `HashMap`, `Box`, `Rc`, `Arc`, or `Mutex` in registers.
- Every register has a documented reset policy and an invariant.
- Naming encodes electrical meaning: `*_requested`, `*_seen`, `*_expired`,
  `*_ready`, `*_triggered`.
- Keep the `Wires` bundle separate and `Default`-able so it can be reset in one
  statement.

---

## 2. Write One Chip Per Responsibility

A chip is a zero-field unit struct implementing `LogicChip`.

```rust
struct DecodeChip;

impl LogicChip<MyBus> for DecodeChip {
    fn tick(&self, pins: &MyPins, bus: &mut MyBus) {
        bus.wires.request = pins.request && !bus.prev_request;
    }
}
```

### 2.1 Chip contract

| Property | Requirement |
|---|---|
| State | Zero fields (unit struct) |
| Inputs | `&MyPins`, `&mut MyBus` only |
| Outputs | Bus registers and wires only |
| Return | `()` — never `Result` for control flow |
| Coupling | Never calls another chip |
| Determinism | Same `(pins, bus)` → same output |
| Scope | Touches only fields relevant to its duty |

### 2.2 Prompt template for AI agents

```text
Implement <ChipName> as a zero-field unit struct implementing LogicChip<MyBus>.
Rules:
- Read only: <listed pins and bus fields>
- Write only: <listed wire/register outputs>
- Do not call other chips
- Preserve determinism
- Respect phase semantics: wires are per-tick
- Model errors as blown-fuse wires, never panic
```

---

## 3. Assemble the Motherboard

Layers are the physical topology. Order matters: a chip may read wires written by
any earlier chip in the same tick.

```rust
let mut mb = Motherboard::<MyBus>::new(4);
mb.install(0, DecodeChip);
mb.install(1, ComputeChip);
mb.install(2, CommitChip);
mb.install(3, ProjectChip);
```

Recommended generic layer roles:

| Layer | Role | Typical chips |
|---|---|---|
| 0 | Input decoding | Map pins to request wires |
| 1 | Pure computation | Timers, detectors, pure resolution |
| 2 | State mutation | Commit registers, advance phases |
| 3 | Output projection | Derive display/telemetry values |

### 3.1 Ordering contracts

- **Timers before consumers:** any chip that sets a trigger wire must precede the
  chips that read it.
- **Detect before mutate:** a detector should observe the current state before a
  mutation chip changes it.
- **Commit last within its concern:** mutation chips that read detection wires
  belong downstream of their detectors.
- **Document order in comments.** Ordering is a load-bearing part of the design.

---

## 4. Respect the Clock Lifecycle

`Motherboard::clock_tick` runs exactly three phases:

```text
clock_tick(pins, bus):
    bus.reset_wires()                          # Phase 0
    backend.execute_layers(layers, pins, bus)  # Phase 1
    bus.latch(pins); bus.advance_tick()        # Phase 2
```

- Phase 0 pulls every wire to ground. Never rely on a wire surviving a tick.
- Phase 1 is the only place chips run. No chip may re-enter the motherboard.
- Phase 2 commits edge/phase state. Registers mutated in Phase 1 become the
  "previous" values for the next tick.

---

## 5. Choose a Backend

`CpuBackend` is the reference and the default. To experiment with another
realization:

```rust
let mut mb = Motherboard::<MyBus>::with_backend(2, Box::new(MyBackend::new()));
```

A backend may batch, fuse, offload, or emulate chips, but the observable tick
order and bus meaning must be identical. A backend may hold private realization
state (device contexts, caches) only if it never changes meaning. Unsupported
chips must be emulated or explicitly rejected — never silently ignored.

---

## 6. Verify with Testbenches

Testing is simulation. Prefer property tests and replay over anecdotes.

```rust
let mut tb = Testbench::new(MyBus::new(), 4);
tb.motherboard.install(0, DecodeChip);
// ...
tb.run(1000, |tick| MyPins::random_like(tick));
```

Suggested invariant categories:

- **Bounds:** coordinates/indices never escape their valid range.
- **Conservation:** totals change only through documented mutations.
- **Monotonicity:** counters that must never decrease do not.
- **Determinism:** identical pin sequences and initial bus produce identical
  states.
- **Phase correctness:** wires are never observed across a tick boundary.

---

## 7. Compliance Checklist

| Constraint | How it is satisfied |
|---|---|
| No OOP call chains | Chips are unit structs; all state is flat in the bus |
| No events/callbacks | One `tick()` per chip, sequential iteration |
| No async/await | Synchronous `tick`; sampling is a boundary concern |
| No multi-threading | Single thread; one `&mut Bus` at a time |
| State/logic separation | All state in the bus; chips have zero fields |
| Tick-driven | Everything happens inside `clock_tick` |
| Flat state | Bus is one struct of primitives and fixed arrays |
| No unsafe | `#![forbid(unsafe_code)]`, no `unsafe` in the framework |
| No `RefCell`/`Mutex` | Exclusive `&mut Bus`; no interior mutability |
| Wires vs registers | Separate bundles; reset enforced at tick start |
| Lamport clock | `tick_count` advances every tick |
| Blown fuse signals | Errors are bus wires, not panics |
| Backend boundary | `Backend` trait; reference CPU implementation |

---

## 8. File Structure

```text
src/
├── main.rs                 # Host: sampling, I/O, rendering, main loop
├── bus.rs                  # Domain pins, wires, and Bus implementation
├── chips/
│   ├── mod.rs              # Chip declarations/re-exports
│   ├── decode.rs           # [Layer 0]
│   ├── compute.rs          # [Layer 1]
│   ├── commit.rs           # [Layer 2]
│   └── project.rs          # [Layer 3]
└── constants.rs            # Domain constants and lookup tables
```

Keep the tree flat and predictable. A reader (human or agent) should be able to
find the bus, the chips, and the topology at a glance.
