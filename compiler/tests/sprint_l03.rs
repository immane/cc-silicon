// ============================================================================
// sprint_l03.rs — zero commit/fold provenance (`/55` const handoff).
//
// The source spelling `0` decodes as canonical decimal zero after the L01
// normalization; commit must accept it, FoldChip must preserve the canonical
// `[0]`/non-negative payload, and every `ConstRecord.origins` entry must name
// the exact literal occurrence. Repeated `0` occurrences stay distinct even
// though their value is identical.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, ConstValue, EffectMask, IntRank, NodeKind, NodeRecord, SemRecord, TaggedProposal,
    TokenKind, TypeKind, TypeRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{FoldChip, LxDecodeInput, LxDecodeLiteralChip, Worker};
use cc_silicon_compiler::commit::commit_proposals;
use cc_silicon_compiler::ids::{ConstId, LiteralId, NodeId, RecordRef, TaskId, TokenId, TypeId};
use cc_silicon_compiler::manifest::{StoreSchema, G1_FOLD_CHIP, LX_DECODE_CHIP};
use cc_silicon_compiler::snapshot::{LiteralKind, Lx08CandidateType};
use cc_silicon_compiler::task::{Payload, TaskDraft, TaskKind, TaskKindRegistry, TaskState};

fn install_chain(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::se_function_slice();
    bus.schema = StoreSchema::ir_slice();
    for chip in [&LxDecodeLiteralChip as &dyn Worker, &FoldChip] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
}

fn bootstrap(
    bus: &mut CompilerBus,
    kind: TaskKind,
    owner: cc_silicon_compiler::ids::ChipId,
    payload: Payload,
) -> TaskId {
    bus.bootstrap_task(TaskDraft {
        kind,
        payload,
        owner,
        parent: None,
        continuation: None,
    })
    .unwrap()
}

fn commit_tagged(
    bus: &mut CompilerBus,
    task: TaskId,
    chip: cc_silicon_compiler::ids::ChipId,
    proposals: Vec<cc_silicon_compiler::task::Proposal>,
) -> Vec<(TaskId, RecordRef)> {
    let tagged: Vec<TaggedProposal> = proposals
        .into_iter()
        .map(|proposal| TaggedProposal {
            chip,
            task,
            proposal,
        })
        .collect();
    commit_proposals(bus, tagged).unwrap().appended
}

/// Decode and commit one integer spelling through the real LX commit gate.
fn commit_spelling(bus: &mut CompilerBus, spelling: &[u8]) -> LiteralId {
    let task = bootstrap(
        bus,
        TaskKind::LEX_DECODE_LITERAL,
        LX_DECODE_CHIP,
        Payload::empty(),
    );
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    let input = LxDecodeInput {
        task,
        state: TaskState::Running,
        token: TokenId::from_index(0),
        token_kind: TokenKind::Integer,
        spelling: spelling.to_vec(),
        literals_allocated: bus.arenas.literals.allocated(),
    };
    let proposals = LxDecodeLiteralChip.compute(&input);
    let appended = commit_tagged(bus, task, LX_DECODE_CHIP, proposals);
    assert_eq!(appended.len(), 1, "decode commits one literal");
    match appended[0].1 {
        RecordRef::Literal(id) => id,
        other => panic!("expected literal append, got {other:?}"),
    }
}

fn seed_int(bus: &mut CompilerBus) -> TypeId {
    let limits = bus.limits();
    bus.arenas
        .types
        .alloc(
            TypeRecord {
                kind: TypeKind::Int {
                    rank: IntRank::Int,
                    signed: true,
                },
            },
            &limits,
        )
        .unwrap()
}

fn seed_binary_node(bus: &mut CompilerBus, int: TypeId) -> NodeId {
    let limits = bus.limits();
    let node = bus
        .arenas
        .nodes
        .alloc(
            NodeRecord {
                kind: NodeKind::BinaryAdd,
                parent: None,
                children: vec![],
                first_token: TokenId::from_index(0),
                last_token: TokenId::from_index(0),
                name: None,
                literal: None,
            },
            &limits,
        )
        .unwrap();
    bus.arenas
        .sem
        .alloc(
            SemRecord {
                node,
                ty: int,
                category: ValueCategory::NonLvalue,
                effects: EffectMask(0),
            },
            &limits,
        )
        .unwrap();
    node
}

fn commit_literal_fold(bus: &mut CompilerBus, literal: LiteralId) -> ConstId {
    let task = bootstrap(
        bus,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        Payload::from_refs(vec![RecordRef::Literal(literal)]),
    );
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    let proposals = FoldChip.handle(task, bus);
    let appended = commit_tagged(bus, task, G1_FOLD_CHIP, proposals);
    assert_eq!(appended.len(), 1, "fold commits one constant");
    match appended[0].1 {
        RecordRef::Const(id) => id,
        other => panic!("expected constant append, got {other:?}"),
    }
}

fn commit_binary_fold(
    bus: &mut CompilerBus,
    node: NodeId,
    lhs: LiteralId,
    rhs: LiteralId,
) -> ConstId {
    let task = bootstrap(
        bus,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(lhs),
            RecordRef::Literal(rhs),
        ]),
    );
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    let proposals = FoldChip.handle(task, bus);
    let appended = commit_tagged(bus, task, G1_FOLD_CHIP, proposals);
    assert_eq!(appended.len(), 1, "fold commits one constant");
    match appended[0].1 {
        RecordRef::Const(id) => id,
        other => panic!("expected constant append, got {other:?}"),
    }
}

fn assert_canonical_zero(bus: &CompilerBus, id: ConstId, int: TypeId, origins: &[LiteralId]) {
    let body = bus.arenas.consts.get(id).unwrap();
    assert_eq!(body.ty, int, "folded zero keeps the committed int type");
    let ConstValue::Integer(value) = &body.value else {
        panic!("folded zero must be an integer payload");
    };
    assert_eq!(value.magnitude, vec![0], "zero is exactly one zero byte");
    assert!(!value.negative, "canonical zero is never negative");
    assert!(value.is_canonical(), "folded zero must be canonical");
    assert_eq!(body.origins.as_slice(), origins);
}

#[test]
fn source_zero_commits_as_canonical_decimal_literal() {
    let mut bus = CompilerBus::default();
    install_chain(&mut bus);

    let literal = commit_spelling(&mut bus, b"0");
    assert_eq!(literal, LiteralId::from_index(0));
    let body = bus.arenas.literals.get(literal).unwrap();
    assert_eq!(body.kind, LiteralKind::Integer);
    assert_eq!(body.radix, 10, "canonical zero is decimal, not octal");
    assert_eq!(body.value, vec![0]);
    assert!(!body.negative);
    assert_eq!(
        body.spelling.as_slice(),
        b"0".as_slice(),
        "spelling stays lexical"
    );
    assert_eq!(body.candidate_type, Lx08CandidateType::Int);
}

#[test]
fn folded_literal_zero_preserves_value_and_origin() {
    let mut bus = CompilerBus::default();
    install_chain(&mut bus);
    let literal = commit_spelling(&mut bus, b"0");
    let int = seed_int(&mut bus);

    let constant = commit_literal_fold(&mut bus, literal);
    assert_canonical_zero(&bus, constant, int, &[literal]);
}

#[test]
fn repeated_zero_occurrences_keep_distinct_origins() {
    let mut bus = CompilerBus::default();
    install_chain(&mut bus);
    let first = commit_spelling(&mut bus, b"0");
    let second = commit_spelling(&mut bus, b"0");
    assert_ne!(first, second, "each occurrence is its own literal record");
    let int = seed_int(&mut bus);

    let first_const = commit_literal_fold(&mut bus, first);
    let second_const = commit_literal_fold(&mut bus, second);
    assert_ne!(first_const, second_const);
    assert_canonical_zero(&bus, first_const, int, &[first]);
    assert_canonical_zero(&bus, second_const, int, &[second]);

    let node = seed_binary_node(&mut bus, int);
    let sum = commit_binary_fold(&mut bus, node, first, second);
    assert_canonical_zero(&bus, sum, int, &[first, second]);
    assert_canonical_zero(&bus, first_const, int, &[first]);
    assert_canonical_zero(&bus, second_const, int, &[second]);
}
