# Wave 3 Slice 5: PP Expansion-Map Freeze (`/30`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/30` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/29` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/30` delta), [T03](T03_PREPROCESS_CHIPS.md) PP27 row, [pragma slice](PP_PRAGMA_SLICE.md) |
| Non-goal | `OriginChain` carrier on the bus (the chains are returned to the wiring layer; no store field lands), PP28 preprocessed emit, once-consultation in include-resolve |

## 1. What this slice is

One new worker closing expansion-provenance USE: `PpExpandMapChip`
(PP27) rebuilds the per-token origin chain (`TokenOrigins ->
OriginChain`, T03:35). The payload carries pp-token refs in payload
order; for each ref the chip follows `span.expansion` through the
committed `parent` links, resolving every `spelling`/`expanded` span on
the way. An unexpanded token carries no frames — its origin is its own
span. Success completes `Ack` with no bus writes (the wiring layer
consumes the chains via `origin_chain` / `origin_root`); every failure
path is a typed `Fail`. Dangling links are never silently truncated.

Annotator-vs-query decision (deliberate, not an oversight): PP12
stamps every synthesized token with the invocation name token's span,
so per-token provenance is a query over already-committed
spans/expansions, not a new append. An annotator chip would need a
frozen `OriginChain` carrier plus a consumer task; until the integrator
freezes that carrier, persisting chains here would invent schema.
Promotion criteria (all required): a frozen chain carrier, an
independent consumer task for the chains (e.g. PP28 emit or LX16
location), and integrator-owned kind/chip registration.

The five T03:35 query cases map to structural chain fields (no
operator tags are invented — `ExpansionRecord` carries none, so
classification beyond structure would be fabrication):

- `#` stringify-raw: the frame's `spelling` span names the raw argument
  bytes; the stringized token's own span is the invocation name span.
- `##` paste: the frame's `expanded` span names the paste product; both
  operand spans stay reachable through the same frame.
- prescan-vs-raw: `spelling` (raw form) and `expanded` (prescanned form)
  are kept side by side per frame instead of collapsing to one.
- blue-paint rescan: `parent` linkage plus per-frame
  `ordinal`/`depth` recover the rescan nesting without re-running
  substitution.
- nested include origins: every frame resolves to a `SpanRecord`,
  whose `source` identifies the physical file at that nesting level.

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| Unexpanded tokens (no expansion links) | `Ack`; empty frames, root is own span |
| Single-frame chain | `Ack`; frame carries expansion/spelling/expanded/ordinal/depth 0 |
| `#` stringify shape | `Ack`; `spelling` is the raw side, token span is the invocation span |
| `##` paste shape | `Ack`; `expanded` is the paste product, operands reachable via the frame |
| Prescan-vs-raw shape | `Ack`; raw and prescanned sides kept distinct |
| Two-level rescan nesting | `Ack`; depths 0/1, ordinals preserved, root is outermost `expanded` |
| Nested include origins | `Ack`; frames resolve across both sources |
| Dangling expansion link | Typed `Invalid` `Fail` (dangling, never silent truncation) |
| Repeated expansion ID on one walk | Typed `Invalid` `Fail` (cycle) |
| Non-`Running` dispatch, empty/non-pp-token payload, wrong kind | Protocol-fault `Fail` (wrong dispatch) |

A repeated expansion ID on one walk fails `Invalid` (cycle), and the
walk additionally stops loudly past the committed expansion count, so a
corrupt parent graph can neither loop forever nor spin silently.

## 3. Mismatch matrix (all typed `Fail`)

- Non-`Running` dispatch → task-protocol failure.
- Payload empty or not pp-token refs → protocol failure.
- Wrong task kind → protocol failure.
- Missing pp-token / span / source, unprojected span, dangling
  expansion or parent, expansion cycle, over-long walk → `Invalid`
  naming the violated rule.

## 4. Frozen registration

- `preprocess.expand_map` (local 34, first code after `/29`),
  `Frozen`; `pp_expand_map_slice()` registry (47 entries, cumulative
  over `pp_pragma_slice()`); stage row (`→ 1`); routed layer 1;
  `PP27_CHIP = 37`; no allowlist row (Ack-only, read-only — verified
  against the manifest's empty write set); same schemas
  (`pp_macro_slice()`).
- Hash rules: `pp.expand-origin-chain`, `pp.origin-paste-prescan`,
  `pp.origin-blue-paint`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| A1-1 | Basic single-frame chain (+ unexpanded empty frames) | basic case |
| A1-2 | `#` stringify reads the raw `spelling` side | stringify case |
| A1-3 | `##` paste product on the `expanded` side | paste case |
| A1-4 | Prescan-vs-raw kept side by side | prescan case |
| A1-5 | Blue-paint rescan nesting via `parent` links | blue-paint case |
| A1-6 | Nested include origins across sources | include case |
| A1-7 | Dangling expansion fails typed | dangling case |
| A1-8 | Registration freeze (kind/registry/stage/layer/chip, read-only) | freeze + gates green |
| A1-9 | Bus dispatch (`Ack`, no writes) | bus case |

## 6. Execution record

Delivered as a single untracked chip file (`pp_expand_map.rs`)
against the `/29` tree; integrated by the T01 integrator: froze local
34 / `ChipId(37)` / `preprocess.expand_map` into `task.rs`,
`PP27_CHIP` + `is_pp_expand_map_slice_kind` + stage row into
`manifest.rs` (no allowlist row: the manifest declares no writes),
three rule ids into `contract.rs`, bumped to `t01-c01-c06/30` with
recomputed hash (`76bf628e…f2f3592`), wired `preprocess/mod.rs` +
`chips/mod.rs`, and re-pointed the chip's proposed consts at the frozen
canonicals (`PP27_LOCAL` + `ChipId(37)` + kind-fallback fn →
`PP27_TASK_KIND` alias, test path → `c30_expand_map.rs`; no logic
change). Verified by `compiler/tests/c30_expand_map.rs` (10 tests:
freeze, basic chain, stringify, paste, prescan, blue-paint, include
origin, dangling, bus dispatch, gates). No integration corrections to
chip logic were needed.

## 7. Explicitly deferred (all loud, never silent)

- `OriginChain` carrier on the bus (PP28 emit / LX16 location
  consumption waits on the carrier freeze; PP12 keeps stamping the
  invocation name span).
- PP28 preprocessed emit (later slice).
- PP05 `pragma` fan-out wiring (control-loop slice scope).
