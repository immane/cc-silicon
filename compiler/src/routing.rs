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
//     until T02 installs the real handlers.
//
// A commit failure never strands a task in `Running`: the shell transitions the
// task to `Failed` (with a diagnostic when capacity allows) and reports the
// structured error. New tasks are visible one tick after they are enqueued.
// ============================================================================

use cc_silicon::Bus;

use crate::bus::{CompilerBus, CompilerPins, JobState, TaggedProposal};
use crate::commit::{commit_proposals, CommitError, CommitReport};
use crate::diagnostic::{DiagnosticDraft, DiagnosticRecord};
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
        let tick = bus.control.tick;
        let mut best: Option<(u16, u64, u32, TaskId)> = None;
        for &id in &bus.tasks.ready {
            let Ok(task) = bus.arenas.tasks.get(id) else {
                continue;
            };
            if task.state != TaskState::Ready || task.ready_tick > tick {
                continue;
            }
            let key = task.schedule_key(phase_priority(task.kind));
            let candidate = (key.0, key.1, key.2, id);
            if best.is_none_or(|current| candidate < current) {
                best = Some(candidate);
            }
        }
        best.map(|(_, _, _, id)| id)
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
    pub fn propagate(&self, bus: &mut CompilerBus) -> Result<TickReport, CommitError> {
        let tick = bus.control.tick;
        bus.control.selected = None;
        bus.tasks.active = None;
        bus.wires.selected = None;

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
        let Some(selected) = self.select(bus) else {
            return Ok(TickReport {
                tick,
                selected: None,
                outcome: TickOutcome::Idle,
            });
        };
        // Read first (the only fallible step); nothing semantic is mutated
        // before this succeeds.
        let (kind, owner) = bus
            .arenas
            .tasks
            .get(selected)
            .map(|task| (task.kind, task.owner))
            .map_err(|_| CommitError::UnknownTask { task: selected })?;
        // The task is known live (it came from the ready queue); this cannot
        // fail, so use a non-fallible update and begin mutation only now.
        if let Ok(record) = bus.arenas.tasks.get_mut(selected) {
            record.state = TaskState::Running;
        }
        bus.control.selected = Some(selected);
        bus.tasks.active = Some(selected);
        bus.tasks.ready.retain(|id| *id != selected);
        bus.wires.selected = Some(selected);

        let resolution = self.resolve(bus, kind);
        let proposal = match resolution {
            Resolution::Noop => Proposal::Complete {
                task: selected,
                value: ResultValue::Empty,
            },
            Resolution::Unsupported => Proposal::Fail {
                task: selected,
                diagnostic: DiagnosticDraft::unsupported(format!(
                    "task kind `{}` ({}) is explicitly unsupported",
                    bus.kind_name(kind),
                    kind.raw()
                )),
            },
            Resolution::Registered { chip, layer } => Proposal::Fail {
                task: selected,
                diagnostic: DiagnosticDraft::unsupported(format!(
                    "task kind `{}` routed to chip {} layer {layer} has no handler installed",
                    bus.kind_name(kind),
                    chip.index()
                )),
            },
            Resolution::Unregistered => Proposal::Fail {
                task: selected,
                diagnostic: DiagnosticDraft::unsupported(format!(
                    "task kind `{}` ({}) is not registered",
                    bus.kind_name(kind),
                    kind.raw()
                )),
            },
        };
        // A chip may only act on a task it owns.
        bus.wires.proposals.push(TaggedProposal {
            chip: owner,
            task: selected,
            proposal,
        });
        let pending = std::mem::take(&mut bus.wires.proposals);
        match commit_proposals(bus, pending) {
            Ok(commit) => Ok(TickReport {
                tick,
                selected: Some(selected),
                outcome: TickOutcome::Executed {
                    task: selected,
                    kind,
                    resolution,
                    commit,
                },
            }),
            Err(error) => {
                let diagnostic = self.fail_selected(bus, selected, &error);
                Ok(TickReport {
                    tick,
                    selected: Some(selected),
                    outcome: TickOutcome::CommitFailed {
                        task: selected,
                        error,
                        diagnostic,
                    },
                })
            }
        }
    }

    /// Transition a stranded task to `Failed`, allocating a diagnostic when
    /// the budget allows. Returns the diagnostic ID when one was committed.
    fn fail_selected(
        &self,
        bus: &mut CompilerBus,
        task: TaskId,
        error: &CommitError,
    ) -> Option<DiagnosticId> {
        let ordinal = bus.control.next_diagnostic_ordinal;
        let limits = bus.limits();
        let record = DiagnosticRecord::commit(ordinal, error.to_diagnostic().with_task(task));
        // The recovery diagnostic must respect the same budget as any other
        // record; if it cannot be committed, use the sentinel so the task is
        // still not stranded.
        let diagnostic = if bus.ensure_total_records(1).is_ok() && bus.ensure_diagnostics(1).is_ok()
        {
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
        let state = match diagnostic {
            Some(id) => TaskState::Failed(id),
            None => TaskState::Failed(DiagnosticId::NONE),
        };
        if let Ok(task) = bus.arenas.tasks.get_mut(task) {
            task.state = state;
        }
        bus.control.selected = None;
        bus.tasks.active = None;
        diagnostic
    }

    /// Run a full clock cycle: reset wires, apply pins, propagate, latch,
    /// advance.
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
            let report = TickReport {
                tick: bus.control.tick,
                selected: None,
                outcome: TickOutcome::Cancelled,
            };
            bus.latch(pins);
            bus.advance_tick();
            return Ok(report);
        }
        let report = self.propagate(bus);
        bus.latch(pins);
        bus.advance_tick();
        report
    }
}
