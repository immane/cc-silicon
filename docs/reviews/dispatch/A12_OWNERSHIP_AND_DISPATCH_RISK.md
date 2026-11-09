# A12 — Ownership and Dispatch-Risk Review

| Field | Value |
|---|---|
| Status | **Integrator synthesis after two upstream-capacity failures.** This report reuses the independently verified A04/A13 findings and the integrator's source checks; it does not claim a fresh full audit by the failed A12 worker. |
| Date | 2026-11-07 |
| Task | A-REV-01 / B1-04 |
| Baseline | `wave4-development`, HEAD `430ae29`, contract `t01-c01-c06/39`, target unverified |
| Inputs | `docs/reviews/dispatch/A04_PARSE_TYPE_SEMANTICS.md` §7; `docs/reviews/dispatch/A13_TESTS_CI_DOCS.md` §5-7; `compiler/src/chips/parse/mod.rs`; `compiler/src/chips/lex/mod.rs`; current `git status` |
| Write scope | This report only. No source, test, contract, Cargo, `mod.rs`, user draft, or ledger was modified. |

## 1. Baseline

The worktree contains user-owned uncommitted work:

- Modified: `compiler/src/chips/parse/mod.rs` (five added `mod` lines).
- Untracked parse drafts: `pa_cast.rs`, `pa_decl_spec.rs`, `pa_expr_ext.rs`,
  `pa_ext_dispatch.rs`, `pa_init.rs`, `pa_stmt.rs`, `pa_typedef_query.rs`.
- Untracked lex drafts: `lx_error.rs`, `lx_extension.rs`.
- Untracked freeze/plan drafts under `docs/tasks/`.

Five parse drafts are compiled through the modified `parse/mod.rs`; two parse
drafts and both lex drafts are not compiled at all. None of the nine drafts is
registered, exported, routed, manifested, or covered by an integration test.
This report treats all of them as **parked user work**, not as Stage-B producers.

## 2. Collision and Duplicate-Producer Map

Frozen `PARSE` locals end at 26 (`PARSE_RECOVERY`); frozen chip IDs end at 57
(`SE_FUNC_CHIP`). Every draft independently claimed the next-free-looking values.

| Resource | Claimants | Risk |
|---|---|---|
| `PARSE` local 27 | `pa_cast` PA21, `pa_decl_spec` PA08, `pa_expr_ext` PA15, `pa_ext_dispatch` PA33, `pa_init` PA15, `pa_stmt` PA27, `pa_typedef_query` PA04 | Seven-way duplicate; registration would reject or silently shadow |
| `PARSE` local 28 | `pa_cast` PA23, `pa_decl_spec` PA10, `pa_expr_ext` PA26, `pa_ext_dispatch` PA34, `pa_stmt` PA29 | Five-way duplicate |
| `PARSE` local 29 | `pa_cast` PA25, `pa_ext_dispatch` PA35, `pa_stmt` PA30 | Three-way duplicate |
| `PARSE` local 30 | `pa_ext_dispatch` PA36, `pa_stmt` PA31 | Two-way duplicate |
| `PARSE` local 31 | `pa_ext_dispatch` PA37 | Single claimant, still unfrozen |
| `ChipId(58)` | All seven parse drafts plus `lx_extension` LX15 | Eight-way duplicate |

Duplicate capabilities:

- **PA15 initializer** has two implementations: `pa_init.rs` (compiled, focused)
  and `pa_expr_ext.rs` (not compiled). Only one may become canonical.
- **PA27 statement dispatch** duplicates the frozen PA28/PA32 acceptance shapes
  and produces no AST; it is not a safe new producer.
- Several drafts contain latent `Ok -> Complete(Ack)` arms even though their
  parsers currently return `Unsupported` (`pa_cast.rs`, `pa_stmt.rs`). Lifting
  the parser alone would fabricate success without a record producer.
- PA08/PA10/PA04/PA15 drafts are Ack-only: registering them without parent/node
  integration would create successful tasks with no parser output.

## 3. Shared-File Ownership

These paths remain integrator-only in every Stage-B batch:

- `compiler/src/chips/parse/mod.rs`, `compiler/src/chips/lex/mod.rs`,
  `compiler/src/chips/mod.rs`
- `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`,
  `compiler/src/ids.rs`, `compiler/src/bus.rs`
- `compiler/Cargo.toml`, root/nested Cargo configuration
- Shared test entry points and any `mod`/registration change

A draft author may own only the individual draft file and a dedicated new test
file. A worker needing a shared row must return a DEFECT or a contract-change
request; it must not invent a private local/chip ID.

## 4. Recommended Ownership Map

| Area | Owner | Paths |
|---|---|---|
| Shared task kinds, chip IDs, stage/allowlist/hash | Integrator | `task.rs`, `manifest.rs`, `contract.rs`, `ids.rs`, `CONTRACT_VERSION` |
| Parse module wiring/exports | Integrator | `chips/parse/mod.rs`, `chips/mod.rs` |
| PA01 exact-shape fix (B1-01) | Parse owner | `chips/parse/pa_tu.rs`, new `tests/c40_pa_guard.rs` |
| Canonical PA15 initializer | One parse owner after freeze | Prefer `pa_init.rs`; park `pa_expr_ext.rs` until explicitly retired |
| PA04 typedef query | One parse/T06 owner after freeze | `pa_typedef_query.rs` plus T06 lookup facts |
| PA08/PA10 declarator support | One parse owner after freeze | `pa_decl_spec.rs` |
| PA21/PA23/PA25 expressions | One parse owner after freeze | `pa_cast.rs`; remove latent Ack arms before registration |
| PA27/PA29-31 statements | One parse owner after V2 freeze | `pa_stmt.rs`; do not duplicate PA28/PA32 |
| PA33-37 extension shells | One parse/T12 owner after freeze | `pa_ext_dispatch.rs` |
| LX15/LX18 | One lex owner after freeze | `lx_extension.rs`, `lx_error.rs` |
| Verification/candidate changes | Integrator + T13/T09 owners | `chips/verify/**`, `chips/ir_lower/**`, `bin/candidate.rs` |

## 5. Safe Dispatch Boundaries

**B1 (no new schema), already dispatched:** PA01 exact validation, candidate
non-UTF-8 argv hardening, docs reconciliation, chip-lint tree tests, orphan
source guard, this ownership report.

**B2 (contract change requests only):** determinism/limits/replay and M1 A3
SE29. No draft registration, no shared `mod.rs` change.

**B3 (after ID/task-kind freeze):** choose the canonical PA15 implementation and
allocate distinct locals/IDs for PA04, PA08/PA10, PA21/PA23/PA25, and the
extension shells. Each owner gets one draft file and one new integration test.
Do not register the Ack-only drafts without a real node/semantic producer.

**B4 (after V1 AST/type/semantic freeze):** wire the selected parse producers
into `parse/mod.rs`, `chips/mod.rs`, manifests, routing, and the candidate
driver. The integrator performs the shared-file edits in one batch; individual
owners do not edit registration.

**B5 (after V2/V3 freezes):** statements/control flow and declarators/calls.
PA27 must not duplicate frozen PA28/PA32; PA29-31 must wait for V2 kinds and CFG.

## 6. Blocking Defects and Required Decisions

1. **Draft disposition:** park the five compiled drafts (remove their temporary
   `mod` lines) or export/register them after a freeze. Do not add blanket
   `#[allow(dead_code)]`; it would hide unregistered producers.
2. **PA15 canonical choice:** `pa_init.rs` is the recommended starting point;
   `pa_expr_ext.rs` should stay parked until explicitly retired.
3. **PA27 disposition:** keep frozen PA28/PA32 as the M1 statement validators;
   do not register a duplicate Ack-only PA27.
4. **ID allocation:** the integrator must allocate distinct `PARSE` locals and
   `ChipId`s, update `TaskKindRegistry`, `STAGE_ASSIGNMENT`, routing, manifests,
   allowlists, and the contract hash together.
5. **Chip boundary:** the Worker-vs-RestrictedChip decision (A02 F01) remains
   open and blocks a uniform Stage-B template.
6. **Verification ownership:** VF06 must be extended by the T13 owner under the
   M1 A3 CCR; a parser owner must not edit it directly.

## 7. Conclusion

The drafts provide useful pure helpers and test material, but none is a
registered capability. The safe parallel unit is **one draft file + one new
test file**, with the integrator owning every `mod.rs`, ID/task-kind row, and
registration change. B1/B2 can proceed now; B3-B5 require the named freezes.
