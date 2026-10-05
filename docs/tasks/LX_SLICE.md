# Wave 2 Slice 2: LX Tokenize/Classify/Decode Freeze (`/11`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-05** as `t01-c01-c06/11` (hash `4484ae13…ce69`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/10` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/11` delta), [T04](T04_LEX_CHIPS.md) §M1 candidate, [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-LX-01`–`M1-LX-07` |
| Non-goal | Float/char/string literals, adjacent-string merging, GNU extensions, PP04 token production, T04 provenance carrier, LX17 orchestrator fan-in |

## 1. What this slice is

Three workers turning seeded committed PP tokens into committed C tokens
and integer literals: `LxInternChip` (LX02 scope) → `LxClassifyChip`
(LX01/LX03/LX04 scope) → `LxDecodeLiteralChip` (LX05–LX08 integer scope).
PP-token/span fixtures are seeded as committed records (PP04 production
stays future work, and must reproduce these fixtures when it lands). The
acceptance fixture is **`L1-LX-01`** (below), explicitly **not** the full
`M1-LX-0x` chain, which needs real PP04 output.

## 2. Frozen decisions

| # | Decision | Rationale |
|---|---|---|
| L1-1 | `PpTokenRecord { kind, span, spelling }`, `PpTokenKind {Identifier, PpNumber, Punctuator, Eof}`; `TokenRecord { kind, span, name, pp_token }`, `TokenKind {Keyword, Identifier, Punctuator, Integer, Eof}` | Minimal M1-closed shapes; no forward literal link (one-directional back-link only) |
| L1-2 | Identifier AND keyword spellings intern (T04 rule stands); M1 fixture interns four spellings (`int`, `main`, `void`, `return`) | Resolves NI-02: PP has no keyword distinction, and LX03 needs interned keyword names |
| L1-3 | Full C11 keyword table, membership-tested | Fixed language fact, not fixture-special-casing |
| L1-4 | Integer decode: decimal digits only, suffix `None`, `Int` candidate, unsigned; lone-CR-style non-M1 inputs are explicit `Unsupported` | `M1-LX-04`/`05` symbolic facts; bit patterns stay probe-gated |
| L1-5 | Decode input is the committed PP spelling/kind; `LiteralRecord.token` is the output-side publish-time back-link | DOC-10 resolution, already recorded in T04 |
| L1-6 | `lex.intern(16)` / `lex.classify(17)` / `lex.decode_literal(18)`, all stage 2; three allowlist rows; chips 4/5/6 | Slice registration |
| L1-7 | `Name` bodies intern lookup-first with read-only capacity simulation (atomicity preserved); future-dated `Name` refs in `Complete` are `UnpredictedRecord` | Workers read committed state only; no prediction in workers |
| L1-8 | `pp.tokens`/`tokens` arenas become typed on freeze; snapshot encodes bodies; `PPTOKEN/TOKEN_KIND_NAMES` + record-field lists join the hash | Group-owner schema freeze pattern |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c11_lex.rs`
(8 tests: kind/stage/allowlist freeze, keyword table, four-name interning,
M1 kind sequence, literal back-links, dangling/non-integer/hex negatives,
stage/layer + manifest gates, snapshot replay determinism).

## 4. Explicitly deferred

- `TokenRecord.literal` forward link (needs the T01 link-freeze; back-link suffices).
- LX17 as an orchestrator chip (fan-in needs the consume-envelope decision).
- PP04 real production (must reproduce the seeded fixtures; add a conformance test then).
- Non-M1 literal forms, `names.entries` cross-group policy, numeric codes.
