# Wave 3 Slice 12: T08 Const-Branch Freeze (`/37`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/37` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/36` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/37` delta), [T08](../tasks/T08_CONST_LAYOUT_INIT.md) CL04/CL07 rows, Gate 1 const-fold subset (`fold.rs` gates, copied chip-local) |
| Non-goal | A branch-result commit carrier (committing branch values stays future work), committed node links, child-task fan-out, non-M1 literal shapes |

## 1. What this slice is

Four Ack-only workers certifying the M1 selected-branch and
static-assert path:

- `BranchAndChip` (CL04) evaluates the condition plus ONLY the
  short-circuit-selected `&&` branch: `2 && 3` acks canonical `1`,
  `0 && <bad>` acks canonical `0` without ever gating the rhs.
- `BranchOrChip` (CL04) mirrors for `||`: `3 || <bad>` acks `1`,
  `0 || 3` acks `1`, `0 || 0` acks `0`.
- `BranchCondChip` (CL04) passes the selected `?:` magnitude through
  verbatim: `1 ? 2 : 3` certifies `2`, `0 ? <bad> : 3` certifies `3`.
- `StaticAssertChip` (CL07) checks one asserted ICE: nonzero passes
  (`Ack`), zero fails as a failed assertion, and a payload that does
  not name a committed literal fails as `NotConstantExpression`
  (never ICE, never a silent pass).

The unselected operand is never subset-checked, never budget-checked,
and may even dangle — mirroring the `0&&1/0`, `1?3:1/0` acceptance.
Logical results are canonical C `0`/`1` magnitudes. Every selected
operand passes the M1 exercised-subset gate (decimal `Integer`, no
suffix, `Int` candidate — anything else is explicit `Unsupported`)
and the configured bit-budget gate (over-budget is the typed
`ConstOverflow` chip diagnostic via `Fail`).

All four are pure-plus-`Ack`: they validate committed literals and
append nothing. No branch-result carrier is frozen; no node links are
committed; no child task is enqueued or awaited.

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| `2 && 3` | `Ack` (canonical `1`) |
| `0 && 3`, `0 && <bad>`, `0 && <dangling>` | `Ack` (canonical `0`; rhs never gated) |
| `3 \|\| <bad>` | `Ack` (canonical `1`; rhs never gated) |
| `0 \|\| 3` / `0 \|\| 0` | `Ack` (canonical `1` / `0`) |
| `1 ? 2 : 3` / `0 ? 2 : 3` | `Ack` (passthrough `2` / `3`) |
| `0 ? <bad> : 3` | `Ack` (`then` never gated) |
| assert `2` | `Ack` |
| assert `0` | `Fail` (`Task` channel, failed assertion) |
| assert non-literal payload | `Fail` (`NotConstantExpression`, never ICE) |
| Selected out-of-subset literal (any shell) | `Fail` (`Unsupported`) |
| Over-budget condition or result (any shell) | `Fail` (`ConstOverflow`) |
| Dangling SELECTED operand | `Fail` (`Task` channel missing) |
| Non-running task, wrong arity, non-literal ref | `Fail` (protocol fault) |
| Wrong stage layer, pre-`/37` registry | Driver/gate `Fail` (never silent) |

## 3. ID arbitration (frozen)

The delivered draft (`fold_branch.rs`) claimed
`TaskKind::CONSTANT_CONST_FOLD` descriptively for all four shells
(which MUST NOT register alongside `FoldChip` — duplicate kind
claim) with draft chip IDs 52–55. The integrator verified the `/36`
head (no `CONSTANT` local past 16, `PA38_CHIP = ChipId(51)`) and froze
the draft IDs linearly with no logic change beyond
kind/chip-id/test-path repointing:

- Kinds: `const_branch_and` (17, CL04 `&&`),
  `const_branch_or` (18, CL04 `||`), `const_branch_cond` (19, CL04
  `?:`), `const_static_assert` (20, CL07 assert) —
  `CONSTANT_LAYOUT_INIT` owners start new codes at local 21.
- Chips: `CL04_AND_CHIP = 52`, `CL04_OR_CHIP = 53`,
  `CL04_COND_CHIP = 54`, `CL07_ASSERT_CHIP = 55`.
- The file's draft consts (`BRANCH_*_CHIP`, `draft_manifest`,
  `draft_reads`) are re-pointed at the frozen canonicals
  (`CL04_*_TASK_KIND` / `CL07_ASSERT_TASK_KIND` aliases, manifest chip
  IDs, test path → `c37_const_branch.rs`); header rewritten from
  UNREGISTERED-draft to frozen `/37`. No chip logic changed beyond
  kind/chip-id/test-path repointing.

## 4. Frozen registration

- `const_branch_and` (local 17), `const_branch_or` (18),
  `const_branch_cond` (19), `const_static_assert` (20) — first codes
  after Gate 1 `const_fold` (local 16), all `Frozen`;
  `const_branch_slice()` registry (67 entries, cumulative over
  `pa_recovery_slice()`); stage rows (all `→ 2`); routed layer 2;
  `CL04_AND_CHIP = 52` + `CL04_OR_CHIP = 53` + `CL04_COND_CHIP = 54` +
  `CL07_ASSERT_CHIP = 55` (all Ack-only, zero writes, no allowlist
  rows; `STORE_OWNER_ALLOWLIST` stays 32); same schemas (`pa_slice()`
  — every read field is declared since `m1`/`foundation`).
- Hash rules: `const.branch-selected`, `const.static-assert`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| F1-1 | `&&` selected-branch certification (canonical `0`/`1`) | `and` case |
| F1-2 | `\|\|` selected-branch certification (canonical `0`/`1`) | `or` case |
| F1-3 | `?:` magnitude passthrough | `cond` case |
| F1-4 | Short-circuit skips (bad + dangling unselected operands) | short-circuit case |
| F1-5 | Assert pass / fail / non-ICE | assert case |
| F1-6 | M1 subset gate on selected operands | subset case |
| F1-7 | Bit-budget overflow gate | overflow case |
| F1-8 | Registration freeze (kinds/registry/stage/layer/chips/allowlist) | freeze + gates green |
| F1-9 | Bus dispatch (all four `Ack`) + replay determinism | bus case |

## 6. Execution record

Delivered as one untracked chip file (`fold_branch.rs`) against the
`/36` tree; integrated by the T01 integrator: verified the `/36` head
(no `CONSTANT` local past 16, chip 51), froze locals 17–20 /
`ChipId(52–55)` (§3) into `task.rs` (`const_branch_slice()`, 67
entries), `CL04_AND_CHIP` + `CL04_OR_CHIP` + `CL04_COND_CHIP` +
`CL07_ASSERT_CHIP` + `is_const_branch_slice_kind` + four stage rows +
zero allowlist rows into `manifest.rs`, two rule ids into
`contract.rs`, bumped to `t01-c01-c06/37` with recomputed hash
(`59721bd8…2fc8c`), wired `constant_layout_init/mod.rs` +
`chips/mod.rs` (pure cores re-exported by the LX-helper precedent),
and re-pointed the draft's kind/chip-id/test-path at the frozen
canonicals (header rewritten from UNREGISTERED-draft to frozen
`/37`). No chip logic changed beyond kind/chip-id/test-path
repointing. Verified by `compiler/tests/c37_const_branch.rs` (10
tests: freeze, `&&`, `||`, `?:`, short-circuit, assert, subset,
overflow, bus dispatch + determinism, stage-layer gates). Full §5
suite green at commit. Superseded pins updated by the integrator
(never the chip owner, never silent): `freeze.rs`, `c08_gate1`
(version, 67 stages, 32 allowlist rows, new rule/marker pins),
`c04_manifest` (no-row comment), `c20_ppscan` + `h04_candidate`
version markers, both READMEs, `T01_COMPILER_CONTRACT.md` §7.1,
`CHIP_PLAN.md`, `context.md`.

## 7. Explicitly deferred (all loud, never silent)

- Branch-result commit carrier (certification only; committing branch
  values awaits the result-carrier freeze).
- Branch operator riding the frozen wire (one fixed `BranchOp` per
  shell until the operator encoding co-freezes).
- Assert-failure numeric code (shares the `Task,3` const-eval failure
  slot; messages distinguish; exact code stays open pending the T01
  code freeze).
- Signed literals, non-decimal radixes, suffixed literals, non-`Int`
  candidates (all `Unsupported`, in the denominator).
- The CL01/CL02/CL03/CL05/CL06/CL08–CL26 remainder (unchanged).
