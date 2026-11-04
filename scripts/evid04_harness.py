#!/usr/bin/env python3
"""Reference/differential harness for Zigbee2MQTT parity evidence.

The harness runs a reference command and a Rust/candidate command over the same
fixture input, normalizes explicitly whitelisted fields, compares the JSON and
writes a machine-readable report. Commands read the fixture JSON on stdin and
must emit JSON on stdout.

Pins: the baseline commits in docs/parity/upstream-lock.json are checked against
the local .upstream checkouts. Missing checkouts are reported as UNVERIFIED;
--require-pins turns that into a failure. Node reference execution is not implied
by a successful fake-command smoke test.
"""

import argparse
import json
import re
import shlex
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_LOCK = ROOT / "docs" / "parity" / "upstream-lock.json"
DEFAULT_PIN_ROOT = ROOT / ".upstream"
BASELINE_REPOS = {
    "zigbee2mqtt": "zigbee2mqtt",
    "zigbee-herdsman": "zigbee-herdsman",
    "zigbee-herdsman-converters": "zigbee-herdsman-converters",
    "frontend": "zigbee2mqtt-windfront",
}


def _path_matches(path, patterns):
    for pattern in patterns:
        if pattern == path:
            return True
        if pattern.endswith(".*"):
            if path.startswith(pattern[:-1]):
                return True
        if pattern.startswith("*."):
            if path.endswith(pattern[1:]):
                return True
        if pattern == "*":
            return True
    return False


def _sort_key(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def normalize(value, ignore=(), sort_paths=(), path=""):
    """Remove ignored dotted paths and sort selected arrays canonically."""
    if isinstance(value, dict):
        result = {}
        for key, item in value.items():
            child = f"{path}.{key}" if path else key
            if _path_matches(child, ignore):
                continue
            result[key] = normalize(item, ignore, sort_paths, child)
        return result
    if isinstance(value, list):
        items = [normalize(item, ignore, sort_paths, path) for item in value]
        if _path_matches(path, sort_paths):
            items.sort(key=_sort_key)
        return items
    return value


def load_fixture(path):
    fixture = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(fixture, dict):
        raise ValueError(f"{path}: fixture must be an object")
    if "id" not in fixture or "input" not in fixture:
        raise ValueError(f"{path}: fixture requires id and input")
    ignore = fixture.get("ignore", [])
    sort_paths = fixture.get("sort", [])
    if not isinstance(ignore, list) or not isinstance(sort_paths, list):
        raise ValueError(f"{path}: ignore/sort must be arrays")
    return fixture


def run_command(command, fixture_input, timeout):
    try:
        completed = subprocess.run(
            shlex.split(command),
            input=json.dumps(fixture_input),
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=str(ROOT),
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return None, f"command failed to run: {error}"
    if completed.returncode != 0:
        return None, (
            f"exit {completed.returncode}: "
            f"{(completed.stderr or completed.stdout).strip()[:500]}"
        )
    try:
        return json.loads(completed.stdout), None
    except json.JSONDecodeError as error:
        return None, f"stdout is not JSON: {error}"


def verify_pins(lock_path, pin_root):
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    baseline = lock.get("baseline", {})
    results = []
    for key, expected in baseline.items():
        directory_name = BASELINE_REPOS.get(key)
        commit = expected.get("commit") if isinstance(expected, dict) else None
        if directory_name is None or commit is None:
            results.append(
                {
                    "source": key,
                    "expected": commit,
                    "actual": None,
                    "status": "UNVERIFIED",
                    "reason": "no checkout mapping or baseline commit",
                }
            )
            continue
        directory = pin_root / directory_name
        if not (directory / ".git").exists():
            results.append(
                {
                    "source": key,
                    "expected": commit,
                    "actual": None,
                    "status": "UNVERIFIED",
                    "reason": f"checkout missing: {directory}",
                }
            )
            continue
        completed = subprocess.run(
            ["git", "-C", str(directory), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            check=False,
        )
        actual = completed.stdout.strip() if completed.returncode == 0 else None
        results.append(
            {
                "source": key,
                "expected": commit,
                "actual": actual,
                "status": "MATCH" if actual == commit else "MISMATCH",
                "reason": None if actual == commit else completed.stderr.strip() or "HEAD differs",
            }
        )
    return results


def compare_fixture(fixture, reference_cmd, candidate_cmd, timeout):
    ignore = fixture.get("ignore", [])
    sort_paths = fixture.get("sort", [])
    reference, reference_error = run_command(reference_cmd, fixture["input"], timeout)
    candidate, candidate_error = run_command(candidate_cmd, fixture["input"], timeout)
    if reference_error:
        return {
            "id": fixture["id"],
            "status": "ERROR",
            "reference_error": reference_error,
        }
    if candidate_error:
        return {
            "id": fixture["id"],
            "status": "ERROR",
            "candidate_error": candidate_error,
        }
    normalized_reference = normalize(reference, ignore, sort_paths)
    normalized_candidate = normalize(candidate, ignore, sort_paths)
    if normalized_reference == normalized_candidate:
        return {
            "id": fixture["id"],
            "status": "PASS",
            "ignore": ignore,
            "sort": sort_paths,
        }
    return {
        "id": fixture["id"],
        "status": "MISMATCH",
        "reference": normalized_reference,
        "candidate": normalized_candidate,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--reference-cmd", required=True)
    parser.add_argument("--candidate-cmd", required=True)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--lock", type=Path, default=DEFAULT_LOCK)
    parser.add_argument("--pin-root", type=Path, default=DEFAULT_PIN_ROOT)
    parser.add_argument("--require-pins", action="store_true")
    parser.add_argument("--skip-pins", action="store_true")
    parser.add_argument("--timeout", type=float, default=30.0)
    args = parser.parse_args(argv)

    if not args.fixtures.is_dir():
        print(f"fixtures directory does not exist: {args.fixtures}", file=sys.stderr)
        return 2

    pins = []
    if not args.skip_pins:
        pins = verify_pins(args.lock, args.pin_root)
        if args.require_pins and any(pin["status"] != "MATCH" for pin in pins):
            for pin in pins:
                if pin["status"] != "MATCH":
                    print(f"pin {pin['source']}: {pin['status']} ({pin['reason']})", file=sys.stderr)
            return 2

    cases = []
    failures = []
    for path in sorted(args.fixtures.glob("*.json")):
        try:
            fixture = load_fixture(path)
        except (OSError, ValueError, json.JSONDecodeError) as error:
            failures.append({"id": path.name, "status": "ERROR", "error": str(error)})
            continue
        result = compare_fixture(fixture, args.reference_cmd, args.candidate_cmd, args.timeout)
        cases.append(result)
        if result["status"] != "PASS":
            failures.append(result)

    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "reference_cmd": args.reference_cmd,
        "candidate_cmd": args.candidate_cmd,
        "fixtures": str(args.fixtures),
        "pins": pins,
        "cases": cases,
        "passed": len(cases) - len([case for case in cases if case["status"] != "PASS"]),
        "failed": len([case for case in cases if case["status"] != "PASS"]),
        "status": "PASS" if not failures else "FAIL",
    }
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(
        f"EVID-04: {report['passed']} passed, {report['failed']} failed, "
        f"pins={[(pin['source'], pin['status']) for pin in pins]}"
    )
    if failures:
        for failure in failures:
            print(json.dumps(failure, indent=2), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
