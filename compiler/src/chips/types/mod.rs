//! T06 type workers (Wave 2 slice 4: canonical types + identity conversions).
//!
//! Group directory per `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md` (`chips/types/`);
//! one file per chip; the integrator owns this `mod.rs`.

mod ty_conv;
mod ty_types;

pub use self::ty_conv::{is_m1_int, project_ty_conv_input, TyConvChip, TyConvInput};
pub use self::ty_types::{canonical_scan, m1_int, project_ty_type_input, TyTypeChip, TyTypeInput};
