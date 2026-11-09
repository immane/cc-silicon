// ============================================================================
// sprint_t05.rs — T05 adversarial regression: duplicate parameters,
// parameter/local collisions, and ordinary-namespace conflicts.
//
// Parameter scope, duplicate parameters, and lawful shadowing follow C11
// 6.2.1p4 and 6.7p3. Declaration conflicts follow the frozen `/55` T06 rule:
// the exact duplicate `(name, scope, kind)` in the ordinary namespace. The
// function-definition check independently rejects a repeated parameter
// spelling before any signature fact is appended. A same-spelling declaration
// in an outer scope remains lawful shadowing, not a conflict.
//
// The bus fixtures are constructed explicitly; no parser or host I/O is
// involved. Every symbol value and count below is hand-derived.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, EffectMask, IntRank, Linkage, NodeKind, NodeRecord, ScopeKind, ScopeRecord,
    SemRecord, StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{
    project_se_function_input, project_ty_declare_input, project_ty_lookup_input, SeFuncChip,
    TySymbolChip,
};
use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticDraft};
use cc_silicon_compiler::ids::{
    ChipId, NameId, NodeId, RecordRef, ScopeId, SymbolId, TaskId, TokenId, TypeId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{SE_FUNC_CHIP, TY_SYMBOL_CHIP};
use cc_silicon_compiler::records::G1DraftBody;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, Proposal, ResultValue, TaskDraft, TaskKind, TaskState};

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

fn alloc_int(bus: &mut CompilerBus) -> TypeId {
    alloc_type(
        bus,
        TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        },
    )
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

fn alloc_symbol(
    bus: &mut CompilerBus,
    name: NameId,
    scope: ScopeId,
    kind: SymbolKind,
    ty: TypeId,
    decl: NodeId,
) -> SymbolId {
    let (linkage, storage) = match kind {
        SymbolKind::Function => (Linkage::External, StorageDuration::Static),
        SymbolKind::Object => (Linkage::None, StorageDuration::Automatic),
    };
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

fn seed_int_sem(bus: &mut CompilerBus, node: NodeId, ty: TypeId) {
    let limits = bus.limits();
    bus.arenas
        .sem
        .alloc(
            SemRecord {
                node,
                ty,
                category: ValueCategory::NonLvalue,
                effects: EffectMask(0),
            },
            &limits,
        )
        .unwrap();
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

fn failure(proposals: &[Proposal]) -> &DiagnosticDraft {
    match &proposals[0] {
        Proposal::Fail { diagnostic, .. } => diagnostic,
        other => panic!("expected Fail, got {other:?}"),
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

fn completed_symbol(proposals: &[Proposal]) -> SymbolId {
    match proposals.last() {
        Some(Proposal::Complete {
            value: ResultValue::Record(RecordRef::Symbol(id)),
            ..
        }) => *id,
        other => panic!("expected a Symbol Complete, got {other:?}"),
    }
}

fn commit_symbol(bus: &mut CompilerBus, proposals: &[Proposal]) -> SymbolId {
    let body = proposed_symbol(proposals);
    let predicted = completed_symbol(proposals);
    let limits = bus.limits();
    let appended = bus.arenas.symbols.alloc(body, &limits).unwrap();
    assert_eq!(appended, predicted);
    appended
}

#[test]
fn duplicate_parameter_names_and_declarations_are_semantic_conflicts() {
    let mut bus = new_bus();
    let int = alloc_int(&mut bus);
    let signature = alloc_type(
        &mut bus,
        TypeKind::Function {
            result: int,
            params: vec![int, int],
            prototype: true,
            variadic: false,
        },
    );
    let f_name = bus.intern_name(b"f").unwrap();
    let a_name = bus.intern_name(b"a").unwrap();
    let b_name = bus.intern_name(b"b").unwrap();

    // `int f(int a, int b) { return 0; }` as an explicit AST fixture.
    let file = alloc_scope(&mut bus, ScopeKind::File, None, None);
    let funcdef = alloc_node(&mut bus, NodeKind::FunctionDefinition, None);
    let specifiers = alloc_node(&mut bus, NodeKind::Specifiers, None);
    let declarator = alloc_node(&mut bus, NodeKind::Declarator, Some(f_name));
    let parameter_list = alloc_node(&mut bus, NodeKind::ParameterList, None);
    let param_a = alloc_node(&mut bus, NodeKind::Parameter, Some(a_name));
    let param_b = alloc_node(&mut bus, NodeKind::Parameter, Some(b_name));
    let compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let ret = alloc_node(&mut bus, NodeKind::Return, None);
    attach(&mut bus, funcdef, vec![specifiers, declarator, compound]);
    attach(&mut bus, declarator, vec![parameter_list]);
    attach(&mut bus, parameter_list, vec![param_a, param_b]);
    attach(&mut bus, compound, vec![ret]);
    let block = alloc_scope(&mut bus, ScopeKind::Block, Some(file), Some(compound));
    alloc_symbol(
        &mut bus,
        f_name,
        file,
        SymbolKind::Function,
        signature,
        declarator,
    );
    seed_int_sem(&mut bus, ret, int);

    // Positive control: two distinct spellings produce the one signature fact.
    let task = running_task(
        &mut bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        vec![RecordRef::Node(funcdef)],
    );
    let input = project_se_function_input(&bus, task).unwrap();
    assert_eq!(input.signature, signature);
    let proposals = SeFuncChip.compute(&input);
    assert_eq!(proposals.len(), 2);
    match &proposals[0] {
        Proposal::AppendRecords { batch, .. } => match &batch.bodies[0] {
            G1DraftBody::Sem(record) => {
                assert_eq!(record.node, funcdef);
                assert_eq!(record.ty, signature);
                assert_eq!(record.category, ValueCategory::NonLvalue);
                assert_eq!(record.effects, EffectMask(0));
            }
            other => panic!("expected a Sem body, got {other:?}"),
        },
        other => panic!("expected AppendRecords, got {other:?}"),
    }

    // C11 6.2.1p4 gives definition parameters block scope; 6.7p3 forbids a
    // second same-scope declaration. Reject before a function fact exists.
    bus.arenas.nodes.get_mut(param_b).unwrap().name = Some(a_name);
    let task = running_task(
        &mut bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        vec![RecordRef::Node(funcdef)],
    );
    let error = project_se_function_input(&bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(
        error.message.contains("duplicate parameter name"),
        "{}",
        error.message
    );
    // The positive proposals above were inspected but never committed.
    assert_eq!(bus.arenas.sem.allocated(), 1);

    // Declaration-level duplicate: both parameters name `a` in the same block.
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(param_a),
            RecordRef::Type(int),
            RecordRef::Scope(block),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(!input.conflict);
    assert_eq!(input.symbol_kind, SymbolKind::Object);
    assert_eq!(input.storage, StorageDuration::Automatic);
    let first = commit_symbol(&mut bus, &TySymbolChip.compute_declare(&input));
    assert_eq!(first.index(), 1);

    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(param_b),
            RecordRef::Type(int),
            RecordRef::Scope(block),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(input.conflict);
    let duplicate_proposals = TySymbolChip.compute_declare(&input);
    let error = failure(&duplicate_proposals);
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(
        error.message.contains("redeclaration conflict"),
        "{}",
        error.message
    );
    // One function symbol plus the first parameter; the duplicate appends none.
    assert_eq!(bus.arenas.symbols.allocated(), 2);
}

#[test]
fn parameter_local_collision_is_rejected_only_within_the_same_block() {
    let mut bus = new_bus();
    let int = alloc_int(&mut bus);
    let file = alloc_scope(&mut bus, ScopeKind::File, None, None);
    let compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let block = alloc_scope(&mut bus, ScopeKind::Block, Some(file), Some(compound));
    let nested_compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let nested = alloc_scope(
        &mut bus,
        ScopeKind::Block,
        Some(block),
        Some(nested_compound),
    );
    let name = bus.intern_name(b"a").unwrap();
    let parameter = alloc_node(&mut bus, NodeKind::Parameter, Some(name));
    let same_block_local = alloc_node(&mut bus, NodeKind::Declarator, Some(name));
    let nested_local = alloc_node(&mut bus, NodeKind::Declarator, Some(name));

    // Declare the parameter `int f(int a)`.
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(parameter),
            RecordRef::Type(int),
            RecordRef::Scope(block),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(!input.conflict);
    assert_eq!(input.name, name);
    assert_eq!(input.scope, block);
    assert_eq!(input.symbol_kind, SymbolKind::Object);
    assert_eq!(input.linkage, Linkage::None);
    assert_eq!(input.storage, StorageDuration::Automatic);
    let parameter_symbol = commit_symbol(&mut bus, &TySymbolChip.compute_declare(&input));
    assert_eq!(parameter_symbol.index(), 0);

    // `int a;` in the function's outermost block is a redeclaration of the
    // parameter: C11 gives definition parameters the body block scope.
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(same_block_local),
            RecordRef::Type(int),
            RecordRef::Scope(block),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(input.conflict);
    let collision_proposals = TySymbolChip.compute_declare(&input);
    let error = failure(&collision_proposals);
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(
        error.message.contains("redeclaration conflict"),
        "{}",
        error.message
    );
    assert_eq!(bus.arenas.symbols.allocated(), 1);

    // A nested block may shadow the parameter; this is not a conflict.
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(nested_local),
            RecordRef::Type(int),
            RecordRef::Scope(nested),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(!input.conflict);
    assert_eq!(input.scope, nested);
    let proposals = TySymbolChip.compute_declare(&input);
    let body = proposed_symbol(&proposals);
    assert_eq!(body.name, name);
    assert_eq!(body.scope, nested);
    assert_eq!(body.kind, SymbolKind::Object);
    let shadow = commit_symbol(&mut bus, &proposals);
    assert_eq!(shadow.index(), 1);
}

#[test]
fn ordinary_namespace_duplicate_function_conflicts_and_inner_object_shadows() {
    let mut bus = new_bus();
    let int = alloc_int(&mut bus);
    let function_type = alloc_type(
        &mut bus,
        TypeKind::Function {
            result: int,
            params: vec![],
            prototype: true,
            variadic: false,
        },
    );
    let file = alloc_scope(&mut bus, ScopeKind::File, None, None);
    let compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let block = alloc_scope(&mut bus, ScopeKind::Block, Some(file), Some(compound));
    let f_name = bus.intern_name(b"f").unwrap();
    let first_decl = alloc_node(&mut bus, NodeKind::Declarator, Some(f_name));
    let duplicate_decl = alloc_node(&mut bus, NodeKind::Declarator, Some(f_name));
    let local_decl = alloc_node(&mut bus, NodeKind::Declarator, Some(f_name));

    // `int f(void);` in file scope.
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(first_decl),
            RecordRef::Type(function_type),
            RecordRef::Scope(file),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(!input.conflict);
    assert_eq!(input.symbol_kind, SymbolKind::Function);
    assert_eq!(input.linkage, Linkage::External);
    assert_eq!(input.storage, StorageDuration::Static);
    let function_symbol = commit_symbol(&mut bus, &TySymbolChip.compute_declare(&input));
    assert_eq!(function_symbol.index(), 0);

    // M1 has no compatible-redeclaration merging: a second Function at file
    // scope is an exact `(name, scope, kind)` ordinary-namespace duplicate.
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(duplicate_decl),
            RecordRef::Type(function_type),
            RecordRef::Scope(file),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(input.conflict);
    let duplicate_proposals = TySymbolChip.compute_declare(&input);
    let error = failure(&duplicate_proposals);
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(
        error.message.contains("redeclaration conflict"),
        "{}",
        error.message
    );
    assert_eq!(bus.arenas.symbols.allocated(), 1);

    // `int f;` in the body block is lawful shadowing, not a conflict.
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(local_decl),
            RecordRef::Type(int),
            RecordRef::Scope(block),
        ],
    );
    let input = project_ty_declare_input(&bus, task).unwrap();
    assert!(!input.conflict);
    assert_eq!(input.symbol_kind, SymbolKind::Object);
    assert_eq!(input.scope, block);
    let local_symbol = commit_symbol(&mut bus, &TySymbolChip.compute_declare(&input));
    assert_eq!(local_symbol.index(), 1);
    assert_ne!(local_symbol, function_symbol);
    assert_eq!(bus.arenas.symbols.allocated(), 2);

    // Both spellings are ordinary-namespace hits; lookup obeys the active
    // chain, so the block sees the object and the file sees the function.
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_LOOKUP,
        TY_SYMBOL_CHIP,
        vec![RecordRef::Scope(block), RecordRef::Name(f_name)],
    );
    let input = project_ty_lookup_input(&bus, task).unwrap();
    assert_eq!(input.chain, vec![block, file]);
    assert_eq!(
        completed_symbol(&TySymbolChip.compute_lookup(&input)),
        local_symbol
    );

    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_LOOKUP,
        TY_SYMBOL_CHIP,
        vec![RecordRef::Scope(file), RecordRef::Name(f_name)],
    );
    let input = project_ty_lookup_input(&bus, task).unwrap();
    assert_eq!(input.chain, vec![file]);
    assert_eq!(
        completed_symbol(&TySymbolChip.compute_lookup(&input)),
        function_symbol
    );
}
