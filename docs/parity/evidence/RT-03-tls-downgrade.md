# RT-03 — reject `mqtts://` instead of silently downgrading

Rust implementation: `src/config.rs` (scheme detection), `src/mqtt/mod.rs`
(connect guard). Behavior: MQTT-011; plan RT-03.

## Contract and prior behavior

Upstream `mqtt.ts` passes the configured server to the MQTT client, which
supports TLS when the URI/options request it. This Rust build has no TLS
transport (`rumqttc` is built with `default-features = false`, and no pure-Rust
TLS provider has been approved yet).

Previously the loader stripped `mqtts://` and the bridge connected with plain
TCP to the same host/port. That is a silent security downgrade and would fail or
expose credentials depending on the broker.

## Change

- `MqttConfig::tls_required` is set when the original server begins with
  `mqtts://`. It is `#[serde(skip)]`, so it is runtime-only and never written
  back.
- `MqttBridge::connect` returns a configuration error before opening any socket:
  “mqtts:// requires TLS, which this build does not support; refusing to fall
  back to plain TCP”.
- The original URI is preserved by the lossless configuration writer
  (STORE-01), so a future TLS implementation can use it.

## Tests added

- `config::tests::mqtts_scheme_marks_tls_required_without_rewriting_the_uri`:
  parses host/port, sets the flag, saves without a `tls_required` key and keeps
  `mqtts://host:8883` in YAML.
- `mqtt::tests::mqtts_is_rejected_instead_of_downgrading_to_plain_tcp`: connect
  fails with the explicit TLS error (no network attempted).

## Remaining gaps (not verified)

- TLS is still unimplemented; a pure-Rust provider decision (ADR-06) is open.
- `mqtts` with credentials from `!secret` was not exercised; the error path is
  tested without secrets.
- Upstream `mqtt.user`/reference precedence and full auth behaviour remain
  CONFIG-004/RT-03 gaps.
