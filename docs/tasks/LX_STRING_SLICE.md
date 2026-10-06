# Wave 3 Slice 8: LX String Freeze (`/33`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/33` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/32` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/33` delta), [T04](T04_LEX_CHIPS.md) LX11/LX12/LX13 rows, [LX slice](LX_SLICE.md), [LX float slice](LX_FLOAT_SLICE.md) |
| Non-goal | A `CodeUnits` result carrier, shared escape helper, character/string candidate vocabulary, adjacent-string merging (LX14), GNU literal extensions (LX15), char/string `TokenKind` members |

## 1. What this slice is

Three new workers covering the escape/char/string USE (T04:19–21):

- `LxEscapeChip` (LX11) validates one literal body against the frozen
  escape rule — simple escapes, octal (at most 3 digits), greedy hex
  (`\x` consumes every following hex digit, at least one required),
  `\uXXXX`/`\UXXXXXXXX` with scalar-range checks (surrogates,
  `> U+10FFFF`, and the C11 6.4.3 `< U+00A0` trio rule) — and completes
  `Ack` on a valid body, typed `Fail` otherwise. It appends nothing:
  the decoded units are certified by the pure `decode_escape_body` core,
  and no `CodeUnits` result carrier is frozen yet (same Ack-only
  position as LX09/LX10).
- `LxCharChip` (LX12) decodes one `prefix'body'` spelling (`none`/`L`/
  `u`/`U`; `u8` is explicit `Unsupported`) to its typed value with the
  frozen multicharacter policy (units masked to the prefix width,
  concatenated big-endian as minimal byte sequences, low 32 bits kept —
  e.g. plain `'ab'` → `0x6162`) and appends exactly one `Character`
  `LiteralRecord` (radix `16`, suffix `None`, `Int` candidate, publish-
  time token back-link), completing `Record`.
- `LxStringChip` (LX13) decodes one `prefix"body"` spelling
  (`none`/`u8`/`L`/`u`/`U`) to code units plus exactly one terminating
  zero with the frozen element-type/width rule (narrow 1 byte, `u`
  UTF-16 with surrogate pairs for non-BMP, `U` UTF-32, `L` from the
  frozen `wchar_t` width — expected 4/utf32, any other width an explicit
  failure) and appends exactly one `String` `LiteralRecord` (radix `0`
  as the explicit non-numeric marker, suffix `None`, `Int` candidate,
  publish-time token back-link), completing `Record`.

All three read the committed T03 PP spelling/kind (DOC-10 resolution;
payload exactly `[Token, PpToken]`, the `/11` `lex.decode_literal`
precedent). Empty character literals fail; the empty string decodes to
exactly one unit (`[0]`). Embedded NULs are ordinary mid-string zero
units and are preserved; only the single final unit is the terminator,
so `units.len()` (value bytes divided by element width) is the string
length including the terminator.

Value semantics (deliberate, not oversights):

- Multicharacter constants are implementation-defined values per the
  frozen truncation shape above — never a constraint failure, never a
  `Fail`. There is no success-plus-warning proposal, so the truncation
  is silent on the wire by construction; it is documented here and must
  surface through a future warning channel (T01 co-freeze).
- Narrow units hold at most `0xFF`: LX13-narrow fails hex/octal/UCN
  values above `0xFF` (multibyte conversion deferred); LX12 truncates
  to the prefix width instead (documented implementation-defined
  behavior). Both sides are loud in the sense that no silent host-ABI
  substitution occurs — but the two policies differ (see §3).
- `radix: 16` (`Character`) and `radix: 0` (`String`) are `/33`-frozen
  encodings, not numeric bases: `0` must never be read as a base, and
  the commit gate admits exactly these shapes alongside the M1 integer
  shape (see §4).
- `candidate_type: Int` on both record shapes is an explicit
  placeholder from the M1-closed `{Int}` vocabulary — never a type. No
  downstream stage may consume it until the character/string candidate
  vocabulary is co-frozen (T04/T08/T01).
- `TokenKind` stays M1-closed (no `Character`/`String` members), so all
  three chips accept any committed token whose PP token has the matching
  literal kind; the C-token kind check is deferred.
- GNU `\e` is an unknown escape (LX15 scope, dialect-gated follow-up).

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| `\a\b\f\n\r\t\v\\\'\"\?`, `\0`, `\1011` → `A`,`1` | Decoded units (LX11 `Ack`; LX12/LX13 values) |
| `\x41B` greedy | LX11 one unit `0x41B`; LX12 truncates to `0x1B`; LX13-narrow `Fail` |
| `\u00E9`, `\U0001F600`, `\u0024`/`\u0040`/`` \u0060 `` trio | Decoded; trio stays legal below `U+00A0` |
| `\x` (no digits), `\q`, `\e`, trailing `\`, raw newline | `Fail` everywhere |
| `\uD800`, `\U00110000`, `\u000A`, truncated `\u12` | `Fail` (surrogate/range/control/truncation) |
| `''`, `'a` (unterminated), `u8'a'` | `Fail` (empty/unterminated/unsupported) |
| `'ab'`, `L'ab'` | `0x6162` (frozen truncation, still success) |
| `""` | Single unit `[0]`; value `[0x00]` |
| `"a\0b"` | Units `[a, 0, b, terminator]`, length 4 |
| `u"\U0001F600"` | Surrogate pair `D83D DE00` + terminator |
| `L"A"` (width 4) | LE bytes `41 00 00 00 00 00 00 00` |
| Wrong kind, non-`Running`, bad payload, missing token, non-utf32 wchar | Protocol/config `Fail` |

## 3. Escape-copy reconciliation (frozen)

No frozen shared escape helper exists, so **each chip keeps its own
chip-local escape copy** by the `compose_map`/`FloatParts` copy
precedent and imports nothing from its siblings. The frozen interchange
is the **spelling** plus the three hashed rules below; convergence on
one helper is deferred to a T04/T01 co-freeze. Known deliberate deltas,
each pinned by the agreement case in `c33_string` (all loud, never
silent substitution):

- Raw non-ASCII bytes: LX11 UTF-8-decodes them (malformed → `Fail`);
  LX12 and LX13-narrow pass bytes through; LX13-wide decodes UTF-8.
- Hex magnitude: LX11 passes values through untruncated (up to
  `u32::MAX`; larger fails); LX12 truncates to the prefix width;
  LX13-narrow fails above `0xFF`.
- UCN range: LX11 enforces the C11 6.4.3 `< U+00A0` trio rule;
  LX12/LX13 check only surrogates and `> U+10FFFF`.

Still open (T04/T01 co-freeze): a shared escape helper, a `CodeUnits`
result carrier, the character/string candidate vocabulary, and the
success-plus-warning channel for multichar truncation.

## 4. Frozen registration

- `lex.escape_decode` (local 21), `lex.char_decode` (local 22),
  `lex.string_decode` (local 23) — first codes after `/32`, all
  `Frozen`; `lx_string_slice()` registry (53 entries, cumulative over
  `lx_float_slice()`); stage rows (all `→ 2`); routed layer 2;
  `LX11_CHIP = 41` (Ack-only, zero writes, no allowlist row),
  `LX12_CHIP = 42` + `LX13_CHIP = 43` (each appends `lex.literals`
  with one allowlist row each; `STORE_OWNER_ALLOWLIST` grows 30 → 32);
  same schemas (`pp_macro_slice()` — `lex.literals` is declared since
  `m1_slice()`).
- Commit gate (`commit.rs`, `/33`): the `Literal` append gate admits the
  M1 integer shape plus exactly the two new shapes — (`Character`,
  `None`, `16`, `Int`) and (`String`, `None`, `0`, `Int`). Anything
  else stays explicit unsupported (`InvalidPatchShape`, never silent).
- Hash rules: `lx.escape-decode`, `lx.char-typed`, `lx.string-record`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| F1-1 | Escape rule (simple/octal/greedy-hex/UCN) | escapes + hex cases |
| F1-2 | Empty char / malformed spellings fail | empty-char case |
| F1-3 | Multichar truncation policy | multichar case |
| F1-4 | String NUL terminator + embedded NUL | nul + embedded cases |
| F1-5 | Wide encodings (wchar/surrogate/UTF-32) | wide case |
| F1-6 | Escape-copy agreement + documented deltas | agreement case |
| F1-7 | Registration freeze (kinds/registry/stage/layer/chips/allowlist) | freeze + gates green |
| F1-8 | Bus dispatch (records committed) + replay determinism | bus case |

## 6. Execution record

Delivered as three untracked chip files (`lx_escape.rs`, `lx_char.rs`,
`lx_string.rs`) against the `/32` tree; integrated by the T01
integrator: froze locals 21/22/23 / `ChipId(41/42/43)` /
`lex.escape_decode` + `lex.char_decode` + `lex.string_decode` into
`task.rs` (`lx_string_slice()`, 53 entries), `LX11_CHIP` + `LX12_CHIP`
+ `LX13_CHIP` + `is_lx_string_slice_kind` + three stage rows + two
allowlist rows into `manifest.rs`, three rule ids into `contract.rs`,
widened the `commit.rs` `Literal` gate to the two `/33` shapes, bumped
to `t01-c01-c06/33` with recomputed hash
(`e8400eb1…eef455`), wired `lex/mod.rs` + `chips/mod.rs`, and
re-pointed the chips' proposed consts at the frozen canonicals
(`LX11_TASK_KIND`/`LX12_TASK_KIND`/`LX13_TASK_KIND` aliases, test path
→ `c33_string.rs`; headers rewritten from UNREGISTERED-draft to frozen
`/33`; escape-copy reconciliation frozen per §3). No chip logic
changed beyond kind/chip-id/test-path repointing. Verified by
`compiler/tests/c33_string.rs` (14 tests: freeze, escapes, hex/UCN,
body split, char failures, char values + multichar, char prefixes,
empty string, embedded NUL, wide, invalid strings, agreement, bus
dispatch + determinism, stage-layer gates). Full §5 suite green at
commit. Superseded pins updated by the integrator (never the chip
owner, never silent): `freeze.rs`, `c08_gate1` (version, 53 stages, 32
allowlist rows), `c04_manifest` (32 rows), `c32_float` (next-free local
24, 32 rows), `c20_ppscan` + `h04_candidate` version markers, both
READMEs, `T01_COMPILER_CONTRACT.md` §7.1, `CHIP_PLAN.md`, `context.md`.

## 7. Explicitly deferred (all loud, never silent)

- `CodeUnits` result carrier (LX11 stays Ack-only; units recomputed via
  the pure core until the carrier freezes).
- Shared escape helper (three local copies kept by precedent until the
  T04/T01 convergence co-freeze).
- Character/string candidate vocabulary (`Int` placeholder must not be
  consumed as a type).
- Success-plus-warning channel for multichar truncation.
- `TokenKind::Character`/`String` members (any committed token
  accepted; kind check deferred).
- LX14 adjacent-string merging, LX15 literal extensions (unchanged).
