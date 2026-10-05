//! T03 preprocessing workers (Wave 2 slice 1: source normalization).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod pp_normalize;

pub use self::pp_normalize::{normalize, project_pp_input, PpInput, PpNormalizeChip};
