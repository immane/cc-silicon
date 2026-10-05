// ============================================================================
// c35_expr.rs — Wave 3 (`/35`) PA expr-slice acceptance.
//
// Covers the frozen closure: the `parse.primary` (local 22),
// `parse.binary` (local 23), and `parse.unary` (local 24) kinds,
// kind-to-stage assignment (stage 2), the PA16/PA20 manifests
// (both Ack-only with zero writes and no allowlist rows), the M1 primary
// (`<int>` backed by one committed literal), the M1 binary
// (`<int> + <int>` as one precedence-climb step at `min_bp = 0`), the M1
// unary (`+<int>` / `-<int>`), the 4-token boundary (`2 + +3` is not one
// PA22 shape; its `+3` suffix is the PA20 shape), the non-M1 matrix
// (`*`, `-`, juxtaposition, identifiers — all explicit `Unsupported`),
// bus dispatch (all `Ack`, replay determinism), and stage/layer gates.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, LiteralRecord, PpTokenKind, PpTokenRecord, SpanRecord, TokenKind,
    TokenRecord,
};
use cc_silicon_compiler::chips::{
    binary_precedence, handler_for, parse_binary_expression, parse_primary, parse_unary,
    PaBinaryChip, PaUnaryChip, ProjectedExprToken, ProjectedUnaryToken, Worker, WorkerRegistry,
    PA16_TASK_KIND, PA20_TASK_KIND, PA22_TASK_KIND,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{ChipId, LiteralId, RecordRef, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    check_stage_layer_agreement, is_pa_expr_slice_kind, stage_of, PA16_CHIP, PA20_CHIP,
    STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType, Snapshot};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, ResultValue, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry, TaskState,
};
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

fn install_pa_expr(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::StoreSchema;
    bus.kinds = TaskKindRegistry::pa_expr_slice();
    bus.schema = StoreSchema::pa_slice();
    for chip in [&PaBinaryChip as &dyn Worker, &PaUnaryChip] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(TaskKind::PARSE_PRIMARY, PA16_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_BINARY, PA16_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_UNARY, PA20_CHIP, 2)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PaBinaryChip).unwrap();
    workers.register(PaUnaryChip).unwrap();
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

fn expr_token(kind: TokenKind, pp: &[u8]) -> ProjectedExprToken {
    ProjectedExprToken {
        kind,
        name: None,
        spelling: Vec::new(),
        pp_spelling: pp.to_vec(),
    }
}

fn unary_token(kind: TokenKind, pp: &[u8]) -> ProjectedUnaryToken {
    ProjectedUnaryToken {
        kind,
        name: None,
        spelling: Vec::new(),
        pp_spelling: pp.to_vec(),
    }
}

fn primary_input(
    slots: &[(TokenKind, &[u8])],
    lit_at: &[usize],
) -> cc_silicon_compiler::chips::PaPrimaryInput {
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for (index, (kind, pp)) in slots.iter().enumerate() {
        let id = TokenId::from_index(index as u32);
        tokens.push(id);
        bodies.insert(id, expr_token(*kind, pp));
    }
    let mut literals_by_token = BTreeMap::new();
    for &position in lit_at {
        literals_by_token.insert(tokens[position], LiteralId::from_index(position as u32));
    }
    cc_silicon_compiler::chips::PaPrimaryInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        bodies,
        literals_by_token,
    }
}

fn binary_input(
    slots: &[(TokenKind, &[u8])],
    lit_at: &[usize],
) -> cc_silicon_compiler::chips::PaBinaryInput {
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for (index, (kind, pp)) in slots.iter().enumerate() {
        let id = TokenId::from_index(index as u32);
        tokens.push(id);
        bodies.insert(id, expr_token(*kind, pp));
    }
    let mut literals_by_token = BTreeMap::new();
    for &position in lit_at {
        literals_by_token.insert(tokens[position], LiteralId::from_index(position as u32));
    }
    cc_silicon_compiler::chips::PaBinaryInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        bodies,
        literals_by_token,
    }
}

fn unary_input(
    slots: &[(TokenKind, &[u8])],
    lit_at: &[usize],
) -> cc_silicon_compiler::chips::PaUnaryInput {
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for (index, (kind, pp)) in slots.iter().enumerate() {
        let id = TokenId::from_index(index as u32);
        tokens.push(id);
        bodies.insert(id, unary_token(*kind, pp));
    }
    let mut literals_by_token = BTreeMap::new();
    for &position in lit_at {
        literals_by_token.insert(tokens[position], LiteralId::from_index(position as u32));
    }
    cc_silicon_compiler::chips::PaUnaryInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        bodies,
        literals_by_token,
    }
}

const N: TokenKind = TokenKind::Integer;
const P: TokenKind = TokenKind::Punctuator;

// --- 1. freeze ---------------------------------------------------------------

#[test]
fn expr_kinds_stage_registry_manifest_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(TaskKind::PARSE_PRIMARY.group(), TaskGroup::PARSE);
    assert_eq!(TaskKind::PARSE_PRIMARY.local(), 22);
    assert_eq!(TaskKind::PARSE_BINARY.local(), 23);
    assert_eq!(TaskKind::PARSE_UNARY.local(), 24);
    assert_eq!(PA16_TASK_KIND, TaskKind::PARSE_PRIMARY);
    assert_eq!(PA22_TASK_KIND, TaskKind::PARSE_BINARY);
    assert_eq!(PA20_TASK_KIND, TaskKind::PARSE_UNARY);
    for kind in [
        TaskKind::PARSE_PRIMARY,
        TaskKind::PARSE_BINARY,
        TaskKind::PARSE_UNARY,
    ] {
        assert!(is_pa_expr_slice_kind(kind));
        assert_eq!(stage_of(kind), Some(2));
    }
    assert!(!is_pa_expr_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pa_expr_slice_kind(TaskKind::PARSE_TU));
    assert!(!is_pa_expr_slice_kind(TaskKind::PARSE_RETURN));
    // The expr registry extends the `/34` head linearly; `PARSE` owners
    // start new codes at local 27 after the `/36` recovery slice.
    assert_eq!(TaskKindRegistry::pa_decl_slice().len(), 58);
    let registry = TaskKindRegistry::pa_expr_slice();
    assert_eq!(registry.len(), 61);
    for (kind, name) in [
        (TaskKind::PARSE_PRIMARY, "parse.primary"),
        (TaskKind::PARSE_BINARY, "parse.binary"),
        (TaskKind::PARSE_UNARY, "parse.unary"),
    ] {
        assert_eq!(registry.lookup(kind).unwrap().name, name);
    }
    assert_eq!(stage_of(TaskKind::new(TaskGroup::PARSE, 27).unwrap()), None);
    // Both chips are Ack-only: zero writes, no allowlist rows.
    assert_eq!(PA16_CHIP, cc_silicon_compiler::ids::ChipId(48));
    assert_eq!(PA20_CHIP, cc_silicon_compiler::ids::ChipId(49));
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 32);
    for chip in [PA16_CHIP, PA20_CHIP] {
        assert!(
            !STORE_OWNER_ALLOWLIST
                .iter()
                .any(|&(owner, _, _, _)| owner == chip),
            "Ack-only chip must hold no allowlist row"
        );
    }
    let manifests = [PaBinaryChip.manifest(), PaUnaryChip.manifest()];
    for manifest in &manifests {
        assert!(manifest.writes.is_empty());
        assert!(manifest.deterministic);
        assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
        assert!(manifest.declares_read(StoreId::Lex, "tokens"));
        assert_eq!(manifest.tests, vec!["compiler/tests/c35_expr.rs"]);
        let routing = {
            let mut bus = new_bus();
            install_pa_expr(&mut bus);
            bus.routing
        };
        assert!(check_stage_layer_agreement(manifest, &routing).is_ok());
    }
    assert_eq!(
        PaBinaryChip.manifest().task_kinds,
        vec![PA16_TASK_KIND, PA22_TASK_KIND]
    );
    assert_eq!(PaUnaryChip.manifest().task_kinds, vec![PA20_TASK_KIND]);
}

// --- 2. primary --------------------------------------------------------------

#[test]
fn primary_int_shape() {
    let input = primary_input(&[(N, b"2")], &[0]);
    let shapes = parse_primary(&input).expect("M1 primary validates");
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].kind_name, "IntLiteral");
    assert_eq!(shapes[0].parent, None);
    assert_eq!((shapes[0].first_token, shapes[0].last_token), (0, 0));
    assert!(shapes[0].has_literal);
    assert!(is_ack(&PaBinaryChip.compute_primary(&input)));
    // An identifier primary is explicit Unsupported, never a pass.
    let ident = primary_input(&[(TokenKind::Identifier, b"x")], &[]);
    assert_eq!(
        fail_group(&PaBinaryChip.compute_primary(&ident)),
        Some(DiagGroup::Unsupported)
    );
    // Arity violations fail loud.
    let empty = primary_input(&[], &[]);
    assert_eq!(
        fail_group(&PaBinaryChip.compute_primary(&empty)),
        Some(DiagGroup::Unsupported)
    );
    // A missing literal is a Task-channel protocol fault, not a zero.
    let no_lit = primary_input(&[(N, b"2")], &[]);
    assert_eq!(
        fail_group(&PaBinaryChip.compute_primary(&no_lit)),
        Some(DiagGroup::Task)
    );
}

// --- 3. binary ---------------------------------------------------------------

#[test]
fn binary_two_plus_three() {
    assert_eq!(binary_precedence(b"+"), Some((10, 11)));
    assert_eq!(binary_precedence(b"-"), None);
    assert_eq!(binary_precedence(b"*"), None);
    let input = binary_input(&[(N, b"2"), (P, b"+"), (N, b"3")], &[0, 2]);
    let shapes = parse_binary_expression(&input).expect("M1 `2+3` validates");
    assert_eq!(shapes.len(), 3);
    assert_eq!(shapes[0].kind_name, "BinaryAdd");
    assert_eq!(shapes[0].parent, None);
    assert_eq!((shapes[0].first_token, shapes[0].last_token), (0, 2));
    assert!(!shapes[0].has_literal);
    assert_eq!(shapes[1].kind_name, "IntLiteral");
    assert_eq!(shapes[1].parent, Some(0));
    assert_eq!((shapes[1].first_token, shapes[1].last_token), (0, 0));
    assert!(shapes[1].has_literal);
    assert_eq!(shapes[2].kind_name, "IntLiteral");
    assert_eq!(shapes[2].parent, Some(0));
    assert_eq!((shapes[2].first_token, shapes[2].last_token), (2, 2));
    assert!(shapes[2].has_literal);
    assert!(is_ack(&PaBinaryChip.compute_binary(&input)));
}

// --- 4. unary ----------------------------------------------------------------

#[test]
fn unary_plus_minus_three() {
    let plus = unary_input(&[(P, b"+"), (N, b"3")], &[1]);
    let shapes = parse_unary(&plus).expect("M1 `+3` validates");
    assert_eq!(shapes.len(), 2);
    assert_eq!(shapes[0].kind_name, "UnaryPlus");
    assert_eq!(shapes[0].parent, None);
    assert_eq!((shapes[0].first_token, shapes[0].last_token), (0, 1));
    assert!(!shapes[0].has_literal);
    assert_eq!(shapes[1].kind_name, "IntLiteral");
    assert_eq!(shapes[1].parent, Some(0));
    assert!(shapes[1].has_literal);
    assert!(is_ack(&PaUnaryChip.compute(&plus)));
    let minus = unary_input(&[(P, b"-"), (N, b"3")], &[1]);
    let shapes = parse_unary(&minus).expect("M1 `-3` validates");
    assert_eq!(shapes[0].kind_name, "UnaryMinus");
    assert!(is_ack(&PaUnaryChip.compute(&minus)));
}

// --- 5. M1 boundary ----------------------------------------------------------

#[test]
fn four_token_boundary() {
    // `2 + +3` is NOT one PA22 shape (4 tokens: explicit Unsupported);
    // its `+3` suffix is the PA20 shape owned by `pa_unary.rs`.
    let four = binary_input(&[(N, b"2"), (P, b"+"), (P, b"+"), (N, b"3")], &[0, 3]);
    assert_eq!(
        fail_group(&PaBinaryChip.compute_binary(&four)),
        Some(DiagGroup::Unsupported)
    );
    let suffix = unary_input(&[(P, b"+"), (N, b"3")], &[1]);
    assert!(is_ack(&PaUnaryChip.compute(&suffix)));
}

// --- 6. non-M1 matrix ----------------------------------------------------------

#[test]
fn star_and_non_m1_unsupported() {
    // `*` is not the M1 operator: explicit Unsupported, never a pass.
    let star = binary_input(&[(N, b"2"), (P, b"*"), (N, b"3")], &[0, 2]);
    assert_eq!(
        fail_group(&PaBinaryChip.compute_binary(&star)),
        Some(DiagGroup::Unsupported)
    );
    // `-` is not the M1 binary operator either.
    let minus = binary_input(&[(N, b"2"), (P, b"-"), (N, b"3")], &[0, 2]);
    assert_eq!(
        fail_group(&PaBinaryChip.compute_binary(&minus)),
        Some(DiagGroup::Unsupported)
    );
    // Juxtaposition (`2 3`) is not a binary expression.
    let juxtaposed = binary_input(&[(N, b"2"), (N, b"3")], &[0, 1]);
    assert_eq!(
        fail_group(&PaBinaryChip.compute_binary(&juxtaposed)),
        Some(DiagGroup::Unsupported)
    );
    // Dereference is not unary plus.
    let deref = unary_input(&[(P, b"*"), (N, b"3")], &[1]);
    assert_eq!(
        fail_group(&PaUnaryChip.compute(&deref)),
        Some(DiagGroup::Unsupported)
    );
    // `!` is not an M1 unary operator.
    let bang = unary_input(&[(P, b"!"), (N, b"3")], &[1]);
    assert_eq!(
        fail_group(&PaUnaryChip.compute(&bang)),
        Some(DiagGroup::Unsupported)
    );
    // A non-integer unary operand is explicit Unsupported.
    let operand = unary_input(&[(P, b"+"), (TokenKind::Identifier, b"x")], &[]);
    assert_eq!(
        fail_group(&PaUnaryChip.compute(&operand)),
        Some(DiagGroup::Unsupported)
    );
}

// --- 7. protocol faults + determinism ------------------------------------------

#[test]
fn compute_rejects_non_running_and_missing_records() {
    let mut primary = primary_input(&[(N, b"2")], &[0]);
    primary.state = TaskState::Ready;
    assert_eq!(
        fail_group(&PaBinaryChip.compute_primary(&primary)),
        Some(DiagGroup::Task)
    );
    let mut binary = binary_input(&[(N, b"2"), (P, b"+"), (N, b"3")], &[0, 2]);
    binary.state = TaskState::Ready;
    assert_eq!(
        fail_group(&PaBinaryChip.compute_binary(&binary)),
        Some(DiagGroup::Task)
    );
    let mut unary = unary_input(&[(P, b"+"), (N, b"3")], &[1]);
    unary.state = TaskState::Ready;
    assert_eq!(
        fail_group(&PaUnaryChip.compute(&unary)),
        Some(DiagGroup::Task)
    );
    // A missing token body is a Task-channel fault, never a guess.
    let mut hollow = binary_input(&[(N, b"2"), (P, b"+"), (N, b"3")], &[0, 2]);
    hollow.bodies.remove(&hollow.tokens[1]);
    assert_eq!(
        fail_group(&PaBinaryChip.compute_binary(&hollow)),
        Some(DiagGroup::Task)
    );
}

#[test]
fn pure_cores_are_deterministic() {
    let primary = primary_input(&[(N, b"2")], &[0]);
    assert_eq!(
        PaBinaryChip.compute_primary(&primary),
        PaBinaryChip.compute_primary(&primary_input(&[(N, b"2")], &[0]))
    );
    let binary = binary_input(&[(N, b"2"), (P, b"+"), (N, b"3")], &[0, 2]);
    assert_eq!(
        parse_binary_expression(&binary),
        parse_binary_expression(&binary_input(&[(N, b"2"), (P, b"+"), (N, b"3")], &[0, 2]))
    );
    let unary = unary_input(&[(P, b"-"), (N, b"3")], &[1]);
    assert_eq!(
        PaUnaryChip.compute(&unary),
        PaUnaryChip.compute(&unary_input(&[(P, b"-"), (N, b"3")], &[1]))
    );
    assert_eq!(binary_precedence(b"+"), binary_precedence(b"+"));
}

// --- 8. bus dispatch -----------------------------------------------------------

/// Seed one committed C token; returns its `TokenId`.
fn seed_token(
    bus: &mut CompilerBus,
    source: cc_silicon_compiler::ids::SourceId,
    kind: TokenKind,
    spelling: &[u8],
    pp_kind: PpTokenKind,
    name: Option<cc_silicon_compiler::ids::NameId>,
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

fn seed_literal(bus: &mut CompilerBus, token: TokenId, spelling: &[u8], value: u8) {
    let limits = bus.limits();
    bus.arenas
        .literals
        .alloc(
            LiteralRecord {
                token: Some(token),
                kind: LiteralKind::Integer,
                radix: 10,
                suffix: LiteralSuffix::None,
                value: vec![value],
                negative: false,
                spelling: spelling.to_vec(),
                candidate_type: Lx08CandidateType::Int,
            },
            &limits,
        )
        .unwrap();
}

/// Seed the committed M1 expression tokens `2 + 3` plus a unary `-` in
/// source order; returns the token IDs.
fn seed_m1_expr(bus: &mut CompilerBus) -> Vec<TokenId> {
    let file = bus.intern_name(b"m1.c").unwrap();
    let source = bus.alloc_source(file, b"2+3-".to_vec()).unwrap();
    let n = TokenKind::Integer;
    let p = TokenKind::Punctuator;
    let num = PpTokenKind::PpNumber;
    let pu = PpTokenKind::Punctuator;
    let tokens = vec![
        seed_token(bus, source, n, b"2", num, None),
        seed_token(bus, source, p, b"+", pu, None),
        seed_token(bus, source, n, b"3", num, None),
        seed_token(bus, source, p, b"-", pu, None),
    ];
    seed_literal(bus, tokens[0], b"2", 2);
    seed_literal(bus, tokens[2], b"3", 3);
    tokens
}

fn refs(ids: &[TokenId]) -> Payload {
    Payload::from_refs(ids.iter().map(|id| RecordRef::Token(*id)).collect())
}

fn run_scenario() -> (Vec<u8>, usize, usize) {
    let mut bus = new_bus();
    install_pa_expr(&mut bus);
    let workers = workers();
    let tokens = seed_m1_expr(&mut bus);
    bootstrap(
        &mut bus,
        TaskKind::PARSE_PRIMARY,
        PA16_CHIP,
        refs(&tokens[0..1]),
    );
    bootstrap(
        &mut bus,
        TaskKind::PARSE_BINARY,
        PA16_CHIP,
        refs(&tokens[0..3]),
    );
    bootstrap(
        &mut bus,
        TaskKind::PARSE_UNARY,
        PA20_CHIP,
        refs(&tokens[1..3]),
    );
    bootstrap(
        &mut bus,
        TaskKind::PARSE_UNARY,
        PA20_CHIP,
        Payload::from_refs(vec![
            RecordRef::Token(tokens[3]),
            RecordRef::Token(tokens[2]),
        ]),
    );
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
        if completed + failed == 4 {
            break;
        }
    }
    (Snapshot::capture(&bus).bytes().to_vec(), completed, failed)
}

#[test]
fn bus_dispatch_commits_ack_and_replays_deterministically() {
    let (first, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (4, 0));
    let (second, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (4, 0));
    assert_eq!(first, second);
}

// --- 9. gates ------------------------------------------------------------------

#[test]
fn expr_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    // A wrong layer is refused on the driver path for all three kinds.
    let cases: [(TaskKind, ChipId, &dyn Worker); 3] = [
        (TaskKind::PARSE_PRIMARY, PA16_CHIP, &PaBinaryChip),
        (TaskKind::PARSE_BINARY, PA16_CHIP, &PaBinaryChip),
        (TaskKind::PARSE_UNARY, PA20_CHIP, &PaUnaryChip),
    ];
    for (kind, chip, worker) in cases {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::pa_expr_slice();
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
    // The pre-`/35` registry knows none of the three kinds, so both
    // manifests are rejected there (frozen kinds are never re-registered).
    let mut stale = ManifestRegistry::new();
    let kinds = TaskKindRegistry::pa_decl_slice();
    let schema = StoreSchema::pa_slice();
    for worker in [&PaBinaryChip as &dyn Worker, &PaUnaryChip] {
        assert!(stale.register(worker.manifest(), &schema, &kinds).is_err());
    }
}
