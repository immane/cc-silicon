# ADR-0006: Type, Layout, and Initialization Common-Base Schema

| Field | Value |
|---|---|
| Status | **Accepted (2026-11-09)** |
| Decision owner | T01 integrator |
| Contract artifact | `t01-c01-c06/46` |
| Extends | `/13` TypeKind, ReservedArena layout/init placeholders |
| Non-claims | No type/layout/init producer, no target layout values, no codegen |

## 1. Context

`/13` froze only `Void/Bool/Char/Int/Function`. A14 correctly records that
layout/initialization cannot be frozen without pointer, array, aggregate,
enum, bitfield, and VLA type facts. `layouts`/`inits` were still
`ReservedArena` placeholders. This ADR appends the minimal common-base
carriers; it does not claim byte layouts or target ABI values.

## 2. Type model append

`TypeKind` keeps `/13` order and appends:

```text
Pointer { pointee, qualifiers }
Array { element, bound }
Aggregate { kind, members }
Enum { underlying, enumerators }
```

Supporting types:

- `TypeQualifiers { const, volatile, restrict, atomic }`
- `ArrayBound::Constant(u64) | Incomplete | Vla { bound: NodeId, scope: ScopeId }`
- `AggregateKind::Struct | Union`
- `AggregateMember { name: Option<NameId>, ty, bit_width: Option<u32>, flexible: bool }`
- `Enumerator { name: NameId, value: Option<ConstId> }`

No target width, byte offset, or host layout is encoded here. `TypeRecord`
remains `{ kind }`.

## 3. Layout/init append

Replace the reserved arenas with typed arenas:

```text
layouts: TypedArena<LayoutId, LayoutRecord>
inits:   TypedArena<InitId, InitRecord>
```

`LayoutRecord` carries a structural key (type, pack, symbolic/probed target),
form, symbolic-or-bit extent size/align, ordered members, and optional VLA
descriptor. `InitRecord` carries type/layout plus a plan, static image, or
runtime-op form. Extents distinguish `Symbolic` from concrete `Bits` so
"not computed in Part A" is never confused with zero.

`G1DraftBody` appends `Layout`/`Init`; snapshot bodies are typed rather than
reserved tombstones. No allowlist row is added: no chip is authorized to write
these families yet.

## 4. Rules

- Type/appended payloads are append-only; no renaming or reinterpretation of
  `/13` variants.
- Aggregate members/VLA bounds are committed IDs; no host pointers.
- A `ConstId` in an enum/init step must be committed when a producer lands.
- Layout/init producers must respect target probe state; `Part A` uses
  `Symbolic`, and `Bits` requires a verified target policy.
- VF01/VF07/layout validation remain future work. No producer may claim
  verification coverage.

## 5. Consequences

- T06 can express pointer/array/aggregate/enum/VLA facts without private
  ad-hoc descriptors.
- T08 can later append layout/init records through the normal typed batch
  path; until then, commit authorization still requires an allowlist row.
- Snapshot bodies for layouts/inits replace reserved ID-only encoding.
- The contract gains type/layout/init field inventories and `G1DraftBody`
  variants; existing M1 records are unchanged.

## 6. Alternatives rejected

| Alternative | Reason |
|---|---|
| Freeze layout before types | No legal upstream type facts; empty schema |
| Keep ReservedArena + separate side tables | Two identities for the same store |
| Host `u64` byte offsets only | Cannot represent Part A symbolic extents |
| Add a second init family for images | Duplicates `InitId` and `RecordRef::Init` |

## 7. References

- [A14 const/layout/init requirements](../reviews/dispatch/A14_CONST_LAYOUT_INIT_CONTRACT.md) §4, §7–8
- [T01 compiler contract](../tasks/T01_COMPILER_CONTRACT.md) §7.1 `/46`
- [T06 symbol/type chips](../tasks/T06_SYMBOL_TYPE_CHIPS.md)
- [T08 constant/layout/init chips](../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md)
