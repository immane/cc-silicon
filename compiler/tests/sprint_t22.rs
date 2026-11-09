// ============================================================================
// sprint_t22.rs — adversarial AST child-graph regression.
//
// The committed node arena is a tree. VF05 must reject a dangling child, a
// child shared by two parents, and an ancestor cycle as typed failures. A
// hand-derived canonical `int main(void){return 2;}` tree proves the control
// shape is accepted before the graph is corrupted. Every worker call runs
// behind a finite wall-clock deadline because a missing cycle guard would
// otherwise hang the test process instead of returning a diagnostic.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, LiteralRecord, NodeKind, NodeRecord, TokenKind, TokenRecord,
};
use cc_silicon_compiler::chips::{Vf05Chip, Worker};
use cc_silicon_compiler::ids::{LiteralId, NodeId, PpTokenId, RecordRef, SpanId, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::VF05_CHIP;
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, Proposal, ResultValue, TaskDraft, TaskKind, TaskState};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// A malformed projection can walk forever. The deadline turns that defect
/// into a test failure while leaving the production code untouched.
const HARNESS_DEADLINE: Duration = Duration::from_millis(200);

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn alloc_node(bus: &mut CompilerBus, expected: u32, body: NodeRecord) -> NodeId {
    let limits = bus.limits();
    let id = bus.arenas.nodes.alloc(body, &limits).unwrap();
    assert_eq!(id, NodeId::from_index(expected));
    id
}

/// Canonical source shape: `int main(void){return 2;}`.
///
/// Node IDs are fixed by allocation order:
/// 0 TU, 1 function, 2 specifiers, 3 declarator, 4 compound, 5 return,
/// 6 integer literal. Token 7 is the literal's origin; token 10 is EOF.
fn valid_fixture() -> (CompilerBus, TaskId) {
    let mut bus = new_bus();
    let name = bus.intern_name(b"main").unwrap();

    let limits = bus.limits();
    for index in 0..=10u32 {
        let eof = index == 10;
        let token = bus
            .arenas
            .tokens
            .alloc(
                TokenRecord {
                    kind: if eof {
                        TokenKind::Eof
                    } else {
                        TokenKind::Identifier
                    },
                    span: SpanId::from_index(0),
                    name: if eof { None } else { Some(name) },
                    pp_token: PpTokenId::from_index(0),
                },
                &limits,
            )
            .unwrap();
        assert_eq!(token, TokenId::from_index(index));
    }

    let literal = bus
        .arenas
        .literals
        .alloc(
            LiteralRecord {
                token: Some(TokenId::from_index(7)),
                kind: LiteralKind::Integer,
                radix: 10,
                suffix: LiteralSuffix::None,
                value: vec![2],
                negative: false,
                spelling: b"2".to_vec(),
                candidate_type: Lx08CandidateType::Int,
            },
            &limits,
        )
        .unwrap();
    assert_eq!(literal, LiteralId::from_index(0));

    let tu = alloc_node(
        &mut bus,
        0,
        NodeRecord {
            kind: NodeKind::TranslationUnit,
            parent: None,
            children: vec![NodeId::from_index(1)],
            first_token: TokenId::from_index(0),
            last_token: TokenId::from_index(9),
            name: None,
            literal: None,
        },
    );
    alloc_node(
        &mut bus,
        1,
        NodeRecord {
            kind: NodeKind::FunctionDefinition,
            parent: Some(tu),
            children: vec![
                NodeId::from_index(2),
                NodeId::from_index(3),
                NodeId::from_index(4),
            ],
            first_token: TokenId::from_index(0),
            last_token: TokenId::from_index(9),
            name: None,
            literal: None,
        },
    );
    alloc_node(
        &mut bus,
        2,
        NodeRecord {
            kind: NodeKind::Specifiers,
            parent: Some(NodeId::from_index(1)),
            children: vec![],
            first_token: TokenId::from_index(0),
            last_token: TokenId::from_index(0),
            name: None,
            literal: None,
        },
    );
    alloc_node(
        &mut bus,
        3,
        NodeRecord {
            kind: NodeKind::Declarator,
            parent: Some(NodeId::from_index(1)),
            children: vec![],
            first_token: TokenId::from_index(1),
            last_token: TokenId::from_index(4),
            name: Some(name),
            literal: None,
        },
    );
    alloc_node(
        &mut bus,
        4,
        NodeRecord {
            kind: NodeKind::Compound,
            parent: Some(NodeId::from_index(1)),
            children: vec![NodeId::from_index(5)],
            first_token: TokenId::from_index(5),
            last_token: TokenId::from_index(9),
            name: None,
            literal: None,
        },
    );
    alloc_node(
        &mut bus,
        5,
        NodeRecord {
            kind: NodeKind::Return,
            parent: Some(NodeId::from_index(4)),
            children: vec![NodeId::from_index(6)],
            first_token: TokenId::from_index(6),
            last_token: TokenId::from_index(8),
            name: None,
            literal: None,
        },
    );
    alloc_node(
        &mut bus,
        6,
        NodeRecord {
            kind: NodeKind::IntLiteral,
            parent: Some(NodeId::from_index(5)),
            children: vec![],
            first_token: TokenId::from_index(7),
            last_token: TokenId::from_index(7),
            name: None,
            literal: Some(literal),
        },
    );

    assert_eq!(bus.arenas.nodes.allocated(), 7);
    assert_eq!(bus.arenas.tokens.allocated(), 11);
    assert_eq!(bus.arenas.literals.allocated(), 1);

    let task = bus
        .bootstrap_task(TaskDraft {
            kind: TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
            owner: VF05_CHIP,
            parent: None,
            payload: Payload::from_refs(vec![RecordRef::Node(tu)]),
            continuation: None,
        })
        .unwrap();
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    (bus, task)
}

/// Run VF05 in a detached thread so an infinite child walk becomes a finite
/// test failure. The bus is moved into the worker; the test never needs it
/// afterwards.
fn bounded_handle(bus: CompilerBus, task: TaskId, case: &str) -> Vec<Proposal> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let proposals = Vf05Chip.handle(task, &bus);
        let _ = sender.send(proposals);
    });
    receiver
        .recv_timeout(HARNESS_DEADLINE)
        .unwrap_or_else(|_| panic!("VF05 {case} did not terminate within {HARNESS_DEADLINE:?}"))
}

fn expect_ack(proposals: &[Proposal]) {
    match proposals {
        [Proposal::Complete {
            value: ResultValue::Ack,
            ..
        }] => {}
        other => panic!("expected one Ack, got {other:?}"),
    }
}

fn expect_fail(task: TaskId, proposals: &[Proposal]) {
    match proposals {
        [Proposal::Fail {
            task: failed_task,
            diagnostic,
        }] => {
            assert_eq!(*failed_task, task);
            assert!(
                !diagnostic.message.is_empty(),
                "a malformed AST must carry a diagnostic message"
            );
        }
        other => panic!("expected one typed failure, got {other:?}"),
    }
}

#[test]
fn vf05_rejects_cyclic_shared_and_dangling_ast_children() {
    // Hand-derived control: the unmodified tree is a legal `/55` function.
    let (bus, task) = valid_fixture();
    expect_ack(&bounded_handle(bus, task, "valid control"));

    // A Return child that is not committed must fail during projection.
    let (mut bus, task) = valid_fixture();
    bus.arenas
        .nodes
        .get_mut(NodeId::from_index(5))
        .unwrap()
        .children = vec![NodeId::from_index(99)];
    expect_fail(task, &bounded_handle(bus, task, "dangling child"));

    // The literal leaf is now reachable from two parents: Specifiers (2) and
    // Return (5). A tree cannot share a child, so VF05 must reject it.
    let (mut bus, task) = valid_fixture();
    bus.arenas
        .nodes
        .get_mut(NodeId::from_index(2))
        .unwrap()
        .children = vec![NodeId::from_index(6)];
    expect_fail(task, &bounded_handle(bus, task, "shared child"));

    // Return (5) points back to its ancestor Compound (4), creating a cycle
    // in the committed child graph. Projection must terminate with failure.
    let (mut bus, task) = valid_fixture();
    bus.arenas
        .nodes
        .get_mut(NodeId::from_index(5))
        .unwrap()
        .children = vec![NodeId::from_index(4)];
    expect_fail(task, &bounded_handle(bus, task, "cyclic child"));
}
