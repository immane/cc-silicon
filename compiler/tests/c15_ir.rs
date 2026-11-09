// ============================================================================
// c15_ir.rs — Wave 2 (`/15`) IR function-lowering slice acceptance, plus the
// `/40` T09 A3 closure cases (A3-U07..A3-U12).
//
// Covers the frozen closure: the exact M1 IR shape (one function, one
// block, one value, `Constant` + `Return`), the `ir.function` kind,
// kind-to-stage assignment, the IR store-owner allowlist rows, stage/layer
// agreement, the same-`ConstId` handoff without refolding, snapshot bodies,
// tick-lifecycle integration, and the `/15` hash participation. Chain
// inputs come from the real SE slice plus SE29; `M1-CL-05` completes here
// (T09 consumes the fold's `ConstRecord`). The `/40` cases pin the
// committed `FunctionDefinition` fact as the signature source (never
// `symbol.ty`), the interim `ir.constant-result-type-from-sem` value type,
// and the new failure precedence. FunctionEnd/IR28 stays deferred.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, EffectMask, IrOp, Linkage, PpTokenKind, PpTokenRecord, SemRecord,
    SpanRecord, TypeKind, TypeRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{
    handler_for, project_ir_function_input, IrFunctionChip, LxClassifyChip, LxDecodeLiteralChip,
    LxInternChip, PaTuChip, SeBinChip, SeFuncChip, SeLitChip, SeRetChip, TyConvChip, TyScopeChip,
    TySymbolChip, TyTypeChip, Vf06Chip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticRecord};
use cc_silicon_compiler::ids::{
    ConstId, NodeId, RecordRef, SemId, SymbolId, TaskId, TokenId, TypeId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_ir_slice_kind, stage_of, FieldPath, ManifestRegistryError, StoreSchema, IR_FUNCTION_CHIP,
    PA_TU_CHIP, SE_BIN_CHIP, SE_FUNC_CHIP, SE_LIT_CHIP, SE_RET_CHIP, STORE_OWNER_ALLOWLIST,
    TY_CONV_CHIP, TY_SCOPE_CHIP, TY_SYMBOL_CHIP, TY_TYPE_CHIP, VF06_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome, TickReport};
use cc_silicon_compiler::snapshot::{
    decode_block, decode_function, decode_instruction, decode_value, encode_block, encode_function,
    encode_instruction, encode_value, Snapshot,
};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry, TaskState,
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

fn install_ir(bus: &mut CompilerBus) {
    // `/40`: the cumulative SE-function registry supplies
    // `semantic.function_def`; the IR schema already declares `Sem/records`.
    bus.kinds = TaskKindRegistry::se_function_slice();
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
        &SeFuncChip,
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
        .register(TaskKind::SEMANTIC_FUNCTION_DEF, SE_FUNC_CHIP, 4)
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
    workers.register(SeFuncChip).unwrap();
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

fn tick(bus: &mut CompilerBus, workers: &WorkerRegistry) -> TickReport {
    RoutingShell
        .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
        .unwrap()
}

fn completed_value(
    report: &TickReport,
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

fn failed_diagnostic<'a>(report: &TickReport, bus: &'a CompilerBus) -> &'a DiagnosticRecord {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 1);
            bus.arenas.diagnostics.get(commit.failed[0].1).unwrap()
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

/// Handles needed downstream.
struct Chain {
    bus: CompilerBus,
    int: TypeId,
    func: TypeId,
    funcdef: NodeId,
    declarator: NodeId,
    ret: NodeId,
    binary: NodeId,
}

fn alloc_sem_with(
    chain: &mut Chain,
    node: NodeId,
    ty: TypeId,
    category: ValueCategory,
    effects: EffectMask,
) -> SemId {
    let limits = chain.bus.limits();
    chain
        .bus
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

fn alloc_sem(chain: &mut Chain, node: NodeId, ty: TypeId) -> SemId {
    alloc_sem_with(chain, node, ty, ValueCategory::NonLvalue, EffectMask(0))
}

fn alloc_type(chain: &mut Chain, kind: TypeKind) -> TypeId {
    let limits = chain.bus.limits();
    chain
        .bus
        .arenas
        .types
        .alloc(TypeRecord { kind }, &limits)
        .unwrap()
}

/// The single committed fact for `node` (the fixtures keep exactly one).
fn sem_for(chain: &Chain, node: NodeId) -> (SemId, SemRecord) {
    let mut matches = chain
        .bus
        .arenas
        .sem
        .iter()
        .filter(|(_, sem)| sem.node == node);
    let (id, sem) = matches.next().expect("expected a committed sem fact");
    assert!(matches.next().is_none(), "expected exactly one sem fact");
    (id, sem.clone())
}

/// Run LX→PA→TY(int, func)→scope→declare. When
/// `divergent_symbol_type` is set, the structural symbol is declared with a
/// second, distinct-ID `int(void)` type so the fact signature differs (the
/// mismatch case); T09 must not adopt the symbol's type.
///
/// Pre-order node allocation from `PARSE_TU`: TU(0) funcdef(1) spec(2)
/// decl(3) compound(4) return(5) binary(6) lit2(7) lit3(8).
fn chain_through_declare_with_divergent_symbol_type(divergent_symbol_type: bool) -> Chain {
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
    let tu = NodeId::from_index(0);
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_SCOPE_ENTER,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu)]),
    );
    let report = tick(&mut bus, &workers);
    let file_scope = match completed_value(&report, &bus) {
        cc_silicon_compiler::task::ResultValue::Record(RecordRef::Scope(id)) => id,
        other => panic!("expected scope ref, got {other:?}"),
    };
    let declared = if divergent_symbol_type {
        let limits = bus.limits();
        bus.arenas
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
            .unwrap()
    } else {
        func
    };
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        Payload::from_refs(vec![
            RecordRef::Node(NodeId::from_index(3)),
            RecordRef::Type(declared),
            RecordRef::Scope(file_scope),
        ]),
    );
    tick(&mut bus, &workers);
    Chain {
        bus,
        int,
        func,
        funcdef: NodeId::from_index(1),
        declarator: NodeId::from_index(3),
        ret: NodeId::from_index(5),
        binary: NodeId::from_index(6),
    }
}

fn chain_through_declare() -> Chain {
    chain_through_declare_with_divergent_symbol_type(false)
}

/// Check both literals, fold the binary operand (3 ticks), and check the
/// return statement, leaving the `Return` child fact committed.
fn check_se_return(chain: &mut Chain, workers: &WorkerRegistry) {
    for node in [NodeId::from_index(7), NodeId::from_index(8)] {
        bootstrap(
            &mut chain.bus,
            TaskKind::SEMANTIC_LITERAL_EXPR,
            SE_LIT_CHIP,
            Payload::from_refs(vec![RecordRef::Node(node)]),
        );
        tick(&mut chain.bus, workers);
    }
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_BINARY_EXPR,
        SE_BIN_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.binary)]),
    );
    tick(&mut chain.bus, workers);
    tick(&mut chain.bus, workers);
    tick(&mut chain.bus, workers);
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_RETURN_STMT,
        SE_RET_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.ret)]),
    );
    tick(&mut chain.bus, workers);
}

/// LX→PA→TY→declare→SE literal/binary/return; no function-definition fact.
fn chain_through_return() -> Chain {
    let mut chain = chain_through_declare();
    let workers = workers();
    check_se_return(&mut chain, &workers);
    chain
}

/// `/40`: SE29 runs after the SE return check and before IR lowering.
fn run_se_function(chain: &mut Chain, workers: &WorkerRegistry) {
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let report = tick(&mut chain.bus, workers);
    match completed_value(&report, &chain.bus) {
        cc_silicon_compiler::task::ResultValue::Record(RecordRef::Sem(_)) => {}
        other => panic!("expected sem ref, got {other:?}"),
    }
}

/// Full PP→LX→PA→TY→SE(return)→SE29 chain.
fn full_chain() -> Chain {
    let mut chain = chain_through_return();
    let workers = workers();
    run_se_function(&mut chain, &workers);
    chain
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
    // A3-U07: the committed function fact supplies the signature.
    let mut chain = full_chain();
    let workers = workers();
    assert_eq!(chain.bus.arenas.sem.allocated(), 5);
    let (function_sem, function_fact) = sem_for(&chain, chain.funcdef);
    assert_eq!(function_fact.ty, chain.func);
    assert_eq!(function_fact.category, ValueCategory::NonLvalue);
    assert_eq!(function_fact.effects, EffectMask(0));
    let (_, binary_fact) = sem_for(&chain, chain.binary);
    let (_, return_fact) = sem_for(&chain, chain.ret);
    assert_eq!(binary_fact.ty, chain.int);
    assert_eq!(return_fact.ty, binary_fact.ty);
    // Exactly one folded constant from the real T07→T08 evaluation.
    assert_eq!(chain.bus.arenas.consts.allocated(), 1);
    let folded = chain
        .bus
        .arenas
        .consts
        .get(cc_silicon_compiler::ids::ConstId::from_index(0))
        .unwrap();
    let folded_integer = folded.value.as_integer().unwrap();
    assert_eq!(folded_integer.magnitude, vec![5]);
    assert!(!folded_integer.negative);
    let task = bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let input = project_ir_function_input(&chain.bus, task).unwrap();
    assert_eq!(input.function_sem, function_sem);
    assert_eq!(input.signature, function_fact.ty);
    assert_eq!(input.constant_ty, binary_fact.ty);
    assert_eq!(input.return_ty, return_fact.ty);
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
    assert_eq!(function_body.signature, function_fact.ty);
    assert_eq!(function_body.signature, chain.func);
    assert_eq!(function_body.linkage, Linkage::External);
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
    assert_eq!(int, chain.int);
    // The value type follows the committed semantic fact, not a host type.
    assert_eq!(value_body.ty, binary_fact.ty);
    assert_eq!(value_body.ty, return_fact.ty);
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
fn ir_a3_u08_missing_function_fact_fails_without_symbol_fallback() {
    // A3-U08: symbol and operand facts are present; only the committed
    // function-definition fact is missing.
    let mut chain = chain_through_return();
    let workers = workers();
    let symbol = chain
        .bus
        .arenas
        .symbols
        .iter()
        .find(|(_, sym)| sym.decl == chain.declarator)
        .expect("the declared main symbol is present")
        .1;
    assert_eq!(symbol.ty, Some(chain.func));
    assert!(chain
        .bus
        .arenas
        .sem
        .iter()
        .all(|(_, sem)| sem.node != chain.funcdef));
    bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Task);
    assert_eq!(diagnostic.code.code, 4);
    // No `symbol.ty` fallback: the fact is required, not reconstructed.
    assert_eq!(
        diagnostic.message,
        "lower reads unchecked function definition"
    );
    assert_eq!(chain.bus.arenas.functions.allocated(), 0);
}

#[test]
fn ir_a3_u09_non_m1_function_fact_types_are_unsupported() {
    // Fact typed as canonical `int` instead of the committed signature.
    {
        let mut chain = chain_through_return();
        let workers = workers();
        let funcdef = chain.funcdef;
        let int = chain.int;
        alloc_sem(&mut chain, funcdef, int);
        bootstrap(
            &mut chain.bus,
            TaskKind::IR_FUNCTION,
            IR_FUNCTION_CHIP,
            Payload::from_refs(vec![RecordRef::Node(funcdef)]),
        );
        let report = tick(&mut chain.bus, &workers);
        let diagnostic = failed_diagnostic(&report, &chain.bus);
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
        assert_eq!(chain.bus.arenas.functions.allocated(), 0);
    }
    // Fact typed as a non-M1 function shape (one parameter).
    {
        let mut chain = chain_through_return();
        let workers = workers();
        let int = chain.int;
        let non_m1 = alloc_type(
            &mut chain,
            TypeKind::Function {
                result: int,
                params: vec![int],
                prototype: true,
                variadic: false,
            },
        );
        let funcdef = chain.funcdef;
        alloc_sem(&mut chain, funcdef, non_m1);
        bootstrap(
            &mut chain.bus,
            TaskKind::IR_FUNCTION,
            IR_FUNCTION_CHIP,
            Payload::from_refs(vec![RecordRef::Node(funcdef)]),
        );
        let report = tick(&mut chain.bus, &workers);
        let diagnostic = failed_diagnostic(&report, &chain.bus);
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
        assert_eq!(chain.bus.arenas.functions.allocated(), 0);
    }
}

#[test]
fn ir_function_fact_category_and_effects_are_rechecked() {
    // O-6: T09 re-checks the fact even when VF06 is not on the chain.
    {
        let mut chain = chain_through_return();
        let workers = workers();
        let funcdef = chain.funcdef;
        let func = chain.func;
        alloc_sem_with(
            &mut chain,
            funcdef,
            func,
            ValueCategory::Lvalue,
            EffectMask(0),
        );
        bootstrap(
            &mut chain.bus,
            TaskKind::IR_FUNCTION,
            IR_FUNCTION_CHIP,
            Payload::from_refs(vec![RecordRef::Node(funcdef)]),
        );
        let report = tick(&mut chain.bus, &workers);
        let diagnostic = failed_diagnostic(&report, &chain.bus);
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
        assert!(
            diagnostic
                .message
                .contains("mistyped function-definition fact"),
            "{}",
            diagnostic.message
        );
        assert_eq!(chain.bus.arenas.functions.allocated(), 0);
    }
    {
        let mut chain = chain_through_return();
        let workers = workers();
        let funcdef = chain.funcdef;
        let func = chain.func;
        alloc_sem_with(
            &mut chain,
            funcdef,
            func,
            ValueCategory::NonLvalue,
            EffectMask(1),
        );
        bootstrap(
            &mut chain.bus,
            TaskKind::IR_FUNCTION,
            IR_FUNCTION_CHIP,
            Payload::from_refs(vec![RecordRef::Node(funcdef)]),
        );
        let report = tick(&mut chain.bus, &workers);
        let diagnostic = failed_diagnostic(&report, &chain.bus);
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
        assert!(
            diagnostic
                .message
                .contains("mistyped function-definition fact"),
            "{}",
            diagnostic.message
        );
        assert_eq!(chain.bus.arenas.functions.allocated(), 0);
    }
}

#[test]
fn ir_a3_u10_symbol_type_mismatch_is_unsupported() {
    // A3-U10: the valid fact signature is `int(void)`; the structural
    // symbol carries a different (distinct-ID) `int(void)` type and must
    // not be used as the source.
    let mut chain = chain_through_declare_with_divergent_symbol_type(true);
    let workers = workers();
    let symbol = chain
        .bus
        .arenas
        .symbols
        .iter()
        .find(|(_, sym)| sym.decl == chain.declarator)
        .expect("the declared main symbol is present")
        .1;
    assert!(symbol.ty.is_some());
    assert_ne!(symbol.ty, Some(chain.func));
    let funcdef = chain.funcdef;
    let signature = chain.func;
    alloc_sem(&mut chain, funcdef, signature);
    bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
    assert_eq!(
        diagnostic.message,
        "lower of a definition inconsistent with its prototype"
    );
    assert_eq!(chain.bus.arenas.functions.allocated(), 0);
}

#[test]
fn ir_a3_u11_duplicate_function_facts_are_a_defect() {
    let mut chain = chain_through_return();
    let workers = workers();
    let funcdef = chain.funcdef;
    let signature = chain.func;
    alloc_sem(&mut chain, funcdef, signature);
    alloc_sem(&mut chain, funcdef, signature);
    bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Task);
    assert_eq!(diagnostic.code.code, 4);
    assert!(
        diagnostic
            .message
            .contains("duplicate function-definition fact"),
        "{}",
        diagnostic.message
    );
    assert_eq!(chain.bus.arenas.functions.allocated(), 0);
}

#[test]
fn ir_a3_u12_fresh_bus_fails_on_missing_function_fact_first() {
    // A3-U12: no SemRecords at all. The missing function-definition fact
    // wins deterministically over the missing `Return`/`BinaryAdd` facts.
    let mut chain = chain_through_declare();
    let workers = workers();
    assert_eq!(chain.bus.arenas.sem.allocated(), 0);
    bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Task);
    assert_eq!(diagnostic.code.code, 4);
    assert_eq!(
        diagnostic.message,
        "lower reads unchecked function definition"
    );
    assert!(!diagnostic.message.contains("unchecked node"));
    assert_eq!(chain.bus.arenas.functions.allocated(), 0);
}

#[test]
fn ir_constant_result_type_must_match_return_fact() {
    // Interim `ir.constant-result-type-from-sem`: a projection whose
    // constant-producing fact and `Return` fact disagree fails, rather than
    // emitting a value typed by either fact alone. The well-formed M1 bus
    // cannot reach this state (both facts are pinned to `int`), so the pure
    // computation is exercised directly.
    let input = cc_silicon_compiler::chips::IrFunctionInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        symbol: SymbolId::from_index(0),
        linkage: Linkage::External,
        signature: TypeId::from_index(3),
        function_sem: SemId::from_index(0),
        int: TypeId::from_index(0),
        constant_ty: TypeId::from_index(1),
        return_ty: TypeId::from_index(2),
        constant_record_ty: TypeId::from_index(1),
        constant: ConstId::from_index(0),
        local: None,
        local_expr: None,
        functions_allocated: 0,
        blocks_allocated: 0,
        values_allocated: 0,
        instructions_allocated: 0,
    };
    match IrFunctionChip.compute(&input).as_slice() {
        [Proposal::Fail { diagnostic, .. }] => {
            assert_eq!(diagnostic.code, DiagnosticCode::new(DiagGroup::Task, 4));
            assert!(
                diagnostic.message.contains("constant type"),
                "{}",
                diagnostic.message
            );
        }
        other => panic!("expected a single failure, got {other:?}"),
    }
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
