// ============================================================================
// chips/parse/pa_typedef_query.rs — T05 PA04 typedef-disambiguation worker
// (M1 slice, candidate registration — NOT frozen, integrator-owned).
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA04 / TypedefDisambiguationChip / PARSE (CANDIDATE:
//   `PARSE` local 27, `PA04_CANDIDATE_CHIP = ChipId(58)`; see the candidate
//   registration below. Neither the kind row nor the chip reservation is
//   frozen; the integrator owns `task.rs`/`manifest.rs` and all registries).
// Contract version/hash: T05 PENDING; T01 current. This file claims the PA04
//   `Name/scope/context → NameClass` row only: query the ordinary namespace
//   and distinguish a typedef-name from an ordinary identifier.
// Owned files: ONLY this file
//   (`compiler/src/chips/parse/pa_typedef_query.rs`). Never edits `mod.rs`,
//   bus/task/ids/manifest schemas, or registrations (see
//   docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: CANDIDATE `parse.typedef_query`
//   (`PARSE` local 27, [`pa04_task_kind`]); payload = exactly
//   `[Scope, Name]` in that order (mirrors the frozen T06
//   `symbol_type.lookup` payload convention); result = `Ack` (no
//   `NameClass` carrier is frozen: the wiring layer consumes the documented
//   [`NameClass`] through [`query_name_class`]; cursor advance stays
//   caller-held per T05 item G.4 OB-30 open).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (ordinary identifiers with no
//   typedef declaration in scope); a visible typedef-name is explicit
//   `Unsupported` (never a pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `symbols.symbols` + `symbols.scopes` (`/13`). Pure query: no bus
//   mutation; workers emit `Proposal`s only. Spellings are never read: per
//   T05 §5 the chip must not guess typedef-ness from the string.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no
//   `Enqueue`, no `AwaitChildren`. The [`NameClass`] rows below copy the
//   disambiguation outcome chip-locally for documentation/future split
//   only; they are NOT committed.
// Dependencies: T06 committed scopes + ordinary-namespace symbols (producer
//   facts only; never calls another chip). The full `(T)*x` cast-vs-Binary
//   and shadowing decisions stay wiring-layer owned until T06 freezes a
//   typedef carrier.
// Preconditions: dispatched task is `Running` with the PA04 candidate
//   kind; payload is exactly `[Scope, Name]`; the scope and every reported
//   symbol are committed.
// Transition states: Running → Complete(Ack) when no typedef-name is
//   visible for the name (M1 ordinary-identifier gate); Running → Fail
//   (`Unsupported` group) when a typedef-name is visible (needs T06);
//   Running → Fail (`Task` group code 4) on not-running/missing-record
//   protocol faults.
// Algorithm obligations: walk the committed scope chain innermost-first;
//   collect committed ordinary-namespace (`Function`/`Object`, T06 item 3)
//   hits in symbol-ID order for the wiring layer; classify through
//   [`query_name_class`] — ordinary when `typedef_visible` is false,
//   `Unsupported` when true.
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = project-then-compute; all paths yield
//   proposals; no `panic!`/`unwrap!`/`expect` on any path (missing records
//   are loud `Task/4` faults; the one `expect` is confined to the
//   compile-time-constant candidate-kind resolution in [`pa04_task_kind`],
//   unreachable for local 27).
// Error codes / recovery: `Unsupported/1` for typedef-name inputs
//   (explicit rejection, stays in the denominator); `Task/4` for protocol
//   faults; `Protocol/1` for unknown-task/wrong-kind scaffolding. No
//   recovery sync is attempted here (PA38 owns it).
// Required fixtures: in-file `#[cfg(test)]` unit tests pin the pure core
//   only (ordinary gate, typedef-present rejection, not-running fault,
//   replay determinism, candidate-constant sanity). Integrator-owned
//   acceptance (`parse.typedef_query` registration, stage row, routed
//   layer) is deferred with the `/6` co-freeze.
// Integration acceptance: OPEN (candidate kind/registration/stage rows,
//   `NameClass` carrier, T06 typedef fact shape — all T05/T01/T06
//   co-freeze).
// Known unsupported: typedef-name queries (`(T)*x`, shadowing), tag/enum
//   namespaces, `typeof`/GNU extension names (all deferred; typedef-name
//   inputs fail `Unsupported`).
//
// Candidate registration (NOT frozen; integrator-owned on acceptance):
//   `PA04_TASK_KIND_CANDIDATE` is `PARSE` local 27 (`parse.typedef_query`)
//   — the first free local after `PARSE_RECOVERY` (local 26);
//   `PA04_CANDIDATE_CHIP` is `ChipId(58)` — the next free ID after
//   `SE_FUNC_CHIP` (`ChipId(57)`). Rows belong in `TaskKindRegistry`,
//   `STAGE_ASSIGNMENT` (stage 2), and the routed layer 2; this file holds
//   no store-owner allowlist row (Ack-only, zero writes).
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-1 (typedef carrier): the frozen `/13` schema has no typedef
//     symbol kind (`SymbolKind` is `Function`/`Object` only), so no
//     committed typedef fact is projectable today. The projector therefore
//     reports `typedef_visible: false` always; the `Unsupported` branch is
//     exercised by direct pure-core construction in the unit tests until
//     T06 freezes the carrier. A program that declares a typedef would
//     mis-resolve as ordinary through this slice — the M1 gate is valid
//     only for typedef-free programs, which the caller guarantees.
//   DEFECT-2 (shadowing): a typedef shadowed by a local ordinary variable
//     needs scope-ordered visibility plus the DEFECT-1 carrier; both are
//     co-freeze. The projected `chain` + `ordinary_hits` preserve the
//     innermost-first evidence the wiring layer will need.
//   DEFECT-3 (cursor/class): `next_cursor` and any committed `NameClass`
//     record have no frozen carrier (T05 item G.4 OB-30 open). M1 completes
//     `Ack`; cursor advance stays caller-held. No scalar/tuple variant is
//     invented here.
//   DEFECT-4 (candidate-slot collision): sibling wave-4 candidate files in
//     this branch (`pa_cast.rs`, `pa_stmt.rs`, `pa_init.rs`,
//     `pa_decl_spec.rs`, `pa_expr_ext.rs`, `pa_ext_dispatch.rs`) each
//     independently claim `PARSE` local 27 and `ChipId(58)` as their own
//     candidate. This file's identical claim follows the same next-free
//     convention and is equally provisional; serializing the candidate
//     locals/chip IDs is integrator-owned (this file must not renumber
//     siblings or invent the frozen registry).
// ============================================================================

use crate::bus::SymbolKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, NameId, RecordRef, ScopeId, SymbolId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Candidate `PARSE` local code for `parse.typedef_query`.
///
/// Local 27 — the first free local after `PARSE_RECOVERY` (local 26).
/// NOT frozen; kept as the documented next-free value for the integrator.
pub const PA04_CANDIDATE_LOCAL: u16 = 27;

/// Candidate chip ID for the PA04 typedef-query worker.
///
/// `ChipId(58)` — the next free ID after `SE_FUNC_CHIP` (`ChipId(57)`).
/// NOT frozen; the reservation lives in `manifest.rs` on acceptance.
pub const PA04_CANDIDATE_CHIP: ChipId = ChipId(58);

/// Candidate task kind for PA04 typedef disambiguation.
///
/// Resolved from [`PA04_CANDIDATE_LOCAL`]; `None` only if the local exceeds
/// `LOCAL_MAX` (impossible for 27 — the `expect` in [`pa04_task_kind`] is
/// unreachable and loud rather than silent).
pub const PA04_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA04_CANDIDATE_LOCAL);

/// Resolve the candidate task kind served by the PA04 worker.
///
/// Registration in `TaskKindRegistry` is integrator-owned and lives in
/// `task.rs`, not here.
pub fn pa04_task_kind() -> TaskKind {
    PA04_TASK_KIND_CANDIDATE.expect("PA04 candidate PARSE local 27 exceeds LOCAL_MAX")
}

/// Name classification returned to the wiring layer.
///
/// The wiring layer (integrator-owned) consumes this through
/// [`query_name_class`] to route cast-vs-binary and declaration-vs-
/// expression decisions. Only [`Self::OrdinaryIdentifier`] is producible
/// in M1; [`Self::TypedefName`] documents the deferred T06 outcome and is
/// never constructed by the projector until the typedef carrier freezes
/// (DEFECT-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameClass {
    /// Ordinary identifier: no typedef-name is visible for the name in
    /// the active scope chain (the M1 gate; completes `Ack`).
    OrdinaryIdentifier,
    /// Typedef-name: visible through T06 typedef facts (deferred; needs
    /// T06 — the pure core reports `Unsupported`, never a silent pass).
    TypedefName,
}

/// Narrow projection for the PA04 typedef-query computation.
#[derive(Clone, Debug)]
pub struct PaTypedefQueryInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Queried name.
    pub name: NameId,
    /// Active scope chain, innermost-first (committed scopes only).
    pub chain: Vec<ScopeId>,
    /// Committed ordinary-namespace (`Function`/`Object`) symbols with
    /// this name, in symbol-ID order, paired with their declaring scope.
    /// Preserved innermost-resolution evidence for the wiring layer; the
    /// M1 gate does not filter on it (an undeclared name is a T06 lookup
    /// matter, not a typedef-name hit).
    pub ordinary_hits: Vec<(ScopeId, SymbolId)>,
    /// Whether a typedef-name is visible for the name. Always `false`
    /// until T06 freezes a typedef carrier (DEFECT-1); the `true` branch
    /// of [`query_name_class`] is pinned by direct unit construction.
    pub typedef_visible: bool,
}

/// Ordinary-namespace membership for the PA04 query (T06 item 3).
///
/// Mirrors `crate::chips::symbols::in_ordinary_namespace` chip-locally so
/// this standalone file carries no cross-module dependency; the two must
/// agree (`Function`/`Object` only) until the typedef carrier freezes.
fn in_ordinary_namespace(kind: SymbolKind) -> bool {
    matches!(kind, SymbolKind::Function | SymbolKind::Object)
}

/// Build the narrow PA04 projection for one dispatched task.
///
/// Payload convention (candidate): exactly `[Scope, Name]` in that order,
/// mirroring the frozen T06 `symbol_type.lookup` convention. A wrong kind,
/// a short/long payload, or mistyped refs are caller protocol faults;
/// typedef content is [`query_name_class`]'s job so future typedef inputs
/// surface as `Unsupported`, never as a silent pass.
pub fn project_pa_typedef_query_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaTypedefQueryInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("typedef-query of unknown task {}", task.index()))
        })?;
    if record.kind != pa04_task_kind() {
        return Err(protocol_fault(format!(
            "typedef-query task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 2 {
        return Err(protocol_fault(format!(
            "typedef-query task {} payload must be exactly [Scope, Name]",
            task.index()
        )));
    }
    let (Some(scope), Some(name)) = (
        match record.payload.refs[0] {
            RecordRef::Scope(id) => Some(id),
            _ => None,
        },
        match record.payload.refs[1] {
            RecordRef::Name(id) => Some(id),
            _ => None,
        },
    ) else {
        return Err(protocol_fault(format!(
            "typedef-query task {} payload must be exactly [Scope, Name]",
            task.index()
        )));
    };
    // Active scope chain only: walk parents to the root (deterministic).
    let mut chain = Vec::new();
    let mut current = Some(scope);
    while let Some(id) = current {
        let body = bus.arenas.scopes.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("typedef-query reads missing scope {}", id.index()),
            )
        })?;
        chain.push(id);
        current = body.parent;
    }
    // Ordinary-namespace hits in symbol-ID order (DEFECT-2 evidence).
    let mut ordinary_hits = Vec::new();
    for (id, symbol) in bus.arenas.symbols.iter() {
        if symbol.name == name && in_ordinary_namespace(symbol.kind) {
            ordinary_hits.push((symbol.scope, id));
        }
    }
    Ok(PaTypedefQueryInput {
        task,
        state: record.state.clone(),
        name,
        chain,
        ordinary_hits,
        // DEFECT-1: no frozen typedef carrier exists, so no committed
        // typedef fact is observable; always false until T06 freezes one.
        typedef_visible: false,
    })
}

/// The T05 PA04 typedef-disambiguation worker (M1 slice: ordinary gate +
/// `Ack`).
pub struct PaTypedefQueryChip;

impl Worker for PaTypedefQueryChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA04_CANDIDATE_CHIP,
            chip_name: "PaTypedefQueryChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![pa04_task_kind()],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Symbols, "symbols"),
                FieldPath::new(StoreId::Symbols, "scopes"),
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
        let input = match project_pa_typedef_query_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PaTypedefQueryChip {
    /// Pure PA04 computation over the narrow projection (no bus access).
    ///
    /// Ordinary identifiers complete `Ack` with no bus writes: the wiring
    /// layer reads the decision through [`query_name_class`]. A visible
    /// typedef-name is a typed `Unsupported` fail (needs T06). Every
    /// failure path is a typed `Fail`.
    pub fn compute(&self, input: &PaTypedefQueryInput) -> Vec<Proposal> {
        match query_name_class(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Pure M1 typedef query: classify one name in its scope chain.
///
/// No bus access; no shared-schema invention; never guesses from the
/// spelling (no spelling is projected). With no typedef-name visible the
/// name is an ordinary identifier (the M1 gate); a visible typedef-name
/// is explicit `Unsupported` until T06 freezes typedef facts.
pub fn query_name_class(input: &PaTypedefQueryInput) -> Result<NameClass, DiagnosticDraft> {
    if input.state != TaskState::Running {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("typedef-query task {} is not running", input.task.index()),
        ));
    }
    if input.typedef_visible {
        return Err(DiagnosticDraft::unsupported(
            "typedef-name disambiguation needs T06 typedef facts",
        ));
    }
    Ok(NameClass::OrdinaryIdentifier)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ordinary_input() -> PaTypedefQueryInput {
        PaTypedefQueryInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            name: NameId::from_index(0),
            chain: vec![ScopeId::from_index(0)],
            ordinary_hits: vec![(ScopeId::from_index(0), SymbolId::from_index(0))],
            typedef_visible: false,
        }
    }

    #[test]
    fn ordinary_identifier_gate_resolves_ordinary() {
        let input = ordinary_input();
        assert_eq!(query_name_class(&input), Ok(NameClass::OrdinaryIdentifier));
    }

    #[test]
    fn undeclared_name_still_resolves_ordinary_without_typedef_evidence() {
        let input = PaTypedefQueryInput {
            ordinary_hits: Vec::new(),
            chain: Vec::new(),
            ..ordinary_input()
        };
        assert_eq!(query_name_class(&input), Ok(NameClass::OrdinaryIdentifier));
    }

    #[test]
    fn typedef_present_fails_unsupported_needs_t06() {
        let input = PaTypedefQueryInput {
            typedef_visible: true,
            ..ordinary_input()
        };
        let diagnostic = query_name_class(&input).expect_err("typedef must not resolve ordinary");
        assert_eq!(
            diagnostic.code,
            DiagnosticCode::new(DiagGroup::Unsupported, 1)
        );
    }

    #[test]
    fn not_running_fails_task_fault() {
        let input = PaTypedefQueryInput {
            state: TaskState::Ready,
            ..ordinary_input()
        };
        let diagnostic = query_name_class(&input).expect_err("non-running must fail");
        assert_eq!(diagnostic.code, DiagnosticCode::new(DiagGroup::Task, 4));
    }

    #[test]
    fn compute_ordinary_completes_ack() {
        let chip = PaTypedefQueryChip;
        let proposals = chip.compute(&ordinary_input());
        assert_eq!(
            proposals,
            vec![Proposal::Complete {
                task: TaskId::from_index(0),
                value: ResultValue::Ack,
            }]
        );
    }

    #[test]
    fn compute_typedef_fails_unsupported() {
        let chip = PaTypedefQueryChip;
        let input = PaTypedefQueryInput {
            typedef_visible: true,
            ..ordinary_input()
        };
        let proposals = chip.compute(&input);
        assert_eq!(proposals.len(), 1);
        let Proposal::Fail { task, diagnostic } = &proposals[0] else {
            panic!("typedef input must fail, not complete");
        };
        assert_eq!(*task, TaskId::from_index(0));
        assert_eq!(
            diagnostic.code,
            DiagnosticCode::new(DiagGroup::Unsupported, 1)
        );
    }

    #[test]
    fn replay_is_deterministic() {
        let input = ordinary_input();
        assert_eq!(query_name_class(&input), query_name_class(&input));
        let chip = PaTypedefQueryChip;
        assert_eq!(chip.compute(&input), chip.compute(&input));
    }

    #[test]
    fn candidate_constants_claim_next_free_slots() {
        assert_eq!(PA04_CANDIDATE_LOCAL, 27);
        assert_eq!(PA04_CANDIDATE_CHIP, ChipId(58));
        let kind = pa04_task_kind();
        assert_eq!(kind.group(), TaskGroup::PARSE);
        assert_eq!(kind.local(), PA04_CANDIDATE_LOCAL);
    }

    #[test]
    fn worker_is_stateless_zst() {
        assert_eq!(std::mem::size_of::<PaTypedefQueryChip>(), 0);
    }

    #[test]
    fn manifest_claims_ack_only_candidate() {
        let manifest = PaTypedefQueryChip.manifest();
        assert_eq!(manifest.id, PA04_CANDIDATE_CHIP);
        assert_eq!(manifest.task_kinds, vec![pa04_task_kind()]);
        assert!(manifest.writes.is_empty());
        assert!(manifest.deterministic);
    }
}
