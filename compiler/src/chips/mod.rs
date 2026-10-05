// ============================================================================
// chips/mod.rs — host-driven worker chips and driver (Wave 1 template)
//
// The routing shell never invokes workers: it resolves task kinds and the
// *host* runs the responsible chip. A worker is a stateless unit that reads
// the frozen bus plus its task and returns proposals; a host driver resolves
// the task kind, finds the worker by chip ID, collects its proposals, and
// hands the batch to the commit path. All worker I/O crosses this boundary
// as `Proposal` values — workers never mutate the bus, never perform Host
// I/O, and never call other workers.
//
// Template rules (every later chip copies this file's shape):
//
// * One file per chip under `chips/`; the file owns the worker struct, its
//   `Worker` impl, and its chip-local helpers. Nothing else.
// * `manifest()` is the chip's exact C04 declaration (frozen kinds, exact
//   reads/writes, phase, capability). The manifest registers cleanly or the
//   chip does not exist.
// * `handle()` returns proposals only. Every failure path is a `Fail`
//   proposal with a structured diagnostic — never a panic, never silence.
// * Predicted record IDs follow the frozen rule: your Nth body of family F
//   in your batch gets `arena.allocated() + (F-bodies applied earlier in
//   the batch)`. Quota-1 single-append workers predict `allocated() + 0`.
//   The commit verifies every future-dated `Complete` reference against its
//   prediction table (`CommitError::UnpredictedRecord`); a misprediction
//   rejects loudly, never completes with a wrong reference.
// ============================================================================

mod fold;

pub use fold::FoldChip;

use crate::bus::{CompilerBus, TaggedProposal};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, TaskId};
use crate::manifest::ChipManifest;
use crate::routing::RoutingShell;
use crate::task::{Proposal, TaskKind};

/// A host-driven worker chip: one frozen task family, read-only bus.
pub trait Worker {
    /// This chip's exact C04 manifest declaration.
    fn manifest(&self) -> ChipManifest;

    /// The chip ID this worker serves (defaults to the manifest ID).
    fn chip_id(&self) -> ChipId {
        self.manifest().id
    }

    /// Handle one task against a read-only bus snapshot, returning the
    /// proposals to commit. Every path — including all failure paths —
    /// yields proposals; workers never mutate.
    fn handle(&self, task: TaskId, bus: &CompilerBus) -> Vec<Proposal>;
}

/// Host-side registry mapping chip IDs to worker implementations.
#[derive(Default)]
pub struct WorkerRegistry {
    /// Workers in registration order (never iterated for decisions).
    workers: Vec<(ChipId, Box<dyn Worker>)>,
}

impl WorkerRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a worker. A duplicate chip ID is rejected.
    pub fn register<W: Worker + 'static>(&mut self, worker: W) -> Result<(), DriveError> {
        let id = worker.chip_id();
        if self.workers.iter().any(|(other, _)| *other == id) {
            return Err(DriveError::DuplicateWorker { chip: id });
        }
        self.workers.push((id, Box::new(worker)));
        Ok(())
    }

    /// Look up the worker serving a chip ID.
    pub fn get(&self, chip: ChipId) -> Option<&dyn Worker> {
        self.workers
            .iter()
            .find(|(id, _)| *id == chip)
            .map(|(_, worker)| worker.as_ref())
    }
}

/// Host-driver failure (the driver refuses to fabricate proposals).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriveError {
    /// The task does not exist.
    UnknownTask {
        /// Offending task.
        task: TaskId,
    },
    /// The task kind has no registered route.
    NotRegistered {
        /// Offending kind.
        kind: TaskKind,
    },
    /// The route exists but no worker serves the chip.
    NoWorker {
        /// Chip with no worker.
        chip: ChipId,
    },
    /// Two workers claim one chip ID.
    DuplicateWorker {
        /// Contended chip.
        chip: ChipId,
    },
    /// The worker serves a different chip than routed.
    WorkerMismatch {
        /// Routed chip.
        chip: ChipId,
    },
    /// The worker does not claim the task kind.
    KindNotClaimed {
        /// Task kind.
        kind: TaskKind,
    },
    /// The task owner is not the routed chip.
    OwnerMismatch {
        /// Task owner.
        owner: ChipId,
    },
}

impl std::fmt::Display for DriveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTask { task } => write!(f, "unknown task {}", task.index()),
            Self::NotRegistered { kind } => {
                write!(f, "task kind {} has no route", kind.raw())
            }
            Self::NoWorker { chip } => write!(f, "chip {} has no worker", chip.index()),
            Self::DuplicateWorker { chip } => {
                write!(f, "chip {} has two workers", chip.index())
            }
            Self::WorkerMismatch { chip } => {
                write!(
                    f,
                    "worker serves a different chip than routed {}",
                    chip.index()
                )
            }
            Self::KindNotClaimed { kind } => {
                write!(f, "worker does not claim task kind {}", kind.raw())
            }
            Self::OwnerMismatch { owner } => {
                write!(f, "task owner {} is not the routed chip", owner.index())
            }
        }
    }
}

impl std::error::Error for DriveError {}

/// Drive one task through its registered worker, collecting tagged
/// proposals for the commit path.
///
/// The driver checks identity only (task exists, kind routed, worker
/// present and matching, kind claimed, owner equals chip). All semantic
/// decisions belong to the worker's proposals and the commit's
/// validation. Non-`Registered` resolutions are driver errors, never
/// synthesized executions.
pub fn drive_task(
    bus: &CompilerBus,
    task: TaskId,
    workers: &WorkerRegistry,
) -> Result<Vec<TaggedProposal>, DriveError> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| DriveError::UnknownTask { task })?;
    let (kind, owner) = (record.kind, record.owner);
    let shell = RoutingShell::new();
    let (chip, _layer) = match shell.resolve(bus, kind) {
        crate::routing::Resolution::Registered { chip, layer } => (chip, layer),
        _ => return Err(DriveError::NotRegistered { kind }),
    };
    let worker = workers.get(chip).ok_or(DriveError::NoWorker { chip })?;
    if worker.chip_id() != chip {
        return Err(DriveError::WorkerMismatch { chip });
    }
    if !worker.manifest().task_kinds.contains(&kind) {
        return Err(DriveError::KindNotClaimed { kind });
    }
    if owner != chip {
        return Err(DriveError::OwnerMismatch { owner });
    }
    Ok(worker
        .handle(task, bus)
        .into_iter()
        .map(|proposal| TaggedProposal {
            chip: owner,
            task,
            proposal,
        })
        .collect())
}

/// Build a structured `Fail` proposal for a worker-side fault.
pub fn fail(task: TaskId, diagnostic: DiagnosticDraft) -> Proposal {
    Proposal::Fail { task, diagnostic }
}

/// Worker-side protocol fault helper (worker or driver input broke a
/// frozen convention).
pub fn protocol_fault(message: impl Into<String>) -> DiagnosticDraft {
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Protocol, 1), message)
}
