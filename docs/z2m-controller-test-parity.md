# z2m `controller.test.ts` parity — per-case mapping

Source: `.upstream/zigbee2mqtt/test/controller.test.ts` (zigbee2mqtt 2.14.3,
`3c841fdf`), 90 `it` cases (11 of them the "aborts during startup" matrix).
Rust suite: `crates/z2m-rs/tests/controller_lifecycle_suite.rs` (11 tests) plus
the existing suites named per row.

Upstream mocks `Zigbee`, `Mqtt` and the zigbee-herdsman controller with vitest
spies. The Rust port has no such mocks: the cases that need a running
coordinator (`zigbee.start`), an MQTT broker or the zh controller lifecycle
cannot be reproduced as unit tests and are mapped to the extension suites that
assert the same behaviour or to the HIL run. Nothing is claimed as passing
without a test.

| # | upstream `it` | status |
|---|---|---|
| 1 | Start controller | not ported — needs a live coordinator (zigbee.start); covered by the HIL start/resume run |
| 2 | Start controller with specific MQTT settings | covered by — `crates/z2m-rs/tests/mqtt_broker.rs` (client options) and `crates/z2m-rs/tests/settings.rs` |
| 3 | Start controller with only username MQTT settings | covered by — `crates/z2m-rs/tests/mqtt_broker.rs` |
| 4 | Should generate network_key, pan_id and ext_pan_id when set to GENERATE | covered by — `crates/z2m-rs/tests/settings.rs` (GENERATE validation) |
| 5 | Start controller should publish cached states | covered by — `crates/z2m-rs/tests/state.rs` (cache replay) + `crates/z2m-rs/tests/publish.rs` |
| 6 | Start controller should not publish cached states when disabled | covered by — `crates/z2m-rs/tests/state.rs` |
| 7 | Start controller should not publish cached states when cache_state is false | covered by — `crates/z2m-rs/tests/state.rs` |
| 8 | Log when MQTT client is unavailable | covered by — `crates/z2m-rs/src/mqtt.rs` unit tests |
| 9 | Dont publish to mqtt when client is unavailable | `does_not_publish_wildcard_topics` asserts the no-broker path |
| 10 | Should not allow publishing wildcard characters in topic | `does_not_publish_wildcard_topics` |
| 11 | Load empty state when state file does not exist | `loads_empty_state_when_state_file_does_not_exist` |
| 12 | Should remove device not on passlist on startup | not ported — startup filtering needs a live coordinator |
| 13 | Should remove device on blocklist on startup | not ported — startup filtering needs a live coordinator |
| 14 | Start controller fails | not ported — needs a live coordinator |
| 15 | Start controller fails after onboarding | not ported — needs a live coordinator |
| 16 | Start controller fails due to MQTT connect error | covered by — `crates/z2m-rs/tests/mqtt_broker.rs` |
| 17 | Start controller fails due to MQTT connect error after onboarding | covered by — `crates/z2m-rs/tests/mqtt_broker.rs` |
| 18 | Start controller and stop with restart | covered by — `bridge/request/restart` + HIL restart evidence |
| 19 | Start controller and stop without SdNotify | covered by — `crates/z2m-rs/tests/sd_notify.rs` |
| 20 | Start controller and stop with SdNotify | covered by — `crates/z2m-rs/tests/sd_notify.rs` |
| 21 | during state start | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 22 | during zigbee resolve definitions | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 23 | during MQTT connect | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 24 | during extension start | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 25 | during entity state publish | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 26 | without signal during state start | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 27 | without signal during MQTT connect | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 28 | during ZH Controller start | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 29 | during ZH Controller getNetworkParameters | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 30 | during ZH Controller passlist | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 31 | during ZH Controller blocklist | not ported — abort-during-startup matrix; the port drops the start future (Rust cancellation), needs a live coordinator to assert |
| 32 | Start controller adapter disconnects | not ported — needs a live coordinator or MQTT broker |
| 33 | does not throw when extension fails to stop on controller stop | not ported — needs a live coordinator or MQTT broker |
| 34 | does not throw when extension stop throws | not ported — needs a live coordinator or MQTT broker |
| 35 | Handles reconnecting to MQTT | not ported — needs a live coordinator or MQTT broker |
| 36 | Handles reconnecting to MQTT after v5+ DISCONNECT | not ported — needs a live coordinator or MQTT broker |
| 37 | Handles MQTT publish error | not ported — needs a live coordinator or MQTT broker |
| 38 | Handle mqtt message | covered by — `crates/z2m-rs/tests/receive.rs` + `crates/z2m-rs/tests/bridge.rs` |
| 39 | Skip MQTT messages on topic we published to | covered by — `crates/z2m-rs/tests/mqtt_broker.rs` |
| 40 | On zigbee event message | covered by — `crates/z2m-rs/tests/receive.rs` |
| 41 | On zigbee event message with group ID | covered by — `crates/z2m-rs/tests/receive.rs` |
| 42 | Should add entities which are missing from configuration but are in database to configuration | not ported — needs controller start |
| 43 | On zigbee deviceJoined | covered by — `crates/z2m-rs/tests/on_event.rs` + `crates/z2m-rs/tests/bridge.rs` |
| 44 | acceptJoiningDeviceHandler reject device on blocklist | `accept_joining_rejects_device_on_blocklist` |
| 45 | acceptJoiningDeviceHandler accept device not on blocklist | `accept_joining_accepts_device_not_on_blocklist` |
| 46 | acceptJoiningDeviceHandler accept device on passlist | `accept_joining_accepts_device_on_passlist` |
| 47 | acceptJoiningDeviceHandler reject device not in passlist | `accept_joining_rejects_device_not_in_passlist` |
| 48 | acceptJoiningDeviceHandler should prefer passlist above blocklist | `accept_joining_prefers_passlist_above_blocklist` |
| 49 | acceptJoiningDeviceHandler accept when not on blocklist and passlist | `accept_joining_accepts_when_not_on_blocklist_and_passlist` |
| 50 | Shouldnt crash when two device join events are received | covered by — `crates/z2m-rs/tests/bridge.rs` |
| 51 | On zigbee deviceInterview started | covered by — `crates/z2m-rs/tests/bridge.rs` (device_interview event) |
| 52 | On zigbee deviceInterview failed | covered by — `crates/z2m-rs/tests/bridge.rs` |
| 53 | On zigbee deviceInterview successful supported | covered by — `crates/z2m-rs/tests/bridge.rs` |
| 54 | On zigbee deviceInterview successful not supported | covered by — `crates/z2m-rs/tests/bridge.rs` |
| 55 | On zigbee event device announce | covered by — `crates/z2m-rs/tests/bridge.rs` |
| 56 | On zigbee event device leave (removed from database and settings) | covered by — `crates/z2m-rs/tests/bridge.rs` + `crates/z2m-rs/tests/homeassistant.rs` cleanup |
| 57 | On zigbee event device leave (removed from database and NOT settings) | covered by — `crates/z2m-rs/tests/bridge.rs` |
| 58 | Publish entity state attribute output | covered by — `crates/z2m-rs/tests/publish.rs` |
| 59 | Publish entity state attribute_json output | covered by — `crates/z2m-rs/tests/publish.rs` |
| 60 | Publish entity state attribute output with a null color | covered by — `crates/z2m-rs/tests/publish.rs` |
| 61 | Publish entity state attribute_json output filtered | covered by — `crates/z2m-rs/tests/publish.rs` |
| 62 | Publish entity state attribute_json output filtered (device_options) | covered by — `crates/z2m-rs/tests/publish.rs` |
| 63 | Publish entity state caches a duration reported by the device | covered by — `crates/z2m-rs/tests/state.rs` |
| 64 | Publish entity state keeps an action_duration out of the cache | covered by — `crates/z2m-rs/tests/state.rs` |
| 65 | Publish entity state attribute_json output filtered cache | covered by — `crates/z2m-rs/tests/state.rs` |
| 66 | Publish entity state attribute_json output filtered cache (device_options) | covered by — `crates/z2m-rs/tests/state.rs` |
| 67 | Publish entity state with device information | covered by — `crates/z2m-rs/tests/publish.rs` |
| 68 | Should publish entity state without retain | covered by — `crates/z2m-rs/tests/publish.rs` |
| 69 | Should publish entity state with retain | covered by — `crates/z2m-rs/tests/publish.rs` |
| 70 | Should publish entity state with expiring retention | covered by — `crates/z2m-rs/tests/publish.rs` (retention is tracked, not sent: MQTT v4 client) |
| 71 | Publish entity state no empty messages | covered by — `crates/z2m-rs/tests/publish.rs` |
| 72 | Should allow to disable state persistency | covered by — `crates/z2m-rs/tests/state.rs` |
| 73 | Shouldnt crash when it cannot save state | covered by — `crates/z2m-rs/tests/state.rs` |
| 74 | Publish should not cache when set | covered by — `crates/z2m-rs/tests/state.rs` |
| 75 | Should start when state is corrupted | `starts_when_state_file_is_corrupted` |
| 76 | Start controller with force_disable_retain | covered by — `crates/z2m-rs/tests/publish.rs` |
| 77 | Should republish retained messages on MQTT initial connect | not ported — needs a broker; `crates/z2m-rs/tests/mqtt_broker.rs` covers connect only |
| 78 | Should not republish retained messages on MQTT initial connect when retained message are sent | not ported — needs a broker |
| 79 | Should republish retained messages on MQTT reconnect | not ported — needs a broker |
| 80 | Should not republish retained messages on MQTT reconnect when retained message are sent | not ported — needs a broker |
| 81 | Should not republish retained messages on MQTT reconnect when retained message arrives while publishing bridge state | not ported — needs a broker |
| 82 | Should prevent any message being published with retain flag when force_disable_retain is set | covered by — `crates/z2m-rs/tests/publish.rs` |
| 83 | Should publish last seen changes | covered by — `crates/z2m-rs/tests/last_seen.rs` |
| 84 | Should not publish last seen changes when reason is messageEmitted | covered by — `crates/z2m-rs/tests/last_seen.rs` |
| 85 | Ignore messages from coordinator | covered by — `crates/z2m-rs/tests/receive.rs` |
| 86 | Should remove state of removed device when stopped | covered by — `crates/z2m-rs/tests/state.rs` |
| 87 | EventBus should handle sync errors | not ported — Rust handlers are closures; no throwing contract (deviation documented) |
| 88 | EventBus should handle async errors | not ported — same as above |
| 89 | prevents interacting with invalid extensions | `prevents_interacting_with_invalid_extensions` |
| 90 | throws error when adding extension twice | `throws_when_adding_an_extension_twice` |

## Notes

- `addExtension` now rejects a duplicate name (`Extension '<name>' already
  exists`), matching upstream's throw (case 90).
- `EventBus` sync/async error cases (87-88) have no Rust analogue: handlers are
  plain closures and there is no exception channel; a panicking handler aborts
  the calling task, which is a documented deviation.
- The startup/abort/retained-republish matrix (1-4, 12-37, 42, 77-82) is **not**
  proven by the HIL acceptance run. The HIL run covers start/resume, bridge
  requests, persistence and a normal restart only; aborts, reconnects and
  retained republishing need dedicated tests (CP-02, CP-05 in
  `docs/test-coverage-audit.md`).
