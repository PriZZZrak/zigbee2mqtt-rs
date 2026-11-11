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

The test records every outgoing UNPI frame and compares the complete ordered
transcript byte-for-byte with
`crates/zigbee-apiarist/tests/fixtures/controller_start_wire.hex` — 38 frames
covering `ping`, `version`, NV reads, `getDeviceInfo`, active endpoints,
simple descriptors, endpoint registration, group lookup, `mgmtPermitJoinReq`
and stop.

### Incoming responses

The same scripted coordinator also sends `AF incomingMsg` responses. A ZCL
`read` request is matched with its `readRsp` by transaction sequence number;
the test asserts the returned `ZclPayload` (cluster, data, address, endpoint,
linkquality, groupID). This covers the response waitress path that cannot be
tested with `disableResponse`.

The same script is executed on upstream `zigbee-herdsman` v11.2.0 with
`tools/oracle/wire_controller.mjs` (installed `dist` build, TCP transport).
Both runs produce the **identical 38-frame transcript**, so the controller start
and stop path is now verified against upstream without hardware.

This differential exposed and fixed two real port bugs:

- `Zdo.Buffalo.buildRequest` was adding an extra leading zero to most ZDO
  request payloads; upstream reserves that byte only for `hasZdoMessageOverhead`.
- the green-power group was configured as `242` instead of `GP_GROUP_ID`
  (`0x0b84`).

This is the coordinator-only smoke that HIL would otherwise provide.

## Regenerate the fixture

```bash
/config/.local/bin/tsx tools/oracle/wire_controller.mjs /tmp/upstream-start.hex
cp /tmp/upstream-start.hex crates/zigbee-apiarist/tests/fixtures/controller_start_wire.hex
cargo test -p zigbee-apiarist --lib adapter::z_stack::adapter::manager::tests::controller_start_with_fake_coordinator
```

## Adapter suite harness (`adapter_zstack_suite`)

`crates/zigbee-apiarist/tests/adapter_zstack_suite.rs` ports the upstream
`test/adapter/z-stack/adapter.test.ts` suite against a serial-wire ZNP mock:

- `tests/adapter_zstack_suite/support.rs` mirrors `ZnpRequestMockBuilder`
  (request/response table, IN/OUT NV items, extended NV tables, data-confirm
  scripting, ZDO response injection, `dataRetrieve` chunking).
- `tests/adapter_zstack_suite/fixtures.rs` reproduces
  `baseZnpRequestMock`, `empty/commissioned 1.2/3.0.x/3.x.0` builders.
- `tests/adapter_zstack_suite/zdo_response.rs` encodes ZDO responses with the
  same layouts as `BuffaloZdo::read_response`.
- Backup JSON fixtures are extracted from upstream
  (`tests/fixtures/adapter_backups/`).

Coverage: 129 tests covering commissioning/backup restore/create errors,
legacy backups, config-adapter mismatch diagnostics, LED/transmit-power,
permit join, ZCL send/retry/route-discovery/timeout, group/all/inter-PAN,
incoming messages (including chunked `dataRetrieve`), device events, `waitFor`
and the ZDO payload table. Paired upstream cases (soft+hard reset, three
`sendZclFrameToAll` broadcasts, default-response variants, announce/network
address events, leave/rejoin, inter-PAN set/restore, 16/64-bit extended
messages) are exercised in one Rust test with both variants.

The suite exposed four port bugs fixed here: APS encryption used cluster
`0x003b` instead of `zigbeeDirectConfiguration` (61), `writeIeeeAddr` emitted
short buffers instead of upstream's two `uint32`s, raw non-ZCL
`incomingMsg` was dropped instead of emitted with `header: None`, and a
missing/zero backup version was reported as an unsupported version instead of
`Unknown backup format`, and `leaveInd` only honored a boolean `rejoin` while
the wire carries UINT8 (numeric non-zero is now treated as rejoin).

## Next slice

- Add retry/route-discovery and incoming-message transcripts from
  `znp.test.ts` / `adapter.test.ts`.
- Extend the scripted coordinator to the remaining R1 bridge surfaces
  (database/state/config, HA discovery, Windfront).
