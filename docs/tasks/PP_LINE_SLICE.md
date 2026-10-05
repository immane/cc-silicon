# Wave 3 Slice 3: PP Line Freeze (`/28`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/28` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/27` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/28` delta), [T03](T03_PREPROCESS_CHIPS.md) PP23 row, [builtin slice](PP_BUILTIN_SLICE.md) |
| Non-goal | Logical-location carrier on the bus (the validated location is returned to the wiring layer; no store field lands), include-return re-assertion wiring, `candidate` line orchestration |

## 1. What this slice is

One new worker closing line-directive USE: `PpLineChip` (PP23)
validates one directive line (`LineTokens -> LogicalLocation`, T03:31).
The payload is the directive line's pp-token refs in payload order, in
either accepted form:

- `#line number "file"?` — second token Identifier `line`, third the
  pp-number line number, optional fourth the file string, nothing after.
- GNU line marker `# lineno "file" flags?` — second token the pp-number
  line number, optional third the file string, further tokens must be
  pp-numbers (flag values accepted and ignored).

The `#` introducer accepts the `%:` spelling, matching PP05. Success
completes `Ack` with no bus writes (the wiring layer consumes the
validated location via `line_location`); every failure path is a typed
`Fail`. Malformed input is never silently accepted.

Physical vs logical: the physical location is where the directive line
sits (payload spans in their owning `SourceId`); the logical location
is what the directive declares for following lines (1-based number plus
optional file), so `__LINE__`, `__FILE__`, and diagnostics agree,
including across an include return (the includer re-asserts its own
logical location). A missing file means retain-current-file
(`file: None`).

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| `#line 42` | `Ack`; `(line 42, file None)` |
| `#line 5 "f.h"` | `Ack`; `(line 5, file "f.h")` |
| `# 7 "g.h" 1 2` | `Ack`; flags ignored |
| `# 9` (no file) | `Ack`; `(line 9, file None)` |
| `2147483647` | `Ack` (boundary max `2^31 - 1`) |
| `0`, `2147483648`, larger | Typed `Invalid` `Fail` (range `1..=2147483647`) |
| Non-digit pp-number (`1e5`) | Typed `Invalid` `Fail` |
| Non-`\\`/`\"` escape, trailing backslash, missing quotes | Typed `Invalid` `Fail` |
| Non-string token in file position, trailing tokens | Typed `Invalid` `Fail` |

File decoding accepts only `\\` and `\"` escapes inside a `"..."`
string literal.

## 3. Mismatch matrix (all typed `Fail`)

- Non-`Running` dispatch → task-protocol failure.
- Payload empty or not pp-token refs, or missing spans → protocol
  failure.
- First token not `#`/`%:`, or second token neither `line` nor a
  pp-number → `Invalid`.
- Every row of §2 marked `Fail` → `Invalid` naming the violated rule.

## 4. Frozen registration

- `preprocess.line_directive` (local 32, first code after `/27`),
  `Frozen`; `pp_line_slice()` registry (45 entries, cumulative over
  `pp_builtin_slice()`); stage row (`→ 1`); routed layer 1;
  `PP23_CHIP = 35`; no allowlist row (Ack-only, read-only — verified
  against the manifest's empty write set); same schemas
  (`pp_macro_slice()`).
- Hash rules: `pp.line-logical`, `pp.line-gnu-marker`, `pp.line-range`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| A1-1 | `#line` with/without file | hash-line cases |
| A1-2 | GNU marker (flags ignored) | gnu case |
| A1-3 | Boundary max accepted | boundary case |
| A1-4 | Zero/overflow fail | range case |
| A1-5 | Bad escape / non-string file fail | escape case |
| A1-6 | Physical-vs-logical split | physical case |
| A1-7 | Registration freeze (kind/registry/stage/layer/chip, read-only) | freeze + gates green |
| A1-8 | Bus dispatch (`Ack`, no writes; zero fails) | bus case |

## 6. Execution record

Delivered as a single untracked chip file (`pp_line.rs`) against the
`/27` tree; integrated by the T01 integrator: froze local 32 /
`ChipId(35)` / `preprocess.line_directive` into `task.rs`, `PP23_CHIP`
+ `is_pp_line_slice_kind` + stage row into `manifest.rs` (no allowlist
row: the manifest declares no writes), three rule ids into
`contract.rs`, bumped to `t01-c01-c06/28` with recomputed hash
(`5af3f3fb…48acc71`), wired `preprocess/mod.rs` + `chips/mod.rs`, and
re-pointed the chip's proposed consts at the frozen canonicals (local
30 → 32, `ChipId(33)` → `PP23_CHIP`, raw-kind check → kind check, test
path → `c28_line.rs`; no logic change). Verified by
`compiler/tests/c28_line.rs` (10 tests: freeze, `#line` ± file, GNU
marker, boundary max, zero/overflow, bad escape, physical-vs-logical,
bus dispatch, gates). No integration corrections to chip logic were
needed.

## 7. Explicitly deferred (all loud, never silent)

- Logical-location carrier on the bus (PP24 `__LINE__` keeps its
  physical-line fallback until the carrier lands).
- Include-return re-assertion wiring (control-loop slice scope).
- PP25 pragma (later slice; file left untracked by this slice).
