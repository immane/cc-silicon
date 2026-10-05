// Observation tests for the working-tree t01-c01-c06/8 review.
// Passing means the reviewed behavior still occurs, not correctness acceptance.
// Compile in an external standalone crate; see the companion review's section 8.
// Direct task/store mutations seed trusted fixtures; worker effects use commit.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use cc_silicon_compiler::bus::{
        CompilerBus, CompilerPins, ConstRecord, LiteralRecord, TaggedProposal,
    };
    use cc_silicon_compiler::chips::{handler_for, FoldChip, Worker, WorkerRegistry};
    use cc_silicon_compiler::commit::{commit_proposals, CommitError};
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
        AppendBatch, ChildRef, DraftRef, Payload, Proposal, ResultValue, TaskDraft, TaskKind,
        TaskKindRegistry, TaskState, WaitSet,
    };

    fn bus(limits: Limits) -> CompilerBus {
        CompilerBus::new(CompilerConfig::new(
            TargetSpec::aarch64_unknown_linux_gnu_unverified(),
            Dialect::C11,
            OptLevel::O0,
            vec![],
            limits,
        ))
    }

    fn install(bus: &mut CompilerBus, layer: u16) {
        bus.kinds = TaskKindRegistry::m1_slice();
        bus.schema = StoreSchema::m1_slice();
        bus.registrations
            .register(FoldChip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
        bus.routing
            .register(TaskKind::CONSTANT_CONST_FOLD, G1_FOLD_CHIP, layer)
            .unwrap();
    }

    fn task(
        bus: &mut CompilerBus,
        kind: TaskKind,
        owner: ChipId,
        parent: Option<TaskId>,
        payload: Payload,
    ) -> TaskId {
        bus.bootstrap_task(TaskDraft {
            kind,
            owner,
            parent,
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

    fn append(task: TaskId, indices: &[u32]) -> Proposal {
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

    fn fold_payload(bus: &mut CompilerBus, node: NodeId) -> Payload {
        let limits = bus.limits();
        let left = bus.arenas.literals.alloc(literal(2), &limits).unwrap();
        let right = bus.arenas.literals.alloc(literal(3), &limits).unwrap();
        Payload::from_refs(vec![
            RecordRef::Node(node),
            RecordRef::Literal(left),
            RecordRef::Literal(right),
        ])
    }

    #[test]
    fn observation_new_limits_collide_in_config_hash_and_snapshot() {
        let baseline = bus(Limits::fixture());
        for field in [
            "max_const_bits",
            "max_task_progress",
            "max_inflight_per_tick",
            "stage_queue_bound",
        ] {
            let mut limits = Limits::fixture();
            match field {
                "max_const_bits" => limits.max_const_bits = 1,
                "max_task_progress" => limits.max_task_progress = 0,
                "max_inflight_per_tick" => limits.max_inflight_per_tick = 2,
                _ => limits.stage_queue_bound[2] = 1,
            }
            let changed = bus(limits);
            assert_eq!(
                config_hash(baseline.config()),
                config_hash(changed.config()),
                "{field}"
            );
            assert_eq!(
                Snapshot::capture(&baseline),
                Snapshot::capture(&changed),
                "{field}"
            );
        }
    }

    #[test]
    fn observation_identical_fold_snapshots_diverge_on_next_tick() {
        let mut low = Limits::fixture();
        low.max_const_bits = 1;
        let mut buses = [bus(Limits::fixture()), bus(low)];
        for bus in &mut buses {
            install(bus, 2);
            let node = bus.arenas.nodes.alloc(&bus.limits()).unwrap();
            let payload = fold_payload(bus, node);
            task(
                bus,
                TaskKind::CONSTANT_CONST_FOLD,
                G1_FOLD_CHIP,
                None,
                payload,
            );
        }
        assert_eq!(Snapshot::capture(&buses[0]), Snapshot::capture(&buses[1]));
        let mut workers = WorkerRegistry::new();
        workers.register(FoldChip).unwrap();
        for bus in &mut buses {
            RoutingShell
                .clock_tick_with(&CompilerPins::default(), bus, handler_for(&workers))
                .unwrap();
        }
        assert_eq!(buses[0].arenas.consts.allocated(), 1);
        assert_eq!(buses[1].arenas.consts.allocated(), 0);
        assert_ne!(Snapshot::capture(&buses[0]), Snapshot::capture(&buses[1]));
    }

    #[test]
    fn observation_append_only_worker_survives_latch_running() {
        let mut bus = bus(Limits::fixture());
        install(&mut bus, 2);
        let id = task(
            &mut bus,
            TaskKind::CONSTANT_CONST_FOLD,
            G1_FOLD_CHIP,
            None,
            Payload::empty(),
        );
        let report = RoutingShell
            .clock_tick_with(&CompilerPins::default(), &mut bus, |id, _| {
                vec![append(id, &[0])]
            })
            .unwrap();
        assert!(matches!(report.outcome, TickOutcome::Executed { .. }));
        assert_eq!(bus.arenas.tasks.get(id).unwrap().state, TaskState::Running);
        assert!(bus.tasks.in_flight.is_empty());
        assert!(bus.tasks.ready.is_empty());
        assert_eq!(bus.arenas.consts.allocated(), 1);
        assert!(matches!(
            RoutingShell
                .clock_tick(&CompilerPins::default(), &mut bus)
                .unwrap()
                .outcome,
            TickOutcome::Idle
        ));
    }

    #[test]
    fn observation_batch_recovery_strands_waiting_parent() {
        let mut bus = bus(Limits::fixture());
        install(&mut bus, 2);
        let parent = task(
            &mut bus,
            TaskKind::CONTROL_NOOP,
            ChipId(0),
            None,
            Payload::empty(),
        );
        let child = task(
            &mut bus,
            TaskKind::CONSTANT_CONST_FOLD,
            G1_FOLD_CHIP,
            Some(parent),
            Payload::empty(),
        );
        bus.arenas.tasks.get_mut(parent).unwrap().state = TaskState::Waiting(WaitSet {
            children: vec![child],
            host_request: None,
        });
        bus.tasks.ready.retain(|id| *id != parent);
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
        assert!(matches!(
            report.outcome,
            TickOutcome::CommitFailed {
                error: CommitError::DuplicateCompletion { .. },
                ..
            }
        ));
        assert!(matches!(
            bus.arenas.tasks.get(child).unwrap().state,
            TaskState::Failed(_)
        ));
        assert!(matches!(
            bus.arenas.tasks.get(parent).unwrap().state,
            TaskState::Waiting(_)
        ));
        assert!(matches!(
            RoutingShell
                .clock_tick(&CompilerPins::default(), &mut bus)
                .unwrap()
                .outcome,
            TickOutcome::Idle
        ));
        assert!(matches!(
            bus.arenas.tasks.get(parent).unwrap().state,
            TaskState::Waiting(_)
        ));
    }

    #[test]
    fn observation_nested_normal_failure_join_is_not_drained() {
        let mut bus = bus(Limits::fixture());
        let grand = task(
            &mut bus,
            TaskKind::CONTROL_NOOP,
            ChipId(0),
            None,
            Payload::empty(),
        );
        let parent = task(
            &mut bus,
            TaskKind::CONTROL_NOOP,
            ChipId(0),
            Some(grand),
            Payload::empty(),
        );
        let leaf = task(
            &mut bus,
            TaskKind::CONTROL_UNSUPPORTED,
            ChipId(0),
            Some(parent),
            Payload::empty(),
        );
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
        assert!(matches!(
            bus.arenas.tasks.get(grand).unwrap().state,
            TaskState::Waiting(_)
        ));
        assert!(matches!(
            RoutingShell
                .clock_tick(&CompilerPins::default(), &mut bus)
                .unwrap()
                .outcome,
            TickOutcome::Idle
        ));
        assert!(matches!(
            bus.arenas.tasks.get(grand).unwrap().state,
            TaskState::Waiting(_)
        ));
    }

    #[test]
    fn observation_own_batch_wait_accepts_nonchild() {
        let mut bus = bus(Limits::fixture());
        install(&mut bus, 2);
        let parent = task(
            &mut bus,
            TaskKind::CONSTANT_CONST_FOLD,
            G1_FOLD_CHIP,
            None,
            Payload::empty(),
        );
        bus.arenas.tasks.get_mut(parent).unwrap().state = TaskState::Running;
        bus.tasks.ready.clear();
        let report = commit_proposals(
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
        .unwrap();
        let child = report.enqueued[0];
        assert_eq!(bus.arenas.tasks.get(child).unwrap().parent, None);
        assert_eq!(
            bus.arenas.tasks.get(parent).unwrap().state,
            TaskState::Waiting(WaitSet {
                children: vec![child],
                host_request: None
            })
        );
    }

    #[test]
    fn observation_append_draft_indices_are_ignored() {
        for indices in [vec![42], vec![0, 0], vec![u32::MAX]] {
            let mut bus = bus(Limits::fixture());
            install(&mut bus, 2);
            let id = task(
                &mut bus,
                TaskKind::CONSTANT_CONST_FOLD,
                G1_FOLD_CHIP,
                None,
                Payload::empty(),
            );
            bus.arenas.tasks.get_mut(id).unwrap().state = TaskState::Running;
            bus.tasks.ready.clear();
            let report = commit_proposals(
                &mut bus,
                vec![
                    tag(id, append(id, &indices)),
                    tag(
                        id,
                        Proposal::Complete {
                            task: id,
                            value: ResultValue::Ack,
                        },
                    ),
                ],
            )
            .unwrap();
            assert_eq!(report.appended.len(), indices.len());
        }
    }

    #[test]
    fn observation_wrong_routed_layer_executes_fold() {
        let mut bus = bus(Limits::fixture());
        install(&mut bus, 9);
        let node = bus.arenas.nodes.alloc(&bus.limits()).unwrap();
        let payload = fold_payload(&mut bus, node);
        let id = task(
            &mut bus,
            TaskKind::CONSTANT_CONST_FOLD,
            G1_FOLD_CHIP,
            None,
            payload,
        );
        let mut workers = WorkerRegistry::new();
        workers.register(FoldChip).unwrap();
        RoutingShell
            .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
            .unwrap();
        assert!(matches!(
            bus.arenas.tasks.get(id).unwrap().state,
            TaskState::Completed(_)
        ));
    }

    #[test]
    fn observation_fold_accepts_nonexistent_node() {
        let mut bus = bus(Limits::fixture());
        install(&mut bus, 2);
        let payload = fold_payload(&mut bus, NodeId::from_index(123));
        let id = task(
            &mut bus,
            TaskKind::CONSTANT_CONST_FOLD,
            G1_FOLD_CHIP,
            None,
            payload,
        );
        let mut workers = WorkerRegistry::new();
        workers.register(FoldChip).unwrap();
        RoutingShell
            .clock_tick_with(&CompilerPins::default(), &mut bus, handler_for(&workers))
            .unwrap();
        assert!(matches!(
            bus.arenas.tasks.get(id).unwrap().state,
            TaskState::Completed(_)
        ));
        assert_eq!(
            bus.arenas
                .consts
                .get(cc_silicon_compiler::ids::ConstId::from_index(0))
                .unwrap()
                .value,
            vec![5]
        );
    }

    #[test]
    fn observation_non_slice_kind_bypasses_const_sole_writer_allowlist() {
        let mut bus = bus(Limits::fixture());
        bus.schema = StoreSchema::m1_slice();
        let other = ChipId(99);
        let mut manifest: ChipManifest = FoldChip.manifest();
        manifest.id = other;
        manifest.chip_name = "SecondConstWriterChip";
        manifest.group = cc_silicon_compiler::task::TaskGroup::CONTROL;
        manifest.task_kinds = vec![TaskKind::CONTROL_START_JOB];
        manifest.reads.clear();
        bus.registrations
            .register(manifest, &bus.schema, &bus.kinds)
            .unwrap();
        let id = task(
            &mut bus,
            TaskKind::CONTROL_START_JOB,
            other,
            None,
            Payload::empty(),
        );
        bus.arenas.tasks.get_mut(id).unwrap().state = TaskState::Running;
        bus.tasks.ready.clear();
        commit_proposals(
            &mut bus,
            vec![
                TaggedProposal {
                    chip: other,
                    task: id,
                    proposal: append(id, &[0]),
                },
                TaggedProposal {
                    chip: other,
                    task: id,
                    proposal: Proposal::Complete {
                        task: id,
                        value: ResultValue::Ack,
                    },
                },
            ],
        )
        .unwrap();
        assert_eq!(bus.arenas.consts.allocated(), 1);
    }

    #[test]
    fn observation_await_all_fails_before_slow_sibling_is_terminal() {
        let mut bus = bus(Limits::fixture());
        let parent = task(
            &mut bus,
            TaskKind::CONTROL_NOOP,
            ChipId(0),
            None,
            Payload::empty(),
        );
        let failed = task(
            &mut bus,
            TaskKind::CONTROL_UNSUPPORTED,
            ChipId(0),
            Some(parent),
            Payload::empty(),
        );
        let slow = task(
            &mut bus,
            TaskKind::CONTROL_NOOP,
            ChipId(0),
            Some(parent),
            Payload::empty(),
        );
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
        assert!(matches!(
            bus.arenas.tasks.get(parent).unwrap().state,
            TaskState::Failed(_)
        ));
    }

    #[test]
    fn observation_append_and_patch_append_same_field_are_accepted() {
        let mut bus = bus(Limits::fixture());
        install(&mut bus, 2);
        let id = task(
            &mut bus,
            TaskKind::CONSTANT_CONST_FOLD,
            G1_FOLD_CHIP,
            None,
            Payload::empty(),
        );
        bus.arenas.tasks.get_mut(id).unwrap().state = TaskState::Running;
        bus.tasks.ready.clear();
        let report = commit_proposals(
            &mut bus,
            vec![
                tag(id, append(id, &[0])),
                tag(
                    id,
                    Proposal::StorePatch(cc_silicon_compiler::task::StorePatch {
                        owner: G1_FOLD_CHIP,
                        task: id,
                        version: 0,
                        store: cc_silicon_compiler::task::StoreId::Constants,
                        field: "records",
                        op: cc_silicon_compiler::task::PatchOp::Append,
                        target: None,
                        value: Some(RecordRef::Const(
                            cc_silicon_compiler::ids::ConstId::from_index(0),
                        )),
                    }),
                ),
                tag(
                    id,
                    Proposal::Complete {
                        task: id,
                        value: ResultValue::Ack,
                    },
                ),
            ],
        )
        .unwrap();
        assert_eq!(report.appended.len(), 1);
        assert_eq!(report.patches, 1);
    }
}
