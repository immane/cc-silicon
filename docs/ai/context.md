# AI Session Context — cc-silicon compiler (`t01-c01-c06/23`)

> Living handoff note for AI agents continuing this work. Updated
> 2026-10-06 after the Wave 2 slice 14 (`/23`) freeze. The frozen contract
> (`compiler/contracts/CONTRACT_VERSION`) plus `docs/tasks/T01_COMPILER_CONTRACT.md`
> §7.1 remain authoritative; this file is an index, not a freeze.

## 1. Where we are

- Frozen artifact: `t01-c01-c06/23`, hash
  `cf8f2194c9b599bfd822e685dbb791f3b2d44b550bc5536705972920785b1ca0`.
- Branch: `initial-compiler-development` (PR #13 targets `main`).
- The M1 C frontend is **closed end-to-end, symbolically modeled,
  syntax-checked, and store-checked**: seeded source bytes flow
  PP01→PP02→PP03→PP04→LX(intern→classify→decode)→PA→TY→SE→fold→IR→VF12
  with no hand-built values (`M1-CL-05` handoff complete at `/15`, meaning
  modeled at `/17`), VF05 re-verifies the token↔AST contract at `/18`,
  and VF01 re-verifies ID ownership/span bounds/reserved-emptiness at
  `/19`.
- Rule: **one serial slice at a time** — each slice freezes
  schemas/kinds/stages/allowlist first, then implements. Never dispatch a
  wave before its freeze lands (rework is guaranteed otherwise).

## 2. Slice history (each amends the version via the R1 auto-bump rule)

| Ver | Slice | Workers (`chips/<group>/`) | Acceptance |
|---|---|---|---|
| `/7` | Gate 1 const-fold types | `constant_layout_init/fold.rs` (FoldChip) | `c08_gate1` 21 |
| `/8` | Worker integration | driver (`chips/mod.rs`) | strict decode, budgets |
| `/9` | Pre-chip readiness fixes (PCR-01..13) | join/idle-drain, transition rule, lint | `c09_readiness` 14 |
| `/10` | PP01 source-normalize | `preprocess/pp_normalize.rs` | `c10_pp01` 7 |
| `/11` | LX tokenize/classify/decode | `lex/{lx_intern,lx_classify,lx_decode}.rs` | `c11_lex` 8 |
| `/12` | PA translation unit | `parse/pa_tu.rs` (9-node fixed tree) | `c12_parse` 5 |
| `/13` | TY scope/symbol/type | `types/{ty_types,ty_conv}.rs`, `symbols/{ty_scope,ty_symbol}.rs` | `c13_ty` 8 |
| `/14` | SE checks + VF06 | `semantic/{se_literal,se_binary,se_return}.rs`, `verify/vf_invariant.rs` | `c14_se` 9 |
| `/15` | IR function lowering | `ir_lower/ir_function.rs` | `c15_ir` 7 |
| `/16` | PP splice/comment/scan | `preprocess/{pp_splice,pp_comment,pp_scan}.rs` | `c16_pp` 8 |
| `/17` | VF12 symbolic interpret | `verify/vf_interpret.rs` | `c17_vf12` 6 |
| `/18` | VF05 token-AST invariant | `verify/vf_syntax.rs` | `c18_vf05` 6 |
| `/19` | VF01 store invariant | `verify/vf_store.rs` | `c19_vf01` 7 |
| `/20` | PP full-token scan | `preprocess/{pp_comment,pp_scan}.rs` (amended) | `c20_ppscan` 8 |
| `/21` | PP directive dispatch + diagnostic | `preprocess/{pp_directive,pp_diagnostic}.rs` | `c21_directive` 8 |
| `/22` | PP conditional inclusion | `preprocess/pp_conditional.rs` | `c22_conditional` 8 |
| `/23` | PP macro definitions + undef | `preprocess/{pp_define,pp_redefine,pp_undef}.rs` | `c23_macro` 10 |

## 3. Patterns every new slice must follow

- **Worker template** (`chips/mod.rs` header): narrow `Input` projection +
  projector (adapter owns the bus) + pure `compute(&Input)`; full bus never
  reaches compute; ZST enforced at registration; stage/layer enforced on the
  driver path; every failure is a `Fail` proposal, never silent/panic.
- **Commit rules**: one transition per task per batch (`TaskNotTransitioned`
  otherwise); `AppendRecords` 1:1 bodies with canonical `DraftRef`
  positions; predicted-ID verification for `Complete` carriers; capacity
  preflight before any mutation; await-all joins with idle drain.
- **Contract bump**: any protocol/encoding/shape change → new amendment
  version (`/17`, …), new `NORMATIVE_RULES` ids, recompute hash via
  `cargo run --manifest-path compiler/Cargo.toml --example freeze_hash`,
  sync `CONTRACT_VERSION`, `contract_version_file()`, `freeze.rs`,
  `c08_gate1.rs` counts, `compiler/README.md`.
- **Registries/schemas are cumulative**: `*_slice()` fns extend the
  previous one (`pp_slice()` = 29 entries); `*_slice()` schemas likewise.
  Never edit a frozen slice fn — add a new one. Frozen kind locals are
  sacred (a past bulk-rename once corrupted two Gate 1 kinds; tests caught
  it via registry-length asserts).
- **Docs per slice**: `docs/tasks/<NAME>_SLICE.md` (freeze items P/Q/R…,
  execution record, explicit deferrals) + T01 §7.1 amendment paragraph +
  `CHIP_PLAN.md` status + README version lines.

## 4. Layout

- `compiler/src/chips/<group>/`: `preprocess`, `lex`, `parse`, `types`,
  `symbols`, `semantic`, `verify`, `ir_lower`, `constant_layout_init`.
  Group `mod.rs` files are integrator-owned; chip files import the driver
  boundary via `crate::chips::{...}` and siblings via `self::`/`super::`
  only (chip-lint rejects bare module roots).
- `compiler/tests/cXX_*.rs`: one acceptance file per slice.
- Reviews live in `docs/reviews/` (read-only records, incl. the 12
  pre-chip audits + the readiness review + repro).

## 5. Verification (all must pass)

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --manifest-path tools/chip-lint/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/chip-lint/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/chip-lint/Cargo.toml
cargo fmt --manifest-path compiler/Cargo.toml -- --check
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path compiler/Cargo.toml
cargo fmt --manifest-path tools/torture/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/torture/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/torture/Cargo.toml
bash tools/torture/probe/tests/run-tests.sh
cargo run --locked --manifest-path tools/chip-lint/Cargo.toml -- compiler/src/chips
git diff --check
```

## 6. Open threads (do not treat as settled)

- T02 control subset (blocked on the `/6` co-freeze) + H04 remainder (`-E`/`-I`/`-D`/`-U`/multi-source/torture flags; Part A driver present) — biggest gap.
- T13 VF remainder on real records (VF02–04/VF13–14; VF01/VF05/VF06/VF12-M1 exist).
- File-Enter edge auto-firing; `TokenRecord.literal` forward link;
  `ConversionPlan`; query-point lookup ordering; FunctionEnd/IR28 hook
  (OB-26); PP diagnostic taxonomy (F4); full scan-state machine;
  `Preprocessed` production; macro/include/multi-source; quota>1 (unaccepted).
- Known tech debt: `poll_await_joins` closure loop is O(waiters²),
  accepted for the small profile (PCR-13).
