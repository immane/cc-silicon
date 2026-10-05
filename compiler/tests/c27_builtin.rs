// ============================================================================
// c27_builtin.rs — Wave 3 (`/27`) PP builtin slice acceptance.
//
// Covers the frozen closure: the `preprocess.macro_builtin` (local 31)
// kind, kind-to-stage assignment (stage 1), the PP24 manifest (reads +
// `Pp.tokens` writes with its allowlist row), single-use expansion
// (`__FILE__`, `__LINE__`, `__COUNTER__`, frozen-target macros),
// replayable `__DATE__`/`__TIME__` and unknown names failing as explicit
// `Unsupported`, and the `/27` hash participation. `__COUNTER__`
// cross-tick monotonicity awaits a frozen counter carrier (the projector
// yields the task-local seed 0); dispatch order today replays the seed.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord};
use cc_silicon_compiler::chips::{
    handler_for, PpBuiltinChip, PpCommentChip, PpNormalizeChip, PpScanChip, PpSpliceChip, Worker,
    WorkerRegistry,
};
use cc_silicon_compiler::ids::{ChipId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    stage_of, PP01_CHIP, PP24_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP, PP_SPLICE_CHIP,
    STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::Snapshot;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, ResultValue, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry,
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
    use cc_silicon_compiler::manifest::StoreSchema;
    bus.kinds = TaskKindRegistry::pp_builtin_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpBuiltinChip,
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
        .register(TaskKind::PREPROCESS_MACRO_BUILTIN, PP24_CHIP, 1)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers.register(PpBuiltinChip).unwrap();
    workers
}

fn bootstrap(bus: &mut CompilerBus, kind: TaskKind, owner: ChipId, payload: Payload) -> TaskId {
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
                ResultValue::Records(refs) => refs.clone(),
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
                ResultValue::Record(RecordRef::Artifact(id)) => *id,
                other => panic!("expected artifact ref, got {other:?}"),
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

/// Find the scanned pp-token ref with an exact spelling.
fn find_ref(bus: &CompilerBus, refs: &[RecordRef], spelling: &[u8]) -> RecordRef {
    refs.iter()
        .copied()
        .find(|reference| match reference {
            RecordRef::PpToken(id) => {
                bus.arenas.pp_tokens.get(*id).unwrap().spelling.as_slice() == spelling
            }
            _ => false,
        })
        .expect("scanned stream must contain the builtin name")
}

/// Bootstrap one PP24 task over a single builtin-use ref and return the
/// single synthesized token.
fn run_builtin(bus: &mut CompilerBus, name: RecordRef) -> PpTokenRecord {
    let workers = workers();
    bootstrap(
        bus,
        TaskKind::PREPROCESS_MACRO_BUILTIN,
        PP24_CHIP,
        Payload::from_refs(vec![name]),
    );
    match tick(bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                ResultValue::Records(refs) => {
                    assert_eq!(refs.len(), 1);
                    match refs[0] {
                        RecordRef::PpToken(id) => bus.arenas.pp_tokens.get(id).unwrap().clone(),
                        other => panic!("expected pp-token ref, got {other:?}"),
                    }
                }
                other => panic!("expected records, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    }
}

/// Bootstrap one PP24 task over a single builtin-use ref and return the
/// failure message.
fn run_builtin_message(bus: &mut CompilerBus, name: RecordRef) -> String {
    let workers = workers();
    let task = bootstrap(
        bus,
        TaskKind::PREPROCESS_MACRO_BUILTIN,
        PP24_CHIP,
        Payload::from_refs(vec![name]),
    );
    tick(bus, &workers);
    failure_message(bus, task)
}

#[test]
fn builtin_kind_stage_registry_allowlist_frozen() {
    use cc_silicon_compiler::manifest::{is_pp_builtin_slice_kind, StoreSchema};
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::PREPROCESS_MACRO_BUILTIN.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_MACRO_BUILTIN.local(), 31);
    assert!(is_pp_builtin_slice_kind(TaskKind::PREPROCESS_MACRO_BUILTIN));
    assert!(!is_pp_builtin_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_builtin_slice_kind(
        TaskKind::PREPROCESS_VARIADIC_MACRO
    ));
    assert_eq!(stage_of(TaskKind::PREPROCESS_MACRO_BUILTIN), Some(1));
    let registry = TaskKindRegistry::pp_builtin_slice();
    assert_eq!(registry.len(), 44);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_MACRO_BUILTIN)
            .unwrap()
            .name,
        "preprocess.macro_builtin"
    );
    assert_eq!(PpBuiltinChip.manifest().id, PP24_CHIP);
    assert_eq!(PP24_CHIP, cc_silicon_compiler::ids::ChipId(34));
    assert!(STORE_OWNER_ALLOWLIST.contains(&(
        PP24_CHIP,
        StoreId::Pp,
        "tokens",
        TaskKind::PREPROCESS_MACRO_BUILTIN
    )));
    let _ = StoreSchema::pp_macro_slice();
    let _ = PpTokenKind::Eof;
}

#[test]
fn builtin_file_expands_source_name() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"f.c", b"int x = __FILE__;\n".to_vec());
    let name = find_ref(&bus, &refs, b"__FILE__");
    let token = run_builtin(&mut bus, name);
    assert_eq!(token.kind, PpTokenKind::StringLiteral);
    assert_eq!(token.spelling, b"\"f.c\"".to_vec());
}

#[test]
fn builtin_line_reports_physical_line() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"l.c",
        b"int a;\nint b;\nint c = __LINE__;\n".to_vec(),
    );
    let name = find_ref(&bus, &refs, b"__LINE__");
    let token = run_builtin(&mut bus, name);
    assert_eq!(token.kind, PpTokenKind::PpNumber);
    assert_eq!(token.spelling, b"3".to_vec());
}

#[test]
fn builtin_counter_seed_zero_and_replayable() {
    // No counter carrier exists on the bus, so every dispatch expands the
    // task-local seed 0; identical dispatches replay identically.
    let run = || {
        let mut bus = new_bus();
        install_pp(&mut bus);
        let refs = scan_source(&mut bus, b"k.c", b"int k = __COUNTER__;\n".to_vec());
        let name = find_ref(&bus, &refs, b"__COUNTER__");
        let token = run_builtin(&mut bus, name);
        assert_eq!(token.kind, PpTokenKind::PpNumber);
        token.spelling
    };
    assert_eq!(run(), b"0".to_vec());
    assert_eq!(run(), b"0".to_vec());
}

#[test]
fn builtin_target_macros() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"t.c",
        b"int s = __STDC__;\nint a = __aarch64__;\nint v = __STDC_VERSION__;\nint l = __LP64__;\n"
            .to_vec(),
    );
    for (name, spelling) in [
        (&b"__STDC__"[..], &b"1"[..]),
        (&b"__aarch64__"[..], &b"1"[..]),
        (&b"__STDC_VERSION__"[..], &b"201112L"[..]),
        (&b"__LP64__"[..], &b"1"[..]),
    ] {
        let target = find_ref(&bus, &refs, name);
        let token = run_builtin(&mut bus, target);
        assert_eq!(token.kind, PpTokenKind::PpNumber, "name: {name:?}");
        assert_eq!(token.spelling, spelling, "name: {name:?}");
    }
}

#[test]
fn builtin_date_time_unsupported() {
    // Replayable values need a frozen config date/time record the config
    // does not carry, so both names fail as explicit `Unsupported`.
    for name in [&b"__DATE__"[..], &b"__TIME__"[..]] {
        let mut bus = new_bus();
        install_pp(&mut bus);
        let raw = [b"int d = ".as_slice(), name, b";\n".as_slice()].concat();
        let refs = scan_source(&mut bus, b"d.c", raw);
        let target = find_ref(&bus, &refs, name);
        let message = run_builtin_message(&mut bus, target);
        assert!(message.contains("PP24"), "message: {message}");
        assert!(
            message.contains("unsupported") || message.contains("Unsupported"),
            "message: {message}"
        );
    }
}

#[test]
fn builtin_unknown_unsupported() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"u.c", b"int u = __NO_SUCH_BUILTIN__;\n".to_vec());
    let name = find_ref(&bus, &refs, b"__NO_SUCH_BUILTIN__");
    let message = run_builtin_message(&mut bus, name);
    assert!(message.contains("PP24"), "message: {message}");
    assert!(message.contains("unknown"), "message: {message}");
}

#[test]
fn builtin_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut bus = new_bus();
    install_pp(&mut bus);
    let manifest = PpBuiltinChip.manifest();
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::pp_builtin_slice();
    mismatched.schema = StoreSchema::pp_macro_slice();
    mismatched
        .registrations
        .register(manifest, &mismatched.schema, &mismatched.kinds)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::PREPROCESS_MACRO_BUILTIN, PP24_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::PREPROCESS_MACRO_BUILTIN,
        PP24_CHIP,
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
            PpBuiltinChip.manifest(),
            &StoreSchema::pp_macro_slice(),
            &TaskKindRegistry::pp_variadic_slice(),
        )
        .is_err());
    let _ = PpTokenKind::Eof;
}

#[test]
fn builtin_snapshot_replay_is_deterministic() {
    let run = || {
        let mut bus = new_bus();
        install_pp(&mut bus);
        let refs = scan_source(&mut bus, b"f.c", b"int x = __FILE__;\n".to_vec());
        let name = find_ref(&bus, &refs, b"__FILE__");
        run_builtin(&mut bus, name);
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
