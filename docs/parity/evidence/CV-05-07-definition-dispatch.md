# CV-05..07 — definition preparation, message metadata and unified dispatch

Rust implementation: `src/converters/external.rs` (`__prepareDefinition`,
`IncomingMessageInfo`, `convert_from_zigbee_with_message`), `src/bridge.rs`
(`merge_converted_values`, message branch). Behavior: CONVERTER-008/009/016;
plan CV-05..07.

## Source contract

Pinned converters `9dabbe4e343573c5a3070605efd843a1b3a16058`:

- `src/index.ts:256–420` (`processExtensions`): modern definitions declare
  `extend: [...]`; each extend contributes `fromZigbee`, `toZigbee`, `exposes`,
  `options`, `configure` and `onEvent`. `prepareDefinition` merges these with the
  definition-level fields; `define` code consumes the prepared definition.
- Converter messages carry the source endpoint and link quality in `msg`;
  `meta.device` identifies the device and `meta.state` the current state.
- App `lib/extension/receive.ts` runs the matching `fromZigbee` converter and
  publishes its result once.

## Rust change

- Added a JS `__prepareDefinition` used by both `convert_from_zigbee` and
  `convert_get`: merges modern-extend `fromZigbee`/`toZigbee`/`exposes`/`options`
  arrays into the definition (undefined configure/onEvent filters are not yet
  modeled).
- Added `IncomingMessageInfo { endpoint, linkquality, group_id }` and
  `convert_from_zigbee_with_message`; the JS `msg` now receives the real source
  endpoint, LQI and group id instead of hardcoded endpoint 1 / LQI 0. The old
  signature remains for callers/tests and uses the default.
- Endpoint helpers: `msg.endpoint.getDevice()` / `deviceIeeeAddress` and
  `meta.device.getEndpoint(ID)` are available to fz converters; the convertGet
  entity exposes `getDevice()`/`deviceIeeeAddress` too.
- Unified dispatch: native parsing and the upstream definition now both run for
  a frame. `merge_converted_values` gives definition values precedence while
  retaining native keys it does not emit; state is merged and published exactly
  once, with suffix/action/availability handling applied to the merged map.

## Tests and evidence

- `extend_only_definition_is_prepared_before_dispatch`: a fixture definition with
  no direct `fromZigbee`, only a modern-extend converter, returns temperature.
- `message_endpoint_and_linkquality_reach_the_converter`: converter observes
  endpoint 3 and LQI 77 from the message metadata.
- `message_endpoint_and_linkquality_reach_the_converter` also asserts
  `getDevice().modelID`, IEEE address and endpoint count.
- `convert_get_entity_exposes_device_identity`: an async convertGet chooses the
  attribute based on `entity.getDevice().modelID`.
- `unified_dispatch_prefers_converter_values_and_keeps_native_extras` and
  `unified_dispatch_without_converter_keeps_native_values`.
- Full suite: `cargo test --locked` → **323 passed** (291 library, 2 binary,
  30 compatibility).

## Divergences and gaps (not verified)

- `__prepareDefinition` is a simplified port: `meta` merge order, configure and
  onEvent arrays, exposes functions and endpoint overrides are not complete.
- Exposes are collected but not yet exposed through a Rust API/HA mapping.
- Endpoint identity helpers are basic; `meta.options`, entity
  write/command/configureReporting/bind, timers, shared converter store and
  multi-frame state remain partial (CV-06/07).
- Native and external conversion both perform work; only publication is
  deduplicated. Converter side effects, `publish` callbacks and action timing are
  not compared against upstream yet.
- No real published package or Node differential test; fixtures only. No HIL.
