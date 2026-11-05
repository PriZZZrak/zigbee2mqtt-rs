import contextlib
import importlib.util
import io
import tarfile
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "check_ipk.py"
SPEC = importlib.util.spec_from_file_location("check_ipk", MODULE_PATH)
check_ipk = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check_ipk)


def _tar_bytes(entries):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
        for name, data, mode in entries:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = mode
            archive.addfile(info, io.BytesIO(data))
    return buffer.getvalue()


def _elf_aarch64(interp: bool = False) -> bytes:
    header = bytearray(64)
    header[0:4] = b"\x7fELF"
    header[18:20] = (0xB7).to_bytes(2, "little")
    payload = bytes(header)
    if interp:
        payload += b"/lib/ld-linux-aarch64.so.1\x00"
    return payload


def build_ipk(
    path: Path,
    *,
    binary: bytes | None = None,
    conffiles: str = "/etc/zigbee2mqtt-rs/config.yaml\n",
    depends: str = "libc, mosquitto",
    architecture: str = "aarch64_cortex-a53",
    include_binary: bool = True,
    include_config: bool = True,
) -> Path:
    control = (
        "Package: zigbee2mqtt-rs\n"
        "Version: 0.1.0-r1\n"
        f"Depends: {depends}\n"
        f"Architecture: {architecture}\n"
        "Installed-Size: 12011520\n"
        "Description: Zigbee to MQTT bridge written in Rust.\n"
        " The binary is statically linked.\n"
    ).encode()
    control_entries = [
        ("./control", control, 0o644),
        ("./conffiles", conffiles.encode(), 0o644),
        ("./postinst", b"#!/bin/sh\n", 0o755),
        ("./prerm", b"#!/bin/sh\n", 0o755),
    ]
    data_entries = [
        ("./etc/init.d/zigbee2mqtt-rs", b"#!/bin/sh\n", 0o755),
        ("./www/zigbee2mqtt/index.html", b"<html></html>", 0o644),
    ]
    if include_config:
        data_entries.append(("./etc/zigbee2mqtt-rs/config.yaml", b"mqtt: {}\n", 0o600))
    if include_binary:
        data_entries.append(
            ("./usr/bin/zigbee2mqtt-rs", binary or _elf_aarch64(), 0o755)
        )
    outer = io.BytesIO()
    with tarfile.open(fileobj=outer, mode="w:gz") as archive:
        for name, payload in (
            ("debian-binary", b"2.0\n"),
            ("control.tar.gz", _tar_bytes(control_entries)),
            ("data.tar.gz", _tar_bytes(data_entries)),
        ):
            info = tarfile.TarInfo(name)
            info.size = len(payload)
            info.mode = 0o644
            archive.addfile(info, io.BytesIO(payload))
    path.write_bytes(outer.getvalue())
    return path


class CheckIpkTests(unittest.TestCase):
    def test_valid_package_passes(self):
        with tempfile.TemporaryDirectory() as directory:
            path = build_ipk(Path(directory) / "pkg.ipk")
            summary = check_ipk.check_ipk(path)
            self.assertEqual(summary["package"], "zigbee2mqtt-rs")
            self.assertEqual(summary["architecture"], "aarch64_cortex-a53")
            self.assertEqual(summary["installed_size"], 12011520)
            self.assertEqual(summary["conffiles"], ["/etc/zigbee2mqtt-rs/config.yaml"])
            self.assertGreater(summary["binary_bytes"], 0)

    def test_missing_conffile_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = build_ipk(Path(directory) / "pkg.ipk", conffiles="\n")
            with self.assertRaises(check_ipk.IpkError) as raised:
                check_ipk.check_ipk(path)
            self.assertIn("conffiles", str(raised.exception))

    def test_non_aarch64_binary_is_rejected(self):
        x86 = bytearray(_elf_aarch64())
        x86[18:20] = (0x3E).to_bytes(2, "little")
        with tempfile.TemporaryDirectory() as directory:
            path = build_ipk(Path(directory) / "pkg.ipk", binary=bytes(x86))
            with self.assertRaises(check_ipk.IpkError) as raised:
                check_ipk.check_ipk(path)
            self.assertIn("AArch64", str(raised.exception))

    def test_dynamic_interpreter_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = build_ipk(
                Path(directory) / "pkg.ipk", binary=_elf_aarch64(interp=True)
            )
            with self.assertRaises(check_ipk.IpkError) as raised:
                check_ipk.check_ipk(path)
            self.assertIn("dynamic interpreter", str(raised.exception))

    def test_missing_binary_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = build_ipk(Path(directory) / "pkg.ipk", include_binary=False)
            with self.assertRaises(check_ipk.IpkError) as raised:
                check_ipk.check_ipk(path)
            self.assertIn("usr/bin/zigbee2mqtt-rs", str(raised.exception))

    def test_missing_config_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = build_ipk(Path(directory) / "pkg.ipk", include_config=False)
            with self.assertRaises(check_ipk.IpkError) as raised:
                check_ipk.check_ipk(path)
            self.assertIn("config.yaml", str(raised.exception))

    def test_missing_mosquitto_dependency_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = build_ipk(Path(directory) / "pkg.ipk", depends="libc")
            with self.assertRaises(check_ipk.IpkError) as raised:
                check_ipk.check_ipk(path)
            self.assertIn("mosquitto", str(raised.exception))

    def test_wrong_architecture_field_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = build_ipk(Path(directory) / "pkg.ipk", architecture="x86_64")
            with self.assertRaises(check_ipk.IpkError) as raised:
                check_ipk.check_ipk(path)
            self.assertIn("architecture", str(raised.exception))

    def test_cli_exit_codes(self):
        with tempfile.TemporaryDirectory() as directory:
            good = build_ipk(Path(directory) / "good.ipk")
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                self.assertEqual(check_ipk.main([str(good)]), 0)
                self.assertEqual(check_ipk.main([str(good), "--json"]), 0)
            self.assertIn("ipk OK", output.getvalue())
            self.assertIn('"package": "zigbee2mqtt-rs"', output.getvalue())
            bad = Path(directory) / "bad.ipk"
            bad.write_bytes(b"not a package")
            errors = io.StringIO()
            with contextlib.redirect_stderr(errors):
                self.assertEqual(check_ipk.main([str(bad)]), 1)
            self.assertIn("ipk check failed", errors.getvalue())


if __name__ == "__main__":
    unittest.main()
