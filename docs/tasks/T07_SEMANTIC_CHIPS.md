# T07: Expression, Statement Semantics, and Side-Effect Chips

Prerequisite T01; interfaces depend on T05 AST/T06 types/T08 constants. Directory `chips/semantics/`. Read AST/symbol/type/config and service results; write typed AST, conversion plans, value category and effect facts. Must not directly generate target instructions. Rule errors go through diagnostic proposals, do not panic.

Unified `CheckRequest(NodeId, ScopeId, FunctionContextId)` → `CheckedNode(type, category, conversions, effects)`. Use tasks for different node children; type/layout/constant requests and AST child waits are all explicit. Must not stuff the entire semantic recursion into one helper.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| SE01 NameExpressionChip | NameNode/scope → BoundExpr | name binding, distinction of function/enumerator/object, undeclared mode policy | typedef cannot be a value; GNU old-style implicit declaration |
| SE02 LiteralExpressionChip | LiteralNode → TypedExpr | type and value category/storage of constant/string | string is an array; enum literal is integer |
| SE03 UnaryArithmeticChip | Unary+/−/~ operands → CheckedUnary | type domain, promotion, result type, overflow semantics | ~unsigned char promoted first, floating-point ~ rejected |
| SE04 LogicalOperationChip | logical NOT/AND/OR operands → CheckedLogical | scalar constraint, result int, short-circuit effect dependency | pointer condition, rhs must not be executed early |
| SE05 AddressExpressionChip | &operand → CheckedAddress | addressable domain, bitfield/register prohibited, function/array special | &*p must not incorrectly load, bitfield cannot be & |
| SE06 DereferenceExpressionChip | *operand → CheckedDeref | pointer pointee, function designator, restrictions on use of incomplete type | void* dereference value policy, function pointer |
| SE07 ArithmeticBinaryChip | +−*/ operands → CheckedBinary | usual conversions, integer/floating domain, signed/unsigned facts | mixed rank, signed division, unsigned wrap |
| SE08 PointerArithmeticChip | Ptr/int or Ptr/Ptr → CheckedPointerOp | element size/layout request, difference ptrdiff_t, completeness | array stride, function/void* GNU mode |
| SE09 ShiftBitwiseChip | Shift/bitwise → CheckedBinary | each side of shift promoted separately, result is the left type; bitwise common type | negative/too-wide shift flagged, not computed with host shift |
| SE10 ComparisonChip | ==/!=/rel operands → CheckedCompare | arithmetic/pointer/null compatibility, result int | void* comparison, qualified pointer, NaN semantics |
| SE11 AssignmentChip | LHS/RHS → CheckedStoreExpr | modifiable lvalue, conversion, assignment result category | const/array assignment rejected, struct assignment |
| SE12 CompoundAssignmentChip | LHS/op/RHS → CheckedRmwExpr | LHS evaluated only once, intermediate arithmetic type then converted back to lhs | `a[i++] += x` exactly one i++ |
| SE13 IncrementChip | Prefix/Postfix target → CheckedIncExpr | scalar/modifiable conditions, pointer stride, old/new result | volatile single semantic access, postfix returns old value |
| SE14 ConditionalExpressionChip | Condition/arms → CheckedConditional | scalar condition, arithmetic/void/aggregate/pointer common type | two-pointer qualifier merge, null arm, GNU a?:b |
| SE15 CommaExpressionChip | Left/Right → CheckedComma | discard lhs, rhs result type, C non-lvalue result, sequence | `(a,b)=1` standard-rejected; side-effect sequencing |
| SE16 CallExpressionChip | Callee/args → CheckedCall | callable determination, argument conversion, return type, call effects | variadic, old-style prototype, recursive call |
| SE17 MemberExpressionChip | Base/member → CheckedMember | ./-> constraints, qualifier propagation, bitfield category | const struct member, anonymous member |
| SE18 SubscriptExpressionChip | Base/index → CheckedSubscript | equivalent to pointer add + deref, supports i[a] | multidimensional array, incomplete element |
| SE19 SizeAlignExpressionChip | sizeof/alignof operand → CheckedSizeAlign | unevaluated context and VLA exception, type completeness, size_t | sizeof i++ not executed; sizeof VLA per C rules |
| SE20 GenericSelectionChip | Controller/associations → SelectedArm | controlling expression not evaluated, matches compatible type, duplicate/default restrictions | _Generic(i++,...) no side effect; duplicate compatible |
| SE21 ReturnStatementChip | Return/function → CheckedReturn | void/nonvoid and conversion/aggregate return | return expr in void policy, struct return |
| SE22 LoopJumpChip | Break/Continue/context → JumpTarget | nearest loop/switch break, continue only loop | continue in switch targets outer loop, error outside loop |
| SE23 SwitchCaseChip | Switch/cases → CheckedSwitch | integer promotion, duplicate after case conversion, range overlap | unsigned case collision, nested switch independent |
| SE24 LabelGotoChip | Labels/gotos/function → CheckedTargets | undefined/duplicate label, VLA scope jump-in restriction | forward goto, goto into VLA diagnostic |
| SE25 ConditionStatementChip | If/loop condition → CheckedCondition | scalar condition, generate explicit truth conversion | struct condition rejected, pointer truth |
| SE26 EffectSequencingChip | CheckedExpr tree → EffectGraph | sequence edges, unevaluated contexts, unsequenced diagnostic/UB facts | `i++&&f()` sequencing; `i++ + i++` must not be arbitrarily defined |
| SE27 VolatileAccessChip | QualifiedAccess → VolatilePlan | volatile load/store explicitly non-deletable/non-mergeable, aggregate access policy | repeated reads retained, compound RMW semantics |
| SE28 AtomicAccessChip | AtomicExpr/order → AtomicPlan | atomic load/store/RMW, order validation and result type | _Atomic++, invalid order combination, non-atomic must not be disguised |
| SE29 FunctionDefinitionChip | FunctionAST/decls → CheckedFunction | parameter definition, old-style parameter types/promotions, definition consistent with prototype | K&R float parameter, duplicate/missing parameter, scope |
| SE30 AlignmentSpecifierChip | AlignmentSyntax/type → AlignmentRequirement | _Alignas/alignas constant/type form, weakening and position constraints | 0 ignored, invalid non-power-of-two, weaker than natural alignment |

SE26 does not require turning C's unspecified evaluation order into a language definition; the implementation may choose one legal and stable order, but optimization must preserve sequenced-before constraints. UB cases should not be fixed to a wrong result merely to match a single GCC output; the T00 oracle is only used for well-defined programs, and the original torture assertions must still be observed.

Each chip tests cast/effect in the typed AST, not merely diagnostic strings. Complex numbers, vectors, and GNU nodes have their specific rules completed by T12, then return a unified CheckedNode; T09 must not re-infer types.
