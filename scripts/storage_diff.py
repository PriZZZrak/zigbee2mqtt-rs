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


def run_raw(command: str, database: Path) -> list:
    argv = shlex.split(command) + ["--db", str(database), "--raw"]
    completed = subprocess.run(argv, capture_output=True, text=True, timeout=120, check=False)
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed ({completed.returncode}): {' '.join(argv)}\n{completed.stderr.strip()}"
        )
    return json.loads(completed.stdout)


def compare_entries(reference: list, candidate: list) -> list[str]:
    """Compare raw database entries (dict order-insensitive, IDs sorted)."""
    expected = {entry.get("id"): entry for entry in reference}
    actual = {entry.get("id"): entry for entry in candidate}
    mismatches = []
    if set(expected) != set(actual):
        mismatches.append(
            f"entry ids differ: reference={sorted(expected)} candidate={sorted(actual)}"
        )
    for entry_id in sorted(set(expected) & set(actual)):
        if expected[entry_id] != actual[entry_id]:
            for key in sorted(set(expected[entry_id]) | set(actual[entry_id])):
                if expected[entry_id].get(key) != actual[entry_id].get(key):
                    mismatches.append(
                        f"id {entry_id}.{key}: reference={expected[entry_id].get(key)!r} "
                        f"candidate={actual[entry_id].get(key)!r}"
                    )
    return mismatches


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

    def add_raw_case(case_id: str, reference: list, candidate: list) -> None:
        nonlocal failures
        mismatches = compare_entries(reference, candidate)
        status = "PASS" if not mismatches else "FAIL"
        if mismatches:
            failures += 1
            print(f"{case_id}: FAIL", file=sys.stderr)
            for mismatch in mismatches:
                print(f"  {mismatch}", file=sys.stderr)
        cases.append({"id": case_id, "status": status, "mismatches": mismatches})

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

    def rewritten_pair(source: Path, directory: Path, label: str) -> Path:
        output = directory / f"{label}.db"
        run(args.candidate_cmd, source, args.config, ["--rewrite", str(output)])
        return output

    with tempfile.TemporaryDirectory(prefix="z2m-storage-") as directory:
        root = Path(directory)
        rewritten = rewritten_pair(args.db, root, "rewritten")
        reference_rewritten = run(args.reference_cmd, rewritten, args.config)
        candidate_rewritten = run(args.candidate_cmd, rewritten, args.config)
        add_case("rewrite-preserves-reference-view", reference_import, reference_rewritten)
        add_case("rewritten-database-import", reference_rewritten, candidate_rewritten)
        add_raw_case(
            "rewrite-preserves-persisted-entries",
            run_raw(args.reference_cmd, args.db),
            run_raw(args.reference_cmd, rewritten),
        )

        # Corrupted line: both importers skip it; rewrite must keep the valid
        # entries (including Group/GreenPower) and drop only the bad line.
        corrupt_source = root / "corrupted.db"
        corrupt_source.write_bytes(
            args.db.read_bytes() + b'\n{"id": 999, "type":'
        )
        add_case(
            "corrupted-database-import",
            run(args.reference_cmd, corrupt_source, args.config),
            run(args.candidate_cmd, corrupt_source, args.config),
        )
        corrupt_rewritten = rewritten_pair(corrupt_source, root, "corrupted-rewritten")
        add_raw_case(
            "corrupted-rewrite-preserves-entries",
            run_raw(args.reference_cmd, corrupt_source),
            run_raw(args.reference_cmd, corrupt_rewritten),
        )

        # Truncated last line (simulated interrupted write).
        truncated_source = root / "truncated.db"
        data = args.db.read_bytes()
        truncated_source.write_bytes(data[:-12])
        add_case(
            "truncated-database-import",
            run(args.reference_cmd, truncated_source, args.config),
            run(args.candidate_cmd, truncated_source, args.config),
        )
        truncated_rewritten = rewritten_pair(truncated_source, root, "truncated-rewritten")
        add_raw_case(
            "truncated-rewrite-preserves-entries",
            run_raw(args.reference_cmd, truncated_source),
            run_raw(args.reference_cmd, truncated_rewritten),
        )

        # Stale .tmp from a previous failed write: Rust must rename it, keep the
        # source entries and produce a database the reference can read.
        stale_source = root / "stale.db"
        stale_source.write_bytes(args.db.read_bytes())
        (root / "stale.db.tmp").write_bytes(b"partial garbage")
        stale_rewritten = rewritten_pair(stale_source, root, "stale-rewritten")
        add_raw_case(
            "stale-temp-write-recovers",
            run_raw(args.reference_cmd, args.db),
            run_raw(args.reference_cmd, stale_rewritten),
        )

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
