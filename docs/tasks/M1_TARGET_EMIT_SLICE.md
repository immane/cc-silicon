# M1 Target Emission Prototype — AArch64 Constant Return and Integer IR

| Field | Value |
|---|---|
| Status | **IMPLEMENTED / VERIFIED (2026-11-10)** — host-only T11 bootstrap; no full target chip family |
| Input | Committed single-block `int(void)` IR: `Constant` + `Return`, or automatic-local load/store plus integer arithmetic and one `Return` |
| Output | GNU AArch64 assembly for `int main(void)` |
| Command | `candidate --emit-asm <path> <source.c>` (constant-return, local-return, and local-add source paths) |
| Non-claims | No general source arithmetic beyond local add, general codegen, calls, memory, control flow, object emission, or torture rate |

## 1. Scope

This slice unlocks the first source-to-executable path for the canonical M1
fixture without pretending T11 is complete. The candidate consumes committed IR
and writes GNU assembly; an external assembler/linker produces the ELF
executable.

Two IR shapes are accepted:

```text
FunctionRecord int(void)
  BlockRecord entry
    InstructionRecord Constant { immediate: ConstRecord<Integer>, result: Value }
    InstructionRecord Return   { operands: [Value] }
```

and the stack-machine shape:

```text
FunctionRecord int(void)
  BlockRecord entry
    Constant / LocalLoad / LocalStore / Add / Sub / Mul / Div / Rem / Neg
    Return { operands: [Value] }
```

Locals must be automatic objects with the canonical signed-`int` type. Every
produced value must carry that same type. Constants must fit `i32`; operands must be produced before use; `Return` must
be last. Stack frames are 16-byte aligned and must fit the AArch64 `sub sp,
sp, #imm` range (at most 4080 bytes after alignment). Any other IR shape
fails closed.

The host candidate frontend produces the local load/store shape for
`int main(void){int x=2;return x;}` and the local-add shape for
`int main(void){int x=2;return x+3;}`. Other source operators remain
IR-fixture-only.

## 2. Emission

For value `5`, the constant-return path emits a leaf function with no frame:

```asm
# cc-silicon M1 AArch64 prototype
.text
.align 2
.globl main
.type main, %function
main:
    movz w0, #5
    ret
.size main, .-main
.section .note.GNU-stack,"",%progbits
```

The stack-machine path reserves 16-byte-aligned slots for locals and values,
then lowers each instruction to `ldr`/`str` and integer ALU instructions. For
example, `int x = 2; return x + 3;` becomes:

```asm
main:
    sub sp, sp, #32
    movz w0, #2
    str w0, [sp, #4]
    ldr w0, [sp, #4]
    str w0, [sp, #0]
    ldr w0, [sp, #0]
    str w0, [sp, #8]
    movz w0, #3
    str w0, [sp, #12]
    ldr w0, [sp, #8]
    ldr w1, [sp, #12]
    add w0, w0, w1
    str w0, [sp, #16]
    ldr w0, [sp, #16]
    add sp, sp, #32
    ret
```

`movz`/`movk` covers any 32-bit constant. Division and remainder use `sdiv`
and a following `msub`; negation uses `neg`. AArch64 `main` returns its `int`
result in `w0`.

## 3. Verification

macOS host (assembly emission plus IR-shape unit tests; Linux execution tests
are cfg-skipped):

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --bin candidate
# 14 passed
cargo test --locked --manifest-path compiler/Cargo.toml --test c55_target_m1
# 1 passed
cargo test --locked --manifest-path compiler/Cargo.toml --test c56_source_locals
# 6 passed
```

Native Linux AArch64 (same unit tests plus assemble/link/run):

```sh
docker run --rm --platform linux/arm64 -v "$PWD":/repo -w /repo \
  -e CARGO_TARGET_DIR=/tmp/target rust:1.85-bookworm \
  /usr/local/cargo/bin/cargo test --locked \
  --manifest-path compiler/Cargo.toml --bin candidate
# 14 passed; the local/add fixture exits 5, the all-operator fixture exits
# 252 (-4), and the signed-constant fixture exits 255 (-1)
# c56_source_locals: 8 passed; local return exits 2 and local add exits 5
```

The `c55_target_m1` integration test separately assembles, links, and runs the
constant-return CLI output with native AArch64 GCC (exit 5). Execution tests
skip when the host GCC is not an AArch64 target.

## 4. Explicit limitations

- Host-only prototype, not a T11 chip or manifest entry.
- No target records, register allocation, calls, memory operands, branches,
  ABI argument classification, relocations, or object emission.
- Stack frames larger than the `sub sp, sp, #imm` range fail closed; no
  large-frame materialization yet.
- Source arithmetic is limited to local add; other operators remain
  IR-fixture-only.
- `-S`/`-c` remain refused; `--emit-asm` is an explicit bootstrap flag.
- External assembler/linker/sysroot/runtime are host inputs, not frozen here.
- No GCC torture corpus or pass rate is involved.
