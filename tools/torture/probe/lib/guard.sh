#!/usr/bin/env bash
# H01 reference target-probe guards.
#
# This file is sourced by run-probe.sh and by tests/run-tests.sh. Every guard is
# a pure predicate over explicit arguments so it can be exercised without a
# compiler or a target host. Guards print one diagnostic to stderr and return
# non-zero on failure; callers must treat any non-zero return as fatal.
#
# Design rule: fail closed. An absent or malformed pin is an error, never a
# silently accepted default. No guard may invent a toolchain identity.

# shellcheck shell=bash

# Print a fatal diagnostic prefixed for the harness. Usage: guard_fail MESSAGE...
guard_fail() {
    printf 't00-probe: FATAL: %s\n' "$*" >&2
    return 1
}

# A required input must be present and non-empty.
# Usage: guard_require_value NAME VALUE
guard_require_value() {
    local name=${1:-}
    local value=${2:-}
    if [ -z "$value" ]; then
        guard_fail "required input ${name} is empty; refusing to run without an explicit pin"
        return 1
    fi
    return 0
}

# The probe must run on 64-bit Linux aarch64. macOS/Darwin can never satisfy
# this, so an accidental local run fails closed instead of recording host facts.
# Usage: guard_platform UNAME_S UNAME_M LONG_BIT
guard_platform() {
    local uname_s=${1:-}
    local uname_m=${2:-}
    local long_bit=${3:-}
    if [ "$uname_s" != "Linux" ]; then
        guard_fail "probe requires Linux, detected '${uname_s}'; macOS/Darwin values must never be recorded"
        return 1
    fi
    case "$uname_m" in
        aarch64 | arm64) ;;
        *)
            guard_fail "probe requires an aarch64 machine, detected '${uname_m}'"
            return 1
            ;;
    esac
    if [ "$long_bit" != "64" ]; then
        guard_fail "probe requires a 64-bit target, getconf LONG_BIT='${long_bit}'"
        return 1
    fi
    return 0
}

# The reference GCC must report the same machine triple the freeze pins.
# Usage: guard_gcc_triple DUMPMACHINE
guard_gcc_triple() {
    local dumpmachine=${1:-}
    case "$dumpmachine" in
        *aarch64*linux*gnu*) ;;
        *)
            guard_fail "reference GCC -dumpmachine='${dumpmachine}' is not an aarch64 GNU/Linux target"
            return 1
            ;;
    esac
    # The frozen target is little-endian LP64. Reject big-endian and ILP32
    # variants even though they contain aarch64/linux/gnu.
    case "$dumpmachine" in
        *ilp32* | *aarch64_be* | *_be-*)
            guard_fail "reference GCC -dumpmachine='${dumpmachine}' is not the frozen little-endian LP64 target"
            return 1
            ;;
    esac
    if [ -n "${T00_REF_GCC_TRIPLE:-}" ] && [ "$dumpmachine" != "$T00_REF_GCC_TRIPLE" ]; then
        guard_fail "reference GCC -dumpmachine='${dumpmachine}' does not match pinned T00_REF_GCC_TRIPLE='${T00_REF_GCC_TRIPLE}'"
        return 1
    fi
    return 0
}

# The target identity is a frozen decision, not a configurable input. Any value
# other than `aarch64-unknown-linux-gnu` must be rejected before compilation so a
# mis-set T00_TARGET_TRIPLE can never be recorded alongside aarch64 probe values.
# Usage: guard_target_triple VALUE
guard_target_triple() {
    local value=${1:-}
    if [ "$value" != "aarch64-unknown-linux-gnu" ]; then
        guard_fail "T00_TARGET_TRIPLE='${value}' is not the frozen target identity 'aarch64-unknown-linux-gnu'"
        return 1
    fi
    return 0
}

# GCC -dumpmachine and the frozen target identity must name the same target.
# `-unknown-` is an optional vendor field, so `aarch64-linux-gnu` and
# `aarch64-unknown-linux-gnu` are consistent; a different arch/os is not.
# Usage: guard_triple_consistent DUMPMACHINE FROZEN
guard_triple_consistent() {
    local dumpmachine=${1:-}
    local frozen=${2:-}
    local canonical_dump=${dumpmachine//-unknown-/-}
    local canonical_frozen=${frozen//-unknown-/-}
    if [ "$canonical_dump" != "$canonical_frozen" ]; then
        guard_fail "reference GCC -dumpmachine='${dumpmachine}' is inconsistent with frozen target '${frozen}'"
        return 1
    fi
    return 0
}

# The reference GCC version must match the pinned version exactly. A bare
# unpinned `gcc` resolved from a moving package index is not acceptable.
# Usage: guard_gcc_version ACTUAL REQUIRED
guard_gcc_version() {
    local actual=${1:-}
    local required=${2:-}
    if [ -z "$required" ]; then
        guard_fail "reference GCC version is not pinned (T00_REF_GCC_VERSION is empty)"
        return 1
    fi
    if [ "$actual" != "$required" ]; then
        guard_fail "reference GCC version mismatch: pinned='${required}' actual='${actual}'"
        return 1
    fi
    return 0
}

# A SHA-256 pin must be well formed before it is compared.
# Usage: guard_sha256_wellformed VALUE
guard_sha256_wellformed() {
    local value=${1:-}
    # Enumerate valid characters literally: bracket ranges like [0-9a-f] are
    # locale/collation dependent and can accept uppercase on some shells.
    case "$value" in
        *[!0123456789abcdef]* | '')
            guard_fail "SHA-256 pin must be 64 lowercase hex characters, got '${value}'"
            return 1
            ;;
    esac
    if [ "${#value}" -ne 64 ]; then
        guard_fail "SHA-256 pin must be 64 lowercase hex characters, got ${#value}"
        return 1
    fi
    return 0
}

# The reference GCC binary digest must match the pinned digest.
# Usage: guard_gcc_digest ACTUAL REQUIRED
guard_gcc_digest() {
    local actual=${1:-}
    local required=${2:-}
    guard_sha256_wellformed "$required" || return 1
    guard_sha256_wellformed "$actual" || return 1
    if [ "$actual" != "$required" ]; then
        guard_fail "reference GCC binary digest mismatch: expected '${required}' actual '${actual}'"
        return 1
    fi
    return 0
}

# The H01 probe is reference-only. A candidate compiler must never be in play.
# Usage: guard_reference_only
guard_reference_only() {
    if [ -n "${T00_CANDIDATE_CC:-}" ]; then
        guard_fail "T00_CANDIDATE_CC is set; H01 probes are reference-only and must not involve a candidate compiler"
        return 1
    fi
    return 0
}

# Every command in the list must be resolvable. Missing tooling is fatal: the
# probe must not silently skip version capture or the object-magic check.
# Usage: guard_require_commands CMD...
guard_require_commands() {
    local required_cmd
    for required_cmd in "$@"; do
        if ! command -v "$required_cmd" >/dev/null 2>&1; then
            guard_fail "required command '${required_cmd}' is not available"
            return 1
        fi
    done
    return 0
}
