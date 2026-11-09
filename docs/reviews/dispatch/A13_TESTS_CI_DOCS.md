# A-AUD-06: Tests, CI, and Documentation-Status Audit

| Field | Value |
|---|---|
| Task | `A-AUD-06` under `docs/tasks/MASS_PARALLEL_DISPATCH.md` |
| Status | Read-only audit; no source, test, contract, Cargo, `mod.rs`, or CI change |
| Date | 2026-11-07 |
| Baseline | Branch `wave4-development`, HEAD `430ae29`, with the uncommitted files listed in §2 |
| Report path | `docs/reviews/dispatch/A13_TESTS_CI_DOCS.md` |

This report separates **observed facts** (things read from the tree or run in this
audit) from **recommendations** (work orders and opinions). It makes no claim
about a GitHub Actions run, a target probe, a GCC torture run, or compiler test
execution that was not performed here.

## 1. Scope and method

Read in full or in the relevant status sections:

- `AGENTS.md`, `docs/tasks/README.md`, `docs/tasks/MASS_PARALLEL_DISPATCH.md`,
  `docs/tasks/DISPATCH_LEDGER.md`
- `.github/workflows/ci.yml`, `.github/workflows/t00-target-probes.yml`
- `Cargo.toml`, `compiler/Cargo.toml`, `tools/chip-lint/Cargo.toml`,
  `tools/torture/Cargo.toml`
- `compiler/src/chips/` tree and the group `mod.rs` files
- `compiler/tests/`, `tests/`, `tools/chip-lint/tests/`,
  `tools/torture/tests/`, `tools/torture/probe/tests/`
- `README.md` project status and testing sections, `compiler/README.md`
- All files under `docs/guide/`

Targeted commands were run on the macOS development host. The full compiler
suite was **not** executed; test discovery was checked with `--list`, which
compiles the test targets but does not run them. The source tree was not
modified.

## 2. Baseline (observed)

Git state:

- Branch: `wave4-development`; HEAD: `430ae29` (`docs: add mass-parallel compiler dispatch plan`).
- Modified: `compiler/src/chips/parse/mod.rs` (five added `mod` declarations).
- Untracked source: `compiler/src/chips/lex/lx_error.rs`,
  `compiler/src/chips/lex/lx_extension.rs`,
  `compiler/src/chips/parse/pa_cast.rs`,
  `compiler/src/chips/parse/pa_decl_spec.rs`,
  `compiler/src/chips/parse/pa_expr_ext.rs`,
  `compiler/src/chips/parse/pa_ext_dispatch.rs`,
  `compiler/src/chips/parse/pa_init.rs`,
  `compiler/src/chips/parse/pa_stmt.rs`,
  `compiler/src/chips/parse/pa_typedef_query.rs`.
- Untracked dispatch/freeze documents also exist (`DISPATCH_LEDGER.md`,
  `CT_FREEZE_01..03.md`, `H04_I_FREEZE.md`, `OP_FREEZE_01..03.md`,
  `VF13_FREEZE.md`). They are not source changes and were treated as user work.

Observed results:

| Check | Command | Result |
|---|---|---|
| Root fmt | `cargo fmt --all -- --check` | exit 0 |
| Chip-lint fmt | `cargo fmt --manifest-path tools/chip-lint/Cargo.toml -- --check` | exit 0 |
| Compiler fmt | `cargo fmt --manifest-path compiler/Cargo.toml -- --check` | exit 0 |
| Torture fmt | `cargo fmt --manifest-path tools/torture/Cargo.toml -- --check` | exit 0 |
| Root tests | `cargo test --locked` | 21 passed: 0 lib unit, 15 `tests/paradigm.rs`, 6 doctests |
| Chip-lint tests | `cargo test --locked --manifest-path tools/chip-lint/Cargo.toml` | 12 passed |
| Torture tests | `cargo test --locked --manifest-path tools/torture/Cargo.toml` | 73 passed: 15 lib unit, 0 bin unit, 7 `tests/cli.rs`, 51 `tests/verify.rs` |
| Probe self-tests | `bash tools/torture/probe/tests/run-tests.sh` | 57 passed, 0 failed |
| Compiler test discovery | `cargo test --locked --manifest-path compiler/Cargo.toml -- --list` | exit 0; 654 tests listed |
| Compiler clippy gate | `cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings` | exit 101; 142 lint diagnostics (137 lib + 5 lib test) |
| Chip scan | `cargo run --locked --manifest-path tools/chip-lint/Cargo.toml -- compiler/src/chips` | `strict chip lint passed: compiler/src/chips` |

The 654 compiler tests listed are: 471 integration tests across 41
`compiler/tests/*.rs` files, 172 library unit tests, 9 `candidate` binary unit
tests, and 2 doctests. The 142 clippy diagnostics are in five parse draft files
only; the detail is in §6.

## 3. Package and check matrix

`Cargo.toml` has no `[workspace]` section, and neither do the three nested
packages (`Cargo.toml:1-15`, `compiler/Cargo.toml:1-21`,
`tools/chip-lint/Cargo.toml:1-10`, `tools/torture/Cargo.toml:1-15`). Each
package is checked through its own manifest path.

| Package | fmt | clippy `-D warnings` | test | extra CI step |
|---|---|---|---|---|
| Root `cc-silicon` | `ci.yml:24-25` | `ci.yml:36-37` | `ci.yml:48-49` | none |
| `tools/chip-lint` | `ci.yml:27-28` | `ci.yml:39-40` | `ci.yml:51-52` | scan `compiler/src/chips`, `ci.yml:60-61` |
| `compiler` | `ci.yml:30-31` | `ci.yml:42-43` | `ci.yml:54-55` | none |
| `tools/torture` | `ci.yml:33-34` | `ci.yml:45-46` | `ci.yml:57-58` | none |
| Probe self-tests | shell only | n/a | `t00-target-probes.yml:59-67` | path-filtered to `tools/torture/probe/**`, `t00-target-probes.yml:46-53` |
| Target probe | n/a | n/a | manual only, `t00-target-probes.yml:69-145` | never a merge gate |

Observed versus configured:

- All four packages are configured for fmt, clippy `-D warnings`, and tests.
  The CI workflow is a single `rust` job on `ubuntu-latest` with Rust `1.99.0`
  (`ci.yml:11-22`), matching `rust-toolchain.toml:1-3`.
- The compiler clippy command is configured correctly as a hard gate, but it
  fails on the current dirty tree (§6). That is a baseline-cleanliness failure,
  not a missing CI step.
- The torture package **is** in CI now. The README statement that it is not is
  stale (§8).
- The chip-lint scan is in CI and targets the real chip tree. The older audit
  statement that no chip tree exists is stale (§8).
- GitHub Actions itself was not run; all observed results are local macOS
  results and are not evidence about the CI runner.
- `MASS_PARALLEL_DISPATCH.md:387-399` lists root, compiler, and torture checks
  but omits the `tools/chip-lint` commands; the same section says to verify the
  chip-lint entry point later (`:402-404`). CI is the current authority here and
  includes chip-lint.

## 4. Test discovery (observed)

No package declares `[[test]]`; Cargo auto-discovers integration files directly
under each `tests/` directory. `tools/torture/tests/common/mod.rs` is a shared
module included by `cli.rs` and `verify.rs`, not a test target.

| Area | Discovery mechanism | Files/targets | Tests listed/run |
|---|---|---|---|
| Root | `tests/paradigm.rs` + lib doctests | 1 integration file; lib; doc-tests | 21 listed and run |
| Compiler integration | every `compiler/tests/*.rs` | 41 files | 471 listed |
| Compiler library | `compiler/src` unit tests | lib target | 172 listed |
| Compiler `candidate` bin | bin unit tests | `src/bin/candidate.rs` + `src/bin/candidate/*.rs` | 9 listed |
| Compiler doctests | `compiler/src` doc comments | doc-test target | 2 listed |
| **Compiler total** | | 43 running targets + doc-tests | **654 listed** |
| Chip-lint | `tools/chip-lint/tests/lint.rs` | 1 integration file | 12 run |
| Torture | `tests/cli.rs`, `tests/verify.rs`, lib unit | 2 integration files + lib | 73 run |
| Probe | shell runner | `tools/torture/probe/tests/run-tests.sh` | 57 run |
| Orphan chip drafts | not compiled | 4 files | 39 **not** discovered (§5) |

Important distinctions:

- **654 is a discovery count, not a pass count.** The dispatch ledger reports
  the compiler suite as PASS, but this audit did not re-execute it.
- The 172 library tests include 47 tests from five draft modules that are
  compiled but not publicly exported. Those tests are discovered, but no
  integration test exercises those modules.
- Four additional files contain 39 inline tests that are **not compiled at
  all** because their parents do not declare them. Wiring them would raise the
  discovery count from 654 to 693, unless the files are deliberately parked.
- The root 21, chip-lint 12, torture 73, and probe 57 results were executed in
  this audit.

## 5. Unwired module and test paths (observed)

### 5.1 Files absent from their parent `mod.rs`

| File | Parent declaration | Inline tests | Evidence |
|---|---|---:|---|
| `compiler/src/chips/parse/pa_expr_ext.rs` | absent from `parse/mod.rs` | 11 | test module at `pa_expr_ext.rs:766` |
| `compiler/src/chips/parse/pa_typedef_query.rs` | absent from `parse/mod.rs` | 10 | test module at `pa_typedef_query.rs:351` |
| `compiler/src/chips/lex/lx_error.rs` | absent from `lex/mod.rs` | 10 | test module at `lx_error.rs:357` |
| `compiler/src/chips/lex/lx_extension.rs` | absent from `lex/mod.rs` | 8 | test module at `lx_extension.rs:303` |

`compiler/src/chips/parse/mod.rs:9-22` declares 14 modules; it does not declare
`pa_expr_ext` or `pa_typedef_query`. `compiler/src/chips/lex/mod.rs:6-13`
declares 8 modules; it does not declare `lx_error` or `lx_extension`. The four
files are therefore invisible to `cargo test`, `cargo clippy --all-targets`,
and `cargo fmt` target discovery, even though the chip-lint scan reads them.

### 5.2 Files compiled but not exported or integration-tested

The uncommitted `compiler/src/chips/parse/mod.rs` adds five modules:

- `pa_cast.rs` (15 inline tests)
- `pa_decl_spec.rs` (9)
- `pa_ext_dispatch.rs` (5)
- `pa_init.rs` (6)
- `pa_stmt.rs` (12)

They are declared at `parse/mod.rs:11-20` but are not re-exported by
`compiler/src/chips/parse/mod.rs:24-64` or by `compiler/src/chips/mod.rs:70-90`.
No file under `compiler/tests/` references their symbols. Their 47 inline tests
are discovered only as library unit tests; there is no integration, routing, or
manifest test for them. This audit did not execute those unit tests.

This is also a duplicate-producer problem, not just a warning problem. The
drafts claim overlapping candidate local IDs:

| Draft | Candidate kind | Local | Evidence |
|---|---|---|---:|
| `pa_expr_ext.rs` | PA15 initializer | 27 | `pa_expr_ext.rs:159`; comment at `:29-30` |
| `pa_init.rs` | PA15 initializer | 27 | `pa_init.rs:129`; comment at `:7` |
| `pa_typedef_query.rs` | PA04 typedef query | 27 | `pa_typedef_query.rs:136`; comment at `:105` |
| `pa_decl_spec.rs` | PA08 array suffix | 27 | `pa_decl_spec.rs:136` |
| `pa_cast.rs` | PA21 cast | 27 | `pa_cast.rs:140`; candidates at `:161-170` |
| `pa_stmt.rs` | PA27 statement | 27 | `pa_stmt.rs:138`; candidates at `:142-150` |

The files themselves warn that `PARSE` local 27 and `ChipId(58)` may be claimed
by parallel drafts (`pa_init.rs:7`, `pa_typedef_query.rs:105`). Adding all
`mod` lines or all registry rows without an integrator allocation would create
duplicate task kinds. `pa_init.rs` and `pa_expr_ext.rs` are also two
implementations of the same PA15 initializer responsibility.

## 6. Compiler warning gate (observed)

CI gates warnings through clippy:

```text
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings
```

(`.github/workflows/ci.yml:42-43`). The crate also has strong in-crate gates:
`#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, and
`#![deny(rustdoc::broken_intra_doc_links)]` (`compiler/src/lib.rs:16-18`).
A plain `cargo test` does not fail on rustc warnings; the dead-code gate is the
clippy step, which is why the ledger could record a passing test suite while
still listing 141 warnings.

On the current dirty tree that gate fails:

- Exit code 101.
- 142 lint diagnostics: 137 in the library target and 5 in the library test
  target.
- All diagnostics are confined to the five draft files listed in §5.2.
- Breakdown: 141 dead-code diagnostics (unused functions, constants, structs,
  methods, enums, fields, type aliases, associated functions) and one
  `clippy::vec-init-then-push` in `pa_cast.rs`. The target split is 137 in the
  library target and 5 in the library test target.
- The `--list` build separately emitted 143 rustc warnings, all from the same
  five files. The dispatch ledger's "141 warnings" is directionally consistent
  with this state, but the warnings come from modules that **are** declared by
  the modified `parse/mod.rs`, not from the four undeclared files.

The four undeclared files produce no warnings because they are never compiled.
So the warning gate detects the five drafts that were added to the build, and
is blind to the four drafts that were not.

HEAD-only clippy was not run. All failing files except `parse/mod.rs` are absent
from HEAD (`git ls-tree HEAD compiler/src/chips`), so the failure is attributable
to the dirty workspace with high confidence, but that is an inference, not an
executed HEAD result. The ledger records clippy as not run for the batch; this
audit ran the command only on the dirty tree, so the committed baseline remains
unverified for clippy.

## 7. Chip-lint coverage (observed)

The scan covers the real tree:

- CI runs `cargo run --locked --manifest-path tools/chip-lint/Cargo.toml -- compiler/src/chips`
  (`ci.yml:60-61`).
- `compiler/src/chips` exists and contains 72 Rust files across nine group
  directories (`compiler/src/chips/mod.rs:38-46`).
- The current scan passes, including the four undeclared drafts and the five
  declared drafts.
- `lint_tree` recursively collects `.rs` files, sorts them, and fails closed on
  a missing or empty directory (`tools/chip-lint/src/lib.rs:64-107`).

What the scan checks:

- Unit chip structs and `silicon_chip!` declarations (`lib.rs:120-141`,
  `:212-245`).
- `RestrictedChip` and inherent `XChip::compute` bodies (`lib.rs:155-178`,
  `:247-283`, `:425-462`).
- Legacy `LogicChip` rejection (`lib.rs:173-179`, `:464-470`).
- Imports, top-level macros, macros/unsafe/calls/denied paths inside `compute`
  (`lib.rs:180-205`, `:292-412`).

What it does not check:

- `Worker::handle` bodies (the adapter/projection side), so host I/O in a
  `handle` method would not be caught.
- Manifests, task-kind uniqueness, routing/registration, or store-owner
  allowlists. That is why the five unregistered drafts pass.
- Files outside `compiler/src/chips` such as `compiler/src/bin/candidate.rs`
  (host code, intentionally outside chip scope).

Test coverage for the lint tool itself is narrow: `tools/chip-lint/tests/lint.rs:1`
imports only `lint_source`. `lint_tree`, `collect_rust_files`, the missing-tree
error, the empty-tree error, and recursive ordering have no unit tests. CI's
single successful scan is the only execution of `lint_tree`.

## 8. Stale documentation claims (observed)

Line numbers are for the current working tree. "Source fact" is the observed
current state.

| Claim / location | Current source or CI fact | Severity |
|---|---|---|
| `README.md:142` says the foundation is `frozen /16` | `compiler/contracts/CONTRACT_VERSION:3` is `t01-c01-c06/39`; `README.md:338` also says `/39` | High: internal contradiction |
| `README.md:124` lists `/7`, `/8`, `/9` without marking them historical | Current identity is `/39` (`CONTRACT_VERSION:3-4`) | Medium |
| `README.md:339` links `compiler/src/chips/fold.rs` | That path does not exist; `FoldChip` is at `compiler/src/chips/constant_layout_init/fold.rs` | High: broken source link |
| `README.md:340` says only the initial fold subset exists | 72 Rust files under `compiler/src/chips`, with PP/LX/PA/TY/SE/IR/VF groups (`chips/mod.rs:38-46`) and tests `c10`–`c39` | High: understates source |
| `README.md:508` says `tests/` is `c01..c08 + freeze` | 41 files, including `c09`–`c39`, `freeze.rs`, `h04_candidate.rs` | High |
| `README.md:520` describes CI as root/chip-lint/compiler | CI also runs `tools/torture` fmt/clippy/test (`ci.yml:33-34,45-46,57-58`) | Medium |
| `README.md:637-665` says `tools/torture` is not wired into CI and lists only three packages | Torture is wired (`ci.yml:33-34,45-46,57-58`); the cited dated audit is itself stale | High: false current claim |
| `compiler/README.md:7` says the package "contains no C language chips" | `compiler/src/chips/` exists with 72 Rust files | High |
| `compiler/README.md:123` says `tests/` is `c01..c07 + freeze` | 41 test files | Medium |
| `compiler/README.md:212-219` says T02 control chips are not implemented, the routing shell has no worker handlers, and no host exists | `WorkerRegistry`/`drive_task`/`handler_for` exist (`chips/mod.rs:137-419`); `compiler/src/bin/candidate.rs` and its helper modules exist | High |
| `docs/guide/PROJECT_STATUS.md:12` says frozen `/7` and 158 integration tests | `/39`; 654 tests listed (471 integration + 172 lib + 9 bin + 2 doc) | High |
| `docs/guide/PROJECT_STATUS.md:13` says no chip source exists | 72 chip source files exist (partial capability; no complete compiler) | High |
| `docs/guide/PROJECT_STATUS.md:17` says chip-lint has 8 tests | 12 tests observed | Medium |
| `docs/guide/PROJECT_STATUS.md:25-31` gives `/7` and hash `a56de…` | `/39`, hash `07f4…` (`CONTRACT_VERSION:3-4`) | High |
| `docs/guide/PROJECT_STATUS.md:62-70` test inventory stops at `freeze.rs`; `c08` is listed as 11 | `c08_gate1.rs` has 21 tests; files `c09`–`c39` and `h04_candidate.rs` are missing | High |
| `docs/guide/PROJECT_STATUS.md:77-79` says the allowlist is an empty skeleton, stage assignment accepts everything, and `AppendRecords` is rejected | Allowlist rows exist (`manifest.rs:771-786`); stage assignment is enforced (`manifest.rs:1437-1446`, `:1516-1536`); `AppendRecords` is handled and exercised (`commit.rs:981`, `c08_gate1.rs:390,575`) | High |
| `docs/guide/PROJECT_STATUS.md:81` says only live IDs are snapshotted | `snapshot.rs:9-16` and `:680-760` encode bodies for typed stores; some stores remain reserved | Medium |
| `docs/guide/PROJECT_OVERVIEW.md:47` says frozen `/6` | `/39` | High |
| `docs/guide/PROJECT_OVERVIEW.md:48` says the C pipeline is "Planned only … no code" | Partial PP/LX/PA/TY/SE/IR/VF code exists; no complete pipeline | High |
| `docs/guide/ARCHITECTURE.md:88` says frozen `/6` | `/39` | High |
| `docs/guide/GLOSSARY.md:31` says `/6` | `/39` | High |
| `docs/guide/GLOSSARY.md:42` says the 331 language chips do not exist yet | Many chip files exist; they are partial slices, not all 331 completed chips | Medium |
| `docs/guide/ROADMAP.md:27` says the foundation is `/6` | `/39` | High |
| `docs/guide/ROADMAP.md:31` says only the four foundation kinds exist | `TaskKindRegistry` exposes many `*_slice` registries (`task.rs:660-1157`); `STAGE_ASSIGNMENT` has non-foundation kinds (`manifest.rs:1444+`) | High |
| `docs/tasks/README.md:3` says "the compiler is not yet implemented" | Defensible only as "no complete compiler"; the same page later says group chip directories and host code do not exist, which is false | Medium |
| `docs/tasks/README.md:64-66` says H04 `-E`/`-D`/`-U` are deferred | `H04_CANDIDATE_DRIVER.md:5-8,28-32` says Part A plus `-E`/`-D`/`-U` are present; only `-I` and multi-source are deferred | High: direct contradiction |
| `docs/tasks/README.md:131,133-139` says group chip directories and host code "do not exist yet" | `compiler/src/chips/<group>/` exists; `compiler/src/bin/candidate.rs` and helpers exist | High |
| `docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md:131` says the torture package is not wired into CI | `ci.yml:33-34,45-46,57-58` wires it | High |
| `docs/reviews/2026-10-05_CI_GATING_AUDIT.md:63-72` says torture and the chip-lint scan are not gated | Both are gated now (`ci.yml:45-46,57-58,60-61`) | Medium: dated record, but README cites it as current |
| `docs/reviews/2026-10-05_CI_GATING_AUDIT.md:99-110` records old test counts (root 15+6, chip-lint 8, compiler 88) | Current observed: root 21, chip-lint 12, compiler 654 listed | Low: explicitly dated evidence |

The README already carries a caveat that linked documents may describe older
versions (`README.md:408-413`). That caveat does not cover the current-status
table's own `/16`/`/39` contradiction or the testing section's false CI claim.

## 9. Missing coverage and risks

Observed gaps:

1. **Orphan source files are invisible to the build.** Four chip files and their
   39 tests are not compiled. There is no check that every `.rs` under
   `compiler/src/chips` is declared by a parent `mod.rs`.
2. **Compiled drafts are invisible to integration.** Five parse drafts are
   declared by the dirty `parse/mod.rs` but have no public export, manifest,
   routing, or integration test. They also claim colliding candidate locals.
3. **`lint_tree` is untested.** The tool's tree walk and fail-closed errors are
   exercised only by one CI success case.
4. **No docs/status check exists.** Stale versions, counts, and CI claims can
   land without a failing check. This is why the README and guide drifted.
5. **The compiler clippy gate is red in the current workspace.** The gate itself
   is correct; the draft disposition is missing.
6. **The target probe remains manual and unrun.** This is intentional and is not
   a pre-Stage-B test gap, but no ABI/execution claim may be made.
7. **Torture has no corpus and no runner.** Its crate is green and CI-gated, but
   H00–H10 are still incomplete; no pass rate exists.

Non-gaps, for clarity:

- Cargo auto-discovers all 41 compiler integration files; no test registration
  is missing for files that are actually present.
- `tools/torture` is now covered by fmt, clippy, and tests in CI.
- The chip-lint scan points at the real tree and fails closed on an empty tree.
- The probe workflow's static/self-test job is path-filtered by design; the real
  target probe is manual by design.
- The root package has no workspace, so `--workspace` commands would not reach
  nested packages. CI's per-manifest commands are the correct mechanism.

## 10. Recommended work orders (recommendation)

These are the smallest work orders that unblock a trustworthy Stage B baseline.
Ownership follows the integrator/shared-file rules in
`docs/tasks/PARALLEL_EXECUTION.md`; a worker must not edit another owner's file.

### WO-A13-01 — Make the compiler clippy gate green without weakening it

- **Priority:** P0, before Stage B integration.
- **Owner:** Integrator for `compiler/src/chips/parse/mod.rs`; the five draft
  authors for their own files.
- **Paths:** `compiler/src/chips/parse/mod.rs`, `pa_cast.rs`,
  `pa_decl_spec.rs`, `pa_ext_dispatch.rs`, `pa_init.rs`, `pa_stmt.rs`.
- **Action:** Decide the disposition of the five drafts. Either (a) export and
  register the canonical ones, add their manifests/routing/integration tests,
  and fix the dead-code warnings, or (b) remove their `mod` lines until their
  freeze is approved and keep them as explicit parked drafts. Do not add a
  blanket `#[allow(dead_code)]`; that would hide unregistered producers.
- **Acceptance:**
  - `cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings` exits 0.
  - `cargo test --locked --manifest-path compiler/Cargo.toml` passes.
  - No draft module remains compiled-but-unexported without an explicit status.
- **Note:** The failure is entirely in these five files; the rest of the
  compiler package compiles.

### WO-A13-02 — Resolve the four orphan drafts and their ID collisions

- **Priority:** P0/P1, before the affected PA/LX chips enter Stage B.
- **Owner:** Integrator for `parse/mod.rs`, `lex/mod.rs`, `chips/mod.rs`,
  `task.rs`, and `manifest.rs`; draft authors for their files.
- **Paths:** `compiler/src/chips/parse/pa_expr_ext.rs`,
  `pa_typedef_query.rs`, `compiler/src/chips/lex/lx_error.rs`,
  `lx_extension.rs`, plus the two group `mod.rs` files,
  `compiler/src/chips/mod.rs`, `compiler/src/task.rs`,
  `compiler/src/manifest.rs`.
- **Action:** Allocate distinct `TaskKind` locals and `ChipId`s. Choose one
  canonical PA15 initializer implementation between `pa_init.rs` and
  `pa_expr_ext.rs`; reconcile `pa_typedef_query.rs`, `pa_decl_spec.rs`,
  `pa_cast.rs`, and `pa_stmt.rs` against their colliding local-27 claims. Then
  either wire each module with a manifest, route, allowlist row, and integration
  test, or move it to an explicitly non-built draft location.
- **Acceptance:**
  - All chip `.rs` files are reachable from a `mod.rs` or listed in a small
    explicit draft allowlist.
  - `cargo test --locked --manifest-path compiler/Cargo.toml -- --list` includes
    the intended additional tests (654 → 693 if all 39 are wired).
  - `cargo test --locked --manifest-path compiler/Cargo.toml` passes.
  - No two candidate kinds claim the same local; no duplicate PA15 producer is
    registered.
- **Dependency:** Integrator freeze for the candidate kind/ChipId allocation.
  Do not let each draft invent its own row.

### WO-A13-03 — Add an orphan-source/reachability guard

- **Priority:** P1, can be done in parallel with WO-A13-01/02.
- **Owner:** Integrator/test owner.
- **Paths:** a new test such as `compiler/tests/c40_wiring.rs`; no Cargo change
  is required because Cargo auto-discovers top-level test files.
- **Action:** Using `env!("CARGO_MANIFEST_DIR")`, walk
  `compiler/src/chips/**/*.rs` and assert that every file other than `mod.rs` is
  named by a `mod` declaration in its parent `mod.rs`, or appears in a short,
  reviewed `DRAFT_FILES` list. Report the file path on failure. Do not use a
  dependency for this; `std::fs` is sufficient in a test.
- **Acceptance:**
  - The test fails on the current four orphans and passes after WO-A13-02 or
    after the drafts are explicitly parked.
  - `cargo test --locked --manifest-path compiler/Cargo.toml --test c40_wiring`
    passes.
- **Benefit:** Prevents a recurrence of the exact failure mode found here: a
  file that exists, has tests, and is never built.

### WO-A13-04 — Cover `lint_tree` and the real-tree scan behavior

- **Priority:** P1, independent of the compiler work.
- **Owner:** `tools/chip-lint` owner.
- **Paths:** `tools/chip-lint/tests/lint.rs`, plus committed fixtures under
  `tools/chip-lint/tests/fixtures/`.
- **Action:** Add tests for `lint_tree` on a nested fixture tree, a missing
  directory (error), an empty directory (error), and a tree containing a syntax
  error (fail-closed diagnostic). Assert deterministic sorted order. Prefer
  committed fixtures over a new temporary-directory dependency.
- **Acceptance:**
  - `cargo test --locked --manifest-path tools/chip-lint/Cargo.toml` passes.
  - `cargo clippy --locked --manifest-path tools/chip-lint/Cargo.toml --all-targets -- -D warnings` passes.
  - CI's `compiler/src/chips` scan remains unchanged and passing.

### WO-A13-05 — Correct stale status and CI documentation

- **Priority:** P1, can run in parallel; docs-only.
- **Owner:** Documentation/integrator writer; do not edit dated audit history
  silently.
- **Paths:** `README.md`, `compiler/README.md`,
  `docs/guide/PROJECT_STATUS.md`, `docs/guide/PROJECT_OVERVIEW.md`,
  `docs/guide/ARCHITECTURE.md`, `docs/guide/GLOSSARY.md`,
  `docs/guide/ROADMAP.md`, `docs/tasks/README.md`,
  `docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md`,
  `docs/reviews/2026-10-05_CI_GATING_AUDIT.md`.
- **Action:** Update current-status text to `/39` and the observed counts;
  describe the chip tree and candidate host code as present but partial; state
  that torture is CI-gated and the chip-lint scan is CI-gated; fix the
  `FoldChip` path; correct the H04 `-E`/`-D`/`-U` contradiction; remove or
  qualify "no chip source", "only the four foundation kinds", and "allowlist is
  empty" claims. Add a "superseded by current CI" banner to the dated
  `2026-10-05_CI_GATING_AUDIT.md` rather than rewriting its historical results.
- **Acceptance:**
  - No current-status page says `/6`, `/7`, or `/16` where `/39` is meant.
  - No page claims torture is not in CI.
  - Local links used in the changed sections resolve.
  - Claims are labeled observed/planned and no unrun test, probe, or pass rate
    is claimed.
- **Commands:** `git diff --check`; manual link/`rg` checks. No project docs
  validator exists today.

### WO-A13-06 (optional) — Add a docs consistency check to CI

- **Priority:** P2; only after WO-A13-05.
- **Paths:** `.github/workflows/ci.yml` and a small script under `tools/` or
  `docs/`.
- **Action:** Check that the `CONTRACT_VERSION` string is present in the status
  pages and that changed Markdown local links resolve. This is a drift guard,
  not a correctness proof.
- **Acceptance:** The check runs on push/PR; a deliberate `/16` or broken local
  link fails it.
- **Caution:** Do not make this a substitute for the content correction; do not
  claim it validates documentation truth.

## 11. Unrun checks

- Full compiler test execution (`cargo test --manifest-path compiler/Cargo.toml`):
  not run. Only `--list` was run (exit 0, 654 tests listed). The dispatch ledger
  reports PASS, but this audit does not re-verify it.
- Compiler library/integration test execution, including the 47 draft-module
  unit tests: not run.
- Clippy for root, `tools/chip-lint`, and `tools/torture`: configured in CI, not
  run here. Only the compiler clippy gate was run (and failed on the dirty
  tree).
- HEAD-only compiler clippy: not run; the dirty tree has five added modules.
- GitHub Actions execution: not run; all results are local macOS results.
- Target probe (`tools/torture/probe/run-probe.sh`): not run; manual and
  environment-gated. No ABI or execution claim is made.
- GCC torture corpus/runner: not fetched and not run. No pass rate is claimed.
- Docs link checker: none exists in the repository; only manual inspection was
  performed.
- M1 end-to-end execution: not run; outside this audit.

## 12. Open questions

1. Are the four undeclared files (`pa_expr_ext`, `pa_typedef_query`,
   `lx_error`, `lx_extension`) intended for the next freeze, or are they
   superseded drafts? Their candidate locals collide, so they cannot all be
   wired as written.
2. Is `pa_init.rs` or `pa_expr_ext.rs` the canonical PA15 initializer producer?
   Both exist and both claim `PARSE` local 27.
3. Should the five compiled drafts in `parse/mod.rs` be exported/registered in
   Stage B, or removed from the default build until their freeze? The answer
   determines whether WO-A13-01 fixes warnings by wiring or by parking.
4. Should `tools/chip-lint` gain a `handle`-body check and a registration/ID
   uniqueness check, or should those remain separate manifest/contract tests?
   The current tool is a purity aid, not a wiring validator.
5. Should the dated `2026-10-05_CI_GATING_AUDIT.md` be updated in place or
   marked superseded? Keeping it as a dated record is safer for provenance.
6. Is the root `Cargo.lock` intentionally untracked and therefore not
   `--locked` in CI? The current `.gitignore` excludes `/Cargo.lock`; this is a
   reproducibility choice, not a test failure, but it should be deliberate.

## 13. Evidence index

Primary source and configuration evidence:

- Contract identity: `compiler/contracts/CONTRACT_VERSION:3-4`
- Chip group modules: `compiler/src/chips/mod.rs:38-46`
- Parse module declarations: `compiler/src/chips/parse/mod.rs:9-22`
- Lex module declarations: `compiler/src/chips/lex/mod.rs:6-13`
- Chip-lint tree walk: `tools/chip-lint/src/lib.rs:64-107`
- Chip-lint body checks: `tools/chip-lint/src/lib.rs:109-210`, `:247-283`,
  `:292-412`, `:425-462`
- Chip-lint tests: `tools/chip-lint/tests/lint.rs:1`
- CI package steps: `.github/workflows/ci.yml:24-61`
- Probe workflow: `.github/workflows/t00-target-probes.yml:46-145`
- Compiler library gates: `compiler/src/lib.rs:16-18`
- Allowlist and stage assignment: `compiler/src/manifest.rs:771-786`,
  `:1437-1446`, `:1516-1536`
- Task-kind slices: `compiler/src/task.rs:660-1157`
- Snapshot bodies: `compiler/src/snapshot.rs:9-16`, `:680-760`
- Candidate host binary: `compiler/src/bin/candidate.rs`,
  `compiler/src/bin/candidate/emit_helper.rs`,
  `compiler/src/bin/candidate/macro_flags.rs`

Commands run in this audit (all on macOS, current tree):

```text
cargo fmt --all -- --check
cargo fmt --manifest-path compiler/Cargo.toml -- --check
cargo fmt --manifest-path tools/chip-lint/Cargo.toml -- --check
cargo fmt --manifest-path tools/torture/Cargo.toml -- --check
cargo test --locked
cargo test --locked --manifest-path tools/chip-lint/Cargo.toml
cargo test --locked --manifest-path tools/torture/Cargo.toml
bash tools/torture/probe/tests/run-tests.sh
cargo test --locked --manifest-path compiler/Cargo.toml -- --list
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/chip-lint/Cargo.toml -- compiler/src/chips
git status --porcelain=v1
git rev-parse --abbrev-ref HEAD
git rev-parse --short HEAD
git ls-tree -r --name-only HEAD compiler/src/chips
```

No command in this list modified a tracked file. The only file created by this
audit is this report.
