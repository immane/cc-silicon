// ============================================================================
// chips/parse/pa_specifier.rs — T05 PA03 DeclarationSpecifiers worker (M1).
// (Wave 3 slice 9, `/34`)
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA03 / DeclarationSpecifiersChip / PARSE (FROZEN as
//   `PA03_CHIP = ChipId(45)`; see the frozen registration below).
// Contract version/hash: T05 PENDING; T01 `/34` current; PA decl slice
//   owns `parse.external_declaration` + `parse.specifiers` +
//   `parse.declarator` + `parse.block` + `parse.return` — this file
//   claims the `parse.specifiers` row only.
// Owned files: ONLY this file (`compiler/src/chips/parse/pa_specifier.rs`).
//   Never edits `mod.rs`, bus/task/ids/manifest schemas, or registrations
//   (see docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: FROZEN `parse.specifiers`
//   (`PARSE` local 18, `PA03_TASK_KIND`), payload = exactly one
//   `RecordRef::Token` cursor; result = `Ack` (no cursor carrier: OB-30 open,
//   caller holds the cursor).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (`int` alone); every other
//   spelling/combination is explicit `Unsupported` (never a pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` (`/11`), `names.entries` (`/11`). Pure query: no bus
//   mutation; workers emit `Proposal`s only.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no `Enqueue`.
//   The `SpecifierRecordShape` below copies the frozen `NodeRecord` field
//   shapes chip-locally for documentation/future split only; it is NOT
//   committed (committed `Specifiers` nodes stay owned by the PA01 TU slice
//   until the full catalog split; a second writer would need an allowlist
//   row this file must not invent).
// Dependencies: T04 committed tokens + interned spellings (producer facts
//   only; never calls another chip). Hands the bundle to the type chip for
//   validation in the full design; M1 performs no type interpretation.
// Preconditions: dispatched task is `Running`; payload carries `Token` refs
//   only; the single cursor token is committed.
// Transition states: Running → Complete(Ack) on `int`; Running → Fail
//   (`Unsupported` group) on any other bundle; Running → Fail (`Task` group
//   code 4) on not-running/missing-token protocol faults.
// Algorithm obligations: accept exactly Keyword `int` (one token); reject
//   everything else as `Unsupported` without guessing from spelling;
//   never derive typedef-ness locally (PA04 owns that query).
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = project-then-compute; all paths yield proposals.
// Error codes / recovery: `Unsupported/1` for non-M1 bundles (explicit
//   rejection, stays in the denominator); `Task/4` for protocol faults;
//   `Protocol/1` for unknown-task/wrong-kind scaffolding.
// Required fixtures: `compiler/tests/c34_parse.rs` (this slice).
// Integration acceptance: FROZEN by `/34` (kind registration, stage row,
//   no allowlist row — Ack-only; cursor carrier, parent linkage open).
// Known unsupported: storage-class specifiers, qualifiers, function
//   specifiers, alignment specifiers, `signed`/`unsigned`/`short`/`long`
//   combinations and ordering, struct/union/enum specifiers, typedef names,
//   `_Static_assert`/attributes in specifier position (all deferred).
//
// Frozen registration (integrator-owned): `PA03_TASK_KIND` aliases
// `TaskKind::PARSE_SPECIFIERS` (`PARSE` local 18, `parse.specifiers`),
// `PA03_CHIP` is `crate::manifest::PA03_CHIP` (`ChipId(45)`), the
// kind-registry row lives in `TaskKindRegistry::pa_decl_slice()`, the
// stage-2 row in `STAGE_ASSIGNMENT`, the routed layer is 2, there is no
// store-owner allowlist row (Ack-only, zero writes), and the acceptance
// test is `compiler/tests/c34_parse.rs`.
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-3 (cursor): `next_cursor` has no frozen `ResultValue` carrier
//     (T05 item G.4 OB-30 open). M1 completes `Ack`; cursor advance stays
//     caller-held. No scalar/tuple variant is invented here.
//   DEFECT-4 (ambiguity): `unsigned long` ordering, `long long`, and any
//     typedef-name-vs-identifier call require PA04 scope queries + T06 type
//     facts; this chip resolves none and fails `Unsupported` instead.
//   DEFECT-5 (parentage): the committed parent link of a future `Specifiers`
//     node belongs to the PA01/PA02/PA14 integration; this Ack-only slice
//     assigns no parent.
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, NameId, NodeId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA03_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Frozen task kind served by the PA03 worker.
///
/// `PARSE` local 18 — the first free local after `PARSE_EXTERNAL_DECL`
/// (local 17); registered `Frozen` in `TaskKindRegistry::pa_decl_slice()`.
pub const PA03_TASK_KIND: TaskKind = TaskKind::PARSE_SPECIFIERS;

/// Chip-local copy of the frozen `NodeRecord` field shapes for one M1
/// `Specifiers` bundle.
///
/// Same field names/order as `NodeRecord` (`parent`, `children`,
/// `first_token`, `last_token`, `name`, `literal`) so the future split can
/// lift it verbatim; the kind is fixed to the `"Specifiers"` name string
/// rather than importing new shared schema. M1 fixes: `parent: None`
/// (DEFECT-5), `children: []`, `first == last == cursor`, `name: None`,
/// `literal: None`. This shape is built by the pure path and documented —
/// it is NOT appended to any arena by this Ack-only slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecifierRecordShape {
    /// Fixed `"Specifiers"` (mirrors `NodeKind::Specifiers`).
    pub kind_name: &'static str,
    /// Parent node (`None` in this slice; owner integration assigns it).
    pub parent: Option<NodeId>,
    /// Ordered child nodes (always empty for the M1 leaf bundle).
    pub children: Vec<NodeId>,
    /// First covered token (committed; equals `last_token` in M1).
    pub first_token: TokenId,
    /// Last covered token (committed, inclusive).
    pub last_token: TokenId,
    /// Declarator name (always `None` for a specifier bundle).
    pub name: Option<NameId>,
    /// Committed literal (always `None` for a specifier bundle).
    pub literal: Option<LiteralId>,
}

impl SpecifierRecordShape {
    /// Build the M1 `int` bundle shape over one committed cursor token.
    pub fn m1_int(cursor: TokenId) -> Self {
        Self {
            kind_name: "Specifiers",
            parent: None,
            children: Vec::new(),
            first_token: cursor,
            last_token: cursor,
            name: None,
            literal: None,
        }
    }
}

/// One projected token: kind plus resolved spelling for named tokens.
#[derive(Clone, Debug)]
pub struct ProjectedSpecifierToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Spelling bytes for `Identifier`/`Keyword` tokens.
    pub spelling: Vec<u8>,
}

/// Narrow projection for the specifier computation.
#[derive(Clone, Debug)]
pub struct PaSpecifierInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order (M1 requires exactly one).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedSpecifierToken>,
}

/// Build the narrow projection for one dispatched task.
///
/// The task kind must be exactly `PA03_TASK_KIND`. Non-`Token` refs are
/// protocol faults (same rule as `PaTuChip`); arity/content is `compute`'s
/// job so non-M1 input surfaces as `Unsupported`, never as a silent pass.
pub fn project_pa_specifier_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaSpecifierInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("specifier parse of unknown task {}", task.index()))
        })?;
    if record.kind != PA03_TASK_KIND {
        return Err(protocol_fault(format!(
            "specifier task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "specifier task {} payload must carry tokens only",
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
                ProjectedSpecifierToken {
                    kind: body.kind,
                    spelling,
                },
            );
        }
    }
    Ok(PaSpecifierInput {
        task,
        state: record.state.clone(),
        tokens,
        bodies,
    })
}

/// The T05 PA03 declaration-specifiers worker (M1 slice: `int` alone).
pub struct PaSpecifierChip;

impl Worker for PaSpecifierChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA03_CHIP,
            chip_name: "PaSpecifierChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![PA03_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Lex, "tokens"),
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
        let input = match project_pa_specifier_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PaSpecifierChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// M1 accepts exactly one Keyword `int` token and completes `Ack`
    /// (cursor advance stays caller-held per DEFECT-3). Anything else is an
    /// explicit `Unsupported` fail; protocol faults use `Task/4`.
    pub fn compute(&self, input: &PaSpecifierInput) -> Vec<Proposal> {
        match parse_specifier(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Pure M1 specifier parse: validate one `int` token and build its local
/// shape. No bus access; no shared-schema invention.
pub fn parse_specifier(input: &PaSpecifierInput) -> Result<SpecifierRecordShape, DiagnosticDraft> {
    if input.state != TaskState::Running {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("specifier task {} is not running", input.task.index()),
        ));
    }
    if input.tokens.len() != 1 {
        return Err(DiagnosticDraft::unsupported(
            "non-M1 specifier bundle: exactly one `int` token required",
        ));
    }
    let cursor = input.tokens[0];
    let Some(projected) = input.bodies.get(&cursor) else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("specifier reads missing token {}", cursor.index()),
        ));
    };
    if projected.kind != TokenKind::Keyword || projected.spelling != b"int" {
        return Err(DiagnosticDraft::unsupported(
            "non-M1 declaration specifiers: only `int` alone is supported",
        ));
    }
    Ok(SpecifierRecordShape::m1_int(cursor))
}
