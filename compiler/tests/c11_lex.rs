// ============================================================================
// c11_lex.rs — Wave 2 (`/11`) LX tokenize/classify/decode slice acceptance.
//
// Covers the frozen closure: seeded PP tokens + spans, the `lex.intern`
// `lex.classify` / `lex.decode_literal` kinds, name interning (four M1
// spellings, NI-02 resolved), keyword classification against the frozen C11
// table, integer-only literal decode with the publish-time token back-link
// (DOC-10), kind-to-stage assignment, the LX store-owner allowlist rows,
// stage/layer agreement, snapshot bodies, tick-lifecycle integration, and
// the `/11` hash participation. PP-token/span fixtures are seeded as
// committed records (Host import and PP04 production stay future work);
// the `M1-LX-0x` rows they exercise are hand-derived, not full-M1 passes.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord, SpanRecord, TokenKind,
};
use cc_silicon_compiler::chips::{
    handler_for, is_keyword, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, Worker,
    WorkerRegistry,
};
use cc_silicon_compiler::commit::{commit_proposals, CommitError};
use cc_silicon_compiler::ids::{NameId, PpTokenId, RecordRef, SourceId, SpanId, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_lx_slice_kind, stage_of, FieldPath, ManifestRegistryError, StoreSchema, LX_CLASSIFY_CHIP,
    LX_DECODE_CHIP, LX_INTERN_CHIP, STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{decode_token, encode_token, Snapshot};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry,
    TaskState,
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

fn install_lx(bus: &mut CompilerBus, layer: u16) {
    bus.kinds = TaskKindRegistry::lx_slice();
    bus.schema = StoreSchema::lx_slice();
    for chip in [
        &LxInternChip as &dyn Worker,
        &LxClassifyChip,
        &LxDecodeLiteralChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(TaskKind::LEX_INTERN, LX_INTERN_CHIP, layer)
        .unwrap();
    bus.routing
        .register(TaskKind::LEX_CLASSIFY, LX_CLASSIFY_CHIP, layer)
        .unwrap();
    bus.routing
        .register(TaskKind::LEX_DECODE_LITERAL, LX_DECODE_CHIP, layer)
        .unwrap();
}

fn seed_source(bus: &mut CompilerBus) -> SourceId {
    let mut raw = b"int main(void){return 2+3;}".to_vec();
    raw.push(b'\n');
    assert_eq!(raw.len(), 28);
    let name = bus.intern_name(b"main.c").unwrap();
    bus.alloc_source(name, raw).unwrap()
}

/// Seed the 14 M1 spans + 13 PP tokens for `M1-SRC-000`.
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
    let tokens: Vec<(PpTokenKind, SpanId, &[u8])> = vec![
        (PpTokenKind::Identifier, span_ids[0], b"int".as_slice()),
        (PpTokenKind::Identifier, span_ids[1], b"main".as_slice()),
        (PpTokenKind::Punctuator, span_ids[2], b"(".as_slice()),
        (PpTokenKind::Identifier, span_ids[3], b"void".as_slice()),
        (PpTokenKind::Punctuator, span_ids[4], b")".as_slice()),
        (PpTokenKind::Punctuator, span_ids[5], b"{".as_slice()),
        (PpTokenKind::Identifier, span_ids[6], b"return".as_slice()),
        (PpTokenKind::PpNumber, span_ids[7], b"2".as_slice()),
        (PpTokenKind::Punctuator, span_ids[8], b"+".as_slice()),
        (PpTokenKind::PpNumber, span_ids[9], b"3".as_slice()),
        (PpTokenKind::Punctuator, span_ids[10], b";".as_slice()),
        (PpTokenKind::Punctuator, span_ids[11], b"}".as_slice()),
        (PpTokenKind::Eof, span_ids[12], b"".as_slice()),
    ];
    let mut ids = Vec::new();
    for (kind, span, spelling) in tokens {
        ids.push(
            bus.arenas
                .pp_tokens
                .alloc(
                    PpTokenRecord {
                        kind,
                        span,
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

fn pp_refs(ids: &[PpTokenId]) -> Payload {
    Payload::from_refs(ids.iter().map(|id| RecordRef::PpToken(*id)).collect())
}

fn run_tick(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
) -> cc_silicon_compiler::routing::TickReport {
    RoutingShell
        .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
        .unwrap()
}

fn full_chain() -> (CompilerBus, Vec<TokenId>) {
    let mut bus = new_bus();
    install_lx(&mut bus, 2);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let mut workers = WorkerRegistry::new();
    workers.register(LxInternChip).unwrap();
    workers.register(LxClassifyChip).unwrap();
    workers.register(LxDecodeLiteralChip).unwrap();
    bootstrap(&mut bus, TaskKind::LEX_INTERN, LX_INTERN_CHIP, pp_refs(&pp));
    run_tick(&mut bus, &workers);
    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        LX_CLASSIFY_CHIP,
        pp_refs(&pp),
    );
    let report = run_tick(&mut bus, &workers);
    let tokens = match report.outcome {
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
    (bus, tokens)
}

#[test]
fn lx_kinds_stages_and_allowlist_are_frozen() {
    assert_eq!(TaskKind::LEX_INTERN.group(), TaskGroup::LEX);
    assert_eq!(TaskKind::LEX_INTERN.local(), 16);
    assert_eq!(TaskKind::LEX_CLASSIFY.local(), 17);
    assert_eq!(TaskKind::LEX_DECODE_LITERAL.local(), 18);
    for kind in [
        TaskKind::LEX_INTERN,
        TaskKind::LEX_CLASSIFY,
        TaskKind::LEX_DECODE_LITERAL,
    ] {
        assert!(is_lx_slice_kind(kind));
        assert_eq!(stage_of(kind), Some(2));
    }
    let registry = TaskKindRegistry::lx_slice();
    assert_eq!(registry.len(), 11);
    for (kind, name) in [
        (TaskKind::LEX_INTERN, "lex.intern"),
        (TaskKind::LEX_CLASSIFY, "lex.classify"),
        (TaskKind::LEX_DECODE_LITERAL, "lex.decode_literal"),
    ] {
        assert_eq!(registry.lookup(kind).unwrap().name, name);
    }
    for row in [
        (
            LX_INTERN_CHIP,
            StoreId::Names,
            "entries",
            TaskKind::LEX_INTERN,
        ),
        (
            LX_CLASSIFY_CHIP,
            StoreId::Lex,
            "tokens",
            TaskKind::LEX_CLASSIFY,
        ),
        (
            LX_DECODE_CHIP,
            StoreId::Lex,
            "literals",
            TaskKind::LEX_DECODE_LITERAL,
        ),
    ] {
        assert!(STORE_OWNER_ALLOWLIST.contains(&row));
    }
}

#[test]
fn lx_keyword_table_is_membership_based() {
    assert!(is_keyword(b"int"));
    assert!(is_keyword(b"void"));
    assert!(is_keyword(b"return"));
    assert!(!is_keyword(b"main"));
    assert!(!is_keyword(b"INT"));
}

#[test]
fn lx_intern_produces_four_names_in_first_seen_order() {
    let mut bus = new_bus();
    install_lx(&mut bus, 2);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let mut workers = WorkerRegistry::new();
    workers.register(LxInternChip).unwrap();
    workers.register(LxClassifyChip).unwrap();
    workers.register(LxDecodeLiteralChip).unwrap();
    bootstrap(&mut bus, TaskKind::LEX_INTERN, LX_INTERN_CHIP, pp_refs(&pp));
    run_tick(&mut bus, &workers);
    // NI-02 resolved: identifier AND keyword spellings intern (PP has no
    // keyword distinction), first-seen order.
    for (spelling, index) in [
        (b"int".as_slice(), 1),
        (b"main".as_slice(), 2),
        (b"void".as_slice(), 3),
        (b"return".as_slice(), 4),
    ] {
        let id = bus.intern.lookup(spelling).unwrap();
        assert_eq!(id, NameId::from_index(index));
    }
    assert_eq!(bus.intern.len(), 5);
}

#[test]
fn lx_classify_produces_m1_token_kinds_in_order() {
    let (bus, tokens) = full_chain();
    assert_eq!(tokens.len(), 13);
    let kinds: Vec<TokenKind> = tokens
        .iter()
        .map(|id| bus.arenas.tokens.get(*id).unwrap().kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Keyword,
            TokenKind::Identifier,
            TokenKind::Punctuator,
            TokenKind::Keyword,
            TokenKind::Punctuator,
            TokenKind::Punctuator,
            TokenKind::Keyword,
            TokenKind::Integer,
            TokenKind::Punctuator,
            TokenKind::Integer,
            TokenKind::Punctuator,
            TokenKind::Punctuator,
            TokenKind::Eof,
        ]
    );
    // Names resolve to the interned spellings; spans reuse committed PP spans.
    let record = bus.arenas.tokens.get(tokens[1]).unwrap();
    let spelling = bus.intern.get(record.name.unwrap()).unwrap();
    assert_eq!(spelling, b"main");
    assert_eq!(bus.arenas.tokens.get(tokens[7]).unwrap().name, None);
    // Snapshot round-trips the new token bodies.
    let bytes = encode_token(record);
    assert_eq!(&decode_token(&bytes).unwrap(), record);
    let _ = Snapshot::capture(&bus);
}

#[test]
fn lx_decode_commits_literals_with_token_back_links() {
    let (mut bus, tokens) = full_chain();
    let pp_ids: Vec<PpTokenId> = tokens
        .iter()
        .map(|id| bus.arenas.tokens.get(*id).unwrap().pp_token)
        .collect();
    let mut workers = WorkerRegistry::new();
    workers.register(LxInternChip).unwrap();
    workers.register(LxClassifyChip).unwrap();
    workers.register(LxDecodeLiteralChip).unwrap();
    for (token_index, spelling) in [(7, b"2".as_slice()), (9, b"3".as_slice())] {
        bootstrap(
            &mut bus,
            TaskKind::LEX_DECODE_LITERAL,
            LX_DECODE_CHIP,
            Payload::from_refs(vec![
                RecordRef::Token(tokens[token_index]),
                RecordRef::PpToken(pp_ids[token_index]),
            ]),
        );
        let report = run_tick(&mut bus, &workers);
        match report.outcome {
            TickOutcome::Executed { commit, .. } => assert_eq!(commit.completed.len(), 1),
            other => panic!("expected executed, got {other:?}"),
        }
        let _ = spelling;
    }
    assert_eq!(bus.arenas.literals.allocated(), 2);
    for (literal_index, token_index, value) in [(0, 7, vec![2]), (1, 9, vec![3])] {
        let literal = bus
            .arenas
            .literals
            .get(cc_silicon_compiler::ids::LiteralId::from_index(
                literal_index,
            ))
            .unwrap();
        // DOC-10 publish-time back-link: the committed token of the same flow.
        assert_eq!(
            literal.token,
            Some(tokens[token_index]),
            "literal {literal_index} must point at its committed token"
        );
        assert_eq!(literal.value, value);
        assert_eq!(
            literal.candidate_type,
            cc_silicon_compiler::snapshot::Lx08CandidateType::Int
        );
    }
}

#[test]
fn lx_rejects_dangling_noninteger_and_hex_inputs() {
    // Dangling PP token in classify.
    let mut bus = new_bus();
    install_lx(&mut bus, 2);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let mut workers = WorkerRegistry::new();
    workers.register(LxInternChip).unwrap();
    workers.register(LxClassifyChip).unwrap();
    workers.register(LxDecodeLiteralChip).unwrap();
    bootstrap(&mut bus, TaskKind::LEX_INTERN, LX_INTERN_CHIP, pp_refs(&pp));
    run_tick(&mut bus, &workers);
    let mut bad_refs = pp_refs(&pp);
    bad_refs
        .refs
        .push(RecordRef::PpToken(PpTokenId::from_index(99)));
    bootstrap(&mut bus, TaskKind::LEX_CLASSIFY, LX_CLASSIFY_CHIP, bad_refs);
    match run_tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }

    // Hex spelling is explicit unsupported at classify.
    let mut bus = new_bus();
    install_lx(&mut bus, 2);
    let source = seed_source(&mut bus);
    let limits = bus.limits();
    let span = bus
        .arenas
        .spans
        .alloc(
            SpanRecord {
                source,
                start: 0,
                end: 4,
                expansion: None,
            },
            &limits,
        )
        .unwrap();
    let hex = bus
        .arenas
        .pp_tokens
        .alloc(
            cc_silicon_compiler::bus::PpTokenRecord {
                kind: PpTokenKind::PpNumber,
                span,
                spelling: b"0x10".to_vec(),
            },
            &limits,
        )
        .unwrap();
    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        LX_CLASSIFY_CHIP,
        Payload::from_refs(vec![RecordRef::PpToken(hex)]),
    );
    match run_tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }

    // Decode of a non-integer token fails loudly.
    let (mut bus, tokens) = full_chain();
    let pp_of = bus.arenas.tokens.get(tokens[0]).unwrap().pp_token;
    bootstrap(
        &mut bus,
        TaskKind::LEX_DECODE_LITERAL,
        LX_DECODE_CHIP,
        Payload::from_refs(vec![RecordRef::Token(tokens[0]), RecordRef::PpToken(pp_of)]),
    );
    match run_tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }

    // Non-canonical draft indexes and duplicate transitions stay rejected.
    let mut bus = new_bus();
    install_lx(&mut bus, 2);
    let _source = seed_source(&mut bus);
    let task = bootstrap(
        &mut bus,
        TaskKind::LEX_INTERN,
        LX_INTERN_CHIP,
        Payload::empty(),
    );
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    bus.tasks.ready.clear();
    let error = commit_proposals(
        &mut bus,
        vec![
            cc_silicon_compiler::bus::TaggedProposal {
                chip: LX_INTERN_CHIP,
                task,
                proposal: Proposal::AppendRecords {
                    task,
                    batch: cc_silicon_compiler::task::AppendBatch {
                        records: vec![cc_silicon_compiler::records::RecordDraft {
                            family: cc_silicon_compiler::ids::RecordFamily::Name,
                            index: cc_silicon_compiler::task::DraftRef(42),
                        }],
                        bodies: vec![cc_silicon_compiler::records::G1DraftBody::Name {
                            spelling: b"x".to_vec(),
                        }],
                    },
                },
            },
            cc_silicon_compiler::bus::TaggedProposal {
                chip: LX_INTERN_CHIP,
                task,
                proposal: Proposal::Complete {
                    task,
                    value: ResultValue::Ack,
                },
            },
        ],
    )
    .unwrap_err();
    assert!(matches!(error, CommitError::InvalidPatchShape { .. }));
}

#[test]
fn lx_stage_layer_and_manifest_gates() {
    let mut bus = new_bus();
    install_lx(&mut bus, 9);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let _ = source;
    let task = bootstrap(&mut bus, TaskKind::LEX_INTERN, LX_INTERN_CHIP, pp_refs(&pp));
    let mut workers = WorkerRegistry::new();
    workers.register(LxInternChip).unwrap();
    workers.register(LxClassifyChip).unwrap();
    workers.register(LxDecodeLiteralChip).unwrap();
    let error = cc_silicon_compiler::chips::drive_task(&bus, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));

    // A manifest missing its write is rejected at registration.
    let mut manifest = LxClassifyChip.manifest();
    manifest.writes = vec![];
    assert!(matches!(
        bus.registrations.register(
            manifest,
            &StoreSchema::lx_slice(),
            &TaskKindRegistry::lx_slice()
        ),
        Err(ManifestRegistryError::Registry(_))
    ));
    let _ = FieldPath::new(StoreId::Lex, "tokens");
}

#[test]
fn lx_snapshot_replay_is_deterministic() {
    let run = || {
        let (bus, _) = full_chain();
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
