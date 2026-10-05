use cc_silicon_compiler::arena::{ArenaError, ReservedArena, TypedArena};
use cc_silicon_compiler::ids::{SpanId, TypeId};
use cc_silicon_compiler::intern::{InternError, InternTable};
use cc_silicon_compiler::limits::Limits;

#[test]
fn ids_are_stable_and_never_reused() {
    let limits = Limits::fixture();
    let mut arena: TypedArena<SpanId, u32> = TypedArena::new();
    let a = arena.alloc(10, &limits).unwrap();
    let b = arena.alloc(20, &limits).unwrap();
    assert_eq!(a.index(), 0);
    assert_eq!(b.index(), 1);
    assert_eq!(*arena.get(a).unwrap(), 10);

    assert_eq!(arena.remove(a).unwrap(), 10);
    let c = arena.alloc(30, &limits).unwrap();
    assert_eq!(c.index(), 2, "IDs are never reused");
    assert!(matches!(arena.get(a), Err(ArenaError::Tombstoned { .. })));
    assert_eq!(*arena.get(c).unwrap(), 30);
    assert_eq!(arena.allocated(), 3);
    assert_eq!(arena.live(), 2);
}

#[test]
fn out_of_bounds_and_sentinel_are_structured() {
    let arena: TypedArena<SpanId, u32> = TypedArena::new();
    assert!(matches!(
        arena.get(SpanId::from_index(0)),
        Err(ArenaError::OutOfBounds { .. })
    ));
    assert!(matches!(
        arena.get(SpanId::NONE),
        Err(ArenaError::Sentinel { .. })
    ));
}

#[test]
fn capacity_failure_is_structured() {
    let limits = Limits {
        max_records_per_arena: 1,
        ..Limits::fixture()
    };
    let mut arena: TypedArena<SpanId, u8> = TypedArena::new();
    assert!(arena.alloc(1, &limits).is_ok());
    assert!(matches!(
        arena.alloc(2, &limits),
        Err(ArenaError::CapacityExceeded { limit: 1, .. })
    ));
}

#[test]
fn reserved_arena_allocates_ids_without_fabricating_records() {
    let limits = Limits::fixture();
    let mut arena: ReservedArena<TypeId> = ReservedArena::new();
    let a = arena.alloc(&limits).unwrap();
    let b = arena.alloc(&limits).unwrap();
    assert_eq!(a.index(), 0);
    assert_eq!(b.index(), 1);
    // A live reserved ID yields the marker, not a invented record.
    assert!(arena.get(a).is_ok());
    arena.remove(a).unwrap();
    assert!(matches!(arena.get(a), Err(ArenaError::Tombstoned { .. })));
    let c = arena.alloc(&limits).unwrap();
    assert_eq!(c.index(), 2);
}

#[test]
fn intern_is_deterministic() {
    let limits = Limits::fixture();
    let mut table = InternTable::new();
    let beta = table.intern(b"beta", &limits).unwrap();
    let alpha = table.intern(b"alpha", &limits).unwrap();
    let beta_again = table.intern(b"beta", &limits).unwrap();
    assert_eq!(beta, beta_again);
    assert_eq!(beta.index(), 0);
    assert_eq!(alpha.index(), 1);
    assert_eq!(table.get(beta).unwrap(), b"beta");
    let ids: Vec<u32> = table.iter().map(|(id, _)| id.index()).collect();
    assert_eq!(ids, vec![0, 1]);
    assert_eq!(table.total_bytes(), 9);
}

#[test]
fn intern_capacity_is_structured() {
    let limits = Limits {
        max_intern_entries: 1,
        ..Limits::fixture()
    };
    let mut table = InternTable::new();
    assert!(table.intern(b"one", &limits).is_ok());
    assert!(matches!(
        table.intern(b"two", &limits),
        Err(InternError::EntryCapacity { limit: 1 })
    ));
}
