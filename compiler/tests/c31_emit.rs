// ============================================================================
// c31_emit.rs — Wave 3 (`/31`) PP emit slice acceptance.
//
// Covers the frozen closure: the `preprocess.emit` (local 35) kind,
// kind-to-stage assignment (stage 1), the PP28 manifest plus its
// `Artifacts fragments` allowlist row (single `Preprocessed` append),
// directive-strip (`#`/`%:` lines dropped), no-gluing (`+ +` never
// becomes `++`), byte-identical string preservation, a valid
// `raw_offsets` map (`check_map`), the empty-stream terminal newline,
// multiline newline separation, the re-lex roundtrip through `scan`,
// and bus dispatch (`Record(Artifact)`).
// ============================================================================

use cc_silicon_compiler::bus::{
    ArtifactKind, ArtifactRecord, CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord, SpanRecord,
};
use cc_silicon_compiler::chips::{
    emit_preprocessed, handler_for, scan, PpCommentChip, PpEmitChip, PpEmitInput, PpNormalizeChip,
    PpScanChip, PpSpliceChip, Worker, WorkerRegistry, PP28_TASK_KIND,
};
use cc_silicon_compiler::ids::{PpTokenId, RecordRef, SourceId, SpanId, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_pp_emit_slice_kind, stage_of, PP01_CHIP, PP28_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP,
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
    bus.kinds = TaskKindRegistry::pp_emit_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpEmitChip,
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
        .register(TaskKind::PREPROCESS_EMIT, PP28_CHIP, 1)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers.register(PpEmitChip).unwrap();
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

/// Build a pure narrow projection from committed-record parts.
fn pure_input(
    tokens: Vec<(PpTokenId, PpTokenRecord)>,
    spans: Vec<(SpanId, SpanRecord)>,
    sources: Vec<(SourceId, Vec<u8>)>,
    primary: SourceId,
) -> PpEmitInput {
    PpEmitInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        spans,
        sources,
        primary,
        artifacts_allocated: 0,
        max_source_bytes: Limits::fixture().max_source_bytes,
    }
}

fn token_record(kind: PpTokenKind, span: SpanId, spelling: &[u8]) -> PpTokenRecord {
    PpTokenRecord {
        kind,
        span,
        spelling: spelling.to_vec(),
    }
}

fn span_record(source: SourceId, start: u64, end: u64) -> SpanRecord {
    SpanRecord {
        source,
        start,
        end,
        expansion: None,
    }
}

/// Projected pp-token bodies in payload order.
type TokenVec = Vec<(PpTokenId, PpTokenRecord)>;
/// Projected span records in first-use order.
type SpanVec = Vec<(SpanId, SpanRecord)>;

/// Single-line stream `int x ;` over `source` (token ids `t0..`, span ids
/// `s0..`, plus a trailing `Eof`).
fn int_x_stream(
    source: SourceId,
    raw: Vec<u8>,
    base_token: u32,
    base_span: u32,
) -> (TokenVec, SpanVec) {
    let spans = vec![
        (SpanId::from_index(base_span), span_record(source, 0, 3)),
        (SpanId::from_index(base_span + 1), span_record(source, 4, 5)),
        (SpanId::from_index(base_span + 2), span_record(source, 6, 7)),
        (
            SpanId::from_index(base_span + 3),
            span_record(source, raw.len() as u64, raw.len() as u64),
        ),
    ];
    let tokens = vec![
        (
            PpTokenId::from_index(base_token),
            token_record(
                PpTokenKind::Identifier,
                SpanId::from_index(base_span),
                b"int",
            ),
        ),
        (
            PpTokenId::from_index(base_token + 1),
            token_record(
                PpTokenKind::Identifier,
                SpanId::from_index(base_span + 1),
                b"x",
            ),
        ),
        (
            PpTokenId::from_index(base_token + 2),
            token_record(
                PpTokenKind::Punctuator,
                SpanId::from_index(base_span + 2),
                b";",
            ),
        ),
        (
            PpTokenId::from_index(base_token + 3),
            token_record(PpTokenKind::Eof, SpanId::from_index(base_span + 3), b""),
        ),
    ];
    let _ = raw;
    (tokens, spans)
}

fn fail_message(proposals: &[Proposal]) -> String {
    match proposals {
        [Proposal::Fail { diagnostic, .. }] => diagnostic.message.clone(),
        other => panic!("expected single Fail, got {other:?}"),
    }
}

#[test]
fn emit_kind_stage_registry_manifest_allowlist_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(TaskKind::PREPROCESS_EMIT.group(), TaskGroup::PREPROCESS);
    assert_eq!(TaskKind::PREPROCESS_EMIT.local(), 35);
    assert_eq!(PP28_TASK_KIND, TaskKind::PREPROCESS_EMIT);
    assert!(is_pp_emit_slice_kind(TaskKind::PREPROCESS_EMIT));
    assert!(!is_pp_emit_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_emit_slice_kind(TaskKind::PREPROCESS_EXPAND_MAP));
    assert_eq!(stage_of(TaskKind::PREPROCESS_EMIT), Some(1));
    let registry = TaskKindRegistry::pp_emit_slice();
    assert_eq!(registry.len(), 48);
    assert_eq!(
        registry.lookup(TaskKind::PREPROCESS_EMIT).unwrap().name,
        "preprocess.emit"
    );
    let manifest = PpEmitChip.manifest();
    assert_eq!(manifest.id, PP28_CHIP);
    assert_eq!(PP28_CHIP, cc_silicon_compiler::ids::ChipId(38));
    // Single `Preprocessed` append: exactly one write, with an allowlist row.
    assert_eq!(
        manifest.writes,
        vec![cc_silicon_compiler::manifest::FieldPath::new(
            StoreId::Artifacts,
            "fragments"
        )]
    );
    assert!(STORE_OWNER_ALLOWLIST
        .iter()
        .any(|&(chip, store, field, kind)| {
            chip == PP28_CHIP
                && store == StoreId::Artifacts
                && field == "fragments"
                && kind == TaskKind::PREPROCESS_EMIT
        }));
    assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
    assert!(manifest.declares_read(StoreId::Pp, "tokens"));
    assert!(manifest.declares_read(StoreId::Sources, "spans"));
    assert!(manifest.declares_read(StoreId::Sources, "bytes"));
    assert!(manifest.declares_read(StoreId::Artifacts, "fragments"));
    assert!(manifest.declares_read(StoreId::Config, "limits"));
}

#[test]
fn directive_lines_are_stripped() {
    // `#define X 1` occupies the first physical line; only `int x ;`
    // survives emission.
    let source = SourceId::from_index(0);
    let raw = b"#define X 1\nint x ;\n".to_vec();
    let spans = vec![
        (SpanId::from_index(0), span_record(source, 0, 1)),
        (SpanId::from_index(1), span_record(source, 1, 7)),
        (SpanId::from_index(2), span_record(source, 8, 9)),
        (SpanId::from_index(3), span_record(source, 10, 11)),
        (SpanId::from_index(4), span_record(source, 12, 15)),
        (SpanId::from_index(5), span_record(source, 16, 17)),
        (SpanId::from_index(6), span_record(source, 18, 19)),
        (SpanId::from_index(7), span_record(source, 20, 20)),
    ];
    let tokens = vec![
        (
            PpTokenId::from_index(0),
            token_record(PpTokenKind::Punctuator, SpanId::from_index(0), b"#"),
        ),
        (
            PpTokenId::from_index(1),
            token_record(PpTokenKind::Identifier, SpanId::from_index(1), b"define"),
        ),
        (
            PpTokenId::from_index(2),
            token_record(PpTokenKind::Identifier, SpanId::from_index(2), b"X"),
        ),
        (
            PpTokenId::from_index(3),
            token_record(PpTokenKind::PpNumber, SpanId::from_index(3), b"1"),
        ),
        (
            PpTokenId::from_index(4),
            token_record(PpTokenKind::Identifier, SpanId::from_index(4), b"int"),
        ),
        (
            PpTokenId::from_index(5),
            token_record(PpTokenKind::Identifier, SpanId::from_index(5), b"x"),
        ),
        (
            PpTokenId::from_index(6),
            token_record(PpTokenKind::Punctuator, SpanId::from_index(6), b";"),
        ),
        (
            PpTokenId::from_index(7),
            token_record(PpTokenKind::Eof, SpanId::from_index(7), b""),
        ),
    ];
    let input = pure_input(tokens, spans, vec![(source, raw)], source);
    let proposals = PpEmitChip.compute(&input);
    match &proposals[..] {
        [Proposal::AppendRecords { batch, .. }, Proposal::Complete { value, .. }] => {
            assert_eq!(batch.bodies.len(), 1);
            assert!(matches!(value, ResultValue::Record(_)));
        }
        other => panic!("expected append plus record completion, got {other:?}"),
    }
    let emitted = emit_preprocessed(&input.tokens, &input.spans, &input.sources, input.primary)
        .expect("emit succeeds");
    assert_eq!(emitted.bytes, b"int x ;\n");
}

#[test]
fn plus_plus_is_never_glued() {
    // `+ +` stays two tokens: the output contains `+ +`, never `++`.
    let source = SourceId::from_index(0);
    let raw = b"a + + b\n".to_vec();
    let spans = vec![
        (SpanId::from_index(0), span_record(source, 0, 1)),
        (SpanId::from_index(1), span_record(source, 2, 3)),
        (SpanId::from_index(2), span_record(source, 4, 5)),
        (SpanId::from_index(3), span_record(source, 6, 7)),
        (SpanId::from_index(4), span_record(source, 8, 8)),
    ];
    let tokens = vec![
        (
            PpTokenId::from_index(0),
            token_record(PpTokenKind::Identifier, SpanId::from_index(0), b"a"),
        ),
        (
            PpTokenId::from_index(1),
            token_record(PpTokenKind::Punctuator, SpanId::from_index(1), b"+"),
        ),
        (
            PpTokenId::from_index(2),
            token_record(PpTokenKind::Punctuator, SpanId::from_index(2), b"+"),
        ),
        (
            PpTokenId::from_index(3),
            token_record(PpTokenKind::Identifier, SpanId::from_index(3), b"b"),
        ),
        (
            PpTokenId::from_index(4),
            token_record(PpTokenKind::Eof, SpanId::from_index(4), b""),
        ),
    ];
    let input = pure_input(tokens, spans, vec![(source, raw)], source);
    let emitted = emit_preprocessed(&input.tokens, &input.spans, &input.sources, input.primary)
        .expect("emit succeeds");
    assert_eq!(emitted.bytes, b"a + + b\n");
    let text = String::from_utf8(emitted.bytes.clone()).expect("ascii output");
    assert!(text.contains("+ +"), "output: {text}");
    assert!(!text.contains("++"), "output: {text}");
}

#[test]
fn string_spelling_is_preserved_byte_identical() {
    // Escapes are never unescaped or requoted: the literal travels through
    // emission byte-identical.
    let source = SourceId::from_index(0);
    let raw = b"x \"a\\nb\" ;\n".to_vec();
    let spans = vec![
        (SpanId::from_index(0), span_record(source, 0, 1)),
        (SpanId::from_index(1), span_record(source, 2, 8)),
        (SpanId::from_index(2), span_record(source, 9, 10)),
        (SpanId::from_index(3), span_record(source, 11, 11)),
    ];
    let spelling = b"\"a\\nb\"";
    let tokens = vec![
        (
            PpTokenId::from_index(0),
            token_record(PpTokenKind::Identifier, SpanId::from_index(0), b"x"),
        ),
        (
            PpTokenId::from_index(1),
            token_record(PpTokenKind::StringLiteral, SpanId::from_index(1), spelling),
        ),
        (
            PpTokenId::from_index(2),
            token_record(PpTokenKind::Punctuator, SpanId::from_index(2), b";"),
        ),
        (
            PpTokenId::from_index(3),
            token_record(PpTokenKind::Eof, SpanId::from_index(3), b""),
        ),
    ];
    let input = pure_input(tokens, spans, vec![(source, raw)], source);
    let emitted = emit_preprocessed(&input.tokens, &input.spans, &input.sources, input.primary)
        .expect("emit succeeds");
    assert_eq!(emitted.bytes, b"x \"a\\nb\" ;\n");
    assert!(
        emitted.bytes.windows(spelling.len()).any(|w| w == spelling),
        "escaped literal must survive verbatim"
    );
}

#[test]
fn raw_offsets_map_is_valid() {
    // The output-boundary map satisfies `check_map` by construction:
    // length `bytes + 1`, starts at zero, monotonic, ends within source.
    let source = SourceId::from_index(0);
    let raw = b"a + + b\n".to_vec();
    let primary_len = raw.len() as u64;
    let spans = vec![
        (SpanId::from_index(0), span_record(source, 0, 1)),
        (SpanId::from_index(1), span_record(source, 2, 3)),
        (SpanId::from_index(2), span_record(source, 4, 5)),
        (SpanId::from_index(3), span_record(source, 6, 7)),
        (SpanId::from_index(4), span_record(source, 8, 8)),
    ];
    let tokens = vec![
        (
            PpTokenId::from_index(0),
            token_record(PpTokenKind::Identifier, SpanId::from_index(0), b"a"),
        ),
        (
            PpTokenId::from_index(1),
            token_record(PpTokenKind::Punctuator, SpanId::from_index(1), b"+"),
        ),
        (
            PpTokenId::from_index(2),
            token_record(PpTokenKind::Punctuator, SpanId::from_index(2), b"+"),
        ),
        (
            PpTokenId::from_index(3),
            token_record(PpTokenKind::Identifier, SpanId::from_index(3), b"b"),
        ),
        (
            PpTokenId::from_index(4),
            token_record(PpTokenKind::Eof, SpanId::from_index(4), b""),
        ),
    ];
    let input = pure_input(tokens, spans, vec![(source, raw)], source);
    let emitted = emit_preprocessed(&input.tokens, &input.spans, &input.sources, input.primary)
        .expect("emit succeeds");
    assert_eq!(
        emitted.raw_offsets.len() as u64,
        emitted.bytes.len() as u64 + 1
    );
    assert_eq!(emitted.raw_offsets.first(), Some(&0));
    let mut prev = 0u64;
    for offset in &emitted.raw_offsets {
        assert!(*offset >= prev, "map must be monotonic");
        prev = *offset;
    }
    let record = ArtifactRecord {
        kind: ArtifactKind::Preprocessed,
        source: Some(source),
        bytes: emitted.bytes,
        raw_offsets: emitted.raw_offsets,
    };
    assert!(record.check_map(primary_len).is_ok());
}

#[test]
fn empty_stream_emits_only_the_terminal_newline() {
    // An `Eof`-only stream still closes with exactly one newline.
    let source = SourceId::from_index(0);
    let spans = vec![(SpanId::from_index(0), span_record(source, 0, 0))];
    let tokens = vec![(
        PpTokenId::from_index(0),
        token_record(PpTokenKind::Eof, SpanId::from_index(0), b""),
    )];
    let input = pure_input(tokens, spans, vec![(source, vec![])], source);
    let emitted = emit_preprocessed(&input.tokens, &input.spans, &input.sources, input.primary)
        .expect("emit succeeds");
    assert_eq!(emitted.bytes, b"\n");
    assert_eq!(emitted.raw_offsets, vec![0, 0]);
}

#[test]
fn multiline_tokens_keep_newline_separation() {
    // Tokens on distinct physical lines are joined with a newline, so the
    // output keeps one token line per input line plus the terminal newline.
    let source = SourceId::from_index(0);
    let raw = b"int x ;\nint y ;\n".to_vec();
    let spans = vec![
        (SpanId::from_index(0), span_record(source, 0, 3)),
        (SpanId::from_index(1), span_record(source, 4, 5)),
        (SpanId::from_index(2), span_record(source, 6, 7)),
        (SpanId::from_index(3), span_record(source, 8, 11)),
        (SpanId::from_index(4), span_record(source, 12, 13)),
        (SpanId::from_index(5), span_record(source, 14, 15)),
        (SpanId::from_index(6), span_record(source, 16, 16)),
    ];
    let tokens = vec![
        (
            PpTokenId::from_index(0),
            token_record(PpTokenKind::Identifier, SpanId::from_index(0), b"int"),
        ),
        (
            PpTokenId::from_index(1),
            token_record(PpTokenKind::Identifier, SpanId::from_index(1), b"x"),
        ),
        (
            PpTokenId::from_index(2),
            token_record(PpTokenKind::Punctuator, SpanId::from_index(2), b";"),
        ),
        (
            PpTokenId::from_index(3),
            token_record(PpTokenKind::Identifier, SpanId::from_index(3), b"int"),
        ),
        (
            PpTokenId::from_index(4),
            token_record(PpTokenKind::Identifier, SpanId::from_index(4), b"y"),
        ),
        (
            PpTokenId::from_index(5),
            token_record(PpTokenKind::Punctuator, SpanId::from_index(5), b";"),
        ),
        (
            PpTokenId::from_index(6),
            token_record(PpTokenKind::Eof, SpanId::from_index(6), b""),
        ),
    ];
    let input = pure_input(tokens, spans, vec![(source, raw)], source);
    let emitted = emit_preprocessed(&input.tokens, &input.spans, &input.sources, input.primary)
        .expect("emit succeeds");
    assert_eq!(emitted.bytes, b"int x ;\nint y ;\n");
}

#[test]
fn emitted_bytes_relex_to_the_same_stream() {
    // Re-lex guarantee: `scan` over the emitted bytes reads back the same
    // token kinds and spellings in the same order.
    let source = SourceId::from_index(0);
    let raw = b"int x ;\nint y ;\n".to_vec();
    let spans = vec![
        (SpanId::from_index(0), span_record(source, 0, 3)),
        (SpanId::from_index(1), span_record(source, 4, 5)),
        (SpanId::from_index(2), span_record(source, 6, 7)),
        (SpanId::from_index(3), span_record(source, 8, 11)),
        (SpanId::from_index(4), span_record(source, 12, 13)),
        (SpanId::from_index(5), span_record(source, 14, 15)),
        (SpanId::from_index(6), span_record(source, 16, 16)),
    ];
    let spellings: Vec<(&[u8], PpTokenKind)> = vec![
        (b"int", PpTokenKind::Identifier),
        (b"x", PpTokenKind::Identifier),
        (b";", PpTokenKind::Punctuator),
        (b"int", PpTokenKind::Identifier),
        (b"y", PpTokenKind::Identifier),
        (b";", PpTokenKind::Punctuator),
    ];
    let tokens: Vec<(PpTokenId, PpTokenRecord)> = spellings
        .iter()
        .enumerate()
        .map(|(index, (spelling, kind))| {
            (
                PpTokenId::from_index(index as u32),
                token_record(*kind, SpanId::from_index(index as u32), spelling),
            )
        })
        .chain(std::iter::once((
            PpTokenId::from_index(6),
            token_record(PpTokenKind::Eof, SpanId::from_index(6), b""),
        )))
        .collect();
    let input = pure_input(tokens, spans, vec![(source, raw)], source);
    let emitted = emit_preprocessed(&input.tokens, &input.spans, &input.sources, input.primary)
        .expect("emit succeeds");
    let rescanned = scan(&emitted.bytes).expect("emitted bytes re-lex");
    assert_eq!(rescanned.len(), spellings.len());
    for (scanned, (spelling, kind)) in rescanned.iter().zip(spellings.iter()) {
        assert_eq!(&scanned.kind, kind);
        assert_eq!(&emitted.bytes[scanned.start..scanned.end], *spelling);
    }
}

/// Run PP01→PP02→PP03→PP04 from real source bytes; return ALL scanned
/// pp-token refs including the trailing EOF token (the chip drops `Eof`
/// tokens itself).
fn scan_full_refs(bus: &mut CompilerBus, raw: Vec<u8>) -> Vec<RecordRef> {
    let name = bus.intern_name(b"emit.c").unwrap();
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
    // The scan appends a trailing EOF token; the emit payload keeps it:
    // the chip drops `Eof` tokens itself.
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

#[test]
fn bus_dispatch_completes_record_of_preprocessed_artifact() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    let full = scan_full_refs(&mut bus, b"int x ;\n".to_vec());
    assert!(!full.is_empty());
    let artifacts_before = bus.arenas.artifacts.allocated();
    let workers = workers();
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_EMIT,
        PP28_CHIP,
        Payload::from_refs(full),
    );
    let artifact = match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                ResultValue::Record(RecordRef::Artifact(id)) => *id,
                other => panic!("expected record completion, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    assert_eq!(bus.arenas.artifacts.allocated(), artifacts_before + 1);
    let record = bus.arenas.artifacts.get(artifact).unwrap();
    assert_eq!(record.kind, ArtifactKind::Preprocessed);
    assert_eq!(record.bytes, b"int x ;\n");
    let source_len = bus
        .arenas
        .sources
        .get(record.source.unwrap())
        .unwrap()
        .bytes
        .len() as u64;
    assert!(record.check_map(source_len).is_ok());
    // The emitted bytes re-lex to the same scanned stream.
    let rescanned = scan(&record.bytes).expect("artifact bytes re-lex");
    assert_eq!(rescanned.len(), 3);
}

/// Run PP01→PP02→PP03→PP04 from real source bytes; return ALL scanned
/// pp-token refs including the trailing EOF token.
#[test]
fn emit_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::pp_emit_slice();
    mismatched.schema = StoreSchema::pp_macro_slice();
    mismatched
        .registrations
        .register(PpEmitChip.manifest(), &mismatched.schema, &mismatched.kinds)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::PREPROCESS_EMIT, PP28_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::PREPROCESS_EMIT,
        PP28_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // The pre-`/31` registry does not know the emit kind, so the
    // manifest is rejected there.
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            PpEmitChip.manifest(),
            &StoreSchema::pp_macro_slice(),
            &TaskKindRegistry::pp_expand_map_slice(),
        )
        .is_err());
}

#[test]
fn int_x_stream_helper_builds_expected_tokens() {
    // Keeps the shared `int x ;` helper honest for future tests, and pins
    // the non-`Running` typed failure path.
    let source = SourceId::from_index(0);
    let raw = b"int x ;\n".to_vec();
    let (tokens, spans) = int_x_stream(source, raw.clone(), 0, 0);
    assert_eq!(tokens.len(), 4);
    assert_eq!(spans.len(), 4);
    let input = pure_input(tokens, spans, vec![(source, raw)], source);
    let emitted = emit_preprocessed(&input.tokens, &input.spans, &input.sources, input.primary)
        .expect("emit succeeds");
    assert_eq!(emitted.bytes, b"int x ;\n");
    let mut waiting = pure_input(vec![], vec![], vec![(source, vec![])], source);
    waiting.state = TaskState::Ready;
    let message = fail_message(&PpEmitChip.compute(&waiting));
    assert!(message.contains("not running"), "message: {message}");
}
