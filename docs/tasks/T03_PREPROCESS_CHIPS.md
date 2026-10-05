# T03: Source Normalization and Preprocessing Chips

Prerequisite T01; integration depends on CT02/CT08/CT06. Directory `chips/preprocess/`. Read `sources/config/pp/tasks`; write its own `pp` tokens/macros/frames and source-map records, with persistent modifications going through commit; must not read files directly. Results are handed to T04.

Protocol: `PpRequest(source/token_range, cursor, expansion_context, include_context)`, outputting `PpResult(token_range, provenance)` or a typed child request. Preprocessing expressions are first macro-processed, then evaluated against the PP integer model; it must not directly use a general C expression evaluator and ignore PP rules. Each tick processes one explicit scan chunk, directive, or expansion frame; chunk results must not depend on scheduling granularity.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| PP01 SourceNormalizeChip | SourceBytes → NormalizedBytes/map | Newlines and the selected standard translation phase; trigraphs in legacy mode according to dialect | CRLF, missing final newline, mode differences |
| PP02 LineSpliceChip | NormalizedBytes → SplicedBytes/map | Backslash-newline splicing before comment/token recognition | Multi-line identifier, string, comment |
| PP03 CommentReplaceChip | SplicedBytes → CommentFreeStream | `/* */` and `//` become whitespace; preserve logical newlines and positions. **Superseded in part (rev 55, review DOC-06):** stateful scan — comments are recognized only in code state, never inside string literals, character constants, or `#include` header-names; see the rev-55 scan-state/PP04 contract below | Quotes in comments, tokens must not be incorrectly glued, unterminated; **rev 55 adds** literal/header-name protection, escaped-quote, include-context, and line-splice-interplay cases (planned matrix below) |
| PP04 PpTokenScanChip | Stream/cursor → PpToken | maximal munch; pp-number, identifier, literal, punctuator. **Rev 55:** consumes PP03's comment-free stream and scan-state classification (below); a protected string/character literal or include header-name stays one pp-token | `1e+foo` is a pp-number; digraph; non-ASCII mode; protected literal/header-name boundaries (rev 55) |
| PP05 DirectiveDispatchChip | LineTokens → DirectiveTask | Recognize `#` directives only at a legal start of line; in inactive regions process only conditional control | Macro-generated `#` must not become a new directive; empty directive |
| PP06 MacroDefinitionChip | DefineTokens → MacroDef | Object/function macro distinction determined by an immediately following parenthesis; record parameters/replacement | `F(x)` vs `F (x)`; empty replacement |
| PP07 MacroRedefinitionChip | Old/NewMacro → Compatible/Diagnostic | Redefinition compatibility under token and whitespace constraints; arbitrary overwriting is prohibited | Equivalent redefinition, non-equivalent, redefinition after undef |
| PP08 MacroUndefChip | UndefName → MacroRemoved | Validate trailing tokens and remove the definition | Undefined name, invalid argument |
| PP09 MacroInvocationChip | Token/MacroTable → ExpandOrPass | Determine object/function invocation, parse context and hide-set | Function macro without invocation; recursive reference termination |
| PP10 MacroArgumentCollectChip | InvocationTokens → ArgumentRanges | Split commas by parenthesis depth; distinguish empty argument/no argument | Nested call, empty argument, unterminated, comma in string |
| PP11 MacroArgumentExpandChip | ArgumentRange → ExpandedArgument | Pre-expand at positions other than `#` and `##` per the rules; keep both raw and expanded versions | Argument itself contains macros; same argument used both ways |
| PP12 MacroSubstituteChip | Macro/Arguments → ReplacementTokens | Parameter substitution, placemarker, token origin | Empty argument substitution, multiple uses, parameter name shadowing |
| PP13 MacroStringifyChip | RawArgument → StringToken | Whitespace folding; quote/backslash escaping inside strings/characters | `#x` is not pre-expanded; contains comments/quotes |
| PP14 MacroPasteChip | TokenPair → PastedToken | Re-pp-tokenize after ## pasting; must form a single token | Empty placemarker, illegal paste, generated macro name |
| PP15 MacroRescanChip | Replacement/context → RescanFrame | Replacement rescan, hide-set/disabled semantics, expansion boundary | Self-recursion, mutual recursion, standard counterexamples for nested function macros |
| PP16 VariadicMacroChip | VariadicArgs → VariadicTokens | `__VA_ARGS__`, dialect-qualified comma swallowing, VA_OPT where the corpus requires it | Empty variadic, GNU comma, mode rejection |
| PP17 IncludeResolveChip | IncludeTokens/context → SourceRequest | Macro-expand the header name; quote/angle path policy comes from config | Nested include, empty name, header not found |
| PP18 IncludeEnterExitChip | SourceResponse/frame → IncludeState | include stack/origin, depth limit, pragma-once policy | Repeated include, guard, depth error |
| PP19 ConditionalDirectiveChip | If/Elif/Else/Endif → ConditionalFrame | Nested active/taken state and structural validation | Bad C in inactive regions is ignored; duplicate else/extra endif |
| PP20 DefinedOperatorChip | ConditionTokens → DefinedResolved | Both defined syntaxes; do not incorrectly expand their operand | `defined X`/`defined(X)`; nested macro boundary |
| PP21 PpExpressionParseChip | ConditionTokens → PpExpr | Precedence, ?:, logical short-circuit, undefined identifier → 0 | `0 && 1/0`, character constant, illegal expression |
| PP22 PpExpressionEvaluateChip | PpExpr/target → BranchValue | PP maximum integer domain, signed/unsigned conversion and overflow policy | High-bit unsigned comparison, division by zero, shift boundary |
| PP23 LineDirectiveChip | LineTokens → LogicalLocation | `#line` and GNU line markers; distinguish physical/logical file | __LINE__/diagnostics consistent; restore on include return |
| PP24 BuiltinMacroChip | MacroName/context → Tokens | FILE/LINE/COUNTER and frozen-target predefined macros; date/time explicitly configured | Replayable date; counter order; target macros |
| PP25 PragmaDispatchChip | PragmaTokens → PragmaRecord | `_Pragma`, pack push/pop, once, supported pragmas | String decoding; unknown-pragma policy |
| PP26 PpDiagnosticChip | Error/WarningDirective → Diagnostic | Produce structured # error/warning information in active regions | Not emitted in inactive regions; correct span |
| PP27 ExpansionSourceMapChip | TokenOrigins → OriginChain | Spelling/expansion locations and macro call chain are traceable | #/##/nested include backtracking |
| PP28 PreprocessedEmitChip | FinalPpTokens → PpArtifact | Output that can be lexed again; avoid accidental token joining | `+ +` does not become ++; strings/line marker |

Scheduling dependencies: PP01→02→03→04; macro expansion PP09→10→11/13/14→12→15 (the concrete exceptions for `#`/`##` pre-expansion are expressed by the already-frozen protocol; do not blindly follow the table order). Conditional stack management and include waits persist across ticks. Macro recursion terminates by the rules, not by truncation at a small fixed count. Final acceptance prohibits using only GCC `-E` as a substitute for this package. **DOC-06/rev 55:** PP03 comment replacement must not alter string/character-literal or `#include` header-name bytes; PP03 and PP04 share the rev-55 scan-state contract below, and the PP03 scan state persists across chunks/ticks.

**DOC-05 correction — the historical macro-expansion dependency chain above is
superseded guidance; do not implement it as written.** The chain
`PP09→10→11/13/14→12→15` places paste (`PP14`) before argument substitution
(`PP12`), but `PP14` must consume the replaced tokens/placemarkers produced by
`PP12`; implementing that order can paste parameter names instead of argument
tokens (e.g. `CAT(a,b)`). The superseding order is `PP09→10→11/13→12→14→15`:

- `PP10` collects the raw argument ranges.
- `PP11` pre-expands **only** the ordinary (non-`#`, non-`##`) uses of an
  argument; an argument used both ways keeps both the raw and the expanded
  form, and `PP13` stringifies `#` operands from the **raw, unexpanded**
  argument. These prepare the argument forms and may run before substitution.
- `PP12` substitutes into the replacement list: **raw** argument tokens for
  parameters adjacent to `##` (a placemarker for an empty argument), `PP13`'s
  string token for `#` operands, and pre-expanded tokens for ordinary
  parameters, recording token origin.
- `PP14` pastes and resolves placemarkers on `PP12`'s replaced tokens; paste must
  run **after** substitution, not before it.
- `PP15` rescans the substituted, `#`/`##`-processed sequence with placemarkers
  removed, under the hide-set/disabled rules.

The chip table and the scheduling text above are superseded historical guidance,
not an implementation order; the "Contract-realization amendments" section below
records the current working basis.

## Contract-realization amendments

These are selected contract decisions for the `/6` revision working basis, **not**
implemented behavior, **not** a freeze, and **not** code/chip authorization. `/5`
remains current. The chip table and scheduling text above are preserved as history;
the DOC-05 correction note above supersedes the historical macro-expansion
dependency chain.
Every item below stays **pending T03/T01 co-freeze** unless explicitly marked
user-accepted; numeric codes, error classifications, and the cross-file
schema/hash are recorded as **open** throughout.

- **Artifact record shape and total `ArtifactKind` set — user-accepted subdecision
  (rev 44, 2026-10-05; CDR §C/§9/§10/§13).** The `artifacts.fragments`
  `ArtifactRecord` is `{ kind, source: Option<SourceId>, bytes, raw_offsets:
  Vec<u64> }` (replacing the checked-in `{kind, bytes}`). The `ArtifactKind` total
  set is **8** variants, with a total `requires_map` predicate: **map-mandatory**
  `Normalized` / `Spliced` / `CommentFree` / `Preprocessed`, **map-optional**
  `Assembly` / `Object` / `Snapshot` / `Trace`. The M1 **exercised scope is only
  single-source `Normalized`** (PP01); the other map-mandatory producers
  (`Spliced`/`CommentFree`/`Preprocessed`) and the multi-source map are
  **deferred**. `Preprocessed` (PP28's re-lexable final artifact) is **declared but
  not produced by the M1 fixture**, all map-mandatory, and remains an explicit
  unexercised gap, not a pass. **Still open:** the exact `raw_offsets`
  invariant/error mapping and all enum numeric codes.
- **Mandatory-map invariants — user-accepted subdecision (rev 45, 2026-10-05; CDR
  §C/§9/§10/§13).** For the map-mandatory kinds the map must satisfy
  `raw_offsets.len() == bytes.len()+1`, first element `== 0`, monotonic
  nondecreasing, last element `<= source.bytes.len()`, and the kind **requires a
  valid `source`**. **Still open:** the source-versus-payload equality rule and the exact artifact error
  classification/numeric codes (`ArtifactSourceMismatch` /
  `ArtifactSourceMissing` remain **proposed names, not frozen**).
- **Optional-kind map rule — integration-agent selected candidate default under
  explicit user delegation (rev 47, 2026-10-05; CDR §9B/§9C/§10/§13). This is NOT
  a T03 owner signoff and NOT a `/6` freeze.** Under the user's explicit
  delegation for non-critical decisions, the integration agent selected: a
  map-optional kind (`requires_map(kind) == false`) has **empty `raw_offsets`**;
  `source` stays `Option<SourceId>` and is **valid when `Some`**; there is **no
  source-payload-equals-artifact-`bytes` requirement** (normalization transforms
  the payload); and any exact source-provenance/equality rule is **deferred**.
  Exact numeric error codes stay open. This default is a candidate only and
  **pending T03/T01 co-freeze**; it does not override any user-selected rev 44/45
  decision.
- **M1 single-source `Normalized` `raw_offsets` semantics — integration-agent
  selected candidate default under explicit user delegation (rev 53, 2026-10-05;
  CDR §C/§9B/§9C/§10/§13). This is NOT a T03 owner signoff and NOT a `/6`
  freeze and NOT a `/6`-frozen boundary formula.** Grounded in a read-only
  `raw_offsets` audit of the accepted rev 44 shape and the accepted rev 45
  mandatory-map invariants; it settles **only** the *meaning* of the already-
  accepted map for the M1 exercised scope (PP01 single-source `Normalized`), not
  the invariants, not the numeric codes, and not any non-M1 kind. Selected
  candidate semantics: `raw_offsets[i]` is the **raw-source boundary associated
  with output boundary `i`**, so an output half-open range `[a,b)` maps to the
  raw half-open range `[raw_offsets[a], raw_offsets[b])`. Retained bytes map
  corresponding start/end boundaries to the same raw bytes. For `CRLF`→`LF`
  normalization, the output `LF` start maps to the raw `CR` start and its end
  maps **after** the raw `LF` (the two raw bytes collapse to one output byte,
  `raw_offsets[b] - raw_offsets[a] == 2` across that output `LF`). For an
  **inserted** terminal `LF` (missing-final-newline case) both its start and end
  boundaries map to the raw `EOF` offset (`raw_offsets[b] - raw_offsets[a] == 0`,
  a zero-width insertion). The identity (`M1-SRC-000`), final-`LF`, `CRLF`
  (`M1-SRC-002`), and whitespace-variant (`M1-SRC-003`) M1 cases are
  **deterministic** under this rule. This single-offset map is a **primary
  location map**, not complete provenance: it does not by itself recover deleted
  splice ranges (`PP02` `\`-newline), comment removal (`PP03`), macro
  expansion/paste origins (`PP12`/`PP14`), or multi-source (`#include`) origins,
  which remain **deferred** to the `/6` provenance/expansion design. **Still
  open:** exact error behavior/codes for a violated map, and any general
  transform-policy formula beyond the M1 cases above. This default is a candidate
  only, **pending T03/T01 co-freeze**; it does not override any user-selected
  rev 44/45 decision and claims no `/6` freeze.
- **PP03 comment-replacement literal/header-name protection and PP04 scan-state
  collaboration — documentation-review remediation candidate (rev 55, 2026-10-05;
  review DOC-06). This is NOT a user-accepted subdecision, NOT a T03/T01 signoff,
  and NOT a `/6` freeze.** Specifies PP03 as a deterministic state machine over
  the PP02 spliced logical stream: comments are recognized only in code state,
  never inside string literals, character constants, or `#include` header-names
  (quoted or angle); literal escapes are honored; include context is recognized
  lexically (no directive dispatch, uniform in active/inactive regions); the
  scan state persists across chunks/ticks; and PP04 consumes the comment-free
  bytes plus the scan-state classification so protected literals and header-names
  stay single pp-tokens. Contract + planned test matrix (literal, escaped-quote,
  include-context, line-splice-interplay, token-gluing, chunk-invariance, and
  unterminated cases) in "PP03 comment-replacement scan-state and PP04
  collaboration contract candidate (rev 55)" below. Doc-only; no code, no
  freeze; every record/store/kind/diagnostic name and numeric code remains open
  pending T03/T01 co-freeze.
- **Cross-file schema/hash pending.** The map + `requires_map` +
  `artifact_kind_name` (total 8) are **proposed to be hash-pinned** in
  `M1AppendSchema` (prospective `/6`; not pinned today), and logical offsets
  remap to raw `SourceRecord.bytes` before any `SpanDraft` is formed. **No `/6`
  freeze or hash is claimed here.**
- **Ownership (unchanged).** `sources.spans` and `sources.expansions` are
  **T03-only**; T04/Token reuse committed T03 PP spans and write no spans, and
  T05 writes no `sources.spans`. `SpanRecord` offsets are proposed `u64`.

## M1 PP01 producer/consumer contract candidate (rev 54)

This is a **compact contract candidate for the M1 exercised scope only**:
single-source `Normalized` input through PP01 `SourceNormalizeChip` and the
immediately adjacent PP-token/span consumers. It is grounded in the accepted
rev 44/45/47 artifact subdecisions, the rev 53 delegated `raw_offsets` boundary
convention, the M1 proposal §5/§6.1/§8 record shapes, and the M1 vertical
acceptance `M1-PP-01`–`M1-PP-08` rows. It is **not** a T03/T01 signoff, **not** a
`/6` freeze, and it **does not** extend M1 to `LineSplice`/`CommentReplace`/
macro/include/multi-source (all deferred). All noncritical ambiguity below uses a
**conservative candidate marked delegated**; exact unresolved fields and the ref
strategy are listed rather than invented.

### 1. Exact source input (producer)

- The PP01 task payload carries **exactly one** live `RecordRef::Source` for the
  M1 single-file fixture (`M1-SRC-000`, no `#include`); 0 refs ->
  `SourceNotDeclared`, >1 -> `MultipleSourceRefs` (**proposed names, not frozen**;
  `source_scoped_one_hop`, M1
  proposal §6.1). The referenced `SourceRecord.bytes` are **raw physical bytes**
  and are read-only; PP01 never reads a file or samples the environment.
- PP01 is **source-scoped** by the batch scan because it emits
  `SpanDraft`/`PpTokenDraft.span`/`ArtifactDraft.source`; the declared source is
  therefore mandatory. This is the M1 **one-source** limit; include expansion
  across multiple `SourceId`s (`PP17`/`PP18`) is a typed multi-source map
  **T03/integrator `/6` blocker**, not modeled here.

### 2. Normalization output (producer)

- PP01 emits, for the M1 exercised scope, the newline-normalized bytes for
  `M1-SRC-000`/`001`/`002`/`003`: identity for the LF-terminated canonical source
  (28 bytes); an **inserted terminal `LF`** for the missing-final-newline variant
  (27 -> 28 bytes, "newline inserted" provenance); `CRLF` -> `LF` for the CRLF
  variant. Trigraph/legacy-mode behavior and the wider standard translation phase
  are **dialect-dependent and deferred** (only the M1 LF/CRLF/insert cases are
  claimed deterministic).
- Output is published as **both** a map-mandatory single-source `Normalized`
  `ArtifactRecord` and the PP-token/span records of the same stage chain
  (§4/§6). The `Normalized` kind is distinct from `Preprocessed` (PP28); the
  `Preprocessed` artifact **kind is declared but not produced by M1** (the PP28
  fixture in `M1-PP-06` exercises the token re-emission, not a `Preprocessed`
  artifact).

### 3. `ArtifactRecord` publication (accepted map rules)

- Shape (user-accepted rev 44): `{ kind: ArtifactKind, source: Option<SourceId>,
  bytes, raw_offsets: Vec<u64> }`, with the **total eight** `ArtifactKind` set and
  total `requires_map` (map-mandatory `Normalized`/`Spliced`/`CommentFree`/
  `Preprocessed`; map-optional `Assembly`/`Object`/`Snapshot`/`Trace`).
- Mandatory-map invariants (user-accepted rev 45): `raw_offsets.len() ==
  bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<=
  source.bytes.len()`, and a valid `source` required. `bytes.len() <=
  max_source_bytes` is a **separate proposed check** (the limit
  name/default/code is an open `/6` item, not part of the accepted rev-45 list).
- Boundary convention (rev 53, delegated under user delegation):
  `raw_offsets[i]` is the **raw-source boundary associated with output boundary
  `i`**, so output `[a,b)` maps to raw `[raw_offsets[a], raw_offsets[b])`;
  retained bytes map corresponding start/end boundaries; the `CRLF`->`LF` output
  `LF` start maps to the raw `CR` start and its end maps **after** the raw `LF`
  (span 2 across that output byte); the **inserted** terminal `LF` maps both
  boundaries to the raw `EOF` offset (zero-width insertion). Identity,
  final-`LF`/insert, `CRLF`, and whitespace-variant M1 cases are deterministic.
- Writer: `artifacts.fragments` is **T03-only** in M1; CT14/ArtifactFinalize only
  reads and publishes a Host write request and **never appends**. `Assembly`
  (Part B, T11/probe-gated) and `Object`/`Snapshot`/`Trace` (Host/Part B) have
  **no M1 writer** and are map-optional.

### 4. PP-token/span interactions (consumer boundary)

- PP04 `PpTokenScanChip` emits `PpTokenRecord { kind, span: SpanId, spelling }`
  in maximal-munch order plus exactly one zero-length `Eof` at raw source end;
  the M1 canonical token spans are `int`[0,3) `main`[4,8) `(`[8,9) `void`[9,13)
  `)`[13,14) `{`[14,15) `return`[15,21) `2`[22,23) `+`[23,24) `3`[24,25)
  `;`[25,26) `}`[26,27) `Eof`[28,28) (M1 `M1-PP-04`); the `M1-SRC-004` case must
  keep the adjacent plus signs as **two** punctuators, never `++` (`M1-PP-07`).
- **T03 owns `sources.spans` and `sources.expansions`** (unchanged). Every
  logical offset produced by T03 is remapped to a **raw physical**
  `SpanRecord.source/start/end` offset through `raw_offsets` **before** any
  `SpanDraft` is formed, so stored spans are always raw `SourceRecord.bytes`
  offsets. `SpanRecord` offsets are proposed `u64`.
- Downstream T04 `TokenRecord.span` **reuses the committed T03 PP span and writes
  no span**; only the one-PP-token->one-C-token case is exact in M1. The
  verifiable token->PP-token provenance carrier (needed for `LX14`/`LX16`) is a
  **T04/T03 `/6` co-freeze blocker** and no carrier is invented here.

### 5. Source-provenance rule

- `source_scoped_one_hop` requires the single declared `RecordRef::Source`; every
  directly referenced Span/Expansion (draft or committed) must resolve one hop to
  that `SourceId` (`SpanSourceMismatch`, **proposed name, not frozen**), and an `ArtifactDraft.source` that is
  `Some` must equal it (`ArtifactSourceMismatch`); a map-mandatory kind with no
  source is `ArtifactSourceMissing`. No recursive graph walk.
- **Deferred:** the exact source-provenance/equality relation. Per the rev 47
  delegated default there is **no source-payload-equals-artifact-`bytes`
  requirement** (normalization transforms the payload); exact provenance/equality
  and the optional-kind map rule remain pending. `ArtifactSourceMismatch`/
  `ArtifactSourceMissing` are **proposed names, not frozen**.

### 6. PP task request/result typing (candidate)

- Existing package protocol (preserved as history) is
  `PpRequest(source/token_range, cursor, expansion_context, include_context)` ->
  `PpResult(token_range, provenance)` or a typed child request.
- Candidate refinement for the M1 PP01 scope (marked delegated): the task is a
  preprocess-group task kind (`TaskKind = group<<12 | local`, group `2`,
  local range 16–39, e.g. local `normalize`), payload `RecordRef`-only, carrying
  the single `RecordRef::Source`; the result is a `RecordRef`-only tuple
  `{ artifact: RecordRef::Artifact, tokens: <pending>, spans: <pending> }` or a
  typed child request. The **exact `PpRequest`/`PpResult` variants, the token
  range representation, and the numeric local TaskKind codes remain open** and
  are not invented here.
- Timing: each tick processes **one explicit scan chunk, directive, or expansion
  frame**; chunk results must not depend on scheduling granularity. Newly
  enqueued children are executable next tick; the artifact/span/token records are
  committed once, per the M1 proposal §7 one ordered atomic commit.

### 7. Field-scoped read/write manifest (candidate)

- **Read:** `tasks.active.{id,kind,payload}`; the single live `SourceRecord`
  (`sources.bytes`/hash) named by the payload; `config.{target,dialect,options,
  limits}` **only where the rule requires it**.
- **Write (append-only, this chip's own fields only):**
  `pp.tokens` (`PpTokenDraft`), `sources.spans` (`SpanDraft`),
  `sources.expansions` (`ExpansionDraft`), `artifacts.fragments`
  (`ArtifactDraft`, kind `Normalized`), plus `wires.proposals[ChipId]` and the
  task's output slot. **No** write to `constants.records`, `lex.*`,
  `parse.*`, `symbols.*`, or any other owner's store. A whole-bus permission is
  not an adequate manifest (T01 §5).
- `phase = propagation`, `deterministic = true`, `category = emulable`,
  `backend_class = cpu-reference`.

### 8. Remaining co-freeze blockers and unresolved fields/ref strategy

- **Diagnostics as remaining co-freeze blockers:** the exact artifact error
  classification and **numeric codes** (`ArtifactSourceMismatch` /
  `ArtifactSourceMissing` / `ArtifactMapInvalid` / `bytes` limit) are open; PP01
  faults must be typed diagnostics, not panics or fabricated success.
- **Unresolved fields/ref strategy (not invented):** exact `PpRequest`/`PpResult`
  variants and token-range representation; numeric TaskKind local codes; the
  `SpanDraft`/`PpTokenDraft`/`ArtifactDraft` field-encoding and `RecordRef`
  wire tags/`RecordFamily` ordinals; the token->PP-token verifiable provenance
  carrier; the source-provenance/equality rule and optional-kind map rule; the
  `artifacts.fragments` store-field path and hash inventory; and the general
  transform-policy formula beyond the M1 cases. All remain **pending T03/T01
  co-freeze**; none is frozen here.
- **Scope guard:** this candidate does **not** claim T03/T01 signoff or a `/6`
  freeze, and does **not** extend M1 to `PP02` splice, `PP03` comment removal,
  macro expansion/paste, or multi-source (`#include`) origins, which stay
  **deferred** to the `/6` provenance/expansion design.

## PP03 comment-replacement scan-state and PP04 collaboration contract candidate (rev 55)

**Status: documentation-review remediation candidate for review finding DOC-06
(2026-10-05) — NOT a user-accepted subdecision, NOT a T03 owner or T01 `[INT]`
signoff, and NOT a `/6` freeze.** It specifies how PP03 `CommentReplaceChip`
replaces comments before the PP04 token scan without corrupting string literals,
character constants, or `#include` header-names, and how PP03 and PP04 share the
scan-state classification. It is grounded in the preserved package protocol
(`PP01→02→03→04`, one explicit scan chunk per tick) and the rev-44 map-mandatory
`CommentFree` kind. M1 exercises PP02/PP03 **identity-only** on the canonical
path (no splice/comment-removal content; rev-54 scope guard), so this scan-state
contract is **beyond-M1**; every record/store/kind/diagnostic name and
numeric code below is **proposed and open** pending T03/T01 co-freeze, and the
test matrix is planned contract-test material, **not executed tests** and **not
implementation authorization**. `/5` remains current.

### 1. Ordering, input, and scope

- PP03 input is the **PP02 spliced logical stream** (`SplicedBytes`): phase-2
  `\`-newline deletion has already run, so PP03 must not splice, unsplice, or
  re-recognize `\`-newline pairs. Useful invariant to rely on: after phase 2 the
  stream contains **no backslash immediately followed by a logical newline**, so
  a backslash seen in any state is an ordinary byte or a literal escape, never a
  line continuation.
- PP03 emits the `CommentFreeStream` bytes plus, as candidate companion state,
  an ordered **scan-state segment classification** (`CommentFreeSegment` /
  `PpScanSegment`, proposed names) that tiles the stream: every input byte
  belongs to exactly one segment, segments are in stream order, and each segment
  carries its state and its input/output ranges. Whether PP04 consumes this
  classification or re-derives it under the same co-frozen predicate is a
  T03/T01 co-freeze item; the two chips must **not** drift (§4).
- PP03 does **not** dispatch directives, evaluate conditionals, expand macros, or
  resolve includes. It performs only the **lexical include-prefix recognition**
  needed to protect a header-name (§2.1), uniformly in active and inactive
  regions (active/inactive state is PP05/PP19's concern). Macro-expanded include
  names and path policy are PP17's; directive dispatch is PP05's.
- The `CommentFree` artifact (map-mandatory, rev 44) is the PP03 producer
  artifact for the output-boundary-to-spliced-input-boundary mapping; exact map
  encoding and error classification stay open (rev 44/45/47).

### 2. Scan states (candidate; names not frozen)

PP03 is a deterministic state machine over bytes; state persists across
ticks/chunks (§3). The states are:

1. **`Code`** (start state): the only state in which `/*` and `//` start
   comments. `"` → `StringLiteral`; `'` → `CharConstant`; a recognized include
   prefix (§2.1) followed by optional whitespace/comments and then `<` →
   `HeaderAngle` or `"` → `HeaderQuoted`; every other byte is code.
2. **`StringLiteral`**: entered at `"` in `Code`; a backslash escapes the next
   byte, so `\"` does not close the literal and `\\` is a literal backslash;
   closes at the first unescaped `"`. `//` and `/*` inside are literal bytes. A
   logical newline or EOF before the closing quote is a typed
   unterminated-literal diagnostic with recovery at that newline/EOF (finite
   progress).
3. **`CharConstant`**: entered at `'` in `Code`; same escape rule; closes at the
   first unescaped `'`; multi-character constants such as `'//'`, `'\''`, and
   `'\\'` stay one segment; comment openers inside are literal; logical
   newline/EOF before the closing quote is a typed diagnostic with recovery.
4. **`LineComment`**: entered at `//` in `Code`; runs to the logical newline;
   the newline is **not** part of the comment and is preserved; comment bytes
   become whitespace (candidate: one space per comment). EOF ends the comment
   without a diagnostic.
5. **`BlockComment`**: entered at `/*` in `Code`; runs to the first `*/`;
   non-newline bytes become whitespace (candidate: one space per comment);
   **every logical newline inside is preserved in order** (the package row's
   "preserve logical newlines and positions"). EOF before `*/` is a typed
   unterminated-comment diagnostic with recovery consuming to EOF (finite;
   consistent with the `M1-NEG-01` unterminated-comment fixture).
6. **`HeaderAngle`**: entered only via include-prefix recognition (§2.1); runs
   to the first `>`; no comment recognition and no escape processing, so `//`,
   `/*`, and quotes inside are header-name bytes; logical newline/EOF before `>`
   is a typed diagnostic with recovery. Splicing has already run, so a
   header-name may occupy a logical line joined by PP02.
7. **`HeaderQuoted`**: the quoted include header-name. For comment protection
   PP03 scans it with the same escape-aware quote rule as `StringLiteral`
   (conservative candidate: a backslash before `"` does not close) and preserves
   the raw bytes; the exact spelling, implementation-defined backslash
   interpretation, and rejection are PP17's. Logical newline/EOF before the
   closing quote is a typed diagnostic with recovery.

A backslash in any literal/header-name state never re-enables comment
recognition, and comment recognition never resumes inside a segment until its
closing delimiter (or the documented recovery point).

### 2.1 Include-prefix recognition (lexical)

- Recognition applies only at the start of a logical line: optional horizontal
  whitespace, then `#` (or the dialect-gated `%:` digraph when digraphs are
  enabled), then optional whitespace, then the directive name `include` (or
  `include_next` where the dialect enables it) as a complete identifier
  boundary, then optional whitespace and comments (§2) skipped, then:
  - next significant byte `<` → `HeaderAngle`;
  - next significant byte `"` → `HeaderQuoted`;
  - anything else → not a header-name; scanning continues in `Code` from that
    byte.
- Only the literal directive spelling is recognized. `#include MACRO`,
  macro-generated `#`, and a `#` that is not at a logical line start are **not**
  header-name contexts here; PP09/PP17 handle macro-expanded include names. A
  `//` comment after `include` ends the logical line (no header-name); a block
  comment is skipped and the header-name may follow it.
- The predicate is purely lexical and independent of active/inactive state; a
  skipped conditional region still classifies header-names lexically, so comment
  removal does not depend on conditional state.

### 3. Scan-state persistence and chunk invariance

- The package protocol processes one explicit scan chunk per tick. The
  in-progress scanner state — current state, cursor, current segment start, and
  the include-prefix sub-state — is **explicit semantic state** that survives
  ticks (T03 package protocol: one explicit scan chunk per tick; wires are reset
  every tick and must not carry it). Candidate carriers: the task continuation
  or a T03-owned `pp` scan-state record. Exact store/field name and encoding
  stay open.
- Results must not depend on chunking: any split of the same spliced stream into
  chunks (including splits inside a literal, a block comment, a header-name, or
  the include prefix) yields identical segments, comment-free bytes,
  diagnostics, and PP04 tokens (`PP03-T17`).
- No hidden state: segments and output are a pure function of the spliced bytes
  plus the persisted state.

### 4. PP04 collaboration contract

- PP04 consumes the comment-free bytes and the PP03 segment classification (or a
  co-frozen shared predicate, §1) and performs maximal munch with these
  obligations:
  - comment segments are whitespace and never yield tokens;
  - a `StringLiteral`/`CharConstant` segment is exactly one pp-token of the
    corresponding kind, spelled as in the source — PP04 must not re-split at
    `//`, `/*`, or an escape;
  - a header-name segment is exactly one `HeaderName` pp-token (quoted/angle
    variant; proposed kind name, encoding open) so PP05/PP17 see it as a single
    token; outside include context `<` and `>` remain ordinary punctuators and
    no `HeaderName` is formed;
  - comment replacement whitespace prevents token gluing (`a/**/b` → `a`,`b`;
    `+/**/+` → `+`,`+`; `"a"/**/"b"` → two string tokens).
- PP04 must not re-recognize comments (none remain in the stream) and must not
  apply a different literal/header-name boundary rule than PP03. If PP04
  re-derives rather than consumes the classification, the predicate is a
  **shared co-frozen contract**, and drift test `PP03-T18` asserts byte-for-byte
  agreement.
- Spans: PP04 token boundaries in the comment-free stream remap through the
  `CommentFree` map to spliced offsets and then through the PP01 map to raw
  `SourceRecord.bytes`; stored spans remain raw per the rev-54 §4 rule.
- Diagnostics: unterminated block comment and logical newline/EOF in a
  string/character/header-name are typed PP03 diagnostics (proposed names, codes
  open), never panics and never fabricated success; each recovery makes finite
  progress.

### 5. Planned contract test matrix (not executed, not M1)

Planned T03 contract tests for the beyond-M1 PP03/PP04 slice; they are **not**
M1 fixtures, **not** executed by this document, and **not** implementation
authorization. `\n` denotes a logical newline in the PP02 output; `<EOF>` marks
end of input.

| ID | PP02 output (spliced) | PP03 expected | PP04 expected tokens | Note |
|---|---|---|---|---|
| `PP03-T01` | `char *s = "https://example";` | unchanged | `char`,`*`,`s`,`=`,StringLiteral,`;` | DOC-06 URL-in-string case |
| `PP03-T02` | `"/*not a comment*/"` | unchanged | one StringLiteral | comment syntax inside string |
| `PP03-T03` | `// "x" /* y */\n` and `/* "x" // y */` | comment → whitespace; newline preserved | no tokens from comments | quotes in comments; no state leak |
| `PP03-T04` | `'/'` `'*'` `'\''` `'\\'` `'//'` | unchanged | one CharConstant each | char constants protect comment openers |
| `PP03-T05` | `"a\"//b"` | unchanged | one StringLiteral | escaped quote |
| `PP03-T06` | `#include <a//b.h>` | unchanged | `#`,`include`,HeaderName | `//` inside angle header-name |
| `PP03-T07` | `#include "a//b.h"` | unchanged | `#`,`include`,HeaderName | quoted header-name |
| `PP03-T08` | `#include /*c*/ <a/*b*/>` | `/*c*/` → space; `<a/*b*/>` unchanged | `#`,`include`,HeaderName | comment before name vs inside name |
| `PP03-T09` | `#define X <a//b>` | `//b>` → space + newline | `#`,`define`,`X`,`<`,`a` | non-include context: `//` is a comment |
| `PP03-T10` | `a /*b*/ c` | `a` space `c` | `a`,`c` | PP02 deleted `\`+LF between `/` and `*`, forming `/*` |
| `PP03-T11` | `// c more\n` | comment → space + newline | none on that line | splice extended a `//` comment |
| `PP03-T12` | `"abcd"` | unchanged | one StringLiteral | splice inside string |
| `PP03-T13` | `#include <ab.h>` | unchanged | `#`,`include`,HeaderName | splice inside header-name |
| `PP03-T14` | `a/**/b` ; `+/**/+` ; `"a"/*x*/"b"` | comment → space | `a`,`b`; `+`,`+`; two StringLiterals | no token gluing |
| `PP03-T15` | `a/*x\ny*/b` | `a` space, newline preserved, `b` | `a`,`b` | multi-line block comment |
| `PP03-T16` | `x /*`+`<EOF>` ; `"abc\n` ; `#include <a\n` | typed diagnostic + finite recovery | per recovery | unterminated comment/string/header-name |
| `PP03-T17` | `PP03-T01`–`T16` split at every chunk boundary | identical result | identical tokens | chunk invariance; state persists |
| `PP03-T18` | all rows | — | PP04 boundaries == PP03 segment boundaries | PP03/PP04 drift test |
| `PP03-T19` | `%:include <a//b>` | unchanged | `#`-equivalent,`include`,HeaderName | dialect-gated digraph prefix |
| `PP03-T20` | `// eof-no-newline`+`<EOF>` ; `/**/` | comment → space; EOF no diagnostic | none | boundary cases |

### 6. Open items (not invented here)

- Exact segment record family/store/fields and the `CommentFree` map encoding.
- The `HeaderName` pp-token kind, its quoted/angle variant, and numeric encoding;
  whether PP04 consumes PP03 segments or re-derives the shared predicate.
- Exact diagnostic names/numeric codes for the unterminated/recovery cases.
- The whitespace folding policy (one space per comment vs retained whitespace)
  for non-newline comment bytes; logical newline preservation is fixed.
- Scan-state persistence store/field and its commit ordering.
- Dialect gates for `%:`, `include_next`, trigraph-era behavior, and non-ASCII
  byte handling in header-names/identifiers.
- All of the above remains **pending T03/T01 co-freeze**; this candidate claims
  no `/6` freeze and authorizes no code.

| Revision | Date | Summary |
|---|---|---|
| Rev 44 | 2026-10-05 | Recorded the user-accepted `ArtifactRecord { kind, source, bytes, raw_offsets }` shape and total eight-`ArtifactKind` map rule (map-mandatory `Normalized`/`Spliced`/`CommentFree`/`Preprocessed`; map-optional `Assembly`/`Object`/`Snapshot`/`Trace`), with the M1 exercised scope only single-source `Normalized`; other producers and the multi-source map deferred. Exact `raw_offsets` invariant/error mapping and enum numeric codes remain open. Doc-only; no freeze, no code. |
| Rev 45 | 2026-10-05 | Recorded the user-accepted mandatory-map invariants (`raw_offsets.len() == bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<= source.bytes.len()`, mandatory kinds require a valid source). Optional-kind map rule, source-versus-payload equality, and exact artifact error classification/numeric codes remain open. Doc-only; no freeze, no code. |
| Rev 47 | 2026-10-05 | Recorded the integration-agent-selected candidate default under explicit user delegation (optional-map kinds have empty `raw_offsets`; `source` valid when `Some`; no source-payload-equals-bytes requirement; source-provenance/equality details deferred; exact numeric error codes open). Marked as **agent-selected under user delegation, not a T03/T01 owner signoff and not a `/6` freeze**, pending T03/T01 co-freeze. Doc-only; no freeze, no code. |
| Rev 53 | 2026-10-05 | Recorded the integration-agent-selected candidate default under explicit user delegation for the **M1 single-source `Normalized` `raw_offsets` semantics** (read-only-grounded): `raw_offsets[i]` is the raw-source boundary for output boundary `i`; output `[a,b)` maps to raw `[raw_offsets[a], raw_offsets[b])`; retained bytes map corresponding boundaries; `CRLF`→`LF` output `LF` start maps to raw `CR` start and end maps after the raw `LF`; an inserted terminal `LF` maps both boundaries to raw `EOF` (zero-width); identity/final-LF/CRLF/whitespace M1 cases deterministic. Stated as a **primary location map, not complete provenance** for deleted splice ranges/macro/paste/multisource (deferred). Marked as **agent-selected under user delegation, not a T03/T01 owner signoff and not a `/6` freeze**; exact error behavior/codes and any general transform policy remain open; pending T03/T01 co-freeze. Doc-only; no freeze, no code. |
| Rev 54 | 2026-10-05 | Added a compact **M1 PP01 producer/consumer contract candidate** for single-source `Normalized` input only: exact source input (one live `RecordRef::Source`, source-scoped one-source limit), normalization output (identity / inserted terminal `LF` / `CRLF`→`LF` / whitespace variant), `ArtifactRecord` publication under the accepted rev 44 shape + rev 45 mandatory-map invariants + rev 53 delegated `raw_offsets` boundary convention, PP-token/span interactions (PP04 maximal munch + T03-only `sources.spans`/`sources.expansions`, raw remap before `SpanDraft`, T04 reuses committed PP span), the `source_scoped_one_hop` source-provenance rule with deferred equality (no source-payload-bytes equality per rev 47 delegated default), a delegated `PpRequest`/`PpResult` typing/timing candidate, a field-scoped read/write manifest, and the remaining co-freeze blockers (artifact diagnostics/classification and numeric codes, exact `PpRequest`/`PpResult` variants, TaskKind local codes, ref/provenance carrier, hash inventory). Noncritical ambiguity uses a conservative candidate marked delegated; unresolved fields/ref strategy listed, not invented. Explicitly **not** a T03/T01 signoff and **not** a `/6` freeze; does **not** extend M1 to splice/comment/macro/multi-source (deferred). Doc-only; no CDR/other-file/code change. |
| Rev 55 | 2026-10-05 | **Documentation-review remediation candidate for DOC-06 (not user-accepted, not a T03/T01 signoff, not a `/6` freeze).** Specified the PP03 `CommentReplaceChip` comment-replacement **scan-state contract** (code / string-literal / char-constant / line-comment / block-comment / angle-header-name / quoted-header-name states; escapes honored; comments recognized only in code state; logical newlines preserved; unterminated/recovery diagnostics; lexical `#`/`%:` `include`/`include_next` prefix recognition with no directive dispatch; explicit scan state persisted across chunks/ticks) and the **PP04 collaboration contract** (comment-free bytes + scan-state segment classification; protected literals/header-names stay single pp-tokens; `HeaderName` in include context; no token gluing; span remap through the `CommentFree` map; shared-predicate drift test). Added a planned 20-row beyond-M1 contract test matrix (URL-in-string, comment syntax in string, quotes in comments, char constants, escaped quotes, quoted/angle include names, non-include `<...>`, splice-created/inside comments/strings/header-names, token gluing, multi-line block comment, unterminated cases, chunk invariance, drift, digraph prefix, EOF/empty-comment boundaries) and annotated the historical PP03/PP04 rows and scheduling line with pointers. All record/store/kind/diagnostic names and numeric codes remain open, pending T03/T01 co-freeze; M1 exercises PP02/PP03 **identity-only** (splice/comment-removal semantics beyond-M1; see M1 vertical §7). Doc-only; no code, no CDR/other-file change. |
| Rev 56 | 2026-10-05 | Wording clarification (docs-only; no freeze, no code, no CDR/other-file change): the scope statements now say M1 exercises `PP02`/`PP03` **identity-only** on the canonical path (matching the M1 vertical §7 chip-list scope), rather than "M1 does not exercise PP02/PP03"; splice/comment-removal semantics remain beyond-M1. |
