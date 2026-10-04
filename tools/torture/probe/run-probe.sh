#!/usr/bin/env bash
#
# H01 reference-toolchain target-probe harness (reference-only).
#
# Runs the C probes (src/*.c) with an explicitly pinned reference GCC on a
# native 64-bit Linux aarch64 host, captures toolchain identity and emitted
# assembly, and writes a deterministic report plus SHA-256.
#
# Safety properties:
#   - fails closed off Linux/aarch64 (so a macOS host can never record facts);
#   - requires an explicit GCC path, version, and binary SHA-256 pin;
#   - never downloads anything, never fetches a corpus or image, never builds
#     a Docker image, and never invokes a candidate C compiler;
#   - uses exactly one compiler, $T00_REF_GCC, for every probe translation unit.
#
# Required environment (all fail closed when empty):
#   T00_REF_GCC          absolute path (or PATH command) of the pinned reference GCC
#   T00_REF_GCC_VERSION  exact `-dumpfullversion`/`-dumpversion` tuple, e.g. 14.2.0
#   T00_REF_GCC_SHA256   SHA-256 of that GCC binary (64 lowercase hex)
# Optional:
#   T00_REF_GCC_TRIPLE   exact expected `-dumpmachine` value; structural check otherwise
#   T00_TARGET_TRIPLE    frozen target identity; only the frozen value
#                        aarch64-unknown-linux-gnu is accepted (any other value
#                        is rejected before compilation)
#   T00_PROBE_OUTDIR     output directory (default: a fresh temp directory)
#
# Usage:
#   T00_REF_GCC=/usr/bin/aarch64-linux-gnu-gcc-14 \
#   T00_REF_GCC_VERSION=14.2.0 \
#   T00_REF_GCC_SHA256=<hex> \
#     bash tools/torture/probe/run-probe.sh
#
#   bash tools/torture/probe/run-probe.sh --self-test   # normalizer/guard tests, no compiler
#   bash tools/torture/probe/run-probe.sh --help

set -euo pipefail

# Deterministic, locale-independent parsing and formatting.
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=lib/guard.sh
. "$script_dir/lib/guard.sh"

usage() {
    sed -n '2,40p' "$script_dir/run-probe.sh" | sed 's/^# \{0,1\}//'
}

case "${1:-}" in
    --help | -h)
        usage
        exit 0
        ;;
    --self-test)
        exec bash "$script_dir/tests/run-tests.sh"
        ;;
    "")
        ;;
    *)
        printf 't00-probe: unknown argument: %s\n' "$1" >&2
        usage >&2
        exit 2
        ;;
esac

# --- Required, fail-closed inputs -------------------------------------------------

guard_require_value T00_REF_GCC "${T00_REF_GCC:-}"
guard_require_value T00_REF_GCC_VERSION "${T00_REF_GCC_VERSION:-}"
guard_require_value T00_REF_GCC_SHA256 "${T00_REF_GCC_SHA256:-}"
guard_reference_only

T00_TARGET_TRIPLE="${T00_TARGET_TRIPLE:-aarch64-unknown-linux-gnu}"
# The target identity is frozen; validate it before any host/compiler work so a
# mis-set triple is rejected even on a host that would otherwise fail the
# platform guard.
guard_target_triple "$T00_TARGET_TRIPLE"

# --- Host prerequisites -----------------------------------------------------------

guard_platform "$(uname -s)" "$(uname -m)" "$(getconf LONG_BIT)"
guard_require_commands python3 od getconf uname awk tr head sed

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        guard_fail "no SHA-256 tool found (need sha256sum or shasum)"
    fi
}

if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
    guard_fail "no SHA-256 tool found (need sha256sum or shasum)"
fi

# --- Resolve and pin the reference compiler --------------------------------------

if command -v "$T00_REF_GCC" >/dev/null 2>&1; then
    gcc_path="$(command -v "$T00_REF_GCC")"
else
    gcc_path="$T00_REF_GCC"
fi
[ -x "$gcc_path" ] || guard_fail "reference GCC '${T00_REF_GCC}' is not an executable"

gcc_version="$("$gcc_path" -dumpfullversion 2>/dev/null || "$gcc_path" -dumpversion)"
gcc_version="$(printf '%s' "$gcc_version" | head -n 1 | tr -d '[:space:]')"
guard_gcc_version "$gcc_version" "$T00_REF_GCC_VERSION"

gcc_sha="$(sha256_of "$gcc_path")"
guard_gcc_digest "$gcc_sha" "$T00_REF_GCC_SHA256"

gcc_triple="$("$gcc_path" -dumpmachine)"
guard_gcc_triple "$gcc_triple"
# Cross-check the compiler's own machine triple against the frozen target
# identity (vendor field optional), so a cross GCC for another target cannot be
# used while the report still claims aarch64-unknown-linux-gnu.
guard_triple_consistent "$gcc_triple" "$T00_TARGET_TRIPLE"

as_path="$("$gcc_path" -print-prog-name=as)"
ld_path="$("$gcc_path" -print-prog-name=ld)"
[ -x "$as_path" ] || guard_fail "reference assembler '${as_path}' is not executable"
[ -x "$ld_path" ] || guard_fail "reference linker '${ld_path}' is not executable"

# --- Workspace --------------------------------------------------------------------

repo_root="$(cd "$script_dir/../../.." && pwd)"
invocation_dir="$(pwd)"
outdir="${T00_PROBE_OUTDIR:-$(mktemp -d "${TMPDIR:-/tmp}/t00-probe.XXXXXX")}"
raw="$outdir/raw"
mkdir -p "$raw"

# Path placeholders keep the report free of machine-specific absolute paths.
replace_args=("--replace" "$outdir=<OUTDIR>" "--replace" "$invocation_dir=<WORKDIR>")
replace_args+=("--replace" "$repo_root=<REPO>" "--replace" "${HOME:-/nonexistent}=<HOME>")

# --- Capture toolchain identity ---------------------------------------------------

mkdir -p "$outdir/bin"

gcc_banner="$("$gcc_path" --version | head -n 1)"
gcc_dumpversion="$("$gcc_path" -dumpversion)"
gcc_sysroot="$("$gcc_path" -print-sysroot)"
gcc_libgcc="$("$gcc_path" -print-libgcc-file-name)"
gcc_include="$("$gcc_path" -print-file-name=include)"
as_banner="$("$as_path" --version | head -n 1)"
ld_banner="$("$ld_path" --version | head -n 1)"

libc_name=""
libc_version=""
if getconf GNU_LIBC_VERSION >/dev/null 2>&1; then
    libc_line="$(getconf GNU_LIBC_VERSION)"
    libc_name="$(printf '%s' "$libc_line" | awk '{print $1}')"
    libc_version="$(printf '%s' "$libc_line" | awk '{print $2}')"
fi
binutils_version="$(printf '%s' "$as_banner" | awk '{print $NF}')"

uname_s="$(uname -s)"
uname_m="$(uname -m)"
uname_r="$(uname -r)"
long_bit="$(getconf LONG_BIT)"

{
    printf 'probe.schema=cc-silicon.torture/target-probe/v1\n'
    printf 'probe.mode=reference-gcc-target-probe\n'
    printf 'probe.status=observed\n'
    printf 'probe.claim=no-c02-verification\n'
    printf 'probe.candidate_free=true\n'
    printf 'target.triple=%s\n' "$T00_TARGET_TRIPLE"
    printf 'substrate.uname_s=%s\n' "$uname_s"
    printf 'substrate.uname_m=%s\n' "$uname_m"
    printf 'substrate.uname_r=%s\n' "$uname_r"
    printf 'substrate.long_bit=%s\n' "$long_bit"
    printf 'substrate.libc=%s\n' "$libc_name"
    printf 'substrate.libc_version=%s\n' "$libc_version"
    # The platform guard confirms Linux/aarch64/64-bit but cannot prove the CPU
    # is not emulated on another kernel. State the assumption explicitly rather
    # than claiming emulation detection.
    printf 'substrate.native_aarch64_assumed=true\n'
    printf 'substrate.emulation_detection=not-performed\n'
    printf 'toolchain.gcc.path=%s\n' "$gcc_path"
    printf 'toolchain.gcc.banner=%s\n' "$gcc_banner"
    printf 'toolchain.gcc.version=%s\n' "$gcc_version"
    printf 'toolchain.gcc.dumpversion=%s\n' "$gcc_dumpversion"
    printf 'toolchain.gcc.dumpmachine=%s\n' "$gcc_triple"
    printf 'toolchain.gcc.sysroot=%s\n' "$gcc_sysroot"
    printf 'toolchain.gcc.libgcc=%s\n' "$gcc_libgcc"
    printf 'toolchain.gcc.include=%s\n' "$gcc_include"
    printf 'toolchain.gcc.sha256=%s\n' "$gcc_sha"
    printf 'toolchain.assembler.path=%s\n' "$as_path"
    printf 'toolchain.assembler.banner=%s\n' "$as_banner"
    printf 'toolchain.linker.path=%s\n' "$ld_path"
    printf 'toolchain.linker.banner=%s\n' "$ld_banner"
    printf 'toolchain.binutils.version=%s\n' "$binutils_version"
} >"$raw/meta.txt"

# --- Reference-only translations --------------------------------------------------

prefix_flags=("-ffile-prefix-map=${repo_root}=." "-fmacro-prefix-map=${repo_root}=." "-fdebug-prefix-map=${repo_root}=.")

# Predefined macros from the compiler default dialect (no -std override).
"$gcc_path" -dM -E -x c - </dev/null >"$raw/macros.txt" 2>"$raw/macros.build.txt"

# Scalar layout/encoding probe: measured at run time.
"$gcc_path" -O0 "${prefix_flags[@]}" -o "$outdir/bin/scalar-layout" "$script_dir/src/scalar-layout.c" \
    2>"$raw/scalar-layout.build.txt"
"$outdir/bin/scalar-layout" >"$raw/scalars.txt"

# ABI argument-classification exercise: run for behaviour, -S for classification evidence.
"$gcc_path" -O0 "${prefix_flags[@]}" -o "$outdir/bin/abi-args" "$script_dir/src/abi-args.c" \
    2>"$raw/abi-args.build.txt"
"$outdir/bin/abi-args" >"$raw/abi.txt"
"$gcc_path" -O0 -S "${prefix_flags[@]}" -o "$raw/abi-args.s" "$script_dir/src/abi-args.c" \
    2>"$raw/abi-args.s.build.txt"

# Object format is measured from the produced binary, not assumed.
magic="$(od -An -tx1 -N4 "$outdir/bin/scalar-layout" | tr -d ' \n')"
printf 'object.magic=%s\n' "$magic" >>"$raw/meta.txt"

# --- Full version evidence --------------------------------------------------------

{
    printf 'uname_s: %s\n' "$uname_s"
    printf 'uname_m: %s\n' "$uname_m"
    printf 'uname_r: %s\n' "$uname_r"
    printf 'getconf_LONG_BIT: %s\n' "$long_bit"
    printf 'getconf_GNU_LIBC_VERSION: %s\n' "$libc_name $libc_version"
    printf 'gcc_path: %s\n' "$gcc_path"
    printf 'gcc_banner: %s\n' "$gcc_banner"
    printf 'gcc_dumpfullversion: %s\n' "$gcc_version"
    printf 'gcc_dumpversion: %s\n' "$gcc_dumpversion"
    printf 'gcc_dumpmachine: %s\n' "$gcc_triple"
    printf 'gcc_print_sysroot: %s\n' "$gcc_sysroot"
    printf 'gcc_print_libgcc_file_name: %s\n' "$gcc_libgcc"
    printf 'gcc_print_file_name_include: %s\n' "$gcc_include"
    printf 'assembler_path: %s\n' "$as_path"
    printf 'assembler_banner: %s\n' "$as_banner"
    printf 'linker_path: %s\n' "$ld_path"
    printf 'linker_banner: %s\n' "$ld_banner"
    printf 'gcc_sha256: %s\n' "$gcc_sha"
} >"$raw/versions-full.txt"

# --- Deterministic normalization --------------------------------------------------

python3 "$script_dir/normalize/normalize.py" \
    --raw "$raw" \
    --out "$outdir/normalized" \
    "${replace_args[@]}"

printf '\nH01 probe capture complete (reference-only; not a C02 verification).\n'
printf 'raw:        %s\n' "$raw"
printf 'report:     %s\n' "$outdir/normalized/report.txt"
printf 'report sha: %s\n' "$outdir/normalized/report.sha256"
