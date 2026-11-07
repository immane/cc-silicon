# OP Freeze 02 (DRAFT ONLY): T10 OP04 SimplifyIdentity + OP09 DeadInstruction Legality

Status: **OPEN — DRAFT ONLY, PROPOSED**. Not an accepted contract, not an ADR, not a freeze. Authorizes no chip code, no bus/schema change, no implementation.

Scope: T10 rows OP04 (SimplifyIdentityChip) and OP09 (DeadInstructionChip) only. Normative sources remain `T01_COMPILER_CONTRACT.md`, `T10_OPTIMIZE_CHIPS.md`, the Silicon paradigm spec, and the SFL contract. If this draft conflicts with any of those, the existing accepted document wins.

## 1. Purpose

Propose the legality guards under which an identity simplification (OP04) or a dead-instruction deletion (OP09) may be emitted as a guarded patch for OP20:

1. OP04 proposes `Rewrite` patches for algebraic identities only when the rewrite preserves observable effects, FP semantics, and trap behavior.
2. OP09 proposes `DeleteOp` patches for unused instructions only when the instruction is provably pure.
3. Both defer version-guard checking, transactional apply, and invalidation to OP20/OP21 (see `OP_FREEZE_03.md` draft); this file only constrains when a candidate is legal to propose.

Goals: no deleted effect, no miscompiled volatile/atomic/call, no incorrect FP identity, no speculative trap introduction or removal that changes defined behavior.

## 2. Common guards (both chips, proposed)

A candidate patch is legal only if ALL of the following hold on the frozen tick input:

1. **Effect class is pure:** the target instruction's effect class is `Pure` in the frozen effect facts. Any of the following makes the candidate illegal: volatile access, atomic access, store, call, unknown/undeclared effect, unclassified intrinsic/asm, or a memory-epoch-unknown load. Unknown means impure.
2. **Volatile guard:** if any operand, result, or accessed object is volatile-qualified (per frozen type/effect facts), reject. Applies even when the value looks algebraically redundant (e.g. `volatile x + 0`).
3. **Atomic guard:** any atomic ordering, fence, or atomic-qualified access rejects. Atomic ops are never identity-simplified or dead-deleted by these two chips.
4. **Call guard:** a call (known, unknown, indirect, helper, builtin) is never rewritten or deleted by OP04/OP09, even if its result is unused. Call clobbers also block rewrites that reorder memory operations across the call (alias/memory-epoch check required where applicable).
5. **FP guard:** any floating-point identity requires an explicit frozen FP-mode match (rounding mode, `fenv` access, fast-math flags). Default (strict) mode allows NO FP identity that changes NaN payload/propagation, signed zero (`x + 0.0`, `x - 0.0`, `x * 0.0`), infinities, subnormals, rounding, exceptions, or contraction behavior. FP identities are allow-listed per rule ID, never derived ad hoc.
6. **Trap/speculation guard:** the rewrite/deletion must not introduce a new fault on any path (divide-by-zero, invalid shift, trap-on-overflow, faulting load) nor remove a fault the source program must exhibit. Zero-trip-loop and unexecuted-arm cases follow the same rule: no additional fault, no disappearing mandatory fault.
7. **Evidence declared:** the patch names the rule ID plus every relied-upon fact (effect class, volatility/atomicity flags, FP mode, use-count/alias epoch where relevant) in `legality_evidence` so OP20 can check versions. A patch omitting a relied-upon fact is malformed.

## 3. OP04 SimplifyIdentity (proposed)

- **Allowed shape:** integer-only algebraic identities from a closed rule list (illustrative, unfrozen): `x + 0 -> x`, `x - 0 -> x`, `x * 1 -> x`, `x / 1 -> x`, `x & x -> x`, `x | x -> x`, `x ^ 0 -> x`, `x << 0 -> x`, plus boolean `x && 1`-style normalizations only as explicitly listed by the integrator. Each rule states its domain exclusions (e.g. signed-overflow, shift-width, division UB per T08/T10 integer rules).
- **Forbidden for OP04:** any FP identity in default mode (including `x + 0.0`, `x * 1.0`, `x - x -> 0.0`); any pointer-arithmetic identity that changes provenance/bounds facts; any rewrite touching volatile/atomic/call operands; any rewrite conditioned on flag/option state the chip did not read and declare.
- **Operand checks:** the "identity element" operand must be a frozen constant of the exact operand type/width (no host-width substitution, no implicit conversion); the surviving operand keeps its type, value category, and sequence-point/effect annotations unchanged.
- **Result:** a `Rewrite`-family guarded patch carrying the before-image, after-image, `base_ir_version`, relied-upon analysis versions, and rule ID. OP04 performs no direct IR write and bumps no version.

## 4. OP09 DeadInstruction (proposed)

- **Pure-only delete rule:** OP09 may propose `DeleteOp` only for an instruction that (a) passes all §2 guards, (b) has zero live uses in the frozen use-def facts at the frozen IR version, and (c) is not a terminator, phi source, block-label target, address-taken value, or exported/root result.
- **Explicitly non-deletable:** volatile/atomic accesses, stores, calls, branches/returns/unreachable-affecting terminators, instructions with unknown effects, and loads under an unknown alias/memory epoch (unknown call/store between the load and its would-be deletion point blocks deletion). An unused load is deletable only under a frozen must-not-alias/no-intervening-effect proof declared in evidence.
- **No reordering:** OP09 deletes; it does not hoist, sink, or merge. Any movement belongs to a different (future) chip and its own legality file.
- **Result:** a `DeleteOp` guarded patch with tombstone-safe target refs and the use/effect versions relied upon. Deleted IDs stay tombstoned per T01; reuse is a protocol error.

## 5. Proposed acceptance (for a future freeze)

- `volatile int x; x + 0` is not simplified; `float x + 0.0` is not simplified in default FP mode; `x * 1` on pure integers is (per rule list).
- An unused pure integer op with no live uses is deletable; an unused volatile load, atomic op, call result, or unknown-effect op is not.
- An unused load crossed by an unknown call/store is not deleted.
- Every admitted candidate carries complete `legality_evidence`; incomplete evidence is rejected as malformed.
- Same frozen inputs → same propose/reject outcome (deterministic replay).

## 6. Open questions (blocking a freeze)

1. Closed OP04 identity rule list with per-rule domain/UB exclusions — integrator-owned.
2. Frozen FP-mode record and fast-math flag schema the FP guard reads.
3. Effect-class taxonomy location (T01 vs. T09 IR records) and unknown-effect encoding.
4. Use-def liveness definition at version boundaries (what "zero live uses" reads).
5. Load-deletion alias proof standard (must-not-alias vs. epoch equality).
6. Numeric encoding/hash coverage of new records under the T01 `/6` foundation.

## 7. What this draft does NOT do

- Freezes nothing; changes no bus, schema, contract version, or chip manifest.
- Adds no chip code, no tests, no workspace changes.
- Does not authorize parallel implementation of OP04/OP09 or any other OP chip.
- Does not claim any pass rate, target behavior, or optimization capability.
