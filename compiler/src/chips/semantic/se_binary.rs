// ============================================================================
// chips/semantic/se_binary.rs — T07 SE07 binary-expression worker
// (Wave 2 slice 5, `/14`)
//
// Reads one committed `BinaryAdd` node, checks both operand leaves (each
// needs a committed `SemRecord` with the canonical `int` type), appends one
// `SemRecord`, enqueues one `const_fold` child with the identical forwarded
// refs, and awaits it. On resume (the join readies this task only after the
// child completes), it completes with the committed `SemRecord`. The join
// consumes nothing; T09 reads the fold's `ConstRecord` directly. Non-`Int`
// operands are explicit `Unsupported`.
// ============================================================================

use super::se_literal::{find_m1_int, sem_for_node};
use crate::bus::{EffectMask, NodeKind, SemRecord, ValueCategory};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, NodeId, RecordRef, TaskId, TypeId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, SE_BIN_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, ChildRef, DraftRef, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup,
    TaskKind, TaskState,
};

/// Narrow projection for the binary-expression check.
#[derive(Clone, Debug)]
pub struct SeBinaryInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Checked node.
    pub node: NodeId,
    /// Operand `(node, literal)` leaves in source order.
    pub leaves: Vec<(NodeId, LiteralId)>,
    /// Canonical `int` type.
    pub int: TypeId,
    /// Committed `SemRecord` for the node, if any.
    pub existing: Option<(crate::ids::SemId, SemRecord)>,
    /// `sem` arena count at dispatch (prediction base).
    pub sem_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_se_binary_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<SeBinaryInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("se-binary of unknown task {}", task.index())))?;
    if record.kind != TaskKind::SEMANTIC_BINARY_EXPR {
        return Err(protocol_fault(format!(
            "se-binary task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "se-binary task {} payload must carry exactly one node",
            task.index()
        )));
    }
    let node = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "se-binary task {} payload must be a node",
                task.index()
            )));
        }
    };
    let node_body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-binary reads missing node {}", node.index()),
        )
    })?;
    if node_body.kind != NodeKind::BinaryAdd {
        return Err(DiagnosticDraft::unsupported(
            "se-binary of a non-additive node",
        ));
    }
    if node_body.children.len() != 2 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-binary node {} must have two operands", node.index()),
        ));
    }
    let int = find_m1_int(bus)?;
    let mut leaves = Vec::new();
    for child in &node_body.children {
        let child_body = bus.arenas.nodes.get(*child).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("se-binary reads missing operand {}", child.index()),
            )
        })?;
        if child_body.kind != NodeKind::IntLiteral {
            return Err(DiagnosticDraft::unsupported(
                "se-binary of a non-literal operand",
            ));
        }
        let Some(literal) = child_body.literal else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("se-binary reads literal-less operand {}", child.index()),
            ));
        };
        if bus.arenas.literals.get(literal).is_err() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("se-binary reads missing literal {}", literal.index()),
            ));
        }
        // Each operand leaf needs its committed `SemRecord` (checked first,
        // in task order) with the canonical `int` type.
        match sem_for_node(bus, *child) {
            Some((_, sem)) if sem.ty == int => {}
            Some(_) => {
                return Err(DiagnosticDraft::unsupported(
                    "se-binary of a non-int operand",
                ));
            }
            None => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-binary reads unchecked operand {}", child.index()),
                ));
            }
        }
        leaves.push((*child, literal));
    }
    Ok(SeBinaryInput {
        task,
        state: record.state.clone(),
        node,
        leaves,
        int,
        existing: sem_for_node(bus, node),
        sem_allocated: bus.arenas.sem.allocated(),
    })
}

/// The T07 binary-expression worker (SE07 slice scope).
pub struct SeBinChip;

impl Worker for SeBinChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: SE_BIN_CHIP,
            chip_name: "SeBinChip",
            group: TaskGroup::SEMANTIC,
            task_kinds: vec![TaskKind::SEMANTIC_BINARY_EXPR],
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
        let input = match project_se_binary_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl SeBinChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// First dispatch: append the `SemRecord`, enqueue the `const_fold`
    /// child with identical forwarded refs, and await it. Resume dispatch
    /// (the committed `SemRecord` exists): complete with it. The join
    /// readies this task only after the child completes, so resuming with
    /// a committed `SemRecord` is the success path, never a duplicate.
    pub fn compute(&self, input: &SeBinaryInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-binary task {} is not running", input.task.index()),
                ),
            )];
        }
        if let Some((existing, _)) = input.existing {
            return vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Sem(existing)),
            }];
        }
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
            Proposal::Enqueue(TaskDraft {
                kind: TaskKind::CONSTANT_CONST_FOLD,
                owner: crate::manifest::G1_FOLD_CHIP,
                parent: Some(input.task),
                payload: crate::task::Payload::from_refs(vec![
                    RecordRef::Node(input.node),
                    RecordRef::Literal(input.leaves[0].1),
                    RecordRef::Literal(input.leaves[1].1),
                ]),
                continuation: None,
            }),
            Proposal::AwaitChildren {
                task: input.task,
                children: vec![ChildRef::OwnBatch(0)],
            },
        ]
    }
}
