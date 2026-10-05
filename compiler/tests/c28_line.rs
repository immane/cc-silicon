// ============================================================================
// c28_line.rs — Wave 3 (`/28`) PP line slice acceptance.
//
// Covers the frozen closure: the `preprocess.line_directive` (local 32)
// kind, kind-to-stage assignment (stage 1), the PP23 manifest (read-only:
// `Ack`-only, so no allowlist row), `#line number "file"?` with and
// without a file, the GNU `# lineno "file" flags?` marker (flags
// accepted and ignored), the `2^31 - 1` boundary, zero/overflow and bad
// escapes failing as typed `Invalid`, and the physical-vs-logical split
// (the owning source stays physical; the declared number/file is the
// logical location the wiring layer persists).
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord, SpanRecord};
use cc_silicon_compiler::chips::{
    handler_for, line_location, PpCommentChip, PpLineChip, PpLineInput, PpNormalizeChip,
    PpScanChip, PpSpliceChip, Worker, WorkerRegistry, PP23_TASK_KIND, PP_LINE_MAX,
};
use cc_silicon_compiler::ids::{ChipId, PpTokenId, RecordRef, SourceId, SpanId, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_pp_line_slice_kind, stage_of, PP01_CHIP, PP23_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP,
    PP_SPLICE_CHIP, STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, ResultValue, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry, TaskState,
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
    bus.kinds = TaskKindRegistry::pp_line_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpLineChip,
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
        .register(TaskKind::PREPROCESS_LINE_DIRECTIVE, PP23_CHIP, 1)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers.register(PpLineChip).unwrap();
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

/// Build a pure narrow projection from `(kind, spelling)` parts: one span
/// per token, all owned by `source`.
fn pure_input(parts: &[(PpTokenKind, &[u8])], source: SourceId) -> PpLineInput {
    let mut tokens = Vec::new();
    let mut spans = Vec::new();
    for (index, (kind, spelling)) in parts.iter().enumerate() {
        let span = SpanId::from_index(index as u32);
        spans.push((
            span,
            SpanRecord {
                source,
                start: index as u64,
                end: index as u64 + 1,
                expansion: None,
            },
        ));
        tokens.push((
            PpTokenId::from_index(index as u32),
            PpTokenRecord {
                kind: *kind,
                span,
                spelling: spelling.to_vec(),
            },
        ));
    }
    PpLineInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        spans,
    }
}

fn acked(proposals: &[Proposal]) -> bool {
    matches!(
        proposals,
        [Proposal::Complete {
            value: ResultValue::Ack,
            ..
        }]
    )
}

fn fail_message(proposals: &[Proposal]) -> String {
    match proposals {
        [Proposal::Fail { diagnostic, .. }] => diagnostic.message.clone(),
        other => panic!("expected single Fail, got {other:?}"),
    }
}

#[test]
fn line_kind_stage_registry_readonly_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::PREPROCESS_LINE_DIRECTIVE.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_LINE_DIRECTIVE.local(), 32);
    assert_eq!(PP23_TASK_KIND, TaskKind::PREPROCESS_LINE_DIRECTIVE);
    assert_eq!(PP_LINE_MAX, 2_147_483_647);
    assert!(is_pp_line_slice_kind(TaskKind::PREPROCESS_LINE_DIRECTIVE));
    assert!(!is_pp_line_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_line_slice_kind(TaskKind::PREPROCESS_MACRO_BUILTIN));
    assert_eq!(stage_of(TaskKind::PREPROCESS_LINE_DIRECTIVE), Some(1));
    let registry = TaskKindRegistry::pp_line_slice();
    assert_eq!(registry.len(), 45);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_LINE_DIRECTIVE)
            .unwrap()
            .name,
        "preprocess.line_directive"
    );
    let manifest = PpLineChip.manifest();
    assert_eq!(manifest.id, PP23_CHIP);
    assert_eq!(PP23_CHIP, cc_silicon_compiler::ids::ChipId(35));
    // Ack-only and read-only: no writes, hence no allowlist row.
    assert!(manifest.writes.is_empty());
    assert!(!STORE_OWNER_ALLOWLIST
        .iter()
        .any(|&(chip, _, _, _)| chip == PP23_CHIP));
    assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
    assert!(manifest.declares_read(StoreId::Pp, "tokens"));
    assert!(manifest.declares_read(StoreId::Sources, "spans"));
}

#[test]
fn hash_line_without_file_retains_current() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    let source = SourceId::from_index(3);
    let input = pure_input(
        &[
            (K::Punctuator, b"#"),
            (K::Identifier, b"line"),
            (K::PpNumber, b"42"),
        ],
        source,
    );
    assert!(acked(&PpLineChip.compute(&input)));
    let location = line_location(&input).unwrap();
    assert_eq!(location.source, source);
    assert_eq!(location.line, 42);
    // No file means retain-current-file: the consumer keeps the physical
    // source, so the location carries no bytes.
    assert_eq!(location.file, None);
}

#[test]
fn hash_line_with_file_reports_decoded_name() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    let source = SourceId::from_index(3);
    let input = pure_input(
        &[
            (K::Punctuator, b"#"),
            (K::Identifier, b"line"),
            (K::PpNumber, b"5"),
            (K::StringLiteral, b"\"f.h\""),
        ],
        source,
    );
    assert!(acked(&PpLineChip.compute(&input)));
    let location = line_location(&input).unwrap();
    assert_eq!(location.source, source);
    assert_eq!(location.line, 5);
    assert_eq!(location.file, Some(b"f.h".to_vec()));
}

#[test]
fn gnu_marker_accepts_and_ignores_flags() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    let source = SourceId::from_index(3);
    let flagged = pure_input(
        &[
            (K::Punctuator, b"#"),
            (K::PpNumber, b"7"),
            (K::StringLiteral, b"\"g.h\""),
            (K::PpNumber, b"1"),
            (K::PpNumber, b"2"),
        ],
        source,
    );
    assert!(acked(&PpLineChip.compute(&flagged)));
    let location = line_location(&flagged).unwrap();
    assert_eq!(location.line, 7);
    assert_eq!(location.file, Some(b"g.h".to_vec()));
    // GNU marker without a file retains the current file.
    let bare = pure_input(
        &[(K::Punctuator, b"#"), (K::PpNumber, b"9")],
        source,
    );
    assert!(acked(&PpLineChip.compute(&bare)));
    let location = line_location(&bare).unwrap();
    assert_eq!((location.line, location.file), (9, None));
}

#[test]
fn boundary_max_line_is_accepted() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    let source = SourceId::from_index(3);
    let input = pure_input(
        &[
            (K::Punctuator, b"#"),
            (K::Identifier, b"line"),
            (K::PpNumber, b"2147483647"),
        ],
        source,
    );
    assert!(acked(&PpLineChip.compute(&input)));
    assert_eq!(line_location(&input).unwrap().line, 2_147_483_647);
}

#[test]
fn zero_and_overflow_line_numbers_fail() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    let source = SourceId::from_index(3);
    for spelling in [&b"0"[..], &b"2147483648"[..], &b"99999999999"[..]] {
        let input = pure_input(
            &[
                (K::Punctuator, b"#"),
                (K::Identifier, b"line"),
                (K::PpNumber, spelling),
            ],
            source,
        );
        let message = fail_message(&PpLineChip.compute(&input));
        assert!(message.contains("out of range"), "message: {message}");
    }
}

#[test]
fn bad_escape_in_file_name_fails() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    let source = SourceId::from_index(3);
    // Only `\\` and `\"` escapes decode; `\q` is a typed failure.
    let input = pure_input(
        &[
            (K::Punctuator, b"#"),
            (K::Identifier, b"line"),
            (K::PpNumber, b"5"),
            (K::StringLiteral, b"\"a\\qb\""),
        ],
        source,
    );
    let message = fail_message(&PpLineChip.compute(&input));
    assert!(message.contains("escape"), "message: {message}");
    // A non-string token in file position is a typed failure too.
    let non_string = pure_input(
        &[
            (K::Punctuator, b"#"),
            (K::Identifier, b"line"),
            (K::PpNumber, b"5"),
            (K::Identifier, b"f"),
        ],
        source,
    );
    let message = fail_message(&PpLineChip.compute(&non_string));
    assert!(message.contains("string"), "message: {message}");
}

#[test]
fn physical_source_and_logical_location_stay_distinct() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    // The directive sits in physical source 11; it declares logical line
    // 1 of another file, e.g. right after an include return where the
    // includer re-asserts its own logical location.
    let physical = SourceId::from_index(11);
    let input = pure_input(
        &[
            (K::Punctuator, b"#"),
            (K::PpNumber, b"1"),
            (K::StringLiteral, b"\"includer.c\""),
        ],
        physical,
    );
    assert!(acked(&PpLineChip.compute(&input)));
    let location = line_location(&input).unwrap();
    assert_eq!(location.source, physical);
    assert_eq!(location.line, 1);
    assert_eq!(location.file, Some(b"includer.c".to_vec()));
}

/// Run PP01→PP02→PP03→PP04 from real source bytes; return the scanned
/// pp-token refs without the trailing EOF token.
fn scan_line_refs(bus: &mut CompilerBus, raw: Vec<u8>) -> Vec<RecordRef> {
    let name = bus.intern_name(b"line.c").unwrap();
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
    let mut refs = match tick(bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                ResultValue::Records(refs) => refs.clone(),
                other => panic!("expected records, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    // The scan appends a trailing EOF token; the directive line is the
    // prefix before it.
    match refs.pop() {
        Some(RecordRef::PpToken(id)) => {
            assert_eq!(bus.arenas.pp_tokens.get(id).unwrap().kind, PpTokenKind::Eof);
        }
        other => panic!("expected trailing EOF token, got {other:?}"),
    }
    refs
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
        TaskState::Failed(diagnostic) => bus
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
fn bus_dispatch_completes_ack_and_fails_zero() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    // A well-formed scanned directive line completes `Ack` with no bus
    // writes (the wiring layer consumes the validated location).
    let refs = scan_line_refs(&mut bus, b"#line 5 \"f.h\"\n".to_vec());
    assert_eq!(refs.len(), 4);
    let tokens_before = bus.arenas.pp_tokens.allocated();
    let workers = workers();
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_LINE_DIRECTIVE,
        PP23_CHIP,
        Payload::from_refs(refs),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            assert!(
                matches!(
                    &bus.arenas.results.get(commit.completed[0].1).unwrap().value,
                    ResultValue::Ack
                ),
                "line directive must complete Ack"
            );
        }
        other => panic!("expected executed, got {other:?}"),
    }
    assert_eq!(bus.arenas.pp_tokens.allocated(), tokens_before);
    // A zero line number fails instead of completing.
    let refs = scan_line_refs(&mut bus, b"#line 0\n".to_vec());
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_LINE_DIRECTIVE,
        PP23_CHIP,
        Payload::from_refs(refs),
    );
    tick(&mut bus, &workers);
    let message = failure_message(&bus, task);
    assert!(message.contains("out of range"), "message: {message}");
}

#[test]
fn line_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::pp_line_slice();
    mismatched.schema = StoreSchema::pp_macro_slice();
    mismatched
        .registrations
        .register(PpLineChip.manifest(), &mismatched.schema, &mismatched.kinds)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::PREPROCESS_LINE_DIRECTIVE, PP23_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::PREPROCESS_LINE_DIRECTIVE,
        PP23_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // The pre-`/28` registry does not know the line kind, so the
    // manifest is rejected there.
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            PpLineChip.manifest(),
            &StoreSchema::pp_macro_slice(),
            &TaskKindRegistry::pp_builtin_slice(),
        )
        .is_err());
}
