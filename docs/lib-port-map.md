# `lib/` port map (upstream zigbee2mqtt 2.14.3, `3c841fdf`)

Every runtime file under `.upstream/zigbee2mqtt/lib/` has one Rust destination
and one or more behavioural test files. Type-only declarations have no runtime
counterpart.

| Upstream file | Rust destination | Status | Evidence |
|---|---|---|---|
| `controller.ts` | `crates/z2m-rs/src/controller.rs` | PORTED | `tests/controller.rs`, `tests/controller_lifecycle.rs`, `tests/last_seen.rs` (publishEntityState, extension wiring, cached state, last_seen formats) |
| `eventBus.ts` | `crates/z2m-rs/src/event_bus.rs` | PORTED | `tests/on_event.rs`, `tests/groups.rs`, `tests/homeassistant.rs` (event dispatch, stats) |
| `mqtt.ts` | `crates/z2m-rs/src/mqtt.rs` | PORTED | `tests/mqtt_broker.rs` (real aedes broker: retained online state, QoS1 delivery, live `queued`), unit tests for server parsing and the pure-Rust TLS config |
| `state.ts` | `crates/z2m-rs/src/state.rs` | PORTED | `tests/state.rs` |
| `zigbee.ts` | `crates/z2m-rs/src/zigbee.rs` | PORTED | `tests/zigbee.rs` |
| `model/device.ts` | `crates/z2m-rs/src/model/device.rs` | PORTED | `tests/model.rs`, converter/publish tests |
| `model/group.ts` | `crates/z2m-rs/src/model/group.rs` | PORTED | `tests/model.rs`, `tests/groups.rs` |
| `util/data.ts` | `crates/z2m-rs/src/util/data.rs` | PORTED | `tests/data.rs` |
| `util/logger.ts` | `crates/z2m-rs/src/util/logger.rs` | PORTED | `tests/logger.rs`, `tests/logger_timestamp.rs` (console/file transports, winston/fecha `timestamp_format` tokens, timed file lines, namespaced levels, syslog RFC 5424 over udp/tcp/unix) |
| `util/objectAssignDeep.ts` | `crates/z2m-rs/src/util/object_assign_deep.rs` | PORTED | `tests/object_assign_deep.rs` |
| `util/onboarding.ts` | `crates/z2m-rs/src/util/onboarding.rs` | PORTED | unit tests in `util/onboarding.rs` (`/data`, `/submit`, `/submit-zip`, ZIP traversal guards, failure payload); Windfront onboarding assets deferred |
| `util/sd-notify.ts` | `crates/z2m-rs/src/util/sd_notify.rs` | PORTED | `tests/sd_notify.rs` |
| `util/settings.ts` | `crates/z2m-rs/src/util/settings.rs` | PORTED | `tests/settings.rs`, `tests/settings_schema.rs` |
| `util/settingsMigration.ts` | `crates/z2m-rs/src/util/settings_migration.rs` | PORTED | `tests/settings_migration.rs` (versions 1→5, notes, icon migration) |
| `util/staticFileServer.ts` | `crates/z2m-rs/src/util/static_file_server.rs` | PORTED | `tests/static_file_server.rs` |
| `util/stringify.ts` | `crates/z2m-rs/src/util/stringify.rs` | PORTED | `tests/utils.rs` (`stable_stringify`) |
| `util/utils.ts` | `crates/z2m-rs/src/util/utils.rs` | PORTED | `tests/utils.rs`, `tests/calibration.rs` (formatDate/humanize, filterProperties, response helpers) |
| `util/yaml.ts` | `crates/z2m-rs/src/util/yaml.rs` | PORTED | `tests/settings.rs`, `tests/settings_migration.rs` |
| `extension/extension.ts` | `crates/z2m-rs/src/extension/extension.rs` | PORTED | every extension test builds on `ExtensionContext` |
| `extension/availability.ts` | `crates/z2m-rs/src/extension/availability.rs` | PORTED | `tests/availability.rs` (incl. paused-clock timers) |
| `extension/bind.ts` | `crates/z2m-rs/src/extension/bind.rs` | PORTED | `tests/bind.rs`; adapter-backed paths HARDWARE_PENDING |
| `extension/bridge.ts` | `crates/z2m-rs/src/extension/bridge.rs` | PORTED | `tests/bridge.rs` (17 tests); approved OMITTED handlers listed in `docs/parity-matrix.md` |
| `extension/configure.ts` | `crates/z2m-rs/src/extension/configure.rs` | PORTED | `tests/configure.rs` |
| `extension/frontend.ts` | `crates/z2m-rs/src/extension/frontend.rs` | PORTED | `tests/frontend.rs` (WS, auth, replay, TLS, unix socket); static assets deferred |
| `extension/groups.ts` | `crates/z2m-rs/src/extension/groups.rs` | PORTED | `tests/groups.rs` |
| `extension/health.ts` | `crates/z2m-rs/src/extension/health.rs` | PORTED | `tests/health.rs` |
| `extension/homeassistant.ts` | `crates/z2m-rs/src/extension/homeassistant.rs` | PORTED | `tests/homeassistant.rs` (29 tests); per-case map in `docs/extension-test-cases.md` |
| `extension/networkMap.ts` | `crates/z2m-rs/src/extension/network_map.rs` | PORTED | `tests/network_map.rs` |
| `extension/onEvent.ts` | `crates/z2m-rs/src/extension/on_event.rs` | PORTED | `tests/on_event.rs` |
| `extension/publish.ts` | `crates/z2m-rs/src/extension/publish.rs` | PORTED | `tests/publish.rs`; converter-database cases tracked in the per-case map |
| `extension/receive.ts` | `crates/z2m-rs/src/extension/receive.rs` | PORTED | `tests/receive.rs`, `tests/calibration.rs` |
| `extension/externalJS.ts` | — | OMITTED | user JavaScript evaluation forbidden (AGENTS §5 Q5) |
| `extension/externalConverters.ts` | — | OMITTED | same |
| `extension/externalExtensions.ts` | — | OMITTED | same |
| `extension/otaUpdate.ts` | — | OMITTED | OTA out of R1 (AGENTS §4) |
| `types/api.ts` | — | types only | no runtime behaviour |
| `types/dom.shim.d.ts` | — | types only | no runtime behaviour |
| `types/types.d.ts` | — | types only | no runtime behaviour |
| `types/unix-dgram.d.ts` | — | types only | replaced by `std::os::unix::net::UnixDatagram` |
| `types/zigbee2mqtt-frontend.d.ts` | — | types only | no runtime behaviour |

## Verification commands

```bash
cargo test -p z2m-rs            # 33 suites (extension + util + model)
cargo test -p zigbee-apiarist   # 953 passed, 0 failed, 11 ignored
cargo clippy --workspace --tests
cargo tree --workspace -e features | grep -E -- '(-sys v|cc v|bindgen|cmake)'
```

Known non-porting gaps live in `docs/parity-matrix.md`: HIL/adapter rows are
`HARDWARE_PENDING`, the converter database is excluded by AGENTS §5 Q4, and the
Windfront static assets are deferred to OpenWrt packaging.
