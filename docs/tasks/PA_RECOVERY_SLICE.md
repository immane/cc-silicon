# Wave 3 Slice 11: PA Recovery Freeze (`/36`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/36` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/35` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/36` delta), [T05](T05_PARSE_CHIPS.md) PA14/PA38 rows, [PA slice](PA_SLICE.md), [PA decl slice](PA_DECL_SLICE.md), [PA expr slice](PA_EXPR_SLICE.md) |
| Non-goal | A `next_cursor` result carrier (OB-30 open), committed DeclNode links, T06 declare fan-out (wiring layer owns it), child-task await cleanup, scope balancing (T06), non-M1 declarators |

## 1. What this slice is

Two Ack-only workers closing the M1 declaration tail and the
single-fault resumption path:

- `PaPodChip` (PA14) accepts exactly the M1 declaration finish
  `main(void);` (identifier `main`, exactly the `(void)` suffix, then
  `;` with the terminator scan at paren depth 0) and certifies the
  point-of-declaration registration (name, spelling, declarator span)
  for the wiring layer, which owns the T06 declare fan-out and the
  PA15 initializer ordering. Comma lists and initializers fail
  `Unsupported` (PA15 territory, where the POD-before-initializer
  ordering invariant applies); a missing `;` is the
  unterminated-declaration defect (`Task` channel, never an implicit
  semicolon).
- `PaRecoveryChip` (PA38) synchronizes the fault suffix (starting at
  the fault cursor) to an explicit delimiter with grounded
  `(paren, bracket, brace)` depth counters: `;` at zero depth
  (consumed), `)` / `}` at zero depth (not consumed — the owning
  frame keeps its closer), `{` at zero depth (not consumed — refuses
  to enter a following function body, the DEFECT-4 anti-swallow
  stop), and committed `Eof`. Every `Ok` path satisfies the finite
  advance guarantee `0 < index <= tokens.len()`; a fault already at
  `Eof`, an empty window, or a window with no sync token fails loudly
  instead of spinning.

Both are pure-plus-`Ack`: they validate committed records and append
nothing. No cursor carrier is frozen (OB-30 stays open); no node
links are committed; no child task is enqueued or awaited (dropping
the failed frame's continuation and balancing block scopes stays
wiring-layer / T06-integration work).

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| `main(void);` (5 tokens) | `Ack` + certified POD registration (`main`, span 0–3) |
| `main(void)` + `}` (missing `;`) | `Fail` (`Task` channel defect, never an implicit semicolon) |
| `main(void),` / interior `=` | `Fail` (`Unsupported`; comma lists and initializers deferred) |
| `bad + ; …` | `Ack`; sync `;` consumed (`index = pos + 1`) |
| `… )` / `… }` at zero depth | `Ack`; stop without consuming (owner keeps the closer) |
| `… {` at zero depth | `Ack`; stop without consuming (anti-swallow, DEFECT-4) |
| `… EOF` | `Ack`; resume at end of input |
| Fault exactly on a closer / open-brace | `Ack` with `index = 1` (finite advance, never zero) |
| Fault already at `Eof`, empty window, no sync token | `Fail` (loud, never a spin or a guess) |
| Nested `( 1 ; 2 ) ;`, stray `]`, spelling-less punctuator | Inner `;` suppressed / `]` ignored / never syncs (spelling-grounded) |
| Non-running task, missing token record | `Fail` (`Task` channel protocol fault) |
| Wrong stage layer, pre-`/36` registry | Driver/gate `Fail` (never silent) |

## 3. ID arbitration (frozen)

The two delivered files both claimed `PARSE` local 25 and `ChipId(50)`
as candidates (each authored as the next-free value against the `/35`
tree, unaware of the other). The integrator verified the `/35` head
(`PARSE_UNARY` = local 24, `PA20_CHIP = ChipId(49)`) and split the
claims linearly, with no logic change beyond kind/chip-id/test-path
repointing:

- Kinds: `parse.decl_finish` (25, PA14 pod finish),
  `parse.recovery` (26, PA38 recovery) — `PARSE` owners start new
  codes at local 27.
- Chips: `PA14_CHIP = 50` (pod), `PA38_CHIP = 51` (recovery).
- The files' candidate consts (`PA14_CANDIDATE_LOCAL` /
  `PA14_CANDIDATE_CHIP` / `pa14_task_kind`, `PA38_LOCAL` /
  `PA38_CANDIDATE_CHIP` / `pa38_task_kind`) are re-pointed at the
  frozen canonicals (`TaskKind::PARSE_DECL_FINISH` /
  `TaskKind::PARSE_RECOVERY` aliases via new `PA14_TASK_KIND` /
  `PA38_TASK_KIND`, manifest chip IDs, test path →
  `c36_recovery.rs`); headers rewritten from CANDIDATE-draft to
  frozen `/36`. No chip logic changed beyond kind/chip-id/test-path
  repointing.

## 4. Frozen registration

- `parse.decl_finish` (local 25), `parse.recovery` (local 26) —
  first codes after `/35`, all `Frozen`; `pa_recovery_slice()`
  registry (63 entries, cumulative over `pa_expr_slice()`); stage
  rows (both `→ 2`); routed layer 2; `PA14_CHIP = 50` + `PA38_CHIP =
  51` (both Ack-only, zero writes, no allowlist rows;
  `STORE_OWNER_ALLOWLIST` stays 32); same schemas (`pa_slice()` —
  every read field is declared since `m1`/`lx`).
- Hash rules: `pa.pod-finish`, `pa.recovery-sync`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| F1-1 | Pod finish (`main(void);` + registration) | pod accept case |
| F1-2 | Missing `;` defect | `Task`-channel case |
| F1-3 | Comma / initializer deferrals | `Unsupported` case |
| F1-4 | Delimiter sync (`;`, `)`, `}`, `{`-stop, EOF) | sync cases |
| F1-5 | Nesting suppression + spelling grounding | nesting case |
| F1-6 | Finite advance (zero-advance shapes, EOF fault) | finite-advance case |
| F1-7 | Registration freeze (kinds/registry/stage/layer/chips/allowlist) | freeze + gates green |
| F1-8 | Bus dispatch (both `Ack`) + replay determinism | bus case |

## 6. Execution record

Delivered as two untracked chip files (`pa_pod.rs`, `pa_recovery.rs`)
against the `/35` tree; integrated by the T01 integrator: verified
the `/35` head (local 24, chip 49), arbitrated the colliding local-25
/ chip-50 claims into locals 25–26 / `ChipId(50–51)` (§3) into
`task.rs` (`pa_recovery_slice()`, 63 entries), `PA14_CHIP` +
`PA38_CHIP` + `is_pa_recovery_slice_kind` + two stage rows + zero
allowlist rows into `manifest.rs`, two rule ids into `contract.rs`,
bumped to `t01-c01-c06/36` with recomputed hash
(`c8d2135b…766e6`), wired `parse/mod.rs` + `chips/mod.rs` (pure cores
re-exported by the LX-helper precedent), and re-pointed the chips'
candidate consts at the frozen canonicals (headers rewritten from
CANDIDATE-draft to frozen `/36`). No chip logic changed beyond
kind/chip-id/test-path repointing. Verified by
`compiler/tests/c36_recovery.rs` (11 tests: freeze, pod accept, pod
defects × 2, sync × 2, nesting, finite advance, determinism, bus
dispatch + determinism, stage-layer gates). Full §5 suite green at
commit. Superseded pins updated by the integrator (never the chip
owner, never silent): `freeze.rs`, `c08_gate1` (version, 63 stages, 32
allowlist rows), `c04_manifest` (no-row comment), `c20_ppscan` +
`h04_candidate` version markers, `c34_parse` + `c35_expr` (`PARSE`
head now local 27), both READMEs, `T01_COMPILER_CONTRACT.md` §7.1,
`CHIP_PLAN.md`, `context.md`.

## 7. Explicitly deferred (all loud, never silent)

- `next_cursor` result carrier (OB-30; caller holds the cursor).
- Committed DeclNode links and the T06 `symbol_type.declare` fan-out
  (wiring layer; a second `parse.nodes` writer needs an allowlist
  row).
- Child-task fan-out, await-all nesting, and failed-frame
  continuation cleanup (PA01 wiring; DEFECT-5 stays open).
- Scope balancing on recovery (`symbol_type.scope_exit`; T06).
- Multi-declarator lists, initializers (PA15/T08), pointers,
  parenthesized/array declarators, non-`main` names, non-`(void)`
  parameters (all `Unsupported`, in the denominator).
- Multi-fault windows (one task recovers one fault),
  heuristic/keyword sync (delimiter-grounded only), recovery across
  the caller-provided window edge (caller-bounded; DEFECT-4 stays
  open).
- Parse-depth accounting (`ParseDepthExceeded`, T05 item D) as a
  future integration check.
- The PA10–PA13/PA15/PA17–PA19/PA21/PA23–PA37 remainder (unchanged).
