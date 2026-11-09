# ADR-0005: Constant v3 — Float and Address Payloads

| Field | Value |
|---|---|
| Status | **Accepted (2026-11-09)** |
| Decision owner | T01 integrator |
| Contract artifact | `t01-c01-c06/45` |
| Extends | `/42` tagged integer constants |
| Non-claims | No float/address producer, no layout, no target codegen |

## 1. Context

`/42` froze `ConstRecord { ty, value: ConstValue }` with only
`ConstValue::Integer`. T08 also needs floating and symbolic-address constants.
The representation must not use host `f64`/pointers or assume a target width
before probe evidence.

## 2. Decision

Append two tags to the closed constant family:

```rust
pub enum ConstValue {
    Integer(IntegerConst),
    Float(FloatConst),
    Address(AddressConst),
}

pub struct FloatConst {
    pub format: FloatFormat,
    pub bits: Vec<u8>, // big-endian raw bits, length = format.byte_len()
}

pub struct AddressConst {
    pub base: Option<SymbolId>,
    pub addend: IntegerConst,
}
```

Rules:

- `FloatFormat` is the target-model format; `IeeeBinary16/32/64/128` use
  2/4/8/16 bytes and `X87Extended` uses 10 bytes. A mismatched length is a
  codec error, never a silent reinterpretation.
- Bits are raw IEEE bytes; signed zero and NaN payloads survive.
- `AddressConst.base = None` with a zero addend is a null pointer constant.
  A base is a committed `SymbolId`, never a host pointer.
- The addend uses the existing canonical `IntegerConst` representation.
- `ConstRecord.ty` must be compatible with the tag: integer requires
  `TypeKind::Int`, float requires a future float type, address requires a
  future pointer type. Type-compatibility enforcement for the new tags waits
  on the T06 type extension; no producer may bypass it.
- Snapshot tags are append-only: integer `0`, float `1`, address `2`.

## 3. Consequences

- `/42` integer tests and producers remain valid; new variants are explicit.
- `VF14` gate decoding accepts only integer constants; float/address gates are
  invalid gates, never PASS.
- No float/address chip is implemented; no target width or ABI is asserted.
- Layout/init records and T06 pointer/aggregate/array types remain a separate
  freeze.

## 4. Alternatives rejected

| Alternative | Reason |
|---|---|
| Host `f64`/`i128` payload | Host width/rounding is not the target model |
| Store only decimal text | Loses NaN payload/signed zero and forces re-parse |
| Host pointer/address value | Non-replayable and ABI-dependent |
| Add float/address as separate record families | Duplicates `RecordRef::Const` and T08 ownership |

## 5. References

- [A14 const/layout/init requirements](../reviews/dispatch/A14_CONST_LAYOUT_INIT_CONTRACT.md) §4.2
- [T01 compiler contract](../tasks/T01_COMPILER_CONTRACT.md) §7.1 `/45`
- [T08 constant/layout/init chips](../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md)
