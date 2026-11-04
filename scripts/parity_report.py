#!/usr/bin/env python3
"""Validate the parity inventory and generate the dashboard region.

The validator is intentionally strict about claims: a row may only be counted as
VERIFIED when it names existing evidence and test references. The generated
region in docs/parity/README.md is byte-compared in CI so percentages cannot be
edited by hand.
"""

import argparse
import csv
import json
import re
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INVENTORY = ROOT / "docs" / "parity" / "inventory.csv"
LOCK = ROOT / "docs" / "parity" / "upstream-lock.json"
CHECKS = ROOT / "docs" / "parity" / "last-checks.json"
DECISIONS = ROOT / "docs" / "parity" / "decisions.md"
DASHBOARD = ROOT / "docs" / "parity" / "README.md"

BEGIN = "<!-- BEGIN GENERATED PARITY DASHBOARD -->"
END = "<!-- END GENERATED PARITY DASHBOARD -->"

COLUMNS = [
    "id", "domain", "behavior", "baseline_sha", "upstream_refs",
    "dependency_refs", "stage", "adapter_family", "priority", "weight",
    "status", "verified_cases", "total_cases", "test_refs", "evidence_ref",
    "owner", "upstream_changed", "notes",
]
STATUSES = {
    "DISCOVERED", "RESEARCHED", "IN_PROGRESS", "IMPLEMENTED_UNVERIFIED",
    "PARTIAL", "VERIFIED", "BLOCKED", "REGRESSION", "OUT_OF_SCOPE_APPROVED",
}
PRIORITIES = {"P0", "P1", "P2"}
FAMILIES = {"shared", "zstack", "ember", "ezsp", "deconz", "zigate", "zboss", "zoh"}
SHA = re.compile(r"^[0-9a-f]{40}$")
HEX_ID = re.compile(r"^[A-Z0-9]+(?:-[A-Z0-9]+)+$")


def load_inventory(path=INVENTORY):
    with open(path, newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames != COLUMNS:
            raise ValueError(f"inventory columns differ from the required schema: {reader.fieldnames}")
        return list(reader)


def load_json(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def iso_utc(value):
    if not isinstance(value, str):
        return False
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return False
    return parsed.tzinfo is not None


def parse_int(value):
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def validate(rows, lock, checks, decisions_text, root=ROOT):
    """Return a list of human-readable validation errors."""
    errors = []
    seen = set()

    baseline = lock.get("baseline", {})
    allowed_shas = {
        baseline.get("zigbee2mqtt", {}).get("commit"),
        baseline.get("zigbee-herdsman", {}).get("commit"),
        baseline.get("zigbee-herdsman-converters", {}).get("commit"),
    }
    allowed_shas.discard(None)
    if len(allowed_shas) != 3:
        errors.append("lock: baseline must pin app, herdsman and converters commits")

    if lock.get("freshness_status") not in {"CURRENT", "DRIFT_DETECTED", "UNVERIFIED"}:
        errors.append("lock: invalid freshness_status")
    if not iso_utc(lock.get("last_checked_utc")):
        errors.append("lock: last_checked_utc must be timezone-aware ISO-8601")
    if not iso_utc(lock.get("last_successful_fetch_utc")):
        errors.append("lock: last_successful_fetch_utc must be timezone-aware ISO-8601")
    for repo, head in lock.get("latest_heads", {}).items():
        if not isinstance(head, str) or not SHA.match(head):
            errors.append(f"lock: latest_heads[{repo}] is not a full commit SHA")

    if not iso_utc(checks.get("captured_utc")):
        errors.append("checks: captured_utc must be timezone-aware ISO-8601")
    commands = checks.get("commands")
    if not isinstance(commands, list) or not commands:
        errors.append("checks: commands must be a non-empty list")
    else:
        for index, item in enumerate(commands):
            if not isinstance(item.get("command"), str) or not item["command"]:
                errors.append(f"checks: commands[{index}] has no command")
            if parse_int(item.get("exit")) is None:
                errors.append(f"checks: commands[{index}] has no integer exit")

    for line, row in enumerate(rows, start=2):
        ident = row.get("id") or f"line {line}"
        where = f"{ident} (line {line})"
        if not row.get("id") or not HEX_ID.match(row["id"]):
            errors.append(f"{where}: id must be an uppercase hyphenated identifier")
        elif row["id"] in seen:
            errors.append(f"{where}: duplicate id")
        seen.add(row.get("id"))

        if row.get("status") not in STATUSES:
            errors.append(f"{where}: invalid status {row.get('status')!r}")
        if row.get("priority") not in PRIORITIES:
            errors.append(f"{where}: invalid priority {row.get('priority')!r}")
        if row.get("adapter_family") not in FAMILIES:
            errors.append(f"{where}: invalid adapter_family {row.get('adapter_family')!r}")
        stage = parse_int(row.get("stage"))
        if stage is None or stage < 1:
            errors.append(f"{where}: stage must be a positive integer")
        weight = parse_int(row.get("weight"))
        if weight is None or weight < 1:
            errors.append(f"{where}: weight must be a positive integer")
        if row.get("upstream_changed") not in {"yes", "no"}:
            errors.append(f"{where}: upstream_changed must be yes or no")
        if not row.get("owner"):
            errors.append(f"{where}: owner must be explicit (use 'unassigned')")
        if not row.get("behavior"):
            errors.append(f"{where}: behavior description is empty")
        if not row.get("upstream_refs"):
            errors.append(f"{where}: upstream_refs is empty")

        sha = row.get("baseline_sha", "")
        if not SHA.match(sha):
            errors.append(f"{where}: baseline_sha must be a full 40-character SHA")
        elif sha not in allowed_shas:
            errors.append(f"{where}: baseline_sha is not a pinned baseline commit")

        verified = parse_int(row.get("verified_cases"))
        total = parse_int(row.get("total_cases"))
        if verified is None or total is None or verified < 0 or total < 0:
            errors.append(f"{where}: case counters must be non-negative integers")
            verified = total = 0
        elif verified > total:
            errors.append(f"{where}: verified_cases exceeds total_cases")

        evidence = row.get("evidence_ref", "")
        if evidence and not (root / evidence).is_file():
            errors.append(f"{where}: evidence_ref does not exist: {evidence}")

        status = row.get("status")
        if status == "VERIFIED":
            if total <= 0:
                errors.append(f"{where}: VERIFIED requires total_cases > 0")
            if verified != total:
                errors.append(f"{where}: VERIFIED requires verified_cases == total_cases")
            if not evidence:
                errors.append(f"{where}: VERIFIED requires evidence_ref")
            if not row.get("test_refs"):
                errors.append(f"{where}: VERIFIED requires test_refs")
            if "HARDWARE_REQUIRED" in row.get("notes", "") or "HARDWARE_PENDING" in row.get("notes", ""):
                errors.append(f"{where}: hardware-dependent row cannot be VERIFIED without HIL evidence")
        if status == "RESEARCHED" and total <= 0:
            errors.append(f"{where}: RESEARCHED requires enumerated total_cases")
        if status == "OUT_OF_SCOPE_APPROVED" and row["id"] not in decisions_text:
            errors.append(f"{where}: OUT_OF_SCOPE_APPROVED requires an approved decision naming the id")

    return errors


def pct(numerator, denominator):
    return "N/A" if denominator == 0 else f"{100.0 * numerator / denominator:.2f}%"


def metrics(rows):
    eligible = [row for row in rows if row["status"] != "OUT_OF_SCOPE_APPROVED"]
    stage1 = [row for row in eligible if row["stage"] == "1"]
    result = {}
    for name, subset in (("stage1", stage1), ("global", eligible)):
        verified = [row for row in subset if row["status"] == "VERIFIED"]
        weight = sum(int(row["weight"]) for row in subset)
        verified_weight = sum(int(row["weight"]) for row in verified)
        total_cases = sum(int(row["total_cases"]) for row in subset)
        verified_cases = sum(int(row["verified_cases"]) for row in subset)
        result[name] = {
            "rows": len(subset),
            "verified": len(verified),
            "percent": pct(len(verified), len(subset)),
            "weight": weight,
            "verified_weight": verified_weight,
            "weighted_percent": pct(verified_weight, weight),
            "scenarios": total_cases,
            "verified_scenarios": verified_cases,
            "scenario_percent": pct(verified_cases, total_cases),
            "unmeasured": sum(1 for row in subset if int(row["total_cases"]) == 0),
            "p0_gaps": sorted(row["id"] for row in subset if row["priority"] == "P0" and row["status"] != "VERIFIED"),
        }
    return result


def _short(sha):
    return sha[:12] if isinstance(sha, str) else "UNKNOWN"


def render(rows, lock, checks):
    stats = metrics(rows)
    baseline = lock["baseline"]
    app = baseline["zigbee2mqtt"]
    herdsman = baseline["zigbee-herdsman"]
    converters = baseline["zigbee-herdsman-converters"]
    heads = lock.get("latest_heads", {})
    statuses = {}
    for row in rows:
        statuses[row["status"]] = statuses.get(row["status"], 0) + 1
    excluded = [row["id"] for row in rows if row["status"] == "OUT_OF_SCOPE_APPROVED"]
    families = {}
    for row in rows:
        counts = families.setdefault(row["adapter_family"], [0, 0])
        counts[0] += 1
        if row["status"] == "VERIFIED":
            counts[1] += 1
    family_text = ", ".join(
        f"{name} {value[1]}/{value[0]}" for name, value in sorted(families.items())
    )
    p0 = stats["global"]["p0_gaps"]
    p0_text = ", ".join(p0[:25]) + (f" (+{len(p0) - 25} more)" if len(p0) > 25 else "")
    checks_text = "; ".join(
        f"`{item['command']}` exit {item['exit']}" for item in checks["commands"]
    )
    lines = [
        BEGIN,
        "Compatibility baseline: zigbee2mqtt "
        f"`{app['commit']}` ({app.get('version', 'UNKNOWN')}) + herdsman "
        f"`{herdsman['commit']}` ({herdsman.get('resolved_version', 'UNKNOWN')}) + converters "
        f"`{converters['commit']}` ({converters.get('resolved_version', 'UNKNOWN')}).",
        "Frontend baseline: **BLOCKED** — the app resolves Windfront "
        f"{lock.get('app_resolved_frontends', {}).get('zigbee2mqtt-windfront', {}).get('resolved_version', 'UNKNOWN')}"
        "; the historical lock pins 2.15.1. See upstream-lock.json.",
        "Latest upstream: "
        f"app `{_short(heads.get('zigbee2mqtt'))}`, herdsman `{_short(heads.get('zigbee-herdsman'))}`, "
        f"converters `{_short(heads.get('zigbee-herdsman-converters'))}`, "
        f"Windfront `{_short(heads.get('zigbee2mqtt-windfront'))}`, "
        f"documentation `{_short(heads.get('zigbee2mqtt.io'))}` "
        f"(checked {lock['last_checked_utc']}).",
        f"Freshness: **{lock['freshness_status']}**",
        f"Discovered: {len(rows)} eligible behaviors; {stats['global']['unmeasured']} unmeasured rows.",
        "Stage 1 Z-Stack parity: "
        f"**{stats['stage1']['verified']}/{stats['stage1']['rows']} = {stats['stage1']['percent']}** "
        "[complete Stage 1 requires CC2531 + CC2652P; R1 release requires Dongle-P only].",
        "Global upstream parity: "
        f"**{stats['global']['verified']}/{stats['global']['rows']} = {stats['global']['percent']}** "
        "[all adapter families].",
        "Stage 1 / global weighted parity: "
        f"{stats['stage1']['weighted_percent']} / {stats['global']['weighted_percent']} "
        f"({stats['stage1']['verified_weight']}/{stats['stage1']['weight']}, "
        f"{stats['global']['verified_weight']}/{stats['global']['weight']}).",
        "Stage 1 / global scenarios: "
        f"{stats['stage1']['verified_scenarios']}/{stats['stage1']['scenarios']} = "
        f"{stats['stage1']['scenario_percent']} and "
        f"{stats['global']['verified_scenarios']}/{stats['global']['scenarios']} = "
        f"{stats['global']['scenario_percent']} "
        f"[{stats['stage1']['unmeasured']}/{stats['global']['unmeasured']} unmeasured rows].",
        "Status breakdown: "
        + ", ".join(f"{status} {statuses[status]}" for status in sorted(statuses))
        + f" ({len(rows)} total).",
        f"Critical P0 gaps: {p0_text or 'none'}",
        f"Adapter coverage (verified/rows): {family_text}",
        "Inventory audit: incomplete; no substantiated domain fraction, catalog total or converter denominator.",
        "Device definition parity: UNKNOWN (EVID-02 catalog audit not performed).",
        "Converter parity: UNKNOWN (per-definition execution coverage not enumerated).",
        "Open upstream deltas: see upstream-changes.md; "
        f"freshness {lock['freshness_status']}.",
        f"Last test run ({checks['captured_utc']}, {checks.get('environment', 'environment UNKNOWN')}): {checks_text}.",
        "Known exclusions: "
        + (", ".join(sorted(excluded)) if excluded else "none approved")
        + "; future adapters are deferred, not excluded.",
        "R1 acceptance is tracked separately in .opencode/plan/10-acceptance-matrix.md; "
        "these percentages are not R1 completion.",
        "Counting rule: only whole VERIFIED rows with existing evidence and test references contribute; "
        "newly discovered work increases denominators and is not counted as verified progress.",
        END,
    ]
    return "\n".join(lines)


def read_region(text):
    start = text.find(BEGIN)
    end = text.find(END)
    if start == -1 or end == -1 or end < start:
        raise ValueError("dashboard markers are missing or out of order")
    return text[start:end + len(END)]


def replace_region(text, region):
    start = text.find(BEGIN)
    end = text.find(END)
    if start == -1 or end == -1 or end < start:
        raise ValueError("dashboard markers are missing or out of order")
    return text[:start] + region + text[end + len(END):]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true", help="rewrite the generated dashboard region")
    mode.add_argument("--check", action="store_true", help="validate and fail if the dashboard is stale")
    parser.add_argument("--inventory", type=Path, default=INVENTORY)
    parser.add_argument("--lock", type=Path, default=LOCK)
    parser.add_argument("--checks", type=Path, default=CHECKS)
    parser.add_argument("--dashboard", type=Path, default=DASHBOARD)
    parser.add_argument("--decisions", type=Path, default=DECISIONS)
    args = parser.parse_args(argv)

    try:
        rows = load_inventory(args.inventory)
        lock = load_json(args.lock)
        checks = load_json(args.checks)
        decisions_text = args.decisions.read_text(encoding="utf-8")
        errors = validate(rows, lock, checks, decisions_text)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"parity validation failed: {error}", file=sys.stderr)
        return 1

    if errors:
        print("parity validation failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    region = render(rows, lock, checks)
    if args.write:
        text = args.dashboard.read_text(encoding="utf-8")
        args.dashboard.write_text(replace_region(text, region) + ("\n" if not text.endswith("\n") else ""), encoding="utf-8")
        print(f"updated {args.dashboard.relative_to(ROOT) if args.dashboard.is_relative_to(ROOT) else args.dashboard}")
        return 0
    if args.check:
        current = read_region(args.dashboard.read_text(encoding="utf-8"))
        if current != region:
            print("dashboard is stale; run scripts/parity_report.py --write", file=sys.stderr)
            return 1
        print("parity inventory and dashboard are valid and current")
        return 0

    print(region.replace(BEGIN, "").replace(END, "").strip())
    return 0


if __name__ == "__main__":
    sys.exit(main())
