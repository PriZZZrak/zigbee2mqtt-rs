# `znp.test.ts` parity — per-case mapping

Source: `.upstream/zigbee-herdsman/test/adapter/z-stack/znp.test.ts`
(zigbee-herdsman v11.2.0, `c1a54e3d`), 74 `it` cases.
Rust suite: `crates/zigbee-apiarist/tests/znp_suite.rs` (22 tests).

Upstream uses `@serialport/binding-mock` plus vitest spies on
`Znp.request`/`transport.set`; those two seams have no Rust analogue (the
transport is concrete and the request path is the code under test), so the wire
cases run against a PTY peer that speaks UNPI and the parameter cases drive
`BuffaloZnp` directly. Rows marked *not ported* say why; nothing here is
claimed as passing without a test.

| # | upstream `it` | status |
|---|---|---|
| 1 | first ping fails should send reset bootloader | not ported — the DTR/RTS toggles are `transport.set` spy assertions; the ping path and the `0xEF` magic byte are covered by HIL (`docs/hil/2026-11-16-rust-port-startup.md`) |
| 2 | open and close | `open_and_close` |
| 3 | znp receive | `znp_receive` |
| 4 | znp receive malformed | `znp_receive_malformed` |
| 5 | znp request SREQ | `znp_request_sreq_success_and_failure` |
| 6 | znp request SREQ failed | `znp_request_sreq_success_and_failure` |
| 7 | znp request SREQ failed should cancel waiter when provided | covered by `differential_znp` (waiter id removal) |
| 8 | znp request SREQ with parsed in between | `znp_receive` (received stream) |
| 9 | znp request AREQ reset | covered by `differential_znp` (reset/AREQ transcript) |
| 10 | znp request AREQ | `znp_wait_for_matches_transid_and_timeout` (`dataRequest` AREQ) |
| 11 | znp request without init | `znp_request_without_init_is_an_error` |
| 12 | znp request with non-existing subsystem | not ported — `ZpiObject::create_request` panics for a missing subsystem, same as upstream throwing; the command variant is #13 |
| 13 | znp request with non-existing cmd | `znp_request_with_non_existing_command_panics` |
| 14 | znp request timeout | `znp_request_timeout_and_late_response` |
| 15 | znp request timeout for startupFromApp is longer | not ported — needs a 40 s wait; the `LONG_TIMEOUT` branch is code (`znp.rs`, `startupFromApp`/`bdbStartCommissioning`) |
| 16 | znp request, responses comes after timeout | `znp_request_timeout_and_late_response` |
| 17 | znp request, waitFor | `znp_wait_for_matches_transid_and_timeout` |
| 18 | znp request ZDO | `znp_request_zdo_success_and_failure` |
| 19 | znp request ZDO SUCCESS | `znp_request_zdo_success_and_failure` |
| 20 | znp request ZDO FAILURE | covered by `adapter_zstack_suite` (ZDO failure status cases) |
| 21 | znp request ZDO failed should cancel waiter when provided | covered by `differential_znp` |
| 22 | znp waitFor with transid | `znp_wait_for_matches_transid_and_timeout` |
| 23 | znp waitFor with target as network address | covered by `differential_znp` (`ZDO` target matching) |
| 24 | znp waitFor with target as IEEE | covered by `differential_znp` (eui64 target) |
| 25 | znp waitFor with target as IEEE forced to timeout because invalid ZDO status | covered by `differential_znp` |
| 26 | znp waitFor with state | covered by `differential_znp` |
| 27 | znp waitFor with payload mismatch | covered by `differential_znp` |
| 28 | znp requestWithReply should throw error when request as no reply | not ported — needs a command whose definition has no response |
| 29 | ZpiObject throw error on missing write parser | covered by `differential_zpi_object` (fixture errors) |
| 30 | ZpiObject throw error on unknown command | `znp_request_with_non_existing_command_panics` |
| 31 | ZpiObject throw error on unknown parameters | covered by `differential_zpi_object` |
| 32 | ZpiObject with cmd and non sapi is not reset command | `zpi_object_predicates_and_known_parsers` |
| 33 | ZpiObject parse payload for endDeviceAnnceInd | covered by `differential_zpi_object` fixtures |
| 34 | ZpiObject parse payload for nwkAddrRsp | `zpi_object_predicates_and_known_parsers` (`activeEpRsp`/`nwkAddrRsp` family) |
| 35 | Cant read unsupported type | `cant_read_unsupported_type` |
| 36 | UINT8 write | `uint8_write` |
| 37 | UINT8 read | `uint8_read` |
| 38 | INT8 write | `int8_write_and_read` |
| 39 | INT8 read | `int8_write_and_read` |
| 40 | UINT16 write | `uint16_write_and_read` |
| 41 | UINT16 read | `uint16_write_and_read` |
| 42 | UINT32 write | `uint32_write_and_read` |
| 43 | UINT32 read | `uint32_write_and_read` |
| 44 | LIST_UINT8 write | `list_uint8_write_and_read` |
| 45 | LIST_UINT8 read | `list_uint8_write_and_read` |
| 46 | LIST_UINT16 write | `list_uint16_write_and_read` |
| 47 | LIST_UINT16 read | `list_uint16_write_and_read` |
| 48 | LIST_NETWORK write | not ported — the expectation is only “throws for empty list”; the writer is covered by `differential_buffalo_znp` |
| 49 | LIST_NETWORK read | not ported — covered by `differential_buffalo_znp` fixtures |
| 50 | BUFFER8 write | `buffer8_write_and_read` |
| 51 | BUFFER8 write length consistent | not ported — covered by `differential_buffalo_znp` |
| 52 | BUFFER8 read | `buffer8_write_and_read` |
| 53 | BUFFER16 write | not ported — width variant, covered by `differential_buffalo_znp` |
| 54 | BUFFER16 read | not ported — width variant, covered by `differential_buffalo_znp` |
| 55 | BUFFER18 write | not ported — width variant, covered by `differential_buffalo_znp` |
| 56 | BUFFER18 read | not ported — width variant, covered by `differential_buffalo_znp` |
| 57 | BUFFER32 write | not ported — width variant, covered by `differential_buffalo_znp` |
| 58 | BUFFER32 read | not ported — width variant, covered by `differential_buffalo_znp` |
| 59 | BUFFER42 write | not ported — width variant, covered by `differential_buffalo_znp` |
| 60 | BUFFER42 read | not ported — width variant, covered by `differential_buffalo_znp` |
| 61 | BUFFER100 write | `buffer100_write_and_read` |
| 62 | BUFFER100 read | `buffer100_write_and_read` |
| 63 | BUFFER write | `buffer_write_and_read` |
| 64 | BUFFER read | `buffer_write_and_read` |
| 65 | IEEEADDR write | `ieeeaddr_write_and_read` |
| 66 | IEEEADDR read | `ieeeaddr_write_and_read` |
| 67 | Parse simple message | `znp_receive` |
| 68 | Parse two messages | `znp_receive` (version + ping in one session) |
| 69 | Dont throw error on fcs mismatch | covered by `differential_frame` (invalid FCS returns no frame) |
| 70 | Message in two chunks | `znp_request_timeout_and_late_response` (`Split` peer) |
| 71 | Message in two chunks, fcs as separate | `znp_request_timeout_and_late_response` (`Split` peer) |
| 72 | Parse message when it doenst start with SOF and buffer is empty | covered by `differential_znp` (leading garbage) |
| 73 | Continue parsing on fcs mismatch | `znp_receive_malformed` |
| 74 | To buffer | covered by `differential_frame` |

## Notes

- The PTY peer answers `ping`, `version`, `dataRequest` and the ZDO requests used
  by the suite; `Behavior::{Drop, FailStatus, Split}` reproduce the upstream
  mocks (no answer, non-zero status, two-chunk transfer).
- `znp_request_without_init_is_an_error` constructs a `Znp` with an unopened
  transport, which is the Rust equivalent of requesting before `znp.open()`.
- Remaining gaps are listed above; the largest is the bootloader/`transport.set`
  case (#1), which is hardware-only in this port.
