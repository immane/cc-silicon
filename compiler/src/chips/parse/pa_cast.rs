// ============================================================================
// chips/parse/pa_cast.rs — T05 PA21 cast + PA23 conditional + PA25 comma
// workers (Wave 4 draft, NOT frozen).
//
// M1 disposition: REJECT. All three productions fail `Unsupported` with a
// documented reason; no M1 shape is accepted here.
//
//   - PA21 needs the T06 `ConversionOp` freeze before any cast can be
//     certified: the operand conversion (usual arithmetic / pointer /
//     `_Bool` decay selection, sign/width application) is T06/T07-owned,
//     and no `ConversionOp` symbol exists in `compiler/src` (verified
//     2026-10-10: zero matches). Certifying a cast node without the frozen
//     conversion operator would fabricate semantics.
//   - PA21 additionally needs the type-name decision (PA10 abstract
//     declarator + PA04 typedef query through T06 scope facts) to
//     distinguish `(T) x` (cast) from `(x) + 1` (parenthesized, PA16
//     territory) and `(T) { 1 }` (compound literal, PA10/T08 territory).
//     A typedef-shadowed name must never be guessed from spelling.
//   - PA23/PA25 need the child-task recursion + await-all join protocol
//     (T05 items A–C) for their operands, which this one-tick Ack-only
//     draft does not enqueue. PA25 must additionally not swallow
//     argument-separating commas (PA18 call territory) or `for`-init
//     commas (PA31 territory).
//
// Pure shape classifiers ([`classify_cast_shape`], [`classify_conditional_shape`],
// [`classify_comma_shape`]) validate the token-level delimiter geometry with
// no bus access so the future split can lift them verbatim; the chip-level
// [`parse_cast_expression`] / [`parse_conditional_expression`] /
// [`parse_comma_expression`] run the classifier, document the future
// [`CastNodeShape`] rows, and then return the M1 `Unsupported` deferral
// (never a pass, never a guess).
//
// Pure validate, no writes: every path yields a `Fail` proposal in M1
// (protocol faults use `Task/4`). No child task is enqueued here, so no
// node-link or child-payload schema is invented. The `next_cursor` carrier
// stays open (T05 item G.4 OB-30); the caller holds the cursor.
//
// Candidate registration (NOT frozen, integrator-owned): `PA21_LOCAL` 27
// (`parse.cast`), `PA23_LOCAL` 28 (`parse.conditional`), `PA25_LOCAL` 29
// (`parse.comma`) — the next three free `PARSE` locals after
// `PARSE_RECOVERY` (local 26, `/36`); `PA_CAST_CANDIDATE_CHIP` is
// `ChipId(58)`, the next free ID after `SE_FUNC_CHIP` (`ChipId(57)`).
// Rows belong in `TaskKindRegistry`, `STAGE_ASSIGNMENT` (stage 2), and the
// routed layer 2; this file must not invent them (see
// docs/tasks/PARALLEL_EXECUTION.md §2). There is no store-owner allowlist
// row (zero writes).
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA21 CastExpressionChip + PA23 ParseConditionalExpressionChip
//   + PA25 ParseCommaExpressionChip / PARSE (candidate `PARSE` locals
//   27/28/29, `PA_CAST_CANDIDATE_CHIP = ChipId(58)`; see the candidate
//   registration below). Single-chip file per the worker template: one
//   zero-sized unit struct, `PaCastChip`, serves all three productions and
//   dispatches on the production field (the dispatched task's `TaskKind`,
//   mirroring `ContinuationRecord.production`).
// Contract version/hash: T05 PENDING; T01 `/36` current (no `/6` freeze
//   claimed by this draft).
// Owned files: ONLY this file (`compiler/src/chips/parse/pa_cast.rs`).
//   Never edits `mod.rs`, bus/task/ids/manifest schemas, or registrations.
// Task kind / payload fields / result tag: candidate `parse.cast`
//   (`PARSE` local 27), `parse.conditional` (local 28), `parse.comma`
//   (local 29); payload = the production token refs in source order
//   (no EOF, no surrounding tokens); result = `Fail(Unsupported)` in M1
//   (no next-cursor carrier: OB-30 open, caller holds the cursor).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: NONE in M1 (all shapes explicit `Unsupported`);
//   GNU omitted-middle `a ?: c` (PA23) stays mode-gated past M1.
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` (`/11`), `pp.tokens` (`/11`, delimiter ground truth),
//   `names.entries` (`/11`). Pure query: no bus mutation; workers emit
//   `Proposal`s only.
// Writes: NONE. No `AppendRecords`, no `StorePatch`, no `Enqueue`. The
//   `CastNodeShape` rows below copy the frozen `NodeRecord` field shapes
//   chip-locally for documentation/future split only; they are NOT
//   committed.
// Dependencies: T04 committed tokens + interned spellings + committed PP
//   spellings (producer facts only; never calls another chip). The
//   type-name decision is a PA10/PA04/T06 fact this file never derives.
// Preconditions: dispatched task is `Running` with a PA21/PA23/PA25
//   candidate kind; payload carries `Token` refs only; referenced tokens
//   are committed.
// Transition states: Running → Fail (`Unsupported` group) on every shape
//   in M1 (explicit rejection, stays in the denominator); Running → Fail
//   (`Task` group code 4) on not-running/missing-record protocol faults.
// Algorithm obligations: classify the delimiter geometry with the pure
//   classifiers, then reject with the production-specific M1 deferral
//   (see [`CastError`]). Never guess typedef-ness, never interpret
//   literal values, never swallow a neighboring production's comma.
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = dispatch-then-project-then-compute; all paths
//   yield proposals.
// Error codes / recovery: `Unsupported/1` for every M1 shape (explicit
//   rejection, stays in the denominator); `Task/4` for protocol faults;
//   `Protocol/1` for unknown-task/wrong-kind scaffolding. No recovery sync
//   is attempted here (PA38 owns it).
// Required fixtures: in-file `#[cfg(test)]` unit tests below pin the pure
//   core only; the shared acceptance suite is integrator-owned future work
//   (no committed suite is claimed by this draft).
// Integration acceptance: NOT FROZEN (candidate locals/chip only;
//   registry/stage/routing/allowlist rows open; cursor carrier, parent
//   linkage, cast recursion protocol, `ConversionOp` freeze open).
// Known unsupported: everything in M1 (all casts, all `?:` conditionals
//   including GNU omitted-middle, all comma expressions).
//
// M1 boundary (see T05 item G): `2+3` is the PA22 shape owned by
//   `pa_binary.rs`; `(x)+1` is parenthesized PA16 territory, never a cast;
//   `(T){1}` is compound-literal PA10/T08 territory, never a bare cast.
//   Folding or nesting any of these is wiring-layer composition through
//   child tasks, deferred.
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-4 (nesting): the full design recurses into operands (and into
//     PA10 for the type name, T06 for the typedef/`ConversionOp` facts)
//     through child tasks and continuations with await-all joins (T05
//     items A–C); this draft validates the flat token geometry in one
//     tick with no `Enqueue`, `AwaitChildren`, or `Progress`.
//     Parse-depth accounting (`ParseDepthExceeded`, item D) stays a future
//     integration check.
//   DEFECT-6 (ambiguity): every input fails `Unsupported` instead of
//     guessing (e.g. `(x)+1` is not read as a cast, `(T){1}` is not read
//     as a bare cast, `a?b:c` is not folded, `a,b` is not claimed from a
//     call-argument window, a missing token record is a loud `Task/4`
//     fault rather than a default).
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, NameId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use crate::task::{Proposal, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Candidate `PARSE` local for `parse.cast` (PA21).
///
/// Local 27 — the first free local after `PARSE_RECOVERY` (local 26,
/// `/36`). Candidate only; the registry row is integrator-owned.
pub const PA21_CANDIDATE_LOCAL: u16 = 27;

/// Candidate `PARSE` local for `parse.conditional` (PA23).
///
/// Local 28 — the next free local after the PA21 candidate. Candidate
/// only; the registry row is integrator-owned.
pub const PA23_CANDIDATE_LOCAL: u16 = 28;

/// Candidate `PARSE` local for `parse.comma` (PA25).
///
/// Local 29 — the next free local after the PA23 candidate. Candidate
/// only; the registry row is integrator-owned.
pub const PA25_CANDIDATE_LOCAL: u16 = 29;

/// Candidate chip ID for the fused PA21/PA23/PA25 worker.
///
/// `ChipId(58)` — the next free ID after `SE_FUNC_CHIP` (`ChipId(57)`).
/// Candidate only; the manifest registration is integrator-owned.
pub const PA_CAST_CANDIDATE_CHIP: ChipId = ChipId(58);

/// Candidate task kind for PA21 cast-expression parsing (`parse.cast`).
pub const PA21_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA21_CANDIDATE_LOCAL);

/// Candidate task kind for PA23 conditional-expression parsing
/// (`parse.conditional`).
pub const PA23_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA23_CANDIDATE_LOCAL);

/// Candidate task kind for PA25 comma-expression parsing (`parse.comma`).
pub const PA25_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA25_CANDIDATE_LOCAL);

/// Resolve the PA21 candidate task kind.
///
/// Returns the `PARSE` local-27 kind; `None` only if the local exceeded
/// `LOCAL_MAX` (impossible for 27 — the `expect` is unreachable and loud
/// rather than silent).
pub fn pa21_task_kind() -> TaskKind {
    PA21_TASK_KIND_CANDIDATE.expect("PA21 candidate local 27 fits in LOCAL_MAX")
}

/// Resolve the PA23 candidate task kind.
pub fn pa23_task_kind() -> TaskKind {
    PA23_TASK_KIND_CANDIDATE.expect("PA23 candidate local 28 fits in LOCAL_MAX")
}

/// Resolve the PA25 candidate task kind.
pub fn pa25_task_kind() -> TaskKind {
    PA25_TASK_KIND_CANDIDATE.expect("PA25 candidate local 29 fits in LOCAL_MAX")
}

/// Production served by [`PaCastChip`]: the dispatch target resolved from
/// the dispatched task's kind (the production field).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaCastProduction {
    /// PA21 cast-expression production (`( type-name ) cast-expr`).
    Cast,
    /// PA23 conditional-expression production (`cond ? then : else`).
    Conditional,
    /// PA25 comma-expression production (`a , b`).
    Comma,
}

impl PaCastProduction {
    /// Resolve the production for a dispatched task kind.
    pub fn of_kind(kind: TaskKind) -> Option<Self> {
        if kind == pa21_task_kind() {
            Some(Self::Cast)
        } else if kind == pa23_task_kind() {
            Some(Self::Conditional)
        } else if kind == pa25_task_kind() {
            Some(Self::Comma)
        } else {
            None
        }
    }
}

/// Chip-local copy of the frozen `NodeRecord` field shapes for one
/// cast/conditional/comma subtree node.
///
/// Same field names/order as `NodeRecord` (`parent`, `first_token`,
/// `last_token`, literal presence) so the future split can lift it
/// verbatim, except positions are indices into the task's own token slice
/// and links are pre-order node indices rather than committed IDs; the kind
/// is the `NodeKind` variant name string rather than imported shared
/// schema. Shapes are built by the pure path and documented — they are NOT
/// appended to any arena by this zero-write draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastNodeShape {
    /// Fixed node-kind name (mirrors `NodeKind`, e.g. `"Cast"`).
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

/// Why a token slice is not an M1 cast/conditional/comma parse.
///
/// Every variant maps to `Unsupported` (explicit rejection, stays in the
/// denominator). Numeric codes are chip-local candidates; the frozen code
/// table is a T01 `/6` detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastError {
    /// A well-formed (or malformed) shape outside the M1 subset.
    Unsupported(&'static str),
}

impl CastError {
    /// Map to the structured failure diagnostic for this rejection.
    pub fn diagnostic(&self) -> DiagnosticDraft {
        match self {
            Self::Unsupported(message) => DiagnosticDraft::unsupported(*message),
        }
    }
}

/// One projected token: kind plus resolved spellings.
///
/// Identifier/keyword spellings resolve through the intern table
/// (`spelling`); punctuator/number spellings resolve through the committed
/// PP token (`pp_spelling`), because committed C punctuator records carry
/// no interned name. Either side is empty only when the backing record is
/// missing, which the computation reports loudly instead of guessing.
#[derive(Clone, Debug)]
pub struct ProjectedCastToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Interned spelling bytes for named tokens.
    pub spelling: Vec<u8>,
    /// Committed PP spelling bytes (delimiter ground truth).
    pub pp_spelling: Vec<u8>,
}

impl ProjectedCastToken {
    /// Effective spelling bytes: intern bytes for named tokens, committed
    /// PP bytes otherwise. The projector resolves which side applies; the
    /// pure core only compares bytes.
    pub fn effective_spelling(&self) -> &[u8] {
        match self.kind {
            TokenKind::Identifier | TokenKind::Keyword => &self.spelling,
            _ => &self.pp_spelling,
        }
    }
}

/// Narrow projection for the PA21/PA23/PA25 computation.
#[derive(Clone, Debug)]
pub struct PaCastInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Production under dispatch (resolved from the task kind).
    pub production: PaCastProduction,
    /// Payload token IDs in source order.
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedCastToken>,
}

/// One cast/conditional/comma token for the pure grammar core.
///
/// `spelling` is the effective spelling: intern bytes for identifiers and
/// keywords, committed PP bytes for punctuators/integers. The projector
/// resolves which side applies; the pure core only compares bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Effective spelling bytes.
    pub spelling: Vec<u8>,
}

/// Delimiter probe for a syntactically plausible cast shape.
///
/// `( type-name ) operand`: the leading paren pair at depth 0 plus a
/// non-empty operand suffix. Whether the paren contents name a type
/// (PA10/PA04/T06 decision) and which `ConversionOp` applies (T06 freeze)
/// are both open — the probe certifies geometry only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastProbe {
    /// Index of the matching `)` for the leading `(`.
    pub rparen: usize,
}

/// Delimiter probe for a syntactically plausible conditional shape.
///
/// `cond ? then : else` with `?` strictly before `:` at depth 0. The
/// operands are child-task work (T05 items A–C); the GNU omitted-middle
/// `a ?: c` is a distinct deferred shape, never silently filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConditionalProbe {
    /// Index of the top-level `?`.
    pub question: usize,
    /// Index of the top-level `:` paired with `question`.
    pub colon: usize,
}

/// Delimiter probe for a syntactically plausible comma shape.
///
/// `lhs , rhs` split at the first top-level comma (left-associative fold
/// point for longer chains). Commas nested inside `(...)` (call arguments,
/// PA18 territory) or claimed by `for`-init (PA31 territory) never reach
/// this probe — the caller bounds the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommaProbe {
    /// Index of the first top-level `,`.
    pub comma: usize,
}

/// Project the token stream for one cast/conditional/comma task.
fn project_cast_tokens(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
    want: TaskKind,
    label: &str,
    production: PaCastProduction,
) -> Result<PaCastInput, DiagnosticDraft> {
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
            let pp_spelling = bus
                .arenas
                .pp_tokens
                .get(body.pp_token)
                .map(|pp| pp.spelling.clone())
                .unwrap_or_default();
            bodies.insert(
                *id,
                ProjectedCastToken {
                    kind: body.kind,
                    name: body.name,
                    spelling,
                    pp_spelling,
                },
            );
        }
    }
    Ok(PaCastInput {
        task,
        state: record.state.clone(),
        production,
        tokens,
        bodies,
    })
}

/// Build the narrow PA21 projection for one dispatched task.
///
/// Payload convention (candidate): the cast token refs in source order,
/// `( type-name ) operand`, no EOF, no surrounding tokens. A wrong kind,
/// a non-token ref, or an unknown task is a caller protocol fault;
/// arity/content is `compute`'s job so M1 input surfaces as `Unsupported`,
/// never as a silent pass.
pub fn project_pa_cast_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaCastInput, DiagnosticDraft> {
    project_cast_tokens(
        bus,
        task,
        pa21_task_kind(),
        "cast parse",
        PaCastProduction::Cast,
    )
}

/// Build the narrow PA23 projection for one dispatched task.
///
/// Payload convention (candidate): the conditional token refs in source
/// order, `cond ? then : else`, no EOF, no surrounding tokens.
pub fn project_pa_conditional_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaCastInput, DiagnosticDraft> {
    project_cast_tokens(
        bus,
        task,
        pa23_task_kind(),
        "conditional parse",
        PaCastProduction::Conditional,
    )
}

/// Build the narrow PA25 projection for one dispatched task.
///
/// Payload convention (candidate): the comma token refs in source order,
/// `lhs , rhs`, no EOF, no surrounding tokens. The caller bounds the
/// window so call-argument and `for`-init commas never arrive here.
pub fn project_pa_comma_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaCastInput, DiagnosticDraft> {
    project_cast_tokens(
        bus,
        task,
        pa25_task_kind(),
        "comma parse",
        PaCastProduction::Comma,
    )
}

/// The fused T05 PA21/PA23/PA25 worker (M1 draft: classify-then-reject).
pub struct PaCastChip;

impl Worker for PaCastChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA_CAST_CANDIDATE_CHIP,
            chip_name: "PaCastChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![pa21_task_kind(), pa23_task_kind(), pa25_task_kind()],
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
                    protocol_fault(format!("cast parse of unknown task {}", task.index())),
                )];
            }
        };
        match PaCastProduction::of_kind(kind) {
            Some(PaCastProduction::Cast) => match project_pa_cast_input(bus, task) {
                Ok(input) => self.compute_cast(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            Some(PaCastProduction::Conditional) => match project_pa_conditional_input(bus, task) {
                Ok(input) => self.compute_conditional(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            Some(PaCastProduction::Comma) => match project_pa_comma_input(bus, task) {
                Ok(input) => self.compute_comma(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            None => vec![fail(
                task,
                protocol_fault(format!(
                    "cast-parse task {} has unexpected kind {}",
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

/// Materialize the effective-spelling views for the pure core.
///
/// A missing committed token record is a loud `Task/4` fault, never a
/// guessed spelling; an empty effective spelling is likewise a fault
/// (the backing record is absent on exactly one side).
fn cast_views(input: &PaCastInput, label: &str) -> Result<Vec<CastToken>, DiagnosticDraft> {
    let mut views = Vec::with_capacity(input.tokens.len());
    for id in &input.tokens {
        let Some(projected) = input.bodies.get(id) else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("parse reads missing token {}", id.index()),
            ));
        };
        let spelling = projected.effective_spelling().to_vec();
        if spelling.is_empty() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("{label} reads token {} with no spelling", id.index()),
            ));
        }
        views.push(CastToken {
            kind: projected.kind,
            spelling,
        });
    }
    Ok(views)
}

impl PaCastChip {
    /// Pure PA21 computation over the narrow projection (no bus access).
    ///
    /// M1 always fails `Unsupported`: the cast needs the unfrozen T06
    /// `ConversionOp` plus the PA10/PA04 type-name decision. Protocol
    /// faults use `Task/4`.
    pub fn compute_cast(&self, input: &PaCastInput) -> Vec<Proposal> {
        match parse_cast_expression(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: crate::task::ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }

    /// Pure PA23 computation over the narrow projection (no bus access).
    ///
    /// M1 always fails `Unsupported`: the operands need child-task
    /// recursion with await-all joins (T05 items A–C), deferred here.
    pub fn compute_conditional(&self, input: &PaCastInput) -> Vec<Proposal> {
        match parse_conditional_expression(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: crate::task::ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }

    /// Pure PA25 computation over the narrow projection (no bus access).
    ///
    /// M1 always fails `Unsupported`: the operands need child-task
    /// recursion (T05 items A–C), and the window bounding that keeps
    /// call/`for` commas out is caller-owned.
    pub fn compute_comma(&self, input: &PaCastInput) -> Vec<Proposal> {
        match parse_comma_expression(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: crate::task::ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Pure cast-shape classifier: locate `( ... ) operand` geometry.
///
/// Accepts a leading `(` with its depth-0 matching `)` and a non-empty
/// operand suffix. Returns the probe for the future split; it does NOT
/// decide whether the paren contents name a type (PA10/PA04/T06), whether
/// the shape is a compound literal (`)` followed by `{`, PA10/T08), or
/// which conversion applies (T06 `ConversionOp`). A `(x)+1` parenthesized
/// operand followed by a trailing operator is classified (geometry only);
/// the PA21 rejection reasons document that geometry is not a cast
/// decision.
pub fn classify_cast_shape(tokens: &[CastToken]) -> Result<CastProbe, CastError> {
    let first = tokens.first().ok_or(CastError::Unsupported(
        "non-cast shape: a cast opens with `(`",
    ))?;
    if first.kind != TokenKind::Punctuator || first.spelling != b"(" {
        return Err(CastError::Unsupported(
            "non-cast shape: a cast opens with `(`, parenthesized-operand tails belong to PA16",
        ));
    }
    let mut depth: u32 = 0;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Punctuator {
            continue;
        }
        if token.spelling == b"(" {
            depth = depth.saturating_add(1);
        } else if token.spelling == b")" {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                if index + 1 >= tokens.len() {
                    return Err(CastError::Unsupported(
                        "cast shape has no operand after `)`; a bare `(T)` with no operand is not a cast",
                    ));
                }
                return Ok(CastProbe { rparen: index });
            }
        }
    }
    Err(CastError::Unsupported(
        "unterminated cast shape: no matching `)` for the leading `(`",
    ))
}

/// Pure conditional-shape classifier: locate top-level `? ... :`.
///
/// Accepts exactly one depth-0 `?` strictly before a depth-0 `:` with a
/// non-empty condition prefix and a non-empty tail after `:`. Adjacent
/// `? :` (GNU omitted-middle `a ?: c`) is classified distinctly and stays
/// mode-gated; nested `?:` tails fold right-associatively in the future
/// split through child tasks.
pub fn classify_conditional_shape(tokens: &[CastToken]) -> Result<ConditionalProbe, CastError> {
    let mut depth: u32 = 0;
    let mut question: Option<usize> = None;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Punctuator {
            continue;
        }
        match token.spelling.as_slice() {
            b"(" => depth = depth.saturating_add(1),
            b")" => depth = depth.saturating_sub(1),
            b"?" if depth == 0 && question.is_none() => question = Some(index),
            b":" if depth == 0 => {
                if let Some(question) = question {
                    if question == 0 {
                        return Err(CastError::Unsupported(
                            "conditional shape has no condition before `?`",
                        ));
                    }
                    if question + 1 == index {
                        return Err(CastError::Unsupported(
                            "GNU omitted-middle `a ?: c` is mode-gated past M1; the middle operand must not be silently filled",
                        ));
                    }
                    if index + 1 >= tokens.len() {
                        return Err(CastError::Unsupported(
                            "conditional shape has no else operand after `:`",
                        ));
                    }
                    return Ok(ConditionalProbe {
                        question,
                        colon: index,
                    });
                }
            }
            _ => {}
        }
    }
    Err(CastError::Unsupported(
        "non-conditional shape: `cond ? then : else` with a top-level `?` before `:` is required",
    ))
}

/// Pure comma-shape classifier: locate the first top-level comma.
///
/// Accepts a `,` at paren depth 0 with non-empty operands on both sides;
/// longer chains fold left-associatively in the future split. A comma
/// nested inside `(...)` never classifies here — the caller must bound
/// the window so PA18 call-argument commas never arrive.
pub fn classify_comma_shape(tokens: &[CastToken]) -> Result<CommaProbe, CastError> {
    let mut depth: u32 = 0;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Punctuator {
            continue;
        }
        match token.spelling.as_slice() {
            b"(" => depth = depth.saturating_add(1),
            b")" => depth = depth.saturating_sub(1),
            b"," if depth == 0 => {
                if index == 0 {
                    return Err(CastError::Unsupported(
                        "comma shape has no left operand before `,`",
                    ));
                }
                if index + 1 >= tokens.len() {
                    return Err(CastError::Unsupported(
                        "comma shape has no right operand after `,`",
                    ));
                }
                return Ok(CommaProbe { comma: index });
            }
            _ => {}
        }
    }
    Err(CastError::Unsupported(
        "non-comma shape: a top-level `,` with operands on both sides is required; nested call-argument commas belong to PA18",
    ))
}

/// Pure M1 cast parse: classify the geometry, document the future shape,
/// then reject.
///
/// Even a geometrically perfect `(T) x` fails `Unsupported`: without the
/// frozen T06 `ConversionOp` and the PA10/PA04 type-name decision this
/// chip cannot distinguish a cast from `(x)+1` (parenthesized) or
/// `(T){1}` (compound literal), so accepting would fabricate semantics.
///
/// Future pre-order shape rows (documented, NOT committed):
/// `Cast(0)` → `TypeName(1)`, `Operand(2)`.
pub fn parse_cast_expression(input: &PaCastInput) -> Result<Vec<CastNodeShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "cast parse")?;
    let views = cast_views(input, "cast parse")?;
    let probe = classify_cast_shape(&views).map_err(|error| error.diagnostic())?;
    if views
        .get(probe.rparen + 1)
        .is_some_and(|next| next.kind == TokenKind::Punctuator && next.spelling == b"{")
    {
        return Err(CastError::Unsupported(
            "compound literal `(T){...}` is PA10/T08 territory, never a bare cast; deferred past M1 pending the type-name and initializer protocols",
        )
        .diagnostic());
    }
    Err(CastError::Unsupported(
        "cast deferred past M1 pending the T06 `ConversionOp` freeze and the PA10/PA04 type-name decision; geometry alone cannot distinguish `(T)x` from `(x)+1`",
    )
    .diagnostic())
}

/// Pure M1 conditional parse: classify the geometry, then reject.
///
/// Even a perfect `a?b:c` fails `Unsupported`: the three operands need
/// child-task recursion with await-all joins (T05 items A–C), deferred.
/// Right-associative nesting (`a?b:c?d:e`) folds in the future split.
///
/// Future pre-order shape rows (documented, NOT committed):
/// `Conditional(0)` → `Cond(1)`, `Then(2)`, `Else(3)`.
pub fn parse_conditional_expression(
    input: &PaCastInput,
) -> Result<Vec<CastNodeShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "conditional parse")?;
    let views = cast_views(input, "conditional parse")?;
    classify_conditional_shape(&views).map_err(|error| error.diagnostic())?;
    Err(CastError::Unsupported(
        "conditional deferred past M1 pending child-task recursion with await-all joins (T05 items A–C); right-associative `?:` tails fold in the future split",
    )
    .diagnostic())
}

/// Pure M1 comma parse: classify the geometry, then reject.
///
/// Even a perfect `a,b` fails `Unsupported`: the operands need child-task
/// recursion (T05 items A–C), and only the caller-bounded window keeps
/// PA18 call-argument and PA31 `for`-init commas out.
///
/// Future pre-order shape rows (documented, NOT committed):
/// `Comma(0)` → `Left(1)`, `Right(2)` (longer chains fold left).
pub fn parse_comma_expression(input: &PaCastInput) -> Result<Vec<CastNodeShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "comma parse")?;
    let views = cast_views(input, "comma parse")?;
    classify_comma_shape(&views).map_err(|error| error.diagnostic())?;
    Err(CastError::Unsupported(
        "comma deferred past M1 pending child-task recursion with await-all joins (T05 items A–C); call-argument and `for`-init commas stay caller-bounded (PA18/PA31)",
    )
    .diagnostic())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{NameId, TaskId, TokenId};
    use crate::task::TaskState;

    fn punct(spelling: &[u8]) -> CastToken {
        CastToken {
            kind: TokenKind::Punctuator,
            spelling: spelling.to_vec(),
        }
    }

    fn ident(spelling: &[u8]) -> CastToken {
        CastToken {
            kind: TokenKind::Identifier,
            spelling: spelling.to_vec(),
            // `CastToken` carries no interned name; spelling is enough.
        }
    }

    fn keyword(spelling: &[u8]) -> CastToken {
        CastToken {
            kind: TokenKind::Keyword,
            spelling: spelling.to_vec(),
        }
    }

    fn input_of(
        production: PaCastProduction,
        views: &[CastToken],
        state: TaskState,
    ) -> PaCastInput {
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
                ProjectedCastToken {
                    kind: view.kind,
                    name: match view.kind {
                        TokenKind::Identifier => Some(NameId::from_index(7)),
                        _ => None,
                    },
                    spelling,
                    pp_spelling,
                },
            );
        }
        PaCastInput {
            task: TaskId::from_index(0),
            state,
            production,
            tokens,
            bodies,
        }
    }

    fn running(production: PaCastProduction, views: &[CastToken]) -> PaCastInput {
        input_of(production, views, TaskState::Running)
    }

    #[test]
    fn cast_classifier_probes_paren_geometry() {
        let views = vec![punct(b"("), keyword(b"int"), punct(b")"), ident(b"x")];
        assert_eq!(classify_cast_shape(&views), Ok(CastProbe { rparen: 2 }));
    }

    #[test]
    fn cast_classifier_rejects_parenthesized_tail_as_non_cast() {
        // `(x)` with no operand suffix is geometry without a cast: the
        // classifier reports it, and the chip-level parse defers loudly.
        let views = vec![punct(b"("), ident(b"x"), punct(b")")];
        assert!(matches!(
            classify_cast_shape(&views),
            Err(CastError::Unsupported(_))
        ));
    }

    #[test]
    fn cast_parse_rejects_even_perfect_geometry_pending_conversion_freeze() {
        let views = vec![punct(b"("), keyword(b"int"), punct(b")"), ident(b"x")];
        let error = parse_cast_expression(&running(PaCastProduction::Cast, &views))
            .expect_err("M1 casts always defer");
        assert!(matches!(error, _ if error.code.group == DiagGroup::Unsupported));
        assert!(error.message.contains("ConversionOp"));
    }

    #[test]
    fn cast_parse_routes_compound_literal_to_pa10_territory() {
        let views = vec![
            punct(b"("),
            keyword(b"int"),
            punct(b")"),
            punct(b"{"),
            punct(b"1"),
            punct(b"}"),
        ];
        let error = parse_cast_expression(&running(PaCastProduction::Cast, &views))
            .expect_err("compound literal is never a bare cast");
        assert_eq!(error.code.group, DiagGroup::Unsupported);
        assert!(error.message.contains("compound literal"));
    }

    #[test]
    fn conditional_classifier_probes_question_colon() {
        let views = vec![
            ident(b"a"),
            punct(b"?"),
            ident(b"b"),
            punct(b":"),
            ident(b"c"),
        ];
        assert_eq!(
            classify_conditional_shape(&views),
            Ok(ConditionalProbe {
                question: 1,
                colon: 3
            })
        );
    }

    #[test]
    fn conditional_classifier_rejects_omitted_middle_as_mode_gated() {
        let views = vec![ident(b"a"), punct(b"?"), punct(b":"), ident(b"c")];
        let error = classify_conditional_shape(&views).expect_err("omitted middle defers");
        assert!(matches!(error, CastError::Unsupported(_)));
        assert!(error.diagnostic().message.contains("omitted-middle"));
    }

    #[test]
    fn conditional_parse_rejects_even_perfect_shape_pending_child_tasks() {
        let views = vec![
            ident(b"a"),
            punct(b"?"),
            ident(b"b"),
            punct(b":"),
            ident(b"c"),
        ];
        let error = parse_conditional_expression(&running(PaCastProduction::Conditional, &views))
            .expect_err("M1 conditionals always defer");
        assert_eq!(error.code.group, DiagGroup::Unsupported);
        assert!(error.message.contains("await-all"));
    }

    #[test]
    fn comma_classifier_probes_first_top_level_comma() {
        let views = vec![ident(b"a"), punct(b","), ident(b"b")];
        assert_eq!(classify_comma_shape(&views), Ok(CommaProbe { comma: 1 }));
    }

    #[test]
    fn comma_classifier_ignores_nested_call_commas() {
        // `f(a,b)`: the only commas sit inside parens, so no top-level
        // comma classifies — PA18 owns that window.
        let views = vec![
            ident(b"f"),
            punct(b"("),
            ident(b"a"),
            punct(b","),
            ident(b"b"),
            punct(b")"),
        ];
        assert!(matches!(
            classify_comma_shape(&views),
            Err(CastError::Unsupported(_))
        ));
    }

    #[test]
    fn comma_parse_rejects_even_perfect_shape_pending_child_tasks() {
        let views = vec![ident(b"a"), punct(b","), ident(b"b")];
        let error = parse_comma_expression(&running(PaCastProduction::Comma, &views))
            .expect_err("M1 commas always defer");
        assert_eq!(error.code.group, DiagGroup::Unsupported);
        assert!(error.message.contains("caller-bounded"));
    }

    #[test]
    fn compute_fails_unsupported_for_all_productions() {
        let chip = PaCastChip;
        let cases = [
            (
                PaCastProduction::Cast,
                vec![punct(b"("), keyword(b"int"), punct(b")"), ident(b"x")],
            ),
            (
                PaCastProduction::Conditional,
                vec![
                    ident(b"a"),
                    punct(b"?"),
                    ident(b"b"),
                    punct(b":"),
                    ident(b"c"),
                ],
            ),
            (
                PaCastProduction::Comma,
                vec![ident(b"a"), punct(b","), ident(b"b")],
            ),
        ];
        for (production, views) in cases {
            let out = match production {
                PaCastProduction::Cast => chip.compute_cast(&running(production, &views)),
                PaCastProduction::Conditional => {
                    chip.compute_conditional(&running(production, &views))
                }
                PaCastProduction::Comma => chip.compute_comma(&running(production, &views)),
            };
            let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
                panic!("{production:?} must fail Unsupported in M1");
            };
            assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
        }
    }

    #[test]
    fn compute_rejects_non_running_task() {
        let chip = PaCastChip;
        let views = vec![ident(b"a"), punct(b","), ident(b"b")];
        let mut input = running(PaCastProduction::Comma, &views);
        input.state = TaskState::Ready;
        let out = chip.compute_comma(&input);
        let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
            panic!("non-running task must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }

    #[test]
    fn missing_token_record_is_loud_task_fault() {
        let chip = PaCastChip;
        let id = TokenId::from_index(0);
        let mut tokens = Vec::new();
        tokens.push(id);
        // `bodies` deliberately omits the token: loud `Task/4`, never a
        // guessed spelling.
        let input = PaCastInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            production: PaCastProduction::Cast,
            tokens,
            bodies: BTreeMap::new(),
        };
        let out = chip.compute_cast(&input);
        let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
            panic!("missing token must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }

    #[test]
    fn candidate_registration_is_next_free() {
        assert_eq!(PA21_CANDIDATE_LOCAL, 27);
        assert_eq!(PA23_CANDIDATE_LOCAL, 28);
        assert_eq!(PA25_CANDIDATE_LOCAL, 29);
        assert_eq!(PA_CAST_CANDIDATE_CHIP, ChipId(58));
        for kind in [pa21_task_kind(), pa23_task_kind(), pa25_task_kind()] {
            assert_eq!(kind.group(), TaskGroup::PARSE);
        }
        assert_eq!(pa21_task_kind().local(), 27);
        assert_eq!(pa23_task_kind().local(), 28);
        assert_eq!(pa25_task_kind().local(), 29);
        assert_ne!(pa21_task_kind(), pa23_task_kind());
        assert_ne!(pa23_task_kind(), pa25_task_kind());
        assert_eq!(
            PaCastProduction::of_kind(pa21_task_kind()),
            Some(PaCastProduction::Cast)
        );
        assert_eq!(
            PaCastProduction::of_kind(pa23_task_kind()),
            Some(PaCastProduction::Conditional)
        );
        assert_eq!(
            PaCastProduction::of_kind(pa25_task_kind()),
            Some(PaCastProduction::Comma)
        );
    }

    #[test]
    fn manifest_is_ack_only_and_deterministic() {
        let manifest = PaCastChip.manifest();
        assert_eq!(manifest.id, PA_CAST_CANDIDATE_CHIP);
        assert!(manifest.writes.is_empty());
        assert!(manifest.deterministic);
        assert_eq!(manifest.task_kinds.len(), 3);
        // Zero-sized worker: no hidden semantic state.
        assert_eq!(std::mem::size_of::<PaCastChip>(), 0);
    }
}
