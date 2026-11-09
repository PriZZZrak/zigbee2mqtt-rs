# AdapterBackup HIL (2026-11-10)

`AdapterBackup::create_backup` was executed against the real Sonoff Dongle Plus
and compared with the backup produced by upstream z2m for the same network.

## Command

```bash
Z2M_SERIAL_PORT=/dev/ttyUSB0 \
Z2M_HIL_DIR=/config/workspace/zigbee2mqtt-rs/ref/hil-2026-11-09 \
  cargo test -p zigbee-apiarist --test hil_znp -- --ignored --nocapture
```

## Result

```text
backup: pan=1a62 channel=11 devices=9 key_len=16 seed=true
test hil_create_backup_matches_router ... ok
test result: ok. 4 passed; 0 failed; 0 ignored
```

Compared fields (all equal to `router-node/coordinator_backup.json`):

- pan ID `0x1a62`;
- logical channel `11`;
- network key (16 bytes);
- extended PAN ID;
- coordinator IEEE `00124b0024c2d400`;
- security level;
- network key sequence number;
- 9 devices;
- TCLK seed present.

## Coverage

- `getStoredBackup`: real router unified backup parsed through
  `from_unified_backup`.
- `createBackup`: NIB, active key info, preconfigured-key flag, address manager
  table, security manager table, APS link-key table, TCLK seed/table, network
  security material table, device link-key recovery.
- `restoreBackup`: implemented, **not yet executed on hardware** (writes NV
  tables; will be validated against the router backup before any restore HIL).
