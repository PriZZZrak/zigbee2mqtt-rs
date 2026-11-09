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
