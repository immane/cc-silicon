// ============================================================================
// c23_macro.rs — Wave 2 (`/23`) PP macro-definition slice acceptance.
//
// Covers the frozen closure: the `preprocess.macro_define` (local 23),
// `preprocess.macro_redefine` (24), `preprocess.macro_undef` (25) kinds,
// kind-to-stage assignment (stage 1), the Macro record family (wire tag
// 27, snapshot round-trip, capacity, predicted IDs), fresh/benign/
// non-benign lifecycles, `#undef` tombstones with redefine-after-undef,
// variadic recording, the PP19 defined-table amendment, stage/layer
// gates, and the `/23` hash participation. Expansion stays deferred.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins};
use cc_silicon_compiler::chips::{
    handler_for, PpCommentChip, PpConditionalChip, PpDefineChip, PpDiagnosticChip, PpDirectiveChip,
    PpNormalizeChip, PpRedefineChip, PpScanChip, PpSpliceChip, PpUndefChip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{MacroId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    stage_of, PP05_CHIP, PP06_CHIP, PP07_CHIP, PP08_CHIP, PP19_CHIP, PP26_CHIP, PP_COMMENT_CHIP,
    PP_SCAN_CHIP, PP_SPLICE_CHIP, STORE_OWNER_ALLOWLIST,
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
    bus.kinds = TaskKindRegistry::pp_macro_slice();
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

fn completed_macro(
    report: &cc_silicon_compiler::routing::TickReport,
    bus: &CompilerBus,
) -> MacroId {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Macro(id)) => *id,
                other => panic!("expected macro ref, got {other:?}"),
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

#[test]
fn macro_kind_stage_registry_allowlist_frozen() {
    use cc_silicon_compiler::manifest::{is_pp_macro_slice_kind, StoreSchema};
    use cc_silicon_compiler::task::{StoreId, TaskKindRegistry as Registry};
    assert_eq!(
        TaskKind::PREPROCESS_MACRO_DEFINE.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_MACRO_DEFINE.local(), 23);
    assert_eq!(TaskKind::PREPROCESS_MACRO_REDEFINE.local(), 24);
    assert_eq!(TaskKind::PREPROCESS_MACRO_UNDEF.local(), 25);
    assert!(is_pp_macro_slice_kind(TaskKind::PREPROCESS_MACRO_DEFINE));
    assert!(is_pp_macro_slice_kind(TaskKind::PREPROCESS_MACRO_REDEFINE));
    assert!(is_pp_macro_slice_kind(TaskKind::PREPROCESS_MACRO_UNDEF));
    assert!(!is_pp_macro_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_macro_slice_kind(TaskKind::PREPROCESS_CONDITIONAL));
    assert_eq!(stage_of(TaskKind::PREPROCESS_MACRO_DEFINE), Some(1));
    assert_eq!(stage_of(TaskKind::PREPROCESS_MACRO_REDEFINE), Some(1));
    assert_eq!(stage_of(TaskKind::PREPROCESS_MACRO_UNDEF), Some(1));
    let registry = Registry::pp_macro_slice();
    assert_eq!(registry.len(), 38);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_MACRO_DEFINE)
            .unwrap()
            .name,
        "preprocess.macro_define"
    );
    // Writers own their field rows; the verifier writes nothing.
    assert!(PpDefineChip.manifest().writes.len() == 1);
    assert!(PpUndefChip.manifest().writes.len() == 1);
    assert!(PpRedefineChip.manifest().writes.is_empty());
    for (chip, kind) in [
        (PP06_CHIP, TaskKind::PREPROCESS_MACRO_DEFINE),
        (PP08_CHIP, TaskKind::PREPROCESS_MACRO_UNDEF),
    ] {
        assert!(STORE_OWNER_ALLOWLIST.contains(&(chip, StoreId::Pp, "macros", kind)));
    }
    assert_eq!(PpDefineChip.manifest().id, PP06_CHIP);
    assert_eq!(PpRedefineChip.manifest().id, PP07_CHIP);
    assert_eq!(PpUndefChip.manifest().id, PP08_CHIP);
    let _ = StoreSchema::pp_macro_slice();
}

fn define_line_task(bus: &mut CompilerBus, refs: Vec<RecordRef>) -> TaskId {
    bootstrap(
        bus,
        TaskKind::PREPROCESS_MACRO_DEFINE,
        PP06_CHIP,
        Payload::from_refs(refs),
    )
}

#[test]
fn macro_object_like_fresh_define() {
    // `#define X 5`: post-`#` line refs are refs[1..4].
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"def.c", b"#define X 5\n".to_vec());
    assert_eq!(refs.len(), 5);
    let workers = workers();
    let task = define_line_task(&mut bus, refs[1..4].to_vec());
    let report = tick(&mut bus, &workers);
    let id = completed_macro(&report, &bus);
    assert_eq!(id, MacroId::from_index(0));
    assert_eq!(bus.arenas.macros.allocated(), 1);
    let record = bus.arenas.macros.get(id).unwrap();
    assert_eq!(record.spelling, b"X");
    assert!(record.params.is_empty());
    assert!(!record.variadic);
    assert!(!record.undefined);
    assert_eq!(record.replacement.len(), 1);
    let replacement = bus.arenas.pp_tokens.get(record.replacement[0]).unwrap();
    assert_eq!(replacement.spelling, b"5");
    // Snapshot round-trips the new record body.
    use cc_silicon_compiler::snapshot::{decode_macro, encode_macro};
    assert_eq!(
        decode_macro(&encode_macro(&record.clone())).unwrap(),
        record.clone()
    );
    let _ = task;
}

#[test]
fn macro_function_like_and_adjacency() {
    // Adjacent `(` opens a function-like list; spaced `(` stays object-like.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"f.c", b"#define F(a) (a)\n".to_vec());
    let workers = workers();
    // refs: [#, define, F, (, a, ), (, a, ), EOF] = 10 refs.
    assert_eq!(refs.len(), 10);
    let task = define_line_task(&mut bus, refs[1..9].to_vec());
    let report = tick(&mut bus, &workers);
    let id = completed_macro(&report, &bus);
    let record = bus.arenas.macros.get(id).unwrap();
    assert_eq!(record.spelling, b"F");
    assert_eq!(record.params, vec![b"a".to_vec()]);
    assert!(!record.variadic);
    let _ = task;
    // Spaced paren: object-like, parens land in the replacement.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"g.c", b"#define G (1)\n".to_vec());
    let _task = define_line_task(&mut bus, refs[1..6].to_vec());
    let report = tick(&mut bus, &workers);
    let id = completed_macro(&report, &bus);
    let record = bus.arenas.macros.get(id).unwrap();
    assert_eq!(record.spelling, b"G");
    assert!(record.params.is_empty());
    // Variadic definitions record the flag (use deferred to PP16).
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"v.c", b"#define V(...) __VA_ARGS__\n".to_vec());
    let task = define_line_task(&mut bus, refs[1..7].to_vec());
    let report = tick(&mut bus, &workers);
    let id = completed_macro(&report, &bus);
    let record = bus.arenas.macros.get(id).unwrap();
    assert!(record.variadic);
    let _ = task;
}

#[test]
fn macro_benign_redefine_acknowledges() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"r.c", b"#define X 5\n".to_vec());
    let workers = workers();
    let first = define_line_task(&mut bus, refs[1..4].to_vec());
    let report = tick(&mut bus, &workers);
    assert_eq!(completed_macro(&report, &bus), MacroId::from_index(0));
    // An identical second definition fans out to PP07 and acknowledges
    // with no new record.
    let second = define_line_task(&mut bus, refs[1..4].to_vec());
    drain_ticks(&mut bus, &workers);
    match &bus.arenas.tasks.get(second).unwrap().state {
        cc_silicon_compiler::task::TaskState::Completed(_) => {}
        other => panic!("expected completed, got {other:?}"),
    }
    assert_eq!(bus.arenas.macros.allocated(), 1);
    let _ = first;
}

fn drain_ticks(bus: &mut CompilerBus, workers: &WorkerRegistry) {
    for _ in 0..16 {
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
        tick(bus, workers);
    }
}

#[test]
fn macro_nonbenign_redefine_fails() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"r.c", b"#define X 5\n".to_vec());
    let workers = workers();
    let first = define_line_task(&mut bus, refs[1..4].to_vec());
    let report = tick(&mut bus, &workers);
    assert_eq!(completed_macro(&report, &bus), MacroId::from_index(0));
    // A differing replacement fails loudly through the PP07 child.
    let refs2 = scan_source(&mut bus, b"r2.c", b"#define X 6\n".to_vec());
    let second = define_line_task(&mut bus, refs2[1..4].to_vec());
    drain_ticks(&mut bus, &workers);
    let message = failure_message(&bus, second);
    assert!(message.contains("replacement"), "message: {message}");
    assert_eq!(bus.arenas.macros.allocated(), 1);
    let _ = first;
}

#[test]
fn macro_undef_lifecycle() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    // Undefining an unknown name is ignored (C rule): Ack, no append.
    let refs = scan_source(&mut bus, b"u.c", b"#undef ZED\n".to_vec());
    let _task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_MACRO_UNDEF,
        PP08_CHIP,
        Payload::from_refs(refs[1..3].to_vec()),
    );
    let report = tick(&mut bus, &workers);
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.completed.len(), 1),
        other => panic!("expected executed, got {other:?}"),
    }
    assert_eq!(bus.arenas.macros.allocated(), 0);
    // Define, then undef: a tombstone lands.
    let refs = scan_source(&mut bus, b"d.c", b"#define X 5\n".to_vec());
    let first = define_line_task(&mut bus, refs[1..4].to_vec());
    let report = tick(&mut bus, &workers);
    assert_eq!(completed_macro(&report, &bus), MacroId::from_index(0));
    let refs = scan_source(&mut bus, b"u2.c", b"#undef X\n".to_vec());
    let _task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_MACRO_UNDEF,
        PP08_CHIP,
        Payload::from_refs(refs[1..3].to_vec()),
    );
    let report = tick(&mut bus, &workers);
    let tomb = completed_macro(&report, &bus);
    assert_eq!(tomb, MacroId::from_index(1));
    assert!(bus.arenas.macros.get(tomb).unwrap().undefined);
    // Redefining after undef appends fresh (the tombstone is superseded).
    let refs = scan_source(&mut bus, b"d2.c", b"#define X 7\n".to_vec());
    let task = define_line_task(&mut bus, refs[1..4].to_vec());
    let report = tick(&mut bus, &workers);
    assert_eq!(completed_macro(&report, &bus), MacroId::from_index(2));
    assert!(
        !bus.arenas
            .macros
            .get(MacroId::from_index(2))
            .unwrap()
            .undefined
    );
    let _ = (first, task);
}

#[test]
fn macro_table_drives_conditionals() {
    // The `/22`-predicted amendment: `#ifdef` goes live once defined.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    let refs = scan_source(&mut bus, b"d.c", b"#define X 5\n".to_vec());
    let task = define_line_task(&mut bus, refs[1..4].to_vec());
    let report = tick(&mut bus, &workers);
    assert_eq!(completed_macro(&report, &bus), MacroId::from_index(0));
    // `#ifdef X` now takes its body.
    let refs = scan_source(&mut bus, b"c.c", b"#ifdef X\nint a;\n#endif\n".to_vec());
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_CONDITIONAL,
        PP19_CHIP,
        Payload::from_refs(refs),
    );
    let report = tick(&mut bus, &workers);
    let kept = match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Records(refs) => refs.clone(),
                other => panic!("expected records, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    let spellings: Vec<Vec<u8>> = kept
        .iter()
        .map(|reference| match reference {
            RecordRef::PpToken(id) => bus.arenas.pp_tokens.get(*id).unwrap().spelling.clone(),
            other => panic!("expected pp-token ref, got {other:?}"),
        })
        .collect();
    assert_eq!(
        spellings,
        vec![b"int".to_vec(), b"a".to_vec(), b";".to_vec(), b"".to_vec()]
    );
    let _ = task;
}

#[test]
fn macro_malformed_inputs_fail() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    // `#define 123`: non-identifier name.
    let refs = scan_source(&mut bus, b"m.c", b"#define 123\n".to_vec());
    let _task = define_line_task(&mut bus, refs[1..3].to_vec());
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // `#define F(a`: missing close paren.
    let refs = scan_source(&mut bus, b"m2.c", b"#define F(a\n".to_vec());
    let task = define_line_task(&mut bus, refs[1..5].to_vec());
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // `#undef A B`: operand count.
    let refs = scan_source(&mut bus, b"m3.c", b"#undef A B\n".to_vec());
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_MACRO_UNDEF,
        PP08_CHIP,
        Payload::from_refs(refs[1..4].to_vec()),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    let _ = task;
}

#[test]
fn macro_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut bus = new_bus();
    install_pp(&mut bus);
    for (kind, chip, manifest) in [
        (
            TaskKind::PREPROCESS_MACRO_DEFINE,
            PP06_CHIP,
            PpDefineChip.manifest(),
        ),
        (
            TaskKind::PREPROCESS_MACRO_REDEFINE,
            PP07_CHIP,
            PpRedefineChip.manifest(),
        ),
        (
            TaskKind::PREPROCESS_MACRO_UNDEF,
            PP08_CHIP,
            PpUndefChip.manifest(),
        ),
    ] {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::pp_macro_slice();
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
    // Manifests claiming the new kinds are rejected against the stale
    // pre-`/23` registry that does not know them.
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            PpDefineChip.manifest(),
            &StoreSchema::pp_macro_slice(),
            &TaskKindRegistry::pp_conditional_slice(),
        )
        .is_err());
}

#[test]
fn macro_snapshot_replay_is_deterministic() {
    let run = || {
        let mut bus = new_bus();
        install_pp(&mut bus);
        let refs = scan_source(&mut bus, b"d.c", b"#define X 5\n".to_vec());
        let workers = workers();
        define_line_task(&mut bus, refs[1..4].to_vec());
        tick(&mut bus, &workers);
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
