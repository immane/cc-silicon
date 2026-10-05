// ============================================================================
// task.rs — task/completion protocol (T01 C03)
//
// Frozen semantic shapes:
//
//   Task   { id, kind, payload, owner, parent, continuation, state, ... }
//   State  = Ready | Running | Waiting(child/request ids) | Completed | Failed
//   Result = tagged payload matching the task kind
//   Proposal = Enqueue | Complete | Fail | AwaitHost | AwaitChildren
//            | StorePatch | AppendRecords | Progress
//
// A task enqueued in tick T is ready in tick T+1. The selection order is
// deterministic: (phase priority, enqueue ordinal, TaskId). Completion and
// result consumption are exactly-once. New tasks are only made visible to the
// scheduler by the commit path (see `commit.rs`).
// ============================================================================

use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{
    ChipId, ContinuationId, DiagnosticId, HostRequestId, LiteralId, NodeId, RecordRef, ResultId,
    ScopeId, TaskId, TokenId,
};
use crate::records::{G1DraftBody, RecordDraft};

/// A compiler pipeline group. Each group owns one stage of the compiler.
///
/// Integrator note: assigning every task kind to exactly one scheduled stage
/// (the versioned kind-to-stage table) and enforcing it at registration with
/// `ManifestError::StageUnassigned { kind }` and
/// `ManifestError::StageLayerMismatch { kind, stage, layer }` is
/// manifest-track work in `manifest.rs`; only the variant shapes are noted
/// here so the two tracks use identical spellings.
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

    /// Gate 1 (`/7`) M1 slice kind: T07 sem-stage const-evaluate request,
    /// literal form (payload: exactly one `RecordRef::Literal`).
    pub const SEMANTIC_CONST_EVAL_LITERAL: Self =
        Self(((TaskGroup::SEMANTIC.0 as u16) << Self::LOCAL_BITS) | 16);
    /// Gate 1 (`/7`) M1 slice kind: T07 sem-stage const-evaluate request,
    /// binary form (payload: `RecordRef::Node` of the committed
    /// `BinaryExpression`, then two `RecordRef::Literal` operands in source
    /// order).
    pub const SEMANTIC_CONST_EVAL_BINARY: Self =
        Self(((TaskGroup::SEMANTIC.0 as u16) << Self::LOCAL_BITS) | 17);
    /// Gate 1 (`/7`) M1 slice kind: T08 const-fold task (decodes the
    /// request, folds with checked addition, appends exactly one
    /// `ConstRecord`).
    pub const CONSTANT_CONST_FOLD: Self =
        Self(((TaskGroup::CONSTANT_LAYOUT_INIT.0 as u16) << Self::LOCAL_BITS) | 16);

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

    /// The Gate 1 (`/7`) M1 slice registry: foundation plus the three frozen
    /// slice kinds (all `Frozen`; group owners start new codes at local 18
    /// for `SEMANTIC` and local 17 for `CONSTANT_LAYOUT_INIT`).
    pub fn m1_slice() -> Self {
        let mut registry = Self::foundation();
        let slice: &[(TaskKind, &str)] = &[
            (
                TaskKind::SEMANTIC_CONST_EVAL_LITERAL,
                "semantic.const_eval_literal",
            ),
            (
                TaskKind::SEMANTIC_CONST_EVAL_BINARY,
                "semantic.const_eval_binary",
            ),
            (TaskKind::CONSTANT_CONST_FOLD, "constant_layout_init.const_fold"),
        ];
        for &(kind, name) in slice {
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

/// M1-closed constant-expression operator (Gate 1 `/7`).
///
/// Only `Add` is produced in M1 (the `2 + 3` exercised subset). Future
/// operators append variants without reinterpreting `Add`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstExprOp {
    /// Checked integer addition.
    Add,
}

impl ConstExprOp {
    /// Canonical encoding name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
        }
    }
}

/// M1-closed per-use constant-expression requirement (Gate 1 `/7`).
///
/// The requirement is implied by the M1 slice kinds (each serves exactly
/// one purpose) and travels as no wire bytes; future C purposes require
/// appended variants and new kinds, never a repurposed lexical type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequiredKind {
    /// An integer constant expression.
    IntegerConstantExpression,
}

impl RequiredKind {
    /// Canonical encoding name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::IntegerConstantExpression => "integer_constant_expression",
        }
    }
}

/// M1-closed evaluation legality outcome (Gate 1 `/7`, result payload
/// field; no extra committed record family).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstLegality {
    /// A legal constant expression with a folded value.
    Legal,
    /// Well-formed but not a constant expression.
    NotConstantExpression,
    /// A construct the M1 subset does not implement.
    Unsupported,
}

impl ConstLegality {
    /// Canonical encoding name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Legal => "legal",
            Self::NotConstantExpression => "not_constant_expression",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Structured constant-request decode failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestError {
    /// The task kind is not a const-evaluate request kind.
    UnexpectedKind {
        /// Offending kind.
        kind: TaskKind,
    },
    /// The payload reference count does not match the kind's convention.
    Arity {
        /// Request kind.
        kind: TaskKind,
        /// References carried.
        got: usize,
    },
    /// The payload reference at a position has the wrong family.
    Family {
        /// Request kind.
        kind: TaskKind,
        /// Reference position.
        position: usize,
    },
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedKind { kind } => {
                write!(f, "task kind {} is not a const-evaluate request", kind.raw())
            }
            Self::Arity { kind, got } => {
                write!(
                    f,
                    "const-evaluate request {} carries {got} references",
                    kind.raw()
                )
            }
            Self::Family { kind, position } => {
                write!(
                    f,
                    "const-evaluate request {} reference {position} has the wrong family",
                    kind.raw()
                )
            }
        }
    }
}

impl std::error::Error for RequestError {}

impl RequestError {
    /// Map to a structured diagnostic (protocol group: the worker or its
    /// caller broke the frozen request convention).
    pub fn to_diagnostic(&self) -> DiagnosticDraft {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Protocol, 1),
            self.to_string(),
        )
    }
}

/// A decoded sem-stage constant-evaluate request (Gate 1 `/7` OPEN-03
/// co-freeze shape).
///
/// The wire form is `(TaskKind, Payload)` only — [`Payload`] stays
/// [`RecordRef`]-only by protocol rule, so `required_kind` and `op` travel
/// as no bytes: the M1 slice kinds each imply
/// [`RequiredKind::IntegerConstantExpression`], and the binary form
/// implies [`ConstExprOp::Add`] (the T07-checked operator).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantRequest {
    /// Evaluate one committed literal.
    Literal {
        /// Committed literal to evaluate.
        literal: LiteralId,
        /// Per-use requirement (always `IntegerConstantExpression` in M1).
        required_kind: RequiredKind,
    },
    /// Fold two committed literals with the checked operator.
    Binary {
        /// Committed `BinaryExpression` node.
        node: NodeId,
        /// Checked operator (always `Add` in M1).
        op: ConstExprOp,
        /// Left operand, in source order.
        lhs: LiteralId,
        /// Right operand, in source order.
        rhs: LiteralId,
        /// Per-use requirement (always `IntegerConstantExpression` in M1).
        required_kind: RequiredKind,
    },
}

impl ConstantRequest {
    /// Decode a `(kind, payload)` pair per the frozen convention:
    /// `const_eval_literal` carries exactly one `RecordRef::Literal`;
    /// `const_eval_binary` carries `RecordRef::Node` then two
    /// `RecordRef::Literal` in source order.
    pub fn decode(kind: TaskKind, payload: &Payload) -> Result<Self, RequestError> {
        fn literal_at(
            kind: TaskKind,
            payload: &Payload,
            position: usize,
        ) -> Result<LiteralId, RequestError> {
            match payload.refs.get(position) {
                Some(RecordRef::Literal(id)) => Ok(*id),
                _ => Err(RequestError::Family { kind, position }),
            }
        }
        if kind == TaskKind::SEMANTIC_CONST_EVAL_LITERAL {
            if payload.refs.len() != 1 {
                return Err(RequestError::Arity {
                    kind,
                    got: payload.refs.len(),
                });
            }
            Ok(Self::Literal {
                literal: literal_at(kind, payload, 0)?,
                required_kind: RequiredKind::IntegerConstantExpression,
            })
        } else if kind == TaskKind::SEMANTIC_CONST_EVAL_BINARY {
            if payload.refs.len() != 3 {
                return Err(RequestError::Arity {
                    kind,
                    got: payload.refs.len(),
                });
            }
            let node = match payload.refs[0] {
                RecordRef::Node(id) => id,
                _ => return Err(RequestError::Family { kind, position: 0 }),
            };
            Ok(Self::Binary {
                node,
                op: ConstExprOp::Add,
                lhs: literal_at(kind, payload, 1)?,
                rhs: literal_at(kind, payload, 2)?,
                required_kind: RequiredKind::IntegerConstantExpression,
            })
        } else {
            Err(RequestError::UnexpectedKind { kind })
        }
    }
}

/// A constant-evaluation outcome and its frozen proposal routing (Gate 1
/// `/7`).
///
/// There is no new `ResultValue` variant: `Legal` completes with
/// [`ResultValue::Record`] carrying the committed `ConstRecord`
/// reference, while non-legal outcomes fail with a structured
/// diagnostic (`NotConstantExpression` is a task error, `Unsupported`
/// is an explicit unsupported).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstantResult {
    /// Committed folded constant (meaningful for `Legal`).
    pub value: RecordRef,
    /// Evaluation legality.
    pub legality: ConstLegality,
}

impl ConstantResult {
    /// Route the outcome to exactly one terminal proposal for `task`.
    pub fn route(self, task: TaskId) -> Proposal {
        match self.legality {
            ConstLegality::Legal => Proposal::Complete {
                task,
                value: ResultValue::Record(self.value),
            },
            ConstLegality::NotConstantExpression => Proposal::Fail {
                task,
                diagnostic: DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 3),
                    "expression is not a constant expression",
                ),
            },
            ConstLegality::Unsupported => Proposal::Fail {
                task,
                diagnostic: DiagnosticDraft::unsupported(
                    "constant-expression construct outside the M1 subset",
                ),
            },
        }
    }
}

/// A registered continuation: the parse state a task needs to resume with
/// after its children complete. Continuations are mechanical; resume semantics
/// belong to the producing chip.
///
/// Frozen `/6` shape: exactly these nine ordered fields. The `/5` fields
/// `resume_kind` (now [`ContinuationRecord::production`]) and `awaited` are
/// superseded: awaited children live only in [`TaskState::Waiting`] around a
/// [`WaitSet`], never duplicated here (the T01 §4 supersession is catalogued
/// at `/6`; there is no `/5` edit). Parse state extends this record; no
/// competing `ParseContinuation` struct exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationRecord {
    /// Task kind the parent will resume with.
    pub production: TaskKind,
    /// Committed token cursor where parsing resumes (the EOF token is allowed).
    pub cursor: TokenId,
    /// Parser context of this frame, semantically distinct from `production`.
    pub context: ParseContext,
    /// Precedence level of this frame.
    pub binding_power: u16,
    /// Optional scope the continuation belongs to.
    pub scope: Option<ScopeId>,
    /// Committed parent node; `None` only for the translation-unit root frame.
    pub parent: Option<NodeId>,
    /// Committed child nodes produced so far, sorted by contiguous ordinals.
    pub partial_children: Vec<NodeId>,
    /// Uniqueness source for the next child ordinal (`max(ordinal) + 1`).
    pub next_child_ordinal: u32,
    /// Committed predecessor frame; frames form a committed-only acyclic chain.
    pub previous: Option<ContinuationId>,
}

/// Parser context of a [`ContinuationRecord`] frame.
///
/// Closed 10-member vocabulary with explicit discriminants 0-9 in declaration
/// order. `context` is semantically distinct from the frame's
/// [`ContinuationRecord::production`] task kind; no mapping between contexts
/// and task kinds is derived here (the context-to-chip mapping is a T05
/// co-freeze item).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ParseContext {
    /// Top-level translation unit frame.
    TranslationUnit = 0,
    /// External declaration frame.
    ExternalDecl = 1,
    /// Specifier frame.
    Specifier = 2,
    /// Declarator frame.
    Declarator = 3,
    /// Parameter list frame.
    ParameterList = 4,
    /// Block frame.
    Block = 5,
    /// Expression frame.
    Expression = 6,
    /// Assignment frame.
    Assignment = 7,
    /// Unary frame.
    Unary = 8,
    /// Primary frame.
    Primary = 9,
}

impl ParseContext {
    /// Every context in discriminant order.
    pub const ALL: [Self; 10] = [
        Self::TranslationUnit,
        Self::ExternalDecl,
        Self::Specifier,
        Self::Declarator,
        Self::ParameterList,
        Self::Block,
        Self::Expression,
        Self::Assignment,
        Self::Unary,
        Self::Primary,
    ];

    /// Explicit discriminant (declaration order, 0-9).
    pub const fn ordinal(self) -> u8 {
        self as u8
    }
}

/// Key of a draft inside one task's own [`AppendBatch`], in append order.
///
/// Scoped to a single task's single batch: a `DraftRef` never names another
/// task's draft. It is a transient wire/proposal input only, resolved to a
/// committed ID before any persistent state exists.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DraftRef(pub u32);

impl DraftRef {
    /// Raw position within the task's own batch.
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// A child task awaited by the same task that enqueued it.
///
/// `Committed` names a live committed child whose `Enqueue` record shows this
/// task as parent. `OwnBatch(k)` indexes this task's own `Enqueue` list in the
/// same batch (`k <` own enqueue count, same task only); a cross-task index is
/// unrepresentable. Own-batch keys are validated in the no-mutation pass
/// before apply and resolved to committed IDs before persistent state.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ChildRef {
    /// A live committed child task.
    Committed(TaskId),
    /// An index into this task's own `Enqueue` list in the same batch.
    OwnBatch(u32),
}

/// Reference to a continuation from an `Enqueue`.
///
/// `Committed` names a live committed continuation. `OwnBatch` names a draft
/// in this task's own [`AppendBatch`] draft range (which must have the
/// `Continuation` family). Own-batch keys are transient wire/proposal inputs
/// only: commit validates and resolves them to a committed [`ContinuationId`]
/// before any persistent state, and no durable cursor, [`WaitSet`], or join
/// may point at a draft, wire, or address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ContinuationRef {
    /// A live committed continuation.
    Committed(ContinuationId),
    /// A key into this task's own batch draft range.
    OwnBatch(DraftRef),
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
///
/// Frozen: this set is unchanged (no `DraftRecords` variant). A legal
/// constant result completes with `Record(RecordRef::Const)`; multi-record
/// results use `Records`; non-legal constant outcomes fail through the chip
/// diagnostic path instead of materializing a value; a parse cursor reuses
/// `ContinuationDraft.cursor`, not a new result variant.
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
    /// Number of `Progress` reinserts consumed (`max_task_progress` bound).
    pub progress_count: u32,
    /// Last accepted `Progress` ordinal (strictly increasing).
    pub progress_ordinal: u64,
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
    ///
    /// Frozen `/5` shape (`Option<ContinuationId>`) is kept byte-compatible.
    /// Carrying a [`ContinuationRef`] (in particular an own-batch draft key)
    /// requires migrating this field plus `Task.continuation` and the
    /// snapshot wire encoding, which is integrator-owned commit-track work;
    /// the [`ContinuationRef`] type itself is frozen here.
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

/// An ordered batch of typed record drafts appended by one task in one tick.
///
/// At most one batch per task per batch is allowed; an empty batch is
/// rejected at commit. The batch counts against
/// `limits.max_proposals_per_tick` together with the proposal count
/// (`proposals.len() + total_drafts`; the limit fields live in `limits.rs`,
/// owned by the limits track).
///
/// `records` carries the reservation handles in append order; `bodies`
/// carries the Gate 1 (`/7`) typed bodies 1:1 positional with `records`.
/// A length or family mismatch rejects the whole batch before any mutation.
/// Families outside [`G1DraftBody`] keep the explicit pending-records-track
/// rejection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppendBatch {
    /// Draft record reservation handles in append order.
    pub records: Vec<RecordDraft>,
    /// Typed draft bodies, 1:1 positional with `records` (Gate 1 `/7`).
    pub bodies: Vec<G1DraftBody>,
}

/// A proposal emitted by a worker during a tick.
///
/// Coexistence (enforced by the commit track, which owns the checks):
///
/// - Per dispatched task per batch, exactly one of `Complete`, `Fail`,
///   `AwaitHost`, `AwaitChildren`, `Progress` (see [`Proposal::is_transition`]);
///   a dispatched task with an empty proposal vector fails per-task rather
///   than completing silently or staying `Running`.
/// - At most one `AppendRecords` per task per batch; `Enqueue` may accompany
///   any transition (a task may spawn children and then await them) and
///   `StorePatch` may accompany a transition.
/// - `AppendRecords` and a `StorePatch` append must not target the same
///   `(store, field)`; `Progress`/`AwaitChildren` are incompatible with
///   `Complete`/`Fail`/`AwaitHost`.
/// - `AwaitChildren.children` entries of the `OwnBatch(k)` form must satisfy
///   `k <` this task's own `Enqueue` count (same task only).
/// - `Payload` stays [`RecordRef`]-only; cross-task committed references are
///   the normal data flow while cross-task draft references are
///   unrepresentable.
///
/// Wire tags: `Enqueue` = 0, `Complete` = 1, `Fail` = 2, `AwaitHost` = 3,
/// `StorePatch` = 4 (frozen `/5`, unchanged), `AppendRecords` = 5,
/// `Progress` = 6, `AwaitChildren` = 7.
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
    ///
    /// Field-scoped ownership is registration-time: the
    /// (`ChipId`, `StoreId`, field, kind) allowlist and its
    /// `ManifestError::StoreOwnerViolation { chip, store, field, expected_kind }`
    /// enforcement live in `manifest.rs` (manifest track), never in the
    /// commit. A second chip writer to `tasks.ready` is rejected there.
    StorePatch(StorePatch),
    /// Append typed record drafts owned by this task, in one atomic batch.
    ///
    /// Validated and resolved in the no-mutation pass before apply; apply
    /// performs only infallible pushes. Per-arena capacity is checked per
    /// backing family against `limits.max_records_per_arena` (limits track);
    /// names are bounded only by the intern limits.
    AppendRecords {
        /// Task owning the batch.
        task: TaskId,
        /// Drafts in append order.
        batch: AppendBatch,
    },
    /// Reschedule the task for the next tick without completing it.
    ///
    /// Reinserts exactly once into the task's own stage queue with
    /// `ready_tick = tick + 1`. The ordinal must advance every reschedule;
    /// exceeding `limits.max_task_progress` (limits track; zero disables
    /// `Progress`) is a per-task failure, mutually exclusive with reinsert.
    Progress {
        /// Task being rescheduled.
        task: TaskId,
        /// Monotonic progress ordinal.
        ordinal: u64,
    },
    /// Block the selected task until its children are terminal.
    ///
    /// Sets `Waiting(WaitSet { children, host_request: None })`; the existing
    /// [`WaitSet`] is the sole awaited-child-ID source. The join itself is
    /// commit-apply work owned by the commit track.
    AwaitChildren {
        /// Task being blocked.
        task: TaskId,
        /// Children awaited: committed children or own-batch enqueue indices.
        children: Vec<ChildRef>,
    },
}

impl Proposal {
    /// Frozen wire tag, in declaration order.
    pub const fn wire_tag(&self) -> u8 {
        match self {
            Self::Enqueue(_) => 0,
            Self::Complete { .. } => 1,
            Self::Fail { .. } => 2,
            Self::AwaitHost { .. } => 3,
            Self::StorePatch(_) => 4,
            Self::AppendRecords { .. } => 5,
            Self::Progress { .. } => 6,
            Self::AwaitChildren { .. } => 7,
        }
    }

    /// Whether this proposal is one of the five per-task transitions
    /// (`Complete`, `Fail`, `AwaitHost`, `AwaitChildren`, `Progress`).
    /// Exactly one transition per dispatched task per batch is required.
    pub const fn is_transition(&self) -> bool {
        matches!(
            self,
            Self::Complete { .. }
                | Self::Fail { .. }
                | Self::AwaitHost { .. }
                | Self::AwaitChildren { .. }
                | Self::Progress { .. }
        )
    }
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
    /// Interned names (`InternTable`-backed; field `entries`). Frozen index 20.
    ///
    /// This store exists so the manifest write-authorization model (keyed by
    /// `StoreId`/field) can authorize and audit name interning like every
    /// other append family. It maps to the `InternTable`, not to an arena;
    /// its version slot bumps only when at least one genuinely new name is
    /// interned (commit-track behavior).
    Names,
}

impl StoreId {
    /// Every store in canonical order.
    pub const ALL: [StoreId; 21] = [
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
        StoreId::Names,
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
            Self::Names => "names",
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
            Self::Names => 20,
        }
    }

    /// Rebuild from a canonical index.
    pub const fn from_index(index: usize) -> Option<Self> {
        if index < 21 {
            Some(Self::ALL[index])
        } else {
            None
        }
    }
}

/// The full set of ID types declared by this contract, in canonical order.
///
/// Used by the frozen schema hash and by manifest/registry documentation.
/// Labels 0-23 are the frozen `/5` inventory, unchanged; the `/6` appends
/// follow in wire-tag order (`literals` = 24, `sem` = 25, `scope_events` =
/// 26). See the wire-tag note on [`RecordRef::wire_tag`].
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
    "literals",
    "sem",
    "scope_events",
];
