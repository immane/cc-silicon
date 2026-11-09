// ============================================================================
// chips/semantic/se_call.rs — `/54` direct-call-expression worker
//
// Reads one committed `Call` node (`callee Identifier` followed by argument
// expressions in source order), resolves the callee through the active scope
// chain innermost-first, and requires an earlier file-scope function
// definition. The callee's canonical signature fixes the arity (0..8); every
// argument needs a committed canonical-`int` `NonLvalue` fact with zero
// effects. The call itself appends one canonical-`int` `NonLvalue` fact with
// zero effects, or reuses the already committed one. The callee identifier
// owns no `SemRecord`. Every failure is typed and writes nothing.
// ============================================================================

use super::se_identifier::enclosing_block;
use super::se_literal::{find_m1_int, sem_for_node};
use crate::bus::{
    EffectMask, NodeKind, ScopeKind, SemRecord, SymbolKind, TypeKind, ValueCategory,
};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NodeId, RecordRef, SymbolId, TaskId, TypeId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, SE_CALL_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the direct-call check.
#[derive(Clone, Debug)]
pub struct SeCallInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Checked `Call` node.
    pub node: NodeId,
    /// Resolved file-scope function symbol.
    pub callee: SymbolId,
    /// Resolved callee function type.
    pub callee_ty: TypeId,
    /// Canonical `int` type.
    pub int: TypeId,
    /// Committed `SemRecord` for the call node, if any.
    pub existing: Option<(crate::ids::SemId, SemRecord)>,
    /// `sem` arena count at dispatch (prediction base).
    pub sem_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_se_call_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<SeCallInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("se-call of unknown task {}", task.index())))?;
    if record.kind != TaskKind::SEMANTIC_CALL_EXPR {
        return Err(protocol_fault(format!(
            "se-call task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "se-call task {} payload must carry exactly one node",
            task.index()
        )));
    }
    let node = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "se-call task {} payload must be a node",
                task.index()
            )));
        }
    };
    let node_body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-call reads missing node {}", node.index()),
        )
    })?;
    if node_body.kind != NodeKind::Call {
        return Err(DiagnosticDraft::unsupported(
            "se-call of a non-call node",
        ));
    }
    let Some(callee_node) = node_body.children.first().copied() else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-call node {} has no callee", node.index()),
        ));
    };
    let callee_body = bus.arenas.nodes.get(callee_node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-call reads missing callee {}", callee_node.index()),
        )
    })?;
    if callee_body.kind != NodeKind::Identifier {
        return Err(DiagnosticDraft::unsupported(
            "se-call with a non-identifier callee",
        ));
    }
    let Some(callee_name) = callee_body.name else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-call reads nameless callee {}", callee_node.index()),
        ));
    };
    if sem_for_node(bus, callee_node).is_some() {
        return Err(protocol_fault(format!(
            "se-call callee {} carries an unexpected SemRecord",
            callee_node.index()
        )));
    }
    let block = enclosing_block(bus, node)?;
    // Innermost-first active scope resolution. An inner object binding
    // shadows a file-scope function of the same name; the binding is then
    // rejected as not callable rather than skipped.
    let mut binding: Option<SymbolId> = None;
    for scope in &block.chain {
        let mut scope_hit = None;
        for (id, symbol) in bus.arenas.symbols.iter() {
            if symbol.name != callee_name || symbol.scope != *scope {
                continue;
            }
            if !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Object) {
                continue;
            }
            if scope_hit.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "se-call reads ambiguous callee binding",
                ));
            }
            scope_hit = Some(id);
        }
        if scope_hit.is_some() {
            binding = scope_hit;
            break;
        }
    }
    let Some(callee) = binding else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 1),
            format!("se-call reads undeclared function {}", callee_name.index()),
        ));
    };
    let callee_symbol = bus.arenas.symbols.get(callee).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-call reads missing symbol {}", callee.index()),
        )
    })?;
    if callee_symbol.kind != SymbolKind::Function {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 1),
            "se-call target is an object, not a function",
        ));
    }
    let callee_scope = bus.arenas.scopes.get(callee_symbol.scope).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "se-call reads missing callee scope {}",
                callee_symbol.scope.index()
            ),
        )
    })?;
    if callee_scope.kind != ScopeKind::File {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 1),
            "se-call target is not a file-scope function",
        ));
    }
    if let Some(main) = bus.intern.lookup(b"main") {
        if callee_symbol.name == main {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Semantic, 1),
                "se-call to `main` is outside the direct-call subset",
            ));
        }
    }
    // Definitions before use: the callee's declarator precedes the
    // enclosing function definition in parse (source) order. This rejects
    // self-recursion, forward references, and mutual recursion.
    if callee_symbol.decl.index() >= block.function.index() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 1),
            format!(
                "se-call target {} is not defined before the enclosing function",
                callee_symbol.decl.index()
            ),
        ));
    }
    let Some(callee_ty) = callee_symbol.ty else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "se-call reads untyped callee",
        ));
    };
    let callee_ty_body = bus.arenas.types.get(callee_ty).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-call reads missing callee type {}", callee_ty.index()),
        )
    })?;
    let TypeKind::Function {
        result,
        params,
        prototype,
        variadic,
    } = &callee_ty_body.kind
    else {
        return Err(DiagnosticDraft::unsupported(
            "se-call target does not have a function type",
        ));
    };
    let int = find_m1_int(bus)?;
    if params.len() > 8 {
        return Err(DiagnosticDraft::unsupported(
            "se-call target has more than eight parameters",
        ));
    }
    if *result != int
        || !params.iter().all(|param| *param == int)
        || !*prototype
        || *variadic
    {
        return Err(DiagnosticDraft::unsupported(
            "se-call target has a non-M1 signature",
        ));
    }
    let arguments = &node_body.children[1..];
    if arguments.len() != params.len() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 1),
            format!(
                "se-call arity mismatch: expected {} arguments, found {}",
                params.len(),
                arguments.len()
            ),
        ));
    }
    for argument in arguments {
        match sem_for_node(bus, *argument) {
            Some((_, sem))
                if sem.ty == int
                    && sem.category == ValueCategory::NonLvalue
                    && sem.effects == EffectMask(0) => {}
            Some((_, sem)) if sem.ty != int => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Semantic, 1),
                    format!("se-call argument {} is not canonical int", argument.index()),
                ));
            }
            Some((_, sem)) if sem.category != ValueCategory::NonLvalue => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Semantic, 1),
                    format!(
                        "se-call argument {} is not a non-lvalue",
                        argument.index()
                    ),
                ));
            }
            Some(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Semantic, 1),
                    format!("se-call argument {} has side effects", argument.index()),
                ));
            }
            None => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-call reads unchecked argument {}", argument.index()),
                ));
            }
        }
    }
    Ok(SeCallInput {
        task,
        state: record.state.clone(),
        node,
        callee,
        callee_ty,
        int,
        existing: sem_for_node(bus, node),
        sem_allocated: bus.arenas.sem.allocated(),
    })
}

/// The `/54` direct-call-expression worker.
pub struct SeCallChip;

impl Worker for SeCallChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: SE_CALL_CHIP,
            chip_name: "SeCallChip",
            group: TaskGroup::SEMANTIC,
            task_kinds: vec![TaskKind::SEMANTIC_CALL_EXPR],
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
            tests: vec!["compiler/tests/call_semantic.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_se_call_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl SeCallChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &SeCallInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-call task {} is not running", input.task.index()),
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
        let _ = (input.callee, input.callee_ty);
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
