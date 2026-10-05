#!/usr/bin/env bash
#
# Self-tests for the H01 reference target-probe harness.
#
# These tests exercise the normalizer and the fail-closed guards only. They do
# NOT run a compiler, do not require AArch64/Linux, and never produce target
# facts. They are safe to run on any developer machine and in the `static` CI
# job.
#
# Usage: bash tools/torture/probe/tests/run-tests.sh

set -uo pipefail

# Deterministic, locale-independent parsing and formatting.
export LC_ALL=C
# Do not leave Python bytecode behind when importing the normalizer.
export PYTHONDONTWRITEBYTECODE=1

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
probe_dir="$(cd "$script_dir/.." && pwd)"

# shellcheck source=../lib/guard.sh
. "$probe_dir/lib/guard.sh"

pass=0
fail=0

ok() {
    printf 'ok   - %s\n' "$1"
    pass=$((pass + 1))
}

bad() {
    printf 'FAIL - %s\n' "$1" >&2
    fail=$((fail + 1))
}

expect_success() {
    local name=$1
    shift
    if "$@" >/dev/null 2>&1; then
        ok "$name"
    else
        bad "$name"
    fi
}

expect_failure() {
    local name=$1
    shift
    if "$@" >/dev/null 2>&1; then
        bad "$name (expected non-zero exit)"
    else
        ok "$name"
    fi
}

compare_files() {
    local name=$1
    local expected=$2
    local actual=$3
    if [ ! -f "$actual" ]; then
        bad "$name (missing $actual)"
        return
    fi
    if diff -u "$expected" "$actual" >/dev/null 2>&1; then
        ok "$name"
    else
        bad "$name"
        diff -u "$expected" "$actual" >&2 || true
    fi
}

hex_of() {
    # Print a 64-character repeated-hex string; $1 is the repeated character.
    local value=""
    local i=0
    while [ "$i" -lt 64 ]; do
        value="${value}$1"
        i=$((i + 1))
    done
    printf '%s' "$value"
}

HEX_A="$(hex_of a)"
HEX_B="$(hex_of b)"
HEX_BAD_UPPER="$(hex_of A)"

# --- Static syntax checks ---------------------------------------------------------

expect_success "bash -n run-probe.sh" bash -n "$probe_dir/run-probe.sh"
expect_success "bash -n lib/guard.sh" bash -n "$probe_dir/lib/guard.sh"
expect_success "bash -n tests/run-tests.sh" bash -n "$script_dir/run-tests.sh"
expect_success "python3 syntax check normalize.py" \
    python3 -c "import ast, sys; ast.parse(open(sys.argv[1], encoding='utf-8').read())" \
    "$probe_dir/normalize/normalize.py"

# --- Guard predicates -------------------------------------------------------------

expect_success "guard_require_value accepts" guard_require_value NAME value
expect_failure "guard_require_value rejects empty" guard_require_value NAME ""
expect_failure "guard_require_value rejects absent" guard_require_value NAME

expect_success "guard_platform linux/aarch64/64" guard_platform Linux aarch64 64
expect_success "guard_platform linux/arm64/64" guard_platform Linux arm64 64
expect_failure "guard_platform rejects Darwin" guard_platform Darwin arm64 64
expect_failure "guard_platform rejects x86_64" guard_platform Linux x86_64 64
expect_failure "guard_platform rejects 32-bit" guard_platform Linux aarch64 32

expect_success "guard_gcc_triple accepts aarch64-linux-gnu" guard_gcc_triple aarch64-linux-gnu
expect_success "guard_gcc_triple accepts aarch64-unknown-linux-gnu" guard_gcc_triple aarch64-unknown-linux-gnu
expect_failure "guard_gcc_triple rejects x86_64" guard_gcc_triple x86_64-linux-gnu
expect_failure "guard_gcc_triple rejects big-endian" guard_gcc_triple aarch64_be-linux-gnu
expect_failure "guard_gcc_triple rejects ilp32" guard_gcc_triple aarch64-unknown-linux-gnu_ilp32
if T00_REF_GCC_TRIPLE=x86_64-linux-gnu guard_gcc_triple aarch64-linux-gnu >/dev/null 2>&1; then
    bad "guard_gcc_triple enforces pinned triple"
else
    ok "guard_gcc_triple enforces pinned triple"
fi

expect_success "guard_target_triple accepts frozen identity" \
    guard_target_triple aarch64-unknown-linux-gnu
expect_failure "guard_target_triple rejects other target" \
    guard_target_triple x86_64-unknown-linux-gnu
expect_failure "guard_target_triple rejects empty" guard_target_triple ""

expect_success "guard_triple_consistent accepts vendor elision" \
    guard_triple_consistent aarch64-linux-gnu aarch64-unknown-linux-gnu
expect_success "guard_triple_consistent accepts identical" \
    guard_triple_consistent aarch64-unknown-linux-gnu aarch64-unknown-linux-gnu
expect_failure "guard_triple_consistent rejects other arch" \
    guard_triple_consistent x86_64-linux-gnu aarch64-unknown-linux-gnu
expect_failure "guard_triple_consistent rejects other os" \
    guard_triple_consistent aarch64-apple-darwin aarch64-unknown-linux-gnu

expect_success "guard_gcc_version matches" guard_gcc_version 14.2.0 14.2.0
expect_failure "guard_gcc_version rejects mismatch" guard_gcc_version 14.2.1 14.2.0
expect_failure "guard_gcc_version rejects empty pin" guard_gcc_version 14.2.0 ""

expect_success "guard_sha256_wellformed accepts" guard_sha256_wellformed "$HEX_A"
expect_failure "guard_sha256_wellformed rejects short" guard_sha256_wellformed abc
expect_failure "guard_sha256_wellformed rejects uppercase" guard_sha256_wellformed "$HEX_BAD_UPPER"
expect_success "guard_gcc_digest matches" guard_gcc_digest "$HEX_A" "$HEX_A"
expect_failure "guard_gcc_digest rejects mismatch" guard_gcc_digest "$HEX_B" "$HEX_A"

unset T00_CANDIDATE_CC
expect_success "guard_reference_only when candidate unset" guard_reference_only
if T00_CANDIDATE_CC=cc guard_reference_only >/dev/null 2>&1; then
    bad "guard_reference_only rejects candidate compiler"
else
    ok "guard_reference_only rejects candidate compiler"
fi

expect_success "guard_require_commands finds sh" guard_require_commands sh python3
expect_failure "guard_require_commands rejects missing" guard_require_commands __t00_probe_missing_command__

# --- Normalizer fixture (no compiler) --------------------------------------------

norm_out="$(mktemp -d)"
if python3 "$probe_dir/normalize/normalize.py" \
    --raw "$script_dir/fixtures/raw" \
    --out "$norm_out" \
    --replace '/home/runner/work/cc-silicon/cc-silicon=<WORKDIR>' \
    --replace '/home/runner=<HOME>' >/dev/null 2>&1; then
    compare_files "fixture report.txt" \
        "$script_dir/fixtures/expected/report.txt" "$norm_out/report.txt"
    compare_files "fixture report.sha256" \
        "$script_dir/fixtures/expected/report.sha256" "$norm_out/report.sha256"
    compare_files "fixture evidence/abi-args.s" \
        "$script_dir/fixtures/expected/evidence/abi-args.s" "$norm_out/evidence/abi-args.s"
    compare_files "fixture evidence/macros.txt" \
        "$script_dir/fixtures/expected/evidence/macros.txt" "$norm_out/evidence/macros.txt"
    compare_files "fixture evidence/versions-full.txt" \
        "$script_dir/fixtures/expected/evidence/versions-full.txt" \
        "$norm_out/evidence/versions-full.txt"
    expect_success "fixture report.sha256 matches report bytes" \
        python3 -c "
import hashlib, os, sys
out = sys.argv[1]
data = open(os.path.join(out, 'report.txt'), 'rb').read()
fields = open(os.path.join(out, 'report.sha256'), encoding='utf-8').read().split()
assert fields == [hashlib.sha256(data).hexdigest(), 'report.txt'], fields
print('report hash ok')
" "$norm_out"
    expect_success "fixture derives wchar_t.encoding=utf32" \
        grep -q '^wchar_t.encoding = utf32$' "$norm_out/report.txt"
else
    bad "normalizer fixture run"
fi
rm -rf "$norm_out"

# --- Normalizer is clock/path independent (regression) ----------------------------

reg_raw_a="$(mktemp -d)"
reg_raw_b="$(mktemp -d)"
cp "$script_dir"/fixtures/raw/* "$reg_raw_a/"
cp "$script_dir"/fixtures/raw/* "$reg_raw_b/"
python3 - "$reg_raw_b/macros.txt" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding='utf-8').read()
text = text.replace('#define __DATE__ "Jan  1 2026"', '#define __DATE__ "Sep 30 2026"')
text = text.replace('#define __TIME__ "00:00:00"', '#define __TIME__ "23:59:59"')
text = text.replace(
    '#define __TIMESTAMP__ "Mon Jan  1 00:00:00 2026"',
    '#define __TIMESTAMP__ "Tue Sep 30 23:59:59 2026"',
)
text = text.replace(
    '/home/runner/work/cc-silicon/cc-silicon/foo.c',
    '/home/other/deeper/checkout/foo.c',
)
open(path, 'w', encoding='utf-8').write(text)
PY
reg_out_a="$(mktemp -d)"
reg_out_b="$(mktemp -d)"
if python3 "$probe_dir/normalize/normalize.py" --raw "$reg_raw_a" --out "$reg_out_a" >/dev/null 2>&1 \
    && python3 "$probe_dir/normalize/normalize.py" --raw "$reg_raw_b" --out "$reg_out_b" >/dev/null 2>&1; then
    if diff -q "$reg_out_a/report.txt" "$reg_out_b/report.txt" >/dev/null 2>&1 \
        && diff -q "$reg_out_a/report.sha256" "$reg_out_b/report.sha256" >/dev/null 2>&1 \
        && diff -q "$reg_out_a/evidence/macros.txt" "$reg_out_b/evidence/macros.txt" >/dev/null 2>&1; then
        ok "differing date/time/path macros produce identical report+evidence"
    else
        bad "differing date/time/path macros produce identical report+evidence"
        diff -u "$reg_out_a/report.txt" "$reg_out_b/report.txt" >&2 || true
    fi
else
    bad "clock/path regression normalizer run"
fi
rm -rf "$reg_raw_a" "$reg_raw_b" "$reg_out_a" "$reg_out_b"

# --- Normalizer derivations (pure functions, no compiler) -------------------------

expect_success "normalizer derivations" python3 -c "
import importlib.util, sys
spec = importlib.util.spec_from_file_location('t00norm', sys.argv[1])
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
assert mod.derive_endianness({'encoding.word.01020304': '04030201'}) == 'little'
assert mod.derive_endianness({'encoding.word.01020304': '01020304'}) == 'big'
assert mod.derive_endianness({'encoding.word.01020304': 'deadbeef'}) == 'unresolved'
assert mod.derive_data_model({'sizeof.int': '4', 'sizeof.long': '8', 'sizeof.pointer': '8'}) == 'lp64'
assert mod.derive_data_model({'sizeof.int': '4', 'sizeof.long': '4', 'sizeof.pointer': '8'}) == 'llp64'
assert mod.derive_data_model({'sizeof.int': '4', 'sizeof.long': '4', 'sizeof.pointer': '4'}) == 'ilp32'
assert mod.derive_data_model({'sizeof.int': '2', 'sizeof.long': '4', 'sizeof.pointer': '2'}) == 'lp32'
assert mod.derive_data_model({'sizeof.int': '4'}) == 'unresolved'
assert mod.derive_longdouble_format({'longdouble.radix': '2', 'longdouble.mant_dig': '113', 'sizeof.longdouble': '16'}) == 'ieee-binary128'
assert mod.derive_longdouble_format({'longdouble.radix': '2', 'longdouble.mant_dig': '64', 'sizeof.longdouble': '16'}) == 'x87-extended'
assert mod.derive_longdouble_format({'longdouble.radix': '10', 'longdouble.mant_dig': '113', 'sizeof.longdouble': '16'}) == 'unresolved'
assert mod.derive_object_format({'object.magic': '7f454c46'}) == 'elf'
assert mod.derive_object_format({'object.magic': 'cafebabe'}) == 'macho'
assert mod.derive_object_format({'object.magic': '0000'}) == 'unresolved'
# wchar_t.encoding is named only when the ISO/IEC 10646 macro, an actually
# compiled non-BMP wide literal, the width, and the range jointly justify it.
# An integer cast value or sizeof alone is never sufficient.
utf32_ok = {'wchar_t.stdc_iso_10646': '201706', 'wchar_t.ucn_literal_available': '1', 'wchar_t.ucn_nonbmp_single': '1', 'wchar_t.max': '2147483647', 'sizeof.wchar_t': '4'}
assert mod.derive_wchar_t_encoding(utf32_ok) == 'utf32'
assert mod.derive_wchar_t_encoding(dict(utf32_ok, **{'wchar_t.max': '0x10FFFF'})) == 'utf32'
assert mod.derive_wchar_t_encoding(dict(utf32_ok, **{'wchar_t.max': '0x10FFFE'})) == 'unresolved'
# Old insufficient cast-only facts must be rejected.
assert mod.derive_wchar_t_encoding({'wchar_t.nonbmp_single': '1', 'wchar_t.max': '2147483647', 'sizeof.wchar_t': '4'}) == 'unresolved'
assert mod.derive_wchar_t_encoding({'wchar_t.nonbmp_single': '1', 'wchar_t.ucn_nonbmp_single': '1', 'wchar_t.max': '2147483647', 'sizeof.wchar_t': '4'}) == 'unresolved'
# Each joint requirement is individually necessary.
assert mod.derive_wchar_t_encoding(dict(utf32_ok, **{'wchar_t.stdc_iso_10646': 'absent'})) == 'unresolved'
assert mod.derive_wchar_t_encoding(dict(utf32_ok, **{'wchar_t.stdc_iso_10646': '0'})) == 'unresolved'
assert mod.derive_wchar_t_encoding({k: v for k, v in utf32_ok.items() if k != 'wchar_t.stdc_iso_10646'}) == 'unresolved'
assert mod.derive_wchar_t_encoding(dict(utf32_ok, **{'wchar_t.ucn_literal_available': '0'})) == 'unresolved'
assert mod.derive_wchar_t_encoding({k: v for k, v in utf32_ok.items() if k != 'wchar_t.ucn_literal_available'}) == 'unresolved'
assert mod.derive_wchar_t_encoding(dict(utf32_ok, **{'wchar_t.ucn_nonbmp_single': '0'})) == 'unresolved'
assert mod.derive_wchar_t_encoding(dict(utf32_ok, **{'wchar_t.ucn_nonbmp_single': 'absent'})) == 'unresolved'
assert mod.derive_wchar_t_encoding(dict(utf32_ok, **{'sizeof.wchar_t': '2'})) == 'unresolved'
assert mod.derive_wchar_t_encoding(dict(utf32_ok, **{'wchar_t.max': 'absent'})) == 'unresolved'
assert mod.derive_wchar_t_encoding({}) == 'unresolved'
assert 'wchar_t.encoding' in mod.REQUIRED_FIELDS
assert len(mod.REQUIRED_FIELDS) == 38, len(mod.REQUIRED_FIELDS)
print('derivations ok')
" "$probe_dir/normalize/normalize.py"

# --- Normalizer fails closed on incomplete capture --------------------------------

incomplete_raw="$(mktemp -d)"
cp "$script_dir/fixtures/raw/meta.txt" "$incomplete_raw/meta.txt"
cp "$script_dir/fixtures/raw/scalars.txt" "$incomplete_raw/scalars.txt"
cp "$script_dir/fixtures/raw/macros.txt" "$incomplete_raw/macros.txt"
cp "$script_dir/fixtures/raw/abi-args.s" "$incomplete_raw/abi-args.s"
cp "$script_dir/fixtures/raw/versions-full.txt" "$incomplete_raw/versions-full.txt"
incomplete_parent="$(mktemp -d)"
incomplete_out="$incomplete_parent/out"
expect_failure "normalizer fails closed on missing abi.txt" \
    python3 "$probe_dir/normalize/normalize.py" \
    --raw "$incomplete_raw" --out "$incomplete_out"
if [ -e "$incomplete_out" ]; then
    bad "missing abi.txt leaves no partial output"
else
    ok "missing abi.txt leaves no partial output"
fi
rm -rf "$incomplete_raw" "$incomplete_parent"

# The last evidence file is also required; a failure must not leave earlier
# evidence blobs written (all inputs are validated before any output is made).
incomplete2_raw="$(mktemp -d)"
cp "$script_dir/fixtures/raw/meta.txt" "$incomplete2_raw/meta.txt"
cp "$script_dir/fixtures/raw/scalars.txt" "$incomplete2_raw/scalars.txt"
cp "$script_dir/fixtures/raw/abi.txt" "$incomplete2_raw/abi.txt"
cp "$script_dir/fixtures/raw/macros.txt" "$incomplete2_raw/macros.txt"
cp "$script_dir/fixtures/raw/abi-args.s" "$incomplete2_raw/abi-args.s"
incomplete2_parent="$(mktemp -d)"
incomplete2_out="$incomplete2_parent/out"
expect_failure "normalizer fails closed on missing versions-full.txt" \
    python3 "$probe_dir/normalize/normalize.py" \
    --raw "$incomplete2_raw" --out "$incomplete2_out"
if [ -e "$incomplete2_out" ]; then
    bad "missing versions-full.txt leaves no partial output"
else
    ok "missing versions-full.txt leaves no partial output"
fi
rm -rf "$incomplete2_raw" "$incomplete2_parent"

# --- Required-field floor: an unmeasured field is `unresolved`, never omitted -----

floor_raw="$(mktemp -d)"
cp "$script_dir"/fixtures/raw/* "$floor_raw/"
python3 - "$floor_raw/scalars.txt" <<'PY'
import sys
path = sys.argv[1]
kept = [line for line in open(path, encoding='utf-8').read().splitlines()
        if not line.startswith('alignof.size_t=')]
open(path, 'w', encoding='utf-8').write("\n".join(kept) + "\n")
PY
floor_out="$(mktemp -d)/out"
if python3 "$probe_dir/normalize/normalize.py" --raw "$floor_raw" --out "$floor_out" >/dev/null 2>&1 \
    && grep -q '^alignof.size_t = unresolved$' "$floor_out/report.txt"; then
    ok "unmeasured required field is emitted unresolved"
else
    bad "unmeasured required field is emitted unresolved"
fi
rm -rf "$floor_raw" "$(dirname "$floor_out")"

# --- wchar_t.encoding fails closed without measured evidence ----------------------

enc_raw="$(mktemp -d)"
cp "$script_dir"/fixtures/raw/* "$enc_raw/"
python3 - "$enc_raw/scalars.txt" <<'PY'
import sys
path = sys.argv[1]
keep = ('wchar_t.stdc_iso_10646=', 'wchar_t.ucn_literal_available=',
        'wchar_t.ucn_nonbmp_single=', 'wchar_t.max=')
kept = [line for line in open(path, encoding='utf-8').read().splitlines()
        if not line.startswith(keep)]
open(path, 'w', encoding='utf-8').write("\n".join(kept) + "\n")
PY
enc_out="$(mktemp -d)/out"
if python3 "$probe_dir/normalize/normalize.py" --raw "$enc_raw" --out "$enc_out" >/dev/null 2>&1 \
    && grep -q '^wchar_t.encoding = unresolved$' "$enc_out/report.txt"; then
    ok "wchar_t.encoding is unresolved without measured evidence"
else
    bad "wchar_t.encoding is unresolved without measured evidence"
fi
rm -rf "$enc_raw" "$(dirname "$enc_out")"

# The old cast-only evidence (nonbmp cast + width + range, no ISO macro and no
# actual wide literal) must not attest UTF-32.
enc_old_raw="$(mktemp -d)"
cp "$script_dir"/fixtures/raw/* "$enc_old_raw/"
python3 - "$enc_old_raw/scalars.txt" <<'PY'
import sys
path = sys.argv[1]
out = []
for line in open(path, encoding='utf-8').read().splitlines():
    if line.startswith(('wchar_t.stdc_iso_10646=',
                        'wchar_t.ucn_literal_available=',
                        'wchar_t.ucn_nonbmp_single=')):
        continue
    out.append(line)
    if line.startswith('wchar_t.signed='):
        out.append('wchar_t.nonbmp_single=1')
open(path, 'w', encoding='utf-8').write("\n".join(out) + "\n")
PY
enc_old_out="$(mktemp -d)/out"
if python3 "$probe_dir/normalize/normalize.py" --raw "$enc_old_raw" --out "$enc_old_out" >/dev/null 2>&1 \
    && grep -q '^wchar_t.encoding = unresolved$' "$enc_old_out/report.txt"; then
    ok "cast-only wchar_t facts do not attest utf32"
else
    bad "cast-only wchar_t facts do not attest utf32"
fi
rm -rf "$enc_old_raw" "$(dirname "$enc_old_out")"

# A two-byte wchar_t that cannot hold a non-BMP scalar must not be named.
enc2_raw="$(mktemp -d)"
cp "$script_dir"/fixtures/raw/* "$enc2_raw/"
python3 - "$enc2_raw/scalars.txt" <<'PY'
import sys
path = sys.argv[1]
out = []
for line in open(path, encoding='utf-8').read().splitlines():
    if line.startswith('wchar_t.stdc_iso_10646='):
        line = 'wchar_t.stdc_iso_10646=absent'
    elif line.startswith('wchar_t.ucn_literal_available='):
        line = 'wchar_t.ucn_literal_available=0'
    elif line.startswith('wchar_t.ucn_nonbmp_single='):
        line = 'wchar_t.ucn_nonbmp_single=absent'
    elif line.startswith('wchar_t.max='):
        line = 'wchar_t.max=65535'
    elif line.startswith('sizeof.wchar_t='):
        line = 'sizeof.wchar_t=2'
    elif line.startswith('alignof.wchar_t='):
        line = 'alignof.wchar_t=2'
    out.append(line)
open(path, 'w', encoding='utf-8').write("\n".join(out) + "\n")
PY
enc2_out="$(mktemp -d)/out"
if python3 "$probe_dir/normalize/normalize.py" --raw "$enc2_raw" --out "$enc2_out" >/dev/null 2>&1 \
    && grep -q '^wchar_t.encoding = unresolved$' "$enc2_out/report.txt"; then
    ok "two-byte wchar_t encoding stays unresolved"
else
    bad "two-byte wchar_t encoding stays unresolved"
fi
rm -rf "$enc2_raw" "$(dirname "$enc2_out")"

# --- run-probe.sh fail-closed integration (stubbed uname, no compiler) -----------

stub_bin="$(mktemp -d)"
cat >"$stub_bin/uname" <<'STUB'
#!/bin/sh
case "$1" in
    -s) echo Darwin ;;
    -m) echo arm64 ;;
    -r) echo "23.0.0" ;;
    *) echo Darwin ;;
esac
STUB
chmod +x "$stub_bin/uname"

# Platform guard must reject the stubbed Darwin host before any compile.
if PATH="$stub_bin:$PATH" \
    T00_REF_GCC=/bin/echo T00_REF_GCC_VERSION=0 \
    T00_REF_GCC_SHA256="$HEX_A" \
    bash "$probe_dir/run-probe.sh" >/dev/null 2>&1; then
    bad "run-probe.sh rejects non-Linux host"
else
    ok "run-probe.sh rejects non-Linux host"
fi

# Missing pins must fail closed.
if env -u T00_REF_GCC -u T00_REF_GCC_VERSION -u T00_REF_GCC_SHA256 \
    bash "$probe_dir/run-probe.sh" >/dev/null 2>&1; then
    bad "run-probe.sh rejects missing pins"
else
    ok "run-probe.sh rejects missing pins"
fi

# A mis-set frozen target identity must be rejected by the direct script before
# the platform guard runs, so the diagnostic names the frozen identity.
triple_err="$(mktemp)"
if T00_REF_GCC=/bin/echo T00_REF_GCC_VERSION=0 T00_REF_GCC_SHA256="$HEX_A" \
    T00_TARGET_TRIPLE=x86_64-unknown-linux-gnu \
    bash "$probe_dir/run-probe.sh" >/dev/null 2>"$triple_err"; then
    bad "run-probe.sh rejects a non-frozen target triple"
elif grep -q "frozen target identity" "$triple_err"; then
    ok "run-probe.sh rejects a non-frozen target triple before platform check"
else
    bad "run-probe.sh rejected the triple for the wrong reason"
    cat "$triple_err" >&2
fi
rm -f "$triple_err"

rm -rf "$stub_bin"

# --- Summary ----------------------------------------------------------------------

printf '\n%d passed, %d failed\n' "$pass" "$fail"
if [ "$fail" -ne 0 ]; then
    exit 1
fi
exit 0
