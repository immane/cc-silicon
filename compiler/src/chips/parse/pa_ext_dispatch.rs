// ============================================================================
// chips/parse/pa_ext_dispatch.rs — T05 PA33/34/35/36/37 ext-dispatch workers
//   (candidate Unsupported-shells; PA37 carries the T12 handoff note).
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA33 LabelStatementChip + PA34 ExpressionStatementChip
//   + PA35 StaticAssertParseChip + PA36 AttributeParseChip + PA37
//   ExtensionSyntaxDispatchChip / PARSE (CANDIDATE — not frozen: `PARSE`
//   locals 27/28/29/30/31, candidate chip `ChipId(58)`; the kind-registry,
//   stage-assignment, and manifest rows are integrator-owned and NOT
//   created here).
// Contract version/hash: T05 PENDING; T01 `/38` current (PA-recovery slice
//   froze `PARSE` through local 26; `PARSE` owners start new codes at local
//   27 — see `TaskKindRegistry::pa_recovery_slice()`). This file claims the
//   five T05 rows PA33/34/35/36/37 only (see
//   docs/tasks/T05_PARSE_CHIPS.md rows 43–47).
// Owned files: ONLY this file
//   (`compiler/src/chips/parse/pa_ext_dispatch.rs`). Never edits `mod.rs`,
//   bus/task/ids/manifest schemas, or registrations (see
//   docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: CANDIDATE `parse.label` (local
//   27) / `parse.expr_statement` (28) / `parse.static_assert` (29) /
//   `parse.attribute` (30) / `parse.extension_dispatch` (31); payload =
//   committed `RecordRef::Token` refs in source order forming the production
//   window; result = never `Complete` in this shell — every well-formed
//   dispatch fails `Unsupported` (explicit gap, stays in the denominator).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A exercises NONE of these productions
//   (the canonical `int main(void){return 2+3;}` holds no label, no
//   expression statement, no `_Static_assert`, no attribute, and no GNU
//   extension), so every dispatched task fails `Unsupported` — never a pass.
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation) only.
//   Pure query: no bus mutation; workers emit `Proposal`s only. Token bodies
//   are deliberately NOT read: the shell rejects before spelling checks.
// Writes: NONE. No `AppendRecords`, no `StorePatch`, no `Enqueue`, no
//   `AwaitChildren`. Committed `Label`/`ExprStatement`/`AssertDecl`/
//   `AttributeSyntax` nodes stay owned by the future full-catalog split; a
//   second `parse.nodes` writer would need an allowlist row this file must
//   not invent. PA37 enqueues NO T12 task itself — the T12 fan-out
//   (`extensions`/`builtins` per docs/tasks/T12_GNU_BUILTIN_CHIPS.md) stays
//   wiring-layer owned; the handoff note names the target.
// Dependencies: T04 committed tokens as producer facts (payload refs only;
//   never calls another chip). PA35 semantics belong to the T08 constant
//   task; PA36 attach semantics belong to T12 EX07/EX08; PA37 bodies belong
//   to T12 EX02/EX03/EX04/EX05/EX13–EX15.
// Preconditions: dispatched task is `Running` with one of the five
//   candidate kinds; payload carries `Token` refs only.
// Transition states: Running → Fail (`Unsupported` group, code 1) on every
//   well-formed dispatch (M1 exercises none of these rows); Running → Fail
//   (`Task` group code 4) on not-running protocol faults; unknown
//   task/wrong-kind/non-token payload → `Protocol/1` scaffolding fault.
// Algorithm obligations: resolve the production from the dispatched task's
//   `TaskKind` (the production field, mirroring
//   `ContinuationRecord.production`); project-then-compute; emit exactly one
//   `Fail` proposal per dispatch with the per-production message below.
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = dispatch-then-project-then-compute; all paths
//   yield proposals; no `panic!`/`unwrap!`/`expect` on any path.
// Error codes / recovery: `Unsupported/1` for every production body
//   (explicit rejection, stays in the denominator); `Task/4` for
//   not-running protocol faults; `Protocol/1` for
//   unknown-task/wrong-kind/non-token scaffolding. No recovery sync is
//   attempted here (PA38 owns it).
// Required fixtures: in-file unit tests only (no integration fixture yet;
//   the manifest `tests` list is empty until the integrator assigns one).
// Integration acceptance: CANDIDATE (not frozen — kind registration,
//   stage-2 rows, routed layer, and allowlist review are integrator-owned).
// Known unsupported: everything on these five rows — labels/adjacent
//   labels/`case`/`default`/GNU `case a...b` (PA33); empty `;` and
//   expression statements (PA34); `_Static_assert`/message/mode handling
//   (PA35, semantics T08-owned); GNU `__attribute__`/C23 `[[]]` syntax and
//   attach positions (PA36, semantics T12 EX07/EX08-owned); statement
//   expressions/`typeof`/`asm`/`&&label`/computed `goto` (PA37, bodies
//   T12-owned).
//
// Frozen-registration note (integrator-owned): the next free `PARSE` local
//   after `PARSE_RECOVERY` (local 26) is 27, and the next free chip ID after
//   `SE_FUNC_CHIP` (`ChipId(57)`) is `ChipId(58)`. The constants below use
//   those next-free values as CANDIDATES only; the frozen rows (kind
//   registry, `STAGE_ASSIGNMENT`, routed layer, store-owner allowlist) must
//   be created by the integrator in a version bump — this file aliases
//   nothing frozen and invents no frozen row.
//
// PA37 → T12 handoff note (wiring-layer owned, documented here): a future
//   extension-dispatch body routes `({…})` to EX03, `typeof` to EX02, `asm`
//   to EX13→EX14→EX15, `&&label` to EX04, and `goto *p` to EX05, with
//   standard-mode rejection via EX01. This shell enqueues nothing and hands
//   the T12 target names to the wiring layer through the diagnostic text.
// ============================================================================

use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use crate::task::{Proposal, StoreId, TaskGroup, TaskKind, TaskState};

/// Candidate `PARSE` local for PA33 `parse.label`.
///
/// First free local after `PARSE_RECOVERY` (local 26). Candidate only —
/// the frozen row is integrator-owned.
pub const PA33_CANDIDATE_LOCAL: u16 = 27;

/// Candidate `PARSE` local for PA34 `parse.expr_statement`.
pub const PA34_CANDIDATE_LOCAL: u16 = 28;

/// Candidate `PARSE` local for PA35 `parse.static_assert`.
pub const PA35_CANDIDATE_LOCAL: u16 = 29;

/// Candidate `PARSE` local for PA36 `parse.attribute`.
pub const PA36_CANDIDATE_LOCAL: u16 = 30;

/// Candidate `PARSE` local for PA37 `parse.extension_dispatch`.
pub const PA37_CANDIDATE_LOCAL: u16 = 31;

/// Candidate chip ID for the fused PA33/34/35/36/37 worker.
///
/// Next free ID after `SE_FUNC_CHIP` (`ChipId(57)`). Candidate only — the
/// frozen manifest row is integrator-owned.
pub const PA_EXT_DISPATCH_CANDIDATE_CHIP: ChipId = ChipId(58);

/// Candidate task kind for PA33 label-statement parsing.
pub const PA33_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA33_CANDIDATE_LOCAL);

/// Candidate task kind for PA34 expression-statement parsing.
pub const PA34_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA34_CANDIDATE_LOCAL);

/// Candidate task kind for PA35 `_Static_assert` parsing.
pub const PA35_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA35_CANDIDATE_LOCAL);

/// Candidate task kind for PA36 attribute-syntax parsing.
pub const PA36_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA36_CANDIDATE_LOCAL);

/// Candidate task kind for PA37 extension-syntax dispatch.
pub const PA37_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA37_CANDIDATE_LOCAL);

/// Production served by [`PaExtDispatchChip`]: resolved from the dispatched
/// task's kind (the production field).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaExtProduction {
    /// PA33 label-statement production (`ident:`, `case`, `default`).
    Label,
    /// PA34 expression-statement production (`;` / `expr;`).
    ExprStatement,
    /// PA35 `_Static_assert` production.
    StaticAssert,
    /// PA36 attribute-syntax production (`__attribute__` / `[[]]`).
    Attribute,
    /// PA37 extension-syntax dispatch (statement expression, `typeof`,
    /// `asm`, label address, computed goto → T12).
    ExtensionDispatch,
}

impl PaExtProduction {
    /// Resolve the production for a dispatched task kind.
    ///
    /// Candidate-only: matches the five `TaskKind::new(PARSE, 27..=31)`
    /// values. Returns `None` for any other kind (including every frozen
    /// kind) so the caller reports a protocol fault instead of guessing.
    pub fn of_kind(kind: TaskKind) -> Option<Self> {
        if Some(kind) == PA33_TASK_KIND_CANDIDATE {
            Some(Self::Label)
        } else if Some(kind) == PA34_TASK_KIND_CANDIDATE {
            Some(Self::ExprStatement)
        } else if Some(kind) == PA35_TASK_KIND_CANDIDATE {
            Some(Self::StaticAssert)
        } else if Some(kind) == PA36_TASK_KIND_CANDIDATE {
            Some(Self::Attribute)
        } else if Some(kind) == PA37_TASK_KIND_CANDIDATE {
            Some(Self::ExtensionDispatch)
        } else {
            None
        }
    }

    /// The candidate task kind for this production.
    pub fn task_kind(self) -> Option<TaskKind> {
        match self {
            Self::Label => PA33_TASK_KIND_CANDIDATE,
            Self::ExprStatement => PA34_TASK_KIND_CANDIDATE,
            Self::StaticAssert => PA35_TASK_KIND_CANDIDATE,
            Self::Attribute => PA36_TASK_KIND_CANDIDATE,
            Self::ExtensionDispatch => PA37_TASK_KIND_CANDIDATE,
        }
    }

    /// Short production name for diagnostics and manifests.
    pub fn name(self) -> &'static str {
        match self {
            Self::Label => "parse.label",
            Self::ExprStatement => "parse.expr_statement",
            Self::StaticAssert => "parse.static_assert",
            Self::Attribute => "parse.attribute",
            Self::ExtensionDispatch => "parse.extension_dispatch",
        }
    }

    /// Explicit `Unsupported` message for this production.
    ///
    /// Every message states the deferred scope and the owning follow-up
    /// (PA38 for recovery, T08 for assert semantics, T12 for attribute
    /// semantics and all extension bodies) so the gap stays in the
    /// denominator with its handoff attached.
    pub fn unsupported_message(self) -> &'static str {
        match self {
            Self::Label => "label statement deferred past M1 (labels, adjacent labels, `case`/`default`, GNU `case a...b`); M1 exercises no label production",
            Self::ExprStatement => "expression statement deferred past M1 (`;` and `expr;`); M1 holds no expression statement — see PA38 for recovery sync",
            Self::StaticAssert => "static assertion deferred past M1 (`_Static_assert` condition/message and mode handling); assertion semantics stay T08-owned",
            Self::Attribute => "attribute syntax deferred past M1 (GNU `__attribute__` and C23 `[[]]`, declarator/type/member positions); attach semantics stay T12 EX07/EX08-owned",
            // PA37 → T12 dispatch handoff (wiring-layer owned; named here).
            Self::ExtensionDispatch => "extension syntax deferred past M1; T12 handoff: `({...})` to EX03 StatementExpression, `typeof` to EX02 Typeof, `asm` to EX13/EX14/EX15 AsmSyntax/Constraint/Effect, `&&label` to EX04 LabelAddress, `goto *p` to EX05 ComputedGoto, mode gate EX01; standard mode rejects, GNU mode dispatches",
        }
    }
}

/// Narrow projection for the PA33/34/35/36/37 computations.
#[derive(Clone, Debug)]
pub struct PaExtInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Resolved production for the dispatched kind.
    pub production: PaExtProduction,
    /// Payload token IDs in source order (the production window).
    pub tokens: Vec<TokenId>,
}

/// Build the narrow projection for one dispatched ext-dispatch task.
///
/// Payload convention (candidate): committed `RecordRef::Token` refs in
/// source order forming the production window. A wrong kind, an empty
/// payload, or a non-token ref is a caller protocol fault.
pub fn project_pa_ext_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaExtInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("ext-dispatch of unknown task {}", task.index()))
        })?;
    let Some(production) = PaExtProduction::of_kind(record.kind) else {
        return Err(protocol_fault(format!(
            "ext-dispatch task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    };
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "ext-dispatch task {} payload must carry the production token refs",
            task.index()
        )));
    }
    let mut tokens = Vec::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "ext-dispatch task {} payload must carry tokens only",
                task.index()
            )));
        };
        tokens.push(*id);
    }
    Ok(PaExtInput {
        task,
        state: record.state.clone(),
        production,
        tokens,
    })
}

/// The fused T05 PA33/34/35/36/37 worker (candidate Unsupported-shells).
pub struct PaExtDispatchChip;

impl Worker for PaExtDispatchChip {
    fn manifest(&self) -> ChipManifest {
        // Candidate kinds only: every `expect` below is unreachable for
        // locals 27–31 (well under `LOCAL_MAX`) and loud rather than
        // silent if the candidate inventory ever shifts.
        let kinds = [
            PA33_TASK_KIND_CANDIDATE,
            PA34_TASK_KIND_CANDIDATE,
            PA35_TASK_KIND_CANDIDATE,
            PA36_TASK_KIND_CANDIDATE,
            PA37_TASK_KIND_CANDIDATE,
        ]
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .expect("PA33-37 candidate locals must resolve");
        ChipManifest {
            id: PA_EXT_DISPATCH_CANDIDATE_CHIP,
            chip_name: "PaExtDispatchChip",
            group: TaskGroup::PARSE,
            task_kinds: kinds,
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            // Candidate: no integration fixture assigned yet; in-file unit
            // tests below are the only coverage.
            tests: vec![],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pa_ext_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PaExtDispatchChip {
    /// Pure computation over the narrow projection (no bus access).
    ///
    /// M1 exercises none of these productions, so every `Running` dispatch
    /// fails `Unsupported` with the per-production handoff message; a
    /// non-running dispatch fails with the `Task/4` protocol defect.
    pub fn compute(&self, input: &PaExtInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("ext-dispatch task {} is not running", input.task.index()),
                ),
            )];
        }
        vec![fail(
            input.task,
            DiagnosticDraft::unsupported(input.production.unsupported_message()),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input_of(production: PaExtProduction) -> PaExtInput {
        PaExtInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            production,
            tokens: vec![TokenId::from_index(0)],
        }
    }

    #[test]
    fn candidate_inventory_is_next_free() {
        assert_eq!(PA33_CANDIDATE_LOCAL, 27);
        assert_eq!(PA34_CANDIDATE_LOCAL, 28);
        assert_eq!(PA35_CANDIDATE_LOCAL, 29);
        assert_eq!(PA36_CANDIDATE_LOCAL, 30);
        assert_eq!(PA37_CANDIDATE_LOCAL, 31);
        assert_eq!(PA_EXT_DISPATCH_CANDIDATE_CHIP, ChipId(58));
        for (local, candidate) in [
            (27, PA33_TASK_KIND_CANDIDATE),
            (28, PA34_TASK_KIND_CANDIDATE),
            (29, PA35_TASK_KIND_CANDIDATE),
            (30, PA36_TASK_KIND_CANDIDATE),
            (31, PA37_TASK_KIND_CANDIDATE),
        ] {
            let kind = candidate.expect("candidate local must resolve");
            assert_eq!(kind.group(), TaskGroup::PARSE);
            assert_eq!(kind.local(), local);
        }
    }

    #[test]
    fn production_round_trips_through_kind() {
        for production in [
            PaExtProduction::Label,
            PaExtProduction::ExprStatement,
            PaExtProduction::StaticAssert,
            PaExtProduction::Attribute,
            PaExtProduction::ExtensionDispatch,
        ] {
            let kind = production.task_kind().expect("candidate must resolve");
            assert_eq!(PaExtProduction::of_kind(kind), Some(production));
        }
        // Frozen neighbours are never claimed by this shell.
        assert_eq!(PaExtProduction::of_kind(TaskKind::PARSE_RECOVERY), None);
        assert_eq!(PaExtProduction::of_kind(TaskKind::PARSE_TU), None);
    }

    #[test]
    fn every_production_fails_unsupported() {
        let chip = PaExtDispatchChip;
        for production in [
            PaExtProduction::Label,
            PaExtProduction::ExprStatement,
            PaExtProduction::StaticAssert,
            PaExtProduction::Attribute,
            PaExtProduction::ExtensionDispatch,
        ] {
            let out = chip.compute(&input_of(production));
            let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
                panic!("{} must fail", production.name());
            };
            assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
            assert_eq!(diagnostic.message, production.unsupported_message());
        }
    }

    #[test]
    fn extension_dispatch_names_the_t12_handoff() {
        let message = PaExtProduction::ExtensionDispatch.unsupported_message();
        for target in ["EX01", "EX02", "EX03", "EX04", "EX05", "EX13"] {
            assert!(message.contains(target), "PA37 handoff must name {target}");
        }
    }

    #[test]
    fn compute_rejects_non_running_task() {
        let chip = PaExtDispatchChip;
        let mut input = input_of(PaExtProduction::Label);
        input.state = TaskState::Ready;
        let out = chip.compute(&input);
        let [Proposal::Fail { diagnostic, .. }] = out.as_slice() else {
            panic!("non-running task must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }
}
