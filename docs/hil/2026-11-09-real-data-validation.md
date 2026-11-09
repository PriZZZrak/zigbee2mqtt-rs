# Real-data re-check (2026-11-09)

Validated the already-ported pure layers against the actual router data and the
adapter NV dump.

Command:

```bash
Z2M_HIL_DIR=ref/hil-2026-11-09 \
  cargo run -p z2m-rs --example validate_hil
```

Output:

```text
settings: ok
database: ok (4 entries)
backup: ok (source="zigbee-herdsman@11.2.0")
adapter NV: ok (NIB/keys match router backup)
all real-data checks passed
```

## What is checked

- `configuration.yaml` through `z2m_rs::util::settings`:
  - `serial.port=/dev/ttyUSB0`, `mqtt.base_topic=zigbee2mqtt`;
  - default `advanced.pan_id=6754`, `network_key=[1,3,...,13]`;
  - 3 devices.
- `database.db` through `zigbee_apiarist::controller::database`:
  - 4 entries: 1 Coordinator, 1 EndDevice, 2 Router.
- `coordinator_backup.json` through `from_unified_backup`/`to_unified_backup`:
  - pan `0x1a62`, channel 11, key length 16, 9 devices;
  - `znpVersion` is numeric `1`;
  - generated `source` is `zigbee-herdsman@11.2.0`.
- `adapter-dump.json` through `Struct`/`Nib`/`NwkKeyDescriptor`:
  - NIB 116 bytes (aligned), pan `0x1a62`, channel 11, channel mask `[11]`,
    extended PAN ID `00124b0024c2d400`, security level 5, update id 0;
  - active key info (17 bytes) and `PRECFGKEY` match the router backup key;
  - `EXTADDR` is little-endian `00d4c224004b1200`.

## Fixes found by real data

- `settings::defaults()` was a minimal/generated set (`pan_id="GENERATE"`,
  `network_key="GENERATE"`, wrong HA/frontend defaults). Replaced with the
  upstream `defaults` object (pan 6754, real network key, ext pan `dd...`,
  availability/map/OTA/health defaults).
- `Backup.znp_version` / `ezsp_version` were `Option<String>`, but the real
  backup and upstream model store numbers. Changed to `Option<u16>` and parse
  numeric values.
