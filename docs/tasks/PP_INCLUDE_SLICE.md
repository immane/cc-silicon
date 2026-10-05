# Wave 2 Slice 16: PP Include Freeze (`/25`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/25` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/24` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/25` delta), [T03](T03_PREPROCESS_CHIPS.md) PP17/PP18 rows |
| Non-goal | Async host fetch (AwaitHost/CT02, T02), fixpoint multi-level nesting (control loops), `#pragma once`, `include_next`, config search paths, `candidate` include orchestration |

## 1. What this slice is

Two new workers establishing single-pass inclusion: `PpIncludeResolveChip
` (PP17) resolves a header name to a committed source (pure lookup, no
I/O — chips stay host-passive; missing files fail `not-loaded` for the
host loop to fetch, T02 owns the async protocol); `PpIncludeEnterChip`
(PP18) splices one level of header tokens into the stream. Single-pass
by construction (no fixpoint → no infinite regress); nested `#include`
lines survive as tokens for the control-loop slice. No new record
families, no schema change.

## 2. Path policy (frozen, minimal)

Resolution scans committed sources in ascending-ID order, comparing the
header spelling (delimiters stripped: `<...>` or `"..."`) against source
names from the intern table: exact full-name match first, else basename
suffix match (bytes after the last `/` in each). First match wins;
multiple basename matches → `Fail` ambiguous (lists the count, never
guesses). Quote vs angle behave IDENTICALLY now (divergence with config
search paths is recorded future work). `include_next` → explicit
`Unsupported`. Empty header name → malformed `Fail`. Macro-generated
header names (non-literal tokens) → explicit `Unsupported` (PP09/PP17
expansion interplay deferred).

## 3. PP17 rules (pure resolver)

- Payload: exactly one `HeaderName` pp-token ref (the header to resolve).
  Anything else → protocol fault. Dangling refs → typed errors.
- Output: `Complete(Record(Source))` of the resolved source. Failures:
  not-loaded (`header \`X\` is not loaded (N sources searched)`),
  ambiguous (`ambiguous header \`X\`: N matches`), malformed/unsupported
  per §2. Never appends, never enqueues.
- Reads: Tasks `active.*`, Pp `tokens`, Sources `bytes`, Names `entries`.
  No writes.

## 4. PP18 rules (single-pass stitch)

- Payload: full-stream pp-token refs (directive lines included, as PP09
  sees them). Finds every `#include` line itself (walk-back + name
  check, chip-local copy): each must carry exactly one `HeaderName`
  token right after `include` (else malformed/unsupported `Fail` —
  macro-generated names hit the §2 rule).
- For each `#include` line: resolve via the §2 policy over committed
  sources (same lookup, chip-local copy). The header's SCANNED tokens are
  then located WITHOUT payload framing ambiguity: PP18 scans the
  committed pp-token arena for tokens whose span source equals the
  resolved header source, in ascending-ID order (spans carry source IDs
  — the linkage is exact). Payload layout (frozen): all stream refs
  (directive lines included) followed by exactly one trailing
  `RecordRef::Source` (the resolved header); anything else is a protocol
  fault.
- Stitch: replace the `#include` line's tokens with the header token
  refs (header's own EOF excluded — the outer stream keeps its single
  EOF; nested `#include` lines inside the header content pass through
  verbatim for the control-loop slice). Other lines (directives incl.)
  verbatim. Complete `Records(stitched)`.
- Unresolvable header → `Fail` (same §2 taxonomy). Missing header-token
  scan (source committed but never scanned) → explicit `Fail`
  (`header tokens not scanned; scan the header source first`) — the host
  loop orders PP04 before PP18.
- Reads: Tasks `active.*`, Pp `tokens`, Sources `bytes`+`spans`, Names
  `entries`. No writes.

## 5. Frozen registration

- `preprocess.include_resolve` (local 28) + `preprocess.include_enter`
  (29), `Frozen`; `pp_include_slice()` registry (42 entries, cumulative
  over `pp_expand_slice()`); stage rows (`→ 1` both); routed layers 1;
  `PP17_CHIP = 31`, `PP18_CHIP = 32`; no writes so no allowlist rows
  (wave-gated via `is_pp_include_slice_kind`). Same schemas.
- Hash rules: `pp.include-path-policy`, `pp.include-single-pass-stitch`.

## 6. Freeze items

| # | Item | Acceptance |
|---|---|---|
| A1-1 | Path policy §2 (exact/basename/ambiguous/empty/include_next) | policy matrix |
| A1-2 | PP17 resolve/not-loaded/ambiguous + malformed payloads | resolve cases |
| A1-3 | PP18 single-pass stitch (directive replaced, rest verbatim, EOF single, nested survives) | stitch shape cases |
| A1-4 | Missing-scan and macro-name failures | ordering cases |
| A1-5 | Registration freeze (kinds/registry/stage/layer/chips, gates) | freeze gates green |
| A1-6 | Worker template ×2 (pure computes, lint-clean, ZST, stage/layer) | `c25_include` green |

## 7. Execution record

Implemented by two parallel chip owners against this freeze
(`pp_resolve.rs`, `pp_enter.rs`), integrated by the T01 integrator.
Verified by `compiler/tests/c25_include.rs` (8 tests: kind/stage/
registry freeze, resolve policy matrix incl. angle brackets,
ambiguity, protocol faults, single-pass stitch shape, nested survival,
missing-scan ordering failure, gates, replay determinism). No
integration corrections were needed; both deliveries arrived lint-clean
with byte-identical wiring reverts.

## 8. Explicitly deferred (all loud, never silent)

- Async fetch (AwaitHost + CT02 consume) and fixpoint nesting (T02
  control loops); host pre-loading + retry is the interim discipline.
- `#pragma once` (PP25); `include_next` (T12); config search paths +
  quote/angle divergence (H04/H01 substrate work).
- `candidate` include orchestration (single-file + leaf headers only
  after this slice; multi-level needs the loop slice).
- Header content downstream evaluation in acceptance (conditionals in
  stitched headers need a second PP19 pass — control-loop scope).
