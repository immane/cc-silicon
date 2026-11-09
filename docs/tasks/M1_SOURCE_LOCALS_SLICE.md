# M1 Source Locals Slice — Return, Local Add, and Binary Operators

| Field | Value |
|---|---|
| Status | **IMPLEMENTED / VERIFIED (2026-11-10)** as contract `/51` (`/49` local return, `/50` local add + modeled integer, `/51` local binary operators) |
| Goal | Produce local load/store and local binary IR from source, emit AArch64, execute on native Linux |
| Evidence | `c56_source_locals` 7 macOS / 10 native AArch64 Linux; runs exit 2, 5, 255, 6, 0, and 2 |
| Non-claims | No calls, memory objects, branches, general ABI, object emission, or torture rate |

## 1. Supported source shapes

```c
int main(void) { int x = 2; return x; }
int main(void) { int x = 2; return x + 3; }
int main(void) { int x = 2; return x - 3; }
int main(void) { int x = 2; return x * 3; }
int main(void) { int x = 2; return x / 3; }
int main(void) { int x = 2; return x % 3; }
```

The binary forms commit the matching `BinaryAdd`, `BinarySub`, `BinaryMul`,
`BinaryDiv`, or `BinaryRem` node. The IR is the same closed shape with the
operator substituted:

```text
Constant 2 -> v0
LocalStore x, v0
LocalLoad x -> v1
Constant 3 -> v2
<Add|Sub|Mul|Div|Rem> v1, v2 -> v3
Return v3
```

The symbolic interpreter returns `2`, `5`, `-1`, `6`, `0`, and `2`
respectively. Native AArch64 execution exits 2, 5, 255, 6, 0, and 2.

Other expression forms — chained operators, parentheses, literal-left binary
expressions, unary `-`, and assignment — remain a later slice.

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
- The TU parser adds the 18-token local-binary shape.
- `SE_BIN` accepts an identifier operand with a committed `int` fact and skips
  constant folding when a non-literal operand is present.
- T09 emits the six-instruction local binary IR.
- VF12 checks the shape and returns the modeled integer.

### `/51` local binary operators

- The same local-binary token shape accepts `+`, `-`, `*`, `/`, and `%`.
- `SEMANTIC_BINARY_EXPR`, `SE_RETURN`, VF05, and VF06 cover all five node kinds.
- Constant folding remains literal/literal `+` only.
- T09 maps the node kind to `IrOp::Add`/`Sub`/`Mul`/`Div`/`Rem`.
- VF12 evaluates with checked canonical-`int` arithmetic; division/remainder by
  zero and out-of-range results fail closed.

New hashed rules: `parse.tu-local-binary-operators`,
`se.local-binary-operators`, `ir.local-binary-operators`, and
`vf12.local-binary-operators`.

Artifact: `t01-c01-c06/51`, hash
`4f110d098a54d56b8c39d1c90d902141e0e9e4f6aab1192203763017fe57a6ce`.

## 3. AST shape

Local return token order:

```text
0 int   1 main  2 (   3 void  4 )   5 {
6 int   7 x     8 =   9 2     10 ;
11 return 12 x  13 ;   14 }    15 EOF
```

Local binary token order:

```text
0 int   1 main  2 (   3 void  4 )   5 {
6 int   7 x     8 =   9 2     10 ;
11 return 12 x  13 <op> 14 3  15 ;
16 }     17 EOF
```

Local return pre-order nodes:

| Node | Children | Range |
|---|---|---|
| `TranslationUnit` | FunctionDefinition | 0..15 |
| `FunctionDefinition` | Specifiers, Declarator(main), Compound | 0..14 |
| `Specifiers` | — | 0..0 |
| `Declarator(main)` | — | 1..4 |
| `Compound` | Declaration, Return | 5..14 |
| `Declaration` | Specifiers, Declarator(x), IntLiteral(2) | 6..10 |
| `Specifiers` | — | 6..6 |
| `Declarator(x)` | — | 7..7 |
| `IntLiteral(2)` | — | 9..9 |
| `Return` | Identifier(x) | 11..13 |
| `Identifier(x)` | — | 12..12 |

Local binary adds the selected binary node (12..14) with `Identifier(x)`
(12..12) and `IntLiteral(3)` (14..14); `Return` spans 11..15 and the
function/TU ranges extend to 16/17. The canonical 9-node tree is unchanged.

## 4. Implemented worker behavior

- **PA01**: dispatches on the closed token length; validates keyword,
  identifier, punctuator, operator spelling, EOF, matching declaration/use
  names, and every committed literal before appending. Failures append no
  nodes.
- **TySymbol**: block-scope `int` objects are automatic, no linkage.
- **SE identifier**: exactly one block scope, visible automatic `int` object.
- **SE binary**: literal/literal `+` still folds; identifier/literal forms for
  all five operators append the checked fact without a fold child.
- **SE return / SE function**: accept the local bodies; preserve canonical.
- **VF05 / VF06**: cover the new node shapes and identifier fact.
- **IR lowering**: resolves the local and each literal's folded constant by
  value, requires unique matches, and emits the exact local IR with the matching
  `IrOp`.
- **VF12**: models local return and all five local binary operators; arithmetic
  is checked canonical `int`; zero divisors and out-of-range results fail.
- **Candidate driver**: enters block scope, declares the local, folds every
  local-shape literal, checks identifier facts before binary facts, and handles
  both `Const` and `Integer` verification results.

## 5. Verification

macOS:

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test c56_source_locals
# 7 passed (Linux execution tests cfg-skipped)
cargo test --locked --manifest-path compiler/Cargo.toml
# 737 tests / 62 suites passed
```

Native AArch64 Linux:

```sh
docker run --rm --platform linux/arm64 -v "$PWD":/repo -w /repo \
  -e CARGO_TARGET_DIR=/tmp/target rust:1.85-bookworm \
  /usr/local/cargo/bin/cargo test --locked \
  --manifest-path compiler/Cargo.toml --test c56_source_locals
# 10 passed; local return exits 2, local add exits 5, and the four
# single-operator cases exit 255, 6, 0, and 2 after assemble/link/run
```

The integration test also checks the canonical `return 2+3` evidence and
assembly, rejects mismatched names/unsupported operators/extra statements,
checks the identifier chip's write scope, and replays the same source
deterministically.

## 6. Explicit exclusions

- No chained operators, parentheses, literal-left binary expressions, unary
  `-`, assignment, multiple declarations, or initializers other than one
  integer literal.
- The initializer and binary operand are matched to folded constants by value,
  so two identical literal values fail closed as ambiguous instead of guessing
  which fold belongs to which use.
- No local address-taking, pointers, arrays, aggregates, or static locals.
- No calls, parameters, branches, loops, or memory objects.
- No `-S`/`-c`; the slice remains reachable through the bootstrap
  `--emit-asm` path until the full T11 chip family exists.
