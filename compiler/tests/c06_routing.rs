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
        vec![
            TaggedProposal {
                chip: ChipId(7),
                task: parent,
                proposal: Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP)),
            },
            TaggedProposal {
                chip: ChipId(7),
                task: parent,
                proposal: Proposal::Complete {
                    task: parent,
                    value: ResultValue::Empty,
                },
            },
        ],
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

#[test]
fn cancel_takes_precedence_over_tick_budget() {
    // Even with the tick budget exhausted, a requested cancel wins
    // pre-selection: no dispatch, no commit.
    let mut bus = CompilerBus::new(config_with_limits(Limits {
        max_ticks: 0,
        ..Limits::fixture()
    }));
    let task = bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
    bus.control.cancel_requested = true;
    let shell = RoutingShell::new();
    let report = shell.propagate(&mut bus).unwrap();
    assert_eq!(report.outcome, TickOutcome::Cancelled);
    assert_eq!(report.selected, None);
    assert_eq!(bus.control.job_state, JobState::Failed);
    assert_eq!(bus.get_task(task).unwrap().state, TaskState::Ready);
    assert!(bus.tasks.in_flight.is_empty());
    // Idempotent: a second cancel tick repeats the outcome.
    let report = shell.propagate(&mut bus).unwrap();
    assert_eq!(report.outcome, TickOutcome::Cancelled);
    assert_eq!(bus.control.job_state, JobState::Failed);
}

#[test]
fn cancel_tick_records_one_terminal_record() {
    let (mut bus, _) = boot(TaskKind::CONTROL_NOOP);
    let shell = RoutingShell::new();
    let pins = CompilerPins {
        cancel: true,
        ..CompilerPins::default()
    };
    let report = shell.clock_tick(&pins, &mut bus).unwrap();
    assert_eq!(report.outcome, TickOutcome::Cancelled);
    assert_eq!(bus.report.len(), 1);
    let record = &bus.report[0];
    assert!(record.dispatched.is_empty());
    assert_eq!(record.selected, None);
    assert_eq!(record.metrics.dispatched, 0);
}

#[test]
fn in_flight_is_empty_at_latch_after_execution() {
    let (mut bus, task) = boot(TaskKind::CONTROL_NOOP);
    let shell = RoutingShell::new();
    let report = shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert!(matches!(report.outcome, TickOutcome::Executed { .. }));
    // No dispatched task remains `Running`; no residual in-flight entry
    // survives at latch or at the next tick start.
    assert!(bus.tasks.in_flight.is_empty());
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Completed(_)
    ));
    assert_eq!(bus.report.len(), 1);
    assert_eq!(bus.report[0].dispatched, vec![task]);
    assert_eq!(bus.report[0].selected, Some(task));
    assert_eq!(bus.report[0].metrics.dispatched, 1);
}

#[test]
fn in_flight_is_empty_after_commit_failure() {
    let mut bus = CompilerBus::new(config_with_limits(Limits {
        max_records_total: 1,
        ..Limits::fixture()
    }));
    let task = bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
    let shell = RoutingShell::new();
    let report = shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert!(matches!(report.outcome, TickOutcome::CommitFailed { .. }));
    assert!(bus.tasks.in_flight.is_empty());
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Failed(_)
    ));
    // The failed tick still records exactly one terminal record.
    assert_eq!(bus.report.len(), 1);
    assert_eq!(bus.report[0].selected, Some(task));
}

#[test]
fn quota_selection_paths() {
    let mut bus = CompilerBus::default();
    let kinds = [
        TaskKind::CONTROL_NOOP,
        TaskKind::CONTROL_UNSUPPORTED,
        TaskKind::new(TaskGroup::LEX, 16).unwrap(),
    ];
    bus.kinds
        .register(kinds[2], "lex.scan", TaskGroup::LEX, KindStatus::GroupOwned)
        .unwrap();
    let mut ids = Vec::new();
    for kind in kinds {
        ids.push(bus.bootstrap_task(draft(kind)).unwrap());
    }
    let shell = RoutingShell::new();
    assert_eq!(RoutingShell::dispatch_quota(&bus), 1);
    // Quota-1 projects onto the frozen head of the order.
    assert_eq!(shell.select(&bus), Some(ids[0]));
    // Wider quotas select deterministically in the same frozen order.
    assert_eq!(shell.select_batch(&bus, 5), ids);
    assert_eq!(shell.select_batch(&bus, 2), ids[..2].to_vec());
    assert!(shell.select_batch(&bus, 0).is_empty());
}
