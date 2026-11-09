// ============================================================================
// chips/semantic/se_function.rs — T07 SE29 function-definition worker
// (Wave 3 slice 14, `/39`; `/54` parameterized direct-call definitions)
//
// Reads one committed `FunctionDefinition` node, checks its declarator
// shape (no child for `(void)`, or one `ParameterList` with 1..8 implicit
// `int` `Parameter` leaves), selects the unique committed function type
// matching that parameter count, requires the declared function symbol to
// carry that type, and checks the committed `Return` child fact. Appends
// one `SemRecord` carrying the signature only (the `Return` node owns its
// own fact), or reuses a committed one. Anything outside the checked shape
// fails explicitly, never a fabricated plan.
// ============================================================================

use super::se_literal::{find_m1_int, sem_for_node};
use crate::bus::{EffectMask, NodeKind, SemRecord, ValueCategory};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NodeId, RecordRef, SemId, TaskId, TypeId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, SE_FUNC_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Frozen task kind for the function-definition check: SEMANTIC local
/// 21, the next free code after `SEMANTIC_RETURN_STMT` (20).
pub const SE_FUNC_TASK_KIND: TaskKind = TaskKind::SEMANTIC_FUNCTION_DEF;

/// Narrow projection for the function-definition check.
#[derive(Clone, Debug)]
pub struct SeFunctionInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Checked node.
    pub node: NodeId,
    /// Committed function signature carried by the record.
    pub signature: TypeId,
    /// Committed `SemRecord` for the node, if any.
    pub existing: Option<(SemId, SemRecord)>,
    /// `sem` arena count at dispatch (prediction base).
    pub sem_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_se_function_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<SeFunctionInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("se-function of unknown task {}", task.index())))?;
    if record.kind != SE_FUNC_TASK_KIND {
        return Err(protocol_fault(format!(
            "se-function task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "se-function task {} payload must carry exactly one node",
            task.index()
        )));
    }
    let node = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "se-function task {} payload must be a node",
                task.index()
            )));
        }
    };
    let node_body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-function reads missing node {}", node.index()),
        )
    })?;
    if node_body.kind != NodeKind::FunctionDefinition {
        return Err(DiagnosticDraft::unsupported(
            "se-function of a non-function-definition node",
        ));
    }
    if node_body.children.len() != 3 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-function reads malformed function node {}", node.index()),
        ));
    }
    let specifiers = node_body.children[0];
    let declarator = node_body.children[1];
    let compound = node_body.children[2];
    let missing = |what: &str, index: u32| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("se-function reads missing {what} {index}"),
        )
    };
    let specifiers_body = bus
        .arenas
        .nodes
        .get(specifiers)
        .map_err(|_| missing("node", specifiers.index()))?;
    if specifiers_body.kind != NodeKind::Specifiers {
        return Err(DiagnosticDraft::unsupported(
            "se-function of a non-M1 specifier bundle",
        ));
    }
    let declarator_body = bus
        .arenas
        .nodes
        .get(declarator)
        .map_err(|_| missing("node", declarator.index()))?;
    if declarator_body.kind != NodeKind::Declarator {
        return Err(DiagnosticDraft::unsupported(
            "se-function of a non-declarator definition",
        ));
    }
    // Declarator shape: no child is `(void)`; one `ParameterList` child
    // carries 1..8 implicit-`int` `Parameter` leaves. Anything else is an
    // old-style or malformed parameter list, explicitly unsupported.
    let param_count = if declarator_body.children.is_empty() {
        0
    } else if declarator_body.children.len() == 1 {
        let list = declarator_body.children[0];
        let list_body = bus
            .arenas
            .nodes
            .get(list)
            .map_err(|_| missing("node", list.index()))?;
        if list_body.kind != NodeKind::ParameterList {
            return Err(DiagnosticDraft::unsupported(
                "se-function of an old-style (K&R) parameter list",
            ));
        }
        if !(1..=8).contains(&list_body.children.len()) {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "se-function parameter list must carry 1..8 parameters, found {}",
                    list_body.children.len()
                ),
            ));
        }
        let mut names = Vec::new();
        for parameter in &list_body.children {
            let parameter_body = bus
                .arenas
                .nodes
                .get(*parameter)
                .map_err(|_| missing("node", parameter.index()))?;
            if parameter_body.kind != NodeKind::Parameter || !parameter_body.children.is_empty() {
                return Err(DiagnosticDraft::unsupported(
                    "se-function of a non-parameter parameter-list child",
                ));
            }
            let Some(name) = parameter_body.name else {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-function reads nameless parameter {}", parameter.index()),
                ));
            };
            if names.contains(&name) {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Semantic, 1),
                    "duplicate parameter name in function definition",
                ));
            }
            names.push(name);
        }
        list_body.children.len()
    } else {
        return Err(DiagnosticDraft::unsupported(
            "se-function of a malformed declarator child list",
        ));
    };
    // Canonical `int`, then the unique committed function type matching this
    // declarator's parameter count. All parameter and result types are
    // canonical `int`; several signatures may be committed in one TU.
    let int = find_m1_int(bus)?;
    let mut signature = None;
    let mut count_matches = 0usize;
    for (id, record) in bus.arenas.types.iter() {
        if let crate::bus::TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        } = &record.kind
        {
            if params.len() != param_count {
                continue;
            }
            count_matches += 1;
            if *result == int
                && params.iter().all(|param| *param == int)
                && *prototype
                && !*variadic
            {
                if signature.is_some() {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        "duplicate function type for this parameter count",
                    ));
                }
                signature = Some(id);
            }
        }
    }
    let signature = match signature {
        Some(signature) => signature,
        None if count_matches == 0 => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "function type is not committed",
            ));
        }
        None => {
            return Err(DiagnosticDraft::unsupported(
                "se-function of a non-M1 signature: canonical int result and parameters required",
            ));
        }
    };
    // The declared function symbol: exactly one symbol declared at this
    // declarator, typed with the checked signature. A missing or
    // divergently-typed symbol fails loudly; duplicates are a DEFECT.
    let mut symbol = None;
    for (id, sym) in bus.arenas.symbols.iter() {
        if sym.decl == declarator {
            if symbol.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "duplicate symbol for declarator",
                ));
            }
            symbol = Some((id, sym.clone()));
        }
    }
    let Some((_symbol_id, symbol_body)) = symbol else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "se-function reads undeclared function",
        ));
    };
    if symbol_body.kind != crate::bus::SymbolKind::Function {
        return Err(DiagnosticDraft::unsupported(
            "se-function with a non-function symbol",
        ));
    }
    match symbol_body.ty {
        Some(ty) if ty == signature => {}
        Some(_) => {
            return Err(DiagnosticDraft::unsupported(
                "se-function with a definition inconsistent with its prototype",
            ));
        }
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "se-function reads untyped symbol",
            ));
        }
    }
    // The body is checked, not re-checked: the single `Return` child must
    // already carry its committed `int`-typed `SemRecord` (the `Return`
    // node owns the return-role facts; this record carries the signature
    // only). A missing child fact is missing input; a mistyped one is
    // unsupported; a nonzero effect mask is a typed failure.
    let compound_body = bus
        .arenas
        .nodes
        .get(compound)
        .map_err(|_| missing("node", compound.index()))?;
    if compound_body.kind != NodeKind::Compound || compound_body.children.is_empty() {
        return Err(DiagnosticDraft::unsupported(
            "se-function of a non-M1 block body",
        ));
    }
    let Some(ret) = compound_body.children.last().copied() else {
        return Err(DiagnosticDraft::unsupported(
            "se-function of an empty block body",
        ));
    };
    if compound_body.children.len() > 1 {
        let declaration = compound_body.children[0];
        let declaration_body = bus
            .arenas
            .nodes
            .get(declaration)
            .map_err(|_| missing("node", declaration.index()))?;
        if declaration_body.kind != NodeKind::Declaration || declaration_body.children.len() != 3 {
            return Err(DiagnosticDraft::unsupported(
                "se-function of a non-covered declaration",
            ));
        }
        for statement in &compound_body.children[1..compound_body.children.len() - 1] {
            let statement_body = bus
                .arenas
                .nodes
                .get(*statement)
                .map_err(|_| missing("node", statement.index()))?;
            if statement_body.kind != NodeKind::Assignment {
                return Err(DiagnosticDraft::unsupported(
                    "se-function of a non-assignment statement",
                ));
            }
        }
    }
    let ret_body = bus
        .arenas
        .nodes
        .get(ret)
        .map_err(|_| missing("node", ret.index()))?;
    if ret_body.kind != NodeKind::Return {
        return Err(DiagnosticDraft::unsupported(
            "se-function of a non-return statement body",
        ));
    }
    match sem_for_node(bus, ret) {
        Some((_, sem)) if sem.ty == int && sem.category == ValueCategory::NonLvalue => {
            if sem.effects != EffectMask(0) {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-function reads effectful node {}", ret.index()),
                ));
            }
        }
        Some(_) => {
            return Err(DiagnosticDraft::unsupported(
                "se-function of a non-int checked return",
            ));
        }
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("se-function reads unchecked return {}", ret.index()),
            ));
        }
    }
    Ok(SeFunctionInput {
        task,
        state: record.state.clone(),
        node,
        signature,
        existing: sem_for_node(bus, node),
        sem_allocated: bus.arenas.sem.allocated(),
    })
}

/// The T07 function-definition worker (SE29 slice scope).
pub struct SeFuncChip;

impl Worker for SeFuncChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: SE_FUNC_CHIP,
            chip_name: "SeFuncChip",
            group: TaskGroup::SEMANTIC,
            task_kinds: vec![SE_FUNC_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Symbols, "symbols"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Sem, "records"),
            ],
            writes: vec![FieldPath::new(StoreId::Sem, "records")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c39_sefunc.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_se_function_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl SeFuncChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &SeFunctionInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("se-function task {} is not running", input.task.index()),
                ),
            )];
        }
        if let Some((existing, _)) = input.existing {
            return vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Sem(existing)),
            }];
        }
        let predicted = SemId::from_index(input.sem_allocated);
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
                        ty: input.signature,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::TaskId;

    fn input(state: TaskState, existing: Option<(SemId, SemRecord)>) -> SeFunctionInput {
        SeFunctionInput {
            task: TaskId::from_index(7),
            state,
            node: NodeId::from_index(1),
            signature: TypeId::from_index(3),
            existing,
            sem_allocated: 4,
        }
    }

    fn sem(node: NodeId, ty: TypeId) -> SemRecord {
        SemRecord {
            node,
            ty,
            category: ValueCategory::NonLvalue,
            effects: EffectMask(0),
        }
    }

    #[test]
    fn appends_signature_record_then_completes() {
        let proposals = SeFuncChip.compute(&input(TaskState::Running, None));
        assert_eq!(proposals.len(), 2);
        match &proposals[0] {
            Proposal::AppendRecords { task, batch } => {
                assert_eq!(*task, TaskId::from_index(7));
                assert_eq!(batch.records.len(), 1);
                assert_eq!(batch.bodies.len(), 1);
                match &batch.bodies[0] {
                    G1DraftBody::Sem(record) => {
                        assert_eq!(*record, sem(NodeId::from_index(1), TypeId::from_index(3)));
                    }
                    other => panic!("expected a Sem body, got {other:?}"),
                }
            }
            other => panic!("expected AppendRecords first, got {other:?}"),
        }
        match &proposals[1] {
            Proposal::Complete { task, value } => {
                assert_eq!(*task, TaskId::from_index(7));
                assert_eq!(
                    *value,
                    ResultValue::Record(RecordRef::Sem(SemId::from_index(4)))
                );
            }
            other => panic!("expected Complete second, got {other:?}"),
        }
    }

    #[test]
    fn reuses_committed_record_without_append() {
        let committed = SemId::from_index(2);
        let proposals = SeFuncChip.compute(&input(
            TaskState::Running,
            Some((committed, sem(NodeId::from_index(1), TypeId::from_index(3)))),
        ));
        assert_eq!(proposals.len(), 1);
        match &proposals[0] {
            Proposal::Complete { task, value } => {
                assert_eq!(*task, TaskId::from_index(7));
                assert_eq!(*value, ResultValue::Record(RecordRef::Sem(committed)));
            }
            other => panic!("expected a reusing Complete, got {other:?}"),
        }
    }

    #[test]
    fn non_running_task_fails() {
        use crate::ids::DiagnosticId;
        let states = [
            TaskState::Ready,
            TaskState::Failed(DiagnosticId::from_index(0)),
        ];
        for state in states {
            match &SeFuncChip.compute(&input(state.clone(), None))[0] {
                Proposal::Fail { task, .. } => assert_eq!(*task, TaskId::from_index(7)),
                other => panic!("expected Fail for {state:?}, got {other:?}"),
            }
        }
    }
}
