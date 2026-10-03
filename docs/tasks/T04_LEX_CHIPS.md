# T04: Final C Token and Literal Chips

Prerequisite T01; input is PP04/final expanded tokens; directory `chips/lex/`. Read pp tokens, config target/dialect, source locations; write lex tokens/literal records and proposals. Preprocessing numeric values must not be treated directly as valid C constants.

Protocol `LexRequest(pp_token_id/range, mode)` → `TokenResult`; numeric parsing uses child tasks; multiple tokens may be merged into a string. Decoded results for integers and FP retain the source spelling, format, and bit pattern for subsequent diagnostics/IR.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| LX01 TokenClassifyChip | PpToken → LexTask | Dispatch identifier/number/literal/punctuation; EOF is unique | Unknown character, bad pp-number, empty input |
| LX02 IdentifierDecodeChip | IdentifierSpelling → NameId | UCN/encoding/dialect character rules; stable intern | Valid Unicode, illegal escape, ID for identical spelling |
| LX03 KeywordClassifyChip | Name/mode → KeywordOrName | Standard and GNU keywords/aliases per dialect | typedef names are not forcibly converted to keywords in the lexer |
| LX04 PunctuatorDecodeChip | Punctuator → TokenKind | All C operators, digraph mapping, ellipsis | `<:`, `%:`, `...` and `.` |
| LX05 IntegerRadixChip | NumberSpelling → DigitSequence/base | Decimal/octal/hexadecimal and binary where the corpus requires it; validate digits | `09`, 0, empty 0x, huge constant |
| LX06 IntegerSuffixChip | NumberTail → Suffix | U/L/LL order, case, extended suffixes controlled by mode | `ULL`, `LLU`, repeated U, mixed case ll |
| LX07 IntegerValueChip | Digits/base → UnsizedBits | Big-integer decoding with no host overflow and range diagnostics | Beyond u64, int128 candidate, leading zeros |
| LX08 IntegerTypeSelectChip | Bits/base/suffix/target → TypedLiteral | C candidate lists: decimal and non-decimal differ | 2147483648, 0xffffffff, ULONG boundary |
| LX09 FloatSyntaxChip | NumberSpelling → FloatParts | Decimal/hex exponent, decimal point, suffix lexing | `0x1.fp+2`, bad exponent, `1e+foo` |
| LX10 FloatValueChip | FloatParts/format → FloatBits | Correct rounding to the target float/double/long double, supporting subnormals | halfway, overflow/underflow, binary128; no host f64 approximation |
| LX11 EscapeDecodeChip | LiteralBody → CodeUnits | Simple/octal/hex/UCN escapes, target character encoding | Greedy hex, octal at most 3 digits, illegal UCN |
| LX12 CharacterLiteralChip | Prefix/body → TypedCharacter | Plain/wide/extended characters, integer values and multicharacter policy | '\xff', L character, empty literal, multichar |
| LX13 StringLiteralChip | Prefix/body → StringRecord | code units, terminating zero, embedded NUL, element type | "a\0b" length, empty string, wide encoding |
| LX14 AdjacentStringChip | StringSequence → MergedString | Translation-phase merging; handle prefix compatibility and terminator | Adjacent macro output, wide/plain mix, only one trailing zero |
| LX15 LiteralExtensionChip | ExtensionLiteral/mode → Literal | GNU imaginary/standard new suffixes supported per corpus switch | Mode allows/rejects; does not misrecognize identifier |
| LX16 TokenLocationChip | OriginChain/token → TokenSpan | Bind token location to physical/logical/macro origin | Paste-generated token, span after #line |
| LX17 TokenPublishChip | DecodedTokens → TokenRange | Publish the complete payload in order; error tokens available for recovery | EOF, single publication, stable downstream ID |
| LX18 LexErrorChip | InvalidToken → Diagnostic/Recovery | Advance finitely on illegal tokens and preserve spelling; do not panic | Arbitrary-bytes fuzz with no infinite loop |

Standalone tests must verify LX08 across different target widths (synthetic models); FP tests use known bit patterns and a trusted oracle rather than host decimal output. The integrated PP→lex→parse preserves whitespace/locations, but whitespace must not change the meaning of valid tokens.
