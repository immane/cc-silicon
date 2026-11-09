# ADR-0003: Parser Architecture and V1 AST Common Base

| Field | Value |
|---|---|
| Status | **Accepted (2026-11-09)** |
| Decision owner | T01 integrator |
| Supersedes | The open V1-vs-V2 parser conflict; `ID-PARSER-01` becomes accepted for V1 |
| Contract artifact | `t01-c01-c06/43` |
| Non-claims | No V1 parser producer, no V2 control-flow parser, no full C AST, no target codegen |

## 1. Context

The Stage A reports disagree on parser architecture:

- A06 proposes one `PARSE_TU` task that appends the complete V1 tree in one
  pre-order `AppendRecords` batch.
- A07 proposes a taskified parser with `NodeLinkRecord`, extended
  `ParseContext`, continuations, and incremental child links for V2 control
  flow.

The current implementation is already a single-task M1 parser (`PaTuChip`)
with an atomic nine-node tree. Running both architectures would create two AST
producers and incompatible child-link rules. No V1/V2 producer may proceed
until this choice is frozen.

## 2. Decision

### 2.1 V1 parser

V1 uses **one parser task, one atomic AST batch**:

- Task kind: existing `PARSE_TU`.
- The parser owns the complete V1 tree append for one translation unit.
- `NodeRecord.children` remains the authoritative ordered child list.
- Parent/children links are resolved inside the batch; a committed node is
  never linked to a child that is not in the same batch.
- No `NodeLinkRecord`, no `SetFrame` task, no parser self-continuation, and no
  extended `ParseContext` members are added in V1.
- The parser may use explicit host-language stacks/counters; recursion depth is
  bounded by the configured limits.
- Failure appends nothing; a partially committed V1 tree is not accepted.

This is a declared **V1 bootstrap fusion**, not the final parser architecture.

### 2.2 AST inventory

`NodeKind` keeps the `/39` names and order unchanged and appends the V1 common
base:

```text
Declaration, Identifier, ExpressionStatement, Assignment,
BinarySub, BinaryMul, BinaryDiv, BinaryRem, UnaryPlus, UnaryMinus
```

`NodeRecord` fields are unchanged: `kind, parent, children, first_token,
last_token, name, literal`. No `ordinal` field is added; `children` already
carries order. `name` is used on `Identifier`; `literal` on `IntLiteral`.
Parentheses are not nodes; their range belongs to the nearest enclosing
non-leaf node.

### 2.3 V2 parser

V2 may choose either:

1. extend the same single parser to V2, or
2. introduce the A07 taskified parser with `NodeLinkRecord` and extended
   `ParseContext`.

The choice must be frozen before V2 parsing starts. If the taskified path wins,
`NodeRecord.children` and `NodeLinkRecord` must not both represent children for
one node. Two AST producers must never run concurrently.

### 2.4 Verification

`VF05` remains the M1 shape verifier at `/43`. It explicitly fails closed on
the new V1 kinds until the V1 shape table and parser producer land. This is a
recorded gap, not a pass.

## 3. Rationale

- The atomic batch avoids the parent-after-child allocation problem without
  inventing a reservation/update protocol.
- It removes the colliding `PARSE` local 27+ window from V1.
- It preserves the existing `/39` AST names and snapshots; the change is
  append-only.
- It lets V1 implement declarations, locals, assignment, and arithmetic before
  the V2 control-flow record family is frozen.

## 4. Consequences

- The V1 parser chip will be large; this is accepted only as a bounded
  bootstrap and must be split if it hides whole language phases or exceeds the
  review budget.
- V2 control-flow chips remain blocked on the V2 parser/`NodeLinkRecord`
  decision.
- VF05 cannot validate V1 trees until the V1 shape table lands; the failure is
  explicit.
- No target/ABI/codegen behavior changes.

## 5. Alternatives rejected

| Alternative | Reason |
|---|---|
| Start with the taskified parser for V1 | Requires new child-link/update/reservation records before any V1 code; collides with frozen parse IDs and delays the common base |
| Keep both parsers selectable | Two AST producers, ambiguous child ownership, replay divergence |
| Add `ordinal` to `NodeRecord` | Redundant with ordered `children`; forces a snapshot/test migration for no semantic gain |
| Rename `/39` node kinds | Breaks frozen snapshots/hash for cosmetic alignment |

## 6. References

- [A06 V1 contract requirements](../reviews/dispatch/A06_V1_CONTRACT_REQUIREMENTS.md) §3.3
- [A07 V2 control-flow contract](../reviews/dispatch/A07_V2_CONTROL_FLOW_CONTRACT.md) §3.1–3.2
- [T01 compiler contract](../tasks/T01_COMPILER_CONTRACT.md) §7.1 `/43`
- [T05 parse chips](../tasks/T05_PARSE_CHIPS.md)
