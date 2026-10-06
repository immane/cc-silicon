// ============================================================================
// chips/parse/pa_pod.rs — T05 PA14 DeclarationFinish worker (M1 slice)
//
// Frozen registration (integrator-owned): `PA14_TASK_KIND` aliases
//   `TaskKind::PARSE_DECL_FINISH` (`PARSE` local 25, `parse.decl_finish`),
//   `PA14_CHIP` is `crate::manifest::PA14_CHIP` (`ChipId(50)`), the
//   kind-registry row lives in `TaskKindRegistry::pa_recovery_slice()`,
//   the stage row is `(PARSE_DECL_FINISH, 2)`, the routed layer is 2,
//   and there is no store-owner allowlist row (Ack-only).
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA14 / DeclarationFinishChip / PARSE (frozen
//   `/36`: `PARSE` local 25, `PA14_CHIP = ChipId(50)`).
// Contract version/hash: T05 frozen; T01 `/36` current. This file claims
//   the PA14 `Declarators/initializers → DeclNodes` row only: comma
//   declarations, semicolons, and point-of-declaration (POD) name
//   registration requests.
// Owned files: ONLY this file (`compiler/src/chips/parse/pa_pod.rs`).
//   Never edits `mod.rs`, bus/task/ids/manifest schemas, or registrations
//   (see docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: CANDIDATE `parse.decl_finish`
//   (`PARSE` local 25, `PA14_TASK_KIND_CANDIDATE`), payload = the
//   declarator-finish token refs in source order (declarator slice plus the
//   terminator); result = `Ack` (no DeclNode carrier and no cursor carrier:
//   OB-30 open, caller holds the cursor; the DeclNode append and the T06
//   `symbol_type.declare` fan-out stay wiring-layer owned).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (exactly `main(void);` — one
//   declarator, no initializer, `;`-terminated); every other shape is an
//   explicit typed `Fail` (never a pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` (`/11`), `pp.tokens`, `names.entries` (`/11`). Pure query:
//   no bus mutation; workers emit `Proposal`s only.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no `Enqueue`.
//   The [`PodRegistrationShape`] below copies the POD request field shapes
//   chip-locally for documentation/future split only; it is NOT committed
//   and no T06 task is enqueued (a second writer or a cross-group enqueue
//   would need allowlist/registration rows this file must not invent).
// Dependencies: T04 committed tokens + interned spellings (producer facts
//   only; never calls another chip). Hands the certified registration to
//   the wiring layer, which owns the T06 declare fan-out and the PA15
//   initializer ordering below.
// Preconditions: dispatched task is `Running`; payload carries `Token` refs
//   only; every projected token is committed.
// Transition states: Running → Complete(Ack) on the M1 single-declaration
//   finish; Running → Fail (`Unsupported` group) on comma/initializer /
//   non-M1 shapes; Running → Fail (`Task` group code 4) on
//   not-running/missing-token/unterminated defects.
// Algorithm obligations: accept exactly `main(void);`; locate the
//   terminator (`;` vs `,` vs `=`) at paren depth 0; certify the POD name
//   and span for the wiring layer; never derive typedef-ness or type
//   compatibility locally (PA04/T06 own those queries).
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = project-then-compute; all paths yield proposals.
// Error codes / recovery: `Unsupported/1` for non-M1 shapes (explicit
//   rejection, stays in the denominator); `Task/4` for protocol and
//   unterminated-declaration defects; `Protocol/1` for
//   unknown-task/wrong-kind scaffolding.
// Required fixtures: `compiler/tests/c36_recovery.rs` (recovery slice;
//   wiring integrator-owned — this Ack-only file ships in-file unit tests
//   plus the shared acceptance suite).
// Integration acceptance: FROZEN `/36` (kind registration in
//   `TaskKindRegistry::pa_recovery_slice()`, stage-2 row, routed layer 2,
//   no allowlist row — Ack-only; DeclNode carrier, parent linkage open).
// Known unsupported: comma-separated declarator lists, initializers
//   (PA15/T08 territory), pointer/parenthesized/array declarators,
//   non-`main` names, non-`(void)` parameters, storage-class/qualifier
//   combinations (all deferred).
//
// Candidate registration (frozen `/36` by the integrator): `PA14_TASK_KIND`
//   is `PARSE` local 25 (`parse.decl_finish`) — the first free local
//   after `PARSE_UNARY` (local 24); `PA14_CHIP` is `ChipId(50)`, the
//   next free ID after `PA20_CHIP` (`ChipId(49)`). Rows live in
//   `TaskKindRegistry::pa_recovery_slice()`, `STAGE_ASSIGNMENT` (stage 2),
//   and the routed layer 2. The constants below alias the frozen
//   canonicals so the manifest and tests agree without touching
//   `task.rs`/`manifest.rs`. Numeric diagnostic codes below are frozen
//   per the T01 `/6` code table.
//
// POD ordering invariant (the PA14 semantic that must survive the split):
//   the point of declaration fires immediately after the declarator and
//   BEFORE any initializer is evaluated, so `int x = x;` reads the NEW `x`
//   (indeterminate), never an outer shadowed `x` and never "undeclared".
//   The wiring layer must therefore enqueue the T06 declare request before
//   the PA15 initializer child task. This Ack-only slice certifies the name
//   and span and documents the order; it enqueues nothing itself.
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-1 (terminator): a slice with no `;` is unterminated — a `Task/4`
//     defect, never read as an implicit semicolon.
//   DEFECT-2 (self-visibility): any `=` keeps the slice in PA15 territory
//     and fails `Unsupported`; the initializer RHS must observe the POD
//     registration above, which only the wiring layer can sequence.
//   DEFECT-3 (cursor/nodes): `next_cursor` and the DeclNode append have no
//     frozen carrier (T05 item G.4 OB-30 open). M1 completes `Ack`; cursor
//     advance and node linkage stay caller-held. No scalar/tuple variant is
//     invented here.
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, NameId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA14_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Frozen task kind served by the PA14 worker.
///
/// Aliases `TaskKind::PARSE_DECL_FINISH` (`PARSE` local 25,
/// `parse.decl_finish`).
pub const PA14_TASK_KIND: TaskKind = TaskKind::PARSE_DECL_FINISH;

/// Candidate local code for the PA14 declaration-finish task.
///
/// `PARSE` local 25 — the first free local after `PARSE_UNARY` (local 24).
/// Frozen `/36`; kept as the documented next-free value.
pub const PA14_CANDIDATE_LOCAL: u16 = 25;

/// Candidate chip ID for the PA14 worker.
///
/// The next free ID after `PA20_CHIP` (`ChipId(49)`). Frozen `/36` as
/// `PA14_CHIP`.
pub const PA14_CANDIDATE_CHIP: ChipId = ChipId(50);

/// Candidate task kind served by the PA14 worker.
///
/// Resolved from [`PA14_CANDIDATE_LOCAL`]; `None` only if the local exceeds
/// `LOCAL_MAX` (impossible for 25 — the `expect` in [`pa14_task_kind`] is
/// unreachable and loud rather than silent).
pub const PA14_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA14_CANDIDATE_LOCAL);

/// Resolve the task kind (the frozen `PA14_TASK_KIND` canonical).
pub fn pa14_task_kind() -> TaskKind {
    PA14_TASK_KIND
}

/// One projected token: kind plus resolved spellings.
///
/// Identifier/keyword spellings resolve through the intern table (`spelling`);
/// punctuator spellings resolve through the committed PP token
/// (`pp_spelling`), because committed C punctuator records carry no interned
/// name. Either side is empty only when the backing record is missing, which
/// the computation reports loudly instead of guessing.
#[derive(Clone, Debug)]
pub struct ProjectedPodToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Interned spelling bytes for named tokens.
    pub spelling: Vec<u8>,
    /// Committed PP spelling bytes (punctuator ground truth).
    pub pp_spelling: Vec<u8>,
}

/// Narrow projection for the declaration-finish computation.
#[derive(Clone, Debug)]
pub struct PaPodInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (declarator slice plus terminator).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedPodToken>,
}

/// Build the narrow projection for one dispatched task.
///
/// Payload convention (candidate): the declarator-finish token refs in
/// source order — exactly `[main, (, void, ), ;]` for M1. The payload
/// carries no specifier tokens (PA03 territory) and no EOF. A wrong kind,
/// an empty payload, or a non-token ref is a caller protocol fault; a
/// missing committed record is a task error surfaced by the computation.
pub fn project_pa_pod_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaPodInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("decl-finish of unknown task {}", task.index())))?;
    if record.kind != pa14_task_kind() {
        return Err(protocol_fault(format!(
            "decl-finish task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "decl-finish task {} payload must carry the declarator-finish token refs",
            task.index()
        )));
    }
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "decl-finish task {} payload must carry tokens only",
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
                ProjectedPodToken {
                    kind: body.kind,
                    name: body.name,
                    spelling,
                    pp_spelling,
                },
            );
        }
    }
    Ok(PaPodInput {
        task,
        state: record.state.clone(),
        tokens,
        bodies,
    })
}

/// The T05 PA14 declaration-finish worker (M1 slice).
pub struct PaPodChip;

impl Worker for PaPodChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA14_CHIP,
            chip_name: "PaPodChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![pa14_task_kind()],
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
            tests: vec!["compiler/tests/c36_recovery.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pa_pod_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// One declarator-finish token for the pure grammar core.
///
/// `spelling` is the effective spelling: intern bytes for identifiers and
/// keywords, committed PP bytes for punctuators. The projector resolves
/// which side applies; the pure core only compares bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PodToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Effective spelling bytes.
    pub spelling: Vec<u8>,
    /// Interned name, if any (carried for the POD-name accessor).
    pub name: Option<NameId>,
}

/// Why a token slice is not the M1 declaration finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PodError {
    /// A well-formed (or malformed) shape outside the M1 subset: comma
    /// lists, initializers, pointer/parenthesized/array declarators,
    /// non-`main` names, non-`(void)` parameters. Maps to `Unsupported`.
    Unsupported(&'static str),
    /// The slice has no `;` terminator: an unterminated declaration, never
    /// an implicit semicolon. Maps to the `Task/4` defect diagnostic.
    Unterminated,
}

impl PodError {
    /// Map to the structured failure diagnostic for this rejection.
    ///
    /// Numeric codes follow the frozen T01 `/6` code table.
    pub fn diagnostic(&self) -> DiagnosticDraft {
        match self {
            Self::Unsupported(message) => DiagnosticDraft::unsupported(*message),
            Self::Unterminated => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "DEFECT: unterminated declaration; M1 requires a `;` terminator",
            ),
        }
    }
}

/// Certified point-of-declaration registration request (chip-local shape).
///
/// Same conceptual fields as the future T06 `symbol_type.declare` request
/// (declarator name, name spelling, declarator token span) so the wiring
/// layer can lift it verbatim; positions are indices into the task's own
/// token slice. This shape is built by the pure path and documented — it is
/// NOT committed and no T06 task is enqueued by this Ack-only slice. The
/// scope and type payloads of the future declare request stay caller-held
/// (PA03 validated the specifiers; the file scope arrives via File-Enter).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PodRegistrationShape {
    /// Interned declarator name, if the token carried one (`main` in M1).
    pub name: Option<NameId>,
    /// Declarator name spelling bytes (`main` in M1).
    pub spelling: Vec<u8>,
    /// Position of the declarator-name token in the task's token slice.
    pub name_pos: usize,
    /// Position of the first declarator token in the task's token slice.
    pub first_pos: usize,
    /// Position of the last declarator token (before `;`) in the slice.
    pub last_pos: usize,
}

impl PodRegistrationShape {
    /// Interned declarator name, if the token carried one.
    pub fn pod_name(&self) -> Option<NameId> {
        self.name
    }
    /// Declarator name spelling bytes.
    pub fn name_spelling(&self) -> &[u8] {
        &self.spelling
    }
    /// Number of declarator tokens covered (excluding the `;`).
    pub fn declarator_len(&self) -> usize {
        self.last_pos
            .saturating_sub(self.first_pos)
            .saturating_add(1)
    }
}

/// Validated M1 declaration finish: one declarator, `;`-terminated, with
/// its POD registration certified for the wiring layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PodFinish {
    /// The single POD registration request of this declaration.
    registration: PodRegistrationShape,
}

impl PodFinish {
    /// The certified point-of-declaration registration request.
    pub fn registration(&self) -> &PodRegistrationShape {
        &self.registration
    }
}

impl PaPodChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// A valid `main(void);` finish completes with `Ack` and appends
    /// nothing. Comma/initializer/non-M1 shapes fail `Unsupported`; a
    /// missing `;` fails with the unterminated-declaration defect.
    pub fn compute(&self, input: &PaPodInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("decl-finish task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.tokens.is_empty() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "decl-finish task {} carries no token refs",
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
                        format!("decl-finish reads missing token {}", id.index()),
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
                        format!("decl-finish reads token {} with no spelling", id.index()),
                    ),
                )];
            }
            views.push(PodToken {
                kind: projected.kind,
                spelling,
                name: projected.name,
            });
        }
        match parse_declaration_finish(&views) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(error) => vec![fail(input.task, error.diagnostic())],
        }
    }
}

/// Parse an M1 declaration finish: one declarator, `;`-terminated.
///
/// M1 accepts `main(void);` only: the identifier `main`, exactly the
/// `(void)` suffix, then `;`. The terminator scan runs at paren depth 0 —
/// a `,` selects the deferred multi-declaration path, an `=` selects the
/// deferred initializer path (PA15 territory, where the POD-before-
/// initializer ordering invariant applies), and a missing `;` is the
/// unterminated-declaration defect. Pointer/parenthesized/array declarators
/// and non-`void` parameters are well-formed C deferred past M1.
pub fn parse_declaration_finish(tokens: &[PodToken]) -> Result<PodFinish, PodError> {
    if tokens.len() < 5 {
        return Err(PodError::Unsupported(
            "truncated declaration finish: declarator plus `;` required",
        ));
    }
    let last = tokens
        .last()
        .expect("length-checked slice has a terminator position");
    if last.kind != TokenKind::Punctuator {
        return Err(PodError::Unterminated);
    }
    if last.spelling == b"," {
        return Err(PodError::Unsupported(
            "comma-separated declarator list deferred past M1; one declarator per declaration",
        ));
    }
    if last.spelling == b"=" {
        return Err(PodError::Unsupported(
            "initializer deferred to PA15; the initializer RHS must observe the POD registration first (`int x = x;` reads the new `x`)",
        ));
    }
    if last.spelling != b";" {
        return Err(PodError::Unterminated);
    }
    let declarator = &tokens[..tokens.len() - 1];
    // An interior `,` or `=` at depth 0 means the payload holds more than
    // one declarator or an initializer — both deferred, never split here.
    let mut depth: u32 = 0;
    for token in declarator {
        if token.kind == TokenKind::Punctuator {
            match token.spelling.as_slice() {
                b"(" => depth = depth.saturating_add(1),
                b")" => depth = depth.saturating_sub(1),
                b"," if depth == 0 => {
                    return Err(PodError::Unsupported(
                        "comma-separated declarator list deferred past M1; one declarator per declaration",
                    ));
                }
                b"=" if depth == 0 => {
                    return Err(PodError::Unsupported(
                        "initializer deferred to PA15; the initializer RHS must observe the POD registration first (`int x = x;` reads the new `x`)",
                    ));
                }
                _ => {}
            }
        }
    }
    parse_finish_declarator(declarator)
}

/// Validate the single M1 declarator inside a `;`-terminated finish.
///
/// Accepts `main(void)` only. Deep declarator validation stays with PA05;
/// this check locates the POD name and span so the registration request is
/// certified, and rejects every non-M1 declarator loudly rather than
/// registering a guessed name.
fn parse_finish_declarator(declarator: &[PodToken]) -> Result<PodFinish, PodError> {
    let Some(first) = declarator.first() else {
        return Err(PodError::Unsupported(
            "empty declarator; M1 requires `main(void)` before `;`",
        ));
    };
    if first.kind == TokenKind::Punctuator && first.spelling == b"*" {
        return Err(PodError::Unsupported(
            "pointer declarator is PA06 territory; M1 accepts `main(void)` only",
        ));
    }
    if first.kind == TokenKind::Punctuator && first.spelling == b"(" {
        return Err(PodError::Unsupported(
            "parenthesized declarator deferred past M1; M1 accepts `main(void)` only",
        ));
    }
    if first.kind != TokenKind::Identifier {
        return Err(PodError::Unsupported(
            "declarator must open with an identifier; M1 accepts `main(void)` only",
        ));
    }
    if first.spelling != b"main" {
        return Err(PodError::Unsupported(
            "non-M1 declarator name; M1 accepts `main` only",
        ));
    }
    let rest = &declarator[1..];
    if rest.is_empty() {
        return Err(PodError::Unsupported(
            "declarator has no suffix; M1 requires the `(void)` parameter list",
        ));
    }
    if rest[0].kind == TokenKind::Punctuator && rest[0].spelling == b"[" {
        return Err(PodError::Unsupported(
            "array suffix is PA08 territory; M1 accepts the `(void)` parameter list only",
        ));
    }
    if !(rest.len() == 3
        && rest[0].kind == TokenKind::Punctuator
        && rest[0].spelling == b"("
        && rest[1].kind == TokenKind::Keyword
        && rest[1].spelling == b"void"
        && rest[2].kind == TokenKind::Punctuator
        && rest[2].spelling == b")")
    {
        return Err(PodError::Unsupported(
            "non-`(void)` parameter list deferred past M1; M1 accepts `(void)` only",
        ));
    }
    Ok(PodFinish {
        registration: PodRegistrationShape {
            name: first.name,
            spelling: first.spelling.clone(),
            name_pos: 0,
            first_pos: 0,
            last_pos: declarator.len() - 1,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(spelling: &[u8]) -> PodToken {
        PodToken {
            kind: TokenKind::Identifier,
            spelling: spelling.to_vec(),
            name: Some(NameId::from_index(7)),
        }
    }

    fn keyword(spelling: &[u8]) -> PodToken {
        PodToken {
            kind: TokenKind::Keyword,
            spelling: spelling.to_vec(),
            name: None,
        }
    }

    fn punct(spelling: &[u8]) -> PodToken {
        PodToken {
            kind: TokenKind::Punctuator,
            spelling: spelling.to_vec(),
            name: None,
        }
    }

    fn m1_views() -> Vec<PodToken> {
        vec![
            ident(b"main"),
            punct(b"("),
            keyword(b"void"),
            punct(b")"),
            punct(b";"),
        ]
    }

    fn input_of(views: &[PodToken]) -> PaPodInput {
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

    #[test]
    fn m1_finish_validates_with_registration() {
        let finish = parse_declaration_finish(&m1_views()).expect("M1 `main(void);` finishes");
        let registration = finish.registration();
        assert_eq!(registration.name_spelling(), b"main");
        assert_eq!(registration.pod_name(), Some(NameId::from_index(7)));
        assert_eq!(registration.name_pos, 0);
        assert_eq!(registration.first_pos, 0);
        assert_eq!(registration.last_pos, 3);
        assert_eq!(registration.declarator_len(), 4);
    }

    #[test]
    fn comma_list_is_unsupported() {
        let views = vec![
            ident(b"main"),
            punct(b"("),
            keyword(b"void"),
            punct(b")"),
            punct(b","),
        ];
        assert!(matches!(
            parse_declaration_finish(&views),
            Err(PodError::Unsupported(_))
        ));
        let diagnostic = PodError::Unsupported("probe").diagnostic();
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn initializer_is_unsupported_with_pod_ordering_note() {
        // `main(void) = ...;` and an interior `=` both stay PA15 territory:
        // the initializer RHS must observe the POD registration first.
        let trailing = vec![
            ident(b"main"),
            punct(b"("),
            keyword(b"void"),
            punct(b")"),
            punct(b"="),
        ];
        let interior = vec![
            ident(b"main"),
            punct(b"="),
            ident(b"main"),
            punct(b"("),
            punct(b")"),
            punct(b";"),
        ];
        for views in [trailing, interior] {
            let error = parse_declaration_finish(&views).expect_err("initializer defers to PA15");
            assert!(matches!(error, PodError::Unsupported(_)));
            assert_eq!(error.diagnostic().code.group, DiagGroup::Unsupported);
        }
    }

    #[test]
    fn missing_semicolon_is_defect() {
        let views = vec![
            ident(b"main"),
            punct(b"("),
            keyword(b"void"),
            punct(b")"),
            punct(b"}"),
        ];
        assert_eq!(
            parse_declaration_finish(&views),
            Err(PodError::Unterminated)
        );
        let diagnostic = PodError::Unterminated.diagnostic();
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }

    #[test]
    fn non_m1_declarators_are_unsupported() {
        let pointer = vec![
            punct(b"*"),
            ident(b"f"),
            punct(b"("),
            keyword(b"void"),
            punct(b")"),
            punct(b";"),
        ];
        let other_name = vec![
            ident(b"other"),
            punct(b"("),
            keyword(b"void"),
            punct(b")"),
            punct(b";"),
        ];
        let non_void = vec![
            ident(b"main"),
            punct(b"("),
            keyword(b"int"),
            punct(b")"),
            punct(b";"),
        ];
        for views in [pointer, other_name, non_void] {
            assert!(matches!(
                parse_declaration_finish(&views),
                Err(PodError::Unsupported(_))
            ));
        }
    }

    #[test]
    fn compute_acks_m1_and_fails_loud() {
        let chip = PaPodChip;
        let ok = chip.compute(&input_of(&m1_views()));
        assert_eq!(
            ok,
            vec![Proposal::Complete {
                task: TaskId::from_index(0),
                value: ResultValue::Ack,
            }]
        );

        let comma = vec![
            ident(b"main"),
            punct(b"("),
            keyword(b"void"),
            punct(b")"),
            punct(b","),
        ];
        let comma_out = chip.compute(&input_of(&comma));
        let [Proposal::Fail { diagnostic, .. }] = comma_out.as_slice() else {
            panic!("comma list must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);

        let unterminated = vec![
            ident(b"main"),
            punct(b"("),
            keyword(b"void"),
            punct(b")"),
            punct(b"}"),
        ];
        let unterminated_out = chip.compute(&input_of(&unterminated));
        let [Proposal::Fail { diagnostic, .. }] = unterminated_out.as_slice() else {
            panic!("missing `;` must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }

    #[test]
    fn compute_rejects_non_running_task() {
        let chip = PaPodChip;
        let mut input = input_of(&m1_views());
        input.state = TaskState::Ready;
        let out = chip.compute(&input);
        let [Proposal::Fail { .. }] = out.as_slice() else {
            panic!("non-running task must fail");
        };
    }

    #[test]
    fn candidate_registration_is_next_free() {
        assert_eq!(PA14_CANDIDATE_LOCAL, 25);
        assert_eq!(PA14_CANDIDATE_CHIP, ChipId(50));
        assert_eq!(PA14_CHIP, ChipId(50));
        assert_eq!(PA14_TASK_KIND, TaskKind::PARSE_DECL_FINISH);
        let kind = pa14_task_kind();
        assert_eq!(kind.group(), TaskGroup::PARSE);
        assert_eq!(kind.local(), 25);
    }
}
