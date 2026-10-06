// ============================================================================
// chips/parse/pa_unary.rs — T05 PA20 unary-expression worker
// (Wave 3 slice 10, `/35`).
//
// M1 exercises only unary `+`/`-` over an integer constant (`+<int>` /
// `-<int>`): the operator spelling (resolved through the committed PP
// token) and the operand kind decide. Dereference/address-of,
// logical/bitwise negation, pre/post `++`/`--`, `sizeof`/`_Alignof`,
// casts (PA21 owns the cast recursion protocol), compound literals, and
// GNU `__extension__` operand forms are all explicit `Unsupported`.
// The operand needs exactly one committed literal; literal *values* are
// never interpreted (sign application is T07/T08-owned).
//
// Pure validate, no writes: success completes `Ack` and the wiring layer
// (the PA01 expression loop, integrator-owned) consumes the documented
// [`UnaryNodeShape`] rows. No child task is enqueued here, so no node-link
// or child-payload schema is invented.
//
// Frozen registration (integrator-owned): `PA20_TASK_KIND` aliases
// `TaskKind::PARSE_UNARY` (`PARSE` local 24, `parse.unary`);
// `PA20_CHIP` is `crate::manifest::PA20_CHIP` (`ChipId(49)`), the
// kind-registry row lives in `TaskKindRegistry::pa_expr_slice()`, the
// stage-2 row in `STAGE_ASSIGNMENT`, the routed layer is 2, there is no
// store-owner allowlist row (Ack-only, zero writes), and the acceptance
// test is `compiler/tests/c35_expr.rs`.
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA20 UnaryExpressionChip / PARSE (frozen kind
//   `PARSE` local 24, `PA20_CHIP = ChipId(49)`; see the frozen
//   registration below).
// Contract version/hash: T05 frozen by `/35`; T01 `/35` current. The PA decl
//   slice froze `parse.external_declaration` + `parse.specifiers` +
//   `parse.declarator` + `parse.block` + `parse.return` (locals 17–21),
//   and the sibling slice `pa_binary.rs` freezes locals 22–23;
//   this slice freezes the next `PARSE` local (24) with
//   registry/stage/routing rows (integrator-owned freeze).
// Owned files: ONLY this file (`compiler/src/chips/parse/pa_unary.rs`).
//   Never edits `mod.rs`, bus/task/ids/manifest schemas, or registrations
//   (see docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: `parse.unary`
//   (`PARSE` local 24, `PA20_TASK_KIND`), payload = exactly two
//   `RecordRef::Token`s in source order (`+<int>` / `-<int>` for M1);
//   result = `Ack` (no next-cursor carrier: OB-30 open, caller holds the
//   cursor).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (unary `+`/`-` over an integer
//   constant); every other unary form is explicit `Unsupported` (never a
//   pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` + `lex.literals` (`/11`), `pp.tokens` (`/11`, punctuator
//   ground truth for the operator), `names.entries` (`/11`). Pure query: no
//   bus mutation; workers emit `Proposal`s only.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no
//   `Enqueue`. The committed `UnaryExpression` node stays owned by the PA01
//   TU slice until the full catalog split; a second `parse.nodes` writer
//   would need an allowlist row this file must not invent. The
//   `UnaryNodeShape` rows below copy the frozen `NodeRecord` field shapes
//   chip-locally for documentation/future split only; they are NOT
//   committed.
// Dependencies: T04 committed tokens + interned spellings + decoded
//   literals (producer facts only; never calls another chip). The operand
//   is a PA16 primary fact in the full design; M1 validates the token
//   slice directly with no child task.
// Preconditions: dispatched task is `Running` with the PA20 frozen
//   kind; payload carries `Token` refs only; referenced tokens and the
//   integer literal are committed.
// Transition states: Running → Complete(Ack) on the exact M1 shapes;
//   Running → Fail (`Unsupported` group) on any other shape; Running →
//   Fail (`Task` group code 4) on not-running/missing-record protocol
//   faults.
// Algorithm obligations: accept exactly [`UnaryOp::Plus`] /
//   [`UnaryOp::Minus`] applied to one `Integer` token backed by one
//   committed literal (see [`parse_unary`]). Literal *values* are not
//   interpreted (folding/sign application is T07/T08-owned).
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = project-then-compute; all paths yield
//   proposals.
// Error codes / recovery: `Unsupported/1` for non-M1 shapes (explicit
//   rejection, stays in the denominator); `Task/4` for protocol faults;
//   `Protocol/1` for unknown-task/wrong-kind scaffolding. No recovery sync
//   is attempted here (PA38 owns it).
// Required fixtures: `compiler/tests/c35_expr.rs` (this slice).
// Integration acceptance: FROZEN by `/35` (kind registration in
//   `TaskKindRegistry`, stage-2 row in `STAGE_ASSIGNMENT`, routed layer 2,
//   no allowlist row — Ack-only; cursor carrier, parent linkage, cast
//   recursion protocol open).
// Known unsupported: dereference/address-of (`*p`, `&x`), logical/bitwise
//   negation (`!x`, `~x`), pre/post `++`/`--` (`*++p`), `sizeof`
//   expression/type ambiguity, `_Alignof`, casts (PA21 owns the cast
//   recursion protocol), compound literals, GNU `__extension__` operand
//   forms (all deferred; all fail `Unsupported`).
//
// Frozen registration (integrator-owned): `PA20_TASK_KIND` is `PARSE`
//   local 24 (`parse.unary`) — the next free local after the
//   `pa_binary.rs` kinds (locals 22–23); `PA20_CHIP` is `ChipId(49)`,
//   the next free ID after `PA16_CHIP` (`ChipId(48)`). Rows live in
//   `TaskKindRegistry::pa_expr_slice()`, `STAGE_ASSIGNMENT` (stage 2),
//   and the routed layer 2.
//
// M1 boundary (see T05 item G): `+3` / `-3` is the PA20 shape (accepted);
//   the 4-token `2 + +3` sequence is NOT a single-chip shape — PA22 rejects
//   it (arity gate in `pa_binary.rs`) and this chip accepts only its `+3`
//   suffix; folding `2 + (+3)` is wiring-layer composition through child
//   tasks, deferred.
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-4 (nesting): the full design recurses into the operand (and
//     into PA21 for casts, T06 `sizeof` type queries) through child tasks
//     and continuations with await-all joins (T05 items A–C); M1 validates
//     the fixed two-token shape in one tick with no `Enqueue`,
//     `AwaitChildren`, or `Progress`. Parse-depth accounting
//     (`ParseDepthExceeded`, item D) stays a future integration check.
//   DEFECT-6 (ambiguity): any input outside the two exact M1 shapes fails
//     `Unsupported` instead of guessing (e.g. `*p` is not read as unary
//     plus, `sizeof 3` is not an operand fetch, a missing literal is a
//     loud `Task/4` fault rather than a zero).
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, NameId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA20_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Frozen task kind for PA20 unary-expression parsing.
///
/// `PARSE` local 24 — the next free local after the `pa_binary.rs`
/// kinds (locals 22–23); registered `Frozen` in
/// `TaskKindRegistry::pa_expr_slice()`.
pub const PA20_TASK_KIND: TaskKind = TaskKind::PARSE_UNARY;

/// M1-supported unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    /// Unary plus (`+x`; M1 boundary operand).
    Plus,
    /// Unary minus (`-x`).
    Minus,
}

impl UnaryOp {
    /// Resolve the operator from its committed PP spelling bytes.
    pub fn of_spelling(spelling: &[u8]) -> Option<Self> {
        match spelling {
            b"+" => Some(Self::Plus),
            b"-" => Some(Self::Minus),
            _ => None,
        }
    }

    /// Canonical spelling bytes of the operator.
    pub fn spelling(self) -> &'static [u8] {
        match self {
            Self::Plus => b"+",
            Self::Minus => b"-",
        }
    }

    /// Fixed node-kind name for the operator node (mirrors `NodeKind`).
    pub fn kind_name(self) -> &'static str {
        match self {
            Self::Plus => "UnaryPlus",
            Self::Minus => "UnaryMinus",
        }
    }
}

/// Chip-local copy of the frozen `NodeRecord` field shapes for one M1
/// unary subtree node.
///
/// Same field names/order as `NodeRecord` (`parent`, `first_token`,
/// `last_token`, literal presence) so the future split can lift it
/// verbatim, except positions are indices into the task's own token slice
/// and links are pre-order node indices rather than committed IDs; the kind
/// is the `NodeKind` variant name string rather than imported shared
/// schema. Shapes are built by the pure path and documented — they are NOT
/// appended to any arena by this Ack-only slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnaryNodeShape {
    /// Fixed node-kind name (mirrors `NodeKind`, e.g. `"UnaryPlus"`).
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

/// Why a token slice is not the M1 unary shape.
///
/// Maps to `Unsupported` (explicit rejection, stays in the denominator).
/// Numeric codes are chip-local candidates; the frozen code table is a
/// T01 `/6` detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryError {
    /// A well-formed (or malformed) shape outside the M1 subset.
    Unsupported(&'static str),
}

impl UnaryError {
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
pub struct ProjectedUnaryToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Interned spelling bytes for named tokens.
    pub spelling: Vec<u8>,
    /// Committed PP spelling bytes (operator/number ground truth).
    pub pp_spelling: Vec<u8>,
}

impl ProjectedUnaryToken {
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

/// Narrow projection for the PA20 unary computation.
#[derive(Clone, Debug)]
pub struct PaUnaryInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (M1 requires exactly two).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedUnaryToken>,
    /// Committed literal by token ID (exactly one, for the operand token).
    pub literals_by_token: BTreeMap<TokenId, LiteralId>,
}

/// Build the narrow PA20 projection for one dispatched task.
///
/// Payload convention (frozen): exactly the two unary token refs in
/// source order (`+<int>` / `-<int>` for M1; no EOF, no surrounding
/// tokens). A wrong kind, a non-token ref, or an unknown task is a caller
/// protocol fault; arity/content is `compute`'s job so non-M1 input
/// surfaces as `Unsupported`, never as a silent pass.
pub fn project_pa_unary_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaUnaryInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("unary parse of unknown task {}", task.index())))?;
    if record.kind != PA20_TASK_KIND {
        return Err(protocol_fault(format!(
            "unary task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "unary task {} payload must carry tokens only",
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
                ProjectedUnaryToken {
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
    Ok(PaUnaryInput {
        task,
        state: record.state.clone(),
        tokens,
        bodies,
        literals_by_token,
    })
}

/// The T05 PA20 unary-expression worker (M1 slice: fixed-shape validate +
/// `Ack`).
pub struct PaUnaryChip;

impl Worker for PaUnaryChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA20_CHIP,
            chip_name: "PaUnaryChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![PA20_TASK_KIND],
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
        let input = match project_pa_unary_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PaUnaryChip {
    /// Pure PA20 computation over the narrow projection (no bus access).
    ///
    /// M1 accepts exactly `+<int>` / `-<int>` and completes `Ack` with the
    /// two-node subtree shape documented (not committed). Anything else is
    /// an explicit fail; protocol faults use `Task/4`.
    pub fn compute(&self, input: &PaUnaryInput) -> Vec<Proposal> {
        match parse_unary(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Pure M1 unary parse: validate `+<int>` / `-<int>` and build its local
/// subtree shape. No bus access; no shared-schema invention.
///
/// The operator resolves through [`UnaryOp::of_spelling`] against the
/// committed PP bytes; the operand must be an `Integer` token backed by
/// one committed literal (the PA16 primary fact, validated here at the
/// token level with no child task). Sign application is T07/T08-owned:
/// this chip certifies syntax only.
///
/// Pre-order shape rows: `UnaryPlus(0)` → `IntLiteral(1)` (or the
/// `UnaryMinus` counterpart).
pub fn parse_unary(input: &PaUnaryInput) -> Result<Vec<UnaryNodeShape>, DiagnosticDraft> {
    if input.state != TaskState::Running {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("unary task {} is not running", input.task.index()),
        ));
    }
    if input.tokens.len() != 2 {
        return Err(UnaryError::Unsupported(
            "non-M1 unary: exactly `<op> <int>` (two tokens) required",
        )
        .diagnostic());
    }
    let mut missing = Vec::new();
    for id in &input.tokens {
        if !input.bodies.contains_key(id) {
            missing.push(id.index());
        }
    }
    if !missing.is_empty() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("parse reads missing tokens {missing:?}"),
        ));
    }
    let op_id = input.tokens[0];
    let operand_id = input.tokens[1];
    if input.bodies[&op_id].kind != TokenKind::Punctuator
        || input.bodies[&operand_id].kind != TokenKind::Integer
    {
        return Err(UnaryError::Unsupported(
            "non-M1 unary: operand/operator kinds must be `<punct> <int>`",
        )
        .diagnostic());
    }
    let Some(op) = UnaryOp::of_spelling(input.bodies[&op_id].effective_spelling()) else {
        return Err(UnaryError::Unsupported(
            "non-M1 unary operator: only `+` and `-` are supported",
        )
        .diagnostic());
    };
    if !input.literals_by_token.contains_key(&operand_id) {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "parse task {} reads missing literal for token {}",
                input.task.index(),
                operand_id.index()
            ),
        ));
    }
    Ok(vec![
        UnaryNodeShape {
            kind_name: op.kind_name(),
            parent: None,
            first_token: 0,
            last_token: 1,
            has_literal: false,
        },
        UnaryNodeShape {
            kind_name: "IntLiteral",
            parent: Some(0),
            first_token: 1,
            last_token: 1,
            has_literal: true,
        },
    ])
}
