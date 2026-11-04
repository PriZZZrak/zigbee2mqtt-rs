# HA-006 — cover entities and per-attribute set topics

Rust implementation: `src/zigbee/zcl/clusters/cover.rs`,
`src/homeassistant.rs` (`cover_discovery_config`, `endpoint_cover_configs`,
cover branch in `configs_from_exposes` and the cluster fallback),
`src/mqtt/mod.rs` (`parse_device_command` attribute handling, `<base>/#`
subscription), `src/bridge.rs` (cover state/position/tilt set, `read_target`
cover keys, cover `state` get fallback). Behavior: HA-006; plan UI/HA
completeness.

## Source contract

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2`:

- `lib/extension/homeassistant.ts:961–1062` (`cover`): `object_id`
  `cover_<endpoint>` or `cover`, `command_topic_prefix`, `state_topic_postfix`,
  `state_open`/`state_closed`/`state_stopped` = `OPEN`/`CLOSE`/`STOP`, and
  `value_template: {{ value_json["state"] }}` on the (stripped) state topic; if
  the expose has `position`, adds `position_template`, `set_position_template:
  { "<property>": {{ position }} }`, `set_position_topic`, `position_topic`; if
  it has `tilt`, adds `tilt_command_topic` (`<...>/set/tilt`),
  `tilt_status_topic`, `tilt_status_template`.
- `homeassistant.ts:1537–1577`: the postfix republish covers
  `cover_<endpoint>` as well as `light_<endpoint>`.
- `homeassistant.ts:1893–1920`: `command_topic` =
  `<base>/<prefix>set<postfix>`; `payload.command_topic_postfix` is used for
  switches whose property differs from `state` and for locks.
- `lib/extension/publish.ts:35–56,60–64`: the topic regex is
  `^<base>/(?!bridge)(.+?)/(get|set)(?:/(.+))?$`; an attribute part turns the
  payload into `{[attribute]: value}` (JSON when parseable, else the raw string).
- `lib/extension/publish.ts:183–214,237–263`: `state_<endpoint>` keys override
  the target endpoint and endpoint converter output is suffixed.

## Rust change

- New Window Covering cluster (0x0102): UpOpen/DownClose/Stop command payloads
  for `OPEN`/`CLOSE`/`STOP`, and `currentPositionLiftPercentage` (0x0008) /
  `currentPositionTiltPercentage` (0x0009) write payloads clamped to 0–100.
- Cover discovery for both exposes and the cluster fallback:
  `object_id` `cover`/`cover_lN`, `unique_id` `{ieee}_cover[_lN]_{base}`,
  main state topic for single-endpoint and `<base>/<friendly>/<lN>` for
  multi-endpoint, `command_topic` `<...>/set`, `state_open/closed/stopped`,
  optional position (`position_template`, `set_position_template`,
  `set_position_topic`, `position_topic`) and tilt (`tilt_command_topic`,
  `tilt_status_topic`, `tilt_status_template`).
- Cleanup topics include cover entities (single and per-endpoint).
- `handle_set` routes `state` to OnOff or Cover depending on the endpoint's
  clusters, and handles `position`/`tilt` plus suffixed variants by writing the
  covering attributes. Optimistic state is limited to the commanded keys.
- `read_target("position"/"tilt")` reads the covering attributes; `get` of
  `state` on a covering device falls back to the lift percentage.
- MQTT command parsing now mirrors z2m: the first `get`/`set` segment splits the
  entity path from an optional attribute; the payload becomes `{attribute:
  value}`; endpoint selection uses a trailing `lN`/`button` segment. The bridge
  subscribes to `<base>/#` (like z2m) instead of fixed-depth wildcards, so
  nested friendly names, endpoints and per-attribute topics all arrive.

## Tests and evidence

`cargo test --locked` → **395 passed** (363 library, 2 binary, 30 compatibility);
`cargo fmt --check` clean; clippy exit 0 with 5 existing warnings; native release
build succeeds; `python3 -m unittest discover -s scripts/tests` → 26 passed.

New tests:

- `cover::tests::state_payload_maps_open_close_stop`,
  `position_and_tilt_write_percentage_attributes`.
- `cover_discovery_maps_position_and_tilt`,
  `cover_without_position_or_tilt_omits_optional_topics`,
  `multi_endpoint_cover_uses_postfix_state_topic`,
  `cover_cleanup_topics_include_all_endpoints`.
- `cover_state_get_targets_lift_position`.
- `per_attribute_set_topics_wrap_the_value` (set/attribute, endpoint+attribute,
  JSON values), `per_attribute_get_topic_requests_one_key`,
  `bridge_topics_are_not_parsed_as_device_commands`.
- Socket-level event loop (see below): commands and retained cleanup over TCP.

## Socket-level MQTT event-loop tests

`src/mqtt/mod.rs::fake_broker` is a minimal in-process MQTT 3.1.1 server
(CONNECT/CONNACK, SUBSCRIBE/SUBACK, PUBLISH both directions, PUBACK,
PINGREQ/PINGRESP). Two tests use the real `rumqttc` client and bridge publish
path over a TCP socket:

- `socket_event_loop_receives_commands_and_publishes_retained_cleanup`: receives
  `<name>/l2/set` as `SetDevice{endpoint: Some(2)}` and
  `<name>/get/position` as `GetDevice{payload: {"position": ""}}`; then publishes
  an empty, retained, QoS1 discovery cleanup observed by the broker.
- `device_remove_over_real_socket_publishes_retained_cleanup_and_response`:
  `process_device_remove` over the real bridge publishes two retained empty
  cleanup configs and the `bridge/response/device/remove` payload with the
  transaction, observed on the socket.

## Divergences and gaps (not verified)

- No live mosquitto/Home Assistant instance; the broker is a test double and the
  client is the real library, so reconnect/auth/TLS and HA entity behaviour are
  still untested.
- Cover semantics are generic: no `running`/`motor_state` templates, no device
  quirks (calibration, inversion), no `set_position_template` per-device
  property mapping beyond `position`, and `command_topic_postfix` is emitted only
  for cover tilt, not for `SWITCH_DIFFERENT` switches or locks.
- Cover writes are fire-and-forget (`send_zcl`), not confirmed via
  `request_zcl`.
- Group covers, `cover` exposes with `endpoint` names other than `lN` in the
  cluster fallback, and discovery cleanup on exposes change are unverified.
- No hardware evidence; HA-006 stays `IN_PROGRESS`.
