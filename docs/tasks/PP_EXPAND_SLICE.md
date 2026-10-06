# Wave 2 Slice 15: PP Macro-Expansion Freeze (`/24`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/24` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/23` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/24` delta), [T03](T03_PREPROCESS_CHIPS.md) PP09–PP16 rows, [macro-define slice](PP_MACRO_DEFINE_SLICE.md) |
| Non-goal | Variadic invocation (PP16), builtins (PP24), `-D` (H04), placemarker-exact C23 edges beyond the frozen rules, target widths |

## 1. What this slice is

Two new workers establishing non-variadic macro expansion: `PpInvokeChip`
(PP09) fans out one `PP12` child per top-level invocation and stitches the
expanded stream; `PpSubstituteChip` (PP12) substitutes one invocation
(argument prescan, `#`/`##`, recursive rescan with blue paint) and
completes the expansion refs. PP10/PP11/PP13/PP14/PP15 ship as tested
pure helpers inside `pp_substitute.rs` (folded with promotion criteria
in §7). No new record families (verbatim copies reuse IDs; synthesized
tokens append `PpToken`s).

## 2. Pipeline order (frozen)

SCAN → CONDITIONAL → PP06-tasks (caller fans out per `#define` line) →
PP09 (active stream) → DIRECTIVE → LX. PP09 runs on the post-conditional
active stream (directive lines INCLUDED — see §3); PP05 dispatches on
PP09's output; LX consumes caller-supplied refs as before. `#define`
lines still fail at PP05 (no PP28 strip yet — correct fail-closed).

## 3. PP09 rules (fan-out + stitch)

- Payload: all active pp-token refs. Directive lines (first token
  `#`/`%:` + walk-back pass, `/21` predicate chip-local) pass through
  VERBATIM (refs reused, never expanded — replacement lists stay raw).
- Top-level invocation scan in stream order, skipping nested spans once
  consumed: `Identifier` spelling matching a defined non-tombstone
  non-function-like macro → object invocation; `Identifier` matching a
  defined function-like macro immediately followed by span-adjacent `(`
  (same adjacency rule as PP06) → function invocation with balanced
  argument collection (parens depth, top-level commas; empty args
  incl. `f()`/`f(,)` shapes preserved as empty lists).
- Variadic-macro USE (definition has `variadic`) → explicit `Fail`
  Unsupported (PP16). Unknown identifiers → plain tokens, never touched.
- Zero invocations → `Complete(Records)` of ALL input refs, no appends,
  no children (M1 flows through).
- Else: one PP12 child per invocation (owner PP12_CHIP, payload =
  `[Macro(def)]` + invocation pp-token refs in order) + single
  `AwaitChildren` over all of them. Resume follows the frozen join
  (failed child ⇒ router fails this task reusing that diagnostic;
  resume-compute runs only all-`Completed`): stitch in stream order —
  untouched refs reused, each invocation replaced by its child's
  `Records` refs from the committed results — and `Complete(Records)`.

## 4. PP12 rules (substitute + rescan one invocation)

- Payload: `[Macro(def)]` + invocation refs (`[name]` object-like;
  `[name, (, args..., )]` function-like, validated shape else protocol
  fault; def/name mismatch → protocol fault).
- Variadic def → explicit `Fail` Unsupported (never expanded here).
- Argument mapping: params ↔ collected args (count must match exactly,
  else malformed `Fail`; zero-param `f()` takes empty list only).
- Prescan: every argument fully expanded by the internal expander
  EXCEPT arguments that appear as operands of `#`/`##` in the
  replacement (passed raw). Prescan uses the same substitution+rescan
  engine with the current paint set.
- Substitution over the replacement list: parameter occurrence →
  prescanned arg tokens (empty arg contributes NOTHING — placemarker);
  `# param` → stringized token (`StringLiteral`, spellings joined with
  single spaces, `\` and `"` escaped, quotes wrapped; non-parameter
  operand → constraint `Fail`); `A ## B` → pasted token (empty side
  drops to the other side; both empty vanish; concatenated spelling
  rescanned — must yield exactly one pp-token else constraint `Fail`);
  `__VA_ARGS__` with non-variadic def → constraint `Fail`.
- Output tokens: verbatim replacement copies REUSE their committed IDs;
  synthesized tokens (prescanned, pasted, stringized) append new
  `PpToken` records with span = the invocation name token's span
  (per-token provenance deferred to PP27).
- Rescan loop: repeat substitution-rounds over the output while some
  unpainted invocation remains; paint the expanded macro during its own
  round (standard blue paint; nested same-name stays identifier).
  Termination: paint grows monotonically per chain (depth ≤ committed
  macro count); circuit breaker at `macros + 2` rounds → loud `Fail`
  (never infinite). Commit capacity preflights total appends as usual.
- Complete `Records(expansion refs)` (possibly empty for empty
  replacement). Appends at most the synthesized tokens (1:1 predicted).

## 5. Frozen registration

- `preprocess.macro_invoke` (local 26) + `preprocess.macro_substitute`
  (27), `Frozen`; `pp_expand_slice()` registry (40 entries, cumulative
  over `pp_macro_slice()`); stage rows (`→ 1` both); routed layers 1;
  `PP09_CHIP = 29`, `PP12_CHIP = 30`; allowlist rows for both writers
  (`Pp`, `tokens`); same schemas.
- Hash rules: `pp.invoke-fanout-stitch`, `pp.substitute-rescan-loop`,
  `pp.stringify-paste-exact`, `pp.blue-paint-guard`.

## 6. Catalog folding (explicit, not silent)

PP10 (arg collect), PP11 (arg expand), PP13 (stringify), PP14 (paste),
PP15 (rescan) ship as tested pure helpers inside `pp_substitute.rs`:
argument processing and rescan never cross a task boundary — they are
subroutines of one substitution decision. Promotion criteria (all
required): an independent consumer task for an intermediate (e.g.
cross-tick rescan state, cached prescans), plus frozen carriers.

## 7. Freeze items

| # | Item | Acceptance |
|---|---|---|
| Z1-1 | Invocation detection (object/function/adjacency/nesting-skip/directive-verbatim) | detection matrix |
| Z1-2 | Argument mapping (counts, empties, prescan-except-`#`/`##`) | mapping cases |
| Z1-3 | `#`/`##` exact (join rule, escapes, placemarkers, invalid-paste fail) | operator cases |
| Z1-4 | Rescan + blue paint + termination (self/mutual recursion, breaker) | recursion cases |
| Z1-5 | Fan-out + stitch incl. zero-invocation passthrough | multi + M1 cases |
| Z1-6 | Variadic-use Unsupported; `__VA_ARGS__`-mismatch Fail | deferred cases |
| Z1-7 | Registration freeze (kinds/registry/stage/layer/chips/allowlist/gates) | freeze gates green |
| Z1-8 | Worker template ×2 (pure computes, lint-clean, ZST, stage/layer) | `c24_expand` green |

## 7. Execution record

Implemented by two parallel chip owners against this freeze
(`pp_invoke.rs`, `pp_substitute.rs`), integrated by the T01 integrator.
Verified by `compiler/tests/c24_expand.rs` (12 tests: kind/stage/
registry/allowlist freeze, object/function/nested expansion,
stringize/paste, blue-paint termination incl. mutual recursion,
variadic-use deferral, arity failure, zero-invocation passthrough,
zero-param call rule, gates, replay determinism). Three integrator-side
corrections: (1) the record gains `function_like` (zero-param
function-like must not expand bare — new hash rule
`pp.macro-function-flag`); (2) PP12 rescan uses real span-adjacency
(stream-adjacency misfires on spaced calls — manifest gains the spans
read); (3) paste validation uses the frozen `scan()` (local classifier
deleted — no drift surface). The `candidate` driver runs a PP09 step
(M1 stitches to itself; `#define` sources still fail at PP05 until the
expansion-runner slice fans out PP06). Deliveries arrived lint-clean
with scratch matrices; one rework round for the three corrections.

## 8. Explicitly deferred

- Variadic invocation/expansion (PP16); builtins (PP24); `-D` (H04).
- `candidate` PP06 fan-out (host per-line tasks; lands with an
  expansion-runner slice).
- Per-token provenance (PP27); cross-tick rescan state.
