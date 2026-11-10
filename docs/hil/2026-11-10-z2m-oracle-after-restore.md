# Upstream z2m oracle after the destructive restore/commissioning run (2026-11-10)

Goal: decide whether the current ZCL timeout on the real thermostat is a defect
of the Rust port or the state of the live devices. The pinned upstream
`zigbee2mqtt` 2.14.3 was started from the dev container with the router data
set on the same Dongle Plus.

## Setup

```bash
mkdir -p /tmp/opencode/z2m-check
cp ref/hil-2026-11-09/z2m-run/{configuration.yaml,database.db,state.json,coordinator_backup.json} \
   /tmp/opencode/z2m-check/
# configuration.yaml: base_topic zigbee2mqtt-check, mqtt://192.168.0.1,
#                      /dev/ttyUSB0, HA/frontend disabled, log_level debug

cd .upstream/zigbee2mqtt
ZIGBEE2MQTT_DATA=/tmp/opencode/z2m-check /config/.local/bin/node index.js
```

## Result

The original z2m resumed the same network:

```text
zh:adapter:zstack:manager: endpoint '242' already registered
zh:controller: Started with result 'resumed'
zh:controller: Injected database: true, adapter: true
getDeviceInfo: {"numassocdevices":0,"assocdeviceslist":[]}
z2m: Currently 3 devices are joined.
MQTT publish: zigbee2mqtt-check/bridge/state {"state":"online"}
```

A live `local_temperature` read to the kitchen thermostat behaved exactly like
the Rust adapter:

```text
--> SREQ: AF - dataRequest - {"dstaddr":18008,...,"clusterid":513,...}
<-- SRSP: AF - dataRequest - {"status":0}
<-- AREQ: AF - dataConfirm - {"status":0,"endpoint":1,"transid":2}
Response timeout (0x404ccafffe5c4ce0:18008,0)
Response timeout recovery: Node relation 0 (0x404ccafffe5c4ce0 / 65534)
Response timeout (0x404ccafffe5c4ce0:18008,1)
Error: ... failed (Timeout after 10000ms ...)
```

`grep -c 'AREQ: AF - incomingMsg'` over the whole z2m run: **0**.

Permit join works but nothing joins:

```text
Received ZDO response: clusterId=PERMIT_JOINING_RESPONSE, status=SUCCESS, payload=undefined
MQTT publish: zigbee2mqtt-check/bridge/response/permit_join {"status":"ok"}
```

## Conclusion

- The adapter/network state is accepted by upstream z2m (`resumed`, same PAN
  `0x1a62`, channel 11, 3 known devices, 9 address-manager entries).
- Upstream z2m reports the same MAC confirmation (status 0) and the same ZCL
  response timeout, and receives no `AF incomingMsg` at all.
- The coordinator's association table is empty (`numassocdevices=0`) and the
  device relation is unknown (`65534`).
- Therefore the current ZCL HIL failure is **not** a port defect: the devices
  lost their association/network key during the destructive re-commissioning
  and must re-join/re-pair.

The log also confirms the newly ported `PERMIT_JOINING_RESPONSE` reader: z2m
handles `mgmtPermitJoinRsp` as `PERMIT_JOINING_RESPONSE status=SUCCESS`.
