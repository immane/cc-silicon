//! T03 preprocessing workers (Wave 2 slice 1: source normalization).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod pp_comment;
mod pp_conditional;
mod pp_define;
mod pp_diagnostic;
mod pp_directive;
mod pp_normalize;
mod pp_redefine;
mod pp_scan;
mod pp_splice;
mod pp_undef;

pub use self::pp_comment::{
    project_pp_comment_input, replace_comments, PpCommentChip, PpCommentInput,
};
pub use self::pp_conditional::{
    project_pp_conditional_input, PpConditionalChip, PpConditionalInput,
};
pub use self::pp_define::{project_pp_define_input, PpDefineChip, PpDefineInput};
pub use self::pp_diagnostic::{project_pp_diagnostic_input, PpDiagnosticChip, PpDiagnosticInput};
pub use self::pp_directive::{project_pp_directive_input, PpDirectiveChip, PpDirectiveInput};
pub use self::pp_normalize::{normalize, project_pp_input, PpInput, PpNormalizeChip};
pub use self::pp_redefine::{project_pp_redefine_input, PpRedefineChip, PpRedefineInput};
pub use self::pp_scan::{
    project_pp_scan_input, scan, PpScanChip, PpScanInput, ScannedToken, M1_PUNCTUATORS,
};
pub use self::pp_splice::{
    compose_map, project_pp_splice_input, splice, PpSpliceChip, PpSpliceInput,
};
pub use self::pp_undef::{project_pp_undef_input, PpUndefChip, PpUndefInput};
