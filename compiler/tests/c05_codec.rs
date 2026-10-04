use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, TaggedProposal};
use cc_silicon_compiler::codec::{hex32, sha256, Writer};
use cc_silicon_compiler::commit::consume_result;
use cc_silicon_compiler::diagnostic::{
    DiagGroup, DiagnosticCode, DiagnosticDraft, DiagnosticRecord,
};
use cc_silicon_compiler::ids::ChipId;
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use cc_silicon_compiler::routing::RoutingShell;
use cc_silicon_compiler::snapshot::{config_hash, Snapshot, Trace};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, OptionFlag, TargetSpec};
use cc_silicon_compiler::task::{
    HostRequestDraft, HostRequestKind, Payload, Proposal, ResultRecord, ResultValue, StoreId,
    TaskDraft, TaskGroup, TaskKind,
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

fn run_scenario() -> (CompilerBus, Trace) {
    let mut bus = CompilerBus::default();
    for kind in [
        TaskKind::CONTROL_NOOP,
        TaskKind::CONTROL_NOOP,
        TaskKind::CONTROL_UNSUPPORTED,
    ] {
        bus.bootstrap_task(draft(kind)).unwrap();
    }
    let shell = RoutingShell::new();
    let mut trace = Trace::new();
    for _ in 0..5 {
        let report = shell
            .clock_tick(&CompilerPins::default(), &mut bus)
            .unwrap();
        trace.record(&bus, &report);
    }
    (bus, trace)
}

#[test]
fn sha256_matches_known_vectors() {
    assert_eq!(
        hex32(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex32(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn writer_is_deterministic() {
    let mut a = Writer::new();
    let mut b = Writer::new();
    for writer in [&mut a, &mut b] {
        writer.u32(7);
        writer.str("hello");
        writer.bytes(&[1, 2, 3]);
        writer.bool(true);
    }
    assert_eq!(a.finish(), b.finish());
}

#[test]
fn snapshot_and_trace_replay_identically() {
    let (bus_a, trace_a) = run_scenario();
    let (bus_b, trace_b) = run_scenario();
    let snap_a = Snapshot::capture(&bus_a);
    let snap_b = Snapshot::capture(&bus_b);
    assert_eq!(snap_a.bytes(), snap_b.bytes());
    assert_eq!(snap_a.hash(), snap_b.hash());
    assert_eq!(snap_a.hash(), sha256(snap_a.bytes()));
    assert_eq!(trace_a.to_bytes(), trace_b.to_bytes());
    assert_eq!(trace_a.hash(), trace_b.hash());
    assert_eq!(trace_a.len(), 5);
}

#[test]
fn different_source_bytes_change_the_snapshot() {
    let mut a = CompilerBus::default();
    let name = a.intern_name(b"a.c").unwrap();
    a.alloc_source(name, b"int x;".to_vec()).unwrap();

    let mut b = CompilerBus::default();
    let name = b.intern_name(b"a.c").unwrap();
    b.alloc_source(name, b"int yy;".to_vec()).unwrap();

    assert_ne!(Snapshot::capture(&a).hash(), Snapshot::capture(&b).hash());
}

#[test]
fn source_hash_is_computed_from_bytes() {
    let mut bus = CompilerBus::default();
    let name = bus.intern_name(b"a.c").unwrap();
    let id = bus.alloc_source(name, b"int x;".to_vec()).unwrap();
    let record = bus.arenas.sources.get(id).unwrap();
    assert_eq!(record.content_hash, sha256(b"int x;"));
}

#[test]
fn different_diagnostic_messages_change_the_snapshot() {
    let limits = Limits::fixture();
    let draft_a = DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Protocol, 1), "a");
    let draft_b = DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Protocol, 1), "b");

    let mut a = CompilerBus::default();
    a.arenas
        .diagnostics
        .alloc(DiagnosticRecord::commit(0, draft_a), &limits)
        .unwrap();
    let mut b = CompilerBus::default();
    b.arenas
        .diagnostics
        .alloc(DiagnosticRecord::commit(0, draft_b), &limits)
        .unwrap();

    assert_ne!(Snapshot::capture(&a).hash(), Snapshot::capture(&b).hash());
}

#[test]
fn routing_is_part_of_the_snapshot() {
    let mut a = CompilerBus::default();
    let mut b = CompilerBus::default();
    let kind = TaskKind::new(TaskGroup::LEX, 16).unwrap();
    b.routing.register(kind, ChipId(7), 2).unwrap();
    assert_ne!(Snapshot::capture(&a).hash(), Snapshot::capture(&b).hash());
    a.routing.register(kind, ChipId(7), 2).unwrap();
    assert_eq!(Snapshot::capture(&a).hash(), Snapshot::capture(&b).hash());
}

#[test]
fn manifests_are_part_of_the_snapshot() {
    let a = CompilerBus::default();
    let mut b = CompilerBus::default();
    let manifest = ChipManifest {
        id: ChipId(7),
        chip_name: "FixtureChip",
        group: TaskGroup::CONTROL,
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
    b.registrations
        .register(manifest, &b.schema, &b.kinds)
        .unwrap();
    assert_ne!(Snapshot::capture(&a).hash(), Snapshot::capture(&b).hash());
}

#[test]
fn config_hash_is_stable_and_order_insensitive() {
    let default_bus = CompilerBus::default();
    let first = config_hash(default_bus.config());
    let second = config_hash(default_bus.config());
    assert_eq!(first, second);

    let target = TargetSpec::aarch64_unknown_linux_gnu_unverified();
    let canonical = CompilerConfig::new(
        target,
        Dialect::C11,
        OptLevel::O0,
        vec![OptionFlag::Fwrapv, OptionFlag::FnoCommon],
        Limits::fixture(),
    );
    let reordered = CompilerConfig::new(
        target,
        Dialect::C11,
        OptLevel::O0,
        vec![OptionFlag::FnoCommon, OptionFlag::Fwrapv],
        Limits::fixture(),
    );
    assert_eq!(config_hash(&canonical), config_hash(&reordered));
}

#[test]
fn a_failed_task_state_is_recorded() {
    let (bus, _) = run_scenario();
    // The unsupported task is the one that fails.
    let failed = bus
        .arenas
        .tasks
        .iter()
        .filter(|(_, task)| task.state.name() == "failed")
        .count();
    assert_eq!(failed, 1);
}

fn wire_bus(proposal: Proposal) -> CompilerBus {
    let mut bus = CompilerBus::default();
    bus.wires.proposals.push(TaggedProposal {
        chip: ChipId(7),
        task: cc_silicon_compiler::ids::TaskId::from_index(0),
        proposal,
    });
    bus
}

#[test]
fn wire_proposal_payloads_change_the_snapshot() {
    let task = cc_silicon_compiler::ids::TaskId::from_index(0);

    // Different Complete result values.
    let empty = wire_bus(Proposal::Complete {
        task,
        value: ResultValue::Empty,
    });
    let ack = wire_bus(Proposal::Complete {
        task,
        value: ResultValue::Ack,
    });
    assert_ne!(
        Snapshot::capture(&empty).hash(),
        Snapshot::capture(&ack).hash()
    );

    // Different Enqueue parents.
    let mut with_parent = draft(TaskKind::CONTROL_NOOP);
    with_parent.parent = Some(cc_silicon_compiler::ids::TaskId::from_index(5));
    let without_parent = wire_bus(Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP)));
    let with = wire_bus(Proposal::Enqueue(with_parent));
    assert_ne!(
        Snapshot::capture(&without_parent).hash(),
        Snapshot::capture(&with).hash()
    );

    // Different Fail diagnostic messages.
    let fail_a = wire_bus(Proposal::Fail {
        task,
        diagnostic: DiagnosticDraft::unsupported("alpha"),
    });
    let fail_b = wire_bus(Proposal::Fail {
        task,
        diagnostic: DiagnosticDraft::unsupported("beta"),
    });
    assert_ne!(
        Snapshot::capture(&fail_a).hash(),
        Snapshot::capture(&fail_b).hash()
    );

    // Different AwaitHost kinds.
    let host_read = wire_bus(Proposal::AwaitHost {
        task,
        request: HostRequestDraft {
            kind: HostRequestKind::ReadSource,
            payload: Payload::empty(),
        },
    });
    let host_write = wire_bus(Proposal::AwaitHost {
        task,
        request: HostRequestDraft {
            kind: HostRequestKind::WriteArtifact,
            payload: Payload::empty(),
        },
    });
    assert_ne!(
        Snapshot::capture(&host_read).hash(),
        Snapshot::capture(&host_write).hash()
    );
}

#[test]
fn consuming_a_result_changes_the_snapshot() {
    let limits = Limits::fixture();
    let mut bus = CompilerBus::default();
    let result = bus
        .arenas
        .results
        .alloc(
            ResultRecord {
                task: cc_silicon_compiler::ids::TaskId::from_index(0),
                kind: TaskKind::CONTROL_NOOP,
                value: ResultValue::Empty,
                version: 0,
                consumed: false,
            },
            &limits,
        )
        .unwrap();
    let before = Snapshot::capture(&bus).hash();
    consume_result(&mut bus, result).unwrap();
    assert_ne!(before, Snapshot::capture(&bus).hash());
}

#[test]
fn reserved_store_tombstones_are_visible() {
    let limits = Limits::fixture();
    let mut allocated_then_removed = CompilerBus::default();
    let id = allocated_then_removed
        .arenas
        .pp_tokens
        .alloc(&limits)
        .unwrap();
    allocated_then_removed.arenas.pp_tokens.remove(id).unwrap();

    let untouched = CompilerBus::default();
    assert_eq!(allocated_then_removed.arenas.pp_tokens.live(), 0);
    assert_eq!(untouched.arenas.pp_tokens.live(), 0);
    assert_eq!(allocated_then_removed.arenas.pp_tokens.allocated(), 1);
    assert_eq!(untouched.arenas.pp_tokens.allocated(), 0);
    assert_ne!(
        Snapshot::capture(&allocated_then_removed).hash(),
        Snapshot::capture(&untouched).hash()
    );
}
