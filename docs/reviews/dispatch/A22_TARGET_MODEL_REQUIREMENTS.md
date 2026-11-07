# A-ABI-02 — Target Integer and Floating-Point Model Requirements

| Field | Value |
|---|---|
| Task | A-ABI-02 (Stage A requirements package; mass-parallel dispatch) |
| Status | **Design/requirements package only.** Read-only inspection. No contract freeze, no authority decision, no source/test/contract change, no implementation authorization, no probe, no verification claim. |
| Date | 2026-11-07 |
| Repository baseline | `430ae29` (`docs: add mass-parallel compiler dispatch plan`); workspace contains unrelated uncommitted user/parallel-agent files, which this report does not touch. |
| Frozen contract | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` (`compiler/contracts/CONTRACT_VERSION:3-4`; `compiler/src/contract.rs:38,44`). |
| Target identity | Frozen: `aarch64-unknown-linux-gnu` / ELF / LP64 / little-endian / AAPCS64 (`docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md:72-88`; `docs/tasks/T01_COMPILER_CONTRACT.md:121-128`). |
| Target concrete values | **UNVERIFIED.** No Linux AArch64 probe report, no H07 classification, no attestation, no `Probed` target exists. |
| Evidence used | Static source/document inspection plus one targeted test run: `cargo test --manifest-path compiler/Cargo.toml --test c02_target` → **11 passed, 0 failed** (2026-11-07). No full suite, no probe, no certification. |
| Deliverable path | `docs/reviews/dispatch/A22_TARGET_MODEL_REQUIREMENTS.md` (this file) |

---

## 1. Executive Result

The repository has a **frozen target identity**, a real but narrow **scalar size/alignment proposal table**, and a real **attestation mechanism**. It does **not** yet have a C integer model or a C floating-point model. The current `TargetSpec` cannot express:

- integer value ranges, value bits, padding bits, representation, or conversion rank;
- `_Bool` representation and conversion, or plain-`char` signedness consumption;
- target formats for `float` and `double` (only `long double` has a format label);
- integer overflow, shift, division, promotion, usual-arithmetic-conversion, or cast rules;
- floating-point rounding, NaN, signed-zero, subnormal, `fenv`, or `FLT_EVAL_METHOD` rules;
- enum underlying types, bit-field allocation, aggregate layout, `_Atomic`, VLA stack dependencies, or a pointer representation model;
- a versioned target-model schema distinct from the probe-report schema and the T01 contract hash.

The current probe contract has **38 required fields** (`compiler/src/target.rs:493-532`) and measures **13 scalar kinds** (`compiler/src/target.rs:327-341`). After `TargetSpec::attest`, eight `ScalarKind` entries still carry the unverified constructor defaults (`Bool`, `SChar`, `UChar`, `UShort`, `UInt`, `ULong`, `ULongLong`, `UInt128`), and `Float`/`Double` keep proposed format labels rather than attested ones. This is a **model-completeness defect**, not merely a probe-substrate delay.

Several already-registered chips also consume target-dependent assumptions without checking `require_verified`:

- `compiler/src/chips/lex/lx_string.rs` reads the unverified `wchar_t` size/encoding and requires UTF-32 for wide strings (`:575-587`, `:645-663`);
- `compiler/src/chips/lex/lx_char.rs` hard-codes `L`/`U` character units as 32-bit (`:44-58`, `:76-98`);
- `compiler/src/chips/lex/lx_float_value.rs` hard-codes binary32/binary64 for `float`/`double` suffixes (`:484-490`, `:539-630`);
- `compiler/src/chips/lex/lx_escape.rs` assumes a UTF-8 target execution character set (`:22-36`);
- no chip in `compiler/src/chips/` calls `TargetSpec::require_verified` (static grep).

The current attestation mechanism is honest about trust (it validates caller-supplied data and a hash, not physical provenance), but it does not enforce model completeness or plausibility. The current contract artifact also cannot represent an attested target: `FrozenSchema::current()` always uses the unverified target (`compiler/src/contract.rs:502-512`) and `contract_version_file()` hard-codes `target_verification=unverified` (`compiler/src/contract.rs:785-800`).

**Bottom line:** Part A symbolic work may continue independently. Every Part B integer/floating rule remains **blocked** until a versioned model schema, an extended probe, H07 classification, integrator attestation, and consistency checks exist. No concrete ABI value is verified by this report.

---

## 2. Current Model and Evidence

### 2.1 Frozen identity (known, not value verification)

| Identity field | Frozen value | Source | Status |
|---|---|---|---|
| Triple | `aarch64-unknown-linux-gnu` | `compiler/src/target.rs:425-451`; `docs/tasks/T01_COMPILER_CONTRACT.md:121-123` | **FROZEN IDENTITY** |
| Object format | ELF | `compiler/src/target.rs:47-55`; `TargetProfile` | **FROZEN IDENTITY** |
| Data model | LP64 | `compiler/src/target.rs:68-91`; ADR-0001 §3 | **FROZEN RELATION**, not measured values |
| Endianness | little | `compiler/src/target.rs:28-44`; ADR-0001 §3 | **FROZEN IDENTITY** |
| ABI name | AAPCS64 | `compiler/src/target.rs:343-359`; ADR-0001 §3 | **FROZEN IDENTITY** |
| Target values | every size/alignment/format/register value | `compiler/contracts/target/aarch64-linux-probe.txt:1-15`; `compiler/contracts/CONTRACT_VERSION:6` | **UNVERIFIED** |

The data model is a declared relation. `TargetSpec::attest` checks the `data_model` label but does **not** cross-check it against measured `sizeof(long)`, `sizeof(int)`, or `sizeof(pointer)` (`compiler/src/target.rs:669-673`). A hash-signed report with an LP64 label and inconsistent measured widths would pass the current code.

### 2.2 Current `TargetSpec` shape

`TargetSpec` is private-field and copyable (`compiler/src/target.rs:563-575`):

```text
TargetSpec {
    profile: TargetProfile,                 // frozen identity
    verification: VerificationState,        // private Unverified | Probed { report_hash }
    scalars: [ScalarModel; 21],             // size u8, align u8, signed Option<bool>, float Option<FloatFormat>
    abi: AbiSpec,                           // name, gp_arg_regs, fp_arg_regs, stack_align, variadic_register_save_area
    wchar_encoding: WideCharEncoding,       // Utf32 | Utf16
}
```

Relevant enums and shapes:

| Item | Definition | What it can express | What it cannot express |
|---|---|---|---|
| `ScalarKind` (21) | `Bool`, `Char`, `SChar`, `UChar`, `Short`, `UShort`, `Int`, `UInt`, `Long`, `ULong`, `LongLong`, `ULongLong`, `Int128`, `UInt128`, `Float`, `Double`, `LongDouble`, `Pointer`, `SizeT`, `PtrDiffT`, `WCharT` (`target.rs:169-242`) | standard scalar names plus `__int128`, `size_t`, `ptrdiff_t`, `wchar_t`, `void *` | arrays, functions, structs, unions, enums, bit-fields, `_Atomic`, `_FloatN`, `char16_t`/`char32_t` |
| `ScalarModel` | `size: u8`, `align: u8`, `signed: Option<bool>`, `float: Option<FloatFormat>` (`target.rs:133-166`) | byte size, byte alignment, signedness for integers, format label for floats | bits, value bits, padding/trap representation, ranges, integer representation, float precision/exponent, NaN/rounding policy |
| `FloatFormat` | `IeeeBinary16`, `IeeeBinary32`, `IeeeBinary64`, `IeeeBinary128`, `X87Extended` (`target.rs:93-131`) | storage format label | precision/range fields, rounding mode, NaN/zero/subnormal policy, `FLT_EVAL_METHOD`, `fenv` |
| `AbiSpec` | `name`, `gp_arg_regs`, `fp_arg_regs`, `stack_align`, `variadic_register_save_area` (`target.rs:343-359`) | four coarse AAPCS64 facts | HFA/HVA rules, aggregate classes, return/sret rules, register names, x18, TLS, relocations, target features |
| `WideCharEncoding` | `Utf32`, `Utf16` (`target.rs:535-561`) | wide-character encoding label | `wchar_t` range, signedness semantics, execution charset, `char16_t`/`char32_t` model |

`aarch64_unknown_linux_gnu_unverified()` hard-codes the current proposal table (`target.rs:577-620`). Those numbers are **proposal values, not requirements and not evidence**. Examples (explicitly `[PROPOSAL, UNVERIFIED]`): `Bool`/`Char` `1/1`, `Int` `4/4`, `Long` `8/8`, `Int128` `16/16`, `LongDouble` `16/16` with `IeeeBinary128`, `Pointer` `8/8`, `WCharT` `4/4` signed, `AbiSpec` `8/8/16/true`. No conclusion may be drawn from them.

### 2.3 Current probe contract and attestation mechanism

`REQUIRED_PROBE_FIELDS` contains 38 keys (`target.rs:493-532`):

| Group | Fields | Count |
|---|---|---:|
| Identity | `triple`, `object_format`, `endianness`, `data_model` | 4 |
| `char` | `sizeof.char`, `alignof.char`, `char.signed` | 3 |
| `short` | `sizeof.short`, `alignof.short` | 2 |
| `int` | `sizeof.int`, `alignof.int` | 2 |
| `long` | `sizeof.long`, `alignof.long` | 2 |
| `long long` | `sizeof.longlong`, `alignof.longlong` | 2 |
| `__int128` | `sizeof.__int128`, `alignof.__int128` | 2 |
| `float` | `sizeof.float`, `alignof.float` | 2 |
| `double` | `sizeof.double`, `alignof.double` | 2 |
| `long double` | `sizeof.longdouble`, `alignof.longdouble`, `longdouble.format` | 3 |
| pointer | `sizeof.pointer`, `alignof.pointer` | 2 |
| `size_t` | `sizeof.size_t`, `alignof.size_t` | 2 |
| `ptrdiff_t` | `sizeof.ptrdiff_t`, `alignof.ptrdiff_t` | 2 |
| `wchar_t` | `sizeof.wchar_t`, `alignof.wchar_t`, `wchar_t.signed`, `wchar_t.encoding` | 4 |
| ABI | `abi.gp_arg_regs`, `abi.fp_arg_regs`, `abi.stack_align`, `abi.variadic_register_save_area` | 4 |
| **Total** | | **38** |

`MEASURED_SCALARS` contains 13 kinds (`target.rs:327-341`): `Char`, `Short`, `Int`, `Long`, `LongLong`, `Int128`, `Float`, `Double`, `LongDouble`, `Pointer`, `SizeT`, `PtrDiffT`, `WCharT`.

`TargetSpec::attest` (`target.rs:658-716`) performs these steps:

1. `report.verify_verified()` — requires `verified = true`, a 64-hex `report_hash`, and SHA-256 over the canonical unsigned body.
2. `report.require_fields(REQUIRED_PROBE_FIELDS)` — presence only.
3. `report.expect(...)` for `triple`, `object_format`, `endianness`, `data_model` — exact string identity.
4. Starts from `aarch64_unknown_linux_gnu_unverified()` and overwrites `size`/`align` for `MEASURED_SCALARS` only.
5. Overwrites `Char.signed`, `WCharT.signed`, `WCharT.encoding`, `LongDouble.float`, and the four `AbiSpec` values.
6. Sets `VerificationState::Probed { report_hash }`.

What remains at proposal defaults after attestation:

| Kind | Fields left un-attested |
|---|---|
| `Bool` | `size`, `align`, `signed` |
| `SChar` | `size`, `align`, `signed` |
| `UChar` | `size`, `align`, `signed` |
| `UShort` | `size`, `align`, `signed` |
| `UInt` | `size`, `align`, `signed` |
| `ULong` | `size`, `align`, `signed` |
| `ULongLong` | `size`, `align`, `signed` |
| `UInt128` | `size`, `align`, `signed` |
| `Float` | `float` format (only `size`/`align` measured) |
| `Double` | `float` format (only `size`/`align` measured) |
| `Pointer` | `signed` interpretation; no representation model |
| `SizeT`/`PtrDiffT` | signedness is a proposal; only size/align measured |
| `WCharT` | range/representation; only size/align/signed/encoding are handled |
| `Int128` | signedness/repr/range; only size/align measured |

`ProbeReport::status()` is weaker than `attest`: after a valid `verified`/`report_hash` and field presence it can return `ProbeStatus::Verified` even while the four ABI fields are `unresolved` (`compiler/README.md:86-90`). It must not be used as the codegen gate.

Attestation limits are documented and real: it validates caller-supplied report data and the report hash; it does **not** prove that a physical probe ran, that the report came from H01/H07, or that the bytes were not substituted (TOCTOU). Trust and freshness are integration responsibilities (`compiler/src/target.rs:10-16`; `compiler/README.md:92-100`).

### 2.4 Current evidence from tests

The targeted run `cargo test --manifest-path compiler/Cargo.toml --test c02_target` produced **11 passed, 0 failed**. The tests cover:

| Test | What it establishes | What it does not establish |
|---|---|---|
| `target_profile_identity_is_frozen` | triple/ELF/LP64/little/AAPCS64 labels | measured values |
| `concrete_values_are_unverified_until_probed` | `UNVERIFIED`, no report hash, `require_verified` fails | that the proposal values are correct |
| `no_darwin_or_macos_values_are_present` | fixture text contains no `darwin`/`macos`/`x86_64`/`ilp32`/`mach-o` | that an attested report could not contain them |
| `probe_fixture_is_unverified_and_matches_every_required_field` | fixture is `verified=false` and matches the proposal table | that the proposal table is the ABI |
| `attested_report_produces_a_verified_target` | a caller-built signed report can be attested | provenance or physical probe |
| `attestation_rejects_tampered_and_incomplete_reports` | hash mismatch, missing field, wrong triple rejected | model completeness/plausibility |
| `attestation_rejects_an_unknown_wchar_encoding` | unknown `wchar_t.encoding` rejected | `utf16`/format plausibility |
| `config_try_new_*` / `config_error_codes_are_stable` | config validation and stable codes | target model semantics |

No test exercises integer ranges, integer representation, `_Bool`, plain-`char` conversion, `float`/`double` format attestation, NaN/zero/fenv, promotions, or target-width constants.

### 2.5 Current Part A symbolic records

| Record/rule | Current implementation | Source | Status |
|---|---|---|---|
| `TypeKind::Void` | symbolic `void` | `compiler/src/bus.rs:295-323` | implemented symbolic |
| `TypeKind::Bool` | symbolic `_Bool` | same | implemented symbolic, no conversion/size |
| `TypeKind::Char(CharKind)` | `Plain`/`Signed`/`Unsigned` | `bus.rs:272-281`, `295-323` | implemented symbolic; `Plain` signedness unresolved |
| `TypeKind::Int { rank, signed }` | `IntRank::{Short,Int,Long,LongLong}` + `signed` | `bus.rs:259-270`, `295-323` | implemented symbolic; no width/range |
| `TypeKind::Function { result, params, prototype, variadic }` | symbolic signature | `bus.rs:313-322` | implemented symbolic; no ABI locations |
| canonical `int` | `m1_int()` = `Int { rank: Int, signed: true }`; deterministic lowest matching `TypeId` scan | `compiler/src/chips/types/ty_types.rs:23-31`, `50-58`, `143-234` | implemented M1 Part A |
| identity conversions | `TyConvChip` accepts only `Int` operands and completes with the operand `TypeId`; no `ConversionPlan` | `compiler/src/chips/types/ty_conv.rs:21-28`, `151-185` | implemented M1 identity only; non-`Int` `Unsupported` |
| `LiteralRecord` | token, kind, radix, suffix, big-endian magnitude, negative, spelling, `Lx08CandidateType` | `compiler/src/bus.rs:564-588`; `compiler/src/snapshot.rs:1335-1345` | implemented; `Int` only, no width |
| `ConstRecord` | big-endian magnitude + negative; **no `ty` field** | `compiler/src/bus.rs:590-602`; `compiler/src/contract.rs:347-348` | implemented M1; type comes from consuming `Sem`/IR value |
| const fold | decimal `Integer`, `None` suffix, `Int` candidate, `Add`; checked big-endian magnitude addition; `max_const_bits` budget; `ConstOverflow` | `compiler/src/chips/constant_layout_init/fold.rs:151-293` | implemented M1 `2+3=5` only |
| `SemRecord` | node, symbolic `ty`, `NonLvalue`, `EffectMask(0)` | `compiler/src/bus.rs:503-513`; `compiler/src/chips/semantic/se_literal.rs:207-225` | implemented M1 |
| IR `Constant` | references `ConstId`; result `ValueRecord.ty` is symbolic `int` | `compiler/src/chips/ir_lower/ir_function.rs`; `compiler/src/chips/verify/vf_interpret.rs:1-14`, `254-326` | implemented symbolic |
| `VF12` interpreter | symbolic covered subset; no target widths; unsupported never pass | `vf_interpret.rs:1-14`, `254-326` | implemented symbolic |

### 2.6 Current Part B leaks and fail-closed gaps

| Location | Current behavior | Why it is a target-model issue | Required disposition |
|---|---|---|---|
| `compiler/src/chips/lex/lx_string.rs:575-587` | projects `WCharT.size` and `WideCharEncoding` from `bus.config().target()` without `require_verified` | wide-string code units, width, endianness, and trailing-zero representation depend on the unverified `wchar_t` model | gate the rule on a verified model, or mark the output proposal-only and never consume it in Part B |
| `compiler/src/chips/lex/lx_string.rs:645-663` | fails unless `wchar_utf32`; emits 2- or 4-byte wide units | UTF-32 is a proposal; `wchar_t` encoding/size is unresolved | require attested `wchar_t` encoding/width; reject unknown |
| `compiler/src/chips/lex/lx_char.rs:44-58`, `76-98` | treats `L`/`U` units as 32-bit; folds value to low 32 bits | `wchar_t` may not be 32-bit; `L'...'` type/value is target-dependent | gate wide/`char32_t` character constants or keep them explicitly syntax-only |
| `compiler/src/chips/lex/lx_float_value.rs:484-490`, `539-630` | suffix maps to `IeeeBinary32`/`IeeeBinary64`; integer core assumes 24/53-bit significands | target `float`/`double` formats are not attested; `float`/`double` `FloatFormat` is not updated by `attest` | require attested `format.float`/`format.double`; keep `Ack`-only until then |
| `compiler/src/chips/lex/lx_escape.rs:22-36` | assumes UTF-8 target execution character set | execution character set is a target model element | record/consume target execution charset; otherwise mark assumption |
| `compiler/src/chips/lex/lx_char.rs:229-264` | raw source bytes each become one unit; no multibyte composition | source/execution character set and `MB_LEN_MAX` are unresolved | target execution-charset model |
| `compiler/src/chips/preprocess/pp_builtin.rs:389-436` | emits only identity macros (`__aarch64__`, `__linux__`, `__ELF__`, `__LP64__`, `__BYTE_ORDER__`) | safe: no widths, no unverified values | keep identity-only; add width macros only after attestation |
| `compiler/src/bin/candidate.rs:104-106` | refuses `-S`/`-c` while unverified | correct fail-closed behavior | preserve until attestation |

No chip in `compiler/src/chips` calls `require_verified` or `ensure_codegen_ready`. The codegen gate exists only in `TargetSpec::require_verified` (`target.rs:647-656`) and `CompilerConfig::ensure_codegen_ready` (`target.rs:1344-1347`), plus the H04 driver's `-S`/`-c` refusal.

### 2.7 What is not implemented

- No integer model beyond size/align/signedness proposals.
- No integer conversion rank/ranges, no representation, no overflow policy.
- No target-width integer arithmetic or bit-vector type.
- No promotions except the `int→int` identity.
- No usual arithmetic conversions, assignment conversions, argument conversions, or explicit casts beyond identity.
- No typing/format for `float`, `double`, or `long double` in `TypeKind`; no float constant record.
- No `_Bool` conversion, no plain-`char` signedness consumption, no `wchar_t` type in the type system.
- No enum underlying type, bit-field layout, aggregate layout, `_Atomic`, or VLA target model.
- No pointer representation, null-pointer representation, pointer-integer conversion, or provenance model.
- No target ABI classification beyond four coarse fields.
- No versioned target-model schema and no attestation of model completeness.

---

## 3. Definitions and Evidence Classes

| Term | Meaning in this report |
|---|---|
| **Part A** | Target-independent symbolic contract and M1 Part A records: types carry rank/signedness but no width; constants carry mathematical magnitude/sign but no target bit pattern; no `sizeof`/alignment is asserted. |
| **Part B** | Target-dependent, probe-gated values and rules: concrete sizes/alignments/ranges/formats, target-width arithmetic, promotions/conversions, layout, ABI, float environment, VLA/stack behavior. |
| **Frozen identity** | The five `TargetProfile` dimensions declared by ADR-0001/T01 §6. Identity is not value verification. |
| **Attested value** | A value parsed from a canonical, hash-signed probe report accepted by `TargetSpec::attest`, with a `VerificationState::Probed { report_hash }`. None exists today. |
| **Proposal value** | A value hard-coded in `aarch64_unknown_linux_gnu_unverified()` or the unverified probe fixture. It is a comparison target for the future probe, never evidence. |
| **Unresolved** | A required model field with no measured value, no derivation rule, or an explicit `unresolved` probe token. Must fail closed for Part B. |
| **Fail closed** | Reject the operation with a typed result/diagnostic; never substitute a host value, a proposal value, a macOS/Darwin value, or a fabricated success. |

Evidence classes used below:

- **FROZEN IDENTITY** — accepted contract text, usable as a symbolic gate.
- **IMPLEMENTED PART A** — code exists and is test-covered for the symbolic M1 subset.
- **IMPLEMENTED MECHANISM** — code exists but has not been applied to a real target (e.g. `attest`).
- **PROPOSAL / UNVERIFIED** — a number or label in `target.rs`/the probe fixture, explicitly not evidence.
- **PART B LEAK** — an implemented rule consumes a target-dependent assumption without a verified model.
- **MISSING / OPEN** — no model field or implementation exists.

---

## 4. Part A Symbolic Record Requirements

Part A must remain constructible, deterministic, and testable without the target probe. It must not carry any unverified width, alignment, bit pattern, format, or ABI location.

### 4.1 Type records

| ID | Requirement | Current status | Notes |
|---|---|---|---|
| A-TY-01 | Symbolic scalar types: `void`, `_Bool`, plain/`signed char`/`unsigned char`, signed/unsigned `short`, `int`, `long`, `long long`, and the `__int128` extension. Rank/signedness only; no width. | partial (`TypeKind` has `Void`, `Bool`, `Char`, `Int`, `Function`; no float/pointer/array/aggregate) | Extend only by contract freeze; do not silently widen M1. |
| A-TY-02 | Symbolic floating types: `float`, `double`, `long double` with a rank/kind but no format/size. | **missing** | `TypeKind` has no float variant. |
| A-TY-03 | Symbolic pointer, array (complete/incomplete/VLA), function, struct/union, enum, qualified, `_Atomic`, complex/vector types. | **missing** beyond `Function` | T06 catalog rows exist; schema is not frozen. |
| A-TY-04 | `IntRank` covers standard integer ranks `Short`, `Int`, `Long`, `LongLong`; `_Bool`/`char` are separate kinds. No numeric width or range. | implemented (`bus.rs:259-270`) | Conversion rules must map `_Bool`/`char` into the C rank order explicitly. |
| A-TY-05 | `CharKind::Plain` remains symbolic; its signedness is not assumed. | implemented (`bus.rs:272-281`) | `Plain` must resolve only at Part B. |
| A-TY-06 | `TypeKind::Int { rank, signed }` identity is stable and canonical. | implemented; `TY13` single producer, lowest-ID reuse scan | Preserve. |
| A-TY-07 | Function type carries result/params/prototype/variadic but **no** ABI register/stack plan. | implemented (`bus.rs:313-322`) | ABI location plans are Part B (T11). |
| A-TY-08 | Type identity must not depend on target size/alignment; equal symbolic types are equal across targets. | implemented for M1 `int`/`int(void)` | Preserve when adding types. |

### 4.2 Literals, constants, and expressions

| ID | Requirement | Current status | Notes |
|---|---|---|---|
| A-CL-01 | `LiteralRecord` carries lexical facts only: radix, suffix, magnitude bytes, sign, spelling, and a **symbolic** candidate type. | implemented (`bus.rs:564-588`) | `Lx08CandidateType::Int` has no width. |
| A-CL-02 | The M1 literal subset is decimal `Integer`, `None` suffix, `Int` candidate, value `2`/`3`. Forms outside are `Unsupported`, never a silent `Int` default. | implemented (`fold.rs:252-265`) | Keep the no-silent-default rule. |
| A-CL-03 | `ConstRecord` carries a mathematical integer magnitude plus sign, not a target bit pattern. | implemented (`bus.rs:590-602`) | Actual record has **no `ty`**; type is supplied by the consuming `Sem`/IR value. |
| A-CL-04 | Constant arithmetic is checked and host-independent; no Rust host overflow/wrap; no target-width wrap. | implemented for magnitude addition (`fold.rs:305-327`) | Only `Add`; no shift/div/cast. |
| A-CL-05 | `max_const_bits` is a configured resource budget, not a target width, `sizeof`, or signed range. | implemented (`limits.rs:70-74`; `fold.rs:273-293`) | `ConstOverflow` under budget is not C signed-overflow semantics. |
| A-CL-06 | `2+3` evaluates to the mathematical `5` in Part A; no target conversion/overflow claim. | implemented and test-covered (`c08_gate1.rs:1249-1295`) | Preserve. |
| A-CL-07 | `SemRecord` carries the symbolic type, value category, and effect mask; M1 allows only `NonLvalue` and `EffectMask(0)`. | implemented (`bus.rs:503-513`; `se_literal.rs:207-225`) | Preserve. |
| A-CL-08 | IR `Constant` references the committed constant and takes its type from the result `ValueRecord.ty`; T09 does not re-fold. | implemented (`vf_interpret.rs:254-326`; `T09` package) | Preserve. |
| A-CL-09 | A constant expression that would require a concrete width/range/format fails closed (`ConstUnsupported`/`ConstOverflow`), not a fabricated target value. | partially implemented; `const.unsupported-unverified-width` is a proposed rule id (`M1_PART_A_CONTRACT_PROPOSAL.md:3831`) | Must be explicit in the frozen rule set. |

### 4.3 Part A prohibited content

Part A records and chips must not contain:

- a concrete `size`, `align`, `bits`, range, or bit pattern for any C type;
- a target float format/precision/exponent, rounding mode, NaN payload, or `fenv` state;
- an ABI register count, stack alignment, HFA/HVA class, or call location;
- a target `wchar_t` size/encoding assumption (the `lx_string`/`lx_char` leaks above are exceptions to be fixed);
- a target execution-character-set assumption (the `lx_escape` UTF-8 assumption is an exception to be fixed);
- any value copied from `aarch64_unknown_linux_gnu_unverified()` or the unverified fixture as if it were measured.

Part A must not fail a *symbolic* identity case merely because the target is unverified. It must fail only when the rule genuinely needs a concrete target property.

---

## 5. Part B Concrete Target Model Requirements

These are **requirements on the future model**, not values. The exact field names and encodings are proposals for the T01/owner co-freeze; they are not frozen here.

### 5.1 Integer model

| ID | Required field/rule | Why it is required | Current model |
|---|---|---|---|
| B-INT-01 | Bits per byte (`CHAR_BIT`/`bits.per.byte`) | `sizeof` is bytes; all bit-level ranges, shifts, and byte order need this | missing |
| B-INT-02 | Per integer scalar kind: `size` (bytes), `align` (bytes), `width` (bits), `value_bits`, `padding_bits`, `min`, `max`, `signed`, `representation`, trap-representation policy | C ranges, promotions, and conversions need value ranges, not only byte size | only `size`/`align` for 13 kinds; others proposal defaults |
| B-INT-03 | Standard integer conversion rank order (including `_Bool` and `char` below `short`) and extended-type rank for `__int128`/`unsigned __int128` | Promotions and usual arithmetic conversions | `IntRank` is symbolic; no target rank/range relation |
| B-INT-04 | Signed integer representation: two's complement / ones' complement / sign-magnitude; sign bit, value bits, padding bits; no trap representation or explicit trap policy | Signed ranges, bit patterns, `-fwrapv`, overflow | missing |
| B-INT-05 | Integer promotions: promote to `int` if `int` represents all values of the source type, otherwise `unsigned int`; explicit handling for `_Bool`, `char`, bit-fields, enums | C11 6.3.1.1; target ranges decide | only `int→int` identity |
| B-INT-06 | Usual arithmetic conversions: `long double` → `double` → `float` → integer promotions; same-signedness higher rank; unsigned rank ≥ signed rank → unsigned; signed can represent all unsigned → signed; otherwise unsigned counterpart | C11 6.3.1.8; target ranges decide | only identity |
| B-INT-07 | Conversion semantics: unsigned conversion modulo `2^N`; signed↔unsigned; signed narrowing implementation-defined/out-of-range; `_Bool` 0/1; pointer/integer; float↔integer (undefined if out of range); float↔float | C11 6.3; target widths decide | identity only; no plans |
| B-INT-08 | Signed overflow: ISO C undefined behavior; unsigned arithmetic modulo `2^N`; `-fwrapv` overlay defines two's-complement wrap; `-fno-strict-overflow` is an optimization assumption; division by zero and `INT_MIN / -1` undefined; shift rules (negative/too-wide undefined; unsigned wrap; negative right shift implementation-defined) | C11 6.5; GCC options | checked magnitude math only; no target-width rule |
| B-INT-09 | `_Bool`: size, alignment, representation 0/1, conversion from scalar/pointer/FP, `_Bool` bit-field, `sizeof`, ABI class | C11 6.3.1.2; target-dependent | symbolic `TypeKind::Bool`; proposal size/align only |
| B-INT-10 | Plain `char`: signedness, `CHAR_MIN`/`CHAR_MAX`, execution character set, source/execution charset, multibyte `MB_LEN_MAX`, escape/character-constant semantics | `char` type and character constants are target-dependent | `char.signed` measured but not consumed; `lx_char` assumes 8/16/32-bit units |
| B-INT-11 | `wchar_t`: size, alignment, signedness, encoding (`utf32`/`utf16`), range, ABI class; `L`/`u`/`U` literal widths and values | wide characters/strings | size/align/signed/encoding are probed; range/type/ABI missing; `lx_string` consumes unverified values |
| B-INT-12 | `size_t`, `ptrdiff_t`, `intptr_t`, `uintptr_t`, `intmax_t`, `uintmax_t`: width/signedness; `sizeof`/`_Alignof` result type; array/VLA index and difference type | pointer arithmetic, VLA, `sizeof` | `size_t`/`ptrdiff_t` size/align only; others missing |
| B-INT-13 | Pointer model: object/function pointer size/alignment, representation (flat/segmented/capability), null representation, pointer↔integer width, `void*` interchange, provenance/address spaces, function/data pointer compatibility | C11 6.3.2.3/6.5.9; target ABI | size/align only; no representation |
| B-INT-14 | `__int128`/`unsigned __int128`: size/align (measured), width/range/representation, ABI class, helper library, conversions/arithmetic | T11 CG01/CG12/CG13/CG21; T08 CL08 | size/align only; `TypeKind` has no `__int128` |
| B-INT-15 | Enum: underlying type selection/range, rank, promotion, `-fshort-enums` overlay, duplicate/overflow rules | T06 TY19; T07 SE23 | missing |
| B-INT-16 | Bit-field layout: allocation unit, bit order, signedness, width, zero-width, packing/`#pragma pack`/`-fpack-struct`, cross-unit access | T08 CL12; T09 IR17; T11 CG18 | missing |
| B-INT-17 | Struct/union layout: alignment, member offsets, padding, tail padding, union size, `max_align_t`, `_Alignas`/`__attribute__((aligned))`; reject stale layout | T08 CL10/CL11/CL26; T11 CG32/CG36 | missing |
| B-INT-18 | `_Atomic`: representation/alignment/lock-free property, atomic object size/ABI, interaction with ordinary loads/stores | T06 TY30; T07 SE28; T09 IR27; T11 CG23 | missing |
| B-INT-19 | `long double`: format, size/align, precision, exponent range, ABI class, operations, conversions, NaN/zero/subnormal behavior, helper symbols | T04 LX10; T08 CL05/CL22; T11 CG19/CG21 | size/align/format labels only; no type/operations |
| B-INT-20 | VLA/dynamic stack: bound/index type (`size_t`), element size/stride (`size_t`), stack alignment, `alloca`/dynamic stack, cleanup/restore, stack-probe/resource limits | T08 CL15/CL25; T09 IR10/IR26; T11 CG32 | missing |
| B-INT-21 | Execution character set and wide/UTF encodings: source charset, execution charset, `char16_t`/`char32_t`, `wchar_t`, string terminator, multichar constants | T04 LX11–LX13; T08 CL19 | `lx_escape` assumes UTF-8; `lx_char`/`lx_string` assume 32-bit wide units |

### 5.2 Floating-point model

| ID | Required field/rule | Why it is required | Current model |
|---|---|---|---|
| B-FP-01 | `float`, `double`, `long double` storage size/alignment and format (precision, exponent range, encoding); do not assume binary32/binary64 | T04 LX10; T08 CL05/CL22; T11 CG01/CG10/CG19 | `float`/`double` sizes measured; formats are proposal labels |
| B-FP-02 | Default rounding mode (strictly: default environment), `FLT_ROUNDS`, `FLT_EVAL_METHOD`, excess precision, contraction/FMA policy | C11 5.2.4.2.2 / 7.6; GCC options | missing |
| B-FP-03 | `fenv` access: `#pragma STDC FENV_ACCESS`, exception flags, traps, `fesetround`/`fegetround`, `fegetenv`/`fesetenv`, default environment, compile-time assumptions | C11 7.6; FP semantics | missing |
| B-FP-04 | NaN model: quiet/signaling, payload propagation/canonicalization, comparisons (ordered/unordered), conversion to integer, `_Bool` conversion, `isnan`/`isinf`/`signbit` | T11 CG20; T07 SE10; T13 | missing |
| B-FP-05 | Signed zero, infinities, subnormals, overflow/underflow, exception flags, flushing behavior | T11 CG19; T10 OP03 | missing |
| B-FP-06 | Floating literal conversion: decimal/hex spelling correctly rounded to target format; range/overflow/underflow; suffix (`f`/`F`, `l`/`L`, none); `long double` deferred or implemented | T04 LX10; T08 CL05/CL16 | binary32/64 core exists but assumes formats; `l/L` `Unsupported` |
| B-FP-07 | Float↔integer conversions: truncation toward zero, out-of-range undefined, NaN/Infinity undefined, `_Bool` from FP; comparisons and predicates | T06 TY29; T07 SE07/SE10; T09 IR07/IR11; T11 CG21 | missing |
| B-FP-08 | Floating ABI: GP/FP argument registers, HFA/HVA, aggregate return, variadic register save area, `long double` class | T11 CG01–CG05; A-ABI-03..07 | only four coarse `AbiSpec` fields |

### 5.3 Consistency and plausibility requirements

Attestation must reject a report that is hash-consistent but structurally impossible. These are **relation checks**, not invented values:

| ID | Check |
|---|---|
| B-CON-01 | `sizeof(char) == 1` byte; `CHAR_BIT` present and valid; all byte→bit conversions use it. |
| B-CON-02 | Every complete object scalar has `size > 0`, `align > 0`, `align` a power of two, and `size % align == 0` (array/object layout requirement). |
| B-CON-03 | Integer size ordering is consistent with the declared ranks and the frozen data model; signed/unsigned pairs have the same size/alignment. |
| B-CON-04 | `sizeof(short) <= sizeof(int) <= sizeof(long) <= sizeof(long long)`; float ordering `sizeof(float) <= sizeof(double) <= sizeof(long double)` (the exact equality cases are target/ABI facts, not assumed). |
| B-CON-05 | `data_model` label is cross-checked against measured `int`/`long`/pointer widths; a hash-signed inconsistent report is rejected. |
| B-CON-06 | `wchar_t.encoding` is consistent with its size/signedness/range; unknown encodings are rejected; `utf16` must have measured UTF-16 evidence if accepted. |
| B-CON-07 | `longdouble.format` is consistent with its storage size/alignment; unknown formats are rejected; no `x87`/Darwin value on this identity unless the ABI/evidence explicitly supports it. |
| B-CON-08 | `float`/`double` format/size consistency; a binary32 label requires at least 32 bits of storage, binary64 at least 64, etc. |
| B-CON-09 | `_Bool` size/alignment and representation 0/1; `_Bool` range is `{0,1}` even if storage is wider. |
| B-CON-10 | ABI counts are nonzero and within the model's representable range; `stack_align` is a power of two and at least the alignment required by the largest scalar (exact threshold is an ABI-freeze decision). |
| B-CON-11 | Integer ranges are consistent with signedness/value bits/representation (`min <= 0 <= max`; unsigned `min == 0`; `max == 2^N-1` for a pure binary representation; signed range follows the recorded representation). |
| B-CON-12 | All model fields are present and no field is `unresolved`; unknown extra fields do not override known fields or claim an unsupported model version. |

### 5.4 Part B fail-closed rules

Part B code must:

1. Call `TargetSpec::require_verified` (or an equivalent model-version check) before consuming concrete values.
2. Reject a missing/unresolved/inconsistent field with a typed diagnostic; never fall back to `aarch64_unknown_linux_gnu_unverified()`.
3. Never use Rust host `size_of`, `align_of`, `i32`/`u64`/`f64` wrapping, `char` signedness, or host float behavior as target semantics.
4. Never use macOS/Darwin arm64 values, Mach-O directives, or the Darwin variadic model.
5. Never treat `ProbeStatus::Verified` as sufficient; only `TargetSpec::attest` + model version/identity checks open codegen.
6. Never silently truncate/convert a target value because a field is missing; emit `Unsupported`/`ConstUnsupported`/`TargetUnverified`.
7. Never claim that a Part A symbolic result is a target bit pattern or a platform `sizeof`.
8. Keep the codegen gate closed until the attested model is bound to the current contract/model version.

---

## 6. Part A vs Part B Matrix

| Model element | Part A representation | Part B concrete requirement | Current state | Gate |
|---|---|---|---|---|
| Target identity | symbolic `TargetProfile` | exact match in attested report | **FROZEN** | none for Part A; identity check in attest |
| Scalar size/alignment | not stored in Part A records | all 21 `ScalarKind` sizes/alignments | 13 measured fields; 8 left at proposal defaults | probe + model schema |
| Integer width/value bits/padding | rank/signedness only | bits/value bits/padding per integer kind | missing | probe + model schema |
| Integer representation | not assumed | two's complement/ones'/sign-magnitude, trap policy | missing | probe + freeze |
| Integer ranges | not stored | min/max per integer kind | missing | probe/derivation + freeze |
| Integer rank/conversion rank | `IntRank` order + `Bool`/`Char` kinds | standard + extended rank, bit-field/enum rank | partial symbolic | Part A freeze; Part B ranges |
| `_Bool` | `TypeKind::Bool` symbolic | size/align/repr/conversion/ABI | symbolic only; proposal size/align | probe |
| Plain `char` | `CharKind::Plain` unresolved | signedness/range/encoding/ABI | `char.signed` measured; not consumed | probe + code fix |
| `wchar_t` | no `TypeKind`; `ScalarKind::WCharT` proposal | size/align/signed/encoding/range/type/ABI | size/align/signed/encoding measured; range/type/ABI missing | probe + model schema |
| `float`/`double` | no `TypeKind` | format/precision/range/rounding/ABI | size/align measured; format proposal | probe + type freeze |
| `long double` | no `TypeKind` | format/size/align/precision/ABI/ops/helpers | size/align/format measured; type/ops missing | probe + model schema |
| Pointer | no `TypeKind` | size/align/repr/null/int conversion/ABI | size/align measured; repr missing | probe + model schema |
| `size_t`/`ptrdiff_t` | no `TypeKind` | width/signedness/relation to pointer/VLA | size/align measured; signedness proposal | probe + model schema |
| `__int128` | no `TypeKind` | size/align/width/repr/ABI/helpers | size/align measured | probe + type freeze |
| Enum | no `TypeKind` | underlying type/range/rank/promotion | missing | Part B freeze |
| Bit-field | no layout | allocation/order/packing/signedness | missing | Part B freeze |
| Struct/union layout | no layout | offsets/padding/alignment/tail | missing | Part B freeze |
| `_Atomic` | no type | size/align/lock-free/ABI | missing | Part B freeze |
| Integer promotions | identity only | range-based promotion rules | identity only | target ranges |
| Usual arithmetic conversions | identity only | rank/signedness/float hierarchy | identity only | target ranges/formats |
| Assignment/argument/cast conversions | identity only | target conversion rules | identity only | target model |
| Integer overflow | checked mathematical magnitude; no wrap | ISO UB/unsigned modulo/`-fwrapv` overlay | checked magnitude only | target width/options |
| Shift/division rules | not implemented | C11 shift/div rules | missing | target width |
| Integer constant expressions | magnitude/sign; `max_const_bits` budget | candidate type/range/overflow diagnostics | M1 `Add` only | target widths |
| Float literal conversion | syntax + binary32/64 core | target format/rounding/range | `Ack`-only; formats assumed | target formats |
| Float rounding/fenv/NaN/zero | none | full model | missing | probe + freeze |
| VLA | plan only | `size_t`/stack/cleanup/alloca | missing | target model |
| ABI arguments/returns | symbolic signature | classes/locations/registers/stack | four coarse fields | A-ABI-03..07 |
| Execution charset | syntax only | source/execution/wide encodings | UTF-8 assumption in `lx_escape` | target model |
| Target options | `OptionFlag` subset | `-fwrapv`/`-ffast-math`/`-fshort-enums`/`-fpack-struct` overlays | partial | option freeze |

---

## 7. Promotion, Conversion, Constant, and Floating Rules

### 7.1 Integer promotion (Part B once ranges exist)

C11 6.3.1.1: if `int` can represent all values of the original type, the value is converted to `int`; otherwise to `unsigned int`. This applies to `_Bool`, `char`, `signed char`, `unsigned char`, `short`, `unsigned short`, bit-fields, and enums (with the standard's nuances). The rule is **range-based**, not size-based: two types with the same byte size may differ in range because of padding/representation. Therefore `B-INT-02` ranges are mandatory before `TY25` can be implemented beyond the `int→int` identity.

Part A must keep `int→int` identity and must not assume that `unsigned short` is or is not covered by `int`. The current `TyConvChip` correctly rejects non-`Int` operands as `Unsupported` (`ty_conv.rs:173-179`).

### 7.2 Usual arithmetic conversions (Part B)

C11 6.3.1.8:

1. If either operand is `long double`, convert the other to `long double`.
2. Else if either is `double`, convert to `double`.
3. Else if either is `float`, convert to `float`.
4. Else perform integer promotions on both operands.
5. If both have the same signedness, convert the lower rank to the higher rank.
6. If the unsigned type has rank greater than or equal to the signed type, convert the signed operand to the unsigned type.
7. If the signed type can represent all values of the unsigned type, convert the unsigned operand to the signed type.
8. Otherwise, convert both to the unsigned type corresponding to the signed type.

Every step after (3) depends on target ranks/ranges. Part A must not implement this with Rust integer widths or host `usize`.

### 7.3 Conversion rank and signedness

The standard order for the standard integer types is `_Bool` < `char` < `short` < `int` < `long` < `long long`, with each signed type and its unsigned counterpart having the same rank. `char`, `signed char`, and `unsigned char` have the same rank; `_Bool` has the lowest rank. Extended integer types (including `__int128`) have implementation-defined rank relative to the standard types; the model must state that rank explicitly if the extension is supported.

The current `IntRank` enum covers only `Short`/`Int`/`Long`/`LongLong`; `_Bool`/`Char` are separate `TypeKind` variants, and `__int128` is not a `TypeKind`. This is adequate for a symbolic M1 subset only. It is not a complete conversion-rank model.

### 7.4 Assignment, argument, and cast conversions

| Conversion context | Required Part B rule | Part A status |
|---|---|---|
| Assignment | Arithmetic conversion to the destination type; `_Bool` truth conversion; compatible aggregate; pointer qualification/null; no implicit aggregate conversion | identity only |
| Argument (prototype) | Convert each argument to the parameter type; `_Bool`/integer/float/pointer rules | not implemented |
| Argument (variadic/old-style) | Default argument promotions: `float`→`double`, integer promotions; ABI save-area rules | not implemented |
| Explicit cast | Scalar/void/pointer-integer domain; target width/format; invalid aggregate casts rejected | not implemented |
| Return | Convert to the function result type; aggregate return ABI | identity `int` only |
| `_Bool` | 0 if the scalar compares equal to 0; 1 otherwise; pointer null → 0; NaN → 1 | not implemented |
| Pointer/integer | Implementation-defined mapping; must fit the target pointer width; no host pointer | not implemented |
| Float/integer | Truncate toward zero; out-of-range undefined; NaN/Infinity undefined; `_Bool` truth | not implemented |

### 7.5 Integer overflow and undefined behavior

| Case | ISO C rule | Part B model requirement | Current implementation |
|---|---|---|---|
| Signed `+`, `-`, `*`, unary `-` overflow | Undefined behavior | Base model: UB; `-fwrapv` overlay: two's-complement wrap; `-fno-strict-overflow` is an optimization assumption, not a value rule | checked magnitude only; no target width |
| Unsigned arithmetic | Modulo `2^N` | N from target value bits; no host wrap | checked magnitude; no target width |
| Signed division `INT_MIN / -1` | Undefined behavior | Diagnose/UB policy; do not compute with host `i32::MIN / -1` | not implemented |
| Division/remainder by zero | Undefined behavior | Diagnose/UB; no fabricated result | not implemented |
| Shift count negative or ≥ width | Undefined behavior | Target width check | not implemented |
| Signed left shift overflow | Undefined behavior | Target width/representation | not implemented |
| Right shift of negative value | Implementation-defined (GCC: arithmetic) | Record target policy; do not assume logical/arithmetic | not implemented |
| Narrowing signed conversion out of range | Implementation-defined or signal (C11 6.3.1.3p3) | Record GCC/target behavior; do not assume wrap | not implemented |
| Unsigned conversion | Modulo `2^N` | Target width | not implemented |
| Constant expression overflow | C11 6.6p4: must be in range of its type; constraint violation | Typed diagnostic; `-fwrapv` does not make a constant expression overflow legal unless the model says so | `max_const_bits` only; not target range |

**Do not** equate `max_const_bits` with a target integer width. It is a configured safety bound (`limits.rs:70-74`; `fold.rs:273-293`). `ConstOverflow` under that budget is not the C signed-overflow rule. A Part B implementation must compute with explicit target widths/ranges and a separate overflow policy.

### 7.6 Constant expression rules

Part A:

- Integer values are mathematical magnitude+sign; arithmetic is checked against `max_const_bits`, not a target width.
- The lexical `Lx08CandidateType` is symbolic (`Int` only in M1); it is **not** a target type or bitvector.
- `RequiredKind::IntegerConstantExpression` is a per-use requirement, distinct from the candidate type.
- `ConstRecord` has no target type field; type comes from `SemRecord`/IR `ValueRecord.ty`.
- Only `Add` is exercised in M1; `2+3=5` is symbolic.
- A rule that would need an unverified width fails closed (`ConstUnsupported`/`ConstOverflow`).

Part B:

- Integer literal candidate lists differ for decimal vs octal/hex; suffixes select `int`/`unsigned int`/`long`/`unsigned long`/`long long`/`unsigned long long` depending on target ranges.
- Enumeration constants use the enum's underlying type; bit-field widths use the bit-field model.
- `sizeof`/`_Alignof` produce `size_t`; pointer differences produce `ptrdiff_t`.
- Signed constant overflow is a constraint violation/diagnostic or a defined wrap only under an explicit overlay; unsigned constant arithmetic is modulo `2^N`.
- Floating constants are correctly rounded to the target format; `long double` must be resolved or explicitly unsupported.
- Address constants and symbol+addend remain Part B/link-time concerns; they are never host pointers (`T08` package line 5).
- Target-width constant folding must not retroactively change Part A results; Part A and Part B results are separate evidence classes.

### 7.7 Floating-point rules

| Topic | Requirement | Current state |
|---|---|---|
| Formats | `float`, `double`, `long double` storage format, precision, exponent range, encoding; no assumption from the Rust host | `FloatFormat` labels; `float`/`double` format not attested |
| Default rounding | Default environment; record `FLT_ROUNDS`/rounding mode; RNE is a proposal until probed | LX10 core implements RNE; not target-attested |
| `fenv` | Access pragma, exception flags/traps, dynamic rounding, default environment; no assumption that `fenv` is inaccessible | missing |
| Excess precision | `FLT_EVAL_METHOD`; evaluation format vs storage; no double-rounding assumption | missing |
| Contraction/FMA | Whether expressions may be contracted; default policy; `-ffp-contract` | missing |
| NaN | Quiet/signaling, payload propagation, canonicalization, ordered/unordered comparisons, conversion to int, `_Bool` truth | missing |
| Signed zero | `+0.0`/`-0.0`, equality, `x + 0.0`, `x - 0.0`, `x * 0.0`, division by zero | missing |
| Subnormal | Gradual underflow, tininess, exception flags, flush policy | LX10 models subnormals for binary32/64 arithmetic core; no target policy |
| Overflow/underflow | Infinity/zero + flags; `float`/`double` conversion; target rounding | LX10 returns infinity/zero + flags; no target claim |
| Literal conversion | Correctly rounded decimal/hex to target format; range; suffix | binary32/64 core; `l/L` `Unsupported` |
| Float↔integer | Truncate toward zero; out-of-range/NaN/Infinity undefined; `_Bool` truth | missing |
| `long double` | Format/ABI/operations/helpers; may be binary128, binary64, or another ABI-supported format; do not assume | size/align/format labels; no operations |
| `-ffast-math` | Explicit overlay: NaN/zero/subnormal/reassociation assumptions; never silently applied | `OptionFlag::FfastMath` exists; no model semantics |

### 7.8 VLA and pointer dependencies

- VLA bound/index arithmetic uses `size_t`/`ptrdiff_t`; their widths/signedness must be attested (`B-INT-12`).
- VLA element stride and `sizeof(VLA)` use `size_t` and target element layout (`T08 CL09/CL15`).
- VLA storage uses the target stack alignment, dynamic-stack mechanism, and cleanup/restore rules (`T09 IR26`, `T11 CG32`).
- Pointer arithmetic uses element size and `ptrdiff_t`; pointer difference must fit `ptrdiff_t` (`T09 IR10`).
- `long double` and `__int128` argument/return behavior depends on the ABI classification (HFA/HVA, GP/FP registers, indirect return); see A-ABI-03..07 and T11 CG01–CG05.
- No VLA is part of M1 Part A; VLA must not be passed off as a static constant (`T08` package line 5).

---

## 8. Attestation and Versioning Plan

### 8.1 Current chain (implemented mechanism, never applied)

```text
H01 probe (harness present, never run)
  -> normalized report + report.sha256 + evidence hashes
H07 classification (not implemented)
  -> resolved ABI fields + annex hashes
T01 integrator
  -> canonical attestation report (38 fields + verified=true + report_hash)
  -> ProbeReport::parse / status / TargetSpec::attest
  -> VerificationState::Probed { report_hash }
  -> CompilerConfig -> ensure_codegen_ready
  -> T11 target code
```

`ProbeReport::parse` stores fields in a sorted `BTreeMap` and rejects malformed/duplicate lines (`target.rs:821-846`). `unsigned_body` emits sorted `key=value\n` excluding `report_hash` (`target.rs:858-872`). `verify_verified` requires `verified=true` and a matching SHA-256 (`target.rs:874-892`). `attest` validates fields, identity, and parses values (`target.rs:658-716`). The private `VerificationState` representation prevents external construction of `Probed` (`target.rs:361-423`), but it does not prevent a caller from supplying a fabricated report.

The H01/H07 handoff is documented in A-ABI-01 (`docs/reviews/dispatch/A09_MINIMAL_TARGET_CHAIN_PLAN.md` §2.3) and T00 §4.1. This report does not duplicate the probe/link chain; it adds the **model-completeness** requirements.

### 8.2 Model-specific attestation gaps

| Gap | Evidence | Consequence |
|---|---|---|
| 38 fields are insufficient for a C integer/float model | `target.rs:493-532`; `tools/torture/probe/README.md:205-221` | no `CHAR_BIT`, ranges, representation, `_Bool`, float formats, rounding/fenv, enum/bitfield/VLA, pointer representation |
| 8 scalar kinds are never measured | `MEASURED_SCALARS` at `target.rs:327-341`; `attest` at `:675-681` | `Bool`, `SChar`, `UChar`, `UShort`, `UInt`, `ULong`, `ULongLong`, `UInt128` remain proposal defaults after attestation |
| `float`/`double` formats are not attested | `attest` only sets `LongDouble.float` (`:695-705`) | an attested target still carries proposal binary32/binary64 labels; `LX10` assumes them |
| No integer ranges/representation | `ScalarModel` (`target.rs:133-166`) has only size/align/signed | promotions/conversions/overflow cannot be implemented |
| No `CHAR_BIT` | not in `REQUIRED_PROBE_FIELDS` | byte sizes cannot be converted to bit widths |
| No `_Bool`/plain-char semantics | `Bool` size/align proposal; `Char.signed` measured but no conversion rule | `_Bool` and `char` arithmetic remain unsafe |
| No float rounding/NaN/zero/fenv | not in the model | FP codegen cannot claim IEEE behavior |
| No enum/bitfield/aggregate/atomic/VLA model | no fields | T08/T09/T11 target layout/runtime cannot be implemented |
| No pointer representation | only size/align | pointer/integer conversion, null, provenance unclear |
| No model schema version/probe schema version | `ProbeReport` has only free-form keys | a report can be accepted without stating which model it satisfies |
| No consistency/plausibility checks | `compiler/README.md:217-230`; `target.rs:658-716` | zero/oversized ABI counts, `align > size`, inconsistent data model can pass if hash-signed |
| `status` is weaker than `attest` | `compiler/README.md:86-90` | code could wrongly gate on `status` |
| Contract artifact cannot represent attested values | `FrozenSchema::current()` hardcodes unverified (`contract.rs:502-512`); `contract_version_file()` hardcodes unverified (`:785-800`) | no frozen-artifact path for a verified target |
| No cross-check of identity vs measured values | `attest` only compares identity strings (`:669-673`) | LP64/ELF/little labels are not validated against measurements |

### 8.3 Proposed versioned target-model artifact

Do not freeze this section as-is; it is a requirements proposal for the T01 integrator/owners. The model should be a **separate versioned schema**, not an ever-growing flat key list.

```text
TargetModelSchema {
    schema_version: TargetModelVersion      // proposed; exact spelling not frozen
    identity: TargetProfile                 // frozen identity
    probe_schema_version: ProbeSchemaVersion
    scalar: Vec<ScalarModel>                // one per scalar kind, complete field set
    integer: IntegerModel
    floating: FloatingModel
    pointer: PointerModel
    layout: LayoutModel
    vla: VlaModel
    abi: AbiSpec                            // extended by T11 work
    option_overlays: OptionOverlayModel
    provenance: ProvenanceRecord
}
```

Required model dimensions:

| Dimension | Required content |
|---|---|
| `SchemaVersion` | field inventory, enum vocabulary, and semantic revision; recognized values only |
| `ProbeSchemaVersion` | H01 report field set/format version; recognized values only |
| `ScalarModel` | kind, size, align, width bits, value bits, padding bits, signedness, integer representation, float format, min/max (where applicable), trap policy |
| `IntegerModel` | `CHAR_BIT`, rank order, signed/unsigned pairs, promotion rule, usual-arithmetic rule, conversion rule, overflow policy, `-fwrapv` overlay, shift/division policy, enum policy, bit-field policy |
| `FloatingModel` | per-format precision/exponent range/storage, default rounding, `FLT_ROUNDS`, `FLT_EVAL_METHOD`, `fenv` access, NaN/zero/subnormal policy, contraction policy, fast-math overlay |
| `PointerModel` | object/function size/alignment, representation, null, integer conversion width, `void*` interchange, provenance/address-space policy |
| `LayoutModel` | struct/union rules, bit-field rules, `max_align_t`, `_Alignas`/packing, `_Atomic`, tail padding |
| `VlaModel` | size/index type, element stride type, stack alignment, dynamic-stack/alloca mechanism, cleanup, resource limits |
| `AbiSpec` | current four fields plus T11-required classes (HFA/HVA, aggregate passing/return, sret, variadic model, register/stack conventions, x18, TLS, relocations, target features) |
| `OptionOverlayModel` | `-fwrapv`, `-fno-strict-overflow`, `-ffast-math`, `-fshort-enums`, `-fpack-struct`, `-fcommon`/`-fno-common`, `-fPIC`/`-fPIE`, dialect-specific options; each overlay must state which model field it changes |
| `ProvenanceRecord` | H01 report file hash, canonical body hash, H07 annex hash, resolved-field hash, toolchain pins, substrate run id, model schema hash |

### 8.4 Proposed required probe fields (names only; no values)

The following are **proposed field names**, not frozen keys and not values. They extend the current 38-field report only after the model schema freeze.

| Group | Proposed fields |
|---|---|
| Byte/bit base | `char.bit` (or `bits.per.byte`), `char.min`, `char.max` |
| Missing scalar sizes/aligns | `sizeof.bool`, `alignof.bool`, `sizeof.schar`, `alignof.schar`, `sizeof.uchar`, `alignof.uchar`, `sizeof.ushort`, `alignof.ushort`, `sizeof.uint`, `alignof.uint`, `sizeof.ulong`, `alignof.ulong`, `sizeof.ulonglong`, `alignof.ulonglong`, `sizeof.unsigned __int128`, `alignof.unsigned __int128` |
| Integer ranges | `min.<kind>` / `max.<kind>` (decimal or magnitude text) for each integer kind, or an equivalent `width` + `repr` + `padding` encoding |
| Integer representation | `int.repr`, `int.padding`, `int.trap`, `bool.repr`, `wchar_t.repr` |
| Float formats | `format.float`, `format.double` (in addition to `longdouble.format`), `float.precision`, `float.emax`, `float.emin`, `double.precision`, `longdouble.precision` |
| FP environment | `float.rounding`, `float.eval_method`, `float.fenv_access`, `float.nan_policy`, `float.signed_zero`, `float.subnormal`, `float.contraction` |
| Pointer | `pointer.repr`, `pointer.null`, `pointer.int_width`, `pointer.function_pointer`, `pointer.address_space` |
| `size_t`/`ptrdiff_t` | `size_t.signed`, `ptrdiff_t.signed` (or range fields) |
| Enum/bitfield/layout | `enum.underlying`, `bitfield.order`, `bitfield.unit`, `bitfield.pack`, `struct.align`, `struct.tail_padding`, `atomic.size`, `atomic.align`, `atomic.lock_free` |
| VLA | `vla.size_type`, `vla.ptr_diff_type`, `vla.stack_align`, `vla.dynamic_stack` |
| Model/probe version | `model.schema_version`, `probe.schema_version`, `model.schema_hash`, `probe.field_inventory_hash` |
| Provenance | H01/H07 hashes, toolchain pins, substrate run id (as proposed in A-ABI-01; extra keys are retained/hashed by `ProbeReport`) |

Field design rules:

- Every field must be measured or derived by a documented, deterministic rule; never copied from `target.rs` or the fixture.
- Ranges must not be forced into `u8`; use decimal text or a magnitude carrier. `ScalarModel.size: u8` is sufficient for byte sizes but not for bit widths up to 128; the schema must widen or use a separate bit-width type.
- All fields in a model version are required; partial reports are rejected.
- Unknown fields are retained for hashing but do not satisfy a required model field.
- A model-version mismatch is a hard failure, not a warning.

### 8.5 Attestation validation matrix (required behavior)

| Condition | Required outcome |
|---|---|
| Missing required model field | `ProbeError::MissingField` / equivalent; fail |
| Field value `unresolved` or non-numeric | `ProbeError::BadValue`; fail |
| Duplicate key | `ProbeError::DuplicateField`; fail |
| Malformed line | `ProbeError::MalformedLine`; fail |
| `verified != true` | `ProbeError::NotVerified`; fail |
| Missing/malformed `report_hash` | `ProbeError::BadHash`; fail |
| Hash mismatch | `ProbeError::HashMismatch`; fail |
| Identity mismatch | `ProbeError::IdentityMismatch`; fail |
| Unknown `wchar_t.encoding`/`longdouble.format`/float format | `BadValue` or a dedicated unsupported-value error; fail |
| Unknown model/probe schema version | New typed error (e.g. `UnsupportedModelVersion`); fail |
| Consistency violation (`B-CON-01..12`) | New typed error (e.g. `InconsistentModel`); fail |
| `ProbeStatus::Verified` only | **not sufficient** for codegen; require `attest` |
| Attested target model version differs from the compiled model | fail; require re-probe/re-attest or explicit migration |
| Report contains Darwin/Mach-O/other identity values | fail; no fallback |
| Extra unknown fields | retain/hash; do not override required fields |
| `report_hash` covers body but not provenance | provenance only bound if included as report fields; document the trust boundary |

`TargetSpec::attest` should return/record the model version and schema hash in addition to `report_hash`, and `require_verified` should check both. The private `VerificationState` pattern can be preserved; add a private `TargetModelVersion`/attestation token so callers cannot forge a verified model.

### 8.6 Versioning model

Use four distinct identities; do not conflate them:

| Version/hash | Scope | When it changes | Relationship |
|---|---|---|---|
| **Contract version** (`CONTRACT_VERSION`, currently `t01-c01-c06/39`) | T01 normative shapes/rules, target profile, limits, policy | normative shape/rule changes; explicit integrator bump | Contract hash covers declared shapes/rule ids, not source semantics |
| **Target model schema version** (proposed) | Target model fields/enums/semantics (integer/float/pointer/layout/VLA/ABI) | model field/semantic change | Must be recognized by `attest`; old attestations invalid |
| **Probe schema version** (proposed) | H01 report field inventory/format | probe field/format change | Must match model schema expectation |
| **Attestation record** (`report_hash`, provenance) | One probe run's values + evidence | every probe/re-attestation | Binds values to model schema/identity; not a schema version |

Rules:

1. **Identity is not re-versioned per probe.** `aarch64-unknown-linux-gnu`/ELF/LP64/little/AAPCS64 stays frozen.
2. **Schema changes bump the target-model version.** Adding/removing a field, changing a field's meaning, or changing an enum vocabulary requires a new model version and a contract update.
3. **Value changes do not silently pass.** A new probe with the same schema produces a new `report_hash`; consumers must reject a stale attestation. `CompilerConfig`/snapshot must carry the model version and report hash.
4. **Contract hash treatment must be explicit.** Today `FrozenSchema::encode` includes the target scalar values and verification state (`contract.rs:588-629`), so an attested target would change the contract hash if `current()` accepted it. `FrozenSchema::current()` instead hardcodes the unverified target (`:502-512`). The integrator must choose one:
   - **(a) Split** the contract schema hash from a target-attestation hash; the frozen contract covers shapes/versions, the attestation covers values/provenance; or
   - **(b) Freeze** a specific attested target as a new contract version with values in the hash and a new `target_verification=probed` artifact.
   Do not compute a runtime contract hash from local probe data and call it the frozen contract.
5. **Snapshot/config binding:** `snapshot.rs:51-85` encodes target values and verification state; add model version, probe schema version, and (if distinct) target schema hash. Two configs with the same values but different model versions must not compare equal for replay.
6. **Codegen gate:** `ensure_codegen_ready` must require an attested model matching the compiled model version. A verified report for an older schema is not valid for new Part B rules.
7. **Migration:** if a model field is added, old reports are incomplete and rejected; there is no "default" for a missing model field. If a field is removed, old reports may be rejected or migrated only by an explicit compatibility rule in the model version.
8. **Provenance:** the report hash binds report data; the contract hash binds schema; the H07 annex hash binds classification; toolchain pins bind the probe environment. None of these alone proves authenticity (TOCTOU), and none may be substituted for another.

### 8.7 Codegen unlock sequence

```text
1. Freeze target-model schema/version + field inventory (T01/owners)
2. Extend H01 probe fields + normalizer + report schema (H01)
3. Run H01 on a pinned, provisioned Linux AArch64 substrate; produce report/evidence
4. H07 classifies ABI/model fields from evidence; requires abi.*.ok=1 and rejects unresolved
5. T01 integrator builds the canonical attestation report, validates model version/consistency, calls attest
6. Freeze the target-model/attestation artifact and update contract/version metadata
7. T11 Part B work opens for that attested model only
```

Part A is independent of steps 1–7. No target code may be generated or claimed before step 7.

### 8.8 What must not be claimed

- No probe has run; no H07 classification exists; no attestation exists.
- No concrete size/alignment/format/ABI value is verified by this report.
- `aarch64_unknown_linux_gnu_unverified()` and `compiler/contracts/target/aarch64-linux-probe.txt` are proposals, not evidence.
- `c02_target` passing proves the *mechanism* against synthetic reports, not the target.
- `TargetSpec::attest` validates data/hash, not physical provenance.
- The current 38-field report is not a complete C integer/float model.
- No macOS/Darwin arm64 value may be used as a substitute.

---

## 9. Affected Chips, Records, and Tests

### 9.1 Records and foundation

| Path | Role | A-ABI-02 impact |
|---|---|---|
| `compiler/src/target.rs` | `TargetProfile`, `ScalarModel`, `ScalarKind`, `FloatFormat`, `AbiSpec`, `VerificationState`, `TargetSpec`, `ProbeReport`, `ProbeError`, `CompilerConfig` | primary schema/attestation surface; model extension and consistency checks |
| `compiler/src/bus.rs` | `TypeKind`, `IntRank`, `CharKind`, `LiteralRecord`, `ConstRecord`, `SemRecord`, `IrOp` | Part A symbolic records; Part B typed constants may need a new record/field |
| `compiler/src/contract.rs` | `FrozenSchema`, `CONTRACT_VERSION`, `CONTRACT_HASH`, normative rules | model-version/hash split; freeze updates |
| `compiler/contracts/CONTRACT_VERSION` | frozen artifact marker | add model/probe version/provenance only by integrator freeze |
| `compiler/contracts/target/aarch64-linux-probe.txt` | unverified probe fixture | extend field inventory after schema freeze; never treat as evidence |

### 9.2 Chips

| Group | Chips/files | A-ABI-02 dependency |
|---|---|---|
| T04 Lex | `compiler/src/chips/lex/lx_decode.rs` (LX08 integer candidate), `lx_float_syntax.rs`/`lx_float_value.rs` (LX09/LX10), `lx_char.rs` (LX12), `lx_string.rs` (LX13), `lx_escape.rs` (LX11) | target candidate types, float formats/rounding, `wchar_t`/execution charset, char signedness |
| T06 Types | `compiler/src/chips/types/ty_types.rs` (TY13/TY17), `ty_conv.rs` (TY25/TY26/TY27) | rank/signedness/ranges; promotions/conversions; `_Bool`/char/pointer/float |
| T06 catalog (unimplemented) | TY15 pointer, TY16 array/VLA, TY19 enum, TY28 argument conversion, TY29 cast, TY30 atomic | pointer/enum/atomic/VLA model |
| T07 Semantic | `compiler/src/chips/semantic/se_literal.rs`, `se_binary.rs`; catalog SE03/SE07/SE09/SE10/SE11/SE13/SE16/SE19/SE21/SE29 | integer/float conversions, overflow, `_Bool`, char/wide, VLA |
| T08 Constants/layout | `compiler/src/chips/constant_layout_init/fold.rs`, `fold_branch.rs`; catalog CL01–CL26 | target-width constants, overflow, layout, bitfields, initialization, float/long double, VLA |
| T09 IR | `compiler/src/chips/ir_lower/ir_function.rs`; catalog IR03/IR05/IR06/IR07/IR08/IR09/IR10/IR11/IR17/IR19/IR26 | target bit patterns, widths, conversions, float, VLA |
| T10 Optimize | OP02 fixed-width fold, OP03 FP fold | target-width/overflow/FP legality |
| T11 Target | planned `chips/target/aarch64/`; CG01–CG39, especially CG01/CG03/CG04/CG05/CG10–CG15/CG18–CG21/CG32/CG38 | all concrete ABI/integer/float/stack/layout values |
| T12 GNU | EX09 vector, EX35 `_FloatN`, EX39 prefetch; runtime helpers | target format/feature/ABI policy |
| T13 Verification | `compiler/src/chips/verify/vf_interpret.rs` (VF12), `vf_invariant.rs` (VF06), `vf_store.rs` (VF01); VF08 IR invariant, VF11 ABI invariant | symbolic vs fixed-width interpretation, typed-fact completeness, ABI verification |

### 9.3 Tests

| Path | Current coverage | Required extension |
|---|---|---|
| `compiler/tests/c02_target.rs` | 11 tests: identity, unverified state, fixture consistency, synthetic attestation, rejection paths, config codes | model completeness, model-version mismatch, consistency checks, extended fields, no unmeasured scalar defaults |
| `compiler/tests/c13_ty.rs` | canonical `int`, function type, scope/symbol, identity conversions, char vocabulary | symbolic rank/signedness, non-`Int` `Unsupported`, Part B promotion/conversion tests (gated) |
| `compiler/tests/c08_gate1.rs` | literal/const schema, fold, budget, `ConstOverflow`, replay | target-width constant/overflow tests (Part B), typed constant carrier if added |
| `compiler/tests/c37_const_branch.rs` | selected-branch/static-assert M1 | target-width branch/constant tests (Part B) |
| `compiler/tests/c32_float.rs` | float syntax/value core, binary32/64 bits, overflow/subnormal, Ack-only | target format attestation, `float`/`double` format, rounding/fenv/NaN/zero tests |
| `compiler/tests/c11_lex.rs` / `c33_string.rs` | character/string decoding | target char/wchar/execution charset, wide unit width, no unverified target consumption |
| `compiler/tests/c14_se.rs` / `c39_sefunc.rs` | M1 semantic facts | conversions/promotions/`_Bool`/char/float effects |
| `compiler/tests/c15_ir.rs` | symbolic IR Constant/Return | target bit patterns/conversions/floats (Part B) |
| `compiler/tests/c17_vf12.rs` | symbolic interpreter | fixed-width Part B interpreter cases (gated) |
| `compiler/tests/freeze.rs` | contract version/hash | target model schema version/hash updates |
| Proposed (integrator-owned) | `compiler/tests/c02_target_model.rs` (name not frozen) | full model schema/attestation/consistency/migration tests |
| `tools/torture/probe/src/scalar-layout.c` | H01 scalar measurements | extended integer/float/char/wchar fields |
| `tools/torture/probe/normalize/normalize.py` | H01 normalization; 38 fields, 4 ABI unresolved | new fields, model/probe version, consistency checks |
| `tools/torture/probe/README.md` | probe contract and limits | document new fields, model versions, no-value claims |
| `docs/tasks/T01_COMPILER_CONTRACT.md` §6 | target/model freeze requirements | update with the frozen model schema/version |
| `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md` | TY13/TY25/TY26/TY27 target inputs | align symbolic/Part B split |
| `docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md` | CL08–CL26 target layout/const/float/VLA | align field requirements |
| `docs/tasks/T11_TARGET_CHIPS.md` | ABI/float/int128/long double probe-gated acceptance | align model fields and gates |
| `docs/tasks/M1_TARGET_ACCEPTANCE.md` | IR-6 symbolic model; B7/B8 ABI checks | keep Part A symbolic; Part B gated |

---

## 10. Ordered Work Orders (Proposed; Not Authorized)

Every work order below is **PROPOSED**. None is authorized by this report. Each requires the T01 integrator to bind it to the current contract version/hash, exact field manifests, test entry points, and owner. The paths are exact where the file exists; future target files follow T11's directory but their names are integrator-owned.

### WO-A22-01 — Freeze the target-model requirements and field inventory

| Field | Value |
|---|---|
| Owner | T01 integrator + T04/T06/T08/T11 owners (co-freeze) |
| Priority | First; blocks all Part B model work |
| Paths | `docs/tasks/T01_COMPILER_CONTRACT.md` §6; `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md`; `docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md`; `docs/tasks/T11_TARGET_CHIPS.md`; `docs/tasks/TASK_TEMPLATE.md`; `compiler/contracts/CONTRACT_VERSION`; `compiler/src/contract.rs` |
| Scope | Adopt the model dimensions in §5/§8.3, the Part A/B matrix in §6, and the version split in §8.6. Decide exact field names, enum vocabularies, and whether ranges are probed or derived. |
| Acceptance | The four documents and the contract marker name the same model version/field inventory; no concrete value is asserted; the Part A/B boundary is explicit. |
| Forbidden | Any probe value, any chip code, any source edit outside the integrator-owned freeze. |
| Gate | This report reviewed; owner/integrator co-freeze. |

### WO-A22-02 — Extend the H01 probe and normalizer for the model

| Field | Value |
|---|---|
| Owner | H01 (probe area); H07 for classification fields |
| Priority | After WO-A22-01 |
| Paths | `tools/torture/probe/src/scalar-layout.c`; `tools/torture/probe/normalize/normalize.py`; `tools/torture/probe/README.md`; `tools/torture/probe/tests/`; `.github/workflows/t00-target-probes.yml` |
| Scope | Add the fields in §8.4: byte/bit base, missing scalar sizes/aligns, ranges/representation, `_Bool`, float formats/rounding, pointer representation, `size_t`/`ptrdiff_t` signedness, enum/bitfield/layout, VLA/stack, model/probe versions, provenance. Keep the four H07 ABI fields and `abi.*.ok` evidence. |
| Acceptance | No field is copied from `target.rs`; normalizer self-tests pass without a compiler; a real probe remains explicitly not run; unknown fields/duplicates are rejected; all fields are deterministic. |
| Forbidden | Running the probe on macOS/Darwin; claiming H01/C02 complete; fabricating toolchain pins or values. |
| Gate | WO-A22-01; provisioned Linux AArch64 substrate and pinned reference GCC. |

### WO-A22-03 — H07 classification of the model/ABI evidence

| Field | Value |
|---|---|
| Owner | H07 |
| Priority | After WO-A22-02 and a real H01 report |
| Paths | H07-owned artifact per `docs/tasks/T00_GCC_TORTURE_GATE.md` §4.1 (no frozen path today; do not invent one); consumes `tools/torture/probe/**` outputs and `compiler/contracts/target/aarch64-linux-probe.txt` field names |
| Scope | Classify every unresolved field from hashed H01 evidence; require all `abi.*.ok=1`; produce resolved-field and annex hashes; reject ambiguous/heuristic values. |
| Acceptance | Field-level evidence and hashes are recorded; unresolved stays unresolved; no proposal value is copied; classifier is deterministic. |
| Forbidden | Attesting the target; editing H01 artifacts; selecting Darwin/Mach-O values. |
| Gate | WO-A22-02; H01 report exists. |

### WO-A22-04 — Target model schema and attestation validation

| Field | Value |
|---|---|
| Owner | T01 integrator |
| Priority | After WO-A22-01; final step after WO-A22-03 |
| Paths | `compiler/src/target.rs`; `compiler/src/contract.rs`; `compiler/contracts/CONTRACT_VERSION`; `compiler/contracts/target/aarch64-linux-probe.txt`; `compiler/tests/c02_target.rs`; `compiler/tests/freeze.rs`; proposed `compiler/tests/c02_target_model.rs` (integrator-owned) |
| Scope | Add the versioned model types/fields, model/probe schema version parsing, consistency checks, `TargetModelVersion`/schema hash, attestation result metadata, and fail-closed errors. Extend `attest` to require the complete model, not 38 fields. Split or explicitly freeze the contract hash treatment. |
| Acceptance | All §8.5 failures are covered by tests; no unmeasured scalar default survives attestation; `require_verified` checks identity + model version; a synthetic complete report verifies, a missing/inconsistent one fails; no real target is claimed. |
| Forbidden | Weakening the private `VerificationState`; accepting `status` as codegen evidence; using proposal values as defaults after attestation; modifying unrelated source. |
| Gate | WO-A22-01/02/03; integrator freeze. |

### WO-A22-05 — Freeze the Part A symbolic type/constant model

| Field | Value |
|---|---|
| Owner | T06/T08 + T01 integrator |
| Priority | Can proceed after WO-A22-01; independent of the probe |
| Paths | `compiler/src/bus.rs` (`TypeKind`, `IntRank`, `CharKind`, `LiteralRecord`, `ConstRecord`, `SemRecord`); `compiler/src/chips/types/ty_types.rs`; `compiler/src/chips/types/ty_conv.rs`; `compiler/src/chips/constant_layout_init/fold.rs`; `compiler/tests/c13_ty.rs`; `compiler/tests/c08_gate1.rs`; `compiler/src/contract.rs`/`CONTRACT_VERSION` |
| Scope | Add symbolic float/pointer/array/aggregate/enum types as needed; keep rank/signedness only; preserve no-width constants; make `ConstUnsupported` explicit for unverified-width rules; decide whether `ConstRecord` needs a symbolic `ty` link or the IR value remains the type carrier. |
| Acceptance | No record contains a target width/bit pattern; `int→int` identity preserved; non-`Int` conversions fail `Unsupported`; replay deterministic; contract hash updated by integrator. |
| Forbidden | Implementing Part B conversions or target layout under this order. |
| Gate | WO-A22-01; T01 `/6`/next contract freeze. |

### WO-A22-06 — Part B integer promotions, conversions, and overflow

| Field | Value |
|---|---|
| Owner | T06/T07/T09 (co-freeze) |
| Priority | After WO-A22-04 (attested ranges/representation) |
| Paths | `compiler/src/chips/types/ty_conv.rs` and new per-chip files under `compiler/src/chips/types/` for TY25/TY26/TY27/TY28/TY29; `compiler/src/chips/semantic/`; `compiler/src/chips/ir_lower/`; `compiler/tests/c13_ty.rs`; `compiler/tests/c14_se.rs`; `compiler/tests/c15_ir.rs`; proposed conversion tests |
| Scope | Range-based promotions, usual arithmetic conversions, assignment/argument/cast conversions, `_Bool`, signed/unsigned narrowing, overflow/shift/division policy, `-fwrapv` overlay. |
| Acceptance | Positive/negative cases against the attested model; no host widths; `INT_MIN`/unsigned boundaries; unsupported cases typed; deterministic replay. |
| Forbidden | Hard-coded widths, host `usize`/`i128` semantics, silent wrap, target code before attestation. |
| Gate | WO-A22-04; target model attested. |

### WO-A22-07 — Part B floating-point model and literal conversion

| Field | Value |
|---|---|
| Owner | T04/T08/T11 (co-freeze) |
| Priority | After WO-A22-04 |
| Paths | `compiler/src/chips/lex/lx_float_syntax.rs`; `compiler/src/chips/lex/lx_float_value.rs`; `compiler/src/target.rs` (`FloatFormat`/floating model); `compiler/src/chips/constant_layout_init/` (float constant/layout chips); `compiler/tests/c32_float.rs`; proposed float model tests |
| Scope | Attested `float`/`double`/`long double` formats; correct rounding; suffix mapping; `long double`; NaN/zero/subnormal/fenv/`FLT_EVAL_METHOD`/contraction; `-ffast-math` overlay. |
| Acceptance | Known-bit fixtures for each attested format; halfway/overflow/subnormal/NaN/zero tests; no host `f64` shortcut; `l/L` either implemented or explicitly `Unsupported`; no target claim without attestation. |
| Forbidden | Assuming binary32/64; using Rust float rounding as target semantics; enabling fast-math silently. |
| Gate | WO-A22-04; target model attested. |

### WO-A22-08 — `_Bool`, plain `char`, `wchar_t`, and execution charset

| Field | Value |
|---|---|
| Owner | T04/T06/T08 |
| Priority | After WO-A22-04 |
| Paths | `compiler/src/chips/lex/lx_char.rs`; `compiler/src/chips/lex/lx_string.rs`; `compiler/src/chips/lex/lx_escape.rs`; `compiler/src/chips/types/ty_types.rs`; `compiler/src/bus.rs`; `compiler/tests/c11_lex.rs`; `compiler/tests/c33_string.rs`; `compiler/tests/c13_ty.rs` |
| Scope | Attested `_Bool` representation/conversion; plain `char` signedness/range; `wchar_t` type/size/encoding/range; execution character set; wide/UTF-16/UTF-32 units; remove the unverified-target consumption leaks. |
| Acceptance | `L`/`u`/`U` units follow the attested model; plain `char` conversions use attested signedness; unknown width/encoding fails; no unverified `WCharT` read. |
| Forbidden | Hard-coded 32-bit wide units; UTF-8 execution-charset claim without a model; reading `TargetSpec` without verification. |
| Gate | WO-A22-04; target model attested. |

### WO-A22-09 — Pointer, `long double`, `__int128`, enum, bit-field, layout, and VLA dependencies

| Field | Value |
|---|---|
| Owner | T08/T09/T11 (co-freeze) |
| Priority | After WO-A22-04; can split by independent owner |
| Paths | `compiler/src/chips/constant_layout_init/` (CL08–CL15, CL22, CL26); `compiler/src/chips/ir_lower/` (IR03/IR05–IR11/IR17/IR19/IR26); target files under `compiler/src/chips/target/aarch64/` per T11 (exact names integrator-owned); `compiler/tests/c15_ir.rs`; proposed layout/ABI tests |
| Scope | Pointer representation/null/int conversion; `long double` ABI/ops/helpers; `__int128`; enum underlying; bit-field/layout; `_Atomic`; VLA/stack; `size_t`/`ptrdiff_t` relations. |
| Acceptance | Layout offsets/sizes/alignments from the attested model; no host `sizeof`; pointer/VLA/ABI tests; unsupported features explicit; T11 ABI verification (VF11) eventually. |
| Forbidden | Inventing concrete layout/ABI values; using host pointers; claiming VLA/bit-field/long-double support from symbolic tests. |
| Gate | WO-A22-04; per-chip interface freeze. |

### WO-A22-10 — Verification and evidence for the target model

| Field | Value |
|---|---|
| Owner | T13 + independent testers |
| Priority | Alongside WO-A22-04..09 |
| Paths | `compiler/src/chips/verify/vf_interpret.rs`; `compiler/src/chips/verify/vf_invariant.rs`; `compiler/src/chips/verify/vf_store.rs`; `compiler/tests/c17_vf12.rs`; `compiler/tests/c19_vf01.rs`; proposed VF06/VF08/VF11 tests; `compiler/tests/c02_target_model.rs` |
| Scope | Verify model completeness, write-scope, replay, symbolic-vs-fixed-width separation, ABI invariant, and failure classification. |
| Acceptance | No unsupported outcome counts as pass; target model version/hash is checked; evidence includes model version/report hash; negative tests for missing/inconsistent fields. |
| Forbidden | Marking symbolic interpreter output as target execution; using a passing unit test as a probe/ABI pass. |
| Gate | WO-A22-04; T13 registration freeze. |

### WO-A22-11 — Codegen gate and contract artifact update

| Field | Value |
|---|---|
| Owner | T01 integrator + H04/T11 |
| Priority | Last; only after a real attestation |
| Paths | `compiler/src/target.rs`; `compiler/src/contract.rs`; `compiler/contracts/CONTRACT_VERSION`; `compiler/src/bin/candidate.rs`; H04/T11 acceptance tests |
| Scope | Bind `ensure_codegen_ready` to the attested model version/hash; update the contract artifact (`target_verification=probed`, model/probe versions, provenance); enable `-S`/`-c` only for the attested model; keep `-c`/link policy per H01/H04. |
| Acceptance | No codegen without matching attestation; stale/old-model reports rejected; contract artifact records the model/version/hash; no target value is claimed beyond the attested report. |
| Forbidden | Turning on codegen from a synthetic/macOS report; updating the contract hash without a freeze. |
| Gate | WO-A22-04; real H01/H07/integrator attestation; T01 freeze. |

---

## 11. Unresolved Values and Open Items

### 11.1 Known identity (not values)

- triple `aarch64-unknown-linux-gnu`;
- object format ELF;
- data model LP64 (declared relation);
- little-endian;
- ABI name AAPCS64;
- the `ScalarKind`/`FloatFormat`/`WideCharEncoding` enum vocabularies and the 38-field probe shape (schema, not values);
- the Part A symbolic M1 subset: `int`, `int(void)`, `LiteralRecord`, `ConstRecord`, `SemRecord`, `2+3=5`;
- the private `VerificationState` mechanism and the `require_verified` gate.

### 11.2 Unresolved concrete values (comprehensive)

| Area | Unresolved values/fields | Current status |
|---|---|---|
| Scalar layout | every `sizeof`/`alignof`; 13 are measured by the probe, 8 are not even in `MEASURED_SCALARS`; all remain unverified until a real attestation | **UNVERIFIED** |
| Integer model | `CHAR_BIT`, width/value bits/padding, ranges, representation, trap policy, rank/width relation | **MISSING/UNVERIFIED** |
| `_Bool` | size, alignment, representation, conversion, bit-field behavior | **MISSING/UNVERIFIED** |
| Plain `char` | signedness, range, execution charset, multibyte policy | `char.signed` field exists; no attested value; no model consumption |
| `wchar_t` | size/alignment/signedness/encoding/range/type/ABI | fields exist; no attested value; model/type missing |
| `short`/`int`/`long`/`long long` | widths/ranges/signedness/representation | size/alignment fields only |
| `__int128`/`unsigned __int128` | width/range/signedness/repr/ABI/helpers | size/alignment fields only |
| `float`/`double` | format/precision/exponent/rounding/ABI | size/alignment; format not attested |
| `long double` | format/precision/exponent/size/align/ABI/ops/helpers | size/align/format labels; no operations/ABI |
| Pointer | representation/null/function pointer/int conversion/provenance | size/align only |
| `size_t`/`ptrdiff_t` | width/signedness/relation to pointer/VLA | size/align only |
| Enum | underlying type/range/rank/promotion | **MISSING** |
| Bit-field | allocation unit/order/signedness/packing | **MISSING** |
| Struct/union | offsets/padding/alignment/tail/`max_align_t`/`_Alignas` | **MISSING** |
| `_Atomic` | size/align/lock-free/ABI | **MISSING** |
| VLA | `size_t`/`ptrdiff_t`, stack alignment, dynamic stack/cleanup | **MISSING** |
| Integer promotions | range-based promotion decisions | **MISSING** (identity only) |
| Usual arithmetic conversions | rank/signedness/float hierarchy | **MISSING** (identity only) |
| Assignment/argument/cast conversions | all target conversion rules | **MISSING** (identity only) |
| Signed overflow | UB/`-fwrapv`/unsigned modulo/shift/div | **MISSING** |
| Integer constants | target candidate list/suffix/range/overflow | Part A only |
| Float literals | target format/rounding/range/`long double` | binary32/64 core only |
| Float environment | rounding mode/`FLT_EVAL_METHOD`/`fenv`/NaN/zero/subnormal/contraction | **MISSING** |
| ABI | GP/FP counts, stack alignment, variadic save area (proposal fields); HFA/HVA, aggregate/return/sret, x18, TLS, relocations, target features | 4 fields only; all unverified |
| Execution charset | source/execution/wide encodings | **MISSING**, UTF-8 assumption in `lx_escape` |
| Target options | `-fwrapv`, `-ffast-math`, `-fshort-enums`, `-fpack-struct`, PIC/PIE | partial `OptionFlag`; no model semantics |
| Model versioning | target model schema version, probe schema version, schema hash, provenance fields | **MISSING** |
| Attestation | complete model fields, consistency checks, model-version check, H07 classification | **MISSING/UNVERIFIED** |

### 11.3 Open questions

1. **Schema placement.** Should the target model be a separate `TargetModelSchema` artifact or an extension of `TargetSpec`? How does `CompilerConfig` carry the version/hash without breaking immutable-config semantics?
2. **Contract hash treatment.** Does the contract hash cover concrete target values (current `FrozenSchema::encode` includes them) or only schema/version? Choose split vs frozen-attested-value artifact explicitly.
3. **Ranges vs width/representation.** Should the probe measure `min/max` directly (simplest for promotions) or measure `CHAR_BIT` + width + representation + padding and derive? Either way, no field may be defaulted.
4. **`CHAR_BIT` and bit widths.** `ScalarModel.size: u8` holds byte sizes; bit widths up to 128 need a wider field. Confirm the representation and whether `CHAR_BIT` can differ from 8 for this identity (do not assume).
5. **Signed representation.** Is the target necessarily two's complement? C11 permits other representations; the model must record it, and `-fwrapv` must be an overlay, not a base assumption.
6. **Signed overflow policy.** Is the baseline ISO UB, and `-fwrapv` the only defined wrap mode? What about `-fno-strict-overflow` and `-ftrapv`? Which options enter `OptionFlag`?
7. **`_Bool` representation.** Is `_Bool` a 1-byte object with values 0/1? What is its alignment/ABI class? Do not assume; probe.
8. **Plain `char`.** Signed or unsigned? How do character constants, string literals, escapes, and `char` objects interact with the execution charset? The current `char.signed` probe exists but no code consumes it.
9. **`wchar_t` encoding.** The probe normalizer only resolves `utf32` or `unresolved`; is `utf16` ever valid for this identity? If not, should `WideCharEncoding::Utf16` be rejected by attestation?
10. **`long double` format.** Is the ABI long double binary128, binary64, or an ABI-specific choice? What is its argument/return class, and which libgcc/compiler-rt helpers are required? T11 must not assume.
11. **Float formats.** Are `float`/`double` always binary32/binary64 for this identity? If so, that is an ABI freeze decision with probe evidence, not a code default. What about `_Float16`, `_Float128`, and `-mlong-double-*`?
12. **FP environment.** What is the baseline `fenv` model: default round-to-nearest-even, exceptions masked, `FENV_ACCESS OFF`? Is `-ffast-math` allowed to change NaN/zero/subnormal behavior, and how is that recorded?
13. **NaN/zero policy.** Which NaN payload propagation/canonicalization rules are required for bit-exact tests? How are `-0.0` and `x + 0.0` handled at `-O0` and optimized levels?
14. **Pointer representation.** Is a flat 64-bit pointer model sufficient? Do function/data pointers share representation? Is `intptr_t` available? Is pointer provenance part of the model or only semantics?
15. **Enum/bit-field/packing.** What is the enum underlying selection rule? What are the bit-field allocation/order/packing rules, and how do `-fshort-enums`/`-fpack-struct`/`#pragma pack` overlay them?
16. **VLA/stack.** What is the VLA size type and dynamic-stack mechanism? How do stack probing, alignment, and cleanup interact with resource limits? Are VLAs in the frozen corpus?
17. **ABI scope.** Which T11 ABI fields belong to A-ABI-02 versus A-ABI-03..08? The boundary proposed here is: integer/float/pointer scalar/type rules in A-ABI-02; argument/return/stack/variadic/aggregate classification in A-ABI-03..07.
18. **Probe provenance.** Should the attestation report include H01 `report.sha256`, canonical body hash, H07 annex hash, toolchain pins, substrate run id, and model schema hash in every case? A-ABI-01 proposes this; A22 requires model-specific versions too.
19. **Test naming/ownership.** The exact target-model test file name and attestation CLI path are integrator-owned; do not invent them as frozen.
20. **Part A/Part B migration.** If Part A adds symbolic float/pointer/aggregate types later, how are existing M1 records migrated without changing the `/39` hash semantics?

---

## 12. Non-Claims and Review Checklist

### 12.1 Non-claims

- This report is **design only**. It changes no source, test, contract, Cargo file, `mod.rs`, shared index, or other document.
- No probe was run; no H01 report/H07 classification/attestation exists.
- No concrete ABI value is verified; every number in `target.rs`/the probe fixture is a proposal.
- The targeted `c02_target` run proves the existing mechanism against synthetic reports, not the target ABI.
- No GCC torture rate, M1 execution, or Part B capability is claimed.
- No macOS/Darwin arm64 value is used or endorsed.
- The work orders in §10 are proposals, not authorizations, and do not start agents or modify code.

### 12.2 Review checklist

1. Does every concrete value in the report carry a proposal/unverified label, and is no proposal value used as an acceptance expectation?
2. Are frozen identity, attested values, proposal values, and unresolved fields explicitly separated?
3. Does the Part A/B matrix keep width/bit-pattern/format/ABI out of Part A?
4. Are `_Bool`, plain `char`, `wchar_t`, `float`/`double`/`long double`, pointer, `__int128`, enum, bit-field, layout, `_Atomic`, VLA, and execution charset all covered?
5. Are integer promotions, usual arithmetic conversions, assignment/argument/cast conversions, overflow, shifts/division, and constant expressions covered with the correct Part A/Part B split?
6. Are float rounding/NaN/signed zero/subnormal/`fenv`/`FLT_EVAL_METHOD`/contraction and `-ffast-math` covered?
7. Does the attestation plan require a complete, versioned model rather than the current 38 fields?
8. Are the eight unmeasured scalar kinds and the unattested `float`/`double` formats called out as defects, not accepted defaults?
9. Are the current Part B leaks (`lx_string`, `lx_char`, `lx_float_value`, `lx_escape`) identified with exact paths and a fail-closed disposition?
10. Does the versioning plan distinguish contract version, target-model schema version, probe schema version, and attestation hash?
11. Are the work orders ordered, path-specific, owner-labeled, and explicitly blocked on the probe/freeze where required?
12. Does the report avoid claiming verification, implementation, probe, or pass rates?

---

## 13. Bottom Line

The target identity is frozen and the Part A symbolic M1 subset is usable. The target **model** is not yet a C integer/floating-point model: it is a narrow size/alignment proposal plus four ABI counts, with no ranges, representation, promotions, overflow, float environment, `_Bool`/char/wide semantics, pointer/VLA/layout model, or model version. Attestation is a sound start but currently attests an incomplete model and leaves eight scalar kinds at proposal defaults.

Part A can continue. Part B must remain fail-closed until the model schema is frozen, the probe is extended, H01 runs on the pinned Linux AArch64 substrate, H07 classifies the evidence, and the T01 integrator attests a complete, versioned model. No target integer/float rule should be implemented before that gate.
