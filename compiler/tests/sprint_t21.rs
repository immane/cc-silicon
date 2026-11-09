// ============================================================================
// sprint_t21.rs — T21 adversarial parse-shape regression.
//
// Focus: malformed delimiters, malformed/missing EOF shapes, trailing tokens
// after a complete body or after EOF, and the absence of a permissive parse
// fallback. The valid controls are hand-derived; every malformed stream must
// fail in one tick with exactly one `Unsupported` diagnostic and zero AST
// nodes, proving that no prefix or fixed-shape tree was committed.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, NodeKind, PpTokenKind, PpTokenRecord, SpanRecord, TokenKind,
};
use cc_silicon_compiler::chips::{
    handler_for, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip, Worker,
    WorkerRegistry,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{ChipId, NodeId, PpTokenId, RecordRef, SourceId, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    StoreSchema, LX_CLASSIFY_CHIP, LX_DECODE_CHIP, LX_INTERN_CHIP, PA_TU_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome, TickReport};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, TaskDraft, TaskKind, TaskKindRegistry};

type RawToken = (PpTokenKind, &'static [u8]);

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_pa(bus: &mut CompilerBus) {
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
    for (kind, owner) in [
        (TaskKind::LEX_INTERN, LX_INTERN_CHIP),
        (TaskKind::LEX_CLASSIFY, LX_CLASSIFY_CHIP),
        (TaskKind::LEX_DECODE_LITERAL, LX_DECODE_CHIP),
        (TaskKind::PARSE_TU, PA_TU_CHIP),
    ] {
        bus.routing.register(kind, owner, 2).unwrap();
    }
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(LxInternChip).unwrap();
    workers.register(LxClassifyChip).unwrap();
    workers.register(LxDecodeLiteralChip).unwrap();
    workers.register(PaTuChip).unwrap();
    workers
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

fn tick(bus: &mut CompilerBus, workers: &WorkerRegistry) -> TickReport {
    RoutingShell
        .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
        .unwrap()
}

/// Canonical `/12` M1 stream: `int main(void){return 2+3;}` plus EOF.
fn canonical_tokens() -> Vec<RawToken> {
    vec![
        (PpTokenKind::Identifier, b"int"),
        (PpTokenKind::Identifier, b"main"),
        (PpTokenKind::Punctuator, b"("),
        (PpTokenKind::Identifier, b"void"),
        (PpTokenKind::Punctuator, b")"),
        (PpTokenKind::Punctuator, b"{"),
        (PpTokenKind::Identifier, b"return"),
        (PpTokenKind::PpNumber, b"2"),
        (PpTokenKind::Punctuator, b"+"),
        (PpTokenKind::PpNumber, b"3"),
        (PpTokenKind::Punctuator, b";"),
        (PpTokenKind::Punctuator, b"}"),
        (PpTokenKind::Eof, b""),
    ]
}

/// Valid `/55` call stream:
/// `int inc(int x) { return x + 1; } int main(void) { return inc(2); }`.
///
/// Token indexes used by the adversarial replacements:
/// 2/5 parameter parens, 6/12 body braces, 11 statement semicolon,
/// 18 main body open, 23 call close paren, 26 trailing EOF.
fn call_tokens() -> Vec<RawToken> {
    vec![
        (PpTokenKind::Identifier, b"int"),
        (PpTokenKind::Identifier, b"inc"),
        (PpTokenKind::Punctuator, b"("),
        (PpTokenKind::Identifier, b"int"),
        (PpTokenKind::Identifier, b"x"),
        (PpTokenKind::Punctuator, b")"),
        (PpTokenKind::Punctuator, b"{"),
        (PpTokenKind::Identifier, b"return"),
        (PpTokenKind::Identifier, b"x"),
        (PpTokenKind::Punctuator, b"+"),
        (PpTokenKind::PpNumber, b"1"),
        (PpTokenKind::Punctuator, b";"),
        (PpTokenKind::Punctuator, b"}"),
        (PpTokenKind::Identifier, b"int"),
        (PpTokenKind::Identifier, b"main"),
        (PpTokenKind::Punctuator, b"("),
        (PpTokenKind::Identifier, b"void"),
        (PpTokenKind::Punctuator, b")"),
        (PpTokenKind::Punctuator, b"{"),
        (PpTokenKind::Identifier, b"return"),
        (PpTokenKind::Identifier, b"inc"),
        (PpTokenKind::Punctuator, b"("),
        (PpTokenKind::PpNumber, b"2"),
        (PpTokenKind::Punctuator, b")"),
        (PpTokenKind::Punctuator, b";"),
        (PpTokenKind::Punctuator, b"}"),
        (PpTokenKind::Eof, b""),
    ]
}

/// Valid zero-argument control:
/// `int one(void) { return 7; } int main(void) { return one(); }`.
fn one_tokens() -> Vec<RawToken> {
    vec![
        (PpTokenKind::Identifier, b"int"),
        (PpTokenKind::Identifier, b"one"),
        (PpTokenKind::Punctuator, b"("),
        (PpTokenKind::Identifier, b"void"),
        (PpTokenKind::Punctuator, b")"),
        (PpTokenKind::Punctuator, b"{"),
        (PpTokenKind::Identifier, b"return"),
        (PpTokenKind::PpNumber, b"7"),
        (PpTokenKind::Punctuator, b";"),
        (PpTokenKind::Punctuator, b"}"),
        (PpTokenKind::Identifier, b"int"),
        (PpTokenKind::Identifier, b"main"),
        (PpTokenKind::Punctuator, b"("),
        (PpTokenKind::Identifier, b"void"),
        (PpTokenKind::Punctuator, b")"),
        (PpTokenKind::Punctuator, b"{"),
        (PpTokenKind::Identifier, b"return"),
        (PpTokenKind::Identifier, b"one"),
        (PpTokenKind::Punctuator, b"("),
        (PpTokenKind::Punctuator, b")"),
        (PpTokenKind::Punctuator, b";"),
        (PpTokenKind::Punctuator, b"}"),
        (PpTokenKind::Eof, b""),
    ]
}

fn replace(
    tokens: &[RawToken],
    position: usize,
    kind: PpTokenKind,
    spelling: &'static [u8],
) -> Vec<RawToken> {
    let mut out = tokens.to_vec();
    out[position] = (kind, spelling);
    out
}

fn insert(
    tokens: &[RawToken],
    position: usize,
    kind: PpTokenKind,
    spelling: &'static [u8],
) -> Vec<RawToken> {
    let mut out = tokens.to_vec();
    out.insert(position, (kind, spelling));
    out
}

fn remove(tokens: &[RawToken], position: usize) -> Vec<RawToken> {
    let mut out = tokens.to_vec();
    out.remove(position);
    out
}

fn push_token(tokens: &[RawToken], kind: PpTokenKind, spelling: &'static [u8]) -> Vec<RawToken> {
    let mut out = tokens.to_vec();
    out.push((kind, spelling));
    out
}

fn seed_source(bus: &mut CompilerBus, tokens: &[RawToken]) -> SourceId {
    let mut raw = Vec::new();
    for (index, (_, spelling)) in tokens.iter().enumerate() {
        if index > 0 {
            raw.push(b' ');
        }
        raw.extend_from_slice(spelling);
    }
    raw.push(b'\n');
    let name = bus.intern_name(b"t21.c").unwrap();
    bus.alloc_source(name, raw).unwrap()
}

fn seed_pp_layer(bus: &mut CompilerBus, source: SourceId, tokens: &[RawToken]) -> Vec<PpTokenId> {
    let limits = bus.limits();
    let mut offset = 0u64;
    let mut ids = Vec::new();
    for (kind, spelling) in tokens {
        let start = offset;
        let end = start + spelling.len() as u64;
        offset = end + 1;
        let span = bus
            .arenas
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
            .unwrap();
        ids.push(
            bus.arenas
                .pp_tokens
                .alloc(
                    PpTokenRecord {
                        kind: *kind,
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

/// Run the real LX intern/classify/decode chain, then return committed tokens.
fn lex_chain(tokens: &[RawToken]) -> (CompilerBus, Vec<TokenId>) {
    let mut bus = new_bus();
    install_pa(&mut bus);
    let source = seed_source(&mut bus, tokens);
    let pp = seed_pp_layer(&mut bus, source, tokens);
    let workers = workers();
    let refs = || {
        pp.iter()
            .map(|id| RecordRef::PpToken(*id))
            .collect::<Vec<_>>()
    };

    bootstrap(
        &mut bus,
        TaskKind::LEX_INTERN,
        LX_INTERN_CHIP,
        Payload::from_refs(refs()),
    );
    let _ = tick(&mut bus, &workers);
    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        LX_CLASSIFY_CHIP,
        Payload::from_refs(refs()),
    );
    let report = tick(&mut bus, &workers);
    let token_ids: Vec<TokenId> = match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert!(commit.failed.is_empty(), "fixture lex/classify failed");
            commit
                .appended
                .iter()
                .map(|(_, reference)| match reference {
                    RecordRef::Token(id) => *id,
                    other => panic!("expected token ref, got {other:?}"),
                })
                .collect()
        }
        other => panic!("expected lex execution, got {other:?}"),
    };

    let mut integers = Vec::new();
    for token in &token_ids {
        if bus.arenas.tokens.get(*token).unwrap().kind == TokenKind::Integer {
            integers.push(*token);
        }
    }
    for token in integers {
        let pp_token = bus.arenas.tokens.get(token).unwrap().pp_token;
        bootstrap(
            &mut bus,
            TaskKind::LEX_DECODE_LITERAL,
            LX_DECODE_CHIP,
            Payload::from_refs(vec![RecordRef::Token(token), RecordRef::PpToken(pp_token)]),
        );
        let _ = tick(&mut bus, &workers);
    }
    (bus, token_ids)
}

fn parse_tu(bus: &mut CompilerBus, tokens: &[TokenId]) -> TickReport {
    bootstrap(
        bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(tokens.iter().map(|id| RecordRef::Token(*id)).collect()),
    );
    tick(bus, &workers())
}

fn parse_ok(label: &str, tokens: &[RawToken]) -> CompilerBus {
    let (mut bus, ids) = lex_chain(tokens);
    let report = parse_tu(&mut bus, &ids);
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert!(commit.failed.is_empty(), "{label}");
            assert_eq!(commit.completed.len(), 1, "{label}");
            assert!(!commit.appended.is_empty(), "{label}");
        }
        other => panic!("{label}: expected parse execution, got {other:?}"),
    }
    assert!(bus.arenas.nodes.allocated() > 0, "{label}");
    bus
}

fn assert_rejected(label: &str, tokens: &[RawToken]) {
    let (mut bus, ids) = lex_chain(tokens);
    let report = parse_tu(&mut bus, &ids);
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 1, "{label}");
            assert!(commit.completed.is_empty(), "{label}");
            assert!(commit.appended.is_empty(), "{label}");
            let diagnostic = bus.arenas.diagnostics.get(commit.failed[0].1).unwrap();
            assert_eq!(diagnostic.code.group, DiagGroup::Unsupported, "{label}");
        }
        other => panic!("{label}: expected executed failure, got {other:?}"),
    }
    assert_eq!(bus.arenas.nodes.allocated(), 0, "{label}");
}

#[test]
fn valid_canonical_and_general_streams_still_parse() {
    let canonical = parse_ok("canonical", &canonical_tokens());
    assert_eq!(canonical.arenas.nodes.allocated(), 9);
    let kinds: Vec<NodeKind> = (0..9)
        .map(|index| {
            canonical
                .arenas
                .nodes
                .get(NodeId::from_index(index))
                .unwrap()
                .kind
        })
        .collect();
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

    let general = parse_ok("general", &one_tokens());
    assert_eq!(general.arenas.nodes.allocated(), 14);
    let kinds: Vec<NodeKind> = (0..14)
        .map(|index| {
            general
                .arenas
                .nodes
                .get(NodeId::from_index(index))
                .unwrap()
                .kind
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            NodeKind::TranslationUnit,
            NodeKind::FunctionDefinition,
            NodeKind::Specifiers,
            NodeKind::Declarator,
            NodeKind::Compound,
            NodeKind::Return,
            NodeKind::IntLiteral,
            NodeKind::FunctionDefinition,
            NodeKind::Specifiers,
            NodeKind::Declarator,
            NodeKind::Compound,
            NodeKind::Return,
            NodeKind::Call,
            NodeKind::Identifier,
        ]
    );
}

#[test]
fn malformed_delimiters_fail_atomically() {
    let call = call_tokens();
    let canonical = canonical_tokens();
    let cases: Vec<(&str, Vec<RawToken>)> = vec![
        (
            "parameter_open_swapped",
            replace(&call, 2, PpTokenKind::Punctuator, b"["),
        ),
        (
            "parameter_close_swapped",
            replace(&call, 5, PpTokenKind::Punctuator, b"]"),
        ),
        (
            "body_open_swapped",
            replace(&call, 6, PpTokenKind::Punctuator, b"("),
        ),
        (
            "statement_semicolon_swapped",
            replace(&call, 11, PpTokenKind::Punctuator, b":"),
        ),
        (
            "body_close_swapped",
            replace(&call, 12, PpTokenKind::Punctuator, b")"),
        ),
        (
            "second_body_open_swapped",
            replace(&call, 18, PpTokenKind::Punctuator, b"["),
        ),
        (
            "argument_close_swapped",
            replace(&call, 23, PpTokenKind::Punctuator, b"]"),
        ),
        ("parameter_open_missing", remove(&call, 2)),
        ("statement_semicolon_missing", remove(&call, 11)),
        ("body_close_missing", remove(&call, 12)),
        (
            "canonical_close_brace_swapped",
            replace(&canonical, 11, PpTokenKind::Punctuator, b")"),
        ),
        (
            "canonical_open_paren_swapped",
            replace(&canonical, 2, PpTokenKind::Punctuator, b"["),
        ),
        ("canonical_close_brace_missing", remove(&canonical, 11)),
    ];
    for (label, tokens) in cases {
        assert_rejected(label, &tokens);
    }
}

#[test]
fn eof_shape_variants_fail_without_partial_tree() {
    let call = call_tokens();
    let canonical = canonical_tokens();
    let cases: Vec<(&str, Vec<RawToken>)> = vec![
        ("missing_eof", remove(&call, 26)),
        (
            "canonical_eof_with_spelling",
            replace(&canonical, 12, PpTokenKind::Eof, b"junk"),
        ),
        (
            "eof_with_spelling",
            replace(&call, 26, PpTokenKind::Eof, b"junk"),
        ),
        ("double_eof", insert(&call, 26, PpTokenKind::Eof, b"")),
        (
            "token_after_eof",
            push_token(&call, PpTokenKind::Punctuator, b";"),
        ),
        (
            "eof_before_second_function",
            replace(&call, 13, PpTokenKind::Eof, b""),
        ),
        (
            "eof_replaced_by_identifier",
            replace(&call, 26, PpTokenKind::Identifier, b"end"),
        ),
        ("eof_only", vec![(PpTokenKind::Eof, b"")]),
        ("empty_stream", Vec::new()),
    ];
    for (label, tokens) in cases {
        assert_rejected(label, &tokens);
    }
}

#[test]
fn trailing_tokens_and_no_parse_fallback() {
    let call = call_tokens();
    let canonical = canonical_tokens();
    let cases: Vec<(&str, Vec<RawToken>)> = vec![
        (
            "canonical_junk_before_eof",
            insert(&canonical, 12, PpTokenKind::Identifier, b"junk"),
        ),
        (
            "canonical_semicolon_before_eof",
            insert(&canonical, 12, PpTokenKind::Punctuator, b";"),
        ),
        (
            "canonical_semicolon_after_eof",
            push_token(&canonical, PpTokenKind::Punctuator, b";"),
        ),
        (
            "canonical_second_eof",
            insert(&canonical, 12, PpTokenKind::Eof, b""),
        ),
        (
            "call_junk_before_eof",
            insert(&call, 26, PpTokenKind::Identifier, b"junk"),
        ),
        (
            "call_incomplete_function_before_eof",
            insert(&call, 26, PpTokenKind::Identifier, b"int"),
        ),
        (
            "call_junk_after_eof",
            push_token(&call, PpTokenKind::Identifier, b"junk"),
        ),
    ];
    for (label, tokens) in cases {
        assert_rejected(label, &tokens);
    }
}
