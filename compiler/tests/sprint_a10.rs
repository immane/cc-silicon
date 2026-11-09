// ============================================================================
// sprint_a10.rs - A10 independent ABI evidence: candidate caller + reference
// callee.
//
// STATE: BLOCKED_CONTRACT (not a pass).
//
// Required evidence: assembly emitted for a candidate caller, an independent
// C callee compiled by a reference compiler for the covered signed-int AAPCS64
// subset, linking both artifacts with an external assembler/linker, and running
// the result on the pinned Linux AArch64 substrate. The reference compiler must
// never compile the candidate function under test.
//
// Boundary review at baseline 5f3c753 (`t01-c01-c06/55`, hash
// e19cf5c1dd86c7ca50da0fe429e25f4acf2ecbcfd8cd18da7efe1fd5d09d7652):
//
//   1. Normal `/55` source rejects unknown, forward, and external callees.
//      `se_call.rs` resolves a callee through the active scope chain and
//      requires an earlier file-scope function definition, so a source caller
//      cannot name a reference C callee compiled in another translation unit.
//
//   2. The candidate CLI has no IR/artifact fixture input. `--emit-asm`
//      consumes C source and `emit_m1_assembly` emits the complete source TU,
//      which must contain exactly one candidate `main`. There is no
//      function-only emission path or per-symbol selection.
//
//   3. `target_m1::validate_calls` resolves every callee SymbolId through the
//      candidate FunctionRecord set. A file-scope function symbol without a
//      candidate definition (the shape a reference callee needs) is rejected
//      as "unresolved callee symbol". There is no accepted
//      Linkage::External/unresolved-callee fixture mode.
//
//   4. No such test-only boundary is frozen in T01_COMPILER_CONTRACT.md,
//      DIRECT_CALL_SLICE.md, or DISPATCH_LEDGER.md. The ledger records this
//      direction as BLOCKED_CONTRACT, and the existing `call_abi` counterpart
//      is an ignored test, not evidence.
//
// Exact missing interface: an accepted test-only external-callee boundary that
// specifies (a) the input artifact or API used to construct a candidate caller,
// (b) how caller-only assembly is emitted without a candidate definition of the
// callee, (c) the allowed external function symbol and linkage records, and
// (d) the reference compile plus external assemble/link/run contract. Without
// that freeze, any fixture would either broaden normal source, delete/rename or
// splice `main` in emitted assembly, or define a candidate callee and call it
// "reference". All of those alter the artifact under test and are not
// independent ABI evidence.
// ============================================================================

#[test]
#[ignore = "BLOCKED_CONTRACT: no frozen test-only external-callee boundary; `/55` source rejects external callees and target_m1 rejects a callee without a candidate FunctionRecord, so a reference callee cannot be linked to a candidate caller."]
fn blocked_candidate_caller_with_reference_callee() {
    panic!(
        "BLOCKED_CONTRACT (A10, not a pass): candidate caller + independent \
         reference callee has no frozen test-only boundary. (1) Normal `/55` \
         source rejects unknown/forward/external callees because `se_call.rs` \
         requires an earlier file-scope function definition. (2) `candidate \
         --emit-asm` consumes C source only and emits the complete TU, including \
         exactly one candidate `main`; there is no IR/artifact fixture input or \
         function-only emission path. (3) `target_m1::validate_calls` requires \
         every callee SymbolId to own a candidate FunctionRecord, so an \
         external/prototype symbol is rejected as an unresolved callee symbol. \
         A candidate caller therefore cannot name a reference callee that GCC \
         compiles separately, while defining the callee as candidate code would \
         not be independent. Required interface: an accepted test-only \
         Linkage::External/unresolved-callee fixture boundary specifying the \
         exact artifact/API, caller-only emission, allowed symbol/linkage \
         records, and the external assemble/link/run contract. Until that \
         boundary is frozen, A10 remains BLOCKED_CONTRACT and no execution or \
         ABI assertion is claimed."
    );
}
