// ============================================================================
// chips/ty_conv.rs — T06 TY25/TY26/TY27 identity-conversion workers
// (Wave 2 slice 4, `/13`)
//
// `TyConvChip` serves `symbol_type.promote`, `symbol_type.common_type`, and
// `symbol_type.return_convert` for the M1 identity-only corner: every operand
// is already `Int`, so each chip completes with the unchanged committed
// `TypeId` and appends nothing. No `ConversionPlan` is recorded at M1
// (identity/no-conversion rule; the shared plan type freezes with the T07
// `SemRecord`). Non-`Int` operands are explicit `Unsupported` (non-identity
// promotions stay deferred per rev 46 / OB-51).
// ============================================================================

use crate::chips::{fail, protocol_fault, Worker};
use crate::bus::{IntRank, TypeKind};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{RecordRef, TaskId, TypeId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, TY_CONV_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Whether a committed type is the M1 `Int` identity corner.
pub fn is_m1_int(kind: &TypeKind) -> bool {
    *kind
        == (TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        })
}

/// Narrow projection for the identity-conversion computation.
#[derive(Clone, Debug)]
pub struct TyConvInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Task kind.
    pub kind: TaskKind,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Operand `(id, kind)` pairs in payload order.
    pub operands: Vec<(TypeId, TypeKind)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_ty_conv_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<TyConvInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("convert of unknown task {}", task.index())))?;
    let arity = match record.kind {
        TaskKind::SYMBOL_PROMOTE => 1,
        TaskKind::SYMBOL_COMMON_TYPE | TaskKind::SYMBOL_RETURN_CONVERT => 2,
        _ => {
            return Err(protocol_fault(format!(
                "convert task {} has unexpected kind {}",
                task.index(),
                record.kind.raw()
            )));
        }
    };
    if record.payload.refs.len() != arity {
        return Err(protocol_fault(format!(
            "convert task {} payload must carry exactly {arity} types",
            task.index()
        )));
    }
    let mut operands = Vec::new();
    for reference in &record.payload.refs {
        let RecordRef::Type(id) = reference else {
            return Err(protocol_fault(format!(
                "convert task {} payload must carry types only",
                task.index()
            )));
        };
        if bus.arenas.types.get(*id).is_err() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("convert reads missing type {}", id.index()),
            ));
        }
        operands.push(*id);
    }
    // Resolve bodies for the identity check (read-only; kinds only).
    let mut pairs = Vec::new();
    for id in &operands {
        let kind = bus
            .arenas
            .types
            .get(*id)
            .map(|body| body.kind.clone())
            .map_err(|_| {
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("convert reads missing type {}", id.index()),
                )
            })?;
        pairs.push((*id, kind));
    }
    Ok(TyConvInput {
        task,
        kind: record.kind,
        state: record.state.clone(),
        operands: pairs,
    })
}

/// The T06 identity-conversion worker (TY25/TY26/TY27 M1 corner).
pub struct TyConvChip;

impl Worker for TyConvChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: TY_CONV_CHIP,
            chip_name: "TyConvChip",
            group: TaskGroup::SYMBOL_TYPE,
            task_kinds: vec![
                TaskKind::SYMBOL_PROMOTE,
                TaskKind::SYMBOL_COMMON_TYPE,
                TaskKind::SYMBOL_RETURN_CONVERT,
            ],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Types, "records"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c13_ty.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_ty_conv_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl TyConvChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &TyConvInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("convert task {} is not running", input.task.index()),
                ),
            )];
        }
        let arity = match input.kind {
            TaskKind::SYMBOL_PROMOTE => 1,
            TaskKind::SYMBOL_COMMON_TYPE | TaskKind::SYMBOL_RETURN_CONVERT => 2,
            _ => {
                return vec![fail(
                    input.task,
                    protocol_fault("convert task kind/result mismatch"),
                )];
            }
        };
        if input.operands.len() != arity || !input.operands.iter().all(|(_, kind)| is_m1_int(kind))
        {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("non-identity conversion is deferred past M1"),
            )];
        }
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Record(RecordRef::Type(input.operands[0].0)),
        }]
    }
}
