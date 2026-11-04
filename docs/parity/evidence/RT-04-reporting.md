# RT-04 — reporting read/configure

Rust implementation: `src/bridge.rs` (`handle_reporting`),
`src/zigbee/zcl/frame.rs` (builder), `src/zigbee/zcl/mod.rs` (parser),
`src/converters/external.rs` (attribute name resolution). Behavior:
WINDFRONT-018/019; plan RT-04.

## Source contract

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2`,
`lib/extension/bridge.ts:509–599`:

- `device/reporting/configure` validates `id`, `endpoint`, `cluster`,
  `attribute`, `minimum_report_interval`, `maximum_report_interval`, optional
  `reportable_change`; it binds the cluster to the coordinator, calls
  `endpoint.configureReporting` and returns the echoed configuration.
- `device/reporting/read` validates `configs`, calls
  `endpoint.readReportingConfig(cluster, configs)` and returns the configs.

Herdsman endpoint/configure-reporting code owns the ZCL encoding and data-type
lookup. This Rust change implements the equivalent ZCL commands for the
attributes present in the published `zcl.json`.

## Rust change

- Added ZCL Read Reporting Configuration (0x08) builder and 0x09 response parser
  (direction, attribute, status, data type, min/max intervals, reportable
  change for fixed-size analog types).
- `device/reporting/read`: resolves attribute names through `zcl.json`, sends the
  request via `request_zcl`, verifies the response command and returns
  `{id, endpoint, cluster, configs[]}` including status and hex reportable
  change.
- `device/reporting/configure`: validates the upstream payload, discovers the
  attribute data type by reading the current reporting configuration (the bridge
  payload does not carry it), binds the cluster to the coordinator, sends
  Configure Reporting (0x06) and checks the 0x07 response status. Unsupported
  types (e.g. variable-length strings, semi-float) are rejected rather than
  encoded incorrectly.
- Both operations return MQTT `error` instead of a fake `ok` on failure.
- Successful read/configure responses are persisted into the endpoint record as
  `configuredReportings` (`cluster`, `attrId`, `minRepIntval`, `maxRepIntval`,
  `repChange`) and the database is written immediately.

## Fake-coordinator end-to-end tests

`ReportingIo` is a small transport seam implemented by `CoordinatorHandle`. The
tests use a fake coordinator and a real in-memory database:

- `reporting_read_e2e_with_fake_coordinator_persists_and_replies`: handler →
  ZCL 0x08 → fake 0x09 response → parsed config (u16, min 10, max 300) →
  persisted `configuredReportings` → `reporting_mqtt_response` status `ok` with
  the transaction field.
- `reporting_configure_e2e_with_fake_coordinator_binds_and_persists`: payload
  validation → type discovery 0x08/0x09 → bind (recorded once) → 0x06 → fake 0x07
  success → persisted min/max/repChange → `error` reply envelope for a failed
  request.

## Tests and evidence

- `read_reporting_config_payload_layout` (ZCL bytes).
- `parses_reporting_configuration_response` (success + unsupported status).
- `codecs_reportable_change_for_analog_and_discrete_types` (u16/u8, discrete,
  float, unsupported string).
- `configured_reporting_is_inserted_and_replaced` (database record insert,
  replace by cluster/attribute, unknown device).
- `reporting_change_values_decode_integer_and_float`.
- Full suite: `cargo test --locked` → **313 passed**.

## Divergences and gaps (not verified)

- The handler is covered with a fake coordinator and in-memory database; a real
  MQTT broker round trip and a real coordinator are still untested.
- Data-type discovery requires the attribute to already have a reporting
  configuration; a never-configured attribute cannot be configured yet.
- Bind is sent but its ZDO response is not awaited; a bind failure may surface
  only later.
- Manufacturer-specific reporting (`manufacturer_code`) is ignored.
- Persistence is covered at the database layer; no fake-coordinator test drives
  the bridge handler into it.
- `options` in the configure payload are ignored.
- No HIL; WINDFRONT-018/019 stay `IN_PROGRESS`.
