// ============================================================================
// c15_ir.rs — Wave 2 (`/15`) IR function-lowering slice acceptance.
//
// Covers the frozen closure: the exact M1 IR shape (one function, one
// block, one value, `Constant` + `Return`), the `ir.function` kind,
// kind-to-stage assignment, the IR store-owner allowlist rows, stage/layer
// agreement, the same-`ConstId` handoff without refolding, snapshot bodies,
// tick-lifecycle integration, and the `/15` hash participation. Chain
// inputs come from the real SE slice; `M1-CL-05` completes here (T09
// consumes the fold's `ConstRecord`). FunctionEnd/IR28 stays deferred.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, IrOp, PpTokenKind, PpTokenRecord, SpanRecord,
};
use cc_silicon_compiler::chips::{
    handler_for, IrFunctionChip, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip,
    SeBinChip, SeLitChip, SeRetChip, TyConvChip, TyScopeChip, TySymbolChip, TyTypeChip, Vf06Chip,
    Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{NodeId, RecordRef, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_ir_slice_kind, stage_of, FieldPath, ManifestRegistryError, StoreSchema, IR_FUNCTION_CHIP,
    PA_TU_CHIP, SE_BIN_CHIP, SE_LIT_CHIP, SE_RET_CHIP, STORE_OWNER_ALLOWLIST, TY_CONV_CHIP,
    TY_SCOPE_CHIP, TY_SYMBOL_CHIP, TY_TYPE_CHIP, VF06_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{
    decode_block, decode_function, decode_instruction, decode_value, encode_block, encode_function,
    encode_instruction, encode_value, Snapshot,
};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_ir(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::ir_slice();
    bus.schema = StoreSchema::ir_slice();
    // The fold worker is registered (never driven directly here): SE binary
    // tasks enqueue `const_fold` children whose destination must be owned.
    bus.registrations
        .register(
            cc_silicon_compiler::chips::FoldChip.manifest(),
            &bus.schema,
            &bus.kinds,
        )
        .unwrap();
    for chip in [
        &LxInternChip as &dyn Worker,
        &LxClassifyChip,
        &LxDecodeLiteralChip,
        &PaTuChip,
        &TyTypeChip,
        &TyScopeChip,
        &TySymbolChip,
        &TyConvChip,
        &SeLitChip,
        &SeBinChip,
        &SeRetChip,
        &Vf06Chip,
        &IrFunctionChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(
            TaskKind::LEX_INTERN,
            cc_silicon_compiler::manifest::LX_INTERN_CHIP,
            2,
        )
        .unwrap();
    bus.routing
        .register(
            TaskKind::LEX_CLASSIFY,
            cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
            2,
        )
        .unwrap();
    bus.routing
        .register(
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
            2,
        )
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_TU, PA_TU_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::SYMBOL_INT_TYPE, TY_TYPE_CHIP, 3)
        .unwrap();
    bus.routing
        .register(TaskKind::SYMBOL_FUNC_TYPE, TY_TYPE_CHIP, 3)
        .unwrap();
    bus.routing
        .register(TaskKind::SYMBOL_SCOPE_ENTER, TY_SCOPE_CHIP, 3)
        .unwrap();
    bus.routing
        .register(TaskKind::SYMBOL_SCOPE_EXIT, TY_SCOPE_CHIP, 3)
        .unwrap();
    bus.routing
        .register(TaskKind::SYMBOL_DECLARE, TY_SYMBOL_CHIP, 3)
        .unwrap();
    bus.routing
        .register(TaskKind::SYMBOL_LOOKUP, TY_SYMBOL_CHIP, 3)
        .unwrap();
    bus.routing
        .register(TaskKind::SYMBOL_PROMOTE, TY_CONV_CHIP, 3)
        .unwrap();
    bus.routing
        .register(TaskKind::SYMBOL_COMMON_TYPE, TY_CONV_CHIP, 3)
        .unwrap();
    bus.routing
        .register(TaskKind::SYMBOL_RETURN_CONVERT, TY_CONV_CHIP, 3)
        .unwrap();
    bus.routing
        .register(TaskKind::SEMANTIC_LITERAL_EXPR, SE_LIT_CHIP, 4)
        .unwrap();
    bus.routing
        .register(TaskKind::SEMANTIC_BINARY_EXPR, SE_BIN_CHIP, 4)
        .unwrap();
    bus.routing
        .register(TaskKind::SEMANTIC_RETURN_STMT, SE_RET_CHIP, 4)
        .unwrap();
    bus.routing
        .register(TaskKind::VERIFICATION_TYPED_INVARIANT, VF06_CHIP, 4)
        .unwrap();
    bus.routing
        .register(TaskKind::IR_FUNCTION, IR_FUNCTION_CHIP, 5)
        .unwrap();
    bus.routing
        .register(
            TaskKind::CONSTANT_CONST_FOLD,
            cc_silicon_compiler::manifest::G1_FOLD_CHIP,
            2,
        )
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(LxInternChip).unwrap();
    workers.register(LxClassifyChip).unwrap();
    workers.register(LxDecodeLiteralChip).unwrap();
    workers.register(PaTuChip).unwrap();
    workers.register(TyTypeChip).unwrap();
    workers.register(TyScopeChip).unwrap();
    workers.register(TySymbolChip).unwrap();
    workers.register(TyConvChip).unwrap();
    workers.register(SeLitChip).unwrap();
    workers.register(SeBinChip).unwrap();
    workers.register(SeRetChip).unwrap();
    workers.register(Vf06Chip).unwrap();
    workers.register(IrFunctionChip).unwrap();
    workers
        .register(cc_silicon_compiler::chips::FoldChip)
        .unwrap();
    workers
}

fn seed_source(bus: &mut CompilerBus) -> cc_silicon_compiler::ids::SourceId {
    let mut raw = b"int main(void){return 2+3;}".to_vec();
    raw.push(b'\n');
    let name = bus.intern_name(b"main.c").unwrap();
    bus.alloc_source(name, raw).unwrap()
}

fn seed_pp_layer(
    bus: &mut CompilerBus,
    source: cc_silicon_compiler::ids::SourceId,
) -> Vec<cc_silicon_compiler::ids::PpTokenId> {
    let limits = bus.limits();
    let spans: Vec<(u64, u64)> = vec![
        (0, 3),
        (4, 8),
        (8, 9),
        (9, 13),
        (13, 14),
        (14, 15),
        (15, 21),
        (22, 23),
        (23, 24),
        (24, 25),
        (25, 26),
        (26, 27),
        (28, 28),
    ];
    let mut span_ids = Vec::new();
    for (start, end) in spans {
        span_ids.push(
            bus.arenas
                .spans
                .alloc(
                    SpanRecord {
                        source,
                        start,
                        end,
                        expansion: None,
                    },
                    &limits,
                )
                .unwrap(),
        );
    }
    let tokens: Vec<(PpTokenKind, usize, &[u8])> = vec![
        (PpTokenKind::Identifier, 0, b"int"),
        (PpTokenKind::Identifier, 1, b"main"),
        (PpTokenKind::Punctuator, 2, b"("),
        (PpTokenKind::Identifier, 3, b"void"),
        (PpTokenKind::Punctuator, 4, b")"),
        (PpTokenKind::Punctuator, 5, b"{"),
        (PpTokenKind::Identifier, 6, b"return"),
        (PpTokenKind::PpNumber, 7, b"2"),
        (PpTokenKind::Punctuator, 8, b"+"),
        (PpTokenKind::PpNumber, 9, b"3"),
        (PpTokenKind::Punctuator, 10, b";"),
        (PpTokenKind::Punctuator, 11, b"}"),
        (PpTokenKind::Eof, 12, b""),
    ];
    let mut ids = Vec::new();
    for (kind, span, spelling) in tokens {
        ids.push(
            bus.arenas
                .pp_tokens
                .alloc(
                    PpTokenRecord {
                        kind,
                        span: span_ids[span],
                        spelling: spelling.to_vec(),
                    },
                    &limits,
                )
                .unwrap(),
        );
    }
    ids
}

fn bootstrap(
    bus: &mut CompilerBus,
    kind: TaskKind,
    owner: cc_silicon_compiler::ids::ChipId,
    payload: Payload,
) -> TaskId {
    bus.bootstrap_task(TaskDraft {
        kind,
        owner,
        parent: None,
        payload,
        continuation: None,
    })
    .unwrap()
}

fn tick(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
) -> cc_silicon_compiler::routing::TickReport {
    RoutingShell
        .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
        .unwrap()
}

fn completed_value(
    report: &cc_silicon_compiler::routing::TickReport,
    bus: &CompilerBus,
) -> cc_silicon_compiler::task::ResultValue {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            bus.arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .value
                .clone()
        }
        other => panic!("expected executed, got {other:?}"),
    }
}

/// Handles needed downstream.
struct Chain {
    bus: CompilerBus,
    funcdef: NodeId,
}

/// Run the full PP→LX→PA→TY→SE chain (declare included for the symbol).
fn full_chain() -> Chain {
    let mut bus = new_bus();
    install_ir(&mut bus);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let workers = workers();
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_INTERN,
        cc_silicon_compiler::manifest::LX_INTERN_CHIP,
        Payload::from_refs(refs),
    );
    tick(&mut bus, &workers);
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
        Payload::from_refs(refs),
    );
    let report = tick(&mut bus, &workers);
    let tokens: Vec<TokenId> = match report.outcome {
        TickOutcome::Executed { commit, .. } => commit
            .appended
            .iter()
            .map(|(_, reference)| match reference {
                RecordRef::Token(id) => *id,
                other => panic!("expected token ref, got {other:?}"),
            })
            .collect(),
        other => panic!("expected executed, got {other:?}"),
    };
    for index in [7, 9] {
        let pp_token = bus.arenas.tokens.get(tokens[index]).unwrap().pp_token;
        bootstrap(
            &mut bus,
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
            Payload::from_refs(vec![
                RecordRef::Token(tokens[index]),
                RecordRef::PpToken(pp_token),
            ]),
        );
        tick(&mut bus, &workers);
    }
    bootstrap(
        &mut bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(tokens.iter().map(|id| RecordRef::Token(*id)).collect()),
    );
    tick(&mut bus, &workers);
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_INT_TYPE,
        TY_TYPE_CHIP,
        Payload::empty(),
    );
    let report = tick(&mut bus, &workers);
    let int = match completed_value(&report, &bus) {
        cc_silicon_compiler::task::ResultValue::Record(RecordRef::Type(id)) => id,
        other => panic!("expected type ref, got {other:?}"),
    };
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_FUNC_TYPE,
        TY_TYPE_CHIP,
        Payload::from_refs(vec![RecordRef::Type(int)]),
    );
    let report = tick(&mut bus, &workers);
    let func = match completed_value(&report, &bus) {
        cc_silicon_compiler::task::ResultValue::Record(RecordRef::Type(id)) => id,
        other => panic!("expected type ref, got {other:?}"),
    };
    // File scope + declare (the IR needs the committed `main` symbol).
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_SCOPE_ENTER,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(0))]),
    );
    let report = tick(&mut bus, &workers);
    let file_scope = match completed_value(&report, &bus) {
        cc_silicon_compiler::task::ResultValue::Record(RecordRef::Scope(id)) => id,
        other => panic!("expected scope ref, got {other:?}"),
    };
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        Payload::from_refs(vec![
            RecordRef::Node(NodeId::from_index(3)),
            RecordRef::Type(func),
            RecordRef::Scope(file_scope),
        ]),
    );
    tick(&mut bus, &workers);
    // SE checks: literals, binary (+fold), return.
    for node in [NodeId::from_index(7), NodeId::from_index(8)] {
        bootstrap(
            &mut bus,
            TaskKind::SEMANTIC_LITERAL_EXPR,
            SE_LIT_CHIP,
            Payload::from_refs(vec![RecordRef::Node(node)]),
        );
        tick(&mut bus, &workers);
    }
    bootstrap(
        &mut bus,
        TaskKind::SEMANTIC_BINARY_EXPR,
        SE_BIN_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(6))]),
    );
    tick(&mut bus, &workers);
    tick(&mut bus, &workers);
    tick(&mut bus, &workers);
    bootstrap(
        &mut bus,
        TaskKind::SEMANTIC_RETURN_STMT,
        SE_RET_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(5))]),
    );
    tick(&mut bus, &workers);
    let _ = (int, func);
    Chain {
        bus,
        funcdef: NodeId::from_index(1),
    }
}

#[test]
fn ir_kind_stage_and_allowlist_are_frozen() {
    assert_eq!(TaskKind::IR_FUNCTION.group(), TaskGroup::IR_LOWER);
    assert_eq!(TaskKind::IR_FUNCTION.local(), 16);
    assert!(is_ir_slice_kind(TaskKind::IR_FUNCTION));
    assert!(!is_ir_slice_kind(TaskKind::CONTROL_NOOP));
    assert_eq!(stage_of(TaskKind::IR_FUNCTION), Some(5));
    let registry = TaskKindRegistry::ir_slice();
    assert_eq!(registry.len(), 26);
    assert_eq!(
        registry.lookup(TaskKind::IR_FUNCTION).unwrap().name,
        "ir.function"
    );
    for row in [
        (
            IR_FUNCTION_CHIP,
            cc_silicon_compiler::task::StoreId::Ir,
            "functions",
            TaskKind::IR_FUNCTION,
        ),
        (
            IR_FUNCTION_CHIP,
            cc_silicon_compiler::task::StoreId::Ir,
            "blocks",
            TaskKind::IR_FUNCTION,
        ),
        (
            IR_FUNCTION_CHIP,
            cc_silicon_compiler::task::StoreId::Ir,
            "values",
            TaskKind::IR_FUNCTION,
        ),
        (
            IR_FUNCTION_CHIP,
            cc_silicon_compiler::task::StoreId::Ir,
            "instructions",
            TaskKind::IR_FUNCTION,
        ),
    ] {
        assert!(STORE_OWNER_ALLOWLIST.contains(&row));
    }
}

#[test]
fn ir_lowerrs_m1_cl05_without_refolding() {
    let mut chain = full_chain();
    let workers = workers();
    // Exactly one folded constant from the real T07→T08 evaluation.
    assert_eq!(chain.bus.arenas.consts.allocated(), 1);
    let folded = chain
        .bus
        .arenas
        .consts
        .get(cc_silicon_compiler::ids::ConstId::from_index(0))
        .unwrap();
    assert_eq!(folded.value, vec![5]);
    bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    let function = match completed_value(&report, &chain.bus) {
        cc_silicon_compiler::task::ResultValue::Record(RecordRef::Function(id)) => id,
        other => panic!("expected function ref, got {other:?}"),
    };
    assert_eq!(function.index(), 0);
    assert_eq!(chain.bus.arenas.functions.allocated(), 1);
    assert_eq!(chain.bus.arenas.blocks.allocated(), 1);
    assert_eq!(chain.bus.arenas.values.allocated(), 1);
    assert_eq!(chain.bus.arenas.instructions.allocated(), 2);
    // No refold: still exactly one ConstRecord, and the Constant names it.
    assert_eq!(chain.bus.arenas.consts.allocated(), 1);
    let function_body = chain.bus.arenas.functions.get(function).unwrap();
    assert_eq!(function_body.entry.index(), 0);
    assert_eq!(
        function_body.linkage,
        cc_silicon_compiler::bus::Linkage::External
    );
    let block = chain
        .bus
        .arenas
        .blocks
        .get(cc_silicon_compiler::ids::BlockId::from_index(0))
        .unwrap();
    assert_eq!(block.ordinal, 0);
    let constant = chain
        .bus
        .arenas
        .instructions
        .get(cc_silicon_compiler::ids::InstructionId::from_index(0))
        .unwrap();
    assert_eq!(constant.op, IrOp::Constant);
    assert!(constant.operands.is_empty());
    assert_eq!(
        constant.immediate,
        Some(cc_silicon_compiler::ids::ConstId::from_index(0))
    );
    let value = constant.result.unwrap();
    let value_body = chain.bus.arenas.values.get(value).unwrap();
    let int = chain
        .bus
        .arenas
        .types
        .iter()
        .find(|(_, record)| {
            record.kind
                == (cc_silicon_compiler::bus::TypeKind::Int {
                    rank: cc_silicon_compiler::bus::IntRank::Int,
                    signed: true,
                })
        })
        .unwrap()
        .0;
    assert_eq!(value_body.ty, int);
    let ret = chain
        .bus
        .arenas
        .instructions
        .get(cc_silicon_compiler::ids::InstructionId::from_index(1))
        .unwrap();
    assert_eq!(ret.op, IrOp::Return);
    assert_eq!(ret.operands, vec![value]);
    assert_eq!(ret.immediate, None);
    assert_eq!(ret.result, None);
    // The terminator is the greatest instruction: nothing follows it.
    assert!(chain
        .bus
        .arenas
        .instructions
        .iter()
        .all(|(id, _)| id.index() <= 1));
    // Snapshot round-trips the new IR bodies.
    let bytes = encode_function(function_body);
    assert_eq!(decode_function(&bytes).unwrap(), *function_body);
    let bytes = encode_block(block);
    assert_eq!(decode_block(&bytes).unwrap(), *block);
    let bytes = encode_value(value_body);
    assert_eq!(decode_value(&bytes).unwrap(), *value_body);
    let bytes = encode_instruction(constant);
    assert_eq!(decode_instruction(&bytes).unwrap(), *constant);
    let bytes = encode_instruction(ret);
    assert_eq!(decode_instruction(&bytes).unwrap(), *ret);
    let _ = Snapshot::capture(&chain.bus);
}

#[test]
fn ir_rejects_missing_facts_and_ambiguous_constants() {
    // No SemRecords at all: fails on the unchecked return.
    let chain = full_chain();
    let workers = workers();
    // Fresh bus through declare only (no SE facts).
    let mut bus = new_bus();
    install_ir(&mut bus);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_INTERN,
        cc_silicon_compiler::manifest::LX_INTERN_CHIP,
        Payload::from_refs(refs),
    );
    tick(&mut bus, &workers);
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
        Payload::from_refs(refs),
    );
    let report = tick(&mut bus, &workers);
    let tokens: Vec<TokenId> = match report.outcome {
        TickOutcome::Executed { commit, .. } => commit
            .appended
            .iter()
            .map(|(_, reference)| match reference {
                RecordRef::Token(id) => *id,
                other => panic!("expected token ref, got {other:?}"),
            })
            .collect(),
        other => panic!("expected executed, got {other:?}"),
    };
    for index in [7, 9] {
        let pp_token = bus.arenas.tokens.get(tokens[index]).unwrap().pp_token;
        bootstrap(
            &mut bus,
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
            Payload::from_refs(vec![
                RecordRef::Token(tokens[index]),
                RecordRef::PpToken(pp_token),
            ]),
        );
        tick(&mut bus, &workers);
    }
    bootstrap(
        &mut bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(tokens.iter().map(|id| RecordRef::Token(*id)).collect()),
    );
    tick(&mut bus, &workers);
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_INT_TYPE,
        TY_TYPE_CHIP,
        Payload::empty(),
    );
    let report = tick(&mut bus, &workers);
    let int = match completed_value(&report, &bus) {
        cc_silicon_compiler::task::ResultValue::Record(RecordRef::Type(id)) => id,
        other => panic!("expected type ref, got {other:?}"),
    };
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_FUNC_TYPE,
        TY_TYPE_CHIP,
        Payload::from_refs(vec![RecordRef::Type(int)]),
    );
    let report = tick(&mut bus, &workers);
    let func = match completed_value(&report, &bus) {
        cc_silicon_compiler::task::ResultValue::Record(RecordRef::Type(id)) => id,
        other => panic!("expected type ref, got {other:?}"),
    };
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_SCOPE_ENTER,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(0))]),
    );
    let report = tick(&mut bus, &workers);
    let file_scope = match completed_value(&report, &bus) {
        cc_silicon_compiler::task::ResultValue::Record(RecordRef::Scope(id)) => id,
        other => panic!("expected scope ref, got {other:?}"),
    };
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        Payload::from_refs(vec![
            RecordRef::Node(NodeId::from_index(3)),
            RecordRef::Type(func),
            RecordRef::Scope(file_scope),
        ]),
    );
    tick(&mut bus, &workers);
    // Lower with no SemRecords: must fail loudly with nothing appended.
    bootstrap(
        &mut bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(1))]),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(bus.arenas.functions.allocated(), 0);
    let _ = chain;
    let _ = workers;
}

#[test]
fn ir_rejects_ambiguous_fold_output() {
    let mut chain = full_chain();
    let workers = workers();
    // Complete the SE facts first (literals, binary+fold, return).
    for node in [NodeId::from_index(7), NodeId::from_index(8)] {
        bootstrap(
            &mut chain.bus,
            TaskKind::SEMANTIC_LITERAL_EXPR,
            SE_LIT_CHIP,
            Payload::from_refs(vec![RecordRef::Node(node)]),
        );
        tick(&mut chain.bus, &workers);
    }
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_BINARY_EXPR,
        SE_BIN_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(6))]),
    );
    tick(&mut chain.bus, &workers);
    tick(&mut chain.bus, &workers);
    tick(&mut chain.bus, &workers);
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_RETURN_STMT,
        SE_RET_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(5))]),
    );
    tick(&mut chain.bus, &workers);
    // Fold the same inputs once more through a direct task: a second Const.
    let lits: Vec<cc_silicon_compiler::ids::LiteralId> =
        chain.bus.arenas.literals.iter().map(|(id, _)| id).collect();
    bootstrap(
        &mut chain.bus,
        TaskKind::CONSTANT_CONST_FOLD,
        cc_silicon_compiler::manifest::G1_FOLD_CHIP,
        Payload::from_refs(vec![
            RecordRef::Node(NodeId::from_index(6)),
            RecordRef::Literal(lits[0]),
            RecordRef::Literal(lits[1]),
        ]),
    );
    tick(&mut chain.bus, &workers);
    assert_eq!(chain.bus.arenas.consts.allocated(), 2);
    bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.functions.allocated(), 0);
}

#[test]
fn ir_rejects_non_function_and_malformed_inputs() {
    let mut chain = full_chain();
    let workers = workers();
    // TU node instead of a function definition.
    bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(0))]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // Empty payload.
    bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::empty(),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.functions.allocated(), 0);
}

#[test]
fn ir_stage_layer_and_manifest_gates() {
    let mut bus = new_bus();
    install_ir(&mut bus);
    // Re-point the IR route at a wrong layer on a scratch bus.
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::ir_slice();
    mismatched.schema = StoreSchema::ir_slice();
    mismatched
        .registrations
        .register(
            IrFunctionChip.manifest(),
            &mismatched.schema,
            &mismatched.kinds,
        )
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::IR_FUNCTION, IR_FUNCTION_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // A manifest missing a write is rejected at registration.
    let mut manifest = IrFunctionChip.manifest();
    manifest.writes.pop();
    assert!(matches!(
        mismatched.registrations.register(
            manifest,
            &StoreSchema::ir_slice(),
            &TaskKindRegistry::ir_slice()
        ),
        Err(ManifestRegistryError::Registry(_))
    ));
    let _ = FieldPath::new(cc_silicon_compiler::task::StoreId::Ir, "functions");
}

#[test]
fn ir_snapshot_replay_is_deterministic() {
    let run = || {
        let mut chain = full_chain();
        let workers = workers();
        bootstrap(
            &mut chain.bus,
            TaskKind::IR_FUNCTION,
            IR_FUNCTION_CHIP,
            Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
        );
        tick(&mut chain.bus, &workers);
        Snapshot::capture(&chain.bus)
    };
    assert_eq!(run(), run());
}
