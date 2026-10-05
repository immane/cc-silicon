use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, TaggedProposal};
use cc_silicon_compiler::commit::{check_proposal_budget, commit_proposals, CommitError};
use cc_silicon_compiler::ids::{ChipId, TaskId};
use cc_silicon_compiler::limits::{LimitError, Limits, STAGE_COUNT};
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

#[test]
fn limits_new_fields_carry_their_documented_defaults() {
    let fixture = Limits::fixture();
    assert_eq!(fixture.max_inflight_per_tick, 1);
    assert_eq!(fixture.max_const_bits, 128);
    assert_eq!(fixture.stage_queue_bound, [1 << 12; STAGE_COUNT]);
    assert_eq!(Limits::default(), Limits::fixture());
}

#[test]
fn limits_try_new_round_trips_the_fixture() {
    let fixture = Limits::fixture();
    let rebuilt = Limits::try_new(
        fixture.max_records_per_arena,
        fixture.max_records_total,
        fixture.max_source_bytes,
        fixture.max_intern_entries,
        fixture.max_intern_bytes,
        fixture.max_queue_len,
        fixture.max_tasks_total,
        fixture.max_task_depth,
        fixture.max_diagnostics,
        fixture.max_ticks,
        fixture.max_proposals_per_tick,
        fixture.max_inflight_per_tick,
        fixture.stage_queue_bound,
        fixture.max_const_bits,
        fixture.max_task_progress,
    )
    .unwrap();
    assert_eq!(rebuilt, fixture);
    assert!(rebuilt.validate().is_ok());
}

#[test]
fn limits_try_new_rejects_invalid_new_bounds() {
    let good = Limits::fixture();

    let mut zero_quota = good;
    zero_quota.max_inflight_per_tick = 0;
    assert!(matches!(
        zero_quota.validate(),
        Err(LimitError::InvalidInflightQuota { value: 0 })
    ));

    let mut zero_stage = good;
    zero_stage.stage_queue_bound[3] = 0;
    assert!(matches!(
        zero_stage.validate(),
        Err(LimitError::InvalidStageQueueBound { stage: 3, value: 0 })
    ));

    for bits in [0, 129, u32::MAX] {
        let mut bad_bits = good;
        bad_bits.max_const_bits = bits;
        assert!(
            matches!(
                bad_bits.validate(),
                Err(LimitError::InvalidConstBits { value }) if value == bits
            ),
            "max_const_bits {bits} must be rejected"
        );
    }

    // The constructor itself rejects; backpressure stays a CommitError and
    // never appears here.
    let mut bad = good;
    bad.max_const_bits = 129;
    assert!(matches!(
        Limits::try_new(
            bad.max_records_per_arena,
            bad.max_records_total,
            bad.max_source_bytes,
            bad.max_intern_entries,
            bad.max_intern_bytes,
            bad.max_queue_len,
            bad.max_tasks_total,
            bad.max_task_depth,
            bad.max_diagnostics,
            bad.max_ticks,
            bad.max_proposals_per_tick,
            bad.max_inflight_per_tick,
            bad.stage_queue_bound,
            bad.max_const_bits,
            bad.max_task_progress,
        ),
        Err(LimitError::InvalidConstBits { value: 129 })
    ));
}

#[test]
fn limit_error_codes_are_stable() {
    assert_eq!(
        LimitError::InvalidInflightQuota { value: 0 }.code(),
        "limits.invalid_inflight_quota"
    );
    assert_eq!(
        LimitError::InvalidStageQueueBound { stage: 0, value: 0 }.code(),
        "limits.invalid_stage_queue_bound"
    );
    assert_eq!(
        LimitError::InvalidConstBits { value: 129 }.code(),
        "limits.invalid_const_bits"
    );
    assert_eq!(
        LimitError::Queue {
            limit: 1,
            requested: 2
        }
        .code(),
        "limits.queue"
    );
    assert_eq!(
        LimitError::DanglingParent {
            parent: TaskId::from_index(0)
        }
        .code(),
        "limits.dangling_parent"
    );
}

#[test]
fn span_offsets_are_u64() {
    use cc_silicon_compiler::bus::SpanRecord;
    let mut bus = CompilerBus::default();
    let name = bus.intern_name(b"wide.c").unwrap();
    let source = bus.alloc_source(name, b"x".to_vec()).unwrap();
    // Offsets beyond `u32::MAX` are representable.
    let wide: u64 = u32::MAX as u64 + 5;
    let limits = bus.limits();
    let span = bus
        .arenas
        .spans
        .alloc(
            SpanRecord {
                source,
                start: wide,
                end: wide + 1,
                expansion: None,
            },
            &limits,
        )
        .unwrap();
    let stored = bus.arenas.spans.get(span).unwrap();
    assert_eq!((stored.start, stored.end), (wide, wide + 1));
}

#[test]
fn bus_report_bounded_by_max_ticks_plus_one() {
    use cc_silicon_compiler::bus::{TickMetrics, TickRecord};
    let mut bus = CompilerBus::new(config_with(Limits {
        max_ticks: 2,
        ..Limits::fixture()
    }));
    let record = || TickRecord {
        dispatched: Vec::new(),
        metrics: TickMetrics { dispatched: 0 },
        selected: None,
    };
    assert!(bus.push_tick_record(record()).is_ok());
    assert!(bus.push_tick_record(record()).is_ok());
    assert!(bus.push_tick_record(record()).is_ok());
    assert_eq!(bus.report.len(), 3);
    let error = bus.push_tick_record(record()).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::bus::ReportCapacityError::ReportFull {
            limit: 3,
            requested: 4
        }
    ));
}

#[test]
fn tick_record_carries_minimal_metrics() {
    let mut bus = CompilerBus::default();
    bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP, None))
        .unwrap();
    let shell = RoutingShell::new();
    shell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert_eq!(bus.report.len(), 1);
    // Minimal carrier: dispatched counts only (full `PipelineMetrics` is
    // deferred to Part B).
    assert_eq!(bus.report[0].metrics.dispatched, 1);
    assert_eq!(bus.report[0].dispatched.len(), 1);
}
