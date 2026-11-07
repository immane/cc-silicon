# DRAFT ONLY: VF13 Quota-1 Replay Canonical-Projection Freeze (`VF13_FREEZE.md`)

| Field | Value |
|---|---|
| Status | **DRAFT ONLY — NOT FROZEN, NOT ACCEPTED, NO VERSION BUMP** |
| Scope | Docs-only proposal; compiler application (`compiler/`); root framework untouched |
| Authority | None — no integrator freeze, no R1 bump, no owner sign-off |
| Depends on | [T13](T13_VERIFICATION_CHIPS.md) VF13 row + H6-M12, [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) G9/G15, [T02](T02_CONTROL_CHIPS.md) quota-1 projection baseline |
| Non-goal | Any chip code, registration, task kind/stage/phase/interface, record schema change, batch/quota>1 replay, H6-M12 fixture definition |

This file is a **freeze proposal draft** for the quota-1 subset of the
`ReplayCompareChip` (VF13) only. It freezes nothing. The H6-M12
replay-across-dispatch-order fixture stays **required / pending** (see §4).

## 1. What this draft proposes

One read-only replay comparison over a single tick at quota 1
(`max_inflight_per_tick = 1`): run the same initial state, config, and
pins twice, and compare tick-for-tick under the **canonical semantic
comparison projection**, with the scheduler snapshot compared
**separately**. This is the quota-1 subset of the T13 VF13 row; the
batch / quota>1 / cross-dispatch-order extension is explicitly out of
scope (§4).

## 2. Semantic projection vs scheduler snapshot (the distinction this freeze turns on)

The two comparisons are **separate, not conflated**:

- **Canonical semantic comparison projection.** The compared surface is
  semantic records / results / diagnostics plus the semantic trace.
  **Scheduler-only registers are excluded** from this projection. Two
  runs that differ only in scheduler-only registers are equal under
  this projection.
- **Scheduler snapshot, separately deterministic / replay-identical.**
  The scheduler snapshot (dispatch cursor, in-flight batch, stage
  queues as scheduler state) is compared on its own track: at quota 1
  it must itself be deterministic and replay-identical across the two
  runs. It is **not** folded into the semantic projection, and the pair
  is **not** a byte-identical whole-state snapshot comparison.

Consequences for the draft freeze:

- `PASS` (a future `Ack`-style verdict, not defined here) would require
  **both** tracks to hold: semantic projection equal **and** scheduler
  snapshot replay-identical.
- A divergence on either track is a typed `Fail`-class outcome (exact
  carrier open, §5); a divergence must never be reported as a pass.
- `Unsupported` (non-covered shape, wider ops, target execution) is
  never equal and never a pass — same rule as the VF12 M1 qualifier.
- Host non-semantic fields explicitly excluded per the VF13 row stay
  excluded here; the exclusion list itself is a co-freeze item (§5).

This matches the G9/G15 wording: quota-1 equivalence is under the
canonical semantic comparison projection with the scheduler snapshot
separately deterministic/replay-identical — **not** byte-identical
snapshots and **not** a frozen single-active-task architecture.

## 3. Single-tick determinism harness reuse (c19 / c17)

The draft proposes reusing the existing single-tick determinism harness
pattern, not inventing a new replay rig:

- `compiler/tests/c19_vf01.rs:626`
  (`vf01_snapshot_replay_is_deterministic`): `run = || { full_chain();
  run_vf01(); Snapshot::capture(&bus) }`, then `assert_eq!(run(),
  run())`.
- `compiler/tests/c17_vf12.rs:654`
  (`vf12_snapshot_replay_is_deterministic`): same shape — build the full
  PP→LX→PA→TY→SE→VF06→IR chain, drive one tick through the workers,
  `Snapshot::capture`, `assert_eq` over two runs.

The VF13 quota-1 draft would extend this shape in the obvious minimal
way: capture **both** the canonical semantic projection **and** the
scheduler snapshot per run, assert equality per track across the two
runs, and add a first-divergence negative (two runs with a forced
semantic drift must diverge on the semantic track, not pass). No new
record families, no schema change (PP-slice schema reuse, as in the
VF01/VF12 slices); the exact projection function, snapshot encoding,
and divergence carrier remain co-freeze items (§5).

Candidate freeze items (proposed, not frozen):

| # | Item | Draft acceptance sketch |
|---|---|---|
| R1-1 | Quota-1 semantic projection equal across replay | two identical runs: semantic projection `==` |
| R1-2 | Scheduler snapshot separately replay-identical | two identical runs: scheduler snapshot `==` |
| R1-3 | First-divergence negative | forced semantic drift ⇒ semantic-track divergence, never pass |
| R1-4 | Scheduler-only drift does not fail the semantic track | scheduler-only perturbation ⇒ semantic `==`, scheduler `!=` |
| R1-5 | No schema change | PP-slice schema reuse; projection is a view, not a new record |
| R1-6 | Loud negatives | non-running task / malformed payload / dangling ref ⇒ typed fail, never pass |

## 4. H6-M12 stays DEFERRED (label)

**H6-M12 (T13: "Quota1 semantic projection vs separate scheduler
snapshot") stays REQUIRED / PENDING — not implemented, not defined,
not sign-off.** The exact fixture encoding, task kind / stage / phase /
records, and verifier registration remain **T01/T02/T13 co-freeze /
pending**; the H9 numeric inventories remain unresolved. Nothing in
this draft defines, implements, freezes, or signs off H6-M12 or any
other H6-M row (M01–M16 unchanged). The batch / quota>1 replay
extension (VF13 full scope: replay across dispatch order) is likewise
required-pending and is not part of quota-1 acceptance per M1 G9.

## 5. Explicitly open (not decided here)

- Exact projection function (which fields are semantic vs
  scheduler-only) and its snapshot/projection encoding.
- Divergence verdict carrier (typed `Fail` shape, first-divergence
  record, `InvalidStage`-style gate handling).
- Host non-semantic exclusion list beyond the T13 row text.
- Registration: task kind, stage ordinal, layer, chip ID, manifest
  rows, hash rules — all T01/T13 co-freeze, none proposed here.
- Any quota>1 or cross-dispatch-order claim (needs measured
  before/after tick counts + separate integrator acceptance per T02
  quota policy; not in this draft).

## 6. Draft history

| Date | Note |
|---|---|
| 2026-10-10 | Created as **DRAFT ONLY** on `wave4-development`; new file only, no freeze, no chip code, no commit. |
