// ============================================================================
// sprint_t16.rs — independent `/55` VF12 live-call-frame depth boundary.
//
// The direct-call interpreter counts the selected entry function as frame 1
// and every simultaneously live callee as one more frame, up to eight.  These
// synthetic-IR cases build a single chain in which every callee receives the
// caller's accumulated value and adds one:
//
//   main(10) -> f1 -> f2 -> ... -> f7
//
// Eight live frames must model 10 + 7 = 17.  Adding f8 makes the ninth
// invocation, which must return exactly one structured `Task:4` failure and
// must not complete with a truncated or fabricated value.
// ============================================================================

use cc_silicon_compiler::bus::{
    BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord,
    IntegerConst, IrOp, Linkage, StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord,
    ValueRecord,
};
use cc_silicon_compiler::chips::{project_vf12_input, Vf12Chip};
use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticCode, Severity};
use cc_silicon_compiler::ids::{
    BlockId, ChipId, ConstId, FunctionId, NameId, NodeId, RecordRef, ScopeId, SymbolId, TaskId,
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
                    scope: ScopeId::from_index(0),
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

    fn int_parameter(&mut self) -> SymbolId {
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
    ) {
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
            .unwrap();
    }

    fn constant_value(&mut self, block: BlockId, value: i32) -> ValueId {
        let immediate = self.constant(value);
        let result = self.value();
        self.instruction(
            block,
            IrOp::Constant,
            vec![],
            Some(immediate),
            Some(result),
            None,
            None,
        );
        result
    }

    fn load(&mut self, block: BlockId, parameter: SymbolId) -> ValueId {
        let result = self.value();
        self.instruction(
            block,
            IrOp::LocalLoad,
            vec![],
            None,
            Some(result),
            Some(parameter),
            None,
        );
        result
    }

    fn add(&mut self, block: BlockId, lhs: ValueId, rhs: ValueId) -> ValueId {
        let result = self.value();
        self.instruction(
            block,
            IrOp::Add,
            vec![lhs, rhs],
            None,
            Some(result),
            None,
            None,
        );
        result
    }

    fn call(&mut self, block: BlockId, callee: SymbolId, argument: ValueId) -> ValueId {
        let result = self.value();
        self.instruction(
            block,
            IrOp::Call,
            vec![argument],
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

    fn model(&mut self, main: FunctionId) -> (TaskId, Vec<Proposal>) {
        let task = bootstrap(
            &mut self.bus,
            TaskKind::VERIFICATION_IR_INTERPRET,
            VF12_CHIP,
            Payload::from_refs(vec![RecordRef::Function(main)]),
        );
        let mut input = project_vf12_input(&self.bus, task).expect("VF12 projection");
        input.state = TaskState::Running;
        (task, Vf12Chip.compute(&input))
    }
}

/// Build a straight chain with `frames` simultaneously live invocations:
/// `main` plus `frames - 1` one-parameter callees.  Every callee adds one to
/// the incoming value, so the modeled result is `seed + (frames - 1)`.
/// Eight frames are legal; nine exceed the contract's live-frame budget.
fn frame_chain(frames: usize, seed: i32) -> (Fixture, FunctionId, i32) {
    assert!(frames >= 2, "a chain needs main and at least one callee");
    let mut fixture = Fixture::new();

    let main_signature = fixture.signature(0);
    let main_symbol = fixture.function_symbol(main_signature);
    let (main, main_block) = fixture.add_function(main_symbol, main_signature, vec![]);

    let mut callees: Vec<(SymbolId, SymbolId, BlockId)> = Vec::new();
    for _ in 1..frames {
        let signature = fixture.signature(1);
        let symbol = fixture.function_symbol(signature);
        let parameter = fixture.int_parameter();
        let (_, block) = fixture.add_function(symbol, signature, vec![parameter]);
        callees.push((symbol, parameter, block));
    }

    let start = fixture.constant_value(main_block, seed);
    let first = fixture.call(main_block, callees[0].0, start);
    fixture.ret(main_block, first);

    for (index, (_, parameter, block)) in callees.iter().enumerate() {
        let received = fixture.load(*block, *parameter);
        let one = fixture.constant_value(*block, 1);
        let incremented = fixture.add(*block, received, one);
        let returned = match callees.get(index + 1) {
            Some((next, _, _)) => fixture.call(*block, *next, incremented),
            None => incremented,
        };
        fixture.ret(*block, returned);
    }

    let expected = seed + (frames as i32 - 1);
    (fixture, main, expected)
}

#[test]
fn eight_live_frames_complete_with_the_hand_computed_depth_eight_value() {
    let (mut fixture, main, expected) = frame_chain(8, 10);
    // main(10) -> f1(11) -> ... -> f7(17): eight simultaneous frames.
    assert_eq!(expected, 17);

    let (_, proposals) = fixture.model(main);
    match proposals.as_slice() {
        [Proposal::Complete {
            value: ResultValue::Integer(integer),
            ..
        }] => assert_eq!(integer_value(integer), 17),
        other => panic!("expected one Integer completion for eight frames, got {other:?}"),
    }
}

#[test]
fn ninth_live_frame_fails_with_one_explicit_typed_diagnostic() {
    let (mut fixture, main, expected) = frame_chain(9, 10);
    // If the ninth invocation were truncated instead of rejected, the chain
    // would fabricate f8's incoming value 18.
    assert_eq!(expected, 18);

    let (task, proposals) = fixture.model(main);
    match proposals.as_slice() {
        [Proposal::Fail {
            task: failed,
            diagnostic,
        }] => {
            assert_eq!(*failed, task);
            assert_eq!(diagnostic.severity, Severity::Error);
            assert_eq!(diagnostic.code, DiagnosticCode::new(DiagGroup::Task, 4));
            assert!(
                diagnostic
                    .message
                    .contains("call depth 9 exceeds live-frame budget 8"),
                "expected the depth-nine diagnostic, got {}",
                diagnostic.message
            );
        }
        other => panic!("expected exactly one typed Failure for nine frames, got {other:?}"),
    }
}
