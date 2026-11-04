import contextlib
import importlib.util
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "evid04_harness.py"
SPEC = importlib.util.spec_from_file_location("evid04_harness", MODULE_PATH)
harness = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(harness)

FAKE = Path(__file__).resolve().parent / "fake_evid04_reference.py"
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "evid04_smoke"


class NormalizationTests(unittest.TestCase):
    def test_ignored_paths_and_sorted_arrays(self):
        value = {
            "last_seen": 1,
            "nested": {"last_seen": 2, "value": 3},
            "actions": ["b", "a"],
        }
        normalized = harness.normalize(
            value,
            ignore=["last_seen", "nested.last_seen"],
            sort_paths=["actions"],
        )
        self.assertEqual(
            normalized,
            {"nested": {"value": 3}, "actions": ["a", "b"]},
        )

    def test_wildcard_ignore(self):
        value = {"a": {"private": 1}, "b": {"private": 2}, "keep": 3}
        normalized = harness.normalize(value, ignore=["*.private"])
        self.assertEqual(normalized, {"a": {}, "b": {}, "keep": 3})


def run_main(args):
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        return harness.main(args)


class HarnessTests(unittest.TestCase):
    def test_fake_reference_passes_after_normalization(self):
        command_ref = f"{sys.executable} {FAKE} reference"
        command_candidate = f"{sys.executable} {FAKE} candidate"
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "report.json"
            exit_code = run_main(
                [
                    "--fixtures",
                    str(FIXTURES),
                    "--reference-cmd",
                    command_ref,
                    "--candidate-cmd",
                    command_candidate,
                    "--skip-pins",
                    "--report",
                    str(report),
                ]
            )
            self.assertEqual(exit_code, 0)
            data = json.loads(report.read_text())
            self.assertEqual(data["status"], "PASS")
            self.assertEqual(data["cases"][0]["status"], "PASS")

    def test_mismatch_fails(self):
        command_ref = f"{sys.executable} {FAKE} reference"
        command_candidate = (
            f"{sys.executable} -c \"import json,sys;d=json.load(sys.stdin);"
            "print(json.dumps({'temperature': 0, 'lqi': 42, 'actions': ['a','b'], 'last_seen': 222}))\""
        )
        exit_code = run_main(
            [
                "--fixtures",
                str(FIXTURES),
                "--reference-cmd",
                command_ref,
                "--candidate-cmd",
                command_candidate,
                "--skip-pins",
            ]
        )
        self.assertEqual(exit_code, 1)

    def test_pin_match_and_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = root / "zigbee2mqtt"
            repo.mkdir()
            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            subprocess.run(["git", "-C", str(repo), "config", "user.email", "test@example.com"], check=True)
            subprocess.run(["git", "-C", str(repo), "config", "user.name", "test"], check=True)
            (repo / "file").write_text("one", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "file"], check=True)
            subprocess.run(["git", "-C", str(repo), "commit", "-qm", "one"], check=True)
            commit = subprocess.check_output(
                ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
            ).strip()
            lock = root / "lock.json"
            lock.write_text(json.dumps({"baseline": {"zigbee2mqtt": {"commit": commit}}}))
            pins = harness.verify_pins(lock, root)
            self.assertEqual(pins[0]["status"], "MATCH")

            (repo / "file").write_text("two", encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "commit", "-qam", "two"], check=True)
            pins = harness.verify_pins(lock, root)
            self.assertEqual(pins[0]["status"], "MISMATCH")


if __name__ == "__main__":
    unittest.main()
