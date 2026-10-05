//! T08 constant/layout/init workers (Wave 1: const-fold integer subset;
//! Wave 3 slice 12: selected-branch `&&`/`||`/`?:` + static assert).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod fold;
mod fold_branch;

pub use self::fold::{const_bits_required, project_fold_input, FoldChip, FoldInput};
pub use self::fold_branch::{
    eval_assert, eval_branch, project_assert_input, project_branch_input, AssertCond, AssertInput,
    BranchAndChip, BranchCondChip, BranchInput, BranchOp, BranchOperand, BranchOrChip, BranchValue,
    StaticAssertChip, CL04_AND_TASK_KIND, CL04_COND_TASK_KIND, CL04_OR_TASK_KIND,
    CL07_ASSERT_TASK_KIND,
};
