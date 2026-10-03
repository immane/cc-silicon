// ============================================================================
// sim.rs — Headless simulation and testbench helpers
// ============================================================================

//! The paradigm is verified by simulation: feed a deterministic pin sequence
//! into a motherboard and assert properties of the resulting bus. These
//! helpers make that easy for any application without prescribing a domain.

use crate::bus::Bus;
use crate::motherboard::Motherboard;

/// Drive `motherboard` for `ticks` cycles, sampling pins from `pins_at(tick)`.
pub fn simulate<B, F>(motherboard: &mut Motherboard<B>, bus: &mut B, ticks: u64, mut pins_at: F)
where
    B: Bus,
    F: FnMut(u64) -> B::Pins,
{
    for tick in 0..ticks {
        let pins = pins_at(tick);
        motherboard.clock_tick(&pins, bus);
    }
}

/// A bus plus a motherboard, ready to run headless.
///
/// Useful for deterministic replay, fuzzing, regression seeds, and
/// property-based testbenches.
pub struct Testbench<B: Bus> {
    pub motherboard: Motherboard<B>,
    pub bus: B,
}

impl<B: Bus> Testbench<B> {
    /// Wrap a bus and create a motherboard with `layer_count` empty layers.
    pub fn new(bus: B, layer_count: usize) -> Self {
        Self {
            motherboard: Motherboard::new(layer_count),
            bus,
        }
    }

    /// Wrap a bus and an already-configured motherboard.
    pub fn with_motherboard(bus: B, motherboard: Motherboard<B>) -> Self {
        Self { motherboard, bus }
    }

    /// Run a single clock cycle.
    pub fn tick(&mut self, pins: &B::Pins) {
        self.motherboard.clock_tick(pins, &mut self.bus);
    }

    /// Run `ticks` cycles, sampling pins from `pins_at(tick)`.
    pub fn run<F>(&mut self, ticks: u64, pins_at: F)
    where
        F: FnMut(u64) -> B::Pins,
    {
        simulate(&mut self.motherboard, &mut self.bus, ticks, pins_at);
    }

    /// Label of the active backend.
    pub fn backend_name(&self) -> &str {
        self.motherboard.backend_name()
    }
}
