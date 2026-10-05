use cc_silicon_compiler::bus::{CompilerBus, TaggedProposal};
use cc_silicon_compiler::commit::{
    begin_await_children, check_backpressure_capacity, check_child_committed, check_progress_limit,
    check_progress_ordinal, check_reciprocal_pair, check_token_literal_reciprocal,
    collect_append_inventory, empty_proposal_fail, poll_await_joins, reinsert_for_progress,
    reserve_predicted_ranges, resolve_child_ref, try_fail_task, validate_draft_links, LinkTarget,
    NamePlan, PendingDraft, PredictedRange, RecordLink, ResolvedTable,
};
use cc_silicon_compiler::commit::{commit_proposals, consume_result, CommitError};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::diagnostic::DiagnosticDraft;
use cc_silicon_compiler::ids::{
    ArtifactId, BlockId, ChipId, ContinuationId, DiagnosticId, LiteralId, NodeId, RecordFamily,
    RecordRef, ScopeEventId, ScopeId, SemId, SourceId, TaskId, TokenId,
};
use cc_silicon_compiler::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use cc_silicon_compiler::task::{
    AppendBatch, ChildRef, ContinuationRecord, ContinuationRef, DraftRef, HostRequestDraft,
    HostRequestKind, KindStatus, ParseContext, PatchOp, Payload, Proposal, RegistryError,
    ResultValue, StoreId, StorePatch, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry, TaskState,
    RECORD_KINDS,
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

#[test]
fn record_ref_wire_tags_are_frozen() {
    // Frozen /5 head and tail.
    assert_eq!(RecordRef::Source(SourceId::from_index(0)).wire_tag(), 0);
    assert_eq!(
        RecordRef::Artifact(ArtifactId::from_index(0)).wire_tag(),
        23
    );
    // Frozen /6 appends: Literal = 24, Sem = 25, ScopeEvent = 26.
    assert_eq!(RecordRef::Literal(LiteralId::from_index(0)).wire_tag(), 24);
    assert_eq!(RecordRef::Sem(SemId::from_index(0)).wire_tag(), 25);
    assert_eq!(
        RecordRef::ScopeEvent(ScopeEventId::from_index(0)).wire_tag(),
        26
    );
    assert_eq!(
        RecordRef::Literal(LiteralId::from_index(3)).label(),
        "literals"
    );
    assert_eq!(RecordRef::Sem(SemId::from_index(3)).label(), "sem");
    assert_eq!(
        RecordRef::ScopeEvent(ScopeEventId::from_index(3)).label(),
        "scope_events"
    );
    // Family mapping and typed construction round-trip.
    assert_eq!(
        RecordRef::Literal(LiteralId::from_index(1)).family(),
        RecordFamily::Literal
    );
    assert_eq!(
        RecordRef::Sem(SemId::from_index(1)).family(),
        RecordFamily::Sem
    );
    assert_eq!(
        RecordRef::ScopeEvent(ScopeEventId::from_index(1)).family(),
        RecordFamily::ScopeEvent
    );
    assert_eq!(
        RecordRef::make(RecordFamily::Sem, 9),
        RecordRef::Sem(SemId::from_index(9))
    );
    // Identifier inventory follows wire-tag order.
    assert_eq!(RECORD_KINDS.len(), 27);
    assert_eq!(RECORD_KINDS[23], "artifacts");
    assert_eq!(RECORD_KINDS[24], "literals");
    assert_eq!(RECORD_KINDS[25], "sem");
    assert_eq!(RECORD_KINDS[26], "scope_events");
}

#[test]
fn record_family_ordinals_are_explicit_and_misaligned_with_wire_tags() {
    assert_eq!(RecordFamily::ALL.len(), 27);
    for (position, family) in RecordFamily::ALL.iter().enumerate() {
        assert_eq!(family.ordinal(), position as u8);
    }
    assert_eq!(RecordFamily::Literal.ordinal(), 6);
    assert_eq!(RecordFamily::ScopeEvent.ordinal(), 9);
    assert_eq!(RecordFamily::Sem.ordinal(), 12);
    // Misaligned by design: family ordinals never equal the wire tags, so no
    // arithmetic converts between the two inventories.
    assert_ne!(
        RecordFamily::Literal.ordinal(),
        RecordRef::Literal(LiteralId::from_index(0)).wire_tag()
    );
    assert_ne!(
        RecordFamily::Sem.ordinal(),
        RecordRef::Sem(SemId::from_index(0)).wire_tag()
    );
    assert_ne!(
        RecordFamily::ScopeEvent.ordinal(),
        RecordRef::ScopeEvent(ScopeEventId::from_index(0)).wire_tag()
    );
    assert_ne!(
        RecordFamily::Continuation.ordinal(),
        RecordRef::Continuation(ContinuationId::from_index(0)).wire_tag()
    );
}

#[test]
fn proposal_transitions_and_wire_tags_are_frozen() {
    let task = TaskId::from_index(1);
    let host = || Proposal::AwaitHost {
        task,
        request: HostRequestDraft {
            kind: HostRequestKind::ReadSource,
            payload: Payload::empty(),
        },
    };

    let enqueue = Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP));
    assert_eq!(enqueue.wire_tag(), 0);
    assert!(!enqueue.is_transition());

    let complete = Proposal::Complete {
        task,
        value: ResultValue::Empty,
    };
    assert_eq!(complete.wire_tag(), 1);
    assert!(complete.is_transition());

    let fail = Proposal::Fail {
        task,
        diagnostic: cc_silicon_compiler::diagnostic::DiagnosticDraft::unsupported("x"),
    };
    assert_eq!(fail.wire_tag(), 2);
    assert!(fail.is_transition());

    assert_eq!(host().wire_tag(), 3);
    assert!(host().is_transition());

    let patch = Proposal::StorePatch(StorePatch {
        owner: ChipId(7),
        task,
        version: 0,
        store: StoreId::Diagnostics,
        field: "entries",
        op: PatchOp::Append,
        target: None,
        value: None,
    });
    assert_eq!(patch.wire_tag(), 4);
    assert!(!patch.is_transition());

    // The batch element type is records-track owned; an empty batch needs no
    // draft value to spell.
    let append = Proposal::AppendRecords {
        task,
        batch: AppendBatch {
            records: Vec::new(),
        },
    };
    assert_eq!(append.wire_tag(), 5);
    assert!(!append.is_transition());

    let progress = Proposal::Progress { task, ordinal: 1 };
    assert_eq!(progress.wire_tag(), 6);
    assert!(progress.is_transition());

    let await_children = Proposal::AwaitChildren {
        task,
        children: vec![ChildRef::Committed(task), ChildRef::OwnBatch(0)],
    };
    assert_eq!(await_children.wire_tag(), 7);
    assert!(await_children.is_transition());
}

#[test]
fn result_value_set_is_unchanged_five_variants() {
    // Exhaustive match: adding a variant (e.g. DraftRecords) fails this test
    // at compile time by design.
    fn tag(value: &ResultValue) -> u8 {
        match value {
            ResultValue::Empty => 0,
            ResultValue::Ack => 1,
            ResultValue::Record(_) => 2,
            ResultValue::Records(_) => 3,
            ResultValue::Diagnostic(_) => 4,
        }
    }
    assert_eq!(tag(&ResultValue::Empty), 0);
    assert_eq!(tag(&ResultValue::Ack), 1);
    // A legal constant result reuses the generic record carrier.
    assert_eq!(
        tag(&ResultValue::Record(RecordRef::Sem(SemId::from_index(0)))),
        2
    );
    assert_eq!(tag(&ResultValue::Records(vec![])), 3);
}

#[test]
fn continuation_record_has_nine_fields_and_no_awaited() {
    // Nine ordered fields; there is no `awaited` field (children live only in
    // `TaskState::Waiting`, so naming `.awaited` here would not compile).
    let record = ContinuationRecord {
        production: TaskKind::CONTROL_NOOP,
        cursor: TokenId::from_index(4),
        context: ParseContext::Expression,
        binding_power: 7,
        scope: Some(ScopeId::from_index(0)),
        parent: Some(NodeId::from_index(1)),
        partial_children: vec![NodeId::from_index(2)],
        next_child_ordinal: 1,
        previous: None,
    };
    assert_eq!(record.production, TaskKind::CONTROL_NOOP);
    assert_eq!(record.cursor, TokenId::from_index(4));
    assert_eq!(record.context, ParseContext::Expression);
    assert_eq!(record.binding_power, 7);
    assert_eq!(record.scope, Some(ScopeId::from_index(0)));
    assert_eq!(record.parent, Some(NodeId::from_index(1)));
    assert_eq!(record.partial_children, vec![NodeId::from_index(2)]);
    assert_eq!(record.next_child_ordinal, 1);
    assert_eq!(record.previous, None);

    // ParseContext: closed 10-member vocabulary, discriminants 0-9.
    assert_eq!(ParseContext::ALL.len(), 10);
    for (position, context) in ParseContext::ALL.iter().enumerate() {
        assert_eq!(context.ordinal(), position as u8);
    }

    // Own-batch reference shapes.
    assert_eq!(DraftRef(2).index(), 2);
    assert_eq!(
        ContinuationRef::Committed(ContinuationId::from_index(5)),
        ContinuationRef::Committed(ContinuationId::from_index(5))
    );
    assert_eq!(
        ContinuationRef::OwnBatch(DraftRef(0)),
        ContinuationRef::OwnBatch(DraftRef(0))
    );
}

#[test]
fn store_id_names_is_index_20() {
    assert_eq!(StoreId::Names.index(), 20);
    assert_eq!(StoreId::Names.name(), "names");
    assert_eq!(StoreId::parse("names"), Some(StoreId::Names));
    assert_eq!(StoreId::from_index(20), Some(StoreId::Names));
    assert_eq!(StoreId::from_index(21), None);
    assert_eq!(StoreId::ALL.len(), 21);
    assert_eq!(StoreId::COUNT, 21);
    // Existing indices are unchanged by the append.
    assert_eq!(StoreId::Wires.index(), 19);
    assert_eq!(StoreId::Config.index(), 0);
}

#[test]
fn commit_error_codes_are_assigned() {
    use cc_silicon_compiler::diagnostic::DiagnosticCode;
    // Freeze record: explicit (group, code) per new variant.
    let task = TaskId::from_index(1);
    assert_eq!(
        CommitError::BackpressureCapacity {
            stage: 0,
            limit: 4,
            requested: 5
        }
        .code(),
        DiagnosticCode::new(DiagGroup::Protocol, 10)
    );
    assert_eq!(
        CommitError::SelectionBatchOverflow { limit: 1, count: 2 }.code(),
        DiagnosticCode::new(DiagGroup::Protocol, 11)
    );
    assert_eq!(
        CommitError::AwaitChildrenRefInvalid { task, reason: "x" }.code(),
        DiagnosticCode::new(DiagGroup::Protocol, 12)
    );
    assert_eq!(
        CommitError::ContinuationRefInvalid { task, reason: "x" }.code(),
        DiagnosticCode::new(DiagGroup::Protocol, 13)
    );
    assert_eq!(
        CommitError::TaskNotTransitioned { task }.code(),
        DiagnosticCode::new(DiagGroup::Protocol, 14)
    );
    assert_eq!(
        CommitError::NonAdvancingProgress { task, ordinal: 3 }.code(),
        DiagnosticCode::new(DiagGroup::Protocol, 15)
    );
    assert_eq!(
        CommitError::ProgressLimit {
            task,
            limit: 2,
            count: 2
        }
        .code(),
        DiagnosticCode::new(DiagGroup::Protocol, 16)
    );
    assert_eq!(
        CommitError::TerminatorMissing {
            task,
            block: BlockId::from_index(0)
        }
        .code(),
        DiagnosticCode::new(DiagGroup::Task, 10)
    );
    // Pre-existing variants keep the `/5` code.
    assert_eq!(
        CommitError::UnknownTask { task }.code(),
        DiagnosticCode::new(DiagGroup::Protocol, 1)
    );
    assert_eq!(
        CommitError::TaskNotTransitioned { task }
            .to_diagnostic()
            .code,
        DiagnosticCode::new(DiagGroup::Protocol, 14)
    );
}

#[test]
fn backpressure_capacity_checked_arithmetic() {
    assert!(check_backpressure_capacity(3, 2, 1, 1, 7, 0).is_ok());
    assert!(matches!(
        check_backpressure_capacity(3, 2, 1, 1, 6, 2),
        Err(CommitError::BackpressureCapacity {
            stage: 2,
            limit: 6,
            requested: 7
        })
    ));
    assert!(matches!(
        check_backpressure_capacity(u32::MAX, 1, 0, 0, u32::MAX, 0),
        Err(CommitError::BackpressureCapacity { .. })
    ));
}

#[test]
fn child_ref_committed_predicate() {
    let mut bus = CompilerBus::default();
    let parent = bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
    let mut child_draft = draft(TaskKind::CONTROL_NOOP);
    child_draft.parent = Some(parent);
    let child = bus.bootstrap_task(child_draft).unwrap();
    let other = bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();

    assert!(check_child_committed(&bus, parent, child).is_ok());
    assert!(matches!(
        check_child_committed(&bus, parent, other),
        Err(CommitError::AwaitChildrenRefInvalid { .. })
    ));
    assert!(matches!(
        check_child_committed(&bus, parent, TaskId::from_index(9999)),
        Err(CommitError::AwaitChildrenRefInvalid { .. })
    ));
    // A task is not its own committed child.
    assert!(matches!(
        check_child_committed(&bus, parent, parent),
        Err(CommitError::AwaitChildrenRefInvalid { .. })
    ));
}

#[test]
fn child_ref_own_batch_resolution() {
    let mut bus = CompilerBus::default();
    let parent = bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
    let mut child_draft = draft(TaskKind::CONTROL_NOOP);
    child_draft.parent = Some(parent);
    let child = bus.bootstrap_task(child_draft).unwrap();

    assert_eq!(
        resolve_child_ref(&bus, parent, &ChildRef::Committed(child), &[]).unwrap(),
        child
    );
    let predicted = [TaskId::from_index(40), TaskId::from_index(41)];
    assert_eq!(
        resolve_child_ref(&bus, parent, &ChildRef::OwnBatch(1), &predicted).unwrap(),
        TaskId::from_index(41)
    );
    assert!(matches!(
        resolve_child_ref(&bus, parent, &ChildRef::OwnBatch(2), &predicted),
        Err(CommitError::AwaitChildrenRefInvalid { .. })
    ));
    assert!(matches!(
        resolve_child_ref(
            &bus,
            parent,
            &ChildRef::Committed(TaskId::from_index(7777)),
            &predicted
        ),
        Err(CommitError::AwaitChildrenRefInvalid { .. })
    ));
}

#[test]
fn progress_ordinal_must_advance() {
    let task = TaskId::from_index(0);
    assert!(check_progress_ordinal(task, 3, 4).is_ok());
    assert!(matches!(
        check_progress_ordinal(task, 4, 4),
        Err(CommitError::NonAdvancingProgress { ordinal: 4, .. })
    ));
    assert!(matches!(
        check_progress_ordinal(task, 9, 2),
        Err(CommitError::NonAdvancingProgress { .. })
    ));
}

#[test]
fn progress_limit_checked() {
    let task = TaskId::from_index(0);
    assert_eq!(check_progress_limit(task, 2, 10).unwrap(), 3);
    assert!(matches!(
        check_progress_limit(task, 10, 10),
        Err(CommitError::ProgressLimit {
            limit: 10,
            count: 10,
            ..
        })
    ));
    // Zero disables `Progress`.
    assert!(matches!(
        check_progress_limit(task, 0, 0),
        Err(CommitError::ProgressLimit { .. })
    ));
    assert!(matches!(
        check_progress_limit(task, u32::MAX, u32::MAX),
        Err(CommitError::ProgressLimit { .. })
    ));
}

#[test]
fn progress_reinsert_reschedules_next_tick() {
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    reinsert_for_progress(&mut bus, task).unwrap();
    let record = bus.get_task(task).unwrap();
    assert_eq!(record.state, TaskState::Ready);
    assert_eq!(record.ready_tick, 1);
    assert_eq!(bus.tasks.ready, vec![task]);

    // A non-running task cannot progress.
    assert!(matches!(
        reinsert_for_progress(&mut bus, task),
        Err(CommitError::TaskNotRunning { .. })
    ));
    assert!(matches!(
        reinsert_for_progress(&mut bus, TaskId::from_index(4242)),
        Err(CommitError::UnknownTask { .. })
    ));
}

#[test]
fn await_children_begin_and_join() {
    use cc_silicon_compiler::task::WaitSet;
    let mut bus = CompilerBus::default();
    let parent = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let child = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    // Empty children are rejected, nothing mutates.
    assert!(matches!(
        begin_await_children(&mut bus, parent, &[]),
        Err(CommitError::AwaitChildrenRefInvalid { .. })
    ));
    assert!(matches!(
        bus.get_task(parent).unwrap().state,
        TaskState::Running
    ));
    // Non-children are rejected.
    assert!(matches!(
        begin_await_children(&mut bus, parent, &[child]),
        Err(CommitError::AwaitChildrenRefInvalid { .. })
    ));

    // Adopt the child and wait: non-terminal children hold `Waiting`, and a
    // join poll changes nothing while the child runs.
    bus.arenas.tasks.get_mut(child).unwrap().parent = Some(parent);
    begin_await_children(&mut bus, parent, &[child]).unwrap();
    assert!(matches!(
        bus.get_task(parent).unwrap().state,
        TaskState::Waiting(WaitSet { .. })
    ));
    let poll = poll_await_joins(&mut bus).unwrap();
    assert!(poll.readied.is_empty() && poll.failed.is_empty());
    assert!(matches!(
        bus.get_task(parent).unwrap().state,
        TaskState::Waiting(_)
    ));

    // The child's completion joins the pre-existing waiter in that same
    // commit: all-`Completed` readies the parent with one queue entry.
    let report = commit_proposals(
        &mut bus,
        vec![tag(
            child,
            Proposal::Complete {
                task: child,
                value: ResultValue::Empty,
            },
        )],
    )
    .unwrap();
    assert_eq!(report.rejoined, vec![parent]);
    assert!(matches!(
        bus.get_task(parent).unwrap().state,
        TaskState::Ready
    ));
    assert_eq!(bus.get_task(parent).unwrap().ready_tick, 1);

    // Poll-driven join: child completed first, then wait, then poll. Results
    // stay unconsumed at join.
    let mut bus = CompilerBus::default();
    let parent = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let child = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    bus.arenas.tasks.get_mut(child).unwrap().parent = Some(parent);
    let report = commit_proposals(
        &mut bus,
        vec![tag(
            child,
            Proposal::Complete {
                task: child,
                value: ResultValue::Empty,
            },
        )],
    )
    .unwrap();
    let result = report.completed[0].1;
    begin_await_children(&mut bus, parent, &[child]).unwrap();
    let poll = poll_await_joins(&mut bus).unwrap();
    assert_eq!(poll.readied, vec![parent]);
    assert!(matches!(
        bus.get_task(parent).unwrap().state,
        TaskState::Ready
    ));
    assert_eq!(bus.get_task(parent).unwrap().ready_tick, 1);
    assert!(bus.tasks.ready.contains(&parent));
    assert!(!bus.arenas.results.get(result).unwrap().consumed);

    // Any-`Failed` fails the parent exactly once, reusing the child
    // diagnostic and never `Ready`. The child fails first (no waiter yet),
    // then the parent waits and the poll decides.
    let mut bus = CompilerBus::default();
    let parent = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let child = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    bus.arenas.tasks.get_mut(child).unwrap().parent = Some(parent);
    let report = commit_proposals(
        &mut bus,
        vec![tag(
            child,
            Proposal::Fail {
                task: child,
                diagnostic: DiagnosticDraft::unsupported("boom"),
            },
        )],
    )
    .unwrap();
    let child_diag = report.failed[0].1;
    begin_await_children(&mut bus, parent, &[child]).unwrap();
    let poll = poll_await_joins(&mut bus).unwrap();
    assert_eq!(poll.failed, vec![(parent, child_diag)]);
    assert!(matches!(
        bus.get_task(parent).unwrap().state,
        TaskState::Failed(diag) if diag == child_diag
    ));
}

#[test]
fn commit_await_children_end_to_end() {
    // Same-batch join: a completing child readies its waiter atomically.
    let mut bus = CompilerBus::default();
    let parent = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let child = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    bus.arenas.tasks.get_mut(child).unwrap().parent = Some(parent);
    let report = commit_proposals(
        &mut bus,
        vec![
            tag(
                parent,
                Proposal::AwaitChildren {
                    task: parent,
                    children: vec![ChildRef::Committed(child)],
                },
            ),
            tag(
                child,
                Proposal::Complete {
                    task: child,
                    value: ResultValue::Empty,
                },
            ),
        ],
    )
    .unwrap();
    assert_eq!(report.rejoined, vec![parent]);
    assert!(matches!(
        bus.get_task(parent).unwrap().state,
        TaskState::Ready
    ));

    // Same-batch failure: the waiter fails once with the child diagnostic.
    let mut bus = CompilerBus::default();
    let parent = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let child = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    bus.arenas.tasks.get_mut(child).unwrap().parent = Some(parent);
    let report = commit_proposals(
        &mut bus,
        vec![
            tag(
                parent,
                Proposal::AwaitChildren {
                    task: parent,
                    children: vec![ChildRef::Committed(child)],
                },
            ),
            tag(
                child,
                Proposal::Fail {
                    task: child,
                    diagnostic: DiagnosticDraft::unsupported("boom"),
                },
            ),
        ],
    )
    .unwrap();
    assert!(report.rejoined.is_empty());
    assert_eq!(report.failed.len(), 2);
    assert_eq!(report.failed[0].0, child);
    assert_eq!(report.failed[1], (parent, report.failed[0].1));

    // `AwaitChildren` shares the single-transition guard.
    let mut bus = CompilerBus::default();
    let parent = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let child = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    bus.arenas.tasks.get_mut(child).unwrap().parent = Some(parent);
    let error = commit_proposals(
        &mut bus,
        vec![
            tag(
                parent,
                Proposal::AwaitChildren {
                    task: parent,
                    children: vec![ChildRef::Committed(child)],
                },
            ),
            tag(
                parent,
                Proposal::Complete {
                    task: parent,
                    value: ResultValue::Empty,
                },
            ),
        ],
    )
    .unwrap_err();
    assert!(matches!(error, CommitError::DuplicateCompletion { .. }));
    assert!(matches!(
        bus.get_task(parent).unwrap().state,
        TaskState::Running
    ));
}

#[test]
fn commit_await_children_own_batch() {
    // `OwnBatch(0)` resolves against this task's own `Enqueue` list even when
    // the await proposal precedes the enqueue in vector order (phase 2b).
    let mut bus = bus_with_manifest();
    let parent = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let report = commit_proposals(
        &mut bus,
        vec![
            tag(
                parent,
                Proposal::AwaitChildren {
                    task: parent,
                    children: vec![ChildRef::OwnBatch(0)],
                },
            ),
            tag(parent, Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP))),
        ],
    )
    .unwrap();
    assert_eq!(report.enqueued.len(), 1);
    let predicted = report.enqueued[0];
    assert_eq!(report.waiting, vec![parent]);
    match &bus.get_task(parent).unwrap().state {
        TaskState::Waiting(wait) => assert_eq!(wait.children, vec![predicted]),
        state => panic!("expected waiting, got {state:?}"),
    }
    // Forward own-batch indexes are rejected atomically.
    let mut bus = bus_with_manifest();
    let parent = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let before = bus.arenas.tasks.allocated();
    let error = commit_proposals(
        &mut bus,
        vec![
            tag(
                parent,
                Proposal::AwaitChildren {
                    task: parent,
                    children: vec![ChildRef::OwnBatch(3)],
                },
            ),
            tag(parent, Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP))),
        ],
    )
    .unwrap_err();
    assert!(matches!(error, CommitError::AwaitChildrenRefInvalid { .. }));
    assert_eq!(bus.arenas.tasks.allocated(), before);
}

#[test]
fn commit_progress_end_to_end() {
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    commit_proposals(
        &mut bus,
        vec![tag(task, Proposal::Progress { task, ordinal: 7 })],
    )
    .unwrap();
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Ready
    ));
    assert_eq!(bus.get_task(task).unwrap().ready_tick, 1);
    assert_eq!(bus.tasks.ready, vec![task]);

    // `Progress` shares the single-transition guard with terminal proposals.
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let error = commit_proposals(
        &mut bus,
        vec![
            tag(task, Proposal::Progress { task, ordinal: 1 }),
            tag(
                task,
                Proposal::Complete {
                    task,
                    value: ResultValue::Empty,
                },
            ),
        ],
    )
    .unwrap_err();
    assert!(matches!(error, CommitError::DuplicateCompletion { .. }));
}

#[test]
fn empty_proposal_vector_fails_per_task() {
    let task = TaskId::from_index(9);
    match empty_proposal_fail(task) {
        Proposal::Fail {
            task: inner,
            diagnostic,
        } => {
            assert_eq!(inner, task);
            assert_eq!(
                diagnostic.code,
                CommitError::TaskNotTransitioned { task }.code()
            );
        }
        other => panic!("expected fail, got {other:?}"),
    }
    // The failure commits through the normal path.
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let report = commit_proposals(&mut bus, vec![tag(task, empty_proposal_fail(task))]).unwrap();
    assert_eq!(report.failed.len(), 1);
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Failed(_)
    ));
}

#[test]
fn try_fail_task_falls_back_to_none() {
    use cc_silicon_compiler::limits::Limits;
    use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
    // Normal path: a committed diagnostic.
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let id = try_fail_task(&mut bus, task, DiagnosticDraft::unsupported("x"));
    assert!(id.is_some());
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Failed(_)
    ));

    // Saturated diagnostics: the `NONE` sentinel, still not stranded.
    let config = CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        Vec::new(),
        Limits {
            max_diagnostics: 0,
            ..Limits::fixture()
        },
    );
    let mut bus = CompilerBus::new(config);
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let id = try_fail_task(&mut bus, task, DiagnosticDraft::unsupported("x"));
    assert_eq!(id, None);
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Failed(DiagnosticId::NONE)
    ));

    // The state guard leaves non-`Running` tasks untouched.
    let mut bus = CompilerBus::default();
    let task = bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
    assert_eq!(
        try_fail_task(&mut bus, task, DiagnosticDraft::unsupported("x")),
        None
    );
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Ready
    ));
}

#[test]
fn append_inventory_counts_drafts() {
    // Only the default (empty) batch is constructible until the records track
    // lands `RecordDraft`; typed batches follow then.
    let task = TaskId::from_index(2);
    let inventory = collect_append_inventory(&[(task, AppendBatch::default())]);
    assert_eq!(inventory.len(), 1);
    assert_eq!(inventory[0].task, task);
    assert_eq!(inventory[0].drafts, 0);
}

#[test]
fn append_reservation_is_deterministic() {
    // Predicted ranges partition the index space in order.
    let ranges = reserve_predicted_ranges(10, &[2, 0, 3]);
    assert_eq!(
        ranges,
        vec![
            PredictedRange { base: 10, count: 2 },
            PredictedRange { base: 12, count: 0 },
            PredictedRange { base: 12, count: 3 },
        ]
    );
    assert!(ranges[0].contains(10) && ranges[0].contains(11));
    assert!(!ranges[0].contains(12));

    // Name plan: first plan wins per slot, deterministic.
    let mut bus = CompilerBus::default();
    let mut plan = NamePlan::new();
    assert!(plan.is_empty());
    let first = plan.plan_name(&mut bus, 0, b"main").unwrap();
    assert_eq!(plan.plan_name(&mut bus, 0, b"main").unwrap(), first);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan.get(0), Some(first));

    // Resolved table: drafts look up, committed refs pass through.
    let mut table = ResolvedTable::new();
    let reference = RecordRef::Diagnostic(DiagnosticId::from_index(2));
    table.insert(4, reference);
    assert_eq!(table.get(4), Some(reference));
    assert_eq!(table.get(5), None);
    let link = RecordLink {
        expect: RecordFamily::Diagnostic,
        target: LinkTarget::Committed(reference),
    };
    assert_eq!(table.resolve_link(&link), Some(reference));
}

#[test]
fn reciprocal_token_literal_links_validate() {
    use cc_silicon_compiler::task::DraftRef;
    let task = TaskId::from_index(0);
    let range = PredictedRange { base: 0, count: 2 };
    let token_links = vec![RecordLink {
        expect: RecordFamily::Literal,
        target: LinkTarget::Draft(DraftRef(1)),
    }];
    let literal_links = vec![RecordLink {
        expect: RecordFamily::Token,
        target: LinkTarget::Draft(DraftRef(0)),
    }];
    assert!(
        check_token_literal_reciprocal(&range, 0, &token_links, 1, &literal_links, task).is_ok()
    );
    // One-sided links are not reciprocal.
    assert!(matches!(
        check_token_literal_reciprocal(&range, 0, &[], 1, &literal_links, task),
        Err(CommitError::InvalidPatchShape { .. })
    ));
    // Out-of-range endpoints are rejected.
    assert!(matches!(
        check_token_literal_reciprocal(&range, 0, &token_links, 9, &literal_links, task),
        Err(CommitError::InvalidPatchShape { .. })
    ));
    assert!(matches!(
        check_reciprocal_pair(&range, 0, 7, task),
        Err(CommitError::InvalidPatchShape { .. })
    ));

    // Draft links validate against the reserved range.
    let drafts = vec![PendingDraft {
        family: RecordFamily::Token,
        links: token_links,
    }];
    assert!(validate_draft_links(&range, &drafts, task).is_ok());
}

#[test]
fn progress_commit_persists_count_and_ordinal() {
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    let tick = bus.control.tick;
    commit_proposals(
        &mut bus,
        vec![tag(task, Proposal::Progress { task, ordinal: 5 })],
    )
    .unwrap();
    let record = bus.arenas.tasks.get(task).unwrap();
    assert!(matches!(record.state, TaskState::Ready));
    assert_eq!(record.progress_count, 1);
    assert_eq!(record.progress_ordinal, 5);
    assert_eq!(record.ready_tick, tick.wrapping_add(1));
    assert!(bus.tasks.ready.contains(&task));
}

#[test]
fn progress_exceedance_fails_task_once() {
    let mut bus = CompilerBus::default();
    let task = running_task(&mut bus, TaskKind::CONTROL_NOOP);
    bus.arenas.tasks.get_mut(task).unwrap().progress_count = u32::MAX;
    commit_proposals(
        &mut bus,
        vec![tag(task, Proposal::Progress { task, ordinal: 1 })],
    )
    .unwrap();
    let record = bus.arenas.tasks.get(task).unwrap();
    assert!(matches!(record.state, TaskState::Failed(_)));
    // Exactly-once: the count was not persisted and the failed task is
    // never selected again (selection skips non-`Ready`).
    assert_eq!(record.progress_count, u32::MAX);
    assert!(!cc_silicon_compiler::routing::RoutingShell::new()
        .select_batch(&bus, u32::MAX)
        .contains(&task));
}
