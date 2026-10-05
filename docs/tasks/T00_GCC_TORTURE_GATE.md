# T00: GCC Torture Acceptance, Corpus Freeze, and Host Engineering

This is a **Host/verification task package**, not an instruction to have chips directly execute external programs. The evaluation system must be established first, then language features can be developed in parallel.

## 1. First Freeze the Meaning of "99%+"

Recommended corpus baseline: GCC `releases/gcc-15.2.0`. This is a reproducible recommended version, not a "latest GCC" claim, and it is not a frozen corpus until the H00 lock resolves the full commit SHA and records the source hash, test file hashes, `.exp/.x`, auxiliary sources, headers, runtime, and DejaGnu/toolchain versions. A moving branch must not be substituted for a commit.

Primary target: the frozen compiler target identity `aarch64-unknown-linux-gnu` (ELF, LP64, little-endian, AAPCS64; [T01](T01_COMPILER_CONTRACT.md) §6). Only the identity is frozen; the concrete target values remain UNVERIFIED until the Linux probe, and the T00 corpus/environment/option freeze is still pending. Run via Linux CI/VM or a cross toolchain + runner; the reference GCC must be the same target/config. In the same mode, long double uses that target's ABI format (usually IEEE binary128), not the macOS arm64 format.

The baseline scopes are counted separately:

1. `gcc/testsuite/gcc.c-torture/compile`: generate test instances according to that release driver. GCC 15.2's `compile.exp` defaults to `assemble` and must produce an object file; succeeding with only `-S` is not sufficient.
2. `gcc/testsuite/gcc.c-torture/execute`: compile, link, and run according to the driver, and check the expected results of the test directives.
3. `gcc/testsuite/gcc.c-torture/execute/ieee`: establish a separate run suite; do not assume it is recursively included by the previous directory.
4. compat, unsorted, and other gcc.dg are not merged into the above rates by default; report their scope and uncovered list separately. Expanding the scope requires adding a frozen profile.

A missing language capability of the candidate C compiler cannot be used as a reason that a target is inapplicable. Objective target limitations are determined only by the fixed target, the original test rules, and the reference probes; freeze exclusions before execution and report their proportion of the original list.

## 2. Option Matrix

Base this on the actual computation of that release's harness; do not guess default values yourself. The default option lists differ per driver in GCC 15.2: `lib/gcc-dg.exp` defines `DG_TORTURE_OPTIONS` (six entries: the list below without `-Og -g`), consumed by `compile/compile.exp` and `execute/execute.exp` through `gcc-dg-runtest`; `lib/c-torture.exp` defines `C_TORTURE_OPTIONS` (all seven entries), consumed by `execute/ieee/ieee.exp` through `c-torture-execute`. The `C_TORTURE_OPTIONS` list is:

```text
-O0
-O1
-O2
-O3 -fomit-frame-pointer -funroll-loops -fpeel-loops -ftracer -finline-functions
-O3 -g
-Os
-Og -g
```

In the end, each instance still has to process loop conditions (loop-only options are dropped per test), `dg-options`, additional/remove options, target predicates, `.x`, and driver overrides; do not simply multiply either list by the file count to obtain the precise denominator. The frozen denominator must apply each driver's own list to its own suite. When the harness detects LTO support it appends its LTO option list as well; the frozen non-LTO profile must exclude those rows explicitly rather than count them here. Fix the default language mode as well as the `-std=` overrides in the tests; the default dialect must be confirmed according to reference release probes, and all tests must not be forcefully changed to C11.

The non-LTO option lists are the primary target; LTO is a separate profile. Do not mix in LTO and then delete it at will. If `-g` is only accepted but does not emit debug info, the limitation must be disclosed; if a case requires debug/dump/scan verification, it still fails. An optimization hint may legitimately not perform a given optimization, but the associated observable option semantics must be implemented, and the matrix must not be covertly disguised as a uniform `-O0`. fast-math is off by default; handle it according to the option when a case requires it.

## 3. Denominator and Judging

One instance = `(suite, testcase, target, dialect, canonical options, auxiliary sources)`, not every PASS entry in the DejaGnu log. For execute, compilation and running are merged into a single instance: failure of either step to meet expectations is a failure.

Definitions:

```text
E = the set of target-applicable instances frozen before execution
P = the instances in E that satisfy the complete original test assertions
instance_rate = |P| / |E|

F = frozen test files that have at least one applicable instance
PF = the files in F for which all applicable instances pass
file_rate = |PF| / |F|
```

In the end, the instance_rate and file_rate of compile, execute, and ieee must **each be > 0.99**; the pooled overall instance_rate and overall file_rate across the three suites must also each be > 0.99. An empty denominator does not count as a pass. To prevent optimization levels from masking problems, the pass rate and failure list must also be reported separately for each matrix configuration; those per-configuration rates are reporting requirements, not additional per-configuration gates.

- FAIL, TIMEOUT, ICE, LINK_FAIL, RUN_CRASH, WRONG_CODE, and UNSUPPORTED/UNRESOLVED caused by missing candidate functionality: remain in E and do not enter P.
- Infrastructure ERROR: retain E, block the publication of the final pass-rate conclusion, fix and rerun; instances must not be deleted.
- XFAIL is not treated as P and is conservatively counted as not passing; XPASS is counted as P only when the actual complete assertion holds, and the original label is retained. This is not the same denominator definition as the official DejaGnu summary; report both.
- Expected negative results (such as `dg-shouldfail`) are judged according to the original assertion; do not count every non-zero exit as failure/pass. An Expected ICE is not treated as a correctness success and is marked separately.
- External assembler/linker, system libc/libm/compiler-rt, or libgcc may serve as explicit runtime/toolchain dependencies; the candidate must not hand C source/AST/IR to GCC/Clang to generate programs. GCC/Clang may only serve as a reference run and oracle.
- Bring-up results that use an external preprocessor are named separately and do not count toward the final end-to-end pass rate.

## 4. Work Items (Parallelizable Peripheral Tasks)

| ID | Function and Delivery | Acceptance |
|---|---|---|
| H00 | Download/locate the fixed GCC corpus, generate the source lock and license records; do not copy GCC implementation into the MIT core | Offline rerun verifies hashes; distributed test assets retain their original licenses; legal boundary is recorded |
| H01 | Fix the target board, sysroot, reference gcc, assembler/linker, and runner; run the target capability probe harness | Same ABI cross-compilation/native run; record the full versions and the hashed probe report |
| H02 | Run the official DejaGnu driver, producing candidate and reference `.sum/.log` | compile is assemble, execute and ieee actually run; do not manually grep the source to replace the rules |
| H03 | Normalize the actual plan into an instance manifest; lock options, headers, extra sources, and target predicates | Two enumerations are exactly identical; do not double-count compile/run sub-results |
| H04 | Candidate command-line driver: `-E/-S/-c/-o/-I/-D/-U/-std`, the M1 Part A evidence commands (snapshot/trace emission and IR interpretation), and necessary options | Flag propagation and quoting/multiple sources; no candidate → fallback to another C compiler |
| H05 | Isolated execution, a separate directory per instance, timeouts/output/resource limits; cancellation and crash classification | A malicious hang does not block the whole suite; identical basenames do not collide |
| H06 | Aggregate JSON/HTML: dual denominators, suite/config stratification, failure reasons, exclusions, unknown rules | Fake PASS/skip/dropped case is rejected by self-test; record all commands and evidence for each instance |
| H07 | Reference compiler runs normally, reference-oracle fixtures, and classification of the H01 target-capability evidence | Compare on the same target; when the reference also fails, investigate explicitly rather than letting the candidate pass |
| H08 | Failure minimization, bisect, and token/AST/IR/asm snapshot localization | Preserve the original failure predicate and freeze the original case; a reduced case does not replace the original acceptance |
| H09 | CI full/smoke/nightly regression and release gate | Meets the §3 gate (each suite's instance_rate and file_rate, and both pooled overall rates, > 0.99); per-configuration rates reported, not separately gated; stable and identical profile for 3 consecutive runs; new failures block merge |
| H10 | Supplementary standard/ABI tests, randomly generated valid C, and undefined-behavior screening | torture compile cannot prove semantic correctness; execute and self-owned tests cover the corresponding rules |

### 4.1 Current Host-task state and ownership boundaries (2026-10-05)

This table is a work plan; naming a task does not mean it exists. Current state:

- **H00 is scaffold only.** The offline lock verifier under `tools/torture/**`
  exists and is tested, but no corpus has been fetched, no asset tree exists, and
  no frozen lock exists; the H00 acceptance above is **not met**. See
  [T00_H00_IMPLEMENTATION_STATUS.md](T00_H00_IMPLEMENTATION_STATUS.md).
- **H01's probe harness has never run.** The harness under
  `tools/torture/probe/**` (with `.github/workflows/t00-target-probes.yml`) is
  present, but no probe report or report hash exists; H01 is **not complete**,
  and the T01 C02 target model remains **UNVERIFIED**. A harness in the tree is
  not evidence that the probe ran.
- **H01/H07 boundary.** H01 owns the substrate/toolchain pins (board, sysroot,
  reference GCC, assembler/linker, runner), the probe harness, the probe run,
  and the hashed report. H07 owns the reference-oracle side: reference-compiler
  baseline runs, oracle fixtures, and the documented classification of
  target-capability evidence — including the four ABI classification fields the
  report deliberately leaves `unresolved` (`abi.gp_arg_regs`,
  `abi.fp_arg_regs`, `abi.stack_align`,
  `abi.variadic_register_save_area`). H07 consumes the H01 report; it does not
  re-run the H01 harness.
- **Probe → attestation is integration-owned.** A report is evidence, not
  verification: the T01 integrator owns the `TargetSpec::attest` step and is the
  only owner who may mark the target verified after incorporating a real report
  hash. `attest` validates caller-supplied data and its hash, not authenticity
  or physical provenance (TOCTOU), and the ABI classification is unresolved, so
  no probe has been attested and C02 remains unverified.
- **H04 is absent, and it gates Part A evidence as well as Part B.** There is no
  candidate driver or binary (`compiler/` builds a library only). H04's surface
  is not limited to the torture flags above: the M1 Part A evidence commands
  (snapshot/trace emission and IR interpretation;
  [M1_TARGET_ACCEPTANCE.md](M1_TARGET_ACCEPTANCE.md) §9) also require an
  H04-owned CLI, so the absent driver blocks Part A **evidence collection** —
  the Part A contract and fixtures remain probe- and driver-independent for
  drafting. H02–H03 and H05–H10 are planned only; H02–H03 remain required for
  any torture run and H05–H10 for the final gate.

H02–H03 do not require fully rewriting Tcl/DejaGnu from scratch. Prefer retaining the official driver and integrating it through a compiler-under-test wrapper and board; if a runner is built from scratch, verify item by item that it matches the official instances/judging. A directive that cannot be recognized must be reported as a harness gap and must block the final claim; it must not be ignored.

All `.c`, `.S`, extra sources, and special drivers enumerated by the original driver enter the list; do not pick only the extensions that are easy to handle yourself. The `.S` path may be completed by the candidate's own preprocessing + an external assembler, and must be recorded distinctly from the ordinary C source path; this is not delegating C to the reference compiler. The candidate compilation/linking rules for multi-file tests are likewise frozen.

## 5. Failure-Driven Chip Task Generation

Register each failure as `(instance_id, stage, chip_ids, reproduction, suspected rule, owner)`. First localize: preprocessing → tokens → AST → types → IR → ABI → assembly → runtime → test infrastructure. A new task must describe the rule, the exact inputs and outputs, and a reproduction test; writing special branches that match file names/test source is not permitted.

The initial catalog cannot possibly exhaust all GCC extensions of the fixed corpus in advance. T00's feature census must supplement missing chips; this is not permission to shrink the denominator. A complete optimizer cannot replace frontend/ABI correctness either.

## 6. Deliverables and References

Recommended deliverables: `tools/torture/{lock,profiles,manifest,runner,reports}`, CI configuration, self-test fixtures, and a baseline report. During the document design phase, do not fill in fake test counts/pass rates; record them after the actual M0 enumeration.

References (verified against official materials and the GCC 15.2 driver at the time of writing):

- https://gcc.gnu.org/onlinedocs/gccint/C-Tests.html
- https://gcc.gnu.org/onlinedocs/gccint/Directives.html
- https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/lib/c-torture.exp
- https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/lib/gcc-dg.exp
- https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/gcc.c-torture/compile/compile.exp
- https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/gcc.c-torture/execute/execute.exp
- https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/gcc.c-torture/execute/ieee/ieee.exp

After the release is locked, the driver/predicates of that commit take precedence; online rolling documentation does not replace the frozen corpus.
