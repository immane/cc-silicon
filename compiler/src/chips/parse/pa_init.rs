// ============================================================================
// chips/parse/pa_init.rs — T05 PA15 InitializerParseChip worker (DRAFT)
//
// DRAFT status: NEW file only; no registration freeze, no `mod.rs` edit,
// no schema edit. Candidate kind/chip IDs below are chip-local and do NOT
// claim frozen rows. Do not wire this file without an integrator freeze
// (PARSE local 27 / ChipId(58) may collide with parallel drafts).
//
// M1 exercises only the scalar literal initializer (`= <int>`): the `=`
// spelling (resolved through the committed PP token) and the operand kind
// decide. Braced lists, designators (`.x`, `[2]`), nesting, trailing
// commas, string initializers, and multi-token expression initializers
// (`= 2+3`, whose `2+3` is PA22-owned) are all explicit `Unsupported`.
// The operand needs exactly one committed literal; literal *values* are
// never interpreted (initialization semantics are T08-owned).
//
// Pure validate, no writes: success completes `Ack` and the wiring layer
// (integrator-owned) consumes the documented [`InitNodeShape`] rows. No
// child task is enqueued here, so no node-link or child-payload schema is
// invented.
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA15 InitializerParseChip / PARSE (CANDIDATE
//   `PARSE` local 27, `PA15_CANDIDATE_CHIP = ChipId(58)`; see the candidate
//   registration below — NOT frozen, integrator-owned).
// Contract version/hash: T05 PENDING; T01 `/36` current. This file claims
//   the PA15 `Cursor → InitTree` row only: scalar/braces/designator
//   recursion entry; semantics handed to T08.
// Owned files: ONLY this file (`compiler/src/chips/parse/pa_init.rs`).
//   Never edits `mod.rs`, bus/task/ids/manifest schemas, or registrations
//   (see docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: CANDIDATE `parse.initializer`
//   (`PARSE` local 27), payload = exactly the two initializer token refs
//   in source order (`= <int>` for M1); result = `Ack` (no InitTree
//   carrier and no next-cursor carrier: OB-30 open, caller holds the
//   cursor; the InitTree append stays wiring-layer owned).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (scalar `= <int>`); every other
//   initializer form is explicit `Unsupported` (never a pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` + `lex.literals` (`/11`), `pp.tokens` (`/11`, punctuator
//   ground truth for `=`), `names.entries` (`/11`). Pure query: no bus
//   mutation; workers emit `Proposal`s only.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no
//   `Enqueue`. The committed InitTree node stays wiring-layer owned until
//   the full catalog split; a second `parse.nodes` writer would need an
//   allowlist row this file must not invent. The [`InitNodeShape`] rows
//   below copy the frozen `NodeRecord` field shapes chip-locally for
//   documentation/future split only; they are NOT committed.
// Dependencies: T04 committed tokens + decoded literals (producer facts
//   only; never calls another chip). The `2+3` operand of a wider scalar
//   initializer is a PA22 fact in the full design; M1 validates the token
//   slice directly with no child task.
// Preconditions: dispatched task is `Running` with the PA15 candidate kind;
//   payload carries `Token` refs only; referenced tokens and the integer
//   literal are committed.
// Transition states: Running → Complete(Ack) on the exact M1 shape;
//   Running → Fail (`Unsupported` group) on any other shape; Running →
//   Fail (`Task` group code 4) on not-running/missing-record protocol
//   faults.
// Algorithm obligations: accept exactly `= <int>` (one `Integer` token
//   backed by one committed literal; see [`parse_initializer`]). Literal
//   *values* are not interpreted (folding/init semantics are T08-owned).
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = project-then-compute; all paths yield
//   proposals.
// Error codes / recovery: `Unsupported/1` for non-M1 shapes (explicit
//   rejection, stays in the denominator); `Task/4` for protocol faults;
//   `Protocol/1` for unknown-task/wrong-kind scaffolding. No recovery sync
//   is attempted here (PA38 owns it).
// Required fixtures: in-file unit tests below (this DRAFT ships no shared
//   acceptance suite; the wiring integrator owns the future suite).
// Integration acceptance: NOT FROZEN (no kind registration, no stage row,
//   no routed layer, no allowlist row — Ack-only; InitTree carrier, parent
//   linkage, child-task nesting open).
// Known unsupported: braced lists (`{1, 2}`), designators (`.x = 1`,
//   `[2] = 1`), nested initializers, trailing commas, string initializers,
//   multi-token expression initializers (`= 2+3`: the `2+3` operand is
//   PA22-owned, composition is wiring-layer work), aggregate/enum member
//   initializers (PA11/PA12/PA13 territory — see DEFECT below).
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-INIT-01 (scope): the dispatch brief names "PA11/PA12
//     initializer", but PA11 (AggregateSpecifierChip) and PA12
//     (EnumSpecifierChip) are specifier chips per T05, while the
//     initializer row is PA15 and the file is `pa_init.rs`. This DRAFT
//     implements the PA15 initializer row only. PA11/PA12 aggregate/enum
//     specifier shapes are NOT claimed here and remain open; any
//     struct/union/enum member initializer defers as `Unsupported`.
//   DEFECT-INIT-02 (nesting): the full design recurses into braced and
//     expression operands through child tasks and continuations with
//     await-all joins (T05 items A–C); M1 validates the fixed two-token
//     shape in one tick with no `Enqueue`, `AwaitChildren`, or
//     `Progress`. Parse-depth accounting (`ParseDepthExceeded`, item D)
//     stays a future integration check.
//   DEFECT-INIT-03 (ambiguity): any input outside the exact M1 shape fails
//     `Unsupported` instead of guessing (e.g. `= 2+3` is not folded here,
//     `{1}` is not read as scalar `1`, a missing literal is a loud
//     `Task/4` fault rather than a zero).
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, LiteralId, NameId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Candidate local code for the PA15 initializer task.
///
/// `PARSE` local 27 — the first free local after `PARSE_RECOVERY` (local
/// 26). DRAFT candidate only; the freeze is integrator-owned.
pub const PA15_CANDIDATE_LOCAL: u16 = 27;

/// Candidate chip ID for the PA15 worker.
///
/// The next free ID after `SE_FUNC_CHIP` (`ChipId(57)`). DRAFT candidate
/// only; the freeze is integrator-owned.
pub const PA15_CANDIDATE_CHIP: ChipId = ChipId(58);

/// Candidate task kind for the PA15 worker.
///
/// Resolved from [`PA15_CANDIDATE_LOCAL`]; `None` only if the local exceeds
/// `LOCAL_MAX` (impossible for 27 — the `expect` in [`pa15_task_kind`] is
/// unreachable and loud rather than silent).
pub const PA15_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA15_CANDIDATE_LOCAL);

/// Resolve the task kind (the PA15 candidate canonical).
pub fn pa15_task_kind() -> TaskKind {
    PA15_TASK_KIND_CANDIDATE.expect("PA15 candidate local 27 fits in TaskKind")
}

/// Why a token slice is not the M1 initializer shape.
///
/// Maps to `Unsupported` (explicit rejection, stays in the denominator).
/// Numeric codes are chip-local candidates; the frozen code table is a
/// T01 `/6` detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitError {
    /// A well-formed (or malformed) shape outside the M1 subset.
    Unsupported(&'static str),
}

impl InitError {
    /// Map to the structured failure diagnostic for this rejection.
    pub fn diagnostic(&self) -> DiagnosticDraft {
        match self {
            Self::Unsupported(message) => DiagnosticDraft::unsupported(*message),
        }
    }
}

/// Chip-local copy of the frozen `NodeRecord` field shapes for one M1
/// initializer subtree node.
///
/// Same field names/order as `NodeRecord` (`parent`, `first_token`,
/// `last_token`, literal presence) so the future split can lift it
/// verbatim, except positions are indices into the task's own token slice
/// and links are pre-order node indices rather than committed IDs; the kind
/// is the `NodeKind` variant name string rather than imported shared
/// schema. Shapes are built by the pure path and documented — they are NOT
/// appended to any arena by this Ack-only slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitNodeShape {
    /// Fixed node-kind name (mirrors `NodeKind`, e.g. `"InitScalar"`).
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

/// One projected token: kind plus resolved spellings.
///
/// Identifier/keyword spellings resolve through the intern table
/// (`spelling`); punctuator/number spellings resolve through the committed
/// PP token (`pp_spelling`), because committed C punctuator records carry
/// no interned name. Either side is empty only when the backing record is
/// missing, which the computation reports loudly instead of guessing.
#[derive(Clone, Debug)]
pub struct ProjectedInitToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Interned spelling bytes for named tokens.
    pub spelling: Vec<u8>,
    /// Committed PP spelling bytes (operator/number ground truth).
    pub pp_spelling: Vec<u8>,
}

impl ProjectedInitToken {
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
    pub bodies: BTreeMap<TokenId, ProjectedInitToken>,
    /// Committed literal by token ID (exactly one, for the operand token).
    pub literals_by_token: BTreeMap<TokenId, LiteralId>,
}

/// Build the narrow PA15 projection for one dispatched task.
///
/// Payload convention (candidate): exactly the two initializer token refs
/// in source order (`= <int>` for M1; no declarator tokens, no EOF). A
/// wrong kind, a non-token ref, or an unknown task is a caller protocol
/// fault; arity/content is `compute`'s job so non-M1 input surfaces as
/// `Unsupported`, never as a silent pass.
pub fn project_pa_init_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaInitInput, DiagnosticDraft> {
    let record = bus.arenas.tasks.get(task).map_err(|_| {
        protocol_fault(format!(
            "initializer parse of unknown task {}",
            task.index()
        ))
    })?;
    if record.kind != pa15_task_kind() {
        return Err(protocol_fault(format!(
            "initializer task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "initializer task {} payload must carry tokens only",
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
                ProjectedInitToken {
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
    Ok(PaInitInput {
        task,
        state: record.state.clone(),
        tokens,
        bodies,
        literals_by_token,
    })
}

/// The T05 PA15 initializer worker (M1 slice: fixed-shape validate +
/// `Ack`).
pub struct PaInitChip;

impl Worker for PaInitChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA15_CANDIDATE_CHIP,
            chip_name: "PaInitChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![pa15_task_kind()],
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
            tests: vec![],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pa_init_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PaInitChip {
    /// Pure PA15 computation over the narrow projection (no bus access).
    ///
    /// M1 accepts exactly `= <int>` and completes `Ack` with the two-node
    /// subtree shape documented (not committed). Anything else is an
    /// explicit fail; protocol faults use `Task/4`.
    pub fn compute(&self, input: &PaInitInput) -> Vec<Proposal> {
        match parse_initializer(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Pure M1 initializer parse: validate `= <int>` and build its local
/// subtree shape. No bus access; no shared-schema invention.
///
/// The `=` resolves against the committed PP bytes; the operand must be an
/// `Integer` token backed by one committed literal (the PA16 primary fact,
/// validated here at the token level with no child task). Value semantics
/// are T08-owned: this chip certifies syntax only.
///
/// Pre-order shape rows: `InitScalar(0)` → `IntLiteral(1)`.
///
/// A wider scalar (`= 2+3`), a braced list (`{1}`), or a designator
/// (`.x = 1`, `[2] = 1`) all fail loudly instead of guessing; nesting via
/// child tasks is wiring-layer work (DEFECT-INIT-02).
pub fn parse_initializer(input: &PaInitInput) -> Result<Vec<InitNodeShape>, DiagnosticDraft> {
    if input.state != TaskState::Running {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("initializer task {} is not running", input.task.index()),
        ));
    }
    if input.tokens.len() != 2 {
        return Err(InitError::Unsupported(
            "non-M1 initializer: exactly `= <int>` (two tokens) required",
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
    let eq_id = input.tokens[0];
    let operand_id = input.tokens[1];
    if input.bodies[&eq_id].kind != TokenKind::Punctuator
        || input.bodies[&operand_id].kind != TokenKind::Integer
    {
        return Err(InitError::Unsupported(
            "non-M1 initializer: operand/operator kinds must be `=` `<int>`",
        )
        .diagnostic());
    }
    if input.bodies[&eq_id].effective_spelling() != b"=" {
        return Err(InitError::Unsupported("non-M1 initializer: must open with `=`").diagnostic());
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::TokenKind;

    fn punct(spelling: &[u8]) -> (TokenKind, Vec<u8>, Vec<u8>) {
        (TokenKind::Punctuator, Vec::new(), spelling.to_vec())
    }

    fn integer() -> (TokenKind, Vec<u8>, Vec<u8>) {
        (TokenKind::Integer, Vec::new(), b"3".to_vec())
    }

    fn input_of(
        parts: &[(TokenKind, Vec<u8>, Vec<u8>)],
        with_literal_for: Option<usize>,
    ) -> PaInitInput {
        let mut tokens = Vec::new();
        let mut bodies = BTreeMap::new();
        let mut literals_by_token = BTreeMap::new();
        for (index, (kind, spelling, pp_spelling)) in parts.iter().enumerate() {
            let id = TokenId::from_index(index as u32);
            tokens.push(id);
            bodies.insert(
                id,
                ProjectedInitToken {
                    kind: *kind,
                    name: None,
                    spelling: spelling.clone(),
                    pp_spelling: pp_spelling.clone(),
                },
            );
            if Some(index) == with_literal_for {
                literals_by_token.insert(id, LiteralId::from_index(index as u32));
            }
        }
        PaInitInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            tokens,
            bodies,
            literals_by_token,
        }
    }

    fn m1_parts() -> Vec<(TokenKind, Vec<u8>, Vec<u8>)> {
        vec![punct(b"="), integer()]
    }

    #[test]
    fn m1_scalar_init_validates_with_shape() {
        let input = input_of(&m1_parts(), Some(1));
        let shapes = parse_initializer(&input).expect("M1 `= <int>` validates");
        assert_eq!(shapes.len(), 2);
        assert_eq!(shapes[0].kind_name, "InitScalar");
        assert_eq!(shapes[0].parent, None);
        assert_eq!((shapes[0].first_token, shapes[0].last_token), (0, 1));
        assert!(!shapes[0].has_literal);
        assert_eq!(shapes[1].kind_name, "IntLiteral");
        assert_eq!(shapes[1].parent, Some(0));
        assert!(shapes[1].has_literal);
    }

    #[test]
    fn braced_and_designator_inits_are_unsupported() {
        // `{1}`: wrong arity and kinds — never read as scalar `1`.
        let braced = input_of(
            &[
                (TokenKind::Punctuator, Vec::new(), b"{".to_vec()),
                integer(),
            ],
            Some(1),
        );
        let error = parse_initializer(&braced).expect_err("braced init defers past M1");
        assert_eq!(error.code.group, DiagGroup::Unsupported);

        // `.x = 1` shape fragment: designator territory (PA15 full split).
        let designator = input_of(
            &[
                (TokenKind::Punctuator, Vec::new(), b".".to_vec()),
                (TokenKind::Identifier, b"x".to_vec(), Vec::new()),
            ],
            None,
        );
        let error = parse_initializer(&designator).expect_err("designator defers past M1");
        assert_eq!(error.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn multi_token_expr_init_is_unsupported() {
        // `= 2+3`: the `2+3` operand is PA22-owned; composition is
        // wiring-layer work, never a single-chip fold here.
        let expr = input_of(&[punct(b"="), integer(), punct(b"+"), integer()], Some(1));
        let error = parse_initializer(&expr).expect_err("expression init defers past M1");
        assert_eq!(error.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn compute_acks_m1_and_fails_loud() {
        let chip = PaInitChip;
        let ok = chip.compute(&input_of(&m1_parts(), Some(1)));
        assert_eq!(
            ok,
            vec![Proposal::Complete {
                task: TaskId::from_index(0),
                value: ResultValue::Ack,
            }]
        );

        let braced = chip.compute(&input_of(
            &[
                (TokenKind::Punctuator, Vec::new(), b"{".to_vec()),
                integer(),
            ],
            Some(1),
        ));
        let [Proposal::Fail { diagnostic, .. }] = braced.as_slice() else {
            panic!("braced init must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);

        // Missing literal is a loud Task/4 fault, never a zero.
        let no_literal = chip.compute(&input_of(&m1_parts(), None));
        let [Proposal::Fail { diagnostic, .. }] = no_literal.as_slice() else {
            panic!("missing literal must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }

    #[test]
    fn compute_rejects_non_running_task() {
        let chip = PaInitChip;
        let mut input = input_of(&m1_parts(), Some(1));
        input.state = TaskState::Ready;
        let out = chip.compute(&input);
        let [Proposal::Fail { .. }] = out.as_slice() else {
            panic!("non-running task must fail");
        };
    }

    #[test]
    fn candidate_registration_is_next_free() {
        assert_eq!(PA15_CANDIDATE_LOCAL, 27);
        assert_eq!(PA15_CANDIDATE_CHIP, ChipId(58));
        let kind = pa15_task_kind();
        assert_eq!(kind.group(), TaskGroup::PARSE);
        assert_eq!(kind.local(), 27);
    }
}
