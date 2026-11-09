# Project Status

Last checked against the repository state on 2026-11-09. When this page and
the code disagree, the code wins; the links below are the places to
re-verify. This page records observed state, not a new acceptance run.

## At a glance

| Area | State | Evidence |
|---|---|---|
| Execution framework (`src/`) | **Implemented** | 15 integration tests + 6 doctests; runnable `examples/counter.rs` |
| Compiler contract foundation (`compiler/`) | **Implemented and frozen as `t01-c01-c06/47`** | Arenas/IDs, task/result protocol, commit, routing shell, snapshots, manifests, target gate; identity/hash in [`CONTRACT_VERSION`](../../compiler/contracts/CONTRACT_VERSION) |
| Language workers (PP/LX/PA/TY/SE/IR/VF) | **Partial slices only** | Multiple implemented and tested slices; no complete parser, type system, semantic pipeline, or code generator |
| M1 Part A A3 closure (`/40`) | **Implemented and tested** | Candidate dispatches SE29; VF06 checks the four-kind typed-fact set; T09 consumes the committed function fact. Evidence: c14 16, c15 13, c43 5, h04 18, candidate `value=05`. Full M1 acceptance is not claimed |
| Determinism/limits/replay (`/41`) | **Implemented and tested** | Append wire bodies, stage-first scheduling, macro/store/result versions, zero-quota fail-closed, pin-aware trace decoder/first divergence, Task.id, ReplayEvidence. Full restore/quota>1/VF13 are deferred to `/47` |
| Tagged, typed constants (`/42`) | **Implemented and tested** | `ConstRecord.ty` + closed integer payload; fold type resolution from committed facts; T09/VF12 immediate-type checks; canonical typed snapshot body. Evidence: c45 8, candidate `value=05`. Float/address tags and full M1 remain future |
| Parser architecture + V1 AST (`/43`) | **Schema frozen; producer not implemented** | ADR-0003 single-task V1 parser decision; append-only `NodeKind` inventory; snapshot names/round-trip. V1 parser and VF05 V1 shape table remain future |
| IR/CFG common base (`/44`) | **Schema frozen; producer/verifier not implemented** | ADR-0004 `InstructionRecord.local`/`successors`; local/arithmetic/branch op append; snapshot round-trip. V1/V2 lowering and VF07/VF08 remain future |
| Constant v3 (`/45`) | **Schema frozen; producers not implemented** | ADR-0005 float/address tags, format-checked raw bits, symbolic addend; VF14 integer-only gate. T06 float/pointer compatibility remains future |
| Type/layout/init (`/46`) | **Schema frozen; producers/verifier not implemented** | ADR-0006 pointer/array/aggregate/enum types; typed layout/init arenas and append handles; symbolic-or-bit extents. Target byte layout and VF07 remain future |
| Diagnostic taxonomy (`/47`) | **Registry frozen; migration pending** | ADR-0007 groups/codes/helpers; legacy `Task:4` call sites remain until owner migration |
| C source → executable | **Not implemented** | Candidate Part A evidence driver exists; no target codegen, assembler/linker integration, or execution |
| AArch64 target values | **One attested probe report; official baseline open** | Identity frozen; Debian-GCC-12.2.0 containerized report classified/attested; global `target_verification=unverified` and candidate codegen gate remain |
| ABI probe harness (`tools/torture/probe/`) | Run once in pinned container | Harness/normalizer/classifier tests; report fixture + `c48_target_probe`; no official CI/bare-metal baseline |
| GCC torture corpus | **Scaffold** | Lock schema + verifier tested; no corpus fetched, no pass rate |
| Static chip linter (`tools/chip-lint`) | Implemented aid, **not a proof** | 16 tests after Stage B; catches common violations only |
| Test/CI coverage | Root, chip-lint, compiler, torture, chip scan | `.github/workflows/ci.yml`; real target probe remains manual-only |

No complete compiler capability, verified target ABI, or GCC pass rate is
claimed by this repository or this page.

## What "frozen `/43`" means

The frozen artifact is `t01-c01-c06/47`:

```text
version=t01-c01-c06/47
hash=3c3d722def942d229e95d15f8e6758ee88f9a71720b170b3751bebf38245f593
target=aarch64-unknown-linux-gnu
target_verification=unverified
```

**It does mean:**

- Shared vocabulary is fixed: record families, task states, result variants,
  proposal names/wire tags, stores, limits, target identity, stage rows,
  allowlist rows, and normative rule IDs.
- Changing them requires a contract amendment and a new version; silent
  redefinition is a breach.
- Consistency tests pin these inventories to their implementations
  (`compiler/tests/freeze.rs`).

**It does not mean:**

- That any C source can be compiled to an executable. Only partial language
  slices and a Part A evidence path exist.
- That the hash proves semantic correctness. It fingerprints names and shapes,
  not chip logic.
- That all future chip interfaces are frozen. Broad per-language record/task
  schemas, most handlers, stage queues, target codegen, and ABI values remain
  ahead.

## Observed test inventory

The following counts were observed during the 2026-11-09 `/43` integration
run (and, for the non-compiler packages, the 2026-11-07 dispatch audit). They
are discovery/run observations, not a permanent promise; the source tree and
`compiler/tests/` are authoritative.

| Package | Observed |
|---|---|
| Root framework | 21 tests (15 integration + 6 doctests) |
| Compiler | **723 tests across 60 suites passed** after `/47` (adds AST, IR/CFG, constant-v3, type/layout/init, and diagnostic-taxonomy schema tests on top of tagged constants, PP driver/include, and target attestation) |
| Chip-lint | 12 tests at audit; 16 after B1-05 |
| Torture verifier | 73 tests |
| Probe self-tests | 57 tests; real probe not run |

## Known limitations

- No complete PP→LX→PA→TY→SE→CL→IR→target pipeline.
- Integrated preprocessing/lexing is narrow: source `#define`/`#undef` and
  quoted single-level `#include "..."` beside the including file are
  orchestrated by B3-03a/B3-03b, but `-I`, angle/system headers, nested
  includes, guards/`once`, and `#line`/`#pragma` remain fail-closed;
  post-conditional source macro mutation is rejected, and the integrated
  lexer is M1-integer-only.
- Many parse/type/semantic/IR/verification catalog rows are absent. Unregistered
  parse drafts are explicitly parked by B-INT-00; their candidate IDs are not
  frozen, and canonical PA15 selection remains future work.
- `ConstRecord.ty`/`ir.op-immediate-type` are implemented at `/42`, but this is
  not full M1 acceptance or target execution.
- A pinned Debian-GCC-12.2.0 containerized probe has run and its report is
  attested in `c48_target_probe`; the official T00 reference baseline/CI
  substrate is still open, the global target state remains unverified, and no
  concrete target value drives candidate codegen yet.
- H05 isolated execution is absent; no untrusted binary may be run at scale.
- Replay is recorded-input re-execution over a canonical fingerprint, not bus
  restoration; full `Snapshot::from_bytes`/restore, quota>1 order, VF13
  projections, and per-stage queues remain deferred to `/47`.
- Adapters, host code, and custom backends remain trusted boundaries;
  determinism there is enforced by review, linting, and replay tests.

## Sources

- Root status table: [README.md](../../README.md) § Project status
- Task master plan: [tasks/README.md](../tasks/README.md)
- Dispatch ledger: [tasks/DISPATCH_LEDGER.md](../tasks/DISPATCH_LEDGER.md)
- Frozen identity: [compiler/contracts/CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION)
- Guardrails (accepted policy, not a freeze):
  [tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md](../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md)
