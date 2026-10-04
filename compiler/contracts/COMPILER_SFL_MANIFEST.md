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
- Adding fields or rules changes the frozen contract hash in
  `compiler/contracts/CONTRACT_VERSION`; the integrator republishes it in one
  place and dependent packages are retested.

## 5. Commit-time enforcement (beyond the lint)

The manifest is not only documentation: the commit path enforces it per
proposal batch (`compiler/src/commit.rs`):

- `ManifestRegistry::register` validates the manifest against the frozen store
  schema and the task-kind registry before recording it; an invalid manifest
  never enters the registry;
- the producing chip must be the owning chip of the task;
- an `Enqueue` destination chip must be registered and accept the destination
  kind (bootstrap is integration-only);
- a store patch must match the enclosing task and the producing chip;
- the patched field must be declared in that chip's registered **write** set;
- the task kind must be accepted by that chip's manifest;
- the `config` store is **read-only** and is rejected both at manifest
  registration and at commit;
- store patches are version-guarded, and a batch is all-or-nothing (no partial
  commit on validation failure; the apply pass performs only infallible
  appends after a capacity preflight).

A producer whose kind or field is not declared is rejected with a structured
`CommitError`, never silently accepted.
