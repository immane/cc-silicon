//! T04 lexical workers (Wave 2 slice 2: tokenize/classify/decode).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod lx_char;
mod lx_classify;
mod lx_decode;
mod lx_escape;
mod lx_float_syntax;
mod lx_float_value;
mod lx_intern;
mod lx_string;

pub use self::lx_char::{
    decode_char_units, fold_char_value, project_lx_char_input, split_char_spelling, u32_magnitude,
    CharPrefix, LxCharChip, LxCharInput, LX12_TASK_KIND,
};
pub use self::lx_classify::{
    is_keyword, project_lx_classify_input, LxClassifyChip, LxClassifyInput, C11_KEYWORDS,
};
pub use self::lx_decode::{
    decimal_magnitude, project_lx_decode_input, LxDecodeInput, LxDecodeLiteralChip,
};
pub use self::lx_escape::{
    decode_escape_body, project_lx_escape_input, split_literal_body, CodeUnits, EscapeError,
    LxEscapeChip, LxEscapeInput, LX11_TASK_KIND,
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
pub use self::lx_string::{
    decode_string_body, element_for_prefix, encode_units_le, project_lx_string_input,
    split_string_prefix, LxStringChip, LxStringInput, StringElementType, StringError, StringPrefix,
    StringRecord, LX13_TASK_KIND,
};
