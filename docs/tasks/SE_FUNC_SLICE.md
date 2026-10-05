# Wave 3 Slice 14: SE29 Function-Definition Freeze (`/39`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/39` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/38` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/39` delta), [T07](T07_SEMANTIC_CHIPS.md) SE29 row |
| Non-goal | Non-`(void)` signatures, multi-statement bodies, K&R definitions, prototype-mismatch plans, `Return`-role facts on the definition record |

## 1. What this slice is

One semantic worker checking the M1 function definition:
`SeFuncChip` (SE29) reads one committed `FunctionDefinition` node
(`int main(void){return 2+3;}`) and checks the M1 `(void)`-only
declarator shape (no K&R identifier list), the single committed TY17
`int(void)` signature, the declared `main` symbol, and the committed
`Return` child fact (which already carries its `int`-typed
`SemRecord`; the `Return` node owns the return-role facts). It then
appends one signature-carrying `SemRecord` (no `Return`-role plan) or
reuses the committed one when it already exists (exactly-one per
node). Anything outside the M1 checked shape fails explicitly, never
a fabricated plan. The acceptance fixture runs the real
LX→PA→TY→declare→SE-literal→SE-binary→SE-return chain ending in **one
committed function `SemRecord` plus a reuse pass**.

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| Complete M1 definition (checked return child, declared symbol, single `int(void)` signature) | `Record(Sem)` (signature carrier: `ty` = signature, `NonLvalue`, zero effects) |
| Same node dispatched twice | `Record(Sem)` of the same id (reuse, no duplicate) |
| Return statement unchecked (missing child fact) | `Fail` (task group: missing input, never a plan) |
| Declarator with an identifier list (K&R) | `Fail` (`Unsupported`, never silently accepted) |
| Symbol typed `int` against the `int(void)` signature | `Fail` (`Unsupported`: definition inconsistent with its prototype) |
| Duplicate function type / duplicate declarator symbol | `Fail` (task group: ambiguous handoff, DEFECT) |
| Effectful return child fact | `Fail` (task group, never recorded) |
| Non-running task, wrong kind, wrong arity, non-node payload | `Fail` (protocol fault) |
| Missing node / missing specifier / undeclared function / untyped symbol | `Fail` (task group: missing input) |
| Multi-statement body, non-return body, non-`int` return, non-M1 signature | `Fail` (`Unsupported`) |
| Wrong stage layer, pre-`/39` registry | Driver/gate `Fail` (never silent) |

## 3. ID arbitration (frozen)

The delivered draft (`se_function.rs`) defined file-local
`SE_FUNC_CHIP` (`ChipId(57)`) plus a `semantic_function_def_kind()`
helper. The integrator verified the `/38` head (no `SEMANTIC` local
past 20, `VF14_CHIP = ChipId(56)`) and froze the draft IDs linearly
with no logic change beyond kind/chip-id/test-path repointing:

- Kind: `function_def` (local 21) — `SEMANTIC` owners start new codes
  at local 22.
- Chip: `SE_FUNC_CHIP = 57`.
- The file's draft consts (`SE_FUNC_CHIP`,
  `semantic_function_def_kind()`) are re-pointed at the frozen
  canonical (`SE_FUNC_TASK_KIND` alias, manifest chip ID, test path →
  `c39_sefunc.rs`); header rewritten from candidate-UNWIRED to frozen
  `/39`. No chip logic changed beyond kind/chip-id/test-path
  repointing.

## 4. Frozen registration

- `function_def` (local 21) — first free code after SE21
  `return_stmt` (local 20), `Frozen`; `se_function_slice()`
  registry (69 entries, cumulative over `vf_evidence_slice()`);
  stage row (`→ 4`, with the SE slice); routed layer 4;
  `SE_FUNC_CHIP = 57` (appends one `sem.records`; one allowlist row
  for `SEMANTIC_FUNCTION_DEF`; `STORE_OWNER_ALLOWLIST` grows 32 →
  33); SE-slice schema reuse (no new fields — every read field
  already declared).
- Hash rules: `se.function-signature`, `se.function-body-checked`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| F1-1 | Normal signature check commits the signature carrier (node, `int(void)` ty, `NonLvalue`, zero effects) | signature case |
| F1-2 | Committed record reused without append (exactly-one per node) | reuse case |
| F1-3 | Unchecked return body fails (task group), nothing appended | missing-body case |
| F1-4 | K&R identifier list is `Unsupported` (never silently accepted) | K&R case |
| F1-5 | Prototype mismatch is `Unsupported` (never a recorded plan) | mismatch case |
| F1-6 | Registration freeze (kind/registry/stage/layer/chip/allowlist row) | freeze green |
| F1-7 | Bus dispatch + snapshot replay determinism | determinism case |
| F1-8 | Wrong layer refused; pre-`/39` registry rejects the manifest | gates case |

## 6. Execution record

Delivered as one untracked chip file (`se_function.rs`) against the
`/38` tree; integrated by the T01 integrator: verified the `/38` head
(no `SEMANTIC` local past 20, chip 57 free after
`VF14_CHIP = ChipId(56)`), froze local 21 / `ChipId(57)` (§3) into
`task.rs` (`se_function_slice()`, 69 entries), `SE_FUNC_CHIP` +
`is_se_function_slice_kind` + one stage row + one allowlist row into
`manifest.rs`, two rule ids into `contract.rs`, bumped to
`t01-c01-c06/39` with recomputed hash (`07f4eade…f224`), wired
`semantic/mod.rs` + `chips/mod.rs` (pure cores re-exported), and
re-pointed the draft's kind/chip-id/test-path at the frozen canonicals
(header rewritten from unwired to frozen `/39`). No chip logic changed
beyond kind/chip-id/test-path repointing. Verified by
`compiler/tests/c39_sefunc.rs` (8 tests: freeze, normal signature,
reuse, missing body, K&R, mismatch, determinism, stage-layer gates).
Full §5 suite green at commit. Superseded pins updated by the
integrator (never the chip owner, never silent): `freeze.rs`,
`c08_gate1` (version, 69 stages, 33 allowlist rows, new rule/marker
pins), `c04_manifest` + `c32`–`c38` allowlist lenses, `c20_ppscan` +
`h04_candidate` version markers, both READMEs,
`T01_COMPILER_CONTRACT.md` §7.1, `CHIP_PLAN.md`, `context.md`.

## 7. Explicitly deferred (all loud, never silent)

- Non-`(void)` signatures (parameters, `...`, unprototyped): the
  projector requires exactly `Function { result: int, params: [],
  prototype: true, variadic: false }`, else `Unsupported`.
- Multi-statement bodies and non-return statements: exactly one
  `Return` child is required, else `Unsupported`.
- K&R definitions and prototype-inconsistent definitions: explicit
  `Unsupported`, never a conversion plan (plans arrive with
  non-identity conversions).
- `Return`-role facts on the definition record: the appended record
  carries the signature only (T07 S6 role split; the `Return` node
  owns the return-role facts).
- The SE30+ remainder (unchanged).
