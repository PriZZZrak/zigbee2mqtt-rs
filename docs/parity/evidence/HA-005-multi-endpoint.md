# HA-005 — multi-endpoint discovery, set topics and state republish

Rust implementation: `src/homeassistant.rs` (`on_off_endpoints`,
`endpoint_on_off_configs`, `expose_endpoint`, per-endpoint branches),
`src/bridge.rs` (`endpoint_suffix`, `parse_endpoint_key`,
`endpoint_state_payloads`, `handle_set`), `src/mqtt/mod.rs`
(`endpoint_index`, `endpoint_state_topic`, endpoint wildcards, `SetDevice.endpoint`).
Behavior: HA-005; plan UI/HA completeness.

## Source contract

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2`:

- `lib/extension/homeassistant.ts:628–712` — `endpointName = exposes[0].endpoint`;
  a light becomes `object_id: light_<endpoint>`, `command_topic_prefix:
  <endpoint>`, `state_topic_postfix: <endpoint>`; a switch keeps the main
  `state_topic`, uses `value_template: {{ value_json["state_<endpoint>"] }}`,
  `object_id: <friendly>_<endpoint>`, `unique_id:
  <ieee>_switch_<endpoint>_<base>`, and `command_topic_prefix: <endpoint>`.
- Finalization (`1789–1791`, `1893–1900`): `state_topic` becomes
  `<base>/<friendly>/<endpoint>`; `command_topic` becomes
  `<base>/<friendly>/<endpoint>/set`.
- `lib/extension/homeassistant.ts:1537–1577` (`onPublishEntityState`): for every
  discovered `light_<endpoint>`/`cover_<endpoint>`, collect state keys ending in
  `_<endpoint>`, strip the suffix and publish that JSON to
  `<base>/<friendly>/<endpoint>`.
- `lib/extension/publish.ts:36–56,183–214`: set topics accept
  `<base>/<friendly>/<endpoint>/set` and keys such as `state_<endpoint>`; the
  converter result is suffixed with the endpoint name.
- Tests: `test/extensions/homeassistant.test.ts:555–585` (per-endpoint switch
  payload), `test/extensions/homeassistant.test.ts:2746–2775`
  (light-with-postfix republish), `test/extensions/publish.test.ts:282,306`
  (`state_l1`/`state_l2` set handling).

## Rust change

- Multi-endpoint cluster fallback: a device with more than one OnOff endpoint
  publishes one entity per endpoint (`light_lN` when the endpoint also has
  Level/Color, otherwise `switch_lN`) instead of a single global entity.
- Exposes path: an expose with an `endpoint` field is suffixed
  (`state_l1`, `temperature_l1`) and routed to `<friendly>/<endpoint>/set`.
- Switch entities keep the main state topic and use bracket-notation
  `state_<endpoint>` templates; light entities use the dedicated
  `<base>/<friendly>/<endpoint>` state topic (no `value_template`), matching
  upstream.
- `publish_state` now derives stripped per-endpoint payloads from suffixed state
  keys and publishes them to `<base>/<friendly>/<endpoint>` (QoS 0, not
  retained), reproducing the z2m postfix republish.
- MQTT subscribes to `<base>/+/+/set` and `<base>/+/+/get`; command parsing maps
  a trailing `l<N>`/`button` segment to `SetDevice.endpoint`, and `handle_set`
  accepts both `state_lN`/`brightness_lN` keys and endpoint-scoped topics.
- `endpoint_suffix` uses `l<N>` for OnOff endpoints and keeps the historical
  `button` name for action-only endpoint 3; single-endpoint discovery output is
  unchanged.

## Tests and evidence

`cargo test --locked` → **383 passed** (351 library, 2 binary, 30 compatibility);
clippy exit 0 with 5 existing warnings; native release build succeeds.

New tests:

- `multi_endpoint_discovery_publishes_one_entity_per_endpoint` — exact light and
  switch payloads (object/unique id, state/command topics, template).
- `multi_endpoint_cleanup_topics_match_discovery_entities` — cleanup mirrors
  discovery.
- `single_endpoint_discovery_shape_is_unchanged` — no regression for one gang.
- `expose_endpoint_produces_suffixed_entity` — endpoint exposes.
- `endpoint_state_payloads_strip_suffix_and_ignore_plain_keys` — republish split
  and `color_temp`/`linkquality` exclusion.
- `endpoint_suffix_uses_l_index_for_on_off_gangs` — `l1`/`l3` vs `button`.
- `endpoint_set_topic_selects_the_endpoint` — `<name>/l2/set`, nested friendly
  names and plain `<name>/set`.
- `endpoint_state_topic_matches_discovery_contract`.

## Divergences and gaps (not verified)

- The cluster fallback names endpoints `l<N>`; definitions with real endpoint
  names (`left`/`right`) are only honoured through the exposes path.
- No live broker/HA test: topic/payload shapes are golden-tested, and the
  republish is unit-tested, but retained/QoS and HA entity migration are not
  observed end to end.
- `cover_<endpoint>` republish is not implemented; only light/switch entities.
- Group multi-endpoint and `command_topic_postfix` (per-attribute set topics)
  are not ported.
- No hardware or Home Assistant instance evidence; HA-005 stays `IN_PROGRESS`.
