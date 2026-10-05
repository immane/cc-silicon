# CI Gating Audit — 2026-10-05

## Status and scope

- Status: **read-only audit record**. This document is not an ADR, contract
  approval, `/6` freeze, sign-off, or implementation authorization. It changes
  no workflow, no accepted decision, and closes no item.
- Scope: the two GitHub Actions workflows
  [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) and
  [`.github/workflows/t00-target-probes.yml`](../../.github/workflows/t00-target-probes.yml),
  and every CI-coverage claim in `compiler/README.md`,
  `docs/tasks/M1_TARGET_ACCEPTANCE.md`, `docs/tasks/T13_VERIFICATION_CHIPS.md`,
  `docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md`,
  `docs/tasks/T00_GCC_TORTURE_GATE.md`, and `tools/torture/probe/README.md`.
- Baseline: the working tree at audit time, including uncommitted and untracked
  documents. Line numbers correspond to that version and may drift.
- This audit **edited no reviewed file**; the only file added is this record.

## Method and evidence

- Both workflow files were read in full; triggers, jobs, steps, and exact
  commands were enumerated (sections 1–2).
- CI-coverage claims were located by repository-wide search for `CI`,
  `ci.yml`, `workflow`, and `chip-lint`, then checked command-by-command
  against the workflows (section 4).
- The gated commands were run locally on the macOS development host (not the
  CI runner) to confirm they exist and pass on the current tree (section 5).
  The non-gated `tools/torture` crate was also run, to show it is green but
  **not enforced by CI**.
- `git diff --check` and `git diff --cached --check` were run on the working
  tree.

## 1. Gated by `.github/workflows/ci.yml` (every push and pull request)

Single job `rust` (`ubuntu-latest`, stable Rust + clippy + rustfmt); no path
filter, so it runs on all pushes/PRs. All nine steps:

| # | Step | Command | Package covered |
|---|---|---|---|
| 1 | Check framework formatting | `cargo fmt --all -- --check` | root only (nested packages are not workspace members) |
| 2 | Check chip-lint formatting | `cargo fmt --manifest-path tools/chip-lint/Cargo.toml -- --check` | `tools/chip-lint` |
| 3 | Check compiler formatting | `cargo fmt --manifest-path compiler/Cargo.toml -- --check` | `compiler` |
| 4 | Lint framework | `cargo clippy --all-targets -- -D warnings` | root (no `--locked`; root `/Cargo.lock` is gitignored) |
| 5 | Lint chip-lint tool | `cargo clippy --locked --manifest-path tools/chip-lint/Cargo.toml --all-targets -- -D warnings` | `tools/chip-lint` (committed lock) |
| 6 | Lint compiler | `cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings` | `compiler` (committed lock) |
| 7 | Test framework | `cargo test` | root (unit + integration + doctests) |
| 8 | Test chip-lint tool | `cargo test --locked --manifest-path tools/chip-lint/Cargo.toml` | `tools/chip-lint` |
| 9 | Test compiler | `cargo test --locked --manifest-path compiler/Cargo.toml` | `compiler` (incl. compile-fail doctests) |

## 2. Gated by `.github/workflows/t00-target-probes.yml`

- Trigger: `workflow_dispatch` (manual) plus `push`/`pull_request` **filtered
  to** `tools/torture/probe/**` and the workflow file itself.
- `static` job (`ubuntu-latest`): runs
  `bash tools/torture/probe/tests/run-tests.sh` — normalizer and fail-closed
  guard self-tests, no compiler. This is the only probe job that runs
  automatically, and only for the path-filtered changes above.
- `target-probe` job (`ubuntu-24.04-arm`): **manual only**
  (`if: github.event_name == 'workflow_dispatch'`), fail-closed on empty
  declared inputs, runs the real reference-GCC probe, uploads the report.
  This is **not a merge gate**.

## 3. Not gated

- **`tools/torture` (H00 verifier crate).** No workflow runs its
  `fmt`, `clippy`, or `test`. `ci.yml` has no step with
  `--manifest-path tools/torture/Cargo.toml`, and the probe workflow's path
  filter excludes `tools/torture/src/**` and its tests.
- **`tools/chip-lint` scan.** CI builds, lints, and tests the linter tool, but
  never runs `cargo run --manifest-path tools/chip-lint/Cargo.toml -- <dir>`
  over any source tree. There is no chip tree in this repository to scan; the
  framework `src/` and `compiler/` are not scanned.
- **H09 gate.** No full/smoke/nightly torture regression or release gate
  workflow exists. The torture runner (H02–H10) is planned only; H02–H03 are
  required for any torture run and H05–H10 for the final gate.
- **Root formatting does not reach nested packages.** `cargo fmt --all` covers
  only the root package; `chip-lint` and `compiler` have explicit steps, and
  `tools/torture` has none.
- **Probe self-tests are path-filtered.** A change outside
  `tools/torture/probe/**`/the workflow (e.g. `tools/torture/src/**`) does not
  trigger them. The `static` job also does not compile the probe C sources
  (`src/abi-args.c`); it only exercises the normalizer and guards.

## 4. CI-coverage claims vs. reality

| Claim | Location (drifting) | Verdict |
|---|---|---|
| "root CI adds explicit compiler steps so the contract app is built and tested in the same root CI job"; "Root CI covers the compiler today" | `compiler/README.md` §Integration layout | **Accurate** for `fmt`/`clippy`/`test` via `--manifest-path`; the nested package is not covered by `cargo fmt --all`/`cargo test` without those steps. |
| "Compiler package is covered by root CI via explicit manifest path" | `docs/tasks/M1_TARGET_ACCEPTANCE.md` §1.4 | **Accurate** (steps 3/6/9). |
| "Its formatting, Clippy, and tests run in `.github/workflows/ci.yml`; no application chip tree exists in this framework-only repository to scan yet." | `docs/tasks/T13_VERIFICATION_CHIPS.md` | **Accurate**; explicitly does not claim the scan runs in CI. |
| "`static` runs on probe/workflow `push` and `pull_request` … `target-probe` is manual only" | `tools/torture/probe/README.md` §CI | **Accurate** (path-filtered static job; manual pinned job). |
| "**Not wired into CI.** … The package is standalone like `tools/chip-lint`." | `docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md` | **Accurate**: the bullet states the gap. "Standalone" describes the package/lockfile layout; unlike `tools/chip-lint`, `tools/torture` has no CI steps — the wording is tight enough but should not be read as CI parity. |
| H09 "CI full/smoke/nightly regression and release gate"; H02–H10 planned only | `docs/tasks/T00_GCC_TORTURE_GATE.md` §4/§4.1 | **Accurate**: H09 is a planned acceptance row, and §4.1 states H02–H03/H05–H10 are planned only, with no torture run or pass rate claimed. |

No document claims that the torture crate, the chip-lint scan, or the H09
gate is currently enforced. **No doc correction was required and none was
applied.**

## 5. Verification runs (2026-10-05, macOS development host)

- Gated `ci.yml` commands: `cargo fmt` checks for root/chip-lint/compiler —
  clean; `cargo clippy` for root/chip-lint/compiler with `-D warnings` —
  clean; `cargo test` root 15 + 6 doctests, chip-lint 8, compiler 88
  (c01 6, c02 7, c03 22, c04 11, c05 14, c06 10, c07 11, freeze 5,
  doctests 2) — all pass.
- Gated probe self-tests: `bash tools/torture/probe/tests/run-tests.sh` —
  57 passed, 0 failed.
- Not-gated coverage check: `cargo test --locked --manifest-path
  tools/torture/Cargo.toml` — 51 passed (green, but not CI-enforced);
  `cargo metadata --locked` succeeds for both committed nested lockfiles.
- `git diff --check` — exit 0; `git diff --cached --check` — exit 0.

Limitations: these runs establish that the gated commands pass on this
development tree; they do not execute the Linux/AArch64 probe, the torture
suite, or any candidate compiler, and they are not evidence about the CI
runner environment.
