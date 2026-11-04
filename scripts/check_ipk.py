#!/usr/bin/env python3
"""Validate a zigbee2mqtt-rs OpenWrt ``.ipk`` package.

The package is produced by the OpenWrt SDK from the static aarch64 binary in
``openwrt/files``. This checker verifies the facts that packaging can silently
break and that install/upgrade cannot repair:

* outer ``.ipk`` layout (``debian-binary``, ``control.tar.gz``, ``data.tar.gz``);
* control metadata (package name, version, architecture, dependencies,
  installed size);
* the config file is registered in ``conffiles`` (upgrades preserve it);
* the payload contains the procd init script, the example config and the
  static aarch64 ELF binary at ``/usr/bin/zigbee2mqtt-rs``;
* the binary is AArch64 and has no dynamic interpreter.

It does **not** install the package: install/upgrade/downgrade behaviour requires
the target device. Usage::

    python3 scripts/check_ipk.py path/to/package.ipk [--json]
"""

from __future__ import annotations

import argparse
import io
import json
import sys
import tarfile
from pathlib import Path

REQUIRED_CONTROL = {"Package", "Version", "Architecture", "Depends", "Installed-Size"}
REQUIRED_DATA = {
    "./usr/bin/zigbee2mqtt-rs",
    "./etc/init.d/zigbee2mqtt-rs",
    "./etc/zigbee2mqtt-rs/config.yaml",
}
CONFFILE = "/etc/zigbee2mqtt-rs/config.yaml"
ELF_MACHINE_AARCH64 = 0xB7
ELF_INTERP = b"/lib/ld-"


class IpkError(Exception):
    """Raised when the package violates the expected layout or metadata."""


def _read_tar_member(archive: tarfile.TarFile, name: str) -> bytes:
    member = archive.getmember(name)
    handle = archive.extractfile(member)
    if handle is None:  # pragma: no cover - directories only
        raise IpkError(f"{name} is not a regular file")
    return handle.read()


def _parse_control(text: str) -> dict[str, str]:
    fields: dict[str, str] = {}
    key = None
    for line in text.splitlines():
        if not line.strip():
            key = None
            continue
        if line.startswith((" ", "\t")) and key is not None:
            fields[key] += "\n" + line.strip()
            continue
        if ":" not in line:
            raise IpkError(f"malformed control line: {line!r}")
        key, value = line.split(":", 1)
        fields[key.strip()] = value.strip()
    return fields


def check_ipk(path: Path) -> dict:
    """Return a summary dict or raise :class:`IpkError`."""
    if not path.is_file():
        raise IpkError(f"package not found: {path}")

    with tarfile.open(path, "r:gz") as outer:
        # OpenWrt's opkg-build emits members with a leading "./".
        members = {name.lstrip("./"): name for name in outer.getnames()}
        for required in ("debian-binary", "control.tar.gz", "data.tar.gz"):
            if required not in members:
                raise IpkError(f"missing outer member {required}")
        version = (
            _read_tar_member(outer, members["debian-binary"])
            .decode("ascii", "replace")
            .strip()
        )
        control_bytes = _read_tar_member(outer, members["control.tar.gz"])
        data_bytes = _read_tar_member(outer, members["data.tar.gz"])

    with tarfile.open(fileobj=io.BytesIO(control_bytes), mode="r:gz") as control:
        control_names = {name.lstrip("./") for name in control.getnames()}
        if "control" not in control_names:
            raise IpkError("control.tar.gz has no control file")
        fields = _parse_control(
            _read_tar_member(control, "./control").decode("utf-8")
            if "./control" in control.getnames()
            else _read_tar_member(control, "control").decode("utf-8")
        )
        conffiles_name = "./conffiles" if "./conffiles" in control.getnames() else "conffiles"
        conffiles = (
            _read_tar_member(control, conffiles_name).decode("utf-8").split()
            if conffiles_name in control.getnames()
            else []
        )

    missing = REQUIRED_CONTROL - set(fields)
    if missing:
        raise IpkError(f"control is missing fields: {', '.join(sorted(missing))}")
    if fields["Package"] != "zigbee2mqtt-rs":
        raise IpkError(f"unexpected package name {fields['Package']!r}")
    if not fields["Version"]:
        raise IpkError("control Version is empty")
    if not fields["Architecture"].startswith("aarch64"):
        raise IpkError(f"unexpected architecture {fields['Architecture']!r}")
    if "mosquitto" not in fields["Depends"]:
        raise IpkError(f"Deps must include mosquitto, got {fields['Depends']!r}")
    try:
        installed_size = int(fields["Installed-Size"])
    except ValueError as error:
        raise IpkError(f"Installed-Size is not an integer: {fields['Installed-Size']!r}") from error
    if installed_size <= 0:
        raise IpkError("Installed-Size must be positive")
    if CONFFILE not in conffiles:
        raise IpkError(f"{CONFFILE} is not in conffiles (upgrades would overwrite config)")

    with tarfile.open(fileobj=io.BytesIO(data_bytes), mode="r:gz") as data:
        data_names = set(data.getnames())
        missing_data = REQUIRED_DATA - data_names
        if missing_data:
            raise IpkError(f"data is missing: {', '.join(sorted(missing_data))}")
        binary_name = "./usr/bin/zigbee2mqtt-rs"
        binary = _read_tar_member(data, binary_name)
        mode = data.getmember(binary_name).mode
        web_bytes = sum(
            member.size
            for member in data.getmembers()
            if member.isfile() and member.name.startswith("./www/")
        )

    if not binary.startswith(b"\x7fELF"):
        raise IpkError("payload binary is not an ELF file")
    if len(binary) < 20:
        raise IpkError("payload binary is truncated")
    machine = int.from_bytes(binary[18:20], "little")
    if machine != ELF_MACHINE_AARCH64:
        raise IpkError(f"payload binary is not AArch64 (e_machine=0x{machine:04x})")
    if ELF_INTERP in binary:
        raise IpkError("payload binary has a dynamic interpreter; expected static linking")
    if mode & 0o111 == 0:
        raise IpkError("payload binary is not executable")

    return {
        "package": fields["Package"],
        "version": fields["Version"],
        "architecture": fields["Architecture"],
        "depends": fields["Depends"],
        "installed_size": installed_size,
        "data_bytes": len(data_bytes),
        "binary_bytes": len(binary),
        "web_bytes": web_bytes,
        "abi_version": version,
        "conffiles": conffiles,
        "payload_members": sorted(
            name for name in data_names if not name.startswith("./www/")
        ),
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path)
    parser.add_argument("--json", action="store_true", help="print the summary as JSON")
    args = parser.parse_args(argv)
    try:
        summary = check_ipk(args.package)
    except (IpkError, tarfile.TarError, OSError) as error:
        print(f"ipk check failed: {error}", file=sys.stderr)
        return 1
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(
            "ipk OK: {package} {version} ({architecture}); "
            "installed {installed_size} B, data {data_bytes} B, "
            "binary {binary_bytes} B, web {web_bytes} B".format(**summary)
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
