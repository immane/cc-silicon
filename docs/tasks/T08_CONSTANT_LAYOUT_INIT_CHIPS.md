# T08: Constant, Layout, Initialization, and Dynamic Object Chips

Prerequisite T01; read T06 types, T07 checked facts or constant contexts to be checked. Directories `chips/constants/`, `chips/layout/`, `chips/initializers/`. Write constants/layout/init stores and proposals, do not read the host ABI. Semantic constant restrictions and optimization fold are strictly distinguished.

`ConstantRequest(node, required_kind)` → `ConstantResult(bits/float/symbol+addend, legality)`; `LayoutRequest(type, target, pack)` → `LayoutId`; `InitRequest(type, init_tree, storage)` → `InitPlan`. A symbolic address is not equal to a host pointer. VLA size and cleanup are left to the runtime plan and must not be passed off as static constants.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| CL01 ConstantContextChip | Expr/context → LegalConstantOrFault | integer constant expr, different constraints for arithmetic/static/address constants | comma/call must not arbitrarily enter ICE; sizeof boundary |
| CL02 ConstantUnaryChip | Op/constant → Constant | fixed-width +−~!, overflow/invalid diagnostic by context | INT_MIN negation, unsigned ~, bool |
| CL03 ConstantBinaryChip | Op/constant pair → Constant | target conversion, /%, shift, comparison bitwise, no host overflow | signed remainder, over-wide shift, division by zero |
| CL04 ConstantBranchChip | Logical/conditional → Constant | compute only the selected branch, ensure short-circuit legality judgment conforms to context | 0&&1/0, 1?3:1/0 |
| CL05 ConstantCastChip | Constant/target type → Constant | truncate/extend, float/int, _Bool, pointer constant conversion | -1→unsigned, fraction truncation, overflow |
| CL06 AddressConstantChip | Symbol/path/index → SymbolicAddress | link-time address, member/array addend, null pointer | global+offset, automatic object non-static initialization |
| CL07 StaticAssertChip | ICE/message → AssertionResult | assertion evaluation and diagnostic location | 0 failure, non-ICE, target sizeof |
| CL08 ScalarLayoutChip | ScalarType/target → SizeAlign | all target scalars including complex/vector per the model | char to long double, int128 non-host value |
| CL09 ArrayLayoutChip | ElementLayout/bound → ArrayLayout | constant array multiplication, overflow, incomplete/VLA distinction | huge bound, incomplete, nested arrays |
| CL10 StructLayoutChip | Members/pack/target → StructLayout | member offsets, padding, tail padding, alignment | char+long, nested, aligned/packed |
| CL11 UnionLayoutChip | Members/target → UnionLayout | maximum size/align, roundup and special members | union align greater than member size |
| CL12 BitFieldLayoutChip | Fields/target policy → BitFieldLayout | allocation units, signedness, zero-width, packed and endianness | cross unit, unnamed zero-width, mixed base type |
| CL13 FlexibleArrayChip | Aggregate tail → FlexibleLayout | must-be-at-end and other constraints, sizeof excludes dynamic tail, GNU difference | non-tail, only FAM, nested restriction |
| CL14 MemberOffsetChip | Type/member path → OffsetPlan | anonymous paths, bitfield cannot be offsetof, builtin designator | nested offsetof, array path, bitfield rejected |
| CL15 VlaBoundChip | Bounds/scope → VlaPlan | bound evaluation timing/once-only, save size/stride and lifetime | multidimensional VLA, side-effect bound, negative/0 policy |
| CL16 ScalarInitializerChip | Type/init expr → ScalarInitPlan | assignment conversion, static constant restrictions, braced scalar | pointer null, float/int, excess elements |
| CL17 AggregateInitializerChip | Type/init tree → SubobjectPlan | braces/elision advance by current object, struct/union selection | nested without brace, array of struct |
| CL18 DesignatedInitializerChip | Designators/type → SubobjectPath | .field/[index], subsequent position, duplicate override; GNU ranges as an extension | sparse array, out-of-range, duplicate init |
| CL19 StringInitializerChip | ArrayType/string → StringInitPlan | complete bound, allow exact bound without trailing zero, prefix compatibility | char a[3]="abc", char a[], wide |
| CL20 IncompleteArrayInferChip | IncompleteArray/init plan → CompletedType | maximum designated index/pos determines bound | sparse [10]=1, empty GNU mode |
| CL21 ZeroInitializeChip | UnspecifiedSubobjects → ZeroPlan | unspecified member target zero value, including FP/pointer layout | padding policy, union, nested aggregate |
| CL22 StaticDataEmitChip | InitPlan/layout → BytesAndRelocs | target endian bytes, symbol relocation, do not generate host address | struct padding, global pointer+addend |
| CL23 AutomaticInitPlanChip | InitPlan → RuntimeInitOps | source side effects/subobject order, memzero/store/copy explicit | local aggregate, designator expression |
| CL24 CompoundLiteralChip | Type/init/context → ObjectPlan | file static/block automatic, lvalue and object identity | same-block lifetime within loop, &literal |
| CL25 VlaLifetimeChip | Scope exits/jumps → StackLifetimePlan | scope exit/return/break/goto cleanup, must not jump into scope | nested VLA, early return, dynamic stack restoration |
| CL26 LayoutValidateChip | Layout/init records → ValidatedLayout | offsets/size/alignment/bounds compatible, reject stale layout | bitfield covering unit, init out of bounds, pack stack |

Dependencies: TY types → CL08/09/10/11/12/13 → CL16–24; CL18 needs layout paths but the semantic selection does not depend on actual host bytes. CL15/25 output IR lowering requests. CL22 outputs to the T11 data/relocation emitter.

Performance is not prioritized over precision: a huge array initialization must not allocate a node per implicit zero; range/zero-fill plans are permitted, and determinism records their semantics. long double and bitfields must be cross-checked against a same-target GCC ABI probe.
