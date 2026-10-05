# Gate 1: M1 First-Slice Freeze Checklist (const-fold chain)

| Field | Value |
|---|---|
| Status | **PROPOSED** — a work list, not a freeze and not code authorization |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | Each item names its decider; only the named authority signs |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/6` foundation), [M1 Part A proposal rev 40](M1_PART_A_CONTRACT_PROPOSAL.md), [CDR rev 56](CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md) |
| Non-goal | Parse/return chain, Part B, quota>1, full C vocabularies |

## 1. What Gate 1 is

Gate 1 freezes the **smallest closed loop that proves typed records can
flow through the protocol**: the M1-CL-05 const-fold chain
(T07 → T08 → T09) for `2 + 3`:

```text
committed LiteralRecord(2), LiteralRecord(3)   [T04 facts, hand-seeded fixture first]
  → ConstantRequest::Literal × 2, then Binary{Add}   [T07 checked operator]
  → exactly one ConstRecord(5), folded with checked addition   [T08]
  → T09 Constant op consuming RecordRef::Const without re-folding
```

The T04 literal-commit chips that *produce* the two `LiteralRecord`s
are slice 2 (they need the token→literal same-batch link, still open
per CDR rev 54). Gate 1 seeds the two literals from a frozen
hand-built fixture so the chain is testable without inventing the
link mechanism.

No language-chip code is written before every item below is frozen.
After the freeze, Wave 1 dispatches **only** the slice chips
([PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md)).

## 2. Freeze items

| # | Item | Decider | Acceptance |
|---|---|---|---|
| G1-1 | `LiteralRecord` exact 8 ordered fields (`token, kind, radix, suffix, value, negative, spelling, candidate_type`) in code, replacing `ReservedArena<LiteralId>` | T01 `[INT]` after T04/T08 owners | Struct + snapshot `encode_*` round-trip; field order pinned by test |
| G1-2 | M1 enums: `LiteralKind{Integer,Character,String}` (only `Integer` produced), `LiteralSuffix{None,U,L,UL,LL,ULL}` (only `None` produced), radix domain `{2,8,10,16}` (M1 decimal only) | T01 `[INT]` after T04/T08 | Closed enums in code; out-of-subset input → explicit unsupported diagnostic, never a silent default (CDR rev 53) |
| G1-3 | `Lx08CandidateType` M1 scope: closed `{Int}`, target-independent, no bit width | T01 `[INT]` after T04/T08 | M1 literals `2`/`3` carry `Int`; complete future member set explicitly **not** frozen |
| G1-4 | `ConstantRequest::{Literal,Binary}` variants + `ConstExprOp` (M1: `Add`) + `RequiredKind` (M1: `IntegerConstantExpression`) | T01/T07/T08 co-freeze (OPEN-03) | Committed-input path only (committed `NodeId`, committed literal refs in source order); fixture M1-CL-05 green |
| G1-5 | `ConstRecord` value carrier (rev-54 left formula/carrier open: `i128` superseded) + `ConstantResult{value, legality}` envelope carrier (open per rev 55) + `ConstLegality{Legal,NotConstantExpression,Unsupported}` | T01 `[INT]` after T08 | Exactly one `ConstRecord` per fold; `ConstRecord` identity/reuse rule stated |
| G1-6 | Slice task kinds registered (T04 literal-commit ×2 kinds or one parameterized kind, T07 const-request kind, T08 const-evaluate kind, T09 const-emit kind) with exact names/codes | T01 `[INT]` after group owners | `TaskKindRegistry` entries + `KindStatus` transitions covered by test |
| G1-7 | kind→stage rows for the slice kinds; `check_stage_assignment` enforces **these kinds** (global turn-on is a separate decision) | T01 `[INT]` + T02 | `StageUnassigned` reachable and tested for slice kinds; no behavior change for existing kinds |
| G1-8 | `STORE_OWNER_ALLOWLIST` rows for the slice chips (chip IDs assigned by T01; `tasks.ready` stays zero-writer) | T01 `[INT]` | Sole-writer violation test for each slice store/field |
| G1-9 | `AppendRecords` materialization for `Literal` + `Const` families only (all other families keep the explicit rejection) | T01 `[INT]` | Reserve/resolve/materialize round-trip; cycle-safe reservation per the T04 open item or a stated restriction |
| G1-10 | Snapshot bodies for the new records + freeze-test coverage (seed self-consistency, kind/stage/allowlist coverage, negative tests) | T01 `[INT]` | `cargo test` green; replay byte-identical |

## 3. Version rule

If any G1 item changes the hashed seed, the result is a **new
amendment version** (e.g. `/7`), not `/6` with the same hash. The
two-tier rule (CDR rev 50) still applies: the frozen seed participates
in the hash, post-seed runtime declarations do not. The version bump
decision belongs to the user on T01's proposal.

## 4. Explicitly deferred

- T04 token→literal production link (needs the cycle-safe ID
  reservation; slice 2).
- T05/T06/T07 parse→symbol→`return` chain and T09 `Return`/`Add`/
  `FunctionEnd` emission (slice 3).
- `ParseContext` final encoding, `ConversionOp`/`ConversionRole`
  inventory, VF06 exact kind/stage, target widths, Part B substrate.
- quota>1 paths stay measured post-freeze work; Gate 1 runs quota 1.

## 5. Done means

All G1-1–G1-10 accepted and signed, version bumped if the hash moved,
and the M1-CL-05 fixture runs end-to-end on frozen types. Only then do
the slice chips get dispatched to Wave 1 authors.
