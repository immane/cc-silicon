// ============================================================================
// cc-silicon — a reusable framework for the Silicon-Based Software
// Architecture paradigm.
//
// Instead of object call chains, a program is modelled as a synchronous
// digital circuit:
//
//   * `InputPins`  — external signals, sampled and frozen for one tick
//   * `Bus`        — the single flat source of truth (registers + wires)
//   * `LogicChip`  — a stateless transition unit
//   * `Motherboard`— the deterministic clock driver and layer pipeline
//
// This crate contains **no domain logic**. Applications define their own
// pin/wire/bus types and their own chips, then wire those chips into a
// motherboard. See `docs/` and `examples/counter.rs`.
// ============================================================================

#![forbid(unsafe_code)]

pub mod backend;
pub mod bus;
pub mod chip;
pub mod clock;
pub mod motherboard;
pub mod prelude;
pub mod sim;

pub use backend::{Backend, CpuBackend};
pub use bus::Bus;
pub use chip::LogicChip;
pub use clock::Clock;
pub use motherboard::Motherboard;
pub use sim::{simulate, Testbench};
