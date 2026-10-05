# Wave 2 Slice 14: PP Macro-Definition Freeze (`/23`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/23` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/22` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/23` delta), [T03](T03_PREPROCESS_CHIPS.md) PP06–PP08 rows, [conditional slice](PP_CONDITIONAL_SLICE.md) (defined amend) |
| Non-goal | Macro invocation/expansion/rescan/paste/stringify (PP09–16), builtins (PP24), `candidate` PP06 step (unobservable until expansion), target widths |

## 1. What this slice is

Macro definitions become explicit bus state plus three workers:
`PpDefineChip` (PP06, per-`#define`-line task fanned out by the caller —
LX-decode precedent), `PpRedefineChip` (PP07, benign-equivalence
verifier), `PpUndefChip` (PP08, tombstone writer). One new record family
`Macro` (wire tag 27, ordinal 27). The PP19 evaluator is amended to read
the committed table for `defined`/`#ifdef` (the `/22` frozen-`false` rule
is AMENDED here, as predicted there).

## 2. MacroDef record (frozen)

```rust
pub struct MacroRecord {
    /// Macro name spelling (raw bytes, NOT interned).
    pub spelling: Vec<u8>,
    /// Parameter spellings in order (`[]` for object-like).
    pub params: Vec<Vec<u8>>,
    /// Variadic (`...`/`__VA_ARGS__` present; USE deferred to PP16).
    pub variadic: bool,
    /// Replacement list (committed pp-token IDs).
    pub replacement: Vec<PpTokenId>,
    /// `#undef` tombstone (no definition while the latest record is set).
    pub undefined: bool,
}
```

Rationale (recorded decision): names/params stay byte spellings so NO
cross-batch intern-ID prediction is ever needed (commit rejects
future-dated `Name` refs; C compares macro spellings anyway). Lookup =
greatest `MacroId` with equal spelling; a latest tombstone (or absence)
means undefined. `MACRO_RECORD_FIELDS =
["spelling","params","variadic","replacement","undefined"]` in the hash.

## 3. PP06 rules (per-`#define`-line task, caller fans out)

- Payload: one post-`#` line's refs; first must be Identifier `define`
  else protocol fault. Name = next Identifier else malformed `Fail`.
- Function-like iff a `(` punctuator token IMMEDIATELY follows the name
  (span-adjacent, same source, no gap — whitespace-sensitivity per C);
  else object-like (rest of line = replacement, possibly empty).
- Params: `Identifier` list, optional trailing `...`; `__VA_ARGS__`
  anywhere (params or replacement) sets `variadic`. Anything else
  (missing `)`, bad param, `...` non-trailing) → malformed `Fail`.
- Lookup committed table: absent or latest-tombstone → append fresh
  (`undefined: false`) → `Complete(Record)`. Present-and-defined →
  enqueue ONE PP07 child (payload = line refs + `Macro(old)`) + await
  (PP05→PP26 pattern). Resume: child `Failed` → join-reuse Fail; child
  `Ack` (benign) → `Ack`, no append.
- Reads: Tasks `active.*`, Pp `tokens`+`macros`, Sources `spans`
  (span-adjacency needs span bodies), plus a `macros_allocated`
  prediction base. Writes: Pp `macros` +
  allowlist row `(PP06_CHIP, Pp, "macros", PREPROCESS_DEFINE)`.
  Amendment note (integrator-accepted at implementation): the frozen
  manifest list omitted spans, but the frozen §3 adjacency rule is
  unobservable without them — spans added, rule text unchanged.

## 4. PP07 rules (benign-equivalence verifier)

- Payload: one post-`#` line's refs + exactly one `Macro` ref (the
  committed incumbent); first identifier must be `define` else protocol
  fault. Re-parses the line with the §3 grammar (chip-local copy per the
  shared-helper precedent).
- Benign iff: same spelling, same params sequence, same variadic flag,
  same replacement SPELLING sequence (whitespace-insensitive by
  construction). Benign → `Ack` (no append). Else `Fail` naming the
  differing part (params vs replacement vs variadic).
- Reads: Tasks `active.*`, Pp `tokens`+`macros`, Sources `spans`
  (same adjacency amendment as PP06). No writes.

## 5. PP08 rules (tombstone writer)

- Payload: one post-`#` line's refs; first identifier `undef` else
  protocol fault; exactly one Identifier operand else malformed `Fail`.
- Lookup: absent or latest-tombstone → `Ack` no-append (C ignores it).
  Present-and-defined → append tombstone
  (`{spelling, [], false, [], true}`) → `Complete(Record)`.
- Reads: Tasks `active.*`, Pp `tokens`+`macros`. Writes: Pp `macros` +
  allowlist row.

## 6. PP19 amendment (integrator-owned, `/22`-predicted)

`defined(X)` / `#ifdef` / `#ifndef` read the committed table (latest
record per spelling; tombstone/absent = false). Manifest gains Pp
`macros` read. Nothing else in PP19 changes; the `/22` frozen-`false`
rule is superseded with this paragraph as its amendment record.

## 7. Frozen registration

- `preprocess.macro_define` (local 23) + `preprocess.macro_redefine`
  (24) + `preprocess.macro_undef` (25), `Frozen`;
  `pp_macro_slice()` registry (38 entries, cumulative over
  `pp_conditional_slice()`); stage rows (`→ 1` all); routed layers 1;
  `PP06_CHIP = 26`, `PP07_CHIP = 27`, `PP08_CHIP = 28`; allowlist rows
  for the two writers; `StoreSchema::pp_macro_slice()` = pp-slice
  schema + `(Pp, "macros")`.
- Family inventory: `RecordFamily::Macro = 27` (ALL 28), wire tag 27,
  `RecordRef::Macro`, `G1DraftBody::Macro`, `M1_STORE_FAMILY_ARENA` row
  `("pp","macros","macro","macros")`, snapshot kind maps both ways.
- Hash rules: `pp.macrodef-record`, `pp.redefine-benign-rule`,
  `pp.undef-tombstone`, `pp.defined-reads-table`.

## 8. Freeze items

| # | Item | Acceptance |
|---|---|---|
| Y1-1 | Family pipeline (IDs, ref/tag/family/make, drafts, commit predict/materialize/budget, snapshot both-ways, schema, hash lists, freeze counts) | encode round-trip + capacity + predicted negatives |
| Y1-2 | PP06 fresh/await/resume paths (object/function-like, adjacency rule, tombstone-supersede) | per-shape cases |
| Y1-3 | PP07 benign matrix (identical/params/replacement/variadic/whitespace-insensitive) | equivalence cases |
| Y1-4 | PP08 tombstone + ignore-unknown + redefine-after-undef | lifecycle cases |
| Y1-5 | PP19 defined/table amendment (+`/22`-rule supersede note) | `#ifdef`-after-`#define` live |
| Y1-6 | Registration freeze (kinds/registry/stage/layer/chips/allowlist/gates) | freeze gates green |
| Y1-7 | Worker template ×3 (pure computes, lint-clean, ZST, stage/layer) | `c23_macro` green |

## 9. Execution record

Implemented by three parallel chip owners against this freeze
(`pp_define.rs`, `pp_redefine.rs`, `pp_undef.rs`; 579 + smaller lines),
integrated by the T01 integrator. Verified by
`compiler/tests/c23_macro.rs` (10 tests: kind/stage/registry/allowlist
freeze, object-like fresh define + snapshot round-trip, function-like
adjacency/object fallback/variadic flag, benign redefine Ack with no
append, non-benign failure naming the part, undef lifecycle incl.
redefine-after-undef, defined-table driving `#ifdef`, malformed inputs,
gates, replay determinism). Two integrator-side corrections: the
frozen manifest lists omitted span reads (required by the frozen
adjacency rule — amended above), and the `STORE_OWNER_ALLOWLIST` writer
rows were added at integration (hash recomputed after). One test-author
fix (post-`#` payload slicing). All deliveries arrived lint-clean with
scratch matrices; no silent guesses.

## 10. Explicitly deferred

- Invocation/expansion/arguments/rescan/paste/stringify (PP09–16),
  incl. ALL uses of recorded macros (variadic or not).
- Builtin macros (PP24); `-D` injection (H04); `candidate` PP06 step
  (unobservable until expansion).
- Interned-name identity for macros (byte spellings by recorded decision).
