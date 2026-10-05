// ============================================================================
// c38_vf14.rs — Wave 3 (`/38`) VF14 evidence-slice acceptance.
//
// Covers the frozen closure: the `verification.evidence_classify` kind
// (local 20), kind-to-stage assignment (stage 6), the VF14 manifest
// (Ack-only with zero writes and no allowlist rows), the complete
// compile/link/run/check classification (all required present and
// passing is PASS; any present failure — in particular compile-ok with
// run-fail — is FAIL at the earliest failing stage, never PASS; a
// required-but-missing stage is rejected, never PASS; an undecodable
// carrier or an invalid stage gate is rejected, never PASS), the total
// precedence order (invalid gate, then earliest missing required stage,
// then earliest undecodable slot, then earliest failing slot), bus
// dispatch (PASS completes `Ack`, every other verdict fails, replay
// determinism), and stage/layer gates.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, ConstRecord};
use cc_silicon_compiler::chips::{
    classify_vf14_evidence, decode_gate_raw, decode_result_value, handler_for, project_vf14_input,
    StageEvidence, Vf14Chip, Vf14Input, Vf14Outcome, Vf14Stage, Worker, WorkerRegistry,
    VF14_TASK_KIND,
};
use cc_silicon_compiler::ids::{ChipId, ConstId, DiagnosticId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    check_stage_layer_agreement, is_vf_evidence_slice_kind, stage_of, ManifestRegistry,
    StoreSchema, STORE_OWNER_ALLOWLIST, VF14_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::Snapshot;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, ResultRecord, ResultValue, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry,
    TaskState,
};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_vf14(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::vf_evidence_slice();
    bus.schema = StoreSchema::pp_slice();
    bus.registrations
        .register(Vf14Chip.manifest(), &bus.schema, &bus.kinds)
        .unwrap();
    bus.routing
        .register(TaskKind::VERIFICATION_EVIDENCE_CLASSIFY, VF14_CHIP, 6)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(Vf14Chip).unwrap();
    workers
}

fn bootstrap(bus: &mut CompilerBus, kind: TaskKind, owner: ChipId, payload: Payload) -> TaskId {
    bus.bootstrap_task(TaskDraft {
        kind,
        owner,
        parent: None,
        payload,
        continuation: None,
    })
    .unwrap()
}

fn tick(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
) -> cc_silicon_compiler::routing::TickReport {
    RoutingShell
        .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
        .unwrap()
}

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
        instance: RecordRef::Const(ConstId::from_index(0)),
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

// --- 1. frozen kind/stage/registry/manifest -----------------------------------

#[test]
fn evidence_kind_stage_registry_manifest_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY.group(),
        TaskGroup::VERIFICATION
    );
    assert_eq!(TaskKind::VERIFICATION_EVIDENCE_CLASSIFY.local(), 20);
    assert_eq!(VF14_TASK_KIND, TaskKind::VERIFICATION_EVIDENCE_CLASSIFY);
    assert!(is_vf_evidence_slice_kind(
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY
    ));
    assert_eq!(stage_of(TaskKind::VERIFICATION_EVIDENCE_CLASSIFY), Some(6));
    assert!(!is_vf_evidence_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_vf_evidence_slice_kind(
        TaskKind::VERIFICATION_STORE_INVARIANT
    ));
    assert!(!is_vf_evidence_slice_kind(
        TaskKind::CONSTANT_CONST_STATIC_ASSERT
    ));
    // The evidence registry extends the `/37` head linearly (67 → 68);
    // `VERIFICATION` owners start new codes at local 21.
    assert_eq!(TaskKindRegistry::const_branch_slice().len(), 67);
    let registry = TaskKindRegistry::vf_evidence_slice();
    assert_eq!(registry.len(), 68);
    assert_eq!(
        registry
            .lookup(TaskKind::VERIFICATION_EVIDENCE_CLASSIFY)
            .unwrap()
            .name,
        "verification.evidence_classify"
    );
    assert_eq!(
        stage_of(TaskKind::new(TaskGroup::VERIFICATION, 21).unwrap()),
        None
    );
    // The chip is Ack-only: zero writes, no allowlist rows.
    assert_eq!(VF14_CHIP, ChipId(56));
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 32);
    assert!(
        !STORE_OWNER_ALLOWLIST
            .iter()
            .any(|&(owner, _, _, _)| owner == VF14_CHIP),
        "Ack-only chip must hold no allowlist row"
    );
    let manifest = Vf14Chip.manifest();
    assert!(manifest.writes.is_empty());
    assert!(manifest.deterministic);
    assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
    assert!(manifest.declares_read(StoreId::Tasks, "results"));
    assert_eq!(manifest.tests, vec!["compiler/tests/c38_vf14.rs"]);
    assert_eq!(manifest.task_kinds, vec![VF14_TASK_KIND]);
    let routing = {
        let mut bus = new_bus();
        install_vf14(&mut bus);
        bus.routing
    };
    assert!(check_stage_layer_agreement(&manifest, &routing).is_ok());
}

// --- 2. complete passing evidence --------------------------------------------

#[test]
fn complete_evidence_passes_with_ack() {
    let chip = Vf14Chip;
    for gate in Vf14Stage::ORDER {
        let input = full_pass_gate(gate);
        assert_eq!(chip.outcome(&input), Vf14Outcome::Pass);
        assert!(chip.outcome(&input).is_pass());
        assert_eq!(classify_vf14_evidence(&input), Vf14Outcome::Pass);
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
    // Evidence past the gate is still evidence: a compile-gate pass with
    // no later evidence pinned is PASS.
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

// --- 3. compile-ok with run-fail is FAIL -------------------------------------

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
    // The earliest failing stage wins in pipeline order.
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

// --- 4. missing required evidence is rejected --------------------------------

#[test]
fn missing_required_evidence_is_rejected_never_pass() {
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
    assert!(!chip.outcome(&input).is_pass());
    let proposals = chip.compute(&input);
    assert_eq!(proposals.len(), 1);
    assert!(matches!(proposals[0], Proposal::Fail { .. }));
}

// --- 5. invalid stage gate is rejected ---------------------------------------

#[test]
fn invalid_stage_gate_is_rejected_never_pass() {
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
    assert!(!chip.outcome(&input).is_pass());
    let proposals = chip.compute(&input);
    assert_eq!(proposals.len(), 1);
    assert!(matches!(proposals[0], Proposal::Fail { .. }));
    assert_eq!(decode_gate_raw(false, &[4]), (4, None));
    assert_eq!(decode_gate_raw(true, &[2]), (2, None));
    assert_eq!(decode_gate_raw(false, &[]), (0xFF, None));
}

// --- 6. undecodable carrier is rejected --------------------------------------

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
    assert!(!chip.outcome(&input).is_pass());
    let proposals = chip.compute(&input);
    assert_eq!(proposals.len(), 1);
    assert!(matches!(proposals[0], Proposal::Fail { .. }));
    // The frozen M1-scope carrier map (DEFECT-VF14-02): `Record` and
    // `Records` have no pass/fail meaning and must never read as PASS.
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

// --- 7. total precedence ------------------------------------------------------

#[test]
fn precedence_is_total_invalid_missing_undecodable_fail() {
    let chip = Vf14Chip;
    // An invalid gate beats every evidence verdict.
    let input = input_for(
        0xFF,
        None,
        StageEvidence::Missing,
        StageEvidence::Undecodable,
        StageEvidence::Fail,
        StageEvidence::Missing,
    );
    assert!(matches!(
        chip.outcome(&input),
        Vf14Outcome::InvalidStage { .. }
    ));
    // A missing required stage beats later undecodable and failing slots.
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
    // An undecodable slot anywhere beats a present failure.
    let input = input_for(
        3,
        Some(Vf14Stage::Check),
        StageEvidence::Fail,
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
    // A present failure past the gate still fails (never PASS).
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
    // Classification is deterministic across repeated calls.
    for input in [
        full_pass_gate(Vf14Stage::Check),
        input_for(
            2,
            Some(Vf14Stage::Run),
            StageEvidence::Pass,
            StageEvidence::Pass,
            StageEvidence::Fail,
            StageEvidence::Missing,
        ),
    ] {
        assert_eq!(chip.outcome(&input), classify_vf14_evidence(&input));
        assert_eq!(chip.compute(&input).len(), 1);
        assert_eq!(chip.compute(&input), chip.compute(&input));
    }
}

// --- 8. bus dispatch ----------------------------------------------------------

fn seed_gate(bus: &mut CompilerBus, raw: u8) -> cc_silicon_compiler::ids::ConstId {
    let limits = bus.limits();
    bus.arenas
        .consts
        .alloc(
            ConstRecord {
                value: vec![raw],
                negative: false,
            },
            &limits,
        )
        .unwrap()
}

fn seed_result(bus: &mut CompilerBus, value: ResultValue) -> cc_silicon_compiler::ids::ResultId {
    let limits = bus.limits();
    bus.arenas
        .results
        .alloc(
            ResultRecord {
                task: TaskId::from_index(0),
                kind: TaskKind::CONTROL_NOOP,
                value,
                version: 0,
                consumed: false,
            },
            &limits,
        )
        .unwrap()
}

fn evidence_payload(
    instance: RecordRef,
    gate: RecordRef,
    compile: RecordRef,
    link: RecordRef,
    run: RecordRef,
    check: RecordRef,
) -> Payload {
    Payload::from_refs(vec![instance, gate, compile, link, run, check])
}

fn run_scenario() -> (Vec<u8>, usize, usize) {
    let mut bus = new_bus();
    install_vf14(&mut bus);
    // Gate consts double as the resolvability-only frozen instance ref
    // (DEFECT-VF14-01: the instance body is never interpreted).
    let gate_check = seed_gate(&mut bus, 3);
    let gate_run = seed_gate(&mut bus, 2);
    let pass = seed_result(&mut bus, ResultValue::Ack);
    let fail = seed_result(
        &mut bus,
        ResultValue::Diagnostic(DiagnosticId::from_index(0)),
    );
    let absent = seed_result(&mut bus, ResultValue::Empty);
    let undecodable = seed_result(&mut bus, ResultValue::Record(RecordRef::Const(gate_check)));
    let r = |id| RecordRef::Result(id);
    let c = |id| RecordRef::Const(id);
    // PASS: check gate over four passing stages → `Ack`.
    bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY,
        VF14_CHIP,
        evidence_payload(
            c(gate_check),
            c(gate_check),
            r(pass),
            r(pass),
            r(pass),
            r(pass),
        ),
    );
    // FAIL: run gate with compile-ok but run-fail → typed `Fail`.
    bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY,
        VF14_CHIP,
        evidence_payload(
            c(gate_run),
            c(gate_run),
            r(pass),
            r(pass),
            r(fail),
            r(absent),
        ),
    );
    // MISSING: check gate with link absent → typed `Fail` (never PASS).
    bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY,
        VF14_CHIP,
        evidence_payload(
            c(gate_check),
            c(gate_check),
            r(pass),
            r(absent),
            r(pass),
            r(pass),
        ),
    );
    // UNDECODABLE: check gate with a record carrier → typed `Fail`.
    bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY,
        VF14_CHIP,
        evidence_payload(
            c(gate_check),
            c(gate_check),
            r(pass),
            r(pass),
            r(undecodable),
            r(pass),
        ),
    );
    let workers = workers();
    let mut completed = 0;
    let mut failed = 0;
    let mut acked = 0;
    for _ in 0..16 {
        let report = tick(&mut bus, &workers);
        match &report.outcome {
            TickOutcome::Executed { commit, .. } => {
                for (_, result) in &commit.completed {
                    match &bus.arenas.results.get(*result).unwrap().value {
                        ResultValue::Ack => acked += 1,
                        other => panic!("expected Ack, got {other:?}"),
                    }
                }
                completed += commit.completed.len();
                failed += commit.failed.len();
            }
            other => panic!("expected executed, got {other:?}"),
        }
        if completed + failed == 4 {
            break;
        }
    }
    assert_eq!(acked, 1);
    (Snapshot::capture(&bus).bytes().to_vec(), completed, failed)
}

#[test]
fn bus_dispatch_commits_pass_and_fail_and_replays_deterministically() {
    let (first, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (1, 3));
    let (second, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (1, 3));
    assert_eq!(first, second);
}

// --- 9. loud failures ----------------------------------------------------------

#[test]
fn non_running_and_malformed_payloads_fail_loudly() {
    let chip = Vf14Chip;
    // A non-running task fails without classifying.
    let mut input = full_pass_gate(Vf14Stage::Check);
    input.state = TaskState::Ready;
    let proposals = chip.compute(&input);
    assert_eq!(proposals.len(), 1);
    assert!(matches!(proposals[0], Proposal::Fail { .. }));
    // A wrong-arity payload fails loudly through the adapter (never PASS).
    let mut bus = new_bus();
    install_vf14(&mut bus);
    let gate = seed_gate(&mut bus, 3);
    let pass = seed_result(&mut bus, ResultValue::Ack);
    let task = bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY,
        VF14_CHIP,
        Payload::from_refs(vec![RecordRef::Const(gate), RecordRef::Result(pass)]),
    );
    let proposals = chip.handle(task, &bus);
    assert_eq!(proposals.len(), 1);
    assert!(matches!(proposals[0], Proposal::Fail { .. }));
    // A dangling frozen instance ref fails loudly (never PASS).
    let task = bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY,
        VF14_CHIP,
        evidence_payload(
            RecordRef::Task(TaskId::from_index(9999)),
            RecordRef::Const(gate),
            RecordRef::Result(pass),
            RecordRef::Result(pass),
            RecordRef::Result(pass),
            RecordRef::Result(pass),
        ),
    );
    let proposals = chip.handle(task, &bus);
    assert_eq!(proposals.len(), 1);
    assert!(matches!(proposals[0], Proposal::Fail { .. }));
    // The projector accepts the well-formed task.
    let task = bootstrap(
        &mut bus,
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY,
        VF14_CHIP,
        evidence_payload(
            RecordRef::Const(gate),
            RecordRef::Const(gate),
            RecordRef::Result(pass),
            RecordRef::Result(pass),
            RecordRef::Result(pass),
            RecordRef::Result(pass),
        ),
    );
    let input = project_vf14_input(&bus, task).expect("well-formed projection");
    assert_eq!(chip.outcome(&input), Vf14Outcome::Pass);
}

// --- 10. gates -----------------------------------------------------------------

#[test]
fn evidence_stage_layer_manifest_gates() {
    // A wrong layer is refused on the driver path.
    let mut mismatched = new_bus();
    mismatched.kinds = TaskKindRegistry::vf_evidence_slice();
    mismatched.schema = StoreSchema::pp_slice();
    mismatched
        .registrations
        .register(Vf14Chip.manifest(), &mismatched.schema, &mismatched.kinds)
        .unwrap();
    mismatched
        .routing
        .register(TaskKind::VERIFICATION_EVIDENCE_CLASSIFY, VF14_CHIP, 9)
        .unwrap();
    let task = bootstrap(
        &mut mismatched,
        TaskKind::VERIFICATION_EVIDENCE_CLASSIFY,
        VF14_CHIP,
        Payload::empty(),
    );
    let error = cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers()).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // The pre-`/38` registry does not know the frozen kind, so the
    // manifest is rejected there (frozen kinds are never re-registered).
    let mut stale = ManifestRegistry::new();
    let kinds = TaskKindRegistry::const_branch_slice();
    let schema = StoreSchema::pp_slice();
    assert!(stale
        .register(Vf14Chip.manifest(), &schema, &kinds)
        .is_err());
}
