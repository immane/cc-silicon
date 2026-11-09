# ADR-0004: IR/CFG Common-Base Schema

| Field | Value |
|---|---|
| Status | **Accepted (2026-11-09)** |
| Decision owner | T01 integrator |
| Contract artifact | `t01-c01-c06/44` |
| Supersedes | Open A06/A15 IR shape conflict for the V1/V2 common base |
| Non-claims | No full lowering producer, no CFG verifier, no target codegen |

## 1. Context

`/15` froze a minimal M1 IR (`Constant`, `Return`) with `InstructionRecord`
fields `op, block, operands, immediate, result`. A06 proposes V1 locals and
arithmetic; A15 proposes CFG successors, memory/effect/call shapes, and
terminator semantics. The common base must be append-only and must not create a
second CFG edge representation.

## 2. Decision

### 2.1 Instruction record

Append two fields to `InstructionRecord`, preserving the existing field order:

```text
local: Option<SymbolId>      // LocalLoad/LocalStore only
successors: Vec<BlockId>     // Branch/CondBranch targets, ordered
```

- `local` is a committed `SymbolId`; no private local table is introduced.
- `successors` is the authoritative ordered successor list. No separate
  `CfgEdgeRecord` family is added.
- Non-terminator instructions have empty `successors`; `Return` remains a
  terminator with no successors.
- `local` is `Some` only for `LocalLoad`/`LocalStore`; `None` otherwise.

### 2.2 Op inventory

The `/15` names remain unchanged. Append:

```text
local_load, local_store, add, sub, mul, div, rem, neg,
branch, cond_branch
```

V1 uses `LocalLoad`, `LocalStore`, arithmetic, and `Neg`; `Branch` and
`CondBranch` are the V2 CFG vocabulary. The op table is closed for `/44`; later
ops append, never renumber.

### 2.3 Effects

Add the V1 effect bit `EFFECT_WRITES_LOCAL = 1 << 0`. Existing M1 facts remain
`EffectMask(0)`; V1 assignment/initialized declaration may set this bit.
Unknown bits fail verification.

### 2.4 Verification

CFG and local-link verification (`VF07`/`VF08`) remain future work. This ADR
freezes the carriers only. No producer may assume verifier coverage until the
VF slice lands.

## 3. Consequences

- Existing M1 instructions migrate by adding `local: None, successors: vec![]`.
- Snapshot body encoding appends local-presence and successor count/list.
- Branch terminators are represented without a parallel edge family.
- Target allocation can consume `SymbolId` locals later without an IR-local
  side table.
- No task kind, stage, allowlist, arena, or store changes.

## 4. Alternatives rejected

| Alternative | Reason |
|---|---|
| Separate `CfgEdgeRecord` family | Duplicates successor ownership and creates two CFG representations |
| `ordinal`/edge indexes | Ordered `successors` already encodes branch order |
| Add memory/call payloads now | No frozen object/address/call shape; append later rather than invent a placeholder |
| Keep IR M1-only | Blocks V1 locals and V2 control flow |

## 5. References

- [A06 V1 contract requirements](../reviews/dispatch/A06_V1_CONTRACT_REQUIREMENTS.md) §3.5
- [A15 IR contract requirements](../reviews/dispatch/A15_IR_CONTRACT_REQUIREMENTS.md) §4.6–4.8
- [T01 compiler contract](../tasks/T01_COMPILER_CONTRACT.md) §7.1 `/44`
- [T09 IR lowering](../tasks/T09_IR_LOWER_CHIPS.md)
