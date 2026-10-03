// ============================================================================
// examples/counter.rs — a minimal silicon circuit built on cc-silicon
//
// A 4-bit wrapping counter with a rising-edge detector and a reset input.
// This is deliberately tiny: it exists to show every primitive of the
// paradigm (pins, wires, bus, chips, layered motherboard) without any
// application-domain complexity.
//
// Run with:  cargo run --example counter
// ============================================================================

use cc_silicon::prelude::*;

// ─── Domain constants ───────────────────────────────────────────────────────
const MODULUS: u32 = 16;

// ─── InputPins: frozen external inputs for one tick ─────────────────────────
#[derive(Clone, Debug, Default)]
struct CounterPins {
    /// Level-sensitive: a rising edge increments the counter.
    increment: bool,
    /// Asynchronous reset request.
    reset: bool,
}

// ─── Wires: ephemeral per-tick signals ──────────────────────────────────────
#[derive(Clone, Debug, Default)]
struct CounterWires {
    reset_requested: bool,
    rising_edge: bool,
    overflow: bool,
}

// ─── SystemBus: registers + embedded wires ──────────────────────────────────
#[derive(Clone, Debug)]
struct CounterBus {
    value: u32,
    wires: CounterWires,
    /// Register: previous tick's `increment` level (edge-detection latch).
    prev_increment: bool,
    tick_count: u64,
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

impl Bus for CounterBus {
    type Pins = CounterPins;
    type Wires = CounterWires;

    fn wires(&self) -> &CounterWires {
        &self.wires
    }

    fn wires_mut(&mut self) -> &mut CounterWires {
        &mut self.wires
    }

    // Phase 2 latch: remember the input level for the next tick's edge detect.
    fn latch(&mut self, pins: &CounterPins) {
        self.prev_increment = pins.increment;
    }

    fn tick_count(&self) -> u64 {
        self.tick_count
    }

    fn advance_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }
}

// ─── Layer 0: input decoding ────────────────────────────────────────────────

/// Relay the asynchronous reset pin onto a wire.
struct ResetRelayChip;

impl LogicChip<CounterBus> for ResetRelayChip {
    fn tick(&self, pins: &CounterPins, bus: &mut CounterBus) {
        bus.wires.reset_requested = pins.reset;
    }
}

/// Detect a rising edge, suppressed while a reset is requested.
///
/// This chip runs *after* `ResetRelayChip` in the same layer, so it observes
/// the wire that chip just wrote — intra-layer, same-tick propagation.
struct EdgeDetectChip;

impl LogicChip<CounterBus> for EdgeDetectChip {
    fn tick(&self, pins: &CounterPins, bus: &mut CounterBus) {
        bus.wires.rising_edge = pins.increment && !bus.prev_increment && !bus.wires.reset_requested;
    }
}

// ─── Layer 1: state mutation ────────────────────────────────────────────────

/// Commit the next counter value for this tick.
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

// ─── Main: assemble the circuit and run it ──────────────────────────────────
fn main() {
    let mut motherboard = Motherboard::<CounterBus>::new(2);
    motherboard.install(0, ResetRelayChip);
    motherboard.install(0, EdgeDetectChip);
    motherboard.install(1, IncrementChip);

    let mut bus = CounterBus::new();

    println!("backend       : {}", motherboard.backend_name());
    println!("chip count    : {}", motherboard.chip_count());
    println!("layers        : {}", motherboard.layer_count());
    println!();
    println!("tick  reset  incr  value  edge  overflow");
    println!("----  -----  ----  -----  ----  --------");

    for tick in 0..40u64 {
        let pins = CounterPins {
            // Toggle the input to create rising edges; reset at tick 20.
            increment: tick % 3 != 2,
            reset: tick == 20,
        };

        motherboard.clock_tick(&pins, &mut bus);

        println!(
            "{:>4}  {:>5}  {:>4}  {:>5}  {:>4}  {:>8}",
            tick, pins.reset, pins.increment, bus.value, bus.wires.rising_edge, bus.wires.overflow
        );
    }

    println!();
    println!("final value   : {}", bus.value);
    println!("tick_count    : {}", bus.tick_count());
}
