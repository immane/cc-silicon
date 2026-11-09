// ============================================================================
// c40_pa_guard.rs — B1-01 PA01 exact-shape gate regression.
//
// Covers the D-01 wrong-acceptance defect: PA01 must validate every fixed
// M1 punctuator spelling (`(` 2, `)` 4, `{` 5, `+` 8, `;` 10, `}` 11) and the
// EOF token shape, not only token kinds and the four keyword/identifier
// spellings. Non-M1 operators (`*`, `-`), a unary-plus shape (`2 + +3`),
// malformed delimiters, and wrong word spellings must fail as typed
// `Unsupported` diagnostics before any AST node is appended. The canonical
// M1 source remains the only accepted shape and still appends the nine-node
// tree. A final probe runs the built candidate binary end to end for the
// three D-01 sources without touching `candidate.rs`.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, NodeKind, PpTokenKind, PpTokenRecord, SpanRecord, TokenKind,
};
use cc_silicon_compiler::chips::{
    handler_for, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip, Worker,
    WorkerRegistry,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{NodeId, PpTokenId, RecordRef, SourceId, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    StoreSchema, LX_CLASSIFY_CHIP, LX_DECODE_CHIP, LX_INTERN_CHIP, PA_TU_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome, TickReport};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, ResultValue, TaskDraft, TaskKind, TaskKindRegistry};

/// One PA01 rejection case: label plus the replacement M1 token stream.
type GuardCase = (&'static str, Vec<(PpTokenKind, &'static [u8])>);
use std::path::{Path, PathBuf};
use std::process::Command;

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
        .register(TaskKind::LEX_INTERN, LX_INTERN_CHIP, layer)
        .unwrap();
    bus.routing
        .register(TaskKind::LEX_CLASSIFY, LX_CLASSIFY_CHIP, layer)
        .unwrap();
    bus.routing
        .register(TaskKind::LEX_DECODE_LITERAL, LX_DECODE_CHIP, layer)
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

/// The canonical 13-token M1 stream, with PP-level spellings.
fn m1_tokens() -> Vec<(PpTokenKind, &'static [u8])> {
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

fn m1_with(
    replacements: &[(usize, PpTokenKind, &'static [u8])],
) -> Vec<(PpTokenKind, &'static [u8])> {
    let mut tokens = m1_tokens();
    for (position, kind, spelling) in replacements {
        tokens[*position] = (*kind, spelling);
    }
    tokens
}

/// The M1 source made of the given token spellings (one space between
/// tokens), used only to keep seeded spans in bounds.
fn seed_source(bus: &mut CompilerBus, tokens: &[(PpTokenKind, &[u8])]) -> SourceId {
    let mut raw = Vec::new();
    for (index, (_, spelling)) in tokens.iter().enumerate() {
        if index > 0 {
            raw.push(b' ');
        }
        raw.extend_from_slice(spelling);
    }
    raw.push(b'\n');
    let name = bus.intern_name(b"guard.c").unwrap();
    bus.alloc_source(name, raw).unwrap()
}

fn seed_pp_layer(
    bus: &mut CompilerBus,
    source: SourceId,
    tokens: &[(PpTokenKind, &[u8])],
) -> Vec<PpTokenId> {
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

/// Run the real LX chain over the given PP token spellings and decode every
/// integer token, returning 13+ committed tokens for PA01.
fn lex_chain(tokens: &[(PpTokenKind, &[u8])]) -> (CompilerBus, Vec<TokenId>) {
    let mut bus = new_bus();
    install_pa(&mut bus, 2);
    let source = seed_source(&mut bus, tokens);
    let pp = seed_pp_layer(&mut bus, source, tokens);
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
    let token_ids: Vec<TokenId> = match report.outcome {
        TickOutcome::Executed { commit, .. } => commit
            .appended
            .iter()
            .map(|(_, reference)| match reference {
                RecordRef::Token(id) => *id,
                other => panic!("expected token ref, got {other:?}"),
            })
            .collect(),
        other => panic!("expected classify execution, got {other:?}"),
    };
    let integers: Vec<TokenId> = token_ids
        .iter()
        .copied()
        .filter(|id| bus.arenas.tokens.get(*id).unwrap().kind == TokenKind::Integer)
        .collect();
    for token in integers {
        let pp_token = bus.arenas.tokens.get(token).unwrap().pp_token;
        bootstrap(
            &mut bus,
            TaskKind::LEX_DECODE_LITERAL,
            LX_DECODE_CHIP,
            Payload::from_refs(vec![RecordRef::Token(token), RecordRef::PpToken(pp_token)]),
        );
        tick(&mut bus, &workers);
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

fn assert_unsupported_without_ast(report: &TickReport, bus: &CompilerBus, label: &str) {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 1, "{label}");
            assert!(commit.completed.is_empty(), "{label}");
            assert!(commit.appended.is_empty(), "{label}");
            let diagnostic = bus.arenas.diagnostics.get(commit.failed[0].1).unwrap();
            assert_eq!(diagnostic.code.group, DiagGroup::Unsupported, "{label}");
        }
        other => panic!("{label}: expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(bus.arenas.nodes.allocated(), 0, "{label}");
}

#[test]
fn canonical_m1_is_accepted_and_appends_the_tree() {
    let (mut bus, tokens) = lex_chain(&m1_tokens());
    let report = parse_tu(&mut bus, &tokens);
    let commit = match &report.outcome {
        TickOutcome::Executed { commit, .. } => commit,
        other => panic!("expected executed, got {other:?}"),
    };
    assert!(commit.failed.is_empty());
    assert_eq!(commit.completed.len(), 1);
    assert_eq!(commit.appended.len(), 9);
    let result = bus.arenas.results.get(commit.completed[0].1).unwrap();
    assert_eq!(
        result.value,
        ResultValue::Record(RecordRef::Node(NodeId::from_index(0)))
    );
    assert_eq!(bus.arenas.nodes.allocated(), 9);
    let kinds: Vec<NodeKind> = (0..9)
        .map(|index| {
            bus.arenas
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
    let binary = bus.arenas.nodes.get(NodeId::from_index(6)).unwrap();
    assert_eq!(
        (binary.first_token, binary.last_token),
        (tokens[7], tokens[9])
    );
}

#[test]
fn non_m1_punctuators_and_eof_shape_are_unsupported() {
    let cases: Vec<GuardCase> = vec![
        ("mul", m1_with(&[(8, PpTokenKind::Punctuator, b"*")])),
        ("sub", m1_with(&[(8, PpTokenKind::Punctuator, b"-")])),
        ("unary_plus", {
            let mut tokens = m1_tokens();
            tokens.insert(9, (PpTokenKind::Punctuator, b"+"));
            tokens
        }),
        ("open_paren", m1_with(&[(2, PpTokenKind::Punctuator, b"[")])),
        (
            "close_paren",
            m1_with(&[(4, PpTokenKind::Punctuator, b"]")]),
        ),
        ("open_brace", m1_with(&[(5, PpTokenKind::Punctuator, b"(")])),
        ("semicolon", m1_with(&[(10, PpTokenKind::Punctuator, b":")])),
        (
            "close_brace",
            m1_with(&[(11, PpTokenKind::Punctuator, b")")]),
        ),
        ("eof_spelling", m1_with(&[(12, PpTokenKind::Eof, b"junk")])),
        (
            "malformed_delimiters",
            m1_with(&[
                (4, PpTokenKind::Punctuator, b"]"),
                (8, PpTokenKind::Punctuator, b"*"),
                (11, PpTokenKind::Punctuator, b")"),
            ]),
        ),
    ];
    for (label, tokens) in cases {
        let (mut bus, token_ids) = lex_chain(&tokens);
        let report = parse_tu(&mut bus, &token_ids);
        assert_unsupported_without_ast(&report, &bus, label);
    }
}

#[test]
fn wrong_word_spellings_are_unsupported() {
    let cases: Vec<GuardCase> = vec![
        ("int", m1_with(&[(0, PpTokenKind::Identifier, b"float")])),
        ("main", m1_with(&[(1, PpTokenKind::Identifier, b"mian")])),
        ("void", m1_with(&[(3, PpTokenKind::Identifier, b"char")])),
        ("return", m1_with(&[(6, PpTokenKind::Identifier, b"break")])),
    ];
    for (label, tokens) in cases {
        let (mut bus, token_ids) = lex_chain(&tokens);
        let report = parse_tu(&mut bus, &token_ids);
        assert_unsupported_without_ast(&report, &bus, label);
    }
}

fn candidate() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "candidate-pa-guard-{}-{}-{}",
        std::process::id(),
        test,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn write_source(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("source write");
    path
}

#[test]
fn candidate_rejects_d01_sources_end_to_end() {
    let dir = workdir("d01");
    let bad: &[(&str, &[u8])] = &[
        ("mul.c", b"int main(void){return 2*3;}\n"),
        ("sub.c", b"int main(void){return 2-3;}\n"),
        ("unary.c", b"int main(void){return 2 + +3;}\n"),
        ("delim.c", b"int main(void]{return 2*3;)\n"),
    ];
    for (name, source) in bad {
        let path = write_source(&dir, name, source);
        let output = Command::new(candidate())
            .arg(&path)
            .output()
            .expect("candidate runs");
        assert_eq!(output.status.code(), Some(1), "{name}");
        let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
        assert!(!stdout.contains("interpret: "), "{name}");
        assert!(!stdout.contains("snapshot: "), "{name}");
        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        assert!(
            stderr.contains("candidate diagnostic: "),
            "{name}: {stderr}"
        );
    }
    let canonical = write_source(&dir, "canonical.c", b"int main(void){return 2+3;}\n");
    let output = Command::new(candidate())
        .arg(&canonical)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("interpret: value=05 negative=false"));
}
