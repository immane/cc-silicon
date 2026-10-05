// ============================================================================
// bus.rs — the CompilerBus storage profile (T01 C01/C03)
//
// The approved CPU storage extension keeps every piece of semantic state in a
// single `CompilerBus`: append-only Vec arenas addressed by stable newtype IDs,
// an intern table, persistent queues/results, diagnostics, routing, and a
// per-tick wire bundle. Registers persist across ticks; wires reset at tick
// start.
//
// The bus never uses `Rc`/`Arc`/`RefCell`/`Mutex`, never treats an address as
// an ID, and never lets map iteration decide ordering. Every configured
// resource bound is enforced through a structured [`LimitError`] before any
// mutation.
// ============================================================================

use crate::arena::{ReservedArena, TypedArena};
use crate::diagnostic::DiagnosticRecord;
use crate::ids::{
    ArtifactId, BlockId, ConstId, ContinuationId, DiagnosticId, ExpansionId, FunctionId,
    HostRequestId, InitId, InstructionId, LayoutId, LiteralId, NameId, NodeId, PpTokenId, ResultId,
    ScopeEventId, ScopeId, SemId, SourceId, SpanId, SymbolId, TaskId, TokenId, TypeId, VRegId,
    ValueId,
};
use crate::intern::InternTable;
use crate::limits::{LimitError, Limits};
use crate::manifest::ManifestRegistry;
use crate::routing::RoutingTable;
use crate::target::CompilerConfig;
use crate::task::{
    ContinuationRecord, HostRequestRecord, ResultRecord, StoreId, Task, TaskDraft, TaskKind,
    TaskKindRegistry,
};
use cc_silicon::Bus;

/// A source file record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRecord {
    /// Interned file name.
    pub name: NameId,
    /// Raw file bytes.
    pub bytes: Vec<u8>,
    /// SHA-256 of the raw bytes.
    pub content_hash: [u8; 32],
}

/// A source span record.
///
/// Offsets are `u64` half-open raw-byte offsets bounded by
/// `limits.max_source_bytes` (the `/6` widening of the `/5` `u32` offsets;
/// T03 finding 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanRecord {
    /// Owning source.
    pub source: SourceId,
    /// Start byte offset.
    pub start: u64,
    /// End byte offset (exclusive).
    pub end: u64,
    /// Expansion that produced this span, if any.
    pub expansion: Option<ExpansionId>,
}

/// A macro expansion provenance record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpansionRecord {
    /// Parent expansion.
    pub parent: Option<ExpansionId>,
    /// Spelling span.
    pub spelling: SpanId,
    /// Expanded span.
    pub expanded: SpanId,
    /// Deterministic expansion ordinal.
    pub ordinal: u32,
}

/// Kind of output artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    /// Preprocessed source.
    Preprocessed,
    /// Generated assembly.
    Assembly,
    /// Object file.
    Object,
    /// A deterministic snapshot.
    Snapshot,
    /// A deterministic trace.
    Trace,
}

/// An output artifact fragment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactRecord {
    /// Artifact kind.
    pub kind: ArtifactKind,
    /// Fragment bytes.
    pub bytes: Vec<u8>,
}

/// Every typed arena owned by the compiler bus.
///
/// Foundation stores carry real record types. Language stores whose record
/// schema is owned by a later task group use [`ReservedArena`] so their stable
/// IDs exist without fabricating language semantics.
#[derive(Default)]
pub struct Arenas {
    /// Source files.
    pub sources: TypedArena<SourceId, SourceRecord>,
    /// Source spans.
    pub spans: TypedArena<SpanId, SpanRecord>,
    /// Macro expansion provenance.
    pub expansions: TypedArena<ExpansionId, ExpansionRecord>,
    /// Preprocessing tokens (schema owned by T03).
    pub pp_tokens: ReservedArena<PpTokenId>,
    /// C tokens (schema owned by T04).
    pub tokens: ReservedArena<TokenId>,
    /// Scopes (schema owned by T06).
    pub scopes: ReservedArena<ScopeId>,
    /// Scope lifecycle events (schema owned by T06).
    ///
    /// A [`ReservedArena`] (stable IDs only): the T06 owner replaces this
    /// with a real typed arena when it freezes the `ScopeEventRecord` schema.
    pub scope_events: ReservedArena<ScopeEventId>,
    /// Symbols (schema owned by T06).
    pub symbols: ReservedArena<SymbolId>,
    /// Canonical types (schema owned by T06).
    pub types: ReservedArena<TypeId>,
    /// Semantic facts, one per checked node (schema owned by T07).
    ///
    /// A [`ReservedArena`] (stable IDs only): the T07 owner replaces this
    /// with a real typed arena when it freezes the `SemRecord` schema.
    pub sem: ReservedArena<SemId>,
    /// AST nodes (schema owned by T05).
    pub nodes: ReservedArena<NodeId>,
    /// T04-owned decoded literals (raw lexical facts; record schema owned by
    /// T04).
    ///
    /// A [`ReservedArena`] (stable IDs only, no fabricated record): the T04
    /// owner replaces this with a real typed arena when it freezes the
    /// `LiteralRecord` schema. (The `ids.rs` working-basis note names a
    /// `TypedArena`; that replacement is the T04 freeze, not this bus change.)
    pub literals: ReservedArena<LiteralId>,
    /// Constants (schema owned by T08).
    pub consts: ReservedArena<ConstId>,
    /// Layout descriptors (schema owned by T08).
    pub layouts: ReservedArena<LayoutId>,
    /// Initialization plans (schema owned by T08).
    pub inits: ReservedArena<InitId>,
    /// IR functions (schema owned by T09).
    pub functions: ReservedArena<FunctionId>,
    /// IR basic blocks (schema owned by T09).
    pub blocks: ReservedArena<BlockId>,
    /// IR values (schema owned by T09).
    pub values: ReservedArena<ValueId>,
    /// IR instructions (schema owned by T09).
    pub instructions: ReservedArena<InstructionId>,
    /// Virtual registers (schema owned by T11).
    pub vregs: ReservedArena<VRegId>,
    /// Continuations (mechanical resume state).
    pub continuations: TypedArena<ContinuationId, ContinuationRecord>,
    /// Tasks.
    pub tasks: TypedArena<TaskId, Task>,
    /// Committed results.
    pub results: TypedArena<ResultId, ResultRecord>,
    /// Committed diagnostics.
    pub diagnostics: TypedArena<DiagnosticId, DiagnosticRecord>,
    /// Host requests.
    pub host_requests: TypedArena<HostRequestId, HostRequestRecord>,
    /// Output artifacts.
    pub artifacts: TypedArena<ArtifactId, ArtifactRecord>,
}

impl Arenas {
    /// Create an empty arena set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total allocated slots across every arena (never reused).
    pub fn allocated_total(&self) -> u64 {
        self.sources.allocated() as u64
            + self.spans.allocated() as u64
            + self.expansions.allocated() as u64
            + self.pp_tokens.allocated() as u64
            + self.tokens.allocated() as u64
            + self.scopes.allocated() as u64
            + self.scope_events.allocated() as u64
            + self.symbols.allocated() as u64
            + self.types.allocated() as u64
            + self.sem.allocated() as u64
            + self.nodes.allocated() as u64
            + self.literals.allocated() as u64
            + self.consts.allocated() as u64
            + self.layouts.allocated() as u64
            + self.inits.allocated() as u64
            + self.functions.allocated() as u64
            + self.blocks.allocated() as u64
            + self.values.allocated() as u64
            + self.instructions.allocated() as u64
            + self.vregs.allocated() as u64
            + self.continuations.allocated() as u64
            + self.tasks.allocated() as u64
            + self.results.allocated() as u64
            + self.diagnostics.allocated() as u64
            + self.host_requests.allocated() as u64
            + self.artifacts.allocated() as u64
    }
}

/// Job lifecycle state.
///
/// The `Idle` default is the pre-job state. The CT01 job-start transition
/// (`Idle` → `Running`) is control-chip work owned by T02 and is NOT performed
/// here; this type only records the state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobState {
    /// No job has been started.
    Idle,
    /// A job is running.
    Running,
    /// A job finished successfully.
    Finished,
    /// A job finished with errors.
    Failed,
}

impl JobState {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Finished => "finished",
            Self::Failed => "failed",
        }
    }
}

/// Current compiler stage. Phase advance is a control decision owned by T02;
/// this type only records the stage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    /// Job setup.
    Init,
    /// Preprocessing.
    Preprocess,
    /// Lexical analysis.
    Lex,
    /// Parsing.
    Parse,
    /// Symbols and types.
    SymbolsTypes,
    /// Semantic analysis.
    Semantic,
    /// Layout/constants/initialization.
    LayoutInit,
    /// IR lowering.
    Ir,
    /// Optimization.
    Optimize,
    /// Target code generation.
    Target,
    /// Finished.
    Done,
}

impl Stage {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Preprocess => "preprocess",
            Self::Lex => "lex",
            Self::Parse => "parse",
            Self::SymbolsTypes => "symbols_types",
            Self::Semantic => "semantic",
            Self::LayoutInit => "layout_init",
            Self::Ir => "ir",
            Self::Optimize => "optimize",
            Self::Target => "target",
            Self::Done => "done",
        }
    }
}

/// Persistent control registers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Control {
    /// Lamport tick.
    pub tick: u64,
    /// Job lifecycle state.
    pub job_state: JobState,
    /// Current stage.
    pub stage: Stage,
    /// The task selected this tick, if any.
    pub selected: Option<TaskId>,
    /// Enqueue ordinal counter.
    pub next_enqueue_ordinal: u64,
    /// Diagnostic ordinal counter.
    pub next_diagnostic_ordinal: u64,
    /// Whether the tick/task budget was reported exhausted.
    pub budget_exhausted: bool,
    /// Whether the host requested cancellation (consumed by T02 CT13).
    pub cancel_requested: bool,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            tick: 0,
            job_state: JobState::Idle,
            stage: Stage::Init,
            selected: None,
            next_enqueue_ordinal: 0,
            next_diagnostic_ordinal: 0,
            budget_exhausted: false,
            cancel_requested: false,
        }
    }
}

/// Persistent task queue state. Task records live in `Arenas::tasks`.
///
/// Result consumption is tracked on each [`crate::task::ResultRecord::consumed`]
/// flag, not duplicated here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskStore {
    /// Ready queue in enqueue order.
    pub ready: Vec<TaskId>,
    /// Task selected this tick.
    pub active: Option<TaskId>,
    /// Ephemeral per-tick scheduler batch: tasks dispatched this tick.
    ///
    /// Populated by the dispatcher's distinct pre-worker `Ready → Running`
    /// mutation and cleared at latch only after every dispatched task has a
    /// terminal/`Waiting`/`Progress` outcome (or the H6 bounded recovery).
    /// Clearing this set is not itself a transition and never clears a task's
    /// `Running` state. `reset_wires` at tick start is a wire reset only and
    /// is not an in-flight lifecycle step; a next-tick-start clear is rejected
    /// as latch-residual-incompatible. The exact clear owner/order is a T01
    /// decision. `Waiting` tasks are never in-flight.
    pub in_flight: Vec<TaskId>,
}

/// Minimal per-tick dispatch metrics (`/6` working basis).
///
/// This is the deferred-minimal carrier: dispatched counts only. The full
/// `PipelineMetrics` shape (per-stage counts, fairness cursor, backpressure
/// counters) is deferred to Part B and is NOT defined here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TickMetrics {
    /// Number of tasks dispatched this tick.
    pub dispatched: u32,
}

/// One tick's dispatch record for the canonical bounded bus report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickRecord {
    /// Tasks dispatched this tick, in dispatch order (canonical).
    pub dispatched: Vec<TaskId>,
    /// Minimal per-tick metrics (dispatched counts only; see [`TickMetrics`]).
    pub metrics: TickMetrics,
    /// Quota-1 projection of the dispatch (derived view of `dispatched`).
    pub selected: Option<TaskId>,
}

/// Structured bus-report failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportCapacityError {
    /// The bounded report (`max_ticks + 1` records) is full.
    ReportFull {
        /// Configured bound (`max_ticks + 1`).
        limit: u64,
        /// Records that would result.
        requested: u64,
    },
}

impl std::fmt::Display for ReportCapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReportFull { limit, requested } => {
                write!(f, "bus report {requested} exceeds limit {limit}")
            }
        }
    }
}

impl std::error::Error for ReportCapacityError {}

/// A worker proposal tagged with its producing chip and task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaggedProposal {
    /// Producing chip.
    pub chip: crate::ids::ChipId,
    /// Producing/selected task.
    pub task: TaskId,
    /// The proposal.
    pub proposal: crate::task::Proposal,
}

/// Per-tick wire bundle. Reset to default at tick start.
#[derive(Clone, Debug, Default)]
pub struct CompilerWires {
    /// Proposals recorded this tick, in emission order.
    pub proposals: Vec<TaggedProposal>,
    /// Task selected this tick.
    pub selected: Option<TaskId>,
    /// A phase advance was requested this tick.
    pub phase_advance_requested: bool,
}

/// A host response frozen into pins for one tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostResponse {
    /// Request being answered.
    pub request: HostRequestId,
    /// Response bytes.
    pub bytes: Vec<u8>,
}

/// Frozen external inputs for one tick.
#[derive(Clone, Debug, Default)]
pub struct CompilerPins {
    /// Host requested a job start (consumed by T02 CT01).
    pub start_job: bool,
    /// Host requested cancellation; the shell records it in `control` for T02.
    pub cancel: bool,
    /// Host responses available this tick (consumed by T02 CT02).
    pub host_responses: Vec<HostResponse>,
    /// The tick budget was reached; the shell marks the job budget-exhausted.
    pub tick_budget_reached: bool,
}

/// The single canonical state carrier for the compiler application.
pub struct CompilerBus {
    /// Read-only job configuration; structurally immutable after construction.
    config: CompilerConfig,
    /// Persistent control registers.
    pub control: Control,
    /// Typed arenas.
    pub arenas: Arenas,
    /// Intern table (the `names` store).
    pub intern: InternTable,
    /// Task queue state.
    pub tasks: TaskStore,
    /// Registered task kinds.
    pub kinds: TaskKindRegistry,
    /// Declared store fields.
    pub schema: crate::manifest::StoreSchema,
    /// Registered chip manifests.
    pub registrations: ManifestRegistry,
    /// Routing table; part of the bus so replay cannot diverge from it.
    pub routing: RoutingTable,
    /// Per-tick proposals.
    pub wires: CompilerWires,
    /// Validated store patches, in commit order.
    pub patch_log: Vec<crate::commit::CommittedPatch>,
    /// Per-store revision counters.
    pub store_versions: crate::commit::StoreVersions,
    /// Canonical bounded tick report (`/6` working basis).
    ///
    /// Bounded by `max_ticks + 1` records. Sole-append-site rule: only the
    /// driver step 6 appends here (the proposed `report.rs`/`driver.rs`; T02).
    /// The current quota-1 routing shell acts as that driver until the
    /// proposed modules land, and appends exactly one record per tick through
    /// [`CompilerBus::push_tick_record`].
    pub report: Vec<TickRecord>,
}

impl CompilerBus {
    /// Create a bus from a configuration, initializing the foundation schema
    /// and the frozen task-kind registry.
    pub fn new(config: CompilerConfig) -> Self {
        Self {
            config,
            control: Control::default(),
            arenas: Arenas::new(),
            intern: InternTable::new(),
            tasks: TaskStore::default(),
            kinds: TaskKindRegistry::foundation(),
            schema: crate::manifest::StoreSchema::foundation(),
            registrations: ManifestRegistry::new(),
            routing: RoutingTable::new(),
            wires: CompilerWires::default(),
            patch_log: Vec::new(),
            store_versions: crate::commit::StoreVersions::new(),
            report: Vec::new(),
        }
    }

    /// Append one tick record to the canonical bounded report.
    ///
    /// Fails with [`ReportCapacityError::ReportFull`] once `max_ticks + 1`
    /// records are retained. The bound uses saturating arithmetic so a
    /// `u64::MAX` tick budget cannot wrap.
    pub fn push_tick_record(&mut self, record: TickRecord) -> Result<(), ReportCapacityError> {
        let limit = self.limits().max_ticks.saturating_add(1);
        let requested = self.report.len() as u64 + 1;
        if requested > limit {
            return Err(ReportCapacityError::ReportFull { limit, requested });
        }
        self.report.push(record);
        Ok(())
    }

    /// The configured resource limits.
    pub fn limits(&self) -> Limits {
        self.config.limits()
    }

    /// The read-only configuration.
    ///
    /// The field is private so a job cannot swap its target/dialect/limits
    /// after initialization; commit also rejects writes to the `config` store.
    pub fn config(&self) -> &CompilerConfig {
        &self.config
    }

    /// Total records counts all arena slots plus the committed patch log.
    pub fn total_records(&self) -> u64 {
        self.arenas.allocated_total() + self.patch_log.len() as u64
    }

    /// Fail if `additional` more records would exceed the total bound.
    pub fn ensure_total_records(&self, additional: u64) -> Result<(), LimitError> {
        let limit = self.limits().max_records_total;
        let requested = self.total_records().saturating_add(additional);
        if requested > limit {
            Err(LimitError::TotalRecords { limit, requested })
        } else {
            Ok(())
        }
    }

    /// Total source bytes held.
    pub fn source_bytes(&self) -> u64 {
        self.arenas
            .sources
            .iter()
            .map(|(_, source)| source.bytes.len() as u64)
            .sum()
    }

    /// Fail if `additional` more source bytes would exceed the bound.
    pub fn ensure_source_bytes(&self, additional: u64) -> Result<(), LimitError> {
        let limit = self.limits().max_source_bytes;
        let requested = self.source_bytes().saturating_add(additional);
        if requested > limit {
            Err(LimitError::SourceBytes { limit, requested })
        } else {
            Ok(())
        }
    }

    /// Ancestor depth of a task.
    ///
    /// Fails with [`LimitError::DanglingParent`] when a parent reference points
    /// at no live task, and with [`LimitError::TaskDepth`] when the bound is
    /// hit. A missing parent is never silently treated as the root.
    pub fn task_depth(&self, task: Option<TaskId>) -> Result<u32, LimitError> {
        let limit = self.limits().max_task_depth;
        let mut depth = 0u32;
        let mut current = task;
        while let Some(id) = current {
            depth = depth.saturating_add(1);
            if depth > limit {
                return Err(LimitError::TaskDepth {
                    limit,
                    requested: depth,
                });
            }
            current = match self.arenas.tasks.get(id) {
                Ok(record) => record.parent,
                Err(_) => return Err(LimitError::DanglingParent { parent: id }),
            };
        }
        Ok(depth)
    }

    /// Fail if `additional` diagnostics would exceed the bound.
    pub fn ensure_diagnostics(&self, additional: u32) -> Result<(), LimitError> {
        let limit = self.limits().max_diagnostics;
        let requested = self
            .arenas
            .diagnostics
            .allocated()
            .saturating_add(additional);
        if requested > limit {
            Err(LimitError::Diagnostics { limit, requested })
        } else {
            Ok(())
        }
    }

    /// Intern a name.
    pub fn intern_name(&mut self, bytes: &[u8]) -> Result<NameId, crate::intern::InternError> {
        let limits = self.limits();
        self.intern.intern(bytes, &limits)
    }

    /// Allocate a source record, enforcing the source-byte and total bounds.
    ///
    /// The content hash is computed here from the bytes; callers cannot supply
    /// an unvalidated hash. Hosts that verified a downloaded artifact must do
    /// so before handing the bytes to the bus.
    pub fn alloc_source(&mut self, name: NameId, bytes: Vec<u8>) -> Result<SourceId, LimitError> {
        self.ensure_source_bytes(bytes.len() as u64)?;
        self.ensure_total_records(1)?;
        let content_hash = crate::codec::sha256(&bytes);
        let limits = self.limits();
        Ok(self.arenas.sources.alloc(
            SourceRecord {
                name,
                bytes,
                content_hash,
            },
            &limits,
        )?)
    }

    /// Seed a task that is immediately eligible.
    ///
    /// **Integration/job-bootstrap only (CT01).** Worker chips must never call
    /// this: they may only propose `Enqueue` through the commit path, which
    /// makes new tasks visible one tick later and enforces the registered
    /// field/kind manifests. This entry point bypasses that next-tick rule for
    /// the initial job task and is used by the host/control integration and by
    /// test fixtures.
    pub fn bootstrap_task(&mut self, draft: TaskDraft) -> Result<TaskId, LimitError> {
        let limit = self.limits().max_queue_len;
        let requested = self.tasks.ready.len() as u32 + 1;
        if requested > limit {
            return Err(LimitError::Queue { limit, requested });
        }
        let id = self.alloc_task(draft, self.control.tick)?;
        self.tasks.ready.push(id);
        Ok(id)
    }

    /// Allocate a task record with an enqueue ordinal, enforcing task bounds.
    pub(crate) fn alloc_task(
        &mut self,
        draft: TaskDraft,
        ready_tick: u64,
    ) -> Result<TaskId, LimitError> {
        let limits = self.limits();
        let depth = self.task_depth(draft.parent)? + 1;
        if depth > limits.max_task_depth {
            return Err(LimitError::TaskDepth {
                limit: limits.max_task_depth,
                requested: depth,
            });
        }
        let requested_tasks = self.arenas.tasks.allocated() as u64 + 1;
        if requested_tasks > limits.max_tasks_total {
            return Err(LimitError::TasksTotal {
                limit: limits.max_tasks_total,
                requested: requested_tasks,
            });
        }
        self.ensure_total_records(1)?;

        let ordinal = self.control.next_enqueue_ordinal;
        let id = self.arenas.tasks.alloc(
            Task {
                id: TaskId::from_index(0),
                kind: draft.kind,
                payload: draft.payload,
                owner: draft.owner,
                parent: draft.parent,
                continuation: draft.continuation,
                state: crate::task::TaskState::Ready,
                enqueue_ordinal: ordinal,
                ready_tick,
                progress_count: 0,
                progress_ordinal: 0,
            },
            &limits,
        )?;
        // Fix up the self ID now that the index is known.
        self.arenas.tasks.get_mut(id)?.id = id;
        self.control.next_enqueue_ordinal += 1;
        Ok(id)
    }

    /// Append a task after a commit preflight has reserved capacity.
    ///
    /// Crate-private and infallible: the commit apply pass must not perform a
    /// fallible step after its first mutation. The caller guarantees the bound
    /// checks were already done.
    pub(crate) fn push_task_reserved(&mut self, draft: TaskDraft, ready_tick: u64) -> TaskId {
        let id = TaskId::from_index(self.arenas.tasks.allocated());
        let ordinal = self.control.next_enqueue_ordinal;
        self.arenas.tasks.push(Task {
            id,
            kind: draft.kind,
            payload: draft.payload,
            owner: draft.owner,
            parent: draft.parent,
            continuation: draft.continuation,
            state: crate::task::TaskState::Ready,
            enqueue_ordinal: ordinal,
            ready_tick,
            progress_count: 0,
            progress_ordinal: 0,
        });
        self.control.next_enqueue_ordinal += 1;
        id
    }

    /// Look up an artifact by key for tests.
    pub fn task_store(&self) -> &TaskStore {
        &self.tasks
    }

    /// Borrow a task.
    pub fn get_task(&self, id: TaskId) -> Result<&Task, crate::arena::ArenaError> {
        self.arenas.tasks.get(id)
    }

    /// Store field-path lookup used by commit validation.
    pub fn store_is_declared(&self, store: StoreId, field: &'static str) -> bool {
        self.schema.fields(store).contains(&field)
    }

    /// The task kind name, when registered.
    pub fn kind_name(&self, kind: TaskKind) -> &'static str {
        self.kinds
            .lookup(kind)
            .map(|entry| entry.name)
            .unwrap_or("unregistered")
    }
}

impl Bus for CompilerBus {
    type Pins = CompilerPins;
    type Wires = CompilerWires;

    fn wires(&self) -> &Self::Wires {
        &self.wires
    }

    fn wires_mut(&mut self) -> &mut Self::Wires {
        &mut self.wires
    }

    fn tick_count(&self) -> u64 {
        self.control.tick
    }

    fn advance_tick(&mut self) {
        self.control.tick = self.control.tick.wrapping_add(1);
    }
}

impl Default for CompilerBus {
    fn default() -> Self {
        Self::new(CompilerConfig::default())
    }
}
