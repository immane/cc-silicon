// ============================================================================
// c37_const_branch.rs — Wave 3 (`/37`) T08 const-branch-slice acceptance.
//
// Covers the frozen closure: the `const_branch_and` (local 17),
// `const_branch_or` (18), `const_branch_cond` (19), and
// `const_static_assert` (20) kinds, kind-to-stage assignment
// (stage 2), the CL04/CL07 manifests (all Ack-only with zero writes
// and no allowlist rows), the selected-branch certification (`&&` /
// `||` canonical `0`/`1`, `?:` magnitude passthrough), short-circuit
// skipping of the unselected operand (never subset-checked, never
// budget-checked, may dangle), the static-assert outcomes (nonzero
// pass, zero fail, non-ICE `NotConstantExpression`), the M1 subset
// gate, the bit-budget overflow gate, bus dispatch (all `Ack`, replay
// determinism), and stage/layer gates.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, LiteralRecord};
use cc_silicon_compiler::chips::{
    handler_for, BranchAndChip, BranchCondChip, BranchInput, BranchOp, BranchOrChip, BranchValue,
    StaticAssertChip, Worker, WorkerRegistry, CL04_AND_TASK_KIND, CL04_COND_TASK_KIND,
    CL04_OR_TASK_KIND, CL07_ASSERT_TASK_KIND,
};
use cc_silicon_compiler::chips::{AssertCond, AssertInput, BranchOperand};
use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticCode};
use cc_silicon_compiler::ids::{ChipId, LiteralId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    check_stage_layer_agreement, is_const_branch_slice_kind, stage_of, CL04_AND_CHIP,
    CL04_COND_CHIP, CL04_OR_CHIP, CL07_ASSERT_CHIP, STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, ResultValue, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry, TaskState,
};
use std::collections::BTreeMap;

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_const_branch(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::StoreSchema;
    bus.kinds = TaskKindRegistry::const_branch_slice();
    bus.schema = StoreSchema::pa_slice();
    for chip in [
        &BranchAndChip as &dyn Worker,
        &BranchOrChip,
        &BranchCondChip,
        &StaticAssertChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    for (kind, chip) in [
        (TaskKind::CONSTANT_CONST_BRANCH_AND, CL04_AND_CHIP),
        (TaskKind::CONSTANT_CONST_BRANCH_OR, CL04_OR_CHIP),
        (TaskKind::CONSTANT_CONST_BRANCH_COND, CL04_COND_CHIP),
        (TaskKind::CONSTANT_CONST_STATIC_ASSERT, CL07_ASSERT_CHIP),
    ] {
        bus.routing.register(kind, chip, 2).unwrap();
    }
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(BranchAndChip).unwrap();
    workers.register(BranchOrChip).unwrap();
    workers.register(BranchCondChip).unwrap();
    workers.register(StaticAssertChip).unwrap();
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

fn is_ack(proposals: &[Proposal]) -> bool {
    matches!(
        proposals,
        [Proposal::Complete {
            value: ResultValue::Ack,
            ..
        }]
    )
}

/// An in-subset M1 literal fixture with the given magnitude bytes.
fn subset_literal(magnitude: Vec<u8>) -> LiteralRecord {
    LiteralRecord {
        token: None,
        kind: LiteralKind::Integer,
        radix: 10,
        suffix: LiteralSuffix::None,
        value: magnitude,
        negative: false,
        spelling: vec![],
        candidate_type: Lx08CandidateType::Int,
    }
}

fn operand(id: u32, body: Option<LiteralRecord>) -> BranchOperand {
    BranchOperand {
        id: LiteralId::from_index(id),
        body,
    }
}

fn present(id: u32, magnitude: Vec<u8>) -> BranchOperand {
    operand(id, Some(subset_literal(magnitude)))
}

fn branch_input(
    op: BranchOp,
    cond: BranchOperand,
    lhs: BranchOperand,
    rhs: BranchOperand,
    max_const_bits: u32,
) -> BranchInput {
    BranchInput {
        task: TaskId::from_index(0),
        op,
        state: TaskState::Running,
        cond,
        lhs,
        rhs,
        max_const_bits,
    }
}

/// `&&`/`||` input: `lhs` doubles as the condition.
fn logical_input(op: BranchOp, lhs: Vec<u8>, rhs_body: Option<LiteralRecord>) -> BranchInput {
    let lhs_operand = present(0, lhs);
    branch_input(
        op,
        lhs_operand.clone(),
        lhs_operand,
        operand(1, rhs_body),
        128,
    )
}

fn assert_input(cond: AssertCond) -> AssertInput {
    AssertInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        cond,
        max_const_bits: 128,
    }
}

fn unsupported_out_of_subset() -> LiteralRecord {
    let mut record = subset_literal(vec![3]);
    record.radix = 16;
    record
}

// --- 1. freeze ---------------------------------------------------------------

#[test]
fn branch_kinds_stage_registry_manifest_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(
        TaskKind::CONSTANT_CONST_BRANCH_AND.group(),
        TaskGroup::CONSTANT_LAYOUT_INIT
    );
    assert_eq!(TaskKind::CONSTANT_CONST_BRANCH_AND.local(), 17);
    assert_eq!(TaskKind::CONSTANT_CONST_BRANCH_OR.local(), 18);
    assert_eq!(TaskKind::CONSTANT_CONST_BRANCH_COND.local(), 19);
    assert_eq!(TaskKind::CONSTANT_CONST_STATIC_ASSERT.local(), 20);
    assert_eq!(CL04_AND_TASK_KIND, TaskKind::CONSTANT_CONST_BRANCH_AND);
    assert_eq!(CL04_OR_TASK_KIND, TaskKind::CONSTANT_CONST_BRANCH_OR);
    assert_eq!(CL04_COND_TASK_KIND, TaskKind::CONSTANT_CONST_BRANCH_COND);
    assert_eq!(
        CL07_ASSERT_TASK_KIND,
        TaskKind::CONSTANT_CONST_STATIC_ASSERT
    );
    for kind in [
        TaskKind::CONSTANT_CONST_BRANCH_AND,
        TaskKind::CONSTANT_CONST_BRANCH_OR,
        TaskKind::CONSTANT_CONST_BRANCH_COND,
        TaskKind::CONSTANT_CONST_STATIC_ASSERT,
    ] {
        assert!(is_const_branch_slice_kind(kind));
        assert_eq!(stage_of(kind), Some(2));
    }
    assert!(!is_const_branch_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_const_branch_slice_kind(TaskKind::CONSTANT_CONST_FOLD));
    assert!(!is_const_branch_slice_kind(TaskKind::PARSE_RECOVERY));
    // The branch registry extends the `/36` head linearly (63 → 67);
    // `CONSTANT_LAYOUT_INIT` owners start new codes at local 21.
    assert_eq!(TaskKindRegistry::pa_recovery_slice().len(), 63);
    let registry = TaskKindRegistry::const_branch_slice();
    assert_eq!(registry.len(), 67);
    for (kind, name) in [
        (
            TaskKind::CONSTANT_CONST_BRANCH_AND,
            "constant_layout_init.const_branch_and",
        ),
        (
            TaskKind::CONSTANT_CONST_BRANCH_OR,
            "constant_layout_init.const_branch_or",
        ),
        (
            TaskKind::CONSTANT_CONST_BRANCH_COND,
            "constant_layout_init.const_branch_cond",
        ),
        (
            TaskKind::CONSTANT_CONST_STATIC_ASSERT,
            "constant_layout_init.const_static_assert",
        ),
    ] {
        assert_eq!(registry.lookup(kind).unwrap().name, name);
    }
    assert_eq!(
        stage_of(TaskKind::new(TaskGroup::CONSTANT_LAYOUT_INIT, 21).unwrap()),
        None
    );
    // All four chips are Ack-only: zero writes, no allowlist rows.
    assert_eq!(CL04_AND_CHIP, ChipId(52));
    assert_eq!(CL04_OR_CHIP, ChipId(53));
    assert_eq!(CL04_COND_CHIP, ChipId(54));
    assert_eq!(CL07_ASSERT_CHIP, ChipId(55));
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 32);
    for chip in [
        CL04_AND_CHIP,
        CL04_OR_CHIP,
        CL04_COND_CHIP,
        CL07_ASSERT_CHIP,
    ] {
        assert!(
            !STORE_OWNER_ALLOWLIST
                .iter()
                .any(|&(owner, _, _, _)| owner == chip),
            "Ack-only chip must hold no allowlist row"
        );
    }
    let manifests = [
        BranchAndChip.manifest(),
        BranchOrChip.manifest(),
        BranchCondChip.manifest(),
        StaticAssertChip.manifest(),
    ];
    for manifest in &manifests {
        assert!(manifest.writes.is_empty());
        assert!(manifest.deterministic);
        assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
        assert!(manifest.declares_read(StoreId::Lex, "literals"));
        assert_eq!(manifest.tests, vec!["compiler/tests/c37_const_branch.rs"]);
        let routing = {
            let mut bus = new_bus();
            install_const_branch(&mut bus);
            bus.routing
        };
        assert!(check_stage_layer_agreement(manifest, &routing).is_ok());
    }
    assert_eq!(
        BranchAndChip.manifest().task_kinds,
        vec![CL04_AND_TASK_KIND]
    );
    assert_eq!(BranchOrChip.manifest().task_kinds, vec![CL04_OR_TASK_KIND]);
    assert_eq!(
        BranchCondChip.manifest().task_kinds,
        vec![CL04_COND_TASK_KIND]
    );
    assert_eq!(
        StaticAssertChip.manifest().task_kinds,
        vec![CL07_ASSERT_TASK_KIND]
    );
}

// --- 2. `&&` selected branch -------------------------------------------------

#[test]
fn and_certifies_selected_branch_canonically() {
    use cc_silicon_compiler::chips::eval_branch;
    // `2 && 3` selects rhs and yields canonical `1`.
    let input = logical_input(BranchOp::And, vec![2], Some(subset_literal(vec![3])));
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![1])));
    // `0 && 3` selects nothing and yields canonical `0`.
    let input = logical_input(BranchOp::And, vec![0], Some(subset_literal(vec![3])));
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![0])));
    assert!(is_ack(&BranchAndChip::compute(&input)));
}

// --- 3. `||` selected branch -------------------------------------------------

#[test]
fn or_certifies_selected_branch_canonically() {
    use cc_silicon_compiler::chips::eval_branch;
    // `3 || <bad>` short-circuits to canonical `1`.
    let input = logical_input(BranchOp::Or, vec![3], Some(subset_literal(vec![3])));
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![1])));
    // `0 || 3` evaluates rhs to canonical `1`; `0 || 0` to `0`.
    let input = logical_input(BranchOp::Or, vec![0], Some(subset_literal(vec![3])));
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![1])));
    let input = logical_input(BranchOp::Or, vec![0], Some(subset_literal(vec![0])));
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![0])));
    assert!(is_ack(&BranchOrChip::compute(&input)));
}

// --- 4. `?:` selected branch -------------------------------------------------

#[test]
fn cond_passes_selected_magnitude_through() {
    use cc_silicon_compiler::chips::eval_branch;
    let input = branch_input(
        BranchOp::Cond,
        present(0, vec![2]),
        present(1, vec![2]),
        present(2, vec![3]),
        128,
    );
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![2])));
    let input = branch_input(
        BranchOp::Cond,
        present(0, vec![0]),
        present(1, vec![2]),
        present(2, vec![3]),
        128,
    );
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![3])));
    assert!(is_ack(&BranchCondChip::compute(&input)));
}

// --- 5. short-circuit ignores the unselected operand --------------------------

#[test]
fn short_circuit_never_gates_the_unselected_operand() {
    use cc_silicon_compiler::chips::eval_branch;
    // `0 && <radix-16>`: the unselected rhs is never subset-checked.
    let input = logical_input(BranchOp::And, vec![0], Some(unsupported_out_of_subset()));
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![0])));
    // `0 && <dangling>`: the unselected rhs may even dangle.
    let input = logical_input(BranchOp::And, vec![0], None);
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![0])));
    // `3 || <radix-16>`: the unselected rhs is never gated.
    let input = logical_input(BranchOp::Or, vec![3], Some(unsupported_out_of_subset()));
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![1])));
    // `0 ? <radix-16> : 3`: the unselected `then` is never gated.
    let input = branch_input(
        BranchOp::Cond,
        present(0, vec![0]),
        operand(1, Some(unsupported_out_of_subset())),
        present(2, vec![3]),
        128,
    );
    assert_eq!(eval_branch(&input), Ok(BranchValue(vec![3])));
    assert!(is_ack(&BranchCondChip::compute(&input)));
    // A SELECTED bad operand still fails loudly.
    let input = logical_input(BranchOp::And, vec![2], Some(unsupported_out_of_subset()));
    let error = eval_branch(&input).expect_err("selected bad rhs must fail");
    assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Unsupported, 1));
    let input = logical_input(BranchOp::And, vec![2], None);
    let error = eval_branch(&input).expect_err("dangling selected rhs must fail");
    assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Task, 4));
}

// --- 6. static assert: pass / fail / non-ICE -----------------------------------

#[test]
fn static_assert_pass_fail_and_non_constant() {
    use cc_silicon_compiler::chips::eval_assert;
    let pass = assert_input(AssertCond::Literal(present(0, vec![2])));
    assert_eq!(eval_assert(&pass), Ok(()));
    assert!(is_ack(&StaticAssertChip::compute(&pass)));
    let failed = assert_input(AssertCond::Literal(present(0, vec![0])));
    let error = eval_assert(&failed).expect_err("zero assert must fail");
    assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Task, 3));
    assert!(matches!(
        StaticAssertChip::compute(&failed).as_slice(),
        [Proposal::Fail { .. }]
    ));
    // A non-literal payload is `NotConstantExpression`, never ICE.
    let non_ice = assert_input(AssertCond::NotConstant);
    let error = eval_assert(&non_ice).expect_err("non-ICE assert must fail");
    assert_eq!(error.code, DiagnosticCode::CONST_NOT_CONSTANT_EXPRESSION);
}

// --- 7. subset gate ------------------------------------------------------------

#[test]
fn out_of_subset_selected_operands_are_unsupported() {
    use cc_silicon_compiler::chips::{eval_assert, eval_branch};
    let input = branch_input(
        BranchOp::Cond,
        operand(0, Some(unsupported_out_of_subset())),
        present(1, vec![2]),
        present(2, vec![3]),
        128,
    );
    let error = eval_branch(&input).expect_err("bad cond must fail");
    assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Unsupported, 1));
    let input = logical_input(BranchOp::Or, vec![0], Some(unsupported_out_of_subset()));
    let error = eval_branch(&input).expect_err("selected bad rhs must fail");
    assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Unsupported, 1));
    let input = assert_input(AssertCond::Literal(operand(
        0,
        Some(unsupported_out_of_subset()),
    )));
    let error = eval_assert(&input).expect_err("bad assert literal must fail");
    assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Unsupported, 1));
}

// --- 8. overflow gate ------------------------------------------------------------

#[test]
fn over_budget_magnitudes_are_const_overflow() {
    use cc_silicon_compiler::chips::{eval_assert, eval_branch};
    // Canonical `1` needs one bit; a zero budget cannot hold it.
    let input = BranchInput {
        max_const_bits: 0,
        ..logical_input(BranchOp::And, vec![2], Some(subset_literal(vec![3])))
    };
    let error = eval_branch(&input).expect_err("over-budget result must fail");
    assert_eq!(error.code, DiagnosticCode::CONST_OVERFLOW);
    // Magnitude `2` needs two bits; a one-bit budget cannot hold it.
    let input = BranchInput {
        max_const_bits: 1,
        ..logical_input(BranchOp::And, vec![2], Some(subset_literal(vec![3])))
    };
    let error = eval_branch(&input).expect_err("over-budget cond must fail");
    assert_eq!(error.code, DiagnosticCode::CONST_OVERFLOW);
    let input = AssertInput {
        max_const_bits: 1,
        ..assert_input(AssertCond::Literal(present(0, vec![2])))
    };
    let error = eval_assert(&input).expect_err("over-budget assert must fail");
    assert_eq!(error.code, DiagnosticCode::CONST_OVERFLOW);
}

// --- 9. bus dispatch ------------------------------------------------------------

/// Seed in-subset literals (`2`, `0`, `3`) on a fresh bus.
fn seeded_bus() -> (CompilerBus, BTreeMap<&'static str, LiteralId>) {
    let mut bus = new_bus();
    install_const_branch(&mut bus);
    let limits = bus.limits();
    let mut ids = BTreeMap::new();
    for (name, magnitude) in [("two", vec![2]), ("zero", vec![0]), ("three", vec![3])] {
        let id = bus
            .arenas
            .literals
            .alloc(subset_literal(magnitude), &limits)
            .expect("seed literal");
        ids.insert(name, id);
    }
    (bus, ids)
}

fn literal_refs(ids: &[LiteralId]) -> Payload {
    Payload::from_refs(ids.iter().map(|id| RecordRef::Literal(*id)).collect())
}

fn run_scenario() -> (Vec<u8>, usize, usize) {
    use cc_silicon_compiler::snapshot::Snapshot;
    let (mut bus, ids) = seeded_bus();
    let workers = workers();
    // `2 && 3` → `Ack`; `0 && 3` → `Ack` (short-circuit); `0 ? 2 : 3` →
    // `Ack`; assert `2` → `Ack`.
    bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_BRANCH_AND,
        CL04_AND_CHIP,
        literal_refs(&[ids["two"], ids["three"]]),
    );
    bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_BRANCH_OR,
        CL04_OR_CHIP,
        literal_refs(&[ids["zero"], ids["three"]]),
    );
    bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_BRANCH_COND,
        CL04_COND_CHIP,
        literal_refs(&[ids["zero"], ids["two"], ids["three"]]),
    );
    bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_STATIC_ASSERT,
        CL07_ASSERT_CHIP,
        literal_refs(&[ids["two"]]),
    );
    let mut completed = 0;
    let mut failed = 0;
    for _ in 0..16 {
        let report = tick(&mut bus, &workers);
        match &report.outcome {
            TickOutcome::Executed { commit, .. } => {
                for (_, result) in &commit.completed {
                    match &bus.arenas.results.get(*result).unwrap().value {
                        ResultValue::Ack => {}
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
    (Snapshot::capture(&bus).bytes().to_vec(), completed, failed)
}

#[test]
fn bus_dispatch_commits_ack_and_replays_deterministically() {
    let (first, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (4, 0));
    let (second, completed, failed) = run_scenario();
    assert_eq!((completed, failed), (4, 0));
    assert_eq!(first, second);
}

// --- 10. gates -------------------------------------------------------------------

#[test]
fn branch_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    // A wrong layer is refused on the driver path for all four kinds.
    let cases: [(TaskKind, ChipId, &dyn Worker); 4] = [
        (
            TaskKind::CONSTANT_CONST_BRANCH_AND,
            CL04_AND_CHIP,
            &BranchAndChip,
        ),
        (
            TaskKind::CONSTANT_CONST_BRANCH_OR,
            CL04_OR_CHIP,
            &BranchOrChip,
        ),
        (
            TaskKind::CONSTANT_CONST_BRANCH_COND,
            CL04_COND_CHIP,
            &BranchCondChip,
        ),
        (
            TaskKind::CONSTANT_CONST_STATIC_ASSERT,
            CL07_ASSERT_CHIP,
            &StaticAssertChip,
        ),
    ];
    for (kind, chip, worker) in cases {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::const_branch_slice();
        mismatched.schema = StoreSchema::pa_slice();
        mismatched
            .registrations
            .register(worker.manifest(), &mismatched.schema, &mismatched.kinds)
            .unwrap();
        mismatched.routing.register(kind, chip, 9).unwrap();
        let task = bootstrap(&mut mismatched, kind, chip, Payload::empty());
        let error =
            cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers()).unwrap_err();
        assert!(matches!(
            error,
            cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
        ));
    }
    // The pre-`/37` registry knows none of the four kinds, so all four
    // manifests are rejected there (frozen kinds are never re-registered).
    let mut stale = ManifestRegistry::new();
    let kinds = TaskKindRegistry::pa_recovery_slice();
    let schema = StoreSchema::pa_slice();
    for worker in [
        &BranchAndChip as &dyn Worker,
        &BranchOrChip,
        &BranchCondChip,
        &StaticAssertChip,
    ] {
        assert!(stale.register(worker.manifest(), &schema, &kinds).is_err());
    }
}
