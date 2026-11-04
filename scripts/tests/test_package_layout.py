import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "check_package_layout.py"
SPEC = importlib.util.spec_from_file_location("check_package_layout", MODULE_PATH)
layout = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(layout)


def build_package(root: Path, ref="v1.0.0"):
    for name in ("devices", "converters", "lib"):
        (root / name).mkdir(parents=True, exist_ok=True)
        (root / name / "placeholder.js").write_text("module.exports = {};\n", encoding="utf-8")
    registry = {
        "ref": ref,
        "lookup": {"model": [["fixture.js", 0]]},
        "models": {},
        "fingerprints": [{"file": "fixture.js", "index": 0, "fingerprint": {}}],
    }
    (root / "devices" / "fixture.js").write_text("module.exports = {definitions: []};\n", encoding="utf-8")
    (root / "registry.json").write_text(json.dumps(registry), encoding="utf-8")
    (root / "zcl.json").write_text(json.dumps({"clusters": {}}), encoding="utf-8")
    (root / "zcl-commands.json").write_text(
        json.dumps(
            {
                "clusters": {
                    "6": {
                        "name": "genOnOff",
                        "attributes": {"0": {"name": "onOff", "type": 16}},
                        "commands": {},
                    }
                }
            }
        ),
        encoding="utf-8",
    )
    files = []
    for path in sorted(root.rglob("*.js")) + [
        root / "registry.json",
        root / "zcl.json",
        root / "zcl-commands.json",
    ]:
        files.append({"path": str(path.relative_to(root)), "bytes": path.stat().st_size, "sha256": "test"})
    manifest = {"schema": 1, "ref": ref, "generated_at": "2026-11-04T00:00:00Z", "counts": {}, "files": files}
    (root / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")


class LayoutTests(unittest.TestCase):
    def test_valid_package(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_package(root)
            self.assertEqual(layout.validate(root, "v1.0.0"), [])

    def test_missing_lookup_candidate_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_package(root)
            (root / "devices" / "fixture.js").unlink()
            errors = layout.validate(root)
            self.assertTrue(any("candidate device file" in error for error in errors))

    def test_missing_helper_directory_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_package(root)
            (root / "converters" / "placeholder.js").unlink()
            errors = layout.validate(root)
            self.assertTrue(any("converters/ contains no JavaScript" in error for error in errors))

    def test_typescript_leak_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_package(root)
            (root / "lib" / "leftover.ts").write_text("export {};\n", encoding="utf-8")
            errors = layout.validate(root)
            self.assertTrue(any("TypeScript source leaked" in error for error in errors))

    def test_missing_zcl_type_table_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_package(root)
            (root / "zcl-commands.json").unlink()
            errors = layout.validate(root)
            self.assertTrue(any("missing zcl-commands.json" in error for error in errors))

    def test_zcl_type_table_without_gen_on_off_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_package(root)
            (root / "zcl-commands.json").write_text(
                json.dumps({"clusters": {"1026": {"name": "x", "attributes": {}}}}),
                encoding="utf-8",
            )
            errors = layout.validate(root)
            self.assertTrue(any("genOnOff" in error for error in errors))

    def test_declaration_files_are_allowed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_package(root)
            (root / "lib" / "utils.d.ts").write_text("export {};\n", encoding="utf-8")
            self.assertEqual(layout.validate(root), [])

    def test_manifest_ref_mismatch_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_package(root, ref="v1.0.0")
            errors = layout.validate(root, "v2.0.0")
            self.assertTrue(any("does not match" in error for error in errors))

    def test_manifest_entry_missing_on_disk_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_package(root)
            manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
            manifest["files"].append({"path": "devices/missing.js", "bytes": 1, "sha256": "x"})
            (root / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
            errors = layout.validate(root)
            self.assertTrue(any("manifest entry missing" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
