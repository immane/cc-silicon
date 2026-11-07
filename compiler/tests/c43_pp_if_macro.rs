// ============================================================================
// c43_pp_if_macro.rs — B3-01 (D-17) `#if`/`#elif` macro-expansion acceptance.
//
// C11 6.10.1: the controlling expression of `#if`/`#elif` is macro-replaced
// before integer evaluation (except for operands of `defined`), and only
// then are remaining identifiers replaced with 0. This suite covers a
// defined object-like macro (`-DX=1`), an undefined identifier, an empty
// definition (`defined(X)` true, but no controlling expression after
// replacement), nested object-like expansion, `defined` operand protection,
// `!X`, bounded self/mutual recursion, typed failure for malformed
// expansions, and repeat-run determinism.
//
// Candidate cases drive the real H04 `-D` seeding plus PP01→PP04→PP19→PP09→
// PP05→PP28 path. Bus cases define the macro table through the real PP06
// chip and then drive PP19 directly.
// ============================================================================

use std::path::PathBuf;
use std::process::{Command, Output};

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins};
use cc_silicon_compiler::chips::{
    handler_for, PpCommentChip, PpConditionalChip, PpDefineChip, PpNormalizeChip, PpScanChip,
    PpSpliceChip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    StoreSchema, PP01_CHIP, PP06_CHIP, PP19_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP, PP_SPLICE_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome, TickReport};
use cc_silicon_compiler::snapshot::Snapshot;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, ResultValue, TaskDraft, TaskKind, TaskKindRegistry};

const M1_BODY: &[u8] = b"int main(void){return 2+3;}\n";
const M1_EMIT_GOLDEN: &[u8] = b"int main ( void ) { return 2 + 3 ; }\n";

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "c43-{}-{}-{}",
        std::process::id(),
        test,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn write_source(dir: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("source write");
    path
}

/// Run `candidate -E` with the given macro flags over one source file.
fn run_candidate(label: &str, flags: &[&str], source: &[u8]) -> Output {
    let dir = workdir(label);
    let path = write_source(&dir, "main.c", source);
    let mut command = Command::new(binary());
    command.arg("-E");
    for flag in flags {
        command.arg(flag);
    }
    command.arg(&path).output().expect("candidate runs")
}

fn assert_emit(label: &str, flags: &[&str], source: &[u8]) {
    let output = run_candidate(label, flags, source);
    assert!(
        output.status.success(),
        "{label}: expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, M1_EMIT_GOLDEN, "{label}: emitted bytes");
    assert!(
        output.stderr.is_empty(),
        "{label}: unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Source that takes the first branch for a true controlling expression and
/// the second for a false one; the wrong branch is a hard `#error`.
fn if_branch(expression: &str) -> Vec<u8> {
    format!(
        "#if {expression}\n{}\n#else\n#error wrong branch for {expression}\n#endif\n",
        String::from_utf8_lossy(M1_BODY)
    )
    .into_bytes()
}

fn if_not_branch(expression: &str) -> Vec<u8> {
    format!(
        "#if {expression}\n#error wrong branch for {expression}\n#else\n{}\n#endif\n",
        String::from_utf8_lossy(M1_BODY)
    )
    .into_bytes()
}

// ----------------------------------------------------------------------------
// Candidate driver cases (real `-D` seeding plus the full PP prefix).
// ----------------------------------------------------------------------------

#[test]
fn candidate_if_defined_macro_takes_true_branch() {
    // The D-17 probe: without expansion this took `#else` and failed on
    // `#error`.
    assert_emit("if-x-1", &["-DX=1"], &if_branch("X"));
}

#[test]
fn candidate_if_undefined_identifier_is_zero() {
    assert_emit("if-undefined", &[], &if_not_branch("NEVER_DEFINED"));
    assert_emit(
        "if-undefined-eq-zero",
        &[],
        &if_branch("NEVER_DEFINED == 0"),
    );
}

#[test]
fn candidate_if_defined_empty_macro() {
    assert_emit("if-empty-defined", &["-DX="], &if_branch("defined(X)"));
    // An empty replacement leaves no controlling expression; C11 rejects
    // it, so the driver must fail instead of silently taking a branch.
    let output = run_candidate(
        "if-empty-expression",
        &["-DX="],
        b"#if X\nint main(void){return 2+3;}\n#endif\n",
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("malformed expression"),
        "expected a typed malformed-expression diagnostic, got: {stderr}"
    );
}

#[test]
fn candidate_if_nested_object_like_expansion() {
    assert_emit("if-nested", &["-DX=Y", "-DY=Z", "-DZ=1"], &if_branch("X"));
}

#[test]
fn candidate_if_defined_operand_survives_definition() {
    assert_emit(
        "if-defined-expanded",
        &["-DX=1"],
        &if_branch("defined(X) && X"),
    );
    assert_emit("if-defined-no-parens", &["-DX=1"], &if_branch("defined X"));
}

#[test]
fn candidate_if_logical_not_expands_macro() {
    assert_emit("if-not-true", &["-DX=1"], &if_not_branch("!X"));
    assert_emit("if-not-false", &["-DX=0"], &if_branch("!X"));
}

#[test]
fn candidate_if_self_reference_is_bounded_and_deterministic() {
    let source = &if_not_branch("X");
    let first = run_candidate("if-self-ref-1", &["-DX=X"], source);
    let second = run_candidate("if-self-ref-2", &["-DX=X"], source);
    assert!(
        first.status.success(),
        "self-reference: expected success, stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(first.stdout, M1_EMIT_GOLDEN);
    assert_eq!(first.status.code(), second.status.code());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}

#[test]
fn candidate_if_malformed_expansion_is_typed_fail() {
    let output = run_candidate(
        "if-malformed",
        &["-DX=1+"],
        b"#if X\nint main(void){return 2+3;}\n#endif\n",
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("candidate diagnostic") && stderr.contains("malformed expression"),
        "expected a typed malformed-expression diagnostic, got: {stderr}"
    );
}

// ----------------------------------------------------------------------------
// Direct PP19 cases over a PP06-built committed macro table.
// ----------------------------------------------------------------------------

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
    bus.kinds = TaskKindRegistry::pp_macro_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpDefineChip,
        &PpConditionalChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    for (kind, owner) in [
        (TaskKind::PREPROCESS_NORMALIZE, PP01_CHIP),
        (TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP),
        (TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP),
        (TaskKind::PREPROCESS_SCAN, PP_SCAN_CHIP),
        (TaskKind::PREPROCESS_MACRO_DEFINE, PP06_CHIP),
        (TaskKind::PREPROCESS_CONDITIONAL, PP19_CHIP),
    ] {
        bus.routing.register(kind, owner, 1).unwrap();
    }
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers.register(PpDefineChip).unwrap();
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

fn tick(bus: &mut CompilerBus, workers: &WorkerRegistry) -> TickReport {
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
    completed_records(&tick(bus, &workers), bus)
}

/// Define one object-like macro through the real PP06 chip. An empty
/// `replacement` produces an empty definition (`#define X`).
fn define_macro(bus: &mut CompilerBus, workers: &WorkerRegistry, name: &str, replacement: &str) {
    let line = if replacement.is_empty() {
        format!("#define {name}\n")
    } else {
        format!("#define {name} {replacement}\n")
    };
    let refs = scan_source(bus, b"define.h", line.into_bytes());
    assert!(refs.len() >= 3, "define line must carry name and EOF");
    bootstrap(
        bus,
        TaskKind::PREPROCESS_MACRO_DEFINE,
        PP06_CHIP,
        Payload::from_refs(refs[1..refs.len() - 1].to_vec()),
    );
    let report = tick(bus, workers);
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.completed.len(), 1),
        other => panic!("expected define execution, got {other:?}"),
    }
}

/// Build a bus with the given object-like definitions committed in order.
fn seeded_bus(definitions: &[(&str, &str)]) -> (CompilerBus, WorkerRegistry) {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let workers = workers();
    for (name, replacement) in definitions {
        define_macro(&mut bus, &workers, name, replacement);
    }
    (bus, workers)
}

fn run_conditional(bus: &mut CompilerBus, workers: &WorkerRegistry, raw: &[u8]) -> TickReport {
    let refs = scan_source(bus, b"cond.c", raw.to_vec());
    bootstrap(
        bus,
        TaskKind::PREPROCESS_CONDITIONAL,
        PP19_CHIP,
        Payload::from_refs(refs),
    );
    tick(bus, workers)
}

fn completed_artifact(
    report: &TickReport,
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

fn completed_records(report: &TickReport, bus: &CompilerBus) -> Vec<RecordRef> {
    match &report.outcome {
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

fn failure_message(report: &TickReport, bus: &CompilerBus) -> String {
    match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 1);
            bus.arenas
                .diagnostics
                .get(commit.failed[0].1)
                .unwrap()
                .message
                .clone()
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
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

fn line_of(identifier: &[u8]) -> Vec<Vec<u8>> {
    vec![
        b"int".to_vec(),
        identifier.to_vec(),
        b";".to_vec(),
        b"".to_vec(),
    ]
}

#[test]
fn pp19_expands_defined_object_like_macro() {
    let (mut bus, workers) = seeded_bus(&[("X", "1")]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if X\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"a")
    );
}

#[test]
fn pp19_expands_macro_in_elif() {
    let (mut bus, workers) = seeded_bus(&[("X", "1")]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if 0\nint a;\n#elif X\nint b;\n#else\nint c;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"b")
    );
}

#[test]
fn pp19_undefined_identifier_is_zero() {
    let (mut bus, workers) = seeded_bus(&[]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if X\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"b")
    );
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if X == 0\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"a")
    );
}

#[test]
fn pp19_defined_empty_macro_is_defined_but_yields_no_expression() {
    // `defined(X)` sees the raw name and is true for an empty definition.
    let (mut bus, workers) = seeded_bus(&[("X", "")]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if defined(X)\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"a")
    );
    // After replacement the expression is empty, which C11 rejects; this is
    // a typed failure, never a silent zero.
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if X\nint a;\n#else\nint b;\n#endif\n",
    );
    let message = failure_message(&report, &bus);
    assert!(
        message.contains("malformed expression"),
        "message: {message}"
    );
}

#[test]
fn pp19_nested_object_like_expansion() {
    let (mut bus, workers) = seeded_bus(&[("X", "Y"), ("Y", "Z"), ("Z", "1")]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if X\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"a")
    );
}

#[test]
fn pp19_defined_operand_is_not_expanded() {
    let (mut bus, workers) = seeded_bus(&[("X", "1")]);
    for raw in [
        b"#if defined(X) && X\nint a;\n#else\nint b;\n#endif\n".as_slice(),
        b"#if defined X\nint a;\n#else\nint b;\n#endif\n".as_slice(),
    ] {
        let report = run_conditional(&mut bus, &workers, raw);
        assert_eq!(
            spellings(&bus, &completed_records(&report, &bus)),
            line_of(b"a")
        );
    }
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if !defined(X)\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"b")
    );
}

#[test]
fn pp19_logical_not_expands_macro() {
    let (mut bus, workers) = seeded_bus(&[("X", "1")]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if !X\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"b")
    );
    let (mut bus, workers) = seeded_bus(&[("X", "0")]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if !X\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"a")
    );
}

#[test]
fn pp19_self_and_mutual_reference_are_bounded() {
    // `X -> X` leaves the painted identifier, which then evaluates as 0.
    let (mut bus, workers) = seeded_bus(&[("X", "X")]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if X\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"b")
    );
    // `X -> Y`, `Y -> X` terminates under the same paint set.
    let (mut bus, workers) = seeded_bus(&[("X", "Y"), ("Y", "X")]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if X\nint a;\n#else\nint b;\n#endif\n",
    );
    assert_eq!(
        spellings(&bus, &completed_records(&report, &bus)),
        line_of(b"b")
    );
}

#[test]
fn pp19_malformed_expansion_is_typed_fail() {
    let (mut bus, workers) = seeded_bus(&[("X", "1 +")]);
    let report = run_conditional(
        &mut bus,
        &workers,
        b"#if X\nint a;\n#else\nint b;\n#endif\n",
    );
    let message = failure_message(&report, &bus);
    assert!(
        message.contains("malformed expression"),
        "message: {message}"
    );
}

#[test]
fn pp19_macro_expansion_repeat_run_is_deterministic() {
    let run = || {
        let (mut bus, workers) = seeded_bus(&[("X", "Y"), ("Y", "Z"), ("Z", "1")]);
        let report = run_conditional(
            &mut bus,
            &workers,
            b"#if X\nint a;\n#else\nint b;\n#endif\n",
        );
        let spellings = spellings(&bus, &completed_records(&report, &bus));
        (Snapshot::capture(&bus), spellings)
    };
    assert_eq!(run(), run());
}
