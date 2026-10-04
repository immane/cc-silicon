# M1 Frontend Acceptance: Fixtures for `int main(void){return 2+3;}`

Status: **planned test design, not implemented capability.**

This document specifies the independent frontend fixtures for the M1 vertical
slice. It is a design artifact for the preprocessing, lexical, parse/declarator,
symbol/type, integer-semantics, and constant-evaluation stages of
`int main(void){return 2+3;}`.

The following are explicitly true of this document:

- **No tests were executed.** Every expected result below is hand-derived from
  ISO C rules and the frozen target *identity*; nothing here is a measured
  result.
- **It is not evidence that any chip, bus, or compiler exists.** The compiler
  application is planned work under `docs/tasks/`; this document does not create
  code, interfaces, or directories.
- **Language schemas and record IDs remain provisional.** The C01–C06
  foundation envelope is recorded as `t01-c01-c06/5`, but it does not freeze
  language-store record bodies or per-group task/result payloads. IDs such as
  `TokenId`, `TypeId`, `NodeId`, `ConstId`, and `DiagnosticId`, and catalog
  labels such as `PP04`, `LX08`, `PA22`, do not become binding for language
  fixtures until their owning contracts are frozen.
- **The M1 language mode is declared per fixture; it is not inferred as the
  compiler default.** See section 4.
- **Concrete ABI values are unverified.** Target identity is frozen
  (`aarch64-unknown-linux-gnu`, ELF, LP64, little-endian, AAPCS64), but every
  scalar width/alignment, `long double` format, and ABI register value remains
  UNVERIFIED until the planned Linux probe reports `verified=true`
  ([ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md)).
- **No GCC/Clang candidate compilation and no pass-rate claim.** No reference
  compiler output is used to derive an expected value, and M1 fixtures are not
  the T00 GCC torture rate.

This is a **draft/planned** acceptance document. It may be superseded when T01
freezes, but it must not be read as a claim that the described behavior is
implemented.

---

## 1. Scope

This document covers the M1 frontend only:

- translation phases 1–3 (preprocessing) for the canonical source,
- final C tokenization of that source,
- parsing and declarator binding for the single function definition,
- symbol/type construction for `main` and its function type,
- integer semantics of `2 + 3`,
- constant evaluation of the integer constant expression,
- malformed/unsupported/replay/write-scope robustness cases.

Downstream IR, ABI, and assembly acceptance is mentioned only as a gate
(`M1-INT-01`); it is owned by T09/T11 and is not specified here. The full M1
end-to-end milestone definition lives in
[docs/tasks/README.md](README.md) section 4 and
[T00_GCC_TORTURE_GATE.md](T00_GCC_TORTURE_GATE.md).

Out of scope for this fixture set: every C feature not needed by the canonical
source. Unsupported-feature fixtures in section 6.2 exist only to prove that
out-of-M1 input fails explicitly, not to enumerate the feature catalog.

## 2. Authority and precedence

Read together with:

- repository rules: [AGENTS.md](../../AGENTS.md)
- compiler task master plan: [docs/tasks/README.md](README.md)
- contract and protocol: [T01_COMPILER_CONTRACT.md](T01_COMPILER_CONTRACT.md)
- parallel/handoff discipline: [PARALLEL_EXECUTION.md](PARALLEL_EXECUTION.md)
- per-chip template: [TASK_TEMPLATE.md](TASK_TEMPLATE.md)
- acceptance gate: [T00_GCC_TORTURE_GATE.md](T00_GCC_TORTURE_GATE.md)
- framework semantics: [SILICON_PARADIGM_SPEC.md](../architecture/SILICON_PARADIGM_SPEC.md),
  [SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md),
  [SFL_SCHEMA_DRAFT.md](../architecture/SFL_SCHEMA_DRAFT.md)
- design: [ARCHITECTURAL_BLUEPRINT.md](../design/ARCHITECTURAL_BLUEPRINT.md)
- storage/target decision: [ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md)

Precedence follows AGENTS.md section 2: explicit user instruction, applicable
accepted ADR, accepted engineering contract, architecture design, then existing
implementation. Because T01 is not frozen, this document cannot bind chip
interfaces; if a fixture conflicts with a later T01 freeze, the freeze wins and
the fixture is updated in one place.

Expected values are derived from ISO C clause behavior (cited informally, for
example "ISO C 6.7.6.3"), not from any vendored standard text. No ISO C standard
document is stored in this repository.

## 3. Canonical source and boundary variants

| Fixture | Exact bytes | Length | Role |
|---|---|---:|---|
| `M1-SRC-000` | `int main(void){return 2+3;}` + `\n` | 28 | Canonical M1 source (matches the milestone spelling) |
| `M1-SRC-001` | canonical without the final `\n` | 27 | Missing-final-newline boundary |
| `M1-SRC-002` | canonical with `\r\n` | 29 | Line-ending-normalization boundary |
| `M1-SRC-003` | `int main(void) { return 2 + 3; }` + `\n` | 33 | Whitespace-invariance boundary; must yield the same semantics as the canonical source with different spans |
| `M1-SRC-004` | `int main(void){return 2+ +3;}` + `\n` | 29 | Two separate `+` punctuators, never a `++` token |

Byte layout of `M1-SRC-000`, 0-based half-open spans:

```text
0..3 int | 3 sp | 4..8 main | 8..9 ( | 9..13 void | 13..14 ) | 14..15 {
15..21 return | 21 sp | 22..23 2 | 23..24 + | 24..25 3 | 25..26 ;
26..27 } | 27..28 \n
```

Whitespace and newline handling is part of the preprocessing fixtures; span
values are location records, not semantic tokens, and must never determine
semantics (T03 acceptance, T04 tail note).

## 4. M1 language-mode policy

To keep the default dialect unsettled, every fixture in this document declares
its mode explicitly. No fixture may rely on `CompilerConfig::default()` or any
other implicit dialect choice.

- The M1 frontend fixtures are written against a **declared** mode in the
  ISO C family. The canonical source
  `int main(void){return 2+3;}` is intentionally chosen so that the frontend
  result is invariant across C89, C99, C11, C17, and C23 (and their GNU
  variants) for the specific facts listed below. It does not follow that the
  compiler's default is C11 or any other value; [T00](T00_GCC_TORTURE_GATE.md)
  section 2 requires the default to be confirmed by reference-release probes.
- Invariant across those modes for this source: keyword identity of `int`,
  `void`, and `return`; `(void)` as a prototype with zero parameters; decimal
  integer literal type selection for 2 and 3; integer promotion and usual
  arithmetic conversion of `int + int`; and `2 + 3 == 5`.
- Mode-sensitive fixtures in section 6 are labeled `T01 + dialect policy` and
  may not be assigned an expected result until the mode is declared and the
  default-dialect question is settled. They exist to force an explicit choice,
  not to assume one.
- The fixtures assert **no** macro dialect default, predefined-macro set, or
  command-line flag default.

## 5. Frontend expected semantics

The Gate column means:

- `T01 freeze` — executable only after the T01 schema/types/task protocol freeze.
- `T01 + target probe` — additionally requires the AArch64 probe to be verified.
- `T01 + dialect policy` — additionally requires an explicit declared mode.
- `T01 + overflow policy` — outside the normative pass criteria (section 9).

Each row is independent: it feeds the chip under test from a hand-built input
record and does not require the full upstream chain. Full-chain behavior is the
single integration row `M1-INT-01`.

### 5.1 Preprocessing (owner T03)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-PP-01` | PP01 | `M1-SRC-000` | normalized bytes equal input; byte-to-location map is identity; one logical line; final LF present | T01 source/span records | T01 freeze |
| `M1-PP-01b` | PP01 | `M1-SRC-001` | LF appended (27 to 28 bytes); "newline inserted" provenance; content otherwise identical | T01 records | T01 freeze |
| `M1-PP-01c` | PP01 | `M1-SRC-002` | CRLF mapped to LF; logical columns map to physical bytes | T01 records | T01 freeze |
| `M1-PP-02` | PP02 | normalized canonical | identity; zero line-splice records (no backslash-newline) | PP01 output | T01 freeze |
| `M1-PP-03` | PP03 | spliced bytes | identity; comment-free; logical line count 1 | PP02 output | T01 freeze |
| `M1-PP-04` | PP04 | comment-free bytes | pp-tokens in order with spans: `int`[0,3) `main`[4,8) `(`[8,9) `void`[9,13) `)`[13,14) `{`[14,15) `return`[15,21) `2`[22,23) `+`[23,24) `3`[24,25) `;`[25,26) `}`[26,27) EOF[28,28); maximal munch | PP03 output | T01 freeze |
| `M1-PP-05` | PP05, PP09, PP17, PP19, PP25 | pp-token sequence | no directive, macro, include, conditional frame, or pragma recognized; no child requests | T01 records | T01 freeze |
| `M1-PP-06` | PP28 | final pp tokens | emits exactly the 12 tokens above plus EOF; no accidental token joining; origin chain length 1 | PP row above | T01 freeze |
| `M1-PP-07` | PP04, PP28 | `M1-SRC-004` | the body scans as `return`, `2`, `+`, `+`, `3`, `;`; the adjacent plus signs stay two punctuators and are never glued into `++` | T01 records; T03 PP28 acceptance | T01 freeze |

### 5.2 Tokenization (owner T04)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-LX-01` | LX01, LX03, LX04 | 12 pp-tokens | kinds: keyword `int`, identifier `main`, punct `(`, keyword `void`, punct `)`, punct `{`, keyword `return`, integer `2`, punct `+`, integer `3`, punct `;`, punct `}`, unique EOF | PP04 output | T01 freeze |
| `M1-LX-02` | LX02 | `main` | one interned name; identical spelling yields the same name record; `main` is not coerced to a keyword | T01 name store | T01 freeze |
| `M1-LX-03` | LX03 | `int`, `void`, `return` | keywords in every ISO C89 to C23 and GNU variant listed in section 4; classification is mode-invariant for these three spellings | declared mode | T01 freeze |
| `M1-LX-04` | LX05, LX06, LX07, LX08 | token `2` | decimal radix; no suffix; value 2; decimal no-suffix candidate list `int`, `long`, `long long`; value fits `int`; signed `int`; bit pattern `0x00000002` | frozen target integer model | T01 + target probe |
| `M1-LX-05` | LX05, LX06, LX07, LX08 | token `3` | same as `M1-LX-04`, value 3, bit pattern `0x00000003` | frozen target integer model | T01 + target probe |
| `M1-LX-06` | LX16, LX17 | 12 tokens | tokens published in source order; spans preserved; exactly one EOF; stable token IDs | T01 token records | T01 freeze |
| `M1-LX-07` | LX01, LX04, LX17 | `M1-SRC-003` | same token kinds as `M1-LX-01`, spellings preserved, only spans differ; whitespace does not change meaning | PP output of variant | T01 freeze |

### 5.3 Parsing and declarator (owner T05)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-PA-01` | PA01 | token range | exactly one external declaration, then EOF; explicit progress each step; no idle spin | T04 tokens | T01 freeze |
| `M1-PA-02` | PA02 | `int main(void){` | classified as a function definition (specifier + declarator + body), not a declaration | tokens; specifier service | T01 freeze |
| `M1-PA-03` | PA03 | `int` | specifier bundle: type specifier `int`; no storage class, qualifier, function specifier, or alignment | T06 TY13 | T01 freeze |
| `M1-PA-04` | PA05, PA06, PA07, PA09 | `main ( void )` | declarator tree: direct declarator naming `main` with one function suffix; params = prototype with zero parameters; variadic false; no pointer chain. `(void)` is the ISO C 6.7.6.3 special "no parameters" form, not one `void` parameter | PA04 query that `main` is an ordinary identifier and not a typedef | T01 freeze |
| `M1-PA-05` | PA28 | `{ return 2+3; }` | one block; scope enters at `{` and exits at `}`; scope balance returns to zero | T06 TY01/TY02 | T01 freeze |
| `M1-PA-06` | PA32 | `return 2+3 ;` | return statement with an expression present (not `return;`) | PA24 | T01 freeze |
| `M1-PA-07` | PA16, PA22 | `2+3` | binary node, additive `+`, lhs integer constant `2`, rhs integer constant `3`, additive precedence level; one operator so associativity is not observable | T04 tokens | T01 freeze |
| `M1-PA-08` | PA20, PA22 | `2+ +3` | valid: the second `+` is unary plus; no diagnostic; no `++` token exists | T04 tokens | T01 freeze |
| `M1-PA-09` | PA09 | `main()` variant | an empty identifier list is "no prototype information" for a declaration and must not be silently equated with `(void)` for later call checking | declared mode; T06 TY17 | T01 + dialect policy |

### 5.4 Symbol and type (owner T06)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-TY-01` | TY13 | `int` | canonical signed integer scalar; target rank between `short` and `long`; width and alignment only from the frozen target model (probe currently unverified) | frozen target integer model | T01 + target probe |
| `M1-TY-02` | TY17 | `int (void)` | function type: result `int`, parameters empty, prototype true, variadic false; not old-style | `M1-TY-01` | T01 freeze |
| `M1-TY-03` | TY20 | base `int` + declarator | declared type is a function returning `int` with an empty prototype; must not become pointer-to-function or array | `M1-PA-04`, `M1-TY-01/02` | T01 freeze |
| `M1-TY-04` | TY07, TY10 | specifiers + declarator | register `main` in the ordinary namespace at file scope, kind function, linkage external by default, type from `M1-TY-03` | scope service; `M1-TY-03` | T01 freeze |
| `M1-TY-05` | TY01, TY02 | body scope operations | scope tree: file scope, function-prototype scope for `(void)` with no named parameters, function-body block scope; every exit balances; records retained for typed AST use | scope service | T01 freeze |
| `M1-TY-06` | TY03 | lookup `main` | found in file scope after declaration with the function type; missing before the point of declaration | `M1-TY-04` | T01 freeze |
| `M1-TY-07` | TY25 | types `int`, `int` | integer promotion of `int` yields `int`, unchanged | frozen target integer model | T01 + target probe |
| `M1-TY-08` | TY26 | types `int`, `int` | usual arithmetic conversion yields common type `int`, no inserted casts | `M1-TY-07` | T01 freeze |
| `M1-TY-09` | TY27 | source `int`, destination `int` | return conversion is identity; no diagnostic | `M1-TY-01/03` | T01 freeze |
| `M1-TY-10` | TY09 | first declaration of `main` | no prior declaration, no merge, one definition, no duplicate-definition diagnostic | `M1-TY-04` | T01 freeze |

### 5.5 Integer and expression semantics (owner T07)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-SE-01` | SE02 | literal nodes `2`, `3` | each has type `int` and value category non-lvalue (an integer constant is not an lvalue); no string or enum involvement | `M1-LX-04/05`, `M1-TY-01` | T01 freeze |
| `M1-SE-02` | SE07 | `2+3` | operand types `int`, `int`; result type `int`; no inserted conversions; non-lvalue; signed addition is well-defined because 2 + 3 is representable | `M1-TY-07/08` | T01 freeze |
| `M1-SE-03` | SE21 | `return <expr>` in `int(void)` | function is non-void and the expression is present; conversion `int` to `int` is identity and valid | `M1-TY-09` | T01 freeze |
| `M1-SE-04` | SE29 | function definition | zero parameters, not K&R; body checked; definition consistent with `M1-TY-03` | `M1-TY-03`, `M1-PA-02` | T01 freeze |
| `M1-SE-05` | SE26 | checked additive tree | no side effects and no unsequenced modifications; the program is well-defined under the frontend model (no UB fact emitted) | `M1-SE-02` | T01 freeze |

### 5.6 Constant evaluation (owner T08)

| Fixture | Chip(s) | Input | Hand-derived expected result | Depends on | Gate |
|---|---|---|---|---|---|
| `M1-CL-01` | CL01 | `2+3` context | legal integer constant expression: integer constants and additive operator only, no assignment, comma, or call | `M1-SE-02` | T01 freeze |
| `M1-CL-02` | CL03 | add of 2 and 3 | arithmetic in target `int` width; value 5; type `int`; bit pattern `0x00000005`; no overflow | `M1-CL-01`, target model | T01 + target probe |
| `M1-CL-03` | CL05 | `int` to `int` on 5 | identity conversion; value 5; type `int` | `M1-CL-02` | T01 freeze |
| `M1-CL-04` | — | coverage note | `CL04`, `CL06`, `CL07`, `CL20`–`CL26`, and all layout/initializer chips are unexercised by M1 and must be reported as gaps, not passes | — | not applicable |

## 6. Robustness fixtures

Every malformed case must produce a structured diagnostic and a finite recovery.
Every unsupported case must produce an explicit unsupported or failed result.
Neither may panic, loop, silently succeed, or fabricate a value
([AGENTS.md](../../AGENTS.md) section 4.2; T01 section 4).

### 6.1 Malformed input

| Fixture | Source variant | Chip(s) | Hand-derived expected result | Gate |
|---|---|---|---|---|
| `M1-NEG-01` | `... 3; } /*` unterminated comment | PP03, PP26 | one unterminated-comment diagnostic; recovery consumes to EOF; finite | T01 freeze |
| `M1-NEG-02` | `#foo` line before the source | PP05, PP26 | unknown or unsupported directive diagnostic; macro text must not become a new directive | T01 freeze |
| `M1-NEG-03` | `#if 1` with no `#endif` | PP19 | structural conditional diagnostic at EOF; conditional frame drained | T01 freeze |
| `M1-NEG-04` | `int main(void){return 2@3;}` | LX01, LX18 | one unknown-character diagnostic at the `@` span; error token preserves spelling; advance exactly one byte; no loop | T01 freeze |
| `M1-NEG-05` | `int main(void){return 1e+foo;}` | LX09 | maximal munch makes `1e+foo` one pp-number; one malformed-float-syntax diagnostic; it is not split into `1`, `e`, `+`, `foo` | T01 freeze |
| `M1-NEG-06` | unterminated `"abc` or `'a` | LX12, LX13 | unterminated-literal diagnostic with span; finite recovery | T01 freeze |
| `M1-NEG-07` | `int main(void){return 18446744073709551616;}` | LX07, LX08 | value exceeds every type in the decimal no-suffix candidate list; range or constraint diagnostic; no wrap or truncation | T01 + target probe |
| `M1-NEG-08` | `int main(void){return 2+3}` (missing `;`) | PA32, PA34, PA38 | one expected-`;` diagnostic; recovery synchronizes at `}`; block closes; no phantom second declaration | T01 freeze |
| `M1-NEG-09` | `int main(void){return 2+3;` (missing `}`) | PA28, PA38 | unterminated-compound diagnostic at EOF; scope cleanup exactly once | T01 freeze |
| `M1-NEG-10` | `int main(void{return 2+3;}` | PA09, PA38 | malformed parameter list diagnostic; `void {` is not accepted as a parameter; recovery to `{` or `;` | T01 freeze |
| `M1-NEG-11` | `int main(void){return 2 3;}` | PA22, PA34, PA38 | expected-operator-or-`;` diagnostic; recovery at `;`; one error only | T01 freeze |
| `M1-NEG-12` | `int f(void){return 2+3;} = 0;` or `int f(void) = 0;` | PA02, TY31 | function definition or declaration cannot carry an object initializer; constraint diagnostic | T01 freeze |
| `M1-NEG-13` | `int f(void x);` | TY17, TY31 | a named parameter of type `void` is a constraint violation; only bare `void` as the sole item is the no-parameter form | T01 freeze |
| `M1-NEG-14` | `int main(void){return x+3;}` | SE01 | mode-sensitive: strict C99 and later is a constraint violation; GNU89-style implicit declaration is a warning/extension path. Expected result is chosen only after the mode is declared | T01 + dialect policy |
| `M1-NEG-15` | `int main(void){return;}` | SE21 | mode/policy-sensitive: returning without a value from an `int` function. Must not be silently accepted or rejected without a declared policy | T01 + dialect policy |
| `M1-NEG-16` | `int main(void){return 2147483647+1;}` | CL02, CL03, SE07 | signed-overflow behavior is deliberately outside this fixture's normative pass criteria; see section 9. If treated as an integer constant expression it is out of range; otherwise it is undefined absent a wrap policy | T01 + overflow policy |
| `M1-NEG-17` | `int main(void){return 5/0;}` | CL03 | division-by-zero diagnostic; not a valid constant expression; no crash | T01 freeze |
| `M1-NEG-18` | `int main(void){return 1<<40;}` | CL03 | shift count at or above the operand width yields a diagnostic; no host shift is used | T01 + target probe |

### 6.2 Unsupported features

Each row must yield an explicit unsupported or failed result, never a
successful compile and never a fabricated artifact. The concrete result
encoding is provisional (section 9).

| Fixture | Variant | Expected frontend outcome |
|---|---|---|
| `M1-UNS-01` | any `float`, `double`, or `long double` use, e.g. `return 2+3.0;` | explicit unsupported result; the conversion is well-defined C but outside the M1 subset |
| `M1-UNS-02` | `struct`, `union`, or `enum` use | explicit unsupported result |
| `M1-UNS-03` | pointer or array declarator, e.g. `int main(int argc, char **argv)` | explicit unsupported result; must not be silently mis-parsed as M1 syntax |
| `M1-UNS-04` | variadic `int f(int, ...)` | explicit unsupported result |
| `M1-UNS-05` | old-style definition with an empty parameter list in a definition | explicit unsupported or an explicitly modeled no-prototype function; must not be conflated with `(void)` |
| `M1-UNS-06` | `#include` or a function-like macro | explicit unsupported result unless a no-op capability is separately declared; no silent ignore |
| `M1-UNS-07` | GNU attribute, `typeof`, statement expression, or inline `asm` | explicit unsupported result |
| `M1-UNS-08` | `void main(void)` | mode/policy decision; ISO C requires `int main`, so this must not be silently accepted as conforming |
| `M1-UNS-09` | a task kind that is not implemented | the foundation `control.unsupported` path must fail explicitly, never return success |

### 6.3 Replay and determinism

| Fixture | Scenario | Hand-derived expected result |
|---|---|---|
| `M1-REC-01` | same input, config, and initial state replayed | identical record IDs, diagnostic order, and per-tick trace; scheduling and ID allocation use only documented ordering, never hash-map iteration or addresses |
| `M1-REC-02` | a child task fails during parse or type construction | the parent fails exactly once; exactly one diagnostic per primary error; no duplicate completion |
| `M1-REC-03` | new task enqueued in tick T | ready in T+1 by default; no same-tick re-entry; selection order is phase priority, then enqueue ordinal, then task ID |
| `M1-REC-04` | malformed or illegal bytes streamed through the lexer | bounded ticks, no infinite loop, no panic; progress is finite |
| `M1-REC-05` | recovery after a bad token followed by a valid function | recovery synchronizes to a declared boundary and does not swallow the following function |

### 6.4 Write-scope and access contract

| Fixture | Scenario | Hand-derived expected result |
|---|---|---|
| `M1-WS-01` | a worker proposes a patch to an undeclared store field | the proposal is rejected; no store mutation occurs |
| `M1-WS-02` | a worker proposes a patch with a wrong owner or stale version | the proposal is rejected; a failed batch commits nothing |
| `M1-WS-03` | a worker receives a task kind it does not own | semantic state is unchanged except for the scheduling diagnostic |
| `M1-WS-04` | manifest audit across the M1 chip set | every chip declares exact reads/writes, phase, category, and backend class; a chip must not be granted read or write of a whole store (`VF04`, `T01` section 5) |
| `M1-WS-05` | adapter projection test | changing a declared output field succeeds; undeclared registers and undeclared fields stay byte-for-byte unchanged |

## 7. Owners, dependencies, and readiness gates

Owner packages and the producer facts they consume (data dependencies, never
chip-to-chip calls):

| Phase | Owner package | Primary chips | Depends on |
|---|---|---|---|
| Preprocessing | T03 | PP01–PP05, PP09, PP17, PP19, PP25, PP28 | T01 source/pp records; CT02/CT08 host import |
| Tokenization | T04 | LX01–LX08, LX16, LX17 | T03 output; target integer model |
| Parsing / declarator | T05 | PA01–PA09, PA16, PA20, PA22, PA24, PA28, PA32, PA34, PA38 | T04 tokens; T06 TY01/TY02/TY17; PA04 query |
| Symbol / type | T06 | TY01–TY13, TY17, TY20, TY25, TY26, TY27, TY31 | T04 names; T01 type/symbol stores |
| Integer semantics | T07 | SE01, SE02, SE07, SE21, SE26, SE29 | T05 AST; T06 types; T08 constant context |
| Constant evaluation | T08 | CL01, CL03, CL05 | T06 types; T07 checked facts; target model |
| Control, diagnostics, commit | T02 | CT03–CT07, CT11, CT12 | T01 task/result protocol |
| Replay, access, evidence | T13 | VF01–VF06, VF13, VF14 | all M1 stages |
| Integration downstream | T09, T11 | IR01, IR19, CG04, CG09, CG33, CG35, CG37 | T01 freeze, target probe, frozen IR/ABI |

Readiness tiers:

| Tier | Fixtures | Blocked by |
|---|---|---|
| Text-level expectations that can be written now but not executed | source bytes, token spellings, spans, diagnostic category, exit value | T01 freeze to bind records; no harness exists |
| Record-bound | `M1-LX-*`, `M1-TY-*`, `M1-SE-*`, `M1-CL-*`, `M1-WS-*`, `M1-REC-*` | T01 freeze |
| Target-dependent | `M1-LX-04/05`, `M1-NEG-07/18`, `M1-TY-01/07`, `M1-CL-02` | T01 freeze plus probe `verified=true` |
| Mode-sensitive | `M1-PA-09`, `M1-NEG-14/15`, `M1-UNS-05/08` | T01 freeze plus declared/dialect policy |
| Policy-sensitive (non-normative) | `M1-NEG-16` | signed-overflow policy (section 9) |
| Integration | `M1-INT-01` | T01 freeze, target probe, T09/T11 interfaces |

## 8. M1 frontend acceptance gates

M1 frontend fixtures are accepted only when all applicable gates below hold.
Passing a gate does not imply the full compiler works and is not a torture rate.

| Gate | Requirement | Owner |
|---|---|---|
| G1 | `M1-PP-01`–`M1-PP-07` produce the declared token stream and provenance, including the boundary and whitespace variants | T03 |
| G2 | `M1-LX-01`–`M1-LX-07` produce the declared token kinds, spellings, spans, and literal records in the declared mode | T04 |
| G3 | `M1-PA-01`–`M1-PA-08` produce the declared declarator tree, AST shape, and scopes; `(void)` is zero parameters | T05 |
| G4 | `M1-TY-01`–`M1-TY-10` produce the declared types, symbol, linkage, and scope tree under the frozen target model | T06 |
| G5 | `M1-SE-01`–`M1-SE-05` produce the declared typed facts and value category for `2+3` in an `int` function | T07 |
| G6 | `M1-CL-01`–`M1-CL-03` produce the constant value 5 of type `int` for the integer constant expression | T08 |
| G7 | each malformed fixture produces exactly one primary diagnostic with the expected location and a finite recovery | T02, T03, T04, T05, T06, T07, T08 |
| G8 | each unsupported fixture produces an explicit unsupported or failed result, never success and never a fabricated artifact | T02, T13 |
| G9 | replay fixtures are tick-for-tick identical for identical input, config, and initial state | T13 |
| G10 | write-scope and access fixtures hold; a failed batch commits nothing | T01, T02, T13 |
| G11 | no expected value in this document was produced by compiling the candidate with GCC or Clang, and no reference output substitutes for the fixed target | all |
| G12 | any fixture asserting a concrete width, alignment, or ABI value is blocked until the AArch64 probe reports `verified=true` | T01, T11 |
| G13 | the mode for every fixture is declared; no fixture depends on an inferred default dialect | all |
| G14 | integration `M1-INT-01` is separately recorded: candidate-generated object, external assembler/linker, process exit status 5 | T09, T11 |

G14 is listed only so the frontend gates are not confused with the M1
end-to-end milestone. Its detailed acceptance belongs to the T09/T11 task
packages, not to this document.

## 9. Explicitly non-normative items

The following are deliberately outside this fixture's normative pass criteria.
They must still be recorded and must not be counted as passes for the frontend.

- **Signed-overflow behavior.** `M1-NEG-16` (`2147483647+1`) has no normative
  expected value here. Whether M1 defines a wrap policy (`-fwrapv`) or treats
  the case as undefined or a constraint failure is a contract decision. Until
  that decision is recorded, the fixture asserts only that the frontend neither
  crashes nor fabricates a value.
- **Unsupported result encoding.** Section 6.2 requires an explicit
  unsupported or failed result, but the concrete encoding (a distinct
  `Unsupported` diagnostic class versus `Failed` with a specific code) is a T01
  protocol decision and is not fixed here. Fixtures must not assert a specific
  encoding until T01 freezes it.
- **Diagnostic wording and codes.** Fixtures assert diagnostic category and
  location, not message text or numeric codes.
- **Concrete target widths, alignment, and ABI values.** These remain
  unverified and are not asserted by any fixture before the probe.
- **Optimization behavior.** Whether the backend or optimizer folds `2+3` is
  not part of the frontend fixture; `M1-CL-02` asserts the semantic constant
  value only.

## 10. Non-claims

- No test in this document has been run; the compiler application does not yet
  implement these chips.
- Nothing here is an interface, type, record, or manifest freeze. All schema
  names and IDs are provisional until the T01 integrator publishes a freeze.
- No default dialect is asserted for the compiler.
- Target identity is frozen, but every concrete ABI value remains unverified.
- No GCC or Clang was used to compile the candidate source, and no reference
  compiler output was used as an oracle.
- No pass rate, coverage percentage, or M1-complete claim is made. Passing the
  M1 frontend gates is necessary but not sufficient for the M1 vertical slice,
  and neither is the T00 GCC torture rate.
- Unexercised constant, layout, and initialization chips are recorded as gaps
  (`M1-CL-04`), not as passes.

## 11. Revision record

| Date | Change | Authority |
|---|---|---|
| 2026-10-04 | Initial planned M1 frontend fixture/acceptance design; no implementation, no execution, no freeze. | M1 preparation session |
