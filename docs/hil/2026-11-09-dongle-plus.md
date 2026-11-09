# HIL record: Sonoff Dongle Plus (container, 2026-11-09)

## Hardware

- Adapter: Sonoff Zigbee 3.0 USB Dongle Plus (TI CC2652P, Z-Stack)
- USB ID: `10c4:ea60`
- Device: `/dev/ttyUSB0` (passed through to the dev container)
- Host: Linux 6.18.38-Unraid, x86_64 (container)
- Router HIL: **not yet** — this record is coordinator-only in the container.

## Firmware / ZNP

`SYS version` returned:

```json
{"transportrev":2,"product":1,"majorrel":2,"minorrel":7,"maintrel":1,"revision":20250321}
```

## Commands

```bash
sudo chmod 666 /dev/ttyUSB0
Z2M_SERIAL_PORT=/dev/ttyUSB0 cargo test -p zigbee-apiarist \
    --test hil_znp -- --ignored --nocapture
```

## Result

```text
version payload: {"maintrel":1,"majorrel":2,"minorrel":7,"product":1,"revision":20250321,"transportrev":2}
test hil_version_request ... ok
NV memory alignment: Some(Aligned)
test hil_nv_memory_init ... ok
test result: ok. 2 passed; 0 failed; 0 ignored
```

`AdapterNvMemory::init` determined `Aligned` (NWKKEY length != 21), matching Z-Stack 3.x.

## Build

- commit: `9da010f5f63e115a1af7256289c290b7c71dc392`
- test binary: `target/debug/deps/hil_znp-b39dd8be4330bfb7`
- sha256: `803fdbd759db695d0c8d26ca00bf4faa0917cb098888aaf517763a23d8eaf353`

## Gaps

- Routerich AX3000 + OpenWrt service/restart/reboot evidence still pending.
- Start/resume, permit join, backup/restore and manager paths still need HIL
  after the manager/adapter ports land.

## Modem status (`AdapterTransport::get`)

```text
modem status: PortStatus { cts: false, dsr: false, dcd: false, ri: false }
test hil_transport_get ... ok
```

`get()` is implemented with serial2 `read_cts`/`read_dsr`/`read_cd`/`read_ri`
and verified on the same coordinator.
