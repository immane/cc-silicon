# H01 reference-toolchain target probe

> **Status: UNVERIFIED artifact.** No AArch64 Linux CI/VM run has produced a
> probe report yet, so no probe artifact exists in this repository. H01 is **not
> complete**, and the T01 C02 target model is **not verified**. Everything here
> is the harness that must run on the real substrate before any target number
> may be treated as frozen. The reference GCC, sysroot, binutils, and glibc
> versions are still unestablished inputs.

This directory is an H01 host-tool area. It is **not** part of the
`cc-silicon` framework, is not built by the root or `tools/torture` Cargo
package, and never links the semantic core.

## What it is

A reference-only target probe for the frozen target identity
**`aarch64-unknown-linux-gnu` / ELF / LP64 / little-endian / AAPCS64**:

- `src/scalar-layout.c` — measures scalar size/alignment/signedness, character
  and pointer widths, endianness, floating formats (`float`, `double`,
  `long double`) and their raw encodings, and the compiler's own identity
  macros. Every value is measured at run time.
- `src/abi-args.c` — exercises AAPCS64 argument/return classification
  (scalar GP/FP, HFA/HVA, split aggregates, by-reference aggregates, and
  variadic calls). It runs behavioural self-checks and is also compiled to
  assembly as classification evidence.
- `run-probe.sh` — fails closed unless it is on 64-bit Linux aarch64, uses the
  one explicitly pinned reference GCC, then compiles/runs the probes, captures
  toolchain identity and predefined macros, normalizes the capture, and writes a
  deterministic report plus SHA-256.
- `lib/guard.sh` — pure, testable fail-closed predicates.
- `normalize/normalize.py` — deterministic normalizer (sorting, path/timestamp
  stripping, pure classifications, evidence hashing). It validates every
  required capture before creating output, writes binary LF, and drops
  clock/path-dependent macros from both the report and the hashed macro
  evidence, so two probes on different clocks/checkouts yield identical bytes.
  It also re-reads the written report and verifies it against
  `report.sha256`.
- `tests/` — normalizer and guard self-tests that need **no compiler**.
- `.github/workflows/t00-target-probes.yml` — isolated CI: a compiler-free
  self-test job, and a manual, pinned, fail-closed real-probe job.

## What it does not do

- It never runs a candidate C compiler; only `$T00_REF_GCC` compiles the probe.
- It never downloads GCC, a corpus, a container image, or any other asset.
- It never fetches or vendors the GCC torture corpus (that remains H00).
- It never claims H01 completion or C02 verification; the report is evidence
  only.
- It never records a macOS/Darwin value: the platform guard rejects anything
  that is not Linux aarch64 before a single probe is compiled.
- It never invents a toolchain digest or version; unresolved pins are required
  inputs and the harness fails closed.

## Exact host prerequisites

The real probe requires a **native 64-bit Linux aarch64** host (CI VM or
machine), **not emulation around a non-aarch64 kernel**, with:

| Requirement | Detail |
|---|---|
| OS / arch | `uname -s` = `Linux`, `uname -m` = `aarch64` (or `arm64`), `getconf LONG_BIT` = `64` |
| Reference C compiler | one explicitly pinned GCC; `T00_REF_GCC` absolute path (a PATH command is resolved and recorded) |
| GCC version pin | `T00_REF_GCC_VERSION` exactly equal to `gcc -dumpfullversion`/`-dumpversion` |
| GCC binary pin | `T00_REF_GCC_SHA256`, 64 lowercase hex, matching the binary |
| Assembler / linker | reachable via `gcc -print-prog-name=as` and `gcc -print-prog-name=ld` and executable |
| libc | glibc with `getconf GNU_LIBC_VERSION` (recorded; not asserted) |
| Tooling | `python3`, `od`, `getconf`, `uname`, `awk`, `tr`, `head`, `sed`, and `sha256sum` or `shasum` |
| C dialect | compiler default dialect is used for the probes (no `-std` override); `_Alignof` must be available (any GCC default since C11) |

If any prerequisite, pin, or command is missing, the harness exits non-zero
**before** compiling anything.

The **self-tests** need only `bash` and `python3` and run on any developer
machine, including macOS.

## Usage

### Self-tests (no compiler, safe anywhere)

```sh
bash tools/torture/probe/tests/run-tests.sh
# or
bash tools/torture/probe/run-probe.sh --self-test
```

### Real probe (native Linux aarch64, pinned GCC)

```sh
T00_REF_GCC=/usr/bin/aarch64-linux-gnu-gcc-14 \
T00_REF_GCC_VERSION=14.2.0 \
T00_REF_GCC_SHA256=<64-lowercase-hex> \
    bash tools/torture/probe/run-probe.sh

# Optional:
#   T00_REF_GCC_TRIPLE=aarch64-linux-gnu  exact -dumpmachine check
#   T00_PROBE_OUTDIR=/tmp/t00-probe       output directory (default: fresh temp dir)
```

Outputs under `T00_PROBE_OUTDIR`:

```text
raw/                    raw reference-GCC captures (meta, macros, scalars, abi, asm, versions)
normalized/report.txt   deterministic, path-stripped report
normalized/report.sha256  SHA-256 of report.txt
normalized/evidence/    normalized evidence blobs referenced by the report
```

### CI

`.github/workflows/t00-target-probes.yml`:

- `static` runs on probe/workflow `push` and `pull_request` and executes only the
  compiler-free self-tests.
- `target-probe` is **manual only** (`workflow_dispatch`) and runs on the
  hard-coded GitHub-hosted `ubuntu-24.04-arm` label. The runner label is
  deliberately **not** an input, so a dispatch cannot select an arbitrary
  self-hosted runner; adding one would be a reviewed code change.
- The job requires the reference GCC path, its exact version, its SHA-256, and
  a declared runner/toolchain digest. Empty inputs fail the job; there is no
  automatic run. The GCC path/version/SHA-256 are enforced by the script before
  compilation; the **runner/toolchain digest is recorded metadata only** — this
  workflow cannot verify a runner image digest and does not invent one, so it is
  not a proof of provenance.
- Provision the reference GCC on the runner before dispatching; the workflow
  installs nothing and fetches no image.
- The `actions/*` steps are referenced by mutable major-version tags
  (`@v4`). Pinning them to an immutable, verified commit SHA is an **unresolved
  hardening item**; no SHA has been independently verified in this repository,
  and none is fabricated. Treat the action supply chain as unpinned until then.

## Report contents and normalization

The report is a `key = value` file. Keys are sorted; checkout, home, output,
and repository path prefixes are replaced by `<WORKDIR>`, `<HOME>`, `<OUTDIR>`,
and `<REPO>`; ANSI escapes are stripped; line endings are LF. Volatile
predefined macros (`__DATE__`, `__TIME__`, `__TIMESTAMP__`, `__FILE__`,
`__FILE_NAME__`, `__BASE_FILE__`) and macro values that are absolute paths (or
contain a path placeholder) are dropped from **both** the `macro.*` report
fields **and** the hashed `evidence/macros.txt` blob, so no timestamp or
machine-specific path affects `report.txt`, `report.sha256`, or any
`evidence.*.sha256` value. Report and evidence are written as binary LF
(platform newline translation cannot change the hash), and the written report
is re-read and checked against `report.sha256` before the script succeeds.

The substrate is assumed to be a **native** aarch64 Linux host. The platform
guard records `uname`/`getconf` and rejects anything that is not Linux/aarch64/
64-bit, but it cannot detect aarch64 emulated on another kernel; the report
states this assumption (`substrate.native_aarch64_assumed`,
`substrate.emulation_detection=not-performed`) rather than claiming detection.

Measured facts (`sizeof.*`, `alignof.*`, signedness, encodings, macros,
toolchain versions) are copied through. A small set of classifications is
derived only from measurements, using documented rules:

| Field | Rule |
|---|---|
| `target.endianness` | `encoding.word.01020304` is `04030201` → `little`, `01020304` → `big` |
| `target.data_model` | integer/long/pointer widths → `lp64` / `llp64` / `ilp32` / `lp32` |
| `longdouble.format` | radix 2 + mantissa/width → `ieee-binary128` / `ieee-binary64` / `ieee-binary32` / `x87-extended` |
| `target.object_format` | first four bytes of the produced binary (ELF magic `7f454c46`) |
| `wchar_t.encoding` | `__STDC_ISO_10646__` reported, an actually compiled non-BMP wide literal `L'\U0001F600'` stores U+1F600 exactly in one `wchar_t`, width ≥ 4, **and** `wchar_t.max` ≥ U+10FFFF → `utf32`; otherwise `unresolved` (never `utf16`, which would need measured surrogate-pair evidence) |

The `wchar_t` evidence (`wchar_t.stdc_iso_10646`, `wchar_t.ucn_literal_available`,
`wchar_t.ucn_nonbmp_single`, `wchar_t.max`) is measured by
`src/scalar-layout.c` at run time. An integer cast alone is **not** accepted: the
non-BMP wide literal is compiled only under a guard requiring ISO/IEC 10646
semantics and a range covering U+1F600, so a toolchain that cannot represent it
never sees the literal and cannot fail to compile; when the evidence is missing
or inconsistent the field stays `unresolved`. No AArch64 value is assumed.

Every required probe field appears in the report: **38 fields in the current
target contract**. Keep the mirrored `REQUIRED_FIELDS` list in
`normalize/normalize.py` synchronized with the contract list (same fields, same
order). Fields that C cannot measure without inspecting emitted code are
explicitly `unresolved`:

- `abi.gp_arg_regs`
- `abi.fp_arg_regs`
- `abi.stack_align`
- `abi.variadic_register_save_area`

The emitted assembly (`gcc -S` of `abi-args.c`, hashed in
`evidence.abi-args.s.sha256`) is the classification evidence for those fields.
The harness deliberately does **not** guess numeric register counts; a later
classifier/integration review must interpret that evidence.

## Unverified requirements (until an arm64 CI artifact exists)

1. The real runner image/toolchain digest — a required declared CI input,
   **recorded as metadata only**. This harness cannot verify or enforce an
   image digest and does not claim to; it remains unresolved. The `actions/*`
   mutable-tag pins are likewise unresolved and are not fabricated.
2. The exact reference GCC version and binary digest, sysroot, binutils, and
   glibc versions — currently required inputs, not established here.
3. The concrete scalar/ABI values — measured only when the harness runs on the
   substrate.
4. The AAPCS64 argument-register counts and variadic register-save-area
   behaviour — evidence captured, not derived numerically.
5. C02 is not verified. Only the T01 integrator may set
   `VerificationState::Probed` after incorporating a real report hash.

## Safety and provenance

- No network access, no downloads, no vendoring, no container build.
- Reference-only: the probe never compiles candidate C, and `T00_CANDIDATE_CC`
  must be unset.
- The test fixtures under `tests/fixtures/` are **synthetic** hand-written
  inputs for the normalizer; they are not target evidence and no macOS value was
  probed.
- The frozen target identity matches the T01 decision; no numeric target
  constant is asserted by these artifacts.

## Ownership

This area (`tools/torture/probe/**` and
`.github/workflows/t00-target-probes.yml`) is owned by the H01 probe task. It is
separate from the H00 verifier (`tools/torture/src/**`, `tools/torture/README.md`)
and from the T01 compiler integrator (`compiler/**`). Do not overwrite either.
