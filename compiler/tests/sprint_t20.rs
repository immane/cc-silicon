#![allow(clippy::needless_range_loop)]
// ============================================================================
// sprint_t20.rs — T20 AST budget and same-batch link integrity.
//
// Focus: the `/55` parser budget accepts exactly 4096 AST nodes and rejects
// 4097; rejection publishes no partial batch. The accepted tree is validated
// independently: every parent/child link must stay inside the one append
// batch, every child must point back at its parent, and every node must be
// reachable from the TU root. Three sentinel nodes are committed before the
// accepted parse so the same-batch prediction base is non-zero.
//
// The fixture is 32 functions, each `int fN(void){ x=x; ... return x; }`.
// Hand-derived node count:
//   TU root                                  1
//   per function (FD, spec, decl, compound,
//                 return, return operand)    6
//   assignment `x=x;`                         3
//   assignment `x=-x;`                        4
//
// Accepted plan: 32*6 + 1301*3 + 1 = 4096.
// Rejected plan: 32*6 + 1300*3 + 1*4 + 1 = 4097.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, NodeKind, NodeRecord, PpTokenKind, PpTokenRecord, SpanRecord,
    TokenKind, TokenRecord,
};
use cc_silicon_compiler::chips::{handler_for, PaTuChip, Worker, WorkerRegistry};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{NodeId, RecordRef, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{StoreSchema, PA_TU_CHIP};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome, TickReport};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, TaskDraft, TaskKind, TaskKindRegistry};
use std::collections::BTreeSet;

const AST_BUDGET: usize = 4096;
const BASE_NODES: u32 = 3;
const FUNCTION_COUNT: usize = 32;

/// One projected-token fixture row.
struct RawToken {
    kind: TokenKind,
    spelling: Vec<u8>,
}

fn raw(kind: TokenKind, spelling: &str) -> RawToken {
    RawToken {
        kind,
        spelling: spelling.as_bytes().to_vec(),
    }
}

fn new_bus() -> CompilerBus {
    // The 4096-node append batch counts its drafts against the proposal
    // budget; the fixture bound is smaller than 4096 + 2.
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
    bus.registrations
        .register(PaTuChip.manifest(), &bus.schema, &bus.kinds)
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_TU, PA_TU_CHIP, 2)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PaTuChip).unwrap();
    workers
}

fn tick(bus: &mut CompilerBus, workers: &WorkerRegistry) -> TickReport {
    RoutingShell
        .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
        .unwrap()
}

/// The accepted body plan: 1301 plain assignments split over 32 functions.
/// 1301 = 32*40 + 21, so the first 21 functions get 41 and the rest get 40.
fn accepted_plans() -> (Vec<usize>, Vec<usize>) {
    let base = 1301 / FUNCTION_COUNT;
    let extra = 1301 % FUNCTION_COUNT;
    let plain = (0..FUNCTION_COUNT)
        .map(|index| base + if index < extra { 1 } else { 0 })
        .collect::<Vec<_>>();
    (plain, vec![0; FUNCTION_COUNT])
}

/// The rejected plan has exactly one more node than the accepted plan:
/// one plain assignment becomes one unary assignment in the last function.
fn rejected_plans() -> (Vec<usize>, Vec<usize>) {
    let (mut plain, mut unary) = accepted_plans();
    plain[FUNCTION_COUNT - 1] -= 1;
    unary[FUNCTION_COUNT - 1] = 1;
    (plain, unary)
}

fn expected_node_count(plain: &[usize], unary: &[usize]) -> usize {
    assert_eq!(plain.len(), unary.len());
    1 + 6 * plain.len() + 3 * plain.iter().sum::<usize>() + 4 * unary.iter().sum::<usize>()
}

/// Build the token stream for `int fN(void){ assignments return x; }`.
fn plan_tokens(plain: &[usize], unary: &[usize]) -> Vec<RawToken> {
    assert_eq!(plain.len(), unary.len());
    let mut tokens = Vec::new();
    for index in 0..plain.len() {
        tokens.push(raw(TokenKind::Keyword, "int"));
        tokens.push(RawToken {
            kind: TokenKind::Identifier,
            spelling: format!("f{index}").into_bytes(),
        });
        tokens.push(raw(TokenKind::Punctuator, "("));
        tokens.push(raw(TokenKind::Keyword, "void"));
        tokens.push(raw(TokenKind::Punctuator, ")"));
        tokens.push(raw(TokenKind::Punctuator, "{"));
        for _ in 0..plain[index] {
            tokens.push(raw(TokenKind::Identifier, "x"));
            tokens.push(raw(TokenKind::Punctuator, "="));
            tokens.push(raw(TokenKind::Identifier, "x"));
            tokens.push(raw(TokenKind::Punctuator, ";"));
        }
        for _ in 0..unary[index] {
            tokens.push(raw(TokenKind::Identifier, "x"));
            tokens.push(raw(TokenKind::Punctuator, "="));
            tokens.push(raw(TokenKind::Punctuator, "-"));
            tokens.push(raw(TokenKind::Identifier, "x"));
            tokens.push(raw(TokenKind::Punctuator, ";"));
        }
        tokens.push(raw(TokenKind::Keyword, "return"));
        tokens.push(raw(TokenKind::Identifier, "x"));
        tokens.push(raw(TokenKind::Punctuator, ";"));
        tokens.push(raw(TokenKind::Punctuator, "}"));
    }
    tokens.push(RawToken {
        kind: TokenKind::Eof,
        spelling: Vec::new(),
    });
    tokens
}

/// Seed committed token records directly. This keeps the fixture focused on
/// the parser budget instead of re-running the LX chain for 5500 tokens.
fn seed_tokens(bus: &mut CompilerBus, lexemes: &[RawToken]) -> Vec<TokenId> {
    let mut source_bytes = Vec::new();
    let mut spans = Vec::with_capacity(lexemes.len());
    for (index, lexeme) in lexemes.iter().enumerate() {
        if index != 0 {
            source_bytes.push(b' ');
        }
        let start = source_bytes.len() as u64;
        source_bytes.extend_from_slice(&lexeme.spelling);
        spans.push((start, source_bytes.len() as u64));
    }
    source_bytes.push(b'\n');

    let source_name = bus.intern_name(b"sprint_t20.c").unwrap();
    let source = bus.alloc_source(source_name, source_bytes).unwrap();
    let limits = bus.limits();
    let mut tokens = Vec::with_capacity(lexemes.len());

    for (lexeme, (start, end)) in lexemes.iter().zip(spans) {
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
        let pp_kind = match lexeme.kind {
            TokenKind::Keyword | TokenKind::Identifier => PpTokenKind::Identifier,
            TokenKind::Punctuator => PpTokenKind::Punctuator,
            TokenKind::Integer => PpTokenKind::PpNumber,
            TokenKind::Eof => PpTokenKind::Eof,
        };
        let pp_spelling = if lexeme.kind == TokenKind::Eof {
            Vec::new()
        } else {
            lexeme.spelling.clone()
        };
        let pp_token = bus
            .arenas
            .pp_tokens
            .alloc(
                PpTokenRecord {
                    kind: pp_kind,
                    span,
                    spelling: pp_spelling,
                },
                &limits,
            )
            .unwrap();
        let name = match lexeme.kind {
            TokenKind::Keyword | TokenKind::Identifier => {
                Some(bus.intern_name(&lexeme.spelling).unwrap())
            }
            TokenKind::Punctuator | TokenKind::Integer | TokenKind::Eof => None,
        };
        let token = bus
            .arenas
            .tokens
            .alloc(
                TokenRecord {
                    kind: lexeme.kind,
                    span,
                    name,
                    pp_token,
                },
                &limits,
            )
            .unwrap();
        tokens.push(token);
    }
    tokens
}

/// Commit `count` unrelated leaf nodes so the next parse batch predicts from
/// a non-zero base when requested.
fn seed_base_nodes(bus: &mut CompilerBus, token: TokenId, count: u32) {
    let limits = bus.limits();
    for _ in 0..count {
        bus.arenas
            .nodes
            .alloc(
                NodeRecord {
                    kind: NodeKind::Specifiers,
                    parent: None,
                    children: Vec::new(),
                    first_token: token,
                    last_token: token,
                    name: None,
                    literal: None,
                },
                &limits,
            )
            .unwrap();
    }
}

fn parse_fixture(lexemes: &[RawToken], base_nodes: u32) -> (CompilerBus, TaskId, TickReport) {
    let mut bus = new_bus();
    install_pa(&mut bus);
    let tokens = seed_tokens(&mut bus, lexemes);
    seed_base_nodes(&mut bus, tokens[0], base_nodes);

    let refs: Vec<RecordRef> = tokens.iter().map(|id| RecordRef::Token(*id)).collect();
    let task = bus
        .bootstrap_task(TaskDraft {
            kind: TaskKind::PARSE_TU,
            owner: PA_TU_CHIP,
            parent: None,
            payload: Payload::from_refs(refs),
            continuation: None,
        })
        .unwrap();
    let report = tick(&mut bus, &workers());
    (bus, task, report)
}

fn node(bus: &CompilerBus, index: u32) -> NodeRecord {
    bus.arenas
        .nodes
        .get(NodeId::from_index(index))
        .unwrap()
        .clone()
}

/// Validate the committed tree without trusting the parser's own indexes.
fn assert_committed_tree(bus: &CompilerBus, plain: &[usize], unary: &[usize]) {
    assert_eq!(plain.len(), FUNCTION_COUNT);
    assert_eq!(unary.len(), FUNCTION_COUNT);

    let start = BASE_NODES as usize;
    let end = start + AST_BUDGET;
    assert_eq!(bus.arenas.nodes.allocated() as usize, end);
    let token_count = bus.arenas.tokens.allocated();

    let root = node(bus, BASE_NODES);
    assert_eq!(root.kind, NodeKind::TranslationUnit);
    assert_eq!(root.parent, None);
    assert_eq!(root.first_token, TokenId::from_index(0));
    assert_eq!(root.last_token, TokenId::from_index(token_count - 1));

    let mut parents: Vec<Option<u32>> = vec![None; end];
    let mut children: Vec<Vec<u32>> = vec![Vec::new(); end];
    let mut translation_units = 0usize;
    let mut function_definitions = 0usize;
    let mut specifiers = 0usize;
    let mut declarators = 0usize;
    let mut compounds = 0usize;
    let mut returns = 0usize;
    let mut assignments = 0usize;
    let mut identifiers = 0usize;
    let mut unary_minuses = 0usize;

    for index in start..end {
        let record = node(bus, index as u32);
        assert!(record.first_token.index() < token_count);
        assert!(record.last_token.index() < token_count);
        assert!(record.first_token.index() <= record.last_token.index());
        parents[index] = record.parent.map(|id| id.index());
        children[index] = record.children.iter().map(|id| id.index()).collect();
        match record.kind {
            NodeKind::TranslationUnit => translation_units += 1,
            NodeKind::FunctionDefinition => function_definitions += 1,
            NodeKind::Specifiers => specifiers += 1,
            NodeKind::Declarator => declarators += 1,
            NodeKind::Compound => compounds += 1,
            NodeKind::Return => returns += 1,
            NodeKind::Assignment => assignments += 1,
            NodeKind::Identifier => identifiers += 1,
            NodeKind::UnaryMinus => unary_minuses += 1,
            other => panic!("unexpected node kind {other:?} at node {index}"),
        }
    }

    let plain_total: usize = plain.iter().sum();
    let unary_total: usize = unary.iter().sum();
    assert_eq!(translation_units, 1);
    assert_eq!(function_definitions, FUNCTION_COUNT);
    assert_eq!(specifiers, FUNCTION_COUNT);
    assert_eq!(declarators, FUNCTION_COUNT);
    assert_eq!(compounds, FUNCTION_COUNT);
    assert_eq!(returns, FUNCTION_COUNT);
    assert_eq!(assignments, plain_total + unary_total);
    assert_eq!(
        identifiers,
        2 * (plain_total + unary_total) + FUNCTION_COUNT
    );
    assert_eq!(unary_minuses, unary_total);
    assert_eq!(
        translation_units
            + function_definitions
            + specifiers
            + declarators
            + compounds
            + returns
            + assignments
            + identifiers
            + unary_minuses,
        AST_BUDGET
    );

    assert_eq!(parents[start], None);
    for index in start + 1..end {
        let parent = parents[index].expect("non-root node has no parent");
        assert!(
            (start..end).contains(&(parent as usize)),
            "parent {parent} of node {index} is outside the same batch"
        );
    }
    for index in start..end {
        let mut unique = BTreeSet::new();
        for &child in &children[index] {
            assert!(
                (start..end).contains(&(child as usize)),
                "child {child} of node {index} is outside the same batch"
            );
            assert!(unique.insert(child), "node {index} repeats child {child}");
            assert_eq!(
                parents[child as usize],
                Some(index as u32),
                "child {child} does not point back at parent {index}"
            );
        }
    }

    let mut seen = vec![false; end];
    let mut stack = vec![BASE_NODES];
    while let Some(index) = stack.pop() {
        assert!(!seen[index as usize], "cycle or repeat at node {index}");
        seen[index as usize] = true;
        for &child in &children[index as usize] {
            stack.push(child);
        }
    }
    assert!(
        seen[start..].iter().all(|visited| *visited),
        "same-batch tree has an unreachable node"
    );

    assert_eq!(root.children.len(), FUNCTION_COUNT);
    for (function_index, function_id) in root.children.iter().enumerate() {
        let function = node(bus, function_id.index());
        assert_eq!(function.kind, NodeKind::FunctionDefinition);
        assert_eq!(function.children.len(), 3);
        let function_specifiers = node(bus, function.children[0].index());
        let declarator = node(bus, function.children[1].index());
        let compound = node(bus, function.children[2].index());
        assert_eq!(function_specifiers.kind, NodeKind::Specifiers);
        assert_eq!(declarator.kind, NodeKind::Declarator);
        assert_eq!(compound.kind, NodeKind::Compound);
        assert_eq!(
            declarator.name,
            bus.intern.lookup(format!("f{function_index}").as_bytes())
        );
        assert!(declarator.children.is_empty());
        assert_eq!(
            compound.children.len(),
            plain[function_index] + unary[function_index] + 1
        );
        let last = compound
            .children
            .last()
            .expect("compound must end with return");
        assert_eq!(node(bus, last.index()).kind, NodeKind::Return);
    }
}

#[test]
fn ast_budget_accepts_4096_nodes_with_no_dangling_same_batch_links() {
    let (plain, unary) = accepted_plans();
    assert_eq!(expected_node_count(&plain, &unary), AST_BUDGET);

    let lexemes = plan_tokens(&plain, &unary);
    let (bus, task, report) = parse_fixture(&lexemes, BASE_NODES);

    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert!(
                commit.failed.is_empty(),
                "unexpected parse failure: {:?}",
                commit.failed
            );
            assert_eq!(commit.completed.len(), 1);
            assert_eq!(commit.completed[0].0, task);
            let appended = commit
                .appended
                .iter()
                .map(|(_, reference)| match reference {
                    RecordRef::Node(id) => id.index(),
                    other => panic!("expected node reference, got {other:?}"),
                })
                .collect::<Vec<_>>();
            assert_eq!(
                appended,
                (BASE_NODES..BASE_NODES + AST_BUDGET as u32).collect::<Vec<_>>()
            );
        }
        other => panic!("expected parse execution, got {other:?}"),
    }

    assert_eq!(bus.arenas.nodes.allocated(), BASE_NODES + AST_BUDGET as u32);
    assert_committed_tree(&bus, &plain, &unary);
}

#[test]
fn ast_budget_rejects_4097_nodes_without_partial_commit() {
    let (plain, unary) = rejected_plans();
    assert_eq!(expected_node_count(&plain, &unary), AST_BUDGET + 1);

    let lexemes = plan_tokens(&plain, &unary);
    let (bus, task, report) = parse_fixture(&lexemes, 0);

    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 0);
            assert_eq!(commit.failed.len(), 1);
            assert_eq!(commit.failed[0].0, task);
            assert!(
                commit.appended.is_empty(),
                "a rejected parse must append no nodes"
            );
            let diagnostic = bus.arenas.diagnostics.get(commit.failed[0].1).unwrap();
            assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
            assert!(
                diagnostic.message.contains("4096-node AST budget"),
                "unexpected diagnostic: {}",
                diagnostic.message
            );
        }
        other => panic!("expected parse execution, got {other:?}"),
    }

    assert_eq!(
        bus.arenas.nodes.allocated(),
        0,
        "rejection must publish no partial AST nodes"
    );
}
