use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, TaggedProposal};
use cc_silicon_compiler::commit::{check_proposal_budget, commit_proposals, CommitError};
use cc_silicon_compiler::ids::{ChipId, TaskId};
use cc_silicon_compiler::limits::{LimitError, Limits};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskKind, TaskState,
};

fn config_with(limits: Limits) -> CompilerConfig {
    CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        Vec::new(),
        limits,
    )
}

fn draft(kind: TaskKind, parent: Option<TaskId>) -> TaskDraft {
    TaskDraft {
        kind,
        payload: Payload::empty(),
        owner: ChipId(7),
        parent,
        continuation: None,
    }
}

/// Register the foundation fixture worker so commit may enqueue work for it.
fn register_control_chip(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::{
        BackendClass, Capability, ChipManifest, ChipPhase, FieldPath,
    };
    let manifest = ChipManifest {
        id: ChipId(7),
        chip_name: "FixtureChip",
        group: cc_silicon_compiler::task::TaskGroup::CONTROL,
        task_kinds: vec![TaskKind::CONTROL_NOOP],
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
fn max_source_bytes_is_enforced() {
    let mut bus = CompilerBus::new(config_with(Limits {
        max_source_bytes: 4,
        ..Limits::fixture()
    }));
    let name = bus.intern_name(b"a.c").unwrap();
    assert!(matches!(
        bus.alloc_source(name, b"12345".to_vec()),
        Err(LimitError::SourceBytes {
            limit: 4,
            requested: 5
        })
    ));
    assert!(bus.alloc_source(name, b"1234".to_vec()).is_ok());
}

#[test]
fn max_records_total_is_enforced() {
    let mut bus = CompilerBus::new(config_with(Limits {
        max_records_total: 2,
        ..Limits::fixture()
    }));
    let name = bus.intern_name(b"a.c").unwrap();
    assert!(bus.alloc_source(name, b"a".to_vec()).is_ok());
    assert!(bus.alloc_source(name, b"b".to_vec()).is_ok());
    assert!(matches!(
        bus.alloc_source(name, b"c".to_vec()),
        Err(LimitError::TotalRecords {
            limit: 2,
            requested: 3
        })
    ));
}

#[test]
fn max_tasks_total_is_enforced() {
    let mut bus = CompilerBus::new(config_with(Limits {
        max_tasks_total: 1,
        ..Limits::fixture()
    }));
    assert!(bus
        .bootstrap_task(draft(TaskKind::CONTROL_NOOP, None))
        .is_ok());
    assert!(matches!(
        bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP, None)),
        Err(LimitError::TasksTotal {
            limit: 1,
            requested: 2
        })
    ));
}

#[test]
fn max_task_depth_is_enforced() {
    let mut bus = CompilerBus::new(config_with(Limits {
        max_task_depth: 1,
        ..Limits::fixture()
    }));
    let parent = bus
        .bootstrap_task(draft(TaskKind::CONTROL_NOOP, None))
        .unwrap();
    assert!(matches!(
        bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP, Some(parent))),
        Err(LimitError::TaskDepth {
            limit: 1,
            requested: 2
        })
    ));
}

#[test]
fn enqueue_depth_failure_is_atomic() {
    let mut bus = CompilerBus::new(config_with(Limits {
        max_task_depth: 1,
        ..Limits::fixture()
    }));
    register_control_chip(&mut bus);
    let parent = bus
        .bootstrap_task(draft(TaskKind::CONTROL_NOOP, None))
        .unwrap();
    bus.arenas.tasks.get_mut(parent).unwrap().state = TaskState::Running;
    let allocated_before = bus.arenas.tasks.allocated();
    let error = commit_proposals(
        &mut bus,
        vec![TaggedProposal {
            chip: ChipId(7),
            task: parent,
            proposal: Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP, Some(parent))),
        }],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        CommitError::Limit(LimitError::TaskDepth {
            limit: 1,
            requested: 2
        })
    ));
    assert_eq!(
        bus.arenas.tasks.allocated(),
        allocated_before,
        "depth failure must not partially commit"
    );
}

#[test]
fn max_queue_len_is_enforced_on_bootstrap() {
    let mut bus = CompilerBus::new(config_with(Limits {
        max_queue_len: 1,
        ..Limits::fixture()
    }));
    assert!(bus
        .bootstrap_task(draft(TaskKind::CONTROL_NOOP, None))
        .is_ok());
    assert!(matches!(
        bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP, None)),
        Err(LimitError::Queue {
            limit: 1,
            requested: 2
        })
    ));
}

#[test]
fn max_diagnostics_is_enforced() {
    let mut bus = CompilerBus::new(config_with(Limits {
        max_diagnostics: 1,
        ..Limits::fixture()
    }));
    let shell = RoutingShell::new();

    bus.bootstrap_task(draft(TaskKind::CONTROL_UNSUPPORTED, None))
        .unwrap();
    shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert_eq!(bus.arenas.diagnostics.live(), 1);

    bus.bootstrap_task(draft(TaskKind::CONTROL_UNSUPPORTED, None))
        .unwrap();
    let report = shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert!(
        matches!(report.outcome, TickOutcome::CommitFailed { .. }),
        "diagnostic bound must fail the second commit: {:?}",
        report.outcome
    );
    assert_eq!(bus.arenas.diagnostics.live(), 1);
}

#[test]
fn max_proposals_per_tick_is_enforced() {
    let mut bus = CompilerBus::new(config_with(Limits {
        max_proposals_per_tick: 1,
        ..Limits::fixture()
    }));
    let first = bus
        .bootstrap_task(draft(TaskKind::CONTROL_NOOP, None))
        .unwrap();
    let second = bus
        .bootstrap_task(draft(TaskKind::CONTROL_NOOP, None))
        .unwrap();
    for task in [first, second] {
        bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    }
    let complete = |task: TaskId| TaggedProposal {
        chip: ChipId(7),
        task,
        proposal: Proposal::Complete {
            task,
            value: ResultValue::Empty,
        },
    };
    assert!(matches!(
        commit_proposals(&mut bus, vec![complete(first), complete(second)]),
        Err(CommitError::TooManyProposals { limit: 1, count: 2 })
    ));
}

#[test]
fn proposal_budget_is_lossless_at_the_boundary() {
    // A batch larger than u32::MAX must not wrap to a small value and pass.
    let over = u32::MAX as usize + 1;
    assert!(matches!(
        check_proposal_budget(over, 0),
        Err(CommitError::TooManyProposals { limit: 0, count }) if count == over
    ));
    assert!(check_proposal_budget(u32::MAX as usize, u32::MAX).is_ok());
    assert!(matches!(
        check_proposal_budget(u32::MAX as usize + 1, u32::MAX),
        Err(CommitError::TooManyProposals { .. })
    ));
    // Truncation regression: the old `len() as u32` would have been 0 here.
    assert_eq!((u32::MAX as usize + 1) as u32, 0);
}

#[test]
fn task_depth_errors_on_a_dangling_parent() {
    let bus = CompilerBus::default();
    assert!(matches!(
        bus.task_depth(Some(TaskId::from_index(4321))),
        Err(LimitError::DanglingParent { parent }) if parent == TaskId::from_index(4321)
    ));
}

#[test]
fn bootstrap_with_a_dangling_parent_is_rejected() {
    let mut bus = CompilerBus::default();
    assert!(matches!(
        bus.bootstrap_task(draft(
            TaskKind::CONTROL_NOOP,
            Some(TaskId::from_index(9999))
        )),
        Err(LimitError::DanglingParent { .. })
    ));
}
