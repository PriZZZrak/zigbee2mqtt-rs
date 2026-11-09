# ZStackAdapter remaining surfaces HIL (2026-11-10)

Ported and verified on the real Dongle Plus:

- `getNetworkParameters`;
- `supportsBackup` / `backup(ieeeAddressesInDatabase)`;
- `addInstallCode`;
- interPAN `setChannelInterPAN` / `restoreChannelInterPAN`,
  `sendZclFrameInterPANToIeeeAddr`, `sendZclFrameInterPANBroadcast`.

## Network parameters

```text
network parameters: NetworkParameters {
  pan_id: 6754,
  extended_pan_id: "0x00124b0024c2d400",
  channel: 11,
  nwk_update_id: 0
}
```

## Backup facade

```text
adapter backup: pan=1a62 channel=11 devices=9
test hil_zstack_network_parameters_and_backup ... ok
```

## interPAN

```text
interPAN channel set
interPAN restored
test hil_zstack_interpan_set_restore ... ok
```

Full HIL suite: **10 passed, 0 failed, 0 ignored**.

## Still pending

- `endDeviceAnnceInd` route discovery with the upstream 60s debounce;
- HIL for `restoreBackup` and commissioning paths;
- endpoint registration evidence beyond the existing manager start.

## endDeviceAnnceInd route discovery

`onZnpReceived` now handles `endDeviceAnnceInd` with the upstream rules:
status check, end-device only (`capabilities.deviceType === 0`), and the 60s
`debounce(..., {immediate: true})` route discovery through the queue.

Transcript tests:

```text
device_announce_route_discovery_is_debounced ... ok
schedules_route_discovery_only_for_end_devices ... ok
```

## Endpoint registration evidence

Read-only `activeEpRsp` from the real coordinator after `manager.start()`:

```text
coordinator active endpoints: [242, 239, 47, 13, 12, 110, 11, 10, 8, 6, 5, 4, 3, 2, 1]
test hil_zstack_active_endpoints_match_upstream ... ok
```

Every endpoint from upstream `endpoints()` is present, so the manager's
`registerEndpoints` path is confirmed against the running adapter.

Full HIL suite: **11 passed, 0 failed, 0 ignored**.
