# A21 — T00 Corpus, H02 DejaGnu, and H03 Manifest Plan

| Field | Value |
|---|---|
| Status | **Design only — no implementation, no corpus fetch, no GCC/DejaGnu run, no pass-rate claim** |
| Task | A-HST-02 / A21 under [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) |
| Date | 2026-11-07 |
| Scope | H00 acquisition/lock/legal/provenance, H02 official DejaGnu integration and candidate wrapper, H03 instance/option/predicate manifest, suite separation, denominators, per-file matrix, failure accounting, reference/candidate separation, reproducible evidence |
| Authority | [T00](../../tasks/T00_GCC_TORTURE_GATE.md) defines the acceptance contract; [H00 status](../../tasks/T00_H00_IMPLEMENTATION_STATUS.md), [H04 driver](../../tasks/H04_CANDIDATE_DRIVER.md), [H04 `-I` draft](../../tasks/H04_I_FREEZE.md), [M1 target acceptance](../../tasks/M1_TARGET_ACCEPTANCE.md), and the current `tools/torture` source are the state of record |
| Non-goals | No source, test, contract, Cargo, `mod.rs`, CI, or shared-index edit in this task; no commit/push; no agent launch; no corpus download; no compiler invocation |

This document is a plan, not a freeze. Every schema name, command, path, and
status below is **proposed** unless it is explicitly marked as existing source
behavior. The current repository has no frozen corpus, no corpus lock, no
DejaGnu integration, no candidate torture run, no verified target ABI, and no
GCC pass rate. Nothing here may be used to claim any of those.

Line references are to the files as read on 2026-11-07; they may drift.

---

## 1. Executive Summary

The current T00 state is an **inspection-confirmed scaffold with no measured
capability**:

- H00 has an offline lock verifier crate with synthetic tests (`tools/torture/`), but no
  corpus, no fetched asset tree, no frozen lock, no license review record, and no
  lock-build/fetch path.
- H01 has a probe harness that has **never run**; there is no probe report or
  report hash, so T01 C02 remains unverified.
- H02 (official DejaGnu runner) and H03 (instance manifest) do not exist.
- H04 exists for Part A evidence plus `-E`/`-D`/`-U`; `-I`, multi-source,
  `-S`/`-c` target emission, and the torture flag matrix are still absent or
  explicitly refused.
- H05–H10 are absent.

The selected H02 approach is to **retain the official GCC/DejaGnu driver** and
integrate it through a compiler-under-test wrapper plus a board/site
configuration. Rebuilding the driver from scratch is not part of this plan; if
the official path proves impossible, any replacement must be validated item by
item against the frozen driver before it can be used for an acceptance claim.

The critical path is:

```text
H00 frozen corpus lock + legal sign-off
        +
H01 pinned Linux AArch64 substrate / toolchain / sysroot / runner / probe
        ↓
H02 reference-only DejaGnu baseline (oracle; never candidate evidence)
        ↓
H03 frozen instance manifest (E) and per-file matrix
        ↓
H04 full driver + T11 target emission + C02 attestation + H05 isolation
        ↓
H02 candidate DejaGnu baseline
        ↓
H06 aggregation / H08 localization / H09 gate
```

H03 freezes the denominator **E** and the per-file matrix. H02 produces raw
official logs plus per-instance evidence. H06 joins them and computes **P** and
the rates. H03 must never compute a candidate pass rate, and H02 must never
replace the official driver with manual greps.

The most important current-gap findings are:

1. The existing lock schema cannot represent per-suite option matrices, DejaGnu
   /Tcl/Expect pins, LTO exclusion policy, or an asset shared by multiple
   suites. A lock schema revision is required before H00 can freeze.
2. The H04 `-I`/multi-source freeze is **draft only**, so H02 must not assume
   include or multi-source behavior.
3. Candidate torture compilation is **Part B gated**: no candidate `-c`/`-S`
   path, no T11 target codegen, no C02 attestation, and no H05 isolation today.
4. The two-enumeration requirement is best satisfied by comparing the frozen
   manifest against reference and candidate driver plans, not by trusting
   `.sum` PASS counts.

---

## 2. Current Status and Scaffold Boundary

### 2.1 Status table

| Area | Current state | Evidence | Consequence for this plan |
|---|---|---|---|
| H00 lock verifier | **Implemented scaffold; synthetic tests only** | `tools/torture/src/{lock,verify,digest,path,diagnostic}.rs`; `tools/torture/tests/{verify,cli}.rs`; [README](../../../tools/torture/README.md) lines 3–8, 49–59 | Can verify a supplied lock; cannot fetch or build one |
| H00 corpus lock | **Absent** | [H00 status](../../tasks/T00_H00_IMPLEMENTATION_STATUS.md) lines 109–132; no `tools/torture/lock/` directory | H00 is not complete; no lock may be treated as frozen |
| H00 legal/provenance | **Policy text only; no real review** | [H00 status](../../tasks/T00_H00_IMPLEMENTATION_STATUS.md) lines 109–126; verifier checks only presence, not legal truth | Human legal review and per-asset license records are required |
| H01 probe | **Harness present; never run** | [probe README](../../../tools/torture/probe/README.md) lines 3–8; [T00 §4.1](../../tasks/T00_GCC_TORTURE_GATE.md) lines 83–106 | H02 reference/candidate runs cannot claim verified target values until H01 runs and H07/T01 process the evidence |
| H02 runner | **Absent** | No `tools/torture/runner/`; [T00 §4.1](../../tasks/T00_GCC_TORTURE_GATE.md) lines 107–121 | Entire official-driver integration is future work |
| H03 manifest | **Absent** | No `tools/torture/manifest/` or manifest code | Instance/options/predicates/denominators are unrepresented |
| H04 driver | **Part A present; rest deferred** | `compiler/src/bin/candidate.rs` lines 14–15, 85, 104, 140–141; [H04 driver](../../tasks/H04_CANDIDATE_DRIVER.md) lines 5–8, 60–66 | Candidate torture path is blocked on `-I`, multi-source, `-S`/`-c`, flag propagation |
| H04 test count drift | **18 tests in source; T00 says 6** | `compiler/tests/h04_candidate.rs` has 18 `#[test]`; [T00 §4.1](../../tasks/T00_GCC_TORTURE_GATE.md) line 108 says 6 | Documentation drift only; do not use either count as a capability metric |
| H05 isolation | **Absent** | [T00 §4](../../tasks/T00_GCC_TORTURE_GATE.md) line 72; [PARALLEL_EXECUTION](../../tasks/PARALLEL_EXECUTION.md) line 9 | No safe execute/ieee run of untrusted binaries yet |
| H06 aggregation | **Absent** | [T00 §4](../../tasks/T00_GCC_TORTURE_GATE.md) line 73 | No P/rate computation; H03 must not absorb this role |
| H07 reference oracle | **Absent as a task artifact** | [T00 §4](../../tasks/T00_GCC_TORTURE_GATE.md) line 74; [T00 §4.1](../../tasks/T00_GCC_TORTURE_GATE.md) lines 92–100 | Default dialect/options and ABI evidence classification are pending |
| H08/H09/H10 | **Absent** | [T00 §4](../../tasks/T00_GCC_TORTURE_GATE.md) lines 75–77 | Failure minimization, CI gate, and supplementary tests are future work |
| Target | **Identity frozen; values unverified** | [M1 target acceptance](../../tasks/M1_TARGET_ACCEPTANCE.md) lines 45–58 | No macOS/Darwin ABI substitution |
| Compiler contract | **`t01-c01-c06/39`** | `compiler/contracts/CONTRACT_VERSION` | H02/H03 are host tasks; no bus/schema change is proposed here |
| CI | **Torture crate fmt/clippy/test runs** | `.github/workflows/ci.yml` lines 33–58 | No H02/H03 runner or corpus job exists; root README lines 658–665 are stale on this point |
| Probe CI | **Manual real probe only** | `.github/workflows/t00-target-probes.yml` lines 27–53, 69–121 | No automatic target execution; action pins are mutable tags |
| Pass rates | **None measured or implied** | [T00 §3](../../tasks/T00_GCC_TORTURE_GATE.md) lines 38–61 | No number may be published from this plan |

### 2.2 Scaffold versus verified capability

| Category | What exists | What is not established |
|---|---|---|
| Lock parsing and offline hashing | Rust code and synthetic tests; deterministic diagnostics and exit codes | A real GCC corpus, real hashes, real per-file licenses, legal sign-off, freeze |
| Probe harness | Shell/Python normalizer and guard self-tests; manual workflow | Any real probe run, report, report hash, ABI classification, attestation |
| Candidate driver | M1 Part A evidence, `-E`, `-D`, `-U`, refusal of `-S`/`-c` | `-I`, multi-source, target emission, torture flag propagation, candidate compilation |
| CI | Root/compiler/chip-lint/torture checks | DejaGnu/reference/candidate run, denominator audit, 3-run stability, >99% gate |
| Documentation | Task catalogs and plans | Implemented H02/H03, official logs, manifests, measured rates |

`tools/torture` tests are **synthetic** and generated at runtime; they contain no
GCC corpus content ([tools/torture README](../../../tools/torture/README.md) lines
236–245). The verifier proves only that a supplied tree matches a supplied lock;
it does not prove H00 completion or compiler correctness
([tools/torture README](../../../tools/torture/README.md) lines 49–59).

---

## 3. Gap Register

| ID | Gap | Evidence | Impact | Owner | Gate |
|---|---|---|---|---|---|
| G-01 | No corpus acquisition path | No fetch script or corpus; H00 status lines 109–132 | H00 cannot start its real work | H00 | H00-G1 |
| G-02 | No frozen lock | No `tools/torture/lock/`; verifier returns exit 3 for draft | No corpus identity, no H02/H03 input | H00 | H00-G3 |
| G-03 | Lock schema lacks per-suite option matrices | `lock.rs` lines 70–76, 141–147: one `ProfileFields` and one `configuration` list; T00 §2 lines 20–36 require per-driver lists | Cannot express `DG_TORTURE_OPTIONS` vs `C_TORTURE_OPTIONS`; freeze would be false | H00 | H00-G2 |
| G-04 | Lock schema lacks DejaGnu/Tcl/Expect/runtest pins | `lock.rs` lines 158–166 has only runner/toolchain/sysroot/assembler/linker; T00 §1 line 7 requires DejaGnu/toolchain versions | A frozen lock cannot pin the harness that produces evidence | H00 | H00-G2 |
| G-05 | Lock schema cannot share one asset across suites | `apply_asset` accepts one `suite` and rejects duplicate fields (`lock.rs` lines 610–648) | Driver/header assets that serve compile/execute/ieee cannot be represented correctly | H00 | H00-G2 |
| G-06 | No LTO exclusion field | No LTO field in `lock.rs`; T00 §2 line 34 requires explicit exclusion | Rows could silently appear or disappear | H00 / H03 | H00-G2, H03-G3 |
| G-07 | Legal truth and signature verification out of scope | H00 status lines 114–126; README lines 148–151 | Tool can verify hashes but not lawful redistribution or authentic release | H00 + human legal | H00-G3 |
| G-08 | No safe fetch/extract/materialize path | No fetch code | Corpus could be fetched unsafely or vendored into the MIT tree | H00 | H00-G1 |
| G-09 | No H02 runner/board/wrapper | No `tools/torture/runner/` | No official logs, no reference baseline, no candidate run | H02 | H02-G2/G3 |
| G-10 | H04 incomplete for torture | `candidate.rs` lines 14–15, 85, 104, 140–141; H04 doc lines 60–66 | Wrapper cannot compile multi-file tests, includes, or target objects | H04 | H02-G4 |
| G-11 | H04 `-I`/multi-source freeze is draft | H04 `-I` freeze header lines 5–11; §7 lines 94–98 | Include/multi-source behavior cannot be assumed or implemented | H04 + H01/T01 | H04 freeze gate |
| G-12 | No target codegen / attestation | T11 absent; C02 unverified; M1 target Part B unavailable | Candidate compile/execute path is blocked by design | T11 / T01 integrator | H02-G4 |
| G-13 | No H05 isolation | H05 absent | Execute/ieee cannot be run safely; timeout/crash classification absent | H05 | H02-G4 |
| G-14 | No H06 aggregation | H06 absent | No P, no dual denominators, no dual summary | H06 | Integration gate |
| G-15 | No H07 reference classification | H07 absent | Default dialect/options and ABI evidence classification unresolved | H07 | H02-G3 |
| G-16 | H02/H03 not in CI | `ci.yml` only runs crate checks | No regression protection for runner/manifest | H02/H03 + integrator | H02-G2, H03-G1 |
| G-17 | No plan-capture interface | No raw plan schema or Tcl hooks | H03 cannot prove the actual plan equals the frozen manifest | H02 | H02-G5 |
| G-18 | No instance/options/predicate schema | No manifest code | Denominator cannot be audited or frozen | H03 | H03-G2 |
| G-19 | No reference/candidate output separation contract | No runner output roots | Reference results could contaminate candidate P | H02/H06 | H02-G3/G4 |
| G-20 | Documentation drift on H04 and CI | T00 line 108 says 6 H04 tests vs 18 in source; root README lines 658–665 says torture not wired, but `ci.yml` runs it | Reviewers may use stale status | integrator | Documentation gate |

---

## 4. H00 — Corpus, Lock, Legal, and Provenance

### 4.1 Acceptance restated

H00 is complete only when:

1. The exact GCC release and full commit SHA are frozen.
2. The source archive and every selected test/driver/aux/header/runtime asset are
   hash-locked and have explicit license and provenance records.
3. The official release hash and, if used, the detached signature are verified
   by an authorized step.
4. The lock records the target identity, substrate, toolchain, DejaGnu/Tcl/
   Expect/runtest versions, per-suite option matrices, and LTO policy.
5. An offline rerun verifies every listed asset and reports no unlisted file in
   the selected corpus root.
6. Human legal review records the distribution boundary: GCC assets remain
   external GPL material and are not vendored into the MIT tree.
7. The lock is either genuinely frozen or explicitly left draft; a draft must
   return exit 3 and must never be used for a final claim.

Source: [T00 §1](../../tasks/T00_GCC_TORTURE_GATE.md) lines 7–18,
[T00 §4](../../tasks/T00_GCC_TORTURE_GATE.md) lines 63–70,
[H00 status](../../tasks/T00_H00_IMPLEMENTATION_STATUS.md) lines 15–107.

### 4.2 Recommended corpus and root layout

Planning choice: use the signed GCC `releases/gcc-15.2.0` release tarball as the
byte-level normative artifact and record the git tag/commit as provenance. The
release tarball and the git tag may differ in generated files; H00 must record
which one defines the file bytes. The final choice is an open question (OQ-01).

Proposed external layout (never committed):

```text
$T00_CORPUS_ROOT/
  gcc-15.2.0.tar.xz                 # archive asset, hash-locked
  gcc-15.2.0.tar.xz.sig             # optional signature asset, not verified by the tool
  gcc/testsuite/...                 # pruned, exactly the frozen asset set
  COPYING / license files           # preserved as corpus assets
```

A **pruned corpus root** is recommended because `--check-unlisted` is meaningful
only when the root contains exactly the frozen assets. A whole extracted GCC
tree would produce millions of unlisted files and make the completeness check
useless. The archive and the pruned tree are both under the external root; the
repository stores only the lock metadata, never the corpus content.

Proposed repository metadata layout:

```text
tools/torture/lock/
  gcc-15.2.0.torture.lock           # proposed lock path; not present today
  licenses.tsv                      # path-pattern -> license/source/reviewer
  LEGAL_REVIEW.md                   # human sign-off record; not a legal opinion
  fetch-corpus.sh                   # authorized network fetch; never run by CI by default
  materialize-corpus.sh             # verify archive, extract, prune to frozen assets
  README.md
```

`tools/torture/lock/` does not exist today. The existing verifier lives at
`tools/torture/src/**`; H00 build/fetch orchestration is new work.

### 4.3 Lock schema changes required

The current `cc-silicon.torture/lock/v1` schema is insufficient for H02/H03. The
following is a **proposed additive `lock/v2`**, not a frozen design.

| Required capability | Current v1 | Proposed v2 |
|---|---|---|
| Per-suite option matrix | One `profile` record with one `configuration` list | One `profile` record per suite, or repeated `suite` plus per-suite configurations; require exactly one profile for each of `compile`, `execute`, `ieee` |
| DejaGnu/Tcl/Expect/runtest pins | Absent | `tooling dejagnu-version=... expect-version=... tcl-version=... runtest-version=... runtest-sha256=... dejagnu-tree-sha256=...` |
| LTO exclusion | Absent | `lto policy="excluded" reason="non-LTO baseline"` plus explicit excluded-option list |
| Shared asset across suites | One `asset suite=` value; duplicates rejected | Repeatable `asset suite=` values, or a canonical `suites=` list |
| Reference compiler pins | `substrate.toolchain` only | `reference gcc-version=... gcc-sha256=... target-triple=... default-dialect=...` |
| Probe/attestation linkage | Absent | `target probe-report-sha256=... attestation-id=...` (H01/H07/T01 inputs, not invented here) |
| Legal review linkage | Presence-only license/provenance | Optional `license-source=` and a human `LEGAL_REVIEW.md` reference; the tool still cannot verify legal truth |

Relevant current behavior:

- `lock.rs` defines `SCHEMA_V1`, `REQUIRED_SUITES`, `KNOWN_ROLES`, a single
  `ProfileFields`, and strict unknown-field rejection (`lock.rs` lines 31–88).
- `validate_profile` requires `compile`, `execute`, and `ieee` in one profile
  (`lock.rs` lines 919–963).
- `validate_substrate` requires non-empty runner/toolchain/sysroot/assembler/
  linker (`lock.rs` lines 1009–1036).
- `validate_reference` requires the non-candidate reference policy (`lock.rs`
  lines 1072–1105).
- The verifier rejects unknown fields and duplicate fields; therefore schema
  changes require code and test changes, not merely new text in a lock.

H00 must decide whether to bump to `lock/v2` or amend `v1` in place. A version
bump is recommended because per-suite profiles and tooling pins change the
meaning of a frozen lock. Existing draft `v1` locks may remain readable; a
frozen H02/H03 lock must be `v2`.

### 4.4 Acquisition, lock build, and verification sequence

Proposed commands (future; **not executed here**):

```sh
# 1. Authorized fetch into a staging area. Network access is explicit.
tools/torture/lock/fetch-corpus.sh \
  --release gcc-15.2.0 \
  --url https://gcc.gnu.org/pub/gcc/releases/gcc-15.2.0/ \
  --staging "$T00_STAGING_ROOT" \
  --signature-key "$T00_GCC_RELEASE_KEY"

# 2. Verify the published SHA-512 and (separately) the detached signature.
#    Record the key fingerprint, command, and output outside the repository.
(
  cd "$T00_STAGING_ROOT"
  grep ' gcc-15.2.0.tar.xz$' sha512.sum | sha512sum -c -
  gpg --verify gcc-15.2.0.tar.xz.sig gcc-15.2.0.tar.xz
)

# 3. Extract and prune to the frozen asset set. Never execute corpus content.
tools/torture/lock/materialize-corpus.sh \
  --staging "$T00_STAGING_ROOT" \
  --dest "$T00_CORPUS_ROOT" \
  --asset-list tools/torture/lock/assets.list

# 4. Build a draft lock from the verified pruned tree. This reads local bytes only.
cargo run --locked --manifest-path tools/torture/Cargo.toml --bin cc-silicon-torture -- \
  lock build \
  --corpus "$T00_CORPUS_ROOT" \
  --license-map tools/torture/lock/licenses.tsv \
  --profile tools/torture/profiles/linux-aarch64-lp64.profile \
  --out tools/torture/lock/gcc-15.2.0.torture.lock

# 5. Verify the draft. Exit 3 is expected until legal/substrate/tooling sign-off.
cargo run --locked --manifest-path tools/torture/Cargo.toml --bin cc-silicon-torture -- \
  tools/torture/lock/gcc-15.2.0.torture.lock \
  "$T00_CORPUS_ROOT" \
  --check-unlisted

# 6. After real sign-off, set status=frozen and verify again. Exit 0 is required.
```

The current verifier already provides the offline part of step 5; `lock build`,
`fetch-corpus.sh`, `materialize-corpus.sh`, and the `v2` schema do not exist.

### 4.5 Legal and provenance rules

1. **Per-asset license, not a blanket license.** T00 and the H00 status both
   warn that testsuite licensing varies; every asset must record its own license
   and provenance. `licenses.tsv` must state whether the license came from a
   file header, a directory `COPYING`, or a reviewed corpus-level policy.
2. **No vendoring.** The lock, licenses map, and review record may live in the
   repository; GCC source/test bytes must stay in the external corpus root.
3. **No legal truth claim by the tool.** The verifier checks presence and
   non-emptiness only; it cannot confirm that a license is correct or that
   redistribution is lawful. Human review is a separate gate
   ([H00 status](../../tasks/T00_H00_IMPLEMENTATION_STATUS.md) lines 114–126).
4. **Signature verification is a separate authorized step.** The tool does not
   verify OpenPGP signatures; the fetch runbook must record the key fingerprint,
   verification command, and result.
5. **Provenance must be revision-linked.** Every asset should carry the same
   immutable corpus revision (commit SHA or verified release identity). The
   current `provenance.revision-unresolved` warning does not change the exit
   code, so a frozen lock can verify with weak provenance; H00 must close that
   gate manually or propose a stricter check.
6. **Evidence logs may contain GPL source excerpts.** GCC diagnostics can quote
   test source. Keep raw `.log`, source excerpts, and per-instance evidence under
   `$T00_RUN_ROOT` outside the repository; commit only metadata/hashes unless a
   legal review approves otherwise.

### 4.6 H00 work orders

| WO | Deliverable | Allowed paths (proposed) | Depends | Gate |
|---|---|---|---|---|
| H00.1 | Authorized fetch runbook and safe fetch script; SHA-512/signature verification; no vendoring | `tools/torture/lock/fetch-corpus.sh`, `tools/torture/lock/README.md`, `tools/torture/tests/fetch/**` | H00 authorization | H00-G1 |
| H00.2 | Lock `v2` schema: per-suite profiles, tooling pins, LTO, repeatable asset suites, reference pins | `tools/torture/src/lock.rs`, `tools/torture/src/lib.rs`, `tools/torture/tests/{verify,cli,common}` | H00.1 design; schema review | H00-G2 |
| H00.3 | `lock build` command: hashes, sizes, roles, suites, license map; no network | `tools/torture/src/main.rs`, `tools/torture/src/lock_build.rs`, `tools/torture/tests/lock_build.rs` | H00.2 | H00-G2 |
| H00.4 | Safe extraction/prune/materialize script and asset list | `tools/torture/lock/materialize-corpus.sh`, `tools/torture/lock/assets.list`, tests | H00.1 | H00-G1 |
| H00.5 | Per-file license/provenance map and human legal review record | `tools/torture/lock/licenses.tsv`, `tools/torture/lock/LEGAL_REVIEW.md` | H00.4; human legal reviewer | H00-G3 |
| H00.6 | Frozen lock and offline verification evidence | `tools/torture/lock/gcc-15.2.0.torture.lock` (metadata), external corpus root | H00.2–H00.5 + H01 pins + H02.1 profile | H00-G3 |
| H00.7 | Offline CI job for schema/build/verify tests (no corpus) | `.github/workflows/ci.yml` (integrator-owned), new tests | H00.2–H00.4 | H00-G4 |

File ownership note: `tools/torture/src/lib.rs`, `src/main.rs`, and
`Cargo.toml` are shared entry points. Only one owner may edit them at a time;
H00.2/H00.3 must be sequential on those files. H02/H03 must not edit them until
the H00 owner lands the module/bin entries.

### 4.7 H00 acceptance gates

| Gate | Criterion |
|---|---|
| H00-G1 | Synthetic archive/tree tests prove fetch→verify→prune is deterministic and rejects bad hashes, unsafe paths, and unlisted files; no network in tests |
| H00-G2 | `lock/v2` parser/validator tests cover per-suite profiles, tooling pins, LTO exclusion, shared asset suites, duplicate/unknown fields, and frozen-vs-draft severity |
| H00-G3 | Real frozen lock verifies with exit 0 and `--check-unlisted` on a quiescent pruned root; every asset has license/provenance; legal review signed; no corpus bytes in the MIT tree |
| H00-G4 | CI runs H00 schema/build/verify tests with no corpus and no network |

An exit code 3 is not a pass. A frozen lock with unresolved provenance warnings
requires explicit review, not silent acceptance.

---

## 5. H02 — Official DejaGnu Integration and Candidate Wrapper

### 5.1 Approach decision

**Retain the official GCC testsuite/DejaGnu driver.** The runner must not
reimplement Tcl judging or manually grep test sources. Integration is through:

1. A pinned `runtest`/DejaGnu installation.
2. A board file that fixes the target, compiler-under-test, assembler, linker,
   runner, timeout, and output directory.
3. A generated `site.exp` that uses the frozen corpus paths and tool pins.
4. A compiler-under-test wrapper used in **reference** or **candidate** mode.
5. A plan-capture hook that records the actual driver plan before execution.
6. H05 for per-instance isolation, timeouts, limits, and crash classification.

A from-scratch runner is out of scope. If the official driver cannot be
integrated, the replacement must be validated item by item against the frozen
driver's instances, options, directives, and judging; that is a separate
decision and would still not permit manual greps as a substitute.

Relevant requirement: [T00 §4.1](../../tasks/T00_GCC_TORTURE_GATE.md) lines
107–121.

### 5.2 Driver integration seams to freeze first

Before writing Tcl or a wrapper, H02 must read the frozen release's
`gcc/testsuite/lib/*.exp`, suite `.exp` files, and board/site conventions and
record the exact seams in `tools/torture/runner/DRIVER_SEAMS.md`. The following
are the expected seams; exact names and load order must be confirmed from the
frozen tree, not assumed:

| Seam | Purpose | What to record |
|---|---|---|
| `runtest --tool gcc` | Official driver entry | Exact `runtest` path, version, SHA-256, Tcl/Expect/DejaGnu versions |
| `--srcdir`, `--objdir`, `--target_board` | Source/output/board selection | Exact arguments and directory layout |
| `site.exp` | Per-run configuration | Generator command and hash; no host defaults |
| `GCC_UNDER_TEST` (expected) | Compiler-under-test path | Exact global name and how it is consumed |
| `gcc_target_compile` / `target_compile` (expected) | Compile/link invocation | Exact proc, argument shape, option ordering |
| `${tool}_load` / `${tool}_run` (expected) | Executable load/run | Exact proc and runner handoff |
| `dg-runtest` / `gcc-dg-runtest` | Per-test loop and option application | Hook point for plan capture and testcase identity |
| Target-support predicates | `dg-require-effective-target`, `dg-skip-if`, `.x` files | Exact predicate oracle and frozen result source |

The H02 seam freeze is a read-only audit. It must not modify the frozen corpus
or upstream `.exp` files.

### 5.3 Compiler-under-test wrapper

Propose one wrapper binary used in both modes, built from
`tools/torture/src/bin/cc-silicon-torture-cc.rs` (or an equivalent separate
crate if the integrator prefers). A single wrapper gives uniform argv/evidence
logging and makes accidental mode mixing visible.

| Mode | Behavior | Must never do |
|---|---|---|
| `reference` | Record argv, then invoke the pinned reference GCC with the same argv; preserve exit status/signal | Invoke the candidate; produce candidate evidence |
| `candidate` | Record argv, classify flags, drive the candidate compiler, external assembler, and external linker per the frozen rule | Invoke GCC/Clang to compile candidate C; fall back on failure; silently drop a flag |

Candidate mode requirements:

1. **Flag matrix from observed reference plan.** Run the reference baseline
   first, capture every compiler flag used by the official plan into
   `tools/torture/runner/flags.tsv`, and classify each as forward, accepted with
   recorded limitation, or unsupported. Do not guess the flag universe.
2. **Unknown/unimplemented flag** → structured candidate diagnostic and nonzero
   exit. Never ignore it, never skip the test, never substitute another compiler.
3. **Compile suite** (`compile`): every applicable instance must produce an
   object file (assemble), not only `-S`. The wrapper must verify object
   existence and record its hash.
4. **Execute/ieee**: compile and link must produce an executable; the run must
   actually happen under H05 isolation. A compile-only success is not a run
   success.
5. **`.S` sources**: candidate preprocesses (if required) and the pinned external
   assembler assembles them. Record this path separately from the C path.
6. **Multi-source/auxiliary sources**: compile each translation unit
   independently; link per the frozen H03 rule. H04 multi-source freeze must be
   accepted first (currently draft only).
7. **Includes**: `-I` and include search behavior must follow the accepted H04
   freeze. Today H04 refuses `-I`; do not invent a private search path.
8. **Link**: use the pinned external linker and sysroot/startup/lib rule. If a
   GCC link driver is used, it must be proven not to compile C; direct `ld` with
   pinned inputs is preferred.
9. **Quoting**: argv arrays, no shell re-quoting; preserve repeated flags,
   empty values, spaces, and `--` semantics; record the exact argv.
10. **Failure classes**: distinguish compiler crash (signal), ICE, timeout,
    unsupported flag/construct, missing artifact, link failure, and ordinary
    compile failure. H02 records raw facts; H06 maps them to T00 categories.
11. **Debug/dump/scan and fast-math are not free passes.** If `-g`, dump, scan,
    or compare-debug flags are accepted without the associated observable
    artifact, disclose the limitation; a test requiring debug/dump/scan still
    fails. fast-math stays off unless the original test requires it.
12. **External-preprocessor runs are labeled separately.** A bring-up that uses
    an external preprocessor must be recorded as `external-preprocessor` and
    excluded from the final end-to-end candidate rate; it cannot be presented as
    a candidate preprocessing pass.

Candidate `-S`/`-c` and target emission are **Part B gated**. Until H04 exposes
the flags, T11 emits AArch64 assembly, C02 is attested, and H05 isolation
exists, candidate mode must remain fail-closed.

### 5.4 Suite run recipes

Proposed runner entry point:

```text
tools/torture/runner/run-suite.sh --mode reference|candidate \
  --suite compile|execute|ieee \
  --lock <lock> --corpus <root> --profile <profile> \
  --manifest <frozen-manifest>            # required for candidate; optional for reference
  --out <output-dir> [--plan-out <plan-file>]
```

The script must generate `site.exp`, invoke the pinned `runtest`, and retain
`gcc.sum`/`gcc.log` plus wrapper/plan evidence. The expected official
invocation shape (confirm exact filenames/procs from the frozen driver) is:

```sh
runtest --tool gcc \
  --target_board=cc-silicon-aarch64-linux \
  --srcdir "$T00_CORPUS_ROOT/gcc/testsuite" \
  --objdir "$OUT" \
  gcc.c-torture/compile/compile.exp          # compile
  # gcc.c-torture/execute/execute.exp        # execute
  # gcc.c-torture/execute/ieee/ieee.exp      # ieee
```

Separate `--objdir` per mode/suite/attempt prevents identical basenames from
colliding. `GCC_UNDER_TEST` (or the confirmed equivalent) points at the wrapper.
`site.exp` must be generated from the frozen lock/profile and must not sample the
host environment for defaults.

Reference run command (future; **not executed**):

```sh
tools/torture/runner/run-suite.sh \
  --mode reference \
  --suite compile \
  --lock tools/torture/lock/gcc-15.2.0.torture.lock \
  --corpus "$T00_CORPUS_ROOT" \
  --profile tools/torture/profiles/linux-aarch64-lp64.profile \
  --out "$T00_RUN_ROOT/reference/attempt-1/compile" \
  --plan-out "$T00_RUN_ROOT/reference/attempt-1/plans/compile.plan"
```

Candidate run command (future; requires H03 manifest and Part B gates):

```sh
tools/torture/runner/run-suite.sh \
  --mode candidate \
  --suite compile \
  --lock tools/torture/lock/gcc-15.2.0.torture.lock \
  --corpus "$T00_CORPUS_ROOT" \
  --profile tools/torture/profiles/linux-aarch64-lp64.profile \
  --manifest "$T00_RUN_ROOT/manifests/linux-aarch64-lp64.manifest" \
  --out "$T00_RUN_ROOT/candidate/attempt-1/compile" \
  --plan-out "$T00_RUN_ROOT/candidate/attempt-1/plans/compile.plan"
```

The reference and candidate output roots must be disjoint. H02 must reject a run
whose mode and output root disagree.

### 5.5 Plan capture and directive coverage

H03 needs the **actual plan**, not a summary. Propose a raw plan format
`cc-silicon.torture/plan/v1` emitted by a Tcl hook loaded after the official
driver libraries. The hook wraps the confirmed compile/run seam, records the
record, then calls the original proc unchanged. It does not modify corpus files.

Minimum record kinds:

| Kind | Meaning | Enters E? |
|---|---|---|
| `instance` | One actual testcase/options/subtest invocation | Yes, after H03 normalization |
| `predicate-probe` | Target/support probe used to decide applicability | No; feeds the frozen predicate table |
| `harness` | Site/board/tool setup record | No |
| `unknown` | A directive or invocation the hook cannot classify | No; hard harness gap |

Minimum instance fields:

```text
kind=instance suite=compile testcase=<relative-path> subtest=<ordinal>
config=<ordinal> mode=assemble|run dialect=<dialect>
argv=<ordered-argv> aux=<relative-path>@<sha256> ...
predicate=<id>:<expression>=<true|false> ...
directive=<directive-id>@<sha256> driver=<exp-file>@<sha256>
```

Rules:

1. Record directives encountered and whether each is recognized. Unknown
   directive → `HARNESS_GAP`, not a skip.
2. Record options after the driver's own transformations (loop-only options
   dropped, `dg-options` applied, extra/remove options applied).
3. Record auxiliary sources and `.S` inputs with hashes and order.
4. Record target predicates and their frozen results; the candidate must not be
   allowed to shrink E by failing a predicate probe.
5. Record the wrapper's actual argv separately; H03 compares plan argv to
   wrapper argv.
6. Flush and close the plan file before the compile/run call; a plan-write
   failure aborts the run.

If the official driver cannot expose a stable plan hook without modifying
upstream tests, the fallback is a separately reviewed wrapper layer; it must
still produce the same records and pass the two-enumeration gate.

### 5.6 Reference/candidate separation

| Aspect | Reference mode | Candidate mode |
|---|---|---|
| Purpose | Oracle, default dialect/options, target-predicate evidence | Candidate pass-rate evidence only |
| Compiler | Pinned reference GCC | Candidate compiler |
| Lock label | `non-candidate` | Candidate run |
| Used for candidate P | **Never** | Yes, after H03 manifest match |
| Output root | `$T00_RUN_ROOT/reference/...` | `$T00_RUN_ROOT/candidate/...` |
| Failure handling | H07 investigates; can inform pre-frozen exclusions only | Counted in E, not P |
| Logs | Retained separately; never merged into candidate rates | Retained separately; H06 joins to manifest |

The lock's reference policy already requires
`used-for-candidate-pass-rate="false"` and `label="non-candidate"`
(`lock.rs` lines 1072–1105). H02/H06 must enforce the same rule at the file and
report level, not merely in metadata.

### 5.7 H02 work orders

| WO | Deliverable | Allowed paths (proposed) | Depends | Gate |
|---|---|---|---|---|
| H02.0 | Read-only driver-seam audit and frozen seam record | `tools/torture/runner/DRIVER_SEAMS.md` | H00 materialized corpus (draft lock acceptable; driver files hash-recorded) | Review |
| H02.1 | Runner profile and raw-plan schema; mode/suite/output contract; profile references, not duplicates, the lock's per-suite option matrix | `tools/torture/profiles/**`, `tools/torture/runner/README.md` | H02.0 | H02-G1 |
| H02.2 | DejaGnu board/site generator and `run-suite.sh` | `tools/torture/runner/dejagnu/**`, `tools/torture/runner/run-suite.sh` | H02.1 | H02-G2 |
| H02.3 | Compiler-under-test wrapper (reference/candidate, no fallback) | `tools/torture/src/bin/cc-silicon-torture-cc.rs`, `tools/torture/tests/wrapper.rs`, `Cargo.toml` (integrator approval) | H02.1; H04 flag inventory | H02-G2 |
| H02.4 | Plan-capture Tcl hook and directive coverage | `tools/torture/runner/dejagnu/plan-capture.exp`, `tools/torture/runner/tests/**` | H02.0, H02.2 | H02-G5 |
| H02.5 | Reference baseline run and evidence bundle | `tools/torture/runner/**`, external `$T00_RUN_ROOT` | H00-G3, H01 pins, H02.2–H02.4, H05 for execute/ieee | H02-G3 |
| H02.6 | Candidate baseline run; isolation/runner handoff to H05 | `tools/torture/runner/**`, external `$T00_RUN_ROOT` | H02.5, H03-G3, H04 full, T11/C02, H05 | H02-G4 |
| H02.7 | Synthetic fake-compiler self-tests (no corpus) | `tools/torture/runner/tests/**`, `tools/torture/tests/wrapper.rs` | H02.2–H02.4 | H02-G2 |

File ownership note: `tools/torture/src/lib.rs`, `src/main.rs`, and
`Cargo.toml` are shared. H02.3 must coordinate with H00.2/H00.3 and H03.1
through the integrator; do not let parallel writers touch the same entry point.

### 5.8 H02 acceptance gates

| Gate | Criterion |
|---|---|
| H02-G1 | Driver seams, profile, raw-plan schema, and mode/suite contract reviewed and frozen; exact `runtest`/global/proc names recorded from the frozen release |
| H02-G2 | Synthetic fake-compiler tests prove: compile/execute/ieee routing, reference/candidate separation, no-fallback guard, unknown-flag rejection, quoting, timeout/crash/ICE/unsupported classification, no basename collision |
| H02-G3 | Reference-only baseline on the real corpus produces official `.sum/.log` for all three suites; compile cases produce objects; execute/ieee cases actually run under H05 isolation; no candidate rate is computed |
| H02-G4 | Candidate baseline produces official logs only after H03 manifest match, Part B gates, and H05 isolation; candidate wrapper never invokes GCC/Clang for C; all failures retained |
| H02-G5 | Plan capture covers every directive encountered; unknown directive → harness gap; per-instance plan argv matches wrapper argv; no silent skips |

H02 does not claim a pass rate. H02 producing a `.sum` is not H02 acceptance by
itself; the run must have the required artifacts and evidence.

---

## 6. H03 — Instance, Option, and Predicate Manifest

### 6.1 Role and boundary

H03 freezes the **denominator plan**:

- **E**: target-applicable instances, frozen before candidate execution.
- **F**: frozen test files with at least one applicable instance.
- **Per-configuration** instance sets and counts.
- **Exclusion ledger**: pre-frozen objective exclusions, with reasons and
  proportion of the original list.

H03 does **not** compute **P**, pass rates, or failure attribution. H02 produces
raw run facts (`.sum`, `.log`, wrapper logs, exit/signal, artifacts); H06 joins
them to the frozen manifest and computes P and rates. This separation prevents a
runner from redefining the denominator after seeing results.

Proposed manifest schema: `cc-silicon.torture/manifest/v1`. Proposed raw plan:
`cc-silicon.torture/plan/v1`. Both are **proposed, not frozen**.

### 6.2 Instance identity and canonical form

One instance is:

```text
(suite, testcase, target/profile, dialect, canonical option argv,
 auxiliary sources/order, subtest ordinal)
```

For `execute` and `ieee`, compilation and running are one instance; a failure
in either step fails the instance. Do not count compile/link/run sub-results as
separate instances.

Proposed stable identity:

```text
instance_id = SHA-256(
  "cc-silicon.instance.v1\n" +
  suite + "\n" +
  relative_testcase_path + "\n" +
  subtest_ordinal + "\n" +
  config_ordinal + "\n" +
  canonical_aux_list + "\n" +
  dialect + "\n" +
  canonical_option_argv
)
```

Sort order must be explicit and stable, for example:

```text
(suite_ordinal: compile=0, execute=1, ieee=2,
 testcase_bytes,
 subtest_ordinal,
 config_ordinal,
 aux_order)
```

No hash-map iteration, locale collation, pointer address, file mtime, or host
path may decide order. Profiles/target/tooling are header-level, not per-instance;
a change to them changes the manifest ID and invalidates the run.

### 6.3 Option, predicate, header, and auxiliary lock

| Field | Rule |
|---|---|
| Base option list | Re-derive from the frozen release driver; do not assume T00's documented list is the final denominator. T00 currently records six `DG_TORTURE_OPTIONS` and seven `C_TORTURE_OPTIONS` entries; treat those as documented expectations to re-confirm. |
| Per-test options | Apply `dg-options`, additional/remove options, loop-condition drops, and driver overrides in driver order. Record the final ordered argv. |
| Dialect | Freeze the reference-confirmed default dialect and every test's `-std=` override; do not force C11 or silently relax standards. |
| Target predicates | Record expression, result, oracle, and evidence hash. Predicate evaluation must not depend on candidate capability. |
| Headers | Corpus-local headers: path + SHA-256. Sysroot headers: pinned sysroot ID + sysroot manifest hash; do not hash every system header per instance. |
| Auxiliary sources | Path + SHA-256 + order; include `.c`, `.S`, special drivers, and any driver-enumerated extra source; do not select only easy extensions. |
| LTO | Record the explicit non-LTO policy and excluded LTO rows; never mix them into E. |
| Modes | `compile` = assemble object required; `execute`/`ieee` = compile, link, run. |
| Expected outcome | `pass`, `xfail`, `expected-ice`, `expected-negative`, etc., from original directives; retain the original label. |

Option canonicalization must be an argv array, not a shell string. Repeated
options and order are significant. An option with spaces/empty value must round-
trip through the manifest parser without shell re-quoting.

Predicate policy is critical: if the candidate is used to evaluate
`dg-require-effective-target`, an incomplete candidate can make tests
"unsupported" and silently shrink E. H02/H03 must evaluate predicates with the
reference target-support oracle and freeze the results, or compare any
candidate-side predicate probe against the frozen result and fail closed on
divergence.

### 6.4 Two enumerations and manifest freeze

T00 requires "two enumerations exactly identical" and no double counting
([T00 §4](../../tasks/T00_GCC_TORTURE_GATE.md) line 70). This plan defines three
equality checks:

1. **Normalization equality:** the raw reference plan (`plan.raw`) normalized
   into the manifest must preserve every instance key, option argv, auxiliary
   source, and predicate exactly.
2. **Cross-mode equality:** the reference-mode plan, candidate-mode plan, and
   frozen manifest must have exactly the same instance-key set. Statuses may
   differ; the plan may not.
3. **Rebuild determinism:** rebuilding the manifest twice from the same frozen
   inputs and raw plan must be byte-identical (canonical form, no timestamps or
   absolute paths).

The second check is the independent cross-run guard. If reviewers require a
fully independent second directive parser, that is OQ-04.

Manifest freeze sequence:

```text
1. H00 frozen lock + H01 pins
2. H02 reference plan capture for all three suites
3. H03 build draft manifest from reference plan + lock/profile/corpus
4. H03 audit: directives, options, predicates, aux, headers, exclusions, matrix
5. H03 freeze manifest (hash) before any candidate execution
6. H02 candidate run must present the same frozen manifest hash
7. H02 candidate plan must match the frozen manifest instance set
8. H06 joins candidate results only to the frozen manifest
```

Any mismatch after step 5 aborts the run; the manifest must not be edited to fit
the candidate.

### 6.5 Per-file matrix

H03 emits the denominator matrix; H06 fills outcomes.

Proposed per-instance row:

```text
instance suite=compile testcase=<relative-path> config=<ordinal>
  id=<sha256> mode=assemble dialect=<dialect>
  option=<token> ... aux=<path>@<sha256> ...
  predicate=<id>:<expr>=<bool> ...
  expected=pass|xfail|... driver=<exp>@<sha256>
```

Proposed file aggregate:

```text
file suite=compile testcase=<relative-path>
  configs=<N> applicable=<N> excluded=<N> in-F=true|false
```

Proposed suite/config ledger:

```text
denominator suite=compile config=<ordinal> instances=<N> files=<N>
denominator suite=compile total instances=<N> files=<N> excluded=<N> original=<N>
```

Rules:

1. A file is in **F** only if it has at least one applicable instance.
2. A file is in **PF** only if **all** applicable instances pass.
3. A file with all instances excluded is not in F; report it separately as
   `NO_APPLICABLE`, not as a pass.
4. A file with an XFAIL instance is not in PF under T00's conservative rule
   (XFAIL is not P).
5. Every E instance appears exactly once in the matrix; no compile/run
   sub-result is a separate row.
6. Exclusions are pre-frozen rows with reason, authority, evidence, and original
   ordinal. Candidate-capability exclusions are forbidden.
7. Empty E or empty F is `NO_DENOMINATOR`, never a pass.

### 6.6 H03 work orders

| WO | Deliverable | Allowed paths (proposed) | Depends | Gate |
|---|---|---|---|---|
| H03.0 | Manifest/plan schema, identity, canonical order, status semantics | `tools/torture/manifest/README.md`, `tools/torture/src/manifest/schema.rs` | H02.0 seam audit | H03-G1 |
| H03.1 | Raw-plan parser and normalizer | `tools/torture/src/manifest/plan.rs`, `tools/torture/tests/manifest.rs` | H03.0, H02.4 | H03-G2 |
| H03.2 | Canonical manifest build/check/diff and CLI wiring | `tools/torture/src/manifest/{canonical,diff}.rs`, `tools/torture/src/main.rs` | H03.1 | H03-G2 |
| H03.3 | Option/predicate/header/auxiliary rules and directive coverage | `tools/torture/src/manifest/rules.rs`, tests | H03.1, H03.2 | H03-G3 |
| H03.4 | Per-file/config matrix and exclusion ledger | `tools/torture/src/manifest/matrix.rs`, tests | H03.2, H03.3 | H03-G4 |
| H03.5 | Evidence hashes, rerun determinism, H06 join contract | `tools/torture/src/manifest/evidence.rs`, `tools/torture/manifest/README.md`, tests | H03.2–H03.4, H06 schema | H03-G5 |
| H03.6 | Synthetic manifest fixtures (no corpus) | `tools/torture/tests/fixtures/manifest/**` | H03.0 | H03-G1 |

File ownership note: adding `pub mod manifest;` to `tools/torture/src/lib.rs`
and subcommands to `src/main.rs` are shared-entry edits; the integrator or a
single designated owner must sequence them after H00/H02 entry-point work.

### 6.7 H03 commands

Proposed commands (future; **not executed**):

```sh
# Build a draft manifest from the reference plan for all three suites.
cargo run --locked --manifest-path tools/torture/Cargo.toml --bin cc-silicon-torture -- \
  manifest build \
  --lock tools/torture/lock/gcc-15.2.0.torture.lock \
  --corpus "$T00_CORPUS_ROOT" \
  --profile tools/torture/profiles/linux-aarch64-lp64.profile \
  --plan-dir "$T00_RUN_ROOT/reference/attempt-1/plans" \
  --out "$T00_RUN_ROOT/manifests/linux-aarch64-lp64.manifest"

# Audit the manifest against lock/profile/corpus and all rules.
cargo run --locked --manifest-path tools/torture/Cargo.toml --bin cc-silicon-torture -- \
  manifest check \
  --manifest "$T00_RUN_ROOT/manifests/linux-aarch64-lp64.manifest" \
  --lock tools/torture/lock/gcc-15.2.0.torture.lock \
  --corpus "$T00_CORPUS_ROOT" \
  --profile tools/torture/profiles/linux-aarch64-lp64.profile

# Compare reference plan, candidate plan, and frozen manifest instance sets.
cargo run --locked --manifest-path tools/torture/Cargo.toml --bin cc-silicon-torture -- \
  manifest diff \
  --manifest "$T00_RUN_ROOT/manifests/linux-aarch64-lp64.manifest" \
  --plan "$T00_RUN_ROOT/candidate/attempt-1/plans"

# Emit the denominator matrix (E counts only; no P, no rates).
cargo run --locked --manifest-path tools/torture/Cargo.toml --bin cc-silicon-torture -- \
  manifest matrix \
  --manifest "$T00_RUN_ROOT/manifests/linux-aarch64-lp64.manifest" \
  --out "$T00_RUN_ROOT/reports/denominator/linux-aarch64-lp64.matrix.tsv"
```

Exit-code proposal: `0` verified/frozen; `1` invalid/mismatch; `2` usage or
operational error; `3` valid but draft/unfrozen. This matches the existing
verifier's discipline ([tools/torture README](../../../tools/torture/README.md)
lines 68–77).

### 6.8 H03 acceptance gates

| Gate | Criterion |
|---|---|
| H03-G1 | Schema, identity, canonical order, and status semantics reviewed; synthetic fixtures cover duplicate IDs, unknown fields, and draft/frozen behavior |
| H03-G2 | Plan normalizer and canonical builder pass synthetic tests; rebuild twice is byte-identical; `manifest diff` is empty for identical inputs |
| H03-G3 | Option order, dialect, predicates, headers, auxiliaries, `.S`, and directive coverage are locked; unknown directive → error; no candidate-capability skip |
| H03-G4 | Matrix covers every E instance exactly once; file aggregates derive F correctly; exclusions have reasons/authority/evidence; no double count |
| H03-G5 | Lock/profile/tooling/corpus hashes cross-check; two enumerations identical; no timestamps/host paths in canonical manifest; H06 join contract documented and tested |

---

## 7. Denominators and Failure Accounting

### 7.1 Core definitions

```text
E  = target-applicable instances frozen before candidate execution
P  = instances in E that satisfy the complete original test assertions
P_rate = |P| / |E|

F  = frozen test files with at least one applicable instance
PF = files in F for which all applicable instances pass
file_rate = |PF| / |F|
```

Required gates ([T00 §3](../../tasks/T00_GCC_TORTURE_GATE.md) lines 38–61):

- `compile`, `execute`, and `ieee` each have instance_rate > 0.99 and
  file_rate > 0.99.
- The pooled overall instance_rate and pooled overall file_rate each > 0.99.
- Per-configuration rates are reported, not separately gated.
- An empty denominator is not a pass.

`E` is per-suite plus pooled; instance keys are disjoint by suite. The pooled
file rate treats a file as passing only if all applicable instances across all
suites pass. If the same logical file appears in more than one suite, the
pooled file is in PF only when every applicable suite/configuration passes.

### 7.2 Failure accounting

| Condition | T00 category | In E? | In P? | Rules |
|---|---|---|---|---|
| Complete assertions pass | PASS | Yes | Yes | Only full assertion satisfaction counts |
| Assertion fails | FAIL | Yes | No | Retained in failure list |
| Expected failure | XFAIL | Yes | **No** | Original label retained; conservative T00 rule |
| Unexpected pass | XPASS | Yes | **Only if** the complete assertion holds | Retain the original label; require actual assertion evidence, not just the `.sum` label |
| Candidate missing feature | UNSUPPORTED | Yes | No | Never a skip; never a pass |
| Harness unresolved | UNRESOLVED / INFRA_ERROR | Yes | No | Infrastructure error unless evidence proves a candidate cause; blocks final publication |
| Compiler timeout | TIMEOUT | Yes | No | Process-group kill; first timeout wins |
| Compiler crash | COMPILER_CRASH | Yes | No | Signal recorded; not a normal failure |
| Internal compiler error | ICE | Yes | No | Expected ICE is separate and not a correctness success |
| Link failure | LINK_FAIL | Yes | No | Object may exist; executable does not |
| Run crash | RUN_CRASH | Yes | No | Includes signals and abnormal termination |
| Wrong result | WRONG_CODE | Yes | No | Exit/output mismatch with original assertion |
| Missing object/executable | MISSING_ARTIFACT | Yes | No | Exit 0 without artifact is not a pass |
| Expected negative result | Per original assertion | Yes | Yes only if original expectation holds | `dg-shouldfail`/expected error/output; nonzero exit alone is not decisive |
| Infrastructure error | INFRA_ERROR | **Yes** | No | Blocks final publication; fix and rerun; never delete instances |
| Pre-frozen objective exclusion | EXCLUDED | **No** | No | Reason/authority/evidence; report proportion of original list |
| Expected but absent instance | HARNESS_GAP / MISSING | **Yes** (retain expected key) | No | Must not disappear; blocks the final claim |
| Unfrozen skip/untested | HARNESS_GAP | Yes | No | Candidate capability cannot justify a skip |

DejaGnu raw statuses must be recorded unchanged and mapped in H06. In
particular:

- `PASS`/`FAIL`/`XFAIL`/`XPASS`/`UNRESOLVED`/`UNSUPPORTED`/`UNTESTED` are raw
  evidence, not the final T00 category.
- `UNSUPPORTED` or `UNTESTED` for an instance the frozen manifest says is
  applicable is a harness gap or candidate failure, not an exclusion.
- A `.sum` PASS count is not the T00 denominator; one instance may produce
  several PASS lines for sub-assertions.

### 7.3 XFAIL, XPASS, and expected negatives

- **XFAIL**: not P. The instance remains in E and in the per-file denominator.
  A file with an XFAIL instance is not PF.
- **XPASS**: P only when the actual complete assertion holds. Retain the XPASS
  label in reports; do not silently convert it to ordinary PASS.
- **Expected negative**: judge the original directive. Do not count every
  nonzero exit as failure or success. `dg-shouldfail`, expected diagnostics, and
  expected output require explicit assertion matching.
- **Expected ICE**: record separately; it is not a correctness success and does
  not enter P.
- Report both the official DejaGnu summary and the T00 instance/file accounting;
  explain the difference (sub-results, XFAIL/XPASS policy, exclusions).

### 7.4 Exclusions, LTO, and empty denominators

1. Exclusions must be frozen **before candidate execution**.
2. Allowed exclusion reasons are objective target/profile facts: target
   inapplicability established by the frozen target rules and reference probes;
   explicitly excluded non-LTO rows; or another pre-approved profile rule.
3. Candidate capability is **never** an exclusion reason. An unimplemented
   feature is `UNSUPPORTED` in E, not an excluded row.
4. LTO: the non-LTO baseline must explicitly exclude LTO rows rather than count
   them or delete them. Record the exclusion count and proportion.
5. Report `original list O`, excluded `X`, and `|X| / |O|`; also report
   `|E| = |O| - |X|` after pre-freeze verification.
6. An empty E or empty F is `NO_DENOMINATOR`; it must never be rendered as 100%
   or as a pass.
7. If a test is skipped for an unknown/unfrozen reason, the instance stays in E
   (or the expected key is retained) and the run is a harness gap.

### 7.5 Per-file and per-configuration matrix

For each file:

```text
file_denominator = all applicable instances across all applicable configurations
file_pass        = all file_denominator instances in P
```

For each configuration:

```text
config_instance_rate = |P_config| / |E_config|
config_file_rate     = |PF_config| / |F_config|
```

Per-configuration rates are reporting requirements. They must not be used to
mask a weak file or suite rate.

The per-file matrix must expose:

- suite, testcase, source hash;
- configuration ordinal and canonical options;
- applicable/excluded instance counts;
- instance IDs and modes;
- file in-F status (H03) and in-PF status (H06);
- failure categories and evidence links (filled by H06);
- exclusion reasons and authority.

### 7.6 Reference-versus-candidate accounting

- Reference results never enter candidate P.
- The reference baseline may establish the default dialect, default options,
  target predicates, and target-capability evidence; these can inform
  **pre-frozen** exclusions only.
- A reference failure does not make a candidate failure pass. H07 must
  investigate and record why the reference failed; the instance remains in E
  unless a reviewed, pre-candidate exclusion rule applies.
- Candidate H02 logs must carry the frozen manifest hash; H06 rejects results
  whose manifest hash differs.
- H06 rejects candidate results if any wrapper log shows GCC/Clang compiling
  candidate C, or if any instance is missing/extra relative to E.

---

## 8. Reproducible Evidence

Proposed external evidence root: `$T00_RUN_ROOT` (never committed by default).

```text
$T00_RUN_ROOT/
  manifests/linux-aarch64-lp64.manifest
  reference/attempt-1/
    plans/{compile,execute,ieee}.plan
    {compile,execute,ieee}/{gcc.sum,gcc.log,wrapper/,instances/}
  candidate/attempt-1/
    plans/{compile,execute,ieee}.plan
    {compile,execute,ieee}/{gcc.sum,gcc.log,wrapper/,instances/}
  reports/<batch-id>/{denominator,candidate-report,...}
```

Environment contract (all required; no silent host defaults):

| Variable | Used by | Meaning |
|---|---|---|
| `T00_CORPUS_ROOT` | H00/H02/H03 | External pruned corpus root; never a repository path |
| `T00_STAGING_ROOT` | H00 | External archive/signature staging area |
| `T00_RUN_ROOT` | H02/H03/H06 | External run/evidence root; never committed by default |
| `T00_LOCK` | H02/H03 | Frozen `lock/v2` path |
| `T00_PROFILE` | H02/H03 | Frozen runner/manifest profile path |
| `T00_REF_GCC`, `T00_REF_GCC_VERSION`, `T00_REF_GCC_SHA256` | H02 reference | Pinned reference compiler and digest |
| `T00_CANDIDATE` | H02 candidate | Absolute path to the candidate driver |
| `T00_ASSEMBLER`, `T00_LINKER`, `T00_SYSROOT`, `T00_RUNNER` | H02 | Pinned external toolchain/runner |
| `T00_DEJAGNU_BIN`, `T00_RUNTEST` | H02 | Pinned DejaGnu/runtest installation |
| `T00_GCC_RELEASE_KEY` | H00 | Authorized release-signature key reference |

A missing or empty variable is a driver error, not a request to sample the host
environment.

Per-instance evidence should include:

- frozen instance ID, manifest hash, lock hash, profile hash;
- exact argv array, cwd, environment allowlist, mode, suite, timeout;
- source/aux/header hashes and include roots;
- compiler/wrapper stdout/stderr and structured diagnostics;
- exit code and signal;
- timeout flag and which layer fired;
- object/executable hashes and ELF/symbol summary for compile/link stages;
- run stdout/stderr, exit status, signal, and assertion comparison;
- raw `.sum` line(s) and `.log` references;
- failure stage and attribution placeholder for H08/H06.

Reproducibility rules:

1. Canonical manifest/report artifacts exclude timestamps, durations, absolute
   host paths, inode numbers, and process IDs from hashed content. Put volatile
   data in a separate `run-meta` file.
2. Use relative corpus paths and content hashes; the external root is identified
   by a stable root ID, not an absolute path.
3. Record the exact commands and tool versions; do not reconstruct them from a
   shell history.
4. Hash the canonical manifest and each report; keep a hash chain
   (`manifest.sha256`, `report.sha256`, `evidence.sha256`).
5. Retain failures and infrastructure errors; do not delete or rewrite a failed
   attempt. H09's three consecutive runs are separate attempts, not retries that
   erase earlier results.
6. Re-run the same frozen profile twice in H02/H03 self-tests and compare
   canonical hashes; H09 later requires three stable runs.
7. Keep raw logs/source excerpts outside the MIT tree unless legal review
   approves. Commit only schemas, templates, hashes, and non-GPL summaries.

`tools/torture/reports/` is proposed for report schemas/templates and small
non-GPL summaries, not for raw corpus evidence.

---

## 9. Dependencies, Sequencing, and Blockers

### 9.1 Dependency table

| Dependency | Needed by | Why | Current state |
|---|---|---|---|
| H00 authorization and network approval | H00.1 | Fetch GCC assets | Absent |
| H01 Linux AArch64 substrate, sysroot, assembler, linker, runner | H00 freeze, H02 reference, H02 candidate | Lock substrate fields and run target binaries | Unresolved; probe never run |
| H01 probe report + H07 classification + T01 attestation | H02 candidate, T11 | Target values/ABI before codegen and execution | Unverified |
| H00 `lock/v2` | H02/H03 | Per-suite options, tooling pins, LTO, shared assets | Absent |
| Legal review | H00 freeze | Lawful external corpus distribution | Absent |
| H04 `-I` + multi-source freeze | H02 candidate | Includes and multi-file torture cases | Draft only |
| H04 full flag set and `-S`/`-c` | H02 candidate | Torture compile/link path | Refused/absent |
| T11 target emission | H02 candidate | Candidate-generated AArch64 assembly/object | Absent |
| C02 attestation | H02 candidate | Unlock codegen | Unverified |
| H05 isolation | H02 reference execute/ieee, H02 candidate | Safe execute/ieee, timeout/crash classification | Absent |
| H06 aggregation | H02/H03 final reporting | P, rates, dual summaries | Absent |
| H07 reference oracle | H02 reference/H03 | Default dialect/options, predicate/target evidence | Absent |
| DejaGnu/Tcl/Expect/runtest pins | H02 | Reproducible official logs | Unpinned |
| CI runner / action pins | H02/H09 | Automated or manual reproducibility | Not provisioned; action tags mutable |
| GPL evidence retention policy | H02/H03 | Logs may contain source excerpts | Not decided |

### 9.2 Phasing

**Phase 0 — Design review (this report).**
Review and accept/reject the proposed lock `v2`, raw-plan schema, manifest
schema, and two-enumeration interpretation. No implementation.

**Phase 1 — Corpus-free scaffolding (parallelizable).**

- H00.1–H00.4 on synthetic archive/tree fixtures.
- H02.1–H02.4 and H02.7 on synthetic fake-compiler/DejaGnu fixtures; H02.0
  starts as soon as the H00 materialized corpus exists (draft lock acceptable).
- H03.0–H03.2 and H03.6 on synthetic plans.
- Allowed without a frozen corpus, probe, or target execution; no rate claim.

**Phase 2 — Reference baseline.**

- H00.5–H00.6 with real fetch, legal sign-off, H01 pins, frozen lock.
- H02.5 reference baseline on Linux AArch64; H05 isolation is required before
  execute/ieee reference runs.
- H03.3–H03.5 build and audit the manifest from the reference plan.
- H07 confirms default dialect/options and classifies target evidence.
- Exit: frozen manifest hash, official reference logs, denominator audit.

**Phase 3 — Candidate baseline (Part B).**

- H04 full driver + T11 codegen + C02 attestation + H05 isolation.
- H02.6 candidate run against the frozen manifest.
- H06 joins results and computes P/rates.
- H08 localizes failures; H09 runs the 3-run stability gate.
- Exit: official candidate logs, every failure attributed, no missing/extra
  instances.

### 9.3 Hard blockers

1. **No legal authorization / no real corpus**: H00 cannot freeze.
2. **H01 probe never run**: target values and C02 remain unverified; H02
   candidate is blocked by design.
3. **H04 incomplete and `-I` freeze draft**: no includes, no multi-source, no
   target `-c`/`-S`.
4. **T11/C02 absent**: no candidate target code.
5. **H05 absent**: no safe execute/ieee run.
6. **H06 absent**: no P or rate can be computed.
7. **No DejaGnu/Tcl/Expect pins**: official logs are not reproducible yet.

These are real blockers, not reasons to narrow E or skip tests.

---

## 10. Open Questions

| ID | Question | Why it matters | Proposed owner |
|---|---|---|---|
| OQ-01 | Is the normative corpus the release tarball or the signed git tag? | File bytes and inventory can differ | H00 + integrator |
| OQ-02 | Should H00 use `lock/v2` or a separate profile artifact referencing `lock/v1`? | Per-suite options/tooling pins need a representation | H00 + integrator |
| OQ-03 | How exactly can the official driver emit a per-instance plan without modifying upstream `.exp` files? | H02/H03 two-enumeration gate depends on it | H02 owner |
| OQ-04 | Is reference-plan vs candidate-plan equality sufficient for "two enumerations," or is a fully independent directive enumerator required? | Acceptance interpretation | H03 reviewer + T00 owner |
| OQ-05 | Which target predicates may use candidate compilation, and which must use the reference oracle? | Candidate capability must not shrink E | H02/H03 + H07 |
| OQ-06 | What reference failures may justify a pre-frozen target exclusion, and who signs it? | Prevents post-hoc exclusion | H07 + integrator |
| OQ-07 | What are the per-instance compile/link/run timeout and resource limits? | H05 policy; classification stability | H05 owner |
| OQ-08 | Which torture cases use `dg-output`, `dg-shouldfail`, `dg-expect-error`, debug/dump/scan, or `.S`? | Requires exact original-assertion handling | H03 + H08 |
| OQ-09 | How are LTO rows detected and excluded in the frozen harness? | T00 requires explicit non-LTO handling | H00/H02 |
| OQ-10 | What is the frozen link rule: raw `ld`, GCC link driver only, startup objects, PIE, libgcc/libc/libm? | Candidate objects must link without C compilation | H01/H02 |
| OQ-11 | What is the default dialect and option list confirmed by the reference run? | Cannot assume C11 or T00's documented list | H07 |
| OQ-12 | What evidence may be committed given GPL test-source excerpts in logs? | Legal/provenance boundary | H00 legal + integrator |
| OQ-13 | Should H02/H03 live in `cc-silicon-torture` or a separate runner crate? | Shared entry points/Cargo ownership | integrator |
| OQ-14 | What is the H06 join schema for raw `.sum`, wrapper logs, and manifest? | H03/H06 boundary | H06 owner |
| OQ-15 | Are GitHub Actions pinned to immutable SHAs for the torture jobs? | Supply-chain reproducibility | integrator/security |
| OQ-16 | Is native AArch64 required, or is qemu-aarch64 acceptable? | Runner exit-code and signal fidelity | H01/H05 |

---

## 11. Non-Claims and Review Checklist

This document does **not** claim:

- a fetched corpus, a frozen lock, or a real asset hash;
- a DejaGnu run, reference baseline, candidate run, or official `.sum/.log`;
- a target probe, ABI value, attestation, or T11 codegen;
- any corpus file count, instance count, denominator, or pass rate;
- any H02/H03 implementation or CI gate;
- any legal conclusion about GPL assets.

Review checklist before accepting this plan:

- [ ] H00/H02/H03 scopes and owner boundaries are accepted.
- [ ] Lock `v2` (or the chosen alternative) is approved, including per-suite
      options, tooling pins, LTO, shared assets, and reference pins.
- [ ] The raw-plan and manifest schemas are approved (or explicitly marked
      draft for Phase 1).
- [ ] The two-enumeration interpretation is accepted (OQ-04).
- [ ] Predicate-oracle policy is accepted (OQ-05).
- [ ] Evidence retention/legal boundary is accepted (OQ-12).
- [ ] No work order starts before its readiness gate and dependency closure.
- [ ] No candidate torture claim is made before H00, H01/C02, H04, T11, H05,
      H06, and H09 are satisfied.

---

## 12. References

- [AGENTS.md](../../../AGENTS.md) — repository rules, validation, security,
  legal/provenance boundaries.
- [T00 GCC torture gate](../../tasks/T00_GCC_TORTURE_GATE.md) — corpus freeze,
  suites, options, denominators, failure accounting, H00–H10.
- [T00/H00 implementation status](../../tasks/T00_H00_IMPLEMENTATION_STATUS.md) —
  current scaffold and legal/tooling limitations.
- [tools/torture README](../../../tools/torture/README.md) — lock format,
  verifier behavior, exit codes, security model.
- [tools/torture source](../../../tools/torture/src/) — `lock.rs`, `verify.rs`,
  `main.rs`, `digest.rs`, `path.rs`, `diagnostic.rs`.
- [tools/torture tests](../../../tools/torture/tests/) — synthetic lock/CLI
  coverage.
- [H04 candidate driver](../../tasks/H04_CANDIDATE_DRIVER.md) — Part A,
  `-E/-D/-U`, deferred flags.
- [H04 `-I`/multi-source freeze](../../tasks/H04_I_FREEZE.md) — **DRAFT ONLY**,
  not accepted; do not assume.
- [M1 target acceptance](../../tasks/M1_TARGET_ACCEPTANCE.md) — target identity,
  Part B gates, evidence discipline.
- [H01 probe README](../../../tools/torture/probe/README.md) — probe harness and
  unresolved ABI fields.
- [PARALLEL_EXECUTION.md](../../tasks/PARALLEL_EXECUTION.md) — waves, ownership,
  handoff template.
- [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) — Stage A/B
  task pools and readiness gates.
- [DISPATCH_LEDGER.md](../../tasks/DISPATCH_LEDGER.md) — A21 assignment and
  write restrictions.
- [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) — current
  torture crate checks.
- [`.github/workflows/t00-target-probes.yml`](../../../.github/workflows/t00-target-probes.yml) —
  manual probe job and trust-boundary notes.
- [project status](../../guide/PROJECT_STATUS.md) — current framework/compiler
  status and known limitations.
