// ============================================================================
// call_ir.rs — `/54` direct-call IR lowering acceptance.
//
// The synthetic bus commits the frozen call schema records directly:
// canonical `int`, function types/symbols, block-scope parameter objects,
// the three-child function AST, checked `SemRecord`s, and origin-tagged
// constants. `IrFunctionChip` must bind parameters in declaration order,
// resolve earlier file-scope callees, lower nested calls left-to-right, and
// keep values/functions/blocks per function. Negative cases pin unknown,
// forward, self, arity, non-int, and constant-origin failures.
// ============================================================================

use std::collections::BTreeSet;

use cc_silicon_compiler::bus::{
    CompilerBus, ConstRecord, ConstValue, EffectMask, FunctionRecord, IntRank, IntegerConst,
    IrOp, Linkage, LiteralRecord, NodeKind, NodeRecord, ScopeKind, ScopeRecord, SemRecord,
    StorageDuration, SymbolKind, SymbolRecord, TaggedProposal, TypeKind, TypeRecord,
    ValueCategory, EFFECT_WRITES_LOCAL,
};
use cc_silicon_compiler::chips::{IrFunctionChip, IrFunctionInput, LocalExpr, Worker};
use cc_silicon_compiler::commit::commit_proposals;
use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticDraft};
use cc_silicon_compiler::ids::{
    BlockId, ConstId, FunctionId, LiteralId, NameId, NodeId, RecordRef, ScopeId, SemId, SymbolId,
    TaskId, TokenId, TypeId,
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
        bus.kinds = TaskKindRegistry::ir_slice();
        bus.schema = StoreSchema::ir_slice();
        bus.registrations
            .register(IrFunctionChip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
        Self {
            bus,
            int,
            file_scope,
        }
    }

    fn name(&mut self, text: &[u8]) -> NameId {
        self.bus.intern_name(text).unwrap()
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

    fn sem_with(
        &mut self,
        node: NodeId,
        ty: TypeId,
        category: ValueCategory,
        effects: EffectMask,
    ) -> SemId {
        let limits = self.bus.limits();
        self.bus
            .arenas
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

    fn sem_int(&mut self, node: NodeId) -> SemId {
        self.sem_with(node, self.int, ValueCategory::NonLvalue, EffectMask(0))
    }

    fn literal_node(&mut self, value: u8) -> (NodeId, LiteralId) {
        let limits = self.bus.limits();
        let literal = self.bus
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
        (node, literal)
    }

    fn int_lit(&mut self, value: u8) -> (NodeId, LiteralId) {
        let (node, literal) = self.literal_node(value);
        self.sem_int(node);
        (node, literal)
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

    fn literal_const(&mut self, literal: LiteralId, value: u8) -> ConstId {
        self.const_record(value, vec![literal])
    }

    fn binary_const(&mut self, lhs: LiteralId, rhs: LiteralId, value: u8) -> ConstId {
        self.const_record(value, vec![lhs, rhs])
    }

    fn identifier(&mut self, name: NameId) -> NodeId {
        let node = self.node(NodeKind::Identifier, vec![], Some(name), None);
        self.sem_int(node);
        node
    }

    fn binary_expr(&mut self, kind: NodeKind, lhs: NodeId, rhs: NodeId) -> NodeId {
        let node = self.node(kind, vec![lhs, rhs], None, None);
        self.sem_int(node);
        node
    }

    fn call(&mut self, callee: NameId, args: Vec<NodeId>) -> NodeId {
        let callee_node = self.node(NodeKind::Identifier, vec![], Some(callee), None);
        let mut children = vec![callee_node];
        children.extend(args);
        let node = self.node(NodeKind::Call, children, None, None);
        self.sem_int(node);
        node
    }

    fn raw_call(&mut self, callee: NameId, args: Vec<NodeId>, effects: EffectMask) -> NodeId {
        let callee_node = self.node(NodeKind::Identifier, vec![], Some(callee), None);
        let mut children = vec![callee_node];
        children.extend(args);
        let node = self.node(NodeKind::Call, children, None, None);
        self.sem_with(node, self.int, ValueCategory::NonLvalue, effects);
        node
    }

    fn return_stmt(&mut self, operand: NodeId) -> NodeId {
        let node = self.node(NodeKind::Return, vec![operand], None, None);
        self.sem_int(node);
        node
    }

    fn assignment_stmt(&mut self, lhs: NodeId, rhs: NodeId) -> NodeId {
        let node = self.node(NodeKind::Assignment, vec![lhs, rhs], None, None);
        self.sem_with(
            node,
            self.int,
            ValueCategory::NonLvalue,
            EffectMask(EFFECT_WRITES_LOCAL),
        );
        node
    }

    fn declaration(&mut self, name: NameId, initializer: NodeId) -> NodeId {
        let specifiers = self.node(NodeKind::Specifiers, vec![], None, None);
        let declarator = self.node(NodeKind::Declarator, vec![], Some(name), None);
        self.node(
            NodeKind::Declaration,
            vec![specifiers, declarator, initializer],
            None,
            None,
        )
    }

    fn func_type(&mut self, parameters: usize) -> TypeId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Function {
                        result: self.int,
                        params: vec![self.int; parameters],
                        prototype: true,
                        variadic: false,
                    },
                },
                &limits,
            )
            .unwrap()
    }

    fn function_symbol(&mut self, name: NameId, ty: TypeId, decl: NodeId) -> SymbolId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name,
                    scope: self.file_scope,
                    kind: SymbolKind::Function,
                    ty: Some(ty),
                    linkage: Linkage::External,
                    storage: StorageDuration::Static,
                    decl,
                },
                &limits,
            )
            .unwrap()
    }

    fn object_symbol(&mut self, name: NameId, scope: ScopeId, decl: NodeId) -> SymbolId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name,
                    scope,
                    kind: SymbolKind::Object,
                    ty: Some(self.int),
                    linkage: Linkage::None,
                    storage: StorageDuration::Automatic,
                    decl,
                },
                &limits,
            )
            .unwrap()
    }

    fn block_scope(&mut self, owner: NodeId) -> ScopeId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .scopes
            .alloc(
                ScopeRecord {
                    kind: ScopeKind::Block,
                    parent: Some(self.file_scope),
                    owner: Some(owner),
                },
                &limits,
            )
            .unwrap()
    }

    fn function(
        &mut self,
        name: NameId,
        parameter_names: &[NameId],
        statements: Vec<NodeId>,
    ) -> BuiltFunction {
        let specifiers = self.node(NodeKind::Specifiers, vec![], None, None);
        let parameters: Vec<NodeId> = parameter_names
            .iter()
            .map(|name| self.node(NodeKind::Parameter, vec![], Some(*name), None))
            .collect();
        let declarator_children = if parameters.is_empty() {
            vec![]
        } else {
            vec![self.node(NodeKind::ParameterList, parameters.clone(), None, None)]
        };
        let declarator = self.node(
            NodeKind::Declarator,
            declarator_children,
            Some(name),
            None,
        );
        let compound = self.node(NodeKind::Compound, statements.clone(), None, None);
        let funcdef = self.node(
            NodeKind::FunctionDefinition,
            vec![specifiers, declarator, compound],
            None,
            None,
        );
        let signature = self.func_type(parameter_names.len());
        let symbol = self.function_symbol(name, signature, declarator);
        let block = self.block_scope(compound);
        let locals: Vec<(NameId, NodeId)> = statements
            .iter()
            .filter_map(|statement| {
                let body = self.bus.arenas.nodes.get(*statement).ok()?;
                if body.kind != NodeKind::Declaration {
                    return None;
                }
                let declarator = *body.children.get(1)?;
                let name = self.bus.arenas.nodes.get(declarator).ok()?.name?;
                Some((name, declarator))
            })
            .collect();
        for (local_name, declarator) in locals {
            self.object_symbol(local_name, block, declarator);
        }
        let parameter_symbols: Vec<SymbolId> = parameters
            .iter()
            .zip(parameter_names)
            .map(|(parameter, name)| self.object_symbol(*name, block, *parameter))
            .collect();
        self.sem_with(
            funcdef,
            signature,
            ValueCategory::NonLvalue,
            EffectMask(0),
        );
        BuiltFunction {
            funcdef,
            symbol,
            parameters: parameter_symbols,
        }
    }
}

#[derive(Clone, Debug)]
struct BuiltFunction {
    funcdef: NodeId,
    symbol: SymbolId,
    parameters: Vec<SymbolId>,
}

fn project(
    fixture: &mut Fixture,
    funcdef: NodeId,
) -> Result<(TaskId, IrFunctionInput), DiagnosticDraft> {
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
    let input = cc_silicon_compiler::chips::project_ir_function_input(&fixture.bus, task)?;
    Ok((task, input))
}

fn commit(
    fixture: &mut Fixture,
    task: TaskId,
    input: IrFunctionInput,
) -> FunctionId {
    let proposals = IrFunctionChip.compute(&input);
    let tagged: Vec<TaggedProposal> = proposals
        .into_iter()
        .map(|proposal| TaggedProposal {
            chip: IR_FUNCTION_CHIP,
            task,
            proposal,
        })
        .collect();
    let report = commit_proposals(&mut fixture.bus, tagged).expect("IR append commits");
    let result = match report.completed.as_slice() {
        [(_, result)] => *result,
        other => panic!("expected one completed IR task, got {other:?}"),
    };
    match fixture.bus.arenas.results.get(result).unwrap().value.clone() {
        ResultValue::Record(RecordRef::Function(function)) => function,
        other => panic!("expected a function result, got {other:?}"),
    }
}

fn lower(fixture: &mut Fixture, funcdef: NodeId) -> Result<FunctionId, DiagnosticDraft> {
    let (task, input) = project(fixture, funcdef)?;
    Ok(commit(fixture, task, input))
}

fn function_record(fixture: &Fixture, function: FunctionId) -> FunctionRecord {
    fixture.bus.arenas.functions.get(function).unwrap().clone()
}

fn instructions_for(fixture: &Fixture, function: FunctionId) -> Vec<cc_silicon_compiler::bus::InstructionRecord> {
    let blocks: BTreeSet<BlockId> = fixture
        .bus
        .arenas
        .blocks
        .iter()
        .filter(|(_, block)| block.function == function)
        .map(|(id, _)| id)
        .collect();
    fixture
        .bus
        .arenas
        .instructions
        .iter()
        .filter(|(_, instruction)| blocks.contains(&instruction.block))
        .map(|(_, instruction)| instruction.clone())
        .collect()
}

fn ops(instructions: &[cc_silicon_compiler::bus::InstructionRecord]) -> Vec<IrOp> {
    instructions.iter().map(|instruction| instruction.op).collect()
}

fn build_add(fixture: &mut Fixture) -> (BuiltFunction, NameId) {
    let add = fixture.name(b"add");
    let a = fixture.name(b"a");
    let b = fixture.name(b"b");
    let lhs = fixture.identifier(a);
    let rhs = fixture.identifier(b);
    let sum = fixture.binary_expr(NodeKind::BinaryAdd, lhs, rhs);
    let ret = fixture.return_stmt(sum);
    (fixture.function(add, &[a, b], vec![ret]), add)
}

fn build_inc(fixture: &mut Fixture) -> (BuiltFunction, NameId) {
    let inc = fixture.name(b"inc");
    let x = fixture.name(b"x");
    let load = fixture.identifier(x);
    let (one_node, one_literal) = fixture.int_lit(1);
    fixture.literal_const(one_literal, 1);
    let sum = fixture.binary_expr(NodeKind::BinaryAdd, load, one_node);
    let ret = fixture.return_stmt(sum);
    (fixture.function(inc, &[x], vec![ret]), inc)
}

#[test]
fn add_and_main_call_lower_with_parameters_and_callee() {
    let mut fixture = Fixture::new();
    let (add, add_name) = build_add(&mut fixture);
    let main = fixture.name(b"main");
    let (two, two_literal) = fixture.int_lit(2);
    let two_const = fixture.literal_const(two_literal, 2);
    let (three, three_literal) = fixture.int_lit(3);
    let three_const = fixture.literal_const(three_literal, 3);
    let call = fixture.call(add_name, vec![two, three]);
    let ret = fixture.return_stmt(call);
    let main_fn = fixture.function(main, &[], vec![ret]);

    let add_fn = lower(&mut fixture, add.funcdef).unwrap();
    let add_record = function_record(&fixture, add_fn);
    assert_eq!(add_record.parameters, add.parameters.clone());
    assert_eq!(add_record.parameters.len(), 2);
    let add_instructions = instructions_for(&fixture, add_fn);
    assert_eq!(
        ops(&add_instructions),
        vec![IrOp::LocalLoad, IrOp::LocalLoad, IrOp::Add, IrOp::Return]
    );
    assert_eq!(add_instructions[0].local, Some(add.parameters[0]));
    assert_eq!(add_instructions[1].local, Some(add.parameters[1]));
    assert_eq!(
        add_instructions[2].operands,
        vec![add_instructions[0].result.unwrap(), add_instructions[1].result.unwrap()]
    );

    let main_fn = lower(&mut fixture, main_fn.funcdef).unwrap();
    let main_record = function_record(&fixture, main_fn);
    assert!(main_record.parameters.is_empty());
    let instructions = instructions_for(&fixture, main_fn);
    assert_eq!(
        ops(&instructions),
        vec![IrOp::Constant, IrOp::Constant, IrOp::Call, IrOp::Return]
    );
    let value_base = add_instructions
        .iter()
        .filter_map(|instruction| instruction.result)
        .map(|value| value.index())
        .max()
        .unwrap()
        + 1;
    assert_eq!(instructions[0].immediate, Some(two_const));
    assert_eq!(instructions[0].result.unwrap().index(), value_base);
    assert_eq!(instructions[1].immediate, Some(three_const));
    assert_eq!(instructions[1].result.unwrap().index(), value_base + 1);
    let call_instruction = &instructions[2];
    assert_eq!(call_instruction.callee, Some(add.symbol));
    assert_eq!(
        call_instruction.operands,
        vec![
            instructions[0].result.unwrap(),
            instructions[1].result.unwrap()
        ]
    );
    assert_eq!(call_instruction.result.unwrap().index(), value_base + 2);
    assert_eq!(call_instruction.local, None);
    assert_eq!(call_instruction.immediate, None);
    assert!(call_instruction.successors.is_empty());
    assert_eq!(
        instructions[3].operands,
        vec![call_instruction.result.unwrap()]
    );
    // The caller's values do not alias the callee's value range.
    for instruction in &instructions {
        for value in instruction
            .operands
            .iter()
            .chain(instruction.result.iter())
        {
            assert!(value.index() >= value_base, "{value:?}");
        }
    }
}

#[test]
fn nested_calls_lower_arguments_left_to_right() {
    let mut fixture = Fixture::new();
    let (inc, inc_name) = build_inc(&mut fixture);
    let (add, add_name) = build_add(&mut fixture);
    let main = fixture.name(b"main");
    let (two, two_literal) = fixture.int_lit(2);
    fixture.literal_const(two_literal, 2);
    let (three, three_literal) = fixture.int_lit(3);
    fixture.literal_const(three_literal, 3);
    let first_inc = fixture.call(inc_name, vec![two]);
    let second_inc = fixture.call(inc_name, vec![three]);
    let outer = fixture.call(add_name, vec![first_inc, second_inc]);
    let ret = fixture.return_stmt(outer);
    let main_fn = fixture.function(main, &[], vec![ret]);

    let inc_fn = lower(&mut fixture, inc.funcdef).unwrap();
    let add_fn = lower(&mut fixture, add.funcdef).unwrap();
    let main_fn = lower(&mut fixture, main_fn.funcdef).unwrap();
    assert_eq!(
        function_record(&fixture, inc_fn).parameters,
        inc.parameters.clone()
    );
    assert_eq!(
        function_record(&fixture, add_fn).parameters,
        add.parameters.clone()
    );
    let instructions = instructions_for(&fixture, main_fn);
    assert_eq!(
        ops(&instructions),
        vec![
            IrOp::Constant,
            IrOp::Call,
            IrOp::Constant,
            IrOp::Call,
            IrOp::Call,
            IrOp::Return
        ]
    );
    assert_eq!(instructions[1].callee, Some(inc.symbol));
    assert_eq!(instructions[3].callee, Some(inc.symbol));
    assert_eq!(instructions[4].callee, Some(add.symbol));
    assert_eq!(
        instructions[4].operands,
        vec![
            instructions[1].result.unwrap(),
            instructions[3].result.unwrap()
        ]
    );
    assert_eq!(
        instructions[5].operands,
        vec![instructions[4].result.unwrap()]
    );
    // Every function owns a disjoint set of instruction results.
    let mut seen = BTreeSet::new();
    for function in [inc_fn, add_fn, main_fn] {
        for instruction in instructions_for(&fixture, function) {
            if let Some(result) = instruction.result {
                assert!(seen.insert(result.index()), "reused value {result:?}");
            }
        }
    }
}

#[test]
fn parameter_only_function_lowers_identifier_to_parameter_symbol() {
    let mut fixture = Fixture::new();
    let id_name = fixture.name(b"id");
    let x = fixture.name(b"x");
    let load = fixture.identifier(x);
    let ret = fixture.return_stmt(load);
    let id = fixture.function(id_name, &[x], vec![ret]);

    let (task, input) = project(&mut fixture, id.funcdef).unwrap();
    assert_eq!(input.local, None);
    match input.local_expr.as_ref() {
        Some(LocalExpr::Program {
            parameters,
            identifiers,
            local,
            initializer,
            assignments,
            ..
        }) => {
            assert_eq!(parameters, &id.parameters);
            assert_eq!(*local, None);
            assert_eq!(*initializer, None);
            assert!(assignments.is_empty());
            assert_eq!(identifiers.get(&load), Some(&id.parameters[0]));
        }
        other => panic!("expected a program envelope, got {other:?}"),
    }
    let function = commit(&mut fixture, task, input);
    assert_eq!(
        function_record(&fixture, function).parameters,
        id.parameters.clone()
    );
    let instructions = instructions_for(&fixture, function);
    assert_eq!(ops(&instructions), vec![IrOp::LocalLoad, IrOp::Return]);
    assert_eq!(instructions[0].local, Some(id.parameters[0]));
    assert_eq!(
        instructions[1].operands,
        vec![instructions[0].result.unwrap()]
    );
}

#[test]
fn eight_parameters_keep_declaration_and_argument_order() {
    let mut fixture = Fixture::new();
    let eight_name = fixture.name(b"eight");
    let names: Vec<NameId> = (b'a'..=b'h')
        .map(|byte| fixture.name(&[byte]))
        .collect();
    let mut expression = fixture.identifier(names[0]);
    for name in &names[1..] {
        let next = fixture.identifier(*name);
        expression = fixture.binary_expr(NodeKind::BinaryAdd, expression, next);
    }
    let ret = fixture.return_stmt(expression);
    let eight = fixture.function(eight_name, &names, vec![ret]);

    let mut arguments = Vec::new();
    for value in 1..=8u8 {
        let (node, literal) = fixture.int_lit(value);
        fixture.literal_const(literal, value);
        arguments.push(node);
    }
    let main = fixture.name(b"main");
    let call = fixture.call(eight_name, arguments);
    let ret = fixture.return_stmt(call);
    let main_fn = fixture.function(main, &[], vec![ret]);

    let eight_fn = lower(&mut fixture, eight.funcdef).unwrap();
    let record = function_record(&fixture, eight_fn);
    assert_eq!(record.parameters, eight.parameters.clone());
    assert_eq!(record.parameters.len(), 8);
    let instructions = instructions_for(&fixture, eight_fn);
    assert_eq!(
        ops(&instructions),
        vec![
            IrOp::LocalLoad,
            IrOp::LocalLoad,
            IrOp::Add,
            IrOp::LocalLoad,
            IrOp::Add,
            IrOp::LocalLoad,
            IrOp::Add,
            IrOp::LocalLoad,
            IrOp::Add,
            IrOp::LocalLoad,
            IrOp::Add,
            IrOp::LocalLoad,
            IrOp::Add,
            IrOp::LocalLoad,
            IrOp::Add,
            IrOp::Return
        ]
    );
    let loads: Vec<_> = instructions
        .iter()
        .filter(|instruction| instruction.op == IrOp::LocalLoad)
        .collect();
    assert_eq!(loads.len(), 8);
    for (load, symbol) in loads.iter().zip(&eight.parameters) {
        assert_eq!(load.local, Some(*symbol));
    }

    let main_fn = lower(&mut fixture, main_fn.funcdef).unwrap();
    let instructions = instructions_for(&fixture, main_fn);
    assert_eq!(
        ops(&instructions),
        vec![
            IrOp::Constant,
            IrOp::Constant,
            IrOp::Constant,
            IrOp::Constant,
            IrOp::Constant,
            IrOp::Constant,
            IrOp::Constant,
            IrOp::Constant,
            IrOp::Call,
            IrOp::Return
        ]
    );
    let call_instruction = &instructions[8];
    assert_eq!(call_instruction.callee, Some(eight.symbol));
    assert_eq!(
        call_instruction.operands,
        instructions[..8]
            .iter()
            .map(|instruction| instruction.result.unwrap())
            .collect::<Vec<_>>()
    );
}

#[test]
fn call_inside_binary_and_assignment_lower_in_source_order() {
    let mut fixture = Fixture::new();
    let (add, add_name) = build_add(&mut fixture);
    let main = fixture.name(b"main");
    let x = fixture.name(b"x");
    let (initializer, initializer_literal) = fixture.int_lit(2);
    fixture.literal_const(initializer_literal, 2);
    let declaration = fixture.declaration(x, initializer);
    let lhs = fixture.identifier(x);
    let argument = fixture.identifier(x);
    let (three, three_literal) = fixture.int_lit(3);
    fixture.literal_const(three_literal, 3);
    let call = fixture.call(add_name, vec![argument, three]);
    let assignment = fixture.assignment_stmt(lhs, call);
    let load = fixture.identifier(x);
    let ret = fixture.return_stmt(load);
    let main_fn = fixture.function(main, &[], vec![declaration, assignment, ret]);

    let add_fn = lower(&mut fixture, add.funcdef).unwrap();
    let main_fn = lower(&mut fixture, main_fn.funcdef).unwrap();
    let add_parameters = function_record(&fixture, add_fn).parameters;
    assert_eq!(add_parameters, add.parameters.clone());
    assert!(function_record(&fixture, main_fn).parameters.is_empty());
    let instructions = instructions_for(&fixture, main_fn);
    assert_eq!(
        ops(&instructions),
        vec![
            IrOp::Constant,
            IrOp::LocalStore,
            IrOp::LocalLoad,
            IrOp::Constant,
            IrOp::Call,
            IrOp::LocalStore,
            IrOp::LocalLoad,
            IrOp::Return
        ]
    );
    // The single local is declared at the first identifier; the argument
    // load, both stores, and the final load all name that same symbol.
    let local = instructions[1].local.expect("initializer store names a local");
    assert_eq!(instructions[2].local, Some(local));
    assert_eq!(instructions[5].local, Some(local));
    assert_eq!(instructions[6].local, Some(local));
    assert_eq!(instructions[4].callee, Some(add.symbol));
    assert_eq!(
        instructions[4].operands,
        vec![
            instructions[2].result.unwrap(),
            instructions[3].result.unwrap()
        ]
    );
    assert_eq!(
        instructions[5].operands,
        vec![instructions[4].result.unwrap()]
    );
}

#[test]
fn local_argument_call_add_x_3_lowers_local_before_literal() {
    let mut fixture = Fixture::new();
    let (add, add_name) = build_add(&mut fixture);
    let main = fixture.name(b"main");
    let x = fixture.name(b"x");
    let (initializer, initializer_literal) = fixture.int_lit(2);
    fixture.literal_const(initializer_literal, 2);
    let declaration = fixture.declaration(x, initializer);
    let argument = fixture.identifier(x);
    let (three, three_literal) = fixture.int_lit(3);
    fixture.literal_const(three_literal, 3);
    let call = fixture.call(add_name, vec![argument, three]);
    let ret = fixture.return_stmt(call);
    let main_fn = fixture.function(main, &[], vec![declaration, ret]);

    let add_fn = lower(&mut fixture, add.funcdef).unwrap();
    assert_eq!(function_record(&fixture, add_fn).parameters, add.parameters);
    let main_fn = lower(&mut fixture, main_fn.funcdef).unwrap();
    let instructions = instructions_for(&fixture, main_fn);
    assert_eq!(
        ops(&instructions),
        vec![
            IrOp::Constant,
            IrOp::LocalStore,
            IrOp::LocalLoad,
            IrOp::Constant,
            IrOp::Call,
            IrOp::Return
        ]
    );
    let local = instructions[1].local.expect("initializer store names a local");
    assert_eq!(instructions[2].local, Some(local));
    assert_eq!(instructions[4].callee, Some(add.symbol));
    assert_eq!(
        instructions[4].operands,
        vec![
            instructions[2].result.unwrap(),
            instructions[3].result.unwrap()
        ]
    );
    assert_eq!(
        instructions[5].operands,
        vec![instructions[4].result.unwrap()]
    );
}

#[test]
fn call_in_binary_return_lowers_in_post_order() {
    let mut fixture = Fixture::new();
    let (add, add_name) = build_add(&mut fixture);
    let main = fixture.name(b"main");
    let (two, two_literal) = fixture.int_lit(2);
    fixture.literal_const(two_literal, 2);
    let (three, three_literal) = fixture.int_lit(3);
    fixture.literal_const(three_literal, 3);
    let call = fixture.call(add_name, vec![two, three]);
    let (one, one_literal) = fixture.int_lit(1);
    fixture.literal_const(one_literal, 1);
    let sum = fixture.binary_expr(NodeKind::BinaryAdd, call, one);
    let ret = fixture.return_stmt(sum);
    let main_fn = fixture.function(main, &[], vec![ret]);

    let _ = lower(&mut fixture, add.funcdef).unwrap();
    let main_fn = lower(&mut fixture, main_fn.funcdef).unwrap();
    let instructions = instructions_for(&fixture, main_fn);
    assert_eq!(
        ops(&instructions),
        vec![
            IrOp::Constant,
            IrOp::Constant,
            IrOp::Call,
            IrOp::Constant,
            IrOp::Add,
            IrOp::Return
        ]
    );
    assert_eq!(instructions[2].callee, Some(add.symbol));
    assert_eq!(
        instructions[4].operands,
        vec![
            instructions[2].result.unwrap(),
            instructions[3].result.unwrap()
        ]
    );
}

#[test]
fn canonical_and_local_assignment_paths_remain_unchanged() {
    let mut fixture = Fixture::new();
    let main = fixture.name(b"main");
    let (two, two_literal) = fixture.int_lit(2);
    let (three, three_literal) = fixture.int_lit(3);
    let sum = fixture.binary_expr(NodeKind::BinaryAdd, two, three);
    let folded = fixture.binary_const(two_literal, three_literal, 5);
    let ret = fixture.return_stmt(sum);
    let canonical = fixture.function(main, &[], vec![ret]);
    let function = lower(&mut fixture, canonical.funcdef).unwrap();
    assert!(function_record(&fixture, function).parameters.is_empty());
    let instructions = instructions_for(&fixture, function);
    assert_eq!(ops(&instructions), vec![IrOp::Constant, IrOp::Return]);
    assert_eq!(instructions[0].immediate, Some(folded));
    assert_eq!(fixture.bus.arenas.values.allocated(), 1);
    assert_eq!(fixture.bus.arenas.instructions.allocated(), 2);

    let mut fixture = Fixture::new();
    let main = fixture.name(b"main");
    let x = fixture.name(b"x");
    let (initializer, initializer_literal) = fixture.int_lit(2);
    fixture.literal_const(initializer_literal, 2);
    let declaration = fixture.declaration(x, initializer);
    let lhs = fixture.identifier(x);
    let load = fixture.identifier(x);
    let (three, three_literal) = fixture.int_lit(3);
    fixture.literal_const(three_literal, 3);
    let rhs = fixture.binary_expr(NodeKind::BinaryAdd, load, three);
    let assignment = fixture.assignment_stmt(lhs, rhs);
    let load = fixture.identifier(x);
    let ret = fixture.return_stmt(load);
    let local_fn = fixture.function(main, &[], vec![declaration, assignment, ret]);
    let function = lower(&mut fixture, local_fn.funcdef).unwrap();
    assert!(function_record(&fixture, function).parameters.is_empty());
    let instructions = instructions_for(&fixture, function);
    assert_eq!(
        ops(&instructions),
        vec![
            IrOp::Constant,
            IrOp::LocalStore,
            IrOp::LocalLoad,
            IrOp::Constant,
            IrOp::Add,
            IrOp::LocalStore,
            IrOp::LocalLoad,
            IrOp::Return
        ]
    );
}

#[test]
fn unknown_forward_self_and_arity_callees_fail_without_ir() {
    // Unknown callee.
    {
        let mut fixture = Fixture::new();
        let main = fixture.name(b"main");
        let missing = fixture.name(b"missing");
        let (one, one_literal) = fixture.int_lit(1);
        fixture.literal_const(one_literal, 1);
        let call = fixture.call(missing, vec![one]);
        let ret = fixture.return_stmt(call);
        let main_fn = fixture.function(main, &[], vec![ret]);
        let error = project(&mut fixture, main_fn.funcdef).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Semantic);
        assert!(error.message.contains("unknown callee"), "{}", error.message);
        assert_eq!(fixture.bus.arenas.functions.allocated(), 0);
    }
    // Forward callee.
    {
        let mut fixture = Fixture::new();
        let main = fixture.name(b"main");
        let later = fixture.name(b"later");
        let (one, one_literal) = fixture.int_lit(1);
        fixture.literal_const(one_literal, 1);
        let call = fixture.call(later, vec![one]);
        let ret = fixture.return_stmt(call);
        let main_fn = fixture.function(main, &[], vec![ret]);
        let (zero, zero_literal) = fixture.int_lit(0);
        fixture.literal_const(zero_literal, 0);
        let ret = fixture.return_stmt(zero);
        let later_fn = fixture.function(later, &[], vec![ret]);
        assert!(later_fn.funcdef.index() > main_fn.funcdef.index());
        let error = project(&mut fixture, main_fn.funcdef).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Semantic);
        assert!(error.message.contains("forward callee"), "{}", error.message);
        assert_eq!(fixture.bus.arenas.functions.allocated(), 0);
    }
    // Self callee.
    {
        let mut fixture = Fixture::new();
        let main = fixture.name(b"main");
        let (one, one_literal) = fixture.int_lit(1);
        fixture.literal_const(one_literal, 1);
        let call = fixture.call(main, vec![one]);
        let ret = fixture.return_stmt(call);
        let main_fn = fixture.function(main, &[], vec![ret]);
        let error = project(&mut fixture, main_fn.funcdef).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Semantic);
        assert!(error.message.contains("self-recursive callee"), "{}", error.message);
        assert_eq!(fixture.bus.arenas.functions.allocated(), 0);
    }
    // Arity mismatch.
    {
        let mut fixture = Fixture::new();
        let (add, add_name) = build_add(&mut fixture);
        let main = fixture.name(b"main");
        let (one, one_literal) = fixture.int_lit(1);
        fixture.literal_const(one_literal, 1);
        let call = fixture.call(add_name, vec![one]);
        let ret = fixture.return_stmt(call);
        let main_fn = fixture.function(main, &[], vec![ret]);
        let error = project(&mut fixture, main_fn.funcdef).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Semantic);
        assert!(error.message.contains("arity mismatch"), "{}", error.message);
        assert_eq!(fixture.bus.arenas.functions.allocated(), 0);
        let _ = add;
    }
}

#[test]
fn non_int_argument_and_effectful_call_fail_without_ir() {
    // A checked argument with a non-canonical type is rejected.
    {
        let mut fixture = Fixture::new();
        let (add, add_name) = build_add(&mut fixture);
        let short = {
            let limits = fixture.bus.limits();
            fixture
                .bus
                .arenas
                .types
                .alloc(
                    TypeRecord {
                        kind: TypeKind::Int {
                            rank: IntRank::Short,
                            signed: true,
                        },
                    },
                    &limits,
                )
                .unwrap()
        };
        let (argument, argument_literal) = fixture.literal_node(1);
        fixture.sem_with(
            argument,
            short,
            ValueCategory::NonLvalue,
            EffectMask(0),
        );
        fixture.literal_const(argument_literal, 1);
        let main = fixture.name(b"main");
        let (other, other_literal) = fixture.int_lit(2);
        fixture.literal_const(other_literal, 2);
        let call = fixture.call(add_name, vec![argument, other]);
        let ret = fixture.return_stmt(call);
        let main_fn = fixture.function(main, &[], vec![ret]);
        let error = project(&mut fixture, main_fn.funcdef).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Unsupported);
        assert_eq!(fixture.bus.arenas.functions.allocated(), 0);
        let _ = add;
    }
    // The checked Call fact must carry zero effects.
    {
        let mut fixture = Fixture::new();
        let (add, add_name) = build_add(&mut fixture);
        let main = fixture.name(b"main");
        let (two, two_literal) = fixture.int_lit(2);
        fixture.literal_const(two_literal, 2);
        let (three, three_literal) = fixture.int_lit(3);
        fixture.literal_const(three_literal, 3);
        let call = fixture.raw_call(add_name, vec![two, three], EffectMask(EFFECT_WRITES_LOCAL));
        let ret = fixture.return_stmt(call);
        let main_fn = fixture.function(main, &[], vec![ret]);
        let error = project(&mut fixture, main_fn.funcdef).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Task);
        assert!(error.message.contains("effectful node"), "{}", error.message);
        assert_eq!(fixture.bus.arenas.functions.allocated(), 0);
        let _ = add;
    }
}

#[test]
fn literal_origin_selection_rejects_missing_ambiguous_and_foreign() {
    // Missing literal origin: only a binary fold and the other literal exist.
    {
        let mut fixture = Fixture::new();
        let (add, add_name) = build_add(&mut fixture);
        let main = fixture.name(b"main");
        let (two, two_literal) = fixture.int_lit(2);
        let (three, three_literal) = fixture.int_lit(3);
        fixture.literal_const(three_literal, 3);
        fixture.binary_const(two_literal, three_literal, 5);
        let call = fixture.call(add_name, vec![two, three]);
        let ret = fixture.return_stmt(call);
        let main_fn = fixture.function(main, &[], vec![ret]);
        let error = project(&mut fixture, main_fn.funcdef).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Task);
        assert!(
            error.message.contains("no literal-origin folded constant"),
            "{}",
            error.message
        );
        let _ = add;
    }
    // Ambiguous literal origin: two constants claim the same literal.
    {
        let mut fixture = Fixture::new();
        let (add, add_name) = build_add(&mut fixture);
        let main = fixture.name(b"main");
        let (two, two_literal) = fixture.int_lit(2);
        fixture.literal_const(two_literal, 2);
        fixture.literal_const(two_literal, 2);
        let (three, three_literal) = fixture.int_lit(3);
        fixture.literal_const(three_literal, 3);
        let call = fixture.call(add_name, vec![two, three]);
        let ret = fixture.return_stmt(call);
        let main_fn = fixture.function(main, &[], vec![ret]);
        let error = project(&mut fixture, main_fn.funcdef).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Task);
        assert!(
            error.message.contains("ambiguous folded constants for literal"),
            "{}",
            error.message
        );
        let _ = add;
    }
    // Foreign (dangling) origin is rejected rather than ignored.
    {
        let mut fixture = Fixture::new();
        let (add, add_name) = build_add(&mut fixture);
        let main = fixture.name(b"main");
        let (two, two_literal) = fixture.int_lit(2);
        fixture.literal_const(two_literal, 2);
        let (three, three_literal) = fixture.int_lit(3);
        fixture.literal_const(three_literal, 3);
        fixture.const_record(9, vec![LiteralId::from_index(999)]);
        let call = fixture.call(add_name, vec![two, three]);
        let ret = fixture.return_stmt(call);
        let main_fn = fixture.function(main, &[], vec![ret]);
        let error = project(&mut fixture, main_fn.funcdef).unwrap_err();
        assert_eq!(error.code.group, DiagGroup::Task);
        assert!(
            error.message.contains("foreign literal origin"),
            "{}",
            error.message
        );
        let _ = add;
    }
}

#[test]
fn canonical_path_selects_its_own_fold_among_other_constants() {
    let mut fixture = Fixture::new();
    let main = fixture.name(b"main");
    let (two, two_literal) = fixture.int_lit(2);
    let (three, three_literal) = fixture.int_lit(3);
    let sum = fixture.binary_expr(NodeKind::BinaryAdd, two, three);
    let folded = fixture.binary_const(two_literal, three_literal, 5);
    let (nine, nine_literal) = fixture.int_lit(9);
    fixture.literal_const(nine_literal, 9);
    let ret = fixture.return_stmt(sum);
    let canonical = fixture.function(main, &[], vec![ret]);
    let function = lower(&mut fixture, canonical.funcdef).unwrap();
    let instructions = instructions_for(&fixture, function);
    assert_eq!(ops(&instructions), vec![IrOp::Constant, IrOp::Return]);
    assert_eq!(instructions[0].immediate, Some(folded));
    assert_eq!(fixture.bus.arenas.values.allocated(), 1);
    let _ = nine;
}
