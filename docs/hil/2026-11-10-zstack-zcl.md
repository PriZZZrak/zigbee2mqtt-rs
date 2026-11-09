# ZStackAdapter event/ZCL slice (2026-11-10)

## Event dispatch

`Znp` now exposes the upstream `received` stream via a broadcast channel.
`ZStackAdapter` subscribes and dispatches with the upstream rules:

- ZDO AREQ → `zdoResponse`;
- `tcDeviceInd` → `deviceJoined`;
- `concentratorIndCb` → `zdoResponse(NETWORK_ADDRESS_RESPONSE)`;
- `leaveInd` (rejoin=false) → `deviceLeave`;
- AF `incomingMsg` / `incomingMsgExt` → ZCL header parse, waitress resolution,
  `zclPayload` event.

Unit transcript tests:

```text
dispatches_leave_indication ... ok
dispatches_af_payload_and_resolves_waitress ... ok
```

## ZCL send

Ported:

- `dataRequest` (AF `dataConfirm` + transaction id);
- `dataRequestExtended` (group/broadcast, confirmation + retry);
- `sendZclFrameToGroup`;
- `sendZclFrameToAll`;
- `selectSourceEndpoint`.

HIL on the real coordinator:

```text
broadcast ZCL read sent
test hil_zstack_adapter_send_zcl_broadcast ... ok
```

## Still pending

- `sendZclFrameToEndpointInternal` (endpoint recovery/assoc add/remove, route
  discovery, response retry);
- interPAN paths;
- `getNetworkParameters`, backup facade.

## Endpoint ZCL round-trip (real thermostat)

`sendZclFrameToEndpointInternal` was added with waitress response, data-confirm
recovery, assoc get/add/remove, route discovery and network-address recheck.
HIL against the kitchen thermostat (`0x404ccafffe5c4ce0`, router):

```text
thermostat network address: 18008
ZCL read response: cluster=0 endpoint=1 cmd=1 data=1807010000002008
test hil_zstack_adapter_zcl_read_roundtrip ... ok
```

Full HIL suite: 8 passed, 0 failed.

A `Frame::create` bug was exposed: `command_from_name("read")` did not set
`response=1`, so the waiter was never registered. Fixed; the read response is
now awaited correctly.
