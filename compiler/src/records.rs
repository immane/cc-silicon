// ============================================================================
// records.rs — draft-record envelope handles (T01 C01/C03, integrator interim)
//
// The closed typed `RecordDraft` enum (per-family draft bodies such as
// `TokenDraft`, `LiteralDraft`, `NodeDraft`) is group-owner work and is NOT
// frozen here. This module provides the interim opaque handle used by
// [`crate::task::AppendBatch`] so the commit track can implement the
// deterministic reservation protocol (P1.0 inventory, P2a name plan, P2b
// predicted refs, P2c link validation, P2d resolve) over families and links.
//
// Typed link bodies live on the commit track's `PendingDraft` carrier
// (`commit.rs`); when the records track freezes the closed enum, this handle
// is replaced and `AppendBatch.records` is re-typed without changing the
// protocol order or hash-relevant wire tags.
// ============================================================================

use crate::bus::{
    ArtifactRecord, BlockRecord, ConstRecord, FunctionRecord, InstructionRecord, LiteralRecord,
    MacroRecord, NodeRecord, PpTokenRecord, ScopeEventRecord, ScopeRecord, SemRecord, SpanRecord,
    SymbolRecord, TokenRecord, TypeRecord, ValueRecord,
};
use crate::ids::RecordFamily;
use crate::task::DraftRef;

/// An opaque same-task draft handle: its family plus its position in the
/// owning task's append batch.
///
/// Identity only — no body bytes. Bodies are group-owner records; links are
/// validated through the commit track against the predicted-ID table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RecordDraft {
    /// Canonical family of the draft (checked at typed apply by the group).
    pub family: RecordFamily,
    /// Position within the owning task's own batch.
    pub index: DraftRef,
}

/// Closed typed draft bodies: the only families the commit
/// materialization path accepts (`Literal`/`Const` frozen at Gate 1 `/7`;
/// `Artifact` added at Wave 2 `/10` for the PP01 slice; `Token`/`Name`
/// added at `/11` for the LX slice; `Node` added at `/12` for the PA slice;
/// `Type`/`Symbol`/`Scope`/`ScopeEvent` added at `/13` for the TY slice;
/// `Function`/`Block`/`Value`/`Instruction` added at `/15` for the IR slice;
/// `Span`/`PpToken` added at `/16` for the PP slice).
/// `Sem` added at `/14` for the SE slice).
/// `Macro` added at `/23` for the PP macro-definition slice.
///
/// Each body is the record-to-be: allocation assigns the stable ID, so the
/// draft body and the committed record share their fields exactly. Bodies
/// travel 1:1 positional with [`crate::task::AppendBatch::records`]; any
/// length or family mismatch rejects the whole batch before mutation.
/// Families outside this enum keep the explicit
/// `"record draft materialization pending the records track"` rejection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum G1DraftBody {
    /// A decoded literal to append to the `literals` arena.
    Literal(LiteralRecord),
    /// A folded constant to append to the `consts` arena.
    Const(ConstRecord),
    /// A normalized artifact to append to the `artifacts` arena (`/10`).
    Artifact(ArtifactRecord),
    /// A C token to append to the `tokens` arena (`/11` LX slice).
    Token(TokenRecord),
    /// A name spelling to intern through the `InternTable` (`/11` LX slice).
    Name {
        /// Raw spelling bytes.
        spelling: Vec<u8>,
    },
    /// An AST node to append to the `nodes` arena (`/12` PA slice).
    Node(NodeRecord),
    /// A canonical type to append to the `types` arena (`/13` TY slice).
    Type(TypeRecord),
    /// A declared symbol to append to the `symbols` arena (`/13`).
    Symbol(SymbolRecord),
    /// A scope to append to the `scopes` arena (`/13`).
    Scope(ScopeRecord),
    /// A scope event to append to the `scope_events` arena (`/13`).
    ScopeEvent(ScopeEventRecord),
    /// A checked-node fact to append to the `sem` arena (`/14` SE slice).
    Sem(SemRecord),
    /// An IR function to append to the `functions` arena (`/15` IR slice).
    Function(FunctionRecord),
    /// An IR block to append to the `blocks` arena (`/15`).
    Block(BlockRecord),
    /// An IR value to append to the `values` arena (`/15`).
    Value(ValueRecord),
    /// An IR instruction to append to the `instructions` arena (`/15`).
    Instruction(InstructionRecord),
    /// A source span to append to the `spans` arena (`/16` PP slice).
    Span(SpanRecord),
    /// A preprocessing token to append to the `pp_tokens` arena (`/16`).
    PpToken(PpTokenRecord),
    /// A macro definition (or `#undef` tombstone) to append to the
    /// `macros` arena (`/23` PP macro-definition slice).
    Macro(MacroRecord),
}

impl G1DraftBody {
    /// Canonical family of this body (must match the paired handle).
    pub const fn family(self: &G1DraftBody) -> RecordFamily {
        match self {
            Self::Literal(_) => RecordFamily::Literal,
            Self::Const(_) => RecordFamily::Const,
            Self::Artifact(_) => RecordFamily::Artifact,
            Self::Token(_) => RecordFamily::Token,
            Self::Name { .. } => RecordFamily::Name,
            Self::Node(_) => RecordFamily::Node,
            Self::Type(_) => RecordFamily::Type,
            Self::Symbol(_) => RecordFamily::Symbol,
            Self::Scope(_) => RecordFamily::Scope,
            Self::ScopeEvent(_) => RecordFamily::ScopeEvent,
            Self::Sem(_) => RecordFamily::Sem,
            Self::Function(_) => RecordFamily::Function,
            Self::Block(_) => RecordFamily::Block,
            Self::Value(_) => RecordFamily::Value,
            Self::Instruction(_) => RecordFamily::Instruction,
            Self::Span(_) => RecordFamily::Span,
            Self::PpToken(_) => RecordFamily::PpToken,
            Self::Macro(_) => RecordFamily::Macro,
        }
    }
}
