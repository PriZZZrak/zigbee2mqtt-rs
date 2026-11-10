# Manager restore/commissioning HIL (2026-11-10)

> **FORBIDDEN TO RE-RUN.** This document records a one-off run of the
> destructive manager paths. `StartCommissioning` and `beginRestore` delete the
> NIB and form a random provisioning network. On a live installation this makes
> the devices lose the network and require physical re-pairing. The tests were
> removed from `tests/hil_znp.rs`; the rule is recorded in `AGENTS.md` §3.11.
> These paths may only be tested against a PTY/fake ZNP or a coordinator the
> user explicitly dedicates to destructive testing.

Both destructive manager paths were executed once on the real Dongle Plus
before the prohibition was introduced.

## Restore backup

`RestoreBackup`: clear `ZNP_HAS_CONFIGURED_ZSTACK3`, commission a random
provisioning network, restore `router-node/coordinator_backup.json`, reset,
start up, compare.

```text
expected: pan=1a62 channel=11 devices=9
(stage-1) adapter is not configured / not commissioned
(stage-2) configuration matches backup
determined startup strategy: RestoreBackup
network commissioned
restore startup result: Restored
restored network: panId=Some(6754) channel=Some(11)
```

Verified: PAN `0x1a62`, channel `11`, extended PAN ID, network key, security
level, key sequence number, coordinator IEEE and 9 address-manager devices.

## Commissioning

`StartCommissioning` with `pan=0x1b63`, channel `15`, then recovery:

```text
determined startup strategy: StartCommissioning
commissioning startup result: Reset
commissioned network: panId=Some(Number(7011)) channel=Some(Number(15))
determined startup strategy: RestoreBackup
recovery startup result: Restored
recovered network: panId=Some(Number(6754)) channel=Some(Number(11))
```

## Bug found by the HIL

`AdapterBackup::restoreBackup` and `updateCommissioningNvItems` serialized
`nwkKeyDescriptor` / `channelList` / `nwkPanId` with the adapter memory
alignment (18 bytes). Upstream calls `.serialize()` without arguments there,
i.e. **unaligned** (17 bytes). Fixed; `osalNvWriteExt` no longer returns
`INVALID_PARAM`.

## Consequence

After the re-commissioning the devices stopped answering. Upstream z2m 2.14.3
on the same coordinator behaves identically (`dataConfirm` success, ZCL
timeout, zero AF incoming messages, `numassocdevices=0`) — see
`2026-11-10-z2m-oracle-after-restore.md`. The devices need re-pairing.
