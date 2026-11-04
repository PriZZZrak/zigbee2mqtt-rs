# HA-003 — Home Assistant discovery from prepared exposes

Rust implementation: `src/homeassistant.rs` (`configs_from_exposes`,
`publish_exposes_discovery`), `src/bridge.rs`
(`publish_device_discovery`). Behavior: HA-003; plan CV-05/HA.

## Source contract

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2`,
`lib/extension/homeassistant.ts`:

- discovery is derived from prepared exposes (`state.value_on/value_off`,
  features, units, device classes), not from ad-hoc cluster inference;
- switch payloads use `value_on.toString()` (line ~1174); binary sensors keep the
  raw `value_on/value_off` types (line ~1192);
- availability, device identifiers (`zigbee2mqtt_<ieee>`), `via_device`,
  unique IDs and object IDs follow the existing z2m template.

## Rust change

- `configs_from_exposes(device, exposes, base_topic, coordinator_ieee)` maps:
  - `numeric` → `sensor` with unit/device_class and
    `{{ value_json.<property> }}`;
  - `binary` with `state` + set access → `switch` with string payloads;
  - other `binary` → `binary_sensor` with raw `value_on/value_off` and known
    device classes (contact/occupancy/tamper/smoke/water_leak);
  - `switch` → `switch`;
  - `light` → JSON-schema light with brightness/color modes from features.
- `unique_id` follows `{ieee}_{property}_{base_topic}`; device/availability
  blocks reuse the existing certificate-compatible helpers.
- `publish_device_discovery` prefers upstream exposes when the definition
  evaluates to a non-empty array; otherwise it keeps the previous
  cluster-inference discovery. One path publishes per device, so normal
  discovery is not duplicated.

## Tests and evidence

- `prepared_exposes_map_to_discovery_configs` (golden): temperature unit/class/
  template/unique_id, battery, contact with raw boolean payloads, switch with
  string payloads, light schema/brightness/color modes.
- `function_exposes_output_uses_the_same_mapping`: the array produced by a
  function-valued expose maps identically.
- External converter tests cover array/function expose evaluation.
- Full suite: `cargo test --locked` → **330 passed** (298 library, 2 binary,
  30 compatibility); clippy exit 0 with 5 existing warnings; release build OK.

## Divergences and gaps (not verified)

- Only numeric/binary/switch/light exposes are mapped. Climate, cover, lock,
  fan, select, number, button, event, update and vendor-specific features are
  still using no/legacy inference.
- No real published-package exposes or Home Assistant instance test; fixtures
  only.
- Manufacturer-specific overrides, endpoint suffixes, `enabled_by_default`,
  entity category and discovery cleanup on rename/remove/exposes change are not
  ported.
- `publish_device_discovery` blocks on Boa evaluation in the bridge loop; the
  persistent-context ADR also affects discovery performance.
