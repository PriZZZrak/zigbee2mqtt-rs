# RT-04 — MQTT get performs native ZCL reads

Rust implementation: `src/mqtt/mod.rs` (`GetDevice` payload),
`src/bridge.rs` (`handle_get`, `build_get_targets`, `read_target`).
Behavior: MQTT-010; plan RT-04.

## Source contract

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2`:

- `lib/extension/publish.ts:110–289`: an MQTT `get` parses requested keys and
  calls the device definition's `convertGet`, which issues device reads. A cache
  publication is not the get operation.
- `lib/extension/receive.ts:102–190`: the resulting report/read response is
  converted and published through the normal state pipeline.

Converters can implement `convertGet` arbitrarily (manufacturer attributes,
special encodings), so this Rust change only claims the **native** read path and
does not execute upstream `convertGet` yet.

## Rust change

- `GetDevice` now carries the MQTT payload. An empty payload means “all readable
  native keys”; a JSON object selects its keys.
- `build_get_targets` maps requested state keys to endpoint/cluster/attributes,
  combines keys that share a cluster, and skips clusters absent from the device.
  Supported native keys: state, brightness, color/color_temp, temperature,
  humidity, illuminance, battery/battery_voltage, occupancy, contact/tamper.
- `handle_get` sends each read through `CoordinatorHandle::request_zcl`
  (3 s per attempt, 2 attempts), parses the real read response with
  `zcl::parse_message`, merges only returned values, and publishes the updated
  state. If no read succeeds it logs an error and publishes nothing — cached
  state is not presented as a device answer.
- Unknown requested keys are ignored (no generic converter `convertGet` yet).

## Tests and evidence

`cargo test --locked` → **308 passed** (276 library, 2 binary, 30 compat).
`cargo clippy --all-targets --locked` → exit 0, 5 existing warnings.

New tests:

- `request_flow_tests` (4) use a fake sender instead of a serial transport:
  response on first attempt, timeout + retry + error, AF request rejection,
  AF confirm failure. They also assert no waiter leaks.
- `bridge::tests`: native key→attribute mapping; combining/deduplicating
  color/color_temp, per-endpoint routing, unknown key handling, and skipping
  clusters the device does not expose.

Existing transaction tests cover response matching, default response, confirm,
expiry and the event-pump replay.

## Divergences and gaps (not verified)

- No broker integration test; MQTT parse → handle_get is not exercised end to end.
- Upstream `convertGet` (including manufacturer-specific reads) is not executed;
  only native keys are refreshed.
- Reads are sequential and can block the bridge loop for up to attempts × timeout
  (up to 6 s per cluster) on a sleeping/unreachable device. Bounded concurrency or
  per-device queueing is still needed.
- Partial failures publish the successful keys only; upstream per-key behavior
  and error surfacing still need a differential test.
- Reporting read/configure via bridge requests remains a stub.
- No hardware evidence; MQTT-010 stays `IN_PROGRESS`.
