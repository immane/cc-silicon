// ============================================================================
// chips/semantic/se_literal.rs — T07 SE02 literal-expression worker
// (Wave 2 slice 5, `/14`)
//
// Reads one committed `IntLiteral` node, checks its committed literal and
// the canonical `int` type, appends one `SemRecord` (`NonLvalue`,
// `EffectMask(0)`), and completes with its reference. Reuses a committed
// `SemRecord` when one already exists for the node (preserves exactly-one
// per node). Anything outside the M1 checked shape fails explicitly.
// ============================================================================

use crate::bus::{EffectMask, NodeKind, SemRecord, ValueCategory};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, NodeId, RecordRef, TaskId, TypeId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, SE_LIT_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the literal-expression check.
#[derive(Clone, Debug)]
pub struct SeLiteralInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Checked node.
    pub node: NodeId,
    /// Committed literal referenced by the node.
    pub literal: LiteralId,
    /// Canonical `int` type.
    pub int: TypeId,
    /// Committed `SemRecord` for the node, if any.
    pub existing: Option<(crate::ids::SemId, SemRecord)>,
    /// `sem` arena count at dispatch (prediction base).
    pub sem_allocated: u32,
}

/// Find the canonical M1 `int` type (exactly one must be committed).
pub fn find_m1_int(bus: &crate::bus::CompilerBus) -> Result<TypeId, DiagnosticDraft> {
    let mut found = None;
    for (id, record) in bus.arenas.types.iter() {
        if record.kind
            == (crate::bus::TypeKind::Int {
                rank: crate::bus::IntRank::Int,
                signed: true,
            })
        {
            if found.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "duplicate canonical int type",
                ));
            }
            found = Some(id);
        }
    }
    found.ok_or_else(|| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "canonical int type is not committed",
        )
    })
}

/// Committed `SemRecord` for a node, if any (at most one by construction).
pub fn sem_for_node(
    bus: &crate::bus::CompilerBus,
    node: NodeId,
) -> Option<(crate::ids::SemId, SemRecord)> {
    bus.arenas
        .sem
        .iter()
        .find(|(_, record)| record.node == node)
        .map(|(id, record)| (id, record.clone()))
}

/// Build the narrow projection for one dispatched task.
pub fn project_se_literal_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<SeLiteralInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("se-literal of unknown task {}", task.index())))?;
    if record.kind != TaskKind::SEMANTIC_LITERAL_EXPR {
        return Err(protocol_fault(format!(
            "se-literal task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "se-literal task {} payload must carry exactly one node",
            task.index()
        )));
    }
    let node = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "se-literal task {} payload must be a node",
                task.index()
            )));
        }
    };
    let node_body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-literal reads missing node {}", node.index()),
        )
    })?;
    if node_body.kind != NodeKind::IntLiteral {
        return Err(DiagnosticDraft::unsupported(
            "se-literal of a non-literal node",
        ));
    }
    let Some(literal) = node_body.literal else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-literal reads literal-less node {}", node.index()),
        ));
    };
    if bus.arenas.literals.get(literal).is_err() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-literal reads missing literal {}", literal.index()),
        ));
    }
    let int = find_m1_int(bus)?;
    Ok(SeLiteralInput {
        task,
        state: record.state.clone(),
        node,
        literal,
        int,
        existing: sem_for_node(bus, node),
        sem_allocated: bus.arenas.sem.allocated(),
    })
}

/// The T07 literal-expression worker (SE02 slice scope).
pub struct SeLitChip;

impl Worker for SeLitChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: SE_LIT_CHIP,
            chip_name: "SeLitChip",
            group: TaskGroup::SEMANTIC,
            task_kinds: vec![TaskKind::SEMANTIC_LITERAL_EXPR],
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
        let input = match project_se_literal_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl SeLitChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &SeLiteralInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-literal task {} is not running", input.task.index()),
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
