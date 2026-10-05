// ============================================================================
// routing.rs — minimal routing integration shell (T01 C06)
//
// This shell carries no language logic. It reads the routing table from the
// bus (so routing is semantic input and replay cannot diverge from it), selects
// at most one ready task per tick by the frozen order (phase priority, enqueue
// ordinal, TaskId), routes it, and commits the resulting proposal through the
// shared commit path:
//
//   * a foundation no-op task terminates successfully;
//   * an unregistered or explicitly unsupported task fails with a structured
//     diagnostic (never a silent skip);
//   * a task routed to a registered chip fails as "handler not installed"
//     under `propagate` until the host installs the real handler via
//     `propagate_with` / `clock_tick_with` (T02 owns the worker set).
//
// A commit failure never strands a task in `Running`: the shell transitions the
// task to `Failed` (with a diagnostic when capacity allows) and reports the
// structured error. New tasks are visible one tick after they are enqueued.
// ============================================================================

use cc_silicon::Bus;

use crate::bus::{CompilerBus, CompilerPins, JobState, TaggedProposal, TickMetrics, TickRecord};
use crate::commit::{commit_proposals, CommitError, CommitReport};
use crate::diagnostic::DiagnosticDraft;
use crate::ids::{ChipId, DiagnosticId, TaskId};
use crate::task::{Proposal, ResultValue, TaskKind, TaskState};

/// Chip ID reserved for the routing shell itself. It is never assigned to a
/// real worker and only appears in trace/proposal metadata.
pub const ROUTING_SHELL_CHIP: ChipId = ChipId(u16::MAX - 1);

/// Deterministic scheduling priority of a task kind: control first, then each
/// pipeline group in declaration order.
pub const fn phase_priority(kind: TaskKind) -> u16 {
    kind.group().raw() as u16
}

/// Structured routing-table failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteError {
    /// The kind is already routed.
    DuplicateKind {
        /// Offending kind.
        kind: TaskKind,
    },
    /// Foundation/reserved kinds have built-in behavior and cannot be rerouted.
    ReservedKind {
        /// Offending kind.
        kind: TaskKind,
    },
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateKind { kind } => {
                write!(f, "task kind {} already routed", kind.raw())
            }
            Self::ReservedKind { kind } => {
                write!(f, "task kind {} is reserved by the shell", kind.raw())
            }
        }
    }
}

impl std::error::Error for RouteError {}

/// One registered route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteEntry {
    /// Routed kind.
    pub kind: TaskKind,
    /// Destination chip.
    pub chip: ChipId,
    /// Destination layer.
    pub layer: u16,
}

/// A routing table. Empty by default; T02 registers worker routes. Stored in
/// the bus so it participates in snapshots and deterministic replay.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoutingTable {
    entries: Vec<RouteEntry>,
}

impl RoutingTable {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a route. Foundation and reserved-local kinds are rejected.
    pub fn register(&mut self, kind: TaskKind, chip: ChipId, layer: u16) -> Result<(), RouteError> {
        if kind.is_foundation() || kind.is_reserved_local() {
            return Err(RouteError::ReservedKind { kind });
        }
        if self.lookup(kind).is_some() {
            return Err(RouteError::DuplicateKind { kind });
        }
        let insert_at = self.entries.partition_point(|entry| entry.kind < kind);
        self.entries
            .insert(insert_at, RouteEntry { kind, chip, layer });
        Ok(())
    }

    /// Look up a route.
    pub fn lookup(&self, kind: TaskKind) -> Option<&RouteEntry> {
        self.entries.iter().find(|entry| entry.kind == kind)
    }

    /// Iterate entries in ascending kind order.
    pub fn iter(&self) -> impl Iterator<Item = &RouteEntry> {
        self.entries.iter()
    }

    /// Number of registered routes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Host-installed worker handler: given a dispatched task and a read-only
/// bus, return that task's proposals.
pub type TickHandler<'a> = dyn Fn(TaskId, &CompilerBus) -> Vec<Proposal> + 'a;

/// How the shell resolved a task kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Foundation no-op.
    Noop,
    /// Explicitly unsupported foundation probe.
    Unsupported,
    /// Routed to a registered chip (no handler installed yet).
    Registered {
        /// Destination chip.
        chip: ChipId,
        /// Destination layer.
        layer: u16,
    },
    /// No route exists.
    Unregistered,
}

/// Outcome of one propagation step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TickOutcome {
    /// No eligible task.
    Idle,
    /// The tick budget was exhausted before selecting.
    BudgetExhausted,
    /// The host cancelled the job before selecting.
    Cancelled,
    /// A task was selected and its proposal committed.
    Executed {
        /// Selected task.
        task: TaskId,
        /// Task kind.
        kind: TaskKind,
        /// How it was routed.
        resolution: Resolution,
        /// Commit report.
        commit: CommitReport,
    },
    /// The selected task's proposal failed to commit; the task was failed.
    CommitFailed {
        /// Selected task.
        task: TaskId,
        /// Structured commit error.
        error: CommitError,
        /// Committed failure diagnostic, when capacity allowed one.
        diagnostic: Option<DiagnosticId>,
    },
    /// No dispatchable task, but `Waiting` parents were join-drained
    /// (`/9` PCR-03: idle ticks still make bounded join progress instead of
    /// stranding parents forever).
    Joined {
        /// Parents reinserted to `Ready` by the await-all join.
        readied: Vec<TaskId>,
        /// Parents failed once by the await-all join.
        failed: Vec<(TaskId, DiagnosticId)>,
    },
}

/// Report of one propagation step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickReport {
    /// Tick at which the step ran.
    pub tick: u64,
    /// Selected task, if any.
    pub selected: Option<TaskId>,
    /// Outcome.
    pub outcome: TickOutcome,
}

/// The routing shell: selection, routing, proposal, and commit.
#[derive(Clone, Debug, Default)]
pub struct RoutingShell;

impl RoutingShell {
    /// Create a shell.
    pub fn new() -> Self {
        Self
    }

    /// Select the next ready task, if any, by the frozen order.
    pub fn select(&self, bus: &CompilerBus) -> Option<TaskId> {
        self.select_batch(bus, u32::MAX).into_iter().next()
    }

    /// Select up to `quota` ready tasks by the frozen order
    /// (phase priority, enqueue ordinal, TaskId).
    ///
    /// Deterministic: eligible keys are collected and sorted, never
    /// hash-iterated. Stage-ordinal keying applies once stage assignment
    /// lands (T01); the current key is the frozen T01 §4 triple.
    pub fn select_batch(&self, bus: &CompilerBus, quota: u32) -> Vec<TaskId> {
        let tick = bus.control.tick;
        let mut eligible: Vec<(u16, u64, u32, TaskId)> = Vec::new();
        for &id in &bus.tasks.ready {
            let Ok(task) = bus.arenas.tasks.get(id) else {
                continue;
            };
            if task.state != TaskState::Ready || task.ready_tick > tick {
                continue;
            }
            let key = task.schedule_key(phase_priority(task.kind));
            eligible.push((key.0, key.1, key.2, id));
        }
        eligible.sort_unstable();
        let take = (quota as usize).min(eligible.len());
        eligible
            .into_iter()
            .take(take)
            .map(|(_, _, _, id)| id)
            .collect()
    }

    /// The per-tick dispatch quota (`max_inflight_per_tick`).
    ///
    /// The configured value is at least 1 under `Limits::try_new`; a legacy
    /// struct-literal zero is clamped defensively (never a panic, never zero
    /// dispatch). Quota `> 1` paths are implemented per Group A but flagged
    /// for T13 fixtures (T13 H6-M matrix, VF02/VF03/VF04/VF13 pending).
    pub fn dispatch_quota(bus: &CompilerBus) -> u32 {
        bus.limits().max_inflight_per_tick.max(1)
    }

    /// Resolve a task kind using the routing table stored in the bus.
    pub fn resolve(&self, bus: &CompilerBus, kind: TaskKind) -> Resolution {
        if kind == TaskKind::CONTROL_NOOP {
            Resolution::Noop
        } else if kind == TaskKind::CONTROL_UNSUPPORTED {
            Resolution::Unsupported
        } else if let Some(entry) = bus.routing.lookup(kind) {
            Resolution::Registered {
                chip: entry.chip,
                layer: entry.layer,
            }
        } else {
            Resolution::Unregistered
        }
    }

    /// Run one propagation step.
    ///
    /// Cancel takes precedence over the tick budget: a cancel tick is handled
    /// once pre-selection with no dispatch and no commit. Quota-1 behavior is
    /// unchanged from `/5` (one ordered atomic commit, single-task recovery).
    pub fn propagate(&self, bus: &mut CompilerBus) -> Result<TickReport, CommitError> {
        self.propagate_inner(bus, None)
    }

    /// Run one propagation step with a host-installed worker handler.
    ///
    /// Identical to [`Self::propagate`] except tasks routed to a registered
    /// chip are driven through `handler` (which collects that chip's
    /// proposals) instead of failing as "handler not installed". Foundation
    /// no-op/unsupported/unregistered resolutions keep their frozen
    /// behavior. An empty handler vector still triggers the
    /// exactly-one-outcome `TaskNotTransitioned` fail below, never a silent
    /// skip. Quota>1 batching is not accepted here: the commit's prediction
    /// table rejects colliding future-dated references loudly.
    pub fn propagate_with<F>(
        &self,
        bus: &mut CompilerBus,
        handler: F,
    ) -> Result<TickReport, CommitError>
    where
        F: Fn(TaskId, &CompilerBus) -> Vec<Proposal>,
    {
        self.propagate_inner(bus, Some(&handler))
    }

    fn propagate_inner(
        &self,
        bus: &mut CompilerBus,
        handler: Option<&TickHandler<'_>>,
    ) -> Result<TickReport, CommitError> {
        let tick = bus.control.tick;
        bus.control.selected = None;
        bus.tasks.active = None;
        bus.wires.selected = None;

        // Cancel precedence: before any budget accounting, selection, or
        // dispatch. Idempotent: repeated cancel ticks repeat this outcome
        // without further state change.
        if bus.control.cancel_requested {
            bus.control.job_state = JobState::Failed;
            bus.control.selected = None;
            bus.tasks.active = None;
            bus.tasks.in_flight.clear();
            return Ok(TickReport {
                tick,
                selected: None,
                outcome: TickOutcome::Cancelled,
            });
        }
        if tick >= bus.limits().max_ticks {
            bus.control.budget_exhausted = true;
            return Ok(TickReport {
                tick,
                selected: None,
                outcome: TickOutcome::BudgetExhausted,
            });
        }
        if bus.control.budget_exhausted {
            return Ok(TickReport {
                tick,
                selected: None,
                outcome: TickOutcome::BudgetExhausted,
            });
        }
        let quota = Self::dispatch_quota(bus);
        let batch = self.select_batch(bus, quota);
        if batch.is_empty() {
            // `/9` PCR-03: idle ticks still drain await-all joins. Without
            // this, a `Waiting` parent whose child just `Failed` (normal
            // failure, commit recovery, or progress-limit failure) would stay
            // `Waiting` forever once no `Ready` task remains.
            match crate::commit::poll_await_joins(bus) {
                Ok(poll) if poll.readied.is_empty() && poll.failed.is_empty() => {
                    return Ok(TickReport {
                        tick,
                        selected: None,
                        outcome: TickOutcome::Idle,
                    });
                }
                Ok(poll) => {
                    return Ok(TickReport {
                        tick,
                        selected: None,
                        outcome: TickOutcome::Joined {
                            readied: poll.readied,
                            failed: poll.failed,
                        },
                    });
                }
                Err(error) => return Err(error),
            }
        }
        // Pre-dispatch guard: the selection batch never exceeds the quota via
        // `select_batch`; a larger batch is a structured pre-dispatch error
        // and leaves every affected task `Ready` (no state mutated yet).
        if batch.len() > quota as usize {
            return Err(CommitError::SelectionBatchOverflow {
                limit: quota,
                count: batch.len(),
            });
        }
        // Distinct pre-worker mutation: mark each dispatched task `Running` in
        // dispatch order and populate the ephemeral in-flight set. This is the
        // dispatcher's mutation only; the one ordered atomic semantic commit
        // happens below through `commit_proposals` (H9 relationship stays a
        // T01 decision). Reads (fallible) precede the first mutation.
        let mut kinds: Vec<(TaskId, TaskKind, ChipId)> = Vec::with_capacity(batch.len());
        for &id in &batch {
            let (kind, owner) = bus
                .arenas
                .tasks
                .get(id)
                .map(|task| (task.kind, task.owner))
                .map_err(|_| CommitError::UnknownTask { task: id })?;
            kinds.push((id, kind, owner));
        }
        for &(id, _, _) in &kinds {
            // Every selected ID came from the ready queue and read back live
            // above; the update cannot fail.
            if let Ok(record) = bus.arenas.tasks.get_mut(id) {
                record.state = TaskState::Running;
            }
            bus.tasks.in_flight.push(id);
        }
        bus.tasks.ready.retain(|id| !batch.contains(id));
        let selected = batch.first().copied();
        bus.control.selected = selected;
        bus.tasks.active = selected;
        bus.wires.selected = selected;

        let mut routed: Vec<(TaskId, TaskKind, Resolution)> = Vec::with_capacity(kinds.len());
        for &(id, kind, owner) in &kinds {
            let resolution = self.resolve(bus, kind);
            routed.push((id, kind, resolution));
            match resolution {
                Resolution::Noop => {
                    bus.wires.proposals.push(TaggedProposal {
                        chip: owner,
                        task: id,
                        proposal: Proposal::Complete {
                            task: id,
                            value: ResultValue::Empty,
                        },
                    });
                }
                Resolution::Unsupported => {
                    bus.wires.proposals.push(TaggedProposal {
                        chip: owner,
                        task: id,
                        proposal: Proposal::Fail {
                            task: id,
                            diagnostic: DiagnosticDraft::unsupported(format!(
                                "task kind `{}` ({}) is explicitly unsupported",
                                bus.kind_name(kind),
                                kind.raw()
                            )),
                        },
                    });
                }
                Resolution::Registered { chip, layer } => {
                    if let Some(run) = handler {
                        // Host-installed worker: collect its proposals tagged
                        // by the task owner for commit validation.
                        for proposal in run(id, bus) {
                            bus.wires.proposals.push(TaggedProposal {
                                chip: owner,
                                task: id,
                                proposal,
                            });
                        }
                    } else {
                        bus.wires.proposals.push(TaggedProposal {
                            chip: owner,
                            task: id,
                            proposal: Proposal::Fail {
                                task: id,
                                diagnostic: DiagnosticDraft::unsupported(format!(
                                    "task kind `{}` routed to chip {} layer {layer} has no handler installed",
                                    bus.kind_name(kind),
                                    chip.index()
                                )),
                            },
                        });
                    }
                }
                Resolution::Unregistered => {
                    bus.wires.proposals.push(TaggedProposal {
                        chip: owner,
                        task: id,
                        proposal: Proposal::Fail {
                            task: id,
                            diagnostic: DiagnosticDraft::unsupported(format!(
                                "task kind `{}` ({}) is not registered",
                                bus.kind_name(kind),
                                kind.raw()
                            )),
                        },
                    });
                }
            }
        }
        // Exactly-one-outcome rule (`/9` PCR-02): any dispatched task without
        // exactly one transition (`Complete`/`Fail`/`AwaitHost`/
        // `AwaitChildren`/`Progress`) fails with the `TaskNotTransitioned`
        // diagnostic (never silent success, never stranded `Running`). A
        // non-empty but transition-free vector (`AppendRecords`-only,
        // `Enqueue`-only, `StorePatch`-only) is the same violation as an
        // empty vector. The shell always emits one proposal per task above,
        // so this covers worker adapters and prepopulated-wire batches. The
        // failure is attributed to the task's owner chip so commit
        // attribution holds. The commit path re-checks the same rule before
        // any mutation for direct `commit_proposals` callers.
        for &id in &batch {
            let covered = bus
                .wires
                .proposals
                .iter()
                .any(|tagged| tagged.task == id && tagged.proposal.is_transition());
            if !covered {
                let owner = kinds
                    .iter()
                    .find(|(kid, _, _)| *kid == id)
                    .map(|(_, _, owner)| *owner);
                if let Some(owner) = owner {
                    bus.wires.proposals.push(TaggedProposal {
                        chip: owner,
                        task: id,
                        proposal: crate::commit::empty_proposal_fail(id),
                    });
                }
            }
        }
        let pending = std::mem::take(&mut bus.wires.proposals);
        let outcome = match commit_proposals(bus, pending) {
            Ok(commit) => {
                let outcome = self.outcome_for_batch(&routed, commit);
                bus.tasks.in_flight.clear();
                return Ok(TickReport {
                    tick,
                    selected,
                    outcome,
                });
            }
            Err(error) => {
                let diagnostics = self.fail_selected(bus, &batch, &error);
                bus.tasks.in_flight.clear();
                // Quota-1 projection preserves the `/5` outcome shape exactly;
                // the empty-batch fallback only satisfies totality (the batch
                // is non-empty on this path).
                match batch.first().copied() {
                    Some(task) => TickOutcome::CommitFailed {
                        task,
                        error,
                        diagnostic: diagnostics.into_iter().next().flatten(),
                    },
                    None => TickOutcome::Idle,
                }
            }
        };
        Ok(TickReport {
            tick,
            selected,
            outcome,
        })
    }

    /// Project a batch commit onto the quota-1 outcome shape.
    ///
    /// Quota-1 returns the full `Executed` outcome. Quota `> 1` (Group A,
    /// flagged for T13 fixtures) projects onto the first dispatched task; a
    /// multi-task outcome shape is a T01/T13 `/6` item.
    fn outcome_for_batch(
        &self,
        routed: &[(TaskId, TaskKind, Resolution)],
        commit: CommitReport,
    ) -> TickOutcome {
        // `routed` is non-empty whenever this runs (the empty batch returns
        // `Idle` before dispatch); the fallback only satisfies totality.
        match routed.first().copied() {
            Some((task, kind, resolution)) => TickOutcome::Executed {
                task,
                kind,
                resolution,
                commit,
            },
            None => TickOutcome::Idle,
        }
    }

    /// `fail_selected` generalized to bounded dispatch-order recovery (H6,
    /// Group A): after a failed atomic batch commit (which committed no
    /// semantic state), transition every dispatched task exactly once, in
    /// dispatch order, to `Failed`.
    ///
    /// Per-task diagnostic attempt with the `DiagnosticId::NONE` fallback (no
    /// pre-reservation of N diagnostics) plus the state guard: only `Running`
    /// tasks transition, so a duplicate transition is impossible. Clearing
    /// the in-flight set is not itself a transition (the caller clears after
    /// every outcome is recorded). Quota-1 behavior is identical to `/5`
    /// (single-task batch, same sentinel semantics). Quota `> 1` follows the
    /// same path; fixtures are T13 work (H6-M matrix pending).
    fn fail_selected(
        &self,
        bus: &mut CompilerBus,
        dispatched: &[TaskId],
        error: &CommitError,
    ) -> Vec<Option<DiagnosticId>> {
        let mut diagnostics = Vec::with_capacity(dispatched.len());
        for &task in dispatched {
            let draft = error.to_diagnostic().with_task(task);
            diagnostics.push(crate::commit::try_fail_task(bus, task, draft));
        }
        bus.control.selected = None;
        bus.tasks.active = None;
        diagnostics
    }

    /// Run a full clock cycle: reset wires, apply pins, propagate, latch,
    /// advance.
    ///
    /// Exactly one [`TickRecord`] is appended to the canonical bounded
    /// `bus.report` per tick (the driver step-6 sole-append-site rule; this
    /// shell acts as that driver until `report.rs`/`driver.rs` land). A full
    /// report therefore also bounds how far past the tick budget the shell
    /// can be driven: further records saturate deterministically at
    /// `max_ticks + 1` while the tick outcome itself is still reported.
    pub fn clock_tick(
        &self,
        pins: &CompilerPins,
        bus: &mut CompilerBus,
    ) -> Result<TickReport, CommitError> {
        bus.reset_wires();
        if pins.tick_budget_reached {
            bus.control.budget_exhausted = true;
        }
        if pins.cancel {
            bus.control.cancel_requested = true;
            bus.control.job_state = JobState::Failed;
            bus.control.selected = None;
            bus.tasks.active = None;
            bus.tasks.in_flight.clear();
            let report = TickReport {
                tick: bus.control.tick,
                selected: None,
                outcome: TickOutcome::Cancelled,
            };
            self.record_tick(bus, &report);
            bus.latch(pins);
            bus.advance_tick();
            return Ok(report);
        }
        let report = self.propagate(bus);
        if let Ok(report) = &report {
            self.record_tick(bus, report);
        }
        bus.latch(pins);
        bus.advance_tick();
        report
    }

    /// Run a full clock cycle with a host-installed worker handler.
    ///
    /// Same tick lifecycle as [`Self::clock_tick`] (reset → propagate →
    /// latch/advance with exactly one tick record); the propagate step uses
    /// [`Self::propagate_with`].
    pub fn clock_tick_with<F>(
        &self,
        pins: &CompilerPins,
        bus: &mut CompilerBus,
        handler: F,
    ) -> Result<TickReport, CommitError>
    where
        F: Fn(TaskId, &CompilerBus) -> Vec<Proposal>,
    {
        bus.reset_wires();
        if pins.tick_budget_reached {
            bus.control.budget_exhausted = true;
        }
        if pins.cancel {
            bus.control.cancel_requested = true;
            bus.control.job_state = JobState::Failed;
            bus.control.selected = None;
            bus.tasks.active = None;
            bus.tasks.in_flight.clear();
            let report = TickReport {
                tick: bus.control.tick,
                selected: None,
                outcome: TickOutcome::Cancelled,
            };
            self.record_tick(bus, &report);
            bus.latch(pins);
            bus.advance_tick();
            return Ok(report);
        }
        let report = self.propagate_with(bus, handler);
        if let Ok(report) = &report {
            self.record_tick(bus, report);
        }
        bus.latch(pins);
        bus.advance_tick();
        report
    }

    /// Append the tick's canonical record (driver step 6).
    fn record_tick(&self, bus: &mut CompilerBus, report: &TickReport) {
        // Canonical dispatched set: quota-1 projects onto `selected`; the
        // multi-dispatch list is a T01/T13 `/6` item (see `outcome_for_batch`).
        let dispatched: Vec<TaskId> = report.selected.into_iter().collect();
        let metrics = TickMetrics {
            dispatched: dispatched.len() as u32,
        };
        let _ = bus.push_tick_record(TickRecord {
            dispatched,
            metrics,
            selected: report.selected,
        });
        // Saturation past `max_ticks + 1` is a documented deterministic
        // truncation (first records retained); the tick outcome above is
        // still reported to the caller.
    }
}
