//! T04 lexical workers (Wave 2 slice 2: tokenize/classify/decode).
//!
//! Group directory per `docs/tasks/PARALLEL_EXECUTION.md` §2: one file per
//! chip (`<group>/<snake_name>.rs`); the integrator owns this `mod.rs`.

mod lx_classify;
mod lx_decode;
mod lx_intern;

pub use self::lx_classify::{
    is_keyword, project_lx_classify_input, LxClassifyChip, LxClassifyInput, C11_KEYWORDS,
};
pub use self::lx_decode::{
    decimal_magnitude, project_lx_decode_input, LxDecodeInput, LxDecodeLiteralChip,
};
pub use self::lx_intern::{project_lx_intern_input, LxInternChip, LxInternInput};
