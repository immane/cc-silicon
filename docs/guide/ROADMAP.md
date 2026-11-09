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

The CLI still refuses `-S`/`-c`; a host-only `--emit-asm` prototype covers
constant return and the `/49` local-return fixture, and one containerized
probe report is classified/attested. General target codegen and the official
T00 reference CI/bare-metal baseline remain blocked. The acceptance documents
still describe the full milestone, not the current narrow slices:

- [tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md](../tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md)
- [tasks/M1_TARGET_ACCEPTANCE.md](../tasks/M1_TARGET_ACCEPTANCE.md)

## Freeze work still needed (before broad chip implementation)

The foundation (`/39`) is frozen, but a parallel workforce cannot start
on all 331 chips yet: the interfaces those chips program against are
not all frozen. Partial slice kinds, records, materialization, and
stage/allowlist rows exist; broader group schemas and control lifecycle
remain ahead. Roughly, in dependency order:

1. **Group task kinds.** Foundation and several slice kinds exist. Each
   language group still needs its remaining kind names, codes, and
   ownership frozen (task packages T02–T13 propose them; the integrator
   disposes).
2. **Language record schemas.** Some arenas are typed; others remain
   reserved. Each family's typed record fields and ordering must be
   frozen by its owning group.
3. **Typed record drafts + materialization.** Typed append
   materialization exists for frozen families; the remaining draft
   vocabulary and the reserve/resolve/materialize chain must be frozen
   before broader `AppendRecords` use.
4. **Group store fields + snapshot bodies.** Group-declared store
   fields and the canonical encoding of each record body follow from
   the schemas above.
5. **Scheduling chain.** Kind→stage assignment and per-kind worker
   routing exist in part; canonical stage queues and the control-chip
   lifecycle (start/import/cancel/budget) must be frozen together —
   these change observable behavior and each needs an explicit decision
   and version.
6. **Join/progress edge semantics.** One documented timing difference
   needs a ruling before it is frozen.
7. **Diagnostics.** Per-group language error codes and attribution.
8. **Target values.** A pinned Linux probe has run and its report is
   attested; the official T00 CI/bare-metal baseline and general target
   codegen remain open. Independent of language work except at the emission
   edge.

The standing rule for this work: **grow through small, complete
vertical slices.** Freeze one slice's schemas, implement it with tests,
then widen. Do not declare all 331 interfaces ready before they are.

**Current freeze state (2026-11-10):** `/40` closed the M1 Part A A3 path,
`/41` closed the B2-00 determinism/limits/replay package (append wire
bodies, stage-first scheduling, macro/store/result versions, pin-aware
trace evidence), and `/42` closed B2-02 tagged, typed constants
(`ConstRecord.ty`, integer payload, immediate-type checks). `/43` freezes the
single-task V1 parser decision and append-only V1 AST inventory. Host-only
B3-03a/B3-03b add source `#define`/`#undef` and bounded quoted includes.
`/44`–`/46` are the IR/CFG, layout/init/constant, and diagnostic schema
batches; `/47` is the diagnostic taxonomy; `/48` is the probe-attested
target baseline; `/49` is the source local load/store slice. `/50` owns full
snapshot restore, quota>1 order, VF13 projections, per-stage queues, and
`StorePatch` materialization.

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
- Resolve stale version references (e.g. historical `/5`–`/38` mentions
  alongside frozen `/39`) by labeling them as history, not by silent
  replacement.
- Keep dated review conclusions in `reviews/`; when the guide cites
  one, say whether the issue is still open.

## Sources

- Master plan and milestones: [tasks/README.md](../tasks/README.md)
- Gate definition: [tasks/T00_GCC_TORTURE_GATE.md](../tasks/T00_GCC_TORTURE_GATE.md)
- Parallel rules: [tasks/PARALLEL_EXECUTION.md](../tasks/PARALLEL_EXECUTION.md)
