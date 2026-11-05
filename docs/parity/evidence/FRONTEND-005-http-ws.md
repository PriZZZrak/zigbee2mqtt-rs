# FRONTEND-005 — frontend HTTP/WebSocket server

Implementation: `src/frontend/mod.rs`, `src/frontend/ws.rs`,
`src/config.rs` (`frontend`), `src/mqtt/mod.rs` (`PublishedEvent` bus),
`src/bin/evid04_frontend_candidate.rs`,
`src/bin/evid04_frontend_static_candidate.rs`,
`scripts/frontend_static_diff.py`.
Behavior: FRONTEND-005; related FRONTEND-001/004, EVID-004.

## Source contract (pinned app `344b6e03…`, `lib/extension/frontend.ts`)

- Binds `frontend.host`/`frontend.port` (`0.0.0.0` when no host; an absolute
  host path means a unix socket upstream).
- WebSocket path is `posix.join(base_url, "api")` (`/api` for the default `/`).
- With `frontend.auth_token`, `?token=` must match; otherwise the socket is
  upgraded and closed with code **4401 "Unauthorized"**.
- On connection, replays `mqtt.retainedMessages` (topic with the base topic
  stripped, payload parsed as JSON) and the state of every non-coordinator
  device (`{topic: device.name, payload: state + linkquality}`).
- Every bridge publish under the base topic is forwarded to all clients as
  `{topic, payload}`.
- A client text frame `{topic, payload}` is handed to `mqtt.onMessage` on
  `<base_topic>/<topic>`.
- With `disable_ui_serving: false` the same listener serves the Windfront assets
  through `createStaticFileServer` (`srvx`): index resolution, precompressed
  `.br`/`.gz` when accepted, `Cache-Control: no-store` for HTML, and a
  404 page with CSP/nosniff headers (`staticFileServer.ts`).

## Rust implementation

- Dependency-free HTTP/1.1 + RFC 6455 subset (`src/frontend/ws.rs`): SHA-1 and
  base64 (known vectors), handshake accept key, masked client frames,
  ping/pong/close, 1 MiB frame limit; no fragmentation (explicit error).
- Config: upstream key set (`enabled`, `host`, `port`, `base_url`,
  `auth_token`, `disable_ui_serving`, `package`), defaults `enabled=false`,
  `port=8080`, `base_url="/"`. Static assets are read from
  `$ZIGBEE2MQTT_FRONTEND_DIR` (OpenWrt package: `/www/zigbee2mqtt`), because a
  Rust binary cannot resolve the upstream npm package name.
- `MqttBridge` publishes a `PublishedEvent{topic,payload,retain}` copy on every
  successful publish (`connect_with_events`, used only when the frontend is
  enabled). The server keeps its own retained map and fans frames out to
  clients; retained replay + device states happen on connect.
- Client messages are parsed by the same `commands_from_publish` helper as MQTT
  input and queued directly into the bridge event loop (`dispatch_frontend_message`),
  matching upstream's direct `mqtt.onMessage` call. No broker round-trip and no
  echo of the command topic. Only recognized device commands / bridge requests
  are accepted; unknown topics are logged and ignored.
- Static: index, content types, `.br`/`.gz` selection, `Vary: Accept-Encoding`,
  `no-store` for HTML, and the exact `srvx` 404 HTML/headers.

## Evidence

Rust integration tests (`src/frontend/mod.rs` tests):

- `websocket_auth_replay_forwarding_and_static_files`: wrong token closes 4401;
  authorized connect replays `bridge/state` and the `bulb` device state; a
  client `bulb/set` frame reaches the bridge command queue as `SetDevice`
  **without** a broker publish; `GET /` serves the file.
- `base_url_redirects_without_trailing_slash`: `/z2m` → 301 `/z2m/`.
- `frontend::ws` tests: SHA-1 vectors, RFC 6455 accept key, base64, masked frame
  decode, oversized-frame rejection, frame round-trip.
- `scripts/frontend_static_diff.py` against the pinned `srvx` reference
  (`docs/parity/evidence/frontend/upstream-static-dump.json`):
  `STATIC: 4 passed, 0 failed` (index, asset, brotli asset, 404), all pins MATCH.
  Only contract fields are compared (status/content-type/cache-control/
  content-encoding/body); `date`, `etag`, `last-modified`, `connection`,
  `keep-alive`, `accept-ranges` are dynamic and ignored.

## Gaps

- No Node WebSocket differential yet: the pinned app's vitest harness cannot
  host the real `ws` server in this environment (`WebSocket.Server` CJS interop
  fails); attempts are recorded in the task log. The auth/close-code contract is
  asserted by our integration test and cited from source.
- Direct dispatch covers command/bridge-request topics only (as upstream); it is
  not exercised against a live broker/HA.
- TLS (`ssl_key`/`ssl_cert`), unix sockets, websocket fragmentation, cache
  validators, and full `srvx` header parity are not implemented.
- No browser/HA/HIL verification.
