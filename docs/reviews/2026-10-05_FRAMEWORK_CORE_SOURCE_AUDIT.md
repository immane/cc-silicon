# Framework Core Source Audit — 2026-10-05

## Status and scope

- Status: review record only. This is not an ADR, contract approval, interface freeze, or implementation authorization.
- No source, test, example, or contract file was modified. This document is the only artifact.
- Findings below are observations about what the framework enforces versus what its documentation claims. They are not confirmations of runtime defects in the default path.
- Date: 2026-10-05.
- Baseline: working tree at audit time (includes existing uncommitted and untracked documentation). Line numbers refer to that revision and may drift.
- Scope: `src/*.rs` of the root `cc-silicon` crate (`lib.rs`, `bus.rs`, `chip.rs`, `motherboard.rs`, `backend.rs`, `clock.rs`, `sim.rs`, `prelude.rs`). Tests, example, README, and architecture documents were consulted as evidence. `compiler/` was consulted only where it exercises framework contracts.
- Method: direct source reading; two compile probes in a scratch crate outside the repository for `silicon_chip!` macro behavior; command verification below.

## Verification evidence

Commands run from the repository root:

| Command | Result |
|---|---|
| `git diff --check` | exit 0, no output (tracked changes only; untracked files are not covered by this command) |
| `cargo test --workspace` | pass: 0 unit + 15 integration (`tests/paradigm.rs`) + 6 doctests |
| `cargo test --manifest-path compiler/Cargo.toml` (supplementary) | pass: 0+6+7+22+11+14+10+11+5 integration tests, 2 doctests |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |

Coverage caveat: the root `Cargo.toml` has no `[workspace]` table, so `cargo test --workspace` resolves to the root `cc-silicon` package only and does not build `compiler/`. The compiler suite was run separately and also passes.

Scratch probes (outside the repository, no repo files touched) are described in FCA-08 and FCA-09.

## Findings

### FCA-01 — `#![forbid(unsafe_code)]` — PASS

- Location: `src/lib.rs:18`; `compiler/src/lib.rs:16`.
- Evidence: no `unsafe` token exists anywhere under `src/`, `tests/`, or `examples/` (grep). `forbid` is crate-wide and cannot be downgraded by inner `allow` attributes.
- Boundary: `forbid` covers this crate only. A downstream application can still write `unsafe` inside its own chips, adapters, or backends. The chip linter (`tools/chip-lint/src/lib.rs:366-370`) flags unsafe blocks in chip sources, but it is heuristic.
- Verdict: the framework keeps the required guarantee; downstream purity remains the application's responsibility.

### FCA-02 — reset → propagate → latch order — PASS

- Location: `src/motherboard.rs:113-123`.
- Evidence: `clock_tick` performs `bus.reset_wires()` → `backend.execute_layers(...)` → `bus.latch(pins)` → `bus.advance_tick()`, matching `docs/architecture/SILICON_PARADIGM_SPEC.md:94-107`.
- Supporting tests: `wires_reset_every_tick`, `layers_and_chips_propagate_within_a_tick`, `latching_enables_edge_detection`, `reset_is_asynchronous_and_wins_the_tick`, `lamport_clock_advances`, `prop_wires_never_leak_across_idle_ticks` (`tests/paradigm.rs`).
- Notes: `Bus::reset_wires` defaults to whole-bundle `Default` assignment (`src/bus.rs:45-47`); `latch` and `advance_tick` default to no-ops (`src/bus.rs:54,64-67`), consistent with `tick_count` defaulting to `0` (`src/bus.rs:60-62`). Overriding hooks are an application contract, not enforced (see FCA-03).

### FCA-03 — Tick-lifecycle authority is a convention, not an enforced boundary

- Severity: medium.
- Location: `src/bus.rs:45-67`; `src/chip.rs:36`; `src/motherboard.rs:23`; claims at `README.md:142-143` and `docs/architecture/SILICON_PARADIGM_SPEC.md:84`.
- Problem: documentation says the motherboard "is the only entity allowed to invoke a chip or to reset/latch the bus". The API does not enforce that:
  - `Bus::reset_wires`, `Bus::latch`, and `Bus::advance_tick` are public trait methods on the public `Bus` trait.
  - Every `LogicChip::tick` receives `&mut B` (`src/chip.rs:36`), so any chip can call `bus.reset_wires()` or `bus.latch(pins)` mid-propagation.
  - `Motherboard::layers` is a `pub` field (`src/motherboard.rs:23`), so any caller can invoke `layers[l][i].tick(pins, bus)` directly, skipping the reset/backend/latch sequence, or mutate the topology at any time between ticks.
- Cross-reference: the first downstream application already uses a second driver. `compiler/src/routing.rs:383-411` implements its own `clock_tick` that calls `bus.reset_wires()`, `bus.latch(pins)`, and `bus.advance_tick()` directly. Phase order is preserved there, but the "only entity" wording is contradicted in practice.
- Assessment: this is consistent with the framework's stated conditional-property model (`README.md:326-337`) and with keeping the hooks public for application-owned schedulers. The defect is the absolute wording, not the API.
- Recommendation (doc-only): describe the motherboard as the only framework entity that performs these operations, and state explicitly that application code with `&mut B` can drive the lifecycle and must preserve the phase order; or tighten the API (seal the hooks behind a crate-internal trait, make `layers` `pub(crate)` with a read-only accessor). Adding a negative fixture that demonstrates a direct `layers[l][i].tick(...)` call bypasses phases would make the boundary testable.

### FCA-04 — Default backend is exactly-once; overrides are unverified

- Severity: low (default path passes; override trust is documented).
- Location: `src/backend.rs:23-43`, `src/backend.rs:49-53`; `docs/architecture/SFL_CONTRACT.md:170-187` and §6.
- Evidence: the provided `Backend::execute_layers` runs every chip exactly once, layers in order, chips in insertion order. `CpuBackend` uses that default. `Motherboard::clock_tick` calls `execute_layers` exactly once per tick (`src/motherboard.rs:118`).
- Gaps:
  - `Backend` is an open trait; an override can skip, duplicate, or reorder chips with no compile-time or runtime check. The requirement is documented (`src/backend.rs:28-30`, `SFL_CONTRACT.md:172-174`) but not enforced.
  - `tests/paradigm.rs:230-274` exercises a custom backend, but it is a faithful delegating override; no test covers a divergent backend.
  - `SFL_CONTRACT.md:191-204` requires every backend to publish a capability matrix; the Rust trait exposes only `name()` (`src/backend.rs:24-25`). No capability or cross-backend equivalence machinery exists in the framework.
- Recommendation (doc-only): keep the trust-boundary wording, and if stronger evidence is wanted later, add a replay-diff test helper comparing a candidate backend against `CpuBackend` on a fixed pin trace. Treat the capability-matrix requirement as an application/backend contract item, not a framework guarantee.

### FCA-05 — `RestrictedChip` has no bus access — PASS

- Location: `src/chip.rs:39-63`, `src/chip.rs:84-109`, `src/chip.rs:195-213`.
- Evidence: `RestrictedChip::compute` receives only `&Self::Input` and returns `Self::Output` (`src/chip.rs:62`). `ProjectedChip::tick` runs adapter `read` → chip `compute` → adapter `commit`, and the chip never sees the bus (`src/chip.rs:104-108`). The `silicon_chip!` macro exposes only the input argument.
- Compile-fail doctests cover direct bus access (`src/chip.rs:163-175`), input mutation (`src/chip.rs:177-193`), and stateful manual implementations when installed or explicitly asserted (`src/chip.rs:149-161`). All four compile-fail doctests pass under `cargo test`.
- `Input`/`Output` are bounded `'static` in the `LogicChip` impl (`src/chip.rs:100-101`), so a projection cannot smuggle a borrowed bus reference.
- Verdict: the no-bus property holds on the supported install path.

### FCA-06 — Adapter trust boundary is correctly documented, but not checkable

- Severity: informational (accepted boundary).
- Location: `src/chip.rs:65-75`; `src/motherboard.rs:74-79`; `README.md:129-131`.
- Evidence: `ChipAdapter::read` receives `&B` and `commit` receives `&mut B`; the framework cannot inspect which fields they touch. The doc comments state this and require small, tested adapters. There is no manifest type; the read/write sets exist only as prose.
- Test coverage: `projected_chip_adapter_commits_only_its_declared_output` (`tests/paradigm.rs:290-310`) verifies one adapter's declared write set and that wire reset stays motherboard-owned; it does not generalize to other adapters.
- Verdict: the boundary is honestly documented. The only mitigation is review plus per-adapter tests; no framework change is implied by this audit.

### FCA-07 — Statelessness (ZST) is enforced only on the `RestrictedChip` path

- Severity: medium (doc-versus-enforcement).
- Location: `src/chip.rs:11-17`, `src/chip.rs:29-37`; `src/motherboard.rs:67-72`; claims at `README.md:120` and `README.md:303-304`; `docs/architecture/SILICON_PARADIGM_SPEC.md:75`.
- Problem: the paradigm documentation says chips are zero-field unit structs and that "zero-field unit structs cannot hide state". The primary `LogicChip` trait has no size assertion, and `Motherboard::install` accepts any `C: LogicChip<B> + 'static` (`src/motherboard.rs:67-72`). A field-bearing or interior-mutable chip — for example `struct C(Rc<Cell<u32>>)` — installs and runs without any framework rejection. The compile-time `ASSERT_STATELESS` check exists only for `RestrictedChip` (`src/chip.rs:53,87,207`).
- Assessment: `README.md:194` calls `LogicChip` the legacy compatibility API and the linter flags it (`tools/chip-lint/src/lib.rs:168`), and `README.md:326-337` correctly disclaims unconditional purity. But `README.md:120` and `README.md:303-304` state the structural guarantee more broadly than the API enforces.
- Recommendation (doc-only): scope the "cannot hide state" claim to `RestrictedChip` / `silicon_chip!`, and state that `LogicChip` statelessness is a reviewed convention. If stronger enforcement is wanted later, that is a separate interface decision.

### FCA-08 — `silicon_chip!` does not tie `$name` to `$name2`; the macro-level ZST assertion can check a different type

- Severity: medium (defense-in-depth gap; the supported install path still rejects it).
- Location: `src/chip.rs:195-213` — the assertion checks `$name` (`src/chip.rs:207`) while the impl is emitted for `$name2` (`src/chip.rs:208`), and the macro never requires `$name == $name2`.
- Evidence (scratch crate outside the repository, `cc-silicon` as a path dependency):

  ```rust
  struct StatefulChip(u32); // pre-existing non-ZST type
  silicon_chip! {
      struct OtherUnit;
      impl RestrictedChip for StatefulChip { /* ... */ }
  }
  ```

  This compiles; `size_of::<StatefulChip>() == 4`; a direct `StatefulChip(7).compute(&Input)` call runs with no compile error. The macro's top-level `const _` asserted only `OtherUnit`.
- Mitigation present: `ProjectedChip::new` evaluates `C::ASSERT_STATELESS` (`src/chip.rs:87`), so `Motherboard::install_projected` rejects a non-ZST target at compile time. The gap is that direct `compute` calls bypass that check, and the macro's advertised declaration-time guarantee does not hold for mismatched names.
- Recommendation (doc-only): emit the `RestrictedChip` impl for `$name` and remove `$name2`, or add an explicit identity assertion; alternatively document that direct `compute` use bypasses `ASSERT_STATELESS`. `ASSERT_STATELESS` is a necessary but not sufficient statelessness condition regardless: a ZST can still read/write statics, perform I/O, or sample a clock, as `src/chip.rs:45-48` acknowledges.

### FCA-09 — Field-bearing compile-fail doctest fails at macro matching, not at the ZST assertion

- Severity: low (evidence quality, not a behavior hole).
- Location: `src/chip.rs:135-147`.
- Evidence: compiling the snippet verbatim produces `error: no rules expected {` pointing at the macro pattern `struct $name;` (`src/chip.rs:198`). The doctest also uses `_: &Input`, which cannot match `$arg:ident: &Self::Input` (`src/chip.rs:202`). The claimed rejection ("field-bearing chip declaration is rejected") is true — the macro pattern only accepts `struct $name;` — but the compile-fail passes for syntax/pattern reasons, not because the zero-size assertion fires.
- Assessment: the mechanism actually exercised is macro syntax, which is still a rejection; the doctest comment is misleading about why.
- Recommendation (doc-only): reword the comment to say the macro accepts only unit-struct declarations, or restructure the negative test so a non-ZST manual impl is rejected through `ASSERT_STATELESS` (the pattern already used at `src/chip.rs:149-161`).

### FCA-10 — Public module/API layering — PASS

- Location: `src/lib.rs:20-33`, `src/prelude.rs:11-17`.
- Evidence: module tree and re-exports are consistent; the prelude mirrors the root re-exports plus `Clock`; `ProjectedChip` fields and `Motherboard::backend` are private; no private-in-public exposure; clippy is clean.
- Public fields: `Motherboard::layers` (covered by FCA-03) and `Testbench::{motherboard, bus}` (`src/sim.rs:28-31`), the latter an intentional testbench convenience.
- Verdict: layering is coherent; the only visibility concern is `Motherboard::layers`.

## Limitations

- No Miri, loom, or concurrency analysis was run; the framework is single-threaded by contract.
- The scratch probes verify macro expansion behavior only; they are not part of the repository test suite and are not committed.
- `git diff --check` does not inspect untracked files, including this record; the file was checked for trailing whitespace manually.
- `cargo test --workspace` does not cover `compiler/` because the root manifest declares no workspace; the compiler suite was run separately.
- Line numbers and quoted wording are valid at the audit revision and may drift after later edits.
