import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "generated_definition_diff.py"
SPEC = importlib.util.spec_from_file_location("generated_definition_diff", MODULE_PATH)
generated_diff = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(generated_diff)

SCRIPT = r"""
import json, sys
fixture = json.load(sys.stdin)
name = fixture["device"]["model_id"]
label = sys.argv[1] if len(sys.argv) > 1 else ""
payload = {
    "definition": {
        "source": "generated",
        "model": name,
        "vendor": "",
        "description": "Automatically generated definition",
        "supports_ota": False,
        "options": [],
        "version": "0.0.0",
        "exposes": [{"name": "linkquality", "type": "numeric"}],
        "extra": label,
    }
}
print(json.dumps(payload))
"""


class GeneratedDefinitionDiffTests(unittest.TestCase):
    def _run(self, fixture: dict, reference_label: str, candidate_label: str):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fixtures = root / "fixtures"
            fixtures.mkdir()
            (fixtures / "case.json").write_text(json.dumps(fixture), encoding="utf-8")
            script = root / "cmd.py"
            script.write_text(SCRIPT, encoding="utf-8")
            report = root / "report.json"
            code = generated_diff.main(
                [
                    "--fixtures",
                    str(fixtures),
                    "--reference-cmd",
                    f"{sys.executable} {script} {reference_label}",
                    "--candidate-cmd",
                    f"{sys.executable} {script} {candidate_label}",
                    "--skip-pins",
                    "--report",
                    str(report),
                ]
            )
            return code, json.loads(report.read_text())

    def test_matching_payloads_pass(self):
        fixture = {"id": "case", "input": {"device": {"model_id": "unknown.device"}}}
        code, report = self._run(fixture, "same", "same")
        self.assertEqual(code, 0)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["passed"], 1)
        self.assertEqual(report["cases"][0]["status"], "PASS")

    def test_mismatching_payloads_fail(self):
        fixture = {"id": "case", "input": {"device": {"model_id": "unknown.device"}}}
        code, report = self._run(fixture, "reference", "candidate")
        self.assertEqual(code, 1)
        self.assertEqual(report["status"], "FAIL")
        self.assertEqual(report["failed"], 1)
        self.assertTrue(
            any("extra" in mismatch for mismatch in report["cases"][0]["mismatches"])
        )


if __name__ == "__main__":
    unittest.main()
