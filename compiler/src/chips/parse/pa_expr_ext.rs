// ============================================================================
// chips/parse/pa_expr_ext.rs — T05 PA15 initializer + PA26 generic-selection
// workers (CANDIDATE draft, unfrozen).
//
// M1 subset is the scalar literal initializer (`= <int>`): the `=`
// spelling (resolved through the committed PP token) and the operand kind
// decide, and the operand needs exactly one committed literal. Literal
// *values* are never interpreted (initialization semantics are T08-owned).
// All other initializer windows — braced lists, designators (`.x`, `[2]`),
// nesting, trailing commas, string initializers, multi-token expression
// initializers (`= 2+3`, whose `2+3` is PA22-owned) — are explicit
// `Unsupported`. PA26 has no M1 shape at all (`_Generic` is C11 and never
// appears in the M1 source): every selection window is `Unsupported`.
//
// NOTE (DEFECT-EXT-01): the canonical M1 source
// `int main(void){return 2+3;}` exercises no initializer (initialization
// chips are recorded M1 gaps, cf. `M1-CL-04`), so the `= <int>` shape is
// the work-order-declared M1 subset, not a canonical-source fixture. The
// parallel PA15 draft (`pa_init.rs`, integrator-owned reconciliation)
// names the same `= <int>` boundary; this file matches it so parallel
// drafts do not define contradictory M1 subsets.
//
// Pure validate, no writes: success completes `Ack` and the wiring layer
// (integrator-owned) consumes the documented [`InitNodeShape`] rows. No
// child task is enqueued here, so no node-link or child-payload schema is
// invented.
//
// Candidate registration (integrator-owned freeze; NOT frozen by this
// file): `PA15_TASK_KIND_CANDIDATE` is `PARSE` local 27
// (`parse.initializer`), `PA26_TASK_KIND_CANDIDATE` is `PARSE` local 28
// (`parse.generic_selection`) — the first two free locals after
// `PARSE_RECOVERY` (local 26); `PA_EXPR_EXT_CANDIDATE_CHIP` is `ChipId(58)`,
// the next free ID after `SE_FUNC_CHIP` (`ChipId(57)`). Kind-registry,
// stage, routing, and allowlist rows (Ack-only needs none) live with the
// integrator in `task.rs` / `manifest.rs` / routing. This file claims the
// candidate values only so the manifest and dispatch agree without touching
// shared schemas.
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA15 InitializerParseChip + PA26
//   GenericSelectionParseChip / PARSE (candidate `PARSE` locals 27/28,
//   candidate `ChipId(58)`; see the candidate registration below).
//   Single-chip file per the worker template: one zero-sized unit struct,
//   `PaExprExtChip`, serves both productions and dispatches on the
//   production field (the dispatched task's `TaskKind`, mirroring
//   `ContinuationRecord.production`).
// Contract version/hash: T05 PENDING (bundle unfrozen); T01
//   `t01-c01-c06/39` current. This file freezes nothing and authorizes no
//   integration: kind/stage/routing rows are integrator-owned.
// Owned files: ONLY this file (`compiler/src/chips/parse/pa_expr_ext.rs`).
//   Never edits `mod.rs`, bus/task/ids/manifest schemas, or registrations
//   (see docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: candidate `parse.initializer`
//   (`PARSE` local 27) with payload = exactly the two initializer token
//   refs in source order (`= <int>` for M1); candidate
//   `parse.generic_selection` (`PARSE` local 28) with payload = the
//   selection token refs in source order (leading `_Generic` included when
//   present); result = `Ack` on the M1 `= <int>` shape, else typed `Fail`
//   (no next-cursor carrier: OB-30 open, caller holds the cursor).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (scalar `= <int>` for PA15;
//   PA26 has no M1 shape — every selection is explicit `Unsupported`,
//   never a pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` + `lex.literals` (`/11`), `pp.tokens` (`/11`,
//   punctuator/keyword ground truth), `names.entries` (`/11`). Pure query:
//   no bus mutation; workers emit `Proposal`s only.
// Writes: NONE (Ack/ Fail-only). No `AppendRecords`, no `StorePatch`, no
//   `Enqueue`. The [`InitNodeShape`] / [`GenericNodeShape`] rows below copy
//   the frozen `NodeRecord` field shapes chip-locally for
//   documentation/future split only; they are NOT committed.
// Dependencies: T04 committed tokens + interned spellings + committed PP
//   spellings (producer facts only; never calls another chip). Initializer
//   value semantics are T08-owned; generic association typing is
//   T06/T07-owned.
// Preconditions: dispatched task is `Running` with a PA15/PA26 candidate
//   kind; payload carries `Token` refs only; referenced tokens are
//   committed.
// Transition states: Running → Complete(Ack) on the exact M1 `= <int>`
//   shape; Running → Fail (`Unsupported` group) on any other window;
//   Running → Fail (`Task` group code 4) on not-running/missing-record
//   protocol faults.
// Algorithm obligations: PA15 accepts exactly `= <int>` (kinds
//   `[Punctuator, Integer]`, `=` spelling resolved through the committed
//   PP token, integer literal-backed) and defers every other class loudly
//   (see [`parse_initializer`]); PA26 requires the leading `_Generic`
//   keyword and defers every selection loudly, naming the missing-`:
//   defect specifically (see [`parse_generic_selection`]). No literal
//   value is interpreted.
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = dispatch-then-project-then-compute; all paths
//   yield proposals; no `panic!`/`unwrap!`/`expect` on any path.
// Error codes / recovery: `Unsupported/1` for non-M1 windows (explicit
//   rejection, stays in the denominator); `Task/4` for protocol faults;
//   `Protocol/1` for unknown-task/wrong-kind scaffolding. No recovery sync
//   is attempted here (PA38 owns it).
// Required fixtures: in-file unit tests below (pure cores + compute arms +
//   dispatch + candidate registration). The shared acceptance suite is
//   integrator-owned (no suite exists for these productions yet).
// Integration acceptance: NOT FROZEN (candidate kinds/stage/layer rows
//   open; cursor carrier OB-30 open; init-tree/generic node schemas and
//   parent linkage open; child-task nesting open).
// Known unsupported: scalar initializers (`= 3`, T08 territory), brace
//   initializers (`{1, 2}`, nesting, trailing comma), designators (`.x`,
//   `[2]`), string-literal initializers, every `_Generic` selection
//   (multiple associations, `default`, missing colon), GNU range
//   designators (all deferred; all fail `Unsupported`).
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-EXT-01 (M1 subset): the canonical M1 source exercises no
//     initializer, so `= <int>` is the work-order-declared M1 subset
//     rather than a canonical-source fixture (initialization chips are
//     recorded M1 gaps, cf. `M1-CL-04`). PA26 has no M1 shape at all.
//     The parallel PA15 draft names the same `= <int>` boundary;
//     reconciliation is integrator-owned.
//   DEFECT-EXT-02 (cursor/nodes): `next_cursor` has no frozen carrier
//     (T05 item G.4 OB-30 open) and no node is appended here (committed
//     parse nodes stay PA01-owned until the full catalog split). No
//     scalar/tuple variant is invented here.
//   DEFECT-EXT-03 (nesting): the full design recurses through child tasks
//     and continuations with await-all joins (T05 items A–C); this shell
//     enqueues nothing. Brace nesting depth and parse-depth accounting
//     (`ParseDepthExceeded`, item D) stay future integration checks.
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, LiteralId, NameId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Candidate `PARSE` local code for `parse.initializer` (PA15).
///
/// Local 27 — the first free local after `PARSE_RECOVERY` (local 26).
/// Candidate only; the kind-registry row is integrator-owned.
pub const PA15_CANDIDATE_LOCAL: u16 = 27;

/// Candidate `PARSE` local code for `parse.generic_selection` (PA26).
///
/// Local 28 — the next free local after the PA15 candidate.
/// Candidate only; the kind-registry row is integrator-owned.
pub const PA26_CANDIDATE_LOCAL: u16 = 28;

/// Candidate chip ID for the fused PA15/PA26 worker.
///
/// The next free ID after `SE_FUNC_CHIP` (`ChipId(57)`).
/// Candidate only; the manifest row is integrator-owned.
pub const PA_EXPR_EXT_CANDIDATE_CHIP: ChipId = ChipId(58);

/// Candidate task kind for PA15 initializer parsing.
///
/// Resolved from [`PA15_CANDIDATE_LOCAL`]; `None` only if the local exceeds
/// `LOCAL_MAX` (impossible for 27 — callers treat `None` as "kind not
/// registered", never as a silent pass).
pub const PA15_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA15_CANDIDATE_LOCAL);

/// Candidate task kind for PA26 generic-selection parsing.
///
/// Resolved from [`PA26_CANDIDATE_LOCAL`]; `None` only if the local exceeds
/// `LOCAL_MAX` (impossible for 28 — callers treat `None` as "kind not
/// registered", never as a silent pass).
pub const PA26_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA26_CANDIDATE_LOCAL);

/// Production served by [`PaExprExtChip`]: the dispatch target resolved from
/// the dispatched task's kind (the production field).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaExprExtProduction {
    /// PA15 initializer production (all windows deferred past M1).
    Initializer,
    /// PA26 generic-selection production (all windows deferred past M1).
    GenericSelection,
}

impl PaExprExtProduction {
    /// Resolve the production for a dispatched task kind.
    pub fn of_kind(kind: TaskKind) -> Option<Self> {
        if Some(kind) == PA15_TASK_KIND_CANDIDATE {
            Some(Self::Initializer)
        } else if Some(kind) == PA26_TASK_KIND_CANDIDATE {
            Some(Self::GenericSelection)
        } else {
            None
        }
    }
}

/// Candidate task kinds claimed by this worker's manifest.
///
/// Panic-free: unresolvable candidates (impossible for locals 27/28) are
/// simply absent rather than fabricated.
fn candidate_kinds() -> Vec<TaskKind> {
    let mut kinds = Vec::new();
    if let Some(kind) = PA15_TASK_KIND_CANDIDATE {
        kinds.push(kind);
    }
    if let Some(kind) = PA26_TASK_KIND_CANDIDATE {
        kinds.push(kind);
    }
    kinds
}

/// Chip-local copy of the frozen `NodeRecord` field shapes for one
/// initializer subtree node.
///
/// Same field names/order as `NodeRecord` (`parent`, `first_token`,
/// `last_token`, literal presence) so the future split can lift it
/// verbatim, except positions are indices into the task's own token slice
/// and links are pre-order node indices rather than committed IDs; the kind
/// is the `NodeKind` variant name string rather than imported shared
/// schema. Shapes are built by the pure path and documented — they are NOT
/// appended to any arena by this shell. The M1 `= <int>` shape builds
/// `InitScalar(0)` → `IntLiteral(1)` in pre-order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitNodeShape {
    /// Fixed node-kind name (mirrors `NodeKind`, e.g. `"InitList"`).
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

/// Chip-local copy of the frozen `NodeRecord` field shapes for one generic-
/// selection subtree node.
///
/// Same conventions as [`InitNodeShape`]: built by the pure path and
/// documented, NOT committed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericNodeShape {
    /// Fixed node-kind name (mirrors `NodeKind`, e.g. `"GenericSelection"`).
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

/// Why a token slice is not an accepted initializer / generic selection.
///
/// Maps to `Unsupported` (explicit rejection, stays in the denominator).
/// Numeric codes are chip-local candidates; the frozen code table is a
/// T01 `/6` detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtError {
    /// A well-formed (or malformed) shape outside the M1 subset.
    Unsupported(&'static str),
}

impl ExtError {
    /// Map to the structured failure diagnostic for this rejection.
    pub fn diagnostic(&self) -> DiagnosticDraft {
        match self {
            Self::Unsupported(message) => DiagnosticDraft::unsupported(*message),
        }
    }
}

/// One projected token for the pure grammar cores.
///
/// `spelling` is the effective spelling: intern bytes for identifiers and
/// keywords, committed PP bytes for punctuators/integers. The projector
/// resolves which side applies; the pure cores only compare bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Effective spelling bytes.
    pub spelling: Vec<u8>,
    /// Interned name, if any (carried for future name accessors).
    pub name: Option<NameId>,
}

/// One projected token: kind plus resolved spellings.
///
/// Identifier/keyword spellings resolve through the intern table
/// (`spelling`); punctuator/number spellings resolve through the committed
/// PP token (`pp_spelling`), because committed C punctuator records carry
/// no interned name. Either side is empty only when the backing record is
/// missing, which the computation reports loudly instead of guessing.
#[derive(Clone, Debug)]
pub struct ProjectedExtToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Interned spelling bytes for named tokens.
    pub spelling: Vec<u8>,
    /// Committed PP spelling bytes (operator/number ground truth).
    pub pp_spelling: Vec<u8>,
}

impl ProjectedExtToken {
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

/// Narrow projection for the PA15 initializer computation.
#[derive(Clone, Debug)]
pub struct PaInitInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (M1 requires exactly two).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedExtToken>,
    /// Committed literal by token ID (exactly one, for the operand token).
    pub literals_by_token: BTreeMap<TokenId, LiteralId>,
}

/// Narrow projection for the PA26 generic-selection computation.
#[derive(Clone, Debug)]
pub struct PaGenericInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order.
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedExtToken>,
}

/// Projected token stream for one generic task: token IDs in source order,
/// present token bodies, dispatch state.
type ExtTokenProjection = (
    Vec<TokenId>,
    BTreeMap<TokenId, ProjectedExtToken>,
    TaskState,
);

fn project_ext_tokens(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
    want: TaskKind,
    label: &str,
) -> Result<ExtTokenProjection, DiagnosticDraft> {
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
                ProjectedExtToken {
                    kind: body.kind,
                    name: body.name,
                    spelling,
                    pp_spelling,
                },
            );
        }
    }
    Ok((tokens, bodies, record.state.clone()))
}

/// Build the narrow PA15 projection for one dispatched task.
///
/// Payload convention (candidate): exactly the two initializer token refs
/// in source order (`= <int>` for M1; no EOF, no surrounding tokens).
/// A wrong kind, a non-token ref, or an unknown task is a caller protocol
/// fault; arity/content is `compute`'s job so non-M1 input surfaces as
/// `Unsupported`, never as a silent pass.
pub fn project_pa_init_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaInitInput, DiagnosticDraft> {
    let Some(want) = PA15_TASK_KIND_CANDIDATE else {
        return Err(protocol_fault("initializer kind is not registered"));
    };
    let (tokens, bodies, state) = project_ext_tokens(bus, task, want, "initializer parse")?;
    // Committed literals indexed by their token back-link (ID order).
    let mut literals_by_token = BTreeMap::new();
    for (id, literal) in bus.arenas.literals.iter() {
        if let Some(token) = literal.token {
            literals_by_token.insert(token, id);
        }
    }
    Ok(PaInitInput {
        task,
        state,
        tokens,
        bodies,
        literals_by_token,
    })
}

/// Build the narrow PA26 projection for one dispatched task.
///
/// Payload convention (candidate): the generic-selection token refs in
/// source order. A wrong kind, a non-token ref, or an unknown task is a
/// caller protocol fault; window content is `compute`'s job so non-M1 input
/// surfaces as `Unsupported`, never as a silent pass.
pub fn project_pa_generic_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaGenericInput, DiagnosticDraft> {
    let Some(want) = PA26_TASK_KIND_CANDIDATE else {
        return Err(protocol_fault("generic-selection kind is not registered"));
    };
    let (tokens, bodies, state) = project_ext_tokens(bus, task, want, "generic-selection parse")?;
    Ok(PaGenericInput {
        task,
        state,
        tokens,
        bodies,
    })
}

/// The fused T05 PA15/PA26 worker (candidate shell: classify + loud defer).
pub struct PaExprExtChip;

impl Worker for PaExprExtChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA_EXPR_EXT_CANDIDATE_CHIP,
            chip_name: "PaExprExtChip",
            group: TaskGroup::PARSE,
            task_kinds: candidate_kinds(),
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Lex, "literals"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Names, "entries"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/src/chips/parse/pa_expr_ext.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let kind = match bus.arenas.tasks.get(task) {
            Ok(record) => record.kind,
            Err(_) => {
                return vec![fail(
                    task,
                    protocol_fault(format!("expr-ext parse of unknown task {}", task.index())),
                )];
            }
        };
        match PaExprExtProduction::of_kind(kind) {
            Some(PaExprExtProduction::Initializer) => match project_pa_init_input(bus, task) {
                Ok(input) => self.compute_init(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            Some(PaExprExtProduction::GenericSelection) => {
                match project_pa_generic_input(bus, task) {
                    Ok(input) => self.compute_generic(&input),
                    Err(diagnostic) => vec![fail(task, diagnostic)],
                }
            }
            None => vec![fail(
                task,
                protocol_fault(format!(
                    "expr-ext parse task {} has unexpected kind {}",
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

/// Project the committed token bodies into pure-core views.
///
/// A missing committed token or a token with no spelling is a loud
/// `Task/4` fault, never a guessed default.
fn project_views(
    task: TaskId,
    tokens: &[TokenId],
    bodies: &BTreeMap<TokenId, ProjectedExtToken>,
    label: &str,
) -> Result<Vec<ExtToken>, DiagnosticDraft> {
    let mut views = Vec::with_capacity(tokens.len());
    for id in tokens {
        let Some(projected) = bodies.get(id) else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "{label} task {} reads missing token {}",
                    task.index(),
                    id.index()
                ),
            ));
        };
        let spelling = projected.effective_spelling().to_vec();
        if spelling.is_empty() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "{label} task {} reads token {} with no spelling",
                    task.index(),
                    id.index()
                ),
            ));
        }
        views.push(ExtToken {
            kind: projected.kind,
            spelling,
            name: projected.name,
        });
    }
    Ok(views)
}

impl PaExprExtChip {
    /// Pure PA15 computation over the narrow projection (no bus access).
    ///
    /// M1 accepts exactly `= <int>` and completes `Ack` with the two-node
    /// subtree shape documented (not committed). Anything else is an
    /// explicit fail; protocol faults use `Task/4`.
    pub fn compute_init(&self, input: &PaInitInput) -> Vec<Proposal> {
        match check_running(input.task, &input.state, "initializer parse")
            .and_then(|()| {
                project_views(
                    input.task,
                    &input.tokens,
                    &input.bodies,
                    "initializer parse",
                )
            })
            .and_then(|views| {
                let operand_has_literal = input
                    .tokens
                    .last()
                    .is_some_and(|id| input.literals_by_token.contains_key(id));
                parse_initializer(&views, operand_has_literal).map(|_| ())
            }) {
            Ok(()) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }

    /// Pure PA26 computation over the narrow projection (no bus access).
    ///
    /// The M1 subset is empty, so every window fails `Unsupported` today;
    /// the `Ok → Ack` arm is structural for the future split. Protocol
    /// faults use `Task/4`.
    pub fn compute_generic(&self, input: &PaGenericInput) -> Vec<Proposal> {
        match check_running(input.task, &input.state, "generic-selection parse")
            .and_then(|()| {
                project_views(
                    input.task,
                    &input.tokens,
                    &input.bodies,
                    "generic-selection parse",
                )
            })
            .and_then(|views| parse_generic_selection(&views).map(|_| ()))
        {
            Ok(()) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Pure PA15 initializer parse: validate `= <int>` and build its local
/// subtree shape; defer every other class loudly. No bus access; no
/// shared-schema invention.
///
/// The `=` operator resolves against the committed PP bytes; the operand
/// must be an `Integer` token backed by one committed literal (the PA16
/// primary fact, validated here at the token level with no child task).
/// `operand_has_literal` is resolved by the caller from the committed
/// literal store; a missing literal is a loud `Task/4` fault rather than
/// a zero. Initialization semantics stay T08-owned: this chip certifies
/// syntax only. A multi-token expression operand (`= 2+3`) is PA22
/// territory and fails the arity gate as explicit `Unsupported`; folding
/// it is wiring-layer composition through child tasks, deferred.
///
/// Pre-order shape rows: `InitScalar(0)` → `IntLiteral(1)`.
pub fn parse_initializer(
    tokens: &[ExtToken],
    operand_has_literal: bool,
) -> Result<Vec<InitNodeShape>, DiagnosticDraft> {
    if tokens.len() != 2 {
        let first = tokens.first();
        if let Some(first) = first {
            if first.kind == TokenKind::Punctuator && first.spelling == b"{" {
                return Err(ExtError::Unsupported(
                    "non-M1 initializer: brace initializer (nesting, trailing comma) deferred past M1; semantics handed to T08",
                )
                .diagnostic());
            }
            if first.kind == TokenKind::Punctuator
                && (first.spelling == b"." || first.spelling == b"[")
            {
                return Err(ExtError::Unsupported(
                    "non-M1 initializer: designator (`.x`, `[2]`) deferred past M1; semantics handed to T08",
                )
                .diagnostic());
            }
        }
        // Any interior `.` designator at any depth is still designator
        // territory, named precisely for the denominator.
        if tokens
            .iter()
            .any(|token| token.kind == TokenKind::Punctuator && token.spelling == b".")
        {
            return Err(ExtError::Unsupported(
                "non-M1 initializer: designator (`.x`, `[2]`) deferred past M1; semantics handed to T08",
            )
            .diagnostic());
        }
        if tokens.is_empty() {
            return Err(ExtError::Unsupported(
                "non-M1 initializer: empty initializer window; M1 requires exactly `= <int>` (two tokens)",
            )
            .diagnostic());
        }
        return Err(ExtError::Unsupported(
            "non-M1 initializer: exactly `= <int>` (two tokens) required; braced, designator, and expression initializers are deferred past M1",
        )
        .diagnostic());
    }
    if tokens[0].kind != TokenKind::Punctuator
        || tokens[0].spelling != b"="
        || tokens[1].kind != TokenKind::Integer
    {
        return Err(ExtError::Unsupported(
            "non-M1 initializer: operand/operator kinds must be `` `=` `<int>` ``",
        )
        .diagnostic());
    }
    if !operand_has_literal {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "initializer parse reads an operand integer with no committed literal",
        ));
    }
    Ok(vec![
        InitNodeShape {
            kind_name: "InitScalar",
            parent: None,
            first_token: 0,
            last_token: 1,
            has_literal: false,
        },
        InitNodeShape {
            kind_name: "IntLiteral",
            parent: Some(0),
            first_token: 1,
            last_token: 1,
            has_literal: true,
        },
    ])
}

/// Pure PA26 generic-selection classification: name the deferred shape loudly.
///
/// Every window is past M1 (DEFECT-EXT-01), so every path returns
/// `Unsupported`; the leading-keyword and missing-colon checks exist so the
/// denominator distinguishes "not a selection" from the T05-specified
/// "missing colon" defect. No bus access; no shared-schema invention;
/// association typing stays T06/T07-owned.
pub fn parse_generic_selection(
    tokens: &[ExtToken],
) -> Result<Vec<GenericNodeShape>, DiagnosticDraft> {
    if tokens.is_empty() {
        return Err(ExtError::Unsupported(
            "non-M1 generic selection: empty selection window; `_Generic` selections are deferred past M1",
        )
        .diagnostic());
    }
    let first = &tokens[0];
    if !(first.kind == TokenKind::Keyword && first.spelling == b"_Generic") {
        return Err(ExtError::Unsupported(
            "non-M1 generic selection: window does not open with `_Generic`; generic selections are deferred past M1",
        )
        .diagnostic());
    }
    let has_colon = tokens
        .iter()
        .any(|token| token.kind == TokenKind::Punctuator && token.spelling == b":");
    if !has_colon {
        return Err(ExtError::Unsupported(
            "non-M1 generic selection: association missing `:`; generic selections are deferred past M1",
        )
        .diagnostic());
    }
    Err(ExtError::Unsupported(
        "non-M1 generic selection: controlling expression and type/default associations deferred past M1; typing handed to T06/T07",
    )
    .diagnostic())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::NameId;

    fn ident(spelling: &[u8]) -> ExtToken {
        ExtToken {
            kind: TokenKind::Identifier,
            spelling: spelling.to_vec(),
            name: Some(NameId::from_index(7)),
        }
    }

    fn keyword(spelling: &[u8]) -> ExtToken {
        ExtToken {
            kind: TokenKind::Keyword,
            spelling: spelling.to_vec(),
            name: None,
        }
    }

    fn punct(spelling: &[u8]) -> ExtToken {
        ExtToken {
            kind: TokenKind::Punctuator,
            spelling: spelling.to_vec(),
            name: None,
        }
    }

    fn int(spelling: &[u8]) -> ExtToken {
        ExtToken {
            kind: TokenKind::Integer,
            spelling: spelling.to_vec(),
            name: None,
        }
    }

    fn input_init_of(views: &[ExtToken], state: TaskState) -> PaInitInput {
        let mut tokens = Vec::new();
        let mut bodies = BTreeMap::new();
        let mut literals_by_token = BTreeMap::new();
        for (index, view) in views.iter().enumerate() {
            let id = TokenId::from_index(index as u32);
            tokens.push(id);
            let (spelling, pp_spelling) = match view.kind {
                TokenKind::Identifier | TokenKind::Keyword => (view.spelling.clone(), Vec::new()),
                _ => (Vec::new(), view.spelling.clone()),
            };
            bodies.insert(
                id,
                ProjectedExtToken {
                    kind: view.kind,
                    name: view.name,
                    spelling,
                    pp_spelling,
                },
            );
            if view.kind == TokenKind::Integer {
                literals_by_token.insert(id, LiteralId::from_index(index as u32));
            }
        }
        PaInitInput {
            task: TaskId::from_index(0),
            state,
            tokens,
            bodies,
            literals_by_token,
        }
    }

    fn input_generic_of(views: &[ExtToken], state: TaskState) -> PaGenericInput {
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
                ProjectedExtToken {
                    kind: view.kind,
                    name: view.name,
                    spelling,
                    pp_spelling,
                },
            );
        }
        PaGenericInput {
            task: TaskId::from_index(0),
            state,
            tokens,
            bodies,
        }
    }

    #[test]
    fn init_m1_scalar_validates_with_shape() {
        let views = vec![punct(b"="), int(b"3")];
        let shapes = parse_initializer(&views, true).expect("M1 `= <int>` validates");
        assert_eq!(shapes.len(), 2);
        assert_eq!(shapes[0].kind_name, "InitScalar");
        assert_eq!(shapes[0].parent, None);
        assert_eq!((shapes[0].first_token, shapes[0].last_token), (0, 1));
        assert!(!shapes[0].has_literal);
        assert_eq!(shapes[1].kind_name, "IntLiteral");
        assert_eq!(shapes[1].parent, Some(0));
        assert_eq!((shapes[1].first_token, shapes[1].last_token), (1, 1));
        assert!(shapes[1].has_literal);
    }

    #[test]
    fn init_scalar_without_literal_is_defect() {
        let views = vec![punct(b"="), int(b"3")];
        let error = parse_initializer(&views, false).expect_err("missing literal is loud");
        assert_eq!(error.code.group, DiagGroup::Task);
    }

    #[test]
    fn init_brace_nesting_and_trailing_comma_are_unsupported() {
        let nested = vec![
            punct(b"{"),
            punct(b"{"),
            int(b"1"),
            punct(b"}"),
            punct(b"}"),
        ];
        let trailing = vec![punct(b"{"), int(b"1"), punct(b","), punct(b"}")];
        for views in [nested, trailing] {
            let error = parse_initializer(&views, true).expect_err("braces defer past M1");
            assert_eq!(error.code.group, DiagGroup::Unsupported);
        }
    }

    #[test]
    fn init_designators_are_unsupported() {
        let dot = vec![punct(b"."), ident(b"x"), punct(b"="), int(b"1")];
        let bracket = vec![punct(b"["), int(b"2"), punct(b"]"), punct(b"="), int(b"1")];
        for views in [dot, bracket] {
            let error = parse_initializer(&views, true).expect_err("designators defer past M1");
            assert_eq!(error.code.group, DiagGroup::Unsupported);
        }
    }

    #[test]
    fn init_expression_operand_and_empty_are_unsupported() {
        // `= 2+3` is PA22 territory: the arity gate rejects it here and the
        // wiring layer composes it through child tasks, deferred.
        let expr = vec![punct(b"="), int(b"2"), punct(b"+"), int(b"3")];
        let error = parse_initializer(&expr, true).expect_err("expression defers");
        assert_eq!(error.code.group, DiagGroup::Unsupported);

        let error = parse_initializer(&[], true).expect_err("empty window is non-M1");
        assert_eq!(error.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn generic_non_selection_is_unsupported() {
        let views = vec![ident(b"x"), punct(b"+"), int(b"1")];
        let error = parse_generic_selection(&views).expect_err("not a selection");
        assert_eq!(error.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn generic_missing_colon_is_unsupported() {
        // `_Generic ( x , int )` — no association colon.
        let views = vec![
            keyword(b"_Generic"),
            punct(b"("),
            ident(b"x"),
            punct(b","),
            keyword(b"int"),
            punct(b")"),
        ];
        let error = parse_generic_selection(&views).expect_err("missing colon defers");
        assert_eq!(error.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn generic_well_formed_is_still_unsupported() {
        // `_Generic ( x , int : 1 , default : 0 )` — well-formed but past M1.
        let views = vec![
            keyword(b"_Generic"),
            punct(b"("),
            ident(b"x"),
            punct(b","),
            keyword(b"int"),
            punct(b":"),
            int(b"1"),
            punct(b","),
            keyword(b"default"),
            punct(b":"),
            int(b"0"),
            punct(b")"),
        ];
        let error = parse_generic_selection(&views).expect_err("past M1");
        assert_eq!(error.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn compute_acks_m1_and_fails_loud() {
        let chip = PaExprExtChip;
        // The M1 `= <int>` shape completes Ack (no writes).
        let m1 = vec![punct(b"="), int(b"3")];
        let ok = chip.compute_init(&input_init_of(&m1, TaskState::Running));
        assert_eq!(
            ok,
            vec![Proposal::Complete {
                task: TaskId::from_index(0),
                value: ResultValue::Ack,
            }]
        );

        // Every other content window fails Unsupported (stays in the
        // denominator).
        let brace = vec![punct(b"{"), int(b"1"), punct(b"}")];
        let out = chip.compute_init(&input_init_of(&brace, TaskState::Running));
        let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
            panic!("brace initializer must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);

        let generic = vec![keyword(b"_Generic"), punct(b"("), punct(b")")];
        let out = chip.compute_generic(&input_generic_of(&generic, TaskState::Running));
        let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
            panic!("generic selection must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);

        // A literal-backed `= <int>` with the literal stripped is a loud
        // Task/4 fault, never a zero.
        let mut stripped = input_init_of(&m1, TaskState::Running);
        stripped
            .literals_by_token
            .remove(stripped.tokens.last().expect("two-token M1 shape"));
        let out = chip.compute_init(&stripped);
        let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
            panic!("missing literal must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);

        // Non-running tasks fail on the Task channel, never as Unsupported.
        let out = chip.compute_init(&input_init_of(&m1, TaskState::Ready));
        let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
            panic!("non-running task must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);

        // A missing committed token is a loud Task/4 fault, not a guess.
        let mut missing = input_init_of(&m1, TaskState::Running);
        missing.bodies.remove(&missing.tokens[0]);
        let out = chip.compute_init(&missing);
        let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
            panic!("missing token must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }

    #[test]
    fn pure_cores_are_deterministic() {
        let m1 = vec![punct(b"="), int(b"3")];
        let first = parse_initializer(&m1, true).expect("M1 validates");
        let second = parse_initializer(&m1, true).expect("M1 validates");
        assert_eq!(first, second);

        let generic = vec![keyword(b"_Generic"), punct(b"("), punct(b")")];
        let first = parse_generic_selection(&generic).expect_err("generic defers");
        let second = parse_generic_selection(&generic).expect_err("generic defers");
        assert_eq!(first, second);
    }

    #[test]
    fn production_dispatch_and_candidate_registration() {
        let init_kind = PA15_TASK_KIND_CANDIDATE.expect("local 27 is valid");
        let generic_kind = PA26_TASK_KIND_CANDIDATE.expect("local 28 is valid");
        assert_eq!(
            PaExprExtProduction::of_kind(init_kind),
            Some(PaExprExtProduction::Initializer)
        );
        assert_eq!(
            PaExprExtProduction::of_kind(generic_kind),
            Some(PaExprExtProduction::GenericSelection)
        );
        assert_eq!(init_kind.group(), TaskGroup::PARSE);
        assert_eq!(init_kind.local(), 27);
        assert_eq!(generic_kind.local(), 28);
        assert_eq!(PA15_CANDIDATE_LOCAL, 27);
        assert_eq!(PA26_CANDIDATE_LOCAL, 28);
        assert_eq!(PA_EXPR_EXT_CANDIDATE_CHIP, ChipId(58));

        // The fused worker claims exactly the two candidate kinds.
        let chip = PaExprExtChip;
        let manifest = chip.manifest();
        assert_eq!(manifest.task_kinds.len(), 2);
        assert!(manifest.task_kinds.contains(&init_kind));
        assert!(manifest.task_kinds.contains(&generic_kind));
        assert!(manifest.writes.is_empty());
        assert!(manifest.deterministic);
    }
}
