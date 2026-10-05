//! T08 constant/layout/init workers (Wave 1: const-fold integer subset).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod fold;

pub use self::fold::{const_bits_required, project_fold_input, FoldChip, FoldInput};
