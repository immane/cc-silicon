// ============================================================================
// sprint_t25.rs — T25: duplicate producers, duplicate function records, and
// missing/followed Return.
//
// Independent synthetic-IR checks for the VF12 preparation rules. The control
// program returns 2+3, so every rejection below is a deliberately malformed
// graph rather than a fixture that cannot execute:
//
//   * every committed value has exactly one producing instruction, including
//     across function boundaries;
//   * the committed function vector has no duplicate FunctionId, and one
//     SymbolId may not resolve to two FunctionRecords at a Call;
//   * every function has exactly one Return, and it is the last instruction.
//
// Each mutation leaves the other records shaped normally. A verifier that
// dropped the check would execute a value or accept a duplicate definition,
// so every malformed graph must produce exactly one typed failure.
// ============================================================================

use cc_silicon_compiler::bus::{
    BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord, IntRank,
    IntegerConst, IrOp, Linkage, StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord,
    ValueRecord,
};
use cc_silicon_compiler::chips::{project_vf12_input, Vf12Chip, Vf12Input};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{
    BlockId, ConstId, FunctionId, InstructionId, NameId, NodeId, RecordRef, ScopeId, SymbolId,
    TypeId, ValueId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::VF12_CHIP;
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

struct Fixture {
    bus: CompilerBus,
    int: TypeId,
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
            .expect("canonical int type");
        Self { bus, int }
    }

    fn signature(&mut self, parameters: usize) -> TypeId {
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
            .expect("function signature")
    }

    fn symbol(
        &mut self,
        kind: SymbolKind,
        ty: Option<TypeId>,
        storage: StorageDuration,
        linkage: Linkage,
    ) -> SymbolId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name: NameId::from_index(0),
                    scope: ScopeId::from_index(0),
                    kind,
                    ty,
                    linkage,
                    storage,
                    decl: NodeId::from_index(0),
                },
                &limits,
            )
            .expect("symbol")
    }

    fn function_symbol(&mut self, signature: TypeId) -> SymbolId {
        self.symbol(
            SymbolKind::Function,
            Some(signature),
            StorageDuration::Static,
            Linkage::External,
        )
    }

    fn add_function(
        &mut self,
        symbol: SymbolId,
        signature: TypeId,
        parameters: Vec<SymbolId>,
    ) -> (FunctionId, BlockId) {
        let function = FunctionId::from_index(self.bus.arenas.functions.allocated());
        let block = BlockId::from_index(self.bus.arenas.blocks.allocated());
        let limits = self.bus.limits();
        self.bus
            .arenas
            .functions
            .alloc(
                FunctionRecord {
                    symbol,
                    signature,
                    entry: block,
                    linkage: Linkage::External,
                    parameters,
                },
                &limits,
            )
            .expect("function");
        self.bus
            .arenas
            .blocks
            .alloc(
                BlockRecord {
                    function,
                    ordinal: 0,
                },
                &limits,
            )
            .expect("entry block");
        (function, block)
    }

    fn function(&mut self, parameters: &[SymbolId]) -> (FunctionId, BlockId) {
        let signature = self.signature(parameters.len());
        let symbol = self.function_symbol(signature);
        self.add_function(symbol, signature, parameters.to_vec())
    }

    fn value(&mut self) -> ValueId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .values
            .alloc(ValueRecord { ty: self.int }, &limits)
            .expect("value")
    }

    fn constant(&mut self, value: i32) -> ConstId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .consts
            .alloc(
                ConstRecord {
                    ty: self.int,
                    value: ConstValue::Integer(IntegerConst::new(
                        value.unsigned_abs().to_be_bytes().to_vec(),
                        value < 0,
                    )),
                    origins: vec![],
                },
                &limits,
            )
            .expect("constant")
    }

    #[allow(clippy::too_many_arguments)]
    fn instruction(
        &mut self,
        block: BlockId,
        op: IrOp,
        operands: Vec<ValueId>,
        immediate: Option<ConstId>,
        result: Option<ValueId>,
        callee: Option<SymbolId>,
    ) -> InstructionId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .instructions
            .alloc(
                InstructionRecord {
                    op,
                    block,
                    operands,
                    immediate,
                    result,
                    local: None,
                    successors: vec![],
                    callee,
                },
                &limits,
            )
            .expect("instruction")
    }

    fn constant_value(&mut self, block: BlockId, value: i32) -> (ValueId, InstructionId) {
        let constant = self.constant(value);
        let result = self.value();
        let instruction = self.instruction(
            block,
            IrOp::Constant,
            vec![],
            Some(constant),
            Some(result),
            None,
        );
        (result, instruction)
    }

    fn add(&mut self, block: BlockId, lhs: ValueId, rhs: ValueId) -> ValueId {
        let result = self.value();
        self.instruction(block, IrOp::Add, vec![lhs, rhs], None, Some(result), None);
        result
    }

    fn call(&mut self, block: BlockId, callee: SymbolId, arguments: Vec<ValueId>) -> ValueId {
        let result = self.value();
        self.instruction(
            block,
            IrOp::Call,
            arguments,
            None,
            Some(result),
            Some(callee),
        );
        result
    }

    fn ret(&mut self, block: BlockId, value: ValueId) {
        self.instruction(block, IrOp::Return, vec![value], None, None, None);
    }

    fn project(&mut self, main: FunctionId) -> Vf12Input {
        let task = self
            .bus
            .bootstrap_task(TaskDraft {
                kind: TaskKind::VERIFICATION_IR_INTERPRET,
                owner: VF12_CHIP,
                parent: None,
                payload: Payload::from_refs(vec![RecordRef::Function(main)]),
                continuation: None,
            })
            .expect("bootstrap VF12 task");
        let mut input = project_vf12_input(&self.bus, task).expect("VF12 projection");
        input.state = TaskState::Running;
        input
    }

    fn model(&mut self, main: FunctionId) -> Vec<Proposal> {
        let input = self.project(main);
        Vf12Chip.compute(&input)
    }
}

fn integer_value(integer: &IntegerConst) -> i128 {
    let mut magnitude: u128 = 0;
    for byte in &integer.magnitude {
        magnitude = (magnitude << 8) | u128::from(*byte);
    }
    if integer.negative {
        -(magnitude as i128)
    } else {
        magnitude as i128
    }
}

fn expect_integer(proposals: &[Proposal], expected: i128) {
    match proposals {
        [Proposal::Complete {
            value: ResultValue::Integer(integer),
            ..
        }] => assert_eq!(
            integer_value(integer),
            expected,
            "hand-derived integer result"
        ),
        other => panic!("expected one Integer result, got {other:?}"),
    }
}

fn expect_failure(proposals: &[Proposal], expected: &str) {
    match proposals {
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
fn well_formed_ir_control_models_two_plus_three() {
    let mut fixture = Fixture::new();
    let (main, block) = fixture.function(&[]);
    let (two, _) = fixture.constant_value(block, 2);
    let (three, _) = fixture.constant_value(block, 3);
    let sum = fixture.add(block, two, three);
    fixture.ret(block, sum);

    // Hand calculation: 2 + 3 = 5.
    expect_integer(&fixture.model(main), 5);
}

#[test]
fn second_instruction_producing_one_value_is_rejected() {
    let mut fixture = Fixture::new();
    let (main, block) = fixture.function(&[]);
    let (two, _) = fixture.constant_value(block, 2);
    let (three, three_instruction) = fixture.constant_value(block, 3);
    let sum = fixture.add(block, two, three);
    fixture.ret(block, sum);

    // Both Constants now claim `two`; the producer schedule would be
    // [instruction 0 -> two, instruction 1 -> two], so the second insertion
    // must fail before the missing producer for `three` is considered.
    fixture
        .bus
        .arenas
        .instructions
        .get_mut(three_instruction)
        .expect("third instruction")
        .result = Some(two);

    expect_failure(&fixture.model(main), "duplicate value producer");
}

#[test]
fn producer_duplicated_across_functions_is_rejected() {
    let mut fixture = Fixture::new();
    let (_other, other_block) = fixture.function(&[]);
    let (shared_value, _) = fixture.constant_value(other_block, 8);
    let (other_value, _) = fixture.constant_value(other_block, 9);
    fixture.ret(other_block, other_value);

    let (main, main_block) = fixture.function(&[]);
    let (main_value, _) = fixture.constant_value(main_block, 7);
    let duplicate = fixture.constant(8);
    fixture.instruction(
        main_block,
        IrOp::Constant,
        vec![],
        Some(duplicate),
        Some(shared_value),
        None,
    );
    fixture.ret(main_block, main_value);

    // `shared_value` is produced by one Constant in each function. Every value
    // still has a producer and each Return consumes a value from its own
    // function, so only the global duplicate-producer rule can reject this;
    // execution of `main` alone would otherwise return 7.
    expect_failure(&fixture.model(main), "duplicate value producer");
}

#[test]
fn duplicate_function_id_records_are_rejected() {
    let mut fixture = Fixture::new();
    let (main, block) = fixture.function(&[]);
    let (value, _) = fixture.constant_value(block, 5);
    fixture.ret(block, value);

    let mut input = fixture.project(main);
    let duplicate = input.functions[0].clone();
    input.functions.push(duplicate);

    // Two committed bodies now carry the same FunctionId. A map has one slot,
    // so the duplicate is a protocol defect even though each body is valid.
    let proposals = Vf12Chip.compute(&input);
    expect_failure(&proposals, "duplicate function id");
}

#[test]
fn duplicate_function_symbols_are_rejected_at_a_call() {
    let mut fixture = Fixture::new();
    let signature = fixture.signature(0);
    let shared_symbol = fixture.function_symbol(signature);

    let (_first, first_block) = fixture.add_function(shared_symbol, signature, vec![]);
    let (first_value, _) = fixture.constant_value(first_block, 7);
    fixture.ret(first_block, first_value);

    let (_second, second_block) = fixture.add_function(shared_symbol, signature, vec![]);
    let (second_value, _) = fixture.constant_value(second_block, 8);
    fixture.ret(second_block, second_value);

    let (main, main_block) = fixture.function(&[]);
    let call_value = fixture.call(main_block, shared_symbol, vec![]);
    fixture.ret(main_block, call_value);

    // Two FunctionRecords name one SymbolId, so the Call has two possible
    // targets rather than exactly one. It must not silently pick either body.
    expect_failure(
        &fixture.model(main),
        "Call target resolves to duplicate functions",
    );
}

#[test]
fn function_without_return_is_rejected() {
    let mut fixture = Fixture::new();
    let (main, block) = fixture.function(&[]);
    let _ = fixture.constant_value(block, 5);

    // The body has zero terminators; the committed contract requires one.
    expect_failure(
        &fixture.model(main),
        "function must have exactly one Return",
    );
}

#[test]
fn sibling_without_return_is_rejected() {
    let mut fixture = Fixture::new();
    let (main, main_block) = fixture.function(&[]);
    let (main_value, _) = fixture.constant_value(main_block, 5);
    fixture.ret(main_block, main_value);

    let (_other, other_block) = fixture.function(&[]);
    let _ = fixture.constant_value(other_block, 6);

    // The selected function is valid; the sibling has no terminator. Every
    // committed function is validated, not only the entry function.
    expect_failure(
        &fixture.model(main),
        "function must have exactly one Return",
    );
}

#[test]
fn instruction_after_return_is_rejected() {
    let mut fixture = Fixture::new();
    let (main, block) = fixture.function(&[]);
    let (value, _) = fixture.constant_value(block, 5);
    fixture.ret(block, value);
    let _ = fixture.constant_value(block, 6);

    // Exactly one Return exists, but it occupies index 1 of 3 rather than the
    // greatest InstructionId, so it is followed by reachable-looking code.
    expect_failure(&fixture.model(main), "Return is not the last instruction");
}

#[test]
fn second_return_is_rejected() {
    let mut fixture = Fixture::new();
    let (main, block) = fixture.function(&[]);
    let (value, _) = fixture.constant_value(block, 5);
    fixture.ret(block, value);
    fixture.ret(block, value);

    // Two terminators exist where the contract permits exactly one.
    expect_failure(
        &fixture.model(main),
        "function must have exactly one Return",
    );
}
