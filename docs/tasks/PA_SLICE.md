# Wave 2 Slice 3: PA Translation-Unit Freeze (`/12`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-05** as `t01-c01-c06/12` (hash `246d37cc…f12b3d`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/11` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/12` delta), [T05](T05_PARSE_CHIPS.md), [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-PA-01`–`M1-PA-09` (selected rows) |
| Non-goal | Non-M1 syntax, declarator/type interpretation (TY20), scope entry (T06), semantic facts (T07), full-catalog chip split |

## 1. What this slice is

One worker (`PaTuChip`, PA01 slice scope) turning 13 committed C tokens
into the nine-node M1 tree in a single pre-order batch: TU →
FunctionDefinition → (Specifiers, Declarator, Compound → Return →
BinaryAdd → two IntLiteral leaves). Integer leaves reference committed
literals; the declarator carries the committed `main` name; every other
token sequence is explicit `Unsupported`. The acceptance fixture is
**`P1-PA-01`** (below). The File-Enter edge stays deferred to the T06
slice: the TU root is committed first (OPEN-01 direction) with no scope
edge emitted.

## 2. Freeze items

| # | Item | Acceptance |
|---|---|---|
| P2-1 | `NodeRecord { kind, parent, children, first_token, last_token, name, literal }`, `NodeKind` M1-closed (8 variants) | struct + snapshot round-trip |
| P2-2 | Nine-node M1 tree shape with pre-order prediction (TU first) and reciprocal parent/children coherence | exact tree + RL-02 coherence test |
| P2-3 | `parse.translation_unit` (`group 4`, local `16`, `Frozen`) + `pa_slice()` registry | registry + kind/status tests |
| P2-4 | Stage row (`translation_unit → 2`) + allowlist row (`PA_TU_CHIP`, `Parse/nodes`) + `PA_TU_CHIP = ChipId(7)` | stage/allowlist/layer gates green |
| P2-5 | `AppendRecords` materialization for the `Node` family (1:1 bodies, per-arena capacity, predicted refs) | capacity + `UnpredictedRecord` via shared paths |
| P2-6 | Snapshot bodies + `NODE_KIND_NAMES`/`NODE_RECORD_FIELDS` hash participation | encode round-trip + replay determinism |
| P2-7 | Worker template: `PaTuChip` (`PaTuInput` projection, pure `compute`, ZST, stage/layer, lint) | `c12_parse` 5 tests green |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c12_parse.rs`
(5 tests: kind/stage/allowlist freeze, exact nine-node tree with coherence,
truncated/dangling/missing-literal negatives, stage/layer + manifest gates,
snapshot replay determinism). The chain input comes from the real LX slice
(intern → classify → decode); the PP layer stays seeded.

## 4. Explicitly deferred

- Full-catalog split (PA02–PA38); non-M1 syntax (all explicit `Unsupported`).
- `TokenRecord.literal` forward link (still deferred; back-link suffices).
- File-Enter edge and `M1-START-01` (T06 slice; TU-first ordering already holds).
- Commit-side node-link validation (worker-enforced + test-pinned; T13 VF05 pending).
- Declarator/type interpretation (TY20), semantic facts (T07), IR lowering (T09).
