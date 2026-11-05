#!/usr/bin/env python3
"""Storage differential: upstream database/config import and rewrite.

Runs the pinned Node reference (zigbee-herdsman `Database` + z2m-style config)
and the Rust storage candidate on the same fixture, then lets Rust rewrite the
database and checks what the reference reads back.

Commands must accept `--db <path> --config <path>` and print normalized device
JSON; the candidate additionally accepts `--rewrite <path>`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path

try:
    from evid04_harness import verify_pins
except ImportError:  # pragma: no cover - module is a sibling script
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from evid04_harness import verify_pins


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


def run(command: str, database: Path, config: Path, extra: list[str] | None = None) -> dict:
    argv = shlex.split(command) + ["--db", str(database), "--config", str(config)]
    if extra:
        argv += extra
    completed = subprocess.run(argv, capture_output=True, text=True, timeout=120, check=False)
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(argv)}\n{completed.stderr.strip()}"
        )
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"command output is not JSON: {error}\n{completed.stdout[:500]}")


def compare(reference: dict, candidate: dict) -> list[str]:
    """Return human-readable mismatches (empty when equal)."""
    mismatches = []
    reference_devices = {device["ieeeAddr"]: device for device in reference}
    candidate_devices = {device["ieeeAddr"]: device for device in candidate}
    if set(reference_devices) != set(candidate_devices):
        mismatches.append(
            f"device sets differ: reference={sorted(reference_devices)} candidate={sorted(candidate_devices)}"
        )
    for ieee in sorted(set(reference_devices) & set(candidate_devices)):
        expected = reference_devices[ieee]
        actual = candidate_devices[ieee]
        if expected != actual:
            for key in sorted(set(expected) | set(actual)):
                if expected.get(key) != actual.get(key):
                    mismatches.append(
                        f"{ieee}.{key}: reference={expected.get(key)!r} candidate={actual.get(key)!r}"
                    )
    return mismatches


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--reference-cmd", required=True)
    parser.add_argument("--candidate-cmd", required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--lock", type=Path, default=Path("docs/parity/upstream-lock.json"))
    parser.add_argument(
        "--pin-root",
        type=Path,
        default=Path(__file__).resolve().parents[1] / ".upstream",
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

    def add_case(case_id: str, reference: dict, candidate: dict) -> None:
        nonlocal failures
        mismatches = compare(reference, candidate)
        status = "PASS" if not mismatches else "FAIL"
        if mismatches:
            failures += 1
            print(f"{case_id}: FAIL", file=sys.stderr)
            for mismatch in mismatches:
                print(f"  {mismatch}", file=sys.stderr)
        cases.append({"id": case_id, "status": status, "mismatches": mismatches})

    reference_import = run(args.reference_cmd, args.db, args.config)
    candidate_import = run(args.candidate_cmd, args.db, args.config)
    add_case("import-upstream-database", reference_import, candidate_import)

    with tempfile.TemporaryDirectory(prefix="z2m-storage-") as directory:
        rewritten = Path(directory) / "database.db"
        run(args.candidate_cmd, args.db, args.config, ["--rewrite", str(rewritten)])
        reference_rewritten = run(args.reference_cmd, rewritten, args.config)
        candidate_rewritten = run(args.candidate_cmd, rewritten, args.config)

    add_case("rewrite-preserves-reference-view", reference_import, reference_rewritten)
    add_case("rewritten-database-import", reference_rewritten, candidate_rewritten)

    report = {
        "status": "PASS" if failures == 0 else "FAIL",
        "passed": len(cases) - failures,
        "failed": failures,
        "database": {"path": str(args.db), "sha256": sha256(args.db)},
        "config": {"path": str(args.config), "sha256": sha256(args.config)},
        "pins": pins,
        "cases": cases,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(
        f"STORAGE: {report['passed']} passed, {report['failed']} failed, "
        f"pins={[(pin['source'], pin['status']) for pin in pins]}"
    )
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
