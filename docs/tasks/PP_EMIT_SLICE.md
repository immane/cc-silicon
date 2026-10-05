# Wave 3 Slice 6: PP Emit Freeze (`/31`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/31` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/30` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/31` delta), [T03](T03_PREPROCESS_CHIPS.md) PP28 row, [expansion-map slice](PP_EXPAND_MAP_SLICE.md) |
| Non-goal | `#line`/GNU markers in the artifact (PP23 owns logical locations at the wiring layer), cross-source provenance beyond the primary map (`/6` design), PP05 `pragma` fan-out wiring |

## 1. What this slice is

One new worker closing the preprocessed-output USE: `PpEmitChip`
(PP28) serializes the final pp-token stream (post-directive,
post-expansion) into one map-mandatory `Preprocessed` artifact whose
bytes re-lex to the same token stream (T03:36). The payload carries
pp-token refs in payload order; the chip drops `Eof` tokens and consumed
directive lines, emits every other token spelling byte-identical joined
with one space (one newline when both neighbors show a line break in
their shared source bytes), and closes with a terminal newline. Success
appends exactly one `ArtifactRecord` and completes
`Record(Artifact)`; every failure path is a typed `Fail`. Malformed
input is never silently accepted.

Emission policy (deliberate, not oversights):

- Directive-strip: a `#`/`%:` punctuator with no expansion that opens a
  physical line is a consumed directive introducer (PP05 rule), so it and
  the rest of its physical line are dropped. A macro-generated `#`
  (expansion present) is never an introducer and is emitted as an
  ordinary token.
- No line markers: PP28 emits NO `#line`/GNU markers. Marker accuracy
  needs the PP23 logical location, which does not travel in the token
  payload; emitting markers from physical spans would disagree with
  `__LINE__`/diagnostics after an include return. The artifact stays
  marker-free so re-lexing cannot resurrect a stale directive.
- No gluing: separation is unconditional (space or newline), so `+ +`
  never becomes `++`, adjacent strings stay two tokens, and newlines are
  layout-only, never semantic.
- Spelling preservation: string/character literals and header names are
  emitted byte-identical (escapes untouched); no unescaping, requoting,
  or normalization is applied at this layer.
- Primary-map-only: the artifact carries the primary source (first
  token's span source) location map only. Foreign-source token bytes are
  emitted verbatim but their map entries collapse zero-width onto the
  running primary boundary, and expansion-reordered primary offsets clamp
  forward so the map stays monotonic. Full cross-source provenance is
  deferred to the `/6` provenance design.

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| Consumed directive lines (`#define`, …) | Stripped; only live tokens emitted |
| `+ +` token pair | `a + + b`, never `++` (no gluing) |
| String literal with escapes | Byte-identical spelling in the output |
| Any well-formed stream | `raw_offsets` satisfies `check_map` |
| `Eof`-only stream | Single terminal newline, map `[0, 0]` |
| Tokens on distinct physical lines | Newline-separated output lines |
| Emitted bytes through `scan` | Same kinds and spellings, same order |
| Non-`Running` dispatch, empty/non-pp-token payload, wrong kind | Protocol-fault `Fail` |
| Missing token/span/source, empty spelling, inverted span, span past source end, over-budget payload | Typed task/config `Fail` |

A map that still fails `check_map` (unreachable by construction) is a
typed internal error; it never lands on the bus.

## 3. Mismatch matrix (all typed `Fail`)

- Non-`Running` dispatch → task-protocol failure.
- Payload empty or not pp-token refs → protocol failure.
- Wrong task kind → protocol failure.
- Missing pp-token / span / source, unprojected span, empty spelling,
  inverted or out-of-range span → `Invalid` naming the violated rule.
- Over-budget payload → config failure.
- `check_map` rejection → internal failure.

## 4. Frozen registration

- `preprocess.emit` (local 35, first code after `/30`),
  `Frozen`; `pp_emit_slice()` registry (48 entries, cumulative over
  `pp_expand_map_slice()`); stage row (`→ 1`); routed layer 1;
  `PP28_CHIP = 38`; one allowlist row (`PP28_CHIP`, `Artifacts`,
  `fragments`, `PREPROCESS_EMIT`) authorizing the single
  `Preprocessed` append; same schemas (`pp_macro_slice()`).
- Hash rules: `pp.emit-directive-strip`, `pp.emit-no-gluing`,
  `pp.emit-map`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| A1-1 | Consumed directive lines stripped | directive-strip case |
| A1-2 | `+ +` never glues to `++` | no-gluing case |
| A1-3 | String spelling preserved byte-identical | string case |
| A1-4 | `raw_offsets` satisfies `check_map` | map case |
| A1-5 | Empty stream closes with one newline | empty case |
| A1-6 | Multiline separation with newlines | multiline case |
| A1-7 | Re-lex roundtrip through `scan` | re-lex case |
| A1-8 | Registration freeze (kind/registry/stage/layer/chip/allowlist) | freeze + gates green |
| A1-9 | Bus dispatch (`Record(Artifact)`, one append, valid map) | bus case |

## 6. Execution record

Delivered as a single untracked chip file (`pp_emit.rs`)
against the `/30` tree; integrated by the T01 integrator: froze local
35 / `ChipId(38)` / `preprocess.emit` into `task.rs`,
`PP28_CHIP` + `is_pp_emit_slice_kind` + stage row + allowlist row into
`manifest.rs`, three rule ids into `contract.rs`, bumped to
`t01-c01-c06/31` with recomputed hash (`ce4dd422…2e2998c1`), wired
`preprocess/mod.rs` + `chips/mod.rs`, and re-pointed the chip's
proposed consts at the frozen canonicals (`PP28_LOCAL` + `ChipId(38)` +
kind-fallback fn → `PP28_TASK_KIND` alias, test path →
`c31_emit.rs`; no logic change). The `/16`-era commit gate that
accepted only `Normalized`/`Spliced`/`CommentFree` artifact appends was
extended to also accept `Preprocessed` (the slice's sole new produced
kind; map-optional kinds stay rejected). Verified by
`compiler/tests/c31_emit.rs` (11 tests: freeze, directive strip,
no-gluing, string preservation, map validity, empty stream, multiline
newlines, re-lex roundtrip, bus dispatch, stage-layer gates, helper).
No integration corrections to chip logic were needed.

## 7. Explicitly deferred (all loud, never silent)

- `#line`/GNU markers in the artifact (PP23 owns the logical location
  at the wiring layer; the artifact stays marker-free).
- Full cross-source provenance (primary-map-only; `/6` design).
- PP05 `pragma` fan-out wiring (control-loop slice scope).
