// ============================================================================
// chips/pa_declarator.rs — T05 PA05/PA07/PA09 declarator worker (M1 slice)
// (Wave 3 slice 9, `/34`)
//
// One fused worker covering three T05 rows for the M1 canonical declarator
// `main(void)` only:
//
// * PA05 DeclaratorChip — coordinate the pointer and direct parts and
//   preserve correct binding (`int *f(void)` vs `int (*f)(void)`).
// * PA07 DirectDeclaratorChip — identifier / parenthesized declarator and
//   the suffix loop.
// * PA09 FunctionDeclaratorChip — `(void)` / prototype / variadic /
//   old-style identifier list.
//
// M1 scope: the direct declarator `main` plus exactly one function suffix
// `(void)` (zero parameters, prototype). Every other well-formed C shape —
// pointer declarators (PA06 territory), parenthesized declarators, array
// suffixes (PA08 territory), non-`void` / variadic / old-style parameters,
// non-`main` names — is an explicit `Unsupported` failure, never a
// fabricated success. The empty parameter list `()` is NOT treated as
// `(void)`: it is ambiguous across C dialects/modes (unspecified parameters
// vs zero parameters, vs an old-style definition per mode) and fails as a
// typed DEFECT diagnostic (see [`DeclaratorError::AmbiguousEmptyParams`]).
//
// The worker is pure plus `Ack`: it validates the committed token slice and
// completes with [`ResultValue::Ack`], appending no records. The validated
// shape is certified through the pure core ([`parse_declarator`],
// [`parse_direct_declarator`], [`parse_parameter_list`]) plus the
// [`DeclaratorTree`] accessor functions; no shared struct crosses chips.
//
// Frozen registration (integrator-owned): `PA05_TASK_KIND` aliases
// `TaskKind::PARSE_DECLARATOR` (`PARSE` local 19, `parse.declarator`),
// `PA05_CHIP` is `crate::manifest::PA05_CHIP` (`ChipId(46)`), the
// kind-registry row lives in `TaskKindRegistry::pa_decl_slice()`, the
// stage-2 row in `STAGE_ASSIGNMENT`, the routed layer is 2, there is no
// store-owner allowlist row (Ack-only, zero writes), and the acceptance
// test is `compiler/tests/c34_parse.rs`. Numeric diagnostic codes below
// are chip-local candidates; the exact codes and hashes remain T01 `/6`
// details.
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NameId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA05_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Frozen task kind served by the declarator worker.
///
/// `PARSE` local 19 — the first free local after `PARSE_SPECIFIERS`
/// (local 18); registered `Frozen` in `TaskKindRegistry::pa_decl_slice()`.
pub const PA05_TASK_KIND: TaskKind = TaskKind::PARSE_DECLARATOR;

/// One projected token: kind plus resolved spellings.
///
/// Identifier/keyword spellings resolve through the intern table (`spelling`);
/// punctuator spellings resolve through the committed PP token
/// (`pp_spelling`), because committed C punctuator records carry no interned
/// name. Either side is empty only when the backing record is missing, which
/// the computation reports loudly instead of guessing.
#[derive(Clone, Debug)]
pub struct ProjectedToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Interned spelling bytes for named tokens.
    pub spelling: Vec<u8>,
    /// Committed PP spelling bytes (punctuator ground truth).
    pub pp_spelling: Vec<u8>,
}

/// Narrow projection for the declarator computation.
#[derive(Clone, Debug)]
pub struct PaDeclaratorInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (the declarator slice only).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedToken>,
}

/// Build the narrow projection for one dispatched task.
///
/// Payload convention (candidate): the declarator token refs in source
/// order — exactly `[main, (, void, )]` for M1, or the 3-token `main()`
/// shape that the computation rejects as ambiguous. The payload carries no
/// EOF and no surrounding specifier/body tokens. A wrong kind, an empty
/// payload, or a non-token ref is a caller protocol fault; a missing
/// committed record is a task error surfaced by the computation.
pub fn project_pa_declarator_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaDeclaratorInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("declarator of unknown task {}", task.index())))?;
    if record.kind != PA05_TASK_KIND {
        return Err(protocol_fault(format!(
            "declarator task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "declarator task {} payload must carry the declarator token refs",
            task.index()
        )));
    }
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "declarator task {} payload must carry tokens only",
                task.index()
            )));
        };
        tokens.push(*id);
        if let Ok(body) = bus.arenas.tokens.get(*id) {
            let spelling = body
                .name
                .and_then(|name| bus.intern.get(name).ok().map(|bytes| bytes.to_vec()))
                .unwrap_or_default();
            let pp_spelling = bus
                .arenas
                .pp_tokens
                .get(body.pp_token)
                .map(|pp| pp.spelling.clone())
                .unwrap_or_default();
            bodies.insert(
                *id,
                ProjectedToken {
                    kind: body.kind,
                    name: body.name,
                    spelling,
                    pp_spelling,
                },
            );
        }
    }
    Ok(PaDeclaratorInput {
        task,
        state: record.state.clone(),
        tokens,
        bodies,
    })
}

/// The T05 PA05/PA07/PA09 declarator worker (M1 slice).
pub struct PaDeclaratorChip;

impl Worker for PaDeclaratorChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA05_CHIP,
            chip_name: "PaDeclaratorChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![PA05_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Names, "entries"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c34_parse.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pa_declarator_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// One declarator token for the pure grammar core.
///
/// `spelling` is the effective spelling: intern bytes for identifiers and
/// keywords, committed PP bytes for punctuators. The projector resolves
/// which side applies; the pure core only compares bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaratorToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Effective spelling bytes.
    pub spelling: Vec<u8>,
    /// Interned name, if any (carried for the declarator-name accessor).
    pub name: Option<NameId>,
}

/// Why a token slice is not the M1 declarator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclaratorError {
    /// A well-formed (or malformed) shape outside the M1 subset: pointer or
    /// parenthesized declarators, array suffixes, non-`void` / variadic /
    /// old-style parameters, non-`main` names. Maps to `Unsupported`.
    Unsupported(&'static str),
    /// The empty parameter list `()`: ambiguous across modes (unspecified
    /// parameters vs zero parameters vs old-style definition) and therefore
    /// a DEFECT, never silently read as `(void)`.
    AmbiguousEmptyParams,
}

impl DeclaratorError {
    /// Map to the structured failure diagnostic for this rejection.
    ///
    /// Numeric codes are chip-local candidates; the frozen code table is a
    /// T01 `/6` detail.
    pub fn diagnostic(&self) -> DiagnosticDraft {
        match self {
            Self::Unsupported(message) => DiagnosticDraft::unsupported(*message),
            Self::AmbiguousEmptyParams => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "DEFECT: ambiguous empty parameter list `()`; M1 requires `(void)`",
            ),
        }
    }
}

/// Validated M1 parameter list: `(void)`, zero parameters, prototype.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamList {
    /// Whether the list is a prototype (always true in M1).
    is_void_prototype: bool,
    /// Parameter count (always 0 in M1).
    count: u32,
}

impl ParamList {
    /// Whether the parameter list is the `(void)` prototype.
    pub fn is_void_prototype(&self) -> bool {
        self.is_void_prototype
    }
    /// Number of parameters (0 for `(void)`).
    pub fn count(&self) -> u32 {
        self.count
    }
    /// Whether the list carries zero parameters.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Validated M1 declarator tree: `main(void)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaratorTree {
    /// Interned declarator name (`main` in M1).
    name: Option<NameId>,
    /// Declarator name spelling bytes.
    spelling: Vec<u8>,
    /// Validated parameter list.
    params: ParamList,
}

impl DeclaratorTree {
    /// Interned declarator name, if the token carried one.
    pub fn declarator_name(&self) -> Option<NameId> {
        self.name
    }
    /// Declarator name spelling bytes (`main` in M1).
    pub fn name_spelling(&self) -> &[u8] {
        &self.spelling
    }
    /// Validated parameter list.
    pub fn params(&self) -> &ParamList {
        &self.params
    }
    /// Whether the declarator takes zero parameters (`(void)`).
    pub fn is_zero_params(&self) -> bool {
        self.params.is_empty()
    }
    /// Number of parameters (0 for `(void)`).
    pub fn param_count(&self) -> u32 {
        self.params.count()
    }
    /// Whether the parameter list is the `(void)` prototype.
    pub fn is_void_prototype(&self) -> bool {
        self.params.is_void_prototype()
    }
}

impl PaDeclaratorChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// A valid `main(void)` slice completes with `Ack` and appends nothing.
    /// Non-M1 shapes fail `Unsupported`; the ambiguous `()` shape fails with
    /// the DEFECT diagnostic.
    pub fn compute(&self, input: &PaDeclaratorInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("declarator task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.tokens.is_empty() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "declarator task {} carries no token refs",
                        input.task.index()
                    ),
                ),
            )];
        }
        let mut views = Vec::with_capacity(input.tokens.len());
        for id in &input.tokens {
            let Some(projected) = input.bodies.get(id) else {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("declarator reads missing token {}", id.index()),
                    ),
                )];
            };
            let spelling = match projected.kind {
                TokenKind::Identifier | TokenKind::Keyword => projected.spelling.clone(),
                _ => projected.pp_spelling.clone(),
            };
            if spelling.is_empty() {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("declarator reads token {} with no spelling", id.index()),
                    ),
                )];
            }
            views.push(DeclaratorToken {
                kind: projected.kind,
                spelling,
                name: projected.name,
            });
        }
        match parse_declarator(&views) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(error) => vec![fail(input.task, error.diagnostic())],
        }
    }
}

/// Parse an M1 declarator: coordinate the PA05 pointer and direct parts.
///
/// M1 accepts direct declarators only; a leading `*` is PA06 territory and
/// fails `Unsupported`. Correct `*`-vs-paren binding (`int *f(void)` vs
/// `int (*f)(void)`) is therefore deferred whole, never guessed.
pub fn parse_declarator(tokens: &[DeclaratorToken]) -> Result<DeclaratorTree, DeclaratorError> {
    let Some(first) = tokens.first() else {
        return Err(DeclaratorError::Unsupported(
            "empty declarator token slice; M1 requires `main(void)`",
        ));
    };
    if first.kind == TokenKind::Punctuator && first.spelling == b"*" {
        return Err(DeclaratorError::Unsupported(
            "pointer declarator is PA06 territory; M1 accepts direct declarators only",
        ));
    }
    parse_direct_declarator(tokens)
}

/// Parse an M1 direct declarator (PA07): identifier plus the suffix loop.
///
/// M1 accepts the identifier `main` followed by exactly one function suffix.
/// Parenthesized declarators, array suffixes, and trailing suffixes are
/// well-formed C deferred past M1 and fail `Unsupported`.
pub fn parse_direct_declarator(
    tokens: &[DeclaratorToken],
) -> Result<DeclaratorTree, DeclaratorError> {
    let Some(first) = tokens.first() else {
        return Err(DeclaratorError::Unsupported(
            "empty declarator token slice; M1 requires `main(void)`",
        ));
    };
    if first.kind == TokenKind::Punctuator && first.spelling == b"(" {
        return Err(DeclaratorError::Unsupported(
            "parenthesized declarator deferred past M1; M1 accepts `main(void)` only",
        ));
    }
    if first.kind != TokenKind::Identifier {
        return Err(DeclaratorError::Unsupported(
            "direct declarator must open with an identifier; M1 accepts `main(void)` only",
        ));
    }
    if first.spelling != b"main" {
        return Err(DeclaratorError::Unsupported(
            "non-M1 declarator name; M1 accepts `main` only",
        ));
    }
    let rest = &tokens[1..];
    if rest.is_empty() {
        return Err(DeclaratorError::Unsupported(
            "declarator has no suffix; M1 requires the `(void)` parameter list",
        ));
    }
    if rest[0].kind == TokenKind::Punctuator && rest[0].spelling == b"[" {
        return Err(DeclaratorError::Unsupported(
            "array suffix is PA08 territory; M1 accepts the `(void)` parameter list only",
        ));
    }
    if !(rest[0].kind == TokenKind::Punctuator && rest[0].spelling == b"(") {
        return Err(DeclaratorError::Unsupported(
            "declarator suffix must be a function parameter list; M1 requires `(void)`",
        ));
    }
    let Some(close) = rest
        .iter()
        .position(|token| token.kind == TokenKind::Punctuator && token.spelling == b")")
    else {
        return Err(DeclaratorError::Unsupported(
            "unterminated parameter list; M1 requires `(void)`",
        ));
    };
    if close != rest.len() - 1 {
        return Err(DeclaratorError::Unsupported(
            "trailing tokens after the parameter list; M1 accepts one suffix only",
        ));
    }
    let params = parse_parameter_list(&rest[..=close])?;
    Ok(DeclaratorTree {
        name: first.name,
        spelling: first.spelling.clone(),
        params,
    })
}

/// Parse an M1 function parameter list (PA09), parens included.
///
/// Accepts `(void)` (zero parameters, prototype) only. The empty list `()`
/// is the documented DEFECT ([`DeclaratorError::AmbiguousEmptyParams`]);
/// variadic, prototype, and old-style identifier parameters are explicit
/// `Unsupported`.
pub fn parse_parameter_list(suffix: &[DeclaratorToken]) -> Result<ParamList, DeclaratorError> {
    if suffix.len() < 2
        || suffix[0].kind != TokenKind::Punctuator
        || suffix[0].spelling != b"("
        || suffix[suffix.len() - 1].kind != TokenKind::Punctuator
        || suffix[suffix.len() - 1].spelling != b")"
    {
        return Err(DeclaratorError::Unsupported(
            "malformed parameter list; M1 requires `(void)`",
        ));
    }
    let inner = &suffix[1..suffix.len() - 1];
    if inner.is_empty() {
        return Err(DeclaratorError::AmbiguousEmptyParams);
    }
    if inner.len() == 1 && inner[0].kind == TokenKind::Keyword && inner[0].spelling == b"void" {
        return Ok(ParamList {
            is_void_prototype: true,
            count: 0,
        });
    }
    if inner
        .iter()
        .any(|token| token.kind == TokenKind::Punctuator && token.spelling == b"...")
    {
        return Err(DeclaratorError::Unsupported(
            "variadic parameter list deferred past M1; M1 accepts `(void)` only",
        ));
    }
    if inner.iter().any(|token| {
        token.kind == TokenKind::Punctuator && (token.spelling == b"[" || token.spelling == b"*")
    }) {
        return Err(DeclaratorError::Unsupported(
            "array/pointer parameters deferred past M1; M1 accepts `(void)` only",
        ));
    }
    if inner.iter().any(|token| {
        token.kind == TokenKind::Punctuator && (token.spelling == b"(" || token.spelling == b")")
    }) {
        return Err(DeclaratorError::Unsupported(
            "nested parameter lists deferred past M1; M1 accepts `(void)` only",
        ));
    }
    if inner.len() == 1 && inner[0].kind == TokenKind::Identifier {
        return Err(DeclaratorError::Unsupported(
            "old-style identifier parameter deferred past M1; M1 accepts `(void)` only",
        ));
    }
    Err(DeclaratorError::Unsupported(
        "non-void parameters deferred past M1; M1 accepts `(void)` only",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(spelling: &[u8]) -> DeclaratorToken {
        DeclaratorToken {
            kind: TokenKind::Identifier,
            spelling: spelling.to_vec(),
            name: Some(NameId::from_index(7)),
        }
    }

    fn keyword(spelling: &[u8]) -> DeclaratorToken {
        DeclaratorToken {
            kind: TokenKind::Keyword,
            spelling: spelling.to_vec(),
            name: None,
        }
    }

    fn punct(spelling: &[u8]) -> DeclaratorToken {
        DeclaratorToken {
            kind: TokenKind::Punctuator,
            spelling: spelling.to_vec(),
            name: None,
        }
    }

    fn m1_views() -> Vec<DeclaratorToken> {
        vec![ident(b"main"), punct(b"("), keyword(b"void"), punct(b")")]
    }

    fn input_of(views: &[DeclaratorToken]) -> PaDeclaratorInput {
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
                ProjectedToken {
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

    #[test]
    fn m1_declarator_validates_with_accessors() {
        let tree = parse_declarator(&m1_views()).expect("M1 `main(void)` validates");
        assert_eq!(tree.name_spelling(), b"main");
        assert_eq!(tree.declarator_name(), Some(NameId::from_index(7)));
        assert!(tree.is_zero_params());
        assert_eq!(tree.param_count(), 0);
        assert!(tree.is_void_prototype());
        assert!(tree.params().is_empty());
    }

    #[test]
    fn empty_parameter_list_is_defect() {
        let views = vec![ident(b"main"), punct(b"("), punct(b")")];
        assert_eq!(
            parse_declarator(&views),
            Err(DeclaratorError::AmbiguousEmptyParams)
        );
        let diagnostic = DeclaratorError::AmbiguousEmptyParams.diagnostic();
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }

    #[test]
    fn pointer_declarator_is_unsupported() {
        let views = vec![
            punct(b"*"),
            ident(b"f"),
            punct(b"("),
            keyword(b"void"),
            punct(b")"),
        ];
        assert!(matches!(
            parse_declarator(&views),
            Err(DeclaratorError::Unsupported(_))
        ));
    }

    #[test]
    fn non_void_params_are_unsupported() {
        for inner in [
            vec![keyword(b"int")],
            vec![ident(b"argc")],
            vec![punct(b"...")],
            vec![punct(b"["), punct(b"]")],
        ] {
            let mut views = vec![ident(b"main"), punct(b"(")];
            views.extend(inner);
            views.push(punct(b")"));
            assert!(matches!(
                parse_declarator(&views),
                Err(DeclaratorError::Unsupported(_))
            ));
        }
    }

    #[test]
    fn non_main_name_is_unsupported() {
        let views = vec![ident(b"other"), punct(b"("), keyword(b"void"), punct(b")")];
        assert!(matches!(
            parse_declarator(&views),
            Err(DeclaratorError::Unsupported(_))
        ));
    }

    #[test]
    fn compute_acks_m1_and_fails_loud() {
        let chip = PaDeclaratorChip;
        let ok = chip.compute(&input_of(&m1_views()));
        assert_eq!(
            ok,
            vec![Proposal::Complete {
                task: TaskId::from_index(0),
                value: ResultValue::Ack,
            }]
        );

        let ambiguous = vec![ident(b"main"), punct(b"("), punct(b")")];
        let ambiguous_input = input_of(&ambiguous);
        let ambiguous_out = chip.compute(&ambiguous_input);
        let [Proposal::Fail { diagnostic, .. }] = ambiguous_out.as_slice() else {
            panic!("ambiguous `()` must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);

        let mut other = vec![ident(b"main"), punct(b"(")];
        other.extend([keyword(b"int")]);
        other.push(punct(b")"));
        let other_input = input_of(&other);
        let other_out = chip.compute(&other_input);
        let [Proposal::Fail { diagnostic, .. }] = other_out.as_slice() else {
            panic!("non-void params must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn compute_rejects_non_running_task() {
        let chip = PaDeclaratorChip;
        let mut input = input_of(&m1_views());
        input.state = TaskState::Ready;
        let out = chip.compute(&input);
        let [Proposal::Fail { .. }] = out.as_slice() else {
            panic!("non-running task must fail");
        };
    }
}
