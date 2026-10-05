# T11: AArch64 Linux ABI, Machine IR, and Assembly Chips

Prerequisite: T01 target **identity** frozen and concrete ABI values **probe-verified** (T01 §6); interfaces depend on T09 IR / T08 static data. Directory `chips/target/aarch64/`. Reads typed IR/layout/config; writes ABI plans, machine IR, frame, assembly, and proposals. category=device_specific, backend_class=aarch64-linux; this is a compilation target, not a cc-silicon execution backend.

The first version emits GNU/Linux AArch64 assembly text acceptable to the GNU assembler; the Host invokes an external assembler/linker, and that Host-side toolchain produces the ELF object. T11 owns assembly emission, not ELF writing; implementing an ELF object writer in-house is a later optional project. The Part B compile gate must produce an object. Darwin symbol prefixes, Mach-O directives, or the Darwin variadic ABI must not be used. All AAPCS64 rules are governed by the AAPCS64 specification and the **probe-verified** target model; do not rely on "arm64 is all the same".

> **Probe-gated qualifier (docs-only, 2026-10-05; no authority decision).** Every concrete ABI value named in the table below — scalar widths and `long double`/`__int128` formats, GP/FP register counts, HFA/HVA bounds, the 16-byte aggregate/stack thresholds, the variadic register save area, `x18` policy, relocation/TLS forms, and target feature availability — is an **AAPCS64/Linux expectation to confirm**, not a frozen value (T01 §6). Until a Linux probe reports `verified=true` and the T01 integrator incorporates the report hash (C02), T11 work is **Part B and blocked** (M1 P4/P5, G12): the acceptance column states the required probe/fixture coverage, not verified values. Coverage limits to carry forward: the probe interprets `abi.gp_arg_regs`, `abi.fp_arg_regs`, `abi.stack_align`, and `abi.variadic_register_save_area` from captured assembly evidence rather than deriving them numerically; HVA is not exercised; and `x18` policy, relocation/TLS forms, and target-feature availability have no probe fixture at all. Those items remain unverified even after a passing probe until classifier/integration review or a fixture extension, and Darwin/Mach-O values remain prohibited substitutes.

Protocols `AbiRequest(signature/layout)`, `SelectRequest(ir_op)`, `AllocateRequest(function_mi)`, `EmitRequest(mi/frame)` → typed plan/result. The intermediate machine IR must mark use/def/clobber, flags, memory, call effects, and stack constraints.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| CG01 ScalarAbiClassChip | ScalarType → AbiClass | GP/FP classes, extension, 128-bit scalar, target widths | char/long/int128/long double |
| CG02 AggregateAbiClassChip | AggregateLayout → AbiClass | HFA/HVA identification, register/stack or indirect, non-homogeneous aggregate | 1–4 homogeneous FP members, nested, packed |
| CG03 ArgumentLocationChip | AbiClasses → ArgLocations | GP/FP allocation, alignment, stack arguments, and split rules | more than 8 GP/FP, int128 alignment, aggregate boundaries |
| CG04 ReturnLocationChip | ReturnClass → ReturnPlan | scalar/multi-reg/FP/indirect result, special sret rules | >16-byte struct, HFA, void |
| CG05 VariadicAbiChip | Function/call/va operations → VaPlan | default promotions, register save area, va_list and va_arg | mixed int/double/struct, va_copy, stack tail |
| CG06 CallPreservationChip | Call/MI liveness → ClobberPlan | caller/callee saved, partial FP saving, reserved regs | live across call, indirect call, x18 policy |
| CG07 ParameterEntryChip | ParameterPlan → EntryMoves | map incoming regs/stack to locals/vregs; aggregate reconstruction | many arguments, address-taken, variadic |
| CG08 CallSequenceChip | CallIR/locations → CallMI | outgoing stack, parallel moves, indirect callee, result recovery | register cycle, nested call, sret |
| CG09 ReturnSequenceChip | ReturnIR/plan → ReturnMI | result moves/copy, unified epilogue, return after cleanup | multiple returns, aggregate, FP |
| CG10 ConstantSelectChip | Integer/FP constant → MI | integer materialization, FP constants/pool, bit-exact | arbitrary 64-bit, −0, NaN payload |
| CG11 IntegerAluSelectChip | Add/sub/bitwise/unary → MI | 32/64-bit, narrow already extended, flags constraints | wraparound, immediate range, ~ |
| CG12 MultiplySelectChip | Mul/highmul/wide → MI | fixed-width mul and wide integer lowering; mark helpers when necessary | signed/unsigned int128, carry |
| CG13 DivideRemainderChip | Div/rem → MI/helper | signed/unsigned; rem constructed from the correct relation; wide division helper | negative remainder, uint64 high bits, int128 |
| CG14 ShiftSelectChip | ShiftIR → MI | arithmetic/logical, variable counts, wide segmentation | signed >>, int128 spanning 64 bits |
| CG15 CompareSelectChip | CompareIR → Flags/ValueMI | signed/unsigned predicate, bool materialization | high-bit unsigned, pointer compare |
| CG16 BranchSelectChip | Terminators → BranchMI | conditional/unconditional/switch, long branch policy | fallthrough, far jump, default |
| CG17 AddressSelectChip | AddressIR → AddressMI | base+offset/index, large offset, symbol relocations | nested layout, negative offset, VLA stride |
| CG18 LoadStoreSelectChip | AccessIR → MemoryMI | width, sign extension, align/unaligned, volatile | packed member, small type, large offset |
| CG19 FloatAluSelectChip | FpArithmetic/format → FpMI/helper | float/double native; binary128 and complex helper policy | subnormal, rounding, long double |
| CG20 FloatCompareSelectChip | FpCompare → PredicateMI | correct condition codes for ordered/unordered; all NaN relations | != NaN, < NaN, !float |
| CG21 NumericConvertSelectChip | CastIR → ConversionMI/helper | sign/zero extension, int↔FP for all widths, narrow back | unsigned64→double, double→int, binary128 |
| CG22 AggregateMoveSelectChip | Copy/zero → MI/runtime call | small inline or explicit memcpy/memmove/memset; overlap semantics | struct copy, alignment, nontrivial size |
| CG23 AtomicSelectChip | AtomicIR/target features → MI/helper | exclusive/CAS loops/fences or runtime; must not be replaced with ordinary loads | acquire/release, weak CAS, 128-bit helper |
| CG24 IntrinsicSelectChip | Builtin/vector/complex plans → MI/helper | consume T12 legal plans; features controlled by config | missing feature results in explicit fallback/rejection |
| CG25 InlineAsmSelectChip | AsmPlan → ConstrainedMI | operands/ties/earlyclobber/memory/cc/labels constraints | register clash, asm goto, volatile asm |
| CG26 MachineUseDefChip | MI → UseDef/Clobbers | explicit/implicit regs, flags, memory, call clobbers | flags live, asm clobbers, partial FP saving |
| CG27 LivenessChip | MICFG/use-def → LiveInOut | fixed point, edge use after phi elimination, loops | loop back edge, switch, multiple blocks |
| CG28 LiveIntervalChip | Liveness/positions → Intervals | splits/holes, register classes/fixed regs, call crossings | branch holes, fixed arg regs, ties |
| CG29 RegisterAssignChip | Intervals/constraints → Assignment | an initial linear scan is acceptable; GP/FP classes and precoloring | pressure > available regs, callee-save selection |
| CG30 SpillInsertChip | Unassigned/spill plan → SpillMI | stack slots, reload/store, scratch constraints, re-analysis | two spilled operands, int128, near a call |
| CG31 ParallelMoveChip | MoveSet → ScheduledMoves | break cycles with scratch or temporary slots; protect live values | 2/3-cycle, GP/FP, stack↔stack |
| CG32 FrameLayoutChip | Locals/spills/calls/VLA → StackFrame | 16-byte alignment, callee saves, large frames, dynamic sp | big local array, variadic save area |
| CG33 PrologueEpilogueChip | Frame → EntryExitMI | SP adjustment, save/restore, frame pointer policy, stack probe needs | leaf/nonleaf, omit FP, dynamic stack, large frame |
| CG34 SymbolRelocationChip | Symbol/config → RelocPlan | local/global/weak/visibility, PIC/PIE/GOT/PLT/TLS | external function/global, local address, TLS |
| CG35 AssemblyInstructionChip | FinalMI → AssemblyLines | legal register/op/immediates; encoding range/pseudo-instruction policy | accepted by the assembler; all virtual regs eliminated |
| CG36 AssemblyDataChip | Bytes/relocs/symbols → DataSections | text/data/bss/rodata, align, size, common/no-common | zero globals, strings, symbol+addend |
| CG37 AssemblyFunctionChip | FunctionLines/metadata → FunctionArtifact | symbols, labels, sections, GNU stack, and necessary unwind/debug policy | multi-function linking, duplicate label, recorded -g limitation |
| CG38 TargetVerifyChip | FinalMI/frame/ABI → ValidatedArtifact | no unassigned vreg, SP/save conventions, operand constraints, legal target | wrong stack align/call clobber/reloc rejected |
| CG39 PhiEliminationChip | SsaCFG/phis → EdgeMoveMI | SSA exit, critical edge splitting, parallel copies; do not break the CFG | loop phi, diamond, critical edge cycle |

Mandatory ABI tests: candidate callee + reference GCC caller, and the reverse; covering scalar, small/large struct, HFA, variadic, long double, and int128. The reference may only generate the independent ABI counterpart and must not compile the function under test on behalf of the candidate.

If `long double` and wide integers go through libgcc/compiler-rt, H01 fixes the symbols, ABI, and library version; T12 provides the RuntimeHelper plan; CG calls are verified the same as ordinary C calls. Introducing a helper is not proof of "supporting all formats"; every operation/conversion needs a fixture.

Native Mach-O, new ISA/vector extensions, and a self-written linker do not automatically enter the current primary target; cases required by the frozen corpus that are unsupported are still recorded as failures, and the candidate's limitations must not be erased with target-exclude.
