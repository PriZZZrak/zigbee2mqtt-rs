# FRONTEND-004 — bridge MQTT API differential

Implementation: `scripts/api_diff.py`, `src/bin/evid04_api_candidate.rs`
(`--features test-support`), checked-in upstream dump
`docs/parity/evidence/api/upstream-dump.json`, generator
`docs/parity/evidence/api/api_dump_reference.test.ts`,
`docs/parity/evidence/api/cluster-map.json`.
Converter work: `ExternalConverter::definition_payload`,
`PREPARE_DEFINITION_JS` in `src/converters/external.rs`.
Behavior: FRONTEND-004; related MQTT-002/MQTT-003/MQTT-005, CONVERTER-020/021,
EVID-004.

## Contract

The frontend consumes four bridge MQTT surfaces (pinned app
`344b6e03b43644c97154a72ca9eae17193c553a2`):

- `<base>/bridge/state` — `{"state": "online" | "offline"}`.
- `<base>/bridge/info` — `lib/extension/bridge.ts:publishInfo`: `version`,
  `commit`, `coordinator{ieee_address,type,meta}`, `network{pan_id,
  extended_pan_id,channel}`, `mqtt{server,version}`, `os{cpus,memory_mb,
  version,node_version}`, `log_level`, `permit_join`, `restart_required`,
  `config`, `config_schema`, `zigbee_herdsman`, `zigbee_herdsman_converters`.
- `<base>/bridge/devices` — `publishDevices`: every herdsman device including the
  coordinator; per device `ieee_address`, `type`, `network_address`,
  `supported`, `friendly_name`, `disabled`, `description`, `definition`,
  `power_source`, `software_build_id`, `date_code`, `model_id`, `interviewing`,
  `interview_completed`, `interview_state`, `manufacturer`, `endpoints`.
  `definition` is `getDefinitionPayload`: `source`, `model`, `vendor`,
  `description`, `exposes` (device resolves function exposes and always appends
  the generic linkquality expose), `supports_ota`, `options`, `version`, `icon`.
- `<base>/bridge/response/permit_join` — `{status:"ok",data:{time}}`.

## Upstream dump

`api_dump_reference.test.ts` is copied into the pinned app checkout, starts the
real `Controller` with the repository's vitest herdsman/MQTT mocks (120 s hook
timeout), and records the latest payload for those topics plus mock device
metadata (identity, `manufId`, `swBuildId`, `dateCode`, endpoints). The dump is
checked in; no network or broker is needed.

Regeneration:

```bash
API_DATA_DIR=/tmp/.../z2m-api-data API_DUMP_PATH=.../upstream-dump.json \
  pnpm vitest run --config ./test/vitest.config.mts test/extensions/api_dump.test.ts
```

## Rust candidate

`evid04_api_candidate --data <dir>` loads `configuration.yaml`/`database.db`
through the bridge importer, starts an in-process MQTT broker plus a fake ZNP
coordinator, and publishes/answers through the real bridge functions. The
converter package can be pointed at a staged build with `Z2M_CONVERTERS_DIR`
(harness-only, `test-support`).

## Normalization

- The dump's synthetic mock devices with a non-hex IEEE or NWK > `0xFFFF` are
  excluded from both sides (neither importer can persist them; they cannot exist
  on a real network).
- `bridge/info`: `coordinator` (ieee/type), `log_level`, `permit_join`,
  `restart_required`, `network`, `mqtt`, the `advanced`/`mqtt.base_topic` config
  subset, and `zigbee_herdsman_converters.version` are compared exactly. `os`
  values, `version`, `commit` and the herdsman version are environment/build
  identity and are only checked for presence/shape; `config_schema` is not
  published by Rust yet. `mqtt.version` accepts 4 or 5 because the Rust transport
  is MQTT 3.1.1 (protocol 4) while the upstream test client reports 5 (MQTT 5
  transport is not implemented).
- `bridge/devices`: identity/metadata plus the coordinator entry and the set of
  endpoint ids; endpoint clusters/bindings/reportings/scenes remain covered by
  the storage differential.
- `bridge-devices-definitions`: `definition` + `supported` per device.
  Generated definitions are compared by marker (`source: "generated"`) because
  the app's test mocks resolve endpoint clusters through a partial name map; the
  content is verified with full ZCL names by
  `scripts/generated_definition_diff.py` (CONVERTER-021).
- `scripts/tests/test_api_diff.py` covers the harness (matching run, renamed
  device failure, known-gap handling).

## Result

```text
$ Z2M_CONVERTERS_DIR=<staged 26.115.1> python3 scripts/api_diff.py \
    --upstream docs/parity/evidence/api/upstream-dump.json \
    --candidate-cmd "./target/release/evid04_api_candidate" \
    --require-pins \
    --report docs/parity/reports/api.json
API: 5 passed, 0 failed, pins=[all four MATCH]

bridge-state                 PASS
bridge-info                  PASS
bridge-devices               PASS   (coordinator + 62 devices)
bridge-devices-definitions   PASS   (native exact; generated marker + supported)
permit-join-response         PASS
```

The report is checked in at `docs/parity/reports/api.json`. Generated definition
content is covered by `docs/parity/reports/generated-definitions.json` (5/5).

## Defects found and fixed by this differential

1. **`bridge/request/permit_join` produced no response** → now answers
   `{status:"ok",data:{time}}`.
2. **Device payload lacked `endpoints`/`interview_state`** and dropped persisted
   `type`/`interviewState` on import (battery `EndDevice` reported as `Router`).
3. **`bridge/devices` was silently not published** for real networks: rumqttc's
   10 KiB outgoing limit dropped the payload. Now 256 MiB with a regression
   test.
4. **`bridge/info` was a stub** → now publishes coordinator network parameters
   (`ZDO_EXT_NWK_INFO`, parsed as `zStackAdapter.getNetworkParameters`), the MQTT
   URI/version, OS information, redacted config (network key/password removed),
   `restart_required`, build commit/version and the staged converter version.
5. **`definition` was fabricated** (`{model,vendor,description:""}`) → now built
   from the converter package with the upstream `prepareDefinition` semantics:
   extend merge, action-expose merge, generic toZigbee prepend, calibration/
   precision options, converter-declared options, function exposes, white-label
   override, and the always-appended linkquality expose.
6. **`supported` used the interview flag**; upstream uses `!!definition`.
7. **Buffer hex decoding failed in Boa**: `String.prototype.substr` is not
   available (Annex B), so `Buffer.from(hex)` threw and every `lib/lumi.js`
   definition failed to load (12 devices). The shim now uses `slice`; a
   regression test covers hex round-trip and offset writes.
8. **Catalog fallback selection** ignored each candidate's `zigbeeModel`; the
   registry now ships `lookupCandidates` and the resolver mirrors upstream
   `findDefinition` (e.g. `Thermostat` → eCozy `1TST-EU`, not the Schneider
   CCTFR6400).

## Frontend data contract (HomePage / About)

Serving the pinned Windfront 2.15.0 exposed two real bridge/info contract gaps on
the device:

- `About-CPBYVYHu.js` reads `bridgeInfo.zigbee_herdsman.version`; the field was
  absent, so the page crashed with `Cannot read properties of undefined
  (reading 'version')`.
- `HomePage-9dp65JcO.js` reads `config.availability.enabled`; the field was
  absent (our config model does not include `availability`), crashing with
  `... (reading 'enabled')`.

`publishInfo` now publishes the upstream **config shape**, not only the modeled
fields: the raw YAML document is deep-merged with the resolved runtime values,
then upstream defaults are filled for `availability` (enabled/active/passive),
`health`, `ota`, `map_options`, `version`, `device_types`, MQTT/serial extras
and `advanced.last_seen`/logging keys. `homeassistant` is normalized to
`{"enabled": ...}` (the YAML accepts a bare bool, but the UI dereferences
`.enabled`), and `zigbee_herdsman.version` (pinned compatibility baseline
`10.10.0`, checked against `upstream-lock.json` by a unit test) and
`os.node_version` (`n/a (Rust)`) are published. Secrets (`advanced.network_key`,
`mqtt.password`, `frontend.auth_token`) are stripped.

The API differential now asserts the shape of these fields
(`config_ui_shape`), so a missing `availability`/`last_seen`/`homeassistant`
object or version field fails CI instead of only crashing the browser.

## Generated definitions

Seven dump devices have upstream **generated** definitions
(`zhc.findByDevice(..., true)` → `generateDefinition`). Rust now generates them
too (CONVERTER-021): the bridge publishes the definition after join/failed
interview (not while `IN_PROGRESS`) and reports `supported=false` because
upstream `isSupported` excludes generated definitions. All five generated
differential fixtures match the pinned Node generator byte-for-byte; the API
case compares the marker and `supported`.

## Other gaps

- `config_schema` is not published; `zigbee_herdsman.version` has no analogue in
  a Rust implementation; `os`/`version`/`commit` are build identity.
- Endpoint bindings/reportings/scenes and `supported` semantics beyond
  definition presence are covered elsewhere.
- No HTTP/WebSocket frontend API or static Windfront routing differential; only
  the MQTT bridge API is covered.
- Upstream side is a vitest harness with mocks; no live broker, retained/QoS
  comparison here, no hardware/HIL.
