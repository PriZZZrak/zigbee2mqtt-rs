import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "storage_diff.py"
SPEC = importlib.util.spec_from_file_location("storage_diff", MODULE_PATH)
storage_diff = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(storage_diff)

SCRIPT = r'''
import argparse, json
parser = argparse.ArgumentParser()
parser.add_argument("--db")
parser.add_argument("--config")
parser.add_argument("--rewrite")
args = parser.parse_args()
devices = [
    {
        "ieeeAddr": "0x000b57fffec6a5b2",
        "nwkAddr": 6543,
        "friendlyName": "NAME",
        "disabled": False,
        "manufacturerName": "IKEA of Sweden",
        "manufacturerId": 4476,
        "powerSource": "Mains (single phase)",
        "modelId": "LED1545G12",
        "swBuildId": None,
        "interviewComplete": True,
        "endpoints": [
            {"id": 1, "profileId": 260, "deviceId": 256, "inputClusters": [0, 6, 8],
             "outputClusters": []}
        ],
    }
]
if args.rewrite:
    open(args.rewrite, "w").write("")
print(json.dumps(devices))
'''


class StorageDiffTests(unittest.TestCase):
    def _commands(self, directory: Path, candidate_name="bulb"):
        reference = directory / "reference.py"
        candidate = directory / "candidate.py"
        reference.write_text(SCRIPT.replace("NAME", "bulb"), encoding="utf-8")
        candidate.write_text(SCRIPT.replace("NAME", candidate_name), encoding="utf-8")
        return f"{sys.executable} {reference}", f"{sys.executable} {candidate}"

    def test_matching_reference_and_candidate_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            db = root / "database.db"
            db.write_text("{}\n", encoding="utf-8")
            config = root / "configuration.yaml"
            config.write_text("devices: {}\n", encoding="utf-8")
            report = root / "report.json"
            reference, candidate = self._commands(root)
            code = storage_diff.main(
                [
                    "--db", str(db),
                    "--config", str(config),
                    "--reference-cmd", reference,
                    "--candidate-cmd", candidate,
                    "--skip-pins",
                    "--report", str(report),
                ]
            )
            self.assertEqual(code, 0)
            data = json.loads(report.read_text())
            self.assertEqual(data["status"], "PASS")
            self.assertEqual(data["passed"], 3)
            self.assertEqual(data["failed"], 0)
            self.assertEqual(
                [case["id"] for case in data["cases"]],
                [
                    "import-upstream-database",
                    "rewrite-preserves-reference-view",
                    "rewritten-database-import",
                ],
            )

    def test_different_device_view_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            db = root / "database.db"
            db.write_text("{}\n", encoding="utf-8")
            config = root / "configuration.yaml"
            config.write_text("devices: {}\n", encoding="utf-8")
            report = root / "report.json"
            reference, candidate = self._commands(root, candidate_name="other")
            code = storage_diff.main(
                [
                    "--db", str(db),
                    "--config", str(config),
                    "--reference-cmd", reference,
                    "--candidate-cmd", candidate,
                    "--skip-pins",
                    "--report", str(report),
                ]
            )
            self.assertEqual(code, 1)
            data = json.loads(report.read_text())
            self.assertEqual(data["status"], "FAIL")
            self.assertEqual(data["failed"], 2)
            self.assertTrue(
                any("friendlyName" in mismatch for mismatch in data["cases"][0]["mismatches"])
            )


if __name__ == "__main__":
    unittest.main()
