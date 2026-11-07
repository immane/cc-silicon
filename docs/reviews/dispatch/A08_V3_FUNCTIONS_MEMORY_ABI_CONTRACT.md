# A-PTS-03 — V3 Functions, Memory, and ABI Contract Requirements

| Field | Value |
|---|---|
| Status | **PROPOSAL / NEEDS_FREEZE.** Design and contract-requirement report only; not an accepted contract, not a `/next` freeze, not implementation authorization, and not evidence of any implemented V3 capability. |
| Date | 2026-11-07 |
| Task | Stage A `A-PTS-03` under [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) §5 and §6.2 V3 |
| Baseline | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` (`compiler/contracts/CONTRACT_VERSION:3-4`; `compiler/src/contract.rs:38-44`) |
| Target | Identity `aarch64-unknown-linux-gnu`, ELF, LP64, little-endian, AAPCS64 is frozen; `target_verification=unverified`; **every concrete target/ABI value remains UNVERIFIED** |
| Repository state | Branch `wave4-development`, HEAD `430ae29`; the worktree already contained uncommitted user parse/lex drafts and freeze drafts, and additional concurrent changes appeared during this read-only task (including `candidate.rs`, `parse/mod.rs`, and chip-lint tests). They were read only where relevant and were not modified. |
| Evidence class | **Static read-only inspection only.** No probe, compiler build, test suite, assembler, linker, runner, target execution, or GCC torture run was performed for this report. No source, test, contract, Cargo file, `mod.rs`, shared index, or ledger was edited. The only file written is this report. |
| Non-claim | No V3 behavior, ABI correctness, target value, pass rate, or execution result is claimed. No numeric ABI value, register number, width, alignment, offset, wire tag, family ordinal, task local code, diagnostic code, or contract hash is assigned here. |

Evidence labels used throughout:

- **[FACT /39]** — a static fact of the current frozen artifact or checked-in source, not a test result.
- **[PROPOSAL]** — a design recommendation in this report requiring T01 integrator and owner co-freeze.
- **[UNVERIFIED]** — a concrete target/ABI value or probe-derived fact that must not drive code generation.
- **[NOT RUN]** — a check, probe, or test that this report did not execute.
- **[BLOCKED_PROBE]** — work that cannot be implemented or accepted until H01 → H07 → T01 attestation completes.

---

## 1. Executive Summary

V3 adds the first real C memory model and the first cross-function boundary. The current `/39` contract has only the M1 symbolic shapes: `int`/`void` types, the `int(void)` function type, one function, one block, `Constant` + `Return`, and no memory, pointer, array, global, argument, or call records. V3 therefore needs a small, append-only shared extension, not a second private type system.

The recommended smallest set is:

1. **Append-only type/AST/symbol/scope/semantic extensions:** `Pointer` and `Array` type kinds, V3 parser nodes, a function scope, symbol definition/identifier facts, an identifier binding on `SemRecord`, and a closed V3 effect vocabulary.
2. **Three new shared record families:** `ConversionRecord` (T07-owned), `MemoryObjectRecord` (T09-owned), and `AbiPlanRecord` (T11-owned, **probe-gated**). Existing reserved `Layout` and `Init` records are typed for V3 but reuse their existing IDs and `RecordRef` variants.
3. **Append-only IR extension:** `FunctionRecord.parameters`; `InstructionRecord` object/conversion/parameter/callee fields; V3 IR ops `Convert`, `ObjectAddress`, `Load`, `Store`, `GetElementPtr`, `Param`, and `Call`, reusing `Constant`, `Add`, and `Return`.
4. **A strict Part A / Part B split:** Part A (target-independent) is the canonical V3 fixture through semantics, memory objects, and typed IR. Part B (target-gated) is layout, ABI locations, frames, assembly, link, and execution. Part A never reads the host ABI; Part B never fabricates a value from AAPCS64 prose or macOS.
5. **Single-writer ownership:** T05 owns AST nodes; T06 owns types, symbols, and scopes; T07 owns `SemRecord` and `ConversionRecord`; T08 owns layout and static initialization; T09 owns memory objects, IR functions, and IR instructions; T11 owns ABI plans and machine records; T13 owns verification facts. T06's catalogued TY23/TY24/TY27/TY28 services must not become a second writer of conversion facts; this is a freeze decision, not a silent merge.
6. **No new `ResultValue` variant:** every V3 task result uses the frozen `Record`/`Records`/`Ack`/`Fail` carriers, with records materialized before completion. This preserves the `/39` result protocol and exactly-once consumption.

The report identifies 26 conflicts with `/39` or current drafts, including the most material ones:

- `/39` `TypeKind` is M1-closed and has no pointer/array; `NodeKind` has only eight kinds; `ScopeKind` has only file/block.
- `SymbolRecord` has no definition state and its `decl` is the M1 declarator node, while the T06 candidate draft says `decl` is the `Identifier` leaf.
- `SemRecord` has no binding or conversion facts; `EffectMask` is M1 zero-only.
- `ConstRecord` has only `{value, negative}` while T08/T09 drafts and A05's audit require a typed constant; this report recommends not adding `ty` to `ConstRecord` but carrying type on `InitRecord`/`ValueRecord` (open decision).
- The probe/`AbiSpec` covers only four ABI fields; V3 argument/return/stack/callee-save/relocation evidence is missing and is **BLOCKED_PROBE**.
- A09 proposes a `FunctionId`-keyed `AbiPlanRecord`; V3 also needs the same signature for external prototypes, so the recommended reconciliation is `signature`-keyed plans plus an optional `FunctionId` link.

No ABI value is assigned anywhere in this report. Where an example needs a number, it appears only as a symbolic expression over `TargetSpec`/`AbiSpec` or as an explicit placeholder, never as a frozen value.

---

## 2. Goal, Scope, and the V3 Vertical Slice

### 2.1 Goal

Produce the **contract-requirement package** that lets a downstream integrator freeze the smallest shared records, task/result shapes, legality rules, ownership, and verification facts needed for a real V3 vertical slice:

- function parameter lists, declarations, definitions, calls, and returns;
- global variables, static locals, automatic locals, and parameters;
- pointers, address-of, dereference, arrays, subscripting, and pointer/array decay;
- lvalue conversion;
- argument and return conversions;
- recursive C functions and bounded compiler task depth;
- target-gated ABI handoff without substituting the Rust host ABI.

### 2.2 In scope

1. Shared record field requirements and append-only enum additions.
2. Task kind / payload / result shapes for V3 producers and consumers.
3. Exact producer/consumer ownership and forbidden duplicate writers.
4. Legality, diagnostic, unsupported, and ABI-gate rules.
5. Determinism, snapshot/hash, and replay impact.
6. Ordered freeze work and implementation work orders with exact owned paths.
7. Explicit separation of `/39` facts, proposals, and unverified ABI expectations.

### 2.3 Out of scope

- Any implementation, chip code, test code, contract edit, or freeze in this task.
- Running the compiler, tests, probe, assembler, linker, runner, or torture suite.
- Filling in any concrete ABI value, register number, stack alignment, width, offset, relocation form, or probe result.
- Full C: qualifiers (`const`/`volatile`/`restrict`/`_Atomic`), floating point, `long double`, `__int128`, structs/unions/enums, bitfields, variadic functions, old-style/K&R definitions, VLAs/alloca, function pointers/indirect calls, compound literals, designated/aggregate initializers, inline assembly, GNU extensions, optimization, multi-TU linking, PIE/PIC/TLS policy, and the official torture gate.
- The separate target environment plan (A-ABI-01/A09), integer/floating model plan (A-ABI-02), and independent V3 tests (A-TST-03). This report consumes their constraints where relevant and does not replace them.

### 2.4 Canonical V3 Part A fixture (design only)

The following source is **not implemented and not run**. It is the smallest fixture that exercises every V3 shared record without requiring floating point, aggregates, variadics, or control flow:

```c
int g = 5;

int add(int a, int b);
int add(int a, int b) { return a + b; }

int main(void) {
    int x = 2;
    int *p = &x;
    int a[2];

    a[0] = 1;
    a[1] = 3;

    return add(g, *p) + a[0];
}
```

The expected Part B exit status, if this fixture is accepted later, is `8` (`g=5`, `*p=x=2`, `a[0]=1`). This expected value is a hand calculation, not a test result. A-TST-03 may choose a different independent fixture; this source only anchors the shared record requirements. The fixture exercises:

| V3 feature | Exercised by |
|---|---|
| Global variable + static init | `int g = 5;` |
| Function prototype + definition compatibility | `int add(int, int);` and the definition |
| Parameters, parameter symbols, function scope | `add(int a, int b)` |
| Direct call, argument order, return conversion | `add(g, *p)` |
| Automatic local, address-of, pointer object | `int x`; `int *p = &x` |
| Dereference and lvalue conversion | `*p` in the call argument |
| Array object, subscript store/load, array decay | `int a[2]; a[0] = 1; a[1] = 3; a[0]` |
| Global object address | `g` in the call argument |
| Pointer/array type construction | `int *`, `int[2]` |
| IR `Param`/`Call`/`Load`/`Store`/`GetElementPtr`/`ObjectAddress`/`Return` | The lowered function/body |

### 2.5 Recursion and task-depth fixtures (design only)

Recursion is part of V3 scope but requires V2 control flow. A separate boundary fixture is:

```c
int sum(int n) {
    if (n == 0) return 0;
    return n + sum(n - 1);
}

int main(void) {
    return sum(3);
}
```

Expected value `6` by hand calculation. This fixture is **V3-REC** and depends on V1/V2 operators and CFG; it is not part of the canonical V3-A fixture. It proves that C recursion is a runtime call-graph cycle, not compiler recursion into function bodies.

The compiler task-depth fixture is independent of C recursion: `int (((((x)))));` and nested pointer/array declarators must consume explicit parser/type frames bounded by `limits.max_task_depth`, not the Rust call stack. V3 must not introduce a new task-depth limit without T01 approval.

### 2.6 Acceptance levels

| Level | Meaning for V3 | Current state |
|---|---|---|
| Contract shape | Records, task kinds, results, ownership, and rules are frozen in a `/next` artifact | **Not frozen; this is the proposal** |
| Independent implementation | Each chip passes synthetic normal/boundary/invalid/unsupported/replay/write-scope tests | **Not implemented / not run** |
| Integration | Producers consume real upstream records; registration/routing/allowlist/snapshot pass | **Not implemented / not run** |
| Part A closed loop | Canonical fixture reaches typed memory objects, IR, and VF06/VF08/VF01/VF05 with deterministic snapshot/trace | **Not implemented / not run** |
| Part B closed loop | Probe-attested target, ABI locations, assembly, assembler/linker, exit `8`, recursion exit `6` | **BLOCKED_PROBE / not run** |

---

## 3. Baseline Facts and Conflicts

### 3.1 `/39` facts relevant to V3

| Fact | Evidence | Consequence for V3 |
|---|---|---|
| `TypeKind` is `Void`, `Bool`, `Char`, `Int`, `Function` only; no pointer/array | `compiler/src/bus.rs:295-323` | V3 needs append-only `Pointer`/`Array`; changes the type-name inventory and hash |
| `SymbolRecord` is `{name, scope, kind, ty, linkage, storage, decl}`; `decl` is documented as the M1 `Declarator` node | `compiler/src/bus.rs:360-377` | V3 needs definition state and an identifier-leaf fact for lookups |
| `ScopeKind` is `File`/`Block` only | `compiler/src/bus.rs:379-386` | V3 parameter scope needs a `Function` scope |
| `NodeKind` is the eight M1 kinds `TranslationUnit`, `FunctionDefinition`, `Specifiers`, `Declarator`, `Compound`, `Return`, `BinaryAdd`, `IntLiteral` | `compiler/src/bus.rs:543-562` | V3 needs identifiers, parameter lists, pointer/array declarators, calls, subscript, address-of, dereference |
| `SemRecord` is `{node, ty, category, effects}`; no binding, no conversions | `compiler/src/bus.rs:496-513` | V3 needs identifier binding and explicit conversion facts |
| `EffectMask` is `u32`; M1 allows only `0` | `compiler/src/bus.rs:435-438`; T07 rev 46 | V3 memory/call effects need a closed V3 bit vocabulary, with the M1 zero-only rule kept M1-scoped |
| `IrOp` is `Constant`/`Return` only | `compiler/src/bus.rs:440-447` | V3 needs conversion/address/load/store/GEP/parameter/call ops |
| `FunctionRecord` is `{symbol, signature, entry, linkage}` | `compiler/src/bus.rs:449-460` | V3 definitions need parameter symbol links |
| `InstructionRecord` is `{op, block, operands, immediate, result}` | `compiler/src/bus.rs:480-494` | V3 instructions need object/conversion/parameter/callee references |
| `ConstRecord` is `{value, negative}`; no `ty` | `compiler/src/bus.rs:590-602` | A05 G4: the documented `ir.op-immediate-type` rule is not implementable as written; V3 must not silently add a different typed constant |
| `ResultValue` has only `Empty`, `Ack`, `Record`, `Records`, `Diagnostic` | `compiler/src/task.rs:1718-1739` | V3 should not add a scalar/tuple result; use records or `Ack`/`Fail` |
| `Payload` is `RecordRef`-only; proposals include `Enqueue`, `Complete`, `Fail`, `AwaitHost`, `StorePatch`, `AppendRecords`, `Progress`, `AwaitChildren` | `compiler/src/task.rs:1885-1960` | V3 must not put scalar flags in payloads; encode them in AST/type/record facts |
| Current stage assignment has rows only at stages 0–6; no target rows | `compiler/src/manifest.rs:1444-1514`; `STAGE_COUNT = 11` in `compiler/src/limits.rs:21` | V3 can use stages 7/8 without renumbering existing stages |
| Schema declares `sem.records`, `types.records`, `symbols.*`, `ir.functions/blocks/values/instructions`; no `sem.conversions`, `ir.memory_objects`, `layout.records`, `init.records`, or `machine.abi_plans` | `compiler/src/manifest.rs:149-350` | V3 needs these fields in the frozen seed |
| `Layout`/`Init` IDs and `RecordRef` variants exist, but their arenas are `ReservedArena` and `G1DraftBody` has no `Layout`/`Init` variants | `compiler/src/bus.rs:645-648`; `compiler/src/records.rs` | V3 must type them and add commit/snapshot materialization |
| `machine` is a `StoreId` but has no typed arena or declared field | `compiler/src/task.rs:2050-2075`; `compiler/src/bus.rs:646-671` | V3 `AbiPlan` needs a typed arena and `machine.abi_plans` |
| Target `AbiSpec` has only `gp_arg_regs`, `fp_arg_regs`, `stack_align`, `variadic_register_save_area`, all `UNVERIFIED` | `compiler/src/target.rs:343-359`; `compiler/src/target.rs:577-620`; `compiler/contracts/target/aarch64-linux-probe.txt`; `docs/tasks/T11_TARGET_CHIPS.md:7` | V3 return/stack/callee-save/relocation evidence is absent; Part B is blocked |
| `TargetSpec::require_verified` is the codegen gate; `attest` validates caller-supplied data/hash, not provenance | `compiler/src/target.rs:647-716`; T01 C02 row | V3 ABI plans must use the gate and VF11 must cross-check plausibility |
| No T11 target chip, task kind, stage, or manifest exists; `candidate -S/-c` fail closed | `compiler/src/chips/` has no target group; A05 §2.1; `compiler/src/bin/candidate.rs` (concurrently modified user file) | V3 Part B is blocked, not a shell-pass |
| `SE29` exists but is not wired into the candidate; VF06 checks only the three M1 expression/statement kinds | `compiler/src/chips/semantic/se_function.rs`; A05 G3 | V3 must not claim complete typed facts from the current candidate evidence |

### 3.2 Conflicts with `/39` and current drafts

The following are **contract-change items**, not unilateral fixes. The resolution column is a recommendation; the owner/T01 freeze decision is required before implementation.

| ID | Conflict | Current `/39` or draft | V3 requirement | Recommended resolution / owner |
|---|---|---|---|---|
| C-01 | M1-closed type kinds | `TypeKind` lacks pointer/array (`compiler/src/bus.rs:295-323`) | `Pointer`/`Array` with completeness rules | Append-only variant extension; T06 + T01 |
| C-02 | M1-closed AST | Eight node kinds (`compiler/src/bus.rs:543-562`) | Identifier/parameter/declarator/call/subscript/address/deref nodes | Append-only `NodeKind`; T05 + T01; V1 kinds co-frozen |
| C-03 | Symbol identity | `SymbolRecord.decl` is the M1 `Declarator` node (`compiler/src/bus.rs:375-376`); `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md:36-37` says `decl` is the `Identifier` leaf | V3 needs a stable identifier-leaf identity without breaking `/39` M1 tests | Append `ident: Option<NodeId>` and keep `decl`; T06 + T01; alternative redefinition is a larger migration |
| C-04 | Definition state | `SymbolRecord` has no `Declaration`/`Tentative`/`Definition` state | Global tentative/extern/definition and function prototype/definition | Append `definition: DefinitionState`; T06 + T01; alternative separate record is larger |
| C-05 | Scope kinds | `ScopeKind` is `File`/`Block` | Function scope for parameters and body | Append `Function`; T06 + T01 |
| C-06 | Sem binding | `SemRecord` has no symbol binding | Identifier → `SymbolId`; `FunctionDefinition` → function symbol | Append `binding: Option<SymbolId>`; T07 + T01 |
| C-07 | Conversion facts | No `ConversionRecord`; T07 docs leave plan shape open | Lvalue conversion, array/function decay, argument/return conversion | New `ConversionRecord` family, T07 sole writer; T07 + T01; see C-16 |
| C-08 | Effects | `EffectMask` is M1 zero-only | Memory read/write/call/unknown effects | Keep the M1 zero-only rule M1-scoped; add a V3 closed bit vocabulary; T07 + T01 |
| C-09 | Typed constant | `ConstRecord` has no `ty`; `docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md:49` keeps `ConstRecord { ty, value }` as an unselected draft, and T09/A05 mention `ir.op-immediate-type` | Global/static init and IR immediate need a type fact | Do **not** silently add `ty` to `ConstRecord`; carry type on `InitRecord`/`ValueRecord`, or decide a T08/T01 schema amendment; see A05 G4 |
| C-10 | Function parameters | `FunctionRecord` has no parameter links | Definition parameter objects/order | Append `parameters: Vec<SymbolId>`; T09 + T06 + T01 |
| C-11 | IR op set | `IrOp` is `Constant`/`Return`; no object/load/store/call | V3 memory/call IR | Append ops and optional instruction refs; T09 + T01 |
| C-12 | Layout/init materialization | `Layout`/`Init` are `ReservedArena`; `G1DraftBody` lacks them | Typed layout/static init records | Type records, add `Layout`/`Init` draft bodies, commit apply, snapshot; T08 + T01 |
| C-13 | Machine store | `StoreId::Machine` has no typed arena/field | ABI plan handoff | Add `AbiPlan` family/arena and `machine.abi_plans`; T11 + T01 |
| C-14 | Result carriers | No scalar/tuple `ResultValue` | Conversion/type/ABI task results | Use `Record`/`Records`/`Ack`/`Fail`; no new `ResultValue`; T01 |
| C-15 | ABI evidence coverage | `AbiSpec` has only four fields; probe leaves them unresolved | Argument/return/stack/callee-save/relocation evidence | Extend H01/H07 probe evidence and C02; T01 + H01/H07; **BLOCKED_PROBE** |
| C-16 | Conversion producer | T06 catalog lists TY23/TY24/TY27/TY28; T07 catalog lists call/return/assignment | One writer for `ConversionRecord` | Recommend T07 owns conversion facts; T06 chips stay type constructors/helpers or are re-scoped; T06/T07 + T01 |
| C-17 | Stage rows | No target stage rows; existing verifier stage 6 | V3 target stage and ABI verification | Add stages 7/8; keep 0–6 unchanged; T01 + T02 |
| C-18 | Allowlist/schema | No V3 writers/fields | `sem.conversions`, `ir.memory_objects`, `layout.records`, `init.records`, `machine.abi_plans` | Add wave-gated allowlist rows and frozen schema fields; T01 |
| C-19 | A09 ABI key | A09 proposes `AbiPlanRecord { function: FunctionId, ... }` | External prototypes have no `FunctionId` | Reconcile to `signature: TypeId` + optional `function: Option<FunctionId>`; A09 owner + T11 + T01 |
| C-20 | T05 draft node inventory | `docs/tasks/T05_PARSE_CHIPS.md:131-137` lists candidate kinds (`TypeSpecifier`, `ParameterList`, `Identifier`, etc.) beyond the eight actually frozen in `/39` | V3 needs a final minimal list, not a doc-only list | Freeze the V3 `NodeKind` list explicitly; T05 + T01 |
| C-21 | M1 "no memory/calls" | M1 acceptance IR-7 explicitly excludes memory/calls | V3 is post-M1 | Scope VF12 to M1; do not use VF12 as V3 execution evidence; T13 |
| C-22 | Pointer/constant width | `ConstRecord` is magnitude bytes with no width; pointer size is probe-gated | Static init/pointer arithmetic need target layout | Keep Part A symbolic; Part B uses `LayoutRecord`/`TargetSpec`; T08 + T11 |
| C-23 | Attestation plausibility | `target.rs` attestation does not reject zero/oversized counts or bad alignment | V3 ABI plan must be internally consistent | VF11 cross-checks counts/stack alignment/locations; T11 + T13 + T01 |
| C-24 | Global relocation/PIE | No probe fixture for relocations/TLS; Linux PIE default is not fixed | Global addresses and `&g` need a linkable form | Part B only; T11 CG34/CG36 + H01/H07; **BLOCKED_PROBE** |
| C-25 | Recursion/task depth | No V3 recursion contract; `max_task_depth` exists for parser/lower frames | C recursion must not recurse the compiler; frames per activation | C recursion allowed; compiler frames bounded; no new limit without T01; T09 + T11 |
| C-26 | User drafts | Uncommitted parse/lex drafts and `parse/mod.rs` changes are user work | V3 parser work must not overwrite them | Integrator owns `mod.rs`; V3 uses new files; no overwrite; T01 + T05 |

### 3.3 Peer Stage A reports consumed, not treated as contracts

- **[A05](A05_IR_TARGET_HOST.md)** records that `ConstRecord.ty` is missing and `SE29`/VF06 are incomplete; this report treats those as open contract issues, not V3 implementation evidence.
- **[A09](A09_MINIMAL_TARGET_CHAIN_PLAN.md)** proposes the minimal target chain, `AbiPlanRecord`, `FrameRecord`, and `MachineInstructionRecord`; this report aligns names where possible and flags the signature-key conflict. A09 is a proposal, not a freeze.
- **[A13](A13_TESTS_CI_DOCS.md)** records the dirty-workspace baseline and current Clippy warnings from uncommitted drafts; V3 work orders must not interpret those drafts as completed V3 work.

---

## 4. Required Shared Schema

All field orders, enum orderings, numeric tags, family ordinals, task local codes, and diagnostic codes below are **PROPOSED** unless explicitly marked `/39`. T01 owns the final freeze. No ABI value is proposed.

### 4.1 Type and signature records

Function signatures remain the existing function type; no separate signature record is needed. V3 adds pointer and array kinds.

`TypeRecord` remains `{ kind: TypeKind }` (`compiler/src/bus.rs:289-293`).

| Proposed `TypeKind` variant | Fields | Meaning | Owner | Freeze status |
|---|---|---|---|---|
| `Pointer` | `pointee: TypeId` | Pointer/object pointer; V3 has no qualifiers | T06 | Append-only, `T01-FREEZE` |
| `Array` | `element: TypeId`, `bound: ArrayBound` | Complete array or incomplete header | T06 | Append-only, `T01-FREEZE` |
| `ArrayBound` | `Complete(u64)` / `Incomplete` | V3 produces complete arrays; incomplete is syntax-only until T08 | T06/T08 | Append-only, `T01-FREEZE` |

`TypeKind::Function { result, params, prototype, variadic }` is reused (`compiler/src/bus.rs:312-322`). V3 fixes:

- `prototype = true` and `variadic = false` for every V3 function type; `()` old-style and `...` variadic are explicit `Unsupported`.
- `params` are already adjusted: array parameter `T[]`/`T[N]` becomes `Pointer { pointee: T }`; function parameter becomes pointer-to-function (V3 may reject function-pointer parameters explicitly).
- The function result must not be an array or function; V3 allows `void`, integer, and pointer results.
- Parameter names are **not** in the function type. They are represented by definition `SymbolRecord`s and `FunctionRecord.parameters`; prototypes may omit names.

No size/alignment belongs in `TypeRecord`. Layout is T08-owned and target-gated.

### 4.2 Symbol, definition, and scope records

`SymbolRecord` is currently `{ name, scope, kind, ty, linkage, storage, decl }` (`compiler/src/bus.rs:360-377`). V3 appends two fields:

| Proposed append field | Type | Meaning | Owner | Freeze status |
|---|---|---|---|---|
| `ident` | `Option<NodeId>` | The `Identifier` leaf that names the symbol; `None` for legacy M1 records | T06 | Append-only, `T01-FREEZE` |
| `definition` | `DefinitionState` | Declaration/tentative/definition state | T06 | Append-only, `T01-FREEZE` |

`DefinitionState` proposed closed values:

| Value | Meaning | Allowed on |
|---|---|---|
| `Declaration` | Prototype, `extern` declaration, or non-defining declaration | Functions and objects |
| `Tentative` | File-scope `int g;` with no initializer and no prior definition | Objects only |
| `Definition` | Function body, initialized object, local, parameter, static local | Functions and objects |

Rules:

- `decl` remains the M1 declaration/declarator node for `/39` compatibility. T06 lookup uses `ident` when present; otherwise it may use the legacy `decl` path for M1 records. If T01 prefers to redefine `decl` as the identifier leaf, M1 tests and the A09/`ir_function.rs` symbol scan must be migrated; that is a larger contract change (C-03).
- `kind` is `Function` or `Object`; `storage` is `Static` for globals/static locals, `Automatic` for locals/parameters, `None` only for non-objects. `Linkage` is `External`/`Internal`/`None` per C.
- Function prototypes and function definitions use the same `Function` symbol; `definition` distinguishes them. A function definition with `static` has `Linkage::Internal`; `main` is external.
- Parameter symbols are `Object`, `Automatic`, `Linkage::None`, `scope = Function`, `definition = Definition`, `ident = parameter Identifier`.
- Static locals are `Object`, `Static`, `Linkage::None`, `scope = Block`, `definition = Definition`.

`ScopeKind` (`compiler/src/bus.rs:379-386`) appends:

| Proposed variant | Owner node | Use |
|---|---|---|
| `Function` | `FunctionDefinition` node | Parameter declarations and the function body; `Enter` after the signature and before parameter lookup, `Exit` after the body |

The body remains a nested `Block` scope. `ScopeRecord.owner` for a function scope is the `FunctionDefinition` node; `ScopeEventRecord.at` is the same node for `Enter`/`Exit`. File scope remains `File` with no `Exit`. Prototype scope is deferred; prototype parameter names are validated for duplicates but are not ordinary symbols visible outside the prototype.

### 4.3 AST node kinds

V3 adds the following `NodeKind` variants (`compiler/src/bus.rs:543-562` is the current closed set). V1-owned `Declaration`, `Initializer`, `ExpressionStatement`, and `Assignment` are required co-freeze dependencies but are not redefined here.

| Proposed node kind | Children / payload | Meaning | Owner |
|---|---|---|---|
| `Identifier` | no children; `name: Some(NameId)` | A name use or declarator identifier leaf | T05 |
| `ParameterList` | zero or more `Parameter` | Named parameter list | T05 |
| `VoidParameterList` | no children | `(void)`; distinguished from old-style `()` | T05 |
| `Parameter` | `[Specifiers]` or `[Specifiers, Declarator]` | One parameter declaration; unnamed prototype parameters have no `Declarator` | T05 |
| `PointerDeclarator` | `[Declarator]` | One pointer level; nested for `int **p` | T05 |
| `ArrayDeclarator` | `[Declarator, bound expression?]` | Complete array; missing bound is incomplete/deferred | T05 |
| `Call` | `[callee, arg0, arg1, ...]` | Function call expression | T05 |
| `Subscript` | `[base, index]` | `base[index]` | T05 |
| `AddressOf` | `[operand]` | `&operand` | T05 |
| `Dereference` | `[operand]` | `*operand` | T05 |

Node shape rules:

- `FunctionDefinition` children remain `[Specifiers, Declarator, Compound]`; the declarator's children are `[Identifier, ParameterList | VoidParameterList]` or a pointer/array wrapper chain.
- `IntLiteral` remains the literal leaf; `Identifier` is the canonical V3 name leaf. `NodeRecord.name` on a `Declarator` is the legacy M1 convenience field; if present in V3 it must match the `Identifier` child spelling. Token ranges are committed token ranges only.
- `ParameterList` with zero children is old-style `()`; `VoidParameterList` is `(void)`. This avoids using an empty `children` vector for two different C meanings.
- `ArrayDeclarator` bound is an integer constant expression node; V3 initially requires a literal or folded constant, not a VLA.
- `Call` is syntactically direct or indirect; V3 semantics accepts only a direct `Identifier` callee or a function designator; indirect/function-pointer calls are `Unsupported`.
- No `Member`, `Arrow`, `Cast`, `Sizeof`, `Unary` arithmetic, `Comma`, `Conditional`, or compound-literal nodes are added by V3. Those remain V1/V2/T12 work and must be explicit dependencies, not silently parsed as V3 nodes.

V1 co-freeze dependencies required by the canonical V3 fixture (V3 does not redefine them):

| Required V1 kind/record | Why V3 needs it | Freeze owner |
|---|---|---|
| `Declaration` node | Global/local object and function declarations | T05 + V1 |
| `Initializer` node | `int g = 5;`, `int x = 2;`, `int *p = &x;` | T05 + V1 |
| `Assignment` node | `a[0] = 1;` in the fixture | T05/V1 |
| `ExpressionStatement` node | Assignment/void-call statements | T05/V1 |
| `Compound` node | Function body block; already `/39` | T05 + `/39` |
| `Return` node | Return statement; already `/39` | T05 + `/39` |

If any required V1 kind is not co-frozen, V3 parser work is explicitly blocked rather than inventing a private declaration/initializer shape.

### 4.4 Semantic fact, conversion, effect, and alias records

`SemRecord` appends one field:

| Proposed append field | Type | Meaning | Owner | Freeze status |
|---|---|---|---|---|
| `binding` | `Option<SymbolId>` | `Some` on `Identifier` (object/function binding) and on `FunctionDefinition` (the function symbol); `None` elsewhere | T07 | Append-only, `T01-FREEZE` |

`SemRecord` remains one record per `NodeId`. The `binding` on `FunctionDefinition` lets T09 consume the checked function symbol instead of re-scanning the declarator (addresses A05 G3).

`ConversionRecord` is a new T07-owned family:

| Field | Type | Meaning | Freeze status |
|---|---|---|---|
| `node` | `NodeId` | The expression/use node the conversion belongs to | `T01-FREEZE` |
| `role` | `ConversionRole` | Where the conversion is required | `T01-FREEZE` |
| `op` | `ConversionOp` | Closed V3 operation | `T01-FREEZE` |
| `source` | `TypeId` | Source type before conversion | `T01-FREEZE` |
| `dest` | `TypeId` | Destination type after conversion | `T01-FREEZE` |
| `effects` | `EffectMask` | Effects of the conversion itself (e.g. read for lvalue conversion) | `T01-FREEZE` |

Proposed `ConversionRole`:

| Value | Meaning |
|---|---|
| `Value` | A value-context use of an expression (lvalue conversion/decay) |
| `Argument(u16)` | Argument `index` of a call; index is 0-based source order |
| `Return` | Operand of a `return` statement |
| `Initializer` | A static initializer conversion (T08 uses; V3 minimal may be absent) |
| `Assignment` | V1 assignment conversion (co-freeze; not V3-owned) |
| `Condition` | V2 condition conversion (co-freeze; not V3-owned) |

Proposed `ConversionOp` (V3 closed set):

| Value | Meaning | `source` → `dest` |
|---|---|---|
| `Identity` | Explicit no-op required by a role; absence normally means no conversion | `T` → `T` |
| `LvalueToRvalue` | Read an lvalue; removes qualifiers; V3 has no qualifiers so type is unchanged | `T` → `T` |
| `ArrayToPointer` | Array lvalue used as a value; no element read | `Array{T,N}` → `Pointer{T}` |
| `FunctionToPointer` | Reserved for a future function-pointer slice; V3-A direct calls only, so a value use requiring it is `Unsupported` | `Function` → `Pointer{Function}` |

Rules:

- Exactly one `ConversionRecord` per required `(node, role)` key; `Argument(i)` keys are distinct. A missing required conversion is a VF06 failure; a duplicate is a VF06 failure.
- A direct callee that is a function designator does **not** get `FunctionToPointer`; the call checks the designator directly. Indirect calls are `Unsupported` in V3, so `FunctionToPointer` is primarily a verifier/future fact and must not be fabricated as a callable pointer.
- `LvalueToRvalue` has `effects = READ_MEMORY`; `ArrayToPointer`/`FunctionToPointer`/`Identity` have zero effects.
- `SemRecord.effects` describes evaluation of the expression as an lvalue/designator (address computation, call effects), not the read performed by a later conversion. This distinction is required so `&*p` does not load and `*p` used as a value does.
- An array name used as the base of a subscript is a value-context use: record `ArrayToPointer` with role `Value` on the base node. The subscript expression itself is an lvalue; a later value use reads the element through `LvalueToRvalue`.

`EffectMask` V3 vocabulary (exact bit assignments are `T01-FREEZE`; no numeric value is assigned here):

| Bit name | Meaning | V3 status |
|---|---|---|
| `READ_MEMORY` | Reads an object through a pointer/lvalue conversion | Required |
| `WRITE_MEMORY` | Writes an object | Required |
| `CALL` | Performs a function call | Required |
| `UNKNOWN_MEMORY` | Call/indirect may read/write unknown memory; conservative | Required |
| `VOLATILE` | Volatile access | Reserved; V3 unsupported |
| `ATOMIC` | Atomic access | Reserved; V3 unsupported |

The `/39` M1 rule `sem.effect-mask-zero-only` remains scoped to M1. V3 adds a separate rule requiring the closed V3 bit set; a `VOLATILE`/`ATOMIC` bit or any undeclared bit is a typed `Unsupported`, not silently ignored.

Alias facts are carried on the memory object, not as an optimization permission:

| `AliasClass` | Meaning |
|---|---|
| `Global` | Static/global object |
| `NonEscapingLocal` | Automatic object whose address is not taken and which does not escape |
| `EscapingLocal` | Automatic object whose address is taken/stored/passed |
| `Parameter` | Parameter object; may alias caller memory |
| `Unknown` | Conservative fallback |

V3 code generation must be correct under `Unknown`: it must not reorder memory operations across calls, and `AliasClass` must not be used by T10 as a no-alias proof without a later accepted analysis. The alias fact exists for verification and future optimization.

### 4.5 Memory object record

`MemoryObjectRecord` is a new T09-owned family. It describes a source-level object template; it is **not** a runtime activation.

| Field | Type | Meaning | Freeze status |
|---|---|---|---|
| `symbol` | `Option<SymbolId>` | `Some` for a named object; `None` reserved for compiler temporaries (V3 has none) | `T01-FREEZE` |
| `kind` | `ObjectKind` | Global / static local / local / parameter | `T01-FREEZE` |
| `ty` | `TypeId` | Object type (may be array/pointer) | `T01-FREEZE` |
| `owner` | `Option<FunctionId>` | Enclosing function for `StaticLocal`/`Local`/`Parameter`; `None` for `Global` | `T01-FREEZE` |
| `parameter_index` | `Option<u16>` | 0-based parameter index for `Parameter`; `None` otherwise | `T01-FREEZE` |
| `decl` | `NodeId` | Declaration/definition node (source location/provenance) | `T01-FREEZE` |
| `layout` | `Option<LayoutId>` | Part B layout descriptor; `None` until layout is target-gated | `T01-FREEZE` |
| `init` | `Option<InitId>` | Static initialization for global/static local; `None` for automatic objects | `T01-FREEZE` |
| `alias` | `AliasClass` | Conservative alias class | `T01-FREEZE` |
| `address_taken` | `bool` | Whether the source takes/stores/passes the address | `T01-FREEZE` |

`ObjectKind` proposed values:

| Value | Storage | Linkage | Activation behavior |
|---|---|---|---|
| `Global` | Static | External/Internal | One object for the program/TU |
| `StaticLocal` | Static | None | One object shared by all activations; constant initialization only in V3 |
| `Local` | Automatic | None | New object per function activation; frame-relative in Part B |
| `Parameter` | Automatic | None | New object per activation, initialized from its ABI location |

`MemoryObjectRecord` rules:

- Exactly one `MemoryObjectRecord` per defined/tentative named object; a declaration-only `extern` object has none.
- A `Local`/`Parameter` record is a template. Recursion creates multiple runtime activations; the record does not represent one activation and must not own a single static frame slot.
- A `StaticLocal` is shared across activations and is not allocated per call. Its assembly label must be deterministic and derived from `SymbolId` (not source path/filename).
- Array objects have array type; their layout/stride is Part B. Pointer arithmetic IR names the element type, not a numeric stride.
- `layout` is `None` in Part A; Part B fills it from T08. `init` may reference a Part A `Zero`/`Integer` `InitRecord` for static objects; `Address` initialization is Part B. Automatic objects have no `InitRecord`; their V1 runtime initialization is IR stores.

### 4.6 Layout, static initialization, and global data

`LayoutRecord` types the existing `LayoutId` reserved arena:

| Field | Type | Meaning | Freeze status |
|---|---|---|---|
| `ty` | `TypeId` | Type whose layout is described | `T01-FREEZE` |
| `kind` | `LayoutKind` | Scalar / pointer / array | `T01-FREEZE` |
| `size` | `Option<u64>` | Target size; `None` unless `ProbeVerified` | `T01-FREEZE` |
| `align` | `Option<u64>` | Target alignment; `None` unless `ProbeVerified` | `T01-FREEZE` |
| `state` | `TargetEvidenceState` | `Symbolic` or `ProbeVerified { report_hash }` | `T01-FREEZE` |

`LayoutKind` proposed values: `Scalar`, `Pointer`, `Array { count: u64 }`. Function/void types have no layout and are rejected. `Array` size/align derive from the element layout and count; overflow is a typed diagnostic, not a host wrap. `Pointer` size/align come from the target pointer scalar; **UNVERIFIED** until attestation. `Scalar` sizes/aligns come from `TargetSpec`; **UNVERIFIED**.

`TargetEvidenceState` is the shared gate discriminator for `LayoutRecord` and `AbiPlanRecord`:

| Variant | Fields | Meaning |
|---|---|---|
| `Symbolic` | none | Shape-only fact; no concrete target size/alignment/location; not codegen-consumable |
| `ProbeVerified` | `report_hash: [u8; 32]` | Values derive from an attested `TargetSpec`; the hash binds the target evidence |

No numeric value is stored in `Symbolic`; the `report_hash` is produced by T01 attestation, not by this report.

`InitRecord` types the existing `InitId` reserved arena:

| Field | Type | Meaning | Freeze status |
|---|---|---|---|
| `symbol` | `SymbolId` | Static object being initialized | `T01-FREEZE` |
| `ty` | `TypeId` | Object type | `T01-FREEZE` |
| `kind` | `InitKind` | Zero / integer / address | `T01-FREEZE` |

`InitKind` proposed values:

| Value | Payload | V3 status |
|---|---|---|
| `Zero` | none | Required for tentative/zero globals and static locals |
| `Integer(ConstId)` | committed T08 constant | Required for `int g = 5;` and constant static-local init |
| `Address { symbol: SymbolId, addend: i64 }` | symbol + byte addend | Boundary; Part B relocation, explicit otherwise |

Rules:

- `InitRecord` is static initialization only. Automatic/local initializers are V1 runtime stores, not `InitRecord`s.
- V3 minimal supports `Zero` and `Integer`; `Address` requires a relocation plan and is **BLOCKED_PROBE** until T11 CG34/CG36 and the target evidence exist.
- `Integer` uses the committed `ConstRecord`; the type/width/truncation check comes from `InitRecord.ty` and, in Part B, the layout. Do not add a second untyped value path.
- No host address, pointer, file path, or timestamp may appear in an `InitRecord`. `Address` is a symbolic symbol + addend.
- Aggregate/designated/global array initializers are T08 CL16–CL22 work and are explicit `Unsupported` in V3 minimal. V3 must not silently zero-fill or partially accept an unsupported initializer form.
- `InitKind::Zero` with a pointer type is not automatically a null pointer: the target null representation is probe-gated. V3 defers pointer null initialization/conversion (Q-18) rather than assuming all-bits-zero.

### 4.7 IR function, value, and instruction records

`FunctionRecord` (`compiler/src/bus.rs:449-460`) appends:

| Proposed append field | Type | Meaning | Freeze status |
|---|---|---|---|
| `parameters` | `Vec<SymbolId>` | Definition parameters in signature order; empty for `(void)`/no parameters | `T01-FREEZE` |

A prototype has no `FunctionRecord`; it has a `SymbolRecord` and a function `TypeId`. A definition's `parameters[i]` must be an `Object`/`Automatic`/parameter symbol whose type equals the adjusted `TypeKind::Function.params[i]`. `entry` remains the definition entry block. No `FunctionContextId` is introduced (rev-42 decision).

`InstructionRecord` (`compiler/src/bus.rs:480-494`) appends optional refs:

| Proposed append field | Type | Meaning | Freeze status |
|---|---|---|---|
| `object` | `Option<MemoryObjectId>` | Object referenced by `ObjectAddress` | `T01-FREEZE` |
| `conversion` | `Option<ConversionId>` | Conversion fact referenced by `Convert` | `T01-FREEZE` |
| `parameter` | `Option<u16>` | Incoming parameter index for `Param` | `T01-FREEZE` |
| `callee` | `Option<SymbolId>` | Direct call target for `Call` | `T01-FREEZE` |

`IrOp` appends the V3 closed set. The exact op table is:

| Op | Operands | Immediate | Object/conversion/param/callee | Result | Terminator | Effect class |
|---|---|---|---|---|---|---|
| `Constant` | 0 | `Some(ConstId)` | none | `Some` | no | none |
| `Add` | 2 | none | none | `Some` | no | none |
| `Convert` | 1 | none | `conversion: Some` | `Some` | no | from conversion |
| `ObjectAddress` | 0 | none | `object: Some` | `Some` | no | none (address computation) |
| `Load` | 1 (address) | none | none | `Some` | no | `READ_MEMORY` |
| `Store` | 2 (address, value) | none | none | `None` | no | `WRITE_MEMORY` |
| `GetElementPtr` | 2 (base pointer, index) | none | none | `Some` | no | none |
| `Param` | 0 | none | `parameter: Some` | `Some` | no | none |
| `Call` | arguments in source order | none | `callee: Some` | `Some` for non-void, `None` for void | no | `CALL`, `READ_MEMORY`, `WRITE_MEMORY`, `UNKNOWN_MEMORY` conservatively |
| `Return` | 0 (void) or 1 | none | none | `None` | yes | none |

Rules:

- `GetElementPtr` takes the pointee/element type from the base operand's `Pointer` type; no numeric stride is stored in IR. T11 computes stride from `LayoutRecord` in Part B.
- `Load`/`Store` widths come from the addressed object/pointee type and layout; Part B only.
- `Call` is direct only; `callee` must be a `Function` symbol. Indirect/function-pointer calls are `Unsupported` in V3.
- `Param` is an entry-block instruction; `parameter < signature.params.len()`. Parameter objects and `Param` results are linked by index and symbol type. T09 must make the parameter object addressable: it may use the `Param` value directly for reads and store it into the parameter `MemoryObject` when the address is taken. The object remains per activation.
- `Return` is the unique terminator and the greatest `InstructionId` in its block; no instruction follows it. `Return` with no operand is the void return. Falloff handling is a T09/T11 rule, not a fabricated `0`.
- `Convert` must name a committed `ConversionRecord` whose `source`/`dest` match the operand/result types.
- No `Phi`, `Br`, `Switch`, `Call` indirect, `Alloca`, `Copy`, atomic, volatile, or aggregate ops in V3.

### 4.8 ABI plan and calling-convention handoff

`AbiPlanRecord` is a new T11-owned family. It is the shared handoff between typed IR calls and target machine lowering; it is **not** a machine instruction record. Every concrete location/width/offset is absent until `ProbeVerified`.

| Field | Type | Meaning | Freeze status |
|---|---|---|---|
| `signature` | `TypeId` | Function type key; one plan per signature | `T01-FREEZE` |
| `function` | `Option<FunctionId>` | `Some` for the plan of a defined function; `None` for an external prototype signature | `T01-FREEZE` |
| `parameter_classes` | `Vec<AbiValueClass>` | Symbolic class per parameter, in signature order | `T01-FREEZE` |
| `return_class` | `AbiValueClass` | Symbolic return class | `T01-FREEZE` |
| `variadic` | `bool` | V3 must be `false`; `true` is `Unsupported` | `T01-FREEZE` |
| `state` | `TargetEvidenceState` | `Symbolic` or `ProbeVerified { report_hash }` | `T01-FREEZE` |
| `return_location` | `Option<AbiReturnLocation>` | Present only when `ProbeVerified` and return is not void | `T01-FREEZE` |
| `arg_locations` | `Vec<AbiArgLocation>` | Empty in `Symbolic`; one per parameter in `ProbeVerified` | `T01-FREEZE` |
| `stack_align` | `Option<u8>` | Copied from attested `AbiSpec.stack_align`; `None` in `Symbolic` | `T01-FREEZE` |

Proposed enums:

| Enum | Values | Notes |
|---|---|---|
| `AbiValueClass` | `Integer`, `Pointer`, `Void` | V3 closed set; `Void` is only legal on `return_class`; no FP/aggregate/HFA/HVA |
| `AbiArgLocation` | `{ parameter_index: u16, class: AbiValueClass, location: AbiArgLocationKind }` | Indices contiguous `0..n-1`; `class` is `Integer`/`Pointer` only |
| `AbiArgLocationKind` | `GpArgument { ordinal: u8 }`, `StackArgument { byte_offset: u64 }` | Argument positions only; ordinals/offsets are probe-derived, never literals here |
| `AbiReturnLocation` | `GpResult { ordinal: u8 }` | V3 scalar integer/pointer return only; no indirect/aggregate |

Reconciliation with A09:

- A09 proposes `AbiPlanRecord { function: FunctionId, return_location, arg_locations, variadic, stack_align, target_report_hash }` for the M1 leaf function. V3 accepts that machine-specific intent but needs a **signature-keyed** plan because an external prototype has no `FunctionId`.
- Recommended shape: `AbiPlanRecord.signature` is the primary key; `function: Option<FunctionId>` links a defined function; A09's `TargetFunctionRecord.abi_plan` continues to link a target function to its plan. `target_report_hash` is carried by `TargetEvidenceState::ProbeVerified`; `stack_align` is Option until verified.
- A09's `ReturnLocation`/`ArgLocation` machine/vreg fields are not duplicated here. They remain T11 Part B machine records. The shared handoff uses `AbiArgLocationKind`/`AbiReturnLocation` (symbolic GP/stack/result positions) until T11 lowers to vregs/machine instructions.
- A09's `FrameRecord` and `MachineInstructionRecord` remain T11-owned machine records and are not redefined as shared V3 schema. V3 requires the frame record to be a per-function template, not a per-call-site static slot: every recursive activation gets its own automatic objects/parameters, while `StaticLocal` objects are shared. Frame size/alignment/register-save fields are **UNVERIFIED** Part B data.

ABI gate rules:

1. `Symbolic` plans may be shape-tested but **must not** drive codegen or machine records. They carry no locations, no stack alignment, and no report hash.
2. `ProbeVerified` plans require `TargetSpec::require_verified() == Ok(())` and a report hash from the attested target; they are the only plans T11 machine lowering may consume.
3. Every `AbiArgLocationKind`/`AbiReturnLocation` value must be derived from `AbiSpec`/probe evidence; no literal register number, stack offset, or alignment may be written in a chip.
4. V3 accepts only `Integer`, `Pointer`, and `Void`; float, aggregate, HFA/HVA, indirect return, variadic, and function-pointer call plans are explicit `Unsupported`.
5. `GpArgument { ordinal }` must satisfy `ordinal < abi.gp_arg_regs` for the attested target; `StackArgument { byte_offset }` must be aligned and within the caller stack area; `GpResult` is a single probe-established result position. The exact bounds/positions are **UNVERIFIED**.
6. No Darwin/Mach-O/Darwin-variadic value is representable; the identity and backend class rule remain `aarch64-linux`.

### 4.9 New IDs, families, stores, and materialization

The three new families are append-only. **No numeric tag, ordinal, or task local code is assigned in this report.**

| New family | ID | Arena / store field | Producer | Notes |
|---|---|---|---|---|
| `Conversion` | `ConversionId` | `sem.conversions` | T07 | One per required `(node, role)` |
| `MemoryObject` | `MemoryObjectId` | `ir.memory_objects` | T09 | One per defined/tentative named object |
| `AbiPlan` | `AbiPlanId` | `machine.abi_plans` | T11 | Signature-keyed; Part B locations gated |

Existing IDs/refs reused: `LayoutId`/`RecordRef::Layout`, `InitId`/`RecordRef::Init`, `FunctionId`, `BlockId`, `ValueId`, `InstructionId`, `SymbolId`, `TypeId`, `NodeId`, `SemId`, `ConstId`.

Required integrator work:

- `compiler/src/ids.rs`: new ID newtypes, `RecordFamily`, `RecordRef`, labels, wire tags (T01 assigns).
- `compiler/src/bus.rs`: typed arenas `sem.conversions`, `ir.memory_objects`, `machine.abi_plans`; type `layout.records`/`init.records`; `Arenas::allocated_total`.
- `compiler/src/records.rs`: `G1DraftBody` variants for `Conversion`, `MemoryObject`, `AbiPlan`, `Layout`, `Init`.
- `compiler/src/commit.rs`: typed materialization, predicted-ref validation, per-arena capacity, family checks for the new families.
- `compiler/src/manifest.rs`: `StoreSchema` fields and wave-gated `STORE_OWNER_ALLOWLIST` rows; no duplicate writers.
- `compiler/src/snapshot.rs`: encode/decode and round-trip for every new record and optional field.
- `compiler/src/contract.rs`: names, record-field lists, enum-name lists, rules, hash; `compiler/contracts/CONTRACT_VERSION`; `compiler/contracts/COMPILER_SFL_MANIFEST.md` if its wording changes.
- `compiler/tests/freeze.rs` and a new V3 freeze test: hash/version consistency, no duplicate rules/tags, snapshot round-trip.

The two-tier hash model applies: the V3 record/task/field/rule shapes must enter the frozen seed; a post-seed runtime `StoreSchema::declare()` alone is not a freeze.

### 4.10 Task and result shapes

All task payloads remain `RecordRef`-only. No scalar flags are placed in payloads; flags come from AST/type/record facts. All results use the frozen `ResultValue` carriers; no new variant.

| Owner | Proposed task kind | Payload (ordered refs) | Result | Notes |
|---|---|---|---|---|
| T06 | `symbol_type.function` (extend existing `SYMBOL_FUNC_TYPE`) | `[result Type, param Type...]` | `Record(Type)` | Prototype true, variadic false; `(void)` empty params; adjusted params |
| T06 | `symbol_type.pointer` | `[pointee Type]` | `Record(Type)` | Pointer to object/function/void; no qualifiers |
| T06 | `symbol_type.array` | `[element Type, bound Const]` | `Record(Type)` | Complete array only; bound must be an ICE |
| T06 | `symbol_type.declarator_bind` | `[base Type, declarator Node]` | `Record(Type)` | Deterministic declarator walk; no Rust recursion |
| T06 | `symbol_type.compatibility` | `[left Type, right Type]` | `Ack` / `Fail` | Exact structural compatibility; composite merge deferred |
| T06 | `symbol_type.declare` (extend existing `SYMBOL_DECLARE`) | `[Specifiers Node, Declarator Node, Type, Scope]` | `Record(Symbol)` | Object/function; reads storage/initializer context from nodes |
| T06 | `symbol_type.scope_enter` (extend existing) | `[owner Node]` | `Record(Scope)` | `FunctionDefinition` → Function scope; `Block` → Block scope |
| T06 | `symbol_type.lookup` (extend existing) | `[Scope, Name, query Node]` | `Record(Symbol)` / `Fail` | Query point from committed node/token range; miss is typed |
| T07 | `semantic.name_expr` | `[Identifier Node]` | `Record(Sem)` | Appends binding; no conversion |
| T07 | `semantic.call_expr` | `[Call Node]` | `Records([Sem, Conversion...])` | Direct callee only; args checked left-to-right; call effects |
| T07 | `semantic.address_of` | `[AddressOf Node]` | `Record(Sem)` | Operand lvalue/designator; no operand read |
| T07 | `semantic.deref` | `[Dereference Node]` | `Record(Sem)` | Pointer to complete object/function; lvalue result |
| T07 | `semantic.subscript` | `[Subscript Node]` | `Record(Sem)` | Array decay or pointer base; lvalue element |
| T07 | `semantic.global_decl` | `[Declaration Node]` | `Ack` | Checks storage/definition/initializer combination; no new record |
| T07 | `semantic.function_def` (extend existing) | `[FunctionDefinition Node]` | `Record(Sem)` | Parameters/function scope/body/return; binding to function symbol |
| T07 | `semantic.return_stmt` (extend existing) | `[Return Node]` | `Records([Sem, Conversion?])` | Return conversion; void/no-expression rules |
| T07 | `semantic.binary_expr` (extend existing) | `[BinaryAdd Node]` | `Record(Sem)` | Int/pointer arithmetic; pointer-pointer difference deferred |
| T08 | `layout.type` | `[Type]` | `Record(Layout)` | `Symbolic` Part A; `ProbeVerified` Part B |
| T08 | `init.static_zero` | `[Symbol, Type]` | `Record(Init)` | Tentative/zero global or static local |
| T08 | `init.static_scalar` | `[Symbol, Type, Const]` | `Record(Init)` | Constant scalar initializer; T08 is the sole `Init` writer |
| T08 | `init.static_address` | `[object Symbol, Type, target Symbol]` | `Record(Init)` | Address of `target` with zero addend only; nonzero addend/relocation Part B; **BLOCKED_PROBE** |
| T09 | `ir.global_object` | `[Symbol]` | `Record(MemoryObject)` | One per defined/tentative global; deterministic order |
| T09 | `ir.function` (extend existing) | `[FunctionDefinition Node]` | `Record(Function)` | Appends function/block/values/instructions/memory objects; V3 straight-line subset |
| T11 | `target.abi_plan` | `[signature Type]` | `Record(AbiPlan)` | Part B; symbolic/verified state; external signatures allowed |
| T11 | `target.call` (Part B) | `[Function, Instruction]` | `Ack` / `Records` | Machine call sequence; depends on `AbiPlan`; not fully specified here |
| T13 | `verification.typed_invariant` (extend existing) | `[TU Node]` | `Ack` | VF06 V3 facts |
| T13 | `verification.ir_invariant` | `[Function]` | `Ack` | VF08 V3 op/memory/type checks |
| T13 | `verification.abi_invariant` | `[AbiPlan]` | `Ack` | VF11 gate/classes/location/stack checks |
| T13 | `verification.target_function_invariant` | `[Function]` | `Ack` | VF11 plan/frame/machine linkage for a defined function (Part B) |
| T13 | `verification.store_invariant` (extend existing) | empty | `Ack` | New record refs, ownership, bounds |
| T13 | `verification.token_ast_invariant` (extend existing) | `[TU Node]` | `Ack` | New node kinds/ranges/children |

Proposed stage placement (T01/T02 assign the final ordinals; existing 0–6 are not renumbered):

| Stage | Content | V3 kinds |
|---|---|---|
| 3 | Symbol/type construction | T06 function/pointer/array/declarator-bind/compatibility/declare/scope-enter/lookup; T08 `layout.type` |
| 4 | Semantic checking and typed facts | T07 name/call/memory/subscript/global/function-definition/return/binary; T08 `init.static_zero`/`init.static_scalar` |
| 5 | IR lowering | T09 `ir.global_object`, `ir.function` |
| 6 | Post-IR/frontend verification | Existing VF01/VF05/VF06 plus new VF08 (`verification.ir_invariant`) |
| 7 | Target ABI/machine lowering | T11 `target.abi_plan`, `target.call`, machine/frame/assembly (Part B) |
| 8 | Target/ABI verification | VF11 (`verification.abi_invariant`, `verification.target_function_invariant`) and target evidence checks (Part B) |

Task lifecycle rules:

- Use `AwaitChildren` for T07 type/lookup/conversion children and T09 lowering children; `WaitSet` is the sole awaited-child source and is not duplicated in a continuation.
- A child enqueued in tick `T` is executable in `T+1`; no same-tick re-entry.
- Exactly one `Complete`/`Fail`/`AwaitHost` transition per task per batch; `Progress` only with a strictly advancing ordinal; `max_task_depth` bounds parser/lower frames; no idle spin.
- `Records` results are ordered by production order and consumed exactly once. `Record` results name the primary record; other appended records are found by deterministic symbol/owner scans, not hidden caches.
- `Fail` carries a typed `DiagnosticDraft`; structural reference violations are commit errors, not semantic `Fail`; target unavailability is `ProbeRequired`/`Config`-class, not fabricated success.
- A task kind not belonging to a chip must not change semantic state; a dispatched task must progress, wait explicitly, complete, or fail.

### 4.11 Verification facts

V3 extends existing VF01/VF05/VF06 and adds VF08/VF11 facts. No new verifier chip ID is asserted; the integrator assigns any new chip IDs.

| Fact ID | Check | Owner | Negative fixture |
|---|---|---|---|
| `VF06-V3-1` | Every `NodeId` consumed by T09 has exactly one committed `SemRecord` | T13 | Missing/duplicate typed fact |
| `VF06-V3-2` | Every `Identifier` has a binding; every `FunctionDefinition` binding is the declared function symbol | T13 | Unbound identifier; wrong function symbol |
| `VF06-V3-3` | Every required `(node, role)` conversion exists exactly once; direct callee has no function decay | T13 | Missing lvalue conversion; duplicate conversion |
| `VF06-V3-4` | Array decay appears exactly once at a value use; `&array` and `sizeof` are exceptions | T13 | Double decay; decay in `&` context |
| `VF06-V3-5` | Call signature/argument count/types/return type are consistent; no implicit declaration | T13 | Wrong arg count; wrong arg type; undeclared callee |
| `VF06-V3-6` | Effects are conservative and contain the closed V3 bits; volatile/atomic absent | T13 | Missing call/read/write effect; volatile bit |
| `VF08-V3-1` | Op table references are well-formed: object/conversion/parameter/callee/operand/result | T13 | Missing object; wrong conversion source |
| `VF08-V3-2` | `MemoryObject` symbol/owner/parameter index/type/layout/init links are consistent | T13 | Parameter index out of range; object without symbol |
| `VF08-V3-3` | `Load`/`Store` address/value types match the pointee/object; no void/function load | T13 | Load of function; store to void/function object |
| `VF08-V3-4` | `Call` callee is a function symbol; args match signature; result type matches | T13 | Call object; result mismatch |
| `VF08-V3-5` | `Return` type matches function result; unique terminator; no instruction after it | T13 | Return int from void; instruction after terminator |
| `VF11-V3-1` | Every consumable `AbiPlan` is `ProbeVerified`; no `Symbolic` plan reaches machine records | T13 | Symbolic plan consumed; missing report hash |
| `VF11-V3-2` | Classes/entries are complete and contiguous; variadic false; no FP/aggregate | T13 | Missing arg entry; variadic true |
| `VF11-V3-3` | Argument GP ordinals < attested `gp_arg_regs`; one result position from target evidence; stack offsets aligned/within area | T13 | Argument ordinal out of range; unaligned stack; missing result evidence |
| `VF11-V3-4` | No Darwin/Mach-O/host ABI; identity is Linux ELF/LP64/LE/AAPCS64 | T13 | Darwin register/stack model |
| `VF11-V3-5` | A defined function's linked `AbiPlan` matches its signature; frame/machine records use that plan; no cross-signature plan reuse | T13 | Wrong plan link; frame/plan mismatch |
| `VF01-V3-1` | New refs resolve; one memory object per symbol; one init per static object; one plan per signature | T13 | Dangling conversion; duplicate object |
| `VF05-V3-1` | V3 node kinds/ranges/children are coherent; Identifier is a leaf; `(void)` vs `()` distinguishable | T13 | Identifier with children; empty list ambiguity |
| `VF13-V3-1` | Replay includes all new records; no scheduler-only drift | T13 | Record field omitted from snapshot; ID order drift |

VF12 remains an M1 symbolic interpreter; V3 must not use it as execution evidence unless a separate T13 contract extension is accepted.

---

## 5. Ownership and Producer/Consumer Matrix

### 5.1 Single-writer matrix

| Fact / store field | Single writer | Consumers | Freeze owner | Forbidden duplicate |
|---|---|---|---|---|
| `types.records` | T06 (`TY13/TY15/TY16/TY17/TY20`) | T05 typedef lookup, T06 lookup/compat, T07, T08, T09, T11, T13 | T06 + T01 | No T07/T09 type appends |
| `symbols.symbols` | T06 (`SYMBOL_DECLARE`) | T07, T09, T11, T13 | T06 + T01 | No parser/T09 symbol appends |
| `symbols.scopes`, `symbols.scope_events` | T06 scope chips | T07, T09, T13 | T06 + T01 | No parser scope writes |
| `parse.nodes` | T05 | T06, T07, T09, T13 | T05 + T01 | No semantic node appends |
| `sem.records` | T07 | T09, T13 | T07 + T01 | No T09 re-check |
| `sem.conversions` | T07 | T09, T13 | T07 + T01 | T06 TY23/24/27/28 must not write |
| `layout.records` | T08 | T08 init, T09, T11, T13 | T08 + T01 | No T11 private layout |
| `init.records` | T08 | T11, T13 | T08 + T01 | No T09 re-fold/re-init |
| `ir.memory_objects` | T09 | T11, T13 | T09 + T01 | No T08/T06 object writes |
| `ir.functions` | T09 | T11, T13 | T09 + T01 | No T07 function record |
| `ir.instructions` | T09 | T11, T13 | T09 + T01 | No T11 re-lowering |
| `machine.abi_plans` | T11 | T11 machine, T13 | T11 + T01 | No T09 ABI locations |
| `config.target` | T01/C02 | T11 Part B only | T01/C02 | No chip reads host ABI |
| Effect bits | T01/T07 co-freeze | T07 writes facts; T09/T11/T13 read | T07 + T01 | No local effect enums |
| Alias facts | T09 | T11/T13; T10 future | T09 + T01 | No optimizer assumption |
| Verification facts | T13 | T13 | T13 + T01 | No verifier source writes |

### 5.2 Producer/consumer dependency graph

```text
T05 AST nodes
  -> T06 types/symbols/scopes (typedef/declarator/lookup)
  -> T07 SemRecord + ConversionRecord (checks, effects)
       -> T08 layout/static init (array bound/global init/type completeness)
       -> T09 MemoryObject + Function + Instruction (IR)
            -> T11 AbiPlan (Part B, probe-gated)
                 -> T11 machine/frame/assembly (Part B)
  -> T13 VF05/VF06/VF01/VF08/VF11/VF13
```

Data flow rules:

- T06 may request T08 layout/constant service for array bounds and completeness, but T06 does not compute target layout itself.
- T07 checks types and conversions; it does not emit target instructions.
- T08 produces layout/static-init records for T09/T11; it does not lower function bodies.
- T09 consumes committed `SemRecord`/`ConversionRecord`/`TypeRecord`/`SymbolRecord`/`LayoutRecord`/`InitRecord`; it does not re-infer types or re-fold constants.
- T11 consumes committed IR and only a `ProbeVerified` `AbiPlan`; it does not read AST or re-check C semantics.
- T13 reads committed records and emits verification facts/diagnostics; it does not mutate semantic stores.
- No chip calls another chip directly; all edges are task requests/results or committed records.

### 5.3 File ownership for V3

| Area | Exact owned paths (proposed unless already existing) |
|---|---|
| T01 integrator freeze | `compiler/src/ids.rs`, `compiler/src/bus.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/records.rs`, `compiler/src/commit.rs`, `compiler/src/snapshot.rs`, `compiler/src/contract.rs`, `compiler/contracts/CONTRACT_VERSION`, `compiler/contracts/COMPILER_SFL_MANIFEST.md`, `compiler/tests/freeze.rs`, `compiler/tests/v3_freeze.rs` |
| T05 parser | `compiler/src/chips/parse/pa_v3_decl.rs`, `pa_v3_params.rs`, `pa_v3_expr.rs`; `compiler/tests/v3_parse.rs` |
| T06 types/symbols | `compiler/src/chips/types/ty_derived.rs`, `ty_function_params.rs`; `compiler/src/chips/symbols/ty_object_decl.rs`, `ty_function_scope.rs`; `compiler/tests/v3_ty.rs` |
| T07 semantics | `compiler/src/chips/semantic/se_name.rs`, `se_call.rs`, `se_memory_expr.rs`, `se_conversion.rs`, `se_global_decl.rs`; `compiler/tests/v3_se.rs` |
| T08 layout/init | `compiler/src/chips/constant_layout_init/cl_layout.rs`, `cl_static_init.rs`; `compiler/tests/v3_layout_init.rs` |
| T09 IR | `compiler/src/chips/ir_lower/ir_global_object.rs`, `ir_memory.rs`, `ir_call.rs`, `ir_function_v3.rs`; `compiler/tests/v3_ir.rs` |
| T11 ABI Part A/B | `compiler/src/chips/target/aarch64/cg_abi_plan.rs`, `cg_location.rs`, `cg_call.rs`, `cg_frame.rs`, `cg_global_data.rs`, `cg_emit.rs`; `compiler/tests/v3_abi_plan.rs`, `v3_abi_probe.rs` |
| T13 verification | `compiler/src/chips/verify/vf_invariant.rs` (extend), `vf_store.rs` (extend), `vf_ir.rs`, `vf_abi.rs`; `compiler/tests/v3_verify.rs` |
| Integration/driver | `compiler/src/chips/mod.rs`, `compiler/src/chips/parse/mod.rs`, `compiler/src/chips/types/mod.rs`, `compiler/src/chips/symbols/mod.rs`, `compiler/src/chips/semantic/mod.rs`, `compiler/src/chips/constant_layout_init/mod.rs`, `compiler/src/chips/ir_lower/mod.rs`, `compiler/src/chips/target/aarch64/mod.rs` (new), `compiler/src/routing.rs`, `compiler/src/bin/candidate.rs`, `compiler/tests/v3_integration.rs` |
| Independent V3 tests | `compiler/tests/v3_acceptance.rs` (test-only; no source edits) |

Group `mod.rs` files and `compiler/src/chips/mod.rs` are integrator-owned even when a group lead proposes their contents. No V3 worker edits them.

Test-path note: the current repository uses flat `compiler/tests/*.rs` files. The V3 paths above follow that convention. If the integrator migrates to `compiler/tests/chips/<group>/`, the integrator assigns the equivalent path in the work order; a worker does not create a private test entry point.

---

## 6. Legality, Error, Unsupported, and ABI-Gate Rules

The following rule IDs are proposed names for the T01/T13 rule inventory; exact hashes and diagnostic codes are **not** assigned here. “Fail” means a typed chip `Fail`/`DiagnosticDraft`; “CommitError” means structural pre-mutation rejection; “Unsupported” means explicit unsupported capability, never a silent success.

### 6.1 Types and declarations

| Rule ID | Requirement | Failure class | Owner |
|---|---|---|---|
| `v3.type.pointer-pointee` | Pointer pointee may be object/function/void; V3 has no qualifiers | Fail(Semantic) / Unsupported | T06 |
| `v3.type.array-element` | Array element is a complete object type, not function/void/incomplete; V3 complete arrays only | Fail(Semantic) / Unsupported | T06/T08 |
| `v3.type.array-bound` | Bound is a positive integer constant expression; zero/negative/VLA/incomplete local array rejected | Fail(Semantic) / Unsupported | T06/T08 |
| `v3.type.object-void` | An object cannot have void or function type; an object definition requires a complete type (incomplete array declarations are a T08 boundary, not a V3-A definition) | Fail(Semantic) | T06 |
| `v3.type.function-result` | Function result is not array/function; V3 allows void/int/pointer | Fail(Semantic) | T06 |
| `v3.type.function-params` | Array/function parameters adjust to pointers; parameter count/order preserved | Fail(Semantic) | T06 |
| `v3.type.function-prototype` | V3 function types are prototypes (`(void)` or typed list); `()` old-style and `...` variadic are Unsupported | Unsupported | T06/T07 |
| `v3.type.compatibility` | Exact structural compatibility for result/params/prototype/array/pointer; composite merge deferred | Fail(Semantic) | T06 |
| `v3.decl.one-definition` | One definition per symbol; multiple initialized definitions conflict | Fail(Semantic) | T06 |
| `v3.decl.tentative` | File-scope `int g;` is tentative; multiple tentatives merge into one zero-initialized object; `-fcommon`/`-fno-common` emission policy is T01/T11, not a source-semantic difference | Fail(Semantic) | T06/T08 |
| `v3.decl.extern` | `extern` declaration creates no object; a later definition creates one | Fail(Semantic) | T06/T09 |
| `v3.decl.static-local` | Static local has static storage, no linkage, one object shared across activations | Fail(Semantic) | T06/T09 |
| `v3.decl.static-function` | `static` function has internal linkage, one definition, and is callable within the TU | Fail(Semantic) | T06/T07 |
| `v3.decl.param-scope` | Parameters are in Function scope and visible in the body; duplicate parameter names rejected | Fail(Semantic) | T06/T07 |
| `v3.decl.visibility` | Point of declaration uses the complete declarator; `int n[n]` outer vs `int n = n` inner behavior preserved | Fail(Semantic) | T06 |
| `v3.decl.main` | V3 `main` is `int(void)`, external linkage; other `main` shapes/static main Unsupported | Unsupported | T06/T07 |
| `v3.decl.no-implicit` | No implicit function declaration; callee must be declared before use | Fail(Semantic) | T07 |

### 6.2 Expressions and conversions

| Rule ID | Requirement | Failure class | Owner |
|---|---|---|---|
| `v3.expr.lvalue-conversion` | A value use of an lvalue gets exactly one `LvalueToRvalue`; exceptions `&`, `sizeof`, `_Alignof`, `++/--`, assignment LHS; array/function designators use decay instead; member base not in V3 | Fail(Semantic) | T07 |
| `v3.expr.array-decay` | Array lvalue in a value use gets exactly one `ArrayToPointer`; no element read; `&array`/`sizeof` do not decay | Fail(Semantic) | T07 |
| `v3.expr.function-decay` | Direct callee does not decay; a function designator used as any other value requires a future function-pointer slice and is `Unsupported` in V3-A | Unsupported | T07 |
| `v3.expr.address-of` | Operand is an lvalue or function designator; result points to operand type; `&*p` emits no load; bitfield/register unsupported | Fail(Semantic) / Unsupported | T07 |
| `v3.expr.deref` | Operand is pointer to complete object/function; `void*` dereference/function dereference unsupported; result lvalue | Fail(Semantic) / Unsupported | T07 |
| `v3.expr.subscript` | Base array/pointer, index integer; one decay; result lvalue element | Fail(Semantic) | T07 |
| `v3.expr.pointer-arith` | Pointer + integer or integer + pointer; pointee complete; void*/function pointer arithmetic unsupported; pointer difference deferred | Fail(Semantic) / Unsupported | T07/T09 |
| `v3.expr.assignment` | V1: modifiable lvalue; V3 pointer assignment identity only; const/volatile/void* conversions unsupported | Fail(Semantic) / Unsupported | T07 (with V1) |
| `v3.expr.conditional-comma` | V1/V2: decay/lvalue rules apply to selected arms/operands; void handling explicit | Fail(Semantic) | V1/V2 + T07 |

### 6.3 Calls and returns

| Rule ID | Requirement | Failure class | Owner |
|---|---|---|---|
| `v3.call.direct` | Callee is a direct function designator/identifier; indirect/function-pointer call unsupported | Unsupported | T07/T09 |
| `v3.call.declared` | Callee declared before use; no implicit declaration; prototype required in V3 | Fail(Semantic) | T07 |
| `v3.call.arg-count` | Argument count exactly matches the prototype; variadic/old-style calls unsupported | Fail(Semantic) / Unsupported | T07 |
| `v3.call.arg-conversion` | Each argument assignment-converted to the adjusted parameter type; exact V3 int/pointer identity; conversions recorded | Fail(Semantic) | T07 |
| `v3.call.eval-order` | Arguments are checked/lowered in one chosen stable source order; side-effect-unspecified programs are not given a fabricated defined result | Fail(Semantic) for illegal; no UB claim | T07/T09 |
| `v3.call.result` | Result type is the function result; void call only in a void/discard context | Fail(Semantic) | T07 |
| `v3.call.effects` | `CALL`, `READ_MEMORY`, `WRITE_MEMORY`, `UNKNOWN_MEMORY` conservatively; no reordering across calls | Fail(Semantic) if inconsistent | T07/T11 |
| `v3.return.conversion` | Return expression assignment-converted to the function result; int/pointer identity in V3; no aggregate | Fail(Semantic) | T07 |
| `v3.return.void` | Void function returns no expression; non-void return requires an expression; `return;` in non-void rejected | Fail(Semantic) | T07 |
| `v3.return.explicit` | V3 non-void functions require an explicit return; `main` falloff `0` may be a separate explicit rule/Unsupported, never synthesized for other functions | Unsupported/Fail | T07/T09 |

### 6.4 Storage, initialization, and memory

| Rule ID | Requirement | Failure class | Owner |
|---|---|---|---|
| `v3.memory.global` | Global object has static storage, external/internal linkage, one object per definition/tentative | Fail(Semantic) | T06/T09 |
| `v3.memory.local` | Automatic local exists per activation; address-taken locals are frame-relative in Part B | Fail(Semantic) | T09/T11 |
| `v3.memory.parameter` | Parameter object is per activation, indexed in signature order, initialized from its ABI location in Part B | Fail(Semantic) / ProbeRequired | T09/T11 |
| `v3.memory.static-local` | Static local is shared across activations; constant initialization only in V3; non-constant init Unsupported | Unsupported | T08/T09 |
| `v3.memory.temporary` | V3 scalar call results use `ValueId`; no compiler `MemoryObject` temporary is required | Fail(Semantic) if shape unsupported | T09 |
| `v3.init.static` | Static initializer is a constant expression; `Zero`/`Integer` required; `Address` is Part B relocation | Fail(Semantic) / Unsupported | T08/T11 |
| `v3.init.local` | Local initializer is V1 runtime stores; no static `InitRecord`; side effects/order preserved | Fail(Semantic) | V1/T09 |
| `v3.effects.conservative` | Calls/unknown pointers are conservative; no volatile/atomic; no alias-based reordering | Fail(Semantic) | T07/T09/T11 |
| `v3.alias.facts` | `AliasClass` is a verification/future-optimization fact, not a no-alias proof | Fail(Semantic) if misused | T09/T13 |

### 6.5 Recursion and compiler task depth

| Rule ID | Requirement | Failure class | Owner |
|---|---|---|---|
| `v3.recursion.allowed` | Direct/indirect C recursion is legal; call-graph cycles are not type errors | Fail(Semantic) only for non-recursive legality | T07/T09 |
| `v3.recursion.runtime-frames` | Each activation has its own automatic objects/parameters; static locals are shared; no per-call-site static slot | Fail(Semantic) / Part B | T09/T11 |
| `v3.recursion.no-compiler-recursion` | Compiler does not recurse into callee bodies to check a call; call graph worklists are explicit and bounded | Protocol/DEFECT | T09/T13 |
| `v3.task-depth` | Parser/lower frames are bounded by `limits.max_task_depth`; excess is a typed diagnostic before child enqueue | Fail(Task/Semantic) | T05/T09 |
| `v3.task-budget` | `max_tasks_total`, `max_ticks`, `max_task_progress` remain the bound; no idle spin | Fail(Task) / CommitError | T02/T09 |

### 6.6 ABI gate and target rules

| Rule ID | Requirement | Failure class | Owner |
|---|---|---|---|
| `v3.abi.part-a-only` | Part A contains no concrete ABI values, sizes, offsets, or registers | Fail(Protocol) if a chip reads them | T11/T01 |
| `v3.abi.require-verified` | Any consumable ABI plan requires `TargetSpec::require_verified()` and a matching attestation hash | `ProbeRequired`/Config | T11/T01 |
| `v3.abi.symbolic-not-codegen` | `AbiPlan::Symbolic` cannot reach machine records or assembly | Fail(Target) | T11/T13 |
| `v3.abi.classes` | Only Integer/Pointer/Void; FP/aggregate/HFA/variadic/indirect unsupported | Unsupported | T11 |
| `v3.abi.locations` | GP ordinal/stack byte offset/result position derived from `AbiSpec`; bounds and alignment checked | Fail(Target) | T11/T13 |
| `v3.abi.no-darwin` | No Mach-O/Darwin symbol/stack/variadic model; no host ABI | Fail(Target) | T11/T13 |
| `v3.abi.plausibility` | VF11 rejects zero/oversized counts, unaligned stack, missing locations, inconsistent classes | Fail(Target) | T13 |
| `v3.abi.fail-closed` | Unverified/missing/invalid target yields a typed failure, never a fabricated plan/assembly/exit | Fail/ProbeRequired | T11/T01 |

### 6.7 Error and unsupported taxonomy

| Situation | Outcome |
|---|---|
| Malformed/unknown record reference, dangling ID, wrong family | `CommitError` (pre-mutation) / `Fail(Protocol)` |
| Missing input record or unchecked node offered to T09 | `Fail(Task/Semantic)` with task id; never fabricated facts |
| C constraint violation (type/argument/return/storage) | `Fail(Semantic)` with a stable rule id |
| Implemented-but-out-of-V3 feature (qualifiers, floats, aggregates, variadic, VLA, K&R) | `Fail(Unsupported)`; never a no-op or a guessed default |
| Part B request while target unverified | `ProbeRequired`/`ConfigError::TargetUnverified`; no partial artifact |
| Ambiguous handoff (e.g. two committed signatures where one is required) | `DEFECT` with minimal reproduction; do not choose silently |
| Unsupported shell/status | `UNSUPPORTED_SHELL`, never PASS or semantic completion |
| Timeout/budget | typed failure/cancellation; original instance remains in the denominator |

Exact numeric diagnostic codes and rule hashes are T01/T13 freeze items and are not assigned here.

---

## 7. Determinism, Snapshot, and Replay Impact

### 7.1 Deterministic allocation and ordering

| Record/fact | Required order |
|---|---|
| Types | Declaration/type-construction order; canonical reuse is the lowest matching committed `TypeId`, no hidden cache |
| Symbols/scopes/events | Declaration/scope event order; scope identity is the owner lexical node, not `(parent, kind)` |
| AST nodes | Committed pre-order; parent/children reciprocal; token ranges committed |
| `SemRecord` | One per checked node, appended in T07 checking order (children before parent; call args left-to-right; return operand before return) |
| `ConversionRecord` | Append when the conversion is checked; `(node, role)` unique; argument roles in source order |
| `MemoryObject` | Globals in TU declaration/SymbolId order, then functions in TU declaration order, then parameters by index, then locals in declaration AST order |
| `FunctionRecord` | Function definitions in TU declaration order; parameters in signature order |
| `InstructionRecord` | Block order by `InstructionId`; `Param` entry first, then body in source evaluation order; `Return` last |
| `LayoutRecord` | Type order; canonical lowest-ID lookup; no target-value cache |
| `InitRecord` | Static object symbol order; one per object |
| `AbiPlanRecord` | One per distinct signature, created in deterministic first-use/definition order; lookup by signature, lowest ID; locations in parameter order |
| Diagnostics | Commit ordinal order; no hash-map iteration |

### 7.2 Snapshot and contract hash

- Every new record family and every optional field must be encoded and round-tripped in `compiler/src/snapshot.rs`; missing encoding is a contract defect, not a silent default.
- `RecordFamily` ordinals and `RecordRef` wire tags are separate inventories. T01 assigns numeric values; this report assigns none. The snapshot encoder must not derive one from the other.
- The V3 seed (`StoreSchema` fields, record-field lists, enum-name lists, rule IDs) participates in the contract hash. A post-seed runtime `StoreSchema::declare()` alone does not freeze V3. Update `contract.rs`, `CONTRACT_VERSION`, and `COMPILER_SFL_MANIFEST.md` atomically at the integrator freeze.
- The contract hash covers normative shapes and rule identifiers, not chip source semantics. It must not include probe-specific numbers, source paths, timestamps, host pointers, or job-specific IDs.
- Job snapshots include concrete `LayoutRecord.size/align`, `AbiPlan.locations/stack_align/report_hash`, and `InitRecord` values. Those are job data, not contract constants.
- Existing `/39` hashes are preserved as history; V3 uses a new integrator-assigned artifact version. Do not overwrite `/39`.

### 7.3 Replay

- Identical initial semantic state, configuration, and pins must produce identical records, IDs, conversion order, memory-object order, ABI plan order, and diagnostics.
- `TargetSpec` is part of configuration; a verified target is a frozen job input, not a host ABI lookup. No chip reads the filesystem, clock, environment, RNG, or process state.
- VF13 compares the canonical semantic projection (new records included) and the scheduler snapshot separately; scheduler-only registers are not mixed into the semantic projection.
- `AbiPlan`'s `report_hash` is stable for a fixed attestation; a new probe/report produces a new job/config snapshot, not a silent in-place mutation.
- No host pointer, source filename, absolute path, build timestamp, or hash-map iteration may influence symbol labels, IDs, or output order.

### 7.4 Forbidden nondeterminism

- Hash-map iteration in record allocation or output.
- Source filename/path in static-local/internal labels or assembly.
- Host `size_of`, `usize`, pointer width, `long double`, or `wchar_t` as a target model substitute.
- Runtime `TargetSpec` lookup in a language chip; target values are frozen pins.
- Reusing an ID after tombstone/deletion.
- Any optimization assumption based on `AliasClass` without a separate accepted analysis.

---

## 8. Ordered Freeze and Implementation Work Orders

### 8.1 Freeze sequence (serial; integrator-owned)

All freeze work orders pin the current `/39` baseline and deliver a single new integrator-assigned artifact. They do not implement language chips.

| Order | Work order | Owner | Exact paths | Deliverable | Exit gate |
|---|---|---|---|---|---|
| F0 | `V3-F0-RECORD-SHAPES` | T01 integrator; reviewers T05/T06/T07/T08/T09/T11/T13 | `compiler/src/ids.rs`, `compiler/src/bus.rs`, `compiler/src/task.rs` (only if `StoreId`/type fields), `compiler/src/records.rs`, `compiler/src/commit.rs`, `compiler/tests/v3_freeze.rs` | Exact record fields/enums/ID families; `G1DraftBody`/commit materialization; predicted-ref rules; compile | No duplicate writers; every new field has an owner; M1 tests updated or explicitly preserved |
| F1 | `V3-F1-TASK-STAGE-ALLOWLIST` | T01 integrator; T02 reviewer | `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/tests/v3_registry.rs` | Task-kind names/local codes (T01-assigned), stages 3/4/5/6 (existing verifier) and 7/8 (target), schema fields, allowlist rows | Registry uniqueness; stage/layer agreement; no duplicate kind; no unauthorized writer |
| F2 | `V3-F2-SNAPSHOT-HASH` | T01 integrator | `compiler/src/snapshot.rs`, `compiler/src/contract.rs`, `compiler/contracts/CONTRACT_VERSION`, `compiler/contracts/COMPILER_SFL_MANIFEST.md`, `compiler/tests/freeze.rs` | Encoders/decoders, field lists, rule IDs, new artifact version/hash | Snapshot round-trip; hash/version consistency; `/39` preserved; no probe values in hash |
| F3 | `V3-F3-DIAGNOSTICS-VERIFY-RULES` | T01 integrator; T13 reviewer | `compiler/src/diagnostic.rs`, `compiler/src/contract.rs`, `compiler/tests/v3_diag.rs` | V3 diagnostic/rule inventory; VF06/VF08/VF11 rule names; no numeric invention | Rule uniqueness; negative fixture names; no PASS-from-unsupported |
| F4 | `V3-F4-READINESS-SIGNOFF` | T01 integrator + owners | No source; review records only | Co-freeze signoff/record; pin artifact hash; list dependent tests | All F0–F3 complete; owners sign their fields; Part B still blocked |

F0 and F1 may be prepared together, but no implementation starts until F0–F3 are complete and F4 records the freeze. Do not use a synthetic "V3" version before the integrator publishes it.

### 8.2 Implementation work orders

The paths below are proposed ownership. A worker edits only the listed paths; `mod.rs`, shared records, Cargo, routing, and the candidate driver are integrator-owned.

| Order | Work order | Depends | Exact owned paths | Acceptance |
|---|---|---|---|---|
| I01 | `V3-I01-PARSER` | F0, F1 | `compiler/src/chips/parse/pa_v3_decl.rs`, `pa_v3_params.rs`, `pa_v3_expr.rs`, `compiler/tests/v3_parse.rs` | Canonical fixture parses to V3 nodes; prototype/definition/params; global/local declarations; pointer/array declarators; call/subscript/address/deref; negative/malformed; `max_task_depth`; no `mod.rs` edit |
| I02 | `V3-I02-TYPES-SYMBOLS-SCOPES` | F0, F1, I01 synthetic nodes | `compiler/src/chips/types/ty_derived.rs`, `ty_function_params.rs`, `compiler/src/chips/symbols/ty_object_decl.rs`, `ty_function_scope.rs`, `compiler/tests/v3_ty.rs` | Pointer/array/function types; canonical reuse; Function scope; parameters; definition state; lookup; compatibility; M1 regression |
| I03 | `V3-I03-SEMANTICS-CONVERSIONS` | F0, F1, I02 | `compiler/src/chips/semantic/se_name.rs`, `se_call.rs`, `se_memory_expr.rs`, `se_conversion.rs`, `se_global_decl.rs`, `compiler/tests/v3_se.rs` | Identifier binding; lvalue conversion/decay; call/return/global; ConversionRecord; effects; negative cases; T06 lookup/type children |
| I04 | `V3-I04-LAYOUT-STATIC-INIT` | F0, F1, I02 | `compiler/src/chips/constant_layout_init/cl_layout.rs`, `cl_static_init.rs`, `compiler/tests/v3_layout_init.rs` | Symbolic layout Part A (no numbers); Part B `TargetSpec`; scalar/pointer/array; `init.static_zero`/`init.static_scalar`; `Address` explicitly gated; overflow/diagnostic |
| I05 | `V3-I05-IR-MEMORY-CALLS` | F0, F1, I03, I04 | `compiler/src/chips/ir_lower/ir_global_object.rs`, `ir_memory.rs`, `ir_call.rs`, `ir_function_v3.rs`, `compiler/tests/v3_ir.rs` | Function/Block/Values/Instructions/MemoryObjects; `Param`/`Call`/`Load`/`Store`/`GEP`/`Convert`; predicted refs; return terminator; no target values |
| I06a | `V3-I06a-ABI-PLAN-SHAPE` | F0, F1, I05 | `compiler/src/chips/target/aarch64/cg_abi_plan.rs`, `compiler/tests/v3_abi_plan.rs` | Symbolic classes only if integrator approves a Part A plan; otherwise pure shape/fixture tests; no location/width/register |
| I06b | `V3-I06b-ABI-PART-B` | H01/H07/TGT-03 probe attestation; I05/I06a | `compiler/src/chips/target/aarch64/cg_location.rs`, `cg_call.rs`, `cg_frame.rs`, `cg_global_data.rs`, `cg_emit.rs`, `compiler/tests/v3_abi_probe.rs` | `ProbeVerified` plans; argument/return/stack locations; call/frame/global data/assembly; mixed-link; **BLOCKED_PROBE** |
| I07 | `V3-I07-VERIFIERS` | F0–F3; each producer | `compiler/src/chips/verify/vf_invariant.rs` (extend), `vf_store.rs` (extend), `vf_ir.rs`, `vf_abi.rs`, `compiler/tests/v3_verify.rs` | VF06/VF08/VF11/VF01/VF05 positive/negative; no source writes; no target claim |
| I08 | `V3-I08-INTEGRATION` | I01–I07 | `compiler/src/chips/mod.rs`, `compiler/src/chips/parse/mod.rs`, `compiler/src/chips/types/mod.rs`, `compiler/src/chips/symbols/mod.rs`, `compiler/src/chips/semantic/mod.rs`, `compiler/src/chips/constant_layout_init/mod.rs`, `compiler/src/chips/ir_lower/mod.rs`, `compiler/src/chips/target/aarch64/mod.rs`, `compiler/src/routing.rs`, `compiler/src/bin/candidate.rs`, `compiler/tests/v3_integration.rs` | Workers registered/routed; canonical Part A fixture reaches IR + verification; snapshot/trace deterministic; `-S/-c` remain fail-closed |
| I09 | `V3-I09-INDEPENDENT-ACCEPTANCE` | I08; A-TST-03 | `compiler/tests/v3_acceptance.rs` only | Independent expected values; boundary/negative/replay; Part B tests marked blocked; no implementation/self-verification |
| I10 | `V3-I10-PART-B-CLOSED-LOOP` | I06b; H01/H07/TGT-03; H05/TGT-07 | `compiler/src/chips/target/aarch64/cg_location.rs`, `cg_call.rs`, `cg_frame.rs`, `cg_global_data.rs`, `cg_emit.rs`; `compiler/src/bin/candidate.rs`; `tools/torture/runner/**` (A09 TGT-07; not created here) | Assembly/link/run canonical exit `8`; recursion exit `6`; ABI mixed-link; no Darwin; **BLOCKED_PROBE** |

### 8.3 Implementation order and parallelism

```text
F0 -> F1 -> F2 -> F3 -> F4
          |        |
          +--> I01 +--> I02 -> I03 -> I05 -> I06a -> I07 -> I08 -> I09
                         |        |
                         +--> I04 +
                                        I06b (after probe) -> I10
```

- I01/I02 may run in parallel after F1 using synthetic tokens/nodes. I01 additionally needs the V1 co-freeze for `Declaration`/`Initializer`/`Assignment`/`ExpressionStatement`; if that is not frozen, I01 is blocked rather than inventing private declaration nodes.
- I03 must not start before the T06 type/symbol surface is frozen and a synthetic T06 fixture exists.
- I04 may run in parallel with I03 once Types are frozen; I05 is the first consumer that needs both.
- I06a is shape-level only and must not create a codegen plan; I06b is blocked.
- I07 can be developed against synthetic records, but real-chain acceptance waits for each producer.
- I08 is integrator-only; it is not a place to hide language rules.
- I09 expected values are independent (specification/hand calculation), not computed by the implementation under test.
- I10 does not start until the probe/attestation and H05 isolation exist.

### 8.4 Readiness checklist for each implementation order

A work order is READY only when all are true:

1. Exact `/next` artifact version/hash and frozen fields/tasks/results/errors are published.
2. Single owner, exact allowed paths, forbidden shared paths, test entry points, producers, and permissions are explicit.
3. Input is a real producer or a valid synthetic fixture; the acceptance level (independent/integration/closed-loop) is explicit.
4. Task guard, stage, category, progress/wait/resume/complete/fail/cancel behavior are testable.
5. Target/corpus gates are satisfied, or the work is marked **BLOCKED_PROBE** and no shell delivery is counted as success.
6. Verification commands exist and apply to the package; resource budgets/timeouts are explicit.
7. The worker is zero-field `RestrictedChip`, reads only declared fields, writes only declared fields, and does not call another chip or access I/O.
8. Tests cover normal, boundary, invalid, unsupported/recovery, replay, non-matching task, and unauthorized-field-unchanged cases.

### 8.5 Exact verification commands to use when the work orders are implemented

These are future commands, not commands run for this report. Use the package that actually changes; do not declare the whole repository passing from one package.

```sh
# Compiler package
cargo fmt --manifest-path compiler/Cargo.toml --all -- --check
cargo clippy --manifest-path compiler/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path compiler/Cargo.toml --test v3_freeze
cargo test --manifest-path compiler/Cargo.toml --test v3_parse
cargo test --manifest-path compiler/Cargo.toml --test v3_ty
cargo test --manifest-path compiler/Cargo.toml --test v3_se
cargo test --manifest-path compiler/Cargo.toml --test v3_layout_init
cargo test --manifest-path compiler/Cargo.toml --test v3_ir
cargo test --manifest-path compiler/Cargo.toml --test v3_verify
cargo test --manifest-path compiler/Cargo.toml --test v3_integration
# Part B only after probe/attestation:
cargo test --manifest-path compiler/Cargo.toml --test v3_abi_plan
cargo test --manifest-path compiler/Cargo.toml --test v3_abi_probe
```

The current dirty workspace already fails the compiler Clippy gate per A13 due to uncommitted parse drafts; a V3 work order must not claim a green baseline from the dirty workspace and must not overwrite those drafts.

---

## 9. Open Questions and Rejected Alternatives

### 9.1 Open questions

| ID | Question | Owner | Blocks | Recommended disposition |
|---|---|---|---|---|
| Q-01 | Is `SymbolRecord.decl` kept as the declaration/declarator node with a new `ident` field, or redefined as the `Identifier` leaf? | T06 + T01 | V3 symbol identity/lookup | Append `ident`; preserve `/39` `decl`; a redefinition is a larger migration |
| Q-02 | Is `DefinitionState` a `SymbolRecord` field or a separate definition record? | T06 + T01 | Globals/functions | Append field; fewer families |
| Q-03 | Is `ConversionRecord` a separate family or a vector on `SemRecord`? | T07 + T01 | Conversions/verification | Separate family; task results can use `Record(Conversion)` and one-per-`(node,role)` is explicit |
| Q-04 | Do T06 TY23/TY24/TY27/TY28 produce conversion facts, or only T07? | T06 + T07 + T01 | Single writer | T07 sole writer; T06 type constructors/helpers only |
| Q-05 | What are the exact V3 `EffectMask` bit assignments and the M1 zero-only scope? | T07 + T01 | Effects/verification | Freeze bit names/hash; add a V3 rule; keep M1 rule M1-scoped |
| Q-06 | Is `AbiPlanRecord` keyed by signature or function? | A09 + T11 + T01 | External calls/ABI | Signature key + optional `FunctionId`; A09's `TargetFunctionRecord` links definitions |
| Q-07 | What probe evidence establishes return location, stack argument order, callee-save set, and relocation forms? | H01 + H07 + T01 | Part B args/returns/globals/recursion | Extend probe/classification; **BLOCKED_PROBE** until evidence exists |
| Q-08 | Is PIE/PIC enabled for V3 global data, and which relocation forms are allowed? | T11 + H01 + T00 | Part B globals/`&g` | Freeze link mode/relocation options; do not infer from macOS or host default |
| Q-09 | Does `ConstRecord` gain `ty`, or do `InitRecord.ty`/`ValueRecord.ty` carry the type fact? | T08 + T01; A05 G4 | Static init/IR immediate | Prefer no `ConstRecord` change; type from `InitRecord`/`ValueRecord`; decide before freeze |
| Q-10 | Are `LayoutRecord` sizes/aligns absent (`None`) in Part A or represented symbolically? | T08 + T01 | Part A layout tests | `None` + `Symbolic`; no fabricated numbers |
| Q-11 | Should `FunctionRecord.parameters` be `Vec<SymbolId>` or a separate parameter-record family? | T09 + T06 + T01 | Definition lowering | `Vec<SymbolId>`; parameter symbols already exist |
| Q-12 | Are the four optional `InstructionRecord` refs the final shape, or should an op-specific aux enum replace them? | T09 + T01 | IR freeze | Optional refs are append-only/minimal; an aux enum is a larger rewrite |
| Q-13 | Is `MemoryObjectRecord` owned by T09, T08, or T06? | T09 + T08 + T06 + T01 | Single writer | T09; T08 owns layout/init; T06 owns symbols |
| Q-14 | Are non-constant static-local initializers supported with a runtime guard? | T08 + T11 | Static locals | V3 constant-only; non-constant `Unsupported` |
| Q-15 | Is `main` fallthrough `return 0` supported in V3? | T07 + T09 | Return/ABI | Explicit return only in V3-A; a separate `main` falloff rule can be added with a test |
| Q-16 | Are qualifiers/volatile/atomic part of V3? | T07 + T01 | Type/effect model | Out of V3; explicit `Unsupported`, never dropped |
| Q-17 | Are multidimensional arrays, pointer-to-array, and function pointers in V3 or boundary-only? | T05 + T06 + T07 + T09 | Declarator/ABI complexity | 1D arrays and object pointers in V3-A; other shapes explicit boundary/Unsupported |
| Q-18 | Are `void*` and null-pointer-constant conversions supported? | T07 + T06 | Assignment/args/return | Defer; explicit `Unsupported`; do not treat as int/pointer identity |
| Q-19 | Is there a runtime call-depth limit? | T09 + T11 + T01 | Recursion | No artificial limit; stack overflow is target runtime behavior; a guard would be a separate contract |
| Q-20 | Which V3 diagnostic/rule IDs and numeric codes are frozen? | T01 + T13 | Error taxonomy | Freeze names/codes after shapes; no stringly diagnostics |
| Q-21 | Do stages 7/8 and routing require new `stage_queue_bound` entries or only existing 11 slots? | T01 + T02 | Schedule | Use existing `STAGE_COUNT=11`; assign target rows to 7/8; do not renumber 0–6 |
| Q-22 | Is the contract hash independent of attested target values? | T01 + T11 | Snapshot/hash | Contract hash covers shapes/rules; job snapshot carries target values/plans |
| Q-23 | May a `Symbolic` `AbiPlan` exist in Part A? | T11 + T01 | Part A ABI tests | Shape-only; no machine consumption; a Part A producer requires separate approval |
| Q-24 | How are internal/static-local labels made unique and deterministic? | T11 + T01 | Assembly/link | Derive from `SymbolId`, not source path/filename; no host path |
| Q-25 | Is V3 single-TU only for Part A? | T01 + T09 | Linking/ABI | Yes; multi-TU/external linking is Part B/Host work |
| Q-26 | Which legal argument evaluation order does V3 choose? | T07 + T09 | Determinism/UB | Choose one stable source order; do not give unsequenced UB a fabricated defined result |

### 9.2 Rejected alternatives

| ID | Alternative | Rejected because |
|---|---|---|
| R-01 | Encode pointers/arrays as tagged `Int` ranks | Loses type safety, pointee/element semantics, and verifier completeness |
| R-02 | Put all new facts as optional fields on `SemRecord` | Creates an opaque union, weakens one-per-node/per-role checks, and hides ownership |
| R-03 | Add new `ResultValue` scalar/tuple variants | Expands the frozen result protocol unnecessarily; records + `Ack`/`Fail` suffice |
| R-04 | Hard-code AAPCS64 register numbers, stack alignment, return register, or sizes now | Violates T01 §6 and the probe gate; would fabricate unverified values |
| R-05 | Use Rust `usize`/`size_of`/host ABI as the target model | Explicitly prohibited; would miscompile AArch64 Linux programs on a macOS host |
| R-06 | Give `Layout`/`Init` to T11 privately | Duplicates T08 ownership and hides target layout from verification |
| R-07 | One fused "V3 compiler" chip | Violates single-responsibility and hides rules; not independently testable |
| R-08 | Recursively lower/check callee bodies at each call to implement recursion | Unbounded compiler recursion; C recursion is a runtime call-graph cycle |
| R-09 | Reintroduce `FunctionContextId` | Already rejected by rev 42; `FunctionId`/`SymbolId` suffice |
| R-10 | Overload `InstructionRecord.immediate` for object/callee/conversion refs | Type confusion; separate typed optional refs are safer |
| R-11 | Treat `void*` as `int*` or pointer as integer | Wrong C semantics and ABI; explicit unsupported is safer |
| R-12 | Use one static frame slot per function for recursion | Wrong for multiple activations and static locals |
| R-13 | Derive static-local labels from source filenames/paths | Nondeterministic, leaks paths, and can collide |
| R-14 | Defer global variables entirely to T08 without T09 address objects | T09 still needs `ObjectAddress`/load/store and frame/relocation consumers |
| R-15 | Treat VF12 as V3 execution evidence | VF12 is an M1 symbolic shape/identity model; it does not execute target code or memory |
| R-16 | Count `Unsupported` shells or `Ack`-only shapes as V3 completion | Prohibited by T00/AGENTS; unsupported is an explicit gap, not PASS |
| R-17 | Assume AAPCS64 prose is evidence without H07 classification | The probe intentionally leaves ABI fields unresolved; evidence, not expectation, is required |
| R-18 | Add a new V3 stage and renumber existing stages | Unnecessary; `STAGE_COUNT=11` has unused ordinals 7/8; renumbering would disturb `/39` routing/tests |

---

## 10. Evidence, Non-Claims, and Freeze Boundary

### 10.1 What this report establishes

- A precise list of the shared records, task/result shapes, ownership, and rules needed for V3, separated into Part A and Part B.
- The exact `/39` facts that conflict with V3, with recommended append-only resolutions and the owner/T01 decision required.
- A freeze sequence and implementation work orders with exact proposed paths and acceptance gates.
- The probe-gated boundary: no concrete ABI value is usable until H01/H07/T01 attestation.

### 10.2 What this report does not establish

- No V3 chip, parser, type, semantic, memory, IR, ABI, or verification implementation.
- No compiler/test/probe/assembler/linker/runner/torture execution or result.
- No accepted contract, `/next` version/hash, tag/ordinal/local-code assignment, or diagnostic code.
- No verified `aarch64-unknown-linux-gnu` ABI value, layout, stack alignment, register count, return location, relocation form, or target feature.
- No M1/V3 acceptance, no closed loop, and no GCC pass rate.

### 10.3 Frozen/probe-verified separation

| Category | Status |
|---|---|
| `/39` foundation and M1 slice shapes | **[FACT /39]** frozen and cited; not changed by this report |
| V3 record/task/rule proposals | **[PROPOSAL]** require T01 + owner co-freeze |
| Concrete ABI/scalar/layout values | **[UNVERIFIED]**; not used; Part B waits for H01 → H07 → T01 attestation |
| Peer reports A05/A09/A13 | **[PROPOSAL/AUDIT]** consumed as input, not accepted contracts |
| Tests/probe/execution | **[NOT RUN]**; no claim |

### 10.4 Freeze boundary and next step

The next action is not implementation. The T01 integrator must:

1. Review and deduplicate the record/task proposals in §4 and the conflicts in §3.2.
2. Decide Q-01 through Q-26 where an owner choice is required, especially the symbol identity, conversion producer, `ConstRecord.ty`, ABI-plan key, and probe evidence gaps.
3. Publish F0–F4 in one integrator-assigned `/next` artifact with exact fields, tags, ordinals, task local codes, schema, allowlist, snapshot, and hash.
4. Only then dispatch I01–I09 with exact paths and tests; keep I06b/I10 **BLOCKED_PROBE** until the target attestation and H05 isolation exist.

Any work that edits `compiler/src/bus.rs`, `ids.rs`, `task.rs`, `manifest.rs`, `records.rs`, `commit.rs`, `snapshot.rs`, `contract.rs`, a `mod.rs`, Cargo, routing, the candidate driver, or an existing `/39` test without an accepted change request is outside this report's authorization and must be returned as a contract change request.
