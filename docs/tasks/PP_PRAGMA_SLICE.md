# Wave 3 Slice 4: PP Pragma Freeze (`/29`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/29` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/28` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/29` delta), [T03](T03_PREPROCESS_CHIPS.md) PP25 row, [line slice](PP_LINE_SLICE.md) |
| Non-goal | `PragmaRecord` carrier on the bus (the classification is returned to the wiring layer; no store field lands), PP05 `pragma` fan-out wiring, once-consultation in include-resolve |

## 1. What this slice is

One new worker closing pragma-dispatch USE: `PpPragmaChip` (PP25)
classifies one pragma construct (`PragmaTokens -> PragmaRecord`, T03:33).
The payload is exactly one pragma construct in payload order, in either
accepted form:

- Post-`#` refs whose first token is Identifier `pragma` — the
  remainder are the directive params, possibly empty.
- The four operator tokens Identifier `_Pragma`, Punctuator `(`,
  StringLiteral, Punctuator `)`.

`_Pragma("...")` strings decode per `decode_pragma_string` (quotes
stripped, simple one-character escapes mapped, optional `u8`/`u`/`U`/`L`
prefix tolerated without transcoding). Params and decoded operator text
classify to `PragmaClass`: `once` (header-guard flag), `pack` with a
`push`/`pop` operator (alignment-stack op), everything else opaque.
Success completes `Ack` with no bus writes (the wiring layer consumes
the classification via `classify_directive_params` /
`classify_operator_text`); every failure path is a typed `Fail`.
Malformed input is never silently accepted.

Unknown-pragma policy (deliberate, per ISO C11 6.10.6p1): any
well-formed but unrecognized pragma classifies `Opaque` and completes
`Ack` — benign ignore, NOT `Fail`. Only a malformed `_Pragma` operand
(wrong arity, non-string literal, missing quotes, raw newline/quote,
dangling backslash, or a non-simple escape) fails as typed `Invalid`.

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| `#pragma once` | `Ack`; `Once` |
| `#pragma pack(push, ...)` / `#pragma pack(pop, ...)` | `Ack`; `PackPush` / `PackPop` |
| `#pragma foobar`, bare `#pragma` | `Ack`; `Opaque` (benign ignore) |
| `_Pragma("once")`, `_Pragma("pack(push)")` | `Ack`; `Once` / `PackPush` |
| `_Pragma("a\nb")`, `_Pragma(u8"once")` | `Ack`; escapes decoded, prefix tolerated |
| `_Pragma` with wrong arity / non-string operand | Typed `Invalid` `Fail` (malformed) |
| `_Pragma("a\qb")` (non-simple escape) | Typed `Invalid` `Fail` (unsupported escape) |
| Unquoted / newline / dangling backslash | Typed `Invalid` `Fail` (malformed) |
| Non-pragma first token, empty payload | Protocol-fault `Fail` (wrong dispatch) |

`pack` arguments beyond the `push`/`pop` operator (e.g. an alignment
value) are not interpreted; the stack op is recorded and the remainder
is benignly ignored with the record store.

## 3. Mismatch matrix (all typed `Fail`)

- Non-`Running` dispatch → task-protocol failure.
- Payload empty or not pp-token refs → protocol failure.
- First token neither `pragma` nor `_Pragma` → protocol failure.
- Every row of §2 marked `Fail` → `Invalid` naming the violated rule.

## 4. Frozen registration

- `preprocess.pragma_directive` (local 33, first code after `/28`),
  `Frozen`; `pp_pragma_slice()` registry (46 entries, cumulative over
  `pp_line_slice()`); stage row (`→ 1`); routed layer 1;
  `PP25_CHIP = 36`; no allowlist row (Ack-only, read-only — verified
  against the manifest's empty write set); same schemas
  (`pp_macro_slice()`).
- Hash rules: `pp.pragma-once`, `pp.pragma-pack`,
  `pp.pragma-unknown-ignore`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| A1-1 | `#pragma once` | once case |
| A1-2 | `pack` push/pop | pack case |
| A1-3 | Unknown pragma ignored (`Ack`) | unknown case |
| A1-4 | Malformed `_Pragma` operand fails | malformed case |
| A1-5 | Bad string escape fails typed | string case |
| A1-6 | Operator decode + classify | operator case |
| A1-7 | Registration freeze (kind/registry/stage/layer/chip, read-only) | freeze + gates green |
| A1-8 | Bus dispatch (`Ack`, no writes; malformed fails) | bus case |

## 6. Execution record

Delivered as a single untracked chip file (`pp_pragma.rs`) against the
`/28` tree; integrated by the T01 integrator: froze local 33 /
`ChipId(36)` / `preprocess.pragma_directive` into `task.rs`, `PP25_CHIP`
+ `is_pp_pragma_slice_kind` + stage row into `manifest.rs` (no allowlist
row: the manifest declares no writes), three rule ids into
`contract.rs`, bumped to `t01-c01-c06/29` with recomputed hash
(`c500d9ff…e019324`), wired `preprocess/mod.rs` + `chips/mod.rs`, and
re-pointed the chip's proposed consts at the frozen canonicals (local
30 → 33, `ChipId(33)` → `PP25_CHIP`, kind-fallback fn → `PP25_TASK_KIND`
alias, test path → `c29_pragma.rs`; no logic change). Verified by
`compiler/tests/c29_pragma.rs` (9 tests: freeze, once, pack push/pop,
unknown ignore, malformed operand, bad string, operator decode, bus
dispatch, gates). No integration corrections to chip logic were needed.

## 7. Explicitly deferred (all loud, never silent)

- `PragmaRecord` carrier on the bus (once-consultation by PP17/PP18
  waits on the record freeze; PP24 `__LINE__` keeps its physical-line
  fallback).
- PP05 `pragma` fan-out wiring (control-loop slice scope).
- PP27 expansion map, PP28 preprocessed emit (later slices).
