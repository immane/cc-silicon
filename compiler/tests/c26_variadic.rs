// ============================================================================
// c26_variadic.rs — Wave 3 (`/26`) PP variadic slice acceptance.
//
// Covers the frozen closure: the `preprocess.variadic_macro` (local 30)
// kind, kind-to-stage assignment (stage 1), the PP16 manifest (reads +
// `Pp.tokens` writes with its allowlist row), stream fan-out + stitch,
// single-mode substitution (`...` collection, empty-tail legality,
// `__VA_OPT__` policy, `#`/`##` with prescan, blue-paint rescan),
// variadic-shaped misuse of non-variadic definitions failing, correct-arity
// non-variadic passthrough, and the `/26` hash participation. GNU
// `, ## __VA_ARGS__` swallowing and dialect gating stay deferred.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, PpTokenKind};
use cc_silicon_compiler::chips::{
    handler_for, PpCommentChip, PpConditionalChip, PpDefineChip, PpDiagnosticChip, PpDirectiveChip,
    PpInvokeChip, PpNormalizeChip, PpRedefineChip, PpScanChip, PpSpliceChip, PpSubstituteChip,
    PpUndefChip, PpVariadicChip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{MacroId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    stage_of, PP05_CHIP, PP06_CHIP, PP07_CHIP, PP08_CHIP, PP09_CHIP, PP12_CHIP, PP16_CHIP,
    PP19_CHIP, PP26_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP, PP_SPLICE_CHIP, STORE_OWNER_ALLOWLIST,
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
    bus.kinds = TaskKindRegistry::pp_variadic_slice();
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
        &PpVariadicChip,
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
        .register(TaskKind::PREPROCESS_VARIADIC_MACRO, PP16_CHIP, 1)
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
    workers.register(PpVariadicChip).unwrap();
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

/// Spellings of pp-token refs, in order.
fn spellings(bus: &CompilerBus, refs: &[RecordRef]) -> Vec<Vec<u8>> {
    refs.iter()
        .map(|reference| match reference {
            RecordRef::PpToken(id) => bus.arenas.pp_tokens.get(*id).unwrap().spelling.clone(),
            other => panic!("expected pp-token ref, got {other:?}"),
        })
        .collect()
}

/// Bootstrap one PP16 stream task over all refs, drain the fan-out, and
/// return the stitched output refs.
fn run_variadic(bus: &mut CompilerBus, refs: Vec<RecordRef>) -> Vec<RecordRef> {
    let workers = workers();
    let task = bootstrap(
        bus,
        TaskKind::PREPROCESS_VARIADIC_MACRO,
        PP16_CHIP,
        Payload::from_refs(refs),
    );
    drain(bus, &workers);
    match &bus.arenas.tasks.get(task).unwrap().state {
        cc_silicon_compiler::task::TaskState::Completed(result) => {
            match &bus.arenas.results.get(*result).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Records(refs) => refs.clone(),
                other => panic!("expected records, got {other:?}"),
            }
        }
        other => panic!("expected completed variadic, got {other:?}"),
    }
}

fn drain(bus: &mut CompilerBus, workers: &WorkerRegistry) {
    for _ in 0..24 {
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

/// Bootstrap one per-line PP06 task (post-`#` refs) and drain it.
fn define_line(bus: &mut CompilerBus, refs: Vec<RecordRef>) {
    let workers = workers();
    bootstrap(
        bus,
        TaskKind::PREPROCESS_MACRO_DEFINE,
        PP06_CHIP,
        Payload::from_refs(refs),
    );
    drain(bus, &workers);
}

#[test]
fn variadic_kind_stage_registry_allowlist_frozen() {
    use cc_silicon_compiler::manifest::{is_pp_variadic_slice_kind, StoreSchema};
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::PREPROCESS_VARIADIC_MACRO.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_VARIADIC_MACRO.local(), 30);
    assert!(is_pp_variadic_slice_kind(
        TaskKind::PREPROCESS_VARIADIC_MACRO
    ));
    assert!(!is_pp_variadic_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_variadic_slice_kind(
        TaskKind::PREPROCESS_MACRO_INVOKE
    ));
    assert_eq!(stage_of(TaskKind::PREPROCESS_VARIADIC_MACRO), Some(1));
    let registry = TaskKindRegistry::pp_variadic_slice();
    assert_eq!(registry.len(), 43);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_VARIADIC_MACRO)
            .unwrap()
            .name,
        "preprocess.variadic_macro"
    );
    assert_eq!(PpVariadicChip.manifest().id, PP16_CHIP);
    assert_eq!(PP16_CHIP, cc_silicon_compiler::ids::ChipId(33));
    assert!(STORE_OWNER_ALLOWLIST.contains(&(
        PP16_CHIP,
        StoreId::Pp,
        "tokens",
        TaskKind::PREPROCESS_VARIADIC_MACRO
    )));
    let _ = StoreSchema::pp_macro_slice();
    let _ = PpTokenKind::Eof;
}

#[test]
fn variadic_basic_collects_tail() {
    // `#define PAIR(a, ...) a __VA_ARGS__` + `PAIR(1, 2)` → `1 2`.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"p.c",
        b"#define PAIR(a, ...) a __VA_ARGS__\nint v = PAIR(1, 2);\n".to_vec(),
    );
    assert_eq!(refs.len(), 21);
    define_line(&mut bus, refs[1..10].to_vec());
    assert_eq!(bus.arenas.macros.allocated(), 1);
    let stitched = run_variadic(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    assert!(got
        .windows(2)
        .any(|window| window == vec![b"1".to_vec(), b"2".to_vec()]));
    // The call-site `PAIR` is consumed; only the definition names it.
    assert_eq!(
        got.iter().filter(|spelling| *spelling == b"PAIR").count(),
        1
    );
}

#[test]
fn variadic_empty_tail_is_legal() {
    // An empty variadic tail binds an empty `__VA_ARGS__`, never a failure.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"e.c",
        b"#define E(fmt, ...) fmt\nint e = E(x);\n".to_vec(),
    );
    assert_eq!(refs.len(), 18);
    define_line(&mut bus, refs[1..9].to_vec());
    let stitched = run_variadic(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    assert!(got
        .windows(3)
        .any(|window| window == vec![b"=".to_vec(), b"x".to_vec(), b";".to_vec()]));
}

#[test]
fn variadic_va_opt_empty_and_nonempty() {
    // `__VA_OPT__(content)` keeps content iff the tail is non-empty.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"o.c",
        b"#define W(x, ...) x __VA_OPT__(+ __VA_ARGS__)\nint a = W(1);\nint b = W(1, 2);\n"
            .to_vec(),
    );
    assert_eq!(refs.len(), 33);
    define_line(&mut bus, refs[1..14].to_vec());
    let stitched = run_variadic(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    // Empty tail: `int a = 1 ;` with no `+`.
    assert!(got
        .windows(4)
        .any(|window| window == vec![b"a".to_vec(), b"=".to_vec(), b"1".to_vec(), b";".to_vec()]));
    // Non-empty tail: `int b = 1 + 2 ;`.
    assert!(got.windows(6).any(|window| window
        == vec![
            b"b".to_vec(),
            b"=".to_vec(),
            b"1".to_vec(),
            b"+".to_vec(),
            b"2".to_vec(),
            b";".to_vec()
        ]));
}

#[test]
fn variadic_stringize_and_paste() {
    // `#a` stringizes the fixed parameter inside a variadic definition.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"s.c",
        b"#define S(a, ...) #a\nconst char *s = S(hi, z);\n".to_vec(),
    );
    assert_eq!(refs.len(), 23);
    define_line(&mut bus, refs[1..10].to_vec());
    let stitched = run_variadic(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    assert!(got.iter().any(|spelling| spelling == b"\"hi\""));
    // `a ## b` pastes the substituted fixed parameter with `b`.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"q.c",
        b"#define C(a, ...) a ## b\nint q = C(x, y);\n".to_vec(),
    );
    assert_eq!(refs.len(), 22);
    define_line(&mut bus, refs[1..11].to_vec());
    let stitched = run_variadic(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    assert!(got.iter().any(|spelling| spelling == b"xb"));
}

#[test]
fn variadic_blue_paint_terminates() {
    // Self-reference paints blue: the expansion keeps one `R`, no loop.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"r.c",
        b"#define R(x, ...) R(x)\nint r = R(1);\n".to_vec(),
    );
    assert_eq!(refs.len(), 21);
    define_line(&mut bus, refs[1..12].to_vec());
    let stitched = run_variadic(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    assert_eq!(got.last().unwrap(), b"");
    // Three `R`s: the definition name, the (verbatim) replacement `R`, and
    // the blue-painted expansion head — the self-reference never re-expands.
    assert_eq!(got.iter().filter(|spelling| *spelling == b"R").count(), 3);
}

#[test]
fn variadic_shaped_misuse_of_nonvariadic_fails() {
    // Stream mode: `F(1)` on non-variadic `#define F(a,b)` names PP16.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"a.c",
        b"#define F(a,b) a\nint x = F(1);\n".to_vec(),
    );
    define_line(&mut bus, refs[1..9].to_vec());
    let workers = workers();
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_VARIADIC_MACRO,
        PP16_CHIP,
        Payload::from_refs(refs),
    );
    drain(&mut bus, &workers);
    let message = failure_message(&bus, task);
    assert!(message.contains("PP16"), "message: {message}");
    // Single mode: a non-variadic def is PP12-owned `Unsupported`.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"u.c",
        b"#define F(a,b) a\nint x = F(1, 2);\n".to_vec(),
    );
    define_line(&mut bus, refs[1..9].to_vec());
    let mut payload = vec![RecordRef::Macro(MacroId::from_index(0))];
    payload.extend_from_slice(&refs[12..18]);
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_VARIADIC_MACRO,
        PP16_CHIP,
        Payload::from_refs(payload),
    );
    drain(&mut bus, &workers);
    let message = failure_message(&bus, task);
    assert!(message.contains("PP16"), "message: {message}");
}

#[test]
fn variadic_nonvariadic_passthrough() {
    // Correct-arity non-variadic uses pass through verbatim (PP09 owns
    // them): the stitched stream equals the input with no appends.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"n.c",
        b"#define F(a) (a)\nint y = F(3);\n".to_vec(),
    );
    define_line(&mut bus, refs[1..9].to_vec());
    let before = bus.arenas.pp_tokens.allocated();
    let stitched = run_variadic(&mut bus, refs.clone());
    assert_eq!(stitched, refs);
    assert_eq!(bus.arenas.pp_tokens.allocated(), before);
}

#[test]
fn variadic_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut bus = new_bus();
    install_pp(&mut bus);
    let manifest = PpVariadicChip.manifest();
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::pp_variadic_slice();
    mismatched.schema = StoreSchema::pp_macro_slice();
    mismatched
        .registrations
        .register(manifest, &mismatched.schema, &mismatched.kinds)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::PREPROCESS_VARIADIC_MACRO, PP16_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::PREPROCESS_VARIADIC_MACRO,
        PP16_CHIP,
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
            PpVariadicChip.manifest(),
            &StoreSchema::pp_macro_slice(),
            &TaskKindRegistry::pp_include_slice(),
        )
        .is_err());
    let _ = PpTokenKind::Eof;
}

#[test]
fn variadic_snapshot_replay_is_deterministic() {
    let run = || {
        let mut bus = new_bus();
        install_pp(&mut bus);
        let refs = scan_source(
            &mut bus,
            b"d.c",
            b"#define PAIR(a, ...) a __VA_ARGS__\nint v = PAIR(1, 2);\n".to_vec(),
        );
        define_line(&mut bus, refs[1..10].to_vec());
        run_variadic(&mut bus, refs);
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}
