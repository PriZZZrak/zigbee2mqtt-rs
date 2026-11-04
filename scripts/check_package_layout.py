#!/usr/bin/env python3
"""Validate a staged herdsman-converters package before publication.

Checks the runtime layout expected by src/converters/external.rs:
registry lookup candidates must exist under devices/, helper directories must be
present, zcl.json and manifest.json must be valid, and no TypeScript sources may
leak into the artifact.
"""

import json
import sys
from pathlib import Path

REQUIRED_DIRS = ("devices", "converters", "lib")


def validate(root: Path, expected_ref=None):
    errors = []
    if not root.is_dir():
        return [f"package directory does not exist: {root}"]

    for name in REQUIRED_DIRS:
        directory = root / name
        if not directory.is_dir():
            errors.append(f"missing required directory: {name}/")
        elif not any(directory.rglob("*.js")):
            errors.append(f"{name}/ contains no JavaScript files")

    for name in ("registry.json", "zcl.json", "zcl-commands.json", "manifest.json"):
        if not (root / name).is_file():
            errors.append(f"missing {name}")

    registry = None
    try:
        registry = json.loads((root / "registry.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"registry.json is unreadable: {error}")

    if isinstance(registry, dict):
        lookup = registry.get("lookup")
        if not isinstance(lookup, dict) or not lookup:
            errors.append("registry.json has no lookup index")
        else:
            missing = 0
            for key, candidates in lookup.items():
                if not isinstance(candidates, list):
                    errors.append(f"lookup[{key!r}] is not a candidate list")
                    continue
                for candidate in candidates:
                    if (
                        not isinstance(candidate, list)
                        or len(candidate) != 2
                        or not isinstance(candidate[0], str)
                        or not isinstance(candidate[1], int)
                    ):
                        errors.append(f"lookup[{key!r}] has a malformed candidate")
                        continue
                    file, _index = candidate
                    if not (root / "devices" / file).is_file():
                        missing += 1
            if missing:
                errors.append(f"{missing} lookup candidate device file(s) missing under devices/")
        if not isinstance(registry.get("fingerprints"), list):
            errors.append("registry.json has no fingerprints list")
    elif registry is not None:
        errors.append("registry.json root must be an object")

    zcl_types = None
    try:
        zcl_types = json.loads((root / "zcl-commands.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"zcl-commands.json is unreadable: {error}")
    if isinstance(zcl_types, dict):
        clusters = zcl_types.get("clusters")
        if not isinstance(clusters, dict) or not clusters:
            errors.append("zcl-commands.json has no clusters table")
        else:
            gen_on_off = clusters.get("6")
            if not isinstance(gen_on_off, dict) or "attributes" not in gen_on_off:
                errors.append("zcl-commands.json is missing genOnOff (cluster 6)")
    elif zcl_types is not None:
        errors.append("zcl-commands.json root must be an object")

    manifest = None
    try:
        manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"manifest.json is unreadable: {error}")
    if isinstance(manifest, dict):
        if manifest.get("schema") != 1:
            errors.append("manifest.json has unsupported schema")
        if not isinstance(manifest.get("files"), list) or not manifest["files"]:
            errors.append("manifest.json has no file list")
        if expected_ref and manifest.get("ref") != expected_ref:
            errors.append(
                f"manifest ref {manifest.get('ref')!r} does not match {expected_ref!r}"
            )
        for entry in manifest.get("files", []):
            if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
                errors.append("manifest.json contains a malformed file entry")
                continue
            if not (root / entry["path"]).is_file():
                errors.append(f"manifest entry missing on disk: {entry['path']}")

    for stale in root.rglob("*.ts"):
        # `dist/lib` legitimately ships `.d.ts` declarations next to the JS;
        # only reject real TypeScript sources.
        if stale.name.endswith(".d.ts"):
            continue
        errors.append(f"TypeScript source leaked into package: {stale.relative_to(root)}")

    return errors


def main(argv):
    if len(argv) not in (2, 3):
        print("usage: check_package_layout.py <package-dir> [expected-ref]", file=sys.stderr)
        return 2
    root = Path(argv[1])
    expected = argv[2] if len(argv) == 3 else None
    errors = validate(root, expected)
    if errors:
        for error in errors:
            print(f"layout error: {error}", file=sys.stderr)
        return 1
    print(f"package layout OK: {root}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
