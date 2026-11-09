// ============================================================================
// chips/semantic/se_assignment.rs — T07 assignment-expression worker
// (`/53` local-statement slice)
//
// Reads one committed `Assignment` node with an `Identifier` lhs and a checked
// rhs, resolves the lhs to a visible automatic `int` object, and appends one
// `SemRecord` carrying `EFFECT_WRITES_LOCAL`. The assignment expression itself
// is a non-lvalue. Missing/wrong bindings and non-int rhs facts fail closed.
// ============================================================================

use super::se_literal::sem_for_node;
use crate::bus::{
    EffectMask, NodeKind, ScopeKind, SemRecord, StorageDuration, SymbolKind, TypeKind,
    ValueCategory, EFFECT_WRITES_LOCAL,
};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NodeId, RecordRef, ScopeId, SymbolId, TaskId, TypeId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, SE_ASSIGN_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the assignment check.
#[derive(Clone, Debug)]
pub struct SeAssignmentInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Checked assignment node.
    pub node: NodeId,
    /// Resolved automatic local.
    pub symbol: SymbolId,
    /// Canonical `int` type.
    pub int: TypeId,
    /// Committed `SemRecord` for the assignment node, if any.
    pub existing: Option<(crate::ids::SemId, SemRecord)>,
    /// `sem` arena count at dispatch (prediction base).
    pub sem_allocated: u32,
}

fn canonical_int(bus: &crate::bus::CompilerBus) -> Result<TypeId, DiagnosticDraft> {
    let mut found = None;
    for (id, record) in bus.arenas.types.iter() {
        if record.kind
            == (TypeKind::Int {
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

/// Build the narrow projection for one dispatched task.
pub fn project_se_assignment_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<SeAssignmentInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("se-assignment of unknown task {}", task.index()))
        })?;
    if record.kind != TaskKind::SEMANTIC_ASSIGNMENT_EXPR {
        return Err(protocol_fault(format!(
            "se-assignment task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "se-assignment task {} payload must carry exactly one node",
            task.index()
        )));
    }
    let node = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "se-assignment task {} payload must be a node",
                task.index()
            )));
        }
    };
    let node_body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-assignment reads missing node {}", node.index()),
        )
    })?;
    if node_body.kind != NodeKind::Assignment || node_body.children.len() != 2 {
        return Err(DiagnosticDraft::unsupported(
            "se-assignment of a non-assignment shape",
        ));
    }
    let lhs = node_body.children[0];
    let rhs = node_body.children[1];
    let lhs_body = bus.arenas.nodes.get(lhs).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-assignment reads missing lhs {}", lhs.index()),
        )
    })?;
    if lhs_body.kind != NodeKind::Identifier {
        return Err(DiagnosticDraft::unsupported(
            "se-assignment of a non-identifier lhs",
        ));
    }
    let rhs_body = bus.arenas.nodes.get(rhs).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-assignment reads missing rhs {}", rhs.index()),
        )
    })?;
    if !matches!(
        rhs_body.kind,
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
            "se-assignment of a non-covered rhs",
        ));
    }
    let Some(name) = lhs_body.name else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "se-assignment reads nameless lhs",
        ));
    };
    let int = canonical_int(bus)?;
    match sem_for_node(bus, rhs) {
        Some((_, sem)) if sem.ty == int => {}
        Some(_) => {
            return Err(DiagnosticDraft::unsupported(
                "se-assignment of a non-int rhs",
            ));
        }
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("se-assignment reads unchecked rhs {}", rhs.index()),
            ));
        }
    }
    let mut blocks: Vec<ScopeId> = bus
        .arenas
        .scopes
        .iter()
        .filter(|(_, scope)| scope.kind == ScopeKind::Block)
        .map(|(id, _)| id)
        .collect();
    blocks.sort_by_key(|id| id.index());
    if blocks.len() != 1 {
        return Err(DiagnosticDraft::unsupported(
            "se-assignment requires exactly one block scope",
        ));
    }
    let mut chain = Vec::new();
    let mut current = blocks.first().copied();
    while let Some(id) = current {
        let scope = bus.arenas.scopes.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("se-assignment reads missing scope {}", id.index()),
            )
        })?;
        chain.push(id);
        current = scope.parent;
    }
    let mut hit = None;
    for scope in &chain {
        for (id, symbol) in bus.arenas.symbols.iter() {
            if symbol.name == name && symbol.scope == *scope && symbol.kind == SymbolKind::Object {
                if hit.is_some() {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        "se-assignment reads ambiguous lhs binding",
                    ));
                }
                hit = Some(id);
            }
        }
        if hit.is_some() {
            break;
        }
    }
    let Some(symbol) = hit else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 2),
            format!("se-assignment reads undeclared name {}", name.index()),
        ));
    };
    let symbol_body = bus.arenas.symbols.get(symbol).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-assignment reads missing symbol {}", symbol.index()),
        )
    })?;
    if symbol_body.storage != StorageDuration::Automatic || symbol_body.ty != Some(int) {
        return Err(DiagnosticDraft::unsupported(
            "se-assignment of a non-automatic-int lhs",
        ));
    }
    Ok(SeAssignmentInput {
        task,
        state: record.state.clone(),
        node,
        symbol,
        int,
        existing: sem_for_node(bus, node),
        sem_allocated: bus.arenas.sem.allocated(),
    })
}

/// The T07 assignment-expression worker.
pub struct SeAssignmentChip;

impl Worker for SeAssignmentChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: SE_ASSIGN_CHIP,
            chip_name: "SeAssignmentChip",
            group: TaskGroup::SEMANTIC,
            task_kinds: vec![TaskKind::SEMANTIC_ASSIGNMENT_EXPR],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Symbols, "symbols"),
                FieldPath::new(StoreId::Symbols, "scopes"),
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
        let input = match project_se_assignment_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl SeAssignmentChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &SeAssignmentInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-assignment task {} is not running", input.task.index()),
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
        let _ = input.symbol;
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
                        effects: EffectMask(EFFECT_WRITES_LOCAL),
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
