#!/usr/bin/env python3
"""H07 ABI classifier for the H01 reference probe.

Reads the normalized H01 report plus the emitted `abi-args.s` assembly,
classifies the four ABI fields H01 deliberately leaves `unresolved`, and
writes a canonical report with `verified=true` and a `report_hash` that
`cc_silicon_compiler::target::TargetSpec::attest` can verify.

This is a host/reference classifier, not a candidate compiler. It never
executes the probe binary and never invents a value: every field must be
derived from measured report fields and assembly evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import math
import re
import sys
from pathlib import Path

REQUIRED_MEASURED = (
    "abi.gp_ten.ok",
    "abi.fp_ten.ok",
    "abi.variadic.ok",
    "target.triple",
    "target.object_format",
    "target.endianness",
    "target.data_model",
)


def fail(message: str) -> "NoReturn":
    print(f"h07-classify: {message}", file=sys.stderr)
    raise SystemExit(1)


def parse_report(path: Path) -> dict[str, str]:
    fields: dict[str, str] = {}
    for line_number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            fail(f"{path}:{line_number}: expected key = value")
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip()
        if not key:
            fail(f"{path}:{line_number}: empty key")
        if key in fields:
            fail(f"{path}:{line_number}: duplicate key {key}")
        fields[key] = value
    return fields


def function_body(assembly: str, name: str) -> str:
    lines = assembly.splitlines()
    start = None
    for index, line in enumerate(lines):
        if line.strip() == f"{name}:":
            start = index
            break
    if start is None:
        fail(f"assembly has no `{name}:` label")
    end = None
    marker = f".size\t{name},"
    marker_alt = f".size {name},"
    for index in range(start + 1, len(lines)):
        stripped = lines[index].strip()
        if stripped.startswith(marker) or stripped.startswith(marker_alt):
            end = index
            break
    if end is None:
        fail(f"assembly has no `.size {name}` marker")
    return "\n".join(lines[start:end])


def saved_gp_registers(body: str, prefix: str) -> set[int]:
    return {
        int(match.group(1))
        for match in re.finditer(
            rf"\bstr\s+{prefix}(\d+)\s*,\s*\[sp(?:,\s*\d+)?\]", body
        )
    }


def classify_gp(assembly: str) -> int:
    body = function_body(assembly, "abi_gp_ten")
    saved = saved_gp_registers(body, "x")
    if saved != set(range(8)):
        fail(f"abi_gp_ten does not save exactly x0..x7 (saved {sorted(saved)})")
    return 8


def classify_fp(assembly: str) -> int:
    body = function_body(assembly, "abi_fp_ten")
    saved = saved_gp_registers(body, "d")
    if saved != set(range(8)):
        fail(f"abi_fp_ten does not save exactly d0..d7 (saved {sorted(saved)})")
    return 8


def classify_stack_align(assembly: str) -> int:
    amounts: set[int] = set()
    amounts.update(
        int(match.group(1))
        for match in re.finditer(r"\bsub\s+sp,\s*sp,\s*#(\d+)", assembly)
    )
    amounts.update(
        int(match.group(1))
        for match in re.finditer(r"\bstr\s+x\d+,\s*\[sp,\s*-(\d+)\]!", assembly)
    )
    if not amounts:
        fail("assembly has no stack-adjustment evidence")
    divisor = 0
    for amount in sorted(amounts):
        divisor = math.gcd(divisor, amount)
    if divisor != 16:
        fail(f"stack adjustments imply alignment {divisor}, not 16: {sorted(amounts)}")
    return 16


def classify_variadic_register_save_area(assembly: str) -> bool:
    body = function_body(assembly, "abi_variadic_sum")
    gp = saved_gp_registers(body, "x")
    fp = saved_gp_registers(body, "q")
    if not set(range(1, 8)).issubset(gp):
        fail(f"variadic prologue does not save x1..x7 (saved {sorted(gp)})")
    if not set(range(8)).issubset(fp):
        fail(f"variadic prologue does not save q0..q7 (saved {sorted(fp)})")
    return True


def canonical_hash(fields: dict[str, str]) -> str:
    body = "".join(
        f"{key}={value}\n"
        for key, value in sorted(fields.items())
        if key != "report_hash"
    )
    return hashlib.sha256(body.encode("utf-8")).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--assembly", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--report-sha", type=Path)
    args = parser.parse_args()

    fields = parse_report(args.report)
    missing = [field for field in REQUIRED_MEASURED if field not in fields]
    if missing:
        fail(f"report misses required fields: {', '.join(missing)}")
    for field in ("abi.gp_ten.ok", "abi.fp_ten.ok", "abi.variadic.ok"):
        if fields.get(field) != "1":
            fail(f"report self-check `{field}` is not 1")
    if fields.get("probe.candidate_free") != "true":
        fail("report is not marked candidate-free")
    if fields.get("target.triple") != "aarch64-unknown-linux-gnu":
        fail(f"unexpected target.triple {fields.get('target.triple')}")

    assembly = args.assembly.read_text(encoding="utf-8")
    if args.report_sha is not None:
        declared = args.report_sha.read_text(encoding="utf-8").split()[0]
        actual = hashlib.sha256(args.report.read_bytes()).hexdigest()
        if declared != actual:
            fail(f"report.sha256 mismatch: declared {declared}, computed {actual}")

    fields["abi.gp_arg_regs"] = str(classify_gp(assembly))
    fields["abi.fp_arg_regs"] = str(classify_fp(assembly))
    fields["abi.stack_align"] = str(classify_stack_align(assembly))
    fields["abi.variadic_register_save_area"] = (
        "true" if classify_variadic_register_save_area(assembly) else "false"
    )

    aliases = {
        "triple": "target.triple",
        "object_format": "target.object_format",
        "endianness": "target.endianness",
        "data_model": "target.data_model",
    }
    for bare, source in aliases.items():
        if source not in fields:
            fail(f"report misses `{source}`")
        fields[bare] = fields[source]

    fields["probe.classifier"] = "h07-abi-classify/v1"
    fields["probe.source_report_sha256"] = hashlib.sha256(
        args.report.read_bytes()
    ).hexdigest()
    fields["probe.assembly_sha256"] = hashlib.sha256(args.assembly.read_bytes()).hexdigest()
    fields["verified"] = "true"
    fields.pop("report_hash", None)
    fields["report_hash"] = canonical_hash(fields)

    lines = [
        "# H07 ABI classification of the H01 reference probe.",
        "# Values are derived from the measured report and abi-args.s evidence.",
        "# attest validates this data/hash; it does not prove physical provenance.",
    ]
    lines.extend(f"{key} = {fields[key]}" for key in sorted(fields))
    args.out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"h07-classify: wrote {args.out}")
    print(f"h07-classify: report_hash={fields['report_hash']}")
    print(
        "h07-classify: "
        f"gp={fields['abi.gp_arg_regs']} fp={fields['abi.fp_arg_regs']} "
        f"stack={fields['abi.stack_align']} "
        f"variadic_save={fields['abi.variadic_register_save_area']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
