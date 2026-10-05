// ============================================================================
// commit.rs — staged, field-scoped patch commit (T01 C03/C06, CT06 protocol)
//
// The commit path is the only way a worker changes shared state. A batch of
// proposals is:
//
//   1. ordered deterministically by (enqueue ordinal, TaskId, index),
//   2. validated in full: every inner task ID is bound to the enclosing task,
//      the task exists and is `Running`, the producing chip owns it, store
//      patches match the registered per-chip write manifest, the `config`
//      store stays read-only, versions are fresh, patch shapes are valid,
//      Enqueue parent references resolve (including earlier predicted sibling
//      IDs) and every resource bound holds,
//   3. applied with infallible appends only.
//
// On validation failure nothing is committed (failure atomicity). The apply
// pass performs no fallible operation after its first mutation: capacity was
// reserved by the preflight, so it uses infallible pushes. New tasks become
// eligible one tick after the commit. Completion and failure are exactly-once
// because a task must be `Running` to complete and the commit transitions it
// to a terminal state.
//
// Record references inside a patch are *not* existence-checked: the mechanical
// commit records validated intent, while materialization into typed group
// stores is owned by each group's commit integration. References into the
// reserved language stores are therefore opaque here.
// ============================================================================

use std::collections::{BTreeMap, BTreeSet};

use crate::arena::ArenaError;
use crate::bus::CompilerBus;
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft, DiagnosticRecord};
use crate::ids::{ChipId, DiagnosticId, RecordRef, ResultId, TaskId};
use crate::limits::LimitError;
use crate::task::{
    HostRequestRecord, PatchOp, Proposal, ResultRecord, ResultValue, StoreId, StorePatch, TaskKind,
    TaskState,
};

/// Per-store revision counters.
///
/// The array length is derived from the store set so adding a store cannot
/// silently leave a version slot missing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoreVersions {
    versions: [u64; StoreId::COUNT],
}

impl StoreVersions {
    /// Create zeroed versions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current version of a store.
    pub fn get(&self, store: StoreId) -> u64 {
        self.versions[store.index()]
    }

    /// Bump a store version by one.
    pub fn bump(&mut self, store: StoreId) {
        let slot = &mut self.versions[store.index()];
        *slot = slot.wrapping_add(1);
    }
}

/// A validated, committed store patch recorded in the patch log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedPatch {
    /// Commit ordinal within the job.
    pub ordinal: u64,
    /// Producing task.
    pub task: TaskId,
    /// Producing chip.
    pub owner: ChipId,
    /// Target store.
    pub store: StoreId,
    /// Declared field path.
    pub field: &'static str,
    /// Operation.
    pub op: PatchOp,
    /// Existing record (replace/tombstone).
    pub target: Option<RecordRef>,
    /// New record (append/replace).
    pub value: Option<RecordRef>,
}

/// Structured commit failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitError {
    /// The referenced task does not exist.
    UnknownTask {
        /// Offending task.
        task: TaskId,
    },
    /// The referenced result does not exist.
    UnknownResult {
        /// Offending result.
        result: ResultId,
    },
    /// The result has already been consumed once.
    ResultAlreadyConsumed {
        /// Offending result.
        result: ResultId,
    },
    /// The task is not in the running state.
    TaskNotRunning {
        /// Offending task.
        task: TaskId,
    },
    /// The task receives more than one terminal/wait transition in one batch
    /// (`Complete`, `Fail`, or `AwaitHost`).
    DuplicateCompletion {
        /// Offending task.
        task: TaskId,
    },
    /// A Complete/Fail/AwaitHost proposal names a task other than its
    /// enclosing task.
    InnerTaskMismatch {
        /// Task named inside the proposal.
        proposal_task: TaskId,
        /// Enclosing (tagged) task.
        task: TaskId,
    },
    /// An Enqueue names a parent that is neither an existing task nor an
    /// earlier predicted task in this batch.
    DanglingParent {
        /// Missing parent.
        parent: TaskId,
    },
    /// The producing chip is not the task's owner.
    ChipNotOwner {
        /// Producing chip.
        chip: ChipId,
        /// Task owner.
        owner: ChipId,
    },
    /// The producing chip has no registered manifest.
    UnregisteredChip {
        /// Offending chip.
        chip: ChipId,
    },
    /// The chip's manifest does not accept the task kind.
    TaskKindNotAccepted {
        /// Offending chip.
        chip: ChipId,
        /// Offending kind.
        kind: TaskKind,
    },
    /// A patch is attributed to a task other than its enclosing task.
    TaskAttributionMismatch {
        /// Task recorded on the patch.
        patch_task: TaskId,
        /// Enclosing task.
        task: TaskId,
    },
    /// A patch owner differs from the producing chip.
    PatchOwnerMismatch {
        /// Owner recorded on the patch.
        patch_owner: ChipId,
        /// Producing chip.
        chip: ChipId,
    },
    /// A patch writes a field not declared in the chip's write manifest.
    WriteNotDeclared {
        /// Offending chip.
        chip: ChipId,
        /// Store label.
        store: &'static str,
        /// Field path.
        field: &'static str,
    },
    /// A patch targets the immutable `config` store.
    ReadOnlyStore {
        /// Store label.
        store: &'static str,
    },
    /// A patch targets a store/field that is not declared at all.
    UndeclaredStoreField {
        /// Store label.
        store: &'static str,
        /// Field path.
        field: &'static str,
    },
    /// A patch was computed against a stale store version.
    StaleVersion {
        /// Store label.
        store: &'static str,
        /// Patch version.
        version: u64,
        /// Current version.
        current: u64,
    },
    /// A patch's operation shape is invalid for its target/value.
    InvalidPatchShape {
        /// Offending task.
        task: TaskId,
        /// Human-readable reason.
        reason: &'static str,
    },
    /// The tick's proposal count exceeds the configured bound.
    TooManyProposals {
        /// Configured bound.
        limit: u32,
        /// Observed count (kept lossless).
        count: usize,
    },
    /// A configured resource bound would be exceeded; nothing committed.
    Limit(LimitError),
    /// An arena is at capacity; nothing was committed.
    Capacity(ArenaError),
}

impl std::fmt::Display for CommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTask { task } => write!(f, "unknown task {}", task.index()),
            Self::UnknownResult { result } => write!(f, "unknown result {}", result.index()),
            Self::ResultAlreadyConsumed { result } => {
                write!(f, "result {} was already consumed", result.index())
            }
            Self::TaskNotRunning { task } => write!(f, "task {} is not running", task.index()),
            Self::DuplicateCompletion { task } => {
                write!(f, "task {} completes twice in one batch", task.index())
            }
            Self::InnerTaskMismatch {
                proposal_task,
                task,
            } => write!(
                f,
                "proposal names task {} but encloses {}",
                proposal_task.index(),
                task.index()
            ),
            Self::DanglingParent { parent } => {
                write!(f, "parent task {} does not exist", parent.index())
            }
            Self::ChipNotOwner { chip, owner } => write!(
                f,
                "chip {} is not the owner {} of the task",
                chip.index(),
                owner.index()
            ),
            Self::UnregisteredChip { chip } => {
                write!(f, "chip {} has no registered manifest", chip.index())
            }
            Self::TaskKindNotAccepted { chip, kind } => write!(
                f,
                "chip {} does not accept task kind {}",
                chip.index(),
                kind.raw()
            ),
            Self::TaskAttributionMismatch { patch_task, task } => write!(
                f,
                "patch attributed to task {} but enclosing task is {}",
                patch_task.index(),
                task.index()
            ),
            Self::PatchOwnerMismatch { patch_owner, chip } => write!(
                f,
                "patch owner {} differs from producing chip {}",
                patch_owner.index(),
                chip.index()
            ),
            Self::WriteNotDeclared { chip, store, field } => write!(
                f,
                "chip {} did not declare write `{store}.{field}`",
                chip.index()
            ),
            Self::ReadOnlyStore { store } => write!(f, "store `{store}` is read-only"),
            Self::UndeclaredStoreField { store, field } => {
                write!(f, "undeclared store field `{store}.{field}`")
            }
            Self::StaleVersion {
                store,
                version,
                current,
            } => write!(
                f,
                "stale patch for store `{store}` (version {version}, current {current})"
            ),
            Self::InvalidPatchShape { task, reason } => {
                write!(f, "task {} patch shape invalid: {reason}", task.index())
            }
            Self::TooManyProposals { limit, count } => {
                write!(f, "proposal count {count} exceeds limit {limit}")
            }
            Self::Limit(error) => write!(f, "limit: {error}"),
            Self::Capacity(error) => write!(f, "capacity: {error}"),
        }
    }
}

impl std::error::Error for CommitError {}

impl CommitError {
    /// Map to a structured diagnostic.
    pub fn to_diagnostic(&self) -> DiagnosticDraft {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Protocol, 1),
            self.to_string(),
        )
    }
}

impl From<ArenaError> for CommitError {
    fn from(error: ArenaError) -> Self {
        Self::Capacity(error)
    }
}

impl From<LimitError> for CommitError {
    fn from(error: LimitError) -> Self {
        Self::Limit(error)
    }
}

/// What a commit batch did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommitReport {
    /// Newly enqueued tasks, in commit order.
    pub enqueued: Vec<TaskId>,
    /// Completed tasks and their result IDs.
    pub completed: Vec<(TaskId, ResultId)>,
    /// Failed tasks and their diagnostic IDs.
    pub failed: Vec<(TaskId, DiagnosticId)>,
    /// Tasks moved to waiting.
    pub waiting: Vec<TaskId>,
    /// Number of store patches committed.
    pub patches: u32,
}

/// Fail if a proposal count exceeds the configured per-tick bound.
///
/// The comparison is done in `usize` so a batch larger than `u32::MAX` cannot
/// wrap around and be accepted. The observed count is reported losslessly.
pub fn check_proposal_budget(count: usize, limit: u32) -> Result<(), CommitError> {
    if count > limit as usize {
        Err(CommitError::TooManyProposals { limit, count })
    } else {
        Ok(())
    }
}

/// Commit a batch of proposals atomically.
///
/// `proposals` is consumed. On success the report lists every applied change;
/// on error no change was applied.
pub fn commit_proposals(
    bus: &mut CompilerBus,
    proposals: Vec<crate::bus::TaggedProposal>,
) -> Result<CommitReport, CommitError> {
    let limits = bus.limits();
    check_proposal_budget(proposals.len(), limits.max_proposals_per_tick)?;

    // Deterministic order: (enqueue ordinal, TaskId, batch index).
    let mut ordered: Vec<(u64, u32, usize)> = Vec::with_capacity(proposals.len());
    for (index, tagged) in proposals.iter().enumerate() {
        let task = bus
            .arenas
            .tasks
            .get(tagged.task)
            .map_err(|_| CommitError::UnknownTask { task: tagged.task })?;
        ordered.push((task.enqueue_ordinal, tagged.task.index(), index));
    }
    ordered.sort_unstable();

    // ---- Validation pass -------------------------------------------------
    // Predicted task IDs for Enqueue drafts, in apply order. A draft may name
    // an *earlier* predicted ID as its parent; the depth is resolved here and
    // reproduced exactly by the apply pass, so validation cannot diverge.
    let predicted_base = bus.arenas.tasks.allocated();
    let mut predicted_next = predicted_base;
    let mut predicted_depth: BTreeMap<TaskId, u32> = BTreeMap::new();
    let mut completing: BTreeSet<TaskId> = BTreeSet::new();
    let mut kinds: Vec<TaskKind> = Vec::with_capacity(ordered.len());
    let mut new_tasks: u32 = 0;
    let mut new_results: u32 = 0;
    let mut new_diagnostics: u32 = 0;
    let mut new_requests: u32 = 0;
    let mut patches: u32 = 0;

    for &(_, _, index) in &ordered {
        let tagged = &proposals[index];
        let task = bus
            .arenas
            .tasks
            .get(tagged.task)
            .map_err(|_| CommitError::UnknownTask { task: tagged.task })?;
        kinds.push(task.kind);
        if !matches!(task.state, TaskState::Running) {
            return Err(if task.state.is_terminal() {
                CommitError::DuplicateCompletion { task: tagged.task }
            } else {
                CommitError::TaskNotRunning { task: tagged.task }
            });
        }
        if tagged.chip != task.owner {
            return Err(CommitError::ChipNotOwner {
                chip: tagged.chip,
                owner: task.owner,
            });
        }
        match &tagged.proposal {
            Proposal::Enqueue(draft) => {
                // The destination chip must be a registered worker that
                // accepts this kind; a producer cannot mint work for an
                // unauthorized or unknown owner.
                let destination = bus
                    .registrations
                    .get(draft.owner)
                    .ok_or(CommitError::UnregisteredChip { chip: draft.owner })?;
                if !destination.accepts_kind(draft.kind) {
                    return Err(CommitError::TaskKindNotAccepted {
                        chip: draft.owner,
                        kind: draft.kind,
                    });
                }
                let parent_depth = match draft.parent {
                    None => 0u32,
                    Some(parent) => match predicted_depth.get(&parent) {
                        Some(&depth) => depth,
                        None if bus.arenas.tasks.get(parent).is_ok() => {
                            bus.task_depth(Some(parent))?
                        }
                        None => return Err(CommitError::DanglingParent { parent }),
                    },
                };
                let depth = parent_depth + 1;
                if depth > limits.max_task_depth {
                    return Err(CommitError::Limit(LimitError::TaskDepth {
                        limit: limits.max_task_depth,
                        requested: depth,
                    }));
                }
                predicted_depth.insert(TaskId::from_index(predicted_next), depth);
                predicted_next += 1;
                new_tasks += 1;
            }
            Proposal::Complete {
                task: proposal_task,
                ..
            } => {
                bind_inner_task(tagged.task, *proposal_task)?;
                if !completing.insert(tagged.task) {
                    return Err(CommitError::DuplicateCompletion { task: tagged.task });
                }
                new_results += 1;
            }
            Proposal::Fail {
                task: proposal_task,
                ..
            } => {
                bind_inner_task(tagged.task, *proposal_task)?;
                if !completing.insert(tagged.task) {
                    return Err(CommitError::DuplicateCompletion { task: tagged.task });
                }
                new_diagnostics += 1;
            }
            Proposal::AwaitHost {
                task: proposal_task,
                ..
            } => {
                bind_inner_task(tagged.task, *proposal_task)?;
                if !completing.insert(tagged.task) {
                    return Err(CommitError::DuplicateCompletion { task: tagged.task });
                }
                new_requests += 1;
            }
            Proposal::StorePatch(patch) => {
                validate_patch(bus, tagged.chip, tagged.task, task, patch)?;
                patches += 1;
            }
        }
    }

    // ---- Capacity preflight ---------------------------------------------
    check_capacity(
        bus,
        new_tasks,
        new_results,
        new_diagnostics,
        new_requests,
        patches,
    )?;

    // ---- Apply pass (infallible appends) --------------------------------
    let mut report = CommitReport {
        patches,
        ..Default::default()
    };
    let ready_tick = bus.control.tick.wrapping_add(1);
    let mut touched_stores: BTreeSet<StoreId> = BTreeSet::new();
    for (&(_, _, index), &kind) in ordered.iter().zip(kinds.iter()) {
        let tagged = &proposals[index];
        match &tagged.proposal {
            Proposal::Enqueue(draft) => {
                let id = bus.push_task_reserved(draft.clone(), ready_tick);
                bus.tasks.ready.push(id);
                report.enqueued.push(id);
            }
            Proposal::Complete { task, value } => {
                let result = bus.arenas.results.push(ResultRecord {
                    task: *task,
                    kind,
                    value: value.clone(),
                    version: 0,
                    consumed: false,
                });
                set_task_state(bus, *task, TaskState::Completed(result));
                report.completed.push((*task, result));
            }
            Proposal::Fail { task, diagnostic } => {
                let ordinal = bus.control.next_diagnostic_ordinal;
                let record = bus
                    .arenas
                    .diagnostics
                    .push(DiagnosticRecord::commit(ordinal, diagnostic.clone()));
                bus.control.next_diagnostic_ordinal += 1;
                set_task_state(bus, *task, TaskState::Failed(record));
                report.failed.push((*task, record));
            }
            Proposal::AwaitHost { task, request } => {
                let request_id = bus.arenas.host_requests.push(HostRequestRecord {
                    task: *task,
                    kind: request.kind,
                    payload: request.payload.clone(),
                    satisfied: false,
                });
                set_task_state(
                    bus,
                    *task,
                    TaskState::Waiting(crate::task::WaitSet {
                        children: Vec::new(),
                        host_request: Some(request_id),
                    }),
                );
                report.waiting.push(*task);
            }
            Proposal::StorePatch(patch) => {
                let ordinal = bus.patch_log.len() as u64;
                bus.patch_log.push(CommittedPatch {
                    ordinal,
                    task: patch.task,
                    owner: patch.owner,
                    store: patch.store,
                    field: patch.field,
                    op: patch.op,
                    target: patch.target,
                    value: patch.value,
                });
                touched_stores.insert(patch.store);
            }
        }
    }
    for store in touched_stores {
        bus.store_versions.bump(store);
    }

    Ok(report)
}

/// Bind an inner proposal task ID to its enclosing task.
fn bind_inner_task(task: TaskId, proposal_task: TaskId) -> Result<(), CommitError> {
    if task == proposal_task {
        Ok(())
    } else {
        Err(CommitError::InnerTaskMismatch {
            proposal_task,
            task,
        })
    }
}

/// Set a task's state. The task is known to exist (validated); failure is
/// impossible and intentionally ignored rather than panicking.
fn set_task_state(bus: &mut CompilerBus, task: TaskId, state: TaskState) {
    if let Ok(record) = bus.arenas.tasks.get_mut(task) {
        record.state = state;
    }
}

fn validate_patch(
    bus: &CompilerBus,
    chip: ChipId,
    task_id: TaskId,
    task: &crate::task::Task,
    patch: &StorePatch,
) -> Result<(), CommitError> {
    if patch.task != task_id {
        return Err(CommitError::TaskAttributionMismatch {
            patch_task: patch.task,
            task: task_id,
        });
    }
    if patch.owner != chip {
        return Err(CommitError::PatchOwnerMismatch {
            patch_owner: patch.owner,
            chip,
        });
    }
    if patch.store == StoreId::Config {
        return Err(CommitError::ReadOnlyStore {
            store: patch.store.name(),
        });
    }
    let manifest = bus
        .registrations
        .get(chip)
        .ok_or(CommitError::UnregisteredChip { chip })?;
    if !manifest.accepts_kind(task.kind) {
        return Err(CommitError::TaskKindNotAccepted {
            chip,
            kind: task.kind,
        });
    }
    if !manifest.declares_write(patch.store, patch.field) {
        return Err(CommitError::WriteNotDeclared {
            chip,
            store: patch.store.name(),
            field: patch.field,
        });
    }
    if !bus
        .schema
        .has_field(&crate::manifest::FieldPath::new(patch.store, patch.field))
    {
        return Err(CommitError::UndeclaredStoreField {
            store: patch.store.name(),
            field: patch.field,
        });
    }
    let current = bus.store_versions.get(patch.store);
    if patch.version != current {
        return Err(CommitError::StaleVersion {
            store: patch.store.name(),
            version: patch.version,
            current,
        });
    }
    match (patch.op, patch.target, patch.value) {
        (PatchOp::Append, None, Some(_)) => Ok(()),
        (PatchOp::Replace, Some(_), Some(_)) => Ok(()),
        (PatchOp::Tombstone, Some(_), None) => Ok(()),
        (PatchOp::Append, _, _) => Err(CommitError::InvalidPatchShape {
            task: task_id,
            reason: "append requires no target and a value",
        }),
        (PatchOp::Replace, _, _) => Err(CommitError::InvalidPatchShape {
            task: task_id,
            reason: "replace requires a target and a value",
        }),
        (PatchOp::Tombstone, _, _) => Err(CommitError::InvalidPatchShape {
            task: task_id,
            reason: "tombstone requires a target and no value",
        }),
    }
}

fn check_capacity(
    bus: &CompilerBus,
    new_tasks: u32,
    new_results: u32,
    new_diagnostics: u32,
    new_requests: u32,
    patches: u32,
) -> Result<(), CommitError> {
    let limits = bus.limits();
    // Queue bound.
    let queue = bus.tasks.ready.len() as u32;
    if new_tasks > limits.max_queue_len.saturating_sub(queue) {
        return Err(CommitError::Limit(LimitError::Queue {
            limit: limits.max_queue_len,
            requested: queue.saturating_add(new_tasks),
        }));
    }
    // Per-arena bound.
    let per_arena = limits.max_records_per_arena;
    let checks: [(u32, u32, &'static str); 4] = [
        (bus.arenas.tasks.allocated(), new_tasks, "tasks"),
        (bus.arenas.results.allocated(), new_results, "results"),
        (
            bus.arenas.diagnostics.allocated(),
            new_diagnostics,
            "diagnostics",
        ),
        (
            bus.arenas.host_requests.allocated(),
            new_requests,
            "host_requests",
        ),
    ];
    for (allocated, additional, arena) in checks {
        if additional > per_arena.saturating_sub(allocated) {
            return Err(CommitError::Capacity(ArenaError::CapacityExceeded {
                arena,
                limit: per_arena,
                requested: allocated.saturating_add(additional),
            }));
        }
    }
    // Task total.
    let requested_tasks = bus.arenas.tasks.allocated() as u64 + new_tasks as u64;
    if requested_tasks > limits.max_tasks_total {
        return Err(CommitError::Limit(LimitError::TasksTotal {
            limit: limits.max_tasks_total,
            requested: requested_tasks,
        }));
    }
    // Diagnostic total.
    bus.ensure_diagnostics(new_diagnostics)?;
    // Total records (includes the patch log).
    let additional =
        (new_tasks + new_results + new_diagnostics + new_requests) as u64 + patches as u64;
    bus.ensure_total_records(additional)?;
    Ok(())
}

/// Mark a result as consumed, exactly once.
pub fn consume_result(bus: &mut CompilerBus, result: ResultId) -> Result<ResultValue, CommitError> {
    let record = bus
        .arenas
        .results
        .get_mut(result)
        .map_err(|_| CommitError::UnknownResult { result })?;
    if record.consumed {
        return Err(CommitError::ResultAlreadyConsumed { result });
    }
    record.consumed = true;
    Ok(record.value.clone())
}
