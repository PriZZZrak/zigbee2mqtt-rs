#!/usr/bin/env python3
"""Deterministic fake reference/candidate used only to smoke-test EVID-04.

This is NOT the Node Zigbee2MQTT reference; it intentionally differs in the
whitelisted timestamp and array order so normalization is exercised.
"""
import json
import sys

mode = sys.argv[1] if len(sys.argv) > 1 else "reference"
data = json.load(sys.stdin)
output = {
    "temperature": data["measuredValue"] / 100.0,
    "lqi": data["linkquality"],
    "actions": ["b", "a"] if mode == "reference" else ["a", "b"],
    "last_seen": 111 if mode == "reference" else 222,
}
print(json.dumps(output))
