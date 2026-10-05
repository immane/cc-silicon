# cc-silicon-compiler — T01 contract foundation (C01–C06)

This package owns the **compiler application contract foundation** on top of
the domain-free [`cc-silicon`](../) framework. It is the frozen T01 artifact
and the shared schema every compiler task package must obey.

It is **not a C compiler** and contains **no C language chips**. It compiles
and tests the storage profile, IDs, task/result protocol, target/config
schema, manifest extension, deterministic serializer, and the minimal routing
shell. Language work (T02–T13) builds on this foundation.

## Status

| Item | State |
|---|---|
| CPU dynamic arena extension | Approved — see [ADR-0001](../docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) |
| Target identity | Frozen: `aarch64-unknown-linux-gnu`, ELF, LP64, little-endian, AAPCS64 |
| Target concrete values | **UNVERIFIED**; codegen readiness fails closed |
| Verification | Private verified representation; `attest` validates caller-supplied report data (not authenticity) |
| Probe substrate | Planned Linux CI/VM; **not provisioned** |
| Corpus | On-demand, hash-locked; **not fetched here** |
| Reference DejaGnu baseline | Authorized as oracle; **not available**, never candidate evidence |
| Resource limits | Every configured bound preflighted before mutation on the checked bus/commit paths; direct public-store mutation is a trusted integration boundary (arena-local checks only) |
| Contract version | `t01-c01-c06/21` (`contracts/CONTRACT_VERSION`) |
| Contract hash | `fba01a2b34fe7eb12527a2c1a6a08980d8015b61b725c1d9e43af300a72754b5` |

## Frozen contract artifact

`contracts/CONTRACT_VERSION` is the obvious, cross-linked artifact:

```text
version=t01-c01-c06/21
hash=fba01a2b34fe7eb12527a2c1a6a08980d8015b61b725c1d9e43af300a72754b5
target=aarch64-unknown-linux-gnu
target_verification=unverified
probe_substrate=linux-ci-vm (planned, not provisioned)
corpus_fetch=on-demand hash-locked
reference_baseline=authorized, not available, not candidate evidence
hash_scope=normative-shapes-and-rule-ids (not source code, not semantic proof)
hash_excludes=runtime-registrations, routing-content, group-declared-store-fields, chip-logic
verified_state=private-VerifiedState; attest validates caller-supplied report data and hash, not authenticity or physical provenance
```

### What the hash covers — and what it does not

The hash is a SHA-256 fingerprint of the contract's **normative shapes and rule
identifiers**: record families, task groups/kinds, normative enumerations
(task states, result values, proposals, patch ops, capabilities, backend
classes, phases, host requests, diagnostic groups, stores), the
[`contract::NORMATIVE_RULES`] identifier list, the target profile and its
verification state, limits, probe/corpus policy, and the foundation store
schema. Enumerations are encoded by **variant name**, not by numeric
discriminant or wire tag: the `*_NAMES` lists and `RECORD_KINDS` pin names and
order, while the numeric snapshot tags (`snapshot.rs`) are hardcoded and are not
derived from or cross-checked against those lists. A tag change does not change
the frozen hash; numeric-value hashing remains a future `/6` item.

It is **not a source-code hash and not a semantic-equivalence proof**. Logic
inside a function can change without changing any identifier. The hash also
excludes runtime state that only appears at execution time: registered
`ManifestRegistry` content, the live `RoutingTable`, and any `StoreSchema`
fields a group adds after `StoreSchema::foundation`. Those are captured by the
**snapshot** (`c05`), not by the frozen hash. To make an intentional normative
change visible:

```sh
cargo run --manifest-path compiler/Cargo.toml --example freeze_hash
```

then bump `CONTRACT_VERSION`, update `CONTRACT_HASH`, regenerate
`contracts/CONTRACT_VERSION`, and rerun every dependent package.
`tests/freeze.rs` fails on any mismatch.

## Target verification (H1)

Verification is deliberately hard to obtain:

- `VerificationState` has a private representation; there is no public
  constructor for the verified state, and `TargetSpec` fields are private.
- `TargetSpec::attest(&ProbeReport)` is the only path to a verified target. It
  requires `verified = true`, every required probe field (including
  `wchar_t.encoding`), the frozen identity, and a `report_hash` equal to
  SHA-256 of the canonical unsigned report body. Unknown wide-character
  encodings are rejected.
- `CompilerConfig::ensure_codegen_ready` fails closed while unverified.
- `ProbeReport::status` is a weaker, presence-only check: with a valid
  `verified`/`report_hash` it reports `ProbeStatus::Verified` without checking
  the frozen identity and even while the four AAPCS64 fields are `unresolved`;
  only `TargetSpec::attest` checks identity and parses those values, failing
  with `ProbeError::BadValue` on an unresolved field.

What is unforgeable is the **private verified-state representation**, not the
report. `attest` validates caller-supplied report data and its hash; it does
**not** establish authenticity or that a physical probe ran. Whoever supplies
the report bytes can still supply a fabricated report, and the report file is
read at a point in time (TOCTOU). Trust and freshness of the report source are
integration responsibilities. The probe harness (H01) emits the
`wchar_t.encoding` field (utf32 or unresolved); this package does not own or
edit the probe tool (`tools/torture/probe/**` is the H01 area in this
repository).

## Layout

```text
compiler/
  src/
    arena.rs       append-only typed arenas, checked access, capacity errors
    ids.rs         stable newtype IDs + RecordRef
    intern.rs      deterministic string interning
    limits.rs      configured limits + LimitError
    target.rs      frozen identity, private verification, probe attestation
    diagnostic.rs  structured diagnostics and error protocol
    task.rs        task/result/proposal protocol and kind registry
    bus.rs         CompilerBus storage profile + limit enforcement
    commit.rs      staged, field-scoped atomic commit (CT06 protocol)
    manifest.rs    SFL manifest extension and validator (C04)
    codec.rs       canonical writer + SHA-256
    snapshot.rs    deterministic full snapshot/trace/config hash
    routing.rs     routing shell, routing table lives in the bus (C06)
    contract.rs    frozen schema, normative rules, content hash
  contracts/       frozen artifact, manifest schema doc, target probe fixture
  examples/        freeze_hash
  tests/           c01..c07 + freeze
```

## Commands and results (2026-10-04)

```sh
cargo fmt    --manifest-path compiler/Cargo.toml -- --check          # clean
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings  # clean
cargo test   --locked --manifest-path compiler/Cargo.toml            # all green
```

Test breakdown: `c01_arena` 7, `c02_target` 11, `c03_task` 46, `c04_manifest` 15,
`c05_codec` 21, `c06_routing` 15, `c07_limits` 18, `freeze` 7, plus 2 compile-fail
doctests for non-forgeable verification. Coverage boundary: `c07_limits`
exercises the checked bus/commit entry points only, so no test establishes
global budgets for direct public-store mutation, and `max_intern_bytes` has no
dedicated test.

## Guarantees

- Deterministic: stable IDs, no reuse, explicit ordering, canonical encoding.
- Checked: every access returns a structured error, never a panic.
- Bounded on the checked paths: `alloc_source`, task allocation (`bootstrap_task`),
  `intern_name`, routing propagation/diagnostic emission, and `commit_proposals`
  preflight every configured limit (source bytes, records total, tasks total,
  task depth, diagnostics, queue, per-arena, proposals per tick, ticks, intern)
  before mutation. The public mutable stores (`bus.arenas` and its public arenas,
  `bus.patch_log`, `bus.tasks.ready`, ...) are a trusted integration/host
  boundary: raw `TypedArena`/`ReservedArena` allocation enforces only the
  per-arena capacity, and public `get_mut` or direct pushes bypass the global
  budgets. Worker chips mutate only through the checked entry points and the
  commit path.
- Atomic: a proposal batch either fully validates and commits or commits
  nothing. Inner `Complete`/`Fail`/`AwaitHost` task IDs are bound to the
  enclosing task; Enqueue parents resolve against existing or earlier predicted
  siblings; and the apply pass performs only infallible appends after the
  capacity preflight.
- Single transition: a task receives at most one of `Complete`, `Fail`, or
  `AwaitHost` per batch; duplicates and mixed transitions are rejected before
  mutation. The proposal budget is compared losslessly in `usize`.
- Attributed: store patches must come from the task's owning chip, match the
  enclosing task, be declared in that chip's registered write manifest, and
  must not target the read-only `config` store. Manifest registration itself
  validates against the schema and kind registry. An `Enqueue` destination chip
  must be registered and accept the destination kind. Source content hashes are
  computed internally from the bytes. Consuming an already-consumed result
  yields the accurate `ResultAlreadyConsumed` error.
- Immutable config: `CompilerBus::config` is private with a getter, so the
  target/dialect/limits cannot be swapped after initialization; commit also
  rejects `config` writes.
- Explicit failure: no-op tasks terminate; unsupported and unregistered tasks
  fail with a structured diagnostic; a commit failure never strands a task in
  `Running`.

## Integration layout (T01 decision)

`compiler/` is a **nested standalone Cargo package** with a path dependency on
the root crate and its own `Cargo.lock`. Rationale:

- the root `cc-silicon` package defaults and its `.gitignore` (`/Cargo.lock`)
  stay untouched;
- `tools/chip-lint` keeps its own isolated lock file, as today;
- root CI adds explicit compiler steps so the contract app is built and tested
  in the same root CI job (`.github/workflows/ci.yml`).

A future single-lock workspace remains possible if the integrator later decides
one is needed; that would be a separate workspace change, not a per-chip
owner change. Root CI covers the compiler today.

## M0 foundation gates

| M0 gate | State | Evidence |
|---|---|---|
| T01 schema frozen | **Fulfilled (envelope)** | compiled types + `tests/freeze.rs`; version/hash in `contracts/CONTRACT_VERSION` |
| Empty task → complete | **Fulfilled** | `c06_routing::noop_task_terminates_normally` |
| Progress / budget | **Fulfilled (checked paths)** | `c06`/`c07` budget, queue, depth, total, diagnostics, source, proposal bounds |
| Replay consistent | **Fulfilled** | `c05_codec::snapshot_and_trace_replay_identically` |
| Full observable snapshot | **Fulfilled (foundation)** | `c05_codec` source/record/routing/manifest sensitivity tests |
| Unimplemented ≠ success | **Fulfilled** | `c06` unsupported/unregistered fail; `c06` commit failure is explicit |
| T00 reference GCC baseline | **Not fulfilled** | no corpus fetched, no runner, no probe |

## What blocks Wave 1

1. **Target probe not run.** C02 concrete ABI values are UNVERIFIED; target
   code chips (T11) must not start; `ensure_codegen_ready` fails closed.
2. **Per-group schemas are proposals.** Concrete task/result payload variants
   and language-store record fields are not frozen. Each group owner must
   submit a freeze request (add kinds to `TaskKindRegistry`, fields to
   `StoreSchema`, records to a reserved arena) before its chips can run.
3. **T02 control chips are not implemented.** The routing shell has no worker
   handlers; CT01–CT14 are pending.
4. **No host/toolchain/runner.** H00–H03 (corpus lock, runner, DejaGnu
   integration) do not exist, so no torture rate can be claimed.

## Explicit limitations and deferred items

- **No C compiler exists.** No language chips, no code generation, no corpus
  run, no pass rate.
- Attestation validates report data, not authenticity or physical provenance;
  TOCTOU applies to the report file (documented above and in `target.rs`). It
  also performs no plausibility cross-checks beyond field presence, frozen
  identity, and hash (for example, zero or oversized ABI register counts, or
  `align > size`, would be accepted); `c02_target::signed_report` recomputes the
  hash over a caller-built report to demonstrate that a fabricated report can be
  attested. The probe harness (H01, not owned here) emits
  `wchar_t.encoding` as `utf32` only with positive ISO/IEC 10646/non-BMP
  evidence, otherwise `unresolved`; a resolved encoding is required before
  attestation.
- The T01 §4 deterministic reserved-ID/local-reference relocation protocol is
  not implemented or frozen: `commit.rs` resolves only earlier predicted
  `Enqueue`-parent IDs within one batch, store-patch `RecordRef`s are not
  existence-checked, and no reservation/apply-map protocol or normative rule
  exists yet (M1 proposal OB-49).
- The `/6` hash-scope reconciliation is open: the accepted two-tier model
  (frozen `foundation + M1AppendSchema` seed participates in the hash; post-seed
  `StoreSchema::declare()` stays excluded) is not yet encoded, so
  `COMPILER_SFL_MANIFEST.md` §4 and the `hash_excludes=group-declared-store-fields`
  token must change atomically with `FrozenSchema::encode` and `freeze.rs` at
  `/6` (M1 proposal OB-34).
- Integration gap (N1): the H01 normalizer emits the four AAPCS64 fields
  (`abi.gp_arg_regs`, `abi.fp_arg_regs`, `abi.stack_align`,
  `abi.variadic_register_save_area`) as `unresolved`, and `verified`/`report_hash`
  are caller-supplied; `attest` requires resolved numeric values, so the raw
  probe report becomes attestable only after an explicit classifier/integration
  step fills them (H07 reference-oracle/ABI classification; the T01 integrator
  owns the `attest`/`Probed` step). The probe is not marked complete and no ABI
  register values are fabricated here
  ([T00](../docs/tasks/T00_GCC_TORTURE_GATE.md) §4.1).
- Per-group task/result payload enums are not frozen. The envelope carries
  typed `RecordRef` payloads; each task group must freeze its own variants.
- Reserved language stores have no record schema yet; the snapshot encodes
  each store's `allocated` count and live IDs (so tombstone positions remain
  visible) but not record bodies (`snapshot.rs`).
- `RecordRef`s inside a store patch are **not** existence-checked by the
  mechanical commit; materialization is owned by each group's commit
  integration (documented in `commit.rs`).
- The manifest validator is a lint, not a parser or proof.
- Routing has no installed worker handlers; T02 installs them.
- `bootstrap_task` is integration/job-bootstrap only (CT01) and is documented
  as such; worker chips may only propose `Enqueue` through the commit path.
- Global resource limits are guaranteed on the checked bus/commit entry points
  (`alloc_source`, task bootstrap/allocation, `intern_name`, routing
  propagation/diagnostic emission, `commit_proposals`). The public mutable
  stores (`bus.arenas`, `bus.patch_log`, `bus.tasks.ready`, ...) are a trusted
  integration/host surface: direct mutation enforces only arena-local checks and
  can exceed the global total/source/task/diagnostic budgets, so worker chips
  must not use it.
- The `start_job`/`host_responses` pins are reserved for T02 CT01/CT02. The
  `cancel` and `tick_budget_reached` pins have defined behavior in the shell
  (`Cancelled` and `BudgetExhausted` respectively).
- Frozen hash coverage excludes runtime registrations, live routing content,
  and group-declared store fields added after `StoreSchema::foundation`; those
  are covered by the snapshot instead.

## Dependency

`cc-silicon = { path = ".." }`. No third-party crates; the serializer and
SHA-256 are implemented in-tree so the contract stays dependency-free and
reproducible. The package declares `rust-version = "1.88"` because the in-tree
SHA-256 uses `slice::as_chunks`, which stabilised in Rust 1.88.
