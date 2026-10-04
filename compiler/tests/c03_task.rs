use cc_silicon_compiler::bus::{CompilerBus, TaggedProposal};
use cc_silicon_compiler::commit::{commit_proposals, consume_result, CommitError};
use cc_silicon_compiler::ids::{ChipId, DiagnosticId, RecordRef, TaskId};
use cc_silicon_compiler::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use cc_silicon_compiler::task::{
    HostRequestDraft, HostRequestKind, KindStatus, PatchOp, Payload, Proposal, RegistryError,
    ResultValue, StoreId, StorePatch, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry, TaskState,
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

fn manifest_with(writes: Vec<FieldPath>) -> ChipManifest {
    ChipManifest {
        id: ChipId(7),
        chip_name: "FixtureChip",
        group: TaskGroup::CONTROL,
        task_kinds: vec![TaskKind::CONTROL_NOOP, TaskKind::CONTROL_UNSUPPORTED],
        reads: vec![],
        writes,
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["tests/chips/control/fixture.rs"],
        dependencies: vec![],
    }
}

fn bus_with_manifest() -> CompilerBus {
    let mut bus = CompilerBus::default();
    let manifest = manifest_with(vec![FieldPath::new(StoreId::Diagnostics, "entries")]);
    bus.registrations
        .register(manifest, &bus.schema, &bus.kinds)
        .unwrap();
    bus
}

fn running_task(bus: &mut CompilerBus, kind: TaskKind) -> TaskId {
    let id = bus.bootstrap_task(draft(kind)).unwrap();
    bus.arenas.tasks.get_mut(id).unwrap().state = TaskState::Running;
    id
}

fn tag(task: TaskId, proposal: Proposal) -> TaggedProposal {
    TaggedProposal {
        chip: ChipId(7),
        task,
        proposal,
    }
}

#[test]
fn kind_encoding_is_stable() {
    assert_eq!(TaskKind::CONTROL_NOOP.group(), TaskGroup::CONTROL);
    assert_eq!(TaskKind::CONTROL_NOOP.local(), 0);
    assert!(TaskKind::CONTROL_NOOP.is_foundation());
    let lex = TaskKind::new(TaskGroup::LEX, 16).unwrap();
    assert_eq!(lex.group(), TaskGroup::LEX);
    assert_eq!(lex.local(), 16);
    assert!(!lex.is_reserved_local());
    let reserved = TaskKind::new(TaskGroup::LEX, 15).unwrap();
    assert!(reserved.is_reserved_local());
    assert!(TaskKind::new(TaskGroup::LEX, 4096).is_none());
}

#[test]
fn registry_rejects_duplicate_names_kinds_and_reserved_local_codes() {
    let mut registry = TaskKindRegistry::foundation();
    let lex = TaskKind::new(TaskGroup::LEX, 16).unwrap();
    assert!(registry
        .register(lex, "control.noop", TaskGroup::LEX, KindStatus::GroupOwned)
        .is_err_and(|error| matches!(error, RegistryError::DuplicateName { .. })));
    let mut registry = TaskKindRegistry::foundation();
    assert!(registry
        .register(lex, "lex.scan", TaskGroup::LEX, KindStatus::GroupOwned)
        .is_ok());
    assert!(registry
        .register(lex, "lex.scan2", TaskGroup::LEX, KindStatus::GroupOwned)
        .is_err_and(|error| matches!(error, RegistryError::DuplicateKind { .. })));
    let mut registry = TaskKindRegistry::foundation();
    assert!(registry
        .register(lex, "lex.scan", TaskGroup::PARSE, KindStatus::GroupOwned)
        .is_err_and(|error| matches!(error, RegistryError::GroupMismatch { .. })));
    let reserved = TaskKind::new(TaskGroup::LEX, 15).unwrap();
    let mut registry = TaskKindRegistry::foundation();
    assert!(registry
        .register(
            reserved,
            "lex.reserved",
            TaskGroup::LEX,
            KindStatus::GroupOwned
        )
        .is_err_and(|error| matches!(error, RegistryError::ReservedLocalCode { .. })));
}

#[test]
fn completion_is_exactly_once() {
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let report = commit_proposals(
        &mut bus,
        vec![tag(
            task,
            Proposal::Complete {
                task,
                value: ResultValue::Empty,
            },
        )],
    )
    .unwrap();
    assert_eq!(report.completed.len(), 1);
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Completed(_)
    ));

    let second = commit_proposals(
        &mut bus,
        vec![tag(
            task,
            Proposal::Complete {
                task,
                value: ResultValue::Empty,
            },
        )],
    );
    assert!(matches!(
        second,
        Err(CommitError::DuplicateCompletion { .. })
    ));
}

#[test]
fn results_are_consumed_once() {
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let report = commit_proposals(
        &mut bus,
        vec![tag(
            task,
            Proposal::Complete {
                task,
                value: ResultValue::Empty,
            },
        )],
    )
    .unwrap();
    let result = report.completed[0].1;
    assert_eq!(
        consume_result(&mut bus, result).unwrap(),
        ResultValue::Empty
    );
    assert!(matches!(
        consume_result(&mut bus, result),
        Err(CommitError::ResultAlreadyConsumed { result: again }) if again == result
    ));
}

#[test]
fn enqueued_tasks_are_ready_next_tick() {
    let mut bus = bus_with_manifest();
    let parent = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let report = commit_proposals(
        &mut bus,
        vec![tag(
            parent,
            Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP)),
        )],
    )
    .unwrap();
    let child = report.enqueued[0];
    let record = bus.get_task(child).unwrap();
    assert_eq!(record.ready_tick, 1);
    assert_eq!(record.state, TaskState::Ready);
}

#[allow(clippy::too_many_arguments)]
fn patch(
    task: TaskId,
    owner: ChipId,
    version: u64,
    store: StoreId,
    field: &'static str,
    op: PatchOp,
    target: Option<RecordRef>,
    value: Option<RecordRef>,
) -> Proposal {
    Proposal::StorePatch(StorePatch {
        owner,
        task,
        version,
        store,
        field,
        op,
        target,
        value,
    })
}

fn diag_ref() -> RecordRef {
    RecordRef::Diagnostic(DiagnosticId::from_index(0))
}

fn append(task: TaskId, owner: ChipId, version: u64) -> Proposal {
    patch(
        task,
        owner,
        version,
        StoreId::Diagnostics,
        "entries",
        PatchOp::Append,
        None,
        Some(diag_ref()),
    )
}

#[test]
fn valid_append_patch_is_committed_and_bumps_version() {
    let mut bus = bus_with_manifest();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let report = commit_proposals(&mut bus, vec![tag(task, append(task, ChipId(7), 0))]).unwrap();
    assert_eq!(report.patches, 1);
    assert_eq!(bus.patch_log.len(), 1);
    assert_eq!(bus.store_versions.get(StoreId::Diagnostics), 1);
}

#[test]
fn patch_owner_chip_attribution_and_field_are_checked() {
    let mut bus = bus_with_manifest();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);

    // Patch owner differs from the producing chip.
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, append(task, ChipId(99), 0))]),
        Err(CommitError::PatchOwnerMismatch { .. })
    ));

    // Producing chip is not the task owner.
    let mut mismatched = tag(task, append(task, ChipId(7), 0));
    mismatched.chip = ChipId(99);
    assert!(matches!(
        commit_proposals(&mut bus, vec![mismatched]),
        Err(CommitError::ChipNotOwner { .. })
    ));

    // A field absent from the chip's write manifest.
    let undeclared = patch(
        task,
        ChipId(7),
        0,
        StoreId::Sources,
        "bytes",
        PatchOp::Append,
        None,
        Some(diag_ref()),
    );
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, undeclared)]),
        Err(CommitError::WriteNotDeclared { .. })
    ));

    // Stale version.
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, append(task, ChipId(7), 1))]),
        Err(CommitError::StaleVersion { .. })
    ));

    // Invalid shape.
    let bad_shape = patch(
        task,
        ChipId(7),
        0,
        StoreId::Diagnostics,
        "entries",
        PatchOp::Append,
        Some(diag_ref()),
        Some(diag_ref()),
    );
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, bad_shape)]),
        Err(CommitError::InvalidPatchShape { .. })
    ));
}

#[test]
fn patch_task_must_match_enclosing_task() {
    let mut bus = bus_with_manifest();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let other = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let mut proposal = append(task, ChipId(7), 0);
    if let Proposal::StorePatch(record) = &mut proposal {
        record.task = other;
    }
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, proposal)]),
        Err(CommitError::TaskAttributionMismatch { .. })
    ));
}

#[test]
fn config_store_is_read_only() {
    let mut bus = bus_with_manifest();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let config_patch = patch(
        task,
        ChipId(7),
        0,
        StoreId::Config,
        "target",
        PatchOp::Append,
        None,
        Some(diag_ref()),
    );
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, config_patch)]),
        Err(CommitError::ReadOnlyStore { .. })
    ));
}

#[test]
fn manifest_with_an_undeclared_field_is_rejected_at_registration() {
    // Registration now validates against the frozen schema, so a manifest that
    // names a field the schema does not declare never enters the registry. The
    // commit-time `UndeclaredStoreField` branch remains as defense in depth.
    let mut bus = CompilerBus::default();
    let manifest = manifest_with(vec![FieldPath::new(StoreId::Parse, "ghost")]);
    assert!(bus
        .registrations
        .register(manifest, &bus.schema, &bus.kinds)
        .is_err());
    assert!(bus.registrations.is_empty());
}

#[test]
fn task_kind_must_be_accepted_by_the_manifest() {
    let mut bus = CompilerBus::default();
    let mut manifest = manifest_with(vec![FieldPath::new(StoreId::Diagnostics, "entries")]);
    manifest.task_kinds = vec![TaskKind::CONTROL_UNSUPPORTED];
    bus.registrations
        .register(manifest, &bus.schema, &bus.kinds)
        .unwrap();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, append(task, ChipId(7), 0))]),
        Err(CommitError::TaskKindNotAccepted { .. })
    ));
}

#[test]
fn unregistered_chip_store_patch_is_rejected() {
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, append(task, ChipId(7), 0))]),
        Err(CommitError::UnregisteredChip { .. })
    ));
}

#[test]
fn failed_batch_commits_nothing() {
    let mut bus = bus_with_manifest();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let good = append(task, ChipId(7), 0);
    let bad = append(task, ChipId(7), 9);
    let error = commit_proposals(&mut bus, vec![tag(task, good), tag(task, bad)]).unwrap_err();
    assert!(matches!(error, CommitError::StaleVersion { .. }));
    assert!(bus.patch_log.is_empty(), "failure atomicity");
    assert_eq!(bus.store_versions.get(StoreId::Diagnostics), 0);
}

#[test]
fn complete_fail_and_await_host_bind_their_inner_task() {
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let other = TaskId::from_index(9999);

    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(
                task,
                Proposal::Complete {
                    task: other,
                    value: ResultValue::Empty,
                },
            )],
        ),
        Err(CommitError::InnerTaskMismatch { .. })
    ));
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(
                task,
                Proposal::Fail {
                    task: other,
                    diagnostic: cc_silicon_compiler::diagnostic::DiagnosticDraft::unsupported("x"),
                },
            )],
        ),
        Err(CommitError::InnerTaskMismatch { .. })
    ));
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(
                task,
                Proposal::AwaitHost {
                    task: other,
                    request: HostRequestDraft {
                        kind: HostRequestKind::ReadSource,
                        payload: Payload::empty(),
                    },
                },
            )],
        ),
        Err(CommitError::InnerTaskMismatch { .. })
    ));
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Running
    ));
}

#[test]
fn await_host_shares_the_single_transition_guard() {
    let host = |task: TaskId| Proposal::AwaitHost {
        task,
        request: HostRequestDraft {
            kind: HostRequestKind::ReadSource,
            payload: Payload::empty(),
        },
    };
    let complete = |task: TaskId| Proposal::Complete {
        task,
        value: ResultValue::Empty,
    };
    let fail = |task: TaskId| Proposal::Fail {
        task,
        diagnostic: cc_silicon_compiler::diagnostic::DiagnosticDraft::unsupported("x"),
    };

    // duplicate AwaitHost
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let before = (
        bus.arenas.host_requests.allocated(),
        bus.arenas.results.allocated(),
    );
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, host(task)), tag(task, host(task))]),
        Err(CommitError::DuplicateCompletion { .. })
    ));
    assert_eq!(
        (
            bus.arenas.host_requests.allocated(),
            bus.arenas.results.allocated()
        ),
        before
    );
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Running
    ));

    // AwaitHost then Complete
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(task, host(task)), tag(task, complete(task))]
        ),
        Err(CommitError::DuplicateCompletion { .. })
    ));
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Running
    ));

    // Complete then AwaitHost
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(task, complete(task)), tag(task, host(task))]
        ),
        Err(CommitError::DuplicateCompletion { .. })
    ));
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Running
    ));

    // Fail then AwaitHost
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    assert!(matches!(
        commit_proposals(&mut bus, vec![tag(task, fail(task)), tag(task, host(task))]),
        Err(CommitError::DuplicateCompletion { .. })
    ));
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Running
    ));
}

#[test]
fn batch_with_a_missing_task_fails_before_any_mutation() {
    let mut bus = CompilerBus::default();
    let producer = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let missing = TaskId::from_index(4242);
    let allocated_before = bus.arenas.tasks.allocated();
    let ready_before = bus.tasks.ready.len();

    let error = commit_proposals(
        &mut bus,
        vec![
            tag(producer, Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP))),
            tag(
                missing,
                Proposal::Complete {
                    task: missing,
                    value: ResultValue::Empty,
                },
            ),
        ],
    )
    .unwrap_err();
    assert!(matches!(error, CommitError::UnknownTask { .. }));
    assert_eq!(bus.arenas.tasks.allocated(), allocated_before);
    assert_eq!(bus.tasks.ready.len(), ready_before);
}

#[test]
fn completion_requires_a_running_task() {
    let mut bus = CompilerBus::default();
    let task = bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
    // Still `Ready`.
    assert!(matches!(
        commit_proposals(
            &mut bus,
            vec![tag(
                task,
                Proposal::Complete {
                    task,
                    value: ResultValue::Empty,
                },
            )],
        ),
        Err(CommitError::TaskNotRunning { .. })
    ));
}

#[test]
fn dangling_enqueue_parent_is_rejected() {
    let mut bus = bus_with_manifest();
    let producer = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let allocated_before = bus.arenas.tasks.allocated();
    let mut draft = draft(TaskKind::CONTROL_NOOP);
    draft.parent = Some(TaskId::from_index(9876));
    let error =
        commit_proposals(&mut bus, vec![tag(producer, Proposal::Enqueue(draft))]).unwrap_err();
    assert!(matches!(
        error,
        CommitError::DanglingParent { parent } if parent == TaskId::from_index(9876)
    ));
    assert_eq!(bus.arenas.tasks.allocated(), allocated_before);
}

#[test]
fn earlier_predicted_sibling_can_be_a_parent() {
    let mut bus = bus_with_manifest();
    let producer = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let base = bus.arenas.tasks.allocated();

    let mut first = draft(TaskKind::CONTROL_NOOP);
    first.parent = None;
    let mut second = draft(TaskKind::CONTROL_NOOP);
    second.parent = Some(TaskId::from_index(base));

    let report = commit_proposals(
        &mut bus,
        vec![
            tag(producer, Proposal::Enqueue(first)),
            tag(producer, Proposal::Enqueue(second)),
        ],
    )
    .unwrap();
    assert_eq!(report.enqueued.len(), 2);
    assert_eq!(report.enqueued[0].index(), base);
    assert_eq!(
        bus.get_task(report.enqueued[1]).unwrap().parent,
        Some(report.enqueued[0])
    );
}

#[test]
fn forward_predicted_parent_is_rejected() {
    let mut bus = bus_with_manifest();
    let producer = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let base = bus.arenas.tasks.allocated();

    let mut first = draft(TaskKind::CONTROL_NOOP);
    // References the *second* predicted ID, which is a forward reference.
    first.parent = Some(TaskId::from_index(base + 1));
    let second = draft(TaskKind::CONTROL_NOOP);

    let error = commit_proposals(
        &mut bus,
        vec![
            tag(producer, Proposal::Enqueue(first)),
            tag(producer, Proposal::Enqueue(second)),
        ],
    )
    .unwrap_err();
    assert!(matches!(error, CommitError::DanglingParent { .. }));
}

#[test]
fn predicted_sibling_depth_limit_is_atomic() {
    use cc_silicon_compiler::limits::Limits;
    use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
    let config = CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        Vec::new(),
        Limits {
            max_task_depth: 1,
            ..Limits::fixture()
        },
    );
    let mut bus = CompilerBus::new(config);
    bus.registrations
        .register(manifest_with(vec![]), &bus.schema, &bus.kinds)
        .unwrap();
    let producer = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let base = bus.arenas.tasks.allocated();
    let allocated_before = bus.arenas.tasks.allocated();

    let first = draft(TaskKind::CONTROL_NOOP);
    let mut second = draft(TaskKind::CONTROL_NOOP);
    second.parent = Some(TaskId::from_index(base));

    let error = commit_proposals(
        &mut bus,
        vec![
            tag(producer, Proposal::Enqueue(first)),
            tag(producer, Proposal::Enqueue(second)),
        ],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        CommitError::Limit(cc_silicon_compiler::limits::LimitError::TaskDepth {
            limit: 1,
            requested: 2
        })
    ));
    assert_eq!(bus.arenas.tasks.allocated(), allocated_before);
}

#[test]
fn enqueue_destination_must_be_registered_and_accept_the_kind() {
    // Unregistered destination owner.
    let mut bus = CompilerBus::default();
    let producer = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let allocated_before = bus.arenas.tasks.allocated();
    let mut child = draft(TaskKind::CONTROL_NOOP);
    child.owner = ChipId(42);
    let error =
        commit_proposals(&mut bus, vec![tag(producer, Proposal::Enqueue(child))]).unwrap_err();
    assert!(matches!(error, CommitError::UnregisteredChip { chip } if chip == ChipId(42)));
    assert_eq!(bus.arenas.tasks.allocated(), allocated_before);

    // Registered destination that does not accept the kind.
    let mut bus = CompilerBus::default();
    let mut manifest = manifest_with(vec![]);
    manifest.task_kinds = vec![TaskKind::CONTROL_UNSUPPORTED];
    bus.registrations
        .register(manifest, &bus.schema, &bus.kinds)
        .unwrap();
    let producer = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let allocated_before = bus.arenas.tasks.allocated();
    let child = draft(TaskKind::CONTROL_NOOP);
    let error =
        commit_proposals(&mut bus, vec![tag(producer, Proposal::Enqueue(child))]).unwrap_err();
    assert!(matches!(
        error,
        CommitError::TaskKindNotAccepted { chip, kind }
            if chip == ChipId(7) && kind == TaskKind::CONTROL_NOOP
    ));
    assert_eq!(bus.arenas.tasks.allocated(), allocated_before);
}
