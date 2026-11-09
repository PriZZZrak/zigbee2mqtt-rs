# Manager start HIL (2026-11-10)

`ZnpAdapterManager::start` was executed against the real Sonoff Dongle Plus
with the router's network configuration and backup.

## Command

```bash
Z2M_SERIAL_PORT=/dev/ttyUSB0 \
Z2M_HIL_DIR=/config/workspace/zigbee2mqtt-rs/ref/hil-2026-11-09 \
  cargo test -p zigbee-apiarist --test hil_znp -- --ignored --nocapture
```

## Result

```text
zh:adapter:zstack:manager: determining znp startup strategy
zh:adapter:zstack:manager: (stage-1) adapter is configured
zh:adapter:zstack:manager: (stage-2) adapter state matches configuration
zh:adapter:zstack:manager: determined startup strategy: Startup
zh:adapter:zstack:manager: endpoint '1' already registered
...
zh:adapter:zstack:manager: endpoint '242' already registered
manager start result: Resumed
test hil_manager_start_resumes_network ... ok
```

All five HIL tests:

```text
version payload: ZStack3x0 (revision 20250321)
NV memory alignment: Some(Aligned)
modem status: PortStatus { cts: false, dsr: false, dcd: false, ri: false }
backup: pan=1a62 channel=11 devices=9 key_len=16 seed=true
manager start result: Resumed
test result: ok. 5 passed; 0 failed; 0 ignored
```

## Coverage

- `parseConfigNetworkOptions`: default EPID replaced with adapter IEEE.
- `determineStrategy`: configured adapter + matching config → `startup`.
- `beginStartup`: coordinator already running as `ZB_COORD`, no reset.
- `registerEndpoints`: active endpoint list fetched over ZDO through the ported
  `ZStackZdo::send_zdo` (`sendZdoInternal`) and matched
  against upstream `Endpoints`.
- `addToGroup`: `extFindGroup` accepts `FAILURE` as expected status.
- `start` returned `resumed`.

Not yet HIL-tested: `restoreBackup` path, `startCommissioning` path and
Z-Stack 1.2 branches (the test adapter is Z-Stack 3.x).
