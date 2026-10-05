# Roadmap

This page lists what comes next and what blocks it. Items are ordered
by dependency, not by optimism: nothing here is promised until its
prerequisites are frozen and its acceptance is measured.

## The next visible victory: M1

Compile `int main(void){return 2+3;}` through the candidate's own
pipeline and get exit code 5. M1 splits into two halves:

- **Part A (target-independent):** source text through IR for the
  return statement — preprocessing, lexing, parsing, types, semantics,
  constant folding, IR lowering.
- **Part B (target-dependent, probe-gated):** IR through AArch64
  assembly, external assemble/link, run, exit code 5.

Part B stays disabled until the Linux probe attests real ABI values.
Part A is where implementation starts. Both halves are acceptance
documents today, not running code:

- [tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md](../tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md)
- [tasks/M1_TARGET_ACCEPTANCE.md](../tasks/M1_TARGET_ACCEPTANCE.md)

## Freeze work still needed (before broad chip implementation)

The foundation (`/6`) is frozen, but a parallel workforce cannot start
on all 331 chips yet: the interfaces those chips program against are
not all frozen. Roughly, in dependency order:

1. **Group task kinds.** Only the four foundation kinds exist. Each
   language group needs its kind names, codes, and ownership frozen
   (task packages T02–T13 propose them; the integrator disposes).
2. **Language record schemas.** The arenas are reserved but schemaless.
   Each family's typed record fields and ordering must be frozen by
   its owning group.
3. **Typed record drafts + materialization.** The draft handle exists;
   the closed draft vocabulary and the reserve/resolve/materialize
   chain must be frozen, which unblocks `AppendRecords`.
4. **Group store fields + snapshot bodies.** Group-declared store
   fields and the canonical encoding of each record body follow from
   the schemas above.
5. **Scheduling chain.** Kind→stage assignment, canonical stage
   queues, per-kind worker routing, and the control-chip lifecycle
   (start/import/cancel/budget) must be frozen together — these change
   observable behavior and each needs an explicit decision and version.
6. **Join/progress edge semantics.** One documented timing difference
   needs a ruling before it is frozen.
7. **Diagnostics.** Per-group language error codes and attribution.
8. **Target values.** Run the Linux probe, attest the report, unblock
   Part B. Independent of language work except at the emission edge.

The standing rule for this work: **grow through small, complete
vertical slices.** Freeze one slice's schemas, implement it with tests,
then widen. Do not declare all 331 interfaces ready before they are.

## After M1: the longer climb

| Milestone | Content | Gate |
|---|---|---|
| M2 | Integers, control flow, functions, pointers, arrays, loops | Layered fixtures green; torture suite starts reporting with a fixed denominator |
| M3 | Aggregates, initialization, bitfields, VLA, floats, variadic, old-style C | Every catalog chip implemented or explicitly gapped; ABI mixed-linking checks |
| M4 | GNU gaps, target features, optimization levels | compile/execute/ieee climbing 90% → 95% → 98%, every failure owned |
| M5 | Full matrix + supplementary suites + regression/perf budget | Each T00-defined rate above 99% |

The final gate (M5) is defined in
[tasks/T00_GCC_TORTURE_GATE.md](../tasks/T00_GCC_TORTURE_GATE.md). The
T00 corpus/environment/option freeze itself is still pending, and no
rate has been measured.

## Documentation work still needed

- Normalize every task package header to the same status block
  (status, scope, required reading, frozen vs. unfrozen interfaces,
  allowed files, acceptance, current blockers) — without changing
  task IDs, content, or links.
- Resolve stale version references (e.g. historical `/5` mentions
  alongside frozen `/6`) by labeling them as history, not by silent
  replacement.
- Keep dated review conclusions in `reviews/`; when the guide cites
  one, say whether the issue is still open.

## Sources

- Master plan and milestones: [tasks/README.md](../tasks/README.md)
- Gate definition: [tasks/T00_GCC_TORTURE_GATE.md](../tasks/T00_GCC_TORTURE_GATE.md)
- Parallel rules: [tasks/PARALLEL_EXECUTION.md](../tasks/PARALLEL_EXECUTION.md)
