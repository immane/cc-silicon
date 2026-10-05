// ============================================================================
// chips/mod.rs — host-driven worker chips and driver (Wave 1 template)
//
// The routing shell never invokes workers directly: it resolves task kinds
// and the *host* runs the responsible chip through `propagate_with` /
// `clock_tick_with`. A worker is a stateless unit that computes from a
// narrow projection plus its task and returns proposals; a host driver
// resolves the task kind, finds the worker by chip ID, collects its
// proposals, and hands the batch to the commit path. All worker I/O crosses
// this boundary as `Proposal` values — workers never mutate the bus, never
// perform Host I/O, and never call other workers.
//
// Template rules (every later chip copies its group file's shape):
//
// * One file per chip under `chips/<group>/`; the file owns the worker
//   struct, its `Worker` impl, its narrow `Input` projection plus projector,
//   its pure `compute(&Input)`, and chip-local helpers. The computation
//   never takes the full bus; the adapter owns the projection. Group layout
//   mirrors the owning task group (`preprocess`, `lex`, `parse`, `types`,
//   `symbols`, `constant_layout_init`, …); the integrator owns every
//   `mod.rs` and no chip file is ever written by two owners.
// * `manifest()` is the chip's exact C04 declaration (frozen kinds, exact
//   reads/writes, phase, capability). The manifest registers cleanly or the
//   chip does not exist.
// * `handle()` is the adapter: project then compute. Every failure path is
//   a `Fail` proposal with a structured diagnostic — never a panic, never
//   silence.
// * Predicted record IDs follow the frozen rule: your Nth body of family F
//   in your batch gets `arena.allocated() + (F-bodies applied earlier in
//   the batch)`. Quota-1 single-append workers predict `allocated() + 0`.
//   The commit verifies every future-dated `Complete` reference (`Record`
//   and `Records` carriers alike) against its prediction table
//   (`CommitError::UnpredictedRecord`); a misprediction rejects loudly,
//   never completes with a wrong reference. Quota>1 batching is not
//   accepted: colliding predictions fail loudly, they do not alias.
// ============================================================================

pub mod constant_layout_init;
pub mod ir_lower;
pub mod lex;
pub mod parse;
pub mod preprocess;
pub mod semantic;
pub mod symbols;
pub mod types;
pub mod verify;

pub use self::constant_layout_init::{
    const_bits_required, project_fold_input, FoldChip, FoldInput,
};
pub use self::ir_lower::{project_ir_function_input, IrFunctionChip, IrFunctionInput};
pub use self::lex::{
    decimal_magnitude, is_keyword, project_lx_classify_input, project_lx_decode_input,
    project_lx_intern_input, LxClassifyChip, LxClassifyInput, LxDecodeInput, LxDecodeLiteralChip,
    LxInternChip, LxInternInput, C11_KEYWORDS,
};
pub use self::parse::{project_pa_tu_input, PaTuChip, PaTuInput};
pub use self::preprocess::{
    classify_directive_params, classify_operator_text, compose_map, decode_pragma_string,
    line_location, normalize, origin_chain, origin_root, project_pp_builtin_input,
    project_pp_comment_input, project_pp_expand_map_input, project_pp_input, project_pp_line_input,
    project_pp_pragma_input, project_pp_scan_input, project_pp_splice_input, replace_comments,
    scan, splice, LogicalLocation, OriginChain, OriginFrame, PpBuiltinChip, PpBuiltinInput,
    PpCommentChip, PpCommentInput, PpConditionalChip, PpConditionalInput, PpDefineChip,
    PpDefineInput, PpDiagnosticChip, PpDiagnosticInput, PpDirectiveChip, PpDirectiveInput,
    PpExpandMapChip, PpExpandMapInput, PpIncludeEnterChip, PpIncludeEnterInput,
    PpIncludeResolveChip, PpIncludeResolveInput, PpInput, PpInvokeChip, PpInvokeInput, PpLineChip,
    PpLineInput, PpNormalizeChip, PpPragmaChip, PpPragmaInput, PpRedefineChip, PpRedefineInput,
    PpScanChip, PpScanInput, PpSpliceChip, PpSpliceInput, PpSubstituteChip, PpSubstituteInput,
    PpUndefChip, PpUndefInput, PpVariadicChip, PpVariadicInput, PpVariadicMode, PragmaClass,
    PragmaStringError, ScannedToken, M1_PUNCTUATORS, PP23_TASK_KIND, PP25_TASK_KIND,
    PP27_TASK_KIND, PP_LINE_MAX,
};
pub use self::semantic::{
    project_se_binary_input, project_se_literal_input, project_se_return_input, SeBinChip,
    SeBinaryInput, SeLitChip, SeLiteralInput, SeRetChip, SeReturnInput,
};
pub use self::symbols::{
    in_ordinary_namespace, project_ty_declare_input, project_ty_lookup_input,
    project_ty_scope_enter_input, project_ty_scope_exit_input, TyScopeChip, TyScopeEnterInput,
    TyScopeExitInput, TySymbolChip,
};
pub use self::types::{
    canonical_scan, is_m1_int, m1_int, project_ty_conv_input, project_ty_type_input, TyConvChip,
    TyConvInput, TyTypeChip, TyTypeInput,
};
pub use self::verify::{
    project_vf01_input, project_vf05_input, project_vf06_input, project_vf12_input, Vf01Chip,
    Vf01Input, Vf05Chip, Vf05Input, Vf06Chip, Vf06Input, Vf12Chip, Vf12Input,
};

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
    ///
    /// `/9` PCR-10: the worker must be a zero-sized unit struct. A
    /// field-bearing worker is rejected instead of running with hidden state.
    pub fn register<W: Worker + 'static>(&mut self, worker: W) -> Result<(), DriveError> {
        let id = worker.chip_id();
        if std::mem::size_of::<W>() != 0 {
            return Err(DriveError::NonStatelessWorker { chip: id });
        }
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
    /// The routed chip has no registered manifest (commit would reject its
    /// appends; the driver refuses earlier).
    UnregisteredChip {
        /// Chip with no manifest.
        chip: ChipId,
    },
    /// The worker is not a zero-sized unit struct (`/9` PCR-10: chips are
    /// stateless; a field-bearing worker would hide semantic state).
    NonStatelessWorker {
        /// Chip with state.
        chip: ChipId,
    },
    /// The routed layer disagrees with the frozen stage assignment
    /// (`/9` PCR-09: the driver enforces `check_stage_layer_agreement`
    /// instead of leaving it as a standalone helper).
    StageLayerMismatch {
        /// Task kind.
        kind: TaskKind,
        /// Frozen stage.
        stage: u16,
        /// Routed layer.
        layer: u16,
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
            Self::UnregisteredChip { chip } => {
                write!(f, "chip {} has no registered manifest", chip.index())
            }
            Self::NonStatelessWorker { chip } => {
                write!(
                    f,
                    "chip {} worker must be a zero-sized unit struct",
                    chip.index()
                )
            }
            Self::StageLayerMismatch { kind, stage, layer } => {
                write!(
                    f,
                    "task kind {} stage {stage} disagrees with routed layer {layer}",
                    kind.raw()
                )
            }
        }
    }
}

impl std::error::Error for DriveError {}

/// Drive one task through its registered worker, collecting tagged
/// proposals for the commit path.
///
/// The driver checks identity only (task exists, kind routed, worker
/// present and matching, kind claimed, owner equals chip, chip manifest
/// registered, routed layer agrees with the frozen stage). All semantic
/// decisions belong to the worker's proposals and the commit's validation.
/// Non-`Registered` resolutions are driver errors, never synthesized
/// executions.
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
    let (chip, layer) = match shell.resolve(bus, kind) {
        crate::routing::Resolution::Registered { chip, layer } => (chip, layer),
        _ => return Err(DriveError::NotRegistered { kind }),
    };
    // `/9` PCR-09: enforce stage/layer agreement on the execution path.
    if let Some(stage) = crate::manifest::stage_of(kind) {
        if layer != stage as u16 {
            return Err(DriveError::StageLayerMismatch {
                kind,
                stage: stage as u16,
                layer,
            });
        }
    }
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
    if bus.registrations.get(chip).is_none() {
        return Err(DriveError::UnregisteredChip { chip });
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

/// Build a tick-handler closure over a worker registry for
/// `RoutingShell::propagate_with` / `clock_tick_with`.
///
/// Lookup failures become loud `Fail` proposals (never silent skips, never
/// fabricated successes); the commit then attributes and validates them.
pub fn handler_for<'a>(
    workers: &'a WorkerRegistry,
) -> impl Fn(TaskId, &CompilerBus) -> Vec<Proposal> + 'a {
    move |task: TaskId, bus: &CompilerBus| {
        let (kind, owner) = match bus.arenas.tasks.get(task) {
            Ok(record) => (record.kind, record.owner),
            Err(_) => {
                return vec![fail(
                    task,
                    protocol_fault(format!("tick handler for unknown task {}", task.index())),
                )];
            }
        };
        let chip = match RoutingShell::new().resolve(bus, kind) {
            crate::routing::Resolution::Registered { chip, layer } => {
                // `/9` PCR-09: the tick handler enforces the same
                // stage/layer agreement as `drive_task`.
                if let Some(stage) = crate::manifest::stage_of(kind) {
                    if layer != stage as u16 {
                        return vec![fail(
                            task,
                            protocol_fault(format!(
                                "tick handler: kind {} stage {stage} disagrees with layer {layer}",
                                kind.raw()
                            )),
                        )];
                    }
                }
                chip
            }
            _ => {
                return vec![fail(
                    task,
                    protocol_fault(format!("tick handler: kind {} has no route", kind.raw())),
                )];
            }
        };
        let Some(worker) = workers.get(chip) else {
            return vec![fail(
                task,
                protocol_fault(format!("tick handler: chip {} has no worker", chip.index())),
            )];
        };
        if worker.chip_id() != chip
            || !worker.manifest().task_kinds.contains(&kind)
            || owner != chip
            || bus.registrations.get(chip).is_none()
        {
            return vec![fail(
                task,
                protocol_fault("tick handler: worker/owner/registration mismatch"),
            )];
        }
        worker.handle(task, bus)
    }
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
