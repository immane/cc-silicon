# T06: Scope, Symbol, and Type Chips

Prerequisite T01. Directories `chips/symbols/`, `chips/types/`. Read symbols/types/config and task inputs; write this owner's records/proposals. Symbol addition/deletion can only be done by the designated declaration/scope tasks; type interning is not permitted to parse the AST incidentally. parse/sem is served through task requests and must not call the chip methods below.

Type records cover the types required by the corpus, such as scalar, pointer, array (complete/incomplete/VLA), function (prototype/old-style/variadic), struct/union, enum, qualified/atomic, GNU vector/complex, etc. target-dependent layout is not privately computed here; it is handed to T08.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| TY01 ScopeEnterChip | Parent/kind → ScopeId | block/function/prototype scope and visibility intervals | nested scopes, prototype tag range |
| TY02 ScopeExitChip | ScopeId → ClosedScope | close visibility, retain records for typed AST reference | shadow restoration, duplicate exit rejected |
| TY03 OrdinaryNameLookupChip | Name/scope/cursor → SymbolOrMissing | ordinary namespace contains variables/functions/typedef/enumerators, lookup by point of declaration | point-of-declaration, self-initialization, shadow |
| TY04 TagNameLookupChip | Name/tag-kind/scope → TagSymbol | tag is an independent space, inner/outer shadowing and incomplete identity | struct X vs variable X; different kind conflict |
| TY05 LabelNameLookupChip | Name/function → LabelSymbol | label is function scope, may be referenced before definition | forward goto, not visible across functions |
| TY06 MemberNameLookupChip | Type/Name → MemberPath | aggregate member space and anonymous member paths | anonymous duplicate name, nested offset path |
| TY07 SymbolDeclareChip | Decl/scope → SymbolId | declaration registration, kind/location, compatible redeclaration request | same-scope conflict, legal outer shadowing |
| TY08 TypedefRegisterChip | Name/Type/scope → TypedefSymbol | typedef category for parse lookup, redefinition rules by mode | typedef vs variable conflict, equivalent typedef |
| TY09 RedeclarationChip | Old/NewDecl → MergedDecl | compatible/composite type, uniqueness of definition | incompatible prototype, duplicate function body |
| TY10 LinkageResolveChip | Decl/context → Linkage | internal/external/none, inherits linkage of previously visible declaration | static followed by extern, block extern |
| TY11 StorageDurationChip | Decl/context → Duration/Storage | automatic/static/thread/allocated model, register restrictions | file local static, _Thread_local combination |
| TY12 TentativeDefinitionChip | TU/global declarations → FinalDefinitions | tentative merging, incomplete array completion, common option | `int a;` multiple times, `int a[];`, extern |
| TY13 BuiltinTypeChip | SpecifierBundle/target → TypeId | basic type combination, signedness, rank, dialect legality | long long, signed char, invalid long float |
| TY14 QualifiedTypeChip | Base/qualifiers → QualifiedType | const/volatile/restrict/_Atomic placement and deduplication | pointer vs pointed qualifier; invalid restrict |
| TY15 PointerTypeChip | Pointee/qualifiers → PointerType | fixed pointer type identity, incomplete pointee is legal | void*, function pointer, recursive struct |
| TY16 ArrayTypeChip | Element/bound kind → ArrayType | incomplete/constant/VLA, element completeness, parameter adjustment request | array of function rejected; zero-bound GNU |
| TY17 FunctionTypeChip | Return/params/mode → FunctionType | parameter adjustment, prototype/old-style, variadic, return restrictions | array parameter becomes pointer, function returning function rejected |
| TY18 AggregateTypeChip | Tag/members → AggregateType | tag identity, forward completion, anonymous independent type | self-pointer, duplicate complete, same name different scope |
| TY19 EnumTypeChip | Enumerators/target/mode → EnumType | enumerator value type/range, underlying strategy, name registration request | negative value, large unsigned, duplicate enumerator |
| TY20 DeclaratorBindChip | Base/DeclaratorTree → DeclaredType | correctly combine pointer/array/function from the syntax tree | pointer array vs array pointer, complex function returning pointer |
| TY21 TypeCompatibilityChip | TypePair/context → Compatible | recursive compatibility relation, qualifiers/prototype/array and tag identity | old-style vs prototype, different struct incompatible |
| TY22 CompositeTypeChip | CompatiblePair → CompositeType | merge array bound and function information without losing qualifiers/ABI | incomplete→complete array, prototype merge |
| TY23 LvalueConversionChip | ExprType/category/context → ConvertedType | lvalue conversion, qualifier removal, exception contexts | sizeof/&/assignment lhs not incorrectly converted |
| TY24 ArrayFunctionDecayChip | Expr/context → DecayedExprType | array/function decay and sizeof/&/string-init exceptions | &array differs from array; sizeof function policy |
| TY25 IntegerPromotionChip | IntegerType/target → PromotedType | integer promotion of rank/range and bitfield/enums | unsigned short can be covered by int; bitfield |
| TY26 ArithmeticConversionChip | OperandTypes → CommonType/casts | integer rank/signed rules, real/complex floating hierarchy | signed long+unsigned int; float+double |
| TY27 AssignmentConversionChip | Source/destination/context → ConversionPlan | arithmetic, compatible aggregate, pointer qualifier/null/_Bool | const-discard diagnostic, void* interchange, 0 vs variable 0 |
| TY28 ArgumentConversionChip | Function/arg types → ArgumentPlans | prototype count/type, variadic tail, default promotions | float→double tail, char→int, void prototype |
| TY29 ExplicitCastChip | Source/destination/mode → CastPlan | scalar/void convertible domain, pointer-integer target semantics | ptr↔integer, invalid aggregate cast, float-to-integer boundary |
| TY30 AtomicTypeChip | Base/context → AtomicType | _Atomic specifier/qualifier and atomic object access model | array/function atomic rejected, qualified base |
| TY31 DeclarationConstraintChip | Decl/type/context → ValidatedDeclaration | storage/qualifier/specifier combinations, object completeness, parameter and member constraints | file register, void object, invalid storage combination |
| TY32 InlineLinkageChip | FunctionDecl/mode → DefinitionEmissionPolicy | C99/GNU89 inline, extern/static combinations and TU definition selection | two-TU linking, extern inline mode difference |
| TY33 EffectiveTypeChip | Object/access history → EffectiveTypeFacts | declared/allocated object, character access, memcpy propagation and alias constraints | char alias, allocated store, union policy |
| TY34 RestrictContractChip | RestrictedPointers/scopes → RestrictFacts | restrict-associated block/access relations, conservatively provide optimization basis | alias unknown, do not assume no alias; legal restrict example |

Key points: TY21's recursive type graph needs a visited pair to avoid cycles; it is not a recursive chip call. TY13's target parameter comes from T01 and must not hardcode the host width. Conversion results must contain actual cast nodes/plans and must not merely change the type in a way that makes the IR lose sign/zero extend.

Additional acceptance: each conversion constructs positive and negative cases with the target bitvector; type and lookup results are fully serializable; during parse, the order of TY01/07/08/03 is made explicit by an integration fixture. GNU vector/typeof type services, once built by T12, still reuse the compatibility protocol here.
