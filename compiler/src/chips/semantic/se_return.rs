// ============================================================================
// chips/semantic/se_return.rs — T07 SE21 return-statement worker
// (Wave 2 slice 5, `/14`)
//
// Reads one committed `Return` node, checks its single `BinaryAdd` operand
// (which needs a committed `int`-typed `SemRecord`), and appends one
// `SemRecord` carrying the function return type. M1 is the `int`-to-`int`
// identity corner: a type mismatch fails loudly instead of recording a
// conversion plan (plans arrive with non-identity conversions).
// ============================================================================

use super::se_literal::{find_m1_int, sem_for_node};
use crate::bus::{EffectMask, NodeKind, SemRecord, ValueCategory};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NodeId, RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, SE_RET_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the return-statement check.
#[derive(Clone, Debug)]
pub struct SeReturnInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Checked node.
    pub node: NodeId,
    /// Function return type (`int` in M1).
    pub return_ty: crate::ids::TypeId,
    /// Committed `SemRecord` for the node, if any.
    pub existing: Option<(crate::ids::SemId, SemRecord)>,
    /// `sem` arena count at dispatch (prediction base).
    pub sem_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_se_return_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<SeReturnInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("se-return of unknown task {}", task.index())))?;
    if record.kind != TaskKind::SEMANTIC_RETURN_STMT {
        return Err(protocol_fault(format!(
            "se-return task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "se-return task {} payload must carry exactly one node",
            task.index()
        )));
    }
    let node = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "se-return task {} payload must be a node",
                task.index()
            )));
        }
    };
    let node_body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-return reads missing node {}", node.index()),
        )
    })?;
    if node_body.kind != NodeKind::Return {
        return Err(DiagnosticDraft::unsupported(
            "se-return of a non-return node",
        ));
    }
    if node_body.children.len() != 1 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-return node {} must have one operand", node.index()),
        ));
    }
    let operand = node_body.children[0];
    let operand_body = bus.arenas.nodes.get(operand).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-return reads missing operand {}", operand.index()),
        )
    })?;
    if operand_body.kind != NodeKind::BinaryAdd {
        return Err(DiagnosticDraft::unsupported(
            "se-return of a non-additive operand",
        ));
    }
    // The operand needs its committed `int`-typed `SemRecord`.
    let int = find_m1_int(bus)?;
    let operand_ty = match sem_for_node(bus, operand) {
        Some((_, sem)) if sem.ty == int => sem.ty,
        Some(_) => {
            return Err(DiagnosticDraft::unsupported(
                "se-return of a non-int operand",
            ));
        }
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("se-return reads unchecked operand {}", operand.index()),
            ));
        }
    };
    // The M1 function return type: the single committed `Function` type;
    // the M1 identity corner requires it to be `int`, matching the operand.
    let mut func_result = None;
    for (_, record) in bus.arenas.types.iter() {
        if let crate::bus::TypeKind::Function { result, .. } = &record.kind {
            if func_result.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "duplicate function type",
                ));
            }
            func_result = Some(*result);
        }
    }
    let Some(func_result) = func_result else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "function type is not committed",
        ));
    };
    if operand_ty != func_result {
        return Err(DiagnosticDraft::unsupported(
            "se-return with a non-identity conversion",
        ));
    }
    Ok(SeReturnInput {
        task,
        state: record.state.clone(),
        node,
        return_ty: operand_ty,
        existing: sem_for_node(bus, node),
        sem_allocated: bus.arenas.sem.allocated(),
    })
}

/// The T07 return-statement worker (SE21 slice scope).
pub struct SeRetChip;

impl Worker for SeRetChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: SE_RET_CHIP,
            chip_name: "SeRetChip",
            group: TaskGroup::SEMANTIC,
            task_kinds: vec![TaskKind::SEMANTIC_RETURN_STMT],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Lex, "literals"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Sem, "records"),
            ],
            writes: vec![FieldPath::new(StoreId::Sem, "records")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c14_se.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_se_return_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl SeRetChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &SeReturnInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-return task {} is not running", input.task.index()),
                ),
            )];
        }
        if let Some((existing, _)) = input.existing {
            return vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Sem(existing)),
            }];
        }
        let predicted = crate::ids::SemId::from_index(input.sem_allocated);
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: crate::ids::RecordFamily::Sem,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::Sem(SemRecord {
                        node: input.node,
                        ty: input.return_ty,
                        category: ValueCategory::NonLvalue,
                        effects: EffectMask(0),
                    })],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Sem(predicted)),
            },
        ]
    }
}
