# HA-007 — config switches, lock entities and cover movement templates

Rust implementation: `src/homeassistant.rs` (`CONFIG_SWITCH_LOOKUP`,
`config_switch_entity`, lock branch, `CoverFeatures`/`MotorState`,
`cover_value_template`), `src/zigbee/zcl/clusters/door_lock.rs`,
`src/bridge.rs` (lock state commands), `src/mqtt/mod.rs` (socket tests).
Behavior: HA-007; plan UI/HA completeness.

## Source contract

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2`:

- `homeassistant.ts:51–60` — `CONFIG_SWITCH_DISCOVERY_LOOKUP` keys are the
  `SWITCH_DIFFERENT` properties: `auto_lock`, `away_mode`, `comfort_smiley`,
  `enable_display`, `indicator`, `tilt_mode`, `valve_detection`,
  `window_detection`, each with `entity_category: "config"` and an icon.
- `homeassistant.ts:706–731` — a `switch`/set-capable `binary` expose with such a
  property becomes a `switch` with `object_id = property`,
  `name = expose.label`, `command_topic_postfix = property`,
  `state_on`/`state_off` and the lookup metadata.
- `homeassistant.ts:935–957` — lock: `object_id` `lock_<endpoint>`/`lock`,
  `value_template: {{ value_json["<state.property>"] }}`, `state_locked`
  (`value_on`) / `state_unlocked` (`value_off`), and
  `command_topic_postfix = state.property` **only for endpoints** (command topic
  becomes `<base>/<friendly>/<endpoint>/set/<state_property>`).
- `homeassistant.ts:961–1062` — cover:
  - `running` feature → template
    `{% if "<running>" in value_json and value_json["<running>"] %} {% if value_json["<position>"] > 0 %} closing {% else %} opening {% endif %} {% else %} stopped {% endif %}`;
  - `motor_state`/`moving` enum with state access → lookup states from
    `COVER_OPENING_LOOKUP` / `COVER_CLOSING_LOOKUP` / `COVER_STOPPED_LOOKUP`
    (`48–50`), then `state_opening`/`state_closing`, `state_open`/`state_closed`,
    `state_stopped` and the `{% set motor = … %}` template (`1003–1029`);
  - fallback template `{{ value_json["<state>"] }}` with `STOP` as stopped state.
- `homeassistant.ts:1893–1920` — `command_topic =
  <base>/<prefix>set<postfix>`.
- Tests: `test/extensions/homeassistant.test.ts:140–175` (config switches),
  `555–585` (endpoint switch).

## Rust change

- `config_switch_entity`: per-attribute command topic
  (`<base>/<friendly>[/<endpoint>]/set/<property>`), `object_id`/`unique_id` on
  the suffixed property, `payload_on/off` **and** `state_on/off`, lookup
  `entity_category`/`icon`, humanized name when the expose has no label.
  Applies to both `switch` and set-capable `binary` exposes whose base property
  is in the table; other binary exposes stay read-only `binary_sensor`s.
- Lock: `lock`/`lock_<endpoint>` discovery with the upstream payload; endpoint
  locks use the suffixed `state_lN` as command-topic postfix and in the
  template, non-endpoint locks use `<base>/<friendly>/set` with dot-notation.
- Door Lock cluster (0x0101): `LOCK`→Lock Door, `UNLOCK`→Unlock Door; the bridge
  `state` handler now selects OnOff → Cover → Lock based on the endpoint's input
  clusters, so a lock endpoint receives the right command.
- Cover templates: structured `CoverFeatures` (state/position/tilt/running/motor)
  with exact upstream `running` and `motor_state` template strings, lookup-based
  `state_opening`/`state_closing`/`state_stopped`, and unsuffixed placeholders
  because endpoint covers read the stripped snapshot topic.

## Tests and evidence

`cargo test --locked` → **404 passed** (372 library, 2 binary, 30 compatibility);
`cargo fmt --check` clean; clippy exit 0 with 5 pre-existing warnings; native
release build succeeds; `python3 -m unittest discover -s scripts/tests` → 26.

New tests:

- `config_switch_uses_per_attribute_command_topic` — exact config switch payload.
- `config_switch_endpoint_suffixes_command_topic` — `<friendly>/l1/set/indicator_l1`.
- `ordinary_binary_sensor_is_not_a_config_switch` — no regression.
- `lock_discovery_matches_upstream`, `endpoint_lock_uses_suffixed_attribute_command_topic`.
- `cover_running_template_reports_movement_direction` — exact template string.
- `cover_motor_state_template_matches_lookup_values` — states + template parts.
- `door_lock::tests` — command payloads and response status parsing.
- Socket event loop (`socket_event_loop_receives_commands_and_publishes_retained_cleanup`)
  now also asserts `set/window_detection` → `{window_detection: "ON"}` and
  `l1/set/state_l1` → endpoint 1 with `{state_l1: "LOCK"}` over a real TCP
  MQTT session.

## Divergences and gaps (not verified)

- Config-switch values now go through `toZigbee.convertSet` (CV-09): the bridge
  runs the converter, encodes the recorded writes/commands with the published
  `zcl-commands.json` type table and sends them. The generator run, complex ZCL
  types and a bridge-level fake-coordinator assertion are still missing.
- Lock writes are fire-and-forget (`send_zcl`); the Lock Door response status is
  parsed by the codec but not awaited, and no lock-specific attributes are read.
- `object_id`/`default_entity_id` follow this project's existing naming
  convention rather than z2m's friendly-name-derived `default_entity_id`
  (unchanged contract, documented in earlier rows).
- Cover `invert_cover`/calibration quirks, `no_position_support`, group covers
  and `command_topic_postfix` for locks without endpoints beyond the above are
  unverified; no live HA/broker or hardware evidence.
