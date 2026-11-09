# Manager restore/commissioning HIL (2026-11-10)

Both destructive manager paths were executed on the real Dongle Plus.

> **Warning:** these tests delete/recreate the coordinator network and
> re-commission a random provisioning network. Live devices stop answering
> while the network is reprovisioned and may need to rejoin/re-pair afterwards.
> They are gated behind `Z2M_HIL_DESTRUCTIVE=1`.

## Restore backup

`hil_manager_restore_backup_recovers_network` clears
`ZNP_HAS_CONFIGURED_ZSTACK3`, selects `RestoreBackup`, commissions a random
provisioning network, restores `router-node/coordinator_backup.json`, resets,
starts up and compares the result with the stored backup.

```text
expected: pan=1a62 channel=11 devices=9
(stage-1) adapter is not configured / not commissioned
(stage-2) configuration matches backup
determined startup strategy: RestoreBackup
beginning network commissioning
network commissioned
restore startup result: Restored
restored network: panId=Some(6754) channel=Some(11)
test hil_manager_restore_backup_recovers_network ... ok
```

Verified after restore: PAN `0x1a62`, channel `11`, extended PAN ID,
network key, security level, key sequence number, coordinator IEEE and the
9 address-manager devices.

## Commissioning

`hil_manager_commissioning_then_restore` commissions a new network
(`pan=0x1b63`, channel `15`, no backup), then recovers the router network.

```text
(stage-2) adapter state does not match configuration
(stage-3) configuration-adapter mismatch (no backup)
determined startup strategy: StartCommissioning
commissioning startup result: Reset
commissioned network: panId=Some(Number(7011)) channel=Some(Number(15))
(stage-5) adapter backup matches configuration
determined startup strategy: RestoreBackup
recovery startup result: Restored
recovered network: panId=Some(Number(6754)) channel=Some(Number(11))
test hil_manager_commissioning_then_restore ... ok
```

## Bug found by the HIL

The first run failed with:

```text
SREQ: SYS - osalNvWriteExt - {"id":58,...,"value":[...,18 bytes]} failed with
status '(0x02: INVALID_PARAM)'
```

`AdapterBackup::restoreBackup` and `updateCommissioningNvItems` serialized
`nwkKeyDescriptor` / `channelList` / `nwkPanId` with the adapter memory
alignment (18 bytes for the key descriptor). Upstream calls `.serialize()`
without arguments there, i.e. **unaligned** (17 bytes). Fixed; the NV item
lengths now match and restore succeeds.

## Device state after the destructive run

After re-commissioning, the restored coordinator reports the correct PAN,
channel and key, and the NV items match the pre-destructive adapter dump
(NIB identical except `SequenceNum`). However, live devices stopped answering
ZDO requests (`nwkAddrRsp`/`ieeeAddrRsp` timeouts, MAC confirmations still
succeed). This is a consequence of re-commissioning on a live network, not of
the restore logic; devices may need a power cycle / re-pair.

## How to run

```bash
Z2M_HIL_DESTRUCTIVE=1 \
Z2M_SERIAL_PORT=/dev/ttyUSB0 \
Z2M_HIL_DIR=/config/workspace/zigbee2mqtt-rs/ref/hil-2026-11-09 \
  cargo test -p zigbee-apiarist --test hil_znp -- --ignored --nocapture
```
