// ============================================================================
// sprint_t24.rs — T24: foreign IR ownership and use-before-definition.
//
// Independent, synthetic IR checks for the VF12 ownership rules. A valid
// control program returns 2+3, so every hand-derived rejection below is a
// deliberately broken graph, not a fixture that cannot start:
//
//   * a block may only belong to a committed function;
//   * a function must own exactly one entry block and its `entry` must name it;
//   * an operand may only name a value produced by an earlier instruction in
//     the same function;
//   * every committed value must have exactly one producing instruction.
//
// The mutations preserve otherwise well-formed records; only the ownership or
// definition-order link is changed. A verifier that silently drops the check
// would model a value and pass, so each case must return one typed failure.
// ============================================================================

use cc_silicon_compiler::bus::{
    BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord, IntRank,
    IntegerConst, IrOp, Linkage, StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord,
    ValueRecord,
};
use cc_silicon_compiler::chips::{project_vf12_input, Vf12Chip};
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
            .unwrap();
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
            .unwrap()
    }

    fn function(&mut self, parameters: &[SymbolId]) -> (FunctionId, BlockId) {
        let signature = self.signature(parameters.len());
        let limits = self.bus.limits();
        let symbol = self
            .bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name: NameId::from_index(0),
                    scope: ScopeId::from_index(0),
                    kind: SymbolKind::Function,
                    ty: Some(signature),
                    linkage: Linkage::External,
                    storage: StorageDuration::Static,
                    decl: NodeId::from_index(0),
                },
                &limits,
            )
            .unwrap();
        let function = FunctionId::from_index(self.bus.arenas.functions.allocated());
        let block = BlockId::from_index(self.bus.arenas.blocks.allocated());
        self.bus
            .arenas
            .functions
            .alloc(
                FunctionRecord {
                    symbol,
                    signature,
                    entry: block,
                    linkage: Linkage::External,
                    parameters: parameters.to_vec(),
                },
                &limits,
            )
            .unwrap();
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
            .unwrap();
        (function, block)
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
            .unwrap()
    }

    fn value(&mut self) -> ValueId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .values
            .alloc(ValueRecord { ty: self.int }, &limits)
            .unwrap()
    }

    fn emit(
        &mut self,
        block: BlockId,
        op: IrOp,
        operands: Vec<ValueId>,
        immediate: Option<ConstId>,
        result: Option<ValueId>,
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
                    callee: None,
                },
                &limits,
            )
            .unwrap()
    }

    fn constant_value(&mut self, block: BlockId, value: i32) -> ValueId {
        let constant = self.constant(value);
        let result = self.value();
        self.emit(block, IrOp::Constant, vec![], Some(constant), Some(result));
        result
    }

    fn add(&mut self, block: BlockId, lhs: ValueId, rhs: ValueId) -> (ValueId, InstructionId) {
        let result = self.value();
        let instruction = self.emit(block, IrOp::Add, vec![lhs, rhs], None, Some(result));
        (result, instruction)
    }

    fn ret(&mut self, block: BlockId, value: ValueId) {
        self.emit(block, IrOp::Return, vec![value], None, None);
    }

    fn model(&mut self, main: FunctionId) -> Vec<Proposal> {
        let task = self
            .bus
            .bootstrap_task(TaskDraft {
                kind: TaskKind::VERIFICATION_IR_INTERPRET,
                owner: VF12_CHIP,
                parent: None,
                payload: Payload::from_refs(vec![RecordRef::Function(main)]),
                continuation: None,
            })
            .unwrap();
        let mut input = project_vf12_input(&self.bus, task).expect("VF12 projection");
        input.state = TaskState::Running;
        Vf12Chip.compute(&input)
    }
}

fn expect_integer(proposals: &[Proposal], expected: i128) {
    match proposals {
        [Proposal::Complete {
            value: ResultValue::Integer(integer),
            ..
        }] => {
            let mut magnitude: u128 = 0;
            for byte in &integer.magnitude {
                magnitude = (magnitude << 8) | u128::from(*byte);
            }
            let actual = if integer.negative {
                -(magnitude as i128)
            } else {
                magnitude as i128
            };
            assert_eq!(actual, expected);
        }
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
    let two = fixture.constant_value(block, 2);
    let three = fixture.constant_value(block, 3);
    let (sum, _) = fixture.add(block, two, three);
    fixture.ret(block, sum);

    // Hand calculation: 2 + 3 = 5.
    expect_integer(&fixture.model(main), 5);
}

#[test]
fn block_must_belong_to_a_committed_function() {
    let mut fixture = Fixture::new();
    let (main, block) = fixture.function(&[]);
    let constant = fixture.constant_value(block, 1);
    fixture.ret(block, constant);

    // The block is reassigned to a function ID that was never committed.
    fixture.bus.arenas.blocks.get_mut(block).unwrap().function = FunctionId::from_index(999);

    expect_failure(&fixture.model(main), "block names a missing function");
}

#[test]
fn function_must_own_its_entry_block() {
    let mut fixture = Fixture::new();
    let (main, main_block) = fixture.function(&[]);
    let main_constant = fixture.constant_value(main_block, 1);
    fixture.ret(main_block, main_constant);
    let (other, other_block) = fixture.function(&[]);
    let other_constant = fixture.constant_value(other_block, 2);
    fixture.ret(other_block, other_constant);

    // Reassigning main's block to a sibling leaves main owning no block.
    fixture
        .bus
        .arenas
        .blocks
        .get_mut(main_block)
        .unwrap()
        .function = other;

    expect_failure(
        &fixture.model(main),
        "function does not own exactly one entry block",
    );
}

#[test]
fn function_entry_must_name_its_own_block() {
    let mut fixture = Fixture::new();
    let (main, main_block) = fixture.function(&[]);
    let constant = fixture.constant_value(main_block, 1);
    fixture.ret(main_block, constant);
    let (_other, other_block) = fixture.function(&[]);
    let other_constant = fixture.constant_value(other_block, 2);
    fixture.ret(other_block, other_constant);

    // main still owns main_block, but its entry points into the sibling.
    fixture.bus.arenas.functions.get_mut(main).unwrap().entry = other_block;

    expect_failure(
        &fixture.model(main),
        "function entry does not name its block",
    );
}

#[test]
fn operand_must_not_cross_a_function_boundary() {
    let mut fixture = Fixture::new();
    let (main, main_block) = fixture.function(&[]);
    let main_value = fixture.constant_value(main_block, 2);
    fixture.ret(main_block, main_value);
    let (_other, other_block) = fixture.function(&[]);
    let lhs = fixture.constant_value(other_block, 3);
    let (sum, add_instruction) = fixture.add(other_block, lhs, lhs);
    fixture.ret(other_block, sum);

    // A value produced in main is substituted into other's Add.
    fixture
        .bus
        .arenas
        .instructions
        .get_mut(add_instruction)
        .unwrap()
        .operands[0] = main_value;

    expect_failure(&fixture.model(main), "operand crosses a function boundary");
}

#[test]
fn operand_must_be_defined_before_use() {
    let mut fixture = Fixture::new();
    let (main, block) = fixture.function(&[]);
    let one_const = fixture.constant(1);
    let two_const = fixture.constant(2);
    let use_value = fixture.value();
    let one_value = fixture.value();
    let two_value = fixture.value();

    // The Add (instruction 0) consumes values produced by later Constants.
    fixture.emit(
        block,
        IrOp::Add,
        vec![one_value, two_value],
        None,
        Some(use_value),
    );
    fixture.emit(
        block,
        IrOp::Constant,
        vec![],
        Some(one_const),
        Some(one_value),
    );
    fixture.emit(
        block,
        IrOp::Constant,
        vec![],
        Some(two_const),
        Some(two_value),
    );
    fixture.emit(block, IrOp::Return, vec![use_value], None, None);

    expect_failure(
        &fixture.model(main),
        "operand is used before its definition",
    );
}

#[test]
fn committed_value_must_have_a_producer() {
    let mut fixture = Fixture::new();
    let (main, block) = fixture.function(&[]);
    let returned = fixture.constant_value(block, 5);
    fixture.ret(block, returned);

    // This value is canonical `int` but no instruction defines it.
    let _unowned = fixture.value();

    expect_failure(&fixture.model(main), "committed value has no producer");
}
