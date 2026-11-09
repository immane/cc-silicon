# A-PTS-01: V1 Contract Requirements — Integer Expressions, Locals, Declarations, Assignment, and Conversions

| Field | Value |
|---|---|
| Task | A-PTS-01 (Stage A, Batch A1) |
| Status | **PROPOSAL / CONTRACT-REQUIREMENT PACKAGE — NOT A FREEZE, NOT AN IMPLEMENTATION, NOT ACCEPTANCE** |
| Date | 2026-11-07 |
| Baseline | `t01-c01-c06/39` (`compiler/contracts/CONTRACT_VERSION`), hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` |
| Scope | Compiler application only; root framework unchanged |
| This report does not provide | Accepted schema, implemented chips, executed tests, target probe, GCC torture result, or a pass rate |
| Write scope used | This file only. Source, tests, contracts, `mod.rs`, Cargo files, and shared indexes were read-only. No commit or push. |

---

## 1. Goal and Scope

### 1.1 Goal

Define the **smallest shared contract delta** that can support a real V1 vertical slice
through `PA → TY → SE → CL/IR → verification` for:

- integer expressions with parentheses and C precedence;
- local automatic variables;
- declarations with and without scalar initialization;
- assignment expressions and assignment/initializer/return conversions.

The package is intentionally a **requirements proposal**. It gives the T01 integrator
exact current-vs-proposed shapes, ownership, legality rules, determinism impact, and
an ordered freeze/implementation queue. It does not freeze any field and does not
authorize chip code.

### 1.2 In scope (V1.0)

| Area | V1.0 scope |
|---|---|
| Translation unit | One source, one translation unit, one definition of `int main(void)`. |
| Types | Symbolic `int` only (`TypeKind::Int { rank: Int, signed: true }`); `int(void)` for `main`; no target width or bit pattern. |
| Declarations | Block-scope `int <identifier> ;` and `int <identifier> = <expression> ;`; one declarator per declaration; declarations may be interleaved with statements. |
| Local storage | Automatic storage duration, no linkage; object identity is the declared `SymbolId`; no pointers or addresses in the C type system. |
| Expressions | Decimal integer literals from the existing T04 M1 literal shape; identifiers; parentheses; unary `+` and `-`; binary `*`, `/`, `%`, `+`, `-`; assignment `=` (right-associative). |
| Precedence | C precedence: unary > multiplicative > additive > assignment; parentheses override grouping; binary operators are left-associative; assignment is right-associative. |
| Statements | Declaration, expression statement (including assignment), and `return <expression> ;` as the last statement of the function body. |
| Conversions | Identity conversions only: `int → int` integer promotion, usual arithmetic conversion, assignment/initializer conversion, return conversion. The conversion **plan shape** is proposed for freeze and later extension; the **value semantics** of non-identity conversions are out of V1.0. |
| Constant evaluation | Existing T08 literal evaluation and additive binary fold (`Add`) of two committed decimal integer literals. No other constant operators are required. |
| Verification | VF05 AST invariants, VF06 typed-fact/conversion completeness, VF12 symbolic IR interpretation with an independently supplied expected constant, VF01 store invariants. |
| Execution | **Part A only**: symbolic IR and verification; no target code, assembler, linker, runner, or execution claim. Real execution remains target-unlocked (Part B). |

A minimal design example (not a test result, not a normative acceptance claim):

```c
int main(void) {
    int x = 2;
    int y = 3;
    x = x + y * (2 + 1);
    return x;
}
```

The V1.0 slice must be able to represent this source, type-check it, record the
local writes and conversions, lower it to explicit local load/store IR, and verify
that the returned value is `11`. This example exercises declaration, initialization,
local reads/writes, assignment, parentheses, precedence, and identity conversion.
It does not require pointers, calls, control flow, globals, arrays, or target widths.

### 1.3 Out of scope (V1.0 and, where stated, later trains)

| Excluded area | Owner / later stage |
|---|---|
| Global variables, static locals, tentative definitions, multiple TUs | V3 / T06/T12 |
| Function parameters, calls, returns from non-`main` functions, multiple functions | V3 / T05/T06/T09/T11 |
| Pointers, arrays, address-of, dereference, pointer arithmetic | V3 |
| struct/union/enum, typedef, qualifiers, storage-class specifiers, `_Atomic` | V3 / T06 |
| Floating point, character/string literals, wide characters | T04 / T08 later slices |
| `if`, loops, switch, break/continue, goto/labels, short-circuit logical AND/OR, CFG | V2 |
| Compound assignment (`+=`), `++`/`--`, comma operator, conditional `?:`, casts, `sizeof`/`alignof` | Later expression slices |
| Relational, equality, bitwise, shift, logical `!` operators | Not required by V1.0; candidate V1.1 or V2 input |
| Multiple declarators (`int x, y;`), braced/designated/aggregate initialization | T05/T08 later |
| Variadic, old-style/K&R, GNU extensions, inline asm | T05/T12 |
| Optimization (T10), target ABI/code generation/execution (T11, Part B) | Later stages |
| Non-identity conversions (`signed/unsigned`, narrowing, float, pointer) | Requires frozen target widths (Part B) or later type model |
| Taskified per-production parser, continuation materialization, general CFG parsing | Deferred; see §8.2 |

### 1.4 Status vocabulary used in this report

- **FROZEN `/39`** — present in the checked-in contract/version and source at the
  baseline. It is evidence of current shape, not of V1 capability.
- **PROPOSED** — a V1 contract requirement in this report. It is not accepted,
  not hashed, and not implemented.
- **OPEN** — a decision the integrator/owners must resolve before freeze.
- **DEFECT / CONFLICT** — a concrete inconsistency between `/39`, the candidate
  drafts, and the V1 need.
- **NOT RUN / NOT CLAIMED** — no test, probe, corpus, or execution evidence.

### 1.5 Non-claims

This report did not run the test suite, the target probe, or any compiler pipeline.
It did not modify source, tests, contracts, `mod.rs`, Cargo files, or shared indexes.
All statements about current behavior are read-only source inspection. The dispatch
ledger's test counts are external observations and are not re-verified here.

---

## 2. Baseline Facts at `/39`

### 2.1 Frozen artifact

`compiler/contracts/CONTRACT_VERSION` records:

```text
version=t01-c01-c06/39
hash=07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24
target=aarch64-unknown-linux-gnu
target_verification=unverified
```

The frozen artifact is a **shape/rule fingerprint**, not a source-code hash and not
a semantic proof (`compiler/src/contract.rs:1-18`). The version comment and hash
constant are at `compiler/src/contract.rs:38-44`; the normative rule list starts at
`compiler/src/contract.rs:51`.

### 2.2 Current shared shapes relevant to V1

| Item | Current `/39` shape | Source |
|---|---|---|
| `NodeKind` | `TranslationUnit`, `FunctionDefinition`, `Specifiers`, `Declarator`, `Compound`, `Return`, `BinaryAdd`, `IntLiteral` (8 variants) | `compiler/src/bus.rs:545-562`; hashed names `compiler/src/contract.rs:380-389` |
| `NodeRecord` | `kind`, `parent`, `children`, `first_token`, `last_token`, `name`, `literal` | `compiler/src/bus.rs:526-541`; field list `compiler/src/contract.rs:462-470` |
| `SymbolKind` | `Function`, `Object` (wire names `function`, `variable`) | `compiler/src/bus.rs:327-332`; names `compiler/src/contract.rs:404` |
| `SymbolRecord` | `name`, `scope`, `kind`, `ty`, `linkage`, `storage`, `decl` | `compiler/src/bus.rs:362-377` |
| `TypeKind` | `Void`, `Bool`, `Char`, `Int { rank, signed }`, `Function { result, params, prototype, variadic }` | `compiler/src/bus.rs:298-323` |
| `ScopeRecord` | `kind`, `parent`, `owner` | `compiler/src/bus.rs:391-399` |
| `ScopeEventRecord` | `scope`, `kind`, `at` | `compiler/src/bus.rs:413-420` |
| `ValueCategory` | `Lvalue`, `NonLvalue`, `FunctionDesignator`, `Void` | `compiler/src/bus.rs:424-433` |
| `EffectMask` | `EffectMask(u32)`; `/14` allows only `0` | `compiler/src/bus.rs:435-438`; `compiler/src/contract.rs:110` |
| `SemRecord` | `node`, `ty`, `category`, `effects`; **no `conversions`**, no constant link | `compiler/src/bus.rs:496-513`; field list `compiler/src/contract.rs:432` |
| `LiteralRecord` | `token`, `kind`, `radix`, `suffix`, `value`, `negative`, `spelling`, `candidate_type` (V1 uses integer only) | `compiler/src/bus.rs:570-588`; field list `compiler/src/contract.rs:337-346` |
| `ConstRecord` | `value: Vec<u8>` big-endian magnitude, `negative: bool`; no type/origin | `compiler/src/bus.rs:590-602`; field list `compiler/src/contract.rs:348` |
| `FunctionRecord` | `symbol`, `signature`, `entry`, `linkage` | `compiler/src/bus.rs:451-460` |
| `BlockRecord` | `function`, `ordinal` | `compiler/src/bus.rs:465-470` |
| `ValueRecord` | `ty` | `compiler/src/bus.rs:475-478` |
| `IrOp` | `Constant`, `Return` (2 variants) | `compiler/src/bus.rs:442-447`; names `compiler/src/contract.rs:435` |
| `InstructionRecord` | `op`, `block`, `operands`, `immediate: Option<ConstId>`, `result: Option<ValueId>` | `compiler/src/bus.rs:483-494`; field list `compiler/src/contract.rs:459` |
| `Payload` | `Vec<RecordRef>` only | `compiler/src/task.rs:1248-1263` |
| `ResultValue` | `Empty`, `Ack`, `Record`, `Records`, `Diagnostic` | `compiler/src/task.rs:1728-1739` |
| `Proposal` | `Enqueue`, `Complete`, `Fail`, `AwaitHost`, `StorePatch`, `AppendRecords`, `Progress`, `AwaitChildren` | `compiler/src/task.rs:1893-1960` |
| `TaskDraft` | `continuation: Option<ContinuationId>` (committed only) | `compiler/src/task.rs:1793-1810` |
| `ContinuationRecord` | 9 ordered fields; no `awaited` | `compiler/src/task.rs:1557-1576` |
| `G1DraftBody` | 18 typed draft families; **no `Continuation`** | `compiler/src/records.rs:55-96` |
| `DiagGroup` | `Protocol`, `Arena`, `Config`, `Target`, `Manifest`, `Task`, `Semantic`, `Unsupported`, `Internal`; no parse/syntax group | `compiler/src/diagnostic.rs:37-57`; names `compiler/src/contract.rs:325-335` |
| IDs / families / tags | 28 record families, 28 `RECORD_KINDS` labels, 28 `RecordRef` wire tags; no V1 additions required by this proposal | `compiler/src/ids.rs:119-178,343-400`; `compiler/src/task.rs:2185-2213` |

### 2.3 Current V1-adjacent capability (read-only inspection)

The table describes the **frozen `/39`/HEAD baseline**. The shared dirty workspace
contains concurrent, uncommitted changes (including `pa_tu.rs`); those changes are
not `/39`, are not reviewed here, and may differ from the cited lines. See §2.5.

| Phase | Current capability | Gap for V1 |
|---|---|---|
| PA | `PaTuChip` accepts exactly the 13-token M1 source and appends a fixed 9-node tree (`compiler/src/chips/parse/pa_tu.rs:154-197,337-365`). PA02/PA03/PA05/PA28/PA32 are Ack-only validators (`compiler/src/chips/parse/pa_external.rs`, `pa_specifier.rs`, `pa_declarator.rs`, `pa_block.rs`). PA14 is Ack-only `main(void);` finish (`pa_pod.rs`). | No general AST production, no declarations with initializers, no identifiers/locals, no assignment, no general precedence parser, no expression statement. |
| TY | Canonical `int`/`int(void)` producers and scope/symbol/type lookup exist (`ty_types.rs`, `ty_scope.rs`, `ty_symbol.rs`). `SYMBOL_DECLARE` rejects non-function types and file scope only (`ty_symbol.rs:120-135`). `ty_conv.rs` has identity-only promote/common-type/return-convert tasks; there is **no assignment-conversion task**. | Declarations of objects in block scope; lookup with a real query point; assignment conversion task. |
| SE | `SE02` literal, `SE07` additive binary, `SE21` return, `SE29` function definition exist for the M1 shape (`se_literal.rs`, `se_binary.rs`, `se_return.rs`, `se_function.rs`). `SemRecord` has no conversions or constant link. `EffectMask(0)` only. | Identifier/lvalue semantics, locals, assignment, unary, general binary, declaration checking, local-write effect, conversion plans, constant link. |
| CL | `FoldChip` evaluates committed literal/binary `Add` and appends one `ConstRecord` (`fold.rs`). | Multiple constants per program need a deterministic node→constant link; current M1 assumes exactly one folded constant (`ir_function.rs:223-241`). |
| IR | `IrFunctionChip` emits exactly `Constant` then `Return` for one function; no locals/arithmetic/store (`ir_function.rs:307-385`). | Local load/store, arithmetic, initialization/assignment stores, statement sequencing. |
| VF | VF06 checks the fixed `{IntLiteral, BinaryAdd, Return}` set (`vf_invariant.rs:5-10`); VF12 interprets only `Constant` then `Return` (`vf_interpret.rs:5-13`); VF05 checks the fixed M1 tree (`vf_syntax.rs`). | New node kinds, conversions/effects, locals/arithmetic IR, expected-value matching. |

### 2.4 Current stage/allowlist facts

- `STAGE_ASSIGNMENT` currently has 69 rows (`compiler/src/manifest.rs:1444-1514`).
  Notably, `SEMANTIC_CONST_EVAL_LITERAL`/`BINARY` are stage **1**, while
  `CONSTANT_CONST_FOLD` is stage **2**, `SEMANTIC_*` checks are stage **4**, and
  `IR_FUNCTION` is stage **5** (lines 1505-1513). This is inconsistent with the
  documented `sem → const → ir` order and is a V1 conflict (§8).
- `STORE_OWNER_ALLOWLIST` currently has multiple writers to `sem.records` and
  `ir.*` (`compiler/src/manifest.rs:784-978`). `IR_FUNCTION_CHIP` currently owns
  `ir.functions`, `ir.blocks`, `ir.values`, and `ir.instructions` (rows 877-899).
- `TaskKindRegistry` slice constructors end at `se_function_slice()` (local 21 for
  `SEMANTIC_FUNCTION_DEF`; `compiler/src/task.rs:722-734`). `PARSE` locals 16-26
  are used; `SYMBOL_TYPE` 16-24; `SEMANTIC` 16-21; `IR_LOWER` 16.
- Current tests pin the old shapes: `compiler/tests/c12_parse.rs` (9-node tree),
  `c13_ty.rs` (symbol/scope), `c14_se.rs` (4-field `SemRecord`), `c15_ir.rs`
  (`Constant`+`Return`), `c17_vf12.rs`, `c34_parse.rs`/`c35_expr.rs`/`c36_recovery.rs`
  (Ack-only PA slices), `c39_sefunc.rs`, and `freeze.rs` (hash/version). V1 changes
  will require explicit test updates, not silent overwrites (§7.3).

### 2.5 Uncommitted candidate drafts (read-only; not part of `/39`)

The workspace contains unregistered candidate files under `compiler/src/chips/parse/`
and `compiler/src/chips/lex/`. They are user work and were not modified. They are
relevant because they claim overlapping V1 IDs and shapes:

| Candidate file | Claimed scope / IDs | Conflict note |
|---|---|---|
| `pa_cast.rs` | PA21/PA23/PA25; `PARSE` locals 27-29; `ChipId(58)` | Claims expression/cast/conditional/comma; V1 expression parser would otherwise use those locals. |
| `pa_decl_spec.rs` | PA08/PA10; `PARSE` locals 27-28; `ChipId(58)` | Claims array/abstract declarator; collides with other 27/58 claims; arrays are V3. |
| `pa_expr_ext.rs` | PA15/PA26; `PARSE` locals 27-28; `ChipId(58)` | Claims initializer/`_Generic`; V1 declaration initializer is a different production. |
| `pa_ext_dispatch.rs` | PA33/34/35/36/37; `PARSE` locals 27-31; `ChipId(58)` | Claims expression statement (PA34) and others; V1 needs expression statement but not this shell. |
| `pa_init.rs` | PA15; `PARSE` local 27; `ChipId(58)` | Duplicate initializer claim with `pa_expr_ext.rs`. |
| `pa_stmt.rs` | PA27/29/30/31; `PARSE` locals 27-30; `ChipId(58)` | Says “T01 `/5` current” (stale); conflicts with V2 control-flow scope and other local claims. |
| `pa_typedef_query.rs` | PA04; `PARSE` local 27; `ChipId(58)` | Typedef is out of V1; same ID collision. |

**Conclusion:** V1 must not claim `PARSE` locals 27-31 or `ChipId(58)` until the
integrator arbitrates these drafts. The recommendation in §3 is to avoid new PA
task kinds/chips entirely by using one V1 parser task (`PARSE_TU`) and thus avoid
the contested window. If a later V1.1 taskified parser is authorized, the integrator
must first retire/renumber the drafts.

Other uncommitted files observed during this read-only pass (for example
`compiler/tests/c41_candidate_argv.rs`, `compiler/tests/c42_wiring.rs`,
`compiler/tests/c40_pa_guard.rs`, `tools/chip-lint/tests/fixtures/`, and the
concurrent modifications to `compiler/src/bin/candidate.rs`,
`compiler/src/chips/parse/mod.rs`, `compiler/src/chips/parse/pa_tu.rs`,
`README.md`, and `compiler/README.md`) are also outside `/39`, belong to other work,
and were not modified. Because `pa_tu.rs` is the proposed V1 parser file and is
already being changed concurrently, W1 must not start until the integrator
establishes a single writer and reconciles that change. Because test numbers are
being consumed concurrently, this report uses descriptive `compiler/tests/v1_*.rs`
names; the integrator assigns final `cXX` numbers and test entry points.

---

## 3. Required Schema, Task, and Result Changes

### 3.1 Design invariants for the V1 delta

The following are proposed as **freeze constraints**, not as implementation claims:

1. **No new ID type, record family, `RecordRef` variant, `RecordFamily` ordinal, or
   wire tag.** V1 reuses `NodeId`, `TypeId`, `SymbolId`, `ScopeId`, `SemId`,
   `ConstId`, `FunctionId`, `BlockId`, `ValueId`, `InstructionId`, `LiteralId`,
   `TokenId`, `TaskId`, `ResultId`, `DiagnosticId`.
2. **No new store or store field.** All new data lives in existing fields:
   `parse.nodes`, `symbols.*`, `types.records`, `sem.records`, `constants.records`,
   `ir.*`.
3. **No new `ResultValue` variant and no new `Proposal` variant.** V1 uses
   `Record`, `Records`, `Ack`, `Fail`, and the existing task proposals.
4. **No new continuation or task-draft mechanism in V1.** The recommended parser
   is a single task, and IR sequencing uses a child-task chain. `ContinuationRecord`
   materialization and `TaskDraft.continuation: Option<ContinuationRef>` remain
   deferred.
5. **No target width, bit pattern, host integer, host pointer, or host ABI in V1
   semantics.** `int` remains symbolic; non-identity conversions and target
   execution are Part B.
6. **One producer per record; multiple authorized appenders per append field.**
   The existing `(ChipId, StoreId, field, TaskKind)` allowlist is the operative
   ownership rule (§4.1).
7. **Atomic batches only.** A parser/SE/IR task either commits its complete
   `AppendRecords` batch or nothing; no partial AST/IR is visible.

### 3.2 Master delta table (current vs proposed)

| Item | Current `/39` | Proposed V1 | Change class |
|---|---|---|---|
| `NodeKind` | 8 variants | +`Declaration`, `Identifier`, `ExpressionStatement`, `Assignment`, `BinarySub`, `BinaryMul`, `BinaryDiv`, `BinaryRem`, `UnaryPlus`, `UnaryMinus` (18 total) | enum append |
| `NodeRecord` | 7 fields | **Unchanged** | none |
| `SemRecord` | `node`, `ty`, `category`, `effects` | +`conversions: Vec<ConversionPlan>`, +`constant: Option<ConstId>`, +`symbol: Option<SymbolId>` | field append |
| `ConversionOp` | absent | `Identity` (V1 closed; future append) | new enum |
| `ConversionRole` | absent | `Return`, `Assignment` (V1 closed; future append) | new enum |
| `ConversionPlan` | absent | `op`, `role`, `from: TypeId`, `to: TypeId` | new inline value type |
| `EffectMask` | only `0` legal | bit 0 = `WritesLocal`; V1 legal masks are `0` and bit 0 | semantic extension |
| `ConstRecord` | `value`, `negative` | **Unchanged** | none |
| `TypeKind` | 5 variants | **Unchanged** (int only) | none |
| `SymbolRecord` | 7 fields | **Unchanged**; `decl` now names the `Identifier` leaf | semantic clarification |
| `ScopeRecord`/`ScopeEventRecord` | existing | **Unchanged** | none |
| `ValueCategory` | 4 variants | **Unchanged** | none |
| `FunctionRecord` | 4 fields | **Unchanged** | none |
| `BlockRecord`/`ValueRecord` | existing | **Unchanged** | none |
| `IrOp` | `Constant`, `Return` | +`LocalLoad`, `LocalStore`, `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg` (10 total) | enum append |
| `InstructionRecord` | 5 fields | +`local: Option<SymbolId>` (append at end) | field append |
| `Payload` | `Vec<RecordRef>` | **Unchanged** | none |
| `ResultValue` | 5 variants | **Unchanged** | none |
| `Proposal` | 8 variants | **Unchanged** | none |
| `TaskDraft` | `Option<ContinuationId>` | **Unchanged** | none |
| `ContinuationRecord` | 9 fields | **Unchanged / unused by V1** | none |
| `G1DraftBody` | 18 families | **Unchanged / no Continuation** | none |
| IDs / families / tags | existing | **No additions** | none |
| `DiagGroup` | 9 groups | +`Parse` (recommended; optional alternative in §8) | enum append |
| `StoreId`/`StoreSchema` | existing | **No new store/field** | none |
| `TaskKind` | existing | +7 V1 kinds (PARSE_TU is reused, not added) | enum append |
| `ChipId` | max 57 | +6 new chips (64-69 proposed); existing chips modified | registration |

**Headline:** V1 needs **zero new IDs/families/wire tags/stores/result variants**
and only the shared enum/record extensions in §3.2: `NodeKind`, `SemRecord`
(+ conversion types and links), `EffectMask`, `IrOp`/`InstructionRecord`, and the
recommended `DiagGroup::Parse`.

### 3.3 AST shape (`parse.nodes`, PA-owned)

#### 3.3.1 Proposed `NodeKind` inventory

Declaration order is the snapshot/hash order. Existing `/39` names stay unchanged;
new variants append:

```rust
pub enum NodeKind {
    // FROZEN /39 (unchanged, existing order)
    TranslationUnit,
    FunctionDefinition,
    Specifiers,
    Declarator,
    Compound,
    Return,
    BinaryAdd,
    IntLiteral,
    // V1 APPEND (proposed order)
    Declaration,
    Identifier,
    ExpressionStatement,
    Assignment,
    BinarySub,
    BinaryMul,
    BinaryDiv,
    BinaryRem,
    UnaryPlus,
    UnaryMinus,
}
```

`Specifiers`, `Compound`, `IntLiteral`, and `BinaryAdd` are retained rather than
renamed to the candidate names `TypeSpecifier`, `Block`, `IntegerConstant`, and
`BinaryExpression`. Renaming frozen variants would break `/39` snapshots/hash for
no semantic gain; the proposal is additive. If the integrator prefers the candidate
names, that is a separate renaming/migration decision (§8).

#### 3.3.2 `NodeRecord` field rules (fields unchanged)

| Field | Type | V1 rule |
|---|---|---|
| `kind` | `NodeKind` | One of §3.3.1. |
| `parent` | `Option<NodeId>` | `None` only for the TU root; every other node references a committed parent. |
| `children` | `Vec<NodeId>` | Ordered by source position; contiguous sibling ranges; empty for leaves. This field remains authoritative (no `ordinal` migration). |
| `first_token` | `TokenId` | Committed token; first token covered by the node (parentheses may be included on the enclosing node). |
| `last_token` | `TokenId` | Committed token; inclusive; `first_token.index() <= last_token.index()`. |
| `name` | `Option<NameId>` | V1: `Some` only on `Identifier`; `None` on other kinds. The existing `Declarator.name` field becomes unused in V1 trees, but remains for `/39` compatibility. |
| `literal` | `Option<LiteralId>` | V1: `Some` only on `IntLiteral`; `None` otherwise. |

No `ordinal` field is added. The T05 candidate `NodeRecord { kind, first_token,
last_token, parent, ordinal }` is rejected for V1 because `children` already
carries ordered siblings and is implemented in `/39`; adding `ordinal` would be
redundant and would force every consumer/test to migrate.

#### 3.3.3 V1 child shapes

| Node kind | `children` | Token range | Notes |
|---|---|---|---|
| `TranslationUnit` | `[FunctionDefinition]` | first TU token .. EOF | One function in V1. |
| `FunctionDefinition` | `[Specifiers, Declarator, Compound]` | first specifier .. `}` | `main` only. |
| `Specifiers` | `[]` | `int` token .. `int` token | `int` only. |
| `Declarator` | `[Identifier]` | declarator start .. declarator end | `main(void)` or object name; `(void)` included. |
| `Identifier` | `[]` | identifier token .. identifier token | `name: Some`; declaration or expression use. |
| `Compound` | declarations/statements in source order | `{` .. `}` | Block scope anchor. |
| `Declaration` | `[Specifiers, Declarator]` or `[Specifiers, Declarator, init-expr]` | first specifier .. `;` | One declarator; initializer optional. |
| `ExpressionStatement` | `[expr]` | first expr token .. `;` | Value discarded. |
| `Return` | `[expr]` | `return` .. `;` | Must be the last statement in V1. |
| `Assignment` | `[lhs, rhs]` | lhs start .. rhs end | `=` only; right-associative; lhs must be a modifiable lvalue (checked by SE). |
| `BinaryAdd`/`Sub`/`Mul`/`Div`/`Rem` | `[lhs, rhs]` | lhs start .. rhs end | Left-associative; parentheses may be included on the parent range. |
| `UnaryPlus`/`UnaryMinus` | `[operand]` | operator .. operand end | Unary plus is a semantic identity but a distinct AST node for syntax/precedence. |
| `IntLiteral` | `[]` | literal token .. literal token | `literal: Some`; committed T04 literal. |

Parentheses are **not** represented by a node. A parenthesized subexpression’s
parentheses are included in the range of its nearest enclosing non-leaf
expression/statement node; a leaf `Identifier`/`IntLiteral` keeps its own
single-token range. This preserves correct tree shape and the leaf rule
(`first_token == last_token`) while retaining source ranges for diagnostics.

#### 3.3.4 AST producer

V1 recommends **one parser task** (`PARSE_TU`) that appends the complete V1 tree in
one `AppendRecords` batch in pre-order. The parser:

- reads the committed token range and literal records;
- uses explicit stacks/counters, not Rust recursion;
- allocates `NodeId = nodes_allocated + position` in pre-order;
- fills `parent`/`children` from the temporary tree;
- rejects any source outside the V1 grammar with a typed diagnostic;
- commits all nodes atomically or nothing.

This avoids the parent-after-child allocation problem that a taskified parser would
create without a record-update or reservation protocol (§8.2). It also avoids the
contested `PARSE` local 27+ window. The parser chip becomes large; this is accepted
only as a declared V1 bootstrap fusion, not as the final parser architecture.

### 3.4 Semantic shape (`sem.records`, SE-owned)

#### 3.4.1 Proposed `SemRecord` fields

**V1 checked-node set.** A `SemRecord` is required for: every expression node
(`Identifier` used as an expression, `IntLiteral`, `UnaryPlus`, `UnaryMinus`,
`BinaryAdd`, `BinarySub`, `BinaryMul`, `BinaryDiv`, `BinaryRem`, `Assignment`),
every `Declaration`, every `Return`, and the `FunctionDefinition`. The following
are **not** checked nodes in V1 and must not have a `SemRecord`: `TranslationUnit`,
`Specifiers`, `Declarator`, `Compound`, `ExpressionStatement` (its child expression
carries the facts), and an `Identifier` used only as a declaration name. This keeps
the one-record-per-checked-node rule unambiguous and avoids inventing facts for
pure syntax wrappers.

Field order is hash/snapshot order; new fields append:

| # | Field | Type | V1 rule |
|---|---|---|---|
| 1 | `node` | `NodeId` | Exactly one `SemRecord` per checked node. |
| 2 | `ty` | `TypeId` | Checked type (`int` for V1 expressions/objects; `int(void)` for the function definition). |
| 3 | `category` | `ValueCategory` | `Lvalue` for identifier expressions; `NonLvalue` for literals/assignment/unary/binary; `Void` for declaration/return/function-definition records. |
| 4 | `effects` | `EffectMask` | `0` for pure reads/literals; bit 0 (`WritesLocal`) for assignment and initialized declaration; OR of child effects. |
| 5 | `conversions` | `Vec<ConversionPlan>` | At most one per `(node, role)` in V1; ordered by role/append order; empty means no plan. |
| 6 | `constant` | `Option<ConstId>` | `Some` only when the node is a constant expression evaluated by T08; the `ConstId` is committed and T08-owned. `None` otherwise. |
| 7 | `symbol` | `Option<SymbolId>` | `Some` for an `Identifier` expression and for a `Declaration`/`FunctionDefinition` that names an object/function; `None` for literals, unary/binary, assignment, and return. T09 uses this link for `LocalLoad`/`LocalStore` and function lookup; it must not re-resolve names. |

#### 3.4.2 Conversion types

```rust
// PROPOSED, NOT FROZEN
pub enum ConversionOp {
    Identity,   // V1 closed; future variants append (promotion/narrowing/float/pointer)
}

pub enum ConversionRole {
    Return,      // return operand -> function result type
    Assignment,  // initializer/assignment rhs -> destination type
}

pub struct ConversionPlan {
    pub op: ConversionOp,
    pub role: ConversionRole,
    pub from: TypeId,
    pub to: TypeId,
}
```

V1 records **only** assignment/initializer and return conversions. Lvalue-to-rvalue
conversion is structural in IR (`LocalLoad`) and is not recorded as a separate plan;
integer promotion and usual arithmetic conversion are identity and are checked by
TY but not recorded as plans when no representation change occurs. This is the
minimum needed to prove the conversion field works without pretending to implement
target-width conversions.

**Conflict:** the T07 candidate checklist (S5) leaves `ConversionOp`/`ConversionRole`
and the `(op, role)` matrix open; the M1 proposal enumerates a larger set
(`IntegerPromotion`, `UsualArithmetic`, `Assignment`, `SignExtend`, `ZeroExtend`,
`Truncate`, `IntToFloat`, `FloatToInt`, `PointerToInt`, `IntToPointer`, `ToBool`).
V1.0 freezes only `Identity` and two roles; the larger set remains a later append,
as proposed in `docs/tasks/T07_SEMANTIC_CHIPS.md` §5 and the M1 proposal §5.
This is not a repurposing of the lexical candidate type.

#### 3.4.3 Effect mask

```rust
// PROPOSED, NOT FROZEN
pub const EFFECT_WRITES_LOCAL: u32 = 1 << 0;
```

- V1 legal masks: `0` and `EFFECT_WRITES_LOCAL`.
- `Assignment` and `Declaration` with an initializer set bit 0.
- Pure literals, identifier reads, unary/binary arithmetic, and return statements
  have `0` unless their operand has bit 0.
- A nonzero unknown bit (bit 1 or above) is an explicit typed `Unsupported`/failure,
  never a silent pass.
- VF06 verifies the expected mask per node shape; IR verification may re-check that
  every `LocalStore` has a corresponding `WritesLocal` semantic fact.

**Conflict:** `/39` rule `sem.effect-mask-zero-only` and `SE_FUNC`’s nonzero-effect
rejection conflict with assignment. V1 must replace that rule with a new rule
(`sem.effect-mask-v1-writes-local`) and update dependent tests. The old rule ID
remains history.

### 3.5 IR shape (`ir.*`, T09-owned)

#### 3.5.1 Proposed `IrOp` and `InstructionRecord`

```rust
// PROPOSED, NOT FROZEN
pub enum IrOp {
    // FROZEN /39
    Constant,
    Return,
    // V1 APPEND
    LocalLoad,
    LocalStore,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
}

pub struct InstructionRecord {
    pub op: IrOp,
    pub block: BlockId,
    pub operands: Vec<ValueId>,
    pub immediate: Option<ConstId>,
    pub result: Option<ValueId>,
    pub local: Option<SymbolId>,   // V1 append; only LocalLoad/LocalStore
}
```

`FunctionRecord`, `BlockRecord`, and `ValueRecord` are unchanged. V1 does not add a
locals list; target allocation can collect automatic `SymbolRecord`s later. V1 does
not add a pointer type or an address-valued IR value.

#### 3.5.2 V1 op table

| Op | Operands | `immediate` | `local` | `result` | Terminator | Rule |
|---|---|---:|---|---|---|---|
| `Constant` | 0 | `Some(ConstId)` | `None` | `Some(ValueId)` | no | Materialize a committed T08 constant; never refold. |
| `LocalLoad` | 0 | `None` | `Some(SymbolId)` | `Some(ValueId)` | no | Load an automatic object; result type = object type. |
| `LocalStore` | 1 | `None` | `Some(SymbolId)` | `None` | no | Store the operand value; assignment/init expression value is the operand value. |
| `Add` | 2 | `None` | `None` | `Some(ValueId)` | no | Signed mathematical add for V1; target-width behavior Part B. |
| `Sub` | 2 | `None` | `None` | `Some(ValueId)` | no | Same. |
| `Mul` | 2 | `None` | `None` | `Some(ValueId)` | no | Same. |
| `Div` | 2 | `None` | `None` | `Some(ValueId)` | no | Signed division; zero divisor is a typed failure in verification. |
| `Rem` | 2 | `None` | `None` | `Some(ValueId)` | no | Signed remainder; zero divisor is a typed failure. |
| `Neg` | 1 | `None` | `None` | `Some(ValueId)` | no | Unary minus. |
| `Return` | 1 | `None` | `None` | `None` | **yes** | The unique terminator; must be the greatest `InstructionId` in the entry block. |

V1 has one entry block and no CFG. Every function body must end in `Return`.
Fallthrough/`return;`/multiple returns are outside V1.0 and fail explicitly.

#### 3.5.3 IR task sequencing

To preserve source order and avoid a giant lowering chip:

1. `IR_FUNCTION` creates `FunctionRecord` + entry `BlockRecord`, then enqueues
   exactly one `IR_STATEMENT` child for the first body item and awaits it.
2. `IR_STATEMENT` for one body item:
   - Declaration with initializer: enqueue `IR_EXPRESSION` for the initializer,
     await, append `LocalStore` to the declared symbol, then enqueue the next
     statement child and await it.
   - ExpressionStatement: enqueue `IR_EXPRESSION`, await, then enqueue the next
     statement child and await it.
   - Return: enqueue `IR_EXPRESSION`, await, append `Return`; it must be the last
     body item, otherwise fail `Unsupported`.
   - Declaration without initializer: enqueue the next statement directly.
3. `IR_EXPRESSION` lowers one expression subtree in one batch using explicit
   iterative post-order:
   - if the node’s `SemRecord.constant = Some(c)`: emit `Constant(c)` and stop
     (no descent, no refold);
   - `Identifier`: `LocalLoad`;
   - `Assignment`: lower RHS, emit `LocalStore` to the LHS symbol, return the RHS
     value;
   - `Binary*`: lower children, emit the matching op;
   - `UnaryPlus`: lower child, return it unchanged;
   - `UnaryMinus`: lower child, emit `Neg`;
   - return `Record(Value(root))`.
4. The statement chain keeps instruction IDs in source order because the next
   statement is enqueued only after the current statement’s expression and store/
   return instructions commit. The chain depth is bounded by `max_task_depth`; an
   over-deep function is a typed `TaskDepth`/limit failure, not a panic.

**Alternative rejected for V1:** enqueue all statements at once and let them run
independently. With quota 1, statement expression children can interleave out of
source order (`S1` enqueues `E1`; `S2` is selected before `E1`), so a later
`LocalLoad` can execute before an earlier `LocalStore`. The chain is the smallest
correct ordering mechanism without a continuation/cursor register.

### 3.6 Task kinds and payloads

#### 3.6.1 Existing kinds whose payload/result/behavior changes

| Task kind | Current `/39` payload → result | Proposed V1 payload → result | Owner chip |
|---|---|---|---|
| `PARSE_TU` (PARSE 16) | all 13 M1 token refs → `Record(Node TU)`; fixed 9-node tree | exactly `[Token first, Token last]` committed range (first ≤ last, same source, EOF included in the TU range) → `Record(Node TU)`; complete V1 AST in one batch | `PA_TU_CHIP` (7), modified |
| `SYMBOL_DECLARE` (SYMBOL 20) | `[Declarator Node, Function Type, File Scope]` → `Record(Symbol)`; function/file only | `[Identifier Node, Type, Scope]` → `Record(Symbol)`; object/block or function/file by derivation | `TY_SYMBOL_CHIP` (10), modified |
| `SYMBOL_LOOKUP` (SYMBOL 21) | `[Scope, Name]` → `Record(Symbol)` or typed miss | `[Scope, Name, Token query]` → `Record(Symbol)` or typed miss; query point = token span start | `TY_SYMBOL_CHIP` (10), modified |
| `SEMANTIC_LITERAL_EXPR` (SEMANTIC 18) | `[IntLiteral Node]` → `Record(Sem)`; no const child | same payload → `Record(Sem)`; enqueue `CONSTANT_CONST_FOLD [Literal]`, await, then append `SemRecord` with `constant: Some(c)` | `SE_LIT_CHIP` (12), modified |
| `SEMANTIC_BINARY_EXPR` (SEMANTIC 19) | `[BinaryAdd Node]` → `Record(Sem)`; literal-only operands; one Add fold | `[BinaryAdd/Sub/Mul/Div/Rem Node]` → `Record(Sem)`; non-literal operands allowed; enqueue TY promotion/common-type and the optional `CONSTANT_CONST_FOLD [Node, Literal, Literal]` in **one** `AwaitChildren` batch; `constant: Some(c)` only for that fold | `SE_BIN_CHIP` (13), modified |
| `SEMANTIC_RETURN_STMT` (SEMANTIC 20) | `[Return Node]` → `Record(Sem)`; BinaryAdd operand only | `[Return Node]` → `Record(Sem)`; any V1 expression operand; enqueue `SYMBOL_RETURN_CONVERT [expr ty, return ty]`, await; record Return conversion | `SE_RET_CHIP` (14), modified |
| `SEMANTIC_FUNCTION_DEF` (SEMANTIC 21) | `[FunctionDefinition Node]` → `Record(Sem)`; direct Return child | `[FunctionDefinition Node]` → `Record(Sem)`; body is a `Compound` with declarations/statements; require all body SemRecords; signature/effects | `SE_FUNC_CHIP` (57), modified |
| `IR_FUNCTION` (IR 16) | `[FunctionDefinition Node]` → `Record(Function)`; emits Constant+Return | same payload → `Record(Function)`; creates Function/Block, enqueues first `IR_STATEMENT`, awaits chain, completes after Return is last | `IR_FUNCTION_CHIP` (16), modified |
| `VERIFICATION_TYPED_INVARIANT` (VF 16) | `[TU Node]` → `Ack`; fixed M1 set | same payload → `Ack`; V1 checked-set/conversion/effect/constant completeness | `VF06_CHIP` (15), modified |
| `VERIFICATION_IR_INTERPRET` (VF 17) | `[Function]` → `Record(Const)`; Constant+Return only | `[Function, Const expected]` → `Ack` on match; `[Function]` legacy mode → `Record(Const)` only for constant-only functions; V1 acceptance uses the 2-ref mode | `VF12_CHIP` (20), modified |
| `VERIFICATION_TOKEN_AST_INVARIANT` (VF 18) | `[TU Node]` → `Ack`; fixed 8 kinds | same payload → `Ack`; V1 18-kind shapes/ranges | `VF05_CHIP` (21), modified |

`PARSE_TU` range payload: the parser reads `lex.tokens` from `first` through `last`
inclusive. The range must be contiguous in committed token order, belong to one
source, and include the EOF token for a TU. A single-token range repeats the same
`TokenId` twice. This replaces the current “all tokens in payload” convention and
avoids duplicating large token vectors in every child payload.

#### 3.6.2 New task kinds

Local codes are proposed (not frozen). All are `Emulable` / `CpuReference` /
`Propagation` / deterministic, and use the existing `(phase priority, enqueue
ordinal, TaskId)` order.

| New task kind | Local | Payload → result | Chip (proposed) | Writes |
|---|---|---|---|---|
| `SYMBOL_ASSIGNMENT_CONVERT` | SYMBOL_TYPE 25 | `[Type source, Type destination]` → `Record(Type)` (identity only) | `TY_CONV_CHIP` (11), extended | none |
| `SEMANTIC_IDENTIFIER_EXPR` | SEMANTIC 22 | `[Identifier Node, Scope]` → `Record(Sem)`; enqueue `SYMBOL_LOOKUP [Scope, Name, Token]`, await | `SE_IDENTIFIER_CHIP` (64) | `sem.records` |
| `SEMANTIC_ASSIGNMENT_EXPR` | SEMANTIC 23 | `[Assignment Node]` → `Record(Sem)`; requires child SemRecords; enqueue `SYMBOL_ASSIGNMENT_CONVERT`, await | `SE_ASSIGNMENT_CHIP` (65) | `sem.records` |
| `SEMANTIC_DECLARATION` | SEMANTIC 24 | `[Declaration Node, Scope]` → `Record(Sem)`; initializer conversion if present | `SE_DECLARATION_CHIP` (66) | `sem.records` |
| `SEMANTIC_UNARY_EXPR` | SEMANTIC 25 | `[UnaryPlus/UnaryMinus Node]` → `Record(Sem)`; enqueue `SYMBOL_PROMOTE [operand ty]`, await | `SE_UNARY_CHIP` (67) | `sem.records` |
| `IR_STATEMENT` | IR_LOWER 17 | `[stmt Node]` → `Ack`; expression child + store/return/chain | `IR_STATEMENT_CHIP` (68) | `ir.instructions` |
| `IR_EXPRESSION` | IR_LOWER 18 | `[expr Node]` → `Record(Value)`; loads/arithmetic/assignment | `IR_EXPRESSION_CHIP` (69) | `ir.values`, `ir.instructions` |

Chip IDs 64-69 are chosen above the uncommitted candidate window (`ChipId(58)`)
and the `/39` maximum (`SE_FUNC_CHIP = 57`). They remain **proposed** until the
integrator arbitrates candidate drafts. No new `PARSE` local is claimed.

#### 3.6.3 Result carriers and child-result convention

V1 uses only existing `ResultValue` variants:

| Producer | Result | Consumer |
|---|---|---|
| Parser (`PARSE_TU`) | `Record(Node TU)` | Integration driver / VF05 / VF06 |
| `SYMBOL_INT_TYPE`/`FUNC_TYPE`/`ASSIGNMENT_CONVERT`/`PROMOTE`/`COMMON_TYPE`/`RETURN_CONVERT` | `Record(Type)` | SE/IR driver |
| `SYMBOL_SCOPE_ENTER` | `Record(Scope)` | Driver/TY |
| `SYMBOL_DECLARE` | `Record(Symbol)` | Driver/SE/IR/VF |
| `SYMBOL_LOOKUP` | `Record(Symbol)`; miss = typed `Fail` | `SEMANTIC_IDENTIFIER_EXPR` |
| `SEMANTIC_*` | `Record(Sem)` | Parent SE task, VF06, IR |
| `CONSTANT_CONST_FOLD` | `Record(Const)` or `Fail` | Parent SE task, `IR_EXPRESSION`, VF12 |
| `IR_FUNCTION` | `Record(Function)` | VF12 |
| `IR_EXPRESSION` | `Record(Value)` | `IR_STATEMENT` |
| `IR_STATEMENT` | `Ack` | `IR_FUNCTION`/previous statement |
| `VERIFICATION_*` | `Ack` or typed `Fail` | Driver |

**Child-result rule (PROPOSED):** a parent task reads its child’s committed
`ResultRecord` through `TaskState::Completed(ResultId)` and **does not consume**
it; the join rule `join.no-consume-at-join` remains in force. Each child has exactly
one parent/consumer in V1, so re-reading is idempotent and no second claim exists.
If the integrator requires explicit consumption, a T01-owned result-claim hook is
needed; that is a larger change and is not required for V1 with range-partitioned/
single-parser tasks.

### 3.7 Diagnostics and error/unsupported taxonomy

Recommended new group:

```rust
pub enum DiagGroup {
    // FROZEN /39
    Protocol, Arena, Config, Target, Manifest, Task, Semantic, Unsupported, Internal,
    // V1 APPEND
    Parse,
}
```

If the integrator rejects a new group, use `Semantic` codes 20+ for syntax and
document the deviation; that is less honest but avoids a group append (§8).

| Group | Code | Meaning | Carrier |
|---|---:|---|---|
| `Parse` | 1 | Expected token/construct | Chip `Fail` |
| `Parse` | 2 | Missing terminator (`;`, `)`, `}`) | Chip `Fail` |
| `Parse` | 3 | Unbalanced delimiter | Chip `Fail` |
| `Parse` | 4 | Empty expression / missing operand | Chip `Fail` |
| `Parse` | 5 | Invalid token range (not committed, wrong source, first > last) | Chip `Fail` / protocol fault |
| `Parse` | 6 | Grammar nesting limit exceeded (reuses `max_task_depth`) | Chip `Fail` |
| `Parse` | 7 | Unexpected top-level/declaration/statement shape in the V1 grammar | Chip `Fail` (recognized but unimplemented features use `Unsupported/6`) |
| `Semantic` | 1 | Symbol conflict (existing) | Chip `Fail` / commit structural check |
| `Semantic` | 2 | Undeclared identifier (existing) | Chip `Fail` |
| `Semantic` | 3 | Not a modifiable lvalue | Chip `Fail` |
| `Semantic` | 4 | Assignment/initializer/return type mismatch | Chip `Fail` |
| `Semantic` | 5 | Use before declaration / not visible at query point | Chip `Fail` |
| `Semantic` | 6 | Duplicate `SemRecord` for a node | Commit or VF06 failure |
| `Semantic` | 7 | Missing/extra conversion plan for a required role | VF06 failure |
| `Semantic` | 8 | Uninitialized local read | VF12 failure |
| `Unsupported` | 1 | Generic unsupported construct (existing) | Chip `Fail` |
| `Unsupported` | 2 | Constant overflow (existing) | Chip `Fail` |
| `Unsupported` | 3 | Constant unsupported form (existing) | Chip `Fail` |
| `Unsupported` | 4 | Not a constant expression (existing) | Chip `Fail` |
| `Unsupported` | 5 | Non-identity conversion outside V1 | Chip `Fail` |
| `Unsupported` | 6 | Non-V1 type/operator/statement | Chip `Fail` |
| `Task` | 4 | Missing referenced record / not running (existing convention) | Chip `Fail` |
| `Protocol` | 1 | Wrong task kind/dispatch/projection (existing convention) | Chip `Fail` |
| `Arena`/`Config`/`Internal` | existing | Capacity/config/internal | Structured commit/projection error |

Rules:

- Semantic recoverable faults are **chip diagnostics** through `Fail`; they do not
  reject a whole batch.
- Structural protocol faults (wrong payload family/arity, unknown task, non-running)
  are typed `Fail` on the task channel; they must not mutate semantic state.
- Unsupported constructs fail closed and stay in the denominator; they never become
  PASS.
- All batch mutations are preflight/atomic; a failed append materializes no partial
  AST/SE/IR records.
- No panic control flow; no silent default; no fabricated success.

### 3.8 Store schema, allowlist, and stage assignment

No new store or field is proposed. New allowlist rows (proposed chip IDs):

| Chip | Store/field | Task kind | Change |
|---|---|---|---|
| `PA_TU_CHIP` (7) | `parse.nodes` | `PARSE_TU` | Existing row; behavior generalized |
| `TY_SYMBOL_CHIP` (10) | `symbols.symbols` | `SYMBOL_DECLARE` | Existing row; object/block now allowed |
| `TY_SCOPE_CHIP` (9) | `symbols.scopes`/`scope_events` | `SYMBOL_SCOPE_ENTER/EXIT` | Existing rows (block scope now exercised) |
| `SE_LIT_CHIP` (12) | `sem.records` | `SEMANTIC_LITERAL_EXPR` | Existing row; const link added |
| `SE_BIN_CHIP` (13) | `sem.records` | `SEMANTIC_BINARY_EXPR` | Existing row; generalized |
| `SE_RET_CHIP` (14) | `sem.records` | `SEMANTIC_RETURN_STMT` | Existing row |
| `SE_FUNC_CHIP` (57) | `sem.records` | `SEMANTIC_FUNCTION_DEF` | Existing row |
| `SE_IDENTIFIER_CHIP` (64) | `sem.records` | `SEMANTIC_IDENTIFIER_EXPR` | New |
| `SE_ASSIGNMENT_CHIP` (65) | `sem.records` | `SEMANTIC_ASSIGNMENT_EXPR` | New |
| `SE_DECLARATION_CHIP` (66) | `sem.records` | `SEMANTIC_DECLARATION` | New |
| `SE_UNARY_CHIP` (67) | `sem.records` | `SEMANTIC_UNARY_EXPR` | New |
| `IR_FUNCTION_CHIP` (16) | `ir.functions`, `ir.blocks` | `IR_FUNCTION` | Narrow: remove `ir.values`/`ir.instructions` rows |
| `IR_STATEMENT_CHIP` (68) | `ir.instructions` | `IR_STATEMENT` | New |
| `IR_EXPRESSION_CHIP` (69) | `ir.values`, `ir.instructions` | `IR_EXPRESSION` | New |
| `G1_FOLD_CHIP` (2) | `constants.records` | `CONSTANT_CONST_FOLD` | Existing row; multiple requests over time |

Proposed stage rows:

| Kind | Current stage | Proposed V1 stage | Reason |
|---|---:|---:|---|
| `PARSE_TU` | 2 | 2 | Unchanged |
| `SYMBOL_*` | 3 | 3 | Unchanged |
| `SEMANTIC_*` existing | 4 | 4 | Unchanged |
| `SEMANTIC_CONST_EVAL_LITERAL/BINARY` | 1 | 4 (or retire) | Align with sem→const order |
| `CONSTANT_CONST_FOLD` | 2 | 4 | Const evaluation must follow SE requests and precede IR |
| `IR_FUNCTION` | 5 | 5 | Unchanged |
| `IR_STATEMENT`/`IR_EXPRESSION` | — | 5 | Same IR layer |
| `SYMBOL_ASSIGNMENT_CONVERT` | — | 3 | TY service |
| `SEMANTIC_IDENTIFIER/ASSIGNMENT/DECLARATION/UNARY` | — | 4 | Semantic layer |
| `VERIFICATION_TYPED_INVARIANT` | 4 | 4 | After SemRecords, before IR |
| `VERIFICATION_IR_INTERPRET` | 6 | 6 | After IR |

**Important:** stage numbers are currently manifest-consistency labels, not strict
runtime barriers; the ready queue orders by enqueue ordinal and quota 1. The V1
freeze should still make the table honest. A later `stage_queues` implementation
must preserve the same dependency order. If the integrator does not want to remap
existing rows, it must explicitly document that `CONSTANT_CONST_FOLD` at stage 2 is
a legacy scheduling label and add a V1 dependency rule instead; that is a weaker
contract and is not recommended.

#### 3.8.1 V1 manifest read/write sketch (PROPOSED)

`tasks.active.{id,kind,payload,state,owner,parent,continuation}` are read by every
worker per T01 §5 and are omitted below except where a task is unusual. `writes`
lists only append/patch fields; the output slot is implicit.

| Chip / task | Reads (field-scoped) | Writes |
|---|---|---|
| `PA_TU_CHIP` / `PARSE_TU` | `lex.tokens`, `lex.literals`, `names.entries`, `parse.nodes` (count), `config.limits` | `parse.nodes` |
| `TY_SYMBOL_CHIP` / `SYMBOL_DECLARE` | `parse.nodes`, `symbols.symbols`, `symbols.scopes`, `types.records` | `symbols.symbols` |
| `TY_SYMBOL_CHIP` / `SYMBOL_LOOKUP` | `symbols.symbols`, `symbols.scopes`, `lex.tokens` | none |
| `TY_CONV_CHIP` / `SYMBOL_ASSIGNMENT_CONVERT` | `types.records` | none |
| `SE_LIT_CHIP` / `SEMANTIC_LITERAL_EXPR` | `parse.nodes`, `lex.literals`, `types.records`, `sem.records` | `sem.records` |
| `SE_IDENTIFIER_CHIP` / `SEMANTIC_IDENTIFIER_EXPR` | `parse.nodes`, `symbols.symbols`, `sem.records` | `sem.records` |
| `SE_ASSIGNMENT_CHIP` / `SEMANTIC_ASSIGNMENT_EXPR` | `parse.nodes`, `sem.records` | `sem.records` |
| `SE_DECLARATION_CHIP` / `SEMANTIC_DECLARATION` | `parse.nodes`, `symbols.symbols`, `sem.records`, `types.records` | `sem.records` |
| `SE_UNARY_CHIP` / `SEMANTIC_UNARY_EXPR` | `parse.nodes`, `sem.records` | `sem.records` |
| `SE_BIN_CHIP` / `SEMANTIC_BINARY_EXPR` | `parse.nodes`, `sem.records`, `lex.literals` | `sem.records` |
| `SE_RET_CHIP` / `SEMANTIC_RETURN_STMT` | `parse.nodes`, `sem.records`, `types.records`, `symbols.symbols` | `sem.records` |
| `SE_FUNC_CHIP` / `SEMANTIC_FUNCTION_DEF` | `parse.nodes`, `sem.records`, `symbols.symbols`, `types.records` | `sem.records` |
| `G1_FOLD_CHIP` / `CONSTANT_CONST_FOLD` | `lex.literals`, `parse.nodes`, `constants.records` (count), `config.limits` | `constants.records` |
| `IR_FUNCTION_CHIP` / `IR_FUNCTION` | `parse.nodes`, `symbols.symbols`, `types.records`, `sem.records`, `ir.functions`/`ir.blocks` (counts) | `ir.functions`, `ir.blocks` |
| `IR_STATEMENT_CHIP` / `IR_STATEMENT` | `parse.nodes`, `sem.records`, `symbols.symbols`, `ir.functions`, `ir.blocks`, `ir.instructions` | `ir.instructions` |
| `IR_EXPRESSION_CHIP` / `IR_EXPRESSION` | `parse.nodes`, `sem.records`, `constants.records`, `symbols.symbols`, `ir.blocks`, `ir.values`, `ir.instructions` | `ir.values`, `ir.instructions` |
| `VF06_CHIP` / `VERIFICATION_TYPED_INVARIANT` | `parse.nodes`, `sem.records`, `symbols.symbols`, `symbols.scopes`, `constants.records`, `types.records` | none |
| `VF12_CHIP` / `VERIFICATION_IR_INTERPRET` | `ir.functions`, `ir.blocks`, `ir.values`, `ir.instructions`, `constants.records`, `types.records` | none |

The table is a **proposal for F2**; the integrator must reject any undeclared field
and any second writer that is not authorized by §3.8. The existing `TY_TYPE_CHIP`,
`TY_SCOPE_CHIP`, and `TY_CONV_CHIP` promote/common/return rows remain as in `/39`
with block-scope use now exercised.

---

## 4. Ownership and Producer–Consumer Matrix

### 4.1 Ownership model

`/39` already has multiple authorized chips appending to `sem.records`
(`SE_LIT_CHIP`, `SE_BIN_CHIP`, `SE_RET_CHIP`, `SE_FUNC_CHIP`) and to `ir.*`
(`IR_FUNCTION_CHIP` for four fields). The operative ownership rule is therefore:
**one producer per record; one `(ChipId, StoreId, field, TaskKind)` allowlist row
per authorized appender; no chip may write a record it does not own.** The
guardrail wording “exactly one owner per store/field” is stricter than the
implemented allowlist model; V1 should not silently resolve that tension. Proposed
resolution: keep the `(ChipId, field, kind)` allowlist as the frozen ownership
unit, and state that multiple appenders to one append-only field are allowed only
when each appends disjoint records and the commit order is deterministic (§8,
O-17). If the integrator insists on one chip per field, V1 cannot be implemented
without a single giant SE/IR chip; that alternative is rejected in §8.2.

### 4.2 Matrix

| Artifact / field | Single producer(s) | Consumers | Current `/39` | V1 change |
|---|---|---|---|---|
| `lex.tokens`, `lex.literals` | T04 chips (`LX_CLASSIFY`, `LX_DECODE`, `LX_CHAR/STRING`) | PA, SE, T08 | Implemented | None |
| `parse.nodes` | `PA_TU_CHIP` (single V1 parser) | TY driver, SE, VF05/VF06, IR | PA_TU only for fixed M1 tree | Parser emits V1 tree; no new PA writer |
| `symbols.scopes`, `scope_events` | `TY_SCOPE_CHIP` | TY lookup, SE, VF | Implemented | Block scope exercised |
| `symbols.symbols` | `TY_SYMBOL_CHIP` (`SYMBOL_DECLARE`) | SE lookup, IR, VF | Function/file only | Object/block added |
| `types.records` | `TY_TYPE_CHIP` (`int`/`func`); no V1 conversion writer | PA? no; SE/IR/VF read | Implemented | None; assignment convert returns an existing type |
| `sem.records` | `SE_LIT/BIN/RET/FUNC` + four new SE chips | VF06, IR, VF12 | Four writers | Four new writers; same field |
| `constants.records` | `G1_FOLD_CHIP` | SE (link), IR, VF12 | One writer | Multiple requests over time; one record per request |
| `ir.functions` | `IR_FUNCTION_CHIP` | VF12, T11 later | Implemented | Function creation only |
| `ir.blocks` | `IR_FUNCTION_CHIP` | VF12, T11 later | Implemented | Entry block only |
| `ir.values` | `IR_EXPRESSION_CHIP` | IR statements, VF12, T11 later | `IR_FUNCTION_CHIP` | Narrowed to new chip |
| `ir.instructions` | `IR_STATEMENT_CHIP`, `IR_EXPRESSION_CHIP` | VF12, T11 later | `IR_FUNCTION_CHIP` | Two new writers; source-order chain |
| `tasks.results` | Commit/control | Parent tasks read child results; no consume | Existing | Child-result read rule (§3.6.3) |
| `tasks.continuations` | No V1 writer | — | Reserved | Deferred |
| `diagnostics.entries` | Commit/control | Host/VF | Existing | New codes/groups |
| `wires.proposals.*` | Every worker | Commit/control | Existing | No new proposal type |

### 4.3 Single-writer checks per work order

- Parser: only `PA_TU_CHIP` may append `parse.nodes` for `PARSE_TU`.
- TY: only `TY_SYMBOL_CHIP` appends `symbols.symbols`; only `TY_SCOPE_CHIP`
  appends `scopes`/`scope_events`; no V1 chip appends `types.records` except the
  existing `TY_TYPE_CHIP`.
- SE: each new chip has exactly one allowlist row; no shared helper writes.
- CL: `G1_FOLD_CHIP` is the sole `constants.records` writer.
- IR: `IR_FUNCTION_CHIP` only functions/blocks; `IR_STATEMENT_CHIP` only
  instructions; `IR_EXPRESSION_CHIP` only values/instructions. No overlap.
- VF: read-only; zero writes.
- Integration driver: no semantic writes; only enqueues/consumes results through
  control APIs/tests.

---

## 5. Legality, Error, and Unsupported Rules

### 5.1 Parser (PA)

1. Payload is exactly `[Token first, Token last]`; both committed, same source,
   `first.index() <= last.index()`; the range covers the whole TU incl. EOF.
2. The parser accepts only the V1 grammar (§1.2). Any other token/construct is a
   typed `Unsupported`/`Parse` failure; no fallback grammar.
3. Node ranges follow §3.3.3; parent/children are reciprocal; sibling ranges are
   ordered and non-overlapping (VF05 re-checks).
4. Parser nesting uses explicit stacks/counters. Reuse `limits.max_task_depth` as
   the grammar nesting bound; exceedance is `Parse/6`, never a Rust stack overflow.
5. The parser appends one atomic batch. If any preflight/commit check fails, no
   node is visible. Exceeding `max_records_per_arena`/`max_records_total`/
   `max_proposals_per_tick` is a structured capacity/limit failure, not a partial
   AST and not a panic.
6. `int main` must have `(void)` (zero parameters); any parameter list, old-style
   list, pointer/array declarator, or non-`int` specifier is `Unsupported`.
7. Declaration: exactly one declarator, identifier only, optional `= expression`.
   Comma lists, storage classes, qualifiers, and braced initializers are
   `Unsupported`.
8. Statements: declaration, expression statement, return-with-expression. `return;`
   is `Unsupported` in V1.0 (the return value policy is not frozen).
9. Return must be the last body item. A non-last return, empty body, or missing
   return is `Unsupported` in V1.0.
10. Single error recovery: V1 fails the parse task at the first syntax error; PA38
    multi-error recovery is not used. The error is typed, located, and finite.

### 5.2 Symbol/type (TY)

1. `SYMBOL_DECLARE` payload is exactly `[Identifier Node, Type, Scope]`.
2. `Identifier.name = Some`; the identifier token's spelling resolves to the
   declared `NameId`.
3. Derivation table:
   - `ScopeKind::File` + `TypeKind::Function` → `SymbolKind::Function`,
     `Linkage::External`, `StorageDuration::Static`.
   - `ScopeKind::Block` + `TypeKind::Int` → `SymbolKind::Object`,
     `Linkage::None`, `StorageDuration::Automatic`.
   - Any other combination is `Unsupported` in V1.0.
4. Duplicate `(name, scope, kind)` is a typed `RedeclarationConflict` chip
   diagnostic; commit may also reject an exact structural duplicate if that check
   is implemented, but V1 does not depend on a new `CommitError`.
5. `SYMBOL_LOOKUP` payload is exactly `[Scope, Name, Token query]`. It walks the
   active scope chain only (`scope` upward to file), never a closed/sibling scope.
   The query point is the identifier token’s span start. A declaration is visible
   after the completion of its complete declarator (C11 6.2.1p7); a symbol whose
   declaration tuple is greater than the query tuple is not visible.
6. Wrong-namespace lookup is a miss, not a conflict.
7. `SYMBOL_PROMOTE`/`COMMON_TYPE`/`RETURN_CONVERT`/`ASSIGNMENT_CONVERT` accept only
   the V1 `int` corner and return the existing `TypeId` unchanged. A non-identity
   request is `Unsupported` (no fabricated cast, no target width).
8. `SYMBOL_SCOPE_ENTER` accepts the committed TU root (file) or `Compound` node
   (block); one file scope only; one `Enter` per owner; block `Exit` after all
   block statements. File scope never exits in V1.

### 5.3 Semantic (SE)

1. Every checked node in the §3.4.1 set gets exactly one `SemRecord`; duplicate
   append/commit is a structural failure/VF06 failure. Syntax wrappers outside that
   set must not get a `SemRecord`.
2. `Identifier` expression: lookup child must return an `Object` (or function
   designator in future); V1 only object; `ty = symbol.ty`, `category = Lvalue`,
   `effects = 0`, `conversions = []`, `constant = None`, `symbol = Some(symbol)`.
3. `IntLiteral`: `ty = int`, `category = NonLvalue`, `effects = 0`,
   `conversions = []`, `constant = Some(c)` from the T08 child, `symbol = None`.
4. Unary:
   - `UnaryPlus`: result type = promoted operand type; category `NonLvalue`;
     effects = operand effects; conversions = []; constant = None (T08 has no
     unary fold in V1).
   - `UnaryMinus`: same, but IR emits `Neg`; constant = None.
5. Binary:
   - Require both child SemRecords. In **one** `AwaitChildren` batch, enqueue
     `SYMBOL_PROMOTE` for each operand, `SYMBOL_COMMON_TYPE` with the two original
     operand types (valid because V1 promotion is identity), and — only if op is
     `Add` and both children are `IntLiteral` leaves with committed literals — one
     `CONSTANT_CONST_FOLD [Node, Literal, Literal]`. Await all children; result
     type = common type; category `NonLvalue`; effects = OR of operands;
     conversions = [] (identity); set `constant = Some(c)` only from the optional
     fold. Do not fold any other operator in V1.
   - The single-batch rule is deliberate: it avoids a multi-phase resume that
     would need a continuation/cursor just to remember which TY child already ran.
     A future non-identity promotion/common-type dependency must add that
     continuation or split the task.
6. Assignment:
   - LHS must be a modifiable lvalue; V1 LHS is an `Identifier` resolving to an
     automatic object; array/function/const/literal LHS is rejected
     (`Semantic/3`).
   - RHS type must be `int`; enqueue `SYMBOL_ASSIGNMENT_CONVERT [rhs, lhs]`;
     await; result type = lhs type; category `NonLvalue`; effects = lhs read
     effects (none in V1) | rhs effects | `WritesLocal`; conversions = one
     `(Identity, Assignment, rhs, lhs)` plan; constant = None.
   - The assignment expression value is the converted RHS value (identity in V1).
7. Declaration:
   - Specifier must be `int`; declarator must name an automatic object in the
     current block scope; the symbol must be committed before the SE check.
   - Without initializer: `ty = int`, `category = Void`, `effects = 0`,
     `conversions = []`, `constant = None`, `symbol = Some(declared symbol)`.
   - With initializer: child expression must be `int`; enqueue
     `SYMBOL_ASSIGNMENT_CONVERT [init, declared]`; await; `category = Void`;
     `effects = init.effects | WritesLocal`; one `(Identity, Assignment, init,
     declared)` plan; `constant = None`; `symbol = Some(declared symbol)`.
   - `int x = x;` is legal syntax; the lookup point makes the initializer `x`
     resolve to the newly declared `x` (C11 6.2.1p7). The value is indeterminate;
     V1 acceptance must not rely on it. Reading an uninitialized automatic object
     is outside the V1 normative value tests; VF12 may fail it closed as
     `Semantic/8`.
8. Return:
   - Exactly one expression child; function result type must be `int`; enqueue
     `SYMBOL_RETURN_CONVERT [expr ty, return ty]`; await; `ty = return ty`,
     `category = Void`, `effects = expr.effects`, one
     `(Identity, Return, expr, return)` plan, `constant = None` (the expression’s
     own record carries any constant).
9. Function definition:
   - `main` signature `int(void)`; symbol must be `Function`/External/Static;
     body must be a `Compound` whose checked declarations/statements are complete;
     `ty = signature`, `category = Void`, `effects = OR of body effects`,
     `conversions = []`, `constant = None`, `symbol = Some(main symbol)`.
10. All other checked nodes (unary, binary, assignment, return, literal) set
    `symbol = None`; only `Identifier`, `Declaration`, and `FunctionDefinition`
    carry a symbol link in V1.
11. No assignment/initialization of a function, no function returning aggregate,
    no `void` value use, no implicit conversion other than identity.
12. Effects: only bit 0 (`WritesLocal`) is legal in V1; unknown bits are
    `Unsupported`. The effect mask is a semantic fact; IR must not delete a
    `LocalStore` that carries it.

### 5.4 Constants (CL/T08)

1. `CONSTANT_CONST_FOLD` accepts exactly:
   - `[Literal]` for a leaf literal; or
   - `[Node, Literal, Literal]` for an additive binary use.
2. It evaluates only decimal integer literals, no suffix, `Int` candidate; `Add`
   only; checked magnitude arithmetic under `limits.max_const_bits`; insufficient
   budget is a typed `ConstOverflow` failure, not a `Legal` success.
3. It commits exactly one `ConstRecord` per successful request. No per-operand
   records, no join, no hand-built value.
4. It returns `Record(Const)` on `Legal`; non-legal outcomes `Fail` through the
   existing `ConstantResult::route` mapping. `NotConstantExpression` and
   `Unsupported` commit no record.
5. The parent SE task sets `SemRecord.constant` to the committed `ConstId` only
   after the child completes. No other chip writes that link.
6. V1 has no `ConstRecord.ty`; the type authority is `SemRecord.ty` (and the IR
   result `ValueRecord.ty`). This is sufficient for `int`-only V1. Adding type or
   origin fields is deferred (§8).

### 5.5 IR (T09)

1. `IR_FUNCTION` requires the checked `FunctionDefinition` node, the committed
   `main` symbol/signature, all body SemRecords, and the body structure.
2. It creates one `FunctionRecord` and one entry `BlockRecord`; it does not create
   values/instructions itself.
3. `IR_STATEMENT` handles one body item:
   - Declaration without initializer: no instruction.
   - Declaration with initializer: expression child, then `LocalStore` to the
     symbol named by the Declaration `SemRecord.symbol`.
   - ExpressionStatement: expression child; no extra instruction (assignment
     already stores).
   - Return: expression child, then `Return`; must be the last item.
   - Anything else: `Unsupported`.
4. `IR_EXPRESSION`:
   - If `SemRecord.constant = Some(c)`, emit one `Constant(c)` and do not descend.
   - `Identifier` → `LocalLoad` using `SemRecord.symbol`; the symbol must be an
     automatic object of the current function and its type must match.
   - `Assignment` → lower the RHS only (do not emit a load of the LHS), `LocalStore`
     to the LHS `Identifier`’s `SemRecord.symbol`, return the RHS value.
   - `Binary*` → lower both operands, emit the matching op; `Div`/`Rem` by a
     constant zero is a typed failure (do not emit a trap as success). For V1
     symbolic integers, `Div` truncates toward zero and `Rem` takes the sign of the
     dividend (C99 rule); target-width overflow is not modeled or claimed.
   - `UnaryPlus` → return operand value unchanged; `UnaryMinus` → `Neg`.
   - `IntLiteral` without a `SemRecord.constant` is a missing-input failure; T08
     must have committed the constant.
5. Every `ValueRecord` has exactly one producing `Instruction.result`; every
   `Instruction` references a committed block; the `Return` is the unique
   terminator and the greatest `InstructionId`; nothing follows it.
6. `InstructionRecord.local` is `Some` only for `LocalLoad`/`LocalStore`; every
   other op has `None`. `immediate` is `Some` only for `Constant`.
7. The IR chain preserves source order. A failure in any statement/expression
   fails the parent chain; no partial function is reported as complete.
8. No type is re-inferred from the AST/IR; T09 consumes committed `SemRecord`s and
   `ConstRecord`s. No target width or host arithmetic is used for C values; V1
   arithmetic is symbolic-mathematical and target-width behavior is Part B. V1
   acceptance programs must not rely on signed overflow, target-width wraparound,
   or indeterminate values; those remain unclaimed/undefined for V1 and are not
   converted into a fabricated result.

### 5.6 Verification (VF)

1. VF05 (token/AST invariant):
   - all 18 node kinds have the shapes in §3.3.3;
   - parent/children reciprocal; ranges contained; sibling ranges ordered and
     non-overlapping; all tokens committed; unique trailing EOF;
   - `Identifier` has `name: Some`; `IntLiteral` has `literal: Some`; containers
     have the expected child arity.
2. VF06 (typed invariant): its manifest must add the `symbols.symbols` and
   `constants.records` read paths (it already reads `sem.records`/`parse.nodes`);
   - every checked node has exactly one `SemRecord`;
   - `ValueCategory` matches the node kind and declaration/return/function rules;
   - `effects` are legal (`0`/bit 0) and match the write/read facts;
   - assignment/initializer/return conversion plans exist with the exact
     `(Identity, role, from, to)`; no duplicate `(node, role)`;
   - `SemRecord.constant = Some(c)` resolves to a committed `ConstRecord` and the
     node is a constant expression; no missing/extra link;
   - `SemRecord.symbol = Some(s)` resolves to a committed `SymbolRecord` of the
     right kind/type for every `Identifier` expression, `Declaration`, and
     `FunctionDefinition`; no missing/extra symbol link;
   - all `Identifier` expressions resolve to symbols of the right kind; all
     declarations have symbols.
3. VF12 (symbolic IR interpretation, V1 mode):
   - payload `[Function, Const expected]`; interpret the entry block in ascending
     `InstructionId` order;
   - maintain a local `SymbolId → value` map; `LocalLoad` before any `LocalStore`
     fails `Semantic/8`; `LocalStore` updates; arithmetic uses mathematical
     integers (no target width claim);
   - `Div`/`Rem` by zero fails; multiple/missing Return fails; instructions after
     Return fail;
   - compare the returned value with the expected `ConstRecord` (magnitude + sign)
     and complete `Ack` on equality; mismatch is a typed `Fail` (never PASS);
   - the expected constant is supplied by an independent fixture, not derived by
     the verifier from the implementation under test;
   - legacy `[Function]` mode remains only for `/39` constant-only tests and
     completes `Record(Const)`; it is not V1 acceptance.
4. VF01:
   - no new reserved arenas/IDs; `layouts`, `inits`, `vregs` remain empty in V1;
   - all new record references resolve; no tombstone/version change.
5. VF14 evidence classification is unaffected.

### 5.7 Protocol/commit rules

- Wrong task kind/payload family/arity → typed `Fail` (`Protocol`/`Task`), no
  semantic mutation.
- Non-`Running` task → typed `Fail`; a dispatched task must produce exactly one
  transition (`Complete`, `Fail`, `AwaitHost`, `AwaitChildren`, or `Progress`).
- A task that enqueues children must `AwaitChildren` or complete; no idle spin.
- Every child result is read once by its single parent; the join does not consume.
- `AppendRecords` must be authorized by the `(ChipId, StoreId, field, TaskKind)`
  allowlist; an undeclared field/writer is a manifest/commit error.
- A parser/SE/IR batch is atomic; a failed preflight/appends nothing.
- A non-matching task does not change semantic state.

---

## 6. Determinism, Snapshot, and Replay Impact

### 6.1 Deterministic order

- AST node IDs: `nodes_allocated + pre-order position`; parser batch order is
  deterministic; same input/state → same IDs.
- Sibling/child order: source order in `children`; consumers use `BTreeMap`/ordered
  scans, never hash-map iteration.
- Symbol order: commit order; lookup orders by declaration tuple
  `(source, start, end, NodeId)` per T06; V1 query point is the identifier token.
- SemRecord order: commit order; one per node; conversions ordered by role/append;
  constant/symbol links point only at committed records.
- Const order: request commit order; one per request; parent link is the committed
  `ConstId`.
- IR instruction order: statement chain + post-order expression emission; `Return`
  is the unique greatest `InstructionId`.
- Task order: `(phase priority, enqueue ordinal, TaskId)`, quota 1 baseline; new
  tasks are ready next tick; no same-tick re-entry.
- Diagnostics: commit ordinal order; no wall clock, randomness, host path, or
  address participates.

### 6.2 Snapshot and hash delta

| Encoder / inventory | Current `/39` | V1 proposed | Impact |
|---|---|---|---|
| `encode_node`/`decode_node` | 7 fields, 8 kinds | **No field change**; `node_kind_name`/`parse_node_kind` add 10 kinds | Snapshot bytes change for V1 trees; field layout unchanged |
| `encode_sem`/`decode_sem` | `node`, `ty`, `category`, `effects` | + conversions count/plans + constant option + symbol option | Byte layout changes; tests update |
| `encode_instruction`/`decode_instruction` | 5 fields, 2 ops | + local option; 10 ops | Byte layout changes; tests update |
| `encode_function`/`encode_block`/`encode_value` | unchanged | unchanged | None |
| `encode_const` | unchanged | unchanged | None |
| `DIAG_GROUP_NAMES` | 9 groups | + `parse` if accepted | Hash/snapshot diagnostic names change |
| `NODE_KIND_NAMES` | 8 | 18 | Hash changes |
| `SEM_RECORD_FIELDS` | 4 | 7 | Hash changes |
| `IR_OP_NAMES` | 2 | 10 | Hash changes |
| `INSTRUCTION_RECORD_FIELDS` | 5 | 6 | Hash changes |
| `CONVERSION_OP_NAMES` / `CONVERSION_ROLE_NAMES` / `CONVERSION_PLAN_FIELDS` | absent | new lists | Must be added to `FrozenSchema::encode`; otherwise not hashed |
| `EFFECT_MASK_NAMES` | absent | `["writes_local"]` (bit 0) | Must be added if bit semantics are frozen |
| `STAGE_ASSIGNMENT` | 69 rows | +7 rows; possible remap of 3 existing rows | Hash changes |
| `STORE_OWNER_ALLOWLIST` | 33 rows | +7 rows, −2 `IR_FUNCTION` rows | Hash changes |
| `NORMATIVE_RULES` | 188 (current list) | +V1 rules | Hash changes |
| `TaskKindRegistry` slice | Gate 1 subset hashed; later kinds only via stage/allowlist raw codes | V1 kind names/status must be added to the hash seed | Otherwise a rename is invisible in the hash |
| `CONTRACT_VERSION` | `t01-c01-c06/39` | e.g. `t01-c01-c06/40` (integrator chooses) | New artifact; `/39` preserved as history |
| `contracts/CONTRACT_VERSION` hash | `07f4…` | **NOT COMPUTED in this report** | Integrator recomputes after freeze |

No new `RecordRef` wire tags, `RecordFamily` ordinals, `Proposal` tags, or
`ResultValue` variants are proposed, so the cold/wire envelope remains stable.
The V1 task-kind IDs are raw `TaskKind` codes and do not change the `RecordRef`
inventory.

### 6.3 Replay and cross-version behavior

- Replay test: same initial bus/config/pins + same task sequence → identical
  snapshot bytes per tick, including AST/SE/IR/diagnostics.
- Property test: parser AST range/parent/children invariants; SE one-record-per-
  node; IR value producer uniqueness; `Return` last; no dangling refs.
- Differential test: V1 reference values (e.g. return 11) come from hand
  calculation/specification, not from the implementation under test.
- Cross-version: `/39` snapshots are not required to decode under a V1 contract
  because record bodies changed. The snapshot has no embedded version field; the
  tooling must gate by `CONTRACT_VERSION`/artifact hash and reject cross-version
  replay explicitly. Do not silently reinterpret old bytes.
- No probe, target, corpus, or execution claim is made by any replay test.

### 6.4 Proposed V1 rule IDs (for `NORMATIVE_RULES`)

These are proposed identifiers, not accepted rules:

```text
v1.parse-token-range
v1.parse-v1-grammar
v1.parse-node-shape
v1.parse-parent-children
v1.symbol-decl-identifier-leaf
v1.symbol-object-block
v1.lookup-query-point
v1.conversion-identity-only
v1.assignment-lvalue
v1.assignment-conversion
v1.initializer-conversion
v1.return-conversion
v1.effect-writes-local
v1.sem-one-per-node
v1.sem-constant-link
v1.sem-symbol-link
v1.const-one-per-request
v1.ir-local-load-store
v1.ir-arithmetic
v1.ir-instruction-order
v1.ir-return-terminator
v1.vf06-completeness
v1.vf12-expected-match
v1.vf05-v1-shapes
v1.result-parent-read
v1.stage-sem-const-order
```

The existing `/39` rules `se.literal-checked-nonlvalue`, `se.binary-forwards-
identical-fold`, `se.return-identity-no-plan`, `sem.effect-mask-zero-only`,
`ir.constant-no-refold`, `ir.return-single-terminator`, `ir.function-single-entry`,
`commit.sem-materialized`, and `commit.ir-materialized` must be either preserved
with clarified V1 semantics or superseded by new IDs; the integrator must decide
and record the supersession. Do not keep a rule name whose semantics changed
silently.

---

## 7. Freeze and Implementation Work Orders

### 7.1 Staged freeze sequence

| Stage | Deliverable | Owner | Blocking inputs | Exit condition |
|---|---|---|---|---|
| F0 | V1 decision/ADR + conflict resolution | T01 integrator | This report, A04/A11/A12/A15/A17 reviews | Accepted scope, parser-fusion decision, `EffectMask`/`DiagGroup`/stage decisions, candidate-draft arbitration |
| F1 | Shared schema freeze | T01 integrator | F0 | `bus.rs`/`task.rs`/`manifest.rs`/`contract.rs`/`snapshot.rs` compile; new hash/version; freeze tests updated; no chip logic |
| F2 | Payload/result/manifest freeze | T01 integrator + T05/T06/T07/T08/T09/T13 owners | F1 | Exact payload tables, stages, allowlist rows, diagnostics, rule IDs, test entry points |
| F3 | Group implementation packages | PA/TY/SE/T08/T09/T13 owners | F2 | Each package passes independent unit tests against synthetic fixtures; no real-chain claim |
| F4 | V1 integration chain | Integrator + independent tester | F3 | Real source → AST → TY → SE → CL → IR → VF; deterministic replay; no target claim |
| F5 | V1 Part A acceptance | Integrator | F4 | Canonical example returns 11 via VF12 expected-match; negative matrix; snapshot/replay; documented gaps |

**Freeze rule:** F0/F1/F2 are serial. F3 packages may overlap only after F2.
F4 starts only after all F3 producers are registered and integrated. No Stage-B
implementation work order is READY before F2.

### 7.2 Implementation work orders (proposed; exact paths)

All paths are relative to the repository root. `mod.rs`, `Cargo.toml`, shared
registries, `contract.rs`, `manifest.rs`, `task.rs`, `snapshot.rs`, and
`CONTRACT_VERSION` are integrator-owned; chip owners must not edit them.

| WO | Package | Owner | Owned implementation paths | Owned test paths | Depends on | Acceptance |
|---|---|---|---|---|---|---|
| W0 | F0/F1/F2 contract freeze | T01 integrator | `compiler/src/bus.rs`, `compiler/src/diagnostic.rs`, `compiler/src/task.rs`, `compiler/src/manifest.rs`, `compiler/src/contract.rs`, `compiler/src/snapshot.rs`, `compiler/contracts/CONTRACT_VERSION`, `compiler/contracts/COMPILER_SFL_MANIFEST.md`, `docs/tasks/T01_COMPILER_CONTRACT.md`, `docs/tasks/T05…T09`, new `docs/tasks/V1_VERTICAL_SLICE_CONTRACT.md` | `compiler/tests/freeze.rs`, `compiler/tests/c05_codec.rs`, `compiler/tests/c09_readiness.rs` | A1 reports | Schema frozen; hash recomputed; old tests updated with explicit supersession |
| W1 | V1 parser | PA owner | `compiler/src/chips/parse/pa_tu.rs` (single V1 parser; no new PA kind/chip) | new `compiler/tests/v1_parse.rs`; update `compiler/tests/c12_parse.rs` | F2 | V1 AST for canonical source; negative grammar matrix; range/parent/children/replay |
| W2 | TY object declaration + lookup + assignment conversion | TY owner | `compiler/src/chips/symbols/ty_symbol.rs`, `compiler/src/chips/types/ty_conv.rs` | new `compiler/tests/v1_ty.rs`; update `compiler/tests/c13_ty.rs` | F2, W1 AST | Object/block declare; query-point lookup; identity assignment convert; unsupported non-int/non-identity |
| W3 | SE literal/identifier/unary/binary/assignment/declaration/return/function | SE owners, split per file (exactly one writer per file) | modify `compiler/src/chips/semantic/se_literal.rs`, `se_binary.rs`, `se_return.rs`, `se_function.rs`; new `se_identifier.rs`, `se_assignment.rs`, `se_declaration.rs`, `se_unary.rs` | new `compiler/tests/v1_se.rs`; update `compiler/tests/c14_se.rs` | F2, W1, W2, W4 for const children | Lvalue/effect/conversion/constant/symbol-link matrix; exactly one SemRecord; unsupported |
| W4 | T08 constant support | T08 owner | `compiler/src/chips/constant_layout_init/fold.rs` (only if stage/manifest/tests change; logic already supports Add) | new `compiler/tests/v1_const.rs` | F2 | One Const per request; leaf/binary Add; multiple constants; overflow/unsupported; no refold |
| W5 | IR function/statement/expression | T09 owner | modify `compiler/src/chips/ir_lower/ir_function.rs`; new `ir_statement.rs`, `ir_expression.rs` | new `compiler/tests/v1_ir.rs`; update `compiler/tests/c15_ir.rs` | F2, W2, W3, W4 | Local load/store; arithmetic; init/assignment; return terminator; source order; symbolic values |
| W6 | Verification | T13 owner | modify `compiler/src/chips/verify/vf_invariant.rs`, `vf_interpret.rs`, `vf_syntax.rs`; `vf_store.rs` only if reserved checks change | new `compiler/tests/v1_verify.rs`; update `compiler/tests/c17_vf12.rs`, `c18_vf05.rs` | F2, W1–W5 | VF05/VF06/VF12 completeness; negative matrix; expected-match; no false pass |
| W7 | V1 chain integration driver | Integrator + independent tester | `compiler/tests/v1_chain.rs` (Part A driver); no semantic writes; `compiler/src/bin/candidate.rs` only under a separate H04 work order | `compiler/tests/v1_chain.rs` | W1–W6, registration | Real source → AST → TY → SE → CL → IR → VF; return 11; replay; failure paths |
| W8 | Registration/manifest/hash | T01 integrator | `compiler/src/chips/mod.rs`, group `mod.rs` files, `manifest.rs`, `contract.rs`, `task.rs`, `snapshot.rs`, `CONTRACT_VERSION` | `compiler/tests/freeze.rs`, `c04_manifest.rs`, `c08_gate1.rs`, `c09_readiness.rs` | W0–W6 | Unique IDs/kinds; allowlist rows; stage/layer agreement; hash; no unauthorized field |
| W9 | Documentation/status | Integrator | `docs/tasks/T01_COMPILER_CONTRACT.md` V1 amendment, `docs/tasks/T05…T09` appendices, `docs/tasks/V1_VERTICAL_SLICE_CONTRACT.md`, `docs/tasks/DISPATCH_LEDGER.md` (integrator-owned) | content/link checks only | F0–F5 | Implemented vs proposed vs unverified clearly separated |

#### 7.2.1 V1 chain order (integration driver, Part A)

The integration driver is not a language chip. It enqueues tasks in this fixed
order and must stop on the first failed task; it may use synthetic fixtures only to
replace upstream production, never to replace the semantics under test.

| Step | Enqueue | Required committed input | Result used by driver |
|---:|---|---|---|
| 1 | `PARSE_TU` | `[Token first, Token last]` covering the source + EOF | `Record(Node TU)` |
| 2 | `SYMBOL_INT_TYPE` | empty | `Record(Type int)` |
| 3 | `SYMBOL_FUNC_TYPE` | `[Type int]` | `Record(Type int(void))` |
| 4 | `SYMBOL_SCOPE_ENTER` | `[TU Node]` | `Record(Scope file)` |
| 5 | `SYMBOL_DECLARE` | `[Identifier main, Type int(void), Scope file]` | `Record(Symbol main)` |
| 6 | `SYMBOL_SCOPE_ENTER` | `[Compound Node]` | `Record(Scope block)` |
| 7 | `SYMBOL_DECLARE` per local declaration | `[Identifier, Type int, Scope block]` | `Record(Symbol)` |
| 8 | SE post-order over the body: `SEMANTIC_LITERAL_EXPR`, `SEMANTIC_IDENTIFIER_EXPR`, `SEMANTIC_UNARY_EXPR`, `SEMANTIC_BINARY_EXPR`, `SEMANTIC_ASSIGNMENT_EXPR` | committed AST + symbols + types | `Record(Sem)` each |
| 9 | `SEMANTIC_DECLARATION` per declaration with/without init; then `SEMANTIC_RETURN_STMT`; then `SEMANTIC_FUNCTION_DEF` | child SemRecords | `Record(Sem)` each |
| 10 | `VERIFICATION_TYPED_INVARIANT` | `[TU Node]` | `Ack` |
| 11 | `IR_FUNCTION` | `[FunctionDefinition Node]` | `Record(Function)`; IR tasks run as children |
| 12 | `VERIFICATION_IR_INTERPRET` | `[Function, Const expected]` | `Ack` on expected value |
| 13 | `SYMBOL_SCOPE_EXIT` | `[Scope block]` | `Ack` |

Notes:

- Step 8 is post-order so every child expression’s `SemRecord` is committed before
  its parent check. The CL layer participates through the `CONSTANT_CONST_FOLD`
  children that the SE literal/binary tasks enqueue and await; the driver does not
  enqueue T08 directly. The driver **waits for each SE task to complete** before
  enqueuing a task that consumes its result; a bulk enqueue of an entire dependent
  level would let a parent run while a child is still Waiting on T08/TY and is not
  allowed. The driver never infers types or values; it only schedules.
- Step 9’s declaration check may run after its initializer expression check; the
  symbol was declared at step 7 (point of declaration).
- Steps 10–12 are the V1 verification chain. VF12 must run after IR completion and
  before the scope-exit step; a failure stops the chain and is reported, never
  converted to success.
- The expected `Const` at step 12 is supplied by an independent fixture/hand
  calculation (e.g., `11` for the canonical example), not by the implementation
  under test.
- A production T02 scheduler may replace this driver later; the payload/result
  contract and the order constraints must be preserved.

**File ownership notes**

- The parser work (W1) deliberately modifies one existing file. If the integrator
  decides the file is too large, it may split into `pa_v1_ast.rs`/`pa_v1_expr.rs`
  under the same owner, but `mod.rs` registration remains integrator-owned and the
  split must not create a second `parse.nodes` writer.
- W3 has multiple SE files; each file has one owner. Do not let one owner edit
  another SE chip file; use separate work orders if parallel.
- W4 may be a no-op implementation; do not force a code change just to claim
  progress. The existing fold logic is already close, but the stage/allowlist and
  multi-request tests are contract work.
- W7 is an integration/test driver, not a language chip. It must not compute
  semantic facts; it only calls the committed APIs/enqueues tasks in the frozen
  order. If a production driver is desired, it is a T02/Host task, not part of
  W7.
- Do not touch uncommitted candidate files (`pa_cast.rs`, `pa_decl_spec.rs`,
  `pa_expr_ext.rs`, `pa_ext_dispatch.rs`, `pa_init.rs`, `pa_stmt.rs`,
  `pa_typedef_query.rs`, `lx_error.rs`, `lx_extension.rs`) in any V1 work order
  until the integrator explicitly arbitrates them.

### 7.3 Affected tests and required updates

| Test | Current pin | V1 impact |
|---|---|---|
| `compiler/tests/c12_parse.rs` | 9-node M1 tree, 13-token payload | Update to V1 tree with `Identifier`; range payload `[first,last]`; keep M1 source as a V1 subset fixture |
| `compiler/tests/c13_ty.rs` | Function/file declare; `[Declarator,Type,FileScope]` | Update to `[Identifier,Type,Scope]`; add object/block; query token |
| `compiler/tests/c14_se.rs` | 4-field `SemRecord`; literal/binary/return | Update field construction; add identifier/assignment/declaration/unary; constant/symbol links |
| `compiler/tests/c15_ir.rs` | 2-op `Constant`+`Return`; 2 instructions | Keep constant-only case; add locals/arithmetic/assignment; field `local` |
| `compiler/tests/c17_vf12.rs` | `[Function]` → `Record(Const)` | Keep legacy mode; add `[Function, Const]` → `Ack` |
| `compiler/tests/c18_vf05.rs` | 8-kind tree | Add 18-kind shapes/ranges |
| `compiler/tests/c34_parse.rs`, `c35_expr.rs`, `c36_recovery.rs`, `c39_sefunc.rs` | Ack-only PA/SE slices | Preserve as historical slice tests; do not claim V1. If their manifests/stages change, update only the affected rows |
| `compiler/tests/freeze.rs`, `c05_codec.rs`, `c09_readiness.rs`, `c08_gate1.rs` | Hash/snapshot/stage counts | Update expected hash/bytes/counts after F1; do not hardcode a fabricated V1 hash |
| `compiler/tests/c10…`, `c11…`, `c16…`, `c20…`–`c33…`, `c37`, `c38` | PP/LX/PA/CL/VF slice snapshots | Only affected if snapshot encoding of `SemRecord`/`InstructionRecord`/`DiagGroup` changes; update expected bytes, not semantics |
| New `compiler/tests/v1_*.rs` | — | Independent V1 unit/chain tests; integrator assigns final `cXX` numbers and entry points; expected values hand-derived |

### 7.4 Work-order gates and stop conditions

A work order is READY only if:

1. F2 is accepted with the exact payload/field/rule tables.
2. Its input can come from a real producer or a valid synthetic fixture.
3. Its write paths are allowlisted and its task/result guard is testable.
4. It does not depend on a contested candidate draft or an unfrozen continuation.
5. Its test entry point is registered by the integrator.

Stop and report `BLOCKED_CONTRACT` if:

- the parser fusion vs taskified parser decision is unresolved;
- `NodeRecord` shape/`children` vs `ordinal` is reopened;
- `EffectMask` bit 0 or `DiagGroup::Parse` is rejected without a replacement;
- the stage remap or allowlist ownership is unresolved;
- `ChipId`/`PARSE` local collisions with candidate drafts are unresolved;
- a work order needs a new record family/ID/result variant beyond §3.2.

---

## 8. Open Questions and Rejected Alternatives

### 8.1 Open questions

| ID | Question | Recommended V1 answer | Blocking? |
|---|---|---|---|
| O-01 | One fused V1 parser or taskified per-production parser? | One `PARSE_TU` task for V1.0; taskified parser is a later contract item because it needs parent/children allocation, continuation materialization, or record update. | Yes |
| O-02 | Keep `NodeRecord.children` or migrate to `ordinal`? | Keep `children`; no migration. | Yes |
| O-03 | Add `SemRecord.constant`/`SemRecord.symbol` or move links to `ConstRecord`/IR? | Add both links on `SemRecord`; keep `ConstRecord` value-only. | Yes |
| O-04 | Assign `EffectMask` bit 0 to local writes or exclude local writes? | Assign bit 0 `WritesLocal`; exclude unknown bits. | Yes |
| O-05 | Add `DiagGroup::Parse` or reuse `Semantic`? | Add `Parse`; if rejected, use `Semantic` 20+ with a written convention. | Yes |
| O-06 | Split IR into `IR_STATEMENT`/`IR_EXPRESSION` or one `IR_FUNCTION`? | Split with a statement chain; one giant lowering chip is rejected. | Yes |
| O-07 | Is `int`-only identity conversion sufficient for “conversions”? | Yes for V1.0; non-identity conversions need frozen widths (Part B) or a later type model. | Yes |
| O-08 | Which operators are V1.0? | `=`, `+ - * / %`, unary `+ -`, parentheses; comparisons/bitwise/shift/logical are V1.1/V2. | Yes |
| O-09 | Are multiple functions/global objects in V1? | No; one `main`, one TU. | Yes |
| O-10 | Is `return;`/fallthrough supported? | No; `return <expr>;` last statement only. | Yes |
| O-11 | Uninitialized local read policy? | Outside normative V1 values; VF12 fails closed as `Semantic/8`. | No |
| O-12 | `int x = x;`? | Legal syntax; lookup point resolves the new symbol; do not use the indeterminate value in acceptance. | No |
| O-13 | Cross-version snapshot decode? | Not required; gate by contract version/hash and reject explicitly. | No |
| O-14 | Contract version label/new hash? | Integrator chooses (`/40` proposed); do not compute in this report. | No |
| O-15 | Candidate draft arbitration (`PARSE` 27+, `ChipId(58)`)? | V1 claims none of them; integrator must retire/renumber before any later parser split. | Yes |
| O-16 | `SymbolRecord.decl` Identifier vs Declarator? | Identifier leaf (T06 item 6b); update tests and TY logic. | Yes |
| O-17 | Multi-appender append fields vs “one owner per field”? | Freeze the `(ChipId, field, kind)` allowlist model; disjoint records, deterministic commit order. | Yes |
| O-18 | Does `IR_FUNCTION` keep `ir.values`/`ir.instructions` rows? | No; narrow to functions/blocks, grant values/instructions to the new IR chips. | Yes |
| O-19 | Is a new `ResultValue` variant needed for VF12 values? | No; use `[Function, Const expected]` → `Ack`. | No |
| O-20 | Should `ConstRecord` gain `ty` for future unsigned constants? | Defer; V1 type authority is `SemRecord.ty`/`ValueRecord.ty`. | No |
| O-21 | Does V1 use the accepted `parse.TranslationUnit → symbol_type.scope-enter` stage edge/T06 item-7 `ResultRecord.consumed` claim? | Not in the V1 Part A harness: the driver enqueues scope-enter explicitly. A production exactly-once edge remains T06/T01 co-freeze and must be added before production wiring. | No (Part A) / Yes (production) |

### 8.2 Rejected alternatives

| Alternative | Why rejected for V1 |
|---|---|
| New `ConversionPlanRecord` family/ID | `ConversionPlan` can live inline in `SemRecord`; a family adds IDs/tags/snapshot/allowlist for no V1 benefit. |
| New `LocalRecord`/`ObjectRecord` family | `SymbolRecord` already carries object identity/type/storage; IR `local: SymbolId` is sufficient. |
| New `InitRecord`/layout plan for scalar init | V1 scalar initialization is a store of the initializer value; an init-plan family is T08 later work. |
| New `ParseResultRecord` or `ResultValue` cursor variant | The single parser task needs no cursor result; VF12 uses expected `Const` instead of a scalar result. |
| New `RecordRef`/`RecordFamily` tags | No new record family is needed; preserving wire tags avoids envelope/snapshot churn. |
| Taskified parser with continuations in V1 | Requires `G1DraftBody::Continuation`, a `TaskDraft.continuation` ref, parent/children allocation or record update, and a cursor carrier. None is `/39`; all are larger than the fused parser. |
| Parent-after-child parser with `children` updated later | `AppendRecords` has no record-body update/replace; `StorePatch.value` is a `RecordRef`, not a record body. A reservation protocol is not frozen. |
| `NodeRecord.ordinal` migration | `children` already stores ordered siblings; ordinal is redundant and breaks consumers/tests. |
| `TypeKind::Pointer` for local addresses | Pointers are V3; V1 local access is by `SymbolId`, not a C pointer value. |
| Non-identity conversions with symbolic `int` | C conversion value semantics require widths; claiming them without a probe would fabricate behavior. |
| Host `i64`/`u64`/pointer arithmetic as C `int` | Violates the frozen target-model rule and the Part A symbolic qualifier. |
| One giant SE/IR chip | Violates single-responsibility/guardrails; the proposed per-node SE chips and IR statement/expression split keep rules testable. |
| Reusing Ack-only PA/SE/IR shells as real producers | Ack is not AST/IR production; it would fake success and violate the dispatch gate. |
| `StorePatch` to mutate committed record bodies | Current patch value is a reference, not a body; no record update protocol is frozen. |
| Deleting/rewriting `/39` tests | Prohibited; tests must be explicitly updated or preserved as historical, with version/status recorded. |
| Adding `ConstRecord.ty`/`origin` immediately | `SemRecord.constant` solves V1 mapping; extra fields are deferred until unsigned/typed constants are in scope. |

---

## Appendix A: Proposed Rust-Like Shape Summary (PROPOSED, NOT FROZEN)

This block is a compact copy target for the integrator. It is not a source patch
and must not be pasted into `bus.rs` before F1 acceptance.

```rust
// AST: NodeKind append only; NodeRecord fields unchanged.
pub enum NodeKind {
    TranslationUnit, FunctionDefinition, Specifiers, Declarator, Compound,
    Return, BinaryAdd, IntLiteral,
    Declaration, Identifier, ExpressionStatement, Assignment,
    BinarySub, BinaryMul, BinaryDiv, BinaryRem, UnaryPlus, UnaryMinus,
}

// Semantic: append three fields and three inline types.
pub enum ConversionOp { Identity }
pub enum ConversionRole { Return, Assignment }
pub struct ConversionPlan {
    pub op: ConversionOp,
    pub role: ConversionRole,
    pub from: TypeId,
    pub to: TypeId,
}
pub struct SemRecord {
    pub node: NodeId,
    pub ty: TypeId,
    pub category: ValueCategory,
    pub effects: EffectMask,
    pub conversions: Vec<ConversionPlan>,
    pub constant: Option<ConstId>,
    pub symbol: Option<SymbolId>,
}
pub const EFFECT_WRITES_LOCAL: u32 = 1 << 0;

// IR: append ops and one field.
pub enum IrOp {
    Constant, Return,
    LocalLoad, LocalStore, Add, Sub, Mul, Div, Rem, Neg,
}
pub struct InstructionRecord {
    pub op: IrOp,
    pub block: BlockId,
    pub operands: Vec<ValueId>,
    pub immediate: Option<ConstId>,
    pub result: Option<ValueId>,
    pub local: Option<SymbolId>,
}

// New task kinds (proposed local codes).
pub const SYMBOL_ASSIGNMENT_CONVERT: TaskKind = /* SYMBOL_TYPE 25 */;
pub const SEMANTIC_IDENTIFIER_EXPR: TaskKind = /* SEMANTIC 22 */;
pub const SEMANTIC_ASSIGNMENT_EXPR: TaskKind = /* SEMANTIC 23 */;
pub const SEMANTIC_DECLARATION: TaskKind = /* SEMANTIC 24 */;
pub const SEMANTIC_UNARY_EXPR: TaskKind = /* SEMANTIC 25 */;
pub const IR_STATEMENT: TaskKind = /* IR_LOWER 17 */;
pub const IR_EXPRESSION: TaskKind = /* IR_LOWER 18 */;
```

**Unchanged:** `NodeRecord`, `ConstRecord`, `TypeKind`, `TypeRecord`, `SymbolRecord`,
`ScopeRecord`, `ScopeEventRecord`, `FunctionRecord`, `BlockRecord`, `ValueRecord`,
`Payload`, `ResultValue`, `Proposal`, `TaskDraft`, `ContinuationRecord`, `G1DraftBody`,
`RecordRef`, `RecordFamily`, `RECORD_KINDS`, `StoreId`, `StoreSchema` fields, all
wire tags.

---

## Appendix B: Conflict Register (`/39`, candidate drafts, V1)

| ID | Conflict | Current `/39` | V1 need | Recommended resolution | Owner |
|---|---|---|---|---|---|
| C-01 | AST field shape vs T05 candidate | `NodeRecord.children` (7 fields) | General ordered children | Keep `children`; reject T05 `ordinal` for V1 | T05/T01 |
| C-02 | AST kind names vs T05 candidate | `Specifiers`, `Compound`, `IntLiteral`, `BinaryAdd` | General expressions/declarations | Append new names; keep frozen names | T05/T01 |
| C-03 | `SemRecord` conversions/links | 4 fields, no conversions | Assignment/return plan; T09 symbol link | Append `conversions`/`constant`/`symbol` | T07/T01 |
| C-04 | `EffectMask` | 0 only | Assignment writes local | Assign bit 0 `WritesLocal`; supersede `sem.effect-mask-zero-only` | T07/T01 |
| C-05 | `ConstRecord` mapping | Value-only, no origin/type | Multiple constants | `SemRecord.constant` link; keep `ConstRecord` | T08/T07/T01 |
| C-06 | `IrOp`/`InstructionRecord` | Constant/Return, no local | Locals/arithmetic | Append ops + `local` field | T09/T01 |
| C-07 | TY declare | Function/file only | Object/block | Generalize payload/behavior | T06/T01 |
| C-08 | TY lookup query point | `[Scope, Name]` | C point-of-declaration | Add `Token` query ref | T06/T05/T01 |
| C-09 | Parser production | Fixed 13-token, 9-node Ack tree | Real V1 AST | One fused V1 `PARSE_TU`; update `/12` test | T05/T01 |
| C-10 | Continuation materialization | `G1DraftBody` lacks `Continuation`; `TaskDraft` committed-only | Taskified parser would need it | Not needed for V1 fused parser; defer | T01/T05 |
| C-11 | Sem/const stage rows | Sem const 1, fold 2, sem 4 | Sem→const→IR | Remap to 4/4 or document legacy exception | T01/T02 |
| C-12 | Diagnostic group | No parse/syntax group | Syntax diagnostics | Add `Parse` or documented `Semantic` codes | T01 |
| C-13 | Candidate draft IDs | `PARSE` 27+, `ChipId(58)` claimed by 7 drafts | V1 may need none | V1 claims no PA local/chip; integrator arbitrates later | Integrator |
| C-14 | Tests pin old shapes | c12/c13/c14/c15 and PA/SE slice tests c34–c36/c39 | V1 shapes | Explicit test updates; preserve history; no deletion | Integrator/owners |
| C-15 | Hash scope | Later task-kind names not all hashed; conversion types absent | New shapes/rules hashed | Extend `FrozenSchema`/seed and rule list | T01 |
| C-16 | Symbolic `int` | No target width | Non-identity conversion values | V1 identity only; Part B for widths | T01/T06 |
| C-17 | “One owner per field” wording | Multiple `sem.records`/`ir.*` appenders | New SE/IR appenders | Freeze `(ChipId, field, kind)` allowlist as owner unit | T01 |
| C-18 | `SymbolRecord.decl` | Current code uses Declarator | T06 says Identifier leaf | V1 uses Identifier; update code/tests | T06/T05 |
| C-19 | T08/T09 Const handoff | Single folded const assumption (`ir_function.rs:223-241`) | Multiple consts | `SemRecord.constant` per node; exact one record per request | T08/T09 |
| C-20 | `ConstRecord` proposal vs `/39` | `/39` magnitude+negative; proposal `{ty,value:i128}` | Keep `/39` | Do not adopt proposal carrier in V1 | T08/T01 |
| C-21 | Closed `ResultValue` | No scalar/cursor result | VF12 value; parser cursor | Expected-const match; fused parser; no new variant | T13/T05/T01 |
| C-22 | `ValueCategory`/effect candidate | `/39` 4 categories; effect 0 | Lvalue/Void + local write | Keep categories; extend effect bit 0 | T07/T01 |
| C-23 | File Enter edge / `ResultRecord.consumed` claim | T06 item-7 candidate; T05 item-F edge; no V1 driver | V1 Part A needs scope enter | V1 harness enqueues scope-enter explicitly; production exactly-once edge/claim remains T06/T01 co-freeze | T06/T05/T01 |
| C-24 | `ScopeRecord` shape/owner | `/39` `{kind,parent,owner}`, file owner `None` | T06 item-6a candidate `{parent,kind}`; item-6b says file owner TU root | Keep `/39` 3 fields and current file-owner behavior for V1; reconcile T06 candidate separately | T06/T01 |

---

## Appendix C: Source Evidence Index (read-only)

| Topic | Path / lines |
|---|---|
| Frozen version/hash | `compiler/contracts/CONTRACT_VERSION`; `compiler/src/contract.rs:38-44` |
| Normative rules | `compiler/src/contract.rs:51-241` |
| AST/records | `compiler/src/bus.rs:283-602` |
| Sem/IR records | `compiler/src/bus.rs:422-513` |
| Task/result/proposal | `compiler/src/task.rs:1245-1503,1546-1628,1676-1990` |
| IDs/families/tags | `compiler/src/ids.rs:57-96,119-178,343-400`; `compiler/src/task.rs:2185-2213` |
| Draft body materialization | `compiler/src/records.rs:38-122`; `compiler/src/commit.rs:663-788,1038-1123,1176-1193` |
| Manifest/stage/allowlist | `compiler/src/manifest.rs:149-380,784-978,1444-1514` |
| Snapshot encoders | `compiler/src/snapshot.rs:1640-1728,2141-2318` |
| Current parser | `compiler/src/chips/parse/pa_tu.rs:154-365`; `pa_external.rs`, `pa_specifier.rs`, `pa_declarator.rs`, `pa_block.rs`, `pa_pod.rs`, `pa_binary.rs`, `pa_unary.rs`, `pa_recovery.rs` |
| Current TY | `compiler/src/chips/symbols/ty_scope.rs`, `ty_symbol.rs`; `compiler/src/chips/types/ty_types.rs`, `ty_conv.rs` |
| Current SE | `compiler/src/chips/semantic/se_literal.rs`, `se_binary.rs`, `se_return.rs`, `se_function.rs` |
| Current CL | `compiler/src/chips/constant_layout_init/fold.rs` |
| Current IR | `compiler/src/chips/ir_lower/ir_function.rs` |
| Current VF | `compiler/src/chips/verify/vf_invariant.rs`, `vf_interpret.rs`, `vf_syntax.rs`, `vf_store.rs` |
| Current tests | `compiler/tests/c12_parse.rs`, `c13_ty.rs`, `c14_se.rs`, `c15_ir.rs`, `c17_vf12.rs`, `c18_vf05.rs`, `c34_parse.rs`, `c35_expr.rs`, `c36_recovery.rs`, `c39_sefunc.rs`, `freeze.rs` |
| Candidate drafts | untracked `compiler/src/chips/parse/pa_cast.rs`, `pa_decl_spec.rs`, `pa_expr_ext.rs`, `pa_ext_dispatch.rs`, `pa_init.rs`, `pa_stmt.rs`, `pa_typedef_query.rs`; `compiler/src/chips/lex/lx_error.rs`, `lx_extension.rs` |
| V1 scope | `docs/tasks/MASS_PARALLEL_DISPATCH.md` §6.2 |
| T01 baseline | `docs/tasks/T01_COMPILER_CONTRACT.md` |
| T05 parser candidate | `docs/tasks/T05_PARSE_CHIPS.md` items A–H |
| T06 lifecycle | `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md` items 1–8 |
| T07 semantic checklist | `docs/tasks/T07_SEMANTIC_CHIPS.md` S1–S7, §4 |
| T08 constant handoff | `docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md` |
| T09 IR checklist | `docs/tasks/T09_IR_LOWER_CHIPS.md` |
| M1 proposal records/IR | `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md` §5, §6.4 |
| Guardrails | `docs/tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md` §3, §5, §6 |
| Dispatch ledger baseline | `docs/tasks/DISPATCH_LEDGER.md` §1 (external observation; not re-run here) |

---

**End of proposal.** No implementation, test execution, probe, corpus, or pass-rate
claim is made. The `/39` artifact remains current until the integrator accepts a
V1 freeze and recomputes the version/hash.
