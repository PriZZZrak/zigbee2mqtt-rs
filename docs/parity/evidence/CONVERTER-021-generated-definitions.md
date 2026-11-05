# CONVERTER-021 — generated definitions for unknown devices

Implementation: `ExternalConverter::generated_definition_payload`,
`__definitionPayloadJson` in `src/converters/external.rs`,
`src/bin/evid04_generated_candidate.rs`,
`scripts/generate_definition_reference.js`,
`scripts/generated_definition_diff.py`.
Behavior: CONVERTER-021; related FRONTEND-004, CONVERTER-012.

## Source contract

- `zigbee-herdsman-converters/dist/index.js:411-482` — Zigbee2MQTT resolves a
  definition with `findByDevice(zh, true)`: when no index candidate/fingerprint
  matches, `findDefinition` calls `generateDefinition` from
  `lib/generateDefinition.js` instead of leaving the device undefined.
- `lib/generateDefinition.js` builds `extend` entries from the device's endpoint
  clusters (temperature/pressure/humidity/CO2/illuminance/occupancy/on-off/
  level/color/cover/fan/thermostat/identify/analog & binary input/output/...),
  adds `deviceEndpoints` when more than one endpoint exists, and returns a
  definition with `generated: true`, `model = modelID`, `vendor =
  manufacturerName`, `description = "Automatically generated definition"`.
- z2m `lib/model/device.ts` — `supported` is
  `type === "Coordinator" || (definition && !definition.generated)`, so
  generated definitions are shown to the frontend but not marked supported;
  a definition is resolved after join/failed interview, but not while the
  interview is still `IN_PROGRESS`.

## Rust change

- `generated_definition_payload` loads `lib/generateDefinition.js` through the
  same deterministic CJS loader used for conversion, builds a device object with
  real endpoint cluster lists (`ExternalDeviceInfo::endpoints`), calls the
  generator, drains Boa's promise jobs, then reuses
  `__definitionPayloadJson(..., source: "generated")` and appends the generic
  linkquality expose.
- The bridge publishes the definition when the interview is not `IN_PROGRESS`
  (including failed interviews, matching upstream), skips definitions entirely
  for streaming interviews, and caches payloads per identity/fingerprint.
- `supported` is now `definition exists && source != "generated"`.

## Differential

`scripts/generated_definition_diff.py` compares the Rust generator with the
pinned Node generator for five fixtures (temperature sensor, heating actuator,
multi-endpoint unknown device, no-endpoint unknown device, external-converter
device). The Node reference builds endpoints with the **full** `Zcl.Clusters`
name map, because the app's vitest mock only maps a subset of clusters and would
otherwise hide `genIdentify`/`genGroups` extenders.

```text
$ python3 scripts/generated_definition_diff.py \
    --fixtures docs/parity/evidence/generated-definitions \
    --reference-cmd "env Z2M_APP=... HC_CONVERTERS_DIST=... node scripts/generate_definition_reference.js" \
    --candidate-cmd "env EVID04_CACHE=... ./target/.../evid04_generated_candidate" \
    --require-pins --report docs/parity/reports/generated-definitions.json
GENERATED: 5 passed, 0 failed, pins=[all four MATCH]
```

The bridge API differential then compares generated devices by marker
(`source: "generated"`) and `supported`, and passes 5/5.

## Gaps

- Only the cluster families listed in upstream `INPUT_EXTENDERS`/
  `OUTPUT_EXTENDERS` are exercised by the fixtures; a new upstream extender must
  be re-tested when the pinned converter package moves.
- GreenPower generated definitions (`generateGreenPowerDefinition`) are not
  covered.
- No hardware run with a genuinely unknown device.
