// ============================================================================
// sprint_t19.rs — `/55` expression-depth adversarial regression (T19).
//
// Focus: grouping, unary, and call-argument nesting at the frozen 32/33
// boundary, including a mixed-nesting case, plus bounded typed failure with no
// partial AST append.
//
// The harness seeds committed PP/token/literal records directly and dispatches
// only `PA_TU_CHIP`, so the assertions isolate parser depth from lexing and
// semantic stages. Every node index below is derived by hand from the
// pre-order assignment in the parsed tree; no expected value is obtained by
// running the implementation under test.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, LiteralRecord, NodeKind, NodeRecord, PpTokenKind, PpTokenRecord,
    SpanRecord, TokenKind, TokenRecord,
};
use cc_silicon_compiler::chips::{handler_for, PaTuChip, Worker, WorkerRegistry};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{DiagnosticId, NodeId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{StoreSchema, PA_TU_CHIP};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, TaskDraft, TaskKind, TaskKindRegistry, TaskState};

/// Frozen `CALL_MAX_EXPRESSION_DEPTH` value from the `/55` contract.
const DEPTH_LIMIT: usize = 32;

/// One hand-built committed token: kind plus surface spelling.
type FixtureToken = (TokenKind, &'static str);

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_parse(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::pa_slice();
    bus.schema = StoreSchema::pa_slice();
    bus.registrations
        .register(PaTuChip.manifest(), &bus.schema, &bus.kinds)
        .expect("register PA_TU");
    bus.routing
        .register(TaskKind::PARSE_TU, PA_TU_CHIP, 2)
        .expect("route PARSE_TU");
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PaTuChip).expect("register PaTuChip");
    workers
}

/// Seed committed PP, token, and integer-literal records in source order.
///
/// The parser only consumes committed records, so bypassing the lexical chips
/// keeps this file focused on PA_TU. The single span and synthetic source are
/// deliberately not inspected by the parser.
fn seed_tokens(bus: &mut CompilerBus, tokens: &[FixtureToken]) -> Vec<RecordRef> {
    let name = bus.intern_name(b"t19.c").expect("intern source name");
    let source = bus
        .alloc_source(name, b"t19 depth fixture".to_vec())
        .expect("allocate source");
    let limits = bus.limits();
    let span = bus
        .arenas
        .spans
        .alloc(
            SpanRecord {
                source,
                start: 0,
                end: tokens.len() as u64,
                expansion: None,
            },
            &limits,
        )
        .expect("allocate span");
    let mut refs = Vec::with_capacity(tokens.len());
    for &(kind, spelling) in tokens {
        let pp_kind = match kind {
            TokenKind::Integer => PpTokenKind::PpNumber,
            TokenKind::Eof => PpTokenKind::Eof,
            TokenKind::Punctuator => PpTokenKind::Punctuator,
            TokenKind::Keyword | TokenKind::Identifier => PpTokenKind::Identifier,
        };
        let pp = bus
            .arenas
            .pp_tokens
            .alloc(
                PpTokenRecord {
                    kind: pp_kind,
                    span,
                    spelling: spelling.as_bytes().to_vec(),
                },
                &limits,
            )
            .expect("allocate pp token");
        let interned = match kind {
            TokenKind::Keyword | TokenKind::Identifier => Some(
                bus.intern_name(spelling.as_bytes())
                    .expect("intern spelling"),
            ),
            _ => None,
        };
        let token = bus
            .arenas
            .tokens
            .alloc(
                TokenRecord {
                    kind,
                    span,
                    name: interned,
                    pp_token: pp,
                },
                &limits,
            )
            .expect("allocate token");
        if kind == TokenKind::Integer {
            bus.arenas
                .literals
                .alloc(
                    LiteralRecord {
                        token: Some(token),
                        kind: LiteralKind::Integer,
                        radix: 10,
                        suffix: LiteralSuffix::None,
                        value: vec![1],
                        negative: false,
                        spelling: spelling.as_bytes().to_vec(),
                        candidate_type: Lx08CandidateType::Int,
                    },
                    &limits,
                )
                .expect("allocate literal");
        }
        refs.push(RecordRef::Token(token));
    }
    refs
}

struct ParseRun {
    bus: CompilerBus,
    task: TaskId,
    accepted: bool,
    failure: Option<DiagnosticId>,
}

fn parse_tokens(tokens: &[FixtureToken]) -> ParseRun {
    let mut bus = new_bus();
    install_parse(&mut bus);
    let workers = workers();
    let refs = seed_tokens(&mut bus, tokens);
    let task = bus
        .bootstrap_task(TaskDraft {
            kind: TaskKind::PARSE_TU,
            owner: PA_TU_CHIP,
            parent: None,
            payload: Payload::from_refs(refs),
            continuation: None,
        })
        .expect("bootstrap parse task");
    let report = RoutingShell
        .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
        .expect("tick");
    let (accepted, failure) = match report.outcome {
        TickOutcome::Executed {
            task: selected,
            commit,
            ..
        } => {
            assert_eq!(selected, task);
            assert_eq!(
                commit.completed.len() + commit.failed.len(),
                1,
                "one terminal outcome for the sole parse task"
            );
            let failure = commit.failed.first().map(|(_, id)| *id);
            (commit.completed.len() == 1, failure)
        }
        other => panic!("expected an executed parse tick, got {other:?}"),
    };
    ParseRun {
        bus,
        task,
        accepted,
        failure,
    }
}

fn node(bus: &CompilerBus, index: u32) -> NodeRecord {
    bus.arenas
        .nodes
        .get(NodeId::from_index(index))
        .expect("committed node")
        .clone()
}

fn children(bus: &CompilerBus, index: u32) -> Vec<u32> {
    node(bus, index)
        .children
        .iter()
        .map(|child| child.index())
        .collect()
}

fn assert_accepted(run: &ParseRun, expected_nodes: u32) {
    assert!(run.accepted, "expected the parse to succeed");
    assert!(run.failure.is_none(), "a successful parse must not fail");
    assert_eq!(
        run.bus.arenas.nodes.allocated(),
        expected_nodes,
        "committed AST node count"
    );
    let state = run
        .bus
        .arenas
        .tasks
        .get(run.task)
        .expect("task")
        .state
        .clone();
    assert!(
        matches!(&state, TaskState::Completed(_)),
        "parse task state {state:?}"
    );
}

fn assert_bounded_failure(run: &ParseRun) {
    assert!(!run.accepted, "depth 33 must be rejected");
    assert_eq!(
        run.bus.arenas.nodes.allocated(),
        0,
        "a failed parse must not append partial AST nodes"
    );
    let state = run
        .bus
        .arenas
        .tasks
        .get(run.task)
        .expect("task")
        .state
        .clone();
    let failed = match &state {
        TaskState::Failed(id) => *id,
        other => panic!("expected a failed task, got {other:?}"),
    };
    assert_eq!(Some(failed), run.failure, "failed diagnostic identity");
    let diagnostic = run
        .bus
        .arenas
        .diagnostics
        .get(failed)
        .expect("committed failure diagnostic");
    assert_eq!(
        diagnostic.code.group,
        DiagGroup::Unsupported,
        "depth excess must be a typed unsupported diagnostic"
    );
}

fn tokens_for(expression: &[FixtureToken]) -> Vec<FixtureToken> {
    let mut tokens = vec![
        (TokenKind::Keyword, "int"),
        (TokenKind::Identifier, "main"),
        (TokenKind::Punctuator, "("),
        (TokenKind::Keyword, "void"),
        (TokenKind::Punctuator, ")"),
        (TokenKind::Punctuator, "{"),
        (TokenKind::Keyword, "return"),
    ];
    tokens.extend_from_slice(expression);
    tokens.push((TokenKind::Punctuator, ";"));
    tokens.push((TokenKind::Punctuator, "}"));
    tokens.push((TokenKind::Eof, ""));
    tokens
}

fn grouping(levels: usize) -> Vec<FixtureToken> {
    let mut expression = vec![(TokenKind::Punctuator, "("); levels];
    expression.push((TokenKind::Integer, "1"));
    expression.extend(vec![(TokenKind::Punctuator, ")"); levels]);
    expression
}

fn unary(levels: usize) -> Vec<FixtureToken> {
    let mut expression = vec![(TokenKind::Punctuator, "-"); levels];
    expression.push((TokenKind::Integer, "1"));
    expression
}

fn nested_calls(levels: usize) -> Vec<FixtureToken> {
    let mut expression = Vec::new();
    for _ in 0..levels {
        expression.push((TokenKind::Identifier, "f"));
        expression.push((TokenKind::Punctuator, "("));
    }
    expression.push((TokenKind::Integer, "1"));
    for _ in 0..levels {
        expression.push((TokenKind::Punctuator, ")"));
    }
    expression
}

fn grouped_unary(groups: usize) -> Vec<FixtureToken> {
    let mut expression = vec![(TokenKind::Punctuator, "("); groups];
    expression.push((TokenKind::Punctuator, "-"));
    expression.push((TokenKind::Integer, "1"));
    expression.extend(vec![(TokenKind::Punctuator, ")"); groups]);
    expression
}

fn grouped_call(groups: usize) -> Vec<FixtureToken> {
    let mut expression = vec![(TokenKind::Punctuator, "("); groups];
    expression.push((TokenKind::Identifier, "f"));
    expression.push((TokenKind::Punctuator, "("));
    expression.push((TokenKind::Integer, "1"));
    expression.push((TokenKind::Punctuator, ")"));
    expression.extend(vec![(TokenKind::Punctuator, ")"); groups]);
    expression
}

#[test]
fn grouping_depth_accepts_32_and_rejects_33() {
    let accepted = parse_tokens(&tokens_for(&grouping(DEPTH_LIMIT)));
    assert_accepted(&accepted, 7);
    assert_eq!(node(&accepted.bus, 5).kind, NodeKind::Return);
    assert_eq!(children(&accepted.bus, 5), vec![6]);
    assert_eq!(node(&accepted.bus, 6).kind, NodeKind::IntLiteral);

    let rejected = parse_tokens(&tokens_for(&grouping(DEPTH_LIMIT + 1)));
    assert_bounded_failure(&rejected);
}

#[test]
fn unary_depth_accepts_32_and_rejects_33() {
    let accepted = parse_tokens(&tokens_for(&unary(DEPTH_LIMIT)));
    assert_accepted(&accepted, 7 + DEPTH_LIMIT as u32);
    assert_eq!(node(&accepted.bus, 5).kind, NodeKind::Return);
    assert_eq!(children(&accepted.bus, 5), vec![6]);
    for index in 6..6 + DEPTH_LIMIT as u32 {
        assert_eq!(
            node(&accepted.bus, index).kind,
            NodeKind::UnaryMinus,
            "unary node {index}"
        );
        assert_eq!(
            children(&accepted.bus, index),
            vec![index + 1],
            "unary child {index}"
        );
    }
    assert_eq!(
        node(&accepted.bus, 6 + DEPTH_LIMIT as u32).kind,
        NodeKind::IntLiteral
    );

    let rejected = parse_tokens(&tokens_for(&unary(DEPTH_LIMIT + 1)));
    assert_bounded_failure(&rejected);
}

#[test]
fn call_argument_depth_accepts_32_and_rejects_33() {
    let accepted = parse_tokens(&tokens_for(&nested_calls(DEPTH_LIMIT)));
    assert_accepted(&accepted, 7 + 2 * DEPTH_LIMIT as u32);
    assert_eq!(node(&accepted.bus, 5).kind, NodeKind::Return);
    assert_eq!(children(&accepted.bus, 5), vec![6]);
    let literal = 6 + 2 * DEPTH_LIMIT as u32;
    for level in 0..DEPTH_LIMIT as u32 {
        let call = 6 + 2 * level;
        let callee = call + 1;
        let argument = call + 2;
        assert_eq!(node(&accepted.bus, call).kind, NodeKind::Call);
        assert_eq!(
            children(&accepted.bus, call),
            vec![callee, argument],
            "call node {call}"
        );
        let callee_node = node(&accepted.bus, callee);
        assert_eq!(callee_node.kind, NodeKind::Identifier);
        assert_eq!(callee_node.name, accepted.bus.intern.lookup(b"f"));
    }
    assert_eq!(
        node(&accepted.bus, literal).kind,
        NodeKind::IntLiteral,
        "innermost literal at {literal}"
    );

    let rejected = parse_tokens(&tokens_for(&nested_calls(DEPTH_LIMIT + 1)));
    assert_bounded_failure(&rejected);
}

#[test]
fn mixed_nesting_shares_the_same_depth_budget() {
    // Thirty-one groups plus one unary operator reach depth 32 and parse.
    let accepted = parse_tokens(&tokens_for(&grouped_unary(DEPTH_LIMIT - 1)));
    assert_accepted(&accepted, 8);
    assert_eq!(node(&accepted.bus, 6).kind, NodeKind::UnaryMinus);
    assert_eq!(children(&accepted.bus, 6), vec![7]);
    assert_eq!(node(&accepted.bus, 7).kind, NodeKind::IntLiteral);

    // Thirty-one groups plus one call argument also reach depth 32.
    let accepted = parse_tokens(&tokens_for(&grouped_call(DEPTH_LIMIT - 1)));
    assert_accepted(&accepted, 9);
    assert_eq!(node(&accepted.bus, 6).kind, NodeKind::Call);
    assert_eq!(children(&accepted.bus, 6), vec![7, 8]);
    assert_eq!(node(&accepted.bus, 7).kind, NodeKind::Identifier);
    assert_eq!(node(&accepted.bus, 8).kind, NodeKind::IntLiteral);

    // One more grouping level pushes each mixed form to depth 33.
    let rejected = parse_tokens(&tokens_for(&grouped_unary(DEPTH_LIMIT)));
    assert_bounded_failure(&rejected);

    let rejected = parse_tokens(&tokens_for(&grouped_call(DEPTH_LIMIT)));
    assert_bounded_failure(&rejected);
}
