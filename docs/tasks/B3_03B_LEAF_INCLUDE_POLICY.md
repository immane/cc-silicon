# B3-03b — Bounded Quote-Include Policy

| Field | Value |
|---|---|
| Status | **ACCEPTED / IMPLEMENTED (2026-11-09)** — host-only bounded policy and candidate slice |
| Scope | Literal quoted `#include "name"` from the including file's directory |
| Baseline | `t01-c01-c06/42` (no contract change) |
| Authority | T01 integrator acceptance for B3-03b only; [B3-03 plan](../reviews/dispatch/B3_03_PP_ORCHESTRATION_PLAN.md) W2 |
| Supersedes | Nothing. `H04_I_FREEZE.md` remains a draft for `-I`, angle/system roots, and multi-source |
| Non-claims | No `-I`, system headers, nested includes, guards/`once`, multi-source, target execution, or full preprocessor capability |

## 1. Why a bounded subset

`H04_I_FREEZE.md` gates `-I`, system/sysroot roots, and multi-source on H01
substrate pins and a T01 config freeze. Those inputs do not exist. The
smallest useful include path that does not need them is a quoted header
resolved beside the including file. This policy accepts only that subset and
keeps everything else fail-closed.

## 2. Accepted behavior

1. Accept three-token lines of the form `# include "name"` only.
2. Reject angle-bracket names (`<...>`) because no H01 system roots are
   pinned.
3. Search only the including file's directory. No `-I`, no environment
   search list, no system directories, no current-directory fallback.
4. Resolve the including directory and candidate file with
   `canonicalize`; the candidate must remain under the canonical including
   directory. `..`, absolute paths, backslashes, NUL, empty components, and
   `.` components are refused.
5. Require a regular file, valid UTF-8 header name, and size at or below
   the configured `limits.max_source_bytes`.
6. Load each canonical path at most once per translation unit; a repeated
   include reuses the committed source. The same path under a different
   spelling is refused.
7. Cap one translation unit at 64 include occurrences.
8. Commit the header as a source named by its inner spelling, then run
   frozen PP01→PP04.
9. Resolve the main-line header token with PP17 and require the result to be
   the loaded source.
10. Splice with one per-line PP18 call (the frozen PP18 contract permits one
    header source per payload; the host supplies the single include line plus
    the outer EOF and drops the returned outer EOF before continuing).
11. Process header content under the same bounded rules:
    - leading `#define`/`#undef` → PP06/PP08;
    - text runs → PP09 at their stream position;
    - null `#` → drop;
    - nested `#include`/`include_next`, conditionals/guards, `#line`,
      `#pragma`, `#error`, `#warning`, unknown or malformed directives →
      explicit `Unsupported`.
12. Includes must appear before the first conditional in the main source.
    An active include after PP19 is explicitly refused.

## 3. Failure behavior

Every refusal is a candidate diagnostic (exit 1), not a silent skip, empty
include, fallback to another preprocessor, or partial evidence. Missing files,
resolved-source mismatches, oversized files, and PP17/PP18 failures propagate
loudly.

## 4. Security boundaries

- Host file access is read-only and confined to the canonical including
  directory; symlink escapes are rejected.
- No environment sampling, no hardcoded `/usr/include`, no external
  preprocessor, no subprocess.
- Header bytes are committed through the checked source path and subject to
  `max_source_bytes`.
- Diagnostics name the include path but do not dump unrelated file contents.

## 5. Explicit non-goals

- `-I`, angle/system/sysroot includes, `include_next`, macro-generated header
  names.
- Nested includes, include depth, include guards, `#pragma once`, conditionals
  in headers.
- `#line` logical locations and pragma effects.
- Multi-source translation units, `-S`/`-c`, target ABI/probe/torture results.
- A general dependency-file or search-path contract.

`H04_I_FREEZE.md` remains the place for the wider `-I`/system/multi-source
freeze. This policy does not amend T01 and does not change the `/42` hash.

## 6. Acceptance evidence

| Check | Result |
|---|---|
| `cargo test --locked --manifest-path compiler/Cargo.toml --test c47_pp_include_driver` | **PASS**, 12 tests |
| `cargo test --locked --manifest-path compiler/Cargo.toml --test c46_pp_driver` | **PASS**, 10 tests |
| `cargo test --locked --manifest-path compiler/Cargo.toml --test h04_candidate` | **PASS**, 18 tests |
| Full compiler suite | **PASS**, 708 tests / 54 suites |
| `cargo clippy ... --all-targets -- -D warnings` | **PASS** |
| `cargo fmt ... -- --check` | **PASS** |

`c47` covers header `#define` expansion, plain text splicing, header `#undef`,
subdirectories, duplicate includes, missing files, angle/traversal refusal,
nested includes, guards/`once`, after-conditional refusal, determinism, and a
Unix symlink escape.
