//! T03 preprocessing workers (Wave 2 slice 1: source normalization).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod pp_builtin;
mod pp_comment;
mod pp_conditional;
mod pp_define;
mod pp_diagnostic;
mod pp_directive;
mod pp_emit;
mod pp_enter;
mod pp_expand_map;
mod pp_invoke;
mod pp_line;
mod pp_normalize;
mod pp_pragma;
mod pp_redefine;
mod pp_resolve;
mod pp_scan;
mod pp_splice;
mod pp_substitute;
mod pp_undef;
mod pp_variadic;

pub use self::pp_builtin::{project_pp_builtin_input, PpBuiltinChip, PpBuiltinInput};
pub use self::pp_comment::{
    project_pp_comment_input, replace_comments, PpCommentChip, PpCommentInput,
};
pub use self::pp_conditional::{
    project_pp_conditional_input, PpConditionalChip, PpConditionalInput,
};
pub use self::pp_define::{project_pp_define_input, PpDefineChip, PpDefineInput};
pub use self::pp_diagnostic::{project_pp_diagnostic_input, PpDiagnosticChip, PpDiagnosticInput};
pub use self::pp_directive::{project_pp_directive_input, PpDirectiveChip, PpDirectiveInput};
pub use self::pp_emit::{
    emit_preprocessed, project_pp_emit_input, EmittedPreprocessed, PpEmitChip, PpEmitInput,
    PP28_TASK_KIND,
};
pub use self::pp_enter::{project_pp_include_enter_input, PpIncludeEnterChip, PpIncludeEnterInput};
pub use self::pp_expand_map::{
    origin_chain, origin_root, project_pp_expand_map_input, OriginChain, OriginFrame,
    PpExpandMapChip, PpExpandMapInput, PP27_TASK_KIND,
};
pub use self::pp_invoke::{project_pp_invoke_input, PpInvokeChip, PpInvokeInput};
pub use self::pp_line::{
    line_location, project_pp_line_input, LogicalLocation, PpLineChip, PpLineInput, PP23_TASK_KIND,
    PP_LINE_MAX,
};
pub use self::pp_normalize::{normalize, project_pp_input, PpInput, PpNormalizeChip};
pub use self::pp_pragma::{
    classify_directive_params, classify_operator_text, decode_pragma_string,
    project_pp_pragma_input, PpPragmaChip, PpPragmaInput, PragmaClass, PragmaStringError,
    PP25_TASK_KIND,
};
pub use self::pp_redefine::{project_pp_redefine_input, PpRedefineChip, PpRedefineInput};
pub use self::pp_resolve::{
    project_pp_include_resolve_input, PpIncludeResolveChip, PpIncludeResolveInput,
};
pub use self::pp_scan::{
    project_pp_scan_input, scan, PpScanChip, PpScanInput, ScannedToken, M1_PUNCTUATORS,
};
pub use self::pp_splice::{
    compose_map, project_pp_splice_input, splice, PpSpliceChip, PpSpliceInput,
};
pub use self::pp_substitute::{project_pp_substitute_input, PpSubstituteChip, PpSubstituteInput};
pub use self::pp_undef::{project_pp_undef_input, PpUndefChip, PpUndefInput};
pub use self::pp_variadic::{
    project_pp_variadic_input, PpVariadicChip, PpVariadicInput, PpVariadicMode,
};
