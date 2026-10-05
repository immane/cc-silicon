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
    // `/6` appends (frozen wire tags 24-26; declaration order below matches
    // tag order). Backing arenas are owned by the bus/arenas track, not by
    // this module:
    //   `LiteralId`     -> new `TypedArena<LiteralId, _>` (`Lex`, `literals`);
    //   `SemId`         -> new `TypedArena<SemId, _>` (`Sem`, `records`);
    //   `ScopeEventId`  -> new `TypedArena<ScopeEventId, _>`
    //                      (`Symbols`, `scope_events`).
    // `NameId` interning stays `InternTable`-backed (store `Names`,
    // `entries`); it is not an arena ID expansion.
    LiteralId => "literals",
    SemId => "sem",
    ScopeEventId => "scope_events",
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
    /// A T04-owned decoded literal record (raw lexical fact, not a constant
    /// value). Frozen wire tag 24.
    Literal(LiteralId),
    /// A semantic fact record (one per checked node). Frozen wire tag 25.
    Sem(SemId),
    /// A scope lifecycle event record (`Enter`/`Exit`). Frozen wire tag 26.
    ScopeEvent(ScopeEventId),
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
            Self::Literal(_) => "literals",
            Self::Sem(_) => "sem",
            Self::ScopeEvent(_) => "scope_events",
        }
    }

    /// Frozen wire tag of this reference, in declaration order.
    ///
    /// Tags 0-23 are the frozen `/5` inventory and are unchanged. The `/6`
    /// appends are `Literal` = 24, `Sem` = 25, `ScopeEvent` = 26.
    ///
    /// Tag-order note (integrator): the draft proposal text (`§5`) sketched
    /// `Sem` = 24, `ScopeEvent` = 25, `Literal` = 26. The frozen assignment
    /// here follows the authorizing freeze instruction instead
    /// (`Literal` = 24, `Sem` = 25, `ScopeEvent` = 26); this function is the
    /// single source of truth and the snapshot encoder mirrors these numbers.
    pub const fn wire_tag(self) -> u8 {
        match self {
            Self::Source(_) => 0,
            Self::Span(_) => 1,
            Self::Expansion(_) => 2,
            Self::PpToken(_) => 3,
            Self::Token(_) => 4,
            Self::Name(_) => 5,
            Self::Scope(_) => 6,
            Self::Symbol(_) => 7,
            Self::Type(_) => 8,
            Self::Node(_) => 9,
            Self::Const(_) => 10,
            Self::Layout(_) => 11,
            Self::Init(_) => 12,
            Self::Function(_) => 13,
            Self::Block(_) => 14,
            Self::Value(_) => 15,
            Self::Instruction(_) => 16,
            Self::VReg(_) => 17,
            Self::Continuation(_) => 18,
            Self::Task(_) => 19,
            Self::Result(_) => 20,
            Self::Diagnostic(_) => 21,
            Self::HostRequest(_) => 22,
            Self::Artifact(_) => 23,
            Self::Literal(_) => 24,
            Self::Sem(_) => 25,
            Self::ScopeEvent(_) => 26,
        }
    }

    /// The closed record family this reference points into.
    ///
    /// Family ordinals and wire tags are distinct inventories: do not derive
    /// one from the other by arithmetic.
    pub const fn family(self) -> RecordFamily {
        match self {
            Self::Source(_) => RecordFamily::Source,
            Self::Span(_) => RecordFamily::Span,
            Self::Expansion(_) => RecordFamily::Expansion,
            Self::PpToken(_) => RecordFamily::PpToken,
            Self::Token(_) => RecordFamily::Token,
            Self::Name(_) => RecordFamily::Name,
            Self::Scope(_) => RecordFamily::Scope,
            Self::Symbol(_) => RecordFamily::Symbol,
            Self::Type(_) => RecordFamily::Type,
            Self::Node(_) => RecordFamily::Node,
            Self::Const(_) => RecordFamily::Const,
            Self::Layout(_) => RecordFamily::Layout,
            Self::Init(_) => RecordFamily::Init,
            Self::Function(_) => RecordFamily::Function,
            Self::Block(_) => RecordFamily::Block,
            Self::Value(_) => RecordFamily::Value,
            Self::Instruction(_) => RecordFamily::Instruction,
            Self::VReg(_) => RecordFamily::VReg,
            Self::Continuation(_) => RecordFamily::Continuation,
            Self::Task(_) => RecordFamily::Task,
            Self::Result(_) => RecordFamily::Result,
            Self::Diagnostic(_) => RecordFamily::Diagnostic,
            Self::HostRequest(_) => RecordFamily::HostRequest,
            Self::Artifact(_) => RecordFamily::Artifact,
            Self::Literal(_) => RecordFamily::Literal,
            Self::Sem(_) => RecordFamily::Sem,
            Self::ScopeEvent(_) => RecordFamily::ScopeEvent,
        }
    }

    /// Build a reference from a family and a raw arena index (for decoding a
    /// serialized snapshot).
    pub const fn make(family: RecordFamily, index: u32) -> Self {
        match family {
            RecordFamily::Source => Self::Source(SourceId::from_index(index)),
            RecordFamily::Span => Self::Span(SpanId::from_index(index)),
            RecordFamily::Expansion => Self::Expansion(ExpansionId::from_index(index)),
            RecordFamily::PpToken => Self::PpToken(PpTokenId::from_index(index)),
            RecordFamily::Token => Self::Token(TokenId::from_index(index)),
            RecordFamily::Name => Self::Name(NameId::from_index(index)),
            RecordFamily::Scope => Self::Scope(ScopeId::from_index(index)),
            RecordFamily::Symbol => Self::Symbol(SymbolId::from_index(index)),
            RecordFamily::Type => Self::Type(TypeId::from_index(index)),
            RecordFamily::Node => Self::Node(NodeId::from_index(index)),
            RecordFamily::Const => Self::Const(ConstId::from_index(index)),
            RecordFamily::Layout => Self::Layout(LayoutId::from_index(index)),
            RecordFamily::Init => Self::Init(InitId::from_index(index)),
            RecordFamily::Function => Self::Function(FunctionId::from_index(index)),
            RecordFamily::Block => Self::Block(BlockId::from_index(index)),
            RecordFamily::Value => Self::Value(ValueId::from_index(index)),
            RecordFamily::Instruction => Self::Instruction(InstructionId::from_index(index)),
            RecordFamily::VReg => Self::VReg(VRegId::from_index(index)),
            RecordFamily::Continuation => Self::Continuation(ContinuationId::from_index(index)),
            RecordFamily::Task => Self::Task(TaskId::from_index(index)),
            RecordFamily::Result => Self::Result(ResultId::from_index(index)),
            RecordFamily::Diagnostic => Self::Diagnostic(DiagnosticId::from_index(index)),
            RecordFamily::HostRequest => Self::HostRequest(HostRequestId::from_index(index)),
            RecordFamily::Artifact => Self::Artifact(ArtifactId::from_index(index)),
            RecordFamily::Literal => Self::Literal(LiteralId::from_index(index)),
            RecordFamily::Sem => Self::Sem(SemId::from_index(index)),
            RecordFamily::ScopeEvent => Self::ScopeEvent(ScopeEventId::from_index(index)),
        }
    }
}

/// Closed family tag, total over every [`RecordRef`] variant (24 frozen `/5`
/// families plus `Literal`, `Sem`, and `ScopeEvent` = 27 variants).
///
/// Family ordinals and [`RecordRef::wire_tag`] wire tags are two distinct
/// closed inventories that are order-misaligned by design (for example
/// `Literal` is family ordinal 6 but wire tag 24, `Sem` is ordinal 12 but
/// wire tag 25, and `ScopeEvent` is ordinal 9 but wire tag 26; even frozen
/// families differ, e.g. `Name` is ordinal 3 but wire tag 5). No positional
/// derivation between the two is valid: convert only through the explicit
/// [`RecordRef::family`] / [`RecordRef::make`] matches.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RecordFamily {
    /// Source file family.
    Source = 0,
    /// Source span family.
    Span = 1,
    /// Macro expansion family.
    Expansion = 2,
    /// Interned name family (`InternTable`-backed, not an arena).
    Name = 3,
    /// Preprocessing token family.
    PpToken = 4,
    /// C token family.
    Token = 5,
    /// Decoded literal family.
    Literal = 6,
    /// Syntax/AST node family.
    Node = 7,
    /// Scope family.
    Scope = 8,
    /// Scope lifecycle event family.
    ScopeEvent = 9,
    /// Symbol family.
    Symbol = 10,
    /// Canonical type family.
    Type = 11,
    /// Semantic fact family.
    Sem = 12,
    /// Constant family.
    Const = 13,
    /// Layout descriptor family (not exercised by M1 Part A).
    Layout = 14,
    /// Initialization plan family (not exercised by M1 Part A).
    Init = 15,
    /// Function family.
    Function = 16,
    /// Basic block family.
    Block = 17,
    /// IR value family.
    Value = 18,
    /// IR instruction family.
    Instruction = 19,
    /// Virtual register family (not exercised by M1 Part A).
    VReg = 20,
    /// Continuation family.
    Continuation = 21,
    /// Task family.
    Task = 22,
    /// Result family.
    Result = 23,
    /// Diagnostic family.
    Diagnostic = 24,
    /// Host request family.
    HostRequest = 25,
    /// Output artifact fragment family.
    Artifact = 26,
}

impl RecordFamily {
    /// Every family in ordinal order.
    pub const ALL: [Self; 27] = [
        Self::Source,
        Self::Span,
        Self::Expansion,
        Self::Name,
        Self::PpToken,
        Self::Token,
        Self::Literal,
        Self::Node,
        Self::Scope,
        Self::ScopeEvent,
        Self::Symbol,
        Self::Type,
        Self::Sem,
        Self::Const,
        Self::Layout,
        Self::Init,
        Self::Function,
        Self::Block,
        Self::Value,
        Self::Instruction,
        Self::VReg,
        Self::Continuation,
        Self::Task,
        Self::Result,
        Self::Diagnostic,
        Self::HostRequest,
        Self::Artifact,
    ];

    /// Explicit ordinal (the discriminant). This is not a wire tag.
    pub const fn ordinal(self) -> u8 {
        self as u8
    }

    /// Canonical family name for the `/6` seed inventory.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Span => "span",
            Self::Expansion => "expansion",
            Self::Name => "name",
            Self::PpToken => "pp_token",
            Self::Token => "token",
            Self::Literal => "literal",
            Self::Node => "node",
            Self::Scope => "scope",
            Self::ScopeEvent => "scope_event",
            Self::Symbol => "symbol",
            Self::Type => "type",
            Self::Sem => "sem",
            Self::Const => "const",
            Self::Layout => "layout",
            Self::Init => "init",
            Self::Function => "function",
            Self::Block => "block",
            Self::Value => "value",
            Self::Instruction => "instruction",
            Self::VReg => "vreg",
            Self::Continuation => "continuation",
            Self::Task => "task",
            Self::Result => "result",
            Self::Diagnostic => "diagnostic",
            Self::HostRequest => "host_request",
            Self::Artifact => "artifact",
        }
    }
}
