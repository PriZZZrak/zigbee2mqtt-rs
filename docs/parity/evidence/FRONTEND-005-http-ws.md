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
  through `createStaticFileServer` (`srvx` `dist/static.mjs`):
  - extensionless paths try `path`, `path.html`, `path/index.html`; a trailing
    slash uses `index.html`;
  - precompressed `.br`/`.gz` variants are served when accepted, `br` before
    `gzip`, honouring `q=0`/`*` (quality only filters, it does not reorder);
  - text/* content types get `; charset=utf-8`; output/HTML/JSON/wasm get
    `Vary: Accept-Encoding`; HTML gets `Cache-Control: no-store`;
  - `ETag: W/"<size-hex>-<mtimeMs-hex>[-br|-gzip]"`, and `Last-Modified` capped
    to now (IMF-fixdate);
  - `Accept-Ranges: bytes` on unencoded responses; `Range` → `206` with
    `Content-Range`, unsatisfiable → `416` with `Content-Range: bytes */<size>`;
    `If-Range` must be the exact date (quoted/weak values never match);
  - `If-None-Match` (weak/list/`*`) → `304`, otherwise `If-Modified-Since` is
    parsed and compared (`last_modified <= since`) → `304`; 304 carries only
    `ETag`/`Last-Modified`/`Vary`/`Cache-Control`;
  - `HEAD` returns the same headers without a body;
  - miss → exact `sendNotFound` HTML with CSP/nosniff headers.

## Rust implementation

- Dependency-free HTTP/1.1 + RFC 6455 subset (`src/frontend/ws.rs`): SHA-1 and
  base64 (known vectors), handshake accept key, masked client frames,
  ping/pong/close, 1 MiB frame limit; no fragmentation (explicit error).
- Config: upstream key set (`enabled`, `host`, `port`, `base_url`,
  `auth_token`, `disable_ui_serving`, `package`), defaults `enabled=false`,
  `port=8080`, `base_url="/"`. An absolute `host` binds a unix socket instead of
  TCP (the HTTP/WebSocket handler is generic over the stream; a stale socket
  file is replaced only when it is a socket). Static assets are read from
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
- Static: index/extensionless resolution, content types, `.br`/`.gz` selection
  (br preferred), `Vary: Accept-Encoding`, `no-store` for HTML, ETag/
  Last-Modified generation, conditional requests, byte ranges, HEAD, and the
  exact `srvx` 404 HTML/headers.

## Evidence

Rust integration tests (`src/frontend/mod.rs` tests):

- `websocket_auth_replay_forwarding_and_static_files`: wrong token closes 4401;
  authorized connect replays `bridge/state` and the `bulb` device state; a
  later device publish is fanned out live; a client `bulb/set` frame reaches the
  bridge command queue as `SetDevice` **without** a broker publish; `GET /`
  serves the file.
- `base_url_redirects_without_trailing_slash`: `/z2m` → 301 `/z2m/`.
- `unix_socket_serves_the_websocket`: an absolute `frontend.host` binds a unix
  socket, accepts the RFC 6455 handshake and forwards a live `bridge/state`
  publish.
- `frontend::ws` tests: SHA-1 vectors, RFC 6455 accept key, base64, masked frame
  decode, oversized-frame rejection, frame round-trip.
- Unit tests for the static contract helpers: `weak_etag` (srvx format with
  encoding suffixes), `http_date`/`parse_http_date` (IMF-fixdate round-trip and
  `If-Modified-Since`), `parse_range` (open/closed/suffix/unsatisfiable/invalid),
  `accepted_encodings` (`q=0`, wildcard, br preference), and conditional
  validators (`If-None-Match` lists/`*`, `If-Range` rejecting quoted/weak).
- `scripts/frontend_static_diff.py` against the pinned `srvx` reference
  (`docs/parity/evidence/frontend/upstream-static-dump.json`, fixture mtimes
  pinned to 1700000000): `STATIC: 14 passed, 0 failed`, all pins MATCH:
  index, asset, brotli, gzip, br preference, HEAD, byte range `206`,
  unsatisfiable `416`, extensionless, 404, `If-None-Match` `304`,
  `If-Modified-Since` `304`, `If-Range` match `206` and stale `200`.
  Only contract fields are compared (status/content-type/cache-control/
  content-encoding/ETag/Last-Modified/Accept-Ranges/Content-Range/Vary/body);
  `date`, `connection`, `keep-alive` and length/chunking are dynamic.

## Gaps

- No Node WebSocket differential yet: the pinned app's vitest harness cannot
  host the real `ws` server in this environment (`WebSocket.Server` CJS interop
  fails); attempts are recorded in the task log. The auth/close-code contract is
  asserted by our integration test and cited from source.
- Direct dispatch covers command/bridge-request topics only (as upstream); it is
  not exercised against a live broker/HA.
- TLS (`ssl_key`/`ssl_cert`) and WebSocket fragmentation are not implemented;
  `disable_ui_serving` still binds an HTTP listener that answers plain-text 404
  instead of only serving the WebSocket.
- On-the-fly compression of large uncompressed assets (`COMPRESS_MIN_SIZE`) is
  not implemented; only precompressed variants are used.
- No browser/HA/HIL verification.
