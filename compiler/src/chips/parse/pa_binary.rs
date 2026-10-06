// ============================================================================
// chips/parse/pa_binary.rs — T05 PA16 primary + PA22 binary workers
// (Wave 3 slice 10, `/35`).
//
// M1 exercises only the integer-constant primary (`<int>`) and the single
// `+` fold (`<int> + <int>`): the operand/operator kinds decide. Any other
// primary or binary shape — including `-`, `*`, multi-operator chains, or
// the 4-token `2 + +3` sequence (whose `+3` operand is PA20-owned) — is an
// explicit typed `Fail`, never a guessed default. The operator spelling
// resolves through the committed PP token; each integer leaf needs exactly
// one committed literal. Literal *values* are never interpreted
// (T07/T08-owned).
//
// Pure validate, no writes: success completes `Ack` and the wiring layer
// (the PA01 expression loop, integrator-owned) consumes the documented
// [`ExprNodeShape`] rows. No child task is enqueued here, so no node-link
// or child-payload schema is invented.
//
// Frozen registration (integrator-owned): `PA16_TASK_KIND` aliases
// `TaskKind::PARSE_PRIMARY` (`PARSE` local 22, `parse.primary`) and
// `PA22_TASK_KIND` aliases `TaskKind::PARSE_BINARY` (`PARSE` local 23,
// `parse.binary`); `PA16_CHIP` is `crate::manifest::PA16_CHIP`
// (`ChipId(48)`), the kind-registry rows live in
// `TaskKindRegistry::pa_expr_slice()`, the stage-2 rows in
// `STAGE_ASSIGNMENT`, the routed layer is 2, there is no store-owner
// allowlist row (Ack-only, zero writes), and the acceptance test is
// `compiler/tests/c35_expr.rs`.
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA16 PrimaryExpressionChip + PA22 BinaryExpressionChip
//   / PARSE (frozen kinds `PARSE` locals 22/23, `PA16_CHIP = ChipId(48)`;
//   see the frozen registration below).
//   Single-chip file per the worker template: one zero-sized unit struct,
//   `PaBinaryChip`, serves both productions and dispatches on the production
//   field (the dispatched task's `TaskKind`, mirroring
//   `ContinuationRecord.production`).
// Contract version/hash: T05 frozen by `/35`; T01 `/35` current. The PA decl
//   slice froze `parse.external_declaration` + `parse.specifiers` +
//   `parse.declarator` + `parse.block` + `parse.return` (locals 17–21);
//   this slice freezes the next two `PARSE` locals (22–23) with
//   registry/stage/routing rows (integrator-owned freeze).
// Owned files: ONLY this file (`compiler/src/chips/parse/pa_binary.rs`).
//   Never edits `mod.rs`, bus/task/ids/manifest schemas, or registrations
//   (see docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: `parse.primary`
//   (`PARSE` local 22, `PA16_TASK_KIND`) with payload = exactly one
//   `RecordRef::Token`; `parse.binary` (`PARSE` local 23,
//   `PA22_TASK_KIND`) with payload = exactly three `RecordRef::Token`s in
//   source order (`<int> + <int>`); result = `Ack` (no next-cursor carrier:
//   OB-30 open, caller holds the cursor).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (integer-constant operands with
//   a single `+` fold); every other primary/binary form is explicit
//   `Unsupported` (never a pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` + `lex.literals` (`/11`), `pp.tokens` (`/11`, punctuator
//   ground truth for `+`), `names.entries` (`/11`). Pure query: no bus
//   mutation; workers emit `Proposal`s only.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no
//   `Enqueue`. Committed `Primary`/`BinaryExpression` nodes stay owned by
//   the PA01 TU slice until the full catalog split; a second `parse.nodes`
//   writer would need an allowlist row this file must not invent. The
//   `ExprNodeShape` rows below copy the frozen `NodeRecord` field shapes
//   chip-locally for documentation/future split only; they are NOT
//   committed.
// Dependencies: T04 committed tokens + interned spellings + decoded
//   literals (producer facts only; never calls another chip).
// Preconditions: dispatched task is `Running` with a PA16/PA22 frozen payload carries `Token` refs only; referenced tokens and integer
//   literals are committed.
// Transition states: Running → Complete(Ack) on the exact M1 shapes;
//   Running → Fail (`Unsupported` group) on any other shape; Running →
//   Fail (`Task` group code 4) on not-running/missing-record protocol
//   faults.
// Algorithm obligations: PA16 accepts exactly one `Integer` token backed
//   by one committed literal; PA22 accepts exactly
//   `<int> + <int>` (kinds `[Integer, Punctuator, Integer]`, middle
//   spelling `+` resolved through the committed PP token, both integers
//   literal-backed) as a single left-associative precedence-climb step at
//   `min_bp = 0` (see [`binary_precedence`]). Literal *values* are not
//   interpreted (folding is T07/T08-owned).
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = dispatch-then-project-then-compute; all paths
//   yield proposals.
// Error codes / recovery: `Unsupported/1` for non-M1 shapes (explicit
//   rejection, stays in the denominator); `Task/4` for protocol faults;
//   `Protocol/1` for unknown-task/wrong-kind scaffolding. No recovery sync
//   is attempted here (PA38 owns it).
// Required fixtures: `compiler/tests/c35_expr.rs` (this slice).
// Integration acceptance: FROZEN by `/35` (kind registration in
//   `TaskKindRegistry`, stage-2 rows in `STAGE_ASSIGNMENT`, routed layer 2,
//   no allowlist row — Ack-only; cursor carrier, parent linkage, child-task
//   nesting open).
// Known unsupported: identifiers/strings/parenthesized primaries (PA16
//   territory past M1), all binary operators other than M1 `+`
//   (`-`, `*`, `/`, `%`, shifts, comparisons, bitwise, logical),
//   multi-operator chains (`a+b*c`), right-associative operators,
//   `2 + +3` as a single PA22 shape (4 tokens: rejected here; the `+3`
//   operand is PA20-owned, composition is wiring-layer work).
//
// Frozen registration (integrator-owned): `PA16_TASK_KIND` is `PARSE`
//   local 22 (`parse.primary`), `PA22_TASK_KIND` is `PARSE` local 23
//   (`parse.binary`) — the first two free locals after `PARSE_RETURN`
//   (local 21); `PA16_CHIP` is `ChipId(48)`, the next free ID after
//   `PA28_CHIP` (`ChipId(47)`). Rows live in `TaskKindRegistry::pa_expr_slice()`,
//   `STAGE_ASSIGNMENT` (stage 2), and the routed layer 2.
//
// M1 boundary (see T05 item G): `2+3` is the PA22 shape (accepted);
//   `2+ +3` is NOT a PA22 shape (4 tokens: explicit `Unsupported` here) —
//   its `+3` operand is the PA20 shape owned by `pa_unary.rs`, and folding
//   `2 + (+3)` is wiring-layer composition through child tasks, deferred.
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-4 (nesting): the full design climbs through child tasks and
//     continuations with await-all joins (T05 items A–C); M1 validates the
//     fixed shapes in one tick with no `Enqueue`, `AwaitChildren`, or
//     `Progress`. Multi-operator precedence chains and parse-depth
//     accounting (`ParseDepthExceeded`, item D) stay future integration
//     checks.
//   DEFECT-6 (ambiguity): any input outside the two exact M1 shapes fails
//     `Unsupported` instead of guessing (e.g. `2-3` is not treated as
//     "binary minus", `2 3` is not juxtaposition, a missing literal is a
//     loud `Task/4` fault rather than a zero).
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, NameId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA16_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Frozen task kind for PA16 primary-expression parsing.
///
/// `PARSE` local 22 — the first free local after `PARSE_RETURN` (local 21);
/// registered `Frozen` in `TaskKindRegistry::pa_expr_slice()`.
pub const PA16_TASK_KIND: TaskKind = TaskKind::PARSE_PRIMARY;

/// Frozen task kind for PA22 binary-expression parsing.
///
/// `PARSE` local 23 — the next free local after the PA16 kind;
/// registered `Frozen` in `TaskKindRegistry::pa_expr_slice()`.
pub const PA22_TASK_KIND: TaskKind = TaskKind::PARSE_BINARY;

/// Production served by [`PaBinaryChip`]: the dispatch target resolved from
/// the dispatched task's kind (the production field).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaBinaryProduction {
    /// PA16 primary-expression production (`<int>`).
    Primary,
    /// PA22 binary-expression production (`<int> + <int>`).
    Binary,
}

impl PaBinaryProduction {
    /// Resolve the production for a dispatched task kind.
    pub fn of_kind(kind: TaskKind) -> Option<Self> {
        if kind == PA16_TASK_KIND {
            Some(Self::Primary)
        } else if kind == PA22_TASK_KIND {
            Some(Self::Binary)
        } else {
            None
        }
    }
}

/// Chip-local copy of the frozen `NodeRecord` field shapes for one M1
/// expression subtree node.
///
/// Same field names/order as `NodeRecord` (`parent`, `first_token`,
/// `last_token`, literal presence) so the future split can lift it
/// verbatim, except positions are indices into the task's own token slice
/// and links are pre-order node indices rather than committed IDs; the kind
/// is the `NodeKind` variant name string rather than imported shared
/// schema. Shapes are built by the pure path and documented — they are NOT
/// appended to any arena by this Ack-only slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExprNodeShape {
    /// Fixed node-kind name (mirrors `NodeKind`, e.g. `"BinaryAdd"`).
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

/// Why a token slice is not the M1 primary/binary shape.
///
/// Maps to `Unsupported` (explicit rejection, stays in the denominator).
/// Numeric codes are chip-local candidates; the frozen code table is a
/// T01 `/6` detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryError {
    /// A well-formed (or malformed) shape outside the M1 subset.
    Unsupported(&'static str),
}

impl BinaryError {
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
pub struct ProjectedExprToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Interned spelling bytes for named tokens.
    pub spelling: Vec<u8>,
    /// Committed PP spelling bytes (operator/number ground truth).
    pub pp_spelling: Vec<u8>,
}

impl ProjectedExprToken {
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

/// Narrow projection for the PA16 primary computation.
#[derive(Clone, Debug)]
pub struct PaPrimaryInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (M1 requires exactly one).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedExprToken>,
    /// Committed literal by token ID (exactly one for the integer token).
    pub literals_by_token: BTreeMap<TokenId, LiteralId>,
}

/// Narrow projection for the PA22 binary computation.
#[derive(Clone, Debug)]
pub struct PaBinaryInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (M1 requires exactly three).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedExprToken>,
    /// Committed literal by token ID (exactly one per integer token).
    pub literals_by_token: BTreeMap<TokenId, LiteralId>,
}

/// Projected token stream for one primary/binary task: token IDs in source
/// order, present token bodies, committed literals by token, dispatch state.
type ExprTokenProjection = (
    Vec<TokenId>,
    BTreeMap<TokenId, ProjectedExprToken>,
    BTreeMap<TokenId, LiteralId>,
    TaskState,
);

fn project_expr_tokens(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
    want: TaskKind,
    label: &str,
) -> Result<ExprTokenProjection, DiagnosticDraft> {
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
                ProjectedExprToken {
                    kind: body.kind,
                    name: body.name,
                    spelling,
                    pp_spelling,
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

/// Build the narrow PA16 projection for one dispatched task.
///
/// Payload convention (frozen): exactly the one primary token ref.
/// A wrong kind, a non-token ref, or an unknown task is a caller protocol
/// fault; arity/content is `compute`'s job so non-M1 input surfaces as
/// `Unsupported`, never as a silent pass.
pub fn project_pa_primary_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaPrimaryInput, DiagnosticDraft> {
    let (tokens, bodies, literals_by_token, state) =
        project_expr_tokens(bus, task, PA16_TASK_KIND, "primary parse")?;
    Ok(PaPrimaryInput {
        task,
        state,
        tokens,
        bodies,
        literals_by_token,
    })
}

/// Build the narrow PA22 projection for one dispatched task.
///
/// Payload convention (frozen): exactly the three binary token refs in
/// source order (`<int> + <int>` for M1; no EOF, no surrounding tokens).
/// A wrong kind, a non-token ref, or an unknown task is a caller protocol
/// fault; arity/content is `compute`'s job so non-M1 input surfaces as
/// `Unsupported`, never as a silent pass.
pub fn project_pa_binary_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaBinaryInput, DiagnosticDraft> {
    let (tokens, bodies, literals_by_token, state) =
        project_expr_tokens(bus, task, PA22_TASK_KIND, "binary parse")?;
    Ok(PaBinaryInput {
        task,
        state,
        tokens,
        bodies,
        literals_by_token,
    })
}

/// The fused T05 PA16/PA22 worker (M1 slice: fixed-shape validate + `Ack`).
pub struct PaBinaryChip;

impl Worker for PaBinaryChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA16_CHIP,
            chip_name: "PaBinaryChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![PA16_TASK_KIND, PA22_TASK_KIND],
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
            tests: vec!["compiler/tests/c35_expr.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let kind = match bus.arenas.tasks.get(task) {
            Ok(record) => record.kind,
            Err(_) => {
                return vec![fail(
                    task,
                    protocol_fault(format!("binary parse of unknown task {}", task.index())),
                )];
            }
        };
        match PaBinaryProduction::of_kind(kind) {
            Some(PaBinaryProduction::Primary) => match project_pa_primary_input(bus, task) {
                Ok(input) => self.compute_primary(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            Some(PaBinaryProduction::Binary) => match project_pa_binary_input(bus, task) {
                Ok(input) => self.compute_binary(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            None => vec![fail(
                task,
                protocol_fault(format!(
                    "binary-parse task {} has unexpected kind {}",
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

fn check_integer_literal(
    task: TaskId,
    token: TokenId,
    literals_by_token: &BTreeMap<TokenId, LiteralId>,
) -> Result<(), DiagnosticDraft> {
    if !literals_by_token.contains_key(&token) {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "parse task {} reads missing literal for token {}",
                task.index(),
                token.index()
            ),
        ));
    }
    Ok(())
}

impl PaBinaryChip {
    /// Pure PA16 computation over the narrow projection (no bus access).
    ///
    /// M1 accepts exactly one integer-constant token and completes `Ack`
    /// with the one-node leaf shape documented (not committed). Anything
    /// else is an explicit fail; protocol faults use `Task/4`.
    pub fn compute_primary(&self, input: &PaPrimaryInput) -> Vec<Proposal> {
        match parse_primary(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }

    /// Pure PA22 computation over the narrow projection (no bus access).
    ///
    /// M1 accepts exactly `<int> + <int>` and completes `Ack` with the
    /// three-node subtree shape documented (not committed). Anything else
    /// is an explicit fail; protocol faults use `Task/4`.
    pub fn compute_binary(&self, input: &PaBinaryInput) -> Vec<Proposal> {
        match parse_binary_expression(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Precedence-climb table: left/right binding powers for one binary
/// operator spelling.
///
/// Returns `(left_bp, right_bp)`; left-associative operators satisfy
/// `right_bp == left_bp + 1` (the climb recurses with `right_bp` as the
/// next minimum). M1 exercises only the additive `+` step `(10, 11)`.
/// Intended full-catalog levels (deferred; all return `None` here):
/// multiplicative `*`/`/`/`%` at `(20, 21)`, shifts at `(30, 31)`,
/// comparisons below shifts, bitwise `&`/`^`/`|` below comparisons,
/// logical `&&`/`||` at the bottom. The deferred rows stay in the
/// denominator as explicit `Unsupported` until the catalog split lands
/// them with child-task recursion.
pub fn binary_precedence(op: &[u8]) -> Option<(u8, u8)> {
    match op {
        b"+" => Some((10, 11)),
        _ => None,
    }
}

/// Pure M1 primary parse: validate one integer constant and build its
/// local leaf shape. No bus access; no shared-schema invention.
///
/// Pre-order shape rows: `IntLiteral(0)`.
pub fn parse_primary(input: &PaPrimaryInput) -> Result<Vec<ExprNodeShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "primary parse")?;
    if input.tokens.len() != 1 {
        return Err(BinaryError::Unsupported(
            "non-M1 primary: exactly one integer-constant token required",
        )
        .diagnostic());
    }
    let id = input.tokens[0];
    let Some(projected) = input.bodies.get(&id) else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("parse reads missing token {}", id.index()),
        ));
    };
    if projected.kind != TokenKind::Integer {
        return Err(BinaryError::Unsupported(
            "non-M1 primary: only an integer constant is supported",
        )
        .diagnostic());
    }
    check_integer_literal(input.task, id, &input.literals_by_token)?;
    Ok(vec![ExprNodeShape {
        kind_name: "IntLiteral",
        parent: None,
        first_token: 0,
        last_token: 0,
        has_literal: true,
    }])
}

/// Pure M1 binary parse: validate `<int> + <int>` as one left-associative
/// precedence-climb step and build its local subtree shape. No bus access;
/// no shared-schema invention.
///
/// The operator's [`binary_precedence`] must resolve and the climb runs at
/// `min_bp = 0`; M1 accepts exactly the single `+` fold below. A second
/// operator (`a+b*c`), a non-`+` operator (`2-3`), a 4-token `2 + +3`
/// shape, or a missing literal all fail loudly instead of guessing.
///
/// Pre-order shape rows: `BinaryAdd(0)` → `IntLiteral(1)`, `IntLiteral(2)`.
pub fn parse_binary_expression(
    input: &PaBinaryInput,
) -> Result<Vec<ExprNodeShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "binary parse")?;
    if input.tokens.len() != 3 {
        return Err(BinaryError::Unsupported(
            "non-M1 binary: exactly `<int> + <int>` (three tokens) required",
        )
        .diagnostic());
    }
    let want = [
        TokenKind::Integer,
        TokenKind::Punctuator,
        TokenKind::Integer,
    ];
    for (position, id) in input.tokens.iter().enumerate() {
        let Some(projected) = input.bodies.get(id) else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("parse reads missing token {}", id.index()),
            ));
        };
        if projected.kind != want[position] {
            return Err(BinaryError::Unsupported(
                "non-M1 binary: operand/operator kinds must be `<int> <punct> <int>`",
            )
            .diagnostic());
        }
    }
    let op_id = input.tokens[1];
    let op = input.bodies[&op_id].effective_spelling().to_vec();
    // Single climb step at `min_bp = 0`: only `+` (left 10, right 11)
    // meets the threshold, so the fixed three-token shape takes the fold
    // exactly once. Chained operators would recurse here with `right_bp`
    // as the next minimum; with no further tokens the climb terminates
    // after this fold (longer chains fail the arity gate above as
    // explicit `Unsupported`).
    let Some((_left_bp, _right_bp)) = binary_precedence(&op) else {
        return Err(
            BinaryError::Unsupported("non-M1 binary operator: only `+` is supported").diagnostic(),
        );
    };
    check_integer_literal(input.task, input.tokens[0], &input.literals_by_token)?;
    check_integer_literal(input.task, input.tokens[2], &input.literals_by_token)?;
    Ok(vec![
        ExprNodeShape {
            kind_name: "BinaryAdd",
            parent: None,
            first_token: 0,
            last_token: 2,
            has_literal: false,
        },
        ExprNodeShape {
            kind_name: "IntLiteral",
            parent: Some(0),
            first_token: 0,
            last_token: 0,
            has_literal: true,
        },
        ExprNodeShape {
            kind_name: "IntLiteral",
            parent: Some(0),
            first_token: 2,
            last_token: 2,
            has_literal: true,
        },
    ])
}
