// ============================================================================
// sprint_t29.rs — T29 execution-guard adversarial regression
//
// Independent, hand-derived checks for four `/55` boundaries:
//   * a task whose kind its routed worker does not claim is refused;
//   * a field-bearing (non-ZST) worker is refused registration;
//   * proposals tagged, named, or owned by the wrong chip/task are refused;
//   * a `StorePatch` outside the chip's declared write set is refused.
//
// Every rejection is also asserted to leave task state, patch log, store
// versions, and diagnostics untouched. A declared-write commit is the control.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, TaggedProposal};
use cc_silicon_compiler::chips::{drive_task, DriveError, Worker, WorkerRegistry};
use cc_silicon_compiler::commit::{commit_proposals, CommitError};
use cc_silicon_compiler::ids::{ChipId, DiagnosticId, RecordRef, TaskId};
use cc_silicon_compiler::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, StoreSchema,
};
use cc_silicon_compiler::task::{
    PatchOp, Payload, Proposal, ResultValue, StoreId, StorePatch, TaskDraft, TaskKind,
    TaskKindRegistry, TaskState,
};

const CONTROL_CHIP: ChipId = ChipId(7);
const FOREIGN_CHIP: ChipId = ChipId(99);
const CLAIM_CHIP: ChipId = ChipId(91);
const STATEFUL_CHIP: ChipId = ChipId(92);

fn control_manifest(id: ChipId, chip_name: &'static str) -> ChipManifest {
    ChipManifest {
        id,
        chip_name,
        group: TaskKind::CONTROL_NOOP.group(),
        task_kinds: vec![TaskKind::CONTROL_NOOP, TaskKind::CONTROL_UNSUPPORTED],
        reads: vec![],
        writes: vec![FieldPath::new(StoreId::Diagnostics, "entries")],
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["compiler/tests/sprint_t29.rs"],
        dependencies: vec![],
    }
}

fn claim_manifest() -> ChipManifest {
    ChipManifest {
        id: CLAIM_CHIP,
        chip_name: "SprintT29ClaimChip",
        group: TaskKind::CONSTANT_CONST_FOLD.group(),
        task_kinds: vec![TaskKind::CONSTANT_CONST_FOLD],
        reads: vec![],
        writes: vec![],
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["compiler/tests/sprint_t29.rs"],
        dependencies: vec![],
    }
}

fn control_bus() -> CompilerBus {
    let mut bus = CompilerBus::default();
    bus.registrations
        .register(
            control_manifest(CONTROL_CHIP, "SprintT29ControlChip"),
            &bus.schema,
            &bus.kinds,
        )
        .unwrap();
    bus
}

fn draft(kind: TaskKind, owner: ChipId) -> TaskDraft {
    TaskDraft {
        kind,
        payload: Payload::empty(),
        owner,
        parent: None,
        continuation: None,
    }
}

fn running_task(bus: &mut CompilerBus, owner: ChipId) -> TaskId {
    let task = bus
        .bootstrap_task(draft(TaskKind::CONTROL_NOOP, owner))
        .unwrap();
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    task
}

fn tag(chip: ChipId, task: TaskId, proposal: Proposal) -> TaggedProposal {
    TaggedProposal {
        chip,
        task,
        proposal,
    }
}

fn patch(
    task: TaskId,
    owner: ChipId,
    version: u64,
    store: StoreId,
    field: &'static str,
) -> Proposal {
    Proposal::StorePatch(StorePatch {
        owner,
        task,
        version,
        store,
        field,
        op: PatchOp::Append,
        target: None,
        value: Some(RecordRef::Diagnostic(DiagnosticId::from_index(0))),
    })
}

fn complete(task: TaskId) -> Proposal {
    Proposal::Complete {
        task,
        value: ResultValue::Empty,
    }
}

/// A zero-sized worker that claims the fold kind only.
struct ClaimWorker;

impl Worker for ClaimWorker {
    fn manifest(&self) -> ChipManifest {
        claim_manifest()
    }

    fn handle(&self, task: TaskId, _bus: &CompilerBus) -> Vec<Proposal> {
        vec![complete(task)]
    }
}

/// A deliberately field-bearing worker: registration must reject it.
#[allow(dead_code)]
struct StatefulWorker(u32);

impl Worker for StatefulWorker {
    fn manifest(&self) -> ChipManifest {
        control_manifest(STATEFUL_CHIP, "SprintT29StatefulChip")
    }

    fn handle(&self, task: TaskId, _bus: &CompilerBus) -> Vec<Proposal> {
        vec![complete(task)]
    }
}

#[test]
fn worker_kind_mismatch_and_zst_registration_are_guarded() {
    assert_eq!(std::mem::size_of::<ClaimWorker>(), 0);
    assert_eq!(std::mem::size_of::<StatefulWorker>(), 4);

    let mut registry = WorkerRegistry::new();
    registry.register(ClaimWorker).unwrap();
    assert_eq!(
        registry.register(StatefulWorker(1)).unwrap_err(),
        DriveError::NonStatelessWorker {
            chip: STATEFUL_CHIP
        }
    );

    // The worker is routed but claims only the fold kind; the semantic task
    // is therefore a nonmatching task for that chip.
    let mut bus = CompilerBus::default();
    bus.kinds = TaskKindRegistry::m1_slice();
    bus.schema = StoreSchema::m1_slice();
    bus.registrations
        .register(claim_manifest(), &bus.schema, &bus.kinds)
        .unwrap();
    bus.routing
        .register(TaskKind::SEMANTIC_CONST_EVAL_LITERAL, CLAIM_CHIP, 1)
        .unwrap();
    let task = bus
        .bootstrap_task(draft(TaskKind::SEMANTIC_CONST_EVAL_LITERAL, CLAIM_CHIP))
        .unwrap();

    let mut workers = WorkerRegistry::new();
    workers.register(ClaimWorker).unwrap();
    assert_eq!(
        drive_task(&bus, task, &workers).unwrap_err(),
        DriveError::KindNotClaimed {
            kind: TaskKind::SEMANTIC_CONST_EVAL_LITERAL
        }
    );
    assert_eq!(bus.get_task(task).unwrap().state, TaskState::Ready);
    assert!(bus.patch_log.is_empty());
}

#[test]
fn nonmatching_task_ids_are_rejected_without_mutation() {
    let mut bus = control_bus();
    let first = running_task(&mut bus, CONTROL_CHIP);
    let second = running_task(&mut bus, CONTROL_CHIP);

    // Inner transition names the sibling task.
    let error =
        commit_proposals(&mut bus, vec![tag(CONTROL_CHIP, first, complete(second))]).unwrap_err();
    match error {
        CommitError::InnerTaskMismatch {
            proposal_task,
            task,
        } => assert_eq!((proposal_task, task), (second, first)),
        other => panic!("expected InnerTaskMismatch, got {other:?}"),
    }
    assert_eq!(bus.get_task(first).unwrap().state, TaskState::Running);
    assert_eq!(bus.get_task(second).unwrap().state, TaskState::Running);

    // Store patch attributes itself to the sibling task.
    let error = commit_proposals(
        &mut bus,
        vec![tag(
            CONTROL_CHIP,
            first,
            patch(second, CONTROL_CHIP, 0, StoreId::Diagnostics, "entries"),
        )],
    )
    .unwrap_err();
    match error {
        CommitError::TaskAttributionMismatch { patch_task, task } => {
            assert_eq!((patch_task, task), (second, first));
        }
        other => panic!("expected TaskAttributionMismatch, got {other:?}"),
    }

    assert!(bus.patch_log.is_empty());
    assert_eq!(bus.store_versions.get(StoreId::Diagnostics), 0);
    assert_eq!(bus.arenas.diagnostics.live(), 0);
    assert_eq!(bus.get_task(first).unwrap().state, TaskState::Running);
}

#[test]
fn wrong_owner_proposals_are_rejected_without_mutation() {
    let mut bus = control_bus();
    let task = running_task(&mut bus, CONTROL_CHIP);

    // The tag names a chip that is not the task owner.
    assert_eq!(
        commit_proposals(&mut bus, vec![tag(FOREIGN_CHIP, task, complete(task))]).unwrap_err(),
        CommitError::ChipNotOwner {
            chip: FOREIGN_CHIP,
            owner: CONTROL_CHIP
        }
    );
    assert_eq!(bus.get_task(task).unwrap().state, TaskState::Running);

    // The patch owner disagrees with the (valid) producing chip tag.
    assert_eq!(
        commit_proposals(
            &mut bus,
            vec![tag(
                CONTROL_CHIP,
                task,
                patch(task, FOREIGN_CHIP, 0, StoreId::Diagnostics, "entries"),
            )]
        )
        .unwrap_err(),
        CommitError::PatchOwnerMismatch {
            patch_owner: FOREIGN_CHIP,
            chip: CONTROL_CHIP
        }
    );

    assert!(bus.patch_log.is_empty());
    assert_eq!(bus.store_versions.get(StoreId::Diagnostics), 0);
    assert_eq!(bus.arenas.diagnostics.live(), 0);
    assert_eq!(bus.get_task(task).unwrap().state, TaskState::Running);
}

#[test]
fn undeclared_write_is_rejected_then_declared_write_commits() {
    let mut bus = control_bus();
    let task = running_task(&mut bus, CONTROL_CHIP);

    // `sources.bytes` is a real schema field but not a declared chip write.
    assert_eq!(
        commit_proposals(
            &mut bus,
            vec![tag(
                CONTROL_CHIP,
                task,
                patch(task, CONTROL_CHIP, 0, StoreId::Sources, "bytes"),
            )]
        )
        .unwrap_err(),
        CommitError::WriteNotDeclared {
            chip: CONTROL_CHIP,
            store: "sources",
            field: "bytes"
        }
    );
    assert!(bus.patch_log.is_empty());
    assert_eq!(bus.store_versions.get(StoreId::Sources), 0);
    assert_eq!(bus.arenas.diagnostics.live(), 0);
    assert_eq!(bus.get_task(task).unwrap().state, TaskState::Running);

    // Control: the same task commits its declared diagnostics write.
    let report = commit_proposals(
        &mut bus,
        vec![
            tag(
                CONTROL_CHIP,
                task,
                patch(task, CONTROL_CHIP, 0, StoreId::Diagnostics, "entries"),
            ),
            tag(CONTROL_CHIP, task, complete(task)),
        ],
    )
    .unwrap();
    assert_eq!(report.patches, 1);
    assert_eq!(report.completed.len(), 1);
    assert_eq!(report.completed[0].0, task);
    assert_eq!(bus.patch_log.len(), 1);
    assert_eq!(bus.patch_log[0].field, "entries");
    assert_eq!(bus.store_versions.get(StoreId::Diagnostics), 1);
    assert!(matches!(
        bus.get_task(task).unwrap().state,
        TaskState::Completed(_)
    ));
}
