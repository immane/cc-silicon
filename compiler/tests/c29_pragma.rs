// ============================================================================
// c29_pragma.rs — Wave 3 (`/29`) PP pragma slice acceptance.
//
// Covers the frozen closure: the `preprocess.pragma_directive` (local 33)
// kind, kind-to-stage assignment (stage 1), the PP25 manifest (read-only:
// `Ack`-only, so no allowlist row), `#pragma once` (header-guard flag),
// `#pragma pack(push/pop)` (alignment-stack op), well-formed unknown
// pragmas completing `Ack` (benign ignore per C11 6.10.6p1), malformed
// `_Pragma` operands failing as typed `Invalid`, `_Pragma("...")` string
// decoding (quotes stripped, simple escapes mapped, `u8`/`u`/`U`/`L`
// prefixes tolerated), and bus dispatch (`Ack`, no writes; malformed
// fails).
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord};
use cc_silicon_compiler::chips::{
    classify_directive_params, classify_operator_text, decode_pragma_string, handler_for,
    PpCommentChip, PpNormalizeChip, PpPragmaChip, PpPragmaInput, PpScanChip, PpSpliceChip, Worker,
    WorkerRegistry, PP25_TASK_KIND,
};
use cc_silicon_compiler::ids::{ChipId, PpTokenId, RecordRef, SpanId, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_pp_pragma_slice_kind, stage_of, PP01_CHIP, PP25_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP,
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
    bus.kinds = TaskKindRegistry::pp_pragma_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpPragmaChip,
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
        .register(TaskKind::PREPROCESS_PRAGMA_DIRECTIVE, PP25_CHIP, 1)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers.register(PpPragmaChip).unwrap();
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

/// Build a pure narrow projection from `(kind, spelling)` parts.
fn pure_input(parts: &[(PpTokenKind, &[u8])]) -> PpPragmaInput {
    let mut tokens = Vec::new();
    for (index, (kind, spelling)) in parts.iter().enumerate() {
        tokens.push((
            PpTokenId::from_index(index as u32),
            PpTokenRecord {
                kind: *kind,
                span: SpanId::from_index(index as u32),
                spelling: spelling.to_vec(),
            },
        ));
    }
    PpPragmaInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
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
fn pragma_kind_stage_registry_readonly_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::PREPROCESS_PRAGMA_DIRECTIVE.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_PRAGMA_DIRECTIVE.local(), 33);
    assert_eq!(PP25_TASK_KIND, TaskKind::PREPROCESS_PRAGMA_DIRECTIVE);
    assert!(is_pp_pragma_slice_kind(
        TaskKind::PREPROCESS_PRAGMA_DIRECTIVE
    ));
    assert!(!is_pp_pragma_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_pragma_slice_kind(
        TaskKind::PREPROCESS_LINE_DIRECTIVE
    ));
    assert_eq!(stage_of(TaskKind::PREPROCESS_PRAGMA_DIRECTIVE), Some(1));
    let registry = TaskKindRegistry::pp_pragma_slice();
    assert_eq!(registry.len(), 46);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_PRAGMA_DIRECTIVE)
            .unwrap()
            .name,
        "preprocess.pragma_directive"
    );
    let manifest = PpPragmaChip.manifest();
    assert_eq!(manifest.id, PP25_CHIP);
    assert_eq!(PP25_CHIP, cc_silicon_compiler::ids::ChipId(36));
    // Ack-only and read-only: no writes, hence no allowlist row.
    assert!(manifest.writes.is_empty());
    assert!(!STORE_OWNER_ALLOWLIST
        .iter()
        .any(|&(chip, _, _, _)| chip == PP25_CHIP));
    assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
    assert!(manifest.declares_read(StoreId::Pp, "tokens"));
}

#[test]
fn hash_pragma_once_is_ack() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    use cc_silicon_compiler::chips::PragmaClass;
    let input = pure_input(&[(K::Identifier, b"pragma"), (K::Identifier, b"once")]);
    assert!(acked(&PpPragmaChip.compute(&input)));
    assert_eq!(
        classify_directive_params(&input.tokens[1..]),
        PragmaClass::Once
    );
}

#[test]
fn pack_push_and_pop_are_ack() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    use cc_silicon_compiler::chips::PragmaClass;
    let push = pure_input(&[
        (K::Identifier, b"pragma"),
        (K::Identifier, b"pack"),
        (K::Punctuator, b"("),
        (K::Identifier, b"push"),
    ]);
    assert!(acked(&PpPragmaChip.compute(&push)));
    assert_eq!(
        classify_directive_params(&push.tokens[1..]),
        PragmaClass::PackPush
    );
    let pop = pure_input(&[
        (K::Identifier, b"pragma"),
        (K::Identifier, b"pack"),
        (K::Punctuator, b"("),
        (K::Identifier, b"pop"),
    ]);
    assert!(acked(&PpPragmaChip.compute(&pop)));
    assert_eq!(
        classify_directive_params(&pop.tokens[1..]),
        PragmaClass::PackPop
    );
}

#[test]
fn unknown_pragma_is_benignly_ignored() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    use cc_silicon_compiler::chips::PragmaClass;
    // Per C11 6.10.6p1 an unrecognized pragma is ignored: well-formed
    // unknown input completes `Ack`, it never fails.
    let input = pure_input(&[(K::Identifier, b"pragma"), (K::Identifier, b"foobar")]);
    assert!(acked(&PpPragmaChip.compute(&input)));
    assert_eq!(
        classify_directive_params(&input.tokens[1..]),
        PragmaClass::Opaque
    );
    // A bare `#pragma` (empty params) is well-formed and opaque too.
    let bare = pure_input(&[(K::Identifier, b"pragma")]);
    assert!(acked(&PpPragmaChip.compute(&bare)));
    assert_eq!(
        classify_directive_params(&bare.tokens[1..]),
        PragmaClass::Opaque
    );
}

#[test]
fn malformed_pragma_operand_fails() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    // Wrong arity: the operator shape needs exactly four tokens.
    let short = pure_input(&[(K::Identifier, b"_Pragma")]);
    let message = fail_message(&PpPragmaChip.compute(&short));
    assert!(message.contains("malformed"), "message: {message}");
    // Non-string operand: the third token must be a string literal.
    let non_string = pure_input(&[
        (K::Identifier, b"_Pragma"),
        (K::Punctuator, b"("),
        (K::Identifier, b"once"),
        (K::Punctuator, b")"),
    ]);
    let message = fail_message(&PpPragmaChip.compute(&non_string));
    assert!(message.contains("string literal"), "message: {message}");
    // A non-pragma first token is a wrong-dispatch protocol fault.
    let wrong = pure_input(&[(K::Identifier, b"define")]);
    let message = fail_message(&PpPragmaChip.compute(&wrong));
    assert!(message.contains("non-pragma"), "message: {message}");
}

#[test]
fn bad_pragma_string_fails_typed() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    use cc_silicon_compiler::chips::PragmaStringError;
    // A non-simple escape (`\q`) is an explicit boundary, not a guess.
    assert_eq!(
        decode_pragma_string(b"\"a\\qb\""),
        Err(PragmaStringError::UnsupportedEscape)
    );
    let escaped = pure_input(&[
        (K::Identifier, b"_Pragma"),
        (K::Punctuator, b"("),
        (K::StringLiteral, b"\"a\\qb\""),
        (K::Punctuator, b")"),
    ]);
    let message = fail_message(&PpPragmaChip.compute(&escaped));
    assert!(message.contains("unsupported escape"), "message: {message}");
    // Missing quotes cannot be given any meaning.
    assert_eq!(
        decode_pragma_string(b"once"),
        Err(PragmaStringError::Malformed)
    );
    let unquoted = pure_input(&[
        (K::Identifier, b"_Pragma"),
        (K::Punctuator, b"("),
        (K::StringLiteral, b"once"),
        (K::Punctuator, b")"),
    ]);
    let message = fail_message(&PpPragmaChip.compute(&unquoted));
    assert!(message.contains("malformed"), "message: {message}");
}

#[test]
fn operator_text_decodes_and_classifies() {
    use cc_silicon_compiler::bus::PpTokenKind as K;
    use cc_silicon_compiler::chips::PragmaClass;
    // Quotes stripped, surrounding whitespace trimmed.
    assert_eq!(decode_pragma_string(b"\"once\""), Ok(b"once".to_vec()));
    assert_eq!(classify_operator_text(b"  once  "), PragmaClass::Once);
    // An optional `u8` prefix is tolerated without transcoding.
    assert_eq!(decode_pragma_string(b"u8\"once\""), Ok(b"once".to_vec()));
    // Simple escapes decode; the operator form classifies pack too.
    assert_eq!(decode_pragma_string(b"\"a\\nb\""), Ok(b"a\nb".to_vec()));
    assert_eq!(classify_operator_text(b"pack(push)"), PragmaClass::PackPush);
    assert_eq!(
        classify_operator_text(b"pack ( pop )"),
        PragmaClass::PackPop
    );
    assert_eq!(
        classify_operator_text(b"STDC FENV_ACCESS ON"),
        PragmaClass::Opaque
    );
    // End to end through the operator token shape.
    let operator = pure_input(&[
        (K::Identifier, b"_Pragma"),
        (K::Punctuator, b"("),
        (K::StringLiteral, b"\"once\""),
        (K::Punctuator, b")"),
    ]);
    assert!(acked(&PpPragmaChip.compute(&operator)));
}

/// Run PP01→PP02→PP03→PP04 from real source bytes; return the scanned
/// pp-token refs without the trailing EOF token.
fn scan_pragma_refs(bus: &mut CompilerBus, raw: Vec<u8>) -> Vec<RecordRef> {
    let name = bus.intern_name(b"pragma.c").unwrap();
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
    // The scan appends a trailing EOF token; the pragma construct is the
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
fn bus_dispatch_completes_ack_and_fails_malformed() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    // A well-formed `#pragma once` line completes `Ack` with no bus
    // writes (the wiring layer consumes the classification). The payload
    // carries the post-`#` refs, so the leading `#` is stripped.
    let mut refs = scan_pragma_refs(&mut bus, b"#pragma once\n".to_vec());
    assert_eq!(refs.len(), 3);
    refs.remove(0);
    let tokens_before = bus.arenas.pp_tokens.allocated();
    let workers = workers();
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_PRAGMA_DIRECTIVE,
        PP25_CHIP,
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
                "pragma dispatch must complete Ack"
            );
        }
        other => panic!("expected executed, got {other:?}"),
    }
    assert_eq!(bus.arenas.pp_tokens.allocated(), tokens_before);
    // A truncated `_Pragma` operand fails instead of completing.
    let refs = scan_pragma_refs(&mut bus, b"_Pragma\n".to_vec());
    let task = bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_PRAGMA_DIRECTIVE,
        PP25_CHIP,
        Payload::from_refs(refs),
    );
    tick(&mut bus, &workers);
    let message = failure_message(&bus, task);
    assert!(message.contains("malformed"), "message: {message}");
}

#[test]
fn pragma_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::pp_pragma_slice();
    mismatched.schema = StoreSchema::pp_macro_slice();
    mismatched
        .registrations
        .register(
            PpPragmaChip.manifest(),
            &mismatched.schema,
            &mismatched.kinds,
        )
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::PREPROCESS_PRAGMA_DIRECTIVE, PP25_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::PREPROCESS_PRAGMA_DIRECTIVE,
        PP25_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // The pre-`/29` registry does not know the pragma kind, so the
    // manifest is rejected there.
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            PpPragmaChip.manifest(),
            &StoreSchema::pp_macro_slice(),
            &TaskKindRegistry::pp_line_slice(),
        )
        .is_err());
}
