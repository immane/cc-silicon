# M1 Source Locals Slice — `int main(void){int x = 2; return x + 3;}`

| Field | Value |
|---|---|
| Status | **PROPOSED / NOT IMPLEMENTED** — requires a `/49` contract amendment |
| Goal | Produce local/arithmetic IR from source, emit AArch64, assemble/link/run, and return 5 |
| Blocks | General T11 codegen, H02/H03 execution evidence, Batch G |
| Non-claims | No calls, memory objects, branches, general ABI, object emission, or torture rate |

## 1. Why this slice

The target emitter already lowers automatic `int` locals, `LocalLoad`/`LocalStore`,
integer arithmetic, and one `Return` at IR level. The missing link is a real
frontend producer. The canonical `return 2+3;` path cannot exercise that
emitter because it folds to one `Constant` and never creates a local.

This slice adds one closed source shape:

```c
int main(void) { int x = 2; return x + 3; }
```

The expected IR is:

```text
Constant 2 -> v0
LocalStore x, v0
LocalLoad x -> v1
Constant 3 -> v2
Add v1, v2 -> v3
Return v3
```

Execution through native AArch64 must exit 5. The returned value must also be
reported by the symbolic IR interpreter, so `--emit-asm` and `--interpret-ir`
remain consistent.

## 2. Contract impact

The current frozen rule `parse.tu-fixed-nine-node-tree` describes only the
canonical `return 2+3;` tree. The new source shape produces a different tree
and needs these declared changes:

1. Replace the fixed-nine rule with a closed **shape-gated** TU rule: exactly
   one of the canonical constant-return shape or the locals shape, with the
   existing committed-token/range checks.
2. Add one semantic task kind for identifier expressions
   (`SEMANTIC` local 22) and its typed fact. The declaration itself needs no
   new semantic task: `SYMBOL_DECLARE` validates the local symbol, `SE_LITERAL`
   validates the initializer, and `SE_IDENTIFIER` validates each use.
3. Record the new normative identifiers and recompute the contract hash as
   `/49`; update `compiler/contracts/CONTRACT_VERSION`.

No IR, type, layout, constant, or diagnostic record shape changes are needed.
`IrOp::LocalLoad`/`LocalStore`/`Add` and `NodeKind::Declaration`/`Identifier`
already exist in `/43`/`/44`.

## 3. AST shape

Token order:

```text
int main ( void ) { int x = 2 ; return x + 3 ; } EOF
```

Pre-order nodes and ranges:

| Node | Children | Range |
|---|---|---|
| `TranslationUnit` | FunctionDefinition | 0..17 (EOF) |
| `FunctionDefinition` | Specifiers, Declarator(main), Compound | 0..16 |
| `Specifiers` | — | 0..0 |
| `Declarator(main)` | — | 1..4 |
| `Compound` | Declaration, Return | 5..16 |
| `Declaration` | Specifiers, Declarator(x), IntLiteral(2) | 6..10 |
| `Specifiers` | — | 6..6 |
| `Declarator(x)` | — | 7..7 |
| `IntLiteral(2)` | — | 9..9 |
| `Return` | BinaryAdd | 11..15 |
| `BinaryAdd` | Identifier(x), IntLiteral(3) | 12..14 |
| `Identifier(x)` | — | 12..12 |
| `IntLiteral(3)` | — | 14..14 |

The canonical shape's node layout and ranges remain unchanged.

## 4. Worker changes

### PA01 TU parser

- Detect one of the two closed token shapes before committing anything.
- Keep the existing canonical path byte-identical.
- For the locals shape, validate every keyword/punctuator spelling, resolve
  the two integer literals, and append the 13-node pre-order tree.
- Any other token sequence remains `Unsupported` and appends no nodes.

### TySymbol declare

- Accept `TypeKind::Int` declarations in a committed `ScopeKind::Block` as
  `SymbolKind::Object`, `Linkage::None`, `StorageDuration::Automatic`.
- Keep the existing function declaration behavior at file scope.
- Conflict checks use `(name, scope, kind)` as today.

### SE identifier

- New worker for `SEMANTIC_IDENTIFIER_EXPR`.
- Reads the `Identifier` node, resolves the symbol in the active block scope,
  requires canonical `int`, appends one `SemRecord`
  (`NonLvalue`, `EffectMask(0)`), and completes with it.
- Missing symbol and wrong type are typed failures, never fabricated facts.

### SE binary

- Accept either two integer-literal leaves (existing, still folds) or the
  locals shape: one `Identifier` leaf with a committed `int` `SemRecord` and
  one integer-literal leaf.
- For the locals shape, do not enqueue `const_fold`; append the `BinaryAdd`
  `SemRecord` only.
- Reject every other operand shape.

### SE return

Already accepts a `BinaryAdd` operand with an `int` `SemRecord`; only the
function-shape gate in `SE_FUNCTION` needs to allow the two-statement body.

### VF06

- Extend the checked set with `Identifier` facts.
- Require the locals body to contain exactly one `Declaration` followed by one
  `Return`; the declaration's declarator and initializer are shape-checked but
  have no separate semantic fact.
- Keep the canonical four-kind checked set unchanged.

### IR function lowering

- Dispatch on the committed body shape.
- Canonical shape: unchanged `Constant` + `Return`.
- Locals shape: validate the declaration/initializer/return facts and the
  local symbol, then append one `Function`, one `Block`, four `Value`s, and six
  `Instruction`s in order:
  `Constant(2)`, `LocalStore(x)`, `LocalLoad(x)`, `Constant(3)`, `Add`,
  `Return`.
- No folding of `x + 3`; the local load must remain observable IR.

### VF12 symbolic interpreter

- Keep the current constant-return fast path.
- Add a deterministic stack-machine path over the covered ops: `Constant`,
  `LocalLoad`, `LocalStore`, `Add`/`Sub`/`Mul`/`Div`/`Rem`, `Neg`, `Return`.
- Track `ValueId -> i64` and `SymbolId -> i64` maps in instruction order.
- Division/remainder by zero, missing values, uninitialized locals, branches,
  and calls fail loudly. The result is returned as the modeled integer.
- This interpreter validates IR semantics; it is not target execution.

### Candidate driver

- Detect the locals source shape after parse and run:
  file scope enter, local block scope enter, local declare, initializer
  literal check, identifier check, binary check, return check, function
  check, VF06, IR function, VF12, store invariant.
- Keep the canonical path unchanged.
- `--emit-asm` then emits the stack-machine frame and native execution must
  exit 5.

## 5. Test matrix

- Parser: canonical shape still 9 nodes; locals shape 13 nodes; wrong keyword,
  missing initializer, extra statements, and unsupported operators fail
  without partial node appends.
- TySymbol: automatic int object in block scope; file-scope object rejected;
  duplicate local rejected.
- SE identifier: hit, undeclared name, non-int symbol.
- SE binary: literal/literal still folds; identifier/literal appends without
  a fold child; unsupported operand shapes fail.
- IR: exact instruction order and `local` back-links for the locals shape;
  canonical shape unchanged.
- VF12: locals shape yields 5; divide-by-zero and unsupported ops fail.
- End-to-end macOS: emission contains local load/store and return.
- End-to-end native AArch64 Linux: assemble/link/run exits 5.
- Full compiler suite and clippy remain clean.

## 6. Explicit exclusions

- No assignment statements, compound assignment, multiple declarations,
  initializers other than one integer literal, or expression statements.
- No local address-taking, pointers, arrays, aggregates, or static locals.
- No calls, parameters, branches, loops, or memory objects.
- No `-S`/`-c`; the slice remains reachable through the bootstrap
  `--emit-asm` path until the full T11 chip family exists.
