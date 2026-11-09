# Target Probe Run — Containerized AArch64 Linux (2026-11-09)

| Field | Value |
|---|---|
| Status | **PROBE OBSERVED / H07 CLASSIFIED / ATTESTATION TEST PASSES**; not an official T00 baseline or bare-metal CI claim |
| Harness | `tools/torture/probe/run-probe.sh` (H01) |
| Classifier | `tools/torture/probe/classify.py` (H07, new in this run) |
| Attestation test | `compiler/tests/c48_target_probe.rs` |
| Report fixture | `compiler/tests/fixtures/target-probe/aarch64-debian12-gcc12.2.0-attested.txt` |

## Substrate

| Field | Value |
|---|---|
| Host | macOS 26.3.1, arm64 |
| Runtime | Docker 29.4.3 (`linux/arm64`) |
| Image | `debian:bookworm` digest `sha256:268fb24c39bca92d326dd83ea3abebe4a3f7751c804ffef23bb6d857dd343ff8` |
| Kernel | `6.12.76-linuxkit` |
| `uname -s` / `uname -m` | `Linux` / `aarch64` |
| `LONG_BIT` | 64 |
| libc | glibc 2.36 |
| Reference GCC | `/usr/bin/gcc`, Debian 12.2.0-14+deb12u1 |
| GCC SHA-256 | `dc53dbc5a583d03ae8ed6272ca9afc0f58873f9f9b86dd7d448b17fb3f88a8d0` |
| binutils | GNU as/ld 2.40 |

The harness records `substrate.native_aarch64_assumed=true` and
`substrate.emulation_detection=not-performed`: the container runs an AArch64
Linux userspace on an arm64 host, but this is a VM/container substrate, not a
bare-metal CI runner. The official T00 substrate and reference baseline remain
unfrozen.

## Commands

```sh
docker run --rm --platform linux/arm64 \
  -v "$PWD":/repo -v <out>:/out -w /repo debian:bookworm bash -lc '
    apt-get update -qq
    apt-get install -y -qq gcc binutils python3 coreutils
    gcc_path=$(command -v gcc)
    gcc_version=$($gcc_path -dumpfullversion || $gcc_path -dumpversion)
    gcc_sha=$(sha256sum "$gcc_path" | awk "{print \$1}")
    T00_REF_GCC="$gcc_path" \
    T00_REF_GCC_VERSION="$gcc_version" \
    T00_REF_GCC_SHA256="$gcc_sha" \
    T00_PROBE_OUTDIR=/out \
      bash tools/torture/probe/run-probe.sh
  '

python3 tools/torture/probe/classify.py \
  --report <out>/normalized/report.txt \
  --assembly <out>/normalized/evidence/abi-args.s \
  --report-sha <out>/normalized/report.sha256 \
  --out <out>/normalized/attested-report.txt
```

## Results

H01 normalized report SHA-256:

```text
6988aae1ffc3e179adcefa757b9168462599d712d776cdde44f77db27f38e7e9
```

H07 classified/attested report SHA-256:

```text
57eadb28155756c2f95ba49606851b642f7825bda0206993ffd7fb5a800c4f90
```

H07 derived ABI fields from `abi-args.s`:

| Field | Value | Evidence |
|---|---|---|
| `abi.gp_arg_regs` | 8 | `abi_gp_ten` saves exactly `x0..x7`; stack arguments follow |
| `abi.fp_arg_regs` | 8 | `abi_fp_ten` saves exactly `d0..d7`; stack arguments follow |
| `abi.stack_align` | 16 | stack adjustments in the emitted assembly have gcd 16 |
| `abi.variadic_register_save_area` | true | `abi_variadic_sum` saves `x1..x7` and `q0..q7` |

Measured scalar facts include LP64, little-endian, ELF, `sizeof(long)=8`,
`sizeof(pointer)=8`, `sizeof(long double)=16`, `longdouble.format=ieee-binary128`,
and `wchar_t.encoding=utf32`.

`compiler/tests/c48_target_probe.rs` parses the attested report,
`TargetSpec::attest` verifies its hash, and the resulting target reports
`verification().is_verified()`. A tampered report is rejected.

## Harness fix

`run-probe.sh` previously required `gcc -print-prog-name=as`/`ld` to print an
absolute path. Debian's native GCC prints bare `as`/`ld`. The harness now
resolves bare command names with `command -v` before the executable check and
continues to record the resolved binary. This is a host-portability fix, not a
target-value change.

## Non-claims

- Not an official T00 reference-baseline freeze; GCC 12.2.0 in Debian 12 is a
  pinned probe toolchain, not the final torture reference compiler.
- Not a bare-metal or official CI-substrate attestation.
- Not a candidate compiler execution, target codegen, ABI mixed-linking test, or
  GCC torture rate.
- `TargetSpec::attest` validates report data/hash; it does not prove physical
  provenance. The global `CONTRACT_VERSION` remains
  `target_verification=unverified` until the official baseline/substrate is
  accepted as a contract state.
