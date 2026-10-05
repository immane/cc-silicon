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
use crate::ids::{
    BlockId, ChipId, ContinuationId, DiagnosticId, NameId, RecordFamily, RecordRef, ResultId,
    TaskId,
};
use crate::intern::InternError;
use crate::limits::LimitError;
use crate::task::{
    ChildRef, ContinuationRef, HostRequestRecord, PatchOp, Proposal, ResultRecord, ResultValue,
    StoreId, StorePatch, TaskKind, TaskState, WaitSet,
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
    /// A per-stage queue bound would be exceeded; nothing committed.
    ///
    /// The commit-path reject-before-apply backpressure signal: the
    /// no-mutation pre-apply pass computed
    /// `checked_add(queue_len, new_tasks + progress_reinserts + join_reinserts)`
    /// against `stage_queue_bound[stage]`. A second *owner group* naming the
    /// same field is never this error; that is the registration-time
    /// `ManifestError::StoreOwnerViolation`.
    BackpressureCapacity {
        /// Stage ordinal whose queue bound was hit.
        stage: usize,
        /// Configured per-stage bound.
        limit: u32,
        /// Queue length that would result (saturating).
        requested: u32,
    },
    /// The pre-dispatch selection batch exceeds the per-tick dispatch quota;
    /// the affected tasks stay `Ready` and no state mutates.
    ///
    /// Carrier note: proposal §6.2.1 classifies dispatch-count failures as
    /// dispatcher/scheduling failures, but `/5` has no such carrier, so `/6`
    /// carries this pre-dispatch guard on `CommitError` (no state mutates;
    /// the affected tasks stay `Ready`). The dispatcher-failure carrier stays
    /// the open T01 inventory item from C17-9; re-homing needs no behavior
    /// change beyond the error type.
    SelectionBatchOverflow {
        /// Configured per-tick bound (`max_inflight_per_tick`).
        limit: u32,
        /// Observed batch size (kept lossless).
        count: usize,
    },
    /// A commit-apply validation found an unterminated entry block at the
    /// committed `FunctionEnd` terminal fact.
    ///
    /// Shape only: the T01-owned typed phase-2b hook and the exact
    /// rejection-vs-recovery ordering (OB-26) are T01/T09 co-freeze and are
    /// NOT implemented here.
    TerminatorMissing {
        /// Task carrying the `FunctionEnd` terminal fact.
        task: TaskId,
        /// Unterminated entry block.
        block: BlockId,
    },
    /// An `AwaitChildren` child reference is invalid (naming rule `/6`).
    AwaitChildrenRefInvalid {
        /// Enclosing task.
        task: TaskId,
        /// Human-readable reason.
        reason: &'static str,
    },
    /// A continuation reference is invalid (naming rule `/6`).
    ContinuationRefInvalid {
        /// Enclosing task.
        task: TaskId,
        /// Human-readable reason.
        reason: &'static str,
    },
    /// A dispatched task produced no transition in one batch.
    TaskNotTransitioned {
        /// Offending task.
        task: TaskId,
    },
    /// A `Progress` ordinal did not strictly advance.
    NonAdvancingProgress {
        /// Offending task.
        task: TaskId,
        /// The non-advancing ordinal.
        ordinal: u64,
    },
    /// A task exhausted its `Progress` reschedules.
    ProgressLimit {
        /// Offending task.
        task: TaskId,
        /// Configured bound (`max_task_progress`).
        limit: u32,
        /// Observed reschedule count.
        count: u32,
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
            Self::BackpressureCapacity {
                stage,
                limit,
                requested,
            } => write!(
                f,
                "stage {stage} queue length {requested} exceeds bound {limit}"
            ),
            Self::SelectionBatchOverflow { limit, count } => {
                write!(f, "selection batch size {count} exceeds limit {limit}")
            }
            Self::TerminatorMissing { task, block } => write!(
                f,
                "task {} entry block {} is unterminated",
                task.index(),
                block.index()
            ),
            Self::AwaitChildrenRefInvalid { task, reason } => {
                write!(
                    f,
                    "task {} await-children reference invalid: {reason}",
                    task.index()
                )
            }
            Self::ContinuationRefInvalid { task, reason } => {
                write!(
                    f,
                    "task {} continuation reference invalid: {reason}",
                    task.index()
                )
            }
            Self::TaskNotTransitioned { task } => {
                write!(f, "task {} produced no transition", task.index())
            }
            Self::NonAdvancingProgress { task, ordinal } => write!(
                f,
                "task {} progress ordinal {ordinal} did not advance",
                task.index()
            ),
            Self::ProgressLimit { task, limit, count } => write!(
                f,
                "task {} progress count {count} exceeds limit {limit}",
                task.index()
            ),
            Self::Limit(error) => write!(f, "limit: {error}"),
            Self::Capacity(error) => write!(f, "capacity: {error}"),
        }
    }
}

impl std::error::Error for CommitError {}

impl CommitError {
    /// Structured diagnostic code for this failure.
    ///
    /// Explicit numeric assignments (group, code), freeze record:
    /// pre-existing variants all map to `(Protocol, 1)` (unchanged `/5`
    /// behavior); the `/6` additions map as follows —
    /// `BackpressureCapacity` → `(Protocol, 10)`,
    /// `SelectionBatchOverflow` → `(Protocol, 11)`,
    /// `AwaitChildrenRefInvalid` → `(Protocol, 12)`,
    /// `ContinuationRefInvalid` → `(Protocol, 13)`,
    /// `TaskNotTransitioned` → `(Protocol, 14)`,
    /// `NonAdvancingProgress` → `(Protocol, 15)`,
    /// `ProgressLimit` → `(Protocol, 16)`,
    /// `TerminatorMissing` → `(Task, 10)`.
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::BackpressureCapacity { .. } => DiagnosticCode::new(DiagGroup::Protocol, 10),
            Self::SelectionBatchOverflow { .. } => DiagnosticCode::new(DiagGroup::Protocol, 11),
            Self::AwaitChildrenRefInvalid { .. } => DiagnosticCode::new(DiagGroup::Protocol, 12),
            Self::ContinuationRefInvalid { .. } => DiagnosticCode::new(DiagGroup::Protocol, 13),
            Self::TaskNotTransitioned { .. } => DiagnosticCode::new(DiagGroup::Protocol, 14),
            Self::NonAdvancingProgress { .. } => DiagnosticCode::new(DiagGroup::Protocol, 15),
            Self::ProgressLimit { .. } => DiagnosticCode::new(DiagGroup::Protocol, 16),
            Self::TerminatorMissing { .. } => DiagnosticCode::new(DiagGroup::Task, 10),
            _ => DiagnosticCode::new(DiagGroup::Protocol, 1),
        }
    }

    /// Map to a structured diagnostic.
    pub fn to_diagnostic(&self) -> DiagnosticDraft {
        DiagnosticDraft::error(self.code(), self.to_string())
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
    /// Failed tasks and their diagnostic IDs (proposal failures and
    /// await-all join failures alike).
    pub failed: Vec<(TaskId, DiagnosticId)>,
    /// Tasks moved to waiting.
    pub waiting: Vec<TaskId>,
    /// Waiting parents reinserted to `Ready` by the await-all join.
    pub rejoined: Vec<TaskId>,
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
    // This batch's predicted `Enqueue` IDs per enclosing task, in validation
    // (== apply) order: phase-2b `OwnBatch` child resolution indexes these.
    let mut own_enqueues: BTreeMap<TaskId, Vec<TaskId>> = BTreeMap::new();
    // `AwaitChildren` proposals deferred to phase 2b (all enqueues known).
    let mut await_children: Vec<(TaskId, Vec<ChildRef>)> = Vec::new();
    // Tasks this batch completes/fails (exact post-batch child states for the
    // join simulation below).
    let mut completed_by_batch: BTreeSet<TaskId> = BTreeSet::new();
    let mut failed_by_batch: BTreeSet<TaskId> = BTreeSet::new();
    // `AppendRecords` owners: at most one batch per task per batch.
    let mut appenders: BTreeSet<TaskId> = BTreeSet::new();
    let mut new_tasks: u32 = 0;
    let mut new_results: u32 = 0;
    let mut new_diagnostics: u32 = 0;
    let mut new_requests: u32 = 0;
    let mut patches: u32 = 0;
    let mut progress_reinserts: u32 = 0;
    // Progress tasks that fail validation per-task (ordinal or limit):
    // routed to `try_fail_task` at apply time, mutually exclusive with
    // reinsert. They consume no reinsert capacity.
    let mut progress_failed: Vec<(TaskId, CommitError)> = Vec::new();
    let mut total_drafts: u32 = 0;

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
                own_enqueues
                    .entry(tagged.task)
                    .or_default()
                    .push(TaskId::from_index(predicted_next));
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
                completed_by_batch.insert(tagged.task);
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
                failed_by_batch.insert(tagged.task);
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
            Proposal::AppendRecords {
                task: proposal_task,
                batch,
            } => {
                bind_inner_task(tagged.task, *proposal_task)?;
                if !appenders.insert(tagged.task) {
                    return Err(CommitError::InvalidPatchShape {
                        task: tagged.task,
                        reason: "at most one append batch per task per batch",
                    });
                }
                if batch.records.is_empty() {
                    return Err(CommitError::InvalidPatchShape {
                        task: tagged.task,
                        reason: "empty append batch",
                    });
                }
                total_drafts = total_drafts.saturating_add(batch.records.len() as u32);
                // Drafts count against the per-tick proposal budget together
                // with the proposal count.
                check_proposal_budget(
                    proposals.len().saturating_add(total_drafts as usize),
                    limits.max_proposals_per_tick,
                )?;
                // Typed draft materialization (phase P2d) is records-track
                // work over the canonical `RecordDraft`: the deterministic
                // reservation core (`reserve_predicted_ranges`, `NamePlan`,
                // `validate_draft_links`, `ResolvedTable`) is provided below
                // for that apply. Until it lands, acceptance would silently
                // drop drafts, so validation rejects explicitly here.
                return Err(CommitError::InvalidPatchShape {
                    task: tagged.task,
                    reason: "record draft materialization pending the records track",
                });
            }
            Proposal::Progress {
                task: proposal_task,
                ordinal,
            } => {
                bind_inner_task(tagged.task, *proposal_task)?;
                if !completing.insert(tagged.task) {
                    return Err(CommitError::DuplicateCompletion { task: tagged.task });
                }
                // Ordinal-advance and progress-limit checks run against the
                // stored `Task.progress_ordinal`/`progress_count`. Exceedance
                // (or a non-advancing ordinal) routes to the per-task `Fail`
                // path at apply time, never a whole-batch reject.
                if let Err(error) =
                    check_progress_ordinal(tagged.task, task.progress_ordinal, *ordinal)
                {
                    progress_failed.push((tagged.task, error));
                    continue;
                }
                match check_progress_limit(
                    tagged.task,
                    task.progress_count,
                    limits.max_task_progress,
                ) {
                    Ok(_) => {
                        progress_reinserts = progress_reinserts.saturating_add(1);
                    }
                    Err(error) => {
                        progress_failed.push((tagged.task, error));
                    }
                }
            }
            Proposal::AwaitChildren {
                task: proposal_task,
                children,
            } => {
                bind_inner_task(tagged.task, *proposal_task)?;
                if !completing.insert(tagged.task) {
                    return Err(CommitError::DuplicateCompletion { task: tagged.task });
                }
                if children.is_empty() {
                    return Err(CommitError::AwaitChildrenRefInvalid {
                        task: tagged.task,
                        reason: "await-children requires at least one child",
                    });
                }
                await_children.push((tagged.task, children.clone()));
            }
            Proposal::StorePatch(patch) => {
                validate_patch(bus, tagged.chip, tagged.task, task, patch)?;
                patches += 1;
            }
        }
    }

    // ---- Phase 2b: own-batch child resolution -----------------------------
    // All `Enqueue`s are known now, so `OwnBatch` indexes resolve against the
    // final per-task predicted lists. Committed children pass the predicate.
    let mut resolved_waits: Vec<(TaskId, Vec<TaskId>)> = Vec::with_capacity(await_children.len());
    let mut resolved_wait_map: BTreeMap<TaskId, Vec<TaskId>> = BTreeMap::new();
    for (parent, children) in &await_children {
        let empty: Vec<TaskId> = Vec::new();
        let own = own_enqueues.get(parent).unwrap_or(&empty);
        let mut resolved = Vec::with_capacity(children.len());
        for child in children {
            resolved.push(resolve_child_ref(bus, *parent, child, own)?);
        }
        resolved_wait_map.insert(*parent, resolved.clone());
        resolved_waits.push((*parent, resolved));
    }

    // ---- Join simulation (exact reinsert counts) --------------------------
    // Decide every join against post-batch child states. Children completed
    // or failed by this batch read post-batch (`completed_by_batch` /
    // `failed_by_batch`, both exact after validation); other dispatched
    // children (`AwaitHost`/`AwaitChildren`/`Progress`) are non-terminal.
    // Waiters that fail only via a batch-failed child defer to apply time,
    // when that child's diagnostic ID exists; their `Failed` outcome needs no
    // capacity, so the reinsert count below stays exact. New waiters from
    // `resolved_waits` join the pre-existing `Waiting` parents in one
    // deterministic scan.
    let mut join_readied: Vec<TaskId> = Vec::new();
    let mut join_failed: Vec<(TaskId, DiagnosticId)> = Vec::new();
    let mut join_deferred: Vec<TaskId> = Vec::new();
    {
        let mut waiter_children: Vec<(TaskId, Vec<TaskId>)> = Vec::new();
        for (id, task) in bus.arenas.tasks.iter() {
            let TaskState::Waiting(wait) = &task.state else {
                continue;
            };
            if wait.host_request.is_some() || wait.children.is_empty() {
                continue;
            }
            waiter_children.push((id, wait.children.clone()));
        }
        waiter_children.extend(resolved_waits.iter().cloned());
        waiter_children.sort_unstable();
        waiter_children.dedup_by(|a, b| a.0 == b.0);
        for (id, children) in &waiter_children {
            let mut first_failed: Option<DiagnosticId> = None;
            let mut all_completed = true;
            let mut deferred = false;
            for child in children {
                if completed_by_batch.contains(child) {
                    continue;
                }
                if failed_by_batch.contains(child) {
                    deferred = true;
                    all_completed = false;
                    break;
                }
                match bus.arenas.tasks.get(*child) {
                    Ok(child_record) => match &child_record.state {
                        TaskState::Completed(_) => {}
                        TaskState::Failed(diagnostic) => {
                            if first_failed.is_none() {
                                first_failed = Some(*diagnostic);
                            }
                            all_completed = false;
                        }
                        _ => {
                            all_completed = false;
                        }
                    },
                    Err(_) => {
                        all_completed = false;
                    }
                }
            }
            if deferred {
                join_deferred.push(*id);
            } else if let Some(diagnostic) = first_failed {
                join_failed.push((*id, diagnostic));
            } else if all_completed {
                join_readied.push(*id);
            }
        }
    }
    // Deferred waiters fail at apply time, when the batch-failed child's
    // diagnostic ID exists; that outcome needs no capacity.

    // ---- Capacity preflight ---------------------------------------------
    // Drafts already counted against the proposal budget per append arm
    // above; the combined check below stays as the single post-loop
    // enforcement point for when materialization lands.
    check_proposal_budget(
        proposals.len().saturating_add(total_drafts as usize),
        limits.max_proposals_per_tick,
    )?;
    check_capacity(
        bus,
        new_tasks,
        new_results,
        new_diagnostics,
        new_requests,
        patches,
    )?;
    // Per-stage backpressure projection with real reinsert counts: the single
    // ready queue stands in for one stage queue and the legacy
    // `max_queue_len` for `stage_queue_bound[stage]` until `stage_queues`
    // land. Subsumed by the `Limit::Queue` check above while the projection
    // holds; the per-stage loop replaces this call site then.
    let join_reinserts = join_readied.len() as u32;
    check_backpressure_capacity(
        bus.tasks.ready.len() as u32,
        new_tasks,
        progress_reinserts,
        join_reinserts,
        bus.limits().max_queue_len,
        0,
    )?;
    // Draft materialization records: each draft becomes one record.
    bus.ensure_total_records(total_drafts as u64)?;

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
            Proposal::AppendRecords { task, .. } => {
                // Unreachable: validation rejects every append batch with
                // `InvalidPatchShape` until the records track lands typed
                // materialization. The arm exists for exhaustiveness only.
                let _ = task;
            }
            Proposal::Progress { task, ordinal } => {
                // Preflighted above (task `Running`; queue capacity reserved
                // including this reinsert): infallible here. Own-stage
                // projection is `tasks.ready` until `stage_queues` land.
                // A validation-time ordinal/limit failure routes to the
                // per-task `Fail` path instead, mutually exclusive with
                // reinsert (never both, never stranded `Running`).
                if let Some((_, error)) = progress_failed.iter().find(|(id, _)| id == task) {
                    let draft = DiagnosticDraft::error(error.code(), error.to_string());
                    try_fail_task(bus, *task, draft);
                    continue;
                }
                if let Ok(record) = bus.arenas.tasks.get_mut(*task) {
                    record.progress_ordinal = *ordinal;
                    record.progress_count = record.progress_count.saturating_add(1);
                }
                reinsert_ready(bus, *task, ready_tick);
            }
            Proposal::AwaitChildren { task, .. } => {
                // Resolved in phase 2b; infallible here. A missing entry is
                // impossible after validation; the fallback fails the task
                // through the infallible `try_fail_task` path instead of
                // stranding it `Running`.
                match resolved_wait_map.get(task) {
                    Some(children) => {
                        set_task_state(
                            bus,
                            *task,
                            TaskState::Waiting(crate::task::WaitSet {
                                children: children.clone(),
                                host_request: None,
                            }),
                        );
                        report.waiting.push(*task);
                    }
                    None => {
                        try_fail_task(bus, *task, task_not_transitioned_draft(*task));
                    }
                }
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
    // ---- Join execution (preflighted exactly) ---------------------------
    // Readied parents reinsert (one own-stage entry each); join failures reuse
    // the first failed child's diagnostic, exactly once, never `Ready`. Child
    // results are not consumed. Deferred waiters fail via their batch-failed
    // child, whose diagnostic exists now; a missing diagnostic is impossible
    // after validation, and such a waiter defensively stays `Waiting`.
    for id in join_readied {
        reinsert_ready(bus, id, ready_tick);
        report.rejoined.push(id);
    }
    for (id, diagnostic) in join_failed {
        set_task_state(bus, id, TaskState::Failed(diagnostic));
        report.failed.push((id, diagnostic));
    }
    for id in join_deferred {
        let diagnostic = bus
            .arenas
            .tasks
            .get(id)
            .ok()
            .and_then(|record| match &record.state {
                TaskState::Waiting(wait) => Some(wait.children.clone()),
                _ => None,
            })
            .and_then(|children| {
                children
                    .iter()
                    .find_map(|child| match bus.arenas.tasks.get(*child) {
                        Ok(child_record) => match child_record.state {
                            TaskState::Failed(diagnostic) => Some(diagnostic),
                            _ => None,
                        },
                        Err(_) => None,
                    })
            });
        if let Some(diagnostic) = diagnostic {
            set_task_state(bus, id, TaskState::Failed(diagnostic));
            report.failed.push((id, diagnostic));
        }
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

// ============================================================================
// `/6` working-basis commit extensions (Group A/B/C decided shapes).
//
// The `Proposal::AppendRecords`/`Progress`/`AwaitChildren` variants,
// `ChildRef`/`ContinuationRef`/`DraftRef`, and the wire tags are canonical in
// `task.rs`. The per-task progress counters, `stage_queues`, the
// `RecordFamily` ordinals, and the `RecordDraft` element type are NOT declared
// yet (parallel tracks). The stand-ins below (`RecordFamily`, `RecordLink`,
// `PendingDraft`, `NamePlan`, `ResolvedTable`, predicted ranges) use the
// decided spellings for the deterministic reservation protocol; the
// integrator wires them to the canonical declarations when those land. Every
// helper preserves failure atomicity: validation runs before any mutation.
// ============================================================================

/// Check a committed child reference: the child exists, is live, and its
/// committed-Enqueue parent is this task. Anything else is
/// [`CommitError::AwaitChildrenRefInvalid`].
pub fn check_child_committed(
    bus: &CompilerBus,
    parent: TaskId,
    child: TaskId,
) -> Result<(), CommitError> {
    match bus.arenas.tasks.get(child) {
        Ok(record) if record.parent == Some(parent) => Ok(()),
        Ok(_) => Err(CommitError::AwaitChildrenRefInvalid {
            task: parent,
            reason: "child is not a committed child of this task",
        }),
        Err(_) => Err(CommitError::AwaitChildrenRefInvalid {
            task: parent,
            reason: "child task does not exist",
        }),
    }
}

/// Check a committed continuation reference: the continuation exists and is
/// live. Anything else is [`CommitError::ContinuationRefInvalid`].
pub fn check_continuation_committed(
    bus: &CompilerBus,
    task: TaskId,
    id: ContinuationId,
) -> Result<(), CommitError> {
    match bus.arenas.continuations.get(id) {
        Ok(_) => Ok(()),
        Err(_) => Err(CommitError::ContinuationRefInvalid {
            task,
            reason: "continuation does not exist",
        }),
    }
}

/// Resolve a child reference against the canonical [`ChildRef`]: committed
/// children pass the [`check_child_committed`] predicate; an own-batch index
/// resolves against this task's own predicted `Enqueue` IDs in batch order
/// (`own_enqueues`; same task only, cross-task indexes unrepresentable).
pub fn resolve_child_ref(
    bus: &CompilerBus,
    parent: TaskId,
    child: &ChildRef,
    own_enqueues: &[TaskId],
) -> Result<TaskId, CommitError> {
    match *child {
        ChildRef::Committed(id) => {
            check_child_committed(bus, parent, id)?;
            Ok(id)
        }
        ChildRef::OwnBatch(index) => match own_enqueues.get(index as usize) {
            Some(id) => Ok(*id),
            None => Err(CommitError::AwaitChildrenRefInvalid {
                task: parent,
                reason: "own-batch child index out of range",
            }),
        },
    }
}

/// Resolve a continuation reference against the canonical
/// [`ContinuationRef`]: committed continuations pass the
/// [`check_continuation_committed`] predicate; an own-batch key resolves
/// against this task's own `AppendRecords` drafts whose continuation family
/// starts at `append_base`.
pub fn resolve_continuation_ref(
    bus: &CompilerBus,
    task: TaskId,
    continuation: &ContinuationRef,
    own_append_count: u32,
    append_base: ContinuationId,
) -> Result<ContinuationId, CommitError> {
    match *continuation {
        ContinuationRef::Committed(id) => {
            check_continuation_committed(bus, task, id)?;
            Ok(id)
        }
        ContinuationRef::OwnBatch(draft) => {
            let index = draft.index();
            if index < own_append_count {
                match append_base.index().checked_add(index) {
                    Some(raw) => Ok(ContinuationId::from_index(raw)),
                    None => Err(CommitError::ContinuationRefInvalid {
                        task,
                        reason: "own-batch continuation index overflows",
                    }),
                }
            } else {
                Err(CommitError::ContinuationRefInvalid {
                    task,
                    reason: "own-batch continuation index out of range",
                })
            }
        }
    }
}

/// Pre-apply backpressure check (T02 H9.2): the no-mutation pass computes
/// `checked_add(queue_len, new_tasks + progress_reinserts + join_reinserts)`
/// against `stage_queue_bound[stage]`.
///
/// All additions are checked (never a panic or silent drop). Overflow or a
/// total above `bound` rejects the whole batch with
/// [`CommitError::BackpressureCapacity`] before any mutation. The canonical
/// `stage_queues` already exclude tasks the dispatcher removed this tick, so
/// callers must not subtract them again.
pub fn check_backpressure_capacity(
    queue_len: u32,
    new_tasks: u32,
    progress_reinserts: u32,
    join_reinserts: u32,
    bound: u32,
    stage: usize,
) -> Result<(), CommitError> {
    let mut total = queue_len;
    for add in [new_tasks, progress_reinserts, join_reinserts] {
        match total.checked_add(add) {
            Some(next) => total = next,
            None => {
                return Err(CommitError::BackpressureCapacity {
                    stage,
                    limit: bound,
                    requested: u32::MAX,
                });
            }
        }
    }
    if total > bound {
        return Err(CommitError::BackpressureCapacity {
            stage,
            limit: bound,
            requested: total,
        });
    }
    Ok(())
}

/// Check a `Progress` ordinal strictly advances; otherwise
/// [`CommitError::NonAdvancingProgress`].
///
/// The canonical ordinal is `u64` (`Proposal::Progress::ordinal`). The last
/// ordinal lives in `task.rs` (pending); this function takes it explicitly so
/// the check is testable now.
pub fn check_progress_ordinal(task: TaskId, last: u64, next: u64) -> Result<(), CommitError> {
    if next > last {
        Ok(())
    } else {
        Err(CommitError::NonAdvancingProgress {
            task,
            ordinal: next,
        })
    }
}

/// Check a `Progress` reschedule against `max_task_progress` and return the
/// next count for the caller to persist.
///
/// `max_task_progress == 0` disables `Progress`: any reschedule exceeds.
/// Exceedance (or counter overflow) is [`CommitError::ProgressLimit`], which
/// the caller routes to the per-task `Fail` path. The `progress_count` field
/// itself lives in `task.rs` (pending); this function takes/returns counts
/// explicitly so the check is testable now.
pub fn check_progress_limit(
    task: TaskId,
    count: u32,
    max_task_progress: u32,
) -> Result<u32, CommitError> {
    match count.checked_add(1) {
        Some(next) if next <= max_task_progress => Ok(next),
        _ => Err(CommitError::ProgressLimit {
            task,
            limit: max_task_progress,
            count,
        }),
    }
}

/// Reinsert a task to `Ready` with exactly one own-stage queue entry.
///
/// Sets `Ready`/`ready_tick`, drops any stale queue entries for the task,
/// then pushes one. The single `tasks.ready` queue is the own-stage
/// projection until `stage_queues` land. Infallible by construction; callers
/// preflight capacity.
fn reinsert_ready(bus: &mut CompilerBus, task: TaskId, ready_tick: u64) {
    if let Ok(record) = bus.arenas.tasks.get_mut(task) {
        record.state = TaskState::Ready;
        record.ready_tick = ready_tick;
    }
    bus.tasks.ready.retain(|id| *id != task);
    bus.tasks.ready.push(task);
}

/// Apply a `Progress` reinsert: a `Running` task returns to `Ready` with
/// `ready_tick = tick + 1` and exactly one own-stage queue entry, selectable
/// next tick.
///
/// The single `tasks.ready` queue is the own-stage projection until
/// `stage_queues[stage_of(task.kind)]` lands (never the derived view twice).
/// Exceeding the queue bound is [`LimitError::Queue`]. Anything but
/// `Running` is [`CommitError::TaskNotRunning`]; missing tasks are
/// [`CommitError::UnknownTask`].
pub fn reinsert_for_progress(bus: &mut CompilerBus, task: TaskId) -> Result<(), CommitError> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| CommitError::UnknownTask { task })?;
    if !matches!(record.state, TaskState::Running) {
        return Err(CommitError::TaskNotRunning { task });
    }
    let limit = bus.limits().max_queue_len;
    let requested = bus.tasks.ready.len() as u32 + 1;
    if requested > limit {
        return Err(CommitError::Limit(LimitError::Queue { limit, requested }));
    }
    reinsert_ready(bus, task, bus.control.tick.wrapping_add(1));
    Ok(())
}

/// Outcome of scanning `Waiting` parents for a join decision.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JoinPoll {
    /// Parents that became `Ready` (one own-stage insert each).
    pub readied: Vec<TaskId>,
    /// Parents that became `Failed` once, with the reused child diagnostic.
    pub failed: Vec<(TaskId, DiagnosticId)>,
}

/// Begin an `AwaitChildren` wait: validate every committed child with
/// [`check_child_committed`], then transition the `Running` parent to
/// `Waiting(WaitSet { children, host_request: None })`.
///
/// Whole-batch validation runs before any mutation. An empty child list is
/// [`CommitError::AwaitChildrenRefInvalid`]; a non-`Running` parent is
/// [`CommitError::TaskNotRunning`]. Own-batch child keys must be resolved to
/// committed IDs with [`resolve_child_ref`] before calling.
pub fn begin_await_children(
    bus: &mut CompilerBus,
    parent: TaskId,
    children: &[TaskId],
) -> Result<(), CommitError> {
    let record = bus
        .arenas
        .tasks
        .get(parent)
        .map_err(|_| CommitError::UnknownTask { task: parent })?;
    if !matches!(record.state, TaskState::Running) {
        return Err(CommitError::TaskNotRunning { task: parent });
    }
    if children.is_empty() {
        return Err(CommitError::AwaitChildrenRefInvalid {
            task: parent,
            reason: "await-children requires at least one child",
        });
    }
    for child in children {
        check_child_committed(bus, parent, *child)?;
    }
    if let Ok(record) = bus.arenas.tasks.get_mut(parent) {
        record.state = TaskState::Waiting(WaitSet {
            children: children.to_vec(),
            host_request: None,
        });
    }
    Ok(())
}

/// Count pending join reinserts: `Waiting` parents (host-independent) whose
/// children are all `Completed`. Feeds the preflight
/// (`join_reinserts` in [`check_backpressure_capacity`]).
pub fn count_join_reinserts(bus: &CompilerBus) -> u32 {
    let mut count = 0u32;
    for (_, task) in bus.arenas.tasks.iter() {
        let TaskState::Waiting(wait) = &task.state else {
            continue;
        };
        if wait.host_request.is_some() || wait.children.is_empty() {
            continue;
        }
        let mut all_completed = true;
        for child in &wait.children {
            match bus.arenas.tasks.get(*child) {
                Ok(child_record) => {
                    if !matches!(child_record.state, TaskState::Completed(_)) {
                        all_completed = false;
                        break;
                    }
                }
                Err(_) => {
                    all_completed = false;
                    break;
                }
            }
        }
        if all_completed {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Await-all join over committed children (no new CT07 carrier).
///
/// For every `Waiting` parent with host-independent, non-empty children, in
/// ascending task-ID order: all-`Completed` → `Ready` plus one own-stage
/// queue entry (selectable next tick); any-`Failed` → `Failed` exactly once,
/// reusing the first failed child's diagnostic ID (never `Ready`). Child
/// results are NOT consumed at join: consumption stays with the parent's own
/// atomic commit. Unresolvable or non-terminal children leave the parent
/// `Waiting`.
///
/// Atomicity: the queue preflight ([`LimitError::Queue`]) and the full
/// decision scan run before the first mutation; the apply pass is infallible.
pub fn poll_await_joins(bus: &mut CompilerBus) -> Result<JoinPoll, CommitError> {
    // ---- Decision pass (no mutation) ------------------------------------
    let mut readied: Vec<TaskId> = Vec::new();
    let mut failed: Vec<(TaskId, DiagnosticId)> = Vec::new();
    for (id, task) in bus.arenas.tasks.iter() {
        let TaskState::Waiting(wait) = &task.state else {
            continue;
        };
        if wait.host_request.is_some() || wait.children.is_empty() {
            continue;
        }
        let mut first_failed: Option<DiagnosticId> = None;
        let mut all_completed = true;
        for child in &wait.children {
            match bus.arenas.tasks.get(*child) {
                Ok(child_record) => match &child_record.state {
                    TaskState::Completed(_) => {}
                    TaskState::Failed(diagnostic) => {
                        if first_failed.is_none() {
                            first_failed = Some(*diagnostic);
                        }
                        all_completed = false;
                    }
                    _ => {
                        all_completed = false;
                    }
                },
                Err(_) => {
                    all_completed = false;
                }
            }
        }
        if let Some(diagnostic) = first_failed {
            failed.push((id, diagnostic));
        } else if all_completed {
            readied.push(id);
        }
    }
    // ---- Queue preflight -------------------------------------------------
    let limit = bus.limits().max_queue_len;
    let requested = bus.tasks.ready.len() as u32 + readied.len() as u32;
    if requested > limit {
        return Err(CommitError::Limit(LimitError::Queue { limit, requested }));
    }
    // ---- Apply pass (infallible) -----------------------------------------
    let ready_tick = bus.control.tick.wrapping_add(1);
    let mut poll = JoinPoll::default();
    for id in readied {
        reinsert_ready(bus, id, ready_tick);
        poll.readied.push(id);
    }
    for (id, diagnostic) in failed {
        set_task_state(bus, id, TaskState::Failed(diagnostic));
        poll.failed.push((id, diagnostic));
    }
    Ok(poll)
}

// ---- AppendRecords deterministic reservation (P1.0/P2a-2d) ----------------
//
// Phases, per the M1 proposal: P1.0 per-task inventory; P2a name plan plus
// predicted-ref reservation; P2b link validation (including reciprocal
// token↔literal pairs, resolvable without topological order because whole-batch
// IDs are pre-reserved); P2c total resolution into the resolved table; P2d
// typed apply (group-owner work over the canonical `RecordDraft`, NOT here).
// `DraftRef` is canonical in `task.rs`.

/// `/6` stand-in: the target of a record link (decided spelling; T01 `/6`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkTarget {
    /// An already-committed record.
    Committed(RecordRef),
    /// A same-task draft resolved through the predicted table.
    Draft(crate::task::DraftRef),
}

/// A typed record link: an expected canonical [`RecordFamily`] plus a
/// target that is either committed or a same-task draft (decided shape;
///
/// `RecordLink` itself is commit-track protem until the records track freezes
/// its path; the family inventory is canonical in `ids.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordLink {
    /// Expected family of the target (checked at typed apply by the group).
    pub expect: RecordFamily,
    /// Link target.
    pub target: LinkTarget,
}

/// `/6` stand-in: one pending record draft (family shape plus links).
///
/// The closed typed `RecordDraft` enum is group-owner work; this generic
/// carrier implements the deterministic reservation protocol over it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingDraft {
    /// Draft family.
    pub family: RecordFamily,
    /// Record links carried by the draft.
    pub links: Vec<RecordLink>,
}

/// Per-task append inventory (phase P1.0): the owning task plus its draft
/// count from the canonical [`crate::task::AppendBatch`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppendInventory {
    /// Owning task.
    pub task: TaskId,
    /// Number of drafts.
    pub drafts: u32,
}

/// Collect the per-task inventory in batch order (phase P1.0).
///
/// Input order is preserved exactly; no map iteration decides ordering.
pub fn collect_append_inventory(
    batches: &[(TaskId, crate::task::AppendBatch)],
) -> Vec<AppendInventory> {
    batches
        .iter()
        .map(|(task, batch)| AppendInventory {
            task: *task,
            drafts: batch.records.len() as u32,
        })
        .collect()
}

/// A reserved predicted-ID range within one arena index space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PredictedRange {
    /// First predicted index.
    pub base: u32,
    /// Number of reserved indices.
    pub count: u32,
}

impl PredictedRange {
    /// Whether a draft index falls in this range.
    pub const fn contains(self, index: u32) -> bool {
        index >= self.base && index < self.base.saturating_add(self.count)
    }
}

/// Reserve predicted-ID ranges for per-task draft counts in order (phase
/// P2a).
///
/// `base` is the arena's allocated count before the batch; each task's range
/// starts where the previous ended. Saturating arithmetic keeps the pure
/// reservation total without panicking; capacity is enforced by the preflight,
/// not here.
pub fn reserve_predicted_ranges(base: u32, counts: &[u32]) -> Vec<PredictedRange> {
    let mut ranges = Vec::with_capacity(counts.len());
    let mut next = base;
    for count in counts {
        ranges.push(PredictedRange {
            base: next,
            count: *count,
        });
        next = next.saturating_add(*count);
    }
    ranges
}

/// The 2a name plan: draft name slots resolved to interned [`NameId`]s.
///
/// Additive support over the existing [`crate::intern::InternTable`] (which
/// is otherwise untouched): names intern once per batch through the normal
/// intern path, so repeated runs on the same input sequence allocate the same
/// IDs. First plan wins per draft slot, deterministically.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NamePlan {
    /// Draft slot to interned name, in no significant order (lookup only).
    entries: BTreeMap<u32, NameId>,
}

impl NamePlan {
    /// An empty plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a planned slot.
    pub fn get(&self, slot: u32) -> Option<NameId> {
        self.entries.get(&slot).copied()
    }

    /// Number of planned slots.
    pub fn len(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Whether the plan is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Plan one draft name slot, interning through the bus.
    ///
    /// Exhaustion surfaces the structured [`InternError`]; the AppendRecords
    /// preflight maps it before any mutation (exact carrier is a T01 `/6`
    /// detail).
    pub fn plan_name(
        &mut self,
        bus: &mut CompilerBus,
        slot: u32,
        bytes: &[u8],
    ) -> Result<NameId, InternError> {
        if let Some(existing) = self.entries.get(&slot) {
            return Ok(*existing);
        }
        let limits = bus.limits();
        let id = bus.intern.intern(bytes, &limits)?;
        self.entries.insert(slot, id);
        Ok(id)
    }
}

/// The resolved table: per-task draft indices to committed [`RecordRef`]s
/// (phase P2c output).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedTable {
    /// Draft index to committed reference.
    entries: BTreeMap<u32, RecordRef>,
}

impl ResolvedTable {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a resolution.
    pub fn insert(&mut self, draft: u32, resolved: RecordRef) {
        self.entries.insert(draft, resolved);
    }

    /// Look up a draft resolution.
    pub fn get(&self, draft: u32) -> Option<RecordRef> {
        self.entries.get(&draft).copied()
    }

    /// Resolve one link: drafts look up the table, committed refs pass
    /// through unchanged.
    pub fn resolve_link(&self, link: &RecordLink) -> Option<RecordRef> {
        match link.target {
            LinkTarget::Committed(reference) => Some(reference),
            LinkTarget::Draft(draft) => self.get(draft.index()),
        }
    }
}

/// Validate every draft link of one task against its reserved range (phase
/// P2b).
///
/// Same-task drafts must fall inside `range` (cross-task draft references are
/// unrepresentable); committed targets pass through (record references are
/// not existence-checked by the mechanical commit, per the module docs).
/// Violations are [`CommitError::InvalidPatchShape`]: no narrower draft-link
/// error name is decided, and no new name may be invented here.
pub fn validate_draft_links(
    range: &PredictedRange,
    drafts: &[PendingDraft],
    task: TaskId,
) -> Result<(), CommitError> {
    for draft in drafts {
        for link in &draft.links {
            if let LinkTarget::Draft(draft_ref) = link.target {
                if !range.contains(draft_ref.index()) {
                    return Err(CommitError::InvalidPatchShape {
                        task,
                        reason: "draft link escapes its task's reserved range",
                    });
                }
            }
        }
    }
    Ok(())
}

/// Check one reciprocal link pair inside a single reserved range (phase P2b).
///
/// Both endpoints already hold predicted durable IDs from whole-batch
/// pre-reservation, so the check is range membership only, with no
/// topological order and no ordering dependency between the two appends.
/// Typed family agreement is group-owner apply work, NOT checked here.
/// Out-of-range endpoints are [`CommitError::InvalidPatchShape`].
pub fn check_reciprocal_pair(
    range: &PredictedRange,
    first: u32,
    second: u32,
    task: TaskId,
) -> Result<(), CommitError> {
    if range.contains(first) && range.contains(second) {
        Ok(())
    } else {
        Err(CommitError::InvalidPatchShape {
            task,
            reason: "reciprocal link escapes its task's reserved range",
        })
    }
}

/// Check the T04 reciprocal token↔literal pair with canonical families.
///
/// `token_draft` (a `Token` draft) must carry a link expecting
/// [`RecordFamily::Literal`] at `Draft(literal_draft)`, and `literal_draft`
/// (a `Literal` draft) a link expecting [`RecordFamily::Token`] at
/// `Draft(token_draft)`; both drafts sit in `range`. Whole-batch
/// pre-reservation makes the cycle resolvable without topological order.
/// Anything else is [`CommitError::InvalidPatchShape`].
pub fn check_token_literal_reciprocal(
    range: &PredictedRange,
    token_draft: u32,
    token_links: &[RecordLink],
    literal_draft: u32,
    literal_links: &[RecordLink],
    task: TaskId,
) -> Result<(), CommitError> {
    use crate::task::DraftRef;
    check_reciprocal_pair(range, token_draft, literal_draft, task)?;
    let forward = token_links.iter().any(|link| {
        link.expect == RecordFamily::Literal
            && matches!(link.target, LinkTarget::Draft(draft) if draft == DraftRef(literal_draft))
    });
    let backward = literal_links.iter().any(|link| {
        link.expect == RecordFamily::Token
            && matches!(link.target, LinkTarget::Draft(draft) if draft == DraftRef(token_draft))
    });
    if forward && backward {
        Ok(())
    } else {
        Err(CommitError::InvalidPatchShape {
            task,
            reason: "token-literal links are not reciprocal",
        })
    }
}

/// The [`CommitError::TaskNotTransitioned`] diagnostic for a task.
fn task_not_transitioned_draft(task: TaskId) -> DiagnosticDraft {
    DiagnosticDraft::error(
        CommitError::TaskNotTransitioned { task }.code(),
        format!("task {} produced no transition", task.index()),
    )
}

/// A dispatched task with an empty proposal vector still needs exactly one
/// outcome: fail it with the [`CommitError::TaskNotTransitioned`] diagnostic.
///
/// This is the per-task `Fail` path for the empty vector (never silent
/// success, never stranded `Running`).
pub fn empty_proposal_fail(task: TaskId) -> Proposal {
    Proposal::Fail {
        task,
        diagnostic: task_not_transitioned_draft(task),
    }
}

/// Attempt to fail one task with a diagnostic, with the `NONE` fallback.
///
/// Per-task diagnostic attempt (H6, no pre-reservation): a committed
/// [`DiagnosticId`] when total-record and diagnostic capacity allow, else the
/// `TaskState::Failed(DiagnosticId::NONE)` sentinel, so the task is never
/// stranded. The state guard only transitions `Running` tasks (anything else
/// is left untouched and reports `None`). Returns the committed diagnostic
/// when one was allocated.
pub fn try_fail_task(
    bus: &mut CompilerBus,
    task: TaskId,
    diagnostic: DiagnosticDraft,
) -> Option<DiagnosticId> {
    let running = matches!(
        bus.arenas.tasks.get(task),
        Ok(record) if record.state == TaskState::Running
    );
    if !running {
        return None;
    }
    let ordinal = bus.control.next_diagnostic_ordinal;
    let record = DiagnosticRecord::commit(ordinal, diagnostic);
    let limits = bus.limits();
    let committed = if bus.ensure_total_records(1).is_ok() && bus.ensure_diagnostics(1).is_ok() {
        match bus.arenas.diagnostics.alloc(record, &limits) {
            Ok(id) => {
                bus.control.next_diagnostic_ordinal += 1;
                Some(id)
            }
            Err(_) => None,
        }
    } else {
        None
    };
    let state = match committed {
        Some(id) => TaskState::Failed(id),
        None => TaskState::Failed(DiagnosticId::NONE),
    };
    set_task_state(bus, task, state);
    committed
}
