// ============================================================================
// call_verify.rs — `/55` direct-call verification acceptance.
//
// Synthetic multi-function IR exercises the VF12 interpreter (direct calls,
// parameter binding, frames, budgets, and its failure surface) without
// claiming a parser, semantic, lowering, target, or native producer. Synthetic
// AST records cover the VF05 call/parameter shapes and the VF06 per-function
// signature plus callee/LHS identifier skips. Existing canonical and
// `/49`–`/53` VF12 results stay on their old result carriers.
// ============================================================================

use cc_silicon_compiler::bus::{
    BlockRecord, CompilerBus, ConstRecord, ConstValue, EffectMask, FunctionRecord,
    InstructionRecord, IntegerConst, IrOp, Linkage, LiteralRecord, NodeKind, NodeRecord, SemRecord,
    StorageDuration, SymbolKind, SymbolRecord, TokenKind, TokenRecord, TypeKind, TypeRecord,
    ValueCategory, ValueRecord,
};
use cc_silicon_compiler::chips::{
    project_vf05_input, project_vf06_input, project_vf12_input, Vf05Chip, Vf05Input, Vf06Chip,
    Vf12Chip,
};
use cc_silicon_compiler::ids::{
    BlockId, ChipId, ConstId, FunctionId, InstructionId, LiteralId, NameId, NodeId, PpTokenId,
    RecordRef, SemId, SpanId, SymbolId, TaskId, TokenId, TypeId, ValueId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{VF05_CHIP, VF06_CHIP, VF12_CHIP};
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, Proposal, ResultValue, TaskDraft, TaskKind, TaskState};
use std::collections::{BTreeMap, BTreeSet};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn bootstrap(bus: &mut CompilerBus, kind: TaskKind, owner: ChipId, payload: Payload) -> TaskId {
    bus.bootstrap_task(TaskDraft {
        kind,
        owner,
        parent: None,
        payload,
        continuation: None,
    })
    .unwrap()
}

fn expect_fail(proposals: &[Proposal]) {
    assert!(
        matches!(proposals, [Proposal::Fail { .. }]),
        "expected one typed failure, got {proposals:?}"
    );
}

fn expect_ack(proposals: &[Proposal]) {
    match proposals {
        [Proposal::Complete {
            value: ResultValue::Ack,
            ..
        }] => {}
        other => panic!("expected one Ack, got {other:?}"),
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

// ---------------------------------------------------------------------------
// VF12 synthetic IR builder.
// ---------------------------------------------------------------------------

struct Program {
    bus: CompilerBus,
    int: TypeId,
}

impl Program {
    fn new() -> Self {
        let mut bus = new_bus();
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
                    scope: cc_silicon_compiler::ids::ScopeId::from_index(0),
                    kind,
                    ty,
                    linkage,
                    storage,
                    decl: NodeId::from_index(0),
                },
                &limits,
            )
            .unwrap()
    }

    fn function_symbol(&mut self, signature: TypeId) -> SymbolId {
        self.symbol(
            SymbolKind::Function,
            Some(signature),
            StorageDuration::Static,
            Linkage::External,
        )
    }

    fn parameter(&mut self) -> SymbolId {
        self.symbol(
            SymbolKind::Object,
            Some(self.int),
            StorageDuration::Automatic,
            Linkage::None,
        )
    }

    fn local(&mut self) -> SymbolId {
        self.parameter()
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

    fn store(&mut self, block: BlockId, local: SymbolId, value: ValueId) {
        self.instruction(
            block,
            IrOp::LocalStore,
            vec![value],
            None,
            None,
            Some(local),
            None,
        );
    }

    fn binary(&mut self, block: BlockId, op: IrOp, lhs: ValueId, rhs: ValueId) -> ValueId {
        let result = self.value();
        self.instruction(block, op, vec![lhs, rhs], None, Some(result), None, None);
        result
    }

    fn neg(&mut self, block: BlockId, operand: ValueId) -> ValueId {
        let result = self.value();
        self.instruction(
            block,
            IrOp::Neg,
            vec![operand],
            None,
            Some(result),
            None,
            None,
        );
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
        let task = bootstrap(
            &mut self.bus,
            TaskKind::VERIFICATION_IR_INTERPRET,
            VF12_CHIP,
            Payload::from_refs(vec![RecordRef::Function(main)]),
        );
        let mut input = project_vf12_input(&self.bus, task).expect("VF12 projection");
        input.state = TaskState::Running;
        Vf12Chip.compute(&input)
    }
}

/// Build the canonical two-parameter `add` function.
fn add_program() -> (Program, SymbolId, FunctionId, ValueId) {
    let mut program = Program::new();
    let signature = program.signature(2);
    let symbol = program.function_symbol(signature);
    let a = program.parameter();
    let b = program.parameter();
    let (function, block) = program.add_function(symbol, signature, vec![a, b]);
    let left = program.load(block, a);
    let right = program.load(block, b);
    let sum = program.binary(block, IrOp::Add, left, right);
    program.ret(block, sum);
    (program, symbol, function, left)
}

/// Add a `main(void)` with no parameters.
fn add_main(program: &mut Program) -> (FunctionId, BlockId) {
    let signature = program.signature(0);
    let symbol = program.function_symbol(signature);
    program.add_function(symbol, signature, vec![])
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

#[test]
fn direct_call_models_add_two_three_as_five() {
    let (mut program, add_symbol, _add, _add_value) = add_program();
    let (main, block) = add_main(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    let sum = program.call(block, add_symbol, vec![two, three]);
    program.ret(block, sum);
    expect_integer(&program.model(main), 5);
}

/// Build `inc` and `add` for nested-call programs.
fn inc_add_program() -> (Program, FunctionId, FunctionId) {
    let mut program = Program::new();
    let inc_signature = program.signature(1);
    let inc_symbol = program.function_symbol(inc_signature);
    let x = program.parameter();
    let (inc, inc_block) = program.add_function(inc_symbol, inc_signature, vec![x]);
    let loaded = program.load(inc_block, x);
    let one = program.constant_instruction(inc_block, 1);
    let incremented = program.binary(inc_block, IrOp::Add, loaded, one);
    program.ret(inc_block, incremented);

    let add_signature = program.signature(2);
    let add_symbol = program.function_symbol(add_signature);
    let a = program.parameter();
    let b = program.parameter();
    let (add, add_block) = program.add_function(add_symbol, add_signature, vec![a, b]);
    let left = program.load(add_block, a);
    let right = program.load(add_block, b);
    let sum = program.binary(add_block, IrOp::Add, left, right);
    program.ret(add_block, sum);
    (program, inc, add)
}

#[test]
fn nested_calls_bind_arguments_left_to_right_and_model_seven() {
    let (mut program, inc, add) = inc_add_program();
    let inc_symbol = program.bus.arenas.functions.get(inc).unwrap().symbol;
    let add_symbol = program.bus.arenas.functions.get(add).unwrap().symbol;
    let (main, block) = add_main(&mut program);
    let two = program.constant_instruction(block, 2);
    let incremented_two = program.call(block, inc_symbol, vec![two]);
    let three = program.constant_instruction(block, 3);
    let incremented_three = program.call(block, inc_symbol, vec![three]);
    let sum = program.call(block, add_symbol, vec![incremented_two, incremented_three]);
    program.ret(block, sum);
    expect_integer(&program.model(main), 7);
}

#[test]
fn call_results_added_in_caller_model_seven() {
    let (mut program, inc, _add) = inc_add_program();
    let inc_symbol = program.bus.arenas.functions.get(inc).unwrap().symbol;
    let (main, block) = add_main(&mut program);
    let two = program.constant_instruction(block, 2);
    let incremented_two = program.call(block, inc_symbol, vec![two]);
    let three = program.constant_instruction(block, 3);
    let incremented_three = program.call(block, inc_symbol, vec![three]);
    let sum = program.binary(block, IrOp::Add, incremented_two, incremented_three);
    program.ret(block, sum);
    expect_integer(&program.model(main), 7);
}

#[test]
fn caller_local_plus_call_models_five() {
    let (mut program, add_symbol, _add, _add_value) = add_program();
    let (main, block) = add_main(&mut program);
    let local = program.local();
    let two = program.constant_instruction(block, 2);
    program.store(block, local, two);
    let loaded = program.load(block, local);
    let three = program.constant_instruction(block, 3);
    let sum = program.call(block, add_symbol, vec![loaded, three]);
    program.ret(block, sum);
    expect_integer(&program.model(main), 5);
}

#[test]
fn eight_parameters_bind_in_declaration_order() {
    let mut program = Program::new();
    let signature = program.signature(8);
    let symbol = program.function_symbol(signature);
    let parameters: Vec<SymbolId> = (0..8).map(|_| program.parameter()).collect();
    let (function, block) = program.add_function(symbol, signature, parameters.clone());
    let mut accumulator = program.load(block, parameters[0]);
    for parameter in &parameters[1..] {
        let ten = program.constant_instruction(block, 10);
        let scaled = program.binary(block, IrOp::Mul, accumulator, ten);
        let loaded = program.load(block, *parameter);
        accumulator = program.binary(block, IrOp::Add, scaled, loaded);
    }
    program.ret(block, accumulator);

    let (main, main_block) = add_main(&mut program);
    let arguments: Vec<ValueId> = (1..=8)
        .map(|value| program.constant_instruction(main_block, value))
        .collect();
    let result = program.call(main_block, symbol, arguments);
    program.ret(main_block, result);
    expect_integer(&program.model(main), 12_345_678);
    let _ = function;
}

#[test]
fn negative_result_keeps_its_sign() {
    let (mut program, add_symbol, _add, _add_value) = add_program();
    let (main, block) = add_main(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    let sum = program.call(block, add_symbol, vec![two, three]);
    let negative = program.neg(block, sum);
    program.ret(block, negative);
    expect_integer(&program.model(main), -5);
}

#[test]
fn repeated_calls_preserve_caller_values_and_rebind_parameters() {
    let (mut program, add_symbol, _add, _add_value) = add_program();
    let (main, block) = add_main(&mut program);
    let local = program.local();
    let two = program.constant_instruction(block, 2);
    program.store(block, local, two);
    let first = program.load(block, local);
    let three = program.constant_instruction(block, 3);
    let first_sum = program.call(block, add_symbol, vec![first, three]);
    let second = program.load(block, local);
    let four = program.constant_instruction(block, 4);
    let second_sum = program.call(block, add_symbol, vec![second, four]);
    let total = program.binary(block, IrOp::Add, first_sum, second_sum);
    program.ret(block, total);
    expect_integer(&program.model(main), 11);
}

#[test]
fn parameter_binding_is_isolated_between_calls() {
    let mut program = Program::new();
    let signature = program.signature(2);
    let symbol = program.function_symbol(signature);
    let a = program.parameter();
    let b = program.parameter();
    let (function, block) = program.add_function(symbol, signature, vec![a, b]);
    let left = program.load(block, a);
    let right = program.load(block, b);
    let difference = program.binary(block, IrOp::Sub, left, right);
    program.ret(block, difference);
    let _ = function;

    let (main, main_block) = add_main(&mut program);
    let nine = program.constant_instruction(main_block, 9);
    let four = program.constant_instruction(main_block, 4);
    let first = program.call(main_block, symbol, vec![nine, four]);
    let two = program.constant_instruction(main_block, 2);
    let seven = program.constant_instruction(main_block, 7);
    let second = program.call(main_block, symbol, vec![two, seven]);
    let total = program.binary(main_block, IrOp::Sub, first, second);
    program.ret(main_block, total);
    expect_integer(&program.model(main), 10);
}

#[test]
fn call_cycle_is_rejected_before_execution() {
    let mut program = Program::new();
    let signature = program.signature(0);
    let first_symbol = program.function_symbol(signature);
    let second_symbol = program.function_symbol(signature);
    let (first, first_block) = program.add_function(first_symbol, signature, vec![]);
    let (second, second_block) = program.add_function(second_symbol, signature, vec![]);
    let a = program.call(first_block, second_symbol, vec![]);
    program.ret(first_block, a);
    let b = program.call(second_block, first_symbol, vec![]);
    program.ret(second_block, b);
    let _ = second;
    expect_fail(&program.model(first));
}

#[test]
fn missing_and_dangling_callees_are_rejected() {
    // Missing function with a well-formed Call.
    let mut program = Program::new();
    let signature = program.signature(0);
    let main_symbol = program.function_symbol(signature);
    let (main, block) = program.add_function(main_symbol, signature, vec![]);
    let result = program.value();
    program.instruction(
        block,
        IrOp::Call,
        vec![],
        None,
        Some(result),
        None,
        Some(SymbolId::from_index(9_999)),
    );
    program.ret(block, result);
    expect_fail(&program.model(main));

    // Missing `callee` field.
    let mut program = Program::new();
    let signature = program.signature(0);
    let main_symbol = program.function_symbol(signature);
    let (main, block) = program.add_function(main_symbol, signature, vec![]);
    let result = program.value();
    program.instruction(block, IrOp::Call, vec![], None, Some(result), None, None);
    program.ret(block, result);
    expect_fail(&program.model(main));
}

#[test]
fn duplicate_callee_symbol_is_rejected() {
    let (mut program, add_symbol, _add, _add_value) = add_program();
    let duplicate_signature = program.signature(0);
    let (duplicate, duplicate_block) =
        program.add_function(add_symbol, duplicate_signature, vec![]);
    let zero = program.constant_instruction(duplicate_block, 0);
    program.ret(duplicate_block, zero);
    let _ = duplicate;
    let (main, block) = add_main(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    let sum = program.call(block, add_symbol, vec![two, three]);
    program.ret(block, sum);
    expect_fail(&program.model(main));
}

#[test]
fn call_arity_mismatch_is_rejected() {
    let (mut program, add_symbol, _add, _add_value) = add_program();
    let (main, block) = add_main(&mut program);
    let two = program.constant_instruction(block, 2);
    let sum = program.call(block, add_symbol, vec![two]);
    program.ret(block, sum);
    expect_fail(&program.model(main));
}

#[test]
fn cross_function_value_operand_is_rejected() {
    let (mut program, _add_symbol, _add, add_value) = add_program();
    let (main, block) = add_main(&mut program);
    program.ret(block, add_value);
    expect_fail(&program.model(main));
}

#[test]
fn wrong_block_owner_is_rejected() {
    let (mut program, _add_symbol, add, _add_value) = add_program();
    let (main, _block) = add_main(&mut program);
    let add_block = program.bus.arenas.functions.get(add).unwrap().entry;
    program
        .bus
        .arenas
        .blocks
        .get_mut(add_block)
        .unwrap()
        .function = main;
    expect_fail(&program.model(main));
}

#[test]
fn duplicate_value_and_missing_return_are_rejected() {
    // Duplicate producer.
    let (mut program, _add_symbol, _add, _add_value) = add_program();
    let (main, _block) = add_main(&mut program);
    let ids: Vec<InstructionId> = program
        .bus
        .arenas
        .instructions
        .iter()
        .map(|(id, _)| id)
        .collect();
    let produced: Vec<(InstructionId, ValueId)> = ids
        .iter()
        .filter_map(|id| {
            let body = program.bus.arenas.instructions.get(*id).unwrap();
            body.result.map(|result| (*id, result))
        })
        .collect();
    let (first_id, first_result) = produced[0];
    let second_id = produced[1].0;
    assert_ne!(first_id, second_id);
    program
        .bus
        .arenas
        .instructions
        .get_mut(second_id)
        .unwrap()
        .result = Some(first_result);
    expect_fail(&program.model(main));

    // Missing Return.
    let mut program = Program::new();
    let signature = program.signature(0);
    let symbol = program.function_symbol(signature);
    let (main, block) = program.add_function(symbol, signature, vec![]);
    let _ = program.constant_instruction(block, 1);
    expect_fail(&program.model(main));
}

fn chain_program(functions: usize) -> (Program, FunctionId) {
    assert!(functions >= 2);
    let mut program = Program::new();
    let signature = program.signature(0);
    let symbols: Vec<SymbolId> = (0..functions)
        .map(|_| program.function_symbol(signature))
        .collect();
    let built: Vec<(FunctionId, BlockId)> = symbols
        .iter()
        .map(|symbol| program.add_function(*symbol, signature, vec![]))
        .collect();
    for index in 0..functions - 1 {
        let (_, block) = built[index];
        let value = program.call(block, symbols[index + 1], vec![]);
        program.ret(block, value);
    }
    let (_, last_block) = built[functions - 1];
    let value = program.constant_instruction(last_block, 7);
    program.ret(last_block, value);
    (program, built[0].0)
}

#[test]
fn depth_boundary_accepts_eight_frames_and_rejects_nine() {
    let (mut program, main) = chain_program(8);
    expect_integer(&program.model(main), 7);
    let (mut program, main) = chain_program(9);
    expect_fail(&program.model(main));
}

fn step_program(constants: usize) -> (Program, FunctionId) {
    let mut program = Program::new();
    let signature = program.signature(0);
    let symbol = program.function_symbol(signature);
    let (main, block) = program.add_function(symbol, signature, vec![]);
    let literal = program.constant(42);
    let mut last = None;
    for _ in 0..constants {
        let result = program.value();
        program.instruction(
            block,
            IrOp::Constant,
            vec![],
            Some(literal),
            Some(result),
            None,
            None,
        );
        last = Some(result);
    }
    let last = last.expect("at least one constant");
    program.ret(block, last);
    (program, main)
}

#[test]
fn step_budget_accepts_4096_and_rejects_4097() {
    let (mut program, main) = step_program(4095);
    expect_integer(&program.model(main), 42);
    let (mut program, main) = step_program(4096);
    expect_fail(&program.model(main));
}

#[test]
fn division_by_zero_uninitialized_load_and_malformed_call_are_rejected() {
    // Division by zero.
    let mut program = Program::new();
    let signature = program.signature(0);
    let symbol = program.function_symbol(signature);
    let (main, block) = program.add_function(symbol, signature, vec![]);
    let lhs = program.constant_instruction(block, 5);
    let rhs = program.constant_instruction(block, 0);
    let quotient = program.binary(block, IrOp::Div, lhs, rhs);
    program.ret(block, quotient);
    expect_fail(&program.model(main));

    // Uninitialized local load.
    let mut program = Program::new();
    let signature = program.signature(0);
    let symbol = program.function_symbol(signature);
    let (main, block) = program.add_function(symbol, signature, vec![]);
    let local = program.local();
    let loaded = program.load(block, local);
    program.ret(block, loaded);
    expect_fail(&program.model(main));

    // Call without a result.
    let (mut program, add_symbol, _add, _add_value) = add_program();
    let (main, block) = add_main(&mut program);
    let two = program.constant_instruction(block, 2);
    let three = program.constant_instruction(block, 3);
    program.instruction(
        block,
        IrOp::Call,
        vec![two, three],
        None,
        None,
        None,
        Some(add_symbol),
    );
    expect_fail(&program.model(main));
}

#[test]
fn canonical_constant_return_keeps_the_record_result() {
    let mut program = Program::new();
    let signature = program.signature(0);
    let symbol = program.function_symbol(signature);
    let (main, block) = program.add_function(symbol, signature, vec![]);
    let value = program.constant_instruction(block, 5);
    program.ret(block, value);
    match &program.model(main)[..] {
        [Proposal::Complete {
            value: ResultValue::Record(RecordRef::Const(id)),
            ..
        }] => assert_eq!(id.index(), 0),
        other => panic!("expected modeled Const record, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Synthetic AST builder shared by VF05 and VF06.
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum AstSpec {
    Node {
        kind: NodeKind,
        name: Option<NameId>,
        literal: Option<i64>,
        children: Vec<AstSpec>,
    },
}

fn ast(kind: NodeKind, children: Vec<AstSpec>) -> AstSpec {
    AstSpec::Node {
        kind,
        name: None,
        literal: None,
        children,
    }
}

fn named(kind: NodeKind, name: u32) -> AstSpec {
    AstSpec::Node {
        kind,
        name: Some(NameId::from_index(name)),
        literal: None,
        children: vec![],
    }
}

fn identifier(name: u32) -> AstSpec {
    named(NodeKind::Identifier, name)
}

fn parameter(name: u32) -> AstSpec {
    named(NodeKind::Parameter, name)
}

fn literal(value: i64) -> AstSpec {
    AstSpec::Node {
        kind: NodeKind::IntLiteral,
        name: None,
        literal: Some(value),
        children: vec![],
    }
}

fn specifiers() -> AstSpec {
    ast(NodeKind::Specifiers, vec![])
}

fn declarator(name: u32, parameters: Option<Vec<AstSpec>>) -> AstSpec {
    let mut children = vec![];
    if let Some(parameters) = parameters {
        children.push(ast(NodeKind::ParameterList, parameters));
    }
    AstSpec::Node {
        kind: NodeKind::Declarator,
        name: Some(NameId::from_index(name)),
        literal: None,
        children,
    }
}

fn compound(statements: Vec<AstSpec>) -> AstSpec {
    ast(NodeKind::Compound, statements)
}

fn function_definition(name: u32, parameters: Option<Vec<AstSpec>>, body: Vec<AstSpec>) -> AstSpec {
    ast(
        NodeKind::FunctionDefinition,
        vec![specifiers(), declarator(name, parameters), compound(body)],
    )
}

fn return_expression(expression: AstSpec) -> AstSpec {
    ast(NodeKind::Return, vec![expression])
}

fn binary(op: NodeKind, lhs: AstSpec, rhs: AstSpec) -> AstSpec {
    ast(op, vec![lhs, rhs])
}

fn call(callee: u32, arguments: Vec<AstSpec>) -> AstSpec {
    let mut children = vec![identifier(callee)];
    children.extend(arguments);
    ast(NodeKind::Call, children)
}

fn declaration(name: u32, value: i64) -> AstSpec {
    ast(
        NodeKind::Declaration,
        vec![specifiers(), declarator(name, None), literal(value)],
    )
}

fn assignment(lhs: u32, rhs: AstSpec) -> AstSpec {
    ast(NodeKind::Assignment, vec![identifier(lhs), rhs])
}

fn call_program_ast() -> AstSpec {
    let add = function_definition(
        1,
        Some(vec![parameter(1), parameter(2)]),
        vec![return_expression(binary(
            NodeKind::BinaryAdd,
            identifier(1),
            identifier(2),
        ))],
    );
    let main = function_definition(
        2,
        None,
        vec![
            declaration(1, 2),
            assignment(1, call(1, vec![identifier(1), literal(3)])),
            return_expression(call(1, vec![identifier(1), literal(3)])),
        ],
    );
    ast(NodeKind::TranslationUnit, vec![add, main])
}

fn alloc_ast(
    bus: &mut CompilerBus,
    spec: AstSpec,
    parent: Option<NodeId>,
    nodes: &mut Vec<NodeId>,
    literal_nodes: &mut Vec<(NodeId, LiteralId)>,
) -> NodeId {
    let AstSpec::Node {
        kind,
        name,
        literal: literal_value,
        children,
    } = spec;
    let limits = bus.limits();
    let id = bus
        .arenas
        .nodes
        .alloc(
            NodeRecord {
                kind,
                parent,
                children: vec![],
                first_token: TokenId::from_index(0),
                last_token: TokenId::from_index(0),
                name,
                literal: None,
            },
            &limits,
        )
        .unwrap();
    nodes.push(id);
    if let Some(value) = literal_value {
        let magnitude = value.unsigned_abs().to_be_bytes().to_vec();
        let literal_id = bus
            .arenas
            .literals
            .alloc(
                LiteralRecord {
                    token: None,
                    kind: LiteralKind::Integer,
                    radix: 10,
                    suffix: LiteralSuffix::None,
                    value: magnitude,
                    negative: value < 0,
                    spelling: value.to_string().into_bytes(),
                    candidate_type: Lx08CandidateType::Int,
                },
                &limits,
            )
            .unwrap();
        bus.arenas.nodes.get_mut(id).unwrap().literal = Some(literal_id);
        literal_nodes.push((id, literal_id));
    }
    let mut child_ids = Vec::new();
    for child in children {
        child_ids.push(alloc_ast(bus, child, Some(id), nodes, literal_nodes));
    }
    bus.arenas.nodes.get_mut(id).unwrap().children = child_ids;
    id
}

fn assign_tokens(bus: &mut CompilerBus, id: NodeId, next: &mut u32) -> u32 {
    let first = *next;
    *next += 1;
    let children = bus.arenas.nodes.get(id).unwrap().children.clone();
    let mut last = first;
    for child in children {
        last = assign_tokens(bus, child, next);
    }
    let node = bus.arenas.nodes.get_mut(id).unwrap();
    node.first_token = TokenId::from_index(first);
    node.last_token = TokenId::from_index(last);
    last
}

fn build_ast(bus: &mut CompilerBus, spec: AstSpec) -> NodeId {
    let mut nodes = Vec::new();
    let mut literal_nodes = Vec::new();
    let root = alloc_ast(bus, spec, None, &mut nodes, &mut literal_nodes);
    let limits = bus.limits();
    for index in 0..nodes.len() + 1 {
        let eof = index == nodes.len();
        bus.arenas
            .tokens
            .alloc(
                TokenRecord {
                    kind: if eof {
                        TokenKind::Eof
                    } else {
                        TokenKind::Identifier
                    },
                    span: SpanId::from_index(0),
                    name: if eof {
                        None
                    } else {
                        Some(NameId::from_index(0))
                    },
                    pp_token: PpTokenId::from_index(0),
                },
                &limits,
            )
            .unwrap();
    }
    let mut next = 0u32;
    let last = assign_tokens(bus, root, &mut next);
    for (node, literal) in literal_nodes {
        let token = bus.arenas.nodes.get(node).unwrap().first_token;
        bus.arenas.literals.get_mut(literal).unwrap().token = Some(token);
    }
    assert_eq!(last + 1, nodes.len() as u32);
    root
}

fn run_vf05(bus: &mut CompilerBus, tu: NodeId) -> Vec<Proposal> {
    let task = bootstrap(
        bus,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu)]),
    );
    let mut input = project_vf05_input(bus, task).expect("VF05 projection");
    input.state = TaskState::Running;
    Vf05Chip.compute(&input)
}

#[test]
fn vf05_accepts_parameters_calls_and_call_expressions() {
    let mut bus = new_bus();
    let tu = build_ast(&mut bus, call_program_ast());
    expect_ack(&run_vf05(&mut bus, tu));
}

#[test]
fn vf05_rejects_function_parameter_and_call_budget_shapes() {
    // Empty TU.
    let mut bus = new_bus();
    let tu = build_ast(&mut bus, ast(NodeKind::TranslationUnit, vec![]));
    expect_fail(&run_vf05(&mut bus, tu));

    // Nine parameters.
    let mut bus = new_bus();
    let nine_parameters: Vec<AstSpec> = (0..9).map(parameter).collect();
    let function = function_definition(
        1,
        Some(nine_parameters),
        vec![return_expression(literal(0))],
    );
    let tu = build_ast(&mut bus, ast(NodeKind::TranslationUnit, vec![function]));
    expect_fail(&run_vf05(&mut bus, tu));

    // Nine call arguments.
    let mut bus = new_bus();
    let nine_arguments: Vec<AstSpec> = (0..9).map(literal).collect();
    let main = function_definition(1, None, vec![return_expression(call(1, nine_arguments))]);
    let tu = build_ast(&mut bus, ast(NodeKind::TranslationUnit, vec![main]));
    expect_fail(&run_vf05(&mut bus, tu));

    // 33 functions.
    let mut bus = new_bus();
    let functions: Vec<AstSpec> = (0..33)
        .map(|index| {
            function_definition(
                index,
                None,
                vec![return_expression(literal(i64::from(index)))],
            )
        })
        .collect();
    let tu = build_ast(&mut bus, ast(NodeKind::TranslationUnit, functions));
    expect_fail(&run_vf05(&mut bus, tu));
}

#[test]
fn vf05_rejects_malformed_callee_arguments_and_local_parameter_lists() {
    // Callee is not an Identifier.
    let mut bus = new_bus();
    let main = function_definition(
        1,
        None,
        vec![return_expression(ast(
            NodeKind::Call,
            vec![literal(0), literal(1)],
        ))],
    );
    let tu = build_ast(&mut bus, ast(NodeKind::TranslationUnit, vec![main]));
    expect_fail(&run_vf05(&mut bus, tu));

    // Argument is a Parameter, not an expression.
    let mut bus = new_bus();
    let main = function_definition(
        1,
        None,
        vec![return_expression(ast(
            NodeKind::Call,
            vec![identifier(1), parameter(2)],
        ))],
    );
    let tu = build_ast(&mut bus, ast(NodeKind::TranslationUnit, vec![main]));
    expect_fail(&run_vf05(&mut bus, tu));

    // A local declarator may not own a parameter list.
    let mut bus = new_bus();
    let main = function_definition(
        1,
        None,
        vec![
            ast(
                NodeKind::Declaration,
                vec![
                    specifiers(),
                    declarator(1, Some(vec![parameter(2)])),
                    literal(2),
                ],
            ),
            return_expression(identifier(1)),
        ],
    );
    let tu = build_ast(&mut bus, ast(NodeKind::TranslationUnit, vec![main]));
    expect_fail(&run_vf05(&mut bus, tu));
}

#[test]
fn vf05_node_budget_accepts_4096_and_rejects_4097() {
    let mut bus = new_bus();
    let tu = build_ast(&mut bus, call_program_ast());
    let task = bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu)]),
    );
    let input = project_vf05_input(&bus, task).expect("VF05 projection");
    let filler = |input: &Vf05Input, target: usize| {
        let mut input = input.clone();
        let mut index = 10_000u32;
        while input.tree.len() < target {
            input.tree.push((
                NodeId::from_index(index),
                NodeRecord {
                    kind: NodeKind::Identifier,
                    parent: None,
                    children: vec![],
                    first_token: TokenId::from_index(0),
                    last_token: TokenId::from_index(0),
                    name: Some(NameId::from_index(0)),
                    literal: None,
                },
            ));
            index += 1;
        }
        input
    };
    let mut accepted = filler(&input, 4096);
    accepted.state = TaskState::Running;
    expect_ack(&Vf05Chip.compute(&accepted));
    let mut rejected = filler(&input, 4097);
    rejected.state = TaskState::Running;
    expect_fail(&Vf05Chip.compute(&rejected));
}

// ---------------------------------------------------------------------------
// VF06 synthetic typed facts.
// ---------------------------------------------------------------------------

struct Vf06Fixture {
    bus: CompilerBus,
    tu: NodeId,
    int: TypeId,
    sig0: TypeId,
    sig2: TypeId,
    add_definition: NodeId,
    main_definition: NodeId,
    call_node: NodeId,
    callee_node: NodeId,
    parameter_node: NodeId,
}

fn allocate_vf06_types(bus: &mut CompilerBus) -> (TypeId, TypeId, TypeId) {
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
    let mut signature = |parameters: usize| {
        bus.arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Function {
                        result: int,
                        params: vec![int; parameters],
                        prototype: true,
                        variadic: false,
                    },
                },
                &limits,
            )
            .unwrap()
    };
    let sig0 = signature(0);
    let sig2 = signature(2);
    (int, sig0, sig2)
}

fn preorder(bus: &CompilerBus, root: NodeId) -> Vec<(NodeId, NodeRecord)> {
    let mut result = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let node = bus.arenas.nodes.get(id).unwrap().clone();
        for child in node.children.iter().rev() {
            stack.push(*child);
        }
        result.push((id, node));
    }
    result
}

fn ast_arity(tree: &[(NodeId, NodeRecord)], id: NodeId) -> usize {
    let node = tree.iter().find(|(nid, _)| *nid == id).unwrap().1.clone();
    let declarator = tree
        .iter()
        .find(|(nid, _)| *nid == node.children[1])
        .unwrap()
        .1
        .clone();
    match declarator.children.first() {
        Some(parameter_list) => tree
            .iter()
            .find(|(nid, _)| *nid == *parameter_list)
            .unwrap()
            .1
            .children
            .len(),
        None => 0,
    }
}

fn seed_vf06_facts(
    bus: &mut CompilerBus,
    tree: &[(NodeId, NodeRecord)],
    int: TypeId,
    signatures: &BTreeMap<usize, TypeId>,
) {
    let assignment_lhs: BTreeSet<NodeId> = tree
        .iter()
        .filter(|(_, node)| node.kind == NodeKind::Assignment)
        .filter_map(|(_, node)| node.children.first().copied())
        .collect();
    let call_callees: BTreeSet<NodeId> = tree
        .iter()
        .filter(|(_, node)| node.kind == NodeKind::Call)
        .filter_map(|(_, node)| node.children.first().copied())
        .collect();
    for (id, node) in tree {
        let (ty, effects) = match node.kind {
            NodeKind::FunctionDefinition => {
                let arity = ast_arity(tree, *id);
                (*signatures.get(&arity).unwrap(), EffectMask(0))
            }
            NodeKind::IntLiteral
            | NodeKind::Return
            | NodeKind::BinaryAdd
            | NodeKind::BinarySub
            | NodeKind::BinaryMul
            | NodeKind::BinaryDiv
            | NodeKind::BinaryRem
            | NodeKind::UnaryPlus
            | NodeKind::UnaryMinus
            | NodeKind::Call => (int, EffectMask(0)),
            NodeKind::Assignment => (int, EffectMask(1)),
            NodeKind::Identifier => {
                if assignment_lhs.contains(id) || call_callees.contains(id) {
                    continue;
                }
                (int, EffectMask(0))
            }
            _ => continue,
        };
        let limits = bus.limits();
        bus.arenas
            .sem
            .alloc(
                SemRecord {
                    node: *id,
                    ty,
                    category: ValueCategory::NonLvalue,
                    effects,
                },
                &limits,
            )
            .unwrap();
    }
}

fn vf06_fixture() -> Vf06Fixture {
    let mut bus = new_bus();
    let (int, sig0, sig2) = allocate_vf06_types(&mut bus);
    let tu = build_ast(&mut bus, call_program_ast());
    let tree = preorder(&bus, tu);
    let definitions: Vec<NodeId> = tree
        .iter()
        .filter(|(_, node)| node.kind == NodeKind::FunctionDefinition)
        .map(|(id, _)| *id)
        .collect();
    let call_node = tree
        .iter()
        .find(|(_, node)| node.kind == NodeKind::Call)
        .map(|(id, _)| *id)
        .unwrap();
    let callee_node = bus.arenas.nodes.get(call_node).unwrap().children[0];
    let parameter_node = tree
        .iter()
        .find(|(_, node)| node.kind == NodeKind::Parameter)
        .map(|(id, _)| *id)
        .unwrap();
    let mut signatures = BTreeMap::new();
    signatures.insert(0, sig0);
    signatures.insert(2, sig2);
    seed_vf06_facts(&mut bus, &tree, int, &signatures);
    Vf06Fixture {
        bus,
        tu,
        int,
        sig0,
        sig2,
        add_definition: definitions[0],
        main_definition: definitions[1],
        call_node,
        callee_node,
        parameter_node,
    }
}

fn run_vf06(bus: &mut CompilerBus, tu: NodeId) -> Vec<Proposal> {
    let task = bootstrap(
        bus,
        TaskKind::VERIFICATION_TYPED_INVARIANT,
        VF06_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu)]),
    );
    let mut input = project_vf06_input(bus, task).expect("VF06 projection");
    input.state = TaskState::Running;
    Vf06Chip.compute(&input)
}

#[test]
fn vf06_accepts_multiple_functions_and_skips_callee_and_lhs_identifiers() {
    let mut fixture = vf06_fixture();
    assert_eq!(
        fixture
            .bus
            .arenas
            .sem
            .iter()
            .filter(|(_, sem)| sem.node == fixture.callee_node)
            .count(),
        0
    );
    expect_ack(&run_vf06(&mut fixture.bus, fixture.tu));
}

#[test]
fn vf06_rejects_missing_duplicate_and_mistyped_call_facts() {
    // Missing Call fact.
    let mut fixture = vf06_fixture();
    let sem = fixture
        .bus
        .arenas
        .sem
        .iter()
        .find(|(_, sem)| sem.node == fixture.call_node)
        .map(|(id, _)| id)
        .unwrap();
    fixture.bus.arenas.sem.remove(sem).unwrap();
    expect_fail(&run_vf06(&mut fixture.bus, fixture.tu));

    // Duplicate Call fact.
    let mut fixture = vf06_fixture();
    let sem = fixture
        .bus
        .arenas
        .sem
        .iter()
        .find(|(_, sem)| sem.node == fixture.call_node)
        .map(|(id, body)| (id, body.clone()))
        .unwrap();
    let limits = fixture.bus.limits();
    let mut duplicate = sem.1;
    duplicate.node = fixture.call_node;
    fixture.bus.arenas.sem.alloc(duplicate, &limits).unwrap();
    expect_fail(&run_vf06(&mut fixture.bus, fixture.tu));

    // Mistyped Call fact.
    let mut fixture = vf06_fixture();
    let sem = fixture
        .bus
        .arenas
        .sem
        .iter()
        .find(|(_, sem)| sem.node == fixture.call_node)
        .map(|(id, _)| id)
        .unwrap();
    fixture.bus.arenas.sem.get_mut(sem).unwrap().ty = fixture.sig0;
    expect_fail(&run_vf06(&mut fixture.bus, fixture.tu));
}

#[test]
fn vf06_matches_each_function_to_its_arity_signature() {
    let mut fixture = vf06_fixture();
    let sem = fixture
        .bus
        .arenas
        .sem
        .iter()
        .find(|(_, sem)| sem.node == fixture.add_definition)
        .map(|(id, _)| id)
        .unwrap();
    fixture.bus.arenas.sem.get_mut(sem).unwrap().ty = fixture.sig0;
    expect_fail(&run_vf06(&mut fixture.bus, fixture.tu));

    let mut fixture = vf06_fixture();
    let sem = fixture
        .bus
        .arenas
        .sem
        .iter()
        .find(|(_, sem)| sem.node == fixture.main_definition)
        .map(|(id, _)| id)
        .unwrap();
    fixture.bus.arenas.sem.get_mut(sem).unwrap().ty = fixture.sig2;
    expect_fail(&run_vf06(&mut fixture.bus, fixture.tu));
}

#[test]
fn vf06_rejects_parameter_facts() {
    let mut fixture = vf06_fixture();
    let limits = fixture.bus.limits();
    fixture
        .bus
        .arenas
        .sem
        .alloc(
            SemRecord {
                node: fixture.parameter_node,
                ty: fixture.int,
                category: ValueCategory::NonLvalue,
                effects: EffectMask(0),
            },
            &limits,
        )
        .unwrap();
    expect_fail(&run_vf06(&mut fixture.bus, fixture.tu));
}

#[test]
fn synthetic_ast_records_do_not_mutate_semantic_stores() {
    let mut fixture = vf06_fixture();
    let before = fixture.bus.arenas.sem.allocated();
    expect_ack(&run_vf06(&mut fixture.bus, fixture.tu));
    assert_eq!(fixture.bus.arenas.sem.allocated(), before);
    let mut bus = new_bus();
    let tu = build_ast(&mut bus, call_program_ast());
    let before = bus.arenas.nodes.allocated();
    expect_ack(&run_vf05(&mut bus, tu));
    assert_eq!(bus.arenas.nodes.allocated(), before);
    let _ = SemId::from_index(0);
}
