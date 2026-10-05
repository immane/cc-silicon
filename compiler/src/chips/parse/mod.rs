//! T05 parse workers (Wave 2 slice 3: translation unit).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod pa_tu;

pub use self::pa_tu::{project_pa_tu_input, PaTuChip, PaTuInput};
