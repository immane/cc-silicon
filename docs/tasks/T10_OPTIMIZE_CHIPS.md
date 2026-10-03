# T10: Optimization Chips, Legality, and Analysis Invalidation

Prerequisite T01, T09 IR. Directory `chips/optimize/`. Reads IR/config/analysis versions; writes this pass's analysis/rewrite proposals. No worker is permitted to directly delete an instruction another worker is reading; ApplyRewrite commits uniformly and bumps the version. The goal is semantic correctness first; do not sacrifice execute for the number of compile cases passed.

Protocols `Analyze(function/version)`, `RewriteCandidate(instruction/version, rule)` → analysis or guarded patch. Every optimization must have a before/after IR interpretation / target run diff. The advanced passes below are not inherently necessary to reach 99%, but observable opt options/scan assertions must be satisfied truthfully, and unknown options must not silently succeed.

| ID / Chip | Input → Output | Function and Goal | Specified Acceptance |
|---|---|---|---|
| OP01 PassSelectChip | OptConfig/function → PassPlan | O0/O1/O2/O3/Os/Og plans with explicit enable/disable rules | traceable config matrix, debug policy |
| OP02 FoldIntegerChip | PureIntegerOp → ConstantPatch | target fixed-width arithmetic; respect signed overflow/shift/UB domains | unsigned wrap, INT_MIN/-1 not incorrectly folded |
| OP03 FoldFloatChip | PureFpOp/options → ConstantPatch | exact-format rounding; respect NaN/−0/fenv/fastmath | x*0 must not be folded by default; NaN compare |
| OP04 SimplifyIdentityChip | Op/facts → Rewrite | legal identities such as x+0; do not delete effects/incorrect FP rules | volatile x, floating-point x+0, signed zero |
| OP05 CopyPropagateChip | Def/use facts → OperandPatch | valid defs and kills; must not propagate incorrectly across unknown stores | alias store, call clobber |
| OP06 LocalValueNumberChip | Block ops/effects → RedundancyPatch | pure expr equivalence within the same block and memory epoch | commutative integer, load invalidated by store |
| OP07 FoldBranchChip | ConstantCondition → CfgPatch | constant conditional/switch to a single edge; maintain phi/CFG | dead arm effects not executed, phi updated |
| OP08 RemoveUnreachableChip | Reachability → DeleteBlockPatch | entry-reachability graph; preserve address-taken/indirect targets | computed goto targets, exception/nonlocal edges |
| OP09 DeadInstructionChip | Use/effect facts → DeleteOpPatch | delete unused pure ops; do not delete volatile/atomic/call | unused load, unknown call, side effects |
| OP10 CfgSimplifyChip | CFG/version → MergePatch | merge empty blocks/single predecessors; preserve labels/phi/cleanup | loop back edge, switch, indirect label |
| OP11 UseDefAnalysisChip | FunctionIR → UseDef | correctly enumerate ordinary/phi/terminator/memory operands | recompute after delete/replace, aggregate |
| OP12 DominatorChip | ReachableCFG → Dominators | fixed point and unreachable policy; entry/loop correct | diamond, irreducible loop |
| OP13 DominanceFrontierChip | CFG/dom → Frontiers | provide merge points for SSA; no hidden coupling with dom computation | nested diamond, loop header |
| OP14 PhiPlacementChip | VariableDefs/frontiers → PhiPlan | only eligible locals; address-taken/volatile are not promoted | loop carried, uninitialized path |
| OP15 SsaRenameChip | PhiPlan/dom → SsaIR | explicit traversal stack/version stack; all uses bound to correct defs | back edge phi, non-SSA slots preserved |
| OP16 SparseConstantChip | SsaCFG → Value/edge facts | joint fixed point over the lattice and executable edges | cyclic phi, undef not mistakenly treated as 0 |
| OP17 AliasAnalysisChip | Memory ops/types/options → AliasFacts | conservative points-to/effective type/restrict; unknown means may alias | union/char alias, memcpy, pointer casts |
| OP18 LoopAnalysisChip | CFG/dom → LoopInfo | loop members/exit/preheader; irreducible handled conservatively | multi-exit, nested, no preheader |
| OP19 LoopInvariantChip | Loop/alias/effect facts → HoistPatch | dominance/use, speculation/trap, memory safety | a zero-iteration loop must not fault additionally, volatile/call |
| OP20 ApplyRewriteChip | GuardedPatches/version → UpdatedIR | check conflicts/version; transactional application, invalidation, verify | stale patch rejected, failure does not partially modify |
| OP21 AnalysisInvalidateChip | MutationSummary → InvalidatedFacts | invalidate all related analyses according to data/CFG/memory changes | old dom/alias must not continue to be used |
| OP22 OptFixpointChip | Worklist/progress → Continue/Done | enqueue affected neighbors; deterministic budget and termination | rule oscillation detection, same initial state gives same fixed point |

OP02 and OP03 must not call T08 chips, but may reuse the bitvector/IEEE mechanical arithmetic library that carries no language context; C ICE legality still belongs solely to T08. Future rules such as inlining/unroll/vectorize must be added as independent chips according to T00 scan/feature gaps, and must not falsely claim that all optimization capabilities are supported by ignoring flags.
