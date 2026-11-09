# M1 Source Locals Slice — Expressions and Assignments

| Field | Value |
|---|---|
| Status | **IMPLEMENTED / VERIFIED (2026-11-10)** as contract `/53` (`/49` local return, `/50` local add + modeled integer, `/51` binary operators, `/52` expression trees, `/53` assignments) |
| Goal | Parse, check, lower, symbolically model, and execute a closed local expression tree over one automatic `int` and integer literals |
| Evidence | `c56_source_locals` 10 macOS / 13 native AArch64 Linux; native runs cover precedence, parentheses, literal-left forms, unary minus, and assignment order |
| Non-claims | No calls, memory objects, branches, general ABI, object emission, or torture rate |

## 1. Supported source shapes

```c
int main(void) { int x = 2; return x; }
int main(void) { int x = 2; return x + 3; }
int main(void) { int x = 2; return (x + 3) * 2; }
int main(void) { int x = 2; return 8 - x * 3; }
int main(void) { int x = 2; return -(x + 3) * 2; }
int main(void) { int x = 2; return +(x + 1); }
int main(void) { int x = 2; x = x + 3; return x; }
int main(void) { int x = 2; x = x + 3; x = x * 2; return x; }
```

The return expression is a closed tree over:

- the single automatic `int` local `x`;
- decimal integer literals;
- binary `+ - * / %` with C precedence and left associativity;
- parentheses;
- unary `+` and `-`.

Assignments are `x = <expression>;` statements between the initializer and the
return; they execute in source order. The initializer remains exactly one
integer literal. The canonical
`int main(void){return 2+3;}` path is unchanged.

Expected examples:

| Source expression | Modeled | Native exit |
|---|---|---|
| `x` | 2 | 2 |
| `x + 3` | 5 | 5 |
| `x - 3` | -1 | 255 |
| `x * 3` | 6 | 6 |
| `x / 3` | 0 | 0 |
| `x % 3` | 2 | 2 |
| `x + 3 * 2` | 8 | 8 |
| `(x + 3) * 2` | 10 | 10 |
| `8 - x * 3` | 2 | 2 |
| `-(x + 3) * 2` | -10 | 246 |

## 2. Contract impact

### `/49` local return

- Shape-gated `PARSE_TU`: canonical constant return or one local declaration
  plus an identifier return.
- `TySymbolChip` declares a block-scope automatic `int`.
- `semantic.identifier_expr` (`SEMANTIC` local 22, `SE_ID_CHIP = ChipId(58)`,
  stage 4, one `sem.records` allowlist row) checks the binding.
- `SE_RETURN` accepts an identifier operand; `SE_FUNCTION` accepts the
  one-declaration-plus-return body.
- VF05/VF06 cover `Declaration` and `Identifier`.
- T09 lowers `Constant`, `LocalStore`, `LocalLoad`, `Return`.
- VF12 models the stored/loaded initializer.

### `/50` local add and modeled integer result

- `ResultValue` appends `Integer(IntegerConst)` at wire tag 5. Verification can
  return a computed scalar without writing a `ConstRecord`; canonical constant
  folding still uses `Record(RecordRef::Const)`.
- The TU parser adds the local binary shape.
- `SE_BIN` accepts an identifier operand with a committed `int` fact and skips
  constant folding when a non-literal operand is present.
- T09 emits the six-instruction local binary IR.
- VF12 checks the shape and returns the modeled integer.

### `/51` local binary operators

- The local binary shape accepts `+`, `-`, `*`, `/`, and `%`.
- `SEMANTIC_BINARY_EXPR`, `SE_RETURN`, VF05, and VF06 cover all five node kinds.
- Constant folding remains literal/literal `+` only.
- T09 maps the node kind to `IrOp::Add`/`Sub`/`Mul`/`Div`/`Rem`.
- VF12 evaluates with checked canonical-`int` arithmetic; zero divisors and
  out-of-range results fail closed.

### `/52` local expression trees

- The parser uses precedence climbing plus recursive unary/primary parsing.
  Parentheses are grouping only and create no AST node.
- AST nodes are committed in pre-order; any parse failure appends nothing.
- `SEMANTIC_UNARY_EXPR` (`SEMANTIC` local 23, `SE_UNARY_CHIP = ChipId(59)`,
  stage 4, one `sem.records` allowlist row) checks unary operands.
- `SE_BIN` accepts nested binary/unary operands with committed `int` facts.
- `SE_RETURN` accepts any covered expression root.
- VF05 covers `UnaryPlus`/`UnaryMinus`; VF06 adds them to the checked set.
- T09 recursively lowers the checked tree in post-order.
- VF12 interprets the covered instruction program with value/local maps,
  canonical-`int` checked arithmetic, and fail-closed uninitialized loads,
  zero divisors, and unsupported control flow.
- Literal constants are paired to literals in fold order, so repeated literal
  values are not ambiguous.

### `/53` assignments

- Zero or more `x = <expr>;` statements are accepted between the initializer
  and return.
- `SEMANTIC_ASSIGNMENT_EXPR` (`SEMANTIC` local 24,
  `SE_ASSIGN_CHIP = ChipId(60)`) appends a `NonLvalue` fact with
  `EFFECT_WRITES_LOCAL`.
- VF05/VF06 cover `Assignment`; the assignment-lhs identifier does not need an
  rvalue fact.
- T09 lowers the rhs and then a `LocalStore` in source order; VF12 models the
  store.

New hashed rules: `parse.tu-local-expression-tree`,
`se.unary-checked-nonlvalue`, `ir.local-expression-tree`,
`vf12.local-expression-program`, `parse.tu-local-assignment`,
`se.assignment-writes-local`, `ir.local-assignment-order`, and
`vf12.local-assignment-model`.

Artifact for this slice: `t01-c01-c06/53`, hash
`b3e6cd836db20dd3180e0f0a639ef2724e796203baed3ebbc743f8afdf66f72e`.
The repository contract has since advanced through `/55`; see
[DIRECT_CALL_SLICE.md](DIRECT_CALL_SLICE.md) for the call amendment.

## 3. AST shape

Fixed prefix:

```text
int main ( void ) { int x = <int> ; return <expr> ; } EOF
```

The prefix nodes remain:

| Node | Children | Range |
|---|---|---|
| `TranslationUnit` | FunctionDefinition | 0..EOF |
| `FunctionDefinition` | Specifiers, Declarator(main), Compound | 0..`}` |
| `Specifiers` | — | 0..0 |
| `Declarator(main)` | — | 1..4 |
| `Compound` | Declaration, Return | 5..`}` |
| `Declaration` | Specifiers, Declarator(x), IntLiteral | 6..10 |
| `Specifiers` | — | 6..6 |
| `Declarator(x)` | — | 7..7 |
| `IntLiteral` | — | 9..9 |
| `Return` | expression root | 11..`;` |

Expression nodes are allocated after `Return` in pre-order:

- `IntLiteral` and `Identifier` leaves have `first == last`;
- binary nodes span their first child through their last child;
- unary nodes span the operator through the operand;
- parentheses are not represented and may fall outside the inner range.

## 4. Implemented worker behavior

- **PA01**: fixed-prefix validation, precedence-climbing expression parser,
  matching declaration/use identifier names, literal resolution, and pre-order
  AST assignment. Failures append no nodes.
- **TySymbol**: block-scope `int` objects are automatic, no linkage.
- **SE identifier**: exactly one block scope, visible automatic `int` object.
- **SE unary**: one checked `int` operand, appends a `NonLvalue`/zero-effect
  fact.
- **SE binary**: literal/literal `+` still folds; any checked operand tree
  appends the binary fact without a fold child.
- **SE return / SE function**: accept the local body and expression root.
- **VF05 / VF06**: cover nested binary/unary nodes and unary leaves.
- **IR lowering**: validates every expression node's committed `int` fact,
  pairs literals to folded constants in fold order, and emits post-order
  `Constant`/`LocalLoad`/`LocalStore`/arithmetic/`Neg`/`Return` instructions.
- **VF12**: general symbolic interpreter over the covered IR ops.
- **Candidate driver**: checks the initializer literal, then walks the return
  expression in post-order, dispatching literal/identifier/binary/unary tasks.

## 5. Verification

macOS:

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test c56_source_locals
# 10 passed (Linux execution tests cfg-skipped)
cargo test --locked --manifest-path compiler/Cargo.toml
# 740 tests / 62 suites passed
```

Native AArch64 Linux:

```sh
docker run --rm --platform linux/arm64 -v "$PWD":/repo -w /repo \
  -e CARGO_TARGET_DIR=/tmp/target rust:1.85-bookworm \
  /usr/local/cargo/bin/cargo test --locked \
  --manifest-path compiler/Cargo.toml --test c56_source_locals
# 13 passed; native execution covers precedence, parentheses, literal-left
# forms, unary minus, the single-operator set, and assignment order
```

## 6. Explicit exclusions

- One local declaration, zero or more assignments, and one return statement only.
- The initializer is one integer literal; no initializer expression.
- Assignment targets are limited to the single declared local.
- No multiple declarations, assignment, address-taking, pointers, arrays,
  aggregates, or static locals.
- No `++`/`--`, casts, `sizeof`, logical/bitwise/comparison operators, or
  comma/conditional expressions.
- No calls, parameters, branches, loops, or memory objects.
- No `-S`/`-c`; the slice remains reachable through the bootstrap
  `--emit-asm` path until the full T11 chip family exists.
