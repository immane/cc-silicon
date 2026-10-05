# Wave 3 Slice 10: PA Expr Freeze (`/35`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/35` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/34` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/35` delta), [T05](T05_PARSE_CHIPS.md) PA16/PA20/PA22 rows, [PA slice](PA_SLICE.md), [PA decl slice](PA_DECL_SLICE.md) |
| Non-goal | A `next_cursor` result carrier (OB-30 open), committed node links, child-task fan-out (PA01 wiring owns it), literal value interpretation (T07/T08), non-M1 operators |

## 1. What this slice is

Two Ack-only workers splitting the M1 expression path (`2+3`, `+3` /
`-3`) into individually testable productions:

- `PaBinaryChip` (fused PA16/PA22) dispatches on the task kind. PA16
  accepts exactly one integer-constant token backed by one committed
  literal (one-node `IntLiteral` leaf shape). PA22 accepts exactly
  `<int> + <int>` (kinds `[Integer, Punctuator, Integer]`, middle
  spelling `+` resolved through the committed PP token, both integers
  literal-backed) as a single left-associative precedence-climb step
  at `min_bp = 0` (`binary_precedence` resolves `+` to `(10, 11)`
  only), building the three-node `BinaryAdd → IntLiteral, IntLiteral`
  shape. Literal *values* are never interpreted (T07/T08-owned).
- `PaUnaryChip` (PA20) accepts exactly `+<int>` / `-<int>` (operator
  resolved through the committed PP bytes via `UnaryOp::of_spelling`,
  operand an integer token backed by one committed literal),
  building the two-node `UnaryPlus/Minus → IntLiteral` shape. Sign
  application is T07/T08-owned: this chip certifies syntax only.

Both are pure-plus-`Ack`: they validate committed records and append
nothing. No cursor carrier is frozen (OB-30 stays open); no node
links are committed (committed expression nodes stay PA01-owned until
the full catalog split); no child task is enqueued (composition —
e.g. folding `2 + (+3)` from the PA22 shape and the PA20 `+3` suffix —
is wiring-layer work through child tasks, deferred).

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| `<int>` (one token, one literal) | `Ack` + 1-row `IntLiteral` shape (PA16) |
| `<int> + <int>` (three tokens, two literals) | `Ack` + 3-row `BinaryAdd` shape (PA22) |
| `+<int>` / `-<int>` (two tokens, one literal) | `Ack` + 2-row `UnaryPlus`/`UnaryMinus` shape (PA20) |
| `2 + +3` as one PA22 shape (4 tokens) | `Unsupported` (PA22 arity gate); the `+3` suffix alone is `Ack` (PA20) |
| `2-3`, `2*3`, `2 3`, identifier primary, `*p`, `!x`, non-`int` operand | `Unsupported` (stays in the denominator, never a pass) |
| Missing literal, missing token, non-`Running` | `Fail` (`Task` channel, never a zero or a guess) |
| Wrong stage layer, pre-`/35` registry | Driver/gate `Fail` (never silent) |

## 3. ID arbitration (frozen)

The two delivered files each claimed `PARSE` locals 22/23/24 and
`ChipId(48)`/`ChipId(49)` as candidates (the next-free values at
authoring time against the `/34` tree). The integrator verified the
claims against the `/34` head (`PARSE_RETURN` = local 21, `PA28_CHIP =
ChipId(47)`) with no collision and froze them as-is:

- Kinds: `parse.primary` (22), `parse.binary` (23), `parse.unary`
  (24) — three kinds because the fused binary chip serves two
  productions dispatched on the task kind; `PARSE` owners start new
  codes at local 25.
- Chips: `PA16_CHIP = 48` (fused PA16/22, claims both kinds),
  `PA20_CHIP = 49`.
- The files' candidate consts (`PA16_TASK_KIND` / `PA22_TASK_KIND` /
  `PA20_TASK_KIND` via `TaskKind::new`, `PA_BINARY_CHIP` /
  `PA_UNARY_CHIP`) are re-pointed at the frozen canonicals
  (`TaskKind::PARSE_PRIMARY` / `PARSE_BINARY` / `PARSE_UNARY` aliases,
  manifest chip IDs, test path → `c35_expr.rs`); headers rewritten
  from CANDIDATE-draft to frozen `/35`. No chip logic changed beyond
  kind/chip-id/test-path repointing.

## 4. Frozen registration

- `parse.primary` (local 22), `parse.binary` (23), `parse.unary` (24)
  — first codes after `/34`, all `Frozen`; `pa_expr_slice()` registry
  (61 entries, cumulative over `pa_decl_slice()`); stage rows (all
  `→ 2`); routed layer 2; `PA16_CHIP = 48` + `PA20_CHIP = 49` (both
  Ack-only, zero writes, no allowlist rows; `STORE_OWNER_ALLOWLIST`
  stays 32); same schemas (`pa_slice()` — every read field is
  declared since `m1`/`lx`).
- Hash rules: `pa.primary-int`, `pa.binary-add`,
  `pa.unary-plus-minus`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| F1-1 | Primary (`<int>` + literal) | primary shape + reject cases |
| F1-2 | Binary (`2+3`, precedence table) | binary shape + reject cases |
| F1-3 | Unary (`+3` / `-3`) | unary shapes + reject cases |
| F1-4 | 4-token boundary (`2 + +3`) | boundary case |
| F1-5 | Non-M1 matrix (`*`, `-`, juxtaposition, identifiers) | matrix case |
| F1-6 | Registration freeze (kinds/registry/stage/layer/chips/allowlist) | freeze + gates green |
| F1-7 | Bus dispatch (all `Ack`) + replay determinism | bus case |

## 6. Execution record

Delivered as two untracked chip files (`pa_binary.rs`,
`pa_unary.rs`) against the `/34` tree; integrated by the T01
integrator: verified distinct locals 22–24 / `ChipId(48–49)` (§3) into
`task.rs` (`pa_expr_slice()`, 61 entries), `PA16_CHIP` + `PA20_CHIP` +
`is_pa_expr_slice_kind` + three stage rows + zero allowlist rows into
`manifest.rs`, three rule ids into `contract.rs`, bumped to
`t01-c01-c06/35` with recomputed hash (`88107dd3…7f8983`), wired
`parse/mod.rs` + `chips/mod.rs` (pure cores re-exported by the
LX-helper precedent), and re-pointed the chips' candidate consts at
the frozen canonicals (headers rewritten from CANDIDATE-draft to
frozen `/35`). No chip logic changed beyond kind/chip-id/test-path
repointing. Verified by `compiler/tests/c35_expr.rs` (10 tests:
freeze, primary, binary, unary, 4-token boundary, non-M1 matrix,
protocol faults, determinism, bus dispatch + determinism,
stage-layer gates). Full §5 suite green at commit. Superseded pins
updated by the integrator (never the chip owner, never silent):
`freeze.rs`, `c08_gate1` (version, 61 stages, 32 allowlist rows),
`c04_manifest` (no-row comment), `c20_ppscan` + `h04_candidate`
version markers, `c34_parse` (`PARSE` head now local 25), both
READMEs, `T01_COMPILER_CONTRACT.md` §7.1, `CHIP_PLAN.md`,
`context.md`.

## 7. Explicitly deferred (all loud, never silent)

- `next_cursor` result carrier (OB-30; caller holds the cursor).
- Committed node links and parent assignment (PA01/PA14/PA16
  integration; a second `parse.nodes` writer needs an allowlist row).
- Child-task fan-out and await-all nesting (PA01 wiring; T05 items
  A–C apply when nesting lands; DEFECT-4 stays open).
- Multi-operator precedence chains (`a+b*c`), right-associative
  operators, cast recursion (PA21), `sizeof` type queries (T06).
- Parse-depth accounting (`ParseDepthExceeded`, T05 item D) as a
  future integration check.
- Non-M1 syntax everywhere (stays in the denominator as
  `Unsupported`).
- PA04 typedef disambiguation, PA06 pointers, PA08 arrays, and the
  PA10–PA15/PA17–PA19/PA21/PA23–PA27/PA29–PA31/PA33–PA38 remainder
  (unchanged).
