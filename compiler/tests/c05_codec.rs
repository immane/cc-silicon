use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, HostResponse, LiteralRecord, TaggedProposal,
};
use cc_silicon_compiler::codec::{hex32, sha256, CodecError, Writer};
use cc_silicon_compiler::commit::consume_result;
use cc_silicon_compiler::contract::{CONTRACT_HASH, CONTRACT_VERSION, G1_DRAFT_BODY_NAMES};
use cc_silicon_compiler::diagnostic::{
    DiagGroup, DiagnosticCode, DiagnosticDraft, DiagnosticRecord,
};
use cc_silicon_compiler::ids::{
    BlockId, ChipId, ConstId, DiagnosticId, FunctionId, HostRequestId, LiteralId, NameId, NodeId,
    PpTokenId, RecordFamily, RecordRef, ResultId, ScopeId, SourceId, SpanId, SymbolId, TaskId,
    TokenId, TypeId, ValueId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use cc_silicon_compiler::records::{G1DraftBody, RecordDraft};
use cc_silicon_compiler::routing::RoutingShell;
use cc_silicon_compiler::snapshot::{
    config_hash, encode_config, encode_record_ref, pins_digest, ReplayEvidence, Snapshot, Trace,
    TraceDivergence, TraceRecord,
};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, OptionFlag, TargetSpec};
use cc_silicon_compiler::task::{
    AppendBatch, DraftRef, HostRequestDraft, HostRequestKind, Payload, Proposal, ResultRecord,
    ResultValue, StoreId, TaskDraft, TaskGroup, TaskKind,
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
    let pins = CompilerPins::default();
    for _ in 0..5 {
        let report = shell.clock_tick(&pins, &mut bus).unwrap();
        trace.record(&bus, &report, &pins);
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
    let id = allocated_then_removed
        .arenas
        .layouts
        .alloc(&limits)
        .unwrap();
    allocated_then_removed.arenas.layouts.remove(id).unwrap();

    let untouched = CompilerBus::default();
    assert_eq!(allocated_then_removed.arenas.layouts.live(), 0);
    assert_eq!(untouched.arenas.layouts.live(), 0);
    assert_eq!(allocated_then_removed.arenas.layouts.allocated(), 1);
    assert_eq!(untouched.arenas.layouts.allocated(), 0);
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
fn record_ref_tags_24_27_are_literal_sem_scope_event_macro() {
    use cc_silicon_compiler::snapshot::{
        decode_record_ref, decode_record_ref_raw, encode_record_ref, encode_record_ref_raw,
        RECORD_REF_TAG_LITERAL, RECORD_REF_TAG_MACRO, RECORD_REF_TAG_SCOPE_EVENT,
        RECORD_REF_TAG_SEM,
    };
    // Frozen assignment (authorizing freeze instruction + `RecordRef::wire_tag`):
    // Literal = 24, Sem = 25, ScopeEvent = 26, Macro (`/23`) = 27.
    assert_eq!(RECORD_REF_TAG_LITERAL, 24);
    assert_eq!(RECORD_REF_TAG_SEM, 25);
    assert_eq!(RECORD_REF_TAG_SCOPE_EVENT, 26);
    assert_eq!(RECORD_REF_TAG_MACRO, 27);
    // Raw round-trip for the four appended tags.
    for tag in [24u8, 25, 26, 27] {
        let bytes = encode_record_ref_raw(tag, 11).unwrap();
        assert_eq!(bytes[0], tag);
        assert_eq!(decode_record_ref_raw(&bytes).unwrap(), (tag, 11));
    }
    // Tags above 27 are rejected on both paths.
    assert!(encode_record_ref_raw(28, 0).is_err());
    assert!(decode_record_ref_raw(&[28, 0, 0, 0, 0]).is_err());
    // Typed decode constructs the real variants with re-encode identity.
    for reference in [
        cc_silicon_compiler::ids::RecordRef::Literal(
            cc_silicon_compiler::ids::LiteralId::from_index(2),
        ),
        cc_silicon_compiler::ids::RecordRef::Sem(cc_silicon_compiler::ids::SemId::from_index(4)),
        cc_silicon_compiler::ids::RecordRef::ScopeEvent(
            cc_silicon_compiler::ids::ScopeEventId::from_index(6),
        ),
        cc_silicon_compiler::ids::RecordRef::Macro(cc_silicon_compiler::ids::MacroId::from_index(
            7,
        )),
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
    use cc_silicon_compiler::bus::{ScopeEventKind, ScopeEventRecord};
    use cc_silicon_compiler::snapshot::{
        decode_artifact, decode_continuation, decode_literal, decode_scope_event, decode_span,
        encode_artifact, encode_artifact_record, encode_continuation, encode_literal,
        encode_scope_event, encode_span, LiteralKind, LiteralRecordView, LiteralSuffix,
        Lx08CandidateType,
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
    // Scope event (committed record since the `/13` freeze).
    let event = ScopeEventRecord {
        scope: cc_silicon_compiler::ids::ScopeId::from_index(1),
        kind: ScopeEventKind::Enter,
        at: cc_silicon_compiler::ids::NodeId::from_index(5),
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
    touched.arenas.inits.alloc(&limits).unwrap();
    touched.arenas.layouts.alloc(&limits).unwrap();
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

// ---- /41 D-12 append bodies and D-15 trace/replay evidence ----

fn append_bus(records: Vec<RecordDraft>, bodies: Vec<G1DraftBody>) -> CompilerBus {
    wire_bus(Proposal::AppendRecords {
        task: TaskId::from_index(0),
        batch: AppendBatch { records, bodies },
    })
}

fn draft_handle(family: RecordFamily, index: u32) -> RecordDraft {
    RecordDraft {
        family,
        index: DraftRef(index),
    }
}

fn all_draft_bodies() -> Vec<G1DraftBody> {
    use cc_silicon_compiler::bus::{
        ArtifactKind, ArtifactRecord, BlockRecord, ConstRecord, EffectMask, FunctionRecord,
        InstructionRecord, IntRank, IrOp, Linkage, MacroRecord, NodeKind, NodeRecord, PpTokenKind,
        PpTokenRecord, ScopeEventKind, ScopeEventRecord, ScopeKind, ScopeRecord, SemRecord,
        SpanRecord, StorageDuration, SymbolKind, SymbolRecord, TokenKind, TokenRecord, TypeKind,
        TypeRecord, ValueCategory, ValueRecord,
    };
    let span = SpanId::from_index(0);
    let name = NameId::from_index(0);
    let node = NodeId::from_index(0);
    let ty = TypeId::from_index(0);
    let token = TokenId::from_index(0);
    vec![
        G1DraftBody::Literal(fixture_literal(7, b"7")),
        G1DraftBody::Const(ConstRecord {
            value: vec![7],
            negative: false,
        }),
        G1DraftBody::Artifact(ArtifactRecord {
            kind: ArtifactKind::Assembly,
            source: None,
            bytes: b"nop".to_vec(),
            raw_offsets: vec![],
        }),
        G1DraftBody::Token(TokenRecord {
            kind: TokenKind::Identifier,
            span,
            name: Some(name),
            pp_token: PpTokenId::from_index(0),
        }),
        G1DraftBody::Name {
            spelling: b"name".to_vec(),
        },
        G1DraftBody::Node(NodeRecord {
            kind: NodeKind::IntLiteral,
            parent: None,
            children: vec![],
            first_token: token,
            last_token: token,
            name: None,
            literal: None,
        }),
        G1DraftBody::Type(TypeRecord {
            kind: TypeKind::Int {
                rank: IntRank::Int,
                signed: true,
            },
        }),
        G1DraftBody::Symbol(SymbolRecord {
            name,
            scope: ScopeId::from_index(0),
            kind: SymbolKind::Function,
            ty: Some(ty),
            linkage: Linkage::External,
            storage: StorageDuration::Static,
            decl: node,
        }),
        G1DraftBody::Scope(ScopeRecord {
            kind: ScopeKind::File,
            parent: None,
            owner: None,
        }),
        G1DraftBody::ScopeEvent(ScopeEventRecord {
            scope: ScopeId::from_index(0),
            kind: ScopeEventKind::Enter,
            at: node,
        }),
        G1DraftBody::Sem(SemRecord {
            node,
            ty,
            category: ValueCategory::NonLvalue,
            effects: EffectMask(0),
        }),
        G1DraftBody::Function(FunctionRecord {
            symbol: SymbolId::from_index(0),
            signature: ty,
            entry: BlockId::from_index(0),
            linkage: Linkage::External,
        }),
        G1DraftBody::Block(BlockRecord {
            function: FunctionId::from_index(0),
            ordinal: 0,
        }),
        G1DraftBody::Value(ValueRecord { ty }),
        G1DraftBody::Instruction(InstructionRecord {
            op: IrOp::Constant,
            block: BlockId::from_index(0),
            operands: vec![],
            immediate: Some(ConstId::from_index(0)),
            result: Some(ValueId::from_index(0)),
        }),
        G1DraftBody::Span(SpanRecord {
            source: SourceId::from_index(0),
            start: 0,
            end: 1,
            expansion: None,
        }),
        G1DraftBody::PpToken(PpTokenRecord {
            kind: PpTokenKind::Identifier,
            span,
            spelling: b"x".to_vec(),
        }),
        G1DraftBody::Macro(MacroRecord {
            spelling: b"m".to_vec(),
            params: vec![],
            function_like: false,
            variadic: false,
            replacement: vec![],
            undefined: false,
        }),
    ]
}

/// A second body of the same variant, differing in one field, so the
/// sensitivity check proves the body bytes (not only the variant name) are
/// encoded.
fn distinct_body(body: &G1DraftBody) -> G1DraftBody {
    use cc_silicon_compiler::bus::{IntRank, IrOp, Linkage, ScopeEventKind, ScopeKind, TypeKind};
    match body.clone() {
        G1DraftBody::Literal(mut record) => {
            record.value = vec![9];
            record.spelling = b"9".to_vec();
            G1DraftBody::Literal(record)
        }
        G1DraftBody::Const(mut record) => {
            record.negative = true;
            G1DraftBody::Const(record)
        }
        G1DraftBody::Artifact(mut record) => {
            record.bytes = b"ret".to_vec();
            G1DraftBody::Artifact(record)
        }
        G1DraftBody::Token(mut record) => {
            record.span = SpanId::from_index(1);
            G1DraftBody::Token(record)
        }
        G1DraftBody::Name { .. } => G1DraftBody::Name {
            spelling: b"other".to_vec(),
        },
        G1DraftBody::Node(mut record) => {
            record.first_token = TokenId::from_index(1);
            G1DraftBody::Node(record)
        }
        G1DraftBody::Type(mut record) => {
            record.kind = TypeKind::Int {
                rank: IntRank::Long,
                signed: true,
            };
            G1DraftBody::Type(record)
        }
        G1DraftBody::Symbol(mut record) => {
            record.linkage = Linkage::Internal;
            G1DraftBody::Symbol(record)
        }
        G1DraftBody::Scope(mut record) => {
            record.kind = ScopeKind::Block;
            G1DraftBody::Scope(record)
        }
        G1DraftBody::ScopeEvent(mut record) => {
            record.kind = ScopeEventKind::Exit;
            G1DraftBody::ScopeEvent(record)
        }
        G1DraftBody::Sem(mut record) => {
            record.node = NodeId::from_index(1);
            G1DraftBody::Sem(record)
        }
        G1DraftBody::Function(mut record) => {
            record.entry = BlockId::from_index(1);
            G1DraftBody::Function(record)
        }
        G1DraftBody::Block(mut record) => {
            record.ordinal = 1;
            G1DraftBody::Block(record)
        }
        G1DraftBody::Value(mut record) => {
            record.ty = TypeId::from_index(1);
            G1DraftBody::Value(record)
        }
        G1DraftBody::Instruction(mut record) => {
            record.op = IrOp::Return;
            G1DraftBody::Instruction(record)
        }
        G1DraftBody::Span(mut record) => {
            record.end = 2;
            G1DraftBody::Span(record)
        }
        G1DraftBody::PpToken(mut record) => {
            record.spelling = b"y".to_vec();
            G1DraftBody::PpToken(record)
        }
        G1DraftBody::Macro(mut record) => {
            record.spelling = b"other".to_vec();
            G1DraftBody::Macro(record)
        }
    }
}

#[test]
fn append_wire_body_sensitivity() {
    let empty = Snapshot::capture(&append_bus(vec![], vec![])).hash();
    let empty_again = Snapshot::capture(&append_bus(vec![], vec![])).hash();
    assert_eq!(empty, empty_again);
    assert_eq!(G1_DRAFT_BODY_NAMES.len(), 18);

    // Every typed body variant changes the snapshot, re-encodes
    // deterministically with the same handle, and remains sensitive to its
    // body bytes (same variant, same handle, one changed field).
    let bodies = all_draft_bodies();
    assert_eq!(bodies.len(), 18);
    for body in &bodies {
        let family = body.family();
        let first = Snapshot::capture(&append_bus(
            vec![draft_handle(family, 0)],
            vec![body.clone()],
        ))
        .hash();
        let second = Snapshot::capture(&append_bus(
            vec![draft_handle(family, 0)],
            vec![body.clone()],
        ))
        .hash();
        let changed = Snapshot::capture(&append_bus(
            vec![draft_handle(family, 0)],
            vec![distinct_body(body)],
        ))
        .hash();
        assert_eq!(first, second, "body {family:?} not deterministic");
        assert_ne!(empty, first, "body {family:?} is invisible");
        assert_ne!(first, changed, "body {family:?} bytes are invisible");
    }

    // Same count, different handle family (the body bytes stay equal: this is
    // deliberately a malformed pair, because the encoder must not validate).
    let const_body = bodies[1].clone();
    let as_const = append_bus(
        vec![draft_handle(RecordFamily::Const, 0)],
        vec![const_body.clone()],
    );
    let as_literal_handle = append_bus(
        vec![draft_handle(RecordFamily::Literal, 0)],
        vec![const_body.clone()],
    );
    assert_ne!(
        Snapshot::capture(&as_const).hash(),
        Snapshot::capture(&as_literal_handle).hash()
    );

    // Same count, different draft index.
    let index_zero = append_bus(
        vec![draft_handle(RecordFamily::Const, 0)],
        vec![const_body.clone()],
    );
    let index_one = append_bus(
        vec![draft_handle(RecordFamily::Const, 1)],
        vec![const_body.clone()],
    );
    assert_ne!(
        Snapshot::capture(&index_zero).hash(),
        Snapshot::capture(&index_one).hash()
    );

    // Same handle, different body bytes.
    let G1DraftBody::Literal(literal_a) = bodies[0].clone() else {
        panic!("fixture order changed");
    };
    let mut literal_b = literal_a.clone();
    literal_b.value = vec![8];
    literal_b.spelling = b"8".to_vec();
    let body_a = append_bus(
        vec![draft_handle(RecordFamily::Literal, 0)],
        vec![G1DraftBody::Literal(literal_a.clone())],
    );
    let body_b = append_bus(
        vec![draft_handle(RecordFamily::Literal, 0)],
        vec![G1DraftBody::Literal(literal_b)],
    );
    assert_ne!(
        Snapshot::capture(&body_a).hash(),
        Snapshot::capture(&body_b).hash()
    );

    // Stored order is preserved: swapping two handles and their bodies
    // changes the snapshot (no sorting).
    let ordered = append_bus(
        vec![
            draft_handle(RecordFamily::Const, 0),
            draft_handle(RecordFamily::Literal, 1),
        ],
        vec![const_body.clone(), G1DraftBody::Literal(literal_a.clone())],
    );
    let reversed = append_bus(
        vec![
            draft_handle(RecordFamily::Literal, 1),
            draft_handle(RecordFamily::Const, 0),
        ],
        vec![G1DraftBody::Literal(literal_a), const_body.clone()],
    );
    assert_ne!(
        Snapshot::capture(&ordered).hash(),
        Snapshot::capture(&reversed).hash()
    );

    // Malformed handle/body count mismatches encode deterministically as-is;
    // commit (not the snapshot encoder) owns the 1:1 validation.
    let missing_body = append_bus(vec![draft_handle(RecordFamily::Const, 0)], vec![]);
    let missing_body_again = append_bus(vec![draft_handle(RecordFamily::Const, 0)], vec![]);
    let missing_handle = append_bus(vec![], vec![const_body]);
    assert_eq!(
        Snapshot::capture(&missing_body).hash(),
        Snapshot::capture(&missing_body_again).hash()
    );
    assert_ne!(empty, Snapshot::capture(&missing_body).hash());
    assert_ne!(empty, Snapshot::capture(&missing_handle).hash());
    assert_ne!(
        Snapshot::capture(&missing_body).hash(),
        Snapshot::capture(&missing_handle).hash()
    );
}

#[test]
fn snapshot_task_self_id_participates() {
    let mut baseline = CompilerBus::default();
    baseline
        .bootstrap_task(draft(TaskKind::CONTROL_NOOP))
        .unwrap();
    let mut divergent = CompilerBus::default();
    let id = divergent
        .bootstrap_task(draft(TaskKind::CONTROL_NOOP))
        .unwrap();
    divergent.arenas.tasks.get_mut(id).unwrap().id = TaskId::from_index(id.index().wrapping_add(1));
    assert_ne!(
        Snapshot::capture(&baseline).hash(),
        Snapshot::capture(&divergent).hash()
    );
}

#[test]
fn snapshot_control_commit_ordinal_participates() {
    let baseline = CompilerBus::default();
    let mut divergent = CompilerBus::default();
    divergent.control.next_commit_ordinal = 1;
    assert_ne!(
        Snapshot::capture(&baseline).hash(),
        Snapshot::capture(&divergent).hash()
    );
}

fn sample_trace_record(tick: u64) -> TraceRecord {
    TraceRecord {
        tick,
        pins_digest: [0x11; 32],
        selected: Some(TaskId::from_index(0)),
        outcome: "executed",
        resolution: "noop",
        error: None,
        ready_len: 1,
        enqueued: vec![TaskId::from_index(1)],
        completed: vec![(TaskId::from_index(0), ResultId::from_index(0))],
        failed: vec![],
        waiting: vec![],
        rejoined: vec![],
        appended: vec![RecordRef::Literal(LiteralId::from_index(0))],
        patches: 2,
    }
}

/// Independently-written canonical trace bytes, used to feed arbitrary IDs
/// into `Trace::from_bytes` (the round-trip test uses a captured trace).
fn encode_trace_records(records: &[TraceRecord]) -> Vec<u8> {
    let mut w = Writer::new();
    w.str(CONTRACT_VERSION);
    w.u64(records.len() as u64);
    for record in records {
        w.u64(record.tick);
        w.raw(&record.pins_digest);
        match record.selected {
            Some(id) => {
                w.u8(1);
                w.u32(id.index());
            }
            None => w.u8(0),
        }
        w.str(record.outcome);
        w.str(record.resolution);
        match record.error {
            Some(code) => {
                w.u8(1);
                w.str(code.group.name());
                w.u16(code.code);
            }
            None => w.u8(0),
        }
        w.u32(record.ready_len);
        w.u64(record.enqueued.len() as u64);
        for id in &record.enqueued {
            w.u32(id.index());
        }
        w.u64(record.completed.len() as u64);
        for (task, result) in &record.completed {
            w.u32(task.index());
            w.u32(result.index());
        }
        w.u64(record.failed.len() as u64);
        for (task, diagnostic) in &record.failed {
            w.u32(task.index());
            w.u32(diagnostic.index());
        }
        w.u64(record.waiting.len() as u64);
        for id in &record.waiting {
            w.u32(id.index());
        }
        w.u64(record.rejoined.len() as u64);
        for id in &record.rejoined {
            w.u32(id.index());
        }
        w.u64(record.appended.len() as u64);
        for reference in &record.appended {
            w.raw(&encode_record_ref(*reference));
        }
        w.u32(record.patches);
    }
    w.finish()
}

fn trace_from_records(records: &[TraceRecord]) -> Trace {
    Trace::from_bytes(&encode_trace_records(records)).unwrap()
}

#[test]
fn pins_digest_is_part_of_the_trace() {
    let pins_a = CompilerPins::default();
    let pins_b = CompilerPins {
        start_job: true,
        ..CompilerPins::default()
    };
    assert_ne!(pins_digest(&pins_a), pins_digest(&pins_b));

    // Host-response order is input identity: reversing equal entries changes
    // the digest.
    let responses = vec![
        HostResponse {
            request: HostRequestId::from_index(1),
            bytes: b"a".to_vec(),
        },
        HostResponse {
            request: HostRequestId::from_index(2),
            bytes: b"b".to_vec(),
        },
    ];
    let mut reversed = responses.clone();
    reversed.reverse();
    let ordered_pins = CompilerPins {
        host_responses: responses,
        ..CompilerPins::default()
    };
    let reversed_pins = CompilerPins {
        host_responses: reversed,
        ..CompilerPins::default()
    };
    assert_ne!(pins_digest(&ordered_pins), pins_digest(&reversed_pins));

    // Same bus state and report, different pins: trace bytes must differ.
    let mut bus = CompilerBus::default();
    bus.bootstrap_task(draft(TaskKind::CONTROL_NOOP)).unwrap();
    let report = RoutingShell::new().clock_tick(&pins_a, &mut bus).unwrap();
    let mut trace_a = Trace::new();
    let mut trace_b = Trace::new();
    trace_a.record(&bus, &report, &pins_a);
    trace_b.record(&bus, &report, &pins_b);
    assert_eq!(trace_a.records()[0].pins_digest, pins_digest(&pins_a));
    assert_ne!(trace_a.to_bytes(), trace_b.to_bytes());
    assert_ne!(trace_a.hash(), trace_b.hash());
}

#[test]
fn trace_decoder_round_trip_and_errors() {
    let (_, trace) = run_scenario();
    let bytes = trace.to_bytes();
    let decoded = Trace::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, trace);
    assert_eq!(decoded.to_bytes(), bytes);
    assert_eq!(decoded.hash(), trace.hash());

    // Synthetic record with every optional/vector field populated.
    let mut full = sample_trace_record(4);
    full.error = Some(DiagnosticCode::new(DiagGroup::Protocol, 18));
    full.failed = vec![(TaskId::from_index(2), DiagnosticId::from_index(3))];
    full.waiting = vec![TaskId::from_index(4)];
    full.rejoined = vec![TaskId::from_index(5)];
    let synthetic = encode_trace_records(&[full]);
    let decoded = Trace::from_bytes(&synthetic).unwrap();
    assert_eq!(decoded.to_bytes(), synthetic);

    // Truncation and trailing bytes are checked errors.
    assert!(Trace::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    assert!(Trace::from_bytes(&[]).is_err());
    let mut trailed = bytes.clone();
    trailed.push(0);
    assert!(matches!(
        Trace::from_bytes(&trailed),
        Err(CodecError::TrailingBytes)
    ));

    // A huge record count cannot reserve against a tiny input.
    let mut w = Writer::new();
    w.str(CONTRACT_VERSION);
    w.u64(u64::MAX);
    assert!(Trace::from_bytes(&w.finish()).is_err());

    // Unknown stable names fail as CodecError, not as silent fallbacks.
    let mut w = Writer::new();
    w.str(CONTRACT_VERSION);
    w.u64(1);
    w.u64(1);
    w.raw(&[0u8; 32]);
    w.u8(0);
    w.str("bogus_outcome");
    w.str("none");
    w.u8(0);
    w.u32(0);
    for _ in 0..6 {
        w.u64(0);
    }
    w.u32(0);
    assert!(matches!(
        Trace::from_bytes(&w.finish()),
        Err(CodecError::Unsupported(_))
    ));

    // Unknown option tag fails as CodecError (mutate the `selected` flag in a
    // full-length record so the bounded-count pre-check does not reject it
    // first).
    let mut unknown_tag = encode_trace_records(&[sample_trace_record(1)]);
    let selected_offset = 8 + CONTRACT_VERSION.len() + 8 + 8 + 32;
    assert_eq!(unknown_tag[selected_offset], 1);
    unknown_tag[selected_offset] = 2;
    assert!(matches!(
        Trace::from_bytes(&unknown_tag),
        Err(CodecError::InvalidTag(2))
    ));
}

#[test]
fn trace_ids_and_first_divergence() {
    let base = sample_trace_record(7);
    let base_trace = trace_from_records(std::slice::from_ref(&base));

    // Equal counts, different completed result ID.
    let mut completed = base.clone();
    completed.completed = vec![(TaskId::from_index(0), ResultId::from_index(1))];
    // Equal counts, different failed diagnostic ID.
    let mut failed = base.clone();
    failed.failed = vec![(TaskId::from_index(0), DiagnosticId::from_index(1))];
    // Equal counts, different appended record reference.
    let mut appended = base.clone();
    appended.appended = vec![RecordRef::Literal(LiteralId::from_index(1))];

    for changed in [completed, failed, appended] {
        let changed_trace = trace_from_records(&[changed]);
        assert_ne!(base_trace.hash(), changed_trace.hash());
        assert_eq!(
            base_trace.diff(&changed_trace).first_divergence,
            Some(TraceDivergence { index: 0, tick: 7 })
        );
    }

    // First divergent record, not the last: records 0 and 2 match, record 1
    // differs, so the reported index/tick is (1, 2).
    let first = sample_trace_record(1);
    let second = sample_trace_record(2);
    let third = sample_trace_record(3);
    let mut changed_second = second.clone();
    changed_second.completed = vec![(TaskId::from_index(0), ResultId::from_index(9))];
    let left = trace_from_records(&[first.clone(), second, third.clone()]);
    let right = trace_from_records(&[first, changed_second, third]);
    assert_eq!(
        left.diff(&right).first_divergence,
        Some(TraceDivergence { index: 1, tick: 2 })
    );
    assert_eq!(left.diff(&left).first_divergence, None);
}

#[test]
fn trace_records_actual_commit_ids() {
    let limits = Limits::fixture();
    let pins = CompilerPins::default();
    let shell = RoutingShell::new();

    let mut baseline = CompilerBus::default();
    let mut shifted = CompilerBus::default();
    for bus in [&mut baseline, &mut shifted] {
        for kind in [
            TaskKind::CONTROL_NOOP,
            TaskKind::CONTROL_NOOP,
            TaskKind::CONTROL_UNSUPPORTED,
        ] {
            bus.bootstrap_task(draft(kind)).unwrap();
        }
    }
    // Consume arena slots so the same scenario produces different result and
    // diagnostic IDs with identical counts.
    shifted
        .arenas
        .results
        .alloc(
            ResultRecord {
                task: TaskId::from_index(0),
                kind: TaskKind::CONTROL_NOOP,
                value: ResultValue::Empty,
                version: 0,
                consumed: false,
            },
            &limits,
        )
        .unwrap();
    shifted
        .arenas
        .diagnostics
        .alloc(
            DiagnosticRecord::commit(
                0,
                DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Protocol, 1), "preallocated"),
            ),
            &limits,
        )
        .unwrap();

    let mut baseline_trace = Trace::new();
    let mut shifted_trace = Trace::new();
    for _ in 0..5 {
        let report = shell.clock_tick(&pins, &mut baseline).unwrap();
        baseline_trace.record(&baseline, &report, &pins);
        let report = shell.clock_tick(&pins, &mut shifted).unwrap();
        shifted_trace.record(&shifted, &report, &pins);
    }

    let completed = |trace: &Trace| {
        trace
            .records()
            .iter()
            .map(|record| record.completed.len())
            .sum::<usize>()
    };
    let failed = |trace: &Trace| {
        trace
            .records()
            .iter()
            .map(|record| record.failed.len())
            .sum::<usize>()
    };
    assert_eq!(completed(&baseline_trace), 2);
    assert_eq!(completed(&shifted_trace), 2);
    assert_eq!(failed(&baseline_trace), 1);
    assert_eq!(failed(&shifted_trace), 1);
    assert_ne!(baseline_trace.hash(), shifted_trace.hash());
    assert_eq!(
        baseline_trace.diff(&shifted_trace).first_divergence,
        Some(TraceDivergence { index: 0, tick: 0 })
    );
}

#[test]
fn replay_evidence_is_deterministic_and_pin_sensitive() {
    let (bus_a, trace_a) = run_scenario();
    let (bus_b, trace_b) = run_scenario();
    let pins = CompilerPins::default();
    let evidence = |bus: &CompilerBus, trace: &Trace| ReplayEvidence {
        contract_version: CONTRACT_VERSION.to_string(),
        contract_hash: CONTRACT_HASH.to_string(),
        config: encode_config(bus.config()),
        initial_snapshot_hash: Snapshot::capture(&CompilerBus::default()).hash(),
        pins_digests: vec![pins_digest(&pins); trace.len()],
        trace_hash: trace.hash(),
    };
    let first = evidence(&bus_a, &trace_a);
    let second = evidence(&bus_b, &trace_b);
    assert_eq!(first.to_bytes(), second.to_bytes());
    assert_eq!(first.hash(), second.hash());
    assert_eq!(
        ReplayEvidence::from_bytes(&first.to_bytes()).unwrap(),
        first
    );

    let mut changed = first.clone();
    changed.pins_digests[0] = pins_digest(&CompilerPins {
        start_job: true,
        ..CompilerPins::default()
    });
    assert_ne!(first.to_bytes(), changed.to_bytes());
    assert_ne!(first.hash(), changed.hash());
}
