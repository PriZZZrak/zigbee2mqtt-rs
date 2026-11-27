#!/usr/bin/env python3
"""Compare runtime_probe output with tools/bench/corpus/golden.json.

Usage: tools/bench/check_golden.py <probe-output.json> [golden.json]
Exits non-zero on payload or host-call signature differences.
"""
import json
import sys

probe = json.load(open(sys.argv[1]))
golden_path = sys.argv[2] if len(sys.argv) > 2 else "tools/bench/corpus/golden.json"
golden = {case["id"]: case for case in json.load(open(golden_path))}


def signature(call):
    attributes = call.get("attributes")
    if isinstance(attributes, dict):
        attributes = tuple(sorted(attributes.keys()))
    else:
        attributes = tuple(attributes or [])
    return (
        call.get("op"),
        call.get("endpoint"),
        call.get("cluster"),
        call.get("command"),
        attributes,
    )


bad = []
for case in probe["cases"]:
    scenario = case["scenarioId"]
    result = case["result"]
    expected = golden[scenario]["expected"]
    if expected.get("result") is not None or result.get("result") is not None:
        if json.dumps(expected.get("result"), sort_keys=True) != json.dumps(
            result.get("result"), sort_keys=True
        ):
            bad.append((scenario, "result"))
        if json.dumps(expected.get("published"), sort_keys=True) != json.dumps(
            result.get("published"), sort_keys=True
        ):
            bad.append((scenario, "published"))
    expected_calls = [signature(call) for call in golden[scenario].get("expectedHostCalls") or []]
    actual_calls = [signature(call) for call in result.get("calls") or []]
    if expected_calls != actual_calls:
        bad.append((scenario, "host-calls"))

if probe.get("failed"):
    bad.append(("probe", f"{probe['failed']} failed scenarios"))
if bad:
    print("golden parity failed:", bad)
    sys.exit(1)
print(json.dumps({"ok": True, "cases": len(probe["cases"]), "failed": 0}))
