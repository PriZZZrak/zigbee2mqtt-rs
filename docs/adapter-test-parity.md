# `adapter/adapter.test.ts` parity — per-case mapping

Source: `.upstream/zigbee-herdsman/test/adapter/adapter.test.ts`
(zigbee-herdsman v11.2.0, `c1a54e3d`), 52 `it` cases.
Rust suite: `crates/zigbee-apiarist/tests/adapter_base_suite.rs` (10 tests) plus
the existing suites named below.

This file covers **device discovery** (`findAllDevices` over
`SerialPort.list()` + mDNS) and TCP path parsing — not `waitFor`/`permitJoin`
(those live in `controller.test.ts` and `adapter/z-stack/adapter.test.ts`).
Upstream mocks `autoDetect().list` and `bonjour`; the port takes the same
inputs through `DiscoveryContext` (`serial_ports`, `mdns_services`,
`mdns_start_error`, `platform`), and the `Adapter.create(...)` cases reduce to
the same detection rules plus the adapter defaults asserted on the discovered
entries.

| # | upstream `it` | status |
|---|---|---|
| 1 | finds all devices | `detects_each_adapter` |
| 2 | finds all devices with quirks | `detects_ember_variants_and_skyconnect_defaults` (ZBT-2 460800/rtscts, SkyConnect rtscts) |
| 3 | handles serial error during find all devices | `handles_serial_and_mdns_errors_during_find_all_devices` |
| 4 | handles mdns error during find all devices | `handles_serial_and_mdns_errors_during_find_all_devices` |
| 5 | handles nested mdns error during find all devices | `handles_serial_and_mdns_errors_during_find_all_devices` |
| 6 | mDNS: for zstack as znp | `discovers_mdns_services_as_tcp_paths` |
| 7 | mDNS: falls back to host if no addresses | `discovers_mdns_services_as_tcp_paths` |
| 8 | mDNS: times out | not ported — `DiscoveryContext` is synchronous; a timed-out browser maps to `mdns_start_error` (row 5) |
| 9 | mDNS: given invalid path | not ported — no mDNS path is enabled in R1 |
| 10 | mDNS: returns invalid format | not ported — same |
| 11 | mDNS: errors inside mdns | `handles_serial_and_mdns_errors_during_find_all_devices` (`mdns_start_error`) |
| 12 | TCP: returns config with tcp path | `tcp_discovery_parses_supported_paths` |
| 13 | TCP: returns config with socket path | `tcp_discovery_parses_supported_paths` |
| 14 | TCP: returns config with hostname path | `tcp_discovery_parses_supported_paths` |
| 15 | TCP: invalid adapter | not ported — adapter-name validation for TCP is a `create` concern |
| 16 | TCP: handles IPv4 | `tcp_discovery_parses_supported_paths` |
| 17 | TCP: handles IPv6 | `tcp_discovery_parses_supported_paths` (`[::1]`, `[fe80::1]`) |
| 18 | TCP: handles domain name | `tcp_discovery_parses_supported_paths` |
| 19 | TCP: handles invalid IPv6 | `tcp_discovery_rejects_invalid_paths` |
| 20 | USB without config: detects each adapter | `detects_each_adapter` |
| 21 | detects on Windows with manufacturer present | `does_not_detect_zwa2_and_windows_cp210x` |
| 22 | detects on Windows without manufacturer present | `does_not_detect_zwa2_and_windows_cp210x` |
| 23 | uses default transportOptions of adapter | `uses_pnp_id_and_friendly_name_for_the_display_name` (Z-Stack keeps adapter defaults) + `detects_ember_variants_and_skyconnect_defaults` |
| 24 | uses provided options instead of default transportOptions of adapter | covered by `adapter_zstack_suite` transport option cases |
| 25 | detects with pnpId instead of path | `uses_pnp_id_and_friendly_name_for_the_display_name` |
| 26 | detects with conflict vendor+product IDs | `detects_each_adapter` (the detection table resolves the conflicts deterministically; ZWA-2 wins as *not detected*) |
| 27 | returns first from list with multiple adapters - nothing to match against | `sorts_serial_ports_by_path` |
| 28 | throws on failure to get autoDetect().list | `handles_serial_and_mdns_errors_during_find_all_devices` (upstream throws in `create`, the port reports the error from `discover_live_adapters`; `find_all_devices` returns an empty list) |
| 29 | throws on failure to detect with conflict vendor+product IDs | not ported — `create` path validation |
| 30 | detects SkyConnect ZBT-1 when ZWA-2 present | `does_not_detect_zwa2_and_windows_cp210x` (ZWA-2 excluded) |
| 31 | detects SkyConnect ZBT-2 when ZWA-2 present | `detects_ember_variants_and_skyconnect_defaults` |
| 32 | USB with adapter+path config: detects each adapter | `detects_each_adapter` |
| 33 | detects with multiple adapters connected | `sorts_serial_ports_by_path` |
| 34 | detects with pnpId instead of path | `uses_pnp_id_and_friendly_name_for_the_display_name` |
| 35 | uses default transportOptions of adapter | `uses_pnp_id_and_friendly_name_for_the_display_name` |
| 36 | detects with conflict vendor+product IDs | `detects_each_adapter` |
| 37 | returns instance anyway on failure to match | not ported — `create` fallback behavior |
| 38 | returns instance anyway on failure to match with different path | not ported — same |
| 39 | returns instance anyway on failure to get autoDetect().list | not ported — same |
| 40 | throws on failure to match invalid adapter | not ported — same |
| 41 | USB with adapter only config: detects each adapter | `detects_each_adapter` |
| 42 | detects with multiple adapters connected | `sorts_serial_ports_by_path` |
| 43 | detects with pnpId instead of path | `uses_pnp_id_and_friendly_name_for_the_display_name` |
| 44 | throws on failure to detect with conflict vendor+product IDs | not ported — `create` path |
| 45 | USB with path only config: detects each adapter | `detects_each_adapter` |
| 46 | detects with multiple adapters connected | `sorts_serial_ports_by_path` |
| 47 | detects with pnpId instead of path | `uses_pnp_id_and_friendly_name_for_the_display_name` |
| 48 | throws on failure to match with different path | not ported — `create` path |
| 49 | throws on failure to match when port info too limited | not ported — `create` path |
| 50 | emits disconnected on transport close | covered by `differential_transport` (close event) and `adapter_zstack_suite` |
| 51 | does not emit disconnected on unopened transport | covered by `differential_transport` |
| 52 | does not emit disconnected on manual close of transport | covered by `differential_transport` (manual close path) |

## Notes

- The `Adapter.create(...)` block (rows 32–49) is the upstream factory that
  resolves *which* discovered port to open and applies the adapter defaults.
  The Rust bridge resolves the port from `configuration.yaml` and passes the
  transport options directly (`serial.port`/`baudrate`/`rtscts`), so the
  discovery rules (asserted above) are the shared behaviour; the factory's
  throw/fallback branches are `not ported` and listed individually.
- `live_serial_ports()` is exercised as a smoke assertion (host without
  dongles must not fail).
