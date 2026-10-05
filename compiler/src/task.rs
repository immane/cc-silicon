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
    /// Wave 2 (`/10`) PP01 slice kind: T03 source-normalize task (payload:
    /// exactly one `RecordRef::Source`; completes with
    /// `Record(RecordRef::Artifact)` carrying the single-source `Normalized`
    /// artifact, or `Fail` with a typed diagnostic).
    pub const PREPROCESS_NORMALIZE: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 16);
    /// Wave 2 (`/11`) LX-slice kind: T04 name-intern task (payload: PP-token
    /// refs in source order; appends one `Name` body per first-seen
    /// identifier spelling; completes `Ack`).
    pub const LEX_INTERN: Self = Self(((TaskGroup::LEX.0 as u16) << Self::LOCAL_BITS) | 16);
    /// Wave 2 (`/11`) LX-slice kind: T04 token-classify task (payload: PP-token
    /// refs in source order; appends one `Token` per ref in order; completes
    /// `Records` of the appended token refs).
    pub const LEX_CLASSIFY: Self = Self(((TaskGroup::LEX.0 as u16) << Self::LOCAL_BITS) | 17);
    /// Wave 2 (`/11`) LX-slice kind: T04 literal-decode task (payload:
    /// exactly `[Token, PpToken]`; appends one `Literal` with
    /// `token: Some(committed TokenId)`; completes `Record`).
    pub const LEX_DECODE_LITERAL: Self = Self(((TaskGroup::LEX.0 as u16) << Self::LOCAL_BITS) | 18);
    /// Wave 2 (`/12`) PA-slice kind: T05 TU-parse task (payload: the 13 M1
    /// token refs in source order; appends the nine-node M1 tree in
    /// pre-order; completes `Record` of the TU root).
    pub const PARSE_TU: Self = Self(((TaskGroup::PARSE.0 as u16) << Self::LOCAL_BITS) | 16);
    /// Wave 2 (`/13`) TY-slice kind: canonical `int` producer (payload:
    /// empty; reuses the committed canonical id when present, else appends
    /// one `TypeRecord`; completes `Record`).
    pub const SYMBOL_INT_TYPE: Self =
        Self(((TaskGroup::SYMBOL_TYPE.0 as u16) << Self::LOCAL_BITS) | 16);
    /// Wave 2 (`/13`) TY-slice kind: `int(void)` producer (payload: exactly
    /// one `RecordRef::Type` result type; reuses or appends; completes
    /// `Record`).
    pub const SYMBOL_FUNC_TYPE: Self =
        Self(((TaskGroup::SYMBOL_TYPE.0 as u16) << Self::LOCAL_BITS) | 17);
    /// Wave 2 (`/13`) TY-slice kind: scope enter (payload: exactly one
    /// `RecordRef::Node`, the TU root for file scope or the `Block` node
    /// for a body scope; appends one `Scope` plus one `ScopeEvent(Enter)`;
    /// completes `Record`).
    pub const SYMBOL_SCOPE_ENTER: Self =
        Self(((TaskGroup::SYMBOL_TYPE.0 as u16) << Self::LOCAL_BITS) | 18);
    /// Wave 2 (`/13`) TY-slice kind: scope exit (payload: exactly one
    /// `RecordRef::Scope`; appends one `ScopeEvent(Exit)`; completes `Ack`).
    pub const SYMBOL_SCOPE_EXIT: Self =
        Self(((TaskGroup::SYMBOL_TYPE.0 as u16) << Self::LOCAL_BITS) | 19);
    /// Wave 2 (`/13`) TY-slice kind: function declaration (payload: exactly
    /// `[Declarator Node, FuncType, FileScope]`; appends one `SymbolRecord`;
    /// completes `Record`).
    pub const SYMBOL_DECLARE: Self =
        Self(((TaskGroup::SYMBOL_TYPE.0 as u16) << Self::LOCAL_BITS) | 20);
    /// Wave 2 (`/13`) TY-slice kind: ordinary-name lookup (payload: exactly
    /// `[Scope, Name]`; completes `Record` on hit, typed `Fail` on miss).
    pub const SYMBOL_LOOKUP: Self =
        Self(((TaskGroup::SYMBOL_TYPE.0 as u16) << Self::LOCAL_BITS) | 21);
    /// Wave 2 (`/13`) TY-slice kind: integer promotion, M1 identity-only
    /// (payload: exactly one `RecordRef::Type`, must be `Int`; completes
    /// `Record` of the same id; non-`Int` is explicit `Unsupported`).
    pub const SYMBOL_PROMOTE: Self =
        Self(((TaskGroup::SYMBOL_TYPE.0 as u16) << Self::LOCAL_BITS) | 22);
    /// Wave 2 (`/13`) TY-slice kind: usual arithmetic conversion, M1
    /// identity-only (payload: exactly two `RecordRef::Type`, both `Int`;
    /// completes `Record`).
    pub const SYMBOL_COMMON_TYPE: Self =
        Self(((TaskGroup::SYMBOL_TYPE.0 as u16) << Self::LOCAL_BITS) | 23);
    /// Wave 2 (`/13`) TY-slice kind: return conversion, M1 identity-only
    /// (payload: exactly `[source Type, destination Type]`, both `Int`;
    /// completes `Record`).
    pub const SYMBOL_RETURN_CONVERT: Self =
        Self(((TaskGroup::SYMBOL_TYPE.0 as u16) << Self::LOCAL_BITS) | 24);
    /// Wave 2 (`/14`) SE-slice kind: literal-expression check (payload:
    /// exactly one `IntLiteral` node; appends one `SemRecord`; completes
    /// `Record`).
    pub const SEMANTIC_LITERAL_EXPR: Self =
        Self(((TaskGroup::SEMANTIC.0 as u16) << Self::LOCAL_BITS) | 18);
    /// Wave 2 (`/14`) SE-slice kind: binary-expression check (payload:
    /// exactly one `BinaryAdd` node; appends one `SemRecord`, enqueues one
    /// `const_fold` child with forwarded refs, and awaits it; on resume
    /// completes `Record` of the committed `SemRecord`).
    pub const SEMANTIC_BINARY_EXPR: Self =
        Self(((TaskGroup::SEMANTIC.0 as u16) << Self::LOCAL_BITS) | 19);
    /// Wave 2 (`/14`) SE-slice kind: return-statement check (payload:
    /// exactly one `Return` node; appends one `SemRecord`; completes
    /// `Record`).
    pub const SEMANTIC_RETURN_STMT: Self =
        Self(((TaskGroup::SEMANTIC.0 as u16) << Self::LOCAL_BITS) | 20);
    /// Wave 2 (`/14`) VF06 verifier kind: typed-AST invariant (payload:
    /// exactly one TU node; checks the M1 checked set; completes `Ack`).
    pub const VERIFICATION_TYPED_INVARIANT: Self =
        Self(((TaskGroup::VERIFICATION.0 as u16) << Self::LOCAL_BITS) | 16);
    /// Wave 2 (`/15`) IR-slice kind: whole-function lowering (payload:
    /// exactly one `FunctionDefinition` node; appends one `Function`, one
    /// `Block`, one `Value`, and two `Instruction`s in order; completes
    /// `Record` of the function).
    pub const IR_FUNCTION: Self = Self(((TaskGroup::IR_LOWER.0 as u16) << Self::LOCAL_BITS) | 16);
    /// Wave 2 (`/16`) PP-slice kind: line-splice task (payload: exactly
    /// one `Normalized` or `Spliced` artifact; appends one `Spliced`
    /// artifact with the composed map; completes `Record`).
    pub const PREPROCESS_SPLICE: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 17);
    /// Wave 2 (`/16`) PP-slice kind: comment-replace task (payload: exactly
    /// one `Spliced` artifact; appends one `CommentFree` artifact; inputs
    /// containing string/character literals are explicit `Unsupported`;
    /// completes `Record`).
    pub const PREPROCESS_COMMENT: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 18);
    /// Wave 2 (`/16`) PP-slice kind: token-scan task (payload: exactly one
    /// `CommentFree` artifact; appends spans plus PP tokens in order;
    /// completes `Records`).
    pub const PREPROCESS_SCAN: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 19);
    /// Wave 2 (`/21`) PP-directive-slice kind: directive-dispatch task
    /// (payload: all committed pp-token refs; groups lines, fans out one
    /// diagnostic child per `#error` line and awaits them; any other
    /// directive fails fast as explicit `Unsupported`; completes `Ack`
    /// when no directive line exists).
    pub const PREPROCESS_DIRECTIVE: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 20);
    /// Wave 2 (`/21`) PP-directive-slice kind: directive-diagnostic task
    /// (payload: one post-`#` directive line's refs; `#error` lines only;
    /// always fails with the joined message — the diagnostic is the
    /// product).
    pub const PREPROCESS_DIAGNOSTIC: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 21);
    /// Wave 2 (`/22`) PP-conditional-slice kind: conditional-inclusion task
    /// (payload: all committed pp-token refs; tracks the conditional stack
    /// with a chip-local PP-int evaluator and completes `Records` of the
    /// active-line refs, active directive lines included).
    pub const PREPROCESS_CONDITIONAL: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 22);
    /// Wave 2 (`/23`) PP-macro-slice kind: macro-definition task (payload:
    /// one post-`#` `#define` line's refs; appends one fresh `MacroRecord`
    /// or fans out one redefine child and awaits it; completes `Record` or
    /// `Ack`).
    pub const PREPROCESS_MACRO_DEFINE: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 23);
    /// Wave 2 (`/23`) PP-macro-slice kind: macro-redefinition check task
    /// (payload: one post-`#` line's refs plus exactly one committed
    /// incumbent `Macro` ref; benign equivalence completes `Ack`,
    /// anything else fails naming the difference).
    pub const PREPROCESS_MACRO_REDEFINE: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 24);
    /// Wave 2 (`/23`) PP-macro-slice kind: macro-undef task (payload: one
    /// post-`#` `#undef` line's refs; appends one tombstone or
    /// acknowledges the ignore).
    pub const PREPROCESS_MACRO_UNDEF: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 25);
    /// Wave 2 (`/24`) PP-expand-slice kind: macro-invocation task
    /// (payload: all active pp-token refs; fans out one substitute child
    /// per top-level invocation and stitches the expanded stream).
    pub const PREPROCESS_MACRO_INVOKE: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 26);
    /// Wave 2 (`/24`) PP-expand-slice kind: macro-substitution task
    /// (payload: one `Macro` def ref plus invocation refs; substitutes
    /// with argument prescan, `#`/`##`, and blue-paint rescan).
    pub const PREPROCESS_MACRO_SUBSTITUTE: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 27);
    /// Wave 2 (`/25`) PP-include-slice kind: include-resolve task
    /// (payload: exactly one `HeaderName` ref; pure lookup over committed
    /// sources; completes `Record(Source)` or fails not-loaded).
    pub const PREPROCESS_INCLUDE_RESOLVE: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 28);
    /// Wave 2 (`/25`) PP-include-slice kind: include-enter task (payload:
    /// full-stream pp-token refs plus exactly one trailing `Source` ref;
    /// splices one level of header tokens, keeps everything else verbatim).
    pub const PREPROCESS_INCLUDE_ENTER: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 29);
    /// Wave 2 (`/26`) PP-variadic-slice kind: variadic-macro task (single
    /// kind for both payload shapes: stream mode carries all active
    /// pp-token refs and fans out one single-mode child per variadic
    /// invocation; single mode carries one `Macro` def ref plus
    /// invocation refs and substitutes with `__VA_ARGS__` collection,
    /// `__VA_OPT__` policy, `#`/`##`, and blue-paint rescan).
    pub const PREPROCESS_VARIADIC_MACRO: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 30);
    /// Wave 2 (`/27`) PP-builtin-slice kind: builtin-macro task (payload:
    /// exactly one `RecordRef::PpToken` naming the builtin use, which must
    /// be an `Identifier`; expands to exactly one synthesized `PpToken`
    /// and completes `Records`; unknown or unconfigured names fail as
    /// explicit `Unsupported`).
    pub const PREPROCESS_MACRO_BUILTIN: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 31);
    /// Wave 3 (`/28`) PP-line-slice kind: line-directive task (payload: one
    /// directive line's pp-token refs in payload order, either `#line
    /// number "file"?` or a GNU `# lineno "file" flags?` marker;
    /// validates the logical location and completes `Ack`; malformed
    /// input fails as a typed `Invalid`).
    pub const PREPROCESS_LINE_DIRECTIVE: Self =
        Self(((TaskGroup::PREPROCESS.0 as u16) << Self::LOCAL_BITS) | 32);
    /// Wave 2 (`/17`) VF12-slice kind: symbolic IR interpret task (payload:
    /// exactly one committed `Function`; walks the M1 covered subset
    /// without executing target code; completes `Record` of the modeled
    /// `Const`; non-covered shapes fail, never pass).
    pub const VERIFICATION_IR_INTERPRET: Self =
        Self(((TaskGroup::VERIFICATION.0 as u16) << Self::LOCAL_BITS) | 17);
    /// Wave 2 (`/18`) VF05-slice kind: token-AST invariant task (payload:
    /// exactly one TU node; checks M1 ranges/order/parent-kind and required
    /// fields; completes `Ack`).
    pub const VERIFICATION_TOKEN_AST_INVARIANT: Self =
        Self(((TaskGroup::VERIFICATION.0 as u16) << Self::LOCAL_BITS) | 18);
    /// Wave 2 (`/19`) VF01-slice kind: store-invariant task (payload must
    /// be empty; checks the global M1 store contract; completes `Ack`).
    pub const VERIFICATION_STORE_INVARIANT: Self =
        Self(((TaskGroup::VERIFICATION.0 as u16) << Self::LOCAL_BITS) | 19);
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

    /// The Wave 2 (`/19`) VF01-slice registry: the VF05 slice plus the
    /// frozen store-invariant kind (all `Frozen`; `VERIFICATION` owners
    /// start new codes at local 20).
    pub fn vf01_slice() -> Self {
        let mut registry = Self::vf05_slice();
        let slice: &[(TaskKind, &str)] = &[(
            TaskKind::VERIFICATION_STORE_INVARIANT,
            "verification.store_invariant",
        )];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/18`) VF05-slice registry: the VF12 slice plus the
    /// frozen token-AST invariant kind (all `Frozen`; `VERIFICATION`
    /// owners start new codes at local 19).
    pub fn vf05_slice() -> Self {
        let mut registry = Self::vf12_slice();
        let slice: &[(TaskKind, &str)] = &[(
            TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
            "verification.token_ast_invariant",
        )];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/17`) VF12-slice registry: the PP slice plus the frozen
    /// symbolic-interpret kind (all `Frozen`; `VERIFICATION` owners start
    /// new codes at local 18).
    pub fn vf12_slice() -> Self {
        let mut registry = Self::pp_slice();
        let slice: &[(TaskKind, &str)] = &[(
            TaskKind::VERIFICATION_IR_INTERPRET,
            "verification.ir_interpret",
        )];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 3 (`/28`) PP-line-slice registry: the PP-builtin
    /// slice plus the frozen line-directive kind (all `Frozen`;
    /// `PREPROCESS` owners start new codes at local 33).
    pub fn pp_line_slice() -> Self {
        let mut registry = Self::pp_builtin_slice();
        let slice: &[(TaskKind, &str)] = &[(
            TaskKind::PREPROCESS_LINE_DIRECTIVE,
            "preprocess.line_directive",
        )];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/27`) PP-builtin-slice registry: the PP-variadic
    /// slice plus the frozen builtin-macro kind (all `Frozen`;
    /// `PREPROCESS` owners start new codes at local 32).
    pub fn pp_builtin_slice() -> Self {
        let mut registry = Self::pp_variadic_slice();
        let slice: &[(TaskKind, &str)] = &[(
            TaskKind::PREPROCESS_MACRO_BUILTIN,
            "preprocess.macro_builtin",
        )];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/26`) PP-variadic-slice registry: the PP-include
    /// slice plus the frozen variadic-macro kind (all `Frozen`;
    /// `PREPROCESS` owners start new codes at local 31).
    pub fn pp_variadic_slice() -> Self {
        let mut registry = Self::pp_include_slice();
        let slice: &[(TaskKind, &str)] = &[(
            TaskKind::PREPROCESS_VARIADIC_MACRO,
            "preprocess.variadic_macro",
        )];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/25`) PP-include-slice registry: the PP-expand
    /// slice plus the frozen include kinds (all `Frozen`; `PREPROCESS`
    /// owners start new codes at local 30).
    pub fn pp_include_slice() -> Self {
        let mut registry = Self::pp_expand_slice();
        let slice: &[(TaskKind, &str)] = &[
            (
                TaskKind::PREPROCESS_INCLUDE_RESOLVE,
                "preprocess.include_resolve",
            ),
            (
                TaskKind::PREPROCESS_INCLUDE_ENTER,
                "preprocess.include_enter",
            ),
        ];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/24`) PP-expand-slice registry: the PP-macro
    /// slice plus the frozen invoke/substitute kinds (all `Frozen`;
    /// `PREPROCESS` owners start new codes at local 28).
    pub fn pp_expand_slice() -> Self {
        let mut registry = Self::pp_macro_slice();
        let slice: &[(TaskKind, &str)] = &[
            (TaskKind::PREPROCESS_MACRO_INVOKE, "preprocess.macro_invoke"),
            (
                TaskKind::PREPROCESS_MACRO_SUBSTITUTE,
                "preprocess.macro_substitute",
            ),
        ];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/23`) PP-macro-slice registry: the PP-conditional
    /// slice plus the frozen macro kinds (all `Frozen`; `PREPROCESS`
    /// owners start new codes at local 26).
    pub fn pp_macro_slice() -> Self {
        let mut registry = Self::pp_conditional_slice();
        let slice: &[(TaskKind, &str)] = &[
            (TaskKind::PREPROCESS_MACRO_DEFINE, "preprocess.macro_define"),
            (
                TaskKind::PREPROCESS_MACRO_REDEFINE,
                "preprocess.macro_redefine",
            ),
            (TaskKind::PREPROCESS_MACRO_UNDEF, "preprocess.macro_undef"),
        ];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/22`) PP-conditional-slice registry: the PP-directive
    /// slice plus the frozen conditional kind (all `Frozen`; `PREPROCESS`
    /// owners start new codes at local 23).
    pub fn pp_conditional_slice() -> Self {
        let mut registry = Self::pp_directive_slice();
        let slice: &[(TaskKind, &str)] =
            &[(TaskKind::PREPROCESS_CONDITIONAL, "preprocess.conditional")];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/21`) PP-directive-slice registry: the VF01 slice plus
    /// the frozen directive/diagnostic kinds (all `Frozen`; `PREPROCESS`
    /// owners start new codes at local 22).
    pub fn pp_directive_slice() -> Self {
        let mut registry = Self::vf01_slice();
        let slice: &[(TaskKind, &str)] = &[
            (TaskKind::PREPROCESS_DIRECTIVE, "preprocess.directive"),
            (TaskKind::PREPROCESS_DIAGNOSTIC, "preprocess.diagnostic"),
        ];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/16`) PP-slice registry: the IR slice plus the three
    /// frozen PP kinds (all `Frozen`; `PREPROCESS` owners start new codes
    /// at local 20).
    pub fn pp_slice() -> Self {
        let mut registry = Self::ir_slice();
        let slice: &[(TaskKind, &str)] = &[
            (TaskKind::PREPROCESS_SPLICE, "preprocess.splice"),
            (TaskKind::PREPROCESS_COMMENT, "preprocess.comment"),
            (TaskKind::PREPROCESS_SCAN, "preprocess.scan"),
        ];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/15`) IR-slice registry: the SE slice plus the frozen
    /// function kind (all `Frozen`; `IR_LOWER` owners start new codes at
    /// local 17).
    pub fn ir_slice() -> Self {
        let mut registry = Self::se_slice();
        let slice: &[(TaskKind, &str)] = &[(TaskKind::IR_FUNCTION, "ir.function")];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/14`) SE-slice registry: the TY slice plus the three
    /// frozen SE kinds and the VF06 verifier kind (all `Frozen`;
    /// `SEMANTIC`/`VERIFICATION` owners start new codes at local 19/17).
    pub fn se_slice() -> Self {
        let mut registry = Self::ty_slice();
        let slice: &[(TaskKind, &str)] = &[
            (TaskKind::SEMANTIC_LITERAL_EXPR, "semantic.literal_expr"),
            (TaskKind::SEMANTIC_BINARY_EXPR, "semantic.binary_expr"),
            (TaskKind::SEMANTIC_RETURN_STMT, "semantic.return_stmt"),
            (
                TaskKind::VERIFICATION_TYPED_INVARIANT,
                "verification.typed_invariant",
            ),
        ];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/13`) TY-slice registry: the PA slice plus the nine
    /// frozen TY kinds (all `Frozen`; `SYMBOL_TYPE` owners start new codes
    /// at local 25).
    pub fn ty_slice() -> Self {
        let mut registry = Self::pa_slice();
        let slice: &[(TaskKind, &str)] = &[
            (TaskKind::SYMBOL_INT_TYPE, "symbol_type.int_type"),
            (TaskKind::SYMBOL_FUNC_TYPE, "symbol_type.func_type"),
            (TaskKind::SYMBOL_SCOPE_ENTER, "symbol_type.scope_enter"),
            (TaskKind::SYMBOL_SCOPE_EXIT, "symbol_type.scope_exit"),
            (TaskKind::SYMBOL_DECLARE, "symbol_type.declare"),
            (TaskKind::SYMBOL_LOOKUP, "symbol_type.lookup"),
            (TaskKind::SYMBOL_PROMOTE, "symbol_type.promote"),
            (TaskKind::SYMBOL_COMMON_TYPE, "symbol_type.common_type"),
            (
                TaskKind::SYMBOL_RETURN_CONVERT,
                "symbol_type.return_convert",
            ),
        ];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/12`) PA-slice registry: the LX slice plus the frozen
    /// TU kind (all `Frozen`; `PARSE` owners start new codes at local 17).
    pub fn pa_slice() -> Self {
        let mut registry = Self::lx_slice();
        let slice: &[(TaskKind, &str)] = &[(TaskKind::PARSE_TU, "parse.translation_unit")];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/11`) LX-slice registry: the PP01 slice plus the three
    /// frozen LX kinds (all `Frozen`; `LEX` owners start new codes at
    /// local 19).
    pub fn lx_slice() -> Self {
        let mut registry = Self::pp01_slice();
        let slice: &[(TaskKind, &str)] = &[
            (TaskKind::LEX_INTERN, "lex.intern"),
            (TaskKind::LEX_CLASSIFY, "lex.classify"),
            (TaskKind::LEX_DECODE_LITERAL, "lex.decode_literal"),
        ];
        for &(kind, name) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = registry.register(kind, name, kind.group(), KindStatus::Frozen);
        }
        registry
    }

    /// The Wave 2 (`/10`) PP01 slice registry: the Gate 1 M1 slice plus
    /// the frozen PP01 kind (all `Frozen`; `PREPROCESS` owners start new
    /// codes at local 17).
    pub fn pp01_slice() -> Self {
        let mut registry = Self::m1_slice();
        let slice: &[(TaskKind, &str)] =
            &[(TaskKind::PREPROCESS_NORMALIZE, "preprocess.normalize")];
        for &(kind, name) in slice {
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
            (
                TaskKind::CONSTANT_CONST_FOLD,
                "constant_layout_init.const_fold",
            ),
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
                write!(
                    f,
                    "task kind {} is not a const-evaluate request",
                    kind.raw()
                )
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

/// A decoded sem-stage constant-evaluate request (`/8` strict kind-shape
/// rule).
///
/// The wire form is `(TaskKind, Payload)` only — [`Payload`] stays
/// [`RecordRef`]-only by protocol rule, so `required_kind` and `op` travel
/// as no bytes: the M1 slice kinds each imply
/// [`RequiredKind::IntegerConstantExpression`], and the binary form
/// implies [`ConstExprOp::Add`] (the T07-checked operator).
///
/// Strict kind→shape rule (`/8`, supersedes the `/7`-era shape-only
/// relaxation): `const_eval_literal` carries exactly one
/// `RecordRef::Literal`; `const_eval_binary` carries `RecordRef::Node`
/// then two `RecordRef::Literal` in source order; `const_fold` accepts
/// either shape because the T07 requester forwards identical payload refs
/// to its fold child. A literal-shaped payload on the binary kind (or vice
/// versa) is `Arity`, never a silent reinterpretation.
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
    /// Decode a `(kind, payload)` pair per the frozen `/8` convention:
    /// `const_eval_literal` decodes only the single-literal shape,
    /// `const_eval_binary` decodes only the node-plus-two-literals shape,
    /// and `const_fold` accepts either forwarded shape.
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
        fn decode_literal(
            kind: TaskKind,
            payload: &Payload,
        ) -> Result<ConstantRequest, RequestError> {
            if payload.refs.len() != 1 {
                return Err(RequestError::Arity {
                    kind,
                    got: payload.refs.len(),
                });
            }
            Ok(ConstantRequest::Literal {
                literal: literal_at(kind, payload, 0)?,
                required_kind: RequiredKind::IntegerConstantExpression,
            })
        }
        fn decode_binary(
            kind: TaskKind,
            payload: &Payload,
        ) -> Result<ConstantRequest, RequestError> {
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
            Ok(ConstantRequest::Binary {
                node,
                op: ConstExprOp::Add,
                lhs: literal_at(kind, payload, 1)?,
                rhs: literal_at(kind, payload, 2)?,
                required_kind: RequiredKind::IntegerConstantExpression,
            })
        }
        match kind {
            TaskKind::SEMANTIC_CONST_EVAL_LITERAL => decode_literal(kind, payload),
            TaskKind::SEMANTIC_CONST_EVAL_BINARY => decode_binary(kind, payload),
            TaskKind::CONSTANT_CONST_FOLD => {
                if payload.refs.len() == 1 {
                    decode_literal(kind, payload)
                } else if payload.refs.len() == 3 {
                    decode_binary(kind, payload)
                } else {
                    Err(RequestError::Arity {
                        kind,
                        got: payload.refs.len(),
                    })
                }
            }
            _ => Err(RequestError::UnexpectedKind { kind }),
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
    "macros",
];
