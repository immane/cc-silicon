// ============================================================================
// sprint_t17.rs — T17: interpreted-work boundary 4096/4097.
//
// The fixture is a structurally valid `/55` direct-call program whose static
// instruction count (1370/1371) is far below the dynamic work it performs.
// `id(x)` executes `LocalLoad` then `Return` (two steps per invocation), and
// `main` calls it 1364 times after materializing one seed constant plus `F`
// unused filler constants. Its trailing `Return` is the last step.
//
//    main steps   = 1 seed + F fillers + 1364 calls + 1 return
//    callee steps = 1364 invocations * 2 (LocalLoad + Return)
//    total        = (1366 + F) + 2728 = 4094 + F
//
// F = 2 is therefore exactly 4096 executed instructions and must complete
// with the seed value 7; F = 3 is 4097 and must fail with the typed work
// budget diagnostic, even though the instruction structure remains valid
// (one entry block per function, unique producers, earlier definitions, and
// exactly one trailing `Return`). The interpreter counts executed work, not
// the size of the committed program.
// ============================================================================

use cc_silicon_compiler::bus::{
    BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord, IntRank,
    IntegerConst, IrOp, Linkage, StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord,
    ValueRecord,
};
use cc_silicon_compiler::chips::{project_vf12_input, Vf12Chip};
use cc_silicon_compiler::diagnostic::Severity;
use cc_silicon_compiler::ids::{
    BlockId, ConstId, FunctionId, NameId, NodeId, RecordRef, ScopeId, SymbolId, TypeId, ValueId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::VF12_CHIP;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, Proposal, ResultValue, TaskDraft, TaskKind, TaskState};

/// Number of `id(x)` invocations in the boundary fixture.
const ID_CALLS: usize = 1364;

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn bootstrap(
    bus: &mut CompilerBus,
    kind: TaskKind,
    payload: Payload,
) -> cc_silicon_compiler::ids::TaskId {
    bus.bootstrap_task(TaskDraft {
        kind,
        owner: VF12_CHIP,
        parent: None,
        payload,
        continuation: None,
    })
    .expect("bootstrap VF12 task")
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

    fn parameter(&mut self) -> SymbolId {
        self.symbol(
            SymbolKind::Object,
            Some(self.int),
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
            .expect("function");
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
            .expect("entry block");
        (function_id, block_id)
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
        local: Option<SymbolId>,
        callee: Option<SymbolId>,
    ) -> cc_silicon_compiler::ids::InstructionId {
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
            .expect("instruction")
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
            Payload::from_refs(vec![RecordRef::Function(main)]),
        );
        let mut input = project_vf12_input(&self.bus, task).expect("VF12 projection of valid IR");
        input.state = TaskState::Running;
        Vf12Chip.compute(&input)
    }
}

/// Hand-derived dynamic work for the fixture: main's own instructions plus
/// the two instructions executed by each `id` invocation.
fn dynamic_steps(filler_constants: usize) -> u64 {
    let main_steps =
        1 /* seed Constant */ + filler_constants as u64 + ID_CALLS as u64 + 1 /* Return */;
    let callee_steps = ID_CALLS as u64 * 2; // LocalLoad + Return per call
    main_steps + callee_steps
}

/// Valid two-function program: `int id(int x) { return x; }` plus a `main`
/// that calls it `ID_CALLS` times, with `filler_constants` extra unused
/// `Constant` instructions before the calls.
fn boundary_program(filler_constants: usize) -> (Program, FunctionId) {
    let mut program = Program::new();

    let callee_signature = program.signature(1);
    let callee_symbol = program.function_symbol(callee_signature);
    let parameter = program.parameter();
    let (_callee, callee_block) =
        program.add_function(callee_symbol, callee_signature, vec![parameter]);
    let loaded = program.load(callee_block, parameter);
    program.ret(callee_block, loaded);

    let main_signature = program.signature(0);
    let main_symbol = program.function_symbol(main_signature);
    let (main, block) = program.add_function(main_symbol, main_signature, vec![]);
    let seed = program.constant_instruction(block, 7);
    for _ in 0..filler_constants {
        let _ = program.constant_instruction(block, 0);
    }
    let mut last = seed;
    for _ in 0..ID_CALLS {
        last = program.call(block, callee_symbol, vec![seed]);
    }
    program.ret(block, last);

    (program, main)
}

fn expect_integer(proposals: &[Proposal], expected: i128) {
    match proposals {
        [Proposal::Complete {
            value: ResultValue::Integer(integer),
            ..
        }] => assert_eq!(
            integer_value(integer),
            expected,
            "completed interpreted value differs from the hand-derived result"
        ),
        other => panic!("expected one Integer completion, got {other:?}"),
    }
}

#[test]
fn interpreted_work_accepts_exactly_4096_dynamic_steps() {
    assert_eq!(
        dynamic_steps(2),
        4096,
        "fixture arithmetic must land exactly on the inclusive boundary"
    );
    let (mut program, main) = boundary_program(2);
    // Every `id` call returns the seed, so the final call result is 7.
    expect_integer(&program.model(main), 7);
}

#[test]
fn interpreted_work_rejects_4097_dynamic_steps_with_typed_failure() {
    assert_eq!(
        dynamic_steps(3),
        4097,
        "fixture arithmetic must land exactly one step past the boundary"
    );
    let (mut program, main) = boundary_program(3);
    // The extra filler is structurally valid; only executed work changes.
    let proposals = program.model(main);
    match &proposals[..] {
        [Proposal::Fail { diagnostic, .. }] => {
            assert_eq!(diagnostic.severity, Severity::Error);
            assert!(
                diagnostic.message.contains("executed step 4097"),
                "budget failure must name the rejected step: {}",
                diagnostic.message
            );
            assert!(
                diagnostic.message.contains("instruction budget 4096"),
                "budget failure must name the inclusive budget: {}",
                diagnostic.message
            );
        }
        other => panic!("expected one typed work-budget failure, got {other:?}"),
    }
}
