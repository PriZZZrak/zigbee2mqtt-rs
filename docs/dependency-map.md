# Rust dependency map and wrapper policy

Purpose: every upstream JavaScript dependency has **one** Rust replacement and
**one** wrapper module. No feature may be re-implemented in two places. The
wrapper is the only module allowed to import the underlying crate.

Last verified against crates.io: **2026-11-09**.

## 1. Direct Rust dependencies

| Crate | Manifest | Locked | Latest stable | Status |
|---|---|---|---|---|
| `serde_json` | `1` | 1.0.154 | 1.0.154 | current |
| `serde_yaml_ng` | `0.10` | 0.10.0 | 0.10.0 | current; replaced deprecated `serde_yaml` |
| `tokio` | `1` | 1.54.1 | 1.54.1 | current |
| `rumqttc` | `0.25` | 0.25.1 | 0.25.1 | current |
| `socket2` | `0.6` | 0.6.5 | 0.6.5 | current |
| `serial2-tokio` | `0.1` | 0.1.26 | 0.1.26 | current |

Notes:

- `serde` was removed as a direct dependency; no crate derives `Serialize` /
  `Deserialize` yet. It remains transitive through `serde_json`/`serde_yaml_ng`.
- `serde_yaml` 0.9 is deprecated (`0.9.34+deprecated`). Migrated to the
  maintained `serde_yaml_ng` 0.10 fork with the same API.
- Verify with:
  ```bash
  cargo tree -e features -p zigbee-apiarist
  cargo tree -e features -p z2m-rs
  ```
  `rumqttc` must stay `default-features = false` (no native TLS/OpenSSL).

## 2. Upstream → Rust replacement

### zigbee-herdsman runtime dependencies

| Upstream | Upstream use | Rust replacement | Wrapper / API | Status |
|---|---|---|---|---|
| `@serialport/bindings-cpp`, `@serialport/stream`, `@serialport/parser-delimiter` | serial transport, framing | `serial2-tokio` | `adapter::transport::AdapterTransport` | PORTED (serial); transcript/HIL pending |
| `bonjour-service` | mDNS adapter discovery | none yet; planned `mdns-sd` 0.21.7 | `adapter::adapter_discovery` (`DiscoveryContext`) | PARTIAL; live mDNS missing |
| `slip` | UNPI framing | custom, no crate | `adapter::z_stack::unpi::frame`, `Znp::onTransportData` | PORTED |
| `@date-fns/tz` | time cluster / DST | custom | `utils::time_service` | PARTIAL (DST/zone gaps) |
| `debounce` | device/controller debouncing | `tokio::time` (planned helper) | none yet | BLOCKED until controller slice |
| `fast-deep-equal` | object equality | `serde_json::Value` `PartialEq` | use directly; do not add a second deep-equal helper | OK |
| `zigbee-on-host` | host adapter helper | not applicable | — | omitted |

### zigbee2mqtt runtime dependencies

| Upstream | Upstream use | Rust replacement | Wrapper / API | Status |
|---|---|---|---|---|
| `mqtt` | MQTT client | `rumqttc` | `z2m_rs::mqtt::Mqtt` | wrapper exists; bridge topics missing |
| `js-yaml` | YAML read/write | `serde_yaml_ng` | `z2m_rs::util::yaml` | PORTED |
| `winston`, `winston-transport`, `winston-syslog` | logging, file/console/syslog | custom | `z2m_rs::util::logger`, `zigbee_apiarist::utils::logger` | PARTIAL (syslog/timestamp format) |
| `ajv` | JSON-schema settings validation | none yet; planned `jsonschema` 0.61.0 | `z2m_rs::util::settings` (validation) | PARTIAL |
| `semver` | version checks | none yet; planned `semver` 1.0.28 | `z2m_rs::util::settings_migration` | PARTIAL |
| `fflate` | gzip | none yet; planned `flate2` 1.1.11 (`miniz_oxide`) | frontend/assets | BLOCKED |
| `humanize-duration` | uptime strings | none yet; planned `humantime` 2.4.0 or custom | `bridge/info` | BLOCKED |
| `throttleit` | request throttling | `tokio::time` (planned helper) | `controller::helpers::request_queue` | PARTIAL |
| `srvx` | HTTP server | custom + planned `axum` 0.8.9 / `hyper` 1.12.0 | `z2m_rs::util::static_file_server` | PARTIAL |
| `ws` | WebSocket | none yet; planned `tokio-tungstenite` 0.30.0 | frontend protocol | BLOCKED |
| `unix-dgram` | syslog socket | `std::os::unix::net::UnixDatagram` | `util::logger` syslog transport | BLOCKED |
| `bind-decorator` | `this` binding | language feature | — | omitted |
| `zigbee-herdsman` | Zigbee stack | `zigbee-apiarist` | workspace crate | PARTIAL |
| `zigbee-herdsman-converters` | device converters | not ported (R1 excludes them) | — | out of scope |
| `zigbee2mqtt-frontend`, `zigbee2mqtt-windfront` | static frontend | served assets | `util::static_file_server` | assets only |

### Development tooling

| Upstream | Rust replacement | Notes |
|---|---|---|
| `vitest` | `cargo test` | plus differential fixtures (see `docs/porting-parity.md`) |
| `@serialport/binding-mock` | `serial2_tokio::SerialPort::pair` + `AdapterTransport::attach_serial` | tests only |
| `@biomejs/biome` | `cargo fmt` + `cargo clippy` | no Biome in Rust |
| `typescript` | `rustc` | — |

## 3. Wrapper rules (mandatory)

One concern, one wrapper. The following imports are only allowed in the listed
module:

| Crate / API | Only allowed in |
|---|---|
| `rumqttc` | `crates/z2m-rs/src/mqtt.rs` |
| `serde_yaml_ng` | `crates/z2m-rs/src/util/yaml.rs` |
| `serial2-tokio` | `crates/zigbee-apiarist/src/adapter/transport.rs` (tests may use `SerialPort::pair`) |
| `socket2` | `crates/zigbee-apiarist/src/adapter/transport.rs` |
| `tokio::net` (TCP client) | `crates/zigbee-apiarist/src/adapter/transport.rs`; frontend/server module when added |
| `SystemTime::now`, `Date.now()` equivalent | `crates/zigbee-apiarist/src/utils/datetime.rs` |
| upstream `wait(ms)` | `crates/zigbee-apiarist/src/utils/wait.rs` |
| upstream `AsyncMutex` | `crates/zigbee-apiarist/src/utils/async_mutex.rs` |
| upstream `Waitress` | `crates/zigbee-apiarist/src/utils/waitress.rs` |
| file paths under the data dir | `crates/z2m-rs/src/util/data.rs` |
| MQTT payload serialization | `crates/z2m-rs/src/mqtt.rs` |
| YAML serialization | `crates/z2m-rs/src/util/yaml.rs` |

Allowed exceptions:

- `serde_json` is the canonical in-memory JSON type; using `serde_json::Value`
  and `json!` directly is allowed. Do not add a second JSON layer.
- `std::fs` is allowed for direct file I/O (upstream also uses `fs` directly),
  but paths must come from `util::data`, not hand-built strings.
- `tokio::sync::Mutex`/`RwLock` are allowed for internal shared state (logger
  registry, received buffers). Upstream `AsyncMutex` request serialization must
  use `utils::async_mutex`.
- `tokio::time::sleep` is allowed for runtime backoff/reconnect; upstream
  `wait(ms)` must use `utils::wait`.

If a new crate is needed:

1. add a row to this document first;
2. state which upstream dependency/file it replaces;
3. create the single wrapper module;
4. forbid direct imports elsewhere in review;
5. add a differential test if it affects observable behaviour.

## 4. Known duplication removed / remaining

Removed in this pass:

- `iso8601_now`/`civil_from_days` duplicated in `backup.rs` and
  `z2m-rs/util/logger.rs` → `utils::datetime`;
- ad-hoc `now_ms` in `event_bus.rs`, `controller/helpers/request.rs`,
  `request_queue.rs` → `utils::datetime::now_ms`;
- `eprintln!` in `Znp` → `utils::logger`;
- direct `serial2_tokio::available_ports` in discovery →
  `AdapterTransport::available_ports`;
- removed unused direct `serde` dependency;
- replaced deprecated `serde_yaml` with `serde_yaml_ng`.

Remaining gaps (tracked, do not hide):

- `z2m-rs/util/logger.rs` lacks upstream `timestamp_format` handling; it now uses
  ISO-8601 instead of the malformed epoch string. Format parity is `PARTIAL`.
- syslog transport, JSON-schema validation, WebSocket, gzip, semver and
  humanize-duration are not implemented yet.
- mDNS discovery is still injected rather than live.
- `AdapterTransport::get()` is ported via serial2 modem-line reads and verified on hardware.

## 5. Dependency update procedure

1. Check the pin in `.upstream/<package>/package.json`.
2. Check the Rust replacement version:
   ```bash
   cargo search <crate> --limit 1
   # or
   curl -s -H 'User-Agent: z2m-rs-dep-check' https://crates.io/api/v1/crates/<crate>
   ```
3. Update the manifest constraint, run `cargo update -p <crate>`, and commit
   `Cargo.lock`.
4. Run `cargo fmt`, `cargo clippy --workspace --tests`, `cargo test --workspace`.
5. If the crate replaces an upstream dependency, update the table in this
   document and the parity matrix.
6. Never add a native/C dependency; verify with `cargo tree -e features`.
