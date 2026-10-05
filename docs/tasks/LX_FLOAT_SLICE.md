# Wave 3 Slice 7: LX Float Freeze (`/32`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/32` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/31` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/32` delta), [T04](T04_LEX_CHIPS.md) LX09/LX10 rows, [LX slice](LX_SLICE.md) |
| Non-goal | A `FloatBits` result carrier or float `LiteralRecord` (both stay unfrozen; float literals are not M1 constants), binary128 conversion, integer/char/string literal changes, LX11–LX15/LX18 wiring |

## 1. What this slice is

Two new workers covering the float USE (T04:17–18): `LxFloatSyntaxChip`
(LX09) validates one committed pp-number spelling (decimal `1e+10`,
`1.5`, `1.`, `.5`, hex `0x1.fp+2` with mandatory binary exponent,
optional `f`/`F`/`l`/`L` suffix) and `LxFloatValueChip` (LX10) checks the
same spelling converts to its suffix-selected format — `f`/`F` →
binary32, absent → binary64 — with correct round-to-nearest-even over
integer arithmetic only (no host floating point). Both chips read
exactly one `RecordRef::PpToken` of kind `PpNumber` (DOC-10 resolution:
decode input is the committed T03 spelling/kind, never a C `TokenId`),
complete `Ack` on success, append nothing, and fail loud otherwise.
pp-numbers are never treated as valid C constants here: no
`LiteralRecord` is emitted (the M1 `LiteralRecord` stays
integer-only; a float literal record is a T04/T08/T01 co-freeze item).

Value semantics (deliberate, not oversights):

- Suffix selects the format: `f`/`F` → binary32, absent → binary64.
  The `l`/`L` suffix selects binary128 (work order + target model),
  whose correctly-rounded conversion is deferred as explicit
  `Unsupported` — never a silent binary64 substitution.
- Range rides as flags, never as value-level failure (C
  `strtod`-like semantics): overflow completes `Ack` (the pure core
  reports infinity + `overflow`/`inexact` flags); underflow to
  zero/subnormal likewise completes `Ack` with flags.
- Malformed float shapes fail `Invalid` with the frozen messages
  (`malformed decimal exponent…`, `hexadecimal float spelling needs a
  binary exponent`, `unsupported suffix…`); integer-shaped spellings
  (LX05–LX08 territory) and non-pp-number inputs fail `Unsupported`.
- The bits are certified by the pure `convert_float_parts` core
  (unit-pinned oracle patterns: halfway-to-even, `0.1`, overflow
  infinity, smallest subnormal), not by the bus `Ack` — the same
  Ack-only position as the LX11 escape draft (D3 there). No host
  `f64` approximation participates anywhere.

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| `1.5`, `1.`, `.5`, `1e+10`, `1E-5`, hex `0x1.fp+2` | `Ack` (both chips) |
| `1e`, `1e+foo`, `0x1`, `0x1.f`, `0x1p` (no digits) | `Invalid` `Fail` (both chips, same messages) |
| `1.5ll`, `1.5d`, `1.0i`, `0x1p+2u` | `Fail` (suffix/exponent taxonomy per §3) |
| `42`, `0x10`-as-integer, `""`, non-pp-number payload | `Fail` (`Unsupported` at LX10; integer territory stays LX05–LX08) |
| `2.5f` / `2.5F` | binary32 path, `Ack` |
| `2.5l` / `2.5L` | `Unsupported` `Fail` (binary128 deferred) at LX10; `Ack` at LX09 (syntax-valid) |
| `16777217.0` (f32), `2^53+1` (f64) | Halfway rounds to even (`0x4B80000`, `0x4340…0000`), `inexact` |
| `0.1` | `0x3DCCCCCD` / `0x3FB999999999999A`, `inexact` |
| `1e400` | Infinity + `overflow`/`inexact` flags; bus `Ack` |
| Smallest subnormal `0x0.…1p-1022`, half-subnormal `0x0.8p-1074` | Exact bits `1`; zero + `underflow`/`inexact` |
| Wrong kind, non-`Running`, empty/non-pp-token payload, missing token | Protocol-fault `Fail` |

## 3. `FloatParts` reconciliation (frozen)

No frozen `FloatParts` record exists, so **neither chip-local shape is
the frozen interchange**. The frozen interchange is the **spelling**
plus the three hashed rules below; each chip keeps its own local copy
by the `compose_map`/`decimal_magnitude` copy precedent and imports
nothing from its sibling:

- LX09-local (syntax-only): `{ radix, has_point, exponent:
  Option<FloatExponent>, suffix }` — classification, no digits.
- LX10-local (digit-bearing): `{ negative, radix, int_digits,
  frac_digits, exponent: i64, suffix }` — conversion input. `negative`
  is always false from spelling (`-` is a separate punctuator); the
  field serves the pure core and its unit tests only.
- LX10's chip-local `spelling_to_value_parts` mirrors the LX09 syntax
  rule case-for-case (same accept set; same `Invalid` messages, pinned
  by the agreement case). Known deliberate deltas, both loud: `l`/`L`
  is syntax-valid (LX09 `Ack`) but value-deferred (LX10
  `Unsupported`); a repeated suffix (`1.5ll`) is `BadExponent` in
  both chips (the suffix rule consumes one letter, the tail check
  rejects the rest — same as LX09).
- Still open (T04/T01 `/6` co-freeze): a shared `FloatParts` record
  carrier, a `FloatBits` result carrier, and the float `LiteralRecord`
  shape/vocabulary. Until then, no downstream stage may treat an
  `Ack` as carrying a value.

## 4. Frozen registration

- `lex.float_syntax` (local 19, first code after `/11`) and
  `lex.float_value` (local 20), both `Frozen`;
  `lx_float_slice()` registry (50 entries, cumulative over
  `pp_emit_slice()`); stage rows (`→ 2`); routed layer 2;
  `LX09_CHIP = 39`, `LX10_CHIP = 40` (first-free after `PP28_CHIP =
  38`; the unwired LX12/LX13 drafts' local `39` claims and LX09's
  draft `40` claim are superseded — those files stay untouched and
  unwired); zero allowlist rows (both chips Ack-only, zero writes;
  `STORE_OWNER_ALLOWLIST` stays 30 rows); same schemas
  (`pp_macro_slice()`).
- Hash rules: `lx.float-syntax`, `lx.float-value-rounding`,
  `lx.float-overflow`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| F1-1 | Decimal syntax accepts | decimal case |
| F1-2 | Hex syntax accepts (mandatory binary exponent) | hex case |
| F1-3 | Bad exponents/integer shapes fail `Invalid`/loud | bad-exponent case |
| F1-4 | Bad suffixes fail | suffix case |
| F1-5 | Suffix selects format; `l` deferred | suffix-format case |
| F1-6 | Halfway rounds to even with known bits | halfway case |
| F1-7 | Overflow → infinity + flags, still `Ack` | overflow case |
| F1-8 | Subnormal grid with underflow flags | subnormal case |
| F1-9 | Syntax/value agreement over the corpus | agreement case |
| F1-10 | Registration freeze (kinds/registry/stage/layer/chips/allowlist) | freeze + gates green |
| F1-11 | Bus dispatch (`Ack`, no appends) + replay determinism | bus case |

## 6. Execution record

Delivered as two untracked chip files (`lx_float_syntax.rs`,
`lx_float_value.rs`) against the `/31` tree; integrated by the T01
integrator: froze locals 19/20 / `ChipId(39/40)` /
`lex.float_syntax` + `lex.float_value` into `task.rs`, `LX09_CHIP` +
`LX10_CHIP` + `is_lx_float_slice_kind` + two stage rows into
`manifest.rs`, three rule ids into `contract.rs`, bumped to
`t01-c01-c06/32` with recomputed hash (`0dd8da06…4a6e1167`), wired
`lex/mod.rs` + `chips/mod.rs`, and re-pointed the chips' proposed
consts at the frozen canonicals (`LX09_TASK_KIND`/`LX10_TASK_KIND`
aliases, test path → `c32_float.rs`). LX09 needed no logic change;
LX10's deferred-wiring `Fail`-always bus path was replaced with a
bus projector over the committed spelling plus the chip-local
spelling→parts parser above (the correctly-rounded core, `Big`,
and its unit tests are unchanged). Verified by
`compiler/tests/c32_float.rs` (12 tests: freeze, decimal, hex, bad
exponent, bad suffix, suffix format, halfway bits, overflow,
subnormal, agreement, bus dispatch + determinism, stage-layer
gates). Full §5 suite green at commit.

## 7. Explicitly deferred (all loud, never silent)

- `FloatBits` result carrier and float `LiteralRecord`
  shape/vocabulary (T04/T08/T01 co-freeze; no downstream stage may
  treat `Ack` as a value).
- binary128 (`l`/`L`) correctly-rounded conversion (explicit
  `Unsupported`; the integer core generalizes to it).
- A shared `FloatParts` record (the two local shapes stay copies by
  precedent until the carrier freezes).
- LX11–LX15/LX18 wiring (their files are untouched by this slice;
  their local `ChipId(39)`/kind claims remain unwired candidates).
