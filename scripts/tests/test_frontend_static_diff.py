import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "frontend_static_diff.py"
SPEC = importlib.util.spec_from_file_location("frontend_static_diff", MODULE_PATH)
frontend_diff = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(frontend_diff)

SCRIPT = r"""
import json, sys
label = sys.argv[1] if len(sys.argv) > 1 else ""
print(json.dumps({"responses": {
    "index": {"status": 200, "headers": {"content-type": "text/html; charset=utf-8"}, "body": "aGk="},
    "missing": {"status": 404, "headers": {"content-type": "text/html; charset=utf-8"}, "body": label},
}}))
"""


class FrontendStaticDiffTests(unittest.TestCase):
    def _run(self, reference_label: str, candidate_label: str):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            upstream = root / "upstream.json"
            upstream.write_text(
                json.dumps(
                    {
                        "responses": {
                            "index": {
                                "status": 200,
                                "headers": {
                                    "content-type": "text/html; charset=utf-8",
                                    "date": "ignored",
                                },
                                "body": "aGk=",
                            },
                            "missing": {
                                "status": 404,
                                "headers": {"content-type": "text/html; charset=utf-8"},
                                "body": reference_label,
                            },
                        }
                    }
                ),
                encoding="utf-8",
            )
            script = root / "cmd.py"
            script.write_text(SCRIPT, encoding="utf-8")
            report = root / "report.json"
            code = frontend_diff.main(
                [
                    "--upstream",
                    str(upstream),
                    "--candidate-cmd",
                    f"{sys.executable} {script} {candidate_label}",
                    "--skip-pins",
                    "--report",
                    str(report),
                ]
            )
            return code, json.loads(report.read_text())

    def test_contract_fields_match(self):
        code, report = self._run("cGFnZQ==", "cGFnZQ==")
        self.assertEqual(code, 0)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["passed"], 2)

    def test_body_mismatch_fails(self):
        code, report = self._run("cGFnZQ==", "b3RoZXI=")
        self.assertEqual(code, 1)
        self.assertEqual(report["failed"], 1)
        self.assertTrue(any("body" in mismatch for mismatch in report["cases"][1]["mismatches"]))


if __name__ == "__main__":
    unittest.main()
