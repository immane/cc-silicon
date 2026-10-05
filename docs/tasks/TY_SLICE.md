# Wave 2 Slice 4: TY Scope/Symbol/Type Freeze (`/13`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-05** as `t01-c01-c06/13` (hash `0de77f06…5fa6d`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/12` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/13` delta), [T06](T06_SYMBOL_TYPE_CHIPS.md) items 1–2 accepted + M1 candidate items, [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-TY-01`–`M1-TY-11` (selected rows) |
| Non-goal | Pointer/array/aggregate/enum/qualified types, typedefs, tags/labels, non-identity conversions, `ConversionPlan`, query-point lookup ordering, automatic File-Enter edge |

## 1. What this slice is

Four workers closing the M1 name/type/scope loop on committed records:
`TyTypeChip` (canonical `int` + `int(void)`) → `TyScopeChip` (file/body
enter + exit) → `TySymbolChip` (declare `main` + lookup) → `TyConvChip`
(identity-only promote/common/return). The `main` type flows
`int → func → declare`; lookup resolves it back. The acceptance fixture
is **`P1-TY-01`** (below). The automatic File-Enter edge stays deferred:
tests dispatch scope tasks after the TU commit (ordering holds and
exactly-once is worker-guarded, so `M1-START-01`'s firing half is still
open).

## 2. Frozen decisions

| # | Decision | Rationale |
|---|---|---|
| Y1-1 | `TypeRecord { kind }`, `TypeKind {Void, Bool, Char(CharKind), Int{rank,signed}, Function{result,params,prototype,variadic}}`, `IntRank`, `CharKind{Plain,Signed,Unsigned}`; M1 produces `Int{Int,true}` + `int(void)` only | T06 item 6a shape; symbolic, no widths |
| Y1-2 | Canonical reuse = deterministic bounded scan, lowest match, no cache (implemented in the projector) | T06 item 1 accepted; TC-01 predicate |
| Y1-3 | `ScopeRecord {kind,parent,owner}`, `ScopeKind{File,Block}`, `ScopeEventRecord {scope,kind,at}`; owner-node identity, never `(parent,kind)` | T06 6a + DOC-13 fix |
| Y1-4 | Enter-once per scope, exit-after-enter, at most one exit, file never exits; second file `Enter` fails (exactly-once guard) | T06 6b lifecycle direction, worker-enforced |
| Y1-5 | `SymbolRecord {name,scope,kind,ty,linkage,storage,decl}` (`decl` = the `Declarator` node: no leaf nodes frozen); `SymbolKind{Function,Object}`; duplicate `(name,scope,kind)` fails as chip `RedeclarationConflict`, never merged | T06 6a shape; 6c conflict direction |
| Y1-6 | Lookup walks the active chain innermost-first, ordinary-namespace hits only, same-scope tie → higher `SymbolId`; miss fails with a typed `Semantic` undeclared-identifier error (M1-NEG-14 carrier, never `Unsupported`) | T06 item 3 direction; query-point ordering deferred (one declarator) |
| Y1-7 | Identity conversions complete with the unchanged `TypeId`, no plan recorded; non-`Int` operands are explicit `Unsupported` | OB-51 identity-only; TC-02 (M1 records nothing) |
| Y1-8 | Nine TY kinds (locals 16–24), all stage 3; four chips (8–11); six allowlist rows; `ty_slice()` registry (21 entries) | Slice registration |
| Y1-9 | `Type`/`Symbol`/`Scope`/`ScopeEvent` append materialization; `Intern` protocol-18 error; future `Name`/`ScopeEvent` refs rejected; `Semantic` diagnostic group | Commit/snapshot/hash closure |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c13_ty.rs`
(8 tests: kind/stage/allowlist freeze, int reuse + func shape, file-once +
body lifecycle + exit guards, declare/conflict/lookup hit/miss with
`Semantic` codes, identity conversions + non-`Int` unsupported, CharKind
vocabulary, stage/layer + manifest gates, snapshot replay determinism).
Chain inputs come from the real LX+PA slices; sources stay seeded.

## 4. Explicitly deferred

- Automatic File-Enter edge firing (ordering + guards delivered; firing stays T02/T06 open).
- Query-point lookup ordering (`int n[n]` contrast needs declarator-completion positions).
- Commit-side `SymbolConflict` + node/scope link validation (worker-enforced; T13 VF01/VF05/VF06 pending).
- `ConversionPlan`, non-identity conversions, full type inventory, char producers.
- Scope exit for files; namespaces beyond ordinary; `M1-TY-11` char-declaration fixture.
