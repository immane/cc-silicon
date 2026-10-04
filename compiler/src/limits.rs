// ============================================================================
// limits.rs — configured resource limits (T01 C01)
//
// The CPU storage extension permits append-only Vec arenas, but every bound
// must be configured explicitly and exhaustion must surface as a structured
// diagnostic, never as panic-driven control flow. These limits are frozen as
// part of the T01 contract and are read-only after job initialization.
// ============================================================================

use crate::arena::ArenaError;
use crate::ids::TaskId;

/// Explicit resource budget for one compiler job.
///
/// All fields are hard upper bounds. A store that would exceed a bound returns
/// a structured capacity error instead of growing without limit. Callers may
/// choose any values, but a job must carry a [`Limits`] value; unbounded
/// operation is not part of the contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Maximum number of records a single arena may allocate (including
    /// tombstones, because IDs are never reused).
    pub max_records_per_arena: u32,
    /// Maximum total records across every arena in one job.
    pub max_records_total: u64,
    /// Maximum bytes of source imported for one job.
    pub max_source_bytes: u64,
    /// Maximum number of interned names.
    pub max_intern_entries: u32,
    /// Maximum total bytes held by the intern table.
    pub max_intern_bytes: u64,
    /// Maximum number of tasks that may be pending in the ready queue.
    pub max_queue_len: u32,
    /// Maximum number of tasks that may ever be allocated in one job.
    pub max_tasks_total: u64,
    /// Maximum continuation/child nesting depth.
    pub max_task_depth: u32,
    /// Maximum number of diagnostics retained for one job.
    pub max_diagnostics: u32,
    /// Maximum number of ticks before the job is reported as budget-exhausted.
    pub max_ticks: u64,
    /// Maximum number of proposals accepted in a single commit batch.
    pub max_proposals_per_tick: u32,
}

impl Limits {
    /// A small, deterministic budget suited to fixtures and tests.
    ///
    /// Deliberately tight so that capacity behavior is exercised by the test
    /// suite rather than only in extreme jobs.
    pub const fn fixture() -> Self {
        Self {
            max_records_per_arena: 1 << 16,
            max_records_total: 1 << 22,
            max_source_bytes: 1 << 20,
            max_intern_entries: 1 << 16,
            max_intern_bytes: 1 << 22,
            max_queue_len: 1 << 12,
            max_tasks_total: 1 << 20,
            max_task_depth: 1 << 10,
            max_diagnostics: 1 << 14,
            max_ticks: 1 << 20,
            max_proposals_per_tick: 1 << 12,
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::fixture()
    }
}

/// A structured resource-budget failure.
///
/// Every convention in [`Limits`] maps to a variant here so that append,
/// queue, source, diagnostic, depth, and total-record exhaustion all fail
/// before mutating the bus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LimitError {
    /// An arena rejected the record (per-arena capacity).
    Arena(ArenaError),
    /// Source bytes exceed the configured maximum.
    SourceBytes {
        /// Configured maximum.
        limit: u64,
        /// Requested total.
        requested: u64,
    },
    /// Total records across all arenas plus the patch log exceed the maximum.
    TotalRecords {
        /// Configured maximum.
        limit: u64,
        /// Requested total.
        requested: u64,
    },
    /// Task records ever allocated exceed the maximum.
    TasksTotal {
        /// Configured maximum.
        limit: u64,
        /// Requested total.
        requested: u64,
    },
    /// A task's parent depth would exceed the maximum.
    TaskDepth {
        /// Configured maximum.
        limit: u32,
        /// Requested depth.
        requested: u32,
    },
    /// Retained diagnostics exceed the maximum.
    Diagnostics {
        /// Configured maximum.
        limit: u32,
        /// Requested total.
        requested: u32,
    },
    /// The ready queue would exceed the maximum.
    Queue {
        /// Configured maximum.
        limit: u32,
        /// Requested length.
        requested: u32,
    },
    /// A parent reference points at no live task.
    ///
    /// This is a task-tree integrity failure, not a budget: the task graph is
    /// malformed and must be rejected rather than silently truncated.
    DanglingParent {
        /// The missing parent.
        parent: TaskId,
    },
}

impl std::fmt::Display for LimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arena(error) => write!(f, "arena limit: {error}"),
            Self::SourceBytes { limit, requested } => {
                write!(f, "source bytes {requested} exceed limit {limit}")
            }
            Self::TotalRecords { limit, requested } => {
                write!(f, "total records {requested} exceed limit {limit}")
            }
            Self::TasksTotal { limit, requested } => {
                write!(f, "task count {requested} exceeds limit {limit}")
            }
            Self::TaskDepth { limit, requested } => {
                write!(f, "task depth {requested} exceeds limit {limit}")
            }
            Self::Diagnostics { limit, requested } => {
                write!(f, "diagnostic count {requested} exceeds limit {limit}")
            }
            Self::Queue { limit, requested } => {
                write!(f, "ready queue length {requested} exceeds limit {limit}")
            }
            Self::DanglingParent { parent } => {
                write!(f, "task parent {} does not exist", parent.index())
            }
        }
    }
}

impl std::error::Error for LimitError {}

impl From<ArenaError> for LimitError {
    fn from(error: ArenaError) -> Self {
        Self::Arena(error)
    }
}
