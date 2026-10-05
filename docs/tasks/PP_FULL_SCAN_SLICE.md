# Wave 2 Slice 11: PP Full-Token Scan Freeze (`/20`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/20` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/19` preserved as history; adopts a scoped subset of the T03 rev-55 candidate (which remains proposal-owned everywhere not listed here) |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/20` delta), [T03](T03_PREPROCESS_CHIPS.md) rev-55 §§1–2/4 (scan states + include prefix, NOT §3 chunk persistence), [PP scan slice](PP_SCAN_SLICE.md) |
| Non-goal | Directive dispatch (PP05, next slice), conditionals/macros/include (later), escape validation (LX), universal-character names, `include_next`, macro-expanded includes |

## 1. What this slice is

Two in-place worker amendments (same chip IDs, kinds, stages, manifests,
schemas) completing the PP token scan: `PpCommentChip` gains
literal/header-name-aware comment replacement, `PpScanChip` gains the full
C11 punctuator table, string/character literal tokens, header-name tokens,
and full pp-numbers. Three new `PpTokenKind` variants carry them
(`StringLiteral`, `CharLiteral`, `HeaderName`). LX classify stays M1 and
rejects the new kinds as explicit `Unsupported`.

## 2. Frozen shared predicate (both chips implement it identically)

Byte states over the spliced stream (single tick, whole input — rev-55 §3
chunk persistence is NOT adopted; no cross-tick scan state exists):

- `Code`: `//`→line comment (to newline, newline preserved, one space);
  `/*`→block comment (to first `*/`, inner newlines preserved, one space,
  unterminated = typed Fail); `"`→string (below); `'`→char (below);
  include-prefix (below) + `<`→angle-header / `"`→quoted-header; else code.
- String/char: `\` escapes the next byte (any); first unescaped quote
  closes; newline/EOF before close = typed Fail (no validation of the
  escape itself — LX job). Comment openers inside are literal bytes.
- Angle-header: runs to first `>`; NO comment/escape processing;
  newline/EOF before `>` = typed Fail.
- Quoted-header: escape-aware quote rule (same as string); newline/EOF
  before close = typed Fail.
- Include prefix (lexical, line-start only): optional `[ \t]`, then `#`
  or `%:` (both accepted), optional `[ \t]`, then identifier `include`
  with an identifier boundary (next byte not `[A-Za-z0-9_]`), then
  whitespace/block-comments skipped (`//` aborts the prefix: no header
  context; newline aborts too), then `<`→angle / `"`→quoted / else Code.
  `include_next`, macro-generated `#`, and non-line-start `#` are NOT
  header contexts (explicit non-header, never mis-tokenized).

## 3. PP03 rules (amends `/16` literal-`Unsupported`)

- Comment openers inside strings/chars/headers are literal bytes and pass
  through byte-identical with per-byte identity map entries (existing
  per-byte map mechanics unchanged).
- `//` inside a header aborts only the prefix scan, never a comment.
- Unterminated literal/header = typed Fail (`Task`, 4); unterminated
  block comment keeps the M1-NEG-01 shape.
- The `/16` rule "literal inputs are explicit `Unsupported`" is
  SUPERSEDED by this slice (recorded here, not silently dropped).

## 4. PP04 rules (amends `/16` scan subset)

- Maximal munch over the full C11 punctuator table. Singles:
  `[ ] ( ) { } . & * + - ~ ! / % < > ^ | ? : ; = , #`. Multi-char (longest
  match first): `... << >> <= >= == != && || ++ -- -> *= /= %= += -= <<= >>=
  &= ^= |= ## <: :> <% %> %: %:%:`. No dialect gating in C11 mode.
- pp-number: `[0-9]` start or `.`+`[0-9]`; continue `[0-9A-Za-z_.]` plus
  `[+-]` iff the previous byte is `e/E/p/P` (extends the `/16` rule with
  the leading-dot start only).
- Identifiers unchanged. Whitespace skipped, EOF rule unchanged.
- String/char/header segments (same predicate as §2) each become exactly
  one pp-token of the corresponding kind with the full source spelling
  (header spelling includes the `<...>`/`"..."` delimiters).
- Lone `\`, `@`, `$`, `` ` ``, non-ASCII: explicit `Unsupported` (never
  mis-tokenized). Universal-character names deferred.

## 5. Cross-file consequences (integrator-owned, frozen here)

- `PpTokenKind += {StringLiteral, CharLiteral, HeaderName}` with wire
  names `string_literal`, `char_literal`, `header_name` (auto-hashed via
  `PPTOKEN_KIND_NAMES`); snapshot kind↔name matches extended both ways.
- LX classify: new kinds → explicit `Unsupported` in both matches (M1 LX
  scope unchanged; zero impact on frozen M1 paths since the scanner never
  produced these before).
- `M1_PUNCTUATORS` stays exported (frozen history); the full table lives
  chip-local in `pp_scan.rs`.
- Same kinds/stages/registry (`pp_slice`, 29 entries), same manifests
  (no read/write change), same schemas, same chip IDs (18/19).

## 6. Freeze items

| # | Item | Acceptance |
|---|---|---|
| V1-1 | PP03 literal/header protection (§§2–3) | markers-inside-literals preserved, escapes honored, unterminated fails, `a/**/b` still splits |
| V1-2 | PP04 full table + literals + headers + dotted pp-numbers (§4) | maximal-munch cases (`++` vs `+ +`, `...`, `##`, digraphs, `.5`), single-token literals/headers |
| V1-3 | Shared-predicate drift: PP03→PP04 literal/header spans agree | byte-identity + spelling cross-check, no reimplemented oracle |
| V1-4 | Protocol: 3 new kind variants + names, snapshot both-ways, LX rejects new kinds | round-trip + classify-unsupported + M1 regression green |
| V1-5 | Worker template ×2 amendments (pure computes, lint-clean, ZST/stage untouched, c16 pins green) | full suite green |
| V1-6 | Hash rules `pp.comment-literal-aware`, `pp.scan-full-punctuators`, `pp.scan-literal-header-tokens` | freeze + replay determinism |

## 7. Execution record

Implemented by two parallel chip owners against this freeze (one file
each: `pp_comment.rs`, `pp_scan.rs`), integrated by the T01 integrator.
Verified by `compiler/tests/c20_ppscan.rs` (8 tests: kinds/registry
unchanged, full punctuator maximal-munch, string/char literals,
header-name contexts and aborts, comment literal/header protection,
PP03→PP04 drift end-to-end, LX rejection of new kinds, snapshot
round-trip + hash participation). The `/16` suite `c16_pp.rs` stays
green with 4 assertions updated to the `/20`-superseded expectations
(literal passthrough, `*`/string scan). Both deliveries arrived
lint-clean with scratch matrices; one coordination defect (stale c16
pins vs. supersede) was resolved integrator-side by updating the pinned
assertions, not by weakening the freeze.

## 8. Explicitly deferred

- PP05 dispatch + `#error` + PP diagnostic taxonomy (next slice).
- Conditionals/macros/include (later slices; `include_next`,
  macro-expanded names → T12/PP17).
- Escape/encoding validation, char-length rules (LX slices).
- Universal-character names, non-ASCII modes (explicit `Unsupported`).
- rev-55 §3 chunk persistence (no cross-tick scan state in this design).
