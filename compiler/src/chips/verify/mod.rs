//! T13 verification workers (Wave 2 slice 5: VF06 typed invariant).
//!
//! Group directory per `docs/tasks/T13_VERIFICATION_CHIPS.md`
//! (`chips/verify/`); one file per chip; the integrator owns this `mod.rs`.

mod vf_invariant;

pub use self::vf_invariant::{project_vf06_input, Vf06Chip, Vf06Input};
