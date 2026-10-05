use cc_silicon::Bus;
use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, JobState, TaggedProposal};
use cc_silicon_compiler::commit::commit_proposals;
use cc_silicon_compiler::ids::{ChipId, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::routing::{Resolution, RoutingShell, TickOutcome};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    KindStatus, Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup, TaskKind, TaskState,
};

fn draft(kind: TaskKind) -> TaskDraft {
    TaskDraft {
        kind,
        payload: Payload::empty(),
        owner: ChipId(7),
        parent: None,
        continuation: None,
    }
}

fn boot(kind: TaskKind) -> (CompilerBus, TaskId) {
    let mut bus = CompilerBus::default();
    let task = bus.bootstrap_task(draft(kind)).unwrap();
    (bus, task)
}

fn config_with_limits(limits: Limits) -> CompilerConfig {
    CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        Vec::new(),
        limits,
    )
}

/// Register the foundation fixture worker so commit may enqueue work for it.
fn register_fixture_chip(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::{
        BackendClass, Capability, ChipManifest, ChipPhase, FieldPath,
    };
    let manifest = ChipManifest {
        id: ChipId(7),
        chip_name: "FixtureChip",
        group: TaskGroup::CONTROL,
        task_kinds: vec![TaskKind::CONTROL_NOOP, TaskKind::CONTROL_UNSUPPORTED],
        reads: vec![],
        writes: vec![FieldPath::new(StoreId::Diagnostics, "entries")],
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["tests"],
        dependencies: vec![],
    };
    bus.registrations
        .register(manifest, &bus.schema, &bus.kinds)
        .unwrap();
}

#[test]
fn noop_task_terminates_normally() {
    let (mut bus, task) = boot(TaskKind::CONTROL_NOOP);
    let shell = RoutingShell::new();
    let report = shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    match report.outcome {
        TickOutcome::Executed {
            resolution, commit, ..
        } => {
            assert_eq!(resolution, Resolution::Noop);
            assert_eq!(commit.completed.len(), 1);
            assert!(commit.failed.is_empty());
        }
        other => panic!("expected execution, got {other:?}"),
    }
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Completed(_)
    ));
    assert_eq!(bus.control.tick, 1);
}

#[test]
fn unsupported_task_fails_explicitly() {
    let (mut bus, task) = boot(TaskKind::CONTROL_UNSUPPORTED);
    let shell = RoutingShell::new();
    let report = shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert!(matches!(
        report.outcome,
        TickOutcome::Executed {
            resolution: Resolution::Unsupported,
            ..
        }
    ));
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Failed(_)
    ));
    assert_eq!(bus.arenas.diagnostics.live(), 1);
}

#[test]
fn unregistered_task_kind_fails_explicitly() {
    let mut bus = CompilerBus::default();
    let kind = TaskKind::new(TaskGroup::LEX, 16).unwrap();
    bus.kinds
        .register(kind, "lex.scan", TaskGroup::LEX, KindStatus::GroupOwned)
        .unwrap();
    let task = bus.bootstrap_task(draft(kind)).unwrap();
    let shell = RoutingShell::new();
    let report = shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert!(matches!(
        report.outcome,
        TickOutcome::Executed {
            resolution: Resolution::Unregistered,
            ..
        }
    ));
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Failed(_)
    ));
}

#[test]
fn idle_bus_clears_selection_and_terminates() {
    let mut bus = CompilerBus::default();
    let shell = RoutingShell::new();
    let report = shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert_eq!(report.outcome, TickOutcome::Idle);
    assert_eq!(bus.control.selected, None);
    assert_eq!(bus.tasks.active, None);
    assert_eq!(bus.control.tick, 1);
}

#[test]
fn tick_budget_exhaustion_is_reported() {
    let mut bus = CompilerBus::new(config_with_limits(Limits {
        max_ticks: 0,
        ..Limits::fixture()
    }));
    bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
    let shell = RoutingShell::new();
    let report = shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert_eq!(report.outcome, TickOutcome::BudgetExhausted);
    assert!(bus.control.budget_exhausted);
}

#[test]
fn tick_budget_pin_is_consumed() {
    let (mut bus, _) = boot(TaskKind::CONTROL_NOOP);
    let shell = RoutingShell::new();
    let pins = CompilerPins {
        tick_budget_reached: true,
        ..CompilerPins::default()
    };
    let report = shell.clock_tick(&pins, &mut bus).unwrap();
    assert_eq!(report.outcome, TickOutcome::BudgetExhausted);
    assert!(bus.control.budget_exhausted);
}

#[test]
fn cancel_pin_is_consumed() {
    let (mut bus, task) = boot(TaskKind::CONTROL_NOOP);
    let shell = RoutingShell::new();
    let pins = CompilerPins {
        cancel: true,
        ..CompilerPins::default()
    };
    let report = shell.clock_tick(&pins, &mut bus).unwrap();
    assert_eq!(report.outcome, TickOutcome::Cancelled);
    assert!(bus.control.cancel_requested);
    assert_eq!(bus.control.job_state, JobState::Failed);
    // Cancellation does not silently execute the task.
    assert_eq!(bus.get_task(task).unwrap().state, TaskState::Ready);
}

#[test]
fn newly_enqueued_task_runs_next_tick() {
    let (mut bus, parent) = boot(TaskKind::CONTROL_NOOP);
    register_fixture_chip(&mut bus);
    bus.arenas.tasks.get_mut(parent).unwrap().state = TaskState::Running;
    let report = commit_proposals(
        &mut bus,
        vec![TaggedProposal {
            chip: ChipId(7),
            task: parent,
            proposal: Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP)),
        }],
    )
    .unwrap();
    let child = report.enqueued[0];

    let shell = RoutingShell::new();
    assert_eq!(shell.select(&bus), None, "child is not ready this tick");
    bus.advance_tick();
    assert_eq!(shell.select(&bus), Some(child));
}

#[test]
fn commit_failure_does_not_strand_a_running_task() {
    let mut bus = CompilerBus::new(config_with_limits(Limits {
        max_records_total: 1,
        ..Limits::fixture()
    }));
    let task = bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
    let shell = RoutingShell::new();
    let report = shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    match report.outcome {
        TickOutcome::CommitFailed {
            task: failed,
            diagnostic,
            ..
        } => {
            assert_eq!(failed, task);
            assert!(
                diagnostic.is_none(),
                "no capacity for a recovery diagnostic"
            );
        }
        other => panic!("expected commit failure, got {other:?}"),
    }
    let state = &bus.get_task(task).unwrap().state;
    assert!(
        matches!(state, TaskState::Failed(_)),
        "not stranded: {state:?}"
    );
    assert_eq!(bus.control.selected, None);
    assert_eq!(bus.tasks.active, None);
}

#[test]
fn propagate_rejects_prepopulated_malformed_wires_without_stranding() {
    let (mut bus, task) = boot(TaskKind::CONTROL_NOOP);
    // Entry path: a malformed proposal was placed on the wires before the step.
    bus.wires.proposals.push(TaggedProposal {
        chip: ChipId(7),
        task: TaskId::from_index(9999),
        proposal: Proposal::Complete {
            task: TaskId::from_index(9999),
            value: ResultValue::Empty,
        },
    });
    assert!(bus.wires.proposals.iter().any(|p| p.task.index() == 9999));

    let shell = RoutingShell::new();
    let report = shell.propagate(&mut bus).unwrap();
    match report.outcome {
        TickOutcome::CommitFailed { task: failed, .. } => assert_eq!(failed, task),
        other => panic!("expected explicit commit failure, got {other:?}"),
    }
    assert!(
        matches!(bus.get_task(task).unwrap().state, TaskState::Failed(_)),
        "selected task must not be stranded Running"
    );
    // The malformed batch was consumed atomically (no partial writes) and the
    // wires no longer hold the malformed proposal.
    assert!(bus.wires.proposals.is_empty());
    assert!(bus.tasks.ready.is_empty());
    assert_eq!(bus.control.selected, None);
    assert_eq!(bus.tasks.active, None);
    assert_eq!(bus.arenas.results.allocated(), 0);
}
