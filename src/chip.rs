// ============================================================================
// chip.rs — LogicChip: a stateless transition unit
// ============================================================================

//! Logic chips are the combinational logic of the circuit.
//!
//! A chip is a zero-field unit struct that performs exactly one deduction per
//! clock tick. It reads the frozen [`crate::Bus::Pins`] and the current bus,
//! then writes wires and (where appropriate) registers.
//!
//! # Rules
//!
//! * **Stateless**: the implementing type carries no fields.
//! * **No chip-to-chip calls**: chips communicate only through the bus.
//! * **Deterministic**: identical `(pins, bus)` input yields identical output.
//! * **No privilege escalation**: a chip touches only the fields relevant to
//!   its single responsibility.

use crate::bus::Bus;

/// A stateless logic gate that performs one pure deduction per clock tick.
///
/// # Borrow-checker safety
///
/// `&self` (a zero-sized value) and `&mut B` are distinct allocations, so
/// there is no aliasing conflict. The motherboard executes chips sequentially,
/// which means exactly one `&mut B` exists at any moment. No `RefCell`, no
/// `Mutex`, no `unsafe`.
pub trait LogicChip<B: Bus> {
    /// Execute one clock cycle of logic for this chip.
    ///
    /// Reads the frozen pins and the current bus (including wires written by
    /// earlier chips this tick), computes new values, and writes them back to
    /// the bus. Returns `()`. Errors are modelled as "blown fuse" signals on
    /// the bus, never as panics or `Result` used for control flow.
    fn tick(&self, pins: &B::Pins, bus: &mut B);
}
