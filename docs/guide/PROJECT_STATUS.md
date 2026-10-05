# Project Status

Last checked against the repository state at the time of writing. When
this page and the code disagree, the code wins; the links below are the
places to re-verify.

## At a glance

| Area | State | Evidence |
|---|---|---|
| Execution framework (`src/`) | **Implemented** | 15 integration tests + 6 compile-fail doctests; runnable `examples/counter.rs` |
| Compiler contract foundation (`compiler/`, C01–C06) | **Implemented, frozen `/6`** | 147 integration tests + doctests; identity and hash in `contracts/CONTRACT_VERSION` |
| C language chips (T02–T13, 331 planned tasks) | **Not implemented** | Task descriptions only; no chip source exists |
| AArch64 target values | **Unverified** | Identity frozen; probe never run |
| ABI probe harness (`tools/torture/probe/`) | Scaffold, **never run** | Harness self-tests pass; no report, no attestation |
| GCC torture corpus | **Scaffold** | Lock schema + verifier tested; no corpus fetched, no pass rate |
| Static chip linter (`tools/chip-lint`) | Implemented aid, **not a proof** | 8 tests; catches common violations only |

No compiler capability, target verification result, or GCC pass rate
is claimed anywhere in this repository — and this page claims none
either.

## What "frozen /6" does and does not mean

The frozen artifact is `t01-c01-c06/6`:

```text
version=t01-c01-c06/6
hash=60935783b7b46cc62fc6fff64c532e840e7019a544055c594093d72dce0bf6d8
target=aarch64-unknown-linux-gnu
target_verification=unverified
```

**It does mean:**

- The shared vocabulary is fixed: record families, task states, the
  five result variants, proposal names and wire tags, store and group
  inventories, limits, target identity, and the normative rule IDs.
- Changing any of them requires a contract amendment and a new
  version — silent redefinition is a breach, not a refactor.
- Consistency tests pin these inventories to their implementations
  (`compiler/tests/freeze.rs`, currently 14 tests).

**It does not mean:**

- That any C code can be compiled. No language chip exists.
- That the frozen shapes are proven correct. The hash is a fingerprint
  of names and shapes, not a semantic proof; chip logic can change
  without changing the hash.
- That every future chip interface is frozen. Only the foundation
  (C01–C06) is frozen; per-language record schemas, group task kinds,
  worker routing, and stage queues are still ahead (see
  [Roadmap](ROADMAP.md)).

## Test inventory (so numbers are checkable, not slogans)

Compiler foundation integration tests by file:

| File | Tests | Covers |
|---|---|---|
| `c01_arena.rs` | 7 | Storage arenas and stable IDs |
| `c02_target.rs` | 11 | Target model (identity frozen, values unverified) |
| `c03_task.rs` | 46 | Task/result/proposal protocol |
| `c04_manifest.rs` | 15 | Manifest declarations and validation |
| `c05_codec.rs` | 21 | Canonical snapshot/trace encoding |
| `c06_routing.rs` | 15 | Dispatcher, budgets, recovery |
| `c07_limits.rs` | 18 | Configuration limits |
| `freeze.rs` | 14 | Hash recomputation + inventory consistency |

Framework: `tests/paradigm.rs` (15 tests) plus 6 doctests, including
compile-fail checks on the `silicon_chip!` macro.

## Known limitations, stated plainly

- The ownership allowlist is an empty skeleton; stage assignment
  accepts everything; canonical stage queues do not exist yet.
- `AppendRecords` proposals are rejected pending the records track;
  store patches record intent in a patch log.
- Language record arenas are reserved but schemaless — only live IDs
  are snapshotted, not record bodies.
- The join behavior differs in one documented edge from the await-all
  direction in an early draft; this needs a ruling before it is
  frozen (see [Roadmap](ROADMAP.md)).
- Adapters, host code, and custom backends remain trusted boundaries:
  determinism there is enforced by review, linting, and replay tests.

## Sources

- Root status table: [README.md](../../README.md) § Project status
- Task master plan: [tasks/README.md](../tasks/README.md)
- Frozen identity: [compiler/contracts/CONTRACT_VERSION](../../compiler/contracts/CONTRACT_VERSION)
- Guardrails (accepted policy, not a freeze):
  [tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md](../tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md)
