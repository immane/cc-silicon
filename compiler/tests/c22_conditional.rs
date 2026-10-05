// ============================================================================
// c22_conditional.rs — Wave 2 (`/22`) PP conditional-inclusion slice
// acceptance.
//
// Covers the frozen closure: the `preprocess.conditional` kind (local 22),
// kind-to-stage assignment (stage 1), the PP19 manifest (reads only),
// the conditional stack (`#if/ifdef/ifndef/elif/else/endif`, nesting,
// unterminated/stray failures), the PP-int evaluator (precedence,
// ternary, shifts, div-by-zero/overflow failures, char-literal
// unsupported), `defined` frozen-false, the output contract (active lines
// kept incl. directives, EOF always), the PP19→PP05 chain (active `#error`
// fires, inactive drops), and the `/22` hash participation. Macros stay
// deferred.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins};
use cc_silicon_compiler::chips::{
    handler_for, PpCommentChip, PpConditionalChip, PpDiagnosticChip, PpDirectiveChip,
    PpNormalizeChip, PpScanChip, PpSpliceChip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    stage_of, PP05_CHIP, PP19_CHIP, PP26_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP, PP_SPLICE_CHIP,
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
    bus.kinds = TaskKindRegistry::pp_conditional_slice();
    bus.schema = StoreSchema::pp_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpDirectiveChip,
        &PpDiagnosticChip,
        &PpConditionalChip,
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

fn completed_records_on(
    report: &cc_silicon_compiler::routing::TickReport,
    bus: &CompilerBus,
) -> Vec<RecordRef> {
    match &report.outcome {
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

/// Run PP01→PP02→PP03→PP04→CONDITIONAL from real source bytes; return the
/// active pp-token refs plus the bus.
fn filtered_refs(raw: Vec<u8>) -> (CompilerBus, Vec<RecordRef>) {
    use cc_silicon_compiler::manifest::PP01_CHIP;
    let mut bus = new_bus();
    install_pp(&mut bus);
    let name = bus.intern_name(b"cond.c").unwrap();
    let source = bus.alloc_source(name, raw).unwrap();
    let workers = workers();
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_NORMALIZE,
        PP01_CHIP,
        Payload::from_refs(vec![RecordRef::Source(source)]),
    );
    let mut artifact = completed_artifact(&tick(&mut bus, &workers), &bus);
    for (kind, owner) in [
        (TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP),
        (TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP),
    ] {
        bootstrap(
            &mut bus,
            kind,
            owner,
            Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
        );
        artifact = completed_artifact(&tick(&mut bus, &workers), &bus);
    }
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_SCAN,
        PP_SCAN_CHIP,
        Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
    );
    let refs = completed_records_on(&tick(&mut bus, &workers), &bus);
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_CONDITIONAL,
        PP19_CHIP,
        Payload::from_refs(refs),
    );
    let refs = completed_records_on(&tick(&mut bus, &workers), &bus);
    (bus, refs)
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

/// Spellings of pp-token refs, in order (EOF included as empty).
fn spellings(bus: &CompilerBus, refs: &[RecordRef]) -> Vec<Vec<u8>> {
    refs.iter()
        .map(|reference| match reference {
            RecordRef::PpToken(id) => bus.arenas.pp_tokens.get(*id).unwrap().spelling.clone(),
            other => panic!("expected pp-token ref, got {other:?}"),
        })
        .collect()
}

#[test]
fn conditional_kind_stage_registry_frozen() {
    use cc_silicon_compiler::manifest::{is_pp_conditional_slice_kind, StoreSchema};
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::PREPROCESS_CONDITIONAL.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_CONDITIONAL.local(), 22);
    assert!(is_pp_conditional_slice_kind(
        TaskKind::PREPROCESS_CONDITIONAL
    ));
    assert!(!is_pp_conditional_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_conditional_slice_kind(
        TaskKind::PREPROCESS_DIRECTIVE
    ));
    assert_eq!(stage_of(TaskKind::PREPROCESS_CONDITIONAL), Some(1));
    let registry = TaskKindRegistry::pp_conditional_slice();
    assert_eq!(registry.len(), 35);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_CONDITIONAL)
            .unwrap()
            .name,
        "preprocess.conditional"
    );
    assert!(PpConditionalChip.manifest().writes.is_empty());
    assert_eq!(PpConditionalChip.manifest().id, PP19_CHIP);
    let _ = StoreSchema::pp_slice();
    let _ = StoreId::Pp;
}

#[test]
fn conditional_taken_dropped_branches() {
    // `#if 1` keeps the body; the directive lines vanish.
    let (bus, refs) = filtered_refs(b"#if 1\nint x;\n#endif\n".to_vec());
    assert_eq!(
        spellings(&bus, &refs),
        vec![b"int".to_vec(), b"x".to_vec(), b";".to_vec(), b"".to_vec()]
    );
    // `#if 0` drops the body; `#else` restores.
    let (bus, refs) = filtered_refs(b"#if 0\nint a;\n#else\nint b;\n#endif\n".to_vec());
    assert_eq!(
        spellings(&bus, &refs),
        vec![b"int".to_vec(), b"b".to_vec(), b";".to_vec(), b"".to_vec()]
    );
    // `#elif` selects the first true branch only.
    let (bus, refs) =
        filtered_refs(b"#if 0\nint a;\n#elif 1\nint b;\n#elif 1\nint c;\n#endif\n".to_vec());
    assert_eq!(
        spellings(&bus, &refs),
        vec![b"int".to_vec(), b"b".to_vec(), b";".to_vec(), b"".to_vec()]
    );
}

#[test]
fn conditional_defined_frozen_false() {
    // No macro table exists: `#ifdef` takes nothing, `#ifndef` takes all.
    let (bus, refs) = filtered_refs(b"#ifdef X\nint a;\n#endif\n".to_vec());
    assert_eq!(spellings(&bus, &refs), vec![b"".to_vec()]);
    let (bus, refs) = filtered_refs(b"#ifndef X\nint a;\n#endif\n".to_vec());
    assert_eq!(
        spellings(&bus, &refs),
        vec![b"int".to_vec(), b"a".to_vec(), b";".to_vec(), b"".to_vec()]
    );
    // `defined()` is frozen-`false` in expressions too.
    let (bus, refs) = filtered_refs(b"#if defined(X)\nint a;\n#endif\n".to_vec());
    assert_eq!(spellings(&bus, &refs), vec![b"".to_vec()]);
    let (bus, refs) = filtered_refs(b"#if !defined(X)\nint a;\n#endif\n".to_vec());
    assert_eq!(
        spellings(&bus, &refs),
        vec![b"int".to_vec(), b"a".to_vec(), b";".to_vec(), b"".to_vec()]
    );
}

#[test]
fn conditional_expression_matrix() {
    // Precedence, ternary, shifts, hex/octal spellings.
    let (bus, refs) = filtered_refs(b"#if 1+2*3 == 7\nint a;\n#endif\n".to_vec());
    assert_eq!(spellings(&bus, &refs).len(), 4);
    let (bus, refs) = filtered_refs(b"#if (1 ? 2 : 3) == 2\nint a;\n#endif\n".to_vec());
    assert_eq!(spellings(&bus, &refs).len(), 4);
    let (bus, refs) = filtered_refs(b"#if 0x10 == 16\nint a;\n#endif\n".to_vec());
    assert_eq!(spellings(&bus, &refs).len(), 4);
    let (bus, refs) = filtered_refs(b"#if 010 == 8\nint a;\n#endif\n".to_vec());
    assert_eq!(spellings(&bus, &refs).len(), 4);
    let (bus, refs) = filtered_refs(b"#if (1 << 4) == 16\nint a;\n#endif\n".to_vec());
    assert_eq!(spellings(&bus, &refs).len(), 4);
    // Falsy outcomes drop the body.
    let (bus, refs) = filtered_refs(b"#if 1+2*3 == 8\nint a;\n#endif\n".to_vec());
    assert_eq!(spellings(&bus, &refs), vec![b"".to_vec()]);
}

#[test]
fn conditional_hard_errors_fail() {
    for raw in [
        b"#if 1/0\nint a;\n#endif\n".to_vec(),
        b"#if 1 << 200\nint a;\n#endif\n".to_vec(),
        b"#if 'a'\nint a;\n#endif\n".to_vec(),
        b"#if 1\nint a;\n".to_vec(),
        b"#endif\nint a;\n".to_vec(),
        b"#else\nint a;\n#endif\n".to_vec(),
        b"#if\nint a;\n#endif\n".to_vec(),
        b"#ifdef\nint a;\n#endif\n".to_vec(),
    ] {
        let mut bus = new_bus();
        install_pp(&mut bus);
        let name = bus.intern_name(b"cond.c").unwrap();
        let source = bus.alloc_source(name, raw).unwrap();
        let workers = workers();
        use cc_silicon_compiler::manifest::PP01_CHIP;
        bootstrap(
            &mut bus,
            TaskKind::PREPROCESS_NORMALIZE,
            PP01_CHIP,
            Payload::from_refs(vec![RecordRef::Source(source)]),
        );
        let mut artifact = completed_artifact(&tick(&mut bus, &workers), &bus);
        for (kind, owner) in [
            (TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP),
            (TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP),
        ] {
            bootstrap(
                &mut bus,
                kind,
                owner,
                Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
            );
            artifact = completed_artifact(&tick(&mut bus, &workers), &bus);
        }
        bootstrap(
            &mut bus,
            TaskKind::PREPROCESS_SCAN,
            PP_SCAN_CHIP,
            Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
        );
        let refs = completed_records_on(&tick(&mut bus, &workers), &bus);
        bootstrap(
            &mut bus,
            TaskKind::PREPROCESS_CONDITIONAL,
            PP19_CHIP,
            Payload::from_refs(refs),
        );
        match tick(&mut bus, &workers).outcome {
            TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
            other => panic!("expected executed-with-failure, got {other:?}"),
        }
    }
}

#[test]
fn conditional_active_error_reaches_dispatch() {
    // An `#error` in an active region survives filtering for PP05.
    let (mut bus, refs) = filtered_refs(b"#error live\nint x;\n".to_vec());
    let workers = workers();
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_DIRECTIVE,
        PP05_CHIP,
        Payload::from_refs(refs),
    );
    let mut failed = false;
    for _ in 0..8 {
        if let TickOutcome::Executed { commit, .. } = tick(&mut bus, &workers).outcome {
            failed = failed || !commit.failed.is_empty();
        }
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
    }
    assert!(failed);
    // An `#error` in a dead region never reaches dispatch: Ack.
    let (mut bus, refs) = filtered_refs(b"#if 0\n#error dead\n#endif\nint x;\n".to_vec());
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_DIRECTIVE,
        PP05_CHIP,
        Payload::from_refs(refs),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.completed.len(), 1),
        other => panic!("expected executed, got {other:?}"),
    }
}

#[test]
fn conditional_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut bus = new_bus();
    install_pp(&mut bus);
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::pp_conditional_slice();
    mismatched.schema = StoreSchema::pp_slice();
    mismatched
        .registrations
        .register(
            PpConditionalChip.manifest(),
            &mismatched.schema,
            &mismatched.kinds,
        )
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::PREPROCESS_CONDITIONAL, PP19_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::PREPROCESS_CONDITIONAL,
        PP19_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            PpConditionalChip.manifest(),
            &StoreSchema::pp_slice(),
            &TaskKindRegistry::pp_directive_slice(),
        )
        .is_err());
}

#[test]
fn conditional_snapshot_replay_is_deterministic() {
    let run = || {
        let (bus, _) = filtered_refs(b"#if 1\nint x;\n#endif\n".to_vec());
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
