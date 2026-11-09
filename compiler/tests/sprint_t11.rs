// ============================================================================
// sprint_t11.rs — `/55` repeated literals across functions.
//
// Two functions use the same literal spelling and value. Every occurrence is
// its own `LiteralRecord`, so each `ConstRecord` must name only that
// occurrence in `origins`. Lowering must select the constant belonging to the
// function under lowering, not the first/last constant or a value-equal
// sibling. The synthetic bus commits AST, type, symbol, semantic, and constant
// facts directly; only `IrFunctionChip` projection/computation runs.
//
// Cases:
//   * literal-fold origin: `int f(void){return 7;}` and `int g(void){return 7;}`
//   * canonical binary-fold origin: both bodies `return 2+3;`
// ============================================================================

use std::collections::BTreeSet;

use cc_silicon_compiler::bus::{
    CompilerBus, ConstRecord, ConstValue, EffectMask, InstructionRecord, IntRank, IntegerConst,
    IrOp, Linkage, LiteralRecord, NodeKind, NodeRecord, ScopeKind, ScopeRecord, SemRecord,
    StorageDuration, SymbolKind, SymbolRecord, TaggedProposal, TypeKind, TypeRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{IrFunctionChip, IrFunctionInput, LocalExpr, Worker};
use cc_silicon_compiler::commit::commit_proposals;
use cc_silicon_compiler::ids::{
    BlockId, ConstId, FunctionId, LiteralId, NameId, NodeId, RecordRef, ScopeId, TaskId, TokenId,
    TypeId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{StoreSchema, IR_FUNCTION_CHIP};
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, ResultValue, TaskDraft, TaskKind, TaskKindRegistry, TaskState,
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

struct Fixture {
    bus: CompilerBus,
    int: TypeId,
    file_scope: ScopeId,
    function_type: TypeId,
}

impl Fixture {
    fn new() -> Self {
        let mut bus = new_bus();
        let limits = bus.limits();
        let int = bus
            .arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Int {
                        rank: IntRank::Int,
                        signed: true,
                    },
                },
                &limits,
            )
            .unwrap();
        let file_scope = bus
            .arenas
            .scopes
            .alloc(
                ScopeRecord {
                    kind: ScopeKind::File,
                    parent: None,
                    owner: None,
                },
                &limits,
            )
            .unwrap();
        let function_type = bus
            .arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Function {
                        result: int,
                        params: vec![],
                        prototype: true,
                        variadic: false,
                    },
                },
                &limits,
            )
            .unwrap();
        bus.kinds = TaskKindRegistry::ir_slice();
        bus.schema = StoreSchema::ir_slice();
        bus.registrations
            .register(IrFunctionChip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
        Self {
            bus,
            int,
            file_scope,
            function_type,
        }
    }

    fn name(&mut self, spelling: &[u8]) -> NameId {
        self.bus.intern_name(spelling).unwrap()
    }

    fn node(
        &mut self,
        kind: NodeKind,
        children: Vec<NodeId>,
        name: Option<NameId>,
        literal: Option<LiteralId>,
    ) -> NodeId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .nodes
            .alloc(
                NodeRecord {
                    kind,
                    parent: None,
                    children,
                    first_token: TokenId::from_index(0),
                    last_token: TokenId::from_index(0),
                    name,
                    literal,
                },
                &limits,
            )
            .unwrap()
    }

    fn sem_int(&mut self, node: NodeId) {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .sem
            .alloc(
                SemRecord {
                    node,
                    ty: self.int,
                    category: ValueCategory::NonLvalue,
                    effects: EffectMask(0),
                },
                &limits,
            )
            .unwrap();
    }

    fn literal_expr(&mut self, value: u8) -> (NodeId, LiteralId) {
        let limits = self.bus.limits();
        let literal = self
            .bus
            .arenas
            .literals
            .alloc(
                LiteralRecord {
                    token: None,
                    kind: LiteralKind::Integer,
                    radix: 10,
                    suffix: LiteralSuffix::None,
                    value: vec![value],
                    negative: false,
                    spelling: value.to_string().into_bytes(),
                    candidate_type: Lx08CandidateType::Int,
                },
                &limits,
            )
            .unwrap();
        let node = self.node(NodeKind::IntLiteral, vec![], None, Some(literal));
        self.sem_int(node);
        (node, literal)
    }

    fn binary_add(&mut self, lhs: NodeId, rhs: NodeId) -> NodeId {
        let node = self.node(NodeKind::BinaryAdd, vec![lhs, rhs], None, None);
        self.sem_int(node);
        node
    }

    fn return_stmt(&mut self, operand: NodeId) -> NodeId {
        let node = self.node(NodeKind::Return, vec![operand], None, None);
        self.sem_int(node);
        node
    }

    fn const_record(&mut self, value: u8, origins: Vec<LiteralId>) -> ConstId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .consts
            .alloc(
                ConstRecord {
                    ty: self.int,
                    value: ConstValue::Integer(IntegerConst::new(vec![value], false)),
                    origins,
                },
                &limits,
            )
            .unwrap()
    }

    fn function(&mut self, name: NameId, statement: NodeId) -> NodeId {
        let specifiers = self.node(NodeKind::Specifiers, vec![], None, None);
        let declarator = self.node(NodeKind::Declarator, vec![], Some(name), None);
        let compound = self.node(NodeKind::Compound, vec![statement], None, None);
        let funcdef = self.node(
            NodeKind::FunctionDefinition,
            vec![specifiers, declarator, compound],
            None,
            None,
        );
        let limits = self.bus.limits();
        self.bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name,
                    scope: self.file_scope,
                    kind: SymbolKind::Function,
                    ty: Some(self.function_type),
                    linkage: Linkage::External,
                    storage: StorageDuration::Static,
                    decl: declarator,
                },
                &limits,
            )
            .unwrap();
        self.bus
            .arenas
            .sem
            .alloc(
                SemRecord {
                    node: funcdef,
                    ty: self.function_type,
                    category: ValueCategory::NonLvalue,
                    effects: EffectMask(0),
                },
                &limits,
            )
            .unwrap();
        funcdef
    }

    fn origins(&self, constant: ConstId) -> Vec<LiteralId> {
        self.bus
            .arenas
            .consts
            .get(constant)
            .unwrap()
            .origins
            .clone()
    }

    fn instruction_stream(&self, function: FunctionId) -> Vec<InstructionRecord> {
        let blocks: BTreeSet<BlockId> = self
            .bus
            .arenas
            .blocks
            .iter()
            .filter(|(_, block)| block.function == function)
            .map(|(id, _)| id)
            .collect();
        self.bus
            .arenas
            .instructions
            .iter()
            .filter(|(_, instruction)| blocks.contains(&instruction.block))
            .map(|(_, instruction)| instruction.clone())
            .collect()
    }
}

fn project(fixture: &mut Fixture, funcdef: NodeId) -> (TaskId, IrFunctionInput) {
    let task = fixture
        .bus
        .bootstrap_task(TaskDraft {
            kind: TaskKind::IR_FUNCTION,
            owner: IR_FUNCTION_CHIP,
            parent: None,
            payload: Payload::from_refs(vec![RecordRef::Node(funcdef)]),
            continuation: None,
        })
        .unwrap();
    fixture.bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    let input = cc_silicon_compiler::chips::project_ir_function_input(&fixture.bus, task).unwrap();
    (task, input)
}

fn commit(fixture: &mut Fixture, task: TaskId, input: IrFunctionInput) -> FunctionId {
    let tagged: Vec<TaggedProposal> = IrFunctionChip
        .compute(&input)
        .into_iter()
        .map(|proposal| TaggedProposal {
            chip: IR_FUNCTION_CHIP,
            task,
            proposal,
        })
        .collect();
    let report = commit_proposals(&mut fixture.bus, tagged).expect("IR append commits");
    assert!(report.failed.is_empty(), "unexpected lowering failure");
    let result = match report.completed.as_slice() {
        [(_, result)] => *result,
        other => panic!("expected one completed lowering task, got {other:?}"),
    };
    match fixture
        .bus
        .arenas
        .results
        .get(result)
        .unwrap()
        .value
        .clone()
    {
        ResultValue::Record(RecordRef::Function(function)) => function,
        other => panic!("expected a lowered function, got {other:?}"),
    }
}

fn assert_single_literal_program(input: &IrFunctionInput, operand: NodeId, constant: ConstId) {
    match input.local_expr.as_ref() {
        Some(LocalExpr::Program { value, local, .. }) => {
            assert_eq!(*local, None);
            match value.as_ref() {
                LocalExpr::Literal {
                    node,
                    constant: selected,
                } => {
                    assert_eq!(*node, operand);
                    assert_eq!(*selected, constant);
                }
                other => panic!("expected a literal program value, got {other:?}"),
            }
        }
        other => panic!("expected the general program envelope, got {other:?}"),
    }
}

fn assert_constant_return(instructions: &[InstructionRecord], constant: ConstId) {
    assert_eq!(
        instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect::<Vec<_>>(),
        vec![IrOp::Constant, IrOp::Return]
    );
    assert_eq!(instructions[0].immediate, Some(constant));
    let value = instructions[0].result.expect("constant result");
    assert_eq!(instructions[1].operands, vec![value]);
}

#[test]
fn repeated_literal_returns_keep_their_own_origins() {
    let mut fixture = Fixture::new();

    let f_name = fixture.name(b"f");
    let (f_operand, f_literal) = fixture.literal_expr(7);
    let f_constant = fixture.const_record(7, vec![f_literal]);
    let f_return = fixture.return_stmt(f_operand);
    let f_funcdef = fixture.function(f_name, f_return);

    let g_name = fixture.name(b"g");
    let (g_operand, g_literal) = fixture.literal_expr(7);
    let g_constant = fixture.const_record(7, vec![g_literal]);
    let g_return = fixture.return_stmt(g_operand);
    let g_funcdef = fixture.function(g_name, g_return);

    // Same spelling, same value, different literal occurrence.
    assert_ne!(f_literal, g_literal);
    assert_ne!(f_constant, g_constant);
    assert_eq!(fixture.origins(f_constant), vec![f_literal]);
    assert_eq!(fixture.origins(g_constant), vec![g_literal]);
    let f_integer = fixture
        .bus
        .arenas
        .consts
        .get(f_constant)
        .unwrap()
        .value
        .as_integer()
        .unwrap()
        .clone();
    let g_integer = fixture
        .bus
        .arenas
        .consts
        .get(g_constant)
        .unwrap()
        .value
        .as_integer()
        .unwrap()
        .clone();
    assert_eq!(
        f_integer, g_integer,
        "value-equal constants must stay distinct"
    );
    assert_eq!(f_integer.magnitude, vec![7]);

    let (f_task, f_input) = project(&mut fixture, f_funcdef);
    assert_eq!(f_input.constant, f_constant);
    assert_single_literal_program(&f_input, f_operand, f_constant);
    let f_function = commit(&mut fixture, f_task, f_input);

    let (g_task, g_input) = project(&mut fixture, g_funcdef);
    assert_eq!(g_input.constant, g_constant);
    assert_single_literal_program(&g_input, g_operand, g_constant);
    let g_function = commit(&mut fixture, g_task, g_input);

    assert_constant_return(&fixture.instruction_stream(f_function), f_constant);
    assert_constant_return(&fixture.instruction_stream(g_function), g_constant);
}

#[test]
fn repeated_literal_pairs_keep_their_own_binary_origins() {
    let mut fixture = Fixture::new();

    let f_name = fixture.name(b"f");
    let (f_lhs, f_lhs_literal) = fixture.literal_expr(2);
    let (f_rhs, f_rhs_literal) = fixture.literal_expr(3);
    fixture.const_record(2, vec![f_lhs_literal]);
    fixture.const_record(3, vec![f_rhs_literal]);
    let f_sum = fixture.binary_add(f_lhs, f_rhs);
    let f_constant = fixture.const_record(5, vec![f_lhs_literal, f_rhs_literal]);
    let f_return = fixture.return_stmt(f_sum);
    let f_funcdef = fixture.function(f_name, f_return);

    let g_name = fixture.name(b"g");
    let (g_lhs, g_lhs_literal) = fixture.literal_expr(2);
    let (g_rhs, g_rhs_literal) = fixture.literal_expr(3);
    fixture.const_record(2, vec![g_lhs_literal]);
    fixture.const_record(3, vec![g_rhs_literal]);
    let g_sum = fixture.binary_add(g_lhs, g_rhs);
    let g_constant = fixture.const_record(5, vec![g_lhs_literal, g_rhs_literal]);
    let g_return = fixture.return_stmt(g_sum);
    let g_funcdef = fixture.function(g_name, g_return);

    assert_ne!(f_constant, g_constant);
    assert_eq!(
        fixture.origins(f_constant),
        vec![f_lhs_literal, f_rhs_literal]
    );
    assert_eq!(
        fixture.origins(g_constant),
        vec![g_lhs_literal, g_rhs_literal]
    );

    // Each canonical function must pick the pair naming its own literal
    // occurrences, even though both folds carry the same value 5.
    let (f_task, f_input) = project(&mut fixture, f_funcdef);
    assert_eq!(f_input.constant, f_constant);
    assert!(
        f_input.local_expr.is_none(),
        "canonical path has no envelope"
    );
    let f_function = commit(&mut fixture, f_task, f_input);

    let (g_task, g_input) = project(&mut fixture, g_funcdef);
    assert_eq!(g_input.constant, g_constant);
    assert!(
        g_input.local_expr.is_none(),
        "canonical path has no envelope"
    );
    let g_function = commit(&mut fixture, g_task, g_input);

    assert_constant_return(&fixture.instruction_stream(f_function), f_constant);
    assert_constant_return(&fixture.instruction_stream(g_function), g_constant);
}
