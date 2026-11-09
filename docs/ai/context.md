# AI Session Context — cc-silicon compiler (`t01-c01-c06/43`, working tree)

> Living handoff note for AI agents continuing this work. Updated
> 2026-11-09 after accepting `/43` parser architecture/V1 AST, implementing
> B2-02 `/42`, B-INT-00, B3-03a source `#define`/`#undef`, B3-03b bounded
> quote include, and the target probe, with the full compiler suite green
> (710 tests / 55 suites; clippy clean).
> The frozen contract (`compiler/contracts/CONTRACT_VERSION`) plus
> `docs/tasks/T01_COMPILER_CONTRACT.md` §7.1 remain authoritative; this
> file is an index, not a freeze.

## 1. Where we are

- Frozen artifact: `t01-c01-c06/43` (parser architecture/V1 AST, tagged typed
  constants, determinism/limits/replay), hash
  `637a0484a4e1870b8e66b04dfd5ad79921d0ad1736972ce7f6a90243bce8df30`.
- Implemented and test-green at `/41`: append handles/bodies in snapshots,
  stage-first scheduling with zero-quota fail-closed, macro/store/result
  version accounting, `Task.id`, pin-aware trace decoder/first divergence,
  and the Host `ReplayEvidence` envelope. Evidence: `c05_codec` 29,
  `c06_routing` 19, `c07_limits` 20, `c03_task` 48, `c08_gate1` 22,
  `c11_lex` 11, `freeze` 15.
- Implemented and test-green at `/42`: `ConstRecord { ty, value:
  ConstValue::Integer(..) }`, fold type resolution from committed
  `SemRecord.ty` (binary) / unique canonical `int` (literal), T09/VF12
  `ir.op-immediate-type` checks, VF14 integer gate decode, canonical typed
  snapshot body. Evidence: `c45_const_ty` 8, `c08_gate1` 22, `c15_ir` 13,
  `c17_vf12` 6, `c38_vf14` 10, `c05_codec` 29, `freeze` 15; direct
  candidate emits `/42` and `value=05` with byte-identical repeated output.
- `/40` remains the M1 Part A A3 closure: SE29 dispatch, VF06 four-kind
  typed-fact set, T09 committed-fact consumption (`c14_se` 16,
  `c15_ir` 13, `c43_a3` 5, `h04_candidate` 18).
- B-INT-00 is complete: all nine unregistered parse/lex drafts are parked,
  clippy is clean, and no candidate ID is frozen
  (`docs/tasks/B_INT_00_DRAFT_DISPOSITION.md`).
- Not done: full `Snapshot::from_bytes`/restore, quota>1 order, VF13
  projections, per-stage queues, and `StorePatch` materialization for
  `/47`; B3-03 PP driver orchestration implementation (design complete,
  gated by H04/T03/T02); H05 isolated execution; official T00
  reference-baseline/CI-substrate freeze; Part B target codegen; full M1
  acceptance. One pinned containerized AArch64 probe report is now
  H07-classified and attested by `c48_target_probe`; the global target state
  remains unverified. No target execution or GCC pass rate is claimed.
- Branch: `wave4-development`. Commits through B3-03b are local (the
  B3-03b commit follows `9d38cdd`); no push was performed (no force-push).
- Dispatcher mode is ACTIVE (user instruction): serial freeze by the
  integrator, parallel implementation via subagents on disjoint files,
  integrate + verify per slice, continue without stopping.
  Open threads in §6 are ordered next-up.

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
| `/23` | PP macro definitions + undef (+Macro family) | `preprocess/{pp_define,pp_redefine,pp_undef}.rs` | `c23_macro` 10 |
| `/24` | PP macro expansion (+`function_like` fix) | `preprocess/{pp_invoke,pp_substitute}.rs` | `c24_expand` 12 |
| `/25` | PP include resolve + enter | `preprocess/{pp_resolve,pp_enter}.rs` | `c25_include` 8 |
| `/26` | PP variadic invocation | `preprocess/pp_variadic.rs` | `c26_variadic` 10 |
| `/27` | PP builtin macros | `preprocess/pp_builtin.rs` | `c27_builtin` 9 |
| `/28` | PP line directives | `preprocess/pp_line.rs` | `c28_line` 10 |
| `/29` | PP pragma dispatch | `preprocess/pp_pragma.rs` | `c29_pragma` 9 |
| `/30` | PP expansion map | `preprocess/pp_expand_map.rs` | `c30_expand_map` 10 |
| `/31` | PP emit | `preprocess/pp_emit.rs` | `c31_emit` 11 |
| `/32` | LX float | `lex/{lx_float_syntax,lx_float_value}.rs` | `c32_float` 12 |
| `/33` | LX string | `lex/{lx_escape,lx_char,lx_string}.rs` | `c33_string` 14 |
| `/34` | PA decl | `parse/{pa_external,pa_specifier,pa_declarator,pa_block}.rs` | `c34_parse` 12 |
| `/35` | PA expr | `parse/{pa_binary,pa_unary}.rs` | `c35_expr` 10 |
| `/36` | PA recovery | `parse/{pa_pod,pa_recovery}.rs` | `c36_recovery` 11 |
| `/37` | T08 const-branch | `constant_layout_init/fold_branch.rs` | `c37_const_branch` 10 |
| `/38` | VF14 evidence-classify | `verify/vf_evidence.rs` | `c38_vf14` 10 |
| `/39` | SE29 function-definition | `semantic/se_function.rs` | `c39_sefunc` 8 |
| `/40` | M1 Part A A3 closure (SE29 wiring + VF06 four-kind + T09 consumption) | `bin/candidate.rs`, `verify/vf_invariant.rs`, `ir_lower/ir_function.rs` | `c14_se` 16, `c15_ir` 13, `c43_a3` 5, `h04_candidate` 18 |
| `/41` | B2-00 determinism/limits/replay (append bodies, stage-first, versions, pin-aware trace) | `snapshot.rs`, `routing.rs`, `commit.rs`, `bus.rs` | `c05_codec` 29, `c06_routing` 19, `c07_limits` 20, full 723/50 |
| `/42` | B2-02 tagged, typed constants (`ConstRecord.ty`, integer payload, immediate-type checks) | `bus.rs`, `snapshot.rs`, `chips/constant_layout_init/fold.rs`, `chips/ir_lower/ir_function.rs`, `chips/verify/{vf_interpret,vf_evidence}.rs` | `c45_const_ty` 8, `c08_gate1` 22, `c15_ir` 13, `c17_vf12` 6, `c38_vf14` 10, full 684/51 |
| — | B3-03a source `#define`/`#undef` driver orchestration (host-only, no version bump) | `compiler/src/bin/candidate.rs`, `compiler/src/bin/candidate/pp_driver.rs` | `c46_pp_driver` 10 |
| — | B3-03b bounded quote-include driver slice (host-only, no version bump) | `candidate/pp_include.rs`, candidate PP17/PP18 wiring | `c47_pp_include_driver` 12 |
| `/43` | Parser architecture + V1 AST common base (single-task V1 parser decision, append-only NodeKind) | `bus.rs`, `snapshot.rs`, `contract.rs`, `contracts/CONTRACT_VERSION` | `c49_ast_schema` 2, full 710/55 |
| — | H01/H07 containerized target probe + attestation test (no contract bump) | `tools/torture/probe/{run-probe.sh,classify.py}`, `compiler/tests/c48_target_probe.rs` | `c48_target_probe` 2 |
| — | H04 Part A candidate driver (no version bump) | `compiler/src/bin/candidate.rs` | `h04_candidate` 18 |

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
  version, new `NORMATIVE_RULES` ids, recompute hash via
  `cargo run --manifest-path compiler/Cargo.toml --example freeze_hash`,
  sync `CONTRACT_VERSION`, `contract_version_file()`, `freeze.rs`,
  `c08_gate1.rs` counts, `compiler/README.md`.
- **Registries/schemas are cumulative AND LINEAR**: each `*_slice()` fn
  extends the previous slice's (chain: `pp_slice` → `vf12` → `vf05` →
  `vf01` → `pp_directive` → `pp_conditional` → `pp_macro` → `pp_expand`
  → `pp_include`); never edit a frozen slice fn; a new phase extending an
  older branch must extend the LATEST (see `/21` chain fix). Frozen kind
  locals are sacred.
- **Version-pin files must move with every bump**: `freeze.rs`,
  `c08_gate1.rs` (version asserts + `STAGE_ASSIGNMENT.len()`),
  `c04_manifest.rs` + `c08` allowlist lens, `c03_task.rs` + `c05_codec.rs`
  closed inventories (families/tags), `RECORD_KINDS`, marker asserts in
  slice tests (`t01-c01-c06/NN`), `h04_candidate.rs` evidence header,
  both READMEs + this file.
- **Frozen-join semantics** (`commit.rs::poll_await_joins`, `/9`): any
  failed awaited child fails the waiter directly, REUSING the first
  failed child's diagnostic (no new record). Resume-compute therefore
  runs only in the all-`Completed` case — design fan-out/await slices
  around this; never plan aggregate-message resumes.
- **Supersede protocol**: when a freeze supersedes pinned test
  assertions, the INTEGRATOR updates those pins (never the chip owner,
  never silent) and records it in the slice execution record.
- **Chip-lint/clippy gates** (toolchain 1.99): no closures in `compute`
  (same-file `fn` helpers only); watch `manual_strip`,
  `type_complexity` (alias long tuples), `needless_range_loop`,
  `collapsible_match`, `manual_contains`, `manual_range_contains`,
  `explicit_auto_deref`, `question_mark`, unused imports/vars in tests.
  Always run `cargo fmt` for the touched package before `--check`.
- **Subagent dispatch protocol**: one disjoint file per agent; brief must
  cite the freeze doc sections + template + frozen-join rule; forbid
  shared-file edits and commits; require temp-wire + revert proof for
  unwired files; DEFECT (structured, no guessing) on ambiguity. Rework
  via continued `sessionID`. Verify serially at integration (parallel
  `cargo` invocations contend). Integrator reviews every diff, writes
  all tests/docs/version bumps, runs the FULL §5 suite, then commits.
- **Docs per slice**: `docs/tasks/<NAME>_SLICE.md` (freeze items,
  execution record, explicit deferrals + amendments) + T01 §7.1 amendment
  paragraph + `CHIP_PLAN.md` status + README version lines.

## 4. Layout

- `compiler/src/chips/<group>/`: `preprocess`, `lex`, `parse`, `types`,
  `symbols`, `semantic`, `verify`, `ir_lower`, `constant_layout_init`.
  Group `mod.rs` files are integrator-owned; chip files import the driver
  boundary via `crate::chips::{...}` and siblings via `self::`/`super::`
  only (chip-lint rejects bare module roots). Pure cross-chip helpers are
  copied per file (precedent: `compose_map`), not shared.
- `compiler/src/bin/candidate.rs`: H04 Part A driver (host orchestration;
  runs the M1 pipeline steps incl. PP05/PP09/PP19; no PP06 fan-out yet).
- `compiler/tests/cXX_*.rs`: one acceptance file per slice;
  `h04_candidate.rs` for the driver.
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

Run `cargo` from the repo root (relative `--manifest-path` breaks one
directory down). `touch compiler/src/lib.rs` to force a fresh clippy
pass when results look cached/stale.

## 6. Open threads (do not treat as settled)

- NEXT UP: Batch B `/44` IR/CFG schema (successors + Branch/CondBranch,
  memory/effects/call/object shapes); then Batch C `/45` layout/init/constant,
  Batch D `/46` diagnostics, Batch E H05 runner, Batch F official target
  baseline, and Batch G torture chain. B3-03c `#line`/`#pragma` and the wider
  H04 include policy remain open. Continue the PP remainder in pipeline order —
  then T04 LX
  remainder (adjacent strings, literal extensions), T05/T06/T07 remainders, T08
  layout/init, T09 IR remainder, T10 optimize, T12 GNU, T02 control +
  `/6` co-freeze (H6/H9), T11 target (probe-gated → Unsupported shells
  per R2 until a Linux runner exists), T13 VF02–04/VF13 (H6/H9-gated),
  H04 remainder (`-E`/`-I`/`-D`/`-U`/multi-source/torture flags), H02/H03
  runner, H05–H10 gate.
- T02 control subset (blocked on the `/6` co-freeze) — biggest gap after
  language coverage; do NOT implement control chips on draft semantics.
- T13 VF remainder on real records (VF02–04/VF13–14 need H6/H9 batch
  fixtures; VF01/VF05/VF06/VF12-M1 exist).
- L1 carryover: multi-line-comment tail classifies non-directive (loud
  downstream failure, recorded in `/21` doc).
- Quota>1 (unaccepted); official probe substrate/baseline unprovisioned (one
  containerized report/attestation exists); no torture corpus.
- Known tech debt: `poll_await_joins` closure loop is O(waiters²),
  accepted for the small profile (PCR-13); PP predicate logic is
  copy-per-chip by precedent (drift-pinned by cross tests).
