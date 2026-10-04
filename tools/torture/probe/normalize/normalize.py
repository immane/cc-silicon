#!/usr/bin/env python3
"""Deterministic normalizer for the H01 reference target-probe raw capture.

The probe harness (../run-probe.sh) writes raw capture files produced by the
pinned reference GCC. This script turns them into one canonical, reproducible
report plus a SHA-256, so identical toolchain output yields byte-identical
artifacts on any machine.

Normalization rules (all deterministic):
  - `key = value` lines are sorted by key;
  - checkout, home, and other supplied path prefixes are replaced by stable
    placeholders, so no machine-specific absolute path survives;
  - ANSI escapes are stripped and line endings are normalized to LF;
  - volatile predefined macros (``__DATE__``, ``__TIME__``, ``__TIMESTAMP__``,
    ``__FILE__``, ``__FILE_NAME__``, ``__BASE_FILE__``) and macro values that
    are absolute paths are dropped from both the ``macro.*`` report fields and
    the hashed ``evidence/macros.txt`` blob, so a run on a different clock or
    checkout directory yields byte-identical report and evidence bytes;
  - the small set of pure classifications (endianness, data model, long double
    format, object format, wide-character encoding) is derived only from
    measured values, using the documented rules below; no target constant is
    asserted, and ``utf32`` is named only when the ISO/IEC 10646 macro, an
    actually compiled non-BMP wide literal, the width, and the range jointly
    justify it, otherwise ``unresolved``;
  - every T01 required probe field is present, or explicitly `unresolved`.

All required raw captures are validated before the output directory is created
or written, so a missing capture leaves no partial report or evidence behind.
Output is written as deterministic binary LF, then the written report is
re-read and its SHA-256 checked against ``report.sha256``.

This script never runs a compiler, never reads the network, and never executes
the probe. It only transforms already-collected text.
"""

import argparse
import hashlib
import os
import re
import sys

SCHEMA = "cc-silicon.torture/target-probe/v1"

# T01 REQUIRED_PROBE_FIELDS, mirrored here so the report cannot silently omit a
# field. If a field is not measured, it is emitted as `unresolved`, never as a
# guessed number.
REQUIRED_FIELDS = [
    "triple",
    "object_format",
    "endianness",
    "data_model",
    "sizeof.char",
    "alignof.char",
    "char.signed",
    "sizeof.short",
    "alignof.short",
    "sizeof.int",
    "alignof.int",
    "sizeof.long",
    "alignof.long",
    "sizeof.longlong",
    "alignof.longlong",
    "sizeof.__int128",
    "alignof.__int128",
    "sizeof.float",
    "alignof.float",
    "sizeof.double",
    "alignof.double",
    "sizeof.longdouble",
    "alignof.longdouble",
    "longdouble.format",
    "sizeof.pointer",
    "alignof.pointer",
    "sizeof.size_t",
    "alignof.size_t",
    "sizeof.ptrdiff_t",
    "alignof.ptrdiff_t",
    "sizeof.wchar_t",
    "alignof.wchar_t",
    "wchar_t.signed",
    "wchar_t.encoding",
    "abi.gp_arg_regs",
    "abi.fp_arg_regs",
    "abi.stack_align",
    "abi.variadic_register_save_area",
]

# AAPCS64 fields that C cannot measure without inspecting emitted code. They are
# deliberately left unresolved so no value is fabricated; the assembly captured
# in raw/abi-args.s is the evidence for a later classifier/integration review.
UNRESOLVED_ABI_FIELDS = [
    "abi.gp_arg_regs",
    "abi.fp_arg_regs",
    "abi.stack_align",
    "abi.variadic_register_save_area",
]

# Raw captures the report/evidence cannot be built without. Validated up front,
# before any output directory is created, so a failure is never partial.
REQUIRED_RAW_FILES = (
    "meta.txt",
    "scalars.txt",
    "abi.txt",
    "macros.txt",
    "abi-args.s",
    "versions-full.txt",
)

# Evidence blobs referenced by `evidence.<name>.sha256` in the report.
EVIDENCE_FILES = ("abi-args.s", "versions-full.txt", "macros.txt")

VOLATILE_MACROS = {
    "__DATE__",
    "__TIME__",
    "__TIMESTAMP__",
    "__FILE__",
    "__FILE_NAME__",
    "__BASE_FILE__",
}

# Stable path placeholders the caller substitutes for machine-specific prefixes.
# A macro value containing any of these is machine-specific evidence and is
# dropped, exactly like a literal absolute path.
PATH_PLACEHOLDERS = ("<HOME>", "<WORKDIR>", "<REPO>", "<OUTDIR>")

ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
HOME_RE = re.compile(r"/(?:home|Users)/[^/\s\"']+")
MACRO_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)((?:\([^)]*\))?)\s*(.*)$")


class NormalizeError(RuntimeError):
    pass


def read_text(path):
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        return handle.read()


def strip_ansi(text):
    return ANSI_RE.sub("", text)


def apply_replacements(text, replacements):
    # Longest prefixes first so nested paths resolve to the most specific token.
    for prefix, token in sorted(replacements, key=lambda item: len(item[0]), reverse=True):
        if prefix:
            text = text.replace(prefix, token)
    return HOME_RE.sub("<HOME>", text)


def canonical_lf(text):
    """Normalize line endings, strip trailing whitespace, and end in one LF."""
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    lines = [line.rstrip() for line in text.split("\n")]
    # Drop trailing blank lines but keep interior structure; output ends in LF.
    while lines and lines[-1] == "":
        lines.pop()
    if not lines:
        return ""
    return "\n".join(lines) + "\n"


def normalize_text(text, replacements):
    return canonical_lf(apply_replacements(strip_ansi(text), replacements))


def validate_inputs(raw_dir, required_files):
    """Fail before any output exists when a required capture is absent."""
    missing = [
        name for name in required_files if not os.path.isfile(os.path.join(raw_dir, name))
    ]
    if missing:
        raise NormalizeError("missing raw capture file(s): %s" % ", ".join(missing))


def parse_kv(text):
    facts = {}
    for line in text.split("\n"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        if key:
            facts[key] = value.strip()
    return facts


def macro_is_machine_specific(name, value):
    """True for clock-dependent or absolute-path macro definitions.

    These must never reach the report hash or the hashed evidence blob, or two
    identical probes on different clocks/checkouts would disagree.
    """
    if name in VOLATILE_MACROS:
        return True
    if value.startswith("/") or '"/' in value:
        return True
    return any(token in value for token in PATH_PLACEHOLDERS)


def parse_macros(text):
    macros = {}
    for line in text.split("\n"):
        line = line.rstrip()
        if not line.startswith("#define "):
            continue
        body = line[len("#define "):]
        match = MACRO_RE.match(body)
        if not match:
            continue
        name = match.group(1)
        value = (match.group(2) + match.group(3)).strip()
        if macro_is_machine_specific(name, value):
            continue
        macros[name] = value
    return macros


def filter_macro_evidence(text):
    """Drop volatile/absolute-path `#define` lines from the macro evidence blob.

    Useful macros are preserved verbatim; only clock-dependent macros and
    machine-specific path values are removed, mirroring ``parse_macros``.
    """
    kept = []
    for line in text.split("\n"):
        stripped = line.lstrip()
        if stripped.startswith("#define "):
            match = MACRO_RE.match(stripped[len("#define "):])
            if match:
                name = match.group(1)
                value = (match.group(2) + match.group(3)).strip()
                if macro_is_machine_specific(name, value):
                    continue
        kept.append(line)
    return canonical_lf("\n".join(kept))


def derive_endianness(facts):
    # scalar-layout.c stores 0x01020304 and prints memory bytes in order.
    # Little-endian memory order is 04 03 02 01; big-endian is 01 02 03 04.
    word = facts.get("encoding.word.01020304", "")
    if word == "04030201":
        return "little"
    if word == "01020304":
        return "big"
    return "unresolved"


def derive_data_model(facts):
    def size(key):
        value = facts.get(key)
        if value is None or not value.isdigit():
            return None
        return int(value)

    int_size = size("sizeof.int")
    long_size = size("sizeof.long")
    pointer_size = size("sizeof.pointer")
    if int_size is None or long_size is None or pointer_size is None:
        return "unresolved"
    if int_size == 4 and long_size == 8 and pointer_size == 8:
        return "lp64"
    if int_size == 4 and long_size == 4 and pointer_size == 8:
        return "llp64"
    if int_size == 4 and long_size == 4 and pointer_size == 4:
        return "ilp32"
    if int_size == 2 and long_size == 4 and pointer_size == 2:
        return "lp32"
    return "unresolved"


def derive_longdouble_format(facts):
    radix = facts.get("longdouble.radix")
    mant = facts.get("longdouble.mant_dig")
    size = facts.get("sizeof.longdouble")
    if radix is None or mant is None or size is None:
        return "unresolved"
    if not (radix.isdigit() and mant.isdigit() and size.isdigit()):
        return "unresolved"
    if int(radix) != 2:
        return "unresolved"
    mant = int(mant)
    size = int(size)
    if mant == 24 and size == 4:
        return "ieee-binary32"
    if mant == 53 and size == 8:
        return "ieee-binary64"
    if mant == 113 and size == 16:
        return "ieee-binary128"
    if mant == 64 and size == 16:
        return "x87-extended"
    return "unresolved"


def derive_object_format(facts):
    magic = facts.get("object.magic", "").lower()
    if magic == "7f454c46":
        return "elf"
    if magic in ("feedface", "feedfacf", "cafebabe", "cefaedfe", "cffaedfe"):
        return "macho"
    return "unresolved"


def derive_wchar_t_encoding(facts):
    """Name the wide-character encoding only when measured evidence permits it.

    ``wchar_t`` is implementation-defined, and an integer cast exercises only
    integer range, not the wide-character literal encoding. So ``utf32`` is
    claimed only when all of the following measured facts agree:

      - ``wchar_t.stdc_iso_10646`` is a positive ISO/IEC 10646 revision, i.e.
        the implementation promises Unicode code-point semantics;
      - ``wchar_t.ucn_literal_available`` is ``1`` and
        ``wchar_t.ucn_nonbmp_single`` is ``1``: an actual wide character literal
        ``L'\\U0001F600'`` was compiled under a representability guard and stores
        the non-BMP scalar U+1F600 exactly in one ``wchar_t``;
      - ``sizeof.wchar_t`` is at least 4 and ``wchar_t.max`` is at least
        U+10FFFF, so every Unicode scalar value fits.

    A bare cast value or ``sizeof`` alone is never sufficient. Anything
    unmeasured, absent, or inconsistent stays ``unresolved`` so attestation
    fails closed rather than guessing a wide-character model.

    The only recognized label emitted is ``utf32``; this is a pure function of
    the measured facts, not an AArch64 expectation.
    """
    iso = facts.get("wchar_t.stdc_iso_10646")
    if iso is None:
        return "unresolved"
    try:
        if int(iso, 0) <= 0:
            return "unresolved"
    except (TypeError, ValueError):
        return "unresolved"
    if facts.get("wchar_t.ucn_literal_available") != "1":
        return "unresolved"
    if facts.get("wchar_t.ucn_nonbmp_single") != "1":
        return "unresolved"
    size = facts.get("sizeof.wchar_t")
    if size is None or not size.isdigit() or int(size) < 4:
        return "unresolved"
    wmax = facts.get("wchar_t.max")
    if wmax is None:
        return "unresolved"
    try:
        if int(wmax, 0) < 0x10FFFF:
            return "unresolved"
    except (TypeError, ValueError):
        return "unresolved"
    return "utf32"


def sha256_hex(data):
    return hashlib.sha256(data).hexdigest()


def build_report(raw_dir, replacements):
    meta = parse_kv(normalize_text(read_text(os.path.join(raw_dir, "meta.txt")), replacements))
    scalars = parse_kv(
        normalize_text(read_text(os.path.join(raw_dir, "scalars.txt")), replacements)
    )
    abi = parse_kv(normalize_text(read_text(os.path.join(raw_dir, "abi.txt")), replacements))
    macros = parse_macros(
        normalize_text(read_text(os.path.join(raw_dir, "macros.txt")), replacements)
    )

    facts = {}
    for source in (meta, scalars, abi):
        for key, value in source.items():
            facts[key] = value

    facts["schema"] = SCHEMA
    facts["target.object_format"] = derive_object_format(facts)
    facts["target.endianness"] = derive_endianness(facts)
    facts["target.data_model"] = derive_data_model(facts)
    facts["target.gcc_triple"] = facts.get("toolchain.gcc.dumpmachine", "unresolved")
    facts["longdouble.format"] = derive_longdouble_format(facts)
    facts["wchar_t.encoding"] = derive_wchar_t_encoding(facts)

    # T01 legacy short field names (kept for direct comparison with the frozen
    # fixture) are projections of the measured/derived target facts.
    facts["object_format"] = facts["target.object_format"]
    facts["endianness"] = facts["target.endianness"]
    facts["data_model"] = facts["target.data_model"]
    if "triple" not in facts and "target.triple" in facts:
        facts["triple"] = facts["target.triple"]

    for field in UNRESOLVED_ABI_FIELDS:
        facts[field] = "unresolved"

    for field in REQUIRED_FIELDS:
        facts.setdefault(field, "unresolved")

    report_lines = [
        "# %s" % SCHEMA,
        "# status: evidence-only; C02 is not verified by this file",
        "# substrate: native aarch64 GNU/Linux assumed; emulation on another kernel is not detected",
        "# target: identity frozen; concrete values below are measured-or-unresolved, never asserted",
    ]
    for key in sorted(facts, key=lambda item: (item.lower(), item)):
        report_lines.append("%s = %s" % (key, facts[key]))
    for name in sorted(macros):
        report_lines.append("macro.%s = %s" % (name, macros[name]))

    report = "\n".join(line.rstrip() for line in report_lines) + "\n"
    return report, macros


def build_evidence(raw_dir, replacements):
    """Return {name: (bytes, sha256)} without touching the output directory."""
    evidence = {}
    for name in EVIDENCE_FILES:
        text = normalize_text(read_text(os.path.join(raw_dir, name)), replacements)
        if name == "macros.txt":
            text = filter_macro_evidence(text)
        data = text.encode("utf-8")
        evidence[name] = (data, sha256_hex(data))
    return evidence


def main(argv=None):
    parser = argparse.ArgumentParser(description="Normalize H01 target-probe captures.")
    parser.add_argument("--raw", required=True, help="directory of raw capture files")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument(
        "--replace",
        action="append",
        default=[],
        metavar="PREFIX=TOKEN",
        help="replace an absolute path prefix with a stable token (repeatable)",
    )
    args = parser.parse_args(argv)

    replacements = []
    for item in args.replace:
        if "=" not in item:
            parser.error("--replace expects PREFIX=TOKEN, got %r" % item)
        prefix, token = item.split("=", 1)
        replacements.append((prefix, token))

    # Validate and build entirely in memory first: a missing or unreadable
    # capture must fail before the output directory or any file exists.
    try:
        validate_inputs(args.raw, REQUIRED_RAW_FILES)
        evidence = build_evidence(args.raw, replacements)
        report, _macros = build_report(args.raw, replacements)
        # Fold evidence hashes into the report, then re-serialize deterministically.
        extra = [
            "evidence.%s.sha256 = %s" % (name, evidence[name][1])
            for name in sorted(evidence)
        ]
        report = report + "\n".join(extra) + "\n"
    except NormalizeError as error:
        sys.stderr.write("t00-probe-normalize: FATAL: %s\n" % error)
        return 2

    report_bytes = report.encode("utf-8")
    digest = sha256_hex(report_bytes)

    os.makedirs(args.out, exist_ok=True)
    evidence_dir = os.path.join(args.out, "evidence")
    os.makedirs(evidence_dir, exist_ok=True)
    for name in sorted(evidence):
        data, _ = evidence[name]
        with open(os.path.join(evidence_dir, name), "wb") as handle:
            handle.write(data)

    # Binary writes keep the bytes identical regardless of platform newline
    # translation; the hash is over the exact bytes on disk.
    report_path = os.path.join(args.out, "report.txt")
    with open(report_path, "wb") as handle:
        handle.write(report_bytes)
    sha_path = os.path.join(args.out, "report.sha256")
    with open(sha_path, "wb") as handle:
        handle.write(("%s  report.txt\n" % digest).encode("utf-8"))

    # Self-check: the report on disk must hash to the recorded digest.
    with open(report_path, "rb") as handle:
        written = handle.read()
    if sha256_hex(written) != digest:
        sys.stderr.write(
            "t00-probe-normalize: FATAL: written report does not match report.sha256\n"
        )
        return 2

    sys.stdout.write("normalized report: %s\n" % report_path)
    sys.stdout.write("report sha256: %s\n" % digest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
