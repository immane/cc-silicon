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
