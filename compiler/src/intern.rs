// ============================================================================
// intern.rs — deterministic string interning (T01 C01)
//
// Interning is mechanical storage: it assigns stable names, never language
// rules. The lookup uses a `BTreeMap`, so enumeration is byte-ordered and
// never depends on hash-map iteration order.
// ============================================================================

use std::collections::BTreeMap;

use crate::arena::{ArenaError, ArenaId};
use crate::ids::NameId;
use crate::limits::Limits;

/// Structured intern-table failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InternError {
    /// The configured entry count was reached.
    EntryCapacity {
        /// Configured bound.
        limit: u32,
    },
    /// The configured byte budget would be exceeded.
    ByteCapacity {
        /// Configured bound.
        limit: u64,
        /// Total bytes that would result.
        requested: u64,
    },
}

impl std::fmt::Display for InternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EntryCapacity { limit } => {
                write!(f, "intern table entry capacity {limit} reached")
            }
            Self::ByteCapacity { limit, requested } => write!(
                f,
                "intern table byte capacity {limit} exceeded (requested {requested})"
            ),
        }
    }
}

impl std::error::Error for InternError {}

/// Append-only intern table with a deterministic (byte-ordered) index.
#[derive(Clone, Debug, Default)]
pub struct InternTable {
    entries: Vec<Vec<u8>>,
    index: BTreeMap<Vec<u8>, NameId>,
    total_bytes: u64,
}

impl InternTable {
    /// Create an empty intern table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern `bytes`, returning the existing ID when already present.
    ///
    /// The first interning order defines the stable IDs; repeated runs on the
    /// same input sequence therefore allocate the same IDs.
    pub fn intern(&mut self, bytes: &[u8], limits: &Limits) -> Result<NameId, InternError> {
        if let Some(existing) = self.index.get(bytes) {
            return Ok(*existing);
        }
        if self.entries.len() as u32 >= limits.max_intern_entries {
            return Err(InternError::EntryCapacity {
                limit: limits.max_intern_entries,
            });
        }
        let requested = self.total_bytes + bytes.len() as u64;
        if requested > limits.max_intern_bytes {
            return Err(InternError::ByteCapacity {
                limit: limits.max_intern_bytes,
                requested,
            });
        }
        let id = NameId::from_index(self.entries.len() as u32);
        let owned = bytes.to_vec();
        self.entries.push(owned.clone());
        self.index.insert(owned, id);
        self.total_bytes = requested;
        Ok(id)
    }

    /// Look up bytes without allocating.
    pub fn lookup(&self, bytes: &[u8]) -> Option<NameId> {
        self.index.get(bytes).copied()
    }

    /// Borrow interned bytes by ID.
    pub fn get(&self, id: NameId) -> Result<&[u8], ArenaError> {
        if id.is_none() {
            return Err(ArenaError::Sentinel {
                arena: NameId::LABEL,
            });
        }
        self.entries
            .get(id.index() as usize)
            .map(Vec::as_slice)
            .ok_or(ArenaError::OutOfBounds {
                arena: NameId::LABEL,
                index: id.index(),
                len: self.entries.len() as u32,
            })
    }

    /// Number of interned names.
    pub fn len(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total bytes held.
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Iterate entries in ascending ID order.
    pub fn iter(&self) -> impl Iterator<Item = (NameId, &[u8])> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, bytes)| (NameId::from_index(index as u32), bytes.as_slice()))
    }
}
