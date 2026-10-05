//! T09 IR lowering workers (Wave 2 slice 6: whole-function lowering).
//!
//! Group directory for the T09 owning group; one file per chip; the
//! integrator owns this `mod.rs`. (T09's deeper `ir/lower` split lands with
//! Wave 3+ chips.)

mod ir_function;

pub use self::ir_function::{project_ir_function_input, IrFunctionChip, IrFunctionInput};
