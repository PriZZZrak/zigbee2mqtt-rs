# CV-01/CV-02 — converter package layout and manifest

Rust/runtime contract: `src/converters/external.rs` fetches
`devices/<file>` and resolves relative requires into `converters/` and `lib/`.
Build: `.github/workflows/convert-converters.yml`,
`scripts/gen-manifest.mjs`, `scripts/check_package_layout.py`. Behavior:
CONVERTER-013; plan CV-01/CV-02.

## Problem confirmed by live HTTP

At `2026-11-03T21:05:22Z` the published `latest/` served `lytko.js` at the root,
while `devices/lytko.js` and `converters/fromZigbee.js` returned 404 (recorded in
`AUDIT-20261104.md`). The workflow flattened `dist/devices/*` into the package
root and merged `dist/converters/*` + `dist/lib/*` into one `lib/` directory.
The Rust runtime asks the registry for a device module and fetches
`<root>/devices/<file>`, so cold-cache delivery could not work.

## Change

- Stage `dist/devices/*.js` under `devices/` and `dist/converters/*.js` under
  `converters/`, preserving relative imports used by compiled device modules.
- Copy the whole `dist/lib/` tree (not only `*.js`) to `lib/`.
- Generate `manifest.json` with schema version, source ref, timestamp, file
  counts, per-file byte length and SHA-256 via `scripts/gen-manifest.mjs`.
- Validate the staged package before publishing with
  `scripts/check_package_layout.py`:
  required directories and JavaScript, `registry.json` lookup candidates exist
  under `devices/`, `zcl.json`, manifest schema/file list/ref, and no leaked
  `.ts` files.
- Keep the existing single gh-pages publication step; `latest` is replaced by
  copying the validated version directory.

## Verification available in this environment

No Node/pnpm is installed, so the workflow and `gen-manifest.mjs` were **not**
executed locally. The layout validator is pure Python and was tested:

```text
python3 -m unittest discover -s scripts/tests
Ran 19 tests ... OK
python3 scripts/check_package_layout.py /nonexistent
layout error: package directory does not exist: /nonexistent
exit 1
```

Tests cover valid package, missing lookup candidate, missing helper JS, leaked
TypeScript, manifest ref mismatch and a manifest entry missing on disk.

## Not yet verified

- No GitHub Actions run has published this layout; the live `latest` still has
  the old structure until the workflow runs.
- `gen-manifest.mjs` and the shell staging steps are unexecuted here; their
  correctness is code-reviewed, not proven by a successful artifact.
- No Node smoke test loads a staged device module from a clean directory yet
  (CV-02 still requires it).
- Package size/digest budget on the Routerich 54 MB storage and atomic
  promotion/concurrent-writer tests remain open.
- Version/digest metadata is not yet consumed by the Rust client.
