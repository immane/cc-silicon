# OP Freeze 03 (DRAFT ONLY): T10 OP20 ApplyRewrite + OP21 Invalidate Version-Guard Protocol

Status: **OPEN — DRAFT ONLY, PROPOSED**. Not an accepted contract, not an ADR, not a freeze. Authorizes no chip code, no bus/schema change, no implementation.

Scope: T10 rows OP20 (ApplyRewriteChip) and OP21 (AnalysisInvalidateChip) only. Normative sources remain `T01_COMPILER_CONTRACT.md`, `T10_OPTIMIZE_CHIPS.md`, the Silicon paradigm spec, and the SFL contract. If this draft conflicts with any of those, the existing accepted document wins.

## 1. Purpose

Freeze (proposed) the version-guard protocol by which optimization rewrite proposals become IR mutations:

1. Producers (OP02–OP10, OP19, future passes) emit guarded patches against a frozen IR/analysis version.
2. OP20 validates guards and applies patches transactionally or rejects them.
3. OP21 invalidates dependent analyses from the resulting mutation summary.

Goals: no stale rewrite lands, no partial application, no continued use of invalidated facts, deterministic replay from the same initial state.

## 2. Proposed `GuardedPatch` shape (unfrozen)

Each patch is a typed proposal, not a direct bus write. Proposed fields (names illustrative, unfrozen):

- `patch_id`: stable ID, unique per (function, pass epoch).
- `base_ir_version`: IR version the patch was computed against (required guard).
- `base_analysis_versions`: map of `analysis_kind -> version` for every fact the legality check relied on (use-def, dom, alias epoch, loop info, memory epoch). Empty means "relies on no analysis".
- `target_refs`: explicit instruction/block/operand IDs + tombstone check (no ID reuse; deleted IDs stay tombstoned per T01).
- `kind`: one of `ReplaceOp | ReplaceOperand | InsertOp | DeleteOp | RewriteCfg | MergeBlock | DeleteBlock | HoistOp` (closed list TBD by integrator; no open-ended "transform" kind).
- `payload`: kind-specific before/after records (old op snapshot + new op image).
- `legality_evidence`: rule ID + relied-upon facts (effect class, alias result, volatility/atomicity, FP mode, CFG condition).
- `ordering_key`: deterministic sequence key for applying multiple patches in one tick.

A patch that omits `base_ir_version`, references a tombstoned/reused ID, or names an analysis version it did not declare is malformed and rejected without application.

## 3. OP20 transactional apply (proposed)

1. **Freeze inputs:** OP20 reads only the frozen tick inputs: candidate `GuardedPatch` set, current IR version counter, current analysis versions, tombstone set.
2. **Guard check (all-or-nothing gate):** for each patch, in deterministic `ordering_key` order:
   - `patch.base_ir_version == current_ir_version`, else stale-reject (see §4).
   - every `base_analysis_versions[k] == current version of k`, else stale-reject.
   - every `target_ref` is live (not tombstoned, still bound to the recorded before-image), else conflict-reject.
   - pairwise conflict check: two patches writing the same instruction/block/operand, or one deleting what another reads, is a conflict; conflicting set is rejected (no partial winner unless a documented deterministic priority rule, TBD, applies).
3. **Transactional commit:** only if all admitted patches pass §3.2 does OP20 emit the mutation batch. Any guard/conflict/verification failure aborts the whole batch: IR, versions, and analyses unchanged, failure reported as a typed diagnostic/result (not a panic, not silent success).
4. **Post-apply verify (same tick proposal):** re-read the patched region (operand binding, phi/CFG well-formedness, use-def consistency for touched IDs); failure rolls back to the pre-batch state and reports failure.
5. **Version bump:** on success, bump the IR version counter exactly once per committed batch and emit a `MutationSummary` (changed blocks/ops, CFG delta, memory-effect delta) for OP21. Version bump is owned by the apply path only; no worker pass bumps versions directly.

## 4. Stale-reject rule (proposed)

- Stale = `base_ir_version` or any relied-upon analysis version differs from current at OP20 evaluation time.
- Stale patches are **rejected whole**: no partial effect, no version bump, no invalidation emitted for them.
- Rejection is typed and attributable: `StalePatch { patch_id, expected_version, observed_version, reason }`, so the producing pass can recompute against the new version in a later tick.
- Replay determinism: same initial IR + versions + patch set + ordering keys → same admit/reject outcome and same end state.

## 5. OP21 invalidation (proposed)

OP21 consumes the `MutationSummary` from OP20 and emits an invalidated-facts set:

- **Data change** (operand/opcode replace, insert, delete): invalidate use-def facts, value-number facts, and constant facts covering touched IDs.
- **CFG change** (edge retarget, block merge/delete, hoist across blocks): additionally invalidate dominator, dominance-frontier, loop-info, and reachability facts for the affected function.
- **Memory-effect change** (store/call/volatile/atomic added, removed, or moved; load hoisted): bump the memory epoch and invalidate alias facts and load-dependent redundancies.
- Invalidation is conservative: unknown or unclassified effects invalidate the superset, never the subset.
- Invalidated analyses must not be consumed by subsequent passes until recomputed; consuming a version that OP21 has marked invalid is a protocol error.
- OP21 performs no IR mutation itself; it only marks facts invalid and requests recomputation (feeds OP22 worklist per T10).

## 6. Proposed acceptance (for a future freeze)

- Stale patch against an old IR version is rejected; state and versions unchanged.
- Conflicting patch pair does not partially apply.
- Failed post-apply verification rolls back fully.
- After a CFG mutation, old dominator/loop facts are unusable until recomputed.
- After a memory-effect mutation, alias-dependent facts are unusable until recomputed.
- Deterministic replay: identical inputs → identical admit/reject + end versions.
- All faults are typed diagnostics; no panic, no fabricated success, no unbounded silent retry.

## 7. Open questions (blocking a freeze)

1. Closed `kind` list and per-kind before/after schema — integrator-owned.
2. Conflict priority vs. pure rejection when two legal patches overlap.
3. Batch granularity: one batch per tick per function vs. per-pass batches.
4. Version-counter scope: per-function, per-TU, global epoch — must match T01 bus ownership.
5. `MutationSummary` schema and OP22 worklist handoff fields.
6. Numeric encoding/hash coverage of the new records under the T01 `/6` foundation.

## 8. What this draft does NOT do

- Freezes nothing; changes no bus, schema, contract version, or chip manifest.
- Adds no chip code, no tests, no workspace changes.
- Does not authorize parallel implementation of OP02–OP22.
- Does not claim any pass rate, target behavior, or optimization capability.
