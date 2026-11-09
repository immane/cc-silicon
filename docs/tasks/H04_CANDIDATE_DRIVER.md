# H04 Candidate Driver — Part A (M1 Evidence)

| Field | Value |
|---|---|
| Status | **PRESENT (Part A + `-E`/`-D`/`-U` + B3-03a `#define`/`#undef` + B3-03b quote include)** 2026-11-09 — binary `candidate` (`compiler/src/bin/candidate.rs`); host tooling, no contract change, no version bump |
| Scope | Part A evidence (source bytes → snapshot/trace/interpret for the M1 subset) plus host-only `-E` emission, `-D`/`-U` seeds, ordered source `#define`/`#undef`, and single-level quoted `#include "..."` beside the including file |
| Authority | [T00](T00_GCC_TORTURE_GATE.md) H04 + [M1 target acceptance](M1_TARGET_ACCEPTANCE.md) P7/§9 (Part A portion) |
| Non-goal | `-I`, multi-source, `-S`/`-c` emission (all explicit errors) |

## 1. What it is

The first true source-bytes-to-evidence run: the driver reads one `.c`
file and drives the frozen M1 worker pipeline from real tasks
(PP01→PP02→PP03→PP04→LX intern/classify/decode→PA→TY→SE→VF06/VF05→IR→
VF12→VF01) with a bounded drain per step — no seeded fixtures, no fixed
node/token positions (integer tokens, AST nodes, and the declarator are
found by tree walk). It performs no compilation itself and never shells
out to another C compiler.

## 2. Flags (spellings illustrative until the H04 contract freezes)

| Flag | Behavior |
|---|---|
| `-std=c11`, `-O0` | Accepted (only these values) |
| `-o <path>` | Write the evidence report to a file instead of stdout |
| `--emit-ir-snapshot` / `--emit-trace` / `--interpret-ir` | Select sections (none given = all) |
| `-S`, `-c` | **Refused**: fail-closed while the target is unverified (exit 2, Part B) |
| `-D NAME` | Defines `NAME` as `1` (bare-`-D` default); committed before preprocessing in flag order |
| `-D NAME=value` | Defines `NAME` with `value` scanned by the frozen PP scan into pp-tokens (empty `value` defines it as empty, still defined) |
| `-U NAME` | Undefines `NAME` (tombstone, supersedes by commit order); repeated `-D`/`-U` for one name resolve last-wins |
| `-E` | Emits the preprocessed source (PP28, host-only Part B) to stdout or `-o <path>` |
| `-I`, multi-source | **Deferred**: explicit error (exit 2), never a silent fallback |
| unknown flags | Usage error (exit 2) |

Exit codes: 0 = evidence; 1 = candidate diagnostic (structured message,
no evidence); 2 = driver error.

## 3. Evidence

```text
candidate evidence (t01-c01-c06/42)
input: main.c
snapshot: <hex>
trace: <hex>
interpret: value=05 negative=false
```

Repeated runs are byte-identical (pinned by `h04_candidate`). The B3-03a
source-macro path is pinned by `c46_pp_driver`.

## 4. Execution record

`compiler/tests/h04_candidate.rs` (18 tests: M1 model of return `5`,
determinism, invalid-input diagnostic with exit 1 and no evidence,
`-S` refusal with exit 2 and no output file, unknown-flag rejection,
`-o` report file with section selection, `-E` golden/determinism/`-o`/
`#error`/clash coverage, plus `-D` bare-default, `-D` value expansion,
`-U` undef, `-D`/`-U` last-wins, empty-`-D`-stays-defined, bad-macro-flag
exit 2, and `-D`-with-evidence intactness). B3-03a adds
`compiler/tests/c46_pp_driver.rs` (10 tests): source `#define`/`#undef`
ordering, benign/non-benign redefinition, leading-define-driven `#if`,
inactive/active post-conditional defines, `#error` fail-closed,
continuation rejection, directive removal, and deterministic reruns. See
[PP_DRIVER_SLICE.md](PP_DRIVER_SLICE.md). B3-03b adds
`compiler/tests/c47_pp_include_driver.rs` (12 tests) for the bounded
quote-include policy: header `#define`/`#undef`, plain text, subdirectories,
duplicate includes, missing/angle/traversal/nested/guard/pragma/
after-conditional refusals, determinism, and symlink escape. See
[B3_03B_LEAF_INCLUDE_POLICY.md](B3_03B_LEAF_INCLUDE_POLICY.md).

## 5. Explicitly deferred (still absent)

- `#line`/`#pragma` effects (T03/T01 logical-location and pragma carriers;
  W3 of B3-03).
- `-I`, angle/system includes, nested includes, include guards/`once`,
  multi-source compilation, and quoting edge cases (need the wider H04
  freeze; B3-03b only accepts quoted headers beside the including file).
- `-S`/`-c` target emission (Part B, probe-gated; the fail-closed rule
  stays until C02 is attested), plus the H02/H03/H05–H10 gate.
- Torture flag propagation matrix (H02/H03 runner integration).
