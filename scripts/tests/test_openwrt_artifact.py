import hashlib
import json
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "build-openwrt-artifact.sh"
PREBUILT = ROOT / "openwrt" / "files" / "zigbee2mqtt-rs"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    digest.update(path.read_bytes())
    return digest.hexdigest()


@unittest.skipUnless(PREBUILT.is_file(), "committed aarch64 binary missing")
class OpenWrtArtifactTests(unittest.TestCase):
    def test_stages_manifest_and_tarball(self):
        with tempfile.TemporaryDirectory() as directory:
            completed = subprocess.run(
                ["bash", str(SCRIPT), str(PREBUILT), directory],
                capture_output=True,
                text=True,
                check=False,
                cwd=str(ROOT),
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            manifests = list(Path(directory).glob("*.manifest.json"))
            self.assertEqual(len(manifests), 1)
            manifest = json.loads(manifests[0].read_text())
            self.assertEqual(manifest["schema"], 1)
            self.assertEqual(manifest["target"], "aarch64-unknown-linux-musl")
            self.assertEqual(manifest["elf_machine"], "aarch64")
            self.assertTrue(manifest["static"])
            self.assertEqual(manifest["binary_sha256"], sha256(PREBUILT))
            self.assertEqual(manifest["binary_bytes"], PREBUILT.stat().st_size)
            self.assertEqual(
                manifest["provenance"], "compiled-binary-supplied-to-packager"
            )
            archive = Path(directory) / manifest["archive"]
            self.assertTrue(archive.is_file())
            self.assertEqual(manifest["archive_sha256"], sha256(archive))
            self.assertEqual(manifest["archive_bytes"], archive.stat().st_size)

            with tarfile.open(archive) as tar:
                names = set(tar.getnames())
            for expected in (
                "./usr/bin/zigbee2mqtt-rs",
                "./etc/init.d/zigbee2mqtt-rs",
                "./etc/zigbee2mqtt-rs/config.yaml",
            ):
                self.assertIn(expected, names)

    def test_rejects_non_aarch64_binary(self):
        host_binary = shutil.which("true") or "/bin/true"
        with tempfile.TemporaryDirectory() as directory:
            completed = subprocess.run(
                ["bash", str(SCRIPT), host_binary, directory],
                capture_output=True,
                text=True,
                check=False,
                cwd=str(ROOT),
            )
            self.assertEqual(completed.returncode, 2)
            self.assertIn("AArch64", completed.stderr)
            self.assertEqual(list(Path(directory).glob("*.tar.gz")), [])


if __name__ == "__main__":
    unittest.main()
