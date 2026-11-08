// ============================================================================
// c43_a3.rs — B2-01 `/40` M1 Part A A3 integration gate (A3-I01..I03, I06).
//
// Builds the seeded PP→LX→PA→TY→SE chain on the cumulative
// `se_function_slice()` registry and the `ir_slice()` schema, then pins the
// A3 closure in one place:
//
//   LX → PA → TY(int, int(void), scope, declare)
//      → SE literal ×2 → SE binary (+ real fold) → SE return
//      → SE29 function → VF06 → IR_FUNCTION
//
// The positive case proves the function-definition `SemRecord` is committed
// exactly once by the real SE29 producer, accepted by VF06, and consumed by
// T09 as the `FunctionRecord.signature` source (same `TypeId`), with the
// single folded `ConstId` carried without a refold and a reproducible
// snapshot (A3-I01, A3-I06). The negative cases prove both consumers fail
// closed when the fact is missing, mistyped, duplicated, or mistyped in
// value category/effects (A3-I02, A3-I03). Negative facts are seeds, not a
// second producer; SE29 remains the only real producer.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, EffectMask, IrOp, Linkage, PpTokenKind, PpTokenRecord, SemRecord,
    SpanRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{
    handler_for, project_ir_function_input, IrFunctionChip, LxClassifyChip, LxDecodeLiteralChip,
    LxInternChip, PaTuChip, SeBinChip, SeFuncChip, SeLitChip, SeRetChip, TyConvChip, TyScopeChip,
    TySymbolChip, TyTypeChip, Vf06Chip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticRecord};
use cc_silicon_compiler::ids::{
    BlockId, ConstId, FunctionId, InstructionId, NodeId, RecordRef, SemId, TaskId, TokenId, TypeId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    StoreSchema, G1_FOLD_CHIP, IR_FUNCTION_CHIP, LX_CLASSIFY_CHIP, LX_DECODE_CHIP, LX_INTERN_CHIP,
    PA_TU_CHIP, SE_BIN_CHIP, SE_FUNC_CHIP, SE_LIT_CHIP, SE_RET_CHIP, TY_CONV_CHIP, TY_SCOPE_CHIP,
    TY_SYMBOL_CHIP, TY_TYPE_CHIP, VF06_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome, TickReport};
use cc_silicon_compiler::snapshot::Snapshot;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, ResultValue, TaskDraft, TaskKind, TaskKindRegistry};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

/// Install the cumulative A3 slice: `se_function_slice()` names every kind,
/// `ir_slice()` declares `Sem/records` and the four `Ir` append fields, and
/// every kind used by the chain is routed at its frozen stage layer.
fn install_a3(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::se_function_slice();
    bus.schema = StoreSchema::ir_slice();
    // The fold worker is registered (never driven directly): SE binary
    // enqueues `const_fold` children whose destination must be owned.
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
    // LX/PA/fold at layer 2, TY at 3, SE/VF06 at 4, IR at 5.
    for (kind, chip, layer) in [
        (TaskKind::LEX_INTERN, LX_INTERN_CHIP, 2),
        (TaskKind::LEX_CLASSIFY, LX_CLASSIFY_CHIP, 2),
        (TaskKind::LEX_DECODE_LITERAL, LX_DECODE_CHIP, 2),
        (TaskKind::PARSE_TU, PA_TU_CHIP, 2),
        (TaskKind::SYMBOL_INT_TYPE, TY_TYPE_CHIP, 3),
        (TaskKind::SYMBOL_FUNC_TYPE, TY_TYPE_CHIP, 3),
        (TaskKind::SYMBOL_SCOPE_ENTER, TY_SCOPE_CHIP, 3),
        (TaskKind::SYMBOL_SCOPE_EXIT, TY_SCOPE_CHIP, 3),
        (TaskKind::SYMBOL_DECLARE, TY_SYMBOL_CHIP, 3),
        (TaskKind::SYMBOL_LOOKUP, TY_SYMBOL_CHIP, 3),
        (TaskKind::SYMBOL_PROMOTE, TY_CONV_CHIP, 3),
        (TaskKind::SYMBOL_COMMON_TYPE, TY_CONV_CHIP, 3),
        (TaskKind::SYMBOL_RETURN_CONVERT, TY_CONV_CHIP, 3),
        (TaskKind::SEMANTIC_LITERAL_EXPR, SE_LIT_CHIP, 4),
        (TaskKind::SEMANTIC_BINARY_EXPR, SE_BIN_CHIP, 4),
        (TaskKind::SEMANTIC_RETURN_STMT, SE_RET_CHIP, 4),
        (TaskKind::SEMANTIC_FUNCTION_DEF, SE_FUNC_CHIP, 4),
        (TaskKind::VERIFICATION_TYPED_INVARIANT, VF06_CHIP, 4),
        (TaskKind::IR_FUNCTION, IR_FUNCTION_CHIP, 5),
        (TaskKind::CONSTANT_CONST_FOLD, G1_FOLD_CHIP, 2),
    ] {
        bus.routing.register(kind, chip, layer).unwrap();
    }
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

fn completed_value(report: &TickReport, bus: &CompilerBus) -> ResultValue {
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

fn completed_ref(report: &TickReport, bus: &CompilerBus) -> RecordRef {
    match completed_value(report, bus) {
        ResultValue::Record(reference) => reference,
        other => panic!("expected record ref, got {other:?}"),
    }
}

/// Require exactly one failed task and no completion; return its diagnostic.
fn failed_diagnostic(report: &TickReport, bus: &CompilerBus) -> DiagnosticRecord {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 0);
            assert_eq!(commit.failed.len(), 1);
            bus.arenas
                .diagnostics
                .get(commit.failed[0].1)
                .unwrap()
                .clone()
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

/// All committed `SemRecord`s in ascending `SemId` order.
///
/// The arena itself iterates by ID, but the test sorts explicitly so every
/// scan is order-defined without relying on storage details.
fn sem_facts(bus: &CompilerBus) -> Vec<(SemId, SemRecord)> {
    let mut facts: Vec<(SemId, SemRecord)> = bus
        .arenas
        .sem
        .iter()
        .map(|(id, record)| (id, record.clone()))
        .collect();
    facts.sort_by_key(|(id, _)| id.index());
    facts
}

/// Committed facts for one node, in ascending `SemId` order.
fn sem_facts_for(bus: &CompilerBus, node: NodeId) -> Vec<(SemId, SemRecord)> {
    sem_facts(bus)
        .into_iter()
        .filter(|(_, record)| record.node == node)
        .collect()
}

/// Unit-isolation fixture: seed one `SemRecord` directly.
///
/// The real producer of every M1 fact is the SE chain, and the real producer
/// of the function-definition fact is SE29 (exercised end-to-end by
/// `a3_i01_...` and by `c39_sefunc.rs`). The negative A3 cases below need
/// states that no conforming producer can emit, so they allocate the records
/// directly instead of introducing a second producer path.
fn seed_sem_fact(
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

/// Handles needed downstream.
struct Chain {
    bus: CompilerBus,
    int: TypeId,
    func: TypeId,
    tu: NodeId,
    funcdef: NodeId,
    declarator: NodeId,
    ret: NodeId,
    binary: NodeId,
}

/// Run LX→PA→TY(int, int(void))→scope→declare.
///
/// Pre-order node allocation from `PARSE_TU`: TU(0) funcdef(1) spec(2)
/// decl(3) compound(4) return(5) binary(6) lit2(7) lit3(8).
fn chain_through_declare() -> Chain {
    let mut bus = new_bus();
    install_a3(&mut bus);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let workers = workers();
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_INTERN,
        LX_INTERN_CHIP,
        Payload::from_refs(refs),
    );
    tick(&mut bus, &workers);
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        LX_CLASSIFY_CHIP,
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
            LX_DECODE_CHIP,
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
    let int = match completed_ref(&report, &bus) {
        RecordRef::Type(id) => id,
        other => panic!("expected int type ref, got {other:?}"),
    };
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_FUNC_TYPE,
        TY_TYPE_CHIP,
        Payload::from_refs(vec![RecordRef::Type(int)]),
    );
    let report = tick(&mut bus, &workers);
    let func = match completed_ref(&report, &bus) {
        RecordRef::Type(id) => id,
        other => panic!("expected function type ref, got {other:?}"),
    };
    let tu = NodeId::from_index(0);
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_SCOPE_ENTER,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu)]),
    );
    let report = tick(&mut bus, &workers);
    let file_scope = match completed_ref(&report, &bus) {
        RecordRef::Scope(id) => id,
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
    Chain {
        bus,
        int,
        func,
        tu,
        funcdef: NodeId::from_index(1),
        declarator: NodeId::from_index(3),
        ret: NodeId::from_index(5),
        binary: NodeId::from_index(6),
    }
}

/// Check both literals, fold the binary operand (3 ticks: fold child,
/// resume, drain), and check the return statement.
fn check_se_through_return(chain: &mut Chain, workers: &WorkerRegistry) {
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

/// LX→PA→TY→declare→SE literal/binary/return: four facts, no SE29 yet.
fn chain_through_return() -> Chain {
    let mut chain = chain_through_declare();
    let workers = workers();
    check_se_through_return(&mut chain, &workers);
    chain
}

/// Run the real SE29 producer and return the committed function fact ID.
fn run_se_function(chain: &mut Chain, workers: &WorkerRegistry) -> SemId {
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let report = tick(&mut chain.bus, workers);
    match completed_ref(&report, &chain.bus) {
        RecordRef::Sem(id) => id,
        other => panic!("expected sem ref, got {other:?}"),
    }
}

fn run_vf06(chain: &mut Chain, workers: &WorkerRegistry) -> TickReport {
    bootstrap(
        &mut chain.bus,
        TaskKind::VERIFICATION_TYPED_INVARIANT,
        VF06_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.tu)]),
    );
    tick(&mut chain.bus, workers)
}

fn expect_vf06_ack(chain: &mut Chain, workers: &WorkerRegistry) {
    let report = run_vf06(chain, workers);
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 0);
            assert_eq!(commit.completed.len(), 1);
            match &chain
                .bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .value
            {
                ResultValue::Ack => {}
                other => panic!("expected Ack, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    }
}

fn dispatch_ir(chain: &mut Chain) -> TaskId {
    bootstrap(
        &mut chain.bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    )
}

fn run_ir(chain: &mut Chain, workers: &WorkerRegistry) -> TickReport {
    dispatch_ir(chain);
    tick(&mut chain.bus, workers)
}

// --- A3-I01 / A3-I06: full chain --------------------------------------------

#[test]
fn a3_i01_full_chain_commits_and_consumes_the_function_fact() {
    let mut chain = chain_through_return();
    let workers = workers();

    // Before SE29: exactly the four expression/statement facts
    // (literals 7/8, binary 6, return 5) and nothing for the function node.
    let before = sem_facts(&chain.bus);
    assert_eq!(before.len(), 4);
    // Committed order is literal 7, literal 8, binary 6, return 5.
    let before_nodes: Vec<u32> = before.iter().map(|(_, sem)| sem.node.index()).collect();
    assert_eq!(before_nodes, vec![7, 8, 6, 5]);
    assert!(before.iter().all(|(_, sem)| sem.node != chain.funcdef));

    // After SE29: exactly one signature-carrying function fact.
    let function_sem = run_se_function(&mut chain, &workers);
    assert_eq!(chain.bus.arenas.sem.allocated(), 5);
    let function_facts = sem_facts_for(&chain.bus, chain.funcdef);
    assert_eq!(function_facts.len(), 1);
    assert_eq!(function_facts[0].0, function_sem);
    let function_fact = &function_facts[0].1;
    assert_eq!(function_fact.ty, chain.func);
    assert_eq!(function_fact.category, ValueCategory::NonLvalue);
    assert_eq!(function_fact.effects, EffectMask(0));

    // VF06 accepts the four-kind checked set.
    expect_vf06_ack(&mut chain, &workers);

    // T09 consumes the same committed fact as its signature source.
    let task = dispatch_ir(&mut chain);
    let input = project_ir_function_input(&chain.bus, task).unwrap();
    assert_eq!(input.function_sem, function_sem);
    assert_eq!(input.signature, chain.func);
    let report = tick(&mut chain.bus, &workers);
    let function = match completed_ref(&report, &chain.bus) {
        RecordRef::Function(id) => id,
        other => panic!("expected function ref, got {other:?}"),
    };
    assert_eq!(function, FunctionId::from_index(0));

    // Frozen M1 IR shape: one function, one block, one value, two
    // instructions (`Constant` + `Return`).
    assert_eq!(chain.bus.arenas.functions.allocated(), 1);
    assert_eq!(chain.bus.arenas.blocks.allocated(), 1);
    assert_eq!(chain.bus.arenas.values.allocated(), 1);
    assert_eq!(chain.bus.arenas.instructions.allocated(), 2);
    let function_body = chain.bus.arenas.functions.get(function).unwrap();
    assert_eq!(function_body.signature, chain.func);
    assert_eq!(function_body.signature, function_fact.ty);
    assert_eq!(function_body.linkage, Linkage::External);
    assert_eq!(function_body.entry, BlockId::from_index(0));

    // The structural symbol link names the declared `main`.
    let (symbol_id, symbol_body) = chain
        .bus
        .arenas
        .symbols
        .iter()
        .find(|(_, sym)| sym.decl == chain.declarator)
        .map(|(id, record)| (id, record.clone()))
        .expect("the declared main symbol is present");
    assert_eq!(function_body.symbol, symbol_id);
    assert_eq!(symbol_body.ty, Some(chain.func));
    let main_name = chain.bus.intern.lookup(b"main").unwrap();
    assert_eq!(symbol_body.name, main_name);

    let block = chain.bus.arenas.blocks.get(function_body.entry).unwrap();
    assert_eq!(block.function, function);
    assert_eq!(block.ordinal, 0);

    // Exactly one folded `ConstRecord`, carried verbatim (no refold).
    assert_eq!(chain.bus.arenas.consts.allocated(), 1);
    let folded = ConstId::from_index(0);
    let folded_body = chain.bus.arenas.consts.get(folded).unwrap();
    assert_eq!(folded_body.value, vec![5]);
    let constant = chain
        .bus
        .arenas
        .instructions
        .get(InstructionId::from_index(0))
        .unwrap();
    assert_eq!(constant.op, IrOp::Constant);
    assert!(constant.operands.is_empty());
    assert_eq!(constant.immediate, Some(folded));
    let value = constant.result.expect("constant produces one value");
    let ret = chain
        .bus
        .arenas
        .instructions
        .get(InstructionId::from_index(1))
        .unwrap();
    assert_eq!(ret.op, IrOp::Return);
    assert_eq!(ret.operands, vec![value]);
    assert_eq!(ret.immediate, None);
    assert_eq!(ret.result, None);
    assert_eq!(chain.bus.arenas.values.get(value).unwrap().ty, chain.int);

    // A3-I06: capturing the same chain twice is byte- and hash-identical.
    let first = Snapshot::capture(&chain.bus);
    let second = Snapshot::capture(&chain.bus);
    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(first.hash(), second.hash());
    assert_eq!(first, second);
}

// --- A3-I02: missing SE29 ---------------------------------------------------

#[test]
fn a3_i02_missing_function_fact_fails_closed() {
    let mut chain = chain_through_return();
    let workers = workers();
    assert_eq!(chain.bus.arenas.sem.allocated(), 4);
    assert!(sem_facts_for(&chain.bus, chain.funcdef).is_empty());

    // VF06: missing typed fact is a Task-group failure with no `Ack`.
    let report = run_vf06(&mut chain, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Task);
    assert_eq!(diagnostic.code.code, 4);
    assert!(
        diagnostic.message.contains("missing typed fact"),
        "unexpected message: {}",
        diagnostic.message
    );
    assert_eq!(chain.bus.arenas.sem.allocated(), 4);

    // T09: refuses the unchecked definition and fabricates no function.
    let report = run_ir(&mut chain, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Task);
    assert_eq!(diagnostic.code.code, 4);
    assert_eq!(
        diagnostic.message,
        "lower reads unchecked function definition"
    );
    assert_eq!(chain.bus.arenas.functions.allocated(), 0);
}

// --- A3-I03: wrong function facts ------------------------------------------

#[test]
fn a3_i03_wrong_function_fact_type_fails_closed() {
    let mut chain = chain_through_return();
    let workers = workers();
    let (funcdef, int) = (chain.funcdef, chain.int);
    seed_sem_fact(
        &mut chain,
        funcdef,
        int,
        ValueCategory::NonLvalue,
        EffectMask(0),
    );

    let report = run_vf06(&mut chain, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Task);
    assert_eq!(diagnostic.code.code, 4);
    assert!(
        diagnostic.message.contains("non-signature type"),
        "unexpected message: {}",
        diagnostic.message
    );

    let report = run_ir(&mut chain, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
    assert!(
        diagnostic.message.contains("non-M1 function signature"),
        "unexpected message: {}",
        diagnostic.message
    );
    assert_eq!(chain.bus.arenas.functions.allocated(), 0);
}

#[test]
fn a3_i03_duplicate_function_facts_fail_closed() {
    let mut chain = chain_through_return();
    let workers = workers();
    let (funcdef, func) = (chain.funcdef, chain.func);
    seed_sem_fact(
        &mut chain,
        funcdef,
        func,
        ValueCategory::NonLvalue,
        EffectMask(0),
    );
    seed_sem_fact(
        &mut chain,
        funcdef,
        func,
        ValueCategory::NonLvalue,
        EffectMask(0),
    );

    let report = run_vf06(&mut chain, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Task);
    assert_eq!(diagnostic.code.code, 4);
    assert!(
        diagnostic.message.contains("duplicate typed fact"),
        "unexpected message: {}",
        diagnostic.message
    );

    let report = run_ir(&mut chain, &workers);
    let diagnostic = failed_diagnostic(&report, &chain.bus);
    assert_eq!(diagnostic.code.group, DiagGroup::Task);
    assert_eq!(diagnostic.code.code, 4);
    assert!(
        diagnostic
            .message
            .contains("duplicate function-definition fact"),
        "unexpected message: {}",
        diagnostic.message
    );
    assert_eq!(chain.bus.arenas.functions.allocated(), 0);
}

#[test]
fn a3_i03_wrong_category_and_effects_fail_closed() {
    // Wrong value category on an otherwise signature-correct fact.
    {
        let mut chain = chain_through_return();
        let workers = workers();
        let (funcdef, func) = (chain.funcdef, chain.func);
        seed_sem_fact(
            &mut chain,
            funcdef,
            func,
            ValueCategory::Lvalue,
            EffectMask(0),
        );

        let report = run_vf06(&mut chain, &workers);
        let diagnostic = failed_diagnostic(&report, &chain.bus);
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
        assert!(
            diagnostic.message.contains("bad value category"),
            "unexpected message: {}",
            diagnostic.message
        );

        let report = run_ir(&mut chain, &workers);
        let diagnostic = failed_diagnostic(&report, &chain.bus);
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
        assert!(
            diagnostic
                .message
                .contains("mistyped function-definition fact"),
            "unexpected message: {}",
            diagnostic.message
        );
        assert_eq!(chain.bus.arenas.functions.allocated(), 0);
    }

    // Nonzero effects on an otherwise signature-correct fact.
    {
        let mut chain = chain_through_return();
        let workers = workers();
        let (funcdef, func) = (chain.funcdef, chain.func);
        seed_sem_fact(
            &mut chain,
            funcdef,
            func,
            ValueCategory::NonLvalue,
            EffectMask(1),
        );

        let report = run_vf06(&mut chain, &workers);
        let diagnostic = failed_diagnostic(&report, &chain.bus);
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
        assert!(
            diagnostic.message.contains("nonzero effect mask"),
            "unexpected message: {}",
            diagnostic.message
        );

        let report = run_ir(&mut chain, &workers);
        let diagnostic = failed_diagnostic(&report, &chain.bus);
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
        assert!(
            diagnostic
                .message
                .contains("mistyped function-definition fact"),
            "unexpected message: {}",
            diagnostic.message
        );
        assert_eq!(chain.bus.arenas.functions.allocated(), 0);
    }
}
