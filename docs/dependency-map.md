# Rust dependency map and wrapper policy

Purpose: every upstream JavaScript dependency has **one** Rust replacement and
**one** wrapper module. No feature may be re-implemented in two places. The
wrapper is the only module allowed to import the underlying crate.

Last verified against crates.io / `Cargo.lock`: **2026-11-14**.

## 1. Direct Rust dependencies

| Crate | Manifest | Locked | Latest stable | Status |
|---|---|---|---|---|
| `serde_json` | `1` | 1.0.154 | 1.0.154 | current |
| `serde_yaml_ng` | `0.10` | 0.10.0 | 0.10.0 | current; replaced deprecated `serde_yaml` |
| `tokio` | `1` | 1.54.1 | 1.54.1 | current; `test-util` is a dev-only feature |
| `rumqttc` | `0.25` | 0.25.1 | 0.25.1 | current; `default-features = false` (no native TLS) |
| `regex` | `1` | 1.13.1 | 1.13.1 | current; imported only by `z2m_rs::util::settings_schema` (Ajv wrapper) and `z2m_rs::util::utils` (`filterProperties`) |
| `zip` | `9.5.1` | 9.5.1 | 9.5.1 | current; `default-features = false, features = ["deflate"]` → `flate2` → pure-Rust `zlib-rs` |
| `base64` | `0.23.1` | 0.23.1 | 0.23.1 | current; replaces Node `Buffer` base64 in backup/icon paths |
| `md-5` | `0.11.0` | 0.11.0 | 0.11.0 | current; replaces Node `crypto.createHash("md5")` |
| `tokio-tungstenite` | `0.30` | 0.30.0 | 0.30.0 | handshake only (`default-features = false`); `z2m_rs::extension::frontend` |
| `futures-util` | `0.3.34` | 0.3.34 | 0.3.34 | `default-features = false, features = ["sink"]`; sink/stream helpers in `frontend` |
| `rustls` / `tokio-rustls` | `0.23` / `0.26` | 0.23.46 / 0.26.6 | 0.23.46 / 0.26.6 | frontend TLS; default features off (no aws-lc/ring) |
| `rustls-rustcrypto` | `0.1.0-pre.0` | 0.1.0-pre.0 | 0.1.0-pre.0 (pre-release) | pure-Rust RustCrypto `CryptoProvider` used by frontend TLS |
| `rustls-pemfile` | `2` | 2.2.0 | 2.2.0 | certificate/key parsing for frontend TLS |
| `socket2` | `0.6` | 0.6.5 | 0.6.5 | current |
| `libc` | `0.2` (unix only) | 0.2.191 | 0.2.191 | current; OS bindings for serial modem lines and `localtime_r` (no C crypto/codec) |
| `serial2-tokio` | `0.1` | 0.1.26 | 0.1.26 | current; `unix`/`windows` features are cfg-gated in the manifest |

Notes:

- `serde` is not a direct dependency of `z2m-rs`; it remains transitive through
  `serde_json`/`serde_yaml_ng` (and is a dev-dependency of `zigbee-apiarist`).
- `serde_yaml` 0.9 is deprecated (`0.9.34+deprecated`). Migrated to the
  maintained `serde_yaml_ng` 0.10 fork with the same API.
- No `*-sys`, `cc`, `bindgen` or `cmake` build dependency exists in the
  workspace tree as of 2026-11-14:
  ```bash
  cargo tree --workspace -e features | grep -E -- '(-sys v|cc v|bindgen|cmake)'
  # (no output)
  ```
  `libc` is used by `getrandom`/`socket2` for OS bindings only, not for C
  crypto or codec implementations.
- Verify feature sets with:
  ```bash
  cargo tree -e features -p zigbee-apiarist
  cargo tree -e features -p z2m-rs
  ```
  `rumqttc` must stay `default-features = false` (no native TLS/OpenSSL).

## 2. Upstream → Rust replacement

### zigbee-herdsman runtime dependencies

| Upstream | Upstream use | Rust replacement | Wrapper / API | Status |
|---|---|---|---|---|
| `@serialport/bindings-cpp`, `@serialport/stream`, `@serialport/parser-delimiter` | serial transport, framing | `serial2-tokio` | `adapter::transport::AdapterTransport` | PORTED (serial transport, modem-line `get()`); HIL not run yet (AGENTS §9) |
| `bonjour-service` | mDNS adapter discovery | none yet; planned `mdns-sd` 0.21.7 | `adapter::adapter_discovery` (`DiscoveryContext`) | PARTIAL; discovery is injected, live mDNS missing |
| `slip` | UNPI framing | custom, no crate | `adapter::z_stack::unpi::frame`, `Znp::onTransportData` | PORTED |
| `@date-fns/tz` | time cluster / DST | custom | `utils::time_service` | PARTIAL (DST/zone gaps) |
| `debounce` | device/controller debouncing | `tokio::time` | `z2m_rs::extension::receive` (`debouncers`, flush-on-conflict) | PORTED |
| `fast-deep-equal` | object equality | `serde_json::Value` `PartialEq` | use directly; do not add a second deep-equal helper | OK |
| `zigbee-on-host` | host adapter helper | not applicable | — | omitted |

### zigbee2mqtt runtime dependencies

| Upstream | Upstream use | Rust replacement | Wrapper / API | Status |
|---|---|---|---|---|
| `mqtt` | MQTT client | `rumqttc` | `z2m_rs::mqtt::Mqtt` | PORTED (connect/subscribe/retained/QoS, bridge topics, `bridge/request/*`, `stats()` with live `queued`); `mqtts://` not enabled yet |
| `js-yaml` | YAML read/write | `serde_yaml_ng` | `z2m_rs::util::yaml` | PORTED |
| `winston`, `winston-transport`, `winston-syslog` | logging, file/console/syslog | custom | `z2m_rs::util::logger`, `zigbee_apiarist::utils::logger` | PARTIAL (file/console done; syslog transport and timestamp formatting missing) |
| `ajv` | JSON-schema settings validation | custom schema subset on `regex` (no `jsonschema` crate) | `z2m_rs::util::settings_schema` (`validate`, `validate_non_required`, `restart_required`), used by `z2m_rs::util::settings` | PORTED for the pinned schema: type/enum/required/properties/pattern(Properties)/propertyNames/items/oneOf/$ref/minimum/maximum; `requiresRestart` computed for overrides; Ajv message text reproduced |
| `semver` | Node version gate in `index.js` (`satisfies(process.version, …)`) | not applicable | — | N/A: there is no Node runtime; no library version comparison is needed |
| `fflate` | ZIP for `bridge/request/backup`, onboarding unzip | `zip` 9.5.1 (deflate) + `base64` for the payloads | `z2m_rs::extension::bridge` (backup), `z2m_rs::util::onboarding` (`/submit-zip`) | PORTED (backup + onboarding unzip with traversal guards) |
| `humanize-duration` | relative `last_seen` strings | custom `utils::humanize_duration` (`largest: 2, round: true`) | `z2m_rs::util::utils` | PORTED (no extra crate); unit-tested against the upstream call shape |
| `throttleit` | request throttling | `tokio::time` | `controller::helpers::request_queue`, `extension::receive` throttle | PORTED |
| `srvx` | HTTP server for static assets/API | custom HTTP server, no `axum`/`hyper` | `z2m_rs::util::static_file_server`, `extension::frontend` | PORTED for API/serving logic; Windfront assets deferred to OpenWrt packaging |
| `ws` | WebSocket | `tokio-tungstenite` 0.30.0 | `z2m_rs::extension::frontend` | PORTED (protocol; `handshake`-only feature set) |
| `unix-dgram` | syslog socket | `std::os::unix::net::UnixDatagram` | `z2m_rs::util::logger` syslog transport | BLOCKED; must be `#[cfg(unix)]` with a no-op fallback on Windows |
| `bind-decorator` | `this` binding | language feature | — | omitted |
| `zigbee-herdsman` | Zigbee stack | `zigbee-apiarist` | workspace crate | PARTIAL |
| `zigbee-herdsman-converters` | device converters | `crates/zigbee-converters` (separate crate) | `zigbee_converters::{Definition, DefinitionRegistry, Expose, ...}` | PORTED minimal surface required by the bridge; converter database and authoring tooling excluded (AGENTS §5 Q4) |
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
| `regex` | `crates/z2m-rs/src/util/settings_schema.rs` (Ajv wrapper), `crates/z2m-rs/src/util/utils.rs` (`filterProperties`) |
| `serial2-tokio` | `crates/zigbee-apiarist/src/adapter/transport.rs` (tests may use `SerialPort::pair`) |
| `libc` | `crates/zigbee-apiarist/src/adapter/transport.rs` (modem lines), `crates/zigbee-apiarist/src/utils/datetime.rs` (`localtime_r` offset) |
| `socket2` | `crates/zigbee-apiarist/src/adapter/transport.rs` |
| `zip` | `crates/z2m-rs/src/extension/bridge.rs` (backup archive) |
| `base64` | `crates/z2m-rs/src/util/utils.rs` (icon data URLs), `crates/z2m-rs/src/extension/bridge.rs` (backup payload) |
| `md-5` | `crates/z2m-rs/src/util/utils.rs` (`md5_hex`; migration reuses it) |
| `rustls`, `tokio-rustls`, `rustls-rustcrypto`, `rustls-pemfile` | `crates/z2m-rs/src/extension/frontend.rs` (TLS section; `build_tls_acceptor`) |
| `tokio-tungstenite`, `futures-util` | `crates/z2m-rs/src/extension/frontend.rs` (WebSocket) |
| `tokio::net` (TCP client) | `crates/zigbee-apiarist/src/adapter/transport.rs`; `crates/z2m-rs/src/extension/frontend.rs` (server) |
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
- `std::os::unix::net::UnixDatagram` is unix-only and must stay behind
  `#[cfg(unix)]`; a non-unix build must fall back to a disabled syslog
  transport instead of failing to compile.

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
- replaced deprecated `serde_yaml` with `serde_yaml_ng`;
- hand-rolled MD5 and a duplicate base64-image parser in
  `util/settings_migration.rs` → `utils::md5_hex` (`md-5` crate) and
  `utils::match_base64_file`;
- `humanize-duration` replacement is the custom `utils::humanize_duration`
  (no `humantime` dependency; `largest: 2, round: true` semantics covered by a
  unit test).

Completed in this pass:

- `AdapterTransport::get()` is ported via `serial2` modem-line reads (the
  hardware run itself is tracked in AGENTS §9, not in this document).

Remaining gaps (tracked, do not hide):

- `z2m-rs/util/logger.rs` lacks upstream `timestamp_format` handling; it uses
  ISO-8601 instead of the malformed epoch string. Format parity is `PARTIAL`.
- syslog transport is not implemented; it needs `#[cfg(unix)]` plus a no-op
  fallback on non-unix targets.
- mDNS discovery is still injected rather than live (`mdns-sd` not added yet).
- `mqtts://` (TLS to the broker) is not enabled: `rumqttc` would need
  `use-rustls-no-provider` and the same `rustls-rustcrypto` provider as the
  frontend.
- TLS interoperability with Chrome/Safari/Firefox has not been validated yet;
  `rustls-rustcrypto` is a pre-release provider.
- gzip: pre-compressed `.gz` assets are served; on-the-fly compression is
  deferred together with the Windfront assets.
- onboarding: `lib/util/onboarding.ts` is ported (`util::onboarding`: data-dir
  preparation, `/data`, `/submit`, `/submit-zip`, failure page, ZIP extraction
  with traversal guards, `onboard()`, server loops); only the Windfront
  onboarding asset bundle is deferred to OpenWrt packaging.
- HIL on the Routerich AX3000 + Dongle-P has not been executed.
- `bridge/info.os.node_version` reports the Rust toolchain version
  (`rustc X.Y.Z`) because there is no Node runtime; upstream reports
  `process.version`.

## 5. Dependency update procedure

1. Check the pin in `.upstream/<package>/package.json`.
2. Check the Rust replacement version:
   ```bash
   cargo search <crate> --limit 1
   # or
   curl -s -H 'User-Agent: z2m-rs-dep-check' https://crates.io/api/v1/crates/<crate>
   ```
   For patch-level updates inside the current major line, prefer
   `cargo update --dry-run` / `cargo upgrade --dry-run` (cargo-edit) over the
   registry-wide search result.
3. Update the manifest constraint, run `cargo update -p <crate>`, and commit
   `Cargo.lock`.
4. Run `cargo fmt`, `cargo clippy --workspace --tests`, `cargo test --workspace`.
5. If the crate replaces an upstream dependency, update the table in this
   document and the parity matrix.
6. Never add a native/C dependency; verify with:
   ```bash
   cargo tree --workspace -e features | grep -E -- '(-sys v|cc v|bindgen|cmake)'
   ```
   Any `*-sys`/`cc`/`bindgen`/`cmake` entry means C code entered the build
   (OS binding crates such as `windows-sys`/`libc` are the only exceptions and
   must be reviewed case by case).
