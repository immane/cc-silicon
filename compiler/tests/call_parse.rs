// ============================================================================
// call_parse.rs — `/55` direct-call parser acceptance (CALL-P1).
//
// Exercises the general multi-function parser over real LANE token records:
// direct calls, parameters, caller locals, the fixed resource budgets, atomic
// failure, and the unchanged canonical/`/53` legacy paths. Every expected node
// index, child list, and token span is hand-derived from the fixture source.
// This target claims parsing only; semantic, IR, verification, and native
// execution are separate lanes.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, NodeKind, NodeRecord, PpTokenKind, PpTokenRecord, SpanRecord,
    TokenKind,
};
use cc_silicon_compiler::chips::{
    handler_for, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip, Worker,
    WorkerRegistry,
};
use cc_silicon_compiler::ids::{NodeId, PpTokenId, RecordRef, SourceId, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{StoreSchema, PA_TU_CHIP};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, TaskDraft, TaskKind, TaskKindRegistry};

fn new_bus() -> CompilerBus {
    // The AST budget (4096 nodes) needs more tokens than the fixture's
    // per-tick proposal budget (4096) allows in one lex/classify batch. Raise
    // only that scheduling bound; the parser budgets come from limits.rs.
    let mut limits = Limits::fixture();
    limits.max_proposals_per_tick = 1 << 16;
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        limits,
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
        (
            TaskKind::LEX_INTERN,
            cc_silicon_compiler::manifest::LX_INTERN_CHIP,
        ),
        (
            TaskKind::LEX_CLASSIFY,
            cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
        ),
        (
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
        ),
    ] {
        bus.routing.register(kind, owner, 2).unwrap();
    }
    bus.routing
        .register(TaskKind::PARSE_TU, PA_TU_CHIP, 2)
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

fn tick(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
) -> cc_silicon_compiler::routing::TickReport {
    RoutingShell
        .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
        .unwrap()
}

/// A small deterministic tokenizer for fixtures. It seeds PP records only;
/// the real LX chips still intern, classify, and decode every token.
fn seed_pp_source(bus: &mut CompilerBus, raw: &[u8]) -> Vec<PpTokenId> {
    let name = bus.intern_name(b"call.c").unwrap();
    let source: SourceId = bus.alloc_source(name, raw.to_vec()).unwrap();
    let limits = bus.limits();
    let mut ids = Vec::new();
    let mut index = 0usize;
    while index < raw.len() {
        if raw[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        let start = index;
        let kind = if raw[index].is_ascii_alphabetic() || raw[index] == b'_' {
            index += 1;
            while index < raw.len() && (raw[index].is_ascii_alphanumeric() || raw[index] == b'_') {
                index += 1;
            }
            PpTokenKind::Identifier
        } else if raw[index].is_ascii_digit() {
            index += 1;
            while index < raw.len() && raw[index].is_ascii_digit() {
                index += 1;
            }
            PpTokenKind::PpNumber
        } else if raw[index..].starts_with(b"...") {
            index += 3;
            PpTokenKind::Punctuator
        } else if b"(){};,=+-*/%".contains(&raw[index]) {
            index += 1;
            PpTokenKind::Punctuator
        } else {
            panic!(
                "fixture tokenizer cannot handle byte {:?} at {index}",
                raw[index] as char
            );
        };
        let span = bus
            .arenas
            .spans
            .alloc(
                SpanRecord {
                    source,
                    start: start as u64,
                    end: index as u64,
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
                        kind,
                        span,
                        spelling: raw[start..index].to_vec(),
                    },
                    &limits,
                )
                .unwrap(),
        );
    }
    let span = bus
        .arenas
        .spans
        .alloc(
            SpanRecord {
                source,
                start: raw.len() as u64,
                end: raw.len() as u64,
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
                    kind: PpTokenKind::Eof,
                    span,
                    spelling: vec![],
                },
                &limits,
            )
            .unwrap(),
    );
    ids
}

fn appended_tokens(report: &cc_silicon_compiler::routing::TickReport) -> Vec<TokenId> {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 0);
            commit
                .appended
                .iter()
                .map(|(_, reference)| match reference {
                    RecordRef::Token(id) => *id,
                    other => panic!("expected token ref, got {other:?}"),
                })
                .collect()
        }
        other => panic!("expected executed, got {other:?}"),
    }
}

fn lex_fixture(bus: &mut CompilerBus, workers: &WorkerRegistry, raw: &[u8]) -> Vec<TokenId> {
    let pp = seed_pp_source(bus, raw);
    let refs = || {
        pp.iter()
            .map(|id| RecordRef::PpToken(*id))
            .collect::<Vec<_>>()
    };
    bootstrap(
        bus,
        TaskKind::LEX_INTERN,
        cc_silicon_compiler::manifest::LX_INTERN_CHIP,
        Payload::from_refs(refs()),
    );
    let _ = tick(bus, workers);
    bootstrap(
        bus,
        TaskKind::LEX_CLASSIFY,
        cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
        Payload::from_refs(refs()),
    );
    let tokens = appended_tokens(&tick(bus, workers));
    for token in &tokens {
        let body = bus.arenas.tokens.get(*token).unwrap();
        if body.kind == TokenKind::Integer {
            let pp_token = body.pp_token;
            bootstrap(
                bus,
                TaskKind::LEX_DECODE_LITERAL,
                cc_silicon_compiler::manifest::LX_DECODE_CHIP,
                Payload::from_refs(vec![RecordRef::Token(*token), RecordRef::PpToken(pp_token)]),
            );
            let report = tick(bus, workers);
            match report.outcome {
                TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 0),
                other => panic!("expected executed, got {other:?}"),
            }
        }
    }
    tokens
}

struct Parsed {
    bus: CompilerBus,
    tokens: Vec<TokenId>,
    accepted: bool,
}

fn parse_source(source: &str) -> Parsed {
    let mut bus = new_bus();
    install_pa(&mut bus);
    let workers = workers();
    let tokens = lex_fixture(&mut bus, &workers, source.as_bytes());
    bootstrap(
        &mut bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(tokens.iter().map(|id| RecordRef::Token(*id)).collect()),
    );
    let report = tick(&mut bus, &workers);
    let accepted = match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len() + commit.failed.len(), 1);
            commit.completed.len() == 1
        }
        other => panic!("expected executed, got {other:?}"),
    };
    Parsed {
        bus,
        tokens,
        accepted,
    }
}

fn node(bus: &CompilerBus, index: u32) -> NodeRecord {
    bus.arenas
        .nodes
        .get(NodeId::from_index(index))
        .unwrap()
        .clone()
}

fn node_children(bus: &CompilerBus, index: u32) -> Vec<u32> {
    node(bus, index)
        .children
        .iter()
        .map(|id| id.index())
        .collect()
}

fn node_parent(bus: &CompilerBus, index: u32) -> Option<u32> {
    node(bus, index).parent.map(|id| id.index())
}

fn assert_children(bus: &CompilerBus, expected: &[Vec<u32>]) {
    for (index, children) in expected.iter().enumerate() {
        assert_eq!(
            node_children(bus, index as u32),
            *children,
            "children of node {index}"
        );
    }
}

fn assert_spans(bus: &CompilerBus, tokens: &[TokenId], expected: &[(usize, usize)]) {
    for (index, (first, last)) in expected.iter().enumerate() {
        let record = node(bus, index as u32);
        assert_eq!(
            (record.first_token, record.last_token),
            (tokens[*first], tokens[*last]),
            "span of node {index}"
        );
    }
}

fn assert_name(bus: &CompilerBus, index: u32, spelling: &[u8]) {
    assert_eq!(
        node(bus, index).name,
        bus.intern.lookup(spelling),
        "name of node {index}"
    );
}

#[test]
fn multi_function_call_tree_shape_and_spans() {
    let parsed = parse_source(
        "int add(int a, int b) { return a + b; }\n\
         int main(void) { return add(2, 3); }\n",
    );
    assert!(parsed.accepted);
    let bus = &parsed.bus;
    assert_eq!(bus.arenas.nodes.allocated(), 21);
    let expected = [
        NodeKind::TranslationUnit,
        NodeKind::FunctionDefinition,
        NodeKind::Specifiers,
        NodeKind::Declarator,
        NodeKind::ParameterList,
        NodeKind::Parameter,
        NodeKind::Parameter,
        NodeKind::Compound,
        NodeKind::Return,
        NodeKind::BinaryAdd,
        NodeKind::Identifier,
        NodeKind::Identifier,
        NodeKind::FunctionDefinition,
        NodeKind::Specifiers,
        NodeKind::Declarator,
        NodeKind::Compound,
        NodeKind::Return,
        NodeKind::Call,
        NodeKind::Identifier,
        NodeKind::IntLiteral,
        NodeKind::IntLiteral,
    ];
    let kinds: Vec<NodeKind> = (0..21).map(|index| node(bus, index).kind).collect();
    assert_eq!(kinds, expected);
    assert_children(
        bus,
        &[
            vec![1, 12],
            vec![2, 3, 7],
            vec![],
            vec![4],
            vec![5, 6],
            vec![],
            vec![],
            vec![8],
            vec![9],
            vec![10, 11],
            vec![],
            vec![],
            vec![13, 14, 15],
            vec![],
            vec![],
            vec![16],
            vec![17],
            vec![18, 19, 20],
            vec![],
            vec![],
            vec![],
        ],
    );
    let parents: Vec<Option<u32>> = (0..21).map(|index| node_parent(bus, index)).collect();
    assert_eq!(
        parents,
        vec![
            None,
            Some(0),
            Some(1),
            Some(1),
            Some(3),
            Some(4),
            Some(4),
            Some(1),
            Some(7),
            Some(8),
            Some(9),
            Some(9),
            Some(0),
            Some(12),
            Some(12),
            Some(12),
            Some(15),
            Some(16),
            Some(17),
            Some(17),
            Some(17),
        ]
    );
    assert_spans(
        bus,
        &parsed.tokens,
        &[
            (0, 31),
            (0, 15),
            (0, 0),
            (1, 8),
            (2, 8),
            (3, 4),
            (6, 7),
            (9, 15),
            (10, 14),
            (11, 13),
            (11, 11),
            (13, 13),
            (16, 30),
            (16, 16),
            (17, 20),
            (21, 30),
            (22, 29),
            (23, 28),
            (23, 23),
            (25, 25),
            (27, 27),
        ],
    );
    assert_name(bus, 3, b"add");
    assert_name(bus, 5, b"a");
    assert_name(bus, 6, b"b");
    assert_name(bus, 10, b"a");
    assert_name(bus, 11, b"b");
    assert_name(bus, 14, b"main");
    assert_name(bus, 18, b"add");
    for (node_index, token_index) in [(19, 25), (20, 27)] {
        let leaf = node(bus, node_index);
        let literal = bus.arenas.literals.get(leaf.literal.unwrap()).unwrap();
        assert_eq!(literal.token, Some(parsed.tokens[token_index]));
    }
}

#[test]
fn zero_argument_declarations_and_call() {
    let parsed = parse_source(
        "int five(void) { return 5; }\n\
         int main(void) { return five(); }\n",
    );
    assert!(parsed.accepted);
    let bus = &parsed.bus;
    assert_eq!(bus.arenas.nodes.allocated(), 14);
    let kinds: Vec<NodeKind> = (0..14).map(|index| node(bus, index).kind).collect();
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
    assert_children(
        bus,
        &[
            vec![1, 7],
            vec![2, 3, 4],
            vec![],
            vec![],
            vec![5],
            vec![6],
            vec![],
            vec![8, 9, 10],
            vec![],
            vec![],
            vec![11],
            vec![12],
            vec![13],
            vec![],
        ],
    );
    // `(void)` declarators stay leaves; direct zero-argument calls carry only
    // the callee identifier.
    assert!(node(bus, 3).children.is_empty());
    assert!(node(bus, 9).children.is_empty());
    assert_eq!(node_children(bus, 12), vec![13]);
    assert_spans(
        bus,
        &parsed.tokens,
        &[
            (0, 22),
            (0, 9),
            (0, 0),
            (1, 4),
            (5, 9),
            (6, 8),
            (7, 7),
            (10, 21),
            (10, 10),
            (11, 14),
            (15, 21),
            (16, 20),
            (17, 19),
            (17, 17),
        ],
    );
    assert_name(bus, 3, b"five");
    assert_name(bus, 9, b"main");
    assert_name(bus, 13, b"five");
}

#[test]
fn nested_calls_preserve_argument_order() {
    let parsed = parse_source(
        "int inc(int x) { return x + 1; }\n\
         int add(int a, int b) { return a + b; }\n\
         int main(void) { return add(inc(2), inc(3)); }\n",
    );
    assert!(parsed.accepted);
    let bus = &parsed.bus;
    assert_eq!(bus.arenas.nodes.allocated(), 35);
    let expected_children = [
        vec![1, 11, 22],
        vec![2, 3, 6],
        vec![],
        vec![4],
        vec![5],
        vec![],
        vec![7],
        vec![8],
        vec![9, 10],
        vec![],
        vec![],
        vec![12, 13, 17],
        vec![],
        vec![14],
        vec![15, 16],
        vec![],
        vec![],
        vec![18],
        vec![19],
        vec![20, 21],
        vec![],
        vec![],
        vec![23, 24, 25],
        vec![],
        vec![],
        vec![26],
        vec![27],
        vec![28, 29, 32],
        vec![],
        vec![30, 31],
        vec![],
        vec![],
        vec![33, 34],
        vec![],
        vec![],
    ];
    assert_children(bus, &expected_children);
    assert_eq!(
        node(bus, 27).kind,
        NodeKind::Call,
        "outer add call is node 27"
    );
    assert_eq!(node_children(bus, 27), vec![28, 29, 32]);
    assert_eq!(node(bus, 29).kind, NodeKind::Call);
    assert_eq!(node_children(bus, 29), vec![30, 31]);
    assert_eq!(node(bus, 32).kind, NodeKind::Call);
    assert_eq!(node_children(bus, 32), vec![33, 34]);
    assert_name(bus, 28, b"add");
    assert_name(bus, 30, b"inc");
    assert_name(bus, 33, b"inc");
    assert_spans(
        bus,
        &parsed.tokens,
        &[
            (0, 50),
            (0, 12),
            (0, 0),
            (1, 5),
            (2, 5),
            (3, 4),
            (6, 12),
            (7, 11),
            (8, 10),
            (8, 8),
            (10, 10),
            (13, 28),
            (13, 13),
            (14, 21),
            (15, 21),
            (16, 17),
            (19, 20),
            (22, 28),
            (23, 27),
            (24, 26),
            (24, 24),
            (26, 26),
            (29, 49),
            (29, 29),
            (30, 33),
            (34, 49),
            (35, 48),
            (36, 47),
            (36, 36),
            (38, 41),
            (38, 38),
            (40, 40),
            (43, 46),
            (43, 43),
            (45, 45),
        ],
    );
    assert_eq!(node(bus, 27).first_token, parsed.tokens[36]);
    assert_eq!(node(bus, 27).last_token, parsed.tokens[47]);
}

#[test]
fn caller_local_call_builds_local_and_call() {
    let parsed = parse_source(
        "int add(int a, int b) { return a + b; }\n\
         int main(void) { int x = 2; return add(x, 3); }\n",
    );
    assert!(parsed.accepted);
    let bus = &parsed.bus;
    assert_eq!(bus.arenas.nodes.allocated(), 25);
    assert_children(
        bus,
        &[
            vec![1, 12],
            vec![2, 3, 7],
            vec![],
            vec![4],
            vec![5, 6],
            vec![],
            vec![],
            vec![8],
            vec![9],
            vec![10, 11],
            vec![],
            vec![],
            vec![13, 14, 15],
            vec![],
            vec![],
            vec![16, 20],
            vec![17, 18, 19],
            vec![],
            vec![],
            vec![],
            vec![21],
            vec![22, 23, 24],
            vec![],
            vec![],
            vec![],
        ],
    );
    assert_eq!(node(bus, 16).kind, NodeKind::Declaration);
    assert_eq!(node(bus, 16).first_token, parsed.tokens[22]);
    assert_eq!(node(bus, 16).last_token, parsed.tokens[26]);
    assert_eq!(node_children(bus, 21), vec![22, 23, 24]);
    assert_name(bus, 14, b"main");
    assert_name(bus, 18, b"x");
    assert_name(bus, 22, b"add");
    assert_name(bus, 23, b"x");
    assert_spans(
        bus,
        &parsed.tokens,
        &[
            (0, 36),
            (0, 15),
            (0, 0),
            (1, 8),
            (2, 8),
            (3, 4),
            (6, 7),
            (9, 15),
            (10, 14),
            (11, 13),
            (11, 11),
            (13, 13),
            (16, 35),
            (16, 16),
            (17, 20),
            (21, 35),
            (22, 26),
            (22, 22),
            (23, 23),
            (25, 25),
            (27, 34),
            (28, 33),
            (28, 28),
            (30, 30),
            (32, 32),
        ],
    );
}

#[test]
fn eight_arguments_accepted_nine_rejected() {
    let parameters = ["a", "b", "c", "d", "e", "f", "g", "h"]
        .iter()
        .map(|name| format!("int {name}"))
        .collect::<Vec<_>>()
        .join(", ");
    let eight = (1..=8)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let accepted = parse_source(&format!(
        "int eight({parameters}) {{ return a; }}\n\
         int main(void) {{ return eight({eight}); }}\n"
    ));
    assert!(accepted.accepted);
    let bus = &accepted.bus;
    let parameter_list = (0..bus.arenas.nodes.allocated())
        .find(|index| node(bus, *index).kind == NodeKind::ParameterList)
        .unwrap();
    assert_eq!(node_children(bus, parameter_list).len(), 8);
    let parameter_names: Vec<Option<cc_silicon_compiler::ids::NameId>> =
        node_children(bus, parameter_list)
            .iter()
            .map(|index| node(bus, *index).name)
            .collect();
    let expected_names: Vec<Option<cc_silicon_compiler::ids::NameId>> = [
        b"a".as_slice(),
        b"b".as_slice(),
        b"c".as_slice(),
        b"d".as_slice(),
        b"e".as_slice(),
        b"f".as_slice(),
        b"g".as_slice(),
        b"h".as_slice(),
    ]
    .iter()
    .map(|name| bus.intern.lookup(name))
    .collect();
    assert_eq!(parameter_names, expected_names);
    let call = (0..bus.arenas.nodes.allocated())
        .find(|index| node(bus, *index).kind == NodeKind::Call)
        .unwrap();
    assert_eq!(node_children(bus, call).len(), 9);

    let nine = (1..=9)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let rejected = parse_source(&format!(
        "int eight({parameters}) {{ return a; }}\n\
         int main(void) {{ return eight({nine}); }}\n"
    ));
    assert!(!rejected.accepted);
    assert_eq!(rejected.bus.arenas.nodes.allocated(), 0);
}

#[test]
fn function_and_parameter_budgets_fail_on_the_next_value() {
    let functions = |count: usize| {
        (0..count)
            .map(|index| format!("int f{index}(void) {{ return 1; }}\n"))
            .collect::<String>()
    };
    let accepted = parse_source(&functions(32));
    assert!(accepted.accepted);
    assert_eq!(accepted.bus.arenas.nodes.allocated(), 1 + 32 * 6);
    assert_eq!(node_children(&accepted.bus, 0).len(), 32);
    let rejected = parse_source(&functions(33));
    assert!(!rejected.accepted);
    assert_eq!(rejected.bus.arenas.nodes.allocated(), 0);

    let nine_parameters =
        "int nine(int a, int b, int c, int d, int e, int f, int g, int h, int i) \
                           { return a; }\nint main(void) { return 1; }\n";
    let rejected = parse_source(nine_parameters);
    assert!(!rejected.accepted);
    assert_eq!(rejected.bus.arenas.nodes.allocated(), 0);
}

#[test]
fn expression_depth_budget_accepts_32_and_rejects_33() {
    let nested = |levels: usize| {
        format!(
            "int main(void) {{ return {}{}{}; }}\n",
            "(".repeat(levels),
            "1",
            ")".repeat(levels)
        )
    };
    let accepted = parse_source(&nested(32));
    assert!(accepted.accepted);
    assert_eq!(accepted.bus.arenas.nodes.allocated(), 7);
    let rejected = parse_source(&nested(33));
    assert!(!rejected.accepted);
    assert_eq!(rejected.bus.arenas.nodes.allocated(), 0);
}

#[test]
fn ast_node_budget_accepts_4096_and_rejects_4097() {
    let accepted_source = {
        let mut source = String::from("int entry(void){int x=1;");
        for _ in 0..1360 {
            source.push_str("x=1;");
        }
        source.push_str("x=1+2;");
        source.push_str("return 1;}");
        source
    };
    let accepted = parse_source(&accepted_source);
    assert!(accepted.accepted);
    assert_eq!(accepted.bus.arenas.nodes.allocated(), 4096);

    let rejected_source = {
        let mut source = String::from("int entry(void){int x=1;");
        for _ in 0..1359 {
            source.push_str("x=1;");
        }
        source.push_str("x=1+2;");
        source.push_str("x=-1;");
        source.push_str("return 1;}");
        source
    };
    let rejected = parse_source(&rejected_source);
    assert!(!rejected.accepted);
    assert_eq!(rejected.bus.arenas.nodes.allocated(), 0);
}

#[test]
fn malformed_calls_fail_atomically() {
    for source in [
        "int ok(void) { return 1; }\nint main(void) { return add(1, 2; }\n",
        "int main(void) { return add(, 2); }\n",
        "int main(void) { return add 1, 2); }\n",
    ] {
        let parsed = parse_source(source);
        assert!(!parsed.accepted, "source unexpectedly accepted: {source}");
        assert_eq!(parsed.bus.arenas.nodes.allocated(), 0);
    }
}

#[test]
fn unsupported_direct_call_shapes_are_rejected() {
    for source in [
        "int add(int a, int b);\n",
        "int main() { return 1; }\n",
        "int add(int a, ...) { return a; }\n",
        "int main(void) { add(1, 2); return 1; }\n",
        "int main(void) { return (add)(1); }\n",
        "int main(void) { return add(void); }\n",
        "int main(void) { int x = 1; int y = 2; return x; }\n",
        "int add(char x) { return 1; }\n",
        "void main(void) { return 1; }\n",
        "int main(void) { { return 1; } }\n",
        "int main(void) { if (1) return 1; return 2; }\n",
    ] {
        let parsed = parse_source(source);
        assert!(!parsed.accepted, "source unexpectedly accepted: {source}");
        assert_eq!(parsed.bus.arenas.nodes.allocated(), 0);
    }
}

#[test]
fn legacy_canonical_tree_is_unchanged() {
    let parsed = parse_source("int main(void){return 2+3;}\n");
    assert!(parsed.accepted);
    let bus = &parsed.bus;
    assert_eq!(bus.arenas.nodes.allocated(), 9);
    let kinds: Vec<NodeKind> = (0..9).map(|index| node(bus, index).kind).collect();
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
    assert_children(
        bus,
        &[
            vec![1],
            vec![2, 3, 4],
            vec![],
            vec![],
            vec![5],
            vec![6],
            vec![7, 8],
            vec![],
            vec![],
        ],
    );
    assert_spans(
        bus,
        &parsed.tokens,
        &[
            (0, 12),
            (0, 11),
            (0, 0),
            (1, 4),
            (5, 11),
            (6, 10),
            (7, 9),
            (7, 7),
            (9, 9),
        ],
    );
    assert_name(bus, 3, b"main");
    for (node_index, token_index) in [(7, 7), (8, 9)] {
        let leaf = node(bus, node_index);
        assert_eq!(leaf.first_token, parsed.tokens[token_index]);
        assert_eq!(leaf.last_token, parsed.tokens[token_index]);
        let literal = bus.arenas.literals.get(leaf.literal.unwrap()).unwrap();
        assert_eq!(literal.token, Some(parsed.tokens[token_index]));
    }
}

#[test]
fn legacy_local_assignment_tree_is_unchanged() {
    let parsed = parse_source("int main(void){int x=2;x=x+3;return x;}\n");
    assert!(parsed.accepted);
    let bus = &parsed.bus;
    assert_eq!(bus.arenas.nodes.allocated(), 16);
    let kinds: Vec<NodeKind> = (0..16).map(|index| node(bus, index).kind).collect();
    assert_eq!(
        kinds,
        vec![
            NodeKind::TranslationUnit,
            NodeKind::FunctionDefinition,
            NodeKind::Specifiers,
            NodeKind::Declarator,
            NodeKind::Compound,
            NodeKind::Declaration,
            NodeKind::Specifiers,
            NodeKind::Declarator,
            NodeKind::IntLiteral,
            NodeKind::Assignment,
            NodeKind::Identifier,
            NodeKind::BinaryAdd,
            NodeKind::Identifier,
            NodeKind::IntLiteral,
            NodeKind::Return,
            NodeKind::Identifier,
        ]
    );
    assert_children(
        bus,
        &[
            vec![1],
            vec![2, 3, 4],
            vec![],
            vec![],
            vec![5, 9, 14],
            vec![6, 7, 8],
            vec![],
            vec![],
            vec![],
            vec![10, 11],
            vec![],
            vec![12, 13],
            vec![],
            vec![],
            vec![15],
            vec![],
        ],
    );
    assert_spans(
        bus,
        &parsed.tokens,
        &[
            (0, 21),
            (0, 20),
            (0, 0),
            (1, 4),
            (5, 20),
            (6, 10),
            (6, 6),
            (7, 7),
            (9, 9),
            (11, 15),
            (11, 11),
            (13, 15),
            (13, 13),
            (15, 15),
            (17, 19),
            (18, 18),
        ],
    );
    assert_name(bus, 3, b"main");
    assert_name(bus, 7, b"x");
    assert_name(bus, 10, b"x");
    assert_name(bus, 12, b"x");
    assert_name(bus, 15, b"x");
}
