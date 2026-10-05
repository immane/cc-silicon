# Compiler Development-Phase Guardrails (Integrator Contract Supplement)

Status: **ACCEPTED as a user-authorized integrator contract supplement
(policy-level, 2026-10-04).** The user explicitly authorized writing this
guardrail into the compiler application contract. It is binding on new
compiler-development work as user-directed policy.

It is **not** a bus/schema freeze and it is **not** a `/6` acceptance. Every
pipeline, bus, schema, manifest, routing, snapshot, limits, or error-code
mechanism named or implied here remains **PROPOSED and inert** until the T01
integrator reconstitutes them in an accepted contract and the `/6` freeze lands.
This supplement **does not override** the frozen `t01-c01-c06/5` artifact, the
accepted [ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md)
decisions, or any accepted higher-level contract.

Authority order (unchanged, `AGENTS.md` §2): explicit user instruction >
accepted ADR > accepted engineering contract > architecture design > existing
implementation. This supplement sits at the user-instruction level for its
guardrail policy; where it describes a shared interface change it is subordinate
to the applicable accepted contract until that contract is amended.

Scope: compiler application development under `compiler/` (crate
`cc-silicon-compiler`). The root `cc-silicon` framework — `#![forbid(unsafe_code)]`
and its flat bus data (fixed arrays, no heap) — is unchanged; the fixed-layout /
no-heap rule is a bus-data constraint, not a whole-framework guarantee
([ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md) §2,
[SILICON_PARADIGM_SPEC.md](../architecture/SILICON_PARADIGM_SPEC.md),
[SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md)).

Cross-references (existing paths only):
[T01](T01_COMPILER_CONTRACT.md), [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md),
[docs/tasks README](README.md), [TASK_TEMPLATE.md](TASK_TEMPLATE.md),
[T02](T02_CONTROL_CHIPS.md),
[ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md),
[M1 Part A Contract Proposal](M1_PART_A_CONTRACT_PROPOSAL.md),
[`compiler/contracts/CONTRACT_VERSION`](../../compiler/contracts/CONTRACT_VERSION).

---

## 1. Binding Priority Order

When guardrails conflict and cannot all be satisfied at once, resolve in this
order (higher wins). Record the trade-off in the delivery/handoff, do not hide
it.

1. **Correctness** — C semantics, frozen target model, determinism, and explicit
   failure over fabricated success.
2. **Contract compliance** — obey the frozen contract version/hash, manifests,
   task/result protocol, and ownership.
3. **Low coupling** — narrow projections and typed proposals; no hidden edges.
4. **Determinism** — stable IDs and explicit ordering; identical state/pins
   produce identical results.
5. **Ownership** — single writer per field/record; no duplicated ownership.
6. **Dependencies** — declared, acyclic, and resolvable through the bus, not
   through imports or calls.
7. **Future fusion / parallel / incremental capability** — design so a future
   optimizer or scheduler *can* fuse, parallelize, or execute incrementally,
   without requiring it now.
8. **Runtime speed** — optimized only behind a measured, explicit task;
   never traded against items 1–7.

Performance is deliberately last. A faster wrong compiler is not progress.

## 2. Chip Model and Isolation

1. **Zero-field chips only.** Every semantic chip is a unit `RestrictedChip`
   with `compute(&self, &Input) -> Output`; no instance state, no interior
   mutability.
2. **Independently understandable and testable.** A chip must be reviewable and
   testable in isolation from its declared task payload, projection, and typed
   proposal, using synthetic fixtures with independently computed expected
   values. Tests must not be self-verifying.
3. **No direct chip calls.** Chips never call, import, or hold a handle to
   another chip. Coordination is only through typed bus records, task
   requests/results, and wires.
4. **No hidden dependencies, state, or caches.** All semantic state lives in the
   bus. Wires are reset every tick. There is no global mutable state, no semantic
   memo/global cache, and no cross-tick value outside an explicit bus register.
5. **No undeclared accesses.** A chip reads/writes only its field-scoped
   manifest set. "Reads the whole bus" and "writes the store" are not adequate
   contracts; refine to exact fields or pause and request an interface freeze.
6. **No cross-chip implementation imports.** Importing another chip's module for
   its types or helpers to perform language work is prohibited. Mechanical
   helpers (ID/bitvector/intern/serialization) are shared; language rules are
   not.
7. **No duplicated ownership.** Exactly one owner per store/field/record family.
   A second writer is a `StoreOwnerViolation` class defect, not a shortcut.
8. **No I/O, environment, clock, randomness, or subprocess access in chips.**
   Chips read frozen pins/projections and emit proposals; the Host crate owns
   file access, environment sampling, subprocess execution, and artifact
   persistence (`AGENTS.md` §3 item 7, T01 §4.1). A chip that reads the filesystem,
   environment, wall clock, RNG, or spawns a process is a defect, not a shortcut.
9. **No shortcuts.** Test special-casing, filename-based behavior, returning
   success for unimplemented behavior, deleting tests to pass, and delegating
   candidate compilation to GCC/Clang are prohibited (`AGENTS.md` §6,
   [T00](T00_GCC_TORTURE_GATE.md)).
10. **No unnecessary serialization or large cloning.** Do not serialize/parse a
   whole store to move a value, and do not clone large records to pass a
   reference. Use stable IDs and narrow projections. (This is distinct from the
   required deterministic snapshot/hash, which is a contract feature.)

## 3. Shared Interfaces and Contract Changes

1. **No unauthorized bus/schema edits.** Chip owners must not modify the
   `CompilerBus`, task/result enums, ID families, record schemas, routing tables,
   workspace/Cargo configuration, or `mod.rs`. Those are integrator-owned
   ([PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) §2).
2. **Shared interface changes require a complete `CONTRACT_CHANGE_REQUEST` and
   prior acceptance before any edit.** If a chip needs a field, kind, error code,
   or helper that is not frozen, it must submit a change request and wait; it may
   not invent a private incompatible version.
3. **Changes affecting multiple chips are contract change requests**, never a
   unilateral cross-file edit.
4. A complete `CONTRACT_CHANGE_REQUEST` contains at least:
   - ID/title, date, author, and target contract version/hash
     (`t01-c01-c06/5` today);
   - the concrete requirement/motivation and the smallest sufficient change;
   - exact current vs proposed shape of each affected shared interface, store,
     field, enum, schema, and file;
   - affected chips, task kinds, manifests, and dependent tests that must be
     rerun;
   - read/write manifest, phase, and ownership impact;
   - determinism/replay and snapshot/hash impact;
   - the chip-local alternative considered and why it is insufficient;
   - migration/freeze plan, its `/6` inventory item, and the integrator
     acceptance record (who/when). The record is required before the edit.
5. Until accepted and frozen, the change is draft text only; `/5` remains
   current. Do not silently weaken an existing contract to make a change fit.

## 4. Algorithmic Complexity and Performance Discipline

1. **Reject algorithmic hazards before implementation.** The following are
   defects even when functionally correct, and must be redesigned or explicitly
   bounded and justified:
   - quadratic (or worse) growth in input size, record count, or nesting;
   - unbounded recursion (C recursion must use explicit task frames/cursors, not
     the Rust call stack);
   - repeated parsing/re-decoding of the same source or store region;
   - hidden full scans of a whole store/queue when an index, cursor, or owner
     order suffices;
   - pathological lookup (linear scans in hot paths, hashing with unstable
     iteration order, address-keyed lookup).
2. **Micro-optimizations wait for evidence.** Do not add caching, batching,
   unsafe-like tricks, or fusion for speed without a profiling result or an
   explicit optimization task. Correctness baseline first; measure before
   optimizing; keep the optimization legality-checked, version-guarded, and
   analysis-invalidating (`AGENTS.md` §4).
3. **Bounds are explicit.** Any data-structure change states its expected
   complexity and its configured resource bound so exhaustion is a structured
   diagnostic, never a panic or silent drop.

## 5. Modularity, Fusion, and Traversals

1. **Chip logic stays modular and single-responsibility.** A chip that embeds
   several independent language rules must be split; shared schemas are not a
   place to hide a whole phase.
2. **Fusion groups are declared where appropriate.** When a set of chips is
   intended to fuse, the group and its boundary are declared in the manifest and
   review notes; fusion is never implicit or hidden inside a helper.
3. **Traversals are explicit and discoverable.** Ordering, cursors, worklists,
   and visited/version guards must be visible bus state or declared task state,
   not hidden inside an opaque helper. A reviewer must be able to reconstruct the
   traversal from the contract and manifest.
4. **No premature fusion.** Declaring a fusion-ready boundary is allowed;
   collapsing chips into a monolith before the correctness baseline, profiling,
   and acceptance is not.
5. **Future optimizer freedom.** Design boundaries so a later optimizer *may*
   fuse, parallelize, or incrementally execute work. Enabling that capability is
   a design constraint; actually fusing/parallelizing/incrementing is a future,
   measured, accepted change.

## 6. Deterministic Pipelines, Stages, Cursors, and Joins

This section reconciles the user's pipeline direction with the current freeze.

1. **Pipeline-ready where cross-tick work needs it.** Cross-tick compiler work
   must be modeled as an explicit staged pipeline with explicit stage
   assignment, committed-ID cursors, continuations, and join points — not ad-hoc
   glued state. Cursors/joins live in the bus over committed
   `TaskId`/`ResultId`/`ContinuationId`; they never point at a draft, wire, or
   address.
2. **Design broad stage composition.** Stage composition should be designed for
   the whole frontend-to-artifact flow so stages can later overlap safely; the
   exact stage set/names remain a `/6` residual
   ([ADR-0002](../architecture/ADR-0002-DETERMINISTIC-COMPILER-PIPELINES.md) §2.2,
   [M1 Part A](M1_PART_A_CONTRACT_PROPOSAL.md) §6.2.1).
3. **One ordered atomic commit; no same-tick cross-task drafts.** Multiple
   in-flight dispatches (when enabled) are executed sequentially by the single
   CPU backend with one deterministic ordered atomic commit per tick.
4. **Do not force runtime quota > 1 now.** `max_inflight_per_tick` defaults to
   **1**; quota 1 is the required correctness baseline. A quota `> 1` is a
   **measured, post-freeze, integrator-accepted** change only after the
   correctness baseline and a before/after measurement on the frozen corpus, and
   is never runtime threading. No throughput claim is made.
5. **Inert until frozen.** The pipeline registers are not implemented or
   depended on until the pipeline `/6` amendments are frozen; quota `> 1` work
   is a later, separate, measured change. This supplement requires the
   *architecture* to be pipeline-ready; it does not turn the pipeline on.

## 7. Review Checklist

A delivery/review passes only if every applicable item is checked. Record the
exact read/write manifest, test commands, results, and unsupported list; "it
compiles" is not a review.

**Correctness and determinism**
- [ ] Semantics validated against the frozen target model, not the Rust host.
- [ ] Normal, boundary, invalid/unsupported, replay, and unauthorized-write
      tests exist and pass; expected values are independently derived.
- [ ] Identical initial state/config/pins produce identical results; stable IDs,
      explicit ordering; no hash-map iteration/address dependence.
- [ ] Recoverable faults are typed diagnostics; no panic control flow; no
      unsupported operation reported as success.

**Chip model and isolation**
- [ ] Chip is zero-field, `RestrictedChip`, `compute(&self, &Input) -> Output`.
- [ ] Independently understandable and testable from declared payload/projection.
- [ ] No direct chip calls, no cross-chip implementation imports.
- [ ] No hidden dependency/state/cache/global mutation; all state is bus state.
- [ ] No undeclared read/write; the manifest is field-scoped, not whole-bus.
- [ ] No duplicated ownership; single writer per field/record.
- [ ] No shortcuts, no special-casing tests, no GCC/Clang delegation.
- [ ] No unnecessary serialization or large cloning.

**Contract and interfaces**
- [ ] Frozen contract version/hash honored; no private incompatible types.
- [ ] Any shared-interface change has a complete, **accepted**
      `CONTRACT_CHANGE_REQUEST` before edit; multi-chip changes go through the
      integrator.
- [ ] Ownership/files per [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md) §2;
      `Cargo`/`mod.rs`/bus/task enums untouched.

**Complexity and performance**
- [ ] No O(n²)+ blowup, unbounded recursion, repeated parsing, hidden full
      scans, or pathological lookup; bounds are explicit.
- [ ] No performance micro-optimization without profiling or an explicit
      optimization task.

**Modularity and future capability**
- [ ] Chip logic modular/single-responsibility; fusion groups declared where
      appropriate; traversals explicit and discoverable; no premature fusion.
- [ ] Cross-tick work uses explicit stage/cursor/join state over committed IDs.
- [ ] Quota kept at 1 unless a measured, accepted change says otherwise.

**Evidence and claims**
- [ ] Exact commands, results, and unrun checks reported; limitations listed.
- [ ] No universal-correctness, torture-pass, or throughput claim from tests or
      stubs.

## 8. Non-Claims

- This supplement authorizes no implementation, no C language chip, no code
  generation, and no `/6` freeze. It changes no existing file.
- No bus/schema/pipeline mechanism described here exists or is frozen; `/5`
  remains current.
- No throughput or performance improvement is claimed; quota 1 is the baseline.
- Target identity is frozen but concrete ABI values remain UNVERIFIED
  ([ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md)); codegen stays
  fail-closed.

## 9. Revision Record

| Date | Change | Authority |
|---|---|---|
| 2026-10-04 | Initial development-phase guardrail contract supplement: binding priority order; chip isolation; contract-change-request gate; algorithmic-hazard rejection and profiling discipline; modularity/fusion/traversal rules; pipeline-ready-but-quota-1 reconciliation; review checklist. Policy-level, user-authorized; every `/6` mechanism remains PROPOSED and requires T01 integrator integration. No existing file changed | User-authorized integrator contract supplement |
