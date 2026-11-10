# Wire differential without hardware

Upstream `zigbee-herdsman` and the Rust port are driven through the same
scripted coordinator over `tcp://`; every byte the bridge writes to the
"serial port" is captured to a file instead of a UART. The captured transcript
is compared byte-for-byte with a golden generated from the pinned upstream
revision.

This replaces HIL for the parts of R1 where coordinator-only evidence is
enough. It never touches a real coordinator and cannot re-commission a live
network.

## Implemented slice

`crates/zigbee-apiarist/tests/wire_capture.rs`

- A TCP server speaks UNPI and answers the Z-Stack startup/request traffic
  (`SYS ping`), then records every `AF` frame in a UNPI capture list.
- `Znp.open` writes the raw CC2530 skip-bootloader payload first; the server
  resynchronises on `SOF` (`0xfe`) exactly like `Znp.onTransportData`.
- Captured bytes are also written to a temporary `.hex` file, proving the file
  sink path.

Two transcript cases:

1. `ZStackAdapter::send_zcl_frame_to_all` → `AF dataRequestExt`.
2. `Endpoint::command` → `ZStackRuntimeAdapter` → `ZStackAdapter` →
   `AF dataRequest` + `AF dataConfirm` (AREQ).

Goldens are produced with the committed oracle:

```bash
printf '%s\n' \
  '{"id":"af","module":"zpi_object","op":"create_request","subsystem":"AF",'\
'"command":"dataRequestExt","payload":{"dstaddrmode":2,'\
'"dstaddr":"0x000000000000fffd","destendpoint":1,"dstpanid":0,'\
'"srcendpoint":1,"clusterid":6,"transid":1,"options":0,"radius":30,'\
'"len":3,"data":[17,7,1]}}' \
  | /config/.local/bin/tsx tools/oracle/run.mjs
# => fe17240202fdff00000000000001000001060001001e03001107013c
```

The test asserts the captured frame equals that string, so the Rust path is
checked against upstream `ZpiObject.createRequest`, not against itself.

## Next slice

- Script the remaining `ZnpAdapterManager` / `ZStackAdapter.start` requests so
  `Controller.start` can run without hardware.
- Run the upstream Node `Controller` against the same TCP script and commit
  both transcripts as a fixture.
- Compare the full ordered frame sequence (start, coordinator creation,
  `permitJoin`, ZCL endpoint/broadcast, stop).
- Then the ZStack runtime wrapper and the controller can move from
  `PARTIAL`/HIL-pending to `PORTED` without a router redeploy.
