# Converter artifact research scripts

Ad-hoc, offline measurement tooling used by
[`docs/converter-artifact-research.md`](../../docs/converter-artifact-research.md).
They never touch the coordinator, MQTT or Home Assistant.

- `measure-node-load.js` — cold Node load of the package root/shared modules/
  individual device files and the TS0601 resolution path. Defaults to
  `/opt/zigbee2mqtt/node_modules/zigbee-herdsman-converters`; set `ROOT` to the
  local checkout for x86 runs.
- `build-rich-index.js` — build-time prototype of the fingerprint-aware index
  (candidate definitions with model, vendor, zigbeeModel and fingerprint
  fields). Writes `/tmp/opencode/rich-index.json`.
- `split-tuya.py` — scans `dist/devices/tuya.js` and reports definition element
  ranges/sizes.
- `generate-tuya-split.py` — generates a shared preamble module plus eight
  tuya fragments in `/tmp/opencode/tuya-split` and writes fragment metadata.
- `verify-tuya-split.js` / `verify-tuya-prepared.js` — compare raw and
  `prepareDefinition` results of original vs fragments (571/571 expected).

These are research prototypes, not production artifact builders. The
production generator will be a pinned build-time Node tool that emits the
binary index and manifest described in the research document.
