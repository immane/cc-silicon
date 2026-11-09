// ============================================================================
// sprint_a09.rs — A09 candidate callee + independent reference caller
//
// Baseline: 5f3c753 (`t01-c01-c06/55`).
// State: BLOCKED_CONTRACT (not a pass).
//
// Boundary review:
//   * `candidate` requires a source `main`; a callee-only source is rejected
//     with `program has no main function` (candidate.rs compile path).
//   * `target_m1::emit_m1_assembly` requires exactly one external
//     `int main(void)`: a module without it fails `target emit: missing main`,
//     and a second definition fails `target emit: duplicate function symbol main`.
//   * `--emit-asm` always emits every committed function of the whole source
//     translation unit, including `main`; there is no function-selection or
//     callee-only emission mode.
//
// The intended two-object link is therefore impossible without changing the
// artifact under test: the candidate-defined callee object would also define
// `main`, while the independently compiled reference caller needs its own
// `main` entry. Deleting, localizing, renaming, or otherwise rewriting the
// candidate `main` is explicitly disallowed. No frozen test-only boundary for
// function-only emission exists at `/55`.
//
// The fixtures below preserve the intended callee/caller shapes and expected
// value so an accepted boundary can be exercised without re-deriving them.
// Do not unignore this test while the boundary is missing.
// ============================================================================

/// Candidate-side callee under test. The current driver rejects this
/// callee-only source before emission because it has no `main`.
const CANDIDATE_CALLEE: &str = "int candidate_callee(int a, int b, int c) {\n\
    return a * 7 + b * 3 + c;\n\
}\n";

/// Independent reference caller. It must be compiled by the reference
/// compiler, never by the candidate, and it supplies the process entry point.
const REFERENCE_CALLER: &str = "int candidate_callee(int a, int b, int c);\n\
int main(void) { return candidate_callee(1, 2, 3); }\n";

/// `1 * 7 + 2 * 3 + 3 = 16`; position-dependent coefficients make argument
/// reversal or register mix-ups observable in the process exit status.
const EXPECTED_EXIT: i32 = 16;

#[test]
#[ignore = "BLOCKED_CONTRACT (not a pass): baseline 5f3c753 /55 has no frozen function-only emission boundary. `candidate` requires a source `main`, and `candidate --emit-asm` always emits the whole TU including `main`; an independent reference caller also needs `main`, so the link has a duplicate entry. Removing, renaming, or rewriting the candidate `main` would alter the artifact under test and is prohibited."]
fn candidate_callee_with_independent_reference_caller() {
    let _fixtures = (CANDIDATE_CALLEE, REFERENCE_CALLER, EXPECTED_EXIT);
    panic!(
        "BLOCKED_CONTRACT (not a pass): candidate callee + independent reference caller \
         is not executable at `/55`. The candidate driver rejects a callee-only source \
         with `program has no main function`, while `target_m1::emit_m1_assembly` \
         requires exactly one external `int main(void)` (`target emit: missing main` / \
         `target emit: duplicate function symbol main`). `--emit-asm` emits the full \
         translation unit including `main`, so a reference caller's `main` would \
         duplicate the entry symbol. No frozen function-only emission mode or \
         test-only callee-only artifact boundary exists. Stripping or renaming the \
         candidate entry is prohibited. Next action: T01 must accept a narrow \
         function-selection/callee-only emission interface before this fixture can \
         exercise the two-object ABI link."
    );
}
