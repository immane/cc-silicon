# Compiler SFL Manifest Extension (T01 C04)

This document defines the compiler application's extension of the SFL schema
draft. It is a **new, application-level schema**; it does not claim that
`docs/architecture/SFL_SCHEMA_DRAFT.md` already ships a validator. The draft is
a document shape; the concrete validator lives in code and is exercised by
tests.

- Types and validator: `compiler/src/manifest.rs`
- Foundation store schema: `StoreSchema::foundation()`
- Tests: `compiler/tests/c04_manifest.rs`

## 1. Relationship to the SFL draft

The SFL draft describes generic pins/wires/bus/chips/backends. This extension
keeps that vocabulary and adds the fields the compiler pipeline needs:

- task guards (`group`, `task_kinds`),
- exact store/field read and write paths (`reads`, `writes`),
- routing intent (`group`, task kinds),
- capability and backend classification (`capability`, `backend_class`),
- phase (`phase`),
- test coverage (`tests`).

A chip that cannot declare these fields cannot enter the compiler pipeline.

## 2. Manifest shape

```text
ChipManifest {
  id: ChipId,                 // unique, stable, never reused
  chip_name: str,             // unique, stable
  group: TaskGroup,           // control/host/preprocess/lex/parse/...
  task_kinds: [TaskKind],     // accepted kinds; each registered, unique owner
  reads: [FieldPath],         // declared store.field paths
  writes: [FieldPath],        // declared store.field paths
  capability: Capability,     // portable|batchable|emulable|device_specific|experimental
  backend_class: BackendClass,// cpu-reference|aarch64-linux
  phase: ChipPhase,           // propagation (latch is forbidden for chips)
  deterministic: bool,        // must be true for default-runtime chips
  tests: [str],               // at least one test path
  dependencies: [TaskKind],   // data dependencies, never chip-to-chip calls
}
```

`FieldPath` is `store.field`, where `store` is one of the compiler storage
partitions from `T01_COMPILER_CONTRACT.md` section 2 (`config`, `control`,
`sources`, `pp`, `lex`, `parse`, `symbols`, `types`, `sem`, `constants`,
`layout`, `init`, `ir`, `opt`, `machine`, `ext`, `tasks`, `diagnostics`,
`artifacts`, `wires`).

## 3. Validation rules

The validator rejects a manifest when:

1. a read path is not declared in the store schema;
2. a write path is not declared in the store schema;
3. the chip declares no task kinds;
4. a task kind is not registered in the task-kind registry;
5. a task kind's group differs from the chip's group;
6. no tests are declared;
7. `deterministic` is false;
8. the phase is `latch` (only the motherboard latches);
9. the capability/backend class contradicts the group rule: target-code chips
   must be `device_specific` + `aarch64-linux`; every other group must be
   `emulable` + `cpu-reference`.

The manifest registry additionally rejects duplicate chip IDs, duplicate chip
names, and two chips claiming the same task kind. Task-kind names are globally
unique in the task-kind registry, so one name can never carry two semantics.

## 4. Bounds and honesty

- The validator is a **lint**, not a proof. It cannot inspect chip code; the
  root `tools/chip-lint` AST linter and review remain necessary.
- Group-owned stores (`pp`, `parse`, `ir`, ...) start with no declared fields.
  A group owner must call `StoreSchema::declare` when freezing its record
  fields; until then a manifest referencing them is rejected, not silently
  accepted.
- Adding a field to the frozen foundation schema (`StoreSchema::foundation()`)
  or a normative rule identifier changes the frozen contract hash in
  `compiler/contracts/CONTRACT_VERSION`; the integrator republishes it in one
  place and dependent packages are retested.
- Two-tier hash scope: the frozen seed (`StoreSchema::foundation()` plus the
  `M1AppendSchema` section frozen at `/6`) participates in the contract hash,
  while post-seed runtime `StoreSchema::declare()` extensions stay excluded
  from that hash (`hash_excludes=... post-seed-declarations ...`, scoped to
  post-seed runtime declarations only) and are captured by the runtime
  snapshot/schema mechanisms instead; declaring them does not by itself change
  the frozen artifact. Per-chip allowlist rows land wave-gated into the seed,
  not here.
- History: at `/5` the boundary was coarser — the whole foundation was the
  seed and every group-declared field added after the foundation via
  `StoreSchema::declare` was excluded from the frozen hash
  (`hash_excludes=group-declared-store-fields`) and captured by the runtime
  snapshot/schema mechanisms instead, without changing the frozen artifact by
  itself. That `/5` behavior and hash are preserved as history; the two-tier
  scope above was applied atomically at `/6` together with the matching
  `hash_excludes` token in `CONTRACT_VERSION`/`contract.rs`/`compiler/README.md`,
  `FrozenSchema::encode`, and the freeze test.

## 5. Commit-time enforcement (beyond the lint)

The manifest is not only documentation: the commit path enforces it per
proposal batch (`compiler/src/commit.rs`). Enforcement is exact per proposal
kind; it is not a uniform producer-manifest check:

- `ManifestRegistry::register` validates the manifest against the frozen store
  schema and the task-kind registry before recording it; an invalid manifest
  never enters the registry;
- every proposal requires its enclosing task to be `Running` and the producing
  chip to be the owning chip of that task. This attribution check does not look
  the producing chip up in the registry, so a task whose owner has no
  registered manifest (for example a bootstrapped foundation task) can still be
  completed, failed, or parked with `AwaitHost` (`compiler/tests/c03_task.rs`,
  `completion_is_exactly_once`);
- a `StorePatch` is the only proposal kind that enforces the producing chip's
  manifest: the producer must be registered and accept the task kind; the patch
  must match the enclosing task and the producing chip; the patched field must
  be declared in that chip's registered **write** set and in the store schema;
  the patch is version-guarded and shape-checked. Negative tests live in
  `compiler/tests/c03_task.rs` (`unregistered_chip_store_patch_is_rejected`,
  `task_kind_must_be_accepted_by_the_manifest`,
  `patch_owner_chip_attribution_and_field_are_checked`);
- an `Enqueue` destination chip must be registered and accept the destination
  kind (bootstrap is integration-only); the producing chip's manifest is not
  consulted for `Enqueue`;
- `Complete`, `Fail`, and `AwaitHost` bind their inner task ID to the enclosing
  task and enforce exactly-once completion; they perform no
  producer-registration or accepted-kind lookup;
- the `config` store is **read-only**: a declared `config` write is rejected at
  manifest registration, and a `config` store patch is rejected at commit;
- a batch is all-or-nothing (no partial commit on validation failure; the apply
  pass performs only infallible appends after a capacity preflight).

A **store-patch** producer whose kind or field is not declared is rejected with
a structured `CommitError`, never silently accepted. `Complete`, `Fail`, and
`AwaitHost` carry no store mutation and are not rejected on that basis; that is
the documented exception for tasks whose owning chip has no registered manifest.
