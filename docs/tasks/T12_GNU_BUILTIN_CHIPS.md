# T12: GNU C, Builtin, and Special Runtime Chips

Prerequisite T01; the T00 feature census determines which priorities are required, and interfaces depend on parse/types/sem/IR/target. Directories `chips/extensions/`, `chips/builtins/`. Read the relevant input records, config dialect/features; write ext/type/checked/lower plans and proposals. Language extensions go through the CPU reference; specific target lowering is delegated to CG24/25. A builtin must not arbitrarily change the meaning of ordinary user functions by name.

Extensions are the key to approaching 99% on torture, not a "do it later when there is time". This directory's initial list is not an exhaustive commitment to all extensions of a frozen GCC; gaps found by T00 must be added as explicit chips. When unsupported, there must be structured diagnostics, failed instances are retained, and pretending that compilation succeeded is prohibited.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| EX01 ExtensionModeChip | Feature/mode/context → Allowed/Diagnostic | Standard/GNU/legacy switch, __extension__ diagnostic suppression scope | Standard rejects and GNU allows; do not expand suppression scope |
| EX02 TypeofChip | typeof operand → TypePlan | type/expr forms, usually unevaluated, variably modified exception | typeof i++, typeof VLA, side-effect boundary |
| EX03 StatementExpressionChip | ({block}) → CheckedExpr/LowerPlan | scope, last-expr result, lifetime, control flow | multiple side effects, void last stmt, escaping object |
| EX04 LabelAddressChip | &&label → LabelAddressPlan | function-local label identity and address-taken CFG facts | forward label, cross-function rejection |
| EX05 ComputedGotoChip | goto *expr → IndirectBranchPlan | pointer validation, indirect target set, target blocks must not be deleted | dispatch table, label-diff target policy |
| EX06 NestedFunctionChip | NestedDefinition → ClosurePlan | lexical captures, static chain, address-taken requires trampoline or descriptor | captured variables, recursion, lifetime after return; safety restrictions public |
| EX07 AttributeSemanticChip | AttributeSyntax/context → Attributes | aligned/packed/unused/noreturn/noinline etc. parsed by attach position | type vs decl, argument validation, unknown policy |
| EX08 SymbolAttributeChip | weak/alias/visibility/section → SymbolPlan | alias target constraints, weak symbol, section and linker contract | alias does not exist, weak linking, custom section |
| EX09 VectorTypeChip | vector_size/type → VectorType | size/element restrictions, alignment, mode/target capability | integer/FP vectors, illegal size |
| EX10 VectorOperationChip | VectorExpr → VectorPlan | lane semantics, conversions, comparison mask, subscript/shuffle | signed lanes, mask width, side-effect index |
| EX11 ComplexTypeChip | _Complex/specifiers → ComplexType | component format, literal/real/imag access semantics | complex float/double/long double |
| EX12 ComplexOperationChip | ComplexExpr → ComplexPlan | arithmetic, truth, comparison, division precision and helper policy | NaN/Inf, complex division, real↔complex |
| EX13 AsmSyntaxChip | asm tokens → AsmSyntax | basic/extended, qualifiers, template, operands/clobbers/labels | asm volatile, named operands, asm goto |
| EX14 AsmConstraintChip | AsmSyntax/target → ConstraintPlan | input/output/tied/earlyclobber/register/memory constraints | =r/+r, matching digit, memory/cc, unknown constraint |
| EX15 AsmEffectChip | ConstraintPlan → AsmIrPlan | volatile reachable effects, read/write memory, flags, goto CFG edges | optimization must not delete effectful asm, memory barrier |
| EX16 BuiltinDispatchChip | CalleeName/args/config → BuiltinTaskOrCall | __builtin_* registration, signature, ordinary libc recognition controlled by -fno-builtin | user redefinition, unknown builtin, wrong arity |
| EX17 BuiltinTypeQueryChip | Type query args → Constant/type | types_compatible_p, classify_type and corresponding GNU rules | qualifiers handling, struct identity |
| EX18 BuiltinChooseChip | choose_expr condition/arms → ChosenExpr | compile-time selection, non-chosen not evaluated, type/category preserved | unselected side effects, ICE condition, lvalue result |
| EX19 BuiltinConstantPChip | Expr/optimization context → ConstnessValue | conservative legal determination, does not evaluate the operand, per mode/pass policy | complex non-const may be 0; must not return 1 for a wrong constant |
| EX20 BuiltinExpectChip | Value/hint → ValueAndBranchHint | value semantics unchanged, hint metadata, probability validation | side effect once, hint does not change program result |
| EX21 BuiltinBitOpsChip | clz/ctz/popcount/parity/ffs/bswap → BitOpPlan | per-suffix width, 0 boundary/UB domain, byteswap bit-exact | 32/64/128, ffs0, clz0 not defined arbitrarily |
| EX22 BuiltinOverflowChip | add/sub/mul_overflow → OverflowPlan | infinite-precision operation concept then converted to dest, bool overflow and stored result | different signed types, INT_MIN, pointer output |
| EX23 BuiltinMemoryChip | memcpy/memmove/memset/memcmp → MemoryPlan | constant/dynamic sizes, alias and overlap, return value/effects | overlap legal only for memmove, zero size, unaligned |
| EX24 BuiltinObjectSizeChip | Object/query → SizeOrUnknown | object_size/dynamic_object_size and unknown sentinel semantics | subobject vs whole, unknown ptr, side effects |
| EX25 BuiltinVaChip | va_start/arg/copy/end → VaOperationPlan | builtin va_list type, last-parameter rules, type/promotions | multiple GP/FP parameters, aggregate va_arg, copy independent cursor |
| EX26 BuiltinStackChip | alloca/align/frame/returnaddress → StackPlan | dynamic stack, alignment, safety-level restrictions and frame policy | loop alloca, return cleanup, unsupported level explicit |
| EX27 BuiltinAtomicChip | __atomic/__sync requests → AtomicPlan | argument order, CAS weak/expected, lock-free, width helper | compare-exchange failure write-back, fence, libatomic |
| EX28 BuiltinFloatChip | fabs/copysign/isnan/isinf/ordered etc. → FpPlan | format exact, −0/NaN, whether there is an fenv effect, suffix signature | long double, copysign -0, unordered |
| EX29 BuiltinControlChip | trap/unreachable/assume-like → ControlPlan | trap termination, UB commitment scope of unreachable, diagnostic builtins | reachable trap actually terminates; do not let the whole program be deleted |
| EX30 NonlocalJumpChip | setjmp/longjmp related → NonlocalPlan | returns-twice/clobber semantics, distinguishing builtin special saves from libc calls | changed automatic locals policy, optimization across setjmp |
| EX31 GnuInitializerChip | Range/extension initializer → InitPlan | GNU range, duplicate designator, empty/zero-length extension | range RHS evaluation count as required by GNU, mode differences |
| EX32 RuntimeHelperChip | Wide/FP/atomic/complex operation → HelperCallPlan | symbol/signature/ABI of the fixed runtime library, missing helper explicitly fails | int128 division, binary128 conversion, complex helper |
| EX33 AutoTypeChip | __auto_type/auto declaration → InferredType | inferred from the initializer per dialect rules, restricted name visibility and declarator | self-reference, pointer initializer, no init rejected |
| EX34 BitIntTypeOperationChip | _BitInt width/operation → Type/OperationPlan | provide arbitrary legal bit-width rank, conversion, layout, and codegen plan when census requires | non-power-of-2 width, unsigned, >128 bit helper, ABI probe |
| EX35 ExtendedFloatTypeChip | _FloatN/_FloatNx/mode → FormatTypePlan | supported target format and literal suffix, ABI/runtime plan | _Float16/_Float128, unsupported format explicitly fails |
| EX36 C23CompatibilityChip | C23 syntax/semantics request → StandardFeaturePlan | split into concrete child tasks per census: nullptr/constexpr/typeof_unqual/new enum rules etc. | standard-version differences; must not accept everything without checks |
| EX37 BuiltinStringChip | strlen/strcmp/strchr etc. → StringOperationPlan | exact return value/boundary, legal constant fold or libc call, effects explicit | embedded NUL, unknown pointer, fno-builtin |
| EX38 BuiltinMathChip | math builtin/signature → MathOperationPlan | suffix format, errno/fenv option, libm or native policy | sqrt negative, signed zero, long double, math errno |
| EX39 BuiltinPrefetchChip | Address/hints → PrefetchPlan | address expression evaluation and hint validation, target may have no prefetch but preserves specified effects | side effects once, invalid hint, no-native policy |

EX34–36 are census-triggered task entry points, **not a single chip responsible for an entire new standard**: if the corpus requires a feature, before execution split it into independent syntax/type/operation/ABI chips per TASK_TEMPLATE; each child chip enters the overall registry and test ledger. An entry chip in the list may only dispatch explicit child tasks and must not hide a complete-feature helper. Missing required child chips still count as gaps.

Delivery must include a feature↔testcase ledger. Builtin unit tests record well-defined domains, UB/unspecified domains, and implementation-defined decisions; a uniform "all builtins return 0" must not be used to make compilation pass. __builtin_constant_p being conservative does not mean any builtin may be folded arbitrarily.

The executable memory/safety policy required for the trampoline output by EX06 belongs to the generated program's target capability and is not the same thing as the Rust framework's own `forbid(unsafe_code)`. A fixed environment and explicit support are required; safety restrictions must not cause a candidate to be skipped yet be deleted from the denominator.

Integration with system headers: predefined macros, __builtin_va_list, attributes, target typedefs must support a real sysroot; simplified headers hand-made for bring-up may only be a separate profile and do not replace the final frozen sysroot. Other builtins (string, math, prefetch, target intrinsics, new-standard features) continue to be split into independent chips according to the actual T00 gaps, not hidden inside an EX16 catch-all match.
