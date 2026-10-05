# H04 Candidate Driver — Part A (M1 Evidence)

| Field | Value |
|---|---|
| Status | **PRESENT (Part A)** 2026-10-06 — binary `candidate` (`compiler/src/bin/candidate.rs`); host tooling, no contract change, no version bump |
| Scope | Part A evidence only: source bytes → snapshot/trace/interpret for the M1 subset |
| Authority | [T00](T00_GCC_TORTURE_GATE.md) H04 + [M1 target acceptance](M1_TARGET_ACCEPTANCE.md) P7/§9 (Part A portion) |
| Non-goal | `-E`, `-I`/`-D`/`-U`, multi-source, `-S`/`-c` emission (all explicit errors) |

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
| `-E`, `-I`/`-D`/`-U`, multi-source | **Deferred**: explicit error (exit 2), never a silent fallback |
| unknown flags | Usage error (exit 2) |

Exit codes: 0 = evidence; 1 = candidate diagnostic (structured message,
no evidence); 2 = driver error.

## 3. Evidence

```text
candidate evidence (t01-c01-c06/19)
input: main.c
snapshot: <hex>
trace: <hex>
interpret: value=05 negative=false
```

Repeated runs are byte-identical (pinned by `h04_candidate`).

## 4. Execution record

`compiler/tests/h04_candidate.rs` (6 tests: M1 model of return `5`,
determinism, invalid-input diagnostic with exit 1 and no evidence,
`-S` refusal with exit 2 and no output file, unknown-flag rejection,
`-o` report file with section selection).

## 5. Explicitly deferred (still absent)

- `-E` preprocessed emission (needs PP28 `Preprocessed` production).
- `-I`/`-D`/`-U`, multi-source compilation, quoting edge cases (need
  macro/include/multi-source freezes).
- `-S`/`-c` target emission (Part B, probe-gated; the fail-closed rule
  stays until C02 is attested).
- Torture flag propagation matrix (H02/H03 runner integration).
