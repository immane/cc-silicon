# Executable Per-Chip Task Template and Concrete Examples

T02–T13 are the initial per-chip function lists. This template is used to bind each row to the real types and fields already frozen in T01; when an interface is not frozen, do the protocol task first and do not let the LLM guess the implementation on its own.

## 1. Task Header in Each Chip's Delivery File

```text
ID / Name / Group:
Contract version/hash:
Owned files:
Task kind / payload fields / result tag:
Category / backend_class / phase:
Allowed dialects/targets:
Reads (record fields for the field/ID):
Writes (own output slot, append area, patch fields, wire slots):
Dependencies (producers of input facts / child task kinds; not chip function calls):
Preconditions:
Transition states (Start / Await / Resume / Complete / Fail):
Algorithm obligations:
Invariants:
Error codes / recovery:
Required fixtures (normal/boundary/invalid/interaction/replay):
Integration acceptance:
Known unsupported / linked torture feature IDs:
```

Naming: the struct follows the catalog name, while the task/result may differ; for example, `IntegerPromotionChip` accepts `PromoteInteger`. IDs are consistent across tests, manifests, and traces; do not conflate TaskId with ChipId.

## 2. Example A: TY25 IntegerPromotionChip

- Prerequisite: TY13/19/bitfield information is frozen; the target integer model exists. Can run in parallel with the parser, testing with synthetic TypeRecord.
- task: `PromoteInteger { source_type: TypeId, bitfield: Option<BitFieldInfoId>, context: ConversionContext }`.
- result: `PromotionResult { source, destination: TypeId, cast_kind, unchanged }`; when result types need interning, submit the corresponding mechanical store proposal.
- reads: the active task payload; `types[source].{kind,rank,width,signedness,enum_info}`; bitfield width/rank; `config.target.integer_model.{int_width,int_rank,unsigned_int_width}`; does not read AST/IR/symbol names.
- writes: its own PromotionResult slot, necessary canonical type append proposals, its own wire completion/fault. It must not modify the source type in place, must not directly cast AST, and must not advance the phase.
- Algorithm: determine the types/bitfields to which integer promotion applies; use int if int can represent all values of that type, otherwise unsigned int; leave the original type unchanged for inapplicable cases. Enums/special integers follow the frozen dialect rules and do not use a sample value to decide the promotion type.
- Errors: invalid TypeId / an inapplicable kind contradicting the task context → structured contract fault; "already long so unchanged" is success, not Unsupported.
- Normal: signed char, signed short; boundary: two synthetic targets where unsigned short is exactly covered by int / cannot be covered; bitfield: a small unsigned field and a full-width one; unchanged: int/unsigned/long; invalid ID.
- Replay: the same payload and target produce a result ID/type consistent with the trace; for a non-matching task the bus is unchanged except for scheduling diagnostics.
- Integration: SE03, SE09, SE23 consume the result; IR07 must generate a real conversion; returning correct on type-check alone while missing the codegen extension is not acceptable.

## 3. Example B: PA05 DeclaratorChip

- Prerequisite: TokenKind, the declarator tree node, the PA04 query/PA06/07 task/result, and the continuation fields are frozen.
- input: `ParseDeclarator { cursor, scope, allow_abstract, specifier_context }`; output `ParsedDeclarator { tree_id, next_cursor, name }`.
- reads: the active payload, the corresponding token kinds/spans, the parser's own frame, completed child results.
- writes: its own frame and tree append proposals, ParsePointer/ParseDirect child task wire proposals; does not write the final TypeStore and does not do layout.
- State machine: Start checks context → enqueue Pointer/Direct tasks → Waiting(child) → Resume combines the tree → Complete; child failures propagate, the cursor has progress or clearly waits; new children execute next tick.
- Preserve syntactic association; TY20 is responsible for interpreting it as a type; PA05 must not run the TY20 chip along the way.
- cases: `int *f(void)`, `int (*f)(void)`, `int (*a[3])(int)`, `int (*(*f(void))[3])(double)`; abstract type names; unbalanced parentheses; name shadowing.
- invariant: next_cursor is within the token range, the tree has no illegal cycles, child results are consumed only once. Deep nesting advances through the task stack rather than the Rust recursion stack.

## 4. Example C: CG05 VariadicAbiChip

- Prerequisite: the va_list layout of the frozen AAPCS64 revision/sysroot; TY28 default promotions; the CG01–03 class/location schema.
- input: `PlanVariadic { signature, named_parameters, operation: Call | Start | Arg(type) | Copy | End }`.
- result: the caller-side argument plan or the callee register save area / cursor update / read-position plan.
- reads: target ABI, argument type/layout, named arg locations, that function's va state; does not read the host libc's va_list.
- writes: its own VaPlan and machine-request proposals; does not emit assembly directly and does not change other functions' frames.
- tests: 8 or more GP, FP arguments, mixed unnamed args, HFA/aggregate rules, independent advance of va_copy, overaligned arg, large indirect aggregate; GCC two-way mixed linking.
- Target: obey Linux AAPCS64, not Darwin arm64 stack-only variadic; input for another ABI must be rejected. Record the policy for illegal promotion requests to va_arg.

## 5. Extending Chip Rules

When the corpus discovers a new feature: first define the task/result and the exact rules, add a unique ChipId and function table, and reuse existing service interfaces. When a single entry point includes multiple independent language rules, it must be further divided; "compatible with all GNU" cannot be taken as a single-chip goal.

A new chip, per the DoD, covers at least one real failure or specification fixture and is associated with motherboard routing and a manifest; one contract change requires retesting all dependent packages. The T00 denominator does not change during feature extension.
