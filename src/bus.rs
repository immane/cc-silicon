// ============================================================================
// bus.rs — PCB traces: the System Bus (Registers + Wires)
// ============================================================================

//! The System Bus — the single flat source of truth for one application.
//!
//! A silicon application implements [`Bus`] on one flat struct that contains
//! every persistent **register** plus one embedded **wire** bundle.
//!
//! * Registers persist across ticks.
//! * Wires are ephemeral: they are pulled to ground (`Default`) at the start
//!   of every tick and are valid only for the duration of that tick.
//!
//! Associated types keep the framework generic. The concrete `InputPins` and
//! `Wires` types are defined by the application, exactly the way a PCB
//! designer chooses which traces to run.

/// The central machine state of a silicon application.
///
/// # Contract
///
/// * All persistent state lives here. There is no other global state.
/// * [`Bus::Wires`] is reset to [`Default`] once per tick by the motherboard.
/// * [`Bus::latch`] commits falling-edge state at the end of a tick.
/// * The bus must be a flat struct: primitives, fixed-size arrays, and plain
///   enums. No interior mutability, no threads.
pub trait Bus: 'static {
    /// External inputs sampled and frozen for exactly one tick.
    type Pins: Clone + 'static;

    /// Ephemeral per-tick signals that connect upstream and downstream chips.
    type Wires: Default + Clone + 'static;

    /// Read-only view of the current tick's wire bundle.
    fn wires(&self) -> &Self::Wires;

    /// Mutable view of the current tick's wire bundle.
    fn wires_mut(&mut self) -> &mut Self::Wires;

    /// Phase 0 of the clock cycle: pull every wire to ground.
    ///
    /// Called by the motherboard *before* any chip runs. The default
    /// implementation assigns [`Default`] to the whole bundle in one shot,
    /// mirroring pull-down resistors between clock edges.
    fn reset_wires(&mut self) {
        *self.wires_mut() = Self::Wires::default();
    }

    /// Phase 2 of the clock cycle: commit falling-edge state for the next tick.
    ///
    /// This is where edge-detection latches (e.g. "previous key state") and
    /// terminal phase transitions are committed, after every chip has run.
    /// The default implementation is a no-op.
    fn latch(&mut self, _pins: &Self::Pins) {}

    /// Monotonic tick counter (a Lamport clock).
    ///
    /// Used for deterministic replay, tracing, and debugging. The default is
    /// a constant `0` for applications that do not track ticks.
    fn tick_count(&self) -> u64 {
        0
    }

    /// Phase 2 hook: advance the Lamport clock by one tick.
    ///
    /// Called once per [`crate::Motherboard::clock_tick`], after latching.
    fn advance_tick(&mut self) {}
}
