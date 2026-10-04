//! T01 compiler application contract foundation.
//!
//! This crate owns the compiler *application* contract: the `CompilerBus`
//! storage profile, stable IDs, the task/completion protocol, the
//! target/config schema, the SFL manifest extension, the deterministic
//! snapshot/serializer, and the minimal routing shell (T01 items C01-C06).
//!
//! It does **not** implement C language chips and it is not a C compiler. The
//! root `cc-silicon` framework remains domain-free; this is the approved
//! application boundary described by
//! `docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md`.
//!
//! Frozen artifact version/hash: see [`contract`] and
//! `contracts/CONTRACT_VERSION`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

/// Append-only, checked arenas (C01).
pub mod arena;
/// The `CompilerBus` storage profile (C01/C03).
pub mod bus;
/// Deterministic serialization and SHA-256 (C05).
pub mod codec;
/// Staged patch commit and task/result protocol (C03/C06).
pub mod commit;
/// Frozen contract version and content hash (C05/C06).
pub mod contract;
/// Structured diagnostics and the error protocol (C01/C03).
pub mod diagnostic;
/// Stable newtype IDs (C01).
pub mod ids;
/// Deterministic string interning (C01).
pub mod intern;
/// Configured resource limits (C01).
pub mod limits;
/// SFL manifest extension and validator (C04).
pub mod manifest;
/// Minimal routing integration shell (C06).
pub mod routing;
/// Deterministic snapshot and trace (C05).
pub mod snapshot;
/// Target/dialect/configuration schema (C02).
pub mod target;
/// Task and completion protocol types (C03).
pub mod task;

/// Convenient re-exports.
pub mod prelude;

pub use arena::{ArenaError, ArenaId, ReservedArena, TypedArena};
pub use bus::{CompilerBus, CompilerPins, CompilerWires};
pub use contract::{compute_contract_hash, CONTRACT_HASH, CONTRACT_VERSION};
pub use ids::ChipId;
pub use limits::Limits;
pub use task::{Proposal, Task, TaskGroup, TaskKind, TaskState};
