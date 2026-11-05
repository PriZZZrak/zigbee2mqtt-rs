import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "api_diff.py"
SPEC = importlib.util.spec_from_file_location("api_diff", MODULE_PATH)
api_diff = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(api_diff)

SCRIPT = r'''
import argparse, json
parser = argparse.ArgumentParser()
parser.add_argument("--data")
parser.add_argument("--name", default="bulb")
args = parser.parse_args()
print(json.dumps({
    "bridge_state": {"state": "online"},
    "bridge_info": {
        "coordinator": {"ieee_address": "0x00124b00120144ae", "type": "z-Stack"},
        "log_level": "info",
        "permit_join": False,
    },
    "bridge_devices": [{
        "ieee_address": "0x000b57fffec6a5b2",
        "type": "Router",
        "network_address": 6543,
        "friendly_name": args.name,
        "power_source": "Mains (single phase)",
        "model_id": "LED1545G12",
        "manufacturer": "IKEA of Sweden",
        "interview_completed": True,
        "interview_state": "SUCCESSFUL",
        "supported": True,
        "disabled": False,
        "endpoints": {"1": {"bindings": [], "clusters": {"input": [0], "output": []},
                             "configured_reportings": [], "scenes": []}},
    }],
    "permit_join_response": {"data": {"time": 120}, "status": "ok"},
}))
'''


def upstream_dump(name="bulb"):
    return {
        "bridge_state": {"state": "online"},
        "bridge_info": {
            "coordinator": {"ieee_address": "0x00124b00120144ae", "type": "z-Stack"},
            "log_level": "info",
            "permit_join": False,
            "mqtt": {"base_topic": "zigbee2mqtt"},
        },
        "bridge_devices": [
            {
                "ieee_address": "0x000b57fffec6a5b2",
                "type": "Router",
                "network_address": 6543,
                "friendly_name": name,
                "power_source": "Mains (single phase)",
                "model_id": "LED1545G12",
                "manufacturer": "IKEA of Sweden",
                "interview_completed": True,
                "interview_state": "SUCCESSFUL",
                "supported": True,
                "disabled": False,
                "endpoints": {"1": {}},
            }
        ],
        "permit_join_response": {"data": {"time": 120}, "status": "ok"},
        "device_meta": [],
    }


class ApiDiffTests(unittest.TestCase):
    def _run(self, upstream, name, extra=None):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dump = root / "upstream.json"
            dump.write_text(json.dumps(upstream), encoding="utf-8")
            script = root / "candidate.py"
            script.write_text(SCRIPT, encoding="utf-8")
            report = root / "report.json"
            argv = [
                "--upstream", str(dump),
                "--candidate-cmd", f"{__import__('sys').executable} {script} --name {name}",
                "--skip-pins",
                "--report", str(report),
            ]
            argv += extra or []
            code = api_diff.main(argv)
            return code, json.loads(report.read_text())

    def test_matching_candidate_passes(self):
        code, report = self._run(upstream_dump(), "bulb")
        self.assertEqual(code, 0)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["passed"], 5)
        self.assertEqual(
            [case["id"] for case in report["cases"]],
            [
                "bridge-state",
                "bridge-info",
                "bridge-devices",
                "bridge-devices-definitions",
                "permit-join-response",
            ],
        )
        self.assertEqual(report["known_gap_cases"], [])

    def test_known_gap_case_does_not_fail_the_run(self):
        code, report = self._run(
            upstream_dump(),
            "other",
            extra=["--known-gap", "bridge-devices"],
        )
        self.assertEqual(code, 0)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["failed"], 0)
        self.assertEqual(report["known_gap_cases"], ["bridge-devices"])
        known = next(case for case in report["cases"] if case["id"] == "bridge-devices")
        self.assertEqual(known["status"], "KNOWN_GAP")
        self.assertTrue(known["mismatches"])

    def test_renamed_device_fails(self):
        code, report = self._run(upstream_dump(), "other")
        self.assertEqual(code, 1)
        self.assertEqual(report["failed"], 1)
        self.assertEqual(report["cases"][2]["status"], "FAIL")
        self.assertTrue(any("friendly_name" in mismatch
                            for mismatch in report["cases"][2]["mismatches"]))


if __name__ == "__main__":
    unittest.main()
