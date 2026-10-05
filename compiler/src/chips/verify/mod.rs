//! T13 verification workers (Wave 2 slice 5: VF06 typed invariant;
//! Wave 2 slice 8: VF12 symbolic interpret).
//!
//! Group directory per `docs/tasks/T13_VERIFICATION_CHIPS.md`
//! (`chips/verify/`); one file per chip; the integrator owns this `mod.rs`.

mod vf_interpret;
mod vf_invariant;

pub use self::vf_interpret::{project_vf12_input, Vf12Chip, Vf12Input};
pub use self::vf_invariant::{project_vf06_input, Vf06Chip, Vf06Input};
