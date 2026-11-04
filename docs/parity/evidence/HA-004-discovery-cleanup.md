# HA-004 — retained discovery cleanup on rename/remove

Rust implementation: `src/homeassistant.rs` (`discovery_topics`,
`cleanup_discovery`), `src/mqtt/mod.rs` (`clear_ha_discovery`,
`ha_discovery_topic`, `ha_clear_payload`), `src/bridge.rs` rename/remove.
Behavior: HA-004; plan HA cleanup.

## Source contract

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2`,
`lib/extension/homeassistant.ts`: when an entity is removed or renamed, the
retained discovery config is deleted by publishing an **empty payload** with
`retain: true, qos: 1` to the same discovery topic (for example lines 1527,
1613, 2058, 2108, 2146). Discovery topics are
`homeassistant/<component>/<node>/<object>/config`.

## Rust change

- `ha_discovery_topic` is the single formula used by publish and cleanup;
  `ha_clear_payload` returns the exact empty payload.
- `MqttBridge::clear_ha_discovery` publishes that empty payload with
  `QoS::AtLeastOnce` and retained=true.
- `discovery_topics` derives the entity list from prepared exposes when
  available, otherwise mirrors the cluster-fallback publisher (light/switch,
  climate, electrical sensors, contact, action, IAS field, climate sensors,
  occupancy, linkquality). Topics are sorted/deduplicated.
- `cleanup_discovery` clears every retained config.
- `device/remove` cleans discovery before removing the device.
- `device/rename` cleans the old entity configs and republishes discovery for the
  new device; the response now reports the actual `homeassistant_rename` state.

## Tests and evidence

- `discovery_cleanup_topics_follow_cluster_fallback`: light + linkquality; a
  multi-sensor entity set; sorted/unique.
- `discovery_cleanup_topics_follow_exposes`: numeric/binary/switch/light →
  exact component/object pairs.
- `discovery_cleanup_topic_and_payload_are_exact`: topic
  `homeassistant/sensor/<ieee>/temperature/config` and empty payload.
- `cleanup_publishes_empty_retained_configs_to_all_topics`: a recording publisher
  captures every exact cleanup topic and the empty payload contract.
- `device_remove_mqtt_e2e_clears_retained_discovery_and_replies`: a fake
  `MqttPublishIo` receives the device/remove command path; it asserts two exact
  discovery topics, empty payloads, QoS 1, retain=true, the bridge devices
  publication and the ok response with transaction.
- `device_remove_mqtt_e2e_reports_unknown_entity`: no cleanup publishes and an
  error response for an unregistered entity.
- Full suite: `cargo test --locked` → **368 passed** (336 library, 2 binary,
  30 compatibility); clippy exit 0 with 5 existing warnings; release build OK.

## Divergences and gaps (not verified)

- The fake MQTT sink verifies topic, empty payload, QoS and retain at the publish
  boundary and drives the remove handler; it does not run the full MQTT socket
  event loop or a real broker.
- The cluster-fallback topic list duplicates publish conditions and can drift;
  it should be generated from one entity list.
- Rename cleanup/republish is tested only through helpers; bridge E2E and HA
  entity migration (unique_id stability, object_id changes) are untested.
- Group discovery, entity category/availability cleanup, HA birth and discovery
  prefix options are not covered.
- No hardware or Home Assistant instance evidence; HA-004 stays `IN_PROGRESS`.
