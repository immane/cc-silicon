// ============================================================================
// c21_directive.rs — Wave 2 (`/21`) PP directive-dispatch slice acceptance.
//
// Covers the frozen closure: the `preprocess.directive` (local 20) and
// `preprocess.diagnostic` (local 21) kinds, kind-to-stage assignment
// (stage 1), the PP05/PP26 manifests (reads only), directive-line
// recognition (real, commented-dead, mid-line-dead lines), the frozen
// diagnostic taxonomy (error fans out, everything else fails fast as
// explicit `Unsupported`), the fan-out + await-all resume with message
// propagation, the PP26 negative-only message join, and the `/21` hash
// participation. Conditionals/macros/include stay deferred.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins};
use cc_silicon_compiler::chips::{
    handler_for, PpCommentChip, PpDiagnosticChip, PpDirectiveChip, PpNormalizeChip, PpScanChip,
    PpSpliceChip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    stage_of, PP05_CHIP, PP26_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP, PP_SPLICE_CHIP,
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
    bus.kinds = TaskKindRegistry::pp_directive_slice();
    bus.schema = StoreSchema::pp_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpDirectiveChip,
        &PpDiagnosticChip,
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
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers.register(PpDirectiveChip).unwrap();
    workers.register(PpDiagnosticChip).unwrap();
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

/// The failure message attached to a failed task.
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

fn run_directive(
    raw: Vec<u8>,
) -> (
    CompilerBus,
    TaskId,
    cc_silicon_compiler::routing::TickReport,
) {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"directive.c", raw);
    let workers = workers();
    // Drain to quiescence: the fan-out needs the child tick plus the join.
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_DIRECTIVE,
        PP05_CHIP,
        Payload::from_refs(refs),
    );
    let mut report = tick(&mut bus, &workers);
    for _ in 0..8 {
        let live = bus.arenas.tasks.iter().any(|(_, record)| {
            !matches!(
                record.state,
                cc_silicon_compiler::task::TaskState::Completed(_)
                    | cc_silicon_compiler::task::TaskState::Failed(_)
            )
        });
        if !live {
            break;
        }
        report = tick(&mut bus, &workers);
    }
    (bus, task, report)
}

#[test]
fn directive_kind_stage_registry_frozen() {
    use cc_silicon_compiler::manifest::{is_pp_directive_slice_kind, StoreSchema};
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::PREPROCESS_DIRECTIVE.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_DIRECTIVE.local(), 20);
    assert_eq!(
        TaskKind::PREPROCESS_DIAGNOSTIC.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_DIAGNOSTIC.local(), 21);
    assert!(is_pp_directive_slice_kind(TaskKind::PREPROCESS_DIRECTIVE));
    assert!(is_pp_directive_slice_kind(TaskKind::PREPROCESS_DIAGNOSTIC));
    assert!(!is_pp_directive_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_directive_slice_kind(TaskKind::PREPROCESS_SCAN));
    assert_eq!(stage_of(TaskKind::PREPROCESS_DIRECTIVE), Some(1));
    assert_eq!(stage_of(TaskKind::PREPROCESS_DIAGNOSTIC), Some(1));
    let registry = TaskKindRegistry::pp_directive_slice();
    assert_eq!(registry.len(), 34);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_DIRECTIVE)
            .unwrap()
            .name,
        "preprocess.directive"
    );
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_DIAGNOSTIC)
            .unwrap()
            .name,
        "preprocess.diagnostic"
    );
    // Read-only dispatchers: no writes, so no allowlist rows are required.
    assert!(PpDirectiveChip.manifest().writes.is_empty());
    assert!(PpDiagnosticChip.manifest().writes.is_empty());
    assert_eq!(PpDirectiveChip.manifest().id, PP05_CHIP);
    assert_eq!(PpDiagnosticChip.manifest().id, PP26_CHIP);
    let _ = StoreSchema::pp_slice();
    let _ = StoreId::Diagnostics;
}

#[test]
fn directive_clean_source_acknowledges() {
    let (bus, task, _) = run_directive(b"int main(void){return 2+3;}\n".to_vec());
    match &bus.arenas.tasks.get(task).unwrap().state {
        cc_silicon_compiler::task::TaskState::Completed(_) => {}
        other => panic!("expected completed, got {other:?}"),
    }
    // A bare `#` line is a null directive, never an error.
    let (bus, task, _) = run_directive(b"#\nint main(void){return 2+3;}\n".to_vec());
    match &bus.arenas.tasks.get(task).unwrap().state {
        cc_silicon_compiler::task::TaskState::Completed(_) => {}
        other => panic!("expected completed, got {other:?}"),
    }
}

#[test]
fn directive_error_fails_with_message() {
    let (bus, task, _) = run_directive(b"#error boom 42\nint main(void){return 2+3;}\n".to_vec());
    // Frozen join: the failure reuses the first failed child's diagnostic.
    let message = failure_message(&bus, task);
    assert_eq!(message, "boom 42");
    // The PP26 child carries the exact joined message.
    let children: Vec<TaskId> = bus
        .arenas
        .tasks
        .iter()
        .filter(|(_, record)| record.parent == Some(task))
        .map(|(id, _)| id)
        .collect();
    assert_eq!(children.len(), 1);
    assert_eq!(failure_message(&bus, children[0]), "boom 42");
    // Two error lines fan out twice; the join surfaces the first message.
    let (bus, task, _) =
        run_directive(b"#error one\n#error two\nint main(void){return 2+3;}\n".to_vec());
    assert_eq!(failure_message(&bus, task), "one");
    let children: Vec<TaskId> = bus
        .arenas
        .tasks
        .iter()
        .filter(|(_, record)| record.parent == Some(task))
        .map(|(id, _)| id)
        .collect();
    assert_eq!(children.len(), 2);
}

#[test]
fn directive_taxonomy_fails_fast_unsupported() {
    // `#define` names its deferred owner instead of miscompiling.
    let (bus, task, _) = run_directive(b"#define X 1\nint main(void){return 2+3;}\n".to_vec());
    let message = failure_message(&bus, task);
    assert!(message.contains("deferr"), "message: {message}");
    // `#warning` has no non-fatal wire: explicit deferral, never silent Ack.
    let (bus, task, _) = run_directive(b"#warning hmm\nint main(void){return 2+3;}\n".to_vec());
    let message = failure_message(&bus, task);
    assert!(message.contains("deferr"), "message: {message}");
    // `#include` is PP17/18 territory.
    let (bus, task, _) =
        run_directive(b"#include <stdio.h>\nint main(void){return 2+3;}\n".to_vec());
    let message = failure_message(&bus, task);
    assert!(message.contains("deferr"), "message: {message}");
}

#[test]
fn directive_malformed_fails_typed() {
    let (bus, task, _) = run_directive(b"# 123\nint main(void){return 2+3;}\n".to_vec());
    let message = failure_message(&bus, task);
    assert!(message.contains("malformed"), "message: {message}");
    assert!(!message.contains("deferred"), "message: {message}");
    // An empty payload violates the task-shape convention.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_DIRECTIVE,
        PP05_CHIP,
        Payload::empty(),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

#[test]
fn directive_commented_lines_stay_dead() {
    // A `//`-killed directive is not a directive: the task acknowledges.
    let (bus, task, _) = run_directive(b"// #error boom\nint main(void){return 2+3;}\n".to_vec());
    match &bus.arenas.tasks.get(task).unwrap().state {
        cc_silicon_compiler::task::TaskState::Completed(_) => {}
        other => panic!("expected completed, got {other:?}"),
    }
    // A same-line block comment is skipped: the directive is live.
    let (bus, task, _) = run_directive(b"/*c*/#error live\nint main(void){return 2+3;}\n".to_vec());
    let message = failure_message(&bus, task);
    assert!(message.contains("live"), "message: {message}");
    // A mid-line `#` is not a directive line.
    let (bus, task, _) =
        run_directive(b"int x; #error boom\nint main(void){return 2+3;}\n".to_vec());
    match &bus.arenas.tasks.get(task).unwrap().state {
        cc_silicon_compiler::task::TaskState::Completed(_) => {}
        other => panic!("expected completed, got {other:?}"),
    }
}

#[test]
fn directive_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut bus = new_bus();
    install_pp(&mut bus);
    for (kind, chip) in [
        (TaskKind::PREPROCESS_DIRECTIVE, PP05_CHIP),
        (TaskKind::PREPROCESS_DIAGNOSTIC, PP26_CHIP),
    ] {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::pp_directive_slice();
        mismatched.schema = StoreSchema::pp_slice();
        let manifest = if chip == PP05_CHIP {
            PpDirectiveChip.manifest()
        } else {
            PpDiagnosticChip.manifest()
        };
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
    // Manifests claiming the new kinds are rejected against the stale
    // pre-`/21` registry that does not know them.
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            PpDirectiveChip.manifest(),
            &StoreSchema::pp_slice(),
            &TaskKindRegistry::pp_slice(),
        )
        .is_err());
    assert!(stale
        .register(
            PpDiagnosticChip.manifest(),
            &StoreSchema::pp_slice(),
            &TaskKindRegistry::pp_slice(),
        )
        .is_err());
}

#[test]
fn directive_snapshot_replay_is_deterministic() {
    let run = || {
        let (bus, _, _) = run_directive(b"#error boom\nint main(void){return 2+3;}\n".to_vec());
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
