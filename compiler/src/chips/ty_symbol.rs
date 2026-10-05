// ============================================================================
// chips/ty_symbol.rs — T06 TY07/TY20/TY04/TY10 declare + TY03 lookup workers
// (Wave 2 slice 4, `/13`)
//
// `TySymbolChip` serves `symbol_type.declare` (payload: exactly
// `[Declarator Node, FuncType, FileScope]`; appends one `SymbolRecord` for
// `main` with external linkage and static storage; duplicate
// `(name, scope, kind)` declarations fail as `RedeclarationConflict` chip
// diagnostics, never silent merges) and `symbol_type.lookup` (payload:
// exactly `[Scope, Name]`; walks the active scope chain innermost-first and
// completes with the hit, or fails with a typed undeclared-identifier
// diagnostic). Namespaces are derived from the symbol kind; a hit in another
// namespace is a miss. Query-point ordering stays deferred: M1 has one
// declarator, so declaration order is the commit order.
// ============================================================================

use super::{fail, protocol_fault, Worker};
use crate::bus::{Linkage, NodeKind, StorageDuration, SymbolKind, TypeKind};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NameId, NodeId, RecordRef, ScopeId, SymbolId, TaskId, TypeId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, TY_SYMBOL_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Ordinary-namespace membership derived from the symbol kind (T06 item 3).
pub fn in_ordinary_namespace(kind: SymbolKind) -> bool {
    matches!(kind, SymbolKind::Function | SymbolKind::Object)
}

/// Narrow projection for the declare computation.
#[derive(Clone, Debug)]
pub struct TyDeclareInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Declarator node.
    pub node: NodeId,
    /// Declarator name.
    pub name: NameId,
    /// Function type.
    pub ty: TypeId,
    /// File scope.
    pub scope: ScopeId,
    /// Whether an identical `(name, scope, kind)` symbol is committed.
    pub conflict: bool,
    /// `symbols` arena count at dispatch (prediction base).
    pub symbols_allocated: u32,
}

/// Build the declare projection for one dispatched task.
pub fn project_ty_declare_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<TyDeclareInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("declare of unknown task {}", task.index())))?;
    if record.kind != TaskKind::SYMBOL_DECLARE {
        return Err(protocol_fault(format!(
            "declare task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 3 {
        return Err(protocol_fault(format!(
            "declare task {} payload must be exactly [Node, Type, Scope]",
            task.index()
        )));
    }
    let (Some(node), Some(ty), Some(scope)) = (
        match record.payload.refs[0] {
            RecordRef::Node(id) => Some(id),
            _ => None,
        },
        match record.payload.refs[1] {
            RecordRef::Type(id) => Some(id),
            _ => None,
        },
        match record.payload.refs[2] {
            RecordRef::Scope(id) => Some(id),
            _ => None,
        },
    ) else {
        return Err(protocol_fault(format!(
            "declare task {} payload must be exactly [Node, Type, Scope]",
            task.index()
        )));
    };
    let node_body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("declare reads missing node {}", node.index()),
        )
    })?;
    if node_body.kind != NodeKind::Declarator {
        return Err(DiagnosticDraft::unsupported(
            "declare of a non-declarator node",
        ));
    }
    let Some(name) = node_body.name else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("declare reads nameless declarator {}", node.index()),
        ));
    };
    let ty_body = bus.arenas.types.get(ty).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("declare reads missing type {}", ty.index()),
        )
    })?;
    if !matches!(ty_body.kind, TypeKind::Function { .. }) {
        return Err(DiagnosticDraft::unsupported(
            "declare of a non-function type",
        ));
    }
    let scope_body = bus.arenas.scopes.get(scope).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("declare reads missing scope {}", scope.index()),
        )
    })?;
    if scope_body.kind != crate::bus::ScopeKind::File {
        return Err(DiagnosticDraft::unsupported(
            "block-scope declarations are deferred past M1",
        ));
    }
    let conflict = bus.arenas.symbols.iter().any(|(_, symbol)| {
        symbol.name == name && symbol.scope == scope && symbol.kind == SymbolKind::Function
    });
    Ok(TyDeclareInput {
        task,
        state: record.state.clone(),
        node,
        name,
        ty,
        scope,
        conflict,
        symbols_allocated: bus.arenas.symbols.allocated(),
    })
}

/// Narrow projection for the lookup computation.
#[derive(Clone, Debug)]
pub struct TyLookupInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Searched name.
    pub name: NameId,
    /// Active scope chain, innermost first.
    pub chain: Vec<ScopeId>,
    /// Committed `(scope, symbol)` hits in symbol-ID order.
    pub hits: Vec<(ScopeId, SymbolId)>,
}

/// Build the lookup projection for one dispatched task.
pub fn project_ty_lookup_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<TyLookupInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("lookup of unknown task {}", task.index())))?;
    if record.kind != TaskKind::SYMBOL_LOOKUP {
        return Err(protocol_fault(format!(
            "lookup task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 2 {
        return Err(protocol_fault(format!(
            "lookup task {} payload must be exactly [Scope, Name]",
            task.index()
        )));
    }
    let (Some(scope), Some(name)) = (
        match record.payload.refs[0] {
            RecordRef::Scope(id) => Some(id),
            _ => None,
        },
        match record.payload.refs[1] {
            RecordRef::Name(id) => Some(id),
            _ => None,
        },
    ) else {
        return Err(protocol_fault(format!(
            "lookup task {} payload must be exactly [Scope, Name]",
            task.index()
        )));
    };
    // Active scope chain only: walk parents to the root (deterministic).
    let mut chain = Vec::new();
    let mut current = Some(scope);
    while let Some(id) = current {
        let body = bus.arenas.scopes.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lookup reads missing scope {}", id.index()),
            )
        })?;
        chain.push(id);
        current = body.parent;
    }
    // Ordinary-namespace hits in symbol-ID order.
    let mut hits = Vec::new();
    for (id, symbol) in bus.arenas.symbols.iter() {
        if symbol.name == name && in_ordinary_namespace(symbol.kind) {
            hits.push((symbol.scope, id));
        }
    }
    Ok(TyLookupInput {
        task,
        state: record.state.clone(),
        name,
        chain,
        hits,
    })
}

/// The T06 symbol worker (declare + lookup slice scope).
pub struct TySymbolChip;

impl Worker for TySymbolChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: TY_SYMBOL_CHIP,
            chip_name: "TySymbolChip",
            group: TaskGroup::SYMBOL_TYPE,
            task_kinds: vec![TaskKind::SYMBOL_DECLARE, TaskKind::SYMBOL_LOOKUP],
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
            ],
            writes: vec![FieldPath::new(StoreId::Symbols, "symbols")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c13_ty.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let kind = match bus.arenas.tasks.get(task) {
            Ok(record) => record.kind,
            Err(_) => {
                return vec![fail(
                    task,
                    protocol_fault(format!("symbol of unknown task {}", task.index())),
                )];
            }
        };
        match kind {
            TaskKind::SYMBOL_DECLARE => match project_ty_declare_input(bus, task) {
                Ok(input) => self.compute_declare(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            TaskKind::SYMBOL_LOOKUP => match project_ty_lookup_input(bus, task) {
                Ok(input) => self.compute_lookup(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            _ => vec![fail(
                task,
                protocol_fault(format!("symbol task {} has unexpected kind", task.index())),
            )],
        }
    }
}

impl TySymbolChip {
    /// Declare computation: one `SymbolRecord`, or a typed conflict failure.
    pub fn compute_declare(&self, input: &TyDeclareInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("declare task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.conflict {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Semantic, 1),
                    "redeclaration conflict: duplicate function declaration in this scope",
                ),
            )];
        }
        let predicted = SymbolId::from_index(input.symbols_allocated);
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: crate::ids::RecordFamily::Symbol,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::Symbol(crate::bus::SymbolRecord {
                        name: input.name,
                        scope: input.scope,
                        kind: SymbolKind::Function,
                        ty: Some(input.ty),
                        linkage: Linkage::External,
                        storage: StorageDuration::Static,
                        decl: input.node,
                    })],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Symbol(predicted)),
            },
        ]
    }

    /// Lookup computation: innermost-first chain hit, or a typed miss.
    pub fn compute_lookup(&self, input: &TyLookupInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("lookup task {} is not running", input.task.index()),
                ),
            )];
        }
        // Innermost active scope first; a same-scope tie selects the higher
        // `SymbolId` (T06 item 3 direction; M1 has one declarator).
        let mut best: Option<SymbolId> = None;
        for scope in &input.chain {
            let mut scope_best: Option<SymbolId> = None;
            for (hit_scope, hit_id) in &input.hits {
                if hit_scope == scope && scope_best.is_none_or(|current| *hit_id > current) {
                    scope_best = Some(*hit_id);
                }
            }
            if scope_best.is_some() {
                best = scope_best;
                break;
            }
        }
        match best {
            Some(id) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Symbol(id)),
            }],
            None => vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Semantic, 2),
                    format!("undeclared identifier {}", input.name.index()),
                ),
            )],
        }
    }
}
