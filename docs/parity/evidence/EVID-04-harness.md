# EVID-04 — reference/differential harness

Implementation: `scripts/evid04_harness.py`, smoke fixture
`scripts/tests/fixtures/evid04_smoke/temperature.json`, fake reference
`scripts/tests/fake_evid04_reference.py`, tests
`scripts/tests/test_evid04_harness.py`, CI step in `.github/workflows/rust-ci.yml`.

## Contract

A fixture is a JSON file:

```json
{
  "id": "case-id",
  "ignore": ["last_seen", "*.private"],
  "sort": ["actions"],
  "input": { "...": "..." }
}
```

The reference and candidate commands receive the fixture `input` on stdin and
must print JSON. The harness compares the outputs after:

- removing only explicitly listed dotted paths (`last_seen`, `nested.field`,
  `*.private` style wildcards);
- sorting only explicitly listed arrays by canonical JSON.

There is no implicit sorting, timestamp stripping or numeric tolerance. A
mismatch prints both normalized values and exits non-zero; a machine-readable
report is written with `--report`.

## Pins

`--lock docs/parity/upstream-lock.json` loads baseline commits and compares them
with `.upstream/<repo>` HEADs. Missing checkouts or differing commits are
`UNVERIFIED`/`MISMATCH`. `--require-pins` fails in that case. CI runs the fixture
smoke with `--skip-pins` because reference checkouts are not present; a dedicated
reference job must use `--require-pins` when they are.

## Commands

```bash
# Validate the harness only (fake reference, NOT parity evidence)
python3 scripts/evid04_harness.py \
  --fixtures scripts/tests/fixtures/evid04_smoke \
  --reference-cmd "python3 scripts/tests/fake_evid04_reference.py reference" \
  --candidate-cmd "python3 scripts/tests/fake_evid04_reference.py candidate" \
  --skip-pins --report /tmp/evid04-smoke.json

# Real reference run (once Node/pnpm and pinned checkouts exist)
python3 scripts/evid04_harness.py \
  --fixtures docs/parity/evidence/evid04/fixtures \
  --reference-cmd "./scripts/run-node-reference.sh" \
  --candidate-cmd "./scripts/run-rust-candidate.sh" \
  --require-pins --report docs/parity/reports/evid04.json
```

The fake reference deliberately differs in the ignored timestamp and array order;
the smoke case proves normalization and mismatch handling, not Zigbee2MQTT
behavior.

## Validation in this environment

- `python3 -m unittest discover -s scripts/tests` → **24 passed**, including
  normalization, wildcard ignore, pass/mismatch exit codes and pin
  MATCH/MISMATCH against a temporary git repository.
- `python3 scripts/evid04_harness.py ... --skip-pins` → `PASS 1 0`, report
  written.
- CI runs the 24 Python tests and the smoke harness with a fake reference,
  uploading `evid04-smoke.json`.

## Gaps (not parity evidence)

- Node/pnpm, pinned reference checkouts and a real fixture corpus are unavailable
  here; no reference capture has been executed.
- No end-to-end Node↔Rust comparison for MQTT, ZCL, storage, converters or
  frontend yet.
- The harness does not start a broker, coordinator or Home Assistant; it compares
  command JSON only. Integrations need separate drivers feeding raw artifacts.
- CI's smoke step is explicitly named as fake and must not be counted as parity.
