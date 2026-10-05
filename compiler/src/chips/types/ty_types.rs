// ============================================================================
// chips/ty_types.rs — T06 TY13/TY17 canonical-type workers
// (Wave 2 slice 4, `/13`)
//
// `TyTypeChip` serves two kinds: `symbol_type.int_type` (empty payload;
// the canonical `int` singleton) and `symbol_type.func_type` (payload:
// exactly one result `Type`; the `int(void)` shape). Both reuse the
// committed canonical id through a deterministic bounded scan (lowest
// matching `TypeId`, no cache) and append only on a miss. Non-M1 type
// shapes are explicit `Unsupported`.
// ============================================================================

use crate::bus::{IntRank, TypeKind, TypeRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{RecordRef, TaskId, TypeId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, TY_TYPE_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// The M1 canonical `int`: `Int { rank: Int, signed: true }`, symbolic.
pub fn m1_int() -> TypeRecord {
    TypeRecord {
        kind: TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        },
    }
}

/// Narrow projection for the type-producer computation.
#[derive(Clone, Debug)]
pub struct TyTypeInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Task kind.
    pub kind: TaskKind,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Result type for `func_type` (committed body).
    pub result: Option<(TypeId, TypeRecord)>,
    /// Committed canonical id matching the wanted type, if any.
    pub existing: Option<TypeId>,
    /// `types` arena count at dispatch (append prediction base).
    pub types_allocated: u32,
}

/// Deterministic bounded reuse scan: lowest committed `TypeId` whose record
/// structurally equals `wanted` (no hidden cache/index).
pub fn canonical_scan(bus: &crate::bus::CompilerBus, wanted: &TypeRecord) -> Option<TypeId> {
    bus.arenas
        .types
        .iter()
        .find(|(_, record)| *record == wanted)
        .map(|(id, _)| id)
}

/// Build the narrow projection for one dispatched task.
pub fn project_ty_type_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<TyTypeInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("type of unknown task {}", task.index())))?;
    let (wanted, result) = match record.kind {
        TaskKind::SYMBOL_INT_TYPE => {
            if !record.payload.refs.is_empty() {
                return Err(protocol_fault(format!(
                    "int_type task {} payload must be empty",
                    task.index()
                )));
            }
            (m1_int(), None)
        }
        TaskKind::SYMBOL_FUNC_TYPE => {
            if record.payload.refs.len() != 1 {
                return Err(protocol_fault(format!(
                    "func_type task {} payload must carry exactly one result type",
                    task.index()
                )));
            }
            let result_id = match record.payload.refs[0] {
                RecordRef::Type(id) => id,
                _ => {
                    return Err(protocol_fault(format!(
                        "func_type task {} payload must be a type",
                        task.index()
                    )));
                }
            };
            let result_body = bus.arenas.types.get(result_id).map_err(|_| {
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("func_type reads missing type {}", result_id.index()),
                )
            })?;
            if result_body.kind
                != (TypeKind::Int {
                    rank: IntRank::Int,
                    signed: true,
                })
            {
                return Err(DiagnosticDraft::unsupported(
                    "non-int function result is outside the M1 exercised subset",
                ));
            }
            (
                TypeRecord {
                    kind: TypeKind::Function {
                        result: result_id,
                        params: vec![],
                        prototype: true,
                        variadic: false,
                    },
                },
                Some((result_id, result_body.clone())),
            )
        }
        _ => {
            return Err(protocol_fault(format!(
                "type task {} has unexpected kind {}",
                task.index(),
                record.kind.raw()
            )));
        }
    };
    let existing = canonical_scan(bus, &wanted);
    Ok(TyTypeInput {
        task,
        kind: record.kind,
        state: record.state.clone(),
        result,
        existing,
        types_allocated: bus.arenas.types.allocated(),
    })
}

/// The T06 canonical-type worker (TY13/TY17 slice scope).
pub struct TyTypeChip;

impl Worker for TyTypeChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: TY_TYPE_CHIP,
            chip_name: "TyTypeChip",
            group: TaskGroup::SYMBOL_TYPE,
            task_kinds: vec![TaskKind::SYMBOL_INT_TYPE, TaskKind::SYMBOL_FUNC_TYPE],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Types, "records"),
            ],
            writes: vec![FieldPath::new(StoreId::Types, "records")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c13_ty.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_ty_type_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl TyTypeChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &TyTypeInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("type task {} is not running", input.task.index()),
                ),
            )];
        }
        if let Some(existing) = input.existing {
            // Canonical reuse: no duplicate appended.
            return vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Type(existing)),
            }];
        }
        let wanted = match (input.kind, &input.result) {
            (TaskKind::SYMBOL_INT_TYPE, _) => m1_int(),
            (TaskKind::SYMBOL_FUNC_TYPE, Some((result_id, _))) => TypeRecord {
                kind: TypeKind::Function {
                    result: *result_id,
                    params: vec![],
                    prototype: true,
                    variadic: false,
                },
            },
            _ => {
                return vec![fail(
                    input.task,
                    protocol_fault("type task kind/result mismatch"),
                )];
            }
        };
        let predicted = TypeId::from_index(input.types_allocated);
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: crate::ids::RecordFamily::Type,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::Type(wanted)],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Type(predicted)),
            },
        ]
    }
}
