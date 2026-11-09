// ============================================================================
// c45_const_ty.rs — B2-02 `/42` tagged, typed constant acceptance.
//
// Pins the new `ConstRecord { ty, value }` shape, the closed integer tag,
// canonical body codec, fold type resolution from committed facts, T09's
// `ir.op-immediate-type` failure, and the `/42` contract fields. Negative
// cases cover missing/ambiguous facts and non-integer types; no test fakes a
// successful type.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, ConstRecord, ConstValue, EffectMask, IntRank, IntegerConst, LiteralRecord,
    NodeRecord, SemRecord, TaggedProposal, TypeKind, TypeRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{drive_task, FoldChip, IrFunctionChip, Worker, WorkerRegistry};
use cc_silicon_compiler::codec::CodecError;
use cc_silicon_compiler::commit::commit_proposals;
use cc_silicon_compiler::contract::{CONST_RECORD_FIELDS, CONST_VALUE_NAMES, INTEGER_CONST_FIELDS};
use cc_silicon_compiler::ids::{ConstId, LiteralId, NodeId, RecordRef, TaskId, TypeId};
use cc_silicon_compiler::manifest::{ChipManifest, FieldPath, StoreSchema, G1_FOLD_CHIP};
use cc_silicon_compiler::snapshot::{
    decode_const, encode_const, LiteralKind, LiteralSuffix, Lx08CandidateType,
};
use cc_silicon_compiler::task::{
    Payload, Proposal, StoreId, TaskDraft, TaskKind, TaskKindRegistry, TaskState,
};

fn int_type() -> TypeRecord {
    TypeRecord {
        kind: TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        },
    }
}

fn literal(magnitude: u8, spelling: &[u8]) -> LiteralRecord {
    LiteralRecord {
        token: None,
        kind: LiteralKind::Integer,
        radix: 10,
        suffix: LiteralSuffix::None,
        value: vec![magnitude],
        negative: false,
        spelling: spelling.to_vec(),
        candidate_type: Lx08CandidateType::Int,
    }
}

fn install_fold(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::m1_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    bus.registrations
        .register(FoldChip.manifest(), &bus.schema, &bus.kinds)
        .unwrap();
    bus.routing
        .register(TaskKind::CONSTANT_CONST_FOLD, G1_FOLD_CHIP, 2)
        .unwrap();
}

fn seed_literals(bus: &mut CompilerBus) -> (LiteralId, LiteralId) {
    let limits = bus.limits();
    let two = bus
        .arenas
        .literals
        .alloc(literal(2, b"2"), &limits)
        .unwrap();
    let three = bus
        .arenas
        .literals
        .alloc(literal(3, b"3"), &limits)
        .unwrap();
    (two, three)
}

fn seed_node(bus: &mut CompilerBus) -> NodeId {
    let limits = bus.limits();
    bus.arenas
        .nodes
        .alloc(
            NodeRecord {
                kind: cc_silicon_compiler::bus::NodeKind::BinaryAdd,
                parent: None,
                children: vec![],
                first_token: cc_silicon_compiler::ids::TokenId::from_index(0),
                last_token: cc_silicon_compiler::ids::TokenId::from_index(0),
                name: None,
                literal: None,
            },
            &limits,
        )
        .unwrap()
}

fn seed_type(bus: &mut CompilerBus, body: TypeRecord) -> TypeId {
    bus.arenas.types.alloc(body, &bus.limits()).unwrap()
}

fn seed_sem(bus: &mut CompilerBus, node: NodeId, ty: TypeId) {
    bus.arenas
        .sem
        .alloc(
            SemRecord {
                node,
                ty,
                category: ValueCategory::NonLvalue,
                effects: EffectMask(0),
            },
            &bus.limits(),
        )
        .unwrap();
}

fn fold_task(bus: &mut CompilerBus, node: NodeId, lhs: LiteralId, rhs: LiteralId) -> TaskId {
    let task = bus
        .bootstrap_task(TaskDraft {
            kind: TaskKind::CONSTANT_CONST_FOLD,
            payload: Payload::from_refs(vec![
                RecordRef::Node(node),
                RecordRef::Literal(lhs),
                RecordRef::Literal(rhs),
            ]),
            owner: G1_FOLD_CHIP,
            parent: None,
            continuation: None,
        })
        .unwrap();
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    task
}

fn drive(bus: &CompilerBus, task: TaskId) -> Vec<TaggedProposal> {
    let mut workers = WorkerRegistry::new();
    workers.register(FoldChip).unwrap();
    drive_task(bus, task, &workers).unwrap()
}

fn expect_fail(proposals: &[TaggedProposal], needle: &str) {
    assert_eq!(proposals.len(), 1, "expected one terminal proposal");
    match &proposals[0].proposal {
        Proposal::Fail { diagnostic, .. } => assert!(
            diagnostic.message.contains(needle),
            "diagnostic `{}` misses `{needle}`",
            diagnostic.message
        ),
        other => panic!("expected Fail, got {other:?}"),
    }
}

#[test]
fn fold_commits_the_committed_sem_type() {
    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let (two, three) = seed_literals(&mut bus);
    let node = seed_node(&mut bus);
    let int = seed_type(&mut bus, int_type());
    seed_sem(&mut bus, node, int);
    let task = fold_task(&mut bus, node, two, three);
    let proposals = drive(&bus, task);
    let report = commit_proposals(&mut bus, proposals).unwrap();
    assert_eq!(report.appended.len(), 1);
    let committed = bus.arenas.consts.get(ConstId::from_index(0)).unwrap();
    assert_eq!(committed.ty, int);
    match &committed.value {
        ConstValue::Integer(value) => {
            assert_eq!(value.magnitude, vec![5]);
            assert!(!value.negative);
        }
        ConstValue::Float(_) | ConstValue::Address(_) => {
            panic!("integer fold produced a non-integer payload")
        }
    }
}

#[test]
fn fold_rejects_missing_sem_fact() {
    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let (two, three) = seed_literals(&mut bus);
    let node = seed_node(&mut bus);
    let task = fold_task(&mut bus, node, two, three);
    expect_fail(&drive(&bus, task), "missing constant type");
    assert_eq!(bus.arenas.consts.allocated(), 0);
}

#[test]
fn fold_rejects_ambiguous_sem_fact() {
    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let (two, three) = seed_literals(&mut bus);
    let node = seed_node(&mut bus);
    let int = seed_type(&mut bus, int_type());
    seed_sem(&mut bus, node, int);
    seed_sem(&mut bus, node, int);
    let task = fold_task(&mut bus, node, two, three);
    expect_fail(&drive(&bus, task), "missing constant type");
    assert_eq!(bus.arenas.consts.allocated(), 0);
}

#[test]
fn fold_rejects_non_integer_type() {
    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let (two, three) = seed_literals(&mut bus);
    let node = seed_node(&mut bus);
    let void = seed_type(
        &mut bus,
        TypeRecord {
            kind: TypeKind::Void,
        },
    );
    seed_sem(&mut bus, node, void);
    let task = fold_task(&mut bus, node, two, three);
    expect_fail(&drive(&bus, task), "integer type");
    assert_eq!(bus.arenas.consts.allocated(), 0);
}

#[test]
fn immediate_type_mismatch_fails_t09() {
    let input = cc_silicon_compiler::chips::IrFunctionInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        symbol: cc_silicon_compiler::ids::SymbolId::from_index(0),
        linkage: cc_silicon_compiler::bus::Linkage::External,
        signature: TypeId::from_index(3),
        function_sem: cc_silicon_compiler::ids::SemId::from_index(0),
        int: TypeId::from_index(0),
        constant_ty: TypeId::from_index(1),
        return_ty: TypeId::from_index(1),
        constant_record_ty: TypeId::from_index(2),
        constant: ConstId::from_index(0),
        local: None,
        local_expr: None,
        local_assignments: vec![],
        functions_allocated: 0,
        blocks_allocated: 0,
        values_allocated: 0,
        instructions_allocated: 0,
    };
    match IrFunctionChip.compute(&input).as_slice() {
        [Proposal::Fail { diagnostic, .. }] => {
            assert!(
                diagnostic.message.contains("immediate type"),
                "{}",
                diagnostic.message
            );
        }
        other => panic!("expected a single failure, got {other:?}"),
    }
}

#[test]
fn const_codec_round_trips_and_rejects_noncanonical_payloads() {
    let record = ConstRecord {
        ty: TypeId::from_index(7),
        value: ConstValue::Integer(IntegerConst {
            magnitude: vec![0x01, 0x00],
            negative: false,
        }),
    };
    assert_eq!(decode_const(&encode_const(&record)).unwrap(), record);

    let mut bad_tag = encode_const(&record);
    bad_tag[4] = 9;
    assert_eq!(decode_const(&bad_tag), Err(CodecError::InvalidTag(9)));

    let negative_zero = ConstRecord {
        ty: TypeId::from_index(7),
        value: ConstValue::Integer(IntegerConst {
            magnitude: vec![0],
            negative: true,
        }),
    };
    assert!(matches!(
        decode_const(&encode_const(&negative_zero)),
        Err(CodecError::Unsupported(_))
    ));
}

#[test]
fn contract_fields_name_the_tagged_shape() {
    assert_eq!(CONST_RECORD_FIELDS, &["ty", "value"]);
    assert_eq!(CONST_VALUE_NAMES, &["integer", "float", "address"]);
    assert_eq!(INTEGER_CONST_FIELDS, &["magnitude", "negative"]);
}

#[test]
fn manifest_declares_the_type_facts_it_reads() {
    let manifest: ChipManifest = FoldChip.manifest();
    assert!(manifest
        .reads
        .contains(&FieldPath::new(StoreId::Sem, "records")));
    assert!(manifest
        .reads
        .contains(&FieldPath::new(StoreId::Types, "records")));
    assert!(!manifest
        .writes
        .contains(&FieldPath::new(StoreId::Types, "records")));
}
