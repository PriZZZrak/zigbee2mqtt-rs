#!/usr/bin/env python3
"""Generated-definition differential.

Compares the Rust port of zhc's `generateDefinition` with a reference that runs
the pinned `zigbee-herdsman-converters` generator, using full ZCL cluster names
(the app's test mocks expose only a partial cluster map, so the bridge API dump
cannot serve as an oracle for generated definitions).

Both commands read the fixture input as JSON on stdin and print the definition
payload. See `docs/parity/evidence/generated-definitions/`.
"""

from __future__ import annotations

import argparse
import json
import shlex
import subprocess
import sys
from pathlib import Path

try:
    from evid04_harness import verify_pins
except ImportError:  # pragma: no cover
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from evid04_harness import verify_pins


def run(command: str, payload: dict) -> dict:
    completed = subprocess.run(
        shlex.split(command),
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {command}\n{completed.stderr.strip()[:2000]}"
        )
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"command output is not JSON: {error}\n{completed.stdout[:500]}")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--reference-cmd", required=True)
    parser.add_argument("--candidate-cmd", required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--lock", type=Path, default=Path("docs/parity/upstream-lock.json"))
    parser.add_argument(
        "--pin-root", type=Path, default=Path(__file__).resolve().parents[1] / ".upstream"
    )
    parser.add_argument("--require-pins", action="store_true")
    parser.add_argument("--skip-pins", action="store_true")
    args = parser.parse_args(argv)

    pins = [] if args.skip_pins else verify_pins(args.lock, args.pin_root)
    if args.require_pins and any(pin["status"] != "MATCH" for pin in pins):
        for pin in pins:
            print(f"{pin['source']}: {pin['status']}", file=sys.stderr)
        return 1

    cases = []
    failures = 0
    for path in sorted(args.fixtures.glob("*.json")):
        fixture = json.loads(path.read_text(encoding="utf-8"))
        try:
            reference = run(args.reference_cmd, fixture["input"])
            candidate = run(args.candidate_cmd, fixture["input"])
        except RuntimeError as error:
            failures += 1
            cases.append(
                {
                    "id": fixture.get("id", path.stem),
                    "status": "ERROR",
                    "error": str(error),
                }
            )
            print(f"{fixture.get('id', path.stem)}: ERROR {error}", file=sys.stderr)
            continue
        reference_payload = reference.get("definition")
        candidate_payload = candidate.get("definition")
        status = "PASS" if reference_payload == candidate_payload else "FAIL"
        mismatches = []
        if status == "FAIL":
            failures += 1
            for key in sorted(set(reference_payload) | set(candidate_payload)):
                if reference_payload.get(key) != candidate_payload.get(key):
                    mismatches.append(
                        f"{key}: reference={reference_payload.get(key)!r} "
                        f"candidate={candidate_payload.get(key)!r}"
                    )
            print(f"{fixture.get('id', path.stem)}: FAIL", file=sys.stderr)
            for mismatch in mismatches:
                print(f"  {mismatch[:400]}", file=sys.stderr)
        cases.append(
            {
                "id": fixture.get("id", path.stem),
                "status": status,
                "mismatches": mismatches,
            }
        )

    report = {
        "status": "PASS" if failures == 0 else "FAIL",
        "passed": len(cases) - failures,
        "failed": failures,
        "pins": pins,
        "cases": cases,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(
        f"GENERATED: {report['passed']} passed, {report['failed']} failed, "
        f"pins={[(pin['source'], pin['status']) for pin in pins]}"
    )
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
