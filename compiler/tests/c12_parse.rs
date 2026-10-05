// ============================================================================
// c12_parse.rs — Wave 2 (`/12`) PA translation-unit slice acceptance.
//
// Covers the frozen closure: the nine-node M1 tree, the `parse.tu` kind,
// kind-to-stage assignment, the PA store-owner allowlist row, stage/layer
// agreement, snapshot bodies, tick-lifecycle integration, and the `/12`
// hash participation. Token/literal fixtures come from the real LX slice
// chain (intern → classify → decode); the PP layer stays seeded. The
// File-Enter edge stays deferred to the T06 slice: the TU root is committed
// first with no scope edge emitted.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, NodeKind, PpTokenKind, PpTokenRecord, SpanRecord,
};
use cc_silicon_compiler::chips::{
    handler_for, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip, Worker,
    WorkerRegistry,
};
use cc_silicon_compiler::ids::{PpTokenId, RecordRef, SourceId, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_pa_slice_kind, stage_of, FieldPath, ManifestRegistryError, StoreSchema, PA_TU_CHIP,
    STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{decode_node, encode_node, Snapshot};
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

fn install_pa(bus: &mut CompilerBus, layer: u16) {
    bus.kinds = TaskKindRegistry::pa_slice();
    bus.schema = StoreSchema::pa_slice();
    for chip in [
        &LxInternChip as &dyn Worker,
        &LxClassifyChip,
        &LxDecodeLiteralChip,
        &PaTuChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(
            TaskKind::LEX_INTERN,
            cc_silicon_compiler::manifest::LX_INTERN_CHIP,
            layer,
        )
        .unwrap();
    bus.routing
        .register(
            TaskKind::LEX_CLASSIFY,
            cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
            layer,
        )
        .unwrap();
    bus.routing
        .register(
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
            layer,
        )
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_TU, PA_TU_CHIP, layer)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(LxInternChip).unwrap();
    workers.register(LxClassifyChip).unwrap();
    workers.register(LxDecodeLiteralChip).unwrap();
    workers.register(PaTuChip).unwrap();
    workers
}

fn seed_source(bus: &mut CompilerBus) -> SourceId {
    let mut raw = b"int main(void){return 2+3;}".to_vec();
    raw.push(b'\n');
    let name = bus.intern_name(b"main.c").unwrap();
    bus.alloc_source(name, raw).unwrap()
}

fn seed_pp_layer(bus: &mut CompilerBus, source: SourceId) -> Vec<PpTokenId> {
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

/// Run the real LX chain, then return the bus with 13 committed tokens.
fn lex_chain() -> (CompilerBus, Vec<TokenId>) {
    let mut bus = new_bus();
    install_pa(&mut bus, 2);
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
    // Decode both integer literals (tokens 7 and 9).
    let pp_of = |bus: &CompilerBus, id: TokenId| bus.arenas.tokens.get(id).unwrap().pp_token;
    for index in [7, 9] {
        let token = tokens[index];
        let pp_token = pp_of(&bus, token);
        bootstrap(
            &mut bus,
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
            Payload::from_refs(vec![RecordRef::Token(token), RecordRef::PpToken(pp_token)]),
        );
        tick(&mut bus, &workers);
    }
    (bus, tokens)
}

fn parse_tu(bus: &mut CompilerBus, workers: &WorkerRegistry, tokens: &[TokenId]) -> TaskId {
    let task = bootstrap(
        bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(tokens.iter().map(|id| RecordRef::Token(*id)).collect()),
    );
    let report = tick(bus, workers);
    match report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.completed.len(), 1),
        other => panic!("expected executed, got {other:?}"),
    }
    task
}

#[test]
fn pa_kind_stage_and_allowlist_are_frozen() {
    assert_eq!(TaskKind::PARSE_TU.group(), TaskGroup::PARSE);
    assert_eq!(TaskKind::PARSE_TU.local(), 16);
    assert!(is_pa_slice_kind(TaskKind::PARSE_TU));
    assert!(!is_pa_slice_kind(TaskKind::CONTROL_NOOP));
    assert_eq!(stage_of(TaskKind::PARSE_TU), Some(2));
    let registry = TaskKindRegistry::pa_slice();
    assert_eq!(registry.len(), 12);
    assert_eq!(
        registry.lookup(TaskKind::PARSE_TU).unwrap().name,
        "parse.translation_unit"
    );
    assert!(STORE_OWNER_ALLOWLIST.contains(&(
        PA_TU_CHIP,
        cc_silicon_compiler::task::StoreId::Parse,
        "nodes",
        TaskKind::PARSE_TU
    )));
}

#[test]
fn pa_tu_commits_the_nine_node_m1_tree() {
    let (mut bus, tokens) = lex_chain();
    let workers = workers();
    parse_tu(&mut bus, &workers, &tokens);
    assert_eq!(bus.arenas.nodes.allocated(), 9);
    let node = |index: u32| {
        bus.arenas
            .nodes
            .get(cc_silicon_compiler::ids::NodeId::from_index(index))
            .unwrap()
            .clone()
    };
    // Kinds in pre-order.
    let kinds: Vec<NodeKind> = (0..9).map(|index| node(index).kind).collect();
    assert_eq!(
        kinds,
        vec![
            NodeKind::TranslationUnit,
            NodeKind::FunctionDefinition,
            NodeKind::Specifiers,
            NodeKind::Declarator,
            NodeKind::Compound,
            NodeKind::Return,
            NodeKind::BinaryAdd,
            NodeKind::IntLiteral,
            NodeKind::IntLiteral,
        ]
    );
    // Reciprocal parent/children coherence (RL-02 pinned by test).
    let children_of: Vec<Vec<u32>> = (0..9)
        .map(|index| node(index).children.iter().map(|id| id.index()).collect())
        .collect();
    assert_eq!(
        children_of,
        vec![
            vec![1],
            vec![2, 3, 4],
            vec![],
            vec![],
            vec![5],
            vec![6],
            vec![7, 8],
            vec![],
            vec![],
        ]
    );
    for (index, children) in children_of.iter().enumerate() {
        for child in children {
            assert_eq!(
                node(*child).parent.map(|id| id.index()),
                Some(index as u32),
                "child {child} must point back at parent {index}"
            );
        }
    }
    assert_eq!(node(0).parent, None);
    // Token ranges, declarator name, and literal leaves reference committed records.
    assert_eq!(
        (node(0).first_token, node(0).last_token),
        (tokens[0], tokens[12])
    );
    assert_eq!(
        (node(3).first_token, node(3).last_token),
        (tokens[1], tokens[4])
    );
    assert_eq!(
        (node(6).first_token, node(6).last_token),
        (tokens[7], tokens[9])
    );
    let main_name = bus.intern.lookup(b"main").unwrap();
    assert_eq!(node(3).name, Some(main_name));
    for (node_index, token_index) in [(7, 7), (8, 9)] {
        let leaf = node(node_index);
        assert_eq!(leaf.first_token, tokens[token_index]);
        assert_eq!(leaf.last_token, tokens[token_index]);
        let literal = bus.arenas.literals.get(leaf.literal.unwrap()).unwrap();
        assert_eq!(literal.token, Some(tokens[token_index]));
    }
    // Snapshot round-trips the new node bodies.
    let bytes = encode_node(&node(6));
    assert_eq!(decode_node(&bytes).unwrap(), node(6));
    let _ = Snapshot::capture(&bus);
}

#[test]
fn pa_tu_rejects_non_m1_shapes_missing_literals_and_dangling_tokens() {
    // Truncated token sequence.
    let (mut bus, tokens) = lex_chain();
    let workers = workers();
    bootstrap(
        &mut bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(
            tokens[..12]
                .iter()
                .map(|id| RecordRef::Token(*id))
                .collect(),
        ),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(bus.arenas.nodes.allocated(), 0);

    // Dangling token reference.
    let (mut bus, tokens) = lex_chain();
    let mut refs: Vec<RecordRef> = tokens.iter().map(|id| RecordRef::Token(*id)).collect();
    refs[0] = RecordRef::Token(TokenId::from_index(99));
    bootstrap(
        &mut bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(refs),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }

    // Missing committed literal for an integer token: remove the literal by
    // parsing before any decode ran.
    let mut bus = new_bus();
    install_pa(&mut bus, 2);
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
    let tokens = match report.outcome {
        TickOutcome::Executed { commit, .. } => commit
            .appended
            .iter()
            .map(|(_, reference)| match reference {
                RecordRef::Token(id) => *id,
                other => panic!("expected token ref, got {other:?}"),
            })
            .collect::<Vec<_>>(),
        other => panic!("expected executed, got {other:?}"),
    };
    bootstrap(
        &mut bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(tokens.iter().map(|id| RecordRef::Token(*id)).collect()),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(bus.arenas.nodes.allocated(), 0);
}

#[test]
fn pa_stage_layer_and_manifest_gates() {
    let (_bus, tokens) = lex_chain();
    // Reinstall on a fresh bus with a wrong layer: the driver refuses.
    let mut bus = new_bus();
    install_pa(&mut bus, 9);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let _ = (pp, source);
    // Reuse committed token IDs from the real chain (payload shape only;
    // the driver rejects on topology before reading them).
    let _ = tokens;
    let task = bootstrap(
        &mut bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(
            (0..13)
                .map(|index| RecordRef::Token(TokenId::from_index(index)))
                .collect(),
        ),
    );
    let workers = workers();
    let error = cc_silicon_compiler::chips::drive_task(&bus, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));

    // A manifest missing its write is rejected at registration.
    let mut manifest = PaTuChip.manifest();
    manifest.writes = vec![];
    assert!(matches!(
        bus.registrations.register(
            manifest,
            &StoreSchema::pa_slice(),
            &TaskKindRegistry::pa_slice()
        ),
        Err(ManifestRegistryError::Registry(_))
    ));
    let _ = FieldPath::new(cc_silicon_compiler::task::StoreId::Parse, "nodes");
}

#[test]
fn pa_snapshot_replay_is_deterministic() {
    let run = || {
        let (mut bus, tokens) = lex_chain();
        let workers = workers();
        parse_tu(&mut bus, &workers, &tokens);
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
