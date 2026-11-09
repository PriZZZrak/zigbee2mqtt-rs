# Adapter dump: Sonoff Dongle Plus (2026-11-09)

Collected while the coordinator was attached to the dev container, before
returning it to the Routerich AX3000.

Raw output (contains keys, **not committed**): `ref/hil-2026-11-09/adapter-dump.json`.

## Identity

- IEEE: `0x00124b0024c2d400`
- `SYS version`: `transportrev=2, product=1, majorrel=2, minorrel=7, maintrel=1, revision=20250321`
- NV alignment: `aligned` (`ZCD_NV_NWKKEY` length 24)

## Network parameters read from NV

| Item | Value |
|---|---|
| `ZCD_NV_PANID` | `0x1a62` |
| `ZCD_NV_CHANLIST` | channel 11 (`00080000`) |
| `ZCD_NV_EXTENDED_PAN_ID` | `00124b0024c2d400` |
| `ZCD_NV_APS_USE_EXT_PANID` | `00124b0024c2d400` |
| `ZCD_NV_LOGICAL_TYPE` | `0x00` |
| `ZCD_NV_ZDO_DIRECT_CB` | `0x01` |
| `ZCD_NV_STARTUP_OPTION` | `0x00` (restored) |
| `ZCD_NV_ZNP_HAS_CONFIGURED_ZSTACK3` | `0x55` |
| `ZCD_NV_PRECFGKEYS_ENABLE` | `0x01` |
| `ZCD_NV_PRECFGKEY` | 16 bytes (redacted) |
| `ZCD_NV_NWKKEY` | 24 bytes aligned (redacted) |
| `ZCD_NV_NWK_ACTIVE_KEY_INFO` | 17 bytes (redacted) |
| `ZCD_NV_NWK_ALTERN_KEY_INFO` | 17 bytes (redacted) |
| `ZCD_NV_NIB` | 116 bytes (aligned; redacted) |

The NV network key matches the router `coordinator_backup.json`
(`zigbee-herdsman@10.9.2`, pan `1a62`, channel 11, 9 devices).

## Extended tables

| Table | Entries read |
|---|---|
| `ZCD_NV_EX_APS_KEY_DATA_TABLE` (6) | 3 |
| `ZCD_NV_EX_NWK_SEC_MATERIAL_TABLE` (7) | 5 |
| `ZCD_NV_EX_TCLK_TABLE` (4) | 64 |

## Router data

Fetched from `root@192.168.0.1` (Routerich AX3000):

```
ref/hil-2026-11-09/router-node/{configuration.yaml,database.db,state.json,coordinator_backup.json}
ref/hil-2026-11-09/router-rust/{config.yaml,database.db,state.json,coordinator_backup.json}
```

These contain secrets and are gitignored.

## Note

An earlier HIL test wrote and deleted `ZCD_NV_STARTUP_OPTION` on the real
coordinator. It was restored to `0x00` with
`cargo run -p zigbee-apiarist --example hil_restore_startup`. The HIL test is now
read-only; write/delete coverage lives in the PTY transcript test.
