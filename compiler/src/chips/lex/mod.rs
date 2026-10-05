//! T04 lexical workers (Wave 2 slice 2: tokenize/classify/decode).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod lx_classify;
mod lx_decode;
mod lx_float_syntax;
mod lx_float_value;
mod lx_intern;

pub use self::lx_classify::{
    is_keyword, project_lx_classify_input, LxClassifyChip, LxClassifyInput, C11_KEYWORDS,
};
pub use self::lx_decode::{
    decimal_magnitude, project_lx_decode_input, LxDecodeInput, LxDecodeLiteralChip,
};
pub use self::lx_float_syntax::{
    parse_float_parts, project_lx_float_syntax_input, FloatExponent as LxFloatSyntaxExponent,
    FloatParts as LxFloatSyntaxParts, FloatSuffix as LxFloatSyntaxSuffix, FloatSyntaxError,
    LxFloatSyntaxChip, LxFloatSyntaxInput, LX09_TASK_KIND,
};
pub use self::lx_float_value::{
    convert_float_parts, project_lx_float_value_input, spelling_to_value_parts,
    FloatConvertOutcome, FloatParts as LxFloatValueParts, FloatSuffix as LxFloatValueSuffix,
    FloatValueError, LxFloatValueChip, LxFloatValueInput, LX10_TASK_KIND,
};
pub use self::lx_intern::{project_lx_intern_input, LxInternChip, LxInternInput};
