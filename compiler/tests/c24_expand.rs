// ============================================================================
// c24_expand.rs — Wave 2 (`/24`) PP macro-expansion slice acceptance.
//
// Covers the frozen closure: the `preprocess.macro_invoke` (local 26) and
// `preprocess.macro_substitute` (27) kinds, kind-to-stage assignment
// (stage 1), the PP09/PP12 manifests (reads + `Pp.tokens` writes with
// allowlist rows), object/function expansion, nested prescan, `#`/`##`,
// blue-paint termination, variadic-use deferral, zero-invocation
// passthrough, and the `/24` hash participation. Argument rescan and
// placements stay deferred to later slices.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins};
use cc_silicon_compiler::chips::{
    handler_for, PpCommentChip, PpConditionalChip, PpDefineChip, PpDiagnosticChip, PpDirectiveChip,
    PpInvokeChip, PpNormalizeChip, PpRedefineChip, PpScanChip, PpSpliceChip, PpSubstituteChip,
    PpUndefChip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{MacroId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    stage_of, PP05_CHIP, PP06_CHIP, PP07_CHIP, PP08_CHIP, PP09_CHIP, PP12_CHIP, PP19_CHIP,
    PP26_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP, PP_SPLICE_CHIP, STORE_OWNER_ALLOWLIST,
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
    bus.kinds = TaskKindRegistry::pp_expand_slice();
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

/// Bootstrap one PP09 task over all refs, drain the fan-out, and return
/// the stitched output refs.
fn run_invoke(bus: &mut CompilerBus, refs: Vec<RecordRef>) -> Vec<RecordRef> {
    let workers = workers();
    let task = bootstrap(
        bus,
        TaskKind::PREPROCESS_MACRO_INVOKE,
        PP09_CHIP,
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
        other => panic!("expected completed invoke, got {other:?}"),
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
fn expand_kind_stage_registry_allowlist_frozen() {
    use cc_silicon_compiler::manifest::{is_pp_expand_slice_kind, StoreSchema};
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::PREPROCESS_MACRO_INVOKE.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_MACRO_INVOKE.local(), 26);
    assert_eq!(TaskKind::PREPROCESS_MACRO_SUBSTITUTE.local(), 27);
    assert!(is_pp_expand_slice_kind(TaskKind::PREPROCESS_MACRO_INVOKE));
    assert!(is_pp_expand_slice_kind(
        TaskKind::PREPROCESS_MACRO_SUBSTITUTE
    ));
    assert!(!is_pp_expand_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_expand_slice_kind(TaskKind::PREPROCESS_MACRO_DEFINE));
    assert_eq!(stage_of(TaskKind::PREPROCESS_MACRO_INVOKE), Some(1));
    assert_eq!(stage_of(TaskKind::PREPROCESS_MACRO_SUBSTITUTE), Some(1));
    let registry = TaskKindRegistry::pp_expand_slice();
    assert_eq!(registry.len(), 40);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_MACRO_INVOKE)
            .unwrap()
            .name,
        "preprocess.macro_invoke"
    );
    assert_eq!(PpInvokeChip.manifest().id, PP09_CHIP);
    assert_eq!(PpSubstituteChip.manifest().id, PP12_CHIP);
    for (chip, kind) in [
        (PP09_CHIP, TaskKind::PREPROCESS_MACRO_INVOKE),
        (PP12_CHIP, TaskKind::PREPROCESS_MACRO_SUBSTITUTE),
    ] {
        assert!(STORE_OWNER_ALLOWLIST.contains(&(chip, StoreId::Pp, "tokens", kind)));
    }
    let _ = StoreSchema::pp_macro_slice();
}

#[test]
fn expand_function_like_with_args() {
    // `#define F(a) (a)` + `int y = F(3);` — args substitute and rescan.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"f.c",
        b"#define F(a) (a)\nint y = F(3);\n".to_vec(),
    );
    define_line(&mut bus, refs[1..9].to_vec());
    let stitched = run_invoke(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    // The call site becomes `( 3 )` (verbatim parens reused).
    assert!(got
        .windows(3)
        .any(|window| window == vec![b"(".to_vec(), b"3".to_vec(), b")".to_vec()]));
}

#[test]
fn expand_nested_prescan() {
    // `F(G)` with `G → 7`: the argument prescans before substitution.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"n.c",
        b"#define G 7\n#define F(a) (a)\nint z = F(G);\n".to_vec(),
    );
    define_line(&mut bus, refs[1..4].to_vec());
    define_line(&mut bus, refs[5..13].to_vec());
    let stitched = run_invoke(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    assert!(got
        .windows(3)
        .any(|window| window == vec![b"(".to_vec(), b"7".to_vec(), b")".to_vec()]));
}

#[test]
fn expand_stringize_and_paste() {
    // `#a` stringizes with single-space join and escapes.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"s.c",
        b"#define S(a) #a\nconst char *s = S(hi);\n".to_vec(),
    );
    define_line(&mut bus, refs[1..8].to_vec());
    let stitched = run_invoke(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    assert!(got.iter().any(|spelling| spelling == b"\"hi\""));
    // `a ## b` pastes `x`+`y` into `xy`.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"p.c",
        b"#define C(a,b) a ## b\nint q = C(x,y);\n".to_vec(),
    );
    define_line(&mut bus, refs[1..11].to_vec());
    let stitched = run_invoke(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    assert!(got.iter().any(|spelling| spelling == b"xy"));
}

#[test]
fn expand_blue_paint_terminates() {
    // Self-reference paints blue: `X` stays an identifier, no loop.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"b.c", b"#define X X\nint a = X;\n".to_vec());
    define_line(&mut bus, refs[1..4].to_vec());
    let stitched = run_invoke(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    assert_eq!(got.last().unwrap(), b"");
    assert!(got.contains(&b"X".to_vec()));
    // Mutual recursion terminates too.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"m.c",
        b"#define A B\n#define B A\nint a = A;\n".to_vec(),
    );
    define_line(&mut bus, refs[1..4].to_vec());
    define_line(&mut bus, refs[5..8].to_vec());
    let stitched = run_invoke(&mut bus, refs);
    assert!(!spellings(&bus, &stitched).is_empty());
}

#[test]
fn expand_variadic_use_is_unsupported() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"v.c",
        b"#define V(...) 1\nint a = V(2);\n".to_vec(),
    );
    define_line(&mut bus, refs[1..7].to_vec());
    assert_eq!(bus.arenas.macros.allocated(), 1);
    let workers = workers();
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_MACRO_INVOKE,
        PP09_CHIP,
        Payload::from_refs(refs),
    );
    drain(&mut bus, &workers);
    let message = failure_message(&bus, task);
    assert!(message.contains("ariadic"), "message: {message}");
}

#[test]
fn expand_arity_mismatch_fails() {
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
        TaskKind::PREPROCESS_MACRO_INVOKE,
        PP09_CHIP,
        Payload::from_refs(refs),
    );
    drain(&mut bus, &workers);
    let message = failure_message(&bus, task);
    assert!(!message.is_empty());
}

#[test]
fn expand_zero_invocation_passthrough() {
    // Directive-free M1 sources stitch to themselves with no appends.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"m1.c", b"int main(void){return 2+3;}\n".to_vec());
    let before = bus.arenas.pp_tokens.allocated();
    let stitched = run_invoke(&mut bus, refs.clone());
    assert_eq!(stitched, refs);
    assert_eq!(bus.arenas.pp_tokens.allocated(), before);
}

#[test]
fn expand_zero_param_function_needs_call() {
    // `#define F() 7`: bare `F` stays, `F()` expands.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(
        &mut bus,
        b"z.c",
        b"#define F() 7\nint a = F();\nint b = F;\n".to_vec(),
    );
    define_line(&mut bus, refs[1..6].to_vec());
    let record = bus.arenas.macros.get(MacroId::from_index(0)).unwrap();
    assert!(record.function_like);
    assert!(record.params.is_empty());
    let stitched = run_invoke(&mut bus, refs);
    let got = spellings(&bus, &stitched);
    // Two `7`s: the definition line (verbatim) plus the one expansion; the
    // bare `F` survives verbatim.
    assert_eq!(got.iter().filter(|sp| *sp == b"7").count(), 2);
    assert!(got.contains(&b"F".to_vec()));
}

#[test]
fn expand_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut bus = new_bus();
    install_pp(&mut bus);
    for (kind, chip, manifest) in [
        (
            TaskKind::PREPROCESS_MACRO_INVOKE,
            PP09_CHIP,
            PpInvokeChip.manifest(),
        ),
        (
            TaskKind::PREPROCESS_MACRO_SUBSTITUTE,
            PP12_CHIP,
            PpSubstituteChip.manifest(),
        ),
    ] {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::pp_expand_slice();
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
            PpInvokeChip.manifest(),
            &StoreSchema::pp_macro_slice(),
            &TaskKindRegistry::pp_macro_slice(),
        )
        .is_err());
}

#[test]
fn expand_snapshot_replay_is_deterministic() {
    let run = || {
        let mut bus = new_bus();
        install_pp(&mut bus);
        let refs = scan_source(&mut bus, b"d.c", b"#define X 5\nint x = X;\n".to_vec());
        define_line(&mut bus, refs[1..4].to_vec());
        run_invoke(&mut bus, refs);
        Snapshot::capture(&bus)
    };
    assert_eq!(run(), run());
}

#[test]
fn expand_object_like() {
    // `#define X 5` (post-`#`: [define,X,5]) + `int x = X;` — the use expands.
    let mut bus = new_bus();
    install_pp(&mut bus);
    let refs = scan_source(&mut bus, b"o.c", b"#define X 5\nint x = X;\n".to_vec());
    assert_eq!(refs.len(), 10);
    define_line(&mut bus, refs[1..4].to_vec());
    assert_eq!(bus.arenas.macros.allocated(), 1);
    let stitched = run_invoke(&mut bus, refs);
    assert_eq!(
        spellings(&bus, &stitched),
        vec![
            b"#".to_vec(),
            b"define".to_vec(),
            b"X".to_vec(),
            b"5".to_vec(),
            b"int".to_vec(),
            b"x".to_vec(),
            b"=".to_vec(),
            b"5".to_vec(),
            b";".to_vec(),
            b"".to_vec(),
        ]
    );
}
