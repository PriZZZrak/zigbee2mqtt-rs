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

Seven transcript cases, taken from upstream `test/adapter/z-stack/adapter.test.ts`:

1. `ZStackAdapter::send_zcl_frame_to_all` → `AF dataRequestExt`
   (upstream "Send zcl frame to all").
2. `Endpoint::command` → `ZStackRuntimeAdapter` → `ZStackAdapter` →
   `AF dataRequest` + `AF dataConfirm` (AREQ) (upstream "Send zcl frame
   network address").
3. `ZStackAdapter::send_zcl_frame_to_group` → `AF dataRequestExt`
   (upstream "Send zcl frame group").
4. `ZStackAdapter::permit_join` → ZDO `mgmtPermitJoinReq` +
   `mgmtPermitJoinRsp` (upstream "Sends proper ZDO request payload for
   PERMIT_JOINING_REQUEST to target").
5. `ZStackAdapter::send_zcl_frame_inter_pan_to_ieee_addr` → `AF
   dataRequestExt` with `dstaddrmode=3` (upstream "Send zcl frame interpan").
6. Retry on `MAC_CHANNEL_ACCESS_FAILURE`: first dataConfirm status 225, retry
   with a new transaction id, both frames compared with upstream goldens.
7. ZDO payloads: `NETWORK_ADDRESS_REQUEST` and `NWK_UPDATE_REQUEST` UNICAST
   (upstream "Sends proper ZDO request payload ...").

The permit-join case exposed a real port bug: `getResponseClusterId` only
knew six responses, so permit join never created its ZDO waiter. It now follows
upstream's request→response map.

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

## Controller start/stop transcript

`adapter/z_stack/adapter/manager.rs` tests now run the real
`ZStackAdapter.start` + `Controller.start` + `Controller.stop` against a
scripted PTY coordinator with an in-memory NV store (NIB, network key, key
info). The fake answers `SYS version/ping`, `osalNvLength/ReadExt/WriteExt`,
`UTIL getDeviceInfo`, `ZDO activeEpReq/simpleDescReq/extNwkInfo`,
`extFindGroup`, `AF register/dataRequest/dataRequestExt` and
`mgmtPermitJoinReq`.

The test records every outgoing UNPI frame to a file and asserts the ordered
presence of the startup stages: `ping`, `version`, `getDeviceInfo`,
`osalNvLength`, `activeEpReq`, `simpleDescReq`, `mgmtPermitJoinReq`.
This is the coordinator-only smoke that HIL would otherwise provide.

## Next slice

- Run the upstream Node `Controller` against the same script and commit both
  transcripts as a fixture.
- Compare the full ordered frame sequence (start, coordinator creation,
  `permitJoin`, ZCL endpoint/broadcast, stop).
- Add retry/route-discovery and incoming-message transcripts from
  `znp.test.ts` / `adapter.test.ts`.
