// ============================================================================
// c36_recovery.rs — Wave 3 (`/36`) PA recovery-slice acceptance.
//
// Covers the frozen closure: the `parse.decl_finish` (local 25) and
// `parse.recovery` (local 26) kinds, kind-to-stage assignment
// (stage 2), the PA14/PA38 manifests (both Ack-only with zero writes
// and no allowlist rows), the M1 pod finish (`main(void);` with its
// point-of-declaration registration certified), the missing-`;`
// defect (`Task` channel), the comma/initializer deferrals
// (`Unsupported`), the recovery delimiter sync (`;` consumed,
// `)`/`}`/`{`-stop/EOF not consumed), nesting suppression, the
// finite-advance guarantee, bus dispatch (all `Ack`, replay
// determinism), and stage/layer gates.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord, SpanRecord, TokenKind, TokenRecord,
};
use cc_silicon_compiler::chips::{
    handler_for, pa14_task_kind, pa38_task_kind, parse_declaration_finish, recover_cursor,
    PaPodChip, PaRecoveryChip, PodError, PodToken, ProjectedPodToken, ProjectedRecoveryToken,
    Worker, WorkerRegistry, PA14_TASK_KIND, PA38_TASK_KIND,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{ChipId, NameId, RecordRef, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    check_stage_layer_agreement, is_pa_recovery_slice_kind, stage_of, PA14_CHIP, PA38_CHIP,
    STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::Snapshot;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, ResultValue, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry, TaskState,
};
use cc_silicon_compiler::chips::{PaPodInput, PaRecoveryInput};
use std::collections::BTreeMap;

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_pa_recovery(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::StoreSchema;
    bus.kinds = TaskKindRegistry::pa_recovery_slice();
    bus.schema = StoreSchema::pa_slice();
    for chip in [&PaPodChip as &dyn Worker, &PaRecoveryChip] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(TaskKind::PARSE_DECL_FINISH, PA14_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_RECOVERY, PA38_CHIP, 2)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PaPodChip).unwrap();
    workers.register(PaRecoveryChip).unwrap();
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

fn is_ack(proposals: &[Proposal]) -> bool {
    matches!(
        proposals,
        [Proposal::Complete {
            value: ResultValue::Ack,
            ..
        }]
    )
}

fn fail_group(proposals: &[Proposal]) -> Option<DiagGroup> {
    match proposals {
        [Proposal::Fail { diagnostic, .. }] => Some(diagnostic.code.group),
        _ => None,
    }
}

// --- pure-input constructors -----------------------------------------------

fn pod_ident(spelling: &[u8]) -> PodToken {
    PodToken {
        kind: TokenKind::Identifier,
        spelling: spelling.to_vec(),
        name: Some(NameId::from_index(7)),
    }
}

fn pod_keyword(spelling: &[u8]) -> PodToken {
    PodToken {
        kind: TokenKind::Keyword,
        spelling: spelling.to_vec(),
        name: None,
    }
}

fn pod_punct(spelling: &[u8]) -> PodToken {
    PodToken {
        kind: TokenKind::Punctuator,
        spelling: spelling.to_vec(),
        name: None,
    }
}

fn pod_views_main() -> Vec<PodToken> {
    vec![
        pod_ident(b"main"),
        pod_punct(b"("),
        pod_keyword(b"void"),
        pod_punct(b")"),
        pod_punct(b";"),
    ]
}

fn pod_input_of(views: &[PodToken]) -> PaPodInput {
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for (index, view) in views.iter().enumerate() {
        let id = TokenId::from_index(index as u32);
        tokens.push(id);
        let (spelling, pp_spelling) = match view.kind {
            TokenKind::Identifier | TokenKind::Keyword => (view.spelling.clone(), Vec::new()),
            _ => (Vec::new(), view.spelling.clone()),
        };
        bodies.insert(
            id,
            ProjectedPodToken {
                kind: view.kind,
                name: view.name,
                spelling,
                pp_spelling,
            },
        );
    }
    PaPodInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        bodies,
    }
}

fn recovery_token(id: u32, kind: TokenKind, pp: &[u8]) -> (TokenId, ProjectedRecoveryToken) {
    let tid = TokenId::from_index(id);
    (
        tid,
        ProjectedRecoveryToken {
            kind,
            name: None,
            spelling: Vec::new(),
            pp_spelling: pp.to_vec(),
        },
    )
}

fn recovery_input(specs: &[(TokenKind, &[u8])]) -> PaRecoveryInput {
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for (position, (kind, pp)) in specs.iter().enumerate() {
        let (id, projected) = recovery_token(position as u32, *kind, pp);
        tokens.push(id);
        bodies.insert(id, projected);
    }
    PaRecoveryInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        bodies,
    }
}

const P: TokenKind = TokenKind::Punctuator;
const K: TokenKind = TokenKind::Keyword;

// --- 1. freeze ---------------------------------------------------------------

#[test]
fn recovery_kinds_stage_registry_manifest_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(TaskKind::PARSE_DECL_FINISH.group(), TaskGroup::PARSE);
    assert_eq!(TaskKind::PARSE_DECL_FINISH.local(), 25);
    assert_eq!(TaskKind::PARSE_RECOVERY.group(), TaskGroup::PARSE);
    assert_eq!(TaskKind::PARSE_RECOVERY.local(), 26);
    assert_eq!(PA14_TASK_KIND, TaskKind::PARSE_DECL_FINISH);
    assert_eq!(PA38_TASK_KIND, TaskKind::PARSE_RECOVERY);
    assert_eq!(pa14_task_kind(), TaskKind::PARSE_DECL_FINISH);
    assert_eq!(pa38_task_kind(), TaskKind::PARSE_RECOVERY);
    for kind in [TaskKind::PARSE_DECL_FINISH, TaskKind::PARSE_RECOVERY] {
        assert!(is_pa_recovery_slice_kind(kind));
        assert_eq!(stage_of(kind), Some(2));
    }
    assert!(!is_pa_recovery_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pa_recovery_slice_kind(TaskKind::PARSE_TU));
    assert!(!is_pa_recovery_slice_kind(TaskKind::PARSE_UNARY));
    // The recovery registry extends the `/35` head linearly; `PARSE`
    // owners start new codes at local 27.
    assert_eq!(TaskKindRegistry::pa_expr_slice().len(), 61);
    let registry = TaskKindRegistry::pa_recovery_slice();
    assert_eq!(registry.len(), 63);
    for (kind, name) in [
        (TaskKind::PARSE_DECL_FINISH, "parse.decl_finish"),
        (TaskKind::PARSE_RECOVERY, "parse.recovery"),
    ] {
        assert_eq!(registry.lookup(kind).unwrap().name, name);
    }
    assert_eq!(stage_of(TaskKind::new(TaskGroup::PARSE, 27).unwrap()), None);
    // Both chips are Ack-only: zero writes, no allowlist rows
    // (`/39` adds the SE29 `sem.records` row elsewhere).
    assert_eq!(PA14_CHIP, ChipId(50));
    assert_eq!(PA38_CHIP, ChipId(51));
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 33);
    for chip in [PA14_CHIP, PA38_CHIP] {
        assert!(
            !STORE_OWNER_ALLOWLIST
                .iter()
                .any(|&(owner, _, _, _)| owner == chip),
            "Ack-only chip must hold no allowlist row"
        );
    }
    let manifests = [PaPodChip.manifest(), PaRecoveryChip.manifest()];
    for manifest in &manifests {
        assert!(manifest.writes.is_empty());
        assert!(manifest.deterministic);
        assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
        assert!(manifest.declares_read(StoreId::Lex, "tokens"));
        assert_eq!(manifest.tests, vec!["compiler/tests/c36_recovery.rs"]);
        let routing = {
            let mut bus = new_bus();
            install_pa_recovery(&mut bus);
            bus.routing
        };
        assert!(check_stage_layer_agreement(manifest, &routing).is_ok());
    }
    assert_eq!(PaPodChip.manifest().task_kinds, vec![PA14_TASK_KIND]);
    assert_eq!(
        PaRecoveryChip.manifest().task_kinds,
        vec![PA38_TASK_KIND]
    );
}

// --- 2. pod accept -----------------------------------------------------------

#[test]
fn pod_accepts_main_void_with_registration() {
    let finish = parse_declaration_finish(&pod_views_main()).expect("M1 `main(void);` finishes");
    let registration = finish.registration();
    assert_eq!(registration.name_spelling(), b"main");
    assert_eq!(registration.pod_name(), Some(NameId::from_index(7)));
    assert_eq!((registration.name_pos, registration.first_pos), (0, 0));
    assert_eq!(registration.last_pos, 3);
    assert_eq!(registration.declarator_len(), 4);
    assert!(is_ack(&PaPodChip.compute(&pod_input_of(&pod_views_main()))));
}

// --- 3. pod defects ----------------------------------------------------------

#[test]
fn pod_missing_semicolon_fails_task_channel() {
    let views = vec![
        pod_ident(b"main"),
        pod_punct(b"("),
        pod_keyword(b"void"),
        pod_punct(b")"),
        pod_punct(b"}"),
    ];
    assert_eq!(
        parse_declaration_finish(&views),
        Err(PodError::Unterminated)
    );
    assert_eq!(
        fail_group(&PaPodChip.compute(&pod_input_of(&views))),
        Some(DiagGroup::Task)
    );
}

#[test]
fn pod_comma_and_initializer_are_unsupported() {
    let comma = vec![
        pod_ident(b"main"),
        pod_punct(b"("),
        pod_keyword(b"void"),
        pod_punct(b")"),
        pod_punct(b","),
    ];
    assert!(matches!(
        parse_declaration_finish(&comma),
        Err(PodError::Unsupported(_))
    ));
    assert_eq!(
        fail_group(&PaPodChip.compute(&pod_input_of(&comma))),
        Some(DiagGroup::Unsupported)
    );
    // An interior `=` keeps the slice in PA15 territory: the initializer
    // RHS must observe the POD registration first.
    let init = vec![
        pod_ident(b"main"),
        pod_punct(b"="),
        pod_ident(b"main"),
        pod_punct(b"("),
        pod_punct(b")"),
        pod_punct(b";"),
    ];
    assert!(matches!(
        parse_declaration_finish(&init),
        Err(PodError::Unsupported(_))
    ));
    assert_eq!(
        fail_group(&PaPodChip.compute(&pod_input_of(&init))),
        Some(DiagGroup::Unsupported)
    );
}

// --- 4. recovery sync ----------------------------------------------------------

#[test]
fn recovery_semicolon_sync_consumes() {
    use cc_silicon_compiler::chips::SyncKind;
    let got = recover_cursor(&recovery_input(&[(K, b""), (P, b"+"), (P, b";"), (K, b"")])).unwrap();
    assert_eq!(got.index, 3);
    assert_eq!(got.sync, SyncKind::Semicolon);
    assert!(got.consumed_sync);
}

#[test]
fn recovery_closers_and_eof_stop_without_consuming() {
    use cc_silicon_compiler::chips::SyncKind;
    // `)` with no open paren: the owning frame keeps its closer.
    let paren = recover_cursor(&recovery_input(&[(K, b""), (P, b")"), (P, b";")])).unwrap();
    assert_eq!(paren.index, 1);
    assert_eq!(paren.sync, SyncKind::CloseParen);
    assert!(!paren.consumed_sync);
    // `}` with no open brace: the owning frame keeps its brace.
    let brace = recover_cursor(&recovery_input(&[(K, b""), (P, b"}"), (K, b"")])).unwrap();
    assert_eq!(brace.index, 1);
    assert_eq!(brace.sync, SyncKind::CloseBrace);
    assert!(!brace.consumed_sync);
    // `{` at zero depth: refuse to enter the next body (anti-swallow).
    let open = recover_cursor(&recovery_input(&[(K, b""), (P, b"{"), (K, b"")])).unwrap();
    assert_eq!(open.index, 1);
    assert_eq!(open.sync, SyncKind::OpenBrace);
    assert!(!open.consumed_sync);
    // Committed EOF terminates the scan at end of input.
    let eof = recover_cursor(&recovery_input(&[(P, b"+"), (TokenKind::Eof, b"")])).unwrap();
    assert_eq!(eof.index, 1);
    assert_eq!(eof.sync, SyncKind::Eof);
    assert!(!eof.consumed_sync);
    assert!(is_ack(
        &PaRecoveryChip.compute(&recovery_input(&[(K, b""), (P, b";")]))
    ));
}

// --- 5. nesting + finite advance -------------------------------------------------

#[test]
fn recovery_nesting_suppresses_inner_delimiters() {
    use cc_silicon_compiler::chips::SyncKind;
    // `f ( 1 ; 2 ) ;` → the first `;` is inside parens; sync at final `;`.
    let specs = [
        (TokenKind::Identifier, b"".as_slice()),
        (P, b"(".as_slice()),
        (TokenKind::Integer, b"1".as_slice()),
        (P, b";".as_slice()),
        (TokenKind::Integer, b"2".as_slice()),
        (P, b")".as_slice()),
        (P, b";".as_slice()),
    ];
    let got = recover_cursor(&recovery_input(&specs)).unwrap();
    assert_eq!(got.index, 7);
    assert_eq!(got.sync, SyncKind::Semicolon);
    // A stray `]` is not a sync; the later `;` syncs.
    let bracket =
        recover_cursor(&recovery_input(&[(K, b""), (P, b"]"), (P, b";")])).unwrap();
    assert_eq!(bracket.index, 3);
    // A kind-only punctuator with no spelling never syncs.
    let err = recover_cursor(&recovery_input(&[(K, b""), (P, b""), (K, b"")])).unwrap_err();
    assert_eq!(err.code.group, DiagGroup::Unsupported);
    // A window with no sync token fails instead of fabricating success.
    let bare = recover_cursor(&recovery_input(&[(K, b""), (P, b"+"), (K, b"")])).unwrap_err();
    assert_eq!(bare.code.group, DiagGroup::Unsupported);
}

#[test]
fn recovery_finite_advance_never_spins() {
    // A fault sitting exactly on a closer still advances one token.
    let got = recover_cursor(&recovery_input(&[(P, b"}"), (P, b";")])).unwrap();
    assert_eq!(got.index, 1);
    assert!(got.consumed_sync);
    // A fault already at EOF fails instead of retrying forever.
    let err = recover_cursor(&recovery_input(&[(TokenKind::Eof, b")")])).unwrap_err();
    void_eof_spin_guard(&err);
    // An empty window has no fault site to advance from.
    let empty = PaRecoveryInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens: Vec::new(),
        bodies: BTreeMap::new(),
    };
    let err = recover_cursor(&empty).unwrap_err();
    assert_eq!(err.code.group, DiagGroup::Task);
}

fn void_eof_spin_guard(err: &cc_silicon_compiler::diagnostic::DiagnosticDraft) {
    assert_eq!(err.code.group, DiagGroup::Unsupported);
}

// --- 6. determinism ------------------------------------------------------------

#[test]
fn pure_cores_are_deterministic() {
    let pod = pod_input_of(&pod_views_main());
    assert_eq!(
        PaPodChip.compute(&pod),
        PaPodChip.compute(&pod_input_of(&pod_views_main()))
    );
    assert_eq!(
        parse_declaration_finish(&pod_views_main()),
        parse_declaration_finish(&pod_views_main())
    );
    let window = recovery_input(&[(K, b""), (P, b"+"), (P, b";")]);
    assert_eq!(
        PaRecoveryChip.compute(&window),
        PaRecoveryChip.compute(&recovery_input(&[(K, b""), (P, b"+"), (P, b";")]))
    );
    assert_eq!(
        recover_cursor(&window),
        recover_cursor(&recovery_input(&[(K, b""), (P, b"+"), (P, b";")]))
    );
}

// --- 7. bus dispatch -----------------------------------------------------------

/// Seed one committed C token; returns its `TokenId`.
fn seed_token(
    bus: &mut CompilerBus,
    source: cc_silicon_compiler::ids::SourceId,
    kind: TokenKind,
    spelling: &[u8],
    pp_kind: PpTokenKind,
    name: Option<NameId>,
) -> TokenId {
    let limits = bus.limits();
    let span = bus
        .arenas
        .spans
        .alloc(
            SpanRecord {
                source,
                start: 0,
                end: spelling.len() as u64,
                expansion: None,
            },
            &limits,
        )
        .unwrap();
    let pp_token = bus
        .arenas
        .pp_tokens
        .alloc(
            PpTokenRecord {
                kind: pp_kind,
                span,
                spelling: spelling.to_vec(),
            },
            &limits,
        )
        .unwrap();
    bus.arenas
        .tokens
        .alloc(
            TokenRecord {
                kind,
                span,
                name,
                pp_token,
            },
            &limits,
        )
        .unwrap()
}

/// Seed the committed M1 pod finish `main(void);` plus the recovery
/// window `bad + ;` in source order; returns `(pod, window)` token IDs.
fn seed_m1_recovery(bus: &mut CompilerBus) -> (Vec<TokenId>, Vec<TokenId>) {
    let file = bus.intern_name(b"m1.c").unwrap();
    let source = bus.alloc_source(file, b"main(void);bad+;".to_vec()).unwrap();
    let main = bus.intern_name(b"main").unwrap();
    let void = bus.intern_name(b"void").unwrap();
    let bad = bus.intern_name(b"bad").unwrap();
    let (i, k, p) = (TokenKind::Identifier, TokenKind::Keyword, TokenKind::Punctuator);
    let (id, pu) = (PpTokenKind::Identifier, PpTokenKind::Punctuator);
    let pod = vec![
        seed_token(bus, source, i, b"main", id, Some(main)),
        seed_token(bus, source, p, b"(", pu, None),
        seed_token(bus, source, k, b"void", id, Some(void)),
        seed_token(bus, source, p, b")", pu, None),
        seed_token(bus, source, p, b";", pu, None),
    ];
    let window = vec![
        seed_token(bus, source, i, b"bad", id, Some(bad)),
        seed_token(bus, source, p, b"+", pu, None),
        seed_token(bus, source, p, b";", pu, None),
    ];
    (pod, window)
}

fn refs(ids: &[TokenId]) -> Payload {
    Payload::from_refs(ids.iter().map(|id| RecordRef::Token(*id)).collect())
}

fn run_scenario() -> (Vec<u8>, usize, usize) {
    let mut bus = new_bus();
    install_pa_recovery(&mut bus);
    let workers = workers();
    let (pod, window) = seed_m1_recovery(&mut bus);
    bootstrap(&mut bus, TaskKind::PARSE_DECL_FINISH, PA14_CHIP, refs(&pod));
    bootstrap(&mut bus, TaskKind::PARSE_RECOVERY, PA38_CHIP, refs(&window));
    let mut completed = 0;
    let mut failed = 0;
    for _ in 0..8 {
        let report = tick(&mut bus, &workers);
        match &report.outcome {
            TickOutcome::Executed { commit, .. } => {
                for (_, result) in &commit.completed {
                    match &bus.arenas.results.get(*result).unwrap().value {
                        ResultValue::Ack => {}
                        other => panic!("expected Ack, got {other:?}"),
                    }
                }
                completed += commit.completed.len();
                failed += commit.failed.len();
            }
            other => panic!("expected executed, got {other:?}"),
        }
        if completed + failed == 2 {
            break;
        }
    }
    (Snapshot::capture(&bus).bytes().to_vec(), completed, failed)
}

#[test]
fn bus_dispatch_commits_ack_and_replays_deterministically() {
    let (first, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (2, 0));
    let (second, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (2, 0));
    assert_eq!(first, second);
}

// --- 8. gates ------------------------------------------------------------------

#[test]
fn recovery_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    // A wrong layer is refused on the driver path for both kinds.
    let cases: [(TaskKind, ChipId, &dyn Worker); 2] = [
        (TaskKind::PARSE_DECL_FINISH, PA14_CHIP, &PaPodChip),
        (TaskKind::PARSE_RECOVERY, PA38_CHIP, &PaRecoveryChip),
    ];
    for (kind, chip, worker) in cases {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::pa_recovery_slice();
        mismatched.schema = StoreSchema::pa_slice();
        mismatched
            .registrations
            .register(worker.manifest(), &mismatched.schema, &mismatched.kinds)
            .unwrap();
        mismatched.routing.register(kind, chip, 9).unwrap();
        let task = bootstrap(&mut mismatched, kind, chip, Payload::empty());
        let error =
            cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers()).unwrap_err();
        assert!(matches!(
            error,
            cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
        ));
    }
    // The pre-`/36` registry knows neither kind, so both manifests are
    // rejected there (frozen kinds are never re-registered).
    let mut stale = ManifestRegistry::new();
    let kinds = TaskKindRegistry::pa_expr_slice();
    let schema = StoreSchema::pa_slice();
    for worker in [&PaPodChip as &dyn Worker, &PaRecoveryChip] {
        assert!(stale.register(worker.manifest(), &schema, &kinds).is_err());
    }
}
