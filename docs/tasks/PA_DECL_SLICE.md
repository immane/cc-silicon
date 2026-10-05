# Wave 3 Slice 9: PA Decl Freeze (`/34`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/34` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/33` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/34` delta), [T05](T05_PARSE_CHIPS.md) PA02/PA03/PA05/PA07/PA09/PA28/PA32 rows, [PA slice](PA_SLICE.md), [LX string slice](LX_STRING_SLICE.md) |
| Non-goal | A `next_cursor` result carrier (OB-30 open), committed node links, child-task fan-out (PA01 wiring owns it), scope enter/exit (T06), type interpretation (T06/T07), non-M1 syntax |

## 1. What this slice is

Four Ack-only workers splitting the M1 declaration path (`int
main(void){return 2+3;}`) into individually testable productions:

- `PaExternalChip` (PA02) classifies one external declaration at the
  continuation cursor (`ExternalDecl` context, bounded 8-token
  lookahead window, minimum 6) as a function definition (`{`) or a
  declaration (`;`). The M1 specifier (`int`) and declarator
  (`main(void)`) gates run first; any third discriminator — EOF, `=`,
  `,`, a K&R parameter name — is an explicit typed `Fail`, never a
  guessed default. Success completes `Ack`; the PA01 wiring layer owns
  the PA03/PA05 fan-out through the returned `ExternalDeclKind`.
- `PaSpecifierChip` (PA03) accepts exactly one committed token,
  Keyword `int`, and completes `Ack` (cursor advance stays
  caller-held). Every other bundle — `unsigned`, `long`,
  `Identifier`-spelled `int`, wrong arity — is explicit `Unsupported`.
- `PaDeclaratorChip` (fused PA05/PA07/PA09) accepts exactly
  `main(void)`: direct declarator `main` plus one `(void)` function
  suffix (zero parameters, prototype), certified through the pure
  `parse_declarator` / `parse_direct_declarator` /
  `parse_parameter_list` core with `DeclaratorTree` accessors. The
  empty `()` shape is the documented DEFECT
  (`AmbiguousEmptyParams`, `Task`-channel diagnostic, never read as
  `(void)`). Pointer declarators (PA06), parenthesized declarators,
  array suffixes (PA08), variadic/old-style/non-`void` parameters, and
  non-`main` names are explicit `Unsupported`.
- `PaBlockChip` (fused PA28/PA32) dispatches on the task kind: PA28
  accepts exactly `{ return <int> + <int> ; }` (7 tokens, `return`
  spelling checked) and PA32 accepts exactly
  `return <int> + <int> ;` (5 tokens). Each integer leaf needs exactly
  one committed literal; literal *values* are never interpreted
  (T07/T08-owned). `return;` is not "return zero", `{}` is not an
  empty compound, `;` alone is not an expression statement (PA34 owns
  that call) — all explicit `Unsupported`.

All four are pure-plus-`Ack`: they validate committed records and
append nothing. No cursor carrier is frozen (OB-30 stays open); no node
links are committed (committed `Specifiers`/`Compound`/`Return` nodes
stay PA01-owned until the full catalog split); no child task is
enqueued (the PA01 external loop owns fan-out).

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| `int main(void){` at cursor | `FunctionDefinition` + `Ack` (PA02) |
| `int main(void);` at cursor | `Declaration` + `Ack` (PA02) |
| `unsigned …`, `=`, K&R name, truncated window, non-`Running` | `Fail` (PA02; `Unsupported`, resp. `Task`) |
| Keyword `int` (one token) | `Ack` + `Specifiers` shape (PA03) |
| `unsigned`, `long`, `Identifier int`, arity ≠ 1 | `Unsupported` (PA03) |
| `main(void)` | `Ack` + validated tree (PA05) |
| `main()` | `Task`-channel DEFECT (PA05) |
| `*f(void)`, `other(void)`, `main(int)`, `main[…]` | `Unsupported` (PA05) |
| `{ return 2+3; }` (7 tokens, 2 literals) | `Ack` + 5-row shape (PA28) |
| `return 2+3;` (5 tokens, 2 literals) | `Ack` + 4-row shape (PA32) |
| `return;`, `{}`, `;`, missing literals, non-`Running` | `Fail` (PA28/PA32; `Unsupported`, resp. `Task`) |
| Wrong stage layer, pre-`/34` registry | Driver/gate `Fail` (never silent) |

## 3. ID arbitration (frozen)

The four delivered files each claimed `ChipId(44)` / `PARSE` local 17
(the next-free values at authoring time). The integrator arbitrated
distinct frozen IDs, verified against the `/33` head (`LX13_CHIP =
ChipId(43)`, `LEX` locals through 23):

- Kinds: `parse.external_declaration` (17), `parse.specifiers` (18),
  `parse.declarator` (19), `parse.block` (20), `parse.return` (21) —
  five kinds because the fused block chip serves two productions
  dispatched on the task kind; `PARSE` owners start new codes at
  local 22.
- Chips: `PA02_CHIP = 44`, `PA03_CHIP = 45`, `PA05_CHIP = 46`
  (fused PA05/07/09), `PA28_CHIP = 47` (fused PA28/32, claims both
  kinds).
- The files' candidate consts/fns (`PA_*_CHIP_CANDIDATE`,
  `*_task_kind_candidate()`, `PA_DECLARATOR_LOCAL`) are re-pointed at
  the frozen canonicals (`PA02_TASK_KIND` / `PA03_TASK_KIND` /
  `PA05_TASK_KIND` / `PA28_TASK_KIND` / `PA32_TASK_KIND` aliases,
  manifest chip IDs, test path → `c34_parse.rs`); headers rewritten
  from UNREGISTERED-draft to frozen `/34`. No chip logic changed
  beyond kind/chip-id/test-path repointing, plus the PA03 projector
  tightened from group-only to the exact frozen kind (its DEFECT-1).

## 4. Frozen registration

- `parse.external_declaration` (local 17), `parse.specifiers` (18),
  `parse.declarator` (19), `parse.block` (20), `parse.return` (21) —
  first codes after `/33`, all `Frozen`; `pa_decl_slice()` registry
  (58 entries, cumulative over `lx_string_slice()`); stage rows (all
  `→ 2`); routed layer 2; `PA02_CHIP = 44` + `PA03_CHIP = 45` +
  `PA05_CHIP = 46` + `PA28_CHIP = 47` (all Ack-only, zero writes, no
  allowlist rows; `STORE_OWNER_ALLOWLIST` stays 32); same schemas
  (`pa_slice()` — every read field is declared since `m1`/`lx`).
- Hash rules: `pa.external-dispatch`, `pa.specifier-int`,
  `pa.declarator-void`, `pa.block-return`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| F1-1 | External dispatch (`{` vs `;`, third token loud) | fn-vs-decl + reject cases |
| F1-2 | Specifier (`int` alone) | int-vs-unsigned case |
| F1-3 | Declarator (`main(void)`, `()` DEFECT) | void-vs-empty + non-M1 cases |
| F1-4 | Block (7-token) + return (5-token) shapes | block + return cases |
| F1-5 | Non-M1 matrix (K&R, `return;`, `{}`, `;`) | matrix case |
| F1-6 | Registration freeze (kinds/registry/stage/layer/chips/allowlist) | freeze + gates green |
| F1-7 | Bus dispatch (all `Ack`) + replay determinism | bus case |

## 6. Execution record

Delivered as four untracked chip files (`pa_external.rs`,
`pa_specifier.rs`, `pa_declarator.rs`, `pa_block.rs`) against the
`/33` tree; integrated by the T01 integrator: arbitrated distinct
locals 17–21 / `ChipId(44–47)` (§3) into `task.rs`
(`pa_decl_slice()`, 58 entries), `PA02_CHIP` + `PA03_CHIP` +
`PA05_CHIP` + `PA28_CHIP` + `is_pa_decl_slice_kind` + five stage rows
+ zero allowlist rows into `manifest.rs`, four rule ids into
`contract.rs`, bumped to `t01-c01-c06/34` with recomputed hash
(`229ae1a7…169be74`), wired `parse/mod.rs` + `chips/mod.rs` (pure
cores re-exported by the LX-helper precedent), and re-pointed the
chips' proposed consts at the frozen canonicals (headers rewritten
from UNREGISTERED-draft to frozen `/34`; PA03 projector tightened to
the exact kind). No chip logic changed beyond kind/chip-id/test-path
repointing. Verified by `compiler/tests/c34_parse.rs` (12 tests:
freeze, fn-vs-decl, external rejects, int-vs-unsigned, void-vs-empty,
declarator rejects, block shape, return shape, non-M1 matrix,
non-running rejects, bus dispatch + determinism, stage-layer gates).
Full §5 suite green at commit. Superseded pins updated by the
integrator (never the chip owner, never silent): `freeze.rs`,
`c08_gate1` (version, 58 stages, 32 allowlist rows), `c04_manifest`
(no-row comment), `c20_ppscan` + `h04_candidate` version markers,
both READMEs, `T01_COMPILER_CONTRACT.md` §7.1, `CHIP_PLAN.md`,
`context.md`.

## 7. Explicitly deferred (all loud, never silent)

- `next_cursor` result carrier (OB-30; caller holds the cursor).
- Committed node links and parent assignment (PA01/PA02/PA14
  integration; a second `parse.nodes` writer needs an allowlist row).
- Child-task fan-out and await-all nesting (PA01 wiring; T05 items
  A–C apply when nesting lands).
- Block scope enter/exit (T06-owned around the committed `Compound`).
- `()` disambiguation across modes (stays a DEFECT until the
  mode/dialect freeze).
- Non-M1 syntax everywhere (stays in the denominator as
  `Unsupported`).
- PA04 typedef disambiguation, PA06 pointers, PA08 arrays, and the
  PA10–PA38 remainder (unchanged).
