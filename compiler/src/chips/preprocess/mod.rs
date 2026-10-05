//! T03 preprocessing workers (Wave 2 slice 1: source normalization).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod pp_comment;
mod pp_normalize;
mod pp_scan;
mod pp_splice;

pub use self::pp_comment::{
    project_pp_comment_input, replace_comments, PpCommentChip, PpCommentInput,
};
pub use self::pp_normalize::{normalize, project_pp_input, PpInput, PpNormalizeChip};
pub use self::pp_scan::{project_pp_scan_input, scan, PpScanChip, PpScanInput, M1_PUNCTUATORS};
pub use self::pp_splice::{
    compose_map, project_pp_splice_input, splice, PpSpliceChip, PpSpliceInput,
};
