//! T07 semantic workers (Wave 2 slice 5: literal/binary/return checks).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip; the integrator owns this `mod.rs`.

mod se_binary;
mod se_literal;
mod se_return;

pub use self::se_binary::{project_se_binary_input, SeBinChip, SeBinaryInput};
pub use self::se_literal::{project_se_literal_input, SeLitChip, SeLiteralInput};
pub use self::se_return::{project_se_return_input, SeRetChip, SeReturnInput};
