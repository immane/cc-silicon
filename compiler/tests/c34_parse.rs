// ============================================================================
// c34_parse.rs — Wave 3 (`/34`) PA decl-slice acceptance.
//
// Covers the frozen closure: the `parse.external_declaration` (local 17),
// `parse.specifiers` (local 18), `parse.declarator` (local 19),
// `parse.block` (local 20), and `parse.return` (local 21) kinds,
// kind-to-stage assignment (stage 2), the PA02/PA03/PA05/PA28 manifests
// (all Ack-only with zero writes and no allowlist rows), the dispatch
// rule (`{` vs `;`, third token is a loud defect), the M1 specifier
// (`int` alone), the M1 declarator (`main(void)` only, `()` a typed
// DEFECT), the fixed block/return shapes (7/5 tokens with committed
// integer literals), bus dispatch (all `Ack`, replay determinism), and
// stage/layer gates.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, LiteralRecord, PpTokenKind, PpTokenRecord, SpanRecord, TokenKind,
    TokenRecord,
};
use cc_silicon_compiler::chips::{
    external_decl_kind, handler_for, parse_block, parse_declarator, parse_return, parse_specifier,
    DeclaratorError, DeclaratorToken, ExternalDeclKind, ExternalToken, PaBlockChip, PaBlockInput,
    PaDeclaratorChip, PaDeclaratorInput, PaDeclaratorProjectedToken, PaExternalChip,
    PaExternalInput, PaReturnInput, PaSpecifierChip, PaSpecifierInput, ProjectedBlockToken,
    ProjectedSpecifierToken, Worker, WorkerRegistry, PA02_TASK_KIND, PA03_TASK_KIND,
    PA05_TASK_KIND, PA28_TASK_KIND, PA32_TASK_KIND,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{ChipId, NameId, RecordRef, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    check_stage_layer_agreement, is_pa_decl_slice_kind, stage_of, PA02_CHIP, PA03_CHIP, PA05_CHIP,
    PA28_CHIP, STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType, Snapshot};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    ContinuationRecord, ParseContext, Payload, Proposal, ResultValue, TaskDraft, TaskGroup,
    TaskKind, TaskKindRegistry, TaskState,
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

fn install_pa_decl(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::StoreSchema;
    bus.kinds = TaskKindRegistry::pa_decl_slice();
    bus.schema = StoreSchema::pa_slice();
    for chip in [
        &PaExternalChip as &dyn Worker,
        &PaSpecifierChip,
        &PaDeclaratorChip,
        &PaBlockChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(TaskKind::PARSE_EXTERNAL_DECL, PA02_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_SPECIFIERS, PA03_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_DECLARATOR, PA05_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_BLOCK, PA28_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::PARSE_RETURN, PA28_CHIP, 2)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PaExternalChip).unwrap();
    workers.register(PaSpecifierChip).unwrap();
    workers.register(PaDeclaratorChip).unwrap();
    workers.register(PaBlockChip).unwrap();
    workers
}

fn bootstrap(bus: &mut CompilerBus, kind: TaskKind, owner: ChipId, payload: Payload) -> TaskId {
    bootstrap_with_continuation(bus, kind, owner, payload, None)
}

fn bootstrap_with_continuation(
    bus: &mut CompilerBus,
    kind: TaskKind,
    owner: ChipId,
    payload: Payload,
    continuation: Option<cc_silicon_compiler::ids::ContinuationId>,
) -> TaskId {
    bus.bootstrap_task(TaskDraft {
        kind,
        owner,
        parent: None,
        payload,
        continuation,
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

fn ext_window(slots: &[(TokenKind, &[u8])]) -> Vec<ExternalToken> {
    slots
        .iter()
        .enumerate()
        .map(|(index, (kind, spelling))| ExternalToken {
            id: TokenId::from_index(index as u32),
            kind: *kind,
            spelling: spelling.to_vec(),
        })
        .collect()
}

fn ext_input(slots: &[(TokenKind, &[u8])]) -> PaExternalInput {
    PaExternalInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        production: PA02_TASK_KIND,
        token_cursor: TokenId::from_index(0),
        scope: None,
        context: ParseContext::ExternalDecl,
        binding_power: 0,
        window: ext_window(slots),
    }
}

const M1_DISPATCH_FN: &[(TokenKind, &[u8])] = &[
    (TokenKind::Keyword, b"int"),
    (TokenKind::Identifier, b"main"),
    (TokenKind::Punctuator, b"("),
    (TokenKind::Keyword, b"void"),
    (TokenKind::Punctuator, b")"),
    (TokenKind::Punctuator, b"{"),
];

fn spec_input(kind: TokenKind, spelling: &[u8]) -> PaSpecifierInput {
    let id = TokenId::from_index(0);
    let mut bodies = std::collections::BTreeMap::new();
    bodies.insert(
        id,
        ProjectedSpecifierToken {
            kind,
            spelling: spelling.to_vec(),
        },
    );
    PaSpecifierInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens: vec![id],
        bodies,
    }
}

fn decl_token(kind: TokenKind, spelling: &[u8]) -> DeclaratorToken {
    DeclaratorToken {
        kind,
        spelling: spelling.to_vec(),
        name: None,
    }
}

fn decl_input(views: &[DeclaratorToken]) -> PaDeclaratorInput {
    let mut tokens = Vec::new();
    let mut bodies = std::collections::BTreeMap::new();
    for (index, view) in views.iter().enumerate() {
        let id = TokenId::from_index(index as u32);
        tokens.push(id);
        let (spelling, pp_spelling) = match view.kind {
            TokenKind::Identifier | TokenKind::Keyword => (view.spelling.clone(), Vec::new()),
            _ => (Vec::new(), view.spelling.clone()),
        };
        bodies.insert(
            id,
            PaDeclaratorProjectedToken {
                kind: view.kind,
                name: view.name,
                spelling,
                pp_spelling,
            },
        );
    }
    PaDeclaratorInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        bodies,
    }
}

fn m1_declarator_views() -> Vec<DeclaratorToken> {
    vec![
        decl_token(TokenKind::Identifier, b"main"),
        decl_token(TokenKind::Punctuator, b"("),
        decl_token(TokenKind::Keyword, b"void"),
        decl_token(TokenKind::Punctuator, b")"),
    ]
}

fn block_shapes(
    kinds: &[TokenKind],
    return_at: usize,
    int_at: &[usize],
) -> (
    Vec<TokenId>,
    std::collections::BTreeMap<TokenId, ProjectedBlockToken>,
) {
    let mut tokens = Vec::new();
    let mut bodies = std::collections::BTreeMap::new();
    for (index, kind) in kinds.iter().enumerate() {
        let id = TokenId::from_index(index as u32);
        tokens.push(id);
        let spelling = if index == return_at {
            b"return".to_vec()
        } else {
            Vec::new()
        };
        bodies.insert(
            id,
            ProjectedBlockToken {
                kind: *kind,
                spelling,
            },
        );
    }
    let _ = int_at;
    (tokens, bodies)
}

fn block_input() -> PaBlockInput {
    let kinds = [
        TokenKind::Punctuator,
        TokenKind::Keyword,
        TokenKind::Integer,
        TokenKind::Punctuator,
        TokenKind::Integer,
        TokenKind::Punctuator,
        TokenKind::Punctuator,
    ];
    let (tokens, bodies) = block_shapes(&kinds, 1, &[2, 4]);
    let mut literals_by_token = std::collections::BTreeMap::new();
    for position in [2, 4] {
        literals_by_token.insert(
            tokens[position],
            cc_silicon_compiler::ids::LiteralId::from_index(position as u32),
        );
    }
    PaBlockInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        bodies,
        literals_by_token,
    }
}

fn return_input() -> PaReturnInput {
    let kinds = [
        TokenKind::Keyword,
        TokenKind::Integer,
        TokenKind::Punctuator,
        TokenKind::Integer,
        TokenKind::Punctuator,
    ];
    let (tokens, bodies) = block_shapes(&kinds, 0, &[1, 3]);
    let mut literals_by_token = std::collections::BTreeMap::new();
    for position in [1, 3] {
        literals_by_token.insert(
            tokens[position],
            cc_silicon_compiler::ids::LiteralId::from_index(position as u32),
        );
    }
    PaReturnInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tokens,
        bodies,
        literals_by_token,
    }
}

// --- 1. freeze ---------------------------------------------------------------

#[test]
fn decl_kinds_stage_registry_manifest_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(TaskKind::PARSE_EXTERNAL_DECL.group(), TaskGroup::PARSE);
    assert_eq!(TaskKind::PARSE_EXTERNAL_DECL.local(), 17);
    assert_eq!(TaskKind::PARSE_SPECIFIERS.local(), 18);
    assert_eq!(TaskKind::PARSE_DECLARATOR.local(), 19);
    assert_eq!(TaskKind::PARSE_BLOCK.local(), 20);
    assert_eq!(TaskKind::PARSE_RETURN.local(), 21);
    assert_eq!(PA02_TASK_KIND, TaskKind::PARSE_EXTERNAL_DECL);
    assert_eq!(PA03_TASK_KIND, TaskKind::PARSE_SPECIFIERS);
    assert_eq!(PA05_TASK_KIND, TaskKind::PARSE_DECLARATOR);
    assert_eq!(PA28_TASK_KIND, TaskKind::PARSE_BLOCK);
    assert_eq!(PA32_TASK_KIND, TaskKind::PARSE_RETURN);
    for kind in [
        TaskKind::PARSE_EXTERNAL_DECL,
        TaskKind::PARSE_SPECIFIERS,
        TaskKind::PARSE_DECLARATOR,
        TaskKind::PARSE_BLOCK,
        TaskKind::PARSE_RETURN,
    ] {
        assert!(is_pa_decl_slice_kind(kind));
        assert_eq!(stage_of(kind), Some(2));
    }
    assert!(!is_pa_decl_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_pa_decl_slice_kind(TaskKind::PARSE_TU));
    // The decl registry extends the `/33` head linearly; `PARSE` owners
    // start new codes at local 27 after the `/36` recovery slice.
    assert_eq!(TaskKindRegistry::pa_slice().len(), 12);
    assert_eq!(TaskKindRegistry::lx_string_slice().len(), 53);
    let registry = TaskKindRegistry::pa_decl_slice();
    assert_eq!(registry.len(), 58);
    for (kind, name) in [
        (TaskKind::PARSE_EXTERNAL_DECL, "parse.external_declaration"),
        (TaskKind::PARSE_SPECIFIERS, "parse.specifiers"),
        (TaskKind::PARSE_DECLARATOR, "parse.declarator"),
        (TaskKind::PARSE_BLOCK, "parse.block"),
        (TaskKind::PARSE_RETURN, "parse.return"),
    ] {
        assert_eq!(registry.lookup(kind).unwrap().name, name);
    }
    assert_eq!(stage_of(TaskKind::new(TaskGroup::PARSE, 27).unwrap()), None);
    // All four chips are Ack-only: zero writes, no allowlist rows.
    assert_eq!(PA02_CHIP, cc_silicon_compiler::ids::ChipId(44));
    assert_eq!(PA03_CHIP, cc_silicon_compiler::ids::ChipId(45));
    assert_eq!(PA05_CHIP, cc_silicon_compiler::ids::ChipId(46));
    assert_eq!(PA28_CHIP, cc_silicon_compiler::ids::ChipId(47));
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 32);
    for chip in [PA02_CHIP, PA03_CHIP, PA05_CHIP, PA28_CHIP] {
        assert!(
            !STORE_OWNER_ALLOWLIST
                .iter()
                .any(|&(owner, _, _, _)| owner == chip),
            "Ack-only chip must hold no allowlist row"
        );
    }
    let manifests = [
        PaExternalChip.manifest(),
        PaSpecifierChip.manifest(),
        PaDeclaratorChip.manifest(),
        PaBlockChip.manifest(),
    ];
    for manifest in &manifests {
        assert!(manifest.writes.is_empty());
        assert!(manifest.deterministic);
        assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
        assert!(manifest.declares_read(StoreId::Lex, "tokens"));
        assert_eq!(manifest.tests, vec!["compiler/tests/c34_parse.rs"]);
        let routing = {
            let mut bus = new_bus();
            install_pa_decl(&mut bus);
            bus.routing
        };
        assert!(check_stage_layer_agreement(manifest, &routing).is_ok());
    }
    assert_eq!(
        PaBlockChip.manifest().task_kinds,
        vec![PA28_TASK_KIND, PA32_TASK_KIND]
    );
}

// --- 2. external dispatch ----------------------------------------------------

#[test]
fn external_fn_vs_decl() {
    assert_eq!(
        external_decl_kind(&ext_input(M1_DISPATCH_FN)),
        Ok(ExternalDeclKind::FunctionDefinition)
    );
    let mut decl = M1_DISPATCH_FN.to_vec();
    decl[5] = (TokenKind::Punctuator, b";");
    assert_eq!(
        external_decl_kind(&ext_input(&decl)),
        Ok(ExternalDeclKind::Declaration)
    );
    assert!(is_ack(&PaExternalChip.compute(&ext_input(M1_DISPATCH_FN))));
    assert!(is_ack(&PaExternalChip.compute(&ext_input(&decl))));
}

#[test]
fn external_rejects_non_m1() {
    // `unsigned` specifier is explicit Unsupported, never a pass.
    let mut unsigned = M1_DISPATCH_FN.to_vec();
    unsigned[0] = (TokenKind::Keyword, b"unsigned");
    let out = PaExternalChip.compute(&ext_input(&unsigned));
    assert_eq!(fail_group(&out), Some(DiagGroup::Unsupported));
    // A third discriminator (`=`, K&R name, EOF-shaped `)`) fails loud.
    for discriminator in [b"=".as_slice(), b"x".as_slice(), b")".as_slice()] {
        let mut other = M1_DISPATCH_FN.to_vec();
        other[5] = (TokenKind::Punctuator, discriminator);
        let out = PaExternalChip.compute(&ext_input(&other));
        assert!(
            fail_group(&out).is_some(),
            "discriminator {discriminator:?} must fail"
        );
    }
    // Truncated window is explicit Unsupported.
    let out = PaExternalChip.compute(&ext_input(&M1_DISPATCH_FN[..4]));
    assert_eq!(fail_group(&out), Some(DiagGroup::Unsupported));
    // Non-running tasks fail on the Task channel.
    let mut idle = ext_input(M1_DISPATCH_FN);
    idle.state = TaskState::Ready;
    assert_eq!(
        fail_group(&PaExternalChip.compute(&idle)),
        Some(DiagGroup::Task)
    );
}

// --- 3. specifier ------------------------------------------------------------

#[test]
fn specifier_int_vs_unsigned() {
    let ok = PaSpecifierChip.compute(&spec_input(TokenKind::Keyword, b"int"));
    assert!(is_ack(&ok));
    let shape = parse_specifier(&spec_input(TokenKind::Keyword, b"int")).expect("M1 `int` parses");
    assert_eq!(shape.kind_name, "Specifiers");
    assert!(shape.children.is_empty());
    // `unsigned` (and every other bundle) fails Unsupported.
    for (kind, spelling) in [
        (TokenKind::Keyword, b"unsigned".as_slice()),
        (TokenKind::Keyword, b"long".as_slice()),
        (TokenKind::Identifier, b"int".as_slice()),
    ] {
        let out = PaSpecifierChip.compute(&spec_input(kind, spelling));
        assert_eq!(
            fail_group(&out),
            Some(DiagGroup::Unsupported),
            "must fail: {spelling:?}"
        );
    }
    // Arity violations and non-running tasks fail loud.
    let mut empty = spec_input(TokenKind::Keyword, b"int");
    empty.tokens.clear();
    assert_eq!(
        fail_group(&PaSpecifierChip.compute(&empty)),
        Some(DiagGroup::Unsupported)
    );
    let mut idle = spec_input(TokenKind::Keyword, b"int");
    idle.state = TaskState::Ready;
    assert_eq!(
        fail_group(&PaSpecifierChip.compute(&idle)),
        Some(DiagGroup::Task)
    );
}

// --- 4. declarator -----------------------------------------------------------

#[test]
fn declarator_void_vs_empty_parens() {
    let views = m1_declarator_views();
    let tree = parse_declarator(&views).expect("M1 `main(void)` validates");
    assert_eq!(tree.name_spelling(), b"main");
    assert!(tree.is_zero_params());
    assert_eq!(tree.param_count(), 0);
    assert!(tree.is_void_prototype());
    assert!(is_ack(&PaDeclaratorChip.compute(&decl_input(&views))));
    // The ambiguous `()` shape fails on the Task channel (DEFECT).
    let ambiguous = vec![
        decl_token(TokenKind::Identifier, b"main"),
        decl_token(TokenKind::Punctuator, b"("),
        decl_token(TokenKind::Punctuator, b")"),
    ];
    assert_eq!(
        parse_declarator(&ambiguous),
        Err(DeclaratorError::AmbiguousEmptyParams)
    );
    let out = PaDeclaratorChip.compute(&decl_input(&ambiguous));
    assert_eq!(fail_group(&out), Some(DiagGroup::Task));
}

#[test]
fn declarator_rejects_non_m1() {
    // Pointer, parenthesized, array, non-main, and non-void shapes are
    // explicit Unsupported.
    let cases: Vec<Vec<DeclaratorToken>> = vec![
        vec![
            decl_token(TokenKind::Punctuator, b"*"),
            decl_token(TokenKind::Identifier, b"f"),
            decl_token(TokenKind::Punctuator, b"("),
            decl_token(TokenKind::Keyword, b"void"),
            decl_token(TokenKind::Punctuator, b")"),
        ],
        vec![
            decl_token(TokenKind::Identifier, b"other"),
            decl_token(TokenKind::Punctuator, b"("),
            decl_token(TokenKind::Keyword, b"void"),
            decl_token(TokenKind::Punctuator, b")"),
        ],
        vec![
            decl_token(TokenKind::Identifier, b"main"),
            decl_token(TokenKind::Punctuator, b"("),
            decl_token(TokenKind::Keyword, b"int"),
            decl_token(TokenKind::Punctuator, b")"),
        ],
        vec![
            decl_token(TokenKind::Identifier, b"main"),
            decl_token(TokenKind::Punctuator, b"["),
            decl_token(TokenKind::Punctuator, b"]"),
        ],
    ];
    for views in &cases {
        let out = PaDeclaratorChip.compute(&decl_input(views));
        assert_eq!(fail_group(&out), Some(DiagGroup::Unsupported));
    }
    let mut idle = decl_input(&m1_declarator_views());
    idle.state = TaskState::Ready;
    assert_eq!(
        fail_group(&PaDeclaratorChip.compute(&idle)),
        Some(DiagGroup::Task)
    );
}

// --- 5. block / return -------------------------------------------------------

#[test]
fn block_seven_token_shape() {
    let input = block_input();
    let shapes = parse_block(&input).expect("M1 block validates");
    assert_eq!(shapes.len(), 5);
    assert_eq!(shapes[0].kind_name, "Compound");
    assert_eq!(shapes[0].parent, None);
    assert_eq!((shapes[0].first_token, shapes[0].last_token), (0, 6));
    assert_eq!(shapes[1].kind_name, "Return");
    assert_eq!(shapes[2].kind_name, "BinaryAdd");
    assert!(shapes[3].has_literal && shapes[4].has_literal);
    assert!(is_ack(&PaBlockChip.compute_block(&input)));
    // `{}` (empty block) is explicit Unsupported.
    let mut hollow = block_input();
    hollow.tokens.truncate(2);
    assert_eq!(
        fail_group(&PaBlockChip.compute_block(&hollow)),
        Some(DiagGroup::Unsupported)
    );
}

#[test]
fn return_five_token_shape() {
    let input = return_input();
    let shapes = parse_return(&input).expect("M1 return validates");
    assert_eq!(shapes.len(), 4);
    assert_eq!(shapes[0].kind_name, "Return");
    assert_eq!(shapes[0].parent, None);
    assert_eq!((shapes[0].first_token, shapes[0].last_token), (0, 4));
    assert_eq!(shapes[1].kind_name, "BinaryAdd");
    assert!(is_ack(&PaBlockChip.compute_return(&input)));
    // `return;` (no expression) is explicit Unsupported, never "return zero".
    let mut bare = return_input();
    bare.tokens.truncate(2);
    assert_eq!(
        fail_group(&PaBlockChip.compute_return(&bare)),
        Some(DiagGroup::Unsupported)
    );
    // Missing literals are Task-channel protocol faults.
    let mut no_lit = return_input();
    no_lit.literals_by_token.clear();
    assert_eq!(
        fail_group(&PaBlockChip.compute_return(&no_lit)),
        Some(DiagGroup::Task)
    );
}

// --- 6. cross-chip negatives + determinism -----------------------------------

#[test]
fn non_m1_unsupported_matrix() {
    // K&R parameter name in the external window: not M1, fails loud.
    let mut knr = M1_DISPATCH_FN.to_vec();
    knr[3] = (TokenKind::Identifier, b"argc");
    assert!(fail_group(&PaExternalChip.compute(&ext_input(&knr))).is_some());
    // `;` alone is not an expression statement here: block/return reject it.
    let mut lone = return_input();
    lone.tokens.truncate(1);
    assert_eq!(
        fail_group(&PaBlockChip.compute_return(&lone)),
        Some(DiagGroup::Unsupported)
    );
    // Pure cores are deterministic: repeated evaluation agrees.
    let input = ext_input(M1_DISPATCH_FN);
    assert_eq!(external_decl_kind(&input), external_decl_kind(&input));
    let decl_views = m1_declarator_views();
    assert_eq!(parse_declarator(&decl_views), parse_declarator(&decl_views));
    assert_eq!(
        PaDeclaratorChip.compute(&decl_input(&decl_views)),
        PaDeclaratorChip.compute(&decl_input(&decl_views))
    );
}

#[test]
fn compute_rejects_non_running() {
    let mut spec = spec_input(TokenKind::Keyword, b"int");
    spec.state = TaskState::Ready;
    assert!(fail_group(&PaSpecifierChip.compute(&spec)).is_some());
    let mut decl = decl_input(&m1_declarator_views());
    decl.state = TaskState::Ready;
    assert!(fail_group(&PaDeclaratorChip.compute(&decl)).is_some());
    let mut block = block_input();
    block.state = TaskState::Ready;
    assert!(fail_group(&PaBlockChip.compute_block(&block)).is_some());
    let mut ret = return_input();
    ret.state = TaskState::Ready;
    assert!(fail_group(&PaBlockChip.compute_return(&ret)).is_some());
    let mut ext = ext_input(M1_DISPATCH_FN);
    ext.state = TaskState::Ready;
    assert!(fail_group(&PaExternalChip.compute(&ext)).is_some());
}

// --- 7. bus dispatch ---------------------------------------------------------

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

/// Seed the committed M1 token prefix `int main(void){return 2+3;}` in
/// source order; returns the token IDs.
fn seed_m1_prefix(bus: &mut CompilerBus) -> Vec<TokenId> {
    let file = bus.intern_name(b"m1.c").unwrap();
    let source = bus
        .alloc_source(file, b"int main(void){return 2+3;}".to_vec())
        .unwrap();
    let int = bus.intern_name(b"int").unwrap();
    let main = bus.intern_name(b"main").unwrap();
    let void = bus.intern_name(b"void").unwrap();
    let ret = bus.intern_name(b"return").unwrap();
    let k = TokenKind::Keyword;
    let i = TokenKind::Identifier;
    let p = TokenKind::Punctuator;
    let n = TokenKind::Integer;
    let id = PpTokenKind::Identifier;
    let pu = PpTokenKind::Punctuator;
    let num = PpTokenKind::PpNumber;
    let tokens = vec![
        seed_token(bus, source, k, b"int", id, Some(int)),
        seed_token(bus, source, i, b"main", id, Some(main)),
        seed_token(bus, source, p, b"(", pu, None),
        seed_token(bus, source, k, b"void", id, Some(void)),
        seed_token(bus, source, p, b")", pu, None),
        seed_token(bus, source, p, b"{", pu, None),
        seed_token(bus, source, k, b"return", id, Some(ret)),
        seed_token(bus, source, n, b"2", num, None),
        seed_token(bus, source, p, b"+", pu, None),
        seed_token(bus, source, n, b"3", num, None),
        seed_token(bus, source, p, b";", pu, None),
        seed_token(bus, source, p, b"}", pu, None),
    ];
    seed_literal(bus, tokens[7], b"2", 2);
    seed_literal(bus, tokens[9], b"3", 3);
    tokens
}

fn refs(ids: &[TokenId]) -> Payload {
    Payload::from_refs(ids.iter().map(|id| RecordRef::Token(*id)).collect())
}

fn run_scenario() -> (Vec<u8>, usize, usize) {
    let mut bus = new_bus();
    install_pa_decl(&mut bus);
    let workers = workers();
    let tokens = seed_m1_prefix(&mut bus);
    // External dispatch at the `int` cursor with an ExternalDecl frame.
    let limits = bus.limits();
    let continuation = bus
        .arenas
        .continuations
        .alloc(
            ContinuationRecord {
                production: PA02_TASK_KIND,
                cursor: tokens[0],
                context: ParseContext::ExternalDecl,
                binding_power: 0,
                scope: None,
                parent: None,
                partial_children: Vec::new(),
                next_child_ordinal: 0,
                previous: None,
            },
            &limits,
        )
        .unwrap();
    bootstrap_with_continuation(
        &mut bus,
        TaskKind::PARSE_EXTERNAL_DECL,
        PA02_CHIP,
        Payload::empty(),
        Some(continuation),
    );
    bootstrap(
        &mut bus,
        TaskKind::PARSE_SPECIFIERS,
        PA03_CHIP,
        refs(&tokens[0..1]),
    );
    bootstrap(
        &mut bus,
        TaskKind::PARSE_DECLARATOR,
        PA05_CHIP,
        refs(&tokens[1..5]),
    );
    bootstrap(
        &mut bus,
        TaskKind::PARSE_BLOCK,
        PA28_CHIP,
        refs(&tokens[5..12]),
    );
    bootstrap(
        &mut bus,
        TaskKind::PARSE_RETURN,
        PA28_CHIP,
        refs(&tokens[6..11]),
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
        if completed + failed == 5 {
            break;
        }
    }
    (Snapshot::capture(&bus).bytes().to_vec(), completed, failed)
}

#[test]
fn bus_dispatch_commits_ack_and_replays_deterministically() {
    let (first, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (5, 0));
    let (second, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (5, 0));
    assert_eq!(first, second);
}

// --- 8. gates ----------------------------------------------------------------

#[test]
fn decl_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    // A wrong layer is refused on the driver path for all five kinds.
    let cases: [(TaskKind, ChipId, &dyn Worker); 5] = [
        (TaskKind::PARSE_EXTERNAL_DECL, PA02_CHIP, &PaExternalChip),
        (TaskKind::PARSE_SPECIFIERS, PA03_CHIP, &PaSpecifierChip),
        (TaskKind::PARSE_DECLARATOR, PA05_CHIP, &PaDeclaratorChip),
        (TaskKind::PARSE_BLOCK, PA28_CHIP, &PaBlockChip),
        (TaskKind::PARSE_RETURN, PA28_CHIP, &PaBlockChip),
    ];
    for (kind, chip, worker) in cases {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::pa_decl_slice();
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
    // The pre-`/34` registry knows none of the five kinds, so all
    // manifests are rejected there (frozen kinds are never re-registered).
    let mut stale = ManifestRegistry::new();
    let kinds = TaskKindRegistry::lx_string_slice();
    let schema = StoreSchema::pa_slice();
    for worker in [
        &PaExternalChip as &dyn Worker,
        &PaSpecifierChip,
        &PaDeclaratorChip,
        &PaBlockChip,
    ] {
        assert!(stale.register(worker.manifest(), &schema, &kinds).is_err());
    }
}
