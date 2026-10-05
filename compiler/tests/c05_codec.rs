use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, LiteralRecord, TaggedProposal};
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

/// A committed M1-subset literal fixture: decimal `Integer`, no suffix,
/// magnitude `magnitude`, unsigned, spelled `spelling`, candidate `Int`.
fn fixture_literal(magnitude: u8, spelling: &[u8]) -> LiteralRecord {
    use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
    LiteralRecord {
        token: None,
        kind: LiteralKind::Integer,
        radix: 10,
        suffix: LiteralSuffix::None,
        value: vec![magnitude],
        negative: false,
        spelling: spelling.to_vec(),
        candidate_type: Lx08CandidateType::Int,
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
fn sha256_processes_multiple_blocks() {
    // FIPS test vector: one million 'a' bytes (block-aligned input plus a
    // padding block), exercising the multi-block chunking path.
    let data = vec![b'a'; 1_000_000];
    assert_eq!(
        hex32(&sha256(&data)),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
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
    let id = allocated_then_removed.arenas.scopes.alloc(&limits).unwrap();
    allocated_then_removed.arenas.scopes.remove(id).unwrap();

    let untouched = CompilerBus::default();
    assert_eq!(allocated_then_removed.arenas.scopes.live(), 0);
    assert_eq!(untouched.arenas.scopes.live(), 0);
    assert_eq!(allocated_then_removed.arenas.scopes.allocated(), 1);
    assert_eq!(untouched.arenas.scopes.allocated(), 0);
    assert_ne!(
        Snapshot::capture(&allocated_then_removed).hash(),
        Snapshot::capture(&untouched).hash()
    );
}

// ---- /6 working-basis additions (snapshot/codec only; new fns, no edits above) ----

#[test]
fn span_offsets_encode_as_fixed_u64_le() {
    use cc_silicon_compiler::snapshot::{decode_span, encode_span};
    // Offsets above `u32::MAX` prove the fixed `u64` layout.
    let bytes = encode_span(7, 0x1_0000_0005, 0x1_0000_0009, Some(3));
    assert_eq!(bytes.len(), 4 + 8 + 8 + 1 + 4);
    assert_eq!(&bytes[0..4], &7u32.to_le_bytes());
    assert_eq!(&bytes[4..12], &0x1_0000_0005u64.to_le_bytes());
    assert_eq!(&bytes[12..20], &0x1_0000_0009u64.to_le_bytes());
    assert_eq!(bytes[20], 1);
    assert_eq!(&bytes[21..25], &3u32.to_le_bytes());
    let decoded = decode_span(&bytes).unwrap();
    assert_eq!(decoded.source, 7);
    assert_eq!(decoded.start, 0x1_0000_0005);
    assert_eq!(decoded.end, 0x1_0000_0009);
    assert_eq!(decoded.expansion, Some(3));
    // Decode + re-encode identity.
    assert_eq!(
        encode_span(
            decoded.source,
            decoded.start,
            decoded.end,
            decoded.expansion
        ),
        bytes
    );
    // Absent expansion is one flag byte with no trailing index.
    let bare = encode_span(0, 0, 9, None);
    assert_eq!(bare.len(), 4 + 8 + 8 + 1);
    let decoded_bare = decode_span(&bare).unwrap();
    assert_eq!(decoded_bare.expansion, None);
    assert_eq!(
        encode_span(
            decoded_bare.source,
            decoded_bare.start,
            decoded_bare.end,
            decoded_bare.expansion
        ),
        bare
    );
    // Truncated and trailed inputs are checked errors, never panics.
    assert!(decode_span(&[]).is_err());
    assert!(decode_span(&bytes[..10]).is_err());
    let mut trailed = bytes.clone();
    trailed.push(0);
    assert!(decode_span(&trailed).is_err());
}

#[test]
fn record_ref_tags_24_26_are_literal_sem_scope_event() {
    use cc_silicon_compiler::snapshot::{
        decode_record_ref, decode_record_ref_raw, encode_record_ref, encode_record_ref_raw,
        RECORD_REF_TAG_LITERAL, RECORD_REF_TAG_SCOPE_EVENT, RECORD_REF_TAG_SEM,
    };
    // Frozen assignment (authorizing freeze instruction + `RecordRef::wire_tag`):
    // Literal = 24, Sem = 25, ScopeEvent = 26.
    assert_eq!(RECORD_REF_TAG_LITERAL, 24);
    assert_eq!(RECORD_REF_TAG_SEM, 25);
    assert_eq!(RECORD_REF_TAG_SCOPE_EVENT, 26);
    // Raw round-trip for the three appended tags.
    for tag in [24u8, 25, 26] {
        let bytes = encode_record_ref_raw(tag, 11).unwrap();
        assert_eq!(bytes[0], tag);
        assert_eq!(decode_record_ref_raw(&bytes).unwrap(), (tag, 11));
    }
    // Tags above 26 are rejected on both paths.
    assert!(encode_record_ref_raw(27, 0).is_err());
    assert!(decode_record_ref_raw(&[27, 0, 0, 0, 0]).is_err());
    // Typed decode constructs the real variants with re-encode identity.
    for reference in [
        cc_silicon_compiler::ids::RecordRef::Literal(
            cc_silicon_compiler::ids::LiteralId::from_index(2),
        ),
        cc_silicon_compiler::ids::RecordRef::Sem(cc_silicon_compiler::ids::SemId::from_index(4)),
        cc_silicon_compiler::ids::RecordRef::ScopeEvent(
            cc_silicon_compiler::ids::ScopeEventId::from_index(6),
        ),
        cc_silicon_compiler::ids::RecordRef::Source(
            cc_silicon_compiler::ids::SourceId::from_index(3),
        ),
        cc_silicon_compiler::ids::RecordRef::Span(cc_silicon_compiler::ids::SpanId::from_index(0)),
        cc_silicon_compiler::ids::RecordRef::Artifact(
            cc_silicon_compiler::ids::ArtifactId::from_index(9),
        ),
        cc_silicon_compiler::ids::RecordRef::Task(cc_silicon_compiler::ids::TaskId::from_index(1)),
    ] {
        let bytes = encode_record_ref(reference);
        assert_eq!(bytes[0], reference.wire_tag());
        assert_eq!(decode_record_ref(&bytes).unwrap(), reference);
        assert_eq!(
            cc_silicon_compiler::snapshot::record_ref_tag(reference),
            reference.wire_tag()
        );
    }
}

#[test]
fn proposal_wire_tags_5_6_7_follow_declaration_order() {
    use cc_silicon_compiler::codec::CodecError;
    use cc_silicon_compiler::snapshot::{
        proposal_wire_name, proposal_wire_tag, result_value_tag, PROPOSAL_WIRE_TAG_APPEND_RECORDS,
        PROPOSAL_WIRE_TAG_AWAIT_CHILDREN, PROPOSAL_WIRE_TAG_PROGRESS, RESULT_VALUE_VARIANT_COUNT,
    };
    // Verified against `task.rs` declaration order: Enqueue 0, Complete 1,
    // Fail 2, AwaitHost 3, StorePatch 4; appends take 5, 6, 7.
    assert_eq!(PROPOSAL_WIRE_TAG_APPEND_RECORDS, 5);
    assert_eq!(PROPOSAL_WIRE_TAG_PROGRESS, 6);
    assert_eq!(PROPOSAL_WIRE_TAG_AWAIT_CHILDREN, 7);
    assert_eq!(proposal_wire_name(0).unwrap(), "enqueue");
    assert_eq!(proposal_wire_name(1).unwrap(), "complete");
    assert_eq!(proposal_wire_name(2).unwrap(), "fail");
    assert_eq!(proposal_wire_name(3).unwrap(), "await_host");
    assert_eq!(proposal_wire_name(4).unwrap(), "store_patch");
    assert_eq!(proposal_wire_name(5).unwrap(), "append_records");
    assert_eq!(proposal_wire_name(6).unwrap(), "progress");
    assert_eq!(proposal_wire_name(7).unwrap(), "await_children");
    assert!(matches!(
        proposal_wire_name(8),
        Err(CodecError::InvalidTag(8))
    ));
    // Existing proposals map to the frozen tags, including the additions.
    let task = cc_silicon_compiler::ids::TaskId::from_index(0);
    assert_eq!(
        proposal_wire_tag(&Proposal::Complete {
            task,
            value: ResultValue::Empty,
        }),
        1
    );
    assert_eq!(
        proposal_wire_tag(&Proposal::Enqueue(draft(TaskKind::CONTROL_NOOP))),
        0
    );
    assert_eq!(
        proposal_wire_tag(&cc_silicon_compiler::task::Proposal::Progress { task, ordinal: 3 }),
        PROPOSAL_WIRE_TAG_PROGRESS
    );
    assert_eq!(
        proposal_wire_tag(&cc_silicon_compiler::task::Proposal::AwaitChildren {
            task,
            children: vec![],
        }),
        PROPOSAL_WIRE_TAG_AWAIT_CHILDREN
    );
    assert_eq!(
        proposal_wire_tag(&cc_silicon_compiler::task::Proposal::AppendRecords {
            task,
            batch: cc_silicon_compiler::task::AppendBatch::default(),
        }),
        PROPOSAL_WIRE_TAG_APPEND_RECORDS
    );
    // ResultValue stays five variants: no DraftRecords tag exists.
    assert_eq!(RESULT_VALUE_VARIANT_COUNT, 5);
    assert_eq!(result_value_tag(&ResultValue::Empty), 0);
    assert_eq!(result_value_tag(&ResultValue::Ack), 1);
    assert_eq!(
        result_value_tag(&ResultValue::Diagnostic(
            cc_silicon_compiler::ids::DiagnosticId::from_index(0)
        )),
        4
    );
}

#[test]
fn per_encoder_round_trip_identity() {
    use cc_silicon_compiler::bus::ArtifactKind;
    use cc_silicon_compiler::snapshot::{
        decode_artifact, decode_continuation, decode_literal, decode_scope_event, decode_span,
        encode_artifact, encode_artifact_record, encode_continuation, encode_literal,
        encode_scope_event, encode_span, LiteralKind, LiteralRecordView, LiteralSuffix,
        Lx08CandidateType, ScopeEventKind, ScopeEventRecordView,
    };
    // Span.
    let span = encode_span(2, 100, 200, None);
    let back = decode_span(&span).unwrap();
    assert_eq!(
        encode_span(back.source, back.start, back.end, back.expansion),
        span
    );
    // Literal.
    let literal = LiteralRecordView {
        token: Some(9),
        kind: LiteralKind::Integer,
        radix: 10,
        suffix: LiteralSuffix::None,
        value: vec![2],
        negative: false,
        spelling: b"2".to_vec(),
        candidate_type: Lx08CandidateType::Int,
    };
    let encoded = encode_literal(&literal);
    let decoded = decode_literal(&encoded).unwrap();
    assert_eq!(decoded, literal);
    assert_eq!(encode_literal(&decoded), encoded);
    // Scope event.
    let event = ScopeEventRecordView {
        scope: 1,
        kind: ScopeEventKind::Enter,
        at: 5,
    };
    let encoded = encode_scope_event(&event);
    let decoded = decode_scope_event(&encoded).unwrap();
    assert_eq!(decoded, event);
    assert_eq!(encode_scope_event(&decoded), encoded);
    // Artifact (`/10` shape with source and map).
    let encoded = encode_artifact(ArtifactKind::Assembly, None, b"mov x0, #0", &[]);
    let record = decode_artifact(&encoded).unwrap();
    assert_eq!(record.kind, ArtifactKind::Assembly);
    assert_eq!(record.source, None);
    assert_eq!(record.bytes, b"mov x0, #0");
    assert!(record.raw_offsets.is_empty());
    assert_eq!(encode_artifact_record(&record), encoded);
    // Continuation (frozen `/6` nine-field shape; typed round-trip).
    let record = cc_silicon_compiler::task::ContinuationRecord {
        production: TaskKind::CONTROL_NOOP,
        cursor: cc_silicon_compiler::ids::TokenId::from_index(4),
        context: cc_silicon_compiler::task::ParseContext::Expression,
        binding_power: 7,
        scope: Some(cc_silicon_compiler::ids::ScopeId::from_index(1)),
        parent: Some(cc_silicon_compiler::ids::NodeId::from_index(2)),
        partial_children: vec![
            cc_silicon_compiler::ids::NodeId::from_index(3),
            cc_silicon_compiler::ids::NodeId::from_index(5),
        ],
        next_child_ordinal: 2,
        previous: None,
    };
    let encoded = encode_continuation(&record);
    let decoded = decode_continuation(&encoded).unwrap();
    assert_eq!(decoded, record);
    assert_eq!(encode_continuation(&decoded), encoded);
    // Truncated inputs are checked errors on every decoder.
    assert!(decode_literal(&[]).is_err());
    assert!(decode_scope_event(&[]).is_err());
    assert!(decode_artifact(&[]).is_err());
    assert!(decode_continuation(&[]).is_err());
}

#[test]
fn new_wire_arms_encode_deterministically() {
    use cc_silicon_compiler::task::ChildRef;
    let task = cc_silicon_compiler::ids::TaskId::from_index(0);
    // Progress ordinals distinguish snapshots.
    let progress_a = wire_bus(cc_silicon_compiler::task::Proposal::Progress { task, ordinal: 1 });
    let progress_b = wire_bus(cc_silicon_compiler::task::Proposal::Progress { task, ordinal: 2 });
    let progress_a_again =
        wire_bus(cc_silicon_compiler::task::Proposal::Progress { task, ordinal: 1 });
    assert_eq!(
        Snapshot::capture(&progress_a).hash(),
        Snapshot::capture(&progress_a_again).hash()
    );
    assert_ne!(
        Snapshot::capture(&progress_a).hash(),
        Snapshot::capture(&progress_b).hash()
    );
    // AwaitChildren child references distinguish snapshots.
    let await_empty = wire_bus(cc_silicon_compiler::task::Proposal::AwaitChildren {
        task,
        children: vec![],
    });
    let await_committed = wire_bus(cc_silicon_compiler::task::Proposal::AwaitChildren {
        task,
        children: vec![ChildRef::Committed(
            cc_silicon_compiler::ids::TaskId::from_index(4),
        )],
    });
    let await_own_batch = wire_bus(cc_silicon_compiler::task::Proposal::AwaitChildren {
        task,
        children: vec![ChildRef::OwnBatch(0)],
    });
    assert_ne!(
        Snapshot::capture(&await_empty).hash(),
        Snapshot::capture(&await_committed).hash()
    );
    assert_ne!(
        Snapshot::capture(&await_committed).hash(),
        Snapshot::capture(&await_own_batch).hash()
    );
    // An empty AppendRecords envelope encodes (draft bodies are records-track
    // owned and pending; the envelope of task + count is frozen).
    let appends = wire_bus(cc_silicon_compiler::task::Proposal::AppendRecords {
        task,
        batch: cc_silicon_compiler::task::AppendBatch::default(),
    });
    let appends_again = wire_bus(cc_silicon_compiler::task::Proposal::AppendRecords {
        task,
        batch: cc_silicon_compiler::task::AppendBatch::default(),
    });
    assert_eq!(
        Snapshot::capture(&appends).hash(),
        Snapshot::capture(&appends_again).hash()
    );
    assert_ne!(
        Snapshot::capture(&appends).hash(),
        Snapshot::capture(&await_empty).hash()
    );
}

#[test]
fn literal_identity_key_excludes_magnitudes() {
    use cc_silicon_compiler::snapshot::{
        encode_literal, literal_identity_key, LiteralKind, LiteralRecordView, LiteralSuffix,
        Lx08CandidateType,
    };
    let base = LiteralRecordView {
        token: Some(1),
        kind: LiteralKind::Integer,
        radix: 10,
        suffix: LiteralSuffix::None,
        value: vec![2],
        negative: false,
        spelling: b"2".to_vec(),
        candidate_type: Lx08CandidateType::Int,
    };
    let other = LiteralRecordView {
        value: vec![3],
        spelling: b"3".to_vec(),
        ..base.clone()
    };
    // Snapshot bytes carry magnitudes (lossless replay) ...
    assert_ne!(encode_literal(&base), encode_literal(&other));
    // ... while the hash-relevant symbolic key excludes them.
    assert_eq!(literal_identity_key(&base), literal_identity_key(&other));
    let different_kind = LiteralRecordView {
        kind: LiteralKind::Character,
        ..base.clone()
    };
    assert_ne!(
        literal_identity_key(&base),
        literal_identity_key(&different_kind)
    );
}

#[test]
fn snapshot_covers_new_reserved_arenas_in_flight_and_report() {
    use cc_silicon_compiler::bus::{TickMetrics, TickRecord};
    use cc_silicon_compiler::ids::TaskId;
    let limits = Limits::fixture();
    let mut touched = CompilerBus::default();
    // `literals` is Gate 1 typed: allocate a committed M1-subset record
    // (the snapshot now carries bodies, not just IDs).
    touched
        .arenas
        .literals
        .alloc(fixture_literal(2, b"2"), &limits)
        .unwrap();
    touched.arenas.sem.alloc(&limits).unwrap();
    touched.arenas.scope_events.alloc(&limits).unwrap();
    touched.tasks.in_flight.push(TaskId::from_index(3));
    touched
        .push_tick_record(TickRecord {
            dispatched: vec![TaskId::from_index(3)],
            metrics: TickMetrics { dispatched: 1 },
            selected: Some(TaskId::from_index(3)),
        })
        .unwrap();
    let plain = CompilerBus::default();
    assert_ne!(
        Snapshot::capture(&touched).hash(),
        Snapshot::capture(&plain).hash()
    );
}
