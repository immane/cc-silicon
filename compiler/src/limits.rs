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

/// Number of pipeline stages bounding [`Limits::stage_queue_bound`].
///
/// This mirrors the `bus::Stage` variant order (`Init = 0` through `Done =
/// 10`): entry `i` of `stage_queue_bound` bounds the canonical queue of the
/// stage with that ordinal. It is fixed here (rather than derived from
/// `bus::Stage`) so [`Limits`] stays `Copy` without `bus` gaining a
/// stage-count API; any future stage-ordinal re-freeze must update this
/// constant together with `bus::Stage`.
pub const STAGE_COUNT: usize = 11;

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
    /// Maximum number of tasks dispatched in a single tick.
    ///
    /// The sole per-tick dispatch-count bound (the redundant
    /// `max_dispatches_per_tick` is not part of this contract). Defaults to
    /// 1, the baseline equivalence mode; quota `> 1` is a measured,
    /// post-freeze change.
    pub max_inflight_per_tick: u32,
    /// Maximum ready-queue length per pipeline stage, indexed by stage
    /// ordinal (see [`STAGE_COUNT`]).
    ///
    /// Each entry must be at least 1. `tasks.ready` is a derived quota-1
    /// compatibility view over these canonical per-stage queues, never a
    /// second write target.
    pub stage_queue_bound: [u32; STAGE_COUNT],
    /// Maximum bit width of a constant value.
    ///
    /// Defaults to 128 and must satisfy `1 <= max_const_bits <= 128`;
    /// anything larger is rejected because the value carrier is `i128`.
    pub max_const_bits: u32,
    /// Maximum number of `Progress` reschedules for one task.
    ///
    /// Zero disables `Progress` (no reschedule is ever allowed); any other
    /// value bounds `task.progress_count + 1`.
    pub max_task_progress: u32,
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
            max_inflight_per_tick: 1,
            stage_queue_bound: [1 << 12; STAGE_COUNT],
            max_const_bits: 128,
            max_task_progress: 1 << 10,
        }
    }

    /// Fallible constructor validating the configured bounds.
    ///
    /// Legacy `/5` bounds accept any value (including 0, preserved for
    /// compatibility); the newer scheduling/constant bounds are strict: the
    /// per-tick dispatch quota and every per-stage queue bound must be at
    /// least 1, and `max_const_bits` must satisfy `1 <= max_const_bits <=
    /// 128`. `max_task_progress` accepts any value (0 disables `Progress`).
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        max_records_per_arena: u32,
        max_records_total: u64,
        max_source_bytes: u64,
        max_intern_entries: u32,
        max_intern_bytes: u64,
        max_queue_len: u32,
        max_tasks_total: u64,
        max_task_depth: u32,
        max_diagnostics: u32,
        max_ticks: u64,
        max_proposals_per_tick: u32,
        max_inflight_per_tick: u32,
        stage_queue_bound: [u32; STAGE_COUNT],
        max_const_bits: u32,
        max_task_progress: u32,
    ) -> Result<Self, LimitError> {
        let limits = Self {
            max_records_per_arena,
            max_records_total,
            max_source_bytes,
            max_intern_entries,
            max_intern_bytes,
            max_queue_len,
            max_tasks_total,
            max_task_depth,
            max_diagnostics,
            max_ticks,
            max_proposals_per_tick,
            max_inflight_per_tick,
            stage_queue_bound,
            max_const_bits,
            max_task_progress,
        };
        limits.validate()?;
        Ok(limits)
    }

    /// Structural validation of the configured bounds (see [`Limits::try_new`]).
    pub fn validate(&self) -> Result<(), LimitError> {
        if self.max_inflight_per_tick == 0 {
            return Err(LimitError::InvalidInflightQuota {
                value: self.max_inflight_per_tick,
            });
        }
        for (stage, bound) in self.stage_queue_bound.iter().enumerate() {
            if *bound == 0 {
                return Err(LimitError::InvalidStageQueueBound {
                    stage,
                    value: *bound,
                });
            }
        }
        if self.max_const_bits == 0 || self.max_const_bits > 128 {
            return Err(LimitError::InvalidConstBits {
                value: self.max_const_bits,
            });
        }
        Ok(())
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
    /// The per-tick dispatch quota is zero; at least one dispatch per tick
    /// is required.
    ///
    /// A limit-configuration violation (rejected by [`Limits::try_new`]),
    /// never a runtime dispatch failure.
    InvalidInflightQuota {
        /// The rejected value.
        value: u32,
    },
    /// A per-stage queue bound is zero.
    ///
    /// A limit-configuration violation (rejected by [`Limits::try_new`]),
    /// never a runtime backpressure failure (backpressure is a `CommitError`
    /// owned by the commit path, not a `LimitError`).
    InvalidStageQueueBound {
        /// Stage ordinal (see [`STAGE_COUNT`]).
        stage: usize,
        /// The rejected value.
        value: u32,
    },
    /// `max_const_bits` is zero or exceeds 128.
    ///
    /// A limit-configuration violation (rejected by [`Limits::try_new`]);
    /// the value carrier is `i128`, so widths above 128 are unrepresentable.
    InvalidConstBits {
        /// The rejected value.
        value: u32,
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
            Self::InvalidInflightQuota { value } => {
                write!(
                    f,
                    "per-tick dispatch quota {value} is invalid (must be at least 1)"
                )
            }
            Self::InvalidStageQueueBound { stage, value } => {
                write!(
                    f,
                    "stage {stage} queue bound {value} is invalid (must be at least 1)"
                )
            }
            Self::InvalidConstBits { value } => {
                write!(
                    f,
                    "max_const_bits {value} is invalid (must satisfy 1 <= bits <= 128)"
                )
            }
        }
    }
}

impl LimitError {
    /// Stable machine-readable code for this failure.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Arena(_) => "limits.arena",
            Self::SourceBytes { .. } => "limits.source_bytes",
            Self::TotalRecords { .. } => "limits.total_records",
            Self::TasksTotal { .. } => "limits.tasks_total",
            Self::TaskDepth { .. } => "limits.task_depth",
            Self::Diagnostics { .. } => "limits.diagnostics",
            Self::Queue { .. } => "limits.queue",
            Self::DanglingParent { .. } => "limits.dangling_parent",
            Self::InvalidInflightQuota { .. } => "limits.invalid_inflight_quota",
            Self::InvalidStageQueueBound { .. } => "limits.invalid_stage_queue_bound",
            Self::InvalidConstBits { .. } => "limits.invalid_const_bits",
        }
    }
}

impl std::error::Error for LimitError {}

impl From<ArenaError> for LimitError {
    fn from(error: ArenaError) -> Self {
        Self::Arena(error)
    }
}
