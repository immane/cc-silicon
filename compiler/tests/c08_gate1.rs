// ============================================================================
// c08_gate1.rs — Gate 1 (`/7`) M1 const-fold slice plus `/8` worker
// integration acceptance (G1-CL-01)
//
// Covers the frozen closure: slice task kinds, the strict kind→shape
// `ConstantRequest` decode convention (`/8`), legality routing without a
// new `ResultValue` variant, authorized typed `AppendRecords`
// materialization for the `Literal`/`Const` families, unified total-record
// preflight, per-arena capacity, predicted-reference checks for both
// `Record` and `Records` carriers, the constant bit-budget overflow,
// kind-to-stage assignment, the wave-gated store-owner allowlist,
// stage/layer agreement, snapshot bodies, tick-lifecycle integration via
// `propagate_with`/`clock_tick_with`, and the `/8` hash participation.
//
// The `G1-CL-01` chain seeds frozen committed fixtures (literals, node)
// and drives one T08-style fold through the real commit path. It is
// explicitly NOT `M1-CL-05`: that fixture requires real upstream
// artifacts (T04/T05/T07 production) and stays the Wave-2 acceptance.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, ConstRecord, LiteralRecord, TaggedProposal};
use cc_silicon_compiler::commit::{commit_proposals, CommitError};
use cc_silicon_compiler::ids::{ChipId, LiteralId, NodeId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_gate1_slice_kind, stage_of, BackendClass, Capability, ChipManifest, ChipPhase, FieldPath,
    ManifestError, ManifestRegistry, ManifestRegistryError, StoreSchema, STAGE_ASSIGNMENT,
    STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::records::{G1DraftBody, RecordDraft};
use cc_silicon_compiler::snapshot::{
    decode_const, decode_literal_record, encode_const, encode_literal_record, LiteralKind,
    LiteralSuffix, Lx08CandidateType, Snapshot,
};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    AppendBatch, ConstExprOp, ConstLegality, ConstantRequest, ConstantResult, DraftRef, Payload,
    Proposal, RequiredKind, ResultValue, StoreId, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry,
    TaskState,
};

/// A committed M1-subset literal fixture: decimal `Integer`, no suffix,
/// magnitude `magnitude`, unsigned, spelled `spelling`, candidate `Int`.
fn fixture_literal(magnitude: u8, spelling: &[u8]) -> LiteralRecord {
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

fn draft(kind: TaskKind, owner: ChipId, payload: Payload) -> TaskDraft {
    TaskDraft {
        kind,
        payload,
        owner,
        parent: None,
        continuation: None,
    }
}

fn running_task(bus: &mut CompilerBus, kind: TaskKind, owner: ChipId, payload: Payload) -> TaskId {
    let id = bus.bootstrap_task(draft(kind, owner, payload)).unwrap();
    bus.arenas.tasks.get_mut(id).unwrap().state = TaskState::Running;
    id
}

fn tag(task: TaskId, chip: ChipId, proposal: Proposal) -> TaggedProposal {
    TaggedProposal {
        chip,
        task,
        proposal,
    }
}

fn fold_chip() -> ChipId {
    cc_silicon_compiler::manifest::G1_FOLD_CHIP
}

/// Install the fold chip's manifest and route on a bus (both are required
/// before any fold append commits: the commit authorizes appends against
/// the registered manifest, and the driver resolves the route).
fn install_fold(bus: &mut CompilerBus) {
    use cc_silicon_compiler::chips::{FoldChip, Worker};
    bus.kinds = TaskKindRegistry::m1_slice();
    bus.schema = StoreSchema::m1_slice();
    let manifest = FoldChip.manifest();
    // Fresh test buses only; installing twice is a test bug.
    if bus.registrations.get(fold_chip()).is_none() {
        bus.registrations
            .register(
                manifest,
                &StoreSchema::m1_slice(),
                &TaskKindRegistry::m1_slice(),
            )
            .unwrap();
    }
    if bus.routing.lookup(TaskKind::CONSTANT_CONST_FOLD).is_none() {
        bus.routing
            .register(TaskKind::CONSTANT_CONST_FOLD, fold_chip(), 2)
            .unwrap();
    }
}

/// Seed the `G1-CL-01` fixtures: committed literals `2` and `3` plus a
/// committed expression node. Returns `(two, three, node)`.
fn seed_g1_fixtures(bus: &mut CompilerBus) -> (LiteralId, LiteralId, NodeId) {
    let limits = bus.limits();
    let two = bus
        .arenas
        .literals
        .alloc(fixture_literal(2, b"2"), &limits)
        .unwrap();
    let three = bus
        .arenas
        .literals
        .alloc(fixture_literal(3, b"3"), &limits)
        .unwrap();
    let node = bus.arenas.nodes.alloc(&limits).unwrap();
    (two, three, node)
}

/// Fixture-side fold simulation: single-byte unsigned magnitudes only.
/// The real algorithm is the future T08 chip's; the frozen envelope is
/// "checked addition, exactly one record, no refold".
fn fold_bytes(lhs: &[u8], rhs: &[u8]) -> Vec<u8> {
    assert_eq!(lhs.len(), 1);
    assert_eq!(rhs.len(), 1);
    vec![lhs[0]
        .checked_add(rhs[0])
        .expect("G1-CL-01 fixture overflow")]
}

#[test]
fn slice_kinds_are_frozen_with_group_local_codes() {
    assert_eq!(
        TaskKind::SEMANTIC_CONST_EVAL_LITERAL.group(),
        TaskGroup::SEMANTIC
    );
    assert_eq!(TaskKind::SEMANTIC_CONST_EVAL_LITERAL.local(), 16);
    assert_eq!(
        TaskKind::SEMANTIC_CONST_EVAL_BINARY.group(),
        TaskGroup::SEMANTIC
    );
    assert_eq!(TaskKind::SEMANTIC_CONST_EVAL_BINARY.local(), 17);
    assert_eq!(
        TaskKind::CONSTANT_CONST_FOLD.group(),
        TaskGroup::CONSTANT_LAYOUT_INIT
    );
    assert_eq!(TaskKind::CONSTANT_CONST_FOLD.local(), 16);
    for kind in [
        TaskKind::SEMANTIC_CONST_EVAL_LITERAL,
        TaskKind::SEMANTIC_CONST_EVAL_BINARY,
        TaskKind::CONSTANT_CONST_FOLD,
    ] {
        assert!(is_gate1_slice_kind(kind));
        assert!(!kind.is_foundation());
        assert!(!kind.is_reserved_local());
    }
    assert!(!is_gate1_slice_kind(TaskKind::CONTROL_NOOP));

    let registry = TaskKindRegistry::m1_slice();
    // Foundation (4) plus the three slice kinds, all frozen.
    assert_eq!(registry.len(), 7);
    for (kind, name) in [
        (
            TaskKind::SEMANTIC_CONST_EVAL_LITERAL,
            "semantic.const_eval_literal",
        ),
        (
            TaskKind::SEMANTIC_CONST_EVAL_BINARY,
            "semantic.const_eval_binary",
        ),
        (
            TaskKind::CONSTANT_CONST_FOLD,
            "constant_layout_init.const_fold",
        ),
    ] {
        let entry = registry.lookup(kind).expect("slice kind registered");
        assert_eq!(entry.name, name);
        assert_eq!(entry.group, kind.group());
        assert_eq!(entry.status, cc_silicon_compiler::task::KindStatus::Frozen);
        assert!(entry.kind.local() >= 16);
    }
}

#[test]
fn request_decode_accepts_m1_conventions() {
    use cc_silicon_compiler::ids::TokenId;
    let literal = LiteralId::from_index(4);
    let node = NodeId::from_index(9);

    // Literal form: exactly one literal reference.
    let request = ConstantRequest::decode(
        TaskKind::SEMANTIC_CONST_EVAL_LITERAL,
        &Payload::from_refs(vec![RecordRef::Literal(literal)]),
    )
    .unwrap();
    assert_eq!(
        request,
        ConstantRequest::Literal {
            literal,
            required_kind: RequiredKind::IntegerConstantExpression,
        }
    );

    // Binary form: node then two literals in source order.
    let other = LiteralId::from_index(5);
    let request = ConstantRequest::decode(
        TaskKind::SEMANTIC_CONST_EVAL_BINARY,
        &Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(literal),
            RecordRef::Literal(other),
        ]),
    )
    .unwrap();
    assert_eq!(
        request,
        ConstantRequest::Binary {
            node,
            op: ConstExprOp::Add,
            lhs: literal,
            rhs: other,
            required_kind: RequiredKind::IntegerConstantExpression,
        }
    );

    // The fold kind accepts either forwarded shape (the T07 requester
    // forwards identical payload refs to its const_fold child).
    let request = ConstantRequest::decode(
        TaskKind::CONSTANT_CONST_FOLD,
        &Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(literal),
            RecordRef::Literal(other),
        ]),
    )
    .unwrap();
    assert!(matches!(request, ConstantRequest::Binary { .. }));
    let request = ConstantRequest::decode(
        TaskKind::CONSTANT_CONST_FOLD,
        &Payload::from_refs(vec![RecordRef::Literal(literal)]),
    )
    .unwrap();
    assert!(matches!(request, ConstantRequest::Literal { .. }));

    // Strict kind→shape rule (`/8`): the T07 request kinds never
    // reinterpret the other shape. A binary payload on the literal kind
    // (and vice versa) is `Arity`, not a silent cross-decode.
    assert!(matches!(
        ConstantRequest::decode(
            TaskKind::SEMANTIC_CONST_EVAL_LITERAL,
            &Payload::from_refs(vec![
                RecordRef::Node(node),
                RecordRef::Literal(literal),
                RecordRef::Literal(other),
            ]),
        ),
        Err(cc_silicon_compiler::task::RequestError::Arity { .. })
    ));
    assert!(matches!(
        ConstantRequest::decode(
            TaskKind::SEMANTIC_CONST_EVAL_BINARY,
            &Payload::from_refs(vec![RecordRef::Literal(literal)]),
        ),
        Err(cc_silicon_compiler::task::RequestError::Arity { .. })
    ));
    assert!(matches!(
        ConstantRequest::decode(TaskKind::CONTROL_NOOP, &Payload::empty()),
        Err(cc_silicon_compiler::task::RequestError::UnexpectedKind { .. })
    ));
    assert!(matches!(
        ConstantRequest::decode(TaskKind::SEMANTIC_CONST_EVAL_LITERAL, &Payload::empty()),
        Err(cc_silicon_compiler::task::RequestError::Arity { .. })
    ));
    assert!(matches!(
        ConstantRequest::decode(
            TaskKind::SEMANTIC_CONST_EVAL_LITERAL,
            &Payload::from_refs(vec![RecordRef::Source(
                cc_silicon_compiler::ids::SourceId::from_index(0)
            )])
        ),
        Err(cc_silicon_compiler::task::RequestError::Family { .. })
    ));
    assert!(matches!(
        ConstantRequest::decode(
            TaskKind::SEMANTIC_CONST_EVAL_BINARY,
            &Payload::from_refs(vec![
                RecordRef::Token(TokenId::from_index(0)),
                RecordRef::Literal(literal),
                RecordRef::Literal(other),
            ])
        ),
        Err(cc_silicon_compiler::task::RequestError::Family { position: 0, .. })
    ));
}

#[test]
fn result_routing_maps_legality_without_new_variant() {
    use cc_silicon_compiler::ids::ResultId;
    let task = TaskId::from_index(0);
    let value = RecordRef::Result(ResultId::from_index(0));

    // Legal completes with the generic record carrier (wire tag 2).
    let proposal = ConstantResult {
        value,
        legality: ConstLegality::Legal,
    }
    .route(task);
    assert_eq!(proposal.wire_tag(), 1);
    assert!(matches!(
        proposal,
        Proposal::Complete {
            value: ResultValue::Record(_),
            ..
        }
    ));

    // Non-legal outcomes fail with structured diagnostics, never records.
    for legality in [
        ConstLegality::NotConstantExpression,
        ConstLegality::Unsupported,
    ] {
        let proposal = ConstantResult { value, legality }.route(task);
        assert_eq!(proposal.wire_tag(), 2);
        assert!(matches!(proposal, Proposal::Fail { .. }));
    }
}

#[test]
fn append_materializes_const_record_end_to_end() {
    // G1-CL-01: fixtures in, one fold through the real commit path.
    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let (two, three, node) = seed_g1_fixtures(&mut bus);
    let payload = Payload::from_refs(vec![
        RecordRef::Node(node),
        RecordRef::Literal(two),
        RecordRef::Literal(three),
    ]);
    let request = ConstantRequest::decode(TaskKind::SEMANTIC_CONST_EVAL_BINARY, &payload).unwrap();
    let (lhs, rhs) = match request {
        ConstantRequest::Binary { lhs, rhs, .. } => (lhs, rhs),
        ConstantRequest::Literal { .. } => panic!("expected binary request"),
    };
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        payload,
    );

    // Fixture-side fold (stands in for the future T08 chip).
    let left = bus.arenas.literals.get(lhs).unwrap().clone();
    let right = bus.arenas.literals.get(rhs).unwrap().clone();
    let folded = ConstRecord {
        value: fold_bytes(&left.value, &right.value),
        negative: false,
    };
    let body = G1DraftBody::Const(folded.clone());
    let report = commit_proposals(
        &mut bus,
        vec![
            tag(
                task,
                fold_chip(),
                Proposal::AppendRecords {
                    task,
                    batch: AppendBatch {
                        records: vec![DraftRef(0)]
                            .into_iter()
                            .map(|index| RecordDraft {
                                family: body.family(),
                                index,
                            })
                            .collect(),
                        bodies: vec![body],
                    },
                },
            ),
            tag(
                task,
                fold_chip(),
                ConstantResult {
                    value: RecordRef::Const(cc_silicon_compiler::ids::ConstId::from_index(0)),
                    legality: ConstLegality::Legal,
                }
                .route(task),
            ),
        ],
    )
    .unwrap();

    // Exactly one ConstRecord committed; the completed result names it.
    assert_eq!(report.appended.len(), 1);
    let committed = bus
        .arenas
        .consts
        .get(cc_silicon_compiler::ids::ConstId::from_index(0))
        .unwrap();
    assert_eq!(*committed, folded);
    assert_eq!(committed.value, vec![5]);
    assert!(matches!(
        bus.arenas.tasks.get(task).unwrap().state,
        TaskState::Completed(_)
    ));
    let consumed =
        cc_silicon_compiler::commit::consume_result(&mut bus, report.completed[0].1).unwrap();
    assert_eq!(
        consumed,
        ResultValue::Record(RecordRef::Const(
            cc_silicon_compiler::ids::ConstId::from_index(0)
        ))
    );
}

#[test]
fn append_rejects_mismatched_and_out_of_subset_batches() {
    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::empty(),
    );
    let handle = RecordDraft {
        family: cc_silicon_compiler::ids::RecordFamily::Const,
        index: DraftRef(0),
    };

    // Bodies/records length mismatch.
    let batch = AppendBatch {
        records: vec![handle],
        bodies: vec![],
    };
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(
                task,
                fold_chip(),
                Proposal::AppendRecords { task, batch }
            )]
        ),
        Err(CommitError::InvalidPatchShape { .. })
    ));

    // Body family mismatch (literal body against a const handle).
    let batch = AppendBatch {
        records: vec![handle],
        bodies: vec![G1DraftBody::Literal(fixture_literal(2, b"2"))],
    };
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(
                task,
                fold_chip(),
                Proposal::AppendRecords { task, batch }
            )]
        ),
        Err(CommitError::InvalidPatchShape { .. })
    ));

    // Literals outside the M1 exercised subset never materialize.
    for literal in [
        LiteralRecord {
            kind: LiteralKind::Character,
            ..fixture_literal(65, b"'A'")
        },
        LiteralRecord {
            radix: 16,
            ..fixture_literal(2, b"0x2")
        },
        LiteralRecord {
            suffix: LiteralSuffix::U,
            ..fixture_literal(2, b"2U")
        },
    ] {
        let batch = AppendBatch {
            records: vec![RecordDraft {
                family: cc_silicon_compiler::ids::RecordFamily::Literal,
                index: DraftRef(0),
            }],
            bodies: vec![G1DraftBody::Literal(literal)],
        };
        assert!(matches!(
            commit_proposals(
                &mut bus,
                vec![tag(
                    task,
                    fold_chip(),
                    Proposal::AppendRecords { task, batch }
                )]
            ),
            Err(CommitError::InvalidPatchShape { .. })
        ));
    }
    // Nothing was committed by any rejected batch.
    assert_eq!(bus.arenas.consts.allocated(), 0);
    assert_eq!(bus.arenas.literals.allocated(), 0);
}

#[test]
fn append_enforces_per_arena_capacity() {
    let mut bus = CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        Vec::new(),
        Limits {
            max_records_per_arena: 2,
            ..Limits::fixture()
        },
    ));
    install_fold(&mut bus);
    // Fill the consts arena to its configured per-arena bound.
    let limits = bus.limits();
    for _ in 0..limits.max_records_per_arena {
        bus.arenas
            .consts
            .alloc(
                ConstRecord {
                    value: vec![0],
                    negative: false,
                },
                &limits,
            )
            .unwrap();
    }
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::empty(),
    );
    let batch = AppendBatch {
        records: vec![RecordDraft {
            family: cc_silicon_compiler::ids::RecordFamily::Const,
            index: DraftRef(0),
        }],
        bodies: vec![G1DraftBody::Const(ConstRecord {
            value: vec![5],
            negative: false,
        })],
    };
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![
                tag(task, fold_chip(), Proposal::AppendRecords { task, batch }),
                tag(
                    task,
                    fold_chip(),
                    Proposal::Complete {
                        task,
                        value: ResultValue::Ack,
                    }
                ),
            ]
        ),
        Err(CommitError::Capacity(_))
    ));
}

#[test]
fn stage_assignment_covers_foundation_and_slice() {
    use cc_silicon_compiler::manifest::check_stage_layer_agreement;
    assert_eq!(STAGE_ASSIGNMENT.len(), 7);
    for (kind, stage) in STAGE_ASSIGNMENT {
        assert_eq!(stage_of(*kind), Some(*stage));
        assert!((*stage as usize) < Limits::fixture().stage_queue_bound.len());
    }
    // Chain order: control < request < fold.
    assert_eq!(stage_of(TaskKind::CONTROL_NOOP), Some(0));
    assert_eq!(stage_of(TaskKind::SEMANTIC_CONST_EVAL_BINARY), Some(1));
    assert_eq!(stage_of(TaskKind::CONSTANT_CONST_FOLD), Some(2));
    // No silent default: unlisted kinds have no stage.
    assert_eq!(stage_of(TaskKind::new(TaskGroup::LEX, 16).unwrap()), None);

    // Registration rejects a manifest whose kind has no stage row.
    let mut kinds = TaskKindRegistry::m1_slice();
    let custom = TaskKind::new(TaskGroup::LEX, 16).unwrap();
    kinds
        .register(
            custom,
            "lex.custom",
            TaskGroup::LEX,
            cc_silicon_compiler::task::KindStatus::GroupOwned,
        )
        .unwrap();
    let manifest = ChipManifest {
        id: ChipId(11),
        chip_name: "CustomChip",
        group: TaskGroup::LEX,
        task_kinds: vec![custom],
        reads: vec![],
        writes: vec![],
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["compiler/tests/c08_gate1.rs"],
        dependencies: vec![],
    };
    let mut registry = ManifestRegistry::new();
    assert!(matches!(
        registry.register(manifest, &StoreSchema::m1_slice(), &kinds),
        Err(ManifestRegistryError::Registry(
            ManifestError::StageUnassigned { .. }
        ))
    ));

    // Stage/layer agreement follows the routing table for routed kinds.
    let fold = ChipManifest {
        id: fold_chip(),
        chip_name: "FoldChip",
        group: TaskGroup::CONSTANT_LAYOUT_INIT,
        task_kinds: vec![TaskKind::CONSTANT_CONST_FOLD],
        reads: vec![FieldPath::new(StoreId::Lex, "literals")],
        writes: vec![FieldPath::new(StoreId::Constants, "records")],
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["compiler/tests/c08_gate1.rs"],
        dependencies: vec![],
    };
    let mut bus = CompilerBus::default();
    bus.routing
        .register(TaskKind::CONSTANT_CONST_FOLD, fold_chip(), 2)
        .unwrap();
    assert!(check_stage_layer_agreement(&fold, &bus.routing).is_ok());
    bus.routing = Default::default();
    bus.routing
        .register(TaskKind::CONSTANT_CONST_FOLD, fold_chip(), 3)
        .unwrap();
    assert!(matches!(
        check_stage_layer_agreement(&fold, &bus.routing),
        Err(ManifestError::StageLayerMismatch { .. })
    ));
}

#[test]
fn allowlist_authorizes_fold_chip_only() {
    // The seed holds exactly the fold-chip row; `tasks.ready` stays writer-free.
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 1);
    assert!(!STORE_OWNER_ALLOWLIST
        .iter()
        .any(|&(_, store, field, _)| store == StoreId::Tasks && field == "queue.ready"));

    let kinds = TaskKindRegistry::m1_slice();
    let schema = StoreSchema::m1_slice();
    let fold = || ChipManifest {
        id: fold_chip(),
        chip_name: "FoldChip",
        group: TaskGroup::CONSTANT_LAYOUT_INIT,
        task_kinds: vec![TaskKind::CONSTANT_CONST_FOLD],
        reads: vec![FieldPath::new(StoreId::Lex, "literals")],
        writes: vec![FieldPath::new(StoreId::Constants, "records")],
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["compiler/tests/c08_gate1.rs"],
        dependencies: vec![],
    };
    let mut registry = ManifestRegistry::new();
    registry.register(fold(), &schema, &kinds).unwrap();

    // A different chip ID with the same write is rejected.
    let mut impostor = fold();
    impostor.id = ChipId(9);
    impostor.chip_name = "ImpostorChip";
    assert!(matches!(
        registry.register(impostor, &schema, &kinds),
        Err(ManifestRegistryError::Registry(
            ManifestError::StoreOwnerViolation { .. }
        ))
    ));

    // An unlisted write on a slice kind is rejected even for the right chip.
    let mut registry = ManifestRegistry::new();
    let mut extra = fold();
    extra
        .writes
        .push(FieldPath::new(StoreId::Diagnostics, "entries"));
    assert!(matches!(
        registry.register(extra, &schema, &kinds),
        Err(ManifestRegistryError::Registry(
            ManifestError::StoreOwnerViolation { .. }
        ))
    ));
}

#[test]
fn snapshot_carries_literal_and_const_bodies_with_replay() {
    // Body round-trips.
    let literal = fixture_literal(2, b"2");
    assert_eq!(
        decode_literal_record(&encode_literal_record(&literal)).unwrap(),
        literal
    );
    let folded = ConstRecord {
        value: vec![5],
        negative: false,
    };
    assert_eq!(decode_const(&encode_const(&folded)).unwrap(), folded);

    // Replay: two identical G1-CL-01 runs hash identically, and a run with
    // no records hashes differently.
    fn run_chain() -> Vec<u8> {
        let mut bus = CompilerBus::default();
        install_fold(&mut bus);
        let (two, three, node) = seed_g1_fixtures(&mut bus);
        let task = running_task(
            &mut bus,
            TaskKind::CONSTANT_CONST_FOLD,
            fold_chip(),
            Payload::from_refs(vec![
                RecordRef::Node(node),
                RecordRef::Literal(two),
                RecordRef::Literal(three),
            ]),
        );
        let left = bus.arenas.literals.get(two).unwrap().clone();
        let right = bus.arenas.literals.get(three).unwrap().clone();
        commit_proposals(
            &mut bus,
            vec![
                tag(
                    task,
                    fold_chip(),
                    Proposal::AppendRecords {
                        task,
                        batch: AppendBatch {
                            records: vec![RecordDraft {
                                family: cc_silicon_compiler::ids::RecordFamily::Const,
                                index: DraftRef(0),
                            }],
                            bodies: vec![G1DraftBody::Const(ConstRecord {
                                value: fold_bytes(&left.value, &right.value),
                                negative: false,
                            })],
                        },
                    },
                ),
                tag(
                    task,
                    fold_chip(),
                    ConstantResult {
                        value: RecordRef::Const(cc_silicon_compiler::ids::ConstId::from_index(0)),
                        legality: ConstLegality::Legal,
                    }
                    .route(task),
                ),
            ],
        )
        .unwrap();
        Snapshot::capture(&bus).bytes().to_vec()
    }
    assert_eq!(run_chain(), run_chain());
    assert_ne!(
        run_chain(),
        Snapshot::capture(&CompilerBus::default()).bytes().to_vec()
    );
}

#[test]
fn m1_slice_schema_declares_gate1_fields() {
    let foundation = StoreSchema::foundation();
    assert!(!foundation.has_field(&FieldPath::new(StoreId::Lex, "literals")));
    assert!(!foundation.has_field(&FieldPath::new(StoreId::Constants, "records")));
    let slice = StoreSchema::m1_slice();
    assert!(slice.has_field(&FieldPath::new(StoreId::Lex, "literals")));
    assert!(slice.has_field(&FieldPath::new(StoreId::Constants, "records")));
    // Foundation fields are preserved, not replaced.
    assert!(slice.has_field(&FieldPath::new(StoreId::Tasks, "queue.ready")));
}

#[test]
fn contract_hash_covers_gate1_section() {
    use cc_silicon_compiler::contract::{
        compute_contract_hash, FrozenSchema, CONST_EXPR_OP_NAMES, CONST_LEGALITY_NAMES,
        CONST_RECORD_FIELDS, CONTRACT_HASH, CONTRACT_VERSION, LITERAL_KIND_NAMES,
        LITERAL_RECORD_FIELDS, LITERAL_SUFFIX_NAMES, LX08_CANDIDATE_NAMES, NORMATIVE_RULES,
        REQUIRED_KIND_NAMES,
    };
    assert_eq!(CONTRACT_VERSION, "t01-c01-c06/9");
    assert_eq!(compute_contract_hash(), CONTRACT_HASH);
    assert_eq!(
        LITERAL_RECORD_FIELDS,
        &[
            "token",
            "kind",
            "radix",
            "suffix",
            "value",
            "negative",
            "spelling",
            "candidate_type"
        ]
    );
    assert_eq!(CONST_RECORD_FIELDS, &["value", "negative"]);
    assert_eq!(LITERAL_KIND_NAMES, &["integer", "character", "string"]);
    assert_eq!(LITERAL_SUFFIX_NAMES, &["none", "U", "L", "UL", "LL", "ULL"]);
    assert_eq!(LX08_CANDIDATE_NAMES, &["Int"]);
    assert_eq!(CONST_EXPR_OP_NAMES, &["add"]);
    assert_eq!(REQUIRED_KIND_NAMES, &["integer_constant_expression"]);
    assert_eq!(
        CONST_LEGALITY_NAMES,
        &["legal", "not_constant_expression", "unsupported"]
    );
    // `/8` worker-integration rules participate in the hash.
    for rule in [
        "append.authorized-registered-declared",
        "commit.total-budget-unified",
        "commit.predicted-records-checked",
        "request.kind-shape-strict",
        "request.const-fold-forwards-identical-refs",
        "const.budget-enforced-chip-overflow",
        "snapshot.config-encodes-all-bounds",
        "commit.transition-required-per-task",
        "join.await-all-requires-all-terminal",
    ] {
        assert!(NORMATIVE_RULES.contains(&rule), "missing rule `{rule}`");
    }
    // The frozen bytes carry the slice: flipping any of these names or the
    // version changes the hash (presence pins the section, the hash test
    // pins the value).
    let bytes = FrozenSchema::current().encode();
    for marker in [
        "t01-c01-c06/9",
        "semantic.const_eval_literal",
        "semantic.const_eval_binary",
        "constant_layout_init.const_fold",
        "m1-append/1",
    ] {
        assert!(
            bytes
                .windows(marker.len())
                .any(|window| window == marker.as_bytes()),
            "frozen bytes miss `{marker}`"
        );
    }
}

#[test]
fn fold_chip_manifest_registers_and_routes() {
    use cc_silicon_compiler::chips::{FoldChip, Worker};
    use cc_silicon_compiler::manifest::check_stage_layer_agreement;

    let chip = FoldChip;
    let manifest = chip.manifest();
    assert_eq!(manifest.id, fold_chip());
    assert_eq!(manifest.group, TaskGroup::CONSTANT_LAYOUT_INIT);
    assert_eq!(manifest.task_kinds, vec![TaskKind::CONSTANT_CONST_FOLD]);
    let kinds = TaskKindRegistry::m1_slice();
    let schema = StoreSchema::m1_slice();
    let mut registry = ManifestRegistry::new();
    registry.register(manifest, &schema, &kinds).unwrap();

    let mut bus = CompilerBus::default();
    bus.routing
        .register(TaskKind::CONSTANT_CONST_FOLD, fold_chip(), 2)
        .unwrap();
    assert!(check_stage_layer_agreement(&chip.manifest(), &bus.routing).is_ok());
}

#[test]
fn fold_chip_drives_g1_chain_through_driver() {
    use cc_silicon_compiler::chips::{drive_task, FoldChip, WorkerRegistry};

    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let mut workers = WorkerRegistry::new();
    workers.register(FoldChip).unwrap();

    let (two, three, node) = seed_g1_fixtures(&mut bus);
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(two),
            RecordRef::Literal(three),
        ]),
    );
    let tagged = drive_task(&bus, task, &workers).unwrap();
    assert_eq!(tagged.len(), 2);
    let report = commit_proposals(&mut bus, tagged).unwrap();

    // The real chip folded 2 + 3 through the commit path: exactly one
    // ConstRecord(5), task completed naming it.
    assert_eq!(report.appended.len(), 1);
    let committed = bus
        .arenas
        .consts
        .get(cc_silicon_compiler::ids::ConstId::from_index(0))
        .unwrap();
    assert_eq!(committed.value, vec![5]);
    assert!(!committed.negative);
    assert!(matches!(
        bus.arenas.tasks.get(task).unwrap().state,
        TaskState::Completed(_)
    ));
    // Deterministic replay of the driven chain.
    let first = Snapshot::capture(&bus).hash();
    let mut again = CompilerBus::default();
    install_fold(&mut again);
    let (two, three, node) = seed_g1_fixtures(&mut again);
    let retry = running_task(
        &mut again,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(two),
            RecordRef::Literal(three),
        ]),
    );
    let tagged = drive_task(&again, retry, &workers).unwrap();
    commit_proposals(&mut again, tagged).unwrap();
    assert_eq!(Snapshot::capture(&again).hash(), first);
}

#[test]
fn driver_rejects_unregistered_mismatched_and_duplicate() {
    use cc_silicon_compiler::chips::{drive_task, DriveError, FoldChip, WorkerRegistry};

    let mut bus = CompilerBus::default();
    bus.kinds = TaskKindRegistry::m1_slice();
    bus.schema = StoreSchema::m1_slice();
    let workers = WorkerRegistry::new();

    // Unregistered kind: the driver refuses to fabricate work.
    let task = running_task(
        &mut bus,
        TaskKind::SEMANTIC_CONST_EVAL_BINARY,
        fold_chip(),
        Payload::empty(),
    );
    assert!(matches!(
        drive_task(&bus, task, &workers),
        Err(DriveError::NotRegistered { .. })
    ));

    // Routed kind with no worker installed.
    bus.routing
        .register(TaskKind::CONSTANT_CONST_FOLD, fold_chip(), 2)
        .unwrap();
    let fold = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::empty(),
    );
    assert!(matches!(
        drive_task(&bus, fold, &workers),
        Err(DriveError::NoWorker { .. })
    ));

    // Routed kind with a worker but no registered manifest: the driver
    // refuses earlier than the commit's `UnregisteredChip`.
    let mut workers = WorkerRegistry::new();
    workers.register(FoldChip).unwrap();
    assert!(matches!(
        drive_task(&bus, fold, &workers),
        Err(DriveError::UnregisteredChip { .. })
    ));

    // Duplicate worker registration is rejected.
    assert!(matches!(
        workers.register(FoldChip),
        Err(DriveError::DuplicateWorker { .. })
    ));

    // Unknown task.
    assert!(matches!(
        drive_task(&bus, TaskId::from_index(999), &workers),
        Err(DriveError::UnknownTask { .. })
    ));
}

#[test]
fn fold_chip_fails_loudly_on_bad_inputs() {
    use cc_silicon_compiler::chips::{FoldChip, Worker};

    let mut bus = CompilerBus::default();
    bus.kinds = TaskKindRegistry::m1_slice();
    let chip = FoldChip;

    // Wrong payload shape for the kind.
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::empty(),
    );
    let proposals = chip.handle(task, &bus);
    assert!(proposals.iter().any(|proposal| proposal.wire_tag() == 2));

    // Missing literal reference fails loudly (no panic, no silence).
    let (two, _, node) = seed_g1_fixtures(&mut bus);
    let dangling = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(two),
            RecordRef::Literal(cc_silicon_compiler::ids::LiteralId::from_index(99)),
        ]),
    );
    let proposals = chip.handle(dangling, &bus);
    assert!(proposals.iter().any(|proposal| proposal.wire_tag() == 2));
    // Nothing was appended by any failed handling.
    assert_eq!(bus.arenas.consts.allocated(), 0);
}

#[test]
fn append_requires_registered_manifest_and_declared_write() {
    use cc_silicon_compiler::ids::ConstId;

    // No manifest registered: the commit rejects the append even though the
    // tagged chip owns the task.
    let mut bus = CompilerBus::default();
    bus.kinds = TaskKindRegistry::m1_slice();
    bus.schema = StoreSchema::m1_slice();
    bus.routing
        .register(TaskKind::CONSTANT_CONST_FOLD, fold_chip(), 2)
        .unwrap();
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::empty(),
    );
    let batch = AppendBatch {
        records: vec![RecordDraft {
            family: cc_silicon_compiler::ids::RecordFamily::Const,
            index: DraftRef(0),
        }],
        bodies: vec![G1DraftBody::Const(ConstRecord {
            value: vec![5],
            negative: false,
        })],
    };
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(
                task,
                fold_chip(),
                Proposal::AppendRecords { task, batch }
            )]
        ),
        Err(CommitError::UnregisteredChip { .. })
    ));

    // Registered fold chip appending a `Literal` body: the chip declares
    // only `constants.records`, so the per-body write gate rejects.
    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::empty(),
    );
    let batch = AppendBatch {
        records: vec![RecordDraft {
            family: cc_silicon_compiler::ids::RecordFamily::Literal,
            index: DraftRef(0),
        }],
        bodies: vec![G1DraftBody::Literal(fixture_literal(2, b"2"))],
    };
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(
                task,
                fold_chip(),
                Proposal::AppendRecords { task, batch }
            )]
        ),
        Err(CommitError::WriteNotDeclared { .. })
    ));

    // Registered fold chip completing a task of a kind it does not accept:
    // the kind gate rejects before any mutation.
    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let other = running_task(
        &mut bus,
        TaskKind::SEMANTIC_CONST_EVAL_BINARY,
        fold_chip(),
        Payload::empty(),
    );
    let batch = AppendBatch {
        records: vec![RecordDraft {
            family: cc_silicon_compiler::ids::RecordFamily::Const,
            index: DraftRef(0),
        }],
        bodies: vec![G1DraftBody::Const(ConstRecord {
            value: vec![5],
            negative: false,
        })],
    };
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(
                other,
                fold_chip(),
                Proposal::AppendRecords { task: other, batch }
            )]
        ),
        Err(CommitError::TaskKindNotAccepted { .. })
    ));
    let _ = ConstId::from_index(0);
}

#[test]
fn total_record_budget_covers_appends_and_results_together() {
    // One slot left, two records attempted (one append + one result): the
    // unified preflight rejects, and nothing commits.
    use cc_silicon_compiler::chips::{drive_task, FoldChip, WorkerRegistry};

    // Tight bus: budget is exactly current usage + 1 after seeding.
    // Probe the usage on a fixture-budget bus first (configs are immutable
    // after construction, so rebuild under the tight budget next).
    let mut probe = CompilerBus::default();
    install_fold(&mut probe);
    let (two, three, node) = seed_g1_fixtures(&mut probe);
    let _probe_task = running_task(
        &mut probe,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(two),
            RecordRef::Literal(three),
        ]),
    );
    let tight_total = probe.total_records().saturating_add(1);
    // Pin the budget to one slot over current usage: append(1) + result(1)
    // = 2 must fail together (the old split checks passed each half).
    // Rebuild an identical bus under the tight budget (configs are
    // immutable after construction, so rebuild rather than mutate).
    let mut bus = CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        Vec::new(),
        Limits {
            max_records_total: tight_total,
            ..Limits::fixture()
        },
    ));
    install_fold(&mut bus);
    let (two, three, node) = seed_g1_fixtures(&mut bus);
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(two),
            RecordRef::Literal(three),
        ]),
    );
    let mut workers = WorkerRegistry::new();
    workers.register(FoldChip).unwrap();
    let tagged = drive_task(&bus, task, &workers).unwrap();
    assert_eq!(tagged.len(), 2);
    let before_consts = bus.arenas.consts.allocated();
    let outcome = commit_proposals(&mut bus, tagged);
    assert!(
        matches!(
            outcome,
            Err(CommitError::Limit(
                cc_silicon_compiler::limits::LimitError::TotalRecords { .. }
            ))
        ),
        "expected unified total-records reject, got {outcome:?}"
    );
    assert_eq!(bus.arenas.consts.allocated(), before_consts);
    assert!(matches!(
        bus.arenas.tasks.get(task).unwrap().state,
        TaskState::Running
    ));
}

#[test]
fn fold_enforces_const_bit_budget_as_chip_overflow() {
    use cc_silicon_compiler::chips::{const_bits_required, FoldChip, Worker};
    use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticCode};

    assert_eq!(const_bits_required(&[0]), 1);
    assert_eq!(const_bits_required(&[1]), 1);
    assert_eq!(const_bits_required(&[2]), 2);
    assert_eq!(const_bits_required(&[5]), 3);
    assert_eq!(const_bits_required(&[255]), 8);
    assert_eq!(const_bits_required(&[1, 0]), 9);

    // Budget 1 cannot represent 2 + 3 = 5 (needs 3 bits): loud overflow
    // `Fail`, never `Legal`, nothing appended.
    let mut bus = CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        Vec::new(),
        Limits {
            max_const_bits: 1,
            ..Limits::fixture()
        },
    ));
    install_fold(&mut bus);
    let (two, three, node) = seed_g1_fixtures(&mut bus);
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(two),
            RecordRef::Literal(three),
        ]),
    );
    let proposals = FoldChip.handle(task, &bus);
    assert_eq!(proposals.len(), 1);
    match &proposals[0] {
        Proposal::Fail { diagnostic, .. } => {
            assert_eq!(diagnostic.code, DiagnosticCode::CONST_OVERFLOW);
            assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
        }
        other => panic!("expected overflow Fail, got {other:?}"),
    }
    assert_eq!(bus.arenas.consts.allocated(), 0);
}

#[test]
fn unpredicted_record_checks_records_carrier() {
    use cc_silicon_compiler::ids::ConstId;

    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::empty(),
    );
    // `Records` carrying a future-dated const this batch never appends:
    // same loud reject as the `Record` carrier.
    let bad = Proposal::Complete {
        task,
        value: ResultValue::Records(vec![RecordRef::Const(ConstId::from_index(7))]),
    };
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, fold_chip(), bad)]),
        Err(CommitError::UnpredictedRecord { .. })
    ));

    // `Records` carrying this task's own predicted append commits.
    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let task = running_task(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        fold_chip(),
        Payload::empty(),
    );
    let report = commit_proposals(
        &mut bus,
        vec![
            tag(
                task,
                fold_chip(),
                Proposal::AppendRecords {
                    task,
                    batch: AppendBatch {
                        records: vec![RecordDraft {
                            family: cc_silicon_compiler::ids::RecordFamily::Const,
                            index: DraftRef(0),
                        }],
                        bodies: vec![G1DraftBody::Const(ConstRecord {
                            value: vec![5],
                            negative: false,
                        })],
                    },
                },
            ),
            tag(
                task,
                fold_chip(),
                Proposal::Complete {
                    task,
                    value: ResultValue::Records(vec![RecordRef::Const(ConstId::from_index(0))]),
                },
            ),
        ],
    )
    .unwrap();
    assert_eq!(report.appended.len(), 1);
    assert_eq!(report.completed.len(), 1);
}

#[test]
fn fold_runs_through_real_tick_lifecycle() {
    use cc_silicon_compiler::bus::CompilerPins;
    use cc_silicon_compiler::chips::{handler_for, FoldChip, WorkerRegistry};
    use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};

    let mut bus = CompilerBus::default();
    install_fold(&mut bus);
    let mut workers = WorkerRegistry::new();
    workers.register(FoldChip).unwrap();
    let (two, three, node) = seed_g1_fixtures(&mut bus);
    // Enqueue through the public bootstrap (Ready, next-tick visible) —
    // no manual `Running` fixup.
    let task = bus
        .bootstrap_task(draft(
            TaskKind::CONSTANT_CONST_FOLD,
            fold_chip(),
            Payload::from_refs(vec![
                RecordRef::Node(node),
                RecordRef::Literal(two),
                RecordRef::Literal(three),
            ]),
        ))
        .unwrap();
    assert!(matches!(
        bus.arenas.tasks.get(task).unwrap().state,
        TaskState::Ready
    ));
    let shell = RoutingShell::new();
    // `bootstrap_task` is immediately eligible: the first tick dispatches
    // it through the real worker and commits; the second tick is idle.
    let first = shell
        .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
        .unwrap();
    match &first.outcome {
        TickOutcome::Executed {
            task: selected,
            commit,
            ..
        } => {
            assert_eq!(*selected, task);
            assert_eq!(commit.appended.len(), 1);
            assert_eq!(commit.completed.len(), 1);
        }
        other => panic!("expected Executed, got {other:?}"),
    }
    let committed = bus
        .arenas
        .consts
        .get(cc_silicon_compiler::ids::ConstId::from_index(0))
        .unwrap();
    assert_eq!(committed.value, vec![5]);
    assert!(matches!(
        bus.arenas.tasks.get(task).unwrap().state,
        TaskState::Completed(_)
    ));
    let second = shell
        .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
        .unwrap();
    assert!(matches!(second.outcome, TickOutcome::Idle));
    assert_eq!(bus.control.tick, 2);
    assert_eq!(bus.report.len(), 2);
}

#[test]
fn fold_compute_needs_no_bus_beyond_its_projection() {
    use cc_silicon_compiler::chips::{FoldChip, FoldInput};
    use std::collections::BTreeMap;

    // Hand-built projection, no bus at all: the computation is a pure
    // function of its input.
    let chip = FoldChip;
    let mut literals = BTreeMap::new();
    literals.insert(LiteralId::from_index(0), fixture_literal(2, b"2"));
    literals.insert(LiteralId::from_index(1), fixture_literal(3, b"3"));
    let mut nodes_present = std::collections::BTreeSet::new();
    nodes_present.insert(NodeId::from_index(0));
    let input = FoldInput {
        task: TaskId::from_index(0),
        kind: TaskKind::CONSTANT_CONST_FOLD,
        state: TaskState::Running,
        payload: Payload::from_refs(vec![
            RecordRef::Node(NodeId::from_index(0)),
            RecordRef::Literal(LiteralId::from_index(0)),
            RecordRef::Literal(LiteralId::from_index(1)),
        ]),
        literals,
        nodes_present,
        consts_allocated: 0,
        max_const_bits: 128,
    };
    let proposals = chip.compute(&input);
    assert_eq!(proposals.len(), 2);
    assert!(proposals.iter().any(|p| p.wire_tag() == 5));
    assert!(proposals.iter().any(|p| p.wire_tag() == 1));
}
