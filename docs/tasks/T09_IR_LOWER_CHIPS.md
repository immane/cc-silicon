# T09: AST-to-CFG/IR Chips

Prerequisite T01; interfaces depend on T07 CheckedNode and T08 layout/init. Directory `chips/ir/lower/`. Reads typed AST/constants/layout/effects; writes IR and lower frames/proposals. The base IR is typed, explicit CFG, load/store, calls, and control terminators; the first version permits non-SSA — correctness first, optimization later.

`LowerRequest(node/function, current_block, value/address/discard_context)` → `LowerResult(value/address/blocks)`. All implicit conversions come from CheckedNode; use explicit ops to express signedness, overflow policy, volatile/atomic, alignment, and memory effects. Subtree lowering goes through tasks, with no mutual calls; after a block ends, ordinary instructions must not be appended again.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| IR01 FunctionBeginChip | FunctionDecl → FunctionIR/frame | Build entry, parameter objects, function context, and an ABI-neutral signature | recursive func, multi-function ID isolation |
| IR02 BlockCreateChip | Parent/context → BlockId | Stably create blocks and register scope/control role | stable IDs for the same task sequence, empty block |
| IR03 ConstantLowerChip | ConstId → IrConstant | integer/FP bit patterns, symbol+addend, string addresses | signed zero, relocatable pointer |
| IR04 ObjectAddressLowerChip | Symbol/object → Address | abstract addresses for local/global/param/static/TLS, etc. | address-taken local, shadow, array |
| IR05 LoadLowerChip | Address/type/access → Value | width/sign/alignment, volatile/atomic attributes | signed char extension, unaligned packed member |
| IR06 StoreLowerChip | Address/value/access → StoreOp | width/store effect after exact conversion; assignment return value | small int truncation, volatile preserved |
| IR07 ConversionLowerChip | CastPlan/value → ConvertedValue | sext/zext/trunc, int/float, pointer, truth conversion | -1 unsigned, pointer bool, float fraction |
| IR08 UnaryLowerChip | CheckedUnary → UnaryOps | +−~! on the promoted operand and the target type | INT_MIN policy, float -0 |
| IR09 ArithmeticLowerChip | CheckedBinary → ArithmeticOps | signed/unsigned division and remainder, FP ops; must not blindly apply host computation | div/rem with negative numbers, unsigned overflow |
| IR10 PointerArithmeticLowerChip | PointerPlan/values → AddressOrDiff | element stride, ptrdiff, VLA stride | ptr++, two-dimensional VLA indexing |
| IR11 CompareLowerChip | CheckedCompare → CompareValue | signed/unsigned/pointer/FP ordered/unordered | NaN !=, unsigned high-bit comparison |
| IR12 ShortCircuitLowerChip | LogicalExpr → BranchCFG/value | RHS executes only conditionally; join values use explicit slots for non-SSA | conditional execution of logical AND/OR RHS side effects |
| IR13 ConditionalLowerChip | ConditionalExpr → ArmCFG/value | conversion ensuring both arms have a consistent type; effects of the chosen arm | aggregate result, GNU a?:b evaluates a once |
| IR14 AssignmentLowerChip | StoreExpr → StoreAndValue | lhs address computed only once; rhs and conversion according to effects | valid ordering of a[i++]=f() |
| IR15 CompoundRmwLowerChip | RmwExpr → LoadOpStore/value | address once, operation type then convert back, prefix/postfix old and new values | *p++ += x, volatile/atomic select different ops |
| IR16 MemberSubscriptLowerChip | Member/SubscriptPlan → Address | layout offsets, anonymous path, array/VLA stride | nested packed member, i[a] |
| IR17 BitFieldLowerChip | BitFieldAccess → Extract/InsertOps | mask/shift/sign extend, preserve other fields in the same unit | signed high bits, cross-unit according to layout, volatile policy |
| IR18 CallLowerChip | CheckedCall → CallIR | operand conversions, indirect/direct, aggregate/variadic signature | function pointer, mixed args, nested call |
| IR19 ReturnLowerChip | CheckedReturn → ReturnTerminator | save the return value and return after scope cleanup | VLA cleanup, do not lose aggregate |
| IR20 IfLowerChip | IfNode → IfCFG | condition, then/else, merge; already-terminated branches are not branched to again | both sides return, no else |
| IR21 LoopLowerChip | LoopNode → LoopCFG | for/while/do condition/step/continue/break targets | continue to for step, do condition, nested |
| IR22 SwitchLowerChip | CheckedSwitch → SwitchCFG | promoted case values, default, fallthrough, range | labels landing in nested block, no default |
| IR23 JumpLabelLowerChip | Goto/label/break/continue → CFGEdges | forward label fixup, scope cleanup; indirect goto is delegated to extensions | forward goto, loop jumps, VLA |
| IR24 InitLowerChip | RuntimeInitPlan → InitIR | zero ranges, stores, copy, side effects as planned | sparse array, local struct, string copy |
| IR25 AggregateCopyLowerChip | AggregateValue/dest → CopyIR | copy according to layout with necessary volatile semantics; alias overlap constraints | struct assignment, union, packed |
| IR26 DynamicStackLowerChip | VlaPlan/alloca → DynamicStackIR | size/align, stack save/restore, spill policy and lifetime | multiple VLA, early return, loop cleanup |
| IR27 AtomicLowerChip | AtomicPlan → AtomicIR | load/store/RMW/CAS/fence, preserve memory order | cmpxchg expected update, fetch old/new |
| IR28 FunctionEndChip | FunctionIR/facts → CompletedFunction | resolve fixups, legal falloff handling, CFG verify request | main falloff 0, missing label, dead block |
| IR29 ExtensionPlanLowerChip | CheckedGNU/vector/complex plan → ExtensionIR | explicitly convert T12 plans to IR; component/lane ops, asm/control/helper must not lose effects | complex truth, vector lane, asm goto edges |

To be added: run IR fixtures against the defined subset using the interpreter (see T13) to confirm short-circuit/bitfield/conversion behavior; it is used only for validation and must not replace final target compilation. The non-SSA join slot/aggregate copy representation must be frozen; T10 must not assume all Values are naturally SSA.

GNU vector/complex, inline asm, setjmp/nonlocal, and nested functions have their lowering plans specified by T12, then consumed by this stage and T11; unimplemented nodes report Unsupported and must not emit a no-op.
