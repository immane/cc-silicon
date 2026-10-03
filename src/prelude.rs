// ============================================================================
// prelude.rs — Convenient re-exports
// ============================================================================

//! The items most applications need. Import with:
//!
//! ```no_run
//! use cc_silicon::prelude::*;
//! ```

pub use crate::backend::{Backend, CpuBackend};
pub use crate::bus::Bus;
pub use crate::chip::LogicChip;
pub use crate::chip::{ChipAdapter, ProjectedChip, RestrictedChip};
pub use crate::clock::Clock;
pub use crate::motherboard::Motherboard;
pub use crate::sim::{simulate, Testbench};
