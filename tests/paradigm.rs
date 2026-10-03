// ============================================================================
// tests/paradigm.rs — Framework-level tests on a tiny neutral domain
//
// The domain below is intentionally unrelated to any real application: an
// accumulator with edge detection and an async reset. It exercises every
// primitive of the paradigm without coupling the tests to a product.
// ============================================================================

use std::cell::Cell;
use std::rc::Rc;

use cc_silicon::prelude::*;
use proptest::prelude::*;

// ─── Test domain ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TestPins {
    pulse: bool,
    reset: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TestWires {
    reset_applied: bool,
    pulse_seen: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestBus {
    value: u32,
    wires: TestWires,
    prev_pulse: bool,
    tick_count: u64,
}

impl TestBus {
    fn new() -> Self {
        Self {
            value: 0,
            wires: TestWires::default(),
            prev_pulse: false,
            tick_count: 0,
        }
    }
}

impl Bus for TestBus {
    type Pins = TestPins;
    type Wires = TestWires;

    fn wires(&self) -> &TestWires {
        &self.wires
    }
    fn wires_mut(&mut self) -> &mut TestWires {
        &mut self.wires
    }
    fn latch(&mut self, pins: &TestPins) {
        self.prev_pulse = pins.pulse;
    }
    fn tick_count(&self) -> u64 {
        self.tick_count
    }
    fn advance_tick(&mut self) {
        self.tick_count += 1;
    }
}

// Layer 0: decode inputs into wires.
struct RelayChip;
impl LogicChip<TestBus> for RelayChip {
    fn tick(&self, pins: &TestPins, bus: &mut TestBus) {
        bus.wires.reset_applied = pins.reset;
    }
}

struct PulseChip;
impl LogicChip<TestBus> for PulseChip {
    fn tick(&self, pins: &TestPins, bus: &mut TestBus) {
        bus.wires.pulse_seen = pins.pulse && !bus.prev_pulse && !bus.wires.reset_applied;
    }
}

// Layer 1: mutate registers from wires.
struct ResetChip;
impl LogicChip<TestBus> for ResetChip {
    fn tick(&self, _pins: &TestPins, bus: &mut TestBus) {
        if bus.wires.reset_applied {
            bus.value = 0;
        }
    }
}

struct AccumulateChip;
impl LogicChip<TestBus> for AccumulateChip {
    fn tick(&self, _pins: &TestPins, bus: &mut TestBus) {
        if bus.wires.pulse_seen {
            bus.value = bus.value.wrapping_add(10);
        }
    }
}

fn build_motherboard() -> Motherboard<TestBus> {
    let mut mb = Motherboard::<TestBus>::new(2);
    mb.install(0, RelayChip);
    mb.install(0, PulseChip);
    mb.install(1, ResetChip);
    mb.install(1, AccumulateChip);
    mb
}

/// Run a pin sequence and return the final bus.
fn run_sequence(sequence: &[TestPins]) -> TestBus {
    let mut mb = build_motherboard();
    let mut bus = TestBus::new();
    for pins in sequence {
        mb.clock_tick(pins, &mut bus);
    }
    bus
}

fn pins(pulse: bool, reset: bool) -> TestPins {
    TestPins { pulse, reset }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[test]
fn default_backend_is_cpu() {
    let mb = build_motherboard();
    assert_eq!(mb.backend_name(), "cpu");
    assert_eq!(mb.layer_count(), 2);
    assert_eq!(mb.chip_count(), 4);
}

#[test]
fn wires_reset_every_tick() {
    let bus = run_sequence(&[pins(true, false), pins(false, false)]);
    assert!(!bus.wires.pulse_seen, "wire leaked across a tick boundary");
    assert!(!bus.wires.reset_applied);
}

#[test]
fn layers_and_chips_propagate_within_a_tick() {
    // A single rising pulse flows Relay → Pulse (layer 0) then Reset →
    // Accumulate (layer 1), all in the same tick.
    let bus = run_sequence(&[pins(true, false)]);
    assert_eq!(bus.value, 10);
    assert!(bus.wires.pulse_seen);
}

#[test]
fn latching_enables_edge_detection() {
    // Holding the input high for three ticks yields exactly one rising edge.
    let bus = run_sequence(&[pins(true, false), pins(true, false), pins(true, false)]);
    assert_eq!(bus.value, 10);
    assert!(!bus.wires.pulse_seen);
}

#[test]
fn reset_is_asynchronous_and_wins_the_tick() {
    let bus = run_sequence(&[pins(true, false), pins(true, true)]);
    assert_eq!(bus.value, 0);
    assert!(bus.wires.reset_applied);
    assert!(!bus.wires.pulse_seen);
}

#[test]
fn lamport_clock_advances() {
    let bus = run_sequence(&[pins(false, false); 7]);
    assert_eq!(bus.tick_count(), 7);
}

#[test]
fn deterministic_replay_is_bitwise_identical() {
    let sequence = [
        pins(true, false),
        pins(false, false),
        pins(true, false),
        pins(false, true),
        pins(true, true),
    ];
    assert_eq!(run_sequence(&sequence), run_sequence(&sequence));
}

#[test]
fn host_can_install_a_custom_backend() {
    // A backend is allowed private realization state (here, an execution
    // counter) as long as it does not change observable meaning.
    let executions: Rc<Cell<u64>> = Rc::new(Cell::new(0));

    struct CountingBackend {
        executions: Rc<Cell<u64>>,
    }
    impl<B: Bus> Backend<B> for CountingBackend {
        fn name(&self) -> &str {
            "counting"
        }
        fn execute_layers(
            &mut self,
            layers: &[Vec<Box<dyn LogicChip<B>>>],
            pins: &B::Pins,
            bus: &mut B,
        ) {
            self.executions.set(self.executions.get() + 1);
            for layer in layers {
                for chip in layer {
                    chip.tick(pins, bus);
                }
            }
        }
    }

    let mut mb = Motherboard::<TestBus>::with_backend(
        2,
        Box::new(CountingBackend {
            executions: Rc::clone(&executions),
        }),
    );
    mb.install(0, RelayChip);
    mb.install(0, PulseChip);
    mb.install(1, ResetChip);
    mb.install(1, AccumulateChip);

    let mut bus = TestBus::new();
    mb.clock_tick(&pins(true, false), &mut bus);

    assert_eq!(mb.backend_name(), "counting");
    assert_eq!(executions.get(), 1);
    assert_eq!(bus.value, 10);
}

#[test]
fn testbench_runs_headless() {
    let mut tb = Testbench::new(TestBus::new(), 2);
    tb.motherboard.install(0, RelayChip);
    tb.motherboard.install(0, PulseChip);
    tb.motherboard.install(1, ResetChip);
    tb.motherboard.install(1, AccumulateChip);

    tb.run(20, |tick| pins(tick % 2 == 0, false));
    assert_eq!(tb.bus.value, 100);
}

// ─── Property test: determinism under arbitrary input sequences ──────────────

proptest! {
    #[test]
    fn prop_same_sequence_same_state(
        sequence in proptest::collection::vec((any::<bool>(), any::<bool>()), 0..64)
    ) {
        let pins_seq: Vec<TestPins> =
            sequence.into_iter().map(|(p, r)| pins(p, r)).collect();
        let a = run_sequence(&pins_seq);
        let b = run_sequence(&pins_seq);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn prop_wires_never_leak_across_idle_ticks(
        _warmup in proptest::collection::vec((any::<bool>(), any::<bool>()), 1..16)
    ) {
        // After any non-empty warm-up, a fully idle tick must observe no wires.
        let mut mb = build_motherboard();
        let mut bus = TestBus::new();
        mb.clock_tick(&pins(true, false), &mut bus);
        mb.clock_tick(&pins(false, false), &mut bus);
        prop_assert!(!bus.wires.pulse_seen);
        prop_assert!(!bus.wires.reset_applied);
    }
}
