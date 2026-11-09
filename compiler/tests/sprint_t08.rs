// ============================================================================
// sprint_t08.rs — T08 adversarial regression: nested calls in separate
// argument positions keep left-to-right IR operand order.
//
// The source shape is:
//
//   int one(void)   { return 1; }
//   int two(void)   { return 2; }
//   int three(void) { return 3; }
//   int mix(int a, int b, int c) { return a * 100 + b * 10 + c; }
//   int main(void) { return mix(two(), one(), three()); }
//
// The argument order is intentionally not the symbol-allocation order
// (one, two, three): a sorter or a right-to-left walk must produce a
// different IR sequence. Only `main` is lowered; the earlier definitions
// supply the callee symbols and signatures that T09 resolves. Expected
// instructions are hand-derived from the source, not from the lowerer.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, EffectMask, InstructionRecord, IntRank, IrOp, Linkage, NodeKind, NodeRecord,
    ScopeKind, ScopeRecord, SemRecord, StorageDuration, SymbolKind, SymbolRecord, TypeKind,
    TypeRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{IrFunctionChip, IrFunctionInput, LocalExpr};
use cc_silicon_compiler::ids::{
    BlockId, NameId, NodeId, RecordRef, ScopeId, SymbolId, TokenId, TypeId, ValueId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::IR_FUNCTION_CHIP;
use cc_silicon_compiler::records::G1DraftBody;
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
        Self {
            bus,
            int,
            file_scope,
        }
    }

    fn name(&mut self, text: &[u8]) -> NameId {
        self.bus.intern_name(text).unwrap()
    }

    fn node(&mut self, kind: NodeKind, children: Vec<NodeId>, name: Option<NameId>) -> NodeId {
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
                    literal: None,
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

    fn function_type(&mut self, parameters: usize) -> TypeId {
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

    /// Allocate an earlier function definition and its file-scope symbol.
    /// Only the declarator link and signature are consumed by callee
    /// resolution; the body is deliberately not lowered in this test.
    fn define(&mut self, name: NameId, arity: usize) -> SymbolId {
        let specifiers = self.node(NodeKind::Specifiers, vec![], None);
        let declarator = self.node(NodeKind::Declarator, vec![], Some(name));
        let compound = self.node(NodeKind::Compound, vec![], None);
        self.node(
            NodeKind::FunctionDefinition,
            vec![specifiers, declarator, compound],
            None,
        );
        let signature = self.function_type(arity);
        self.function_symbol(name, signature, declarator)
    }

    fn call(&mut self, callee: NameId, arguments: Vec<NodeId>) -> NodeId {
        let callee_node = self.node(NodeKind::Identifier, vec![], Some(callee));
        let mut children = vec![callee_node];
        children.extend(arguments);
        let call = self.node(NodeKind::Call, children, None);
        self.sem_int(call);
        call
    }
}

fn project(fixture: &mut Fixture, function: NodeId) -> IrFunctionInput {
    let task = fixture
        .bus
        .bootstrap_task(TaskDraft {
            kind: TaskKind::IR_FUNCTION,
            owner: IR_FUNCTION_CHIP,
            parent: None,
            payload: Payload::from_refs(vec![RecordRef::Node(function)]),
            continuation: None,
        })
        .unwrap();
    fixture.bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    cc_silicon_compiler::chips::project_ir_function_input(&fixture.bus, task)
        .expect("T08 projection")
}

fn instructions(input: &IrFunctionInput) -> Vec<InstructionRecord> {
    let proposals = IrFunctionChip.compute(input);
    assert_eq!(proposals.len(), 2, "expected append + complete");
    assert!(
        matches!(&proposals[1], Proposal::Complete { .. }),
        "expected the lowering task to complete"
    );
    let batch = match &proposals[0] {
        Proposal::AppendRecords { task, batch } => {
            assert_eq!(*task, input.task);
            batch
        }
        other => panic!("expected an append batch, got {other:?}"),
    };
    assert_eq!(batch.records.len(), batch.bodies.len());
    batch
        .bodies
        .iter()
        .filter_map(|body| match body {
            G1DraftBody::Instruction(instruction) => Some(instruction.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn nested_calls_in_separate_arguments_lower_left_to_right() {
    let mut fixture = Fixture::new();
    let one_name = fixture.name(b"one");
    let two_name = fixture.name(b"two");
    let three_name = fixture.name(b"three");
    let mix_name = fixture.name(b"mix");
    let main_name = fixture.name(b"main");

    // Allocation order one, two, three differs from the source order below.
    let one = fixture.define(one_name, 0);
    let two = fixture.define(two_name, 0);
    let three = fixture.define(three_name, 0);
    let mix = fixture.define(mix_name, 3);

    // mix(two(), one(), three())
    let two_call = fixture.call(two_name, vec![]);
    let one_call = fixture.call(one_name, vec![]);
    let three_call = fixture.call(three_name, vec![]);
    let mix_call = fixture.call(mix_name, vec![two_call, one_call, three_call]);
    let ret = fixture.node(NodeKind::Return, vec![mix_call], None);
    fixture.sem_int(ret);

    let specifiers = fixture.node(NodeKind::Specifiers, vec![], None);
    let declarator = fixture.node(NodeKind::Declarator, vec![], Some(main_name));
    let compound = fixture.node(NodeKind::Compound, vec![ret], None);
    let main_definition = fixture.node(
        NodeKind::FunctionDefinition,
        vec![specifiers, declarator, compound],
        None,
    );
    let main_signature = fixture.function_type(0);
    let main_symbol = fixture.function_symbol(main_name, main_signature, declarator);
    let limits = fixture.bus.limits();
    fixture
        .bus
        .arenas
        .sem
        .alloc(
            SemRecord {
                node: main_definition,
                ty: main_signature,
                category: ValueCategory::NonLvalue,
                effects: EffectMask(0),
            },
            &limits,
        )
        .unwrap();

    let input = project(&mut fixture, main_definition);
    assert_eq!(input.symbol, main_symbol);
    assert_eq!(input.signature, main_signature);

    // The projection must retain source argument order before lowering.
    let value = match input.local_expr.as_ref() {
        Some(LocalExpr::Program { value, .. }) => value.as_ref(),
        other => panic!("expected a program envelope, got {other:?}"),
    };
    let arguments = match value {
        LocalExpr::Call { callee, args, .. } => {
            assert_eq!(*callee, mix);
            args
        }
        other => panic!("expected the outer call, got {other:?}"),
    };
    assert_eq!(arguments.len(), 3);
    for (argument, expected) in arguments.iter().zip([two, one, three]) {
        match argument {
            LocalExpr::Call { callee, args, .. } => {
                assert_eq!(*callee, expected, "argument callee order");
                assert!(args.is_empty(), "inner calls take no arguments");
            }
            other => panic!("expected a nested call, got {other:?}"),
        }
    }

    // Hand-derived IR: two() -> v0, one() -> v1, three() -> v2,
    // mix(v0, v1, v2) -> v3, then return v3. No constant is materialized.
    let instructions = instructions(&input);
    let ops: Vec<IrOp> = instructions
        .iter()
        .map(|instruction| instruction.op)
        .collect();
    assert_eq!(
        ops,
        vec![IrOp::Call, IrOp::Call, IrOp::Call, IrOp::Call, IrOp::Return]
    );
    assert_eq!(instructions[0].callee, Some(two));
    assert_eq!(instructions[1].callee, Some(one));
    assert_eq!(instructions[2].callee, Some(three));
    assert_eq!(instructions[3].callee, Some(mix));
    assert_eq!(
        instructions[3].operands,
        vec![
            ValueId::from_index(0),
            ValueId::from_index(1),
            ValueId::from_index(2)
        ],
        "outer call arguments must follow source order"
    );
    assert_eq!(instructions[4].operands, vec![ValueId::from_index(3)]);

    for (index, instruction) in instructions.iter().enumerate() {
        assert_eq!(instruction.block, BlockId::from_index(0));
        assert_eq!(instruction.local, None);
        assert!(instruction.immediate.is_none());
        assert!(instruction.successors.is_empty());
        if index < 4 {
            assert_eq!(instruction.result, Some(ValueId::from_index(index as u32)));
        } else {
            assert_eq!(instruction.result, None);
        }
    }
    for instruction in &instructions[..3] {
        assert!(
            instruction.operands.is_empty(),
            "zero-argument inner calls have no operands"
        );
    }
}
