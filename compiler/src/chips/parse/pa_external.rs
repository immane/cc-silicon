// ============================================================================
// chips/parse/pa_external.rs — T05 PA02 external-declaration dispatch worker
// (Wave 3 slice 9, `/34`)
//
// Classifies one external declaration at the continuation cursor as either a
// function definition or a declaration. M1 exercises only the
// function-definition path (`int main(void){...}`): the specifier
// (`int`), the declarator (`main(void)`), and the next token (`{` vs
// `;`) decide. A `{` selects the function-definition path; a `;`
// selects the declaration path; any third token — including EOF, `=`,
// `,`, or an old-style (K&R) parameter name — is an explicit typed
// `Fail`, never a guessed default. The lookahead window, the declarator
// name, and every punctuator spelling reference committed records; a
// truncated window is explicit `Unsupported`.
//
// Pure classify, no writes: success completes `Ack` and the wiring layer
// (the PA01 external loop, integrator-owned) consumes the [`ExternalDeclKind`]
// returned by [`external_decl_kind`] to enqueue the PA03/PA05 child tasks.
// No child task is enqueued here, so no node-link or child-payload schema
// is invented.
//
// Frozen registration (integrator-owned): `PA02_TASK_KIND` aliases
// `TaskKind::PARSE_EXTERNAL_DECL` (`PARSE` local 17,
// `parse.external_declaration`), `PA02_CHIP` is
// `crate::manifest::PA02_CHIP` (`ChipId(44)`), the kind-registry row lives
// in `TaskKindRegistry::pa_decl_slice()`, the stage-2 row in
// `STAGE_ASSIGNMENT`, the routed layer is 2, there is no store-owner
// allowlist row (Ack-only, zero writes), and the acceptance test is
// `compiler/tests/c34_parse.rs`.
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ScopeId, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA02_CHIP};
use crate::task::{ParseContext, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Frozen task kind served by the PA02 worker.
///
/// `PARSE` local 17 — the first free local after frozen `PARSE_TU`
/// (local 16); registered `Frozen` in `TaskKindRegistry::pa_decl_slice()`.
pub const PA02_TASK_KIND: TaskKind = TaskKind::PARSE_EXTERNAL_DECL;

/// Committed tokens projected from the dispatch cursor, in source order.
///
/// Eight covers the whole M1 decision (`int main ( void ) {`, six tokens)
/// plus margin to observe a non-`{` discriminator without over-reading.
pub const PA_EXTERNAL_WINDOW: usize = 8;

/// Minimum window for a decision: specifier + declarator + discriminator.
pub const PA_EXTERNAL_MIN_WINDOW: usize = 6;

/// One projected token in the dispatch lookahead window.
#[derive(Clone, Debug)]
pub struct ExternalToken {
    /// Committed token ID.
    pub id: TokenId,
    /// C-token kind.
    pub kind: TokenKind,
    /// Resolved spelling: interned name bytes for `Keyword`/`Identifier`
    /// tokens, originating PP-token spelling otherwise (punctuators carry
    /// no name, so `{` vs `;` vs `(`/`)` resolves through the PP layer).
    pub spelling: Vec<u8>,
}

/// Narrow projection of the T05 PA02 parse request.
#[derive(Clone, Debug)]
pub struct PaExternalInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Registered parser task to resume (candidate PA02 kind).
    pub production: TaskKind,
    /// Committed token cursor where the external declaration starts.
    pub token_cursor: TokenId,
    /// Scope of the frame (`Some` file scope once File-Enter commits).
    pub scope: Option<ScopeId>,
    /// Grammar category at the saved cursor (must be `ExternalDecl`).
    pub context: ParseContext,
    /// Precedence level of the frame (carried, never consulted: external
    /// declarations are not precedence-climbing).
    pub binding_power: u16,
    /// Committed tokens from the cursor, in source order (bounded window).
    pub window: Vec<ExternalToken>,
}

/// External-declaration classification returned to the wiring layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalDeclKind {
    /// `int main(void){...}`: the discriminator is `{`.
    FunctionDefinition,
    /// Prototype or object declaration: the discriminator is `;`.
    Declaration,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pa_external_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaExternalInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("external-decl of unknown task {}", task.index()))
        })?;
    if record.kind != PA02_TASK_KIND {
        return Err(protocol_fault(format!(
            "external-decl task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    let continuation_id = record.continuation.ok_or_else(|| {
        protocol_fault(format!(
            "external-decl task {} carries no continuation",
            task.index()
        ))
    })?;
    let continuation = bus.arenas.continuations.get(continuation_id).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "external-decl reads missing continuation {}",
                continuation_id.index()
            ),
        )
    })?;
    if continuation.context != ParseContext::ExternalDecl {
        return Err(protocol_fault(format!(
            "external-decl task {} has unexpected context ordinal {}",
            task.index(),
            continuation.context.ordinal()
        )));
    }
    let tokens_allocated = bus.arenas.tokens.allocated();
    let mut window = Vec::new();
    for offset in 0..PA_EXTERNAL_WINDOW {
        let index = continuation.cursor.index().saturating_add(offset as u32);
        if index >= tokens_allocated {
            break;
        }
        let id = TokenId::from_index(index);
        let body = bus.arenas.tokens.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("external-decl reads missing token {}", id.index()),
            )
        })?;
        let spelling = match body.name {
            Some(name) => bus.intern.get(name).unwrap_or_default().to_vec(),
            None => bus
                .arenas
                .pp_tokens
                .get(body.pp_token)
                .map_err(|_| {
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!(
                            "external-decl reads missing pp-token {}",
                            body.pp_token.index()
                        ),
                    )
                })?
                .spelling
                .clone(),
        };
        window.push(ExternalToken {
            id,
            kind: body.kind,
            spelling,
        });
        if body.kind == TokenKind::Eof {
            break;
        }
    }
    Ok(PaExternalInput {
        task,
        state: record.state.clone(),
        production: continuation.production,
        token_cursor: continuation.cursor,
        scope: continuation.scope,
        context: continuation.context,
        binding_power: continuation.binding_power,
        window,
    })
}

/// The T05 external-declaration dispatch worker (PA02 candidate scope).
pub struct PaExternalChip;

impl Worker for PaExternalChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA02_CHIP,
            chip_name: "PaExternalChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![PA02_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Tasks, "active.continuation"),
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
        let input = match project_pa_external_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PaExternalChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// Success completes `Ack` with no bus writes: the wiring layer reads
    /// the decision through [`external_decl_kind`] and owns the PA03/PA05
    /// fan-out. Every failure path is a typed `Fail`.
    pub fn compute(&self, input: &PaExternalInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("external-decl task {} is not running", input.task.index()),
                ),
            )];
        }
        match external_decl_kind(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Classify one external declaration from the narrow projection.
///
/// Pure over the input: checks the M1 specifier (`int`), the M1 declarator
/// (`main(void)`), then the discriminator (`{` vs `;`). The identifier
/// gate keeps the chip M1-scoped; generalization awaits the full PA05
/// declarator. Old-style (K&R) parameter names, initializers, comma lists,
/// and every other non-M1 shape are explicit `Unsupported`. Ambiguity is a
/// `Fail` defect, never a guessed default: a third token in discriminator
/// position, or a window too short to reach the discriminator, fails loudly.
pub fn external_decl_kind(input: &PaExternalInput) -> Result<ExternalDeclKind, DiagnosticDraft> {
    if input.window.len() < PA_EXTERNAL_MIN_WINDOW {
        return Err(DiagnosticDraft::unsupported(
            "truncated external declaration: specifier, declarator, and `{`/`;` required",
        ));
    }
    check_specifier(input)?;
    check_declarator(input)?;
    check_discriminator(input)
}

/// Check the M1 specifier: exactly `int` (keyword).
fn check_specifier(input: &PaExternalInput) -> Result<(), DiagnosticDraft> {
    let token = &input.window[0];
    if token.kind != TokenKind::Keyword || token.spelling.as_slice() != b"int" {
        return Err(DiagnosticDraft::unsupported(
            "non-M1 declaration specifiers: exactly `int` required",
        ));
    }
    Ok(())
}

/// Check the M1 declarator: `main(void)` (identifier, parens, `void`).
fn check_declarator(input: &PaExternalInput) -> Result<(), DiagnosticDraft> {
    let expected: &[(TokenKind, &[u8])] = &[
        (TokenKind::Identifier, b"main".as_slice()),
        (TokenKind::Punctuator, b"(".as_slice()),
        (TokenKind::Keyword, b"void".as_slice()),
        (TokenKind::Punctuator, b")".as_slice()),
    ];
    for (offset, (kind, spelling)) in expected.iter().enumerate() {
        let token = &input.window[1 + offset];
        if token.kind != *kind || token.spelling.as_slice() != *spelling {
            return Err(DiagnosticDraft::unsupported(
                "non-M1 external declarator: exactly `main(void)` required",
            ));
        }
    }
    Ok(())
}

/// Check the discriminator: `{` selects the function-definition path and
/// `;` selects the declaration path; anything else is a loud defect.
fn check_discriminator(input: &PaExternalInput) -> Result<ExternalDeclKind, DiagnosticDraft> {
    let token = &input.window[5];
    if token.kind != TokenKind::Punctuator {
        return Err(DiagnosticDraft::unsupported(
            "ambiguous external declaration: `{` or `;` required after the declarator",
        ));
    }
    if token.spelling.as_slice() == b"{" {
        return Ok(ExternalDeclKind::FunctionDefinition);
    }
    if token.spelling.as_slice() == b";" {
        return Ok(ExternalDeclKind::Declaration);
    }
    Err(DiagnosticDraft::unsupported(
        "ambiguous external declaration: `{` or `;` required after the declarator",
    ))
}
