# Wave Dispatch: serial bootstrap → 100+ parallel generation

| Field | Value |
|---|---|
| Status | **DESIGN** — execution machinery, not a freeze |
| Principle | One human round (this document + three rules below), then autonomous waves |
| Capacity | 100+ concurrent chip authors; one serial integrator |

## 1. Pipeline

```text
Phase 0  SERIAL (integrator)     template chip + layout + work-order format + merge machinery
Phase 1  SERIAL (integrator)     freeze the wave's schemas/kinds/stages/manifests in code (one version bump)
Phase 2  PARALLEL (100+ agents)  one agent per chip (or per 2-3 adjacent chips), disjoint files only
Phase 3  SERIAL (integrator)     merge patches, full suite, fix fallout, report
         → next wave
```

## 2. Why serial first, and what it produces

Mass parallelism only works when parallel workers never need to ask.
Phase 0/1 produce, once:

- `compiler/src/chips/` layout + one template chip proving the
  adapter/manifest/routing/test pattern (`TASK_TEMPLATE` executable form).
- A **work-order schema**: every dispatch carries the pinned contract
  version + hash, the chip's frozen inputs/outputs, assigned files
  (nothing else writable), the exact verify commands, and the defect
  protocol. Incomplete work orders are never dispatched.
- Merge machinery: agents write disjoint paths only; the integrator
  owns all shared files (`lib.rs` mod lists, registries, routes,
  schemas, hashes) and the final full-suite run.

## 3. Agent contract (no-human-contact rules)

1. **Never ask.** Either deliver passing work or return a structured
   `DEFECT` (missing interface, contradictory spec, blocked file).
   Guessing an interface is a breach.
2. **Never touch shared files.** Assigned chip file(s) + assigned test
   file(s) only. Registry/route/schema/manifest edits are integrator
   work; needing one = file a `DEFECT`.
3. **Prove it.** Every delivery ends with the work order's verify
   commands and their passing output pasted in the response.
4. **Unsupported is a shell delivery.** `Unsupported` diagnostics with tests
   are a complete shell delivery (explicit gap recorded, failure retained),
   never a semantic completion and never an acceptance PASS. Faked success
   is a breach.

## 4. Integrator merge loop (Phase 3, no human needed)

1. Collect deliveries per batch; apply to the wave branch.
2. `cargo fmt --check`, `clippy -D warnings`, full `cargo test`.
3. On failure: bisect to the offending delivery, revert it, re-dispatch
   that chip with a corrected work order. The wave lands when the suite
   is green; stragglers ride the next wave, they never block it.
4. Report: per-chip pass/defect ledger. Human sees the ledger, not questions. Required-producer stragglers block closed-loop acceptance: a green suite without a required producer is not a pass for its consumers.

## 5. What stays human (the only three rules)

- **R1 — version bumps:** each wave freeze bumps the amendment version
  automatically (`/8`, `/9`, …), hash recomputed, history preserved.
  No per-bump approval once this rule is accepted.
- **R2 — unknowables stay unsupported:** probe values (no probe data
  exists) and census scope (no corpus fetched) are NEVER invented.
  Dependent chips are generated with explicit `Unsupported` paths and
  deferred bodies, and unblocked by a later probe/census wave.
- **R3 — abort threshold:** if a wave's defect rate exceeds 20%, the
  integrator stops and reports instead of burning 100 agents on a bad
  freeze. Below 20%, defects recycle automatically.

## 6. Wave sizes (from CHIP_PLAN.md)

| Wave | Chips | Agents | Notes |
|---|---|---|---|
| 1 | 1 (T08 fold) | integrator only | proves the pattern; G1-CL-01 graduates to tick-level |
| 2 | ~120 (M1 frontend + VF subset) | ~60 (2 chips each) | needs per-slice serial freezes first (Phase 1) |
| 3 | ~150 (full C + T10 + rest) | ~100 | biggest parallel push |
| 4 | 39 + dependents (probe-gated) | per R2: generated as Unsupported-shells until probe lands | bodies fill in post-probe without re-dispatch |
| 5 | census splits | on demand | registered with ledger entries |

Counts are dispatch planning, not success metrics.
