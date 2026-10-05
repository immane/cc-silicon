// ============================================================================
// arena.rs — append-only arenas with checked access (T01 C01)
//
// The approved CPU storage extension allows append-only Vec arenas in the
// compiler application bus. The invariants this module enforces:
//
//   * IDs are dense, monotonically allocated, and never reused. `remove`
//     leaves a tombstone so a stale ID can never alias a new record.
//   * Every access is checked; out-of-bounds, tombstoned, and sentinel IDs
//     return a structured [`ArenaError`], never a panic.
//   * Every allocation is bounded by [`crate::limits::Limits`]; exceeding a
//     bound returns [`ArenaError::CapacityExceeded`].
//
// `Vec` is used only as the storage substrate. Records reference one another
// through newtype IDs, never through `Rc`/`Arc`/`RefCell`/`Mutex` or raw
// addresses, so a snapshot is deterministic.
// ============================================================================

use std::marker::PhantomData;

use crate::limits::Limits;

/// A stable newtype identifier that can index an arena.
///
/// Implemented by the ID types in [`crate::ids`]. The sentinel rule
/// (`raw() == u32::MAX`) is part of the contract.
pub trait ArenaId: Copy + Eq + Ord + std::fmt::Debug + 'static {
    /// Human-readable arena label, used in diagnostics and snapshots.
    const LABEL: &'static str;

    /// Build an ID from a raw index (used by decoders).
    fn from_raw(raw: u32) -> Self;

    /// The raw index.
    fn raw(self) -> u32;

    /// Whether this is the reserved "no record" sentinel.
    fn is_sentinel(self) -> bool {
        self.raw() == u32::MAX
    }
}

/// Structured arena failure. No variant is produced by a panic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArenaError {
    /// The index is at or beyond the highest allocated slot.
    OutOfBounds {
        /// Arena label.
        arena: &'static str,
        /// Requested raw index.
        index: u32,
        /// Number of slots ever allocated.
        len: u32,
    },
    /// The slot was allocated and later removed; IDs are never reused.
    Tombstoned {
        /// Arena label.
        arena: &'static str,
        /// Raw index.
        index: u32,
    },
    /// The reserved "no record" sentinel was used as a real ID.
    Sentinel {
        /// Arena label.
        arena: &'static str,
    },
    /// The arena is already at its configured capacity.
    CapacityExceeded {
        /// Arena label.
        arena: &'static str,
        /// Configured upper bound.
        limit: u32,
        /// Raw index that would have been allocated.
        requested: u32,
    },
}

impl std::fmt::Display for ArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfBounds { arena, index, len } => {
                write!(
                    f,
                    "arena `{arena}`: index {index} out of bounds (len {len})"
                )
            }
            Self::Tombstoned { arena, index } => {
                write!(f, "arena `{arena}`: index {index} is a tombstone")
            }
            Self::Sentinel { arena } => {
                write!(f, "arena `{arena}`: sentinel ID used as a record")
            }
            Self::CapacityExceeded {
                arena,
                limit,
                requested,
            } => write!(
                f,
                "arena `{arena}`: capacity {limit} exceeded (requested {requested})"
            ),
        }
    }
}

impl std::error::Error for ArenaError {}

/// A flat append-only store. Slots are never reclaimed.
#[derive(Clone, Debug)]
pub struct Arena<T> {
    slots: Vec<Option<T>>,
    live: u32,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            live: 0,
        }
    }
}

impl<T> Arena<T> {
    /// Create an empty arena.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of slots ever allocated, including tombstones.
    pub fn allocated(&self) -> u32 {
        self.slots.len() as u32
    }

    /// Number of live (non-tombstoned) records.
    pub fn live(&self) -> u32 {
        self.live
    }

    /// Whether a raw index is a live record.
    pub fn is_live(&self, index: u32) -> bool {
        self.slots.get(index as usize).is_some_and(Option::is_some)
    }

    /// Append a record and return its raw index. The caller has already
    /// enforced the configured capacity.
    pub(crate) fn push(&mut self, value: T) -> u32 {
        let index = self.slots.len() as u32;
        self.slots.push(Some(value));
        self.live += 1;
        index
    }

    /// Borrow a live record by raw index.
    pub fn get_raw(&self, index: u32) -> Result<&T, ArenaError> {
        if index == u32::MAX {
            return Err(ArenaError::Sentinel { arena: "raw" });
        }
        match self.slots.get(index as usize) {
            Some(Some(value)) => Ok(value),
            Some(None) => Err(ArenaError::Tombstoned {
                arena: "raw",
                index,
            }),
            None => Err(ArenaError::OutOfBounds {
                arena: "raw",
                index,
                len: self.slots.len() as u32,
            }),
        }
    }

    /// Mutably borrow a live record by raw index.
    pub fn get_raw_mut(&mut self, index: u32) -> Result<&mut T, ArenaError> {
        if index == u32::MAX {
            return Err(ArenaError::Sentinel { arena: "raw" });
        }
        let len = self.slots.len() as u32;
        match self.slots.get_mut(index as usize) {
            Some(Some(value)) => Ok(value),
            Some(None) => Err(ArenaError::Tombstoned {
                arena: "raw",
                index,
            }),
            None => Err(ArenaError::OutOfBounds {
                arena: "raw",
                index,
                len,
            }),
        }
    }

    /// Tombstone a live record and return it. The index is destroyed forever.
    pub fn remove_raw(&mut self, index: u32) -> Result<T, ArenaError> {
        if index == u32::MAX {
            return Err(ArenaError::Sentinel { arena: "raw" });
        }
        let len = self.slots.len() as u32;
        match self.slots.get_mut(index as usize) {
            Some(slot) => match slot.take() {
                Some(value) => {
                    self.live -= 1;
                    Ok(value)
                }
                None => Err(ArenaError::Tombstoned {
                    arena: "raw",
                    index,
                }),
            },
            None => Err(ArenaError::OutOfBounds {
                arena: "raw",
                index,
                len,
            }),
        }
    }

    /// Iterate live records in ascending index order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.as_ref().map(|value| (index as u32, value)))
    }
}

fn relabel(error: ArenaError, arena: &'static str) -> ArenaError {
    match error {
        ArenaError::OutOfBounds { index, len, .. } => ArenaError::OutOfBounds { arena, index, len },
        ArenaError::Tombstoned { index, .. } => ArenaError::Tombstoned { arena, index },
        ArenaError::Sentinel { .. } => ArenaError::Sentinel { arena },
        ArenaError::CapacityExceeded {
            limit, requested, ..
        } => ArenaError::CapacityExceeded {
            arena,
            limit,
            requested,
        },
    }
}

/// A typed, capacity-checked arena of `T` addressed by `I`.
///
/// The `I` type parameter is the only way to address a record, which makes it
/// a compile error to pass an ID into the wrong arena.
pub struct TypedArena<I: ArenaId, T> {
    inner: Arena<T>,
    marker: PhantomData<fn() -> I>,
}

impl<I: ArenaId, T> Default for TypedArena<I, T> {
    fn default() -> Self {
        Self {
            inner: Arena::new(),
            marker: PhantomData,
        }
    }
}

impl<I: ArenaId, T> TypedArena<I, T> {
    /// Create an empty typed arena.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of slots ever allocated, including tombstones.
    pub fn allocated(&self) -> u32 {
        self.inner.allocated()
    }

    /// Number of live records.
    pub fn live(&self) -> u32 {
        self.inner.live()
    }

    /// Append a record, enforcing the per-arena limit.
    pub fn alloc(&mut self, value: T, limits: &Limits) -> Result<I, ArenaError> {
        let index = self.inner.allocated();
        if index >= limits.max_records_per_arena {
            return Err(ArenaError::CapacityExceeded {
                arena: I::LABEL,
                limit: limits.max_records_per_arena,
                requested: index,
            });
        }
        let raw = self.inner.push(value);
        Ok(I::from_raw(raw))
    }

    /// Append a record without capacity checks.
    ///
    /// Crate-private: only call from a commit apply pass whose preflight has
    /// already reserved the capacity, so that no fallible step can occur after
    /// the first mutation.
    pub(crate) fn push(&mut self, value: T) -> I {
        I::from_raw(self.inner.push(value))
    }

    /// Borrow a live record.
    pub fn get(&self, id: I) -> Result<&T, ArenaError> {
        self.inner
            .get_raw(id.raw())
            .map_err(|e| relabel(e, I::LABEL))
    }

    /// Mutably borrow a live record.
    pub fn get_mut(&mut self, id: I) -> Result<&mut T, ArenaError> {
        self.inner
            .get_raw_mut(id.raw())
            .map_err(|e| relabel(e, I::LABEL))
    }

    /// Tombstone a record. The ID remains invalid forever.
    pub fn remove(&mut self, id: I) -> Result<T, ArenaError> {
        self.inner
            .remove_raw(id.raw())
            .map_err(|e| relabel(e, I::LABEL))
    }

    /// Iterate live records in ascending ID order.
    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.inner
            .iter()
            .map(|(index, value)| (I::from_raw(index), value))
    }

    /// Iterate live IDs in ascending order.
    pub fn live_ids(&self) -> impl Iterator<Item = I> + '_ {
        self.inner.iter().map(|(index, _)| I::from_raw(index))
    }
}

/// A marker returned when a reserved arena's ID is valid but the record
/// schema has not been frozen by its group owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reserved;

/// A typed arena for an ID family whose record schema is not frozen yet.
///
/// T01 declares the owning arena and the stable ID for every record family,
/// but the concrete fields of language records (types, AST nodes, IR, ...)
/// are owned by the corresponding task group and must be frozen before that
/// group is dispatched. This type allocates stable IDs mechanically and
/// exposes only the [`Reserved`] marker, never a fabricated record. The group
/// owner replaces this type with a real [`TypedArena`] when it freezes the
/// schema.
#[derive(Clone, Debug)]
pub struct ReservedArena<I: ArenaId> {
    inner: Arena<Reserved>,
    marker: PhantomData<fn() -> I>,
}

impl<I: ArenaId> Default for ReservedArena<I> {
    fn default() -> Self {
        Self {
            inner: Arena::new(),
            marker: PhantomData,
        }
    }
}

impl<I: ArenaId> ReservedArena<I> {
    /// Create an empty reserved arena.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of IDs ever allocated, including tombstones.
    pub fn allocated(&self) -> u32 {
        self.inner.allocated()
    }

    /// Number of live IDs.
    pub fn live(&self) -> u32 {
        self.inner.live()
    }

    /// Allocate a stable ID.
    pub fn alloc(&mut self, limits: &Limits) -> Result<I, ArenaError> {
        let index = self.inner.allocated();
        if index >= limits.max_records_per_arena {
            return Err(ArenaError::CapacityExceeded {
                arena: I::LABEL,
                limit: limits.max_records_per_arena,
                requested: index,
            });
        }
        let raw = self.inner.push(Reserved);
        Ok(I::from_raw(raw))
    }

    /// Check an ID. A live ID yields the [`Reserved`] marker.
    pub fn get(&self, id: I) -> Result<Reserved, ArenaError> {
        self.inner
            .get_raw(id.raw())
            .copied()
            .map_err(|e| relabel(e, I::LABEL))
    }

    /// Tombstone an allocated ID. IDs are never reused.
    pub fn remove(&mut self, id: I) -> Result<Reserved, ArenaError> {
        self.inner
            .remove_raw(id.raw())
            .map_err(|e| relabel(e, I::LABEL))
    }

    /// Iterate live IDs in ascending order.
    pub fn live_ids(&self) -> impl Iterator<Item = I> + '_ {
        self.inner.iter().map(|(index, _)| I::from_raw(index))
    }
}
