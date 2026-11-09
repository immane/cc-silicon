// ============================================================================
// c14_se.rs — Wave 2 (`/14`) SE semantic-check + VF06 slice acceptance.
//
// Covers the frozen closure: the three SE kinds plus the VF06 verifier
// kind, kind-to-stage assignment, the SE store-owner allowlist rows,
// stage/layer agreement, the two-phase binary handoff through a real
// `const_fold` child (M1-CL-05 up to T09), snapshot bodies,
// tick-lifecycle integration, and the `/14` hash participation. The `/40`
// VF06 delta extends the checked set to the four M1 kinds
// (`IntLiteral`, `BinaryAdd`, `Return`, `FunctionDefinition`) with the
// unique committed `int(void)` signature fact. Chain inputs come from the
// real LX+PA+TY slices; the automatic File-Enter edge stays deferred
// (scope tasks dispatch after the TU commit).
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, EffectMask, NodeKind, PpTokenKind, PpTokenRecord, SemRecord,
    SpanRecord, TypeKind, TypeRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{
    handler_for, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip, SeBinChip, SeLitChip,
    SeRetChip, TyConvChip, TyScopeChip, TySymbolChip, TyTypeChip, Vf06Chip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{NodeId, RecordRef, SemId, TaskId, TokenId, TypeId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_se_slice_kind, stage_of, FieldPath, ManifestRegistryError, StoreSchema, PA_TU_CHIP,
    SE_BIN_CHIP, SE_LIT_CHIP, SE_RET_CHIP, STORE_OWNER_ALLOWLIST, TY_CONV_CHIP, TY_SCOPE_CHIP,
    TY_SYMBOL_CHIP, TY_TYPE_CHIP, VF06_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{decode_sem, encode_sem, Snapshot};
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

fn install_se(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::se_slice();
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
        &Vf06Chip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    // Routes pin each kind to its frozen stage (driver-enforced): LX/PA at
    // layer 2, TY at 3, SE/VF06 at 4, fold at 2.
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

fn completed_ref(
    report: &cc_silicon_compiler::routing::TickReport,
    bus: &CompilerBus,
) -> RecordRef {
    match completed_value(report, bus) {
        cc_silicon_compiler::task::ResultValue::Record(reference) => reference,
        other => panic!("expected record ref, got {other:?}"),
    }
}

/// Handles carried through the chain without borrow fights.
struct Chain {
    bus: CompilerBus,
    int: TypeId,
    func: TypeId,
    tu: NodeId,
    /// Function definition node (pre-order `PA_TU` allocation: node 1).
    funcdef: NodeId,
    lit2: NodeId,
    lit3: NodeId,
    binary: NodeId,
    ret: NodeId,
}

/// Run PP→LX→PA→TY(int, func) and return the bus plus node/type handles.
fn base_chain() -> Chain {
    let mut bus = new_bus();
    install_se(&mut bus);
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
    Chain {
        bus,
        int,
        func,
        tu,
        funcdef: NodeId::from_index(1),
        lit2: NodeId::from_index(7),
        lit3: NodeId::from_index(8),
        binary: NodeId::from_index(6),
        ret: NodeId::from_index(5),
    }
}

/// Drive the frozen SE literal → binary(+fold) → return chain, leaving the
/// four `/14` facts (two literals, binary, return) committed.
fn seed_complete_se_facts(chain: &mut Chain, workers: &WorkerRegistry) {
    for node in [chain.lit2, chain.lit3] {
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
    // One tick runs the fold child, one resumes the parent, and two drain
    // (the `/14` positive VF06 sequence).
    for _ in 0..4 {
        tick(&mut chain.bus, workers);
    }
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_RETURN_STMT,
        SE_RET_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.ret)]),
    );
    tick(&mut chain.bus, workers);
    assert_eq!(chain.bus.arenas.sem.allocated(), 4);
}

/// Seed one committed `SemRecord` directly.
///
/// Unit isolation: the real producers are the SE chips, and the
/// function-definition fact is normally committed by SE29 (exercised
/// end-to-end in `c39_sefunc.rs`). Direct seeding keeps these VF06 cases
/// focused on the verifier's four-kind checked set.
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

/// Seed the canonical `FunctionDefinition` signature fact.
fn seed_function_fact(chain: &mut Chain) -> SemId {
    seed_sem_fact(
        chain,
        chain.funcdef,
        chain.func,
        ValueCategory::NonLvalue,
        EffectMask(0),
    )
}

/// Append one committed node (test-only negative-shape construction).
fn alloc_node(chain: &mut Chain, kind: NodeKind, children: Vec<NodeId>) -> NodeId {
    let limits = chain.bus.limits();
    chain
        .bus
        .arenas
        .nodes
        .alloc(
            cc_silicon_compiler::bus::NodeRecord {
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

/// Dispatch VF06 on the chain TU and tick once.
fn vf06_tick(
    chain: &mut Chain,
    workers: &WorkerRegistry,
) -> cc_silicon_compiler::routing::TickReport {
    bootstrap(
        &mut chain.bus,
        TaskKind::VERIFICATION_TYPED_INVARIANT,
        VF06_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.tu)]),
    );
    tick(&mut chain.bus, workers)
}

/// Require VF06 to complete `Ack`.
fn vf06_expect_ack(chain: &mut Chain, workers: &WorkerRegistry) {
    let report = vf06_tick(chain, workers);
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

/// Require VF06 to fail with one diagnostic; return its group and message.
///
/// VF06 writes nothing, including on failure.
fn vf06_fail(chain: &mut Chain, workers: &WorkerRegistry) -> (DiagGroup, String) {
    let sem_before = chain.bus.arenas.sem.allocated();
    let report = vf06_tick(chain, workers);
    let (group, message) = match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 0);
            assert_eq!(commit.failed.len(), 1);
            let diagnostic = chain
                .bus
                .arenas
                .diagnostics
                .get(commit.failed[0].1)
                .unwrap();
            (diagnostic.code.group, diagnostic.message.clone())
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    };
    assert_eq!(chain.bus.arenas.sem.allocated(), sem_before);
    (group, message)
}

#[test]
fn se_kinds_stages_and_allowlist_are_frozen() {
    let kinds = [
        (TaskKind::SEMANTIC_LITERAL_EXPR, "semantic.literal_expr", 18),
        (TaskKind::SEMANTIC_BINARY_EXPR, "semantic.binary_expr", 19),
        (TaskKind::SEMANTIC_RETURN_STMT, "semantic.return_stmt", 20),
        (
            TaskKind::VERIFICATION_TYPED_INVARIANT,
            "verification.typed_invariant",
            16,
        ),
    ];
    for (kind, _, _) in kinds {
        assert!(is_se_slice_kind(kind));
        assert_eq!(stage_of(kind), Some(4));
    }
    assert!(!is_se_slice_kind(TaskKind::CONTROL_NOOP));
    let registry = TaskKindRegistry::se_slice();
    assert_eq!(registry.len(), 25);
    for (kind, name, _) in kinds {
        assert_eq!(registry.lookup(kind).unwrap().name, name);
    }
    assert_eq!(TaskKind::SEMANTIC_LITERAL_EXPR.group(), TaskGroup::SEMANTIC);
    assert_eq!(
        TaskKind::VERIFICATION_TYPED_INVARIANT.group(),
        TaskGroup::VERIFICATION
    );
    for row in [
        (
            SE_LIT_CHIP,
            cc_silicon_compiler::task::StoreId::Sem,
            "records",
            TaskKind::SEMANTIC_LITERAL_EXPR,
        ),
        (
            SE_BIN_CHIP,
            cc_silicon_compiler::task::StoreId::Sem,
            "records",
            TaskKind::SEMANTIC_BINARY_EXPR,
        ),
        (
            SE_RET_CHIP,
            cc_silicon_compiler::task::StoreId::Sem,
            "records",
            TaskKind::SEMANTIC_RETURN_STMT,
        ),
    ] {
        assert!(STORE_OWNER_ALLOWLIST.contains(&row));
    }
}

#[test]
fn se_literal_commits_checked_facts_with_reuse() {
    let mut chain = base_chain();
    let workers = workers();
    for node in [chain.lit2, chain.lit3] {
        bootstrap(
            &mut chain.bus,
            TaskKind::SEMANTIC_LITERAL_EXPR,
            SE_LIT_CHIP,
            Payload::from_refs(vec![RecordRef::Node(node)]),
        );
        let report = tick(&mut chain.bus, &workers);
        match completed_ref(&report, &chain.bus) {
            RecordRef::Sem(id) => {
                let sem = chain.bus.arenas.sem.get(id).unwrap();
                assert_eq!(sem.node, node);
                assert_eq!(sem.ty, chain.int);
                assert_eq!(sem.category, ValueCategory::NonLvalue);
                assert_eq!(sem.effects, EffectMask(0));
            }
            other => panic!("expected sem ref, got {other:?}"),
        }
    }
    assert_eq!(chain.bus.arenas.sem.allocated(), 2);
    // A second task for the same node reuses the committed fact: no duplicate.
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_LITERAL_EXPR,
        SE_LIT_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.lit2)]),
    );
    let report = tick(&mut chain.bus, &workers);
    match completed_ref(&report, &chain.bus) {
        RecordRef::Sem(id) => assert_eq!(id.index(), 0),
        other => panic!("expected sem ref, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.sem.allocated(), 2);
    // Snapshot round-trips the new sem bodies.
    let sem = chain
        .bus
        .arenas
        .sem
        .get(cc_silicon_compiler::ids::SemId::from_index(0))
        .unwrap()
        .clone();
    let bytes = encode_sem(&sem);
    assert_eq!(decode_sem(&bytes).unwrap(), sem);
    let _ = Snapshot::capture(&chain.bus);
    let _ = NodeKind::IntLiteral;
}

#[test]
fn se_binary_two_phase_handoff_folds_through_a_real_child() {
    let mut chain = base_chain();
    let workers = workers();
    for node in [chain.lit2, chain.lit3] {
        bootstrap(
            &mut chain.bus,
            TaskKind::SEMANTIC_LITERAL_EXPR,
            SE_LIT_CHIP,
            Payload::from_refs(vec![RecordRef::Node(node)]),
        );
        tick(&mut chain.bus, &workers);
    }
    // Phase 1: Sem appended, fold child enqueued, parent waiting.
    let parent = bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_BINARY_EXPR,
        SE_BIN_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.binary)]),
    );
    let report = tick(&mut chain.bus, &workers);
    match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.waiting, vec![parent]);
            assert_eq!(commit.enqueued.len(), 1);
        }
        other => panic!("expected executed, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.sem.allocated(), 3);
    // Phase 2 (next tick): the fold runs and completes `Const 5`; the join
    // readies the parent in the same commit.
    let report = tick(&mut chain.bus, &workers);
    match report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.completed.len(), 1),
        other => panic!("expected executed, got {other:?}"),
    }
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
    // Phase 3: the readied parent resumes and completes with its SemRecord.
    let report = tick(&mut chain.bus, &workers);
    match completed_ref(&report, &chain.bus) {
        RecordRef::Sem(id) => {
            let sem = chain.bus.arenas.sem.get(id).unwrap();
            assert_eq!(sem.node, chain.binary);
            assert_eq!(sem.ty, chain.int);
        }
        other => panic!("expected sem ref, got {other:?}"),
    }
    // Exactly one SemRecord for the binary node: no duplicate on resume.
    assert_eq!(
        chain
            .bus
            .arenas
            .sem
            .iter()
            .filter(|(_, sem)| sem.node == chain.binary)
            .count(),
        1
    );
}

#[test]
fn se_binary_requires_checked_operands() {
    // Dispatch binary check with NO literal SemRecords: must fail loudly
    // with nothing appended and no fold child enqueued.
    let mut chain = base_chain();
    let workers = workers();
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_BINARY_EXPR,
        SE_BIN_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.binary)]),
    );
    let report = tick(&mut chain.bus, &workers);
    match report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.sem.allocated(), 0);
    assert_eq!(chain.bus.arenas.consts.allocated(), 0);
}

#[test]
fn se_return_checks_the_identity_corner() {
    let mut chain = base_chain();
    let workers = workers();
    for node in [chain.lit2, chain.lit3] {
        bootstrap(
            &mut chain.bus,
            TaskKind::SEMANTIC_LITERAL_EXPR,
            SE_LIT_CHIP,
            Payload::from_refs(vec![RecordRef::Node(node)]),
        );
        tick(&mut chain.bus, &workers);
    }
    // Binary without its fold join yet: run phase 1 only, then return must
    // fail (operand unchecked) — ordering is enforced, not assumed.
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_RETURN_STMT,
        SE_RET_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.ret)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // Complete the binary handoff (3 ticks: fold, resume, idle drain).
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_BINARY_EXPR,
        SE_BIN_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.binary)]),
    );
    tick(&mut chain.bus, &workers);
    tick(&mut chain.bus, &workers);
    tick(&mut chain.bus, &workers);
    tick(&mut chain.bus, &workers);
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_RETURN_STMT,
        SE_RET_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.ret)]),
    );
    assert!(matches!(
        chain.bus.arenas.types.get(chain.func).unwrap().kind,
        cc_silicon_compiler::bus::TypeKind::Function { .. }
    ));
    let report = tick(&mut chain.bus, &workers);
    match completed_ref(&report, &chain.bus) {
        RecordRef::Sem(id) => {
            let sem = chain.bus.arenas.sem.get(id).unwrap();
            assert_eq!(sem.node, chain.ret);
            assert_eq!(sem.ty, chain.int);
            assert_eq!(sem.category, ValueCategory::NonLvalue);
            assert_eq!(sem.effects, EffectMask(0));
        }
        other => panic!("expected sem ref, got {other:?}"),
    }
}

#[test]
fn vf06_passes_on_complete_facts_and_fails_on_gaps() {
    // Full facts (four checked kinds plus the seeded function-signature
    // fact): pass.
    let mut chain = base_chain();
    let workers = workers();
    seed_complete_se_facts(&mut chain, &workers);
    seed_function_fact(&mut chain);
    assert_eq!(chain.bus.arenas.sem.allocated(), 5);
    vf06_expect_ack(&mut chain, &workers);
    // Gap: nodes committed but no SemRecords — must fail loudly.
    let mut chain = base_chain();
    let (group, _) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Task);
    assert_eq!(chain.bus.arenas.sem.allocated(), 0);
}

// A3-U01: the full M1 fact set (two literals, binary, return, function
// definition) is the exact four-kind checked set and completes `Ack`.
#[test]
fn vf06_acks_full_m1_four_kind_facts() {
    let mut chain = base_chain();
    let workers = workers();
    seed_complete_se_facts(&mut chain, &workers);
    let function_sem = seed_function_fact(&mut chain);
    assert_eq!(chain.bus.arenas.sem.allocated(), 5);
    let sem = chain.bus.arenas.sem.get(function_sem).unwrap();
    assert_eq!(sem.node, chain.funcdef);
    assert_eq!(sem.ty, chain.func);
    assert_eq!(sem.category, ValueCategory::NonLvalue);
    assert_eq!(sem.effects, EffectMask(0));
    vf06_expect_ack(&mut chain, &workers);
}

// A3-U02: without the function-definition fact (all four expression facts
// present), VF06 fails as missing input — never a vacuous pass.
#[test]
fn vf06_missing_function_fact_fails() {
    let mut chain = base_chain();
    let workers = workers();
    seed_complete_se_facts(&mut chain, &workers);
    let (group, message) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Task);
    assert!(
        message.contains("missing typed fact"),
        "unexpected message: {message}"
    );
}

// A3-U03: two function-definition facts are a duplicate-fact defect.
#[test]
fn vf06_duplicate_function_facts_fail() {
    let mut chain = base_chain();
    let workers = workers();
    seed_complete_se_facts(&mut chain, &workers);
    seed_function_fact(&mut chain);
    seed_function_fact(&mut chain);
    assert_eq!(chain.bus.arenas.sem.allocated(), 6);
    let (group, message) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Task);
    assert!(
        message.contains("duplicate typed fact"),
        "unexpected message: {message}"
    );
}

// A3-U04: a function-definition fact typed with canonical `int` is the
// wrong signature fact; the `/14` blanket `ty == int` rule must not apply.
#[test]
fn vf06_function_fact_with_int_type_fails() {
    let mut chain = base_chain();
    let workers = workers();
    seed_complete_se_facts(&mut chain, &workers);
    let funcdef = chain.funcdef;
    let int = chain.int;
    seed_sem_fact(
        &mut chain,
        funcdef,
        int,
        ValueCategory::NonLvalue,
        EffectMask(0),
    );
    let (group, message) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Task);
    assert!(
        message.contains("non-signature type"),
        "unexpected message: {message}"
    );
}

// A3-U05: the function fact requires `NonLvalue` and `EffectMask(0)`.
#[test]
fn vf06_function_fact_requires_nonlvalue_and_zero_effects() {
    // Wrong value category.
    let mut chain = base_chain();
    let workers = workers();
    seed_complete_se_facts(&mut chain, &workers);
    let funcdef = chain.funcdef;
    let func = chain.func;
    seed_sem_fact(
        &mut chain,
        funcdef,
        func,
        ValueCategory::Lvalue,
        EffectMask(0),
    );
    let (group, message) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Task);
    assert!(
        message.contains("value category"),
        "unexpected message: {message}"
    );
    // Nonzero effects.
    let mut chain = base_chain();
    seed_complete_se_facts(&mut chain, &workers);
    let funcdef = chain.funcdef;
    let func = chain.func;
    seed_sem_fact(
        &mut chain,
        funcdef,
        func,
        ValueCategory::NonLvalue,
        EffectMask(1),
    );
    let (group, message) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Task);
    assert!(
        message.contains("effect mask"),
        "unexpected message: {message}"
    );
}

// A3-U06: zero or multiple `FunctionDefinition` nodes in the M1 TU fail;
// VF06 must not pass vacuously when the lowering consumer's node is absent.
#[test]
fn vf06_requires_exactly_one_function_definition_node() {
    let workers = workers();
    // Zero: strip the TU child.
    let mut chain = base_chain();
    chain
        .bus
        .arenas
        .nodes
        .get_mut(chain.tu)
        .unwrap()
        .children
        .clear();
    let (group, message) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Task);
    assert!(
        message.contains("missing function definition"),
        "unexpected message: {message}"
    );
    // Multiple: a second function-definition node under the TU.
    let mut chain = base_chain();
    let extra = alloc_node(&mut chain, NodeKind::FunctionDefinition, vec![]);
    chain
        .bus
        .arenas
        .nodes
        .get_mut(chain.tu)
        .unwrap()
        .children
        .push(extra);
    let (group, message) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Unsupported);
    assert!(
        message.contains("multiple function definitions outside M1"),
        "unexpected message: {message}"
    );
}

// A3-U14: a committed non-M1 function type (parameters, non-prototype, or
// variadic) is an explicit unsupported failure, never a silent pass. The
// same projection requires a signature at all and rejects duplicates.
#[test]
fn vf06_requires_exactly_one_m1_signature() {
    let workers = workers();
    // Non-M1 function shapes committed next to the M1 signature.
    for index in 0..3 {
        let mut chain = base_chain();
        let kind = match index {
            0 => TypeKind::Function {
                result: chain.int,
                params: vec![chain.int],
                prototype: true,
                variadic: false,
            },
            1 => TypeKind::Function {
                result: chain.int,
                params: vec![],
                prototype: false,
                variadic: false,
            },
            _ => TypeKind::Function {
                result: chain.int,
                params: vec![],
                prototype: true,
                variadic: true,
            },
        };
        let limits = chain.bus.limits();
        chain
            .bus
            .arenas
            .types
            .alloc(TypeRecord { kind }, &limits)
            .unwrap();
        let (group, message) = vf06_fail(&mut chain, &workers);
        assert_eq!(group, DiagGroup::Unsupported);
        assert!(
            message.contains("non-M1 function signature"),
            "unexpected message: {message}"
        );
    }
    // Zero committed function types: missing input, not a pass.
    let mut chain = base_chain();
    chain.bus.arenas.types.remove(chain.func).unwrap();
    let (group, message) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Task);
    assert!(
        message.contains("missing int(void) signature"),
        "unexpected message: {message}"
    );
    // Duplicate M1 signatures: an ambiguous handoff.
    let mut chain = base_chain();
    let limits = chain.bus.limits();
    chain
        .bus
        .arenas
        .types
        .alloc(
            TypeRecord {
                kind: TypeKind::Function {
                    result: chain.int,
                    params: vec![],
                    prototype: true,
                    variadic: false,
                },
            },
            &limits,
        )
        .unwrap();
    let (group, message) = vf06_fail(&mut chain, &workers);
    assert_eq!(group, DiagGroup::Task);
    assert!(
        message.contains("duplicate int(void) signature"),
        "unexpected message: {message}"
    );
}

#[test]
fn se_rejects_malformed_and_dangling_inputs() {
    let mut chain = base_chain();
    let workers = workers();
    // Empty payload.
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_LITERAL_EXPR,
        SE_LIT_CHIP,
        Payload::empty(),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // Dangling node.
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_LITERAL_EXPR,
        SE_LIT_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(77))]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // Wrong node kind (TU is not an expression).
    bootstrap(
        &mut chain.bus,
        TaskKind::SEMANTIC_LITERAL_EXPR,
        SE_LIT_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.tu)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.sem.allocated(), 0);
}

#[test]
fn se_stage_layer_and_manifest_gates() {
    let workers = workers();
    // Drive-task refusal uses a bus whose route disagrees with the stage.
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::se_slice();
    mismatched.schema = StoreSchema::se_slice();
    for chip in [&SeLitChip as &dyn Worker, &SeBinChip, &SeRetChip, &Vf06Chip] {
        mismatched
            .registrations
            .register(chip.manifest(), &mismatched.schema, &mismatched.kinds)
            .unwrap();
    }
    mismatched
        .routing
        .register(TaskKind::SEMANTIC_LITERAL_EXPR, SE_LIT_CHIP, 9)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::SEMANTIC_BINARY_EXPR, SE_BIN_CHIP, 9)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::SEMANTIC_RETURN_STMT, SE_RET_CHIP, 9)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::VERIFICATION_TYPED_INVARIANT, VF06_CHIP, 9)
        .unwrap();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::SEMANTIC_LITERAL_EXPR,
        SE_LIT_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // A manifest missing its write is rejected at registration.
    let mut manifest = SeLitChip.manifest();
    manifest.writes = vec![];
    assert!(matches!(
        mismatched.registrations.register(
            manifest,
            &StoreSchema::se_slice(),
            &TaskKindRegistry::se_slice()
        ),
        Err(ManifestRegistryError::Registry(_))
    ));
    let _ = FieldPath::new(cc_silicon_compiler::task::StoreId::Sem, "records");
    let _ = DiagGroup::Semantic;
}

#[test]
fn se_snapshot_replay_is_deterministic() {
    let run = || {
        let mut chain = base_chain();
        let workers = workers();
        for node in [chain.lit2, chain.lit3] {
            bootstrap(
                &mut chain.bus,
                TaskKind::SEMANTIC_LITERAL_EXPR,
                SE_LIT_CHIP,
                Payload::from_refs(vec![RecordRef::Node(node)]),
            );
            tick(&mut chain.bus, &workers);
        }
        Snapshot::capture(&chain.bus)
    };
    assert_eq!(run(), run());
}
