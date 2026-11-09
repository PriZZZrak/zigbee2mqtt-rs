# ZStackAdapter start/stop HIL (2026-11-10)

`ZStackAdapter` constructor, `start`, `stop`, `getCoordinatorIEEE` and
`getCoordinatorVersion` were executed against the real Dongle Plus.

## Command

```bash
Z2M_SERIAL_PORT=/dev/ttyUSB0 \
Z2M_HIL_DIR=/config/workspace/zigbee2mqtt-rs/ref/hil-2026-11-09 \
  cargo test -p zigbee-apiarist --test hil_znp -- --ignored --nocapture
```

## Result

```text
zstack adapter start: Resumed
coordinator IEEE: 0x00124b0024c2d400
coordinator version: CoordinatorVersion {
  type_name: "ZStack3x0",
  meta: {"transportrev":2,"product":1,"majorrel":2,"minorrel":7,"maintrel":1,"revision":20250321}
}
test hil_zstack_adapter_start_stop ... ok
```

Full HIL suite:

```text
test hil_version_request ... ok
test hil_nv_memory_init ... ok
test hil_transport_get ... ok
test hil_create_backup_matches_router ... ok
test hil_manager_start_resumes_network ... ok
test hil_zstack_adapter_start_stop ... ok
test result: ok. 6 passed; 0 failed; 0 ignored
```

## Architectural change

`Znp` is now shared as `Arc<tokio::sync::Mutex<Znp>>` between
`ZStackAdapter`, `AdapterNvMemory`, `AdapterBackup` and `ZnpAdapterManager`,
matching upstream's shared-object model instead of exclusive Rust ownership.

## Still pending

- `onZnpReceived` event dispatch;
- ZCL `sendZclFrameToEndpoint` / `sendZclFrameToGroup` / `sendZclFrameToAll`
  and their waitress paths;
- interPAN paths, `getNetworkParameters`, backup facade, LED/transmit-power
  edge cases beyond start.
