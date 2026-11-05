#!/usr/bin/env python3
"""Static frontend differential against the pinned `srvx` middleware.

The upstream dump is produced by `static_file_server_dump.test.ts` (real
`node:http` + the pinned app's `createStaticFileServer`); the Rust candidate
serves the same fixture tree through the bridge frontend server.

Only the contract fields are compared: status, content-type, cache-control,
content-encoding and the body. Dynamic validators (`date`, `etag`,
`last-modified`, `connection`, `keep-alive`, `accept-ranges`) are ignored.
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

KEEP_HEADERS = ("content-type", "cache-control", "content-encoding")


def normalize(response: dict) -> dict:
    headers = {key.lower(): value for key, value in (response.get("headers") or {}).items()}
    return {
        "status": response.get("status"),
        "headers": {key: headers.get(key) for key in KEEP_HEADERS if key in headers},
        "body": response.get("body"),
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", type=Path, required=True)
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

    upstream = json.loads(args.upstream.read_text(encoding="utf-8"))["responses"]
    completed = subprocess.run(
        shlex.split(args.candidate_cmd),
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
    )
    if completed.returncode != 0:
        print(completed.stderr, file=sys.stderr)
        return 1
    candidate = json.loads(completed.stdout)["responses"]

    cases = []
    failures = 0
    for case_id in sorted(upstream):
        expected = normalize(upstream[case_id])
        actual = normalize(candidate.get(case_id, {}))
        if expected == actual:
            cases.append({"id": case_id, "status": "PASS", "mismatches": []})
            continue
        failures += 1
        mismatches = [
            f"{key}: reference={expected.get(key)!r} candidate={actual.get(key)!r}"
            for key in ("status", "headers", "body")
            if expected.get(key) != actual.get(key)
        ]
        cases.append({"id": case_id, "status": "FAIL", "mismatches": mismatches})
        print(f"{case_id}: FAIL", file=sys.stderr)
        for mismatch in mismatches:
            print(f"  {mismatch[:400]}", file=sys.stderr)

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
        f"STATIC: {report['passed']} passed, {report['failed']} failed, "
        f"pins={[(pin['source'], pin['status']) for pin in pins]}"
    )
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
