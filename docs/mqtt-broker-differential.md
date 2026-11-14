# MQTT real-broker differential

Upstream: `.upstream/zigbee2mqtt/lib/mqtt.ts` (zigbee2mqtt 2.14.3, `3c841fdf`).

The extension tests in `test/extensions/*` stub `mqtt.publish`, so they cannot
prove the transport layer. This document records the differential run against a
real broker (aedes 0.51 on `127.0.0.1:18830`) using the ignored integration test
`crates/z2m-rs/tests/mqtt_broker.rs`.

## Reproduction

```bash
# terminal 1
cd /tmp/opencode/broker && node broker.js

# terminal 2
cd /config/workspace/zigbee2mqtt-rs
MQTT_TEST_SERVER=mqtt://127.0.0.1:18830 \
  cargo test -p z2m-rs --test mqtt_broker -- --ignored --nocapture
```

## What the run proves

1. `Mqtt::connect` reaches the broker and publishes the retained
   `<base_topic>/bridge/state` = `{"state":"online"}` payload on `CONNACK`,
   subscribing `<base_topic>/#` in the same step (`mqtt.ts:154-191`).
2. `Mqtt::publish(topic, payload, {qos:1})` actually delivers bytes: an
   independent `rumqttc` observer client subscribed to
   `z2m-rs-broker-test/state` receives `{"state":"ON"}`.
3. `Mqtt::stats()` reports `connected` and a live `queued` value. `queued` is
   the number of in-flight `publish()` calls plus the PKIDs that were sent with
   QoS 1/2 and not yet acknowledged (`PUBACK`/`PUBCOMP`), which is the closest
   Rust equivalent of upstream `client.queue.length`; it is not a hardcoded
   constant.
4. `Mqtt::disconnect` publishes the retained `{"state":"offline"}` payload.

## Transcript (2026-11-14)

```
running 1 test
mqtt.stats = {"connected":true,"queued":0}
test real_broker_connect_publish_and_queue_stats ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s

server=mqtt://127.0.0.1:18830
stats={"connected":true,"queued":0}
delivered=Some("{\"state\":\"ON\"}")
```

`queued` is `0` in the steady state after the QoS 1 acknowledgement; the
counter is exercised at non-zero only while a publish is in flight or
unacknowledged, which the code path tracks with an RAII guard plus the outgoing
packet-id set (`crates/z2m-rs/src/mqtt.rs`).

## Not covered by this transcript

- Reconnect/offline transitions against a broker that drops the socket: the
  event loop logs the error and retries after 1s, but no automated test kills
  the broker mid-run yet.
- Retained-topic replay after reconnect.
- HIL against a coordinator (`AGENTS.md` §9).

These remain tracked in `docs/parity-matrix.md`.
