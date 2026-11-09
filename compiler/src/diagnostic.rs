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
    /// Semantic (name/type/scope) failures (`/13` TY slice: undeclared
    /// identifiers, redeclaration conflicts).
    Semantic,
    /// Explicitly unsupported capability.
    Unsupported,
    /// An internal invariant was violated (a bug, reported not panicked).
    Internal,
    // `/47` taxonomy append.
    /// Lexical/token or literal-decoding failure.
    Lex,
    /// Parser/grammar failure.
    Parse,
    /// Type construction/compatibility failure.
    Type,
    /// Constant/layout/initialization failure.
    Const,
    /// IR/CFG lowering or verification failure.
    Ir,
    /// Target code generation/ABI failure.
    Codegen,
    /// Host I/O, toolchain, or runner failure.
    Host,
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
            Self::Semantic => "semantic",
            Self::Unsupported => "unsupported",
            Self::Internal => "internal",
            Self::Lex => "lex",
            Self::Parse => "parse",
            Self::Type => "type",
            Self::Const => "const",
            Self::Ir => "ir",
            Self::Codegen => "codegen",
            Self::Host => "host",
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
    /// Explicitly unsupported capability (`/47` name for the legacy code).
    pub const UNSUPPORTED_CAPABILITY: Self = Self::new(DiagGroup::Unsupported, 1);
    /// Task protocol/shape fault (`/47` name for the legacy catch-all).
    pub const TASK_PROTOCOL: Self = Self::new(DiagGroup::Task, 4);
    /// A required committed record is missing.
    pub const TASK_MISSING_RECORD: Self = Self::new(DiagGroup::Task, 5);
    /// A dispatched task was not running.
    pub const TASK_NOT_RUNNING: Self = Self::new(DiagGroup::Task, 6);
    /// A handoff is ambiguous (zero/multiple candidates).
    pub const TASK_AMBIGUOUS: Self = Self::new(DiagGroup::Task, 7);
    /// Invalid lexical token.
    pub const LEX_INVALID_TOKEN: Self = Self::new(DiagGroup::Lex, 1);
    /// Literal decoding failure.
    pub const LEX_LITERAL_DECODE: Self = Self::new(DiagGroup::Lex, 2);
    /// Unexpected parser token.
    pub const PARSE_UNEXPECTED_TOKEN: Self = Self::new(DiagGroup::Parse, 1);
    /// Malformed declaration/expression shape.
    pub const PARSE_MALFORMED: Self = Self::new(DiagGroup::Parse, 2);
    /// Bounded parser recovery failure.
    pub const PARSE_RECOVERY: Self = Self::new(DiagGroup::Parse, 3);
    /// Unknown type/name.
    pub const TYPE_UNKNOWN: Self = Self::new(DiagGroup::Type, 1);
    /// Incompatible types.
    pub const TYPE_INCOMPATIBLE: Self = Self::new(DiagGroup::Type, 2);
    /// Undeclared identifier (`/47` name for the semantic group).
    pub const SEM_UNDECLARED: Self = Self::new(DiagGroup::Semantic, 1);
    /// Redeclaration conflict.
    pub const SEM_REDECLARATION: Self = Self::new(DiagGroup::Semantic, 2);
    /// Constant overflow under the `/47` taxonomy.
    pub const CONST_OVERFLOW_TAXONOMY: Self = Self::new(DiagGroup::Const, 1);
    /// Non-constant expression under the `/47` taxonomy.
    pub const CONST_NOT_CONSTANT_TAXONOMY: Self = Self::new(DiagGroup::Const, 2);
    /// Invalid IR/CFG shape.
    pub const IR_INVALID_SHAPE: Self = Self::new(DiagGroup::Ir, 1);
    /// Missing IR fact.
    pub const IR_MISSING_FACT: Self = Self::new(DiagGroup::Ir, 2);
    /// IR type mismatch.
    pub const IR_TYPE_MISMATCH: Self = Self::new(DiagGroup::Ir, 3);
    /// Target is not probe-verified.
    pub const CODEGEN_TARGET_UNVERIFIED: Self = Self::new(DiagGroup::Codegen, 1);
    /// Target operation unsupported.
    pub const CODEGEN_UNSUPPORTED: Self = Self::new(DiagGroup::Codegen, 2);
    /// Host I/O failure.
    pub const HOST_IO: Self = Self::new(DiagGroup::Host, 1);
    /// Host toolchain failure.
    pub const HOST_TOOLCHAIN: Self = Self::new(DiagGroup::Host, 2);
    /// Host timeout/cancellation.
    pub const HOST_TIMEOUT: Self = Self::new(DiagGroup::Host, 3);
}

/// One named diagnostic code in the frozen `/47` registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticCodeSpec {
    /// Stable name (hashed).
    pub name: &'static str,
    /// Structured code.
    pub code: DiagnosticCode,
}

/// The `/47` named diagnostic-code registry.
///
/// Legacy codes remain present with their existing names; new groups/codes are
/// append-only. This table freezes names and codes, not every call site: the
/// `Task:4` migration remains tracked separately.
pub const DIAGNOSTIC_CODES: &[DiagnosticCodeSpec] = &[
    DiagnosticCodeSpec {
        name: "unsupported.capability",
        code: DiagnosticCode::UNSUPPORTED_CAPABILITY,
    },
    DiagnosticCodeSpec {
        name: "task.protocol",
        code: DiagnosticCode::TASK_PROTOCOL,
    },
    DiagnosticCodeSpec {
        name: "task.missing-record",
        code: DiagnosticCode::TASK_MISSING_RECORD,
    },
    DiagnosticCodeSpec {
        name: "task.not-running",
        code: DiagnosticCode::TASK_NOT_RUNNING,
    },
    DiagnosticCodeSpec {
        name: "task.ambiguous",
        code: DiagnosticCode::TASK_AMBIGUOUS,
    },
    DiagnosticCodeSpec {
        name: "lex.invalid-token",
        code: DiagnosticCode::LEX_INVALID_TOKEN,
    },
    DiagnosticCodeSpec {
        name: "lex.literal-decode",
        code: DiagnosticCode::LEX_LITERAL_DECODE,
    },
    DiagnosticCodeSpec {
        name: "parse.unexpected-token",
        code: DiagnosticCode::PARSE_UNEXPECTED_TOKEN,
    },
    DiagnosticCodeSpec {
        name: "parse.malformed",
        code: DiagnosticCode::PARSE_MALFORMED,
    },
    DiagnosticCodeSpec {
        name: "parse.recovery",
        code: DiagnosticCode::PARSE_RECOVERY,
    },
    DiagnosticCodeSpec {
        name: "type.unknown",
        code: DiagnosticCode::TYPE_UNKNOWN,
    },
    DiagnosticCodeSpec {
        name: "type.incompatible",
        code: DiagnosticCode::TYPE_INCOMPATIBLE,
    },
    DiagnosticCodeSpec {
        name: "semantic.undeclared",
        code: DiagnosticCode::SEM_UNDECLARED,
    },
    DiagnosticCodeSpec {
        name: "semantic.redeclaration",
        code: DiagnosticCode::SEM_REDECLARATION,
    },
    DiagnosticCodeSpec {
        name: "const.overflow-taxonomy",
        code: DiagnosticCode::CONST_OVERFLOW_TAXONOMY,
    },
    DiagnosticCodeSpec {
        name: "const.not-constant-taxonomy",
        code: DiagnosticCode::CONST_NOT_CONSTANT_TAXONOMY,
    },
    DiagnosticCodeSpec {
        name: "ir.invalid-shape",
        code: DiagnosticCode::IR_INVALID_SHAPE,
    },
    DiagnosticCodeSpec {
        name: "ir.missing-fact",
        code: DiagnosticCode::IR_MISSING_FACT,
    },
    DiagnosticCodeSpec {
        name: "ir.type-mismatch",
        code: DiagnosticCode::IR_TYPE_MISMATCH,
    },
    DiagnosticCodeSpec {
        name: "codegen.target-unverified",
        code: DiagnosticCode::CODEGEN_TARGET_UNVERIFIED,
    },
    DiagnosticCodeSpec {
        name: "codegen.unsupported",
        code: DiagnosticCode::CODEGEN_UNSUPPORTED,
    },
    DiagnosticCodeSpec {
        name: "host.io",
        code: DiagnosticCode::HOST_IO,
    },
    DiagnosticCodeSpec {
        name: "host.toolchain",
        code: DiagnosticCode::HOST_TOOLCHAIN,
    },
    DiagnosticCodeSpec {
        name: "host.timeout",
        code: DiagnosticCode::HOST_TIMEOUT,
    },
    DiagnosticCodeSpec {
        name: "const.overflow-legacy",
        code: DiagnosticCode::CONST_OVERFLOW,
    },
    DiagnosticCodeSpec {
        name: "const.unsupported-legacy",
        code: DiagnosticCode::CONST_UNSUPPORTED,
    },
    DiagnosticCodeSpec {
        name: "const.not-constant-legacy",
        code: DiagnosticCode::CONST_NOT_CONSTANT_EXPRESSION,
    },
];

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

    /// A required committed record is missing.
    pub fn missing_record(message: impl Into<String>) -> Self {
        Self::error(DiagnosticCode::TASK_MISSING_RECORD, message)
    }

    /// A parser saw an unexpected token.
    pub fn parse_unexpected(message: impl Into<String>) -> Self {
        Self::error(DiagnosticCode::PARSE_UNEXPECTED_TOKEN, message)
    }

    /// Two types/records are incompatible.
    pub fn type_mismatch(message: impl Into<String>) -> Self {
        Self::error(DiagnosticCode::TYPE_INCOMPATIBLE, message)
    }

    /// An IR/CFG shape is invalid.
    pub fn ir_invalid(message: impl Into<String>) -> Self {
        Self::error(DiagnosticCode::IR_INVALID_SHAPE, message)
    }

    /// A host I/O operation failed.
    pub fn host_io(message: impl Into<String>) -> Self {
        Self::error(DiagnosticCode::HOST_IO, message)
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
