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

use crate::bus::{ArtifactRecord, ConstRecord, LiteralRecord, NodeRecord, TokenRecord};
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
/// added at `/11` for the LX slice; `Node` added at `/12` for the PA slice).
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
        }
    }
}
