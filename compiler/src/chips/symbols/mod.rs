//! T06 symbol/scope workers (Wave 2 slice 4: scopes, symbols, lookup).
//!
//! Group directory per `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md`
//! (`chips/symbols/`); one file per chip; the integrator owns this `mod.rs`.

mod ty_scope;
mod ty_symbol;

pub use self::ty_scope::{
    project_ty_scope_enter_input, project_ty_scope_exit_input, TyScopeChip, TyScopeEnterInput,
    TyScopeExitInput,
};
pub use self::ty_symbol::{
    in_ordinary_namespace, project_ty_declare_input, project_ty_lookup_input, TySymbolChip,
};
