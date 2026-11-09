# Manager restore/commissioning HIL (2026-11-10)

## What was implemented

`ZnpAdapterManager` restore and commissioning paths are exercised by new HIL
tests in `crates/zigbee-apiarist/tests/hil_znp.rs`:

- `hil_manager_restore_backup_recovers_network`:
  - reads the current backup from the adapter;
  - clears `ZNP_HAS_CONFIGURED_ZSTACK3` to simulate a fresh adapter;
  - `manager.start()` must select `RestoreBackup`, commission a random
    provisioning network, restore the backup, rewrite commissioning NV items,
    reset and start up;
  - the restored network (PAN/channel/key/IEEE/devices) is compared with the
    original backup.

## Run log up to the interruption

```text
original: pan=1a62 channel=11 devices=9
zh:adapter:zstack:manager: determining znp startup strategy
zh:adapter:zstack:manager: (stage-1) adapter is not configured / not commissioned
zh:adapter:zstack:manager: (stage-2) configuration matches backup
zh:adapter:zstack:manager: determined startup strategy: RestoreBackup
zh:adapter:zstack:manager: commissioning random provisioning network:
zh:adapter:zstack:manager: beginning network commissioning
zh:adapter:zstack:manager: network commissioned
```

The run then failed inside `manager.start()` after commissioning. The repeat
run could not open the coordinator:

```text
open coordinator: "Permission denied (os error 13)"
```

`/dev/ttyUSB0` is now `crw-rw---- root:dialout` while the build user is
`uid=99(abc)` without the `dialout` group, so the HIL test suite cannot run
until the device permissions are restored.

## Status

`HARDWARE_PENDING` for restore/commissioning. The next run of the test restores
the router network from `router-node/coordinator_backup.json` regardless of the
adapter's current state, so the test is self-recovering.
