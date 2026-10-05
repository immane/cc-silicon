// ============================================================================
// c25_include.rs — Wave 2 (`/25`) PP include slice acceptance.
//
// Covers the frozen closure: the `preprocess.include_resolve` (local 28)
// and `preprocess.include_enter` (29) kinds, kind-to-stage assignment
// (stage 1), the PP17/PP18 manifests (reads only), the frozen path
// policy (exact/basename/ambiguous/not-loaded), single-pass stitching
// (directive replaced, rest verbatim, EOF single, nested survives),
// missing-scan and macro-name failures, stage/layer gates, and the `/25`
// hash participation. Async fetch and fixpoint nesting stay deferred.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, PpTokenKind};
use cc_silicon_compiler::chips::{
    handler_for, PpCommentChip, PpConditionalChip, PpDefineChip, PpDiagnosticChip, PpDirectiveChip,
    PpIncludeEnterChip, PpIncludeResolveChip, PpInvokeChip, PpNormalizeChip, PpRedefineChip,
    PpScanChip, PpSpliceChip, PpSubstituteChip, PpUndefChip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{RecordRef, SourceId, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    stage_of, PP05_CHIP, PP06_CHIP, PP07_CHIP, PP08_CHIP, PP09_CHIP, PP12_CHIP, PP17_CHIP,
    PP18_CHIP, PP19_CHIP, PP26_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP, PP_SPLICE_CHIP,
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

fn install_pp(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::{StoreSchema, PP01_CHIP};
    bus.kinds = TaskKindRegistry::pp_include_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpDirectiveChip,
        &PpDiagnosticChip,
        &PpConditionalChip,
        &PpDefineChip,
        &PpRedefineChip,
        &PpUndefChip,
        &PpInvokeChip,
        &PpSubstituteChip,
        &PpIncludeResolveChip,
        &PpIncludeEnterChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(TaskKind::PREPROCESS_NORMALIZE, PP01_CHIP, 1)
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
    bus.routing
        .register(TaskKind::PREPROCESS_DIRECTIVE, PP05_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_DIAGNOSTIC, PP26_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_CONDITIONAL, PP19_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_MACRO_DEFINE, PP06_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_MACRO_REDEFINE, PP07_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_MACRO_UNDEF, PP08_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_MACRO_INVOKE, PP09_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_MACRO_SUBSTITUTE, PP12_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_INCLUDE_RESOLVE, PP17_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_INCLUDE_ENTER, PP18_CHIP, 1)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers.register(PpDirectiveChip).unwrap();
    workers.register(PpDiagnosticChip).unwrap();
    workers.register(PpConditionalChip).unwrap();
    workers.register(PpDefineChip).unwrap();
    workers.register(PpRedefineChip).unwrap();
    workers.register(PpUndefChip).unwrap();
    workers.register(PpInvokeChip).unwrap();
    workers.register(PpSubstituteChip).unwrap();
    workers.register(PpIncludeResolveChip).unwrap();
    workers.register(PpIncludeEnterChip).unwrap();
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

/// Run PP01→PP02→PP03→PP04 from real source bytes; return all pp-token refs.
fn scan_source(bus: &mut CompilerBus, name: &[u8], raw: Vec<u8>) -> Vec<RecordRef> {
    use cc_silicon_compiler::manifest::PP01_CHIP;
    let name = bus.intern_name(name).unwrap();
    let source = bus.alloc_source(name, raw).unwrap();
    let workers = workers();
    bootstrap(
        bus,
        TaskKind::PREPROCESS_NORMALIZE,
        PP01_CHIP,
        Payload::from_refs(vec![RecordRef::Source(source)]),
    );
    let mut artifact = completed_artifact(&tick(bus, &workers), bus);
    for (kind, owner) in [
        (TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP),
        (TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP),
    ] {
        bootstrap(
            bus,
            kind,
            owner,
            Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
        );
        artifact = completed_artifact(&tick(bus, &workers), bus);
    }
    bootstrap(
        bus,
        TaskKind::PREPROCESS_SCAN,
        PP_SCAN_CHIP,
        Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
    );
    match tick(bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Records(refs) => refs.clone(),
                other => panic!("expected records, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    }
}

fn completed_artifact(
    report: &cc_silicon_compiler::routing::TickReport,
    bus: &CompilerBus,
) -> cc_silicon_compiler::ids::ArtifactId {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Artifact(id)) => *id,
                other => panic!("expected artifact ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    }
}

fn completed_source(
    report: &cc_silicon_compiler::routing::TickReport,
    bus: &CompilerBus,
) -> SourceId {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Source(id)) => *id,
                other => panic!("expected source ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    }
}

fn failure_message(bus: &CompilerBus, task: TaskId) -> String {
    match &bus.arenas.tasks.get(task).unwrap().state {
        cc_silicon_compiler::task::TaskState::Failed(diagnostic) => bus
            .arenas
            .diagnostics
            .get(*diagnostic)
            .unwrap()
            .message
            .clone(),
        other => panic!("expected failed task, got {other:?}"),
    }
}

fn spellings(bus: &CompilerBus, refs: &[RecordRef]) -> Vec<Vec<u8>> {
    refs.iter()
        .map(|reference| match reference {
            RecordRef::PpToken(id) => bus.arenas.pp_tokens.get(*id).unwrap().spelling.clone(),
            other => panic!("expected pp-token ref, got {other:?}"),
        })
        .collect()
}

#[test]
fn include_kind_stage_registry_frozen() {
    use cc_silicon_compiler::manifest::{is_pp_include_slice_kind, StoreSchema};
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::PREPROCESS_INCLUDE_RESOLVE.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_INCLUDE_RESOLVE.local(), 28);
    assert_eq!(TaskKind::PREPROCESS_INCLUDE_ENTER.local(), 29);
    assert!(is_pp_include_slice_kind(
        TaskKind::PREPROCESS_INCLUDE_RESOLVE
    ));
    assert!(is_pp_include_slice_kind(TaskKind::PREPROCESS_INCLUDE_ENTER));
    assert!(!is_pp_include_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_include_slice_kind(TaskKind::PREPROCESS_MACRO_INVOKE));
    assert_eq!(stage_of(TaskKind::PREPROCESS_INCLUDE_RESOLVE), Some(1));
    assert_eq!(stage_of(TaskKind::PREPROCESS_INCLUDE_ENTER), Some(1));
    let registry = TaskKindRegistry::pp_include_slice();
    assert_eq!(registry.len(), 42);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_INCLUDE_RESOLVE)
            .unwrap()
            .name,
        "preprocess.include_resolve"
    );
    assert!(PpIncludeResolveChip.manifest().writes.is_empty());
    assert!(PpIncludeEnterChip.manifest().writes.is_empty());
    assert_eq!(PpIncludeResolveChip.manifest().id, PP17_CHIP);
    assert_eq!(PpIncludeEnterChip.manifest().id, PP18_CHIP);
    let _ = StoreSchema::pp_macro_slice();
    let _ = StoreId::Names;
}

#[test]
fn include_resolve_policy_matrix() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    // Commit main + two headers (one exact, one basename-only).
    let main_refs = scan_source(&mut bus, b"main.c", b"#include \"h.h\"\nint x;\n".to_vec());
    scan_source(&mut bus, b"h.h", b"int h = 1;\n".to_vec());
    scan_source(&mut bus, b"dir/other.h", b"int o = 2;\n".to_vec());
    // The header token is refs[2] ([#, include, HeaderName]).
    let header = main_refs[2];
    let _task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_RESOLVE,
        PP17_CHIP,
        Payload::from_refs(vec![header]),
    );
    let report = tick(&mut bus, &workers);
    let resolved = completed_source(&report, &bus);
    let name = bus
        .intern
        .get(bus.arenas.sources.get(resolved).unwrap().name)
        .unwrap()
        .to_vec();
    assert_eq!(name, b"h.h".to_vec());
    // Basename match: `other.h` resolves inside `dir/`.
    let main_refs = scan_source(&mut bus, b"m2.c", b"#include \"other.h\"\n".to_vec());
    let _task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_RESOLVE,
        PP17_CHIP,
        Payload::from_refs(vec![main_refs[2]]),
    );
    let report = tick(&mut bus, &workers);
    let resolved = completed_source(&report, &bus);
    let name = bus
        .intern
        .get(bus.arenas.sources.get(resolved).unwrap().name)
        .unwrap()
        .to_vec();
    assert_eq!(name, b"dir/other.h".to_vec());
    // Angle brackets resolve the same way.
    let main_refs = scan_source(&mut bus, b"m4.c", b"#include <h.h>\n".to_vec());
    let _task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_RESOLVE,
        PP17_CHIP,
        Payload::from_refs(vec![main_refs[2]]),
    );
    let report = tick(&mut bus, &workers);
    let resolved = completed_source(&report, &bus);
    let name = bus
        .intern
        .get(bus.arenas.sources.get(resolved).unwrap().name)
        .unwrap()
        .to_vec();
    assert_eq!(name, b"h.h".to_vec());
    // Not loaded fails loudly.
    let main_refs = scan_source(&mut bus, b"m3.c", b"#include \"nope.h\"\n".to_vec());
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_RESOLVE,
        PP17_CHIP,
        Payload::from_refs(vec![main_refs[2]]),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert!(failure_message(&bus, task).contains("not loaded"));
    // Non-HeaderName payload is a protocol fault failure.
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_RESOLVE,
        PP17_CHIP,
        Payload::from_refs(vec![main_refs[0]]),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    let _ = task;
}

#[test]
fn include_ambiguous_fails() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    scan_source(&mut bus, b"a/dup.h", b"int a = 1;\n".to_vec());
    scan_source(&mut bus, b"b/dup.h", b"int b = 2;\n".to_vec());
    let main_refs = scan_source(&mut bus, b"m.c", b"#include \"dup.h\"\n".to_vec());
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_RESOLVE,
        PP17_CHIP,
        Payload::from_refs(vec![main_refs[2]]),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert!(failure_message(&bus, task).contains("mbiguous"));
}

#[test]
fn include_enter_single_pass_stitch() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    let main_refs = scan_source(&mut bus, b"main.c", b"#include \"h.h\"\nint x;\n".to_vec());
    scan_source(&mut bus, b"h.h", b"int h = 1;\n".to_vec());
    // Resolve first (PP17), then enter with [stream..., Source].
    let _task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_RESOLVE,
        PP17_CHIP,
        Payload::from_refs(vec![main_refs[2]]),
    );
    let header = completed_source(&tick(&mut bus, &workers), &bus);
    let mut payload = main_refs.clone();
    payload.push(RecordRef::Source(header));
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_ENTER,
        PP18_CHIP,
        Payload::from_refs(payload),
    );
    let report = tick(&mut bus, &workers);
    let stitched = match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Records(refs) => refs.clone(),
                other => panic!("expected records, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    // The directive line is gone; header content sits in place; EOF single.
    assert_eq!(
        spellings(&bus, &stitched),
        vec![
            b"int".to_vec(),
            b"h".to_vec(),
            b"=".to_vec(),
            b"1".to_vec(),
            b";".to_vec(),
            b"int".to_vec(),
            b"x".to_vec(),
            b";".to_vec(),
            b"".to_vec(),
        ]
    );
    let _ = task;
}

#[test]
fn include_nested_survives_single_pass() {
    // A nested `#include` inside header content passes through verbatim
    // (fixpoint nesting belongs to the control-loop slice).
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    let main_refs = scan_source(&mut bus, b"main.c", b"#include \"h.h\"\n".to_vec());
    scan_source(
        &mut bus,
        b"h.h",
        b"int h = 1;\n#include \"inner.h\"\n".to_vec(),
    );
    let _task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_RESOLVE,
        PP17_CHIP,
        Payload::from_refs(vec![main_refs[2]]),
    );
    let header = completed_source(&tick(&mut bus, &workers), &bus);
    let mut payload = main_refs.clone();
    payload.push(RecordRef::Source(header));
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_ENTER,
        PP18_CHIP,
        Payload::from_refs(payload),
    );
    let report = tick(&mut bus, &workers);
    let stitched = match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Records(refs) => refs.clone(),
                other => panic!("expected records, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    let spellings = spellings(&bus, &stitched);
    assert!(spellings.contains(&b"include".to_vec()));
    assert!(spellings.contains(&b"\"inner.h\"".to_vec()));
    let _ = task;
}

#[test]
fn include_missing_scan_fails() {
    // Header committed but never scanned: explicit ordering failure.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    let main_refs = scan_source(&mut bus, b"main.c", b"#include \"h.h\"\n".to_vec());
    // Commit the header source bytes WITHOUT scanning (no pp-tokens).
    let header_name = bus.intern_name(b"h.h").unwrap();
    let header = bus
        .alloc_source(header_name, b"int h = 1;\n".to_vec())
        .unwrap();
    let mut payload = main_refs.clone();
    payload.push(RecordRef::Source(header));
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_INCLUDE_ENTER,
        PP18_CHIP,
        Payload::from_refs(payload),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert!(failure_message(&bus, task).contains("not scanned"));
}

#[test]
fn include_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut bus = new_bus();
    install_pp(&mut bus);
    for (kind, chip, manifest) in [
        (
            TaskKind::PREPROCESS_INCLUDE_RESOLVE,
            PP17_CHIP,
            PpIncludeResolveChip.manifest(),
        ),
        (
            TaskKind::PREPROCESS_INCLUDE_ENTER,
            PP18_CHIP,
            PpIncludeEnterChip.manifest(),
        ),
    ] {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::pp_include_slice();
        mismatched.schema = StoreSchema::pp_macro_slice();
        mismatched
            .registrations
            .register(manifest, &mismatched.schema, &mismatched.kinds)
            .unwrap();
        mismatched.routing.register(kind, chip, 9).unwrap();
        let workers = workers();
        let task = bootstrap(&mut mismatched, kind, chip, Payload::empty());
        let error =
            cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
        assert!(matches!(
            error,
            cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
        ));
    }
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            PpIncludeEnterChip.manifest(),
            &StoreSchema::pp_macro_slice(),
            &TaskKindRegistry::pp_expand_slice(),
        )
        .is_err());
    let _ = PpTokenKind::Eof;
}

#[test]
fn include_snapshot_replay_is_deterministic() {
    let run = || {
        let mut bus = new_bus();
        install_pp(&mut bus);
        let workers = workers();
        let main_refs = scan_source(&mut bus, b"main.c", b"#include \"h.h\"\nint x;\n".to_vec());
        scan_source(&mut bus, b"h.h", b"int h = 1;\n".to_vec());
        let task = bootstrap(
            &mut bus,
            TaskKind::PREPROCESS_INCLUDE_RESOLVE,
            PP17_CHIP,
            Payload::from_refs(vec![main_refs[2]]),
        );
        let header = completed_source(&tick(&mut bus, &workers), &bus);
        let mut payload = main_refs.clone();
        payload.push(RecordRef::Source(header));
        bootstrap(
            &mut bus,
            TaskKind::PREPROCESS_INCLUDE_ENTER,
            PP18_CHIP,
            Payload::from_refs(payload),
        );
        tick(&mut bus, &workers);
        let _ = task;
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
