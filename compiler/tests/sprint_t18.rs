// ============================================================================
// sprint_t18.rs — T18 function-count boundary with independent AST checks.
//
// The `/55` contract accepts at most 32 function definitions per translation
// unit and rejects the next value without appending a partial AST. This test
// drives the real LX chain and general parser over 32 and 33 distinct
// `int fN(void){return N;}` definitions, then hand-verifies every node index,
// parent, child list, token span, declarator name, and literal magnitude of
// the accepted tree before asking the VF05 invariant verifier for an
// independent pass. The 33-function case must fail before any node append.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, NodeKind, PpTokenKind, PpTokenRecord, SpanRecord, TokenKind,
};
use cc_silicon_compiler::chips::{
    handler_for, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip, Vf05Chip, Worker,
    WorkerRegistry,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{NodeId, PpTokenId, RecordRef, SourceId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    StoreSchema, LX_CLASSIFY_CHIP, LX_DECODE_CHIP, LX_INTERN_CHIP, PA_TU_CHIP, VF05_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome, TickReport};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, ResultValue, TaskDraft, TaskKind, TaskKindRegistry};

const ACCEPTED_FUNCTIONS: usize = 32;
const REJECTED_FUNCTIONS: usize = 33;
/// `int NAME ( void ) { return N ; }` contributes ten tokens.
const TOKENS_PER_FUNCTION: usize = 10;
/// FunctionDefinition, Specifiers, Declarator, Compound, Return, IntLiteral.
const NODES_PER_FUNCTION: usize = 6;

#[derive(Clone, Debug)]
struct FixtureToken {
    kind: PpTokenKind,
    spelling: Vec<u8>,
}

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_vf05(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::vf05_slice();
    bus.schema = StoreSchema::pp_slice();
    for chip in [
        &LxInternChip as &dyn Worker,
        &LxClassifyChip,
        &LxDecodeLiteralChip,
        &PaTuChip,
        &Vf05Chip,
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
        (TaskKind::VERIFICATION_TOKEN_AST_INVARIANT, VF05_CHIP),
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
    workers.register(Vf05Chip).unwrap();
    workers
}

fn bootstrap(
    bus: &mut CompilerBus,
    kind: TaskKind,
    owner: cc_silicon_compiler::ids::ChipId,
    payload: Payload,
) -> cc_silicon_compiler::ids::TaskId {
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

fn expect_executed<'a>(
    report: &'a TickReport,
    label: &str,
) -> &'a cc_silicon_compiler::commit::CommitReport {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert!(commit.failed.is_empty(), "{label}: unexpected failure");
            commit
        }
        other => panic!("{label}: expected executed, got {other:?}"),
    }
}

/// The hand-derived token stream for N distinct functions.
fn fixture_tokens(functions: usize) -> Vec<FixtureToken> {
    let mut tokens = Vec::new();
    for index in 0..functions {
        let name = format!("f{index:02}").into_bytes();
        let value = index.to_string().into_bytes();
        tokens.extend([
            FixtureToken {
                kind: PpTokenKind::Identifier,
                spelling: b"int".to_vec(),
            },
            FixtureToken {
                kind: PpTokenKind::Identifier,
                spelling: name,
            },
            FixtureToken {
                kind: PpTokenKind::Punctuator,
                spelling: b"(".to_vec(),
            },
            FixtureToken {
                kind: PpTokenKind::Identifier,
                spelling: b"void".to_vec(),
            },
            FixtureToken {
                kind: PpTokenKind::Punctuator,
                spelling: b")".to_vec(),
            },
            FixtureToken {
                kind: PpTokenKind::Punctuator,
                spelling: b"{".to_vec(),
            },
            FixtureToken {
                kind: PpTokenKind::Identifier,
                spelling: b"return".to_vec(),
            },
            FixtureToken {
                kind: PpTokenKind::PpNumber,
                spelling: value,
            },
            FixtureToken {
                kind: PpTokenKind::Punctuator,
                spelling: b";".to_vec(),
            },
            FixtureToken {
                kind: PpTokenKind::Punctuator,
                spelling: b"}".to_vec(),
            },
        ]);
    }
    // `}(` and `){` remain separate tokens; a source separator is not needed
    // for the seeded PP records, but the generated source inserts one below.
    tokens.push(FixtureToken {
        kind: PpTokenKind::Eof,
        spelling: vec![],
    });
    tokens
}

/// Seed one source and its PP-token/span records from the fixture stream.
fn seed_source(bus: &mut CompilerBus, tokens: &[FixtureToken]) -> (SourceId, Vec<PpTokenId>) {
    let mut raw = Vec::new();
    for (position, token) in tokens.iter().enumerate() {
        if token.kind == PpTokenKind::Eof {
            continue;
        }
        if position > 0 {
            raw.push(b' ');
        }
        raw.extend_from_slice(&token.spelling);
    }
    raw.push(b'\n');
    let eof_offset = raw.len() as u64;
    let name = bus.intern_name(b"sprint_t18.c").unwrap();
    let source = bus.alloc_source(name, raw).unwrap();
    let limits = bus.limits();
    let mut offset = 0u64;
    let mut ids = Vec::new();
    for token in tokens {
        let (start, end) = if token.kind == PpTokenKind::Eof {
            (eof_offset, eof_offset)
        } else {
            let start = offset;
            let end = start + token.spelling.len() as u64;
            offset = end + 1;
            (start, end)
        };
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
                        kind: token.kind,
                        span,
                        spelling: token.spelling.clone(),
                    },
                    &limits,
                )
                .unwrap(),
        );
    }
    (source, ids)
}

/// Intern, classify, and decode every integer token in source order.
fn lex_fixture(functions: usize) -> (CompilerBus, Vec<TokenId>) {
    let mut bus = new_bus();
    install_vf05(&mut bus);
    let workers = workers();
    let fixture = fixture_tokens(functions);
    let (_, pp) = seed_source(&mut bus, &fixture);
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
    let report = tick(&mut bus, &workers);
    expect_executed(&report, "intern");

    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        LX_CLASSIFY_CHIP,
        Payload::from_refs(refs()),
    );
    let report = tick(&mut bus, &workers);
    let commit = expect_executed(&report, "classify");
    let tokens: Vec<TokenId> = commit
        .appended
        .iter()
        .map(|(_, reference)| match reference {
            RecordRef::Token(id) => *id,
            other => panic!("expected token ref, got {other:?}"),
        })
        .collect();
    assert_eq!(tokens.len(), functions * TOKENS_PER_FUNCTION + 1);

    for token in &tokens {
        let body = bus.arenas.tokens.get(*token).unwrap();
        if body.kind == TokenKind::Integer {
            let pp_token = body.pp_token;
            bootstrap(
                &mut bus,
                TaskKind::LEX_DECODE_LITERAL,
                LX_DECODE_CHIP,
                Payload::from_refs(vec![RecordRef::Token(*token), RecordRef::PpToken(pp_token)]),
            );
            let report = tick(&mut bus, &workers);
            expect_executed(&report, "decode");
        }
    }
    (bus, tokens)
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

fn node(bus: &CompilerBus, index: u32) -> cc_silicon_compiler::bus::NodeRecord {
    bus.arenas
        .nodes
        .get(NodeId::from_index(index))
        .unwrap()
        .clone()
}

/// Independent structural walk of the accepted 32-function tree.
fn verify_accepted_tree(bus: &CompilerBus, tokens: &[TokenId]) {
    assert_eq!(
        bus.arenas.nodes.allocated(),
        (1 + ACCEPTED_FUNCTIONS * NODES_PER_FUNCTION) as u32
    );

    let expected_kinds = {
        let mut kinds = vec![NodeKind::TranslationUnit];
        for _ in 0..ACCEPTED_FUNCTIONS {
            kinds.extend([
                NodeKind::FunctionDefinition,
                NodeKind::Specifiers,
                NodeKind::Declarator,
                NodeKind::Compound,
                NodeKind::Return,
                NodeKind::IntLiteral,
            ]);
        }
        kinds
    };
    let kinds: Vec<NodeKind> = (0..bus.arenas.nodes.allocated())
        .map(|index| node(bus, index).kind)
        .collect();
    assert_eq!(kinds, expected_kinds);

    let root = node(bus, 0);
    let expected_children: Vec<NodeId> = (0..ACCEPTED_FUNCTIONS)
        .map(|index| NodeId::from_index(1 + (index * NODES_PER_FUNCTION) as u32))
        .collect();
    assert_eq!(root.kind, NodeKind::TranslationUnit);
    assert_eq!(root.parent, None);
    assert_eq!(root.children, expected_children);
    assert_eq!(
        (root.first_token, root.last_token),
        (tokens[0], tokens[ACCEPTED_FUNCTIONS * TOKENS_PER_FUNCTION])
    );

    for index in 0..ACCEPTED_FUNCTIONS {
        let base = (1 + index * NODES_PER_FUNCTION) as u32;
        let token_base = index * TOKENS_PER_FUNCTION;
        let function = node(bus, base);
        assert_eq!(function.kind, NodeKind::FunctionDefinition);
        assert_eq!(function.parent, Some(NodeId::from_index(0)));
        assert_eq!(
            function.children,
            vec![
                NodeId::from_index(base + 1),
                NodeId::from_index(base + 2),
                NodeId::from_index(base + 3),
            ]
        );
        assert_eq!(
            (function.first_token, function.last_token),
            (tokens[token_base], tokens[token_base + 9])
        );

        let specifiers = node(bus, base + 1);
        assert_eq!(specifiers.kind, NodeKind::Specifiers);
        assert_eq!(specifiers.parent, Some(NodeId::from_index(base)));
        assert!(specifiers.children.is_empty());
        assert_eq!(
            (specifiers.first_token, specifiers.last_token),
            (tokens[token_base], tokens[token_base])
        );

        let declarator = node(bus, base + 2);
        assert_eq!(declarator.kind, NodeKind::Declarator);
        assert_eq!(declarator.parent, Some(NodeId::from_index(base)));
        assert!(declarator.children.is_empty());
        let expected_name = format!("f{index:02}");
        assert_eq!(
            declarator.name,
            bus.intern.lookup(expected_name.as_bytes()),
            "declarator name for {expected_name}"
        );
        assert_eq!(
            (declarator.first_token, declarator.last_token),
            (tokens[token_base + 1], tokens[token_base + 4])
        );

        let compound = node(bus, base + 3);
        assert_eq!(compound.kind, NodeKind::Compound);
        assert_eq!(compound.parent, Some(NodeId::from_index(base)));
        assert_eq!(compound.children, vec![NodeId::from_index(base + 4)]);
        assert_eq!(
            (compound.first_token, compound.last_token),
            (tokens[token_base + 5], tokens[token_base + 9])
        );

        let returned = node(bus, base + 4);
        assert_eq!(returned.kind, NodeKind::Return);
        assert_eq!(returned.parent, Some(NodeId::from_index(base + 3)));
        assert_eq!(returned.children, vec![NodeId::from_index(base + 5)]);
        assert_eq!(
            (returned.first_token, returned.last_token),
            (tokens[token_base + 6], tokens[token_base + 8])
        );

        let literal = node(bus, base + 5);
        assert_eq!(literal.kind, NodeKind::IntLiteral);
        assert_eq!(literal.parent, Some(NodeId::from_index(base + 4)));
        assert!(literal.children.is_empty());
        assert_eq!(
            (literal.first_token, literal.last_token),
            (tokens[token_base + 7], tokens[token_base + 7])
        );
        let literal_id = literal.literal.expect("literal leaf owns a literal");
        let record = bus.arenas.literals.get(literal_id).unwrap();
        assert_eq!(record.spelling, index.to_string().into_bytes());
        assert_eq!(record.value, vec![index as u8]);
        assert_eq!(record.token, Some(tokens[token_base + 7]));
    }
}

fn run_vf05(bus: &mut CompilerBus, tu: NodeId) -> TickReport {
    bootstrap(
        bus,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu)]),
    );
    tick(bus, &workers())
}

#[test]
fn thirty_two_functions_parse_and_pass_independent_ast_verification() {
    let (mut bus, tokens) = lex_fixture(ACCEPTED_FUNCTIONS);
    let report = parse_tu(&mut bus, &tokens);
    let commit = match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert!(commit.failed.is_empty());
            assert_eq!(commit.completed.len(), 1);
            assert_eq!(
                commit.appended.len(),
                1 + ACCEPTED_FUNCTIONS * NODES_PER_FUNCTION
            );
            commit
        }
        other => panic!("expected parse execution, got {other:?}"),
    };
    let result = bus.arenas.results.get(commit.completed[0].1).unwrap();
    assert_eq!(
        result.value,
        ResultValue::Record(RecordRef::Node(NodeId::from_index(0)))
    );
    verify_accepted_tree(&bus, &tokens);

    let before = bus.arenas.nodes.allocated();
    let report = run_vf05(&mut bus, NodeId::from_index(0));
    let commit = expect_executed(&report, "vf05");
    assert_eq!(commit.completed.len(), 1);
    assert!(commit.appended.is_empty());
    let value = bus
        .arenas
        .results
        .get(commit.completed[0].1)
        .unwrap()
        .value
        .clone();
    assert_eq!(value, ResultValue::Ack);
    assert_eq!(bus.arenas.nodes.allocated(), before);
}

#[test]
fn thirty_three_functions_fail_without_appending_an_ast() {
    let (mut bus, tokens) = lex_fixture(REJECTED_FUNCTIONS);
    let report = parse_tu(&mut bus, &tokens);
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 1);
            assert!(commit.completed.is_empty());
            assert!(commit.appended.is_empty());
            let diagnostic = bus.arenas.diagnostics.get(commit.failed[0].1).unwrap();
            assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(bus.arenas.nodes.allocated(), 0);
}
