# B-INT-00 — Unregistered parse/lex draft disposition

| Field | Value |
|---|---|
| Status | **COMPLETE / INTEGRATOR VERIFIED (2026-11-09)** |
| Scope | D-04 (clippy red baseline) and D-05 (duplicate candidate IDs / two PA15 drafts) |
| Decision | Park every unregistered draft; preserve source; compile/register none; freeze no candidate ID |
| Authority | T01 integrator dispatch disposition; no contract freeze, no schema change, no capability claim |

## 1. Problem

`compiler/src/chips/parse/mod.rs` compiled five unregistered parse drafts
(`pa_cast`, `pa_decl_spec`, `pa_ext_dispatch`, `pa_init`, `pa_stmt`). They
failed `clippy -D warnings` with 137 diagnostics and claimed colliding
candidate IDs (`PARSE` local 27 and `ChipId(58)`, among others). Four more
drafts were already unreferenced. None had a frozen kind, stage, routing row,
allowlist row, or manifest; none was reachable from the candidate driver.

## 2. Disposition

| File | Scope | Decision | Reason |
|---|---|---|---|
| `lex/lx_error.rs` | LX error draft | Park | Unregistered; no frozen kind/registration |
| `lex/lx_extension.rs` | Lex extension draft | Park | Unregistered; no frozen kind/registration |
| `parse/pa_cast.rs` | PA21/PA23/PA25 draft | Park | Unregistered; candidate IDs collide |
| `parse/pa_decl_spec.rs` | PA08/PA10 draft | Park | Unregistered; candidate IDs collide |
| `parse/pa_ext_dispatch.rs` | PA33–PA37 draft | Park | Unregistered; candidate IDs collide |
| `parse/pa_init.rs` | PA15 initializer draft | Park (future PA15 candidate) | Unregistered; intended canonical PA15 direction, but IDs are not frozen |
| `parse/pa_stmt.rs` | PA27/PA29–PA31 draft | Park | Unregistered; candidate IDs collide |
| `parse/pa_expr_ext.rs` | PA15 + PA26 draft | Park (non-canonical) | Duplicate PA15 direction; must not be wired alongside `pa_init.rs` |
| `parse/pa_typedef_query.rs` | Typedef-query draft | Park | Unregistered; no frozen kind/registration |

Parking is implemented by removing the five `mod` declarations from
`compiler/src/chips/parse/mod.rs` and listing all nine files in the exact
`DRAFT_FILES` allowlist of `compiler/tests/c42_wiring.rs`. Source files are
not deleted or rewritten.

## 3. D-04 / D-05 disposition

- **D-04 closed.** The draft diagnostics no longer enter the clippy gate. The
  only remaining clippy failure was a pre-existing `type_complexity` in
  `c40_pa_guard.rs`, fixed by an integrator-owned `GuardCase` type alias.
- **D-05 dispositioned, not a parser freeze.** No parked file is compiled or
  registered, so the duplicate `PARSE` local 27 / `ChipId(58)` values are
  inactive. `parse/pa_init.rs` is the designated **future** PA15 candidate;
  `parse/pa_expr_ext.rs` is explicitly non-canonical. No candidate ID is
  frozen, allocated, or routed. B3-04 still owns the canonical PA15 / parse-ID
  freeze before any wiring.

## 4. Verification

| Check | Result |
|---|---|
| `cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings` | **PASS** (clean) |
| `cargo test --locked --manifest-path compiler/Cargo.toml --test c42_wiring` | **PASS**, 1 test |
| `cargo test --locked --manifest-path compiler/Cargo.toml` | **PASS**, 684 tests / 51 suites (mixed-tree with `/42`) |
| Isolated `/41` + B-INT-00 test snapshot | **PASS**, 676 tests / 50 suites; parking removes 47 draft-module unit tests from the compiled suite while preserving their source |
| `cargo fmt --manifest-path compiler/Cargo.toml -- --check` | **PASS** |

## 5. Non-claims

No parser capability is implemented or accepted. The parked drafts are user
work preserved for future review, not evidence of PA15/PA21/PA26/PA33–PA37
support. No contract version/hash change is made by this disposition.
