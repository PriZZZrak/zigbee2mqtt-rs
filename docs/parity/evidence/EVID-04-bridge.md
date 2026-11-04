# EVID-04 — bridge-level MQTT differential

Implementation: `scripts/evid04_node_reference.js` (mode `bridge`),
`src/bin/evid04_bridge_candidate.rs`, Cargo feature `test-support`,
`Bridge::process_zcl_message_for_diff`, and the bridge fixes it found.
Related: MQTT-003, EVID-004, CONVERTER-005.

## Source contract

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2`, converters
`9dabbe4e343573c5a3070605efd843a1b3a16058`:

- `lib/extension/receive.ts:137` — converter payloads go through
  `zhc.postProcessConvertedFromZigbeeMessage(definition, payload, options, zh)`.
- `lib/controller.ts:382–450` (`publishEntityState`) — `state.set` merges the
  payload into the cached state; with `advanced.cache_state` (**default true**,
  `lib/util/settings.ts:108`) the full cached state is published;
  `linkquality` and `last_seen` are added afterwards when enabled
  (`lib/util/settings.ts:111` defaults `last_seen` to `disable`).
- Topic is `<mqtt.base_topic>/<entity.name>`.
- Converter endpoint suffixes are produced by the converter itself via
  `postfixWithEndpointName` (`converters/dist/lib/utils.js:251–262`); Zigbee2MQTT
  core does **not** add another suffix.

## Harness

- Node reference (`mode: "bridge"`): resolves the definition via
  `models-index.json`, prepares it with upstream `prepareDefinition`, runs the
  matching `fromZigbee` converters, calls the real
  `postProcessConvertedFromZigbeeMessage`, merges `input.state` (cache) and
  adds `linkquality`. It models the upstream **default** `last_seen: disable`:
  when the converter produces no keys, the reference publishes nothing
  (`payload: null`). Output: `{topic, payload}`.
- Rust candidate: builds the same ZCL frame from the fixture, registers a
  `Device`, starts the in-process MQTT broker, drives the real
  `Bridge::process_zcl_message` (the function the event loop calls) and prints
  the observed `{topic, payload}`.
- Commands whose converters only answer the frame (time sync, gateway status,
  weather request) or have no converter (OTA) compare `payload: null` on both
  sides. For Tuya weather the reference and Rust host both hand the converter a
  `Buffer`; time sync stays an array.
- The harness compares normalized JSON; state fixtures ignore only
  `payload.last_seen` (a wall-clock value, because Rust always adds it while the
  reference uses upstream's default `disable`). No other field is normalized.

## Fixtures and result

| fixture | published payload (after ignoring last_seen) |
| --- | --- |
| `bridge-tuya-trv-report` (`commandDataReport`, dp 3) | `{"local_temperature":14,"linkquality":42}` |
| `bridge-multi-endpoint-ts0002` (endpoint 2, cached `state_l1`) | `{"state_l1":"OFF","state_l2":"ON","linkquality":42}` |
| `bridge-snzb02-temperature` (cached `battery`) | `{"battery":90,"temperature":22.5,"linkquality":42}` |
| `bridge-tuya-mcu-sync-time` | no state published (`payload: null`) |
| `bridge-tuya-gateway-status` | no state published (`payload: null`) |
| `bridge-tuya-weather-request` | no state published (`payload: null`) |
| `bridge-tuya-ota-block` | no state published (`payload: null`; Rust logs explicit OTA rejection) |

```text
$ python3 scripts/evid04_harness.py \
    --fixtures docs/parity/evidence/evid04/bridge-fixtures \
    --reference-cmd "env HC_CONVERTERS_DIST=.../zigbee-herdsman-converters/dist \
        EVID04_REGISTRY=.../registry.json node scripts/evid04_node_reference.js" \
    --candidate-cmd "env EVID04_CACHE=... ./target/release/evid04_bridge_candidate" \
    --require-pins --report docs/parity/reports/evid04-bridge.json
EVID-04: 7 passed, 0 failed,
pins=[('zigbee2mqtt','MATCH'),('zigbee-herdsman','MATCH'),
      ('zigbee-herdsman-converters','MATCH'),('frontend','MATCH')]
```

The report is checked in at `docs/parity/reports/evid04-bridge.json`.

## Defects found and fixed by this differential

1. **Double endpoint suffix.** The bridge suffixed every merged key, so a
   converter that already produced `state_l2` (upstream
   `postfixWithEndpointName`) became `state_l2_l2`. Only native
   cluster-fallback values are suffixed now, before converter output is merged.
2. **Boolean attributes reached converters as `true`/`false`.** Herdsman reads
   BOOLEAN/DATA8/BITMAP8/UINT8/ENUM8 with `readUInt8()` (numbers 0/1), and
   definitions such as `TS0002` compare `msg.data.onOff === 1`. The bridge now
   converts parsed booleans to 0/1 before dispatch; `TS0002` consequently
   publishes `state_l2: "ON"` instead of `"OFF"`.

Regression tests: `converter_attribute_json_uses_herdsman_numeric_booleans`,
`endpoint_suffix_applies_only_to_native_values`.

## Gaps

- `payload.last_seen` is ignored: the Rust bridge always includes it, while
  upstream defaults to `disable`. That divergence is known and not resolved here.
- For time sync / gateway status / weather, the converter’s response command is
  forwarded by the bridge (CONVERTER-020); the bridge differential observes only
  the MQTT side, while the wire frame and the requested commands are covered by
  the fake-coordinator test and the `fromZigbeeOps` differential.
- Not compared: retain/QoS flags, availability topic, Home Assistant
  `adjustMessageBeforePublish` mock properties, debounce/throttle, groups.
- One frame per fixture, no persistence/restart, no live broker or HA.
