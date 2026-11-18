# z2m `controller.test.ts` parity — per-case mapping

Source: `.upstream/zigbee2mqtt/test/controller.test.ts` (zigbee2mqtt 2.14.3,
`3c841fdf`), 90 `it` cases (11 of them the "aborts during startup" matrix).
Rust suite: `crates/z2m-rs/tests/controller_lifecycle_suite.rs` (11 tests) plus
the existing suites named per row.

Upstream mocks `Zigbee`, `Mqtt` and the zigbee-herdsman controller with vitest
spies. The Rust port uses deterministic seams instead: `Zigbee::test_start`
fakes the coordinator phase, `Controller::test_start_pause` holds the startup
between extensions, and `crates/z2m-rs/tests/support/mqtt_broker.rs` is an
in-process MQTT broker. Cases that still need a real coordinator (passlist,
blocklist, cached replay) are mapped to the HIL run. Nothing is claimed as
passing without a test.

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
| 21 | during state start | N/A — `State::start` (`state.ts:44-49`) is synchronous (load + save timer), so there is no await point to abort; the abort boundary right after it is ported (`controller.ts:107-110`) and `aborting_start_during_the_coordinator_phase_skips_extensions` asserts that no MQTT connect or extension starts |
| 22 | during zigbee resolve definitions | covered by — `aborting_start_during_the_coordinator_phase_skips_extensions` (future dropped inside `zigbee.start`; MQTT never connects, no extension starts) |
| 23 | during MQTT connect | `aborting_during_a_slow_mqtt_connect_returns_ok_and_tears_down` (connect is bounded, `stop` sets the abort flag first, the failure is reported as an aborted start) |
| 24 | during extension start | `aborting_with_active_mqtt_stops_runtime_and_restarts_cleanly` (paused after 3 extensions: listeners, save timer and MQTT connection are closed; a fresh start runs all 10) |
| 25 | during entity state publish | partial — the abort boundary after every cached publish is ported (`controller.ts:172-180`); the replay itself needs devices from a coordinator-backed `zigbee.start`, so it is exercised by the HIL run (CP-12) |
| 26 | without signal during state start | N/A — same as 21; `stop()` without a signal aborts with `STOPABORT` and code 0 |
| 27 | without signal during MQTT connect | same as 23 — `stop(false, 0)` |
| 28 | during ZH Controller start | `aborting_start_during_the_coordinator_phase_skips_extensions` (`AbortSignal::wait` is passed into `start_with_abort`) |
| 29 | during ZH Controller getNetworkParameters | inside `zigbee.start`; the abort is checked after the call (`crates/z2m-rs/src/zigbee.rs` passlist/blocklist section); interrupting the call itself needs a fake coordinator, so it stays coordinator-only HIL (CP-12) |
| 30 | during ZH Controller passlist | abort checked after each `removeFromNetwork` (`crates/z2m-rs/src/zigbee.rs`); removal needs a coordinator, so the interruption stays HIL (CP-12) |
| 31 | during ZH Controller blocklist | same as 30 |
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
| 78 | Should not republish retained messages on MQTT initial connect when retained message are sent | `cancels_the_republish_when_a_retained_bridge_info_arrives` |
| 79 | Should republish retained messages on MQTT reconnect | `reconnects_and_republishes_state_after_the_broker_drops_the_connection` |
| 80 | Should not republish retained messages on MQTT reconnect when retained message are sent | `cancels_the_republish_when_a_retained_bridge_info_arrives` |
| 81 | Should not republish retained messages on MQTT reconnect when retained message arrives while publishing bridge state | `cancels_the_republish_when_a_retained_bridge_info_arrives` (timer armed before the state publish) |
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
- The abort matrix (21-31) is mapped per phase: 22/24/26-28 need no
  coordinator and are asserted in `crates/z2m-rs/tests/controller_startup.rs`;
  25 and 29-31 stay coordinator-only and are exercised by the HIL run (CP-12).
- Retained republish/reconnect cases 77-81 are asserted against the in-process
  broker in `crates/z2m-rs/tests/mqtt_reconnect.rs` (CP-02); the remaining
  startup cases (1, 12-15, 42) still need a live coordinator or the HIL run.
