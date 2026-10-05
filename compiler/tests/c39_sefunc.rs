// ============================================================================
// c39_sefunc.rs — Wave 3 (`/39`) SE29 function-definition slice acceptance.
//
// Covers the frozen closure: the `semantic.function_def` kind (local 21),
// kind-to-stage assignment (stage 4), the SE29 manifest (one `sem.records`
// allowlist row), the normal signature check (M1 `(void)`-only declarator,
// single committed TY17 `int(void)` signature, declared `main` symbol,
// committed `Return` child fact → one signature-carrying `SemRecord`),
// committed-record reuse, loud failures (unchecked return body, K&R
// parameter list, prototype mismatch), bus replay determinism, and
// stage/layer gates. Chain inputs come from the real LX+PA+TY slices
// plus the SE literal/binary/return checks.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, EffectMask, Linkage, NodeKind, NodeRecord, PpTokenKind,
    PpTokenRecord, SemRecord, SpanRecord, StorageDuration, SymbolKind, SymbolRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{
    drive_task, handler_for, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip,
    SeBinChip, SeFuncChip, SeLitChip, SeRetChip, TyConvChip, TyScopeChip, TySymbolChip, TyTypeChip,
    Worker, WorkerRegistry, SE_FUNC_TASK_KIND,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{ChipId, NodeId, RecordRef, TaskId, TokenId, TypeId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    check_stage_layer_agreement, is_se_function_slice_kind, stage_of, ManifestRegistry,
    StoreSchema, PA_TU_CHIP, SE_BIN_CHIP, SE_FUNC_CHIP, SE_LIT_CHIP, SE_RET_CHIP,
    STORE_OWNER_ALLOWLIST, TY_CONV_CHIP, TY_SCOPE_CHIP, TY_SYMBOL_CHIP, TY_TYPE_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::Snapshot;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, ResultValue, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry,
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

fn install_sefunc(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::se_function_slice();
    bus.schema = StoreSchema::se_slice();
    // The fold worker is registered (but never driven directly here):
    // `se.binary` enqueues `const_fold` children whose destination must be
    // a registered manifest owner.
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
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    // Routes pin each kind to its frozen stage (driver-enforced): LX/PA at
    // layer 2, TY at 3, SE at 4, fold at 2.
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
) -> ResultValue {
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

fn completed_ref(
    report: &cc_silicon_compiler::routing::TickReport,
    bus: &CompilerBus,
) -> RecordRef {
    match completed_value(report, bus) {
        ResultValue::Record(reference) => reference,
        other => panic!("expected record ref, got {other:?}"),
    }
}

fn failed_group(report: &cc_silicon_compiler::routing::TickReport, bus: &CompilerBus) -> DiagGroup {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 1);
            bus.arenas
                .diagnostics
                .get(commit.failed[0].1)
                .unwrap()
                .code
                .group
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

/// Handles carried through the chain without borrow fights.
struct Chain {
    bus: CompilerBus,
    int: TypeId,
    func: TypeId,
    tu: NodeId,
    funcdef: NodeId,
    ret: NodeId,
    binary: NodeId,
}

/// Run LX→PA→TY(int, func)→scope→declare and return the bus plus handles.
///
/// Pre-order node allocation from `PARSE_TU`: TU(0) funcdef(1) spec(2)
/// decl(3) compound(4) return(5) binary(6) lit2(7) lit3(8).
fn base_chain() -> Chain {
    let mut bus = new_bus();
    install_sefunc(&mut bus);
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
    let tokens: Vec<cc_silicon_compiler::ids::TokenId> = match report.outcome {
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
    let int = match completed_ref(&report, &bus) {
        RecordRef::Type(id) => id,
        other => panic!("expected type ref, got {other:?}"),
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
        ret: NodeId::from_index(5),
        binary: NodeId::from_index(6),
    }
}

/// Check both literals, fold the binary operand (3 ticks), and check the
/// return statement, leaving the `Return` child fact committed.
fn check_return_child(chain: &mut Chain, workers: &WorkerRegistry) {
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
    let report = tick(&mut chain.bus, workers);
    match completed_ref(&report, &chain.bus) {
        RecordRef::Sem(_) => {}
        other => panic!("expected sem ref, got {other:?}"),
    }
}

// --- 1. frozen kind/stage/registry/manifest -----------------------------------

#[test]
fn se_func_kind_stage_registry_manifest_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(TaskKind::SEMANTIC_FUNCTION_DEF.group(), TaskGroup::SEMANTIC);
    assert_eq!(TaskKind::SEMANTIC_FUNCTION_DEF.local(), 21);
    assert_eq!(SE_FUNC_TASK_KIND, TaskKind::SEMANTIC_FUNCTION_DEF);
    assert!(is_se_function_slice_kind(TaskKind::SEMANTIC_FUNCTION_DEF));
    assert_eq!(stage_of(TaskKind::SEMANTIC_FUNCTION_DEF), Some(4));
    assert!(!is_se_function_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_se_function_slice_kind(TaskKind::SEMANTIC_RETURN_STMT));
    assert!(!is_se_function_slice_kind(
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY
    ));
    // The function registry extends the `/38` head linearly (68 → 69);
    // `SEMANTIC` owners start new codes at local 22.
    assert_eq!(TaskKindRegistry::vf_evidence_slice().len(), 68);
    let registry = TaskKindRegistry::se_function_slice();
    assert_eq!(registry.len(), 69);
    assert_eq!(
        registry
            .lookup(TaskKind::SEMANTIC_FUNCTION_DEF)
            .unwrap()
            .name,
        "semantic.function_def"
    );
    assert_eq!(
        stage_of(TaskKind::new(TaskGroup::SEMANTIC, 22).unwrap()),
        None
    );
    // The chip appends exactly one `sem.records` row for its kind.
    assert_eq!(SE_FUNC_CHIP, ChipId(57));
    assert!(STORE_OWNER_ALLOWLIST.contains(&(
        SE_FUNC_CHIP,
        StoreId::Sem,
        "records",
        TaskKind::SEMANTIC_FUNCTION_DEF,
    )));
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 33);
    let manifest = SeFuncChip.manifest();
    assert_eq!(
        manifest.writes,
        vec![cc_silicon_compiler::manifest::FieldPath::new(
            StoreId::Sem,
            "records"
        )]
    );
    assert!(manifest.deterministic);
    assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
    assert!(manifest.declares_read(StoreId::Parse, "nodes"));
    assert!(manifest.declares_read(StoreId::Symbols, "symbols"));
    assert!(manifest.declares_read(StoreId::Types, "records"));
    assert!(manifest.declares_read(StoreId::Sem, "records"));
    assert_eq!(manifest.tests, vec!["compiler/tests/c39_sefunc.rs"]);
    assert_eq!(manifest.task_kinds, vec![SE_FUNC_TASK_KIND]);
    let routing = {
        let mut bus = new_bus();
        install_sefunc(&mut bus);
        bus.routing
    };
    assert!(check_stage_layer_agreement(&manifest, &routing).is_ok());
}

// --- 2. normal signature ------------------------------------------------------

#[test]
fn se_func_checks_the_normal_signature() {
    let mut chain = base_chain();
    let workers = workers();
    check_return_child(&mut chain, &workers);
    assert_eq!(chain.bus.arenas.sem.allocated(), 4);
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    match completed_ref(&report, &chain.bus) {
        RecordRef::Sem(id) => {
            let sem = chain.bus.arenas.sem.get(id).unwrap();
            assert_eq!(sem.node, chain.funcdef);
            assert_eq!(sem.ty, chain.func);
            assert_eq!(sem.category, ValueCategory::NonLvalue);
            assert_eq!(sem.effects, EffectMask(0));
        }
        other => panic!("expected sem ref, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.sem.allocated(), 5);
    // Exactly one SemRecord carries the function-definition node.
    assert_eq!(
        chain
            .bus
            .arenas
            .sem
            .iter()
            .filter(|(_, sem)| sem.node == chain.funcdef)
            .count(),
        1
    );
    let _ = Snapshot::capture(&chain.bus);
    let _ = chain.tu;
    let _ = chain.int;
}

// --- 3. reuse ------------------------------------------------------------------

#[test]
fn se_func_reuses_the_committed_record() {
    let mut chain = base_chain();
    let workers = workers();
    check_return_child(&mut chain, &workers);
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    let first = match completed_ref(&report, &chain.bus) {
        RecordRef::Sem(id) => id,
        other => panic!("expected sem ref, got {other:?}"),
    };
    assert_eq!(chain.bus.arenas.sem.allocated(), 5);
    // A second task for the same node reuses the committed fact: no duplicate.
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    match completed_ref(&report, &chain.bus) {
        RecordRef::Sem(id) => assert_eq!(id, first),
        other => panic!("expected sem ref, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.sem.allocated(), 5);
}

// --- 4. missing body -----------------------------------------------------------

#[test]
fn se_func_missing_checked_body_fails() {
    // Dispatch with the return statement UNCHECKED: the missing child fact
    // fails loudly with nothing appended for the function node.
    let mut chain = base_chain();
    let workers = workers();
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
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // A missing child fact is missing input (task group), and nothing is
    // appended for the function node.
    assert_eq!(failed_group(&report, &chain.bus), DiagGroup::Task);
    assert_eq!(chain.bus.arenas.sem.allocated(), 2);
}

// --- 5. K&R parameter list ------------------------------------------------------

fn alloc_node(chain: &mut Chain, kind: NodeKind, children: Vec<NodeId>) -> NodeId {
    let limits = chain.bus.limits();
    chain
        .bus
        .arenas
        .nodes
        .alloc(
            NodeRecord {
                kind,
                parent: None,
                children,
                first_token: TokenId::from_index(0),
                last_token: TokenId::from_index(0),
                name: None,
                literal: None,
            },
            &limits,
        )
        .unwrap()
}

#[test]
fn se_func_kr_parameter_list_is_unsupported() {
    let mut chain = base_chain();
    let workers = workers();
    // A declarator carrying an old-style identifier list is explicitly
    // unsupported (never silently accepted as `(void)`).
    let spec = alloc_node(&mut chain, NodeKind::Specifiers, vec![]);
    let kndr = alloc_node(&mut chain, NodeKind::Declarator, vec![spec]);
    let ret = alloc_node(&mut chain, NodeKind::Return, vec![]);
    let compound = alloc_node(&mut chain, NodeKind::Compound, vec![ret]);
    let funcdef = alloc_node(
        &mut chain,
        NodeKind::FunctionDefinition,
        vec![spec, kndr, compound],
    );
    let sem_before = chain.bus.arenas.sem.allocated();
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(failed_group(&report, &chain.bus), DiagGroup::Unsupported);
    assert_eq!(chain.bus.arenas.sem.allocated(), sem_before);
}

// --- 6. prototype mismatch -------------------------------------------------------

#[test]
fn se_func_prototype_mismatch_is_unsupported() {
    let mut chain = base_chain();
    let workers = workers();
    // A second definition shape whose declared symbol carries `int`
    // instead of the committed `int(void)` signature diverges from its
    // prototype: explicit `Unsupported`, never a recorded plan.
    let spec = alloc_node(&mut chain, NodeKind::Specifiers, vec![]);
    let decl = alloc_node(&mut chain, NodeKind::Declarator, vec![]);
    let ret = alloc_node(&mut chain, NodeKind::Return, vec![]);
    let compound = alloc_node(&mut chain, NodeKind::Compound, vec![ret]);
    let funcdef = alloc_node(
        &mut chain,
        NodeKind::FunctionDefinition,
        vec![spec, decl, compound],
    );
    let limits = chain.bus.limits();
    chain
        .bus
        .arenas
        .sem
        .alloc(
            SemRecord {
                node: ret,
                ty: chain.int,
                category: ValueCategory::NonLvalue,
                effects: EffectMask(0),
            },
            &limits,
        )
        .unwrap();
    let main = chain.bus.intern.lookup(b"main").unwrap();
    let file_scope = chain
        .bus
        .arenas
        .scopes
        .iter()
        .next()
        .map(|(id, _)| id)
        .unwrap();
    chain
        .bus
        .arenas
        .symbols
        .alloc(
            SymbolRecord {
                name: main,
                scope: file_scope,
                kind: SymbolKind::Function,
                ty: Some(chain.int),
                linkage: Linkage::External,
                storage: StorageDuration::Static,
                decl,
            },
            &limits,
        )
        .unwrap();
    let sem_before = chain.bus.arenas.sem.allocated();
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(funcdef)]),
    );
    let report = tick(&mut chain.bus, &workers);
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(failed_group(&report, &chain.bus), DiagGroup::Unsupported);
    assert_eq!(chain.bus.arenas.sem.allocated(), sem_before);
}

// --- 7. determinism ---------------------------------------------------------------

fn run_scenario() -> (Vec<u8>, usize, usize) {
    let mut chain = base_chain();
    let workers = workers();
    check_return_child(&mut chain, &workers);
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.funcdef)]),
    );
    let mut completed = 0;
    let mut failed = 0;
    for _ in 0..8 {
        let report = tick(&mut chain.bus, &workers);
        match &report.outcome {
            TickOutcome::Executed { commit, .. } => {
                completed += commit.completed.len();
                failed += commit.failed.len();
            }
            other => panic!("expected executed, got {other:?}"),
        }
        if completed + failed >= 1 {
            break;
        }
    }
    (
        Snapshot::capture(&chain.bus).bytes().to_vec(),
        completed,
        failed,
    )
}

#[test]
fn se_func_bus_dispatch_replays_deterministically() {
    let (first, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (1, 0));
    let (second, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (1, 0));
    assert_eq!(first, second);
}

// --- 8. gates ---------------------------------------------------------------------

#[test]
fn se_func_stage_layer_manifest_gates() {
    // A wrong layer is refused on the driver path.
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::se_function_slice();
    mismatched.schema = StoreSchema::se_slice();
    mismatched
        .registrations
        .register(SeFuncChip.manifest(), &mismatched.schema, &mismatched.kinds)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::SEMANTIC_FUNCTION_DEF, SE_FUNC_CHIP, 9)
        .unwrap();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::empty(),
    );
    let error = drive_task(&mismatched, task, &workers()).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // The pre-`/39` registry does not know the frozen kind, so the
    // manifest is rejected there (frozen kinds are never re-registered).
    let mut stale = ManifestRegistry::new();
    let kinds = TaskKindRegistry::vf_evidence_slice();
    let schema = StoreSchema::se_slice();
    assert!(stale
        .register(SeFuncChip.manifest(), &schema, &kinds)
        .is_err());
}
