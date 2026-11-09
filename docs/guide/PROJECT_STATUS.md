# Project Status

Last checked against the repository state on 2026-11-10. When this page and
the code disagree, the code wins; the links below are the places to
re-verify. This page records observed state, not a new acceptance run.

## At a glance

| Area | State | Evidence |
|---|---|---|
| Execution framework (`src/`) | **Implemented** | 15 integration tests + 6 doctests; runnable `examples/counter.rs` |
| Compiler contract foundation (`compiler/`) | **Implemented and frozen as `t01-c01-c06/54`** | Arenas/IDs, task/result protocol, commit, routing shell, snapshots, manifests, target gate; identity/hash in [`CONTRACT_VERSION`](../../compiler/contracts/CONTRACT_VERSION) |
| Language workers (PP/LX/PA/TY/SE/IR/VF) | **Partial slices only** | Multiple implemented and tested slices; no complete parser, type system, semantic pipeline, or code generator |
| M1 Part A A3 closure (`/40`) | **Implemented and tested** | Candidate dispatches SE29; VF06 checks the four-kind typed-fact set; T09 consumes the committed function fact. Evidence: c14 16, c15 13, c43 5, h04 18, candidate `value=05`. Full M1 acceptance is not claimed |
| Determinism/limits/replay (`/41`) | **Implemented and tested** | Append wire bodies, stage-first scheduling, macro/store/result versions, zero-quota fail-closed, pin-aware trace decoder/first divergence, Task.id, ReplayEvidence. Full restore/quota>1/VF13 are deferred to `/55` |
| Tagged, typed constants (`/42`) | **Implemented and tested** | `ConstRecord.ty` + closed integer payload; fold type resolution from committed facts; T09/VF12 immediate-type checks; canonical typed snapshot body. Evidence: c45 8, candidate `value=05`. Float/address tags and full M1 remain future |
| Parser architecture + V1 AST (`/43`) | **Schema frozen; producer not implemented** | ADR-0003 single-task V1 parser decision; append-only `NodeKind` inventory; snapshot names/round-trip. V1 parser and VF05 V1 shape table remain future |
| IR/CFG common base (`/44`) | **Schema frozen; producer/verifier not implemented** | ADR-0004 `InstructionRecord.local`/`successors`; local/arithmetic/branch op append; snapshot round-trip. V1/V2 lowering and VF07/VF08 remain future |
| Constant v3 (`/45`) | **Schema frozen; producers not implemented** | ADR-0005 float/address tags, format-checked raw bits, symbolic addend; VF14 integer-only gate. T06 float/pointer compatibility remains future |
| Type/layout/init (`/46`) | **Schema frozen; producers/verifier not implemented** | ADR-0006 pointer/array/aggregate/enum types; typed layout/init arenas and append handles; symbolic-or-bit extents. Target byte layout and VF07 remain future |
| Diagnostic taxonomy (`/47`) | **Registry frozen; migration pending** | ADR-0007 groups/codes/helpers; legacy `Task:4` call sites remain until owner migration |
| Probe-attested target baseline (`/48`) | **Probe-attested; official baseline open** | Pinned Debian 12 arm64 / GCC 12.2.0 report, `target_verification=probe-attested`; `c48_target_probe` 3 |
| Source locals (`/49`) | **Implemented and tested** | `int main(void){int x=2;return x;}` parses, declares a block automatic, lowers `Constant`/`LocalStore`/`LocalLoad`/`Return`, models 2, and executes on native AArch64; `c56_source_locals` 5 macOS / 6 Linux |
| Local add + modeled integer (`/50`) | **Implemented and tested** | `int main(void){int x=2;return x+3;}` lowers to six IR instructions, VF12 returns `ResultValue::Integer(5)`, and native AArch64 exits 5; `c56_source_locals` 6 macOS / 8 Linux |
| Local binary operators (`/51`) | **Implemented and tested** | `-`, `*`, `/`, `%` lower to the matching IR and native AArch64 exits 255, 6, 0, 2; `c56_source_locals` 7 macOS / 10 Linux |
| Local expression trees (`/52`) | **Implemented and tested** | Precedence, parentheses, literal-left forms, and unary `+`/`-` parse/check/lower/interpret; native AArch64 covers the tree cases; `c56_source_locals` 8 macOS / 12 Linux |
| Local assignments (`/53`) | **Implemented and tested** | Ordered `x = <expr>;` statements, effect-carrying facts, `LocalStore` lowering, VF12 store model; `c56_source_locals` 10 macOS / 13 Linux |
| C source → executable | **M1 constant-return and local expression/assignment paths only** | `candidate --emit-asm` + native AArch64 assembler/linker/run covers `return 2+3`, closed local expression trees, and ordered assignments over one `int`. No general codegen, `-S`/`-c`, calls, memory, or control flow |
| AArch64 target values | **Probe-attested baseline; official CI/bare-metal baseline open** | Identity frozen; Debian-GCC-12.2.0 containerized report classified/attested; global `target_verification=probe-attested`; candidate codegen gate remains |
| ABI probe harness (`tools/torture/probe/`) | Run once in pinned container | Harness/normalizer/classifier tests; report fixture + `c48_target_probe`; no official CI/bare-metal baseline |
| GCC torture corpus | **Scaffold** | Lock schema + verifier tested; no corpus fetched, no pass rate |
| Static chip linter (`tools/chip-lint`) | Implemented aid, **not a proof** | 16 tests after Stage B; catches common violations only |
| Test/CI coverage | Root, chip-lint, compiler, torture, H05 runner, chip scan | `.github/workflows/ci.yml`; real target probe remains manual-only |

No complete compiler capability, verified target ABI, or GCC pass rate is
claimed by this repository or this page.

## What "frozen `/53`" means

The frozen artifact is `t01-c01-c06/54`:

```text
version=t01-c01-c06/54
hash=850adb9a2e4d4409072c9ad12586cd14aed5e5b305cbacc68a675b935f081e39
target=aarch64-unknown-linux-gnu
target_verification=probe-attested
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
  schemas, most handlers, stage queues, general target codegen, and the
  official T00 ABI/CI baseline remain ahead.

## Observed test inventory

The following counts were observed during the 2026-11-09 `/43` integration
run (and, for the non-compiler packages, the 2026-11-07 dispatch audit). They
are discovery/run observations, not a permanent promise; the source tree and
`compiler/tests/` are authoritative.

| Package | Observed |
|---|---|
| Root framework | 21 tests (15 integration + 6 doctests) |
| Compiler | **740 tests across 62 suites (746 on native AArch64 Linux) passed** after the `/53` assignment slice (adds ordered-statement and native assignment coverage) |
| Chip-lint | 12 tests at audit; 16 after B1-05 |
| Torture verifier | 73 tests |
| Probe self-tests | 57 tests; one containerized real probe report + H07 classification |

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
  attested in `c48_target_probe`; the global target state is
  `probe-attested`. The official T00 reference baseline/CI substrate is still
  open, and target codegen is only the M1 constant-return and local expression/assignment source paths plus an IR-shape arithmetic emitter.
- H05 process supervision is implemented in `tools/runner` (timeout/output
  caps/cancellation/Linux process-group+prlimit). It is not a kernel namespace
  sandbox, and H02/H03 target orchestration remains absent.
- Replay is recorded-input re-execution over a canonical fingerprint, not bus
  restoration; full `Snapshot::from_bytes`/restore, quota>1 order, VF13
  projections, and per-stage queues remain deferred to `/55`.
- Adapters, host code, and custom backends remain trusted boundaries;
  determinism there is enforced by review, linting, and replay tests.

## Sources

- Root status table: [README.md](../../README.md) § Project status
- Task master plan: [tasks/README.md](../tasks/README.md)
- Dispatch ledger: [tasks/DISPATCH_LEDGER.md](../tasks/DISPATCH_LEDGER.md)
- Frozen identity: [compiler/contracts/CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION)
- Guardrails (accepted policy, not a freeze):
  [tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md](../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md)
