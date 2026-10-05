// ============================================================================
// diagnostic.rs — structured diagnostics and the error protocol (T01 C01/C03)
//
// Recoverable faults are values. Every error type in this crate can be mapped
// to a [`DiagnosticDraft`], which the commit chip turns into a stable
// [`crate::ids::DiagnosticId`] in ascending ordinal order. Panics are never
// used as semantic control flow.
// ============================================================================

use crate::ids::{SpanId, TaskId};

/// Diagnostic severity.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Severity {
    /// A recoverable error.
    Error,
    /// A warning.
    Warning,
    /// An informational note.
    Note,
}

impl Severity {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

/// Coarse diagnostic family. Concrete codes are structured, not stringly
/// typed, so tooling can match without parsing prose.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum DiagGroup {
    /// Task/proposal protocol violations.
    Protocol,
    /// Arena or capacity failures.
    Arena,
    /// Configuration and target failures.
    Config,
    /// Target probe/verification failures.
    Target,
    /// SFL manifest validation failures.
    Manifest,
    /// Task lifecycle failures.
    Task,
    /// Explicitly unsupported capability.
    Unsupported,
    /// An internal invariant was violated (a bug, reported not panicked).
    Internal,
}

impl DiagGroup {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Protocol => "protocol",
            Self::Arena => "arena",
            Self::Config => "config",
            Self::Target => "target",
            Self::Manifest => "manifest",
            Self::Task => "task",
            Self::Unsupported => "unsupported",
            Self::Internal => "internal",
        }
    }
}

/// A structured diagnostic code: family plus numeric code.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DiagnosticCode {
    /// Diagnostic family.
    pub group: DiagGroup,
    /// Numeric code within the family.
    pub code: u16,
}

impl DiagnosticCode {
    /// Build a diagnostic code.
    pub const fn new(group: DiagGroup, code: u16) -> Self {
        Self { group, code }
    }

    /// Chip diagnostic: a constant value overflows its representable range
    /// (T08 `Fail`/`DiagnosticDraft` path; never a `CommitError`).
    pub const CONST_OVERFLOW: Self = Self::new(DiagGroup::Unsupported, 2);
    /// Chip diagnostic: a constant expression uses an unsupported form (T08
    /// `Fail`/`DiagnosticDraft` path; never a `CommitError`).
    pub const CONST_UNSUPPORTED: Self = Self::new(DiagGroup::Unsupported, 3);
    /// Chip diagnostic: an expression is not a constant expression
    /// (`NotConstantExpression` legality family; T08 `Fail`/`DiagnosticDraft`
    /// path; never a `CommitError`).
    pub const CONST_NOT_CONSTANT_EXPRESSION: Self = Self::new(DiagGroup::Unsupported, 4);
}

/// A not-yet-committed diagnostic produced by a worker or the protocol layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticDraft {
    /// Severity.
    pub severity: Severity,
    /// Structured code.
    pub code: DiagnosticCode,
    /// Human-readable message.
    pub message: String,
    /// Optional source span.
    pub span: Option<SpanId>,
    /// Optional associated task.
    pub task: Option<TaskId>,
}

impl DiagnosticDraft {
    /// Build an error draft.
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            span: None,
            task: None,
        }
    }

    /// Build an unsupported-capability error draft.
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::error(DiagnosticCode::new(DiagGroup::Unsupported, 1), message)
    }

    /// Attach a source span.
    pub fn with_span(mut self, span: SpanId) -> Self {
        self.span = Some(span);
        self
    }

    /// Attach an owning task.
    pub fn with_task(mut self, task: TaskId) -> Self {
        self.task = Some(task);
        self
    }
}

/// A committed diagnostic with a stable ordinal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticRecord {
    /// Monotonic commit ordinal; defines output order.
    pub ordinal: u64,
    /// Severity.
    pub severity: Severity,
    /// Structured code.
    pub code: DiagnosticCode,
    /// Human-readable message.
    pub message: String,
    /// Optional source span.
    pub span: Option<SpanId>,
    /// Optional associated task.
    pub task: Option<TaskId>,
}

impl DiagnosticRecord {
    /// Build a record from a draft and a commit ordinal.
    pub fn commit(ordinal: u64, draft: DiagnosticDraft) -> Self {
        Self {
            ordinal,
            severity: draft.severity,
            code: draft.code,
            message: draft.message,
            span: draft.span,
            task: draft.task,
        }
    }
}
