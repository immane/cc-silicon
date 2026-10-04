// ============================================================================
// ids.rs — stable newtype identifiers (T01 C01)
//
// Every semantic record family owns a distinct ID type backed by a distinct
// arena. Stable newtypes prevent an ID from one arena being passed to another,
// and the arena never reuses an index (deletion leaves a tombstone). IDs are
// dense `u32` indices; `u32::MAX` is reserved as the "no record" sentinel and
// is never allocated because `Limits::max_records_per_arena` is far smaller.
// ============================================================================

use crate::arena::ArenaId;

/// Declare a stable newtype ID plus its [`ArenaId`] implementation.
macro_rules! define_ids {
    ($( $name:ident => $label:literal ),* $(,)?) => {
        $(
            #[doc = concat!("Stable identifier for the `", $label, "` arena.")]
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
            pub struct $name(u32);

            impl $name {
                /// Sentinel meaning "no record". Never produced by an arena.
                pub const NONE: Self = Self(u32::MAX);

                /// Build an ID from a raw arena index. Prefer arena allocation;
                /// this exists for decoding a serialized snapshot.
                pub const fn from_index(index: u32) -> Self {
                    Self(index)
                }

                /// The raw arena index.
                pub const fn index(self) -> u32 {
                    self.0
                }

                /// Whether this is the [`Self::NONE`] sentinel.
                pub const fn is_none(self) -> bool {
                    self.0 == u32::MAX
                }
            }

            impl ArenaId for $name {
                const LABEL: &'static str = $label;

                fn from_raw(raw: u32) -> Self {
                    Self(raw)
                }

                fn raw(self) -> u32 {
                    self.0
                }
            }
        )*
    };
}

define_ids! {
    SourceId => "sources",
    SpanId => "spans",
    ExpansionId => "expansions",
    PpTokenId => "pp_tokens",
    TokenId => "tokens",
    NameId => "names",
    ScopeId => "scopes",
    SymbolId => "symbols",
    TypeId => "types",
    NodeId => "nodes",
    ConstId => "constants",
    LayoutId => "layouts",
    InitId => "inits",
    FunctionId => "functions",
    BlockId => "blocks",
    ValueId => "values",
    InstructionId => "instructions",
    VRegId => "vregs",
    ContinuationId => "continuations",
    TaskId => "tasks",
    ResultId => "results",
    DiagnosticId => "diagnostics",
    HostRequestId => "host_requests",
    ArtifactId => "artifacts",
}

/// Identifier of a registered chip (a stateless transition unit).
///
/// Distinct from [`TaskId`]: a chip accepts many tasks over its lifetime. The
/// SFL manifest and every trace entry reference this ID.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ChipId(pub u16);

impl ChipId {
    /// Sentinel meaning "unassigned chip".
    pub const NONE: Self = Self(u16::MAX);

    /// Raw chip number.
    pub const fn index(self) -> u16 {
        self.0
    }
}

/// A tagged reference to a record in one of the typed arenas.
///
/// Used by task payloads, results, and store patches so that the protocol
/// never carries an untyped pointer or raw index.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RecordRef {
    /// A source file record.
    Source(SourceId),
    /// A source span record.
    Span(SpanId),
    /// A macro expansion record.
    Expansion(ExpansionId),
    /// A preprocessing token record.
    PpToken(PpTokenId),
    /// A C token record.
    Token(TokenId),
    /// An interned name record.
    Name(NameId),
    /// A scope record.
    Scope(ScopeId),
    /// A symbol record.
    Symbol(SymbolId),
    /// A canonical type record.
    Type(TypeId),
    /// A syntax/AST node record.
    Node(NodeId),
    /// A constant record.
    Const(ConstId),
    /// A layout descriptor record.
    Layout(LayoutId),
    /// An initialization plan record.
    Init(InitId),
    /// A function record.
    Function(FunctionId),
    /// A basic block record.
    Block(BlockId),
    /// An IR value record.
    Value(ValueId),
    /// An IR instruction record.
    Instruction(InstructionId),
    /// A virtual register record.
    VReg(VRegId),
    /// A continuation record.
    Continuation(ContinuationId),
    /// A task record.
    Task(TaskId),
    /// A result record.
    Result(ResultId),
    /// A diagnostic record.
    Diagnostic(DiagnosticId),
    /// A host request record.
    HostRequest(HostRequestId),
    /// An output artifact fragment record.
    Artifact(ArtifactId),
}

impl RecordRef {
    /// The name of the arena this reference points into.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Source(_) => "sources",
            Self::Span(_) => "spans",
            Self::Expansion(_) => "expansions",
            Self::PpToken(_) => "pp_tokens",
            Self::Token(_) => "tokens",
            Self::Name(_) => "names",
            Self::Scope(_) => "scopes",
            Self::Symbol(_) => "symbols",
            Self::Type(_) => "types",
            Self::Node(_) => "nodes",
            Self::Const(_) => "constants",
            Self::Layout(_) => "layouts",
            Self::Init(_) => "inits",
            Self::Function(_) => "functions",
            Self::Block(_) => "blocks",
            Self::Value(_) => "values",
            Self::Instruction(_) => "instructions",
            Self::VReg(_) => "vregs",
            Self::Continuation(_) => "continuations",
            Self::Task(_) => "tasks",
            Self::Result(_) => "results",
            Self::Diagnostic(_) => "diagnostics",
            Self::HostRequest(_) => "host_requests",
            Self::Artifact(_) => "artifacts",
        }
    }
}
