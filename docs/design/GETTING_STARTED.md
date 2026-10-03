# Getting Started with cc-silicon

This guide builds a complete, minimal silicon circuit from scratch: a 4-bit
wrapping counter with rising-edge detection and an asynchronous reset. The full
source is [`examples/counter.rs`](../../examples/counter.rs); run it with:

```bash
cargo run --example counter
```

By the end you will have touched every primitive of the paradigm: pins, wires,
bus, chips, layers, and the motherboard clock.

---

## 0. Add the dependency

```toml
[dependencies]
cc-silicon = { path = "../cc-silicon" }
```

```rust
use cc_silicon::prelude::*;
```

`prelude` re-exports `Bus`, `LogicChip`, `Motherboard`, `Backend`, `CpuBackend`,
`Clock`, `Testbench`, and `simulate`.

---

## 1. Define the frozen inputs (`Pins`)

The pins are the physical world for one tick. They are sampled by the host and
never mutated by chips.

```rust
#[derive(Clone, Debug, Default)]
struct CounterPins {
    increment: bool, // level-sensitive
    reset: bool,     // asynchronous
}
```

Keep this a plain struct. Every field is an input signal.

---

## 2. Define the ephemeral signals (`Wires`)

Wires are the traces that connect chips within a single tick. They are reset to
`Default` at the start of every tick.

```rust
#[derive(Clone, Debug, Default)]
struct CounterWires {
    reset_requested: bool,
    rising_edge: bool,
    overflow: bool,
}
```

Every default must be the "ground" state (`false`, `0`).

---

## 3. Define the persistent state (`Bus`)

The bus holds all registers plus the embedded wire bundle.

```rust
use cc_silicon::prelude::*;

const MODULUS: u32 = 16;

#[derive(Clone, Debug)]
struct CounterBus {
    value: u32,
    wires: CounterWires,
    prev_increment: bool, // edge-detection latch
    tick_count: u64,      // Lamport clock
}

impl CounterBus {
    fn new() -> Self {
        Self {
            value: 0,
            wires: CounterWires::default(),
            prev_increment: false,
            tick_count: 0,
        }
    }
}
```

Implement the `Bus` trait. This is where you declare the pins/wires types and
optionally hook the latching phase.

```rust
impl Bus for CounterBus {
    type Pins = CounterPins;
    type Wires = CounterWires;

    fn wires(&self) -> &CounterWires { &self.wires }
    fn wires_mut(&mut self) -> &mut CounterWires { &mut self.wires }

    // Phase 2: remember this tick's input level for the next edge detect.
    fn latch(&mut self, pins: &CounterPins) {
        self.prev_increment = pins.increment;
    }

    fn tick_count(&self) -> u64 { self.tick_count }
    fn advance_tick(&mut self) { self.tick_count = self.tick_count.wrapping_add(1); }
}
```

You can ignore `tick_count`/`advance_tick` if you do not need replay tracing; the
trait provides no-op defaults.

---

## 4. Write the chips

Each chip is a zero-field unit struct. Here we write four, arranged in two layers.

### Layer 0 — input decoding

```rust
struct ResetRelayChip;
impl LogicChip<CounterBus> for ResetRelayChip {
    fn tick(&self, pins: &CounterPins, bus: &mut CounterBus) {
        bus.wires.reset_requested = pins.reset;
    }
}

struct EdgeDetectChip;
impl LogicChip<CounterBus> for EdgeDetectChip {
    fn tick(&self, pins: &CounterPins, bus: &mut CounterBus) {
        bus.wires.rising_edge =
            pins.increment && !bus.prev_increment && !bus.wires.reset_requested;
    }
}
```

`EdgeDetectChip` reads `bus.wires.reset_requested`, which `ResetRelayChip` wrote
*earlier in the same layer, in the same tick*. That is combinational propagation:
no function call, only a shared trace.

### Layer 1 — state mutation

```rust
struct IncrementChip;
impl LogicChip<CounterBus> for IncrementChip {
    fn tick(&self, _pins: &CounterPins, bus: &mut CounterBus) {
        if bus.wires.reset_requested {
            bus.value = 0;
            bus.wires.overflow = false;
            return;
        }
        if bus.wires.rising_edge {
            let next = bus.value + 1;
            bus.wires.overflow = next == MODULUS;
            bus.value = next % MODULUS;
        }
    }
}
```

This chip reads wires produced by layer 0 and writes a register. It never calls a
peer chip.

---

## 5. Assemble and run

```rust
fn main() {
    let mut motherboard = Motherboard::<CounterBus>::new(2);
    motherboard.install(0, ResetRelayChip);
    motherboard.install(0, EdgeDetectChip);
    motherboard.install(1, IncrementChip);

    let mut bus = CounterBus::new();

    for tick in 0..40u64 {
        let pins = CounterPins {
            increment: tick % 3 != 2,
            reset: tick == 20,
        };
        motherboard.clock_tick(&pins, &mut bus);
        println!("{} -> value={} overflow={}", tick, bus.value, bus.wires.overflow);
    }
}
```

`clock_tick` performs, in order:

1. `bus.reset_wires()` — all wires returned to ground;
2. the backend runs layer 0 then layer 1;
3. `bus.latch(pins)` and `bus.advance_tick()` commit the falling edge.

---

## 6. Verify headlessly

Swap the printing loop for a `Testbench` and assert properties:

```rust
let mut tb = Testbench::new(CounterBus::new(), 2);
tb.motherboard.install(0, ResetRelayChip);
tb.motherboard.install(0, EdgeDetectChip);
tb.motherboard.install(1, IncrementChip);

tb.run(40, |tick| CounterPins { increment: tick % 3 != 2, reset: tick == 20 });
assert!(tb.bus.value < MODULUS);
```

Determinism is a first-class property: the same pin sequence and the same initial
bus always produce the same final bus. Add property tests for your own invariants
(bounds, conservation, monotonicity) as shown in
[`tests/paradigm.rs`](../../tests/paradigm.rs).

---

## 7. Where to go next

- Read the [architectural blueprint](ARCHITECTURAL_BLUEPRINT.md) for layer roles,
  chip contracts, and the compliance checklist.
- Read the [SFL contract](../architecture/SFL_CONTRACT.md) before adding a second
  backend.
- When a chip grows a second responsibility, split it. One chip, one deduction.
- When behavior needs new data flow, add a wire to the bus — do not add a chip-to-
  chip call.
