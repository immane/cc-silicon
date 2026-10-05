// ============================================================================
// c10_pp01.rs — Wave 2 (`/10`) PP01 source-normalize slice acceptance.
//
// Covers the frozen closure: the single-source `Normalized` artifact shape
// (rev-44), mandatory-map invariants (rev-45), the M1 boundary convention
// (identity / inserted-LF / CRLF collapse), the `preprocess.normalize` kind,
// kind-to-stage assignment, the PP01 store-owner allowlist row, stage/layer
// agreement, snapshot bodies, tick-lifecycle integration, and the `/10` hash
// participation. Seeded sources stand in for Host import; `M1-CL-05` on real
// upstream artifacts stays the Wave-2 acceptance for later slices.
// ============================================================================

use cc_silicon_compiler::bus::{ArtifactKind, CompilerBus, CompilerPins};
use cc_silicon_compiler::chips::{handler_for, normalize, PpNormalizeChip, Worker, WorkerRegistry};
use cc_silicon_compiler::commit::{commit_proposals, CommitError};
use cc_silicon_compiler::ids::{ArtifactId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_pp01_slice_kind, stage_of, FieldPath, ManifestRegistryError, StoreSchema, PP01_CHIP,
    STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::records::{G1DraftBody, RecordDraft};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{decode_artifact, encode_artifact_record, Snapshot};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    AppendBatch, DraftRef, Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup, TaskKind,
    TaskKindRegistry, TaskState,
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

fn install_pp01(bus: &mut CompilerBus, layer: u16) {
    bus.kinds = TaskKindRegistry::pp01_slice();
    bus.schema = StoreSchema::m1_slice();
    bus.registrations
        .register(PpNormalizeChip.manifest(), &bus.schema, &bus.kinds)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_NORMALIZE, PP01_CHIP, layer)
        .unwrap();
}

fn seed_source(
    bus: &mut CompilerBus,
    name: &[u8],
    bytes: Vec<u8>,
) -> cc_silicon_compiler::ids::SourceId {
    let name = bus.intern_name(name).unwrap();
    bus.alloc_source(name, bytes).unwrap()
}

fn normalize_task(bus: &mut CompilerBus, source: cc_silicon_compiler::ids::SourceId) -> TaskId {
    bus.bootstrap_task(TaskDraft {
        kind: TaskKind::PREPROCESS_NORMALIZE,
        owner: PP01_CHIP,
        parent: None,
        payload: Payload::from_refs(vec![RecordRef::Source(source)]),
        continuation: None,
    })
    .unwrap()
}

fn canonical_base() -> Vec<u8> {
    b"int main(void){return 2+3;}".to_vec()
}

#[test]
fn pp01_kind_stage_and_allowlist_are_frozen() {
    assert_eq!(
        TaskKind::PREPROCESS_NORMALIZE.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_NORMALIZE.local(), 16);
    assert!(is_pp01_slice_kind(TaskKind::PREPROCESS_NORMALIZE));
    assert!(!is_pp01_slice_kind(TaskKind::CONTROL_NOOP));
    assert_eq!(stage_of(TaskKind::PREPROCESS_NORMALIZE), Some(1));
    assert!(STORE_OWNER_ALLOWLIST.contains(&(
        PP01_CHIP,
        StoreId::Artifacts,
        "fragments",
        TaskKind::PREPROCESS_NORMALIZE
    )));
    let registry = TaskKindRegistry::pp01_slice();
    assert_eq!(registry.len(), 8);
    let entry = registry.lookup(TaskKind::PREPROCESS_NORMALIZE).unwrap();
    assert_eq!(entry.name, "preprocess.normalize");
}

#[test]
fn pp01_normalize_identity_insert_and_crlf() {
    // Unit-level: the pure boundary function on the three M1 variants.
    let base = canonical_base();
    let mut src000 = base.clone();
    src000.push(b'\n');
    assert_eq!(src000.len(), 28);
    let (out, map) = normalize(&src000).unwrap();
    assert_eq!(out, src000);
    assert_eq!(map, (0..=28).collect::<Vec<u64>>());

    let src001 = base.clone();
    assert_eq!(src001.len(), 27);
    let (out, map) = normalize(&src001).unwrap();
    let mut expected = src001.clone();
    expected.push(b'\n');
    assert_eq!(out, expected);
    let mut expected_map: Vec<u64> = (0..=27).collect();
    expected_map.push(27);
    assert_eq!(map, expected_map);

    let mut src002 = base.clone();
    src002.extend_from_slice(b"\r\n");
    assert_eq!(src002.len(), 29);
    let (out, map) = normalize(&src002).unwrap();
    assert_eq!(out, src000);
    let mut expected_map: Vec<u64> = (0..=27).collect();
    expected_map.push(29);
    assert_eq!(map, expected_map);

    // Lone CR is explicit unsupported, never a silent reinterpretation.
    assert!(normalize(b"a\rb").is_err());
}

#[test]
fn pp01_tick_produces_single_normalized_artifact() {
    let mut bus = new_bus();
    install_pp01(&mut bus, 1);
    let mut raw = canonical_base();
    raw.push(b'\n');
    let source = seed_source(&mut bus, b"main.c", raw.clone());
    normalize_task(&mut bus, source);
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    let report = RoutingShell
        .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
        .unwrap();
    match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            assert_eq!(commit.appended.len(), 1);
        }
        other => panic!("expected executed, got {other:?}"),
    }
    assert_eq!(bus.arenas.artifacts.allocated(), 1);
    let artifact = bus.arenas.artifacts.get(ArtifactId::from_index(0)).unwrap();
    assert_eq!(artifact.kind, ArtifactKind::Normalized);
    assert_eq!(artifact.source, Some(source));
    assert_eq!(artifact.bytes, raw);
    assert_eq!(artifact.raw_offsets, (0..=28).collect::<Vec<u64>>());
    // Snapshot round-trips the new fields.
    let bytes = encode_artifact_record(artifact);
    let decoded = decode_artifact(&bytes).unwrap();
    assert_eq!(&decoded, artifact);
    let _ = Snapshot::capture(&bus);
}

#[test]
fn pp01_crlf_and_insert_tick_paths() {
    for (raw, expected_map) in [
        (
            [canonical_base(), b"\r\n".to_vec()].concat(),
            [(0..=27).collect::<Vec<u64>>(), vec![29]].concat(),
        ),
        (
            canonical_base(),
            [(0..=27).collect::<Vec<u64>>(), vec![27]].concat(),
        ),
    ] {
        let mut bus = new_bus();
        install_pp01(&mut bus, 1);
        let source = seed_source(&mut bus, b"main.c", raw.clone());
        normalize_task(&mut bus, source);
        let mut workers = WorkerRegistry::new();
        workers.register(PpNormalizeChip).unwrap();
        RoutingShell
            .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
            .unwrap();
        let artifact = bus.arenas.artifacts.get(ArtifactId::from_index(0)).unwrap();
        let mut expected = canonical_base();
        expected.push(b'\n');
        assert_eq!(artifact.bytes, expected);
        assert_eq!(artifact.raw_offsets, expected_map);
    }
}

#[test]
fn pp01_rejects_bad_payload_source_and_map() {
    // Wrong arity.
    let mut bus = new_bus();
    install_pp01(&mut bus, 1);
    let task = bus
        .bootstrap_task(TaskDraft {
            kind: TaskKind::PREPROCESS_NORMALIZE,
            owner: PP01_CHIP,
            parent: None,
            payload: Payload::empty(),
            continuation: None,
        })
        .unwrap();
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    let report = RoutingShell
        .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
        .unwrap();
    match report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(bus.arenas.artifacts.allocated(), 0);
    let _ = task;

    // Dangling source.
    let mut bus = new_bus();
    install_pp01(&mut bus, 1);
    bus.bootstrap_task(TaskDraft {
        kind: TaskKind::PREPROCESS_NORMALIZE,
        owner: PP01_CHIP,
        parent: None,
        payload: Payload::from_refs(vec![RecordRef::Source(
            cc_silicon_compiler::ids::SourceId::from_index(77),
        )]),
        continuation: None,
    })
    .unwrap();
    let report = RoutingShell
        .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
        .unwrap();
    match report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }

    // Malformed map is rejected before mutation.
    let mut bus = new_bus();
    install_pp01(&mut bus, 1);
    let source = seed_source(&mut bus, b"main.c", b"int x;\n".to_vec());
    let task = normalize_task(&mut bus, source);
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    bus.tasks.ready.clear();
    let bad = cc_silicon_compiler::bus::ArtifactRecord {
        kind: ArtifactKind::Normalized,
        source: Some(source),
        bytes: b"ab".to_vec(),
        raw_offsets: vec![0, 5],
    };
    let error = commit_proposals(
        &mut bus,
        vec![
            cc_silicon_compiler::bus::TaggedProposal {
                chip: PP01_CHIP,
                task,
                proposal: Proposal::AppendRecords {
                    task,
                    batch: AppendBatch {
                        records: vec![RecordDraft {
                            family: cc_silicon_compiler::ids::RecordFamily::Artifact,
                            index: DraftRef(0),
                        }],
                        bodies: vec![G1DraftBody::Artifact(bad)],
                    },
                },
            },
            cc_silicon_compiler::bus::TaggedProposal {
                chip: PP01_CHIP,
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
    assert_eq!(bus.arenas.artifacts.allocated(), 0);
}

#[test]
fn pp01_stage_layer_and_manifest_gates() {
    // Wrong routed layer fails on the driver path, never a silent fold.
    let mut bus = new_bus();
    install_pp01(&mut bus, 9);
    let source = seed_source(&mut bus, b"main.c", b"int x;\n".to_vec());
    let task = normalize_task(&mut bus, source);
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    let error = cc_silicon_compiler::chips::drive_task(&bus, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));

    // Manifest without the artifact write is rejected at registration.
    let mut manifest = PpNormalizeChip.manifest();
    manifest.writes = vec![];
    assert!(matches!(
        bus.registrations.register(
            manifest,
            &StoreSchema::m1_slice(),
            &TaskKindRegistry::pp01_slice()
        ),
        Err(ManifestRegistryError::Registry(_))
    ));
    let _ = FieldPath::new(StoreId::Artifacts, "fragments");
}

#[test]
fn pp01_snapshot_replay_is_deterministic() {
    let run = || {
        let mut bus = new_bus();
        install_pp01(&mut bus, 1);
        let source = seed_source(&mut bus, b"main.c", b"int x;\n".to_vec());
        normalize_task(&mut bus, source);
        let mut workers = WorkerRegistry::new();
        workers.register(PpNormalizeChip).unwrap();
        RoutingShell
            .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
            .unwrap();
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
