//! T13 verification workers (Wave 2 slice 5: VF06 typed invariant;
//! Wave 2 slice 8: VF12 symbolic interpret; Wave 2 slice 9: VF05 token-AST
//! invariant).
//!
//! Group directory per `docs/tasks/T13_VERIFICATION_CHIPS.md`
//! (`chips/verify/`); one file per chip; the integrator owns this `mod.rs`.

mod vf_interpret;
mod vf_invariant;
mod vf_syntax;

pub use self::vf_interpret::{project_vf12_input, Vf12Chip, Vf12Input};
pub use self::vf_invariant::{project_vf06_input, Vf06Chip, Vf06Input};
pub use self::vf_syntax::{project_vf05_input, Vf05Chip, Vf05Input};
