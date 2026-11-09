//! T07 semantic workers (Wave 2 slice 5: literal/binary/return checks;
//! Wave 3 slice 14: function-definition check; `/49`: identifier check;
//! `/52`: unary-expression check; `/53`: assignment check).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip; the integrator owns this `mod.rs`.

mod se_assignment;
mod se_binary;
mod se_function;
mod se_identifier;
mod se_literal;
mod se_return;
mod se_unary;

pub use self::se_assignment::{project_se_assignment_input, SeAssignmentChip, SeAssignmentInput};
pub use self::se_binary::{project_se_binary_input, SeBinChip, SeBinaryInput};
pub use self::se_function::{
    project_se_function_input, SeFuncChip, SeFunctionInput, SE_FUNC_TASK_KIND,
};
pub use self::se_identifier::{project_se_identifier_input, SeIdentifierChip, SeIdentifierInput};
pub use self::se_literal::{project_se_literal_input, SeLitChip, SeLiteralInput};
pub use self::se_return::{project_se_return_input, SeRetChip, SeReturnInput};
pub use self::se_unary::{project_se_unary_input, SeUnaryChip, SeUnaryInput};
