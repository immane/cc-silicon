// ============================================================================
// chips/semantic/se_unary.rs — T07 unary-expression worker
// (`/52` local expression-tree slice)
//
// Reads one committed `UnaryPlus`/`UnaryMinus` node, requires one child with a
// committed canonical-`int` `SemRecord`, appends one `SemRecord`
// (`NonLvalue`, `EffectMask(0)`), and completes with its reference. The
// operand is already checked in post-order; this worker never re-enters the
// bus or calls another chip.
// ============================================================================

use super::se_literal::sem_for_node;
use crate::bus::{EffectMask, NodeKind, SemRecord, ValueCategory};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NodeId, RecordRef, TaskId, TypeId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, SE_UNARY_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the unary-expression check.
#[derive(Clone, Debug)]
pub struct SeUnaryInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Checked node.
    pub node: NodeId,
    /// Checked operand node.
    pub operand: NodeId,
    /// Canonical `int` type.
    pub int: TypeId,
    /// Committed `SemRecord` for the node, if any.
    pub existing: Option<(crate::ids::SemId, SemRecord)>,
    /// `sem` arena count at dispatch (prediction base).
    pub sem_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_se_unary_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<SeUnaryInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("se-unary of unknown task {}", task.index())))?;
    if record.kind != TaskKind::SEMANTIC_UNARY_EXPR {
        return Err(protocol_fault(format!(
            "se-unary task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "se-unary task {} payload must carry exactly one node",
            task.index()
        )));
    }
    let node = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "se-unary task {} payload must be a node",
                task.index()
            )));
        }
    };
    let node_body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-unary reads missing node {}", node.index()),
        )
    })?;
    if !matches!(node_body.kind, NodeKind::UnaryPlus | NodeKind::UnaryMinus) {
        return Err(DiagnosticDraft::unsupported("se-unary of a non-unary node"));
    }
    if node_body.children.len() != 1 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-unary node {} must have one operand", node.index()),
        ));
    }
    let operand = node_body.children[0];
    let operand_body = bus.arenas.nodes.get(operand).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-unary reads missing operand {}", operand.index()),
        )
    })?;
    if !matches!(
        operand_body.kind,
        NodeKind::IntLiteral
            | NodeKind::Identifier
            | NodeKind::BinaryAdd
            | NodeKind::BinarySub
            | NodeKind::BinaryMul
            | NodeKind::BinaryDiv
            | NodeKind::BinaryRem
            | NodeKind::UnaryPlus
            | NodeKind::UnaryMinus
    ) {
        return Err(DiagnosticDraft::unsupported(
            "se-unary of a non-covered operand",
        ));
    }
    let mut int = None;
    for (id, record) in bus.arenas.types.iter() {
        if record.kind
            == (crate::bus::TypeKind::Int {
                rank: crate::bus::IntRank::Int,
                signed: true,
            })
        {
            if int.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "duplicate canonical int type",
                ));
            }
            int = Some(id);
        }
    }
    let Some(int) = int else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "canonical int type is not committed",
        ));
    };
    match sem_for_node(bus, operand) {
        Some((_, sem)) if sem.ty == int => {}
        Some(_) => {
            return Err(DiagnosticDraft::unsupported(
                "se-unary of a non-int operand",
            ));
        }
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("se-unary reads unchecked operand {}", operand.index()),
            ));
        }
    }
    Ok(SeUnaryInput {
        task,
        state: record.state.clone(),
        node,
        operand,
        int,
        existing: sem_for_node(bus, node),
        sem_allocated: bus.arenas.sem.allocated(),
    })
}

/// The T07 unary-expression worker.
pub struct SeUnaryChip;

impl Worker for SeUnaryChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: SE_UNARY_CHIP,
            chip_name: "SeUnaryChip",
            group: TaskGroup::SEMANTIC,
            task_kinds: vec![TaskKind::SEMANTIC_UNARY_EXPR],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Sem, "records"),
            ],
            writes: vec![FieldPath::new(StoreId::Sem, "records")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c56_source_locals.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_se_unary_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl SeUnaryChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &SeUnaryInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-unary task {} is not running", input.task.index()),
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
        let _ = input.operand;
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
                        ty: input.int,
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
