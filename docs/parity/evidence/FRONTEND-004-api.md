# FRONTEND-004 — bridge MQTT API differential

Implementation: `scripts/api_diff.py`, `src/bin/evid04_api_candidate.rs`
(`--features test-support`), checked-in upstream dump
`docs/parity/evidence/api/upstream-dump.json`, generator
`docs/parity/evidence/api/api_dump_reference.test.ts`,
`docs/parity/evidence/api/cluster-map.json`.
Behavior: FRONTEND-004; related MQTT-002/MQTT-003/MQTT-005, FRONTEND-001.

## Contract

The frontend consumes four bridge MQTT surfaces (pinned app
`344b6e03b43644c97154a72ca9eae17193c553a2`):

- `<base>/bridge/state` — `{"state": "online" | "offline"}`,
  `lib/extension/bridge.ts` state publish.
- `<base>/bridge/info` — `lib/extension/bridge.ts:publishInfo`; the API
  consumer reads `coordinator.{ieee_address,type}`, `log_level`, `permit_join`.
- `<base>/bridge/devices` — `lib/extension/bridge.ts:publishDevices`; each device
  carries `ieee_address`, `type`, `network_address`, `friendly_name`,
  `power_source`, `model_id`, `manufacturer`, `interview_completed`,
  `interview_state`, `supported`, `disabled` and `endpoints`.
- `<base>/bridge/response/permit_join` — request/response envelope
  `{status: "ok", data: {time: <seconds>}}` for
  `bridge/request/permit_join {time}`.

## Upstream dump

`api_dump_reference.test.ts` is copied into the pinned app checkout
(`test/extensions/api_dump.test.ts`), starts the real `Controller` with the
repository's vitest herdsman/MQTT mocks, and records the latest payload for
those topics plus the mock device metadata used to build the Rust fixture. The
dump is checked in at `docs/parity/evidence/api/upstream-dump.json` (no live
network/broker required).

Regeneration command:

```bash
API_DATA_DIR=/tmp/.../z2m-api-data API_DUMP_PATH=.../upstream-dump.json \
  pnpm vitest run --config ./test/vitest.config.mts test/extensions/api_dump.test.ts
```

## Rust candidate

`evid04_api_candidate --data <dir>`:

1. loads `configuration.yaml` and `database.db` through the same
   `Config::load` + `database::load_database` the bridge uses;
2. starts the in-process MQTT broker and a fake ZNP coordinator;
3. publishes `bridge/state`, `bridge/info`, `bridge/devices` through the real
   bridge functions and produces the real `permit_join` response (the
   coordinator call is attempted against the fake transport);
4. prints the observed payloads as JSON.

The harness writes the config/database fixture for that directory from the
upstream dump, so both sides describe the same devices, names and flags.

## Normalization

Explicit and small:

- The dump's synthetic mock devices with a non-hex IEEE
  (`0x90fd9ffffe4b64ax`) and with NWK > `0xFFFF` are excluded from both sides:
  neither importer can persist them, and they cannot exist on a real network.
- `bridge/info` compares only `coordinator.ieee_address`, `coordinator.type`,
  `log_level`, `permit_join` (other fields are separate rows).
- `bridge/devices` compares identity/metadata and the set of endpoint ids.
  Endpoint clusters/bindings/configured reportings/scenes are covered by the
  storage differential (STORAGE-009), and `supported` depends on converter
  catalog availability (catalog-parity row), so both are excluded here.
- Coordinator entries are excluded (published separately / not in the dump's
  device view).

## Result

```text
$ python3 scripts/api_diff.py \
    --upstream docs/parity/evidence/api/upstream-dump.json \
    --candidate-cmd "./target/release/evid04_api_candidate" \
    --require-pins --report docs/parity/reports/api.json
API: 4 passed, 0 failed, pins=[all four MATCH]

bridge-state          PASS
bridge-info           PASS
bridge-devices        PASS   (62 non-coordinator devices)
permit-join-response  PASS
```

Harness regression tests: `scripts/tests/test_api_diff.py` (2 tests: matching
candidate passes with all four cases; a renamed device fails `bridge-devices`).

## Defects found and fixed by this differential

1. **`bridge/request/permit_join` produced no response.** The request fell
   through to "Unsupported bridge request"; the bridge now answers
   `{status:"ok",data:{time:<duration>}}` and logs the permit-join line.
2. **Device payload lacked `endpoints` and `interview_state`.** `bridge/devices`
   entries now include the endpoint map (with empty bindings/reportings/scenes
   and input/output clusters) and the persisted interview state.
3. **Persisted `type` and `interviewState` were dropped on import.** The
   database importer ignored the entry `type`; `Device` now keeps `device_type`
   and `interview_state`, so a battery `EndDevice` is not reported as a `Router`
   and `IN_PROGRESS`/`FAILED` states survive.
4. **`bridge/devices` was silently not published for real networks.** rumqttc
   defaults to a 10 KiB outgoing packet limit; the payload for 62 devices is
   larger, so the publish was dropped without an error. The bridge now sets a
   256 MiB limit (the MQTT maximum) and a regression test publishes a payload
   larger than 10 KiB and asserts it arrives retained.

## Gaps

- Full `bridge/info` fields (version details, `config`, `config_schema`,
  `network`, `os`, `mqtt`, dependency versions), the coordinator entry inside
  `bridge/devices`, and per-device `definition`/`options`/`exposes` are not
  compared.
- Endpoint bindings/reportings/scenes and `supported` are excluded here (other
  rows).
- No HTTP/WebSocket frontend API and no static Windfront routing differential;
  only the MQTT bridge API is covered.
- The upstream side is a vitest harness with mocks, not a running broker plus
  real radio; no hardware/HIL, no retained/QoS assertions here (QoS/retain for
  `bridge/devices` is asserted only in the Rust regression test).
