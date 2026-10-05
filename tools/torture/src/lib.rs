#![forbid(unsafe_code)]
//! Offline corpus-lock and provenance verifier for the T00/H00 GCC torture gate.
//!
//! This library verifies an explicitly supplied local lock and asset tree. It
//! never downloads corpus assets, never invokes GCC/Clang, and never produces a
//! pass-rate claim. A successful exit proves only that the supplied tree matches
//! the supplied frozen lock; it does not prove H00 completion on its own.
//!
//! The corpus policy encoded here is fetch-on-demand with hash locking (assets
//! are not vendored), and the target identity is frozen as AArch64 GNU/Linux ELF
//! LP64 little-endian AAPCS64. The concrete runner/toolchain remains an explicit
//! unfilled input until H01 establishes it.

pub mod diagnostic;
pub mod digest;
pub mod lock;
pub mod path;
pub mod verify;

pub use diagnostic::{Diagnostic, Severity};
pub use lock::{
    is_filled, AssetFields, LockStatus, ParsedLock, Spanned, SCHEMA_V1, SUPPORTED_SCHEMAS,
};
pub use verify::{verify_lock_file, verify_lock_text, Report, VerifyOptions};

/// Tool version, surfaced by the CLI `--version` flag.
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
