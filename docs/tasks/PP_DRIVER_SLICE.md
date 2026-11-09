# PP Driver Slice — B3-03a (`#define` / `#undef`)

| Field | Value |
|---|---|
| Status | **IMPLEMENTED / INTEGRATOR VERIFIED (2026-11-09)** — host-only orchestration; no contract version bump |
| Scope | B3-03a W1: source `#define` / `#undef` in the candidate driver |
| Baseline | `t01-c01-c06/42` |
| Authority | [B3-03 plan](../reviews/dispatch/B3_03_PP_ORCHESTRATION_PLAN.md) W1; [DISPATCH_LEDGER](DISPATCH_LEDGER.md) D-19/B3-03 |
| Non-claims | No `#line`/`#pragma` effects, no full preprocessor, no target/probe/torture/M1 acceptance. Include is covered separately by [B3-03b](B3_03B_LEAF_INCLUDE_POLICY.md) |

## 1. Implemented behavior

The candidate driver replaces the fixed PP19→PP09→PP05 block with a bounded
host line walk over the committed PP-token stream:

1. `PP01 → PP02 → PP03 → PP04` produce the scanned stream.
2. Committed `-D`/`-U` seeds remain the initial macro table.
3. The stream is grouped into raw physical lines by `(source, line)`.
4. Leading source `#define`/`#undef` lines are applied through PP06/PP08 at
   their stream position; PP07 handles benign/non-benign redefinition.
5. Contiguous text runs are expanded through PP09 at their position, so a
   use before a definition is not expanded and a use after it is.
6. At the first conditional line, PP19 runs once over the remaining stream
   with the macro table accumulated so far. Active non-conditional
   directives are then handled by PP05 or rejected.
7. Every consumed directive line is removed; exactly one EOF token is kept.
8. `-E` continues through the frozen PP28 emit worker; evidence continues
   through LX/PA/IR.

## 2. Bounded fail-closed rules

| Input | Behavior |
|---|---|
| Leading `#define`/`#undef` | Applied in stream order |
| Benign redefinition | PP07 `Ack`; no append |
| Non-benign redefinition | Typed PP07 failure (exit 1) |
| Use before definition | Not expanded |
| Inactive post-conditional `#define` | Dropped by PP19 |
| Active post-conditional `#define`/`#undef` | Explicit `Unsupported`; never silently reordered |
| Directive continued with `\` | Explicit `Unsupported`; logical-line grouping is not frozen |
| `#include "..."` | B3-03b bounded quote include; see [leaf-include policy](B3_03B_LEAF_INCLUDE_POLICY.md) |
| `#line` / `#pragma` | Existing PP05 explicit `Unsupported` |
| `#error` | Existing PP02/PP05 diagnostic path |
| Null `#` line | Dropped |

## 3. Files

| File | Change |
|---|---|
| `compiler/src/bin/candidate.rs` | Register/route PP06/PP07/PP08; bounded line walk; segment expansion; directive removal |
| `compiler/src/bin/candidate/pp_driver.rs` (new) | Host-only raw-line grouping, directive classification, continuation guard |
| `compiler/tests/c46_pp_driver.rs` (new) | 10 end-to-end acceptance tests |

No chip logic, task kind, stage, allowlist, record, store, or contract hash
changed.

## 4. Acceptance

| Check | Result |
|---|---|
| `cargo test --locked --manifest-path compiler/Cargo.toml --test c46_pp_driver` | **PASS**, 10 tests |
| `cargo test --locked --manifest-path compiler/Cargo.toml --test h04_candidate` | **PASS**, 18 tests |
| `cargo test --locked --manifest-path compiler/Cargo.toml` | **PASS**, 706 tests / 53 suites at the current tree (includes B3-03b `c47`) |
| `cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings` | **PASS** |
| `cargo fmt --manifest-path compiler/Cargo.toml -- --check` | **PASS** |

`c46_pp_driver` covers: later-text expansion; use-before-define; `#undef`;
benign/non-benign redefinition; leading defines driving `#if`; inactive
post-conditional define dropped; active post-conditional define rejected;
include/error fail-closed; continuation rejection; deterministic reruns.
The `#define X 2+3` + `return X;` evidence path models `value=05`.

## 5. Explicit limitations

- B3-03b adds only quoted, single-level includes beside the including file;
  `-I`, angle/system headers, nested includes, guards/`once`, and `#line`/
  `#pragma` effects remain unimplemented and fail closed.
- A post-conditional source macro mutation is rejected even if a fuller
  control loop could interleave it correctly; that is T02/T03 W4 work.
- Raw physical-line grouping cannot represent a backslash-continued
  directive; such inputs are rejected.
- The M1 parser still accepts only the fixed `return <literal> + <literal>`
  shape, so a macro expanding to a bare `return 2;` fails at the parser even
  though expansion succeeded.
- No new schema, no contract bump, no target code generation.
