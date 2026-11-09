# CCR B2-02 — Tagged, typed constants (`ConstRecord.ty` / D-09)

| Field | Value |
|---|---|
| Status | **ACCEPTED 2026-11-09 — frozen as `t01-c01-c06/42`; implementation and verification recorded in §11** |
| Owner | T01 integrator |
| Finding | **D-09 (P1)**: `ConstRecord` has no type field, so the documented `ir.op-immediate-type` rule cannot be enforced. |
| Baseline | `t01-c01-c06/41`, hash `500abb528a1e62622f6d327456d2e27f6bf1abdc1992e6b991e9dd3e0e832966` |
| Depends on | T06 committed canonical types; T07 committed `SemRecord.ty`; T09 `Constant`/`ValueRecord`; B2-01 `/40` function closure |
| Excludes | Float/address payloads, target widths, `-S`/`-c`, target execution, GCC torture evidence |

## 1. Problem

`/41` stores a constant as `{ value: Vec<u8>, negative: bool }`. A `ConstId` therefore has no semantic type. T09 can compare a `Constant` result type with a `Return` operand type, but it cannot verify the documented immediate rule:

```text
InstructionRecord.immediate -> ConstRecord.ty == ValueRecord.ty
```

D-09 cannot be closed by a documentation change, and it cannot be closed by substituting `SemRecord.ty` or `SymbolRecord.ty` at the use site: the constant record itself must carry the committed type.

## 2. Decision

Replace the `/41` body with one tagged family:

```rust
pub struct ConstRecord {
    pub ty: TypeId,
    pub value: ConstValue,
}

pub enum ConstValue {
    Integer(IntegerConst),
}

pub struct IntegerConst {
    pub magnitude: Vec<u8>, // big-endian unsigned magnitude, canonical
    pub negative: bool,     // sign; zero is canonical negative=false
}
```

Decisions:

| ID | Decision |
|---|---|
| T-01 | `ConstRecord.ty` is a committed T06 `TypeId`; it is never a private T08 type or a host type. |
| T-02 | `ConstRecord.value` is a closed tag. `/42` freezes exactly `Integer`; float/address variants are **not** added until a committed canonical type and a producer exist. Future variants are append-only tags. |
| T-03 | `IntegerConst.magnitude` is big-endian, non-empty, minimally encoded (one zero byte for zero, no leading zero otherwise). `negative` is sign-magnitude, not two's complement. |
| T-04 | Zero is canonical only as `negative = false`. A decode that finds negative zero is invalid. |
| T-05 | For `ConstantRequest::Binary`, the chip reads `ty` from the committed `SemRecord` whose `node` is the request's `BinaryExpression`. Missing or ambiguous facts fail closed. |
| T-06 | For `ConstantRequest::Literal`, the frozen wire shape carries no node. The chip uses the unique committed canonical `int` type (`TypeKind::Int { rank: Int, signed: true }`); missing or ambiguous canonical types fail closed. This is a narrow compatibility rule for the M1 literal form, not a universal type inference rule. |
| T-07 | An integer payload requires `TypeKind::Int`. Range checking against a target width stays target-gated and is not claimed. |
| T-08 | T09 enforces `ConstRecord.ty == ValueRecord.ty` for every `Constant` immediate (`ir.op-immediate-type`). The `/40` `ir.constant-result-type-from-sem` check remains as an additional consistency check. |
| T-09 | VF12 checks the same immediate type when it models a returned constant. |
| T-10 | No new task kind, chip, stage, allowlist row, arena, store, `ResultValue`, or `Proposal` is introduced. The shape change is the constant body plus hashed schema tables and rules. |

## 3. Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Add only `ty` and keep `{ value, negative }` | Closes the immediate rule but leaves no tag for future float/address payloads and invites invalid field combinations. |
| Derive the type at the consumer from `SemRecord.ty` | A `ConstId` would still be meaningless outside one use; the record could not be checked independently or replayed. |
| Store `i128` | Host-width representation and signed overflow semantics are not the C target model. |
| Add `Float`/`Address` variants now | No committed canonical float/pointer type and no producer exist in `/42`; empty variants would be decorative schema, not capability. |

## 4. Canonical encoding and hash

`encode_const` writes, in order:

```text
ty (u32 arena index) | value tag (u8, integer = 0) |
magnitude bytes | negative bool
```

New hashed tables:

```text
CONST_RECORD_FIELDS   = ["ty", "value"]
CONST_VALUE_NAMES     = ["integer"]
INTEGER_CONST_FIELDS  = ["magnitude", "negative"]
```

New normative rules:

```text
const.record-type-committed
const.record-type-from-sem
const.integer-payload-type-int
const.zero-canonical
ir.op-immediate-type
snapshot.const-record-typed-body
```

`ir.constant-result-type-from-sem` is retained. The `/41` rule inventory is otherwise unchanged.

## 5. Producer obligations

`FoldChip`:

1. resolves `constant_type` from committed facts in the narrow projection;
2. rejects a missing/ambiguous type, a non-integer type for an integer payload, and a type-arena ID that does not resolve;
3. commits exactly one `ConstRecord { ty, value: ConstValue::Integer(..) }` per evaluation;
4. keeps the existing budget, subset, node-liveness, and single-append rules unchanged.

No refold is introduced. `ConstRecord.ty` is computed from committed inputs, not guessed from the Rust host.

## 6. Consumer obligations

`IrFunctionChip` projects the committed `ConstRecord` body and checks:

- `ConstRecord.ty` resolves to a committed type;
- `ConstRecord.ty == ValueRecord.ty` for the `Constant` result;
- `ConstRecord.ty == Return` operand type through the existing `/40` check.

Violations are typed failures, never a guessed constant or a fallback type.

`Vf12Chip` rejects a modeled constant whose `ty` differs from the returned value type.

`Vf14` gate constants are decoded through the `Integer` tag; a non-integer gate is an invalid gate, never PASS.

## 7. Snapshot / replay impact

Full append bodies already encode every committed constant. `/42` extends that body with `ty` and the value tag; canonical field order is fixed by §4. Replay remains recorded-input re-execution over a canonical fingerprint, not bus restoration. Full `Snapshot::from_bytes`/restore remains deferred to `/43`.

## 8. Failure behavior

| Condition | Result |
|---|---|
| Request node has no committed `SemRecord` (binary form) | typed `Fail` |
| More than one matching `SemRecord` | typed `Fail` |
| `ConstRecord.ty` does not resolve | typed `Fail` |
| Integer payload with non-`Int` type | typed `Fail` |
| Constant immediate type differs from value/return type | typed `Fail` |
| Non-integer value tag in decode | `CodecError` |
| Negative zero or non-canonical magnitude in decode | `CodecError` |
| Non-integer gate constant | invalid gate (`0xFF`, no stage), never PASS |

No error path fabricates a constant, defaults to `int`, or skips a record.

## 9. Scope and non-goals

In scope:

- `ConstRecord` shape, canonical body codec, field/tag hashing, new rules;
- T08 fold type resolution for the M1 literal/binary forms;
- T09 immediate-type check;
- VF12/VF14 typed-constant consumption;
- fixture, negative, snapshot, and regression tests;
- T01/M1/ledger/status documentation and the `/42` freeze record.

Not in scope:

- float or address constants, pointer provenance, target widths/ABI values;
- full `max_const_bits` signed-range formula (still T08/T01 open);
- new task kinds, chips, stages, allowlists, or dispatch behavior;
- `Snapshot` restore, quota>1, VF13, `StorePatch` materialization (all `/43`);
- any target code generation, probe, or torture claim.

## 10. Acceptance checklist

- [x] `/41` baseline hash and tests recorded before the change.
- [x] New body shape and codec round-trip.
- [x] Fold rejects missing/ambiguous/non-`Int` constant types.
- [x] T09 enforces `ir.op-immediate-type`; negative fixture fails.
- [x] VF12 and VF14 consume the tagged payload.
- [x] Snapshot body changes are visible in the frozen hash.
- [x] `candidate` still models `int main(void){return 2+3;}` as `value=05`.
- [x] Full compiler suite, clippy, formatting, and freeze checks pass.
- [x] `/42` version and hash recorded in `compiler/contracts/CONTRACT_VERSION`.

## 11. Implementation evidence (2026-11-09)

| Check | Result |
|---|---|
| Artifact | `t01-c01-c06/42`, hash `a9a1da92b5e7559cd6b3cfc025553dcd5148b96632cd769381e084bda9abf1a5` |
| New acceptance | `c45_const_ty` 8/8: typed fold, missing/ambiguous `SemRecord`, non-integer type, T09 immediate mismatch, codec round-trip/unknown tag/negative zero, contract fields, manifest reads |
| Dependent suites | `c08_gate1` 22, `c15_ir` 13, `c17_vf12` 6, `c38_vf14` 10, `c05_codec` 29, `freeze` 15 |
| Full compiler suite | 684 tests across 51 suites, all passing after B-INT-00 parked the unregistered parse drafts |
| Formatting / clippy | `cargo fmt --manifest-path compiler/Cargo.toml -- --check` clean; `cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings` clean |
| Candidate determinism | Three direct runs on the same `int main(void){return 2+3;}` path were byte-identical, exited 0, and emitted `candidate evidence (t01-c01-c06/42)` plus `interpret: value=05 negative=false`; trace `ef095d87…d39196a` |
| Non-claims | No full M1 acceptance, no target ABI/probe, no target execution, no GCC torture rate |

Implementation files: `compiler/src/bus.rs`, `contract.rs`, `snapshot.rs`,
`chips/constant_layout_init/fold.rs`, `chips/ir_lower/ir_function.rs`,
`chips/verify/vf_interpret.rs`, `chips/verify/vf_evidence.rs`,
`bin/candidate.rs`, dependent tests, `compiler/contracts/CONTRACT_VERSION`.
B-INT-00 changed only the parse draft parking and its guard; it is a separate
disposition record, not part of the `/42` schema.
