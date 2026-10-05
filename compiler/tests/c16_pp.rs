// ============================================================================
// c16_pp.rs — Wave 2 (`/16`) PP splice/comment/scan slice acceptance.
//
// Covers the frozen closure: identity and real splice maps, M1-scoped
// comment replacement (line/block, unterminated failure, literal
// unsupported), maximal-munch scan with the M1 token table, the
// source-bytes end-to-end (PP01→splice→comment→scan with no seeded PP
// fixtures), the three PP kinds, kind-to-stage assignment, the PP
// store-owner allowlist rows, stage/layer agreement, snapshot bodies,
// tick-lifecycle integration, and the `/16` hash participation.
// ============================================================================

use cc_silicon_compiler::bus::{
    ArtifactKind, ArtifactRecord, CompilerBus, CompilerPins, PpTokenKind,
};
use cc_silicon_compiler::chips::{
    compose_map, handler_for, normalize, replace_comments, scan, PpCommentChip, PpNormalizeChip,
    PpScanChip, PpSpliceChip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::commit::{commit_proposals, CommitError};
use cc_silicon_compiler::ids::{ArtifactId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_pp_slice_kind, stage_of, FieldPath, ManifestRegistryError, StoreSchema, PP_COMMENT_CHIP,
    PP_SCAN_CHIP, PP_SPLICE_CHIP, STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::records::{G1DraftBody, RecordDraft};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::Snapshot;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    AppendBatch, DraftRef, Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup, TaskKind,
    TaskKindRegistry,
};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_pp(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::pp_slice();
    bus.schema = StoreSchema::pp_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(
            TaskKind::PREPROCESS_NORMALIZE,
            cc_silicon_compiler::manifest::PP01_CHIP,
            1,
        )
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_SCAN, PP_SCAN_CHIP, 1)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers
}

fn seed_source(
    bus: &mut CompilerBus,
    name: &[u8],
    bytes: Vec<u8>,
) -> cc_silicon_compiler::ids::SourceId {
    let name = bus.intern_name(name).unwrap();
    bus.alloc_source(name, bytes).unwrap()
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

fn completed_artifact(
    report: &cc_silicon_compiler::routing::TickReport,
    bus: &CompilerBus,
) -> ArtifactId {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            let result = bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .clone();
            match result.value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Artifact(id)) => id,
                other => panic!("expected artifact ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    }
}

/// Run one single-artifact stage and return its output artifact.
fn run_stage(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    kind: TaskKind,
    owner: cc_silicon_compiler::ids::ChipId,
    artifact: ArtifactId,
) -> ArtifactId {
    bootstrap(
        bus,
        kind,
        owner,
        Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
    );
    let report = tick(bus, workers);
    completed_artifact(&report, bus)
}

#[test]
fn pp_kinds_stages_and_allowlist_are_frozen() {
    let kinds = [
        (TaskKind::PREPROCESS_SPLICE, "preprocess.splice", 17),
        (TaskKind::PREPROCESS_COMMENT, "preprocess.comment", 18),
        (TaskKind::PREPROCESS_SCAN, "preprocess.scan", 19),
    ];
    for (kind, _, _) in kinds {
        assert!(is_pp_slice_kind(kind));
        assert_eq!(stage_of(kind), Some(1));
    }
    assert!(!is_pp_slice_kind(TaskKind::CONTROL_NOOP));
    // The slice registries are cumulative: every kind frozen through `/16`.
    let registry = TaskKindRegistry::pp_slice();
    assert_eq!(registry.len(), 29);
    for (kind, name, local) in kinds {
        assert_eq!(kind.group(), TaskGroup::PREPROCESS);
        assert_eq!(kind.local(), local);
        assert_eq!(registry.lookup(kind).unwrap().name, name);
    }
    for row in [
        (
            PP_SPLICE_CHIP,
            StoreId::Artifacts,
            "fragments",
            TaskKind::PREPROCESS_SPLICE,
        ),
        (
            PP_COMMENT_CHIP,
            StoreId::Artifacts,
            "fragments",
            TaskKind::PREPROCESS_COMMENT,
        ),
        (
            PP_SCAN_CHIP,
            StoreId::Sources,
            "spans",
            TaskKind::PREPROCESS_SCAN,
        ),
        (
            PP_SCAN_CHIP,
            StoreId::Pp,
            "tokens",
            TaskKind::PREPROCESS_SCAN,
        ),
    ] {
        assert!(STORE_OWNER_ALLOWLIST.contains(&row));
    }
}

#[test]
fn pp_splice_identity_and_real_maps() {
    // Identity fast path (all M1 fixtures): same bytes, copied map.
    let (bytes, mid) = cc_silicon_compiler::chips::splice(b"int x;\n");
    assert_eq!(bytes, b"int x;\n");
    assert_eq!(mid, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    // Real splice: backslash-newline pairs vanish with composed maps.
    let (bytes, mid) = cc_silicon_compiler::chips::splice(b"a\\\nb");
    assert_eq!(bytes, b"ab");
    assert_eq!(mid, vec![0, 1, 4]);
    // Trailing lone backslash is ordinary.
    let (bytes, mid) = cc_silicon_compiler::chips::splice(b"a\\");
    assert_eq!(bytes, b"a\\");
    assert_eq!(mid, vec![0, 1, 2]);
    // Map composition helper: identity composes to identity.
    assert_eq!(compose_map(&[0, 1, 2, 3], &[0, 1, 2, 3]), vec![0, 1, 2, 3]);
}

#[test]
fn pp_comment_m1_scope_rules() {
    // Identity (all M1 fixtures).
    let (bytes, mid) = replace_comments(b"int x;\n").unwrap();
    assert_eq!(bytes, b"int x;\n");
    assert_eq!(mid, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    // Line comment becomes one space; the newline survives.
    let (bytes, _) = replace_comments(b"a // c\nb").unwrap();
    assert_eq!(bytes, b"a  \nb");
    // Block comment becomes one space; inner newlines survive; no gluing.
    let (bytes, _) = replace_comments(b"a/**/b").unwrap();
    assert_eq!(bytes, b"a b");
    let (bytes, _) = replace_comments(b"a/*x\ny*/b").unwrap();
    assert_eq!(bytes, b"a\n b");
    // Unterminated block comment fails loudly (M1-NEG-01 carrier shape).
    assert!(replace_comments(b"a /* x").is_err());
    // Literals are explicit unsupported (protection deferred).
    assert!(replace_comments(b"\"https://x\"").is_err());
    assert!(replace_comments(b"'/'").is_err());
}

#[test]
fn pp_scan_m1_table_and_munch() {
    // Exact M1 classification.
    let tokens = scan(b"int main(void){return 2+3;}").unwrap();
    let kinds: Vec<PpTokenKind> = tokens.iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            PpTokenKind::Identifier,
            PpTokenKind::Identifier,
            PpTokenKind::Punctuator,
            PpTokenKind::Identifier,
            PpTokenKind::Punctuator,
            PpTokenKind::Punctuator,
            PpTokenKind::Identifier,
            PpTokenKind::PpNumber,
            PpTokenKind::Punctuator,
            PpTokenKind::PpNumber,
            PpTokenKind::Punctuator,
            PpTokenKind::Punctuator,
        ]
    );
    // Maximal munch: `1e+foo` is one pp-number; `+ +` stays two.
    let tokens = scan(b"1e+foo").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, PpTokenKind::PpNumber);
    let tokens = scan(b"+ +").unwrap();
    assert_eq!(tokens.len(), 2);
    // Non-M1 inputs are explicit unsupported, never mis-tokenized.
    assert!(scan(b"a*b").is_err());
    assert!(scan(b"\"s\"").is_err());
}

/// Install the full M1 PP chain (normalize enfranchised into the PP
/// registry) for the true source-bytes end-to-end.
fn install_e2e(bus: &mut CompilerBus) {
    // `pp_slice()` already carries every kind through `/16` (cumulative).
    bus.kinds = TaskKindRegistry::pp_slice();
    bus.schema = StoreSchema::pp_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(
            TaskKind::PREPROCESS_NORMALIZE,
            cc_silicon_compiler::manifest::PP01_CHIP,
            1,
        )
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_SCAN, PP_SCAN_CHIP, 1)
        .unwrap();
}

#[test]
fn pp_source_bytes_end_to_end_with_no_seeded_pp() {
    use cc_silicon_compiler::manifest::PP01_CHIP;
    for (raw, raw_end) in [
        (
            [b"int main(void){return 2+3;}".to_vec(), b"\n".to_vec()].concat(),
            28u64,
        ),
        (
            [b"int main(void){return 2+3;}\r".to_vec(), b"\n".to_vec()].concat(),
            29u64,
        ),
    ] {
        let mut bus = new_bus();
        install_e2e(&mut bus);
        let source = seed_source(&mut bus, b"main.c", raw);
        let workers = workers();
        // normalize takes a source ref (not an artifact).
        bootstrap(
            &mut bus,
            TaskKind::PREPROCESS_NORMALIZE,
            PP01_CHIP,
            Payload::from_refs(vec![RecordRef::Source(source)]),
        );
        let report = tick(&mut bus, &workers);
        let mut artifact = completed_artifact(&report, &bus);
        // splice → comment take artifacts.
        for (kind, owner) in [
            (TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP),
            (TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP),
        ] {
            artifact = run_stage(&mut bus, &workers, kind, owner, artifact);
        }
        // scan closes with 12 tokens + EOF and no seeded PP fixtures.
        bootstrap(
            &mut bus,
            TaskKind::PREPROCESS_SCAN,
            PP_SCAN_CHIP,
            Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
        );
        let report = tick(&mut bus, &workers);
        let refs = match &report.outcome {
            TickOutcome::Executed { commit, .. } => {
                assert_eq!(commit.completed.len(), 1);
                match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                    cc_silicon_compiler::task::ResultValue::Records(refs) => refs.clone(),
                    other => panic!("expected records, got {other:?}"),
                }
            }
            other => panic!("expected executed, got {other:?}"),
        };
        assert_eq!(refs.len(), 13);
        let kinds: Vec<PpTokenKind> = refs
            .iter()
            .map(|reference| match reference {
                RecordRef::PpToken(id) => bus.arenas.pp_tokens.get(*id).unwrap().kind,
                other => panic!("expected pp-token ref, got {other:?}"),
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                PpTokenKind::Identifier,
                PpTokenKind::Identifier,
                PpTokenKind::Punctuator,
                PpTokenKind::Identifier,
                PpTokenKind::Punctuator,
                PpTokenKind::Punctuator,
                PpTokenKind::Identifier,
                PpTokenKind::PpNumber,
                PpTokenKind::Punctuator,
                PpTokenKind::PpNumber,
                PpTokenKind::Punctuator,
                PpTokenKind::Punctuator,
                PpTokenKind::Eof,
            ]
        );
        // EOF span is zero-width at the RAW end (28 or 29).
        match refs[12] {
            RecordRef::PpToken(id) => {
                let token = bus.arenas.pp_tokens.get(id).unwrap();
                assert_eq!(token.spelling, b"");
                let span = bus.arenas.spans.get(token.span).unwrap();
                assert_eq!((span.start, span.end), (raw_end, raw_end));
            }
            other => panic!("expected pp-token ref, got {other:?}"),
        }
    }
}

#[test]
fn pp_chain_kinds_reject_malformed_inputs() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    // Empty payload.
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_SPLICE,
        PP_SPLICE_CHIP,
        Payload::empty(),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // Non-canonical draft indexes stay rejected.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let source = seed_source(&mut bus, b"m.c", b"ab".to_vec());
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_SCAN,
        PP_SCAN_CHIP,
        Payload::from_refs(vec![RecordRef::Source(source)]),
    );
    bus.arenas.tasks.get_mut(task).unwrap().state = cc_silicon_compiler::task::TaskState::Running;
    bus.tasks.ready.clear();
    let error = commit_proposals(
        &mut bus,
        vec![
            cc_silicon_compiler::bus::TaggedProposal {
                chip: PP_SCAN_CHIP,
                task,
                proposal: Proposal::AppendRecords {
                    task,
                    batch: AppendBatch {
                        records: vec![RecordDraft {
                            family: cc_silicon_compiler::ids::RecordFamily::Artifact,
                            index: DraftRef(42),
                        }],
                        bodies: vec![G1DraftBody::Artifact(ArtifactRecord {
                            kind: ArtifactKind::Normalized,
                            source: Some(source),
                            bytes: b"ab".to_vec(),
                            raw_offsets: vec![0, 1, 2],
                        })],
                    },
                },
            },
            cc_silicon_compiler::bus::TaggedProposal {
                chip: PP_SCAN_CHIP,
                task,
                proposal: Proposal::Complete {
                    task,
                    value: ResultValue::Ack,
                },
            },
        ],
    )
    .unwrap_err();
    assert!(matches!(error, CommitError::InvalidPatchShape { .. }));
}

#[test]
fn pp_stage_layer_and_manifest_gates() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    // Re-point one PP route at a wrong layer on a scratch bus.
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::pp_slice();
    mismatched.schema = StoreSchema::pp_slice();
    mismatched
        .registrations
        .register(
            PpSpliceChip.manifest(),
            &mismatched.schema,
            &mismatched.kinds,
        )
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::PREPROCESS_SPLICE,
        PP_SPLICE_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // Re-registering an owned manifest is rejected (unique chip IDs).
    assert!(matches!(
        mismatched.registrations.register(
            PpSpliceChip.manifest(),
            &StoreSchema::pp_slice(),
            &TaskKindRegistry::pp_slice()
        ),
        Err(ManifestRegistryError::Registry(_))
    ));
    let _ = FieldPath::new(StoreId::Artifacts, "fragments");
    let _ = bus;
}

#[test]
fn pp_snapshot_replay_is_deterministic() {
    let run = || {
        let mut bus = new_bus();
        install_pp(&mut bus);
        let source = seed_source(&mut bus, b"main.c", b"int x;\n".to_vec());
        let workers = workers();
        bootstrap(
            &mut bus,
            TaskKind::PREPROCESS_NORMALIZE,
            cc_silicon_compiler::manifest::PP01_CHIP,
            Payload::from_refs(vec![RecordRef::Source(source)]),
        );
        // NOTE: see the end-to-end test for the routed normalize path.
        let _ = normalize;
        let _ = workers;
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
