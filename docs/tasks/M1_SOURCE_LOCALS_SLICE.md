# M1 Source Locals Slice — Local Return and Local Add

| Field | Value |
|---|---|
| Status | **IMPLEMENTED / VERIFIED (2026-11-10)** as contract `/50` (`/49` local return, `/50` local add) |
| Goal | Produce local load/store and local-add IR from source, emit AArch64, execute on native Linux |
| Evidence | `c56_source_locals` 6 macOS / 8 native AArch64 Linux; runs exit 2 and 5 |
| Non-claims | No calls, memory objects, branches, general ABI, object emission, or torture rate |

## 1. Supported source shapes

```c
int main(void) { int x = 2; return x; }
int main(void) { int x = 2; return x + 3; }
```

Committed IR:

```text
Constant 2 -> v0
LocalStore x, v0
LocalLoad x -> v1
Return v1
```

```text
Constant 2 -> v0
LocalStore x, v0
LocalLoad x -> v1
Constant 3 -> v2
Add v1, v2 -> v3
Return v3
```

The symbolic interpreter returns 2 and 5 respectively. The emitted AArch64
assembles, links, and exits 2 and 5 on native Linux. Other operators
(`-`, `*`, `/`, `%`, unary `-`) remain a later slice even though the target
emitter already handles them at IR level.

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
- The TU parser adds the 18-token local-add shape.
- `SE_BIN` accepts an identifier operand with a committed `int` fact and skips
  constant folding when a non-literal operand is present.
- T09 emits the six-instruction local-add IR.
- VF12 checks the exact shape and returns the modeled integer 5.

New hashed rules: `se.binary-identifier-operand`, `ir.local-add-return`,
`vf12.local-add-modeled-integer`, `result.integer-modeled`.

Artifact: `t01-c01-c06/50`, hash
`ff7177a944b5b5b7a65f4463e199ce5a90a8729700994162d4a82cafe310006c`.

## 3. AST shapes

Local return token order:

```text
0 int   1 main  2 (   3 void  4 )   5 {
6 int   7 x     8 =   9 2     10 ;
11 return 12 x  13 ;   14 }    15 EOF
```

Local add token order:

```text
0 int   1 main  2 (   3 void  4 )   5 {
6 int   7 x     8 =   9 2     10 ;
11 return 12 x  13 +   14 3    15 ;
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

Local add adds `BinaryAdd` (12..14) with `Identifier(x)` (12..12) and
`IntLiteral(3)` (14..14); `Return` spans 11..15 and the function/TU ranges
extend to 16/17. The canonical 9-node tree is unchanged.

## 4. Implemented worker behavior

- **PA01**: dispatches on the closed token length; validates keyword,
  identifier, punctuator, EOF, matching declaration/use names, and every
  committed literal before appending. Failures append no nodes.
- **TySymbol**: block-scope `int` objects are automatic, no linkage.
- **SE identifier**: exactly one block scope, visible automatic `int` object.
- **SE binary**: literal/literal still folds; identifier/literal appends the
  `BinaryAdd` fact without a fold child.
- **SE return / SE function**: accept both local bodies; preserve canonical.
- **VF05 / VF06**: cover the new node shapes and identifier fact.
- **IR lowering**: resolves the local and each literal's folded constant by
  value, requires unique matches, and emits the exact local IR.
- **VF12**: models both local shapes; local return yields its initializer,
  local add yields the computed canonical `int`.
- **Candidate driver**: enters block scope, declares the local, folds every
  local-shape literal, checks identifier facts before binary facts, and handles
  both `Const` and `Integer` verification results.

## 5. Verification

macOS:

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test c56_source_locals
# 6 passed (Linux execution tests cfg-skipped)
cargo test --locked --manifest-path compiler/Cargo.toml
# 736 tests / 62 suites passed
```

Native AArch64 Linux:

```sh
docker run --rm --platform linux/arm64 -v "$PWD":/repo -w /repo \
  -e CARGO_TARGET_DIR=/tmp/target rust:1.85-bookworm \
  /usr/local/cargo/bin/cargo test --locked \
  --manifest-path compiler/Cargo.toml --test c56_source_locals
# 8 passed; local return exits 2 and local add exits 5 after assemble/link/run
```

The integration test also checks the canonical `return 2+3` evidence and
assembly, rejects mismatched names/unsupported operators/extra statements,
checks the identifier chip's write scope, and replays the same source
deterministically.

## 6. Explicit exclusions

- No source `-`, `*`, `/`, `%`, unary `-`, assignment, multiple declarations,
  or initializers other than one integer literal.
- The initializer and local-add operand are matched to folded constants by
  value, so two identical literal values fail closed as ambiguous instead of
  guessing which fold belongs to which use.
- No local address-taking, pointers, arrays, aggregates, or static locals.
- No calls, parameters, branches, loops, or memory objects.
- No `-S`/`-c`; the slice remains reachable through the bootstrap
  `--emit-asm` path until the full T11 chip family exists.
