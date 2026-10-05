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
// owning task group freezes them.
// ============================================================================

use crate::bus::{ArtifactKind, CompilerBus};
use crate::codec::{hex32, sha256, Writer};
use crate::diagnostic::{DiagnosticRecord, Severity};
use crate::ids::{DiagnosticId, RecordRef, ResultId, TaskId};
use crate::routing::{Resolution, TickOutcome, TickReport};
use crate::target::{CompilerConfig, VerificationState};
use crate::task::{HostRequestKind, PatchOp, ResultRecord, ResultValue, StoreId, TaskState};

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

fn push_record_ref(w: &mut Writer, reference: RecordRef) {
    use RecordRef::*;
    let (tag, index) = match reference {
        Source(id) => (0u8, id.index()),
        Span(id) => (1, id.index()),
        Expansion(id) => (2, id.index()),
        PpToken(id) => (3, id.index()),
        Token(id) => (4, id.index()),
        Name(id) => (5, id.index()),
        Scope(id) => (6, id.index()),
        Symbol(id) => (7, id.index()),
        Type(id) => (8, id.index()),
        Node(id) => (9, id.index()),
        Const(id) => (10, id.index()),
        Layout(id) => (11, id.index()),
        Init(id) => (12, id.index()),
        Function(id) => (13, id.index()),
        Block(id) => (14, id.index()),
        Value(id) => (15, id.index()),
        Instruction(id) => (16, id.index()),
        VReg(id) => (17, id.index()),
        Continuation(id) => (18, id.index()),
        Task(id) => (19, id.index()),
        Result(id) => (20, id.index()),
        Diagnostic(id) => (21, id.index()),
        HostRequest(id) => (22, id.index()),
        Artifact(id) => (23, id.index()),
    };
    w.u8(tag);
    w.u32(index);
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

fn push_result_value(w: &mut Writer, value: &ResultValue) {
    match value {
        ResultValue::Empty => w.u8(0),
        ResultValue::Ack => w.u8(1),
        ResultValue::Record(reference) => {
            w.u8(2);
            push_record_ref(w, *reference);
        }
        ResultValue::Records(references) => {
            w.u8(3);
            w.u64(references.len() as u64);
            for reference in references {
                push_record_ref(w, *reference);
            }
        }
        ResultValue::Diagnostic(diagnostic) => {
            w.u8(4);
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

fn push_wires(w: &mut Writer, bus: &CompilerBus) {
    w.u64(bus.wires.proposals.len() as u64);
    for tagged in &bus.wires.proposals {
        w.u16(tagged.chip.index());
        w.u32(tagged.task.index());
        match &tagged.proposal {
            crate::task::Proposal::Enqueue(draft) => {
                w.u8(0);
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
                w.u8(1);
                w.u32(task.index());
                push_result_value(w, value);
            }
            crate::task::Proposal::Fail { task, diagnostic } => {
                w.u8(2);
                w.u32(task.index());
                push_diagnostic_draft(w, diagnostic);
            }
            crate::task::Proposal::AwaitHost { task, request } => {
                w.u8(3);
                w.u32(task.index());
                w.str(host_request_kind_name(request.kind));
                push_payload(w, &request.payload);
            }
            crate::task::Proposal::StorePatch(patch) => {
                w.u8(4);
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

        // Artifacts.
        w.u64(bus.arenas.artifacts.allocated() as u64);
        for (id, artifact) in bus.arenas.artifacts.iter() {
            w.u32(id.index());
            w.str(artifact_kind_name(artifact.kind));
            w.bytes(&artifact.bytes);
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

        // Spans and expansions.
        w.u64(bus.arenas.spans.allocated() as u64);
        for (id, span) in bus.arenas.spans.iter() {
            w.u32(id.index());
            w.u32(span.source.index());
            w.u32(span.start);
            w.u32(span.end);
            match span.expansion {
                Some(expansion) => {
                    w.u8(1);
                    w.u32(expansion.index());
                }
                None => w.u8(0),
            }
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

        // Continuations.
        w.u64(bus.arenas.continuations.allocated() as u64);
        for (id, continuation) in bus.arenas.continuations.iter() {
            w.u32(id.index());
            w.u16(continuation.resume_kind.raw());
            w.u64(continuation.awaited.len() as u64);
            for child in &continuation.awaited {
                w.u32(child.index());
            }
            match continuation.scope {
                Some(scope) => {
                    w.u8(1);
                    w.u32(scope.index());
                }
                None => w.u8(0),
            }
        }

        // Reserved language stores: allocated count plus live IDs. Tombstone
        // positions are visible because removed IDs are absent from the live
        // list while `allocated` keeps counting, so trailing tombstones still
        // distinguish states. Record *bodies* remain reserved.
        push_reserved(
            &mut w,
            bus.arenas.pp_tokens.allocated(),
            bus.arenas.pp_tokens.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.tokens.allocated(),
            bus.arenas.tokens.live_ids(),
        );
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
        push_reserved(
            &mut w,
            bus.arenas.nodes.allocated(),
            bus.arenas.nodes.live_ids(),
        );
        push_reserved(
            &mut w,
            bus.arenas.consts.allocated(),
            bus.arenas.consts.live_ids(),
        );
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
