# A-ABI-01 — Minimal AArch64 Linux Target Chain Plan

| Field | Value |
|---|---|
| Status | **PROPOSAL — Stage A planning report; not an accepted contract, not implementation authorization** |
| Date | 2026-11-07 |
| Scope | H01 substrate pins, H07 ABI classification, attestation handoff, minimal machine/assembly contract, Constant/Return target path, assembler/linker/runner boundary, fail-closed behavior, M1 acceptance evidence for `int main(void){return 2+3;}` |
| Baseline artifact | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24`; `target_verification=unverified` |
| Target identity | `aarch64-unknown-linux-gnu`, ELF, LP64, little-endian, AAPCS64 (identity only is frozen) |
| Evidence run for this report | **Static inspection only. No probe, compiler, assembler, linker, runner, test, or target execution was performed.** |
| Non-claim | No concrete ABI value is verified. No M1 target acceptance is passed. No torture rate is measured or implied. |

> **Evidence discipline.** `[FACT]` means a static reading of the current tree (document or
> source), not a test result. `[PROPOSAL]` means a design recommendation in this report.
> `[UNVERIFIED]` means a concrete target value that must not drive code generation.
> `[NOT RUN]` means a command is specified but was not executed for this report.
> Nothing in this report may be converted into probe data or a pass result without the
> H01 → H07 → T01-integrator handoff described in §2.

> **Hard rule for all concrete ABI values.** Every concrete scalar/ABI value in the tree
> (`compiler/contracts/target/aarch64-linux-probe.txt`, `TargetSpec::aarch64_unknown_linux_gnu_unverified`,
> and any number quoted below) is an **expectation/proposal**, not evidence. In particular,
> the current proposal numbers for `abi.gp_arg_regs`, `abi.fp_arg_regs`, `abi.stack_align`, and
> `abi.variadic_register_save_area` are **UNVERIFIED** and must not be copied into an
> attestation report. The H01 normalizer emits those four fields as `unresolved`; H07 must
> derive them from captured evidence.

---

## 1. Scope, Baseline, and Non-Claims

### 1.1 Scope

A-ABI-01 is the cross-cutting Stage A plan needed to turn the current symbolic M1 evidence
into a real AArch64 Linux compile/link/run closed loop. It covers:

1. the H01 substrate/toolchain pins and probe commands, with no results invented;
2. the H07 classification of the H01 report’s unresolved ABI fields;
3. the exact probe-report → attestation handoff, including hash and provenance rules;
4. the smallest machine-record and assembly-output contract that can carry M1;
5. the IR `Constant`/`Return` → machine instruction path;
6. the Host-side assembler/linker/runner boundary;
7. fail-closed behavior before attestation;
8. the acceptance evidence required for `int main(void){return 2+3;}` returning `5`.

Non-goals: full AAPCS64 argument/return coverage, floating point, aggregates, variadics,
`long double`, `__int128`, TLS, relocations/PIC, unwinding/debug, optimization, the GCC
torture matrix, and H02/H03/H05/H06/H08/H09/H10 implementation. Those remain T11/T00 work
after the minimal chain is accepted.

### 1.2 Static evidence baseline

The following facts were read from the current tree. No command below was executed as a
test or probe.

| Fact | Evidence | Meaning |
|---|---|---|
| Current frozen artifact is `t01-c01-c06/39` with the recorded hash | `compiler/contracts/CONTRACT_VERSION:3-4`; `compiler/src/contract.rs:38-44` | The foundation envelope is frozen; this is not a target-code or `/6` language freeze. |
| Target verification is currently unverified | `compiler/contracts/CONTRACT_VERSION:5-7` | `target_verification=unverified`; probe substrate is “planned, not provisioned”. |
| Concrete target values are explicitly proposals | `compiler/src/target.rs:577-620` | `aarch64_unknown_linux_gnu_unverified()` constructs the expected values with `VerificationState::UNVERIFIED`. |
| The probe fixture is unverified | `compiler/contracts/target/aarch64-linux-probe.txt:1-9` | `verified = false`; the listed numbers are expectations, not a report. |
| The H01 harness exists but has never run | `tools/torture/probe/README.md:1-8`, `:223-238` | No probe report or report hash exists; H01 is not complete; C02 is not verified. |
| The normalizer deliberately leaves four ABI fields unresolved | `tools/torture/probe/normalize/normalize.py:91-99` | `abi.gp_arg_regs`, `abi.fp_arg_regs`, `abi.stack_align`, `abi.variadic_register_save_area` are always emitted as `unresolved` by H01. |
| The report must contain 38 required fields | `compiler/src/target.rs:493-532`; `tools/torture/probe/README.md:205-221` | Attestation requires all 38; H01 emits them or `unresolved`. |
| Assembly is the classification evidence for those four fields | `tools/torture/probe/README.md:216-221` | H07 must interpret the hashed `abi-args.s`; H01 does not guess counts. |
| `abi.*.ok` self-checks are not a H01 gate | `tools/torture/probe/README.md:158-177` | A complete report with a valid hash is **not** a self-check pass; H07 must require every `abi.*.ok` to be `1` and present. |
| Attestation validates caller-supplied data and a hash, not authenticity | `compiler/src/target.rs:658-716`, `:858-892`; `compiler/README.md` C02 notes | `TargetSpec::attest` checks fields, identity, and `report_hash`; it does not prove a physical probe ran (TOCTOU). |
| The candidate driver is Part A only and refuses target emission | `compiler/src/bin/candidate.rs:104-107`, `:199-210`; `compiler/tests/h04_candidate.rs:195-208` | `-S`/`-c` exit 2 with a fail-closed message; no output file. |
| The IR path currently emits only `Constant` + `Return` | `compiler/src/chips/ir_lower/ir_function.rs:321-384`; `compiler/src/bus.rs:442-494` | One function, one block, one value, `Constant`, `Return`; no target lowering. |
| The symbolic interpreter models return `5` | `compiler/tests/c17_vf12.rs` (whole file) | This is Part A verification, not target execution. |
| No target-code chips exist | `compiler/src/chips/` has no `target/` group; `compiler/src/manifest.rs:758-765` only defines the backend rule | The `TARGET_CODE` task group exists, but no target task kind/chip is registered. |
| The machine store has no declared schema | `compiler/src/task.rs:2030-2060`; `compiler/src/manifest.rs:160-211` | `StoreId::Machine` exists; no `machine.*` field is declared; no machine record family is typed. |
| Virtual registers are a reserved arena only | `compiler/src/bus.rs:646-658` | `VRegId` exists; `VRegRecord` is not typed yet. |
| The contract’s current target model is unverified | `compiler/src/contract.rs:489-511` | `FrozenSchema::current()` uses `aarch64_unknown_linux_gnu_unverified()`; the version file hardcodes `target_verification=unverified` in `contract_version_file()`. |
| T11 target work is Part B and probe-gated | `docs/tasks/T11_TARGET_CHIPS.md:3-7` | Every concrete ABI value in T11 is an expectation to confirm; target work is blocked until probe + integrator attestation. |
| M1 target acceptance is not runnable today | `docs/tasks/M1_TARGET_ACCEPTANCE.md:59-106`, `:247-339`; `docs/tasks/T00_GCC_TORTURE_GATE.md:79-117` | Part B (assembly/link/run) is unavailable; H05 isolation is absent. |
| H04 documentation/test-count drift | `docs/tasks/H04_CANDIDATE_DRIVER.md:52-58` says 18 tests; `docs/tasks/M1_TARGET_ACCEPTANCE.md:461` still says 6 | Static count in `compiler/tests/h04_candidate.rs` is 18 `#[test]` items. This is a documentation consistency note, not a target fact. |

### 1.3 Non-claims

- This report does **not** run H01, H07, attestation, `cargo test`, `cargo build`, `as`, `ld`,
  the candidate driver, or any target binary.
- This report does **not** assert that AAPCS64 values are `8/8/16/true` or any other
  concrete value. The current source/fixture contains those as proposals; they are
  **UNVERIFIED**.
- This report does **not** change `compiler/src/target.rs`, the probe fixture, the contract
  hash, `CONTRACT_VERSION`, any chip, Cargo file, or shared index. It is the only file
  created by this task.
- macOS arm64 values, Mach-O, and Darwin’s stack-only variadic model are forbidden
  substitutes at every step.

---

## 2. H01, H07, and Attestation

### 2.1 H01 substrate pins

H01 owns the substrate, toolchain pins, probe harness, probe run, and hashed report. H01
does **not** classify the four ABI fields and does **not** attest the target. All values
below are required inputs; none is established here.

| Pin | Required state | Recorded by H01 | Fail-closed owner |
|---|---|---|---|
| Host OS/architecture | Native 64-bit Linux AArch64 (`uname -s` = `Linux`; `uname -m` = `aarch64`/`arm64`; `getconf LONG_BIT` = 64) | `substrate.uname_s`, `substrate.uname_m`, `substrate.uname_r`, `substrate.long_bit` | H01 platform guard |
| Reference compiler path | Absolute path or resolvable command, recorded | `toolchain.gcc.path` | H01 |
| Reference compiler version | Exact `-dumpfullversion`/`-dumpversion`; explicit pin | `toolchain.gcc.version`, `toolchain.gcc.dumpversion` | H01 version guard |
| Reference compiler binary digest | 64 lowercase hex; matches `sha256sum`/`shasum -a 256` | `toolchain.gcc.sha256` | H01 digest guard |
| Reference compiler triple | `aarch64*linux*gnu*`, not big-endian/ILP32; consistent with the frozen identity | `toolchain.gcc.dumpmachine`, `target.gcc_triple` | H01 triple guards |
| Sysroot | Explicit value from the pinned compiler | `toolchain.gcc.sysroot` | H01 |
| Include directory | Explicit value | `toolchain.gcc.include` | H01 |
| libgcc | Explicit value | `toolchain.gcc.libgcc` | H01 |
| Assembler | Reachable via `gcc -print-prog-name=as`, executable | `toolchain.assembler.path`, `toolchain.assembler.banner` | H01 |
| Linker | Reachable via `gcc -print-prog-name=ld`, executable | `toolchain.linker.path`, `toolchain.linker.banner` | H01 |
| libc | glibc version recorded (not asserted) | `substrate.libc`, `substrate.libc_version` | H01 |
| Probe tooling | `python3`, `od`, `getconf`, `uname`, `awk`, `tr`, `head`, `sed`, `sha256sum` or `shasum` | tool/version captures | H01 guard |
| Runner/CI image | Native AArch64 runner; image digest is **recorded metadata only**, not verified | workflow `probe-report/toolchain-metadata.txt` (`toolchain.digest.metadata`, `toolchain.digest.enforced=false`) | Workflow/H01 |
| Candidate-free probe | `T00_CANDIDATE_CC` unset; probe is reference-only | `probe.candidate_free=true` | H01 guard |
| Frozen target triple | Only `aarch64-unknown-linux-gnu` accepted | `target.triple` | H01 guard |

Important boundary: the H01 harness **cannot verify the runner image digest** and does not
claim to. The workflow records a human-declared digest as metadata only. This is a
provenance limitation, not a pin.

#### 2.1.1 Pin-capture commands (NOT RUN)

These are the commands H01 must use to establish the pins. Placeholders must be replaced by
the frozen H01 inputs; they are not values.

```sh
# Host and libc identity
uname -s
uname -m
uname -r
getconf LONG_BIT
getconf GNU_LIBC_VERSION

# Reference compiler identity and pins
"$T00_REF_GCC" --version | head -n 1
"$T00_REF_GCC" -dumpfullversion
"$T00_REF_GCC" -dumpversion
"$T00_REF_GCC" -dumpmachine
"$T00_REF_GCC" -print-sysroot
"$T00_REF_GCC" -print-file-name=include
"$T00_REF_GCC" -print-libgcc-file-name
sha256sum "$T00_REF_GCC"        # or: shasum -a 256 "$T00_REF_GCC"

# Binutils
"$T00_REF_GCC" -print-prog-name=as
"$T00_REF_GCC" -print-prog-name=ld
"$("$T00_REF_GCC" -print-prog-name=as)" --version | head -n 1
"$("$T00_REF_GCC" -print-prog-name=ld)" --version | head -n 1
```

#### 2.1.2 Probe commands (NOT RUN)

Compiler-free self-tests (safe anywhere; they do not produce target facts):

```sh
bash tools/torture/probe/tests/run-tests.sh
# or
bash tools/torture/probe/run-probe.sh --self-test
```

Real probe (native Linux AArch64 only; do **not** run on macOS):

```sh
T00_REF_GCC=/absolute/path/to/pinned/aarch64-linux-gnu-gcc \
T00_REF_GCC_VERSION=<exact-version> \
T00_REF_GCC_SHA256=<64-lowercase-hex> \
T00_REF_GCC_TRIPLE=<exact-gcc-dumpmachine> \
T00_TARGET_TRIPLE=aarch64-unknown-linux-gnu \
T00_PROBE_OUTDIR=/absolute/run/t00-probe-<run-id> \
bash tools/torture/probe/run-probe.sh
```

The CI equivalent is a manual `workflow_dispatch` of
`.github/workflows/t00-target-probes.yml` with the four declared inputs
(`toolchain_digest`, `ref_gcc`, `ref_gcc_version`, `ref_gcc_sha256`); the runner label is
hard-coded to `ubuntu-24.04-arm` and is not an input. The workflow installs nothing and
fetches no GCC, corpus, or image.

Do not run this on the macOS development host. The platform guard rejects non-Linux/non-AArch64
before compiling, and macOS values are forbidden even if a guard were bypassed.

#### 2.1.3 H01 output and acceptance checks

Expected output tree:

```text
$T00_PROBE_OUTDIR/
  raw/                    raw reference-GCC captures (meta, macros, scalars, abi, asm, versions)
  normalized/report.txt   deterministic, path-stripped report
  normalized/report.sha256
  normalized/evidence/    normalized evidence blobs referenced by the report
```

H01 report acceptance (H01-owned; NOT RUN):

1. `normalized/report.sha256` verifies against `report.txt`
   (`cd normalized && sha256sum -c report.sha256`).
2. The report contains all 38 required fields, with the four ABI fields present as
   `unresolved` (H01 must not guess them).
3. Identity fields are `target.triple=aarch64-unknown-linux-gnu`, `object_format=elf`,
   `endianness=little`, `data_model=lp64`.
4. Platform fields show Linux/AArch64/64-bit; `probe.candidate_free=true`;
   `probe.claim=no-c02-verification`.
5. Toolchain pins match the declared inputs; the GCC binary digest matches; the
   `-dumpmachine` triple is consistent with the frozen target.
6. Evidence blobs exist and their `evidence.*.sha256` values match the report.
7. The eight `abi.*.ok` fields are present (H01 does not require them to be `1`; H07 does).
8. No `verified=true` and no `report_hash` are invented by H01. The report is evidence,
   not C02 verification.

The report also records `substrate.native_aarch64_assumed=true` and
`substrate.emulation_detection=not-performed`: the platform guard cannot prove the CPU is
not emulated on another kernel. That is a documented limitation, not a hidden pass.

H01 must not edit the report after production. If the probe must be rerun, it produces a new
`$T00_PROBE_OUTDIR`/run ID; old evidence is retained, not overwritten.

### 2.2 H07 ABI classification

H07 consumes the H01 report and its hashed evidence; it does **not** rerun the H01 harness,
compile anything, or change the H01 report/evidence. H07’s job is to turn the four
`unresolved` fields into a documented classification, with field-level evidence pointers,
and to validate the rest of the ABI evidence.

#### 2.2.1 H07 inputs

- `normalized/report.txt` and `report.sha256`;
- `normalized/evidence/abi-args.s` and its `evidence.abi-args.s.sha256`;
- `normalized/evidence/versions-full.txt` and its hash;
- `normalized/evidence/macros.txt` and its hash;
- `raw/abi.txt`, `raw/scalars.txt`, `raw/meta.txt` (cross-checks only; the normalized
  report/evidence is the authoritative hashed input);
- the pinned toolchain/substrate facts in the report.

H07 must first verify the report file hash and every evidence hash. It must not classify
from an unverified or partially copied report.

#### 2.2.2 Fields and classification rules

The four currently unresolved fields are the required H07 output. The classification must
be derived from emitted assembly or measured fields; the AAPCS64 expectation and the
current `target.rs` proposal are **cross-checks, not evidence**.

| Field | Type | Evidence source | Proposed classification rule | Failure state |
|---|---|---|---|---|
| `abi.gp_arg_regs` | `u8` | `abi_gp_ten` in `abi-args.s`; cross-check `abi_mixed_many` | Identify the non-stack integer argument registers actually used for the 10 `long` parameters. If arguments `0..N-1` use distinct GP argument registers and argument `N` is read from a stack slot, record `N`. Reject zero, overflow beyond the `u8`/ABI register space, or inconsistent evidence. Do not infer from the function declaration or AAPCS64 prose. | `unresolved` / block attestation |
| `abi.fp_arg_regs` | `u8` | `abi_fp_ten` in `abi-args.s`; cross-check `abi_mixed_many` | Same rule for the 10 `double` parameters: count distinct FP/SIMD argument registers before the first stack-homed argument. Treat `dN`/`vN`/`sN` spellings of the same register as one register. Record the observed count, not the expected one. | `unresolved` / block attestation |
| `abi.stack_align` | `u8` | Prologues/epilogues and stack argument offsets in `abi_gp_ten`, `abi_fp_ten`, `abi_mixed_many`, `abi_variadic_sum` | Derive the ABI-required alignment, **not** the largest frame size. Inspect stack argument slot offsets, SP adjustments, and any explicit SP mask (`and sp, sp, #-A`); a frame adjustment larger than the requirement is not evidence for a larger requirement. Record the value only when the evidence uniquely supports it; if the evidence gives only a lower bound, record `unresolved` rather than guessing. If the expected public-interface alignment (currently proposed as 16, **UNVERIFIED**) is directly evidenced, record it with an explicit residual-risk note. | `unresolved` / block attestation |
| `abi.variadic_register_save_area` | `bool` | `abi_variadic_sum` body in `abi-args.s`; `abi.sizeof.va_list`, `abi.alignof.va_list` | `true` only if the emitted variadic callee stores the incoming GP and FP argument registers into a register-save area and the `va_list`/`va_arg` path reads from it. `false` only if the assembly explicitly uses a stack-only variadic model; otherwise `unresolved`. Do not set `true` from the type declaration or AAPCS64 prose alone. | `unresolved` / block attestation |

The other required fields must also be validated for attestation:

- all 33 non-ABI scalar/identity fields (`triple`, `object_format`, `endianness`, `data_model`,
  `sizeof.*`, `alignof.*`, `char.signed`, `wchar_t.signed`, `longdouble.format`);
- `wchar_t.encoding` must be a recognized resolved label (`utf32`/`utf16`), not
  `unresolved`; the H01 normalizer only emits `utf32` with positive ISO/IEC 10646 +
  non-BMP literal evidence;
- all eight `abi.*.ok` self-checks must be **present and equal to `1`**; a `0` or missing
  check is not passing ABI evidence (H01 itself does not enforce this);
- no required field may remain `unresolved` in the report used for attestation.

#### 2.2.3 H07 evidence commands (NOT RUN)

These commands read the H01 output only. They do not compile or execute anything.

```sh
RUN=/absolute/run/t00-probe-<run-id>
REPORT="$RUN/normalized/report.txt"
EVID="$RUN/normalized/evidence"

# 1. Verify the H01 report hash and evidence hashes.
( cd "$RUN/normalized" && sha256sum -c report.sha256 )
awk -F' = ' '$1 == "evidence.abi-args.s.sha256" { print $2 }' "$REPORT"
sha256sum "$EVID/abi-args.s"
awk -F' = ' '$1 == "evidence.versions-full.txt.sha256" { print $2 }' "$REPORT"
sha256sum "$EVID/versions-full.txt"
awk -F' = ' '$1 == "evidence.macros.txt.sha256" { print $2 }' "$REPORT"
sha256sum "$EVID/macros.txt"

# 2. Inspect the ABI assembly evidence (text only; record line numbers).
sed -n '/^abi_gp_ten:/,/^[[:space:]]*\.size[[:space:]]*abi_gp_ten/p' "$EVID/abi-args.s"
sed -n '/^abi_fp_ten:/,/^[[:space:]]*\.size[[:space:]]*abi_fp_ten/p' "$EVID/abi-args.s"
sed -n '/^abi_mixed_many:/,/^[[:space:]]*\.size[[:space:]]*abi_mixed_many/p' "$EVID/abi-args.s"
sed -n '/^abi_variadic_sum:/,/^[[:space:]]*\.size[[:space:]]*abi_variadic_sum/p' "$EVID/abi-args.s"
sed -n '/^abi_ret_hfa_4f:/,/^[[:space:]]*\.size[[:space:]]*abi_ret_hfa_4f/p' "$EVID/abi-args.s"
sed -n '/^abi_ret_hfa_4d:/,/^[[:space:]]*\.size[[:space:]]*abi_ret_hfa_4d/p' "$EVID/abi-args.s"
sed -n '/^abi_ret_split:/,/^[[:space:]]*\.size[[:space:]]*abi_ret_split/p' "$EVID/abi-args.s"
sed -n '/^abi_ret_big:/,/^[[:space:]]*\.size[[:space:]]*abi_ret_big/p' "$EVID/abi-args.s"

# 3. Record the observed stack adjustments/offsets and variadic save-area stores.
grep -nE 'sub[[:space:]]+sp, sp, #|add[[:space:]]+sp, sp, #|and[[:space:]]+sp, sp, #' "$EVID/abi-args.s"
grep -nE 'stp[[:space:]]+x[0-9]+, x[0-9]+, \[sp|stp[[:space:]]+d[0-9]+, d[0-9]+, \[sp|ldr[[:space:]]+x[0-9]+, \[sp|ldr[[:space:]]+d[0-9]+, \[sp' "$EVID/abi-args.s"
```

The H07 annex must cite exact evidence paths and line numbers (or symbol-relative
instruction ordinals) for every derived value. A narrative “AAPCS64 says 8” is not a
classification record.

#### 2.2.4 H07 output and hash

Proposed H07 outputs (paths are proposals; H07 owns them, not this report):

```text
tools/torture/reports/target/<run-id>/h07/
  classification.md          human-readable annex with the four field records
  resolved-fields.txt        exactly the four resolved `field=value` lines
  resolved-fields.sha256     SHA-256 of resolved-fields.txt
  annex.sha256               SHA-256 of classification.md
```

Each field record must include at least:

```text
field=abi.gp_arg_regs
value=<derived-integer>
derivation_rule=<rule-id or exact prose>
evidence_path=normalized/evidence/abi-args.s
evidence_sha256=<64 lowercase hex>
evidence_locator=<symbol + line/instruction range>
observed=<raw observation>
cross_check=<secondary observation or none>
residual_risk=<none or explicit limitation>
source_report_sha256=<H01 report.sha256>
source_report_canonical_body_sha256=<optional H01 body hash>
classifier=<name/version>
result=resolved | unresolved
```

`resolved-fields.txt` must contain only the four `field=value` lines, sorted by field name,
with no comments. This fragment is what the integrator merges; it reduces manual
transcription risk. If any field is `unresolved`, H07 must not emit a “resolved” fragment
and attestation must not proceed. H07 outputs must be canonical (LF, sorted field order,
no wall-clock timestamps, no host-specific absolute paths) so their hashes are reproducible;
use stable run-relative placeholders for evidence paths.

### 2.3 Attestation handoff

#### 2.3.1 Handoff chain

```text
H01 run
  -> raw/ + normalized/report.txt + report.sha256 + normalized/evidence/*
H07 classification
  -> classification.md + resolved-fields.txt + hashes
T01 integrator
  -> canonical attestation report (all 38 fields + provenance + verified=true + report_hash)
  -> ProbeReport::status / TargetSpec::attest
  -> VerificationState::Probed { report_hash }
  -> contract update (verified target model + version/hash/provenance)
  -> codegen gate opens only for that attested target
```

H01 owns the probe run and hashed report. H07 owns the documented ABI classification.
The T01 integrator is the **only** owner who may call `TargetSpec::attest`/mark the target
`Probed` and incorporate the report hash. No probe result or attestation exists today.

#### 2.3.2 Artifact and hash inventory

| Artifact | Producer | Hash | Covers |
|---|---|---|---|
| Raw capture under `raw/` | H01 | none (inputs to normalization) | Reference-GCC output |
| `normalized/report.txt` | H01 normalizer | `report.sha256` = SHA-256 of the **report file bytes** | The file including comments and `key = value` spacing |
| Evidence blobs | H01 normalizer | `evidence.<name>.sha256` fields in the report | Exact normalized evidence bytes (`abi-args.s`, `versions-full.txt`, `macros.txt`) |
| H07 classification annex | H07 | `annex.sha256` | H07 field records and evidence pointers |
| H07 resolved-field fragment | H07 | `resolved-fields.sha256` | Exactly the four resolved field values |
| Attestation report | T01 integrator | `report_hash` = SHA-256 of the **canonical unsigned body** | Sorted `key=value\n` lines for every field except `report_hash` |
| Contract artifact | T01 integrator | `CONTRACT_HASH` | Normative shapes/rule ids, not probe authenticity |

Critical distinction: `report.sha256` (H01 file hash) is **not** `report_hash`
(attestation canonical-body hash). The H01 report is written with comments and `key = value`
spacing; `ProbeReport::unsigned_body()` emits `key=value\n` in sorted key order and excludes
`report_hash` (see `compiler/src/target.rs:858-892`). Both hashes must be recorded, and
neither may be substituted for the other.

#### 2.3.3 Canonical attestation report rules

The integrator must build the attestation report deterministically, for example:

```python
# Proposed canonicalization; the Rust implementation in ProbeReport::unsigned_body()
# remains authoritative.
body = "".join(f"{k}={v}\n" for k, v in sorted(fields.items()) if k != "report_hash")
report_hash = sha256(body.encode("utf-8")).hexdigest()
```

Requirements:

1. All 38 `REQUIRED_PROBE_FIELDS` are present.
2. `triple`, `object_format`, `endianness`, `data_model` match the frozen identity.
3. The four H07 fields are the exact derived values from `resolved-fields.txt`; no manual
   retyping and no copying from `target.rs`/the fixture.
4. `wchar_t.encoding` and `longdouble.format` are recognized resolved values.
5. Every numeric field parses as the expected type (`u8`/bool) and is internally plausible;
   H07/integration must reject zero register counts, non-power-of-two stack alignment, or
   values that `TargetSpec::attest` would otherwise accept without cross-checks.
6. `verified=true`.
7. `report_hash` is lowercase hex over the canonical body; it is the **last** field added
   and is excluded from its own body.
8. Provenance fields are included in the body and therefore bound by `report_hash`, for
   example:
   `provenance.h01_report_file_sha256`, `provenance.h01_report_canonical_body_sha256`,
   `provenance.h07_annex_sha256`, `provenance.h07_resolved_fields_sha256`,
   `provenance.toolchain_gcc_sha256`, `provenance.substrate_run_id`.
   Extra keys are retained and hashed by `ProbeReport`; they do not need new `TargetSpec`
   fields.
9. Keep the H01 legacy projections (`triple`, `object_format`, `endianness`, `data_model`)
   consistent with their `target.*` counterparts. `TargetSpec::attest` reads the legacy
   names; the normalizer emits both. A mismatch is a defect, not a value choice.
10. The original H01 report and evidence remain byte-identical. Attestation creates a new
   file; it never edits the H01 artifacts.

Proposed output paths (not created here):

```text
compiler/contracts/target/aarch64-linux-attestation.txt   # canonical report or its immutable copy
compiler/contracts/target/aarch64-linux-attestation.sha256
```

#### 2.3.4 Attestation command (NOT RUN)

There is currently no attestation CLI. A minimal integrator-owned integration test is
proposed (no source change is made by this report):

```text
compiler/tests/c02_attest.rs          # proposed, integrator-owned
CC_SILICON_PROBE_REPORT=/absolute/path/to/aarch64-linux-attestation.txt \
cargo test --manifest-path compiler/Cargo.toml --test c02_attest -- --nocapture
```

The test must:

- parse the report with `ProbeReport::parse`;
- call `ProbeReport::status`/`verify_verified` and fail on missing fields, hash mismatch,
  identity mismatch, or an unresolved required value;
- call `TargetSpec::attest`; print the attested `report_hash` and the four ABI values;
- perform the H07/integration plausibility checks that `attest` does not perform;
- not write to the bus, contract, or any persistent store.

If a CLI is preferred, `compiler/src/bin/attest.rs` is an alternative integrator-owned
path; no path or flag is frozen by this report.

#### 2.3.5 Contract update after attestation

Attestation is not complete when `TargetSpec::attest` returns `Ok`. The integrator must
also make the verified target the one consumed by codegen and record provenance:

1. Add/construct a verified target model with the attested values (for example an
   integrator-approved `TargetSpec` constructor or a compile-time attested report); do not
   leave `FrozenSchema::current()` on `aarch64_unknown_linux_gnu_unverified()`.
2. Update `compiler/src/contract.rs` `contract_version_file()` so it no longer hardcodes
   `target_verification=unverified` and `probe_substrate=... (planned, not provisioned)`.
3. Update `compiler/contracts/CONTRACT_VERSION` with the verification state and report
   hash/provenance fields; bump the contract version/hash only under integrator rules and
   after dependent tests are updated.
4. Keep the original `aarch64-linux-probe.txt` as the unverified proposal history, or
   mark it superseded; do not silently overwrite its proposal values with report values.
5. Recompute `CONTRACT_HASH`, run the freeze test, and update all dependent test fixtures.
6. Do not attest a target if any required field is `unresolved`, if `abi.*.ok` is not `1`,
   or if H07’s evidence pointers do not match the hashed evidence.
7. The H04 target path must consume the attested `TargetSpec`; it must not keep the
   hardcoded `aarch64_unknown_linux_gnu_unverified()` constructor on the `-S` path.

#### 2.3.6 Provenance and trust limits

`TargetSpec::attest` validates caller-supplied data and a hash; it does **not** prove
authenticity or physical provenance. A caller who can supply fabricated report bytes can
still create a fabricated attested report, and the report file is read at a point in time
(TOCTOU). Integration therefore must:

- retain the original H01 report/evidence and H07 annex, with hashes;
- record the exact toolchain/substrate pins and the run ID;
- require H07’s evidence pointers and the `abi.*.ok=1` gate;
- require an integrator review/sign-off separate from the hash check;
- treat CI image-digest metadata and mutable `actions/*@v4` tags as unpinned provenance
  limitations, not proof.

---

## 3. Minimal Machine and Assembly Contract (PROPOSAL)

### 3.1 Design constraints

The minimal M1 chain should avoid work that is not needed to prove the closed loop:

- use the committed IR `FunctionId` and its `Constant`/`Return` records; do not re-lower the
  AST and do not re-fold `2+3`;
- use **fixed physical registers** for the single `int` return value (no register
  allocation, no spills, no liveness);
- emit **no frame** for the leaf `main` (no prologue/epilogue, no saved registers, no SP
  adjustment);
- emit one assembly artifact through the existing `ArtifactKind::Assembly` (already
  declared); do not write ELF objects in the candidate;
- read target widths/alignment only from the attested `TargetSpec`; never from Rust host
  types or macOS arm64;
- record explicit defs/uses/clobbers, flags, memory, call effects, and stack effects even
  when they are empty for M1;
- keep the M1 op set closed; any unimplemented op is a typed `Unsupported`, never a no-op.

### 3.2 Proposed record families and fields

All names, numeric tags, enum ordinals, and `RecordRef` tags below are **proposals**. The
integrator freezes exact tags/fields and updates the snapshot/hash. No numeric tag is
asserted here. The task payloads are minimal realizations of T11’s
`AbiRequest`/`SelectRequest`/`AllocateRequest`/`EmitRequest` protocols; they do not replace
the T11 catalog or its final field definitions. All target chips are
`Capability::DeviceSpecific`, `BackendClass::Aarch64Linux`, `ChipPhase::Propagation`, and
`deterministic=true`; `matches_backend_rule` enforces that classification at registration.

| Proposed family / ID | Fields (proposal) | M1 content |
|---|---|---|
| `VRegRecord` / `VRegId` (family/arena already reserved) | `class: RegClass`, `width: RegWidth`, `fixed: Option<PhysReg>`, `origin: Option<ValueId>` | One vreg: `class=Gp`, `width=W32`, `fixed=Some(PhysReg::Gp(0))`, `origin=Some(Constant result)` |
| `AbiPlanRecord` / `AbiPlanId` | `function: FunctionId`, `return_location: ReturnLocation`, `arg_locations: Vec<ArgLocation>`, `variadic: bool`, `stack_align: u8`, `target_report_hash: [u8;32]` | `return_location=ScalarReg { vreg, width: W32 }`; empty args; `variadic=false`; `stack_align` copied from attested target; report hash binds the plan |
| `FrameRecord` / `FrameId` | `function: FunctionId`, `size: u32`, `align: u8`, `saved_gp: Vec<PhysReg>`, `saved_fp: Vec<PhysReg>`, `uses_frame_pointer: bool`, `dynamic: bool` | `size=0`; `align=attested stack_align`; empty saves; no frame pointer; not dynamic |
| `MachineInstructionRecord` / `MachineInstructionId` | `function: FunctionId`, `block: BlockId`, `ordinal: u32`, `op: MiOp`, `defs: Vec<VRegId>`, `uses: Vec<VRegId>`, `clobbers: Vec<PhysReg>`, `immediate: Option<MiImmediate>`, `effects: MiEffects`, `terminator: bool` | Two instructions: `MovImm` (defs `[v0]`, no uses, immediate width `sizeof.int * 8`, bits `5`, no effects, not terminator); `Ret` (no defs, uses `[v0]`, no immediate, terminator) |
| `TargetFunctionRecord` / `TargetFunctionId` | `function: FunctionId`, `symbol: SymbolId`, `abi_plan: AbiPlanId`, `frame: Option<FrameId>`, `first_instruction: MachineInstructionId`, `instruction_count: u32`, `assembly: ArtifactId`, `global: bool`, `function_symbol: bool` | `main`, global function, 2 instructions, assembly artifact reference, frame `Some(zero frame)` |

Supporting proposed enums:

```text
RegClass   = Gp | Fp
RegWidth   = W32 | X64 | S32 | D64 | Q128
PhysReg    = Gp(u8 /* 0..=30 */) | Sp | Fp(u8 /* 0..=31 */)
MiOp       = MovImm | Ret                    # M1 closed set; future ops require a freeze
MiImmediate= { width_bits: u8, bits: u64 }   # M1: width_bits = sizeof.int * 8, bits = 5
MiEffects  = {
               reads_flags: bool,
               writes_flags: bool,
               memory: MiMemoryEffect,       # M1: None
               reads_lr: bool,               # Ret: true
               reads_sp: bool,               # Ret: true
               writes_sp: bool,              # M1: false
               call: bool,                   # M1: false
               stack_delta: i32              # M1: 0
             }
MiMemoryEffect = None                        # M1 closed set; future Load/Store require a freeze
```

`ReturnLocation` for M1 is `ScalarReg { vreg, width }`. Indirect returns, HFAs, multiple
registers, and aggregate returns are explicitly deferred. `PhysReg::Gp(30)` is LR and
`Gp(29)` is FP; `Sp` is separate. These are AArch64 architectural register names, not
probe-derived ABI values. Do not infer a host register or ABI from these names.

New `RecordRef` variants and `ResultValue` use must follow the existing tagged-payload
pattern; no new `ResultValue` variant is needed (target tasks complete with
`Record(RecordRef::...)`). The snapshot encoder must include every new arena and field list;
the contract hash must cover the new names/fields.

### 3.3 Assembly artifact

Reuse the existing `ArtifactRecord` with `ArtifactKind::Assembly` (declared in
`compiler/src/bus.rs:77-105`). For M1:

```text
ArtifactRecord {
  kind: ArtifactKind::Assembly,
  source: None,          # map-optional artifact; no source map
  bytes: <assembly text>,
  raw_offsets: []        # map-optional rule
}
```

One assembly artifact per M1 translation unit is sufficient. The artifact is Host-persisted;
no chip writes files. The assembly text must be deterministic and free of host paths,
timestamps, usernames, and environment data.

Implementation constraint: the current commit path materializes only the map-mandatory
artifact kinds (`Normalized`/`Spliced`/`CommentFree`/`Preprocessed`; see
`compiler/src/commit.rs:1147-1165`) and rejects `Assembly` because it is map-optional. The
target contract freeze must extend artifact materialization for `ArtifactKind::Assembly`
with `source=None` and empty `raw_offsets`, plus the appropriate allowlist row; otherwise
the target chip cannot commit its output.

### 3.4 IR `Constant`/`Return` → machine path

The M1 IR produced by `IrFunctionChip` is:

```text
FunctionRecord { symbol=main, signature=int(void), entry=block0, linkage=external }
BlockRecord    { function, ordinal=0 }
ValueRecord    { ty=int }
Instruction 0  { op=Constant, block=block0, operands=[], immediate=Some(ConstId), result=Some(v0) }
Instruction 1  { op=Return,   block=block0, operands=[v0], immediate=None, result=None }
```

The target path must consume the same committed records:

1. **CG04 / `target.abi_plan`** — read `FunctionRecord`, the checked signature, and the
   attested `TargetSpec`; write `AbiPlanRecord` with `return_location = ScalarReg(W32)`,
   empty args, `variadic=false`, `stack_align` from the attested target. Bind
   `target_report_hash`. If the signature is not the M1 `int(void)` or the target is not
   attested, fail explicitly.
2. **CG10 / `target.constant`** — read the IR `Constant` and its committed `ConstRecord`;
   do not re-fold. Read `sizeof.int` from the attested target (M1 requires the attested
   value to be the C `int` width used by the fixture; the current proposal is 4 bytes,
   **UNVERIFIED**; if the attested width is not the expected M1 width, M1 must be re-scoped
   rather than silently changed). Materialize the constant into the ABI return register:
   `VRegRecord` fixed to `PhysReg::Gp(0)`/`W32`; `MachineInstructionRecord` `MovImm` with
   `immediate={width_bits: sizeof.int*8, bits: 5}`. `W32` is a register-view name; its use
   for M1 depends on the attested `int` width. The chip must lower an arbitrary in-range
   `int` constant, not special-case `5`.
3. **CG09 / `target.return`** — read the IR `Return` and its operand, plus the ABI plan.
   Because the vreg is fixed to the return register, emit `Ret` with `uses=[v0]`; if a move
   were required, this is outside the M1 fixed-register path and must be an explicit
   unsupported/extension, not a silent no-op.
4. **CG33 / `target.frame`** — write `FrameRecord { size: 0, align: stack_align,
   saved_gp: [], saved_fp: [], uses_frame_pointer: false, dynamic: false }`. A frame is not
   needed for the leaf function.
5. **CG35/CG37 / `target.emit`** — read the MI/frame/ABI records and emit the assembly
   artifact. `target.emit` is a planning placeholder for the CG35/CG37 pair; if the
   integrator keeps one task kind per catalog chip, split it into
   `target.assembly_instruction` and `target.assembly_function` without changing the
   proposed record shapes. No chip invokes `as` or `ld`.
6. **CG38 / `target.verify`** — check the invariants in §3.6; complete with the assembly
   artifact reference or a typed failure. No partial artifact is committed on failure.

The minimal chain does not need `CG06` call preservation, `CG29` register assignment,
`CG31` parallel moves, or `CG32` spill insertion because M1 uses one fixed register, no
calls, and no frame. Those chips remain deferred; if a future M1 variant uses a vreg without
a fixed physical register, the chain must add the relevant chips and evidence.

### 3.5 Proposed assembly text (UNVERIFIED)

The exact text is a proposal until the pinned GNU assembler accepts it and the H01 target is
attested. A minimal acceptable form is:

```asm
	.text
	.globl main
	.type main, %function
main:
	mov w0, #5
	ret
	.size main, .-main
	.section .note.GNU-stack,"",%progbits
```

Requirements:

- LF-terminated ASCII/UTF-8 text;
- no leading-underscore `_main`; no Darwin/Mach-O directives or sections;
- `main` is a global function symbol (`GLOBAL`, `FUNC`);
- the constant is materialized into the return register; no literal pool/data section for
  M1;
- `ret` is the last instruction in the function; no instruction after it;
- no frame/SP adjustment for the M1 leaf function; no callee-saved register writes;
- no `.cfi`, debug, `.ident`, host path, timestamp, or comment containing environment data;
- no `.arch`/`.cpu`/feature directive unless the frozen assembler pins require one; that
  choice is a contract decision, not an implicit host default.

If GNU `as` rejects `mov w0, #5` or the `.note.GNU-stack` spelling, the emitter/contract
must be revised and re-reviewed; it must not fall back to Mach-O or a different ABI.

### 3.6 CG38 verification invariants

At minimum, `target.verify` must reject:

- any vreg that is not fixed to a physical register in the M1 path;
- a return location that is not the ABI plan’s scalar `int` return register;
- a `MachineInstructionRecord` after `Ret`, or a block without exactly one `Ret`
  terminator;
- a nonzero frame size, stack adjustment, saved-register write, or SP write for the M1
  leaf function;
- a `Constant` whose committed `ConstRecord` is missing/mistyped, or a width that does not
  match the attested `int` model;
- an assembly artifact with a non-`main` global function, a missing `main` definition, or a
  Mach-O/Darwin marker;
- a target plan generated under a different `target_report_hash` than the attested target;
- any unimplemented op or unsupported type: explicit `Unsupported`, never a successful
  no-op.

### 3.7 Deferred target work (not part of M1)

- full scalar/aggregate argument classification, HFA/HVA, indirect returns, `sret`;
- variadics and the register save area in generated code;
- floating point, `long double`, `__int128`;
- calls, relocations, GOT/PLT, PIC/PIE selection, TLS, symbol visibility;
- stack frames, spills, callee-saved registers, VLA/dynamic stack;
- `x18` policy, target features, atomic/exclusive sequences;
- unwind/debug metadata;
- mixed candidate/reference ABI linking (T11 mandatory later);
- `-c` driver emission and ELF writing in-house (external assembler remains the M1 path).

---

## 4. Implementation and Integration Sequence

### 4.1 Owners and paths

| Step | Owner | Proposed paths | Exit criterion |
|---:|---|---|---|
| 0 | T01 integrator | `docs/tasks/`, `compiler/contracts/` (read-only for this plan) | This plan reviewed; target values remain unverified; no codegen opened |
| 1 | H01 | `tools/torture/probe/**`, `.github/workflows/t00-target-probes.yml`; output under `tools/torture/reports/target/<run-id>/` (proposed) | Pins recorded; probe report + `report.sha256` + evidence produced; H01 acceptance checks pass **or** explicit blocker |
| 2 | H07 | `tools/torture/h07/**` (new classifier, proposed); output `tools/torture/reports/target/<run-id>/h07/**` | Four ABI fields resolved from hashed evidence; `abi.*.ok=1`; annex/fragment hashes produced; no rerun of H01 |
| 3 | T01 integrator | `compiler/src/target.rs` (attest already), `compiler/tests/c02_attest.rs` (proposed), `compiler/contracts/target/`, `compiler/contracts/CONTRACT_VERSION`, `compiler/src/contract.rs`, `compiler/README.md` | Canonical attestation report verified; `TargetSpec::attest` succeeds; contract records verified target + report hash; provenance reviewed |
| 4 | T01 integrator | `compiler/src/ids.rs`, `bus.rs`, `task.rs`, `manifest.rs`, `records.rs`, `snapshot.rs`, `contract.rs`, `chips/mod.rs`; proposed `compiler/tests/c40_target_schema.rs` | Target task kinds, record families, machine fields, allowlist, stage/layer, snapshot/hash, and freeze tests land; new contract version/hash |
| 5 | T11 target owners | `compiler/src/chips/target/aarch64/*.rs` (new, one file per chip); tests `compiler/tests/cg_target_m1.rs` (proposed) | CG04/CG10/CG09/CG33/CG35/CG37/CG38 unit tests pass against synthetic IR/ABI/layout; no assembler invocation; no shared-file edits |
| 6 | H04 + T01 integrator | `compiler/src/bin/candidate.rs`, `compiler/src/bin/candidate/*` (only if needed), `compiler/tests/h04_candidate.rs` | `-S` writes a deterministic assembly artifact after attestation using the attested `TargetSpec` (not the unverified fixture constructor); `-c` stays closed; unverified path still refuses with exit 2 and no output |
| 7 | H01/H05 + H04 Host chain | `tools/torture/runner/**` (proposed), `tools/torture/reports/m1/<run-id>/**` | Pinned `as`, link-only driver/`ld`, and isolated runner execute the candidate object; exit 5; evidence retained |
| 8 | T13/integrator review | `compiler/tests/` (new target verification/evidence tests), `tools/torture/reports/m1/<run-id>/checks/**` | Acceptance matrix §5.2 passes with three repeats; fail-closed cases §5.4 hold; no pass rate claimed |

### 4.2 Ordering and parallelization

```text
H01 pins/substrate -> H01 probe -> H07 classification -> attestation -> target contract freeze
      -> target chips -> H04 -S -> host as/link/run -> M1 acceptance
```

- Part A symbolic work (already present) can continue independently; it must not be
  presented as target execution.
- H01/H07/attestation are the critical path for all Part B target work. Do not start target
  chips, `-S`, or execution before attestation and the target contract freeze.
- T11 target chips may be designed/unit-tested in parallel **only** against synthetic
  records after the contract freeze; they must not read a fabricated target or macOS
  values.
- No owner may edit `Cargo.toml`, `compiler/src/chips/mod.rs`, `manifest.rs`, `task.rs`,
  `bus.rs`, `ids.rs`, `records.rs`, `snapshot.rs`, or `contract.rs` except the integrator.
  Target chip owners edit only their own files under `compiler/src/chips/target/aarch64/`
  and their own test paths.
- The candidate driver owns Host orchestration only; it must not hide target logic or call
  another C compiler. The target chip must not invoke Host tools.

### 4.3 Contract-freeze contents (integrator)

The integrator’s target freeze must include, at minimum:

1. target task kinds (`target.abi_plan`, `target.constant`, `target.return`, `target.frame`,
   `target.emit`, `target.verify`; `target.emit` may split into
   `target.assembly_instruction`/`target.assembly_function` for CG35/CG37) with stage/layer
   assignment; local codes start at 16 under `TaskGroup::TARGET_CODE`. The target stage
   ordinal is an integrator decision (existing `STAGE_ASSIGNMENT` rows stop at 6;
   `STAGE_COUNT=11`);
2. new record families/IDs, field lists, `RecordRef` tags, result carriers, snapshot
   encoders, `AppendRecords` materialization, and `NORMATIVE_RULES`;
3. `machine` store fields and `STORE_OWNER_ALLOWLIST` rows for each target writer;
4. typed `VRegRecord` and the machine arena(s); no whole-store write permission;
5. exact assembly artifact producer ownership (`Artifacts`/`fragments`, `Assembly` kind) and
   extension of the append materialization rule to the map-optional `Assembly` artifact
   (`source=None`, empty `raw_offsets`);
6. exit/failure diagnostics and the fail-closed gate for target tasks;
7. hashed rule IDs for the target shape and assembly contract;
8. dependent tests and a new `CONTRACT_VERSION`/hash.

No target value is frozen by this step; it only freezes shapes and the verified-target
reference.

---

## 5. Acceptance Matrix and Fail-Closed Behavior

### 5.1 M1 fixture and commands

Exact source, 28 bytes including the final LF (no BOM, no CRLF):

```c
int main(void){return 2+3;}
```

Declared mode/options for M1: `-std=c11 -O0` (explicit; do not infer the compiler default).
The source is a hand-written fixture, not a torture case.

Planned Part A evidence command (before target attestation; still only symbolic):

```sh
<built-candidate> -std=c11 -O0 --emit-ir-snapshot --emit-trace --interpret-ir main.c
# expected evidence line: interpret: value=05 negative=false   [NOT RUN]
```

Planned target command sequence (only after H01/H07/attestation and the target freeze):

```sh
# H04 target emission
<built-candidate> -std=c11 -O0 -S -o main.s main.c

# Pinned assembler from H01 (path recorded; no PATH fallback)
"$T00_REF_AS" -o main.o main.s

# Link-only invocation: only the candidate object; no .c/.s input, no -x c
"$T00_REF_GCC" -o a.out main.o

# Inspect and run (H05 isolated directory/timeout/limits)
readelf -h main.o; readelf -S main.o; readelf -s main.o
readelf -h a.out; readelf -s a.out
./a.out; status=$?; test "$status" -eq 5
```

The exact candidate binary path, `-S` spelling, assembler flags, link invocation, and ELF
inspection tools are **illustrative/proposed** until H04/H01 freeze them. `readelf`/`file`
are not in the current H01 required-command list, so the M1 Host inventory must pin and
verify them (or a documented equivalent) before these checks are runnable. If a link-only
GCC driver is used, it must be recorded as link-only and must never receive candidate C,
AST, IR, preprocessed source, or `-x c`. A pure `ld` link with pinned CRT objects is an
alternative only if H01 pins the startup objects and the runner verifies exit propagation.

### 5.2 Acceptance matrix

Every row below is **NOT RUN** unless marked `[FACT]`. Passing M1 is not a GCC torture
result.

| ID | Check | Expected evidence | Owner | Status |
|---|---|---|---|---|
| M1-0 | Baseline recorded | `/39` hash, `target_verification=unverified`, no probe report | integrator | `[FACT]` static baseline |
| M1-1 | Exact fixture bytes | 28-byte LF-terminated source; hash recorded | H04/T13 | NOT RUN |
| M1-2 | Part A symbolic chain | Snapshot/trace hashes; `interpret: value=05 negative=false`; same `ConstId` as T08; no hand-built `5` | H04/T08/T09/T13 | NOT RUN |
| M1-3 | H01 report | `report.txt` + `report.sha256`; all 38 fields; four `unresolved`; evidence hashes; platform/pins | H01 | NOT RUN |
| M1-4 | H07 classification | Four fields resolved with evidence pointers; all `abi.*.ok=1`; annex/fragment hashes | H07 | NOT RUN |
| M1-5 | Attestation | Canonical report; `report_hash` verifies; `TargetSpec::attest` succeeds; contract records verified target/hash | T01 integrator | NOT RUN |
| M1-6 | `-S` output | Assembly artifact exists; deterministic 3×; no Mach-O/Darwin markers; `main` global/function; `ret` is last | H04/T11 | NOT RUN |
| M1-7 | Assemble | Pinned `as` produces ELF64 AArch64 relocatable object; `.text` AX; `main` defined global | H01/Host | NOT RUN |
| M1-8 | Link | ELF64 AArch64 executable; `main` defined global; object-only link command recorded; no Mach-O | H01/Host | NOT RUN |
| M1-9 | Run | Process terminates normally; exit status exactly `5`; no signal/timeout; no corrupting output | H01/H05 | NOT RUN |
| M1-10 | Determinism | Assembly byte-identical 3×; Part A snapshot/trace hashes identical 3×; ELF compared semantically only | H04/T13 | NOT RUN |
| M1-11 | Compile gate | Assembled object exists (not `-S` only) and passes ELF/symbol checks | H01/H04 | NOT RUN |
| M1-12 | Unsupported path | Non-`Constant`/`Return`, non-`int`, or unsupported target shape yields explicit diagnostic, no assembly | T11/T13 | NOT RUN |
| M1-13 | Fail-closed unverified | Before attestation, `-S`/`-c` exit 2, fail-closed message, no output file; `ensure_codegen_ready` rejects | H04/T01 | NOT RUN (static refusal exists) |
| M1-14 | Tampered/incomplete report | Hash mismatch, missing field, identity mismatch, unresolved field, or `abi.*.ok != 1` blocks attestation | T01/H07 | NOT RUN |
| M1-15 | Evidence provenance | H01 file hash, H07 annex/fragment hashes, attestation report hash, toolchain pins recorded; originals unchanged | T01/H07 | NOT RUN |
| M1-16 | No cheating | No candidate C/AST/IR/preprocessed source passed to GCC/Clang/as a compiler; no filename special-case; no external `-E` | reviewer | NOT RUN |
| M1-17 | Isolation | Per-run temp dirs; timeouts/output/resource limits; no basename collisions; H05 classification | H05 | NOT RUN |

M1 acceptance requires M1-3 through M1-17 to be evidenced, not just “the compiler built”.
The compile gate requires an object; `-S` alone is not acceptable for the T00 compile
meaning, even though M1 uses `-S` as the candidate’s first target stage.

### 5.3 Evidence bundle

Proposed bundle under `tools/torture/reports/m1/<run-id>/`:

```text
source/main.c, source/main.c.sha256
part-a/report.txt, part-a/snapshot-trace.txt
target/main.s, target/main.s.sha256
host/pins.txt, host/as.argv, host/as.version, host/as.log
target/main.o, target/main.o.readelf-h.txt, target/main.o.readelf-S.txt,
target/main.o.readelf-s.txt
host/link.argv, host/link.log
target/a.out, target/a.out.readelf-h.txt, target/a.out.readelf-s.txt
run/run-1/exit-status.txt, run/run-1/stdout.txt, run/run-1/stderr.txt
run/run-2/, run/run-3/
checks/summary.txt
provenance/h01-report.sha256, provenance/h07-annex.sha256,
provenance/attestation-hash.txt
```

Rules:

- Keep original H01/H07 evidence immutable; the M1 bundle references hashes, not copies
  that can diverge.
- Do not put wall-clock timestamps or host paths into semantic hashes (snapshot/trace/
  assembly); run metadata may record them separately.
- ELF equality across runs is **semantic** (headers/symbol/section summary + exit status),
  not byte-identical; build-id/timestamps may differ.
- A failed stage retains its evidence and does not become PASS; unsupported/other failures
  remain in the denominator of any future T00 result.

### 5.4 Fail-closed cases

| Trigger | Required behavior | Owner |
|---|---|---|
| Target unverified; `-S`/`-c` requested | Refuse before reading/writing output; exit 2; structured fail-closed message; no artifact | H04/T01 |
| Target task reached without `ensure_codegen_ready` | Typed `TargetUnverified`/target diagnostic; no MI/assembly committed | T11/T01 |
| H01 report missing/tampered/hash mismatch | H07/attestation stops; no target marked verified | H07/T01 |
| Required field missing, `unresolved`, or bad type | Attestation fails; no fallback to fixture/default | T01 |
| `abi.*.ok` missing or `0` | H07 does not classify a passing ABI; attestation blocked | H07 |
| Identity mismatch (`triple`/format/endian/model) | Attestation fails; no Darwin/macOS substitution | T01 |
| H07 classification ambiguous (especially stack alignment) | Leave `unresolved`; block attestation rather than guess | H07 |
| Target plan generated under a different report hash | Reject; re-attest before codegen | T11 |
| IR op/type outside M1 | Explicit `Unsupported`/diagnostic; no assembly, no silent truncation | T11/T13 |
| Constant width mismatch (`sizeof.int` not M1 width) | Reject/regenerate plan; do not patch `5` into the wrong width | T11 |
| Unfixed vreg / missing return location / extra instruction after `ret` | CG38 rejects; no artifact | T11 |
| Nonzero frame/SP write for M1 leaf | CG38 rejects; no artifact | T11 |
| Assembler rejects text / unsupported directive | Host records command/output; no object; M1 fails | H01/Host |
| Linked ELF wrong machine/format or Mach-O | Reject before run | H01/Host |
| Run exits nonzero, by signal, or times out | Record failure; never PASS; retain evidence | H05 |
| `-S` only output presented as compile evidence | Reject compile gate; object required | H01/H04 |
| Candidate C/AST/IR handed to GCC/Clang or external `-E` used | Reject the evidence; not candidate evidence | reviewer |
| Partial output on failure | Delete/quarantine partial artifact; atomic commit only | T11/Host |

Current behavior to preserve until the target contract freeze: `candidate` rejects `-S` and
`-c` at argument parsing with exit 2 and a “fail-closed” message. After attestation, `-S`
may open, but the target tasks and driver must still call `ensure_codegen_ready()` before
creating any output. `-c` remains closed until the `-S`/assemble/link/run chain is accepted
and the object path is reviewed.

### 5.5 Determinism and replay

- Part A snapshot/trace hashes must be identical across three runs of the same input.
- Assembly artifact bytes must be identical across three target runs (D3).
- ELF objects/executables are compared semantically (D6), not byte-for-byte.
- Selection order remains `(stage ordinal, phase priority, enqueue ordinal, TaskId)`; new
  target tasks obey next-tick enqueue and exactly-once completion.
- A target task that cannot progress must `Fail` or wait explicitly; no idle spin.

---

## 6. Unresolved Values and Blockers

### 6.1 All concrete values remain UNVERIFIED

**UNVERIFIED** includes every scalar size/alignment, `char.signed`, `wchar_t.signed`/
encoding, `longdouble.format`, pointer/size_t/ptrdiff_t widths, and all four ABI fields.
The current proposal numbers in `target.rs` and the probe fixture are not evidence. In
particular, do not state or encode any of these as fact in a report, test golden, or
assembly template until attestation:

- `abi.gp_arg_regs` — proposal exists; **UNVERIFIED**;
- `abi.fp_arg_regs` — proposal exists; **UNVERIFIED**;
- `abi.stack_align` — proposal exists; **UNVERIFIED**;
- `abi.variadic_register_save_area` — proposal exists; **UNVERIFIED**;
- `sizeof.int`/`alignof.int` — proposal exists; **UNVERIFIED** (M1 needs the attested value);
- `longdouble.format`, `wchar_t.encoding`, all other scalar values — **UNVERIFIED**.

The probe fixture’s `verified = false` is the authoritative status, not the numbers next to
it.

### 6.2 Blockers and open decisions

| ID | Blocker / open item | Effect | Owner |
|---|---|---|---|
| B1 | No Linux AArch64 substrate/VM/CI is provisioned | H01 cannot run; Part B blocked | H01/infra |
| B2 | No frozen reference GCC path/version/binary digest/sysroot/as/ld/libc pins | H01 cannot start; no reproducible report | H01 |
| B3 | No probe report or `report.sha256` exists | H07 has no input; C02 unverified | H01 |
| B4 | Four ABI fields are `unresolved` by the normalizer | Attestation blocked until H07 classification | H07 |
| B5 | `wchar_t.encoding` may be `unresolved` | `TargetSpec::attest` requires it | H01/H07/T01 |
| B6 | H07 has no implemented classifier/annex | No field-level evidence or hashes | H07 |
| B7 | `abi.*.ok` is not enforced by H01 | A valid report can hide failed ABI round-trips | H07 |
| B8 | No attestation has occurred; no verified target model | `ensure_codegen_ready` stays closed | T01 |
| B9 | Target contract not frozen: no target kinds, record families, machine fields, allowlist, stage/layer, snapshot/hash | Target chips cannot be registered | T01 |
| B10 | `VRegRecord`/machine store are reserved only | No machine IR can be committed | T01 |
| B11 | No target-code chips exist | No MI, assembly, or target verification | T11 |
| B12 | H04 has no `-S` target path | Cannot emit candidate assembly | H04 |
| B13 | Link mode/PIE/CRT/dynamic loader/symbol visibility are unresolved | Link command cannot be frozen; exit behavior uncertain | H01 |
| B14 | Runner exit-code propagation (native vs emulator) unresolved | A run result could be misclassified | H01/H05 |
| B15 | H05 isolation/timeouts/limits are absent | Untrusted execution and M1 run safety not established | H05 |
| B16 | `TargetSpec::attest` has no plausibility/consistency checks beyond field presence/identity/hash | A malformed but hash-consistent report can pass | T01/H07 |
| B17 | Authenticity/provenance/TOCTOU are not proven by `attest` | Fabricated caller-supplied reports are possible | T01/integration |
| B18 | Runner image digest and `actions/*@v4` pins are unresolved | CI provenance limited | H01/workflow |
| B19 | HVA is not exercised; `x18` policy, relocation/TLS forms, target features have no probe fixture | Cannot claim those ABI/target areas from H01 | H01/T11 |
| B20 | No mixed candidate/reference ABI linking tests | T11 mandatory ABI coverage not met | T11 |
| B21 | Full scalar/aggregate/FP/variadic/int128/long-double ABI remains unverified | Not M1; later T11 work | T11 |
| B22 | Contract docs/status still say `target_verification=unverified`; `FrozenSchema::current` uses the proposal target | Attestation must update the contract, not just local code | T01 |
| B23 | No `FunctionEnd`/IR28 target dependency is needed for M1 positive, but wider IR remains frozen/deferred | Do not infer full IR support | T09 |
| B24 | H04 doc/test-count drift (6 vs 18) | Evidence baseline inconsistency, not a target blocker | H04/docs |
| B25 | `Assembly` artifact is declared but the commit path rejects map-optional artifact kinds | Target chip cannot commit assembly until the append rule/allowlist is extended | T01 |

### 6.3 Decisions required before target implementation

1. Approve this minimal chain and the fixed-register/no-frame M1 scope.
2. Freeze target task kind names/local codes/stage/layer and record family tags/fields.
3. Freeze the machine store fields, allowlist rows, and snapshot/hash participation.
4. Decide whether H07’s `stack_align` evidence is sufficient or a probe fixture extension
   is required; do not accept an ambiguous value.
5. Freeze the exact assembly text and the pinned assembler/link invocation (including
   link-only GCC vs pure `ld`, PIE/CRT policy, and runner).
6. Freeze H04 `-S` CLI/exit behavior; keep `-c` closed for M1.
7. Freeze the attestation report path, contract update, hash scope, and provenance record.
8. Require H07 to gate on all `abi.*.ok=1` and to reject unresolved fields.

### 6.4 Blocking order

Target work is blocked in this order:

```text
B1/B2 substrate+pins -> B3 probe report -> B4/B5/B7 H07 classification -> B8/B16/B17 attestation
  -> B9/B10/B22 target contract freeze -> B11 target chips -> B12 H04 -S
  -> B13/B14 host link/runner -> B15 isolation -> M1 acceptance
```

Part A symbolic work is independent of B1–B14 and may continue; it does not authorize target
emission.

---

## 7. Non-Claims and Review Checklist

- No probe, classification, attestation, assembly, link, run, or test was performed for
  this report.
- No concrete ABI value is verified; no target is marked `Probed`; no M1 acceptance exists.
- No GCC torture rate is measured or implied; M1 is one hand-written fixture.
- No candidate C/AST/IR may be handed to GCC/Clang to generate the program; the link-only
  driver is allowed only on the candidate object and must be recorded.
- No macOS/Darwin ABI, Mach-O, or Darwin variadic model may be used as a substitute.
- Unsupported features must be explicit typed diagnostics, never silent no-ops or fabricated
  success.

Review checklist:

1. Does every concrete ABI value carry `[UNVERIFIED]`, and is no fixture value copied into
   the attestation report?
2. Are `report.sha256` (H01 file hash) and `report_hash` (canonical body hash) kept
   distinct and both recorded?
3. Does H07 cite hashed evidence and line/instruction locations for all four fields?
4. Are all eight `abi.*.ok` checks required to be `1` before H07 calls the ABI evidence
   passing?
5. Does attestation bind H01 report hash, H07 annex/fragment hashes, toolchain pins, and
   substrate run ID, while acknowledging TOCTOU/authenticity limits?
6. Are the machine records field-scoped, typed, deterministic, and free of hidden host
   state?
7. Does the target path use the same committed `ConstId`/`FunctionId` and never re-fold?
8. Are all target tools Host-only, with no chip I/O or cross-chip calls?
9. Is `-S`/`-c` fail-closed before attestation, with no output file, and is `-S` alone not
   counted as the compile gate?
10. Does the acceptance evidence include an assembled object, ELF identity checks, exact
    exit status `5`, three repeats, and retained failures?

---

## 8. References

- [AGENTS.md](../../../AGENTS.md)
- [docs/tasks/README.md](../../tasks/README.md)
- [T00_GCC_TORTURE_GATE.md](../../tasks/T00_GCC_TORTURE_GATE.md)
- [T01_COMPILER_CONTRACT.md](../../tasks/T01_COMPILER_CONTRACT.md)
- [T09_IR_LOWER_CHIPS.md](../../tasks/T09_IR_LOWER_CHIPS.md)
- [T11_TARGET_CHIPS.md](../../tasks/T11_TARGET_CHIPS.md)
- [M1_TARGET_ACCEPTANCE.md](../../tasks/M1_TARGET_ACCEPTANCE.md)
- [M1_VERTICAL_SLICE_ACCEPTANCE.md](../../tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md)
- [H04_CANDIDATE_DRIVER.md](../../tasks/H04_CANDIDATE_DRIVER.md)
- [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md)
- [PARALLEL_EXECUTION.md](../../tasks/PARALLEL_EXECUTION.md)
- [compiler/contracts/CONTRACT_VERSION](../../../compiler/contracts/CONTRACT_VERSION)
- [compiler/contracts/target/aarch64-linux-probe.txt](../../../compiler/contracts/target/aarch64-linux-probe.txt)
- [compiler/src/target.rs](../../../compiler/src/target.rs)
- [compiler/src/chips/ir_lower/ir_function.rs](../../../compiler/src/chips/ir_lower/ir_function.rs)
- [compiler/src/bin/candidate.rs](../../../compiler/src/bin/candidate.rs)
- [tools/torture/probe/README.md](../../../tools/torture/probe/README.md)
- [tools/torture/probe/run-probe.sh](../../../tools/torture/probe/run-probe.sh)
- [tools/torture/probe/normalize/normalize.py](../../../tools/torture/probe/normalize/normalize.py)
- [.github/workflows/t00-target-probes.yml](../../../.github/workflows/t00-target-probes.yml)

---

## 9. Revision Record

| Date | Change | Authority |
|---|---|---|
| 2026-11-07 | Initial A-ABI-01 Stage A planning report: H01 pins/probe commands, H07 classification rules for the four unresolved ABI fields, attestation hash/provenance handoff, minimal machine/assembly contract, Constant/Return fixed-register path, assembler/linker/runner boundary, fail-closed matrix, M1 acceptance evidence. No probe, test, or execution run; all concrete ABI values marked UNVERIFIED. | A-ABI-01 subagent, master dispatch plan Stage A |
