// ============================================================================
// chips/verify/vf_evidence.rs — T13 VF14 evidence-classify verifier
// (FROZEN `/38`): classifies the T00 gate outcome over the complete
// compile/link/run/check evidence vector.
//
// Frozen kind: `verification.evidence_classify` (`VERIFICATION` local 20)
// — first free code after VF01 (`store_invariant`, local 19);
// `VERIFICATION` owners start new codes at local 21. Frozen chip:
// `VF14_CHIP = ChipId(56)` — first free ID after `CL07_ASSERT_CHIP`.
// Stage 6, layer 6, `vf_evidence_slice()` registry (68 entries,
// cumulative over `const_branch_slice()`); no schema change; no allowlist
// rows (PASS completes `Ack`, every other path emits exactly one typed
// `Fail`, `writes` is empty).
//
// Semantics: every required stage present and passing is PASS; any
// present failure (in particular compile-ok with run-fail) is FAIL at
// the earliest failing stage; a required-but-missing stage is Reject
// (typed `Fail`, never PASS); an undecodable gate or evidence carrier
// is Reject (typed `Fail`, never PASS). PASS completes `Ack`; every
// other path emits exactly one typed `Fail` proposal. No batch logic
// and no H6 dependency: exactly one transition proposal per handle,
// never `Progress`/`Await*`.
//
// M1-scope conventions (frozen for this slice; richer schemas stay
// future work):
// - Instance (DEFECT-VF14-01): no `HostTestEvidence` record family and
//   no frozen test-instance schema exist yet (`RecordRef` has no
//   evidence/instance variant). The projector accepts ANY committed
//   record as the frozen instance ref (resolvability check only, never
//   interpretation).
// - Carrier (DEFECT-VF14-02): `ResultValue` carries no boolean; the
//   frozen map is `Ack` -> pass, `Diagnostic` -> fail, `Empty` ->
//   absent, and `Record`/`Records` -> `Undecodable` Reject (never PASS).
// - Registration (DEFECT-VF14-03, resolved by this freeze): kind local
//   20, chip 56, stage 6, layer 6, `Ack`-only manifest.
// ============================================================================

use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, VF14_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Frozen `/38` task kind served by this chip (never constructed from
/// raw bits so the group/local encoding stays frozen-canonical).
pub const VF14_TASK_KIND: TaskKind = TaskKind::VERIFICATION_EVIDENCE_CLASSIFY;

/// Pipeline stage gate: the required-depth level the test instance was
/// exercised through. Required evidence is the pipeline prefix ending at
/// the gate: Compile needs compile; Link needs compile+link; Run needs
/// compile+link+run; Check needs all four.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vf14Stage {
    /// Compile gate (raw 0).
    Compile,
    /// Link gate (raw 1).
    Link,
    /// Run gate (raw 2).
    Run,
    /// Check gate (raw 3).
    Check,
}

impl Vf14Stage {
    /// Canonical pipeline order (deterministic scan order).
    pub const ORDER: [Self; 4] = [Self::Compile, Self::Link, Self::Run, Self::Check];

    /// Decode a raw gate byte; anything above 3 is invalid.
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Compile),
            1 => Some(Self::Link),
            2 => Some(Self::Run),
            3 => Some(Self::Check),
            _ => None,
        }
    }

    /// Canonical raw encoding.
    pub const fn raw(self) -> u8 {
        match self {
            Self::Compile => 0,
            Self::Link => 1,
            Self::Run => 2,
            Self::Check => 3,
        }
    }

    /// Stable label used in diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Link => "link",
            Self::Run => "run",
            Self::Check => "check",
        }
    }

    /// Number of required evidence slots (pipeline prefix length).
    pub const fn required(self) -> usize {
        match self {
            Self::Compile => 1,
            Self::Link => 2,
            Self::Run => 3,
            Self::Check => 4,
        }
    }
}

/// One stage's evidence state in the narrow projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageEvidence {
    /// No evidence pinned for this stage.
    Missing,
    /// Evidence present and passing.
    Pass,
    /// Evidence present and failing.
    Fail,
    /// Evidence present but carried by a record with no frozen pass/fail
    /// meaning (see DEFECT-VF14-02).
    Undecodable,
}

impl StageEvidence {
    /// Whether any evidence is pinned for this stage.
    pub const fn present(self) -> bool {
        match self {
            Self::Missing => false,
            Self::Pass | Self::Fail | Self::Undecodable => true,
        }
    }

    /// Whether the pinned evidence passes (false for missing/failing and
    /// for undecodable carriers, which must never read as PASS).
    pub const fn passed(self) -> bool {
        match self {
            Self::Pass => true,
            Self::Missing | Self::Fail | Self::Undecodable => false,
        }
    }
}

/// Narrow projection for the evidence-classify computation.
#[derive(Clone, Debug)]
pub struct Vf14Input {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Raw gate byte as pinned (preserved for diagnostics).
    pub stage_raw: u8,
    /// Decoded gate (`None` = invalid raw gate).
    pub stage: Option<Vf14Stage>,
    /// Frozen test-instance ref (resolvability verified, never interpreted).
    pub instance: RecordRef,
    /// Compile evidence.
    pub compile: StageEvidence,
    /// Link evidence.
    pub link: StageEvidence,
    /// Run evidence.
    pub run: StageEvidence,
    /// Check evidence.
    pub check: StageEvidence,
}

/// Pure classification outcome with an accessor surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vf14Outcome {
    /// All required evidence present and passing (and no present failure
    /// anywhere in the vector).
    Pass,
    /// A present failure; names the earliest failing stage in pipeline
    /// order (compile-ok with run-fail reports `Run`, never PASS).
    Fail {
        /// Earliest failing stage.
        stage: Vf14Stage,
    },
    /// A required stage has no evidence (missing rejected, never PASS).
    Missing {
        /// Earliest missing required stage.
        stage: Vf14Stage,
    },
    /// A pinned carrier has no frozen pass/fail meaning (rejected, never
    /// PASS; see DEFECT-VF14-02).
    Undecodable {
        /// Earliest undecodable stage in pipeline order.
        stage: Vf14Stage,
    },
    /// The gate byte does not name a stage.
    InvalidStage {
        /// Raw gate byte as pinned.
        raw: u8,
    },
}

impl Vf14Outcome {
    /// Whether this outcome is the PASS verdict.
    pub const fn is_pass(self) -> bool {
        match self {
            Self::Pass => true,
            Self::Fail { .. }
            | Self::Missing { .. }
            | Self::Undecodable { .. }
            | Self::InvalidStage { .. } => false,
        }
    }

    /// Stable verdict label used in snapshots and diagnostics.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail { .. } => "fail",
            Self::Missing { .. } => "missing",
            Self::Undecodable { .. } => "undecodable",
            Self::InvalidStage { .. } => "invalid-stage",
        }
    }
}

/// Pure classification over the narrow projection (no bus access).
/// Precedence is total and deterministic: invalid gate, then earliest
/// missing required stage, then earliest undecodable slot anywhere, then
/// earliest failing slot anywhere, else PASS.
pub fn classify_vf14_evidence(input: &Vf14Input) -> Vf14Outcome {
    let Some(gate) = input.stage else {
        return Vf14Outcome::InvalidStage {
            raw: input.stage_raw,
        };
    };
    let evidence = [input.compile, input.link, input.run, input.check];
    let mut i = 0;
    while i < gate.required() {
        if evidence[i] == StageEvidence::Missing {
            return Vf14Outcome::Missing {
                stage: Vf14Stage::ORDER[i],
            };
        }
        i += 1;
    }
    let mut j = 0;
    while j < evidence.len() {
        if evidence[j] == StageEvidence::Undecodable {
            return Vf14Outcome::Undecodable {
                stage: Vf14Stage::ORDER[j],
            };
        }
        j += 1;
    }
    let mut k = 0;
    while k < evidence.len() {
        if evidence[k] == StageEvidence::Fail {
            return Vf14Outcome::Fail {
                stage: Vf14Stage::ORDER[k],
            };
        }
        k += 1;
    }
    Vf14Outcome::Pass
}

/// Decode one committed stage-result value into evidence state
/// (frozen M1-scope carrier map, see DEFECT-VF14-02).
pub fn decode_result_value(value: &ResultValue) -> StageEvidence {
    match value {
        ResultValue::Empty => StageEvidence::Missing,
        ResultValue::Ack => StageEvidence::Pass,
        ResultValue::Diagnostic(_) => StageEvidence::Fail,
        ResultValue::Record(_) | ResultValue::Records(_) => StageEvidence::Undecodable,
    }
}

/// Decode a gate `ConstRecord` body into its raw byte and stage.
/// Requires exactly one non-negative byte; anything else is an invalid
/// gate (raw `0xFF` sentinel when no single byte exists).
pub fn decode_gate_raw(negative: bool, bytes: &[u8]) -> (u8, Option<Vf14Stage>) {
    if bytes.len() != 1 {
        return (0xFF, None);
    }
    let raw = bytes[0];
    if negative {
        return (raw, None);
    }
    (raw, Vf14Stage::from_raw(raw))
}

/// Check that a frozen instance ref resolves to a committed record.
/// Existence only; the body is never interpreted (see DEFECT-VF14-01).
fn instance_resolves(bus: &crate::bus::CompilerBus, instance: &RecordRef) -> bool {
    match *instance {
        RecordRef::Source(id) => bus.arenas.sources.get(id).is_ok(),
        RecordRef::Span(id) => bus.arenas.spans.get(id).is_ok(),
        RecordRef::Expansion(id) => bus.arenas.expansions.get(id).is_ok(),
        RecordRef::PpToken(id) => bus.arenas.pp_tokens.get(id).is_ok(),
        RecordRef::Token(id) => bus.arenas.tokens.get(id).is_ok(),
        RecordRef::Name(id) => id.index() < bus.intern.len(),
        RecordRef::Scope(id) => bus.arenas.scopes.get(id).is_ok(),
        RecordRef::Symbol(id) => bus.arenas.symbols.get(id).is_ok(),
        RecordRef::Type(id) => bus.arenas.types.get(id).is_ok(),
        RecordRef::Node(id) => bus.arenas.nodes.get(id).is_ok(),
        RecordRef::Const(id) => bus.arenas.consts.get(id).is_ok(),
        RecordRef::Layout(id) => bus.arenas.layouts.get(id).is_ok(),
        RecordRef::Init(id) => bus.arenas.inits.get(id).is_ok(),
        RecordRef::Function(id) => bus.arenas.functions.get(id).is_ok(),
        RecordRef::Block(id) => bus.arenas.blocks.get(id).is_ok(),
        RecordRef::Value(id) => bus.arenas.values.get(id).is_ok(),
        RecordRef::Instruction(id) => bus.arenas.instructions.get(id).is_ok(),
        RecordRef::VReg(id) => bus.arenas.vregs.get(id).is_ok(),
        RecordRef::Continuation(id) => bus.arenas.continuations.get(id).is_ok(),
        RecordRef::Task(id) => bus.arenas.tasks.get(id).is_ok(),
        RecordRef::Result(id) => bus.arenas.results.get(id).is_ok(),
        RecordRef::Diagnostic(id) => bus.arenas.diagnostics.get(id).is_ok(),
        RecordRef::HostRequest(id) => bus.arenas.host_requests.get(id).is_ok(),
        RecordRef::Artifact(id) => bus.arenas.artifacts.get(id).is_ok(),
        RecordRef::Literal(id) => bus.arenas.literals.get(id).is_ok(),
        RecordRef::Sem(id) => bus.arenas.sem.get(id).is_ok(),
        RecordRef::ScopeEvent(id) => bus.arenas.scope_events.get(id).is_ok(),
        RecordRef::Macro(id) => bus.arenas.macros.get(id).is_ok(),
    }
}

/// Build the narrow projection for one dispatched task.
///
/// Frozen pin convention: exactly six payload refs —
/// `[instance, gate, compile, link, run, check]` — where the gate is a
/// `RecordRef::Const` decoded by [`decode_gate_raw`] and each evidence slot
/// is a `RecordRef::Result` decoded by [`decode_result_value`]. Any shape
/// violation fails loudly here (never PASS); gate/evidence content verdicts
/// flow through [`classify_vf14_evidence`] instead.
pub fn project_vf14_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<Vf14Input, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("vf14 of unknown task {}", task.index())))?;
    if record.kind != VF14_TASK_KIND {
        return Err(protocol_fault(format!(
            "vf14 task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 6 {
        return Err(protocol_fault(format!(
            "vf14 task {} payload must carry exactly six refs \
             [instance, gate, compile, link, run, check]",
            task.index()
        )));
    }
    let refs = record.payload.refs.clone();
    let state = record.state.clone();
    let instance = refs[0];
    if !instance_resolves(bus, &instance) {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf14 reads dangling frozen instance {}", instance.label()),
        ));
    }
    let gate_id = match refs[1] {
        RecordRef::Const(id) => id,
        other => {
            return Err(protocol_fault(format!(
                "vf14 task {} gate pin must be a const ref, found {}",
                task.index(),
                other.label()
            )));
        }
    };
    let gate_body = bus.arenas.consts.get(gate_id).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf14 reads missing gate const {}", gate_id.index()),
        )
    })?;
    let (stage_raw, stage) = decode_gate_raw(gate_body.negative, &gate_body.value);
    let mut evidence = [StageEvidence::Missing; 4];
    let mut slot = 0;
    while slot < 4 {
        let result_id = match refs[2 + slot] {
            RecordRef::Result(id) => id,
            other => {
                return Err(protocol_fault(format!(
                    "vf14 task {} evidence pin {} must be a result ref, found {}",
                    task.index(),
                    slot,
                    other.label()
                )));
            }
        };
        let result_body = bus.arenas.results.get(result_id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("vf14 reads missing evidence result {}", result_id.index()),
            )
        })?;
        evidence[slot] = decode_result_value(&result_body.value);
        slot += 1;
    }
    Ok(Vf14Input {
        task,
        state,
        stage_raw,
        stage,
        instance,
        compile: evidence[0],
        link: evidence[1],
        run: evidence[2],
        check: evidence[3],
    })
}

/// Build the typed `Fail` proposal for a non-PASS verdict (never PASS).
fn evidence_fail(task: TaskId, detail: &str) -> Proposal {
    fail(
        task,
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf14: rejected evidence ({detail})"),
        ),
    )
}

/// The VF14 evidence-classify verifier (frozen; see file header).
pub struct Vf14Chip;

/// Compile-time statelessness check (mirrors the `silicon_chip!` size-0
/// assertion; `WorkerRegistry` re-enforces this at registration).
const _: () = assert!(std::mem::size_of::<Vf14Chip>() == 0);

impl Worker for Vf14Chip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: VF14_CHIP,
            chip_name: "Vf14Chip",
            group: TaskGroup::VERIFICATION,
            task_kinds: vec![VF14_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Tasks, "results"),
                FieldPath::new(StoreId::Tasks, "completed"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Sources, "spans"),
                FieldPath::new(StoreId::Sources, "expansion"),
                FieldPath::new(StoreId::Names, "entries"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Lex, "literals"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Symbols, "symbols"),
                FieldPath::new(StoreId::Symbols, "scopes"),
                FieldPath::new(StoreId::Symbols, "scope_events"),
                FieldPath::new(StoreId::Sem, "records"),
                FieldPath::new(StoreId::Constants, "records"),
                FieldPath::new(StoreId::Ir, "functions"),
                FieldPath::new(StoreId::Ir, "blocks"),
                FieldPath::new(StoreId::Ir, "values"),
                FieldPath::new(StoreId::Ir, "instructions"),
                FieldPath::new(StoreId::Diagnostics, "entries"),
                FieldPath::new(StoreId::Artifacts, "fragments"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c38_vf14.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_vf14_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl Vf14Chip {
    /// Pure outcome accessor over the narrow projection (no bus access).
    pub fn outcome(&self, input: &Vf14Input) -> Vf14Outcome {
        classify_vf14_evidence(input)
    }

    /// Pure evidence classification over the narrow projection (no bus
    /// access, no store writes): PASS completes `Ack`, every other verdict
    /// emits exactly one typed `Fail`.
    pub fn compute(&self, input: &Vf14Input) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("vf14 task {} is not running", input.task.index()),
                ),
            )];
        }
        match self.outcome(input) {
            Vf14Outcome::Pass => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Vf14Outcome::Fail { stage } => vec![evidence_fail(
                input.task,
                &format!("stage {} failed", stage.name()),
            )],
            Vf14Outcome::Missing { stage } => vec![evidence_fail(
                input.task,
                &format!("missing required {} evidence", stage.name()),
            )],
            Vf14Outcome::Undecodable { stage } => vec![evidence_fail(
                input.task,
                &format!("undecodable {} carrier (DEFECT-VF14-02)", stage.name()),
            )],
            Vf14Outcome::InvalidStage { raw } => vec![evidence_fail(
                input.task,
                &format!("invalid stage gate {raw}"),
            )],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ConstId, DiagnosticId};

    fn input_for(
        stage_raw: u8,
        stage: Option<Vf14Stage>,
        compile: StageEvidence,
        link: StageEvidence,
        run: StageEvidence,
        check: StageEvidence,
    ) -> Vf14Input {
        Vf14Input {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            stage_raw,
            stage,
            instance: RecordRef::Task(TaskId::from_index(1)),
            compile,
            link,
            run,
            check,
        }
    }

    fn full_pass_gate(gate: Vf14Stage) -> Vf14Input {
        input_for(
            gate.raw(),
            Some(gate),
            StageEvidence::Pass,
            StageEvidence::Pass,
            StageEvidence::Pass,
            StageEvidence::Pass,
        )
    }

    #[test]
    fn complete_passing_evidence_is_pass() {
        let chip = Vf14Chip;
        let input = full_pass_gate(Vf14Stage::Check);
        assert_eq!(chip.outcome(&input), Vf14Outcome::Pass);
        assert!(chip.outcome(&input).is_pass());
        let proposals = chip.compute(&input);
        assert_eq!(proposals.len(), 1);
        match &proposals[0] {
            Proposal::Complete { task, value } => {
                assert_eq!(*task, input.task);
                assert_eq!(*value, ResultValue::Ack);
            }
            other => panic!("PASS must complete Ack, found {other:?}"),
        }
    }

    #[test]
    fn compile_ok_run_fail_is_fail_run_never_pass() {
        let chip = Vf14Chip;
        let input = input_for(
            2,
            Some(Vf14Stage::Run),
            StageEvidence::Pass,
            StageEvidence::Pass,
            StageEvidence::Fail,
            StageEvidence::Missing,
        );
        assert_eq!(
            chip.outcome(&input),
            Vf14Outcome::Fail {
                stage: Vf14Stage::Run
            }
        );
        assert!(!chip.outcome(&input).is_pass());
        let proposals = chip.compute(&input);
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn failure_beyond_gate_still_fails() {
        let chip = Vf14Chip;
        let input = input_for(
            0,
            Some(Vf14Stage::Compile),
            StageEvidence::Pass,
            StageEvidence::Missing,
            StageEvidence::Fail,
            StageEvidence::Missing,
        );
        assert_eq!(
            chip.outcome(&input),
            Vf14Outcome::Fail {
                stage: Vf14Stage::Run
            }
        );
    }

    #[test]
    fn earliest_failure_wins_in_pipeline_order() {
        let chip = Vf14Chip;
        let input = input_for(
            3,
            Some(Vf14Stage::Check),
            StageEvidence::Pass,
            StageEvidence::Fail,
            StageEvidence::Fail,
            StageEvidence::Pass,
        );
        assert_eq!(
            chip.outcome(&input),
            Vf14Outcome::Fail {
                stage: Vf14Stage::Link
            }
        );
    }

    #[test]
    fn missing_required_link_is_rejected_never_pass() {
        let chip = Vf14Chip;
        let input = input_for(
            3,
            Some(Vf14Stage::Check),
            StageEvidence::Pass,
            StageEvidence::Missing,
            StageEvidence::Pass,
            StageEvidence::Pass,
        );
        assert_eq!(
            chip.outcome(&input),
            Vf14Outcome::Missing {
                stage: Vf14Stage::Link
            }
        );
        let proposals = chip.compute(&input);
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn missing_beyond_gate_is_ignored() {
        let chip = Vf14Chip;
        let input = input_for(
            0,
            Some(Vf14Stage::Compile),
            StageEvidence::Pass,
            StageEvidence::Missing,
            StageEvidence::Missing,
            StageEvidence::Missing,
        );
        assert_eq!(chip.outcome(&input), Vf14Outcome::Pass);
    }

    #[test]
    fn invalid_stage_gate_fails() {
        let chip = Vf14Chip;
        let input = input_for(
            9,
            None,
            StageEvidence::Pass,
            StageEvidence::Pass,
            StageEvidence::Pass,
            StageEvidence::Pass,
        );
        assert_eq!(chip.outcome(&input), Vf14Outcome::InvalidStage { raw: 9 });
        let proposals = chip.compute(&input);
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn undecodable_carrier_is_rejected_never_pass() {
        let chip = Vf14Chip;
        let input = input_for(
            3,
            Some(Vf14Stage::Check),
            StageEvidence::Pass,
            StageEvidence::Pass,
            StageEvidence::Undecodable,
            StageEvidence::Pass,
        );
        assert_eq!(
            chip.outcome(&input),
            Vf14Outcome::Undecodable {
                stage: Vf14Stage::Run
            }
        );
        let proposals = chip.compute(&input);
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn missing_required_takes_precedence_over_later_failure() {
        let chip = Vf14Chip;
        let input = input_for(
            3,
            Some(Vf14Stage::Check),
            StageEvidence::Pass,
            StageEvidence::Missing,
            StageEvidence::Fail,
            StageEvidence::Pass,
        );
        assert_eq!(
            chip.outcome(&input),
            Vf14Outcome::Missing {
                stage: Vf14Stage::Link
            }
        );
    }

    #[test]
    fn non_running_task_fails_without_classifying() {
        let chip = Vf14Chip;
        let mut input = full_pass_gate(Vf14Stage::Check);
        input.state = TaskState::Ready;
        let proposals = chip.compute(&input);
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn classification_is_deterministic() {
        let chip = Vf14Chip;
        let inputs = [
            full_pass_gate(Vf14Stage::Check),
            full_pass_gate(Vf14Stage::Compile),
            input_for(
                2,
                Some(Vf14Stage::Run),
                StageEvidence::Pass,
                StageEvidence::Pass,
                StageEvidence::Fail,
                StageEvidence::Missing,
            ),
            input_for(
                9,
                None,
                StageEvidence::Missing,
                StageEvidence::Missing,
                StageEvidence::Missing,
                StageEvidence::Missing,
            ),
        ];
        for input in &inputs {
            let first = chip.outcome(input);
            let second = classify_vf14_evidence(input);
            assert_eq!(first, second);
            assert_eq!(chip.compute(input).len(), 1);
        }
    }

    #[test]
    fn result_value_carrier_map() {
        assert_eq!(decode_result_value(&ResultValue::Ack), StageEvidence::Pass);
        assert_eq!(
            decode_result_value(&ResultValue::Diagnostic(DiagnosticId::from_index(0))),
            StageEvidence::Fail
        );
        assert_eq!(
            decode_result_value(&ResultValue::Empty),
            StageEvidence::Missing
        );
        assert_eq!(
            decode_result_value(&ResultValue::Record(RecordRef::Const(ConstId::from_index(
                0
            )))),
            StageEvidence::Undecodable
        );
        assert_eq!(
            decode_result_value(&ResultValue::Records(vec![])),
            StageEvidence::Undecodable
        );
    }

    #[test]
    fn gate_byte_map() {
        assert_eq!(decode_gate_raw(false, &[0]), (0, Some(Vf14Stage::Compile)));
        assert_eq!(decode_gate_raw(false, &[1]), (1, Some(Vf14Stage::Link)));
        assert_eq!(decode_gate_raw(false, &[2]), (2, Some(Vf14Stage::Run)));
        assert_eq!(decode_gate_raw(false, &[3]), (3, Some(Vf14Stage::Check)));
        assert_eq!(decode_gate_raw(false, &[4]), (4, None));
        assert_eq!(decode_gate_raw(true, &[2]), (2, None));
        assert_eq!(decode_gate_raw(false, &[]), (0xFF, None));
        assert_eq!(decode_gate_raw(false, &[1, 2]), (0xFF, None));
    }

    #[test]
    fn stage_encodings_round_trip() {
        for stage in Vf14Stage::ORDER {
            assert_eq!(Vf14Stage::from_raw(stage.raw()), Some(stage));
        }
        assert_eq!(Vf14Stage::from_raw(4), None);
        assert_eq!(Vf14Stage::from_raw(0xFF), None);
        assert_eq!(Vf14Stage::Compile.required(), 1);
        assert_eq!(Vf14Stage::Link.required(), 2);
        assert_eq!(Vf14Stage::Run.required(), 3);
        assert_eq!(Vf14Stage::Check.required(), 4);
    }

    #[test]
    fn vf14_kind_is_verification_local_20() {
        assert_eq!(VF14_TASK_KIND, TaskKind::VERIFICATION_EVIDENCE_CLASSIFY);
        let kind = VF14_TASK_KIND;
        assert_eq!(kind.group(), TaskGroup::VERIFICATION);
        assert_eq!(kind.local(), 20);
    }

    #[test]
    fn evidence_presence_semantics() {
        assert!(!StageEvidence::Missing.present());
        assert!(!StageEvidence::Missing.passed());
        assert!(StageEvidence::Pass.present());
        assert!(StageEvidence::Pass.passed());
        assert!(StageEvidence::Fail.present());
        assert!(!StageEvidence::Fail.passed());
        assert!(StageEvidence::Undecodable.present());
        assert!(!StageEvidence::Undecodable.passed());
    }
}
