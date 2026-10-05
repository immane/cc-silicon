// ============================================================================
// c30_expand_map.rs — Wave 3 (`/30`) PP expansion-map slice acceptance.
//
// Covers the frozen closure: the `preprocess.expand_map` (local 34)
// kind, kind-to-stage assignment (stage 1), the PP27 manifest (read-only:
// `Ack`-only, so no allowlist row), the per-token origin chain (basic
// single-frame rebuild plus the unexpanded empty-frames case), the
// structural query cases (`#` stringify reads the raw `spelling` side,
// `##` paste products land on the `expanded` side, prescan-vs-raw stays
// side by side, blue-paint rescan replays `parent` links, nested include
// origins name every physical source), dangling links failing as typed
// `Invalid`, and bus dispatch (`Ack`, no writes).
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, ExpansionRecord, PpTokenKind, PpTokenRecord, SpanRecord,
};
use cc_silicon_compiler::chips::{
    handler_for, origin_chain, origin_root, PpCommentChip, PpExpandMapChip, PpExpandMapInput,
    PpNormalizeChip, PpScanChip, PpSpliceChip, Worker, WorkerRegistry, PP27_TASK_KIND,
};
use cc_silicon_compiler::ids::{
    ChipId, ExpansionId, PpTokenId, RecordRef, SourceId, SpanId, TaskId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_pp_expand_map_slice_kind, stage_of, PP01_CHIP, PP27_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP,
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
    bus.kinds = TaskKindRegistry::pp_expand_map_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpExpandMapChip,
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
        .register(TaskKind::PREPROCESS_EXPAND_MAP, PP27_CHIP, 1)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers.register(PpExpandMapChip).unwrap();
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

fn token_record(kind: PpTokenKind, span: SpanId, spelling: &[u8]) -> PpTokenRecord {
    PpTokenRecord {
        kind,
        span,
        spelling: spelling.to_vec(),
    }
}

fn span_record(
    source: SourceId,
    start: u64,
    end: u64,
    expansion: Option<ExpansionId>,
) -> SpanRecord {
    SpanRecord {
        source,
        start,
        end,
        expansion,
    }
}

fn expansion_record(
    parent: Option<ExpansionId>,
    spelling: SpanId,
    expanded: SpanId,
    ordinal: u32,
) -> ExpansionRecord {
    ExpansionRecord {
        parent,
        spelling,
        expanded,
        ordinal,
    }
}

/// Build a pure narrow projection from committed-record parts.
fn pure_input(
    tokens: Vec<(PpTokenId, PpTokenRecord)>,
    spans: Vec<(SpanId, SpanRecord)>,
    expansions: Vec<(ExpansionId, ExpansionRecord)>,
    sources: Vec<SourceId>,
) -> PpExpandMapInput {
    PpExpandMapInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        spans,
        expansions,
        sources,
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
fn expand_map_kind_stage_registry_readonly_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::PREPROCESS_EXPAND_MAP.group(),
        TaskGroup::PREPROCESS
    );
    assert_eq!(TaskKind::PREPROCESS_EXPAND_MAP.local(), 34);
    assert_eq!(PP27_TASK_KIND, TaskKind::PREPROCESS_EXPAND_MAP);
    assert!(is_pp_expand_map_slice_kind(TaskKind::PREPROCESS_EXPAND_MAP));
    assert!(!is_pp_expand_map_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pp_expand_map_slice_kind(
        TaskKind::PREPROCESS_PRAGMA_DIRECTIVE
    ));
    assert_eq!(stage_of(TaskKind::PREPROCESS_EXPAND_MAP), Some(1));
    let registry = TaskKindRegistry::pp_expand_map_slice();
    assert_eq!(registry.len(), 47);
    assert_eq!(
        registry
            .lookup(TaskKind::PREPROCESS_EXPAND_MAP)
            .unwrap()
            .name,
        "preprocess.expand_map"
    );
    let manifest = PpExpandMapChip.manifest();
    assert_eq!(manifest.id, PP27_CHIP);
    assert_eq!(PP27_CHIP, cc_silicon_compiler::ids::ChipId(37));
    // Ack-only and read-only: no writes, hence no allowlist row.
    assert!(manifest.writes.is_empty());
    assert!(!STORE_OWNER_ALLOWLIST
        .iter()
        .any(|&(chip, _, _, _)| chip == PP27_CHIP));
    assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
    assert!(manifest.declares_read(StoreId::Pp, "tokens"));
    assert!(manifest.declares_read(StoreId::Sources, "spans"));
    assert!(manifest.declares_read(StoreId::Sources, "expansion"));
}

#[test]
fn basic_chain_rebuilds_one_frame() {
    let source = SourceId::from_index(0);
    let token_span = SpanId::from_index(0);
    let spelling = SpanId::from_index(1);
    let expanded = SpanId::from_index(2);
    let expansion = ExpansionId::from_index(0);
    let token = PpTokenId::from_index(0);
    let plain_span = SpanId::from_index(3);
    let plain = PpTokenId::from_index(1);
    let input = pure_input(
        vec![
            (
                token,
                token_record(PpTokenKind::Identifier, token_span, b"x"),
            ),
            (
                plain,
                token_record(PpTokenKind::Punctuator, plain_span, b"+"),
            ),
        ],
        vec![
            (token_span, span_record(source, 0, 1, Some(expansion))),
            (spelling, span_record(source, 10, 11, None)),
            (expanded, span_record(source, 20, 21, None)),
            (plain_span, span_record(source, 30, 31, None)),
        ],
        vec![(expansion, expansion_record(None, spelling, expanded, 0))],
        vec![source],
    );
    let chain = origin_chain(&input, token).expect("chain builds");
    assert_eq!(chain.token, token);
    assert_eq!(chain.span, token_span);
    assert_eq!(chain.frames.len(), 1);
    let frame = chain.frames[0];
    assert_eq!(frame.expansion, expansion);
    assert_eq!(frame.spelling, spelling);
    assert_eq!(frame.expanded, expanded);
    assert_eq!(frame.ordinal, 0);
    assert_eq!(frame.depth, 0);
    assert_eq!(origin_root(&chain), expanded);
    // An unexpanded token carries no frames; its origin is its own span.
    let plain_chain = origin_chain(&input, plain).expect("plain chain builds");
    assert!(plain_chain.frames.is_empty());
    assert_eq!(origin_root(&plain_chain), plain_span);
    assert!(acked(&PpExpandMapChip.compute(&input)));
}

#[test]
fn stringify_reads_raw_spelling_side() {
    // `#` operands read the raw, unexpanded argument bytes: the frame's
    // `spelling` span names the raw form while the token's own span is
    // the invocation name span.
    let source = SourceId::from_index(0);
    let token_span = SpanId::from_index(0);
    let raw = SpanId::from_index(1);
    let product = SpanId::from_index(2);
    let expansion = ExpansionId::from_index(0);
    let token = PpTokenId::from_index(0);
    let input = pure_input(
        vec![(
            token,
            token_record(PpTokenKind::StringLiteral, token_span, b"\"a\""),
        )],
        vec![
            (token_span, span_record(source, 0, 5, Some(expansion))),
            (raw, span_record(source, 40, 41, None)),
            (product, span_record(source, 50, 53, None)),
        ],
        vec![(expansion, expansion_record(None, raw, product, 1))],
        vec![source],
    );
    let chain = origin_chain(&input, token).expect("chain builds");
    assert_eq!(chain.span, token_span);
    assert_eq!(chain.frames[0].spelling, raw);
    assert_ne!(chain.frames[0].spelling, chain.frames[0].expanded);
    assert!(acked(&PpExpandMapChip.compute(&input)));
}

#[test]
fn paste_product_lands_on_expanded_side() {
    // `##` products land on the `expanded` side; both operand spans stay
    // reachable through the same frame.
    let source = SourceId::from_index(0);
    let token_span = SpanId::from_index(0);
    let operands = SpanId::from_index(1);
    let product = SpanId::from_index(2);
    let expansion = ExpansionId::from_index(0);
    let token = PpTokenId::from_index(0);
    let input = pure_input(
        vec![(
            token,
            token_record(PpTokenKind::Identifier, token_span, b"ab"),
        )],
        vec![
            (token_span, span_record(source, 0, 2, Some(expansion))),
            (operands, span_record(source, 60, 64, None)),
            (product, span_record(source, 70, 72, None)),
        ],
        vec![(expansion, expansion_record(None, operands, product, 2))],
        vec![source],
    );
    let chain = origin_chain(&input, token).expect("chain builds");
    assert_eq!(chain.frames[0].expanded, product);
    assert_eq!(chain.frames[0].spelling, operands);
    assert_eq!(origin_root(&chain), product);
    assert!(acked(&PpExpandMapChip.compute(&input)));
}

#[test]
fn prescan_and_raw_stay_side_by_side() {
    // Prescan-vs-raw: `spelling` keeps the raw form and `expanded` the
    // prescanned form instead of collapsing to one.
    let source = SourceId::from_index(0);
    let token_span = SpanId::from_index(0);
    let raw = SpanId::from_index(1);
    let prescanned = SpanId::from_index(2);
    let expansion = ExpansionId::from_index(0);
    let token = PpTokenId::from_index(0);
    let input = pure_input(
        vec![(
            token,
            token_record(PpTokenKind::Identifier, token_span, b"y"),
        )],
        vec![
            (token_span, span_record(source, 0, 1, Some(expansion))),
            (raw, span_record(source, 80, 81, None)),
            (prescanned, span_record(source, 90, 91, None)),
        ],
        vec![(expansion, expansion_record(None, raw, prescanned, 3))],
        vec![source],
    );
    let chain = origin_chain(&input, token).expect("chain builds");
    assert_eq!(chain.frames[0].spelling, raw);
    assert_eq!(chain.frames[0].expanded, prescanned);
    assert_ne!(chain.frames[0].spelling, chain.frames[0].expanded);
    assert!(acked(&PpExpandMapChip.compute(&input)));
}

#[test]
fn blue_paint_rescan_nesting_replays_parent_links() {
    // Blue-paint rescan: `parent` linkage plus per-frame
    // `ordinal`/`depth` recover the rescan nesting without re-running
    // substitution.
    let source = SourceId::from_index(0);
    let token_span = SpanId::from_index(0);
    let inner_spelling = SpanId::from_index(1);
    let inner_expanded = SpanId::from_index(2);
    let outer_spelling = SpanId::from_index(3);
    let outer_expanded = SpanId::from_index(4);
    let inner = ExpansionId::from_index(0);
    let outer = ExpansionId::from_index(1);
    let token = PpTokenId::from_index(0);
    let input = pure_input(
        vec![(
            token,
            token_record(PpTokenKind::Identifier, token_span, b"z"),
        )],
        vec![
            (token_span, span_record(source, 0, 1, Some(inner))),
            (inner_spelling, span_record(source, 100, 101, Some(outer))),
            (inner_expanded, span_record(source, 110, 111, None)),
            (outer_spelling, span_record(source, 120, 121, None)),
            (outer_expanded, span_record(source, 130, 131, None)),
        ],
        vec![
            (
                inner,
                expansion_record(Some(outer), inner_spelling, inner_expanded, 4),
            ),
            (
                outer,
                expansion_record(None, outer_spelling, outer_expanded, 5),
            ),
        ],
        vec![source],
    );
    let chain = origin_chain(&input, token).expect("chain builds");
    assert_eq!(chain.frames.len(), 2);
    assert_eq!(chain.frames[0].expansion, inner);
    assert_eq!(chain.frames[0].depth, 0);
    assert_eq!(chain.frames[0].ordinal, 4);
    assert_eq!(chain.frames[1].expansion, outer);
    assert_eq!(chain.frames[1].depth, 1);
    assert_eq!(chain.frames[1].ordinal, 5);
    assert_eq!(origin_root(&chain), outer_expanded);
    assert!(acked(&PpExpandMapChip.compute(&input)));
}

#[test]
fn nested_include_origins_name_every_source() {
    // Nested include origins: every frame resolves to a `SpanRecord`
    // whose `source` identifies the physical file at that nesting level.
    let outer_source = SourceId::from_index(0);
    let header_source = SourceId::from_index(1);
    let token_span = SpanId::from_index(0);
    let header_spelling = SpanId::from_index(1);
    let header_expanded = SpanId::from_index(2);
    let outer_spelling = SpanId::from_index(3);
    let outer_expanded = SpanId::from_index(4);
    let header_expansion = ExpansionId::from_index(0);
    let outer_expansion = ExpansionId::from_index(1);
    let token = PpTokenId::from_index(0);
    let input = pure_input(
        vec![(
            token,
            token_record(PpTokenKind::Identifier, token_span, b"w"),
        )],
        vec![
            (
                token_span,
                span_record(header_source, 0, 1, Some(header_expansion)),
            ),
            (
                header_spelling,
                span_record(header_source, 10, 11, Some(outer_expansion)),
            ),
            (header_expanded, span_record(header_source, 20, 21, None)),
            (outer_spelling, span_record(outer_source, 200, 201, None)),
            (outer_expanded, span_record(outer_source, 210, 211, None)),
        ],
        vec![
            (
                header_expansion,
                expansion_record(Some(outer_expansion), header_spelling, header_expanded, 6),
            ),
            (
                outer_expansion,
                expansion_record(None, outer_spelling, outer_expanded, 7),
            ),
        ],
        vec![outer_source, header_source],
    );
    let chain = origin_chain(&input, token).expect("chain builds");
    assert_eq!(chain.frames.len(), 2);
    assert_eq!(input.sources.len(), 2);
    assert_eq!(origin_root(&chain), outer_expanded);
    assert!(acked(&PpExpandMapChip.compute(&input)));
}

#[test]
fn dangling_expansion_fails_typed() {
    // A token span naming an expansion missing from the projection is a
    // typed `Invalid` failure, never a silent truncation.
    let source = SourceId::from_index(0);
    let token_span = SpanId::from_index(0);
    let missing = ExpansionId::from_index(9);
    let token = PpTokenId::from_index(0);
    let input = pure_input(
        vec![(
            token,
            token_record(PpTokenKind::Identifier, token_span, b"q"),
        )],
        vec![(token_span, span_record(source, 0, 1, Some(missing)))],
        vec![],
        vec![source],
    );
    let message = fail_message(&PpExpandMapChip.compute(&input));
    assert!(message.contains("dangling"), "message: {message}");
}

/// Run PP01→PP02→PP03→PP04 from real source bytes; return the scanned
/// pp-token refs without the trailing EOF token.
fn scan_expand_refs(bus: &mut CompilerBus, raw: Vec<u8>) -> Vec<RecordRef> {
    let name = bus.intern_name(b"expand.c").unwrap();
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
    // The scan appends a trailing EOF token; the expand-map payload is the
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

#[test]
fn bus_dispatch_completes_ack_with_no_writes() {
    let mut bus = new_bus();
    install_pp(&mut bus);
    // Unexpanded scan output carries no expansion links, so every origin
    // chain is the token's own span and the task completes `Ack` with no
    // bus writes (the wiring layer consumes the chains).
    let refs = scan_expand_refs(&mut bus, b"int x;\n".to_vec());
    assert!(!refs.is_empty());
    let tokens_before = bus.arenas.pp_tokens.allocated();
    let spans_before = bus.arenas.spans.allocated();
    let workers = workers();
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_EXPAND_MAP,
        PP27_CHIP,
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
                "expand-map dispatch must complete Ack"
            );
        }
        other => panic!("expected executed, got {other:?}"),
    }
    assert_eq!(bus.arenas.pp_tokens.allocated(), tokens_before);
    assert_eq!(bus.arenas.spans.allocated(), spans_before);
}

#[test]
fn expand_map_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::pp_expand_map_slice();
    mismatched.schema = StoreSchema::pp_macro_slice();
    mismatched
        .registrations
        .register(
            PpExpandMapChip.manifest(),
            &mismatched.schema,
            &mismatched.kinds,
        )
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::PREPROCESS_EXPAND_MAP, PP27_CHIP, 9)
        .unwrap();
    let workers = workers();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::PREPROCESS_EXPAND_MAP,
        PP27_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // The pre-`/30` registry does not know the expand-map kind, so the
    // manifest is rejected there.
    let mut stale = ManifestRegistry::new();
    assert!(stale
        .register(
            PpExpandMapChip.manifest(),
            &StoreSchema::pp_macro_slice(),
            &TaskKindRegistry::pp_pragma_slice(),
        )
        .is_err());
}
