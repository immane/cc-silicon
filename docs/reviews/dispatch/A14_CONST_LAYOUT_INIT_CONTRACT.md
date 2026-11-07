# A14 — Constants, Layout, and Initialization Contract Requirements

| Field | Value |
|---|---|
| Task | `A-PTS-04` (constants, layout, initialization contract requirements) |
| Status | **Design/requirements report only — not a freeze, not an implementation authorization, not code** |
| Repository | `cc-silicon`, application crate `compiler/` |
| Baseline | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` |
| Branch / HEAD | `wave4-development` / `430ae29` |
| Target | `aarch64-unknown-linux-gnu`, ELF, LP64, little-endian, AAPCS64; `target_verification=unverified` |
| Scope | T08 `CL01`–`CL26`: constant representation/evaluation, layout, initialization, VLA descriptors, errors, ownership, determinism, target gate |
| Write scope | only this report file. No source, test, contract, Cargo, `mod.rs`, shared index, or user file was modified |
| Targeted checks | `cargo test --manifest-path compiler/Cargo.toml --test c08_gate1 --test c37_const_branch` — 31 passed, 0 failed |
| Not claimed | no `/6` freeze, no new schema, no chip code, no target probe, no GCC torture result, no M1-CL-05 closed loop |

The dispatch plan's Stage-A table groups constants/layout/initialization under `A-CLI-01..16`, while the active batch ledger assigns this scope to `A-PTS-04`. This report follows the ledger and the explicit assignment.

## 1. Evidence classes used in this report

Every normative-looking statement is labelled by evidence class. Do not read a proposal as a frozen contract.

| Label | Meaning |
|---|---|
| **FACT `/39`** | Observed in the current frozen artifact/source at the baseline above. |
| **ACCEPTED SUBDECISION** | A user-recorded or user-delegated subdecision in the CDR/task package that is not a `/6` freeze and is not all of T08. |
| **PROPOSED** | A contract requirement recommended by this report; requires T01/owner co-freeze. |
| **UNVERIFIED / PART B** | Depends on a concrete target value or ABI rule that no probe has verified. |
| **NOT RUN** | No evidence was produced in this task. |

## 2. Baseline facts

### 2.1 Frozen `/39` facts

- The frozen artifact is `t01-c01-c06/39`; the target identity is frozen but concrete values are `unverified`, the probe substrate is planned and not provisioned, and no reference baseline is available ([CONTRACT_VERSION](../../../compiler/contracts/CONTRACT_VERSION)).
- T01 already defines the `constants`, `layout`, and `init` partitions ([T01 §2](../../tasks/T01_COMPILER_CONTRACT.md#2-register-partitions-and-lifetimes)); `LayoutId`/`InitId` and `RecordRef::Layout`/`RecordRef::Init` exist ([ids.rs](../../../compiler/src/ids.rs)); `StoreId::Layout`/`Init` and `RecordFamily::Layout`/`Init` exist ([task.rs](../../../compiler/src/task.rs), [ids.rs](../../../compiler/src/ids.rs)).
- `LiteralRecord` is typed and frozen with the exact fields `token, kind, radix, suffix, value, negative, spelling, candidate_type` ([bus.rs](../../../compiler/src/bus.rs), [contract.rs](../../../compiler/src/contract.rs)).
- `ConstRecord` is typed and frozen as `{ value: Vec<u8>, negative: bool }` — a big-endian magnitude plus sign, **with no type field** ([bus.rs](../../../compiler/src/bus.rs)). `ConstRecord` is the only typed constant family in `/39`.
- `ConstantRequest` is `Literal { literal: LiteralId, required_kind }` and `Binary { node: NodeId, op: ConstExprOp, lhs: LiteralId, rhs: LiteralId, required_kind }`; `ConstantResult { value: RecordRef, legality }`; `RequiredKind` is M1-closed to `IntegerConstantExpression`; `ConstLegality` is `Legal`/`NotConstantExpression`/`Unsupported` ([task.rs](../../../compiler/src/task.rs)).
- `layouts` and `inits` are `ReservedArena` placeholders, not typed record arenas ([bus.rs](../../../compiler/src/bus.rs)); snapshots only record their allocated/live counts, not bodies ([snapshot.rs](../../../compiler/src/snapshot.rs)).
- `G1DraftBody` has no `Layout` or `Init` body; commit materialization therefore cannot append a layout or initialization record ([records.rs](../../../compiler/src/records.rs), [commit.rs](../../../compiler/src/commit.rs)).
- `StoreSchema::m1_slice()` declares only `lex.literals` and `constants.records`; no `layout.*` or `init.*` field is declared ([manifest.rs](../../../compiler/src/manifest.rs)).
- The only implemented T08 chips are the M1 integer fold (`FoldChip`) and the `/37` selected-branch/static-assert narrows. Both are narrow: `FoldChip` commits one `ConstRecord`; the branch/assert shells complete `Ack` and write nothing ([fold.rs](../../../compiler/src/chips/constant_layout_init/fold.rs), [fold_branch.rs](../../../compiler/src/chips/constant_layout_init/fold_branch.rs)).
- Current tests `c08_gate1` and `c37_const_branch` cover the narrow slices, not layout, initialization, VLA, float, address constants, or a real upstream T07→T08→T09 handoff. The targeted run in this task passed 21 + 10 tests.

### 2.2 Documented but not `/39`-implemented shapes

- The T08 package and OPEN-03 co-freeze record a per-use `ConstantRequest::Literal { literal, node, required_kind }` and a binary variant with a committed node and committed operand literals; the T08→T09 handoff is the committed `RecordRef::Const` ([T08](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md), [CDR §C](../../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md), [M1 proposal §5](../../tasks/M1_PART_A_CONTRACT_PROPOSAL.md)). The current source's literal variant does **not** carry `node`, and its binary operands are decoded `LiteralId`s, not generic `RecordRef`s. Treat the documented shapes as co-freeze text, not as current source.
- The M1 proposal's draft `ConstRecord { ty: TypeId, value: i128 }` is explicitly a draft, and its `i128` formula was later marked historical/superseded ([M1 proposal §5](../../tasks/M1_PART_A_CONTRACT_PROPOSAL.md), [CDR §G](../../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)). Do not implement `ConstRecord.ty` or `i128` as if frozen.
- The accepted `max_const_bits` subdecision is only the origin/cap/projection: a hashed `Limits` value, default/cap 128, config rejects `> 128`, and the bound reaches the chip through an explicit task input projection. The representable-set formula, carrier, and enforcement split remain open ([T08 rev 44/45](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md), [CDR §G](../../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)).
- The current target values in `TargetSpec::aarch64_unknown_linux_gnu_unverified()` are expected values recorded for comparison only; they are not authoritative and must not be used as frozen ABI values ([target.rs](../../../compiler/src/target.rs), [T01 §6](../../tasks/T01_COMPILER_CONTRACT.md#6-target-and-language-model-must-be-frozen)).

### 2.3 Current uncommitted drafts

The workspace contains uncommitted parse/lex candidates, including `compiler/src/chips/parse/pa_init.rs`. `parse/mod.rs` is modified to include several of them, but they are not registered or frozen; `pa_init.rs` defines chip-local `InitError`/`InitNodeShape` and an Ack-only `= <int>` candidate ([pa_init.rs](../../../compiler/src/chips/parse/pa_init.rs)). Do not import these as the T08 contract. The uncommitted `CT_FREEZE_*`, `OP_FREEZE_*`, `VF13_FREEZE.md`, and `H04_I_FREEZE.md` files are drafts for other areas and do not freeze T08.

## 3. Goal, scope, and non-goals

### 3.1 Goal

Define the smallest shared contract that lets T08:

1. represent integer, floating, and symbolic-address constants without host types or unverified target widths;
2. evaluate integer/float constant expressions, selected conditional branches, and `_Static_assert`;
3. compute object/member/array/union/bitfield/flexible-array layout and VLA descriptors;
4. build static and automatic initialization plans without a per-element tree for implicit zeroes;
5. report deterministic, typed legality/error outcomes;
6. let T06/T05/T07/T09/T11/T13 consume the results through committed records, without T08 duplicating another owner's type/AST/target logic.

### 3.2 In scope

- T08 `CL01`–`CL26` ([T08 table](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md)).
- Constant representation, literal-to-constant handoff, integer/float folding, ICE certification, selected conditional branches, static assertions.
- Scalar/array/struct/union/bitfield/FAM/member-offset layout, VLA bound/lifetime descriptors, layout validation.
- Scalar/aggregate/designated/string/zero initialization, incomplete-array inference, static image/relocations, automatic init ops, compound literals.
- Error/overflow/unsupported taxonomy, producer/consumer ownership, determinism/replay, freeze/implementation ordering.

### 3.3 Non-goals

- No code, no schema change, no freeze, no version bump, no commit/push, no agents.
- No concrete ABI values, no macOS arm64 values, no `sizeof`/alignment/bitfield/`long double` assertions before a Linux probe is attested.
- No full C constant-expression standard; only the rules required by the frozen corpus and the T08 catalog, with explicit unsupported gaps.
- No optimizer folding policy; T10 is out of scope. T08 constant values are semantic facts, not optimization decisions.
- No target machine emission; T11 consumes T08 records and owns ABI/machine/assembly decisions.

## 4. The smallest shared constant representation

### 4.1 Frozen lexical input (`LiteralRecord`)

**FACT `/39`.** `LiteralRecord` is T04-owned and lives in `lex.literals`. It carries raw lexical facts and a symbolic `Lx08CandidateType`; it is not a constant value and has no width ([bus.rs](../../../compiler/src/bus.rs), [T08 rev 45](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md)).

| Field | Type | Owner | Meaning |
|---|---|---|---|
| `token` | `Option<TokenId>` | T04 | Originating committed token, `None` only for a deliberate synthetic literal. |
| `kind` | `LiteralKind` | T04 | `Integer`/`Character`/`String`; M1 produces only `Integer`. |
| `radix` | `u8` | T04 | `2/8/10/16`; M1 decimal only. |
| `suffix` | `LiteralSuffix` | T04 | Candidate suffix; M1 `None` only. |
| `value` | `Vec<u8>` | T04 | Big-endian magnitude, no sign and no width. |
| `negative` | `bool` | T04 | Explicit source sign; not a two's-complement bit. |
| `spelling` | `Vec<u8>` | T04 | Exact source spelling. |
| `candidate_type` | `Lx08CandidateType` | T04 | Symbolic candidate type; M1 `{Int}`, no width. |

### 4.2 Proposed `ConstRecord` v2

**PROPOSED.** Replace the `/39` integer-only `ConstRecord { value, negative }` with one tagged constant family. This is the smallest **sound** representation that covers integer, float, and symbolic address without parallel `Const` families or host-width fields.

```rust
pub struct ConstRecord {
    pub ty: TypeId,          // committed canonical constant type (T06-owned id)
    pub value: ConstValue,   // closed, tagged payload
}

pub enum ConstValue {
    Integer(IntegerConst),
    Float(FloatConst),
    Address(AddressConst),
}

pub struct IntegerConst {
    pub magnitude: Vec<u8>,  // big-endian, unsigned magnitude, no width
    pub negative: bool,      // sign; zero is canonical negative=false
}

pub struct FloatConst {
    pub format: FloatFormat, // target float format (unverified until Part B)
    pub bits: Vec<u8>,       // big-endian raw IEEE bits; preserves signed zero/NaN payload
}

pub struct AddressConst {
    pub base: Option<SymbolId>, // None + zero addend = null pointer constant
    pub addend: IntegerConst,   // link-time addend, no host pointer
}
```

| Field | Type | Owner/status | Rule |
|---|---|---|---|
| `ConstRecord.ty` | `TypeId` | **PROPOSED** new field; id is T06-owned | Must name a committed canonical type compatible with the payload. Never a private T08 type. |
| `ConstRecord.value` | `ConstValue` | **PROPOSED** replacement of `/39` `value`/`negative` | Exactly one tag; no invalid combinations. |
| `IntegerConst.magnitude` | `Vec<u8>` | **PROPOSED** (preserves `/39` name/semantics) | Big-endian, minimal or canonical zero; no target width. |
| `IntegerConst.negative` | `bool` | **PROPOSED** (preserves `/39` name/semantics) | Sign-magnitude; `-0` is not canonical. |
| `FloatConst.format` | `FloatFormat` | **PROPOSED**; values **UNVERIFIED** | Must match the target model; no host `f64` fallback. |
| `FloatConst.bits` | `Vec<u8>` | **PROPOSED** | Raw bits, length fixed by format; preserves signed zero and NaN payload. |
| `AddressConst.base` | `Option<SymbolId>` | **PROPOSED** | Link-time symbol, never a host pointer. |
| `AddressConst.addend` | `IntegerConst` | **PROPOSED** | Same width-neutral integer representation. |

**Why replace rather than add a parallel family?** A second `FloatConstRecord`/`AddressConstRecord` family would duplicate T08 ownership, force T09 to branch across families, and create two `RecordRef::Const`-like concepts. One tagged `ConstRecord` is smaller and keeps T08 the sole `constants.records` writer. The migration cost is real: `/39` tests and `fold.rs` construct `ConstRecord { value, negative }` and must be updated at the freeze.

**Alternative considered and rejected:** keep `/39` `{value, negative}` and derive the type from the use-site `SemRecord.ty` without storing `ConstRecord.ty`. This is smaller in bytes but cannot satisfy a T09 check of `ConstRecord.ty == ValueRecord.ty`, cannot distinguish integer/float/address payloads, and makes a `ConstId` meaningless outside its exact use. See C-01/C-17 in §12.

### 4.3 Type link and target mode

**PROPOSED.** T08 must not invent a type. `ConstRecord.ty` is read from the committed T07 `SemRecord.ty` for the request's `node` (or from an explicit `TypeId` payload for a cast/sizeof/address task). T08 must not call T06; a missing or non-scalar type is a typed protocol/task fault.

**PROPOSED.** The task input projection carries a target mode, not a private ABI:

```rust
pub enum ConstTargetMode {
    Symbolic,                    // Part A: no target width, no bit pattern
    Probed(TargetFingerprint),   // Part B: verified target identity only
}
```

`TargetFingerprint` is a shared T01-owned identity (`Symbolic` or a probed report hash), not a copy of 21 scalar values in every constant. The chip reads the projected scalar facts only when the mode is `Probed`; the adapter owns the projection. A `Symbolic` mode must not fabricate a width or bit pattern. A `Probed` mode requires `TargetSpec::verification().is_verified()`/`require_verified()`; otherwise the task fails closed as `TargetUnverified` ([target.rs](../../../compiler/src/target.rs)).

### 4.4 Integer value rules

- **FACT / PROPOSED boundary:** `/39` stores only magnitude + sign; the proposed v2 keeps that representation and adds the type. A target bit pattern is not stored in Part A; in Part B it is deterministically derived from `ty` + target model by T11 (or, if T01 insists, stored as a verified derived field — open Q4).
- `IntegerConst` values are mathematical integers, not host `i32`/`i128`. Arithmetic must use explicit wide/big-integer operations with checked carry/length; host overflow is never the semantic rule.
- **Part A:** exact integer arithmetic is allowed only for operators whose C result does not require a target width: literal evaluation, `+` (the M1 `2+3` case), and other operators explicitly frozen as target-independent. Unary `~`, shifts, unsigned wrap, integer conversions, and `sizeof`-driven values are **Part B** or `Unsupported`.
- **Part B:** apply T06/T07 types and the verified target model. Signed overflow in an ICE is a constraint/undefined-behavior diagnostic, not a silent wrap; unsigned arithmetic wraps modulo the target width. Division/remainder by zero, `INT_MIN / -1`, invalid shift counts, and out-of-range conversions are diagnostics. `-fwrapv`/dialect interaction is an open policy item (Q5); it must be part of config, not a chip default.
- The sign is not a second value: `ty` carries signedness; `negative` is the sign of the mathematical value. `_Bool` constants must be canonical `0`/`1`; char/enum constants follow T06's type rules.

### 4.5 Floating value rules

- **PART B / UNVERIFIED.** Float constants are not part of the M1 exercised path. `FloatFormat` and scalar widths are unverified until the Linux probe; `long double` is especially unverified ([T01 §6](../../tasks/T01_COMPILER_CONTRACT.md#6-target-and-language-model-must-be-frozen), [T11 probe qualifier](../../tasks/T11_TARGET_CHIPS.md)).
- Store raw bits, not host `f64`. `FloatConst.bits` preserves signed zero, infinities, and NaN payloads; arithmetic uses a deterministic software implementation with a frozen rounding mode (round-to-nearest-even unless the dialect/options say otherwise) and explicit exception policy. Do not use Rust host floating-point operations for a format that is not exactly binary32/binary64, and do not use host `f64` for `long double`.
- Part A float folding is `Unsupported`/deferred. A float constant that is merely decoded/transported may be represented, but no arithmetic or static-assert/evaluation result may claim a target format without the gate.
- Float-to-int and int-to-float conversions require the target model and the C conversion boundary rules; out-of-range behavior must be a typed diagnostic/unsupported, not a host cast.

### 4.6 Symbolic address constants

- **PROPOSED / PART A for symbolic link-time form.** `AddressConst` carries a symbol (or null) plus a width-neutral addend. It is a link-time symbolic address, not a host pointer and not a target virtual address.
- **PART B** for pointer width, relocation form, TLS, section, visibility, PIC/PIE, and addend range. Unknown relocation/ABI policy is `Unsupported`, not a zero.
- An address constant is never a `ConstValue::Integer` cast of a host address. A null pointer constant is `base=None, addend=0`; it may be converted to a pointer type only by the frozen conversion rules.

### 4.7 Identity and reuse

**PROPOSED.** T08 appends exactly one `ConstRecord` per evaluation. It does not intern or deduplicate constants in M1. Multiple uses of the same mathematical value may produce distinct `ConstId`s; T09 consumes the exact `ConstId` produced by the evaluation and never re-folds. A future deterministic lowest-`ConstId` reuse scan may be added only as an explicit contract change; it must not be a hidden cache.

## 5. Constant task and result schema

### 5.1 Request variants

The wire payload remains `TaskKind` + ordered `RecordRef`s; decoded structs may use `LiteralId` after validating the reference family. `ConstRef` below means a wire-level `RecordRef::Literal` or `RecordRef::Const` — it is not a new record type. Every `ConstRef` must resolve to a committed body or the task fails with a protocol/task fault.

| Chip | Proposed task kind | Ordered wire payload | Decoded request | Result carrier |
|---|---|---|---|---|
| CL01 ConstantContextChip | `constant_layout_init.const_classify` | `[Node]` plus a task-kind-encoded required context | `ConstClassify { node, required_kind }` | `Ack` or typed legality/fault; no value record |
| CL02 ConstantUnaryChip | `constant_layout_init.const_eval_unary` | `[Node, ConstRef]` | `Unary { node, op, operand, required_kind }` | `ConstantResult` → one `ConstRecord` on `Legal` |
| CL03 ConstantBinaryChip | `constant_layout_init.const_eval_binary` | `[Node, ConstRef, ConstRef]` (M1 source order: node + literal + literal) | `Binary { node, op, lhs, rhs, required_kind }` | `ConstantResult` → one `ConstRecord` |
| CL04 ConstantBranchChip (`&&`) | `constant_layout_init.const_eval_logical_and` | `[Node, ConstRef, ConstRef]` | `Logical { node, op: And, lhs, rhs, required_kind }` | `ConstantResult` → one `ConstRecord` (canonical 0/1) |
| CL04 ConstantBranchChip (logical OR) | `constant_layout_init.const_eval_logical_or` | `[Node, ConstRef, ConstRef]` | `Logical { node, op: Or, lhs, rhs, required_kind }` | `ConstantResult` → one `ConstRecord` (canonical 0/1) |
| CL04 ConstantBranchChip (`?:`) | `constant_layout_init.const_eval_conditional` | `[Node, ConstRef, ConstRef, ConstRef]` | `Conditional { node, cond, then_arm, else_arm, required_kind }` | `ConstantResult` → one `ConstRecord` of the selected arm's common type |
| CL05 ConstantCastChip | `constant_layout_init.const_eval_cast` | `[Node, ConstRef, Type]` | `Cast { node, operand, target_ty, required_kind }` | `ConstantResult` → one `ConstRecord` |
| CL06 AddressConstantChip | `constant_layout_init.const_eval_address` | `[Node, Symbol, ConstRef]` (exact addend form open) | `Address { node, base, addend, required_kind }` | `ConstantResult` → one `ConstRecord::Address` |
| CL07 StaticAssertChip | `constant_layout_init.const_certify_assert` | `[Node, ConstRef]` plus optional `[Literal]` message | `StaticAssert { node, condition, message }` | `Ack`/`Fail`; no new value record |

**M1 compatibility.** The current `/39` literal and binary kinds remain the M1 exercised subset. The final contract should make the literal variant carry the per-use `node` (for diagnostics and `SemRecord.ty`); the current source omits it. The binary variant's operands should eventually be `ConstRef`s so nested subexpressions and already-folded operands are usable; M1 may keep the literal-only shape until the freeze.

**Task-kind spelling/numeric codes.** All proposed kind names, group-local numbers, payload variant spellings, and wire tags remain freeze items. No number is asserted by this report.

### 5.2 Result and legality

```rust
pub struct ConstantResult {
    pub value: Option<RecordRef>, // Some(RecordRef::Const) only for Legal
    pub legality: ConstLegality,  // Legal | NotConstantExpression | Unsupported
}
```

| Legality | Terminal proposal | Record effect | Notes |
|---|---|---|---|
| `Legal` | `Complete { value: ResultValue::Record(RecordRef::Const) }` | Exactly one committed `ConstRecord` in the same task batch | T08 is the sole writer; no operand records. |
| `NotConstantExpression` | `Fail` with `NotConstantExpression` diagnostic | None | Do not append a zero/placeholder const. |
| `Unsupported` | `Fail` with `ConstUnsupported` diagnostic | None | Explicit gap, never a silent success. |
| (representation overflow) | `Fail` with `ConstOverflow` diagnostic | None | `max_const_bits`/representation, not C signed overflow. |
| (C constraint/UB) | `Fail` with `ConstConstraintViolation` (proposed) | None | Divide by zero, bad shift, signed ICE overflow, etc. |
| (target unverified for a concrete result) | `Fail` with `TargetUnverified`/`Unsupported` | None | Part B gate. |

`ConstantResult.value` should be `Option` in the freeze; `/39`'s non-optional `RecordRef` forces an invalid value on non-legal paths. This is a small but real schema cleanup (C-18).

### 5.3 Integer folding rules (CL02/CL03/CL05)

| Operation | Part A symbolic | Part B verified target |
|---|---|---|
| Literal | Decode magnitude/sign; no width; check `max_const_bits`. | Decode, then check `ty` representability. |
| `+`, `-`, `*` | Exact checked integer; overflow of the representation budget is `ConstOverflow`. | Apply target type/usual conversions; signed overflow is a constraint diagnostic; unsigned wraps modulo width. |
| `/`, `%` | Defined exact integer division only when operands are exact and nonzero; otherwise diagnostic. | Target signedness/rounding; zero divisor and `INT_MIN / -1` are diagnostics. |
| Unary `+`, `-` | Exact; `-` of zero canonicalizes to zero. | Apply type/width; `-INT_MIN` is a constraint diagnostic unless the dialect says otherwise. |
| `~`, shifts, bitwise, comparisons | Only if explicitly frozen target-independent; otherwise `Unsupported`. | Full target-width semantics; shift-count/negative-shift diagnostics; no host shift. |
| Integer conversion/cast | Identity only if source and destination are the same symbolic type; otherwise `Unsupported`. | Full T06/T07 conversion plan; truncate/extend, `_Bool`, signed/unsigned conversion; out-of-range conversion diagnostic/unsupported as C requires. |
| Character/enum | Use T06 type and T04 literal; no re-decode. | Underlying type/width from target model; enum values follow T06. |

**Important:** `max_const_bits` is a resource/representation budget, **not** the C `int` width. Never use it as `sizeof(int)` or as the target arithmetic width.

### 5.4 Static assertion (CL07)

**PROPOSED.** `_Static_assert` is a certification, not a value-producing constant:

- Input: a committed `ConstRef` for the condition plus the committed use `NodeId`; optional message from a committed string `LiteralRecord`.
- The condition must be an integer constant expression. A non-constant condition is `NotConstantExpression`; a non-integer condition is a semantic failure.
- Condition value zero is a failed assertion; nonzero passes. The result is `Ack` or `Fail` with the assertion location; no `ConstRecord` is appended by CL07.
- The message is optional (C11 two-argument form); string decoding is T04's job. CL07 must not re-lex/re-decode the message or fabricate one.
- `_Static_assert(sizeof(int) == 4, ...)` is **Part B** because `sizeof` needs the target model. A symbolic-only condition that does not require a width may be Part A.
- Current `/37` `StaticAssertChip` accepts only one bare literal and uses `Task,3` for the failed assertion; that is a narrow slice, not the final message/expression contract (C-12, C-16).

### 5.5 Conditional certification (CL04)

**PROPOSED.** `&&`, `||`, and `?:` are constant evaluations that commit the selected value; they are not Ack-only forever.

- The selected operand is evaluated; the unselected operand is not value-evaluated and is not budget-checked.
- **Correctness rule:** the unselected operand must still be syntactically and semantically valid, and its committed semantic fact must exist. C11 6.6 permits non-constant constructs in an unevaluated subexpression, but the whole expression must type-check. A dangling/unchecked reference is a protocol fault or a T07 failure, not a successful constant certification. The current `/37` narrow test that accepts `0 && <dangling>` is acceptable only as a value-selection certificate; it is not the final contract (C-13).
- `&&`/`||` produce canonical `0`/`1` of type `int` (subject to the target's `int` type); `?:` produces the common type of the two arms as determined by T06/T07. The current "pass magnitude through verbatim" behavior is a narrow M1 convenience, not the final type rule.
- The condition truthiness must follow the condition type (integer zero/nonzero, pointer null/nonnull, `_Bool`); float truthiness is Part B and must follow the target format. No host truthiness.

### 5.6 Float folding

**PART B / PROPOSED.** No float folding is implemented or frozen at `/39`. The freeze must specify:

- which C floating types and formats are in scope (`float`, `double`, `long double`; complex/vector deferred);
- exact rounding mode and exception behavior;
- NaN payload/signaling policy and signed-zero preservation;
- conversion boundaries (float→int, int→float, float→float);
- whether constant folding uses a software format implementation or a verified exact library;
- proof that the result is bit-identical to the target model, not the host FPU.

Until then, a float constant operation is `Unsupported`; a decoded float literal may be transported but not evaluated.

## 6. Target-width / Part A vs Part B gate

| Capability | Part A (symbolic, no probe) | Part B (probe verified) | Gate/notes |
|---|---|---|---|
| Integer literal + additive `+` | Yes (the M1 `2`, `3`, `2+3` path) | Yes with target types | Part A records carry no width/bit pattern. |
| Unary/bitwise/shift/unsigned wrap/casts | No for width-dependent forms; explicit `Unsupported` | Yes | Requires target scalar model and conversion plans. |
| Float arithmetic/format | No (`Unsupported`) | Yes, after format/rounding freeze | `long double` unverified; no host `f64`. |
| Symbolic address + addend | Yes for link-time symbolic form | Yes with pointer width/relocation | No host pointer; no target address value. |
| Scalar `sizeof`/`alignof` | No | Yes | Read `TargetSpec` scalars only when attested. |
| Struct/union/array/FAM size/offset | Structural checks only; no byte offsets if element size unknown | Yes | Target model + ABI layout rules. |
| Bitfield allocation/extraction | No | Yes | AAPCS64 bitfield rules need probe/H07 classification; not frozen here. |
| VLA descriptor/lifetime | Structural descriptor yes; runtime size no | Yes with stride/stack alignment | VLA size is never a static `ConstRecord`. |
| Static data bytes/relocations | No (no width/endian bytes) | Yes | Target endian/relocation policy. |
| Initialization plan/designators | Structural plan yes; concrete offsets/size Part B | Yes | Static initializer values still need constant legality. |
| `_Static_assert` with `sizeof`/`alignof` | No | Yes | Pure symbolic condition may be Part A. |
| Code generation | No | Yes | `ensure_codegen_ready()` must fail closed until attested. |

**Rule.** A chip that cannot distinguish `Symbolic` from `Probed` must fail closed. It must never read the unverified expected numbers as if they were target facts, and it must never substitute macOS arm64 or Rust host values.

## 7. Layout contract

### 7.1 Proposed `LayoutRecord`

**PROPOSED.** Replace `ReservedArena<LayoutId>` with `TypedArena<LayoutId, LayoutRecord>` under the `layout` store.

```rust
pub struct LayoutRecord {
    pub key: LayoutKey,
    pub form: LayoutForm,
    pub size: LayoutExtent,
    pub align: LayoutExtent,
    pub members: Vec<LayoutMember>,
    pub vla: Option<VlaDescriptor>,
}

pub struct LayoutKey {
    pub ty: TypeId,
    pub pack: Option<u8>,             // effective pack; None = natural
    pub target: TargetFingerprint,    // Symbolic or probed report hash
}

pub enum LayoutExtent {
    Symbolic,       // Part A: not a concrete byte/bit count
    Bits(u64),      // concrete, with checked arithmetic
}

pub enum LayoutForm {
    Scalar,
    Pointer,
    Array { element: LayoutId, count: Option<u64> }, // None = incomplete/VLA
    Record,
    Union,
    BitFieldStorage,
    Vla,
}

pub struct LayoutMember {
    pub index: u32,
    pub name: Option<NameId>,
    pub ty: TypeId,
    pub layout: LayoutId,
    pub offset: LayoutExtent,         // bit offset from object start
    pub bit_width: Option<u32>,
    pub bit_offset: LayoutExtent,     // bit offset within allocation unit
}
```

`LayoutExtent::Symbolic` is deliberately different from `Option<u64>`: "not computed in Part A" and "zero bits" must not be confused. All arithmetic is checked; a size/offset/stride overflow is a typed diagnostic, never a wrap.

### 7.2 Required upstream type facts (T06-owned)

**PROPOSED / BLOCKED ON T06.** T08 must not define a private aggregate descriptor. T06 must expose, through the shared `TypeRecord`/`TypeKind` (or an explicitly co-frozen T06 record), at least:

| Fact | Needed by | Current `/39` |
|---|---|---|
| Pointer pointee + qualifiers | CL08/CL14 | `TypeKind` has no pointer |
| Array element + bound kind (`Constant`, `Incomplete`, `Vla`) | CL09/CL15 | `TypeKind` has no array |
| Struct/union member order, names, types, completeness | CL10/CL11/CL14 | `TypeKind` has no aggregate |
| Bitfield declared type/width, unnamed/zero-width flags | CL12 | absent |
| Flexible array member marker/last-member rule | CL13 | absent |
| Enum underlying type/enumerators | CL03/CL05 | absent |
| `pack`/`aligned` attributes resolved from pragmas/declarations | CL10–CL13 | absent |
| VLA bound expression/type | CL15 | absent |

The current `/39` `TypeKind` is the M1-closed `{Void, Bool, Char, Int, Function}` set ([bus.rs](../../../compiler/src/bus.rs)), while T06's package describes aggregates, pointers, arrays, enums, bitfields, and VLA ([T06](../../tasks/T06_SYMBOL_TYPE_CHIPS.md)). That is a direct document/source conflict (C-04). Layout cannot be frozen before T06 exposes these facts.

### 7.3 Layout rules

**Object/scalar layout (Part B unless fully symbolic):**

- Scalar size/alignment comes only from the attested `TargetSpec`; no host `size_of`, no macOS values.
- `void`, incomplete types, functions, and zero-size objects are invalid object layouts; arrays of incomplete/function types are invalid; an object must be complete when its size is required.
- Pointer size/alignment comes from the target pointer scalar; function pointers may differ only if the target model says so (currently not verified).
- `_Bool` size/alignment follows the target model; bit width may be less than a byte only as a bitfield.

**Array layout:**

- Element layout is computed first (child task or committed `LayoutId`); stride = element size rounded to element alignment; total = count × stride with checked multiplication.
- Incomplete array: `size` is `Symbolic` until completed; `sizeof` is invalid; a completed array is a new T06 type, not a T08 type write.
- Zero-length arrays are a GNU extension; dialect-gated and explicit.
- VLA: see §7.4; size/stride may be runtime.

**Struct layout:**

- Members in declaration order; offset = round up current bit offset to the member's alignment (unless packed); member occupies its size/bitfield; final size rounds up to the struct alignment.
- Anonymous members/unnamed bitfields follow T06's member facts; anonymous aggregate names may collide across nested paths and require TY06 lookup rules.
- Tail padding is included in `sizeof`; padding bytes are unspecified for automatic objects but zero for static initialization (see §8).
- Packed/aligned attributes alter alignment/offset; the effective pack is committed, never inferred from a host pragma or a private header.

**Union layout:**

- Size = max member size, rounded up to the union alignment; alignment = max member alignment; each member offset is zero (or ABI-defined overlay policy).
- A union member may be larger in alignment than another; the maximum wins.
- Bitfield union members overlay according to the same allocation-unit policy; no host union layout.

**Bitfield layout:**

- Allocation unit = the declared bitfield base type's size/alignment from the verified target model, unless the ABI/pack policy says otherwise.
- Fields are allocated in declaration order; a field that does not fit in the remaining bits either advances to the next unit or straddles only if the frozen ABI says so. **This policy is not frozen here; it needs H07/probe classification.**
- A zero-width unnamed bitfield advances to the next allocation-unit boundary and is not a member; a zero-width named bitfield is invalid.
- Width must be <= the base type width; `_Bool` width <= 1; negative width invalid.
- `offsetof` on a bitfield is invalid; `sizeof` includes allocation units, not just used bits.
- Bit numbering/endianness for extraction is target-defined; the current target identity says little-endian, but the extraction contract must be explicit and tested. No Rust `#[repr]`/host bitfield layout.
- Signedness and sign extension of a bitfield value follow the declared type; packed bitfields must preserve the same observable value rules.

**Flexible array member:**

- Only the last member of a struct, and only in a struct with at least one other named member (C11); `sizeof` excludes the flexible tail; alignment may increase the struct size.
- GNU differences (e.g. `[0]`) are dialect-gated and explicit.
- A struct with a FAM cannot be a member of an array/another struct in the standard form; initialization of the tail is an extension.

**Member offset (CL14):**

- Follows nested anonymous member paths; returns a `ConstRecord` of the offset type (`size_t`) only in Part B; bitfield members are rejected; array indexing is a path component, not a separate offset record.
- `offsetof` of an incomplete type or a bitfield is a typed diagnostic.

### 7.4 VLA descriptor

**PROPOSED.** VLA descriptors live on `LayoutRecord.vla` (no separate ID family):

```rust
pub struct VlaDescriptor {
    pub element: LayoutId,
    pub bound_node: NodeId,          // committed bound expression
    pub bound_const: Option<ConstId>,// Some only if the bound is a constant expression
    pub runtime: bool,               // true when the bound is not an ICE
    pub stride: LayoutExtent,
    pub align: LayoutExtent,
    pub scope: ScopeId,              // lifetime scope
    pub cleanup: bool,               // needs scope-exit cleanup
}
```

Rules:

- VLA size/stride are runtime facts, **never** a static `ConstRecord`; `sizeof(VLA)` is not an integer constant expression unless the bound itself is constant and the type is not variably modified.
- The bound expression is evaluated once at the declaration point; side effects occur once. T08 records the bound node and constant fact; T09 lowers the runtime evaluation.
- Constant bound <= 0 is a constraint violation; a runtime bound <= 0 is undefined/implementation-defined and needs a typed runtime check/trap plan, not a silent zero.
- Multidimensional VLA stride is element layout × runtime outer bound with checked arithmetic.
- `cleanup`/lifetime edges are T09-owned: T08 supplies the descriptor/scope, T09/IR26 supplies return/break/goto/scope-exit cleanup and forbids jumps into the VLA scope. CL25 must not fabricate a CFG.
- VLA types are not `ConstRecord`s and must not be accepted by `const.eval` as values.

### 7.5 Layout task/result table

| Chip | Proposed task kind | Ordered wire payload | Result | Gate |
|---|---|---|---|---|
| CL08 ScalarLayoutChip | `layout.scalar` | `[Type]` | `Record(Layout)` | Part B |
| CL09 ArrayLayoutChip | `layout.array` | `[Type]` | `Record(Layout)` | Part B (unless element symbolic) |
| CL10 StructLayoutChip | `layout.struct` | `[Type]` | `Record(Layout)` | Part B |
| CL11 UnionLayoutChip | `layout.union` | `[Type]` | `Record(Layout)` | Part B |
| CL12 BitFieldLayoutChip | `layout.bitfield` | `[Type]` | `Record(Layout)` | Part B + ABI/probe |
| CL13 FlexibleArrayChip | `layout.flexible_array` | `[Type]` | `Record(Layout)` | Part B |
| CL14 MemberOffsetChip | `layout.member_offset` | `[Type, Name]` or `[Type, Const index]` | `Record(Const)` offset | Part B |
| CL15 VlaBoundChip | `layout.vla_bound` | `[Type, Node, Scope]` | `Record(Layout)` with `vla` | Structural Part A; size Part B |
| CL26 LayoutValidateChip | `layout.validate` | `[Layout]` | `Ack`/`Fail` | Reads layout/init; no writes |

Array/struct/union chips need child layout tasks for element/member types; they must `AwaitChildren`, not recursively call another chip or read uncommitted drafts. A layout record may be reused by a deterministic lowest-`LayoutId` key scan; no hidden cache. `LayoutKey` includes target fingerprint and pack so a stale layout is detectable.

## 8. Initialization contract

### 8.1 Initialization tree input

**PROPOSED / BLOCKED ON T05.** The initializer tree is a committed AST subtree owned by T05, not a private T08 tree. T05 must expose (exact `NodeKind`/field shapes are T05's freeze):

| Needed fact | Used by | Current `/39` |
|---|---|---|
| Initializer list / braced list / scalar initializer | CL16/CL17 | `NodeKind` has no initializer nodes |
| Designator `.field` / `[index]` / GNU range | CL18 | absent |
| String literal initializer | CL19 | `LiteralRecord` has a `String` kind but T04 M1 does not produce it |
| Incomplete array bound inference | CL20 | absent |
| Compound literal type/initializer | CL24 | absent |
| Address constant initializer | CL06/CL16 | absent |

The uncommitted `pa_init.rs` is a candidate parser with chip-local `InitNodeShape`; it is not a shared schema and must not be imported by T08 (C-20).

### 8.2 Proposed `InitRecord`

**PROPOSED.** Replace `ReservedArena<InitId>` with `TypedArena<InitId, InitRecord>` under the `init` store. One family covers plans, static images, and runtime ops; no new `InitImageId`/wire tag is needed.

```rust
pub struct InitRecord {
    pub ty: TypeId,
    pub layout: LayoutId,
    pub form: InitForm,
}

pub enum InitForm {
    Plan { storage: InitStorage, steps: Vec<InitStep> },
    StaticImage { plan: InitId, bytes: Vec<u8>, relocs: Vec<Reloc> },
    RuntimeOps { plan: InitId, ops: Vec<InitStep> },
}

pub enum InitStorage { Static, Automatic, Thread }

pub enum InitStep {
    Zero { offset: LayoutExtent, length: LayoutExtent },
    Scalar { offset: LayoutExtent, value: ConstId },
    Copy { offset: LayoutExtent, source: RecordRef, length: LayoutExtent },
    Relocation { offset: LayoutExtent, base: Option<SymbolId>, addend: IntegerConst },
    RuntimeStore { offset: NodeId, value: NodeId },
}

pub struct Reloc {
    pub offset: u64,          // byte offset in the static image; checked
    pub base: SymbolId,
    pub addend: IntegerConst,
}
```

Rules:

- A `ConstId` in an init step must be a committed `ConstRecord` of a compatible type; T08 does not re-fold it.
- `Zero` ranges describe implicit zeroes; a large array initialization must use ranges, not one step/record per element. The plan is the compact representation; the image may be materialized only when required and within a configured byte budget.
- `Copy`/`RuntimeStore` reference committed literal/init/node facts; automatic initialization preserves source-order side effects. T09 lowers the runtime ops; T08 does not emit machine stores.
- `StaticImage.bytes` use target endian/width and relocations; no host address. `StaticImage` is Part B. Relocation order is deterministic (offset, then symbol ID, then addend).
- `InitForm::StaticImage`/`RuntimeOps` reference the `Plan` they materialize; do not duplicate the plan body.
- Padding: static images zero-fill padding; automatic padding is unspecified and must not be claimed zeroed. Unspecified members are zero-initialized per C rules; unions initialize only the selected member (or the first member for static zero initialization).

### 8.3 Initialization rules

| Rule | Required behavior |
|---|---|
| Scalar initialization | Assignment conversion (T06/T07) then constant/runtime value; braced scalar is allowed; excess elements diagnostic. |
| Aggregate braces/elision | Current object advances according to braces/elision; nested aggregates without braces follow C11; excess elements diagnostic. |
| Designators | `.member` and `[index]` select a subobject, then initialization continues after it; duplicate/overridden initializers have a deterministic last-wins or diagnostic policy—choose and freeze one; GNU range designators are dialect-gated. |
| Incomplete array | The maximum designated/positional index determines the bound; completion creates a new T06 type, not a T08 type write. |
| String initialization | `char a[N] = "..."`: exact fit without trailing NUL is allowed when N == literal length; wider/narrower prefixes follow T06; wide strings use the target `wchar_t`/encoding (Part B). |
| Zero initialization | Unspecified subobjects (including padding for static) are zero; pointers are null, not all-zero bytes; float zero is positive zero; union/nested rules explicit. |
| Static initialization | Must be a constant expression or address constant; no function call/side effect; T08 emits a static image only in Part B. |
| Automatic initialization | May have side effects; source order preserved; explicit memzero/store/copy ops, no per-element zero nodes. |
| Compound literal | File-scope static vs block automatic; object identity/lifetime is defined by C; T08 plans it, T09 owns object/address lowering. |
| VLA initialization | VLA objects cannot be initialized; a pointer to VLA can be. |
| Out-of-bounds | Any initializer outside the object layout is a typed diagnostic; no silent clamp. |

### 8.4 Init task/result table

| Chip | Proposed task kind | Ordered wire payload | Result | Gate |
|---|---|---|---|---|
| CL16 ScalarInitializerChip | `init.scalar` | `[Type, Node]` + storage task variant | `Record(Init)` Plan | Part A structural; values Part A/B |
| CL17 AggregateInitializerChip | `init.aggregate` | `[Type, Node]` | `Record(Init)` Plan | Part A structural |
| CL18 DesignatedInitializerChip | `init.designated` | `[Type, Node]` | `Record(Init)` Plan | Part A structural |
| CL19 StringInitializerChip | `init.string` | `[Type, Literal]` | `Record(Init)` Plan | Part B for wide/encoding |
| CL20 IncompleteArrayInferChip | `init.infer_array` | `[Type, Node]` | `Record(Const)` bound + T06 completion request | T06 co-freeze |
| CL21 ZeroInitializeChip | `init.zero` | `[Type, Layout]` | `Record(Init)` Plan | Part A/B |
| CL22 StaticDataEmitChip | `init.static_image` | `[Init]` | `Record(Init)` StaticImage | Part B |
| CL23 AutomaticInitPlanChip | `init.runtime_plan` | `[Init]` | `Record(Init)` RuntimeOps | Part A structural; T09 |
| CL24 CompoundLiteralChip | `init.compound_literal` | `[Type, Node, Symbol]` | `Record(Init)` Plan | T09 object identity |
| CL25 VlaLifetimeChip | `vla.lifetime` | `[Layout, Scope]` | `Ack`/`Fail`; T09 derives cleanup edges from `VlaDescriptor` | T09 CFG |
| CL26 LayoutValidateChip | `layout.validate` | `[Layout]` | `Ack`/`Fail` | T08/T13 |

CL20 is an ownership-sensitive row: if implemented as `CompletedType`, T08 would write `types.records`, violating T06's single writer. The contract must split "infer bound" (T08) from "complete the array type" (T06), or move CL20 to T06 (C-15).

## 9. Error, legality, overflow, and unsupported taxonomy

### 9.1 Classification rules

- **Protocol faults** are malformed task/payload/ref/state conditions. They use the task/protocol diagnostic group, fail the task, and never append a semantic record.
- **`NotConstantExpression`** means well-formed but not a constant expression for the requested purpose. It is not `Unsupported`.
- **`Unsupported`** means a construct outside the implemented/frozen subset. It must be explicit and must not be reported as success.
- **`ConstOverflow`** is a representation/resource-budget failure (`max_const_bits`/carrier capacity). It is **not** C signed overflow and not a `CommitError`.
- **`ConstConstraintViolation`** (proposed) is a C constraint/undefined-behavior condition in an ICE: division by zero, bad shift, signed overflow, invalid conversion, etc.
- **`LayoutError`** covers incomplete/invalid types, size/offset/stride overflow, invalid pack/alignment, bitfield constraints, and stale target/pack.
- **`InitError`** covers excess elements, invalid designators, non-constant static initializers, out-of-bounds initializers, string incompatibility, and invalid union selection.
- **`VlaError`** covers non-integer/zero/negative constant bounds, runtime checks, and jumps into a VLA scope.

### 9.2 Proposed error table

| Condition | Class | Carrier | Owner | Commit effect |
|---|---|---|---|---|
| Unknown task / wrong kind / wrong ref family / wrong arity | Protocol fault | `Fail` | T01/T08 adapter | None |
| Missing committed literal/const/node/semantic fact | Protocol/task fault | `Fail` | T08 | None |
| `required_kind` not satisfied | `NotConstantExpression` | `Fail` (`CONST_NOT_CONSTANT_EXPRESSION` or frozen code) | T08 | None |
| Construct outside the frozen subset | `ConstUnsupported` | `Fail` (`CONST_UNSUPPORTED`) | T08 | None |
| Result exceeds `max_const_bits`/carrier | `ConstOverflow` | `Fail` (`CONST_OVERFLOW`) | T08 | None |
| Signed overflow / div-by-zero / bad shift / `INT_MIN / -1` | `ConstConstraintViolation` | `Fail` (new named code) | T08 | None |
| Static assertion condition zero | `StaticAssertFailed` | `Fail` | CL07 | None |
| Static assertion non-ICE | `NotConstantExpression` | `Fail` | CL07/T08 | None |
| Incomplete/invalid type for layout | `LayoutIncomplete`/`LayoutInvalid` | `Fail` | CL08–CL14 | None |
| Size/offset/stride arithmetic overflow | `LayoutOverflow` | `Fail` | T08 | None |
| Bitfield width/zero-width/offsetof violation | `BitFieldConstraint` | `Fail` | CL12/CL14 | None |
| Stale layout (target/pack/type mismatch) | `StaleLayout` | `Fail` | CL26 | None |
| Initializer out of bounds/excess/duplicate | `InitConstraint` | `Fail` | CL16–CL21 | None |
| Static initializer not constant | `InitNotConstant`/`NotConstantExpression` | `Fail` | CL16/CL22 | None |
| VLA constant bound <= 0 | `VlaBoundConstraint` | `Fail` | CL15 | None |
| VLA runtime bound check | `VlaRuntimeCheck` plan | typed plan in `Layout`/`Init`; T09 consumes | CL15/T09 | None (runtime plan) |
| Jump into VLA scope | `VlaJumpIntoScope` | `Fail` | CL25/T09 | None |
| Unverified target needed for concrete result | `TargetUnverified`/`Unsupported` | `Fail` | T08/T11 | None |
| Capacity/append mismatch | Commit structural error | `CommitError` | commit | Whole batch reject/no partial state |

All numeric codes, group-local numbers, and exact rule IDs remain freeze items. Existing named codes are `CONST_OVERFLOW`, `CONST_UNSUPPORTED`, and `CONST_NOT_CONSTANT_EXPRESSION` ([diagnostic.rs](../../../compiler/src/diagnostic.rs)); the current use of `Task,3` for both a non-constant expression and a failed static assert is a taxonomy inconsistency (C-16).

## 10. Ownership matrix

| Store/field or record | Sole writer | Readers | Forbidden writers | Notes |
|---|---|---|---|---|
| `lex.literals` / `LiteralRecord` | T04 | T08, T13 | T08/T09/T11 | T08 consumes; never re-decodes or mutates. |
| `types.records` / `TypeRecord` | T06 (allowlist) | T08, T09, T11 | T08/T05/T09 | T08 must not complete or append types; CL20 requests T06. |
| `parse.nodes` / `NodeRecord` | T05 | T08, T07, T09, T13 | T08/T09 | T08 reads initializer/expression nodes only. |
| `sem.records` / `SemRecord` | T07 | T08, T09, T13 | T08 | T08 reads `ty`/checked operator/effects; no writes. |
| `constants.records` / `ConstRecord` | **T08** | T09, T13 | T04/T07/T09/T11 | T08 sole constant writer; T09 consumes exact `ConstId`, no refold. |
| `layout.records` / `LayoutRecord` | **T08** | T09, T11, T13 | T06/T09/T11 | T06 owns type facts; T08 owns layout. |
| `layout.vla` / `VlaDescriptor` (or `LayoutRecord.vla`) | **T08** | T09, T11 | T06/T09 | Runtime size/cleanup is T09/T11. |
| `init.records` / `InitRecord` | **T08** | T09, T11, T13 | T05/T09/T11 | One family for plan/image/runtime ops. |
| `ir.*` | T09 | T11, T13 | T08 | T08 never lowers IR. |
| `machine.*`/ABI plans | T11 | T13 | T08 | Part B/probe-gated. |
| `artifacts.fragments` | T03 | Host/T13 | T08 | T08 static image is an `Init` record, not an artifact. |
| `config.*` | T01/integrator | chips via projection | all chips | Read-only; no chip patch. |
| `target`/`TargetFingerprint` | T01 | T08/T11 | T08/T11 | Identity/attestation owned by T01; no private ABI copy. |
| `diagnostics` | T02/commit path | all | chips | Chips emit `DiagnosticDraft`/`Fail`. |
| `wires.proposals` | each chip | commit | no cross-write | Per-chip slot only. |

**Conclusion:** T08 is the sole writer of `constants.records`, `layout.records`, and `init.records`. T04/T06/T05/T07/T09/T11 are never second writers. Any need for a type/AST/artifact change is a request to that owner, not a T08 private record.

## 11. Determinism, replay, snapshot, and hash impact

### 11.1 Determinism requirements

- **Stable IDs:** append-only arenas; no ID reuse; no address/hash-map iteration; constants/layouts/inits reference committed IDs only. Newly enqueued tasks execute next tick; no same-tick child call.
- **Predicted IDs:** a chip that appends one `ConstRecord` predicts the next `ConstId` from the arena count, as `FoldChip` does today. A chip that appends multiple records (e.g., layout + VLA/plan) must use one `AppendRecords` batch with a deterministic draft order and predicted IDs; commit validates the prediction.
- **Layout reuse:** if implemented, scan committed `LayoutId`s in ascending order for an equal `LayoutKey`; return the lowest. No hidden cache. The scan is bounded by `max_records_per_arena`; document the complexity.
- **Member/step order:** declaration order for members, source order for initializer steps, then deterministic zero-range coalescing; relocation order by `(offset, symbol, addend)`; no sorting by hash/address.
- **Designators:** last-wins vs duplicate diagnostic must be frozen; the chosen rule must be deterministic and tested. Range designators must expand or summarize deterministically.
- **Zero initialization:** large arrays use `Zero` ranges/stride steps, not per-element records; the canonical range representation must be identical across replays.
- **Float:** exact raw bits, fixed rounding mode, no host FPU; NaN payload/signed-zero rules frozen before Part B.
- **VLA:** bound node/Const IDs and runtime flag are committed; the runtime size is not serialized as a constant; cleanup order is T09-owned and deterministic.
- **Target mode:** `Symbolic` vs `Probed(fingerprint)` is part of the record key and task input, so a Part A result cannot silently validate against a Part B target.

### 11.2 Snapshot and hash impact

| Change | Snapshot/hash impact | Required action |
|---|---|---|
| `ConstRecord` v2 (`ty`/tagged value) | Body encoding, field list, snapshot hash | New `/` version; update `snapshot.rs`, `contract.rs`, `freeze.rs`, `c08`. |
| `Literal` request gains `node` | Task payload/wire shape, trace/replay | T01/T07/T08 co-freeze; new kind/revision. |
| `ConstantResult.value: Option` | Result encoding | Snapshot/result encoding update; migration tests. |
| Layout/Init typed arenas | Reserved-count encoding replaced by bodies | `snapshot.rs` encoders; `contract.rs` store-family rows; `freeze.rs`. |
| `layout.records`/`init.records` schema fields | `StoreSchema` seed and manifest allowlist | `m1_slice()`/new `t08_slice()`; allowlist rows; hash. |
| `LayoutKey` target/pack | Layout body/reuse key | Target fingerprint; stale-layout validation. |
| `InitRecord` images/runtime ops | Large bodies and relocations | Byte budget; deterministic encoding; hash. |
| New task kinds/rules/error names | `NORMATIVE_RULES`, task registry, hash | Integrator-owned inventory; no local numbers. |
| Part A/B target mode | Config/snapshot/replay | Separate snapshots; no mixed validation. |

Current `snapshot.rs` encodes layout/init as reserved counts only; current `contract.rs` has no layout/init record field lists. Therefore adding typed layout/init is a real version/hash change, not a post-seed runtime declaration. Do not claim the `/39` hash covers it.

### 11.3 Replay gates

- Same input/config/initial state ⇒ identical `ConstId`/`LayoutId`/`InitId` allocation, identical record bodies, identical diagnostics, identical tick trace.
- `M1-REC-03`/`G15`/`VF13` remain the named replay gates; VF13 is draft only ([VF13_FREEZE](../../tasks/VF13_FREEZE.md)).
- A replay of a failed constant/layout task must be byte-identical, including the failure classification and no partial append.
- Part A and Part B runs are separate evidence classes; a Part A pass cannot validate a Part B layout/float/ABI result.

## 12. Conflicts with `/39` and current drafts

| ID | Conflict | Evidence | Severity | Recommended resolution |
|---|---|---|---|---|
| C-01 | `ConstRecord` has no `ty`; T09's candidate contract requires `ConstRecord.ty == ValueRecord.ty`. | [bus.rs](../../../compiler/src/bus.rs), [T09 checklist](../../tasks/T09_IR_LOWER_CHIPS.md#m1-only-ir-shape-checklist-one-function-candidate-not-frozen) | **High** | Add `ConstRecord.ty` in the freeze; update T09 check. |
| C-02 | Documented `ConstantRequest::Literal` carries per-use `node`; `/39` source decodes only a literal. | [task.rs](../../../compiler/src/task.rs), [T08](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md), [CDR §C](../../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md) | **High** | Add `[Node, Literal]` payload at the freeze; update tests. |
| C-03 | `RequiredKind` is only `IntegerConstantExpression`; CL01/CL06/CL07/CL08 need address/static/arithmetic/`sizeof` purposes. | [task.rs](../../../compiler/src/task.rs), [T08 table](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md) | **High** | Append required-kind variants/rules; do not repurpose lexical candidate type. |
| C-04 | `/39` `TypeKind` is `{Void,Bool,Char,Int,Function}`; layout needs pointer/array/struct/union/enum/bitfield/VLA facts. | [bus.rs](../../../compiler/src/bus.rs), [T06](../../tasks/T06_SYMBOL_TYPE_CHIPS.md) | **High** | T06 must freeze the additional type facts before T08 layout. |
| C-05 | `/39` `NodeKind` is the 8-node M1 tree; init/designator/`sizeof`/`_Static_assert`/compound literal nodes are absent. | [bus.rs](../../../compiler/src/bus.rs), [T05](../../tasks/T05_PARSE_CHIPS.md) | **High** | T05 must freeze the required AST forms; T08 consumes only. |
| C-06 | `layouts`/`inits` are `ReservedArena`; no typed records exist. | [bus.rs](../../../compiler/src/bus.rs) | **High** | Add typed `LayoutRecord`/`InitRecord` as a freeze. |
| C-07 | `G1DraftBody`/commit have no Layout/Init materialization. | [records.rs](../../../compiler/src/records.rs), [commit.rs](../../../compiler/src/commit.rs) | **High** | Add draft bodies, store paths, apply/preflight tests. |
| C-08 | `StoreSchema::m1_slice()` declares no `layout.*`/`init.*`; T08 manifests would fail registration. | [manifest.rs](../../../compiler/src/manifest.rs) | **High** | Add `t08_slice()`/fields + allowlist rows. |
| C-09 | Snapshot encodes only reserved layout/init counts; contract has no layout/init field lists. | [snapshot.rs](../../../compiler/src/snapshot.rs), [contract.rs](../../../compiler/src/contract.rs) | **High** | Add body encoders and hash inventory; version bump. |
| C-10 | `#pragma pack` is Ack-only (PP25); no committed pack stack/attribute carrier. | [T03](../../tasks/T03_PREPROCESS_CHIPS.md), [bus.rs](../../../compiler/src/bus.rs) | **High** | T03/T06 must commit effective pack/align on the type or a shared record. |
| C-11 | No committed target fingerprint; layout records cannot reject stale target values. | [target.rs](../../../compiler/src/target.rs), [T01 §6](../../tasks/T01_COMPILER_CONTRACT.md#6-target-and-language-model-must-be-frozen) | **High** | Add T01-owned `TargetFingerprint`/mode; `LayoutKey` includes it. |
| C-12 | `/37` branch/assert are Ack-only and cannot feed a value use; `?:` passes magnitude without common-type conversion. | [fold_branch.rs](../../../compiler/src/chips/constant_layout_init/fold_branch.rs), [c37](../../../compiler/tests/c37_const_branch.rs) | **High** | Final contract commits one `ConstRecord` for conditional value use; T07 supplies common type. |
| C-13 | `/37` accepts `0 && <dangling>`; final C semantics still require the unselected operand to be valid/type-checked. | [c37 test](../../../compiler/tests/c37_const_branch.rs) | **Medium** | Keep narrow test as selection-only; final contract requires committed semantic facts. |
| C-14 | Budget uses magnitude bit length only; positive signed values need a sign bit, negative values need two's-complement width. | [fold.rs](../../../compiler/src/chips/constant_layout_init/fold.rs), [c08 test](../../../compiler/tests/c08_gate1.rs) | **Medium** | Freeze one signed-width formula; current M1 values unaffected, boundary tests need update. |
| C-15 | CL20 output "CompletedType" would make T08 write `types.records`, conflicting with T06 sole writer. | [T08 table](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md), [T06](../../tasks/T06_SYMBOL_TYPE_CHIPS.md) | **High** | T08 infers bound; T06 completes type, or move CL20 to T06. |
| C-16 | `Task,3` is used for both non-constant expression and failed static assert; `ConstUnsupported` is defined but not used consistently. | [fold_branch.rs](../../../compiler/src/chips/constant_layout_init/fold_branch.rs), [diagnostic.rs](../../../compiler/src/diagnostic.rs) | **Medium** | Freeze distinct named codes; keep classifications separate. |
| C-17 | `/39` `ConstRecord` has no float/address payload; CL05/CL06/CL22 cannot represent their results. | [bus.rs](../../../compiler/src/bus.rs), [T08 table](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md) | **High** | Adopt tagged `ConstValue`; Part B for float/address specifics. |
| C-18 | `ConstantResult.value` is non-optional; non-legal paths carry a meaningless ref. | [task.rs](../../../compiler/src/task.rs) | **Low** | Change to `Option<RecordRef>` at freeze. |
| C-19 | M1 proposal excludes Layout/Init from M1 Part A; T08 table includes them. | [M1 proposal §3 item 6](../../tasks/M1_PART_A_CONTRACT_PROPOSAL.md), [T08 table](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md) | **Medium** | Keep layout/init post-M1/Part B; do not gate M1 on them. |
| C-20 | `pa_init.rs` is an uncommitted private candidate with chip-local `InitError`/`InitNodeShape`; not shared schema. | [pa_init.rs](../../../compiler/src/chips/parse/pa_init.rs) | **Medium** | Do not import; T05 must freeze shared AST forms. |
| C-21 | Concrete target values remain unverified; any layout/bitfield/float assertion would be false evidence. | [CONTRACT_VERSION](../../../compiler/contracts/CONTRACT_VERSION), [target.rs](../../../compiler/src/target.rs) | **Part B blocker** | No concrete record values until H01/H07/attest. |
| C-22 | T01 §3's full-contract "target width + bit-pattern" wording conflicts with the Part A symbolic records and the proposed type+mathematical value. | [T01 §3](../../tasks/T01_COMPILER_CONTRACT.md#3-ids-and-records) | **Medium** | Keep Part A symbolic; Part B derives bits or stores verified bits under an explicit rule. |
| C-23 | No byte budget exists for static init images/relocations. | [limits.rs](../../../compiler/src/limits.rs) | **Medium** | Reuse/add an explicit configured byte budget; no unbounded image. |
| C-24 | CL22/CL23/CL24 output carriers are not defined; T09/T11 consumers are unresolved. | [T08 table](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md), [T09](../../tasks/T09_IR_LOWER_CHIPS.md) | **High** | Freeze `InitRecord` forms and consumer ownership (T09 runtime, T11 image). |

## 13. Ordered freeze work orders

These are contract-freeze orders, not implementation authorizations. Each must be accepted by the named owners and the T01 integrator before code. **Order matters:** shapes first, then hash/allowlist, then consumers.

### F-01 — Constant record v2 and type link

- **Owner:** T08 + T01 `[INT]`; T06 consulted for `TypeId`.
- **Freeze:** `ConstRecord { ty, value: ConstValue }`; `IntegerConst`/`FloatConst`/`AddressConst`; `ConstantResult.value: Option`; exact field order/encoding; `ConstValue` names; error/legality split.
- **Exact paths:** `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/records.rs`, `compiler/src/commit.rs`, `compiler/src/snapshot.rs`, `compiler/src/contract.rs`, `compiler/tests/c08_gate1.rs`, `compiler/tests/freeze.rs`, `docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md`, `docs/tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md`, `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md`.
- **Gate:** `/39` tests migrated; T09 co-sign for `ConstRecord.ty`.

### F-02 — Constant request variants and task typing

- **Owner:** T07 + T08 + T01.
- **Freeze:** literal `node`; unary/logical/conditional/cast/address variant payload order; `RequiredKind` appended variants; task-kind spellings/local codes; result routing; numeric codes/rules.
- **Exact paths:** `compiler/src/task.rs`, `compiler/src/contract.rs`, `compiler/src/snapshot.rs`, `compiler/src/chips/constant_layout_init/fold.rs`, `compiler/src/chips/constant_layout_init/fold_branch.rs`, `compiler/tests/c08_gate1.rs`, `compiler/tests/c37_const_branch.rs`, T08/T07/T01 docs.
- **Gate:** F-01; no task code before freeze.

### F-03 — Layout/Init record schema and typed arenas

- **Owner:** T08 + T01.
- **Freeze:** `LayoutRecord`, `LayoutKey`, `LayoutExtent`, `LayoutMember`, `VlaDescriptor`, `InitRecord`, `InitStep`, `Reloc`; replace `ReservedArena` with typed arenas; draft bodies; commit materialization; snapshot encoders; store fields.
- **Exact paths:** `compiler/src/bus.rs`, `compiler/src/ids.rs` (only if an ID is added), `compiler/src/records.rs`, `compiler/src/commit.rs`, `compiler/src/snapshot.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/tests/freeze.rs`, T08/T01 docs.
- **Gate:** F-01/F-02 co-freeze; no layout/init values before target gate.

### F-04 — T06 layout/type facts

- **Owner:** T06 + T01.
- **Freeze:** public type facts for pointer/array/aggregate/enum/bitfield/FAM/VLA; member order/name/type/bit width; `pack`/`aligned`; completeness; array bound kind; VLA bound expression.
- **Exact paths:** `compiler/src/bus.rs` (`TypeKind`/`TypeRecord`), `compiler/src/chips/types/ty_types.rs`, `compiler/src/chips/types/ty_conv.rs`, `compiler/tests/c13_ty.rs`, `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md`.
- **Gate:** must precede I-03/I-04.

### F-05 — T05 initializer/constant-expression AST

- **Owner:** T05 + T01.
- **Freeze:** initializer list/scalar/designator/range/compound literal nodes; `sizeof`/`_Alignof`; `_Static_assert`; cast/conditional/comma forms needed by T08; token ranges/locations.
- **Exact paths:** `compiler/src/bus.rs` (`NodeKind`/`NodeRecord`), `compiler/src/chips/parse/pa_init.rs`, `compiler/src/chips/parse/pa_unary.rs`, `compiler/src/chips/parse/pa_cast.rs`, `compiler/src/chips/parse/pa_stmt.rs`, `compiler/tests/c34_parse.rs`, `compiler/tests/c35_expr.rs`, `docs/tasks/T05_PARSE_CHIPS.md`.
- **Gate:** must precede I-05/I-06; do not import `pa_init.rs` private shapes.

### F-06 — T03/T06 pack-state carrier

- **Owner:** T03 + T06 + T08.
- **Freeze:** effective `#pragma pack`/`aligned` state exposed to T06/T08 as a committed fact; no scalar in `Payload` unless a shared record carries it.
- **Exact paths:** `compiler/src/chips/preprocess/pp_pragma.rs`, `compiler/src/bus.rs`, `compiler/src/chips/types/ty_types.rs`, `docs/tasks/T03_PREPROCESS_CHIPS.md`, T06/T08 docs.
- **Gate:** layout record cannot be correct without it; may be explicitly unsupported in the first slice.

### F-07 — T01 target fingerprint and Part A/B mode

- **Owner:** T01.
- **Freeze:** `TargetFingerprint`/`ConstTargetMode`/`LayoutMode`; attested report hash identity; projection field; no unverified numbers in records.
- **Exact paths:** `compiler/src/target.rs`, `compiler/src/contract.rs`, `compiler/src/snapshot.rs`, `compiler/tests/c02_target.rs`, `compiler/tests/freeze.rs`, `compiler/contracts/CONTRACT_VERSION`, T01/T11 docs.
- **Gate:** Part B only; H01/H07.

### F-08 — Error/rule/code inventory and schema/hash

- **Owner:** T08 + T01 integrator.
- **Freeze:** named error codes, rule IDs, `Layout/Init` record field lists, `M1AppendSchema`/`t08_slice`, allowlist rows, task-kind registry, snapshot encoders, version/hash.
- **Exact paths:** `compiler/src/diagnostic.rs`, `compiler/src/contract.rs`, `compiler/src/manifest.rs`, `compiler/src/commit.rs`, `compiler/src/snapshot.rs`, `compiler/contracts/CONTRACT_VERSION`, `compiler/contracts/COMPILER_SFL_MANIFEST.md`, `compiler/tests/freeze.rs`.
- **Gate:** after F-01–F-07; one atomic version bump.

### F-09 — T09 consumption contract

- **Owner:** T09 + T08 + T01.
- **Freeze:** IR03 `ConstRecord.ty` check; IR24 init plan consumption; IR26 VLA dynamic stack; no refold; object identity for compound literals.
- **Exact paths:** `compiler/src/chips/ir_lower/*.rs`, `compiler/src/task.rs` (if result typing), `compiler/tests/c15_ir.rs`, T09/T08 docs.
- **Gate:** F-01/F-03 shapes.

### F-10 — T11 Part B layout/ABI consumption

- **Owner:** T11 + T01.
- **Freeze:** scalar/aggregate/bitfield/HVA layout mapping, `StaticImage` bytes/relocations, VLA frame rules; all values probe-gated.
- **Exact paths:** `compiler/src/target.rs`, future `compiler/src/chips/target/aarch64/*.rs`, `compiler/tests/c02_target.rs`, T11 docs, H01/H07 report.
- **Gate:** **BLOCKED** until the Linux probe is attested; no macOS values.

### F-11 — T13 verification/replay fixtures

- **Owner:** T13.
- **Freeze:** VF01 store invariants for layout/init; VF06 typed AST; VF13 replay; negative/overflow/unsupported fixtures.
- **Exact paths:** `compiler/src/chips/verify/vf_store.rs`, `compiler/src/chips/verify/vf_syntax.rs`, `compiler/src/chips/verify/vf_interpret.rs`, `compiler/src/chips/verify/vf_evidence.rs`, `compiler/src/chips/verify/vf_invariant.rs`, `compiler/tests/c19_vf01.rs`, `compiler/tests/c38_vf14.rs`, `compiler/tests/c17_vf12.rs`, T13 docs.
- **Gate:** after F-08; no pass claim from stubs.

## 14. Ordered implementation work orders

These are **post-freeze** and must not start on unfrozen interfaces. Proposed test paths are integrator-owned entry points; the integrator must register them per [PARALLEL_EXECUTION §2/§4](../../tasks/PARALLEL_EXECUTION.md). Each owner edits only its assigned chip file and test file.

| Order | Scope | Exact implementation paths (proposed) | Tests (proposed) | Dependency/gate |
|---|---|---|---|---|
| I-01 | CL01/CL02/CL03/CL05 constant core | `compiler/src/chips/constant_layout_init/const_context.rs`, `const_unary.rs`, `const_binary.rs`, `const_cast.rs`; migrate/replace `fold.rs` | `compiler/tests/c08_gate1.rs`, new `compiler/tests/c40_const_eval.rs` | F-01/F-02; M1 |
| I-02 | CL04/CL07 branch/static assert | `compiler/src/chips/constant_layout_init/const_branch.rs`, `static_assert.rs` (replace/extend `fold_branch.rs`) | `compiler/tests/c37_const_branch.rs`, new `c41_const_certify.rs` | F-02/F-05; T07 semantic facts |
| I-03 | CL08–CL11 scalar/array/struct/union layout | `scalar_layout.rs`, `array_layout.rs`, `struct_layout.rs`, `union_layout.rs` | new `c42_layout.rs` | F-03/F-04/F-06; Part B for concrete |
| I-04 | CL12–CL14 bitfield/FAM/member offset | `bitfield_layout.rs`, `flexible_array.rs`, `member_offset.rs` | `c42_layout.rs` | F-03/F-04/F-06; probe |
| I-05 | CL15/CL25 VLA | `vla_bound.rs`, `vla_lifetime.rs` | new `c43_vla.rs` | F-03/F-04; T09 CFG |
| I-06 | CL16–CL21 init plans | `scalar_initializer.rs`, `aggregate_initializer.rs`, `designated_initializer.rs`, `string_initializer.rs`, `incomplete_array_infer.rs`, `zero_initialize.rs` | new `c44_init_plan.rs` | F-03/F-05; T06 completion |
| I-07 | CL22–CL24 static/runtime/compound | `static_data_emit.rs`, `automatic_init_plan.rs`, `compound_literal.rs` | new `c45_init_emit.rs` | F-03/F-07; Part B for image |
| I-08 | CL26 layout validate | `layout_validate.rs` | `c45_init_emit.rs`/new `c46_layout_validate.rs` | F-03/F-07 |
| I-09 | T09 consumption | `compiler/src/chips/ir_lower/ir_constant.rs`, `ir_init.rs`, `ir_vla.rs` | new `c47_ir_const_init.rs` | F-09; after I-01/I-06 |
| I-10 | T11 Part B consumption | future `compiler/src/chips/target/aarch64/*.rs`; `compiler/src/target.rs` only if F-07 changes | new `c48_target_layout.rs` | F-10; **probe attested** |
| I-11 | T13 verification/replay | `compiler/src/chips/verify/vf_store.rs`, `vf_syntax.rs`, `vf_interpret.rs`, `vf_evidence.rs`, `vf_invariant.rs` | `compiler/tests/c19_vf01.rs`, `compiler/tests/c38_vf14.rs`, new replay fixtures | F-11; after I-01–I-09 |
| I-12 | Integration/schema/allowlist | only integrator-owned files: `compiler/src/manifest.rs`, `commit.rs`, `records.rs`, `snapshot.rs`, `contract.rs`, `compiler/contracts/*`, `compiler/src/chips/mod.rs` | `freeze.rs`, `c04_manifest.rs`, `c06_routing.rs` | F-08; never chip owner |

**Order:** I-01/I-02 first for M1; I-03–I-08 are post-M1/Stage D; I-09 follows; I-10 is Part B and remains blocked until F-10; I-11 follows functional slices; I-12 is continuous integration, not a semantic chip.

## 15. Open questions

Each question is unresolved unless marked otherwise; recommendations are proposals, not decisions.

1. **`ConstRecord` migration:** replace `/39` `{value,negative}` with `{ty,value: ConstValue}` (recommended) or add a parallel record family? Owner T08/T01. (C-01/C-17)
2. **Literal `node`:** add the per-use `NodeId` to `ConstantRequest::Literal` (recommended for diagnostics/type) or keep a node-free literal flow? Owner T07/T08/T01. (C-02)
3. **`RequiredKind` length:** which variants beyond `IntegerConstantExpression` are needed for address/static/arithmetic/`sizeof`/`_Static_assert`? Owner T07/T08/T01. (C-03)
4. **Part B integer representation:** derive bits in T11 from mathematical value + type, or store a verified bit pattern in `ConstRecord`? Owner T11/T08/T01. (C-22)
5. **Signed overflow policy:** is ICE signed overflow always a diagnostic, or does `-fwrapv`/dialect change it? Freeze config semantics; no chip default. Owner T07/T01.
6. **Bit budget formula:** current magnitude-only bits vs minimal two's-complement width (`1` for 0/-1, `bits(v)+1` for positive, `bits(|v|-1)+1` for negative). Recommend the latter; update boundary tests. Owner T08/T01. (C-14)
7. **Float scope:** which formats, rounding, NaN/signed-zero, `long double`; is any float constant allowed in Part A? Owner T08/T11/T01. (C-21)
8. **Address constants:** exact `AddressConst` carrier, null representation, addend range, relocation/TLS policy. Owner T08/T11/T01. (C-17)
9. **Layout target identity:** exact `TargetFingerprint`/`LayoutMode` shape and whether Part A layout records are stored or only structural checks. Owner T01/T08. (C-11)
10. **T06 type scope:** which of pointer/array/aggregate/enum/bitfield/FAM/VLA facts are frozen for M2/M3; exact `TypeKind` shape is T06's. Owner T06/T01. (C-04)
11. **Pack/aligned carrier:** committed T06 type attributes vs a T03 pack-event record; per-member `aligned` representation. Owner T03/T06/T08. (C-10)
12. **Init tree ownership:** committed T05 AST subtree vs a T08 plan-only representation; how designators/compound literals are represented. Owner T05/T08. (C-05/C-20)
13. **Init record split:** one discriminated `InitRecord` (recommended) vs separate plan/image/ops families; static image byte budget. Owner T08/T01. (C-23/C-24)
14. **VLA runtime policy:** negative/zero runtime bound check/trap; cleanup ownership; whether CL25 emits a record or only a requirement. Owner T08/T09. (C-24)
15. **Bitfield ABI:** allocation-unit/straddle/bit-numbering policy for AAPCS64; probe/H07 classification. Owner T08/T11/T01. (C-21)
16. **Constant identity:** no dedup (recommended for M1) vs deterministic lowest-id reuse. Owner T08/T01.
17. **CL20 type completion:** T06 owns type append; T08 only infers the bound (recommended). Owner T06/T08. (C-15)
18. **Duplicate/range designators:** last-wins vs diagnostic; GNU ranges dialect-gated. Owner T08/T05.
19. **Static assert message:** optional string `LiteralRecord`; how diagnostics quote it without leaking unrelated data. Owner T04/T08/T02.
20. **Numeric codes/rule IDs/hash:** all proposed names remain open; one integrator inventory. Owner T01. (C-16)
21. **M1 vs post-M1:** confirm layout/init remain outside M1 acceptance (recommended), so T08 layout work cannot block M1. Owner T01/T08. (C-19)
22. **`ConstRecord` `ty` for address constants:** pointer type vs integer null constant before conversion; T06/T07 conversion ordering. Owner T06/T07/T08.

## 16. Rejected alternatives

1. **Private T08 constant/layout/init types.** Rejected: violates one shared schema and T01 ownership; T09/T11 cannot consume them.
2. **Host `i32`/`i128`/`f64`/`usize` as the C value.** Rejected: target-width, overflow, rounding, and determinism would follow the Rust host.
3. **Bit pattern without type.** Rejected: unsignedness/signedness and T09 `ConstRecord.ty` checks become impossible.
4. **Keep `ConstRecord` tyless and derive type only from the use.** Rejected: same `ConstId` could mean different types; T09 check cannot hold.
5. **Parallel `FloatConstRecord`/`AddressConstRecord` families.** Rejected for the first freeze: duplicate ownership and multiple `Const` concepts; one tagged payload is smaller.
6. **Per-element initialization nodes for large arrays.** Rejected: violates T08's explicit huge-array requirement and creates quadratic/space blowup.
7. **Layout records with unverified numbers.** Rejected: would treat expected AAPCS64/macOS values as facts.
8. **Raw `LayoutId`/`ConstId` indices in task payloads.** Rejected: payloads use typed `RecordRef`; raw indices lose family/tag checking and stable identity.
9. **T08 writes `types.records` to complete an array/compound type.** Rejected: T06 is the sole type writer; use a T06 request.
10. **T09 re-folds constants or re-lays-out types.** Rejected: violates the T08/T09 split and replay identity.
11. **Commit recomputes/validates constant values.** Rejected: constants are chip diagnostics; commit only checks structure/capacity/refs.
12. **One `Unsupported` for every failure.** Rejected: loses `NotConstantExpression` vs `ConstOverflow` vs constraint vs target-unverified distinctions and damages acceptance accounting.
13. **A VLA runtime size as a `ConstRecord`.** Rejected: VLA size is runtime; a static constant would be fabricated.
14. **Keep `ReservedArena` as the final layout/init schema.** Rejected: no records, no snapshots, no manifests, no consumers.
15. **Treat `/37` Ack-only branch/assert as the final value contract.** Rejected: no value for T09/static initialization; only a narrow selection certificate.
16. **Float folding with host `f64` or Rust `f32`.** Rejected: NaN/signed-zero/rounding/`long double` mismatch.
17. **Bitfield layout via Rust `#[repr]`/`size_of`.** Rejected: host ABI is not the target ABI.
18. **Address constants as host pointers or absolute target addresses.** Rejected: non-replayable and non-relocatable.
19. **New `ResultValue` variant for constants/layout/inits.** Rejected: use `ResultValue::Record` with `RecordRef::Const`/`Layout`/`Init`; no envelope growth.
20. **`max_const_bits` as the target `int` width.** Rejected: it is a resource/representation budget; target width comes from `TargetSpec`.
21. **Static assert failure as a `CommitError`.** Rejected: it is a chip diagnostic; commit does not decide C semantics.
22. **Importing `pa_init.rs`'s `InitNodeShape`/`InitError` into T08.** Rejected: chip-local uncommitted candidate; not shared or frozen.
23. **Using `#pragma pack` PP classification directly as layout state.** Rejected: PP25 is Ack-only; layout needs a committed effective pack fact.
24. **A second target model copy in T08 records.** Rejected: T01 owns the target identity; use a fingerprint/projection.

## 17. Checks and non-claims

### 17.1 Checks actually run

```sh
cargo test --manifest-path compiler/Cargo.toml --test c08_gate1 --test c37_const_branch
```

Result: `c08_gate1` 21 passed, `c37_const_branch` 10 passed, 0 failed. The build emitted 137 warnings from uncommitted/untracked parse candidates; those warnings are not a T08 verdict. This is a baseline narrow-slice check, not a full suite and not M1-CL-05.

### 17.2 Not run / not claimed

- No full `cargo test`/`clippy`/`fmt` repository verdict.
- No Linux probe, no `TargetSpec::attest`, no H01/H07, no concrete ABI/layout/bitfield/float value.
- No GCC torture corpus, denominator, pass rate, or T00 gate.
- No M1-CL-05 real T07→T08→T09 chain; no layout/init/VLA/float/address chip implementation.
- No `/6` freeze, no version bump, no hash change, no commit/push, no agent dispatch.
- No claim that the current `/39` `ConstRecord`/`ConstantRequest` shape is complete or that T08 layout/init are implemented.

## 18. References

- [T08: Constant, Layout, Initialization, and Dynamic Object Chips](../../tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md)
- [T01: Compiler Bus and Protocol Freeze](../../tasks/T01_COMPILER_CONTRACT.md)
- [CDR: M1 Pipeline and Part A Schema](../../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)
- [M1 Part A Contract Proposal](../../tasks/M1_PART_A_CONTRACT_PROPOSAL.md)
- [T06: Scope, Symbol, and Type Chips](../../tasks/T06_SYMBOL_TYPE_CHIPS.md)
- [T09: AST-to-CFG/IR Chips](../../tasks/T09_IR_LOWER_CHIPS.md)
- [T11: AArch64 Linux ABI, Machine IR, and Assembly Chips](../../tasks/T11_TARGET_CHIPS.md)
- [M1 Vertical-Slice Acceptance](../../tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md)
- [Compiler Development Guardrails](../../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md)
- [Parallel Execution and Handoff](../../tasks/PARALLEL_EXECUTION.md)
- [TASK_TEMPLATE](../../tasks/TASK_TEMPLATE.md)
- [Mass-Parallel Dispatch Plan](../../tasks/MASS_PARALLEL_DISPATCH.md)
- [Frozen Contract Version](../../../compiler/contracts/CONTRACT_VERSION)
- [Compiler SFL Manifest Extension](../../../compiler/contracts/COMPILER_SFL_MANIFEST.md)
- [bus.rs](../../../compiler/src/bus.rs)
- [task.rs](../../../compiler/src/task.rs)
- [records.rs](../../../compiler/src/records.rs)
- [commit.rs](../../../compiler/src/commit.rs)
- [manifest.rs](../../../compiler/src/manifest.rs)
- [snapshot.rs](../../../compiler/src/snapshot.rs)
- [contract.rs](../../../compiler/src/contract.rs)
- [target.rs](../../../compiler/src/target.rs)
- [diagnostic.rs](../../../compiler/src/diagnostic.rs)
- [limits.rs](../../../compiler/src/limits.rs)
- [fold.rs](../../../compiler/src/chips/constant_layout_init/fold.rs)
- [fold_branch.rs](../../../compiler/src/chips/constant_layout_init/fold_branch.rs)
- [c08_gate1.rs](../../../compiler/tests/c08_gate1.rs)
- [c37_const_branch.rs](../../../compiler/tests/c37_const_branch.rs)
- [pa_init.rs (uncommitted candidate, not shared schema)](../../../compiler/src/chips/parse/pa_init.rs)
