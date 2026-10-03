# T05: Taskified Parsing Chips

Prerequisite T01; the interface depends on T04 tokens and T06 scope/name/type requests. Directory `chips/parse/`. Read lex/parse frames and typed service results; write AST/declarator/frame records and proposals; do not perform type compatibility/ABI itself. During parse, scope and typedef visibility must be maintained, so it is not a pipeline fully isolated from the symbol phase.

Unified `ParseRequest(production, token_cursor, scope_id, context, binding_power)` → `ParseResult(node/declarator, next_cursor)`. All nesting goes through child tasks/continuations. Without a typedef query result, it must not guess from the string; a T01 implementation may first return a declaration AST, but during parse it must register enough name categories.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| PA01 TranslationUnitChip | TU/cursor → ExternalDeclList | Loop over external declarations/definitions until EOF, with explicit progress | Multiple decls, empty TU, bad-decl recovery |
| PA02 ExternalDeclarationChip | Cursor/context → DeclOrFunction | Distinguish function definition/declaration; old-style parameter definitions per mode | prototype and body, K&R definition |
| PA03 DeclarationSpecifiersChip | Cursor → SpecifierBundle | storage/type/qualifier/function/alignment combinations, handed to the type chip for validation | `unsigned long` ordering, typedef ambiguity |
| PA04 TypedefDisambiguationChip | Name/scope/context → NameClass | Query the ordinary namespace; distinguish type-name/expression | typedef shadowed by a local variable; `(T)*x` |
| PA05 DeclaratorChip | Cursor/specifiers → DeclaratorTree | Coordinate the pointer and direct parts; preserve correct binding | `int *f(void)` vs `int (*f)(void)` |
| PA06 PointerDeclaratorChip | Cursor → PointerChain | Pointer qualifiers/attributes at each level | Multiple const/volatile levels, not reversed |
| PA07 DirectDeclaratorChip | Cursor → DirectDeclarator | identifier/parenthesized declarator and suffix loop | Deep parentheses, array of function pointers |
| PA08 ArrayDeclaratorChip | Suffix → ArrayDescriptor | [], bound expr, static/qualifiers, [*] context | Parameter array, VLA, static in illegal position |
| PA09 FunctionDeclaratorChip | Suffix → ParameterDescriptor | `(void)`/prototype/variadic/old-style identifier list | Empty () mode differences, ellipsis, duplicate param |
| PA10 AbstractDeclaratorChip | TypeNameCursor → AbstractTree | Type names without identifiers and complex parentheses | sizeof(int(*)[3]), cast function pointer |
| PA11 AggregateSpecifierChip | Struct/UnionTokens → AggregateDecl | tag, member list, anonymous aggregate, optional body | Forward declaration, nested/anonymous, bitfield |
| PA12 EnumSpecifierChip | EnumTokens → EnumDecl | enumerators, optional value, trailing comma, dialect extensions | Duplicate name, tag reference, expression value |
| PA13 MemberDeclarationChip | MemberTokens → MemberDecl | member declarators, bitfield width, anonymous members | Unnamed bitfield, flexible member syntax |
| PA14 DeclarationFinishChip | Declarators/initializers → DeclNodes | Comma declarations, semicolons, point-of-declaration name registration requests | `int x=x;` visibility, multiple declarations |
| PA15 InitializerParseChip | Cursor → InitTree | scalar/braces/designator recursion; semantics handed to T08 | `.x`, `[2]`, nesting, trailing comma |
| PA16 PrimaryExpressionChip | Cursor → Expr | name, constant, string, parenthesized expression | Adjacent strings, nested parentheses, wrong token |
| PA17 PostfixExpressionChip | Expr/cursor → PostfixChain | Chained combinations of call/subscript/member/++-- | `p->f(x)[i]++` |
| PA18 ParseCallExpressionChip | Callee/cursor → CallExpr | Arguments are assignment-expressions, comma-separated | `f((a,b),c)`, zero arguments |
| PA19 MemberSubscriptChip | Base/suffix → Member/SubscriptExpr | ./-> and [] syntax; do not check types for now | a[b], p->x, missing member token |
| PA20 UnaryExpressionChip | Cursor → UnaryExpr | prefix ops, sizeof, alignof dispatch, cast recursion protocol | `*++p`, sizeof expression/type ambiguity |
| PA21 CastExpressionChip | Cursor/type-name decision → CastExpr | Distinguish parenthesis, cast, compound literal | `(T){1}`, `(x)+1`, typedefshadow |
| PA22 BinaryExpressionChip | LHS/binding power → BinaryExpr | Table-driven precedence climbing; correct left/right associativity | `a+b*c`, shift/compare/bitwise precedence |
| PA23 ParseConditionalExpressionChip | Condition/cursor → ConditionalExpr | ?: is right-associative; GNU omitted-middle controlled by mode | a?b:c?d:e, middle comma |
| PA24 AssignmentExpressionChip | LHS/cursor → AssignmentExpr | = and compound assignment are right-associative | a=b=c, syntactic lvalue separated from semantics |
| PA25 ParseCommaExpressionChip | Expr/cursor → CommaExpr | Lowest-precedence left-associative; does not swallow argument-separating commas | for/init expression and call boundary |
| PA26 GenericSelectionParseChip | _GenericTokens → GenericExpr | controlling expr and type/default associations | Multiple associations, missing colon |
| PA27 StatementDispatchChip | Cursor/context → StatementTask | keyword/block/label/expression dispatch | colon after identifier, does not mistake typedef |
| PA28 CompoundStatementChip | BlockTokens → BlockNode | scope enter/exit requests; interleaved declarations and statements | shadow, empty block, scope balance on error recovery |
| PA29 IfStatementChip | Tokens → IfNode | condition/then/else; dangling else binds to the nearest | Nested if, missing parentheses |
| PA30 SwitchStatementChip | Tokens → SwitchNode | condition/body and switch context frames | nested switch, body not a block |
| PA31 LoopStatementChip | For/While/DoTokens → LoopNode | Each of the three loops has its own continuation; for declaration scope | do while trailing semicolon, for missing items and declaration |
| PA32 JumpStatementChip | Return/Break/Continue/Goto → JumpNode | Optional return expr and label reference; GNU computed goto handed to extensions | return;, goto *p mode |
| PA33 LabelStatementChip | Name/Case/DefaultTokens → LabelNode | statement labels and case expr; GNU range controlled by mode | Adjacent labels, case a...b |
| PA34 ExpressionStatementChip | Cursor → ExprStatement | Empty statement or semicolon after expr | `;`, does not retry endlessly on a missing semicolon |
| PA35 StaticAssertParseChip | AssertTokens → AssertDecl | Condition/message and mode differences; semantics handed to the constant task | File/block/member position |
| PA36 AttributeParseChip | AttributeTokens → AttributeSyntax | GNU/C standard attribute syntax and attach position; does not judge semantics itself | declarator/type/member position |
| PA37 ExtensionSyntaxDispatchChip | ExtensionToken → ExtensionTask | Route stmt expr/typeof/asm/label-address tasks to T12 | GNU switch, standard-mode diagnostics |
| PA38 ParseRecoveryChip | ParseFault/context → RecoveredCursor | Synchronize to an explicit semicolon/parenthesis/brace; clean up await and scope | One error, two diagnostics; EOF; does not swallow subsequent functions |

Priority M1: PA01–10, 14, 16, 18, 20–25, 27–29, 32, 34 necessary paths; do not replace the closed loop with a large number of stubs. PA31 and the like may later be further split into three chips, as long as the protocol/responsibility stays clear. Each row requires an independent AST snapshot and child-task replay; the nesting depth limit has a structured error and does not rely on Rust stack overflow.
