// ============================================================================
// chips/semantic/se_identifier.rs — T07 identifier-expression worker
// (`/49` M1 source-locals slice)
//
// Reads one committed `Identifier` node, resolves the enclosing function
// body's active block scope from committed parent links, resolves the named
// automatic `int` object there, appends one `SemRecord` (`NonLvalue`,
// `EffectMask(0)`), and completes with its reference. Sibling function
// bodies have independent scopes. The declaration itself is validated by
// `TySymbolChip`; this worker only consumes the committed symbol. Missing
// or non-int bindings fail loudly.
// ============================================================================

use crate::bus::{
    EffectMask, NodeKind, ScopeKind, SemRecord, StorageDuration, SymbolKind, TypeKind,
    ValueCategory,
};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NodeId, RecordRef, ScopeId, SymbolId, TaskId, TypeId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, SE_ID_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// The enclosing function body and its committed block-scope chain.
#[derive(Clone, Debug)]
pub(crate) struct EnclosingBlock {
    /// Active scope chain, innermost first (the block, then its parents).
    pub chain: Vec<ScopeId>,
    /// Enclosing `FunctionDefinition` node.
    pub function: NodeId,
}

/// Resolve the block scope owned by the node's enclosing function body.
///
/// Walks committed parent links to the nearest `Compound` and
/// `FunctionDefinition`, then selects the unique block scope whose owner is
/// that `Compound`. Multiple function bodies therefore have independent
/// scopes; no global "exactly one block scope" assumption is made.
pub(crate) fn enclosing_block(
    bus: &crate::bus::CompilerBus,
    node: NodeId,
) -> Result<EnclosingBlock, DiagnosticDraft> {
    let mut current = Some(node);
    let mut compound = None;
    let mut function = None;
    let mut hops = 0u32;
    while let Some(id) = current {
        if hops > bus.arenas.nodes.allocated() {
            return Err(protocol_fault(format!(
                "parent chain from node {} is cyclic",
                node.index()
            )));
        }
        let body = bus.arenas.nodes.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("reads missing ancestor node {}", id.index()),
            )
        })?;
        if compound.is_none() && body.kind == NodeKind::Compound {
            compound = Some(id);
        }
        if function.is_none() && body.kind == NodeKind::FunctionDefinition {
            function = Some(id);
        }
        if compound.is_some() && function.is_some() {
            break;
        }
        current = body.parent;
        hops += 1;
    }
    let Some(compound) = compound else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("node {} is not inside a function body", node.index()),
        ));
    };
    let Some(function) = function else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("node {} is not inside a function definition", node.index()),
        ));
    };
    let mut scopes: Vec<ScopeId> = bus
        .arenas
        .scopes
        .iter()
        .filter(|(_, scope)| scope.kind == ScopeKind::Block && scope.owner == Some(compound))
        .map(|(id, _)| id)
        .collect();
    scopes.sort_by_key(|id| id.index());
    if scopes.len() != 1 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "function body {} must own exactly one block scope",
                compound.index()
            ),
        ));
    }
    let scope = scopes[0];
    let mut chain = Vec::new();
    let mut current = Some(scope);
    let mut hops = 0u32;
    while let Some(id) = current {
        if hops > bus.arenas.scopes.allocated() {
            return Err(protocol_fault(format!(
                "parent chain from scope {} is cyclic",
                scope.index()
            )));
        }
        let body = bus.arenas.scopes.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("reads missing scope {}", id.index()),
            )
        })?;
        chain.push(id);
        current = body.parent;
        hops += 1;
    }
    Ok(EnclosingBlock { chain, function })
}

/// Narrow projection for the identifier-expression check.
#[derive(Clone, Debug)]
pub struct SeIdentifierInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Checked node.
    pub node: NodeId,
    /// Resolved automatic object.
    pub symbol: SymbolId,
    /// Canonical `int` type.
    pub int: TypeId,
    /// Committed `SemRecord` for the node, if any.
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

fn sem_for_node(
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
pub fn project_se_identifier_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<SeIdentifierInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("se-identifier of unknown task {}", task.index()))
        })?;
    if record.kind != TaskKind::SEMANTIC_IDENTIFIER_EXPR {
        return Err(protocol_fault(format!(
            "se-identifier task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "se-identifier task {} payload must carry exactly one node",
            task.index()
        )));
    }
    let node = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "se-identifier task {} payload must be a node",
                task.index()
            )));
        }
    };
    let node_body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-identifier reads missing node {}", node.index()),
        )
    })?;
    if node_body.kind != NodeKind::Identifier {
        return Err(DiagnosticDraft::unsupported(
            "se-identifier of a non-identifier node",
        ));
    }
    let Some(name) = node_body.name else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-identifier reads nameless node {}", node.index()),
        ));
    };
    // Resolve the active block scope from this identifier's enclosing
    // function body; sibling function bodies are independent.
    let block = enclosing_block(bus, node)?;
    let mut hit: Option<SymbolId> = None;
    for scope in &block.chain {
        let mut scope_hit = None;
        for (id, symbol) in bus.arenas.symbols.iter() {
            if symbol.name != name || symbol.scope != *scope {
                continue;
            }
            if !matches!(symbol.kind, SymbolKind::Object | SymbolKind::Function) {
                continue;
            }
            if scope_hit.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "se-identifier reads ambiguous symbol binding",
                ));
            }
            scope_hit = Some(id);
        }
        if scope_hit.is_some() {
            hit = scope_hit;
            break;
        }
    }
    let Some(symbol) = hit else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 2),
            format!("se-identifier reads undeclared name {}", name.index()),
        ));
    };
    let symbol_body = bus.arenas.symbols.get(symbol).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-identifier reads missing symbol {}", symbol.index()),
        )
    })?;
    let int = canonical_int(bus)?;
    if symbol_body.kind != SymbolKind::Object
        || symbol_body.storage != StorageDuration::Automatic
        || symbol_body.ty != Some(int)
    {
        return Err(DiagnosticDraft::unsupported(
            "se-identifier of a non-automatic-int object",
        ));
    }
    Ok(SeIdentifierInput {
        task,
        state: record.state.clone(),
        node,
        symbol,
        int,
        existing: sem_for_node(bus, node),
        sem_allocated: bus.arenas.sem.allocated(),
    })
}

/// The T07 identifier-expression worker.
pub struct SeIdentifierChip;

impl Worker for SeIdentifierChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: SE_ID_CHIP,
            chip_name: "SeIdentifierChip",
            group: TaskGroup::SEMANTIC,
            task_kinds: vec![TaskKind::SEMANTIC_IDENTIFIER_EXPR],
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
        let input = match project_se_identifier_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl SeIdentifierChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &SeIdentifierInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-identifier task {} is not running", input.task.index()),
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
