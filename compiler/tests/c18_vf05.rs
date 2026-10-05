// ============================================================================
// c18_vf05.rs — Wave 2 (`/18`) VF05 token-AST invariant slice acceptance.
//
// Covers the frozen closure: the `verification.token_ast_invariant` kind
// (local 18), kind-to-stage assignment (stage 2), the VF05 manifest (reads
// only, no allowlist writes), the M1 syntax contract (parent/children
// reciprocity, token ranges contained with ordered non-overlapping
// siblings, per-kind child counts, `Declarator` name + `IntLiteral`
// literal with origin token, unique trailing EOF), malformed/broken
// negatives, stage/layer agreement, and the `/18` hash participation.
// Chain inputs come from the real PP→LX→PA slices; wider syntax stays
// deferred.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord, SpanRecord};
use cc_silicon_compiler::chips::{
    handler_for, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip, Vf05Chip, Worker,
    WorkerRegistry,
};
use cc_silicon_compiler::ids::{NodeId, RecordRef, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_vf05_slice_kind, stage_of, FieldPath, PA_TU_CHIP, VF05_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::Snapshot;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry};

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
    use cc_silicon_compiler::manifest::StoreSchema;
    bus.kinds = TaskKindRegistry::vf05_slice();
    // No new record families: the VF05 slice reuses the PP-slice schema.
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
    bus.routing
        .register(
            TaskKind::LEX_INTERN,
            cc_silicon_compiler::manifest::LX_INTERN_CHIP,
            2,
        )
        .unwrap();
    bus.routing
        .register(
            TaskKind::LEX_CLASSIFY,
            cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
            2,
        )
        .unwrap();
    bus.routing
        .register(
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
            2,
        )
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_TU, PA_TU_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::VERIFICATION_TOKEN_AST_INVARIANT, VF05_CHIP, 2)
        .unwrap();
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

fn seed_source(bus: &mut CompilerBus) -> cc_silicon_compiler::ids::SourceId {
    let mut raw = b"int main(void){return 2+3;}".to_vec();
    raw.push(b'\n');
    let name = bus.intern_name(b"main.c").unwrap();
    bus.alloc_source(name, raw).unwrap()
}

fn seed_pp_layer(
    bus: &mut CompilerBus,
    source: cc_silicon_compiler::ids::SourceId,
) -> Vec<cc_silicon_compiler::ids::PpTokenId> {
    let limits = bus.limits();
    let spans: Vec<(u64, u64)> = vec![
        (0, 3),
        (4, 8),
        (8, 9),
        (9, 13),
        (13, 14),
        (14, 15),
        (15, 21),
        (22, 23),
        (23, 24),
        (24, 25),
        (25, 26),
        (26, 27),
        (28, 28),
    ];
    let mut span_ids = Vec::new();
    for (start, end) in spans {
        span_ids.push(
            bus.arenas
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
                .unwrap(),
        );
    }
    let tokens: Vec<(PpTokenKind, usize, &[u8])> = vec![
        (PpTokenKind::Identifier, 0, b"int"),
        (PpTokenKind::Identifier, 1, b"main"),
        (PpTokenKind::Punctuator, 2, b"("),
        (PpTokenKind::Identifier, 3, b"void"),
        (PpTokenKind::Punctuator, 4, b")"),
        (PpTokenKind::Punctuator, 5, b"{"),
        (PpTokenKind::Identifier, 6, b"return"),
        (PpTokenKind::PpNumber, 7, b"2"),
        (PpTokenKind::Punctuator, 8, b"+"),
        (PpTokenKind::PpNumber, 9, b"3"),
        (PpTokenKind::Punctuator, 10, b";"),
        (PpTokenKind::Punctuator, 11, b"}"),
        (PpTokenKind::Eof, 12, b""),
    ];
    let mut ids = Vec::new();
    for (kind, span, spelling) in tokens {
        ids.push(
            bus.arenas
                .pp_tokens
                .alloc(
                    PpTokenRecord {
                        kind,
                        span: span_ids[span],
                        spelling: spelling.to_vec(),
                    },
                    &limits,
                )
                .unwrap(),
        );
    }
    ids
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

/// Handles needed downstream.
struct Chain {
    bus: CompilerBus,
    tu: NodeId,
}

/// Run the PP→LX→PA chain from seeded PP fixtures (stops at the TU).
fn full_chain() -> Chain {
    let mut bus = new_bus();
    install_vf05(&mut bus);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let workers = workers();
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_INTERN,
        cc_silicon_compiler::manifest::LX_INTERN_CHIP,
        Payload::from_refs(refs),
    );
    tick(&mut bus, &workers);
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
        Payload::from_refs(refs),
    );
    let report = tick(&mut bus, &workers);
    let tokens: Vec<TokenId> = match report.outcome {
        TickOutcome::Executed { commit, .. } => commit
            .appended
            .iter()
            .map(|(_, reference)| match reference {
                RecordRef::Token(id) => *id,
                other => panic!("expected token ref, got {other:?}"),
            })
            .collect(),
        other => panic!("expected executed, got {other:?}"),
    };
    for index in [7, 9] {
        let pp_token = bus.arenas.tokens.get(tokens[index]).unwrap().pp_token;
        bootstrap(
            &mut bus,
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
            Payload::from_refs(vec![
                RecordRef::Token(tokens[index]),
                RecordRef::PpToken(pp_token),
            ]),
        );
        tick(&mut bus, &workers);
    }
    bootstrap(
        &mut bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(tokens.iter().map(|id| RecordRef::Token(*id)).collect()),
    );
    tick(&mut bus, &workers);
    Chain {
        bus,
        tu: NodeId::from_index(0),
    }
}

#[test]
fn vf05_kind_stage_and_allowlist_are_frozen() {
    use cc_silicon_compiler::manifest::StoreSchema;
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT.group(),
        TaskGroup::VERIFICATION
    );
    assert_eq!(TaskKind::VERIFICATION_TOKEN_AST_INVARIANT.local(), 18);
    assert!(is_vf05_slice_kind(
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT
    ));
    assert!(!is_vf05_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_vf05_slice_kind(TaskKind::VERIFICATION_TYPED_INVARIANT));
    assert!(!is_vf05_slice_kind(TaskKind::VERIFICATION_IR_INTERPRET));
    assert_eq!(
        stage_of(TaskKind::VERIFICATION_TOKEN_AST_INVARIANT),
        Some(2)
    );
    let registry = TaskKindRegistry::vf05_slice();
    assert_eq!(registry.len(), 31);
    assert_eq!(
        registry
            .lookup(TaskKind::VERIFICATION_TOKEN_AST_INVARIANT)
            .unwrap()
            .name,
        "verification.token_ast_invariant"
    );
    // A read-only verifier: no writes, so no allowlist rows are required.
    assert!(Vf05Chip.manifest().writes.is_empty());
    let _ = StoreSchema::pp_slice();
    let _ = StoreId::Lex;
    let _ = FieldPath::new(StoreId::Parse, "nodes");
}

#[test]
fn vf05_accepts_m1_tree() {
    let mut chain = full_chain();
    let workers = workers();
    assert_eq!(chain.bus.arenas.nodes.allocated(), 9);
    assert_eq!(chain.bus.arenas.tokens.allocated(), 13);
    bootstrap(
        &mut chain.bus,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.tu)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            let value = chain
                .bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .value
                .clone();
            assert_eq!(value, cc_silicon_compiler::task::ResultValue::Ack);
        }
        other => panic!("expected executed, got {other:?}"),
    }
    // The verifier appends nothing.
    assert_eq!(chain.bus.arenas.nodes.allocated(), 9);
    assert_eq!(chain.bus.arenas.tokens.allocated(), 13);
}

#[test]
fn vf05_rejects_non_tu_and_malformed_inputs() {
    let mut chain = full_chain();
    let workers = workers();
    // A function-definition node is not a TU root.
    bootstrap(
        &mut chain.bus,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(1))]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // Empty payload.
    bootstrap(
        &mut chain.bus,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::empty(),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // Dangling node reference.
    bootstrap(
        &mut chain.bus,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::from_refs(vec![RecordRef::Node(NodeId::from_index(99))]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

#[test]
fn vf05_rejects_broken_contract() {
    use cc_silicon_compiler::chips::{project_vf05_input, Vf05Chip};
    use cc_silicon_compiler::task::TaskState;
    let mut chain = full_chain();
    let task = bootstrap(
        &mut chain.bus,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.tu)]),
    );
    let good = project_vf05_input(&chain.bus, task).expect("M1 shape projects");
    // The projection carries dispatch state (`Ready` pre-tick); promote.
    let mut good = good;
    good.state = TaskState::Running;
    let fails = |input: &cc_silicon_compiler::chips::Vf05Input| {
        assert!(
            matches!(
                Vf05Chip.compute(input)[0],
                cc_silicon_compiler::task::Proposal::Fail { .. }
            ),
            "expected a loud failure"
        );
    };
    // Declarator without its required name.
    let mut no_name = good.clone();
    let declarator = no_name
        .tree
        .iter_mut()
        .find(|(id, _)| *id == NodeId::from_index(3))
        .expect("declarator listed");
    declarator.1.name = None;
    fails(&no_name);
    // IntLiteral without its required literal.
    let mut no_literal = good.clone();
    let leaf = no_literal
        .tree
        .iter_mut()
        .find(|(id, _)| *id == NodeId::from_index(7))
        .expect("leaf listed");
    leaf.1.literal = None;
    fails(&no_literal);
    // Swapped `BinaryAdd` operands break sibling order.
    let mut swapped = good.clone();
    let binary = swapped
        .tree
        .iter_mut()
        .find(|(id, _)| *id == NodeId::from_index(6))
        .expect("binary listed");
    binary.1.children.reverse();
    fails(&swapped);
    // A non-reciprocal parent link breaks the walk.
    let mut bad_parent = good.clone();
    let binary = bad_parent
        .tree
        .iter_mut()
        .find(|(id, _)| *id == NodeId::from_index(6))
        .expect("binary listed");
    binary.1.parent = Some(NodeId::from_index(4));
    fails(&bad_parent);
    // The good shape acknowledges.
    match &Vf05Chip.compute(&good)[..] {
        [cc_silicon_compiler::task::Proposal::Complete { value, .. }] => {
            assert_eq!(*value, cc_silicon_compiler::task::ResultValue::Ack)
        }
        other => panic!("expected single ack, got {other:?}"),
    }
}

#[test]
fn vf05_stage_layer_and_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut bus = new_bus();
    install_vf05(&mut bus);
    // Re-point the VF05 route at a wrong layer on a scratch bus.
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::vf05_slice();
    mismatched.schema = StoreSchema::pp_slice();
    mismatched
        .registrations
        .register(Vf05Chip.manifest(), &mismatched.schema, &mismatched.kinds)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::VERIFICATION_TOKEN_AST_INVARIANT, VF05_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // A manifest claiming the new kind is rejected against the stale
    // pre-`/18` registry that does not know it.
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            Vf05Chip.manifest(),
            &StoreSchema::pp_slice(),
            &TaskKindRegistry::vf12_slice(),
        )
        .is_err());
    let _ = FieldPath::new(StoreId::Lex, "tokens");
    use cc_silicon_compiler::task::StoreId;
}

#[test]
fn vf05_snapshot_replay_is_deterministic() {
    let run = || {
        let mut chain = full_chain();
        let workers = workers();
        bootstrap(
            &mut chain.bus,
            TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
            VF05_CHIP,
            Payload::from_refs(vec![RecordRef::Node(chain.tu)]),
        );
        tick(&mut chain.bus, &workers);
        Snapshot::capture(&chain.bus)
    };
    assert_eq!(run(), run());
}
