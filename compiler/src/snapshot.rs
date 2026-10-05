// ============================================================================
// snapshot.rs — deterministic snapshot and trace (T01 C05)
//
// A snapshot encodes every observable store of the compiler bus into a
// canonical byte string. Addresses and private caches never participate. A
// trace records one entry per tick. Replaying identical input, configuration,
// and initial state therefore yields byte-identical snapshots and traces.
//
// Foundation records (tasks, results, diagnostics, host requests, artifacts,
// sources, spans, expansions, continuations) are encoded in full. Routing,
// registered manifests, the task-kind registry, the store schema, the intern
// table, committed patches, and per-store versions are encoded. Reserved
// language stores have no frozen record schema yet, so the snapshot encodes
// their allocated live IDs only; their record bodies remain reserved until the
// owning task group freezes them. Gate 1 (`/7`) types the `literals` and
// `consts` arenas, whose bodies are encoded in full below.
// ============================================================================

use crate::bus::{
    ArtifactKind, ArtifactRecord, CompilerBus, ConstRecord, LiteralRecord, NodeKind, NodeRecord,
    PpTokenKind, PpTokenRecord, SpanRecord, TokenKind, TokenRecord,
};
use crate::codec::{hex32, sha256, CodecError, Reader, Writer};
use crate::diagnostic::{DiagnosticRecord, Severity};
use crate::ids::{
    ArtifactId, BlockId, ConstId, ContinuationId, DiagnosticId, ExpansionId, FunctionId,
    HostRequestId, InitId, InstructionId, LayoutId, LiteralId, NameId, NodeId, PpTokenId,
    RecordRef, ResultId, ScopeEventId, ScopeId, SemId, SourceId, SpanId, SymbolId, TaskId, TokenId,
    TypeId, VRegId, ValueId,
};
use crate::routing::{Resolution, TickOutcome, TickReport};
use crate::target::{CompilerConfig, VerificationState};
use crate::task::{
    ChildRef, ContinuationRecord, HostRequestKind, ParseContext, PatchOp, Proposal, ResultRecord,
    ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

fn push_verification(w: &mut Writer, verification: VerificationState) {
    match verification.report_hash() {
        Some(report_hash) => {
            w.u8(1);
            w.raw(&report_hash);
        }
        None => w.u8(0),
    }
}

fn push_config(w: &mut Writer, config: &CompilerConfig) {
    let target = config.target();
    let profile = target.profile();
    w.str(profile.triple);
    w.str(profile.object_format.name());
    w.str(profile.data_model.name());
    w.str(profile.endianness.name());
    w.str(profile.abi_name);
    push_verification(w, target.verification());
    for kind in crate::target::ScalarKind::ALL {
        let scalar = target.scalar(kind);
        w.u8(scalar.size);
        w.u8(scalar.align);
        match scalar.signed {
            Some(signed) => {
                w.u8(1);
                w.bool(signed);
            }
            None => w.u8(0),
        }
        match scalar.float {
            Some(format) => {
                w.u8(1);
                w.str(format.name());
            }
            None => w.u8(0),
        }
    }
    let abi = target.abi();
    w.str(abi.name);
    w.u8(abi.gp_arg_regs);
    w.u8(abi.fp_arg_regs);
    w.u8(abi.stack_align);
    w.bool(abi.variadic_register_save_area);
    w.str(target.wchar_encoding().name());
    w.str(config.dialect().name());
    w.str(config.opt_level().name());
    w.u64(config.options().len() as u64);
    for option in config.options() {
        w.str(option.name());
    }
    let limits = config.limits();
    w.u32(limits.max_records_per_arena);
    w.u64(limits.max_records_total);
    w.u64(limits.max_source_bytes);
    w.u32(limits.max_intern_entries);
    w.u64(limits.max_intern_bytes);
    w.u32(limits.max_queue_len);
    w.u64(limits.max_tasks_total);
    w.u32(limits.max_task_depth);
    w.u32(limits.max_diagnostics);
    w.u64(limits.max_ticks);
    w.u32(limits.max_proposals_per_tick);
    // `/9`: every configured bound participates in the canonical config.
    // Omitting a bound here would let two jobs share a snapshot but diverge
    // on the next tick (bit budget, dispatch quota, per-stage bound, or
    // progress bound). Keep this list in sync with `FrozenSchema::encode`.
    w.u32(limits.max_inflight_per_tick);
    for bound in limits.stage_queue_bound {
        w.u32(bound);
    }
    w.u32(limits.max_const_bits);
    w.u32(limits.max_task_progress);
}

/// Canonical bytes of a configuration. Stable across runs.
pub fn encode_config(config: &CompilerConfig) -> Vec<u8> {
    let mut w = Writer::new();
    push_config(&mut w, config);
    w.finish()
}

/// SHA-256 of a configuration's canonical bytes.
pub fn config_hash(config: &CompilerConfig) -> [u8; 32] {
    sha256(&encode_config(config))
}

// Frozen `/6` wire tags for the three appended families, in `RecordRef`
// declaration order: `Literal` = 24, `Sem` = 25, `ScopeEvent` = 26
// (`RecordRef::wire_tag` in `ids.rs` is authoritative; the constants below
// mirror it so snapshot-local encode/decode paths name the frozen numbers).
// Tags 0-23 are the frozen `/5` inventory and are unchanged.
//
// Record-family declaration order (matching `RecordRef` declaration order):
// Source = 0, Span = 1, Expansion = 2, PpToken = 3, Token = 4, Name = 5,
// Scope = 6, Symbol = 7, Type = 8, Node = 9, Const = 10, Layout = 11,
// Init = 12, Function = 13, Block = 14, Value = 15, Instruction = 16,
// VReg = 17, Continuation = 18, Task = 19, Result = 20, Diagnostic = 21,
// HostRequest = 22, Artifact = 23, Literal = 24, Sem = 25, ScopeEvent = 26.
//
// Tag-order note: the draft proposal text (`§5`) once sketched `Sem` = 24,
// `ScopeEvent` = 25, `Literal` = 26. The frozen assignment follows the
// authorizing freeze instruction and `RecordRef::wire_tag` instead
// (`Literal` = 24, `Sem` = 25, `ScopeEvent` = 26); the sketch order is not
// used anywhere here.
/// Frozen `/6` `RecordRef` wire tag for the `literals` family.
pub const RECORD_REF_TAG_LITERAL: u8 = 24;
/// Frozen `/6` `RecordRef` wire tag for the `sem` family.
pub const RECORD_REF_TAG_SEM: u8 = 25;
/// Frozen `/6` `RecordRef` wire tag for the `scope_events` family.
pub const RECORD_REF_TAG_SCOPE_EVENT: u8 = 26;
/// Number of frozen `/5` `RecordRef` variants (tags 0-23).
pub const RECORD_REF_FROZEN_COUNT: usize = 24;

/// Wire tag of a record reference, in `RecordRef` declaration order.
///
/// Delegates to [`RecordRef::wire_tag`] (single source of truth in `ids.rs`):
/// frozen `/5` tags 0-23 plus the `/6` appends `Literal` = 24, `Sem` = 25,
/// `ScopeEvent` = 26.
pub fn record_ref_tag(reference: RecordRef) -> u8 {
    reference.wire_tag()
}

/// Arena index carried by a record reference.
pub fn record_ref_index(reference: RecordRef) -> u32 {
    use RecordRef::*;
    match reference {
        Source(id) => id.index(),
        Span(id) => id.index(),
        Expansion(id) => id.index(),
        PpToken(id) => id.index(),
        Token(id) => id.index(),
        Name(id) => id.index(),
        Scope(id) => id.index(),
        Symbol(id) => id.index(),
        Type(id) => id.index(),
        Node(id) => id.index(),
        Const(id) => id.index(),
        Layout(id) => id.index(),
        Init(id) => id.index(),
        Function(id) => id.index(),
        Block(id) => id.index(),
        Value(id) => id.index(),
        Instruction(id) => id.index(),
        VReg(id) => id.index(),
        Continuation(id) => id.index(),
        Task(id) => id.index(),
        Result(id) => id.index(),
        Diagnostic(id) => id.index(),
        HostRequest(id) => id.index(),
        Artifact(id) => id.index(),
        Literal(id) => id.index(),
        Sem(id) => id.index(),
        ScopeEvent(id) => id.index(),
    }
}

fn push_record_ref(w: &mut Writer, reference: RecordRef) {
    w.u8(record_ref_tag(reference));
    w.u32(record_ref_index(reference));
}

fn push_payload(w: &mut Writer, payload: &crate::task::Payload) {
    w.u64(payload.refs.len() as u64);
    for reference in &payload.refs {
        push_record_ref(w, *reference);
    }
}

fn push_task_state(w: &mut Writer, state: &TaskState) {
    match state {
        TaskState::Ready => w.u8(0),
        TaskState::Running => w.u8(1),
        TaskState::Waiting(wait) => {
            w.u8(2);
            w.u64(wait.children.len() as u64);
            for child in &wait.children {
                w.u32(child.index());
            }
            match wait.host_request {
                Some(request) => {
                    w.u8(1);
                    w.u32(request.index());
                }
                None => w.u8(0),
            }
        }
        TaskState::Completed(result) => {
            w.u8(3);
            w.u32(result.index());
        }
        TaskState::Failed(diagnostic) => {
            w.u8(4);
            w.u32(diagnostic.index());
        }
    }
}

/// Number of frozen `ResultValue` variants on the wire (tags 0-4).
///
/// `DraftRecords` is explicitly *not* added: the wire set stays five variants.
pub const RESULT_VALUE_VARIANT_COUNT: usize = 5;

/// Wire tag of a result value, in `ResultValue` declaration order.
///
/// Frozen: `Empty` = 0, `Ack` = 1, `Record` = 2, `Records` = 3,
/// `Diagnostic` = 4. There is no tag 5; `DraftRecords` was considered and
/// rejected for this revision.
pub fn result_value_tag(value: &ResultValue) -> u8 {
    match value {
        ResultValue::Empty => 0,
        ResultValue::Ack => 1,
        ResultValue::Record(_) => 2,
        ResultValue::Records(_) => 3,
        ResultValue::Diagnostic(_) => 4,
    }
}

fn push_result_value(w: &mut Writer, value: &ResultValue) {
    w.u8(result_value_tag(value));
    match value {
        ResultValue::Empty | ResultValue::Ack => {}
        ResultValue::Record(reference) => {
            push_record_ref(w, *reference);
        }
        ResultValue::Records(references) => {
            w.u64(references.len() as u64);
            for reference in references {
                push_record_ref(w, *reference);
            }
        }
        ResultValue::Diagnostic(diagnostic) => {
            w.u32(diagnostic.index());
        }
    }
}

fn push_routing(w: &mut Writer, bus: &CompilerBus) {
    w.u64(bus.routing.len() as u64);
    for entry in bus.routing.iter() {
        w.u16(entry.kind.raw());
        w.u16(entry.chip.index());
        w.u16(entry.layer);
    }
}

fn push_manifests(w: &mut Writer, bus: &CompilerBus) {
    w.u64(bus.registrations.len() as u64);
    for manifest in bus.registrations.iter() {
        w.u16(manifest.id.index());
        w.str(manifest.chip_name);
        w.u8(manifest.group.raw());
        w.u64(manifest.task_kinds.len() as u64);
        for kind in &manifest.task_kinds {
            w.u16(kind.raw());
        }
        push_field_paths(w, &manifest.reads);
        push_field_paths(w, &manifest.writes);
        w.str(manifest.capability.name());
        w.str(manifest.backend_class.name());
        w.str(manifest.phase.name());
        w.bool(manifest.deterministic);
        w.u64(manifest.tests.len() as u64);
        for test in &manifest.tests {
            w.str(test);
        }
        w.u64(manifest.dependencies.len() as u64);
        for kind in &manifest.dependencies {
            w.u16(kind.raw());
        }
    }
}

fn push_field_paths(w: &mut Writer, paths: &[crate::manifest::FieldPath]) {
    w.u64(paths.len() as u64);
    for path in paths {
        w.str(path.store.name());
        w.str(path.field);
    }
}

fn push_kind_registry(w: &mut Writer, bus: &CompilerBus) {
    w.u64(bus.kinds.len() as u64);
    for entry in bus.kinds.iter() {
        w.u16(entry.kind.raw());
        w.str(entry.name);
        w.u8(entry.group.raw());
        w.u8(match entry.status {
            crate::task::KindStatus::Frozen => 0,
            crate::task::KindStatus::Reserved => 1,
            crate::task::KindStatus::GroupOwned => 2,
        });
    }
}

fn push_store_schema(w: &mut Writer, bus: &CompilerBus) {
    for store in StoreId::ALL {
        w.str(store.name());
        let fields = bus.schema.fields(store);
        w.u64(fields.len() as u64);
        for field in fields {
            w.str(field);
        }
    }
}

/// Frozen `/6` `Proposal` wire tag for `AppendRecords`.
///
/// Verified against `task.rs` and `Proposal::wire_tag`: the `/5` `Proposal`
/// declares `Enqueue` = 0, `Complete` = 1, `Fail` = 2, `AwaitHost` = 3,
/// `StorePatch` = 4, and the `/6` additions extend the enum in declaration
/// order `AppendRecords` = 5, `Progress` = 6, `AwaitChildren` = 7
/// (proposal §6.2/§12 item 2; S4).
pub const PROPOSAL_WIRE_TAG_APPEND_RECORDS: u8 = 5;
/// Frozen `/6` `Proposal` wire tag for `Progress`. See
/// [`PROPOSAL_WIRE_TAG_APPEND_RECORDS`] for the ordering rationale.
pub const PROPOSAL_WIRE_TAG_PROGRESS: u8 = 6;
/// Frozen `/6` `Proposal` wire tag for `AwaitChildren`. See
/// [`PROPOSAL_WIRE_TAG_APPEND_RECORDS`] for the ordering rationale.
pub const PROPOSAL_WIRE_TAG_AWAIT_CHILDREN: u8 = 7;

/// Wire tag of a proposal, in `Proposal` declaration order.
///
/// Delegates to [`Proposal::wire_tag`] (single source of truth in `task.rs`):
/// frozen `/5` tags 0-4 plus the `/6` additions `AppendRecords` = 5,
/// `Progress` = 6, `AwaitChildren` = 7.
pub fn proposal_wire_tag(proposal: &Proposal) -> u8 {
    proposal.wire_tag()
}

/// Canonical name of a proposal wire tag, in tag order.
///
/// Tags 0-4 name the frozen `/5` variants; tags 5-7 name the `/6` additions
/// (`append_records`, `progress`, `await_children`). Unknown tags fail with
/// [`CodecError::InvalidTag`].
pub fn proposal_wire_name(tag: u8) -> Result<&'static str, CodecError> {
    match tag {
        0 => Ok("enqueue"),
        1 => Ok("complete"),
        2 => Ok("fail"),
        3 => Ok("await_host"),
        4 => Ok("store_patch"),
        5 => Ok("append_records"),
        6 => Ok("progress"),
        7 => Ok("await_children"),
        other => Err(CodecError::InvalidTag(other)),
    }
}

fn push_wires(w: &mut Writer, bus: &CompilerBus) {
    w.u64(bus.wires.proposals.len() as u64);
    for tagged in &bus.wires.proposals {
        w.u16(tagged.chip.index());
        w.u32(tagged.task.index());
        w.u8(proposal_wire_tag(&tagged.proposal));
        match &tagged.proposal {
            crate::task::Proposal::Enqueue(draft) => {
                w.u16(draft.kind.raw());
                w.u16(draft.owner.index());
                match draft.parent {
                    Some(parent) => {
                        w.u8(1);
                        w.u32(parent.index());
                    }
                    None => w.u8(0),
                }
                match draft.continuation {
                    Some(continuation) => {
                        w.u8(1);
                        w.u32(continuation.index());
                    }
                    None => w.u8(0),
                }
                push_payload(w, &draft.payload);
            }
            crate::task::Proposal::Complete { task, value } => {
                w.u32(task.index());
                push_result_value(w, value);
            }
            crate::task::Proposal::Fail { task, diagnostic } => {
                w.u32(task.index());
                push_diagnostic_draft(w, diagnostic);
            }
            crate::task::Proposal::AwaitHost { task, request } => {
                w.u32(task.index());
                w.str(host_request_kind_name(request.kind));
                push_payload(w, &request.payload);
            }
            crate::task::Proposal::StorePatch(patch) => {
                w.u32(patch.task.index());
                w.u16(patch.owner.index());
                w.u64(patch.version);
                w.str(patch.store.name());
                w.str(patch.field);
                w.u8(match patch.op {
                    PatchOp::Append => 0,
                    PatchOp::Replace => 1,
                    PatchOp::Tombstone => 2,
                });
                push_optional_ref(w, patch.target);
                push_optional_ref(w, patch.value);
            }
            crate::task::Proposal::AppendRecords { task, batch } => {
                w.u32(task.index());
                // Draft bodies are owned by the records track (closed
                // `RecordDraft` enum with no settled module path yet; the name
                // is still unresolved in `task.rs`): only the frozen envelope
                // (owner task + draft count) encodes here. The integrator
                // appends per-draft bodies when the records track lands; wire
                // tag 5 stays.
                w.u64(batch.records.len() as u64);
            }
            crate::task::Proposal::Progress { task, ordinal } => {
                w.u32(task.index());
                w.u64(*ordinal);
            }
            crate::task::Proposal::AwaitChildren { task, children } => {
                w.u32(task.index());
                w.u64(children.len() as u64);
                // `children` iterates in stored order, which is deterministic.
                for child in children {
                    match child {
                        ChildRef::Committed(id) => {
                            w.u8(0);
                            w.u32(id.index());
                        }
                        ChildRef::OwnBatch(index) => {
                            w.u8(1);
                            w.u32(*index);
                        }
                    }
                }
            }
        }
    }
    w.bool(bus.wires.phase_advance_requested);
    match bus.wires.selected {
        Some(id) => {
            w.u8(1);
            w.u32(id.index());
        }
        None => w.u8(0),
    }
}

fn push_optional_ref(w: &mut Writer, reference: Option<RecordRef>) {
    match reference {
        Some(reference) => {
            w.u8(1);
            push_record_ref(w, reference);
        }
        None => w.u8(0),
    }
}

fn push_diagnostic_draft(w: &mut Writer, draft: &crate::diagnostic::DiagnosticDraft) {
    w.str(severity_name(draft.severity));
    w.str(draft.code.group.name());
    w.u16(draft.code.code);
    w.str(&draft.message);
    match draft.span {
        Some(span) => {
            w.u8(1);
            w.u32(span.index());
        }
        None => w.u8(0),
    }
    match draft.task {
        Some(task) => {
            w.u8(1);
            w.u32(task.index());
        }
        None => w.u8(0),
    }
}

/// A canonical snapshot of the whole observable bus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    bytes: Vec<u8>,
}

impl Snapshot {
    /// Capture the current bus state.
    pub fn capture(bus: &CompilerBus) -> Self {
        let mut w = Writer::new();
        w.str(crate::contract::CONTRACT_VERSION);
        // Control.
        w.u64(bus.control.tick);
        w.str(bus.control.job_state.name());
        w.str(bus.control.stage.name());
        match bus.control.selected {
            Some(id) => {
                w.u8(1);
                w.u32(id.index());
            }
            None => w.u8(0),
        }
        w.u64(bus.control.next_enqueue_ordinal);
        w.u64(bus.control.next_diagnostic_ordinal);
        w.bool(bus.control.budget_exhausted);
        w.bool(bus.control.cancel_requested);

        // Configuration + routing + manifests + registry + schema.
        w.raw(&config_hash(bus.config()));
        push_routing(&mut w, bus);
        push_manifests(&mut w, bus);
        push_kind_registry(&mut w, bus);
        push_store_schema(&mut w, bus);

        // Intern table (in ID order).
        w.u64(bus.intern.len() as u64);
        for (_id, bytes) in bus.intern.iter() {
            w.bytes(bytes);
        }

        // Queue state.
        w.u64(bus.tasks.ready.len() as u64);
        for id in &bus.tasks.ready {
            w.u32(id.index());
        }
        match bus.tasks.active {
            Some(id) => {
                w.u8(1);
                w.u32(id.index());
            }
            None => w.u8(0),
        }

        // Tasks.
        w.u64(bus.arenas.tasks.allocated() as u64);
        for (id, task) in bus.arenas.tasks.iter() {
            w.u32(id.index());
            w.u16(task.kind.raw());
            w.u16(task.owner.index());
            w.u64(task.enqueue_ordinal);
            w.u64(task.ready_tick);
            w.u32(task.progress_count);
            w.u64(task.progress_ordinal);
            match task.parent {
                Some(parent) => {
                    w.u8(1);
                    w.u32(parent.index());
                }
                None => w.u8(0),
            }
            match task.continuation {
                Some(continuation) => {
                    w.u8(1);
                    w.u32(continuation.index());
                }
                None => w.u8(0),
            }
            push_payload(&mut w, &task.payload);
            push_task_state(&mut w, &task.state);
        }

        // Results.
        w.u64(bus.arenas.results.allocated() as u64);
        for (id, result) in bus.arenas.results.iter() {
            push_result(&mut w, id, result);
        }

        // Diagnostics.
        w.u64(bus.arenas.diagnostics.allocated() as u64);
        for (id, diagnostic) in bus.arenas.diagnostics.iter() {
            push_diagnostic(&mut w, id, diagnostic);
        }

        // Host requests.
        w.u64(bus.arenas.host_requests.allocated() as u64);
        for (id, request) in bus.arenas.host_requests.iter() {
            w.u32(id.index());
            w.u32(request.task.index());
            w.str(host_request_kind_name(request.kind));
            push_payload(&mut w, &request.payload);
            w.bool(request.satisfied);
        }

        // Artifacts (`/10` rev-44 shape with source and map).
        w.u64(bus.arenas.artifacts.allocated() as u64);
        for (id, artifact) in bus.arenas.artifacts.iter() {
            w.u32(id.index());
            w.str(artifact_kind_name(artifact.kind));
            match artifact.source {
                Some(source) => {
                    w.u8(1);
                    w.u32(source.index());
                }
                None => w.u8(0),
            }
            w.bytes(&artifact.bytes);
            w.u64(artifact.raw_offsets.len() as u64);
            for offset in &artifact.raw_offsets {
                w.u64(*offset);
            }
        }

        // Sources: full bytes plus a hash recomputed from the bytes.
        w.u64(bus.arenas.sources.allocated() as u64);
        for (id, source) in bus.arenas.sources.iter() {
            w.u32(id.index());
            w.u32(source.name.index());
            w.bytes(&source.bytes);
            w.raw(&sha256(&source.bytes));
            w.raw(&source.content_hash);
        }

        // Spans and expansions. Offsets encode as fixed little-endian `u64`
        // (see `encode_span`): the `/5` bus record still holds `u32` offsets,
        // which widen here without loss; the `/6` record widens to `u64`.
        // The encoder below is the single source of truth so capture and
        // standalone bytes agree.
        w.u64(bus.arenas.spans.allocated() as u64);
        for (id, span) in bus.arenas.spans.iter() {
            w.u32(id.index());
            w.raw(&encode_span_record(span));
        }
        w.u64(bus.arenas.expansions.allocated() as u64);
        for (id, expansion) in bus.arenas.expansions.iter() {
            w.u32(id.index());
            match expansion.parent {
                Some(parent) => {
                    w.u8(1);
                    w.u32(parent.index());
                }
                None => w.u8(0),
            }
            w.u32(expansion.spelling.index());
            w.u32(expansion.expanded.index());
            w.u32(expansion.ordinal);
        }

        // Continuations (frozen `/6` nine-field shape; the encoder below is
        // the single source of truth so capture and standalone bytes agree).
        w.u64(bus.arenas.continuations.allocated() as u64);
        for (id, continuation) in bus.arenas.continuations.iter() {
            w.u32(id.index());
            w.raw(&encode_continuation(continuation));
        }

        // Wave 2 (`/11`) typed pp-tokens and C tokens: allocated count plus
        // per-record bodies in ascending ID order (same convention as the
        // typed literals/consts below).
        w.u64(bus.arenas.pp_tokens.allocated() as u64);
        for (id, token) in bus.arenas.pp_tokens.iter() {
            w.u32(id.index());
            w.raw(&encode_pp_token(token));
        }
        w.u64(bus.arenas.tokens.allocated() as u64);
        for (id, token) in bus.arenas.tokens.iter() {
            w.u32(id.index());
            w.raw(&encode_token(token));
        }
        // Reserved language stores: allocated count plus live IDs. Tombstone
        // positions are visible because removed IDs are absent from the live
        // list while `allocated` keeps counting, so trailing tombstones still
        // distinguish states. Record *bodies* remain reserved.
        push_reserved(
            &mut w,
            bus.arenas.scopes.allocated(),
            bus.arenas.scopes.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.symbols.allocated(),
            bus.arenas.symbols.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.types.allocated(),
            bus.arenas.types.live_ids(),
        );
        // Wave 2 (`/12`) typed AST nodes: allocated count plus per-record
        // bodies in ascending ID order (same convention as tokens above).
        w.u64(bus.arenas.nodes.allocated() as u64);
        for (id, node) in bus.arenas.nodes.iter() {
            w.u32(id.index());
            w.raw(&encode_node(node));
        }
        // Gate 1 (`/7`) typed constants: allocated count plus per-record
        // bodies in ascending ID order (tombstones stay visible as gaps,
        // same convention as the typed continuations above).
        w.u64(bus.arenas.consts.allocated() as u64);
        for (id, constant) in bus.arenas.consts.iter() {
            w.u32(id.index());
            w.raw(&encode_const(constant));
        }
        push_reserved(
            &mut w,
            bus.arenas.layouts.allocated(),
            bus.arenas.layouts.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.inits.allocated(),
            bus.arenas.inits.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.functions.allocated(),
            bus.arenas.functions.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.blocks.allocated(),
            bus.arenas.blocks.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.values.allocated(),
            bus.arenas.values.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.instructions.allocated(),
            bus.arenas.instructions.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.vregs.allocated(),
            bus.arenas.vregs.live_ids(),
        );
        // Gate 1 (`/7`) typed literals: allocated count plus per-record
        // bodies in ascending ID order (same convention as constants).
        w.u64(bus.arenas.literals.allocated() as u64);
        for (id, literal) in bus.arenas.literals.iter() {
            w.u32(id.index());
            w.raw(&encode_literal_record(literal));
        }
        push_reserved(
            &mut w,
            bus.arenas.sem.allocated(),
            bus.arenas.sem.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.scope_events.allocated(),
            bus.arenas.scope_events.live_ids(),
        );

        // In-flight dispatch set (ephemeral per-tick batch; empty at latch).
        w.u64(bus.tasks.in_flight.len() as u64);
        for id in &bus.tasks.in_flight {
            w.u32(id.index());
        }

        // Canonical bounded bus report (driver step-6 sole append site).
        w.u64(bus.report.len() as u64);
        for record in &bus.report {
            w.u64(record.dispatched.len() as u64);
            for id in &record.dispatched {
                w.u32(id.index());
            }
            w.u32(record.metrics.dispatched);
            match record.selected {
                Some(id) => {
                    w.u8(1);
                    w.u32(id.index());
                }
                None => w.u8(0),
            }
        }

        // Committed patches.
        w.u64(bus.patch_log.len() as u64);
        for patch in &bus.patch_log {
            w.u64(patch.ordinal);
            w.u32(patch.task.index());
            w.u16(patch.owner.index());
            w.str(patch.store.name());
            w.str(patch.field);
            w.u8(match patch.op {
                PatchOp::Append => 0,
                PatchOp::Replace => 1,
                PatchOp::Tombstone => 2,
            });
            match patch.target {
                Some(reference) => {
                    w.u8(1);
                    push_record_ref(&mut w, reference);
                }
                None => w.u8(0),
            }
            match patch.value {
                Some(reference) => {
                    w.u8(1);
                    push_record_ref(&mut w, reference);
                }
                None => w.u8(0),
            }
        }

        // Store versions.
        for store in StoreId::ALL {
            w.u64(bus.store_versions.get(store));
        }

        // Per-tick wires.
        push_wires(&mut w, bus);

        Self { bytes: w.finish() }
    }

    /// The canonical bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// SHA-256 of the canonical bytes.
    pub fn hash(&self) -> [u8; 32] {
        sha256(&self.bytes)
    }

    /// Hex SHA-256 of the canonical bytes.
    pub fn hex_hash(&self) -> String {
        hex32(&self.hash())
    }
}

fn push_reserved<I: crate::arena::ArenaId>(
    w: &mut Writer,
    allocated: u32,
    ids: impl Iterator<Item = I>,
) {
    w.u32(allocated);
    let ids: Vec<I> = ids.collect();
    w.u64(ids.len() as u64);
    for id in ids {
        w.u32(id.raw());
    }
}

fn push_result(w: &mut Writer, id: ResultId, result: &ResultRecord) {
    w.u32(id.index());
    w.u32(result.task.index());
    w.u16(result.kind.raw());
    w.u64(result.version);
    w.bool(result.consumed);
    push_result_value(w, &result.value);
}

fn push_diagnostic(w: &mut Writer, id: DiagnosticId, diagnostic: &DiagnosticRecord) {
    w.u32(id.index());
    w.u64(diagnostic.ordinal);
    w.str(severity_name(diagnostic.severity));
    w.str(diagnostic.code.group.name());
    w.u16(diagnostic.code.code);
    w.str(&diagnostic.message);
    match diagnostic.span {
        Some(span) => {
            w.u8(1);
            w.u32(span.index());
        }
        None => w.u8(0),
    }
    match diagnostic.task {
        Some(task) => {
            w.u8(1);
            w.u32(task.index());
        }
        None => w.u8(0),
    }
}

fn host_request_kind_name(kind: HostRequestKind) -> &'static str {
    match kind {
        HostRequestKind::ReadSource => "read_source",
        HostRequestKind::WriteArtifact => "write_artifact",
        HostRequestKind::InvokeToolchain => "invoke_toolchain",
        HostRequestKind::Cancel => "cancel",
    }
}

fn artifact_kind_name(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Normalized => "normalized",
        ArtifactKind::Spliced => "spliced",
        ArtifactKind::CommentFree => "comment_free",
        ArtifactKind::Preprocessed => "preprocessed",
        ArtifactKind::Assembly => "assembly",
        ArtifactKind::Object => "object",
        ArtifactKind::Snapshot => "snapshot",
        ArtifactKind::Trace => "trace",
    }
}

fn severity_name(severity: Severity) -> &'static str {
    severity.name()
}

/// A compact per-tick trace record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceRecord {
    /// Tick.
    pub tick: u64,
    /// Selected task.
    pub selected: Option<TaskId>,
    /// Outcome tag.
    pub outcome: &'static str,
    /// Resolution tag.
    pub resolution: &'static str,
    /// Ready-queue length after the step.
    pub ready_len: u32,
    /// Newly enqueued tasks.
    pub enqueued: u32,
    /// Completed tasks.
    pub completed: u32,
    /// Failed tasks.
    pub failed: u32,
}

/// A deterministic tick-by-tick trace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trace {
    records: Vec<TraceRecord>,
}

impl Trace {
    /// An empty trace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one propagation report.
    pub fn record(&mut self, bus: &CompilerBus, report: &TickReport) {
        let (outcome, resolution, enqueued, completed, failed) = match &report.outcome {
            TickOutcome::Idle => ("idle", "none", 0, 0, 0),
            TickOutcome::BudgetExhausted => ("budget_exhausted", "none", 0, 0, 0),
            TickOutcome::Cancelled => ("cancelled", "none", 0, 0, 0),
            TickOutcome::Executed {
                resolution, commit, ..
            } => (
                "executed",
                resolution_name(*resolution),
                commit.enqueued.len() as u32,
                commit.completed.len() as u32,
                commit.failed.len() as u32,
            ),
            TickOutcome::CommitFailed { .. } => ("commit_failed", "none", 0, 0, 1),
            TickOutcome::Joined { readied, failed } => (
                "joined",
                "none",
                0,
                readied.len() as u32,
                failed.len() as u32,
            ),
        };
        self.records.push(TraceRecord {
            tick: report.tick,
            selected: report.selected,
            outcome,
            resolution,
            ready_len: bus.tasks.ready.len() as u32,
            enqueued,
            completed,
            failed,
        });
    }

    /// Number of recorded ticks.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the trace is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The records.
    pub fn records(&self) -> &[TraceRecord] {
        &self.records
    }

    /// Canonical bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.str(crate::contract::CONTRACT_VERSION);
        w.u64(self.records.len() as u64);
        for record in &self.records {
            w.u64(record.tick);
            match record.selected {
                Some(id) => {
                    w.u8(1);
                    w.u32(id.index());
                }
                None => w.u8(0),
            }
            w.str(record.outcome);
            w.str(record.resolution);
            w.u32(record.ready_len);
            w.u32(record.enqueued);
            w.u32(record.completed);
            w.u32(record.failed);
        }
        w.finish()
    }

    /// SHA-256 of the canonical bytes.
    pub fn hash(&self) -> [u8; 32] {
        sha256(&self.to_bytes())
    }
}

fn resolution_name(resolution: Resolution) -> &'static str {
    match resolution {
        Resolution::Noop => "noop",
        Resolution::Unsupported => "unsupported",
        Resolution::Registered { .. } => "registered",
        Resolution::Unregistered => "unregistered",
    }
}

// ============================================================================
// Per-record canonical encoders (C05; `/6` working basis).
//
// Every encoder below writes a self-contained canonical byte string with the
// documented fixed layout, and every encoder has a fallible decoder so tests
// can assert decode + re-encode identity. All failures are checked
// [`CodecError`] values; there are no panics, no `todo!()`, and no
// `unimplemented!()` anywhere on these paths.
//
// Numeric-symbolic carve-out: enum identities are encoded as canonical NAMES
// (never raw discriminants) and field order is fixed per encoder, so symbolic
// evolution (appending variants) cannot silently reinterpret bytes.
// Magnitudes and widths — literal `value`/`spelling` bytes, `radix`, and any
// candidate-type bit width (none in M1; the M1 subset is the symbolic `Int`
// with no width) — are CARRIED in the snapshot bytes for lossless replay but
// are EXCLUDED from hash-relevant identity: `literal_identity_key` projects a
// literal to (kind name, suffix name, candidate-type name) only. Any future
// snapshot/hash comparison must use the symbolic key for the numeric domain
// and must not treat magnitude bytes as semantic identity.
// ============================================================================

/// Canonical bytes of one typed record reference: tag `u8` + index `u32` LE.
pub fn encode_record_ref(reference: RecordRef) -> Vec<u8> {
    let mut w = Writer::new();
    w.u8(record_ref_tag(reference));
    w.u32(record_ref_index(reference));
    w.finish()
}

/// Canonical bytes of one raw `(tag, index)` reference.
///
/// Accepts tags 0-26: the frozen `/5` tags 0-23 plus the `/6` working-basis
/// tags 24-26 ([`RECORD_REF_TAG_SEM`], [`RECORD_REF_TAG_SCOPE_EVENT`],
/// [`RECORD_REF_TAG_LITERAL`]). Anything higher is [`CodecError::InvalidTag`].
pub fn encode_record_ref_raw(tag: u8, index: u32) -> Result<Vec<u8>, CodecError> {
    if tag > RECORD_REF_TAG_SCOPE_EVENT {
        return Err(CodecError::InvalidTag(tag));
    }
    let mut w = Writer::new();
    w.u8(tag);
    w.u32(index);
    Ok(w.finish())
}

/// Decode one raw `(tag, index)` reference; consumes the whole input.
///
/// Tags 0-26 decode successfully (24-26 are the `/6` appends `Literal`, `Sem`,
/// `ScopeEvent`). Tags above 26 are [`CodecError::InvalidTag`].
pub fn decode_record_ref_raw(bytes: &[u8]) -> Result<(u8, u32), CodecError> {
    let mut r = Reader::new(bytes);
    let tag = r.u8()?;
    if tag > RECORD_REF_TAG_SCOPE_EVENT {
        return Err(CodecError::InvalidTag(tag));
    }
    let index = r.u32()?;
    r.finish()?;
    Ok((tag, index))
}

/// Decode one typed record reference (all 27 frozen tags).
///
/// Tags 0-23 are the frozen `/5` inventory; tags 24-26 are the `/6` appends
/// (`Literal` = 24, `Sem` = 25, `ScopeEvent` = 26) with explicit match arms
/// below. Tags above 26 are rejected by [`decode_record_ref_raw`] before this
/// match, so the trailing arm is unreachable-but-checked rather than a panic.
pub fn decode_record_ref(bytes: &[u8]) -> Result<RecordRef, CodecError> {
    let (tag, index) = decode_record_ref_raw(bytes)?;
    match tag {
        0 => Ok(RecordRef::Source(SourceId::from_index(index))),
        1 => Ok(RecordRef::Span(SpanId::from_index(index))),
        2 => Ok(RecordRef::Expansion(ExpansionId::from_index(index))),
        3 => Ok(RecordRef::PpToken(PpTokenId::from_index(index))),
        4 => Ok(RecordRef::Token(TokenId::from_index(index))),
        5 => Ok(RecordRef::Name(NameId::from_index(index))),
        6 => Ok(RecordRef::Scope(ScopeId::from_index(index))),
        7 => Ok(RecordRef::Symbol(SymbolId::from_index(index))),
        8 => Ok(RecordRef::Type(TypeId::from_index(index))),
        9 => Ok(RecordRef::Node(NodeId::from_index(index))),
        10 => Ok(RecordRef::Const(ConstId::from_index(index))),
        11 => Ok(RecordRef::Layout(LayoutId::from_index(index))),
        12 => Ok(RecordRef::Init(InitId::from_index(index))),
        13 => Ok(RecordRef::Function(FunctionId::from_index(index))),
        14 => Ok(RecordRef::Block(BlockId::from_index(index))),
        15 => Ok(RecordRef::Value(ValueId::from_index(index))),
        16 => Ok(RecordRef::Instruction(InstructionId::from_index(index))),
        17 => Ok(RecordRef::VReg(VRegId::from_index(index))),
        18 => Ok(RecordRef::Continuation(ContinuationId::from_index(index))),
        19 => Ok(RecordRef::Task(TaskId::from_index(index))),
        20 => Ok(RecordRef::Result(ResultId::from_index(index))),
        21 => Ok(RecordRef::Diagnostic(DiagnosticId::from_index(index))),
        22 => Ok(RecordRef::HostRequest(HostRequestId::from_index(index))),
        23 => Ok(RecordRef::Artifact(ArtifactId::from_index(index))),
        RECORD_REF_TAG_LITERAL => Ok(RecordRef::Literal(LiteralId::from_index(index))),
        RECORD_REF_TAG_SEM => Ok(RecordRef::Sem(SemId::from_index(index))),
        RECORD_REF_TAG_SCOPE_EVENT => Ok(RecordRef::ScopeEvent(ScopeEventId::from_index(index))),
        other => Err(CodecError::InvalidTag(other)),
    }
}

/// Canonical encoding of one span record.
///
/// Byte layout (fixed, 21 or 25 bytes): source `u32` LE | start `u64` LE |
/// end `u64` LE | expansion-present `u8` (`0`/`1`) | optional expansion
/// `u32` LE. Offsets are fixed little-endian `u64`: values above `u32::MAX`
/// round-trip, and `/5` `u32` offsets widen without loss.
pub fn encode_span(source: u32, start: u64, end: u64, expansion: Option<u32>) -> Vec<u8> {
    let mut w = Writer::new();
    w.u32(source);
    w.u64(start);
    w.u64(end);
    match expansion {
        Some(id) => {
            w.u8(1);
            w.u32(id);
        }
        None => w.u8(0),
    }
    w.finish()
}

/// Canonical bytes of a `/5` bus span record, widening `u32` offsets to `u64`.
pub fn encode_span_record(span: &SpanRecord) -> Vec<u8> {
    encode_span(
        span.source.index(),
        span.start,
        span.end,
        span.expansion.map(|id| id.index()),
    )
}

/// A decoded span record with `/6` `u64` offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedSpan {
    /// Owning source arena index.
    pub source: u32,
    /// Start byte offset.
    pub start: u64,
    /// End byte offset (exclusive).
    pub end: u64,
    /// Expansion arena index, if any.
    pub expansion: Option<u32>,
}

/// Decode one span record; consumes the whole input.
pub fn decode_span(bytes: &[u8]) -> Result<DecodedSpan, CodecError> {
    let mut r = Reader::new(bytes);
    let source = r.u32()?;
    let start = r.u64()?;
    let end = r.u64()?;
    let expansion = match r.u8()? {
        0 => None,
        1 => Some(r.u32()?),
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    r.finish()?;
    Ok(DecodedSpan {
        source,
        start,
        end,
        expansion,
    })
}

/// M1 candidate literal kind (rev 45 exact scope).
///
/// Only `Integer` is produced in M1; `Character` and `String` reserve the
/// vocabulary. Encoded by canonical name, never by discriminant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralKind {
    /// An integer literal.
    Integer,
    /// A character literal (reserved; not produced in M1).
    Character,
    /// A string literal (reserved; not produced in M1).
    String,
}

impl LiteralKind {
    /// Canonical encoding name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Character => "character",
            Self::String => "string",
        }
    }

    /// Parse a canonical name. Unknown names are
    /// [`CodecError::Unsupported`]: a future vocabulary addition, never silent
    /// corruption.
    pub fn parse(name: &str) -> Result<Self, CodecError> {
        match name {
            "integer" => Ok(Self::Integer),
            "character" => Ok(Self::Character),
            "string" => Ok(Self::String),
            _ => Err(CodecError::Unsupported("unknown literal kind name")),
        }
    }
}

/// M1 candidate literal suffix (rev 45 exact scope: only `None` produced).
///
/// Encoded by canonical name (`none`, `U`, `L`, `UL`, `LL`, `ULL`), never by
/// discriminant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralSuffix {
    /// No suffix.
    None,
    /// `U` suffix (reserved; not produced in M1).
    U,
    /// `L` suffix (reserved; not produced in M1).
    L,
    /// `UL` suffix (reserved; not produced in M1).
    Ul,
    /// `LL` suffix (reserved; not produced in M1).
    Ll,
    /// `ULL` suffix (reserved; not produced in M1).
    Ull,
}

impl LiteralSuffix {
    /// Canonical encoding name.
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::U => "U",
            Self::L => "L",
            Self::Ul => "UL",
            Self::Ll => "LL",
            Self::Ull => "ULL",
        }
    }

    /// Parse a canonical name. Unknown names are
    /// [`CodecError::Unsupported`].
    pub fn parse(name: &str) -> Result<Self, CodecError> {
        match name {
            "none" => Ok(Self::None),
            "U" => Ok(Self::U),
            "L" => Ok(Self::L),
            "UL" => Ok(Self::Ul),
            "LL" => Ok(Self::Ll),
            "ULL" => Ok(Self::Ull),
            _ => Err(CodecError::Unsupported("unknown literal suffix name")),
        }
    }
}

/// M1 closed one-member `LX08` candidate type: the symbolic target-independent
/// `Int` with no bit width (M1 literals 2 and 3 are `Int`).
///
/// The full member set and any numeric encodings remain open; future
/// categories append without reinterpretation. No silent `Int` defaulting
/// applies outside the exercised subset — other literals are explicit
/// unsupported/deferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lx08CandidateType {
    /// Target-independent integer candidate (M1 only).
    Int,
}

impl Lx08CandidateType {
    /// Canonical encoding name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Int => "Int",
        }
    }

    /// Parse a canonical name. Unknown names are
    /// [`CodecError::Unsupported`].
    pub fn parse(name: &str) -> Result<Self, CodecError> {
        match name {
            "Int" => Ok(Self::Int),
            _ => Err(CodecError::Unsupported("unknown candidate type name")),
        }
    }
}

/// Snapshot-local view of the candidate `LiteralRecord` (rev 45 exact ordered
/// fields).
///
/// Field order is frozen as declared: `token`, `kind`, `radix`, `suffix`,
/// `value`, `negative`, `spelling`, `candidate_type`. There is no bus arena
/// for literals yet, so this view is the encoder input until the T04 `/6`
/// co-freeze lands the committed record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiteralRecordView {
    /// Originating token arena index, if any (`None` for synthetic literals).
    pub token: Option<u32>,
    /// Literal kind.
    pub kind: LiteralKind,
    /// Numeric radix (`2`, `8`, `10`, `16`; M1 decimal only).
    pub radix: u8,
    /// Literal suffix.
    pub suffix: LiteralSuffix,
    /// Big-endian magnitude bytes (no sign, no width prefix).
    pub value: Vec<u8>,
    /// Whether the literal was preceded by `-` in the source.
    pub negative: bool,
    /// Original spelling bytes.
    pub spelling: Vec<u8>,
    /// Lexical candidate type (symbolic; no bit width in M1).
    pub candidate_type: Lx08CandidateType,
}

/// Canonical encoding of one candidate literal record.
///
/// Byte layout (fixed field order): token-present `u8` + optional token `u32`
/// LE | kind-name str | radix `u8` | suffix-name str | value bytes |
/// negative bool | spelling bytes | candidate-type-name str.
pub fn encode_literal(record: &LiteralRecordView) -> Vec<u8> {
    let mut w = Writer::new();
    match record.token {
        Some(token) => {
            w.u8(1);
            w.u32(token);
        }
        None => w.u8(0),
    }
    w.str(record.kind.name());
    w.u8(record.radix);
    w.str(record.suffix.name());
    w.bytes(&record.value);
    w.bool(record.negative);
    w.bytes(&record.spelling);
    w.str(record.candidate_type.name());
    w.finish()
}

/// Decode one candidate literal record; consumes the whole input.
pub fn decode_literal(bytes: &[u8]) -> Result<LiteralRecordView, CodecError> {
    let mut r = Reader::new(bytes);
    let token = match r.u8()? {
        0 => None,
        1 => Some(r.u32()?),
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    let kind_name = r.string()?;
    let kind = LiteralKind::parse(&kind_name)?;
    let radix = r.u8()?;
    let suffix_name = r.string()?;
    let suffix = LiteralSuffix::parse(&suffix_name)?;
    let value = r.bytes()?;
    let negative = r.bool()?;
    let spelling = r.bytes()?;
    let candidate_name = r.string()?;
    let candidate_type = Lx08CandidateType::parse(&candidate_name)?;
    r.finish()?;
    Ok(LiteralRecordView {
        token,
        kind,
        radix,
        suffix,
        value,
        negative,
        spelling,
        candidate_type,
    })
}

/// Canonical encoding of one committed [`LiteralRecord`].
///
/// The body layout reuses [`encode_literal`] through the view projection;
/// the token is its arena index.
pub fn encode_literal_record(record: &LiteralRecord) -> Vec<u8> {
    encode_literal(&LiteralRecordView {
        token: record.token.map(|token| token.index()),
        kind: record.kind,
        radix: record.radix,
        suffix: record.suffix,
        value: record.value.clone(),
        negative: record.negative,
        spelling: record.spelling.clone(),
        candidate_type: record.candidate_type,
    })
}

/// Decode one committed [`LiteralRecord`]; consumes the whole input.
pub fn decode_literal_record(bytes: &[u8]) -> Result<LiteralRecord, CodecError> {
    let view = decode_literal(bytes)?;
    Ok(LiteralRecord {
        token: view.token.map(crate::ids::TokenId::from_index),
        kind: view.kind,
        radix: view.radix,
        suffix: view.suffix,
        value: view.value,
        negative: view.negative,
        spelling: view.spelling,
        candidate_type: view.candidate_type,
    })
}

/// Canonical encoding of one committed [`ConstRecord`].
///
/// Byte layout (fixed field order): value bytes | negative bool.
pub fn encode_const(record: &ConstRecord) -> Vec<u8> {
    let mut w = Writer::new();
    w.bytes(&record.value);
    w.bool(record.negative);
    w.finish()
}

/// Decode one committed [`ConstRecord`]; consumes the whole input.
pub fn decode_const(bytes: &[u8]) -> Result<ConstRecord, CodecError> {
    let mut r = Reader::new(bytes);
    let value = r.bytes()?;
    let negative = r.bool()?;
    r.finish()?;
    Ok(ConstRecord { value, negative })
}

/// Name of a [`PpTokenKind`], in declaration order.
pub fn pp_token_kind_name(kind: PpTokenKind) -> &'static str {
    match kind {
        PpTokenKind::Identifier => "identifier",
        PpTokenKind::PpNumber => "pp_number",
        PpTokenKind::Punctuator => "punctuator",
        PpTokenKind::Eof => "eof",
    }
}

fn parse_pp_token_kind(name: &str) -> Option<PpTokenKind> {
    match name {
        "identifier" => Some(PpTokenKind::Identifier),
        "pp_number" => Some(PpTokenKind::PpNumber),
        "punctuator" => Some(PpTokenKind::Punctuator),
        "eof" => Some(PpTokenKind::Eof),
        _ => None,
    }
}

/// Name of a [`TokenKind`], in declaration order.
pub fn token_kind_name(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Keyword => "keyword",
        TokenKind::Identifier => "identifier",
        TokenKind::Punctuator => "punctuator",
        TokenKind::Integer => "integer",
        TokenKind::Eof => "eof",
    }
}

fn parse_token_kind(name: &str) -> Option<TokenKind> {
    match name {
        "keyword" => Some(TokenKind::Keyword),
        "identifier" => Some(TokenKind::Identifier),
        "punctuator" => Some(TokenKind::Punctuator),
        "integer" => Some(TokenKind::Integer),
        "eof" => Some(TokenKind::Eof),
        _ => None,
    }
}

/// Canonical encoding of one committed [`PpTokenRecord`].
///
/// Byte layout (fixed field order): kind-name str | span `u32` LE |
/// spelling bytes.
pub fn encode_pp_token(record: &PpTokenRecord) -> Vec<u8> {
    let mut w = Writer::new();
    w.str(pp_token_kind_name(record.kind));
    w.u32(record.span.index());
    w.bytes(&record.spelling);
    w.finish()
}

/// Decode one committed [`PpTokenRecord`]; consumes the whole input.
pub fn decode_pp_token(bytes: &[u8]) -> Result<PpTokenRecord, CodecError> {
    let mut r = Reader::new(bytes);
    let name = r.string()?;
    let kind =
        parse_pp_token_kind(&name).ok_or(CodecError::Unsupported("unknown pp-token kind"))?;
    let span = SpanId::from_index(r.u32()?);
    let spelling = r.bytes()?;
    r.finish()?;
    Ok(PpTokenRecord {
        kind,
        span,
        spelling,
    })
}

/// Canonical encoding of one committed [`TokenRecord`].
///
/// Byte layout (fixed field order): kind-name str | span `u32` LE |
/// name-present `u8` + optional name `u32` LE | pp-token `u32` LE.
pub fn encode_token(record: &TokenRecord) -> Vec<u8> {
    let mut w = Writer::new();
    w.str(token_kind_name(record.kind));
    w.u32(record.span.index());
    match record.name {
        Some(name) => {
            w.u8(1);
            w.u32(name.index());
        }
        None => w.u8(0),
    }
    w.u32(record.pp_token.index());
    w.finish()
}

/// Decode one committed [`TokenRecord`]; consumes the whole input.
pub fn decode_token(bytes: &[u8]) -> Result<TokenRecord, CodecError> {
    let mut r = Reader::new(bytes);
    let name = r.string()?;
    let kind = parse_token_kind(&name).ok_or(CodecError::Unsupported("unknown token kind"))?;
    let span = SpanId::from_index(r.u32()?);
    let token_name = match r.u8()? {
        0 => None,
        1 => Some(NameId::from_index(r.u32()?)),
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    let pp_token = PpTokenId::from_index(r.u32()?);
    r.finish()?;
    Ok(TokenRecord {
        kind,
        span,
        name: token_name,
        pp_token,
    })
}

/// Hash-relevant symbolic projection of a literal.
///
/// Encodes (kind name, suffix name, candidate-type name) only. Magnitudes
/// (`value`), spellings, radixes, and signs are carried in the snapshot bytes
/// for lossless replay but are excluded here: two literals that differ only in
/// magnitude bytes share an identity key. See the section header for the full
/// numeric-symbolic carve-out rationale.
pub fn literal_identity_key(record: &LiteralRecordView) -> Vec<u8> {
    let mut w = Writer::new();
    w.str(record.kind.name());
    w.str(record.suffix.name());
    w.str(record.candidate_type.name());
    w.finish()
}

/// Scope lifecycle event kind.
///
/// Encoded by canonical name (`enter`/`exit`), never by discriminant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeEventKind {
    /// Scope entry.
    Enter,
    /// Scope exit.
    Exit,
}

impl ScopeEventKind {
    /// Canonical encoding name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Enter => "enter",
            Self::Exit => "exit",
        }
    }

    /// Parse a canonical name. Unknown names are
    /// [`CodecError::Unsupported`].
    pub fn parse(name: &str) -> Result<Self, CodecError> {
        match name {
            "enter" => Ok(Self::Enter),
            "exit" => Ok(Self::Exit),
            _ => Err(CodecError::Unsupported("unknown scope event kind name")),
        }
    }
}

/// Snapshot-local view of the candidate `ScopeEventRecord`
/// (`{ scope, kind, at }`; append-only, no stored ordinal — order is the
/// `ScopeEventId` allocation order).
///
/// There is no bus arena for scope events yet, so this view is the encoder
/// input until the T06 `/6` co-freeze lands the committed record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeEventRecordView {
    /// Scope arena index.
    pub scope: u32,
    /// Event kind.
    pub kind: ScopeEventKind,
    /// Owner lexical node arena index (the committed TU node for file scope).
    pub at: u32,
}

/// Canonical encoding of one candidate scope-event record.
///
/// Byte layout (fixed): scope `u32` LE | kind-name str | `at` `u32` LE.
pub fn encode_scope_event(record: &ScopeEventRecordView) -> Vec<u8> {
    let mut w = Writer::new();
    w.u32(record.scope);
    w.str(record.kind.name());
    w.u32(record.at);
    w.finish()
}

/// Decode one candidate scope-event record; consumes the whole input.
pub fn decode_scope_event(bytes: &[u8]) -> Result<ScopeEventRecordView, CodecError> {
    let mut r = Reader::new(bytes);
    let scope = r.u32()?;
    let kind_name = r.string()?;
    let kind = ScopeEventKind::parse(&kind_name)?;
    let at = r.u32()?;
    r.finish()?;
    Ok(ScopeEventRecordView { scope, kind, at })
}

/// Canonical encoding of one artifact record: kind-name str + fragment bytes.
///
/// Name of a [`NodeKind`], in declaration order.
pub fn node_kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::TranslationUnit => "translation_unit",
        NodeKind::FunctionDefinition => "function_definition",
        NodeKind::Specifiers => "specifiers",
        NodeKind::Declarator => "declarator",
        NodeKind::Compound => "compound",
        NodeKind::Return => "return",
        NodeKind::BinaryAdd => "binary_add",
        NodeKind::IntLiteral => "int_literal",
    }
}

fn parse_node_kind(name: &str) -> Option<NodeKind> {
    match name {
        "translation_unit" => Some(NodeKind::TranslationUnit),
        "function_definition" => Some(NodeKind::FunctionDefinition),
        "specifiers" => Some(NodeKind::Specifiers),
        "declarator" => Some(NodeKind::Declarator),
        "compound" => Some(NodeKind::Compound),
        "return" => Some(NodeKind::Return),
        "binary_add" => Some(NodeKind::BinaryAdd),
        "int_literal" => Some(NodeKind::IntLiteral),
        _ => None,
    }
}

/// Canonical encoding of one committed [`NodeRecord`].
///
/// Byte layout (fixed field order): kind-name str | parent-present `u8` +
/// optional parent `u32` LE | children-count `u64` LE + child `u32` LE each |
/// first-token `u32` LE | last-token `u32` LE | name-present `u8` + optional
/// name `u32` LE | literal-present `u8` + optional literal `u32` LE.
pub fn encode_node(record: &NodeRecord) -> Vec<u8> {
    let mut w = Writer::new();
    w.str(node_kind_name(record.kind));
    match record.parent {
        Some(parent) => {
            w.u8(1);
            w.u32(parent.index());
        }
        None => w.u8(0),
    }
    w.u64(record.children.len() as u64);
    for child in &record.children {
        w.u32(child.index());
    }
    w.u32(record.first_token.index());
    w.u32(record.last_token.index());
    match record.name {
        Some(name) => {
            w.u8(1);
            w.u32(name.index());
        }
        None => w.u8(0),
    }
    match record.literal {
        Some(literal) => {
            w.u8(1);
            w.u32(literal.index());
        }
        None => w.u8(0),
    }
    w.finish()
}

/// Decode one committed [`NodeRecord`]; consumes the whole input.
pub fn decode_node(bytes: &[u8]) -> Result<NodeRecord, CodecError> {
    let mut r = Reader::new(bytes);
    let name = r.string()?;
    let kind = parse_node_kind(&name).ok_or(CodecError::Unsupported("unknown node kind"))?;
    let parent = match r.u8()? {
        0 => None,
        1 => Some(NodeId::from_index(r.u32()?)),
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    let child_count = r.u64()? as usize;
    let mut children = Vec::with_capacity(child_count);
    for _ in 0..child_count {
        children.push(NodeId::from_index(r.u32()?));
    }
    let first_token = TokenId::from_index(r.u32()?);
    let last_token = TokenId::from_index(r.u32()?);
    let node_name = match r.u8()? {
        0 => None,
        1 => Some(NameId::from_index(r.u32()?)),
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    let literal = match r.u8()? {
        0 => None,
        1 => Some(LiteralId::from_index(r.u32()?)),
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    r.finish()?;
    Ok(NodeRecord {
        kind,
        parent,
        children,
        first_token,
        last_token,
        name: node_name,
        literal,
    })
}

/// Canonical encoding of a `/10` artifact: kind name, optional source index,
/// bytes, then the `raw_offsets` map.
pub fn encode_artifact(
    kind: ArtifactKind,
    source: Option<SourceId>,
    bytes: &[u8],
    raw_offsets: &[u64],
) -> Vec<u8> {
    let mut w = Writer::new();
    w.str(artifact_kind_name(kind));
    match source {
        Some(id) => {
            w.u8(1);
            w.u32(id.index());
        }
        None => w.u8(0),
    }
    w.bytes(bytes);
    w.u64(raw_offsets.len() as u64);
    for offset in raw_offsets {
        w.u64(*offset);
    }
    w.finish()
}

/// Canonical encoding of a bus artifact record. Mirrors `Snapshot::capture`.
pub fn encode_artifact_record(record: &ArtifactRecord) -> Vec<u8> {
    encode_artifact(
        record.kind,
        record.source,
        &record.bytes,
        &record.raw_offsets,
    )
}

fn parse_artifact_kind(name: &str) -> Option<ArtifactKind> {
    match name {
        "normalized" => Some(ArtifactKind::Normalized),
        "spliced" => Some(ArtifactKind::Spliced),
        "comment_free" => Some(ArtifactKind::CommentFree),
        "preprocessed" => Some(ArtifactKind::Preprocessed),
        "assembly" => Some(ArtifactKind::Assembly),
        "object" => Some(ArtifactKind::Object),
        "snapshot" => Some(ArtifactKind::Snapshot),
        "trace" => Some(ArtifactKind::Trace),
        _ => None,
    }
}

/// Decode one artifact record; consumes the whole input.
///
/// Unknown kind names are [`CodecError::Unsupported`]: a future `ArtifactKind`
/// addition, never silent corruption.
pub fn decode_artifact(bytes: &[u8]) -> Result<ArtifactRecord, CodecError> {
    let mut r = Reader::new(bytes);
    let name = r.string()?;
    let kind =
        parse_artifact_kind(&name).ok_or(CodecError::Unsupported("unknown artifact kind name"))?;
    let source = match r.u8()? {
        0 => None,
        1 => Some(SourceId::from_index(r.u32()?)),
        _ => return Err(CodecError::Unsupported("unknown artifact source tag")),
    };
    let data = r.bytes()?;
    let map_len = r.u64()? as usize;
    let mut raw_offsets = Vec::with_capacity(map_len);
    for _ in 0..map_len {
        raw_offsets.push(r.u64()?);
    }
    r.finish()?;
    Ok(ArtifactRecord {
        kind,
        source,
        bytes: data,
        raw_offsets,
    })
}

/// Canonical encoding of one continuation record (frozen `/6` nine-field shape).
///
/// Byte layout (fixed field order):
/// production-kind `u16` LE (raw task-kind code), cursor `u32` LE (token
/// arena index), context `u8` (`ParseContext` ordinal 0-9), binding-power
/// `u16` LE, scope-present `u8` + optional scope `u32` LE, parent-present
/// `u8` + optional parent `u32` LE (node arena index),
/// partial-children-count `u64` LE + child `u32` LE each (contiguous
/// ordinals), next-child-ordinal `u32` LE, previous-present `u8` + optional
/// previous `u32` LE (continuation arena index).
/// `Snapshot::capture` reuses this encoder verbatim.
pub fn encode_continuation(record: &ContinuationRecord) -> Vec<u8> {
    let mut w = Writer::new();
    w.u16(record.production.raw());
    w.u32(record.cursor.index());
    w.u8(record.context.ordinal());
    w.u16(record.binding_power);
    match record.scope {
        Some(id) => {
            w.u8(1);
            w.u32(id.index());
        }
        None => w.u8(0),
    }
    match record.parent {
        Some(id) => {
            w.u8(1);
            w.u32(id.index());
        }
        None => w.u8(0),
    }
    // `partial_children` iterates in stored contiguous-ordinal order, which is
    // deterministic.
    w.u64(record.partial_children.len() as u64);
    for child in &record.partial_children {
        w.u32(child.index());
    }
    w.u32(record.next_child_ordinal);
    match record.previous {
        Some(id) => {
            w.u8(1);
            w.u32(id.index());
        }
        None => w.u8(0),
    }
    w.finish()
}

/// Decode one continuation record into the frozen `/6` shape; consumes the
/// whole input.
///
/// The production kind is rebuilt from its raw `u16` code (4-bit group +
/// 12-bit local code) with checked errors: an unregistered group byte is
/// [`CodecError::InvalidTag`]. The context ordinal maps to the closed
/// 10-member [`ParseContext`] vocabulary; anything else is
/// [`CodecError::InvalidTag`].
pub fn decode_continuation(bytes: &[u8]) -> Result<ContinuationRecord, CodecError> {
    let mut r = Reader::new(bytes);
    let production_raw = r.u16()?;
    let group_byte = (production_raw >> TaskKind::LOCAL_BITS) as u8;
    let group = TaskGroup::from_raw(group_byte).ok_or(CodecError::InvalidTag(group_byte))?;
    let production = TaskKind::new(group, production_raw & TaskKind::LOCAL_MASK)
        .ok_or(CodecError::InvalidTag(group_byte))?;
    let cursor = TokenId::from_index(r.u32()?);
    let context = match r.u8()? {
        0 => ParseContext::TranslationUnit,
        1 => ParseContext::ExternalDecl,
        2 => ParseContext::Specifier,
        3 => ParseContext::Declarator,
        4 => ParseContext::ParameterList,
        5 => ParseContext::Block,
        6 => ParseContext::Expression,
        7 => ParseContext::Assignment,
        8 => ParseContext::Unary,
        9 => ParseContext::Primary,
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    let binding_power = r.u16()?;
    let scope = match r.u8()? {
        0 => None,
        1 => Some(ScopeId::from_index(r.u32()?)),
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    let parent = match r.u8()? {
        0 => None,
        1 => Some(NodeId::from_index(r.u32()?)),
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    let child_count = usize::try_from(r.u64()?).map_err(|_| CodecError::Truncated)?;
    // Bound the reservation by the bytes actually present so a corrupt count
    // cannot force a huge allocation; the loop below still fails fast with
    // `Truncated` on the first missing entry.
    let mut partial_children = Vec::with_capacity(child_count.min(r.remaining() / 4));
    for _ in 0..child_count {
        partial_children.push(NodeId::from_index(r.u32()?));
    }
    let next_child_ordinal = r.u32()?;
    let previous = match r.u8()? {
        0 => None,
        1 => Some(ContinuationId::from_index(r.u32()?)),
        tag => return Err(CodecError::InvalidTag(tag)),
    };
    r.finish()?;
    Ok(ContinuationRecord {
        production,
        cursor,
        context,
        binding_power,
        scope,
        parent,
        partial_children,
        next_child_ordinal,
        previous,
    })
}
