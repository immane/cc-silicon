# OP_FREEZE_01: OP01 PassSelect + OP11 UseDef Freeze + Design (DRAFT ONLY)

Status: **OPEN / DRAFT ONLY — NOT FROZEN, NOT ACCEPTED.**

Scope: freeze+design proposal for **T10 OP01 PassSelectChip** and **T10 OP11 UseDefAnalysisChip** only. This document authorizes **no chip code, no contract freeze, no `/6` change**. All schemas, matrices, and rules below are **candidate** pending T01-integrator co-freeze.

Prerequisites (version-guard, must hold before any freeze):
- T01 foundation accepted version frozen (`t01-c01-c06/6` or successor named explicitly at freeze time); any T01 amendment re-opens this draft.
- T09 base IR shape frozen sufficiently to name function/block/instruction/value IDs, terminators, and non-SSA join-slot representation (T09 IR base explicitly permits non-SSA; OP11 must not assume SSA).
- No implementation may be claimed from this draft; T00 corpus/option freeze remains pending.

## 1. OP01 PassSelect: OptConfig / PassPlan (candidate)

### 1.1 OptConfig (candidate frozen input, read-only to OP01)

Proposed fields (names candidate, types unspecified pending T01 co-freeze):

- `opt_level`: enum `{ O0, O1, O2, O3 }` — derived from driver `-O` flag; unknown/missing flag handling is a Host/driver concern, OP01 receives only the typed value.
- `extra_flags`: explicit per-pass enable/disable overrides (e.g. `-fno-xxx` / `-fxxx`), each with a defined default of "no override". Unknown option strings must arrive as a typed `UnknownOption` value, never silently dropped.
- `target_mode`: frozen target identity reference (T01 §6 / ADR-0001); OP01 does not interpret ABI values, only gates target-conditional passes on equality.
- `config_version`: schema version of OptConfig itself; mismatch is a typed diagnostic, never silent fallback.

Non-goals: `-Os`, `-Og`, `-Oz`, link-time / profile-guided options are **out of scope** for this freeze; if present they must surface as `UnsupportedOption`, not map silently onto O1/O2.

### 1.2 PassPlan (candidate output, read-only to downstream passes)

Proposed shape: ordered list of pass IDs drawn from the T10 OP02–OP22 set (subset only — no inlining/unroll/vectorize passes invented here), plus per-pass enable flag and deterministic order index. Plan construction is a pure function of `(OptConfig, frozen pass catalog version)`; identical inputs yield identical plans.

### 1.3 Level matrix (candidate, -O0..-O3)

| Level | Semantics | Candidate pass set (ordered) |
|---|---|---|
| `-O0` | No optimization; analyses may still run read-only for verification but no rewrite pass is enabled. | Empty rewrite plan. OP11 UseDef may run (read-only). |
| `-O1` | Local, always-safe scalar cleanups only. | OP02 integer-fold, OP04 identity (non-FP-risky subset), OP07 fold-branch, OP09 dead-instruction (pure-local subset), OP20 apply + OP21 invalidate + OP22 fixpoint scaffolding. |
| `-O2` | Full local + CFG cleanup +CopyProp/GVN working set. | O1 set + OP05 copy-propagate, OP06 local value numbering, OP08 unreachable removal, OP10 CFG simplify. FP folding (OP03) stays conservative: default off for any rule that changes NaN/−0/fenv behavior. |
| `-O3` | Same as O2 for this freeze. No additional aggressive passes are defined here; any future aggressive pass requires its own chip row + legality proof and must not ride silently on `-O3`. | Identical to O2 candidate set; differences reserved for future named passes. |

Rules:
- Every enabled rewrite pass must carry its legality guard reference (T10 table acceptance column); enabling a pass without its guard is a freeze rejection.
- Unknown `-O` spellings / combined flag conflicts resolve to typed `UnknownOption` / `ConflictingOptions` diagnostics; OP01 must never silently coerce to O2.
- Debug policy: PassPlan must be traceable — plan dump includes `(opt_level, config_version, catalog version, per-pass on/off + reason)`.

## 2. OP11 UseDef: read-only analysis schema (candidate)

### 2.1 Role and purity

OP11 is a **read-only analysis**: it computes def-use / use-def facts from a frozen `(FunctionIR, ir_version)` snapshot and emits a new `UseDef` fact record. It **writes no IR**, mutates no instruction, and bumps no IR version. All downstream consumers (OP05/OP06/OP09/OP20) read the facts keyed by `(function_id, ir_version)`; a version mismatch is stale and must be rejected, never used.

### 2.2 Candidate `UseDef` schema (field names candidate)

- `function_id` + `ir_version`: the exact snapshot this fact set was computed from.
- Per-instruction entries keyed by stable instruction ID:
  - `defs`: list of value IDs defined (0, 1, or N for call/aggregate cases as frozen by T09).
  - `uses`: ordered list of value IDs read, each tagged with operand role: `ordinary | phi | terminator | memory-effect`.
  - `memory_class`: `pure | reads-memory | writes-memory | unknown-effect` (conservative; unknown means not-pure).
- Aggregate index: `def_site(value_id) -> instruction_id`, `use_sites(value_id) -> [instruction_id]`; every use must resolve to exactly one in-snapshot def or to an explicit `param | global | unknown-entry` source — dangling uses are a typed error, never an empty list passed as success.
- Non-SSA note: multiple defs of the same slot across blocks are permitted (T09 join slots); OP11 records each def site separately and does not invent SSA versions (SSA versioning belongs to OP14/OP15 only).

### 2.3 Recompute / invalidation contract

- After any IR mutation (OP20 commit bumps `ir_version`), all `UseDef` facts keyed to the old version are **invalid**; OP21 records the invalidation and consumers must request recompute via `Analyze(function, new_version)`.
- Recompute is deterministic: same `(FunctionIR, ir_version)` yields byte-identical fact ordering (stable instruction-ID order, then operand index order; no hash-map iteration order leaks).

## 3. Version-guard prerequisites (candidate rules)

1. Every OP01 `PassPlan` carries `(config_version, catalog_version)`; every OP11 `UseDef` carries `(ir_version)`; every downstream patch (OP02–OP10 proposals) carries `(ir_version, analysis_version)` pairing the UseDef/Dom/Alias facts it was validated against.
2. OP20 ApplyRewrite accepts a patch only on exact version match; stale patches are rejected whole (no partial apply), and rejection invalidates nothing.
3. Any T01 / T09 schema amendment that renames IR IDs, terminators, or join-slot representation voids this draft's candidate field bindings and returns status to OPEN.

## 4. Acceptance checklist (design only, no code, no run)

- [ ] Candidate `OptConfig` / `PassPlan` / `UseDef` field tables co-frozen with T01 integrator (exact names/types/encodings; numeric tags uninvented here).
- [ ] `-O0..-O3` matrix trace test vectors defined (one plan dump per level + unknown-option / conflict diagnostics).
- [ ] OP11 fact fixtures defined: ordinary/phi/terminator/memory operands; multi-def slot; stale-version rejection; recompute-after-delete determinism.
- [ ] Write-scope proof: OP11 writes only the `UseDef` fact store, OP01 only the `PassPlan` store; unauthorized-field-unchanged check defined.
- [ ] Replay test: identical snapshot + config yields identical plan/facts.

## 5. Explicit non-claims

- No chip implemented; no contract frozen; no pass rate claimed.
- `-Os/-Og` behavior, SSA passes (OP12–OP16), alias/loop analyses (OP17/OP18), and rewrite application (OP20–OP22) are **not** frozen by this document.
- All numeric encodings, enum tags, and store layouts are deliberately uninvented here; they belong to the T01 co-freeze.
