// ============================================================================
// sprint_t23.rs — `/55` adversarial regression: malformed direct-call IR.
//
// The model under test is the VF12 symbolic interpreter. Each case builds a
// synthetic two-function program whose source meaning is:
//
//     int add(int a, int b) { return a + b; }
//     int main(void) { return add(2, 3); }
//
// A complete Call record must carry `callee = Some(add)`, `operands =
// [2, 3]`, `result = Some(v)`, and `immediate`/`local`/`successors` absent.
// The interpreter may model the complete program as 5, but must reject every
// malformed Call field instead of guessing or skipping the operation.
// Expectations are hand-derived from that source and the frozen `/55`
// instruction shape; no parser, semantic, lowering, or target producer is
// claimed here.
// ============================================================================

use cc_silicon_compiler::bus::{
    BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord,
    IntegerConst, IrOp, Linkage, StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord,
    ValueRecord,
};
use cc_silicon_compiler::chips::{project_vf12_input, Vf12Chip};
use cc_silicon_compiler::ids::{
    BlockId, ConstId, FunctionId, InstructionId, NameId, NodeId, RecordRef, ScopeId, SymbolId,
    TypeId, ValueId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::VF12_CHIP;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, Proposal, ResultValue, TaskDraft, TaskKind, TaskState};

struct Program {
    bus: CompilerBus,
    int: TypeId,
}

impl Program {
    fn new() -> Self {
        let mut bus = CompilerBus::new(CompilerConfig::new(
            TargetSpec::aarch64_unknown_linux_gnu_unverified(),
            Dialect::C11,
            OptLevel::O0,
            vec![],
            Limits::fixture(),
        ));
        let limits = bus.limits();
        let int = bus
            .arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Int {
                        rank: cc_silicon_compiler::bus::IntRank::Int,
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

    fn symbol(&mut self, kind: SymbolKind, storage: StorageDuration, linkage: Linkage) -> SymbolId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name: NameId::from_index(0),
                    scope: ScopeId::from_index(0),
                    kind,
                    ty: Some(self.int),
                    linkage,
                    storage,
                    decl: NodeId::from_index(0),
                },
                &limits,
            )
            .unwrap()
    }

    fn function_symbol(&mut self, _signature: TypeId) -> SymbolId {
        self.symbol(
            SymbolKind::Function,
            StorageDuration::Static,
            Linkage::External,
        )
    }

    fn parameter(&mut self) -> SymbolId {
        self.symbol(
            SymbolKind::Object,
            StorageDuration::Automatic,
            Linkage::None,
        )
    }

    fn add_function(
        &mut self,
        symbol: SymbolId,
        signature: TypeId,
        parameters: Vec<SymbolId>,
    ) -> (FunctionId, BlockId) {
        let function_id = FunctionId::from_index(self.bus.arenas.functions.allocated());
        let block_id = BlockId::from_index(self.bus.arenas.blocks.allocated());
        let limits = self.bus.limits();
        self.bus
            .arenas
            .functions
            .alloc(
                FunctionRecord {
                    symbol,
                    signature,
                    entry: block_id,
                    linkage: Linkage::External,
                    parameters,
                },
                &limits,
            )
            .unwrap();
        self.bus
            .arenas
            .blocks
            .alloc(
                BlockRecord {
                    function: function_id,
                    ordinal: 0,
                },
                &limits,
            )
            .unwrap();
        (function_id, block_id)
    }

    fn value(&mut self) -> ValueId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .values
            .alloc(ValueRecord { ty: self.int }, &limits)
            .unwrap()
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

    #[allow(clippy::too_many_arguments)]
    fn instruction(
        &mut self,
        block: BlockId,
        op: IrOp,
        operands: Vec<ValueId>,
        immediate: Option<ConstId>,
        result: Option<ValueId>,
        local: Option<SymbolId>,
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
                    local,
                    successors: vec![],
                    callee,
                },
                &limits,
            )
            .unwrap()
    }

    fn constant_instruction(&mut self, block: BlockId, value: i32) -> ValueId {
        let constant = self.constant(value);
        let result = self.value();
        self.instruction(
            block,
            IrOp::Constant,
            vec![],
            Some(constant),
            Some(result),
            None,
            None,
        );
        result
    }

    fn load(&mut self, block: BlockId, local: SymbolId) -> ValueId {
        let result = self.value();
        self.instruction(
            block,
            IrOp::LocalLoad,
            vec![],
            None,
            Some(result),
            Some(local),
            None,
        );
        result
    }

    fn binary(&mut self, block: BlockId, op: IrOp, lhs: ValueId, rhs: ValueId) -> ValueId {
        let result = self.value();
        self.instruction(block, op, vec![lhs, rhs], None, Some(result), None, None);
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
            None,
            Some(callee),
        );
        result
    }

    fn ret(&mut self, block: BlockId, value: ValueId) {
        self.instruction(block, IrOp::Return, vec![value], None, None, None, None);
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

fn expect_failure(proposals: &[Proposal]) {
    assert!(
        matches!(proposals, [Proposal::Fail { .. }]),
        "expected one typed failure, got {proposals:?}"
    );
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
        }] => assert_eq!(integer_value(integer), expected),
        other => panic!("expected one Integer result, got {other:?}"),
    }
}

/// Build `int add(int a, int b) { return a + b; }`; return its symbol.
fn add_function(program: &mut Program) -> SymbolId {
    let signature = program.signature(2);
    let symbol = program.function_symbol(signature);
    let a = program.parameter();
    let b = program.parameter();
    let (_, block) = program.add_function(symbol, signature, vec![a, b]);
    let left = program.load(block, a);
    let right = program.load(block, b);
    let sum = program.binary(block, IrOp::Add, left, right);
    program.ret(block, sum);
    symbol
}

/// Build `int main(void)`; return its ID and entry block.
fn main_function(program: &mut Program) -> (FunctionId, BlockId) {
    let signature = program.signature(0);
    let symbol = program.function_symbol(signature);
    program.add_function(symbol, signature, vec![])
}

#[test]
fn complete_two_argument_call_is_the_five_baseline() {
    let mut program = Program::new();
    let add = add_function(&mut program);
    let (main, block) = main_function(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    let sum = program.call(block, add, vec![two, three]);
    program.ret(block, sum);
    expect_integer(&program.model(main), 5);
}

#[test]
fn call_without_result_is_rejected() {
    let mut program = Program::new();
    let add = add_function(&mut program);
    let (main, block) = main_function(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    // A complete call would produce a value; this record discards it. The
    // following constant makes the surrounding function otherwise valid, so
    // rejection must come from the Call result field, not a missing Return.
    program.instruction(
        block,
        IrOp::Call,
        vec![two, three],
        None,
        None,
        None,
        Some(add),
    );
    let seven = program.constant_instruction(block, 7);
    program.ret(block, seven);
    expect_failure(&program.model(main));
}

#[test]
fn call_with_missing_argument_is_rejected() {
    let mut program = Program::new();
    let add = add_function(&mut program);
    let (main, block) = main_function(&mut program);
    let two = program.constant_instruction(block, 2);
    // `add` has two parameters; one operand is incomplete.
    let sum = program.call(block, add, vec![two]);
    program.ret(block, sum);
    expect_failure(&program.model(main));
}

#[test]
fn call_with_extra_argument_is_rejected() {
    let mut program = Program::new();
    let add = add_function(&mut program);
    let (main, block) = main_function(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    let four = program.constant_instruction(block, 4);
    // `add` has two parameters; the third operand is extra.
    let sum = program.call(block, add, vec![two, three, four]);
    program.ret(block, sum);
    expect_failure(&program.model(main));
}

#[test]
fn call_with_immediate_field_is_rejected() {
    let mut program = Program::new();
    let add = add_function(&mut program);
    let (main, block) = main_function(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    let marker = program.constant(2);
    let result = program.value();
    // `immediate` belongs only to Constant instructions.
    program.instruction(
        block,
        IrOp::Call,
        vec![two, three],
        Some(marker),
        Some(result),
        None,
        Some(add),
    );
    program.ret(block, result);
    expect_failure(&program.model(main));
}

#[test]
fn call_with_local_field_is_rejected() {
    let mut program = Program::new();
    let add = add_function(&mut program);
    let (main, block) = main_function(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    let local = program.parameter();
    let result = program.value();
    // `local` belongs only to LocalLoad/LocalStore instructions.
    program.instruction(
        block,
        IrOp::Call,
        vec![two, three],
        None,
        Some(result),
        Some(local),
        Some(add),
    );
    program.ret(block, result);
    expect_failure(&program.model(main));
}

#[test]
fn call_with_successor_field_is_rejected() {
    let mut program = Program::new();
    let add = add_function(&mut program);
    let (main, block) = main_function(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    let result = program.value();
    let call = program.instruction(
        block,
        IrOp::Call,
        vec![two, three],
        None,
        Some(result),
        None,
        Some(add),
    );
    program
        .bus
        .arenas
        .instructions
        .get_mut(call)
        .unwrap()
        .successors = vec![block];
    program.ret(block, result);
    expect_failure(&program.model(main));
}

#[test]
fn call_without_callee_field_is_rejected() {
    let mut program = Program::new();
    add_function(&mut program);
    let (main, block) = main_function(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    let result = program.value();
    // `callee` is mandatory for Call; operands alone do not identify it.
    program.instruction(
        block,
        IrOp::Call,
        vec![two, three],
        None,
        Some(result),
        None,
        None,
    );
    program.ret(block, result);
    expect_failure(&program.model(main));
}
