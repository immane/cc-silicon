# M1 Target Emission Prototype — AArch64 Constant Return

| Field | Value |
|---|---|
| Status | **IMPLEMENTED / VERIFIED (2026-11-09)** — host-only T11 bootstrap; no full target chip family |
| Input | Committed M1 IR: one function, one block, `Constant` + `Return` |
| Output | GNU AArch64 assembly for `int main(void){return <constant>;}` |
| Command | `candidate --emit-asm <path> <source.c>` |
| Non-claims | No general codegen, ABI calls, locals, memory, control flow, object emission, or torture rate |

## 1. Scope

This slice unlocks the first source-to-executable path for the canonical M1
fixture without pretending T11 is complete. The candidate consumes the already
committed IR and writes GNU assembly; an external assembler/linker produces the
ELF executable.

Supported shape:

```text
FunctionRecord int(void)
  BlockRecord entry
    InstructionRecord Constant { immediate: ConstRecord<Integer>, result: Value }
    InstructionRecord Return   { operands: [Value] }
```

The constant must fit `i32`; every other IR shape fails closed.

## 2. Emission

For value `5`:

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

`movz`/`movk` covers any 32-bit value. AArch64 `main` returns its `int` result in
`w0`; no prologue/epilogue is needed for a leaf function with no locals.

## 3. Verification

macOS host (assembly emission test only):

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test c55_target_m1
# 1 passed (Linux execution test cfg-skipped)
```

Native Linux AArch64 (assembly + assemble/link/run):

```sh
docker run --rm --platform linux/arm64 -v "$PWD":/repo -w /repo \
  -e CARGO_TARGET_DIR=/tmp/target rust:1.85-bookworm \
  /usr/local/cargo/bin/cargo test --locked \
  --manifest-path compiler/Cargo.toml --test c55_target_m1
# 2 passed; compiled executable exits 5 for int main(void){return 2+3;}
```

The Linux test skips when the host GCC is not an AArch64 target.

## 4. Explicit limitations

- Host-only prototype, not a T11 chip or manifest entry.
- No target records, register allocation, stack frames, calls, locals, memory,
  branches, ABI argument classification, relocations, or object emission.
- `-S`/`-c` remain refused; `--emit-asm` is an explicit bootstrap flag.
- External assembler/linker/sysroot/runtime are host inputs, not frozen here.
- No GCC torture corpus or pass rate is involved.
