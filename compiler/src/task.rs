// ============================================================================
// task.rs — task/completion protocol (T01 C03)
//
// Frozen semantic shapes:
//
//   Task   { id, kind, payload, owner, parent, continuation, state, ... }
//   State  = Ready | Running | Waiting(child/request ids) | Completed | Failed
//   Result = tagged payload matching the task kind
//   Proposal = Enqueue | Complete | Fail | AwaitHost | StorePatch
//
// A task enqueued in tick T is ready in tick T+1. The selection order is
// deterministic: (phase priority, enqueue ordinal, TaskId). Completion and
// result consumption are exactly-once. New tasks are only made visible to the
// scheduler by the commit path (see `commit.rs`).
// ============================================================================

use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{
    ChipId, ContinuationId, DiagnosticId, HostRequestId, RecordRef, ResultId, ScopeId, TaskId,
};

/// A compiler pipeline group. Each group owns one stage of the compiler.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TaskGroup(u8);

impl TaskGroup {
    /// Control/scheduling chips.
    pub const CONTROL: Self = Self(0);
    /// Host request/response tasks.
    pub const HOST: Self = Self(1);
    /// Preprocessing.
    pub const PREPROCESS: Self = Self(2);
    /// Lexical analysis.
    pub const LEX: Self = Self(3);
    /// Parsing.
    pub const PARSE: Self = Self(4);
    /// Symbols and types.
    pub const SYMBOL_TYPE: Self = Self(5);
    /// Semantic analysis.
    pub const SEMANTIC: Self = Self(6);
    /// Constants, layout, and initialization.
    pub const CONSTANT_LAYOUT_INIT: Self = Self(7);
    /// IR lowering.
    pub const IR_LOWER: Self = Self(8);
    /// Optimization.
    pub const OPTIMIZE: Self = Self(9);
    /// Target code generation.
    pub const TARGET_CODE: Self = Self(10);
    /// GNU extensions and builtins.
    pub const GNU_BUILTIN: Self = Self(11);
    /// Verification.
    pub const VERIFICATION: Self = Self(12);

    /// Every group in canonical order.
    pub const ALL: [Self; 13] = [
        Self::CONTROL,
        Self::HOST,
        Self::PREPROCESS,
        Self::LEX,
        Self::PARSE,
        Self::SYMBOL_TYPE,
        Self::SEMANTIC,
        Self::CONSTANT_LAYOUT_INIT,
        Self::IR_LOWER,
        Self::OPTIMIZE,
        Self::TARGET_CODE,
        Self::GNU_BUILTIN,
        Self::VERIFICATION,
    ];

    /// Raw discriminant.
    pub const fn raw(self) -> u8 {
        self.0
    }

    /// Rebuild from a raw discriminant.
    pub const fn from_raw(raw: u8) -> Option<Self> {
        if raw < 13 {
            Some(Self(raw))
        } else {
            None
        }
    }

    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self.0 {
            0 => "control",
            1 => "host",
            2 => "preprocess",
            3 => "lex",
            4 => "parse",
            5 => "symbol_type",
            6 => "semantic",
            7 => "constant_layout_init",
            8 => "ir_lower",
            9 => "optimize",
            10 => "target_code",
            11 => "gnu_builtin",
            12 => "verification",
            _ => "unknown",
        }
    }
}

/// A task kind: 4-bit group plus a 12-bit per-group local code.
///
/// The encoding is stable and frozen. Local codes `0..=15` of every group are
/// reserved for foundation/protocol kinds and may only be registered with
/// [`KindStatus::Frozen`]; group owners claim codes from `16` upward. Two
/// kinds with the same name but different codes are rejected by
/// [`TaskKindRegistry`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TaskKind(u16);

impl TaskKind {
    /// Number of low bits reserved for the local code.
    pub const LOCAL_BITS: u16 = 12;
    /// Mask selecting the local code.
    pub const LOCAL_MASK: u16 = (1 << Self::LOCAL_BITS) - 1;
    /// Maximum local code.
    pub const LOCAL_MAX: u16 = Self::LOCAL_MASK;
    /// Highest reserved local code; group owners start at `16`.
    pub const RESERVED_LOCAL_MAX: u16 = 15;

    /// Build a kind from a group and a local code.
    pub const fn new(group: TaskGroup, local: u16) -> Option<Self> {
        if local > Self::LOCAL_MAX {
            return None;
        }
        Some(Self(((group.raw() as u16) << Self::LOCAL_BITS) | local))
    }

    /// The owning group.
    pub const fn group(self) -> TaskGroup {
        // The high nibble is always < 16 because `new` rejects larger groups.
        TaskGroup((self.0 >> Self::LOCAL_BITS) as u8)
    }

    /// The per-group local code.
    pub const fn local(self) -> u16 {
        self.0 & Self::LOCAL_MASK
    }

    /// Raw encoding.
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// A foundation control task that terminates successfully as a no-op.
    pub const CONTROL_NOOP: Self = Self(0);
    /// A foundation control task that always fails as unsupported.
    pub const CONTROL_UNSUPPORTED: Self = Self(1);
    /// A foundation control task that starts a job.
    pub const CONTROL_START_JOB: Self = Self(2);
    /// A foundation control task that imports a source response.
    pub const CONTROL_IMPORT_SOURCE: Self = Self(3);

    /// Whether this is one of the frozen foundation kinds.
    pub const fn is_foundation(self) -> bool {
        self.0 <= Self::CONTROL_IMPORT_SOURCE.0
    }

    /// Whether this kind lives in a group's reserved local-code range.
    pub const fn is_reserved_local(self) -> bool {
        self.local() <= Self::RESERVED_LOCAL_MAX
    }
}

/// Registration status of a task kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KindStatus {
    /// Frozen by T01 and cannot be redefined.
    Frozen,
    /// Reserved by a group owner but not yet implemented.
    Reserved,
    /// Owned by a group and available for registration.
    GroupOwned,
}

/// A registered task kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KindEntry {
    /// The kind.
    pub kind: TaskKind,
    /// Globally unique name.
    pub name: &'static str,
    /// Owning group (must equal `kind.group()`).
    pub group: TaskGroup,
    /// Registration status.
    pub status: KindStatus,
}

/// Structured task-registry failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// The kind code already exists.
    DuplicateKind {
        /// Offending kind.
        kind: TaskKind,
    },
    /// The name already exists with different semantics.
    DuplicateName {
        /// Offending name.
        name: &'static str,
    },
    /// The declared group disagrees with the encoded group.
    GroupMismatch {
        /// Encoded group.
        encoded: TaskGroup,
        /// Declared group.
        declared: TaskGroup,
    },
    /// The kind is frozen and cannot be registered again.
    Frozen {
        /// Offending kind.
        kind: TaskKind,
    },
    /// A non-frozen kind claimed a reserved local code (`0..=15`).
    ReservedLocalCode {
        /// Offending kind.
        kind: TaskKind,
    },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateKind { kind } => {
                write!(f, "task kind {} already registered", kind.raw())
            }
            Self::DuplicateName { name } => write!(f, "task kind name `{name}` already registered"),
            Self::GroupMismatch { encoded, declared } => write!(
                f,
                "task kind group mismatch: encoded `{}`, declared `{}`",
                encoded.name(),
                declared.name()
            ),
            Self::Frozen { kind } => write!(f, "task kind {} is frozen", kind.raw()),
            Self::ReservedLocalCode { kind } => write!(
                f,
                "task kind {} uses reserved local code {}",
                kind.raw(),
                kind.local()
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

impl RegistryError {
    /// Map to a structured diagnostic.
    pub fn to_diagnostic(&self) -> DiagnosticDraft {
        DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 2), self.to_string())
    }
}

/// A registry of task kinds with globally unique names.
#[derive(Clone, Debug, Default)]
pub struct TaskKindRegistry {
    /// Entries sorted by raw kind code.
    entries: Vec<KindEntry>,
}

impl TaskKindRegistry {
    /// The frozen foundation registry.
    pub fn foundation() -> Self {
        let mut registry = Self::default();
        let foundation: &[(TaskKind, &str)] = &[
            (TaskKind::CONTROL_NOOP, "control.noop"),
            (TaskKind::CONTROL_UNSUPPORTED, "control.unsupported"),
            (TaskKind::CONTROL_START_JOB, "control.start_job"),
            (TaskKind::CONTROL_IMPORT_SOURCE, "control.import_source"),
        ];
        for &(kind, name) in foundation {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// Register a task kind or return a structured error.
    pub fn register(
        &mut self,
        kind: TaskKind,
        name: &'static str,
        group: TaskGroup,
        status: KindStatus,
    ) -> Result<(), RegistryError> {
        if kind.group() != group {
            return Err(RegistryError::GroupMismatch {
                encoded: kind.group(),
                declared: group,
            });
        }
        if kind.is_reserved_local() && status != KindStatus::Frozen {
            return Err(RegistryError::ReservedLocalCode { kind });
        }
        if let Some(existing) = self.lookup(kind) {
            if existing.status == KindStatus::Frozen {
                return Err(RegistryError::Frozen { kind });
            }
            return Err(RegistryError::DuplicateKind { kind });
        }
        if self.entries.iter().any(|entry| entry.name == name) {
            return Err(RegistryError::DuplicateName { name });
        }
        let insert_at = self.entries.partition_point(|entry| entry.kind < kind);
        self.entries.insert(
            insert_at,
            KindEntry {
                kind,
                name,
                group,
                status,
            },
        );
        Ok(())
    }

    /// Look up a kind.
    pub fn lookup(&self, kind: TaskKind) -> Option<&KindEntry> {
        self.entries.iter().find(|entry| entry.kind == kind)
    }

    /// Look up a name.
    pub fn lookup_name(&self, name: &str) -> Option<&KindEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    /// Iterate entries in ascending kind order.
    pub fn iter(&self) -> impl Iterator<Item = &KindEntry> {
        self.entries.iter()
    }

    /// Number of registered kinds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A typed task payload: a set of record references produced by upstream
/// stores. The concrete meaning is defined by the task kind's group owner.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Payload {
    /// Ordered record references.
    pub refs: Vec<RecordRef>,
}

impl Payload {
    /// An empty payload.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build a payload from record references.
    pub fn from_refs(refs: Vec<RecordRef>) -> Self {
        Self { refs }
    }
}

/// A registered continuation: the state a parent task needs to resume after
/// its children complete. Continuations are mechanical; resume semantics
/// belong to the producing chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationRecord {
    /// Task kind the parent will resume with.
    pub resume_kind: TaskKind,
    /// Child tasks awaited, in enqueue order.
    pub awaited: Vec<TaskId>,
    /// Optional scope the continuation belongs to.
    pub scope: Option<ScopeId>,
}

/// Task lifecycle state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskState {
    /// Not yet selected.
    Ready,
    /// Selected and executing this tick.
    Running,
    /// Waiting on children and/or a host request.
    Waiting(WaitSet),
    /// Finished successfully.
    Completed(ResultId),
    /// Finished with a structured failure.
    Failed(DiagnosticId),
}

impl TaskState {
    /// Stable label used in snapshots.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Waiting(_) => "waiting",
            Self::Completed(_) => "completed",
            Self::Failed(_) => "failed",
        }
    }

    /// Whether the task has reached a terminal state.
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed(_) | Self::Failed(_))
    }
}

/// What a waiting task depends on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WaitSet {
    /// Child task IDs awaited.
    pub children: Vec<TaskId>,
    /// Optional host request awaited.
    pub host_request: Option<HostRequestId>,
}

/// A result payload. Concrete variants are extended by each group; the
/// foundation variants cover no-op, acknowledgement, record references, and
/// committed diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultValue {
    /// No value (a successful no-op).
    Empty,
    /// A generic acknowledgement.
    Ack,
    /// A single produced record.
    Record(RecordRef),
    /// Several produced records, in production order.
    Records(Vec<RecordRef>),
    /// A committed diagnostic reference.
    Diagnostic(DiagnosticId),
}

/// A committed result record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultRecord {
    /// Producing task.
    pub task: TaskId,
    /// Task kind the result matches.
    pub kind: TaskKind,
    /// Result payload.
    pub value: ResultValue,
    /// Store/protocol version the result was produced against.
    pub version: u64,
    /// Whether a consumer has already taken this result.
    pub consumed: bool,
}

/// A persistent task record. Live in the `tasks` arena.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    /// Stable ID.
    pub id: TaskId,
    /// Kind.
    pub kind: TaskKind,
    /// Typed input references.
    pub payload: Payload,
    /// Chip responsible for executing this task.
    pub owner: ChipId,
    /// Producing parent task, if any.
    pub parent: Option<TaskId>,
    /// Continuation to resume on this task's completion, if any.
    pub continuation: Option<ContinuationId>,
    /// Lifecycle state.
    pub state: TaskState,
    /// Global monotonic enqueue ordinal.
    pub enqueue_ordinal: u64,
    /// Earliest tick in which this task may be selected.
    pub ready_tick: u64,
}

impl Task {
    /// The deterministic scheduling key: `(phase priority, enqueue ordinal,
    /// TaskId)`.
    pub fn schedule_key(&self, phase_priority: u16) -> (u16, u64, u32) {
        (phase_priority, self.enqueue_ordinal, self.id.index())
    }
}

/// A not-yet-committed task creation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskDraft {
    /// Kind of the new task.
    pub kind: TaskKind,
    /// Input references.
    pub payload: Payload,
    /// Chip that will own the new task.
    pub owner: ChipId,
    /// Producing parent task.
    pub parent: Option<TaskId>,
    /// Continuation to attach.
    pub continuation: Option<ContinuationId>,
}

/// A staged, field-scoped store write proposed by a worker.
///
/// Patches are the only way a worker may change a shared store; a batch of
/// patches commits atomically (see [`crate::commit`]). `owner` and `version`
/// scope the write to the producing chip and store revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorePatch {
    /// Producing chip.
    pub owner: ChipId,
    /// Producing task.
    pub task: TaskId,
    /// Store revision the patch was computed against.
    pub version: u64,
    /// Target store.
    pub store: StoreId,
    /// Declared field path within the store.
    pub field: &'static str,
    /// Patch operation.
    pub op: PatchOp,
    /// Existing record the patch targets (for replace/tombstone).
    pub target: Option<RecordRef>,
    /// New record value (for append/replace).
    pub value: Option<RecordRef>,
}

/// A store patch operation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PatchOp {
    /// Append a new record.
    Append,
    /// Replace an existing record.
    Replace,
    /// Tombstone an existing record.
    Tombstone,
}

/// A proposal emitted by a worker during a tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Proposal {
    /// Create a new task, visible next tick.
    Enqueue(TaskDraft),
    /// Complete the selected task with a result value.
    Complete {
        /// Task being completed.
        task: TaskId,
        /// Result value.
        value: ResultValue,
    },
    /// Fail the selected task with a structured diagnostic.
    Fail {
        /// Task being failed.
        task: TaskId,
        /// Failure diagnostic.
        diagnostic: DiagnosticDraft,
    },
    /// Block the selected task on a host request.
    AwaitHost {
        /// Task being blocked.
        task: TaskId,
        /// Request record to create.
        request: HostRequestDraft,
    },
    /// Stage a field-scoped store write.
    StorePatch(StorePatch),
}

/// A not-yet-committed host request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRequestDraft {
    /// Machine-readable request kind.
    pub kind: HostRequestKind,
    /// Typed request payload references.
    pub payload: Payload,
}

/// Kinds of host interaction the compiler may request.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum HostRequestKind {
    /// Read one source file.
    ReadSource,
    /// Write one output artifact.
    WriteArtifact,
    /// Run the external assembler/linker.
    InvokeToolchain,
    /// Cancellation acknowledgement.
    Cancel,
}

/// A committed host request record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRequestRecord {
    /// Requesting task.
    pub task: TaskId,
    /// Request kind.
    pub kind: HostRequestKind,
    /// Typed payload references.
    pub payload: Payload,
    /// Whether a response has been consumed.
    pub satisfied: bool,
}

/// Compiler storage partitions (T01 section 2). A `StorePatch` targets one of
/// these; the SFL manifest validator checks field paths against this set.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum StoreId {
    /// Read-only job configuration.
    Config,
    /// Phase/tick/job control.
    Control,
    /// Source bytes and provenance.
    Sources,
    /// Preprocessing store.
    Pp,
    /// Lexical store.
    Lex,
    /// Parse/AST store.
    Parse,
    /// Symbol store.
    Symbols,
    /// Type store.
    Types,
    /// Semantic facts.
    Sem,
    /// Constant store.
    Constants,
    /// Layout store.
    Layout,
    /// Initialization store.
    Init,
    /// IR store.
    Ir,
    /// Optimization store.
    Opt,
    /// Machine/ABI store.
    Machine,
    /// GNU/builtin/asm store.
    Ext,
    /// Task queue/results.
    Tasks,
    /// Diagnostics.
    Diagnostics,
    /// Output artifacts.
    Artifacts,
    /// Per-tick proposals.
    Wires,
}

impl StoreId {
    /// Every store in canonical order.
    pub const ALL: [StoreId; 20] = [
        StoreId::Config,
        StoreId::Control,
        StoreId::Sources,
        StoreId::Pp,
        StoreId::Lex,
        StoreId::Parse,
        StoreId::Symbols,
        StoreId::Types,
        StoreId::Sem,
        StoreId::Constants,
        StoreId::Layout,
        StoreId::Init,
        StoreId::Ir,
        StoreId::Opt,
        StoreId::Machine,
        StoreId::Ext,
        StoreId::Tasks,
        StoreId::Diagnostics,
        StoreId::Artifacts,
        StoreId::Wires,
    ];

    /// Number of stores, derived from [`StoreId::ALL`].
    pub const COUNT: usize = Self::ALL.len();

    /// Stable label, matching SFL field-path segments.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Control => "control",
            Self::Sources => "sources",
            Self::Pp => "pp",
            Self::Lex => "lex",
            Self::Parse => "parse",
            Self::Symbols => "symbols",
            Self::Types => "types",
            Self::Sem => "sem",
            Self::Constants => "constants",
            Self::Layout => "layout",
            Self::Init => "init",
            Self::Ir => "ir",
            Self::Opt => "opt",
            Self::Machine => "machine",
            Self::Ext => "ext",
            Self::Tasks => "tasks",
            Self::Diagnostics => "diagnostics",
            Self::Artifacts => "artifacts",
            Self::Wires => "wires",
        }
    }

    /// Parse a store label.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|store| store.name() == name)
    }

    /// Canonical index within [`StoreId::ALL`].
    pub const fn index(self) -> usize {
        match self {
            Self::Config => 0,
            Self::Control => 1,
            Self::Sources => 2,
            Self::Pp => 3,
            Self::Lex => 4,
            Self::Parse => 5,
            Self::Symbols => 6,
            Self::Types => 7,
            Self::Sem => 8,
            Self::Constants => 9,
            Self::Layout => 10,
            Self::Init => 11,
            Self::Ir => 12,
            Self::Opt => 13,
            Self::Machine => 14,
            Self::Ext => 15,
            Self::Tasks => 16,
            Self::Diagnostics => 17,
            Self::Artifacts => 18,
            Self::Wires => 19,
        }
    }

    /// Rebuild from a canonical index.
    pub const fn from_index(index: usize) -> Option<Self> {
        if index < 20 {
            Some(Self::ALL[index])
        } else {
            None
        }
    }
}

/// The full set of ID types declared by this contract, in canonical order.
///
/// Used by the frozen schema hash and by manifest/registry documentation.
pub const RECORD_KINDS: &[&str] = &[
    "sources",
    "spans",
    "expansions",
    "pp_tokens",
    "tokens",
    "names",
    "scopes",
    "symbols",
    "types",
    "nodes",
    "constants",
    "layouts",
    "inits",
    "functions",
    "blocks",
    "values",
    "instructions",
    "vregs",
    "continuations",
    "tasks",
    "results",
    "diagnostics",
    "host_requests",
    "artifacts",
];
