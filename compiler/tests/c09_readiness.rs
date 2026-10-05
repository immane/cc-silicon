// ============================================================================
// c09_readiness.rs — `/9` pre-chip readiness regression tests (PCR-01..10).
//
// Each test pins a `/9` fix: config/snapshot completeness, per-task
// transition, idle join drain with closure, await-all terminal rule,
// own-batch parent binding, canonical draft indexes, append/patch conflict,
// live binary nodes, stage/layer enforcement, and stateless workers.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, ConstRecord, LiteralRecord, TaggedProposal,
};
use cc_silicon_compiler::chips::{drive_task, handler_for, FoldChip, Worker, WorkerRegistry};
use cc_silicon_compiler::commit::{commit_proposals, poll_await_joins, CommitError};
use cc_silicon_compiler::ids::{ChipId, NodeId, RecordFamily, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{ChipManifest, StoreSchema, G1_FOLD_CHIP};
use cc_silicon_compiler::records::{G1DraftBody, RecordDraft};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{
    config_hash, LiteralKind, LiteralSuffix, Lx08CandidateType, Snapshot,
};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    AppendBatch, ChildRef, DraftRef, Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup,
    TaskKind, TaskKindRegistry, TaskState, WaitSet,
};

fn bus_with(limits: Limits) -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        limits,
    ))
}

fn install_fold(bus: &mut CompilerBus, layer: u16) {
    bus.kinds = TaskKindRegistry::m1_slice();
    bus.schema = StoreSchema::m1_slice();
    bus.registrations
        .register(FoldChip.manifest(), &bus.schema, &bus.kinds)
        .unwrap();
    bus.routing
        .register(TaskKind::CONSTANT_CONST_FOLD, G1_FOLD_CHIP, layer)
        .unwrap();
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

fn tag(task: TaskId, proposal: Proposal) -> TaggedProposal {
    TaggedProposal {
        chip: G1_FOLD_CHIP,
        task,
        proposal,
    }
}

fn const_append(task: TaskId, indices: &[u32]) -> Proposal {
    Proposal::AppendRecords {
        task,
        batch: AppendBatch {
            records: indices
                .iter()
                .map(|i| RecordDraft {
                    family: RecordFamily::Const,
                    index: DraftRef(*i),
                })
                .collect(),
            bodies: indices
                .iter()
                .map(|_| {
                    G1DraftBody::Const(ConstRecord {
                        value: vec![5],
                        negative: false,
                    })
                })
                .collect(),
        },
    }
}

fn literal(value: u8) -> LiteralRecord {
    LiteralRecord {
        token: None,
        kind: LiteralKind::Integer,
        radix: 10,
        suffix: LiteralSuffix::None,
        value: vec![value],
        negative: false,
        spelling: value.to_string().into_bytes(),
        candidate_type: Lx08CandidateType::Int,
    }
}

#[test]
fn pcr01_new_limits_change_config_hash_and_snapshot() {
    let baseline = bus_with(Limits::fixture());
    let cases: Vec<(&str, Limits)> = vec![
        (
            "max_const_bits",
            Limits {
                max_const_bits: 1,
                ..Limits::fixture()
            },
        ),
        (
            "max_task_progress",
            Limits {
                max_task_progress: 0,
                ..Limits::fixture()
            },
        ),
        (
            "max_inflight_per_tick",
            Limits {
                max_inflight_per_tick: 2,
                ..Limits::fixture()
            },
        ),
        (
            "stage_queue_bound",
            Limits {
                stage_queue_bound: [1; 11],
                ..Limits::fixture()
            },
        ),
    ];
    for (name, limits) in cases {
        let changed = bus_with(limits);
        assert_ne!(
            config_hash(baseline.config()),
            config_hash(changed.config()),
            "{name}"
        );
        assert_ne!(
            Snapshot::capture(&baseline),
            Snapshot::capture(&changed),
            "{name}"
        );
    }
}

#[test]
fn pcr02_append_only_batch_is_rejected_before_mutation() {
    let mut bus = bus_with(Limits::fixture());
    install_fold(&mut bus, 2);
    let task = bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        Payload::empty(),
    );
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    bus.tasks.ready.clear();
    let before = bus.arenas.consts.allocated();
    let error = commit_proposals(&mut bus, vec![tag(task, const_append(task, &[0]))]).unwrap_err();
    assert!(matches!(error, CommitError::TaskNotTransitioned { .. }));
    assert_eq!(bus.arenas.consts.allocated(), before);
    assert_eq!(
        bus.arenas.tasks.get(task).unwrap().state,
        TaskState::Running
    );
}

#[test]
fn pcr02_append_only_worker_fails_instead_of_stranding() {
    let mut bus = bus_with(Limits::fixture());
    install_fold(&mut bus, 2);
    bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        Payload::empty(),
    );
    let report = RoutingShell
        .clock_tick_with(&CompilerPins::default(), &mut bus, |id, _| {
            vec![const_append(id, &[0])]
        })
        .unwrap();
    match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 1);
            assert_eq!(commit.completed.len(), 0);
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // Recovery fails the task; no `Running` remains at latch.
    assert!(bus
        .arenas
        .tasks
        .iter()
        .all(|(_, record)| record.state != TaskState::Running));
    assert!(bus.tasks.in_flight.is_empty());
}

#[test]
fn pcr03_idle_tick_drains_recovery_stranded_parent() {
    let mut bus = bus_with(Limits::fixture());
    install_fold(&mut bus, 2);
    let parent = bootstrap(
        &mut bus,
        TaskKind::CONTROL_NOOP,
        ChipId(0),
        Payload::empty(),
    );
    let child = bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        Payload::empty(),
    );
    bus.arenas.tasks.get_mut(child).unwrap().parent = Some(parent);
    bus.arenas.tasks.get_mut(parent).unwrap().state = TaskState::Waiting(WaitSet {
        children: vec![child],
        host_request: None,
    });
    bus.tasks.ready.retain(|id| *id != parent);
    // Duplicate completion fails the batch; recovery fails the child.
    let report = RoutingShell
        .clock_tick_with(&CompilerPins::default(), &mut bus, |id, _| {
            vec![
                Proposal::Complete {
                    task: id,
                    value: ResultValue::Ack,
                },
                Proposal::Complete {
                    task: id,
                    value: ResultValue::Ack,
                },
            ]
        })
        .unwrap();
    assert!(matches!(report.outcome, TickOutcome::CommitFailed { .. }));
    assert!(matches!(
        bus.arenas.tasks.get(child).unwrap().state,
        TaskState::Failed(_)
    ));
    // The next idle tick must drain the parent join (closure), not strand it.
    let report = RoutingShell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert!(matches!(report.outcome, TickOutcome::Joined { .. }));
    assert!(matches!(
        bus.arenas.tasks.get(parent).unwrap().state,
        TaskState::Failed(_)
    ));
}

#[test]
fn pcr03_nested_failure_drains_in_one_idle_poll() {
    let mut bus = bus_with(Limits::fixture());
    let grand = bootstrap(
        &mut bus,
        TaskKind::CONTROL_NOOP,
        ChipId(0),
        Payload::empty(),
    );
    let parent = bootstrap(
        &mut bus,
        TaskKind::CONTROL_NOOP,
        ChipId(0),
        Payload::empty(),
    );
    let leaf = bootstrap(
        &mut bus,
        TaskKind::CONTROL_UNSUPPORTED,
        ChipId(0),
        Payload::empty(),
    );
    bus.arenas.tasks.get_mut(parent).unwrap().parent = Some(grand);
    bus.arenas.tasks.get_mut(leaf).unwrap().parent = Some(parent);
    for (id, child) in [(grand, parent), (parent, leaf)] {
        bus.arenas.tasks.get_mut(id).unwrap().state = TaskState::Waiting(WaitSet {
            children: vec![child],
            host_request: None,
        });
        bus.tasks.ready.retain(|entry| *entry != id);
    }
    RoutingShell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert!(matches!(
        bus.arenas.tasks.get(parent).unwrap().state,
        TaskState::Failed(_)
    ));
    // Poll drains the remaining grandparent level with closure.
    let poll = poll_await_joins(&mut bus).unwrap();
    assert!(poll.failed.iter().any(|(id, _)| *id == grand));
    assert!(matches!(
        bus.arenas.tasks.get(grand).unwrap().state,
        TaskState::Failed(_)
    ));
}

#[test]
fn pcr04_await_all_waits_for_slow_sibling() {
    let mut bus = bus_with(Limits::fixture());
    let parent = bootstrap(
        &mut bus,
        TaskKind::CONTROL_NOOP,
        ChipId(0),
        Payload::empty(),
    );
    let failed = bootstrap(
        &mut bus,
        TaskKind::CONTROL_UNSUPPORTED,
        ChipId(0),
        Payload::empty(),
    );
    let slow = bootstrap(
        &mut bus,
        TaskKind::CONTROL_NOOP,
        ChipId(0),
        Payload::empty(),
    );
    for child in [failed, slow] {
        bus.arenas.tasks.get_mut(child).unwrap().parent = Some(parent);
    }
    bus.arenas.tasks.get_mut(parent).unwrap().state = TaskState::Waiting(WaitSet {
        children: vec![failed, slow],
        host_request: None,
    });
    bus.tasks.ready.retain(|id| *id != parent);
    bus.arenas.tasks.get_mut(slow).unwrap().ready_tick = 100;
    RoutingShell
        .clock_tick(&CompilerPins::default(), &mut bus)
        .unwrap();
    assert!(matches!(
        bus.arenas.tasks.get(failed).unwrap().state,
        TaskState::Failed(_)
    ));
    assert_eq!(bus.arenas.tasks.get(slow).unwrap().state, TaskState::Ready);
    // Await-all: the parent stays waiting while a sibling is non-terminal.
    assert!(matches!(
        bus.arenas.tasks.get(parent).unwrap().state,
        TaskState::Waiting(_)
    ));
}

#[test]
fn pcr05_own_batch_requires_parent_link() {
    let mut bus = bus_with(Limits::fixture());
    install_fold(&mut bus, 2);
    let parent = bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        Payload::empty(),
    );
    bus.arenas.tasks.get_mut(parent).unwrap().state = TaskState::Running;
    bus.tasks.ready.clear();
    let error = commit_proposals(
        &mut bus,
        vec![
            tag(
                parent,
                Proposal::Enqueue(TaskDraft {
                    kind: TaskKind::CONSTANT_CONST_FOLD,
                    owner: G1_FOLD_CHIP,
                    parent: None,
                    payload: Payload::empty(),
                    continuation: None,
                }),
            ),
            tag(
                parent,
                Proposal::AwaitChildren {
                    task: parent,
                    children: vec![ChildRef::OwnBatch(0)],
                },
            ),
        ],
    )
    .unwrap_err();
    assert!(matches!(error, CommitError::AwaitChildrenRefInvalid { .. }));
}

#[test]
fn pcr06_draft_index_must_equal_position() {
    for indices in [vec![42], vec![0, 0], vec![u32::MAX]] {
        let mut bus = bus_with(Limits::fixture());
        install_fold(&mut bus, 2);
        let task = bootstrap(
            &mut bus,
            TaskKind::CONSTANT_CONST_FOLD,
            G1_FOLD_CHIP,
            Payload::empty(),
        );
        bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
        bus.tasks.ready.clear();
        let error = commit_proposals(
            &mut bus,
            vec![
                tag(task, const_append(task, &indices)),
                tag(
                    task,
                    Proposal::Complete {
                        task,
                        value: ResultValue::Ack,
                    },
                ),
            ],
        )
        .unwrap_err();
        assert!(
            matches!(error, CommitError::InvalidPatchShape { .. }),
            "indices {indices:?} gave {error:?}"
        );
    }
}

#[test]
fn pcr07_append_and_patch_conflict_is_rejected() {
    let mut bus = bus_with(Limits::fixture());
    install_fold(&mut bus, 2);
    let task = bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        Payload::empty(),
    );
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    bus.tasks.ready.clear();
    let error = commit_proposals(
        &mut bus,
        vec![
            tag(task, const_append(task, &[0])),
            tag(
                task,
                Proposal::StorePatch(cc_silicon_compiler::task::StorePatch {
                    owner: G1_FOLD_CHIP,
                    task,
                    version: 0,
                    store: StoreId::Constants,
                    field: "records",
                    op: cc_silicon_compiler::task::PatchOp::Append,
                    target: None,
                    value: Some(RecordRef::Const(
                        cc_silicon_compiler::ids::ConstId::from_index(0),
                    )),
                }),
            ),
            tag(
                task,
                Proposal::Complete {
                    task,
                    value: ResultValue::Ack,
                },
            ),
        ],
    )
    .unwrap_err();
    assert!(matches!(error, CommitError::InvalidPatchShape { .. }));
}

#[test]
fn pcr08_binary_fold_rejects_dangling_node() {
    let mut bus = bus_with(Limits::fixture());
    install_fold(&mut bus, 2);
    let limits = bus.limits();
    let left = bus.arenas.literals.alloc(literal(2), &limits).unwrap();
    let right = bus.arenas.literals.alloc(literal(3), &limits).unwrap();
    let payload = Payload::from_refs(vec![
        RecordRef::Node(NodeId::from_index(123)),
        RecordRef::Literal(left),
        RecordRef::Literal(right),
    ]);
    bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        payload,
    );
    let mut workers = WorkerRegistry::new();
    workers.register(FoldChip).unwrap();
    let report = RoutingShell
        .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
        .unwrap();
    match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 0);
            assert_eq!(commit.failed.len(), 1);
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(bus.arenas.consts.allocated(), 0);
}

#[test]
fn pcr09_wrong_layer_is_rejected_on_the_driver_path() {
    let mut bus = bus_with(Limits::fixture());
    install_fold(&mut bus, 9);
    let limits = bus.limits();
    let node = bus.arenas.nodes.alloc(&limits).unwrap();
    let left = bus.arenas.literals.alloc(literal(2), &limits).unwrap();
    let right = bus.arenas.literals.alloc(literal(3), &limits).unwrap();
    let payload = Payload::from_refs(vec![
        RecordRef::Node(node),
        RecordRef::Literal(left),
        RecordRef::Literal(right),
    ]);
    let task = bootstrap(
        &mut bus,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        payload,
    );
    let mut workers = WorkerRegistry::new();
    workers.register(FoldChip).unwrap();
    // Direct driver refuses the mismatched topology.
    let error = drive_task(&bus, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // The tick handler turns the same mismatch into a loud `Fail`, never a
    // folded constant.
    let report = RoutingShell
        .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
        .unwrap();
    match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 0);
            assert_eq!(bus.arenas.consts.allocated(), 0);
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

#[test]
fn pcr10_worker_registry_rejects_stateful_workers() {
    use cc_silicon_compiler::chips::DriveError;
    use cc_silicon_compiler::manifest::{BackendClass, Capability, ChipPhase};

    #[allow(dead_code)]
    struct StatefulWorker(u32);
    impl Worker for StatefulWorker {
        fn manifest(&self) -> ChipManifest {
            ChipManifest {
                id: ChipId(90),
                chip_name: "StatefulWorker",
                group: TaskGroup::CONTROL,
                task_kinds: vec![TaskKind::CONTROL_NOOP],
                reads: vec![],
                writes: vec![],
                capability: Capability::Emulable,
                backend_class: BackendClass::CpuReference,
                phase: ChipPhase::Propagation,
                deterministic: true,
                tests: vec!["compiler/tests/c09_readiness.rs"],
                dependencies: vec![],
            }
        }
        fn handle(
            &self,
            task: TaskId,
            _bus: &cc_silicon_compiler::bus::CompilerBus,
        ) -> Vec<Proposal> {
            vec![Proposal::Complete {
                task,
                value: ResultValue::Empty,
            }]
        }
    }

    let mut registry = WorkerRegistry::new();
    let error = registry.register(StatefulWorker(1)).unwrap_err();
    assert!(matches!(error, DriveError::NonStatelessWorker { .. }));
    assert_eq!(std::mem::size_of::<FoldChip>(), 0);
}

#[test]
fn pcr10_fold_manifest_declares_its_mechanical_reads() {
    let manifest = FoldChip.manifest();
    for (store, field) in [
        (StoreId::Tasks, "active.kind"),
        (StoreId::Lex, "literals"),
        (StoreId::Constants, "records"),
        (StoreId::Config, "limits"),
    ] {
        assert!(
            manifest.declares_read(store, field),
            "missing read {}/{field}",
            store.name()
        );
    }
    assert!(manifest.declares_write(StoreId::Constants, "records"));
}

#[test]
fn pcr13_small_profile_completes_with_bounded_ticks() {
    // `/9` PCR-13 bounded acceptance: the reference scheduler scans the
    // ready set and the waiter set each tick. This is accepted for the small
    // G1 profile only; large waves need an explicit waiter index/benchmark.
    let mut bus = bus_with(Limits::fixture());
    for _ in 0..32 {
        bootstrap(
            &mut bus,
            TaskKind::CONTROL_NOOP,
            ChipId(0),
            Payload::empty(),
        );
    }
    let mut ticks = 0u32;
    while !bus.tasks.ready.is_empty() && ticks < 64 {
        RoutingShell
            .clock_tick(&CompilerPins::default(), &mut bus)
            .unwrap();
        ticks += 1;
    }
    assert_eq!(bus.tasks.ready.len(), 0);
    assert!(ticks <= 64);
}
