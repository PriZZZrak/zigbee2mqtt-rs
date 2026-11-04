import importlib.util
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "parity_report.py"
SPEC = importlib.util.spec_from_file_location("parity_report", MODULE_PATH)
parity = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(parity)

APP = "344b6e03b43644c97154a72ca9eae17193c553a2"
HERDSMAN = "3cb89ad906bd9b2de8b3dfed20d5a21070a467d2"
CONVERTERS = "9dabbe4e343573c5a3070605efd843a1b3a16058"


def row(**overrides):
    base = {
        "id": "RUNTIME-001",
        "domain": "Runtime",
        "behavior": "Example behavior",
        "baseline_sha": APP,
        "upstream_refs": ".upstream/zigbee2mqtt/lib/controller.ts",
        "dependency_refs": "",
        "stage": "1",
        "adapter_family": "shared",
        "priority": "P0",
        "weight": "1",
        "status": "DISCOVERED",
        "verified_cases": "0",
        "total_cases": "0",
        "test_refs": "",
        "evidence_ref": "",
        "owner": "unassigned",
        "upstream_changed": "no",
        "notes": "",
    }
    base.update(overrides)
    return base


def lock():
    return {
        "baseline": {
            "zigbee2mqtt": {"commit": APP},
            "zigbee-herdsman": {"commit": HERDSMAN, "resolved_version": "10.10.0"},
            "zigbee-herdsman-converters": {"commit": CONVERTERS, "resolved_version": "26.115.1"},
        },
        "latest_heads": {"zigbee2mqtt": APP},
        "last_successful_fetch_utc": "2026-11-04T09:08:33+00:00",
        "last_checked_utc": "2026-11-04T09:08:33+00:00",
        "freshness_status": "DRIFT_DETECTED",
    }


def checks():
    return {
        "captured_utc": "2026-11-04T10:00:00+00:00",
        "environment": "test",
        "commands": [{"command": "cargo test", "exit": 0, "result": "247 passed"}],
    }


class ValidationTests(unittest.TestCase):
    def validate(self, rows, decisions="", root=None):
        return parity.validate(rows, lock(), checks(), decisions, root=root or Path(tempfile.gettempdir()))

    def test_valid_discovery_row(self):
        self.assertEqual(self.validate([row()]), [])

    def test_duplicate_id(self):
        errors = self.validate([row(), row()])
        self.assertTrue(any("duplicate id" in error for error in errors))

    def test_invalid_status(self):
        errors = self.validate([row(status="DONE")])
        self.assertTrue(any("invalid status" in error for error in errors))

    def test_verified_requires_evidence_and_tests(self):
        errors = self.validate([row(status="VERIFIED", total_cases="2", verified_cases="2")])
        self.assertTrue(any("requires evidence_ref" in error for error in errors))
        self.assertTrue(any("requires test_refs" in error for error in errors))

    def test_verified_requires_all_cases(self):
        errors = self.validate([row(status="VERIFIED", total_cases="3", verified_cases="2")])
        self.assertTrue(any("verified_cases == total_cases" in error for error in errors))

    def test_rejects_case_overflow(self):
        errors = self.validate([row(total_cases="1", verified_cases="2")])
        self.assertTrue(any("exceeds total_cases" in error for error in errors))

    def test_rejects_unpinned_sha(self):
        errors = self.validate([row(baseline_sha="c18c32d")])
        self.assertTrue(any("full 40-character SHA" in error for error in errors))
        errors = self.validate([row(baseline_sha="a" * 40)])
        self.assertTrue(any("not a pinned baseline commit" in error for error in errors))

    def test_rejects_hardware_claim_without_hil(self):
        errors = self.validate([row(status="VERIFIED", total_cases="1", verified_cases="1",
                                    test_refs="tests/mock.rs", evidence_ref="ev.md",
                                    notes="HARDWARE_PENDING")], root=self._root_with_evidence())
        self.assertTrue(any("hardware-dependent" in error for error in errors))

    def test_exclusion_requires_decision(self):
        errors = self.validate([row(status="OUT_OF_SCOPE_APPROVED")])
        self.assertTrue(any("approved decision" in error for error in errors))
        self.assertEqual(self.validate([row(status="OUT_OF_SCOPE_APPROVED")], decisions="RUNTIME-001 approved"), [])

    def _root_with_evidence(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        (Path(directory.name) / "ev.md").write_text("evidence", encoding="utf-8")
        return Path(directory.name)

    def test_zero_denominator_is_not_100_percent(self):
        self.assertEqual(parity.pct(0, 0), "N/A")
        self.assertEqual(parity.pct(1, 4), "25.00%")

    def test_metrics_separate_stage1_from_global(self):
        rows = [row(), row(id="ADAPTER-010", stage="2", adapter_family="ember")]
        stats = parity.metrics(rows)
        self.assertEqual((stats["stage1"]["verified"], stats["stage1"]["rows"]), (0, 1))
        self.assertEqual((stats["global"]["verified"], stats["global"]["rows"]), (0, 2))
        self.assertEqual(stats["global"]["unmeasured"], 2)

    def test_generated_region_roundtrip(self):
        text = f"head\n{parity.BEGIN}\nold\n{parity.END}\ntail\n"
        replaced = parity.replace_region(text, f"{parity.BEGIN}\nnew\n{parity.END}")
        self.assertEqual(parity.read_region(replaced), f"{parity.BEGIN}\nnew\n{parity.END}")
        self.assertTrue(replaced.startswith("head\n"))
        self.assertTrue(replaced.endswith("\ntail\n"))


class SchemaTests(unittest.TestCase):
    def test_rejects_shifted_columns(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "inventory.csv"
            path.write_text("id,status\nA-1,DISCOVERED\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                parity.load_inventory(path)


if __name__ == "__main__":
    unittest.main()
