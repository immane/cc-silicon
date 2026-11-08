# Project Status

Last checked against the repository state on 2026-11-09. When this page and
the code disagree, the code wins; the links below are the places to
re-verify. This page records observed state, not a new acceptance run.

## At a glance

| Area | State | Evidence |
|---|---|---|
| Execution framework (`src/`) | **Implemented** | 15 integration tests + 6 doctests; runnable `examples/counter.rs` |
| Compiler contract foundation (`compiler/`) | **Implemented and frozen as `t01-c01-c06/40`** | Arenas/IDs, task/result protocol, commit, routing shell, snapshots, manifests, target gate; identity/hash in [`CONTRACT_VERSION`](../../compiler/contracts/CONTRACT_VERSION) |
| Language workers (PP/LX/PA/TY/SE/IR/VF) | **Partial slices only** | Multiple implemented and tested slices; no complete parser, type system, semantic pipeline, or code generator |
| M1 Part A A3 closure (`/40`) | **Implemented and tested** | Candidate dispatches SE29; VF06 checks the four-kind typed-fact set; T09 consumes the committed function fact. Evidence: c14 16, c15 13, c43 5, h04 18, candidate `value=05`. Full M1 acceptance and IR-4(b) are not claimed |
| C source → executable | **Not implemented** | Candidate Part A evidence driver exists; no target codegen, assembler/linker integration, or execution |
| AArch64 target values | **Unverified** | Identity frozen; probe never run; concrete values fail closed |
| ABI probe harness (`tools/torture/probe/`) | Scaffold, **never run** | Harness self-tests pass; no report, no attestation |
| GCC torture corpus | **Scaffold** | Lock schema + verifier tested; no corpus fetched, no pass rate |
| Static chip linter (`tools/chip-lint`) | Implemented aid, **not a proof** | 16 tests after Stage B; catches common violations only |
| Test/CI coverage | Root, chip-lint, compiler, torture, chip scan | `.github/workflows/ci.yml`; real target probe remains manual-only |

No complete compiler capability, verified target ABI, or GCC pass rate is
claimed by this repository or this page.

## What "frozen `/40`" means

The frozen artifact is `t01-c01-c06/40`:

```text
version=t01-c01-c06/40
hash=c6961ac65fb3fb7e4ad4fdb666c47584b94042949cb6ed17111a4d81b3b10c14
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

The following counts were observed during the 2026-11-09 `/40` integration
run (and, for the non-compiler packages, the 2026-11-07 dispatch audit). They
are discovery/run observations, not a permanent promise; the source tree and
`compiler/tests/` are authoritative.

| Package | Observed |
|---|---|
| Root framework | 21 tests (15 integration + 6 doctests) |
| Compiler | **700 tests across 50 suites passed** after `/40` (adds c43 and the `/40` VF06/T09/h04 cases) |
| Chip-lint | 12 tests at audit; 16 after B1-05 |
| Torture verifier | 73 tests |
| Probe self-tests | 57 tests; real probe not run |

## Known limitations

- No complete PP→LX→PA→TY→SE→CL→IR→target pipeline.
- Integrated preprocessing/lexing is narrow: source macros/includes are not
  orchestrated end-to-end, and the integrated lexer is M1-integer-only.
- Many parse/type/semantic/IR/verification catalog rows are absent; several
  parse drafts are registered nowhere or claim colliding IDs.
- `ConstRecord.ty`/`ir.op-immediate-type` remain deferred (B2-02); M1 Part A
  IR-4(b) must not be claimed at `/40`.
- The ABI/target probe has never run; no concrete target value may drive
  codegen.
- H05 isolated execution is absent; no untrusted binary may be run at scale.
- `AppendRecords` snapshot bodies, scheduler/limit invariants, and replay
  restore have known gaps tracked in the dispatch ledger.
- Adapters, host code, and custom backends remain trusted boundaries;
  determinism there is enforced by review, linting, and replay tests.

## Sources

- Root status table: [README.md](../../README.md) § Project status
- Task master plan: [tasks/README.md](../tasks/README.md)
- Dispatch ledger: [tasks/DISPATCH_LEDGER.md](../tasks/DISPATCH_LEDGER.md)
- Frozen identity: [compiler/contracts/CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION)
- Guardrails (accepted policy, not a freeze):
  [tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md](../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md)
