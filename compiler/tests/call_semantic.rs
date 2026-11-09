// ============================================================================
// call_semantic.rs — `/54` CALL-S1 direct-call semantic acceptance.
//
// Synthetic buses exercise the frozen direct-call schema without depending on
// the parser lane: parameterized signatures, parameter declarations, call
// facts, nested calls as operands, innermost-first shadowing, and the typed
// failure cases (arity, non-int/effectful arguments, object/unknown/forward
// callees, self-recursion, duplicate parameters). Values are hand-derived.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, EffectMask, IntRank, Linkage, NodeKind, NodeRecord, ScopeKind, ScopeRecord,
    SemRecord, StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord, ValueCategory,
    EFFECT_WRITES_LOCAL,
};
use cc_silicon_compiler::chips::{
    project_se_assignment_input, project_se_binary_input, project_se_call_input,
    project_se_function_input, project_se_identifier_input, project_se_return_input,
    project_se_unary_input, project_ty_declare_input, project_ty_type_input, SeAssignmentChip,
    SeBinChip, SeCallChip, SeFuncChip, SeIdentifierChip, SeRetChip, SeUnaryChip, TySymbolChip,
    TyTypeChip, Worker,
};
use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticDraft};
use cc_silicon_compiler::ids::{
    ChipId, NameId, NodeId, RecordRef, ScopeId, SemId, SymbolId, TaskId, TokenId, TypeId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    BackendClass, Capability, ChipPhase, FieldPath, SE_CALL_CHIP, STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::records::G1DraftBody;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskKind, TaskState,
};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn alloc_type(bus: &mut CompilerBus, kind: TypeKind) -> TypeId {
    let limits = bus.limits();
    bus.arenas
        .types
        .alloc(TypeRecord { kind }, &limits)
        .unwrap()
}

fn alloc_node(bus: &mut CompilerBus, kind: NodeKind, name: Option<NameId>) -> NodeId {
    let limits = bus.limits();
    bus.arenas
        .nodes
        .alloc(
            NodeRecord {
                kind,
                parent: None,
                children: Vec::new(),
                first_token: TokenId::from_index(0),
                last_token: TokenId::from_index(0),
                name,
                literal: None,
            },
            &limits,
        )
        .unwrap()
}

fn attach(bus: &mut CompilerBus, parent: NodeId, children: Vec<NodeId>) {
    for child in &children {
        bus.arenas.nodes.get_mut(*child).unwrap().parent = Some(parent);
    }
    bus.arenas.nodes.get_mut(parent).unwrap().children = children;
}

fn set_parent(bus: &mut CompilerBus, node: NodeId, parent: NodeId) {
    bus.arenas.nodes.get_mut(node).unwrap().parent = Some(parent);
}

fn alloc_scope(
    bus: &mut CompilerBus,
    kind: ScopeKind,
    parent: Option<ScopeId>,
    owner: Option<NodeId>,
) -> ScopeId {
    let limits = bus.limits();
    bus.arenas
        .scopes
        .alloc(
            ScopeRecord {
                kind,
                parent,
                owner,
            },
            &limits,
        )
        .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn alloc_symbol(
    bus: &mut CompilerBus,
    name: NameId,
    scope: ScopeId,
    kind: SymbolKind,
    ty: TypeId,
    linkage: Linkage,
    storage: StorageDuration,
    decl: NodeId,
) -> SymbolId {
    let limits = bus.limits();
    bus.arenas
        .symbols
        .alloc(
            SymbolRecord {
                name,
                scope,
                kind,
                ty: Some(ty),
                linkage,
                storage,
                decl,
            },
            &limits,
        )
        .unwrap()
}

fn alloc_sem(
    bus: &mut CompilerBus,
    node: NodeId,
    ty: TypeId,
    category: ValueCategory,
    effects: EffectMask,
) -> SemId {
    let limits = bus.limits();
    bus.arenas
        .sem
        .alloc(
            SemRecord {
                node,
                ty,
                category,
                effects,
            },
            &limits,
        )
        .unwrap()
}

fn seed_int_sem(bus: &mut CompilerBus, int: TypeId, node: NodeId) {
    alloc_sem(
        bus,
        node,
        int,
        ValueCategory::NonLvalue,
        EffectMask(0),
    );
}

fn running_task(
    bus: &mut CompilerBus,
    kind: TaskKind,
    owner: ChipId,
    refs: Vec<RecordRef>,
) -> TaskId {
    let task = bus
        .bootstrap_task(TaskDraft {
            kind,
            owner,
            parent: None,
            payload: Payload::from_refs(refs),
            continuation: None,
        })
        .unwrap();
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    task
}

fn project_call(
    bus: &mut CompilerBus,
    node: NodeId,
) -> Result<cc_silicon_compiler::chips::SeCallInput, DiagnosticDraft> {
    let task = running_task(
        bus,
        TaskKind::SEMANTIC_CALL_EXPR,
        SE_CALL_CHIP,
        vec![RecordRef::Node(node)],
    );
    project_se_call_input(bus, task)
}

fn proposed_sem(proposals: &[Proposal]) -> SemRecord {
    match &proposals[0] {
        Proposal::AppendRecords { batch, .. } => match &batch.bodies[0] {
            G1DraftBody::Sem(record) => record.clone(),
            other => panic!("expected a Sem body, got {other:?}"),
        },
        other => panic!("expected AppendRecords, got {other:?}"),
    }
}

fn proposed_type(proposals: &[Proposal]) -> TypeRecord {
    match &proposals[0] {
        Proposal::AppendRecords { batch, .. } => match &batch.bodies[0] {
            G1DraftBody::Type(record) => record.clone(),
            other => panic!("expected a Type body, got {other:?}"),
        },
        other => panic!("expected AppendRecords, got {other:?}"),
    }
}

fn proposed_symbol(proposals: &[Proposal]) -> SymbolRecord {
    match &proposals[0] {
        Proposal::AppendRecords { batch, .. } => match &batch.bodies[0] {
            G1DraftBody::Symbol(record) => record.clone(),
            other => panic!("expected a Symbol body, got {other:?}"),
        },
        other => panic!("expected AppendRecords, got {other:?}"),
    }
}

fn predicted_sem(proposals: &[Proposal]) -> SemId {
    match proposals.last() {
        Some(Proposal::Complete {
            value: ResultValue::Record(RecordRef::Sem(id)),
            ..
        }) => *id,
        other => panic!("expected a Sem Complete, got {other:?}"),
    }
}

fn predicted_symbol(proposals: &[Proposal]) -> SymbolId {
    match proposals.last() {
        Some(Proposal::Complete {
            value: ResultValue::Record(RecordRef::Symbol(id)),
            ..
        }) => *id,
        other => panic!("expected a Symbol Complete, got {other:?}"),
    }
}

fn has_sem_for(bus: &CompilerBus, node: NodeId) -> bool {
    bus.arenas.sem.iter().any(|(_, sem)| sem.node == node)
}

fn failure(proposals: &[Proposal]) -> &DiagnosticDraft {
    match &proposals[0] {
        Proposal::Fail { diagnostic, .. } => diagnostic,
        other => panic!("expected Fail, got {other:?}"),
    }
}

/// Two source-order functions:
///
/// ```c
/// int add(int a, int b) { return a + b; }
/// int main(void) { return add(2, 3); }
/// ```
struct Model {
    bus: CompilerBus,
    int: TypeId,
    func_two: TypeId,
    func_void: TypeId,
    file: ScopeId,
    add_block: ScopeId,
    main_block: ScopeId,
    add_funcdef: NodeId,
    add_compound: NodeId,
    add_param_a: NodeId,
    add_param_b: NodeId,
    add_return: NodeId,
    add_id_a: NodeId,
    add_symbol: SymbolId,
    main_funcdef: NodeId,
    main_compound: NodeId,
    main_return: NodeId,
    call: NodeId,
    callee: NodeId,
    arg_a: NodeId,
    arg_b: NodeId,
    add_name: NameId,
    a_name: NameId,
    b_name: NameId,
    x_name: NameId,
}

fn build_model() -> Model {
    let mut bus = new_bus();
    let int = alloc_type(
        &mut bus,
        TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        },
    );
    let func_two = alloc_type(
        &mut bus,
        TypeKind::Function {
            result: int,
            params: vec![int, int],
            prototype: true,
            variadic: false,
        },
    );
    let func_void = alloc_type(
        &mut bus,
        TypeKind::Function {
            result: int,
            params: vec![],
            prototype: true,
            variadic: false,
        },
    );
    let add_name = bus.intern_name(b"add").unwrap();
    let main_name = bus.intern_name(b"main").unwrap();
    let a_name = bus.intern_name(b"a").unwrap();
    let b_name = bus.intern_name(b"b").unwrap();
    let x_name = bus.intern_name(b"x").unwrap();

    // Parse-order node IDs: earlier definitions have lower IDs.
    let tu = alloc_node(&mut bus, NodeKind::TranslationUnit, None);
    let add_funcdef = alloc_node(&mut bus, NodeKind::FunctionDefinition, None);
    let add_spec = alloc_node(&mut bus, NodeKind::Specifiers, None);
    let add_decl = alloc_node(&mut bus, NodeKind::Declarator, Some(add_name));
    let add_param_list = alloc_node(&mut bus, NodeKind::ParameterList, None);
    let add_param_a = alloc_node(&mut bus, NodeKind::Parameter, Some(a_name));
    let add_param_b = alloc_node(&mut bus, NodeKind::Parameter, Some(b_name));
    let add_compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let add_return = alloc_node(&mut bus, NodeKind::Return, None);
    let add_binary = alloc_node(&mut bus, NodeKind::BinaryAdd, None);
    let add_id_a = alloc_node(&mut bus, NodeKind::Identifier, Some(a_name));
    let add_id_b = alloc_node(&mut bus, NodeKind::Identifier, Some(b_name));
    let main_funcdef = alloc_node(&mut bus, NodeKind::FunctionDefinition, None);
    let main_spec = alloc_node(&mut bus, NodeKind::Specifiers, None);
    let main_decl = alloc_node(&mut bus, NodeKind::Declarator, Some(main_name));
    let main_compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let main_return = alloc_node(&mut bus, NodeKind::Return, None);
    let call = alloc_node(&mut bus, NodeKind::Call, None);
    let callee = alloc_node(&mut bus, NodeKind::Identifier, Some(add_name));
    let arg_a = alloc_node(&mut bus, NodeKind::IntLiteral, None);
    let arg_b = alloc_node(&mut bus, NodeKind::IntLiteral, None);

    attach(&mut bus, tu, vec![add_funcdef, main_funcdef]);
    attach(
        &mut bus,
        add_funcdef,
        vec![add_spec, add_decl, add_compound],
    );
    attach(&mut bus, add_decl, vec![add_param_list]);
    attach(&mut bus, add_param_list, vec![add_param_a, add_param_b]);
    attach(&mut bus, add_compound, vec![add_return]);
    attach(&mut bus, add_return, vec![add_binary]);
    attach(&mut bus, add_binary, vec![add_id_a, add_id_b]);
    attach(
        &mut bus,
        main_funcdef,
        vec![main_spec, main_decl, main_compound],
    );
    attach(&mut bus, main_compound, vec![main_return]);
    attach(&mut bus, main_return, vec![call]);
    attach(&mut bus, call, vec![callee, arg_a, arg_b]);

    let file = alloc_scope(&mut bus, ScopeKind::File, None, None);
    let add_block = alloc_scope(&mut bus, ScopeKind::Block, Some(file), Some(add_compound));
    let main_block = alloc_scope(
        &mut bus,
        ScopeKind::Block,
        Some(file),
        Some(main_compound),
    );
    let add_symbol = alloc_symbol(
        &mut bus,
        add_name,
        file,
        SymbolKind::Function,
        func_two,
        Linkage::External,
        StorageDuration::Static,
        add_decl,
    );
    alloc_symbol(
        &mut bus,
        main_name,
        file,
        SymbolKind::Function,
        func_void,
        Linkage::External,
        StorageDuration::Static,
        main_decl,
    );

    Model {
        bus,
        int,
        func_two,
        func_void,
        file,
        add_block,
        main_block,
        add_funcdef,
        add_compound,
        add_param_a,
        add_param_b,
        add_return,
        add_id_a,
        add_symbol,
        main_funcdef,
        main_compound,
        main_return,
        call,
        callee,
        arg_a,
        arg_b,
        add_name,
        a_name,
        b_name,
        x_name,
    }
}

#[test]
fn function_type_projection_builds_parameter_list_and_reuses() {
    let mut bus = new_bus();
    let int = alloc_type(
        &mut bus,
        TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        },
    );
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_FUNC_TYPE,
        cc_silicon_compiler::manifest::TY_TYPE_CHIP,
        vec![
            RecordRef::Type(int),
            RecordRef::Type(int),
            RecordRef::Type(int),
        ],
    );
    let input = project_ty_type_input(&bus, task).unwrap();
    assert_eq!(input.params, vec![int, int]);
    let proposals = TyTypeChip.compute(&input);
    assert_eq!(proposals.len(), 2);
    let body = proposed_type(&proposals);
    match &body.kind {
        TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        } => {
            assert_eq!(*result, int);
            assert_eq!(params, &vec![int, int]);
            assert!(*prototype);
            assert!(!*variadic);
        }
        other => panic!("expected a function type, got {other:?}"),
    }
    let limits = bus.limits();
    let appended = bus.arenas.types.alloc(body, &limits).unwrap();
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_FUNC_TYPE,
        cc_silicon_compiler::manifest::TY_TYPE_CHIP,
        vec![
            RecordRef::Type(int),
            RecordRef::Type(int),
            RecordRef::Type(int),
        ],
    );
    let input = project_ty_type_input(&bus, task).unwrap();
    assert_eq!(input.existing, Some(appended));
    let proposals = TyTypeChip.compute(&input);
    assert_eq!(proposals.len(), 1);
    match &proposals[0] {
        Proposal::Complete {
            value: ResultValue::Record(RecordRef::Type(id)),
            ..
        } => assert_eq!(*id, appended),
        other => panic!("expected a reuse Complete, got {other:?}"),
    }
    // The boundary is 8 parameters; a ninth parameter type is rejected.
    let mut nine = vec![RecordRef::Type(int)];
    nine.extend(std::iter::repeat_n(RecordRef::Type(int), 9));
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_FUNC_TYPE,
        cc_silicon_compiler::manifest::TY_TYPE_CHIP,
        nine,
    );
    let error = project_ty_type_input(&bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Protocol);
}

#[test]
fn parameter_declaration_is_automatic_int_object_and_duplicate_fails() {
    let mut bus = new_bus();
    let int = alloc_type(
        &mut bus,
        TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        },
    );
    let compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let file = alloc_scope(&mut bus, ScopeKind::File, None, None);
    let block = alloc_scope(&mut bus, ScopeKind::Block, Some(file), Some(compound));
    let a_name = bus.intern_name(b"a").unwrap();
    let parameter = alloc_node(&mut bus, NodeKind::Parameter, Some(a_name));
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        cc_silicon_compiler::manifest::TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(parameter),
            RecordRef::Type(int),
            RecordRef::Scope(block),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert_eq!(input.symbol_kind, SymbolKind::Object);
    assert_eq!(input.linkage, Linkage::None);
    assert_eq!(input.storage, StorageDuration::Automatic);
    assert_eq!(input.ty, int);
    assert!(!input.conflict);
    let proposals = TySymbolChip.compute_declare(&input);
    let body = proposed_symbol(&proposals);
    assert_eq!(body.name, a_name);
    assert_eq!(body.scope, block);
    assert_eq!(body.kind, SymbolKind::Object);
    assert_eq!(body.ty, Some(int));
    assert_eq!(body.linkage, Linkage::None);
    assert_eq!(body.storage, StorageDuration::Automatic);
    assert_eq!(body.decl, parameter);
    let predicted = predicted_symbol(&proposals);
    let limits = bus.limits();
    let appended = bus.arenas.symbols.alloc(body, &limits).unwrap();
    assert_eq!(appended, predicted);

    // A duplicate parameter name in the same block is a typed conflict.
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        cc_silicon_compiler::manifest::TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(parameter),
            RecordRef::Type(int),
            RecordRef::Scope(block),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(input.conflict);
    let proposals = TySymbolChip.compute_declare(&input);
    assert_eq!(failure(&proposals).code.group, DiagGroup::Semantic);
    assert_eq!(bus.arenas.symbols.allocated(), 1);
}

#[test]
fn call_commits_one_int_fact_without_callee_fact() {
    let mut model = build_model();
    seed_int_sem(&mut model.bus, model.int, model.arg_a);
    seed_int_sem(&mut model.bus, model.int, model.arg_b);
    let callee = model.callee;
    let call = model.call;
    let int = model.int;
    let func_two = model.func_two;
    let add_symbol = model.add_symbol;
    let sem_before = model.bus.arenas.sem.allocated();
    let input = project_call(&mut model.bus, call).unwrap();
    assert_eq!(input.callee, add_symbol);
    assert_eq!(input.callee_ty, func_two);
    assert_eq!(input.int, int);
    assert!(input.existing.is_none());
    assert!(!has_sem_for(&model.bus, callee));
    let proposals = SeCallChip.compute(&input);
    assert_eq!(proposals.len(), 2);
    let sem = proposed_sem(&proposals);
    assert_eq!(sem.node, call);
    assert_eq!(sem.ty, int);
    assert_eq!(sem.category, ValueCategory::NonLvalue);
    assert_eq!(sem.effects, EffectMask(0));
    let predicted = predicted_sem(&proposals);
    let limits = model.bus.limits();
    let appended = model.bus.arenas.sem.alloc(sem, &limits).unwrap();
    assert_eq!(appended, predicted);
    assert_eq!(model.bus.arenas.sem.allocated(), sem_before + 1);
    assert!(!has_sem_for(&model.bus, callee));
}

#[test]
fn nested_call_and_expression_operands_are_checked() {
    let mut model = build_model();
    seed_int_sem(&mut model.bus, model.int, model.arg_a);
    seed_int_sem(&mut model.bus, model.int, model.arg_b);

    // inner = add(2, 3)
    let inner_arg_a = alloc_node(&mut model.bus, NodeKind::IntLiteral, None);
    let inner_arg_b = alloc_node(&mut model.bus, NodeKind::IntLiteral, None);
    let inner = alloc_node(&mut model.bus, NodeKind::Call, None);
    let callee = model.callee;
    attach(
        &mut model.bus,
        inner,
        vec![callee, inner_arg_a, inner_arg_b],
    );
    set_parent(&mut model.bus, inner, model.main_compound);
    seed_int_sem(&mut model.bus, model.int, inner_arg_a);
    seed_int_sem(&mut model.bus, model.int, inner_arg_b);
    let inner_input = project_call(&mut model.bus, inner).unwrap();
    let inner_sem = proposed_sem(&SeCallChip.compute(&inner_input));
    let inner_id = model.bus.arenas.sem.alloc(inner_sem, &model.bus.limits()).unwrap();

    // outer = add(inner, 2)
    let outer_arg = alloc_node(&mut model.bus, NodeKind::IntLiteral, None);
    seed_int_sem(&mut model.bus, model.int, outer_arg);
    let outer = alloc_node(&mut model.bus, NodeKind::Call, None);
    attach(&mut model.bus, outer, vec![callee, inner, outer_arg]);
    set_parent(&mut model.bus, outer, model.main_compound);
    let outer_input = project_call(&mut model.bus, outer).unwrap();
    let outer_sem = proposed_sem(&SeCallChip.compute(&outer_input));
    let outer_id = model.bus.arenas.sem.alloc(outer_sem, &model.bus.limits()).unwrap();

    // A checked call is a valid binary operand.
    let binary = alloc_node(&mut model.bus, NodeKind::BinaryAdd, None);
    attach(&mut model.bus, binary, vec![inner, outer]);
    set_parent(&mut model.bus, binary, model.main_compound);
    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_BINARY_EXPR,
        cc_silicon_compiler::manifest::SE_BIN_CHIP,
        vec![RecordRef::Node(binary)],
    );
    let input = project_se_binary_input(&model.bus, task).unwrap();
    assert!(input.leaves.is_empty());
    let proposals = SeBinChip.compute(&input);
    assert_eq!(proposed_sem(&proposals).node, binary);

    // A checked call is a valid unary operand.
    let unary = alloc_node(&mut model.bus, NodeKind::UnaryMinus, None);
    attach(&mut model.bus, unary, vec![outer]);
    set_parent(&mut model.bus, unary, model.main_compound);
    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_UNARY_EXPR,
        cc_silicon_compiler::manifest::SE_UNARY_CHIP,
        vec![RecordRef::Node(unary)],
    );
    let input = project_se_unary_input(&model.bus, task).unwrap();
    assert_eq!(input.operand, outer);
    assert_eq!(proposed_sem(&SeUnaryChip.compute(&input)).node, unary);

    // A checked call is a valid assignment rhs.
    let local_decl = alloc_node(&mut model.bus, NodeKind::Declarator, Some(model.x_name));
    alloc_symbol(
        &mut model.bus,
        model.x_name,
        model.main_block,
        SymbolKind::Object,
        model.int,
        Linkage::None,
        StorageDuration::Automatic,
        local_decl,
    );
    let lhs = alloc_node(&mut model.bus, NodeKind::Identifier, Some(model.x_name));
    let assignment = alloc_node(&mut model.bus, NodeKind::Assignment, None);
    attach(&mut model.bus, assignment, vec![lhs, outer]);
    set_parent(&mut model.bus, assignment, model.main_compound);
    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_ASSIGNMENT_EXPR,
        cc_silicon_compiler::manifest::SE_ASSIGN_CHIP,
        vec![RecordRef::Node(assignment)],
    );
    let input = project_se_assignment_input(&model.bus, task).unwrap();
    let assignment_sem = proposed_sem(&SeAssignmentChip.compute(&input));
    assert_eq!(assignment_sem.node, assignment);
    assert_eq!(
        assignment_sem.effects,
        EffectMask(EFFECT_WRITES_LOCAL)
    );

    // A checked call is a valid return operand.
    attach(&mut model.bus, model.main_return, vec![outer]);
    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_RETURN_STMT,
        cc_silicon_compiler::manifest::SE_RET_CHIP,
        vec![RecordRef::Node(model.main_return)],
    );
    let input = project_se_return_input(&model.bus, task).unwrap();
    assert_eq!(input.return_ty, model.int);
    let ret_sem = proposed_sem(&SeRetChip.compute(&input));
    assert_eq!(ret_sem.node, model.main_return);
    assert_eq!(ret_sem.ty, model.int);
    // Both nested facts are distinct and committed once each.
    assert_ne!(inner_id, outer_id);
    assert_eq!(
        model
            .bus
            .arenas
            .sem
            .iter()
            .filter(|(_, sem)| sem.node == inner)
            .count(),
        1
    );
}

#[test]
fn inner_object_shadows_the_file_function() {
    let mut model = build_model();
    seed_int_sem(&mut model.bus, model.int, model.arg_a);
    seed_int_sem(&mut model.bus, model.int, model.arg_b);
    let decl = alloc_node(&mut model.bus, NodeKind::Declarator, Some(model.add_name));
    alloc_symbol(
        &mut model.bus,
        model.add_name,
        model.main_block,
        SymbolKind::Object,
        model.int,
        Linkage::None,
        StorageDuration::Automatic,
        decl,
    );
    let sem_before = model.bus.arenas.sem.allocated();
    let error = project_call(&mut model.bus, model.call).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(error.message.contains("object"), "{}", error.message);
    assert_eq!(model.bus.arenas.sem.allocated(), sem_before);
}

#[test]
fn arity_mismatch_is_rejected() {
    let mut model = build_model();
    seed_int_sem(&mut model.bus, model.int, model.arg_a);
    let callee = model.callee;
    let arg_a = model.arg_a;
    attach(&mut model.bus, model.call, vec![callee, arg_a]);
    let error = project_call(&mut model.bus, model.call).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(error.message.contains("arity"), "{}", error.message);
}

#[test]
fn non_int_argument_is_rejected() {
    let mut model = build_model();
    let void_ty = alloc_type(&mut model.bus, TypeKind::Void);
    let arg_a = model.arg_a;
    alloc_sem(
        &mut model.bus,
        arg_a,
        void_ty,
        ValueCategory::NonLvalue,
        EffectMask(0),
    );
    seed_int_sem(&mut model.bus, model.int, model.arg_b);
    let error = project_call(&mut model.bus, model.call).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(
        error.message.contains("canonical int"),
        "{}",
        error.message
    );
}

#[test]
fn object_callee_is_rejected() {
    let mut model = build_model();
    seed_int_sem(&mut model.bus, model.int, model.arg_a);
    seed_int_sem(&mut model.bus, model.int, model.arg_b);
    let decl = alloc_node(&mut model.bus, NodeKind::Declarator, Some(model.x_name));
    alloc_symbol(
        &mut model.bus,
        model.x_name,
        model.main_block,
        SymbolKind::Object,
        model.int,
        Linkage::None,
        StorageDuration::Automatic,
        decl,
    );
    model.bus.arenas.nodes.get_mut(model.callee).unwrap().name = Some(model.x_name);
    let error = project_call(&mut model.bus, model.call).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(error.message.contains("object"), "{}", error.message);
}

#[test]
fn unknown_forward_and_self_calls_are_rejected() {
    // Unknown name.
    let mut model = build_model();
    seed_int_sem(&mut model.bus, model.int, model.arg_a);
    seed_int_sem(&mut model.bus, model.int, model.arg_b);
    let missing = model.bus.intern_name(b"missing").unwrap();
    model.bus.arenas.nodes.get_mut(model.callee).unwrap().name = Some(missing);
    let error = project_call(&mut model.bus, model.call).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(error.message.contains("undeclared"), "{}", error.message);

    // A later definition (declarator node after the enclosing function) is a
    // forward reference.
    let mut forward = build_model();
    seed_int_sem(&mut forward.bus, forward.int, forward.arg_a);
    seed_int_sem(&mut forward.bus, forward.int, forward.arg_b);
    let later_name = forward.bus.intern_name(b"later").unwrap();
    let later_decl = alloc_node(&mut forward.bus, NodeKind::Declarator, Some(later_name));
    alloc_symbol(
        &mut forward.bus,
        later_name,
        forward.file,
        SymbolKind::Function,
        forward.func_void,
        Linkage::External,
        StorageDuration::Static,
        later_decl,
    );
    forward.bus.arenas.nodes.get_mut(forward.callee).unwrap().name = Some(later_name);
    let error = project_call(&mut forward.bus, forward.call).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(
        error.message.contains("not defined before"),
        "{}",
        error.message
    );

    // Self-recursion: inside `add`, the callee's declarator is not before
    // the enclosing function definition node.
    let mut self_model = build_model();
    let self_arg_a = alloc_node(&mut self_model.bus, NodeKind::IntLiteral, None);
    let self_arg_b = alloc_node(&mut self_model.bus, NodeKind::IntLiteral, None);
    let self_callee = alloc_node(
        &mut self_model.bus,
        NodeKind::Identifier,
        Some(self_model.add_name),
    );
    let self_call = alloc_node(&mut self_model.bus, NodeKind::Call, None);
    attach(
        &mut self_model.bus,
        self_call,
        vec![self_callee, self_arg_a, self_arg_b],
    );
    set_parent(&mut self_model.bus, self_call, self_model.add_compound);
    seed_int_sem(&mut self_model.bus, self_model.int, self_arg_a);
    seed_int_sem(&mut self_model.bus, self_model.int, self_arg_b);
    let error = project_call(&mut self_model.bus, self_call).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(
        error.message.contains("not defined before"),
        "{}",
        error.message
    );
}

#[test]
fn duplicate_parameter_is_rejected() {
    // Declaration-level duplicate (same name, same block).
    let mut bus = new_bus();
    let int = alloc_type(
        &mut bus,
        TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        },
    );
    let compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let file = alloc_scope(&mut bus, ScopeKind::File, None, None);
    let block = alloc_scope(&mut bus, ScopeKind::Block, Some(file), Some(compound));
    let name = bus.intern_name(b"a").unwrap();
    let parameter = alloc_node(&mut bus, NodeKind::Parameter, Some(name));
    alloc_symbol(
        &mut bus,
        name,
        block,
        SymbolKind::Object,
        int,
        Linkage::None,
        StorageDuration::Automatic,
        parameter,
    );
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        cc_silicon_compiler::manifest::TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(parameter),
            RecordRef::Type(int),
            RecordRef::Scope(block),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(input.conflict);

    // Function-definition-level duplicate parameter names.
    let mut model = build_model();
    let a_name = model.a_name;
    let add_return = model.add_return;
    model
        .bus
        .arenas
        .nodes
        .get_mut(model.add_param_b)
        .unwrap()
        .name = Some(a_name);
    seed_int_sem(&mut model.bus, model.int, add_return);
    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        cc_silicon_compiler::manifest::SE_FUNC_CHIP,
        vec![RecordRef::Node(model.add_funcdef)],
    );
    let error = project_se_function_input(&model.bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(error.message.contains("duplicate"), "{}", error.message);
}

#[test]
fn effectful_argument_is_rejected() {
    let mut model = build_model();
    let arg_a = model.arg_a;
    let int = model.int;
    alloc_sem(
        &mut model.bus,
        arg_a,
        int,
        ValueCategory::NonLvalue,
        EffectMask(EFFECT_WRITES_LOCAL),
    );
    seed_int_sem(&mut model.bus, model.int, model.arg_b);
    let error = project_call(&mut model.bus, model.call).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(error.message.contains("side effects"), "{}", error.message);
}

#[test]
fn function_definition_selects_signature_by_parameter_count() {
    let mut model = build_model();
    let add_return = model.add_return;
    let main_return = model.main_return;
    seed_int_sem(&mut model.bus, model.int, add_return);
    seed_int_sem(&mut model.bus, model.int, main_return);

    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        cc_silicon_compiler::manifest::SE_FUNC_CHIP,
        vec![RecordRef::Node(model.add_funcdef)],
    );
    let input = project_se_function_input(&model.bus, task).unwrap();
    assert_eq!(input.signature, model.func_two);
    let sem = proposed_sem(&SeFuncChip.compute(&input));
    assert_eq!(sem.node, model.add_funcdef);
    assert_eq!(sem.ty, model.func_two);
    assert_eq!(sem.category, ValueCategory::NonLvalue);
    assert_eq!(sem.effects, EffectMask(0));

    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        cc_silicon_compiler::manifest::SE_FUNC_CHIP,
        vec![RecordRef::Node(model.main_funcdef)],
    );
    let input = project_se_function_input(&model.bus, task).unwrap();
    assert_eq!(input.signature, model.func_void);
}

#[test]
fn identifier_and_assignment_use_the_enclosing_block_scope() {
    let mut model = build_model();
    let a_name = model.a_name;
    let b_name = model.b_name;
    let int = model.int;
    let add_block = model.add_block;
    let add_param_a = model.add_param_a;
    let add_param_b = model.add_param_b;
    alloc_symbol(
        &mut model.bus,
        a_name,
        add_block,
        SymbolKind::Object,
        int,
        Linkage::None,
        StorageDuration::Automatic,
        add_param_a,
    );
    alloc_symbol(
        &mut model.bus,
        b_name,
        add_block,
        SymbolKind::Object,
        int,
        Linkage::None,
        StorageDuration::Automatic,
        add_param_b,
    );
    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_IDENTIFIER_EXPR,
        cc_silicon_compiler::manifest::SE_ID_CHIP,
        vec![RecordRef::Node(model.add_id_a)],
    );
    let input = project_se_identifier_input(&model.bus, task).unwrap();
    assert_eq!(input.int, int);
    let sem = proposed_sem(&SeIdentifierChip.compute(&input));
    assert_eq!(sem.node, model.add_id_a);

    // A local in the second function body resolves there, not in `add`'s
    // block scope: multiple block scopes are independent.
    let local_decl = alloc_node(&mut model.bus, NodeKind::Declarator, Some(model.x_name));
    alloc_symbol(
        &mut model.bus,
        model.x_name,
        model.main_block,
        SymbolKind::Object,
        int,
        Linkage::None,
        StorageDuration::Automatic,
        local_decl,
    );
    let id_x = alloc_node(&mut model.bus, NodeKind::Identifier, Some(model.x_name));
    set_parent(&mut model.bus, id_x, model.main_compound);
    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_IDENTIFIER_EXPR,
        cc_silicon_compiler::manifest::SE_ID_CHIP,
        vec![RecordRef::Node(id_x)],
    );
    let input = project_se_identifier_input(&model.bus, task).unwrap();
    let sem = proposed_sem(&SeIdentifierChip.compute(&input));
    assert_eq!(sem.node, id_x);

    // Assignment uses the same enclosing-scope resolution for its lhs.
    let assignment = alloc_node(&mut model.bus, NodeKind::Assignment, None);
    attach(&mut model.bus, assignment, vec![id_x, model.arg_a]);
    set_parent(&mut model.bus, assignment, model.main_compound);
    seed_int_sem(&mut model.bus, model.int, model.arg_a);
    let task = running_task(
        &mut model.bus,
        TaskKind::SEMANTIC_ASSIGNMENT_EXPR,
        cc_silicon_compiler::manifest::SE_ASSIGN_CHIP,
        vec![RecordRef::Node(assignment)],
    );
    let input = project_se_assignment_input(&model.bus, task).unwrap();
    let sem = proposed_sem(&SeAssignmentChip.compute(&input));
    assert_eq!(sem.node, assignment);
    assert_eq!(sem.effects, EffectMask(EFFECT_WRITES_LOCAL));
}

#[test]
fn call_fact_reuses_and_replays_deterministically() {
    let run = || {
        let mut model = build_model();
        seed_int_sem(&mut model.bus, model.int, model.arg_a);
        seed_int_sem(&mut model.bus, model.int, model.arg_b);
        let input = project_call(&mut model.bus, model.call).unwrap();
        let proposals = SeCallChip.compute(&input);
        (input.callee, input.callee_ty, proposed_sem(&proposals))
    };
    assert_eq!(run(), run());

    let mut model = build_model();
    seed_int_sem(&mut model.bus, model.int, model.arg_a);
    seed_int_sem(&mut model.bus, model.int, model.arg_b);
    let input = project_call(&mut model.bus, model.call).unwrap();
    let first = proposed_sem(&SeCallChip.compute(&input));
    let first_id = model
        .bus
        .arenas
        .sem
        .alloc(first.clone(), &model.bus.limits())
        .unwrap();
    let input = project_call(&mut model.bus, model.call).unwrap();
    assert_eq!(input.existing, Some((first_id, first)));
    let proposals = SeCallChip.compute(&input);
    assert_eq!(proposals.len(), 1);
    match &proposals[0] {
        Proposal::Complete {
            value: ResultValue::Record(RecordRef::Sem(id)),
            ..
        } => assert_eq!(*id, first_id),
        other => panic!("expected a reuse Complete, got {other:?}"),
    }
    // Two argument facts plus the reused call fact.
    assert_eq!(model.bus.arenas.sem.allocated(), 3);
}

#[test]
fn call_chip_manifest_is_write_scoped() {
    let manifest = SeCallChip.manifest();
    assert_eq!(manifest.id, SE_CALL_CHIP);
    assert_eq!(manifest.chip_name, "SeCallChip");
    assert_eq!(manifest.group, cc_silicon_compiler::task::TaskGroup::SEMANTIC);
    assert_eq!(
        manifest.task_kinds,
        vec![TaskKind::SEMANTIC_CALL_EXPR]
    );
    assert_eq!(
        manifest.writes,
        vec![FieldPath::new(StoreId::Sem, "records")]
    );
    assert_eq!(manifest.writes.len(), 1);
    assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
    assert!(manifest.declares_read(StoreId::Parse, "nodes"));
    assert!(manifest.declares_read(StoreId::Types, "records"));
    assert!(manifest.declares_read(StoreId::Symbols, "symbols"));
    assert!(manifest.declares_read(StoreId::Symbols, "scopes"));
    assert!(manifest.declares_read(StoreId::Sem, "records"));
    assert_eq!(manifest.capability, Capability::Emulable);
    assert_eq!(manifest.backend_class, BackendClass::CpuReference);
    assert_eq!(manifest.phase, ChipPhase::Propagation);
    assert!(manifest.deterministic);
    assert_eq!(manifest.tests, vec!["compiler/tests/call_semantic.rs"]);
    assert!(STORE_OWNER_ALLOWLIST.contains(&(
        SE_CALL_CHIP,
        StoreId::Sem,
        "records",
        TaskKind::SEMANTIC_CALL_EXPR,
    )));
    assert_eq!(
        cc_silicon_compiler::manifest::stage_of(TaskKind::SEMANTIC_CALL_EXPR),
        Some(4)
    );
    assert!(cc_silicon_compiler::manifest::is_se_slice_kind(
        TaskKind::SEMANTIC_CALL_EXPR
    ));
}
