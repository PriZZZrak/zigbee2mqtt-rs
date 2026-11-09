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
