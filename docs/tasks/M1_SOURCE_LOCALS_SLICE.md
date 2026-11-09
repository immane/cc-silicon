# M1 Source Locals Slice — `int main(void){int x = 2; return x;}`

| Field | Value |
|---|---|
| Status | **IMPLEMENTED / VERIFIED (2026-11-10)** as contract `/49` |
| Goal | Produce local load/store IR from source, emit AArch64, assemble/link/run, and return 2 |
| Evidence | `c56_source_locals` 5 macOS / 6 native AArch64 Linux; native run exits 2 |
| Non-claims | No source arithmetic, calls, memory objects, branches, general ABI, object emission, or torture rate |

## 1. Why this slice

The target emitter already lowered automatic `int` locals, `LocalLoad`/`LocalStore`,
and one `Return` at IR level. This slice supplies the missing frontend producer:

```c
int main(void) { int x = 2; return x; }
```

The committed IR is:

```text
Constant 2 -> v0
LocalStore x, v0
LocalLoad x -> v1
Return v1
```

The symbolic interpreter returns the initializer `ConstRecord`, and external
AArch64 assembly/link/run exits 2. Source-level arithmetic (`x + 3`) remains a
later slice because the current verification protocol has no approved carrier
for a computed non-constant result.

## 2. Contract `/49`

The frozen rule `parse.tu-fixed-nine-node-tree` is replaced by
`parse.tu-shape-gated-tree`: one `PARSE_TU` kind accepts exactly the canonical
constant-return shape or the locals shape. Other declared changes:

1. `SEMANTIC` local 22: `semantic.identifier_expr`, `SE_ID_CHIP = ChipId(58)`,
   stage 4, one `sem.records` allowlist row.
2. `TySymbolChip` may declare an automatic `int` object in a committed block
   scope (`sym.declare-block-object`).
3. `SE_RETURN` accepts an identifier operand; `SE_FUNCTION` accepts the
   one-declaration-plus-return body.
4. VF05 covers `Declaration` and `Identifier`; VF06 adds `Identifier` to its
   checked set.
5. T09 lowers `Constant`, `LocalStore`, `LocalLoad`, `Return`.
6. VF12 models the stored/loaded initializer.

New hashed rules: `parse.tu-shape-gated-tree`,
`symbol.declare-block-object`, `se.identifier-checked-nonlvalue`,
`ir.local-load-store-return`, `vf12.local-load-store-return`.

Artifact: `t01-c01-c06/49`, hash
`ecf40bd3d0ca1dc9f48140ea461e9716b222a8b0d41aa623a075144bea062bd9`.

No IR, type, layout, constant, or diagnostic record shape changed.

## 3. AST shape

Token order:

```text
0 int   1 main  2 (   3 void  4 )   5 {
6 int   7 x     8 =   9 2     10 ;
11 return 12 x  13 ;   14 }    15 EOF
```

Pre-order nodes and ranges:

| Node | Children | Range |
|---|---|---|
| `TranslationUnit` | FunctionDefinition | 0..15 (EOF) |
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

The canonical 9-node tree and ranges are unchanged.

## 4. Implemented worker behavior

- **PA01**: dispatches on token-sequence length; validates every keyword,
  identifier, punctuator, EOF shape, matching declaration/use names, and the
  committed literal before appending. Failures append no nodes.
- **TySymbol**: function declarations remain file-scope/static/external; `int`
  declarations are block-scope objects with no linkage and automatic storage.
- **SE identifier**: requires exactly one block scope and a visible automatic
  `int` object; appends one `NonLvalue`, zero-effect fact.
- **SE return / SE function**: accept the identifier return and the
  two-statement body; preserve the canonical path.
- **VF05 / VF06**: cover the new node shapes and the identifier fact.
- **IR lowering**: resolves the local symbol and the initializer's folded
  constant by value, then emits the four-instruction local shape. The match
  must be unique.
- **VF12**: accepts exactly `Constant`, `LocalStore`, `LocalLoad`, `Return`
  with the same local and a returned loaded value; rejects uninitialized or
  unsupported shapes.
- **Candidate driver**: enters the block scope, declares the local, folds the
  initializer, runs the identifier fact, and keeps the canonical path
  unchanged.

## 5. Verification

macOS:

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test c56_source_locals
# 5 passed (Linux execution test cfg-skipped)
cargo test --locked --manifest-path compiler/Cargo.toml
# 735 tests / 62 suites passed
```

Native AArch64 Linux:

```sh
docker run --rm --platform linux/arm64 -v "$PWD":/repo -w /repo \
  -e CARGO_TARGET_DIR=/tmp/target rust:1.85-bookworm \
  /usr/local/cargo/bin/cargo test --locked \
  --manifest-path compiler/Cargo.toml --test c56_source_locals
# 6 passed; emitted assembly assembles, links, and exits 2
```

The integration test also checks the canonical `return 2+3` evidence and
assembly remain unchanged, rejects mismatched names/extra statements, checks
the identifier chip's write scope, and replays the same source deterministically.

## 6. Explicit exclusions

- No source arithmetic, assignment statements, compound assignment, multiple
  declarations, initializers other than one integer literal, or expression
  statements.
- No local address-taking, pointers, arrays, aggregates, or static locals.
- No calls, parameters, branches, loops, or memory objects.
- No `-S`/`-c`; the slice remains reachable through the bootstrap
  `--emit-asm` path until the full T11 chip family exists.
