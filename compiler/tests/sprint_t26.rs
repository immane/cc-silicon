// ============================================================================
// sprint_t26.rs — T26: malformed parameter symbols/types/scope and signature
// count mismatch.
//
// Three independent boundaries are checked:
//
//   * TY parameter declaration projection: a `Parameter` node is only a
//     canonical `int` automatic object in a block scope; function/void types,
//     file scope, and a non-leaf parameter are typed unsupported failures.
//   * SE function-definition projection: every parameter-list child must be a
//     named leaf `Parameter`, and the committed signature must match the
//     declarator's parameter count exactly; duplicate or non-M1 signatures
//     fail closed.
//   * VF12 validation: every `FunctionRecord.parameters` entry must resolve to
//     an automatic int object, and the committed signature arity must equal
//     the function's parameter count.
//
// Expected values follow the frozen `/55` shapes, not the implementation.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, ConstRecord, ConstValue, FunctionRecord, IntRank, IntegerConst, Linkage, NodeKind,
    NodeRecord, ScopeKind, ScopeRecord, StorageDuration, SymbolKind, SymbolRecord, TypeKind,
    TypeRecord, ValueRecord,
};
use cc_silicon_compiler::chips::{
    project_se_function_input, project_ty_declare_input, Vf12Chip, Vf12Input,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{
    BlockId, ChipId, ConstId, FunctionId, NameId, NodeId, RecordRef, ScopeId, SymbolId, TaskId,
    TokenId, TypeId, ValueId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{SE_FUNC_CHIP, TY_SYMBOL_CHIP};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, Proposal, TaskDraft, TaskKind, TaskState};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
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

fn canonical_int(bus: &mut CompilerBus) -> TypeId {
    alloc_type(
        bus,
        TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        },
    )
}

fn function_type(
    bus: &mut CompilerBus,
    result: TypeId,
    params: Vec<TypeId>,
    prototype: bool,
    variadic: bool,
) -> TypeId {
    alloc_type(
        bus,
        TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        },
    )
}

// ---------------------------------------------------------------------------
// TY parameter declaration shape.
// ---------------------------------------------------------------------------

#[test]
fn parameter_declaration_rejects_non_int_types_and_non_block_scope() {
    let mut bus = new_bus();
    let int = canonical_int(&mut bus);
    let void = alloc_type(&mut bus, TypeKind::Void);
    let function = function_type(&mut bus, int, vec![], true, false);
    let compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let file = alloc_scope(&mut bus, ScopeKind::File, None, None);
    let block = alloc_scope(&mut bus, ScopeKind::Block, Some(file), Some(compound));
    let name = bus.intern_name(b"p").unwrap();
    let parameter = alloc_node(&mut bus, NodeKind::Parameter, Some(name));

    // A parameter is neither a function object nor a file-scope object, and
    // `void` is not the canonical int type.
    for (ty, scope) in [(function, block), (int, file), (void, block)] {
        let expected = "parameters require a canonical int";
        let task = running_task(
            &mut bus,
            TaskKind::SYMBOL_DECLARE,
            TY_SYMBOL_CHIP,
            vec![
                RecordRef::Node(parameter),
                RecordRef::Type(ty),
                RecordRef::Scope(scope),
            ],
        );
        let error = project_ty_declare_input(&bus, task).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Unsupported);
        assert!(
            error.message.contains(expected),
            "expected {expected:?}, got {:?}",
            error.message
        );
    }

    // A parameter must be a leaf: children belong to a declarator, not a
    // parameter object.
    let child = alloc_node(&mut bus, NodeKind::IntLiteral, None);
    let with_child = alloc_node(&mut bus, NodeKind::Parameter, Some(name));
    attach(&mut bus, with_child, vec![child]);
    let task = running_task(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        vec![
            RecordRef::Node(with_child),
            RecordRef::Type(int),
            RecordRef::Scope(block),
        ],
    );
    let error = project_ty_declare_input(&bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Unsupported);
    assert!(error.message.contains("non-leaf parameter"));

    // Control: the same node with canonical int in the block scope projects
    // an automatic no-linkage object and appends nothing.
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
    assert_eq!(input.symbol_kind, SymbolKind::Object);
    assert_eq!(input.ty, int);
    assert_eq!(input.scope, block);
    assert_eq!(input.linkage, Linkage::None);
    assert_eq!(input.storage, StorageDuration::Automatic);
    assert!(!input.conflict);
    assert_eq!(bus.arenas.symbols.allocated(), 0);
}

// ---------------------------------------------------------------------------
// SE function-definition parameter-list shape.
// ---------------------------------------------------------------------------

struct FunctionAst {
    bus: CompilerBus,
    funcdef: NodeId,
    param_list: NodeId,
    params: Vec<NodeId>,
}

fn function_ast(parameter_count: usize) -> FunctionAst {
    let mut bus = new_bus();
    let tu = alloc_node(&mut bus, NodeKind::TranslationUnit, None);
    let funcdef = alloc_node(&mut bus, NodeKind::FunctionDefinition, None);
    let specifiers = alloc_node(&mut bus, NodeKind::Specifiers, None);
    let name = bus.intern_name(b"f").unwrap();
    let declarator = alloc_node(&mut bus, NodeKind::Declarator, Some(name));
    let param_list = alloc_node(&mut bus, NodeKind::ParameterList, None);
    let compound = alloc_node(&mut bus, NodeKind::Compound, None);
    let ret = alloc_node(&mut bus, NodeKind::Return, None);
    let mut params = Vec::new();
    for index in 0..parameter_count {
        let spelling = format!("p{index}");
        let name = bus.intern_name(spelling.as_bytes()).unwrap();
        params.push(alloc_node(&mut bus, NodeKind::Parameter, Some(name)));
    }
    attach(&mut bus, tu, vec![funcdef]);
    attach(&mut bus, funcdef, vec![specifiers, declarator, compound]);
    attach(&mut bus, declarator, vec![param_list]);
    attach(&mut bus, param_list, params.clone());
    attach(&mut bus, compound, vec![ret]);
    FunctionAst {
        bus,
        funcdef,
        param_list,
        params,
    }
}

fn function_task(bus: &mut CompilerBus, funcdef: NodeId) -> TaskId {
    running_task(
        bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        vec![RecordRef::Node(funcdef)],
    )
}

#[test]
fn function_definition_rejects_malformed_parameter_nodes() {
    // A parameter-list child that is not a `Parameter` node.
    let mut ast = function_ast(2);
    let first = ast.params[0];
    ast.bus.arenas.nodes.get_mut(first).unwrap().kind = NodeKind::Declarator;
    let task = function_task(&mut ast.bus, ast.funcdef);
    let error = project_se_function_input(&ast.bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Unsupported);
    assert!(error.message.contains("non-parameter"));

    // A `Parameter` with children is not a leaf parameter.
    let mut ast = function_ast(2);
    let child = alloc_node(&mut ast.bus, NodeKind::IntLiteral, None);
    let first = ast.params[0];
    attach(&mut ast.bus, first, vec![child]);
    let task = function_task(&mut ast.bus, ast.funcdef);
    let error = project_se_function_input(&ast.bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Unsupported);
    assert!(error.message.contains("non-parameter"));

    // A nameless parameter cannot bind an object.
    let mut ast = function_ast(2);
    let first = ast.params[0];
    ast.bus.arenas.nodes.get_mut(first).unwrap().name = None;
    let task = function_task(&mut ast.bus, ast.funcdef);
    let error = project_se_function_input(&ast.bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Task);
    assert!(error.message.contains("nameless parameter"));

    // The parameter list itself must stay a `ParameterList`.
    let mut ast = function_ast(2);
    ast.bus.arenas.nodes.get_mut(ast.param_list).unwrap().kind = NodeKind::Declarator;
    let task = function_task(&mut ast.bus, ast.funcdef);
    let error = project_se_function_input(&ast.bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Unsupported);
    assert!(error.message.contains("K&R"));
}

#[test]
fn function_definition_rejects_signature_count_mismatch() {
    // Two source parameters, but only int(void) and int(int) are committed:
    // no signature has the required arity, so this is missing input, not a
    // silently degraded zero-parameter definition.
    let mut ast = function_ast(2);
    let int = canonical_int(&mut ast.bus);
    function_type(&mut ast.bus, int, vec![], true, false);
    function_type(&mut ast.bus, int, vec![int], true, false);
    let task = function_task(&mut ast.bus, ast.funcdef);
    let error = project_se_function_input(&ast.bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Task);
    assert!(error.message.contains("function type is not committed"));

    // Arity two is committed, but one parameter type is `void`: the count
    // matches while the M1 int signature does not.
    let mut ast = function_ast(2);
    let int = canonical_int(&mut ast.bus);
    let void = alloc_type(&mut ast.bus, TypeKind::Void);
    function_type(&mut ast.bus, int, vec![int, void], true, false);
    let task = function_task(&mut ast.bus, ast.funcdef);
    let error = project_se_function_input(&ast.bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Unsupported);
    assert!(error.message.contains("non-M1 signature"));

    // A non-prototype or variadic arity-two type is also outside M1.
    for (prototype, variadic) in [(false, false), (true, true)] {
        let mut ast = function_ast(2);
        let int = canonical_int(&mut ast.bus);
        function_type(&mut ast.bus, int, vec![int, int], prototype, variadic);
        let task = function_task(&mut ast.bus, ast.funcdef);
        let error = project_se_function_input(&ast.bus, task).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Unsupported);
        assert!(error.message.contains("non-M1 signature"));
    }

    // Duplicate identical signatures for the same arity are ambiguous
    // protocol input, not a reason to select the first record.
    let mut ast = function_ast(2);
    let int = canonical_int(&mut ast.bus);
    function_type(&mut ast.bus, int, vec![int, int], true, false);
    function_type(&mut ast.bus, int, vec![int, int], true, false);
    let task = function_task(&mut ast.bus, ast.funcdef);
    let error = project_se_function_input(&ast.bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Task);
    assert!(error
        .message
        .contains("duplicate function type for this parameter count"));
}

// ---------------------------------------------------------------------------
// VF12 parameter-symbol and signature-arity validation.
// ---------------------------------------------------------------------------

fn parameter_symbol(
    ty: Option<TypeId>,
    kind: SymbolKind,
    storage: StorageDuration,
) -> SymbolRecord {
    SymbolRecord {
        name: NameId::from_index(0),
        scope: ScopeId::from_index(0),
        kind,
        ty,
        linkage: Linkage::None,
        storage,
        decl: NodeId::from_index(0),
    }
}

fn vf12_input(
    signature_params: Vec<TypeId>,
    signature_result: TypeId,
    signature_prototype: bool,
    signature_variadic: bool,
    function_parameters: Vec<SymbolId>,
    symbols: Vec<(SymbolId, SymbolRecord)>,
) -> Vf12Input {
    let int = TypeId::from_index(0);
    let signature = TypeId::from_index(1);
    let void = TypeId::from_index(2);
    let function = FunctionId::from_index(0);
    let function_body = FunctionRecord {
        symbol: SymbolId::from_index(9),
        signature,
        entry: BlockId::from_index(0),
        linkage: Linkage::External,
        parameters: function_parameters,
    };
    Vf12Input {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        function,
        function_body: function_body.clone(),
        blocks: Vec::new(),
        instructions: Vec::new(),
        ret_value: (ValueId::from_index(u32::MAX), ValueRecord { ty: int }),
        modeled: (
            ConstId::from_index(u32::MAX),
            ConstRecord {
                ty: int,
                value: ConstValue::Integer(IntegerConst::new(vec![0], false)),
                origins: Vec::new(),
            },
        ),
        int,
        consts: Vec::new(),
        functions: vec![(function, function_body)],
        all_blocks: Vec::new(),
        all_instructions: Vec::new(),
        values: Vec::new(),
        types: vec![
            (
                int,
                TypeRecord {
                    kind: TypeKind::Int {
                        rank: IntRank::Int,
                        signed: true,
                    },
                },
            ),
            (
                signature,
                TypeRecord {
                    kind: TypeKind::Function {
                        result: signature_result,
                        params: signature_params,
                        prototype: signature_prototype,
                        variadic: signature_variadic,
                    },
                },
            ),
            (
                void,
                TypeRecord {
                    kind: TypeKind::Void,
                },
            ),
        ],
        symbols,
    }
}

fn expect_vf12_failure(input: &Vf12Input, expected: &str) {
    match Vf12Chip.compute(input).as_slice() {
        [Proposal::Fail { diagnostic, .. }] => {
            assert_eq!(
                diagnostic.code.group,
                DiagGroup::Task,
                "wrong diagnostic group: {}",
                diagnostic.message
            );
            assert!(
                diagnostic.message.contains(expected),
                "expected {expected:?}, got {:?}",
                diagnostic.message
            );
        }
        other => panic!("expected one typed failure, got {other:?}"),
    }
}

#[test]
fn vf12_rejects_signature_parameter_count_mismatch() {
    let int = TypeId::from_index(0);
    let parameter = SymbolId::from_index(0);
    // Signature: int(int, int); function body: one parameter -> mismatch.
    let input = vf12_input(
        vec![int, int],
        int,
        true,
        false,
        vec![parameter],
        vec![(
            parameter,
            parameter_symbol(Some(int), SymbolKind::Object, StorageDuration::Automatic),
        )],
    );
    expect_vf12_failure(&input, "signature does not match its parameters");
}

#[test]
fn vf12_rejects_signature_parameter_type_malformations() {
    let int = TypeId::from_index(0);
    let void = TypeId::from_index(2);
    let parameter = SymbolId::from_index(0);
    let symbols = vec![(
        parameter,
        parameter_symbol(Some(int), SymbolKind::Object, StorageDuration::Automatic),
    )];

    // A committed `void` parameter type is not canonical int.
    let input = vf12_input(
        vec![void],
        int,
        true,
        false,
        vec![parameter],
        symbols.clone(),
    );
    expect_vf12_failure(&input, "signature does not match its parameters");

    // Non-prototype and variadic signatures are outside the direct-call
    // subset even when their arity and parameter types happen to match.
    for (prototype, variadic) in [(false, false), (true, true)] {
        let input = vf12_input(
            vec![int],
            int,
            prototype,
            variadic,
            vec![parameter],
            symbols.clone(),
        );
        expect_vf12_failure(&input, "signature does not match its parameters");
    }
}

#[test]
fn vf12_rejects_malformed_parameter_symbols() {
    let int = TypeId::from_index(0);
    let signature = TypeId::from_index(1);
    let parameter = SymbolId::from_index(0);
    let valid = parameter_symbol(Some(int), SymbolKind::Object, StorageDuration::Automatic);

    // Missing symbol record.
    let input = vf12_input(vec![int], int, true, false, vec![parameter], vec![]);
    expect_vf12_failure(&input, "parameter symbol is not committed");

    // Kind, storage, type and missing type are all checked as one invariant:
    // an automatic canonical-int object.
    for bad in [
        parameter_symbol(Some(int), SymbolKind::Function, StorageDuration::Automatic),
        parameter_symbol(Some(int), SymbolKind::Object, StorageDuration::Static),
        parameter_symbol(
            Some(signature),
            SymbolKind::Object,
            StorageDuration::Automatic,
        ),
        parameter_symbol(None, SymbolKind::Object, StorageDuration::Automatic),
    ] {
        let input = vf12_input(
            vec![int],
            int,
            true,
            false,
            vec![parameter],
            vec![(parameter, bad)],
        );
        expect_vf12_failure(&input, "parameter is not an automatic int object");
    }

    // Control: the same arity with a valid automatic int object passes the
    // parameter checks; with no entry block the fixture then fails on block
    // ownership, which proves the parameter invariant was the only blocker.
    let input = vf12_input(
        vec![int],
        int,
        true,
        false,
        vec![parameter],
        vec![(parameter, valid)],
    );
    expect_vf12_failure(&input, "function does not own exactly one entry block");
}
