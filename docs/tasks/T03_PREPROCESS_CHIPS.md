# T03: Source Normalization and Preprocessing Chips

Prerequisite T01; integration depends on CT02/CT08/CT06. Directory `chips/preprocess/`. Read `sources/config/pp/tasks`; write its own `pp` tokens/macros/frames and source-map records, with persistent modifications going through commit; must not read files directly. Results are handed to T04.

Protocol: `PpRequest(source/token_range, cursor, expansion_context, include_context)`, outputting `PpResult(token_range, provenance)` or a typed child request. Preprocessing expressions are first macro-processed, then evaluated against the PP integer model; it must not directly use a general C expression evaluator and ignore PP rules. Each tick processes one explicit scan chunk, directive, or expansion frame; chunk results must not depend on scheduling granularity.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| PP01 SourceNormalizeChip | SourceBytes → NormalizedBytes/map | Newlines and the selected standard translation phase; trigraphs in legacy mode according to dialect | CRLF, missing final newline, mode differences |
| PP02 LineSpliceChip | NormalizedBytes → SplicedBytes/map | Backslash-newline splicing before comment/token recognition | Multi-line identifier, string, comment |
| PP03 CommentReplaceChip | SplicedBytes → CommentFreeStream | `/* */` and `//` become whitespace; preserve logical newlines and positions | Quotes in comments, tokens must not be incorrectly glued, unterminated |
| PP04 PpTokenScanChip | Stream/cursor → PpToken | maximal munch; pp-number, identifier, literal, punctuator | `1e+foo` is a pp-number; digraph; non-ASCII mode |
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

Scheduling dependencies: PP01→02→03→04; macro expansion PP09→10→11/13/14→12→15 (the concrete exceptions for `#`/`##` pre-expansion are expressed by the already-frozen protocol; do not blindly follow the table order). Conditional stack management and include waits persist across ticks. Macro recursion terminates by the rules, not by truncation at a small fixed count. Final acceptance prohibits using only GCC `-E` as a substitute for this package.
