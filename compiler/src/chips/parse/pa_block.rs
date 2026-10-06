// ============================================================================
// chips/parse/pa_block.rs — T05 PA28/PA32 block + return workers (M1).
// (Wave 3 slice 9, `/34`)
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA28 CompoundStatementChip + PA32 JumpStatementChip
//   (return-with-expression minimal) / PARSE (FROZEN as `PA28_CHIP =
//   ChipId(47)`; see the frozen registration below). Single-chip file per
//   the worker template: one zero-sized unit struct, `PaBlockChip`, serves
//   both productions and dispatches on the production field (the dispatched
//   task's `TaskKind`, mirroring `ContinuationRecord.production`).
// Contract version/hash: T05 PENDING; T01 `/34` current; PA decl slice
//   owns `parse.external_declaration` + `parse.specifiers` +
//   `parse.declarator` + `parse.block` + `parse.return` — this file
//   claims the `parse.block` / `parse.return` rows only.
// Owned files: ONLY this file (`compiler/src/chips/parse/pa_block.rs`).
//   Never edits `mod.rs`, bus/task/ids/manifest schemas, or registrations
//   (see docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: FROZEN `parse` group locals
//   20 (PA28) and 21 (PA32) (`PA28_TASK_KIND` / `PA32_TASK_KIND`),
//   payload = committed `RecordRef::Token`s in source order
//   (exactly 7 for PA28, exactly 5 for PA32); result = `Ack` (no
//   next-cursor carrier: OB-30 open, caller holds the cursor).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (`{return 2+3;}` with a single
//   return of `<int>+<int>`); every other block/jump form is explicit
//   `Unsupported` (never a pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` + `lex.literals` (`/11`), `names.entries` (`/11`). Pure
//   query: no bus mutation; workers emit `Proposal`s only.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no
//   `Enqueue`. Committed `Compound`/`Return` nodes stay owned by the PA01
//   TU slice until the full catalog split; a second `parse.nodes` writer
//   would need an allowlist row this file must not invent. Scope enter/exit
//   is T06-owned. The `LocalNodeShape` rows below copy the
//   frozen `NodeRecord` field shapes chip-locally for documentation/future
//   split only; they are NOT committed.
// Dependencies: T04 committed tokens + interned spellings + decoded
//   literals (producer facts only; never calls another chip).
// Preconditions: dispatched task is `Running` with a frozen PA28/PA32
//   kind; payload carries `Token` refs only; referenced tokens and integer
//   literals are committed.
// Transition states: Running → Complete(Ack) on the exact M1 shape;
//   Running → Fail (`Unsupported` group) on any other shape; Running →
//   Fail (`Task` group code 4) on not-running/missing-record protocol
//   faults.
// Algorithm obligations: PA28 accepts exactly
//   `{` `return` `<int>` `+` `<int>` `;` `}` (7 tokens, `return` spelling
//   checked; punctuator spellings live behind unprojected links so only
//   kinds are checked for them); PA32 accepts exactly
//   `return` `<int>` `+` `<int>` `;` (5 tokens). Each integer leaf needs
//   exactly one committed literal. Literal *values* are not interpreted
//   (folding is T07/T08-owned).
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = dispatch-then-project-then-compute; all paths
//   yield proposals.
// Error codes / recovery: `Unsupported/1` for non-M1 shapes (explicit
//   rejection, stays in the denominator); `Task/4` for protocol faults;
//   `Protocol/1` for unknown-task/wrong-kind scaffolding. No recovery sync
//   is attempted here (PA38 owns it).
// Required fixtures: `compiler/tests/c34_parse.rs` (this slice).
// Integration acceptance: FROZEN by `/34` (kind registration, stage rows,
//   no allowlist rows — Ack-only; cursor carrier, parent linkage, child-task
//   nesting open).
// Known unsupported: empty blocks, interleaved declarations/statements,
//   `return;` (no expression), `break`/`continue`/`goto` (incl. GNU
//   computed goto), labels/case inside blocks, nested blocks, any operator
//   other than M1 `+` (all deferred; all fail `Unsupported`).
//
// Frozen registration (integrator-owned): `PA28_TASK_KIND` aliases
// `TaskKind::PARSE_BLOCK` (`PARSE` local 20, `parse.block`) and
// `PA32_TASK_KIND` aliases `TaskKind::PARSE_RETURN` (`PARSE` local 21,
// `parse.return`); `PA28_CHIP` is `crate::manifest::PA28_CHIP`
// (`ChipId(47)`), the kind-registry rows live in
// `TaskKindRegistry::pa_decl_slice()`, the stage-2 rows in
// `STAGE_ASSIGNMENT`, the routed layer is 2, there is no store-owner
// allowlist row (Ack-only, zero writes), and the acceptance test is
// `compiler/tests/c34_parse.rs`.
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-4 (nesting): the full design nests statements/expressions via
//     child tasks and continuations with await-all joins (T05 items A–C);
//     M1 validates the fixed shape in one tick with no `Enqueue`,
//     `AwaitChildren`, or `Progress`. Parse-depth accounting
//     (`ParseDepthExceeded`, item D) stays a future integration check.
//   DEFECT-5 (scope): block scope enter/exit is T06-owned
//     (`symbol_type.scope_enter`/`scope_exit` around the committed
//     `Compound` node). This chip writes nothing — not even `parse.nodes`
//     — so the PA28 "scope balance on error recovery" acceptance is
//     T06-integration work, documented here and deferred there.
//   DEFECT-6 (ambiguity): any input outside the two exact M1 shapes fails
//     `Unsupported` instead of guessing (e.g. `return;` is not treated as
//     "return zero", `{}` is not an empty compound, `;` alone is not an
//     expression statement — PA34 owns that call).
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA28_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Frozen task kind for PA28 compound-statement parsing.
///
/// `PARSE` local 20 — the first free local after `PARSE_DECLARATOR`
/// (local 19); registered `Frozen` in `TaskKindRegistry::pa_decl_slice()`.
pub const PA28_TASK_KIND: TaskKind = TaskKind::PARSE_BLOCK;

/// Frozen task kind for PA32 return-statement parsing.
///
/// `PARSE` local 21 — the next free local after the PA28 kind;
/// registered `Frozen` in `TaskKindRegistry::pa_decl_slice()`.
pub const PA32_TASK_KIND: TaskKind = TaskKind::PARSE_RETURN;

/// Production served by [`PaBlockChip`]: the dispatch target resolved from
/// the dispatched task's kind (the production field).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaBlockProduction {
    /// PA28 compound-statement production (`{ return 2+3; }`).
    Block,
    /// PA32 return-with-expression production (`return 2+3;`).
    Return,
}

impl PaBlockProduction {
    /// Resolve the production for a dispatched task kind.
    pub fn of_kind(kind: TaskKind) -> Option<Self> {
        if kind == PA28_TASK_KIND {
            Some(Self::Block)
        } else if kind == PA32_TASK_KIND {
            Some(Self::Return)
        } else {
            None
        }
    }
}

/// Chip-local copy of the frozen `NodeRecord` field shapes for one M1
/// subtree node.
///
/// Same field names/order as `NodeRecord` (`parent`, `first_token`,
/// `last_token`, literal presence) so the future split can lift it
/// verbatim, except positions are indices into the task's own token slice
/// and links are pre-order node indices rather than committed IDs; the kind
/// is the `NodeKind` variant name string rather than imported shared
/// schema. Shapes are built by the pure path and documented — they are NOT
/// appended to any arena by this Ack-only slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalNodeShape {
    /// Fixed node-kind name (mirrors `NodeKind`, e.g. `"Compound"`).
    pub kind_name: &'static str,
    /// Pre-order index of the parent node (`None` for the subtree root;
    /// owner integration assigns the committed parent — see DEFECT-5).
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
pub struct ProjectedBlockToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Spelling bytes for `Identifier`/`Keyword` tokens.
    pub spelling: Vec<u8>,
}

/// Narrow projection for the PA28 block computation.
#[derive(Clone, Debug)]
pub struct PaBlockInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (M1 requires exactly 7).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedBlockToken>,
    /// Committed literal by token ID (exactly one per integer token).
    pub literals_by_token: BTreeMap<TokenId, LiteralId>,
}

/// Narrow projection for the PA32 return computation.
#[derive(Clone, Debug)]
pub struct PaReturnInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (M1 requires exactly 5).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedBlockToken>,
    /// Committed literal by token ID (exactly one per integer token).
    pub literals_by_token: BTreeMap<TokenId, LiteralId>,
}

/// Projected token stream for one block/return task: token IDs in source
/// order, present token bodies, committed literals by token, dispatch state.
type BlockTokenProjection = (
    Vec<TokenId>,
    BTreeMap<TokenId, ProjectedBlockToken>,
    BTreeMap<TokenId, LiteralId>,
    TaskState,
);

fn project_block_tokens(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
    want: TaskKind,
    label: &str,
) -> Result<BlockTokenProjection, DiagnosticDraft> {
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
                ProjectedBlockToken {
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

/// Build the narrow PA28 projection for one dispatched task.
pub fn project_pa_block_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaBlockInput, DiagnosticDraft> {
    let (tokens, bodies, literals_by_token, state) =
        project_block_tokens(bus, task, PA28_TASK_KIND, "block parse")?;
    Ok(PaBlockInput {
        task,
        state,
        tokens,
        bodies,
        literals_by_token,
    })
}

/// Build the narrow PA32 projection for one dispatched task.
pub fn project_pa_return_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaReturnInput, DiagnosticDraft> {
    let (tokens, bodies, literals_by_token, state) =
        project_block_tokens(bus, task, PA32_TASK_KIND, "return parse")?;
    Ok(PaReturnInput {
        task,
        state,
        tokens,
        bodies,
        literals_by_token,
    })
}

/// The fused T05 PA28/PA32 worker (M1 slice: fixed-shape validate + `Ack`).
pub struct PaBlockChip;

impl Worker for PaBlockChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA28_CHIP,
            chip_name: "PaBlockChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![PA28_TASK_KIND, PA32_TASK_KIND],
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
            tests: vec!["compiler/tests/c34_parse.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let kind = match bus.arenas.tasks.get(task) {
            Ok(record) => record.kind,
            Err(_) => {
                return vec![fail(
                    task,
                    protocol_fault(format!("block parse of unknown task {}", task.index())),
                )];
            }
        };
        match PaBlockProduction::of_kind(kind) {
            Some(PaBlockProduction::Block) => match project_pa_block_input(bus, task) {
                Ok(input) => self.compute_block(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            Some(PaBlockProduction::Return) => match project_pa_return_input(bus, task) {
                Ok(input) => self.compute_return(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            None => vec![fail(
                task,
                protocol_fault(format!(
                    "block-parse task {} has unexpected kind {}",
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
    bodies: &BTreeMap<TokenId, ProjectedBlockToken>,
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
    bodies: &BTreeMap<TokenId, ProjectedBlockToken>,
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

impl PaBlockChip {
    /// Pure PA28 computation over the narrow projection (no bus access).
    ///
    /// M1 accepts exactly `{ return <int> + <int> ; }` and completes `Ack`
    /// with the five-node subtree shape documented (not committed).
    /// Anything else is an explicit fail; protocol faults use `Task/4`.
    pub fn compute_block(&self, input: &PaBlockInput) -> Vec<Proposal> {
        match parse_block(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }

    /// Pure PA32 computation over the narrow projection (no bus access).
    ///
    /// M1 accepts exactly `return <int> + <int> ;` and completes `Ack`
    /// with the four-node subtree shape documented (not committed).
    pub fn compute_return(&self, input: &PaReturnInput) -> Vec<Proposal> {
        match parse_return(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Pure M1 block parse: validate `{ return 2+3; }` and build its local
/// subtree shape. No bus access; no shared-schema invention.
///
/// Pre-order shape rows: `Compound(0)` → `Return(1)` → `BinaryAdd(2)` →
/// `IntLiteral(3)`, `IntLiteral(4)`.
pub fn parse_block(input: &PaBlockInput) -> Result<Vec<LocalNodeShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "block parse")?;
    check_token_kinds(
        &input.tokens,
        &input.bodies,
        M1_BLOCK_KINDS,
        "non-M1 block: exactly `{ return <int> + <int> ; }` required",
    )?;
    check_return_spelling(&input.tokens, &input.bodies, 1)?;
    check_integer_literals(input.task, &input.tokens, &input.literals_by_token, &[2, 4])?;
    Ok(vec![
        LocalNodeShape {
            kind_name: "Compound",
            parent: None,
            first_token: 0,
            last_token: 6,
            has_literal: false,
        },
        LocalNodeShape {
            kind_name: "Return",
            parent: Some(0),
            first_token: 1,
            last_token: 5,
            has_literal: false,
        },
        LocalNodeShape {
            kind_name: "BinaryAdd",
            parent: Some(1),
            first_token: 2,
            last_token: 4,
            has_literal: false,
        },
        LocalNodeShape {
            kind_name: "IntLiteral",
            parent: Some(2),
            first_token: 2,
            last_token: 2,
            has_literal: true,
        },
        LocalNodeShape {
            kind_name: "IntLiteral",
            parent: Some(2),
            first_token: 4,
            last_token: 4,
            has_literal: true,
        },
    ])
}

/// Pure M1 return parse: validate `return 2+3;` and build its local
/// subtree shape. No bus access; no shared-schema invention.
///
/// Pre-order shape rows: `Return(0)` → `BinaryAdd(1)` → `IntLiteral(2)`,
/// `IntLiteral(3)`.
pub fn parse_return(input: &PaReturnInput) -> Result<Vec<LocalNodeShape>, DiagnosticDraft> {
    check_running(input.task, &input.state, "return parse")?;
    check_token_kinds(
        &input.tokens,
        &input.bodies,
        M1_RETURN_KINDS,
        "non-M1 return: exactly `return <int> + <int> ;` required",
    )?;
    check_return_spelling(&input.tokens, &input.bodies, 0)?;
    check_integer_literals(input.task, &input.tokens, &input.literals_by_token, &[1, 3])?;
    Ok(vec![
        LocalNodeShape {
            kind_name: "Return",
            parent: None,
            first_token: 0,
            last_token: 4,
            has_literal: false,
        },
        LocalNodeShape {
            kind_name: "BinaryAdd",
            parent: Some(0),
            first_token: 1,
            last_token: 3,
            has_literal: false,
        },
        LocalNodeShape {
            kind_name: "IntLiteral",
            parent: Some(1),
            first_token: 1,
            last_token: 1,
            has_literal: true,
        },
        LocalNodeShape {
            kind_name: "IntLiteral",
            parent: Some(1),
            first_token: 3,
            last_token: 3,
            has_literal: true,
        },
    ])
}
