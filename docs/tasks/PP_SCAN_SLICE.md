# Wave 2 Slice 7: PP Splice/Comment/Scan Freeze (`/16`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-05** as `t01-c01-c06/16` (hash `04d8e4b4…f9f10`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/15` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/16` delta), [T03](T03_PREPROCESS_CHIPS.md) rev-44/45/53/55, [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-PP-01`/`01b`/`01c`/`04` |
| Non-goal | Macro/include/multi-source, `Preprocessed` production (PP28), full scan-state machine, token emission past M1 |

## 1. What this slice is

Three workers closing the source-bytes gap: `PpSpliceChip` (PP02, real
splice with composed maps) → `PpCommentChip` (PP03, M1-scoped comment
removal) → `PpScanChip` (PP04, maximal-munch scan with spans). The chain
runs from seeded source bytes with no seeded PP fixtures, so the M1
frontend is now end-to-end from source bytes to IR. The acceptance
fixture is **`P1-PP-02`**.

## 2. Freeze items

| # | Item | Acceptance |
|---|---|---|
| Q1-1 | Splice: delete every `\`+newline pair; identity fast path; lone trailing backslash ordinary; output-to-input map composed to raw source | identity + real + trailing-backslash unit cases |
| Q1-2 | Comment M1 scope: `//`→one space (newline kept), `/*`→one space (inner newlines kept, no gluing); `"`/`'` inputs explicit `Unsupported`; unterminated `/*` typed failure (M1-NEG-01 carrier shape) | unit + negative cases |
| Q1-3 | Scan maximal munch: identifiers, decimal-led pp-numbers with `e`/`E`/`p`/`P` sign continuation, six M1 punctuators, exactly one zero-width EOF; everything else explicit `Unsupported` | M1 table + `1e+foo` + `+ +` + negatives |
| Q1-4 | Spans remapped to raw source before any draft; EOF span zero-width at the raw end (CRLF-aware) | end-to-end raw-coordinate assertions |
| Q1-5 | `preprocess.splice(17)` / `comment(18)` / `scan(19)`, all stage 1; chips 17–19; four allowlist rows; `pp_slice()` registry (29 entries, cumulative) | kind/stage/allowlist/layer gates green |
| Q1-6 | `Span`/`PpToken` append materialization (1:1 bodies, capacity, predicted PpToken refs; future `Span` refs rejected); map-mandatory kinds only (`Normalized`/`Spliced`/`CommentFree`) | negatives + capacity |
| Q1-7 | Snapshot bodies already typed (no encoding change); `SPAN_RECORD_FIELDS` in the hash | round-trip + replay determinism |
| Q1-8 | Worker template ×3 (narrow projections, pure computes, ZST, stage/layer, lint) | `c16_pp` 8 tests green |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c16_pp.rs`
(8 tests: kind/stage/allowlist freeze, splice/comment/scan unit rules,
source-bytes end-to-end on canonical + CRLF inputs, malformed/dangling
negatives, stage/layer + manifest gates, snapshot replay determinism).

## 4. Explicitly deferred

- Full rev-55 scan-state machine (literal/header-name protection).
- `Preprocessed` artifact production (PP28), macro/include/multi-source.
- Non-M1 punctuators/literals (explicit `Unsupported`, never mis-tokenized).
- PP diagnostic taxonomy (Task-group errors pending the F4 freeze).
