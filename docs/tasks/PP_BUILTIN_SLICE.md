# Wave 3 Slice 2: PP Builtins Freeze (`/27`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/27` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/26` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/27` delta), [T03](T03_PREPROCESS_CHIPS.md) PP24 row, [variadic slice](PP_VARIADIC_SLICE.md) |
| Non-goal | PP23 logical line mapping (physical-line fallback stays), cross-tick `__COUNTER__` monotonicity (frozen counter carrier pending), `__DATE__`/`__TIME__` replay (frozen config date/time record pending), `-D` (H04), `candidate` builtin orchestration |

## 1. What this slice is

One new worker closing builtin-macro USE: `PpBuiltinChip` (PP24)
expands one builtin-macro use (`MacroName/context -> Tokens`, T03:32).
The payload is exactly one `RecordRef::PpToken` naming the builtin use
(which must be an `Identifier`); all context (`__FILE__` source name,
`__LINE__` span, `__COUNTER__` seed) derives from that token's span
chain, so no fan-out is needed: pure single-task (one dispatch, at
most one quota-1 `PpToken` append, `Complete(Records)`), and the
frozen-join path never applies (no children are ever enqueued).

Documented fallbacks (no guessing, no host reads):

- `__LINE__`: PP23 (`LineDirectiveChip`) is not implemented and no
  logical-location carrier exists on the bus, so the physical line is
  used (newlines before `span.start` plus one). The projector prefers
  the frozen PP23 logical location once that carrier lands.
- `__COUNTER__`: the bus-state source is the task-associated counter
  record when present, else the task-local seed 0. No counter record
  family exists on the bus, so the projector always yields the seed 0
  today. Each dispatch performs exactly one expansion, so the emitted
  value equals the projected base. Cross-tick monotonicity awaits a
  frozen counter carrier.
- `__DATE__` / `__TIME__`: replayable values must be read from the
  frozen config record, never from wall-clock time. `CompilerConfig`
  carries no date/time record, so these names always fail as explicit
  `Unsupported` naming PP24.
- Frozen-target predefined macros are read off the projected target
  model (`TargetSpec` profile plus `Dialect`), never off the host. The
  closed table names only frozen T01 facts (triple arch/os, ELF object
  format, LP64 data model, little-endian order) plus `__STDC__` and the
  dialect-derived `__STDC_VERSION__`. Any other name fails as explicit
  `Unsupported` (unknown builtin name).

## 2. Expansion table (frozen)

| Name | Expansion |
|---|---|
| `__FILE__` | Quoted source-name bytes (`"`/`\` escaped, PP13 stringize rule) |
| `__LINE__` | Physical line number (decimal `PpNumber`) |
| `__COUNTER__` | Projected counter base (decimal `PpNumber`; seed 0 today) |
| `__STDC__` | `1` |
| `__STDC_VERSION__` | Dialect spelling (`199901L`/`201112L`/`201710L`/`202311L`; C89 modes have no mapping and fail instead of fabricating) |
| `__aarch64__`, `__linux__`, `__gnu_linux__` | `1` (triple-checked, never host) |
| `__ELF__` | `1` (object-format-checked) |
| `__LP64__`, `_LP64` | `1` (data-model-checked) |
| `__ORDER_LITTLE_ENDIAN__` | `1234` |
| `__ORDER_BIG_ENDIAN__` | `4321` |
| `__BYTE_ORDER__` | `1234` (endianness-checked) |
| `__DATE__`, `__TIME__` | Explicit `Unsupported` (missing frozen config record) |
| anything else | Explicit `Unsupported` (unknown builtin name) |

A model check that stops matching yields `None` (unknown-name failure),
never a fabricated value.

## 3. Mismatch matrix (all typed `Fail`, all naming PP24)

- Non-`Running` dispatch → task-protocol failure.
- Payload not exactly one `PpToken` ref, or a non-`Identifier` name →
  protocol failure (`Task`, 4).
- `__DATE__` / `__TIME__` → `Unsupported` (needs a frozen config
  date/time record the config does not carry).
- Unknown builtin name → `Unsupported` (no frozen expansion).
- `__STDC_VERSION__` under a C89 dialect → `Unsupported` (macro
  undefined; no value fabricated).

## 4. Frozen registration

- `preprocess.macro_builtin` (local 31, first code after `/26`),
  `Frozen`; `pp_builtin_slice()` registry (44 entries, cumulative
  over `pp_variadic_slice()`); stage row (`→ 1`); routed layer 1;
  `PP24_CHIP = 34`; one allowlist row (`PP24_CHIP`, `Pp`, `"tokens"`
  for the synthesized append); same schemas (`pp_macro_slice()`).
- Hash rules: `pp.builtin-file-line`, `pp.builtin-counter`,
  `pp.builtin-target`, `pp.builtin-date-replayable`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| A1-1 | `__FILE__` quoted source-name expansion | file case |
| A1-2 | `__LINE__` physical-line expansion | line case |
| A1-3 | `__COUNTER__` seed-0 replayable expansion | counter case |
| A1-4 | Frozen-target predefined macros | target case |
| A1-5 | `__DATE__`/`__TIME__` explicit `Unsupported` | date-time case |
| A1-6 | Unknown names explicit `Unsupported` | unknown case |
| A1-7 | Registration freeze (kind/registry/stage/layer/chip/allowlist) | freeze + gates green |
| A1-8 | Snapshot replay determinism | determinism case |

## 6. Execution record

Delivered as a single untracked chip file (`pp_builtin.rs`) against
the `/26` tree; integrated by the T01 integrator: froze local 31 /
`ChipId(34)` / `preprocess.macro_builtin` into `task.rs`, `PP24_CHIP`
+ `is_pp_builtin_slice_kind` + allowlist row + stage row into
`manifest.rs`, four rule ids into `contract.rs`, bumped to
`t01-c01-c06/27` with recomputed hash (`1c4c6547…9708faa`), wired
`preprocess/mod.rs` + `chips/mod.rs`, and re-pointed the chip's
proposed consts at the frozen canonicals (no logic change). Verified
by `compiler/tests/c27_builtin.rs` (9 tests: freeze, `__FILE__`,
`__LINE__`, `__COUNTER__` seed, target macros, `__DATE__`/`__TIME__`,
unknown, gates, replay determinism). No integration corrections to
chip logic were needed.

## 7. Explicitly deferred (all loud, never silent)

- PP23 logical line mapping (physical-line fallback stays until the
  logical-location carrier lands).
- Cross-tick `__COUNTER__` monotonicity (needs a frozen counter
  carrier; every dispatch replays the seed until then).
- `__DATE__`/`__TIME__` replay (needs a frozen config date/time
  record; wall-clock sampling is forbidden).
- `candidate` builtin orchestration and multi-pass PP looping
  (control-loop slice scope).
- PP23 line, PP25 pragma (later slices; files left untracked by this
  slice).
