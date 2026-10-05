//! T05 parse workers (Wave 2 slice 3: translation unit; Wave 3 slice 9:
//! external-declaration dispatch, specifiers, declarator, block, return;
//! Wave 3 slice 10: primary, binary, unary expressions).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod pa_binary;
mod pa_block;
mod pa_declarator;
mod pa_external;
mod pa_specifier;
mod pa_tu;
mod pa_unary;

pub use self::pa_binary::{
    binary_precedence, parse_binary_expression, parse_primary, project_pa_binary_input,
    project_pa_primary_input, BinaryError, ExprNodeShape, PaBinaryChip, PaBinaryInput,
    PaBinaryProduction, PaPrimaryInput, ProjectedExprToken, PA16_TASK_KIND, PA22_TASK_KIND,
};
pub use self::pa_unary::{
    parse_unary, project_pa_unary_input, PaUnaryChip, PaUnaryInput, ProjectedUnaryToken,
    UnaryError, UnaryNodeShape, UnaryOp, PA20_TASK_KIND,
};

pub use self::pa_block::{
    parse_block, parse_return, project_pa_block_input, project_pa_return_input, LocalNodeShape,
    PaBlockChip, PaBlockInput, PaBlockProduction, PaReturnInput, ProjectedBlockToken,
    PA28_TASK_KIND, PA32_TASK_KIND,
};
pub use self::pa_declarator::ProjectedToken as PaDeclaratorProjectedToken;
pub use self::pa_declarator::{
    parse_declarator, parse_direct_declarator, parse_parameter_list, project_pa_declarator_input,
    DeclaratorError, DeclaratorToken, DeclaratorTree, PaDeclaratorChip, PaDeclaratorInput,
    ParamList, PA05_TASK_KIND,
};
pub use self::pa_external::{
    external_decl_kind, project_pa_external_input, ExternalDeclKind, ExternalToken, PaExternalChip,
    PaExternalInput, PA02_TASK_KIND,
};
pub use self::pa_specifier::{
    parse_specifier, project_pa_specifier_input, PaSpecifierChip, PaSpecifierInput,
    ProjectedSpecifierToken, SpecifierRecordShape, PA03_TASK_KIND,
};
pub use self::pa_tu::{project_pa_tu_input, PaTuChip, PaTuInput};
