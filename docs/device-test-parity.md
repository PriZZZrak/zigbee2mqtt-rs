# `device.test.ts` parity — per-case mapping

Source: `.upstream/zigbee-herdsman/test/device.test.ts` (v11.2.0, `c1a54e3d`),
19 `it` cases. Rust suite: `crates/zigbee-apiarist/tests/device_suite.rs`
(19 tests).

Upstream spies on `endpoint.readResponse` / `command` / `read` /
`defaultResponse`. The Rust model has no per-endpoint spies, so every case
asserts on the ZCL frames the device actually sends through the injected
adapter (`MockAdapter::sent_endpoint`): cluster, command, transaction sequence
number, direction, disable-default-response and payload. That is strictly more
observable than the upstream mocks.

| # | upstream `it` | Rust test | asserts |
|---|---|---|---|
| 1 | replies to basic read | `replies_to_basic_read` | one `readRsp` on `genBasic`, TSN 1, `zclVersion` with status SUCCESS, `0xfffa` with `NO_DATA` (0x86), no `read`/`defaultRsp` |
| 2 | replies to time read | `replies_to_time_read` | `readRsp` on `genTime` with a generated non-zero `time` + `0xfffa` |
| 3 | replies to unsupported read | `replies_to_unsupported_read` | `readRsp` on `genGroups` with attr `0` `NO_DATA` |
| 4 | replies to read with custom response | `replies_to_read_with_custom_response` | handler returning `false` still produces the default `readRsp` |
| 5 | replies to read with override custom response | `replies_to_read_with_override_custom_response` | handler returning `true` sends nothing |
| 6 | does not send default response for default response | `does_not_send_default_response_for_default_response` | no frame at all |
| 7 | replies to IAS Zone Enroll request | `replies_to_ias_zone_enroll_request` | `enrollRsp` `{enrollrspcode: 0, zoneid: 23}`, TSN, disable-default-response, no `defaultRsp` |
| 8 | fails to reply to IAS Zone Enroll request | `fails_to_reply_to_ias_zone_enroll_request` | `enrollRsp` then `defaultRsp` `statusCode: 1` for the enroll command |
| 9 | replies to Poll Control Check-in | `replies_to_poll_control_check_in` | `checkinRsp` `{startFastPolling:1, fastPollTimeout:0}` + `read(genPollCtrl)` + `fastPollStop`; second check-in answers `startFastPolling:0` only |
| 10 | fails to reply to Poll Control Check-in | `fails_to_reply_to_poll_control_check_in` | `checkinRsp` then `defaultRsp` FAILURE |
| 11 | fails to stop fast poll | `fails_to_stop_fast_poll` | `checkinRsp` + `read` + `fastPollStop` attempted, no `defaultRsp` |
| 12 | replies to GLOBAL with SUCCESS default response by default | `replies_to_global_with_success_default_response_by_default` | `defaultRsp` on `genOnOff`, `statusCode: 0`, direction inverted to `CLIENT_TO_SERVER` |
| 13 | replies to SPECIFIC from SERVER with SUCCESS default response by default | `replies_to_specific_from_server_with_success_default_response_by_default` | same on `genScenes` |
| 14 | replies to SPECIFIC from CLIENT with SUCCESS default response by default | `replies_to_specific_from_client_with_success_default_response_by_default` | `defaultRsp` direction `SERVER_TO_CLIENT` |
| 15 | replies with default response from caller | `replies_with_default_response_from_caller` | caller status `0x87` used, direction inverted |
| 16 | replies with non-SUCCESS default response even when disable default response is ON | `replies_with_non_success_default_response_even_when_disable_default_response_is_on` | non-SUCCESS status is sent despite `disableDefaultResponse` |
| 17 | skips replying with default response when deviced option set | `skips_replying_with_default_response_when_device_option_set` | `skipDefaultResponse` suppresses everything |
| 18 | de-duplicates replying with default response | `de_duplicates_replying_with_default_response` | duplicate TSN answered once, a new TSN again |
| 19 | skips replying with default response when Tuya device option set | `skips_replying_with_default_response_when_tuya_device_option_set` | `DISABLE_TUYA_DEFAULT_RESPONSE=1` + `_TZ…` manufacturer suppresses |

## Bugs found while porting

- `Endpoint::default_response` overwrote the caller's `direction` with
  `SERVER_TO_CLIENT`. Upstream spreads
  `{direction: SERVER_TO_CLIENT, ...options, transactionSequenceNumber}`, so a
  caller-provided direction wins — the device layer relies on this to answer
  with the inverted direction. The port emitted every default response with
  the incoming frame's direction.
- `Device::on_zcl_data` gated the default response on
  `!header.disableDefaultResponse` too early, so a non-SUCCESS status (or a
  caller-provided status) was never sent for frames that set
  `disableDefaultResponse`. Upstream keeps the early gate to
  broadcast/`command.response === undefined` and applies
  `disableDefaultResponse` only for SUCCESS (`device.ts:394`, `:588-597`).

Both are covered by the tests above (cases 12–16).
