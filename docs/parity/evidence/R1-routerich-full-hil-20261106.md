# R1 full hardware-in-the-loop run — Routerich AX3000 v1 + SONOFF Dongle-P

Date: 2026-11-06 (UTC). Host: RouteRich/OpenWrt 24.10.8, aarch64,
kernel 6.6.144. Coordinator: TI CC2652P / SONOFF Zigbee 3.0 USB Dongle Plus
(`/dev/ttyUSB0`, cp210x, Z-Stack 2.7, product_id=1). Network: PAN `0x1a62`,
extended PAN `0x00124b0024c2d400`, channel 11.

This run exercised the R1 milestone end-to-end on real hardware. It is not a
claim of complete Stage 1 parity (CC2531 hardware evidence and the remaining
inventory are still open).

## Artifacts

| Item | Value |
| --- | --- |
| Final binary | `9b3bc1653e13a933175a8fa07473b5928fece6f1603f0d8d5c434b72e81b1b89` (aarch64 musl, binary-only deploy; `.ipk` not rebuilt at user request) |
| Pre-run backup | `/root/z2mback/hil-20261106T135234Z/` (config, database, state, external converters, binary) |
| Config | `/etc/zigbee2mqtt-rs/config.yaml`, `advanced.log_level=info` |
| Devices | 2× Lytko `L101Ze.v2` / `L101Ze.v2-TT`, 1× Tuya `TS0601` (`_TZE200_locansqn`, TH01Z) |
| External converter | `L101Ze_z2m.js` (28,750 bytes) |

## Results

### 1. Bridge API over MQTT

- `bridge/state` = `{"state":"online"}`; `bridge/info` reports coordinator
  `0x00124b0024c2d400`, channel 11, PAN 0x1a62, `log_level=info`.
- `bridge/devices`: 4 entries (coordinator + 3 devices). Lytko definitions
  resolve `source=external`; Tuya resolves the packaged catalog definition
  (`native`); `supported=true` for all three.
- `bridge/converters` lists `L101Ze_z2m.js`; `bridge/groups` is `[]`.
- `health_check` answered in 216 ms; `networkmap` returns nodes (links are
  still a stub — open gap).
- `bridge/request/options` answered in **300 ms** with
  `{"status":"ok","data":{"restart_required":true},"transaction":…}` and
  persisted `last_seen: ISO_8601` to `configuration.yaml`.

### 2. permit_join

- `{"time":30}` → `{"status":"ok","data":{"time":30},"transaction":…}` in
  231 ms; `bridge/info.permit_join=true` and `permit_join_end` populated.
- `{"time":0}` → `false`/`null`, 214 ms.
- Fixed during this run: the response previously omitted `transaction`, and
  `bridge/info` was not republished on permit-join changes.

### 3. Lytko commands (real hardware)

Kitchen `L101Ze.v2` (single channel):

- `occupied_heating_setpoint` 28 → 20 → 28, `system_mode` off → heat → off.
  Setpoint was deliberately below room temperature so the relay stayed idle;
  every state change was observed on the retained topic.
- `get {"local_temperature_l1":""}` returned the current value.

Bedroom `L101Ze.v2-TT` (two channels):

- `system_mode_l1` off → heat → off took effect.
- `occupied_heating_setpoint_l1` initially produced
  `no toZigbee converter handles 'occupied_heating_setpoint_l1'`. Root cause:
  the endpoint suffix was not stripped before `toZigbee.convertSet`. Fixed by
  `split_set_key()` (only real endpoint suffixes such as `l1`, `l2`, `button`,
  or a numeric endpoint are split; `system_mode` stays intact) and by
  postfixing optimistic state keys with the endpoint name (upstream
  `publish.ts`). After the fix the write frame
  (`Write Attributes 0x0201/0x0012`, int16 2000) reached the device, which
  replied `Write Attributes Response` success, and the optimistic
  `occupied_heating_setpoint_l1` was published.
- Residual gap: after a restart the converter sessions warm for ~30-40 s on
  this 2-core router. Set commands issued in that window can wait behind the
  warmup and time out on the caller side (the job still completes later).
  Commands after warmup are immediate. Making `convert_set` asynchronous is
  the next step; see the inventory note for CONVERTER-023.

### 4. Device rename / database persistence

- `device/rename` now answers in ~200 ms (previously the handler evaluated Boa
  and built the device list on the event loop and did not answer within 20 s).
- Renaming to `Датчик HIL2`/`Датчик HIL3` cleared the old retained state topic
  and published the new one. The friendly name was written to
  `configuration.yaml` and survived a service restart, `kill -9`/procd respawn
  and a full router reboot.
- `bridge/response/device/rename` contains `from`, `to`,
  `homeassistant_rename` and the transaction.

### 5. State cache and graceful shutdown

- `SIGTERM` → `bridge/state` retained becomes `offline`, `state.json` is
  written on shutdown (mtime/size updated), process exits.
- Start restores `Restored cached state for 3 device(s)` and the sleepy Tuya
  sensor immediately publishes cached `temperature=26.4`, `humidity=45`,
  `battery=45` instead of empty.
- Periodic writer updates `state.json` (observed mtime changes every ~30 s).

### 6. External converter lifecycle

- `converter/remove {"name":"L101Ze_z2m.js"}` → `ok`; the file disappears and
  definitions fall back; `converter/save` restores it; `bridge/converters`
  lists it again and both Lytko definitions resolve `external`.
- Invalid JavaScript (`bad_hil.js`) is rejected; the temporary file is removed
  again (verified by directory listing after the response).
- While reloading the 28 KB converter, `health_check` stayed responsive
  (~4.5 s worst case during the full Boa reload, frontend timeout is 20 s).

### 7. MQTT broker restart / process crash

- Restarting mosquitto: the bridge logged
  `MQTT event loop error: ConnectionReset; retrying in 1s` and reconnected/
  resubscribed one second later; retained state and command handling continued.
- `kill -9` of the bridge: procd respawned it; after startup the bridge was
  online, the three devices were restored, the renamed device kept its name and
  cached state was present.

### 8. Router reboot

- Router rebooted, SSH returned after 58 s. procd auto-started the service
  (`/etc/rc.d/S99zigbee2mqtt-rs`).
- Boot timeline: cache restored before the ZNP reset, `Device registry: 3 devices`, ZNP soft
  reset, `resuming the existing network without rewriting NV`, network restored
  from NV, `Startup complete`.
- After boot: `bridge/state=online`, names persisted, definitions correct,
  sensor cached values present. No NV rewrite occurred.

### 9. Frontend HTTP / WebSocket and HA discovery

- `GET /` → HTTP 200, 886-byte HTML.
- WebSocket `/api` handshake → `101 Switching Protocols`.
- Sending `{"topic":"bridge/request/health_check","payload":{…}}` over WS now
  returns `bridge/response/health_check`; `bridge/request/options` over WS
  returns `bridge/response/options` with the same transaction. **This was
  broken**: `publish_bridge_response`, `publish_bridge_health`,
  `publish_availability`, `publish_bridge_networkmap` and HA discovery
  published to MQTT without forwarding to the frontend event bus, so the UI
  reported `Request timed out` even when MQTT subscribers received the reply.
  They now go through `MqttBridge::publish_bus`.
- Retained HA discovery: 126 configs (40 binary_sensor, 59 sensor, 14 number,
  5 select, 4 climate, 3 switch, 1 button). Spot-checked sensor config:
  `unique_id=0xa4c13887bf43cc0f_temperature_zigbee2mqtt`,
  `state_topic=zigbee2mqtt/Датчик температуры`, `device_class=temperature`,
  `value_template={{ value_json.temperature }}`, bridge `via_device` set.
- Startup publish order was changed to publish `bridge/devices` before HA
  discovery; previously a stale retained device list (e.g. with a `generated`
  definition left over from a converter remove) could remain visible for ~60 s.

## Defects found and fixed in this run

1. permit_join response missing `transaction`; `bridge/info` not updated on
   permit-join change.
2. Endpoint-suffixed `toZigbee` keys (`system_mode_l1`,
   `occupied_heating_setpoint_l1`) were passed unsplit to the converter.
3. Optimistic converter state keys were not postfixed with the endpoint name.
4. Frontend WS never received bridge responses/health/availability/networkmap/
   HA discovery (MQTT-only publish paths).
5. `device/rename` blocked the event loop on Boa work, did not respond within
   the UI timeout, did not persist the name to `configuration.yaml`, and did
   not clear the old retained state topic.
6. Stale retained `bridge/devices` served for up to ~60 s after startup because
   HA discovery was published before the device list.
7. Converter command queue was FIFO; user commands could sit behind cold-session
   warmups (now commands jump the queue; incoming frames do not starve warmup).
8. Registry/zcl/zcl-commands JSON were parsed on hot paths; now cached.
9. `database.db` (`sync_all`) and `state.json` writes moved off async workers.

## Not verified / open gaps

- CC2531 (Z-Stack HA 1.2) hardware: still required for complete Stage 1.
- Command latency during the first ~30-40 s after restart (cold Boa sessions).
- `advanced.last_seen` setting is persisted but the bridge still always
  publishes `last_seen` (and uses `+00:00` instead of upstream's ISO `…Z`).
- `networkmap` links are empty; `convertGet` for setpoint returned no ZCL read
  in this run.
- Live Home Assistant instance, OTA flashing, `group/add`/scenes, full
  `config_schema`, `.ipk` upgrade/rollback and CC2531 remain open.
- Upstream freshness was not re-fetched during this HIL run; the recorded
  baseline is unchanged (`freshness=DRIFT_DETECTED` from 2026-11-05).

## Post-HIL incident: `device/remove` hang (2026-11-06)

After the HIL run the user removed the Tuya sensor from the UI. The bridge
stopped answering (process alive, MQTT connection closed, broker LWT set
`bridge/state=offline`) and had to be SIGKILLed by procd after SIGTERM did not
complete. Root cause: `device/remove` executed `external.exposes()` and
`device_list_json()` synchronously on the bridge event loop. `exposes()` also
waits on the process-wide Boa evaluation lock; the single converter thread was
busy warming the just-rejoined device, so the event loop deadlocked behind a
non-async mutex and could not run the MQTT/shutdown tasks.

Fix (deployed binary `3a0a7267…`):

- `device/remove` now removes the entity, clears the old retained state topic,
  deletes the `configuration.yaml` device entry and answers
  `{"status":"ok","data":{"id":…},"transaction":…}` **before** any Boa work.
- HA discovery cleanup and the device-list rebuild run in a spawned task;
  `exposes`/`device_list_json` use `spawn_blocking` with 20 s/40 s timeouts so a
  slow converter host can no longer pin the event loop.
- `DeviceLeft` and the post-interview device-list publish use the same async
  helper.

Verification on the ghost sensor left by the hang (IEEE
`0xa4c13887bf43cc0f`, no model/endpoints after the interrupted removal):

```text
device/remove response: 99 ms, transaction echoed
health_check immediately after: 413 ms (2 devices)
bridge/state: online
configuration.yaml / database.db: 0 occurrences of the IEEE
service restart: Device registry: 2 devices; sensor does not return
retained zigbee2mqtt/Датчик температуры: cleared
```

Residual: because the ghost had no resolvable definition (`model="-"`),
`cleanup_discovery` could not derive its 13 HA retained configs; they were
cleared manually from the broker. Devices removed while their definition is
resolvable clean up automatically. A related data-quality risk remains: the
interrupted join/interview wrote a stub (empty model/endpoints) for the device
into `database.db`; preserving the last complete DB record across an
interrupted interview is a follow-up.

### Follow-up: the removed sensor returned (2026-11-06)

After the first remove, the sensor announced itself again and the bridge
re-created it as a ghost (empty model/endpoints), so HA rediscovered it. Two
behaviours were missing compared with upstream:

- `Device.removeFromNetwork` sends a **ZDO Mgmt_LeaveReq** with
  `WITHOUT_REJOIN`; without it the device stays on the network and re-announces.
- `device/remove` honours `block` (persist the IEEE in `blocklist`), `force`
  (skip the leave), `keep_config` and `clear_cache`; the response includes
  those flags.

Implemented and verified on the same sensor:

```text
device/remove {"id":"0xa4c13887bf43cc0f","block":true}
  -> 115 ms, {"id":...,"block":true,"force":false,"keep_config":false,"clear_cache":false}
health_check -> 221 ms, devices=2
config.yaml -> blocklist: ['0xa4c13887bf43cc0f']
log -> Device left: 0xa4c13887bf43cc0f
60 s later -> device list still 2 devices (no re-add)
restart -> Device registry: 2 devices, bridge online
retained HA discovery topics for the IEEE -> 0
```

The runtime blocklist is checked for joins/announces before a device can be
recreated, and it is loaded from config at startup. If Home Assistant still
shows the old device card after this, reload the MQTT integration (or restart
HA); the retained discovery configs are gone, but HA can keep an orphaned
device-registry entry until it reloads.
