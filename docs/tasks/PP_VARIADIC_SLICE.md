# Wave 3 Slice 1: PP Variadic Freeze (`/26`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/26` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/25` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/26` delta), [T03](T03_PREPROCESS_CHIPS.md) PP16 row, [macro-define slice](PP_MACRO_DEFINE_SLICE.md), [expansion slice](PP_EXPAND_SLICE.md) |
| Non-goal | GNU `, ## __VA_ARGS__` comma swallowing (dialect-gated follow-up), PP09/PP16 rescan coordination for nested variadic calls inside non-variadic arguments, `-D` (H04), `candidate` variadic orchestration |

## 1. What this slice is

One new worker closing variadic-macro USE (definitions were recorded
since `/23` but every variadic use failed as explicit `Unsupported`):
`PpVariadicChip` (PP16) handles the `MacroRecord::variadic`
definitions that PP09/PP12 defer. Two payload shapes share one kind:

- Stream mode (PP09 extension): payload = all active pp-token refs.
  Scans like `pp_invoke.rs` (directive lines pass through verbatim via
  the `/21` raw walk-back, nested spans never double-dispatched), but
  only variadic-definition invocations fan out — one single-mode child
  per invocation behind a single `AwaitChildren` — and stitch on resume.
- Single mode (PP12 counterpart): payload = `[Macro(def)]` +
  invocation refs. Substitutes one variadic invocation and completes
  the expansion refs.

Frozen-join semantics (same as PP05/PP09): resume runs only when every
awaited child is terminal; the first `Failed` child fails this task
reusing that child's diagnostic (no aggregate minted); stitching runs
only in the all-`Completed` case.

## 2. Substitution rules (frozen)

- Fixed/variadic parameters: `def.params` entries spelling
  `__VA_ARGS__` alias the variadic tail; every other param is fixed and
  the call must supply at least that many arguments. Extra arguments
  form the tail in order (possibly empty).
- `__VA_OPT__` policy (C23 standard): `__VA_OPT__(content)` expands to
  `content` iff the variadic tail holds at least one preprocessing
  token, else to nothing. Parameters inside the content substitute like
  the rest; nesting recurses. An empty tail is LEGAL: `f("x")` on
  `#define f(fmt, ...)` binds an empty `__VA_ARGS__` (placemarker) —
  never a failure.
- `#`/`##` follow the frozen PP12 rules verbatim (prescan, exact
  stringize/paste, blue-paint rescan with a macro-count+2 breaker).
  GNU `, ## __VA_ARGS__` comma swallowing is NOT implemented: a
  retained comma survives an empty tail.
- Fail-closed edges (all typed `Fail`s naming PP16): `# __VA_OPT__`
  and `##` adjacent to `__VA_OPT__` are constraint failures; bare
  `__VA_OPT__` without `(` is a constraint failure; unbalanced
  `__VA_OPT__(` is malformed.

## 3. Mismatch matrix (all typed `Fail`, all naming PP16)

- Single mode with a non-variadic def → `Unsupported` (PP12 owns
  non-variadic definitions; mirrors PP12's variadic-`Unsupported`).
- Stream mode with extra or missing arguments on a non-variadic def
  (variadic-shaped use) → malformed (`Task`, 4).
- Function-like variadic name without span-adjacent `(` → plain
  identifier (PP06 whitespace-sensitivity rule).
- Unknown identifiers and tombstoned/undefined macros → plain tokens.
- Nested variadic invocations inside a NON-variadic invocation's
  argument list stay PP09/PP12-owned (explicit `Unsupported` until a
  PP09/PP16 rescan coordinator lands); the single-mode engine itself
  expands nested macros of either flavor internally.
- Non-variadic definitions with matching arity pass through verbatim
  (PP09 owns them).

## 4. Frozen registration

- `preprocess.variadic_macro` (local 30, first code after `/25`),
  `Frozen`; `pp_variadic_slice()` registry (43 entries, cumulative
  over `pp_include_slice()`); stage row (`→ 1`); routed layer 1;
  `PP16_CHIP = 33`; one allowlist row (`PP16_CHIP`, `Pp`, `"tokens"`
  for single-mode synthesized appends; stream stitch reuses committed
  IDs with no appends); same schemas (`pp_macro_slice()`).
- Hash rules: `pp.variadic-collect`, `pp.va-opt-policy`,
  `pp.variadic-arity`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| A1-1 | Stream fan-out + frozen-join stitch | basic + determinism cases |
| A1-2 | `...` collection incl. empty-tail legality | basic + empty cases |
| A1-3 | `__VA_OPT__` empty/non-empty policy | va-opt case |
| A1-4 | `#`/`##` inside variadic substitution | stringize/paste case |
| A1-5 | Blue-paint termination (self-reference) | blue-paint case |
| A1-6 | Mismatch matrix (stream arity, single non-variadic) | misuse case |
| A1-7 | Non-variadic passthrough (verbatim, no appends) | passthrough case |
| A1-8 | Registration freeze (kind/registry/stage/layer/chip/allowlist) | freeze + gates green |

## 6. Execution record

Delivered as a single untracked chip file (`pp_variadic.rs`) against
the `/25` tree; integrated by the T01 integrator: froze local 30 /
`ChipId(33)` / `preprocess.variadic_macro` into `task.rs`, `PP16_CHIP`
+ `is_pp_variadic_slice_kind` + allowlist row + stage row into
`manifest.rs`, three rule ids into `contract.rs`, bumped to
`t01-c01-c06/26` with recomputed hash (`59bdf0f5…9031a006`), wired
`preprocess/mod.rs` + `chips/mod.rs`, and re-pointed the chip's
proposed consts at the frozen canonicals (no logic change). Verified
by `compiler/tests/c26_variadic.rs` (10 tests: freeze, basic tail,
empty tail, VA_OPT both polarities, `#`/`##`, blue-paint, stream +
single misuse, passthrough, gates, replay determinism). One
integration correction during verification: the blue-paint test's `R`
count (2 → 3 — the verbatim replacement-line `R` also survives in the
stream); chip logic untouched.

## 7. Explicitly deferred (all loud, never silent)

- GNU `, ## __VA_ARGS__` comma swallowing (needs a dialect pin;
  same reason PP12 has no dialect reads).
- PP09/PP16 rescan coordination for nested variadic calls inside
  non-variadic argument lists (explicit `Unsupported` until then).
- `candidate` variadic orchestration and multi-pass PP looping
  (control-loop slice scope).
- PP24 builtins, PP23 line, PP25 pragma (later slices; files left
  untracked by this slice).
