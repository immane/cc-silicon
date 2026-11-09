// ============================================================================
// chips/parse/pa_stmt.rs — T05 PA27/PA29/PA30/PA31 statement workers (M1).
//
// DRAFT / CANDIDATE-ONLY (Wave 4). No frozen registration: the `PARSE`
// local codes, the chip ID, the kind-registry rows, the stage rows, and any
// store-owner allowlist rows are integrator-owned and live in
// `task.rs`/`manifest.rs`, not here. This file defines CANDIDATE constants
// only and must not be read as a freeze.
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA27 StatementDispatchChip + PA29 IfStatementChip +
//   PA30 SwitchStatementChip + PA31 LoopStatementChip / PARSE (CANDIDATE).
//   Single-chip file: one zero-sized unit struct, `PaStmtChip`, serves all
//   four productions and dispatches on the production field (the dispatched
//   task's `TaskKind`, mirroring `ContinuationRecord.production`).
// Contract version/hash: T05 PENDING; T01 `/5` current. This file claims the
//   PA27/PA29/PA30/PA31 rows only.
// Owned files: ONLY this file (`compiler/src/chips/parse/pa_stmt.rs`).
//   Never edits `mod.rs`, bus/task/ids/manifest schemas, or registrations
//   (see docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: CANDIDATE `PARSE` locals 27–30
//   (`PA27_CANDIDATE_LOCAL` … `PA31_CANDIDATE_LOCAL`), payload = committed
//   `RecordRef::Token`s in source order; result = `Ack` (no next-cursor
//   carrier: OB-30 open, caller holds the cursor).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only. PA27 accepts exactly the two M1
//   statement shapes also owned by the PA28/PA32 slice — the 7-token block
//   `{ return <int> + <int> ; }` and the 5-token return
//   `return <int> + <int> ;` — and completes `Ack`. PA29/PA30/PA31 accept
//   nothing in M1: every input fails `Unsupported` (explicit rejection,
//   stays in the denominator; never a pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` + `lex.literals` (`/11`), `names.entries` (`/11`). Pure
//   query: no bus mutation; workers emit `Proposal`s only.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no
//   `Enqueue`. Committed statement nodes stay owned by the future full
//   catalog split; a second `parse.nodes` writer would need an allowlist
//   row this file must not invent. Scope enter/exit is T06-owned. The
//   `LocalStmtShape` rows below copy the frozen `NodeRecord` field shapes
//   chip-locally for documentation/future split only; they are NOT
//   committed.
// Dependencies: T04 committed tokens + interned spellings + decoded
//   literals (producer facts only; never calls another chip).
// Preconditions: dispatched task is `Running` with a frozen-candidate
//   PA27/PA29/PA30/PA31 kind; payload carries `Token` refs only;
//   referenced tokens and integer literals are committed.
// Transition states: Running → Complete(Ack) on the exact M1 shape (PA27
//   only); Running → Fail (`Unsupported` group) on any other shape
//   (including ALL PA29/PA30/PA31 inputs); Running → Fail (`Task` group
//   code 4) on not-running/missing-record protocol faults.
// Algorithm obligations: PA27 dispatches on shape — 7 tokens with leading
//   `{` is checked against the M1 block form, 5 tokens with leading
//   `return` against the M1 return form, anything else is `Unsupported`.
//   PA29/PA30/PA31 never accept: after the `Running` + token-presence
//   protocol checks they fail `Unsupported` unconditionally (no keyword
//   sniffing, no partial `if`/`switch`/`for` acceptance).
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = dispatch-then-project-then-compute; all paths
//   yield proposals.
// Error codes / recovery: `Unsupported/1` for non-M1 shapes and for every
//   PA29/PA30/PA31 input (explicit rejection, stays in the denominator);
//   `Task/4` for protocol faults; `Protocol/1` for unknown-task/wrong-kind
//   scaffolding. No recovery sync is attempted here (PA38 owns it).
// Required fixtures: in-file unit tests below (pure path, no bus); the
//   shared acceptance suite is integrator-owned.
// Integration acceptance: OPEN (candidate locals/chip/stage rows, cursor
//   carrier, parent linkage, child-task nesting all pending T05/T01
//   co-freeze).
// Known unsupported: `if`/`else` (incl. dangling-else), `switch`/`case`/
//   `default`, `for`/`while`/`do-while` (incl. declaration-in-`for` and the
//   `do…while;` trailing semicolon), labels, expression statements, empty
//   `;`, `break`/`continue`/`goto`, GNU statement extensions (all deferred;
//   all fail `Unsupported`).
//
// Candidate registration (integrator-owned, NOT frozen here):
//   `PA27_CANDIDATE_LOCAL` = 27, `PA29_CANDIDATE_LOCAL` = 28,
//   `PA30_CANDIDATE_LOCAL` = 29, `PA31_CANDIDATE_LOCAL` = 30 — the first
//   four free `PARSE` locals after `PARSE_RECOVERY` (local 26); and
//   `PA_STMT_CANDIDATE_CHIP` = `ChipId(58)`, the next free chip ID after
//   `SE_FUNC_CHIP` (`ChipId(57)`). Kind-registry rows, stage-2 rows, routed
//   layer 2, and the (empty, Ack-only) allowlist rows all stay
//   integrator-owned.
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-1 (dispatch vs ownership): PA27 certifies the M1 statement shape
//     only. The committed statement/block node append and any child-task
//     fan-out (PA28/PA32) stay wiring-layer owned; this Ack-only slice
//     enqueues nothing and appends nothing.
//   DEFECT-2 (nesting): the full design nests statements/expressions via
//     child tasks and continuations with await-all joins (T05 items A–C);
//     M1 validates the fixed shape in one tick with no `Enqueue`,
//     `AwaitChildren`, or `Progress`. Parse-depth accounting
//     (`ParseDepthExceeded`, item D) stays a future integration check.
//   DEFECT-3 (scope): block scope enter/exit is T06-owned. This chip writes
//     nothing, so the "scope balance on error recovery" acceptance is
//     T06-integration work, documented here and deferred there.
//   DEFECT-4 (ambiguity): any input outside the two exact M1 shapes fails
//     `Unsupported` instead of guessing (e.g. `if (1) return 2+3;` is not
//     partially accepted, `;` alone is not an expression statement — PA34
//     owns that call).
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, LiteralId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Candidate `PARSE` local for PA27 statement dispatch.
///
/// First free local after `PARSE_RECOVERY` (local 26). Candidate only;
/// the frozen row is integrator-owned.
pub const PA27_CANDIDATE_LOCAL: u16 = 27;

/// Candidate `PARSE` local for PA29 if-statement parsing (M1: always
/// `Unsupported`). Candidate only; the frozen row is integrator-owned.
pub const PA29_CANDIDATE_LOCAL: u16 = 28;

/// Candidate `PARSE` local for PA30 switch-statement parsing (M1: always
/// `Unsupported`). Candidate only; the frozen row is integrator-owned.
pub const PA30_CANDIDATE_LOCAL: u16 = 29;

/// Candidate `PARSE` local for PA31 loop-statement parsing (M1: always
/// `Unsupported`). Candidate only; the frozen row is integrator-owned.
pub const PA31_CANDIDATE_LOCAL: u16 = 30;

/// Candidate chip ID for the fused PA27/PA29/PA30/PA31 worker.
///
/// Next free ID after `SE_FUNC_CHIP` (`ChipId(57)`). Candidate only; the
/// frozen manifest row is integrator-owned.
pub const PA_STMT_CANDIDATE_CHIP: ChipId = ChipId(58);

/// Candidate task kind for PA27 statement dispatch.
pub const PA27_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA27_CANDIDATE_LOCAL);

/// Candidate task kind for PA29 if-statement parsing.
pub const PA29_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA29_CANDIDATE_LOCAL);

/// Candidate task kind for PA30 switch-statement parsing.
pub const PA30_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA30_CANDIDATE_LOCAL);

/// Candidate task kind for PA31 loop-statement parsing.
pub const PA31_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA31_CANDIDATE_LOCAL);

/// Resolve the PA27 candidate kind (`expect` is unreachable: local 27 is
/// far below `LOCAL_MAX`).
pub fn pa27_task_kind() -> TaskKind {
    PA27_TASK_KIND_CANDIDATE.expect("PA27 candidate local must resolve")
}

/// Resolve the PA29 candidate kind.
pub fn pa29_task_kind() -> TaskKind {
    PA29_TASK_KIND_CANDIDATE.expect("PA29 candidate local must resolve")
}

/// Resolve the PA30 candidate kind.
pub fn pa30_task_kind() -> TaskKind {
    PA30_TASK_KIND_CANDIDATE.expect("PA30 candidate local must resolve")
}

/// Resolve the PA31 candidate kind.
pub fn pa31_task_kind() -> TaskKind {
    PA31_TASK_KIND_CANDIDATE.expect("PA31 candidate local must resolve")
}

/// Production served by [`PaStmtChip`]: the dispatch target resolved from
/// the dispatched task's kind (the production field).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaStmtProduction {
    /// PA27 statement dispatch (M1: block or return shape only).
    Statement,
    /// PA29 if statement (M1: always `Unsupported`).
    If,
    /// PA30 switch statement (M1: always `Unsupported`).
    Switch,
    /// PA31 loop statement (M1: always `Unsupported`).
    Loop,
}

impl PaStmtProduction {
    /// Resolve the production for a dispatched task kind.
    pub fn of_kind(kind: TaskKind) -> Option<Self> {
        if Some(kind) == PA27_TASK_KIND_CANDIDATE {
            Some(Self::Statement)
        } else if Some(kind) == PA29_TASK_KIND_CANDIDATE {
            Some(Self::If)
        } else if Some(kind) == PA30_TASK_KIND_CANDIDATE {
            Some(Self::Switch)
        } else if Some(kind) == PA31_TASK_KIND_CANDIDATE {
            Some(Self::Loop)
        } else {
            None
        }
    }
}

/// Chip-local copy of the frozen `NodeRecord` field shapes for one M1
/// statement subtree node.
///
/// Same field names/order as `NodeRecord` (`parent`, `first_token`,
/// `last_token`, literal presence) so the future split can lift it
/// verbatim, except positions are indices into the task's own token slice
/// and links are pre-order node indices rather than committed IDs; the kind
/// is the `NodeKind` variant name string rather than imported shared
/// schema. Shapes are built by the pure path and documented — they are NOT
/// appended to any arena by this Ack-only slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalStmtShape {
    /// Fixed node-kind name (mirrors `NodeKind`, e.g. `"Compound"`).
    pub kind_name: &'static str,
    /// Pre-order index of the parent node (`None` for the subtree root;
    /// owner integration assigns the committed parent).
    pub parent: Option<usize>,
    /// Position of the first covered token in the task's token slice.
    pub first_token: usize,
    /// Position of the last covered token (inclusive).
    pub last_token: usize,
    /// Whether the node is an integer leaf backed by a committed literal.
    pub has_literal: bool,
}

/// Expected M1 block token kinds in source order:
///
/// `{` `return` `2` `+` `3` `;` `}`.
const M1_BLOCK_KINDS: &[TokenKind] = &[
    TokenKind::Punctuator,
    TokenKind::Keyword,
    TokenKind::Integer,
    TokenKind::Punctuator,
    TokenKind::Integer,
    TokenKind::Punctuator,
    TokenKind::Punctuator,
];

/// Expected M1 return token kinds in source order:
///
/// `return` `2` `+` `3` `;`.
const M1_RETURN_KINDS: &[TokenKind] = &[
    TokenKind::Keyword,
    TokenKind::Integer,
    TokenKind::Punctuator,
    TokenKind::Integer,
    TokenKind::Punctuator,
];

/// One projected token: kind plus resolved spelling for named tokens.
#[derive(Clone, Debug)]
pub struct ProjectedStmtToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Spelling bytes for `Identifier`/`Keyword` tokens.
    pub spelling: Vec<u8>,
}

/// Narrow projection for the PA27 statement-dispatch computation.
#[derive(Clone, Debug)]
pub struct PaStmtInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (M1: exactly 7 or exactly 5).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedStmtToken>,
    /// Committed literal by token ID (exactly one per integer token).
    pub literals_by_token: BTreeMap<TokenId, LiteralId>,
}

/// Narrow projection for the PA29 if-statement computation (M1: always
/// `Unsupported` after protocol checks).
#[derive(Clone, Debug)]
pub struct PaIfInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order.
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedStmtToken>,
}

/// Narrow projection for the PA30 switch-statement computation (M1: always
/// `Unsupported` after protocol checks).
#[derive(Clone, Debug)]
pub struct PaSwitchInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order.
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedStmtToken>,
}

/// Narrow projection for the PA31 loop-statement computation (M1: always
/// `Unsupported` after protocol checks).
#[derive(Clone, Debug)]
pub struct PaLoopInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order.
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedStmtToken>,
}

/// Projected token stream for one statement task: token IDs in source
/// order, present token bodies, committed literals by token, dispatch state.
type StmtTokenProjection = (
    Vec<TokenId>,
    BTreeMap<TokenId, ProjectedStmtToken>,
    BTreeMap<TokenId, LiteralId>,
    TaskState,
);

fn project_stmt_tokens(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
    want: TaskKind,
    label: &str,
) -> Result<StmtTokenProjection, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("{label} of unknown task {}", task.index())))?;
    if record.kind != want {
        return Err(protocol_fault(format!(
            "{label} task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "{label} task {} payload must carry tokens only",
                task.index()
            )));
        };
        tokens.push(*id);
        if let Ok(body) = bus.arenas.tokens.get(*id) {
            let spelling = body
                .name
                .and_then(|name| bus.intern.get(name).ok().map(|bytes| bytes.to_vec()))
                .unwrap_or_default();
            bodies.insert(
                *id,
                ProjectedStmtToken {
                    kind: body.kind,
                    spelling,
                },
            );
        }
    }
    // Committed literals indexed by their token back-link (ID order).
    let mut literals_by_token = BTreeMap::new();
    for (id, literal) in bus.arenas.literals.iter() {
        if let Some(token) = literal.token {
            literals_by_token.insert(token, id);
        }
    }
    Ok((tokens, bodies, literals_by_token, record.state.clone()))
}

/// Build the narrow PA27 projection for one dispatched task.
pub fn project_pa_stmt_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaStmtInput, DiagnosticDraft> {
    let (tokens, bodies, literals_by_token, state) =
        project_stmt_tokens(bus, task, pa27_task_kind(), "statement dispatch")?;
    Ok(PaStmtInput {
        task,
        state,
        tokens,
        bodies,
        literals_by_token,
    })
}

/// Build the narrow PA29 projection for one dispatched task.
pub fn project_pa_if_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaIfInput, DiagnosticDraft> {
    let (tokens, bodies, _, state) = project_stmt_tokens(bus, task, pa29_task_kind(), "if parse")?;
    Ok(PaIfInput {
        task,
        state,
        tokens,
        bodies,
    })
}

/// Build the narrow PA30 projection for one dispatched task.
pub fn project_pa_switch_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaSwitchInput, DiagnosticDraft> {
    let (tokens, bodies, _, state) =
        project_stmt_tokens(bus, task, pa30_task_kind(), "switch parse")?;
    Ok(PaSwitchInput {
        task,
        state,
        tokens,
        bodies,
    })
}

/// Build the narrow PA31 projection for one dispatched task.
pub fn project_pa_loop_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaLoopInput, DiagnosticDraft> {
    let (tokens, bodies, _, state) =
        project_stmt_tokens(bus, task, pa31_task_kind(), "loop parse")?;
    Ok(PaLoopInput {
        task,
        state,
        tokens,
        bodies,
    })
}

/// The fused T05 PA27/PA29/PA30/PA31 worker (M1 slice: fixed-shape
/// dispatch + `Ack`; if/switch/loop always `Unsupported`).
pub struct PaStmtChip;

impl Worker for PaStmtChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA_STMT_CANDIDATE_CHIP,
            chip_name: "PaStmtChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![
                pa27_task_kind(),
                pa29_task_kind(),
                pa30_task_kind(),
                pa31_task_kind(),
            ],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Lex, "literals"),
                FieldPath::new(StoreId::Names, "entries"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            // Candidate-only: no shared acceptance suite exists yet. The
            // wiring integrator owns the acceptance path when the rows
            // freeze; in-file unit tests below cover the pure path.
            tests: vec![],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let kind = match bus.arenas.tasks.get(task) {
            Ok(record) => record.kind,
            Err(_) => {
                return vec![fail(
                    task,
                    protocol_fault(format!("statement parse of unknown task {}", task.index())),
                )];
            }
        };
        match PaStmtProduction::of_kind(kind) {
            Some(PaStmtProduction::Statement) => match project_pa_stmt_input(bus, task) {
                Ok(input) => self.compute_statement(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            Some(PaStmtProduction::If) => match project_pa_if_input(bus, task) {
                Ok(input) => self.compute_if(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            Some(PaStmtProduction::Switch) => match project_pa_switch_input(bus, task) {
                Ok(input) => self.compute_switch(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            Some(PaStmtProduction::Loop) => match project_pa_loop_input(bus, task) {
                Ok(input) => self.compute_loop(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            None => vec![fail(
                task,
                protocol_fault(format!(
                    "statement-parse task {} has unexpected kind {}",
                    task.index(),
                    kind.raw()
                )),
            )],
        }
    }
}

fn check_running(task: TaskId, state: &TaskState, label: &str) -> Result<(), DiagnosticDraft> {
    if *state != TaskState::Running {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("{label} task {} is not running", task.index()),
        ));
    }
    Ok(())
}

fn check_token_kinds(
    tokens: &[TokenId],
    bodies: &BTreeMap<TokenId, ProjectedStmtToken>,
    want: &[TokenKind],
    arity_note: &str,
) -> Result<(), DiagnosticDraft> {
    if tokens.len() != want.len() {
        return Err(DiagnosticDraft::unsupported(arity_note));
    }
    for (position, id) in tokens.iter().enumerate() {
        let Some(projected) = bodies.get(id) else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("parse reads missing token {}", id.index()),
            ));
        };
        if projected.kind != want[position] {
            return Err(DiagnosticDraft::unsupported(
                "non-M1 token shape at fixed position",
            ));
        }
    }
    Ok(())
}

fn check_return_spelling(
    tokens: &[TokenId],
    bodies: &BTreeMap<TokenId, ProjectedStmtToken>,
    position: usize,
) -> Result<(), DiagnosticDraft> {
    let id = tokens[position];
    if bodies[&id].spelling != b"return".as_slice() {
        return Err(DiagnosticDraft::unsupported(
            "non-M1 keyword spelling (expected `return`)",
        ));
    }
    Ok(())
}

fn check_integer_literals(
    task: TaskId,
    tokens: &[TokenId],
    literals_by_token: &BTreeMap<TokenId, LiteralId>,
    positions: &[usize],
) -> Result<(), DiagnosticDraft> {
    for position in positions {
        let id = tokens[*position];
        if !literals_by_token.contains_key(&id) {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "parse task {} reads missing literal for token {}",
                    task.index(),
                    id.index()
                ),
            ));
        }
    }
    Ok(())
}

/// Require every payload token to have a committed body (protocol check
/// shared by the always-`Unsupported` PA29/PA30/PA31 paths).
fn check_bodies_present(
    tokens: &[TokenId],
    bodies: &BTreeMap<TokenId, ProjectedStmtToken>,
) -> Result<(), DiagnosticDraft> {
    for id in tokens {
        if !bodies.contains_key(id) {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("parse reads missing token {}", id.index()),
            ));
        }
    }
    Ok(())
}

impl PaStmtChip {
    /// Pure PA27 computation over the narrow projection (no bus access).
    ///
    /// M1 accepts exactly the 7-token block or the 5-token return and
    /// completes `Ack` with the documented subtree shape (not committed).
    /// Anything else is an explicit `Unsupported` fail; protocol faults
    /// use `Task/4`.
    pub fn compute_statement(&self, input: &PaStmtInput) -> Vec<Proposal> {
        match parse_statement_dispatch(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }

    /// Pure PA29 computation (M1: always `Unsupported` after protocol
    /// checks; no bus access).
    pub fn compute_if(&self, input: &PaIfInput) -> Vec<Proposal> {
        match parse_if_statement(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }

    /// Pure PA30 computation (M1: always `Unsupported` after protocol
    /// checks; no bus access).
    pub fn compute_switch(&self, input: &PaSwitchInput) -> Vec<Proposal> {
        match parse_switch_statement(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }

    /// Pure PA31 computation (M1: always `Unsupported` after protocol
    /// checks; no bus access).
    pub fn compute_loop(&self, input: &PaLoopInput) -> Vec<Proposal> {
        match parse_loop_statement(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Pure M1 statement dispatch: validate the 7-token block or the 5-token
/// return and build its local subtree shape. No bus access.
///
/// Pre-order shape rows for the block: `Compound(0)` → `Return(1)` →
/// `BinaryAdd(2)` → `IntLiteral(3)`, `IntLiteral(4)`. For the return:
/// `Return(0)` → `BinaryAdd(1)` → `IntLiteral(2)`, `IntLiteral(3)`.
pub fn parse_statement_dispatch(
    input: &PaStmtInput,
) -> Result<Vec<LocalStmtShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "statement dispatch")?;
    if input.tokens.len() == M1_BLOCK_KINDS.len() {
        check_token_kinds(
            &input.tokens,
            &input.bodies,
            M1_BLOCK_KINDS,
            "non-M1 statement: exactly `{ return <int> + <int> ; }` or `return <int> + <int> ;` required",
        )?;
        check_return_spelling(&input.tokens, &input.bodies, 1)?;
        check_integer_literals(input.task, &input.tokens, &input.literals_by_token, &[2, 4])?;
        return Ok(vec![
            LocalStmtShape {
                kind_name: "Compound",
                parent: None,
                first_token: 0,
                last_token: 6,
                has_literal: false,
            },
            LocalStmtShape {
                kind_name: "Return",
                parent: Some(0),
                first_token: 1,
                last_token: 5,
                has_literal: false,
            },
            LocalStmtShape {
                kind_name: "BinaryAdd",
                parent: Some(1),
                first_token: 2,
                last_token: 4,
                has_literal: false,
            },
            LocalStmtShape {
                kind_name: "IntLiteral",
                parent: Some(2),
                first_token: 2,
                last_token: 2,
                has_literal: true,
            },
            LocalStmtShape {
                kind_name: "IntLiteral",
                parent: Some(2),
                first_token: 4,
                last_token: 4,
                has_literal: true,
            },
        ]);
    }
    if input.tokens.len() == M1_RETURN_KINDS.len() {
        check_token_kinds(
            &input.tokens,
            &input.bodies,
            M1_RETURN_KINDS,
            "non-M1 statement: exactly `{ return <int> + <int> ; }` or `return <int> + <int> ;` required",
        )?;
        check_return_spelling(&input.tokens, &input.bodies, 0)?;
        check_integer_literals(input.task, &input.tokens, &input.literals_by_token, &[1, 3])?;
        return Ok(vec![
            LocalStmtShape {
                kind_name: "Return",
                parent: None,
                first_token: 0,
                last_token: 4,
                has_literal: false,
            },
            LocalStmtShape {
                kind_name: "BinaryAdd",
                parent: Some(0),
                first_token: 1,
                last_token: 3,
                has_literal: false,
            },
            LocalStmtShape {
                kind_name: "IntLiteral",
                parent: Some(1),
                first_token: 1,
                last_token: 1,
                has_literal: true,
            },
            LocalStmtShape {
                kind_name: "IntLiteral",
                parent: Some(1),
                first_token: 3,
                last_token: 3,
                has_literal: true,
            },
        ]);
    }
    Err(DiagnosticDraft::unsupported(
        "non-M1 statement: exactly `{ return <int> + <int> ; }` or `return <int> + <int> ;` required",
    ))
}

/// Pure PA29 if-statement parse (M1: always `Unsupported`).
///
/// Runs the `Running` + token-presence protocol checks, then rejects: the
/// `if`/`else` grammar (incl. dangling-else) is deferred past M1.
pub fn parse_if_statement(input: &PaIfInput) -> Result<Vec<LocalStmtShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "if parse")?;
    check_bodies_present(&input.tokens, &input.bodies)?;
    Err(DiagnosticDraft::unsupported(
        "non-M1 statement: `if` is unsupported in the M1 slice",
    ))
}

/// Pure PA30 switch-statement parse (M1: always `Unsupported`).
///
/// Runs the `Running` + token-presence protocol checks, then rejects: the
/// `switch`/`case`/`default` grammar is deferred past M1.
pub fn parse_switch_statement(
    input: &PaSwitchInput,
) -> Result<Vec<LocalStmtShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "switch parse")?;
    check_bodies_present(&input.tokens, &input.bodies)?;
    Err(DiagnosticDraft::unsupported(
        "non-M1 statement: `switch` is unsupported in the M1 slice",
    ))
}

/// Pure PA31 loop-statement parse (M1: always `Unsupported`).
///
/// Runs the `Running` + token-presence protocol checks, then rejects: the
/// `for`/`while`/`do-while` grammar (incl. declaration-in-`for` and the
/// `do…while;` trailing semicolon) is deferred past M1.
pub fn parse_loop_statement(input: &PaLoopInput) -> Result<Vec<LocalStmtShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "loop parse")?;
    check_bodies_present(&input.tokens, &input.bodies)?;
    Err(DiagnosticDraft::unsupported(
        "non-M1 statement: loops are unsupported in the M1 slice",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskState;

    fn token_ids(n: u32) -> Vec<TokenId> {
        (0..n).map(TokenId::from_index).collect()
    }

    fn stmt_bodies(
        kinds: &[TokenKind],
        spellings: &[(&str, usize)],
    ) -> BTreeMap<TokenId, ProjectedStmtToken> {
        let mut bodies = BTreeMap::new();
        for (position, kind) in kinds.iter().enumerate() {
            let id = TokenId::from_index(position as u32);
            let spelling = spellings
                .iter()
                .find(|(_, at)| *at == position)
                .map(|(text, _)| text.as_bytes().to_vec())
                .unwrap_or_default();
            bodies.insert(
                id,
                ProjectedStmtToken {
                    kind: *kind,
                    spelling,
                },
            );
        }
        bodies
    }

    fn stmt_input(
        tokens: Vec<TokenId>,
        bodies: BTreeMap<TokenId, ProjectedStmtToken>,
        literal_at: &[usize],
    ) -> PaStmtInput {
        let mut literals_by_token = BTreeMap::new();
        for position in literal_at {
            literals_by_token.insert(tokens[*position], LiteralId::from_index(*position as u32));
        }
        PaStmtInput {
            task: TaskId::from_index(7),
            state: TaskState::Running,
            tokens,
            bodies,
            literals_by_token,
        }
    }

    #[test]
    fn candidate_locals_are_first_free_parse_locals() {
        assert_eq!(PA27_CANDIDATE_LOCAL, 27);
        assert_eq!(PA29_CANDIDATE_LOCAL, 28);
        assert_eq!(PA30_CANDIDATE_LOCAL, 29);
        assert_eq!(PA31_CANDIDATE_LOCAL, 30);
        assert_eq!(PA_STMT_CANDIDATE_CHIP, ChipId(58));
    }

    #[test]
    fn candidate_kinds_resolve_in_parse_group() {
        for kind in [
            pa27_task_kind(),
            pa29_task_kind(),
            pa30_task_kind(),
            pa31_task_kind(),
        ] {
            assert_eq!(kind.group(), TaskGroup::PARSE);
        }
        assert_eq!(
            pa27_task_kind(),
            PA27_TASK_KIND_CANDIDATE.expect("PA27 resolves")
        );
    }

    #[test]
    fn production_dispatch_covers_all_four_kinds() {
        assert_eq!(
            PaStmtProduction::of_kind(pa27_task_kind()),
            Some(PaStmtProduction::Statement)
        );
        assert_eq!(
            PaStmtProduction::of_kind(pa29_task_kind()),
            Some(PaStmtProduction::If)
        );
        assert_eq!(
            PaStmtProduction::of_kind(pa30_task_kind()),
            Some(PaStmtProduction::Switch)
        );
        assert_eq!(
            PaStmtProduction::of_kind(pa31_task_kind()),
            Some(PaStmtProduction::Loop)
        );
        assert_eq!(
            PaStmtProduction::of_kind(TaskKind::PARSE_BLOCK),
            None,
            "frozen PA28 kind must not route into this candidate file"
        );
    }

    #[test]
    fn dispatch_accepts_m1_block() {
        let tokens = token_ids(7);
        let bodies = stmt_bodies(M1_BLOCK_KINDS, &[("return", 1)]);
        let input = stmt_input(tokens, bodies, &[2, 4]);
        let shapes = parse_statement_dispatch(&input).expect("M1 block must accept");
        assert_eq!(shapes.len(), 5);
        assert_eq!(shapes[0].kind_name, "Compound");
        assert_eq!(shapes[0].parent, None);
        assert_eq!((shapes[0].first_token, shapes[0].last_token), (0, 6));
        assert_eq!(shapes[1].kind_name, "Return");
        assert!(shapes[3].has_literal && shapes[4].has_literal);
    }

    #[test]
    fn dispatch_accepts_m1_return() {
        let tokens = token_ids(5);
        let bodies = stmt_bodies(M1_RETURN_KINDS, &[("return", 0)]);
        let input = stmt_input(tokens, bodies, &[1, 3]);
        let shapes = parse_statement_dispatch(&input).expect("M1 return must accept");
        assert_eq!(shapes.len(), 4);
        assert_eq!(shapes[0].kind_name, "Return");
        assert_eq!(shapes[0].parent, None);
    }

    #[test]
    fn dispatch_rejects_wrong_keyword_spelling() {
        let tokens = token_ids(5);
        let bodies = stmt_bodies(M1_RETURN_KINDS, &[("break", 0)]);
        let input = stmt_input(tokens, bodies, &[1, 3]);
        assert!(parse_statement_dispatch(&input).is_err());
    }

    #[test]
    fn dispatch_rejects_empty_and_if_shaped_inputs() {
        let empty = PaStmtInput {
            task: TaskId::from_index(7),
            state: TaskState::Running,
            tokens: Vec::new(),
            bodies: BTreeMap::new(),
            literals_by_token: BTreeMap::new(),
        };
        assert!(parse_statement_dispatch(&empty).is_err());

        // `if (1) return 2+3;`-shaped token soup: wrong arity and wrong
        // kinds at fixed positions — must be Unsupported, never accepted.
        let kinds = [
            TokenKind::Keyword,
            TokenKind::Punctuator,
            TokenKind::Integer,
            TokenKind::Punctuator,
            TokenKind::Keyword,
            TokenKind::Integer,
            TokenKind::Punctuator,
            TokenKind::Integer,
            TokenKind::Punctuator,
        ];
        let tokens = token_ids(9);
        let bodies = stmt_bodies(&kinds, &[("if", 0), ("return", 4)]);
        let input = stmt_input(tokens, bodies, &[2, 5, 7]);
        assert!(parse_statement_dispatch(&input).is_err());
    }

    #[test]
    fn missing_literal_is_task_protocol_fault() {
        let tokens = token_ids(5);
        let bodies = stmt_bodies(M1_RETURN_KINDS, &[("return", 0)]);
        // Only one of the two integer leaves is backed by a literal.
        let input = stmt_input(tokens, bodies, &[1]);
        let err = parse_statement_dispatch(&input).expect_err("must reject");
        assert_eq!(err.code.group, DiagGroup::Task);
    }

    #[test]
    fn not_running_is_task_protocol_fault() {
        let tokens = token_ids(5);
        let bodies = stmt_bodies(M1_RETURN_KINDS, &[("return", 0)]);
        let mut input = stmt_input(tokens, bodies, &[1, 3]);
        input.state = TaskState::Ready;
        let err = parse_statement_dispatch(&input).expect_err("must reject");
        assert_eq!(err.code.group, DiagGroup::Task);
    }

    #[test]
    fn if_switch_loop_are_always_unsupported() {
        let kinds = [
            TokenKind::Keyword,
            TokenKind::Punctuator,
            TokenKind::Integer,
        ];
        let tokens = token_ids(3);
        let if_input = PaIfInput {
            task: TaskId::from_index(11),
            state: TaskState::Running,
            tokens: tokens.clone(),
            bodies: stmt_bodies(&kinds, &[("if", 0)]),
        };
        assert!(parse_if_statement(&if_input).is_err());
        let switch_input = PaSwitchInput {
            task: TaskId::from_index(12),
            state: TaskState::Running,
            tokens: tokens.clone(),
            bodies: stmt_bodies(&kinds, &[("switch", 0)]),
        };
        assert!(parse_switch_statement(&switch_input).is_err());
        let loop_input = PaLoopInput {
            task: TaskId::from_index(13),
            state: TaskState::Running,
            tokens,
            bodies: stmt_bodies(&kinds, &[("while", 0)]),
        };
        assert!(parse_loop_statement(&loop_input).is_err());
    }

    #[test]
    fn worker_is_zero_sized_stateless() {
        assert_eq!(std::mem::size_of::<PaStmtChip>(), 0);
    }

    #[test]
    fn manifest_is_ack_only_and_deterministic() {
        let manifest = PaStmtChip.manifest();
        assert_eq!(manifest.id, PA_STMT_CANDIDATE_CHIP);
        assert!(manifest.writes.is_empty());
        assert!(manifest.deterministic);
        assert_eq!(manifest.task_kinds.len(), 4);
    }
}
