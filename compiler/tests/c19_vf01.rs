// ============================================================================
// c19_vf01.rs — Wave 2 (`/19`) VF01 store-invariant slice acceptance.
//
// Covers the frozen closure: the `verification.store_invariant` kind
// (local 19), kind-to-stage assignment (stage 6), the VF01 manifest (reads
// only, no allowlist writes), the M1 store contract (payload/result
// references resolve, parents/continuations resolve, spans name committed
// sources within bounds, reserved stores hold nothing), malformed/
// dangling/reserved negatives, stage/layer agreement, and the `/19` hash
// participation. Chain inputs come from the real PP→LX→PA→TY→SE→VF06→IR
// slices; host-request references and tombstone liveness stay deferred.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord, SpanRecord};
use cc_silicon_compiler::chips::{
    handler_for, IrFunctionChip, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip,
    SeBinChip, SeLitChip, SeRetChip, TyConvChip, TyScopeChip, TySymbolChip, TyTypeChip, Vf01Chip,
    Vf06Chip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{NodeId, RecordRef, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_vf01_slice_kind, stage_of, FieldPath, IR_FUNCTION_CHIP, PA_TU_CHIP, SE_BIN_CHIP,
    SE_LIT_CHIP, SE_RET_CHIP, TY_CONV_CHIP, TY_SCOPE_CHIP, TY_SYMBOL_CHIP, TY_TYPE_CHIP, VF01_CHIP,
    VF06_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::Snapshot;
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

fn install_vf01(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::StoreSchema;
    bus.kinds = TaskKindRegistry::vf01_slice();
    // No new record families: the VF01 slice reuses the PP-slice schema.
    bus.schema = StoreSchema::pp_slice();
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
        &Vf01Chip,
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
        .register(TaskKind::VERIFICATION_STORE_INVARIANT, VF01_CHIP, 6)
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
    workers.register(Vf01Chip).unwrap();
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

/// Run the full PP→LX→PA→TY→SE→VF06→IR chain from seeded PP fixtures.
fn full_chain() -> CompilerBus {
    let mut bus = new_bus();
    install_vf01(&mut bus);
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
    bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_TYPED_INVARIANT,
        VF06_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(0))]),
    );
    tick(&mut bus, &workers);
    bootstrap(
        &mut bus,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(1))]),
    );
    tick(&mut bus, &workers);
    let _ = (int, func);
    bus
}

fn run_vf01(bus: &mut CompilerBus) -> cc_silicon_compiler::routing::TickReport {
    let workers = workers();
    bootstrap(
        bus,
        TaskKind::VERIFICATION_STORE_INVARIANT,
        VF01_CHIP,
        Payload::empty(),
    );
    tick(bus, &workers)
}

#[test]
fn vf01_kind_stage_and_allowlist_are_frozen() {
    use cc_silicon_compiler::manifest::StoreSchema;
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::VERIFICATION_STORE_INVARIANT.group(),
        TaskGroup::VERIFICATION
    );
    assert_eq!(TaskKind::VERIFICATION_STORE_INVARIANT.local(), 19);
    assert!(is_vf01_slice_kind(TaskKind::VERIFICATION_STORE_INVARIANT));
    assert!(!is_vf01_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_vf01_slice_kind(
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT
    ));
    assert_eq!(stage_of(TaskKind::VERIFICATION_STORE_INVARIANT), Some(6));
    let registry = TaskKindRegistry::vf01_slice();
    assert_eq!(registry.len(), 32);
    assert_eq!(
        registry
            .lookup(TaskKind::VERIFICATION_STORE_INVARIANT)
            .unwrap()
            .name,
        "verification.store_invariant"
    );
    // A read-only verifier: no writes, so no allowlist rows are required.
    assert!(Vf01Chip.manifest().writes.is_empty());
    assert!(!Vf01Chip.manifest().reads.is_empty());
    let _ = StoreSchema::pp_slice();
    let _ = StoreId::Diagnostics;
    let _ = FieldPath::new(StoreId::Sources, "spans");
}

#[test]
fn vf01_accepts_healthy_bus() {
    let mut bus = full_chain();
    let report = run_vf01(&mut bus);
    match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            let value = bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .value
                .clone();
            assert_eq!(value, cc_silicon_compiler::task::ResultValue::Ack);
        }
        other => panic!("expected executed, got {other:?}"),
    }
}

#[test]
fn vf01_rejects_malformed_inputs() {
    let mut bus = full_chain();
    let workers = workers();
    // A non-empty payload violates the global-snapshot convention.
    bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_STORE_INVARIANT,
        VF01_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(0))]),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

#[test]
fn vf01_rejects_reserved_records_and_escaping_spans() {
    // A record in a still-reserved store breaks the M1 contract.
    let mut bus = full_chain();
    let limits = bus.limits();
    bus.arenas.layouts.alloc(&limits).unwrap();
    match run_vf01(&mut bus).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // A span escaping its source breaks the span contract.
    let mut bus = full_chain();
    let limits = bus.limits();
    let source = bus.arenas.sources.iter().next().unwrap().0;
    bus.arenas
        .spans
        .alloc(
            SpanRecord {
                source,
                start: 0,
                end: 999_999,
                expansion: None,
            },
            &limits,
        )
        .unwrap();
    match run_vf01(&mut bus).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

#[test]
fn vf01_rejects_dangling_references() {
    use cc_silicon_compiler::chips::{project_vf01_input, Vf01Chip};
    use cc_silicon_compiler::task::TaskState;
    let mut bus = full_chain();
    let task = bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_STORE_INVARIANT,
        VF01_CHIP,
        Payload::empty(),
    );
    let good = project_vf01_input(&bus, task).expect("healthy bus projects");
    let mut good = good;
    good.state = TaskState::Running;
    // A dangling payload reference breaks ID ownership.
    let mut dangling = good.clone();
    dangling.tasks[0]
        .1
        .push(RecordRef::Node(NodeId::from_index(9999)));
    assert!(matches!(
        Vf01Chip.compute(&dangling)[0],
        cc_silicon_compiler::task::Proposal::Fail { .. }
    ));
    // The healthy snapshot acknowledges.
    match &Vf01Chip.compute(&good)[..] {
        [cc_silicon_compiler::task::Proposal::Complete { value, .. }] => {
            assert_eq!(*value, cc_silicon_compiler::task::ResultValue::Ack)
        }
        other => panic!("expected single ack, got {other:?}"),
    }
}

#[test]
fn vf01_stage_layer_and_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut bus = new_bus();
    install_vf01(&mut bus);
    // Re-point the VF01 route at a wrong layer on a scratch bus.
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::vf01_slice();
    mismatched.schema = StoreSchema::pp_slice();
    mismatched
        .registrations
        .register(Vf01Chip.manifest(), &mismatched.schema, &mismatched.kinds)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::VERIFICATION_STORE_INVARIANT, VF01_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::VERIFICATION_STORE_INVARIANT,
        VF01_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // A manifest claiming the new kind is rejected against the stale
    // pre-`/19` registry that does not know it.
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            Vf01Chip.manifest(),
            &StoreSchema::pp_slice(),
            &TaskKindRegistry::vf05_slice(),
        )
        .is_err());
    let _ = FieldPath::new(StoreId::Diagnostics, "entries");
    use cc_silicon_compiler::task::StoreId;
}

#[test]
fn vf01_snapshot_replay_is_deterministic() {
    let run = || {
        let mut bus = full_chain();
        run_vf01(&mut bus);
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
