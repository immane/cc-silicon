//! T05 parse workers (Wave 2 slice 3: translation unit; Wave 3 slice 9:
//! external-declaration dispatch, specifiers, declarator, block, return).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod pa_block;
mod pa_declarator;
mod pa_external;
mod pa_specifier;
mod pa_tu;

pub use self::pa_block::{
    project_pa_block_input, project_pa_return_input, parse_block, parse_return, LocalNodeShape,
    PaBlockChip, PaBlockInput, PaBlockProduction, PaReturnInput, ProjectedBlockToken,
    PA28_TASK_KIND, PA32_TASK_KIND,
};
pub use self::pa_declarator::{
    project_pa_declarator_input, parse_declarator, parse_direct_declarator, parse_parameter_list,
    DeclaratorError, DeclaratorToken, DeclaratorTree, PaDeclaratorChip, PaDeclaratorInput,
    ParamList, PA05_TASK_KIND,
};
pub use self::pa_declarator::ProjectedToken as PaDeclaratorProjectedToken;
pub use self::pa_external::{
    project_pa_external_input, external_decl_kind, ExternalDeclKind, ExternalToken,
    PaExternalChip, PaExternalInput, PA02_TASK_KIND,
};
pub use self::pa_specifier::{
    project_pa_specifier_input, parse_specifier, PaSpecifierChip, PaSpecifierInput,
    ProjectedSpecifierToken, SpecifierRecordShape, PA03_TASK_KIND,
};
pub use self::pa_tu::{project_pa_tu_input, PaTuChip, PaTuInput};
